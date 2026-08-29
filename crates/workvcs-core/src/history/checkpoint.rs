use crate::canonical::{
    CanonicalValue, WorkState, canonical_bytes, content_object_digest, parse_canonical_json,
    work_state_mapping_digest,
};
use crate::error::{Result, WorkVcsError, storage_error};
use crate::identity::{
    CheckpointId, CommitId, Digest, EntityId, EntityVersionId, RelationId, RelationVersionId,
    WorkspaceId,
};
use crate::store::{StoreConnection, current_epoch_micros};
use rusqlite::{OptionalExtension, Transaction, TransactionBehavior, params};

const CHECKPOINT_FORMAT: &str = "workvcs-workstate-checkpoint-v1";
const CHECKPOINT_FORMAT_VERSION: i64 = 1;
const CHECKPOINT_MEDIA_TYPE: &str = "application/vnd.workvcs.workstate-checkpoint+json";
const CONTENT_OBJECT_KIND: &str = "checkpoint content object";
const USABILITY_STATE_INVALID: &str = "invalid";
const USABILITY_STATE_USABLE: &str = "usable";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CheckpointCreateOptions {
    commit_id: CommitId,
}

impl CheckpointCreateOptions {
    pub fn new(commit_id: CommitId) -> Self {
        Self { commit_id }
    }

    pub fn commit_id(self) -> CommitId {
        self.commit_id
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CheckpointCreateResult {
    pub checkpoint: CheckpointSnapshot,
    pub entity_count: usize,
    pub relation_count: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CheckpointSnapshot {
    pub checkpoint_id: CheckpointId,
    pub workspace_id: WorkspaceId,
    pub commit_id: CommitId,
    pub state_digest: Digest,
    pub checkpoint_format_version: i64,
    pub content_digest: Digest,
    pub content_size_bytes: i64,
    pub media_type: Option<String>,
    pub format_metadata: CanonicalValue,
    pub created_at_us: i64,
    pub usability_state: String,
    pub last_validated_at_us: i64,
    pub status_detail: CanonicalValue,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CheckpointValidationResult {
    pub checkpoint: CheckpointSnapshot,
    pub valid: bool,
    pub expected_state_digest: Digest,
    pub expected_content_digest: Digest,
    pub expected_content_size_bytes: i64,
    pub problem: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CheckpointListOptions {
    commit_id: CommitId,
}

impl CheckpointListOptions {
    pub fn for_commit(commit_id: CommitId) -> Self {
        Self { commit_id }
    }

    pub fn commit_id(self) -> CommitId {
        self.commit_id
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CheckpointListResult {
    pub commit_id: CommitId,
    pub checkpoints: Vec<CheckpointSnapshot>,
}

pub(crate) fn create_checkpoint(
    connection: &mut StoreConnection,
    options: CheckpointCreateOptions,
) -> Result<CheckpointCreateResult> {
    connection.verify_foreign_keys()?;
    let replayed = super::state_at(connection, options.commit_id())?;
    let rebuilt_digest = work_state_mapping_digest(&replayed.state);
    if rebuilt_digest != replayed.state_digest {
        return Err(WorkVcsError::ReplayInvalid(format!(
            "commit {} replayed WorkState digest does not match checkpoint input",
            replayed.commit_id
        )));
    }

    let checkpoint_content_bytes = checkpoint_content_bytes(&replayed)?;
    let content_digest = content_object_digest(&checkpoint_content_bytes);
    let content_size_bytes = i64::try_from(checkpoint_content_bytes.len()).map_err(|_| {
        WorkVcsError::QueryInvalid(
            "checkpoint content is too large for SQLite size_bytes".to_owned(),
        )
    })?;
    let checkpoint_id = CheckpointId::new_v7();
    let created_at_us = current_epoch_micros()?;
    let format_metadata_json = canonical_object_json(
        "checkpoint content format metadata",
        &checkpoint_format_metadata()?,
    )?;
    let status_detail_json = canonical_object_json(
        "checkpoint status detail",
        &checkpoint_status_detail(
            &replayed.state,
            replayed.state_digest,
            content_digest,
            content_size_bytes,
        )?,
    )?;

    let transaction = connection
        .inner_mut()
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(storage_error)?;
    ensure_checkpoint_content_object(
        &transaction,
        content_digest,
        content_size_bytes,
        Some(CHECKPOINT_MEDIA_TYPE),
        &format_metadata_json,
    )?;

    transaction
        .execute(
            "INSERT INTO checkpoint(
                checkpoint_id,
                workspace_id,
                commit_id,
                state_digest,
                checkpoint_format_version,
                content_digest,
                created_at_us
             )
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                &checkpoint_id.raw_bytes()[..],
                &replayed.workspace_id.raw_bytes()[..],
                &replayed.commit_id.raw_bytes()[..],
                &replayed.state_digest.as_bytes()[..],
                CHECKPOINT_FORMAT_VERSION,
                &content_digest.as_bytes()[..],
                created_at_us
            ],
        )
        .map_err(storage_error)?;

    transaction
        .execute(
            "INSERT INTO checkpoint_status(
                checkpoint_id,
                usability_state,
                last_validated_at_us,
                detail_json
             )
             VALUES (?1, ?2, ?3, ?4)",
            params![
                &checkpoint_id.raw_bytes()[..],
                USABILITY_STATE_USABLE,
                created_at_us,
                status_detail_json
            ],
        )
        .map_err(storage_error)?;
    transaction.commit().map_err(storage_error)?;

    Ok(CheckpointCreateResult {
        checkpoint: load_checkpoint(connection, checkpoint_id)?,
        entity_count: replayed.state.entities().len(),
        relation_count: replayed.state.relations().len(),
    })
}

pub(crate) fn checkpoint(
    connection: &StoreConnection,
    checkpoint_id: CheckpointId,
) -> Result<CheckpointSnapshot> {
    connection.verify_foreign_keys()?;
    load_checkpoint(connection, checkpoint_id)
}

pub(crate) fn validate_checkpoint(
    connection: &mut StoreConnection,
    checkpoint_id: CheckpointId,
) -> Result<CheckpointValidationResult> {
    connection.verify_foreign_keys()?;
    let current = load_checkpoint(connection, checkpoint_id)?;
    let replayed = super::state_at(connection, current.commit_id)?;
    let checkpoint_content_bytes = checkpoint_content_bytes(&replayed)?;
    let expected_content_digest = content_object_digest(&checkpoint_content_bytes);
    let expected_content_size_bytes =
        i64::try_from(checkpoint_content_bytes.len()).map_err(|_| {
            WorkVcsError::QueryInvalid(
                "checkpoint content is too large for SQLite size_bytes".to_owned(),
            )
        })?;
    let expected_format_metadata = checkpoint_format_metadata()?;
    let problem = checkpoint_validation_problem(
        &current,
        &replayed,
        expected_content_digest,
        expected_content_size_bytes,
        &expected_format_metadata,
    );
    let valid = problem.is_none();
    let usability_state = if valid {
        USABILITY_STATE_USABLE
    } else {
        USABILITY_STATE_INVALID
    };
    let last_validated_at_us = current_epoch_micros()?;
    let status_detail_json = canonical_object_json(
        "checkpoint validation detail",
        &checkpoint_validation_status_detail(
            valid,
            problem.as_deref(),
            replayed.state_digest,
            expected_content_digest,
            expected_content_size_bytes,
        )?,
    )?;

    let transaction = connection
        .inner_mut()
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(storage_error)?;
    let updated = transaction
        .execute(
            "UPDATE checkpoint_status
             SET usability_state = ?2,
                 last_validated_at_us = ?3,
                 detail_json = ?4
             WHERE checkpoint_id = ?1",
            params![
                &checkpoint_id.raw_bytes()[..],
                usability_state,
                last_validated_at_us,
                status_detail_json
            ],
        )
        .map_err(storage_error)?;
    if updated != 1 {
        return Err(WorkVcsError::QueryInvalid(format!(
            "checkpoint {checkpoint_id} has no checkpoint_status row"
        )));
    }
    transaction.commit().map_err(storage_error)?;

    Ok(CheckpointValidationResult {
        checkpoint: load_checkpoint(connection, checkpoint_id)?,
        valid,
        expected_state_digest: replayed.state_digest,
        expected_content_digest,
        expected_content_size_bytes,
        problem,
    })
}

pub(crate) fn checkpoints(
    connection: &StoreConnection,
    options: CheckpointListOptions,
) -> Result<CheckpointListResult> {
    connection.verify_foreign_keys()?;
    let checkpoint_ids = checkpoint_ids_for_commit(connection, options.commit_id())?;
    let checkpoints = checkpoint_ids
        .into_iter()
        .map(|checkpoint_id| load_checkpoint(connection, checkpoint_id))
        .collect::<Result<Vec<_>>>()?;
    Ok(CheckpointListResult {
        commit_id: options.commit_id(),
        checkpoints,
    })
}

fn checkpoint_content_bytes(replayed: &super::ReplayedState) -> Result<Vec<u8>> {
    canonical_bytes(&CanonicalValue::object(vec![
        string_field("checkpoint_format", CHECKPOINT_FORMAT),
        integer_field("checkpoint_format_version", CHECKPOINT_FORMAT_VERSION)?,
        string_field("commit_id", replayed.commit_id.to_string()),
        string_field("state_digest", replayed.state_digest.to_string()),
        (
            "work_state".to_owned(),
            CanonicalValue::object(vec![
                ("entities".to_owned(), entity_entries(&replayed.state)?),
                ("relations".to_owned(), relation_entries(&replayed.state)?),
            ])?,
        ),
        string_field("workspace_id", replayed.workspace_id.to_string()),
    ])?)
}

fn entity_entries(state: &WorkState) -> Result<CanonicalValue> {
    let mut entries = state.entities().to_vec();
    entries.sort_by_key(|(entity_id, _)| entity_id.raw_bytes());
    Ok(CanonicalValue::Array(
        entries
            .into_iter()
            .map(|(entity_id, entity_version_id)| entity_entry(entity_id, entity_version_id))
            .collect::<Result<Vec<_>>>()?,
    ))
}

fn entity_entry(entity_id: EntityId, entity_version_id: EntityVersionId) -> Result<CanonicalValue> {
    CanonicalValue::object(vec![
        string_field("entity_id", entity_id.to_string()),
        string_field("entity_version_id", entity_version_id.to_string()),
    ])
}

fn relation_entries(state: &WorkState) -> Result<CanonicalValue> {
    let mut entries = state.relations().to_vec();
    entries.sort_by_key(|(relation_id, _)| relation_id.raw_bytes());
    Ok(CanonicalValue::Array(
        entries
            .into_iter()
            .map(|(relation_id, relation_version_id)| {
                relation_entry(relation_id, relation_version_id)
            })
            .collect::<Result<Vec<_>>>()?,
    ))
}

fn relation_entry(
    relation_id: RelationId,
    relation_version_id: RelationVersionId,
) -> Result<CanonicalValue> {
    CanonicalValue::object(vec![
        string_field("relation_id", relation_id.to_string()),
        string_field("relation_version_id", relation_version_id.to_string()),
    ])
}

fn checkpoint_format_metadata() -> Result<CanonicalValue> {
    CanonicalValue::object(vec![
        string_field("checkpoint_format", CHECKPOINT_FORMAT),
        integer_field("checkpoint_format_version", CHECKPOINT_FORMAT_VERSION)?,
    ])
}

fn checkpoint_status_detail(
    state: &WorkState,
    state_digest: Digest,
    content_digest: Digest,
    content_size_bytes: i64,
) -> Result<CanonicalValue> {
    CanonicalValue::object(vec![
        string_field("validation", "replayed_work_state_digest_match"),
        string_field("state_digest", state_digest.to_string()),
        string_field("content_digest", content_digest.to_string()),
        integer_field("content_size_bytes", content_size_bytes)?,
        integer_field(
            "entity_count",
            usize_to_i64("entity_count", state.entities().len())?,
        )?,
        integer_field(
            "relation_count",
            usize_to_i64("relation_count", state.relations().len())?,
        )?,
    ])
}

fn checkpoint_validation_problem(
    snapshot: &CheckpointSnapshot,
    replayed: &super::ReplayedState,
    expected_content_digest: Digest,
    expected_content_size_bytes: i64,
    expected_format_metadata: &CanonicalValue,
) -> Option<String> {
    if snapshot.workspace_id != replayed.workspace_id {
        return Some(format!(
            "checkpoint workspace {} does not match replayed workspace {}",
            snapshot.workspace_id, replayed.workspace_id
        ));
    }
    if snapshot.state_digest != replayed.state_digest {
        return Some(format!(
            "checkpoint state digest {} does not match replayed state digest {}",
            snapshot.state_digest, replayed.state_digest
        ));
    }
    if snapshot.checkpoint_format_version != CHECKPOINT_FORMAT_VERSION {
        return Some(format!(
            "checkpoint format version {} is not supported version {}",
            snapshot.checkpoint_format_version, CHECKPOINT_FORMAT_VERSION
        ));
    }
    if snapshot.content_digest != expected_content_digest {
        return Some(format!(
            "checkpoint content digest {} does not match rebuilt content digest {}",
            snapshot.content_digest, expected_content_digest
        ));
    }
    if snapshot.content_size_bytes != expected_content_size_bytes {
        return Some(format!(
            "checkpoint content size {} does not match rebuilt content size {}",
            snapshot.content_size_bytes, expected_content_size_bytes
        ));
    }
    if snapshot.media_type.as_deref() != Some(CHECKPOINT_MEDIA_TYPE) {
        return Some(format!(
            "checkpoint content media type {:?} does not match expected media type {}",
            snapshot.media_type, CHECKPOINT_MEDIA_TYPE
        ));
    }
    if &snapshot.format_metadata != expected_format_metadata {
        return Some(
            "checkpoint content format metadata does not match expected metadata".to_owned(),
        );
    }
    None
}

fn checkpoint_validation_status_detail(
    valid: bool,
    problem: Option<&str>,
    expected_state_digest: Digest,
    expected_content_digest: Digest,
    expected_content_size_bytes: i64,
) -> Result<CanonicalValue> {
    CanonicalValue::object(vec![
        string_field("validation", "checkpoint_rebuild_validation"),
        ("valid".to_owned(), CanonicalValue::Bool(valid)),
        (
            "problem".to_owned(),
            problem
                .map(|message| CanonicalValue::String(message.to_owned()))
                .unwrap_or(CanonicalValue::Null),
        ),
        string_field("expected_state_digest", expected_state_digest.to_string()),
        string_field(
            "expected_content_digest",
            expected_content_digest.to_string(),
        ),
        integer_field("expected_content_size_bytes", expected_content_size_bytes)?,
    ])
}

fn ensure_checkpoint_content_object(
    transaction: &Transaction<'_>,
    content_digest: Digest,
    size_bytes: i64,
    media_type: Option<&str>,
    format_metadata_json: &str,
) -> Result<()> {
    let row = transaction
        .query_row(
            "SELECT size_bytes, media_type, format_metadata_json
             FROM content_object
             WHERE content_digest = ?1",
            params![&content_digest.as_bytes()[..]],
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

    if let Some((stored_size_bytes, stored_media_type, stored_format_metadata_json)) = row {
        parse_canonical_object_json(
            "content_object.format_metadata_json",
            &stored_format_metadata_json,
        )?;
        if stored_size_bytes != size_bytes
            || stored_media_type.as_deref() != media_type
            || stored_format_metadata_json != format_metadata_json
        {
            return Err(WorkVcsError::QueryInvalid(format!(
                "{CONTENT_OBJECT_KIND} {content_digest} already exists with different metadata"
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
                &content_digest.as_bytes()[..],
                size_bytes,
                media_type,
                format_metadata_json
            ],
        )
        .map_err(storage_error)?;
    Ok(())
}

fn load_checkpoint(
    connection: &StoreConnection,
    checkpoint_id: CheckpointId,
) -> Result<CheckpointSnapshot> {
    let row = connection
        .inner()
        .query_row(
            "SELECT checkpoint.workspace_id,
                    checkpoint.commit_id,
                    checkpoint.state_digest,
                    checkpoint.checkpoint_format_version,
                    checkpoint.content_digest,
                    checkpoint.created_at_us,
                    content_object.size_bytes,
                    content_object.media_type,
                    content_object.format_metadata_json,
                    checkpoint_status.usability_state,
                    checkpoint_status.last_validated_at_us,
                    checkpoint_status.detail_json
             FROM checkpoint
             JOIN content_object
               ON content_object.content_digest = checkpoint.content_digest
             JOIN checkpoint_status
               ON checkpoint_status.checkpoint_id = checkpoint.checkpoint_id
             WHERE checkpoint.checkpoint_id = ?1",
            params![&checkpoint_id.raw_bytes()[..]],
            |row| {
                Ok((
                    row.get::<_, Vec<u8>>(0)?,
                    row.get::<_, Vec<u8>>(1)?,
                    row.get::<_, Vec<u8>>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, Vec<u8>>(4)?,
                    row.get::<_, i64>(5)?,
                    row.get::<_, i64>(6)?,
                    row.get::<_, Option<String>>(7)?,
                    row.get::<_, String>(8)?,
                    row.get::<_, String>(9)?,
                    row.get::<_, i64>(10)?,
                    row.get::<_, String>(11)?,
                ))
            },
        )
        .optional()
        .map_err(storage_error)?;

    let Some((
        workspace_id,
        commit_id,
        state_digest,
        checkpoint_format_version,
        content_digest,
        created_at_us,
        content_size_bytes,
        media_type,
        format_metadata_json,
        usability_state,
        last_validated_at_us,
        status_detail_json,
    )) = row
    else {
        return Err(WorkVcsError::QueryInvalid(format!(
            "checkpoint {checkpoint_id} does not exist or is missing content/status metadata"
        )));
    };

    if checkpoint_format_version <= 0 {
        return Err(WorkVcsError::QueryInvalid(
            "checkpoint.checkpoint_format_version must be positive".to_owned(),
        ));
    }
    if content_size_bytes < 0 {
        return Err(WorkVcsError::QueryInvalid(
            "content_object.size_bytes must be non-negative".to_owned(),
        ));
    }
    validate_stored_text("checkpoint_status.usability_state", &usability_state)?;
    if let Some(media_type) = media_type.as_deref() {
        validate_stored_text("content_object.media_type", media_type)?;
    }
    if created_at_us < 0 || last_validated_at_us < 0 {
        return Err(WorkVcsError::QueryInvalid(
            "checkpoint timestamps must be non-negative".to_owned(),
        ));
    }

    Ok(CheckpointSnapshot {
        checkpoint_id,
        workspace_id: decode_workspace_id("checkpoint.workspace_id", workspace_id)?,
        commit_id: decode_commit_id("checkpoint.commit_id", commit_id)?,
        state_digest: decode_digest("checkpoint.state_digest", state_digest)?,
        checkpoint_format_version,
        content_digest: decode_digest("checkpoint.content_digest", content_digest)?,
        content_size_bytes,
        media_type,
        format_metadata: parse_canonical_object_json(
            "content_object.format_metadata_json",
            &format_metadata_json,
        )?,
        created_at_us,
        usability_state,
        last_validated_at_us,
        status_detail: parse_canonical_object_json(
            "checkpoint_status.detail_json",
            &status_detail_json,
        )?,
    })
}

fn checkpoint_ids_for_commit(
    connection: &StoreConnection,
    commit_id: CommitId,
) -> Result<Vec<CheckpointId>> {
    let mut statement = connection
        .inner()
        .prepare(
            "SELECT checkpoint_id
             FROM checkpoint
             WHERE commit_id = ?1
             ORDER BY created_at_us, checkpoint_id",
        )
        .map_err(storage_error)?;
    let rows = statement
        .query_map(params![&commit_id.raw_bytes()[..]], |row| {
            row.get::<_, Vec<u8>>(0)
        })
        .map_err(storage_error)?;

    let mut checkpoint_ids = Vec::new();
    for row in rows {
        checkpoint_ids.push(decode_checkpoint_id(
            "checkpoint.checkpoint_id",
            row.map_err(storage_error)?,
        )?);
    }
    Ok(checkpoint_ids)
}

fn string_field(name: &str, value: impl Into<String>) -> (String, CanonicalValue) {
    (name.to_owned(), CanonicalValue::String(value.into()))
}

fn integer_field(name: &str, value: i64) -> Result<(String, CanonicalValue)> {
    Ok((name.to_owned(), CanonicalValue::safe_integer(value)?))
}

fn usize_to_i64(label: &str, value: usize) -> Result<i64> {
    i64::try_from(value)
        .map_err(|_| WorkVcsError::QueryInvalid(format!("{label} does not fit i64")))
}

fn canonical_object_json(label: &str, value: &CanonicalValue) -> Result<String> {
    require_object_value(label, value)?;
    canonical_json_string(value)
}

fn parse_canonical_object_json(label: &str, input: &str) -> Result<CanonicalValue> {
    let value = parse_canonical_json(input.as_bytes())?;
    require_object_value(label, &value)?;
    let reencoded = canonical_json_string(&value)?;
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
            "{label} must be a canonical JSON object"
        ))),
    }
}

fn canonical_json_string(value: &CanonicalValue) -> Result<String> {
    let bytes = canonical_bytes(value)?;
    String::from_utf8(bytes).map_err(|error| {
        WorkVcsError::CanonicalEncodingInvalid(format!("canonical JSON was not UTF-8: {error}"))
    })
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

fn decode_workspace_id(column: &str, bytes: Vec<u8>) -> Result<WorkspaceId> {
    WorkspaceId::from_bytes(fixed_bytes(column, bytes)?).map_err(|error| {
        WorkVcsError::StorageFailure(format!("{column} is not a UUIDv7 value: {error}"))
    })
}

fn decode_commit_id(column: &str, bytes: Vec<u8>) -> Result<CommitId> {
    CommitId::from_bytes(fixed_bytes(column, bytes)?).map_err(|error| {
        WorkVcsError::StorageFailure(format!("{column} is not a UUIDv7 value: {error}"))
    })
}

fn decode_checkpoint_id(column: &str, bytes: Vec<u8>) -> Result<CheckpointId> {
    CheckpointId::from_bytes(fixed_bytes(column, bytes)?).map_err(|error| {
        WorkVcsError::StorageFailure(format!("{column} is not a UUIDv7 value: {error}"))
    })
}

fn decode_digest(column: &str, bytes: Vec<u8>) -> Result<Digest> {
    Ok(Digest::from_bytes(fixed_bytes(column, bytes)?))
}

fn fixed_bytes<const N: usize>(column: &str, bytes: Vec<u8>) -> Result<[u8; N]> {
    let len = bytes.len();
    bytes.try_into().map_err(|_| {
        WorkVcsError::StorageFailure(format!("{column} must be {N} bytes, found {len}"))
    })
}
