use crate::canonical::{CanonicalValue, WorkState, canonical_bytes, work_state_mapping_digest};
use crate::error::{Result, WorkVcsError, storage_error};
use crate::history::branch::validate_branch_name;
use crate::identity::{BranchId, ChangeSetId, CommitId, Digest, EventId, StoreId, WorkspaceId};
use crate::store::{StoreConnection, current_epoch_micros};
use rusqlite::{OptionalExtension, Params, TransactionBehavior, params};

const DEFAULT_INITIAL_BRANCH_NAME: &str = "main";
const GENESIS_OPERATION_SCHEMA_VERSION: i64 = 1;
const GENESIS_OPERATION_TYPE: &str = "workspace.genesis";
const GENESIS_COMMIT_KIND: &str = "genesis";
const INITIAL_BRANCH_LIFECYCLE_STATE: &str = "active";
const WORKSPACE_INITIALIZED_EVENT_KIND: &str = "workspace.initialized";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkspaceInitOptions {
    display_name: String,
    initial_branch_name: String,
}

impl WorkspaceInitOptions {
    pub fn new(display_name: impl Into<String>) -> Result<Self> {
        let display_name = display_name.into();
        validate_workspace_display_name(&display_name)?;
        Ok(Self {
            display_name,
            initial_branch_name: DEFAULT_INITIAL_BRANCH_NAME.to_owned(),
        })
    }

    pub fn with_initial_branch_name(mut self, name: impl Into<String>) -> Result<Self> {
        let name = name.into();
        validate_branch_name(&name)?;
        self.initial_branch_name = name;
        Ok(self)
    }

    pub fn display_name(&self) -> &str {
        &self.display_name
    }

    pub fn initial_branch_name(&self) -> &str {
        &self.initial_branch_name
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkspaceInfo {
    pub workspace_id: WorkspaceId,
    pub display_name: String,
    pub genesis_commit_id: CommitId,
    pub genesis_changeset_id: ChangeSetId,
    pub initial_branch_id: BranchId,
    pub initial_branch_name: String,
    pub state_digest: Digest,
    pub created_at_us: i64,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct WorkspaceListOptions;

impl WorkspaceListOptions {
    pub fn all() -> Self {
        Self
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkspaceListResult {
    pub workspaces: Vec<WorkspaceInfo>,
}

pub(crate) fn create_workspace(
    connection: &mut StoreConnection,
    store_id: StoreId,
    options: &WorkspaceInitOptions,
) -> Result<WorkspaceInfo> {
    connection.verify_foreign_keys()?;

    let workspace_id = WorkspaceId::new_v7();
    let changeset_id = ChangeSetId::new_v7();
    let commit_id = CommitId::new_v7();
    let branch_id = BranchId::new_v7();
    let event_id = EventId::new_v7();
    let now_us = current_epoch_micros()?;
    let state_digest = empty_work_state_digest();
    let empty_object_json = canonical_empty_object_json()?;

    let workspace_id_bytes = workspace_id.raw_bytes();
    let store_id_bytes = store_id.raw_bytes();
    let changeset_id_bytes = changeset_id.raw_bytes();
    let commit_id_bytes = commit_id.raw_bytes();
    let branch_id_bytes = branch_id.raw_bytes();
    let event_id_bytes = event_id.raw_bytes();
    let state_digest_bytes = state_digest.as_bytes();

    let transaction = connection
        .inner_mut()
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(storage_error)?;
    transaction
        .execute(
            "INSERT INTO workspace(
                workspace_id,
                store_id,
                display_name,
                genesis_commit_id,
                created_at_us
             )
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                &workspace_id_bytes[..],
                &store_id_bytes[..],
                options.display_name(),
                &commit_id_bytes[..],
                now_us
            ],
        )
        .map_err(storage_error)?;
    transaction
        .execute(
            "INSERT INTO changeset(
                changeset_id,
                workspace_id,
                operation_type,
                operation_schema_version,
                operation_payload_json,
                rationale_json,
                origin_session_id,
                created_at_us
             )
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, NULL, ?7)",
            params![
                &changeset_id_bytes[..],
                &workspace_id_bytes[..],
                GENESIS_OPERATION_TYPE,
                GENESIS_OPERATION_SCHEMA_VERSION,
                empty_object_json,
                empty_object_json,
                now_us
            ],
        )
        .map_err(storage_error)?;
    transaction
        .execute(
            "INSERT INTO workstate_commit(
                commit_id,
                workspace_id,
                changeset_id,
                commit_kind,
                state_digest,
                committed_at_us
             )
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                &commit_id_bytes[..],
                &workspace_id_bytes[..],
                &changeset_id_bytes[..],
                GENESIS_COMMIT_KIND,
                &state_digest_bytes[..],
                now_us
            ],
        )
        .map_err(storage_error)?;
    transaction
        .execute(
            "INSERT INTO branch(
                branch_id,
                workspace_id,
                name,
                head_commit_id,
                lifecycle_state,
                created_at_us
             )
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                &branch_id_bytes[..],
                &workspace_id_bytes[..],
                options.initial_branch_name(),
                &commit_id_bytes[..],
                INITIAL_BRANCH_LIFECYCLE_STATE,
                now_us
            ],
        )
        .map_err(storage_error)?;
    transaction
        .execute(
            "INSERT INTO event(
                event_id,
                workspace_id,
                changeset_id,
                session_id,
                event_kind,
                occurred_at_us,
                payload_json
             )
             VALUES (?1, ?2, ?3, NULL, ?4, ?5, ?6)",
            params![
                &event_id_bytes[..],
                &workspace_id_bytes[..],
                &changeset_id_bytes[..],
                WORKSPACE_INITIALIZED_EVENT_KIND,
                now_us,
                empty_object_json
            ],
        )
        .map_err(storage_error)?;
    transaction.commit().map_err(storage_error)?;

    load_workspace_info(connection, workspace_id)
}

pub(crate) fn load_workspace_info(
    connection: &StoreConnection,
    workspace_id: WorkspaceId,
) -> Result<WorkspaceInfo> {
    let workspace_id_bytes = workspace_id.raw_bytes();
    let workspace = connection
        .inner()
        .query_row(
            "SELECT display_name, genesis_commit_id, created_at_us
             FROM workspace
             WHERE workspace_id = ?1",
            params![&workspace_id_bytes[..]],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, Vec<u8>>(1)?,
                    row.get::<_, i64>(2)?,
                ))
            },
        )
        .optional()
        .map_err(storage_error)?;

    let Some((display_name, genesis_commit_id_bytes, created_at_us)) = workspace else {
        return Err(WorkVcsError::WorkspaceNotFound(format!(
            "workspace {workspace_id} does not exist"
        )));
    };
    validate_workspace_display_name(&display_name)?;
    validate_created_at_us("workspace.created_at_us", created_at_us)?;

    let genesis_commit_id =
        decode_commit_id("workspace.genesis_commit_id", genesis_commit_id_bytes)?;
    let genesis_commit_id_bytes = genesis_commit_id.raw_bytes();
    let commit = connection
        .inner()
        .query_row(
            "SELECT changeset_id, commit_kind, state_digest
             FROM workstate_commit
             WHERE workspace_id = ?1
               AND commit_id = ?2",
            params![&workspace_id_bytes[..], &genesis_commit_id_bytes[..]],
            |row| {
                Ok((
                    row.get::<_, Vec<u8>>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Vec<u8>>(2)?,
                ))
            },
        )
        .optional()
        .map_err(storage_error)?;

    let Some((changeset_id_bytes, commit_kind, state_digest_bytes)) = commit else {
        return Err(WorkVcsError::StoreBootstrapInvalid(format!(
            "workspace {workspace_id} references a missing Genesis commit"
        )));
    };
    if commit_kind != GENESIS_COMMIT_KIND {
        return Err(WorkVcsError::StoreBootstrapInvalid(format!(
            "workspace {workspace_id} Genesis commit has kind {commit_kind:?}"
        )));
    }

    let genesis_changeset_id =
        decode_changeset_id("workstate_commit.changeset_id", changeset_id_bytes)?;
    let state_digest = decode_digest("workstate_commit.state_digest", state_digest_bytes)?;
    let expected_state_digest = empty_work_state_digest();
    if state_digest != expected_state_digest {
        return Err(WorkVcsError::StoreBootstrapInvalid(format!(
            "workspace {workspace_id} Genesis commit does not use the empty WorkState digest"
        )));
    }

    validate_genesis_changeset(connection, workspace_id, genesis_changeset_id)?;
    validate_workspace_genesis_shape(
        connection,
        workspace_id,
        genesis_commit_id,
        genesis_changeset_id,
    )?;

    let branch = load_initial_branch(connection, workspace_id)?;

    Ok(WorkspaceInfo {
        workspace_id,
        display_name,
        genesis_commit_id,
        genesis_changeset_id,
        initial_branch_id: branch.0,
        initial_branch_name: branch.1,
        state_digest,
        created_at_us,
    })
}

pub(crate) fn workspaces(
    connection: &StoreConnection,
    _options: &WorkspaceListOptions,
) -> Result<WorkspaceListResult> {
    connection.verify_foreign_keys()?;

    let workspace_ids = list_workspace_ids(connection)?;
    let mut workspaces = Vec::with_capacity(workspace_ids.len());
    for workspace_id in workspace_ids {
        workspaces.push(load_workspace_info(connection, workspace_id)?);
    }
    Ok(WorkspaceListResult { workspaces })
}

fn list_workspace_ids(connection: &StoreConnection) -> Result<Vec<WorkspaceId>> {
    let mut statement = connection
        .inner()
        .prepare(
            "SELECT workspace_id
             FROM workspace
             ORDER BY created_at_us, workspace_id",
        )
        .map_err(storage_error)?;
    let rows = statement
        .query_map([], |row| row.get::<_, Vec<u8>>(0))
        .map_err(storage_error)?;

    let mut workspace_ids = Vec::new();
    for row in rows {
        workspace_ids.push(decode_workspace_id(
            "workspace.workspace_id",
            row.map_err(storage_error)?,
        )?);
    }
    Ok(workspace_ids)
}

fn validate_genesis_changeset(
    connection: &StoreConnection,
    workspace_id: WorkspaceId,
    changeset_id: ChangeSetId,
) -> Result<()> {
    let workspace_id_bytes = workspace_id.raw_bytes();
    let changeset_id_bytes = changeset_id.raw_bytes();
    let empty_object_json = canonical_empty_object_json()?;
    let changeset = connection
        .inner()
        .query_row(
            "SELECT
                operation_type,
                operation_schema_version,
                operation_payload_json,
                rationale_json
             FROM changeset
             WHERE workspace_id = ?1
               AND changeset_id = ?2",
            params![&workspace_id_bytes[..], &changeset_id_bytes[..]],
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
        changeset
    else {
        return Err(WorkVcsError::StoreBootstrapInvalid(format!(
            "workspace {workspace_id} Genesis references a missing ChangeSet"
        )));
    };
    if operation_type != GENESIS_OPERATION_TYPE {
        return Err(WorkVcsError::StoreBootstrapInvalid(format!(
            "workspace {workspace_id} Genesis ChangeSet has operation type {operation_type:?}"
        )));
    }
    if operation_schema_version != GENESIS_OPERATION_SCHEMA_VERSION {
        return Err(WorkVcsError::StoreBootstrapInvalid(format!(
            "workspace {workspace_id} Genesis ChangeSet has operation schema version {operation_schema_version}"
        )));
    }
    if operation_payload_json != empty_object_json {
        return Err(WorkVcsError::StoreBootstrapInvalid(format!(
            "workspace {workspace_id} Genesis ChangeSet operation payload is not canonical empty JSON"
        )));
    }
    if rationale_json != empty_object_json {
        return Err(WorkVcsError::StoreBootstrapInvalid(format!(
            "workspace {workspace_id} Genesis ChangeSet rationale is not canonical empty JSON"
        )));
    }
    Ok(())
}

fn validate_workspace_genesis_shape(
    connection: &StoreConnection,
    workspace_id: WorkspaceId,
    commit_id: CommitId,
    changeset_id: ChangeSetId,
) -> Result<()> {
    let workspace_id_bytes = workspace_id.raw_bytes();
    let commit_id_bytes = commit_id.raw_bytes();
    let changeset_id_bytes = changeset_id.raw_bytes();
    let empty_object_json = canonical_empty_object_json()?;

    require_count(
        connection,
        "Genesis commits per workspace",
        1,
        "SELECT count(*)
         FROM workstate_commit
         WHERE workspace_id = ?1
           AND commit_kind = 'genesis'",
        params![&workspace_id_bytes[..]],
    )?;
    require_count(
        connection,
        "Genesis commit parents",
        0,
        "SELECT count(*)
         FROM commit_parent
         WHERE commit_id = ?1",
        params![&commit_id_bytes[..]],
    )?;
    require_count(
        connection,
        "Genesis ChangeOperations",
        0,
        "SELECT count(*)
         FROM change_operation
         WHERE changeset_id = ?1",
        params![&changeset_id_bytes[..]],
    )?;

    let branch_count = query_count(
        connection,
        "SELECT count(*)
         FROM branch
         WHERE workspace_id = ?1",
        params![&workspace_id_bytes[..]],
    )?;
    if branch_count < 1 {
        return Err(WorkVcsError::StoreBootstrapInvalid(format!(
            "workspace {workspace_id} must have at least one Branch"
        )));
    }

    let event_count = query_count(
        connection,
        "SELECT count(*)
         FROM event
         WHERE workspace_id = ?1
           AND changeset_id = ?2
           AND event_kind = ?3
           AND payload_json = ?4",
        params![
            &workspace_id_bytes[..],
            &changeset_id_bytes[..],
            WORKSPACE_INITIALIZED_EVENT_KIND,
            empty_object_json
        ],
    )?;
    if event_count < 1 {
        return Err(WorkVcsError::StoreBootstrapInvalid(format!(
            "workspace {workspace_id} Genesis must have a provenance Event"
        )));
    }

    Ok(())
}

fn load_initial_branch(
    connection: &StoreConnection,
    workspace_id: WorkspaceId,
) -> Result<(BranchId, String)> {
    let workspace_id_bytes = workspace_id.raw_bytes();
    let branch = connection
        .inner()
        .query_row(
            "SELECT branch_id, name
             FROM branch
             WHERE workspace_id = ?1
             ORDER BY created_at_us ASC, branch_id ASC
             LIMIT 1",
            params![&workspace_id_bytes[..]],
            |row| Ok((row.get::<_, Vec<u8>>(0)?, row.get::<_, String>(1)?)),
        )
        .optional()
        .map_err(storage_error)?;

    let Some((branch_id_bytes, name)) = branch else {
        return Err(WorkVcsError::StoreBootstrapInvalid(format!(
            "workspace {workspace_id} Genesis has no initial Branch"
        )));
    };
    validate_branch_name(&name).map_err(|error| {
        WorkVcsError::StoreBootstrapInvalid(format!(
            "workspace {workspace_id} has an invalid stored Branch name: {error}"
        ))
    })?;
    Ok((decode_branch_id("branch.branch_id", branch_id_bytes)?, name))
}

fn validate_workspace_display_name(value: &str) -> Result<()> {
    if value.is_empty() {
        Err(WorkVcsError::WorkspaceInvalid(
            "workspace display name must not be empty".to_owned(),
        ))
    } else {
        Ok(())
    }
}

fn validate_created_at_us(column: &str, value: i64) -> Result<()> {
    if value <= 0 {
        return Err(WorkVcsError::StoreBootstrapInvalid(format!(
            "{column} must be positive, found {value}"
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

fn empty_work_state_digest() -> Digest {
    work_state_mapping_digest(&WorkState::empty())
}

fn require_count<P: Params>(
    connection: &StoreConnection,
    label: &str,
    expected: i64,
    sql: &str,
    params: P,
) -> Result<()> {
    let actual = query_count(connection, sql, params)?;
    if actual == expected {
        Ok(())
    } else {
        Err(WorkVcsError::StoreBootstrapInvalid(format!(
            "{label} expected count {expected}, found {actual}"
        )))
    }
}

fn query_count<P: Params>(connection: &StoreConnection, sql: &str, params: P) -> Result<i64> {
    connection
        .inner()
        .query_row(sql, params, |row| row.get(0))
        .map_err(storage_error)
}

fn decode_workspace_id(column: &str, bytes: Vec<u8>) -> Result<WorkspaceId> {
    let bytes = decode_16(column, bytes)?;
    WorkspaceId::from_bytes(bytes).map_err(|error| {
        WorkVcsError::StoreBootstrapInvalid(format!("{column} is not a UUIDv7 value: {error}"))
    })
}

fn decode_commit_id(column: &str, bytes: Vec<u8>) -> Result<CommitId> {
    let bytes = decode_16(column, bytes)?;
    CommitId::from_bytes(bytes).map_err(|error| {
        WorkVcsError::StoreBootstrapInvalid(format!("{column} is not a UUIDv7 value: {error}"))
    })
}

fn decode_changeset_id(column: &str, bytes: Vec<u8>) -> Result<ChangeSetId> {
    let bytes = decode_16(column, bytes)?;
    ChangeSetId::from_bytes(bytes).map_err(|error| {
        WorkVcsError::StoreBootstrapInvalid(format!("{column} is not a UUIDv7 value: {error}"))
    })
}

fn decode_branch_id(column: &str, bytes: Vec<u8>) -> Result<BranchId> {
    let bytes = decode_16(column, bytes)?;
    BranchId::from_bytes(bytes).map_err(|error| {
        WorkVcsError::StoreBootstrapInvalid(format!("{column} is not a UUIDv7 value: {error}"))
    })
}

fn decode_16(column: &str, bytes: Vec<u8>) -> Result<[u8; 16]> {
    bytes.try_into().map_err(|bytes: Vec<u8>| {
        WorkVcsError::StoreBootstrapInvalid(format!(
            "{column} must be 16 bytes, found {}",
            bytes.len()
        ))
    })
}

fn decode_digest(column: &str, bytes: Vec<u8>) -> Result<Digest> {
    let bytes = bytes.try_into().map_err(|bytes: Vec<u8>| {
        WorkVcsError::StoreBootstrapInvalid(format!(
            "{column} must be 32 bytes, found {}",
            bytes.len()
        ))
    })?;
    Ok(Digest::from_bytes(bytes))
}
