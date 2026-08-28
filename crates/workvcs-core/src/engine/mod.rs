use crate::BranchId;
use crate::ClaimId;
use crate::CommitId;
use crate::EntityId;
use crate::SessionId;
use crate::WorkspaceId;
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
use crate::store::{Store, StoreInfo, StoreInitOptions};
use crate::{
    ClaimReleaseOptions, ClaimReleaseResult, ClaimSnapshot, ClaimTaskOptions, ClaimTaskResult,
    RunnableTasksOptions, RunnableTasksProjection, SessionEndOptions, SessionEndResult,
    SessionFocusOptions, SessionFocusUpdateResult, SessionSnapshot, SessionStartOptions,
    SessionStartResult,
};
use std::path::Path;

pub struct Engine {
    store: Store,
}

impl Engine {
    pub fn init(path: impl AsRef<Path>, options: StoreInitOptions) -> Result<Self> {
        Ok(Self {
            store: Store::init(path.as_ref(), options)?,
        })
    }

    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        Ok(Self {
            store: Store::open(path.as_ref())?,
        })
    }

    pub fn store_info(&self) -> Result<StoreInfo> {
        self.store.info()
    }

    pub fn create_workspace(&mut self, options: WorkspaceInitOptions) -> Result<WorkspaceInfo> {
        self.store.create_workspace(&options)
    }

    pub fn workspace_info(&self, workspace_id: WorkspaceId) -> Result<WorkspaceInfo> {
        self.store.workspace_info(workspace_id)
    }

    pub fn state_at(&self, commit_id: CommitId) -> Result<ReplayedState> {
        self.store.state_at(commit_id)
    }

    pub fn show_at(&self, commit_id: CommitId) -> Result<ReplayedState> {
        self.store.show_at(commit_id)
    }

    pub fn branch_head(&self, branch_id: BranchId) -> Result<BranchHead> {
        self.store.branch_head(branch_id)
    }

    pub fn history(&self, options: HistoryQueryOptions) -> Result<HistoryQueryResult> {
        self.store.history(&options)
    }

    pub fn validate_integrity(&self) -> Result<IntegrityReport> {
        self.store.validate_integrity()
    }

    pub fn commit_entity_transition(
        &mut self,
        options: EntityTransitionOptions,
    ) -> Result<EntityTransitionCommit> {
        self.store.commit_entity_transition(&options)
    }

    pub fn create_task(&mut self, options: TaskCreateOptions) -> Result<TaskCreateCommit> {
        self.store.create_task(&options)
    }

    pub fn transition_task(
        &mut self,
        options: TaskTransitionOptions,
    ) -> Result<TaskTransitionCommit> {
        self.store.transition_task(&options)
    }

    pub fn create_task_scheduling_relation(
        &mut self,
        options: TaskSchedulingRelationCreateOptions,
    ) -> Result<TaskSchedulingRelationCreateCommit> {
        self.store.create_task_scheduling_relation(&options)
    }

    pub fn create_acceptance_criterion(
        &mut self,
        options: AcceptanceCriterionCreateOptions,
    ) -> Result<AcceptanceCriterionCreateCommit> {
        self.store.create_acceptance_criterion(&options)
    }

    pub fn revise_acceptance_criterion(
        &mut self,
        options: AcceptanceCriterionRevisionOptions,
    ) -> Result<AcceptanceCriterionRevisionCommit> {
        self.store.revise_acceptance_criterion(&options)
    }

    pub fn create_verification_requirement(
        &mut self,
        options: VerificationRequirementCreateOptions,
    ) -> Result<VerificationRequirementCreateCommit> {
        self.store.create_verification_requirement(&options)
    }

    pub fn revise_verification_requirement(
        &mut self,
        options: VerificationRequirementRevisionOptions,
    ) -> Result<VerificationRequirementRevisionCommit> {
        self.store.revise_verification_requirement(&options)
    }

    pub fn create_verification(
        &mut self,
        options: VerificationCreateOptions,
    ) -> Result<VerificationCreateCommit> {
        self.store.create_verification(&options)
    }

    pub fn task_at(&self, commit_id: CommitId, task_entity_id: EntityId) -> Result<TaskSnapshot> {
        self.store.task_at(commit_id, task_entity_id)
    }

    pub fn acceptance_criterion_at(
        &self,
        commit_id: CommitId,
        acceptance_criterion_entity_id: EntityId,
    ) -> Result<AcceptanceCriterionSnapshot> {
        self.store
            .acceptance_criterion_at(commit_id, acceptance_criterion_entity_id)
    }

    pub fn verification_requirement_at(
        &self,
        commit_id: CommitId,
        verification_requirement_entity_id: EntityId,
    ) -> Result<VerificationRequirementSnapshot> {
        self.store
            .verification_requirement_at(commit_id, verification_requirement_entity_id)
    }

    pub fn verification_at(
        &self,
        commit_id: CommitId,
        verification_entity_id: EntityId,
    ) -> Result<VerificationSnapshot> {
        self.store
            .verification_at(commit_id, verification_entity_id)
    }

    pub fn task_scheduling_relations_at(
        &self,
        commit_id: CommitId,
    ) -> Result<Vec<TaskSchedulingRelationSnapshot>> {
        self.store.task_scheduling_relations_at(commit_id)
    }

    pub fn acceptance_criterion_effective_status(
        &self,
        commit_id: CommitId,
        acceptance_criterion_entity_id: EntityId,
    ) -> Result<AcceptanceCriterionEffectiveStatus> {
        self.store
            .acceptance_criterion_effective_status(commit_id, acceptance_criterion_entity_id)
    }

    pub fn start_session(&mut self, options: SessionStartOptions) -> Result<SessionStartResult> {
        self.store.start_session(&options)
    }

    pub fn session_snapshot(&self, session_id: SessionId) -> Result<SessionSnapshot> {
        self.store.session_snapshot(session_id)
    }

    pub fn set_session_focus(
        &mut self,
        options: SessionFocusOptions,
    ) -> Result<SessionFocusUpdateResult> {
        self.store.set_session_focus(&options)
    }

    pub fn clear_session_focus(
        &mut self,
        session_id: SessionId,
    ) -> Result<SessionFocusUpdateResult> {
        self.store.clear_session_focus(session_id)
    }

    pub fn end_session(&mut self, options: SessionEndOptions) -> Result<SessionEndResult> {
        self.store.end_session(&options)
    }

    pub fn claim_task(&mut self, options: ClaimTaskOptions) -> Result<ClaimTaskResult> {
        self.store.claim_task(&options)
    }

    pub fn claim_snapshot(&self, claim_id: ClaimId) -> Result<ClaimSnapshot> {
        self.store.claim_snapshot(claim_id)
    }

    pub fn release_claim(&mut self, options: ClaimReleaseOptions) -> Result<ClaimReleaseResult> {
        self.store.release_claim(&options)
    }

    pub fn runnable_tasks(&self, options: RunnableTasksOptions) -> Result<RunnableTasksProjection> {
        self.store.runnable_tasks(&options)
    }
}
