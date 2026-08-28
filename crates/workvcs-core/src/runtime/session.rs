use super::claim;
use crate::canonical::{CanonicalValue, WorkState, canonical_bytes, parse_canonical_json};
use crate::error::{Result, WorkVcsError, storage_error};
use crate::history;
use crate::identity::{
    BranchId, CommitId, EntityId, EventId, RelationId, SessionDiffId, SessionId, WorkspaceId,
};
use crate::store::{StoreConnection, current_epoch_micros};
use rusqlite::{Connection, OptionalExtension, Transaction, TransactionBehavior, params};

const ACTIVE_BRANCH_LIFECYCLE_STATE: &str = "active";
const SESSION_OBJECT_KIND: &str = "session";
const ACTIVE_SESSION_LIFECYCLE_STATE: &str = "active";
const ENDED_SESSION_LIFECYCLE_STATE: &str = "ended";
const SESSION_STARTED_EVENT_KIND: &str = "session.started";
const SESSION_FOCUS_SET_EVENT_KIND: &str = "session.focus_set";
const SESSION_FOCUS_CLEARED_EVENT_KIND: &str = "session.focus_cleared";
const SESSION_ENDED_EVENT_KIND: &str = "session.ended";
const SESSION_DIFF_OBJECT_KIND: &str = "session_diff";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SessionLifecycleState {
    Active,
    Ended,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SessionFocusPathEntry {
    pub path_entity_id: EntityId,
    pub incoming_relation_id: Option<RelationId>,
}

impl SessionFocusPathEntry {
    pub fn new(path_entity_id: EntityId, incoming_relation_id: Option<RelationId>) -> Self {
        Self {
            path_entity_id,
            incoming_relation_id,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SessionFocus {
    pub focus_entity_id: EntityId,
    pub path: Vec<SessionFocusPathEntry>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SessionFocusOptions {
    session_id: SessionId,
    focus_entity_id: EntityId,
    path: Vec<SessionFocusPathEntry>,
}

impl SessionFocusOptions {
    pub fn new(session_id: SessionId, focus_entity_id: EntityId) -> Self {
        Self {
            session_id,
            focus_entity_id,
            path: Vec::new(),
        }
    }

    pub fn with_path(mut self, path: Vec<SessionFocusPathEntry>) -> Self {
        self.path = path;
        self
    }

    pub fn session_id(&self) -> SessionId {
        self.session_id
    }

    pub fn focus_entity_id(&self) -> EntityId {
        self.focus_entity_id
    }

    pub fn path(&self) -> &[SessionFocusPathEntry] {
        &self.path
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SessionStartOptions {
    active_workspace_id: WorkspaceId,
    active_branch_id: BranchId,
    metadata: CanonicalValue,
}

impl SessionStartOptions {
    pub fn new(active_workspace_id: WorkspaceId, active_branch_id: BranchId) -> Result<Self> {
        Ok(Self {
            active_workspace_id,
            active_branch_id,
            metadata: CanonicalValue::object(Vec::new())?,
        })
    }

    pub fn with_metadata(mut self, metadata: CanonicalValue) -> Result<Self> {
        require_object_value("session metadata", &metadata)?;
        self.metadata = metadata;
        Ok(self)
    }

    pub fn active_workspace_id(&self) -> WorkspaceId {
        self.active_workspace_id
    }

    pub fn active_branch_id(&self) -> BranchId {
        self.active_branch_id
    }

    pub fn metadata(&self) -> &CanonicalValue {
        &self.metadata
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SessionEndOptions {
    session_id: SessionId,
    summary: CanonicalValue,
}

impl SessionEndOptions {
    pub fn new(session_id: SessionId) -> Result<Self> {
        Ok(Self {
            session_id,
            summary: CanonicalValue::object(Vec::new())?,
        })
    }

    pub fn with_summary(mut self, summary: CanonicalValue) -> Result<Self> {
        require_object_value("session end summary", &summary)?;
        self.summary = summary;
        Ok(self)
    }

    pub fn session_id(&self) -> SessionId {
        self.session_id
    }

    pub fn summary(&self) -> &CanonicalValue {
        &self.summary
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SessionStartResult {
    pub session_id: SessionId,
    pub workspace_id: WorkspaceId,
    pub branch_id: BranchId,
    pub started_at_us: i64,
    pub state: SessionSnapshot,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SessionFocusUpdateResult {
    pub session_id: SessionId,
    pub occurred_at_us: i64,
    pub state: SessionSnapshot,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SessionEndResult {
    pub session_id: SessionId,
    pub session_diff_id: SessionDiffId,
    pub ended_at_us: i64,
    pub summary: CanonicalValue,
    pub state: SessionSnapshot,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SessionSnapshot {
    pub session_id: SessionId,
    pub lifecycle_state: SessionLifecycleState,
    pub started_at_us: i64,
    pub last_activity_at_us: Option<i64>,
    pub metadata: CanonicalValue,
    pub active_workspace_id: Option<WorkspaceId>,
    pub active_branch_id: Option<BranchId>,
    pub context_workspaces: Vec<WorkspaceId>,
    pub focus: Option<SessionFocus>,
    pub session_diff_id: Option<SessionDiffId>,
}

pub(crate) fn start_session(
    connection: &mut StoreConnection,
    options: &SessionStartOptions,
) -> Result<SessionStartResult> {
    connection.verify_foreign_keys()?;
    let metadata_json = canonical_object_json("session metadata", options.metadata())?;
    let runtime_json = active_runtime_json()?;

    let session_id = SessionId::new_v7();
    let event_id = EventId::new_v7();
    let now_us = current_epoch_micros()?;
    let event_payload_json = session_started_payload_json(
        session_id,
        options.active_workspace_id(),
        options.active_branch_id(),
    )?;

    let session_id_bytes = session_id.raw_bytes();
    let event_id_bytes = event_id.raw_bytes();
    let workspace_id_bytes = options.active_workspace_id().raw_bytes();
    let branch_id_bytes = options.active_branch_id().raw_bytes();

    let transaction = connection
        .inner_mut()
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(storage_error)?;

    ensure_workspace_exists(&transaction, options.active_workspace_id())?;
    let branch = load_active_branch(&transaction, options.active_branch_id())?;
    if branch.workspace_id != options.active_workspace_id() {
        return Err(WorkVcsError::SessionInvalid(format!(
            "branch {} belongs to workspace {}, not active workspace {}",
            options.active_branch_id(),
            branch.workspace_id,
            options.active_workspace_id()
        )));
    }

    transaction
        .execute(
            "INSERT INTO object_identity(object_id, object_kind, created_at_us)
             VALUES (?1, ?2, ?3)",
            params![&session_id_bytes[..], SESSION_OBJECT_KIND, now_us],
        )
        .map_err(storage_error)?;
    transaction
        .execute(
            "INSERT INTO session(session_id, started_at_us, metadata_json)
             VALUES (?1, ?2, ?3)",
            params![&session_id_bytes[..], now_us, metadata_json],
        )
        .map_err(storage_error)?;
    transaction
        .execute(
            "INSERT INTO session_runtime(
                session_id,
                active_workspace_id,
                active_branch_id,
                last_activity_at_us,
                runtime_json
             )
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                &session_id_bytes[..],
                &workspace_id_bytes[..],
                &branch_id_bytes[..],
                now_us,
                runtime_json
            ],
        )
        .map_err(storage_error)?;
    transaction
        .execute(
            "INSERT INTO session_context_workspace(session_id, workspace_id)
             VALUES (?1, ?2)",
            params![&session_id_bytes[..], &workspace_id_bytes[..]],
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
             VALUES (?1, ?2, NULL, ?3, ?4, ?5, ?6)",
            params![
                &event_id_bytes[..],
                &workspace_id_bytes[..],
                &session_id_bytes[..],
                SESSION_STARTED_EVENT_KIND,
                now_us,
                event_payload_json
            ],
        )
        .map_err(storage_error)?;

    transaction.commit().map_err(storage_error)?;

    let state = session_snapshot(connection, session_id)?;
    Ok(SessionStartResult {
        session_id,
        workspace_id: options.active_workspace_id(),
        branch_id: options.active_branch_id(),
        started_at_us: now_us,
        state,
    })
}

pub(crate) fn set_session_focus(
    connection: &mut StoreConnection,
    options: &SessionFocusOptions,
) -> Result<SessionFocusUpdateResult> {
    connection.verify_foreign_keys()?;
    let active = active_session_projection(connection, options.session_id())?;
    let branch_head = history::branch_head(connection, active.active_branch_id)?;
    if branch_head.workspace_id != active.active_workspace_id {
        return Err(WorkVcsError::SessionInvalid(format!(
            "session {} active branch {} belongs to workspace {}, not active workspace {}",
            options.session_id(),
            active.active_branch_id,
            branch_head.workspace_id,
            active.active_workspace_id
        )));
    }
    if branch_head.lifecycle_state != ACTIVE_BRANCH_LIFECYCLE_STATE {
        return Err(WorkVcsError::SessionInvalid(format!(
            "session {} active branch {} has lifecycle state {:?}",
            options.session_id(),
            active.active_branch_id,
            branch_head.lifecycle_state
        )));
    }
    let replayed = history::state_at(connection, branch_head.head_commit_id)?;
    validate_focus_against_work_state(&replayed.state, options)?;

    let now_us = current_epoch_micros()?;
    let event_id = EventId::new_v7();
    let event_payload_json = session_focus_set_payload_json(options)?;
    let event_id_bytes = event_id.raw_bytes();
    let session_id_bytes = options.session_id().raw_bytes();
    let workspace_id_bytes = active.active_workspace_id.raw_bytes();
    let focus_entity_id_bytes = options.focus_entity_id().raw_bytes();

    let transaction = connection
        .inner_mut()
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(storage_error)?;
    let current_runtime =
        load_active_session_runtime_for_update(&transaction, options.session_id())?;
    if current_runtime.active_workspace_id != active.active_workspace_id
        || current_runtime.active_branch_id != active.active_branch_id
    {
        return Err(WorkVcsError::SessionInvalid(format!(
            "session {} active target changed before focus could be written",
            options.session_id()
        )));
    }
    let current_branch = load_active_branch(&transaction, active.active_branch_id)?;
    if current_branch.workspace_id != active.active_workspace_id {
        return Err(WorkVcsError::SessionInvalid(format!(
            "session {} active branch {} belongs to workspace {}, not active workspace {}",
            options.session_id(),
            active.active_branch_id,
            current_branch.workspace_id,
            active.active_workspace_id
        )));
    }
    if current_branch.head_commit_id != branch_head.head_commit_id {
        return Err(WorkVcsError::BranchHeadConflict(format!(
            "branch {} head changed before session {} focus could be written",
            active.active_branch_id,
            options.session_id()
        )));
    }

    transaction
        .execute(
            "DELETE FROM session_focus_path
             WHERE session_id = ?1",
            params![&session_id_bytes[..]],
        )
        .map_err(storage_error)?;
    transaction
        .execute(
            "DELETE FROM session_focus
             WHERE session_id = ?1",
            params![&session_id_bytes[..]],
        )
        .map_err(storage_error)?;
    transaction
        .execute(
            "INSERT INTO session_focus(session_id, focus_entity_id)
             VALUES (?1, ?2)",
            params![&session_id_bytes[..], &focus_entity_id_bytes[..]],
        )
        .map_err(storage_error)?;

    for (index, entry) in options.path().iter().enumerate() {
        let ordinal = i64::try_from(index).map_err(|_| {
            WorkVcsError::SessionInvalid("session focus path is too long".to_owned())
        })?;
        let path_entity_id_bytes = entry.path_entity_id.raw_bytes();
        let incoming_relation_id_bytes = entry
            .incoming_relation_id
            .map(|relation_id| relation_id.raw_bytes());
        let incoming_relation_id_param =
            incoming_relation_id_bytes.as_ref().map(|bytes| &bytes[..]);
        transaction
            .execute(
                "INSERT INTO session_focus_path(
                    session_id,
                    ordinal,
                    path_entity_id,
                    incoming_relation_id
                 )
                 VALUES (?1, ?2, ?3, ?4)",
                params![
                    &session_id_bytes[..],
                    ordinal,
                    &path_entity_id_bytes[..],
                    incoming_relation_id_param
                ],
            )
            .map_err(storage_error)?;
    }

    update_session_activity(&transaction, options.session_id(), now_us)?;
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
             VALUES (?1, ?2, NULL, ?3, ?4, ?5, ?6)",
            params![
                &event_id_bytes[..],
                &workspace_id_bytes[..],
                &session_id_bytes[..],
                SESSION_FOCUS_SET_EVENT_KIND,
                now_us,
                event_payload_json
            ],
        )
        .map_err(storage_error)?;

    transaction.commit().map_err(storage_error)?;

    Ok(SessionFocusUpdateResult {
        session_id: options.session_id(),
        occurred_at_us: now_us,
        state: session_snapshot(connection, options.session_id())?,
    })
}

pub(crate) fn clear_session_focus(
    connection: &mut StoreConnection,
    session_id: SessionId,
) -> Result<SessionFocusUpdateResult> {
    connection.verify_foreign_keys()?;
    let now_us = current_epoch_micros()?;
    let event_id = EventId::new_v7();
    let event_payload_json = session_focus_cleared_payload_json(session_id)?;
    let event_id_bytes = event_id.raw_bytes();
    let session_id_bytes = session_id.raw_bytes();

    let transaction = connection
        .inner_mut()
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(storage_error)?;
    let current_runtime = load_active_session_runtime_for_update(&transaction, session_id)?;
    let workspace_id_bytes = current_runtime.active_workspace_id.raw_bytes();
    transaction
        .execute(
            "DELETE FROM session_focus_path
             WHERE session_id = ?1",
            params![&session_id_bytes[..]],
        )
        .map_err(storage_error)?;
    transaction
        .execute(
            "DELETE FROM session_focus
             WHERE session_id = ?1",
            params![&session_id_bytes[..]],
        )
        .map_err(storage_error)?;
    update_session_activity(&transaction, session_id, now_us)?;
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
             VALUES (?1, ?2, NULL, ?3, ?4, ?5, ?6)",
            params![
                &event_id_bytes[..],
                &workspace_id_bytes[..],
                &session_id_bytes[..],
                SESSION_FOCUS_CLEARED_EVENT_KIND,
                now_us,
                event_payload_json
            ],
        )
        .map_err(storage_error)?;

    transaction.commit().map_err(storage_error)?;

    Ok(SessionFocusUpdateResult {
        session_id,
        occurred_at_us: now_us,
        state: session_snapshot(connection, session_id)?,
    })
}

pub(crate) fn end_session(
    connection: &mut StoreConnection,
    options: &SessionEndOptions,
) -> Result<SessionEndResult> {
    connection.verify_foreign_keys()?;
    let summary_json = canonical_object_json("session end summary", options.summary())?;
    let session_diff_id = SessionDiffId::new_v7();
    let event_id = EventId::new_v7();
    let now_us = current_epoch_micros()?;
    let event_payload_json = session_ended_payload_json(options.session_id(), session_diff_id)?;
    let session_id_bytes = options.session_id().raw_bytes();
    let session_diff_id_bytes = session_diff_id.raw_bytes();
    let event_id_bytes = event_id.raw_bytes();

    let transaction = connection
        .inner_mut()
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(storage_error)?;
    let current_runtime =
        load_active_session_runtime_for_update(&transaction, options.session_id())?;
    ensure_no_session_diff(&transaction, options.session_id())?;
    let workspace_id_bytes = current_runtime.active_workspace_id.raw_bytes();

    transaction
        .execute(
            "INSERT INTO object_identity(object_id, object_kind, created_at_us)
             VALUES (?1, ?2, ?3)",
            params![&session_diff_id_bytes[..], SESSION_DIFF_OBJECT_KIND, now_us],
        )
        .map_err(storage_error)?;
    transaction
        .execute(
            "INSERT INTO session_diff(
                session_diff_id,
                session_id,
                created_at_us,
                summary_json,
                detail_content_digest
             )
             VALUES (?1, ?2, ?3, ?4, NULL)",
            params![
                &session_diff_id_bytes[..],
                &session_id_bytes[..],
                now_us,
                summary_json
            ],
        )
        .map_err(storage_error)?;
    transaction
        .execute(
            "DELETE FROM session_focus_path
             WHERE session_id = ?1",
            params![&session_id_bytes[..]],
        )
        .map_err(storage_error)?;
    transaction
        .execute(
            "DELETE FROM session_focus
             WHERE session_id = ?1",
            params![&session_id_bytes[..]],
        )
        .map_err(storage_error)?;
    transaction
        .execute(
            "DELETE FROM session_context_workspace
             WHERE session_id = ?1",
            params![&session_id_bytes[..]],
        )
        .map_err(storage_error)?;
    transaction
        .execute(
            "DELETE FROM session_context_knowledge_space
             WHERE session_id = ?1",
            params![&session_id_bytes[..]],
        )
        .map_err(storage_error)?;
    claim::release_active_claims_for_session_end(&transaction, options.session_id(), now_us)?;
    let removed_runtime = transaction
        .execute(
            "DELETE FROM session_runtime
             WHERE session_id = ?1",
            params![&session_id_bytes[..]],
        )
        .map_err(storage_error)?;
    if removed_runtime != 1 {
        return Err(WorkVcsError::SessionInvalid(format!(
            "session {} runtime cleanup affected {removed_runtime} rows",
            options.session_id()
        )));
    }
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
             VALUES (?1, ?2, NULL, ?3, ?4, ?5, ?6)",
            params![
                &event_id_bytes[..],
                &workspace_id_bytes[..],
                &session_id_bytes[..],
                SESSION_ENDED_EVENT_KIND,
                now_us,
                event_payload_json
            ],
        )
        .map_err(storage_error)?;

    transaction.commit().map_err(storage_error)?;

    Ok(SessionEndResult {
        session_id: options.session_id(),
        session_diff_id,
        ended_at_us: now_us,
        summary: options.summary().clone(),
        state: session_snapshot(connection, options.session_id())?,
    })
}

pub(crate) fn session_snapshot(
    connection: &StoreConnection,
    session_id: SessionId,
) -> Result<SessionSnapshot> {
    connection.verify_foreign_keys()?;
    let transaction = connection
        .inner()
        .unchecked_transaction()
        .map_err(storage_error)?;
    let snapshot = session_snapshot_from_connection(&transaction, session_id)?;
    transaction.commit().map_err(storage_error)?;
    Ok(snapshot)
}

fn session_snapshot_from_connection(
    connection: &Connection,
    session_id: SessionId,
) -> Result<SessionSnapshot> {
    let session = load_session(connection, session_id)?;
    let runtime = load_session_runtime(connection, session_id)?;
    let session_diff_id = load_session_diff_id(connection, session_id)?;
    match (runtime, session_diff_id) {
        (Some(runtime), None) => {
            validate_active_runtime_json(&runtime.runtime_json, session_id)?;
            let active_workspace_id = runtime.active_workspace_id.ok_or_else(|| {
                WorkVcsError::SessionInvalid(format!(
                    "active session {session_id} has no active workspace"
                ))
            })?;
            let active_branch_id = runtime.active_branch_id.ok_or_else(|| {
                WorkVcsError::SessionInvalid(format!(
                    "active session {session_id} has no active branch"
                ))
            })?;
            let context_workspaces = load_context_workspaces(connection, session_id)?;
            if !context_workspaces.contains(&active_workspace_id) {
                return Err(WorkVcsError::SessionInvalid(format!(
                    "active session {session_id} context set does not include active workspace {active_workspace_id}"
                )));
            }
            Ok(SessionSnapshot {
                session_id,
                lifecycle_state: SessionLifecycleState::Active,
                started_at_us: session.started_at_us,
                last_activity_at_us: Some(runtime.last_activity_at_us),
                metadata: session.metadata,
                active_workspace_id: Some(active_workspace_id),
                active_branch_id: Some(active_branch_id),
                context_workspaces,
                focus: load_session_focus(connection, session_id)?,
                session_diff_id: None,
            })
        }
        (None, Some(session_diff_id)) => Ok(SessionSnapshot {
            session_id,
            lifecycle_state: SessionLifecycleState::Ended,
            started_at_us: session.started_at_us,
            last_activity_at_us: None,
            metadata: session.metadata,
            active_workspace_id: None,
            active_branch_id: None,
            context_workspaces: Vec::new(),
            focus: None,
            session_diff_id: Some(session_diff_id),
        }),
        (Some(_), Some(session_diff_id)) => Err(WorkVcsError::SessionInvalid(format!(
            "session {session_id} has active runtime and final session diff {session_diff_id}"
        ))),
        (None, None) => Err(WorkVcsError::SessionInvalid(format!(
            "session {session_id} has no active runtime or final session diff"
        ))),
    }
}

struct BranchRuntimeRow {
    workspace_id: WorkspaceId,
    head_commit_id: CommitId,
}

struct SessionRow {
    started_at_us: i64,
    metadata: CanonicalValue,
}

struct SessionRuntimeRow {
    active_workspace_id: Option<WorkspaceId>,
    active_branch_id: Option<BranchId>,
    last_activity_at_us: i64,
    runtime_json: String,
}

pub(super) struct ActiveSessionProjection {
    pub(super) active_workspace_id: WorkspaceId,
    pub(super) active_branch_id: BranchId,
}

pub(super) fn active_session_projection(
    connection: &StoreConnection,
    session_id: SessionId,
) -> Result<ActiveSessionProjection> {
    let snapshot = session_snapshot(connection, session_id)?;
    if snapshot.lifecycle_state != SessionLifecycleState::Active {
        return Err(WorkVcsError::SessionInvalid(format!(
            "session {session_id} is not active"
        )));
    }
    Ok(ActiveSessionProjection {
        active_workspace_id: snapshot.active_workspace_id.ok_or_else(|| {
            WorkVcsError::SessionInvalid(format!(
                "active session {session_id} has no active workspace"
            ))
        })?,
        active_branch_id: snapshot.active_branch_id.ok_or_else(|| {
            WorkVcsError::SessionInvalid(format!(
                "active session {session_id} has no active branch"
            ))
        })?,
    })
}

fn ensure_workspace_exists(transaction: &Transaction<'_>, workspace_id: WorkspaceId) -> Result<()> {
    let count = transaction
        .query_row(
            "SELECT count(*)
             FROM workspace
             WHERE workspace_id = ?1",
            params![&workspace_id.raw_bytes()[..]],
            |row| row.get::<_, i64>(0),
        )
        .map_err(storage_error)?;
    if count == 1 {
        Ok(())
    } else {
        Err(WorkVcsError::WorkspaceNotFound(format!(
            "workspace {workspace_id} does not exist"
        )))
    }
}

fn load_active_branch(
    transaction: &Transaction<'_>,
    branch_id: BranchId,
) -> Result<BranchRuntimeRow> {
    let row = transaction
        .query_row(
            "SELECT workspace_id, head_commit_id, lifecycle_state
             FROM branch
             WHERE branch_id = ?1",
            params![&branch_id.raw_bytes()[..]],
            |row| {
                Ok((
                    row.get::<_, Vec<u8>>(0)?,
                    row.get::<_, Vec<u8>>(1)?,
                    row.get::<_, String>(2)?,
                ))
            },
        )
        .optional()
        .map_err(storage_error)?;

    let Some((workspace_id, head_commit_id, lifecycle_state)) = row else {
        return Err(WorkVcsError::BranchNotFound(format!(
            "branch {branch_id} does not exist"
        )));
    };
    validate_stored_text("branch.lifecycle_state", &lifecycle_state)?;
    if lifecycle_state != ACTIVE_BRANCH_LIFECYCLE_STATE {
        return Err(WorkVcsError::SessionInvalid(format!(
            "branch {branch_id} has lifecycle state {lifecycle_state:?}"
        )));
    }

    Ok(BranchRuntimeRow {
        workspace_id: decode_workspace_id("branch.workspace_id", workspace_id)?,
        head_commit_id: decode_commit_id("branch.head_commit_id", head_commit_id)?,
    })
}

pub(super) fn load_active_session_runtime_for_update(
    transaction: &Transaction<'_>,
    session_id: SessionId,
) -> Result<ActiveSessionProjection> {
    let row = transaction
        .query_row(
            "SELECT active_workspace_id,
                    active_branch_id,
                    runtime_json
             FROM session_runtime
             WHERE session_id = ?1",
            params![&session_id.raw_bytes()[..]],
            |row| {
                Ok((
                    row.get::<_, Option<Vec<u8>>>(0)?,
                    row.get::<_, Option<Vec<u8>>>(1)?,
                    row.get::<_, String>(2)?,
                ))
            },
        )
        .optional()
        .map_err(storage_error)?;

    let Some((active_workspace_id, active_branch_id, runtime_json)) = row else {
        if session_exists(transaction, session_id)? {
            return Err(WorkVcsError::SessionInvalid(format!(
                "session {session_id} is not active"
            )));
        }
        return Err(WorkVcsError::SessionNotFound(format!(
            "session {session_id} does not exist"
        )));
    };
    validate_active_runtime_json(&runtime_json, session_id)?;
    let active_workspace_id = active_workspace_id
        .map(|bytes| decode_workspace_id("session_runtime.active_workspace_id", bytes))
        .transpose()?
        .ok_or_else(|| {
            WorkVcsError::SessionInvalid(format!(
                "active session {session_id} has no active workspace"
            ))
        })?;
    let active_branch_id = active_branch_id
        .map(|bytes| decode_branch_id("session_runtime.active_branch_id", bytes))
        .transpose()?
        .ok_or_else(|| {
            WorkVcsError::SessionInvalid(format!(
                "active session {session_id} has no active branch"
            ))
        })?;

    Ok(ActiveSessionProjection {
        active_workspace_id,
        active_branch_id,
    })
}

fn session_exists(transaction: &Transaction<'_>, session_id: SessionId) -> Result<bool> {
    let count = transaction
        .query_row(
            "SELECT count(*)
             FROM session
             WHERE session_id = ?1",
            params![&session_id.raw_bytes()[..]],
            |row| row.get::<_, i64>(0),
        )
        .map_err(storage_error)?;
    Ok(count == 1)
}

fn ensure_no_session_diff(transaction: &Transaction<'_>, session_id: SessionId) -> Result<()> {
    let count = transaction
        .query_row(
            "SELECT count(*)
             FROM session_diff
             WHERE session_id = ?1",
            params![&session_id.raw_bytes()[..]],
            |row| row.get::<_, i64>(0),
        )
        .map_err(storage_error)?;
    if count == 0 {
        Ok(())
    } else {
        Err(WorkVcsError::SessionInvalid(format!(
            "session {session_id} already has a final session diff"
        )))
    }
}

pub(super) fn update_session_activity(
    transaction: &Transaction<'_>,
    session_id: SessionId,
    occurred_at_us: i64,
) -> Result<()> {
    let updated = transaction
        .execute(
            "UPDATE session_runtime
             SET last_activity_at_us = ?1
             WHERE session_id = ?2",
            params![occurred_at_us, &session_id.raw_bytes()[..]],
        )
        .map_err(storage_error)?;
    if updated == 1 {
        Ok(())
    } else {
        Err(WorkVcsError::SessionInvalid(format!(
            "session {session_id} activity update affected {updated} rows"
        )))
    }
}

fn load_session(connection: &Connection, session_id: SessionId) -> Result<SessionRow> {
    let row = connection
        .query_row(
            "SELECT started_at_us, metadata_json
             FROM session
             WHERE session_id = ?1",
            params![&session_id.raw_bytes()[..]],
            |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?)),
        )
        .optional()
        .map_err(storage_error)?;

    let Some((started_at_us, metadata_json)) = row else {
        return Err(WorkVcsError::SessionNotFound(format!(
            "session {session_id} does not exist"
        )));
    };

    Ok(SessionRow {
        started_at_us,
        metadata: parse_canonical_object_json("session.metadata_json", &metadata_json)?,
    })
}

fn load_session_runtime(
    connection: &Connection,
    session_id: SessionId,
) -> Result<Option<SessionRuntimeRow>> {
    let row = connection
        .query_row(
            "SELECT active_workspace_id,
                    active_branch_id,
                    last_activity_at_us,
                    runtime_json
             FROM session_runtime
             WHERE session_id = ?1",
            params![&session_id.raw_bytes()[..]],
            |row| {
                Ok((
                    row.get::<_, Option<Vec<u8>>>(0)?,
                    row.get::<_, Option<Vec<u8>>>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, String>(3)?,
                ))
            },
        )
        .optional()
        .map_err(storage_error)?;

    row.map(
        |(active_workspace_id, active_branch_id, last_activity_at_us, runtime_json)| {
            Ok(SessionRuntimeRow {
                active_workspace_id: active_workspace_id
                    .map(|bytes| decode_workspace_id("session_runtime.active_workspace_id", bytes))
                    .transpose()?,
                active_branch_id: active_branch_id
                    .map(|bytes| decode_branch_id("session_runtime.active_branch_id", bytes))
                    .transpose()?,
                last_activity_at_us,
                runtime_json,
            })
        },
    )
    .transpose()
}

fn load_session_diff_id(
    connection: &Connection,
    session_id: SessionId,
) -> Result<Option<SessionDiffId>> {
    let row = connection
        .query_row(
            "SELECT session_diff_id, summary_json
             FROM session_diff
             WHERE session_id = ?1",
            params![&session_id.raw_bytes()[..]],
            |row| Ok((row.get::<_, Vec<u8>>(0)?, row.get::<_, String>(1)?)),
        )
        .optional()
        .map_err(storage_error)?;

    row.map(|(session_diff_id, summary_json)| {
        parse_canonical_object_json("session_diff.summary_json", &summary_json)?;
        decode_session_diff_id("session_diff.session_diff_id", session_diff_id)
    })
    .transpose()
}

fn load_context_workspaces(
    connection: &Connection,
    session_id: SessionId,
) -> Result<Vec<WorkspaceId>> {
    let mut statement = connection
        .prepare(
            "SELECT workspace_id
             FROM session_context_workspace
             WHERE session_id = ?1
             ORDER BY workspace_id",
        )
        .map_err(storage_error)?;
    let rows = statement
        .query_map(params![&session_id.raw_bytes()[..]], |row| {
            row.get::<_, Vec<u8>>(0)
        })
        .map_err(storage_error)?;

    let mut workspaces = Vec::new();
    for row in rows {
        workspaces.push(decode_workspace_id(
            "session_context_workspace.workspace_id",
            row.map_err(storage_error)?,
        )?);
    }
    Ok(workspaces)
}

fn load_session_focus(
    connection: &Connection,
    session_id: SessionId,
) -> Result<Option<SessionFocus>> {
    let focus_entity_id = connection
        .query_row(
            "SELECT focus_entity_id
             FROM session_focus
             WHERE session_id = ?1",
            params![&session_id.raw_bytes()[..]],
            |row| row.get::<_, Vec<u8>>(0),
        )
        .optional()
        .map_err(storage_error)?
        .map(|bytes| decode_entity_id("session_focus.focus_entity_id", bytes))
        .transpose()?;

    let Some(focus_entity_id) = focus_entity_id else {
        return Ok(None);
    };

    let mut statement = connection
        .prepare(
            "SELECT path_entity_id, incoming_relation_id
             FROM session_focus_path
             WHERE session_id = ?1
             ORDER BY ordinal",
        )
        .map_err(storage_error)?;
    let rows = statement
        .query_map(params![&session_id.raw_bytes()[..]], |row| {
            Ok((row.get::<_, Vec<u8>>(0)?, row.get::<_, Option<Vec<u8>>>(1)?))
        })
        .map_err(storage_error)?;

    let mut path = Vec::new();
    for row in rows {
        let (path_entity_id, incoming_relation_id) = row.map_err(storage_error)?;
        path.push(SessionFocusPathEntry {
            path_entity_id: decode_entity_id("session_focus_path.path_entity_id", path_entity_id)?,
            incoming_relation_id: incoming_relation_id
                .map(|bytes| decode_relation_id("session_focus_path.incoming_relation_id", bytes))
                .transpose()?,
        });
    }

    Ok(Some(SessionFocus {
        focus_entity_id,
        path,
    }))
}

fn active_runtime_json() -> Result<String> {
    canonical_json_string(&CanonicalValue::object(vec![(
        "lifecycle_state".to_owned(),
        CanonicalValue::String(ACTIVE_SESSION_LIFECYCLE_STATE.to_owned()),
    )])?)
}

fn validate_active_runtime_json(runtime_json: &str, session_id: SessionId) -> Result<()> {
    let value = parse_canonical_object_json("session_runtime.runtime_json", runtime_json)?;
    let CanonicalValue::Object(entries) = value else {
        unreachable!("parse_canonical_object_json returns only objects");
    };
    if entries.len() != 1 {
        return Err(WorkVcsError::SessionInvalid(format!(
            "active session {session_id} runtime_json must contain only lifecycle_state"
        )));
    }
    match &entries[0] {
        (key, CanonicalValue::String(value))
            if key == "lifecycle_state" && value == ACTIVE_SESSION_LIFECYCLE_STATE =>
        {
            Ok(())
        }
        _ => Err(WorkVcsError::SessionInvalid(format!(
            "active session {session_id} runtime_json must project lifecycle_state active"
        ))),
    }
}

fn session_started_payload_json(
    session_id: SessionId,
    workspace_id: WorkspaceId,
    branch_id: BranchId,
) -> Result<String> {
    canonical_json_string(&CanonicalValue::object(vec![
        (
            "active_branch_id".to_owned(),
            CanonicalValue::String(branch_id.to_string()),
        ),
        (
            "active_workspace_id".to_owned(),
            CanonicalValue::String(workspace_id.to_string()),
        ),
        (
            "lifecycle_state".to_owned(),
            CanonicalValue::String(ACTIVE_SESSION_LIFECYCLE_STATE.to_owned()),
        ),
        (
            "session_id".to_owned(),
            CanonicalValue::String(session_id.to_string()),
        ),
    ])?)
}

fn session_focus_set_payload_json(options: &SessionFocusOptions) -> Result<String> {
    canonical_json_string(&CanonicalValue::object(vec![
        (
            "focus_entity_id".to_owned(),
            CanonicalValue::String(options.focus_entity_id().to_string()),
        ),
        (
            "path".to_owned(),
            CanonicalValue::Array(focus_path_payload_values(options.path())?),
        ),
        (
            "session_id".to_owned(),
            CanonicalValue::String(options.session_id().to_string()),
        ),
    ])?)
}

fn session_focus_cleared_payload_json(session_id: SessionId) -> Result<String> {
    canonical_json_string(&CanonicalValue::object(vec![(
        "session_id".to_owned(),
        CanonicalValue::String(session_id.to_string()),
    )])?)
}

fn session_ended_payload_json(
    session_id: SessionId,
    session_diff_id: SessionDiffId,
) -> Result<String> {
    canonical_json_string(&CanonicalValue::object(vec![
        (
            "lifecycle_state".to_owned(),
            CanonicalValue::String(ENDED_SESSION_LIFECYCLE_STATE.to_owned()),
        ),
        (
            "session_diff_id".to_owned(),
            CanonicalValue::String(session_diff_id.to_string()),
        ),
        (
            "session_id".to_owned(),
            CanonicalValue::String(session_id.to_string()),
        ),
    ])?)
}

fn focus_path_payload_values(path: &[SessionFocusPathEntry]) -> Result<Vec<CanonicalValue>> {
    path.iter()
        .map(|entry| {
            let incoming_relation_id = entry
                .incoming_relation_id
                .map(|relation_id| CanonicalValue::String(relation_id.to_string()))
                .unwrap_or(CanonicalValue::Null);
            CanonicalValue::object(vec![
                ("incoming_relation_id".to_owned(), incoming_relation_id),
                (
                    "path_entity_id".to_owned(),
                    CanonicalValue::String(entry.path_entity_id.to_string()),
                ),
            ])
        })
        .collect()
}

fn validate_focus_against_work_state(
    state: &WorkState,
    options: &SessionFocusOptions,
) -> Result<()> {
    if !work_state_contains_entity(state, options.focus_entity_id()) {
        return Err(WorkVcsError::SessionInvalid(format!(
            "focus entity {} is not present at the active branch head",
            options.focus_entity_id()
        )));
    }
    for entry in options.path() {
        if !work_state_contains_entity(state, entry.path_entity_id) {
            return Err(WorkVcsError::SessionInvalid(format!(
                "focus path entity {} is not present at the active branch head",
                entry.path_entity_id
            )));
        }
        if let Some(relation_id) = entry.incoming_relation_id
            && !work_state_contains_relation(state, relation_id)
        {
            return Err(WorkVcsError::SessionInvalid(format!(
                "focus path relation {relation_id} is not present at the active branch head"
            )));
        }
    }
    Ok(())
}

fn work_state_contains_entity(state: &WorkState, entity_id: EntityId) -> bool {
    state
        .entities()
        .iter()
        .any(|(current_entity_id, _)| *current_entity_id == entity_id)
}

fn work_state_contains_relation(state: &WorkState, relation_id: RelationId) -> bool {
    state
        .relations()
        .iter()
        .any(|(current_relation_id, _)| *current_relation_id == relation_id)
}

fn require_object_value(label: &str, value: &CanonicalValue) -> Result<()> {
    match value {
        CanonicalValue::Object(_) => Ok(()),
        _ => Err(WorkVcsError::SessionInvalid(format!(
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
        WorkVcsError::SessionInvalid(format!("{label} is not valid canonical JSON: {error}"))
    })?;
    require_object_value(label, &parsed)?;
    let encoded = canonical_json_string(&parsed)?;
    if input != encoded {
        return Err(WorkVcsError::SessionInvalid(format!(
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

fn validate_stored_text(label: &str, value: &str) -> Result<()> {
    if value.is_empty() || value.trim() != value {
        return Err(WorkVcsError::SessionInvalid(format!(
            "{label} is not stored as non-empty trimmed text"
        )));
    }
    if value
        .as_bytes()
        .iter()
        .any(|byte| *byte == b'\0' || *byte < 0x20 || *byte == 0x7f)
    {
        return Err(WorkVcsError::SessionInvalid(format!(
            "{label} contains NUL or ASCII control characters"
        )));
    }
    Ok(())
}

fn decode_workspace_id(column: &str, bytes: Vec<u8>) -> Result<WorkspaceId> {
    let bytes = decode_16(column, bytes)?;
    WorkspaceId::from_bytes(bytes).map_err(|error| {
        WorkVcsError::SessionInvalid(format!("{column} is not a UUIDv7 value: {error}"))
    })
}

fn decode_branch_id(column: &str, bytes: Vec<u8>) -> Result<BranchId> {
    let bytes = decode_16(column, bytes)?;
    BranchId::from_bytes(bytes).map_err(|error| {
        WorkVcsError::SessionInvalid(format!("{column} is not a UUIDv7 value: {error}"))
    })
}

fn decode_commit_id(column: &str, bytes: Vec<u8>) -> Result<CommitId> {
    let bytes = decode_16(column, bytes)?;
    CommitId::from_bytes(bytes).map_err(|error| {
        WorkVcsError::SessionInvalid(format!("{column} is not a UUIDv7 value: {error}"))
    })
}

fn decode_session_diff_id(column: &str, bytes: Vec<u8>) -> Result<SessionDiffId> {
    let bytes = decode_16(column, bytes)?;
    SessionDiffId::from_bytes(bytes).map_err(|error| {
        WorkVcsError::SessionInvalid(format!("{column} is not a UUIDv7 value: {error}"))
    })
}

fn decode_entity_id(column: &str, bytes: Vec<u8>) -> Result<EntityId> {
    let bytes = decode_16(column, bytes)?;
    EntityId::from_bytes(bytes).map_err(|error| {
        WorkVcsError::SessionInvalid(format!("{column} is not a UUIDv7 value: {error}"))
    })
}

fn decode_relation_id(column: &str, bytes: Vec<u8>) -> Result<RelationId> {
    let bytes = decode_16(column, bytes)?;
    RelationId::from_bytes(bytes).map_err(|error| {
        WorkVcsError::SessionInvalid(format!("{column} is not a UUIDv7 value: {error}"))
    })
}

fn decode_16(column: &str, bytes: Vec<u8>) -> Result<[u8; 16]> {
    bytes.try_into().map_err(|bytes: Vec<u8>| {
        WorkVcsError::SessionInvalid(format!("{column} must be 16 bytes, found {}", bytes.len()))
    })
}
