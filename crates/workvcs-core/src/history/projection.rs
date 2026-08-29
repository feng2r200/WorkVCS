use crate::error::{Result, WorkVcsError, storage_error};
use crate::identity::{BranchId, CommitId, Digest, RelationId, RelationVersionId, WorkspaceId};
use crate::store::{StoreConnection, current_epoch_micros};
use rusqlite::{OptionalExtension, Transaction, TransactionBehavior, params};

const COMPLETE_PROJECTION_STATUS: &str = "complete";
const INVALID_PROJECTION_STATUS: &str = "invalid";
const NOT_MATERIALIZED_PROJECTION_STATUS: &str = "not_materialized";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BranchProjectionStatus {
    Complete,
    NotMaterialized,
    Stale,
    Invalid,
}

impl BranchProjectionStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Complete => COMPLETE_PROJECTION_STATUS,
            Self::NotMaterialized => NOT_MATERIALIZED_PROJECTION_STATUS,
            Self::Stale => "stale",
            Self::Invalid => INVALID_PROJECTION_STATUS,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BranchProjectionRefreshOptions {
    branch_id: BranchId,
}

impl BranchProjectionRefreshOptions {
    pub fn new(branch_id: BranchId) -> Self {
        Self { branch_id }
    }

    pub fn branch_id(self) -> BranchId {
        self.branch_id
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BranchProjectionRefreshResult {
    pub workspace_id: WorkspaceId,
    pub branch_id: BranchId,
    pub projected_commit_id: CommitId,
    pub projection_state_digest: Digest,
    pub entity_count: usize,
    pub relation_count: usize,
    pub updated_at_us: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BranchProjectionSnapshot {
    pub workspace_id: WorkspaceId,
    pub branch_id: BranchId,
    pub head_commit_id: CommitId,
    pub head_state_digest: Digest,
    pub status: BranchProjectionStatus,
    pub stored_status: Option<String>,
    pub projected_commit_id: Option<CommitId>,
    pub projection_state_digest: Option<Digest>,
    pub entity_count: usize,
    pub relation_count: usize,
    pub updated_at_us: Option<i64>,
}

impl BranchProjectionSnapshot {
    pub fn is_current(&self) -> bool {
        self.status == BranchProjectionStatus::Complete
    }
}

struct StoredProjectionState {
    projection_status: String,
    projected_commit_id: Option<CommitId>,
    projection_state_digest: Option<Digest>,
    updated_at_us: i64,
}

struct ProjectionBranchHead {
    workspace_id: WorkspaceId,
    head_commit_id: CommitId,
    state_digest: Digest,
}

struct RelationProjectionRow {
    relation_type: String,
    source_object_id: [u8; 16],
    target_object_id: [u8; 16],
    relation_discriminator: String,
}

pub(crate) fn refresh_branch_projection(
    connection: &mut StoreConnection,
    options: BranchProjectionRefreshOptions,
) -> Result<BranchProjectionRefreshResult> {
    connection.verify_foreign_keys()?;
    let branch = super::branch_head(connection, options.branch_id())?;
    let replayed = super::state_at(connection, branch.head_commit_id)?;
    if replayed.workspace_id != branch.workspace_id {
        return Err(WorkVcsError::QueryInvalid(format!(
            "branch {} head {} replays in workspace {}, not {}",
            options.branch_id(),
            branch.head_commit_id,
            replayed.workspace_id,
            branch.workspace_id
        )));
    }
    if replayed.state_digest != branch.state_digest {
        return Err(WorkVcsError::QueryInvalid(format!(
            "branch {} head {} digest does not match replayed WorkState",
            options.branch_id(),
            branch.head_commit_id
        )));
    }

    let updated_at_us = current_epoch_micros()?;
    let transaction = connection
        .inner_mut()
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(storage_error)?;
    let latest = load_branch_head(&transaction, options.branch_id())?;
    if latest.head_commit_id != branch.head_commit_id {
        return Err(WorkVcsError::BranchHeadConflict(format!(
            "branch {} head moved from {} to {} before projection refresh",
            options.branch_id(),
            branch.head_commit_id,
            latest.head_commit_id
        )));
    }
    if latest.state_digest != branch.state_digest || latest.workspace_id != branch.workspace_id {
        return Err(WorkVcsError::QueryInvalid(format!(
            "branch {} head metadata changed before projection refresh",
            options.branch_id()
        )));
    }

    replace_projection_rows(&transaction, options.branch_id(), &replayed, updated_at_us)?;
    transaction.commit().map_err(storage_error)?;

    Ok(BranchProjectionRefreshResult {
        workspace_id: replayed.workspace_id,
        branch_id: options.branch_id(),
        projected_commit_id: branch.head_commit_id,
        projection_state_digest: replayed.state_digest,
        entity_count: replayed.state.entities().len(),
        relation_count: replayed.state.relations().len(),
        updated_at_us,
    })
}

pub(crate) fn branch_projection(
    connection: &StoreConnection,
    branch_id: BranchId,
) -> Result<BranchProjectionSnapshot> {
    connection.verify_foreign_keys()?;
    let head = super::branch_head(connection, branch_id)?;
    let stored = load_stored_projection(connection, branch_id)?;
    let entity_count = count_projection_rows(connection, "branch_entity_current", branch_id)?;
    let relation_count = count_projection_rows(connection, "branch_relation_current", branch_id)?;

    let Some(stored) = stored else {
        return Ok(BranchProjectionSnapshot {
            workspace_id: head.workspace_id,
            branch_id,
            head_commit_id: head.head_commit_id,
            head_state_digest: head.state_digest,
            status: BranchProjectionStatus::NotMaterialized,
            stored_status: None,
            projected_commit_id: None,
            projection_state_digest: None,
            entity_count,
            relation_count,
            updated_at_us: None,
        });
    };

    let status = effective_status(&stored, head.head_commit_id, head.state_digest);
    Ok(BranchProjectionSnapshot {
        workspace_id: head.workspace_id,
        branch_id,
        head_commit_id: head.head_commit_id,
        head_state_digest: head.state_digest,
        status,
        stored_status: Some(stored.projection_status),
        projected_commit_id: stored.projected_commit_id,
        projection_state_digest: stored.projection_state_digest,
        entity_count,
        relation_count,
        updated_at_us: Some(stored.updated_at_us),
    })
}

fn replace_projection_rows(
    transaction: &Transaction<'_>,
    branch_id: BranchId,
    replayed: &super::ReplayedState,
    updated_at_us: i64,
) -> Result<()> {
    let branch_id_bytes = branch_id.raw_bytes();
    transaction
        .execute(
            "DELETE FROM branch_relation_current
             WHERE branch_id = ?1",
            params![&branch_id_bytes[..]],
        )
        .map_err(storage_error)?;
    transaction
        .execute(
            "DELETE FROM branch_entity_current
             WHERE branch_id = ?1",
            params![&branch_id_bytes[..]],
        )
        .map_err(storage_error)?;
    transaction
        .execute(
            "DELETE FROM branch_projection_state
             WHERE branch_id = ?1",
            params![&branch_id_bytes[..]],
        )
        .map_err(storage_error)?;

    let projected_commit_id_bytes = replayed.commit_id.raw_bytes();
    let state_digest_bytes = replayed.state_digest.as_bytes();
    transaction
        .execute(
            "INSERT INTO branch_projection_state(
                branch_id,
                projection_status,
                projected_commit_id,
                projection_state_digest,
                updated_at_us
             )
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                &branch_id_bytes[..],
                COMPLETE_PROJECTION_STATUS,
                &projected_commit_id_bytes[..],
                &state_digest_bytes[..],
                updated_at_us
            ],
        )
        .map_err(storage_error)?;

    for (entity_id, entity_version_id) in replayed.state.entities() {
        let entity_id_bytes = entity_id.raw_bytes();
        let entity_version_id_bytes = entity_version_id.raw_bytes();
        transaction
            .execute(
                "INSERT INTO branch_entity_current(
                    branch_id,
                    entity_id,
                    entity_version_id
                 )
                 VALUES (?1, ?2, ?3)",
                params![
                    &branch_id_bytes[..],
                    &entity_id_bytes[..],
                    &entity_version_id_bytes[..]
                ],
            )
            .map_err(storage_error)?;
    }

    for (relation_id, relation_version_id) in replayed.state.relations() {
        let relation = load_relation_projection_row(
            transaction,
            replayed.workspace_id,
            *relation_id,
            *relation_version_id,
        )?;
        let relation_id_bytes = relation_id.raw_bytes();
        let relation_version_id_bytes = relation_version_id.raw_bytes();
        transaction
            .execute(
                "INSERT INTO branch_relation_current(
                    branch_id,
                    relation_id,
                    relation_version_id,
                    relation_type,
                    source_object_id,
                    target_object_id,
                    relation_discriminator
                 )
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    &branch_id_bytes[..],
                    &relation_id_bytes[..],
                    &relation_version_id_bytes[..],
                    relation.relation_type,
                    &relation.source_object_id[..],
                    &relation.target_object_id[..],
                    relation.relation_discriminator
                ],
            )
            .map_err(storage_error)?;
    }
    Ok(())
}

fn effective_status(
    stored: &StoredProjectionState,
    head_commit_id: CommitId,
    head_state_digest: Digest,
) -> BranchProjectionStatus {
    match stored.projection_status.as_str() {
        NOT_MATERIALIZED_PROJECTION_STATUS => BranchProjectionStatus::NotMaterialized,
        INVALID_PROJECTION_STATUS => BranchProjectionStatus::Invalid,
        COMPLETE_PROJECTION_STATUS => {
            match (stored.projected_commit_id, stored.projection_state_digest) {
                (Some(projected_commit_id), Some(projection_state_digest))
                    if projected_commit_id == head_commit_id
                        && projection_state_digest == head_state_digest =>
                {
                    BranchProjectionStatus::Complete
                }
                (Some(projected_commit_id), Some(_)) if projected_commit_id != head_commit_id => {
                    BranchProjectionStatus::Stale
                }
                _ => BranchProjectionStatus::Invalid,
            }
        }
        _ => BranchProjectionStatus::Invalid,
    }
}

fn load_stored_projection(
    connection: &StoreConnection,
    branch_id: BranchId,
) -> Result<Option<StoredProjectionState>> {
    let row = connection
        .inner()
        .query_row(
            "SELECT projection_status,
                    projected_commit_id,
                    projection_state_digest,
                    updated_at_us
             FROM branch_projection_state
             WHERE branch_id = ?1",
            params![&branch_id.raw_bytes()[..]],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, Option<Vec<u8>>>(1)?,
                    row.get::<_, Option<Vec<u8>>>(2)?,
                    row.get::<_, i64>(3)?,
                ))
            },
        )
        .optional()
        .map_err(storage_error)?;

    row.map(
        |(projection_status, projected_commit_id, projection_state_digest, updated_at_us)| {
            validate_stored_text(
                "branch_projection_state.projection_status",
                &projection_status,
            )?;
            if updated_at_us < 0 {
                return Err(WorkVcsError::QueryInvalid(
                    "branch_projection_state.updated_at_us must be non-negative".to_owned(),
                ));
            }
            Ok(StoredProjectionState {
                projection_status,
                projected_commit_id: decode_optional_commit_id(
                    "branch_projection_state.projected_commit_id",
                    projected_commit_id,
                )?,
                projection_state_digest: decode_optional_digest(
                    "branch_projection_state.projection_state_digest",
                    projection_state_digest,
                )?,
                updated_at_us,
            })
        },
    )
    .transpose()
}

fn load_branch_head(
    transaction: &Transaction<'_>,
    branch_id: BranchId,
) -> Result<ProjectionBranchHead> {
    let row = transaction
        .query_row(
            "SELECT branch.workspace_id,
                    branch.head_commit_id,
                    workstate_commit.state_digest
             FROM branch
             JOIN workstate_commit
               ON workstate_commit.workspace_id = branch.workspace_id
              AND workstate_commit.commit_id = branch.head_commit_id
             WHERE branch.branch_id = ?1",
            params![&branch_id.raw_bytes()[..]],
            |row| {
                Ok((
                    row.get::<_, Vec<u8>>(0)?,
                    row.get::<_, Vec<u8>>(1)?,
                    row.get::<_, Vec<u8>>(2)?,
                ))
            },
        )
        .optional()
        .map_err(storage_error)?;

    let Some((workspace_id, head_commit_id, state_digest)) = row else {
        return Err(WorkVcsError::BranchNotFound(format!(
            "branch {branch_id} does not exist or has no current head commit"
        )));
    };
    Ok(ProjectionBranchHead {
        workspace_id: decode_workspace_id("branch.workspace_id", workspace_id)?,
        head_commit_id: decode_commit_id("branch.head_commit_id", head_commit_id)?,
        state_digest: decode_digest("workstate_commit.state_digest", state_digest)?,
    })
}

fn load_relation_projection_row(
    transaction: &Transaction<'_>,
    workspace_id: WorkspaceId,
    relation_id: RelationId,
    relation_version_id: RelationVersionId,
) -> Result<RelationProjectionRow> {
    let row = transaction
        .query_row(
            "SELECT relation.workspace_id,
                    relation.relation_type,
                    relation.source_object_id,
                    relation.target_object_id,
                    relation.relation_discriminator
             FROM relation
             JOIN relation_version
               ON relation_version.relation_id = relation.object_id
              AND relation_version.relation_version_id = ?2
             WHERE relation.object_id = ?1",
            params![
                &relation_id.raw_bytes()[..],
                &relation_version_id.raw_bytes()[..]
            ],
            |row| {
                Ok((
                    row.get::<_, Vec<u8>>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Vec<u8>>(2)?,
                    row.get::<_, Vec<u8>>(3)?,
                    row.get::<_, String>(4)?,
                ))
            },
        )
        .optional()
        .map_err(storage_error)?;

    let Some((
        relation_workspace_id,
        relation_type,
        source_object_id,
        target_object_id,
        relation_discriminator,
    )) = row
    else {
        return Err(WorkVcsError::ReplayInvalid(format!(
            "RelationVersion {relation_version_id} for relation {relation_id} does not exist"
        )));
    };
    let relation_workspace_id =
        decode_workspace_id("relation.workspace_id", relation_workspace_id)?;
    if relation_workspace_id != workspace_id {
        return Err(WorkVcsError::ReplayInvalid(format!(
            "relation {relation_id} belongs to workspace {relation_workspace_id}, not {workspace_id}"
        )));
    }
    validate_stored_text("relation.relation_type", &relation_type)?;
    validate_stored_text_allow_empty("relation.relation_discriminator", &relation_discriminator)?;
    Ok(RelationProjectionRow {
        relation_type,
        source_object_id: decode_16("relation.source_object_id", source_object_id)?,
        target_object_id: decode_16("relation.target_object_id", target_object_id)?,
        relation_discriminator,
    })
}

fn count_projection_rows(
    connection: &StoreConnection,
    table: &str,
    branch_id: BranchId,
) -> Result<usize> {
    let count = connection
        .inner()
        .query_row(
            &format!("SELECT count(*) FROM {table} WHERE branch_id = ?1"),
            params![&branch_id.raw_bytes()[..]],
            |row| row.get::<_, i64>(0),
        )
        .map_err(storage_error)?;
    usize::try_from(count).map_err(|_| {
        WorkVcsError::QueryInvalid(format!("{table} row count does not fit into usize"))
    })
}

fn decode_optional_commit_id(column: &str, value: Option<Vec<u8>>) -> Result<Option<CommitId>> {
    value
        .map(|bytes| decode_commit_id(column, bytes))
        .transpose()
}

fn decode_optional_digest(column: &str, value: Option<Vec<u8>>) -> Result<Option<Digest>> {
    value.map(|bytes| decode_digest(column, bytes)).transpose()
}

fn decode_workspace_id(column: &str, bytes: Vec<u8>) -> Result<WorkspaceId> {
    let bytes = decode_16(column, bytes)?;
    WorkspaceId::from_bytes(bytes)
        .map_err(|error| WorkVcsError::QueryInvalid(format!("{column} is invalid: {error}")))
}

fn decode_commit_id(column: &str, bytes: Vec<u8>) -> Result<CommitId> {
    let bytes = decode_16(column, bytes)?;
    CommitId::from_bytes(bytes)
        .map_err(|error| WorkVcsError::QueryInvalid(format!("{column} is invalid: {error}")))
}

fn decode_digest(column: &str, bytes: Vec<u8>) -> Result<Digest> {
    let bytes = bytes.try_into().map_err(|bytes: Vec<u8>| {
        WorkVcsError::QueryInvalid(format!("{column} must be 32 bytes, found {}", bytes.len()))
    })?;
    Ok(Digest::from_bytes(bytes))
}

fn decode_16(column: &str, bytes: Vec<u8>) -> Result<[u8; 16]> {
    bytes.try_into().map_err(|bytes: Vec<u8>| {
        WorkVcsError::QueryInvalid(format!("{column} must be 16 bytes, found {}", bytes.len()))
    })
}

fn validate_stored_text(label: &str, value: &str) -> Result<()> {
    if value.trim() != value || value.is_empty() {
        return Err(WorkVcsError::QueryInvalid(format!(
            "{label} must be non-empty and trimmed"
        )));
    }
    if value
        .as_bytes()
        .iter()
        .any(|byte| *byte == b'\0' || *byte < 0x20 || *byte == 0x7f)
    {
        return Err(WorkVcsError::QueryInvalid(format!(
            "{label} must not contain NUL or ASCII control characters"
        )));
    }
    Ok(())
}

fn validate_stored_text_allow_empty(label: &str, value: &str) -> Result<()> {
    if value.trim() != value {
        return Err(WorkVcsError::QueryInvalid(format!(
            "{label} must be trimmed"
        )));
    }
    if value
        .as_bytes()
        .iter()
        .any(|byte| *byte == b'\0' || *byte < 0x20 || *byte == 0x7f)
    {
        return Err(WorkVcsError::QueryInvalid(format!(
            "{label} must not contain NUL or ASCII control characters"
        )));
    }
    Ok(())
}
