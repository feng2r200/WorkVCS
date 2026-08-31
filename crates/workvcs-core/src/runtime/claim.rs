use super::runnable::{self, RunnableTaskClaimCoordination, RunnableTasksOptions};
use super::session::{self, SessionFocus, SessionLifecycleState};
use crate::canonical::{CanonicalValue, canonical_bytes};
use crate::error::{Result, WorkVcsError, storage_error};
use crate::history;
use crate::identity::{BranchId, ClaimId, CommitId, EntityId, EventId, SessionId, WorkspaceId};
use crate::store::{StoreConnection, current_epoch_micros};
use rusqlite::{Connection, OptionalExtension, Transaction, TransactionBehavior, params};
use std::collections::BTreeSet;

const ACTIVE_BRANCH_LIFECYCLE_STATE: &str = "active";
const CLAIM_OBJECT_KIND: &str = "claim";
const EXCLUSIVE_CLAIM_MODE: &str = "exclusive";
const SHARED_CLAIM_MODE: &str = "shared";
const ACTIVE_CLAIM_LIFECYCLE_STATE: &str = "active";
const RELEASED_CLAIM_LIFECYCLE_STATE: &str = "released";
const CLAIM_CREATED_EVENT_KIND: &str = "claim.created";
const CLAIM_RELEASED_EVENT_KIND: &str = "claim.released";
const CLAIM_TRANSFERRED_EVENT_KIND: &str = "claim.transferred";
const CLAIM_FORCE_TAKEN_OVER_EVENT_KIND: &str = "claim.force_taken_over";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClaimLifecycleState {
    Active,
    Released,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClaimMode {
    Exclusive,
    Shared,
}

impl ClaimMode {
    fn as_str(self) -> &'static str {
        match self {
            Self::Exclusive => EXCLUSIVE_CLAIM_MODE,
            Self::Shared => SHARED_CLAIM_MODE,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClaimGuardAction {
    TerminalTaskMutation,
    StructuralTaskMutation,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClaimGuardReason {
    Unclaimed,
    OwnedExclusiveClaim,
    UniqueSharedClaimant,
    ExclusiveClaimOwnedByOtherSession,
    SharedClaimSetDoesNotIncludeSession,
    NonUniqueSharedClaimSet,
}

impl ClaimGuardReason {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Unclaimed => "unclaimed",
            Self::OwnedExclusiveClaim => "owned_exclusive_claim",
            Self::UniqueSharedClaimant => "unique_shared_claimant",
            Self::ExclusiveClaimOwnedByOtherSession => "exclusive_claim_owned_by_other_session",
            Self::SharedClaimSetDoesNotIncludeSession => {
                "shared_claim_set_does_not_include_session"
            }
            Self::NonUniqueSharedClaimSet => "non_unique_shared_claim_set",
        }
    }

    fn allows_protected_task_action(self) -> bool {
        matches!(
            self,
            Self::Unclaimed | Self::OwnedExclusiveClaim | Self::UniqueSharedClaimant
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClaimTaskOptions {
    session_id: SessionId,
    task_entity_id: EntityId,
    mode: ClaimMode,
}

impl ClaimTaskOptions {
    pub fn new(session_id: SessionId, task_entity_id: EntityId) -> Self {
        Self {
            session_id,
            task_entity_id,
            mode: ClaimMode::Exclusive,
        }
    }

    pub fn with_mode(mut self, mode: ClaimMode) -> Self {
        self.mode = mode;
        self
    }

    pub fn session_id(&self) -> SessionId {
        self.session_id
    }

    pub fn task_entity_id(&self) -> EntityId {
        self.task_entity_id
    }

    pub fn mode(&self) -> ClaimMode {
        self.mode
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClaimReleaseOptions {
    session_id: SessionId,
    claim_id: ClaimId,
}

impl ClaimReleaseOptions {
    pub fn new(session_id: SessionId, claim_id: ClaimId) -> Self {
        Self {
            session_id,
            claim_id,
        }
    }

    pub fn session_id(&self) -> SessionId {
        self.session_id
    }

    pub fn claim_id(&self) -> ClaimId {
        self.claim_id
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClaimTransferOptions {
    from_session_id: SessionId,
    to_session_id: SessionId,
    claim_id: ClaimId,
}

impl ClaimTransferOptions {
    pub fn new(from_session_id: SessionId, to_session_id: SessionId, claim_id: ClaimId) -> Self {
        Self {
            from_session_id,
            to_session_id,
            claim_id,
        }
    }

    pub fn from_session_id(&self) -> SessionId {
        self.from_session_id
    }

    pub fn to_session_id(&self) -> SessionId {
        self.to_session_id
    }

    pub fn claim_id(&self) -> ClaimId {
        self.claim_id
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClaimForceTakeoverOptions {
    session_id: SessionId,
    claim_id: ClaimId,
    rationale: String,
}

impl ClaimForceTakeoverOptions {
    pub fn new(
        session_id: SessionId,
        claim_id: ClaimId,
        rationale: impl Into<String>,
    ) -> Result<Self> {
        let rationale = rationale.into();
        if rationale.trim().is_empty() {
            return Err(WorkVcsError::ClaimInvalid(
                "claim force takeover rationale must not be empty".to_owned(),
            ));
        }
        Ok(Self {
            session_id,
            claim_id,
            rationale,
        })
    }

    pub fn session_id(&self) -> SessionId {
        self.session_id
    }

    pub fn claim_id(&self) -> ClaimId {
        self.claim_id
    }

    pub fn rationale(&self) -> &str {
        &self.rationale
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClaimNextOptions {
    session_id: SessionId,
    mode: ClaimMode,
}

impl ClaimNextOptions {
    pub fn new(session_id: SessionId) -> Self {
        Self {
            session_id,
            mode: ClaimMode::Exclusive,
        }
    }

    pub fn with_mode(mut self, mode: ClaimMode) -> Self {
        self.mode = mode;
        self
    }

    pub fn session_id(&self) -> SessionId {
        self.session_id
    }

    pub fn mode(&self) -> ClaimMode {
        self.mode
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClaimListOptions {
    session_id: SessionId,
}

impl ClaimListOptions {
    pub fn for_session(session_id: SessionId) -> Self {
        Self { session_id }
    }

    pub fn session_id(&self) -> SessionId {
        self.session_id
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClaimGuardOptions {
    session_id: SessionId,
    task_entity_id: EntityId,
    action: ClaimGuardAction,
}

impl ClaimGuardOptions {
    pub fn new(session_id: SessionId, task_entity_id: EntityId, action: ClaimGuardAction) -> Self {
        Self {
            session_id,
            task_entity_id,
            action,
        }
    }

    pub fn terminal_task_mutation(session_id: SessionId, task_entity_id: EntityId) -> Self {
        Self::new(
            session_id,
            task_entity_id,
            ClaimGuardAction::TerminalTaskMutation,
        )
    }

    pub fn structural_task_mutation(session_id: SessionId, task_entity_id: EntityId) -> Self {
        Self::new(
            session_id,
            task_entity_id,
            ClaimGuardAction::StructuralTaskMutation,
        )
    }

    pub fn session_id(&self) -> SessionId {
        self.session_id
    }

    pub fn task_entity_id(&self) -> EntityId {
        self.task_entity_id
    }

    pub fn action(&self) -> ClaimGuardAction {
        self.action
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClaimTaskResult {
    pub claim_id: ClaimId,
    pub session_id: SessionId,
    pub workspace_id: WorkspaceId,
    pub branch_id: BranchId,
    pub task_entity_id: EntityId,
    pub mode: ClaimMode,
    pub claimed_at_us: i64,
    pub state: ClaimSnapshot,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClaimReleaseResult {
    pub claim_id: ClaimId,
    pub session_id: SessionId,
    pub released_at_us: i64,
    pub state: ClaimSnapshot,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClaimTransferResult {
    pub previous_claim_id: ClaimId,
    pub claim_id: ClaimId,
    pub from_session_id: SessionId,
    pub to_session_id: SessionId,
    pub transferred_at_us: i64,
    pub previous_last_activity_at_us: i64,
    pub previous_state: ClaimSnapshot,
    pub state: ClaimSnapshot,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClaimForceTakeoverResult {
    pub previous_claim_id: ClaimId,
    pub claim_id: ClaimId,
    pub previous_session_id: SessionId,
    pub previous_session_lifecycle_state: SessionLifecycleState,
    pub session_id: SessionId,
    pub taken_over_at_us: i64,
    pub previous_last_activity_at_us: i64,
    pub rationale: String,
    pub previous_state: ClaimSnapshot,
    pub state: ClaimSnapshot,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClaimNextResult {
    pub session_id: SessionId,
    pub workspace_id: WorkspaceId,
    pub branch_id: BranchId,
    pub head_commit_id: CommitId,
    pub inspected_candidates: usize,
    pub selected: Option<ClaimTaskResult>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClaimListResult {
    pub session_id: SessionId,
    pub claims: Vec<ClaimSnapshot>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClaimGuardResult {
    pub session_id: SessionId,
    pub workspace_id: WorkspaceId,
    pub branch_id: BranchId,
    pub head_commit_id: CommitId,
    pub task_entity_id: EntityId,
    pub action: ClaimGuardAction,
    pub allowed: bool,
    pub reason: ClaimGuardReason,
    pub active_claims: Vec<ClaimSnapshot>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClaimSnapshot {
    pub claim_id: ClaimId,
    pub lifecycle_state: ClaimLifecycleState,
    pub session_id: SessionId,
    pub workspace_id: WorkspaceId,
    pub branch_id: BranchId,
    pub task_entity_id: EntityId,
    pub mode: ClaimMode,
    pub created_at_us: i64,
    pub last_activity_at_us: Option<i64>,
}

pub(crate) fn claim_next_task(
    connection: &mut StoreConnection,
    options: &ClaimNextOptions,
) -> Result<ClaimNextResult> {
    connection.verify_foreign_keys()?;
    let projection =
        runnable::runnable_tasks(connection, &RunnableTasksOptions::new(options.session_id()))?;
    let selected_candidate = projection
        .candidates
        .iter()
        .find(|candidate| claim_next_candidate_matches_mode(candidate, options.mode()))
        .cloned();
    let Some(selected_candidate) = selected_candidate else {
        return Ok(ClaimNextResult {
            session_id: options.session_id(),
            workspace_id: projection.workspace_id,
            branch_id: projection.branch_id,
            head_commit_id: projection.head_commit_id,
            inspected_candidates: projection.candidates.len(),
            selected: None,
        });
    };

    let claim_id = ClaimId::new_v7();
    let event_id = EventId::new_v7();
    let now_us = current_epoch_micros()?;
    let event_payload_json = claim_created_payload_json(
        claim_id,
        options.session_id(),
        projection.workspace_id,
        projection.branch_id,
        selected_candidate.task.task_entity_id,
        options.mode(),
    )?;

    let claim_id_bytes = claim_id.raw_bytes();
    let event_id_bytes = event_id.raw_bytes();
    let session_id_bytes = options.session_id().raw_bytes();
    let workspace_id_bytes = projection.workspace_id.raw_bytes();
    let branch_id_bytes = projection.branch_id.raw_bytes();
    let task_entity_id_bytes = selected_candidate.task.task_entity_id.raw_bytes();

    let transaction = connection
        .inner_mut()
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(storage_error)?;
    let current_runtime =
        session::load_active_session_runtime_for_update(&transaction, options.session_id())?;
    if current_runtime.active_workspace_id != projection.workspace_id
        || current_runtime.active_branch_id != projection.branch_id
    {
        return Err(WorkVcsError::ClaimInvalid(format!(
            "session {} active target changed before claim-next could be written",
            options.session_id()
        )));
    }
    let current_branch = load_active_branch(&transaction, projection.branch_id)?;
    if current_branch.workspace_id != projection.workspace_id {
        return Err(WorkVcsError::ClaimInvalid(format!(
            "session {} active branch {} belongs to workspace {}, not active workspace {}",
            options.session_id(),
            projection.branch_id,
            current_branch.workspace_id,
            projection.workspace_id
        )));
    }
    if current_branch.head_commit_id != projection.head_commit_id {
        return Err(WorkVcsError::BranchHeadConflict(format!(
            "branch {} head changed before claim-next {} could be written",
            projection.branch_id, claim_id
        )));
    }
    ensure_active_claim_mode_allowed(
        &transaction,
        projection.workspace_id,
        projection.branch_id,
        selected_candidate.task.task_entity_id,
        options.session_id(),
        options.mode(),
    )?;

    transaction
        .execute(
            "INSERT INTO object_identity(object_id, object_kind, created_at_us)
             VALUES (?1, ?2, ?3)",
            params![&claim_id_bytes[..], CLAIM_OBJECT_KIND, now_us],
        )
        .map_err(storage_error)?;
    transaction
        .execute(
            "INSERT INTO claim(
                claim_id,
                session_id,
                workspace_id,
                branch_id,
                task_entity_id,
                mode,
                created_at_us
             )
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                &claim_id_bytes[..],
                &session_id_bytes[..],
                &workspace_id_bytes[..],
                &branch_id_bytes[..],
                &task_entity_id_bytes[..],
                options.mode().as_str(),
                now_us
            ],
        )
        .map_err(storage_error)?;
    transaction
        .execute(
            "INSERT INTO claim_runtime(claim_id, last_activity_at_us)
             VALUES (?1, ?2)",
            params![&claim_id_bytes[..], now_us],
        )
        .map_err(storage_error)?;
    session::replace_session_focus_with_event(
        &transaction,
        options.session_id(),
        projection.workspace_id,
        &SessionFocus {
            focus_entity_id: selected_candidate.task.task_entity_id,
            path: Vec::new(),
        },
        now_us,
    )?;
    session::update_session_activity(&transaction, options.session_id(), now_us)?;
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
                CLAIM_CREATED_EVENT_KIND,
                now_us,
                event_payload_json
            ],
        )
        .map_err(storage_error)?;

    transaction.commit().map_err(storage_error)?;

    let state = claim_snapshot(connection, claim_id)?;
    Ok(ClaimNextResult {
        session_id: options.session_id(),
        workspace_id: projection.workspace_id,
        branch_id: projection.branch_id,
        head_commit_id: projection.head_commit_id,
        inspected_candidates: projection.candidates.len(),
        selected: Some(ClaimTaskResult {
            claim_id,
            session_id: options.session_id(),
            workspace_id: projection.workspace_id,
            branch_id: projection.branch_id,
            task_entity_id: selected_candidate.task.task_entity_id,
            mode: options.mode(),
            claimed_at_us: now_us,
            state,
        }),
    })
}

fn claim_next_candidate_matches_mode(
    candidate: &runnable::RunnableTaskCandidate,
    mode: ClaimMode,
) -> bool {
    match mode {
        ClaimMode::Exclusive => {
            candidate.runnable
                && matches!(
                    candidate.claim_coordination,
                    RunnableTaskClaimCoordination::Unclaimed
                )
        }
        ClaimMode::Shared => {
            if candidate.runnable
                && matches!(
                    candidate.claim_coordination,
                    RunnableTaskClaimCoordination::Unclaimed
                )
            {
                return true;
            }
            matches!(
                &candidate.claim_coordination,
                RunnableTaskClaimCoordination::Shared {
                    claimed_by_session: false,
                    ..
                }
            ) && candidate.blocked_reasons.as_slice()
                == [runnable::RunnableTaskBlockedReason::ClaimBlocked]
        }
    }
}

pub(crate) fn release_claim(
    connection: &mut StoreConnection,
    options: &ClaimReleaseOptions,
) -> Result<ClaimReleaseResult> {
    connection.verify_foreign_keys()?;
    session::active_session_projection(connection, options.session_id())?;
    let now_us = current_epoch_micros()?;

    let transaction = connection
        .inner_mut()
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(storage_error)?;
    session::load_active_session_runtime_for_update(&transaction, options.session_id())?;
    let claim = load_claim(&transaction, options.claim_id())?;
    if claim.session_id != options.session_id() {
        return Err(WorkVcsError::ClaimInvalid(format!(
            "claim {} is owned by session {}, not releasing session {}",
            options.claim_id(),
            claim.session_id,
            options.session_id()
        )));
    }
    ensure_claim_runtime_exists(&transaction, options.claim_id())?;
    release_claim_runtime_for_row(&transaction, options.claim_id(), &claim, now_us)?;
    session::update_session_activity(&transaction, options.session_id(), now_us)?;
    transaction.commit().map_err(storage_error)?;

    Ok(ClaimReleaseResult {
        claim_id: options.claim_id(),
        session_id: options.session_id(),
        released_at_us: now_us,
        state: claim_snapshot(connection, options.claim_id())?,
    })
}

pub(crate) fn transfer_claim(
    connection: &mut StoreConnection,
    options: &ClaimTransferOptions,
) -> Result<ClaimTransferResult> {
    connection.verify_foreign_keys()?;
    if options.from_session_id() == options.to_session_id() {
        return Err(WorkVcsError::ClaimInvalid(
            "claim transfer source and target sessions must differ".to_owned(),
        ));
    }
    let now_us = current_epoch_micros()?;

    let transaction = connection
        .inner_mut()
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(storage_error)?;
    let from_active =
        session::load_active_session_runtime_for_update(&transaction, options.from_session_id())?;
    let to_active =
        session::load_active_session_runtime_for_update(&transaction, options.to_session_id())?;
    let previous_claim = load_claim(&transaction, options.claim_id())?;
    if previous_claim.session_id != options.from_session_id() {
        return Err(WorkVcsError::ClaimInvalid(format!(
            "claim {} is owned by session {}, not transfer source {}",
            options.claim_id(),
            previous_claim.session_id,
            options.from_session_id()
        )));
    }
    ensure_claim_session_target(
        "source",
        options.from_session_id(),
        &from_active,
        options.claim_id(),
        &previous_claim,
    )?;
    ensure_claim_session_target(
        "target",
        options.to_session_id(),
        &to_active,
        options.claim_id(),
        &previous_claim,
    )?;
    let previous_last_activity_at_us =
        require_active_claim_runtime(&transaction, options.claim_id())?;

    delete_claim_runtime_row(&transaction, options.claim_id())?;
    ensure_active_claim_mode_allowed(
        &transaction,
        previous_claim.workspace_id,
        previous_claim.branch_id,
        previous_claim.task_entity_id,
        options.to_session_id(),
        previous_claim.mode,
    )?;

    let claim_id = ClaimId::new_v7();
    insert_claim_occurrence(
        &transaction,
        claim_id,
        options.to_session_id(),
        &previous_claim,
        now_us,
    )?;
    let event_payload_json = claim_transferred_payload_json(
        options.claim_id(),
        claim_id,
        &previous_claim,
        previous_last_activity_at_us,
        options.to_session_id(),
    )?;
    insert_claim_replacement_event(
        &transaction,
        options.from_session_id(),
        previous_claim.workspace_id,
        CLAIM_TRANSFERRED_EVENT_KIND,
        now_us,
        event_payload_json,
    )?;
    session::update_session_activity(&transaction, options.from_session_id(), now_us)?;
    session::update_session_activity(&transaction, options.to_session_id(), now_us)?;
    transaction.commit().map_err(storage_error)?;

    Ok(ClaimTransferResult {
        previous_claim_id: options.claim_id(),
        claim_id,
        from_session_id: options.from_session_id(),
        to_session_id: options.to_session_id(),
        transferred_at_us: now_us,
        previous_last_activity_at_us,
        previous_state: claim_snapshot(connection, options.claim_id())?,
        state: claim_snapshot(connection, claim_id)?,
    })
}

pub(crate) fn force_takeover_claim(
    connection: &mut StoreConnection,
    options: &ClaimForceTakeoverOptions,
) -> Result<ClaimForceTakeoverResult> {
    connection.verify_foreign_keys()?;
    let now_us = current_epoch_micros()?;

    let transaction = connection
        .inner_mut()
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(storage_error)?;
    let taking_active =
        session::load_active_session_runtime_for_update(&transaction, options.session_id())?;
    let previous_claim = load_claim(&transaction, options.claim_id())?;
    if previous_claim.session_id == options.session_id() {
        return Err(WorkVcsError::ClaimInvalid(format!(
            "session {} already owns active claim {}",
            options.session_id(),
            options.claim_id()
        )));
    }
    ensure_claim_session_target(
        "taking",
        options.session_id(),
        &taking_active,
        options.claim_id(),
        &previous_claim,
    )?;
    let previous_last_activity_at_us =
        require_active_claim_runtime(&transaction, options.claim_id())?;
    let previous_session_lifecycle_state =
        session::session_lifecycle_state_for_connection(&transaction, previous_claim.session_id)?;
    if previous_session_lifecycle_state != SessionLifecycleState::PotentiallyStale {
        return Err(WorkVcsError::ClaimInvalid(format!(
            "claim {} previous session {} must be potentially_stale before forced takeover",
            options.claim_id(),
            previous_claim.session_id
        )));
    }

    delete_claim_runtime_row(&transaction, options.claim_id())?;
    ensure_active_claim_mode_allowed(
        &transaction,
        previous_claim.workspace_id,
        previous_claim.branch_id,
        previous_claim.task_entity_id,
        options.session_id(),
        previous_claim.mode,
    )?;

    let claim_id = ClaimId::new_v7();
    insert_claim_occurrence(
        &transaction,
        claim_id,
        options.session_id(),
        &previous_claim,
        now_us,
    )?;
    let event_payload_json = claim_force_taken_over_payload_json(
        options.claim_id(),
        claim_id,
        &previous_claim,
        previous_last_activity_at_us,
        session_lifecycle_state_label(previous_session_lifecycle_state),
        options.session_id(),
        options.rationale(),
    )?;
    insert_claim_replacement_event(
        &transaction,
        options.session_id(),
        previous_claim.workspace_id,
        CLAIM_FORCE_TAKEN_OVER_EVENT_KIND,
        now_us,
        event_payload_json,
    )?;
    session::update_session_activity(&transaction, options.session_id(), now_us)?;
    transaction.commit().map_err(storage_error)?;

    Ok(ClaimForceTakeoverResult {
        previous_claim_id: options.claim_id(),
        claim_id,
        previous_session_id: previous_claim.session_id,
        previous_session_lifecycle_state,
        session_id: options.session_id(),
        taken_over_at_us: now_us,
        previous_last_activity_at_us,
        rationale: options.rationale().to_owned(),
        previous_state: claim_snapshot(connection, options.claim_id())?,
        state: claim_snapshot(connection, claim_id)?,
    })
}

pub(crate) fn claim_task(
    connection: &mut StoreConnection,
    options: &ClaimTaskOptions,
) -> Result<ClaimTaskResult> {
    connection.verify_foreign_keys()?;
    let active = session::active_session_projection(connection, options.session_id())?;
    let branch_head = history::branch_head(connection, active.active_branch_id)?;
    if branch_head.workspace_id != active.active_workspace_id {
        return Err(WorkVcsError::ClaimInvalid(format!(
            "session {} active branch {} belongs to workspace {}, not active workspace {}",
            options.session_id(),
            active.active_branch_id,
            branch_head.workspace_id,
            active.active_workspace_id
        )));
    }
    if branch_head.lifecycle_state != ACTIVE_BRANCH_LIFECYCLE_STATE {
        return Err(WorkVcsError::ClaimInvalid(format!(
            "session {} active branch {} has lifecycle state {:?}",
            options.session_id(),
            active.active_branch_id,
            branch_head.lifecycle_state
        )));
    }
    history::task_at(
        connection,
        branch_head.head_commit_id,
        options.task_entity_id(),
    )?;

    let claim_id = ClaimId::new_v7();
    let event_id = EventId::new_v7();
    let now_us = current_epoch_micros()?;
    let event_payload_json = claim_created_payload_json(
        claim_id,
        options.session_id(),
        active.active_workspace_id,
        active.active_branch_id,
        options.task_entity_id(),
        options.mode(),
    )?;

    let claim_id_bytes = claim_id.raw_bytes();
    let event_id_bytes = event_id.raw_bytes();
    let session_id_bytes = options.session_id().raw_bytes();
    let workspace_id_bytes = active.active_workspace_id.raw_bytes();
    let branch_id_bytes = active.active_branch_id.raw_bytes();
    let task_entity_id_bytes = options.task_entity_id().raw_bytes();

    let transaction = connection
        .inner_mut()
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(storage_error)?;
    let current_runtime =
        session::load_active_session_runtime_for_update(&transaction, options.session_id())?;
    if current_runtime.active_workspace_id != active.active_workspace_id
        || current_runtime.active_branch_id != active.active_branch_id
    {
        return Err(WorkVcsError::ClaimInvalid(format!(
            "session {} active target changed before claim could be written",
            options.session_id()
        )));
    }
    let current_branch = load_active_branch(&transaction, active.active_branch_id)?;
    if current_branch.workspace_id != active.active_workspace_id {
        return Err(WorkVcsError::ClaimInvalid(format!(
            "session {} active branch {} belongs to workspace {}, not active workspace {}",
            options.session_id(),
            active.active_branch_id,
            current_branch.workspace_id,
            active.active_workspace_id
        )));
    }
    if current_branch.head_commit_id != branch_head.head_commit_id {
        return Err(WorkVcsError::BranchHeadConflict(format!(
            "branch {} head changed before claim {} could be written",
            active.active_branch_id, claim_id
        )));
    }
    ensure_active_claim_mode_allowed(
        &transaction,
        active.active_workspace_id,
        active.active_branch_id,
        options.task_entity_id(),
        options.session_id(),
        options.mode(),
    )?;

    transaction
        .execute(
            "INSERT INTO object_identity(object_id, object_kind, created_at_us)
             VALUES (?1, ?2, ?3)",
            params![&claim_id_bytes[..], CLAIM_OBJECT_KIND, now_us],
        )
        .map_err(storage_error)?;
    transaction
        .execute(
            "INSERT INTO claim(
                claim_id,
                session_id,
                workspace_id,
                branch_id,
                task_entity_id,
                mode,
                created_at_us
             )
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                &claim_id_bytes[..],
                &session_id_bytes[..],
                &workspace_id_bytes[..],
                &branch_id_bytes[..],
                &task_entity_id_bytes[..],
                options.mode().as_str(),
                now_us
            ],
        )
        .map_err(storage_error)?;
    transaction
        .execute(
            "INSERT INTO claim_runtime(claim_id, last_activity_at_us)
             VALUES (?1, ?2)",
            params![&claim_id_bytes[..], now_us],
        )
        .map_err(storage_error)?;
    session::update_session_activity(&transaction, options.session_id(), now_us)?;
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
                CLAIM_CREATED_EVENT_KIND,
                now_us,
                event_payload_json
            ],
        )
        .map_err(storage_error)?;

    transaction.commit().map_err(storage_error)?;

    let state = claim_snapshot(connection, claim_id)?;
    Ok(ClaimTaskResult {
        claim_id,
        session_id: options.session_id(),
        workspace_id: active.active_workspace_id,
        branch_id: active.active_branch_id,
        task_entity_id: options.task_entity_id(),
        mode: options.mode(),
        claimed_at_us: now_us,
        state,
    })
}

pub(crate) fn claim_snapshot(
    connection: &StoreConnection,
    claim_id: ClaimId,
) -> Result<ClaimSnapshot> {
    connection.verify_foreign_keys()?;
    let transaction = connection
        .inner()
        .unchecked_transaction()
        .map_err(storage_error)?;
    let snapshot = claim_snapshot_from_connection(&transaction, claim_id)?;
    transaction.commit().map_err(storage_error)?;
    Ok(snapshot)
}

pub(crate) fn active_claims_for_session(
    connection: &StoreConnection,
    options: &ClaimListOptions,
) -> Result<ClaimListResult> {
    connection.verify_foreign_keys()?;
    session::session_snapshot(connection, options.session_id())?;
    let transaction = connection
        .inner()
        .unchecked_transaction()
        .map_err(storage_error)?;
    let claim_ids = load_active_claims_for_session(&transaction, options.session_id())?
        .into_iter()
        .map(|(claim_id, _)| claim_id)
        .collect::<Vec<_>>();
    let mut claims = Vec::with_capacity(claim_ids.len());
    for claim_id in claim_ids {
        claims.push(claim_snapshot_from_connection(&transaction, claim_id)?);
    }
    transaction.commit().map_err(storage_error)?;
    Ok(ClaimListResult {
        session_id: options.session_id(),
        claims,
    })
}

pub(crate) fn task_claim_guard(
    connection: &StoreConnection,
    options: &ClaimGuardOptions,
) -> Result<ClaimGuardResult> {
    connection.verify_foreign_keys()?;
    let active = session::active_session_projection(connection, options.session_id())?;
    let branch_head = history::branch_head(connection, active.active_branch_id)?;
    if branch_head.workspace_id != active.active_workspace_id {
        return Err(WorkVcsError::ClaimInvalid(format!(
            "session {} active branch {} belongs to workspace {}, not active workspace {}",
            options.session_id(),
            active.active_branch_id,
            branch_head.workspace_id,
            active.active_workspace_id
        )));
    }
    if branch_head.lifecycle_state != ACTIVE_BRANCH_LIFECYCLE_STATE {
        return Err(WorkVcsError::ClaimInvalid(format!(
            "session {} active branch {} has lifecycle state {:?}",
            options.session_id(),
            active.active_branch_id,
            branch_head.lifecycle_state
        )));
    }
    history::task_at(
        connection,
        branch_head.head_commit_id,
        options.task_entity_id(),
    )?;

    let transaction = connection
        .inner()
        .unchecked_transaction()
        .map_err(storage_error)?;
    let active_claims = load_active_claim_snapshots_for_task(
        &transaction,
        active.active_workspace_id,
        active.active_branch_id,
        options.task_entity_id(),
    )?;
    let reason = protected_task_action_guard_reason(
        options.session_id(),
        active.active_branch_id,
        options.task_entity_id(),
        &active_claims,
    )?;
    transaction.commit().map_err(storage_error)?;

    Ok(ClaimGuardResult {
        session_id: options.session_id(),
        workspace_id: active.active_workspace_id,
        branch_id: active.active_branch_id,
        head_commit_id: branch_head.head_commit_id,
        task_entity_id: options.task_entity_id(),
        action: options.action(),
        allowed: reason.allows_protected_task_action(),
        reason,
        active_claims,
    })
}

pub(super) fn release_active_claims_for_session_end(
    transaction: &Transaction<'_>,
    session_id: SessionId,
    occurred_at_us: i64,
) -> Result<usize> {
    let active_claims = load_active_claims_for_session(transaction, session_id)?;
    for (claim_id, claim) in &active_claims {
        release_claim_runtime_for_row(transaction, *claim_id, claim, occurred_at_us)?;
    }
    Ok(active_claims.len())
}

pub(super) fn release_active_claims_for_session_branch(
    transaction: &Transaction<'_>,
    session_id: SessionId,
    workspace_id: WorkspaceId,
    branch_id: BranchId,
    occurred_at_us: i64,
) -> Result<usize> {
    let active_claims = load_active_claims_for_session(transaction, session_id)?;
    let mut released = 0;
    for (claim_id, claim) in &active_claims {
        if claim.workspace_id == workspace_id && claim.branch_id == branch_id {
            release_claim_runtime_for_row(transaction, *claim_id, claim, occurred_at_us)?;
            released += 1;
        }
    }
    Ok(released)
}

fn claim_snapshot_from_connection(
    connection: &Connection,
    claim_id: ClaimId,
) -> Result<ClaimSnapshot> {
    let claim = load_claim(connection, claim_id)?;
    let last_activity_at_us = load_claim_runtime(connection, claim_id)?;
    Ok(ClaimSnapshot {
        claim_id,
        lifecycle_state: if last_activity_at_us.is_some() {
            ClaimLifecycleState::Active
        } else {
            ClaimLifecycleState::Released
        },
        session_id: claim.session_id,
        workspace_id: claim.workspace_id,
        branch_id: claim.branch_id,
        task_entity_id: claim.task_entity_id,
        mode: claim.mode,
        created_at_us: claim.created_at_us,
        last_activity_at_us,
    })
}

struct BranchRuntimeRow {
    workspace_id: WorkspaceId,
    head_commit_id: CommitId,
}

struct ClaimRow {
    session_id: SessionId,
    workspace_id: WorkspaceId,
    branch_id: BranchId,
    task_entity_id: EntityId,
    mode: ClaimMode,
    created_at_us: i64,
}

fn load_active_claims_for_session(
    transaction: &Transaction<'_>,
    session_id: SessionId,
) -> Result<Vec<(ClaimId, ClaimRow)>> {
    let mut statement = transaction
        .prepare(
            "SELECT claim.claim_id,
                    claim.session_id,
                    claim.workspace_id,
                    claim.branch_id,
                    claim.task_entity_id,
                    claim.mode,
                    claim.created_at_us
             FROM claim_runtime
             INNER JOIN claim ON claim.claim_id = claim_runtime.claim_id
             WHERE claim.session_id = ?1
             ORDER BY claim.claim_id",
        )
        .map_err(storage_error)?;
    let rows = statement
        .query_map(params![&session_id.raw_bytes()[..]], |row| {
            Ok((
                row.get::<_, Vec<u8>>(0)?,
                row.get::<_, Vec<u8>>(1)?,
                row.get::<_, Vec<u8>>(2)?,
                row.get::<_, Vec<u8>>(3)?,
                row.get::<_, Vec<u8>>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, i64>(6)?,
            ))
        })
        .map_err(storage_error)?;

    let mut claims = Vec::new();
    for row in rows {
        let (claim_id, session_id, workspace_id, branch_id, task_entity_id, mode, created_at_us) =
            row.map_err(storage_error)?;
        let claim_id = decode_claim_id("claim.claim_id", claim_id)?;
        claims.push((
            claim_id,
            ClaimRow {
                session_id: decode_session_id("claim.session_id", session_id)?,
                workspace_id: decode_workspace_id("claim.workspace_id", workspace_id)?,
                branch_id: decode_branch_id("claim.branch_id", branch_id)?,
                task_entity_id: decode_entity_id("claim.task_entity_id", task_entity_id)?,
                mode: decode_claim_mode(&mode)?,
                created_at_us,
            },
        ));
    }
    Ok(claims)
}

fn load_active_claim_snapshots_for_task(
    connection: &Connection,
    workspace_id: WorkspaceId,
    branch_id: BranchId,
    task_entity_id: EntityId,
) -> Result<Vec<ClaimSnapshot>> {
    let mut statement = connection
        .prepare(
            "SELECT claim.claim_id
             FROM claim_runtime
             INNER JOIN claim ON claim.claim_id = claim_runtime.claim_id
             WHERE claim.workspace_id = ?1
               AND claim.branch_id = ?2
               AND claim.task_entity_id = ?3
             ORDER BY claim.claim_id",
        )
        .map_err(storage_error)?;
    let rows = statement
        .query_map(
            params![
                &workspace_id.raw_bytes()[..],
                &branch_id.raw_bytes()[..],
                &task_entity_id.raw_bytes()[..]
            ],
            |row| row.get::<_, Vec<u8>>(0),
        )
        .map_err(storage_error)?;

    let mut claims = Vec::new();
    for row in rows {
        let claim_id = decode_claim_id("claim.claim_id", row.map_err(storage_error)?)?;
        claims.push(claim_snapshot_from_connection(connection, claim_id)?);
    }
    Ok(claims)
}

fn protected_task_action_guard_reason(
    session_id: SessionId,
    branch_id: BranchId,
    task_entity_id: EntityId,
    active_claims: &[ClaimSnapshot],
) -> Result<ClaimGuardReason> {
    let mut exclusive_claim = None;
    let mut shared_claims = Vec::new();
    let mut shared_session_ids = BTreeSet::new();
    for claim in active_claims {
        if claim.lifecycle_state != ClaimLifecycleState::Active {
            return Err(WorkVcsError::ClaimInvalid(format!(
                "claim {} for task {task_entity_id} on branch {branch_id} is not active",
                claim.claim_id
            )));
        }
        match claim.mode {
            ClaimMode::Exclusive => {
                if exclusive_claim.replace(claim).is_some() {
                    return Err(WorkVcsError::ClaimInvalid(format!(
                        "task {task_entity_id} has more than one active exclusive claim on branch {branch_id}"
                    )));
                }
            }
            ClaimMode::Shared => {
                if !shared_session_ids.insert(claim.session_id) {
                    return Err(WorkVcsError::ClaimInvalid(format!(
                        "task {task_entity_id} has multiple active shared claims for session {} on branch {branch_id}",
                        claim.session_id
                    )));
                }
                shared_claims.push(claim);
            }
        }
    }

    if let Some(exclusive_claim) = exclusive_claim {
        if !shared_claims.is_empty() {
            return Err(WorkVcsError::ClaimInvalid(format!(
                "task {task_entity_id} has mixed active exclusive and shared claims on branch {branch_id}"
            )));
        }
        return Ok(if exclusive_claim.session_id == session_id {
            ClaimGuardReason::OwnedExclusiveClaim
        } else {
            ClaimGuardReason::ExclusiveClaimOwnedByOtherSession
        });
    }

    if shared_claims.is_empty() {
        return Ok(ClaimGuardReason::Unclaimed);
    }
    if !shared_claims
        .iter()
        .any(|claim| claim.session_id == session_id)
    {
        return Ok(ClaimGuardReason::SharedClaimSetDoesNotIncludeSession);
    }
    if shared_claims.len() == 1 {
        Ok(ClaimGuardReason::UniqueSharedClaimant)
    } else {
        Ok(ClaimGuardReason::NonUniqueSharedClaimSet)
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
    if lifecycle_state != ACTIVE_BRANCH_LIFECYCLE_STATE {
        return Err(WorkVcsError::ClaimInvalid(format!(
            "branch {branch_id} has lifecycle state {lifecycle_state:?}"
        )));
    }

    Ok(BranchRuntimeRow {
        workspace_id: decode_workspace_id("branch.workspace_id", workspace_id)?,
        head_commit_id: decode_commit_id("branch.head_commit_id", head_commit_id)?,
    })
}

fn ensure_active_claim_mode_allowed(
    transaction: &Transaction<'_>,
    workspace_id: WorkspaceId,
    branch_id: BranchId,
    task_entity_id: EntityId,
    session_id: SessionId,
    requested_mode: ClaimMode,
) -> Result<()> {
    let mut statement = transaction
        .prepare(
            "SELECT claim.claim_id, claim.session_id, claim.mode
             FROM claim_runtime
             INNER JOIN claim ON claim.claim_id = claim_runtime.claim_id
             WHERE claim.workspace_id = ?1
               AND claim.branch_id = ?2
               AND claim.task_entity_id = ?3
             ORDER BY claim.claim_id",
        )
        .map_err(storage_error)?;
    let rows = statement
        .query_map(
            params![
                &workspace_id.raw_bytes()[..],
                &branch_id.raw_bytes()[..],
                &task_entity_id.raw_bytes()[..]
            ],
            |row| {
                Ok((
                    row.get::<_, Vec<u8>>(0)?,
                    row.get::<_, Vec<u8>>(1)?,
                    row.get::<_, String>(2)?,
                ))
            },
        )
        .map_err(storage_error)?;

    let mut active_claims = Vec::new();
    for row in rows {
        let (claim_id, claim_session_id, mode) = row.map_err(storage_error)?;
        active_claims.push((
            decode_claim_id("claim.claim_id", claim_id)?,
            decode_session_id("claim.session_id", claim_session_id)?,
            decode_claim_mode(&mode)?,
        ));
    }

    match requested_mode {
        ClaimMode::Exclusive => {
            if let Some((claim_id, _, mode)) = active_claims.first() {
                return Err(WorkVcsError::ClaimInvalid(format!(
                    "task {task_entity_id} already has active {} claim {claim_id} on branch {branch_id}",
                    mode.as_str()
                )));
            }
        }
        ClaimMode::Shared => {
            for (claim_id, claim_session_id, mode) in &active_claims {
                match mode {
                    ClaimMode::Exclusive => {
                        return Err(WorkVcsError::ClaimInvalid(format!(
                            "task {task_entity_id} already has active exclusive claim {claim_id} on branch {branch_id}"
                        )));
                    }
                    ClaimMode::Shared if *claim_session_id == session_id => {
                        return Err(WorkVcsError::ClaimInvalid(format!(
                            "session {session_id} already has active shared claim {claim_id} for task {task_entity_id} on branch {branch_id}"
                        )));
                    }
                    ClaimMode::Shared => {}
                }
            }
        }
    }
    Ok(())
}

fn load_claim(connection: &Connection, claim_id: ClaimId) -> Result<ClaimRow> {
    let row = connection
        .query_row(
            "SELECT session_id,
                    workspace_id,
                    branch_id,
                    task_entity_id,
                    mode,
                    created_at_us
             FROM claim
             WHERE claim_id = ?1",
            params![&claim_id.raw_bytes()[..]],
            |row| {
                Ok((
                    row.get::<_, Vec<u8>>(0)?,
                    row.get::<_, Vec<u8>>(1)?,
                    row.get::<_, Vec<u8>>(2)?,
                    row.get::<_, Vec<u8>>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, i64>(5)?,
                ))
            },
        )
        .optional()
        .map_err(storage_error)?;

    let Some((session_id, workspace_id, branch_id, task_entity_id, mode, created_at_us)) = row
    else {
        return Err(WorkVcsError::ClaimNotFound(format!(
            "claim {claim_id} does not exist"
        )));
    };

    Ok(ClaimRow {
        session_id: decode_session_id("claim.session_id", session_id)?,
        workspace_id: decode_workspace_id("claim.workspace_id", workspace_id)?,
        branch_id: decode_branch_id("claim.branch_id", branch_id)?,
        task_entity_id: decode_entity_id("claim.task_entity_id", task_entity_id)?,
        mode: decode_claim_mode(&mode)?,
        created_at_us,
    })
}

fn load_claim_runtime(connection: &Connection, claim_id: ClaimId) -> Result<Option<i64>> {
    connection
        .query_row(
            "SELECT last_activity_at_us
             FROM claim_runtime
             WHERE claim_id = ?1",
            params![&claim_id.raw_bytes()[..]],
            |row| row.get::<_, i64>(0),
        )
        .optional()
        .map_err(storage_error)
}

fn ensure_claim_runtime_exists(connection: &Connection, claim_id: ClaimId) -> Result<()> {
    if load_claim_runtime(connection, claim_id)?.is_some() {
        Ok(())
    } else {
        Err(WorkVcsError::ClaimInvalid(format!(
            "claim {claim_id} is not active"
        )))
    }
}

fn require_active_claim_runtime(connection: &Connection, claim_id: ClaimId) -> Result<i64> {
    load_claim_runtime(connection, claim_id)?
        .ok_or_else(|| WorkVcsError::ClaimInvalid(format!("claim {claim_id} is not active")))
}

fn ensure_claim_session_target(
    label: &str,
    session_id: SessionId,
    active: &session::ActiveSessionProjection,
    claim_id: ClaimId,
    claim: &ClaimRow,
) -> Result<()> {
    if active.active_workspace_id != claim.workspace_id
        || active.active_branch_id != claim.branch_id
    {
        return Err(WorkVcsError::ClaimInvalid(format!(
            "{label} session {session_id} active target {}/{} does not match claim {} target {}/{}",
            active.active_workspace_id,
            active.active_branch_id,
            claim_id,
            claim.workspace_id,
            claim.branch_id
        )));
    }
    Ok(())
}

fn delete_claim_runtime_row(transaction: &Transaction<'_>, claim_id: ClaimId) -> Result<()> {
    let deleted = transaction
        .execute(
            "DELETE FROM claim_runtime
             WHERE claim_id = ?1",
            params![&claim_id.raw_bytes()[..]],
        )
        .map_err(storage_error)?;
    if deleted != 1 {
        return Err(WorkVcsError::ClaimInvalid(format!(
            "claim {claim_id} runtime replacement affected {deleted} rows"
        )));
    }
    Ok(())
}

fn insert_claim_occurrence(
    transaction: &Transaction<'_>,
    claim_id: ClaimId,
    session_id: SessionId,
    claim: &ClaimRow,
    occurred_at_us: i64,
) -> Result<()> {
    transaction
        .execute(
            "INSERT INTO object_identity(object_id, object_kind, created_at_us)
             VALUES (?1, ?2, ?3)",
            params![&claim_id.raw_bytes()[..], CLAIM_OBJECT_KIND, occurred_at_us],
        )
        .map_err(storage_error)?;
    transaction
        .execute(
            "INSERT INTO claim(
                claim_id,
                session_id,
                workspace_id,
                branch_id,
                task_entity_id,
                mode,
                created_at_us
             )
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                &claim_id.raw_bytes()[..],
                &session_id.raw_bytes()[..],
                &claim.workspace_id.raw_bytes()[..],
                &claim.branch_id.raw_bytes()[..],
                &claim.task_entity_id.raw_bytes()[..],
                claim.mode.as_str(),
                occurred_at_us
            ],
        )
        .map_err(storage_error)?;
    transaction
        .execute(
            "INSERT INTO claim_runtime(claim_id, last_activity_at_us)
             VALUES (?1, ?2)",
            params![&claim_id.raw_bytes()[..], occurred_at_us],
        )
        .map_err(storage_error)?;
    Ok(())
}

fn insert_claim_replacement_event(
    transaction: &Transaction<'_>,
    session_id: SessionId,
    workspace_id: WorkspaceId,
    event_kind: &str,
    occurred_at_us: i64,
    payload_json: String,
) -> Result<()> {
    let event_id = EventId::new_v7();
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
                &event_id.raw_bytes()[..],
                &workspace_id.raw_bytes()[..],
                &session_id.raw_bytes()[..],
                event_kind,
                occurred_at_us,
                payload_json
            ],
        )
        .map_err(storage_error)?;
    Ok(())
}

fn release_claim_runtime_for_row(
    transaction: &Transaction<'_>,
    claim_id: ClaimId,
    claim: &ClaimRow,
    occurred_at_us: i64,
) -> Result<()> {
    let claim_id_bytes = claim_id.raw_bytes();
    let deleted = transaction
        .execute(
            "DELETE FROM claim_runtime
             WHERE claim_id = ?1",
            params![&claim_id_bytes[..]],
        )
        .map_err(storage_error)?;
    if deleted != 1 {
        return Err(WorkVcsError::ClaimInvalid(format!(
            "claim {claim_id} runtime cleanup affected {deleted} rows"
        )));
    }
    let event_id = EventId::new_v7();
    let event_id_bytes = event_id.raw_bytes();
    let workspace_id_bytes = claim.workspace_id.raw_bytes();
    let session_id_bytes = claim.session_id.raw_bytes();
    let event_payload_json = claim_released_payload_json(claim_id, claim)?;
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
                CLAIM_RELEASED_EVENT_KIND,
                occurred_at_us,
                event_payload_json
            ],
        )
        .map_err(storage_error)?;
    Ok(())
}

fn claim_created_payload_json(
    claim_id: ClaimId,
    session_id: SessionId,
    workspace_id: WorkspaceId,
    branch_id: BranchId,
    task_entity_id: EntityId,
    mode: ClaimMode,
) -> Result<String> {
    canonical_json_string(&CanonicalValue::object(vec![
        (
            "branch_id".to_owned(),
            CanonicalValue::String(branch_id.to_string()),
        ),
        (
            "claim_id".to_owned(),
            CanonicalValue::String(claim_id.to_string()),
        ),
        (
            "lifecycle_state".to_owned(),
            CanonicalValue::String(ACTIVE_CLAIM_LIFECYCLE_STATE.to_owned()),
        ),
        (
            "mode".to_owned(),
            CanonicalValue::String(mode.as_str().to_owned()),
        ),
        (
            "session_id".to_owned(),
            CanonicalValue::String(session_id.to_string()),
        ),
        (
            "task_entity_id".to_owned(),
            CanonicalValue::String(task_entity_id.to_string()),
        ),
        (
            "workspace_id".to_owned(),
            CanonicalValue::String(workspace_id.to_string()),
        ),
    ])?)
}

fn claim_released_payload_json(claim_id: ClaimId, claim: &ClaimRow) -> Result<String> {
    canonical_json_string(&CanonicalValue::object(vec![
        (
            "branch_id".to_owned(),
            CanonicalValue::String(claim.branch_id.to_string()),
        ),
        (
            "claim_id".to_owned(),
            CanonicalValue::String(claim_id.to_string()),
        ),
        (
            "lifecycle_state".to_owned(),
            CanonicalValue::String(RELEASED_CLAIM_LIFECYCLE_STATE.to_owned()),
        ),
        (
            "mode".to_owned(),
            CanonicalValue::String(claim.mode.as_str().to_owned()),
        ),
        (
            "session_id".to_owned(),
            CanonicalValue::String(claim.session_id.to_string()),
        ),
        (
            "task_entity_id".to_owned(),
            CanonicalValue::String(claim.task_entity_id.to_string()),
        ),
        (
            "workspace_id".to_owned(),
            CanonicalValue::String(claim.workspace_id.to_string()),
        ),
    ])?)
}

fn claim_transferred_payload_json(
    previous_claim_id: ClaimId,
    claim_id: ClaimId,
    previous_claim: &ClaimRow,
    previous_last_activity_at_us: i64,
    to_session_id: SessionId,
) -> Result<String> {
    canonical_json_string(&CanonicalValue::object(vec![
        (
            "branch_id".to_owned(),
            CanonicalValue::String(previous_claim.branch_id.to_string()),
        ),
        (
            "claim_id".to_owned(),
            CanonicalValue::String(claim_id.to_string()),
        ),
        (
            "from_session_id".to_owned(),
            CanonicalValue::String(previous_claim.session_id.to_string()),
        ),
        (
            "lifecycle_state".to_owned(),
            CanonicalValue::String(ACTIVE_CLAIM_LIFECYCLE_STATE.to_owned()),
        ),
        (
            "mode".to_owned(),
            CanonicalValue::String(previous_claim.mode.as_str().to_owned()),
        ),
        (
            "previous_claim_id".to_owned(),
            CanonicalValue::String(previous_claim_id.to_string()),
        ),
        (
            "previous_last_activity_at_us".to_owned(),
            CanonicalValue::safe_integer(previous_last_activity_at_us)?,
        ),
        (
            "reason".to_owned(),
            CanonicalValue::String("transfer".to_owned()),
        ),
        (
            "task_entity_id".to_owned(),
            CanonicalValue::String(previous_claim.task_entity_id.to_string()),
        ),
        (
            "to_session_id".to_owned(),
            CanonicalValue::String(to_session_id.to_string()),
        ),
        (
            "workspace_id".to_owned(),
            CanonicalValue::String(previous_claim.workspace_id.to_string()),
        ),
    ])?)
}

fn claim_force_taken_over_payload_json(
    previous_claim_id: ClaimId,
    claim_id: ClaimId,
    previous_claim: &ClaimRow,
    previous_last_activity_at_us: i64,
    previous_session_lifecycle_state: &str,
    session_id: SessionId,
    rationale: &str,
) -> Result<String> {
    canonical_json_string(&CanonicalValue::object(vec![
        (
            "branch_id".to_owned(),
            CanonicalValue::String(previous_claim.branch_id.to_string()),
        ),
        (
            "claim_id".to_owned(),
            CanonicalValue::String(claim_id.to_string()),
        ),
        (
            "lifecycle_state".to_owned(),
            CanonicalValue::String(ACTIVE_CLAIM_LIFECYCLE_STATE.to_owned()),
        ),
        (
            "mode".to_owned(),
            CanonicalValue::String(previous_claim.mode.as_str().to_owned()),
        ),
        (
            "previous_claim_id".to_owned(),
            CanonicalValue::String(previous_claim_id.to_string()),
        ),
        (
            "previous_last_activity_at_us".to_owned(),
            CanonicalValue::safe_integer(previous_last_activity_at_us)?,
        ),
        (
            "previous_session_id".to_owned(),
            CanonicalValue::String(previous_claim.session_id.to_string()),
        ),
        (
            "previous_session_lifecycle_state".to_owned(),
            CanonicalValue::String(previous_session_lifecycle_state.to_owned()),
        ),
        (
            "rationale".to_owned(),
            CanonicalValue::String(rationale.to_owned()),
        ),
        (
            "reason".to_owned(),
            CanonicalValue::String("force".to_owned()),
        ),
        (
            "session_id".to_owned(),
            CanonicalValue::String(session_id.to_string()),
        ),
        (
            "task_entity_id".to_owned(),
            CanonicalValue::String(previous_claim.task_entity_id.to_string()),
        ),
        (
            "workspace_id".to_owned(),
            CanonicalValue::String(previous_claim.workspace_id.to_string()),
        ),
    ])?)
}

fn session_lifecycle_state_label(lifecycle_state: SessionLifecycleState) -> &'static str {
    match lifecycle_state {
        SessionLifecycleState::Active => "active",
        SessionLifecycleState::PotentiallyStale => "potentially_stale",
        SessionLifecycleState::Ended => "ended",
    }
}

fn canonical_json_string(value: &CanonicalValue) -> Result<String> {
    String::from_utf8(canonical_bytes(value)?).map_err(|error| {
        WorkVcsError::CanonicalEncodingInvalid(format!("canonical JSON was not UTF-8: {error}"))
    })
}

fn decode_claim_mode(value: &str) -> Result<ClaimMode> {
    match value {
        EXCLUSIVE_CLAIM_MODE => Ok(ClaimMode::Exclusive),
        SHARED_CLAIM_MODE => Ok(ClaimMode::Shared),
        other => Err(WorkVcsError::ClaimInvalid(format!(
            "claim mode {other:?} is not supported"
        ))),
    }
}

fn decode_session_id(column: &str, bytes: Vec<u8>) -> Result<SessionId> {
    let bytes = decode_16(column, bytes)?;
    SessionId::from_bytes(bytes).map_err(|error| {
        WorkVcsError::ClaimInvalid(format!("{column} is not a UUIDv7 value: {error}"))
    })
}

fn decode_workspace_id(column: &str, bytes: Vec<u8>) -> Result<WorkspaceId> {
    let bytes = decode_16(column, bytes)?;
    WorkspaceId::from_bytes(bytes).map_err(|error| {
        WorkVcsError::ClaimInvalid(format!("{column} is not a UUIDv7 value: {error}"))
    })
}

fn decode_branch_id(column: &str, bytes: Vec<u8>) -> Result<BranchId> {
    let bytes = decode_16(column, bytes)?;
    BranchId::from_bytes(bytes).map_err(|error| {
        WorkVcsError::ClaimInvalid(format!("{column} is not a UUIDv7 value: {error}"))
    })
}

fn decode_commit_id(column: &str, bytes: Vec<u8>) -> Result<CommitId> {
    let bytes = decode_16(column, bytes)?;
    CommitId::from_bytes(bytes).map_err(|error| {
        WorkVcsError::ClaimInvalid(format!("{column} is not a UUIDv7 value: {error}"))
    })
}

fn decode_claim_id(column: &str, bytes: Vec<u8>) -> Result<ClaimId> {
    let bytes = decode_16(column, bytes)?;
    ClaimId::from_bytes(bytes).map_err(|error| {
        WorkVcsError::ClaimInvalid(format!("{column} is not a UUIDv7 value: {error}"))
    })
}

fn decode_entity_id(column: &str, bytes: Vec<u8>) -> Result<EntityId> {
    let bytes = decode_16(column, bytes)?;
    EntityId::from_bytes(bytes).map_err(|error| {
        WorkVcsError::ClaimInvalid(format!("{column} is not a UUIDv7 value: {error}"))
    })
}

fn decode_16(column: &str, bytes: Vec<u8>) -> Result<[u8; 16]> {
    bytes.try_into().map_err(|bytes: Vec<u8>| {
        WorkVcsError::ClaimInvalid(format!("{column} must be 16 bytes, found {}", bytes.len()))
    })
}
