use crate::BranchId;
use crate::CheckpointId;
use crate::ClaimId;
use crate::CommitId;
use crate::EntityId;
use crate::EvidenceId;
use crate::ExternalRefId;
use crate::ImportId;
use crate::KnowledgeSpaceId;
use crate::LineageId;
use crate::MigrationId;
use crate::RelationId;
use crate::ResourceId;
use crate::ResourceObservationId;
use crate::SessionId;
use crate::WorkspaceId;
use crate::error::Result;
use crate::history::{
    AcceptanceCriterionCreateCommit, AcceptanceCriterionCreateOptions,
    AcceptanceCriterionEffectiveStatus, AcceptanceCriterionRevisionCommit,
    AcceptanceCriterionRevisionOptions, AcceptanceCriterionSnapshot, BranchForkOptions,
    BranchForkResult, BranchHead, BranchProjectionRefreshOptions, BranchProjectionRefreshResult,
    BranchProjectionSnapshot, BundleExportManifest, BundleExportOptions,
    BundleImportAttemptListOptions, BundleImportAttemptListResult, BundleImportAttemptOptions,
    BundleImportAttemptResult, BundleImportAttemptSnapshot, BundleImportPreflightOptions,
    BundleImportPreflightResult, BundleManifestValidationOptions, BundleManifestValidationResult,
    BundlePayloadExport, BundlePayloadExportOptions, BundlePayloadValidationOptions,
    BundlePayloadValidationResult, CheckpointCreateOptions, CheckpointCreateResult,
    CheckpointLatestOptions, CheckpointLatestResult, CheckpointListOptions, CheckpointListResult,
    CheckpointSnapshot, CheckpointValidationResult, DecisionRecordSupersedeCommit,
    DecisionRecordSupersedeOptions, EntityTransitionCommit, EntityTransitionOptions,
    EvidenceCreateOptions, EvidenceCreateResult, EvidenceSnapshot, GoalCreateCommit,
    GoalCreateOptions, GoalSnapshot, GoalTransitionCommit, GoalTransitionOptions,
    HistoryQueryOptions, HistoryQueryResult, IntegrityReport, KnowledgeCreateCommit,
    KnowledgeCreateOptions, KnowledgeListOptions, KnowledgeListResult,
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
use crate::history::{
    ExternalObjectRefListOptions, ExternalObjectRefListResult, ExternalObjectRefRecordOptions,
    ExternalObjectRefRecordResult, ExternalObjectRefSnapshot,
};
use crate::history::{
    KnowledgeSpaceCreateOptions, KnowledgeSpaceCreateResult, KnowledgeSpaceListOptions,
    KnowledgeSpaceListResult, KnowledgeSpaceSnapshot,
};
use crate::history::{
    StoreLineageListOptions, StoreLineageListResult, StoreLineageRecordOptions,
    StoreLineageRecordResult, StoreLineageSnapshot,
};
use crate::history::{
    StoreMigrationAttemptSnapshot, StoreMigrationListOptions, StoreMigrationListResult,
    StoreMigrationRecordOptions, StoreMigrationRecordResult,
};
use crate::store::{Store, StoreInfo, StoreInitOptions};
use crate::{
    ClaimNextOptions, ClaimNextResult, ClaimReleaseOptions, ClaimReleaseResult, ClaimSnapshot,
    ClaimTaskOptions, ClaimTaskResult, ContextOverview, ContextOverviewOptions, MergeAbortOptions,
    MergeAbortResult, MergeAttemptSnapshot, MergeContinueOptions, MergeContinueResult,
    MergeFreezeResolutionsOptions, MergeFreezeResolutionsResult, MergeListOptions, MergeListResult,
    MergeResolveOptions, MergeResolveResult, MergeStartOptions, MergeStartResult, NextWorkOptions,
    NextWorkResult, RunnableTasksOptions, RunnableTasksProjection, SessionEndOptions,
    SessionEndResult, SessionFocusOptions, SessionFocusUpdateResult, SessionSnapshot,
    SessionStartOptions, SessionStartResult, SessionSwitchOptions, SessionSwitchResult,
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

    pub fn list_branches(&self, workspace_id: WorkspaceId) -> Result<Vec<BranchHead>> {
        self.store.list_branches(workspace_id)
    }

    pub fn fork_branch(&mut self, options: BranchForkOptions) -> Result<BranchForkResult> {
        self.store.fork_branch(&options)
    }

    pub fn history(&self, options: HistoryQueryOptions) -> Result<HistoryQueryResult> {
        self.store.history(&options)
    }

    pub fn diff(&self, options: WorkStateDiffOptions) -> Result<WorkStateDiff> {
        self.store.diff(&options)
    }

    pub fn refresh_branch_projection(
        &mut self,
        options: BranchProjectionRefreshOptions,
    ) -> Result<BranchProjectionRefreshResult> {
        self.store.refresh_branch_projection(options)
    }

    pub fn branch_projection(&self, branch_id: BranchId) -> Result<BranchProjectionSnapshot> {
        self.store.branch_projection(branch_id)
    }

    pub fn restore_work_state(
        &mut self,
        options: WorkStateRestoreOptions,
    ) -> Result<WorkStateRestoreCommit> {
        self.store.restore_work_state(&options)
    }

    pub fn create_checkpoint(
        &mut self,
        options: CheckpointCreateOptions,
    ) -> Result<CheckpointCreateResult> {
        self.store.create_checkpoint(options)
    }

    pub fn checkpoint(&self, checkpoint_id: CheckpointId) -> Result<CheckpointSnapshot> {
        self.store.checkpoint(checkpoint_id)
    }

    pub fn validate_checkpoint(
        &mut self,
        checkpoint_id: CheckpointId,
    ) -> Result<CheckpointValidationResult> {
        self.store.validate_checkpoint(checkpoint_id)
    }

    pub fn checkpoints(&self, options: CheckpointListOptions) -> Result<CheckpointListResult> {
        self.store.checkpoints(options)
    }

    pub fn latest_usable_checkpoint(
        &self,
        options: CheckpointLatestOptions,
    ) -> Result<CheckpointLatestResult> {
        self.store.latest_usable_checkpoint(options)
    }

    pub fn export_bundle_manifest(
        &self,
        options: BundleExportOptions,
    ) -> Result<BundleExportManifest> {
        self.store.export_bundle_manifest(options)
    }

    pub fn validate_bundle_manifest(
        &self,
        options: BundleManifestValidationOptions,
    ) -> Result<BundleManifestValidationResult> {
        self.store.validate_bundle_manifest(options)
    }

    pub fn export_bundle_payloads(
        &self,
        options: BundlePayloadExportOptions,
    ) -> Result<BundlePayloadExport> {
        self.store.export_bundle_payloads(options)
    }

    pub fn validate_bundle_payloads(
        &self,
        options: BundlePayloadValidationOptions,
    ) -> Result<BundlePayloadValidationResult> {
        self.store.validate_bundle_payloads(options)
    }

    pub fn preflight_bundle_import(
        &self,
        options: BundleImportPreflightOptions,
    ) -> Result<BundleImportPreflightResult> {
        self.store.preflight_bundle_import(options)
    }

    pub fn record_bundle_import_attempt(
        &mut self,
        options: BundleImportAttemptOptions,
    ) -> Result<BundleImportAttemptResult> {
        self.store.record_bundle_import_attempt(options)
    }

    pub fn bundle_import_attempt(
        &self,
        import_id: ImportId,
    ) -> Result<BundleImportAttemptSnapshot> {
        self.store.bundle_import_attempt(import_id)
    }

    pub fn bundle_import_attempts(
        &self,
        options: BundleImportAttemptListOptions,
    ) -> Result<BundleImportAttemptListResult> {
        self.store.bundle_import_attempts(options)
    }

    pub fn record_store_lineage(
        &mut self,
        options: StoreLineageRecordOptions,
    ) -> Result<StoreLineageRecordResult> {
        self.store.record_store_lineage(options)
    }

    pub fn store_lineage(&self, lineage_id: LineageId) -> Result<StoreLineageSnapshot> {
        self.store.store_lineage(lineage_id)
    }

    pub fn store_lineages(
        &self,
        options: StoreLineageListOptions,
    ) -> Result<StoreLineageListResult> {
        self.store.store_lineages(options)
    }

    pub fn record_store_migration(
        &mut self,
        options: StoreMigrationRecordOptions,
    ) -> Result<StoreMigrationRecordResult> {
        self.store.record_store_migration(options)
    }

    pub fn store_migration(
        &self,
        migration_id: MigrationId,
    ) -> Result<StoreMigrationAttemptSnapshot> {
        self.store.store_migration(migration_id)
    }

    pub fn store_migrations(
        &self,
        options: StoreMigrationListOptions,
    ) -> Result<StoreMigrationListResult> {
        self.store.store_migrations(options)
    }

    pub fn record_external_object_ref(
        &mut self,
        options: ExternalObjectRefRecordOptions,
    ) -> Result<ExternalObjectRefRecordResult> {
        self.store.record_external_object_ref(options)
    }

    pub fn external_object_ref(
        &self,
        external_ref_id: ExternalRefId,
    ) -> Result<ExternalObjectRefSnapshot> {
        self.store.external_object_ref(external_ref_id)
    }

    pub fn external_object_refs(
        &self,
        options: ExternalObjectRefListOptions,
    ) -> Result<ExternalObjectRefListResult> {
        self.store.external_object_refs(options)
    }

    pub fn create_knowledge_space(
        &mut self,
        options: KnowledgeSpaceCreateOptions,
    ) -> Result<KnowledgeSpaceCreateResult> {
        self.store.create_knowledge_space(&options)
    }

    pub fn knowledge_space(
        &self,
        knowledge_space_id: KnowledgeSpaceId,
    ) -> Result<KnowledgeSpaceSnapshot> {
        self.store.knowledge_space(knowledge_space_id)
    }

    pub fn knowledge_spaces(
        &self,
        options: KnowledgeSpaceListOptions,
    ) -> Result<KnowledgeSpaceListResult> {
        self.store.knowledge_spaces(options)
    }

    pub fn why(&self, options: WhyQueryOptions) -> Result<WhyQueryResult> {
        self.store.why(&options)
    }

    pub fn validate_integrity(&self) -> Result<IntegrityReport> {
        self.store.validate_integrity()
    }

    pub fn create_evidence(
        &mut self,
        options: EvidenceCreateOptions,
    ) -> Result<EvidenceCreateResult> {
        self.store.create_evidence(&options)
    }

    pub fn evidence(&self, evidence_id: EvidenceId) -> Result<EvidenceSnapshot> {
        self.store.evidence(evidence_id)
    }

    pub fn create_knowledge(
        &mut self,
        options: KnowledgeCreateOptions,
    ) -> Result<KnowledgeCreateCommit> {
        self.store.create_knowledge(&options)
    }

    pub fn knowledge_at(
        &self,
        commit_id: CommitId,
        knowledge_entity_id: EntityId,
    ) -> Result<KnowledgeSnapshot> {
        self.store.knowledge_at(commit_id, knowledge_entity_id)
    }

    pub fn knowledges_at(&self, options: KnowledgeListOptions) -> Result<KnowledgeListResult> {
        self.store.knowledges_at(&options)
    }

    pub fn transition_knowledge(
        &mut self,
        options: KnowledgeTransitionOptions,
    ) -> Result<KnowledgeTransitionCommit> {
        self.store.transition_knowledge(&options)
    }

    pub fn create_knowledge_relation(
        &mut self,
        options: KnowledgeRelationCreateOptions,
    ) -> Result<KnowledgeRelationCreateCommit> {
        self.store.create_knowledge_relation(&options)
    }

    pub fn knowledge_relations_at(
        &self,
        options: KnowledgeRelationListOptions,
    ) -> Result<KnowledgeRelationListResult> {
        self.store.knowledge_relations_at(&options)
    }

    pub fn knowledge_relation_at(
        &self,
        commit_id: CommitId,
        relation_id: RelationId,
    ) -> Result<KnowledgeRelationSnapshot> {
        self.store.knowledge_relation_at(commit_id, relation_id)
    }

    pub fn remove_knowledge_relation(
        &mut self,
        options: KnowledgeRelationRemoveOptions,
    ) -> Result<KnowledgeRelationRemoveCommit> {
        self.store.remove_knowledge_relation(&options)
    }

    pub fn restore_knowledge_relation(
        &mut self,
        options: KnowledgeRelationRestoreOptions,
    ) -> Result<KnowledgeRelationRestoreCommit> {
        self.store.restore_knowledge_relation(&options)
    }

    pub fn create_resource(
        &mut self,
        options: ResourceCreateOptions,
    ) -> Result<ResourceCreateResult> {
        self.store.create_resource(&options)
    }

    pub fn resource(&self, resource_id: ResourceId) -> Result<ResourceSnapshot> {
        self.store.resource(resource_id)
    }

    pub fn bind_resource(&mut self, options: ResourceBindOptions) -> Result<ResourceBindResult> {
        self.store.bind_resource(&options)
    }

    pub fn associate_workspace_resource(
        &mut self,
        options: WorkspaceResourceAssociationOptions,
    ) -> Result<WorkspaceResourceAssociationResult> {
        self.store.associate_workspace_resource(&options)
    }

    pub fn record_resource_observation(
        &mut self,
        options: ResourceObservationCreateOptions,
    ) -> Result<ResourceObservationCreateResult> {
        self.store.record_resource_observation(&options)
    }

    pub fn resource_observation(
        &self,
        observation_id: ResourceObservationId,
    ) -> Result<ResourceObservationSnapshot> {
        self.store.resource_observation(observation_id)
    }

    pub fn commit_entity_transition(
        &mut self,
        options: EntityTransitionOptions,
    ) -> Result<EntityTransitionCommit> {
        self.store.commit_entity_transition(&options)
    }

    pub fn create_plan(&mut self, options: PlanCreateOptions) -> Result<PlanCreateCommit> {
        self.store.create_plan(&options)
    }

    pub fn transition_plan(
        &mut self,
        options: PlanTransitionOptions,
    ) -> Result<PlanTransitionCommit> {
        self.store.transition_plan(&options)
    }

    pub fn create_goal(&mut self, options: GoalCreateOptions) -> Result<GoalCreateCommit> {
        self.store.create_goal(&options)
    }

    pub fn transition_goal(
        &mut self,
        options: GoalTransitionOptions,
    ) -> Result<GoalTransitionCommit> {
        self.store.transition_goal(&options)
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

    pub fn create_primary_containment(
        &mut self,
        options: PrimaryContainmentCreateOptions,
    ) -> Result<PrimaryContainmentCreateCommit> {
        self.store.create_primary_containment(&options)
    }

    pub fn create_structural_reference(
        &mut self,
        options: StructuralReferenceCreateOptions,
    ) -> Result<StructuralReferenceCreateCommit> {
        self.store.create_structural_reference(&options)
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

    pub fn create_record(&mut self, options: RecordCreateOptions) -> Result<RecordCreateCommit> {
        self.store.create_record(&options)
    }

    pub fn transition_record(
        &mut self,
        options: RecordTransitionOptions,
    ) -> Result<RecordTransitionCommit> {
        self.store.transition_record(&options)
    }

    pub fn supersede_decision_record(
        &mut self,
        options: DecisionRecordSupersedeOptions,
    ) -> Result<DecisionRecordSupersedeCommit> {
        self.store.supersede_decision_record(&options)
    }

    pub fn record_verification_applicability(
        &mut self,
        options: VerificationApplicabilityRecordOptions,
    ) -> Result<VerificationApplicabilityCacheSnapshot> {
        self.store.record_verification_applicability(&options)
    }

    pub fn task_at(&self, commit_id: CommitId, task_entity_id: EntityId) -> Result<TaskSnapshot> {
        self.store.task_at(commit_id, task_entity_id)
    }

    pub fn plan_at(&self, commit_id: CommitId, plan_entity_id: EntityId) -> Result<PlanSnapshot> {
        self.store.plan_at(commit_id, plan_entity_id)
    }

    pub fn goal_at(&self, commit_id: CommitId, goal_entity_id: EntityId) -> Result<GoalSnapshot> {
        self.store.goal_at(commit_id, goal_entity_id)
    }

    pub fn record_at(
        &self,
        commit_id: CommitId,
        record_entity_id: EntityId,
    ) -> Result<RecordSnapshot> {
        self.store.record_at(commit_id, record_entity_id)
    }

    pub fn records_at(&self, options: RecordListOptions) -> Result<RecordListResult> {
        self.store.records_at(&options)
    }

    pub fn create_record_relation(
        &mut self,
        options: RecordRelationCreateOptions,
    ) -> Result<RecordRelationCreateCommit> {
        self.store.create_record_relation(&options)
    }

    pub fn create_record_knowledge_relation(
        &mut self,
        options: RecordKnowledgeRelationCreateOptions,
    ) -> Result<RecordKnowledgeRelationCreateCommit> {
        self.store.create_record_knowledge_relation(&options)
    }

    pub fn remove_record_relation(
        &mut self,
        options: RecordRelationRemoveOptions,
    ) -> Result<RecordRelationRemoveCommit> {
        self.store.remove_record_relation(&options)
    }

    pub fn remove_record_knowledge_relation(
        &mut self,
        options: RecordKnowledgeRelationRemoveOptions,
    ) -> Result<RecordKnowledgeRelationRemoveCommit> {
        self.store.remove_record_knowledge_relation(&options)
    }

    pub fn restore_record_knowledge_relation(
        &mut self,
        options: RecordKnowledgeRelationRestoreOptions,
    ) -> Result<RecordKnowledgeRelationRestoreCommit> {
        self.store.restore_record_knowledge_relation(&options)
    }

    pub fn restore_record_relation(
        &mut self,
        options: RecordRelationRestoreOptions,
    ) -> Result<RecordRelationRestoreCommit> {
        self.store.restore_record_relation(&options)
    }

    pub fn record_relations_at(
        &self,
        options: RecordRelationListOptions,
    ) -> Result<RecordRelationListResult> {
        self.store.record_relations_at(&options)
    }

    pub fn record_knowledge_relations_at(
        &self,
        options: RecordKnowledgeRelationListOptions,
    ) -> Result<RecordKnowledgeRelationListResult> {
        self.store.record_knowledge_relations_at(&options)
    }

    pub fn record_knowledge_relation_at(
        &self,
        commit_id: CommitId,
        relation_id: RelationId,
    ) -> Result<RecordKnowledgeRelationSnapshot> {
        self.store
            .record_knowledge_relation_at(commit_id, relation_id)
    }

    pub fn record_relation_at(
        &self,
        commit_id: CommitId,
        relation_id: RelationId,
    ) -> Result<RecordRelationSnapshot> {
        self.store.record_relation_at(commit_id, relation_id)
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

    pub fn verification_applicability_cache(
        &self,
        branch_id: BranchId,
        verification_entity_id: EntityId,
    ) -> Result<Option<VerificationApplicabilityCacheSnapshot>> {
        self.store
            .verification_applicability_cache(branch_id, verification_entity_id)
    }

    pub fn task_scheduling_relations_at(
        &self,
        commit_id: CommitId,
    ) -> Result<Vec<TaskSchedulingRelationSnapshot>> {
        self.store.task_scheduling_relations_at(commit_id)
    }

    pub fn primary_containment_relations_at(
        &self,
        commit_id: CommitId,
    ) -> Result<Vec<PrimaryContainmentSnapshot>> {
        self.store.primary_containment_relations_at(commit_id)
    }

    pub fn structural_references_at(
        &self,
        commit_id: CommitId,
    ) -> Result<Vec<StructuralReferenceSnapshot>> {
        self.store.structural_references_at(commit_id)
    }

    pub fn acceptance_criterion_effective_status(
        &self,
        commit_id: CommitId,
        acceptance_criterion_entity_id: EntityId,
    ) -> Result<AcceptanceCriterionEffectiveStatus> {
        self.store
            .acceptance_criterion_effective_status(commit_id, acceptance_criterion_entity_id)
    }

    pub fn acceptance_criterion_effective_status_for_branch(
        &self,
        branch_id: BranchId,
        acceptance_criterion_entity_id: EntityId,
    ) -> Result<AcceptanceCriterionEffectiveStatus> {
        self.store.acceptance_criterion_effective_status_for_branch(
            branch_id,
            acceptance_criterion_entity_id,
        )
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

    pub fn switch_session(&mut self, options: SessionSwitchOptions) -> Result<SessionSwitchResult> {
        self.store.switch_session(&options)
    }

    pub fn end_session(&mut self, options: SessionEndOptions) -> Result<SessionEndResult> {
        self.store.end_session(&options)
    }

    pub fn start_merge(&mut self, options: MergeStartOptions) -> Result<MergeStartResult> {
        self.store.start_merge(&options)
    }

    pub fn abort_merge(&mut self, options: MergeAbortOptions) -> Result<MergeAbortResult> {
        self.store.abort_merge(&options)
    }

    pub fn resolve_merge_item(
        &mut self,
        options: MergeResolveOptions,
    ) -> Result<MergeResolveResult> {
        self.store.resolve_merge_item(&options)
    }

    pub fn freeze_merge_resolutions(
        &mut self,
        options: MergeFreezeResolutionsOptions,
    ) -> Result<MergeFreezeResolutionsResult> {
        self.store.freeze_merge_resolutions(&options)
    }

    pub fn continue_merge(&mut self, options: MergeContinueOptions) -> Result<MergeContinueResult> {
        self.store.continue_merge(&options)
    }

    pub fn merge_attempt(&self, merge_id: crate::MergeId) -> Result<MergeAttemptSnapshot> {
        self.store.merge_attempt(merge_id)
    }

    pub fn merge_attempts(&self, options: MergeListOptions) -> Result<MergeListResult> {
        self.store.merge_attempts(&options)
    }

    pub fn claim_task(&mut self, options: ClaimTaskOptions) -> Result<ClaimTaskResult> {
        self.store.claim_task(&options)
    }

    pub fn claim_next_task(&mut self, options: ClaimNextOptions) -> Result<ClaimNextResult> {
        self.store.claim_next_task(&options)
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

    pub fn context_overview(&self, options: ContextOverviewOptions) -> Result<ContextOverview> {
        self.store.context_overview(&options)
    }

    pub fn next_work(&mut self, options: NextWorkOptions) -> Result<NextWorkResult> {
        self.store.next_work(&options)
    }
}
