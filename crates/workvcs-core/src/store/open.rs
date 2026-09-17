use crate::canonical::{CanonicalValue, content_object_digest};
use crate::error::Result;
use crate::history::{
    AcceptanceCriterionCreateCommit, AcceptanceCriterionCreateOptions,
    AcceptanceCriterionEffectiveStatus, AcceptanceCriterionRevisionCommit,
    AcceptanceCriterionRevisionOptions, AcceptanceCriterionSnapshot,
    AuthorizationReceiptConsumeOptions, AuthorizationReceiptConsumeResult,
    AuthorizationReceiptIssueOptions, AuthorizationReceiptIssueResult,
    AuthorizationReceiptListOptions, AuthorizationReceiptListResult, AuthorizationReceiptSnapshot,
    BranchForkOptions, BranchForkResult, BranchHead, BranchProjectionRefreshOptions,
    BranchProjectionRefreshResult, BranchProjectionSnapshot, BundleExportManifest,
    BundleExportOptions, BundleImportApplyOptions, BundleImportApplyResult,
    BundleImportAttemptListOptions, BundleImportAttemptListResult, BundleImportAttemptOptions,
    BundleImportAttemptResult, BundleImportAttemptSnapshot, BundleImportPreflightOptions,
    BundleImportPreflightResult, BundleManifestValidationOptions, BundleManifestValidationResult,
    BundlePayloadExport, BundlePayloadExportOptions, BundlePayloadValidationOptions,
    BundlePayloadValidationResult, ChangeOperationListResult, ChangeSetCausalAnchorListResult,
    ChangeSetSnapshot, CheckpointCreateOptions, CheckpointCreateResult, CheckpointLatestOptions,
    CheckpointLatestResult, CheckpointListOptions, CheckpointListResult, CheckpointSnapshot,
    CheckpointValidationResult, CognitionCaptureOptions, CognitionCaptureResult, CommitSnapshot,
    EntityTransitionCommit, EntityTransitionOptions, EventListOptions, EventListResult,
    EventSnapshot, EvidenceContentReadResult, EvidenceCreateOptions, EvidenceCreateResult,
    EvidenceListOptions, EvidenceListResult, EvidenceSnapshot, GoalCreateCommit, GoalCreateOptions,
    GoalSnapshot, GoalTransitionCommit, GoalTransitionOptions, HistoryQueryOptions,
    HistoryQueryResult, IntegrityReport, KnowledgeCreateCommit, KnowledgeCreateOptions,
    KnowledgeListOptions, KnowledgeListResult, KnowledgeRelationCreateCommit,
    KnowledgeRelationCreateOptions, KnowledgeRelationListOptions, KnowledgeRelationListResult,
    KnowledgeRelationRemoveCommit, KnowledgeRelationRemoveOptions, KnowledgeRelationRestoreCommit,
    KnowledgeRelationRestoreOptions, KnowledgeRelationSnapshot, KnowledgeSnapshot,
    KnowledgeTransitionCommit, KnowledgeTransitionOptions, PlanAdmissionOptions,
    PlanAdmissionResult, PlanCreateCommit, PlanCreateOptions, PlanEvolutionOptions,
    PlanEvolutionResult, PlanSnapshot, PlanTransitionCommit, PlanTransitionOptions,
    PrimaryContainmentCreateCommit, PrimaryContainmentCreateOptions, PrimaryContainmentSnapshot,
    RecordCreateCommit, RecordCreateOptions, RecordCurrentnessAuditOptions,
    RecordCurrentnessAuditResult, RecordKnowledgeRelationCreateCommit,
    RecordKnowledgeRelationCreateOptions, RecordKnowledgeRelationListOptions,
    RecordKnowledgeRelationListResult, RecordKnowledgeRelationRemoveCommit,
    RecordKnowledgeRelationRemoveOptions, RecordKnowledgeRelationRestoreCommit,
    RecordKnowledgeRelationRestoreOptions, RecordKnowledgeRelationSnapshot, RecordListOptions,
    RecordListResult, RecordRelationCreateCommit, RecordRelationCreateOptions,
    RecordRelationListOptions, RecordRelationListResult, RecordRelationRemoveCommit,
    RecordRelationRemoveOptions, RecordRelationRestoreCommit, RecordRelationRestoreOptions,
    RecordRelationSnapshot, RecordSnapshot, RecordTransitionCommit, RecordTransitionOptions,
    ReplayedState, ResourceBindOptions, ResourceBindResult, ResourceCreateOptions,
    ResourceCreateResult, ResourceListOptions, ResourceListResult,
    ResourceObservationCreateOptions, ResourceObservationCreateResult,
    ResourceObservationListOptions, ResourceObservationListResult, ResourceObservationSnapshot,
    ResourceSnapshot, StructuralReferenceCreateCommit, StructuralReferenceCreateOptions,
    StructuralReferenceSnapshot, TaskCreateCommit, TaskCreateOptions,
    TaskSchedulingRelationCreateCommit, TaskSchedulingRelationCreateOptions,
    TaskSchedulingRelationSnapshot, TaskSnapshot, TaskTransitionCommit, TaskTransitionOptions,
    VerificationApplicabilityCacheListOptions, VerificationApplicabilityCacheListResult,
    VerificationApplicabilityCacheSnapshot, VerificationApplicabilityRecordOptions,
    VerificationApplicabilityRefreshOptions, VerificationCreateCommit, VerificationCreateOptions,
    VerificationRequirementCreateCommit, VerificationRequirementCreateOptions,
    VerificationRequirementRevisionCommit, VerificationRequirementRevisionOptions,
    VerificationRequirementSnapshot, VerificationSnapshot, WhyQueryOptions, WhyQueryResult,
    WorkStateDiff, WorkStateDiffOptions, WorkStateRestoreCommit, WorkStateRestoreOptions,
    WorkspaceInfo, WorkspaceInitOptions, WorkspaceListOptions, WorkspaceListResult,
    WorkspaceResourceAssociationListOptions, WorkspaceResourceAssociationListResult,
    WorkspaceResourceAssociationOptions, WorkspaceResourceAssociationResult,
};
use crate::history::{
    ExternalObjectRefListOptions, ExternalObjectRefListResult, ExternalObjectRefRecordOptions,
    ExternalObjectRefRecordResult, ExternalObjectRefSnapshot,
};
use crate::history::{
    KnowledgeExposureAdoptOptions, KnowledgeExposureAdoptResult,
    KnowledgeExposureAdoptionCandidateOptions, KnowledgeExposureAdoptionCandidateResult,
    KnowledgeExposureCreateLocalOptions, KnowledgeExposureCreateResult,
    KnowledgeExposureDerivedFromRelationCreateCommit,
    KnowledgeExposureDerivedFromRelationCreateOptions, KnowledgeExposureListOptions,
    KnowledgeExposureListResult, KnowledgeExposureRefreshSourceStatusOptions,
    KnowledgeExposureRefreshSourceStatusResult, KnowledgeExposureSnapshot,
    KnowledgeExposureWithdrawOptions, KnowledgeExposureWithdrawResult,
    KnowledgeSpaceAvailableExposuresOptions, KnowledgeSpaceAvailableExposuresResult,
    KnowledgeSpaceHistoricalExposuresOptions, KnowledgeSpaceHistoricalExposuresResult,
    KnowledgeSpaceRefreshSourceStatusesOptions, KnowledgeSpaceRefreshSourceStatusesResult,
    KnowledgeSpaceSourceStaleExposuresOptions, KnowledgeSpaceSourceStaleExposuresResult,
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
use crate::identity::{
    ChangeSetId, CheckpointId, ContextPacketId, ExposureId, ExternalRefId, KnowledgeSpaceId,
    LineageId, MigrationId, RelationId,
};
use crate::runtime::{
    ClaimForceTakeoverOptions, ClaimForceTakeoverResult, ClaimGuardOptions, ClaimGuardResult,
    ClaimListOptions, ClaimListResult, ClaimNextOptions, ClaimNextResult, ClaimReleaseOptions,
    ClaimReleaseResult, ClaimSnapshot, ClaimTaskOptions, ClaimTaskResult, ClaimTransferOptions,
    ClaimTransferResult, ContextOverview, ContextOverviewOptions, ContextPacket,
    ContextPacketListOptions, ContextPacketListResult, ContextPacketOptions,
    ContextPacketSaveResult, ContextPacketSnapshot, MergeAbortOptions, MergeAbortResult,
    MergeAttemptSnapshot, MergeContinueOptions, MergeContinueResult, MergeFreezeResolutionsOptions,
    MergeFreezeResolutionsResult, MergeListOptions, MergeListResult, MergeResolveOptions,
    MergeResolveResult, MergeStartOptions, MergeStartResult, NextWorkOptions, NextWorkResult,
    RunnableTasksOptions, RunnableTasksProjection, SessionDiffSnapshot, SessionEndOptions,
    SessionEndResult, SessionFocusOptions, SessionFocusUpdateResult, SessionLifecycleState,
    SessionListOptions, SessionListResult, SessionMarkStaleOptions, SessionMarkStaleResult,
    SessionSnapshot, SessionStartOptions, SessionStartResult, SessionSwitchOptions,
    SessionSwitchResult, VerifyOptions, VerifyResult,
};
use crate::store::bootstrap::{
    STORE_FORMAT_VERSION, StoreInfo, StoreInitOptions, current_epoch_micros, ensure_empty_database,
    initialize_manifest, load_store_info, validate_application_id, validate_bootstrap,
};
use crate::store::connection::StoreConnection;
use crate::store::schema;
use crate::{
    BranchId, ClaimId, CommitId, EntityId, EventId, EvidenceId, ImportId, ResourceId,
    ResourceObservationId, SessionDiffId, SessionId, StoreId, WorkVcsError, WorkspaceId, history,
    runtime,
};
use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

pub(crate) struct Store {
    connection: StoreConnection,
    info: StoreInfo,
    path: PathBuf,
}

const LOCAL_CONTENT_OBJECT_DIR: &str = ".workvcs-objects";

fn prepare_local_evidence_content_locations(
    store_path: &Path,
    store_id: StoreId,
    options: &mut EvidenceCreateOptions,
) -> Result<()> {
    let observed_at_us = current_epoch_micros()?;
    for content in options.contents_mut() {
        let Some(raw_bytes) = content.raw_bytes() else {
            continue;
        };
        let locator = persist_local_content_object(
            store_path,
            store_id,
            content.content_digest(),
            content.size_bytes(),
            raw_bytes,
        )?;
        content.set_local_storage_location(locator, observed_at_us)?;
    }
    Ok(())
}

pub(crate) fn persist_local_content_object(
    store_path: &Path,
    store_id: StoreId,
    content_digest: crate::Digest,
    size_bytes: i64,
    raw_bytes: &[u8],
) -> Result<String> {
    if i64::try_from(raw_bytes.len()).ok() != Some(size_bytes) {
        return Err(WorkVcsError::EvidenceInvalid(format!(
            "raw content size {} does not match declared size {size_bytes}",
            raw_bytes.len()
        )));
    }
    let actual_digest = content_object_digest(raw_bytes);
    if actual_digest != content_digest {
        return Err(WorkVcsError::EvidenceInvalid(format!(
            "raw content digest {actual_digest} does not match declared digest {content_digest}"
        )));
    }
    let store_parent = store_path.parent().ok_or_else(|| {
        WorkVcsError::EvidenceInvalid(format!(
            "Store path {} has no parent for local content storage",
            store_path.display()
        ))
    })?;
    let digest_text = content_digest.to_string();
    let relative_path = local_content_relative_path(store_id, content_digest);
    let object_path = store_parent.join(&relative_path);
    if object_path.is_file() {
        verify_local_content_file(&object_path, content_digest, size_bytes)?;
        return relative_path_to_locator(&relative_path);
    }
    let object_parent = object_path.parent().ok_or_else(|| {
        WorkVcsError::EvidenceInvalid("local content object path has no parent".to_owned())
    })?;
    fs::create_dir_all(object_parent).map_err(|error| {
        WorkVcsError::EvidenceInvalid(format!(
            "cannot create local content directory {}: {error}",
            object_parent.display()
        ))
    })?;
    let temp_path = object_parent.join(format!(
        ".{}.{}.{}.tmp",
        digest_text,
        std::process::id(),
        current_epoch_micros()?
    ));
    let write_result = (|| -> Result<()> {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp_path)
            .map_err(|error| {
                WorkVcsError::EvidenceInvalid(format!(
                    "cannot create local content temporary file {}: {error}",
                    temp_path.display()
                ))
            })?;
        file.write_all(raw_bytes).map_err(|error| {
            WorkVcsError::EvidenceInvalid(format!(
                "cannot write local content temporary file {}: {error}",
                temp_path.display()
            ))
        })?;
        file.sync_all().map_err(|error| {
            WorkVcsError::EvidenceInvalid(format!(
                "cannot sync local content temporary file {}: {error}",
                temp_path.display()
            ))
        })?;
        fs::rename(&temp_path, &object_path).map_err(|error| {
            WorkVcsError::EvidenceInvalid(format!(
                "cannot publish local content object {}: {error}",
                object_path.display()
            ))
        })?;
        Ok(())
    })();
    if write_result.is_err() {
        let _ = fs::remove_file(&temp_path);
    }
    write_result?;
    verify_local_content_file(&object_path, content_digest, size_bytes)?;
    relative_path_to_locator(&relative_path)
}

pub(crate) fn local_content_relative_path(
    store_id: StoreId,
    content_digest: crate::Digest,
) -> PathBuf {
    let digest_text = content_digest.to_string();
    PathBuf::from(LOCAL_CONTENT_OBJECT_DIR)
        .join(store_id.to_string())
        .join("blake3-256")
        .join(&digest_text[..2])
        .join(digest_text)
}

pub(crate) fn verify_local_content_file(
    path: &Path,
    expected_digest: crate::Digest,
    expected_size_bytes: i64,
) -> Result<Vec<u8>> {
    let mut file = OpenOptions::new().read(true).open(path).map_err(|error| {
        WorkVcsError::EvidenceInvalid(format!(
            "cannot open local content object {}: {error}",
            path.display()
        ))
    })?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes).map_err(|error| {
        WorkVcsError::EvidenceInvalid(format!(
            "cannot read local content object {}: {error}",
            path.display()
        ))
    })?;
    if i64::try_from(bytes.len()).ok() != Some(expected_size_bytes) {
        return Err(WorkVcsError::EvidenceInvalid(format!(
            "local content object {} size {} does not match {expected_size_bytes}",
            path.display(),
            bytes.len()
        )));
    }
    let actual_digest = content_object_digest(&bytes);
    if actual_digest != expected_digest {
        return Err(WorkVcsError::EvidenceInvalid(format!(
            "local content object {} digest {actual_digest} does not match {expected_digest}",
            path.display()
        )));
    }
    Ok(bytes)
}

pub(crate) fn relative_path_to_locator(path: &Path) -> Result<String> {
    path.to_str().map(str::to_owned).ok_or_else(|| {
        WorkVcsError::EvidenceInvalid(format!(
            "local content locator {} is not valid UTF-8",
            path.display()
        ))
    })
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContextPacketSnapshotSchemaMigrationResult {
    pub store_info: StoreInfo,
    pub migrated: bool,
    pub added_schema_objects: Vec<String>,
    pub migration: Option<StoreMigrationAttemptSnapshot>,
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
        Ok(Self {
            connection,
            info,
            path: path.to_path_buf(),
        })
    }

    pub(crate) fn open(path: &Path) -> Result<Self> {
        let connection = StoreConnection::open(path)?;
        let info = validate_bootstrap(&connection)?;
        Ok(Self {
            connection,
            info,
            path: path.to_path_buf(),
        })
    }

    pub(crate) fn open_readonly(path: &Path) -> Result<Self> {
        let connection = StoreConnection::open_readonly(path)?;
        let info = validate_bootstrap(&connection)?;
        Ok(Self {
            connection,
            info,
            path: path.to_path_buf(),
        })
    }

    pub(crate) fn migrate_context_packet_snapshot_schema(
        path: &Path,
    ) -> Result<ContextPacketSnapshotSchemaMigrationResult> {
        let mut connection = StoreConnection::open(path)?;
        validate_application_id(&connection)?;
        let before_info = load_store_info(&connection)?;
        let migration = schema::migrate_context_packet_snapshot_schema(&mut connection)?;
        let added_schema_objects = migration.added_schema_objects;
        let recorded_migration = if added_schema_objects.is_empty() {
            None
        } else {
            let detail = context_packet_snapshot_schema_migration_detail(&added_schema_objects)?;
            Some(
                history::record_store_migration(
                    &mut connection,
                    StoreMigrationRecordOptions::new(
                        STORE_FORMAT_VERSION,
                        STORE_FORMAT_VERSION,
                        before_info.manifest.schema_version,
                        before_info.manifest.schema_version,
                        "workvcs-context-packet-snapshot-v1",
                        "completed",
                        detail,
                    )?,
                )?
                .migration,
            )
        };
        let store_info = validate_bootstrap(&connection)?;
        Ok(ContextPacketSnapshotSchemaMigrationResult {
            store_info,
            migrated: !added_schema_objects.is_empty(),
            added_schema_objects,
            migration: recorded_migration,
        })
    }

    pub(crate) fn info(&self) -> Result<StoreInfo> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        Ok(current)
    }

    pub(crate) fn path(&self) -> &Path {
        &self.path
    }

    pub(crate) fn create_workspace(
        &mut self,
        options: &WorkspaceInitOptions,
    ) -> Result<WorkspaceInfo> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        history::create_workspace(&mut self.connection, current.store_id, options)
    }

    pub(crate) fn admit_plan(
        &mut self,
        options: &PlanAdmissionOptions,
    ) -> Result<PlanAdmissionResult> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        history::admit_plan(&mut self.connection, options)
    }

    pub(crate) fn capture_cognition(
        &mut self,
        options: &CognitionCaptureOptions,
    ) -> Result<CognitionCaptureResult> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        history::capture_cognition(&mut self.connection, options)
    }

    pub(crate) fn evolve_plan(
        &mut self,
        options: &PlanEvolutionOptions,
    ) -> Result<PlanEvolutionResult> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        history::evolve_plan(&mut self.connection, options)
    }

    pub(crate) fn issue_authorization_receipt(
        &mut self,
        options: &AuthorizationReceiptIssueOptions,
    ) -> Result<AuthorizationReceiptIssueResult> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        history::issue_authorization_receipt(&mut self.connection, options)
    }

    pub(crate) fn consume_authorization_receipt(
        &mut self,
        options: &AuthorizationReceiptConsumeOptions,
    ) -> Result<AuthorizationReceiptConsumeResult> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        history::consume_authorization_receipt(&mut self.connection, options)
    }

    pub(crate) fn authorization_receipt_at(
        &self,
        commit_id: CommitId,
        receipt_entity_id: EntityId,
    ) -> Result<AuthorizationReceiptSnapshot> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        history::authorization_receipt_at(&self.connection, commit_id, receipt_entity_id)
    }

    pub(crate) fn authorization_receipts_at(
        &self,
        options: &AuthorizationReceiptListOptions,
    ) -> Result<AuthorizationReceiptListResult> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        history::authorization_receipts_at(&self.connection, options)
    }

    pub(crate) fn workspace_info(&self, workspace_id: WorkspaceId) -> Result<WorkspaceInfo> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        history::load_workspace_info(&self.connection, workspace_id)
    }

    pub(crate) fn workspaces(&self, options: &WorkspaceListOptions) -> Result<WorkspaceListResult> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        history::workspaces(&self.connection, options)
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

    pub(crate) fn changeset(&self, changeset_id: ChangeSetId) -> Result<ChangeSetSnapshot> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        history::changeset(&self.connection, changeset_id)
    }

    pub(crate) fn changeset_operations(
        &self,
        changeset_id: ChangeSetId,
    ) -> Result<ChangeOperationListResult> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        history::changeset_operations(&self.connection, changeset_id)
    }

    pub(crate) fn changeset_causal_anchors(
        &self,
        changeset_id: ChangeSetId,
    ) -> Result<ChangeSetCausalAnchorListResult> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        history::changeset_causal_anchors(&self.connection, changeset_id)
    }

    pub(crate) fn commit(&self, commit_id: CommitId) -> Result<CommitSnapshot> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        history::commit(&self.connection, commit_id)
    }

    pub(crate) fn event(&self, event_id: EventId) -> Result<EventSnapshot> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        history::event(&self.connection, event_id)
    }

    pub(crate) fn events(&self, options: &EventListOptions) -> Result<EventListResult> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        history::query_events(&self.connection, options)
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

    pub(crate) fn validate_checkpoint(
        &mut self,
        checkpoint_id: CheckpointId,
    ) -> Result<CheckpointValidationResult> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        history::validate_checkpoint(&mut self.connection, checkpoint_id)
    }

    pub(crate) fn checkpoints(
        &self,
        options: CheckpointListOptions,
    ) -> Result<CheckpointListResult> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        history::checkpoints(&self.connection, options)
    }

    pub(crate) fn latest_usable_checkpoint(
        &self,
        options: CheckpointLatestOptions,
    ) -> Result<CheckpointLatestResult> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        history::latest_usable_checkpoint(&self.connection, options)
    }

    pub(crate) fn export_bundle_manifest(
        &self,
        options: BundleExportOptions,
    ) -> Result<BundleExportManifest> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        history::export_bundle_manifest(&self.connection, &current, &self.path, options)
    }

    pub(crate) fn validate_bundle_manifest(
        &self,
        options: BundleManifestValidationOptions,
    ) -> Result<BundleManifestValidationResult> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        history::validate_bundle_manifest(&self.connection, &current, &self.path, options)
    }

    pub(crate) fn export_bundle_payloads(
        &self,
        options: BundlePayloadExportOptions,
    ) -> Result<BundlePayloadExport> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        history::export_bundle_payloads(&self.connection, &current, &self.path, options)
    }

    pub(crate) fn validate_bundle_payloads(
        &self,
        options: BundlePayloadValidationOptions,
    ) -> Result<BundlePayloadValidationResult> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        history::validate_bundle_payloads(&self.connection, &current, &self.path, options)
    }

    pub(crate) fn preflight_bundle_import(
        &self,
        options: BundleImportPreflightOptions,
    ) -> Result<BundleImportPreflightResult> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        history::preflight_bundle_import(&self.connection, &current, options)
    }

    pub(crate) fn record_bundle_import_attempt(
        &mut self,
        options: BundleImportAttemptOptions,
    ) -> Result<BundleImportAttemptResult> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        history::record_bundle_import_attempt(&mut self.connection, &current, options)
    }

    pub(crate) fn apply_bundle_import(
        &mut self,
        options: BundleImportApplyOptions,
    ) -> Result<BundleImportApplyResult> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        history::apply_bundle_import(&mut self.connection, &current, &self.path, options)
    }

    pub(crate) fn bundle_import_attempt(
        &self,
        import_id: ImportId,
    ) -> Result<BundleImportAttemptSnapshot> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        history::bundle_import_attempt(&self.connection, import_id)
    }

    pub(crate) fn bundle_import_attempts(
        &self,
        options: BundleImportAttemptListOptions,
    ) -> Result<BundleImportAttemptListResult> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        history::bundle_import_attempts(&self.connection, options)
    }

    pub(crate) fn record_store_lineage(
        &mut self,
        options: StoreLineageRecordOptions,
    ) -> Result<StoreLineageRecordResult> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        history::record_store_lineage(&mut self.connection, &current, options)
    }

    pub(crate) fn store_lineage(&self, lineage_id: LineageId) -> Result<StoreLineageSnapshot> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        history::store_lineage(&self.connection, lineage_id)
    }

    pub(crate) fn store_lineages(
        &self,
        options: StoreLineageListOptions,
    ) -> Result<StoreLineageListResult> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        history::store_lineages(&self.connection, options)
    }

    pub(crate) fn record_store_migration(
        &mut self,
        options: StoreMigrationRecordOptions,
    ) -> Result<StoreMigrationRecordResult> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        history::record_store_migration(&mut self.connection, options)
    }

    pub(crate) fn store_migration(
        &self,
        migration_id: MigrationId,
    ) -> Result<StoreMigrationAttemptSnapshot> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        history::store_migration(&self.connection, migration_id)
    }

    pub(crate) fn store_migrations(
        &self,
        options: StoreMigrationListOptions,
    ) -> Result<StoreMigrationListResult> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        history::store_migrations(&self.connection, options)
    }

    pub(crate) fn record_external_object_ref(
        &mut self,
        options: ExternalObjectRefRecordOptions,
    ) -> Result<ExternalObjectRefRecordResult> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        history::record_external_object_ref(&mut self.connection, &current, options)
    }

    pub(crate) fn external_object_ref(
        &self,
        external_ref_id: ExternalRefId,
    ) -> Result<ExternalObjectRefSnapshot> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        history::external_object_ref(&self.connection, external_ref_id)
    }

    pub(crate) fn external_object_refs(
        &self,
        options: ExternalObjectRefListOptions,
    ) -> Result<ExternalObjectRefListResult> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        history::external_object_refs(&self.connection, options)
    }

    pub(crate) fn create_knowledge_space(
        &mut self,
        options: &KnowledgeSpaceCreateOptions,
    ) -> Result<KnowledgeSpaceCreateResult> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        history::create_knowledge_space(&mut self.connection, options)
    }

    pub(crate) fn knowledge_space(
        &self,
        knowledge_space_id: KnowledgeSpaceId,
    ) -> Result<KnowledgeSpaceSnapshot> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        history::knowledge_space(&self.connection, knowledge_space_id)
    }

    pub(crate) fn knowledge_spaces(
        &self,
        options: KnowledgeSpaceListOptions,
    ) -> Result<KnowledgeSpaceListResult> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        history::knowledge_spaces(&self.connection, options)
    }

    pub(crate) fn create_local_knowledge_exposure(
        &mut self,
        options: KnowledgeExposureCreateLocalOptions,
    ) -> Result<KnowledgeExposureCreateResult> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        history::create_local_knowledge_exposure(&mut self.connection, options)
    }

    pub(crate) fn create_knowledge_exposure_derived_from_relation(
        &mut self,
        options: &KnowledgeExposureDerivedFromRelationCreateOptions,
    ) -> Result<KnowledgeExposureDerivedFromRelationCreateCommit> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        history::create_knowledge_exposure_derived_from_relation(&mut self.connection, options)
    }

    pub(crate) fn adopt_knowledge_exposure(
        &mut self,
        options: KnowledgeExposureAdoptOptions,
    ) -> Result<KnowledgeExposureAdoptResult> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        history::adopt_knowledge_exposure(&mut self.connection, current.store_id, options)
    }

    pub(crate) fn knowledge_exposure(
        &self,
        exposure_id: ExposureId,
    ) -> Result<KnowledgeExposureSnapshot> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        history::knowledge_exposure(&self.connection, exposure_id)
    }

    pub(crate) fn knowledge_exposure_adoption_candidate(
        &self,
        options: KnowledgeExposureAdoptionCandidateOptions,
    ) -> Result<KnowledgeExposureAdoptionCandidateResult> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        history::knowledge_exposure_adoption_candidate(&self.connection, current.store_id, options)
    }

    pub(crate) fn knowledge_exposures(
        &self,
        options: KnowledgeExposureListOptions,
    ) -> Result<KnowledgeExposureListResult> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        history::knowledge_exposures(&self.connection, options)
    }

    pub(crate) fn withdraw_knowledge_exposure(
        &mut self,
        options: KnowledgeExposureWithdrawOptions,
    ) -> Result<KnowledgeExposureWithdrawResult> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        history::withdraw_knowledge_exposure(&mut self.connection, options)
    }

    pub(crate) fn refresh_knowledge_exposure_source_status(
        &mut self,
        options: KnowledgeExposureRefreshSourceStatusOptions,
    ) -> Result<KnowledgeExposureRefreshSourceStatusResult> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        history::refresh_knowledge_exposure_source_status(&mut self.connection, options)
    }

    pub(crate) fn refresh_knowledge_space_source_statuses(
        &mut self,
        options: KnowledgeSpaceRefreshSourceStatusesOptions,
    ) -> Result<KnowledgeSpaceRefreshSourceStatusesResult> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        history::refresh_knowledge_space_source_statuses(&mut self.connection, options)
    }

    pub(crate) fn knowledge_space_available_exposures(
        &self,
        options: KnowledgeSpaceAvailableExposuresOptions,
    ) -> Result<KnowledgeSpaceAvailableExposuresResult> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        history::knowledge_space_available_exposures(&self.connection, options)
    }

    pub(crate) fn knowledge_space_source_stale_exposures(
        &self,
        options: KnowledgeSpaceSourceStaleExposuresOptions,
    ) -> Result<KnowledgeSpaceSourceStaleExposuresResult> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        history::knowledge_space_source_stale_exposures(&self.connection, options)
    }

    pub(crate) fn knowledge_space_historical_exposures(
        &self,
        options: KnowledgeSpaceHistoricalExposuresOptions,
    ) -> Result<KnowledgeSpaceHistoricalExposuresResult> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        history::knowledge_space_historical_exposures(&self.connection, options)
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
        mut options: EvidenceCreateOptions,
    ) -> Result<EvidenceCreateResult> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        prepare_local_evidence_content_locations(&self.path, current.store_id, &mut options)?;
        history::create_evidence(&mut self.connection, &options)
    }

    pub(crate) fn evidence(&self, evidence_id: EvidenceId) -> Result<EvidenceSnapshot> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        history::evidence(&self.connection, evidence_id)
    }

    pub(crate) fn read_evidence_content(
        &self,
        evidence_id: EvidenceId,
        ordinal: usize,
    ) -> Result<EvidenceContentReadResult> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        let evidence = history::evidence(&self.connection, evidence_id)?;
        let content = evidence
            .contents
            .into_iter()
            .find(|item| item.ordinal == ordinal)
            .ok_or_else(|| {
                WorkVcsError::EvidenceInvalid(format!(
                    "evidence {evidence_id} does not have content ordinal {ordinal}"
                ))
            })?;
        let expected_relative_path =
            local_content_relative_path(current.store_id, content.content_digest);
        let expected_locator = relative_path_to_locator(&expected_relative_path)?;
        let location = content
            .storage_locations
            .iter()
            .find(|location| {
                location.storage_backend == history::LOCAL_CONTENT_STORAGE_BACKEND
                    && location.locator == expected_locator
                    && location.availability_state == "available"
            })
            .ok_or_else(|| {
                WorkVcsError::EvidenceInvalid(format!(
                    "evidence {evidence_id} content ordinal {ordinal} has no available local WorkVCS object"
                ))
            })?;
        let store_parent = self.path.parent().ok_or_else(|| {
            WorkVcsError::EvidenceInvalid(format!(
                "Store path {} has no parent for local content storage",
                self.path.display()
            ))
        })?;
        let object_path = store_parent.join(&expected_relative_path);
        let raw_bytes =
            verify_local_content_file(&object_path, content.content_digest, content.size_bytes)?;
        Ok(EvidenceContentReadResult {
            evidence_id,
            ordinal,
            role: content.role,
            content_digest: content.content_digest,
            size_bytes: content.size_bytes,
            media_type: content.media_type,
            storage_backend: location.storage_backend.clone(),
            locator: location.locator.clone(),
            raw_bytes,
        })
    }

    pub(crate) fn evidences(&self, options: &EvidenceListOptions) -> Result<EvidenceListResult> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        history::evidences(&self.connection, options)
    }

    pub(crate) fn validate_local_content_storage(&self) -> Result<usize> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        let evidences = history::evidences(&self.connection, &EvidenceListOptions::all())?;
        let mut verified = 0_usize;
        for evidence in evidences.evidences {
            for content in evidence.contents {
                if content.storage_locations.iter().any(|location| {
                    location.storage_backend == history::LOCAL_CONTENT_STORAGE_BACKEND
                        && location.availability_state == "available"
                }) {
                    self.read_evidence_content(evidence.evidence_id, content.ordinal)?;
                    verified += 1;
                }
            }
        }
        Ok(verified)
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

    pub(crate) fn record_currentness_audit(
        &self,
        options: &RecordCurrentnessAuditOptions,
    ) -> Result<RecordCurrentnessAuditResult> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        history::record_currentness_audit(&self.connection, options)
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

    pub(crate) fn correct_finding_record(
        &mut self,
        options: &crate::FindingRecordCorrectionOptions,
    ) -> Result<crate::FindingRecordCorrectionCommit> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        history::correct_finding_record(&mut self.connection, options)
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

    pub(crate) fn resources(&self, options: &ResourceListOptions) -> Result<ResourceListResult> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        history::resources(&self.connection, options)
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

    pub(crate) fn workspace_resource_associations(
        &self,
        options: &WorkspaceResourceAssociationListOptions,
    ) -> Result<WorkspaceResourceAssociationListResult> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        history::workspace_resource_associations(&self.connection, options)
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

    pub(crate) fn resource_observations(
        &self,
        options: &ResourceObservationListOptions,
    ) -> Result<ResourceObservationListResult> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        history::resource_observations(&self.connection, options)
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
        if options.next_status().is_terminal()
            && let Some(actor_session_id) = options.actor_session_id()
        {
            let guard = runtime::task_claim_guard(
                &self.connection,
                &ClaimGuardOptions::terminal_task_mutation(
                    actor_session_id,
                    options.task_entity_id(),
                ),
            )?;
            self.ensure_claim_guard_targets_operation_head(
                "terminal task transition",
                actor_session_id,
                options.branch_id(),
                options.expected_head_commit_id(),
                options.task_entity_id(),
                &guard,
            )?;
            self.ensure_claim_guard_allows_operation(
                "terminal task transition",
                actor_session_id,
                options.task_entity_id(),
                &guard,
            )?;
        }
        history::transition_task(&mut self.connection, options)
    }

    pub(crate) fn create_task_scheduling_relation(
        &mut self,
        options: &TaskSchedulingRelationCreateOptions,
    ) -> Result<TaskSchedulingRelationCreateCommit> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        if let Some(actor_session_id) = options.actor_session_id() {
            self.ensure_actor_session_targets_operation_head(
                "task scheduling relation create",
                actor_session_id,
                options.branch_id(),
                options.expected_head_commit_id(),
            )?;
            self.ensure_structural_task_claim_guard_allows(
                "task scheduling relation create",
                actor_session_id,
                options.branch_id(),
                options.expected_head_commit_id(),
                options.source_task_entity_id(),
            )?;
            self.ensure_structural_task_claim_guard_allows(
                "task scheduling relation create",
                actor_session_id,
                options.branch_id(),
                options.expected_head_commit_id(),
                options.target_task_entity_id(),
            )?;
        }
        history::create_task_scheduling_relation(&mut self.connection, options)
    }

    pub(crate) fn create_primary_containment(
        &mut self,
        options: &PrimaryContainmentCreateOptions,
    ) -> Result<PrimaryContainmentCreateCommit> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        if let Some(actor_session_id) = options.actor_session_id() {
            self.ensure_actor_session_targets_operation_head(
                "primary containment create",
                actor_session_id,
                options.branch_id(),
                options.expected_head_commit_id(),
            )?;
            for task_entity_id in self.primary_containment_task_endpoint_ids(options)? {
                self.ensure_structural_task_claim_guard_allows(
                    "primary containment create",
                    actor_session_id,
                    options.branch_id(),
                    options.expected_head_commit_id(),
                    task_entity_id,
                )?;
            }
        }
        history::create_primary_containment(&mut self.connection, options)
    }

    fn ensure_actor_session_targets_operation_head(
        &self,
        operation_label: &str,
        actor_session_id: SessionId,
        branch_id: BranchId,
        expected_head_commit_id: CommitId,
    ) -> Result<()> {
        let session = runtime::session_snapshot(&self.connection, actor_session_id)?;
        if session.lifecycle_state != SessionLifecycleState::Active {
            return Err(WorkVcsError::SessionInvalid(format!(
                "session {actor_session_id} is not active"
            )));
        }
        let active_workspace_id = session.active_workspace_id.ok_or_else(|| {
            WorkVcsError::SessionInvalid(format!(
                "active session {actor_session_id} has no active workspace"
            ))
        })?;
        let active_branch_id = session.active_branch_id.ok_or_else(|| {
            WorkVcsError::SessionInvalid(format!(
                "active session {actor_session_id} has no active branch"
            ))
        })?;
        if active_branch_id != branch_id {
            return Err(WorkVcsError::ClaimInvalid(format!(
                "{operation_label} targets branch {branch_id}, but actor session {actor_session_id} is active on branch {active_branch_id}"
            )));
        }
        let branch = history::branch_head(&self.connection, active_branch_id)?;
        if branch.workspace_id != active_workspace_id {
            return Err(WorkVcsError::ClaimInvalid(format!(
                "session {actor_session_id} active branch {active_branch_id} belongs to workspace {}, not active workspace {active_workspace_id}",
                branch.workspace_id
            )));
        }
        if branch.lifecycle_state != "active" {
            return Err(WorkVcsError::ClaimInvalid(format!(
                "session {actor_session_id} active branch {active_branch_id} has lifecycle state {:?}",
                branch.lifecycle_state
            )));
        }
        if branch.head_commit_id != expected_head_commit_id {
            return Err(WorkVcsError::BranchHeadConflict(format!(
                "branch {branch_id} expected head {expected_head_commit_id}, found active session head {} before {operation_label}",
                branch.head_commit_id
            )));
        }
        Ok(())
    }

    fn ensure_structural_task_claim_guard_allows(
        &self,
        operation_label: &str,
        actor_session_id: SessionId,
        branch_id: BranchId,
        expected_head_commit_id: CommitId,
        task_entity_id: EntityId,
    ) -> Result<()> {
        let guard = runtime::task_claim_guard(
            &self.connection,
            &ClaimGuardOptions::structural_task_mutation(actor_session_id, task_entity_id),
        )?;
        self.ensure_claim_guard_targets_operation_head(
            operation_label,
            actor_session_id,
            branch_id,
            expected_head_commit_id,
            task_entity_id,
            &guard,
        )?;
        self.ensure_claim_guard_allows_operation(
            operation_label,
            actor_session_id,
            task_entity_id,
            &guard,
        )
    }

    fn ensure_claim_guard_targets_operation_head(
        &self,
        operation_label: &str,
        actor_session_id: SessionId,
        branch_id: BranchId,
        expected_head_commit_id: CommitId,
        task_entity_id: EntityId,
        guard: &ClaimGuardResult,
    ) -> Result<()> {
        if guard.branch_id != branch_id {
            return Err(WorkVcsError::ClaimInvalid(format!(
                "{operation_label} for task {task_entity_id} targets branch {branch_id}, but actor session {actor_session_id} is active on branch {}",
                guard.branch_id
            )));
        }
        if guard.head_commit_id != expected_head_commit_id {
            return Err(WorkVcsError::BranchHeadConflict(format!(
                "branch {branch_id} expected head {expected_head_commit_id}, found active session head {} before {operation_label} for task {task_entity_id}",
                guard.head_commit_id
            )));
        }
        Ok(())
    }

    fn ensure_claim_guard_allows_operation(
        &self,
        operation_label: &str,
        actor_session_id: SessionId,
        task_entity_id: EntityId,
        guard: &ClaimGuardResult,
    ) -> Result<()> {
        if guard.allowed {
            return Ok(());
        }
        Err(WorkVcsError::ClaimInvalid(format!(
            "{operation_label} for task {task_entity_id} by session {actor_session_id} is blocked by claim guard reason {}",
            guard.reason.as_str()
        )))
    }

    fn primary_containment_task_endpoint_ids(
        &self,
        options: &PrimaryContainmentCreateOptions,
    ) -> Result<Vec<EntityId>> {
        let mut task_entity_ids = Vec::new();
        for endpoint_id in [options.parent_entity_id(), options.child_entity_id()] {
            if self.entity_is_task_at(options.expected_head_commit_id(), endpoint_id)? {
                task_entity_ids.push(endpoint_id);
            }
        }
        task_entity_ids.sort();
        task_entity_ids.dedup();
        Ok(task_entity_ids)
    }

    fn entity_is_task_at(&self, commit_id: CommitId, entity_id: EntityId) -> Result<bool> {
        match history::task_at(&self.connection, commit_id, entity_id) {
            Ok(_) => Ok(true),
            Err(WorkVcsError::TaskNotFound(_)) => Ok(false),
            Err(WorkVcsError::TaskInvalid(message)) if message.contains("not \"task\"") => {
                Ok(false)
            }
            Err(error) => Err(error),
        }
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

    pub(crate) fn refresh_verification_applicability(
        &mut self,
        options: &VerificationApplicabilityRefreshOptions,
    ) -> Result<VerificationApplicabilityCacheSnapshot> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        history::refresh_verification_applicability(&mut self.connection, options)
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

    pub(crate) fn tasks_at(&self, commit_id: CommitId) -> Result<Vec<TaskSnapshot>> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        history::tasks_at(&self.connection, commit_id)
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

    pub(crate) fn plans_at(&self, commit_id: CommitId) -> Result<Vec<PlanSnapshot>> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        history::plans_at(&self.connection, commit_id)
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

    pub(crate) fn goals_at(&self, commit_id: CommitId) -> Result<Vec<GoalSnapshot>> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        history::goals_at(&self.connection, commit_id)
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

    pub(crate) fn acceptance_criteria_at(
        &self,
        commit_id: CommitId,
    ) -> Result<Vec<AcceptanceCriterionSnapshot>> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        history::acceptance_criteria_at(&self.connection, commit_id)
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

    pub(crate) fn verification_requirements_at(
        &self,
        commit_id: CommitId,
    ) -> Result<Vec<VerificationRequirementSnapshot>> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        history::verification_requirements_at(&self.connection, commit_id)
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

    pub(crate) fn verifications_at(
        &self,
        commit_id: CommitId,
    ) -> Result<Vec<VerificationSnapshot>> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        history::verifications_at(&self.connection, commit_id)
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

    pub(crate) fn verification_applicability_caches(
        &self,
        options: &VerificationApplicabilityCacheListOptions,
    ) -> Result<VerificationApplicabilityCacheListResult> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        history::verification_applicability_caches(&self.connection, options)
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

    pub(crate) fn session_diff(
        &self,
        session_diff_id: SessionDiffId,
    ) -> Result<SessionDiffSnapshot> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        runtime::session_diff(&self.connection, session_diff_id)
    }

    pub(crate) fn sessions(&self, options: &SessionListOptions) -> Result<SessionListResult> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        runtime::sessions(&self.connection, options)
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

    pub(crate) fn mark_session_potentially_stale(
        &mut self,
        options: &SessionMarkStaleOptions,
    ) -> Result<SessionMarkStaleResult> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        runtime::mark_session_potentially_stale(&mut self.connection, options)
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

    pub(crate) fn active_claims_for_session(
        &self,
        options: &ClaimListOptions,
    ) -> Result<ClaimListResult> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        runtime::active_claims_for_session(&self.connection, options)
    }

    pub(crate) fn task_claim_guard(&self, options: &ClaimGuardOptions) -> Result<ClaimGuardResult> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        runtime::task_claim_guard(&self.connection, options)
    }

    pub(crate) fn release_claim(
        &mut self,
        options: &ClaimReleaseOptions,
    ) -> Result<ClaimReleaseResult> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        runtime::release_claim(&mut self.connection, options)
    }

    pub(crate) fn transfer_claim(
        &mut self,
        options: &ClaimTransferOptions,
    ) -> Result<ClaimTransferResult> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        runtime::transfer_claim(&mut self.connection, options)
    }

    pub(crate) fn force_takeover_claim(
        &mut self,
        options: &ClaimForceTakeoverOptions,
    ) -> Result<ClaimForceTakeoverResult> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        runtime::force_takeover_claim(&mut self.connection, options)
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

    pub(crate) fn context_packet(&self, options: &ContextPacketOptions) -> Result<ContextPacket> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        runtime::context_packet(&self.connection, options)
    }

    pub(crate) fn save_context_packet(
        &mut self,
        options: &ContextPacketOptions,
    ) -> Result<ContextPacketSaveResult> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        runtime::save_context_packet(&mut self.connection, options)
    }

    pub(crate) fn context_packet_snapshot(
        &self,
        context_packet_id: ContextPacketId,
    ) -> Result<ContextPacketSnapshot> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        runtime::context_packet_snapshot(&self.connection, context_packet_id)
    }

    pub(crate) fn context_packet_snapshots(
        &self,
        options: &ContextPacketListOptions,
    ) -> Result<ContextPacketListResult> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        runtime::context_packet_snapshots(&self.connection, options)
    }

    pub(crate) fn next_work(&mut self, options: &NextWorkOptions) -> Result<NextWorkResult> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        runtime::next_work(&mut self.connection, options)
    }

    pub(crate) fn verify(&mut self, options: &VerifyOptions) -> Result<VerifyResult> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        runtime::preflight_verify(&self.connection, options)?;
        let mut options = options.clone();
        prepare_local_evidence_content_locations(
            &self.path,
            current.store_id,
            options.evidence_mut(),
        )?;
        runtime::verify(&mut self.connection, &options)
    }
}

fn context_packet_snapshot_schema_migration_detail(
    added_schema_objects: &[String],
) -> Result<CanonicalValue> {
    CanonicalValue::object(vec![
        (
            "migration_kind".to_owned(),
            CanonicalValue::String("context_packet_snapshot_schema_addition".to_owned()),
        ),
        (
            "added_schema_objects".to_owned(),
            CanonicalValue::Array(
                added_schema_objects
                    .iter()
                    .map(|object| CanonicalValue::String(object.clone()))
                    .collect(),
            ),
        ),
    ])
}
