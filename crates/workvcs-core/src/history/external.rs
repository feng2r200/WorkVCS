use crate::canonical::{
    CanonicalValue, canonical_bytes, content_object_digest, parse_canonical_json,
};
use crate::error::{Result, WorkVcsError, storage_error};
use crate::identity::{Digest, ExternalObjectId, ExternalRefId, ExternalVersionId, StoreId};
use crate::store::{StoreConnection, StoreInfo, current_epoch_micros};
use rusqlite::{OptionalExtension, TransactionBehavior, params};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExternalObjectReferenceScope {
    Object,
    Version,
}

impl ExternalObjectReferenceScope {
    pub fn parse(value: &str) -> Result<Self> {
        match value {
            "object" => Ok(Self::Object),
            "version" => Ok(Self::Version),
            other => Err(WorkVcsError::QueryInvalid(format!(
                "external object reference scope {other:?} is not supported"
            ))),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Object => "object",
            Self::Version => "version",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExternalObjectRefRecordOptions {
    external_store_id: StoreId,
    external_object_id: ExternalObjectId,
    object_kind: String,
    reference_scope: ExternalObjectReferenceScope,
    external_version_ref: Option<ExternalVersionId>,
    descriptor: CanonicalValue,
}

impl ExternalObjectRefRecordOptions {
    pub fn for_object(
        external_store_id: StoreId,
        external_object_id: ExternalObjectId,
        object_kind: impl Into<String>,
        descriptor: CanonicalValue,
    ) -> Result<Self> {
        Self::new(
            external_store_id,
            external_object_id,
            object_kind,
            ExternalObjectReferenceScope::Object,
            None,
            descriptor,
        )
    }

    pub fn for_version(
        external_store_id: StoreId,
        external_object_id: ExternalObjectId,
        object_kind: impl Into<String>,
        external_version_ref: ExternalVersionId,
        descriptor: CanonicalValue,
    ) -> Result<Self> {
        Self::new(
            external_store_id,
            external_object_id,
            object_kind,
            ExternalObjectReferenceScope::Version,
            Some(external_version_ref),
            descriptor,
        )
    }

    fn new(
        external_store_id: StoreId,
        external_object_id: ExternalObjectId,
        object_kind: impl Into<String>,
        reference_scope: ExternalObjectReferenceScope,
        external_version_ref: Option<ExternalVersionId>,
        descriptor: CanonicalValue,
    ) -> Result<Self> {
        match (reference_scope, external_version_ref) {
            (ExternalObjectReferenceScope::Object, None)
            | (ExternalObjectReferenceScope::Version, Some(_)) => {}
            (ExternalObjectReferenceScope::Object, Some(_)) => {
                return Err(WorkVcsError::QueryInvalid(
                    "object-scope external refs cannot include external_version_ref".to_owned(),
                ));
            }
            (ExternalObjectReferenceScope::Version, None) => {
                return Err(WorkVcsError::QueryInvalid(
                    "version-scope external refs require external_version_ref".to_owned(),
                ));
            }
        }
        require_object_value("external object ref descriptor", &descriptor)?;
        Ok(Self {
            external_store_id,
            external_object_id,
            object_kind: validate_text("external object ref object_kind", object_kind)?,
            reference_scope,
            external_version_ref,
            descriptor,
        })
    }

    fn external_store_id(&self) -> StoreId {
        self.external_store_id
    }

    fn external_object_id(&self) -> ExternalObjectId {
        self.external_object_id
    }

    fn object_kind(&self) -> &str {
        &self.object_kind
    }

    fn reference_scope(&self) -> ExternalObjectReferenceScope {
        self.reference_scope
    }

    fn external_version_ref(&self) -> Option<ExternalVersionId> {
        self.external_version_ref
    }

    fn descriptor(&self) -> &CanonicalValue {
        &self.descriptor
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExternalObjectRefRecordResult {
    pub external_ref: ExternalObjectRefSnapshot,
    pub created: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExternalObjectRefSnapshot {
    pub external_ref_id: ExternalRefId,
    pub external_store_id: StoreId,
    pub external_object_id: ExternalObjectId,
    pub object_kind: String,
    pub reference_scope: ExternalObjectReferenceScope,
    pub external_version_ref: Option<ExternalVersionId>,
    pub descriptor: CanonicalValue,
    pub descriptor_digest: Digest,
    pub descriptor_size_bytes: i64,
    pub created_at_us: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExternalObjectRefListOptions {
    limit: usize,
    external_store_id: Option<StoreId>,
    object_kind: Option<String>,
    reference_scope: Option<ExternalObjectReferenceScope>,
}

impl ExternalObjectRefListOptions {
    pub fn new() -> Self {
        Self {
            limit: 50,
            external_store_id: None,
            object_kind: None,
            reference_scope: None,
        }
    }

    pub fn with_limit(mut self, limit: usize) -> Result<Self> {
        if limit == 0 {
            return Err(WorkVcsError::QueryInvalid(
                "external object ref list limit must be positive".to_owned(),
            ));
        }
        self.limit = limit;
        Ok(self)
    }

    pub fn with_external_store_id(mut self, external_store_id: StoreId) -> Self {
        self.external_store_id = Some(external_store_id);
        self
    }

    pub fn with_object_kind(mut self, object_kind: impl Into<String>) -> Result<Self> {
        self.object_kind = Some(validate_text(
            "external object ref object_kind filter",
            object_kind,
        )?);
        Ok(self)
    }

    pub fn with_reference_scope(mut self, reference_scope: ExternalObjectReferenceScope) -> Self {
        self.reference_scope = Some(reference_scope);
        self
    }

    fn limit(&self) -> usize {
        self.limit
    }

    fn external_store_id(&self) -> Option<StoreId> {
        self.external_store_id
    }

    fn object_kind(&self) -> Option<&str> {
        self.object_kind.as_deref()
    }

    fn reference_scope(&self) -> Option<ExternalObjectReferenceScope> {
        self.reference_scope
    }
}

impl Default for ExternalObjectRefListOptions {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExternalObjectRefListResult {
    pub external_refs: Vec<ExternalObjectRefSnapshot>,
}

pub(crate) fn record_external_object_ref(
    connection: &mut StoreConnection,
    store_info: &StoreInfo,
    options: ExternalObjectRefRecordOptions,
) -> Result<ExternalObjectRefRecordResult> {
    connection.verify_foreign_keys()?;
    if options.external_store_id() == store_info.store_id {
        return Err(WorkVcsError::QueryInvalid(
            "external object ref store_id must differ from local store_id".to_owned(),
        ));
    }
    let descriptor_json =
        canonical_object_json("external object ref descriptor", options.descriptor())?;
    if let Some(existing) = find_existing_external_object_ref(connection, &options)? {
        if existing.object_kind != options.object_kind()
            || existing.reference_scope != options.reference_scope()
            || existing.external_version_ref != options.external_version_ref()
            || existing.descriptor != *options.descriptor()
        {
            return Err(WorkVcsError::QueryInvalid(
                "external object ref conflicts with an existing reference key".to_owned(),
            ));
        }
        return Ok(ExternalObjectRefRecordResult {
            external_ref: existing,
            created: false,
        });
    }

    let external_ref_id = ExternalRefId::new_v7();
    let created_at_us = current_epoch_micros()?;
    let external_ref_id_bytes = external_ref_id.raw_bytes();
    let external_store_id_bytes = options.external_store_id().raw_bytes();
    let external_object_id_bytes = options.external_object_id().raw_bytes();
    let external_version_ref_bytes = options.external_version_ref().map(|id| id.raw_bytes());
    let descriptor_digest = content_object_digest(descriptor_json.as_bytes());
    let descriptor_size_bytes = usize_to_i64(
        "external object ref descriptor_json size",
        descriptor_json.len(),
    )?;

    let transaction = connection
        .inner_mut()
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(storage_error)?;
    transaction
        .execute(
            "INSERT INTO external_object_ref(
                external_ref_id,
                external_store_id,
                external_object_id,
                object_kind,
                reference_scope,
                external_version_ref,
                descriptor_json,
                created_at_us
             )
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                &external_ref_id_bytes[..],
                &external_store_id_bytes[..],
                &external_object_id_bytes[..],
                options.object_kind(),
                options.reference_scope().as_str(),
                external_version_ref_bytes.as_ref().map(|bytes| &bytes[..]),
                descriptor_json,
                created_at_us
            ],
        )
        .map_err(storage_error)?;
    transaction.commit().map_err(storage_error)?;

    Ok(ExternalObjectRefRecordResult {
        external_ref: ExternalObjectRefSnapshot {
            external_ref_id,
            external_store_id: options.external_store_id(),
            external_object_id: options.external_object_id(),
            object_kind: options.object_kind().to_owned(),
            reference_scope: options.reference_scope(),
            external_version_ref: options.external_version_ref(),
            descriptor: options.descriptor().clone(),
            descriptor_digest,
            descriptor_size_bytes,
            created_at_us,
        },
        created: true,
    })
}

pub(crate) fn external_object_ref(
    connection: &StoreConnection,
    external_ref_id: ExternalRefId,
) -> Result<ExternalObjectRefSnapshot> {
    connection.verify_foreign_keys()?;
    load_external_object_ref_snapshot(connection, external_ref_id)?.ok_or_else(|| {
        WorkVcsError::QueryInvalid(format!("ExternalObjectRef {external_ref_id} not found"))
    })
}

pub(crate) fn external_object_refs(
    connection: &StoreConnection,
    options: ExternalObjectRefListOptions,
) -> Result<ExternalObjectRefListResult> {
    connection.verify_foreign_keys()?;
    let limit = usize_to_i64("external object ref list limit", options.limit())?;
    let external_store_id_bytes = options.external_store_id().map(|id| id.raw_bytes());
    let reference_scope = options
        .reference_scope()
        .map(ExternalObjectReferenceScope::as_str);
    let mut statement = connection
        .inner()
        .prepare(
            "SELECT external_ref_id
             FROM external_object_ref
             WHERE (?2 IS NULL OR external_store_id = ?2)
               AND (?3 IS NULL OR object_kind = ?3)
               AND (?4 IS NULL OR reference_scope = ?4)
             ORDER BY created_at_us DESC, external_ref_id DESC
             LIMIT ?1",
        )
        .map_err(storage_error)?;
    let rows = statement
        .query_map(
            params![
                limit,
                external_store_id_bytes.as_ref().map(|bytes| &bytes[..]),
                options.object_kind(),
                reference_scope
            ],
            |row| row.get::<_, Vec<u8>>(0),
        )
        .map_err(storage_error)?;

    let mut external_refs = Vec::new();
    for row in rows {
        let external_ref_id = decode_external_ref_id(
            "external_object_ref.external_ref_id",
            row.map_err(storage_error)?,
        )?;
        external_refs.push(external_object_ref(connection, external_ref_id)?);
    }
    Ok(ExternalObjectRefListResult { external_refs })
}

fn find_existing_external_object_ref(
    connection: &StoreConnection,
    options: &ExternalObjectRefRecordOptions,
) -> Result<Option<ExternalObjectRefSnapshot>> {
    let external_store_id_bytes = options.external_store_id().raw_bytes();
    let external_object_id_bytes = options.external_object_id().raw_bytes();
    let external_version_ref_bytes = options.external_version_ref().map(|id| id.raw_bytes());
    let row = connection
        .inner()
        .query_row(
            "SELECT external_ref_id
             FROM external_object_ref
             WHERE external_store_id = ?1
               AND external_object_id = ?2
               AND reference_scope = ?3
               AND (
                    (?4 IS NULL AND external_version_ref IS NULL)
                    OR external_version_ref = ?4
               )",
            params![
                &external_store_id_bytes[..],
                &external_object_id_bytes[..],
                options.reference_scope().as_str(),
                external_version_ref_bytes.as_ref().map(|bytes| &bytes[..])
            ],
            |row| row.get::<_, Vec<u8>>(0),
        )
        .optional()
        .map_err(storage_error)?;
    row.map(|bytes| {
        let external_ref_id = decode_external_ref_id("external_object_ref.external_ref_id", bytes)?;
        external_object_ref(connection, external_ref_id)
    })
    .transpose()
}

fn load_external_object_ref_snapshot(
    connection: &StoreConnection,
    external_ref_id: ExternalRefId,
) -> Result<Option<ExternalObjectRefSnapshot>> {
    let row = connection
        .inner()
        .query_row(
            "SELECT external_store_id,
                    external_object_id,
                    object_kind,
                    reference_scope,
                    external_version_ref,
                    descriptor_json,
                    created_at_us
             FROM external_object_ref
             WHERE external_ref_id = ?1",
            params![&external_ref_id.raw_bytes()[..]],
            |row| {
                Ok((
                    row.get::<_, Vec<u8>>(0)?,
                    row.get::<_, Vec<u8>>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, Option<Vec<u8>>>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, i64>(6)?,
                ))
            },
        )
        .optional()
        .map_err(storage_error)?;
    let Some((
        external_store_id,
        external_object_id,
        object_kind,
        reference_scope,
        external_version_ref,
        descriptor_json,
        created_at_us,
    )) = row
    else {
        return Ok(None);
    };
    validate_stored_text("external_object_ref.object_kind", &object_kind)?;
    validate_positive_i64("external_object_ref.created_at_us", created_at_us)?;
    let reference_scope = ExternalObjectReferenceScope::parse(&reference_scope)?;
    let external_version_ref = decode_optional_external_version_id(
        "external_object_ref.external_version_ref",
        external_version_ref,
    )?;
    match (reference_scope, external_version_ref) {
        (ExternalObjectReferenceScope::Object, None)
        | (ExternalObjectReferenceScope::Version, Some(_)) => {}
        (ExternalObjectReferenceScope::Object, Some(_)) => {
            return Err(WorkVcsError::QueryInvalid(
                "external_object_ref object scope has external_version_ref".to_owned(),
            ));
        }
        (ExternalObjectReferenceScope::Version, None) => {
            return Err(WorkVcsError::QueryInvalid(
                "external_object_ref version scope lacks external_version_ref".to_owned(),
            ));
        }
    }
    let descriptor =
        parse_canonical_object_json("external_object_ref.descriptor_json", &descriptor_json)?;
    Ok(Some(ExternalObjectRefSnapshot {
        external_ref_id,
        external_store_id: decode_store_id(
            "external_object_ref.external_store_id",
            external_store_id,
        )?,
        external_object_id: decode_external_object_id(
            "external_object_ref.external_object_id",
            external_object_id,
        )?,
        object_kind,
        reference_scope,
        external_version_ref,
        descriptor,
        descriptor_digest: content_object_digest(descriptor_json.as_bytes()),
        descriptor_size_bytes: usize_to_i64(
            "external_object_ref.descriptor_json size",
            descriptor_json.len(),
        )?,
        created_at_us,
    }))
}

fn canonical_object_json(label: &str, value: &CanonicalValue) -> Result<String> {
    require_object_value(label, value)?;
    let bytes = canonical_bytes(value)?;
    String::from_utf8(bytes).map_err(|error| {
        WorkVcsError::CanonicalEncodingInvalid(format!("canonical JSON was not UTF-8: {error}"))
    })
}

fn parse_canonical_object_json(label: &str, input: &str) -> Result<CanonicalValue> {
    let value = parse_canonical_json(input.as_bytes())?;
    require_object_value(label, &value)?;
    let reencoded = canonical_object_json(label, &value)?;
    if reencoded != input {
        return Err(WorkVcsError::QueryInvalid(format!(
            "{label} is not canonical JSON"
        )));
    }
    Ok(value)
}

fn require_object_value(label: &str, value: &CanonicalValue) -> Result<()> {
    match value {
        CanonicalValue::Object(_) => Ok(()),
        _ => Err(WorkVcsError::QueryInvalid(format!(
            "{label} must be an object"
        ))),
    }
}

fn validate_text(label: &str, value: impl Into<String>) -> Result<String> {
    let value = value.into();
    validate_stored_text(label, &value)?;
    Ok(value)
}

fn validate_stored_text(label: &str, value: &str) -> Result<()> {
    if value.is_empty() {
        return Err(WorkVcsError::QueryInvalid(format!(
            "{label} cannot be empty"
        )));
    }
    if value.chars().any(char::is_control) {
        return Err(WorkVcsError::QueryInvalid(format!(
            "{label} cannot contain control characters"
        )));
    }
    Ok(())
}

fn validate_positive_i64(label: &str, value: i64) -> Result<()> {
    if value <= 0 {
        return Err(WorkVcsError::QueryInvalid(format!(
            "{label} must be positive"
        )));
    }
    Ok(())
}

fn usize_to_i64(label: &str, value: usize) -> Result<i64> {
    i64::try_from(value)
        .map_err(|_| WorkVcsError::QueryInvalid(format!("{label} does not fit i64")))
}

fn decode_store_id(column: &str, bytes: Vec<u8>) -> Result<StoreId> {
    let bytes = decode_16(column, bytes)?;
    StoreId::from_bytes(bytes)
        .map_err(|error| WorkVcsError::QueryInvalid(format!("{column}: {error}")))
}

fn decode_external_ref_id(column: &str, bytes: Vec<u8>) -> Result<ExternalRefId> {
    let bytes = decode_16(column, bytes)?;
    ExternalRefId::from_bytes(bytes)
        .map_err(|error| WorkVcsError::QueryInvalid(format!("{column}: {error}")))
}

fn decode_external_object_id(column: &str, bytes: Vec<u8>) -> Result<ExternalObjectId> {
    let bytes = decode_16(column, bytes)?;
    ExternalObjectId::from_bytes(bytes)
        .map_err(|error| WorkVcsError::QueryInvalid(format!("{column}: {error}")))
}

fn decode_optional_external_version_id(
    column: &str,
    bytes: Option<Vec<u8>>,
) -> Result<Option<ExternalVersionId>> {
    bytes
        .map(|bytes| {
            let bytes = decode_16(column, bytes)?;
            ExternalVersionId::from_bytes(bytes)
                .map_err(|error| WorkVcsError::QueryInvalid(format!("{column}: {error}")))
        })
        .transpose()
}

fn decode_16(column: &str, bytes: Vec<u8>) -> Result<[u8; 16]> {
    bytes.try_into().map_err(|bytes: Vec<u8>| {
        WorkVcsError::QueryInvalid(format!("{column} must be 16 bytes, found {}", bytes.len()))
    })
}
