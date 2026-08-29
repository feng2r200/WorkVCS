use super::session;
use crate::canonical::{CanonicalValue, canonical_bytes, parse_canonical_json};
use crate::error::{Result, WorkVcsError, storage_error};
use crate::history;
use crate::identity::{
    BranchId, CommitId, EntityId, EntityVersionId, EventId, MergeId, MergeItemId, RelationId,
    RelationVersionId, SessionId, WorkspaceId,
};
use crate::store::{StoreConnection, current_epoch_micros};
use rusqlite::{Connection, OptionalExtension, Transaction, TransactionBehavior, params};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

const ACTIVE_BRANCH_LIFECYCLE_STATE: &str = "active";
const ACTIVE_MERGE_RUNTIME_STATE: &str = "active";
const ABORTED_MERGE_RUNTIME_STATE: &str = "aborted";
const COMPLETED_MERGE_RUNTIME_STATE: &str = "completed";
const ABORTED_MERGE_OUTCOME: &str = "aborted";
const COMPLETED_MERGE_OUTCOME: &str = "completed";
const AUTO_MERGE_ITEM_CLASSIFICATION: &str = "AUTO";
const CONFLICT_MERGE_ITEM_CLASSIFICATION: &str = "CONFLICT";
const REVIEW_MERGE_ITEM_CLASSIFICATION: &str = "REVIEW";
const ENTITY_MERGE_ITEM_SUBJECT_KIND: &str = "entity";
const RELATION_MERGE_ITEM_SUBJECT_KIND: &str = "relation";
const MERGE_ATTEMPT_OBJECT_KIND: &str = "merge_attempt";
const MERGE_STARTED_EVENT_KIND: &str = "merge.started";
const MERGE_ABORTED_EVENT_KIND: &str = "merge.aborted";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MergeRuntimeState {
    Active,
    Aborted,
    Completed,
}

impl MergeRuntimeState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Active => ACTIVE_MERGE_RUNTIME_STATE,
            Self::Aborted => ABORTED_MERGE_RUNTIME_STATE,
            Self::Completed => COMPLETED_MERGE_RUNTIME_STATE,
        }
    }

    fn from_str(value: &str) -> Option<Self> {
        match value {
            ACTIVE_MERGE_RUNTIME_STATE => Some(Self::Active),
            ABORTED_MERGE_RUNTIME_STATE => Some(Self::Aborted),
            COMPLETED_MERGE_RUNTIME_STATE => Some(Self::Completed),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MergeOutcome {
    Aborted,
    Completed,
}

impl MergeOutcome {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Aborted => ABORTED_MERGE_OUTCOME,
            Self::Completed => COMPLETED_MERGE_OUTCOME,
        }
    }

    fn from_str(value: &str) -> Option<Self> {
        match value {
            ABORTED_MERGE_OUTCOME => Some(Self::Aborted),
            COMPLETED_MERGE_OUTCOME => Some(Self::Completed),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MergeItemClassification {
    Auto,
    Conflict,
    Review,
}

impl MergeItemClassification {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Auto => AUTO_MERGE_ITEM_CLASSIFICATION,
            Self::Conflict => CONFLICT_MERGE_ITEM_CLASSIFICATION,
            Self::Review => REVIEW_MERGE_ITEM_CLASSIFICATION,
        }
    }

    fn from_str(value: &str) -> Option<Self> {
        match value {
            AUTO_MERGE_ITEM_CLASSIFICATION => Some(Self::Auto),
            CONFLICT_MERGE_ITEM_CLASSIFICATION => Some(Self::Conflict),
            REVIEW_MERGE_ITEM_CLASSIFICATION => Some(Self::Review),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MergeItemSubject {
    Entity(EntityId),
    Relation(RelationId),
}

impl MergeItemSubject {
    pub fn kind(self) -> &'static str {
        match self {
            Self::Entity(_) => ENTITY_MERGE_ITEM_SUBJECT_KIND,
            Self::Relation(_) => RELATION_MERGE_ITEM_SUBJECT_KIND,
        }
    }

    pub fn id_string(self) -> String {
        match self {
            Self::Entity(entity_id) => entity_id.to_string(),
            Self::Relation(relation_id) => relation_id.to_string(),
        }
    }

    fn object_id_bytes(self) -> [u8; 16] {
        match self {
            Self::Entity(entity_id) => entity_id.raw_bytes(),
            Self::Relation(relation_id) => relation_id.raw_bytes(),
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MergeAbortOptions {
    merge_id: MergeId,
    abort_session_id: Option<SessionId>,
    detail: CanonicalValue,
}

impl MergeAbortOptions {
    pub fn new(merge_id: MergeId) -> Result<Self> {
        Ok(Self {
            merge_id,
            abort_session_id: None,
            detail: CanonicalValue::object(Vec::new())?,
        })
    }

    pub fn with_abort_session_id(mut self, abort_session_id: SessionId) -> Self {
        self.abort_session_id = Some(abort_session_id);
        self
    }

    pub fn with_detail(mut self, detail: CanonicalValue) -> Result<Self> {
        require_object_value("merge abort detail", &detail)?;
        self.detail = detail;
        Ok(self)
    }

    pub fn merge_id(&self) -> MergeId {
        self.merge_id
    }

    pub fn abort_session_id(&self) -> Option<SessionId> {
        self.abort_session_id
    }

    pub fn detail(&self) -> &CanonicalValue {
        &self.detail
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MergeAbortResult {
    pub merge_id: MergeId,
    pub workspace_id: WorkspaceId,
    pub target_branch_id: BranchId,
    pub source_branch_id: BranchId,
    pub merge_base_commit_id: CommitId,
    pub target_head_commit_id: CommitId,
    pub source_head_commit_id: CommitId,
    pub origin_session_id: Option<SessionId>,
    pub abort_session_id: Option<SessionId>,
    pub runtime_state: MergeRuntimeState,
    pub event_id: EventId,
    pub aborted_at_us: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MergeOutcomeSnapshot {
    pub outcome: MergeOutcome,
    pub result_commit_id: Option<CommitId>,
    pub completed_at_us: i64,
    pub detail: CanonicalValue,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MergeItemSnapshot {
    pub merge_item_id: MergeItemId,
    pub ordinal: i64,
    pub classification: MergeItemClassification,
    pub subject: Option<MergeItemSubject>,
    pub payload: CanonicalValue,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MergeAttemptSnapshot {
    pub merge_id: MergeId,
    pub workspace_id: WorkspaceId,
    pub target_branch_id: BranchId,
    pub source_branch_id: BranchId,
    pub merge_base_commit_id: CommitId,
    pub target_head_commit_id: CommitId,
    pub source_head_commit_id: CommitId,
    pub origin_session_id: Option<SessionId>,
    pub created_at_us: i64,
    pub runtime_state: MergeRuntimeState,
    pub items: Vec<MergeItemSnapshot>,
    pub outcome: Option<MergeOutcomeSnapshot>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MergeListOptions {
    workspace_id: WorkspaceId,
    target_branch_id: Option<BranchId>,
    include_closed: bool,
}

impl MergeListOptions {
    pub fn new(workspace_id: WorkspaceId) -> Self {
        Self {
            workspace_id,
            target_branch_id: None,
            include_closed: false,
        }
    }

    pub fn with_target_branch(mut self, target_branch_id: BranchId) -> Self {
        self.target_branch_id = Some(target_branch_id);
        self
    }

    pub fn include_closed(mut self) -> Self {
        self.include_closed = true;
        self
    }

    pub fn workspace_id(&self) -> WorkspaceId {
        self.workspace_id
    }

    pub fn target_branch_id(&self) -> Option<BranchId> {
        self.target_branch_id
    }

    pub fn includes_closed(&self) -> bool {
        self.include_closed
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MergeListResult {
    pub workspace_id: WorkspaceId,
    pub target_branch_id: Option<BranchId>,
    pub include_closed: bool,
    pub merges: Vec<MergeAttemptSnapshot>,
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

    let captured_target = history::branch_head(connection, options.target_branch_id())?;
    let captured_source = history::branch_head(connection, options.source_branch_id())?;
    if captured_target.workspace_id != captured_source.workspace_id {
        return Err(WorkVcsError::WorkspaceInvalid(format!(
            "merge target branch {} belongs to workspace {}, but source branch {} belongs to workspace {}",
            options.target_branch_id(),
            captured_target.workspace_id,
            options.source_branch_id(),
            captured_source.workspace_id
        )));
    }
    let merge_base_commit_id = find_merge_base(
        connection.inner(),
        captured_target.workspace_id,
        captured_target.head_commit_id,
        captured_source.head_commit_id,
    )?;
    let merge_items = classify_merge_items(
        connection,
        captured_target.workspace_id,
        merge_base_commit_id,
        captured_target.head_commit_id,
        captured_source.head_commit_id,
    )?;

    let merge_id = MergeId::new_v7();
    let event_id = EventId::new_v7();
    let created_at_us = current_epoch_micros()?;
    let runtime_json = runtime_json(MergeRuntimeState::Active)?;

    let transaction = connection
        .inner_mut()
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(storage_error)?;
    let target = load_active_branch(&transaction, options.target_branch_id())?;
    let source = load_active_branch(&transaction, options.source_branch_id())?;
    if target.workspace_id != captured_target.workspace_id
        || target.head_commit_id != captured_target.head_commit_id
    {
        return Err(WorkVcsError::BranchHeadConflict(format!(
            "merge target branch {} moved from {} to {}",
            options.target_branch_id(),
            captured_target.head_commit_id,
            target.head_commit_id
        )));
    }
    if source.workspace_id != captured_source.workspace_id
        || source.head_commit_id != captured_source.head_commit_id
    {
        return Err(WorkVcsError::BranchHeadConflict(format!(
            "merge source branch {} moved from {} to {}",
            options.source_branch_id(),
            captured_source.head_commit_id,
            source.head_commit_id
        )));
    }
    if let Some(origin_session_id) = options.origin_session_id() {
        let session =
            session::load_active_session_runtime_for_update(&transaction, origin_session_id)?;
        if session.active_workspace_id != captured_target.workspace_id {
            return Err(WorkVcsError::SessionInvalid(format!(
                "merge origin session {origin_session_id} belongs to workspace {}, not {}",
                session.active_workspace_id, captured_target.workspace_id
            )));
        }
        session::update_session_activity(&transaction, origin_session_id, created_at_us)?;
    }
    ensure_no_active_merge_for_target(&transaction, options.target_branch_id())?;

    let merge_id_bytes = merge_id.raw_bytes();
    let workspace_id_bytes = captured_target.workspace_id.raw_bytes();
    let target_branch_id_bytes = options.target_branch_id().raw_bytes();
    let source_branch_id_bytes = options.source_branch_id().raw_bytes();
    let merge_base_commit_id_bytes = merge_base_commit_id.raw_bytes();
    let target_head_commit_id_bytes = captured_target.head_commit_id.raw_bytes();
    let source_head_commit_id_bytes = captured_source.head_commit_id.raw_bytes();
    let origin_session_id_bytes = options.origin_session_id().map(|id| id.raw_bytes());
    let event_id_bytes = event_id.raw_bytes();
    let event_payload_json = merge_started_payload_json(&MergeStartedPayload {
        merge_id,
        workspace_id: captured_target.workspace_id,
        target_branch_id: options.target_branch_id(),
        source_branch_id: options.source_branch_id(),
        merge_base_commit_id,
        target_head_commit_id: captured_target.head_commit_id,
        source_head_commit_id: captured_source.head_commit_id,
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
    insert_merge_items(&transaction, merge_id, &merge_items)?;
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
        workspace_id: captured_target.workspace_id,
        target_branch_id: options.target_branch_id(),
        source_branch_id: options.source_branch_id(),
        merge_base_commit_id,
        target_head_commit_id: captured_target.head_commit_id,
        source_head_commit_id: captured_source.head_commit_id,
        origin_session_id: options.origin_session_id(),
        runtime_state: MergeRuntimeState::Active,
        event_id,
        created_at_us,
    })
}

pub(crate) fn abort_merge(
    connection: &mut StoreConnection,
    options: &MergeAbortOptions,
) -> Result<MergeAbortResult> {
    connection.verify_foreign_keys()?;

    let event_id = EventId::new_v7();
    let aborted_at_us = current_epoch_micros()?;
    let aborted_runtime_json = runtime_json(MergeRuntimeState::Aborted)?;
    let detail_json = canonical_json_string(options.detail())?;

    let transaction = connection
        .inner_mut()
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(storage_error)?;
    let merge = load_active_merge_for_update(&transaction, options.merge_id())?;
    if let Some(abort_session_id) = options.abort_session_id() {
        let session =
            session::load_active_session_runtime_for_update(&transaction, abort_session_id)?;
        if session.active_workspace_id != merge.workspace_id {
            return Err(WorkVcsError::SessionInvalid(format!(
                "merge abort session {abort_session_id} belongs to workspace {}, not {}",
                session.active_workspace_id, merge.workspace_id
            )));
        }
        session::update_session_activity(&transaction, abort_session_id, aborted_at_us)?;
    }

    let merge_id_bytes = merge.merge_id.raw_bytes();
    let workspace_id_bytes = merge.workspace_id.raw_bytes();
    let abort_session_id_bytes = options.abort_session_id().map(|id| id.raw_bytes());
    let event_id_bytes = event_id.raw_bytes();
    let event_payload_json = merge_aborted_payload_json(&MergeAbortedPayload {
        merge,
        abort_session_id: options.abort_session_id(),
        detail: options.detail().clone(),
    })?;

    transaction
        .execute(
            "DELETE FROM merge_resolution_runtime
             WHERE merge_item_id IN (
                 SELECT merge_item_id
                 FROM merge_item
                 WHERE merge_id = ?1
             )",
            params![&merge_id_bytes[..]],
        )
        .map_err(storage_error)?;
    transaction
        .execute(
            "UPDATE merge_runtime
             SET runtime_json = ?2
             WHERE merge_id = ?1",
            params![&merge_id_bytes[..], aborted_runtime_json],
        )
        .map_err(storage_error)?;
    transaction
        .execute(
            "INSERT INTO merge_attempt_outcome(
                merge_id,
                outcome,
                result_commit_id,
                completed_at_us,
                detail_json
             )
             VALUES (?1, ?2, NULL, ?3, ?4)",
            params![
                &merge_id_bytes[..],
                ABORTED_MERGE_OUTCOME,
                aborted_at_us,
                detail_json
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
             VALUES (?1, ?2, NULL, ?3, ?4, ?5, ?6)",
            params![
                &event_id_bytes[..],
                &workspace_id_bytes[..],
                abort_session_id_bytes.as_ref().map(|bytes| &bytes[..]),
                MERGE_ABORTED_EVENT_KIND,
                aborted_at_us,
                event_payload_json
            ],
        )
        .map_err(storage_error)?;
    transaction.commit().map_err(storage_error)?;

    Ok(MergeAbortResult {
        merge_id: merge.merge_id,
        workspace_id: merge.workspace_id,
        target_branch_id: merge.target_branch_id,
        source_branch_id: merge.source_branch_id,
        merge_base_commit_id: merge.merge_base_commit_id,
        target_head_commit_id: merge.target_head_commit_id,
        source_head_commit_id: merge.source_head_commit_id,
        origin_session_id: merge.origin_session_id,
        abort_session_id: options.abort_session_id(),
        runtime_state: MergeRuntimeState::Aborted,
        event_id,
        aborted_at_us,
    })
}

pub(crate) fn merge_attempt(
    connection: &StoreConnection,
    merge_id: MergeId,
) -> Result<MergeAttemptSnapshot> {
    connection.verify_foreign_keys()?;
    load_merge_attempt_snapshot(connection.inner(), merge_id)
}

pub(crate) fn merge_attempts(
    connection: &StoreConnection,
    options: &MergeListOptions,
) -> Result<MergeListResult> {
    connection.verify_foreign_keys()?;
    let ids = merge_attempt_ids(connection.inner(), options)?;
    let mut merges = Vec::with_capacity(ids.len());
    for merge_id in ids {
        merges.push(load_merge_attempt_snapshot(connection.inner(), merge_id)?);
    }
    Ok(MergeListResult {
        workspace_id: options.workspace_id(),
        target_branch_id: options.target_branch_id(),
        include_closed: options.includes_closed(),
        merges,
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct BranchRow {
    workspace_id: WorkspaceId,
    head_commit_id: CommitId,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct MergeAttemptRow {
    merge_id: MergeId,
    workspace_id: WorkspaceId,
    target_branch_id: BranchId,
    source_branch_id: BranchId,
    merge_base_commit_id: CommitId,
    target_head_commit_id: CommitId,
    source_head_commit_id: CommitId,
    origin_session_id: Option<SessionId>,
    created_at_us: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct PreparedMergeItem {
    merge_item_id: MergeItemId,
    ordinal: i64,
    classification: MergeItemClassification,
    subject: MergeItemSubject,
    payload_json: String,
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

fn load_active_merge_for_update(
    transaction: &Transaction<'_>,
    merge_id: MergeId,
) -> Result<MergeAttemptRow> {
    let row = transaction
        .query_row(
            "SELECT merge_attempt.workspace_id,
                    merge_attempt.target_branch_id,
                    merge_attempt.source_branch_id,
                    merge_attempt.merge_base_commit_id,
                    merge_attempt.target_head_commit_id,
                    merge_attempt.source_head_commit_id,
                    merge_attempt.origin_session_id,
                    merge_attempt.created_at_us,
                    merge_runtime.runtime_json,
                    merge_attempt_outcome.outcome
             FROM merge_attempt
             JOIN merge_runtime
               ON merge_runtime.merge_id = merge_attempt.merge_id
             LEFT JOIN merge_attempt_outcome
               ON merge_attempt_outcome.merge_id = merge_attempt.merge_id
             WHERE merge_attempt.merge_id = ?1",
            params![&merge_id.raw_bytes()[..]],
            |row| {
                Ok((
                    row.get::<_, Vec<u8>>(0)?,
                    row.get::<_, Vec<u8>>(1)?,
                    row.get::<_, Vec<u8>>(2)?,
                    row.get::<_, Vec<u8>>(3)?,
                    row.get::<_, Vec<u8>>(4)?,
                    row.get::<_, Vec<u8>>(5)?,
                    row.get::<_, Option<Vec<u8>>>(6)?,
                    row.get::<_, i64>(7)?,
                    row.get::<_, String>(8)?,
                    row.get::<_, Option<String>>(9)?,
                ))
            },
        )
        .optional()
        .map_err(storage_error)?;

    let Some((
        workspace_id,
        target_branch_id,
        source_branch_id,
        merge_base_commit_id,
        target_head_commit_id,
        source_head_commit_id,
        origin_session_id,
        created_at_us,
        runtime_json,
        outcome,
    )) = row
    else {
        return Err(WorkVcsError::WorkspaceInvalid(format!(
            "merge {merge_id} does not exist"
        )));
    };
    validate_runtime_json(
        merge_id,
        &runtime_json,
        MergeRuntimeState::Active,
        "active merge",
    )?;
    if let Some(outcome) = outcome {
        validate_stored_text("merge_attempt_outcome.outcome", &outcome)?;
        return Err(WorkVcsError::WorkspaceInvalid(format!(
            "merge {merge_id} already has outcome {outcome:?}"
        )));
    }
    Ok(MergeAttemptRow {
        merge_id,
        workspace_id: decode_workspace_id("merge_attempt.workspace_id", workspace_id)?,
        target_branch_id: decode_branch_id("merge_attempt.target_branch_id", target_branch_id)?,
        source_branch_id: decode_branch_id("merge_attempt.source_branch_id", source_branch_id)?,
        merge_base_commit_id: decode_commit_id(
            "merge_attempt.merge_base_commit_id",
            merge_base_commit_id,
        )?,
        target_head_commit_id: decode_commit_id(
            "merge_attempt.target_head_commit_id",
            target_head_commit_id,
        )?,
        source_head_commit_id: decode_commit_id(
            "merge_attempt.source_head_commit_id",
            source_head_commit_id,
        )?,
        origin_session_id: origin_session_id
            .map(|bytes| decode_session_id("merge_attempt.origin_session_id", bytes))
            .transpose()?,
        created_at_us,
    })
}

fn merge_attempt_ids(connection: &Connection, options: &MergeListOptions) -> Result<Vec<MergeId>> {
    let workspace_id = options.workspace_id().raw_bytes();
    let mut ids = Vec::new();
    if let Some(target_branch_id) = options.target_branch_id() {
        let target_branch_id = target_branch_id.raw_bytes();
        let sql = if options.includes_closed() {
            "SELECT merge_attempt.merge_id
             FROM merge_attempt
             WHERE merge_attempt.workspace_id = ?1
               AND merge_attempt.target_branch_id = ?2
             ORDER BY merge_attempt.created_at_us, merge_attempt.merge_id"
        } else {
            "SELECT merge_attempt.merge_id
             FROM merge_attempt
             LEFT JOIN merge_attempt_outcome
               ON merge_attempt_outcome.merge_id = merge_attempt.merge_id
             WHERE merge_attempt.workspace_id = ?1
               AND merge_attempt.target_branch_id = ?2
               AND merge_attempt_outcome.merge_id IS NULL
             ORDER BY merge_attempt.created_at_us, merge_attempt.merge_id"
        };
        let mut statement = connection.prepare(sql).map_err(storage_error)?;
        let rows = statement
            .query_map(params![&workspace_id[..], &target_branch_id[..]], |row| {
                row.get::<_, Vec<u8>>(0)
            })
            .map_err(storage_error)?;
        for row in rows {
            ids.push(decode_merge_id(
                "merge_attempt.merge_id",
                row.map_err(storage_error)?,
            )?);
        }
    } else {
        let sql = if options.includes_closed() {
            "SELECT merge_attempt.merge_id
             FROM merge_attempt
             WHERE merge_attempt.workspace_id = ?1
             ORDER BY merge_attempt.created_at_us, merge_attempt.merge_id"
        } else {
            "SELECT merge_attempt.merge_id
             FROM merge_attempt
             LEFT JOIN merge_attempt_outcome
               ON merge_attempt_outcome.merge_id = merge_attempt.merge_id
             WHERE merge_attempt.workspace_id = ?1
               AND merge_attempt_outcome.merge_id IS NULL
             ORDER BY merge_attempt.created_at_us, merge_attempt.merge_id"
        };
        let mut statement = connection.prepare(sql).map_err(storage_error)?;
        let rows = statement
            .query_map(params![&workspace_id[..]], |row| row.get::<_, Vec<u8>>(0))
            .map_err(storage_error)?;
        for row in rows {
            ids.push(decode_merge_id(
                "merge_attempt.merge_id",
                row.map_err(storage_error)?,
            )?);
        }
    }
    Ok(ids)
}

fn load_merge_attempt_snapshot(
    connection: &Connection,
    merge_id: MergeId,
) -> Result<MergeAttemptSnapshot> {
    let row = connection
        .query_row(
            "SELECT merge_attempt.workspace_id,
                    merge_attempt.target_branch_id,
                    merge_attempt.source_branch_id,
                    merge_attempt.merge_base_commit_id,
                    merge_attempt.target_head_commit_id,
                    merge_attempt.source_head_commit_id,
                    merge_attempt.origin_session_id,
                    merge_attempt.created_at_us,
                    merge_runtime.runtime_json,
                    merge_attempt_outcome.outcome,
                    merge_attempt_outcome.result_commit_id,
                    merge_attempt_outcome.completed_at_us,
                    merge_attempt_outcome.detail_json
             FROM merge_attempt
             JOIN merge_runtime
               ON merge_runtime.merge_id = merge_attempt.merge_id
             LEFT JOIN merge_attempt_outcome
               ON merge_attempt_outcome.merge_id = merge_attempt.merge_id
             WHERE merge_attempt.merge_id = ?1",
            params![&merge_id.raw_bytes()[..]],
            |row| {
                Ok((
                    row.get::<_, Vec<u8>>(0)?,
                    row.get::<_, Vec<u8>>(1)?,
                    row.get::<_, Vec<u8>>(2)?,
                    row.get::<_, Vec<u8>>(3)?,
                    row.get::<_, Vec<u8>>(4)?,
                    row.get::<_, Vec<u8>>(5)?,
                    row.get::<_, Option<Vec<u8>>>(6)?,
                    row.get::<_, i64>(7)?,
                    row.get::<_, String>(8)?,
                    row.get::<_, Option<String>>(9)?,
                    row.get::<_, Option<Vec<u8>>>(10)?,
                    row.get::<_, Option<i64>>(11)?,
                    row.get::<_, Option<String>>(12)?,
                ))
            },
        )
        .optional()
        .map_err(storage_error)?;
    let Some((
        workspace_id,
        target_branch_id,
        source_branch_id,
        merge_base_commit_id,
        target_head_commit_id,
        source_head_commit_id,
        origin_session_id,
        created_at_us,
        runtime_json,
        outcome,
        result_commit_id,
        completed_at_us,
        detail_json,
    )) = row
    else {
        return Err(WorkVcsError::WorkspaceInvalid(format!(
            "merge {merge_id} does not exist"
        )));
    };
    let runtime_state = parse_runtime_state(merge_id, &runtime_json, "merge")?;
    let outcome = match outcome {
        Some(outcome) => Some(decode_outcome_snapshot(
            merge_id,
            &outcome,
            result_commit_id,
            completed_at_us,
            detail_json,
        )?),
        None => None,
    };
    validate_snapshot_state(merge_id, runtime_state, outcome.as_ref())?;
    let items = load_merge_items(connection, merge_id)?;
    Ok(MergeAttemptSnapshot {
        merge_id,
        workspace_id: decode_workspace_id("merge_attempt.workspace_id", workspace_id)?,
        target_branch_id: decode_branch_id("merge_attempt.target_branch_id", target_branch_id)?,
        source_branch_id: decode_branch_id("merge_attempt.source_branch_id", source_branch_id)?,
        merge_base_commit_id: decode_commit_id(
            "merge_attempt.merge_base_commit_id",
            merge_base_commit_id,
        )?,
        target_head_commit_id: decode_commit_id(
            "merge_attempt.target_head_commit_id",
            target_head_commit_id,
        )?,
        source_head_commit_id: decode_commit_id(
            "merge_attempt.source_head_commit_id",
            source_head_commit_id,
        )?,
        origin_session_id: origin_session_id
            .map(|bytes| decode_session_id("merge_attempt.origin_session_id", bytes))
            .transpose()?,
        created_at_us,
        runtime_state,
        items,
        outcome,
    })
}

fn classify_merge_items(
    connection: &StoreConnection,
    workspace_id: WorkspaceId,
    merge_base_commit_id: CommitId,
    target_head_commit_id: CommitId,
    source_head_commit_id: CommitId,
) -> Result<Vec<PreparedMergeItem>> {
    let base = history::state_at(connection, merge_base_commit_id)?;
    let target = history::state_at(connection, target_head_commit_id)?;
    let source = history::state_at(connection, source_head_commit_id)?;
    if base.workspace_id != workspace_id
        || target.workspace_id != workspace_id
        || source.workspace_id != workspace_id
    {
        return Err(WorkVcsError::WorkspaceInvalid(format!(
            "merge state replay crossed workspace boundary for workspace {workspace_id}"
        )));
    }

    let base_entities = entity_map(base.state.entities());
    let target_entities = entity_map(target.state.entities());
    let source_entities = entity_map(source.state.entities());
    let base_relations = relation_map(base.state.relations());
    let target_relations = relation_map(target.state.relations());
    let source_relations = relation_map(source.state.relations());

    let mut items = Vec::new();
    classify_entity_items(
        &base_entities,
        &target_entities,
        &source_entities,
        &mut items,
    )?;
    classify_relation_items(
        &base_relations,
        &target_relations,
        &source_relations,
        &mut items,
    )?;
    Ok(items)
}

fn classify_entity_items(
    base: &BTreeMap<EntityId, EntityVersionId>,
    target: &BTreeMap<EntityId, EntityVersionId>,
    source: &BTreeMap<EntityId, EntityVersionId>,
    items: &mut Vec<PreparedMergeItem>,
) -> Result<()> {
    let mut entity_ids = BTreeSet::new();
    entity_ids.extend(base.keys().copied());
    entity_ids.extend(target.keys().copied());
    entity_ids.extend(source.keys().copied());
    for entity_id in entity_ids {
        let base_version = base.get(&entity_id).copied();
        let target_version = target.get(&entity_id).copied();
        let source_version = source.get(&entity_id).copied();
        let Some(classification) = classify_versions(base_version, target_version, source_version)
        else {
            continue;
        };
        let payload = CanonicalValue::object(vec![
            (
                "base_entity_version_id".to_owned(),
                optional_entity_version_value(base_version),
            ),
            (
                "entity_id".to_owned(),
                CanonicalValue::String(entity_id.to_string()),
            ),
            (
                "kind".to_owned(),
                CanonicalValue::String(ENTITY_MERGE_ITEM_SUBJECT_KIND.to_owned()),
            ),
            (
                "source_entity_version_id".to_owned(),
                optional_entity_version_value(source_version),
            ),
            (
                "target_entity_version_id".to_owned(),
                optional_entity_version_value(target_version),
            ),
        ])?;
        items.push(PreparedMergeItem {
            merge_item_id: MergeItemId::new_v7(),
            ordinal: items.len() as i64,
            classification,
            subject: MergeItemSubject::Entity(entity_id),
            payload_json: canonical_json_string(&payload)?,
        });
    }
    Ok(())
}

fn classify_relation_items(
    base: &BTreeMap<RelationId, RelationVersionId>,
    target: &BTreeMap<RelationId, RelationVersionId>,
    source: &BTreeMap<RelationId, RelationVersionId>,
    items: &mut Vec<PreparedMergeItem>,
) -> Result<()> {
    let mut relation_ids = BTreeSet::new();
    relation_ids.extend(base.keys().copied());
    relation_ids.extend(target.keys().copied());
    relation_ids.extend(source.keys().copied());
    for relation_id in relation_ids {
        let base_version = base.get(&relation_id).copied();
        let target_version = target.get(&relation_id).copied();
        let source_version = source.get(&relation_id).copied();
        let Some(classification) = classify_versions(base_version, target_version, source_version)
        else {
            continue;
        };
        let payload = CanonicalValue::object(vec![
            (
                "base_relation_version_id".to_owned(),
                optional_relation_version_value(base_version),
            ),
            (
                "kind".to_owned(),
                CanonicalValue::String(RELATION_MERGE_ITEM_SUBJECT_KIND.to_owned()),
            ),
            (
                "relation_id".to_owned(),
                CanonicalValue::String(relation_id.to_string()),
            ),
            (
                "source_relation_version_id".to_owned(),
                optional_relation_version_value(source_version),
            ),
            (
                "target_relation_version_id".to_owned(),
                optional_relation_version_value(target_version),
            ),
        ])?;
        items.push(PreparedMergeItem {
            merge_item_id: MergeItemId::new_v7(),
            ordinal: items.len() as i64,
            classification,
            subject: MergeItemSubject::Relation(relation_id),
            payload_json: canonical_json_string(&payload)?,
        });
    }
    Ok(())
}

fn classify_versions<VersionId: Copy + Eq>(
    base: Option<VersionId>,
    target: Option<VersionId>,
    source: Option<VersionId>,
) -> Option<MergeItemClassification> {
    if target == source || source == base {
        None
    } else if target == base {
        Some(MergeItemClassification::Auto)
    } else {
        Some(MergeItemClassification::Conflict)
    }
}

fn entity_map(entries: &[(EntityId, EntityVersionId)]) -> BTreeMap<EntityId, EntityVersionId> {
    entries.iter().copied().collect()
}

fn relation_map(
    entries: &[(RelationId, RelationVersionId)],
) -> BTreeMap<RelationId, RelationVersionId> {
    entries.iter().copied().collect()
}

fn optional_entity_version_value(version: Option<EntityVersionId>) -> CanonicalValue {
    version
        .map(|version| CanonicalValue::String(version.to_string()))
        .unwrap_or(CanonicalValue::Null)
}

fn optional_relation_version_value(version: Option<RelationVersionId>) -> CanonicalValue {
    version
        .map(|version| CanonicalValue::String(version.to_string()))
        .unwrap_or(CanonicalValue::Null)
}

fn insert_merge_items(
    transaction: &Transaction<'_>,
    merge_id: MergeId,
    items: &[PreparedMergeItem],
) -> Result<()> {
    let merge_id_bytes = merge_id.raw_bytes();
    for item in items {
        let merge_item_id_bytes = item.merge_item_id.raw_bytes();
        let subject_object_id_bytes = item.subject.object_id_bytes();
        transaction
            .execute(
                "INSERT INTO merge_item(
                    merge_item_id,
                    merge_id,
                    ordinal,
                    classification,
                    subject_object_id,
                    item_payload_json
                 )
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    &merge_item_id_bytes[..],
                    &merge_id_bytes[..],
                    item.ordinal,
                    item.classification.as_str(),
                    &subject_object_id_bytes[..],
                    item.payload_json
                ],
            )
            .map_err(storage_error)?;
    }
    Ok(())
}

fn load_merge_items(connection: &Connection, merge_id: MergeId) -> Result<Vec<MergeItemSnapshot>> {
    let mut statement = connection
        .prepare(
            "SELECT merge_item.merge_item_id,
                    merge_item.ordinal,
                    merge_item.classification,
                    merge_item.subject_object_id,
                    object_identity.object_kind,
                    merge_item.item_payload_json
             FROM merge_item
             LEFT JOIN object_identity
               ON object_identity.object_id = merge_item.subject_object_id
             WHERE merge_item.merge_id = ?1
             ORDER BY merge_item.ordinal",
        )
        .map_err(storage_error)?;
    let rows = statement
        .query_map(params![&merge_id.raw_bytes()[..]], |row| {
            Ok((
                row.get::<_, Vec<u8>>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, Option<Vec<u8>>>(3)?,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, String>(5)?,
            ))
        })
        .map_err(storage_error)?;

    let mut items = Vec::new();
    for row in rows {
        let (
            merge_item_id,
            ordinal,
            classification,
            subject_object_id,
            object_kind,
            item_payload_json,
        ) = row.map_err(storage_error)?;
        validate_stored_text("merge_item.classification", &classification)?;
        let classification =
            MergeItemClassification::from_str(&classification).ok_or_else(|| {
                WorkVcsError::WorkspaceInvalid(format!(
                    "merge {merge_id} has unsupported merge_item classification {classification:?}"
                ))
            })?;
        let subject = decode_merge_item_subject(merge_id, subject_object_id, object_kind)?;
        let payload = parse_canonical_json(item_payload_json.as_bytes()).map_err(|error| {
            WorkVcsError::WorkspaceInvalid(format!(
                "merge {merge_id} item_payload_json is not canonical JSON: {error}"
            ))
        })?;
        require_object_value("merge item payload", &payload)?;
        items.push(MergeItemSnapshot {
            merge_item_id: decode_merge_item_id("merge_item.merge_item_id", merge_item_id)?,
            ordinal,
            classification,
            subject,
            payload,
        });
    }
    Ok(items)
}

fn decode_merge_item_subject(
    merge_id: MergeId,
    subject_object_id: Option<Vec<u8>>,
    object_kind: Option<String>,
) -> Result<Option<MergeItemSubject>> {
    match (subject_object_id, object_kind) {
        (None, None) => Ok(None),
        (Some(bytes), Some(object_kind)) => {
            validate_stored_text("object_identity.object_kind", &object_kind)?;
            match object_kind.as_str() {
                ENTITY_MERGE_ITEM_SUBJECT_KIND => Ok(Some(MergeItemSubject::Entity(
                    decode_entity_id("merge_item.subject_object_id", bytes)?,
                ))),
                RELATION_MERGE_ITEM_SUBJECT_KIND => Ok(Some(MergeItemSubject::Relation(
                    decode_relation_id("merge_item.subject_object_id", bytes)?,
                ))),
                _ => Err(WorkVcsError::WorkspaceInvalid(format!(
                    "merge {merge_id} item subject has unsupported object kind {object_kind:?}"
                ))),
            }
        }
        (Some(_), None) => Err(WorkVcsError::WorkspaceInvalid(format!(
            "merge {merge_id} item subject_object_id does not resolve to object_identity"
        ))),
        (None, Some(object_kind)) => Err(WorkVcsError::WorkspaceInvalid(format!(
            "merge {merge_id} item has object kind {object_kind:?} without subject_object_id"
        ))),
    }
}

fn find_merge_base(
    connection: &Connection,
    workspace_id: WorkspaceId,
    target_head_commit_id: CommitId,
    source_head_commit_id: CommitId,
) -> Result<CommitId> {
    let target_ancestors = ancestor_depths(connection, workspace_id, target_head_commit_id)?;
    let source_ancestors = ancestor_depths(connection, workspace_id, source_head_commit_id)?;
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
    connection: &Connection,
    workspace_id: WorkspaceId,
    start_commit_id: CommitId,
) -> Result<BTreeMap<CommitId, usize>> {
    let mut ancestors = BTreeMap::new();
    let mut queue = VecDeque::from([(start_commit_id, 0usize)]);
    while let Some((commit_id, depth)) = queue.pop_front() {
        if ancestors.contains_key(&commit_id) {
            continue;
        }
        ensure_commit_belongs_to_workspace(connection, workspace_id, commit_id)?;
        ancestors.insert(commit_id, depth);
        for parent_commit_id in parent_commit_ids(connection, commit_id)? {
            queue.push_back((parent_commit_id, depth + 1));
        }
    }
    Ok(ancestors)
}

fn ensure_commit_belongs_to_workspace(
    connection: &Connection,
    workspace_id: WorkspaceId,
    commit_id: CommitId,
) -> Result<()> {
    let row = connection
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

fn parent_commit_ids(connection: &Connection, commit_id: CommitId) -> Result<Vec<CommitId>> {
    let mut statement = connection
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

fn runtime_json(state: MergeRuntimeState) -> Result<String> {
    canonical_json_string(&CanonicalValue::object(vec![(
        "lifecycle_state".to_owned(),
        CanonicalValue::String(state.as_str().to_owned()),
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

struct MergeAbortedPayload {
    merge: MergeAttemptRow,
    abort_session_id: Option<SessionId>,
    detail: CanonicalValue,
}

fn merge_aborted_payload_json(payload: &MergeAbortedPayload) -> Result<String> {
    let abort_session_id = match payload.abort_session_id {
        Some(abort_session_id) => CanonicalValue::String(abort_session_id.to_string()),
        None => CanonicalValue::Null,
    };
    let origin_session_id = match payload.merge.origin_session_id {
        Some(origin_session_id) => CanonicalValue::String(origin_session_id.to_string()),
        None => CanonicalValue::Null,
    };
    canonical_json_string(&CanonicalValue::object(vec![
        ("abort_session_id".to_owned(), abort_session_id),
        ("detail".to_owned(), payload.detail.clone()),
        (
            "lifecycle_state".to_owned(),
            CanonicalValue::String(ABORTED_MERGE_RUNTIME_STATE.to_owned()),
        ),
        (
            "merge_base_commit_id".to_owned(),
            CanonicalValue::String(payload.merge.merge_base_commit_id.to_string()),
        ),
        (
            "merge_id".to_owned(),
            CanonicalValue::String(payload.merge.merge_id.to_string()),
        ),
        ("origin_session_id".to_owned(), origin_session_id),
        (
            "outcome".to_owned(),
            CanonicalValue::String(ABORTED_MERGE_OUTCOME.to_owned()),
        ),
        (
            "source_branch_id".to_owned(),
            CanonicalValue::String(payload.merge.source_branch_id.to_string()),
        ),
        (
            "source_head_commit_id".to_owned(),
            CanonicalValue::String(payload.merge.source_head_commit_id.to_string()),
        ),
        (
            "target_branch_id".to_owned(),
            CanonicalValue::String(payload.merge.target_branch_id.to_string()),
        ),
        (
            "target_head_commit_id".to_owned(),
            CanonicalValue::String(payload.merge.target_head_commit_id.to_string()),
        ),
        (
            "workspace_id".to_owned(),
            CanonicalValue::String(payload.merge.workspace_id.to_string()),
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

fn validate_runtime_json(
    merge_id: MergeId,
    runtime_json: &str,
    expected_state: MergeRuntimeState,
    label: &str,
) -> Result<()> {
    let state = parse_runtime_state(merge_id, runtime_json, label)?;
    if state == expected_state {
        Ok(())
    } else {
        Err(WorkVcsError::WorkspaceInvalid(format!(
            "{label} {merge_id} runtime_json must project lifecycle_state {}",
            expected_state.as_str()
        )))
    }
}

fn parse_runtime_state(
    merge_id: MergeId,
    runtime_json: &str,
    label: &str,
) -> Result<MergeRuntimeState> {
    validate_stored_text("merge_runtime.runtime_json", runtime_json)?;
    let value = parse_canonical_json(runtime_json.as_bytes()).map_err(|error| {
        WorkVcsError::WorkspaceInvalid(format!(
            "{label} {merge_id} runtime_json is not canonical JSON: {error}"
        ))
    })?;
    let CanonicalValue::Object(entries) = value else {
        return Err(WorkVcsError::WorkspaceInvalid(format!(
            "{label} {merge_id} runtime_json must be an object"
        )));
    };
    if entries.len() != 1 {
        return Err(WorkVcsError::WorkspaceInvalid(format!(
            "{label} {merge_id} runtime_json must contain only lifecycle_state"
        )));
    }
    match &entries[0] {
        (key, CanonicalValue::String(value)) if key == "lifecycle_state" => {
            MergeRuntimeState::from_str(value).ok_or_else(|| {
                WorkVcsError::WorkspaceInvalid(format!(
                    "{label} {merge_id} runtime_json has unsupported lifecycle_state {value:?}"
                ))
            })
        }
        _ => Err(WorkVcsError::WorkspaceInvalid(format!(
            "{label} {merge_id} runtime_json must project lifecycle_state"
        ))),
    }
}

fn decode_outcome_snapshot(
    merge_id: MergeId,
    outcome: &str,
    result_commit_id: Option<Vec<u8>>,
    completed_at_us: Option<i64>,
    detail_json: Option<String>,
) -> Result<MergeOutcomeSnapshot> {
    validate_stored_text("merge_attempt_outcome.outcome", outcome)?;
    let outcome = MergeOutcome::from_str(outcome).ok_or_else(|| {
        WorkVcsError::WorkspaceInvalid(format!(
            "merge {merge_id} has unsupported outcome {outcome:?}"
        ))
    })?;
    let completed_at_us = completed_at_us.ok_or_else(|| {
        WorkVcsError::WorkspaceInvalid(format!(
            "merge {merge_id} outcome must include completed_at_us"
        ))
    })?;
    let detail_json = detail_json.ok_or_else(|| {
        WorkVcsError::WorkspaceInvalid(format!("merge {merge_id} outcome must include detail_json"))
    })?;
    let detail = parse_canonical_json(detail_json.as_bytes()).map_err(|error| {
        WorkVcsError::WorkspaceInvalid(format!(
            "merge {merge_id} outcome detail_json is not canonical JSON: {error}"
        ))
    })?;
    require_object_value("merge outcome detail", &detail)?;
    Ok(MergeOutcomeSnapshot {
        outcome,
        result_commit_id: result_commit_id
            .map(|bytes| decode_commit_id("merge_attempt_outcome.result_commit_id", bytes))
            .transpose()?,
        completed_at_us,
        detail,
    })
}

fn validate_snapshot_state(
    merge_id: MergeId,
    runtime_state: MergeRuntimeState,
    outcome: Option<&MergeOutcomeSnapshot>,
) -> Result<()> {
    match (runtime_state, outcome) {
        (MergeRuntimeState::Active, None) => Ok(()),
        (MergeRuntimeState::Aborted, Some(outcome))
            if outcome.outcome == MergeOutcome::Aborted && outcome.result_commit_id.is_none() =>
        {
            Ok(())
        }
        (MergeRuntimeState::Completed, Some(outcome))
            if outcome.outcome == MergeOutcome::Completed && outcome.result_commit_id.is_some() =>
        {
            Ok(())
        }
        _ => Err(WorkVcsError::WorkspaceInvalid(format!(
            "merge {merge_id} runtime and outcome are inconsistent"
        ))),
    }
}

fn require_object_value(label: &str, value: &CanonicalValue) -> Result<()> {
    match value {
        CanonicalValue::Object(_) => Ok(()),
        _ => Err(WorkVcsError::WorkspaceInvalid(format!(
            "{label} must be an object"
        ))),
    }
}

fn decode_branch_id(column: &str, bytes: Vec<u8>) -> Result<BranchId> {
    BranchId::from_bytes(decode_uuid_bytes(column, bytes)?).map_err(|error| {
        WorkVcsError::WorkspaceInvalid(format!("{column} is not a valid BranchId: {error}"))
    })
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

fn decode_merge_item_id(column: &str, bytes: Vec<u8>) -> Result<MergeItemId> {
    MergeItemId::from_bytes(decode_uuid_bytes(column, bytes)?).map_err(|error| {
        WorkVcsError::WorkspaceInvalid(format!("{column} is not a valid MergeItemId: {error}"))
    })
}

fn decode_entity_id(column: &str, bytes: Vec<u8>) -> Result<EntityId> {
    EntityId::from_bytes(decode_uuid_bytes(column, bytes)?).map_err(|error| {
        WorkVcsError::WorkspaceInvalid(format!("{column} is not a valid EntityId: {error}"))
    })
}

fn decode_relation_id(column: &str, bytes: Vec<u8>) -> Result<RelationId> {
    RelationId::from_bytes(decode_uuid_bytes(column, bytes)?).map_err(|error| {
        WorkVcsError::WorkspaceInvalid(format!("{column} is not a valid RelationId: {error}"))
    })
}

fn decode_session_id(column: &str, bytes: Vec<u8>) -> Result<SessionId> {
    SessionId::from_bytes(decode_uuid_bytes(column, bytes)?).map_err(|error| {
        WorkVcsError::WorkspaceInvalid(format!("{column} is not a valid SessionId: {error}"))
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
