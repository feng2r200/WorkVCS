use crate::canonical::{
    CanonicalValue, canonical_bytes, content_object_digest, parse_canonical_json,
};
use crate::error::{Result, WorkVcsError, storage_error};
use crate::identity::{Digest, MigrationId};
use crate::store::{StoreConnection, current_epoch_micros};
use rusqlite::{OptionalExtension, TransactionBehavior, params};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoreMigrationRecordOptions {
    from_store_format_version: i64,
    to_store_format_version: i64,
    from_schema_version: i64,
    to_schema_version: i64,
    tool_version: String,
    outcome: String,
    detail: CanonicalValue,
}

impl StoreMigrationRecordOptions {
    pub fn new(
        from_store_format_version: i64,
        to_store_format_version: i64,
        from_schema_version: i64,
        to_schema_version: i64,
        tool_version: impl Into<String>,
        outcome: impl Into<String>,
        detail: CanonicalValue,
    ) -> Result<Self> {
        validate_positive_i64(
            "store migration from_store_format_version",
            from_store_format_version,
        )?;
        validate_positive_i64(
            "store migration to_store_format_version",
            to_store_format_version,
        )?;
        validate_positive_i64("store migration from_schema_version", from_schema_version)?;
        validate_positive_i64("store migration to_schema_version", to_schema_version)?;
        require_object_value("store migration detail", &detail)?;
        Ok(Self {
            from_store_format_version,
            to_store_format_version,
            from_schema_version,
            to_schema_version,
            tool_version: validate_text("store migration tool_version", tool_version)?,
            outcome: validate_text("store migration outcome", outcome)?,
            detail,
        })
    }

    fn source_store_format_version(&self) -> i64 {
        self.from_store_format_version
    }

    fn target_store_format_version(&self) -> i64 {
        self.to_store_format_version
    }

    fn source_schema_version(&self) -> i64 {
        self.from_schema_version
    }

    fn target_schema_version(&self) -> i64 {
        self.to_schema_version
    }

    fn tool_version(&self) -> &str {
        &self.tool_version
    }

    fn outcome(&self) -> &str {
        &self.outcome
    }

    fn detail(&self) -> &CanonicalValue {
        &self.detail
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoreMigrationRecordResult {
    pub migration: StoreMigrationAttemptSnapshot,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoreMigrationAttemptSnapshot {
    pub migration_id: MigrationId,
    pub from_store_format_version: i64,
    pub to_store_format_version: i64,
    pub from_schema_version: i64,
    pub to_schema_version: i64,
    pub tool_version: String,
    pub started_at_us: i64,
    pub outcome: Option<StoreMigrationOutcomeSnapshot>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoreMigrationOutcomeSnapshot {
    pub outcome: String,
    pub completed_at_us: i64,
    pub detail: CanonicalValue,
    pub detail_digest: Digest,
    pub detail_size_bytes: i64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StoreMigrationListOptions {
    limit: usize,
}

impl StoreMigrationListOptions {
    pub fn new() -> Self {
        Self { limit: 50 }
    }

    pub fn with_limit(mut self, limit: usize) -> Result<Self> {
        if limit == 0 {
            return Err(WorkVcsError::QueryInvalid(
                "store migration list limit must be positive".to_owned(),
            ));
        }
        self.limit = limit;
        Ok(self)
    }

    fn limit(self) -> usize {
        self.limit
    }
}

impl Default for StoreMigrationListOptions {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoreMigrationListResult {
    pub migrations: Vec<StoreMigrationAttemptSnapshot>,
}

pub(crate) fn record_store_migration(
    connection: &mut StoreConnection,
    options: StoreMigrationRecordOptions,
) -> Result<StoreMigrationRecordResult> {
    connection.verify_foreign_keys()?;
    let migration_id = MigrationId::new_v7();
    let now_us = current_epoch_micros()?;
    let detail_json = canonical_object_json("store migration detail", options.detail())?;
    let detail_digest = content_object_digest(detail_json.as_bytes());
    let detail_size_bytes = usize_to_i64("store migration detail_json size", detail_json.len())?;

    let migration_id_bytes = migration_id.raw_bytes();
    let transaction = connection
        .inner_mut()
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(storage_error)?;
    transaction
        .execute(
            "INSERT INTO store_migration_attempt(
                migration_id,
                from_store_format_version,
                to_store_format_version,
                from_schema_version,
                to_schema_version,
                tool_version,
                started_at_us
             )
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                &migration_id_bytes[..],
                options.source_store_format_version(),
                options.target_store_format_version(),
                options.source_schema_version(),
                options.target_schema_version(),
                options.tool_version(),
                now_us
            ],
        )
        .map_err(storage_error)?;
    transaction
        .execute(
            "INSERT INTO store_migration_outcome(
                migration_id,
                outcome,
                completed_at_us,
                detail_json
             )
             VALUES (?1, ?2, ?3, ?4)",
            params![
                &migration_id_bytes[..],
                options.outcome(),
                now_us,
                detail_json
            ],
        )
        .map_err(storage_error)?;
    transaction.commit().map_err(storage_error)?;

    Ok(StoreMigrationRecordResult {
        migration: StoreMigrationAttemptSnapshot {
            migration_id,
            from_store_format_version: options.source_store_format_version(),
            to_store_format_version: options.target_store_format_version(),
            from_schema_version: options.source_schema_version(),
            to_schema_version: options.target_schema_version(),
            tool_version: options.tool_version().to_owned(),
            started_at_us: now_us,
            outcome: Some(StoreMigrationOutcomeSnapshot {
                outcome: options.outcome().to_owned(),
                completed_at_us: now_us,
                detail: options.detail().clone(),
                detail_digest,
                detail_size_bytes,
            }),
        },
    })
}

pub(crate) fn store_migration(
    connection: &StoreConnection,
    migration_id: MigrationId,
) -> Result<StoreMigrationAttemptSnapshot> {
    connection.verify_foreign_keys()?;
    load_store_migration_snapshot(connection, migration_id)?.ok_or_else(|| {
        WorkVcsError::QueryInvalid(format!("StoreMigrationAttempt {migration_id} not found"))
    })
}

pub(crate) fn store_migrations(
    connection: &StoreConnection,
    options: StoreMigrationListOptions,
) -> Result<StoreMigrationListResult> {
    connection.verify_foreign_keys()?;
    let limit = usize_to_i64("store migration list limit", options.limit())?;
    let mut statement = connection
        .inner()
        .prepare(
            "SELECT migration_id
             FROM store_migration_attempt
             ORDER BY started_at_us DESC, migration_id DESC
             LIMIT ?1",
        )
        .map_err(storage_error)?;
    let rows = statement
        .query_map(params![limit], |row| row.get::<_, Vec<u8>>(0))
        .map_err(storage_error)?;

    let mut migrations = Vec::new();
    for row in rows {
        let migration_id = decode_migration_id(
            "store_migration_attempt.migration_id",
            row.map_err(storage_error)?,
        )?;
        migrations.push(store_migration(connection, migration_id)?);
    }
    Ok(StoreMigrationListResult { migrations })
}

fn load_store_migration_snapshot(
    connection: &StoreConnection,
    migration_id: MigrationId,
) -> Result<Option<StoreMigrationAttemptSnapshot>> {
    let row = connection
        .inner()
        .query_row(
            "SELECT from_store_format_version,
                    to_store_format_version,
                    from_schema_version,
                    to_schema_version,
                    tool_version,
                    started_at_us
             FROM store_migration_attempt
             WHERE migration_id = ?1",
            params![&migration_id.raw_bytes()[..]],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, i64>(5)?,
                ))
            },
        )
        .optional()
        .map_err(storage_error)?;
    let Some((
        from_store_format_version,
        to_store_format_version,
        from_schema_version,
        to_schema_version,
        tool_version,
        started_at_us,
    )) = row
    else {
        return Ok(None);
    };
    validate_positive_i64(
        "store_migration_attempt.from_store_format_version",
        from_store_format_version,
    )?;
    validate_positive_i64(
        "store_migration_attempt.to_store_format_version",
        to_store_format_version,
    )?;
    validate_positive_i64(
        "store_migration_attempt.from_schema_version",
        from_schema_version,
    )?;
    validate_positive_i64(
        "store_migration_attempt.to_schema_version",
        to_schema_version,
    )?;
    validate_stored_text("store_migration_attempt.tool_version", &tool_version)?;
    validate_positive_i64("store_migration_attempt.started_at_us", started_at_us)?;
    Ok(Some(StoreMigrationAttemptSnapshot {
        migration_id,
        from_store_format_version,
        to_store_format_version,
        from_schema_version,
        to_schema_version,
        tool_version,
        started_at_us,
        outcome: load_store_migration_outcome(connection, migration_id)?,
    }))
}

fn load_store_migration_outcome(
    connection: &StoreConnection,
    migration_id: MigrationId,
) -> Result<Option<StoreMigrationOutcomeSnapshot>> {
    let row = connection
        .inner()
        .query_row(
            "SELECT outcome,
                    completed_at_us,
                    detail_json
             FROM store_migration_outcome
             WHERE migration_id = ?1",
            params![&migration_id.raw_bytes()[..]],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, String>(2)?,
                ))
            },
        )
        .optional()
        .map_err(storage_error)?;
    let Some((outcome, completed_at_us, detail_json)) = row else {
        return Ok(None);
    };
    validate_stored_text("store_migration_outcome.outcome", &outcome)?;
    validate_positive_i64("store_migration_outcome.completed_at_us", completed_at_us)?;
    let detail = parse_canonical_object_json("store_migration_outcome.detail_json", &detail_json)?;
    Ok(Some(StoreMigrationOutcomeSnapshot {
        outcome,
        completed_at_us,
        detail,
        detail_digest: content_object_digest(detail_json.as_bytes()),
        detail_size_bytes: usize_to_i64(
            "store_migration_outcome.detail_json size",
            detail_json.len(),
        )?,
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

fn decode_migration_id(column: &str, bytes: Vec<u8>) -> Result<MigrationId> {
    let bytes = decode_16(column, bytes)?;
    MigrationId::from_bytes(bytes)
        .map_err(|error| WorkVcsError::QueryInvalid(format!("{column}: {error}")))
}

fn decode_16(column: &str, bytes: Vec<u8>) -> Result<[u8; 16]> {
    bytes.try_into().map_err(|bytes: Vec<u8>| {
        WorkVcsError::QueryInvalid(format!("{column} must be 16 bytes, found {}", bytes.len()))
    })
}
