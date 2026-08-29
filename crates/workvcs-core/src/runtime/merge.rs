use super::session;
use crate::canonical::{CanonicalValue, canonical_bytes};
use crate::error::{Result, WorkVcsError, storage_error};
use crate::identity::{BranchId, CommitId, EventId, MergeId, SessionId, WorkspaceId};
use crate::store::{StoreConnection, current_epoch_micros};
use rusqlite::{OptionalExtension, Transaction, TransactionBehavior, params};
use std::collections::{BTreeMap, VecDeque};

const ACTIVE_BRANCH_LIFECYCLE_STATE: &str = "active";
const ACTIVE_MERGE_RUNTIME_STATE: &str = "active";
const MERGE_ATTEMPT_OBJECT_KIND: &str = "merge_attempt";
const MERGE_STARTED_EVENT_KIND: &str = "merge.started";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MergeRuntimeState {
    Active,
}

impl MergeRuntimeState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Active => ACTIVE_MERGE_RUNTIME_STATE,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MergeStartOptions {
    target_branch_id: BranchId,
    source_branch_id: BranchId,
    origin_session_id: Option<SessionId>,
}

impl MergeStartOptions {
    pub fn new(target_branch_id: BranchId, source_branch_id: BranchId) -> Self {
        Self {
            target_branch_id,
            source_branch_id,
            origin_session_id: None,
        }
    }

    pub fn with_origin_session_id(mut self, origin_session_id: SessionId) -> Self {
        self.origin_session_id = Some(origin_session_id);
        self
    }

    pub fn target_branch_id(&self) -> BranchId {
        self.target_branch_id
    }

    pub fn source_branch_id(&self) -> BranchId {
        self.source_branch_id
    }

    pub fn origin_session_id(&self) -> Option<SessionId> {
        self.origin_session_id
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MergeStartResult {
    pub merge_id: MergeId,
    pub workspace_id: WorkspaceId,
    pub target_branch_id: BranchId,
    pub source_branch_id: BranchId,
    pub merge_base_commit_id: CommitId,
    pub target_head_commit_id: CommitId,
    pub source_head_commit_id: CommitId,
    pub origin_session_id: Option<SessionId>,
    pub runtime_state: MergeRuntimeState,
    pub event_id: EventId,
    pub created_at_us: i64,
}

pub(crate) fn start_merge(
    connection: &mut StoreConnection,
    options: &MergeStartOptions,
) -> Result<MergeStartResult> {
    connection.verify_foreign_keys()?;
    if options.target_branch_id() == options.source_branch_id() {
        return Err(WorkVcsError::WorkspaceInvalid(
            "merge target and source branches must be distinct".to_owned(),
        ));
    }

    let merge_id = MergeId::new_v7();
    let event_id = EventId::new_v7();
    let created_at_us = current_epoch_micros()?;
    let runtime_json = active_runtime_json()?;

    let transaction = connection
        .inner_mut()
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(storage_error)?;
    let target = load_active_branch(&transaction, options.target_branch_id())?;
    let source = load_active_branch(&transaction, options.source_branch_id())?;
    if target.workspace_id != source.workspace_id {
        return Err(WorkVcsError::WorkspaceInvalid(format!(
            "merge target branch {} belongs to workspace {}, but source branch {} belongs to workspace {}",
            options.target_branch_id(),
            target.workspace_id,
            options.source_branch_id(),
            source.workspace_id
        )));
    }
    if let Some(origin_session_id) = options.origin_session_id() {
        let session =
            session::load_active_session_runtime_for_update(&transaction, origin_session_id)?;
        if session.active_workspace_id != target.workspace_id {
            return Err(WorkVcsError::SessionInvalid(format!(
                "merge origin session {origin_session_id} belongs to workspace {}, not {}",
                session.active_workspace_id, target.workspace_id
            )));
        }
        session::update_session_activity(&transaction, origin_session_id, created_at_us)?;
    }
    ensure_no_active_merge_for_target(&transaction, options.target_branch_id())?;

    let merge_base_commit_id = find_merge_base(
        &transaction,
        target.workspace_id,
        target.head_commit_id,
        source.head_commit_id,
    )?;
    let merge_id_bytes = merge_id.raw_bytes();
    let workspace_id_bytes = target.workspace_id.raw_bytes();
    let target_branch_id_bytes = options.target_branch_id().raw_bytes();
    let source_branch_id_bytes = options.source_branch_id().raw_bytes();
    let merge_base_commit_id_bytes = merge_base_commit_id.raw_bytes();
    let target_head_commit_id_bytes = target.head_commit_id.raw_bytes();
    let source_head_commit_id_bytes = source.head_commit_id.raw_bytes();
    let origin_session_id_bytes = options.origin_session_id().map(|id| id.raw_bytes());
    let event_id_bytes = event_id.raw_bytes();
    let event_payload_json = merge_started_payload_json(&MergeStartedPayload {
        merge_id,
        workspace_id: target.workspace_id,
        target_branch_id: options.target_branch_id(),
        source_branch_id: options.source_branch_id(),
        merge_base_commit_id,
        target_head_commit_id: target.head_commit_id,
        source_head_commit_id: source.head_commit_id,
        origin_session_id: options.origin_session_id(),
    })?;

    transaction
        .execute(
            "INSERT INTO object_identity(object_id, object_kind, created_at_us)
             VALUES (?1, ?2, ?3)",
            params![
                &merge_id_bytes[..],
                MERGE_ATTEMPT_OBJECT_KIND,
                created_at_us
            ],
        )
        .map_err(storage_error)?;
    transaction
        .execute(
            "INSERT INTO merge_attempt(
                merge_id,
                workspace_id,
                target_branch_id,
                source_branch_id,
                merge_base_commit_id,
                target_head_commit_id,
                source_head_commit_id,
                origin_session_id,
                created_at_us
             )
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                &merge_id_bytes[..],
                &workspace_id_bytes[..],
                &target_branch_id_bytes[..],
                &source_branch_id_bytes[..],
                &merge_base_commit_id_bytes[..],
                &target_head_commit_id_bytes[..],
                &source_head_commit_id_bytes[..],
                origin_session_id_bytes.as_ref().map(|bytes| &bytes[..]),
                created_at_us
            ],
        )
        .map_err(storage_error)?;
    transaction
        .execute(
            "INSERT INTO merge_runtime(merge_id, runtime_json)
             VALUES (?1, ?2)",
            params![&merge_id_bytes[..], runtime_json],
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
                origin_session_id_bytes.as_ref().map(|bytes| &bytes[..]),
                MERGE_STARTED_EVENT_KIND,
                created_at_us,
                event_payload_json
            ],
        )
        .map_err(storage_error)?;
    transaction.commit().map_err(storage_error)?;

    Ok(MergeStartResult {
        merge_id,
        workspace_id: target.workspace_id,
        target_branch_id: options.target_branch_id(),
        source_branch_id: options.source_branch_id(),
        merge_base_commit_id,
        target_head_commit_id: target.head_commit_id,
        source_head_commit_id: source.head_commit_id,
        origin_session_id: options.origin_session_id(),
        runtime_state: MergeRuntimeState::Active,
        event_id,
        created_at_us,
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct BranchRow {
    workspace_id: WorkspaceId,
    head_commit_id: CommitId,
}

fn load_active_branch(transaction: &Transaction<'_>, branch_id: BranchId) -> Result<BranchRow> {
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
        return Err(WorkVcsError::WorkspaceInvalid(format!(
            "branch {branch_id} has lifecycle state {lifecycle_state:?}"
        )));
    }
    Ok(BranchRow {
        workspace_id: decode_workspace_id("branch.workspace_id", workspace_id)?,
        head_commit_id: decode_commit_id("branch.head_commit_id", head_commit_id)?,
    })
}

fn ensure_no_active_merge_for_target(
    transaction: &Transaction<'_>,
    target_branch_id: BranchId,
) -> Result<()> {
    let row = transaction
        .query_row(
            "SELECT merge_attempt.merge_id
             FROM merge_attempt
             JOIN merge_runtime
               ON merge_runtime.merge_id = merge_attempt.merge_id
             LEFT JOIN merge_attempt_outcome
               ON merge_attempt_outcome.merge_id = merge_attempt.merge_id
             WHERE merge_attempt.target_branch_id = ?1
               AND merge_attempt_outcome.merge_id IS NULL
             LIMIT 1",
            params![&target_branch_id.raw_bytes()[..]],
            |row| row.get::<_, Vec<u8>>(0),
        )
        .optional()
        .map_err(storage_error)?;
    if let Some(bytes) = row {
        let merge_id = decode_merge_id("merge_attempt.merge_id", bytes)?;
        return Err(WorkVcsError::WorkspaceInvalid(format!(
            "target branch {target_branch_id} already has active merge {merge_id}"
        )));
    }
    Ok(())
}

fn find_merge_base(
    transaction: &Transaction<'_>,
    workspace_id: WorkspaceId,
    target_head_commit_id: CommitId,
    source_head_commit_id: CommitId,
) -> Result<CommitId> {
    let target_ancestors = ancestor_depths(transaction, workspace_id, target_head_commit_id)?;
    let source_ancestors = ancestor_depths(transaction, workspace_id, source_head_commit_id)?;
    target_ancestors
        .iter()
        .filter_map(|(commit_id, target_depth)| {
            source_ancestors.get(commit_id).map(|source_depth| {
                (
                    target_depth + source_depth,
                    *target_depth,
                    *source_depth,
                    *commit_id,
                )
            })
        })
        .min()
        .map(|(_, _, _, commit_id)| commit_id)
        .ok_or_else(|| {
            WorkVcsError::WorkspaceInvalid(format!(
                "branches {target_head_commit_id} and {source_head_commit_id} have no common merge base"
            ))
        })
}

fn ancestor_depths(
    transaction: &Transaction<'_>,
    workspace_id: WorkspaceId,
    start_commit_id: CommitId,
) -> Result<BTreeMap<CommitId, usize>> {
    let mut ancestors = BTreeMap::new();
    let mut queue = VecDeque::from([(start_commit_id, 0usize)]);
    while let Some((commit_id, depth)) = queue.pop_front() {
        if ancestors.contains_key(&commit_id) {
            continue;
        }
        ensure_commit_belongs_to_workspace(transaction, workspace_id, commit_id)?;
        ancestors.insert(commit_id, depth);
        for parent_commit_id in parent_commit_ids(transaction, commit_id)? {
            queue.push_back((parent_commit_id, depth + 1));
        }
    }
    Ok(ancestors)
}

fn ensure_commit_belongs_to_workspace(
    transaction: &Transaction<'_>,
    workspace_id: WorkspaceId,
    commit_id: CommitId,
) -> Result<()> {
    let row = transaction
        .query_row(
            "SELECT workspace_id
             FROM workstate_commit
             WHERE commit_id = ?1",
            params![&commit_id.raw_bytes()[..]],
            |row| row.get::<_, Vec<u8>>(0),
        )
        .optional()
        .map_err(storage_error)?;
    let Some(bytes) = row else {
        return Err(WorkVcsError::CommitNotFound(format!(
            "commit {commit_id} does not exist"
        )));
    };
    let commit_workspace_id = decode_workspace_id("workstate_commit.workspace_id", bytes)?;
    if commit_workspace_id != workspace_id {
        return Err(WorkVcsError::WorkspaceInvalid(format!(
            "commit {commit_id} belongs to workspace {commit_workspace_id}, not {workspace_id}"
        )));
    }
    Ok(())
}

fn parent_commit_ids(transaction: &Transaction<'_>, commit_id: CommitId) -> Result<Vec<CommitId>> {
    let mut statement = transaction
        .prepare(
            "SELECT parent_commit_id
             FROM commit_parent
             WHERE commit_id = ?1
             ORDER BY parent_ordinal",
        )
        .map_err(storage_error)?;
    let rows = statement
        .query_map(params![&commit_id.raw_bytes()[..]], |row| {
            row.get::<_, Vec<u8>>(0)
        })
        .map_err(storage_error)?;
    let mut parents = Vec::new();
    for row in rows {
        parents.push(decode_commit_id(
            "commit_parent.parent_commit_id",
            row.map_err(storage_error)?,
        )?);
    }
    Ok(parents)
}

fn active_runtime_json() -> Result<String> {
    canonical_json_string(&CanonicalValue::object(vec![(
        "lifecycle_state".to_owned(),
        CanonicalValue::String(ACTIVE_MERGE_RUNTIME_STATE.to_owned()),
    )])?)
}

struct MergeStartedPayload {
    merge_id: MergeId,
    workspace_id: WorkspaceId,
    target_branch_id: BranchId,
    source_branch_id: BranchId,
    merge_base_commit_id: CommitId,
    target_head_commit_id: CommitId,
    source_head_commit_id: CommitId,
    origin_session_id: Option<SessionId>,
}

fn merge_started_payload_json(payload: &MergeStartedPayload) -> Result<String> {
    let origin_session_id = match payload.origin_session_id {
        Some(origin_session_id) => CanonicalValue::String(origin_session_id.to_string()),
        None => CanonicalValue::Null,
    };
    canonical_json_string(&CanonicalValue::object(vec![
        (
            "lifecycle_state".to_owned(),
            CanonicalValue::String(ACTIVE_MERGE_RUNTIME_STATE.to_owned()),
        ),
        (
            "merge_base_commit_id".to_owned(),
            CanonicalValue::String(payload.merge_base_commit_id.to_string()),
        ),
        (
            "merge_id".to_owned(),
            CanonicalValue::String(payload.merge_id.to_string()),
        ),
        ("origin_session_id".to_owned(), origin_session_id),
        (
            "source_branch_id".to_owned(),
            CanonicalValue::String(payload.source_branch_id.to_string()),
        ),
        (
            "source_head_commit_id".to_owned(),
            CanonicalValue::String(payload.source_head_commit_id.to_string()),
        ),
        (
            "target_branch_id".to_owned(),
            CanonicalValue::String(payload.target_branch_id.to_string()),
        ),
        (
            "target_head_commit_id".to_owned(),
            CanonicalValue::String(payload.target_head_commit_id.to_string()),
        ),
        (
            "workspace_id".to_owned(),
            CanonicalValue::String(payload.workspace_id.to_string()),
        ),
    ])?)
}

fn canonical_json_string(value: &CanonicalValue) -> Result<String> {
    String::from_utf8(canonical_bytes(value)?).map_err(|error| {
        WorkVcsError::CanonicalEncodingInvalid(format!("canonical JSON was not UTF-8: {error}"))
    })
}

fn validate_stored_text(column: &str, value: &str) -> Result<()> {
    if value.is_empty() {
        return Err(WorkVcsError::WorkspaceInvalid(format!(
            "{column} must not be empty"
        )));
    }
    if value
        .as_bytes()
        .iter()
        .any(|byte| *byte == b'\0' || *byte < 0x20 || *byte == 0x7f)
    {
        return Err(WorkVcsError::WorkspaceInvalid(format!(
            "{column} must not contain NUL or ASCII control characters"
        )));
    }
    Ok(())
}

fn decode_workspace_id(column: &str, bytes: Vec<u8>) -> Result<WorkspaceId> {
    WorkspaceId::from_bytes(decode_uuid_bytes(column, bytes)?).map_err(|error| {
        WorkVcsError::WorkspaceInvalid(format!("{column} is not a valid WorkspaceId: {error}"))
    })
}

fn decode_commit_id(column: &str, bytes: Vec<u8>) -> Result<CommitId> {
    CommitId::from_bytes(decode_uuid_bytes(column, bytes)?).map_err(|error| {
        WorkVcsError::WorkspaceInvalid(format!("{column} is not a valid CommitId: {error}"))
    })
}

fn decode_merge_id(column: &str, bytes: Vec<u8>) -> Result<MergeId> {
    MergeId::from_bytes(decode_uuid_bytes(column, bytes)?).map_err(|error| {
        WorkVcsError::WorkspaceInvalid(format!("{column} is not a valid MergeId: {error}"))
    })
}

fn decode_uuid_bytes(column: &str, bytes: Vec<u8>) -> Result<[u8; 16]> {
    bytes.try_into().map_err(|bytes: Vec<u8>| {
        WorkVcsError::WorkspaceInvalid(format!(
            "{column} must contain 16 bytes, found {}",
            bytes.len()
        ))
    })
}
