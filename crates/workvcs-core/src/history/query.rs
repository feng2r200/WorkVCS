use crate::canonical::{canonical_bytes, content_object_digest, parse_canonical_json};
use crate::error::{Result, WorkVcsError, storage_error};
use crate::identity::{
    BranchId, ChangeSetId, CommitId, Digest, EntityId, EventId, OperationId, RelationId, SessionId,
    WorkspaceId,
};
use crate::store::StoreConnection;
use rusqlite::{OptionalExtension, Params, Row, params};
use std::collections::HashSet;

const GENESIS_COMMIT_KIND: &str = "genesis";
const NORMAL_COMMIT_KIND: &str = "normal";
const MERGE_COMMIT_KIND: &str = "merge";
const PRIMARY_PARENT_ROLE: &str = "primary";
const SECONDARY_PARENT_ROLE: &str = "secondary";
const ENTITY_SUBJECT_FAMILY: &str = "entity";
const RELATION_SUBJECT_FAMILY: &str = "relation";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BranchHead {
    pub workspace_id: WorkspaceId,
    pub branch_id: BranchId,
    pub name: String,
    pub head_commit_id: CommitId,
    pub head_changeset_id: ChangeSetId,
    pub head_commit_kind: String,
    pub head_operation_type: String,
    pub head_operation_schema_version: i64,
    pub head_committed_at_us: i64,
    pub head_changeset_created_at_us: i64,
    pub lifecycle_state: String,
    pub state_digest: Digest,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HistoryStart {
    Branch(BranchId),
    Commit(CommitId),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HistoryQueryOptions {
    start: HistoryStart,
    limit: Option<usize>,
}

impl HistoryQueryOptions {
    pub fn from_branch(branch_id: BranchId) -> Self {
        Self {
            start: HistoryStart::Branch(branch_id),
            limit: None,
        }
    }

    pub fn from_commit(commit_id: CommitId) -> Self {
        Self {
            start: HistoryStart::Commit(commit_id),
            limit: None,
        }
    }

    pub fn with_limit(mut self, limit: usize) -> Result<Self> {
        if limit == 0 {
            return Err(WorkVcsError::QueryInvalid(
                "history limit must be greater than zero".to_owned(),
            ));
        }
        self.limit = Some(limit);
        Ok(self)
    }

    pub fn start(&self) -> HistoryStart {
        self.start
    }

    pub fn limit(&self) -> Option<usize> {
        self.limit
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HistoryQueryResult {
    pub start_commit_id: CommitId,
    pub entries: Vec<HistoryEntry>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HistoryEntry {
    pub workspace_id: WorkspaceId,
    pub commit_id: CommitId,
    pub changeset_id: ChangeSetId,
    pub commit_kind: String,
    pub state_digest: Digest,
    pub committed_at_us: i64,
    pub operation_type: String,
    pub operation_schema_version: i64,
    pub changeset_created_at_us: i64,
    pub parent_commit_id: Option<CommitId>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommitSnapshot {
    pub workspace_id: WorkspaceId,
    pub commit_id: CommitId,
    pub changeset_id: ChangeSetId,
    pub commit_kind: String,
    pub state_digest: Digest,
    pub committed_at_us: i64,
    pub operation_type: String,
    pub operation_schema_version: i64,
    pub changeset_created_at_us: i64,
    pub origin_session_id: Option<SessionId>,
    pub parents: Vec<CommitParentSnapshot>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommitParentSnapshot {
    pub parent_ordinal: i64,
    pub parent_role: String,
    pub parent_commit_id: CommitId,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChangeSetSnapshot {
    pub workspace_id: WorkspaceId,
    pub changeset_id: ChangeSetId,
    pub operation_type: String,
    pub operation_schema_version: i64,
    pub operation_payload_json: String,
    pub operation_payload_digest: Digest,
    pub operation_payload_size_bytes: i64,
    pub rationale_json: String,
    pub rationale_digest: Digest,
    pub rationale_size_bytes: i64,
    pub origin_session_id: Option<SessionId>,
    pub created_at_us: i64,
    pub change_operation_count: i64,
    pub event_count: i64,
    pub commits: Vec<ChangeSetCommitSnapshot>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChangeSetCommitSnapshot {
    pub commit_id: CommitId,
    pub commit_kind: String,
    pub state_digest: Digest,
    pub committed_at_us: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChangeOperationListResult {
    pub workspace_id: WorkspaceId,
    pub changeset_id: ChangeSetId,
    pub operations: Vec<ChangeOperationSnapshot>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChangeOperationSnapshot {
    pub operation_id: OperationId,
    pub ordinal: i64,
    pub subject: ChangeOperationSubject,
    pub operation_payload_json: String,
    pub operation_payload_digest: Digest,
    pub operation_payload_size_bytes: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ChangeOperationSubject {
    Entity(EntityId),
    Relation(RelationId),
}

impl ChangeOperationSubject {
    pub fn family(&self) -> &'static str {
        match self {
            Self::Entity(_) => ENTITY_SUBJECT_FAMILY,
            Self::Relation(_) => RELATION_SUBJECT_FAMILY,
        }
    }

    pub fn object_id(&self) -> String {
        match self {
            Self::Entity(entity_id) => entity_id.to_string(),
            Self::Relation(relation_id) => relation_id.to_string(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EventListTarget {
    ChangeSet(ChangeSetId),
    Session(SessionId),
    Workspace(WorkspaceId),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EventListOptions {
    target: EventListTarget,
    limit: Option<usize>,
}

impl EventListOptions {
    pub fn for_changeset(changeset_id: ChangeSetId) -> Self {
        Self {
            target: EventListTarget::ChangeSet(changeset_id),
            limit: None,
        }
    }

    pub fn for_session(session_id: SessionId) -> Self {
        Self {
            target: EventListTarget::Session(session_id),
            limit: None,
        }
    }

    pub fn for_workspace(workspace_id: WorkspaceId) -> Self {
        Self {
            target: EventListTarget::Workspace(workspace_id),
            limit: None,
        }
    }

    pub fn with_limit(mut self, limit: usize) -> Result<Self> {
        if limit == 0 {
            return Err(WorkVcsError::QueryInvalid(
                "event list limit must be greater than zero".to_owned(),
            ));
        }
        self.limit = Some(limit);
        Ok(self)
    }

    pub fn target(&self) -> EventListTarget {
        self.target
    }

    pub fn limit(&self) -> Option<usize> {
        self.limit
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EventListResult {
    pub events: Vec<EventSnapshot>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EventSnapshot {
    pub event_id: EventId,
    pub workspace_id: Option<WorkspaceId>,
    pub changeset_id: Option<ChangeSetId>,
    pub session_id: Option<SessionId>,
    pub event_kind: String,
    pub occurred_at_us: i64,
    pub payload_json: String,
    pub payload_digest: Digest,
    pub payload_size_bytes: i64,
}

struct BranchRow {
    workspace_id: WorkspaceId,
    name: String,
    head_commit_id: CommitId,
    lifecycle_state: String,
}

pub(crate) fn branch_head(connection: &StoreConnection, branch_id: BranchId) -> Result<BranchHead> {
    let branch = load_branch(connection, branch_id)?;
    let head = load_history_entry(connection, branch.head_commit_id)?;
    if head.workspace_id != branch.workspace_id {
        return Err(WorkVcsError::QueryInvalid(format!(
            "branch {branch_id} head {} belongs to workspace {}, not {}",
            branch.head_commit_id, head.workspace_id, branch.workspace_id
        )));
    }

    Ok(BranchHead {
        workspace_id: branch.workspace_id,
        branch_id,
        name: branch.name,
        head_commit_id: branch.head_commit_id,
        head_changeset_id: head.changeset_id,
        head_commit_kind: head.commit_kind,
        head_operation_type: head.operation_type,
        head_operation_schema_version: head.operation_schema_version,
        head_committed_at_us: head.committed_at_us,
        head_changeset_created_at_us: head.changeset_created_at_us,
        lifecycle_state: branch.lifecycle_state,
        state_digest: head.state_digest,
    })
}

pub(crate) fn query_history(
    connection: &StoreConnection,
    options: &HistoryQueryOptions,
) -> Result<HistoryQueryResult> {
    let start_commit_id = match options.start() {
        HistoryStart::Branch(branch_id) => branch_head(connection, branch_id)?.head_commit_id,
        HistoryStart::Commit(commit_id) => commit_id,
    };
    let mut entries = Vec::new();
    let mut visiting = HashSet::new();
    let mut next_commit_id = Some(start_commit_id);

    while let Some(commit_id) = next_commit_id {
        if let Some(limit) = options.limit()
            && entries.len() >= limit
        {
            break;
        }
        if !visiting.insert(commit_id.raw_bytes()) {
            return Err(WorkVcsError::QueryInvalid(format!(
                "cycle detected while reading history at commit {commit_id}"
            )));
        }

        let mut entry = load_history_entry(connection, commit_id)?;
        let parent_commit_id =
            load_first_parent(connection, commit_id, entry.commit_kind.as_str())?;
        entry.parent_commit_id = parent_commit_id;
        next_commit_id = parent_commit_id;
        entries.push(entry);
    }

    Ok(HistoryQueryResult {
        start_commit_id,
        entries,
    })
}

pub(crate) fn commit(connection: &StoreConnection, commit_id: CommitId) -> Result<CommitSnapshot> {
    let entry = load_history_entry(connection, commit_id)?;
    let origin_session_id =
        load_changeset_origin_session(connection, entry.workspace_id, entry.changeset_id)?;
    let parents = load_commit_parents(connection, commit_id)?;
    validate_commit_parent_shape(commit_id, &entry.commit_kind, &parents)?;

    Ok(CommitSnapshot {
        workspace_id: entry.workspace_id,
        commit_id: entry.commit_id,
        changeset_id: entry.changeset_id,
        commit_kind: entry.commit_kind,
        state_digest: entry.state_digest,
        committed_at_us: entry.committed_at_us,
        operation_type: entry.operation_type,
        operation_schema_version: entry.operation_schema_version,
        changeset_created_at_us: entry.changeset_created_at_us,
        origin_session_id,
        parents,
    })
}

pub(crate) fn changeset(
    connection: &StoreConnection,
    changeset_id: ChangeSetId,
) -> Result<ChangeSetSnapshot> {
    let row = connection
        .inner()
        .query_row(
            "SELECT workspace_id,
                    operation_type,
                    operation_schema_version,
                    operation_payload_json,
                    rationale_json,
                    origin_session_id,
                    created_at_us
             FROM changeset
             WHERE changeset_id = ?1",
            params![&changeset_id.raw_bytes()[..]],
            |row| {
                Ok((
                    row.get::<_, Vec<u8>>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, Option<Vec<u8>>>(5)?,
                    row.get::<_, i64>(6)?,
                ))
            },
        )
        .optional()
        .map_err(storage_error)?;

    let Some((
        workspace_id,
        operation_type,
        operation_schema_version,
        operation_payload_json,
        rationale_json,
        origin_session_id,
        created_at_us,
    )) = row
    else {
        return Err(WorkVcsError::QueryInvalid(format!(
            "ChangeSet {changeset_id} does not exist"
        )));
    };
    validate_stored_text("changeset.operation_type", &operation_type)?;
    if operation_schema_version <= 0 {
        return Err(WorkVcsError::QueryInvalid(format!(
            "ChangeSet {changeset_id} has invalid operation schema version {operation_schema_version}"
        )));
    }
    validate_canonical_json_text("changeset.operation_payload_json", &operation_payload_json)?;
    validate_canonical_json_text("changeset.rationale_json", &rationale_json)?;
    validate_nonnegative("changeset.created_at_us", created_at_us)?;

    let change_operation_count = count_rows(
        connection,
        "SELECT count(*)
         FROM change_operation
         WHERE changeset_id = ?1",
        params![&changeset_id.raw_bytes()[..]],
    )?;
    let event_count = count_rows(
        connection,
        "SELECT count(*)
         FROM event
         WHERE changeset_id = ?1",
        params![&changeset_id.raw_bytes()[..]],
    )?;

    Ok(ChangeSetSnapshot {
        workspace_id: decode_workspace_id("changeset.workspace_id", workspace_id)?,
        changeset_id,
        operation_type,
        operation_schema_version,
        operation_payload_digest: content_object_digest(operation_payload_json.as_bytes()),
        operation_payload_size_bytes: usize_to_i64(
            "changeset.operation_payload_json size",
            operation_payload_json.len(),
        )?,
        operation_payload_json,
        rationale_digest: content_object_digest(rationale_json.as_bytes()),
        rationale_size_bytes: usize_to_i64("changeset.rationale_json size", rationale_json.len())?,
        rationale_json,
        origin_session_id: decode_optional_session_id(
            "changeset.origin_session_id",
            origin_session_id,
        )?,
        created_at_us,
        change_operation_count,
        event_count,
        commits: load_changeset_commits(connection, changeset_id)?,
    })
}

pub(crate) fn changeset_operations(
    connection: &StoreConnection,
    changeset_id: ChangeSetId,
) -> Result<ChangeOperationListResult> {
    let summary = changeset(connection, changeset_id)?;
    let mut statement = connection
        .inner()
        .prepare(
            "SELECT operation_id,
                    ordinal,
                    subject_family,
                    subject_object_id,
                    operation_payload_json
             FROM change_operation
             WHERE changeset_id = ?1
             ORDER BY ordinal, operation_id",
        )
        .map_err(storage_error)?;
    let rows = statement
        .query_map(params![&changeset_id.raw_bytes()[..]], |row| {
            Ok((
                row.get::<_, Vec<u8>>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, Vec<u8>>(3)?,
                row.get::<_, String>(4)?,
            ))
        })
        .map_err(storage_error)?;

    let mut operations = Vec::new();
    for row in rows {
        let (operation_id, ordinal, subject_family, subject_object_id, operation_payload_json) =
            row.map_err(storage_error)?;
        validate_nonnegative("change_operation.ordinal", ordinal)?;
        validate_canonical_json_text(
            "change_operation.operation_payload_json",
            &operation_payload_json,
        )?;
        operations.push(ChangeOperationSnapshot {
            operation_id: decode_operation_id("change_operation.operation_id", operation_id)?,
            ordinal,
            subject: decode_change_operation_subject(
                "change_operation.subject",
                subject_family.as_str(),
                subject_object_id,
            )?,
            operation_payload_digest: content_object_digest(operation_payload_json.as_bytes()),
            operation_payload_size_bytes: usize_to_i64(
                "change_operation.operation_payload_json size",
                operation_payload_json.len(),
            )?,
            operation_payload_json,
        });
    }
    let actual = usize_to_i64("change_operation result count", operations.len())?;
    if actual != summary.change_operation_count {
        return Err(WorkVcsError::QueryInvalid(format!(
            "ChangeSet {changeset_id} expected {} change operations, found {actual}",
            summary.change_operation_count
        )));
    }

    Ok(ChangeOperationListResult {
        workspace_id: summary.workspace_id,
        changeset_id,
        operations,
    })
}

pub(crate) fn event(connection: &StoreConnection, event_id: EventId) -> Result<EventSnapshot> {
    let row = connection
        .inner()
        .query_row(
            "SELECT event_id,
                    workspace_id,
                    changeset_id,
                    session_id,
                    event_kind,
                    occurred_at_us,
                    payload_json
             FROM event
             WHERE event_id = ?1",
            params![&event_id.raw_bytes()[..]],
            raw_event_row,
        )
        .optional()
        .map_err(storage_error)?;

    let Some(row) = row else {
        return Err(WorkVcsError::QueryInvalid(format!(
            "event {event_id} does not exist"
        )));
    };

    decode_event_row(row)
}

pub(crate) fn query_events(
    connection: &StoreConnection,
    options: &EventListOptions,
) -> Result<EventListResult> {
    let (sql, selector_bytes) = match options.target() {
        EventListTarget::ChangeSet(changeset_id) => (
            "SELECT event_id,
                    workspace_id,
                    changeset_id,
                    session_id,
                    event_kind,
                    occurred_at_us,
                    payload_json
             FROM event
             WHERE changeset_id = ?1
             ORDER BY occurred_at_us, event_id
             LIMIT ?2",
            changeset_id.raw_bytes(),
        ),
        EventListTarget::Session(session_id) => (
            "SELECT event_id,
                    workspace_id,
                    changeset_id,
                    session_id,
                    event_kind,
                    occurred_at_us,
                    payload_json
             FROM event
             WHERE session_id = ?1
             ORDER BY occurred_at_us, event_id
             LIMIT ?2",
            session_id.raw_bytes(),
        ),
        EventListTarget::Workspace(workspace_id) => (
            "SELECT event_id,
                    workspace_id,
                    changeset_id,
                    session_id,
                    event_kind,
                    occurred_at_us,
                    payload_json
             FROM event
             WHERE workspace_id = ?1
             ORDER BY occurred_at_us, event_id
             LIMIT ?2",
            workspace_id.raw_bytes(),
        ),
    };
    let limit = match options.limit() {
        Some(limit) => usize_to_i64("event list limit", limit)?,
        None => -1,
    };
    let mut statement = connection.inner().prepare(sql).map_err(storage_error)?;
    let rows = statement
        .query_map(params![&selector_bytes[..], limit], raw_event_row)
        .map_err(storage_error)?;
    let mut events = Vec::new();
    for row in rows {
        events.push(decode_event_row(row.map_err(storage_error)?)?);
    }
    Ok(EventListResult { events })
}

fn load_branch(connection: &StoreConnection, branch_id: BranchId) -> Result<BranchRow> {
    let branch_id_bytes = branch_id.raw_bytes();
    let row = connection
        .inner()
        .query_row(
            "SELECT workspace_id, name, head_commit_id, lifecycle_state
             FROM branch
             WHERE branch_id = ?1",
            params![&branch_id_bytes[..]],
            |row| {
                Ok((
                    row.get::<_, Vec<u8>>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Vec<u8>>(2)?,
                    row.get::<_, String>(3)?,
                ))
            },
        )
        .optional()
        .map_err(storage_error)?;

    let Some((workspace_id, name, head_commit_id, lifecycle_state)) = row else {
        return Err(WorkVcsError::QueryInvalid(format!(
            "branch {branch_id} does not exist"
        )));
    };
    validate_stored_text("branch.name", &name)?;
    validate_stored_text("branch.lifecycle_state", &lifecycle_state)?;

    Ok(BranchRow {
        workspace_id: decode_workspace_id("branch.workspace_id", workspace_id)?,
        name,
        head_commit_id: decode_commit_id("branch.head_commit_id", head_commit_id)?,
        lifecycle_state,
    })
}

struct RawEventRow {
    event_id: Vec<u8>,
    workspace_id: Option<Vec<u8>>,
    changeset_id: Option<Vec<u8>>,
    session_id: Option<Vec<u8>>,
    event_kind: String,
    occurred_at_us: i64,
    payload_json: String,
}

fn raw_event_row(row: &Row<'_>) -> rusqlite::Result<RawEventRow> {
    Ok(RawEventRow {
        event_id: row.get(0)?,
        workspace_id: row.get(1)?,
        changeset_id: row.get(2)?,
        session_id: row.get(3)?,
        event_kind: row.get(4)?,
        occurred_at_us: row.get(5)?,
        payload_json: row.get(6)?,
    })
}

fn decode_event_row(row: RawEventRow) -> Result<EventSnapshot> {
    validate_stored_text("event.event_kind", &row.event_kind)?;
    validate_nonnegative("event.occurred_at_us", row.occurred_at_us)?;
    validate_canonical_json_text("event.payload_json", &row.payload_json)?;
    Ok(EventSnapshot {
        event_id: decode_event_id("event.event_id", row.event_id)?,
        workspace_id: decode_optional_workspace_id("event.workspace_id", row.workspace_id)?,
        changeset_id: decode_optional_changeset_id("event.changeset_id", row.changeset_id)?,
        session_id: decode_optional_session_id("event.session_id", row.session_id)?,
        event_kind: row.event_kind,
        occurred_at_us: row.occurred_at_us,
        payload_digest: content_object_digest(row.payload_json.as_bytes()),
        payload_size_bytes: usize_to_i64("event.payload_json size", row.payload_json.len())?,
        payload_json: row.payload_json,
    })
}

fn load_history_entry(connection: &StoreConnection, commit_id: CommitId) -> Result<HistoryEntry> {
    let commit_id_bytes = commit_id.raw_bytes();
    let row = connection
        .inner()
        .query_row(
            "SELECT workstate_commit.workspace_id,
                    workstate_commit.changeset_id,
                    workstate_commit.commit_kind,
                    workstate_commit.state_digest,
                    workstate_commit.committed_at_us,
                    changeset.operation_type,
                    changeset.operation_schema_version,
                    changeset.created_at_us
             FROM workstate_commit
             JOIN changeset
               ON changeset.workspace_id = workstate_commit.workspace_id
              AND changeset.changeset_id = workstate_commit.changeset_id
             WHERE workstate_commit.commit_id = ?1",
            params![&commit_id_bytes[..]],
            |row| {
                Ok((
                    row.get::<_, Vec<u8>>(0)?,
                    row.get::<_, Vec<u8>>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, Vec<u8>>(3)?,
                    row.get::<_, i64>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, i64>(6)?,
                    row.get::<_, i64>(7)?,
                ))
            },
        )
        .optional()
        .map_err(storage_error)?;

    let Some((
        workspace_id,
        changeset_id,
        commit_kind,
        state_digest,
        committed_at_us,
        operation_type,
        operation_schema_version,
        changeset_created_at_us,
    )) = row
    else {
        return Err(WorkVcsError::QueryInvalid(format!(
            "commit {commit_id} does not exist or lacks its ChangeSet"
        )));
    };
    validate_stored_text("workstate_commit.commit_kind", &commit_kind)?;
    validate_stored_text("changeset.operation_type", &operation_type)?;
    if operation_schema_version <= 0 {
        return Err(WorkVcsError::QueryInvalid(format!(
            "commit {commit_id} ChangeSet has invalid operation schema version {operation_schema_version}"
        )));
    }

    Ok(HistoryEntry {
        workspace_id: decode_workspace_id("workstate_commit.workspace_id", workspace_id)?,
        commit_id,
        changeset_id: decode_changeset_id("workstate_commit.changeset_id", changeset_id)?,
        commit_kind,
        state_digest: decode_digest("workstate_commit.state_digest", state_digest)?,
        committed_at_us,
        operation_type,
        operation_schema_version,
        changeset_created_at_us,
        parent_commit_id: None,
    })
}

fn load_changeset_origin_session(
    connection: &StoreConnection,
    workspace_id: WorkspaceId,
    changeset_id: ChangeSetId,
) -> Result<Option<SessionId>> {
    let row = connection
        .inner()
        .query_row(
            "SELECT origin_session_id
             FROM changeset
             WHERE workspace_id = ?1
               AND changeset_id = ?2",
            params![&workspace_id.raw_bytes()[..], &changeset_id.raw_bytes()[..]],
            |row| row.get::<_, Option<Vec<u8>>>(0),
        )
        .optional()
        .map_err(storage_error)?;
    let Some(origin_session_id) = row else {
        return Err(WorkVcsError::QueryInvalid(format!(
            "commit ChangeSet {changeset_id} does not exist in workspace {workspace_id}"
        )));
    };
    decode_optional_session_id("changeset.origin_session_id", origin_session_id)
}

fn load_changeset_commits(
    connection: &StoreConnection,
    changeset_id: ChangeSetId,
) -> Result<Vec<ChangeSetCommitSnapshot>> {
    let mut statement = connection
        .inner()
        .prepare(
            "SELECT commit_id, commit_kind, state_digest, committed_at_us
             FROM workstate_commit
             WHERE changeset_id = ?1
             ORDER BY committed_at_us, commit_id",
        )
        .map_err(storage_error)?;
    let rows = statement
        .query_map(params![&changeset_id.raw_bytes()[..]], |row| {
            Ok((
                row.get::<_, Vec<u8>>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Vec<u8>>(2)?,
                row.get::<_, i64>(3)?,
            ))
        })
        .map_err(storage_error)?;

    let mut commits = Vec::new();
    for row in rows {
        let (commit_id, commit_kind, state_digest, committed_at_us) = row.map_err(storage_error)?;
        validate_stored_text("workstate_commit.commit_kind", &commit_kind)?;
        validate_nonnegative("workstate_commit.committed_at_us", committed_at_us)?;
        commits.push(ChangeSetCommitSnapshot {
            commit_id: decode_commit_id("workstate_commit.commit_id", commit_id)?,
            commit_kind,
            state_digest: decode_digest("workstate_commit.state_digest", state_digest)?,
            committed_at_us,
        });
    }
    Ok(commits)
}

fn load_commit_parents(
    connection: &StoreConnection,
    commit_id: CommitId,
) -> Result<Vec<CommitParentSnapshot>> {
    let mut statement = connection
        .inner()
        .prepare(
            "SELECT parent_ordinal, parent_role, parent_commit_id
             FROM commit_parent
             WHERE commit_id = ?1
             ORDER BY parent_ordinal, parent_commit_id",
        )
        .map_err(storage_error)?;
    let rows = statement
        .query_map(params![&commit_id.raw_bytes()[..]], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Vec<u8>>(2)?,
            ))
        })
        .map_err(storage_error)?;

    let mut parents = Vec::new();
    for row in rows {
        let (parent_ordinal, parent_role, parent_commit_id) = row.map_err(storage_error)?;
        validate_nonnegative("commit_parent.parent_ordinal", parent_ordinal)?;
        validate_stored_text("commit_parent.parent_role", &parent_role)?;
        parents.push(CommitParentSnapshot {
            parent_ordinal,
            parent_role,
            parent_commit_id: decode_commit_id("commit_parent.parent_commit_id", parent_commit_id)?,
        });
    }
    Ok(parents)
}

fn validate_commit_parent_shape(
    commit_id: CommitId,
    commit_kind: &str,
    parents: &[CommitParentSnapshot],
) -> Result<()> {
    match commit_kind {
        GENESIS_COMMIT_KIND if parents.is_empty() => Ok(()),
        NORMAL_COMMIT_KIND
            if parents.len() == 1 && parent_matches(&parents[0], 0, PRIMARY_PARENT_ROLE) =>
        {
            Ok(())
        }
        MERGE_COMMIT_KIND
            if parents.len() == 2
                && parent_matches(&parents[0], 0, PRIMARY_PARENT_ROLE)
                && parent_matches(&parents[1], 1, SECONDARY_PARENT_ROLE) =>
        {
            Ok(())
        }
        GENESIS_COMMIT_KIND => Err(WorkVcsError::QueryInvalid(format!(
            "genesis commit {commit_id} must not have parents"
        ))),
        NORMAL_COMMIT_KIND => Err(WorkVcsError::QueryInvalid(format!(
            "normal commit {commit_id} must have one ordinal-0 primary parent"
        ))),
        MERGE_COMMIT_KIND => Err(WorkVcsError::QueryInvalid(format!(
            "merge commit {commit_id} must have ordinal-0 primary and ordinal-1 secondary parents"
        ))),
        other => Err(WorkVcsError::QueryInvalid(format!(
            "unsupported WorkStateCommit kind {other:?}"
        ))),
    }
}

fn parent_matches(parent: &CommitParentSnapshot, ordinal: i64, role: &str) -> bool {
    parent.parent_ordinal == ordinal && parent.parent_role == role
}

fn load_first_parent(
    connection: &StoreConnection,
    commit_id: CommitId,
    commit_kind: &str,
) -> Result<Option<CommitId>> {
    match commit_kind {
        GENESIS_COMMIT_KIND => {
            require_count(
                connection,
                "Genesis history parents",
                0,
                "SELECT count(*)
                 FROM commit_parent
                 WHERE commit_id = ?1",
                params![&commit_id.raw_bytes()[..]],
            )?;
            Ok(None)
        }
        NORMAL_COMMIT_KIND => {
            require_count(
                connection,
                "Normal history parents",
                1,
                "SELECT count(*)
                 FROM commit_parent
                 WHERE commit_id = ?1",
                params![&commit_id.raw_bytes()[..]],
            )?;
            Ok(Some(load_primary_parent(
                connection,
                commit_id,
                "normal commit",
            )?))
        }
        MERGE_COMMIT_KIND => {
            require_count(
                connection,
                "Merge history parents",
                2,
                "SELECT count(*)
                 FROM commit_parent
                 WHERE commit_id = ?1",
                params![&commit_id.raw_bytes()[..]],
            )?;
            Ok(Some(load_primary_parent(
                connection,
                commit_id,
                "merge commit",
            )?))
        }
        other => Err(WorkVcsError::QueryInvalid(format!(
            "unsupported WorkStateCommit kind {other:?}"
        ))),
    }
}

fn load_primary_parent(
    connection: &StoreConnection,
    commit_id: CommitId,
    commit_label: &str,
) -> Result<CommitId> {
    let parent = connection
        .inner()
        .query_row(
            "SELECT parent_commit_id
             FROM commit_parent
             WHERE commit_id = ?1
               AND parent_ordinal = 0
               AND parent_role = ?2",
            params![&commit_id.raw_bytes()[..], PRIMARY_PARENT_ROLE],
            |row| row.get::<_, Vec<u8>>(0),
        )
        .optional()
        .map_err(storage_error)?;
    let Some(parent) = parent else {
        return Err(WorkVcsError::QueryInvalid(format!(
            "{commit_label} {commit_id} does not have an ordinal-0 primary parent"
        )));
    };
    decode_commit_id("commit_parent.parent_commit_id", parent)
}

fn validate_stored_text(label: &str, value: &str) -> Result<()> {
    if value.is_empty() {
        return Err(WorkVcsError::QueryInvalid(format!(
            "{label} must not be empty"
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
        Err(WorkVcsError::QueryInvalid(format!(
            "{label} expected count {expected}, found {actual}"
        )))
    }
}

fn count_rows<P: Params>(connection: &StoreConnection, sql: &str, params: P) -> Result<i64> {
    connection
        .inner()
        .query_row(sql, params, |row| row.get::<_, i64>(0))
        .map_err(storage_error)
}

fn decode_workspace_id(column: &str, bytes: Vec<u8>) -> Result<WorkspaceId> {
    let bytes = decode_16(column, bytes)?;
    WorkspaceId::from_bytes(bytes)
        .map_err(|error| WorkVcsError::QueryInvalid(format!("{column} is invalid: {error}")))
}

fn decode_event_id(column: &str, bytes: Vec<u8>) -> Result<EventId> {
    let bytes = decode_16(column, bytes)?;
    EventId::from_bytes(bytes)
        .map_err(|error| WorkVcsError::QueryInvalid(format!("{column} is invalid: {error}")))
}

fn decode_changeset_id(column: &str, bytes: Vec<u8>) -> Result<ChangeSetId> {
    let bytes = decode_16(column, bytes)?;
    ChangeSetId::from_bytes(bytes)
        .map_err(|error| WorkVcsError::QueryInvalid(format!("{column} is invalid: {error}")))
}

fn decode_operation_id(column: &str, bytes: Vec<u8>) -> Result<OperationId> {
    let bytes = decode_16(column, bytes)?;
    OperationId::from_bytes(bytes)
        .map_err(|error| WorkVcsError::QueryInvalid(format!("{column} is invalid: {error}")))
}

fn decode_entity_id(column: &str, bytes: Vec<u8>) -> Result<EntityId> {
    let bytes = decode_16(column, bytes)?;
    EntityId::from_bytes(bytes)
        .map_err(|error| WorkVcsError::QueryInvalid(format!("{column} is invalid: {error}")))
}

fn decode_relation_id(column: &str, bytes: Vec<u8>) -> Result<RelationId> {
    let bytes = decode_16(column, bytes)?;
    RelationId::from_bytes(bytes)
        .map_err(|error| WorkVcsError::QueryInvalid(format!("{column} is invalid: {error}")))
}

fn decode_change_operation_subject(
    column: &str,
    subject_family: &str,
    subject_object_id: Vec<u8>,
) -> Result<ChangeOperationSubject> {
    validate_stored_text("change_operation.subject_family", subject_family)?;
    match subject_family {
        ENTITY_SUBJECT_FAMILY => Ok(ChangeOperationSubject::Entity(decode_entity_id(
            column,
            subject_object_id,
        )?)),
        RELATION_SUBJECT_FAMILY => Ok(ChangeOperationSubject::Relation(decode_relation_id(
            column,
            subject_object_id,
        )?)),
        other => Err(WorkVcsError::QueryInvalid(format!(
            "unsupported change_operation.subject_family {other:?}"
        ))),
    }
}

fn decode_session_id(column: &str, bytes: Vec<u8>) -> Result<SessionId> {
    let bytes = decode_16(column, bytes)?;
    SessionId::from_bytes(bytes)
        .map_err(|error| WorkVcsError::QueryInvalid(format!("{column} is invalid: {error}")))
}

fn decode_commit_id(column: &str, bytes: Vec<u8>) -> Result<CommitId> {
    let bytes = decode_16(column, bytes)?;
    CommitId::from_bytes(bytes)
        .map_err(|error| WorkVcsError::QueryInvalid(format!("{column} is invalid: {error}")))
}

fn decode_16(column: &str, bytes: Vec<u8>) -> Result<[u8; 16]> {
    bytes.try_into().map_err(|bytes: Vec<u8>| {
        WorkVcsError::QueryInvalid(format!("{column} must be 16 bytes, found {}", bytes.len()))
    })
}

fn decode_digest(column: &str, bytes: Vec<u8>) -> Result<Digest> {
    let bytes = bytes.try_into().map_err(|bytes: Vec<u8>| {
        WorkVcsError::QueryInvalid(format!("{column} must be 32 bytes, found {}", bytes.len()))
    })?;
    Ok(Digest::from_bytes(bytes))
}

fn decode_optional_workspace_id(
    column: &str,
    bytes: Option<Vec<u8>>,
) -> Result<Option<WorkspaceId>> {
    bytes
        .map(|bytes| decode_workspace_id(column, bytes))
        .transpose()
}

fn decode_optional_changeset_id(
    column: &str,
    bytes: Option<Vec<u8>>,
) -> Result<Option<ChangeSetId>> {
    bytes
        .map(|bytes| decode_changeset_id(column, bytes))
        .transpose()
}

fn decode_optional_session_id(column: &str, bytes: Option<Vec<u8>>) -> Result<Option<SessionId>> {
    bytes
        .map(|bytes| decode_session_id(column, bytes))
        .transpose()
}

fn validate_nonnegative(label: &str, value: i64) -> Result<()> {
    if value < 0 {
        Err(WorkVcsError::QueryInvalid(format!(
            "{label} must be nonnegative, found {value}"
        )))
    } else {
        Ok(())
    }
}

fn validate_canonical_json_text(label: &str, value: &str) -> Result<()> {
    let parsed = parse_canonical_json(value.as_bytes())
        .map_err(|error| WorkVcsError::QueryInvalid(format!("{label} is invalid: {error}")))?;
    let encoded = canonical_bytes(&parsed)
        .map_err(|error| WorkVcsError::QueryInvalid(format!("{label} is invalid: {error}")))?;
    if encoded == value.as_bytes() {
        Ok(())
    } else {
        Err(WorkVcsError::QueryInvalid(format!(
            "{label} is not canonical fixed-point JSON"
        )))
    }
}

fn usize_to_i64(label: &str, value: usize) -> Result<i64> {
    i64::try_from(value).map_err(|_| {
        WorkVcsError::QueryInvalid(format!("{label} does not fit in signed 64-bit storage"))
    })
}
