use crate::error::Result;
use crate::history::{
    AcceptanceCriterionCreateCommit, AcceptanceCriterionCreateOptions,
    AcceptanceCriterionEffectiveStatus, AcceptanceCriterionRevisionCommit,
    AcceptanceCriterionRevisionOptions, AcceptanceCriterionSnapshot, BranchForkOptions,
    BranchForkResult, BranchHead, BranchProjectionRefreshOptions, BranchProjectionRefreshResult,
    BranchProjectionSnapshot, CheckpointCreateOptions, CheckpointCreateResult, CheckpointSnapshot,
    EntityTransitionCommit, EntityTransitionOptions, EvidenceCreateOptions, EvidenceCreateResult,
    EvidenceSnapshot, GoalCreateCommit, GoalCreateOptions, GoalSnapshot, GoalTransitionCommit,
    GoalTransitionOptions, HistoryQueryOptions, HistoryQueryResult, IntegrityReport,
    KnowledgeCreateCommit, KnowledgeCreateOptions, KnowledgeListOptions, KnowledgeListResult,
    KnowledgeRelationCreateCommit, KnowledgeRelationCreateOptions, KnowledgeRelationListOptions,
    KnowledgeRelationListResult, KnowledgeRelationRemoveCommit, KnowledgeRelationRemoveOptions,
    KnowledgeRelationRestoreCommit, KnowledgeRelationRestoreOptions, KnowledgeRelationSnapshot,
    KnowledgeSnapshot, KnowledgeTransitionCommit, KnowledgeTransitionOptions, PlanCreateCommit,
    PlanCreateOptions, PlanSnapshot, PlanTransitionCommit, PlanTransitionOptions,
    PrimaryContainmentCreateCommit, PrimaryContainmentCreateOptions, PrimaryContainmentSnapshot,
    RecordCreateCommit, RecordCreateOptions, RecordKnowledgeRelationCreateCommit,
    RecordKnowledgeRelationCreateOptions, RecordKnowledgeRelationListOptions,
    RecordKnowledgeRelationListResult, RecordKnowledgeRelationRemoveCommit,
    RecordKnowledgeRelationRemoveOptions, RecordKnowledgeRelationRestoreCommit,
    RecordKnowledgeRelationRestoreOptions, RecordKnowledgeRelationSnapshot, RecordListOptions,
    RecordListResult, RecordRelationCreateCommit, RecordRelationCreateOptions,
    RecordRelationListOptions, RecordRelationListResult, RecordRelationRemoveCommit,
    RecordRelationRemoveOptions, RecordRelationRestoreCommit, RecordRelationRestoreOptions,
    RecordRelationSnapshot, RecordSnapshot, RecordTransitionCommit, RecordTransitionOptions,
    ReplayedState, ResourceBindOptions, ResourceBindResult, ResourceCreateOptions,
    ResourceCreateResult, ResourceObservationCreateOptions, ResourceObservationCreateResult,
    ResourceObservationSnapshot, ResourceSnapshot, StructuralReferenceCreateCommit,
    StructuralReferenceCreateOptions, StructuralReferenceSnapshot, TaskCreateCommit,
    TaskCreateOptions, TaskSchedulingRelationCreateCommit, TaskSchedulingRelationCreateOptions,
    TaskSchedulingRelationSnapshot, TaskSnapshot, TaskTransitionCommit, TaskTransitionOptions,
    VerificationApplicabilityCacheSnapshot, VerificationApplicabilityRecordOptions,
    VerificationCreateCommit, VerificationCreateOptions, VerificationRequirementCreateCommit,
    VerificationRequirementCreateOptions, VerificationRequirementRevisionCommit,
    VerificationRequirementRevisionOptions, VerificationRequirementSnapshot, VerificationSnapshot,
    WhyQueryOptions, WhyQueryResult, WorkStateDiff, WorkStateDiffOptions, WorkStateRestoreCommit,
    WorkStateRestoreOptions, WorkspaceInfo, WorkspaceInitOptions,
    WorkspaceResourceAssociationOptions, WorkspaceResourceAssociationResult,
};
use crate::identity::{CheckpointId, RelationId};
use crate::runtime::{
    ClaimNextOptions, ClaimNextResult, ClaimReleaseOptions, ClaimReleaseResult, ClaimSnapshot,
    ClaimTaskOptions, ClaimTaskResult, ContextOverview, ContextOverviewOptions, MergeAbortOptions,
    MergeAbortResult, MergeAttemptSnapshot, MergeContinueOptions, MergeContinueResult,
    MergeFreezeResolutionsOptions, MergeFreezeResolutionsResult, MergeListOptions, MergeListResult,
    MergeResolveOptions, MergeResolveResult, MergeStartOptions, MergeStartResult, NextWorkOptions,
    NextWorkResult, RunnableTasksOptions, RunnableTasksProjection, SessionEndOptions,
    SessionEndResult, SessionFocusOptions, SessionFocusUpdateResult, SessionSnapshot,
    SessionStartOptions, SessionStartResult, SessionSwitchOptions, SessionSwitchResult,
};
use crate::store::bootstrap::{
    StoreInfo, StoreInitOptions, ensure_empty_database, initialize_manifest, validate_bootstrap,
};
use crate::store::connection::StoreConnection;
use crate::store::schema;
use crate::{
    BranchId, ClaimId, CommitId, EntityId, EvidenceId, ResourceId, ResourceObservationId,
    SessionId, WorkspaceId, history, runtime,
};
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

    pub(crate) fn list_branches(&self, workspace_id: WorkspaceId) -> Result<Vec<BranchHead>> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        history::list_branches(&self.connection, workspace_id)
    }

    pub(crate) fn fork_branch(&mut self, options: &BranchForkOptions) -> Result<BranchForkResult> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        history::fork_branch(&mut self.connection, options)
    }

    pub(crate) fn history(&self, options: &HistoryQueryOptions) -> Result<HistoryQueryResult> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        history::query_history(&self.connection, options)
    }

    pub(crate) fn diff(&self, options: &WorkStateDiffOptions) -> Result<WorkStateDiff> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        history::diff_work_state(&self.connection, options)
    }

    pub(crate) fn refresh_branch_projection(
        &mut self,
        options: BranchProjectionRefreshOptions,
    ) -> Result<BranchProjectionRefreshResult> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        history::refresh_branch_projection(&mut self.connection, options)
    }

    pub(crate) fn branch_projection(
        &self,
        branch_id: BranchId,
    ) -> Result<BranchProjectionSnapshot> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        history::branch_projection(&self.connection, branch_id)
    }

    pub(crate) fn restore_work_state(
        &mut self,
        options: &WorkStateRestoreOptions,
    ) -> Result<WorkStateRestoreCommit> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        history::restore_work_state(&mut self.connection, options)
    }

    pub(crate) fn create_checkpoint(
        &mut self,
        options: CheckpointCreateOptions,
    ) -> Result<CheckpointCreateResult> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        history::create_checkpoint(&mut self.connection, options)
    }

    pub(crate) fn checkpoint(&self, checkpoint_id: CheckpointId) -> Result<CheckpointSnapshot> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        history::checkpoint(&self.connection, checkpoint_id)
    }

    pub(crate) fn why(&self, options: &WhyQueryOptions) -> Result<WhyQueryResult> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        history::explain_why(&self.connection, options)
    }

    pub(crate) fn validate_integrity(&self) -> Result<IntegrityReport> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        history::validate_integrity(&self.connection)
    }

    pub(crate) fn create_evidence(
        &mut self,
        options: &EvidenceCreateOptions,
    ) -> Result<EvidenceCreateResult> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        history::create_evidence(&mut self.connection, options)
    }

    pub(crate) fn evidence(&self, evidence_id: EvidenceId) -> Result<EvidenceSnapshot> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        history::evidence(&self.connection, evidence_id)
    }

    pub(crate) fn create_knowledge(
        &mut self,
        options: &KnowledgeCreateOptions,
    ) -> Result<KnowledgeCreateCommit> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        history::create_knowledge(&mut self.connection, options)
    }

    pub(crate) fn knowledge_at(
        &self,
        commit_id: CommitId,
        knowledge_entity_id: EntityId,
    ) -> Result<KnowledgeSnapshot> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        history::knowledge_at(&self.connection, commit_id, knowledge_entity_id)
    }

    pub(crate) fn knowledges_at(
        &self,
        options: &KnowledgeListOptions,
    ) -> Result<KnowledgeListResult> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        history::knowledges_at(&self.connection, options)
    }

    pub(crate) fn transition_knowledge(
        &mut self,
        options: &KnowledgeTransitionOptions,
    ) -> Result<KnowledgeTransitionCommit> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        history::transition_knowledge(&mut self.connection, options)
    }

    pub(crate) fn create_knowledge_relation(
        &mut self,
        options: &KnowledgeRelationCreateOptions,
    ) -> Result<KnowledgeRelationCreateCommit> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        history::create_knowledge_relation(&mut self.connection, options)
    }

    pub(crate) fn knowledge_relations_at(
        &self,
        options: &KnowledgeRelationListOptions,
    ) -> Result<KnowledgeRelationListResult> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        history::knowledge_relations_at(&self.connection, options)
    }

    pub(crate) fn knowledge_relation_at(
        &self,
        commit_id: CommitId,
        relation_id: RelationId,
    ) -> Result<KnowledgeRelationSnapshot> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        history::knowledge_relation_at(&self.connection, commit_id, relation_id)
    }

    pub(crate) fn remove_knowledge_relation(
        &mut self,
        options: &KnowledgeRelationRemoveOptions,
    ) -> Result<KnowledgeRelationRemoveCommit> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        history::remove_knowledge_relation(&mut self.connection, options)
    }

    pub(crate) fn restore_knowledge_relation(
        &mut self,
        options: &KnowledgeRelationRestoreOptions,
    ) -> Result<KnowledgeRelationRestoreCommit> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        history::restore_knowledge_relation(&mut self.connection, options)
    }

    pub(crate) fn create_record(
        &mut self,
        options: &RecordCreateOptions,
    ) -> Result<RecordCreateCommit> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        history::create_record(&mut self.connection, options)
    }

    pub(crate) fn record_at(
        &self,
        commit_id: CommitId,
        record_entity_id: EntityId,
    ) -> Result<RecordSnapshot> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        history::record_at(&self.connection, commit_id, record_entity_id)
    }

    pub(crate) fn records_at(&self, options: &RecordListOptions) -> Result<RecordListResult> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        history::records_at(&self.connection, options)
    }

    pub(crate) fn create_record_relation(
        &mut self,
        options: &RecordRelationCreateOptions,
    ) -> Result<RecordRelationCreateCommit> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        history::create_record_relation(&mut self.connection, options)
    }

    pub(crate) fn create_record_knowledge_relation(
        &mut self,
        options: &RecordKnowledgeRelationCreateOptions,
    ) -> Result<RecordKnowledgeRelationCreateCommit> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        history::create_record_knowledge_relation(&mut self.connection, options)
    }

    pub(crate) fn remove_record_relation(
        &mut self,
        options: &RecordRelationRemoveOptions,
    ) -> Result<RecordRelationRemoveCommit> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        history::remove_record_relation(&mut self.connection, options)
    }

    pub(crate) fn remove_record_knowledge_relation(
        &mut self,
        options: &RecordKnowledgeRelationRemoveOptions,
    ) -> Result<RecordKnowledgeRelationRemoveCommit> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        history::remove_record_knowledge_relation(&mut self.connection, options)
    }

    pub(crate) fn restore_record_knowledge_relation(
        &mut self,
        options: &RecordKnowledgeRelationRestoreOptions,
    ) -> Result<RecordKnowledgeRelationRestoreCommit> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        history::restore_record_knowledge_relation(&mut self.connection, options)
    }

    pub(crate) fn restore_record_relation(
        &mut self,
        options: &RecordRelationRestoreOptions,
    ) -> Result<RecordRelationRestoreCommit> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        history::restore_record_relation(&mut self.connection, options)
    }

    pub(crate) fn record_relations_at(
        &self,
        options: &RecordRelationListOptions,
    ) -> Result<RecordRelationListResult> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        history::record_relations_at(&self.connection, options)
    }

    pub(crate) fn record_knowledge_relations_at(
        &self,
        options: &RecordKnowledgeRelationListOptions,
    ) -> Result<RecordKnowledgeRelationListResult> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        history::record_knowledge_relations_at(&self.connection, options)
    }

    pub(crate) fn record_knowledge_relation_at(
        &self,
        commit_id: CommitId,
        relation_id: RelationId,
    ) -> Result<RecordKnowledgeRelationSnapshot> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        history::record_knowledge_relation_at(&self.connection, commit_id, relation_id)
    }

    pub(crate) fn record_relation_at(
        &self,
        commit_id: CommitId,
        relation_id: RelationId,
    ) -> Result<RecordRelationSnapshot> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        history::record_relation_at(&self.connection, commit_id, relation_id)
    }

    pub(crate) fn transition_record(
        &mut self,
        options: &RecordTransitionOptions,
    ) -> Result<RecordTransitionCommit> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        history::transition_record(&mut self.connection, options)
    }

    pub(crate) fn supersede_decision_record(
        &mut self,
        options: &crate::DecisionRecordSupersedeOptions,
    ) -> Result<crate::DecisionRecordSupersedeCommit> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        history::supersede_decision_record(&mut self.connection, options)
    }

    pub(crate) fn create_resource(
        &mut self,
        options: &ResourceCreateOptions,
    ) -> Result<ResourceCreateResult> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        history::create_resource(&mut self.connection, options)
    }

    pub(crate) fn resource(&self, resource_id: ResourceId) -> Result<ResourceSnapshot> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        history::resource(&self.connection, resource_id)
    }

    pub(crate) fn bind_resource(
        &mut self,
        options: &ResourceBindOptions,
    ) -> Result<ResourceBindResult> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        history::bind_resource(&mut self.connection, options)
    }

    pub(crate) fn associate_workspace_resource(
        &mut self,
        options: &WorkspaceResourceAssociationOptions,
    ) -> Result<WorkspaceResourceAssociationResult> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        history::associate_workspace_resource(&mut self.connection, options)
    }

    pub(crate) fn record_resource_observation(
        &mut self,
        options: &ResourceObservationCreateOptions,
    ) -> Result<ResourceObservationCreateResult> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        history::record_resource_observation(&mut self.connection, options)
    }

    pub(crate) fn resource_observation(
        &self,
        observation_id: ResourceObservationId,
    ) -> Result<ResourceObservationSnapshot> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        history::resource_observation(&self.connection, observation_id)
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

    pub(crate) fn create_plan(&mut self, options: &PlanCreateOptions) -> Result<PlanCreateCommit> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        history::create_plan(&mut self.connection, options)
    }

    pub(crate) fn transition_plan(
        &mut self,
        options: &PlanTransitionOptions,
    ) -> Result<PlanTransitionCommit> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        history::transition_plan(&mut self.connection, options)
    }

    pub(crate) fn create_goal(&mut self, options: &GoalCreateOptions) -> Result<GoalCreateCommit> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        history::create_goal(&mut self.connection, options)
    }

    pub(crate) fn transition_goal(
        &mut self,
        options: &GoalTransitionOptions,
    ) -> Result<GoalTransitionCommit> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        history::transition_goal(&mut self.connection, options)
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

    pub(crate) fn create_primary_containment(
        &mut self,
        options: &PrimaryContainmentCreateOptions,
    ) -> Result<PrimaryContainmentCreateCommit> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        history::create_primary_containment(&mut self.connection, options)
    }

    pub(crate) fn create_structural_reference(
        &mut self,
        options: &StructuralReferenceCreateOptions,
    ) -> Result<StructuralReferenceCreateCommit> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        history::create_structural_reference(&mut self.connection, options)
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

    pub(crate) fn record_verification_applicability(
        &mut self,
        options: &VerificationApplicabilityRecordOptions,
    ) -> Result<VerificationApplicabilityCacheSnapshot> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        history::record_verification_applicability(&mut self.connection, options)
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

    pub(crate) fn plan_at(
        &self,
        commit_id: CommitId,
        plan_entity_id: EntityId,
    ) -> Result<PlanSnapshot> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        history::plan_at(&self.connection, commit_id, plan_entity_id)
    }

    pub(crate) fn goal_at(
        &self,
        commit_id: CommitId,
        goal_entity_id: EntityId,
    ) -> Result<GoalSnapshot> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        history::goal_at(&self.connection, commit_id, goal_entity_id)
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

    pub(crate) fn verification_applicability_cache(
        &self,
        branch_id: BranchId,
        verification_entity_id: EntityId,
    ) -> Result<Option<VerificationApplicabilityCacheSnapshot>> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        history::verification_applicability_cache(
            &self.connection,
            branch_id,
            verification_entity_id,
        )
    }

    pub(crate) fn task_scheduling_relations_at(
        &self,
        commit_id: CommitId,
    ) -> Result<Vec<TaskSchedulingRelationSnapshot>> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        history::task_scheduling_relations_at(&self.connection, commit_id)
    }

    pub(crate) fn primary_containment_relations_at(
        &self,
        commit_id: CommitId,
    ) -> Result<Vec<PrimaryContainmentSnapshot>> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        history::primary_containment_relations_at(&self.connection, commit_id)
    }

    pub(crate) fn structural_references_at(
        &self,
        commit_id: CommitId,
    ) -> Result<Vec<StructuralReferenceSnapshot>> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        history::structural_references_at(&self.connection, commit_id)
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

    pub(crate) fn acceptance_criterion_effective_status_for_branch(
        &self,
        branch_id: BranchId,
        acceptance_criterion_entity_id: EntityId,
    ) -> Result<AcceptanceCriterionEffectiveStatus> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        history::acceptance_criterion_effective_status_for_branch(
            &self.connection,
            branch_id,
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

    pub(crate) fn switch_session(
        &mut self,
        options: &SessionSwitchOptions,
    ) -> Result<SessionSwitchResult> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        runtime::switch_session(&mut self.connection, options)
    }

    pub(crate) fn end_session(&mut self, options: &SessionEndOptions) -> Result<SessionEndResult> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        runtime::end_session(&mut self.connection, options)
    }

    pub(crate) fn start_merge(&mut self, options: &MergeStartOptions) -> Result<MergeStartResult> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        runtime::start_merge(&mut self.connection, options)
    }

    pub(crate) fn abort_merge(&mut self, options: &MergeAbortOptions) -> Result<MergeAbortResult> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        runtime::abort_merge(&mut self.connection, options)
    }

    pub(crate) fn resolve_merge_item(
        &mut self,
        options: &MergeResolveOptions,
    ) -> Result<MergeResolveResult> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        runtime::resolve_merge_item(&mut self.connection, options)
    }

    pub(crate) fn freeze_merge_resolutions(
        &mut self,
        options: &MergeFreezeResolutionsOptions,
    ) -> Result<MergeFreezeResolutionsResult> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        runtime::freeze_merge_resolutions(&mut self.connection, options)
    }

    pub(crate) fn continue_merge(
        &mut self,
        options: &MergeContinueOptions,
    ) -> Result<MergeContinueResult> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        runtime::continue_merge(&mut self.connection, options)
    }

    pub(crate) fn merge_attempt(&self, merge_id: crate::MergeId) -> Result<MergeAttemptSnapshot> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        runtime::merge_attempt(&self.connection, merge_id)
    }

    pub(crate) fn merge_attempts(&self, options: &MergeListOptions) -> Result<MergeListResult> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        runtime::merge_attempts(&self.connection, options)
    }

    pub(crate) fn claim_task(&mut self, options: &ClaimTaskOptions) -> Result<ClaimTaskResult> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        runtime::claim_task(&mut self.connection, options)
    }

    pub(crate) fn claim_next_task(
        &mut self,
        options: &ClaimNextOptions,
    ) -> Result<ClaimNextResult> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        runtime::claim_next_task(&mut self.connection, options)
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

    pub(crate) fn context_overview(
        &self,
        options: &ContextOverviewOptions,
    ) -> Result<ContextOverview> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        runtime::context_overview(&self.connection, options)
    }

    pub(crate) fn next_work(&mut self, options: &NextWorkOptions) -> Result<NextWorkResult> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        runtime::next_work(&mut self.connection, options)
    }
}
