use crate::canonical::{
    CanonicalValue, canonical_bytes, content_object_digest, parse_canonical_json,
};
use crate::error::{Result, WorkVcsError, storage_error};
use crate::identity::{Digest, LineageId, StoreId};
use crate::store::{StoreConnection, StoreInfo, current_epoch_micros};
use rusqlite::{OptionalExtension, TransactionBehavior, params};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoreLineageRecordOptions {
    source_store_id: StoreId,
    derivation_kind: String,
    source_root_descriptor: CanonicalValue,
    source_bundle_digest: Option<Digest>,
}

impl StoreLineageRecordOptions {
    pub fn new(
        source_store_id: StoreId,
        derivation_kind: impl Into<String>,
        source_root_descriptor: CanonicalValue,
    ) -> Result<Self> {
        let derivation_kind = validate_text("store lineage derivation_kind", derivation_kind)?;
        require_object_value(
            "store lineage source_root_descriptor",
            &source_root_descriptor,
        )?;
        Ok(Self {
            source_store_id,
            derivation_kind,
            source_root_descriptor,
            source_bundle_digest: None,
        })
    }

    pub fn with_source_bundle_digest(mut self, source_bundle_digest: Digest) -> Self {
        self.source_bundle_digest = Some(source_bundle_digest);
        self
    }

    fn source_store_id(&self) -> StoreId {
        self.source_store_id
    }

    fn derivation_kind(&self) -> &str {
        &self.derivation_kind
    }

    fn source_root_descriptor(&self) -> &CanonicalValue {
        &self.source_root_descriptor
    }

    fn source_bundle_digest(&self) -> Option<Digest> {
        self.source_bundle_digest
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoreLineageRecordResult {
    pub lineage: StoreLineageSnapshot,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoreLineageSnapshot {
    pub lineage_id: LineageId,
    pub source_store_id: StoreId,
    pub derivation_kind: String,
    pub source_root_descriptor: CanonicalValue,
    pub source_root_descriptor_digest: Digest,
    pub source_root_descriptor_size_bytes: i64,
    pub source_bundle_digest: Option<Digest>,
    pub created_at_us: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoreLineageListOptions {
    limit: usize,
    source_store_id: Option<StoreId>,
    derivation_kind: Option<String>,
    source_bundle_digest: Option<Digest>,
}

impl StoreLineageListOptions {
    pub fn new() -> Self {
        Self {
            limit: 50,
            source_store_id: None,
            derivation_kind: None,
            source_bundle_digest: None,
        }
    }

    pub fn with_limit(mut self, limit: usize) -> Result<Self> {
        if limit == 0 {
            return Err(WorkVcsError::QueryInvalid(
                "store lineage list limit must be positive".to_owned(),
            ));
        }
        self.limit = limit;
        Ok(self)
    }

    pub fn with_source_store_id(mut self, source_store_id: StoreId) -> Self {
        self.source_store_id = Some(source_store_id);
        self
    }

    pub fn with_derivation_kind(mut self, derivation_kind: impl Into<String>) -> Result<Self> {
        self.derivation_kind = Some(validate_text(
            "store lineage derivation_kind filter",
            derivation_kind,
        )?);
        Ok(self)
    }

    pub fn with_source_bundle_digest(mut self, source_bundle_digest: Digest) -> Self {
        self.source_bundle_digest = Some(source_bundle_digest);
        self
    }

    fn limit(&self) -> usize {
        self.limit
    }

    fn source_store_id(&self) -> Option<StoreId> {
        self.source_store_id
    }

    fn derivation_kind(&self) -> Option<&str> {
        self.derivation_kind.as_deref()
    }

    fn source_bundle_digest(&self) -> Option<Digest> {
        self.source_bundle_digest
    }
}

impl Default for StoreLineageListOptions {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoreLineageListResult {
    pub lineages: Vec<StoreLineageSnapshot>,
}

pub(crate) fn record_store_lineage(
    connection: &mut StoreConnection,
    store_info: &StoreInfo,
    options: StoreLineageRecordOptions,
) -> Result<StoreLineageRecordResult> {
    connection.verify_foreign_keys()?;
    if options.source_store_id() == store_info.store_id {
        return Err(WorkVcsError::QueryInvalid(
            "store lineage source_store_id must differ from local store_id".to_owned(),
        ));
    }

    let lineage_id = LineageId::new_v7();
    let created_at_us = current_epoch_micros()?;
    let source_root_descriptor_json = canonical_object_json(
        "store lineage source_root_descriptor",
        options.source_root_descriptor(),
    )?;
    let source_root_descriptor_digest =
        content_object_digest(source_root_descriptor_json.as_bytes());
    let source_root_descriptor_size_bytes = usize_to_i64(
        "store lineage source_root_descriptor_json size",
        source_root_descriptor_json.len(),
    )?;

    let lineage_id_bytes = lineage_id.raw_bytes();
    let source_store_id_bytes = options.source_store_id().raw_bytes();
    let source_bundle_digest_bytes = options
        .source_bundle_digest()
        .map(|digest| *digest.as_bytes());

    let transaction = connection
        .inner_mut()
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(storage_error)?;
    transaction
        .execute(
            "INSERT INTO store_lineage(
                lineage_id,
                source_store_id,
                derivation_kind,
                source_root_descriptor_json,
                source_bundle_digest,
                created_at_us
             )
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                &lineage_id_bytes[..],
                &source_store_id_bytes[..],
                options.derivation_kind(),
                source_root_descriptor_json,
                source_bundle_digest_bytes.as_ref().map(|bytes| &bytes[..]),
                created_at_us
            ],
        )
        .map_err(storage_error)?;
    transaction.commit().map_err(storage_error)?;

    Ok(StoreLineageRecordResult {
        lineage: StoreLineageSnapshot {
            lineage_id,
            source_store_id: options.source_store_id(),
            derivation_kind: options.derivation_kind().to_owned(),
            source_root_descriptor: options.source_root_descriptor().clone(),
            source_root_descriptor_digest,
            source_root_descriptor_size_bytes,
            source_bundle_digest: options.source_bundle_digest(),
            created_at_us,
        },
    })
}

pub(crate) fn store_lineage(
    connection: &StoreConnection,
    lineage_id: LineageId,
) -> Result<StoreLineageSnapshot> {
    connection.verify_foreign_keys()?;
    load_store_lineage_snapshot(connection, lineage_id)?
        .ok_or_else(|| WorkVcsError::QueryInvalid(format!("StoreLineage {lineage_id} not found")))
}

pub(crate) fn store_lineages(
    connection: &StoreConnection,
    options: StoreLineageListOptions,
) -> Result<StoreLineageListResult> {
    connection.verify_foreign_keys()?;
    let limit = usize_to_i64("store lineage list limit", options.limit())?;
    let source_store_id_bytes = options.source_store_id().map(|id| id.raw_bytes());
    let source_bundle_digest_bytes = options
        .source_bundle_digest()
        .map(|digest| *digest.as_bytes());
    let mut statement = connection
        .inner()
        .prepare(
            "SELECT lineage_id
             FROM store_lineage
             WHERE (?2 IS NULL OR source_store_id = ?2)
               AND (?3 IS NULL OR derivation_kind = ?3)
               AND (?4 IS NULL OR source_bundle_digest = ?4)
             ORDER BY created_at_us DESC, lineage_id DESC
             LIMIT ?1",
        )
        .map_err(storage_error)?;
    let rows = statement
        .query_map(
            params![
                limit,
                source_store_id_bytes.as_ref().map(|bytes| &bytes[..]),
                options.derivation_kind(),
                source_bundle_digest_bytes.as_ref().map(|bytes| &bytes[..])
            ],
            |row| row.get::<_, Vec<u8>>(0),
        )
        .map_err(storage_error)?;

    let mut lineages = Vec::new();
    for row in rows {
        let lineage_id =
            decode_lineage_id("store_lineage.lineage_id", row.map_err(storage_error)?)?;
        lineages.push(store_lineage(connection, lineage_id)?);
    }
    Ok(StoreLineageListResult { lineages })
}

fn load_store_lineage_snapshot(
    connection: &StoreConnection,
    lineage_id: LineageId,
) -> Result<Option<StoreLineageSnapshot>> {
    let row = connection
        .inner()
        .query_row(
            "SELECT source_store_id,
                    derivation_kind,
                    source_root_descriptor_json,
                    source_bundle_digest,
                    created_at_us
             FROM store_lineage
             WHERE lineage_id = ?1",
            params![&lineage_id.raw_bytes()[..]],
            |row| {
                Ok((
                    row.get::<_, Vec<u8>>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, Option<Vec<u8>>>(3)?,
                    row.get::<_, i64>(4)?,
                ))
            },
        )
        .optional()
        .map_err(storage_error)?;
    let Some((
        source_store_id,
        derivation_kind,
        source_root_descriptor_json,
        source_bundle_digest,
        created_at_us,
    )) = row
    else {
        return Ok(None);
    };
    validate_stored_text("store_lineage.derivation_kind", &derivation_kind)?;
    validate_positive_i64("store_lineage.created_at_us", created_at_us)?;
    let source_root_descriptor = parse_canonical_object_json(
        "store_lineage.source_root_descriptor_json",
        &source_root_descriptor_json,
    )?;
    Ok(Some(StoreLineageSnapshot {
        lineage_id,
        source_store_id: decode_store_id("store_lineage.source_store_id", source_store_id)?,
        derivation_kind,
        source_root_descriptor,
        source_root_descriptor_digest: content_object_digest(
            source_root_descriptor_json.as_bytes(),
        ),
        source_root_descriptor_size_bytes: usize_to_i64(
            "store_lineage.source_root_descriptor_json size",
            source_root_descriptor_json.len(),
        )?,
        source_bundle_digest: decode_optional_digest(
            "store_lineage.source_bundle_digest",
            source_bundle_digest,
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

fn decode_lineage_id(column: &str, bytes: Vec<u8>) -> Result<LineageId> {
    let bytes = decode_16(column, bytes)?;
    LineageId::from_bytes(bytes)
        .map_err(|error| WorkVcsError::QueryInvalid(format!("{column}: {error}")))
}

fn decode_optional_digest(column: &str, bytes: Option<Vec<u8>>) -> Result<Option<Digest>> {
    bytes.map(|bytes| decode_digest(column, bytes)).transpose()
}

fn decode_digest(column: &str, bytes: Vec<u8>) -> Result<Digest> {
    let bytes: [u8; 32] = bytes.try_into().map_err(|bytes: Vec<u8>| {
        WorkVcsError::QueryInvalid(format!("{column} must be 32 bytes, found {}", bytes.len()))
    })?;
    Ok(Digest::from_bytes(bytes))
}

fn decode_16(column: &str, bytes: Vec<u8>) -> Result<[u8; 16]> {
    bytes.try_into().map_err(|bytes: Vec<u8>| {
        WorkVcsError::QueryInvalid(format!("{column} must be 16 bytes, found {}", bytes.len()))
    })
}
