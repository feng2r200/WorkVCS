use crate::error::Result;
use crate::history::{
    AcceptanceCriterionCreateCommit, AcceptanceCriterionCreateOptions,
    AcceptanceCriterionEffectiveStatus, AcceptanceCriterionRevisionCommit,
    AcceptanceCriterionRevisionOptions, AcceptanceCriterionSnapshot, BranchHead,
    EntityTransitionCommit, EntityTransitionOptions, HistoryQueryOptions, HistoryQueryResult,
    IntegrityReport, ReplayedState, TaskCreateCommit, TaskCreateOptions,
    TaskSchedulingRelationCreateCommit, TaskSchedulingRelationCreateOptions,
    TaskSchedulingRelationSnapshot, TaskSnapshot, TaskTransitionCommit, TaskTransitionOptions,
    VerificationCreateCommit, VerificationCreateOptions, VerificationRequirementCreateCommit,
    VerificationRequirementCreateOptions, VerificationRequirementRevisionCommit,
    VerificationRequirementRevisionOptions, VerificationRequirementSnapshot, VerificationSnapshot,
    WorkspaceInfo, WorkspaceInitOptions,
};
use crate::runtime::{
    ClaimReleaseOptions, ClaimReleaseResult, ClaimSnapshot, ClaimTaskOptions, ClaimTaskResult,
    RunnableTasksOptions, RunnableTasksProjection, SessionEndOptions, SessionEndResult,
    SessionFocusOptions, SessionFocusUpdateResult, SessionSnapshot, SessionStartOptions,
    SessionStartResult,
};
use crate::store::bootstrap::{
    StoreInfo, StoreInitOptions, ensure_empty_database, initialize_manifest, validate_bootstrap,
};
use crate::store::connection::StoreConnection;
use crate::store::schema;
use crate::{BranchId, ClaimId, CommitId, EntityId, SessionId, WorkspaceId, history, runtime};
use std::path::Path;

pub(crate) struct Store {
    connection: StoreConnection,
    info: StoreInfo,
}

impl Store {
    pub(crate) fn init(path: &Path, options: StoreInitOptions) -> Result<Self> {
        let mut connection = StoreConnection::open(path)?;
        ensure_empty_database(&connection)?;
        schema::install(&connection)?;
        let info = initialize_manifest(&mut connection, &options)?;
        let info = validate_bootstrap(&connection).map(|validated| {
            debug_assert_eq!(validated, info);
            validated
        })?;
        Ok(Self { connection, info })
    }

    pub(crate) fn open(path: &Path) -> Result<Self> {
        let connection = StoreConnection::open(path)?;
        let info = validate_bootstrap(&connection)?;
        Ok(Self { connection, info })
    }

    pub(crate) fn info(&self) -> Result<StoreInfo> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        Ok(current)
    }

    pub(crate) fn create_workspace(
        &mut self,
        options: &WorkspaceInitOptions,
    ) -> Result<WorkspaceInfo> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        history::create_workspace(&mut self.connection, current.store_id, options)
    }

    pub(crate) fn workspace_info(&self, workspace_id: WorkspaceId) -> Result<WorkspaceInfo> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        history::load_workspace_info(&self.connection, workspace_id)
    }

    pub(crate) fn state_at(&self, commit_id: CommitId) -> Result<ReplayedState> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        history::state_at(&self.connection, commit_id)
    }

    pub(crate) fn show_at(&self, commit_id: CommitId) -> Result<ReplayedState> {
        self.state_at(commit_id)
    }

    pub(crate) fn branch_head(&self, branch_id: BranchId) -> Result<BranchHead> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        history::branch_head(&self.connection, branch_id)
    }

    pub(crate) fn history(&self, options: &HistoryQueryOptions) -> Result<HistoryQueryResult> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        history::query_history(&self.connection, options)
    }

    pub(crate) fn validate_integrity(&self) -> Result<IntegrityReport> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        history::validate_integrity(&self.connection)
    }

    pub(crate) fn commit_entity_transition(
        &mut self,
        options: &EntityTransitionOptions,
    ) -> Result<EntityTransitionCommit> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        history::reject_reserved_semantic_entity_transition(&self.connection, options)?;
        history::commit_entity_transition(&mut self.connection, options)
    }

    pub(crate) fn create_task(&mut self, options: &TaskCreateOptions) -> Result<TaskCreateCommit> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        history::create_task(&mut self.connection, options)
    }

    pub(crate) fn transition_task(
        &mut self,
        options: &TaskTransitionOptions,
    ) -> Result<TaskTransitionCommit> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        history::transition_task(&mut self.connection, options)
    }

    pub(crate) fn create_task_scheduling_relation(
        &mut self,
        options: &TaskSchedulingRelationCreateOptions,
    ) -> Result<TaskSchedulingRelationCreateCommit> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        history::create_task_scheduling_relation(&mut self.connection, options)
    }

    pub(crate) fn create_acceptance_criterion(
        &mut self,
        options: &AcceptanceCriterionCreateOptions,
    ) -> Result<AcceptanceCriterionCreateCommit> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        history::create_acceptance_criterion(&mut self.connection, options)
    }

    pub(crate) fn revise_acceptance_criterion(
        &mut self,
        options: &AcceptanceCriterionRevisionOptions,
    ) -> Result<AcceptanceCriterionRevisionCommit> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        history::revise_acceptance_criterion(&mut self.connection, options)
    }

    pub(crate) fn create_verification_requirement(
        &mut self,
        options: &VerificationRequirementCreateOptions,
    ) -> Result<VerificationRequirementCreateCommit> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        history::create_verification_requirement(&mut self.connection, options)
    }

    pub(crate) fn revise_verification_requirement(
        &mut self,
        options: &VerificationRequirementRevisionOptions,
    ) -> Result<VerificationRequirementRevisionCommit> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        history::revise_verification_requirement(&mut self.connection, options)
    }

    pub(crate) fn create_verification(
        &mut self,
        options: &VerificationCreateOptions,
    ) -> Result<VerificationCreateCommit> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        history::create_verification(&mut self.connection, options)
    }

    pub(crate) fn task_at(
        &self,
        commit_id: CommitId,
        task_entity_id: EntityId,
    ) -> Result<TaskSnapshot> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        history::task_at(&self.connection, commit_id, task_entity_id)
    }

    pub(crate) fn acceptance_criterion_at(
        &self,
        commit_id: CommitId,
        acceptance_criterion_entity_id: EntityId,
    ) -> Result<AcceptanceCriterionSnapshot> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        history::acceptance_criterion_at(
            &self.connection,
            commit_id,
            acceptance_criterion_entity_id,
        )
    }

    pub(crate) fn verification_requirement_at(
        &self,
        commit_id: CommitId,
        verification_requirement_entity_id: EntityId,
    ) -> Result<VerificationRequirementSnapshot> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        history::verification_requirement_at(
            &self.connection,
            commit_id,
            verification_requirement_entity_id,
        )
    }

    pub(crate) fn verification_at(
        &self,
        commit_id: CommitId,
        verification_entity_id: EntityId,
    ) -> Result<VerificationSnapshot> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        history::verification_at(&self.connection, commit_id, verification_entity_id)
    }

    pub(crate) fn task_scheduling_relations_at(
        &self,
        commit_id: CommitId,
    ) -> Result<Vec<TaskSchedulingRelationSnapshot>> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        history::task_scheduling_relations_at(&self.connection, commit_id)
    }

    pub(crate) fn acceptance_criterion_effective_status(
        &self,
        commit_id: CommitId,
        acceptance_criterion_entity_id: EntityId,
    ) -> Result<AcceptanceCriterionEffectiveStatus> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        history::acceptance_criterion_effective_status(
            &self.connection,
            commit_id,
            acceptance_criterion_entity_id,
        )
    }

    pub(crate) fn start_session(
        &mut self,
        options: &SessionStartOptions,
    ) -> Result<SessionStartResult> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        runtime::start_session(&mut self.connection, options)
    }

    pub(crate) fn session_snapshot(&self, session_id: SessionId) -> Result<SessionSnapshot> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        runtime::session_snapshot(&self.connection, session_id)
    }

    pub(crate) fn set_session_focus(
        &mut self,
        options: &SessionFocusOptions,
    ) -> Result<SessionFocusUpdateResult> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        runtime::set_session_focus(&mut self.connection, options)
    }

    pub(crate) fn clear_session_focus(
        &mut self,
        session_id: SessionId,
    ) -> Result<SessionFocusUpdateResult> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        runtime::clear_session_focus(&mut self.connection, session_id)
    }

    pub(crate) fn end_session(&mut self, options: &SessionEndOptions) -> Result<SessionEndResult> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        runtime::end_session(&mut self.connection, options)
    }

    pub(crate) fn claim_task(&mut self, options: &ClaimTaskOptions) -> Result<ClaimTaskResult> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        runtime::claim_task(&mut self.connection, options)
    }

    pub(crate) fn claim_snapshot(&self, claim_id: ClaimId) -> Result<ClaimSnapshot> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        runtime::claim_snapshot(&self.connection, claim_id)
    }

    pub(crate) fn release_claim(
        &mut self,
        options: &ClaimReleaseOptions,
    ) -> Result<ClaimReleaseResult> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        runtime::release_claim(&mut self.connection, options)
    }

    pub(crate) fn runnable_tasks(
        &self,
        options: &RunnableTasksOptions,
    ) -> Result<RunnableTasksProjection> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        runtime::runnable_tasks(&self.connection, options)
    }
}
