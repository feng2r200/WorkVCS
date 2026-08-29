use crate::canonical::{
    CanonicalValue, canonical_bytes, content_object_digest, parse_canonical_json,
};
use crate::error::{Result, WorkVcsError, storage_error};
use crate::identity::{Digest, EvidenceId, SessionId};
use crate::store::{StoreConnection, current_epoch_micros};
use rusqlite::{OptionalExtension, Transaction, TransactionBehavior, params};

pub(crate) const EVIDENCE_OBJECT_KIND: &str = "evidence";
const CONTENT_OBJECT_KIND: &str = "content object";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EvidenceContentInput {
    role: String,
    content_digest: Digest,
    size_bytes: i64,
    media_type: Option<String>,
    format_metadata: CanonicalValue,
}

impl EvidenceContentInput {
    pub fn from_raw_bytes(role: impl Into<String>, raw_bytes: impl AsRef<[u8]>) -> Result<Self> {
        let raw_bytes = raw_bytes.as_ref();
        let size_bytes = i64::try_from(raw_bytes.len()).map_err(|_| {
            WorkVcsError::EvidenceInvalid("content is too large for SQLite size_bytes".to_owned())
        })?;
        Self::from_digest(role, content_object_digest(raw_bytes), size_bytes)
    }

    pub fn from_digest(
        role: impl Into<String>,
        content_digest: Digest,
        size_bytes: i64,
    ) -> Result<Self> {
        if size_bytes < 0 {
            return Err(WorkVcsError::EvidenceInvalid(
                "content size_bytes must be non-negative".to_owned(),
            ));
        }
        let role = role.into();
        validate_stored_text("evidence content role", &role)?;
        Ok(Self {
            role,
            content_digest,
            size_bytes,
            media_type: None,
            format_metadata: CanonicalValue::object(Vec::new())?,
        })
    }

    pub fn with_media_type(mut self, media_type: impl Into<String>) -> Result<Self> {
        let media_type = media_type.into();
        validate_stored_text("content media_type", &media_type)?;
        self.media_type = Some(media_type);
        Ok(self)
    }

    pub fn with_format_metadata(mut self, format_metadata: CanonicalValue) -> Result<Self> {
        require_object_value("content format_metadata", &format_metadata)?;
        self.format_metadata = format_metadata;
        Ok(self)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EvidenceCreateOptions {
    evidence_kind: String,
    source_session_id: Option<SessionId>,
    metadata: CanonicalValue,
    contents: Vec<EvidenceContentInput>,
}

impl EvidenceCreateOptions {
    pub fn new(evidence_kind: impl Into<String>, metadata: CanonicalValue) -> Result<Self> {
        let evidence_kind = evidence_kind.into();
        validate_stored_text("evidence kind", &evidence_kind)?;
        require_object_value("evidence metadata", &metadata)?;
        Ok(Self {
            evidence_kind,
            source_session_id: None,
            metadata,
            contents: Vec::new(),
        })
    }

    pub fn with_source_session_id(mut self, source_session_id: SessionId) -> Self {
        self.source_session_id = Some(source_session_id);
        self
    }

    pub fn with_contents<I>(mut self, contents: I) -> Result<Self>
    where
        I: IntoIterator<Item = EvidenceContentInput>,
    {
        self.contents = contents.into_iter().collect();
        validate_evidence_contents(&self.contents)?;
        Ok(self)
    }

    pub fn evidence_kind(&self) -> &str {
        &self.evidence_kind
    }

    pub fn source_session_id(&self) -> Option<SessionId> {
        self.source_session_id
    }

    pub fn metadata(&self) -> &CanonicalValue {
        &self.metadata
    }

    pub fn contents(&self) -> &[EvidenceContentInput] {
        &self.contents
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EvidenceCreateResult {
    pub evidence_id: EvidenceId,
    pub evidence_kind: String,
    pub captured_at_us: i64,
    pub source_session_id: Option<SessionId>,
    pub metadata: CanonicalValue,
    pub contents: Vec<EvidenceContentSnapshot>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EvidenceSnapshot {
    pub evidence_id: EvidenceId,
    pub evidence_kind: String,
    pub captured_at_us: i64,
    pub source_session_id: Option<SessionId>,
    pub metadata: CanonicalValue,
    pub contents: Vec<EvidenceContentSnapshot>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EvidenceContentSnapshot {
    pub ordinal: usize,
    pub role: String,
    pub content_digest: Digest,
    pub size_bytes: i64,
    pub media_type: Option<String>,
    pub format_metadata: CanonicalValue,
}

pub(crate) fn create_evidence(
    connection: &mut StoreConnection,
    options: &EvidenceCreateOptions,
) -> Result<EvidenceCreateResult> {
    connection.verify_foreign_keys()?;
    validate_stored_text("evidence kind", options.evidence_kind())?;
    require_object_value("evidence metadata", options.metadata())?;
    validate_evidence_contents(options.contents())?;

    let evidence_id = EvidenceId::new_v7();
    let captured_at_us = current_epoch_micros()?;
    let metadata_json = canonical_object_json("evidence metadata", options.metadata())?;

    let transaction = connection
        .inner_mut()
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(storage_error)?;
    write_evidence(
        &transaction,
        evidence_id,
        captured_at_us,
        &metadata_json,
        options,
    )?;
    transaction.commit().map_err(storage_error)?;

    let snapshot = evidence(connection, evidence_id)?;
    Ok(EvidenceCreateResult {
        evidence_id,
        evidence_kind: snapshot.evidence_kind,
        captured_at_us: snapshot.captured_at_us,
        source_session_id: snapshot.source_session_id,
        metadata: snapshot.metadata,
        contents: snapshot.contents,
    })
}

pub(crate) fn evidence(
    connection: &StoreConnection,
    evidence_id: EvidenceId,
) -> Result<EvidenceSnapshot> {
    let row = connection
        .inner()
        .query_row(
            "SELECT object_identity.object_kind,
                    evidence.evidence_kind,
                    evidence.captured_at_us,
                    evidence.source_session_id,
                    evidence.metadata_json
             FROM evidence
             JOIN object_identity
               ON object_identity.object_id = evidence.evidence_id
             WHERE evidence.evidence_id = ?1",
            params![&evidence_id.raw_bytes()[..]],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, Option<Vec<u8>>>(3)?,
                    row.get::<_, String>(4)?,
                ))
            },
        )
        .optional()
        .map_err(storage_error)?;

    let Some((object_kind, evidence_kind, captured_at_us, source_session_id, metadata_json)) = row
    else {
        return Err(WorkVcsError::EvidenceNotFound(format!(
            "evidence {evidence_id} does not exist"
        )));
    };
    if object_kind != EVIDENCE_OBJECT_KIND {
        return Err(WorkVcsError::EvidenceInvalid(format!(
            "evidence {evidence_id} has object kind {object_kind:?}"
        )));
    }
    validate_stored_text("evidence kind", &evidence_kind)?;
    if captured_at_us < 0 {
        return Err(WorkVcsError::EvidenceInvalid(format!(
            "evidence {evidence_id} has negative captured_at_us"
        )));
    }
    let source_session_id = source_session_id
        .map(|bytes| decode_session_id("evidence.source_session_id", bytes))
        .transpose()?;
    let metadata = parse_canonical_object_json("evidence.metadata_json", &metadata_json)?;

    Ok(EvidenceSnapshot {
        evidence_id,
        evidence_kind,
        captured_at_us,
        source_session_id,
        metadata,
        contents: load_evidence_contents(connection, evidence_id)?,
    })
}

pub(crate) fn require_evidence_exists(
    connection: &StoreConnection,
    evidence_id: EvidenceId,
) -> Result<()> {
    let object_kind = connection
        .inner()
        .query_row(
            "SELECT object_identity.object_kind
             FROM evidence
             JOIN object_identity
               ON object_identity.object_id = evidence.evidence_id
             WHERE evidence.evidence_id = ?1",
            params![&evidence_id.raw_bytes()[..]],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(storage_error)?;
    let Some(object_kind) = object_kind else {
        return Err(WorkVcsError::EvidenceNotFound(format!(
            "evidence {evidence_id} does not exist"
        )));
    };
    if object_kind != EVIDENCE_OBJECT_KIND {
        return Err(WorkVcsError::EvidenceInvalid(format!(
            "evidence {evidence_id} has object kind {object_kind:?}"
        )));
    }
    Ok(())
}

fn write_evidence(
    transaction: &Transaction<'_>,
    evidence_id: EvidenceId,
    captured_at_us: i64,
    metadata_json: &str,
    options: &EvidenceCreateOptions,
) -> Result<()> {
    let evidence_id_bytes = evidence_id.raw_bytes();
    let source_session_id_bytes = options.source_session_id().map(|id| id.raw_bytes());
    transaction
        .execute(
            "INSERT INTO object_identity(object_id, object_kind, created_at_us)
             VALUES (?1, ?2, ?3)",
            params![&evidence_id_bytes[..], EVIDENCE_OBJECT_KIND, captured_at_us],
        )
        .map_err(storage_error)?;
    transaction
        .execute(
            "INSERT INTO evidence(
                evidence_id,
                evidence_kind,
                captured_at_us,
                source_session_id,
                metadata_json
             )
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                &evidence_id_bytes[..],
                options.evidence_kind(),
                captured_at_us,
                source_session_id_bytes.as_ref().map(|bytes| &bytes[..]),
                metadata_json
            ],
        )
        .map_err(storage_error)?;

    for (ordinal, content) in options.contents().iter().enumerate() {
        ensure_content_object(transaction, content)?;
        transaction
            .execute(
                "INSERT INTO evidence_content(
                    evidence_id,
                    ordinal,
                    content_digest,
                    role
                 )
                 VALUES (?1, ?2, ?3, ?4)",
                params![
                    &evidence_id_bytes[..],
                    i64::try_from(ordinal).map_err(|_| {
                        WorkVcsError::EvidenceInvalid(
                            "evidence content ordinal is too large".to_owned(),
                        )
                    })?,
                    &content.content_digest.as_bytes()[..],
                    content.role.as_str()
                ],
            )
            .map_err(storage_error)?;
    }
    Ok(())
}

fn ensure_content_object(
    transaction: &Transaction<'_>,
    content: &EvidenceContentInput,
) -> Result<()> {
    let format_metadata_json =
        canonical_object_json("content format_metadata", &content.format_metadata)?;
    let row = transaction
        .query_row(
            "SELECT size_bytes, media_type, format_metadata_json
             FROM content_object
             WHERE content_digest = ?1",
            params![&content.content_digest.as_bytes()[..]],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, Option<String>>(1)?,
                    row.get::<_, String>(2)?,
                ))
            },
        )
        .optional()
        .map_err(storage_error)?;

    if let Some((size_bytes, media_type, stored_format_metadata_json)) = row {
        let stored_format_metadata = parse_canonical_object_json(
            "content_object.format_metadata_json",
            &stored_format_metadata_json,
        )?;
        if size_bytes != content.size_bytes
            || media_type != content.media_type
            || stored_format_metadata != content.format_metadata
        {
            return Err(WorkVcsError::EvidenceInvalid(format!(
                "{CONTENT_OBJECT_KIND} {} already exists with different metadata",
                content.content_digest
            )));
        }
        return Ok(());
    }

    transaction
        .execute(
            "INSERT INTO content_object(
                content_digest,
                size_bytes,
                media_type,
                format_metadata_json
             )
             VALUES (?1, ?2, ?3, ?4)",
            params![
                &content.content_digest.as_bytes()[..],
                content.size_bytes,
                content.media_type.as_deref(),
                format_metadata_json
            ],
        )
        .map_err(storage_error)?;
    Ok(())
}

fn load_evidence_contents(
    connection: &StoreConnection,
    evidence_id: EvidenceId,
) -> Result<Vec<EvidenceContentSnapshot>> {
    let mut statement = connection
        .inner()
        .prepare(
            "SELECT evidence_content.ordinal,
                    evidence_content.role,
                    content_object.content_digest,
                    content_object.size_bytes,
                    content_object.media_type,
                    content_object.format_metadata_json
             FROM evidence_content
             JOIN content_object
               ON content_object.content_digest = evidence_content.content_digest
             WHERE evidence_content.evidence_id = ?1
             ORDER BY evidence_content.ordinal",
        )
        .map_err(storage_error)?;
    let rows = statement
        .query_map(params![&evidence_id.raw_bytes()[..]], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Vec<u8>>(2)?,
                row.get::<_, i64>(3)?,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, String>(5)?,
            ))
        })
        .map_err(storage_error)?;

    let mut contents = Vec::new();
    for row in rows {
        let (ordinal, role, content_digest, size_bytes, media_type, format_metadata_json) =
            row.map_err(storage_error)?;
        if ordinal != contents.len() as i64 {
            return Err(WorkVcsError::EvidenceInvalid(format!(
                "evidence {evidence_id} content ordinal {ordinal} is not contiguous"
            )));
        }
        validate_stored_text("evidence content role", &role)?;
        if let Some(media_type) = &media_type {
            validate_stored_text("content media_type", media_type)?;
        }
        if size_bytes < 0 {
            return Err(WorkVcsError::EvidenceInvalid(format!(
                "evidence {evidence_id} content ordinal {ordinal} has negative size_bytes"
            )));
        }
        let content_digest = decode_digest("content_object.content_digest", content_digest)?;
        let format_metadata = parse_canonical_object_json(
            "content_object.format_metadata_json",
            &format_metadata_json,
        )?;
        contents.push(EvidenceContentSnapshot {
            ordinal: usize::try_from(ordinal).map_err(|_| {
                WorkVcsError::EvidenceInvalid(format!(
                    "evidence {evidence_id} content ordinal {ordinal} is invalid"
                ))
            })?,
            role,
            content_digest,
            size_bytes,
            media_type,
            format_metadata,
        });
    }
    Ok(contents)
}

fn validate_evidence_contents(contents: &[EvidenceContentInput]) -> Result<()> {
    for content in contents {
        validate_stored_text("evidence content role", &content.role)?;
        if content.size_bytes < 0 {
            return Err(WorkVcsError::EvidenceInvalid(
                "content size_bytes must be non-negative".to_owned(),
            ));
        }
        if let Some(media_type) = &content.media_type {
            validate_stored_text("content media_type", media_type)?;
        }
        require_object_value("content format_metadata", &content.format_metadata)?;
    }
    Ok(())
}

fn validate_stored_text(label: &str, value: &str) -> Result<()> {
    if value.trim().is_empty() {
        return Err(WorkVcsError::EvidenceInvalid(format!(
            "{label} must not be empty"
        )));
    }
    if value.trim() != value {
        return Err(WorkVcsError::EvidenceInvalid(format!(
            "{label} must not have leading or trailing whitespace"
        )));
    }
    if value
        .as_bytes()
        .iter()
        .any(|byte| *byte == b'\0' || *byte < 0x20 || *byte == 0x7f)
    {
        return Err(WorkVcsError::EvidenceInvalid(format!(
            "{label} must not contain NUL or ASCII control characters"
        )));
    }
    Ok(())
}

fn require_object_value(label: &str, value: &CanonicalValue) -> Result<()> {
    match value {
        CanonicalValue::Object(_) => Ok(()),
        _ => Err(WorkVcsError::EvidenceInvalid(format!(
            "{label} must be a canonical JSON object"
        ))),
    }
}

fn canonical_object_json(label: &str, value: &CanonicalValue) -> Result<String> {
    require_object_value(label, value)?;
    canonical_json_string(value)
}

fn parse_canonical_object_json(label: &str, input: &str) -> Result<CanonicalValue> {
    let parsed = parse_canonical_json(input.as_bytes()).map_err(|error| {
        WorkVcsError::EvidenceInvalid(format!("{label} is not valid canonical JSON: {error}"))
    })?;
    require_object_value(label, &parsed)?;
    let encoded = canonical_json_string(&parsed)?;
    if input != encoded {
        return Err(WorkVcsError::EvidenceInvalid(format!(
            "{label} is not stored in WorkVCS canonical JSON form"
        )));
    }
    Ok(parsed)
}

fn canonical_json_string(value: &CanonicalValue) -> Result<String> {
    String::from_utf8(canonical_bytes(value)?).map_err(|error| {
        WorkVcsError::CanonicalEncodingInvalid(format!("canonical JSON was not UTF-8: {error}"))
    })
}

fn decode_session_id(column: &str, bytes: Vec<u8>) -> Result<SessionId> {
    let bytes: [u8; 16] = bytes.try_into().map_err(|bytes: Vec<u8>| {
        WorkVcsError::EvidenceInvalid(format!("{column} must be 16 bytes, found {}", bytes.len()))
    })?;
    SessionId::from_bytes(bytes).map_err(|error| {
        WorkVcsError::EvidenceInvalid(format!("{column} is not a canonical UUIDv7: {error}"))
    })
}

fn decode_digest(column: &str, bytes: Vec<u8>) -> Result<Digest> {
    let bytes: [u8; 32] = bytes.try_into().map_err(|bytes: Vec<u8>| {
        WorkVcsError::EvidenceInvalid(format!("{column} must be 32 bytes, found {}", bytes.len()))
    })?;
    Ok(Digest::from_bytes(bytes))
}
