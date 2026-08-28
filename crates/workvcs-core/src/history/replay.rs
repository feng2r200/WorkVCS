use crate::canonical::{CanonicalValue, WorkState, canonical_bytes, work_state_mapping_digest};
use crate::error::{Result, WorkVcsError, storage_error};
use crate::identity::{ChangeSetId, CommitId, Digest, WorkspaceId};
use crate::store::StoreConnection;
use rusqlite::{OptionalExtension, Params, params};

const GENESIS_OPERATION_SCHEMA_VERSION: i64 = 1;
const GENESIS_OPERATION_TYPE: &str = "workspace.genesis";
const GENESIS_COMMIT_KIND: &str = "genesis";
const NORMAL_COMMIT_KIND: &str = "normal";
const MERGE_COMMIT_KIND: &str = "merge";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReplayedState {
    pub workspace_id: WorkspaceId,
    pub commit_id: CommitId,
    pub state: WorkState,
    pub state_digest: Digest,
}

pub(crate) fn state_at(connection: &StoreConnection, commit_id: CommitId) -> Result<ReplayedState> {
    let commit = load_commit(connection, commit_id)?;
    match commit.commit_kind.as_str() {
        GENESIS_COMMIT_KIND => replay_genesis(connection, commit_id, commit),
        NORMAL_COMMIT_KIND | MERGE_COMMIT_KIND => Err(WorkVcsError::ReplayUnsupported(format!(
            "{:?} WorkStateCommit replay is deferred until ChangeOperation replay is implemented",
            commit.commit_kind
        ))),
        other => Err(WorkVcsError::ReplayInvalid(format!(
            "unsupported WorkStateCommit kind {other:?}"
        ))),
    }
}

struct CommitRow {
    workspace_id: WorkspaceId,
    changeset_id: ChangeSetId,
    commit_kind: String,
    state_digest: Digest,
}

fn load_commit(connection: &StoreConnection, commit_id: CommitId) -> Result<CommitRow> {
    let commit_id_bytes = commit_id.raw_bytes();
    let row = connection
        .inner()
        .query_row(
            "SELECT workspace_id, changeset_id, commit_kind, state_digest
             FROM workstate_commit
             WHERE commit_id = ?1",
            params![&commit_id_bytes[..]],
            |row| {
                Ok((
                    row.get::<_, Vec<u8>>(0)?,
                    row.get::<_, Vec<u8>>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, Vec<u8>>(3)?,
                ))
            },
        )
        .optional()
        .map_err(storage_error)?;

    let Some((workspace_id, changeset_id, commit_kind, state_digest)) = row else {
        return Err(WorkVcsError::CommitNotFound(format!(
            "commit {commit_id} does not exist"
        )));
    };

    Ok(CommitRow {
        workspace_id: decode_workspace_id("workstate_commit.workspace_id", workspace_id)?,
        changeset_id: decode_changeset_id("workstate_commit.changeset_id", changeset_id)?,
        commit_kind,
        state_digest: decode_digest("workstate_commit.state_digest", state_digest)?,
    })
}

fn replay_genesis(
    connection: &StoreConnection,
    commit_id: CommitId,
    commit: CommitRow,
) -> Result<ReplayedState> {
    validate_workspace_genesis_link(connection, commit.workspace_id, commit_id)?;
    validate_genesis_changeset(connection, commit.workspace_id, commit.changeset_id)?;
    require_count(
        connection,
        "Genesis commit parents",
        0,
        "SELECT count(*)
         FROM commit_parent
         WHERE commit_id = ?1",
        params![&commit_id.raw_bytes()[..]],
    )?;
    require_count(
        connection,
        "Genesis ChangeOperations",
        0,
        "SELECT count(*)
         FROM change_operation
         WHERE changeset_id = ?1",
        params![&commit.changeset_id.raw_bytes()[..]],
    )?;

    let state = WorkState::empty();
    let actual_digest = work_state_mapping_digest(&state);
    if commit.state_digest != actual_digest {
        return Err(WorkVcsError::ReplayInvalid(format!(
            "Genesis commit {commit_id} state digest does not match replayed empty WorkState"
        )));
    }

    Ok(ReplayedState {
        workspace_id: commit.workspace_id,
        commit_id,
        state,
        state_digest: actual_digest,
    })
}

fn validate_workspace_genesis_link(
    connection: &StoreConnection,
    workspace_id: WorkspaceId,
    commit_id: CommitId,
) -> Result<()> {
    require_count(
        connection,
        "Workspace Genesis link",
        1,
        "SELECT count(*)
         FROM workspace
         WHERE workspace_id = ?1
           AND genesis_commit_id = ?2",
        params![&workspace_id.raw_bytes()[..], &commit_id.raw_bytes()[..]],
    )
}

fn validate_genesis_changeset(
    connection: &StoreConnection,
    workspace_id: WorkspaceId,
    changeset_id: ChangeSetId,
) -> Result<()> {
    let row = connection
        .inner()
        .query_row(
            "SELECT operation_type, operation_schema_version, operation_payload_json, rationale_json
             FROM changeset
             WHERE workspace_id = ?1
               AND changeset_id = ?2",
            params![&workspace_id.raw_bytes()[..], &changeset_id.raw_bytes()[..]],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                ))
            },
        )
        .optional()
        .map_err(storage_error)?;

    let Some((operation_type, operation_schema_version, operation_payload_json, rationale_json)) =
        row
    else {
        return Err(WorkVcsError::ReplayInvalid(format!(
            "Genesis ChangeSet {changeset_id} does not exist"
        )));
    };
    if operation_type != GENESIS_OPERATION_TYPE {
        return Err(WorkVcsError::ReplayInvalid(format!(
            "Genesis ChangeSet {changeset_id} has operation type {operation_type:?}"
        )));
    }
    if operation_schema_version != GENESIS_OPERATION_SCHEMA_VERSION {
        return Err(WorkVcsError::ReplayInvalid(format!(
            "Genesis ChangeSet {changeset_id} has operation schema version {operation_schema_version}"
        )));
    }

    let empty_object_json = canonical_empty_object_json()?;
    if operation_payload_json != empty_object_json {
        return Err(WorkVcsError::ReplayInvalid(format!(
            "Genesis ChangeSet {changeset_id} operation payload is not canonical empty JSON"
        )));
    }
    if rationale_json != empty_object_json {
        return Err(WorkVcsError::ReplayInvalid(format!(
            "Genesis ChangeSet {changeset_id} rationale is not canonical empty JSON"
        )));
    }
    Ok(())
}

fn canonical_empty_object_json() -> Result<String> {
    let value = CanonicalValue::object(Vec::new())?;
    String::from_utf8(canonical_bytes(&value)?).map_err(|error| {
        WorkVcsError::CanonicalEncodingInvalid(format!(
            "canonical empty object JSON was not UTF-8: {error}"
        ))
    })
}

fn require_count<P: Params>(
    connection: &StoreConnection,
    label: &str,
    expected: i64,
    sql: &str,
    params: P,
) -> Result<()> {
    let actual = connection
        .inner()
        .query_row(sql, params, |row| row.get::<_, i64>(0))
        .map_err(storage_error)?;
    if actual == expected {
        Ok(())
    } else {
        Err(WorkVcsError::ReplayInvalid(format!(
            "{label} expected count {expected}, found {actual}"
        )))
    }
}

fn decode_workspace_id(column: &str, bytes: Vec<u8>) -> Result<WorkspaceId> {
    let bytes = decode_16(column, bytes)?;
    WorkspaceId::from_bytes(bytes).map_err(|error| {
        WorkVcsError::ReplayInvalid(format!("{column} is not a UUIDv7 value: {error}"))
    })
}

fn decode_changeset_id(column: &str, bytes: Vec<u8>) -> Result<ChangeSetId> {
    let bytes = decode_16(column, bytes)?;
    ChangeSetId::from_bytes(bytes).map_err(|error| {
        WorkVcsError::ReplayInvalid(format!("{column} is not a UUIDv7 value: {error}"))
    })
}

fn decode_16(column: &str, bytes: Vec<u8>) -> Result<[u8; 16]> {
    bytes.try_into().map_err(|bytes: Vec<u8>| {
        WorkVcsError::ReplayInvalid(format!("{column} must be 16 bytes, found {}", bytes.len()))
    })
}

fn decode_digest(column: &str, bytes: Vec<u8>) -> Result<Digest> {
    let bytes = bytes.try_into().map_err(|bytes: Vec<u8>| {
        WorkVcsError::ReplayInvalid(format!("{column} must be 32 bytes, found {}", bytes.len()))
    })?;
    Ok(Digest::from_bytes(bytes))
}
