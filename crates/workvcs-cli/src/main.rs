use clap::{ArgGroup, Parser, Subcommand};
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use workvcs_core::{
    AcceptanceCriterionClassification, AcceptanceCriterionCreateCommit,
    AcceptanceCriterionCreateOptions, AcceptanceCriterionEffectiveStatus,
    AcceptanceCriterionRevisionCommit, AcceptanceCriterionRevisionOptions,
    AcceptanceCriterionSnapshot, ApplicabilityResourceObservationStatus,
    ApplicabilityResourceStampInput, BranchForkOptions, BranchForkResult, BranchHead, BranchId,
    BranchProjectionRefreshOptions, BranchProjectionRefreshResult, BranchProjectionSnapshot,
    BundleExportManifest, BundleExportOptions, BundleImportApplyOptions, BundleImportApplyResult,
    BundleImportAttemptListOptions, BundleImportAttemptListResult, BundleImportAttemptOptions,
    BundleImportAttemptResult, BundleImportAttemptSnapshot, BundleImportPreflightOptions,
    BundleImportPreflightResult, BundleManifestValidationOptions, BundleManifestValidationResult,
    BundlePayloadExport, BundlePayloadExportOptions, BundlePayloadInput,
    BundlePayloadValidationOptions, BundlePayloadValidationResult, CanonicalValue,
    ChangeOperationListResult, ChangeSetCausalAnchorListResult, ChangeSetId, ChangeSetSnapshot,
    CheckpointCreateOptions, CheckpointCreateResult, CheckpointId, CheckpointLatestOptions,
    CheckpointLatestResult, CheckpointListOptions, CheckpointListResult, CheckpointSnapshot,
    CheckpointValidationResult, ClaimGuardAction, ClaimGuardOptions, ClaimGuardReason,
    ClaimGuardResult, ClaimId, ClaimLifecycleState, ClaimListOptions, ClaimListResult, ClaimMode,
    ClaimNextOptions, ClaimNextResult, ClaimReleaseOptions, ClaimReleaseResult, ClaimSnapshot,
    ClaimTaskOptions, ClaimTaskResult, CommitId, CommitSnapshot, ContextOverview,
    ContextOverviewOptions, DecisionRecordSupersedeCommit, DecisionRecordSupersedeOptions, Digest,
    Engine, EntityId, EntityTransitionCommit, EntityTransitionOptions, EntityVersionId, EventId,
    EventListOptions, EventListResult, EventSnapshot, EvidenceContentInput,
    EvidenceContentSnapshot, EvidenceCreateOptions, EvidenceCreateResult, EvidenceId,
    EvidenceListOptions, EvidenceListResult, EvidenceSnapshot, ExposureId, ExposureTransitionId,
    ExternalObjectId, ExternalObjectRefListOptions, ExternalObjectRefListResult,
    ExternalObjectRefRecordOptions, ExternalObjectRefRecordResult, ExternalObjectRefSnapshot,
    ExternalObjectReferenceScope, ExternalRefId, ExternalVersionId, GoalCreateCommit,
    GoalCreateOptions, GoalSnapshot, GoalStatus, GoalTransitionCommit, GoalTransitionOptions,
    HistoryEntry, HistoryQueryOptions, ImportId, IntegrityReport, KnowledgeCreateCommit,
    KnowledgeCreateOptions, KnowledgeExposureAdoptOptions, KnowledgeExposureAdoptResult,
    KnowledgeExposureAdoptionCandidateOptions, KnowledgeExposureAdoptionCandidateResult,
    KnowledgeExposureCreateLocalOptions, KnowledgeExposureCreateResult,
    KnowledgeExposureDerivedFromRelationCreateCommit,
    KnowledgeExposureDerivedFromRelationCreateOptions, KnowledgeExposureLifecycleStatus,
    KnowledgeExposureListOptions, KnowledgeExposureListResult,
    KnowledgeExposureRefreshSourceStatusOptions, KnowledgeExposureRefreshSourceStatusResult,
    KnowledgeExposureSnapshot, KnowledgeExposureSourceStatus, KnowledgeExposureWithdrawOptions,
    KnowledgeExposureWithdrawResult, KnowledgeListOptions, KnowledgeListResult,
    KnowledgeRelationCreateCommit, KnowledgeRelationCreateOptions, KnowledgeRelationListOptions,
    KnowledgeRelationListResult, KnowledgeRelationRemoveCommit, KnowledgeRelationRemoveOptions,
    KnowledgeRelationRestoreCommit, KnowledgeRelationRestoreOptions, KnowledgeRelationSnapshot,
    KnowledgeSnapshot, KnowledgeSpaceAvailableExposuresOptions,
    KnowledgeSpaceAvailableExposuresResult, KnowledgeSpaceCreateOptions,
    KnowledgeSpaceCreateResult, KnowledgeSpaceHistoricalExposuresOptions,
    KnowledgeSpaceHistoricalExposuresResult, KnowledgeSpaceId, KnowledgeSpaceListOptions,
    KnowledgeSpaceListResult, KnowledgeSpaceRefreshSourceStatusesOptions,
    KnowledgeSpaceRefreshSourceStatusesResult, KnowledgeSpaceSnapshot,
    KnowledgeSpaceSourceStaleExposuresOptions, KnowledgeSpaceSourceStaleExposuresResult,
    KnowledgeStatus, KnowledgeTransitionCommit, KnowledgeTransitionOptions, LineageId,
    MergeAbortOptions, MergeAbortResult, MergeAttemptSnapshot, MergeContinueOptions,
    MergeContinueResult, MergeFreezeResolutionsOptions, MergeFreezeResolutionsResult, MergeId,
    MergeItemId, MergeItemResolutionSnapshot, MergeItemSnapshot, MergeItemSubject,
    MergeListOptions, MergeListResult, MergeOutcomeSnapshot, MergeResolutionKind,
    MergeResolveOptions, MergeResolveResult, MergeStartOptions, MergeStartResult, MigrationId,
    NextWorkOptions, NextWorkResult, OperationId, PlanCreateCommit, PlanCreateOptions,
    PlanSnapshot, PlanStatus, PlanTransitionCommit, PlanTransitionOptions,
    PrimaryContainmentCreateCommit, PrimaryContainmentCreateOptions, PrimaryContainmentSnapshot,
    RecordCreateCommit, RecordCreateOptions, RecordKind, RecordKnowledgeRelationCreateCommit,
    RecordKnowledgeRelationCreateOptions, RecordKnowledgeRelationListOptions,
    RecordKnowledgeRelationListResult, RecordKnowledgeRelationRemoveCommit,
    RecordKnowledgeRelationRemoveOptions, RecordKnowledgeRelationRestoreCommit,
    RecordKnowledgeRelationRestoreOptions, RecordKnowledgeRelationSnapshot, RecordListOptions,
    RecordListResult, RecordRelationCreateCommit, RecordRelationCreateOptions,
    RecordRelationListOptions, RecordRelationListResult, RecordRelationRemoveCommit,
    RecordRelationRemoveOptions, RecordRelationRestoreCommit, RecordRelationRestoreOptions,
    RecordRelationSnapshot, RecordRelationType, RecordSnapshot, RecordStatus,
    RecordTransitionCommit, RecordTransitionOptions, RelationId, RelationVersionId, ReplayedState,
    ResolvedWhyQuerySubject, ResourceBindOptions, ResourceBindResult, ResourceCreateOptions,
    ResourceCreateResult, ResourceId, ResourceListOptions, ResourceListResult,
    ResourceObservationCreateOptions, ResourceObservationCreateResult,
    ResourceObservationDetailInput, ResourceObservationId, ResourceObservationListOptions,
    ResourceObservationListResult, ResourceObservationSnapshot, ResourceSnapshot, Result,
    RunnableTaskBlockedReason, RunnableTaskCandidate, RunnableTaskClaimCoordination,
    RunnableTasksOptions, RunnableTasksProjection, SessionDiffId, SessionEndOptions,
    SessionEndResult, SessionFocusOptions, SessionFocusUpdateResult, SessionId,
    SessionLifecycleState, SessionListOptions, SessionListResult, SessionSnapshot,
    SessionStartOptions, SessionStartResult, SessionSwitchOptions, SessionSwitchResult, StoreId,
    StoreInfo, StoreInitOptions, StoreLineageListOptions, StoreLineageListResult,
    StoreLineageRecordOptions, StoreLineageRecordResult, StoreLineageSnapshot,
    StoreMigrationAttemptSnapshot, StoreMigrationListOptions, StoreMigrationListResult,
    StoreMigrationRecordOptions, StoreMigrationRecordResult, StructuralReferenceCreateCommit,
    StructuralReferenceCreateOptions, StructuralReferenceSnapshot, TaskCreateCommit,
    TaskCreateOptions, TaskSchedulingRelationCreateCommit, TaskSchedulingRelationCreateOptions,
    TaskSchedulingRelationSnapshot, TaskSnapshot, TaskStatus, TaskTransitionCommit,
    TaskTransitionOptions, VerificationApplicability, VerificationApplicabilityCacheListOptions,
    VerificationApplicabilityCacheListResult, VerificationApplicabilityCacheSnapshot,
    VerificationApplicabilityRecordOptions, VerificationCreateCommit, VerificationCreateOptions,
    VerificationRequirementCreateCommit, VerificationRequirementCreateOptions,
    VerificationRequirementRevisionCommit, VerificationRequirementRevisionOptions,
    VerificationRequirementSnapshot, VerificationResourceBasis, VerificationResult,
    VerificationSnapshot, VerificationTarget, WhyDeferredRelationFamily, WhyEntityKind,
    WhyQueryOptions, WhyQueryResult, WhyQueryTarget, WhyRelationDirection, WhyRelationEndpoint,
    WhyRelationKind, WorkState, WorkStateDiff, WorkStateDiffChangeKind, WorkStateDiffOptions,
    WorkStateDiffTarget, WorkStateRestoreCommit, WorkStateRestoreOptions, WorkVcsError,
    WorkspaceId, WorkspaceInfo, WorkspaceInitOptions, WorkspaceListOptions, WorkspaceListResult,
    WorkspaceResourceAssociationListOptions, WorkspaceResourceAssociationListResult,
    WorkspaceResourceAssociationOptions, WorkspaceResourceAssociationResult, canonical_bytes,
    content_object_digest, entity_version_digest, parse_canonical_json, relation_version_digest,
    work_state_mapping_digest,
};

#[derive(Debug, Parser)]
#[command(name = "workvcs")]
#[command(about = "WorkVCS v0.1 thin command shell")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Init {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long, default_value = "WorkVCS store")]
        display_name: String,
    },
    Doctor {
        #[arg(value_name = "STORE")]
        store: PathBuf,
    },
    Canonical {
        #[command(subcommand)]
        command: CanonicalCommand,
    },
    Id {
        #[command(subcommand)]
        command: IdCommand,
    },
    Store {
        #[command(subcommand)]
        command: StoreCommand,
    },
    #[command(group(
        ArgGroup::new("history-start")
            .required(true)
            .multiple(false)
            .args(["branch", "commit"])
    ))]
    History {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        branch: Option<String>,

        #[arg(long)]
        commit: Option<String>,

        #[arg(long)]
        limit: Option<usize>,
    },
    Changeset {
        #[command(subcommand)]
        command: ChangeSetCommand,
    },
    Commit {
        #[command(subcommand)]
        command: CommitCommand,
    },
    Event {
        #[command(subcommand)]
        command: EventCommand,
    },
    #[command(group(
        ArgGroup::new("show-at-target")
            .required(true)
            .multiple(false)
            .args(["branch", "commit"])
    ))]
    ShowAt {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        branch: Option<String>,

        #[arg(long)]
        commit: Option<String>,
    },
    #[command(group(
        ArgGroup::new("diff-from")
            .required(true)
            .multiple(false)
            .args(["from_branch", "from_commit"])
    ))]
    #[command(group(
        ArgGroup::new("diff-to")
            .required(true)
            .multiple(false)
            .args(["to_branch", "to_commit"])
    ))]
    #[command(group(
        ArgGroup::new("diff-target-id")
            .multiple(false)
            .args(["entity", "relation"])
    ))]
    Diff {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        from_branch: Option<String>,

        #[arg(long)]
        from_commit: Option<String>,

        #[arg(long)]
        to_branch: Option<String>,

        #[arg(long)]
        to_commit: Option<String>,

        #[arg(long)]
        target_kind: Option<String>,

        #[arg(long)]
        change_kind: Option<String>,

        #[arg(long)]
        entity: Option<String>,

        #[arg(long)]
        relation: Option<String>,
    },
    Entity {
        #[command(subcommand)]
        command: EntityCommand,
    },
    Reference {
        #[command(subcommand)]
        command: ReferenceCommand,
    },
    Restore {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        branch: String,

        #[arg(long)]
        head: String,

        #[arg(long)]
        target_commit: String,

        #[arg(long, default_value = "{}")]
        rationale_json: String,
    },
    #[command(group(
        ArgGroup::new("why-target")
            .required(true)
            .multiple(false)
            .args(["branch", "commit"])
    ))]
    #[command(group(
        ArgGroup::new("why-subject")
            .required(true)
            .multiple(false)
            .args(["entity", "evidence", "exposure"])
    ))]
    Why {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        branch: Option<String>,

        #[arg(long)]
        commit: Option<String>,

        #[arg(long)]
        entity: Option<String>,

        #[arg(long)]
        evidence: Option<String>,

        #[arg(long)]
        exposure: Option<String>,
    },
    Workspace {
        #[command(subcommand)]
        command: WorkspaceCommand,
    },
    Branch {
        #[command(subcommand)]
        command: BranchCommand,
    },
    Knowledge {
        #[command(subcommand)]
        command: KnowledgeCommand,
    },
    Goal {
        #[command(subcommand)]
        command: GoalCommand,
    },
    Plan {
        #[command(subcommand)]
        command: PlanCommand,
    },
    Task {
        #[command(subcommand)]
        command: TaskCommand,
    },
    Ac {
        #[command(subcommand)]
        command: AcceptanceCriterionCommand,
    },
    Vr {
        #[command(subcommand)]
        command: VerificationRequirementCommand,
    },
    Evidence {
        #[command(subcommand)]
        command: EvidenceCommand,
    },
    Resource {
        #[command(subcommand)]
        command: ResourceCommand,
    },
    Record {
        #[command(subcommand)]
        command: RecordCommand,
    },
    Session {
        #[command(subcommand)]
        command: SessionCommand,
    },
    Claim {
        #[command(subcommand)]
        command: ClaimCommand,
    },
    Context {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        session: String,
    },
    Next {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        session: String,

        #[arg(long, default_value = "exclusive")]
        mode: String,
    },
    Runnable {
        #[command(subcommand)]
        command: RunnableCommand,
    },
    Verification {
        #[command(subcommand)]
        command: VerificationCommand,
    },
    Projection {
        #[command(subcommand)]
        command: ProjectionCommand,
    },
    Bundle {
        #[command(subcommand)]
        command: BundleCommand,
    },
    Checkpoint {
        #[command(subcommand)]
        command: CheckpointCommand,
    },
    Merge {
        #[command(subcommand)]
        command: MergeCommand,
    },
}

#[derive(Debug, Subcommand)]
enum CanonicalCommand {
    #[command(group(
        ArgGroup::new("canonical-encode-source")
            .required(true)
            .multiple(false)
            .args(["json", "json_file"])
    ))]
    Encode {
        #[arg(long)]
        json: Option<String>,

        #[arg(long, value_name = "PATH")]
        json_file: Option<PathBuf>,
    },
    #[command(group(
        ArgGroup::new("canonical-digest-source")
            .required(true)
            .multiple(false)
            .args(["json", "json_file"])
    ))]
    Digest {
        #[arg(long)]
        domain: String,

        #[arg(long)]
        json: Option<String>,

        #[arg(long, value_name = "PATH")]
        json_file: Option<PathBuf>,
    },
    #[command(group(
        ArgGroup::new("canonical-content-source")
            .required(true)
            .multiple(false)
            .args(["content", "content_hex", "content_file"])
    ))]
    ContentDigest {
        #[arg(long)]
        content: Option<String>,

        #[arg(long)]
        content_hex: Option<String>,

        #[arg(long, value_name = "PATH")]
        content_file: Option<PathBuf>,
    },
    WorkStateDigest {
        #[arg(long, value_name = "ENTITY_ID=ENTITY_VERSION_ID")]
        entity: Vec<String>,

        #[arg(long, value_name = "RELATION_ID=RELATION_VERSION_ID")]
        relation: Vec<String>,
    },
}

#[derive(Debug, Subcommand)]
enum IdCommand {
    New {
        #[arg(long)]
        kind: String,
    },
}

#[derive(Debug, Subcommand)]
enum StoreCommand {
    Info {
        #[arg(value_name = "STORE")]
        store: PathBuf,
    },
    Integrity {
        #[arg(value_name = "STORE")]
        store: PathBuf,
    },
    #[command(name = "lineage-record")]
    Record {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        source_store: String,

        #[arg(long)]
        derivation_kind: String,

        #[arg(long, default_value = "{}")]
        source_root_json: String,

        #[arg(long)]
        source_bundle_digest: Option<String>,
    },
    #[command(name = "lineage-show")]
    Show {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        lineage: String,
    },
    #[command(name = "lineage-list")]
    List {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        limit: Option<usize>,

        #[arg(long)]
        source_store: Option<String>,

        #[arg(long)]
        derivation_kind: Option<String>,

        #[arg(long)]
        source_bundle_digest: Option<String>,
    },
    #[command(name = "migration-record")]
    MigrationRecord {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        from_store_format_version: i64,

        #[arg(long)]
        to_store_format_version: i64,

        #[arg(long)]
        from_schema_version: i64,

        #[arg(long)]
        to_schema_version: i64,

        #[arg(long)]
        tool_version: String,

        #[arg(long)]
        outcome: String,

        #[arg(long, default_value = "{}")]
        detail_json: String,
    },
    #[command(name = "migration-show")]
    MigrationShow {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        migration: String,
    },
    #[command(name = "migration-list")]
    MigrationList {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        limit: Option<usize>,

        #[arg(long)]
        tool_version: Option<String>,

        #[arg(long)]
        outcome: Option<String>,
    },
    #[command(name = "external-ref-record")]
    ExternalRefRecord {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        external_store: String,

        #[arg(long)]
        external_object: String,

        #[arg(long)]
        object_kind: String,

        #[arg(long)]
        scope: String,

        #[arg(long)]
        external_version_ref: Option<String>,

        #[arg(long, default_value = "{}")]
        descriptor_json: String,
    },
    #[command(name = "external-ref-show")]
    ExternalRefShow {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        external_ref: String,
    },
    #[command(name = "external-ref-list")]
    ExternalRefList {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        limit: Option<usize>,

        #[arg(long)]
        external_store: Option<String>,

        #[arg(long)]
        object_kind: Option<String>,

        #[arg(long)]
        scope: Option<String>,
    },
    #[command(name = "knowledge-space-create")]
    KnowledgeSpaceCreate {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        name: String,
    },
    #[command(name = "knowledge-space-show")]
    KnowledgeSpaceShow {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        knowledge_space: String,
    },
    #[command(name = "knowledge-space-list")]
    KnowledgeSpaceList {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        limit: Option<usize>,

        #[arg(long)]
        name: Option<String>,
    },
    #[command(name = "knowledge-space-available-exposures")]
    KnowledgeSpaceAvailableExposures {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        knowledge_space: String,

        #[arg(long)]
        limit: Option<usize>,
    },
    #[command(name = "knowledge-space-source-stale-exposures")]
    KnowledgeSpaceSourceStaleExposures {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        knowledge_space: String,

        #[arg(long)]
        limit: Option<usize>,
    },
    #[command(name = "knowledge-space-historical-exposures")]
    KnowledgeSpaceHistoricalExposures {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        knowledge_space: String,

        #[arg(long)]
        limit: Option<usize>,
    },
    #[command(name = "knowledge-space-refresh-source-statuses")]
    KnowledgeSpaceRefreshSourceStatuses {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        knowledge_space: String,

        #[arg(long)]
        limit: Option<usize>,
    },
    #[command(name = "knowledge-exposure-create-local")]
    KnowledgeExposureCreateLocal {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        knowledge_space: String,

        #[arg(long)]
        workspace: String,

        #[arg(long)]
        knowledge: String,

        #[arg(long)]
        knowledge_version: String,

        #[arg(long, default_value = "{}")]
        detail_json: String,
    },
    #[command(name = "knowledge-exposure-show")]
    KnowledgeExposureShow {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        exposure: String,
    },
    #[command(name = "knowledge-exposure-adoption-candidate")]
    KnowledgeExposureAdoptionCandidate {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        exposure: String,
    },
    #[command(name = "knowledge-exposure-adopt")]
    KnowledgeExposureAdopt {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        branch: String,

        #[arg(long)]
        head: String,

        #[arg(long)]
        exposure: String,

        #[arg(long)]
        rationale: String,
    },
    #[command(name = "knowledge-exposure-derived-from-link")]
    KnowledgeExposureDerivedFromLink {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        branch: String,

        #[arg(long)]
        head: String,

        #[arg(long)]
        knowledge: String,

        #[arg(long)]
        exposure: String,

        #[arg(long)]
        rationale: String,
    },
    #[command(name = "knowledge-exposure-withdraw")]
    KnowledgeExposureWithdraw {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        exposure: String,

        #[arg(long)]
        current_transition: String,

        #[arg(long, default_value = "{}")]
        detail_json: String,
    },
    #[command(name = "knowledge-exposure-refresh-source-status")]
    KnowledgeExposureRefreshSourceStatus {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        exposure: String,
    },
    #[command(name = "knowledge-exposure-list")]
    KnowledgeExposureList {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        limit: Option<usize>,

        #[arg(long)]
        knowledge_space: Option<String>,

        #[arg(long)]
        workspace: Option<String>,

        #[arg(long)]
        knowledge: Option<String>,

        #[arg(long)]
        lifecycle_status: Option<String>,

        #[arg(long)]
        source_status: Option<String>,
    },
}

#[derive(Debug, Subcommand)]
enum EntityCommand {
    Create {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        branch: String,

        #[arg(long)]
        head: String,

        #[arg(long)]
        kind: String,

        #[arg(long)]
        state_json: String,

        #[arg(long, default_value = "{}")]
        rationale_json: String,
    },
    Update {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        branch: String,

        #[arg(long)]
        head: String,

        #[arg(long)]
        entity: String,

        #[arg(long)]
        entity_version: String,

        #[arg(long)]
        state_json: String,

        #[arg(long, default_value = "{}")]
        rationale_json: String,
    },
}

#[derive(Debug, Subcommand)]
enum ReferenceCommand {
    Create {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        branch: String,

        #[arg(long)]
        head: String,

        #[arg(long)]
        referrer: String,

        #[arg(long)]
        target: String,

        #[arg(long, default_value = "{}")]
        rationale_json: String,
    },
    #[command(group(
        ArgGroup::new("reference-list-target")
            .required(true)
            .multiple(false)
            .args(["branch", "commit"])
    ))]
    List {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        branch: Option<String>,

        #[arg(long)]
        commit: Option<String>,

        #[arg(long)]
        referrer: Option<String>,

        #[arg(long)]
        target: Option<String>,

        #[arg(long)]
        referrer_kind: Option<String>,

        #[arg(long)]
        target_kind: Option<String>,
    },
}

#[derive(Debug, Subcommand)]
enum WorkspaceCommand {
    Create {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        display_name: String,

        #[arg(long)]
        initial_branch_name: Option<String>,
    },
    Show {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        workspace: String,
    },
    List {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        display_name: Option<String>,

        #[arg(long)]
        limit: Option<usize>,
    },
}

#[derive(Debug, Subcommand)]
enum BranchCommand {
    List {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        workspace: String,

        #[arg(long)]
        name: Option<String>,

        #[arg(long)]
        lifecycle_state: Option<String>,

        #[arg(long)]
        limit: Option<usize>,
    },
    Head {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        branch: String,
    },
    #[command(group(
        ArgGroup::new("branch-fork-source")
            .required(true)
            .multiple(false)
            .args(["from_branch", "from_commit"])
    ))]
    Fork {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        from_branch: Option<String>,

        #[arg(long)]
        from_commit: Option<String>,

        #[arg(long)]
        name: String,
    },
}

#[derive(Debug, Subcommand)]
enum ProjectionCommand {
    Refresh {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        branch: String,
    },
    Show {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        branch: String,
    },
}

#[derive(Debug, Subcommand)]
enum ChangeSetCommand {
    Show {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        changeset: String,
    },
    Operations {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        changeset: String,
    },
    Anchors {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        changeset: String,
    },
}

#[derive(Debug, Subcommand)]
enum CommitCommand {
    Show {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        commit: String,
    },
}

#[derive(Debug, Subcommand)]
enum EventCommand {
    Show {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        event: String,
    },
    #[command(group(
        ArgGroup::new("event-list-target")
            .required(true)
            .multiple(false)
            .args(["changeset", "session", "workspace"])
    ))]
    List {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        changeset: Option<String>,

        #[arg(long)]
        session: Option<String>,

        #[arg(long)]
        workspace: Option<String>,

        #[arg(long)]
        kind: Option<String>,

        #[arg(long)]
        limit: Option<usize>,
    },
}

#[derive(Debug, Subcommand)]
enum CheckpointCommand {
    Create {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        commit: String,
    },
    Show {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        checkpoint: String,
    },
    Validate {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        checkpoint: String,
    },
    List {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        commit: String,

        #[arg(long)]
        usability_state: Option<String>,

        #[arg(long)]
        content_digest: Option<String>,
    },
    Latest {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        commit: String,
    },
}

#[derive(Debug, Subcommand)]
enum BundleCommand {
    Export {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        commit: String,
    },
    ExportJson {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        commit: String,
    },
    ExportDir {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        commit: String,

        #[arg(long)]
        output_dir: PathBuf,
    },
    ValidateDir {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        commit: String,

        #[arg(long)]
        input_dir: PathBuf,
    },
    PreflightDir {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        input_dir: PathBuf,
    },
    ApplyDir {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        input_dir: PathBuf,
    },
    ImportDir {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        input_dir: PathBuf,
    },
    ImportShow {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        import: String,
    },
    ImportList {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        limit: Option<usize>,

        #[arg(long)]
        source_store: Option<String>,

        #[arg(long)]
        bundle_digest: Option<String>,
    },
    ValidateManifest {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        commit: String,

        #[arg(long)]
        manifest_file: PathBuf,
    },
}

#[derive(Debug, Subcommand)]
enum MergeCommand {
    Start {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        target_branch: String,

        #[arg(long)]
        source_branch: String,

        #[arg(long)]
        session: Option<String>,
    },
    Abort {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        merge: String,

        #[arg(long)]
        session: Option<String>,

        #[arg(long, default_value = "{}")]
        detail_json: String,
    },
    Resolve {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        item: String,

        #[arg(long)]
        kind: String,

        #[arg(long)]
        session: Option<String>,

        #[arg(long)]
        custom_payload_json: Option<String>,

        #[arg(long, default_value = "{}")]
        rationale_json: String,
    },
    Freeze {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        merge: String,
    },
    Continue {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        merge: String,

        #[arg(long)]
        session: Option<String>,

        #[arg(long, default_value = "{}")]
        detail_json: String,
    },
    Show {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        merge: String,
    },
    List {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        workspace: String,

        #[arg(long)]
        target_branch: Option<String>,

        #[arg(long)]
        include_closed: bool,

        #[arg(long)]
        runtime_state: Option<String>,

        #[arg(long)]
        outcome: Option<String>,
    },
}

#[derive(Debug, Subcommand)]
enum KnowledgeCommand {
    Create {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        branch: String,

        #[arg(long)]
        head: String,

        #[arg(long)]
        statement: String,

        #[arg(long)]
        scope_json: Option<String>,

        #[arg(long)]
        provenance_json: Option<String>,
    },
    #[command(group(
        ArgGroup::new("knowledge-show-target")
            .required(true)
            .multiple(false)
            .args(["branch", "commit"])
    ))]
    Show {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        branch: Option<String>,

        #[arg(long)]
        commit: Option<String>,

        #[arg(long)]
        knowledge: String,
    },
    #[command(group(
        ArgGroup::new("knowledge-list-target")
            .required(true)
            .multiple(false)
            .args(["branch", "commit"])
    ))]
    List {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        branch: Option<String>,

        #[arg(long)]
        commit: Option<String>,

        #[arg(long)]
        status: Option<String>,

        #[arg(long)]
        scope_json: Option<String>,

        #[arg(long)]
        statement_contains: Option<String>,
    },
    Invalidate {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        branch: String,

        #[arg(long)]
        head: String,

        #[arg(long)]
        knowledge: String,

        #[arg(long)]
        knowledge_version: String,

        #[arg(long)]
        rationale: String,
    },
    Supersede {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        branch: String,

        #[arg(long)]
        head: String,

        #[arg(long)]
        knowledge: String,

        #[arg(long)]
        knowledge_version: String,

        #[arg(long)]
        rationale: String,
    },
    LinkSupersedes {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        branch: String,

        #[arg(long)]
        head: String,

        #[arg(long)]
        replacement_knowledge: String,

        #[arg(long)]
        prior_knowledge: String,

        #[arg(long)]
        rationale: String,
    },
    #[command(group(
        ArgGroup::new("knowledge-relation-list-target")
            .required(true)
            .multiple(false)
            .args(["branch", "commit"])
    ))]
    RelationList {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        branch: Option<String>,

        #[arg(long)]
        commit: Option<String>,

        #[arg(long)]
        replacement_knowledge: Option<String>,

        #[arg(long)]
        prior_knowledge: Option<String>,
    },
    #[command(group(
        ArgGroup::new("knowledge-relation-show-target")
            .required(true)
            .multiple(false)
            .args(["branch", "commit"])
    ))]
    RelationShow {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        branch: Option<String>,

        #[arg(long)]
        commit: Option<String>,

        #[arg(long)]
        relation: String,
    },
    RelationRemove {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        branch: String,

        #[arg(long)]
        head: String,

        #[arg(long)]
        relation: String,

        #[arg(long)]
        relation_version: String,

        #[arg(long)]
        rationale: String,
    },
    RelationRestore {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        branch: String,

        #[arg(long)]
        head: String,

        #[arg(long)]
        relation: String,

        #[arg(long)]
        relation_version: String,

        #[arg(long)]
        rationale: String,
    },
}

#[derive(Debug, Subcommand)]
enum TaskCommand {
    Create {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        branch: String,

        #[arg(long)]
        head: String,

        #[arg(long)]
        description: String,

        #[arg(long)]
        priority: Option<i64>,
    },
    #[command(group(
        ArgGroup::new("task-show-target")
            .required(true)
            .multiple(false)
            .args(["branch", "commit"])
    ))]
    Show {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        branch: Option<String>,

        #[arg(long)]
        commit: Option<String>,

        #[arg(long)]
        task: String,
    },
    #[command(group(
        ArgGroup::new("task-list-target")
            .required(true)
            .multiple(false)
            .args(["branch", "commit"])
    ))]
    List {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        branch: Option<String>,

        #[arg(long)]
        commit: Option<String>,

        #[arg(long)]
        status: Option<String>,

        #[arg(long)]
        limit: Option<usize>,
    },
    Transition {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        branch: String,

        #[arg(long)]
        head: String,

        #[arg(long)]
        task: String,

        #[arg(long)]
        task_version: String,

        #[arg(long)]
        status: String,

        #[arg(long)]
        outcome: Option<String>,

        #[arg(long)]
        session: Option<String>,
    },
    DependsOn {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        branch: String,

        #[arg(long)]
        head: String,

        #[arg(long)]
        task: String,

        #[arg(long)]
        depends_on: String,

        #[arg(long)]
        session: Option<String>,
    },
    OrderedBefore {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        branch: String,

        #[arg(long)]
        head: String,

        #[arg(long)]
        earlier: String,

        #[arg(long)]
        later: String,

        #[arg(long)]
        session: Option<String>,
    },
    Contain {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        branch: String,

        #[arg(long)]
        head: String,

        #[arg(long)]
        parent: String,

        #[arg(long)]
        child: String,

        #[arg(long)]
        session: Option<String>,
    },
    #[command(group(
        ArgGroup::new("task-scheduling-list-target")
            .required(true)
            .multiple(false)
            .args(["branch", "commit"])
    ))]
    SchedulingList {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        branch: Option<String>,

        #[arg(long)]
        commit: Option<String>,

        #[arg(long)]
        relation_type: Option<String>,

        #[arg(long)]
        source_task: Option<String>,

        #[arg(long)]
        target_task: Option<String>,
    },
    #[command(group(
        ArgGroup::new("task-containment-list-target")
            .required(true)
            .multiple(false)
            .args(["branch", "commit"])
    ))]
    ContainmentList {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        branch: Option<String>,

        #[arg(long)]
        commit: Option<String>,

        #[arg(long)]
        parent: Option<String>,

        #[arg(long)]
        child: Option<String>,

        #[arg(long)]
        parent_kind: Option<String>,

        #[arg(long)]
        child_kind: Option<String>,
    },
}

#[derive(Debug, Subcommand)]
enum GoalCommand {
    Create {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        branch: String,

        #[arg(long)]
        head: String,

        #[arg(long)]
        description: String,
    },
    #[command(group(
        ArgGroup::new("goal-show-target")
            .required(true)
            .multiple(false)
            .args(["branch", "commit"])
    ))]
    Show {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        branch: Option<String>,

        #[arg(long)]
        commit: Option<String>,

        #[arg(long)]
        goal: String,
    },
    #[command(group(
        ArgGroup::new("goal-list-target")
            .required(true)
            .multiple(false)
            .args(["branch", "commit"])
    ))]
    List {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        branch: Option<String>,

        #[arg(long)]
        commit: Option<String>,

        #[arg(long)]
        status: Option<String>,

        #[arg(long)]
        limit: Option<usize>,
    },
    Achieve {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        branch: String,

        #[arg(long)]
        head: String,

        #[arg(long)]
        goal: String,

        #[arg(long)]
        goal_version: String,

        #[arg(long)]
        rationale: String,
    },
    Abandon {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        branch: String,

        #[arg(long)]
        head: String,

        #[arg(long)]
        goal: String,

        #[arg(long)]
        goal_version: String,

        #[arg(long)]
        rationale: String,
    },
    Reopen {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        branch: String,

        #[arg(long)]
        head: String,

        #[arg(long)]
        goal: String,

        #[arg(long)]
        goal_version: String,

        #[arg(long)]
        rationale: String,
    },
}

#[derive(Debug, Subcommand)]
enum PlanCommand {
    Create {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        branch: String,

        #[arg(long)]
        head: String,

        #[arg(long)]
        description: String,

        #[arg(long)]
        strategy: String,

        #[arg(long = "constraint")]
        constraints: Vec<String>,
    },
    #[command(group(
        ArgGroup::new("plan-show-target")
            .required(true)
            .multiple(false)
            .args(["branch", "commit"])
    ))]
    Show {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        branch: Option<String>,

        #[arg(long)]
        commit: Option<String>,

        #[arg(long)]
        plan: String,
    },
    #[command(group(
        ArgGroup::new("plan-list-target")
            .required(true)
            .multiple(false)
            .args(["branch", "commit"])
    ))]
    List {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        branch: Option<String>,

        #[arg(long)]
        commit: Option<String>,

        #[arg(long)]
        status: Option<String>,

        #[arg(long)]
        limit: Option<usize>,
    },
    Complete {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        branch: String,

        #[arg(long)]
        head: String,

        #[arg(long)]
        plan: String,

        #[arg(long)]
        plan_version: String,

        #[arg(long)]
        completion_rationale: Option<String>,
    },
    Abandon {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        branch: String,

        #[arg(long)]
        head: String,

        #[arg(long)]
        plan: String,

        #[arg(long)]
        plan_version: String,

        #[arg(long)]
        rationale: String,
    },
    Reopen {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        branch: String,

        #[arg(long)]
        head: String,

        #[arg(long)]
        plan: String,

        #[arg(long)]
        plan_version: String,

        #[arg(long)]
        rationale: String,
    },
}

#[derive(Debug, Subcommand)]
enum AcceptanceCriterionCommand {
    Create {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        branch: String,

        #[arg(long)]
        head: String,

        #[arg(long)]
        task: String,

        #[arg(long)]
        task_version: String,

        #[arg(long)]
        local_key: String,

        #[arg(long)]
        statement: String,

        #[arg(long, default_value = "required")]
        classification: String,
    },
    Revise {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        branch: String,

        #[arg(long)]
        head: String,

        #[arg(long)]
        criterion: String,

        #[arg(long)]
        criterion_version: String,

        #[arg(long)]
        statement: String,

        #[arg(long)]
        classification: Option<String>,

        #[arg(long, default_value = "{}")]
        rationale_json: String,
    },
    #[command(group(
        ArgGroup::new("acceptance-criterion-show-target")
            .required(true)
            .multiple(false)
            .args(["branch", "commit"])
    ))]
    Show {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        branch: Option<String>,

        #[arg(long)]
        commit: Option<String>,

        #[arg(long)]
        criterion: String,
    },
    #[command(group(
        ArgGroup::new("acceptance-criterion-list-target")
            .required(true)
            .multiple(false)
            .args(["branch", "commit"])
    ))]
    List {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        branch: Option<String>,

        #[arg(long)]
        commit: Option<String>,

        #[arg(long)]
        task: Option<String>,

        #[arg(long)]
        classification: Option<String>,

        #[arg(long)]
        limit: Option<usize>,
    },
    #[command(group(
        ArgGroup::new("acceptance-criterion-status-target")
            .required(true)
            .multiple(false)
            .args(["branch", "commit"])
    ))]
    Status {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        branch: Option<String>,

        #[arg(long)]
        commit: Option<String>,

        #[arg(long)]
        criterion: String,
    },
}

#[derive(Debug, Subcommand)]
enum VerificationRequirementCommand {
    Create {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        branch: String,

        #[arg(long)]
        head: String,

        #[arg(long)]
        criterion: String,

        #[arg(long)]
        criterion_version: String,

        #[arg(long)]
        local_key: String,

        #[arg(long)]
        statement: String,
    },
    Revise {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        branch: String,

        #[arg(long)]
        head: String,

        #[arg(long)]
        requirement: String,

        #[arg(long)]
        requirement_version: String,

        #[arg(long)]
        statement: String,

        #[arg(long, default_value = "{}")]
        rationale_json: String,
    },
    #[command(group(
        ArgGroup::new("verification-requirement-show-target")
            .required(true)
            .multiple(false)
            .args(["branch", "commit"])
    ))]
    Show {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        branch: Option<String>,

        #[arg(long)]
        commit: Option<String>,

        #[arg(long)]
        requirement: String,
    },
    #[command(group(
        ArgGroup::new("verification-requirement-list-target")
            .required(true)
            .multiple(false)
            .args(["branch", "commit"])
    ))]
    List {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        branch: Option<String>,

        #[arg(long)]
        commit: Option<String>,

        #[arg(long)]
        criterion: Option<String>,

        #[arg(long)]
        local_key: Option<String>,

        #[arg(long)]
        limit: Option<usize>,
    },
}

#[derive(Debug, Subcommand)]
#[allow(clippy::large_enum_variant)]
enum EvidenceCommand {
    #[command(group(
        ArgGroup::new("evidence-content-source")
            .multiple(false)
            .args(["content", "content_digest", "content_file"])
    ))]
    Create {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        kind: String,

        #[arg(long, default_value = "{}")]
        metadata_json: String,

        #[arg(long)]
        source_session: Option<String>,

        #[arg(long)]
        content_role: Option<String>,

        #[arg(long)]
        content: Option<String>,

        #[arg(long)]
        content_digest: Option<String>,

        #[arg(long, value_name = "PATH")]
        content_file: Option<PathBuf>,

        #[arg(long)]
        content_size_bytes: Option<i64>,

        #[arg(long)]
        media_type: Option<String>,

        #[arg(long, default_value = "{}")]
        format_metadata_json: String,
    },
    Show {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        evidence: String,
    },
    List {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        kind: Option<String>,

        #[arg(long)]
        source_session: Option<String>,

        #[arg(long)]
        limit: Option<usize>,
    },
}

#[derive(Debug, Subcommand)]
#[allow(clippy::large_enum_variant)]
enum ResourceCommand {
    Create {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        kind: String,
    },
    Show {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        resource: String,
    },
    List {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        kind: Option<String>,

        #[arg(long)]
        bound: Option<bool>,

        #[arg(long)]
        workspace: Option<String>,

        #[arg(long)]
        limit: Option<usize>,
    },
    Bind {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        resource: String,

        #[arg(long)]
        adapter_kind: String,

        #[arg(long)]
        locator: String,

        #[arg(long, default_value = "{}")]
        binding_config_json: String,
    },
    AssociateWorkspace {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        workspace: String,

        #[arg(long)]
        resource: String,

        #[arg(long, default_value = "{}")]
        metadata_json: String,
    },
    WorkspaceAssociationList {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        workspace: String,

        #[arg(long)]
        resource: Option<String>,
    },
    #[command(group(
        ArgGroup::new("resource-observation-fingerprint")
            .required(true)
            .multiple(false)
            .args(["fingerprint", "content", "content_file"])
    ))]
    #[command(group(
        ArgGroup::new("resource-observation-detail-source")
            .multiple(false)
            .args(["detail_content", "detail_content_digest", "detail_content_file"])
    ))]
    Observe {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        resource: String,

        #[arg(long)]
        adapter_kind: String,

        #[arg(long)]
        adapter_schema_version: i64,

        #[arg(long)]
        fingerprint: Option<String>,

        #[arg(long)]
        content: Option<String>,

        #[arg(long, value_name = "PATH")]
        content_file: Option<PathBuf>,

        #[arg(long, default_value = "{}")]
        summary_json: String,

        #[arg(long)]
        detail_content: Option<String>,

        #[arg(long, value_name = "PATH")]
        detail_content_file: Option<PathBuf>,

        #[arg(long)]
        detail_content_digest: Option<String>,

        #[arg(long)]
        detail_content_size_bytes: Option<i64>,

        #[arg(long)]
        detail_media_type: Option<String>,

        #[arg(long, default_value = "{}")]
        detail_format_metadata_json: String,

        #[arg(long)]
        source_session: Option<String>,
    },
    ObservationShow {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        observation: String,
    },
    ObservationList {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        resource: Option<String>,

        #[arg(long)]
        adapter_kind: Option<String>,

        #[arg(long)]
        adapter_schema_version: Option<i64>,

        #[arg(long)]
        source_session: Option<String>,

        #[arg(long)]
        limit: Option<usize>,
    },
}

#[derive(Debug, Subcommand)]
enum RecordCommand {
    #[command(group(
        ArgGroup::new("record-show-target")
            .required(true)
            .multiple(false)
            .args(["branch", "commit"])
    ))]
    Show {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        branch: Option<String>,

        #[arg(long)]
        commit: Option<String>,

        #[arg(long)]
        record: String,
    },
    #[command(group(
        ArgGroup::new("record-list-target")
            .required(true)
            .multiple(false)
            .args(["branch", "commit"])
    ))]
    List {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        branch: Option<String>,

        #[arg(long)]
        commit: Option<String>,

        #[arg(long)]
        kind: Option<String>,

        #[arg(long)]
        status: Option<String>,

        #[arg(long)]
        scope_json: Option<String>,

        #[arg(long)]
        statement_contains: Option<String>,
    },
    LinkInvalidates {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        branch: String,

        #[arg(long)]
        head: String,

        #[arg(long)]
        source_record: String,

        #[arg(long)]
        target_record: String,

        #[arg(long)]
        rationale: String,
    },
    LinkInvalidatesKnowledge {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        branch: String,

        #[arg(long)]
        head: String,

        #[arg(long)]
        source_record: String,

        #[arg(long)]
        target_knowledge: String,

        #[arg(long)]
        rationale: String,
    },
    LinkValidates {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        branch: String,

        #[arg(long)]
        head: String,

        #[arg(long)]
        source_record: String,

        #[arg(long)]
        target_record: String,

        #[arg(long)]
        rationale: String,
    },
    LinkValidatesKnowledge {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        branch: String,

        #[arg(long)]
        head: String,

        #[arg(long)]
        source_record: String,

        #[arg(long)]
        target_knowledge: String,

        #[arg(long)]
        rationale: String,
    },
    LinkSupports {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        branch: String,

        #[arg(long)]
        head: String,

        #[arg(long)]
        source_record: String,

        #[arg(long)]
        target_record: String,

        #[arg(long)]
        rationale: String,
    },
    LinkSupportsKnowledge {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        branch: String,

        #[arg(long)]
        head: String,

        #[arg(long)]
        source_record: String,

        #[arg(long)]
        target_knowledge: String,

        #[arg(long)]
        rationale: String,
    },
    LinkContradicts {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        branch: String,

        #[arg(long)]
        head: String,

        #[arg(long)]
        source_record: String,

        #[arg(long)]
        target_record: String,

        #[arg(long)]
        rationale: String,
    },
    LinkContradictsKnowledge {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        branch: String,

        #[arg(long)]
        head: String,

        #[arg(long)]
        source_record: String,

        #[arg(long)]
        target_knowledge: String,

        #[arg(long)]
        rationale: String,
    },
    LinkDerivedFrom {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        branch: String,

        #[arg(long)]
        head: String,

        #[arg(long)]
        result_record: String,

        #[arg(long)]
        source_record: String,

        #[arg(long)]
        rationale: String,
    },
    LinkRelatedTo {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        branch: String,

        #[arg(long)]
        head: String,

        #[arg(long)]
        source_record: String,

        #[arg(long)]
        target_record: String,

        #[arg(long)]
        label: String,

        #[arg(long)]
        rationale: String,
    },
    SupersedeDecision {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        branch: String,

        #[arg(long)]
        head: String,

        #[arg(long)]
        replacement_record: String,

        #[arg(long)]
        prior_record: String,

        #[arg(long)]
        prior_record_version: String,

        #[arg(long)]
        because_record: Option<String>,

        #[arg(long)]
        rationale: String,
    },
    #[command(group(
        ArgGroup::new("record-relation-list-target")
            .required(true)
            .multiple(false)
            .args(["branch", "commit"])
    ))]
    RelationList {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        branch: Option<String>,

        #[arg(long)]
        commit: Option<String>,

        #[arg(long = "type")]
        relation_type: Option<String>,

        #[arg(long)]
        label: Option<String>,

        #[arg(long)]
        source_record: Option<String>,

        #[arg(long)]
        target_record: Option<String>,
    },
    #[command(group(
        ArgGroup::new("record-knowledge-relation-list-target")
            .required(true)
            .multiple(false)
            .args(["branch", "commit"])
    ))]
    KnowledgeRelationList {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        branch: Option<String>,

        #[arg(long)]
        commit: Option<String>,

        #[arg(long = "type")]
        relation_type: Option<String>,

        #[arg(long)]
        source_record: Option<String>,

        #[arg(long)]
        target_knowledge: Option<String>,
    },
    #[command(group(
        ArgGroup::new("record-knowledge-relation-show-target")
            .required(true)
            .multiple(false)
            .args(["branch", "commit"])
    ))]
    KnowledgeRelationShow {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        branch: Option<String>,

        #[arg(long)]
        commit: Option<String>,

        #[arg(long)]
        relation: String,
    },
    KnowledgeRelationRemove {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        branch: String,

        #[arg(long)]
        head: String,

        #[arg(long)]
        relation: String,

        #[arg(long)]
        relation_version: String,

        #[arg(long)]
        rationale: String,
    },
    KnowledgeRelationRestore {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        branch: String,

        #[arg(long)]
        head: String,

        #[arg(long)]
        relation: String,

        #[arg(long)]
        relation_version: String,

        #[arg(long)]
        rationale: String,
    },
    #[command(group(
        ArgGroup::new("record-relation-show-target")
            .required(true)
            .multiple(false)
            .args(["branch", "commit"])
    ))]
    RelationShow {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        branch: Option<String>,

        #[arg(long)]
        commit: Option<String>,

        #[arg(long)]
        relation: String,
    },
    RelationRemove {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        branch: String,

        #[arg(long)]
        head: String,

        #[arg(long)]
        relation: String,

        #[arg(long)]
        relation_version: String,

        #[arg(long)]
        rationale: String,
    },
    RelationRestore {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        branch: String,

        #[arg(long)]
        head: String,

        #[arg(long)]
        relation: String,

        #[arg(long)]
        relation_version: String,

        #[arg(long)]
        rationale: String,
    },
    Assumption {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        branch: String,

        #[arg(long)]
        head: String,

        #[arg(long)]
        statement: String,

        #[arg(long)]
        scope_json: Option<String>,
    },
    Attempt {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        branch: String,

        #[arg(long)]
        head: String,

        #[arg(long)]
        statement: String,

        #[arg(long)]
        scope_json: Option<String>,
    },
    AttemptStatus {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        branch: String,

        #[arg(long)]
        head: String,

        #[arg(long)]
        record: String,

        #[arg(long)]
        record_version: String,

        #[arg(long)]
        status: String,

        #[arg(long)]
        rationale: String,
    },
    AssumptionStatus {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        branch: String,

        #[arg(long)]
        head: String,

        #[arg(long)]
        record: String,

        #[arg(long)]
        record_version: String,

        #[arg(long)]
        status: String,

        #[arg(long)]
        rationale: String,
    },
    Decision {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        branch: String,

        #[arg(long)]
        head: String,

        #[arg(long)]
        statement: String,

        #[arg(long)]
        scope_json: Option<String>,
    },
    DecisionStatus {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        branch: String,

        #[arg(long)]
        head: String,

        #[arg(long)]
        record: String,

        #[arg(long)]
        record_version: String,

        #[arg(long)]
        status: String,

        #[arg(long)]
        rationale: String,
    },
    Finding {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        branch: String,

        #[arg(long)]
        head: String,

        #[arg(long)]
        statement: String,

        #[arg(long)]
        scope_json: Option<String>,
    },
    Handoff {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        branch: String,

        #[arg(long)]
        head: String,

        #[arg(long)]
        statement: String,

        #[arg(long)]
        scope_json: Option<String>,
    },
    Question {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        branch: String,

        #[arg(long)]
        head: String,

        #[arg(long)]
        statement: String,

        #[arg(long)]
        scope_json: Option<String>,
    },
    Risk {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        branch: String,

        #[arg(long)]
        head: String,

        #[arg(long)]
        statement: String,

        #[arg(long)]
        scope_json: Option<String>,
    },
}

#[derive(Debug, Subcommand)]
enum SessionCommand {
    Start {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        workspace: String,

        #[arg(long)]
        branch: String,

        #[arg(long, default_value = "{}")]
        metadata_json: String,
    },
    Show {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        session: String,
    },
    List {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        lifecycle: Option<String>,

        #[arg(long)]
        workspace: Option<String>,

        #[arg(long)]
        branch: Option<String>,

        #[arg(long)]
        focus: Option<String>,
    },
    FocusSet {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        session: String,

        #[arg(long)]
        focus: String,
    },
    FocusClear {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        session: String,
    },
    Switch {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        session: String,

        #[arg(long)]
        workspace: String,

        #[arg(long)]
        branch: String,

        #[arg(long)]
        focus: Option<String>,
    },
    End {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        session: String,

        #[arg(long, default_value = "{}")]
        summary_json: String,
    },
}

#[derive(Debug, Subcommand)]
enum ClaimCommand {
    Show {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        claim: String,
    },
    List {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        session: String,

        #[arg(long)]
        task: Option<String>,

        #[arg(long)]
        mode: Option<String>,
    },
    Guard {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        session: String,

        #[arg(long)]
        task: String,

        #[arg(long, default_value = "terminal-task")]
        action: String,
    },
    Next {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        session: String,

        #[arg(long, default_value = "exclusive")]
        mode: String,
    },
    Task {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        session: String,

        #[arg(long)]
        task: String,

        #[arg(long, default_value = "exclusive")]
        mode: String,
    },
    Release {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        session: String,

        #[arg(long)]
        claim: String,
    },
}

#[derive(Debug, Subcommand)]
enum RunnableCommand {
    Tasks {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        session: String,

        #[arg(long)]
        task: Option<String>,

        #[arg(long)]
        status: Option<String>,

        #[arg(long)]
        runnable: Option<bool>,
    },
}

#[derive(Debug, Subcommand)]
enum VerificationCommand {
    #[command(group(
        ArgGroup::new("verification-target")
            .required(true)
            .multiple(false)
            .args(["acceptance_criterion", "verification_requirement"])
    ))]
    Record {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        branch: String,

        #[arg(long)]
        head: String,

        #[arg(long)]
        result: String,

        #[arg(long)]
        method: Option<String>,

        #[arg(long)]
        evidence: Vec<String>,

        #[arg(long)]
        acceptance_criterion: Option<String>,

        #[arg(long)]
        verification_requirement: Option<String>,

        #[arg(long)]
        resource: Option<String>,

        #[arg(long)]
        adapter_kind: Option<String>,

        #[arg(long)]
        adapter_schema_version: Option<i64>,

        #[arg(long)]
        scope_kind: Option<String>,

        #[arg(long)]
        scope_schema_version: Option<i64>,

        #[arg(long)]
        scope_payload_json: Option<String>,

        #[arg(long)]
        baseline_fingerprint: Option<String>,

        #[arg(long)]
        baseline_observation: Option<String>,
    },
    #[command(group(
        ArgGroup::new("verification-show-target")
            .required(true)
            .multiple(false)
            .args(["branch", "commit"])
    ))]
    Show {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        branch: Option<String>,

        #[arg(long)]
        commit: Option<String>,

        #[arg(long)]
        verification: String,
    },
    #[command(group(
        ArgGroup::new("verification-list-target")
            .required(true)
            .multiple(false)
            .args(["branch", "commit"])
    ))]
    List {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        branch: Option<String>,

        #[arg(long)]
        commit: Option<String>,

        #[arg(long)]
        target_kind: Option<String>,

        #[arg(long)]
        target: Option<String>,

        #[arg(long)]
        result: Option<String>,
    },
    CacheRecord {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        branch: String,

        #[arg(long)]
        head: String,

        #[arg(long)]
        verification: String,

        #[arg(long, default_value_t = 0)]
        resource_basis_ordinal: i64,

        #[arg(long)]
        adapter_kind: String,

        #[arg(long)]
        adapter_schema_version: i64,

        #[arg(long)]
        scope_schema_version: i64,

        #[arg(long)]
        observation_status: String,

        #[arg(long)]
        observed_fingerprint: Option<String>,

        #[arg(long)]
        observation: Option<String>,

        #[arg(long, default_value = "{}")]
        detail_json: String,
    },
    CacheShow {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        branch: String,

        #[arg(long)]
        verification: String,
    },
    CacheList {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        branch: String,

        #[arg(long)]
        verification: Option<String>,

        #[arg(long)]
        applicability: Option<String>,

        #[arg(long)]
        reason_code: Option<String>,

        #[arg(long)]
        limit: Option<usize>,
    },
}

fn main() {
    match run(Cli::parse()) {
        Ok(output) => print!("{output}"),
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(1);
        }
    }
}

fn run(cli: Cli) -> Result<String> {
    match cli.command {
        Command::Init {
            store,
            display_name,
        } => {
            let engine = Engine::init(store, StoreInitOptions::new(display_name)?)?;
            let info = engine.store_info()?;
            Ok(format!(
                "initialized store_id={} schema_version={}\n",
                info.store_id, info.manifest.schema_version
            ))
        }
        Command::Doctor { store } => {
            let engine = Engine::open(store)?;
            let info = engine.store_info()?;
            let integrity = engine.validate_integrity()?;
            Ok(format!(
                "ok store_id={} schema_version={} canonical_json_profile={} checked_branches={} checked_commits={} checked_changesets={} checked_change_operations={} checked_changeset_causal_anchors={} checked_events={} checked_checkpoints={} invalid_checkpoints={}\n",
                info.store_id,
                info.manifest.schema_version,
                info.manifest.canonical_json_profile,
                integrity.checked_branches,
                integrity.checked_commits,
                integrity.checked_changesets,
                integrity.checked_change_operations,
                integrity.checked_changeset_causal_anchors,
                integrity.checked_events,
                integrity.checked_checkpoints,
                integrity.invalid_checkpoints
            ))
        }
        Command::Canonical { command } => match command {
            CanonicalCommand::Encode { json, json_file } => {
                let bytes = canonical_json_input_bytes("canonical encode JSON", json, json_file)?;
                render_canonical_encode(&bytes)
            }
            CanonicalCommand::Digest {
                domain,
                json,
                json_file,
            } => {
                let bytes = canonical_json_input_bytes("canonical digest JSON", json, json_file)?;
                render_canonical_digest(&domain, &bytes)
            }
            CanonicalCommand::ContentDigest {
                content,
                content_hex,
                content_file,
            } => render_content_digest(content, content_hex, content_file),
            CanonicalCommand::WorkStateDigest { entity, relation } => {
                render_work_state_mapping_digest(entity, relation)
            }
        },
        Command::Id { command } => match command {
            IdCommand::New { kind } => render_new_id(&kind),
        },
        Command::Store { command } => match command {
            StoreCommand::Info { store } => {
                let engine = Engine::open(store)?;
                render_store_info(&engine.store_info()?)
            }
            StoreCommand::Integrity { store } => {
                let engine = Engine::open(store)?;
                Ok(render_integrity_report(&engine.validate_integrity()?))
            }
            StoreCommand::Record {
                store,
                source_store,
                derivation_kind,
                source_root_json,
                source_bundle_digest,
            } => {
                let mut engine = Engine::open(store)?;
                let source_root_descriptor =
                    parse_cli_object("store lineage source_root_json", &source_root_json)?;
                let mut options = StoreLineageRecordOptions::new(
                    StoreId::parse_canonical(&source_store)?,
                    derivation_kind,
                    source_root_descriptor,
                )?;
                if let Some(source_bundle_digest) = source_bundle_digest {
                    options =
                        options.with_source_bundle_digest(Digest::from_hex(&source_bundle_digest)?);
                }
                let result = engine.record_store_lineage(options)?;
                render_store_lineage_record_result(&result)
            }
            StoreCommand::Show { store, lineage } => {
                let engine = Engine::open(store)?;
                let snapshot = engine.store_lineage(LineageId::parse_canonical(&lineage)?)?;
                render_store_lineage_snapshot(&snapshot)
            }
            StoreCommand::List {
                store,
                limit,
                source_store,
                derivation_kind,
                source_bundle_digest,
            } => {
                let engine = Engine::open(store)?;
                let mut options = StoreLineageListOptions::new();
                if let Some(limit) = limit {
                    options = options.with_limit(limit)?;
                }
                if let Some(source_store) = source_store {
                    options =
                        options.with_source_store_id(StoreId::parse_canonical(&source_store)?);
                }
                if let Some(derivation_kind) = derivation_kind {
                    options = options.with_derivation_kind(derivation_kind)?;
                }
                if let Some(source_bundle_digest) = source_bundle_digest {
                    options =
                        options.with_source_bundle_digest(Digest::from_hex(&source_bundle_digest)?);
                }
                let result = engine.store_lineages(options)?;
                render_store_lineage_list(&result)
            }
            StoreCommand::MigrationRecord {
                store,
                from_store_format_version,
                to_store_format_version,
                from_schema_version,
                to_schema_version,
                tool_version,
                outcome,
                detail_json,
            } => {
                let mut engine = Engine::open(store)?;
                let detail = parse_cli_object("store migration detail_json", &detail_json)?;
                let options = StoreMigrationRecordOptions::new(
                    from_store_format_version,
                    to_store_format_version,
                    from_schema_version,
                    to_schema_version,
                    tool_version,
                    outcome,
                    detail,
                )?;
                let result = engine.record_store_migration(options)?;
                render_store_migration_record_result(&result)
            }
            StoreCommand::MigrationShow { store, migration } => {
                let engine = Engine::open(store)?;
                let snapshot = engine.store_migration(MigrationId::parse_canonical(&migration)?)?;
                render_store_migration_snapshot(&snapshot)
            }
            StoreCommand::MigrationList {
                store,
                limit,
                tool_version,
                outcome,
            } => {
                let engine = Engine::open(store)?;
                let mut options = StoreMigrationListOptions::new();
                if let Some(limit) = limit {
                    options = options.with_limit(limit)?;
                }
                if let Some(tool_version) = tool_version {
                    options = options.with_tool_version(tool_version)?;
                }
                if let Some(outcome) = outcome {
                    options = options.with_outcome(outcome)?;
                }
                let result = engine.store_migrations(options)?;
                render_store_migration_list(&result)
            }
            StoreCommand::ExternalRefRecord {
                store,
                external_store,
                external_object,
                object_kind,
                scope,
                external_version_ref,
                descriptor_json,
            } => {
                let mut engine = Engine::open(store)?;
                let descriptor =
                    parse_cli_object("external object ref descriptor_json", &descriptor_json)?;
                let external_store_id = StoreId::parse_canonical(&external_store)?;
                let external_object_id = ExternalObjectId::parse_canonical(&external_object)?;
                let reference_scope = ExternalObjectReferenceScope::parse(&scope)?;
                let options = match reference_scope {
                    ExternalObjectReferenceScope::Object => {
                        if external_version_ref.is_some() {
                            return Err(WorkVcsError::QueryInvalid(
                                "object-scope external refs cannot include --external-version-ref"
                                    .to_owned(),
                            ));
                        }
                        ExternalObjectRefRecordOptions::for_object(
                            external_store_id,
                            external_object_id,
                            object_kind,
                            descriptor,
                        )?
                    }
                    ExternalObjectReferenceScope::Version => {
                        let external_version_ref = external_version_ref.ok_or_else(|| {
                            WorkVcsError::QueryInvalid(
                                "version-scope external refs require --external-version-ref"
                                    .to_owned(),
                            )
                        })?;
                        ExternalObjectRefRecordOptions::for_version(
                            external_store_id,
                            external_object_id,
                            object_kind,
                            ExternalVersionId::parse_canonical(&external_version_ref)?,
                            descriptor,
                        )?
                    }
                };
                let result = engine.record_external_object_ref(options)?;
                render_external_object_ref_record_result(&result)
            }
            StoreCommand::ExternalRefShow {
                store,
                external_ref,
            } => {
                let engine = Engine::open(store)?;
                let snapshot =
                    engine.external_object_ref(ExternalRefId::parse_canonical(&external_ref)?)?;
                render_external_object_ref_snapshot(&snapshot)
            }
            StoreCommand::ExternalRefList {
                store,
                limit,
                external_store,
                object_kind,
                scope,
            } => {
                let engine = Engine::open(store)?;
                let mut options = ExternalObjectRefListOptions::new();
                if let Some(limit) = limit {
                    options = options.with_limit(limit)?;
                }
                if let Some(external_store) = external_store {
                    options =
                        options.with_external_store_id(StoreId::parse_canonical(&external_store)?);
                }
                if let Some(object_kind) = object_kind {
                    options = options.with_object_kind(object_kind)?;
                }
                if let Some(scope) = scope {
                    options =
                        options.with_reference_scope(ExternalObjectReferenceScope::parse(&scope)?);
                }
                let result = engine.external_object_refs(options)?;
                render_external_object_ref_list(&result)
            }
            StoreCommand::KnowledgeSpaceCreate { store, name } => {
                let mut engine = Engine::open(store)?;
                let result =
                    engine.create_knowledge_space(KnowledgeSpaceCreateOptions::new(name)?)?;
                Ok(render_knowledge_space_create_result(&result))
            }
            StoreCommand::KnowledgeSpaceShow {
                store,
                knowledge_space,
            } => {
                let engine = Engine::open(store)?;
                let snapshot =
                    engine.knowledge_space(KnowledgeSpaceId::parse_canonical(&knowledge_space)?)?;
                Ok(render_knowledge_space_snapshot(&snapshot))
            }
            StoreCommand::KnowledgeSpaceList { store, limit, name } => {
                let engine = Engine::open(store)?;
                let mut options = KnowledgeSpaceListOptions::new();
                if let Some(limit) = limit {
                    options = options.with_limit(limit)?;
                }
                if let Some(name) = name {
                    options = options.with_name(name)?;
                }
                let result = engine.knowledge_spaces(options)?;
                Ok(render_knowledge_space_list(&result))
            }
            StoreCommand::KnowledgeSpaceAvailableExposures {
                store,
                knowledge_space,
                limit,
            } => {
                let engine = Engine::open(store)?;
                let mut options = KnowledgeSpaceAvailableExposuresOptions::new(
                    KnowledgeSpaceId::parse_canonical(&knowledge_space)?,
                );
                if let Some(limit) = limit {
                    options = options.with_limit(limit)?;
                }
                render_knowledge_space_available_exposures(
                    &engine.knowledge_space_available_exposures(options)?,
                )
            }
            StoreCommand::KnowledgeSpaceSourceStaleExposures {
                store,
                knowledge_space,
                limit,
            } => {
                let engine = Engine::open(store)?;
                let mut options = KnowledgeSpaceSourceStaleExposuresOptions::new(
                    KnowledgeSpaceId::parse_canonical(&knowledge_space)?,
                );
                if let Some(limit) = limit {
                    options = options.with_limit(limit)?;
                }
                render_knowledge_space_source_stale_exposures(
                    &engine.knowledge_space_source_stale_exposures(options)?,
                )
            }
            StoreCommand::KnowledgeSpaceHistoricalExposures {
                store,
                knowledge_space,
                limit,
            } => {
                let engine = Engine::open(store)?;
                let mut options = KnowledgeSpaceHistoricalExposuresOptions::new(
                    KnowledgeSpaceId::parse_canonical(&knowledge_space)?,
                );
                if let Some(limit) = limit {
                    options = options.with_limit(limit)?;
                }
                render_knowledge_space_historical_exposures(
                    &engine.knowledge_space_historical_exposures(options)?,
                )
            }
            StoreCommand::KnowledgeSpaceRefreshSourceStatuses {
                store,
                knowledge_space,
                limit,
            } => {
                let mut engine = Engine::open(store)?;
                let mut options = KnowledgeSpaceRefreshSourceStatusesOptions::new(
                    KnowledgeSpaceId::parse_canonical(&knowledge_space)?,
                );
                if let Some(limit) = limit {
                    options = options.with_limit(limit)?;
                }
                render_knowledge_space_refresh_source_statuses(
                    &engine.refresh_knowledge_space_source_statuses(options)?,
                )
            }
            StoreCommand::KnowledgeExposureCreateLocal {
                store,
                knowledge_space,
                workspace,
                knowledge,
                knowledge_version,
                detail_json,
            } => {
                let mut engine = Engine::open(store)?;
                let options = KnowledgeExposureCreateLocalOptions::new(
                    KnowledgeSpaceId::parse_canonical(&knowledge_space)?,
                    workvcs_core::WorkspaceId::parse_canonical(&workspace)?,
                    EntityId::parse_canonical(&knowledge)?,
                    EntityVersionId::parse_canonical(&knowledge_version)?,
                )?
                .with_detail(parse_cli_object(
                    "knowledge exposure transition detail_json",
                    &detail_json,
                )?)?;
                let result = engine.create_local_knowledge_exposure(options)?;
                render_knowledge_exposure_create_result(&result)
            }
            StoreCommand::KnowledgeExposureShow { store, exposure } => {
                let engine = Engine::open(store)?;
                let snapshot =
                    engine.knowledge_exposure(ExposureId::parse_canonical(&exposure)?)?;
                render_knowledge_exposure_snapshot(&snapshot)
            }
            StoreCommand::KnowledgeExposureAdoptionCandidate { store, exposure } => {
                let engine = Engine::open(store)?;
                let result = engine.knowledge_exposure_adoption_candidate(
                    KnowledgeExposureAdoptionCandidateOptions::new(ExposureId::parse_canonical(
                        &exposure,
                    )?),
                )?;
                render_knowledge_exposure_adoption_candidate(&result)
            }
            StoreCommand::KnowledgeExposureAdopt {
                store,
                branch,
                head,
                exposure,
                rationale,
            } => {
                let mut engine = Engine::open(store)?;
                let result =
                    engine.adopt_knowledge_exposure(KnowledgeExposureAdoptOptions::new(
                        BranchId::parse_canonical(&branch)?,
                        CommitId::parse_canonical(&head)?,
                        ExposureId::parse_canonical(&exposure)?,
                        rationale,
                    )?)?;
                render_knowledge_exposure_adopt(&result)
            }
            StoreCommand::KnowledgeExposureDerivedFromLink {
                store,
                branch,
                head,
                knowledge,
                exposure,
                rationale,
            } => {
                let mut engine = Engine::open(store)?;
                let result = engine.create_knowledge_exposure_derived_from_relation(
                    KnowledgeExposureDerivedFromRelationCreateOptions::new(
                        BranchId::parse_canonical(&branch)?,
                        CommitId::parse_canonical(&head)?,
                        EntityId::parse_canonical(&knowledge)?,
                        ExposureId::parse_canonical(&exposure)?,
                        rationale,
                    )?,
                )?;
                Ok(render_knowledge_exposure_derived_from_relation_create(
                    &result,
                ))
            }
            StoreCommand::KnowledgeExposureWithdraw {
                store,
                exposure,
                current_transition,
                detail_json,
            } => {
                let mut engine = Engine::open(store)?;
                let options = KnowledgeExposureWithdrawOptions::new(
                    ExposureId::parse_canonical(&exposure)?,
                    ExposureTransitionId::parse_canonical(&current_transition)?,
                )?
                .with_detail(parse_cli_object(
                    "knowledge exposure withdrawal detail_json",
                    &detail_json,
                )?)?;
                let result = engine.withdraw_knowledge_exposure(options)?;
                render_knowledge_exposure_withdraw_result(&result)
            }
            StoreCommand::KnowledgeExposureRefreshSourceStatus { store, exposure } => {
                let mut engine = Engine::open(store)?;
                let result = engine.refresh_knowledge_exposure_source_status(
                    KnowledgeExposureRefreshSourceStatusOptions::new(ExposureId::parse_canonical(
                        &exposure,
                    )?),
                )?;
                render_knowledge_exposure_refresh_source_status_result(&result)
            }
            StoreCommand::KnowledgeExposureList {
                store,
                limit,
                knowledge_space,
                workspace,
                knowledge,
                lifecycle_status,
                source_status,
            } => {
                let engine = Engine::open(store)?;
                let mut options = KnowledgeExposureListOptions::new();
                if let Some(limit) = limit {
                    options = options.with_limit(limit)?;
                }
                if let Some(knowledge_space) = knowledge_space {
                    options = options.with_knowledge_space_id(KnowledgeSpaceId::parse_canonical(
                        &knowledge_space,
                    )?);
                }
                if let Some(workspace) = workspace {
                    options = options
                        .with_workspace_id(workvcs_core::WorkspaceId::parse_canonical(&workspace)?);
                }
                if let Some(knowledge) = knowledge {
                    options =
                        options.with_knowledge_entity_id(EntityId::parse_canonical(&knowledge)?);
                }
                if let Some(lifecycle_status) = lifecycle_status {
                    options = options.with_lifecycle_status(
                        KnowledgeExposureLifecycleStatus::parse(&lifecycle_status)?,
                    );
                }
                if let Some(source_status) = source_status {
                    options = options
                        .with_source_status(KnowledgeExposureSourceStatus::parse(&source_status)?);
                }
                let result = engine.knowledge_exposures(options)?;
                render_knowledge_exposure_list(&result)
            }
        },
        Command::History {
            store,
            branch,
            commit,
            limit,
        } => {
            let engine = Engine::open(store)?;
            let mut options = match (branch, commit) {
                (Some(branch_id), None) => {
                    HistoryQueryOptions::from_branch(BranchId::parse_canonical(&branch_id)?)
                }
                (None, Some(commit_id)) => {
                    HistoryQueryOptions::from_commit(CommitId::parse_canonical(&commit_id)?)
                }
                _ => {
                    return Err(WorkVcsError::QueryInvalid(
                        "history requires exactly one of --branch or --commit".to_owned(),
                    ));
                }
            };
            if let Some(limit) = limit {
                options = options.with_limit(limit)?;
            }
            let history = engine.history(options)?;
            let mut output = format!(
                "start_commit_id={}\nentries={}\n",
                history.start_commit_id,
                history.entries.len()
            );
            for entry in &history.entries {
                render_history_entry(&mut output, entry);
            }
            Ok(output)
        }
        Command::Changeset { command } => match command {
            ChangeSetCommand::Show { store, changeset } => {
                let engine = Engine::open(store)?;
                let snapshot = engine.changeset(ChangeSetId::parse_canonical(&changeset)?)?;
                Ok(render_changeset_snapshot(&snapshot))
            }
            ChangeSetCommand::Operations { store, changeset } => {
                let engine = Engine::open(store)?;
                let result =
                    engine.changeset_operations(ChangeSetId::parse_canonical(&changeset)?)?;
                Ok(render_change_operations(&result))
            }
            ChangeSetCommand::Anchors { store, changeset } => {
                let engine = Engine::open(store)?;
                let result =
                    engine.changeset_causal_anchors(ChangeSetId::parse_canonical(&changeset)?)?;
                Ok(render_changeset_causal_anchors(&result))
            }
        },
        Command::Commit { command } => match command {
            CommitCommand::Show { store, commit } => {
                let engine = Engine::open(store)?;
                let snapshot = engine.commit(CommitId::parse_canonical(&commit)?)?;
                Ok(render_commit_snapshot(&snapshot))
            }
        },
        Command::Event { command } => match command {
            EventCommand::Show { store, event } => {
                let engine = Engine::open(store)?;
                let snapshot = engine.event(EventId::parse_canonical(&event)?)?;
                Ok(render_event_snapshot(&snapshot))
            }
            EventCommand::List {
                store,
                changeset,
                session,
                workspace,
                kind,
                limit,
            } => {
                let engine = Engine::open(store)?;
                let mut options = match (changeset, session, workspace) {
                    (Some(changeset), None, None) => {
                        EventListOptions::for_changeset(ChangeSetId::parse_canonical(&changeset)?)
                    }
                    (None, Some(session), None) => {
                        EventListOptions::for_session(SessionId::parse_canonical(&session)?)
                    }
                    (None, None, Some(workspace)) => EventListOptions::for_workspace(
                        workvcs_core::WorkspaceId::parse_canonical(&workspace)?,
                    ),
                    _ => {
                        return Err(WorkVcsError::QueryInvalid(
                            "event list requires exactly one of --changeset, --session, or --workspace"
                                .to_owned(),
                        ));
                    }
                };
                if matches!(limit, Some(0)) {
                    return Err(WorkVcsError::QueryInvalid(
                        "event list limit must be greater than zero".to_owned(),
                    ));
                }
                if kind.is_none()
                    && let Some(limit) = limit
                {
                    options = options.with_limit(limit)?;
                }
                let mut result = engine.events(options)?;
                if let Some(kind) = kind {
                    result.events.retain(|event| event.event_kind == kind);
                    if let Some(limit) = limit {
                        result.events.truncate(limit);
                    }
                }
                Ok(render_event_list(&result))
            }
        },
        Command::ShowAt {
            store,
            branch,
            commit,
        } => {
            let engine = Engine::open(store)?;
            let commit_id = resolve_show_at_commit(&engine, branch, commit)?;
            let state = engine.show_at(commit_id)?;
            Ok(render_replayed_state(&state))
        }
        Command::Diff {
            store,
            from_branch,
            from_commit,
            to_branch,
            to_commit,
            target_kind,
            change_kind,
            entity,
            relation,
        } => {
            let engine = Engine::open(store)?;
            let from = work_state_diff_target_from_cli("from", from_branch, from_commit)?;
            let to = work_state_diff_target_from_cli("to", to_branch, to_commit)?;
            let mut diff = engine.diff(WorkStateDiffOptions::new(from, to))?;
            if let Some(target_kind) = target_kind {
                filter_work_state_diff_target_kind(&mut diff, &target_kind)?;
            }
            if let Some(change_kind) = change_kind {
                let change_kind = parse_work_state_diff_change_kind(&change_kind)?;
                diff.entity_changes
                    .retain(|change| change.change_kind == change_kind);
                diff.relation_changes
                    .retain(|change| change.change_kind == change_kind);
            }
            if let Some(entity) = entity {
                let entity_id = EntityId::parse_canonical(&entity)?;
                diff.entity_changes
                    .retain(|change| change.entity_id == entity_id);
                diff.relation_changes.clear();
            }
            if let Some(relation) = relation {
                let relation_id = RelationId::parse_canonical(&relation)?;
                diff.relation_changes
                    .retain(|change| change.relation_id == relation_id);
                diff.entity_changes.clear();
            }
            Ok(render_work_state_diff(&diff))
        }
        Command::Entity { command } => match command {
            EntityCommand::Create {
                store,
                branch,
                head,
                kind,
                state_json,
                rationale_json,
            } => {
                let mut engine = Engine::open(store)?;
                let options = EntityTransitionOptions::create(
                    BranchId::parse_canonical(&branch)?,
                    CommitId::parse_canonical(&head)?,
                    kind,
                    parse_canonical_json(state_json.as_bytes())?,
                )?
                .with_rationale(parse_cli_object("entity rationale", &rationale_json)?);
                Ok(render_entity_transition_commit(
                    &engine.commit_entity_transition(options)?,
                ))
            }
            EntityCommand::Update {
                store,
                branch,
                head,
                entity,
                entity_version,
                state_json,
                rationale_json,
            } => {
                let mut engine = Engine::open(store)?;
                let options = EntityTransitionOptions::update(
                    BranchId::parse_canonical(&branch)?,
                    CommitId::parse_canonical(&head)?,
                    EntityId::parse_canonical(&entity)?,
                    EntityVersionId::parse_canonical(&entity_version)?,
                    parse_canonical_json(state_json.as_bytes())?,
                )?
                .with_rationale(parse_cli_object("entity rationale", &rationale_json)?);
                Ok(render_entity_transition_commit(
                    &engine.commit_entity_transition(options)?,
                ))
            }
        },
        Command::Reference { command } => match command {
            ReferenceCommand::Create {
                store,
                branch,
                head,
                referrer,
                target,
                rationale_json,
            } => {
                let mut engine = Engine::open(store)?;
                let options = StructuralReferenceCreateOptions::new(
                    BranchId::parse_canonical(&branch)?,
                    CommitId::parse_canonical(&head)?,
                    EntityId::parse_canonical(&referrer)?,
                    EntityId::parse_canonical(&target)?,
                )?
                .with_rationale(parse_cli_object(
                    "structural reference rationale",
                    &rationale_json,
                )?);
                Ok(render_structural_reference_create(
                    &engine.create_structural_reference(options)?,
                ))
            }
            ReferenceCommand::List {
                store,
                branch,
                commit,
                referrer,
                target,
                referrer_kind,
                target_kind,
            } => {
                let engine = Engine::open(&store)?;
                let commit_id = resolve_reference_query_commit(&engine, branch, commit)?;
                let mut references = engine.structural_references_at(commit_id)?;
                if let Some(referrer) = referrer {
                    let referrer_id = EntityId::parse_canonical(&referrer)?;
                    references.retain(|reference| reference.referrer_entity_id == referrer_id);
                }
                if let Some(target) = target {
                    let target_id = EntityId::parse_canonical(&target)?;
                    references.retain(|reference| reference.target_entity_id == target_id);
                }
                if let Some(referrer_kind) = referrer_kind {
                    let referrer_kind = parse_goal_plan_task_endpoint_kind(&referrer_kind)?;
                    references
                        .retain(|reference| reference.referrer_kind.as_str() == referrer_kind);
                }
                if let Some(target_kind) = target_kind {
                    let target_kind = parse_goal_plan_task_endpoint_kind(&target_kind)?;
                    references.retain(|reference| reference.target_kind.as_str() == target_kind);
                }
                Ok(render_structural_reference_list(&references))
            }
        },
        Command::Restore {
            store,
            branch,
            head,
            target_commit,
            rationale_json,
        } => {
            let mut engine = Engine::open(store)?;
            let options = WorkStateRestoreOptions::new(
                BranchId::parse_canonical(&branch)?,
                CommitId::parse_canonical(&head)?,
                CommitId::parse_canonical(&target_commit)?,
            )?
            .with_rationale(parse_cli_object("restore rationale", &rationale_json)?)?;
            Ok(render_work_state_restore(
                &engine.restore_work_state(options)?,
            ))
        }
        Command::Projection { command } => match command {
            ProjectionCommand::Refresh { store, branch } => {
                let mut engine = Engine::open(store)?;
                let result = engine.refresh_branch_projection(
                    BranchProjectionRefreshOptions::new(BranchId::parse_canonical(&branch)?),
                )?;
                Ok(render_branch_projection_refresh(&result))
            }
            ProjectionCommand::Show { store, branch } => {
                let engine = Engine::open(store)?;
                let snapshot = engine.branch_projection(BranchId::parse_canonical(&branch)?)?;
                Ok(render_branch_projection_snapshot(&snapshot))
            }
        },
        Command::Bundle { command } => match command {
            BundleCommand::Export { store, commit } => {
                let engine = Engine::open(store)?;
                let manifest = engine.export_bundle_manifest(BundleExportOptions::for_commit(
                    CommitId::parse_canonical(&commit)?,
                ))?;
                Ok(render_bundle_export_manifest(&manifest))
            }
            BundleCommand::ExportJson { store, commit } => {
                let engine = Engine::open(store)?;
                let manifest = engine.export_bundle_manifest(BundleExportOptions::for_commit(
                    CommitId::parse_canonical(&commit)?,
                ))?;
                render_bundle_export_manifest_json(&manifest)
            }
            BundleCommand::ExportDir {
                store,
                commit,
                output_dir,
            } => {
                let engine = Engine::open(store)?;
                let export = engine.export_bundle_payloads(
                    BundlePayloadExportOptions::for_commit(CommitId::parse_canonical(&commit)?),
                )?;
                write_bundle_payload_export_directory(&output_dir, &export)?;
                Ok(render_bundle_payload_export(&export, &output_dir))
            }
            BundleCommand::ValidateDir {
                store,
                commit,
                input_dir,
            } => {
                let engine = Engine::open(store)?;
                let manifest_bytes = read_bundle_file(&input_dir.join("manifest.json"))?;
                let payload_index_bytes = read_bundle_file(&input_dir.join("payload-index.json"))?;
                let payloads = read_bundle_payload_inputs(&input_dir)?;
                let validation =
                    engine.validate_bundle_payloads(BundlePayloadValidationOptions::from_parts(
                        CommitId::parse_canonical(&commit)?,
                        manifest_bytes,
                        payload_index_bytes,
                        payloads,
                    )?)?;
                Ok(render_bundle_payload_validation(&validation))
            }
            BundleCommand::PreflightDir { store, input_dir } => {
                let engine = Engine::open(store)?;
                let manifest_bytes = read_bundle_file(&input_dir.join("manifest.json"))?;
                let payload_index_bytes = read_bundle_file(&input_dir.join("payload-index.json"))?;
                let payloads = read_bundle_payload_inputs(&input_dir)?;
                let preflight =
                    engine.preflight_bundle_import(BundleImportPreflightOptions::from_parts(
                        manifest_bytes,
                        payload_index_bytes,
                        payloads,
                    )?)?;
                Ok(render_bundle_import_preflight(&preflight))
            }
            BundleCommand::ApplyDir { store, input_dir } => {
                let mut engine = Engine::open(store)?;
                let manifest_bytes = read_bundle_file(&input_dir.join("manifest.json"))?;
                let payload_index_bytes = read_bundle_file(&input_dir.join("payload-index.json"))?;
                let payloads = read_bundle_payload_inputs(&input_dir)?;
                let result = engine.apply_bundle_import(BundleImportApplyOptions::from_parts(
                    manifest_bytes,
                    payload_index_bytes,
                    payloads,
                )?)?;
                Ok(render_bundle_import_apply(&result))
            }
            BundleCommand::ImportDir { store, input_dir } => {
                let mut engine = Engine::open(store)?;
                let manifest_bytes = read_bundle_file(&input_dir.join("manifest.json"))?;
                let payload_index_bytes = read_bundle_file(&input_dir.join("payload-index.json"))?;
                let payloads = read_bundle_payload_inputs(&input_dir)?;
                let result =
                    engine.record_bundle_import_attempt(BundleImportAttemptOptions::from_parts(
                        manifest_bytes,
                        payload_index_bytes,
                        payloads,
                    )?)?;
                Ok(render_bundle_import_attempt(&result))
            }
            BundleCommand::ImportShow { store, import } => {
                let engine = Engine::open(store)?;
                let snapshot = engine.bundle_import_attempt(ImportId::parse_canonical(&import)?)?;
                Ok(render_bundle_import_attempt_snapshot(&snapshot))
            }
            BundleCommand::ImportList {
                store,
                limit,
                source_store,
                bundle_digest,
            } => {
                let engine = Engine::open(store)?;
                let mut options = BundleImportAttemptListOptions::new();
                if let Some(limit) = limit {
                    options = options.with_limit(limit)?;
                }
                if let Some(source_store) = source_store {
                    options =
                        options.with_source_store_id(StoreId::parse_canonical(&source_store)?);
                }
                if let Some(bundle_digest) = bundle_digest {
                    options = options.with_bundle_digest(Digest::from_hex(&bundle_digest)?);
                }
                let result = engine.bundle_import_attempts(options)?;
                Ok(render_bundle_import_attempt_list(&result))
            }
            BundleCommand::ValidateManifest {
                store,
                commit,
                manifest_file,
            } => {
                let engine = Engine::open(store)?;
                let manifest_bytes = fs::read(&manifest_file).map_err(|error| {
                    WorkVcsError::QueryInvalid(format!(
                        "failed to read bundle manifest file {}: {error}",
                        manifest_file.display()
                    ))
                })?;
                let validation = engine.validate_bundle_manifest(
                    BundleManifestValidationOptions::from_bytes(
                        CommitId::parse_canonical(&commit)?,
                        manifest_bytes,
                    )?,
                )?;
                Ok(render_bundle_manifest_validation(&validation))
            }
        },
        Command::Checkpoint { command } => match command {
            CheckpointCommand::Create { store, commit } => {
                let mut engine = Engine::open(store)?;
                let result = engine.create_checkpoint(CheckpointCreateOptions::new(
                    CommitId::parse_canonical(&commit)?,
                ))?;
                Ok(render_checkpoint_create(&result))
            }
            CheckpointCommand::Show { store, checkpoint } => {
                let engine = Engine::open(store)?;
                let snapshot = engine.checkpoint(CheckpointId::parse_canonical(&checkpoint)?)?;
                Ok(render_checkpoint_snapshot(&snapshot))
            }
            CheckpointCommand::Validate { store, checkpoint } => {
                let mut engine = Engine::open(store)?;
                let result =
                    engine.validate_checkpoint(CheckpointId::parse_canonical(&checkpoint)?)?;
                Ok(render_checkpoint_validation(&result))
            }
            CheckpointCommand::List {
                store,
                commit,
                usability_state,
                content_digest,
            } => {
                let engine = Engine::open(store)?;
                let mut result = engine.checkpoints(CheckpointListOptions::for_commit(
                    CommitId::parse_canonical(&commit)?,
                ))?;
                if let Some(usability_state) = usability_state {
                    result
                        .checkpoints
                        .retain(|checkpoint| checkpoint.usability_state == usability_state);
                }
                if let Some(content_digest) = content_digest {
                    let content_digest = Digest::from_hex(&content_digest)?;
                    result
                        .checkpoints
                        .retain(|checkpoint| checkpoint.content_digest == content_digest);
                }
                Ok(render_checkpoint_list(&result))
            }
            CheckpointCommand::Latest { store, commit } => {
                let engine = Engine::open(store)?;
                let result = engine.latest_usable_checkpoint(
                    CheckpointLatestOptions::usable_for_commit(CommitId::parse_canonical(&commit)?),
                )?;
                Ok(render_checkpoint_latest(&result))
            }
        },
        Command::Why {
            store,
            branch,
            commit,
            entity,
            evidence,
            exposure,
        } => {
            let engine = Engine::open(store)?;
            let target = match (branch, commit) {
                (Some(branch_id), None) => {
                    WhyQueryTarget::branch_head(BranchId::parse_canonical(&branch_id)?)
                }
                (None, Some(commit_id)) => {
                    WhyQueryTarget::commit(CommitId::parse_canonical(&commit_id)?)
                }
                _ => {
                    return Err(WorkVcsError::QueryInvalid(
                        "why requires exactly one of --branch or --commit".to_owned(),
                    ));
                }
            };
            let options = match (entity, evidence, exposure) {
                (Some(entity_id), None, None) => {
                    WhyQueryOptions::for_entity(target, EntityId::parse_canonical(&entity_id)?)
                }
                (None, Some(evidence_id), None) => WhyQueryOptions::for_evidence(
                    target,
                    EvidenceId::parse_canonical(&evidence_id)?,
                ),
                (None, None, Some(exposure_id)) => WhyQueryOptions::for_knowledge_exposure(
                    target,
                    ExposureId::parse_canonical(&exposure_id)?,
                ),
                _ => {
                    return Err(WorkVcsError::QueryInvalid(
                        "why requires exactly one of --entity, --evidence, or --exposure"
                            .to_owned(),
                    ));
                }
            };
            Ok(render_why(&engine.why(options)?))
        }
        Command::Workspace {
            command:
                WorkspaceCommand::Create {
                    store,
                    display_name,
                    initial_branch_name,
                },
        } => {
            let mut engine = Engine::open(store)?;
            let mut options = WorkspaceInitOptions::new(display_name)?;
            if let Some(initial_branch_name) = initial_branch_name {
                options = options.with_initial_branch_name(initial_branch_name)?;
            }
            let workspace = engine.create_workspace(options)?;
            Ok(render_workspace_info(&workspace))
        }
        Command::Workspace {
            command: WorkspaceCommand::Show { store, workspace },
        } => {
            let engine = Engine::open(store)?;
            let workspace =
                engine.workspace_info(workvcs_core::WorkspaceId::parse_canonical(&workspace)?)?;
            Ok(render_workspace_info(&workspace))
        }
        Command::Workspace {
            command:
                WorkspaceCommand::List {
                    store,
                    display_name,
                    limit,
                },
        } => {
            let engine = Engine::open(store)?;
            let mut result = engine.workspaces(WorkspaceListOptions::all())?;
            if let Some(display_name) = display_name {
                result
                    .workspaces
                    .retain(|workspace| workspace.display_name == display_name);
            }
            if matches!(limit, Some(0)) {
                return Err(WorkVcsError::QueryInvalid(
                    "workspace list limit must be greater than zero".to_owned(),
                ));
            }
            if let Some(limit) = limit {
                result.workspaces.truncate(limit);
            }
            Ok(render_workspace_list(&result))
        }
        Command::Branch {
            command:
                BranchCommand::List {
                    store,
                    workspace,
                    name,
                    lifecycle_state,
                    limit,
                },
        } => {
            let engine = Engine::open(store)?;
            let mut branches =
                engine.list_branches(workvcs_core::WorkspaceId::parse_canonical(&workspace)?)?;
            if let Some(name) = name {
                branches.retain(|branch| branch.name == name);
            }
            if let Some(lifecycle_state) = lifecycle_state {
                branches.retain(|branch| branch.lifecycle_state == lifecycle_state);
            }
            if matches!(limit, Some(0)) {
                return Err(WorkVcsError::QueryInvalid(
                    "branch list limit must be greater than zero".to_owned(),
                ));
            }
            if let Some(limit) = limit {
                branches.truncate(limit);
            }
            Ok(render_branch_list(&branches))
        }
        Command::Branch {
            command: BranchCommand::Head { store, branch },
        } => {
            let engine = Engine::open(store)?;
            let head = engine.branch_head(BranchId::parse_canonical(&branch)?)?;
            Ok(render_branch_head(&head))
        }
        Command::Branch {
            command:
                BranchCommand::Fork {
                    store,
                    from_branch,
                    from_commit,
                    name,
                },
        } => {
            let mut engine = Engine::open(store)?;
            let options = match (from_branch, from_commit) {
                (Some(branch), None) => {
                    BranchForkOptions::from_branch(BranchId::parse_canonical(&branch)?, name)?
                }
                (None, Some(commit)) => {
                    BranchForkOptions::from_commit(CommitId::parse_canonical(&commit)?, name)?
                }
                _ => {
                    return Err(WorkVcsError::WorkspaceInvalid(
                        "branch fork requires exactly one source".to_owned(),
                    ));
                }
            };
            let forked = engine.fork_branch(options)?;
            Ok(render_branch_fork(&forked))
        }
        Command::Knowledge {
            command:
                KnowledgeCommand::Create {
                    store,
                    branch,
                    head,
                    statement,
                    scope_json,
                    provenance_json,
                },
        } => {
            let mut engine = Engine::open(store)?;
            let mut options = KnowledgeCreateOptions::new(
                BranchId::parse_canonical(&branch)?,
                CommitId::parse_canonical(&head)?,
                statement,
            )?;
            if let Some(scope_json) = scope_json {
                options = options.with_scope(parse_cli_object("knowledge scope", &scope_json)?)?;
            }
            if let Some(provenance_json) = provenance_json {
                options = options
                    .with_provenance(parse_cli_object("knowledge provenance", &provenance_json)?)?;
            }
            Ok(render_knowledge_create(&engine.create_knowledge(options)?)?)
        }
        Command::Knowledge {
            command:
                KnowledgeCommand::Show {
                    store,
                    branch,
                    commit,
                    knowledge,
                },
        } => {
            let engine = Engine::open(store)?;
            let commit_id = resolve_knowledge_query_commit(&engine, branch, commit)?;
            Ok(render_knowledge_snapshot(&engine.knowledge_at(
                commit_id,
                EntityId::parse_canonical(&knowledge)?,
            )?)?)
        }
        Command::Knowledge {
            command:
                KnowledgeCommand::List {
                    store,
                    branch,
                    commit,
                    status,
                    scope_json,
                    statement_contains,
                },
        } => {
            let engine = Engine::open(store)?;
            let mut options =
                KnowledgeListOptions::new(resolve_knowledge_query_commit(&engine, branch, commit)?);
            if let Some(status) = status {
                options = options.with_status(parse_knowledge_status(&status)?);
            }
            if let Some(scope_json) = scope_json {
                options = options.with_scope(parse_cli_object("knowledge scope", &scope_json)?)?;
            }
            if let Some(statement_contains) = statement_contains {
                options = options.with_statement_contains(statement_contains)?;
            }
            Ok(render_knowledge_list(&engine.knowledges_at(options)?)?)
        }
        Command::Knowledge {
            command:
                KnowledgeCommand::Invalidate {
                    store,
                    branch,
                    head,
                    knowledge,
                    knowledge_version,
                    rationale,
                },
        } => {
            let mut engine = Engine::open(store)?;
            let options = KnowledgeTransitionOptions::invalidate(
                BranchId::parse_canonical(&branch)?,
                CommitId::parse_canonical(&head)?,
                EntityId::parse_canonical(&knowledge)?,
                EntityVersionId::parse_canonical(&knowledge_version)?,
                rationale,
            )?;
            Ok(render_knowledge_transition(
                &engine.transition_knowledge(options)?,
            )?)
        }
        Command::Knowledge {
            command:
                KnowledgeCommand::Supersede {
                    store,
                    branch,
                    head,
                    knowledge,
                    knowledge_version,
                    rationale,
                },
        } => {
            let mut engine = Engine::open(store)?;
            let options = KnowledgeTransitionOptions::supersede(
                BranchId::parse_canonical(&branch)?,
                CommitId::parse_canonical(&head)?,
                EntityId::parse_canonical(&knowledge)?,
                EntityVersionId::parse_canonical(&knowledge_version)?,
                rationale,
            )?;
            Ok(render_knowledge_transition(
                &engine.transition_knowledge(options)?,
            )?)
        }
        Command::Knowledge {
            command:
                KnowledgeCommand::LinkSupersedes {
                    store,
                    branch,
                    head,
                    replacement_knowledge,
                    prior_knowledge,
                    rationale,
                },
        } => {
            let mut engine = Engine::open(store)?;
            let relation =
                engine.create_knowledge_relation(KnowledgeRelationCreateOptions::supersedes(
                    BranchId::parse_canonical(&branch)?,
                    CommitId::parse_canonical(&head)?,
                    EntityId::parse_canonical(&replacement_knowledge)?,
                    EntityId::parse_canonical(&prior_knowledge)?,
                    rationale,
                )?)?;
            Ok(render_knowledge_relation_create(&relation))
        }
        Command::Knowledge {
            command:
                KnowledgeCommand::RelationList {
                    store,
                    branch,
                    commit,
                    replacement_knowledge,
                    prior_knowledge,
                },
        } => {
            let engine = Engine::open(store)?;
            let mut options = KnowledgeRelationListOptions::new(resolve_knowledge_query_commit(
                &engine, branch, commit,
            )?);
            if let Some(replacement_knowledge) = replacement_knowledge {
                options = options
                    .with_replacement_knowledge(EntityId::parse_canonical(&replacement_knowledge)?);
            }
            if let Some(prior_knowledge) = prior_knowledge {
                options =
                    options.with_prior_knowledge(EntityId::parse_canonical(&prior_knowledge)?);
            }
            Ok(render_knowledge_relation_list(
                &engine.knowledge_relations_at(options)?,
            ))
        }
        Command::Knowledge {
            command:
                KnowledgeCommand::RelationShow {
                    store,
                    branch,
                    commit,
                    relation,
                },
        } => {
            let engine = Engine::open(store)?;
            let commit_id = resolve_knowledge_query_commit(&engine, branch, commit)?;
            Ok(render_knowledge_relation_snapshot(
                &engine
                    .knowledge_relation_at(commit_id, RelationId::parse_canonical(&relation)?)?,
            ))
        }
        Command::Knowledge {
            command:
                KnowledgeCommand::RelationRemove {
                    store,
                    branch,
                    head,
                    relation,
                    relation_version,
                    rationale,
                },
        } => {
            let mut engine = Engine::open(store)?;
            let removed = engine.remove_knowledge_relation(KnowledgeRelationRemoveOptions::new(
                BranchId::parse_canonical(&branch)?,
                CommitId::parse_canonical(&head)?,
                RelationId::parse_canonical(&relation)?,
                RelationVersionId::parse_canonical(&relation_version)?,
                rationale,
            )?)?;
            Ok(render_knowledge_relation_remove(&removed))
        }
        Command::Knowledge {
            command:
                KnowledgeCommand::RelationRestore {
                    store,
                    branch,
                    head,
                    relation,
                    relation_version,
                    rationale,
                },
        } => {
            let mut engine = Engine::open(store)?;
            let restored =
                engine.restore_knowledge_relation(KnowledgeRelationRestoreOptions::new(
                    BranchId::parse_canonical(&branch)?,
                    CommitId::parse_canonical(&head)?,
                    RelationId::parse_canonical(&relation)?,
                    RelationVersionId::parse_canonical(&relation_version)?,
                    rationale,
                )?)?;
            Ok(render_knowledge_relation_restore(&restored))
        }
        Command::Goal { command } => match command {
            GoalCommand::Create {
                store,
                branch,
                head,
                description,
            } => {
                let mut engine = Engine::open(store)?;
                let goal = engine.create_goal(GoalCreateOptions::new(
                    BranchId::parse_canonical(&branch)?,
                    CommitId::parse_canonical(&head)?,
                    description,
                )?)?;
                Ok(render_goal_create(&goal))
            }
            GoalCommand::Show {
                store,
                branch,
                commit,
                goal,
            } => {
                let engine = Engine::open(store)?;
                let commit_id = resolve_goal_query_commit(&engine, branch, commit)?;
                render_goal_snapshot(&engine.goal_at(commit_id, EntityId::parse_canonical(&goal)?)?)
            }
            GoalCommand::List {
                store,
                branch,
                commit,
                status,
                limit,
            } => {
                let engine = Engine::open(store)?;
                let commit_id = resolve_goal_query_commit(&engine, branch, commit)?;
                let mut goals = engine.goals_at(commit_id)?;
                if let Some(status) = status {
                    let status = parse_goal_list_status(&status)?;
                    goals.retain(|goal| goal.state.status == status);
                }
                if matches!(limit, Some(0)) {
                    return Err(WorkVcsError::QueryInvalid(
                        "goal list limit must be greater than zero".to_owned(),
                    ));
                }
                if let Some(limit) = limit {
                    goals.truncate(limit);
                }
                render_goal_list(commit_id, &goals)
            }
            GoalCommand::Achieve {
                store,
                branch,
                head,
                goal,
                goal_version,
                rationale,
            } => {
                let mut engine = Engine::open(store)?;
                let transition = engine.transition_goal(GoalTransitionOptions::achieve(
                    BranchId::parse_canonical(&branch)?,
                    CommitId::parse_canonical(&head)?,
                    EntityId::parse_canonical(&goal)?,
                    EntityVersionId::parse_canonical(&goal_version)?,
                    rationale,
                )?)?;
                Ok(render_goal_transition(&transition))
            }
            GoalCommand::Abandon {
                store,
                branch,
                head,
                goal,
                goal_version,
                rationale,
            } => {
                let mut engine = Engine::open(store)?;
                let transition = engine.transition_goal(GoalTransitionOptions::abandon(
                    BranchId::parse_canonical(&branch)?,
                    CommitId::parse_canonical(&head)?,
                    EntityId::parse_canonical(&goal)?,
                    EntityVersionId::parse_canonical(&goal_version)?,
                    rationale,
                )?)?;
                Ok(render_goal_transition(&transition))
            }
            GoalCommand::Reopen {
                store,
                branch,
                head,
                goal,
                goal_version,
                rationale,
            } => {
                let mut engine = Engine::open(store)?;
                let transition = engine.transition_goal(GoalTransitionOptions::reopen(
                    BranchId::parse_canonical(&branch)?,
                    CommitId::parse_canonical(&head)?,
                    EntityId::parse_canonical(&goal)?,
                    EntityVersionId::parse_canonical(&goal_version)?,
                    rationale,
                )?)?;
                Ok(render_goal_transition(&transition))
            }
        },
        Command::Plan { command } => match command {
            PlanCommand::Create {
                store,
                branch,
                head,
                description,
                strategy,
                constraints,
            } => {
                let mut engine = Engine::open(store)?;
                let mut options = PlanCreateOptions::new(
                    BranchId::parse_canonical(&branch)?,
                    CommitId::parse_canonical(&head)?,
                    description,
                    strategy,
                )?;
                if !constraints.is_empty() {
                    options = options.with_constraints(constraints)?;
                }
                let plan = engine.create_plan(options)?;
                Ok(render_plan_create(&plan))
            }
            PlanCommand::Show {
                store,
                branch,
                commit,
                plan,
            } => {
                let engine = Engine::open(store)?;
                let commit_id = resolve_plan_query_commit(&engine, branch, commit)?;
                render_plan_snapshot(&engine.plan_at(commit_id, EntityId::parse_canonical(&plan)?)?)
            }
            PlanCommand::List {
                store,
                branch,
                commit,
                status,
                limit,
            } => {
                let engine = Engine::open(store)?;
                let commit_id = resolve_plan_query_commit(&engine, branch, commit)?;
                let mut plans = engine.plans_at(commit_id)?;
                if let Some(status) = status {
                    let status = parse_plan_list_status(&status)?;
                    plans.retain(|plan| plan.state.status == status);
                }
                if matches!(limit, Some(0)) {
                    return Err(WorkVcsError::QueryInvalid(
                        "plan list limit must be greater than zero".to_owned(),
                    ));
                }
                if let Some(limit) = limit {
                    plans.truncate(limit);
                }
                render_plan_list(commit_id, &plans)
            }
            PlanCommand::Complete {
                store,
                branch,
                head,
                plan,
                plan_version,
                completion_rationale,
            } => {
                let mut engine = Engine::open(store)?;
                let mut options = PlanTransitionOptions::complete(
                    BranchId::parse_canonical(&branch)?,
                    CommitId::parse_canonical(&head)?,
                    EntityId::parse_canonical(&plan)?,
                    EntityVersionId::parse_canonical(&plan_version)?,
                )?;
                if let Some(completion_rationale) = completion_rationale {
                    options = options.with_completion_rationale(completion_rationale)?;
                }
                let transition = engine.transition_plan(options)?;
                Ok(render_plan_transition(&transition))
            }
            PlanCommand::Abandon {
                store,
                branch,
                head,
                plan,
                plan_version,
                rationale,
            } => {
                let mut engine = Engine::open(store)?;
                let transition = engine.transition_plan(PlanTransitionOptions::abandon(
                    BranchId::parse_canonical(&branch)?,
                    CommitId::parse_canonical(&head)?,
                    EntityId::parse_canonical(&plan)?,
                    EntityVersionId::parse_canonical(&plan_version)?,
                    rationale,
                )?)?;
                Ok(render_plan_transition(&transition))
            }
            PlanCommand::Reopen {
                store,
                branch,
                head,
                plan,
                plan_version,
                rationale,
            } => {
                let mut engine = Engine::open(store)?;
                let transition = engine.transition_plan(PlanTransitionOptions::reopen(
                    BranchId::parse_canonical(&branch)?,
                    CommitId::parse_canonical(&head)?,
                    EntityId::parse_canonical(&plan)?,
                    EntityVersionId::parse_canonical(&plan_version)?,
                    rationale,
                )?)?;
                Ok(render_plan_transition(&transition))
            }
        },
        Command::Task {
            command:
                TaskCommand::Create {
                    store,
                    branch,
                    head,
                    description,
                    priority,
                },
        } => {
            let mut engine = Engine::open(store)?;
            let mut options = TaskCreateOptions::new(
                BranchId::parse_canonical(&branch)?,
                CommitId::parse_canonical(&head)?,
                description,
            )?;
            if let Some(priority) = priority {
                options = options.with_priority(priority)?;
            }
            let task = engine.create_task(options)?;
            Ok(render_task_create(&task))
        }
        Command::Task {
            command:
                TaskCommand::Show {
                    store,
                    branch,
                    commit,
                    task,
                },
        } => {
            let engine = Engine::open(store)?;
            let commit_id = resolve_task_query_commit(&engine, branch, commit)?;
            render_task_snapshot(&engine.task_at(commit_id, EntityId::parse_canonical(&task)?)?)
        }
        Command::Task {
            command:
                TaskCommand::List {
                    store,
                    branch,
                    commit,
                    status,
                    limit,
                },
        } => {
            let engine = Engine::open(store)?;
            let commit_id = resolve_task_query_commit(&engine, branch, commit)?;
            let mut tasks = engine.tasks_at(commit_id)?;
            if let Some(status) = status {
                let status = parse_task_list_status(&status)?;
                tasks.retain(|task| task.state.status == status);
            }
            if matches!(limit, Some(0)) {
                return Err(WorkVcsError::QueryInvalid(
                    "task list limit must be greater than zero".to_owned(),
                ));
            }
            if let Some(limit) = limit {
                tasks.truncate(limit);
            }
            render_task_list(commit_id, &tasks)
        }
        Command::Task {
            command:
                TaskCommand::Transition {
                    store,
                    branch,
                    head,
                    task,
                    task_version,
                    status,
                    outcome,
                    session,
                },
        } => {
            let mut engine = Engine::open(store)?;
            let mut options = TaskTransitionOptions::new(
                BranchId::parse_canonical(&branch)?,
                CommitId::parse_canonical(&head)?,
                EntityId::parse_canonical(&task)?,
                EntityVersionId::parse_canonical(&task_version)?,
                parse_task_status(&status)?,
            )?;
            if let Some(outcome) = outcome {
                options = options.with_outcome(outcome)?;
            }
            if let Some(session) = session {
                options = options.with_actor_session(SessionId::parse_canonical(&session)?);
            }
            let transition = engine.transition_task(options)?;
            Ok(render_task_transition(&transition))
        }
        Command::Task {
            command:
                TaskCommand::DependsOn {
                    store,
                    branch,
                    head,
                    task,
                    depends_on,
                    session,
                },
        } => {
            let mut engine = Engine::open(store)?;
            let mut options = TaskSchedulingRelationCreateOptions::depends_on(
                BranchId::parse_canonical(&branch)?,
                CommitId::parse_canonical(&head)?,
                EntityId::parse_canonical(&task)?,
                EntityId::parse_canonical(&depends_on)?,
            )?;
            if let Some(session) = session {
                options = options.with_actor_session(SessionId::parse_canonical(&session)?);
            }
            let relation = engine.create_task_scheduling_relation(options)?;
            Ok(render_task_scheduling_relation_create(&relation))
        }
        Command::Task {
            command:
                TaskCommand::OrderedBefore {
                    store,
                    branch,
                    head,
                    earlier,
                    later,
                    session,
                },
        } => {
            let mut engine = Engine::open(store)?;
            let mut options = TaskSchedulingRelationCreateOptions::ordered_before(
                BranchId::parse_canonical(&branch)?,
                CommitId::parse_canonical(&head)?,
                EntityId::parse_canonical(&earlier)?,
                EntityId::parse_canonical(&later)?,
            )?;
            if let Some(session) = session {
                options = options.with_actor_session(SessionId::parse_canonical(&session)?);
            }
            let relation = engine.create_task_scheduling_relation(options)?;
            Ok(render_task_scheduling_relation_create(&relation))
        }
        Command::Task {
            command:
                TaskCommand::Contain {
                    store,
                    branch,
                    head,
                    parent,
                    child,
                    session,
                },
        } => {
            let mut engine = Engine::open(store)?;
            let mut options = PrimaryContainmentCreateOptions::new(
                BranchId::parse_canonical(&branch)?,
                CommitId::parse_canonical(&head)?,
                EntityId::parse_canonical(&parent)?,
                EntityId::parse_canonical(&child)?,
            )?;
            if let Some(session) = session {
                options = options.with_actor_session(SessionId::parse_canonical(&session)?);
            }
            let relation = engine.create_primary_containment(options)?;
            Ok(render_primary_containment_create(&relation))
        }
        Command::Task {
            command:
                TaskCommand::SchedulingList {
                    store,
                    branch,
                    commit,
                    relation_type,
                    source_task,
                    target_task,
                },
        } => {
            let engine = Engine::open(store)?;
            let commit_id = resolve_task_query_commit(&engine, branch, commit)?;
            let mut relations = engine.task_scheduling_relations_at(commit_id)?;
            if let Some(relation_type) = relation_type {
                let relation_type = parse_task_scheduling_relation_type(&relation_type)?;
                relations.retain(|relation| relation.relation_type.as_str() == relation_type);
            }
            if let Some(source_task) = source_task {
                let source_task_id = EntityId::parse_canonical(&source_task)?;
                relations.retain(|relation| relation.source_task_entity_id == source_task_id);
            }
            if let Some(target_task) = target_task {
                let target_task_id = EntityId::parse_canonical(&target_task)?;
                relations.retain(|relation| relation.target_task_entity_id == target_task_id);
            }
            Ok(render_task_scheduling_relation_list(commit_id, &relations))
        }
        Command::Task {
            command:
                TaskCommand::ContainmentList {
                    store,
                    branch,
                    commit,
                    parent,
                    child,
                    parent_kind,
                    child_kind,
                },
        } => {
            let engine = Engine::open(store)?;
            let commit_id = resolve_task_query_commit(&engine, branch, commit)?;
            let mut relations = engine.primary_containment_relations_at(commit_id)?;
            if let Some(parent) = parent {
                let parent_id = EntityId::parse_canonical(&parent)?;
                relations.retain(|relation| relation.parent_entity_id == parent_id);
            }
            if let Some(child) = child {
                let child_id = EntityId::parse_canonical(&child)?;
                relations.retain(|relation| relation.child_entity_id == child_id);
            }
            if let Some(parent_kind) = parent_kind {
                let parent_kind = parse_goal_plan_task_endpoint_kind(&parent_kind)?;
                relations.retain(|relation| relation.parent_kind.as_str() == parent_kind);
            }
            if let Some(child_kind) = child_kind {
                let child_kind = parse_goal_plan_task_endpoint_kind(&child_kind)?;
                relations.retain(|relation| relation.child_kind.as_str() == child_kind);
            }
            Ok(render_primary_containment_list(commit_id, &relations))
        }
        Command::Ac {
            command:
                AcceptanceCriterionCommand::Create {
                    store,
                    branch,
                    head,
                    task,
                    task_version,
                    local_key,
                    statement,
                    classification,
                },
        } => {
            let mut engine = Engine::open(store)?;
            let criterion =
                engine.create_acceptance_criterion(AcceptanceCriterionCreateOptions::new(
                    BranchId::parse_canonical(&branch)?,
                    CommitId::parse_canonical(&head)?,
                    EntityId::parse_canonical(&task)?,
                    EntityVersionId::parse_canonical(&task_version)?,
                    local_key,
                    statement,
                    parse_acceptance_criterion_classification(&classification)?,
                )?)?;
            Ok(render_acceptance_criterion_create(&criterion))
        }
        Command::Ac {
            command:
                AcceptanceCriterionCommand::Revise {
                    store,
                    branch,
                    head,
                    criterion,
                    criterion_version,
                    statement,
                    classification,
                    rationale_json,
                },
        } => {
            let mut engine = Engine::open(store)?;
            let branch_id = BranchId::parse_canonical(&branch)?;
            let head_id = CommitId::parse_canonical(&head)?;
            let criterion_id = EntityId::parse_canonical(&criterion)?;
            let criterion_version_id = EntityVersionId::parse_canonical(&criterion_version)?;
            let classification = match classification {
                Some(classification) => parse_acceptance_criterion_classification(&classification)?,
                None => {
                    engine
                        .acceptance_criterion_at(head_id, criterion_id)?
                        .state
                        .classification
                }
            };
            let options = AcceptanceCriterionRevisionOptions::new(
                branch_id,
                head_id,
                criterion_id,
                criterion_version_id,
                statement,
                classification,
            )?
            .with_rationale(parse_cli_object(
                "acceptance criterion revision rationale",
                &rationale_json,
            )?);
            let revision = engine.revise_acceptance_criterion(options)?;
            render_acceptance_criterion_revision(&revision)
        }
        Command::Ac {
            command:
                AcceptanceCriterionCommand::Show {
                    store,
                    branch,
                    commit,
                    criterion,
                },
        } => {
            let engine = Engine::open(store)?;
            let commit_id = resolve_task_query_commit(&engine, branch, commit)?;
            render_acceptance_criterion_snapshot(
                &engine
                    .acceptance_criterion_at(commit_id, EntityId::parse_canonical(&criterion)?)?,
            )
        }
        Command::Ac {
            command:
                AcceptanceCriterionCommand::List {
                    store,
                    branch,
                    commit,
                    task,
                    classification,
                    limit,
                },
        } => {
            let engine = Engine::open(store)?;
            let commit_id = resolve_task_query_commit(&engine, branch, commit)?;
            let mut criteria = engine.acceptance_criteria_at(commit_id)?;
            if let Some(task) = task {
                let task_id = EntityId::parse_canonical(&task)?;
                criteria.retain(|criterion| criterion.task_entity_id == task_id);
            }
            if let Some(classification) = classification {
                let classification = parse_acceptance_criterion_classification(&classification)?;
                criteria.retain(|criterion| criterion.state.classification == classification);
            }
            if matches!(limit, Some(0)) {
                return Err(WorkVcsError::QueryInvalid(
                    "ac list limit must be greater than zero".to_owned(),
                ));
            }
            if let Some(limit) = limit {
                criteria.truncate(limit);
            }
            render_acceptance_criterion_list(commit_id, &criteria)
        }
        Command::Ac {
            command:
                AcceptanceCriterionCommand::Status {
                    store,
                    branch,
                    commit,
                    criterion,
                },
        } => {
            let engine = Engine::open(store)?;
            let criterion_id = EntityId::parse_canonical(&criterion)?;
            let status = match (branch, commit) {
                (Some(branch), None) => engine.acceptance_criterion_effective_status_for_branch(
                    BranchId::parse_canonical(&branch)?,
                    criterion_id,
                )?,
                (None, Some(commit)) => engine.acceptance_criterion_effective_status(
                    CommitId::parse_canonical(&commit)?,
                    criterion_id,
                )?,
                _ => {
                    return Err(WorkVcsError::QueryInvalid(
                        "ac status requires exactly one of --branch or --commit".to_owned(),
                    ));
                }
            };
            Ok(render_acceptance_criterion_status(status))
        }
        Command::Vr {
            command:
                VerificationRequirementCommand::Create {
                    store,
                    branch,
                    head,
                    criterion,
                    criterion_version,
                    local_key,
                    statement,
                },
        } => {
            let mut engine = Engine::open(store)?;
            let requirement = engine.create_verification_requirement(
                VerificationRequirementCreateOptions::new(
                    BranchId::parse_canonical(&branch)?,
                    CommitId::parse_canonical(&head)?,
                    EntityId::parse_canonical(&criterion)?,
                    EntityVersionId::parse_canonical(&criterion_version)?,
                    local_key,
                    statement,
                )?,
            )?;
            Ok(render_verification_requirement_create(&requirement))
        }
        Command::Vr {
            command:
                VerificationRequirementCommand::Revise {
                    store,
                    branch,
                    head,
                    requirement,
                    requirement_version,
                    statement,
                    rationale_json,
                },
        } => {
            let mut engine = Engine::open(store)?;
            let options = VerificationRequirementRevisionOptions::new(
                BranchId::parse_canonical(&branch)?,
                CommitId::parse_canonical(&head)?,
                EntityId::parse_canonical(&requirement)?,
                EntityVersionId::parse_canonical(&requirement_version)?,
                statement,
            )?
            .with_rationale(parse_cli_object(
                "verification requirement revision rationale",
                &rationale_json,
            )?);
            let revision = engine.revise_verification_requirement(options)?;
            render_verification_requirement_revision(&revision)
        }
        Command::Vr {
            command:
                VerificationRequirementCommand::Show {
                    store,
                    branch,
                    commit,
                    requirement,
                },
        } => {
            let engine = Engine::open(store)?;
            let commit_id = resolve_task_query_commit(&engine, branch, commit)?;
            render_verification_requirement_snapshot(
                &engine.verification_requirement_at(
                    commit_id,
                    EntityId::parse_canonical(&requirement)?,
                )?,
            )
        }
        Command::Vr {
            command:
                VerificationRequirementCommand::List {
                    store,
                    branch,
                    commit,
                    criterion,
                    local_key,
                    limit,
                },
        } => {
            let engine = Engine::open(store)?;
            let commit_id = resolve_task_query_commit(&engine, branch, commit)?;
            let mut requirements = engine.verification_requirements_at(commit_id)?;
            if let Some(criterion) = criterion {
                let criterion_id = EntityId::parse_canonical(&criterion)?;
                requirements.retain(|requirement| {
                    requirement.acceptance_criterion_entity_id == criterion_id
                });
            }
            if let Some(local_key) = local_key {
                requirements.retain(|requirement| requirement.local_key == local_key);
            }
            if matches!(limit, Some(0)) {
                return Err(WorkVcsError::QueryInvalid(
                    "vr list limit must be greater than zero".to_owned(),
                ));
            }
            if let Some(limit) = limit {
                requirements.truncate(limit);
            }
            render_verification_requirement_list(commit_id, &requirements)
        }
        Command::Evidence {
            command:
                EvidenceCommand::Create {
                    store,
                    kind,
                    metadata_json,
                    source_session,
                    content_role,
                    content,
                    content_digest,
                    content_file,
                    content_size_bytes,
                    media_type,
                    format_metadata_json,
                },
        } => {
            let mut engine = Engine::open(store)?;
            let mut options = EvidenceCreateOptions::new(
                kind,
                parse_cli_object("evidence metadata", &metadata_json)?,
            )?;
            if let Some(source_session) = source_session {
                options =
                    options.with_source_session_id(SessionId::parse_canonical(&source_session)?);
            }
            if let Some(content) = evidence_content_from_cli(EvidenceContentArgs {
                role: content_role,
                content,
                content_digest,
                content_file,
                content_size_bytes,
                media_type,
                format_metadata_json,
            })? {
                options = options.with_contents(vec![content])?;
            }
            render_evidence_create(&engine.create_evidence(options)?)
        }
        Command::Evidence {
            command: EvidenceCommand::Show { store, evidence },
        } => {
            let engine = Engine::open(store)?;
            render_evidence_snapshot(&engine.evidence(EvidenceId::parse_canonical(&evidence)?)?)
        }
        Command::Evidence {
            command:
                EvidenceCommand::List {
                    store,
                    kind,
                    source_session,
                    limit,
                },
        } => {
            let engine = Engine::open(store)?;
            let mut options = match kind {
                Some(kind) => EvidenceListOptions::for_kind(kind)?,
                None => EvidenceListOptions::all(),
            };
            if let Some(source_session) = source_session {
                options =
                    options.with_source_session_id(SessionId::parse_canonical(&source_session)?);
            }
            if matches!(limit, Some(0)) {
                return Err(WorkVcsError::QueryInvalid(
                    "evidence list limit must be greater than zero".to_owned(),
                ));
            }
            let mut result = engine.evidences(options)?;
            if let Some(limit) = limit {
                result.evidences.truncate(limit);
            }
            render_evidence_list(&result)
        }
        Command::Resource {
            command: ResourceCommand::Create { store, kind },
        } => {
            let mut engine = Engine::open(store)?;
            let resource = engine.create_resource(ResourceCreateOptions::new(kind)?)?;
            Ok(render_resource_create(&resource))
        }
        Command::Resource {
            command: ResourceCommand::Show { store, resource },
        } => {
            let engine = Engine::open(store)?;
            render_resource_snapshot(&engine.resource(ResourceId::parse_canonical(&resource)?)?)
        }
        Command::Resource {
            command:
                ResourceCommand::List {
                    store,
                    kind,
                    bound,
                    workspace,
                    limit,
                },
        } => {
            let engine = Engine::open(store)?;
            let options = match kind {
                Some(kind) => ResourceListOptions::for_kind(kind)?,
                None => ResourceListOptions::all(),
            };
            let mut result = engine.resources(options)?;
            if let Some(bound) = bound {
                result
                    .resources
                    .retain(|resource| resource.binding.is_some() == bound);
            }
            if let Some(workspace) = workspace {
                let workspace_id = workvcs_core::WorkspaceId::parse_canonical(&workspace)?;
                result.resources.retain(|resource| {
                    resource
                        .workspace_associations
                        .iter()
                        .any(|association| association.workspace_id == workspace_id)
                });
            }
            if matches!(limit, Some(0)) {
                return Err(WorkVcsError::QueryInvalid(
                    "resource list limit must be greater than zero".to_owned(),
                ));
            }
            if let Some(limit) = limit {
                result.resources.truncate(limit);
            }
            Ok(render_resource_list(&result))
        }
        Command::Resource {
            command:
                ResourceCommand::Bind {
                    store,
                    resource,
                    adapter_kind,
                    locator,
                    binding_config_json,
                },
        } => {
            let mut engine = Engine::open(store)?;
            let options = ResourceBindOptions::new(
                ResourceId::parse_canonical(&resource)?,
                adapter_kind,
                locator,
            )?
            .with_binding_config(parse_cli_object(
                "resource binding config",
                &binding_config_json,
            )?)?;
            render_resource_bind(&engine.bind_resource(options)?)
        }
        Command::Resource {
            command:
                ResourceCommand::AssociateWorkspace {
                    store,
                    workspace,
                    resource,
                    metadata_json,
                },
        } => {
            let mut engine = Engine::open(store)?;
            let options = WorkspaceResourceAssociationOptions::new(
                workvcs_core::WorkspaceId::parse_canonical(&workspace)?,
                ResourceId::parse_canonical(&resource)?,
            )?
            .with_association_metadata(parse_cli_object(
                "workspace resource association metadata",
                &metadata_json,
            )?)?;
            render_workspace_resource_association(&engine.associate_workspace_resource(options)?)
        }
        Command::Resource {
            command:
                ResourceCommand::WorkspaceAssociationList {
                    store,
                    workspace,
                    resource,
                },
        } => {
            let engine = Engine::open(store)?;
            let options = WorkspaceResourceAssociationListOptions::for_workspace(
                workvcs_core::WorkspaceId::parse_canonical(&workspace)?,
            );
            let mut result = engine.workspace_resource_associations(options)?;
            if let Some(resource) = resource {
                let resource_id = ResourceId::parse_canonical(&resource)?;
                result
                    .associations
                    .retain(|association| association.resource_id == resource_id);
            }
            render_workspace_resource_association_list(&result)
        }
        Command::Resource {
            command:
                ResourceCommand::Observe {
                    store,
                    resource,
                    adapter_kind,
                    adapter_schema_version,
                    fingerprint,
                    content,
                    content_file,
                    summary_json,
                    detail_content,
                    detail_content_file,
                    detail_content_digest,
                    detail_content_size_bytes,
                    detail_media_type,
                    detail_format_metadata_json,
                    source_session,
                },
        } => {
            let mut engine = Engine::open(store)?;
            let fingerprint = fingerprint_from_cli(fingerprint, content, content_file)?;
            let mut options = ResourceObservationCreateOptions::new(
                ResourceId::parse_canonical(&resource)?,
                adapter_kind,
                adapter_schema_version,
                fingerprint,
                parse_cli_object("resource observation summary", &summary_json)?,
            )?;
            if let Some(detail_content) =
                resource_observation_detail_from_cli(ResourceObservationDetailArgs {
                    content: detail_content,
                    content_file: detail_content_file,
                    content_digest: detail_content_digest,
                    content_size_bytes: detail_content_size_bytes,
                    media_type: detail_media_type,
                    format_metadata_json: detail_format_metadata_json,
                })?
            {
                options = options.with_detail_content(detail_content);
            }
            if let Some(source_session) = source_session {
                options =
                    options.with_source_session_id(SessionId::parse_canonical(&source_session)?);
            }
            let observation = engine.record_resource_observation(options)?;
            Ok(render_resource_observation_create(&observation))
        }
        Command::Resource {
            command: ResourceCommand::ObservationShow { store, observation },
        } => {
            let engine = Engine::open(store)?;
            render_resource_observation_snapshot(
                &engine
                    .resource_observation(ResourceObservationId::parse_canonical(&observation)?)?,
            )
        }
        Command::Resource {
            command:
                ResourceCommand::ObservationList {
                    store,
                    resource,
                    adapter_kind,
                    adapter_schema_version,
                    source_session,
                    limit,
                },
        } => {
            let engine = Engine::open(store)?;
            let mut options = match resource {
                Some(resource) => ResourceObservationListOptions::for_resource(
                    ResourceId::parse_canonical(&resource)?,
                ),
                None => ResourceObservationListOptions::all(),
            };
            if let Some(adapter_kind) = adapter_kind {
                options = options.with_adapter_kind(adapter_kind)?;
            }
            if let Some(adapter_schema_version) = adapter_schema_version {
                options = options.with_adapter_schema_version(adapter_schema_version)?;
            }
            if let Some(source_session) = source_session {
                options =
                    options.with_source_session_id(SessionId::parse_canonical(&source_session)?);
            }
            if matches!(limit, Some(0)) {
                return Err(WorkVcsError::QueryInvalid(
                    "resource observation-list limit must be greater than zero".to_owned(),
                ));
            }
            let mut result = engine.resource_observations(options)?;
            if let Some(limit) = limit {
                result.observations.truncate(limit);
            }
            render_resource_observation_list(&result)
        }
        Command::Verification {
            command:
                VerificationCommand::Record {
                    store,
                    branch,
                    head,
                    result,
                    method,
                    evidence,
                    acceptance_criterion,
                    verification_requirement,
                    resource,
                    adapter_kind,
                    adapter_schema_version,
                    scope_kind,
                    scope_schema_version,
                    scope_payload_json,
                    baseline_fingerprint,
                    baseline_observation,
                },
        } => {
            let mut engine = Engine::open(store)?;
            let target = match (acceptance_criterion, verification_requirement) {
                (Some(acceptance_criterion), None) => VerificationTarget::AcceptanceCriterion(
                    EntityId::parse_canonical(&acceptance_criterion)?,
                ),
                (None, Some(verification_requirement)) => {
                    VerificationTarget::VerificationRequirement(EntityId::parse_canonical(
                        &verification_requirement,
                    )?)
                }
                _ => {
                    return Err(WorkVcsError::TaskInvalid(
                        "verification record requires exactly one target".to_owned(),
                    ));
                }
            };
            let mut options = VerificationCreateOptions::new(
                BranchId::parse_canonical(&branch)?,
                CommitId::parse_canonical(&head)?,
                target,
                parse_verification_result(&result)?,
            )?;
            if let Some(method) = method {
                options = options.with_method(method_value(&method))?;
            }
            if !evidence.is_empty() {
                let evidence_ids = evidence
                    .iter()
                    .map(|evidence_id| EvidenceId::parse_canonical(evidence_id))
                    .collect::<Result<Vec<_>>>()?;
                options = options.with_evidence(evidence_ids)?;
            }
            if resource.is_some()
                || adapter_kind.is_some()
                || adapter_schema_version.is_some()
                || scope_kind.is_some()
                || scope_schema_version.is_some()
                || scope_payload_json.is_some()
                || baseline_fingerprint.is_some()
                || baseline_observation.is_some()
            {
                options = options.with_resource_basis(vec![resource_basis_from_cli(
                    ResourceBasisArgs {
                        resource,
                        adapter_kind,
                        adapter_schema_version,
                        scope_kind,
                        scope_schema_version,
                        scope_payload_json,
                        baseline_fingerprint,
                        baseline_observation,
                    },
                )?])?;
            }
            let verification = engine.create_verification(options)?;
            Ok(render_verification_create(&verification))
        }
        Command::Verification {
            command:
                VerificationCommand::Show {
                    store,
                    branch,
                    commit,
                    verification,
                },
        } => {
            let engine = Engine::open(store)?;
            let commit_id = resolve_task_query_commit(&engine, branch, commit)?;
            render_verification_snapshot(
                &engine.verification_at(commit_id, EntityId::parse_canonical(&verification)?)?,
            )
        }
        Command::Verification {
            command:
                VerificationCommand::List {
                    store,
                    branch,
                    commit,
                    target_kind,
                    target,
                    result,
                },
        } => {
            let engine = Engine::open(store)?;
            let commit_id = resolve_task_query_commit(&engine, branch, commit)?;
            let mut verifications = engine.verifications_at(commit_id)?;
            if let Some(target_kind) = target_kind {
                let target_kind = parse_verification_target_kind(&target_kind)?;
                verifications.retain(|verification| {
                    verification_target_kind(verification.target) == target_kind
                });
            }
            if let Some(target) = target {
                let target_id = EntityId::parse_canonical(&target)?;
                verifications.retain(|verification| verification.target.entity_id() == target_id);
            }
            if let Some(result) = result {
                let result = parse_verification_result(&result)?;
                verifications.retain(|verification| verification.state.result == result);
            }
            render_verification_list(commit_id, &verifications)
        }
        Command::Verification {
            command:
                VerificationCommand::CacheRecord {
                    store,
                    branch,
                    head,
                    verification,
                    resource_basis_ordinal,
                    adapter_kind,
                    adapter_schema_version,
                    scope_schema_version,
                    observation_status,
                    observed_fingerprint,
                    observation,
                    detail_json,
                },
        } => {
            let mut engine = Engine::open(store)?;
            let stamp = applicability_stamp_from_cli(
                resource_basis_ordinal,
                adapter_kind,
                adapter_schema_version,
                scope_schema_version,
                &observation_status,
                observed_fingerprint,
                observation,
            )?;
            let snapshot = engine.record_verification_applicability(
                VerificationApplicabilityRecordOptions::new(
                    BranchId::parse_canonical(&branch)?,
                    EntityId::parse_canonical(&verification)?,
                    CommitId::parse_canonical(&head)?,
                )?
                .with_resource_stamps(vec![stamp])?
                .with_detail(parse_cli_object(
                    "verification applicability detail",
                    &detail_json,
                )?)?,
            )?;
            Ok(render_verification_applicability_cache(&snapshot))
        }
        Command::Verification {
            command:
                VerificationCommand::CacheShow {
                    store,
                    branch,
                    verification,
                },
        } => {
            let engine = Engine::open(store)?;
            let branch_id = BranchId::parse_canonical(&branch)?;
            let verification_id = EntityId::parse_canonical(&verification)?;
            let snapshot = engine.verification_applicability_cache(branch_id, verification_id)?;
            render_verification_applicability_cache_lookup(
                branch_id,
                verification_id,
                snapshot.as_ref(),
            )
        }
        Command::Verification {
            command:
                VerificationCommand::CacheList {
                    store,
                    branch,
                    verification,
                    applicability,
                    reason_code,
                    limit,
                },
        } => {
            let engine = Engine::open(store)?;
            let mut options =
                VerificationApplicabilityCacheListOptions::new(BranchId::parse_canonical(&branch)?);
            if let Some(verification) = verification {
                options =
                    options.with_verification_entity_id(EntityId::parse_canonical(&verification)?);
            }
            if let Some(applicability) = applicability {
                options =
                    options.with_applicability(parse_verification_applicability(&applicability)?);
            }
            let mut result = engine.verification_applicability_caches(options)?;
            if let Some(reason_code) = reason_code {
                result
                    .caches
                    .retain(|cache| cache.reason_code == reason_code);
            }
            if matches!(limit, Some(0)) {
                return Err(WorkVcsError::TaskInvalid(
                    "verification cache-list limit must be greater than zero".to_owned(),
                ));
            }
            if let Some(limit) = limit {
                result.caches.truncate(limit);
            }
            Ok(render_verification_applicability_cache_list(&result))
        }
        Command::Record {
            command:
                RecordCommand::Show {
                    store,
                    branch,
                    commit,
                    record,
                },
        } => {
            let engine = Engine::open(store)?;
            let commit_id = resolve_record_query_commit(&engine, branch, commit)?;
            render_record_show(&engine.record_at(commit_id, EntityId::parse_canonical(&record)?)?)
        }
        Command::Record {
            command:
                RecordCommand::List {
                    store,
                    branch,
                    commit,
                    kind,
                    status,
                    scope_json,
                    statement_contains,
                },
        } => {
            let engine = Engine::open(store)?;
            let mut options =
                RecordListOptions::new(resolve_record_query_commit(&engine, branch, commit)?);
            if let Some(kind) = kind {
                options = options.with_kind(parse_record_kind(&kind)?);
            }
            if let Some(status) = status {
                options = options.with_status(parse_record_status(&status)?);
            }
            if let Some(scope_json) = scope_json {
                options = options.with_scope(parse_cli_object("record scope", &scope_json)?)?;
            }
            if let Some(statement_contains) = statement_contains {
                options = options.with_statement_contains(statement_contains)?;
            }
            Ok(render_record_list(&engine.records_at(options)?))
        }
        Command::Record {
            command:
                RecordCommand::LinkInvalidates {
                    store,
                    branch,
                    head,
                    source_record,
                    target_record,
                    rationale,
                },
        } => {
            let mut engine = Engine::open(store)?;
            let relation =
                engine.create_record_relation(RecordRelationCreateOptions::invalidates(
                    BranchId::parse_canonical(&branch)?,
                    CommitId::parse_canonical(&head)?,
                    EntityId::parse_canonical(&source_record)?,
                    EntityId::parse_canonical(&target_record)?,
                    rationale,
                )?)?;
            Ok(render_record_relation_create(&relation))
        }
        Command::Record {
            command:
                RecordCommand::LinkInvalidatesKnowledge {
                    store,
                    branch,
                    head,
                    source_record,
                    target_knowledge,
                    rationale,
                },
        } => {
            let mut engine = Engine::open(store)?;
            let relation = engine.create_record_knowledge_relation(
                RecordKnowledgeRelationCreateOptions::invalidates(
                    BranchId::parse_canonical(&branch)?,
                    CommitId::parse_canonical(&head)?,
                    EntityId::parse_canonical(&source_record)?,
                    EntityId::parse_canonical(&target_knowledge)?,
                    rationale,
                )?,
            )?;
            Ok(render_record_knowledge_relation_create(&relation))
        }
        Command::Record {
            command:
                RecordCommand::LinkValidates {
                    store,
                    branch,
                    head,
                    source_record,
                    target_record,
                    rationale,
                },
        } => {
            let mut engine = Engine::open(store)?;
            let relation =
                engine.create_record_relation(RecordRelationCreateOptions::validates(
                    BranchId::parse_canonical(&branch)?,
                    CommitId::parse_canonical(&head)?,
                    EntityId::parse_canonical(&source_record)?,
                    EntityId::parse_canonical(&target_record)?,
                    rationale,
                )?)?;
            Ok(render_record_relation_create(&relation))
        }
        Command::Record {
            command:
                RecordCommand::LinkValidatesKnowledge {
                    store,
                    branch,
                    head,
                    source_record,
                    target_knowledge,
                    rationale,
                },
        } => {
            let mut engine = Engine::open(store)?;
            let relation = engine.create_record_knowledge_relation(
                RecordKnowledgeRelationCreateOptions::validates(
                    BranchId::parse_canonical(&branch)?,
                    CommitId::parse_canonical(&head)?,
                    EntityId::parse_canonical(&source_record)?,
                    EntityId::parse_canonical(&target_knowledge)?,
                    rationale,
                )?,
            )?;
            Ok(render_record_knowledge_relation_create(&relation))
        }
        Command::Record {
            command:
                RecordCommand::LinkSupports {
                    store,
                    branch,
                    head,
                    source_record,
                    target_record,
                    rationale,
                },
        } => {
            let mut engine = Engine::open(store)?;
            let relation = engine.create_record_relation(RecordRelationCreateOptions::supports(
                BranchId::parse_canonical(&branch)?,
                CommitId::parse_canonical(&head)?,
                EntityId::parse_canonical(&source_record)?,
                EntityId::parse_canonical(&target_record)?,
                rationale,
            )?)?;
            Ok(render_record_relation_create(&relation))
        }
        Command::Record {
            command:
                RecordCommand::LinkSupportsKnowledge {
                    store,
                    branch,
                    head,
                    source_record,
                    target_knowledge,
                    rationale,
                },
        } => {
            let mut engine = Engine::open(store)?;
            let relation = engine.create_record_knowledge_relation(
                RecordKnowledgeRelationCreateOptions::supports(
                    BranchId::parse_canonical(&branch)?,
                    CommitId::parse_canonical(&head)?,
                    EntityId::parse_canonical(&source_record)?,
                    EntityId::parse_canonical(&target_knowledge)?,
                    rationale,
                )?,
            )?;
            Ok(render_record_knowledge_relation_create(&relation))
        }
        Command::Record {
            command:
                RecordCommand::LinkContradicts {
                    store,
                    branch,
                    head,
                    source_record,
                    target_record,
                    rationale,
                },
        } => {
            let mut engine = Engine::open(store)?;
            let relation =
                engine.create_record_relation(RecordRelationCreateOptions::contradicts(
                    BranchId::parse_canonical(&branch)?,
                    CommitId::parse_canonical(&head)?,
                    EntityId::parse_canonical(&source_record)?,
                    EntityId::parse_canonical(&target_record)?,
                    rationale,
                )?)?;
            Ok(render_record_relation_create(&relation))
        }
        Command::Record {
            command:
                RecordCommand::LinkContradictsKnowledge {
                    store,
                    branch,
                    head,
                    source_record,
                    target_knowledge,
                    rationale,
                },
        } => {
            let mut engine = Engine::open(store)?;
            let relation = engine.create_record_knowledge_relation(
                RecordKnowledgeRelationCreateOptions::contradicts(
                    BranchId::parse_canonical(&branch)?,
                    CommitId::parse_canonical(&head)?,
                    EntityId::parse_canonical(&source_record)?,
                    EntityId::parse_canonical(&target_knowledge)?,
                    rationale,
                )?,
            )?;
            Ok(render_record_knowledge_relation_create(&relation))
        }
        Command::Record {
            command:
                RecordCommand::LinkDerivedFrom {
                    store,
                    branch,
                    head,
                    result_record,
                    source_record,
                    rationale,
                },
        } => {
            let mut engine = Engine::open(store)?;
            let relation =
                engine.create_record_relation(RecordRelationCreateOptions::derived_from(
                    BranchId::parse_canonical(&branch)?,
                    CommitId::parse_canonical(&head)?,
                    EntityId::parse_canonical(&result_record)?,
                    EntityId::parse_canonical(&source_record)?,
                    rationale,
                )?)?;
            Ok(render_record_relation_create(&relation))
        }
        Command::Record {
            command:
                RecordCommand::LinkRelatedTo {
                    store,
                    branch,
                    head,
                    source_record,
                    target_record,
                    label,
                    rationale,
                },
        } => {
            let mut engine = Engine::open(store)?;
            let relation =
                engine.create_record_relation(RecordRelationCreateOptions::related_to(
                    BranchId::parse_canonical(&branch)?,
                    CommitId::parse_canonical(&head)?,
                    EntityId::parse_canonical(&source_record)?,
                    EntityId::parse_canonical(&target_record)?,
                    label,
                    rationale,
                )?)?;
            Ok(render_record_relation_create(&relation))
        }
        Command::Record {
            command:
                RecordCommand::SupersedeDecision {
                    store,
                    branch,
                    head,
                    replacement_record,
                    prior_record,
                    prior_record_version,
                    because_record,
                    rationale,
                },
        } => {
            let mut engine = Engine::open(store)?;
            let mut options = DecisionRecordSupersedeOptions::new(
                BranchId::parse_canonical(&branch)?,
                CommitId::parse_canonical(&head)?,
                EntityId::parse_canonical(&replacement_record)?,
                EntityId::parse_canonical(&prior_record)?,
                EntityVersionId::parse_canonical(&prior_record_version)?,
                rationale,
            )?;
            if let Some(because_record) = because_record {
                options = options.with_causal_record(EntityId::parse_canonical(&because_record)?);
            }
            let superseded = engine.supersede_decision_record(options)?;
            Ok(render_decision_record_supersede(&superseded))
        }
        Command::Record {
            command:
                RecordCommand::RelationList {
                    store,
                    branch,
                    commit,
                    relation_type,
                    label,
                    source_record,
                    target_record,
                },
        } => {
            let engine = Engine::open(store)?;
            let mut options = RecordRelationListOptions::new(resolve_record_query_commit(
                &engine, branch, commit,
            )?);
            if let Some(relation_type) = relation_type {
                options = options.with_relation_type(parse_record_relation_type(&relation_type)?);
            }
            if let Some(label) = label {
                options = options.with_relation_label(label)?;
            }
            if let Some(source_record) = source_record {
                options = options.with_source_record(EntityId::parse_canonical(&source_record)?);
            }
            if let Some(target_record) = target_record {
                options = options.with_target_record(EntityId::parse_canonical(&target_record)?);
            }
            Ok(render_record_relation_list(
                &engine.record_relations_at(options)?,
            ))
        }
        Command::Record {
            command:
                RecordCommand::KnowledgeRelationList {
                    store,
                    branch,
                    commit,
                    relation_type,
                    source_record,
                    target_knowledge,
                },
        } => {
            let engine = Engine::open(store)?;
            let mut options = RecordKnowledgeRelationListOptions::new(resolve_record_query_commit(
                &engine, branch, commit,
            )?);
            if let Some(relation_type) = relation_type {
                options = options
                    .with_relation_type(parse_record_knowledge_relation_type(&relation_type)?);
            }
            if let Some(source_record) = source_record {
                options = options.with_source_record(EntityId::parse_canonical(&source_record)?);
            }
            if let Some(target_knowledge) = target_knowledge {
                options =
                    options.with_target_knowledge(EntityId::parse_canonical(&target_knowledge)?);
            }
            Ok(render_record_knowledge_relation_list(
                &engine.record_knowledge_relations_at(options)?,
            ))
        }
        Command::Record {
            command:
                RecordCommand::KnowledgeRelationShow {
                    store,
                    branch,
                    commit,
                    relation,
                },
        } => {
            let engine = Engine::open(store)?;
            let commit_id = resolve_record_query_commit(&engine, branch, commit)?;
            Ok(render_record_knowledge_relation_snapshot(
                &engine.record_knowledge_relation_at(
                    commit_id,
                    RelationId::parse_canonical(&relation)?,
                )?,
            ))
        }
        Command::Record {
            command:
                RecordCommand::KnowledgeRelationRemove {
                    store,
                    branch,
                    head,
                    relation,
                    relation_version,
                    rationale,
                },
        } => {
            let mut engine = Engine::open(store)?;
            let removed = engine.remove_record_knowledge_relation(
                RecordKnowledgeRelationRemoveOptions::new(
                    BranchId::parse_canonical(&branch)?,
                    CommitId::parse_canonical(&head)?,
                    RelationId::parse_canonical(&relation)?,
                    RelationVersionId::parse_canonical(&relation_version)?,
                    rationale,
                )?,
            )?;
            Ok(render_record_knowledge_relation_remove(&removed))
        }
        Command::Record {
            command:
                RecordCommand::KnowledgeRelationRestore {
                    store,
                    branch,
                    head,
                    relation,
                    relation_version,
                    rationale,
                },
        } => {
            let mut engine = Engine::open(store)?;
            let restored = engine.restore_record_knowledge_relation(
                RecordKnowledgeRelationRestoreOptions::new(
                    BranchId::parse_canonical(&branch)?,
                    CommitId::parse_canonical(&head)?,
                    RelationId::parse_canonical(&relation)?,
                    RelationVersionId::parse_canonical(&relation_version)?,
                    rationale,
                )?,
            )?;
            Ok(render_record_knowledge_relation_restore(&restored))
        }
        Command::Record {
            command:
                RecordCommand::RelationShow {
                    store,
                    branch,
                    commit,
                    relation,
                },
        } => {
            let engine = Engine::open(store)?;
            let commit_id = resolve_record_query_commit(&engine, branch, commit)?;
            Ok(render_record_relation_snapshot(
                &engine.record_relation_at(commit_id, RelationId::parse_canonical(&relation)?)?,
            ))
        }
        Command::Record {
            command:
                RecordCommand::RelationRemove {
                    store,
                    branch,
                    head,
                    relation,
                    relation_version,
                    rationale,
                },
        } => {
            let mut engine = Engine::open(store)?;
            let removed = engine.remove_record_relation(RecordRelationRemoveOptions::new(
                BranchId::parse_canonical(&branch)?,
                CommitId::parse_canonical(&head)?,
                RelationId::parse_canonical(&relation)?,
                RelationVersionId::parse_canonical(&relation_version)?,
                rationale,
            )?)?;
            Ok(render_record_relation_remove(&removed))
        }
        Command::Record {
            command:
                RecordCommand::RelationRestore {
                    store,
                    branch,
                    head,
                    relation,
                    relation_version,
                    rationale,
                },
        } => {
            let mut engine = Engine::open(store)?;
            let restored = engine.restore_record_relation(RecordRelationRestoreOptions::new(
                BranchId::parse_canonical(&branch)?,
                CommitId::parse_canonical(&head)?,
                RelationId::parse_canonical(&relation)?,
                RelationVersionId::parse_canonical(&relation_version)?,
                rationale,
            )?)?;
            Ok(render_record_relation_restore(&restored))
        }
        Command::Record {
            command:
                RecordCommand::Assumption {
                    store,
                    branch,
                    head,
                    statement,
                    scope_json,
                },
        } => {
            let mut engine = Engine::open(store)?;
            let mut options = RecordCreateOptions::assumption(
                BranchId::parse_canonical(&branch)?,
                CommitId::parse_canonical(&head)?,
                statement,
            )?;
            if let Some(scope_json) = scope_json {
                options = options.with_scope(parse_cli_object("record scope", &scope_json)?)?;
            }
            let record = engine.create_record(options)?;
            Ok(render_record_create(&record))
        }
        Command::Record {
            command:
                RecordCommand::Attempt {
                    store,
                    branch,
                    head,
                    statement,
                    scope_json,
                },
        } => {
            let mut engine = Engine::open(store)?;
            let mut options = RecordCreateOptions::attempt(
                BranchId::parse_canonical(&branch)?,
                CommitId::parse_canonical(&head)?,
                statement,
            )?;
            if let Some(scope_json) = scope_json {
                options = options.with_scope(parse_cli_object("record scope", &scope_json)?)?;
            }
            let record = engine.create_record(options)?;
            Ok(render_record_create(&record))
        }
        Command::Record {
            command:
                RecordCommand::AttemptStatus {
                    store,
                    branch,
                    head,
                    record,
                    record_version,
                    status,
                    rationale,
                },
        } => {
            let mut engine = Engine::open(store)?;
            let branch_id = BranchId::parse_canonical(&branch)?;
            let head_id = CommitId::parse_canonical(&head)?;
            let record_id = EntityId::parse_canonical(&record)?;
            let record_version_id = EntityVersionId::parse_canonical(&record_version)?;
            let options = match parse_attempt_record_status(&status)? {
                RecordStatus::Succeeded => RecordTransitionOptions::complete_attempt_succeeded(
                    branch_id,
                    head_id,
                    record_id,
                    record_version_id,
                    rationale,
                )?,
                RecordStatus::Failed => RecordTransitionOptions::complete_attempt_failed(
                    branch_id,
                    head_id,
                    record_id,
                    record_version_id,
                    rationale,
                )?,
                RecordStatus::Inconclusive => {
                    RecordTransitionOptions::complete_attempt_inconclusive(
                        branch_id,
                        head_id,
                        record_id,
                        record_version_id,
                        rationale,
                    )?
                }
                _ => {
                    return Err(WorkVcsError::RecordInvalid(format!(
                        "attempt status {status:?} is not a transition target"
                    )));
                }
            };
            let record = engine.transition_record(options)?;
            Ok(render_record_transition(&record))
        }
        Command::Record {
            command:
                RecordCommand::AssumptionStatus {
                    store,
                    branch,
                    head,
                    record,
                    record_version,
                    status,
                    rationale,
                },
        } => {
            let mut engine = Engine::open(store)?;
            let branch_id = BranchId::parse_canonical(&branch)?;
            let head_id = CommitId::parse_canonical(&head)?;
            let record_id = EntityId::parse_canonical(&record)?;
            let record_version_id = EntityVersionId::parse_canonical(&record_version)?;
            let options = match parse_assumption_record_status(&status)? {
                RecordStatus::Validated => RecordTransitionOptions::validate_assumption(
                    branch_id,
                    head_id,
                    record_id,
                    record_version_id,
                    rationale,
                )?,
                RecordStatus::Invalidated => RecordTransitionOptions::invalidate_assumption(
                    branch_id,
                    head_id,
                    record_id,
                    record_version_id,
                    rationale,
                )?,
                _ => {
                    return Err(WorkVcsError::RecordInvalid(format!(
                        "assumption status {status:?} is not a transition target"
                    )));
                }
            };
            let record = engine.transition_record(options)?;
            Ok(render_record_transition(&record))
        }
        Command::Record {
            command:
                RecordCommand::Finding {
                    store,
                    branch,
                    head,
                    statement,
                    scope_json,
                },
        } => {
            let mut engine = Engine::open(store)?;
            let mut options = RecordCreateOptions::finding(
                BranchId::parse_canonical(&branch)?,
                CommitId::parse_canonical(&head)?,
                statement,
            )?;
            if let Some(scope_json) = scope_json {
                options = options.with_scope(parse_cli_object("record scope", &scope_json)?)?;
            }
            let record = engine.create_record(options)?;
            Ok(render_record_create(&record))
        }
        Command::Record {
            command:
                RecordCommand::Handoff {
                    store,
                    branch,
                    head,
                    statement,
                    scope_json,
                },
        } => {
            let mut engine = Engine::open(store)?;
            let mut options = RecordCreateOptions::handoff(
                BranchId::parse_canonical(&branch)?,
                CommitId::parse_canonical(&head)?,
                statement,
            )?;
            if let Some(scope_json) = scope_json {
                options = options.with_scope(parse_cli_object("record scope", &scope_json)?)?;
            }
            let record = engine.create_record(options)?;
            Ok(render_record_create(&record))
        }
        Command::Record {
            command:
                RecordCommand::Decision {
                    store,
                    branch,
                    head,
                    statement,
                    scope_json,
                },
        } => {
            let mut engine = Engine::open(store)?;
            let mut options = RecordCreateOptions::decision(
                BranchId::parse_canonical(&branch)?,
                CommitId::parse_canonical(&head)?,
                statement,
            )?;
            if let Some(scope_json) = scope_json {
                options = options.with_scope(parse_cli_object("record scope", &scope_json)?)?;
            }
            let record = engine.create_record(options)?;
            Ok(render_record_create(&record))
        }
        Command::Record {
            command:
                RecordCommand::DecisionStatus {
                    store,
                    branch,
                    head,
                    record,
                    record_version,
                    status,
                    rationale,
                },
        } => {
            let mut engine = Engine::open(store)?;
            let branch_id = BranchId::parse_canonical(&branch)?;
            let head_id = CommitId::parse_canonical(&head)?;
            let record_id = EntityId::parse_canonical(&record)?;
            let record_version_id = EntityVersionId::parse_canonical(&record_version)?;
            let options = match parse_decision_record_status(&status)? {
                RecordStatus::Superseded => RecordTransitionOptions::supersede_decision(
                    branch_id,
                    head_id,
                    record_id,
                    record_version_id,
                    rationale,
                )?,
                RecordStatus::Withdrawn => RecordTransitionOptions::withdraw_decision(
                    branch_id,
                    head_id,
                    record_id,
                    record_version_id,
                    rationale,
                )?,
                _ => {
                    return Err(WorkVcsError::RecordInvalid(format!(
                        "decision status {status:?} is not a transition target"
                    )));
                }
            };
            let record = engine.transition_record(options)?;
            Ok(render_record_transition(&record))
        }
        Command::Record {
            command:
                RecordCommand::Question {
                    store,
                    branch,
                    head,
                    statement,
                    scope_json,
                },
        } => {
            let mut engine = Engine::open(store)?;
            let mut options = RecordCreateOptions::question(
                BranchId::parse_canonical(&branch)?,
                CommitId::parse_canonical(&head)?,
                statement,
            )?;
            if let Some(scope_json) = scope_json {
                options = options.with_scope(parse_cli_object("record scope", &scope_json)?)?;
            }
            let record = engine.create_record(options)?;
            Ok(render_record_create(&record))
        }
        Command::Record {
            command:
                RecordCommand::Risk {
                    store,
                    branch,
                    head,
                    statement,
                    scope_json,
                },
        } => {
            let mut engine = Engine::open(store)?;
            let mut options = RecordCreateOptions::risk(
                BranchId::parse_canonical(&branch)?,
                CommitId::parse_canonical(&head)?,
                statement,
            )?;
            if let Some(scope_json) = scope_json {
                options = options.with_scope(parse_cli_object("record scope", &scope_json)?)?;
            }
            let record = engine.create_record(options)?;
            Ok(render_record_create(&record))
        }
        Command::Session {
            command:
                SessionCommand::Start {
                    store,
                    workspace,
                    branch,
                    metadata_json,
                },
        } => {
            let mut engine = Engine::open(store)?;
            let session = engine.start_session(
                SessionStartOptions::new(
                    workvcs_core::WorkspaceId::parse_canonical(&workspace)?,
                    BranchId::parse_canonical(&branch)?,
                )?
                .with_metadata(parse_cli_object("session metadata", &metadata_json)?)?,
            )?;
            Ok(render_session_start(&session))
        }
        Command::Session {
            command: SessionCommand::Show { store, session },
        } => {
            let engine = Engine::open(store)?;
            render_session_snapshot(
                &engine.session_snapshot(SessionId::parse_canonical(&session)?)?,
            )
        }
        Command::Session {
            command:
                SessionCommand::List {
                    store,
                    lifecycle,
                    workspace,
                    branch,
                    focus,
                },
        } => {
            let engine = Engine::open(store)?;
            let mut options = SessionListOptions::all();
            if let Some(lifecycle) = lifecycle {
                options = options.with_lifecycle_state(parse_session_lifecycle_state(&lifecycle)?);
            }
            if let Some(workspace) = workspace {
                options = options.with_active_workspace_id(
                    workvcs_core::WorkspaceId::parse_canonical(&workspace)?,
                );
            }
            if let Some(branch) = branch {
                options = options.with_active_branch_id(BranchId::parse_canonical(&branch)?);
            }
            let mut result = engine.sessions(options)?;
            if let Some(focus) = focus {
                let focus_entity_id = EntityId::parse_canonical(&focus)?;
                result.sessions.retain(|session| {
                    session
                        .focus
                        .as_ref()
                        .is_some_and(|focus| focus.focus_entity_id == focus_entity_id)
                });
            }
            render_session_list(&result)
        }
        Command::Session {
            command:
                SessionCommand::FocusSet {
                    store,
                    session,
                    focus,
                },
        } => {
            let mut engine = Engine::open(store)?;
            let updated = engine.set_session_focus(SessionFocusOptions::new(
                SessionId::parse_canonical(&session)?,
                EntityId::parse_canonical(&focus)?,
            ))?;
            Ok(render_session_focus_update(&updated))
        }
        Command::Session {
            command: SessionCommand::FocusClear { store, session },
        } => {
            let mut engine = Engine::open(store)?;
            let updated = engine.clear_session_focus(SessionId::parse_canonical(&session)?)?;
            Ok(render_session_focus_update(&updated))
        }
        Command::Session {
            command:
                SessionCommand::Switch {
                    store,
                    session,
                    workspace,
                    branch,
                    focus,
                },
        } => {
            let mut engine = Engine::open(store)?;
            let mut options = SessionSwitchOptions::new(
                SessionId::parse_canonical(&session)?,
                workvcs_core::WorkspaceId::parse_canonical(&workspace)?,
                BranchId::parse_canonical(&branch)?,
            );
            if let Some(focus) = focus {
                options = options.with_focus(EntityId::parse_canonical(&focus)?);
            }
            let switched = engine.switch_session(options)?;
            Ok(render_session_switch(&switched))
        }
        Command::Session {
            command:
                SessionCommand::End {
                    store,
                    session,
                    summary_json,
                },
        } => {
            let mut engine = Engine::open(store)?;
            let ended = engine.end_session(
                SessionEndOptions::new(SessionId::parse_canonical(&session)?)?
                    .with_summary(parse_cli_object("session summary", &summary_json)?)?,
            )?;
            Ok(render_session_end(&ended))
        }
        Command::Claim {
            command: ClaimCommand::Show { store, claim },
        } => {
            let engine = Engine::open(store)?;
            let snapshot = engine.claim_snapshot(ClaimId::parse_canonical(&claim)?);
            Ok(render_claim_show(&snapshot?))
        }
        Command::Claim {
            command:
                ClaimCommand::List {
                    store,
                    session,
                    task,
                    mode,
                },
        } => {
            let engine = Engine::open(store)?;
            let mut claims = engine.active_claims_for_session(ClaimListOptions::for_session(
                SessionId::parse_canonical(&session)?,
            ))?;
            if let Some(task) = task {
                let task_id = EntityId::parse_canonical(&task)?;
                claims
                    .claims
                    .retain(|claim| claim.task_entity_id == task_id);
            }
            if let Some(mode) = mode {
                let mode = parse_claim_mode(&mode)?;
                claims.claims.retain(|claim| claim.mode == mode);
            }
            Ok(render_claim_list(&claims))
        }
        Command::Claim {
            command:
                ClaimCommand::Guard {
                    store,
                    session,
                    task,
                    action,
                },
        } => {
            let engine = Engine::open(store)?;
            let guard = engine.task_claim_guard(ClaimGuardOptions::new(
                SessionId::parse_canonical(&session)?,
                EntityId::parse_canonical(&task)?,
                parse_claim_guard_action(&action)?,
            ))?;
            Ok(render_claim_guard(&guard))
        }
        Command::Claim {
            command:
                ClaimCommand::Next {
                    store,
                    session,
                    mode,
                },
        } => {
            let mut engine = Engine::open(store)?;
            let claimed = engine.claim_next_task(
                ClaimNextOptions::new(SessionId::parse_canonical(&session)?)
                    .with_mode(parse_claim_mode(&mode)?),
            )?;
            Ok(render_claim_next(&claimed))
        }
        Command::Claim {
            command:
                ClaimCommand::Task {
                    store,
                    session,
                    task,
                    mode,
                },
        } => {
            let mut engine = Engine::open(store)?;
            let claim = engine.claim_task(
                ClaimTaskOptions::new(
                    SessionId::parse_canonical(&session)?,
                    EntityId::parse_canonical(&task)?,
                )
                .with_mode(parse_claim_mode(&mode)?),
            )?;
            Ok(render_claim_task(&claim))
        }
        Command::Claim {
            command:
                ClaimCommand::Release {
                    store,
                    session,
                    claim,
                },
        } => {
            let mut engine = Engine::open(store)?;
            let released = engine.release_claim(ClaimReleaseOptions::new(
                SessionId::parse_canonical(&session)?,
                ClaimId::parse_canonical(&claim)?,
            ))?;
            Ok(render_claim_release(&released))
        }
        Command::Context { store, session } => {
            let engine = Engine::open(store)?;
            let context = engine.context_overview(ContextOverviewOptions::new(
                SessionId::parse_canonical(&session)?,
            ))?;
            Ok(render_context_overview(&context))
        }
        Command::Next {
            store,
            session,
            mode,
        } => {
            let mut engine = Engine::open(store)?;
            let next = engine.next_work(
                NextWorkOptions::new(SessionId::parse_canonical(&session)?)
                    .with_mode(parse_claim_mode(&mode)?),
            )?;
            Ok(render_next_work(&next))
        }
        Command::Runnable {
            command:
                RunnableCommand::Tasks {
                    store,
                    session,
                    task,
                    status,
                    runnable,
                },
        } => {
            let engine = Engine::open(store)?;
            let mut projection = engine.runnable_tasks(RunnableTasksOptions::new(
                SessionId::parse_canonical(&session)?,
            ))?;
            if let Some(task) = task {
                let task_id = EntityId::parse_canonical(&task)?;
                projection
                    .candidates
                    .retain(|candidate| candidate.task.task_entity_id == task_id);
            }
            if let Some(status) = status {
                let status = parse_task_list_status(&status)?;
                projection
                    .candidates
                    .retain(|candidate| candidate.task.state.status == status);
            }
            if let Some(runnable) = runnable {
                projection
                    .candidates
                    .retain(|candidate| candidate.runnable == runnable);
            }
            Ok(render_runnable_tasks(&projection))
        }
        Command::Merge {
            command:
                MergeCommand::Start {
                    store,
                    target_branch,
                    source_branch,
                    session,
                },
        } => {
            let mut engine = Engine::open(store)?;
            let mut options = MergeStartOptions::new(
                BranchId::parse_canonical(&target_branch)?,
                BranchId::parse_canonical(&source_branch)?,
            );
            if let Some(session) = session {
                options = options.with_origin_session_id(SessionId::parse_canonical(&session)?);
            }
            Ok(render_merge_start(&engine.start_merge(options)?))
        }
        Command::Merge {
            command:
                MergeCommand::Abort {
                    store,
                    merge,
                    session,
                    detail_json,
                },
        } => {
            let mut engine = Engine::open(store)?;
            let mut options = MergeAbortOptions::new(MergeId::parse_canonical(&merge)?)?
                .with_detail(parse_cli_object("merge abort detail", &detail_json)?)?;
            if let Some(session) = session {
                options = options.with_abort_session_id(SessionId::parse_canonical(&session)?);
            }
            Ok(render_merge_abort(&engine.abort_merge(options)?))
        }
        Command::Merge {
            command:
                MergeCommand::Resolve {
                    store,
                    item,
                    kind,
                    session,
                    custom_payload_json,
                    rationale_json,
                },
        } => {
            let mut engine = Engine::open(store)?;
            let merge_item_id = MergeItemId::parse_canonical(&item)?;
            let kind = parse_merge_resolution_kind(&kind)?;
            let mut options = match kind {
                MergeResolutionKind::Ours => {
                    reject_custom_payload_for_non_custom(custom_payload_json.as_ref())?;
                    MergeResolveOptions::ours(merge_item_id)?
                }
                MergeResolutionKind::Theirs => {
                    reject_custom_payload_for_non_custom(custom_payload_json.as_ref())?;
                    MergeResolveOptions::theirs(merge_item_id)?
                }
                MergeResolutionKind::Custom => {
                    let custom_payload_json = custom_payload_json.ok_or_else(|| {
                        WorkVcsError::TaskInvalid(
                            "custom merge resolution requires --custom-payload-json".to_owned(),
                        )
                    })?;
                    MergeResolveOptions::custom(
                        merge_item_id,
                        parse_cli_object("merge custom resolution payload", &custom_payload_json)?,
                    )?
                }
            }
            .with_rationale(parse_cli_object(
                "merge resolution rationale",
                &rationale_json,
            )?)?;
            if let Some(session) = session {
                options =
                    options.with_resolved_by_session_id(SessionId::parse_canonical(&session)?);
            }
            Ok(render_merge_resolve(&engine.resolve_merge_item(options)?)?)
        }
        Command::Merge {
            command: MergeCommand::Freeze { store, merge },
        } => {
            let mut engine = Engine::open(store)?;
            let frozen = engine.freeze_merge_resolutions(MergeFreezeResolutionsOptions::new(
                MergeId::parse_canonical(&merge)?,
            ))?;
            Ok(render_merge_freeze(&frozen))
        }
        Command::Merge {
            command:
                MergeCommand::Continue {
                    store,
                    merge,
                    session,
                    detail_json,
                },
        } => {
            let mut engine = Engine::open(store)?;
            let mut options = MergeContinueOptions::new(MergeId::parse_canonical(&merge)?)?
                .with_detail(parse_cli_object("merge continue detail", &detail_json)?)?;
            if let Some(session) = session {
                options = options.with_continue_session_id(SessionId::parse_canonical(&session)?);
            }
            Ok(render_merge_continue(&engine.continue_merge(options)?))
        }
        Command::Merge {
            command: MergeCommand::Show { store, merge },
        } => {
            let engine = Engine::open(store)?;
            Ok(render_merge_attempt(
                &engine.merge_attempt(MergeId::parse_canonical(&merge)?)?,
            )?)
        }
        Command::Merge {
            command:
                MergeCommand::List {
                    store,
                    workspace,
                    target_branch,
                    include_closed,
                    runtime_state,
                    outcome,
                },
        } => {
            let engine = Engine::open(store)?;
            let mut options =
                MergeListOptions::new(workvcs_core::WorkspaceId::parse_canonical(&workspace)?);
            if let Some(target_branch) = target_branch {
                options = options.with_target_branch(BranchId::parse_canonical(&target_branch)?);
            }
            if include_closed {
                options = options.include_closed();
            }
            let mut result = engine.merge_attempts(options)?;
            if let Some(runtime_state) = runtime_state {
                result
                    .merges
                    .retain(|merge| merge.runtime_state.as_str() == runtime_state);
            }
            if let Some(outcome) = outcome {
                result.merges.retain(|merge| {
                    merge
                        .outcome
                        .as_ref()
                        .map(|outcome| outcome.outcome.as_str())
                        .unwrap_or("none")
                        == outcome
                });
            }
            Ok(render_merge_list(&result)?)
        }
    }
}

fn parse_task_status(value: &str) -> Result<TaskStatus> {
    match value {
        "pending" => Ok(TaskStatus::Pending),
        "in_progress" => Ok(TaskStatus::InProgress),
        "blocked" => Ok(TaskStatus::Blocked),
        "done" => Ok(TaskStatus::Done),
        "failed" => Ok(TaskStatus::Failed),
        "cancelled" => Ok(TaskStatus::Cancelled),
        other => Err(WorkVcsError::TaskInvalid(format!(
            "task status {other:?} is not in the CLI vocabulary"
        ))),
    }
}

fn parse_task_list_status(value: &str) -> Result<TaskStatus> {
    match value {
        "superseded" => Ok(TaskStatus::Superseded),
        _ => parse_task_status(value),
    }
}

fn parse_goal_list_status(value: &str) -> Result<GoalStatus> {
    match value {
        "active" => Ok(GoalStatus::Active),
        "achieved" => Ok(GoalStatus::Achieved),
        "abandoned" => Ok(GoalStatus::Abandoned),
        other => Err(WorkVcsError::GoalInvalid(format!(
            "goal status {other:?} is not in the CLI vocabulary"
        ))),
    }
}

fn parse_plan_list_status(value: &str) -> Result<PlanStatus> {
    match value {
        "active" => Ok(PlanStatus::Active),
        "completed" => Ok(PlanStatus::Completed),
        "abandoned" => Ok(PlanStatus::Abandoned),
        "superseded" => Ok(PlanStatus::Superseded),
        other => Err(WorkVcsError::PlanInvalid(format!(
            "plan status {other:?} is not in the CLI vocabulary"
        ))),
    }
}

fn parse_merge_resolution_kind(value: &str) -> Result<MergeResolutionKind> {
    match value {
        "ours" => Ok(MergeResolutionKind::Ours),
        "theirs" => Ok(MergeResolutionKind::Theirs),
        "custom" => Ok(MergeResolutionKind::Custom),
        other => Err(WorkVcsError::TaskInvalid(format!(
            "merge resolution kind {other:?} is not in the CLI vocabulary"
        ))),
    }
}

fn reject_custom_payload_for_non_custom(custom_payload_json: Option<&String>) -> Result<()> {
    if custom_payload_json.is_some() {
        return Err(WorkVcsError::TaskInvalid(
            "--custom-payload-json is only valid with --kind custom".to_owned(),
        ));
    }
    Ok(())
}

fn parse_acceptance_criterion_classification(
    value: &str,
) -> Result<AcceptanceCriterionClassification> {
    match value {
        "required" => Ok(AcceptanceCriterionClassification::Required),
        "optional" => Ok(AcceptanceCriterionClassification::Optional),
        other => Err(WorkVcsError::TaskInvalid(format!(
            "acceptance criterion classification {other:?} is not in the CLI vocabulary"
        ))),
    }
}

fn parse_verification_result(value: &str) -> Result<VerificationResult> {
    match value {
        "passed" => Ok(VerificationResult::Passed),
        "failed" => Ok(VerificationResult::Failed),
        "inconclusive" => Ok(VerificationResult::Inconclusive),
        other => Err(WorkVcsError::TaskInvalid(format!(
            "verification result {other:?} is not in the CLI vocabulary"
        ))),
    }
}

fn parse_verification_applicability(value: &str) -> Result<VerificationApplicability> {
    match value {
        "applicable" => Ok(VerificationApplicability::Applicable),
        "stale" => Ok(VerificationApplicability::Stale),
        "unknown" => Ok(VerificationApplicability::Unknown),
        other => Err(WorkVcsError::TaskInvalid(format!(
            "verification applicability {other:?} is not in the CLI vocabulary"
        ))),
    }
}

fn parse_verification_target_kind(value: &str) -> Result<&'static str> {
    match value {
        "acceptance_criterion" => Ok("acceptance_criterion"),
        "verification_requirement" => Ok("verification_requirement"),
        other => Err(WorkVcsError::TaskInvalid(format!(
            "verification target kind {other:?} is not in the CLI vocabulary"
        ))),
    }
}

fn parse_goal_plan_task_endpoint_kind(value: &str) -> Result<&'static str> {
    match value {
        "goal" => Ok("goal"),
        "plan" => Ok("plan"),
        "task" => Ok("task"),
        other => Err(WorkVcsError::TaskInvalid(format!(
            "goal/plan/task endpoint kind {other:?} is not in the CLI vocabulary"
        ))),
    }
}

fn parse_task_scheduling_relation_type(value: &str) -> Result<&'static str> {
    match value {
        "depends_on" => Ok("depends_on"),
        "ordered_before" => Ok("ordered_before"),
        other => Err(WorkVcsError::TaskInvalid(format!(
            "task scheduling relation type {other:?} is not in the CLI vocabulary"
        ))),
    }
}

fn parse_assumption_record_status(value: &str) -> Result<RecordStatus> {
    match value {
        "validated" => Ok(RecordStatus::Validated),
        "invalidated" => Ok(RecordStatus::Invalidated),
        other => Err(WorkVcsError::RecordInvalid(format!(
            "assumption status {other:?} is not in the CLI transition vocabulary"
        ))),
    }
}

fn parse_attempt_record_status(value: &str) -> Result<RecordStatus> {
    match value {
        "succeeded" => Ok(RecordStatus::Succeeded),
        "failed" => Ok(RecordStatus::Failed),
        "inconclusive" => Ok(RecordStatus::Inconclusive),
        other => Err(WorkVcsError::RecordInvalid(format!(
            "attempt status {other:?} is not in the CLI transition vocabulary"
        ))),
    }
}

fn parse_decision_record_status(value: &str) -> Result<RecordStatus> {
    match value {
        "superseded" => Ok(RecordStatus::Superseded),
        "withdrawn" => Ok(RecordStatus::Withdrawn),
        other => Err(WorkVcsError::RecordInvalid(format!(
            "decision status {other:?} is not in the CLI transition vocabulary"
        ))),
    }
}

fn resolve_record_query_commit(
    engine: &Engine,
    branch: Option<String>,
    commit: Option<String>,
) -> Result<CommitId> {
    match (branch, commit) {
        (Some(branch), None) => Ok(engine
            .branch_head(BranchId::parse_canonical(&branch)?)
            .map(|head| head.head_commit_id)?),
        (None, Some(commit)) => CommitId::parse_canonical(&commit),
        _ => Err(WorkVcsError::QueryInvalid(
            "record query target requires exactly one of --branch or --commit".to_owned(),
        )),
    }
}

fn resolve_knowledge_query_commit(
    engine: &Engine,
    branch: Option<String>,
    commit: Option<String>,
) -> Result<CommitId> {
    match (branch, commit) {
        (Some(branch), None) => Ok(engine
            .branch_head(BranchId::parse_canonical(&branch)?)
            .map(|head| head.head_commit_id)?),
        (None, Some(commit)) => CommitId::parse_canonical(&commit),
        _ => Err(WorkVcsError::QueryInvalid(
            "knowledge query target requires exactly one of --branch or --commit".to_owned(),
        )),
    }
}

fn resolve_goal_query_commit(
    engine: &Engine,
    branch: Option<String>,
    commit: Option<String>,
) -> Result<CommitId> {
    match (branch, commit) {
        (Some(branch), None) => Ok(engine
            .branch_head(BranchId::parse_canonical(&branch)?)
            .map(|head| head.head_commit_id)?),
        (None, Some(commit)) => CommitId::parse_canonical(&commit),
        _ => Err(WorkVcsError::QueryInvalid(
            "goal query target requires exactly one of --branch or --commit".to_owned(),
        )),
    }
}

fn resolve_plan_query_commit(
    engine: &Engine,
    branch: Option<String>,
    commit: Option<String>,
) -> Result<CommitId> {
    match (branch, commit) {
        (Some(branch), None) => Ok(engine
            .branch_head(BranchId::parse_canonical(&branch)?)
            .map(|head| head.head_commit_id)?),
        (None, Some(commit)) => CommitId::parse_canonical(&commit),
        _ => Err(WorkVcsError::QueryInvalid(
            "plan query target requires exactly one of --branch or --commit".to_owned(),
        )),
    }
}

fn resolve_task_query_commit(
    engine: &Engine,
    branch: Option<String>,
    commit: Option<String>,
) -> Result<CommitId> {
    match (branch, commit) {
        (Some(branch), None) => Ok(engine
            .branch_head(BranchId::parse_canonical(&branch)?)
            .map(|head| head.head_commit_id)?),
        (None, Some(commit)) => CommitId::parse_canonical(&commit),
        _ => Err(WorkVcsError::QueryInvalid(
            "task query target requires exactly one of --branch or --commit".to_owned(),
        )),
    }
}

fn parse_knowledge_status(value: &str) -> Result<KnowledgeStatus> {
    match value {
        "active" => Ok(KnowledgeStatus::Active),
        "invalidated" => Ok(KnowledgeStatus::Invalidated),
        "superseded" => Ok(KnowledgeStatus::Superseded),
        other => Err(WorkVcsError::KnowledgeInvalid(format!(
            "knowledge status {other:?} is not in the CLI vocabulary"
        ))),
    }
}

fn parse_record_kind(value: &str) -> Result<RecordKind> {
    match value {
        "assumption" => Ok(RecordKind::Assumption),
        "attempt" => Ok(RecordKind::Attempt),
        "decision" => Ok(RecordKind::Decision),
        "finding" => Ok(RecordKind::Finding),
        "handoff" => Ok(RecordKind::Handoff),
        "question" => Ok(RecordKind::Question),
        "risk" => Ok(RecordKind::Risk),
        other => Err(WorkVcsError::RecordInvalid(format!(
            "record kind {other:?} is not in the CLI vocabulary"
        ))),
    }
}

fn parse_record_status(value: &str) -> Result<RecordStatus> {
    match value {
        "active" => Ok(RecordStatus::Active),
        "failed" => Ok(RecordStatus::Failed),
        "inconclusive" => Ok(RecordStatus::Inconclusive),
        "invalidated" => Ok(RecordStatus::Invalidated),
        "running" => Ok(RecordStatus::Running),
        "succeeded" => Ok(RecordStatus::Succeeded),
        "superseded" => Ok(RecordStatus::Superseded),
        "unverified" => Ok(RecordStatus::Unverified),
        "validated" => Ok(RecordStatus::Validated),
        "withdrawn" => Ok(RecordStatus::Withdrawn),
        other => Err(WorkVcsError::RecordInvalid(format!(
            "record status {other:?} is not in the CLI vocabulary"
        ))),
    }
}

fn parse_record_relation_type(value: &str) -> Result<RecordRelationType> {
    match value {
        "contradicts" => Ok(RecordRelationType::Contradicts),
        "derived_from" => Ok(RecordRelationType::DerivedFrom),
        "invalidates" => Ok(RecordRelationType::Invalidates),
        "related_to" => Ok(RecordRelationType::RelatedTo),
        "supersedes" => Ok(RecordRelationType::Supersedes),
        "supports" => Ok(RecordRelationType::Supports),
        "validates" => Ok(RecordRelationType::Validates),
        other => Err(WorkVcsError::RecordInvalid(format!(
            "record relation type {other:?} is not in the CLI vocabulary"
        ))),
    }
}

fn parse_record_knowledge_relation_type(value: &str) -> Result<RecordRelationType> {
    match value {
        "contradicts" => Ok(RecordRelationType::Contradicts),
        "invalidates" => Ok(RecordRelationType::Invalidates),
        "supports" => Ok(RecordRelationType::Supports),
        "validates" => Ok(RecordRelationType::Validates),
        other => Err(WorkVcsError::RecordInvalid(format!(
            "record knowledge relation type {other:?} is not in the CLI vocabulary"
        ))),
    }
}

fn parse_observation_status(value: &str) -> Result<ApplicabilityResourceObservationStatus> {
    match value {
        "observed" => Ok(ApplicabilityResourceObservationStatus::Observed),
        "unavailable" => Ok(ApplicabilityResourceObservationStatus::Unavailable),
        "error" => Ok(ApplicabilityResourceObservationStatus::Error),
        other => Err(WorkVcsError::TaskInvalid(format!(
            "resource observation status {other:?} is not in the CLI vocabulary"
        ))),
    }
}

fn parse_claim_mode(value: &str) -> Result<ClaimMode> {
    match value {
        "exclusive" => Ok(ClaimMode::Exclusive),
        "shared" => Ok(ClaimMode::Shared),
        other => Err(WorkVcsError::ClaimInvalid(format!(
            "claim mode {other:?} is not supported"
        ))),
    }
}

fn parse_claim_guard_action(value: &str) -> Result<ClaimGuardAction> {
    match value {
        "terminal-task" | "terminal_task" => Ok(ClaimGuardAction::TerminalTaskMutation),
        "structural-task" | "structural_task" => Ok(ClaimGuardAction::StructuralTaskMutation),
        other => Err(WorkVcsError::ClaimInvalid(format!(
            "claim guard action {other:?} is not supported"
        ))),
    }
}

fn parse_cli_object(label: &str, json: &str) -> Result<CanonicalValue> {
    let value = parse_canonical_json(json.as_bytes())?;
    if matches!(value, CanonicalValue::Object(_)) {
        Ok(value)
    } else {
        Err(WorkVcsError::TaskInvalid(format!(
            "{label} must be an object"
        )))
    }
}

fn canonical_json_input_bytes(
    label: &str,
    json: Option<String>,
    json_file: Option<PathBuf>,
) -> Result<Vec<u8>> {
    match (json, json_file) {
        (Some(json), None) => Ok(json.into_bytes()),
        (None, Some(path)) => read_cli_file(label, &path),
        _ => Err(WorkVcsError::QueryInvalid(format!(
            "{label} requires exactly one of --json or --json-file"
        ))),
    }
}

fn render_canonical_encode(json: &[u8]) -> Result<String> {
    let value = parse_canonical_json(json)?;
    let canonical_json = canonical_cli_json("canonical JSON", &value)?;
    Ok(format!(
        "canonical_json={canonical_json}\nsize_bytes={}\n",
        canonical_json.len()
    ))
}

fn render_canonical_digest(domain: &str, json: &[u8]) -> Result<String> {
    let value = parse_canonical_json(json)?;
    let canonical_json = canonical_cli_json("canonical JSON", &value)?;
    let digest = match domain {
        "entity-version" => entity_version_digest(&value)?,
        "relation-version" => relation_version_digest(&value)?,
        other => {
            return Err(WorkVcsError::DigestInvalid(format!(
                "canonical digest domain {other:?} is not supported"
            )));
        }
    };
    Ok(format!(
        "domain={domain}\ndigest={digest}\ncanonical_json={canonical_json}\nsize_bytes={}\n",
        canonical_json.len()
    ))
}

fn render_content_digest(
    content: Option<String>,
    content_hex: Option<String>,
    content_file: Option<PathBuf>,
) -> Result<String> {
    let bytes = match (content, content_hex, content_file) {
        (Some(content), None, None) => content.into_bytes(),
        (None, Some(content_hex), None) => hex::decode(&content_hex).map_err(|error| {
            WorkVcsError::DigestInvalid(format!("content hex decode failed: {error}"))
        })?,
        (None, None, Some(path)) => read_cli_file("canonical content", &path)?,
        _ => {
            return Err(WorkVcsError::DigestInvalid(
                "content digest requires exactly one of --content, --content-hex, or --content-file"
                    .to_owned(),
            ));
        }
    };
    Ok(format!(
        "content_digest={}\nsize_bytes={}\n",
        content_object_digest(&bytes),
        bytes.len()
    ))
}

fn read_cli_file(label: &str, path: &Path) -> Result<Vec<u8>> {
    fs::read(path).map_err(|error| {
        WorkVcsError::QueryInvalid(format!(
            "failed to read {label} file {}: {error}",
            path.display()
        ))
    })
}

fn render_new_id(kind: &str) -> Result<String> {
    let id = match kind {
        "store" => StoreId::new_v7().to_string(),
        "workspace" => WorkspaceId::new_v7().to_string(),
        "entity" => EntityId::new_v7().to_string(),
        "entity-version" => EntityVersionId::new_v7().to_string(),
        "exposure" => ExposureId::new_v7().to_string(),
        "exposure-transition" => ExposureTransitionId::new_v7().to_string(),
        "external-object" => ExternalObjectId::new_v7().to_string(),
        "external-ref" => ExternalRefId::new_v7().to_string(),
        "external-version" => ExternalVersionId::new_v7().to_string(),
        "evidence" => EvidenceId::new_v7().to_string(),
        "resource" => ResourceId::new_v7().to_string(),
        "resource-observation" => ResourceObservationId::new_v7().to_string(),
        "knowledge-space" => KnowledgeSpaceId::new_v7().to_string(),
        "relation" => RelationId::new_v7().to_string(),
        "relation-version" => RelationVersionId::new_v7().to_string(),
        "branch" => BranchId::new_v7().to_string(),
        "commit" => CommitId::new_v7().to_string(),
        "changeset" => ChangeSetId::new_v7().to_string(),
        "checkpoint" => CheckpointId::new_v7().to_string(),
        "import" => ImportId::new_v7().to_string(),
        "lineage" => LineageId::new_v7().to_string(),
        "migration" => MigrationId::new_v7().to_string(),
        "operation" => OperationId::new_v7().to_string(),
        "event" => EventId::new_v7().to_string(),
        "session" => SessionId::new_v7().to_string(),
        "session-diff" => SessionDiffId::new_v7().to_string(),
        "claim" => ClaimId::new_v7().to_string(),
        "merge" => MergeId::new_v7().to_string(),
        "merge-item" => MergeItemId::new_v7().to_string(),
        other => {
            return Err(WorkVcsError::IdentityInvalid(format!(
                "id kind {other:?} is not supported"
            )));
        }
    };
    Ok(format!("kind={kind}\nid={id}\n"))
}

fn render_store_info(info: &StoreInfo) -> Result<String> {
    Ok(format!(
        "store_id={}\ndisplay_name={}\ncreated_at_us={}\nstore_format_version={}\nschema_version={}\nobject_store_format_version={}\nid_scheme={}\ndigest_algorithm={}\ncanonical_json_profile={}\nmanifest_json={}\n",
        info.store_id,
        info.display_name,
        info.created_at_us,
        info.manifest.store_format_version,
        info.manifest.schema_version,
        info.manifest.object_store_format_version,
        info.manifest.id_scheme,
        info.manifest.digest_algorithm,
        info.manifest.canonical_json_profile,
        info.manifest.canonical_manifest_json()?
    ))
}

fn render_integrity_report(report: &IntegrityReport) -> String {
    format!(
        "checked_branches={}\nchecked_commits={}\nchecked_changesets={}\nchecked_change_operations={}\nchecked_changeset_causal_anchors={}\nchecked_events={}\nchecked_checkpoints={}\ninvalid_checkpoints={}\n",
        report.checked_branches,
        report.checked_commits,
        report.checked_changesets,
        report.checked_change_operations,
        report.checked_changeset_causal_anchors,
        report.checked_events,
        report.checked_checkpoints,
        report.invalid_checkpoints
    )
}

fn render_work_state_mapping_digest(
    entity_mappings: Vec<String>,
    relation_mappings: Vec<String>,
) -> Result<String> {
    let entities = entity_mappings
        .iter()
        .map(|mapping| parse_work_state_entity_mapping(mapping))
        .collect::<Result<Vec<_>>>()?;
    let relations = relation_mappings
        .iter()
        .map(|mapping| parse_work_state_relation_mapping(mapping))
        .collect::<Result<Vec<_>>>()?;
    let state = WorkState::new(entities, relations)?;
    Ok(format!(
        "work_state_digest={}\nentities={}\nrelations={}\n",
        work_state_mapping_digest(&state),
        state.entities().len(),
        state.relations().len()
    ))
}

fn parse_work_state_entity_mapping(value: &str) -> Result<(EntityId, EntityVersionId)> {
    let (entity_id, version_id) = split_work_state_mapping("entity", value)?;
    Ok((
        EntityId::parse_canonical(entity_id)?,
        EntityVersionId::parse_canonical(version_id)?,
    ))
}

fn parse_work_state_relation_mapping(value: &str) -> Result<(RelationId, RelationVersionId)> {
    let (relation_id, version_id) = split_work_state_mapping("relation", value)?;
    Ok((
        RelationId::parse_canonical(relation_id)?,
        RelationVersionId::parse_canonical(version_id)?,
    ))
}

fn split_work_state_mapping<'a>(label: &str, value: &'a str) -> Result<(&'a str, &'a str)> {
    let (subject, version) = value.split_once('=').ok_or_else(|| {
        WorkVcsError::CanonicalEncodingInvalid(format!(
            "{label} mapping must use SUBJECT_ID=VERSION_ID"
        ))
    })?;
    if subject.is_empty() || version.is_empty() || version.contains('=') {
        return Err(WorkVcsError::CanonicalEncodingInvalid(format!(
            "{label} mapping must use SUBJECT_ID=VERSION_ID"
        )));
    }
    Ok((subject, version))
}

struct EvidenceContentArgs {
    role: Option<String>,
    content: Option<String>,
    content_digest: Option<String>,
    content_file: Option<PathBuf>,
    content_size_bytes: Option<i64>,
    media_type: Option<String>,
    format_metadata_json: String,
}

fn evidence_content_from_cli(args: EvidenceContentArgs) -> Result<Option<EvidenceContentInput>> {
    let has_content_args = args.role.is_some()
        || args.content.is_some()
        || args.content_digest.is_some()
        || args.content_file.is_some()
        || args.content_size_bytes.is_some()
        || args.media_type.is_some()
        || args.format_metadata_json != "{}";
    if !has_content_args {
        return Ok(None);
    }

    let role = required_arg("--content-role", args.role)?;
    let mut content = match (
        args.content,
        args.content_digest,
        args.content_file,
        args.content_size_bytes,
    ) {
        (Some(content), None, None, None) => {
            EvidenceContentInput::from_raw_bytes(role, content.as_bytes())?
        }
        (None, Some(content_digest), None, Some(content_size_bytes)) => {
            EvidenceContentInput::from_digest(
                role,
                Digest::from_hex(&content_digest)?,
                content_size_bytes,
            )?
        }
        (None, None, Some(path), None) => {
            let bytes = read_cli_file("evidence content", &path)?;
            EvidenceContentInput::from_raw_bytes(role, bytes)?
        }
        _ => {
            return Err(WorkVcsError::EvidenceInvalid(
                "evidence content requires exactly one of --content, --content-file, or --content-digest with --content-size-bytes"
                    .to_owned(),
            ));
        }
    };
    if let Some(media_type) = args.media_type {
        content = content.with_media_type(media_type)?;
    }
    content = content.with_format_metadata(parse_cli_object(
        "evidence content format metadata",
        &args.format_metadata_json,
    )?)?;
    Ok(Some(content))
}

fn fingerprint_from_cli(
    fingerprint: Option<String>,
    content: Option<String>,
    content_file: Option<PathBuf>,
) -> Result<Digest> {
    match (fingerprint, content, content_file) {
        (Some(fingerprint), None, None) => Digest::from_hex(&fingerprint),
        (None, Some(content), None) => Ok(content_object_digest(content.as_bytes())),
        (None, None, Some(path)) => {
            let bytes = read_cli_file("resource observation content", &path)?;
            Ok(content_object_digest(&bytes))
        }
        _ => Err(WorkVcsError::TaskInvalid(
            "expected exactly one fingerprint source".to_owned(),
        )),
    }
}

struct ResourceObservationDetailArgs {
    content: Option<String>,
    content_file: Option<PathBuf>,
    content_digest: Option<String>,
    content_size_bytes: Option<i64>,
    media_type: Option<String>,
    format_metadata_json: String,
}

fn resource_observation_detail_from_cli(
    args: ResourceObservationDetailArgs,
) -> Result<Option<ResourceObservationDetailInput>> {
    let has_detail_args = args.content.is_some()
        || args.content_file.is_some()
        || args.content_digest.is_some()
        || args.content_size_bytes.is_some()
        || args.media_type.is_some()
        || args.format_metadata_json != "{}";
    if !has_detail_args {
        return Ok(None);
    }

    let mut detail = match (
        args.content,
        args.content_file,
        args.content_digest,
        args.content_size_bytes,
    ) {
        (Some(content), None, None, None) => {
            ResourceObservationDetailInput::from_raw_bytes(content.as_bytes())?
        }
        (None, Some(path), None, None) => {
            let bytes = read_cli_file("resource observation detail content", &path)?;
            ResourceObservationDetailInput::from_raw_bytes(bytes)?
        }
        (None, None, Some(content_digest), Some(content_size_bytes)) => {
            ResourceObservationDetailInput::from_digest(
                Digest::from_hex(&content_digest)?,
                content_size_bytes,
            )?
        }
        _ => {
            return Err(WorkVcsError::ResourceInvalid(
                "resource observation detail requires exactly one of --detail-content, --detail-content-file, or --detail-content-digest with --detail-content-size-bytes"
                    .to_owned(),
            ));
        }
    };
    if let Some(media_type) = args.media_type {
        detail = detail.with_media_type(media_type)?;
    }
    detail = detail.with_format_metadata(parse_cli_object(
        "resource observation detail format metadata",
        &args.format_metadata_json,
    )?)?;
    Ok(Some(detail))
}

fn required_arg<T>(label: &str, value: Option<T>) -> Result<T> {
    value.ok_or_else(|| WorkVcsError::TaskInvalid(format!("{label} is required")))
}

struct ResourceBasisArgs {
    resource: Option<String>,
    adapter_kind: Option<String>,
    adapter_schema_version: Option<i64>,
    scope_kind: Option<String>,
    scope_schema_version: Option<i64>,
    scope_payload_json: Option<String>,
    baseline_fingerprint: Option<String>,
    baseline_observation: Option<String>,
}

fn resource_basis_from_cli(args: ResourceBasisArgs) -> Result<VerificationResourceBasis> {
    let mut basis = VerificationResourceBasis::new(
        ResourceId::parse_canonical(&required_arg("--resource", args.resource)?)?,
        required_arg("--adapter-kind", args.adapter_kind)?,
        required_arg("--adapter-schema-version", args.adapter_schema_version)?,
        required_arg("--scope-kind", args.scope_kind)?,
        required_arg("--scope-schema-version", args.scope_schema_version)?,
        parse_cli_object(
            "resource basis scope payload",
            &required_arg("--scope-payload-json", args.scope_payload_json)?,
        )?,
        Digest::from_hex(&required_arg(
            "--baseline-fingerprint",
            args.baseline_fingerprint,
        )?)?,
    )?;
    if let Some(baseline_observation) = args.baseline_observation {
        basis = basis.with_baseline_observation_id(ResourceObservationId::parse_canonical(
            &baseline_observation,
        )?)?;
    }
    Ok(basis)
}

fn applicability_stamp_from_cli(
    resource_basis_ordinal: i64,
    adapter_kind: String,
    adapter_schema_version: i64,
    scope_schema_version: i64,
    observation_status: &str,
    observed_fingerprint: Option<String>,
    observation: Option<String>,
) -> Result<ApplicabilityResourceStampInput> {
    let mut stamp = match parse_observation_status(observation_status)? {
        ApplicabilityResourceObservationStatus::Observed => {
            ApplicabilityResourceStampInput::observed(
                resource_basis_ordinal,
                adapter_kind,
                adapter_schema_version,
                scope_schema_version,
                Digest::from_hex(&required_arg(
                    "--observed-fingerprint",
                    observed_fingerprint,
                )?)?,
            )?
        }
        ApplicabilityResourceObservationStatus::Unavailable => {
            if observed_fingerprint.is_some() || observation.is_some() {
                return Err(WorkVcsError::TaskInvalid(
                    "unavailable resource stamp must not include observed data".to_owned(),
                ));
            }
            ApplicabilityResourceStampInput::unavailable(
                resource_basis_ordinal,
                adapter_kind,
                adapter_schema_version,
                scope_schema_version,
            )?
        }
        ApplicabilityResourceObservationStatus::Error => {
            if observed_fingerprint.is_some() || observation.is_some() {
                return Err(WorkVcsError::TaskInvalid(
                    "error resource stamp must not include observed data".to_owned(),
                ));
            }
            ApplicabilityResourceStampInput::error(
                resource_basis_ordinal,
                adapter_kind,
                adapter_schema_version,
                scope_schema_version,
            )?
        }
    };
    if let Some(observation) = observation {
        stamp = stamp.with_observation_id(ResourceObservationId::parse_canonical(&observation)?)?;
    }
    Ok(stamp)
}

fn method_value(name: &str) -> CanonicalValue {
    CanonicalValue::object(vec![
        (
            "kind".to_owned(),
            CanonicalValue::String("manual".to_owned()),
        ),
        ("name".to_owned(), CanonicalValue::String(name.to_owned())),
    ])
    .expect("method object")
}

fn render_workspace_info(workspace: &WorkspaceInfo) -> String {
    format!(
        "workspace_id={}\ndisplay_name={}\nbranch_id={}\nbranch_name={}\ngenesis_commit_id={}\ngenesis_changeset_id={}\nstate_digest={}\ncreated_at_us={}\n",
        workspace.workspace_id,
        workspace.display_name,
        workspace.initial_branch_id,
        workspace.initial_branch_name,
        workspace.genesis_commit_id,
        workspace.genesis_changeset_id,
        workspace.state_digest,
        workspace.created_at_us
    )
}

fn render_workspace_list(result: &WorkspaceListResult) -> String {
    let mut output = format!("workspaces={}\n", result.workspaces.len());
    for (index, workspace) in result.workspaces.iter().enumerate() {
        let _ = writeln!(
            output,
            "workspace.{index}.workspace_id={}",
            workspace.workspace_id
        );
        let _ = writeln!(
            output,
            "workspace.{index}.display_name={}",
            workspace.display_name
        );
        let _ = writeln!(
            output,
            "workspace.{index}.branch_id={}",
            workspace.initial_branch_id
        );
        let _ = writeln!(
            output,
            "workspace.{index}.branch_name={}",
            workspace.initial_branch_name
        );
        let _ = writeln!(
            output,
            "workspace.{index}.genesis_commit_id={}",
            workspace.genesis_commit_id
        );
        let _ = writeln!(
            output,
            "workspace.{index}.genesis_changeset_id={}",
            workspace.genesis_changeset_id
        );
        let _ = writeln!(
            output,
            "workspace.{index}.state_digest={}",
            workspace.state_digest
        );
        let _ = writeln!(
            output,
            "workspace.{index}.created_at_us={}",
            workspace.created_at_us
        );
    }
    output
}

fn render_branch_head(head: &BranchHead) -> String {
    format!(
        "workspace_id={}\nbranch_id={}\nbranch_name={}\nhead_commit_id={}\nhead_changeset_id={}\nhead_commit_kind={}\nhead_operation_type={}\nhead_operation_schema_version={}\nhead_committed_at_us={}\nhead_changeset_created_at_us={}\nlifecycle_state={}\nstate_digest={}\n",
        head.workspace_id,
        head.branch_id,
        head.name,
        head.head_commit_id,
        head.head_changeset_id,
        head.head_commit_kind,
        head.head_operation_type,
        head.head_operation_schema_version,
        head.head_committed_at_us,
        head.head_changeset_created_at_us,
        head.lifecycle_state,
        head.state_digest
    )
}

fn render_branch_list(branches: &[BranchHead]) -> String {
    let workspace_id = branches
        .first()
        .map(|branch| branch.workspace_id.to_string())
        .unwrap_or_else(|| "none".to_owned());
    let mut output = format!("workspace_id={workspace_id}\nbranches={}\n", branches.len());
    for (index, branch) in branches.iter().enumerate() {
        let _ = writeln!(output, "branch.{index}.branch_id={}", branch.branch_id);
        let _ = writeln!(output, "branch.{index}.branch_name={}", branch.name);
        let _ = writeln!(
            output,
            "branch.{index}.head_commit_id={}",
            branch.head_commit_id
        );
        let _ = writeln!(
            output,
            "branch.{index}.head_changeset_id={}",
            branch.head_changeset_id
        );
        let _ = writeln!(
            output,
            "branch.{index}.head_commit_kind={}",
            branch.head_commit_kind
        );
        let _ = writeln!(
            output,
            "branch.{index}.head_operation_type={}",
            branch.head_operation_type
        );
        let _ = writeln!(
            output,
            "branch.{index}.head_operation_schema_version={}",
            branch.head_operation_schema_version
        );
        let _ = writeln!(
            output,
            "branch.{index}.head_committed_at_us={}",
            branch.head_committed_at_us
        );
        let _ = writeln!(
            output,
            "branch.{index}.head_changeset_created_at_us={}",
            branch.head_changeset_created_at_us
        );
        let _ = writeln!(
            output,
            "branch.{index}.lifecycle_state={}",
            branch.lifecycle_state
        );
        let _ = writeln!(
            output,
            "branch.{index}.state_digest={}",
            branch.state_digest
        );
    }
    output
}

fn render_branch_fork(branch: &BranchForkResult) -> String {
    let source_branch_id = branch
        .source_branch_id
        .map(|branch_id| branch_id.to_string())
        .unwrap_or_else(|| "none".to_owned());
    format!(
        "workspace_id={}\nbranch_id={}\nbranch_name={}\nsource_branch_id={}\nhead_commit_id={}\nstate_digest={}\nevent_id={}\ncreated_at_us={}\n",
        branch.workspace_id,
        branch.branch_id,
        branch.name,
        source_branch_id,
        branch.head_commit_id,
        branch.state_digest,
        branch.event_id,
        branch.created_at_us
    )
}

fn render_knowledge_create(knowledge: &KnowledgeCreateCommit) -> Result<String> {
    let statement_json = knowledge_statement_json(&knowledge.state.statement)?;
    let scope_json = knowledge_value_json("knowledge scope", &knowledge.state.scope)?;
    let provenance_json =
        knowledge_value_json("knowledge provenance", &knowledge.state.provenance)?;
    Ok(format!(
        "workspace_id={}\nbranch_id={}\nprevious_head_commit_id={}\ncommit_id={}\nchangeset_id={}\noperation_id={}\nknowledge_entity_id={}\nknowledge_entity_version_id={}\nknowledge_state_digest={}\nwork_state_digest={}\nknowledge_status={}\nknowledge_statement_json={}\nknowledge_scope_json={}\nknowledge_provenance_json={}\n",
        knowledge.workspace_id,
        knowledge.branch_id,
        knowledge.previous_head_commit_id,
        knowledge.commit_id,
        knowledge.changeset_id,
        knowledge.operation_id,
        knowledge.knowledge_entity_id,
        knowledge.knowledge_entity_version_id,
        knowledge.knowledge_state_digest,
        knowledge.work_state_digest,
        knowledge.state.status,
        statement_json,
        scope_json,
        provenance_json
    ))
}

fn render_knowledge_snapshot(knowledge: &KnowledgeSnapshot) -> Result<String> {
    let statement_json = knowledge_statement_json(&knowledge.state.statement)?;
    let scope_json = knowledge_value_json("knowledge scope", &knowledge.state.scope)?;
    let provenance_json =
        knowledge_value_json("knowledge provenance", &knowledge.state.provenance)?;
    Ok(format!(
        "workspace_id={}\ncommit_id={}\nknowledge_entity_id={}\nknowledge_entity_version_id={}\nknowledge_state_digest={}\nknowledge_status={}\nknowledge_statement_json={}\nknowledge_scope_json={}\nknowledge_provenance_json={}\n",
        knowledge.workspace_id,
        knowledge.commit_id,
        knowledge.knowledge_entity_id,
        knowledge.knowledge_entity_version_id,
        knowledge.state_digest,
        knowledge.state.status,
        statement_json,
        scope_json,
        provenance_json
    ))
}

fn render_knowledge_transition(knowledge: &KnowledgeTransitionCommit) -> Result<String> {
    let statement_json = knowledge_statement_json(&knowledge.state.statement)?;
    let scope_json = knowledge_value_json("knowledge scope", &knowledge.state.scope)?;
    let provenance_json =
        knowledge_value_json("knowledge provenance", &knowledge.state.provenance)?;
    Ok(format!(
        "workspace_id={}\nbranch_id={}\nprevious_head_commit_id={}\ncommit_id={}\nchangeset_id={}\noperation_id={}\nknowledge_entity_id={}\nprevious_knowledge_entity_version_id={}\nknowledge_entity_version_id={}\nknowledge_state_digest={}\nwork_state_digest={}\nprevious_knowledge_status={}\nknowledge_status={}\nknowledge_statement_json={}\nknowledge_scope_json={}\nknowledge_provenance_json={}\n",
        knowledge.workspace_id,
        knowledge.branch_id,
        knowledge.previous_head_commit_id,
        knowledge.commit_id,
        knowledge.changeset_id,
        knowledge.operation_id,
        knowledge.knowledge_entity_id,
        knowledge.previous_knowledge_entity_version_id,
        knowledge.knowledge_entity_version_id,
        knowledge.knowledge_state_digest,
        knowledge.work_state_digest,
        knowledge.previous_state.status,
        knowledge.state.status,
        statement_json,
        scope_json,
        provenance_json
    ))
}

fn render_knowledge_relation_create(relation: &KnowledgeRelationCreateCommit) -> String {
    format!(
        "workspace_id={}\nbranch_id={}\nprevious_head_commit_id={}\ncommit_id={}\nchangeset_id={}\noperation_id={}\nrelation_id={}\nrelation_version_id={}\nrelation_type={}\nreplacement_knowledge_entity_id={}\nprior_knowledge_entity_id={}\nrelation_state_digest={}\nwork_state_digest={}\n",
        relation.workspace_id,
        relation.branch_id,
        relation.previous_head_commit_id,
        relation.commit_id,
        relation.changeset_id,
        relation.operation_id,
        relation.relation_id,
        relation.relation_version_id,
        relation.relation_type,
        relation.replacement_knowledge_entity_id,
        relation.prior_knowledge_entity_id,
        relation.relation_state_digest,
        relation.work_state_digest
    )
}

fn render_knowledge_relation_list(result: &KnowledgeRelationListResult) -> String {
    let mut output = format!(
        "workspace_id={}\ncommit_id={}\nrelations={}\n",
        result.workspace_id,
        result.commit_id,
        result.relations.len()
    );
    for (index, relation) in result.relations.iter().enumerate() {
        writeln!(
            output,
            "relation.{index}.relation_id={}",
            relation.relation_id
        )
        .expect("write to String");
        writeln!(
            output,
            "relation.{index}.relation_version_id={}",
            relation.relation_version_id
        )
        .expect("write to String");
        writeln!(
            output,
            "relation.{index}.relation_type={}",
            relation.relation_type
        )
        .expect("write to String");
        writeln!(
            output,
            "relation.{index}.replacement_knowledge_entity_id={}",
            relation.replacement_knowledge_entity_id
        )
        .expect("write to String");
        writeln!(
            output,
            "relation.{index}.prior_knowledge_entity_id={}",
            relation.prior_knowledge_entity_id
        )
        .expect("write to String");
        writeln!(
            output,
            "relation.{index}.relation_state_digest={}",
            relation.state_digest
        )
        .expect("write to String");
    }
    output
}

fn render_knowledge_relation_snapshot(relation: &KnowledgeRelationSnapshot) -> String {
    format!(
        "workspace_id={}\ncommit_id={}\nrelation_id={}\nrelation_version_id={}\nrelation_type={}\nreplacement_knowledge_entity_id={}\nprior_knowledge_entity_id={}\nrelation_state_digest={}\n",
        relation.workspace_id,
        relation.commit_id,
        relation.relation_id,
        relation.relation_version_id,
        relation.relation_type,
        relation.replacement_knowledge_entity_id,
        relation.prior_knowledge_entity_id,
        relation.state_digest
    )
}

fn render_knowledge_relation_remove(relation: &KnowledgeRelationRemoveCommit) -> String {
    format!(
        "workspace_id={}\nbranch_id={}\nprevious_head_commit_id={}\ncommit_id={}\nchangeset_id={}\noperation_id={}\nrelation_id={}\nprevious_relation_version_id={}\nrelation_type={}\nreplacement_knowledge_entity_id={}\nprior_knowledge_entity_id={}\nwork_state_digest={}\n",
        relation.workspace_id,
        relation.branch_id,
        relation.previous_head_commit_id,
        relation.commit_id,
        relation.changeset_id,
        relation.operation_id,
        relation.relation_id,
        relation.previous_relation_version_id,
        relation.relation_type,
        relation.replacement_knowledge_entity_id,
        relation.prior_knowledge_entity_id,
        relation.work_state_digest
    )
}

fn render_knowledge_relation_restore(relation: &KnowledgeRelationRestoreCommit) -> String {
    format!(
        "workspace_id={}\nbranch_id={}\nprevious_head_commit_id={}\ncommit_id={}\nchangeset_id={}\noperation_id={}\nrelation_id={}\nrelation_version_id={}\nrelation_type={}\nreplacement_knowledge_entity_id={}\nprior_knowledge_entity_id={}\nrelation_state_digest={}\nwork_state_digest={}\n",
        relation.workspace_id,
        relation.branch_id,
        relation.previous_head_commit_id,
        relation.commit_id,
        relation.changeset_id,
        relation.operation_id,
        relation.relation_id,
        relation.relation_version_id,
        relation.relation_type,
        relation.replacement_knowledge_entity_id,
        relation.prior_knowledge_entity_id,
        relation.relation_state_digest,
        relation.work_state_digest
    )
}

fn render_knowledge_list(result: &KnowledgeListResult) -> Result<String> {
    let mut output = format!(
        "workspace_id={}\ncommit_id={}\nknowledge={}\n",
        result.workspace_id,
        result.commit_id,
        result.knowledge.len()
    );
    for (index, knowledge) in result.knowledge.iter().enumerate() {
        writeln!(
            output,
            "knowledge.{index}.knowledge_entity_id={}",
            knowledge.knowledge_entity_id
        )
        .expect("write to String");
        writeln!(
            output,
            "knowledge.{index}.knowledge_entity_version_id={}",
            knowledge.knowledge_entity_version_id
        )
        .expect("write to String");
        writeln!(
            output,
            "knowledge.{index}.knowledge_state_digest={}",
            knowledge.state_digest
        )
        .expect("write to String");
        writeln!(
            output,
            "knowledge.{index}.knowledge_status={}",
            knowledge.state.status
        )
        .expect("write to String");
        writeln!(
            output,
            "knowledge.{index}.knowledge_statement_json={}",
            knowledge_statement_json(&knowledge.state.statement)?
        )
        .expect("write to String");
        writeln!(
            output,
            "knowledge.{index}.knowledge_scope_json={}",
            knowledge_value_json("knowledge scope", &knowledge.state.scope)?
        )
        .expect("write to String");
        writeln!(
            output,
            "knowledge.{index}.knowledge_provenance_json={}",
            knowledge_value_json("knowledge provenance", &knowledge.state.provenance)?
        )
        .expect("write to String");
    }
    Ok(output)
}

fn knowledge_statement_json(statement: &str) -> Result<String> {
    serde_json::to_string(statement).map_err(|error| {
        WorkVcsError::KnowledgeInvalid(format!("knowledge statement encode failed: {error}"))
    })
}

fn knowledge_value_json(label: &str, value: &CanonicalValue) -> Result<String> {
    String::from_utf8(canonical_bytes(value)?).map_err(|error| {
        WorkVcsError::KnowledgeInvalid(format!("{label} encode produced non-UTF-8: {error}"))
    })
}

fn canonical_text_json(label: &str, value: &str) -> Result<String> {
    knowledge_value_json(label, &CanonicalValue::String(value.to_owned()))
}

fn canonical_optional_text_json(label: &str, value: Option<&str>) -> Result<String> {
    let value = value
        .map(|value| CanonicalValue::String(value.to_owned()))
        .unwrap_or(CanonicalValue::Null);
    knowledge_value_json(label, &value)
}

fn canonical_string_array_json(label: &str, values: &[String]) -> Result<String> {
    let value = CanonicalValue::Array(
        values
            .iter()
            .map(|value| CanonicalValue::String(value.clone()))
            .collect(),
    );
    knowledge_value_json(label, &value)
}

fn render_goal_create(goal: &GoalCreateCommit) -> String {
    format!(
        "workspace_id={}\nbranch_id={}\nprevious_head_commit_id={}\ncommit_id={}\nchangeset_id={}\noperation_id={}\ngoal_entity_id={}\ngoal_entity_version_id={}\ngoal_state_digest={}\nwork_state_digest={}\nstatus={}\n",
        goal.workspace_id,
        goal.branch_id,
        goal.previous_head_commit_id,
        goal.commit_id,
        goal.changeset_id,
        goal.operation_id,
        goal.goal_entity_id,
        goal.goal_entity_version_id,
        goal.goal_state_digest,
        goal.work_state_digest,
        goal.state.status
    )
}

fn render_goal_snapshot(goal: &GoalSnapshot) -> Result<String> {
    let description_json = canonical_text_json("goal description", &goal.state.description)?;
    let terminal_rationale_json = canonical_optional_text_json(
        "goal terminal_rationale",
        goal.state.terminal_rationale.as_deref(),
    )?;
    Ok(format!(
        "workspace_id={}\ncommit_id={}\ngoal_entity_id={}\ngoal_entity_version_id={}\ngoal_state_digest={}\nstatus={}\ndescription_json={}\nterminal_rationale_json={}\n",
        goal.workspace_id,
        goal.commit_id,
        goal.goal_entity_id,
        goal.goal_entity_version_id,
        goal.state_digest,
        goal.state.status,
        description_json,
        terminal_rationale_json
    ))
}

fn render_goal_list(commit_id: CommitId, goals: &[GoalSnapshot]) -> Result<String> {
    let mut output = format!("commit_id={commit_id}\ngoals={}\n", goals.len());
    for (index, goal) in goals.iter().enumerate() {
        writeln!(output, "goal.{index}.workspace_id={}", goal.workspace_id)
            .expect("write to String");
        writeln!(
            output,
            "goal.{index}.goal_entity_id={}",
            goal.goal_entity_id
        )
        .expect("write to String");
        writeln!(
            output,
            "goal.{index}.goal_entity_version_id={}",
            goal.goal_entity_version_id
        )
        .expect("write to String");
        writeln!(
            output,
            "goal.{index}.goal_state_digest={}",
            goal.state_digest
        )
        .expect("write to String");
        writeln!(output, "goal.{index}.status={}", goal.state.status).expect("write to String");
        writeln!(
            output,
            "goal.{index}.description_json={}",
            canonical_text_json("goal description", &goal.state.description)?
        )
        .expect("write to String");
        writeln!(
            output,
            "goal.{index}.terminal_rationale_json={}",
            canonical_optional_text_json(
                "goal terminal_rationale",
                goal.state.terminal_rationale.as_deref()
            )?
        )
        .expect("write to String");
    }
    Ok(output)
}

fn render_goal_transition(goal: &GoalTransitionCommit) -> String {
    format!(
        "workspace_id={}\nbranch_id={}\nprevious_head_commit_id={}\ncommit_id={}\nchangeset_id={}\noperation_id={}\ngoal_entity_id={}\nprevious_goal_entity_version_id={}\ngoal_entity_version_id={}\ngoal_state_digest={}\nwork_state_digest={}\nprevious_status={}\nstatus={}\n",
        goal.workspace_id,
        goal.branch_id,
        goal.previous_head_commit_id,
        goal.commit_id,
        goal.changeset_id,
        goal.operation_id,
        goal.goal_entity_id,
        goal.previous_goal_entity_version_id,
        goal.goal_entity_version_id,
        goal.goal_state_digest,
        goal.work_state_digest,
        goal.previous_state.status,
        goal.state.status
    )
}

fn render_plan_create(plan: &PlanCreateCommit) -> String {
    format!(
        "workspace_id={}\nbranch_id={}\nprevious_head_commit_id={}\ncommit_id={}\nchangeset_id={}\noperation_id={}\nplan_entity_id={}\nplan_entity_version_id={}\nplan_state_digest={}\nwork_state_digest={}\nstatus={}\nconstraints={}\n",
        plan.workspace_id,
        plan.branch_id,
        plan.previous_head_commit_id,
        plan.commit_id,
        plan.changeset_id,
        plan.operation_id,
        plan.plan_entity_id,
        plan.plan_entity_version_id,
        plan.plan_state_digest,
        plan.work_state_digest,
        plan.state.status,
        plan.state.constraints.len()
    )
}

fn render_plan_snapshot(plan: &PlanSnapshot) -> Result<String> {
    let description_json = canonical_text_json("plan description", &plan.state.description)?;
    let strategy_json = canonical_text_json("plan strategy", &plan.state.strategy)?;
    let constraints_json =
        canonical_string_array_json("plan constraints", &plan.state.constraints)?;
    let completion_rationale_json = canonical_optional_text_json(
        "plan completion_rationale",
        plan.state.completion_rationale.as_deref(),
    )?;
    Ok(format!(
        "workspace_id={}\ncommit_id={}\nplan_entity_id={}\nplan_entity_version_id={}\nplan_state_digest={}\nstatus={}\ndescription_json={}\nstrategy_json={}\nconstraints={}\nconstraints_json={}\ncompletion_rationale_json={}\n",
        plan.workspace_id,
        plan.commit_id,
        plan.plan_entity_id,
        plan.plan_entity_version_id,
        plan.state_digest,
        plan.state.status,
        description_json,
        strategy_json,
        plan.state.constraints.len(),
        constraints_json,
        completion_rationale_json
    ))
}

fn render_plan_list(commit_id: CommitId, plans: &[PlanSnapshot]) -> Result<String> {
    let mut output = format!("commit_id={commit_id}\nplans={}\n", plans.len());
    for (index, plan) in plans.iter().enumerate() {
        writeln!(output, "plan.{index}.workspace_id={}", plan.workspace_id)
            .expect("write to String");
        writeln!(
            output,
            "plan.{index}.plan_entity_id={}",
            plan.plan_entity_id
        )
        .expect("write to String");
        writeln!(
            output,
            "plan.{index}.plan_entity_version_id={}",
            plan.plan_entity_version_id
        )
        .expect("write to String");
        writeln!(
            output,
            "plan.{index}.plan_state_digest={}",
            plan.state_digest
        )
        .expect("write to String");
        writeln!(output, "plan.{index}.status={}", plan.state.status).expect("write to String");
        writeln!(
            output,
            "plan.{index}.description_json={}",
            canonical_text_json("plan description", &plan.state.description)?
        )
        .expect("write to String");
        writeln!(
            output,
            "plan.{index}.strategy_json={}",
            canonical_text_json("plan strategy", &plan.state.strategy)?
        )
        .expect("write to String");
        writeln!(
            output,
            "plan.{index}.constraints={}",
            plan.state.constraints.len()
        )
        .expect("write to String");
        writeln!(
            output,
            "plan.{index}.constraints_json={}",
            canonical_string_array_json("plan constraints", &plan.state.constraints)?
        )
        .expect("write to String");
        writeln!(
            output,
            "plan.{index}.completion_rationale_json={}",
            canonical_optional_text_json(
                "plan completion_rationale",
                plan.state.completion_rationale.as_deref()
            )?
        )
        .expect("write to String");
    }
    Ok(output)
}

fn render_plan_transition(plan: &PlanTransitionCommit) -> String {
    format!(
        "workspace_id={}\nbranch_id={}\nprevious_head_commit_id={}\ncommit_id={}\nchangeset_id={}\noperation_id={}\nplan_entity_id={}\nprevious_plan_entity_version_id={}\nplan_entity_version_id={}\nplan_state_digest={}\nwork_state_digest={}\nprevious_status={}\nstatus={}\nconstraints={}\ncompletion_rationale={}\n",
        plan.workspace_id,
        plan.branch_id,
        plan.previous_head_commit_id,
        plan.commit_id,
        plan.changeset_id,
        plan.operation_id,
        plan.plan_entity_id,
        plan.previous_plan_entity_version_id,
        plan.plan_entity_version_id,
        plan.plan_state_digest,
        plan.work_state_digest,
        plan.previous_state.status,
        plan.state.status,
        plan.state.constraints.len(),
        plan.state.completion_rationale.as_deref().unwrap_or("")
    )
}

fn render_task_create(task: &TaskCreateCommit) -> String {
    format!(
        "workspace_id={}\nbranch_id={}\nprevious_head_commit_id={}\ncommit_id={}\nchangeset_id={}\ntask_entity_id={}\ntask_entity_version_id={}\ntask_state_digest={}\nwork_state_digest={}\nstatus={}\n",
        task.workspace_id,
        task.branch_id,
        task.previous_head_commit_id,
        task.commit_id,
        task.changeset_id,
        task.task_entity_id,
        task.task_entity_version_id,
        task.task_state_digest,
        task.work_state_digest,
        task.state.status
    )
}

fn render_task_snapshot(task: &TaskSnapshot) -> Result<String> {
    let description_json = canonical_text_json("task description", &task.state.description)?;
    let outcome_json = canonical_optional_text_json("task outcome", task.state.outcome.as_deref())?;
    Ok(format!(
        "workspace_id={}\ncommit_id={}\ntask_entity_id={}\ntask_entity_version_id={}\ntask_state_digest={}\nstatus={}\ndescription_json={}\noutcome_json={}\npriority={}\nacceptance_criteria={}\n",
        task.workspace_id,
        task.commit_id,
        task.task_entity_id,
        task.task_entity_version_id,
        task.state_digest,
        task.state.status,
        description_json,
        outcome_json,
        task.state.priority,
        task.state.acceptance_criteria.len()
    ))
}

fn render_task_list(commit_id: CommitId, tasks: &[TaskSnapshot]) -> Result<String> {
    let mut output = format!("commit_id={commit_id}\ntasks={}\n", tasks.len());
    for (index, task) in tasks.iter().enumerate() {
        writeln!(output, "task.{index}.workspace_id={}", task.workspace_id)
            .expect("write to String");
        writeln!(
            output,
            "task.{index}.task_entity_id={}",
            task.task_entity_id
        )
        .expect("write to String");
        writeln!(
            output,
            "task.{index}.task_entity_version_id={}",
            task.task_entity_version_id
        )
        .expect("write to String");
        writeln!(
            output,
            "task.{index}.task_state_digest={}",
            task.state_digest
        )
        .expect("write to String");
        writeln!(output, "task.{index}.status={}", task.state.status).expect("write to String");
        writeln!(
            output,
            "task.{index}.description_json={}",
            canonical_text_json("task description", &task.state.description)?
        )
        .expect("write to String");
        writeln!(
            output,
            "task.{index}.outcome_json={}",
            canonical_optional_text_json("task outcome", task.state.outcome.as_deref())?
        )
        .expect("write to String");
        writeln!(output, "task.{index}.priority={}", task.state.priority).expect("write to String");
        writeln!(
            output,
            "task.{index}.acceptance_criteria={}",
            task.state.acceptance_criteria.len()
        )
        .expect("write to String");
    }
    Ok(output)
}

fn render_task_transition(transition: &TaskTransitionCommit) -> String {
    format!(
        "workspace_id={}\nbranch_id={}\nprevious_head_commit_id={}\ncommit_id={}\nchangeset_id={}\ntask_entity_id={}\nprevious_task_entity_version_id={}\ntask_entity_version_id={}\ntask_state_digest={}\nwork_state_digest={}\nstatus={}\n",
        transition.workspace_id,
        transition.branch_id,
        transition.previous_head_commit_id,
        transition.commit_id,
        transition.changeset_id,
        transition.task_entity_id,
        transition.previous_task_entity_version_id,
        transition.task_entity_version_id,
        transition.task_state_digest,
        transition.work_state_digest,
        transition.state.status
    )
}

fn render_task_scheduling_relation_create(relation: &TaskSchedulingRelationCreateCommit) -> String {
    format!(
        "workspace_id={}\nbranch_id={}\nprevious_head_commit_id={}\ncommit_id={}\nchangeset_id={}\noperation_id={}\nrelation_id={}\nrelation_version_id={}\nrelation_type={}\nsource_task_entity_id={}\ntarget_task_entity_id={}\nrelation_state_digest={}\nwork_state_digest={}\n",
        relation.workspace_id,
        relation.branch_id,
        relation.previous_head_commit_id,
        relation.commit_id,
        relation.changeset_id,
        relation.operation_id,
        relation.relation_id,
        relation.relation_version_id,
        relation.relation_type,
        relation.source_task_entity_id,
        relation.target_task_entity_id,
        relation.relation_state_digest,
        relation.work_state_digest
    )
}

fn render_task_scheduling_relation_list(
    commit_id: CommitId,
    relations: &[TaskSchedulingRelationSnapshot],
) -> String {
    let mut output = format!("commit_id={commit_id}\nrelations={}\n", relations.len());
    for (index, relation) in relations.iter().enumerate() {
        writeln!(
            output,
            "relation.{index}.workspace_id={}",
            relation.workspace_id
        )
        .expect("write to String");
        writeln!(output, "relation.{index}.commit_id={}", relation.commit_id)
            .expect("write to String");
        writeln!(
            output,
            "relation.{index}.relation_id={}",
            relation.relation_id
        )
        .expect("write to String");
        writeln!(
            output,
            "relation.{index}.relation_version_id={}",
            relation.relation_version_id
        )
        .expect("write to String");
        writeln!(
            output,
            "relation.{index}.relation_type={}",
            relation.relation_type
        )
        .expect("write to String");
        writeln!(
            output,
            "relation.{index}.source_task_entity_id={}",
            relation.source_task_entity_id
        )
        .expect("write to String");
        writeln!(
            output,
            "relation.{index}.target_task_entity_id={}",
            relation.target_task_entity_id
        )
        .expect("write to String");
        writeln!(
            output,
            "relation.{index}.relation_state_digest={}",
            relation.state_digest
        )
        .expect("write to String");
    }
    output
}

fn render_primary_containment_create(relation: &PrimaryContainmentCreateCommit) -> String {
    format!(
        "workspace_id={}\nbranch_id={}\nprevious_head_commit_id={}\ncommit_id={}\nchangeset_id={}\noperation_id={}\nrelation_id={}\nrelation_version_id={}\nparent_entity_id={}\nparent_kind={}\nchild_entity_id={}\nchild_kind={}\nrelation_state_digest={}\nwork_state_digest={}\n",
        relation.workspace_id,
        relation.branch_id,
        relation.previous_head_commit_id,
        relation.commit_id,
        relation.changeset_id,
        relation.operation_id,
        relation.relation_id,
        relation.relation_version_id,
        relation.parent_entity_id,
        relation.parent_kind,
        relation.child_entity_id,
        relation.child_kind,
        relation.relation_state_digest,
        relation.work_state_digest
    )
}

fn render_primary_containment_list(
    commit_id: CommitId,
    relations: &[PrimaryContainmentSnapshot],
) -> String {
    let mut output = format!("commit_id={commit_id}\nrelations={}\n", relations.len());
    for (index, relation) in relations.iter().enumerate() {
        writeln!(
            output,
            "relation.{index}.workspace_id={}",
            relation.workspace_id
        )
        .expect("write to String");
        writeln!(output, "relation.{index}.commit_id={}", relation.commit_id)
            .expect("write to String");
        writeln!(
            output,
            "relation.{index}.relation_id={}",
            relation.relation_id
        )
        .expect("write to String");
        writeln!(
            output,
            "relation.{index}.relation_version_id={}",
            relation.relation_version_id
        )
        .expect("write to String");
        writeln!(
            output,
            "relation.{index}.parent_entity_id={}",
            relation.parent_entity_id
        )
        .expect("write to String");
        writeln!(
            output,
            "relation.{index}.parent_kind={}",
            relation.parent_kind
        )
        .expect("write to String");
        writeln!(
            output,
            "relation.{index}.child_entity_id={}",
            relation.child_entity_id
        )
        .expect("write to String");
        writeln!(
            output,
            "relation.{index}.child_kind={}",
            relation.child_kind
        )
        .expect("write to String");
        writeln!(
            output,
            "relation.{index}.relation_state_digest={}",
            relation.state_digest
        )
        .expect("write to String");
    }
    output
}

fn render_acceptance_criterion_create(criterion: &AcceptanceCriterionCreateCommit) -> String {
    format!(
        "workspace_id={}\nbranch_id={}\nprevious_head_commit_id={}\ncommit_id={}\nchangeset_id={}\ntask_entity_id={}\ntask_entity_version_id={}\nacceptance_criterion_entity_id={}\nacceptance_criterion_entity_version_id={}\nacceptance_criterion_state_digest={}\nwork_state_digest={}\nlocal_key={}\nclassification={}\n",
        criterion.workspace_id,
        criterion.branch_id,
        criterion.previous_head_commit_id,
        criterion.commit_id,
        criterion.changeset_id,
        criterion.task_entity_id,
        criterion.task_entity_version_id,
        criterion.acceptance_criterion_entity_id,
        criterion.acceptance_criterion_entity_version_id,
        criterion.acceptance_criterion_state_digest,
        criterion.work_state_digest,
        criterion.local_key,
        criterion.state.classification
    )
}

fn render_acceptance_criterion_revision(
    criterion: &AcceptanceCriterionRevisionCommit,
) -> Result<String> {
    let previous_statement_json = canonical_text_json(
        "previous acceptance criterion statement",
        &criterion.previous_state.statement,
    )?;
    let statement_json =
        canonical_text_json("acceptance criterion statement", &criterion.state.statement)?;
    Ok(format!(
        "workspace_id={}\nbranch_id={}\nprevious_head_commit_id={}\ncommit_id={}\nchangeset_id={}\noperation_id={}\ntask_entity_id={}\nacceptance_criterion_entity_id={}\nprevious_acceptance_criterion_entity_version_id={}\nacceptance_criterion_entity_version_id={}\nacceptance_criterion_state_digest={}\nwork_state_digest={}\nlocal_key={}\nprevious_classification={}\nclassification={}\nprevious_statement_json={}\nstatement_json={}\nverification_requirements={}\n",
        criterion.workspace_id,
        criterion.branch_id,
        criterion.previous_head_commit_id,
        criterion.commit_id,
        criterion.changeset_id,
        criterion.operation_id,
        criterion.task_entity_id,
        criterion.acceptance_criterion_entity_id,
        criterion.previous_acceptance_criterion_entity_version_id,
        criterion.acceptance_criterion_entity_version_id,
        criterion.acceptance_criterion_state_digest,
        criterion.work_state_digest,
        criterion.local_key,
        criterion.previous_state.classification,
        criterion.state.classification,
        previous_statement_json,
        statement_json,
        criterion.state.verification_requirements.len()
    ))
}

fn render_acceptance_criterion_snapshot(criterion: &AcceptanceCriterionSnapshot) -> Result<String> {
    let statement_json =
        canonical_text_json("acceptance criterion statement", &criterion.state.statement)?;
    let mut output = format!(
        "workspace_id={}\ncommit_id={}\ntask_entity_id={}\nlocal_key={}\nacceptance_criterion_entity_id={}\nacceptance_criterion_entity_version_id={}\nacceptance_criterion_state_digest={}\nclassification={}\nstatement_json={}\nverification_requirements={}\n",
        criterion.workspace_id,
        criterion.commit_id,
        criterion.task_entity_id,
        criterion.local_key,
        criterion.acceptance_criterion_entity_id,
        criterion.acceptance_criterion_entity_version_id,
        criterion.state_digest,
        criterion.state.classification,
        statement_json,
        criterion.state.verification_requirements.len()
    );
    for (index, requirement) in criterion.state.verification_requirements.iter().enumerate() {
        writeln!(
            output,
            "verification_requirement.{index}.local_key={}",
            requirement.local_key
        )
        .expect("write to String");
        writeln!(
            output,
            "verification_requirement.{index}.verification_requirement_entity_id={}",
            requirement.verification_requirement_entity_id
        )
        .expect("write to String");
    }
    Ok(output)
}

fn render_acceptance_criterion_list(
    commit_id: CommitId,
    criteria: &[AcceptanceCriterionSnapshot],
) -> Result<String> {
    let mut output = format!("commit_id={commit_id}\ncriteria={}\n", criteria.len());
    for (index, criterion) in criteria.iter().enumerate() {
        writeln!(
            output,
            "criterion.{index}.workspace_id={}",
            criterion.workspace_id
        )
        .expect("write to String");
        writeln!(
            output,
            "criterion.{index}.task_entity_id={}",
            criterion.task_entity_id
        )
        .expect("write to String");
        writeln!(
            output,
            "criterion.{index}.local_key={}",
            criterion.local_key
        )
        .expect("write to String");
        writeln!(
            output,
            "criterion.{index}.acceptance_criterion_entity_id={}",
            criterion.acceptance_criterion_entity_id
        )
        .expect("write to String");
        writeln!(
            output,
            "criterion.{index}.acceptance_criterion_entity_version_id={}",
            criterion.acceptance_criterion_entity_version_id
        )
        .expect("write to String");
        writeln!(
            output,
            "criterion.{index}.acceptance_criterion_state_digest={}",
            criterion.state_digest
        )
        .expect("write to String");
        writeln!(
            output,
            "criterion.{index}.classification={}",
            criterion.state.classification
        )
        .expect("write to String");
        writeln!(
            output,
            "criterion.{index}.statement_json={}",
            canonical_text_json("acceptance criterion statement", &criterion.state.statement)?
        )
        .expect("write to String");
        writeln!(
            output,
            "criterion.{index}.verification_requirements={}",
            criterion.state.verification_requirements.len()
        )
        .expect("write to String");
    }
    Ok(output)
}

fn render_acceptance_criterion_status(status: AcceptanceCriterionEffectiveStatus) -> String {
    format!("status={status}\n")
}

fn render_verification_requirement_create(
    requirement: &VerificationRequirementCreateCommit,
) -> String {
    format!(
        "workspace_id={}\nbranch_id={}\nprevious_head_commit_id={}\ncommit_id={}\nchangeset_id={}\nacceptance_criterion_entity_id={}\nacceptance_criterion_entity_version_id={}\nverification_requirement_entity_id={}\nverification_requirement_entity_version_id={}\nverification_requirement_state_digest={}\nwork_state_digest={}\nlocal_key={}\n",
        requirement.workspace_id,
        requirement.branch_id,
        requirement.previous_head_commit_id,
        requirement.commit_id,
        requirement.changeset_id,
        requirement.acceptance_criterion_entity_id,
        requirement.acceptance_criterion_entity_version_id,
        requirement.verification_requirement_entity_id,
        requirement.verification_requirement_entity_version_id,
        requirement.verification_requirement_state_digest,
        requirement.work_state_digest,
        requirement.local_key
    )
}

fn render_verification_requirement_revision(
    requirement: &VerificationRequirementRevisionCommit,
) -> Result<String> {
    let previous_statement_json = canonical_text_json(
        "previous verification requirement statement",
        &requirement.previous_state.statement,
    )?;
    let statement_json = canonical_text_json(
        "verification requirement statement",
        &requirement.state.statement,
    )?;
    Ok(format!(
        "workspace_id={}\nbranch_id={}\nprevious_head_commit_id={}\ncommit_id={}\nchangeset_id={}\noperation_id={}\nacceptance_criterion_entity_id={}\nlocal_key={}\nverification_requirement_entity_id={}\nprevious_verification_requirement_entity_version_id={}\nverification_requirement_entity_version_id={}\nverification_requirement_state_digest={}\nwork_state_digest={}\nprevious_statement_json={}\nstatement_json={}\n",
        requirement.workspace_id,
        requirement.branch_id,
        requirement.previous_head_commit_id,
        requirement.commit_id,
        requirement.changeset_id,
        requirement.operation_id,
        requirement.acceptance_criterion_entity_id,
        requirement.local_key,
        requirement.verification_requirement_entity_id,
        requirement.previous_verification_requirement_entity_version_id,
        requirement.verification_requirement_entity_version_id,
        requirement.verification_requirement_state_digest,
        requirement.work_state_digest,
        previous_statement_json,
        statement_json
    ))
}

fn render_verification_requirement_snapshot(
    requirement: &VerificationRequirementSnapshot,
) -> Result<String> {
    let statement_json = canonical_text_json(
        "verification requirement statement",
        &requirement.state.statement,
    )?;
    Ok(format!(
        "workspace_id={}\ncommit_id={}\nacceptance_criterion_entity_id={}\nlocal_key={}\nverification_requirement_entity_id={}\nverification_requirement_entity_version_id={}\nverification_requirement_state_digest={}\nstatement_json={}\n",
        requirement.workspace_id,
        requirement.commit_id,
        requirement.acceptance_criterion_entity_id,
        requirement.local_key,
        requirement.verification_requirement_entity_id,
        requirement.verification_requirement_entity_version_id,
        requirement.state_digest,
        statement_json
    ))
}

fn render_verification_requirement_list(
    commit_id: CommitId,
    requirements: &[VerificationRequirementSnapshot],
) -> Result<String> {
    let mut output = format!(
        "commit_id={commit_id}\nrequirements={}\n",
        requirements.len()
    );
    for (index, requirement) in requirements.iter().enumerate() {
        writeln!(
            output,
            "requirement.{index}.workspace_id={}",
            requirement.workspace_id
        )
        .expect("write to String");
        writeln!(
            output,
            "requirement.{index}.acceptance_criterion_entity_id={}",
            requirement.acceptance_criterion_entity_id
        )
        .expect("write to String");
        writeln!(
            output,
            "requirement.{index}.local_key={}",
            requirement.local_key
        )
        .expect("write to String");
        writeln!(
            output,
            "requirement.{index}.verification_requirement_entity_id={}",
            requirement.verification_requirement_entity_id
        )
        .expect("write to String");
        writeln!(
            output,
            "requirement.{index}.verification_requirement_entity_version_id={}",
            requirement.verification_requirement_entity_version_id
        )
        .expect("write to String");
        writeln!(
            output,
            "requirement.{index}.verification_requirement_state_digest={}",
            requirement.state_digest
        )
        .expect("write to String");
        writeln!(
            output,
            "requirement.{index}.statement_json={}",
            canonical_text_json(
                "verification requirement statement",
                &requirement.state.statement
            )?
        )
        .expect("write to String");
    }
    Ok(output)
}

fn render_verification_create(verification: &VerificationCreateCommit) -> String {
    format!(
        "workspace_id={}\nbranch_id={}\nprevious_head_commit_id={}\ncommit_id={}\nchangeset_id={}\nverification_entity_id={}\nverification_entity_version_id={}\nverification_state_digest={}\nverifies_relation_id={}\nverifies_relation_version_id={}\nwork_state_digest={}\nresult={}\n",
        verification.workspace_id,
        verification.branch_id,
        verification.previous_head_commit_id,
        verification.commit_id,
        verification.changeset_id,
        verification.verification_entity_id,
        verification.verification_entity_version_id,
        verification.verification_state_digest,
        verification.verifies_relation_id,
        verification.verifies_relation_version_id,
        verification.work_state_digest,
        verification.state.result
    )
}

fn verification_target_kind(target: VerificationTarget) -> &'static str {
    match target {
        VerificationTarget::AcceptanceCriterion(_) => "acceptance_criterion",
        VerificationTarget::VerificationRequirement(_) => "verification_requirement",
    }
}

fn render_verification_snapshot(verification: &VerificationSnapshot) -> Result<String> {
    let method_json = canonical_cli_json("verification method", &verification.state.method)?;
    let mut output = format!(
        "workspace_id={}\ncommit_id={}\nverification_entity_id={}\nverification_entity_version_id={}\nverification_state_digest={}\nverifies_relation_id={}\nverifies_relation_version_id={}\nverifies_relation_state_digest={}\ntarget_kind={}\ntarget_entity_id={}\nresult={}\nverified_at_commit_id={}\nmethod_json={}\nsemantic_dependencies={}\nevidence={}\nevidenced_by_relations={}\nresource_basis={}\n",
        verification.workspace_id,
        verification.commit_id,
        verification.verification_entity_id,
        verification.verification_entity_version_id,
        verification.state_digest,
        verification.verifies_relation_id,
        verification.verifies_relation_version_id,
        verification.verifies_relation_state_digest,
        verification_target_kind(verification.target),
        verification.target.entity_id(),
        verification.state.result,
        verification.state.verified_at_commit_id,
        method_json,
        verification.state.semantic_dependencies.len(),
        verification.state.evidence.len(),
        verification.evidenced_by_relations.len(),
        verification.state.resource_basis.len()
    );
    for (index, dependency) in verification.state.semantic_dependencies.iter().enumerate() {
        writeln!(
            output,
            "semantic_dependency.{index}.entity_id={}",
            dependency.entity_id
        )
        .expect("write to String");
        writeln!(
            output,
            "semantic_dependency.{index}.entity_version_id={}",
            dependency.entity_version_id
        )
        .expect("write to String");
    }
    for (index, evidence) in verification.state.evidence.iter().enumerate() {
        writeln!(
            output,
            "evidence.{index}.evidence_id={}",
            evidence.evidence_id
        )
        .expect("write to String");
    }
    for (index, relation) in verification.evidenced_by_relations.iter().enumerate() {
        writeln!(
            output,
            "evidenced_by_relation.{index}.evidence_id={}",
            relation.evidence_id
        )
        .expect("write to String");
        writeln!(
            output,
            "evidenced_by_relation.{index}.relation_id={}",
            relation.relation_id
        )
        .expect("write to String");
        writeln!(
            output,
            "evidenced_by_relation.{index}.relation_version_id={}",
            relation.relation_version_id
        )
        .expect("write to String");
        writeln!(
            output,
            "evidenced_by_relation.{index}.relation_state_digest={}",
            relation.state_digest
        )
        .expect("write to String");
    }
    for (index, basis) in verification.state.resource_basis.iter().enumerate() {
        writeln!(
            output,
            "resource_basis.{index}.resource_id={}",
            basis.resource_id
        )
        .expect("write to String");
        writeln!(
            output,
            "resource_basis.{index}.adapter_kind={}",
            basis.adapter_kind
        )
        .expect("write to String");
        writeln!(
            output,
            "resource_basis.{index}.adapter_schema_version={}",
            basis.adapter_schema_version
        )
        .expect("write to String");
        writeln!(
            output,
            "resource_basis.{index}.scope_kind={}",
            basis.scope_kind
        )
        .expect("write to String");
        writeln!(
            output,
            "resource_basis.{index}.scope_schema_version={}",
            basis.scope_schema_version
        )
        .expect("write to String");
        writeln!(
            output,
            "resource_basis.{index}.scope_payload_json={}",
            canonical_cli_json(
                "verification resource basis scope payload",
                &basis.scope_payload
            )?
        )
        .expect("write to String");
        writeln!(
            output,
            "resource_basis.{index}.baseline_observation_id={}",
            render_optional_display_or_none(basis.baseline_observation_id.as_ref())
        )
        .expect("write to String");
        writeln!(
            output,
            "resource_basis.{index}.baseline_fingerprint={}",
            basis.baseline_fingerprint
        )
        .expect("write to String");
    }
    Ok(output)
}

fn render_verification_list(
    commit_id: CommitId,
    verifications: &[VerificationSnapshot],
) -> Result<String> {
    let mut output = format!(
        "commit_id={commit_id}\nverifications={}\n",
        verifications.len()
    );
    for (index, verification) in verifications.iter().enumerate() {
        writeln!(
            output,
            "verification.{index}.workspace_id={}",
            verification.workspace_id
        )
        .expect("write to String");
        writeln!(
            output,
            "verification.{index}.verification_entity_id={}",
            verification.verification_entity_id
        )
        .expect("write to String");
        writeln!(
            output,
            "verification.{index}.verification_entity_version_id={}",
            verification.verification_entity_version_id
        )
        .expect("write to String");
        writeln!(
            output,
            "verification.{index}.verification_state_digest={}",
            verification.state_digest
        )
        .expect("write to String");
        writeln!(
            output,
            "verification.{index}.target_kind={}",
            verification_target_kind(verification.target)
        )
        .expect("write to String");
        writeln!(
            output,
            "verification.{index}.target_entity_id={}",
            verification.target.entity_id()
        )
        .expect("write to String");
        writeln!(
            output,
            "verification.{index}.result={}",
            verification.state.result
        )
        .expect("write to String");
        writeln!(
            output,
            "verification.{index}.verified_at_commit_id={}",
            verification.state.verified_at_commit_id
        )
        .expect("write to String");
        writeln!(
            output,
            "verification.{index}.method_json={}",
            canonical_cli_json("verification method", &verification.state.method)?
        )
        .expect("write to String");
        writeln!(
            output,
            "verification.{index}.semantic_dependencies={}",
            verification.state.semantic_dependencies.len()
        )
        .expect("write to String");
        writeln!(
            output,
            "verification.{index}.evidence={}",
            verification.state.evidence.len()
        )
        .expect("write to String");
        writeln!(
            output,
            "verification.{index}.resource_basis={}",
            verification.state.resource_basis.len()
        )
        .expect("write to String");
    }
    Ok(output)
}

fn render_evidence_create(evidence: &EvidenceCreateResult) -> Result<String> {
    let mut output = format!(
        "evidence_id={}\nevidence_kind={}\ncaptured_at_us={}\nsource_session_id={}\nmetadata_json={}\ncontents={}\n",
        evidence.evidence_id,
        evidence.evidence_kind,
        evidence.captured_at_us,
        render_optional_display_or_none(evidence.source_session_id.as_ref()),
        canonical_cli_json("evidence metadata", &evidence.metadata)?,
        evidence.contents.len()
    );
    write_evidence_content_fields(&mut output, &evidence.contents)?;
    Ok(output)
}

fn render_evidence_snapshot(evidence: &EvidenceSnapshot) -> Result<String> {
    let mut output = format!(
        "evidence_id={}\nevidence_kind={}\ncaptured_at_us={}\nsource_session_id={}\nmetadata_json={}\ncontents={}\n",
        evidence.evidence_id,
        evidence.evidence_kind,
        evidence.captured_at_us,
        render_optional_display_or_none(evidence.source_session_id.as_ref()),
        canonical_cli_json("evidence metadata", &evidence.metadata)?,
        evidence.contents.len()
    );
    write_evidence_content_fields(&mut output, &evidence.contents)?;
    Ok(output)
}

fn render_evidence_list(result: &EvidenceListResult) -> Result<String> {
    let mut output = format!("evidences={}\n", result.evidences.len());
    for (index, evidence) in result.evidences.iter().enumerate() {
        writeln!(
            output,
            "evidence.{index}.evidence_id={}",
            evidence.evidence_id
        )
        .expect("write to String");
        writeln!(
            output,
            "evidence.{index}.evidence_kind={}",
            evidence.evidence_kind
        )
        .expect("write to String");
        writeln!(
            output,
            "evidence.{index}.captured_at_us={}",
            evidence.captured_at_us
        )
        .expect("write to String");
        writeln!(
            output,
            "evidence.{index}.source_session_id={}",
            render_optional_display_or_none(evidence.source_session_id.as_ref())
        )
        .expect("write to String");
        writeln!(
            output,
            "evidence.{index}.metadata_json={}",
            canonical_cli_json("evidence metadata", &evidence.metadata)?
        )
        .expect("write to String");
        writeln!(
            output,
            "evidence.{index}.contents={}",
            evidence.contents.len()
        )
        .expect("write to String");
    }
    Ok(output)
}

fn write_evidence_content_fields(
    output: &mut String,
    contents: &[EvidenceContentSnapshot],
) -> Result<()> {
    for content in contents {
        let index = content.ordinal;
        writeln!(output, "content.{index}.ordinal={}", content.ordinal).expect("write to String");
        writeln!(output, "content.{index}.role={}", content.role).expect("write to String");
        writeln!(
            output,
            "content.{index}.content_digest={}",
            content.content_digest
        )
        .expect("write to String");
        writeln!(output, "content.{index}.size_bytes={}", content.size_bytes)
            .expect("write to String");
        writeln!(
            output,
            "content.{index}.media_type={}",
            render_optional_display_or_none(content.media_type.as_ref())
        )
        .expect("write to String");
        writeln!(
            output,
            "content.{index}.format_metadata_json={}",
            canonical_cli_json("evidence content format metadata", &content.format_metadata)?
        )
        .expect("write to String");
    }
    Ok(())
}

fn render_resource_create(resource: &ResourceCreateResult) -> String {
    format!(
        "resource_id={}\nresource_kind={}\ncreated_at_us={}\n",
        resource.resource_id, resource.resource_kind, resource.created_at_us
    )
}

fn render_resource_snapshot(resource: &ResourceSnapshot) -> Result<String> {
    let mut output = format!(
        "resource_id={}\nresource_kind={}\ncreated_at_us={}\nbinding_present={}\nworkspace_associations={}\n",
        resource.resource_id,
        resource.resource_kind,
        resource.created_at_us,
        resource.binding.is_some(),
        resource.workspace_associations.len()
    );
    if let Some(binding) = &resource.binding {
        writeln!(output, "binding.resource_id={}", binding.resource_id).expect("write to String");
        writeln!(output, "binding.adapter_kind={}", binding.adapter_kind).expect("write to String");
        writeln!(output, "binding.locator={}", binding.locator).expect("write to String");
        writeln!(
            output,
            "binding.binding_config_json={}",
            canonical_cli_json("resource binding config", &binding.binding_config)?
        )
        .expect("write to String");
        writeln!(output, "binding.bound_at_us={}", binding.bound_at_us).expect("write to String");
    }
    for (index, association) in resource.workspace_associations.iter().enumerate() {
        writeln!(
            output,
            "workspace_association.{index}.workspace_id={}",
            association.workspace_id
        )
        .expect("write to String");
        writeln!(
            output,
            "workspace_association.{index}.resource_id={}",
            association.resource_id
        )
        .expect("write to String");
        writeln!(
            output,
            "workspace_association.{index}.metadata_json={}",
            canonical_cli_json(
                "workspace resource association metadata",
                &association.association_metadata
            )?
        )
        .expect("write to String");
    }
    Ok(output)
}

fn render_resource_list(result: &ResourceListResult) -> String {
    let mut output = format!("resources={}\n", result.resources.len());
    for (index, resource) in result.resources.iter().enumerate() {
        writeln!(
            output,
            "resource.{index}.resource_id={}",
            resource.resource_id
        )
        .expect("write to String");
        writeln!(
            output,
            "resource.{index}.resource_kind={}",
            resource.resource_kind
        )
        .expect("write to String");
        writeln!(
            output,
            "resource.{index}.created_at_us={}",
            resource.created_at_us
        )
        .expect("write to String");
        writeln!(
            output,
            "resource.{index}.binding_present={}",
            resource.binding.is_some()
        )
        .expect("write to String");
        writeln!(
            output,
            "resource.{index}.workspace_associations={}",
            resource.workspace_associations.len()
        )
        .expect("write to String");
    }
    output
}

fn render_resource_bind(result: &ResourceBindResult) -> Result<String> {
    Ok(format!(
        "resource_id={}\nbound_at_us={}\nadapter_kind={}\nlocator={}\nbinding_config_json={}\n",
        result.resource_id,
        result.bound_at_us,
        result.state.adapter_kind,
        result.state.locator,
        canonical_cli_json("resource binding config", &result.state.binding_config)?
    ))
}

fn render_workspace_resource_association(
    result: &WorkspaceResourceAssociationResult,
) -> Result<String> {
    Ok(format!(
        "workspace_id={}\nresource_id={}\nmetadata_json={}\n",
        result.workspace_id,
        result.resource_id,
        canonical_cli_json(
            "workspace resource association metadata",
            &result.state.association_metadata
        )?
    ))
}

fn render_workspace_resource_association_list(
    result: &WorkspaceResourceAssociationListResult,
) -> Result<String> {
    let mut output = format!(
        "workspace_id={}\nworkspace_associations={}\n",
        result.workspace_id,
        result.associations.len()
    );
    for (index, association) in result.associations.iter().enumerate() {
        writeln!(
            output,
            "workspace_association.{index}.workspace_id={}",
            association.workspace_id
        )
        .expect("write to String");
        writeln!(
            output,
            "workspace_association.{index}.resource_id={}",
            association.resource_id
        )
        .expect("write to String");
        writeln!(
            output,
            "workspace_association.{index}.metadata_json={}",
            canonical_cli_json(
                "workspace resource association metadata",
                &association.association_metadata
            )?
        )
        .expect("write to String");
    }
    Ok(output)
}

fn render_resource_observation_create(observation: &ResourceObservationCreateResult) -> String {
    format!(
        "observation_id={}\nresource_id={}\nadapter_kind={}\nadapter_schema_version={}\nfingerprint={}\ncaptured_at_us={}\n",
        observation.observation_id,
        observation.resource_id,
        observation.state.adapter_kind,
        observation.state.adapter_schema_version,
        observation.state.fingerprint,
        observation.captured_at_us
    )
}

fn render_resource_observation_snapshot(
    observation: &ResourceObservationSnapshot,
) -> Result<String> {
    let mut output = format!(
        "observation_id={}\nresource_id={}\nadapter_kind={}\nadapter_schema_version={}\nfingerprint={}\ncaptured_at_us={}\nsummary_json={}\ndetail_content_present={}\nsource_session_id={}\n",
        observation.observation_id,
        observation.resource_id,
        observation.adapter_kind,
        observation.adapter_schema_version,
        observation.fingerprint,
        observation.captured_at_us,
        canonical_cli_json("resource observation summary", &observation.summary)?,
        observation.detail_content.is_some(),
        render_optional_display_or_none(observation.source_session_id.as_ref())
    );
    if let Some(detail) = &observation.detail_content {
        writeln!(output, "detail.content_digest={}", detail.content_digest)
            .expect("write to String");
        writeln!(output, "detail.size_bytes={}", detail.size_bytes).expect("write to String");
        writeln!(
            output,
            "detail.media_type={}",
            render_optional_display_or_none(detail.media_type.as_ref())
        )
        .expect("write to String");
        writeln!(
            output,
            "detail.format_metadata_json={}",
            canonical_cli_json(
                "resource observation detail format metadata",
                &detail.format_metadata
            )?
        )
        .expect("write to String");
    }
    Ok(output)
}

fn render_resource_observation_list(result: &ResourceObservationListResult) -> Result<String> {
    let mut output = format!("observations={}\n", result.observations.len());
    for (index, observation) in result.observations.iter().enumerate() {
        writeln!(
            output,
            "observation.{index}.observation_id={}",
            observation.observation_id
        )
        .expect("write to String");
        writeln!(
            output,
            "observation.{index}.resource_id={}",
            observation.resource_id
        )
        .expect("write to String");
        writeln!(
            output,
            "observation.{index}.adapter_kind={}",
            observation.adapter_kind
        )
        .expect("write to String");
        writeln!(
            output,
            "observation.{index}.adapter_schema_version={}",
            observation.adapter_schema_version
        )
        .expect("write to String");
        writeln!(
            output,
            "observation.{index}.captured_at_us={}",
            observation.captured_at_us
        )
        .expect("write to String");
        writeln!(
            output,
            "observation.{index}.fingerprint={}",
            observation.fingerprint
        )
        .expect("write to String");
        writeln!(
            output,
            "observation.{index}.summary_json={}",
            canonical_cli_json("resource observation summary", &observation.summary)?
        )
        .expect("write to String");
        writeln!(
            output,
            "observation.{index}.detail_content_present={}",
            observation.detail_content.is_some()
        )
        .expect("write to String");
        writeln!(
            output,
            "observation.{index}.source_session_id={}",
            render_optional_display_or_none(observation.source_session_id.as_ref())
        )
        .expect("write to String");
    }
    Ok(output)
}

fn render_verification_applicability_cache(
    snapshot: &VerificationApplicabilityCacheSnapshot,
) -> String {
    format!(
        "branch_id={}\nverification_entity_id={}\nevaluated_commit_id={}\napplicability={}\nreason_code={}\nevaluated_at_us={}\nresource_stamps={}\n",
        snapshot.branch_id,
        snapshot.verification_entity_id,
        snapshot.evaluated_commit_id,
        snapshot.applicability,
        snapshot.reason_code,
        snapshot.evaluated_at_us,
        snapshot.resource_stamps.len()
    )
}

fn render_verification_applicability_cache_lookup(
    branch_id: BranchId,
    verification_entity_id: EntityId,
    snapshot: Option<&VerificationApplicabilityCacheSnapshot>,
) -> Result<String> {
    let Some(snapshot) = snapshot else {
        return Ok(format!(
            "branch_id={branch_id}\nverification_entity_id={verification_entity_id}\ncache_found=false\n"
        ));
    };
    let detail_json =
        canonical_cli_json("verification applicability cache detail", &snapshot.detail)?;
    let mut output = format!(
        "branch_id={}\nverification_entity_id={}\ncache_found=true\nevaluated_commit_id={}\napplicability={}\nreason_code={}\nevaluated_at_us={}\ndetail_json={}\nresource_stamps={}\n",
        snapshot.branch_id,
        snapshot.verification_entity_id,
        snapshot.evaluated_commit_id,
        snapshot.applicability,
        snapshot.reason_code,
        snapshot.evaluated_at_us,
        detail_json,
        snapshot.resource_stamps.len()
    );
    for (index, stamp) in snapshot.resource_stamps.iter().enumerate() {
        writeln!(
            output,
            "resource_stamp.{index}.resource_basis_ordinal={}",
            stamp.resource_basis_ordinal
        )
        .expect("write to String");
        writeln!(
            output,
            "resource_stamp.{index}.adapter_kind={}",
            stamp.adapter_kind
        )
        .expect("write to String");
        writeln!(
            output,
            "resource_stamp.{index}.adapter_schema_version={}",
            stamp.adapter_schema_version
        )
        .expect("write to String");
        writeln!(
            output,
            "resource_stamp.{index}.scope_schema_version={}",
            stamp.scope_schema_version
        )
        .expect("write to String");
        writeln!(
            output,
            "resource_stamp.{index}.observation_status={}",
            stamp.observation_status
        )
        .expect("write to String");
        writeln!(
            output,
            "resource_stamp.{index}.observed_fingerprint={}",
            render_optional_display_or_none(stamp.observed_fingerprint.as_ref())
        )
        .expect("write to String");
        writeln!(
            output,
            "resource_stamp.{index}.observation_id={}",
            render_optional_display_or_none(stamp.observation_id.as_ref())
        )
        .expect("write to String");
        writeln!(
            output,
            "resource_stamp.{index}.observed_at_us={}",
            stamp.observed_at_us
        )
        .expect("write to String");
    }
    Ok(output)
}

fn render_verification_applicability_cache_list(
    result: &VerificationApplicabilityCacheListResult,
) -> String {
    let mut output = format!(
        "branch_id={}\ncaches={}\n",
        result.branch_id,
        result.caches.len()
    );
    for (index, cache) in result.caches.iter().enumerate() {
        writeln!(
            output,
            "cache.{index}.verification_entity_id={}",
            cache.verification_entity_id
        )
        .expect("write to String");
        writeln!(
            output,
            "cache.{index}.evaluated_commit_id={}",
            cache.evaluated_commit_id
        )
        .expect("write to String");
        writeln!(
            output,
            "cache.{index}.applicability={}",
            cache.applicability
        )
        .expect("write to String");
        writeln!(output, "cache.{index}.reason_code={}", cache.reason_code)
            .expect("write to String");
        writeln!(
            output,
            "cache.{index}.evaluated_at_us={}",
            cache.evaluated_at_us
        )
        .expect("write to String");
        writeln!(
            output,
            "cache.{index}.resource_stamps={}",
            cache.resource_stamps.len()
        )
        .expect("write to String");
    }
    output
}

fn render_record_create(record: &RecordCreateCommit) -> String {
    format!(
        "workspace_id={}\nbranch_id={}\nprevious_head_commit_id={}\ncommit_id={}\nchangeset_id={}\noperation_id={}\nrecord_entity_id={}\nrecord_entity_version_id={}\nrecord_state_digest={}\nwork_state_digest={}\nrecord_kind={}\nrecord_status={}\n",
        record.workspace_id,
        record.branch_id,
        record.previous_head_commit_id,
        record.commit_id,
        record.changeset_id,
        record.operation_id,
        record.record_entity_id,
        record.record_entity_version_id,
        record.record_state_digest,
        record.work_state_digest,
        record.state.kind,
        record.state.status
    )
}

fn render_record_transition(record: &RecordTransitionCommit) -> String {
    format!(
        "workspace_id={}\nbranch_id={}\nprevious_head_commit_id={}\ncommit_id={}\nchangeset_id={}\noperation_id={}\nrecord_entity_id={}\nprevious_record_entity_version_id={}\nrecord_entity_version_id={}\nrecord_state_digest={}\nwork_state_digest={}\nrecord_kind={}\nprevious_record_status={}\nrecord_status={}\n",
        record.workspace_id,
        record.branch_id,
        record.previous_head_commit_id,
        record.commit_id,
        record.changeset_id,
        record.operation_id,
        record.record_entity_id,
        record.previous_record_entity_version_id,
        record.record_entity_version_id,
        record.record_state_digest,
        record.work_state_digest,
        record.state.kind,
        record.previous_state.status,
        record.state.status
    )
}

fn render_record_show(record: &RecordSnapshot) -> Result<String> {
    let statement_json = serde_json::to_string(&record.state.statement).map_err(|error| {
        WorkVcsError::RecordInvalid(format!("record statement encode failed: {error}"))
    })?;
    let scope_json = String::from_utf8(canonical_bytes(&record.state.scope)?).map_err(|error| {
        WorkVcsError::RecordInvalid(format!("record scope encode produced non-UTF-8: {error}"))
    })?;
    Ok(format!(
        "workspace_id={}\ncommit_id={}\nrecord_entity_id={}\nrecord_entity_version_id={}\nrecord_state_digest={}\nrecord_kind={}\nrecord_status={}\nrecord_statement_json={}\nrecord_scope_json={}\n",
        record.workspace_id,
        record.commit_id,
        record.record_entity_id,
        record.record_entity_version_id,
        record.state_digest,
        record.state.kind,
        record.state.status,
        statement_json,
        scope_json
    ))
}

fn render_record_list(result: &RecordListResult) -> String {
    let mut output = format!(
        "workspace_id={}\ncommit_id={}\nrecords={}\n",
        result.workspace_id,
        result.commit_id,
        result.records.len()
    );
    for (index, record) in result.records.iter().enumerate() {
        writeln!(
            output,
            "record.{index}.record_entity_id={}",
            record.record_entity_id
        )
        .expect("write to String");
        writeln!(
            output,
            "record.{index}.record_entity_version_id={}",
            record.record_entity_version_id
        )
        .expect("write to String");
        writeln!(
            output,
            "record.{index}.record_state_digest={}",
            record.state_digest
        )
        .expect("write to String");
        writeln!(output, "record.{index}.record_kind={}", record.state.kind)
            .expect("write to String");
        writeln!(
            output,
            "record.{index}.record_status={}",
            record.state.status
        )
        .expect("write to String");
    }
    output
}

fn render_record_relation_create(relation: &RecordRelationCreateCommit) -> String {
    format!(
        "workspace_id={}\nbranch_id={}\nprevious_head_commit_id={}\ncommit_id={}\nchangeset_id={}\noperation_id={}\nrelation_id={}\nrelation_version_id={}\nrelation_type={}\nrelation_label={}\nsource_record_entity_id={}\ntarget_record_entity_id={}\nrelation_state_digest={}\nwork_state_digest={}\n",
        relation.workspace_id,
        relation.branch_id,
        relation.previous_head_commit_id,
        relation.commit_id,
        relation.changeset_id,
        relation.operation_id,
        relation.relation_id,
        relation.relation_version_id,
        relation.relation_type,
        relation.relation_label.as_deref().unwrap_or(""),
        relation.source_record_entity_id,
        relation.target_record_entity_id,
        relation.relation_state_digest,
        relation.work_state_digest
    )
}

fn render_record_knowledge_relation_create(
    relation: &RecordKnowledgeRelationCreateCommit,
) -> String {
    format!(
        "workspace_id={}\nbranch_id={}\nprevious_head_commit_id={}\ncommit_id={}\nchangeset_id={}\noperation_id={}\nrelation_id={}\nrelation_version_id={}\nrelation_type={}\nsource_record_entity_id={}\ntarget_knowledge_entity_id={}\nrelation_state_digest={}\nwork_state_digest={}\n",
        relation.workspace_id,
        relation.branch_id,
        relation.previous_head_commit_id,
        relation.commit_id,
        relation.changeset_id,
        relation.operation_id,
        relation.relation_id,
        relation.relation_version_id,
        relation.relation_type,
        relation.source_record_entity_id,
        relation.target_knowledge_entity_id,
        relation.relation_state_digest,
        relation.work_state_digest
    )
}

fn render_record_relation_remove(relation: &RecordRelationRemoveCommit) -> String {
    format!(
        "workspace_id={}\nbranch_id={}\nprevious_head_commit_id={}\ncommit_id={}\nchangeset_id={}\noperation_id={}\nrelation_id={}\nprevious_relation_version_id={}\nrelation_type={}\nrelation_label={}\nsource_record_entity_id={}\ntarget_record_entity_id={}\nwork_state_digest={}\n",
        relation.workspace_id,
        relation.branch_id,
        relation.previous_head_commit_id,
        relation.commit_id,
        relation.changeset_id,
        relation.operation_id,
        relation.relation_id,
        relation.previous_relation_version_id,
        relation.relation_type,
        relation.relation_label.as_deref().unwrap_or(""),
        relation.source_record_entity_id,
        relation.target_record_entity_id,
        relation.work_state_digest
    )
}

fn render_record_knowledge_relation_remove(
    relation: &RecordKnowledgeRelationRemoveCommit,
) -> String {
    format!(
        "workspace_id={}\nbranch_id={}\nprevious_head_commit_id={}\ncommit_id={}\nchangeset_id={}\noperation_id={}\nrelation_id={}\nprevious_relation_version_id={}\nrelation_type={}\nsource_record_entity_id={}\ntarget_knowledge_entity_id={}\nwork_state_digest={}\n",
        relation.workspace_id,
        relation.branch_id,
        relation.previous_head_commit_id,
        relation.commit_id,
        relation.changeset_id,
        relation.operation_id,
        relation.relation_id,
        relation.previous_relation_version_id,
        relation.relation_type,
        relation.source_record_entity_id,
        relation.target_knowledge_entity_id,
        relation.work_state_digest
    )
}

fn render_record_knowledge_relation_restore(
    relation: &RecordKnowledgeRelationRestoreCommit,
) -> String {
    format!(
        "workspace_id={}\nbranch_id={}\nprevious_head_commit_id={}\ncommit_id={}\nchangeset_id={}\noperation_id={}\nrelation_id={}\nrelation_version_id={}\nrelation_type={}\nsource_record_entity_id={}\ntarget_knowledge_entity_id={}\nrelation_state_digest={}\nwork_state_digest={}\n",
        relation.workspace_id,
        relation.branch_id,
        relation.previous_head_commit_id,
        relation.commit_id,
        relation.changeset_id,
        relation.operation_id,
        relation.relation_id,
        relation.relation_version_id,
        relation.relation_type,
        relation.source_record_entity_id,
        relation.target_knowledge_entity_id,
        relation.relation_state_digest,
        relation.work_state_digest
    )
}

fn render_record_relation_restore(relation: &RecordRelationRestoreCommit) -> String {
    format!(
        "workspace_id={}\nbranch_id={}\nprevious_head_commit_id={}\ncommit_id={}\nchangeset_id={}\noperation_id={}\nrelation_id={}\nrelation_version_id={}\nrelation_type={}\nrelation_label={}\nsource_record_entity_id={}\ntarget_record_entity_id={}\nrelation_state_digest={}\nwork_state_digest={}\n",
        relation.workspace_id,
        relation.branch_id,
        relation.previous_head_commit_id,
        relation.commit_id,
        relation.changeset_id,
        relation.operation_id,
        relation.relation_id,
        relation.relation_version_id,
        relation.relation_type,
        relation.relation_label.as_deref().unwrap_or(""),
        relation.source_record_entity_id,
        relation.target_record_entity_id,
        relation.relation_state_digest,
        relation.work_state_digest
    )
}

fn render_decision_record_supersede(superseded: &DecisionRecordSupersedeCommit) -> String {
    format!(
        "workspace_id={}\nbranch_id={}\nprevious_head_commit_id={}\ncommit_id={}\nchangeset_id={}\nprior_record_operation_id={}\nrelation_operation_id={}\nreplacement_record_entity_id={}\nprior_record_entity_id={}\nprevious_prior_record_entity_version_id={}\nprior_record_entity_version_id={}\nprior_record_state_digest={}\nrelation_id={}\nrelation_version_id={}\nrelation_type={}\nrelation_state_digest={}\ncausal_record_entity_id={}\ncausal_relation_operation_id={}\ncausal_relation_id={}\ncausal_relation_version_id={}\ncausal_relation_type={}\ncausal_relation_state_digest={}\nwork_state_digest={}\nprevious_prior_record_status={}\nprior_record_status={}\n",
        superseded.workspace_id,
        superseded.branch_id,
        superseded.previous_head_commit_id,
        superseded.commit_id,
        superseded.changeset_id,
        superseded.prior_record_operation_id,
        superseded.relation_operation_id,
        superseded.replacement_record_entity_id,
        superseded.prior_record_entity_id,
        superseded.previous_prior_record_entity_version_id,
        superseded.prior_record_entity_version_id,
        superseded.prior_record_state_digest,
        superseded.relation_id,
        superseded.relation_version_id,
        RecordRelationType::Supersedes,
        superseded.relation_state_digest,
        render_optional_display(superseded.causal_record_entity_id.as_ref()),
        render_optional_display(superseded.causal_relation_operation_id.as_ref()),
        render_optional_display(superseded.causal_relation_id.as_ref()),
        render_optional_display(superseded.causal_relation_version_id.as_ref()),
        superseded
            .causal_relation_id
            .as_ref()
            .map(|_| RecordRelationType::DerivedFrom.as_str())
            .unwrap_or(""),
        render_optional_display(superseded.causal_relation_state_digest.as_ref()),
        superseded.work_state_digest,
        superseded.previous_prior_state.status,
        superseded.prior_state.status
    )
}

fn render_optional_display<T: std::fmt::Display>(value: Option<&T>) -> String {
    value
        .map(std::string::ToString::to_string)
        .unwrap_or_default()
}

fn render_record_relation_list(result: &RecordRelationListResult) -> String {
    let mut output = format!(
        "workspace_id={}\ncommit_id={}\nrelations={}\n",
        result.workspace_id,
        result.commit_id,
        result.relations.len()
    );
    for (index, relation) in result.relations.iter().enumerate() {
        writeln!(
            output,
            "relation.{index}.relation_id={}",
            relation.relation_id
        )
        .expect("write to String");
        writeln!(
            output,
            "relation.{index}.relation_version_id={}",
            relation.relation_version_id
        )
        .expect("write to String");
        writeln!(
            output,
            "relation.{index}.relation_type={}",
            relation.relation_type
        )
        .expect("write to String");
        writeln!(
            output,
            "relation.{index}.relation_label={}",
            relation.relation_label.as_deref().unwrap_or("")
        )
        .expect("write to String");
        writeln!(
            output,
            "relation.{index}.source_record_entity_id={}",
            relation.source_record_entity_id
        )
        .expect("write to String");
        writeln!(
            output,
            "relation.{index}.target_record_entity_id={}",
            relation.target_record_entity_id
        )
        .expect("write to String");
        writeln!(
            output,
            "relation.{index}.relation_state_digest={}",
            relation.state_digest
        )
        .expect("write to String");
    }
    output
}

fn render_record_knowledge_relation_list(result: &RecordKnowledgeRelationListResult) -> String {
    let mut output = format!(
        "workspace_id={}\ncommit_id={}\nrelations={}\n",
        result.workspace_id,
        result.commit_id,
        result.relations.len()
    );
    for (index, relation) in result.relations.iter().enumerate() {
        writeln!(
            output,
            "relation.{index}.relation_id={}",
            relation.relation_id
        )
        .expect("write to String");
        writeln!(
            output,
            "relation.{index}.relation_version_id={}",
            relation.relation_version_id
        )
        .expect("write to String");
        writeln!(
            output,
            "relation.{index}.relation_type={}",
            relation.relation_type
        )
        .expect("write to String");
        writeln!(
            output,
            "relation.{index}.source_record_entity_id={}",
            relation.source_record_entity_id
        )
        .expect("write to String");
        writeln!(
            output,
            "relation.{index}.target_knowledge_entity_id={}",
            relation.target_knowledge_entity_id
        )
        .expect("write to String");
        writeln!(
            output,
            "relation.{index}.relation_state_digest={}",
            relation.state_digest
        )
        .expect("write to String");
    }
    output
}

fn render_record_knowledge_relation_snapshot(relation: &RecordKnowledgeRelationSnapshot) -> String {
    format!(
        "workspace_id={}\ncommit_id={}\nrelation_id={}\nrelation_version_id={}\nrelation_type={}\nsource_record_entity_id={}\ntarget_knowledge_entity_id={}\nrelation_state_digest={}\n",
        relation.workspace_id,
        relation.commit_id,
        relation.relation_id,
        relation.relation_version_id,
        relation.relation_type,
        relation.source_record_entity_id,
        relation.target_knowledge_entity_id,
        relation.state_digest
    )
}

fn render_record_relation_snapshot(relation: &RecordRelationSnapshot) -> String {
    format!(
        "workspace_id={}\ncommit_id={}\nrelation_id={}\nrelation_version_id={}\nrelation_type={}\nrelation_label={}\nsource_record_entity_id={}\ntarget_record_entity_id={}\nrelation_state_digest={}\n",
        relation.workspace_id,
        relation.commit_id,
        relation.relation_id,
        relation.relation_version_id,
        relation.relation_type,
        relation.relation_label.as_deref().unwrap_or(""),
        relation.source_record_entity_id,
        relation.target_record_entity_id,
        relation.state_digest
    )
}

fn render_session_start(session: &SessionStartResult) -> String {
    format!(
        "session_id={}\nworkspace_id={}\nbranch_id={}\nstarted_at_us={}\nlifecycle_state={}\n",
        session.session_id,
        session.workspace_id,
        session.branch_id,
        session.started_at_us,
        session_lifecycle_state(session.state.lifecycle_state)
    )
}

fn render_session_snapshot(session: &SessionSnapshot) -> Result<String> {
    let focus_entity_id = session
        .focus
        .as_ref()
        .map(|focus| focus.focus_entity_id.to_string())
        .unwrap_or_else(|| "none".to_owned());
    let mut output = format!(
        "session_id={}\nlifecycle_state={}\nstarted_at_us={}\nlast_activity_at_us={}\nmetadata_json={}\nactive_workspace_id={}\nactive_branch_id={}\ncontext_workspaces={}\nfocus_entity_id={}\nfocus_path_entries={}\nsession_diff_id={}\n",
        session.session_id,
        session_lifecycle_state(session.lifecycle_state),
        session.started_at_us,
        render_optional_display_or_none(session.last_activity_at_us.as_ref()),
        canonical_cli_json("session metadata", &session.metadata)?,
        render_optional_display_or_none(session.active_workspace_id.as_ref()),
        render_optional_display_or_none(session.active_branch_id.as_ref()),
        session.context_workspaces.len(),
        focus_entity_id,
        session
            .focus
            .as_ref()
            .map(|focus| focus.path.len())
            .unwrap_or(0),
        render_optional_display_or_none(session.session_diff_id.as_ref())
    );
    for (index, workspace_id) in session.context_workspaces.iter().enumerate() {
        writeln!(
            output,
            "context_workspace.{index}.workspace_id={workspace_id}"
        )
        .expect("write to String");
    }
    if let Some(focus) = &session.focus {
        for (index, path) in focus.path.iter().enumerate() {
            writeln!(
                output,
                "focus_path.{index}.path_entity_id={}",
                path.path_entity_id
            )
            .expect("write to String");
            writeln!(
                output,
                "focus_path.{index}.incoming_relation_id={}",
                render_optional_display_or_none(path.incoming_relation_id.as_ref())
            )
            .expect("write to String");
        }
    }
    Ok(output)
}

fn render_session_list(result: &SessionListResult) -> Result<String> {
    let mut output = format!("sessions={}\n", result.sessions.len());
    for (index, session) in result.sessions.iter().enumerate() {
        writeln!(output, "session.{index}.session_id={}", session.session_id)
            .expect("write to String");
        writeln!(
            output,
            "session.{index}.lifecycle_state={}",
            session_lifecycle_state(session.lifecycle_state)
        )
        .expect("write to String");
        writeln!(
            output,
            "session.{index}.started_at_us={}",
            session.started_at_us
        )
        .expect("write to String");
        writeln!(
            output,
            "session.{index}.last_activity_at_us={}",
            render_optional_display_or_none(session.last_activity_at_us.as_ref())
        )
        .expect("write to String");
        writeln!(
            output,
            "session.{index}.metadata_json={}",
            canonical_cli_json("session metadata", &session.metadata)?
        )
        .expect("write to String");
        writeln!(
            output,
            "session.{index}.active_workspace_id={}",
            render_optional_display_or_none(session.active_workspace_id.as_ref())
        )
        .expect("write to String");
        writeln!(
            output,
            "session.{index}.active_branch_id={}",
            render_optional_display_or_none(session.active_branch_id.as_ref())
        )
        .expect("write to String");
        writeln!(
            output,
            "session.{index}.focus_entity_id={}",
            session
                .focus
                .as_ref()
                .map(|focus| focus.focus_entity_id.to_string())
                .unwrap_or_else(|| "none".to_owned())
        )
        .expect("write to String");
        writeln!(
            output,
            "session.{index}.session_diff_id={}",
            render_optional_display_or_none(session.session_diff_id.as_ref())
        )
        .expect("write to String");
    }
    Ok(output)
}

fn render_session_focus_update(result: &SessionFocusUpdateResult) -> String {
    let focus_entity_id = result
        .state
        .focus
        .as_ref()
        .map(|focus| focus.focus_entity_id.to_string())
        .unwrap_or_else(|| "none".to_owned());
    let focus_path_entries = result
        .state
        .focus
        .as_ref()
        .map(|focus| focus.path.len())
        .unwrap_or(0);
    format!(
        "session_id={}\noccurred_at_us={}\nlifecycle_state={}\nfocus_entity_id={}\nfocus_path_entries={}\n",
        result.session_id,
        result.occurred_at_us,
        session_lifecycle_state(result.state.lifecycle_state),
        focus_entity_id,
        focus_path_entries
    )
}

fn render_session_switch(session: &SessionSwitchResult) -> String {
    let focus_entity_id = session
        .state
        .focus
        .as_ref()
        .map(|focus| focus.focus_entity_id.to_string())
        .unwrap_or_else(|| "none".to_owned());
    format!(
        "session_id={}\nprevious_workspace_id={}\nprevious_branch_id={}\nworkspace_id={}\nbranch_id={}\nreleased_claims={}\nfocus_entity_id={}\noccurred_at_us={}\nlifecycle_state={}\n",
        session.session_id,
        session.previous_workspace_id,
        session.previous_branch_id,
        session.active_workspace_id,
        session.active_branch_id,
        session.released_claims,
        focus_entity_id,
        session.occurred_at_us,
        session_lifecycle_state(session.state.lifecycle_state)
    )
}

fn render_session_end(session: &SessionEndResult) -> String {
    format!(
        "session_id={}\nsession_diff_id={}\nended_at_us={}\nlifecycle_state={}\n",
        session.session_id,
        session.session_diff_id,
        session.ended_at_us,
        session_lifecycle_state(session.state.lifecycle_state)
    )
}

fn render_merge_start(merge: &MergeStartResult) -> String {
    let origin_session_id = merge
        .origin_session_id
        .map(|session_id| session_id.to_string())
        .unwrap_or_else(|| "none".to_owned());
    format!(
        "merge_id={}\nworkspace_id={}\ntarget_branch_id={}\nsource_branch_id={}\nmerge_base_commit_id={}\ntarget_head_commit_id={}\nsource_head_commit_id={}\norigin_session_id={}\nevent_id={}\ncreated_at_us={}\nruntime_state={}\n",
        merge.merge_id,
        merge.workspace_id,
        merge.target_branch_id,
        merge.source_branch_id,
        merge.merge_base_commit_id,
        merge.target_head_commit_id,
        merge.source_head_commit_id,
        origin_session_id,
        merge.event_id,
        merge.created_at_us,
        merge.runtime_state.as_str()
    )
}

fn render_merge_abort(merge: &MergeAbortResult) -> String {
    let origin_session_id = merge
        .origin_session_id
        .map(|session_id| session_id.to_string())
        .unwrap_or_else(|| "none".to_owned());
    let abort_session_id = merge
        .abort_session_id
        .map(|session_id| session_id.to_string())
        .unwrap_or_else(|| "none".to_owned());
    format!(
        "merge_id={}\nworkspace_id={}\ntarget_branch_id={}\nsource_branch_id={}\nmerge_base_commit_id={}\ntarget_head_commit_id={}\nsource_head_commit_id={}\norigin_session_id={}\nabort_session_id={}\nevent_id={}\naborted_at_us={}\nruntime_state={}\n",
        merge.merge_id,
        merge.workspace_id,
        merge.target_branch_id,
        merge.source_branch_id,
        merge.merge_base_commit_id,
        merge.target_head_commit_id,
        merge.source_head_commit_id,
        origin_session_id,
        abort_session_id,
        merge.event_id,
        merge.aborted_at_us,
        merge.runtime_state.as_str()
    )
}

fn render_merge_resolve(result: &MergeResolveResult) -> Result<String> {
    let mut output = format!(
        "merge_id={}\nmerge_item_id={}\nworkspace_id={}\nresolution={}\n",
        result.merge_id,
        result.merge_item_id,
        result.workspace_id,
        result.resolution.resolution_kind.as_str()
    );
    render_merge_item_resolution(&mut output, "resolution", &result.resolution)?;
    Ok(output)
}

fn render_merge_freeze(result: &MergeFreezeResolutionsResult) -> String {
    format!(
        "merge_id={}\nworkspace_id={}\nfrozen_items={}\n",
        result.merge_id, result.workspace_id, result.frozen_items
    )
}

fn render_merge_continue(result: &MergeContinueResult) -> String {
    let continued_by_session_id = result
        .continued_by_session_id
        .map(|session_id| session_id.to_string())
        .unwrap_or_else(|| "none".to_owned());
    format!(
        "merge_id={}\nworkspace_id={}\ntarget_branch_id={}\nsource_branch_id={}\nresult_commit_id={}\nchangeset_id={}\nwork_state_digest={}\ncontinued_by_session_id={}\nevent_id={}\ncompleted_at_us={}\nruntime_state={}\n",
        result.merge_id,
        result.workspace_id,
        result.target_branch_id,
        result.source_branch_id,
        result.result_commit_id,
        result.changeset_id,
        result.work_state_digest,
        continued_by_session_id,
        result.event_id,
        result.completed_at_us,
        result.runtime_state.as_str()
    )
}

fn render_merge_attempt(merge: &MergeAttemptSnapshot) -> Result<String> {
    let origin_session_id = merge
        .origin_session_id
        .map(|session_id| session_id.to_string())
        .unwrap_or_else(|| "none".to_owned());
    let mut output = format!(
        "merge_id={}\nworkspace_id={}\ntarget_branch_id={}\nsource_branch_id={}\nmerge_base_commit_id={}\ntarget_head_commit_id={}\nsource_head_commit_id={}\norigin_session_id={}\ncreated_at_us={}\nruntime_state={}\nitems={}\noutcome={}\n",
        merge.merge_id,
        merge.workspace_id,
        merge.target_branch_id,
        merge.source_branch_id,
        merge.merge_base_commit_id,
        merge.target_head_commit_id,
        merge.source_head_commit_id,
        origin_session_id,
        merge.created_at_us,
        merge.runtime_state.as_str(),
        merge.items.len(),
        merge
            .outcome
            .as_ref()
            .map(|outcome| outcome.outcome.as_str())
            .unwrap_or("none")
    );
    for (index, item) in merge.items.iter().enumerate() {
        render_merge_item(&mut output, &format!("item.{index}"), item)?;
    }
    if let Some(outcome) = &merge.outcome {
        render_merge_outcome(&mut output, "outcome", outcome)?;
    }
    Ok(output)
}

fn render_merge_item(output: &mut String, prefix: &str, item: &MergeItemSnapshot) -> Result<()> {
    let (subject_kind, subject_id) = match item.subject {
        Some(MergeItemSubject::Entity(entity_id)) => ("entity", entity_id.to_string()),
        Some(MergeItemSubject::Relation(relation_id)) => ("relation", relation_id.to_string()),
        None => ("none", "none".to_owned()),
    };
    let payload_json = String::from_utf8(canonical_bytes(&item.payload)?).map_err(|error| {
        WorkVcsError::CanonicalEncodingInvalid(format!("merge item payload was not UTF-8: {error}"))
    })?;
    writeln!(output, "{prefix}.merge_item_id={}", item.merge_item_id).expect("write to String");
    writeln!(output, "{prefix}.ordinal={}", item.ordinal).expect("write to String");
    writeln!(
        output,
        "{prefix}.classification={}",
        item.classification.as_str()
    )
    .expect("write to String");
    writeln!(output, "{prefix}.subject_kind={subject_kind}").expect("write to String");
    writeln!(output, "{prefix}.subject_id={subject_id}").expect("write to String");
    writeln!(output, "{prefix}.payload_json={payload_json}").expect("write to String");
    let resolution = item
        .resolution
        .as_ref()
        .map(|resolution| resolution.resolution_kind.as_str())
        .unwrap_or("none");
    writeln!(output, "{prefix}.resolution={resolution}").expect("write to String");
    if let Some(resolution) = &item.resolution {
        render_merge_item_resolution(output, &format!("{prefix}.resolution"), resolution)?;
    }
    Ok(())
}

fn render_merge_item_resolution(
    output: &mut String,
    prefix: &str,
    resolution: &MergeItemResolutionSnapshot,
) -> Result<()> {
    let custom_payload_json = match &resolution.custom_payload {
        Some(custom_payload) => {
            String::from_utf8(canonical_bytes(custom_payload)?).map_err(|error| {
                WorkVcsError::CanonicalEncodingInvalid(format!(
                    "merge custom resolution payload was not UTF-8: {error}"
                ))
            })?
        }
        None => "none".to_owned(),
    };
    let rationale_json =
        String::from_utf8(canonical_bytes(&resolution.rationale)?).map_err(|error| {
            WorkVcsError::CanonicalEncodingInvalid(format!(
                "merge resolution rationale was not UTF-8: {error}"
            ))
        })?;
    let resolved_by_session_id = resolution
        .resolved_by_session_id
        .map(|session_id| session_id.to_string())
        .unwrap_or_else(|| "none".to_owned());
    writeln!(
        output,
        "{prefix}.kind={}",
        resolution.resolution_kind.as_str()
    )
    .expect("write to String");
    writeln!(output, "{prefix}.custom_payload_json={custom_payload_json}")
        .expect("write to String");
    writeln!(output, "{prefix}.rationale_json={rationale_json}").expect("write to String");
    writeln!(
        output,
        "{prefix}.resolved_by_session_id={resolved_by_session_id}"
    )
    .expect("write to String");
    writeln!(
        output,
        "{prefix}.resolved_at_us={}",
        resolution.resolved_at_us
    )
    .expect("write to String");
    Ok(())
}

fn render_merge_list(result: &MergeListResult) -> Result<String> {
    let target_branch_id = result
        .target_branch_id
        .map(|branch_id| branch_id.to_string())
        .unwrap_or_else(|| "none".to_owned());
    let mut output = format!(
        "workspace_id={}\ntarget_branch_id={}\ninclude_closed={}\nmerges={}\n",
        result.workspace_id,
        target_branch_id,
        result.include_closed,
        result.merges.len()
    );
    for (index, merge) in result.merges.iter().enumerate() {
        let origin_session_id = merge
            .origin_session_id
            .map(|session_id| session_id.to_string())
            .unwrap_or_else(|| "none".to_owned());
        let outcome = merge
            .outcome
            .as_ref()
            .map(|outcome| outcome.outcome.as_str())
            .unwrap_or("none");
        writeln!(output, "merge.{index}.merge_id={}", merge.merge_id).expect("write to String");
        writeln!(
            output,
            "merge.{index}.target_branch_id={}",
            merge.target_branch_id
        )
        .expect("write to String");
        writeln!(
            output,
            "merge.{index}.source_branch_id={}",
            merge.source_branch_id
        )
        .expect("write to String");
        writeln!(
            output,
            "merge.{index}.merge_base_commit_id={}",
            merge.merge_base_commit_id
        )
        .expect("write to String");
        writeln!(
            output,
            "merge.{index}.target_head_commit_id={}",
            merge.target_head_commit_id
        )
        .expect("write to String");
        writeln!(
            output,
            "merge.{index}.source_head_commit_id={}",
            merge.source_head_commit_id
        )
        .expect("write to String");
        writeln!(
            output,
            "merge.{index}.origin_session_id={origin_session_id}"
        )
        .expect("write to String");
        writeln!(
            output,
            "merge.{index}.runtime_state={}",
            merge.runtime_state.as_str()
        )
        .expect("write to String");
        writeln!(output, "merge.{index}.items={}", merge.items.len()).expect("write to String");
        writeln!(output, "merge.{index}.outcome={outcome}").expect("write to String");
    }
    Ok(output)
}

fn render_merge_outcome(
    output: &mut String,
    prefix: &str,
    outcome: &MergeOutcomeSnapshot,
) -> Result<()> {
    let result_commit_id = outcome
        .result_commit_id
        .map(|commit_id| commit_id.to_string())
        .unwrap_or_else(|| "none".to_owned());
    let detail_json = String::from_utf8(canonical_bytes(&outcome.detail)?).map_err(|error| {
        WorkVcsError::CanonicalEncodingInvalid(format!(
            "merge outcome detail was not UTF-8: {error}"
        ))
    })?;
    writeln!(output, "{prefix}.kind={}", outcome.outcome.as_str()).expect("write to String");
    writeln!(output, "{prefix}.result_commit_id={result_commit_id}").expect("write to String");
    writeln!(
        output,
        "{prefix}.completed_at_us={}",
        outcome.completed_at_us
    )
    .expect("write to String");
    writeln!(output, "{prefix}.detail_json={detail_json}").expect("write to String");
    Ok(())
}

fn render_claim_task(claim: &ClaimTaskResult) -> String {
    format!(
        "claim_id={}\nsession_id={}\nworkspace_id={}\nbranch_id={}\ntask_entity_id={}\nmode={}\nclaimed_at_us={}\nlifecycle_state={}\n",
        claim.claim_id,
        claim.session_id,
        claim.workspace_id,
        claim.branch_id,
        claim.task_entity_id,
        claim_mode(claim.mode),
        claim.claimed_at_us,
        claim_lifecycle_state(claim.state.lifecycle_state)
    )
}

fn render_claim_show(claim: &ClaimSnapshot) -> String {
    format!(
        "claim_id={}\nsession_id={}\nworkspace_id={}\nbranch_id={}\ntask_entity_id={}\nmode={}\ncreated_at_us={}\nlast_activity_at_us={}\nlifecycle_state={}\n",
        claim.claim_id,
        claim.session_id,
        claim.workspace_id,
        claim.branch_id,
        claim.task_entity_id,
        claim_mode(claim.mode),
        claim.created_at_us,
        render_optional_display(claim.last_activity_at_us.as_ref()),
        claim_lifecycle_state(claim.lifecycle_state)
    )
}

fn render_claim_list(result: &ClaimListResult) -> String {
    let mut output = format!(
        "session_id={}\nclaims={}\n",
        result.session_id,
        result.claims.len()
    );
    for (index, claim) in result.claims.iter().enumerate() {
        let _ = writeln!(output, "claim.{index}.claim_id={}", claim.claim_id);
        let _ = writeln!(output, "claim.{index}.workspace_id={}", claim.workspace_id);
        let _ = writeln!(output, "claim.{index}.branch_id={}", claim.branch_id);
        let _ = writeln!(
            output,
            "claim.{index}.task_entity_id={}",
            claim.task_entity_id
        );
        let _ = writeln!(output, "claim.{index}.mode={}", claim_mode(claim.mode));
        let _ = writeln!(
            output,
            "claim.{index}.created_at_us={}",
            claim.created_at_us
        );
        let _ = writeln!(
            output,
            "claim.{index}.last_activity_at_us={}",
            render_optional_display(claim.last_activity_at_us.as_ref())
        );
        let _ = writeln!(
            output,
            "claim.{index}.lifecycle_state={}",
            claim_lifecycle_state(claim.lifecycle_state)
        );
    }
    output
}

fn render_claim_guard(result: &ClaimGuardResult) -> String {
    let mut output = format!(
        "session_id={}\nworkspace_id={}\nbranch_id={}\nhead_commit_id={}\ntask_entity_id={}\naction={}\nallowed={}\nreason={}\nactive_claims={}\n",
        result.session_id,
        result.workspace_id,
        result.branch_id,
        result.head_commit_id,
        result.task_entity_id,
        claim_guard_action(result.action),
        result.allowed,
        claim_guard_reason(result.reason),
        result.active_claims.len()
    );
    for (index, claim) in result.active_claims.iter().enumerate() {
        let _ = writeln!(output, "active_claim.{index}.claim_id={}", claim.claim_id);
        let _ = writeln!(
            output,
            "active_claim.{index}.session_id={}",
            claim.session_id
        );
        let _ = writeln!(
            output,
            "active_claim.{index}.mode={}",
            claim_mode(claim.mode)
        );
        let _ = writeln!(
            output,
            "active_claim.{index}.last_activity_at_us={}",
            render_optional_display(claim.last_activity_at_us.as_ref())
        );
    }
    output
}

fn render_claim_next(result: &ClaimNextResult) -> String {
    let mut output = format!(
        "session_id={}\nworkspace_id={}\nbranch_id={}\nhead_commit_id={}\ninspected_candidates={}\nselected={}\n",
        result.session_id,
        result.workspace_id,
        result.branch_id,
        result.head_commit_id,
        result.inspected_candidates,
        result.selected.is_some()
    );
    if let Some(claim) = &result.selected {
        let _ = writeln!(output, "claim_id={}", claim.claim_id);
        let _ = writeln!(output, "task_entity_id={}", claim.task_entity_id);
        let _ = writeln!(output, "mode={}", claim_mode(claim.mode));
        let _ = writeln!(output, "claimed_at_us={}", claim.claimed_at_us);
        let _ = writeln!(
            output,
            "lifecycle_state={}",
            claim_lifecycle_state(claim.state.lifecycle_state)
        );
    }
    output
}

fn render_claim_release(claim: &ClaimReleaseResult) -> String {
    format!(
        "claim_id={}\nsession_id={}\nreleased_at_us={}\nlifecycle_state={}\n",
        claim.claim_id,
        claim.session_id,
        claim.released_at_us,
        claim_lifecycle_state(claim.state.lifecycle_state)
    )
}

fn render_context_overview(context: &ContextOverview) -> String {
    let session = &context.session;
    let focus_entity_id = session
        .focus
        .as_ref()
        .map(|focus| focus.focus_entity_id.to_string())
        .unwrap_or_else(|| "none".to_owned());
    let last_activity_at_us = session
        .last_activity_at_us
        .map(|value| value.to_string())
        .unwrap_or_else(|| "none".to_owned());
    let runnable_ready = context
        .runnable_tasks
        .candidates
        .iter()
        .filter(|candidate| candidate.runnable)
        .count();
    let mut output = format!(
        "session_id={}\nlifecycle_state={}\nworkspace_id={}\nbranch_id={}\nbranch_name={}\nhead_commit_id={}\nstate_digest={}\nstarted_at_us={}\nlast_activity_at_us={}\nfocus_entity_id={}\nfocus_path_entries={}\ncontext_workspaces={}\nrunnable_candidates={}\nrunnable_ready={}\nknowledge={}\nknowledge_relations={}\nknowledge_exposure_relations={}\nrecords={}\nrecord_relations={}\nrecord_knowledge_relations={}\n",
        session.session_id,
        session_lifecycle_state(session.lifecycle_state),
        context.branch.workspace_id,
        context.branch.branch_id,
        context.branch.name,
        context.branch.head_commit_id,
        context.branch.state_digest,
        session.started_at_us,
        last_activity_at_us,
        focus_entity_id,
        session
            .focus
            .as_ref()
            .map(|focus| focus.path.len())
            .unwrap_or(0),
        session.context_workspaces.len(),
        context.runnable_tasks.candidates.len(),
        runnable_ready,
        context.knowledge.knowledge.len(),
        context.knowledge_relations.relations.len(),
        context.knowledge_exposure_relations.len(),
        context.records.records.len(),
        context.record_relations.relations.len(),
        context.record_knowledge_relations.relations.len()
    );
    for (index, workspace_id) in session.context_workspaces.iter().enumerate() {
        let _ = writeln!(
            output,
            "context_workspace.{index}.workspace_id={workspace_id}"
        );
    }
    for (index, candidate) in context.runnable_tasks.candidates.iter().enumerate() {
        render_runnable_candidate(&mut output, index, candidate);
    }
    for (index, knowledge) in context.knowledge.knowledge.iter().enumerate() {
        let statement_json =
            serde_json::to_string(&knowledge.state.statement).expect("knowledge statement JSON");
        let scope_json = context_canonical_json(&knowledge.state.scope);
        let provenance_json = context_canonical_json(&knowledge.state.provenance);
        let _ = writeln!(
            output,
            "context_knowledge.{index}.knowledge_entity_id={}",
            knowledge.knowledge_entity_id
        );
        let _ = writeln!(
            output,
            "context_knowledge.{index}.knowledge_entity_version_id={}",
            knowledge.knowledge_entity_version_id
        );
        let _ = writeln!(
            output,
            "context_knowledge.{index}.knowledge_state_digest={}",
            knowledge.state_digest
        );
        let _ = writeln!(
            output,
            "context_knowledge.{index}.knowledge_status={}",
            knowledge.state.status
        );
        let _ = writeln!(
            output,
            "context_knowledge.{index}.knowledge_statement_json={statement_json}"
        );
        let _ = writeln!(
            output,
            "context_knowledge.{index}.knowledge_scope_json={scope_json}"
        );
        let _ = writeln!(
            output,
            "context_knowledge.{index}.knowledge_provenance_json={provenance_json}"
        );
    }
    for (index, relation) in context.knowledge_relations.relations.iter().enumerate() {
        let _ = writeln!(
            output,
            "context_knowledge_relation.{index}.relation_id={}",
            relation.relation_id
        );
        let _ = writeln!(
            output,
            "context_knowledge_relation.{index}.relation_version_id={}",
            relation.relation_version_id
        );
        let _ = writeln!(
            output,
            "context_knowledge_relation.{index}.relation_type={}",
            relation.relation_type
        );
        let _ = writeln!(
            output,
            "context_knowledge_relation.{index}.replacement_knowledge_entity_id={}",
            relation.replacement_knowledge_entity_id
        );
        let _ = writeln!(
            output,
            "context_knowledge_relation.{index}.prior_knowledge_entity_id={}",
            relation.prior_knowledge_entity_id
        );
        let _ = writeln!(
            output,
            "context_knowledge_relation.{index}.relation_state_digest={}",
            relation.state_digest
        );
    }
    for (index, relation) in context.knowledge_exposure_relations.iter().enumerate() {
        let source_knowledge_entity_id = match relation.source {
            WhyRelationEndpoint::Entity {
                entity_kind: WhyEntityKind::Knowledge,
                entity_id,
            } => entity_id.to_string(),
            _ => String::new(),
        };
        let target_exposure_id = match relation.target {
            WhyRelationEndpoint::KnowledgeExposure { exposure_id } => exposure_id.to_string(),
            _ => String::new(),
        };
        let _ = writeln!(
            output,
            "context_knowledge_exposure_relation.{index}.relation_id={}",
            relation.relation_id
        );
        let _ = writeln!(
            output,
            "context_knowledge_exposure_relation.{index}.relation_version_id={}",
            relation.relation_version_id
        );
        let _ = writeln!(
            output,
            "context_knowledge_exposure_relation.{index}.relation_kind={}",
            why_relation_kind(relation.relation_kind)
        );
        let _ = writeln!(
            output,
            "context_knowledge_exposure_relation.{index}.direction={}",
            why_relation_direction(relation.direction)
        );
        let _ = writeln!(
            output,
            "context_knowledge_exposure_relation.{index}.source_knowledge_entity_id={source_knowledge_entity_id}"
        );
        let _ = writeln!(
            output,
            "context_knowledge_exposure_relation.{index}.target_exposure_id={target_exposure_id}"
        );
        let _ = writeln!(
            output,
            "context_knowledge_exposure_relation.{index}.relation_state_digest={}",
            relation.state_digest
        );
    }
    for (index, record) in context.records.records.iter().enumerate() {
        let statement_json =
            serde_json::to_string(&record.state.statement).expect("record statement JSON");
        let _ = writeln!(
            output,
            "context_record.{index}.record_entity_id={}",
            record.record_entity_id
        );
        let _ = writeln!(
            output,
            "context_record.{index}.record_entity_version_id={}",
            record.record_entity_version_id
        );
        let _ = writeln!(
            output,
            "context_record.{index}.record_state_digest={}",
            record.state_digest
        );
        let _ = writeln!(
            output,
            "context_record.{index}.record_kind={}",
            record.state.kind
        );
        let _ = writeln!(
            output,
            "context_record.{index}.record_status={}",
            record.state.status
        );
        let _ = writeln!(
            output,
            "context_record.{index}.record_statement_json={statement_json}"
        );
    }
    for (index, relation) in context.record_relations.relations.iter().enumerate() {
        let _ = writeln!(
            output,
            "context_record_relation.{index}.relation_id={}",
            relation.relation_id
        );
        let _ = writeln!(
            output,
            "context_record_relation.{index}.relation_version_id={}",
            relation.relation_version_id
        );
        let _ = writeln!(
            output,
            "context_record_relation.{index}.relation_type={}",
            relation.relation_type
        );
        let _ = writeln!(
            output,
            "context_record_relation.{index}.relation_label={}",
            relation.relation_label.as_deref().unwrap_or("")
        );
        let _ = writeln!(
            output,
            "context_record_relation.{index}.source_record_entity_id={}",
            relation.source_record_entity_id
        );
        let _ = writeln!(
            output,
            "context_record_relation.{index}.target_record_entity_id={}",
            relation.target_record_entity_id
        );
        let _ = writeln!(
            output,
            "context_record_relation.{index}.relation_state_digest={}",
            relation.state_digest
        );
    }
    for (index, relation) in context
        .record_knowledge_relations
        .relations
        .iter()
        .enumerate()
    {
        let _ = writeln!(
            output,
            "context_record_knowledge_relation.{index}.relation_id={}",
            relation.relation_id
        );
        let _ = writeln!(
            output,
            "context_record_knowledge_relation.{index}.relation_version_id={}",
            relation.relation_version_id
        );
        let _ = writeln!(
            output,
            "context_record_knowledge_relation.{index}.relation_type={}",
            relation.relation_type
        );
        let _ = writeln!(
            output,
            "context_record_knowledge_relation.{index}.source_record_entity_id={}",
            relation.source_record_entity_id
        );
        let _ = writeln!(
            output,
            "context_record_knowledge_relation.{index}.target_knowledge_entity_id={}",
            relation.target_knowledge_entity_id
        );
        let _ = writeln!(
            output,
            "context_record_knowledge_relation.{index}.relation_state_digest={}",
            relation.state_digest
        );
    }
    output
}

fn context_canonical_json(value: &CanonicalValue) -> String {
    String::from_utf8(canonical_bytes(value).expect("canonical context JSON"))
        .expect("canonical context JSON must be UTF-8")
}

fn render_next_work(result: &NextWorkResult) -> String {
    let selected = result.claim_next.selected.is_some();
    let focus_entity_id = result
        .context
        .session
        .focus
        .as_ref()
        .map(|focus| focus.focus_entity_id.to_string())
        .unwrap_or_else(|| "none".to_owned());
    let runnable_ready = result
        .context
        .runnable_tasks
        .candidates
        .iter()
        .filter(|candidate| candidate.runnable)
        .count();
    let mut output = format!(
        "session_id={}\nworkspace_id={}\nbranch_id={}\nhead_commit_id={}\ninspected_candidates={}\nselected={}\ncontext_focus_entity_id={}\ncontext_workspaces={}\ncontext_runnable_candidates={}\ncontext_runnable_ready={}\ncontext_knowledge={}\ncontext_knowledge_relations={}\ncontext_knowledge_exposure_relations={}\ncontext_records={}\ncontext_record_relations={}\ncontext_record_knowledge_relations={}\n",
        result.claim_next.session_id,
        result.claim_next.workspace_id,
        result.claim_next.branch_id,
        result.claim_next.head_commit_id,
        result.claim_next.inspected_candidates,
        selected,
        focus_entity_id,
        result.context.session.context_workspaces.len(),
        result.context.runnable_tasks.candidates.len(),
        runnable_ready,
        result.context.knowledge.knowledge.len(),
        result.context.knowledge_relations.relations.len(),
        result.context.knowledge_exposure_relations.len(),
        result.context.records.records.len(),
        result.context.record_relations.relations.len(),
        result.context.record_knowledge_relations.relations.len()
    );
    if let Some(claim) = &result.claim_next.selected {
        let _ = writeln!(output, "claim_id={}", claim.claim_id);
        let _ = writeln!(output, "task_entity_id={}", claim.task_entity_id);
        let _ = writeln!(output, "mode={}", claim_mode(claim.mode));
        let _ = writeln!(output, "claimed_at_us={}", claim.claimed_at_us);
        let _ = writeln!(
            output,
            "lifecycle_state={}",
            claim_lifecycle_state(claim.state.lifecycle_state)
        );
    }
    output
}

fn render_runnable_tasks(projection: &RunnableTasksProjection) -> String {
    let mut output = format!(
        "session_id={}\nworkspace_id={}\nbranch_id={}\nhead_commit_id={}\ncandidates={}\n",
        projection.session_id,
        projection.workspace_id,
        projection.branch_id,
        projection.head_commit_id,
        projection.candidates.len()
    );
    for (index, candidate) in projection.candidates.iter().enumerate() {
        render_runnable_candidate(&mut output, index, candidate);
    }
    output
}

fn render_runnable_candidate(output: &mut String, index: usize, candidate: &RunnableTaskCandidate) {
    let blocked_reasons = candidate
        .blocked_reasons
        .iter()
        .map(|reason| runnable_blocked_reason(*reason))
        .collect::<Vec<_>>()
        .join(",");
    let unsatisfied_dependencies = candidate
        .unsatisfied_dependency_entity_ids
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(",");
    let _ = writeln!(
        output,
        "candidate.{index}.task_entity_id={}",
        candidate.task.task_entity_id
    );
    let _ = writeln!(
        output,
        "candidate.{index}.task_entity_version_id={}",
        candidate.task.task_entity_version_id
    );
    let _ = writeln!(
        output,
        "candidate.{index}.status={}",
        candidate.task.state.status
    );
    let _ = writeln!(
        output,
        "candidate.{index}.priority={}",
        candidate.task.state.priority
    );
    let _ = writeln!(output, "candidate.{index}.runnable={}", candidate.runnable);
    let _ = writeln!(
        output,
        "candidate.{index}.lifecycle_eligible={}",
        candidate.lifecycle_eligible
    );
    let _ = writeln!(
        output,
        "candidate.{index}.dependency_ready={}",
        candidate.dependency_ready
    );
    let _ = writeln!(
        output,
        "candidate.{index}.claim={}",
        runnable_claim_coordination(&candidate.claim_coordination)
    );
    let _ = writeln!(
        output,
        "candidate.{index}.unsatisfied_dependencies={unsatisfied_dependencies}"
    );
    let _ = writeln!(
        output,
        "candidate.{index}.blocked_reasons={blocked_reasons}"
    );
}

fn session_lifecycle_state(state: SessionLifecycleState) -> &'static str {
    match state {
        SessionLifecycleState::Active => "active",
        SessionLifecycleState::Ended => "ended",
    }
}

fn parse_session_lifecycle_state(value: &str) -> Result<SessionLifecycleState> {
    match value {
        "active" => Ok(SessionLifecycleState::Active),
        "ended" => Ok(SessionLifecycleState::Ended),
        other => Err(WorkVcsError::SessionInvalid(format!(
            "unknown session lifecycle state {other:?}"
        ))),
    }
}

fn claim_lifecycle_state(state: ClaimLifecycleState) -> &'static str {
    match state {
        ClaimLifecycleState::Active => "active",
        ClaimLifecycleState::Released => "released",
    }
}

fn claim_mode(mode: ClaimMode) -> &'static str {
    match mode {
        ClaimMode::Exclusive => "exclusive",
        ClaimMode::Shared => "shared",
    }
}

fn claim_guard_action(action: ClaimGuardAction) -> &'static str {
    match action {
        ClaimGuardAction::TerminalTaskMutation => "terminal_task",
        ClaimGuardAction::StructuralTaskMutation => "structural_task",
    }
}

fn claim_guard_reason(reason: ClaimGuardReason) -> &'static str {
    match reason {
        ClaimGuardReason::Unclaimed => "unclaimed",
        ClaimGuardReason::OwnedExclusiveClaim => "owned_exclusive_claim",
        ClaimGuardReason::UniqueSharedClaimant => "unique_shared_claimant",
        ClaimGuardReason::ExclusiveClaimOwnedByOtherSession => {
            "exclusive_claim_owned_by_other_session"
        }
        ClaimGuardReason::SharedClaimSetDoesNotIncludeSession => {
            "shared_claim_set_does_not_include_session"
        }
        ClaimGuardReason::NonUniqueSharedClaimSet => "non_unique_shared_claim_set",
    }
}

fn runnable_claim_coordination(claim: &RunnableTaskClaimCoordination) -> String {
    match claim {
        RunnableTaskClaimCoordination::Unclaimed => "unclaimed".to_owned(),
        RunnableTaskClaimCoordination::ClaimedBySession { claim_id } => {
            format!("claimed_by_session:{claim_id}")
        }
        RunnableTaskClaimCoordination::ClaimedByOtherSession {
            claim_id,
            session_id,
        } => format!("claimed_by_other_session:{claim_id}:{session_id}"),
        RunnableTaskClaimCoordination::Shared {
            claim_ids,
            session_ids,
            claimed_by_session,
        } => {
            let claim_ids = claim_ids
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(",");
            let session_ids = session_ids
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(",");
            format!("shared:{claimed_by_session}:{claim_ids}:{session_ids}")
        }
    }
}

fn runnable_blocked_reason(reason: RunnableTaskBlockedReason) -> &'static str {
    match reason {
        RunnableTaskBlockedReason::LifecycleIneligible => "lifecycle_ineligible",
        RunnableTaskBlockedReason::DependencyBlocked => "dependency_blocked",
        RunnableTaskBlockedReason::ClaimBlocked => "claim_blocked",
    }
}

fn render_history_entry(output: &mut String, entry: &HistoryEntry) {
    let parent = entry
        .parent_commit_id
        .map(|commit_id| commit_id.to_string())
        .unwrap_or_else(|| "none".to_owned());
    let _ = writeln!(
        output,
        "commit={} changeset={} kind={} operation={} schema={} parent={} committed_at_us={} changeset_created_at_us={} state_digest={}",
        entry.commit_id,
        entry.changeset_id,
        entry.commit_kind,
        entry.operation_type,
        entry.operation_schema_version,
        parent,
        entry.committed_at_us,
        entry.changeset_created_at_us,
        entry.state_digest
    );
}

fn render_changeset_snapshot(changeset: &ChangeSetSnapshot) -> String {
    let origin_session_id = changeset
        .origin_session_id
        .map(|session_id| session_id.to_string())
        .unwrap_or_else(|| "none".to_owned());
    let mut output = format!(
        "workspace_id={}\nchangeset_id={}\noperation_type={}\noperation_schema_version={}\ncreated_at_us={}\norigin_session_id={}\noperation_payload_digest={}\noperation_payload_size_bytes={}\noperation_payload_json={}\nrationale_digest={}\nrationale_size_bytes={}\nrationale_json={}\nchange_operations={}\ncausal_anchors={}\nevents={}\ncommits={}\n",
        changeset.workspace_id,
        changeset.changeset_id,
        changeset.operation_type,
        changeset.operation_schema_version,
        changeset.created_at_us,
        origin_session_id,
        changeset.operation_payload_digest,
        changeset.operation_payload_size_bytes,
        changeset.operation_payload_json,
        changeset.rationale_digest,
        changeset.rationale_size_bytes,
        changeset.rationale_json,
        changeset.change_operation_count,
        changeset.causal_anchor_count,
        changeset.event_count,
        changeset.commits.len()
    );
    for (index, commit) in changeset.commits.iter().enumerate() {
        let _ = writeln!(output, "commit[{index}].commit_id={}", commit.commit_id);
        let _ = writeln!(output, "commit[{index}].commit_kind={}", commit.commit_kind);
        let _ = writeln!(
            output,
            "commit[{index}].state_digest={}",
            commit.state_digest
        );
        let _ = writeln!(
            output,
            "commit[{index}].committed_at_us={}",
            commit.committed_at_us
        );
    }
    output
}

fn render_change_operations(result: &ChangeOperationListResult) -> String {
    let mut output = format!(
        "workspace_id={}\nchangeset_id={}\noperations={}\n",
        result.workspace_id,
        result.changeset_id,
        result.operations.len()
    );
    for (index, operation) in result.operations.iter().enumerate() {
        let _ = writeln!(
            output,
            "operation[{index}].operation_id={}",
            operation.operation_id
        );
        let _ = writeln!(output, "operation[{index}].ordinal={}", operation.ordinal);
        let _ = writeln!(
            output,
            "operation[{index}].subject_family={}",
            operation.subject.family()
        );
        let _ = writeln!(
            output,
            "operation[{index}].subject_object_id={}",
            operation.subject.object_id()
        );
        let _ = writeln!(
            output,
            "operation[{index}].operation_payload_digest={}",
            operation.operation_payload_digest
        );
        let _ = writeln!(
            output,
            "operation[{index}].operation_payload_size_bytes={}",
            operation.operation_payload_size_bytes
        );
        let _ = writeln!(
            output,
            "operation[{index}].operation_payload_json={}",
            operation.operation_payload_json
        );
    }
    output
}

fn render_changeset_causal_anchors(result: &ChangeSetCausalAnchorListResult) -> String {
    let mut output = format!(
        "workspace_id={}\nchangeset_id={}\ncausal_anchors={}\n",
        result.workspace_id,
        result.changeset_id,
        result.anchors.len()
    );
    for (index, anchor) in result.anchors.iter().enumerate() {
        let _ = writeln!(output, "anchor[{index}].ordinal={}", anchor.ordinal);
        let _ = writeln!(
            output,
            "anchor[{index}].object_id={}",
            anchor.anchor_object_id
        );
        let _ = writeln!(
            output,
            "anchor[{index}].object_kind={}",
            anchor.anchor_object_kind
        );
    }
    output
}

fn render_commit_snapshot(commit: &CommitSnapshot) -> String {
    let origin_session_id = commit
        .origin_session_id
        .map(|session_id| session_id.to_string())
        .unwrap_or_else(|| "none".to_owned());
    let mut output = format!(
        "workspace_id={}\ncommit_id={}\nchangeset_id={}\ncommit_kind={}\nstate_digest={}\ncommitted_at_us={}\noperation_type={}\noperation_schema_version={}\nchangeset_created_at_us={}\norigin_session_id={}\nparents={}\n",
        commit.workspace_id,
        commit.commit_id,
        commit.changeset_id,
        commit.commit_kind,
        commit.state_digest,
        commit.committed_at_us,
        commit.operation_type,
        commit.operation_schema_version,
        commit.changeset_created_at_us,
        origin_session_id,
        commit.parents.len()
    );
    for (index, parent) in commit.parents.iter().enumerate() {
        let _ = writeln!(output, "parent[{index}].ordinal={}", parent.parent_ordinal);
        let _ = writeln!(output, "parent[{index}].role={}", parent.parent_role);
        let _ = writeln!(
            output,
            "parent[{index}].commit_id={}",
            parent.parent_commit_id
        );
    }
    output
}

fn render_event_snapshot(event: &EventSnapshot) -> String {
    let mut output = String::new();
    render_event_fields(&mut output, None, event);
    output
}

fn render_event_list(result: &EventListResult) -> String {
    let mut output = format!("events={}\n", result.events.len());
    for (index, event) in result.events.iter().enumerate() {
        render_event_fields(&mut output, Some(index), event);
    }
    output
}

fn render_event_fields(output: &mut String, index: Option<usize>, event: &EventSnapshot) {
    let prefix = index
        .map(|index| format!("event[{index}]."))
        .unwrap_or_default();
    let workspace_id = event
        .workspace_id
        .map(|workspace_id| workspace_id.to_string())
        .unwrap_or_else(|| "none".to_owned());
    let changeset_id = event
        .changeset_id
        .map(|changeset_id| changeset_id.to_string())
        .unwrap_or_else(|| "none".to_owned());
    let session_id = event
        .session_id
        .map(|session_id| session_id.to_string())
        .unwrap_or_else(|| "none".to_owned());
    let _ = writeln!(output, "{prefix}event_id={}", event.event_id);
    let _ = writeln!(output, "{prefix}workspace_id={workspace_id}");
    let _ = writeln!(output, "{prefix}changeset_id={changeset_id}");
    let _ = writeln!(output, "{prefix}session_id={session_id}");
    let _ = writeln!(output, "{prefix}event_kind={}", event.event_kind);
    let _ = writeln!(output, "{prefix}occurred_at_us={}", event.occurred_at_us);
    let _ = writeln!(output, "{prefix}payload_digest={}", event.payload_digest);
    let _ = writeln!(
        output,
        "{prefix}payload_size_bytes={}",
        event.payload_size_bytes
    );
    let _ = writeln!(output, "{prefix}payload_json={}", event.payload_json);
}

fn render_replayed_state(state: &ReplayedState) -> String {
    let mut output = format!(
        "workspace_id={}\ncommit_id={}\nstate_digest={}\n",
        state.workspace_id, state.commit_id, state.state_digest
    );
    render_work_state(&mut output, &state.state);
    output
}

fn render_work_state_restore(restore: &WorkStateRestoreCommit) -> String {
    format!(
        "workspace_id={}\nbranch_id={}\nprevious_head_commit_id={}\ntarget_commit_id={}\ncommit_id={}\nchangeset_id={}\noperation_count={}\nwork_state_digest={}\n",
        restore.workspace_id,
        restore.branch_id,
        restore.previous_head_commit_id,
        restore.target_commit_id,
        restore.commit_id,
        restore.changeset_id,
        restore.operation_count,
        restore.work_state_digest
    )
}

fn render_branch_projection_refresh(refresh: &BranchProjectionRefreshResult) -> String {
    format!(
        "workspace_id={}\nbranch_id={}\nprojected_commit_id={}\nprojection_state_digest={}\nentity_count={}\nrelation_count={}\nupdated_at_us={}\n",
        refresh.workspace_id,
        refresh.branch_id,
        refresh.projected_commit_id,
        refresh.projection_state_digest,
        refresh.entity_count,
        refresh.relation_count,
        refresh.updated_at_us
    )
}

fn render_branch_projection_snapshot(snapshot: &BranchProjectionSnapshot) -> String {
    format!(
        "workspace_id={}\nbranch_id={}\nhead_commit_id={}\nhead_state_digest={}\nstatus={}\nstored_status={}\nprojected_commit_id={}\nprojection_state_digest={}\nentity_count={}\nrelation_count={}\nupdated_at_us={}\nis_current={}\n",
        snapshot.workspace_id,
        snapshot.branch_id,
        snapshot.head_commit_id,
        snapshot.head_state_digest,
        snapshot.status.as_str(),
        snapshot.stored_status.as_deref().unwrap_or("none"),
        render_optional_display(snapshot.projected_commit_id.as_ref()),
        render_optional_display(snapshot.projection_state_digest.as_ref()),
        snapshot.entity_count,
        snapshot.relation_count,
        snapshot
            .updated_at_us
            .map(|value| value.to_string())
            .unwrap_or_else(|| "none".to_owned()),
        snapshot.is_current()
    )
}

fn render_bundle_export_manifest(manifest: &BundleExportManifest) -> String {
    let mut output = format!(
        "bundle_manifest_profile={}\nbundle_manifest_version={}\nstore_id={}\nworkspace_id={}\ncommit_id={}\nstate_digest={}\nmanifest_digest={}\nmanifest_size_bytes={}\ncommits={}\nexported_branch_heads={}\nentities={}\nrelations={}\nentity_versions={}\nacceptance_criterion_identities={}\nverification_requirement_identities={}\nrelation_versions={}\ncontent_objects={}\nsessions={}\nsession_diffs={}\nevidences={}\nevidence_contents={}\nresources={}\nresource_observations={}\nverification_bases={}\nverification_resource_bases={}\nverification_semantic_dependencies={}\nevents={}\nknowledge_spaces={}\nknowledge_exposures={}\nknowledge_exposure_local_sources={}\nknowledge_exposure_transitions={}\nknowledge_exposure_source_statuses={}\nentity_membership_changes={}\nrelation_membership_changes={}\nchangeset_causal_anchors={}\ncheckpoint_candidates={}\n",
        manifest.manifest_profile,
        manifest.manifest_version,
        manifest.store_id,
        manifest.workspace_id,
        manifest.commit_id,
        manifest.state_digest,
        manifest.manifest_digest,
        manifest.manifest_size_bytes,
        manifest.commit_count,
        manifest.exported_branch_heads.len(),
        manifest.entity_count,
        manifest.relation_count,
        manifest.entity_versions.len(),
        manifest.acceptance_criterion_identities.len(),
        manifest.verification_requirement_identities.len(),
        manifest.relation_versions.len(),
        manifest.content_objects.len(),
        manifest.sessions.len(),
        manifest.session_diffs.len(),
        manifest.evidences.len(),
        manifest.evidence_contents.len(),
        manifest.resources.len(),
        manifest.resource_observations.len(),
        manifest.verification_bases.len(),
        manifest.verification_resource_bases.len(),
        manifest.verification_semantic_dependencies.len(),
        manifest.events.len(),
        manifest.knowledge_spaces.len(),
        manifest.knowledge_exposures.len(),
        manifest.knowledge_exposure_local_sources.len(),
        manifest.knowledge_exposure_transitions.len(),
        manifest.knowledge_exposure_source_statuses.len(),
        manifest.entity_membership_changes.len(),
        manifest.relation_membership_changes.len(),
        manifest.changeset_causal_anchors.len(),
        manifest.checkpoint_candidates.len()
    );
    for (index, anchor) in manifest.changeset_causal_anchors.iter().enumerate() {
        writeln!(
            output,
            "changeset_causal_anchor[{index}].changeset_id={}",
            anchor.changeset_id
        )
        .expect("write to String");
        writeln!(
            output,
            "changeset_causal_anchor[{index}].anchor_object_id={}",
            anchor.anchor_object_id
        )
        .expect("write to String");
        writeln!(
            output,
            "changeset_causal_anchor[{index}].anchor_object_kind={}",
            anchor.anchor_object_kind
        )
        .expect("write to String");
    }
    for (index, event) in manifest.events.iter().enumerate() {
        writeln!(output, "event[{index}].id={}", event.event_id).expect("write to String");
        writeln!(output, "event[{index}].kind={}", event.event_kind).expect("write to String");
    }
    for (index, checkpoint) in manifest.checkpoint_candidates.iter().enumerate() {
        writeln!(
            output,
            "checkpoint_candidate[{index}].id={}",
            checkpoint.checkpoint_id
        )
        .expect("write to String");
        writeln!(
            output,
            "checkpoint_candidate[{index}].commit_id={}",
            checkpoint.commit_id
        )
        .expect("write to String");
        writeln!(
            output,
            "checkpoint_candidate[{index}].state_digest={}",
            checkpoint.state_digest
        )
        .expect("write to String");
        writeln!(
            output,
            "checkpoint_candidate[{index}].content_digest={}",
            checkpoint.content_digest
        )
        .expect("write to String");
        writeln!(
            output,
            "checkpoint_candidate[{index}].usability_state={}",
            checkpoint.usability_state
        )
        .expect("write to String");
    }
    output
}

fn render_bundle_export_manifest_json(manifest: &BundleExportManifest) -> Result<String> {
    String::from_utf8(canonical_bytes(&manifest.manifest)?).map_err(|error| {
        WorkVcsError::CanonicalEncodingInvalid(format!(
            "bundle export manifest JSON was not UTF-8: {error}"
        ))
    })
}

fn write_bundle_payload_export_directory(
    output_dir: &Path,
    export: &BundlePayloadExport,
) -> Result<()> {
    fs::create_dir_all(output_dir).map_err(|error| {
        WorkVcsError::QueryInvalid(format!(
            "failed to create bundle output directory {}: {error}",
            output_dir.display()
        ))
    })?;
    write_new_file(output_dir.join("manifest.json"), &export.manifest_bytes)?;
    write_new_file(
        output_dir.join("payload-index.json"),
        &export.payload_index_bytes,
    )?;
    fs::create_dir_all(output_dir.join("payloads")).map_err(|error| {
        WorkVcsError::QueryInvalid(format!(
            "failed to create bundle payload directory {}: {error}",
            output_dir.join("payloads").display()
        ))
    })?;
    for payload in &export.payload_files {
        write_new_file(output_dir.join(&payload.relative_path), &payload.bytes)?;
    }
    Ok(())
}

fn write_new_file(path: PathBuf, bytes: &[u8]) -> Result<()> {
    fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .and_then(|mut file| std::io::Write::write_all(&mut file, bytes))
        .map_err(|error| {
            WorkVcsError::QueryInvalid(format!(
                "failed to write bundle export file {}: {error}",
                path.display()
            ))
        })
}

fn read_bundle_file(path: &Path) -> Result<Vec<u8>> {
    fs::read(path).map_err(|error| {
        WorkVcsError::QueryInvalid(format!(
            "failed to read bundle file {}: {error}",
            path.display()
        ))
    })
}

fn read_bundle_payload_inputs(input_dir: &Path) -> Result<Vec<BundlePayloadInput>> {
    let payload_dir = input_dir.join("payloads");
    let entries = fs::read_dir(&payload_dir).map_err(|error| {
        WorkVcsError::QueryInvalid(format!(
            "failed to read bundle payload directory {}: {error}",
            payload_dir.display()
        ))
    })?;
    let mut payloads = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|error| {
            WorkVcsError::QueryInvalid(format!(
                "failed to read bundle payload directory entry {}: {error}",
                payload_dir.display()
            ))
        })?;
        let file_type = entry.file_type().map_err(|error| {
            WorkVcsError::QueryInvalid(format!(
                "failed to inspect bundle payload file {}: {error}",
                entry.path().display()
            ))
        })?;
        if !file_type.is_file() {
            return Err(WorkVcsError::QueryInvalid(format!(
                "bundle payload path {} is not a file",
                entry.path().display()
            )));
        }
        let file_name = entry.file_name().into_string().map_err(|_| {
            WorkVcsError::QueryInvalid(format!(
                "bundle payload file name {} is not valid UTF-8",
                entry.path().display()
            ))
        })?;
        let relative_path = format!("payloads/{file_name}");
        let bytes = read_bundle_file(&entry.path())?;
        payloads.push(BundlePayloadInput::new(relative_path, bytes)?);
    }
    payloads.sort_by(|left, right| left.relative_path.cmp(&right.relative_path));
    Ok(payloads)
}

fn render_bundle_payload_export(export: &BundlePayloadExport, output_dir: &Path) -> String {
    format!(
        "bundle_payload_export_profile={}\nbundle_payload_index_version={}\noutput_dir={}\ncommit_id={}\nmanifest_digest={}\npayload_index_digest={}\npayload_index_size_bytes={}\npayload_files={}\npayload_references={}\n",
        "workvcs-local-payload-index-v1",
        1,
        output_dir.display(),
        export.manifest.commit_id,
        export.manifest.manifest_digest,
        export.payload_index_digest,
        export.payload_index_size_bytes,
        export.payload_files.len(),
        export.payload_references.len()
    )
}

fn render_bundle_payload_validation(result: &BundlePayloadValidationResult) -> String {
    format!(
        "commit_id={}\nvalid={}\nexpected_manifest_digest={}\nactual_manifest_digest={}\nexpected_payload_index_digest={}\nactual_payload_index_digest={}\nexpected_payload_files={}\nactual_payload_files={}\nexpected_payload_references={}\nproblem={}\n",
        result.commit_id,
        result.valid,
        result.expected_manifest_digest,
        result.actual_manifest_digest,
        result.expected_payload_index_digest,
        result.actual_payload_index_digest,
        result.expected_payload_files,
        result.actual_payload_files,
        result.expected_payload_references,
        result.problem.as_deref().unwrap_or("none")
    )
}

fn render_bundle_import_preflight(result: &BundleImportPreflightResult) -> String {
    format!(
        "valid={}\nformat_compatible={}\nsource_store_id={}\ntarget_workspace_id={}\ntarget_commit_id={}\ntarget_state_digest={}\nsource_store_relation={}\nincoming_commit_present={}\nimport_required={}\ncan_apply={}\naction={}\nmanifest_digest={}\npayload_index_digest={}\npayload_files={}\npayload_references={}\nexported_branch_heads={}\nbranch_heads_already_present={}\nbranch_heads_missing={}\nbranch_heads_fast_forward={}\nbranch_heads_diverged={}\nproblem={}\n",
        result.valid,
        result.format_compatible,
        result
            .source_store_id
            .map(|value| value.to_string())
            .unwrap_or_else(|| "none".to_owned()),
        result
            .target_workspace_id
            .map(|value| value.to_string())
            .unwrap_or_else(|| "none".to_owned()),
        result
            .target_commit_id
            .map(|value| value.to_string())
            .unwrap_or_else(|| "none".to_owned()),
        result
            .target_state_digest
            .map(|value| value.to_string())
            .unwrap_or_else(|| "none".to_owned()),
        result.source_store_relation,
        result.incoming_commit_present,
        result.import_required,
        result.can_apply,
        result.action,
        result.manifest_digest,
        result.payload_index_digest,
        result.payload_files,
        result.payload_references,
        result.exported_branch_heads,
        result.branch_heads_already_present,
        result.branch_heads_missing,
        result.branch_heads_fast_forward,
        result.branch_heads_diverged,
        result.problem.as_deref().unwrap_or("none")
    )
}

fn render_bundle_import_attempt(result: &BundleImportAttemptResult) -> String {
    format!(
        "recorded={}\nimport_id={}\nbundle_digest={}\nimport_profile={}\nstarted_at_us={}\ncompleted_at_us={}\noutcome={}\nvalid={}\nformat_compatible={}\nsource_store_id={}\ntarget_workspace_id={}\ntarget_commit_id={}\ntarget_state_digest={}\nsource_store_relation={}\nincoming_commit_present={}\nimport_required={}\ncan_apply={}\nexported_branch_heads={}\nbranch_heads_already_present={}\nbranch_heads_missing={}\nbranch_heads_fast_forward={}\nbranch_heads_diverged={}\nproblem={}\n",
        result.recorded,
        render_optional_display_or_none(result.import_id.as_ref()),
        result.bundle_digest,
        result.import_profile,
        render_optional_display_or_none(result.started_at_us.as_ref()),
        render_optional_display_or_none(result.completed_at_us.as_ref()),
        result.outcome,
        result.preflight.valid,
        result.preflight.format_compatible,
        render_optional_display_or_none(result.preflight.source_store_id.as_ref()),
        render_optional_display_or_none(result.preflight.target_workspace_id.as_ref()),
        render_optional_display_or_none(result.preflight.target_commit_id.as_ref()),
        render_optional_display_or_none(result.preflight.target_state_digest.as_ref()),
        result.preflight.source_store_relation,
        result.preflight.incoming_commit_present,
        result.preflight.import_required,
        result.preflight.can_apply,
        result.preflight.exported_branch_heads,
        result.preflight.branch_heads_already_present,
        result.preflight.branch_heads_missing,
        result.preflight.branch_heads_fast_forward,
        result.preflight.branch_heads_diverged,
        result.preflight.problem.as_deref().unwrap_or("none")
    )
}

fn render_bundle_import_apply(result: &BundleImportApplyResult) -> String {
    format!(
        "applied={}\nimport_id={}\nbundle_digest={}\nimport_profile={}\nstarted_at_us={}\ncompleted_at_us={}\noutcome={}\nvalid={}\nformat_compatible={}\nsource_store_id={}\ntarget_workspace_id={}\ntarget_commit_id={}\ntarget_state_digest={}\nsource_store_relation={}\nincoming_commit_present={}\nimport_required={}\ncan_apply={}\nexported_branch_heads={}\nbranch_heads_already_present={}\nbranch_heads_missing={}\nbranch_heads_fast_forward={}\nbranch_heads_diverged={}\nimported_commits={}\nimported_entity_versions={}\nimported_acceptance_criterion_identities={}\nimported_verification_requirement_identities={}\nimported_content_objects={}\nimported_sessions={}\nimported_session_diffs={}\nimported_evidences={}\nimported_resources={}\nimported_resource_observations={}\nimported_verification_bases={}\nimported_events={}\nimported_knowledge_spaces={}\nimported_knowledge_exposures={}\nimported_knowledge_exposure_local_sources={}\nimported_knowledge_exposure_transitions={}\nimported_knowledge_exposure_source_statuses={}\nimported_relation_versions={}\nimported_changeset_causal_anchors={}\nimported_checkpoints={}\nimported_checkpoint_statuses={}\nupdated_branch_heads={}\nproblem={}\n",
        result.applied,
        render_optional_display_or_none(result.import_id.as_ref()),
        result.bundle_digest,
        result.import_profile,
        render_optional_display_or_none(result.started_at_us.as_ref()),
        render_optional_display_or_none(result.completed_at_us.as_ref()),
        result.outcome,
        result.preflight.valid,
        result.preflight.format_compatible,
        render_optional_display_or_none(result.preflight.source_store_id.as_ref()),
        render_optional_display_or_none(result.preflight.target_workspace_id.as_ref()),
        render_optional_display_or_none(result.preflight.target_commit_id.as_ref()),
        render_optional_display_or_none(result.preflight.target_state_digest.as_ref()),
        result.preflight.source_store_relation,
        result.preflight.incoming_commit_present,
        result.preflight.import_required,
        result.preflight.can_apply,
        result.preflight.exported_branch_heads,
        result.preflight.branch_heads_already_present,
        result.preflight.branch_heads_missing,
        result.preflight.branch_heads_fast_forward,
        result.preflight.branch_heads_diverged,
        result.imported_commits,
        result.imported_entity_versions,
        result.imported_acceptance_criterion_identities,
        result.imported_verification_requirement_identities,
        result.imported_content_objects,
        result.imported_sessions,
        result.imported_session_diffs,
        result.imported_evidences,
        result.imported_resources,
        result.imported_resource_observations,
        result.imported_verification_bases,
        result.imported_events,
        result.imported_knowledge_spaces,
        result.imported_knowledge_exposures,
        result.imported_knowledge_exposure_local_sources,
        result.imported_knowledge_exposure_transitions,
        result.imported_knowledge_exposure_source_statuses,
        result.imported_relation_versions,
        result.imported_changeset_causal_anchors,
        result.imported_checkpoints,
        result.imported_checkpoint_statuses,
        result.updated_branch_heads,
        result.preflight.problem.as_deref().unwrap_or("none")
    )
}

fn render_bundle_import_attempt_snapshot(snapshot: &BundleImportAttemptSnapshot) -> String {
    let mut output = String::new();
    write_bundle_import_attempt_snapshot_fields(&mut output, None, snapshot);
    output
}

fn render_bundle_import_attempt_list(result: &BundleImportAttemptListResult) -> String {
    let mut output = format!("imports={}\n", result.attempts.len());
    for (index, snapshot) in result.attempts.iter().enumerate() {
        write_bundle_import_attempt_snapshot_fields(
            &mut output,
            Some(&format!("import[{index}]")),
            snapshot,
        );
    }
    output
}

fn write_bundle_import_attempt_snapshot_fields(
    output: &mut String,
    prefix: Option<&str>,
    snapshot: &BundleImportAttemptSnapshot,
) {
    let key = |name: &str| {
        prefix
            .map(|prefix| format!("{prefix}.{name}"))
            .unwrap_or_else(|| name.to_owned())
    };
    let (outcome, completed_at_us, detail_digest, detail_size_bytes) =
        if let Some(outcome) = &snapshot.outcome {
            (
                outcome.outcome.clone(),
                outcome.completed_at_us.to_string(),
                outcome.detail_digest.to_string(),
                outcome.detail_size_bytes.to_string(),
            )
        } else {
            (
                "none".to_owned(),
                "none".to_owned(),
                "none".to_owned(),
                "none".to_owned(),
            )
        };
    writeln!(output, "{}={}", key("import_id"), snapshot.import_id).expect("write to String");
    writeln!(
        output,
        "{}={}",
        key("source_store_id"),
        snapshot.source_store_id
    )
    .expect("write to String");
    writeln!(
        output,
        "{}={}",
        key("bundle_digest"),
        snapshot.bundle_digest
    )
    .expect("write to String");
    writeln!(
        output,
        "{}={}",
        key("import_profile"),
        snapshot.import_profile
    )
    .expect("write to String");
    writeln!(
        output,
        "{}={}",
        key("origin_session_id"),
        render_optional_display_or_none(snapshot.origin_session_id.as_ref())
    )
    .expect("write to String");
    writeln!(
        output,
        "{}={}",
        key("started_at_us"),
        snapshot.started_at_us
    )
    .expect("write to String");
    writeln!(output, "{}={outcome}", key("outcome")).expect("write to String");
    writeln!(output, "{}={completed_at_us}", key("completed_at_us")).expect("write to String");
    writeln!(output, "{}={detail_digest}", key("detail_digest")).expect("write to String");
    writeln!(output, "{}={detail_size_bytes}", key("detail_size_bytes")).expect("write to String");
}

fn render_optional_display_or_none<T: std::fmt::Display>(value: Option<&T>) -> String {
    value
        .map(std::string::ToString::to_string)
        .unwrap_or_else(|| "none".to_owned())
}

fn render_store_lineage_record_result(result: &StoreLineageRecordResult) -> Result<String> {
    render_store_lineage_snapshot(&result.lineage)
}

fn render_store_lineage_snapshot(snapshot: &StoreLineageSnapshot) -> Result<String> {
    let mut output = String::new();
    write_store_lineage_snapshot_fields(&mut output, None, snapshot)?;
    Ok(output)
}

fn render_store_lineage_list(result: &StoreLineageListResult) -> Result<String> {
    let mut output = format!("lineages={}\n", result.lineages.len());
    for (index, snapshot) in result.lineages.iter().enumerate() {
        write_store_lineage_snapshot_fields(
            &mut output,
            Some(&format!("lineage[{index}]")),
            snapshot,
        )?;
    }
    Ok(output)
}

fn write_store_lineage_snapshot_fields(
    output: &mut String,
    prefix: Option<&str>,
    snapshot: &StoreLineageSnapshot,
) -> Result<()> {
    let key = |name: &str| {
        prefix
            .map(|prefix| format!("{prefix}.{name}"))
            .unwrap_or_else(|| name.to_owned())
    };
    writeln!(output, "{}={}", key("lineage_id"), snapshot.lineage_id).expect("write to String");
    writeln!(
        output,
        "{}={}",
        key("source_store_id"),
        snapshot.source_store_id
    )
    .expect("write to String");
    writeln!(
        output,
        "{}={}",
        key("derivation_kind"),
        snapshot.derivation_kind
    )
    .expect("write to String");
    writeln!(
        output,
        "{}={}",
        key("source_root_descriptor_json"),
        canonical_cli_json(
            "store lineage source_root_descriptor",
            &snapshot.source_root_descriptor
        )?
    )
    .expect("write to String");
    writeln!(
        output,
        "{}={}",
        key("source_root_descriptor_digest"),
        snapshot.source_root_descriptor_digest
    )
    .expect("write to String");
    writeln!(
        output,
        "{}={}",
        key("source_root_descriptor_size_bytes"),
        snapshot.source_root_descriptor_size_bytes
    )
    .expect("write to String");
    writeln!(
        output,
        "{}={}",
        key("source_bundle_digest"),
        render_optional_display_or_none(snapshot.source_bundle_digest.as_ref())
    )
    .expect("write to String");
    writeln!(
        output,
        "{}={}",
        key("created_at_us"),
        snapshot.created_at_us
    )
    .expect("write to String");
    Ok(())
}

fn canonical_cli_json(label: &str, value: &CanonicalValue) -> Result<String> {
    String::from_utf8(canonical_bytes(value)?).map_err(|error| {
        WorkVcsError::CanonicalEncodingInvalid(format!("{label} was not UTF-8: {error}"))
    })
}

fn render_knowledge_space_create_result(result: &KnowledgeSpaceCreateResult) -> String {
    render_knowledge_space_snapshot(&result.knowledge_space)
}

fn render_knowledge_space_snapshot(snapshot: &KnowledgeSpaceSnapshot) -> String {
    let mut output = String::new();
    write_knowledge_space_snapshot_fields(&mut output, None, snapshot);
    output
}

fn render_knowledge_space_list(result: &KnowledgeSpaceListResult) -> String {
    let mut output = format!("knowledge_spaces={}\n", result.knowledge_spaces.len());
    for (index, snapshot) in result.knowledge_spaces.iter().enumerate() {
        write_knowledge_space_snapshot_fields(
            &mut output,
            Some(&format!("knowledge_space[{index}]")),
            snapshot,
        );
    }
    output
}

fn render_knowledge_space_available_exposures(
    result: &KnowledgeSpaceAvailableExposuresResult,
) -> Result<String> {
    let mut output = format!(
        "knowledge_space_id={}\nexposures={}\n",
        result.knowledge_space_id,
        result.exposures.len()
    );
    for (index, snapshot) in result.exposures.iter().enumerate() {
        write_knowledge_exposure_snapshot_fields(
            &mut output,
            Some(&format!("exposure[{index}]")),
            snapshot,
        )?;
    }
    Ok(output)
}

fn render_knowledge_space_source_stale_exposures(
    result: &KnowledgeSpaceSourceStaleExposuresResult,
) -> Result<String> {
    let mut output = format!(
        "knowledge_space_id={}\nexposures={}\n",
        result.knowledge_space_id,
        result.exposures.len()
    );
    for (index, snapshot) in result.exposures.iter().enumerate() {
        write_knowledge_exposure_snapshot_fields(
            &mut output,
            Some(&format!("exposure[{index}]")),
            snapshot,
        )?;
    }
    Ok(output)
}

fn render_knowledge_space_historical_exposures(
    result: &KnowledgeSpaceHistoricalExposuresResult,
) -> Result<String> {
    let mut output = format!(
        "knowledge_space_id={}\nexposures={}\n",
        result.knowledge_space_id,
        result.exposures.len()
    );
    for (index, snapshot) in result.exposures.iter().enumerate() {
        write_knowledge_exposure_snapshot_fields(
            &mut output,
            Some(&format!("exposure[{index}]")),
            snapshot,
        )?;
    }
    Ok(output)
}

fn render_knowledge_space_refresh_source_statuses(
    result: &KnowledgeSpaceRefreshSourceStatusesResult,
) -> Result<String> {
    let mut output = format!(
        "knowledge_space_id={}\nrefreshed_exposures={}\ncurrent={}\nstale={}\nunknown={}\nunresolved={}\n",
        result.knowledge_space_id,
        result.refreshed_exposures.len(),
        result.current_count,
        result.stale_count,
        result.unknown_count,
        result.unresolved_count
    );
    for (index, snapshot) in result.refreshed_exposures.iter().enumerate() {
        write_knowledge_exposure_snapshot_fields(
            &mut output,
            Some(&format!("exposure[{index}]")),
            snapshot,
        )?;
    }
    Ok(output)
}

fn write_knowledge_space_snapshot_fields(
    output: &mut String,
    prefix: Option<&str>,
    snapshot: &KnowledgeSpaceSnapshot,
) {
    let key = |name: &str| {
        prefix
            .map(|prefix| format!("{prefix}.{name}"))
            .unwrap_or_else(|| name.to_owned())
    };
    writeln!(
        output,
        "{}={}",
        key("knowledge_space_id"),
        snapshot.knowledge_space_id
    )
    .expect("write to String");
    writeln!(output, "{}={}", key("name"), snapshot.name).expect("write to String");
    writeln!(
        output,
        "{}={}",
        key("created_at_us"),
        snapshot.created_at_us
    )
    .expect("write to String");
}

fn render_knowledge_exposure_create_result(
    result: &KnowledgeExposureCreateResult,
) -> Result<String> {
    render_knowledge_exposure_snapshot(&result.exposure)
}

fn render_knowledge_exposure_withdraw_result(
    result: &KnowledgeExposureWithdrawResult,
) -> Result<String> {
    render_knowledge_exposure_snapshot(&result.exposure)
}

fn render_knowledge_exposure_refresh_source_status_result(
    result: &KnowledgeExposureRefreshSourceStatusResult,
) -> Result<String> {
    render_knowledge_exposure_snapshot(&result.exposure)
}

fn render_knowledge_exposure_adoption_candidate(
    result: &KnowledgeExposureAdoptionCandidateResult,
) -> Result<String> {
    let candidate = &result.candidate;
    let statement_json = knowledge_statement_json(&candidate.source_knowledge_state.statement)?;
    let scope_json = knowledge_value_json(
        "knowledge exposure adoption candidate scope",
        &candidate.source_knowledge_state.scope,
    )?;
    let provenance_json = knowledge_value_json(
        "knowledge exposure adoption candidate provenance",
        &candidate.source_knowledge_state.provenance,
    )?;
    let adoption_provenance_json = knowledge_value_json(
        "knowledge exposure adoption provenance",
        &candidate.adoption_provenance,
    )?;
    Ok(format!(
        "exposure_id={}\nknowledge_space_id={}\nsource_store_id={}\nsource_workspace_id={}\nsource_knowledge_entity_id={}\nsource_knowledge_entity_version_id={}\nsource_knowledge_state_digest={}\nsource_status={}\nknowledge_status={}\nknowledge_statement_json={}\nknowledge_scope_json={}\nknowledge_provenance_json={}\nadoption_provenance_json={}\n",
        candidate.exposure.exposure_id,
        candidate.exposure.knowledge_space_id,
        candidate.source_store_id,
        candidate.exposure.source.workspace_id,
        candidate.exposure.source.knowledge_entity_id,
        candidate.exposure.source.knowledge_entity_version_id,
        candidate.source_knowledge_state_digest,
        candidate.exposure.source_status.source_status,
        candidate.source_knowledge_state.status,
        statement_json,
        scope_json,
        provenance_json,
        adoption_provenance_json
    ))
}

fn render_knowledge_exposure_adopt(result: &KnowledgeExposureAdoptResult) -> Result<String> {
    let candidate = &result.candidate;
    let knowledge = &result.knowledge;
    let relation = &result.relation;
    let adopted_provenance_json = knowledge_value_json(
        "knowledge exposure adopted knowledge provenance",
        &knowledge.state.provenance,
    )?;
    Ok(format!(
        "exposure_id={}\nknowledge_space_id={}\nsource_status={}\nsource_knowledge_entity_id={}\nsource_knowledge_entity_version_id={}\nsource_knowledge_state_digest={}\nadopted_knowledge_entity_id={}\nadopted_knowledge_entity_version_id={}\nadopted_knowledge_commit_id={}\nadopted_knowledge_state_digest={}\nadopted_knowledge_provenance_json={}\nrelation_id={}\nrelation_version_id={}\nrelation_type={}\nrelation_commit_id={}\nfinal_head_commit_id={}\nwork_state_digest={}\n",
        candidate.exposure.exposure_id,
        candidate.exposure.knowledge_space_id,
        candidate.exposure.source_status.source_status,
        candidate.exposure.source.knowledge_entity_id,
        candidate.exposure.source.knowledge_entity_version_id,
        candidate.source_knowledge_state_digest,
        knowledge.knowledge_entity_id,
        knowledge.knowledge_entity_version_id,
        knowledge.commit_id,
        knowledge.knowledge_state_digest,
        adopted_provenance_json,
        relation.relation_id,
        relation.relation_version_id,
        relation.relation_type,
        relation.commit_id,
        relation.commit_id,
        relation.work_state_digest
    ))
}

fn render_knowledge_exposure_derived_from_relation_create(
    relation: &KnowledgeExposureDerivedFromRelationCreateCommit,
) -> String {
    format!(
        "workspace_id={}\nbranch_id={}\nprevious_head_commit_id={}\ncommit_id={}\nchangeset_id={}\noperation_id={}\nrelation_id={}\nrelation_version_id={}\nrelation_type={}\nknowledge_entity_id={}\nexposure_id={}\nrelation_state_digest={}\nwork_state_digest={}\n",
        relation.workspace_id,
        relation.branch_id,
        relation.previous_head_commit_id,
        relation.commit_id,
        relation.changeset_id,
        relation.operation_id,
        relation.relation_id,
        relation.relation_version_id,
        relation.relation_type,
        relation.knowledge_entity_id,
        relation.exposure_id,
        relation.relation_state_digest,
        relation.work_state_digest
    )
}

fn render_knowledge_exposure_snapshot(snapshot: &KnowledgeExposureSnapshot) -> Result<String> {
    let mut output = String::new();
    write_knowledge_exposure_snapshot_fields(&mut output, None, snapshot)?;
    Ok(output)
}

fn render_knowledge_exposure_list(result: &KnowledgeExposureListResult) -> Result<String> {
    let mut output = format!("exposures={}\n", result.exposures.len());
    for (index, snapshot) in result.exposures.iter().enumerate() {
        write_knowledge_exposure_snapshot_fields(
            &mut output,
            Some(&format!("exposure[{index}]")),
            snapshot,
        )?;
    }
    Ok(output)
}

fn write_knowledge_exposure_snapshot_fields(
    output: &mut String,
    prefix: Option<&str>,
    snapshot: &KnowledgeExposureSnapshot,
) -> Result<()> {
    let key = |name: &str| {
        prefix
            .map(|prefix| format!("{prefix}.{name}"))
            .unwrap_or_else(|| name.to_owned())
    };
    writeln!(output, "{}={}", key("exposure_id"), snapshot.exposure_id).expect("write to String");
    writeln!(
        output,
        "{}={}",
        key("knowledge_space_id"),
        snapshot.knowledge_space_id
    )
    .expect("write to String");
    writeln!(
        output,
        "{}={}",
        key("created_at_us"),
        snapshot.created_at_us
    )
    .expect("write to String");
    writeln!(
        output,
        "{}={}",
        key("lifecycle_status"),
        snapshot.lifecycle_status.as_str()
    )
    .expect("write to String");
    writeln!(
        output,
        "{}={}",
        key("transition_id"),
        snapshot.transition_id
    )
    .expect("write to String");
    writeln!(
        output,
        "{}={}",
        key("previous_transition_id"),
        render_optional_display_or_none(snapshot.previous_transition_id.as_ref())
    )
    .expect("write to String");
    writeln!(
        output,
        "{}={}",
        key("changed_at_us"),
        snapshot.changed_at_us
    )
    .expect("write to String");
    writeln!(
        output,
        "{}={}",
        key("transition_detail_json"),
        canonical_cli_json(
            "knowledge exposure transition detail",
            &snapshot.transition_detail
        )?
    )
    .expect("write to String");
    writeln!(
        output,
        "{}={}",
        key("transition_detail_digest"),
        snapshot.transition_detail_digest
    )
    .expect("write to String");
    writeln!(
        output,
        "{}={}",
        key("transition_detail_size_bytes"),
        snapshot.transition_detail_size_bytes
    )
    .expect("write to String");
    writeln!(output, "{}=local", key("source_kind")).expect("write to String");
    writeln!(
        output,
        "{}={}",
        key("source_workspace_id"),
        snapshot.source.workspace_id
    )
    .expect("write to String");
    writeln!(
        output,
        "{}={}",
        key("source_knowledge_entity_id"),
        snapshot.source.knowledge_entity_id
    )
    .expect("write to String");
    writeln!(
        output,
        "{}={}",
        key("source_knowledge_entity_version_id"),
        snapshot.source.knowledge_entity_version_id
    )
    .expect("write to String");
    writeln!(
        output,
        "{}={}",
        key("source_knowledge_state_digest"),
        snapshot.source.knowledge_state_digest
    )
    .expect("write to String");
    writeln!(
        output,
        "{}={}",
        key("source_status"),
        snapshot.source_status.source_status.as_str()
    )
    .expect("write to String");
    writeln!(
        output,
        "{}={}",
        key("source_status_checked_at_us"),
        snapshot.source_status.checked_at_us
    )
    .expect("write to String");
    writeln!(
        output,
        "{}={}",
        key("source_status_detail_json"),
        canonical_cli_json(
            "knowledge exposure source status detail",
            &snapshot.source_status.detail
        )?
    )
    .expect("write to String");
    writeln!(
        output,
        "{}={}",
        key("source_status_detail_digest"),
        snapshot.source_status.detail_digest
    )
    .expect("write to String");
    writeln!(
        output,
        "{}={}",
        key("source_status_detail_size_bytes"),
        snapshot.source_status.detail_size_bytes
    )
    .expect("write to String");
    Ok(())
}

fn render_external_object_ref_record_result(
    result: &ExternalObjectRefRecordResult,
) -> Result<String> {
    let mut output = format!("created={}\n", result.created);
    write_external_object_ref_snapshot_fields(&mut output, None, &result.external_ref)?;
    Ok(output)
}

fn render_external_object_ref_snapshot(snapshot: &ExternalObjectRefSnapshot) -> Result<String> {
    let mut output = String::new();
    write_external_object_ref_snapshot_fields(&mut output, None, snapshot)?;
    Ok(output)
}

fn render_external_object_ref_list(result: &ExternalObjectRefListResult) -> Result<String> {
    let mut output = format!("external_refs={}\n", result.external_refs.len());
    for (index, snapshot) in result.external_refs.iter().enumerate() {
        write_external_object_ref_snapshot_fields(
            &mut output,
            Some(&format!("external_ref[{index}]")),
            snapshot,
        )?;
    }
    Ok(output)
}

fn write_external_object_ref_snapshot_fields(
    output: &mut String,
    prefix: Option<&str>,
    snapshot: &ExternalObjectRefSnapshot,
) -> Result<()> {
    let key = |name: &str| {
        prefix
            .map(|prefix| format!("{prefix}.{name}"))
            .unwrap_or_else(|| name.to_owned())
    };
    writeln!(
        output,
        "{}={}",
        key("external_ref_id"),
        snapshot.external_ref_id
    )
    .expect("write to String");
    writeln!(
        output,
        "{}={}",
        key("external_store_id"),
        snapshot.external_store_id
    )
    .expect("write to String");
    writeln!(
        output,
        "{}={}",
        key("external_object_id"),
        snapshot.external_object_id
    )
    .expect("write to String");
    writeln!(output, "{}={}", key("object_kind"), snapshot.object_kind).expect("write to String");
    writeln!(
        output,
        "{}={}",
        key("reference_scope"),
        snapshot.reference_scope.as_str()
    )
    .expect("write to String");
    writeln!(
        output,
        "{}={}",
        key("external_version_ref"),
        render_optional_display_or_none(snapshot.external_version_ref.as_ref())
    )
    .expect("write to String");
    writeln!(
        output,
        "{}={}",
        key("descriptor_json"),
        canonical_cli_json("external object ref descriptor", &snapshot.descriptor)?
    )
    .expect("write to String");
    writeln!(
        output,
        "{}={}",
        key("descriptor_digest"),
        snapshot.descriptor_digest
    )
    .expect("write to String");
    writeln!(
        output,
        "{}={}",
        key("descriptor_size_bytes"),
        snapshot.descriptor_size_bytes
    )
    .expect("write to String");
    writeln!(
        output,
        "{}={}",
        key("created_at_us"),
        snapshot.created_at_us
    )
    .expect("write to String");
    Ok(())
}

fn render_store_migration_record_result(result: &StoreMigrationRecordResult) -> Result<String> {
    render_store_migration_snapshot(&result.migration)
}

fn render_store_migration_snapshot(snapshot: &StoreMigrationAttemptSnapshot) -> Result<String> {
    let mut output = String::new();
    write_store_migration_snapshot_fields(&mut output, None, snapshot)?;
    Ok(output)
}

fn render_store_migration_list(result: &StoreMigrationListResult) -> Result<String> {
    let mut output = format!("migrations={}\n", result.migrations.len());
    for (index, snapshot) in result.migrations.iter().enumerate() {
        write_store_migration_snapshot_fields(
            &mut output,
            Some(&format!("migration[{index}]")),
            snapshot,
        )?;
    }
    Ok(output)
}

fn write_store_migration_snapshot_fields(
    output: &mut String,
    prefix: Option<&str>,
    snapshot: &StoreMigrationAttemptSnapshot,
) -> Result<()> {
    let key = |name: &str| {
        prefix
            .map(|prefix| format!("{prefix}.{name}"))
            .unwrap_or_else(|| name.to_owned())
    };
    let (outcome, completed_at_us, detail_json, detail_digest, detail_size_bytes) =
        if let Some(outcome) = &snapshot.outcome {
            (
                outcome.outcome.clone(),
                outcome.completed_at_us.to_string(),
                canonical_cli_json("store migration detail", &outcome.detail)?,
                outcome.detail_digest.to_string(),
                outcome.detail_size_bytes.to_string(),
            )
        } else {
            (
                "none".to_owned(),
                "none".to_owned(),
                "none".to_owned(),
                "none".to_owned(),
                "none".to_owned(),
            )
        };
    writeln!(output, "{}={}", key("migration_id"), snapshot.migration_id).expect("write to String");
    writeln!(
        output,
        "{}={}",
        key("from_store_format_version"),
        snapshot.from_store_format_version
    )
    .expect("write to String");
    writeln!(
        output,
        "{}={}",
        key("to_store_format_version"),
        snapshot.to_store_format_version
    )
    .expect("write to String");
    writeln!(
        output,
        "{}={}",
        key("from_schema_version"),
        snapshot.from_schema_version
    )
    .expect("write to String");
    writeln!(
        output,
        "{}={}",
        key("to_schema_version"),
        snapshot.to_schema_version
    )
    .expect("write to String");
    writeln!(output, "{}={}", key("tool_version"), snapshot.tool_version).expect("write to String");
    writeln!(
        output,
        "{}={}",
        key("started_at_us"),
        snapshot.started_at_us
    )
    .expect("write to String");
    writeln!(output, "{}={outcome}", key("outcome")).expect("write to String");
    writeln!(output, "{}={completed_at_us}", key("completed_at_us")).expect("write to String");
    writeln!(output, "{}={detail_json}", key("detail_json")).expect("write to String");
    writeln!(output, "{}={detail_digest}", key("detail_digest")).expect("write to String");
    writeln!(output, "{}={detail_size_bytes}", key("detail_size_bytes")).expect("write to String");
    Ok(())
}

fn render_bundle_manifest_validation(result: &BundleManifestValidationResult) -> String {
    format!(
        "commit_id={}\nvalid={}\nexpected_manifest_digest={}\nactual_manifest_digest={}\nactual_manifest_size_bytes={}\nproblem={}\n",
        result.commit_id,
        result.valid,
        result.expected_manifest_digest,
        result.actual_manifest_digest,
        result.actual_manifest_size_bytes,
        result.problem.as_deref().unwrap_or("none")
    )
}

fn render_checkpoint_create(result: &CheckpointCreateResult) -> String {
    let mut output = render_checkpoint_snapshot(&result.checkpoint);
    writeln!(output, "entity_count={}", result.entity_count).expect("write to String");
    writeln!(output, "relation_count={}", result.relation_count).expect("write to String");
    output
}

fn render_checkpoint_snapshot(snapshot: &CheckpointSnapshot) -> String {
    format!(
        "checkpoint_id={}\nworkspace_id={}\ncommit_id={}\nstate_digest={}\ncheckpoint_format_version={}\ncontent_digest={}\ncontent_size_bytes={}\nmedia_type={}\ncreated_at_us={}\nusability_state={}\nlast_validated_at_us={}\n",
        snapshot.checkpoint_id,
        snapshot.workspace_id,
        snapshot.commit_id,
        snapshot.state_digest,
        snapshot.checkpoint_format_version,
        snapshot.content_digest,
        snapshot.content_size_bytes,
        snapshot.media_type.as_deref().unwrap_or("none"),
        snapshot.created_at_us,
        snapshot.usability_state,
        snapshot.last_validated_at_us
    )
}

fn render_checkpoint_validation(result: &CheckpointValidationResult) -> String {
    let mut output = render_checkpoint_snapshot(&result.checkpoint);
    writeln!(output, "valid={}", result.valid).expect("write to String");
    writeln!(
        output,
        "expected_state_digest={}",
        result.expected_state_digest
    )
    .expect("write to String");
    writeln!(
        output,
        "expected_content_digest={}",
        result.expected_content_digest
    )
    .expect("write to String");
    writeln!(
        output,
        "expected_content_size_bytes={}",
        result.expected_content_size_bytes
    )
    .expect("write to String");
    writeln!(
        output,
        "problem={}",
        result.problem.as_deref().unwrap_or("none")
    )
    .expect("write to String");
    output
}

fn render_checkpoint_list(result: &CheckpointListResult) -> String {
    let mut output = format!(
        "commit_id={}\ncheckpoints={}\n",
        result.commit_id,
        result.checkpoints.len()
    );
    for (index, checkpoint) in result.checkpoints.iter().enumerate() {
        writeln!(
            output,
            "checkpoint[{index}].id={}",
            checkpoint.checkpoint_id
        )
        .expect("write to String");
        writeln!(
            output,
            "checkpoint[{index}].content_digest={}",
            checkpoint.content_digest
        )
        .expect("write to String");
        writeln!(
            output,
            "checkpoint[{index}].usability_state={}",
            checkpoint.usability_state
        )
        .expect("write to String");
    }
    output
}

fn render_checkpoint_latest(result: &CheckpointLatestResult) -> String {
    let mut output = format!(
        "commit_id={}\ncheckpoint_found={}\n",
        result.commit_id,
        result.checkpoint.is_some()
    );
    if let Some(checkpoint) = &result.checkpoint {
        writeln!(output, "checkpoint_id={}", checkpoint.checkpoint_id).expect("write to String");
        writeln!(output, "content_digest={}", checkpoint.content_digest).expect("write to String");
        writeln!(output, "usability_state={}", checkpoint.usability_state)
            .expect("write to String");
    }
    output
}

fn render_why(result: &WhyQueryResult) -> String {
    let mut output = String::new();
    match result.target.target {
        WhyQueryTarget::BranchHead(branch_id) => {
            writeln!(output, "target_kind=branch_head").expect("write to String");
            writeln!(output, "target_branch_id={branch_id}").expect("write to String");
        }
        WhyQueryTarget::Commit(commit_id) => {
            writeln!(output, "target_kind=commit").expect("write to String");
            writeln!(output, "target_commit_id={commit_id}").expect("write to String");
        }
    }
    writeln!(output, "workspace_id={}", result.target.workspace_id).expect("write to String");
    writeln!(output, "commit_id={}", result.target.commit_id).expect("write to String");
    writeln!(output, "state_digest={}", result.target.state_digest).expect("write to String");
    match result.subject {
        ResolvedWhyQuerySubject::Entity {
            entity_id,
            entity_version_id,
            entity_kind,
        } => {
            writeln!(output, "subject_kind=entity").expect("write to String");
            writeln!(output, "subject_entity_id={entity_id}").expect("write to String");
            writeln!(
                output,
                "subject_entity_kind={}",
                why_entity_kind(entity_kind)
            )
            .expect("write to String");
            writeln!(output, "subject_entity_version_id={entity_version_id}")
                .expect("write to String");
        }
        ResolvedWhyQuerySubject::Evidence { evidence_id } => {
            writeln!(output, "subject_kind=evidence").expect("write to String");
            writeln!(output, "subject_evidence_id={evidence_id}").expect("write to String");
        }
        ResolvedWhyQuerySubject::KnowledgeExposure { exposure_id } => {
            writeln!(output, "subject_kind=knowledge_exposure").expect("write to String");
            writeln!(output, "subject_exposure_id={exposure_id}").expect("write to String");
        }
    }
    writeln!(output, "relation_edges={}", result.relation_edges.len()).expect("write to String");
    for (index, edge) in result.relation_edges.iter().enumerate() {
        writeln!(
            output,
            "relation.{index}.relation_kind={}",
            why_relation_kind(edge.relation_kind)
        )
        .expect("write to String");
        writeln!(
            output,
            "relation.{index}.direction={}",
            why_relation_direction(edge.direction)
        )
        .expect("write to String");
        writeln!(output, "relation.{index}.relation_id={}", edge.relation_id)
            .expect("write to String");
        writeln!(
            output,
            "relation.{index}.relation_version_id={}",
            edge.relation_version_id
        )
        .expect("write to String");
        writeln!(
            output,
            "relation.{index}.relation_label={}",
            edge.relation_label.as_deref().unwrap_or("")
        )
        .expect("write to String");
        render_why_endpoint(&mut output, index, "source", edge.source);
        render_why_endpoint(&mut output, index, "target", edge.target);
        writeln!(
            output,
            "relation.{index}.relation_state_digest={}",
            edge.state_digest
        )
        .expect("write to String");
    }
    writeln!(
        output,
        "deferred_relation_families={}",
        result.deferred_relation_families.len()
    )
    .expect("write to String");
    for (index, family) in result.deferred_relation_families.iter().enumerate() {
        writeln!(
            output,
            "deferred_relation_family.{index}={}",
            why_deferred_relation_family(*family)
        )
        .expect("write to String");
    }
    output
}

fn render_why_endpoint(
    output: &mut String,
    index: usize,
    side: &str,
    endpoint: WhyRelationEndpoint,
) {
    match endpoint {
        WhyRelationEndpoint::Entity {
            entity_kind,
            entity_id,
        } => {
            writeln!(output, "relation.{index}.{side}_kind=entity").expect("write to String");
            writeln!(
                output,
                "relation.{index}.{side}_entity_kind={}",
                why_entity_kind(entity_kind)
            )
            .expect("write to String");
            writeln!(output, "relation.{index}.{side}_entity_id={entity_id}")
                .expect("write to String");
        }
        WhyRelationEndpoint::Evidence { evidence_id } => {
            writeln!(output, "relation.{index}.{side}_kind=evidence").expect("write to String");
            writeln!(output, "relation.{index}.{side}_evidence_id={evidence_id}")
                .expect("write to String");
        }
        WhyRelationEndpoint::KnowledgeExposure { exposure_id } => {
            writeln!(output, "relation.{index}.{side}_kind=knowledge_exposure")
                .expect("write to String");
            writeln!(output, "relation.{index}.{side}_exposure_id={exposure_id}")
                .expect("write to String");
        }
    }
}

fn why_relation_kind(kind: WhyRelationKind) -> &'static str {
    match kind {
        WhyRelationKind::PrimaryContainment => "primary_containment",
        WhyRelationKind::StructuralReference => "structural_reference",
        WhyRelationKind::Verifies => "verifies",
        WhyRelationKind::EvidencedBy => "evidenced_by",
        WhyRelationKind::RecordContradicts => "record_contradicts",
        WhyRelationKind::RecordDerivedFrom => "record_derived_from",
        WhyRelationKind::RecordInvalidates => "record_invalidates",
        WhyRelationKind::RecordRelatedTo => "record_related_to",
        WhyRelationKind::RecordSupports => "record_supports",
        WhyRelationKind::RecordSupersedes => "record_supersedes",
        WhyRelationKind::RecordValidates => "record_validates",
        WhyRelationKind::KnowledgeExposureDerivedFrom => "knowledge_exposure_derived_from",
        WhyRelationKind::KnowledgeSupersedes => "knowledge_supersedes",
    }
}

fn why_relation_direction(direction: WhyRelationDirection) -> &'static str {
    match direction {
        WhyRelationDirection::Incoming => "incoming",
        WhyRelationDirection::Outgoing => "outgoing",
    }
}

fn why_entity_kind(kind: WhyEntityKind) -> &'static str {
    match kind {
        WhyEntityKind::Goal => "goal",
        WhyEntityKind::Plan => "plan",
        WhyEntityKind::Task => "task",
        WhyEntityKind::AcceptanceCriterion => "acceptance_criterion",
        WhyEntityKind::VerificationRequirement => "verification_requirement",
        WhyEntityKind::Verification => "verification",
        WhyEntityKind::Record => "record",
        WhyEntityKind::Knowledge => "knowledge",
    }
}

fn why_deferred_relation_family(family: WhyDeferredRelationFamily) -> &'static str {
    match family {
        WhyDeferredRelationFamily::Evolution => "evolution",
        WhyDeferredRelationFamily::Epistemic => "epistemic",
    }
}

fn render_work_state(output: &mut String, state: &WorkState) {
    let _ = writeln!(output, "entities={}", state.entities().len());
    for (entity_id, entity_version_id) in state.entities() {
        let _ = writeln!(output, "entity={} version={}", entity_id, entity_version_id);
    }
    let _ = writeln!(output, "relations={}", state.relations().len());
    for (relation_id, relation_version_id) in state.relations() {
        let _ = writeln!(
            output,
            "relation={} version={}",
            relation_id, relation_version_id
        );
    }
}

fn work_state_diff_target_from_cli(
    label: &str,
    branch: Option<String>,
    commit: Option<String>,
) -> Result<WorkStateDiffTarget> {
    match (branch, commit) {
        (Some(branch), None) => Ok(WorkStateDiffTarget::branch_head(BranchId::parse_canonical(
            &branch,
        )?)),
        (None, Some(commit)) => Ok(WorkStateDiffTarget::commit(CommitId::parse_canonical(
            &commit,
        )?)),
        _ => Err(WorkVcsError::QueryInvalid(format!(
            "{label} target requires exactly one branch or commit selector"
        ))),
    }
}

fn resolve_show_at_commit(
    engine: &Engine,
    branch: Option<String>,
    commit: Option<String>,
) -> Result<CommitId> {
    match (branch, commit) {
        (Some(branch), None) => Ok(engine
            .branch_head(BranchId::parse_canonical(&branch)?)
            .map(|head| head.head_commit_id)?),
        (None, Some(commit)) => CommitId::parse_canonical(&commit),
        _ => Err(WorkVcsError::QueryInvalid(
            "show-at target requires exactly one of --branch or --commit".to_owned(),
        )),
    }
}

fn filter_work_state_diff_target_kind(diff: &mut WorkStateDiff, value: &str) -> Result<()> {
    match value {
        "entity" => diff.relation_changes.clear(),
        "relation" => diff.entity_changes.clear(),
        other => {
            return Err(WorkVcsError::QueryInvalid(format!(
                "diff target kind {other:?} is not in the CLI vocabulary"
            )));
        }
    }
    Ok(())
}

fn parse_work_state_diff_change_kind(value: &str) -> Result<WorkStateDiffChangeKind> {
    match value {
        "added" => Ok(WorkStateDiffChangeKind::Added),
        "removed" => Ok(WorkStateDiffChangeKind::Removed),
        "updated" => Ok(WorkStateDiffChangeKind::Updated),
        other => Err(WorkVcsError::QueryInvalid(format!(
            "diff change kind {other:?} is not in the CLI vocabulary"
        ))),
    }
}

fn resolve_reference_query_commit(
    engine: &Engine,
    branch: Option<String>,
    commit: Option<String>,
) -> Result<CommitId> {
    match (branch, commit) {
        (Some(branch), None) => Ok(engine
            .branch_head(BranchId::parse_canonical(&branch)?)
            .map(|head| head.head_commit_id)?),
        (None, Some(commit)) => CommitId::parse_canonical(&commit),
        _ => Err(WorkVcsError::QueryInvalid(
            "structural reference query target requires exactly one of --branch or --commit"
                .to_owned(),
        )),
    }
}

fn render_work_state_diff(diff: &WorkStateDiff) -> String {
    let mut output = String::new();
    render_work_state_diff_target(&mut output, "from", &diff.from);
    render_work_state_diff_target(&mut output, "to", &diff.to);
    let _ = writeln!(output, "entity_changes={}", diff.entity_changes.len());
    for (index, change) in diff.entity_changes.iter().enumerate() {
        let prefix = format!("entity[{index}]");
        let _ = writeln!(output, "{prefix}.entity_id={}", change.entity_id);
        let _ = writeln!(
            output,
            "{prefix}.change_kind={}",
            render_work_state_diff_change_kind(change.change_kind)
        );
        let _ = writeln!(
            output,
            "{prefix}.before_entity_version_id={}",
            render_optional_display_or_none(change.before_entity_version_id.as_ref())
        );
        let _ = writeln!(
            output,
            "{prefix}.after_entity_version_id={}",
            render_optional_display_or_none(change.after_entity_version_id.as_ref())
        );
    }
    let _ = writeln!(output, "relation_changes={}", diff.relation_changes.len());
    for (index, change) in diff.relation_changes.iter().enumerate() {
        let prefix = format!("relation[{index}]");
        let _ = writeln!(output, "{prefix}.relation_id={}", change.relation_id);
        let _ = writeln!(
            output,
            "{prefix}.change_kind={}",
            render_work_state_diff_change_kind(change.change_kind)
        );
        let _ = writeln!(
            output,
            "{prefix}.before_relation_version_id={}",
            render_optional_display_or_none(change.before_relation_version_id.as_ref())
        );
        let _ = writeln!(
            output,
            "{prefix}.after_relation_version_id={}",
            render_optional_display_or_none(change.after_relation_version_id.as_ref())
        );
    }
    output
}

fn render_work_state_diff_target(
    output: &mut String,
    prefix: &str,
    target: &workvcs_core::ResolvedWorkStateDiffTarget,
) {
    let (target_kind, target_id) = match target.target {
        WorkStateDiffTarget::BranchHead(branch_id) => ("branch-head", branch_id.to_string()),
        WorkStateDiffTarget::Commit(commit_id) => ("commit", commit_id.to_string()),
    };
    let _ = writeln!(output, "{prefix}_target={target_kind}");
    let _ = writeln!(output, "{prefix}_target_id={target_id}");
    let _ = writeln!(output, "{prefix}_workspace_id={}", target.workspace_id);
    let _ = writeln!(output, "{prefix}_commit_id={}", target.commit_id);
    let _ = writeln!(output, "{prefix}_state_digest={}", target.state_digest);
}

fn render_work_state_diff_change_kind(kind: WorkStateDiffChangeKind) -> &'static str {
    match kind {
        WorkStateDiffChangeKind::Added => "added",
        WorkStateDiffChangeKind::Removed => "removed",
        WorkStateDiffChangeKind::Updated => "updated",
    }
}

fn render_entity_transition_commit(commit: &EntityTransitionCommit) -> String {
    format!(
        "workspace_id={}\nbranch_id={}\nprevious_head_commit_id={}\ncommit_id={}\nchangeset_id={}\noperation_id={}\nentity_id={}\nentity_version_id={}\nentity_state_digest={}\nwork_state_digest={}\n",
        commit.workspace_id,
        commit.branch_id,
        commit.previous_head_commit_id,
        commit.commit_id,
        commit.changeset_id,
        commit.operation_id,
        commit.entity_id,
        commit.entity_version_id,
        commit.entity_state_digest,
        commit.work_state_digest
    )
}

fn render_structural_reference_create(reference: &StructuralReferenceCreateCommit) -> String {
    format!(
        "workspace_id={}\nbranch_id={}\nprevious_head_commit_id={}\ncommit_id={}\nchangeset_id={}\noperation_id={}\nrelation_id={}\nrelation_version_id={}\nreferrer_entity_id={}\nreferrer_kind={}\ntarget_entity_id={}\ntarget_kind={}\nrelation_state_digest={}\nwork_state_digest={}\n",
        reference.workspace_id,
        reference.branch_id,
        reference.previous_head_commit_id,
        reference.commit_id,
        reference.changeset_id,
        reference.operation_id,
        reference.relation_id,
        reference.relation_version_id,
        reference.referrer_entity_id,
        reference.referrer_kind,
        reference.target_entity_id,
        reference.target_kind,
        reference.relation_state_digest,
        reference.work_state_digest
    )
}

fn render_structural_reference_list(references: &[StructuralReferenceSnapshot]) -> String {
    let mut output = format!("structural_references={}\n", references.len());
    for (index, reference) in references.iter().enumerate() {
        let prefix = format!("reference[{index}]");
        let _ = writeln!(output, "{prefix}.workspace_id={}", reference.workspace_id);
        let _ = writeln!(output, "{prefix}.commit_id={}", reference.commit_id);
        let _ = writeln!(output, "{prefix}.relation_id={}", reference.relation_id);
        let _ = writeln!(
            output,
            "{prefix}.relation_version_id={}",
            reference.relation_version_id
        );
        let _ = writeln!(
            output,
            "{prefix}.referrer_entity_id={}",
            reference.referrer_entity_id
        );
        let _ = writeln!(output, "{prefix}.referrer_kind={}", reference.referrer_kind);
        let _ = writeln!(
            output,
            "{prefix}.target_entity_id={}",
            reference.target_entity_id
        );
        let _ = writeln!(output, "{prefix}.target_kind={}", reference.target_kind);
        let _ = writeln!(output, "{prefix}.state_digest={}", reference.state_digest);
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn cli_exposes_thin_command_shells() {
        let command = Cli::command();
        let names = command
            .get_subcommands()
            .map(|command| command.get_name())
            .collect::<Vec<_>>();
        assert_eq!(
            names,
            vec![
                "init",
                "doctor",
                "canonical",
                "id",
                "store",
                "history",
                "changeset",
                "commit",
                "event",
                "show-at",
                "diff",
                "entity",
                "reference",
                "restore",
                "why",
                "workspace",
                "branch",
                "knowledge",
                "goal",
                "plan",
                "task",
                "ac",
                "vr",
                "evidence",
                "resource",
                "record",
                "session",
                "claim",
                "context",
                "next",
                "runnable",
                "verification",
                "projection",
                "bundle",
                "checkpoint",
                "merge"
            ]
        );
    }

    #[test]
    fn history_requires_one_start_selector() {
        let missing = Cli::try_parse_from(["workvcs", "history", "store.sqlite"]);
        assert!(missing.is_err());

        let branch = BranchId::new_v7().to_string();
        let commit = CommitId::new_v7().to_string();
        let both = Cli::try_parse_from([
            "workvcs",
            "history",
            "store.sqlite",
            "--branch",
            &branch,
            "--commit",
            &commit,
        ]);
        assert!(both.is_err());
    }

    #[test]
    fn show_at_requires_one_target_selector() {
        let missing = Cli::try_parse_from(["workvcs", "show-at", "store.sqlite"]);
        assert!(missing.is_err());

        let branch = BranchId::new_v7().to_string();
        let commit = CommitId::new_v7().to_string();
        let both = Cli::try_parse_from([
            "workvcs",
            "show-at",
            "store.sqlite",
            "--branch",
            &branch,
            "--commit",
            &commit,
        ]);
        assert!(both.is_err());
    }

    #[test]
    fn id_cli_generates_typed_uuidv7_values() {
        let entity = run(
            Cli::try_parse_from(["workvcs", "id", "new", "--kind", "entity"])
                .expect("parse entity id"),
        )
        .expect("generate entity id");
        assert_eq!(value(&entity, "kind"), "entity");
        EntityId::parse_canonical(&value(&entity, "id")).expect("entity UUIDv7");

        let relation_version =
            run(
                Cli::try_parse_from(["workvcs", "id", "new", "--kind", "relation-version"])
                    .expect("parse relation version id"),
            )
            .expect("generate relation version id");
        assert_eq!(value(&relation_version, "kind"), "relation-version");
        RelationVersionId::parse_canonical(&value(&relation_version, "id"))
            .expect("relation version UUIDv7");

        let unsupported = run(
            Cli::try_parse_from(["workvcs", "id", "new", "--kind", "unknown"])
                .expect("parse unsupported id kind"),
        );
        assert!(unsupported.is_err());
    }

    #[test]
    fn store_info_cli_shows_manifest_baseline() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let path = tempdir.path().join("workvcs.sqlite");
        let store = path.to_str().expect("path text");
        run(
            Cli::try_parse_from(["workvcs", "init", store, "--display-name", "info-store"])
                .expect("parse init"),
        )
        .expect("init store");

        let info = run(
            Cli::try_parse_from(["workvcs", "store", "info", store]).expect("parse store info")
        )
        .expect("store info");
        StoreId::parse_canonical(&value(&info, "store_id")).expect("store UUIDv7");
        assert_eq!(value(&info, "display_name"), "info-store");
        assert_eq!(value(&info, "schema_version"), "1");
        assert_eq!(value(&info, "id_scheme"), "uuidv7-blob16");
        assert_eq!(value(&info, "digest_algorithm"), "blake3-256");
        assert_eq!(value(&info, "canonical_json_profile"), "workvcs-jcs-v1");
        parse_canonical_json(value(&info, "manifest_json").as_bytes()).expect("manifest JSON");
    }

    #[test]
    fn store_integrity_cli_reports_validation_counts() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let path = tempdir.path().join("workvcs.sqlite");
        let store = path.to_str().expect("path text");
        run(Cli::try_parse_from([
            "workvcs",
            "init",
            store,
            "--display-name",
            "integrity-store",
        ])
        .expect("parse init"))
        .expect("init store");

        let empty = run(
            Cli::try_parse_from(["workvcs", "store", "integrity", store])
                .expect("parse empty integrity"),
        )
        .expect("empty store integrity");
        assert_eq!(value(&empty, "checked_branches"), "0");
        assert_eq!(value(&empty, "checked_commits"), "0");
        assert_eq!(value(&empty, "checked_changesets"), "0");
        assert_eq!(value(&empty, "invalid_checkpoints"), "0");

        let workspace = run(Cli::try_parse_from([
            "workvcs",
            "workspace",
            "create",
            store,
            "--display-name",
            "integrity workspace",
        ])
        .expect("parse workspace"))
        .expect("create workspace");

        let report = run(
            Cli::try_parse_from(["workvcs", "store", "integrity", store]).expect("parse integrity"),
        )
        .expect("store integrity");
        assert_eq!(value(&report, "checked_branches"), "1");
        assert_eq!(value(&report, "checked_commits"), "1");
        assert_eq!(value(&report, "checked_changesets"), "1");
        assert_eq!(value(&report, "checked_change_operations"), "0");
        assert_eq!(value(&report, "checked_changeset_causal_anchors"), "0");
        assert_eq!(value(&report, "checked_events"), "1");
        assert_eq!(value(&report, "checked_checkpoints"), "0");
        assert_eq!(value(&report, "invalid_checkpoints"), "0");
        assert!(
            !value(&workspace, "genesis_commit_id").is_empty(),
            "workspace creation should produce a replayable genesis commit"
        );
    }

    #[test]
    fn diff_requires_one_selector_per_side() {
        let missing_from = Cli::try_parse_from([
            "workvcs",
            "diff",
            "store.sqlite",
            "--to-commit",
            &CommitId::new_v7().to_string(),
        ]);
        assert!(missing_from.is_err());

        let both_from = Cli::try_parse_from([
            "workvcs",
            "diff",
            "store.sqlite",
            "--from-branch",
            &BranchId::new_v7().to_string(),
            "--from-commit",
            &CommitId::new_v7().to_string(),
            "--to-commit",
            &CommitId::new_v7().to_string(),
        ]);
        assert!(both_from.is_err());

        let both_to = Cli::try_parse_from([
            "workvcs",
            "diff",
            "store.sqlite",
            "--from-commit",
            &CommitId::new_v7().to_string(),
            "--to-branch",
            &BranchId::new_v7().to_string(),
            "--to-commit",
            &CommitId::new_v7().to_string(),
        ]);
        assert!(both_to.is_err());

        let both_target_ids = Cli::try_parse_from([
            "workvcs",
            "diff",
            "store.sqlite",
            "--from-commit",
            &CommitId::new_v7().to_string(),
            "--to-commit",
            &CommitId::new_v7().to_string(),
            "--entity",
            &EntityId::new_v7().to_string(),
            "--relation",
            &RelationId::new_v7().to_string(),
        ]);
        assert!(both_target_ids.is_err());
    }

    #[test]
    fn cli_diffs_work_state_between_commit_and_branch_head() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let path = tempdir.path().join("workvcs.sqlite");
        let store = path.to_str().expect("path text");
        run(
            Cli::try_parse_from(["workvcs", "init", store, "--display-name", "diff-store"])
                .expect("parse init"),
        )
        .expect("init store");
        let workspace = run(Cli::try_parse_from([
            "workvcs",
            "workspace",
            "create",
            store,
            "--display-name",
            "workspace",
        ])
        .expect("parse workspace"))
        .expect("create workspace");
        let branch = value(&workspace, "branch_id");
        let genesis = value(&workspace, "genesis_commit_id");
        let task = run(Cli::try_parse_from([
            "workvcs",
            "task",
            "create",
            store,
            "--branch",
            &branch,
            "--head",
            &genesis,
            "--description",
            "Diff target task",
        ])
        .expect("parse task create"))
        .expect("create task");

        let diff = run(Cli::try_parse_from([
            "workvcs",
            "diff",
            store,
            "--from-commit",
            &genesis,
            "--to-branch",
            &branch,
        ])
        .expect("parse diff"))
        .expect("diff work state");
        assert_eq!(value(&diff, "from_target"), "commit");
        assert_eq!(value(&diff, "from_commit_id"), genesis);
        assert_eq!(value(&diff, "to_target"), "branch-head");
        assert_eq!(value(&diff, "to_target_id"), branch);
        assert_eq!(value(&diff, "to_commit_id"), value(&task, "commit_id"));
        assert_eq!(value(&diff, "entity_changes"), "1");
        assert_eq!(value(&diff, "relation_changes"), "0");
        assert_eq!(
            value(&diff, "entity[0].entity_id"),
            value(&task, "task_entity_id")
        );
        assert_eq!(value(&diff, "entity[0].change_kind"), "added");
        assert_eq!(value(&diff, "entity[0].before_entity_version_id"), "none");
        assert_eq!(
            value(&diff, "entity[0].after_entity_version_id"),
            value(&task, "task_entity_version_id")
        );

        let branch_state =
            run(
                Cli::try_parse_from(["workvcs", "show-at", store, "--branch", &branch])
                    .expect("parse branch show-at"),
            )
            .expect("show branch state");
        assert_eq!(value(&branch_state, "commit_id"), value(&task, "commit_id"));
        assert_eq!(value(&branch_state, "entities"), "1");
        assert_eq!(value(&branch_state, "relations"), "0");

        let entity_diff = run(Cli::try_parse_from([
            "workvcs",
            "diff",
            store,
            "--from-commit",
            &genesis,
            "--to-branch",
            &branch,
            "--target-kind",
            "entity",
        ])
        .expect("parse entity diff"))
        .expect("diff entity changes");
        assert_eq!(value(&entity_diff, "entity_changes"), "1");
        assert_eq!(value(&entity_diff, "relation_changes"), "0");

        let relation_diff = run(Cli::try_parse_from([
            "workvcs",
            "diff",
            store,
            "--from-commit",
            &genesis,
            "--to-branch",
            &branch,
            "--target-kind",
            "relation",
        ])
        .expect("parse relation diff"))
        .expect("diff relation changes");
        assert_eq!(value(&relation_diff, "entity_changes"), "0");
        assert_eq!(value(&relation_diff, "relation_changes"), "0");

        let added_diff = run(Cli::try_parse_from([
            "workvcs",
            "diff",
            store,
            "--from-commit",
            &genesis,
            "--to-branch",
            &branch,
            "--change-kind",
            "added",
        ])
        .expect("parse added diff"))
        .expect("diff added changes");
        assert_eq!(value(&added_diff, "entity_changes"), "1");
        assert_eq!(value(&added_diff, "relation_changes"), "0");

        let removed_diff = run(Cli::try_parse_from([
            "workvcs",
            "diff",
            store,
            "--from-commit",
            &genesis,
            "--to-branch",
            &branch,
            "--change-kind",
            "removed",
        ])
        .expect("parse removed diff"))
        .expect("diff removed changes");
        assert_eq!(value(&removed_diff, "entity_changes"), "0");
        assert_eq!(value(&removed_diff, "relation_changes"), "0");

        let entity_id_diff = run(Cli::try_parse_from([
            "workvcs",
            "diff",
            store,
            "--from-commit",
            &genesis,
            "--to-branch",
            &branch,
            "--entity",
            &value(&task, "task_entity_id"),
        ])
        .expect("parse entity id diff"))
        .expect("diff one entity");
        assert_eq!(value(&entity_id_diff, "entity_changes"), "1");
        assert_eq!(value(&entity_id_diff, "relation_changes"), "0");
        assert_eq!(
            value(&entity_id_diff, "entity[0].entity_id"),
            value(&task, "task_entity_id")
        );

        let missing_entity_diff = run(Cli::try_parse_from([
            "workvcs",
            "diff",
            store,
            "--from-commit",
            &genesis,
            "--to-branch",
            &branch,
            "--entity",
            &EntityId::new_v7().to_string(),
        ])
        .expect("parse missing entity diff"))
        .expect("diff missing entity");
        assert_eq!(value(&missing_entity_diff, "entity_changes"), "0");
        assert_eq!(value(&missing_entity_diff, "relation_changes"), "0");

        let successor = run(Cli::try_parse_from([
            "workvcs",
            "task",
            "create",
            store,
            "--branch",
            &branch,
            "--head",
            &value(&task, "commit_id"),
            "--description",
            "Diff relation successor",
        ])
        .expect("parse successor task"))
        .expect("create successor task");
        let before_relation = value(&successor, "commit_id");
        let dependency = run(Cli::try_parse_from([
            "workvcs",
            "task",
            "depends-on",
            store,
            "--branch",
            &branch,
            "--head",
            &before_relation,
            "--task",
            &value(&successor, "task_entity_id"),
            "--depends-on",
            &value(&task, "task_entity_id"),
        ])
        .expect("parse relation create"))
        .expect("create relation");

        let relation_id_diff = run(Cli::try_parse_from([
            "workvcs",
            "diff",
            store,
            "--from-commit",
            &before_relation,
            "--to-branch",
            &branch,
            "--relation",
            &value(&dependency, "relation_id"),
        ])
        .expect("parse relation id diff"))
        .expect("diff one relation");
        assert_eq!(value(&relation_id_diff, "entity_changes"), "0");
        assert_eq!(value(&relation_id_diff, "relation_changes"), "1");
        assert_eq!(
            value(&relation_id_diff, "relation[0].relation_id"),
            value(&dependency, "relation_id")
        );
        assert_eq!(value(&relation_id_diff, "relation[0].change_kind"), "added");

        let missing_relation_diff = run(Cli::try_parse_from([
            "workvcs",
            "diff",
            store,
            "--from-commit",
            &before_relation,
            "--to-branch",
            &branch,
            "--relation",
            &RelationId::new_v7().to_string(),
        ])
        .expect("parse missing relation diff"))
        .expect("diff missing relation");
        assert_eq!(value(&missing_relation_diff, "entity_changes"), "0");
        assert_eq!(value(&missing_relation_diff, "relation_changes"), "0");
    }

    #[test]
    fn cli_commits_generic_entity_transitions() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let path = tempdir.path().join("workvcs.sqlite");
        let store = path.to_str().expect("path text");
        run(
            Cli::try_parse_from(["workvcs", "init", store, "--display-name", "entity-store"])
                .expect("parse init"),
        )
        .expect("init store");
        let workspace = run(Cli::try_parse_from([
            "workvcs",
            "workspace",
            "create",
            store,
            "--display-name",
            "workspace",
        ])
        .expect("parse workspace"))
        .expect("create workspace");
        let branch = value(&workspace, "branch_id");
        let genesis = value(&workspace, "genesis_commit_id");

        let created = run(Cli::try_parse_from([
            "workvcs",
            "entity",
            "create",
            store,
            "--branch",
            &branch,
            "--head",
            &genesis,
            "--kind",
            "generic_record",
            "--state-json",
            r#"{"title":"draft","status":"open"}"#,
            "--rationale-json",
            r#"{"reason":"cli"}"#,
        ])
        .expect("parse entity create"))
        .expect("create generic entity");
        assert_eq!(value(&created, "branch_id"), branch);
        assert_eq!(value(&created, "previous_head_commit_id"), genesis);

        let updated = run(Cli::try_parse_from([
            "workvcs",
            "entity",
            "update",
            store,
            "--branch",
            &branch,
            "--head",
            &value(&created, "commit_id"),
            "--entity",
            &value(&created, "entity_id"),
            "--entity-version",
            &value(&created, "entity_version_id"),
            "--state-json",
            r#"{"status":"done","title":"draft"}"#,
        ])
        .expect("parse entity update"))
        .expect("update generic entity");
        assert_eq!(value(&updated, "entity_id"), value(&created, "entity_id"));
        assert_ne!(
            value(&updated, "entity_version_id"),
            value(&created, "entity_version_id")
        );
        assert_ne!(
            value(&updated, "entity_state_digest"),
            value(&created, "entity_state_digest")
        );

        let diff = run(Cli::try_parse_from([
            "workvcs",
            "diff",
            store,
            "--from-commit",
            &value(&created, "commit_id"),
            "--to-commit",
            &value(&updated, "commit_id"),
        ])
        .expect("parse update diff"))
        .expect("diff update");
        assert_eq!(value(&diff, "entity_changes"), "1");
        assert_eq!(value(&diff, "entity[0].change_kind"), "updated");
        assert_eq!(
            value(&diff, "entity[0].before_entity_version_id"),
            value(&created, "entity_version_id")
        );
        assert_eq!(
            value(&diff, "entity[0].after_entity_version_id"),
            value(&updated, "entity_version_id")
        );
    }

    #[test]
    fn cli_creates_and_lists_structural_references() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let path = tempdir.path().join("workvcs.sqlite");
        let store = path.to_str().expect("path text");
        run(Cli::try_parse_from([
            "workvcs",
            "init",
            store,
            "--display-name",
            "reference-store",
        ])
        .expect("parse init"))
        .expect("init store");
        let workspace = run(Cli::try_parse_from([
            "workvcs",
            "workspace",
            "create",
            store,
            "--display-name",
            "workspace",
        ])
        .expect("parse workspace"))
        .expect("create workspace");
        let branch = value(&workspace, "branch_id");
        let genesis = value(&workspace, "genesis_commit_id");
        let goal = run(Cli::try_parse_from([
            "workvcs",
            "goal",
            "create",
            store,
            "--branch",
            &branch,
            "--head",
            &genesis,
            "--description",
            "Reference goal",
        ])
        .expect("parse goal create"))
        .expect("create goal");
        let plan = run(Cli::try_parse_from([
            "workvcs",
            "plan",
            "create",
            store,
            "--branch",
            &branch,
            "--head",
            &value(&goal, "commit_id"),
            "--description",
            "Reference plan",
            "--strategy",
            "Keep structure explicit",
        ])
        .expect("parse plan create"))
        .expect("create plan");

        let reference = run(Cli::try_parse_from([
            "workvcs",
            "reference",
            "create",
            store,
            "--branch",
            &branch,
            "--head",
            &value(&plan, "commit_id"),
            "--referrer",
            &value(&goal, "goal_entity_id"),
            "--target",
            &value(&plan, "plan_entity_id"),
            "--rationale-json",
            r#"{"reason":"cli"}"#,
        ])
        .expect("parse reference create"))
        .expect("create reference");
        assert_eq!(
            value(&reference, "referrer_entity_id"),
            value(&goal, "goal_entity_id")
        );
        assert_eq!(value(&reference, "referrer_kind"), "goal");
        assert_eq!(
            value(&reference, "target_entity_id"),
            value(&plan, "plan_entity_id")
        );
        assert_eq!(value(&reference, "target_kind"), "plan");

        let listed =
            run(
                Cli::try_parse_from(["workvcs", "reference", "list", store, "--branch", &branch])
                    .expect("parse reference list"),
            )
            .expect("list references");
        assert_eq!(value(&listed, "structural_references"), "1");
        assert_eq!(
            value(&listed, "reference[0].relation_id"),
            value(&reference, "relation_id")
        );
        assert_eq!(
            value(&listed, "reference[0].referrer_entity_id"),
            value(&goal, "goal_entity_id")
        );
        assert_eq!(value(&listed, "reference[0].target_kind"), "plan");

        let listed_by_referrer = run(Cli::try_parse_from([
            "workvcs",
            "reference",
            "list",
            store,
            "--branch",
            &branch,
            "--referrer",
            &value(&goal, "goal_entity_id"),
        ])
        .expect("parse reference list by referrer"))
        .expect("list references by referrer");
        assert_eq!(value(&listed_by_referrer, "structural_references"), "1");

        let listed_by_target = run(Cli::try_parse_from([
            "workvcs",
            "reference",
            "list",
            store,
            "--branch",
            &branch,
            "--target",
            &value(&plan, "plan_entity_id"),
        ])
        .expect("parse reference list by target"))
        .expect("list references by target");
        assert_eq!(value(&listed_by_target, "structural_references"), "1");

        let listed_by_referrer_kind = run(Cli::try_parse_from([
            "workvcs",
            "reference",
            "list",
            store,
            "--branch",
            &branch,
            "--referrer-kind",
            "goal",
        ])
        .expect("parse reference list by referrer kind"))
        .expect("list references by referrer kind");
        assert_eq!(
            value(&listed_by_referrer_kind, "structural_references"),
            "1"
        );

        let listed_by_target_kind = run(Cli::try_parse_from([
            "workvcs",
            "reference",
            "list",
            store,
            "--branch",
            &branch,
            "--target-kind",
            "plan",
        ])
        .expect("parse reference list by target kind"))
        .expect("list references by target kind");
        assert_eq!(value(&listed_by_target_kind, "structural_references"), "1");

        let listed_by_combined_filters = run(Cli::try_parse_from([
            "workvcs",
            "reference",
            "list",
            store,
            "--branch",
            &branch,
            "--referrer-kind",
            "goal",
            "--target-kind",
            "plan",
            "--target",
            &value(&plan, "plan_entity_id"),
        ])
        .expect("parse reference list by combined filters"))
        .expect("list references by combined filters");
        assert_eq!(
            value(&listed_by_combined_filters, "structural_references"),
            "1"
        );

        let listed_by_missing_kind = run(Cli::try_parse_from([
            "workvcs",
            "reference",
            "list",
            store,
            "--branch",
            &branch,
            "--target-kind",
            "task",
        ])
        .expect("parse reference list by missing kind"))
        .expect("list references by missing kind");
        assert_eq!(value(&listed_by_missing_kind, "structural_references"), "0");
    }

    #[test]
    fn canonical_cli_encodes_and_digests_confirmed_profiles() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let json_file = tempdir.path().join("semantic.json");
        fs::write(&json_file, br#"{"b":2,"a":1}"#).expect("write JSON input");
        let content_file = tempdir.path().join("content.bin");
        fs::write(&content_file, [0x00, 0xff]).expect("write content input");
        let json_file_path = json_file.to_str().expect("JSON path text");
        let content_file_path = content_file.to_str().expect("content path text");

        let encoded = run(Cli::try_parse_from([
            "workvcs",
            "canonical",
            "encode",
            "--json",
            r#"{"b":2,"a":1}"#,
        ])
        .expect("parse canonical encode"))
        .expect("canonical encode");
        assert_eq!(value(&encoded, "canonical_json"), r#"{"a":1,"b":2}"#);
        assert_eq!(value(&encoded, "size_bytes"), "13");

        let encoded_from_file = run(Cli::try_parse_from([
            "workvcs",
            "canonical",
            "encode",
            "--json-file",
            json_file_path,
        ])
        .expect("parse canonical encode file"))
        .expect("canonical encode file");
        assert_eq!(
            value(&encoded_from_file, "canonical_json"),
            value(&encoded, "canonical_json")
        );

        let entity_digest = run(Cli::try_parse_from([
            "workvcs",
            "canonical",
            "digest",
            "--domain",
            "entity-version",
            "--json",
            r#"{"b":2,"a":1}"#,
        ])
        .expect("parse entity digest"))
        .expect("entity digest");
        let relation_digest = run(Cli::try_parse_from([
            "workvcs",
            "canonical",
            "digest",
            "--domain",
            "relation-version",
            "--json",
            r#"{"a":1,"b":2}"#,
        ])
        .expect("parse relation digest"))
        .expect("relation digest");
        let file_digest = run(Cli::try_parse_from([
            "workvcs",
            "canonical",
            "digest",
            "--domain",
            "entity-version",
            "--json-file",
            json_file_path,
        ])
        .expect("parse file digest"))
        .expect("file digest");
        assert_eq!(value(&entity_digest, "domain"), "entity-version");
        assert_eq!(
            value(&entity_digest, "canonical_json"),
            value(&relation_digest, "canonical_json")
        );
        assert_eq!(
            value(&file_digest, "digest"),
            value(&entity_digest, "digest")
        );
        assert_ne!(
            value(&entity_digest, "digest"),
            value(&relation_digest, "digest")
        );

        let raw_digest = run(Cli::try_parse_from([
            "workvcs",
            "canonical",
            "content-digest",
            "--content",
            r#"{"b":2,"a":1}"#,
        ])
        .expect("parse raw content digest"))
        .expect("raw content digest");
        let canonical_content_digest = run(Cli::try_parse_from([
            "workvcs",
            "canonical",
            "content-digest",
            "--content",
            r#"{"a":1,"b":2}"#,
        ])
        .expect("parse canonical content digest"))
        .expect("canonical content digest");
        assert_ne!(
            value(&raw_digest, "content_digest"),
            value(&canonical_content_digest, "content_digest")
        );

        let hex_digest = run(Cli::try_parse_from([
            "workvcs",
            "canonical",
            "content-digest",
            "--content-hex",
            "00ff",
        ])
        .expect("parse hex content digest"))
        .expect("hex content digest");
        assert_eq!(value(&hex_digest, "size_bytes"), "2");

        let file_content_digest = run(Cli::try_parse_from([
            "workvcs",
            "canonical",
            "content-digest",
            "--content-file",
            content_file_path,
        ])
        .expect("parse file content digest"))
        .expect("file content digest");
        assert_eq!(
            value(&file_content_digest, "content_digest"),
            value(&hex_digest, "content_digest")
        );
        assert_eq!(value(&file_content_digest, "size_bytes"), "2");

        let both_json_sources = Cli::try_parse_from([
            "workvcs",
            "canonical",
            "encode",
            "--json",
            "{}",
            "--json-file",
            json_file_path,
        ]);
        assert!(both_json_sources.is_err());

        let both_content_sources = Cli::try_parse_from([
            "workvcs",
            "canonical",
            "content-digest",
            "--content",
            "x",
            "--content-file",
            content_file_path,
        ]);
        assert!(both_content_sources.is_err());

        let float =
            run(
                Cli::try_parse_from(["workvcs", "canonical", "encode", "--json", r#"{"n":1.0}"#])
                    .expect("parse float canonical encode"),
            );
        assert!(float.is_err());
    }

    #[test]
    fn canonical_cli_computes_work_state_digest() {
        let e1 = EntityId::new_v7().to_string();
        let ev1 = EntityVersionId::new_v7().to_string();
        let e2 = EntityId::new_v7().to_string();
        let ev2 = EntityVersionId::new_v7().to_string();
        let r1 = RelationId::new_v7().to_string();
        let rv1 = RelationVersionId::new_v7().to_string();
        let first_entity = format!("{e1}={ev1}");
        let second_entity = format!("{e2}={ev2}");
        let relation = format!("{r1}={rv1}");

        let forward = run(Cli::try_parse_from([
            "workvcs",
            "canonical",
            "work-state-digest",
            "--entity",
            &second_entity,
            "--relation",
            &relation,
            "--entity",
            &first_entity,
        ])
        .expect("parse forward work state digest"))
        .expect("forward work state digest");
        let reverse = run(Cli::try_parse_from([
            "workvcs",
            "canonical",
            "work-state-digest",
            "--relation",
            &relation,
            "--entity",
            &first_entity,
            "--entity",
            &second_entity,
        ])
        .expect("parse reverse work state digest"))
        .expect("reverse work state digest");

        assert_eq!(value(&forward, "entities"), "2");
        assert_eq!(value(&forward, "relations"), "1");
        assert_eq!(
            value(&forward, "work_state_digest"),
            value(&reverse, "work_state_digest")
        );

        let duplicate = run(Cli::try_parse_from([
            "workvcs",
            "canonical",
            "work-state-digest",
            "--entity",
            &first_entity,
            "--entity",
            &format!("{e1}={ev2}"),
        ])
        .expect("parse duplicate work state digest"));
        assert!(duplicate.is_err());

        let malformed = run(Cli::try_parse_from([
            "workvcs",
            "canonical",
            "work-state-digest",
            "--relation",
            &r1,
        ])
        .expect("parse malformed work state digest"));
        assert!(malformed.is_err());
    }

    #[test]
    fn event_list_requires_one_target_selector() {
        let missing = Cli::try_parse_from(["workvcs", "event", "list", "store.sqlite"]);
        assert!(missing.is_err());

        let changeset = ChangeSetId::new_v7().to_string();
        let session = SessionId::new_v7().to_string();
        let both = Cli::try_parse_from([
            "workvcs",
            "event",
            "list",
            "store.sqlite",
            "--changeset",
            &changeset,
            "--session",
            &session,
        ]);
        assert!(both.is_err());
    }

    #[test]
    fn init_and_doctor_use_engine_store_boundary() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let path = tempdir.path().join("workvcs.sqlite");

        let init = run(Cli::try_parse_from([
            "workvcs",
            "init",
            path.to_str().expect("path text"),
            "--display-name",
            "cli-store",
        ])
        .expect("parse init"))
        .expect("run init");
        assert!(init.starts_with("initialized store_id="));
        assert!(init.contains("schema_version=1"));

        let doctor =
            run(
                Cli::try_parse_from(["workvcs", "doctor", path.to_str().expect("path text")])
                    .expect("parse doctor"),
            )
            .expect("run doctor");
        assert!(doctor.starts_with("ok store_id="));
        assert!(doctor.contains("canonical_json_profile=workvcs-jcs-v1"));
        assert!(doctor.contains("checked_branches=0"));
        assert!(doctor.contains("checked_commits=0"));
        assert!(doctor.contains("checked_changesets=0"));
        assert!(doctor.contains("checked_change_operations=0"));
        assert!(doctor.contains("checked_changeset_causal_anchors=0"));
        assert!(doctor.contains("checked_events=0"));
        assert!(doctor.contains("checked_checkpoints=0"));
        assert!(doctor.contains("invalid_checkpoints=0"));
    }

    #[test]
    fn cli_creates_shows_and_lists_workspaces() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let path = tempdir.path().join("workvcs.sqlite");
        let store = path.to_str().expect("path text");

        run(Cli::try_parse_from(["workvcs", "init", store]).expect("parse init"))
            .expect("init store");
        let empty = run(Cli::try_parse_from(["workvcs", "workspace", "list", store])
            .expect("parse empty workspace list"))
        .expect("list empty workspaces");
        assert_eq!(value(&empty, "workspaces"), "0");

        let created = run(Cli::try_parse_from([
            "workvcs",
            "workspace",
            "create",
            store,
            "--display-name",
            "workspace alpha",
            "--initial-branch-name",
            "trunk",
        ])
        .expect("parse workspace create"))
        .expect("create workspace");
        let workspace_id = value(&created, "workspace_id");
        assert_eq!(value(&created, "display_name"), "workspace alpha");
        assert_eq!(value(&created, "branch_name"), "trunk");
        assert_ne!(value(&created, "created_at_us"), "");

        let shown = run(Cli::try_parse_from([
            "workvcs",
            "workspace",
            "show",
            store,
            "--workspace",
            &workspace_id,
        ])
        .expect("parse workspace show"))
        .expect("show workspace");
        assert_eq!(value(&shown, "workspace_id"), workspace_id);
        assert_eq!(value(&shown, "display_name"), "workspace alpha");
        assert_eq!(value(&shown, "branch_id"), value(&created, "branch_id"));
        assert_eq!(
            value(&shown, "genesis_commit_id"),
            value(&created, "genesis_commit_id")
        );
        assert_eq!(
            value(&shown, "genesis_changeset_id"),
            value(&created, "genesis_changeset_id")
        );
        assert_eq!(
            value(&shown, "state_digest"),
            value(&created, "state_digest")
        );

        let listed = run(Cli::try_parse_from(["workvcs", "workspace", "list", store])
            .expect("parse workspace list"))
        .expect("list workspaces");
        assert_eq!(value(&listed, "workspaces"), "1");
        assert_eq!(value(&listed, "workspace.0.workspace_id"), workspace_id);
        assert_eq!(
            value(&listed, "workspace.0.display_name"),
            "workspace alpha"
        );
        assert_eq!(
            value(&listed, "workspace.0.branch_id"),
            value(&created, "branch_id")
        );
        assert_eq!(value(&listed, "workspace.0.branch_name"), "trunk");

        let beta = run(Cli::try_parse_from([
            "workvcs",
            "workspace",
            "create",
            store,
            "--display-name",
            "workspace beta",
        ])
        .expect("parse beta workspace create"))
        .expect("create beta workspace");
        let beta_workspace_id = value(&beta, "workspace_id");

        let alpha_filtered = run(Cli::try_parse_from([
            "workvcs",
            "workspace",
            "list",
            store,
            "--display-name",
            "workspace alpha",
        ])
        .expect("parse alpha workspace list"))
        .expect("list alpha workspace");
        assert_eq!(value(&alpha_filtered, "workspaces"), "1");
        assert_eq!(
            value(&alpha_filtered, "workspace.0.workspace_id"),
            workspace_id
        );

        let beta_filtered = run(Cli::try_parse_from([
            "workvcs",
            "workspace",
            "list",
            store,
            "--display-name",
            "workspace beta",
        ])
        .expect("parse beta workspace list"))
        .expect("list beta workspace");
        assert_eq!(value(&beta_filtered, "workspaces"), "1");
        assert_eq!(
            value(&beta_filtered, "workspace.0.workspace_id"),
            beta_workspace_id
        );

        let limited =
            run(
                Cli::try_parse_from(["workvcs", "workspace", "list", store, "--limit", "1"])
                    .expect("parse limited workspace list"),
            )
            .expect("list limited workspaces");
        assert_eq!(value(&limited, "workspaces"), "1");
        assert_ne!(value(&limited, "workspace.0.workspace_id"), "");

        let zero_limit =
            run(
                Cli::try_parse_from(["workvcs", "workspace", "list", store, "--limit", "0"])
                    .expect("parse zero-limit workspace list"),
            );
        assert!(zero_limit.is_err());

        let missing_filtered = run(Cli::try_parse_from([
            "workvcs",
            "workspace",
            "list",
            store,
            "--display-name",
            "workspace missing",
        ])
        .expect("parse missing workspace list"))
        .expect("list missing workspace");
        assert_eq!(value(&missing_filtered, "workspaces"), "0");
    }

    #[test]
    fn cli_shows_and_lists_events_by_changeset() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let path = tempdir.path().join("workvcs.sqlite");
        let store = path.to_str().expect("path text");

        run(Cli::try_parse_from(["workvcs", "init", store]).expect("parse init"))
            .expect("init store");
        let workspace = run(Cli::try_parse_from([
            "workvcs",
            "workspace",
            "create",
            store,
            "--display-name",
            "event-query workspace",
        ])
        .expect("parse workspace create"))
        .expect("create workspace");
        let branch_id = value(&workspace, "branch_id");
        let head = value(&workspace, "genesis_commit_id");

        let task = run(Cli::try_parse_from([
            "workvcs",
            "task",
            "create",
            store,
            "--branch",
            &branch_id,
            "--head",
            &head,
            "--description",
            "cli event query task",
        ])
        .expect("parse task create"))
        .expect("create task");
        let commit_id = value(&task, "commit_id");
        let changeset_id = value(&task, "changeset_id");
        let task_entity_id = value(&task, "task_entity_id");

        let shown_commit =
            run(
                Cli::try_parse_from(["workvcs", "commit", "show", store, "--commit", &commit_id])
                    .expect("parse commit show"),
            )
            .expect("show commit");
        assert_eq!(value(&shown_commit, "commit_id"), commit_id);
        assert_eq!(value(&shown_commit, "changeset_id"), changeset_id);
        assert_eq!(value(&shown_commit, "commit_kind"), "normal");
        assert_eq!(value(&shown_commit, "operation_type"), "entity.transition");
        assert_eq!(value(&shown_commit, "origin_session_id"), "none");
        assert_eq!(value(&shown_commit, "parents"), "1");
        assert_eq!(value(&shown_commit, "parent[0].ordinal"), "0");
        assert_eq!(value(&shown_commit, "parent[0].role"), "primary");
        assert_eq!(value(&shown_commit, "parent[0].commit_id"), head);

        let shown_changeset = run(Cli::try_parse_from([
            "workvcs",
            "changeset",
            "show",
            store,
            "--changeset",
            &changeset_id,
        ])
        .expect("parse changeset show"))
        .expect("show changeset");
        assert_eq!(value(&shown_changeset, "changeset_id"), changeset_id);
        assert_eq!(
            value(&shown_changeset, "operation_type"),
            "entity.transition"
        );
        assert_eq!(value(&shown_changeset, "origin_session_id"), "none");
        assert_eq!(value(&shown_changeset, "change_operations"), "1");
        assert_eq!(value(&shown_changeset, "causal_anchors"), "0");
        assert_eq!(value(&shown_changeset, "events"), "1");
        assert_eq!(value(&shown_changeset, "commits"), "1");
        assert_eq!(value(&shown_changeset, "commit[0].commit_id"), commit_id);
        assert_eq!(value(&shown_changeset, "commit[0].commit_kind"), "normal");
        assert_ne!(value(&shown_changeset, "operation_payload_json"), "");
        assert_ne!(value(&shown_changeset, "rationale_json"), "");

        let operations = run(Cli::try_parse_from([
            "workvcs",
            "changeset",
            "operations",
            store,
            "--changeset",
            &changeset_id,
        ])
        .expect("parse changeset operations"))
        .expect("list changeset operations");
        assert_eq!(value(&operations, "changeset_id"), changeset_id);
        assert_eq!(value(&operations, "operations"), "1");
        assert_eq!(value(&operations, "operation[0].ordinal"), "0");
        assert_eq!(value(&operations, "operation[0].subject_family"), "entity");
        assert_eq!(
            value(&operations, "operation[0].subject_object_id"),
            task_entity_id
        );
        assert_ne!(
            value(&operations, "operation[0].operation_payload_json"),
            ""
        );

        let anchors = run(Cli::try_parse_from([
            "workvcs",
            "changeset",
            "anchors",
            store,
            "--changeset",
            &changeset_id,
        ])
        .expect("parse changeset anchors"))
        .expect("list changeset anchors");
        assert_eq!(value(&anchors, "changeset_id"), changeset_id);
        assert_eq!(value(&anchors, "causal_anchors"), "0");

        let listed = run(Cli::try_parse_from([
            "workvcs",
            "event",
            "list",
            store,
            "--changeset",
            &changeset_id,
        ])
        .expect("parse event list"))
        .expect("list events");
        assert_eq!(value(&listed, "events"), "1");
        assert_eq!(value(&listed, "event[0].changeset_id"), changeset_id);
        assert_eq!(value(&listed, "event[0].event_kind"), "entity.transitioned");
        assert_ne!(value(&listed, "event[0].payload_json"), "");
        let event_id = value(&listed, "event[0].event_id");

        let listed_by_kind = run(Cli::try_parse_from([
            "workvcs",
            "event",
            "list",
            store,
            "--changeset",
            &changeset_id,
            "--kind",
            "entity.transitioned",
        ])
        .expect("parse event list by kind"))
        .expect("list events by kind");
        assert_eq!(value(&listed_by_kind, "events"), "1");
        assert_eq!(value(&listed_by_kind, "event[0].event_id"), event_id);

        let listed_by_missing_kind = run(Cli::try_parse_from([
            "workvcs",
            "event",
            "list",
            store,
            "--changeset",
            &changeset_id,
            "--kind",
            "workspace.created",
        ])
        .expect("parse event list by missing kind"))
        .expect("list events by missing kind");
        assert_eq!(value(&listed_by_missing_kind, "events"), "0");

        let listed_by_kind_with_limit = run(Cli::try_parse_from([
            "workvcs",
            "event",
            "list",
            store,
            "--changeset",
            &changeset_id,
            "--kind",
            "entity.transitioned",
            "--limit",
            "1",
        ])
        .expect("parse event list by kind and limit"))
        .expect("list events by kind and limit");
        assert_eq!(value(&listed_by_kind_with_limit, "events"), "1");

        let history = run(Cli::try_parse_from([
            "workvcs", "history", store, "--branch", &branch_id, "--limit", "1",
        ])
        .expect("parse history"))
        .expect("history");
        assert_eq!(value(&history, "entries"), "1");
        assert!(history.contains(&format!("changeset={changeset_id}")));
        assert!(history.contains("committed_at_us="));
        assert!(history.contains("changeset_created_at_us="));

        let shown =
            run(
                Cli::try_parse_from(["workvcs", "event", "show", store, "--event", &event_id])
                    .expect("parse event show"),
            )
            .expect("show event");
        assert_eq!(value(&shown, "event_id"), event_id);
        assert_eq!(value(&shown, "changeset_id"), changeset_id);
        assert_eq!(
            value(&shown, "payload_digest"),
            value(&listed, "event[0].payload_digest")
        );
    }

    #[test]
    fn cli_records_shows_and_lists_store_lineage() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let source_path = tempdir.path().join("source.sqlite");
        let target_path = tempdir.path().join("target.sqlite");
        let source = source_path.to_str().expect("source path text");
        let target = target_path.to_str().expect("target path text");

        run(
            Cli::try_parse_from(["workvcs", "init", source, "--display-name", "source-store"])
                .expect("parse source init"),
        )
        .expect("init source");
        let source_store_id = Engine::open(source)
            .expect("open source")
            .store_info()
            .expect("source info")
            .store_id
            .to_string();
        run(
            Cli::try_parse_from(["workvcs", "init", target, "--display-name", "target-store"])
                .expect("parse target init"),
        )
        .expect("init target");

        let source_bundle_digest = content_object_digest(b"cli-lineage-bundle").to_string();
        let recorded = run(Cli::try_parse_from([
            "workvcs",
            "store",
            "lineage-record",
            target,
            "--source-store",
            &source_store_id,
            "--derivation-kind",
            "explicit_fork",
            "--source-root-json",
            "{\"path\":\"/tmp/source\",\"profile\":\"workvcs-local-payload-directory-v1\"}",
            "--source-bundle-digest",
            &source_bundle_digest,
        ])
        .expect("parse store lineage-record"))
        .expect("record store lineage");
        assert_eq!(value(&recorded, "source_store_id"), source_store_id);
        assert_eq!(value(&recorded, "derivation_kind"), "explicit_fork");
        assert_eq!(
            value(&recorded, "source_root_descriptor_json"),
            "{\"path\":\"/tmp/source\",\"profile\":\"workvcs-local-payload-directory-v1\"}"
        );
        assert_eq!(
            value(&recorded, "source_bundle_digest"),
            source_bundle_digest
        );
        let lineage_id = value(&recorded, "lineage_id");

        let shown = run(Cli::try_parse_from([
            "workvcs",
            "store",
            "lineage-show",
            target,
            "--lineage",
            &lineage_id,
        ])
        .expect("parse store lineage-show"))
        .expect("show store lineage");
        assert_eq!(value(&shown, "lineage_id"), lineage_id);
        assert_eq!(
            value(&shown, "source_root_descriptor_digest"),
            value(&recorded, "source_root_descriptor_digest")
        );

        let listed = run(Cli::try_parse_from([
            "workvcs",
            "store",
            "lineage-list",
            target,
            "--source-store",
            &source_store_id,
            "--derivation-kind",
            "explicit_fork",
            "--source-bundle-digest",
            &source_bundle_digest,
        ])
        .expect("parse store lineage-list"))
        .expect("list store lineages");
        assert_eq!(value(&listed, "lineages"), "1");
        assert_eq!(value(&listed, "lineage[0].lineage_id"), lineage_id);
        assert_eq!(
            value(&listed, "lineage[0].source_bundle_digest"),
            source_bundle_digest
        );
    }

    #[test]
    fn cli_records_shows_and_lists_store_migration() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let path = tempdir.path().join("workvcs.sqlite");
        let store = path.to_str().expect("path text");

        run(Cli::try_parse_from([
            "workvcs",
            "init",
            store,
            "--display-name",
            "migration-store",
        ])
        .expect("parse init"))
        .expect("init store");

        let recorded = run(Cli::try_parse_from([
            "workvcs",
            "store",
            "migration-record",
            store,
            "--from-store-format-version",
            "1",
            "--to-store-format-version",
            "2",
            "--from-schema-version",
            "1",
            "--to-schema-version",
            "2",
            "--tool-version",
            "workvcs-cli-test/0.1",
            "--outcome",
            "completed",
            "--detail-json",
            "{\"manifest_delta\":{},\"operator\":\"cli-test\"}",
        ])
        .expect("parse store migration-record"))
        .expect("record store migration");
        assert_eq!(value(&recorded, "from_store_format_version"), "1");
        assert_eq!(value(&recorded, "to_store_format_version"), "2");
        assert_eq!(value(&recorded, "tool_version"), "workvcs-cli-test/0.1");
        assert_eq!(value(&recorded, "outcome"), "completed");
        assert_eq!(
            value(&recorded, "detail_json"),
            "{\"manifest_delta\":{},\"operator\":\"cli-test\"}"
        );
        let migration_id = value(&recorded, "migration_id");

        let shown = run(Cli::try_parse_from([
            "workvcs",
            "store",
            "migration-show",
            store,
            "--migration",
            &migration_id,
        ])
        .expect("parse store migration-show"))
        .expect("show store migration");
        assert_eq!(value(&shown, "migration_id"), migration_id);
        assert_eq!(
            value(&shown, "detail_digest"),
            value(&recorded, "detail_digest")
        );

        let listed =
            run(
                Cli::try_parse_from(["workvcs", "store", "migration-list", store, "--limit", "1"])
                    .expect("parse store migration-list"),
            )
            .expect("list store migrations");
        assert_eq!(value(&listed, "migrations"), "1");
        assert_eq!(value(&listed, "migration[0].migration_id"), migration_id);
        assert_eq!(value(&listed, "migration[0].outcome"), "completed");

        let failed_recorded = run(Cli::try_parse_from([
            "workvcs",
            "store",
            "migration-record",
            store,
            "--from-store-format-version",
            "2",
            "--to-store-format-version",
            "3",
            "--from-schema-version",
            "2",
            "--to-schema-version",
            "3",
            "--tool-version",
            "external-migrator/9.0",
            "--outcome",
            "failed",
            "--detail-json",
            "{\"error\":\"incompatible source\"}",
        ])
        .expect("parse failed store migration-record"))
        .expect("record failed store migration");
        let failed_migration_id = value(&failed_recorded, "migration_id");

        let tool_version_filtered = run(Cli::try_parse_from([
            "workvcs",
            "store",
            "migration-list",
            store,
            "--tool-version",
            "workvcs-cli-test/0.1",
            "--limit",
            "1",
        ])
        .expect("parse tool-version store migration-list"))
        .expect("list tool-version store migrations");
        assert_eq!(value(&tool_version_filtered, "migrations"), "1");
        assert_eq!(
            value(&tool_version_filtered, "migration[0].migration_id"),
            migration_id
        );

        let outcome_filtered = run(Cli::try_parse_from([
            "workvcs",
            "store",
            "migration-list",
            store,
            "--outcome",
            "failed",
        ])
        .expect("parse outcome store migration-list"))
        .expect("list outcome store migrations");
        assert_eq!(value(&outcome_filtered, "migrations"), "1");
        assert_eq!(
            value(&outcome_filtered, "migration[0].migration_id"),
            failed_migration_id
        );

        let combined_filtered = run(Cli::try_parse_from([
            "workvcs",
            "store",
            "migration-list",
            store,
            "--tool-version",
            "external-migrator/9.0",
            "--outcome",
            "failed",
        ])
        .expect("parse combined store migration-list"))
        .expect("list combined store migrations");
        assert_eq!(value(&combined_filtered, "migrations"), "1");
        assert_eq!(
            value(&combined_filtered, "migration[0].migration_id"),
            failed_migration_id
        );

        let missing_filtered = run(Cli::try_parse_from([
            "workvcs",
            "store",
            "migration-list",
            store,
            "--outcome",
            "skipped",
        ])
        .expect("parse missing store migration-list"))
        .expect("list missing store migrations");
        assert_eq!(value(&missing_filtered, "migrations"), "0");
    }

    #[test]
    fn cli_records_shows_and_lists_external_object_refs() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let path = tempdir.path().join("workvcs.sqlite");
        let store = path.to_str().expect("path text");
        run(Cli::try_parse_from([
            "workvcs",
            "init",
            store,
            "--display-name",
            "external-ref-store",
        ])
        .expect("parse init"))
        .expect("init store");

        let external_store_id = StoreId::new_v7().to_string();
        let external_object_id = ExternalObjectId::new_v7().to_string();
        let external_version_ref = ExternalVersionId::new_v7().to_string();
        let recorded = run(Cli::try_parse_from([
            "workvcs",
            "store",
            "external-ref-record",
            store,
            "--external-store",
            &external_store_id,
            "--external-object",
            &external_object_id,
            "--object-kind",
            "knowledge",
            "--scope",
            "version",
            "--external-version-ref",
            &external_version_ref,
            "--descriptor-json",
            "{\"origin\":\"bundle\",\"path\":\"knowledge/source\"}",
        ])
        .expect("parse external-ref-record"))
        .expect("record external ref");
        assert_eq!(value(&recorded, "created"), "true");
        assert_eq!(value(&recorded, "external_store_id"), external_store_id);
        assert_eq!(value(&recorded, "external_object_id"), external_object_id);
        assert_eq!(value(&recorded, "reference_scope"), "version");
        assert_eq!(
            value(&recorded, "external_version_ref"),
            external_version_ref
        );
        assert_eq!(
            value(&recorded, "descriptor_json"),
            "{\"origin\":\"bundle\",\"path\":\"knowledge/source\"}"
        );
        let external_ref_id = value(&recorded, "external_ref_id");

        let idempotent = run(Cli::try_parse_from([
            "workvcs",
            "store",
            "external-ref-record",
            store,
            "--external-store",
            &external_store_id,
            "--external-object",
            &external_object_id,
            "--object-kind",
            "knowledge",
            "--scope",
            "version",
            "--external-version-ref",
            &external_version_ref,
            "--descriptor-json",
            "{\"origin\":\"bundle\",\"path\":\"knowledge/source\"}",
        ])
        .expect("parse idempotent external-ref-record"))
        .expect("record idempotent external ref");
        assert_eq!(value(&idempotent, "created"), "false");
        assert_eq!(value(&idempotent, "external_ref_id"), external_ref_id);

        let shown = run(Cli::try_parse_from([
            "workvcs",
            "store",
            "external-ref-show",
            store,
            "--external-ref",
            &external_ref_id,
        ])
        .expect("parse external-ref-show"))
        .expect("show external ref");
        assert_eq!(value(&shown, "external_ref_id"), external_ref_id);
        assert_eq!(
            value(&shown, "descriptor_digest"),
            value(&recorded, "descriptor_digest")
        );

        let listed = run(Cli::try_parse_from([
            "workvcs",
            "store",
            "external-ref-list",
            store,
            "--external-store",
            &external_store_id,
            "--object-kind",
            "knowledge",
            "--scope",
            "version",
        ])
        .expect("parse external-ref-list"))
        .expect("list external refs");
        assert_eq!(value(&listed, "external_refs"), "1");
        assert_eq!(
            value(&listed, "external_ref[0].external_ref_id"),
            external_ref_id
        );
    }

    #[test]
    fn cli_creates_shows_and_lists_knowledge_spaces() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let path = tempdir.path().join("workvcs.sqlite");
        let store = path.to_str().expect("path text");
        run(Cli::try_parse_from([
            "workvcs",
            "init",
            store,
            "--display-name",
            "knowledge-space-store",
        ])
        .expect("parse init"))
        .expect("init store");

        let created = run(Cli::try_parse_from([
            "workvcs",
            "store",
            "knowledge-space-create",
            store,
            "--name",
            "Research",
        ])
        .expect("parse knowledge-space-create"))
        .expect("create knowledge space");
        assert_eq!(value(&created, "name"), "Research");
        let knowledge_space_id = value(&created, "knowledge_space_id");

        let shown = run(Cli::try_parse_from([
            "workvcs",
            "store",
            "knowledge-space-show",
            store,
            "--knowledge-space",
            &knowledge_space_id,
        ])
        .expect("parse knowledge-space-show"))
        .expect("show knowledge space");
        assert_eq!(value(&shown, "knowledge_space_id"), knowledge_space_id);
        assert_eq!(value(&shown, "name"), "Research");

        let listed = run(Cli::try_parse_from([
            "workvcs",
            "store",
            "knowledge-space-list",
            store,
            "--limit",
            "1",
        ])
        .expect("parse knowledge-space-list"))
        .expect("list knowledge spaces");
        assert_eq!(value(&listed, "knowledge_spaces"), "1");
        assert_eq!(
            value(&listed, "knowledge_space[0].knowledge_space_id"),
            knowledge_space_id
        );

        let created_records = run(Cli::try_parse_from([
            "workvcs",
            "store",
            "knowledge-space-create",
            store,
            "--name",
            "Records",
        ])
        .expect("parse second knowledge-space-create"))
        .expect("create second knowledge space");
        let records_space_id = value(&created_records, "knowledge_space_id");

        let name_filtered = run(Cli::try_parse_from([
            "workvcs",
            "store",
            "knowledge-space-list",
            store,
            "--name",
            "Records",
        ])
        .expect("parse name-filtered knowledge-space-list"))
        .expect("list name-filtered knowledge spaces");
        assert_eq!(value(&name_filtered, "knowledge_spaces"), "1");
        assert_eq!(
            value(&name_filtered, "knowledge_space[0].knowledge_space_id"),
            records_space_id
        );

        let case_sensitive_filtered = run(Cli::try_parse_from([
            "workvcs",
            "store",
            "knowledge-space-list",
            store,
            "--name",
            "records",
        ])
        .expect("parse case-sensitive knowledge-space-list"))
        .expect("list case-sensitive knowledge spaces");
        assert_eq!(value(&case_sensitive_filtered, "knowledge_spaces"), "0");
    }

    #[test]
    fn cli_creates_shows_and_lists_local_knowledge_exposures() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let path = tempdir.path().join("workvcs.sqlite");
        let store = path.to_str().expect("path text");
        run(Cli::try_parse_from([
            "workvcs",
            "init",
            store,
            "--display-name",
            "knowledge-exposure-store",
        ])
        .expect("parse init"))
        .expect("init store");
        let workspace = run(Cli::try_parse_from([
            "workvcs",
            "workspace",
            "create",
            store,
            "--display-name",
            "workspace",
        ])
        .expect("parse workspace"))
        .expect("create workspace");
        let branch = value(&workspace, "branch_id");
        let genesis = value(&workspace, "genesis_commit_id");
        let workspace_id = value(&workspace, "workspace_id");
        let knowledge = run(Cli::try_parse_from([
            "workvcs",
            "knowledge",
            "create",
            store,
            "--branch",
            &branch,
            "--head",
            &genesis,
            "--statement",
            "Expose this reusable knowledge",
        ])
        .expect("parse knowledge create"))
        .expect("create knowledge");
        let knowledge_space = run(Cli::try_parse_from([
            "workvcs",
            "store",
            "knowledge-space-create",
            store,
            "--name",
            "Research",
        ])
        .expect("parse knowledge-space-create"))
        .expect("create knowledge space");

        let exposure = run(Cli::try_parse_from([
            "workvcs",
            "store",
            "knowledge-exposure-create-local",
            store,
            "--knowledge-space",
            &value(&knowledge_space, "knowledge_space_id"),
            "--workspace",
            &workspace_id,
            "--knowledge",
            &value(&knowledge, "knowledge_entity_id"),
            "--knowledge-version",
            &value(&knowledge, "knowledge_entity_version_id"),
            "--detail-json",
            "{\"reason\":\"cli\"}",
        ])
        .expect("parse knowledge-exposure-create-local"))
        .expect("create local exposure");
        assert_eq!(
            value(&exposure, "knowledge_space_id"),
            value(&knowledge_space, "knowledge_space_id")
        );
        assert_eq!(value(&exposure, "lifecycle_status"), "active");
        assert_eq!(value(&exposure, "source_kind"), "local");
        assert_eq!(value(&exposure, "source_workspace_id"), workspace_id);
        assert_eq!(
            value(&exposure, "source_knowledge_entity_id"),
            value(&knowledge, "knowledge_entity_id")
        );
        assert_eq!(
            value(&exposure, "source_knowledge_entity_version_id"),
            value(&knowledge, "knowledge_entity_version_id")
        );
        assert_eq!(
            value(&exposure, "source_knowledge_state_digest"),
            value(&knowledge, "knowledge_state_digest")
        );
        assert_eq!(value(&exposure, "source_status"), "current");
        assert_eq!(
            value(&exposure, "transition_detail_json"),
            "{\"reason\":\"cli\"}"
        );

        let shown = run(Cli::try_parse_from([
            "workvcs",
            "store",
            "knowledge-exposure-show",
            store,
            "--exposure",
            &value(&exposure, "exposure_id"),
        ])
        .expect("parse knowledge-exposure-show"))
        .expect("show knowledge exposure");
        assert_eq!(
            value(&shown, "exposure_id"),
            value(&exposure, "exposure_id")
        );

        let listed = run(Cli::try_parse_from([
            "workvcs",
            "store",
            "knowledge-exposure-list",
            store,
            "--knowledge-space",
            &value(&knowledge_space, "knowledge_space_id"),
            "--workspace",
            &workspace_id,
            "--knowledge",
            &value(&knowledge, "knowledge_entity_id"),
            "--lifecycle-status",
            "active",
            "--source-status",
            "current",
        ])
        .expect("parse knowledge-exposure-list"))
        .expect("list knowledge exposures");
        assert_eq!(value(&listed, "exposures"), "1");
        assert_eq!(
            value(&listed, "exposure[0].exposure_id"),
            value(&exposure, "exposure_id")
        );
    }

    #[test]
    fn cli_withdraws_local_knowledge_exposure() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let path = tempdir.path().join("workvcs.sqlite");
        let store = path.to_str().expect("path text");
        run(Cli::try_parse_from([
            "workvcs",
            "init",
            store,
            "--display-name",
            "knowledge-exposure-withdraw-store",
        ])
        .expect("parse init"))
        .expect("init store");
        let workspace = run(Cli::try_parse_from([
            "workvcs",
            "workspace",
            "create",
            store,
            "--display-name",
            "workspace",
        ])
        .expect("parse workspace"))
        .expect("create workspace");
        let branch = value(&workspace, "branch_id");
        let genesis = value(&workspace, "genesis_commit_id");
        let workspace_id = value(&workspace, "workspace_id");
        let knowledge = run(Cli::try_parse_from([
            "workvcs",
            "knowledge",
            "create",
            store,
            "--branch",
            &branch,
            "--head",
            &genesis,
            "--statement",
            "Withdraw this reusable knowledge exposure",
        ])
        .expect("parse knowledge create"))
        .expect("create knowledge");
        let knowledge_space = run(Cli::try_parse_from([
            "workvcs",
            "store",
            "knowledge-space-create",
            store,
            "--name",
            "Research",
        ])
        .expect("parse knowledge-space-create"))
        .expect("create knowledge space");
        let exposure = run(Cli::try_parse_from([
            "workvcs",
            "store",
            "knowledge-exposure-create-local",
            store,
            "--knowledge-space",
            &value(&knowledge_space, "knowledge_space_id"),
            "--workspace",
            &workspace_id,
            "--knowledge",
            &value(&knowledge, "knowledge_entity_id"),
            "--knowledge-version",
            &value(&knowledge, "knowledge_entity_version_id"),
        ])
        .expect("parse knowledge-exposure-create-local"))
        .expect("create local exposure");

        let withdrawn = run(Cli::try_parse_from([
            "workvcs",
            "store",
            "knowledge-exposure-withdraw",
            store,
            "--exposure",
            &value(&exposure, "exposure_id"),
            "--current-transition",
            &value(&exposure, "transition_id"),
            "--detail-json",
            "{\"reason\":\"cli-withdraw\"}",
        ])
        .expect("parse knowledge-exposure-withdraw"))
        .expect("withdraw exposure");
        assert_eq!(
            value(&withdrawn, "exposure_id"),
            value(&exposure, "exposure_id")
        );
        assert_eq!(value(&withdrawn, "lifecycle_status"), "withdrawn");
        assert_eq!(
            value(&withdrawn, "previous_transition_id"),
            value(&exposure, "transition_id")
        );
        assert_eq!(
            value(&withdrawn, "transition_detail_json"),
            "{\"reason\":\"cli-withdraw\"}"
        );
        assert_eq!(value(&withdrawn, "source_workspace_id"), workspace_id);

        let active = run(Cli::try_parse_from([
            "workvcs",
            "store",
            "knowledge-exposure-list",
            store,
            "--knowledge-space",
            &value(&knowledge_space, "knowledge_space_id"),
            "--lifecycle-status",
            "active",
        ])
        .expect("parse active list"))
        .expect("list active exposures");
        assert_eq!(value(&active, "exposures"), "0");

        let withdrawn_list = run(Cli::try_parse_from([
            "workvcs",
            "store",
            "knowledge-exposure-list",
            store,
            "--knowledge-space",
            &value(&knowledge_space, "knowledge_space_id"),
            "--lifecycle-status",
            "withdrawn",
        ])
        .expect("parse withdrawn list"))
        .expect("list withdrawn exposures");
        assert_eq!(value(&withdrawn_list, "exposures"), "1");
        assert_eq!(
            value(&withdrawn_list, "exposure[0].exposure_id"),
            value(&exposure, "exposure_id")
        );
    }

    #[test]
    fn cli_refreshes_local_knowledge_exposure_source_status() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let path = tempdir.path().join("workvcs.sqlite");
        let store = path.to_str().expect("path text");
        run(Cli::try_parse_from([
            "workvcs",
            "init",
            store,
            "--display-name",
            "knowledge-exposure-refresh-store",
        ])
        .expect("parse init"))
        .expect("init store");
        let workspace = run(Cli::try_parse_from([
            "workvcs",
            "workspace",
            "create",
            store,
            "--display-name",
            "workspace",
        ])
        .expect("parse workspace"))
        .expect("create workspace");
        let branch = value(&workspace, "branch_id");
        let genesis = value(&workspace, "genesis_commit_id");
        let workspace_id = value(&workspace, "workspace_id");
        let knowledge = run(Cli::try_parse_from([
            "workvcs",
            "knowledge",
            "create",
            store,
            "--branch",
            &branch,
            "--head",
            &genesis,
            "--statement",
            "Refresh this reusable knowledge exposure",
        ])
        .expect("parse knowledge create"))
        .expect("create knowledge");
        let knowledge_space = run(Cli::try_parse_from([
            "workvcs",
            "store",
            "knowledge-space-create",
            store,
            "--name",
            "Research",
        ])
        .expect("parse knowledge-space-create"))
        .expect("create knowledge space");
        let exposure = run(Cli::try_parse_from([
            "workvcs",
            "store",
            "knowledge-exposure-create-local",
            store,
            "--knowledge-space",
            &value(&knowledge_space, "knowledge_space_id"),
            "--workspace",
            &workspace_id,
            "--knowledge",
            &value(&knowledge, "knowledge_entity_id"),
            "--knowledge-version",
            &value(&knowledge, "knowledge_entity_version_id"),
        ])
        .expect("parse knowledge-exposure-create-local"))
        .expect("create local exposure");

        let invalidated = run(Cli::try_parse_from([
            "workvcs",
            "knowledge",
            "invalidate",
            store,
            "--branch",
            &branch,
            "--head",
            &value(&knowledge, "commit_id"),
            "--knowledge",
            &value(&knowledge, "knowledge_entity_id"),
            "--knowledge-version",
            &value(&knowledge, "knowledge_entity_version_id"),
            "--rationale",
            "source claim changed",
        ])
        .expect("parse knowledge invalidate"))
        .expect("invalidate knowledge");
        assert_ne!(
            value(&invalidated, "knowledge_entity_version_id"),
            value(&exposure, "source_knowledge_entity_version_id")
        );

        let refreshed = run(Cli::try_parse_from([
            "workvcs",
            "store",
            "knowledge-exposure-refresh-source-status",
            store,
            "--exposure",
            &value(&exposure, "exposure_id"),
        ])
        .expect("parse knowledge-exposure-refresh-source-status"))
        .expect("refresh exposure source status");

        assert_eq!(value(&refreshed, "source_status"), "stale");
        assert_eq!(
            value(&refreshed, "source_status_detail_json"),
            "{\"checked_branch_heads\":1,\"drifted_branch_heads\":1,\"matching_branch_heads\":0,\"missing_branch_heads\":0}"
        );
        assert_eq!(
            value(&refreshed, "source_knowledge_entity_version_id"),
            value(&exposure, "source_knowledge_entity_version_id")
        );
    }

    #[test]
    fn cli_lists_available_knowledge_space_exposures() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let path = tempdir.path().join("workvcs.sqlite");
        let store = path.to_str().expect("path text");
        run(Cli::try_parse_from([
            "workvcs",
            "init",
            store,
            "--display-name",
            "knowledge-space-available-store",
        ])
        .expect("parse init"))
        .expect("init store");
        let workspace = run(Cli::try_parse_from([
            "workvcs",
            "workspace",
            "create",
            store,
            "--display-name",
            "workspace",
        ])
        .expect("parse workspace"))
        .expect("create workspace");
        let branch = value(&workspace, "branch_id");
        let genesis = value(&workspace, "genesis_commit_id");
        let workspace_id = value(&workspace, "workspace_id");
        let current_knowledge = run(Cli::try_parse_from([
            "workvcs",
            "knowledge",
            "create",
            store,
            "--branch",
            &branch,
            "--head",
            &genesis,
            "--statement",
            "Available reusable knowledge",
        ])
        .expect("parse current knowledge create"))
        .expect("create current knowledge");
        let stale_knowledge = run(Cli::try_parse_from([
            "workvcs",
            "knowledge",
            "create",
            store,
            "--branch",
            &branch,
            "--head",
            &value(&current_knowledge, "commit_id"),
            "--statement",
            "Unavailable stale knowledge",
        ])
        .expect("parse stale knowledge create"))
        .expect("create stale knowledge");
        let knowledge_space = run(Cli::try_parse_from([
            "workvcs",
            "store",
            "knowledge-space-create",
            store,
            "--name",
            "Research",
        ])
        .expect("parse knowledge-space-create"))
        .expect("create knowledge space");

        let current_exposure = run(Cli::try_parse_from([
            "workvcs",
            "store",
            "knowledge-exposure-create-local",
            store,
            "--knowledge-space",
            &value(&knowledge_space, "knowledge_space_id"),
            "--workspace",
            &workspace_id,
            "--knowledge",
            &value(&current_knowledge, "knowledge_entity_id"),
            "--knowledge-version",
            &value(&current_knowledge, "knowledge_entity_version_id"),
        ])
        .expect("parse current exposure create"))
        .expect("create current exposure");
        let stale_exposure = run(Cli::try_parse_from([
            "workvcs",
            "store",
            "knowledge-exposure-create-local",
            store,
            "--knowledge-space",
            &value(&knowledge_space, "knowledge_space_id"),
            "--workspace",
            &workspace_id,
            "--knowledge",
            &value(&stale_knowledge, "knowledge_entity_id"),
            "--knowledge-version",
            &value(&stale_knowledge, "knowledge_entity_version_id"),
        ])
        .expect("parse stale exposure create"))
        .expect("create stale exposure");

        run(Cli::try_parse_from([
            "workvcs",
            "knowledge",
            "invalidate",
            store,
            "--branch",
            &branch,
            "--head",
            &value(&stale_knowledge, "commit_id"),
            "--knowledge",
            &value(&stale_knowledge, "knowledge_entity_id"),
            "--knowledge-version",
            &value(&stale_knowledge, "knowledge_entity_version_id"),
            "--rationale",
            "source claim changed",
        ])
        .expect("parse knowledge invalidate"))
        .expect("invalidate stale knowledge");
        let stale_refreshed = run(Cli::try_parse_from([
            "workvcs",
            "store",
            "knowledge-exposure-refresh-source-status",
            store,
            "--exposure",
            &value(&stale_exposure, "exposure_id"),
        ])
        .expect("parse stale exposure refresh"))
        .expect("refresh stale exposure");
        assert_eq!(value(&stale_refreshed, "source_status"), "stale");

        let available = run(Cli::try_parse_from([
            "workvcs",
            "store",
            "knowledge-space-available-exposures",
            store,
            "--knowledge-space",
            &value(&knowledge_space, "knowledge_space_id"),
        ])
        .expect("parse knowledge-space-available-exposures"))
        .expect("list available exposures");

        assert_eq!(
            value(&available, "knowledge_space_id"),
            value(&knowledge_space, "knowledge_space_id")
        );
        assert_eq!(value(&available, "exposures"), "1");
        assert_eq!(
            value(&available, "exposure[0].exposure_id"),
            value(&current_exposure, "exposure_id")
        );
        assert_eq!(value(&available, "exposure[0].source_status"), "current");
    }

    #[test]
    fn cli_lists_source_stale_knowledge_space_exposures() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let path = tempdir.path().join("workvcs.sqlite");
        let store = path.to_str().expect("path text");
        run(Cli::try_parse_from([
            "workvcs",
            "init",
            store,
            "--display-name",
            "knowledge-space-source-stale-store",
        ])
        .expect("parse init"))
        .expect("init store");
        let workspace = run(Cli::try_parse_from([
            "workvcs",
            "workspace",
            "create",
            store,
            "--display-name",
            "workspace",
        ])
        .expect("parse workspace"))
        .expect("create workspace");
        let branch = value(&workspace, "branch_id");
        let genesis = value(&workspace, "genesis_commit_id");
        let workspace_id = value(&workspace, "workspace_id");
        let current_knowledge = run(Cli::try_parse_from([
            "workvcs",
            "knowledge",
            "create",
            store,
            "--branch",
            &branch,
            "--head",
            &genesis,
            "--statement",
            "Current reusable knowledge",
        ])
        .expect("parse current knowledge create"))
        .expect("create current knowledge");
        let stale_knowledge = run(Cli::try_parse_from([
            "workvcs",
            "knowledge",
            "create",
            store,
            "--branch",
            &branch,
            "--head",
            &value(&current_knowledge, "commit_id"),
            "--statement",
            "Source stale reusable knowledge",
        ])
        .expect("parse stale knowledge create"))
        .expect("create stale knowledge");
        let knowledge_space = run(Cli::try_parse_from([
            "workvcs",
            "store",
            "knowledge-space-create",
            store,
            "--name",
            "Research",
        ])
        .expect("parse knowledge-space-create"))
        .expect("create knowledge space");
        let current_exposure = run(Cli::try_parse_from([
            "workvcs",
            "store",
            "knowledge-exposure-create-local",
            store,
            "--knowledge-space",
            &value(&knowledge_space, "knowledge_space_id"),
            "--workspace",
            &workspace_id,
            "--knowledge",
            &value(&current_knowledge, "knowledge_entity_id"),
            "--knowledge-version",
            &value(&current_knowledge, "knowledge_entity_version_id"),
        ])
        .expect("parse current exposure create"))
        .expect("create current exposure");
        let stale_exposure = run(Cli::try_parse_from([
            "workvcs",
            "store",
            "knowledge-exposure-create-local",
            store,
            "--knowledge-space",
            &value(&knowledge_space, "knowledge_space_id"),
            "--workspace",
            &workspace_id,
            "--knowledge",
            &value(&stale_knowledge, "knowledge_entity_id"),
            "--knowledge-version",
            &value(&stale_knowledge, "knowledge_entity_version_id"),
        ])
        .expect("parse stale exposure create"))
        .expect("create stale exposure");

        run(Cli::try_parse_from([
            "workvcs",
            "knowledge",
            "invalidate",
            store,
            "--branch",
            &branch,
            "--head",
            &value(&stale_knowledge, "commit_id"),
            "--knowledge",
            &value(&stale_knowledge, "knowledge_entity_id"),
            "--knowledge-version",
            &value(&stale_knowledge, "knowledge_entity_version_id"),
            "--rationale",
            "source claim changed",
        ])
        .expect("parse knowledge invalidate"))
        .expect("invalidate stale knowledge");
        run(Cli::try_parse_from([
            "workvcs",
            "store",
            "knowledge-exposure-refresh-source-status",
            store,
            "--exposure",
            &value(&stale_exposure, "exposure_id"),
        ])
        .expect("parse stale exposure refresh"))
        .expect("refresh stale exposure");

        let source_stale = run(Cli::try_parse_from([
            "workvcs",
            "store",
            "knowledge-space-source-stale-exposures",
            store,
            "--knowledge-space",
            &value(&knowledge_space, "knowledge_space_id"),
        ])
        .expect("parse knowledge-space-source-stale-exposures"))
        .expect("list source-stale exposures");

        assert_eq!(
            value(&source_stale, "knowledge_space_id"),
            value(&knowledge_space, "knowledge_space_id")
        );
        assert_eq!(value(&source_stale, "exposures"), "1");
        assert_eq!(
            value(&source_stale, "exposure[0].exposure_id"),
            value(&stale_exposure, "exposure_id")
        );
        assert_ne!(
            value(&source_stale, "exposure[0].exposure_id"),
            value(&current_exposure, "exposure_id")
        );
        assert_eq!(value(&source_stale, "exposure[0].source_status"), "stale");
    }

    #[test]
    fn cli_lists_historical_knowledge_space_exposures() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let path = tempdir.path().join("workvcs.sqlite");
        let store = path.to_str().expect("path text");
        run(Cli::try_parse_from([
            "workvcs",
            "init",
            store,
            "--display-name",
            "knowledge-space-historical-store",
        ])
        .expect("parse init"))
        .expect("init store");
        let workspace = run(Cli::try_parse_from([
            "workvcs",
            "workspace",
            "create",
            store,
            "--display-name",
            "workspace",
        ])
        .expect("parse workspace"))
        .expect("create workspace");
        let branch = value(&workspace, "branch_id");
        let genesis = value(&workspace, "genesis_commit_id");
        let workspace_id = value(&workspace, "workspace_id");
        let active_knowledge = run(Cli::try_parse_from([
            "workvcs",
            "knowledge",
            "create",
            store,
            "--branch",
            &branch,
            "--head",
            &genesis,
            "--statement",
            "Active reusable knowledge",
        ])
        .expect("parse active knowledge create"))
        .expect("create active knowledge");
        let withdrawn_knowledge = run(Cli::try_parse_from([
            "workvcs",
            "knowledge",
            "create",
            store,
            "--branch",
            &branch,
            "--head",
            &value(&active_knowledge, "commit_id"),
            "--statement",
            "Withdrawn reusable knowledge",
        ])
        .expect("parse withdrawn knowledge create"))
        .expect("create withdrawn knowledge");
        let knowledge_space = run(Cli::try_parse_from([
            "workvcs",
            "store",
            "knowledge-space-create",
            store,
            "--name",
            "Research",
        ])
        .expect("parse knowledge-space-create"))
        .expect("create knowledge space");
        let active_exposure = run(Cli::try_parse_from([
            "workvcs",
            "store",
            "knowledge-exposure-create-local",
            store,
            "--knowledge-space",
            &value(&knowledge_space, "knowledge_space_id"),
            "--workspace",
            &workspace_id,
            "--knowledge",
            &value(&active_knowledge, "knowledge_entity_id"),
            "--knowledge-version",
            &value(&active_knowledge, "knowledge_entity_version_id"),
        ])
        .expect("parse active exposure create"))
        .expect("create active exposure");
        let withdrawn_exposure = run(Cli::try_parse_from([
            "workvcs",
            "store",
            "knowledge-exposure-create-local",
            store,
            "--knowledge-space",
            &value(&knowledge_space, "knowledge_space_id"),
            "--workspace",
            &workspace_id,
            "--knowledge",
            &value(&withdrawn_knowledge, "knowledge_entity_id"),
            "--knowledge-version",
            &value(&withdrawn_knowledge, "knowledge_entity_version_id"),
        ])
        .expect("parse withdrawn exposure create"))
        .expect("create withdrawn exposure");
        let withdrawn = run(Cli::try_parse_from([
            "workvcs",
            "store",
            "knowledge-exposure-withdraw",
            store,
            "--exposure",
            &value(&withdrawn_exposure, "exposure_id"),
            "--current-transition",
            &value(&withdrawn_exposure, "transition_id"),
        ])
        .expect("parse exposure withdraw"))
        .expect("withdraw exposure");

        let historical = run(Cli::try_parse_from([
            "workvcs",
            "store",
            "knowledge-space-historical-exposures",
            store,
            "--knowledge-space",
            &value(&knowledge_space, "knowledge_space_id"),
        ])
        .expect("parse knowledge-space-historical-exposures"))
        .expect("list historical exposures");

        assert_eq!(
            value(&historical, "knowledge_space_id"),
            value(&knowledge_space, "knowledge_space_id")
        );
        assert_eq!(value(&historical, "exposures"), "1");
        assert_eq!(
            value(&historical, "exposure[0].exposure_id"),
            value(&withdrawn, "exposure_id")
        );
        assert_ne!(
            value(&historical, "exposure[0].exposure_id"),
            value(&active_exposure, "exposure_id")
        );
        assert_eq!(
            value(&historical, "exposure[0].lifecycle_status"),
            "withdrawn"
        );
    }

    #[test]
    fn cli_refreshes_knowledge_space_source_statuses() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let path = tempdir.path().join("workvcs.sqlite");
        let store = path.to_str().expect("path text");
        run(Cli::try_parse_from([
            "workvcs",
            "init",
            store,
            "--display-name",
            "knowledge-space-refresh-store",
        ])
        .expect("parse init"))
        .expect("init store");
        let workspace = run(Cli::try_parse_from([
            "workvcs",
            "workspace",
            "create",
            store,
            "--display-name",
            "workspace",
        ])
        .expect("parse workspace"))
        .expect("create workspace");
        let branch = value(&workspace, "branch_id");
        let genesis = value(&workspace, "genesis_commit_id");
        let workspace_id = value(&workspace, "workspace_id");
        let current_knowledge = run(Cli::try_parse_from([
            "workvcs",
            "knowledge",
            "create",
            store,
            "--branch",
            &branch,
            "--head",
            &genesis,
            "--statement",
            "Current reusable knowledge",
        ])
        .expect("parse current knowledge create"))
        .expect("create current knowledge");
        let stale_knowledge = run(Cli::try_parse_from([
            "workvcs",
            "knowledge",
            "create",
            store,
            "--branch",
            &branch,
            "--head",
            &value(&current_knowledge, "commit_id"),
            "--statement",
            "Stale reusable knowledge",
        ])
        .expect("parse stale knowledge create"))
        .expect("create stale knowledge");
        let withdrawn_knowledge = run(Cli::try_parse_from([
            "workvcs",
            "knowledge",
            "create",
            store,
            "--branch",
            &branch,
            "--head",
            &value(&stale_knowledge, "commit_id"),
            "--statement",
            "Withdrawn reusable knowledge",
        ])
        .expect("parse withdrawn knowledge create"))
        .expect("create withdrawn knowledge");
        let knowledge_space = run(Cli::try_parse_from([
            "workvcs",
            "store",
            "knowledge-space-create",
            store,
            "--name",
            "Research",
        ])
        .expect("parse knowledge-space-create"))
        .expect("create knowledge space");
        let current_exposure = run(Cli::try_parse_from([
            "workvcs",
            "store",
            "knowledge-exposure-create-local",
            store,
            "--knowledge-space",
            &value(&knowledge_space, "knowledge_space_id"),
            "--workspace",
            &workspace_id,
            "--knowledge",
            &value(&current_knowledge, "knowledge_entity_id"),
            "--knowledge-version",
            &value(&current_knowledge, "knowledge_entity_version_id"),
        ])
        .expect("parse current exposure create"))
        .expect("create current exposure");
        let stale_exposure = run(Cli::try_parse_from([
            "workvcs",
            "store",
            "knowledge-exposure-create-local",
            store,
            "--knowledge-space",
            &value(&knowledge_space, "knowledge_space_id"),
            "--workspace",
            &workspace_id,
            "--knowledge",
            &value(&stale_knowledge, "knowledge_entity_id"),
            "--knowledge-version",
            &value(&stale_knowledge, "knowledge_entity_version_id"),
        ])
        .expect("parse stale exposure create"))
        .expect("create stale exposure");
        let withdrawn_exposure = run(Cli::try_parse_from([
            "workvcs",
            "store",
            "knowledge-exposure-create-local",
            store,
            "--knowledge-space",
            &value(&knowledge_space, "knowledge_space_id"),
            "--workspace",
            &workspace_id,
            "--knowledge",
            &value(&withdrawn_knowledge, "knowledge_entity_id"),
            "--knowledge-version",
            &value(&withdrawn_knowledge, "knowledge_entity_version_id"),
        ])
        .expect("parse withdrawn exposure create"))
        .expect("create withdrawn exposure");
        run(Cli::try_parse_from([
            "workvcs",
            "knowledge",
            "invalidate",
            store,
            "--branch",
            &branch,
            "--head",
            &value(&withdrawn_knowledge, "commit_id"),
            "--knowledge",
            &value(&stale_knowledge, "knowledge_entity_id"),
            "--knowledge-version",
            &value(&stale_knowledge, "knowledge_entity_version_id"),
            "--rationale",
            "source drift",
        ])
        .expect("parse knowledge invalidate"))
        .expect("invalidate stale knowledge");
        run(Cli::try_parse_from([
            "workvcs",
            "store",
            "knowledge-exposure-withdraw",
            store,
            "--exposure",
            &value(&withdrawn_exposure, "exposure_id"),
            "--current-transition",
            &value(&withdrawn_exposure, "transition_id"),
        ])
        .expect("parse withdraw exposure"))
        .expect("withdraw exposure");

        let refreshed = run(Cli::try_parse_from([
            "workvcs",
            "store",
            "knowledge-space-refresh-source-statuses",
            store,
            "--knowledge-space",
            &value(&knowledge_space, "knowledge_space_id"),
        ])
        .expect("parse knowledge-space-refresh-source-statuses"))
        .expect("refresh knowledge space source statuses");

        assert_eq!(
            value(&refreshed, "knowledge_space_id"),
            value(&knowledge_space, "knowledge_space_id")
        );
        assert_eq!(value(&refreshed, "refreshed_exposures"), "2");
        assert_eq!(value(&refreshed, "current"), "1");
        assert_eq!(value(&refreshed, "stale"), "1");
        assert_eq!(value(&refreshed, "unknown"), "0");
        assert_eq!(value(&refreshed, "unresolved"), "0");
        let first_exposure_id = value(&refreshed, "exposure[0].exposure_id");
        let first_source_status = value(&refreshed, "exposure[0].source_status");
        let second_exposure_id = value(&refreshed, "exposure[1].exposure_id");
        let second_source_status = value(&refreshed, "exposure[1].source_status");
        assert_ne!(first_exposure_id, value(&withdrawn_exposure, "exposure_id"));
        assert_ne!(
            second_exposure_id,
            value(&withdrawn_exposure, "exposure_id")
        );
        if first_exposure_id == value(&current_exposure, "exposure_id") {
            assert_eq!(first_source_status, "current");
            assert_eq!(second_exposure_id, value(&stale_exposure, "exposure_id"));
            assert_eq!(second_source_status, "stale");
        } else {
            assert_eq!(first_exposure_id, value(&stale_exposure, "exposure_id"));
            assert_eq!(first_source_status, "stale");
            assert_eq!(second_exposure_id, value(&current_exposure, "exposure_id"));
            assert_eq!(second_source_status, "current");
        }
    }

    #[test]
    fn cli_shows_knowledge_exposure_adoption_candidate() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let path = tempdir.path().join("workvcs.sqlite");
        let store = path.to_str().expect("path text");
        run(Cli::try_parse_from([
            "workvcs",
            "init",
            store,
            "--display-name",
            "knowledge-exposure-adoption-candidate-store",
        ])
        .expect("parse init"))
        .expect("init store");
        let source_store_id = Engine::open(&path)
            .expect("open store")
            .store_info()
            .expect("store info")
            .store_id
            .to_string();
        let workspace = run(Cli::try_parse_from([
            "workvcs",
            "workspace",
            "create",
            store,
            "--display-name",
            "workspace",
        ])
        .expect("parse workspace"))
        .expect("create workspace");
        let branch = value(&workspace, "branch_id");
        let genesis = value(&workspace, "genesis_commit_id");
        let workspace_id = value(&workspace, "workspace_id");
        let knowledge = run(Cli::try_parse_from([
            "workvcs",
            "knowledge",
            "create",
            store,
            "--branch",
            &branch,
            "--head",
            &genesis,
            "--statement",
            "Reusable adoption candidate knowledge",
            "--scope-json",
            "{\"kind\":\"workspace\"}",
            "--provenance-json",
            "{\"source\":\"cli\"}",
        ])
        .expect("parse knowledge create"))
        .expect("create knowledge");
        let knowledge_space = run(Cli::try_parse_from([
            "workvcs",
            "store",
            "knowledge-space-create",
            store,
            "--name",
            "Research",
        ])
        .expect("parse knowledge-space-create"))
        .expect("create knowledge space");
        let exposure = run(Cli::try_parse_from([
            "workvcs",
            "store",
            "knowledge-exposure-create-local",
            store,
            "--knowledge-space",
            &value(&knowledge_space, "knowledge_space_id"),
            "--workspace",
            &workspace_id,
            "--knowledge",
            &value(&knowledge, "knowledge_entity_id"),
            "--knowledge-version",
            &value(&knowledge, "knowledge_entity_version_id"),
        ])
        .expect("parse exposure create"))
        .expect("create exposure");

        let candidate = run(Cli::try_parse_from([
            "workvcs",
            "store",
            "knowledge-exposure-adoption-candidate",
            store,
            "--exposure",
            &value(&exposure, "exposure_id"),
        ])
        .expect("parse adoption candidate"))
        .expect("show adoption candidate");

        assert_eq!(
            value(&candidate, "exposure_id"),
            value(&exposure, "exposure_id")
        );
        assert_eq!(value(&candidate, "source_store_id"), source_store_id);
        assert_eq!(
            value(&candidate, "source_workspace_id"),
            value(&exposure, "source_workspace_id")
        );
        assert_eq!(
            value(&candidate, "source_knowledge_entity_id"),
            value(&knowledge, "knowledge_entity_id")
        );
        assert_eq!(
            value(&candidate, "source_knowledge_entity_version_id"),
            value(&knowledge, "knowledge_entity_version_id")
        );
        assert_eq!(
            value(&candidate, "source_knowledge_state_digest"),
            value(&knowledge, "knowledge_state_digest")
        );
        assert_eq!(value(&candidate, "source_status"), "current");
        assert_eq!(value(&candidate, "knowledge_status"), "active");
        assert_eq!(
            value(&candidate, "knowledge_statement_json"),
            "\"Reusable adoption candidate knowledge\""
        );
        assert_eq!(
            value(&candidate, "knowledge_scope_json"),
            "{\"kind\":\"workspace\"}"
        );
        assert_eq!(
            value(&candidate, "knowledge_provenance_json"),
            "{\"source\":\"cli\"}"
        );
        let adoption_provenance = value(&candidate, "adoption_provenance_json");
        assert!(adoption_provenance.contains(&format!(
            "\"adopted_from_exposure_id\":\"{}\"",
            value(&exposure, "exposure_id")
        )));
        assert!(
            adoption_provenance.contains(&format!("\"source_store_id\":\"{source_store_id}\""))
        );
    }

    #[test]
    fn cli_adopts_knowledge_exposure() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let path = tempdir.path().join("workvcs.sqlite");
        let store = path.to_str().expect("path text");
        run(Cli::try_parse_from([
            "workvcs",
            "init",
            store,
            "--display-name",
            "knowledge-exposure-adoption-store",
        ])
        .expect("parse init"))
        .expect("init store");
        let workspace = run(Cli::try_parse_from([
            "workvcs",
            "workspace",
            "create",
            store,
            "--display-name",
            "workspace",
        ])
        .expect("parse workspace"))
        .expect("create workspace");
        let branch = value(&workspace, "branch_id");
        let genesis = value(&workspace, "genesis_commit_id");
        let workspace_id = value(&workspace, "workspace_id");
        let knowledge = run(Cli::try_parse_from([
            "workvcs",
            "knowledge",
            "create",
            store,
            "--branch",
            &branch,
            "--head",
            &genesis,
            "--statement",
            "Reusable adoption knowledge",
            "--scope-json",
            "{\"domain\":\"research\"}",
            "--provenance-json",
            "{\"source\":\"cli\"}",
        ])
        .expect("parse knowledge create"))
        .expect("create knowledge");
        let knowledge_space = run(Cli::try_parse_from([
            "workvcs",
            "store",
            "knowledge-space-create",
            store,
            "--name",
            "Research",
        ])
        .expect("parse knowledge-space-create"))
        .expect("create knowledge space");
        let exposure = run(Cli::try_parse_from([
            "workvcs",
            "store",
            "knowledge-exposure-create-local",
            store,
            "--knowledge-space",
            &value(&knowledge_space, "knowledge_space_id"),
            "--workspace",
            &workspace_id,
            "--knowledge",
            &value(&knowledge, "knowledge_entity_id"),
            "--knowledge-version",
            &value(&knowledge, "knowledge_entity_version_id"),
        ])
        .expect("parse exposure create"))
        .expect("create exposure");

        let adoption = run(Cli::try_parse_from([
            "workvcs",
            "store",
            "knowledge-exposure-adopt",
            store,
            "--branch",
            &branch,
            "--head",
            &value(&knowledge, "commit_id"),
            "--exposure",
            &value(&exposure, "exposure_id"),
            "--rationale",
            "adopt current exposure",
        ])
        .expect("parse adoption"))
        .expect("adopt exposure");

        assert_eq!(
            value(&adoption, "exposure_id"),
            value(&exposure, "exposure_id")
        );
        assert_eq!(value(&adoption, "source_status"), "current");
        assert_eq!(
            value(&adoption, "source_knowledge_entity_id"),
            value(&knowledge, "knowledge_entity_id")
        );
        assert_eq!(
            value(&adoption, "source_knowledge_entity_version_id"),
            value(&knowledge, "knowledge_entity_version_id")
        );
        assert_eq!(
            value(&adoption, "source_knowledge_state_digest"),
            value(&knowledge, "knowledge_state_digest")
        );
        assert!(!value(&adoption, "adopted_knowledge_entity_id").is_empty());
        assert!(!value(&adoption, "adopted_knowledge_entity_version_id").is_empty());
        assert!(!value(&adoption, "adopted_knowledge_commit_id").is_empty());
        assert_eq!(value(&adoption, "relation_type"), "derived_from");
        assert_eq!(
            value(&adoption, "relation_commit_id"),
            value(&adoption, "final_head_commit_id")
        );
        let provenance = value(&adoption, "adopted_knowledge_provenance_json");
        assert!(provenance.contains("\"knowledge_exposure_adoption_v1\""));
        assert!(provenance.contains("\"source_provenance\":{\"source\":\"cli\"}"));
    }

    #[test]
    fn cli_bundle_export_reports_knowledge_exposure_closure() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let path = tempdir.path().join("workvcs.sqlite");
        let store = path.to_str().expect("path text");
        run(Cli::try_parse_from([
            "workvcs",
            "init",
            store,
            "--display-name",
            "bundle-knowledge-exposure-store",
        ])
        .expect("parse init"))
        .expect("init store");
        let workspace = run(Cli::try_parse_from([
            "workvcs",
            "workspace",
            "create",
            store,
            "--display-name",
            "workspace",
        ])
        .expect("parse workspace"))
        .expect("create workspace");
        let branch = value(&workspace, "branch_id");
        let genesis = value(&workspace, "genesis_commit_id");
        let workspace_id = value(&workspace, "workspace_id");
        let knowledge = run(Cli::try_parse_from([
            "workvcs",
            "knowledge",
            "create",
            store,
            "--branch",
            &branch,
            "--head",
            &genesis,
            "--statement",
            "Reusable bundle exposure knowledge",
        ])
        .expect("parse knowledge create"))
        .expect("create knowledge");
        let knowledge_space = run(Cli::try_parse_from([
            "workvcs",
            "store",
            "knowledge-space-create",
            store,
            "--name",
            "Research",
        ])
        .expect("parse knowledge-space-create"))
        .expect("create knowledge space");
        let exposure = run(Cli::try_parse_from([
            "workvcs",
            "store",
            "knowledge-exposure-create-local",
            store,
            "--knowledge-space",
            &value(&knowledge_space, "knowledge_space_id"),
            "--workspace",
            &workspace_id,
            "--knowledge",
            &value(&knowledge, "knowledge_entity_id"),
            "--knowledge-version",
            &value(&knowledge, "knowledge_entity_version_id"),
        ])
        .expect("parse exposure create"))
        .expect("create exposure");
        let adoption = run(Cli::try_parse_from([
            "workvcs",
            "store",
            "knowledge-exposure-adopt",
            store,
            "--branch",
            &branch,
            "--head",
            &value(&knowledge, "commit_id"),
            "--exposure",
            &value(&exposure, "exposure_id"),
            "--rationale",
            "adopt current exposure",
        ])
        .expect("parse adoption"))
        .expect("adopt exposure");

        let exported = run(Cli::try_parse_from([
            "workvcs",
            "bundle",
            "export",
            store,
            "--commit",
            &value(&adoption, "final_head_commit_id"),
        ])
        .expect("parse bundle export"))
        .expect("export bundle manifest");

        assert_eq!(value(&exported, "knowledge_spaces"), "1");
        assert_eq!(value(&exported, "knowledge_exposures"), "1");
        assert_eq!(value(&exported, "knowledge_exposure_local_sources"), "1");
        assert_eq!(value(&exported, "knowledge_exposure_transitions"), "1");
        assert_eq!(value(&exported, "knowledge_exposure_source_statuses"), "1");
    }

    #[test]
    fn cli_why_shows_knowledge_exposure_adoption_provenance() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let path = tempdir.path().join("workvcs.sqlite");
        let store = path.to_str().expect("path text");
        run(Cli::try_parse_from([
            "workvcs",
            "init",
            store,
            "--display-name",
            "why-knowledge-exposure-provenance-store",
        ])
        .expect("parse init"))
        .expect("init store");
        let workspace = run(Cli::try_parse_from([
            "workvcs",
            "workspace",
            "create",
            store,
            "--display-name",
            "workspace",
        ])
        .expect("parse workspace"))
        .expect("create workspace");
        let branch = value(&workspace, "branch_id");
        let genesis = value(&workspace, "genesis_commit_id");
        let workspace_id = value(&workspace, "workspace_id");
        let knowledge = run(Cli::try_parse_from([
            "workvcs",
            "knowledge",
            "create",
            store,
            "--branch",
            &branch,
            "--head",
            &genesis,
            "--statement",
            "Reusable why adoption knowledge",
        ])
        .expect("parse knowledge create"))
        .expect("create knowledge");
        let knowledge_space = run(Cli::try_parse_from([
            "workvcs",
            "store",
            "knowledge-space-create",
            store,
            "--name",
            "Research",
        ])
        .expect("parse knowledge-space-create"))
        .expect("create knowledge space");
        let exposure = run(Cli::try_parse_from([
            "workvcs",
            "store",
            "knowledge-exposure-create-local",
            store,
            "--knowledge-space",
            &value(&knowledge_space, "knowledge_space_id"),
            "--workspace",
            &workspace_id,
            "--knowledge",
            &value(&knowledge, "knowledge_entity_id"),
            "--knowledge-version",
            &value(&knowledge, "knowledge_entity_version_id"),
        ])
        .expect("parse exposure create"))
        .expect("create exposure");
        let adoption = run(Cli::try_parse_from([
            "workvcs",
            "store",
            "knowledge-exposure-adopt",
            store,
            "--branch",
            &branch,
            "--head",
            &value(&knowledge, "commit_id"),
            "--exposure",
            &value(&exposure, "exposure_id"),
            "--rationale",
            "adopt current exposure",
        ])
        .expect("parse adoption"))
        .expect("adopt exposure");

        let why = run(Cli::try_parse_from([
            "workvcs",
            "why",
            store,
            "--commit",
            &value(&adoption, "final_head_commit_id"),
            "--entity",
            &value(&adoption, "adopted_knowledge_entity_id"),
        ])
        .expect("parse why"))
        .expect("why adopted knowledge");

        assert_eq!(value(&why, "relation_edges"), "1");
        assert_eq!(
            value(&why, "relation.0.relation_kind"),
            "knowledge_exposure_derived_from"
        );
        assert_eq!(value(&why, "relation.0.direction"), "outgoing");
        assert_eq!(value(&why, "relation.0.source_kind"), "entity");
        assert_eq!(value(&why, "relation.0.source_entity_kind"), "knowledge");
        assert_eq!(
            value(&why, "relation.0.source_entity_id"),
            value(&adoption, "adopted_knowledge_entity_id")
        );
        assert_eq!(value(&why, "relation.0.target_kind"), "knowledge_exposure");
        assert_eq!(
            value(&why, "relation.0.target_exposure_id"),
            value(&exposure, "exposure_id")
        );
    }

    #[test]
    fn cli_why_accepts_knowledge_exposure_subject() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let path = tempdir.path().join("workvcs.sqlite");
        let store = path.to_str().expect("path text");
        run(Cli::try_parse_from([
            "workvcs",
            "init",
            store,
            "--display-name",
            "why-knowledge-exposure-subject-store",
        ])
        .expect("parse init"))
        .expect("init store");
        let workspace = run(Cli::try_parse_from([
            "workvcs",
            "workspace",
            "create",
            store,
            "--display-name",
            "workspace",
        ])
        .expect("parse workspace"))
        .expect("create workspace");
        let branch = value(&workspace, "branch_id");
        let genesis = value(&workspace, "genesis_commit_id");
        let workspace_id = value(&workspace, "workspace_id");
        let knowledge = run(Cli::try_parse_from([
            "workvcs",
            "knowledge",
            "create",
            store,
            "--branch",
            &branch,
            "--head",
            &genesis,
            "--statement",
            "Reusable exposure subject knowledge",
        ])
        .expect("parse knowledge create"))
        .expect("create knowledge");
        let knowledge_space = run(Cli::try_parse_from([
            "workvcs",
            "store",
            "knowledge-space-create",
            store,
            "--name",
            "Research",
        ])
        .expect("parse knowledge-space-create"))
        .expect("create knowledge space");
        let exposure = run(Cli::try_parse_from([
            "workvcs",
            "store",
            "knowledge-exposure-create-local",
            store,
            "--knowledge-space",
            &value(&knowledge_space, "knowledge_space_id"),
            "--workspace",
            &workspace_id,
            "--knowledge",
            &value(&knowledge, "knowledge_entity_id"),
            "--knowledge-version",
            &value(&knowledge, "knowledge_entity_version_id"),
        ])
        .expect("parse exposure create"))
        .expect("create exposure");
        let adoption = run(Cli::try_parse_from([
            "workvcs",
            "store",
            "knowledge-exposure-adopt",
            store,
            "--branch",
            &branch,
            "--head",
            &value(&knowledge, "commit_id"),
            "--exposure",
            &value(&exposure, "exposure_id"),
            "--rationale",
            "adopt current exposure",
        ])
        .expect("parse adoption"))
        .expect("adopt exposure");

        let why = run(Cli::try_parse_from([
            "workvcs",
            "why",
            store,
            "--commit",
            &value(&adoption, "final_head_commit_id"),
            "--exposure",
            &value(&exposure, "exposure_id"),
        ])
        .expect("parse why"))
        .expect("why exposure");

        assert_eq!(value(&why, "subject_kind"), "knowledge_exposure");
        assert_eq!(
            value(&why, "subject_exposure_id"),
            value(&exposure, "exposure_id")
        );
        assert_eq!(value(&why, "relation_edges"), "1");
        assert_eq!(
            value(&why, "relation.0.relation_kind"),
            "knowledge_exposure_derived_from"
        );
        assert_eq!(value(&why, "relation.0.direction"), "incoming");
        assert_eq!(value(&why, "relation.0.source_kind"), "entity");
        assert_eq!(value(&why, "relation.0.source_entity_kind"), "knowledge");
        assert_eq!(
            value(&why, "relation.0.source_entity_id"),
            value(&adoption, "adopted_knowledge_entity_id")
        );
        assert_eq!(value(&why, "relation.0.target_kind"), "knowledge_exposure");
        assert_eq!(
            value(&why, "relation.0.target_exposure_id"),
            value(&exposure, "exposure_id")
        );
    }

    #[test]
    fn cli_links_knowledge_derived_from_exposure() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let path = tempdir.path().join("workvcs.sqlite");
        let store = path.to_str().expect("path text");
        run(Cli::try_parse_from([
            "workvcs",
            "init",
            store,
            "--display-name",
            "knowledge-exposure-derived-from-store",
        ])
        .expect("parse init"))
        .expect("init store");
        let workspace = run(Cli::try_parse_from([
            "workvcs",
            "workspace",
            "create",
            store,
            "--display-name",
            "workspace",
        ])
        .expect("parse workspace"))
        .expect("create workspace");
        let branch = value(&workspace, "branch_id");
        let genesis = value(&workspace, "genesis_commit_id");
        let workspace_id = value(&workspace, "workspace_id");
        let source_knowledge = run(Cli::try_parse_from([
            "workvcs",
            "knowledge",
            "create",
            store,
            "--branch",
            &branch,
            "--head",
            &genesis,
            "--statement",
            "Reusable source knowledge",
        ])
        .expect("parse source knowledge create"))
        .expect("create source knowledge");
        let adopted_knowledge = run(Cli::try_parse_from([
            "workvcs",
            "knowledge",
            "create",
            store,
            "--branch",
            &branch,
            "--head",
            &value(&source_knowledge, "commit_id"),
            "--statement",
            "Workspace-local adopted knowledge",
        ])
        .expect("parse adopted knowledge create"))
        .expect("create adopted knowledge");
        let knowledge_space = run(Cli::try_parse_from([
            "workvcs",
            "store",
            "knowledge-space-create",
            store,
            "--name",
            "Research",
        ])
        .expect("parse knowledge-space-create"))
        .expect("create knowledge space");
        let exposure = run(Cli::try_parse_from([
            "workvcs",
            "store",
            "knowledge-exposure-create-local",
            store,
            "--knowledge-space",
            &value(&knowledge_space, "knowledge_space_id"),
            "--workspace",
            &workspace_id,
            "--knowledge",
            &value(&source_knowledge, "knowledge_entity_id"),
            "--knowledge-version",
            &value(&source_knowledge, "knowledge_entity_version_id"),
        ])
        .expect("parse exposure create"))
        .expect("create exposure");

        let relation = run(Cli::try_parse_from([
            "workvcs",
            "store",
            "knowledge-exposure-derived-from-link",
            store,
            "--branch",
            &branch,
            "--head",
            &value(&adopted_knowledge, "commit_id"),
            "--knowledge",
            &value(&adopted_knowledge, "knowledge_entity_id"),
            "--exposure",
            &value(&exposure, "exposure_id"),
            "--rationale",
            "adopted from exposed source",
        ])
        .expect("parse derived_from link"))
        .expect("create derived_from link");

        assert_eq!(value(&relation, "workspace_id"), workspace_id);
        assert_eq!(value(&relation, "branch_id"), branch);
        assert_eq!(
            value(&relation, "previous_head_commit_id"),
            value(&adopted_knowledge, "commit_id")
        );
        assert_eq!(value(&relation, "relation_type"), "derived_from");
        assert_eq!(
            value(&relation, "knowledge_entity_id"),
            value(&adopted_knowledge, "knowledge_entity_id")
        );
        assert_eq!(
            value(&relation, "exposure_id"),
            value(&exposure, "exposure_id")
        );
        assert!(!value(&relation, "relation_id").is_empty());
        assert!(!value(&relation, "relation_version_id").is_empty());
        assert!(!value(&relation, "work_state_digest").is_empty());
    }

    #[test]
    fn cli_restores_branch_to_historical_work_state() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let path = tempdir.path().join("workvcs.sqlite");
        let store = path.to_str().expect("path text");

        run(
            Cli::try_parse_from(["workvcs", "init", store, "--display-name", "cli-store"])
                .expect("parse init"),
        )
        .expect("run init");
        let workspace = run(Cli::try_parse_from([
            "workvcs",
            "workspace",
            "create",
            store,
            "--display-name",
            "workspace",
        ])
        .expect("parse workspace"))
        .expect("create workspace");
        let branch = value(&workspace, "branch_id");
        let genesis = value(&workspace, "genesis_commit_id");

        let task = run(Cli::try_parse_from([
            "workvcs",
            "task",
            "create",
            store,
            "--branch",
            &branch,
            "--head",
            &genesis,
            "--description",
            "Temporary work",
        ])
        .expect("parse task"))
        .expect("create task");
        let head = value(&task, "commit_id");

        let restored = run(Cli::try_parse_from([
            "workvcs",
            "restore",
            store,
            "--branch",
            &branch,
            "--head",
            &head,
            "--target-commit",
            &genesis,
            "--rationale-json",
            "{\"reason\":\"return to baseline\"}",
        ])
        .expect("parse restore"))
        .expect("restore");
        assert_eq!(value(&restored, "branch_id"), branch);
        assert_eq!(value(&restored, "previous_head_commit_id"), head);
        assert_eq!(value(&restored, "target_commit_id"), genesis);
        assert_eq!(value(&restored, "operation_count"), "1");
        let restore_commit = value(&restored, "commit_id");
        let restore_changeset = value(&restored, "changeset_id");

        let branch_head =
            run(
                Cli::try_parse_from(["workvcs", "branch", "head", store, "--branch", &branch])
                    .expect("parse branch head"),
            )
            .expect("branch head");
        assert_eq!(value(&branch_head, "head_commit_id"), restore_commit);
        assert_eq!(value(&branch_head, "head_changeset_id"), restore_changeset);
        assert_eq!(value(&branch_head, "head_commit_kind"), "normal");
        assert_eq!(
            value(&branch_head, "head_operation_type"),
            "workstate.restore"
        );

        let state =
            run(
                Cli::try_parse_from(["workvcs", "show-at", store, "--commit", &restore_commit])
                    .expect("parse show-at restore"),
            )
            .expect("show restore state");
        assert_eq!(value(&state, "entities"), "0");
        assert_eq!(value(&state, "relations"), "0");
    }

    #[test]
    fn cli_refreshes_and_shows_branch_projection() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let path = tempdir.path().join("workvcs.sqlite");
        let store = path.to_str().expect("path text");

        run(
            Cli::try_parse_from(["workvcs", "init", store, "--display-name", "cli-store"])
                .expect("parse init"),
        )
        .expect("run init");
        let workspace = run(Cli::try_parse_from([
            "workvcs",
            "workspace",
            "create",
            store,
            "--display-name",
            "workspace",
        ])
        .expect("parse workspace"))
        .expect("create workspace");
        let branch = value(&workspace, "branch_id");
        let genesis = value(&workspace, "genesis_commit_id");

        let initial =
            run(
                Cli::try_parse_from(["workvcs", "projection", "show", store, "--branch", &branch])
                    .expect("parse projection show"),
            )
            .expect("show initial projection");
        assert_eq!(value(&initial, "status"), "not_materialized");
        assert_eq!(value(&initial, "is_current"), "false");

        let refreshed = run(Cli::try_parse_from([
            "workvcs",
            "projection",
            "refresh",
            store,
            "--branch",
            &branch,
        ])
        .expect("parse projection refresh"))
        .expect("refresh projection");
        assert_eq!(value(&refreshed, "projected_commit_id"), genesis);
        assert_eq!(value(&refreshed, "entity_count"), "0");

        let task = run(Cli::try_parse_from([
            "workvcs",
            "task",
            "create",
            store,
            "--branch",
            &branch,
            "--head",
            &genesis,
            "--description",
            "Move projection stale",
        ])
        .expect("parse task"))
        .expect("create task");
        let task_commit = value(&task, "commit_id");

        let not_materialized =
            run(
                Cli::try_parse_from(["workvcs", "projection", "show", store, "--branch", &branch])
                    .expect("parse not-materialized projection show"),
            )
            .expect("show not-materialized projection");
        assert_eq!(value(&not_materialized, "status"), "not_materialized");
        assert_eq!(value(&not_materialized, "projected_commit_id"), "");
        assert_eq!(value(&not_materialized, "head_commit_id"), task_commit);

        let refreshed = run(Cli::try_parse_from([
            "workvcs",
            "projection",
            "refresh",
            store,
            "--branch",
            &branch,
        ])
        .expect("parse second projection refresh"))
        .expect("refresh stale projection");
        assert_eq!(value(&refreshed, "projected_commit_id"), task_commit);
        assert_eq!(value(&refreshed, "entity_count"), "1");

        let current =
            run(
                Cli::try_parse_from(["workvcs", "projection", "show", store, "--branch", &branch])
                    .expect("parse current projection show"),
            )
            .expect("show current projection");
        assert_eq!(value(&current, "status"), "complete");
        assert_eq!(value(&current, "is_current"), "true");
    }

    #[test]
    fn cli_creates_and_shows_checkpoint() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let path = tempdir.path().join("workvcs.sqlite");
        let store = path.to_str().expect("path text");

        run(
            Cli::try_parse_from(["workvcs", "init", store, "--display-name", "cli-store"])
                .expect("parse init"),
        )
        .expect("run init");
        let workspace = run(Cli::try_parse_from([
            "workvcs",
            "workspace",
            "create",
            store,
            "--display-name",
            "workspace",
        ])
        .expect("parse workspace"))
        .expect("create workspace");
        let genesis = value(&workspace, "genesis_commit_id");

        let created = run(Cli::try_parse_from([
            "workvcs",
            "checkpoint",
            "create",
            store,
            "--commit",
            &genesis,
        ])
        .expect("parse checkpoint create"))
        .expect("create checkpoint");
        assert_eq!(value(&created, "commit_id"), genesis);
        assert_eq!(value(&created, "checkpoint_format_version"), "1");
        assert_eq!(value(&created, "usability_state"), "usable");
        assert_eq!(value(&created, "entity_count"), "0");
        let checkpoint = value(&created, "checkpoint_id");

        let shown = run(Cli::try_parse_from([
            "workvcs",
            "checkpoint",
            "show",
            store,
            "--checkpoint",
            &checkpoint,
        ])
        .expect("parse checkpoint show"))
        .expect("show checkpoint");
        assert_eq!(value(&shown, "checkpoint_id"), checkpoint);
        assert_eq!(value(&shown, "commit_id"), genesis);
        assert_eq!(
            value(&shown, "content_digest"),
            value(&created, "content_digest")
        );

        let validated = run(Cli::try_parse_from([
            "workvcs",
            "checkpoint",
            "validate",
            store,
            "--checkpoint",
            &checkpoint,
        ])
        .expect("parse checkpoint validate"))
        .expect("validate checkpoint");
        assert_eq!(value(&validated, "checkpoint_id"), checkpoint);
        assert_eq!(value(&validated, "valid"), "true");
        assert_eq!(value(&validated, "problem"), "none");
        assert_eq!(
            value(&validated, "expected_content_digest"),
            value(&created, "content_digest")
        );

        let listed = run(Cli::try_parse_from([
            "workvcs",
            "checkpoint",
            "list",
            store,
            "--commit",
            &genesis,
        ])
        .expect("parse checkpoint list"))
        .expect("list checkpoints");
        assert_eq!(value(&listed, "commit_id"), genesis);
        assert_eq!(value(&listed, "checkpoints"), "1");
        assert_eq!(value(&listed, "checkpoint[0].id"), checkpoint);

        let listed_usable = run(Cli::try_parse_from([
            "workvcs",
            "checkpoint",
            "list",
            store,
            "--commit",
            &genesis,
            "--usability-state",
            "usable",
        ])
        .expect("parse usable checkpoint list"))
        .expect("list usable checkpoints");
        assert_eq!(value(&listed_usable, "checkpoints"), "1");
        assert_eq!(value(&listed_usable, "checkpoint[0].id"), checkpoint);

        let listed_by_digest = run(Cli::try_parse_from([
            "workvcs",
            "checkpoint",
            "list",
            store,
            "--commit",
            &genesis,
            "--content-digest",
            &value(&created, "content_digest"),
        ])
        .expect("parse checkpoint list by digest"))
        .expect("list checkpoints by digest");
        assert_eq!(value(&listed_by_digest, "checkpoints"), "1");
        assert_eq!(value(&listed_by_digest, "checkpoint[0].id"), checkpoint);

        let listed_missing_state = run(Cli::try_parse_from([
            "workvcs",
            "checkpoint",
            "list",
            store,
            "--commit",
            &genesis,
            "--usability-state",
            "quarantined",
        ])
        .expect("parse missing checkpoint list by state"))
        .expect("list missing checkpoints by state");
        assert_eq!(value(&listed_missing_state, "checkpoints"), "0");

        let latest = run(Cli::try_parse_from([
            "workvcs",
            "checkpoint",
            "latest",
            store,
            "--commit",
            &genesis,
        ])
        .expect("parse checkpoint latest"))
        .expect("latest checkpoint");
        assert_eq!(value(&latest, "commit_id"), genesis);
        assert_eq!(value(&latest, "checkpoint_found"), "true");
        assert_eq!(value(&latest, "checkpoint_id"), checkpoint);

        let exported =
            run(
                Cli::try_parse_from(["workvcs", "bundle", "export", store, "--commit", &genesis])
                    .expect("parse bundle export"),
            )
            .expect("export bundle manifest");
        assert_eq!(
            value(&exported, "bundle_manifest_profile"),
            "workvcs-local-export-manifest-v1"
        );
        assert_eq!(value(&exported, "bundle_manifest_version"), "1");
        assert_eq!(value(&exported, "commit_id"), genesis);
        assert_eq!(value(&exported, "commits"), "1");
        assert_eq!(value(&exported, "exported_branch_heads"), "1");
        assert_eq!(value(&exported, "entities"), "0");
        assert_eq!(value(&exported, "entity_versions"), "0");
        assert_eq!(value(&exported, "relation_versions"), "0");
        assert_eq!(value(&exported, "entity_membership_changes"), "0");
        assert_eq!(value(&exported, "relation_membership_changes"), "0");
        assert_eq!(value(&exported, "checkpoint_candidates"), "1");
        assert_eq!(value(&exported, "checkpoint_candidate[0].id"), checkpoint);
        assert_eq!(
            value(&exported, "checkpoint_candidate[0].commit_id"),
            genesis
        );
        assert_eq!(
            value(&exported, "checkpoint_candidate[0].content_digest"),
            value(&created, "content_digest")
        );

        let manifest_json = run(Cli::try_parse_from([
            "workvcs",
            "bundle",
            "export-json",
            store,
            "--commit",
            &genesis,
        ])
        .expect("parse bundle export-json"))
        .expect("export bundle manifest JSON");
        parse_canonical_json(manifest_json.trim_end().as_bytes())
            .expect("export-json emits canonical semantic JSON");
        let manifest_file = tempdir.path().join("bundle-manifest.json");
        fs::write(&manifest_file, manifest_json.as_bytes()).expect("write manifest file");
        let validation = run(Cli::try_parse_from([
            "workvcs",
            "bundle",
            "validate-manifest",
            store,
            "--commit",
            &genesis,
            "--manifest-file",
            manifest_file.to_str().expect("manifest file path"),
        ])
        .expect("parse bundle validate-manifest"))
        .expect("validate bundle manifest");
        assert_eq!(value(&validation, "commit_id"), genesis);
        assert_eq!(value(&validation, "valid"), "true");
        assert_eq!(value(&validation, "problem"), "none");

        let export_dir = tempdir.path().join("bundle-export");
        let exported_dir = run(Cli::try_parse_from([
            "workvcs",
            "bundle",
            "export-dir",
            store,
            "--commit",
            &genesis,
            "--output-dir",
            export_dir.to_str().expect("export dir path"),
        ])
        .expect("parse bundle export-dir"))
        .expect("export bundle directory");
        assert_eq!(
            value(&exported_dir, "bundle_payload_export_profile"),
            "workvcs-local-payload-index-v1"
        );
        assert_eq!(value(&exported_dir, "bundle_payload_index_version"), "1");
        assert_eq!(value(&exported_dir, "commit_id"), genesis);
        assert_eq!(value(&exported_dir, "payload_files"), "3");
        assert_eq!(value(&exported_dir, "payload_references"), "5");
        assert_eq!(
            fs::read_to_string(export_dir.join("manifest.json")).expect("manifest file"),
            manifest_json
        );
        let payload_index =
            fs::read(export_dir.join("payload-index.json")).expect("payload index file");
        parse_canonical_json(&payload_index).expect("payload index is canonical JSON");

        let validated_dir = run(Cli::try_parse_from([
            "workvcs",
            "bundle",
            "validate-dir",
            store,
            "--commit",
            &genesis,
            "--input-dir",
            export_dir.to_str().expect("export dir path"),
        ])
        .expect("parse bundle validate-dir"))
        .expect("validate bundle directory");
        assert_eq!(value(&validated_dir, "commit_id"), genesis);
        assert_eq!(value(&validated_dir, "valid"), "true");
        assert_eq!(value(&validated_dir, "expected_payload_files"), "3");
        assert_eq!(value(&validated_dir, "actual_payload_files"), "3");
        assert_eq!(value(&validated_dir, "expected_payload_references"), "5");
        assert_eq!(value(&validated_dir, "problem"), "none");

        let preflight = run(Cli::try_parse_from([
            "workvcs",
            "bundle",
            "preflight-dir",
            store,
            "--input-dir",
            export_dir.to_str().expect("export dir path"),
        ])
        .expect("parse bundle preflight-dir"))
        .expect("preflight bundle directory");
        assert_eq!(value(&preflight, "valid"), "true");
        assert_eq!(value(&preflight, "format_compatible"), "true");
        assert_eq!(value(&preflight, "target_commit_id"), genesis);
        assert_eq!(value(&preflight, "source_store_relation"), "same_store");
        assert_eq!(value(&preflight, "incoming_commit_present"), "true");
        assert_eq!(value(&preflight, "import_required"), "false");
        assert_eq!(value(&preflight, "action"), "already_present");
        assert_eq!(value(&preflight, "exported_branch_heads"), "1");
        assert_eq!(value(&preflight, "branch_heads_already_present"), "1");
        assert_eq!(value(&preflight, "branch_heads_fast_forward"), "0");
        assert_eq!(value(&preflight, "branch_heads_diverged"), "0");
        assert_eq!(value(&preflight, "problem"), "none");

        let import_attempt = run(Cli::try_parse_from([
            "workvcs",
            "bundle",
            "import-dir",
            store,
            "--input-dir",
            export_dir.to_str().expect("export dir path"),
        ])
        .expect("parse bundle import-dir"))
        .expect("record bundle import attempt");
        assert_eq!(value(&import_attempt, "recorded"), "true");
        assert_ne!(value(&import_attempt, "import_id"), "none");
        assert_eq!(
            value(&import_attempt, "import_profile"),
            "workvcs-local-payload-directory-v1"
        );
        assert_eq!(value(&import_attempt, "outcome"), "already_present");
        assert_eq!(value(&import_attempt, "valid"), "true");
        assert_eq!(
            value(&import_attempt, "source_store_relation"),
            "same_store"
        );
        assert_eq!(value(&import_attempt, "incoming_commit_present"), "true");
        assert_eq!(value(&import_attempt, "import_required"), "false");
        assert_eq!(value(&import_attempt, "can_apply"), "false");
        assert_eq!(value(&import_attempt, "exported_branch_heads"), "1");
        assert_eq!(value(&import_attempt, "branch_heads_already_present"), "1");
        assert_eq!(value(&import_attempt, "problem"), "none");
        let import_id = value(&import_attempt, "import_id");

        let shown_import = run(Cli::try_parse_from([
            "workvcs",
            "bundle",
            "import-show",
            store,
            "--import",
            &import_id,
        ])
        .expect("parse bundle import-show"))
        .expect("show bundle import attempt");
        assert_eq!(value(&shown_import, "import_id"), import_id);
        assert_eq!(value(&shown_import, "outcome"), "already_present");
        assert_ne!(value(&shown_import, "detail_digest"), "none");
        assert_ne!(value(&shown_import, "detail_size_bytes"), "none");

        let listed_imports =
            run(
                Cli::try_parse_from(["workvcs", "bundle", "import-list", store, "--limit", "1"])
                    .expect("parse bundle import-list"),
            )
            .expect("list bundle import attempts");
        assert_eq!(value(&listed_imports, "imports"), "1");
        assert_eq!(value(&listed_imports, "import[0].import_id"), import_id);
        assert_eq!(
            value(&listed_imports, "import[0].outcome"),
            "already_present"
        );

        let listed_by_bundle = run(Cli::try_parse_from([
            "workvcs",
            "bundle",
            "import-list",
            store,
            "--bundle-digest",
            &value(&import_attempt, "bundle_digest"),
        ])
        .expect("parse bundle import-list bundle filter"))
        .expect("list bundle import attempts by bundle digest");
        assert_eq!(value(&listed_by_bundle, "imports"), "1");
        assert_eq!(value(&listed_by_bundle, "import[0].import_id"), import_id);
    }

    #[test]
    fn cli_applies_same_store_bundle_fast_forward() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let source_path = tempdir.path().join("source.sqlite");
        let old_path = tempdir.path().join("old.sqlite");
        let source_store = source_path.to_str().expect("source path text");
        let old_store = old_path.to_str().expect("old path text");

        run(Cli::try_parse_from([
            "workvcs",
            "init",
            source_store,
            "--display-name",
            "cli-source-store",
        ])
        .expect("parse init"))
        .expect("run init");
        let workspace = run(Cli::try_parse_from([
            "workvcs",
            "workspace",
            "create",
            source_store,
            "--display-name",
            "workspace",
        ])
        .expect("parse workspace"))
        .expect("create workspace");
        let branch = value(&workspace, "branch_id");
        let genesis = value(&workspace, "genesis_commit_id");

        let first = run(Cli::try_parse_from([
            "workvcs",
            "task",
            "create",
            source_store,
            "--branch",
            &branch,
            "--head",
            &genesis,
            "--description",
            "Old branch head task",
        ])
        .expect("parse first task"))
        .expect("create first task");
        fs::copy(&source_path, &old_path).expect("copy old store");

        let second = run(Cli::try_parse_from([
            "workvcs",
            "task",
            "create",
            source_store,
            "--branch",
            &branch,
            "--head",
            &value(&first, "commit_id"),
            "--description",
            "Exported branch head task",
        ])
        .expect("parse second task"))
        .expect("create second task");
        let second_commit = value(&second, "commit_id");

        let export_dir = tempdir.path().join("same-store-bundle");
        run(Cli::try_parse_from([
            "workvcs",
            "bundle",
            "export-dir",
            source_store,
            "--commit",
            &second_commit,
            "--output-dir",
            export_dir.to_str().expect("export dir path"),
        ])
        .expect("parse bundle export-dir"))
        .expect("export bundle directory");

        let preflight = run(Cli::try_parse_from([
            "workvcs",
            "bundle",
            "preflight-dir",
            old_store,
            "--input-dir",
            export_dir.to_str().expect("export dir path"),
        ])
        .expect("parse bundle preflight-dir"))
        .expect("preflight bundle directory");
        assert_eq!(value(&preflight, "action"), "same_store_fast_forward_ready");
        assert_eq!(value(&preflight, "can_apply"), "true");

        let applied = run(Cli::try_parse_from([
            "workvcs",
            "bundle",
            "apply-dir",
            old_store,
            "--input-dir",
            export_dir.to_str().expect("export dir path"),
        ])
        .expect("parse bundle apply-dir"))
        .expect("apply bundle directory");
        assert_eq!(value(&applied, "applied"), "true");
        assert_ne!(value(&applied, "import_id"), "none");
        assert_eq!(
            value(&applied, "outcome"),
            "same_store_fast_forward_applied"
        );
        assert_eq!(value(&applied, "imported_commits"), "1");
        assert_eq!(value(&applied, "imported_entity_versions"), "1");
        assert_eq!(value(&applied, "updated_branch_heads"), "1");

        let branch_head =
            run(
                Cli::try_parse_from(["workvcs", "branch", "head", old_store, "--branch", &branch])
                    .expect("parse branch head"),
            )
            .expect("branch head");
        assert_eq!(value(&branch_head, "head_commit_id"), second_commit);

        let shown_import = run(Cli::try_parse_from([
            "workvcs",
            "bundle",
            "import-show",
            old_store,
            "--import",
            &value(&applied, "import_id"),
        ])
        .expect("parse bundle import-show"))
        .expect("show bundle import attempt");
        assert_eq!(
            value(&shown_import, "outcome"),
            "same_store_fast_forward_applied"
        );
    }

    #[test]
    fn cli_runs_branch_head_and_fork_workflow() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let path = tempdir.path().join("workvcs.sqlite");
        let store = path.to_str().expect("path text");

        run(
            Cli::try_parse_from(["workvcs", "init", store, "--display-name", "cli-store"])
                .expect("parse init"),
        )
        .expect("run init");
        let workspace = run(Cli::try_parse_from([
            "workvcs",
            "workspace",
            "create",
            store,
            "--display-name",
            "workspace",
        ])
        .expect("parse workspace"))
        .expect("create workspace");
        let source_branch = value(&workspace, "branch_id");
        let mut source_head = value(&workspace, "genesis_commit_id");

        let task = run(Cli::try_parse_from([
            "workvcs",
            "task",
            "create",
            store,
            "--branch",
            &source_branch,
            "--head",
            &source_head,
            "--description",
            "Create fork source state",
        ])
        .expect("parse task"))
        .expect("create task");
        source_head = value(&task, "commit_id");
        let source_changeset = value(&task, "changeset_id");

        let source = run(Cli::try_parse_from([
            "workvcs",
            "branch",
            "head",
            store,
            "--branch",
            &source_branch,
        ])
        .expect("parse source head"))
        .expect("source head");
        assert_eq!(value(&source, "head_commit_id"), source_head);
        assert_eq!(value(&source, "head_changeset_id"), source_changeset);
        assert_eq!(value(&source, "head_commit_kind"), "normal");
        assert_eq!(value(&source, "head_operation_type"), "entity.transition");

        let fork = run(Cli::try_parse_from([
            "workvcs",
            "branch",
            "fork",
            store,
            "--from-branch",
            &source_branch,
            "--name",
            "experiment",
        ])
        .expect("parse fork"))
        .expect("fork branch");
        let fork_branch = value(&fork, "branch_id");
        assert_eq!(value(&fork, "source_branch_id"), source_branch);
        assert_eq!(value(&fork, "head_commit_id"), source_head);

        let branches = run(Cli::try_parse_from([
            "workvcs",
            "branch",
            "list",
            store,
            "--workspace",
            &value(&workspace, "workspace_id"),
        ])
        .expect("parse branch list"))
        .expect("list branches");
        assert!(branches.contains("branches=2"));
        assert!(branches.contains("branch.0.branch_name=experiment"));
        assert_eq!(value(&branches, "branch.0.head_commit_id"), source_head);
        assert_eq!(
            value(&branches, "branch.0.head_changeset_id"),
            source_changeset
        );

        let branches_by_name = run(Cli::try_parse_from([
            "workvcs",
            "branch",
            "list",
            store,
            "--workspace",
            &value(&workspace, "workspace_id"),
            "--name",
            "experiment",
        ])
        .expect("parse branch list by name"))
        .expect("list branches by name");
        assert!(branches_by_name.contains("branches=1"));
        assert_eq!(value(&branches_by_name, "branch.0.branch_id"), fork_branch);

        let active_branches = run(Cli::try_parse_from([
            "workvcs",
            "branch",
            "list",
            store,
            "--workspace",
            &value(&workspace, "workspace_id"),
            "--lifecycle-state",
            "active",
        ])
        .expect("parse branch list by lifecycle state"))
        .expect("list branches by lifecycle state");
        assert!(active_branches.contains("branches=2"));

        let active_experiment = run(Cli::try_parse_from([
            "workvcs",
            "branch",
            "list",
            store,
            "--workspace",
            &value(&workspace, "workspace_id"),
            "--name",
            "experiment",
            "--lifecycle-state",
            "active",
        ])
        .expect("parse branch list by name and lifecycle state"))
        .expect("list branches by name and lifecycle state");
        assert!(active_experiment.contains("branches=1"));
        assert_eq!(value(&active_experiment, "branch.0.branch_id"), fork_branch);

        let limited_branches = run(Cli::try_parse_from([
            "workvcs",
            "branch",
            "list",
            store,
            "--workspace",
            &value(&workspace, "workspace_id"),
            "--limit",
            "1",
        ])
        .expect("parse limited branch list"))
        .expect("list limited branches");
        assert!(limited_branches.contains("branches=1"));
        assert_ne!(value(&limited_branches, "branch.0.branch_id"), "");

        let zero_limit_branches = run(Cli::try_parse_from([
            "workvcs",
            "branch",
            "list",
            store,
            "--workspace",
            &value(&workspace, "workspace_id"),
            "--limit",
            "0",
        ])
        .expect("parse zero-limit branch list"));
        assert!(zero_limit_branches.is_err());

        let missing_branch_name = run(Cli::try_parse_from([
            "workvcs",
            "branch",
            "list",
            store,
            "--workspace",
            &value(&workspace, "workspace_id"),
            "--name",
            "missing",
        ])
        .expect("parse branch list by missing name"))
        .expect("list branches by missing name");
        assert!(missing_branch_name.contains("branches=0"));

        let later_source = run(Cli::try_parse_from([
            "workvcs",
            "task",
            "create",
            store,
            "--branch",
            &source_branch,
            "--head",
            &source_head,
            "--description",
            "Advance source after fork",
        ])
        .expect("parse later task"))
        .expect("advance source branch");
        assert_ne!(value(&later_source, "commit_id"), source_head);

        let fork_head = run(Cli::try_parse_from([
            "workvcs",
            "branch",
            "head",
            store,
            "--branch",
            &fork_branch,
        ])
        .expect("parse fork head"))
        .expect("fork head");
        assert_eq!(value(&fork_head, "branch_name"), "experiment");
        assert_eq!(value(&fork_head, "head_commit_id"), source_head);
        assert_eq!(value(&fork_head, "head_changeset_id"), source_changeset);

        let fork_history = run(Cli::try_parse_from([
            "workvcs",
            "history",
            store,
            "--branch",
            &fork_branch,
            "--limit",
            "1",
        ])
        .expect("parse fork history"))
        .expect("fork history");
        assert!(fork_history.contains(&format!("start_commit_id={source_head}")));
        assert!(fork_history.contains("entries=1"));
    }

    #[test]
    fn cli_runs_session_switch_to_forked_branch_workflow() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let path = tempdir.path().join("workvcs.sqlite");
        let store = path.to_str().expect("path text");

        run(
            Cli::try_parse_from(["workvcs", "init", store, "--display-name", "cli-store"])
                .expect("parse init"),
        )
        .expect("run init");
        let workspace = run(Cli::try_parse_from([
            "workvcs",
            "workspace",
            "create",
            store,
            "--display-name",
            "workspace",
        ])
        .expect("parse workspace"))
        .expect("create workspace");
        let workspace_id = value(&workspace, "workspace_id");
        let source_branch = value(&workspace, "branch_id");
        let source_head = value(&workspace, "genesis_commit_id");

        let task = run(Cli::try_parse_from([
            "workvcs",
            "task",
            "create",
            store,
            "--branch",
            &source_branch,
            "--head",
            &source_head,
            "--description",
            "Switch session to fork",
        ])
        .expect("parse task"))
        .expect("create task");
        let task_id = value(&task, "task_entity_id");
        let task_head = value(&task, "commit_id");

        let fork = run(Cli::try_parse_from([
            "workvcs",
            "branch",
            "fork",
            store,
            "--from-branch",
            &source_branch,
            "--name",
            "runtime",
        ])
        .expect("parse fork"))
        .expect("fork branch");
        let fork_branch = value(&fork, "branch_id");
        assert_eq!(value(&fork, "head_commit_id"), task_head);

        let session = run(Cli::try_parse_from([
            "workvcs",
            "session",
            "start",
            store,
            "--workspace",
            &workspace_id,
            "--branch",
            &source_branch,
        ])
        .expect("parse session start"))
        .expect("start session");
        let session_id = value(&session, "session_id");

        let shown_session = run(Cli::try_parse_from([
            "workvcs",
            "session",
            "show",
            store,
            "--session",
            &session_id,
        ])
        .expect("parse session show"))
        .expect("show session");
        assert_eq!(value(&shown_session, "session_id"), session_id);
        assert_eq!(value(&shown_session, "lifecycle_state"), "active");
        assert_eq!(value(&shown_session, "metadata_json"), "{}");
        assert_eq!(value(&shown_session, "active_workspace_id"), workspace_id);
        assert_eq!(value(&shown_session, "active_branch_id"), source_branch);
        assert_eq!(value(&shown_session, "context_workspaces"), "1");
        assert_eq!(value(&shown_session, "focus_entity_id"), "none");
        assert_eq!(value(&shown_session, "session_diff_id"), "none");

        let claim = run(Cli::try_parse_from([
            "workvcs",
            "claim",
            "task",
            store,
            "--session",
            &session_id,
            "--task",
            &task_id,
        ])
        .expect("parse claim"))
        .expect("claim source task");
        assert!(claim.contains("lifecycle_state=active"));

        let switched = run(Cli::try_parse_from([
            "workvcs",
            "session",
            "switch",
            store,
            "--session",
            &session_id,
            "--workspace",
            &workspace_id,
            "--branch",
            &fork_branch,
            "--focus",
            &task_id,
        ])
        .expect("parse switch"))
        .expect("switch session");
        assert_eq!(value(&switched, "previous_branch_id"), source_branch);
        assert_eq!(value(&switched, "branch_id"), fork_branch);
        assert_eq!(value(&switched, "released_claims"), "1");
        assert_eq!(value(&switched, "focus_entity_id"), task_id);
        assert!(switched.contains("lifecycle_state=active"));

        let shown_switched_session = run(Cli::try_parse_from([
            "workvcs",
            "session",
            "show",
            store,
            "--session",
            &session_id,
        ])
        .expect("parse switched session show"))
        .expect("show switched session");
        assert_eq!(value(&shown_switched_session, "session_id"), session_id);
        assert_eq!(
            value(&shown_switched_session, "active_branch_id"),
            fork_branch
        );
        assert_eq!(value(&shown_switched_session, "focus_entity_id"), task_id);
        assert_eq!(value(&shown_switched_session, "focus_path_entries"), "0");

        let sessions =
            run(Cli::try_parse_from(["workvcs", "session", "list", store])
                .expect("parse session list"))
            .expect("list sessions");
        assert_eq!(value(&sessions, "sessions"), "1");
        assert_eq!(value(&sessions, "session.0.session_id"), session_id);
        assert_eq!(value(&sessions, "session.0.lifecycle_state"), "active");
        assert_eq!(value(&sessions, "session.0.active_branch_id"), fork_branch);
        assert_eq!(value(&sessions, "session.0.focus_entity_id"), task_id);

        let focused_sessions =
            run(
                Cli::try_parse_from(["workvcs", "session", "list", store, "--focus", &task_id])
                    .expect("parse focused session list"),
            )
            .expect("list focused sessions");
        assert_eq!(value(&focused_sessions, "sessions"), "1");
        assert_eq!(value(&focused_sessions, "session.0.session_id"), session_id);

        let missing_focused_sessions = run(Cli::try_parse_from([
            "workvcs",
            "session",
            "list",
            store,
            "--focus",
            "018b4ed6-0e2f-7000-8000-000000000003",
        ])
        .expect("parse missing focused session list"))
        .expect("list missing focused sessions");
        assert_eq!(value(&missing_focused_sessions, "sessions"), "0");

        let active_sessions = run(Cli::try_parse_from([
            "workvcs",
            "session",
            "list",
            store,
            "--lifecycle",
            "active",
        ])
        .expect("parse active session list"))
        .expect("list active sessions");
        assert_eq!(value(&active_sessions, "sessions"), "1");
        assert_eq!(value(&active_sessions, "session.0.session_id"), session_id);

        let workspace_sessions = run(Cli::try_parse_from([
            "workvcs",
            "session",
            "list",
            store,
            "--workspace",
            &workspace_id,
        ])
        .expect("parse workspace session list"))
        .expect("list workspace sessions");
        assert_eq!(value(&workspace_sessions, "sessions"), "1");
        assert_eq!(
            value(&workspace_sessions, "session.0.session_id"),
            session_id
        );

        let branch_sessions = run(Cli::try_parse_from([
            "workvcs",
            "session",
            "list",
            store,
            "--branch",
            &fork_branch,
        ])
        .expect("parse branch session list"))
        .expect("list branch sessions");
        assert_eq!(value(&branch_sessions, "sessions"), "1");
        assert_eq!(value(&branch_sessions, "session.0.session_id"), session_id);

        let old_branch_sessions = run(Cli::try_parse_from([
            "workvcs",
            "session",
            "list",
            store,
            "--branch",
            &source_branch,
        ])
        .expect("parse old branch session list"))
        .expect("list old branch sessions");
        assert_eq!(value(&old_branch_sessions, "sessions"), "0");

        let combined_sessions = run(Cli::try_parse_from([
            "workvcs",
            "session",
            "list",
            store,
            "--lifecycle",
            "active",
            "--workspace",
            &workspace_id,
            "--branch",
            &fork_branch,
        ])
        .expect("parse combined session list"))
        .expect("list combined sessions");
        assert_eq!(value(&combined_sessions, "sessions"), "1");
        assert_eq!(
            value(&combined_sessions, "session.0.session_id"),
            session_id
        );

        let ended_sessions =
            run(
                Cli::try_parse_from(["workvcs", "session", "list", store, "--lifecycle", "ended"])
                    .expect("parse ended session list"),
            )
            .expect("list ended sessions");
        assert_eq!(value(&ended_sessions, "sessions"), "0");

        let cleared_focus = run(Cli::try_parse_from([
            "workvcs",
            "session",
            "focus-clear",
            store,
            "--session",
            &session_id,
        ])
        .expect("parse focus clear"))
        .expect("clear session focus");
        assert_eq!(value(&cleared_focus, "session_id"), session_id);
        assert_eq!(value(&cleared_focus, "focus_entity_id"), "none");

        let shown_cleared_session = run(Cli::try_parse_from([
            "workvcs",
            "session",
            "show",
            store,
            "--session",
            &session_id,
        ])
        .expect("parse cleared session show"))
        .expect("show cleared session");
        assert_eq!(value(&shown_cleared_session, "focus_entity_id"), "none");

        let focused_session = run(Cli::try_parse_from([
            "workvcs",
            "session",
            "focus-set",
            store,
            "--session",
            &session_id,
            "--focus",
            &task_id,
        ])
        .expect("parse focus set"))
        .expect("set session focus");
        assert_eq!(value(&focused_session, "session_id"), session_id);
        assert_eq!(value(&focused_session, "focus_entity_id"), task_id);

        let runnable = run(Cli::try_parse_from([
            "workvcs",
            "runnable",
            "tasks",
            store,
            "--session",
            &session_id,
        ])
        .expect("parse runnable"))
        .expect("runnable on fork");
        assert!(runnable.contains("candidates=1"));
        assert!(runnable.contains(&format!("candidate.0.task_entity_id={task_id}")));
        assert!(runnable.contains("candidate.0.claim=unclaimed"));
    }

    #[test]
    fn cli_runs_claim_next_workflow() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let path = tempdir.path().join("workvcs.sqlite");
        let store = path.to_str().expect("path text");

        run(
            Cli::try_parse_from(["workvcs", "init", store, "--display-name", "cli-store"])
                .expect("parse init"),
        )
        .expect("run init");
        let workspace = run(Cli::try_parse_from([
            "workvcs",
            "workspace",
            "create",
            store,
            "--display-name",
            "workspace",
        ])
        .expect("parse workspace"))
        .expect("create workspace");
        let workspace_id = value(&workspace, "workspace_id");
        let branch = value(&workspace, "branch_id");
        let mut head = value(&workspace, "genesis_commit_id");

        let first = run(Cli::try_parse_from([
            "workvcs",
            "task",
            "create",
            store,
            "--branch",
            &branch,
            "--head",
            &head,
            "--description",
            "First CLI next task",
        ])
        .expect("parse first task"))
        .expect("create first task");
        head = value(&first, "commit_id");
        let second = run(Cli::try_parse_from([
            "workvcs",
            "task",
            "create",
            store,
            "--branch",
            &branch,
            "--head",
            &head,
            "--description",
            "Second CLI next task",
        ])
        .expect("parse second task"))
        .expect("create second task");

        let session = run(Cli::try_parse_from([
            "workvcs",
            "session",
            "start",
            store,
            "--workspace",
            &workspace_id,
            "--branch",
            &branch,
        ])
        .expect("parse session start"))
        .expect("start session");
        let session_id = value(&session, "session_id");

        let claimed =
            run(
                Cli::try_parse_from(["workvcs", "claim", "next", store, "--session", &session_id])
                    .expect("parse claim next"),
            )
            .expect("claim next");
        assert_eq!(value(&claimed, "selected"), "true");
        assert!(claimed.contains("claim_id="));
        assert!(claimed.contains("lifecycle_state=active"));
        let selected_task = value(&claimed, "task_entity_id");
        assert!(
            selected_task == value(&first, "task_entity_id")
                || selected_task == value(&second, "task_entity_id")
        );

        let runnable = run(Cli::try_parse_from([
            "workvcs",
            "runnable",
            "tasks",
            store,
            "--session",
            &session_id,
        ])
        .expect("parse runnable"))
        .expect("runnable after claim next");
        assert!(runnable.contains(&format!("candidate.0.task_entity_id={selected_task}")));
        assert!(runnable.contains("candidate.0.claim=claimed_by_session:"));
    }

    #[test]
    fn cli_runs_minimal_semantic_workflow_through_engine() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let path = tempdir.path().join("workvcs.sqlite");
        let store = path.to_str().expect("path text");

        run(
            Cli::try_parse_from(["workvcs", "init", store, "--display-name", "cli-store"])
                .expect("parse init"),
        )
        .expect("run init");

        let workspace = run(Cli::try_parse_from([
            "workvcs",
            "workspace",
            "create",
            store,
            "--display-name",
            "workspace",
        ])
        .expect("parse workspace"))
        .expect("create workspace");
        let branch = value(&workspace, "branch_id");
        let mut head = value(&workspace, "genesis_commit_id");

        let task = run(Cli::try_parse_from([
            "workvcs",
            "task",
            "create",
            store,
            "--branch",
            &branch,
            "--head",
            &head,
            "--description",
            "Ship CLI workflow",
        ])
        .expect("parse task"))
        .expect("create task");
        let task_id = value(&task, "task_entity_id");
        head = value(&task, "commit_id");

        let criterion = run(Cli::try_parse_from([
            "workvcs",
            "ac",
            "create",
            store,
            "--branch",
            &branch,
            "--head",
            &head,
            "--task",
            &task_id,
            "--task-version",
            &value(&task, "task_entity_version_id"),
            "--local-key",
            "AC-1",
            "--statement",
            "The CLI can run the minimal semantic workflow.",
        ])
        .expect("parse ac"))
        .expect("create ac");
        let criterion_id = value(&criterion, "acceptance_criterion_entity_id");
        let current_task_version = value(&criterion, "task_entity_version_id");
        head = value(&criterion, "commit_id");

        let status = run(Cli::try_parse_from([
            "workvcs",
            "ac",
            "status",
            store,
            "--branch",
            &branch,
            "--criterion",
            &criterion_id,
        ])
        .expect("parse status"))
        .expect("initial status");
        assert_eq!(status, "status=unverified\n");

        let status_at_commit = run(Cli::try_parse_from([
            "workvcs",
            "ac",
            "status",
            store,
            "--commit",
            &head,
            "--criterion",
            &criterion_id,
        ])
        .expect("parse status at commit"))
        .expect("initial status at commit");
        assert_eq!(status_at_commit, "status=unverified\n");

        let status_without_target = Cli::try_parse_from([
            "workvcs",
            "ac",
            "status",
            store,
            "--criterion",
            &criterion_id,
        ]);
        assert!(status_without_target.is_err());

        let status_with_both_targets = Cli::try_parse_from([
            "workvcs",
            "ac",
            "status",
            store,
            "--branch",
            &branch,
            "--commit",
            &head,
            "--criterion",
            &criterion_id,
        ]);
        assert!(status_with_both_targets.is_err());

        let verification = run(Cli::try_parse_from([
            "workvcs",
            "verification",
            "record",
            store,
            "--branch",
            &branch,
            "--head",
            &head,
            "--acceptance-criterion",
            &criterion_id,
            "--result",
            "passed",
            "--method",
            "manual-review",
        ])
        .expect("parse verification"))
        .expect("record verification");
        head = value(&verification, "commit_id");

        let verified = run(Cli::try_parse_from([
            "workvcs",
            "ac",
            "status",
            store,
            "--branch",
            &branch,
            "--criterion",
            &criterion_id,
        ])
        .expect("parse verified"))
        .expect("verified status");
        assert_eq!(verified, "status=verified\n");

        let transition = run(Cli::try_parse_from([
            "workvcs",
            "task",
            "transition",
            store,
            "--branch",
            &branch,
            "--head",
            &head,
            "--task",
            &task_id,
            "--task-version",
            &current_task_version,
            "--status",
            "done",
            "--outcome",
            "completed",
        ])
        .expect("parse transition"))
        .expect("transition task");
        assert!(transition.contains("status=done"));
    }

    #[test]
    fn cli_shows_and_lists_task_snapshots_at_branch_or_commit() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let path = tempdir.path().join("workvcs.sqlite");
        let store = path.to_str().expect("path text");

        run(
            Cli::try_parse_from(["workvcs", "init", store, "--display-name", "cli-store"])
                .expect("parse init"),
        )
        .expect("run init");
        let workspace = run(Cli::try_parse_from([
            "workvcs",
            "workspace",
            "create",
            store,
            "--display-name",
            "workspace",
        ])
        .expect("parse workspace"))
        .expect("create workspace");
        let branch = value(&workspace, "branch_id");
        let genesis = value(&workspace, "genesis_commit_id");

        let empty_tasks =
            run(
                Cli::try_parse_from(["workvcs", "task", "list", store, "--commit", &genesis])
                    .expect("parse empty task list"),
            )
            .expect("list empty tasks");
        assert_eq!(value(&empty_tasks, "tasks"), "0");

        let task = run(Cli::try_parse_from([
            "workvcs",
            "task",
            "create",
            store,
            "--branch",
            &branch,
            "--head",
            &genesis,
            "--description",
            "Inspect task snapshots",
            "--priority",
            "7",
        ])
        .expect("parse task create"))
        .expect("create task");
        let task_id = value(&task, "task_entity_id");

        let task_at_create = run(Cli::try_parse_from([
            "workvcs",
            "task",
            "show",
            store,
            "--commit",
            &value(&task, "commit_id"),
            "--task",
            &task_id,
        ])
        .expect("parse task show at commit"))
        .expect("show task at commit");
        assert_eq!(value(&task_at_create, "status"), "pending");
        assert_eq!(
            value(&task_at_create, "task_entity_version_id"),
            value(&task, "task_entity_version_id")
        );
        assert_eq!(
            value(&task_at_create, "description_json"),
            "\"Inspect task snapshots\""
        );
        assert_eq!(value(&task_at_create, "outcome_json"), "null");
        assert_eq!(value(&task_at_create, "priority"), "7");

        let tasks_at_create = run(Cli::try_parse_from([
            "workvcs",
            "task",
            "list",
            store,
            "--commit",
            &value(&task, "commit_id"),
        ])
        .expect("parse task list at commit"))
        .expect("list tasks at commit");
        assert_eq!(value(&tasks_at_create, "tasks"), "1");
        assert_eq!(value(&tasks_at_create, "task.0.task_entity_id"), task_id);
        assert_eq!(value(&tasks_at_create, "task.0.status"), "pending");

        let pending_tasks_at_create = run(Cli::try_parse_from([
            "workvcs",
            "task",
            "list",
            store,
            "--commit",
            &value(&task, "commit_id"),
            "--status",
            "pending",
        ])
        .expect("parse pending task list at commit"))
        .expect("list pending tasks at commit");
        assert_eq!(value(&pending_tasks_at_create, "tasks"), "1");
        assert_eq!(
            value(&pending_tasks_at_create, "task.0.task_entity_id"),
            task_id
        );

        let blocked_tasks_at_create = run(Cli::try_parse_from([
            "workvcs",
            "task",
            "list",
            store,
            "--commit",
            &value(&task, "commit_id"),
            "--status",
            "blocked",
        ])
        .expect("parse blocked task list at commit"))
        .expect("list blocked tasks at commit");
        assert_eq!(value(&blocked_tasks_at_create, "tasks"), "0");

        let blocked_task = run(Cli::try_parse_from([
            "workvcs",
            "task",
            "transition",
            store,
            "--branch",
            &branch,
            "--head",
            &value(&task, "commit_id"),
            "--task",
            &task_id,
            "--task-version",
            &value(&task, "task_entity_version_id"),
            "--status",
            "blocked",
        ])
        .expect("parse task transition"))
        .expect("block task");

        let task_at_branch = run(Cli::try_parse_from([
            "workvcs", "task", "show", store, "--branch", &branch, "--task", &task_id,
        ])
        .expect("parse task show at branch"))
        .expect("show task at branch");
        assert_eq!(
            value(&task_at_branch, "commit_id"),
            value(&blocked_task, "commit_id")
        );
        assert_eq!(value(&task_at_branch, "status"), "blocked");
        assert_eq!(
            value(&task_at_branch, "task_entity_version_id"),
            value(&blocked_task, "task_entity_version_id")
        );

        let tasks_at_branch =
            run(
                Cli::try_parse_from(["workvcs", "task", "list", store, "--branch", &branch])
                    .expect("parse task list at branch"),
            )
            .expect("list tasks at branch");
        assert_eq!(
            value(&tasks_at_branch, "commit_id"),
            value(&blocked_task, "commit_id")
        );
        assert_eq!(value(&tasks_at_branch, "tasks"), "1");
        assert_eq!(value(&tasks_at_branch, "task.0.status"), "blocked");
        assert_eq!(value(&tasks_at_branch, "task.0.priority"), "7");

        let blocked_tasks_at_branch = run(Cli::try_parse_from([
            "workvcs", "task", "list", store, "--branch", &branch, "--status", "blocked",
        ])
        .expect("parse blocked task list at branch"))
        .expect("list blocked tasks at branch");
        assert_eq!(value(&blocked_tasks_at_branch, "tasks"), "1");
        assert_eq!(
            value(&blocked_tasks_at_branch, "task.0.task_entity_id"),
            task_id
        );

        let limited_blocked_tasks_at_branch = run(Cli::try_parse_from([
            "workvcs", "task", "list", store, "--branch", &branch, "--status", "blocked",
            "--limit", "1",
        ])
        .expect("parse limited blocked task list at branch"))
        .expect("list limited blocked tasks at branch");
        assert_eq!(value(&limited_blocked_tasks_at_branch, "tasks"), "1");
        assert_eq!(
            value(&limited_blocked_tasks_at_branch, "task.0.task_entity_id"),
            task_id
        );

        let zero_limit_tasks_at_branch = run(Cli::try_parse_from([
            "workvcs", "task", "list", store, "--branch", &branch, "--limit", "0",
        ])
        .expect("parse zero-limit task list at branch"));
        assert!(zero_limit_tasks_at_branch.is_err());

        let pending_tasks_at_branch = run(Cli::try_parse_from([
            "workvcs", "task", "list", store, "--branch", &branch, "--status", "pending",
        ])
        .expect("parse pending task list at branch"))
        .expect("list pending tasks at branch");
        assert_eq!(value(&pending_tasks_at_branch, "tasks"), "0");

        let superseded_tasks_at_branch = run(Cli::try_parse_from([
            "workvcs",
            "task",
            "list",
            store,
            "--branch",
            &branch,
            "--status",
            "superseded",
        ])
        .expect("parse superseded task list at branch"))
        .expect("list superseded tasks at branch");
        assert_eq!(value(&superseded_tasks_at_branch, "tasks"), "0");
    }

    #[test]
    fn cli_shows_acceptance_criterion_and_verification_requirement_snapshots() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let path = tempdir.path().join("workvcs.sqlite");
        let store = path.to_str().expect("path text");

        run(
            Cli::try_parse_from(["workvcs", "init", store, "--display-name", "cli-store"])
                .expect("parse init"),
        )
        .expect("run init");
        let workspace = run(Cli::try_parse_from([
            "workvcs",
            "workspace",
            "create",
            store,
            "--display-name",
            "workspace",
        ])
        .expect("parse workspace"))
        .expect("create workspace");
        let branch = value(&workspace, "branch_id");
        let genesis = value(&workspace, "genesis_commit_id");

        let task = run(Cli::try_parse_from([
            "workvcs",
            "task",
            "create",
            store,
            "--branch",
            &branch,
            "--head",
            &genesis,
            "--description",
            "Inspect AC and VR snapshots",
        ])
        .expect("parse task create"))
        .expect("create task");

        let criterion = run(Cli::try_parse_from([
            "workvcs",
            "ac",
            "create",
            store,
            "--branch",
            &branch,
            "--head",
            &value(&task, "commit_id"),
            "--task",
            &value(&task, "task_entity_id"),
            "--task-version",
            &value(&task, "task_entity_version_id"),
            "--local-key",
            "AC-1",
            "--statement",
            "The AC snapshot is visible.",
        ])
        .expect("parse ac create"))
        .expect("create ac");
        let criterion_id = value(&criterion, "acceptance_criterion_entity_id");

        let criterion_at_create = run(Cli::try_parse_from([
            "workvcs",
            "ac",
            "show",
            store,
            "--commit",
            &value(&criterion, "commit_id"),
            "--criterion",
            &criterion_id,
        ])
        .expect("parse ac show at commit"))
        .expect("show ac at commit");
        assert_eq!(value(&criterion_at_create, "classification"), "required");
        assert_eq!(
            value(
                &criterion_at_create,
                "acceptance_criterion_entity_version_id"
            ),
            value(&criterion, "acceptance_criterion_entity_version_id")
        );
        assert_eq!(
            value(&criterion_at_create, "statement_json"),
            "\"The AC snapshot is visible.\""
        );
        assert_eq!(
            value(&criterion_at_create, "verification_requirements"),
            "0"
        );

        let requirement = run(Cli::try_parse_from([
            "workvcs",
            "vr",
            "create",
            store,
            "--branch",
            &branch,
            "--head",
            &value(&criterion, "commit_id"),
            "--criterion",
            &criterion_id,
            "--criterion-version",
            &value(&criterion, "acceptance_criterion_entity_version_id"),
            "--local-key",
            "VR-1",
            "--statement",
            "Manual review must pass.",
        ])
        .expect("parse vr create"))
        .expect("create vr");
        let requirement_id = value(&requirement, "verification_requirement_entity_id");

        let requirement_at_branch = run(Cli::try_parse_from([
            "workvcs",
            "vr",
            "show",
            store,
            "--branch",
            &branch,
            "--requirement",
            &requirement_id,
        ])
        .expect("parse vr show at branch"))
        .expect("show vr at branch");
        assert_eq!(
            value(&requirement_at_branch, "commit_id"),
            value(&requirement, "commit_id")
        );
        assert_eq!(value(&requirement_at_branch, "local_key"), "VR-1");
        assert_eq!(
            value(&requirement_at_branch, "acceptance_criterion_entity_id"),
            criterion_id
        );
        assert_eq!(
            value(&requirement_at_branch, "statement_json"),
            "\"Manual review must pass.\""
        );

        let criterion_at_branch = run(Cli::try_parse_from([
            "workvcs",
            "ac",
            "show",
            store,
            "--branch",
            &branch,
            "--criterion",
            &value(&requirement, "acceptance_criterion_entity_id"),
        ])
        .expect("parse ac show at branch"))
        .expect("show ac at branch");
        assert_eq!(
            value(
                &criterion_at_branch,
                "acceptance_criterion_entity_version_id"
            ),
            value(&requirement, "acceptance_criterion_entity_version_id")
        );
        assert_eq!(
            value(&criterion_at_branch, "verification_requirements"),
            "1"
        );
        assert_eq!(
            value(
                &criterion_at_branch,
                "verification_requirement.0.verification_requirement_entity_id"
            ),
            requirement_id
        );
    }

    #[test]
    fn cli_lists_acceptance_criteria_and_verification_requirements() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let path = tempdir.path().join("workvcs.sqlite");
        let store = path.to_str().expect("path text");

        run(
            Cli::try_parse_from(["workvcs", "init", store, "--display-name", "cli-store"])
                .expect("parse init"),
        )
        .expect("run init");
        let workspace = run(Cli::try_parse_from([
            "workvcs",
            "workspace",
            "create",
            store,
            "--display-name",
            "workspace",
        ])
        .expect("parse workspace"))
        .expect("create workspace");
        let branch = value(&workspace, "branch_id");
        let genesis = value(&workspace, "genesis_commit_id");

        let empty_criteria =
            run(
                Cli::try_parse_from(["workvcs", "ac", "list", store, "--commit", &genesis])
                    .expect("parse empty ac list"),
            )
            .expect("list empty ac");
        assert_eq!(value(&empty_criteria, "criteria"), "0");

        let empty_requirements =
            run(
                Cli::try_parse_from(["workvcs", "vr", "list", store, "--commit", &genesis])
                    .expect("parse empty vr list"),
            )
            .expect("list empty vr");
        assert_eq!(value(&empty_requirements, "requirements"), "0");

        let task = run(Cli::try_parse_from([
            "workvcs",
            "task",
            "create",
            store,
            "--branch",
            &branch,
            "--head",
            &genesis,
            "--description",
            "List AC and VR snapshots",
        ])
        .expect("parse task create"))
        .expect("create task");

        let criterion = run(Cli::try_parse_from([
            "workvcs",
            "ac",
            "create",
            store,
            "--branch",
            &branch,
            "--head",
            &value(&task, "commit_id"),
            "--task",
            &value(&task, "task_entity_id"),
            "--task-version",
            &value(&task, "task_entity_version_id"),
            "--local-key",
            "AC-1",
            "--statement",
            "The AC list is visible.",
        ])
        .expect("parse ac create"))
        .expect("create ac");
        let criterion_id = value(&criterion, "acceptance_criterion_entity_id");

        let criteria_at_create = run(Cli::try_parse_from([
            "workvcs",
            "ac",
            "list",
            store,
            "--commit",
            &value(&criterion, "commit_id"),
        ])
        .expect("parse ac list at commit"))
        .expect("list ac at commit");
        assert_eq!(value(&criteria_at_create, "criteria"), "1");
        assert_eq!(
            value(
                &criteria_at_create,
                "criterion.0.acceptance_criterion_entity_id"
            ),
            criterion_id
        );
        assert_eq!(value(&criteria_at_create, "criterion.0.local_key"), "AC-1");
        assert_eq!(
            value(&criteria_at_create, "criterion.0.verification_requirements"),
            "0"
        );

        let requirement = run(Cli::try_parse_from([
            "workvcs",
            "vr",
            "create",
            store,
            "--branch",
            &branch,
            "--head",
            &value(&criterion, "commit_id"),
            "--criterion",
            &criterion_id,
            "--criterion-version",
            &value(&criterion, "acceptance_criterion_entity_version_id"),
            "--local-key",
            "VR-1",
            "--statement",
            "The VR list is visible.",
        ])
        .expect("parse vr create"))
        .expect("create vr");
        let requirement_id = value(&requirement, "verification_requirement_entity_id");

        let requirements_before_create = run(Cli::try_parse_from([
            "workvcs",
            "vr",
            "list",
            store,
            "--commit",
            &value(&criterion, "commit_id"),
        ])
        .expect("parse vr list before create"))
        .expect("list vr before create");
        assert_eq!(value(&requirements_before_create, "requirements"), "0");

        let requirements_at_branch =
            run(
                Cli::try_parse_from(["workvcs", "vr", "list", store, "--branch", &branch])
                    .expect("parse vr list at branch"),
            )
            .expect("list vr at branch");
        assert_eq!(
            value(&requirements_at_branch, "commit_id"),
            value(&requirement, "commit_id")
        );
        assert_eq!(value(&requirements_at_branch, "requirements"), "1");
        assert_eq!(
            value(
                &requirements_at_branch,
                "requirement.0.verification_requirement_entity_id"
            ),
            requirement_id
        );
        assert_eq!(
            value(&requirements_at_branch, "requirement.0.local_key"),
            "VR-1"
        );

        let criteria_at_branch =
            run(
                Cli::try_parse_from(["workvcs", "ac", "list", store, "--branch", &branch])
                    .expect("parse ac list at branch"),
            )
            .expect("list ac at branch");
        assert_eq!(
            value(&criteria_at_branch, "commit_id"),
            value(&requirement, "commit_id")
        );
        assert_eq!(value(&criteria_at_branch, "criteria"), "1");
        assert_eq!(
            value(&criteria_at_branch, "criterion.0.verification_requirements"),
            "1"
        );
        assert_eq!(
            value(&criteria_at_branch, "criterion.0.statement_json"),
            "\"The AC list is visible.\""
        );

        let requirements_for_criterion = run(Cli::try_parse_from([
            "workvcs",
            "vr",
            "list",
            store,
            "--branch",
            &branch,
            "--criterion",
            &criterion_id,
        ])
        .expect("parse vr list by criterion"))
        .expect("list vr by criterion");
        assert_eq!(value(&requirements_for_criterion, "requirements"), "1");
        assert_eq!(
            value(
                &requirements_for_criterion,
                "requirement.0.verification_requirement_entity_id"
            ),
            requirement_id
        );

        let limited_requirements_for_criterion = run(Cli::try_parse_from([
            "workvcs",
            "vr",
            "list",
            store,
            "--branch",
            &branch,
            "--criterion",
            &criterion_id,
            "--limit",
            "1",
        ])
        .expect("parse limited vr list by criterion"))
        .expect("list limited vr by criterion");
        assert_eq!(
            value(&limited_requirements_for_criterion, "requirements"),
            "1"
        );
        assert_eq!(
            value(
                &limited_requirements_for_criterion,
                "requirement.0.verification_requirement_entity_id"
            ),
            requirement_id
        );

        let zero_limit_requirements = run(Cli::try_parse_from([
            "workvcs", "vr", "list", store, "--branch", &branch, "--limit", "0",
        ])
        .expect("parse zero-limit vr list"));
        assert!(zero_limit_requirements.is_err());

        let requirements_for_local_key = run(Cli::try_parse_from([
            "workvcs",
            "vr",
            "list",
            store,
            "--branch",
            &branch,
            "--local-key",
            "VR-1",
        ])
        .expect("parse vr list by local key"))
        .expect("list vr by local key");
        assert_eq!(value(&requirements_for_local_key, "requirements"), "1");

        let requirements_for_missing_local_key = run(Cli::try_parse_from([
            "workvcs",
            "vr",
            "list",
            store,
            "--branch",
            &branch,
            "--criterion",
            &criterion_id,
            "--local-key",
            "VR-404",
        ])
        .expect("parse vr list by missing local key"))
        .expect("list vr by missing local key");
        assert_eq!(
            value(&requirements_for_missing_local_key, "requirements"),
            "0"
        );

        let task_after_requirement = run(Cli::try_parse_from([
            "workvcs",
            "task",
            "show",
            store,
            "--branch",
            &branch,
            "--task",
            &value(&task, "task_entity_id"),
        ])
        .expect("parse task show after requirement"))
        .expect("show task after requirement");
        let optional_criterion = run(Cli::try_parse_from([
            "workvcs",
            "ac",
            "create",
            store,
            "--branch",
            &branch,
            "--head",
            &value(&requirement, "commit_id"),
            "--task",
            &value(&task, "task_entity_id"),
            "--task-version",
            &value(&task_after_requirement, "task_entity_version_id"),
            "--local-key",
            "AC-2",
            "--statement",
            "The optional AC list filter is visible.",
            "--classification",
            "optional",
        ])
        .expect("parse optional ac create"))
        .expect("create optional ac");
        let optional_criterion_id = value(&optional_criterion, "acceptance_criterion_entity_id");

        let required_criteria = run(Cli::try_parse_from([
            "workvcs",
            "ac",
            "list",
            store,
            "--branch",
            &branch,
            "--classification",
            "required",
        ])
        .expect("parse required ac list"))
        .expect("list required ac");
        assert_eq!(value(&required_criteria, "criteria"), "1");
        assert_eq!(
            value(
                &required_criteria,
                "criterion.0.acceptance_criterion_entity_id"
            ),
            criterion_id
        );

        let optional_criteria = run(Cli::try_parse_from([
            "workvcs",
            "ac",
            "list",
            store,
            "--branch",
            &branch,
            "--classification",
            "optional",
        ])
        .expect("parse optional ac list"))
        .expect("list optional ac");
        assert_eq!(value(&optional_criteria, "criteria"), "1");
        assert_eq!(
            value(
                &optional_criteria,
                "criterion.0.acceptance_criterion_entity_id"
            ),
            optional_criterion_id
        );

        let task_criteria = run(Cli::try_parse_from([
            "workvcs",
            "ac",
            "list",
            store,
            "--branch",
            &branch,
            "--task",
            &value(&task, "task_entity_id"),
        ])
        .expect("parse ac list by task"))
        .expect("list ac by task");
        assert_eq!(value(&task_criteria, "criteria"), "2");

        let limited_task_criteria = run(Cli::try_parse_from([
            "workvcs",
            "ac",
            "list",
            store,
            "--branch",
            &branch,
            "--task",
            &value(&task, "task_entity_id"),
            "--limit",
            "1",
        ])
        .expect("parse limited ac list by task"))
        .expect("list limited ac by task");
        assert_eq!(value(&limited_task_criteria, "criteria"), "1");
        assert_ne!(
            value(
                &limited_task_criteria,
                "criterion.0.acceptance_criterion_entity_id"
            ),
            ""
        );

        let zero_limit_task_criteria = run(Cli::try_parse_from([
            "workvcs", "ac", "list", store, "--branch", &branch, "--limit", "0",
        ])
        .expect("parse zero-limit ac list"));
        assert!(zero_limit_task_criteria.is_err());

        let optional_task_criteria = run(Cli::try_parse_from([
            "workvcs",
            "ac",
            "list",
            store,
            "--branch",
            &branch,
            "--task",
            &value(&task, "task_entity_id"),
            "--classification",
            "optional",
        ])
        .expect("parse ac list by task and classification"))
        .expect("list ac by task and classification");
        assert_eq!(value(&optional_task_criteria, "criteria"), "1");
        assert_eq!(
            value(
                &optional_task_criteria,
                "criterion.0.acceptance_criterion_entity_id"
            ),
            optional_criterion_id
        );

        let requirements_for_optional_criterion = run(Cli::try_parse_from([
            "workvcs",
            "vr",
            "list",
            store,
            "--branch",
            &branch,
            "--criterion",
            &optional_criterion_id,
        ])
        .expect("parse vr list by optional criterion"))
        .expect("list vr by optional criterion");
        assert_eq!(
            value(&requirements_for_optional_criterion, "requirements"),
            "0"
        );
    }

    #[test]
    fn cli_revises_acceptance_criteria_and_verification_requirements() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let path = tempdir.path().join("workvcs.sqlite");
        let store = path.to_str().expect("path text");

        run(
            Cli::try_parse_from(["workvcs", "init", store, "--display-name", "cli-store"])
                .expect("parse init"),
        )
        .expect("run init");
        let workspace = run(Cli::try_parse_from([
            "workvcs",
            "workspace",
            "create",
            store,
            "--display-name",
            "workspace",
        ])
        .expect("parse workspace"))
        .expect("create workspace");
        let branch = value(&workspace, "branch_id");
        let genesis = value(&workspace, "genesis_commit_id");

        let task = run(Cli::try_parse_from([
            "workvcs",
            "task",
            "create",
            store,
            "--branch",
            &branch,
            "--head",
            &genesis,
            "--description",
            "Revise AC and VR snapshots",
        ])
        .expect("parse task create"))
        .expect("create task");

        let criterion = run(Cli::try_parse_from([
            "workvcs",
            "ac",
            "create",
            store,
            "--branch",
            &branch,
            "--head",
            &value(&task, "commit_id"),
            "--task",
            &value(&task, "task_entity_id"),
            "--task-version",
            &value(&task, "task_entity_version_id"),
            "--local-key",
            "AC-1",
            "--statement",
            "Old acceptance criterion statement.",
            "--classification",
            "optional",
        ])
        .expect("parse ac create"))
        .expect("create ac");
        let criterion_id = value(&criterion, "acceptance_criterion_entity_id");

        let revised_criterion = run(Cli::try_parse_from([
            "workvcs",
            "ac",
            "revise",
            store,
            "--branch",
            &branch,
            "--head",
            &value(&criterion, "commit_id"),
            "--criterion",
            &criterion_id,
            "--criterion-version",
            &value(&criterion, "acceptance_criterion_entity_version_id"),
            "--statement",
            "New acceptance criterion statement.",
        ])
        .expect("parse ac revise"))
        .expect("revise ac");
        assert_eq!(
            value(
                &revised_criterion,
                "previous_acceptance_criterion_entity_version_id"
            ),
            value(&criterion, "acceptance_criterion_entity_version_id")
        );
        assert_ne!(
            value(&revised_criterion, "acceptance_criterion_entity_version_id"),
            value(&criterion, "acceptance_criterion_entity_version_id")
        );
        assert_eq!(
            value(&revised_criterion, "previous_classification"),
            "optional"
        );
        assert_eq!(value(&revised_criterion, "classification"), "optional");
        assert_eq!(
            value(&revised_criterion, "previous_statement_json"),
            "\"Old acceptance criterion statement.\""
        );
        assert_eq!(
            value(&revised_criterion, "statement_json"),
            "\"New acceptance criterion statement.\""
        );

        let old_criterion = run(Cli::try_parse_from([
            "workvcs",
            "ac",
            "show",
            store,
            "--commit",
            &value(&criterion, "commit_id"),
            "--criterion",
            &criterion_id,
        ])
        .expect("parse old ac show"))
        .expect("show old ac");
        assert_eq!(
            value(&old_criterion, "statement_json"),
            "\"Old acceptance criterion statement.\""
        );

        let current_criterion = run(Cli::try_parse_from([
            "workvcs",
            "ac",
            "show",
            store,
            "--branch",
            &branch,
            "--criterion",
            &criterion_id,
        ])
        .expect("parse current ac show"))
        .expect("show current ac");
        assert_eq!(
            value(&current_criterion, "statement_json"),
            "\"New acceptance criterion statement.\""
        );

        let requirement = run(Cli::try_parse_from([
            "workvcs",
            "vr",
            "create",
            store,
            "--branch",
            &branch,
            "--head",
            &value(&revised_criterion, "commit_id"),
            "--criterion",
            &criterion_id,
            "--criterion-version",
            &value(&revised_criterion, "acceptance_criterion_entity_version_id"),
            "--local-key",
            "VR-1",
            "--statement",
            "Old verification requirement statement.",
        ])
        .expect("parse vr create"))
        .expect("create vr");
        let requirement_id = value(&requirement, "verification_requirement_entity_id");

        let revised_requirement = run(Cli::try_parse_from([
            "workvcs",
            "vr",
            "revise",
            store,
            "--branch",
            &branch,
            "--head",
            &value(&requirement, "commit_id"),
            "--requirement",
            &requirement_id,
            "--requirement-version",
            &value(&requirement, "verification_requirement_entity_version_id"),
            "--statement",
            "New verification requirement statement.",
        ])
        .expect("parse vr revise"))
        .expect("revise vr");
        assert_eq!(
            value(
                &revised_requirement,
                "previous_verification_requirement_entity_version_id"
            ),
            value(&requirement, "verification_requirement_entity_version_id")
        );
        assert_ne!(
            value(
                &revised_requirement,
                "verification_requirement_entity_version_id"
            ),
            value(&requirement, "verification_requirement_entity_version_id")
        );
        assert_eq!(
            value(&revised_requirement, "previous_statement_json"),
            "\"Old verification requirement statement.\""
        );
        assert_eq!(
            value(&revised_requirement, "statement_json"),
            "\"New verification requirement statement.\""
        );

        let old_requirement = run(Cli::try_parse_from([
            "workvcs",
            "vr",
            "show",
            store,
            "--commit",
            &value(&requirement, "commit_id"),
            "--requirement",
            &requirement_id,
        ])
        .expect("parse old vr show"))
        .expect("show old vr");
        assert_eq!(
            value(&old_requirement, "statement_json"),
            "\"Old verification requirement statement.\""
        );

        let current_requirement = run(Cli::try_parse_from([
            "workvcs",
            "vr",
            "show",
            store,
            "--branch",
            &branch,
            "--requirement",
            &requirement_id,
        ])
        .expect("parse current vr show"))
        .expect("show current vr");
        assert_eq!(
            value(&current_requirement, "statement_json"),
            "\"New verification requirement statement.\""
        );
    }

    #[test]
    fn cli_creates_evidence_with_content_metadata() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let path = tempdir.path().join("workvcs.sqlite");
        let store = path.to_str().expect("path text");
        let content_file = tempdir.path().join("evidence.txt");
        fs::write(&content_file, b"evidence bytes").expect("write evidence content");
        let content_file_path = content_file.to_str().expect("content file path text");

        run(
            Cli::try_parse_from(["workvcs", "init", store, "--display-name", "cli-store"])
                .expect("parse init"),
        )
        .expect("run init");
        let workspace = run(Cli::try_parse_from([
            "workvcs",
            "workspace",
            "create",
            store,
            "--display-name",
            "evidence-workspace",
        ])
        .expect("parse workspace create"))
        .expect("create workspace");
        let workspace_id = value(&workspace, "workspace_id");
        let branch_id = value(&workspace, "branch_id");
        let session = run(Cli::try_parse_from([
            "workvcs",
            "session",
            "start",
            store,
            "--workspace",
            &workspace_id,
            "--branch",
            &branch_id,
            "--metadata-json",
            r#"{"purpose":"evidence capture"}"#,
        ])
        .expect("parse session start"))
        .expect("start session");
        let session_id = value(&session, "session_id");

        let evidence = run(Cli::try_parse_from([
            "workvcs",
            "evidence",
            "create",
            store,
            "--kind",
            "terminal-log",
            "--metadata-json",
            r#"{"summary":"captured output"}"#,
            "--source-session",
            &session_id,
            "--content-role",
            "stdout",
            "--content-file",
            content_file_path,
            "--media-type",
            "text/plain",
            "--format-metadata-json",
            r#"{"encoding":"utf-8"}"#,
        ])
        .expect("parse evidence create"))
        .expect("create evidence");
        let evidence_id = value(&evidence, "evidence_id");
        assert_eq!(value(&evidence, "evidence_kind"), "terminal-log");
        assert_eq!(value(&evidence, "source_session_id"), session_id);
        assert_eq!(value(&evidence, "contents"), "1");
        assert_eq!(value(&evidence, "content.0.role"), "stdout");
        assert_eq!(value(&evidence, "content.0.size_bytes"), "14");
        assert_eq!(value(&evidence, "content.0.media_type"), "text/plain");
        assert_eq!(
            value(&evidence, "content.0.format_metadata_json"),
            r#"{"encoding":"utf-8"}"#
        );

        let shown = run(Cli::try_parse_from([
            "workvcs",
            "evidence",
            "show",
            store,
            "--evidence",
            &evidence_id,
        ])
        .expect("parse evidence show"))
        .expect("show evidence");
        assert_eq!(value(&shown, "evidence_id"), evidence_id);
        assert_eq!(value(&shown, "source_session_id"), session_id);
        assert_eq!(
            value(&shown, "content.0.content_digest"),
            value(&evidence, "content.0.content_digest")
        );
        assert_eq!(value(&shown, "content.0.role"), "stdout");
        assert_eq!(value(&shown, "content.0.size_bytes"), "14");
        assert_eq!(value(&shown, "content.0.media_type"), "text/plain");
        assert_eq!(
            value(&shown, "content.0.format_metadata_json"),
            r#"{"encoding":"utf-8"}"#
        );

        let duplicate_content_source = Cli::try_parse_from([
            "workvcs",
            "evidence",
            "create",
            store,
            "--kind",
            "terminal-log",
            "--content-role",
            "stdout",
            "--content",
            "evidence bytes",
            "--content-file",
            content_file_path,
        ]);
        assert!(duplicate_content_source.is_err());

        let second_evidence = run(Cli::try_parse_from([
            "workvcs",
            "evidence",
            "create",
            store,
            "--kind",
            "manual-review",
            "--metadata-json",
            r#"{"summary":"reviewed"}"#,
        ])
        .expect("parse second evidence create"))
        .expect("create second evidence");
        let second_evidence_id = value(&second_evidence, "evidence_id");

        let list = run(Cli::try_parse_from(["workvcs", "evidence", "list", store])
            .expect("parse evidence list"))
        .expect("list evidence");
        assert_eq!(value(&list, "evidences"), "2");
        let listed_ids = [
            value(&list, "evidence.0.evidence_id"),
            value(&list, "evidence.1.evidence_id"),
        ];
        assert!(listed_ids.contains(&evidence_id));
        assert!(listed_ids.contains(&second_evidence_id));

        let limited_list =
            run(
                Cli::try_parse_from(["workvcs", "evidence", "list", store, "--limit", "1"])
                    .expect("parse limited evidence list"),
            )
            .expect("list limited evidence");
        assert_eq!(value(&limited_list, "evidences"), "1");
        assert_ne!(value(&limited_list, "evidence.0.evidence_id"), "");

        let zero_limit_list =
            run(
                Cli::try_parse_from(["workvcs", "evidence", "list", store, "--limit", "0"])
                    .expect("parse zero-limit evidence list"),
            );
        assert!(zero_limit_list.is_err());

        let filtered = run(Cli::try_parse_from([
            "workvcs",
            "evidence",
            "list",
            store,
            "--kind",
            "terminal-log",
        ])
        .expect("parse filtered evidence list"))
        .expect("list filtered evidence");
        assert_eq!(value(&filtered, "evidences"), "1");
        assert_eq!(value(&filtered, "evidence.0.evidence_id"), evidence_id);
        assert_eq!(value(&filtered, "evidence.0.evidence_kind"), "terminal-log");
        assert_eq!(value(&filtered, "evidence.0.contents"), "1");

        let source_session_filtered = run(Cli::try_parse_from([
            "workvcs",
            "evidence",
            "list",
            store,
            "--source-session",
            &session_id,
        ])
        .expect("parse source-session evidence list"))
        .expect("list source-session evidence");
        assert_eq!(value(&source_session_filtered, "evidences"), "1");
        assert_eq!(
            value(&source_session_filtered, "evidence.0.evidence_id"),
            evidence_id
        );
        assert_eq!(
            value(&source_session_filtered, "evidence.0.source_session_id"),
            session_id
        );

        let combined_filtered = run(Cli::try_parse_from([
            "workvcs",
            "evidence",
            "list",
            store,
            "--kind",
            "terminal-log",
            "--source-session",
            &session_id,
        ])
        .expect("parse combined evidence list"))
        .expect("list combined evidence");
        assert_eq!(value(&combined_filtered, "evidences"), "1");
        assert_eq!(
            value(&combined_filtered, "evidence.0.evidence_id"),
            evidence_id
        );

        let missing_source_session = run(Cli::try_parse_from([
            "workvcs",
            "evidence",
            "list",
            store,
            "--kind",
            "manual-review",
            "--source-session",
            &session_id,
        ])
        .expect("parse missing source-session evidence list"))
        .expect("list missing source-session evidence");
        assert_eq!(value(&missing_source_session, "evidences"), "0");
    }

    #[test]
    fn cli_shows_and_lists_verifications() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let path = tempdir.path().join("workvcs.sqlite");
        let store = path.to_str().expect("path text");

        run(
            Cli::try_parse_from(["workvcs", "init", store, "--display-name", "cli-store"])
                .expect("parse init"),
        )
        .expect("run init");
        let workspace = run(Cli::try_parse_from([
            "workvcs",
            "workspace",
            "create",
            store,
            "--display-name",
            "workspace",
        ])
        .expect("parse workspace"))
        .expect("create workspace");
        let branch = value(&workspace, "branch_id");
        let genesis = value(&workspace, "genesis_commit_id");

        let task = run(Cli::try_parse_from([
            "workvcs",
            "task",
            "create",
            store,
            "--branch",
            &branch,
            "--head",
            &genesis,
            "--description",
            "Show and list verification snapshots",
        ])
        .expect("parse task create"))
        .expect("create task");

        let criterion = run(Cli::try_parse_from([
            "workvcs",
            "ac",
            "create",
            store,
            "--branch",
            &branch,
            "--head",
            &value(&task, "commit_id"),
            "--task",
            &value(&task, "task_entity_id"),
            "--task-version",
            &value(&task, "task_entity_version_id"),
            "--local-key",
            "AC-1",
            "--statement",
            "The verification snapshot is visible.",
        ])
        .expect("parse ac create"))
        .expect("create ac");
        let criterion_id = value(&criterion, "acceptance_criterion_entity_id");

        let evidence = run(Cli::try_parse_from([
            "workvcs",
            "evidence",
            "create",
            store,
            "--kind",
            "manual-review",
            "--metadata-json",
            r#"{"summary":"reviewed locally"}"#,
        ])
        .expect("parse evidence create"))
        .expect("create evidence");
        let evidence_id = value(&evidence, "evidence_id");
        assert_eq!(value(&evidence, "evidence_kind"), "manual-review");
        assert_eq!(
            value(&evidence, "metadata_json"),
            r#"{"summary":"reviewed locally"}"#
        );
        assert_eq!(value(&evidence, "contents"), "0");

        let shown_evidence = run(Cli::try_parse_from([
            "workvcs",
            "evidence",
            "show",
            store,
            "--evidence",
            &evidence_id,
        ])
        .expect("parse evidence show"))
        .expect("show evidence");
        assert_eq!(value(&shown_evidence, "evidence_id"), evidence_id);
        assert_eq!(value(&shown_evidence, "evidence_kind"), "manual-review");
        assert_eq!(
            value(&shown_evidence, "metadata_json"),
            r#"{"summary":"reviewed locally"}"#
        );
        assert_eq!(value(&shown_evidence, "contents"), "0");

        let empty_verifications = run(Cli::try_parse_from([
            "workvcs",
            "verification",
            "list",
            store,
            "--commit",
            &value(&criterion, "commit_id"),
        ])
        .expect("parse empty verification list"))
        .expect("list empty verifications");
        assert_eq!(value(&empty_verifications, "verifications"), "0");

        let verification = run(Cli::try_parse_from([
            "workvcs",
            "verification",
            "record",
            store,
            "--branch",
            &branch,
            "--head",
            &value(&criterion, "commit_id"),
            "--result",
            "passed",
            "--method",
            "manual-review",
            "--evidence",
            &evidence_id,
            "--acceptance-criterion",
            &criterion_id,
        ])
        .expect("parse verification record"))
        .expect("record verification");
        let verification_id = value(&verification, "verification_entity_id");

        let verification_at_branch = run(Cli::try_parse_from([
            "workvcs",
            "verification",
            "show",
            store,
            "--branch",
            &branch,
            "--verification",
            &verification_id,
        ])
        .expect("parse verification show at branch"))
        .expect("show verification at branch");
        assert_eq!(
            value(&verification_at_branch, "commit_id"),
            value(&verification, "commit_id")
        );
        assert_eq!(
            value(&verification_at_branch, "verification_entity_version_id"),
            value(&verification, "verification_entity_version_id")
        );
        assert_eq!(
            value(&verification_at_branch, "target_kind"),
            "acceptance_criterion"
        );
        assert_eq!(
            value(&verification_at_branch, "target_entity_id"),
            criterion_id
        );
        assert_eq!(value(&verification_at_branch, "result"), "passed");
        assert_eq!(
            value(&verification_at_branch, "verified_at_commit_id"),
            value(&criterion, "commit_id")
        );
        assert_eq!(
            value(&verification_at_branch, "method_json"),
            r#"{"kind":"manual","name":"manual-review"}"#
        );
        assert_eq!(value(&verification_at_branch, "semantic_dependencies"), "1");
        assert_eq!(
            value(&verification_at_branch, "semantic_dependency.0.entity_id"),
            criterion_id
        );
        assert_eq!(
            value(
                &verification_at_branch,
                "semantic_dependency.0.entity_version_id"
            ),
            value(&criterion, "acceptance_criterion_entity_version_id")
        );
        assert_eq!(value(&verification_at_branch, "evidence"), "1");
        assert_eq!(
            value(&verification_at_branch, "evidence.0.evidence_id"),
            evidence_id
        );
        assert_eq!(
            value(&verification_at_branch, "evidenced_by_relations"),
            "1"
        );
        assert_eq!(
            value(
                &verification_at_branch,
                "evidenced_by_relation.0.evidence_id"
            ),
            evidence_id
        );
        assert_eq!(value(&verification_at_branch, "resource_basis"), "0");

        let verifications_at_branch = run(Cli::try_parse_from([
            "workvcs",
            "verification",
            "list",
            store,
            "--branch",
            &branch,
        ])
        .expect("parse verification list at branch"))
        .expect("list verification at branch");
        assert_eq!(
            value(&verifications_at_branch, "commit_id"),
            value(&verification, "commit_id")
        );
        assert_eq!(value(&verifications_at_branch, "verifications"), "1");
        assert_eq!(
            value(
                &verifications_at_branch,
                "verification.0.verification_entity_id"
            ),
            verification_id
        );
        assert_eq!(
            value(&verifications_at_branch, "verification.0.target_kind"),
            "acceptance_criterion"
        );
        assert_eq!(
            value(&verifications_at_branch, "verification.0.target_entity_id"),
            criterion_id
        );
        assert_eq!(
            value(&verifications_at_branch, "verification.0.result"),
            "passed"
        );
        assert_eq!(
            value(&verifications_at_branch, "verification.0.method_json"),
            r#"{"kind":"manual","name":"manual-review"}"#
        );
        assert_eq!(
            value(&verifications_at_branch, "verification.0.evidence"),
            "1"
        );

        let passed_verifications = run(Cli::try_parse_from([
            "workvcs",
            "verification",
            "list",
            store,
            "--branch",
            &branch,
            "--result",
            "passed",
        ])
        .expect("parse verification list by passed result"))
        .expect("list verification by passed result");
        assert_eq!(value(&passed_verifications, "verifications"), "1");
        assert_eq!(
            value(
                &passed_verifications,
                "verification.0.verification_entity_id"
            ),
            verification_id
        );

        let failed_verifications = run(Cli::try_parse_from([
            "workvcs",
            "verification",
            "list",
            store,
            "--branch",
            &branch,
            "--result",
            "failed",
        ])
        .expect("parse verification list by failed result"))
        .expect("list verification by failed result");
        assert_eq!(value(&failed_verifications, "verifications"), "0");

        let acceptance_target_verifications = run(Cli::try_parse_from([
            "workvcs",
            "verification",
            "list",
            store,
            "--branch",
            &branch,
            "--target-kind",
            "acceptance_criterion",
        ])
        .expect("parse verification list by acceptance target kind"))
        .expect("list verification by acceptance target kind");
        assert_eq!(
            value(&acceptance_target_verifications, "verifications"),
            "1"
        );

        let requirement_target_verifications = run(Cli::try_parse_from([
            "workvcs",
            "verification",
            "list",
            store,
            "--branch",
            &branch,
            "--target-kind",
            "verification_requirement",
        ])
        .expect("parse verification list by requirement target kind"))
        .expect("list verification by requirement target kind");
        assert_eq!(
            value(&requirement_target_verifications, "verifications"),
            "0"
        );

        let target_verifications = run(Cli::try_parse_from([
            "workvcs",
            "verification",
            "list",
            store,
            "--branch",
            &branch,
            "--target",
            &criterion_id,
        ])
        .expect("parse verification list by target"))
        .expect("list verification by target");
        assert_eq!(value(&target_verifications, "verifications"), "1");

        let wrong_target_verifications = run(Cli::try_parse_from([
            "workvcs",
            "verification",
            "list",
            store,
            "--branch",
            &branch,
            "--target",
            &value(&task, "task_entity_id"),
        ])
        .expect("parse verification list by wrong target"))
        .expect("list verification by wrong target");
        assert_eq!(value(&wrong_target_verifications, "verifications"), "0");

        let combined_verifications = run(Cli::try_parse_from([
            "workvcs",
            "verification",
            "list",
            store,
            "--branch",
            &branch,
            "--target-kind",
            "acceptance_criterion",
            "--target",
            &criterion_id,
            "--result",
            "passed",
        ])
        .expect("parse verification list by combined filters"))
        .expect("list verification by combined filters");
        assert_eq!(value(&combined_verifications, "verifications"), "1");
    }

    #[test]
    fn cli_creates_structural_task_relations_with_actor_session() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let path = tempdir.path().join("workvcs.sqlite");
        let store = path.to_str().expect("path text");

        run(
            Cli::try_parse_from(["workvcs", "init", store, "--display-name", "cli-store"])
                .expect("parse init"),
        )
        .expect("run init");
        let workspace = run(Cli::try_parse_from([
            "workvcs",
            "workspace",
            "create",
            store,
            "--display-name",
            "workspace",
        ])
        .expect("parse workspace"))
        .expect("create workspace");
        let branch = value(&workspace, "branch_id");
        let genesis = value(&workspace, "genesis_commit_id");
        let workspace_id = value(&workspace, "workspace_id");

        let first = run(Cli::try_parse_from([
            "workvcs",
            "task",
            "create",
            store,
            "--branch",
            &branch,
            "--head",
            &genesis,
            "--description",
            "First structural CLI task",
        ])
        .expect("parse first task"))
        .expect("create first task");
        let second = run(Cli::try_parse_from([
            "workvcs",
            "task",
            "create",
            store,
            "--branch",
            &branch,
            "--head",
            &value(&first, "commit_id"),
            "--description",
            "Second structural CLI task",
        ])
        .expect("parse second task"))
        .expect("create second task");
        let session = run(Cli::try_parse_from([
            "workvcs",
            "session",
            "start",
            store,
            "--workspace",
            &workspace_id,
            "--branch",
            &branch,
        ])
        .expect("parse session start"))
        .expect("start session");
        let session_id = value(&session, "session_id");
        let first_task = value(&first, "task_entity_id");
        let second_task = value(&second, "task_entity_id");

        let dependency = run(Cli::try_parse_from([
            "workvcs",
            "task",
            "depends-on",
            store,
            "--branch",
            &branch,
            "--head",
            &value(&second, "commit_id"),
            "--task",
            &second_task,
            "--depends-on",
            &first_task,
            "--session",
            &session_id,
        ])
        .expect("parse depends-on"))
        .expect("create dependency");
        assert_eq!(value(&dependency, "relation_type"), "depends_on");
        assert_eq!(value(&dependency, "source_task_entity_id"), second_task);
        assert_eq!(value(&dependency, "target_task_entity_id"), first_task);

        let order = run(Cli::try_parse_from([
            "workvcs",
            "task",
            "ordered-before",
            store,
            "--branch",
            &branch,
            "--head",
            &value(&dependency, "commit_id"),
            "--earlier",
            &first_task,
            "--later",
            &second_task,
            "--session",
            &session_id,
        ])
        .expect("parse ordered-before"))
        .expect("create order");
        assert_eq!(value(&order, "relation_type"), "ordered_before");
        assert_eq!(value(&order, "source_task_entity_id"), first_task);
        assert_eq!(value(&order, "target_task_entity_id"), second_task);

        let containment = run(Cli::try_parse_from([
            "workvcs",
            "task",
            "contain",
            store,
            "--branch",
            &branch,
            "--head",
            &value(&order, "commit_id"),
            "--parent",
            &first_task,
            "--child",
            &second_task,
            "--session",
            &session_id,
        ])
        .expect("parse containment"))
        .expect("create containment");
        assert_eq!(value(&containment, "parent_entity_id"), first_task);
        assert_eq!(value(&containment, "parent_kind"), "task");
        assert_eq!(value(&containment, "child_entity_id"), second_task);
        assert_eq!(value(&containment, "child_kind"), "task");

        let scheduling_list = run(Cli::try_parse_from([
            "workvcs",
            "task",
            "scheduling-list",
            store,
            "--branch",
            &branch,
        ])
        .expect("parse scheduling list"))
        .expect("list scheduling relations");
        assert_eq!(value(&scheduling_list, "relations"), "2");
        assert_eq!(
            value(&scheduling_list, "relation.0.relation_type"),
            "depends_on"
        );
        assert_eq!(
            value(&scheduling_list, "relation.1.relation_type"),
            "ordered_before"
        );
        assert_eq!(
            value(&scheduling_list, "relation.0.source_task_entity_id"),
            second_task
        );
        assert_eq!(
            value(&scheduling_list, "relation.0.target_task_entity_id"),
            first_task
        );

        let depends_on_list = run(Cli::try_parse_from([
            "workvcs",
            "task",
            "scheduling-list",
            store,
            "--branch",
            &branch,
            "--relation-type",
            "depends_on",
        ])
        .expect("parse depends-on scheduling list"))
        .expect("list depends-on scheduling relations");
        assert_eq!(value(&depends_on_list, "relations"), "1");
        assert_eq!(
            value(&depends_on_list, "relation.0.relation_type"),
            "depends_on"
        );

        let ordered_before_list = run(Cli::try_parse_from([
            "workvcs",
            "task",
            "scheduling-list",
            store,
            "--branch",
            &branch,
            "--relation-type",
            "ordered_before",
        ])
        .expect("parse ordered-before scheduling list"))
        .expect("list ordered-before scheduling relations");
        assert_eq!(value(&ordered_before_list, "relations"), "1");
        assert_eq!(
            value(&ordered_before_list, "relation.0.relation_type"),
            "ordered_before"
        );

        let source_task_list = run(Cli::try_parse_from([
            "workvcs",
            "task",
            "scheduling-list",
            store,
            "--branch",
            &branch,
            "--source-task",
            &first_task,
        ])
        .expect("parse scheduling list by source task"))
        .expect("list scheduling relations by source task");
        assert_eq!(value(&source_task_list, "relations"), "1");
        assert_eq!(
            value(&source_task_list, "relation.0.relation_type"),
            "ordered_before"
        );

        let target_task_list = run(Cli::try_parse_from([
            "workvcs",
            "task",
            "scheduling-list",
            store,
            "--branch",
            &branch,
            "--target-task",
            &first_task,
        ])
        .expect("parse scheduling list by target task"))
        .expect("list scheduling relations by target task");
        assert_eq!(value(&target_task_list, "relations"), "1");
        assert_eq!(
            value(&target_task_list, "relation.0.relation_type"),
            "depends_on"
        );

        let combined_scheduling_list = run(Cli::try_parse_from([
            "workvcs",
            "task",
            "scheduling-list",
            store,
            "--branch",
            &branch,
            "--relation-type",
            "ordered_before",
            "--source-task",
            &first_task,
            "--target-task",
            &second_task,
        ])
        .expect("parse combined scheduling list"))
        .expect("list scheduling relations by combined filters");
        assert_eq!(value(&combined_scheduling_list, "relations"), "1");

        let missing_scheduling_list = run(Cli::try_parse_from([
            "workvcs",
            "task",
            "scheduling-list",
            store,
            "--branch",
            &branch,
            "--relation-type",
            "depends_on",
            "--target-task",
            &second_task,
        ])
        .expect("parse missing scheduling list"))
        .expect("list missing scheduling relations");
        assert_eq!(value(&missing_scheduling_list, "relations"), "0");

        let containment_list = run(Cli::try_parse_from([
            "workvcs",
            "task",
            "containment-list",
            store,
            "--branch",
            &branch,
        ])
        .expect("parse containment list"))
        .expect("list containment relations");
        assert_eq!(value(&containment_list, "relations"), "1");
        assert_eq!(
            value(&containment_list, "relation.0.relation_id"),
            value(&containment, "relation_id")
        );
        assert_eq!(
            value(&containment_list, "relation.0.parent_entity_id"),
            first_task
        );
        assert_eq!(
            value(&containment_list, "relation.0.child_entity_id"),
            second_task
        );

        let containment_by_parent = run(Cli::try_parse_from([
            "workvcs",
            "task",
            "containment-list",
            store,
            "--branch",
            &branch,
            "--parent",
            &first_task,
        ])
        .expect("parse containment list by parent"))
        .expect("list containment by parent");
        assert_eq!(value(&containment_by_parent, "relations"), "1");

        let containment_by_child = run(Cli::try_parse_from([
            "workvcs",
            "task",
            "containment-list",
            store,
            "--branch",
            &branch,
            "--child",
            &second_task,
        ])
        .expect("parse containment list by child"))
        .expect("list containment by child");
        assert_eq!(value(&containment_by_child, "relations"), "1");

        let containment_by_parent_kind = run(Cli::try_parse_from([
            "workvcs",
            "task",
            "containment-list",
            store,
            "--branch",
            &branch,
            "--parent-kind",
            "task",
        ])
        .expect("parse containment list by parent kind"))
        .expect("list containment by parent kind");
        assert_eq!(value(&containment_by_parent_kind, "relations"), "1");

        let containment_by_child_kind = run(Cli::try_parse_from([
            "workvcs",
            "task",
            "containment-list",
            store,
            "--branch",
            &branch,
            "--child-kind",
            "task",
        ])
        .expect("parse containment list by child kind"))
        .expect("list containment by child kind");
        assert_eq!(value(&containment_by_child_kind, "relations"), "1");

        let containment_by_combined_filters = run(Cli::try_parse_from([
            "workvcs",
            "task",
            "containment-list",
            store,
            "--branch",
            &branch,
            "--parent",
            &first_task,
            "--child",
            &second_task,
            "--parent-kind",
            "task",
            "--child-kind",
            "task",
        ])
        .expect("parse containment list by combined filters"))
        .expect("list containment by combined filters");
        assert_eq!(value(&containment_by_combined_filters, "relations"), "1");

        let containment_by_missing_child = run(Cli::try_parse_from([
            "workvcs",
            "task",
            "containment-list",
            store,
            "--branch",
            &branch,
            "--child",
            &first_task,
        ])
        .expect("parse containment list by missing child"))
        .expect("list containment by missing child");
        assert_eq!(value(&containment_by_missing_child, "relations"), "0");

        let historical_containment_list = run(Cli::try_parse_from([
            "workvcs",
            "task",
            "containment-list",
            store,
            "--commit",
            &value(&order, "commit_id"),
        ])
        .expect("parse historical containment list"))
        .expect("list historical containment relations");
        assert_eq!(value(&historical_containment_list, "relations"), "0");
    }

    #[test]
    fn cli_creates_goal_plan_and_task_containment_hierarchy() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let path = tempdir.path().join("workvcs.sqlite");
        let store = path.to_str().expect("path text");

        run(
            Cli::try_parse_from(["workvcs", "init", store, "--display-name", "cli-store"])
                .expect("parse init"),
        )
        .expect("run init");
        let workspace = run(Cli::try_parse_from([
            "workvcs",
            "workspace",
            "create",
            store,
            "--display-name",
            "workspace",
        ])
        .expect("parse workspace"))
        .expect("create workspace");
        let branch = value(&workspace, "branch_id");
        let genesis = value(&workspace, "genesis_commit_id");
        let workspace_id = value(&workspace, "workspace_id");

        let goal = run(Cli::try_parse_from([
            "workvcs",
            "goal",
            "create",
            store,
            "--branch",
            &branch,
            "--head",
            &genesis,
            "--description",
            "Deliver the CLI hierarchy",
        ])
        .expect("parse goal create"))
        .expect("create goal");
        assert_eq!(value(&goal, "status"), "active");

        let plan = run(Cli::try_parse_from([
            "workvcs",
            "plan",
            "create",
            store,
            "--branch",
            &branch,
            "--head",
            &value(&goal, "commit_id"),
            "--description",
            "Implement the hierarchy",
            "--strategy",
            "Use existing Engine APIs",
            "--constraint",
            "stay within v0.1",
        ])
        .expect("parse plan create"))
        .expect("create plan");
        assert_eq!(value(&plan, "status"), "active");
        assert_eq!(value(&plan, "constraints"), "1");

        let task = run(Cli::try_parse_from([
            "workvcs",
            "task",
            "create",
            store,
            "--branch",
            &branch,
            "--head",
            &value(&plan, "commit_id"),
            "--description",
            "Wire containment",
        ])
        .expect("parse task create"))
        .expect("create task");
        let session = run(Cli::try_parse_from([
            "workvcs",
            "session",
            "start",
            store,
            "--workspace",
            &workspace_id,
            "--branch",
            &branch,
        ])
        .expect("parse session start"))
        .expect("start session");
        let session_id = value(&session, "session_id");

        let goal_contains_plan = run(Cli::try_parse_from([
            "workvcs",
            "task",
            "contain",
            store,
            "--branch",
            &branch,
            "--head",
            &value(&task, "commit_id"),
            "--parent",
            &value(&goal, "goal_entity_id"),
            "--child",
            &value(&plan, "plan_entity_id"),
            "--session",
            &session_id,
        ])
        .expect("parse goal contains plan"))
        .expect("create goal-plan containment");
        assert_eq!(value(&goal_contains_plan, "parent_kind"), "goal");
        assert_eq!(value(&goal_contains_plan, "child_kind"), "plan");

        let plan_contains_task = run(Cli::try_parse_from([
            "workvcs",
            "task",
            "contain",
            store,
            "--branch",
            &branch,
            "--head",
            &value(&goal_contains_plan, "commit_id"),
            "--parent",
            &value(&plan, "plan_entity_id"),
            "--child",
            &value(&task, "task_entity_id"),
            "--session",
            &session_id,
        ])
        .expect("parse plan contains task"))
        .expect("create plan-task containment");
        assert_eq!(value(&plan_contains_task, "parent_kind"), "plan");
        assert_eq!(value(&plan_contains_task, "child_kind"), "task");

        let containment = run(Cli::try_parse_from([
            "workvcs",
            "task",
            "containment-list",
            store,
            "--branch",
            &branch,
        ])
        .expect("parse containment list"))
        .expect("list containment");
        assert_eq!(value(&containment, "relations"), "2");
        assert!(containment.contains("relation.0.parent_kind=goal"));
        assert!(containment.contains("relation.1.parent_kind=plan"));
    }

    #[test]
    fn cli_transitions_goal_and_plan_lifecycles() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let path = tempdir.path().join("workvcs.sqlite");
        let store = path.to_str().expect("path text");

        run(
            Cli::try_parse_from(["workvcs", "init", store, "--display-name", "cli-store"])
                .expect("parse init"),
        )
        .expect("run init");
        let workspace = run(Cli::try_parse_from([
            "workvcs",
            "workspace",
            "create",
            store,
            "--display-name",
            "workspace",
        ])
        .expect("parse workspace"))
        .expect("create workspace");
        let branch = value(&workspace, "branch_id");
        let genesis = value(&workspace, "genesis_commit_id");

        let goal = run(Cli::try_parse_from([
            "workvcs",
            "goal",
            "create",
            store,
            "--branch",
            &branch,
            "--head",
            &genesis,
            "--description",
            "Deliver the lifecycle CLI",
        ])
        .expect("parse goal create"))
        .expect("create goal");
        assert_eq!(value(&goal, "status"), "active");
        let goal_id = value(&goal, "goal_entity_id");
        let active_goal_version = value(&goal, "goal_entity_version_id");

        let achieved_goal = run(Cli::try_parse_from([
            "workvcs",
            "goal",
            "achieve",
            store,
            "--branch",
            &branch,
            "--head",
            &value(&goal, "commit_id"),
            "--goal",
            &goal_id,
            "--goal-version",
            &active_goal_version,
            "--rationale",
            "accepted by CLI workflow",
        ])
        .expect("parse goal achieve"))
        .expect("achieve goal");
        assert_eq!(value(&achieved_goal, "previous_status"), "active");
        assert_eq!(value(&achieved_goal, "status"), "achieved");
        assert_ne!(
            value(&achieved_goal, "goal_entity_version_id"),
            active_goal_version
        );

        let reopened_goal = run(Cli::try_parse_from([
            "workvcs",
            "goal",
            "reopen",
            store,
            "--branch",
            &branch,
            "--head",
            &value(&achieved_goal, "commit_id"),
            "--goal",
            &goal_id,
            "--goal-version",
            &value(&achieved_goal, "goal_entity_version_id"),
            "--rationale",
            "new CLI work discovered",
        ])
        .expect("parse goal reopen"))
        .expect("reopen goal");
        assert_eq!(value(&reopened_goal, "previous_status"), "achieved");
        assert_eq!(value(&reopened_goal, "status"), "active");

        let plan = run(Cli::try_parse_from([
            "workvcs",
            "plan",
            "create",
            store,
            "--branch",
            &branch,
            "--head",
            &value(&reopened_goal, "commit_id"),
            "--description",
            "Exercise plan lifecycle CLI",
            "--strategy",
            "Use thin wrappers",
            "--constraint",
            "no semantic expansion",
        ])
        .expect("parse plan create"))
        .expect("create plan");
        assert_eq!(value(&plan, "status"), "active");
        let plan_id = value(&plan, "plan_entity_id");
        let active_plan_version = value(&plan, "plan_entity_version_id");

        let completed_plan = run(Cli::try_parse_from([
            "workvcs",
            "plan",
            "complete",
            store,
            "--branch",
            &branch,
            "--head",
            &value(&plan, "commit_id"),
            "--plan",
            &plan_id,
            "--plan-version",
            &active_plan_version,
            "--completion-rationale",
            "done through CLI",
        ])
        .expect("parse plan complete"))
        .expect("complete plan");
        assert_eq!(value(&completed_plan, "previous_status"), "active");
        assert_eq!(value(&completed_plan, "status"), "completed");
        assert_eq!(value(&completed_plan, "constraints"), "1");
        assert_eq!(
            value(&completed_plan, "completion_rationale"),
            "done through CLI"
        );
        assert_ne!(
            value(&completed_plan, "plan_entity_version_id"),
            active_plan_version
        );

        let reopened_plan = run(Cli::try_parse_from([
            "workvcs",
            "plan",
            "reopen",
            store,
            "--branch",
            &branch,
            "--head",
            &value(&completed_plan, "commit_id"),
            "--plan",
            &plan_id,
            "--plan-version",
            &value(&completed_plan, "plan_entity_version_id"),
            "--rationale",
            "follow-up work needed",
        ])
        .expect("parse plan reopen"))
        .expect("reopen plan");
        assert_eq!(value(&reopened_plan, "previous_status"), "completed");
        assert_eq!(value(&reopened_plan, "status"), "active");
        assert_eq!(value(&reopened_plan, "completion_rationale"), "");

        let abandoned_plan = run(Cli::try_parse_from([
            "workvcs",
            "plan",
            "abandon",
            store,
            "--branch",
            &branch,
            "--head",
            &value(&reopened_plan, "commit_id"),
            "--plan",
            &plan_id,
            "--plan-version",
            &value(&reopened_plan, "plan_entity_version_id"),
            "--rationale",
            "superseded outside this CLI workflow",
        ])
        .expect("parse plan abandon"))
        .expect("abandon plan");
        assert_eq!(value(&abandoned_plan, "previous_status"), "active");
        assert_eq!(value(&abandoned_plan, "status"), "abandoned");
    }

    #[test]
    fn cli_shows_goal_and_plan_snapshots_at_branch_or_commit() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let path = tempdir.path().join("workvcs.sqlite");
        let store = path.to_str().expect("path text");

        run(
            Cli::try_parse_from(["workvcs", "init", store, "--display-name", "cli-store"])
                .expect("parse init"),
        )
        .expect("run init");
        let workspace = run(Cli::try_parse_from([
            "workvcs",
            "workspace",
            "create",
            store,
            "--display-name",
            "workspace",
        ])
        .expect("parse workspace"))
        .expect("create workspace");
        let branch = value(&workspace, "branch_id");
        let genesis = value(&workspace, "genesis_commit_id");

        let goal = run(Cli::try_parse_from([
            "workvcs",
            "goal",
            "create",
            store,
            "--branch",
            &branch,
            "--head",
            &genesis,
            "--description",
            "Inspect goal snapshots",
        ])
        .expect("parse goal create"))
        .expect("create goal");
        let goal_id = value(&goal, "goal_entity_id");

        let goal_at_create = run(Cli::try_parse_from([
            "workvcs",
            "goal",
            "show",
            store,
            "--commit",
            &value(&goal, "commit_id"),
            "--goal",
            &goal_id,
        ])
        .expect("parse goal show at commit"))
        .expect("show goal at commit");
        assert_eq!(value(&goal_at_create, "status"), "active");
        assert_eq!(
            value(&goal_at_create, "goal_entity_version_id"),
            value(&goal, "goal_entity_version_id")
        );
        assert_eq!(
            value(&goal_at_create, "description_json"),
            "\"Inspect goal snapshots\""
        );
        assert_eq!(value(&goal_at_create, "terminal_rationale_json"), "null");

        let achieved_goal = run(Cli::try_parse_from([
            "workvcs",
            "goal",
            "achieve",
            store,
            "--branch",
            &branch,
            "--head",
            &value(&goal, "commit_id"),
            "--goal",
            &goal_id,
            "--goal-version",
            &value(&goal, "goal_entity_version_id"),
            "--rationale",
            "snapshot visible",
        ])
        .expect("parse goal achieve"))
        .expect("achieve goal");

        let goal_at_branch = run(Cli::try_parse_from([
            "workvcs", "goal", "show", store, "--branch", &branch, "--goal", &goal_id,
        ])
        .expect("parse goal show at branch"))
        .expect("show goal at branch");
        assert_eq!(
            value(&goal_at_branch, "commit_id"),
            value(&achieved_goal, "commit_id")
        );
        assert_eq!(value(&goal_at_branch, "status"), "achieved");
        assert_eq!(
            value(&goal_at_branch, "goal_entity_version_id"),
            value(&achieved_goal, "goal_entity_version_id")
        );
        assert_eq!(
            value(&goal_at_branch, "terminal_rationale_json"),
            "\"snapshot visible\""
        );

        let plan = run(Cli::try_parse_from([
            "workvcs",
            "plan",
            "create",
            store,
            "--branch",
            &branch,
            "--head",
            &value(&achieved_goal, "commit_id"),
            "--description",
            "Inspect plan snapshots",
            "--strategy",
            "Use show commands",
            "--constraint",
            "branch selector",
            "--constraint",
            "commit selector",
        ])
        .expect("parse plan create"))
        .expect("create plan");
        let plan_id = value(&plan, "plan_entity_id");

        let plan_at_create = run(Cli::try_parse_from([
            "workvcs",
            "plan",
            "show",
            store,
            "--commit",
            &value(&plan, "commit_id"),
            "--plan",
            &plan_id,
        ])
        .expect("parse plan show at commit"))
        .expect("show plan at commit");
        assert_eq!(value(&plan_at_create, "status"), "active");
        assert_eq!(value(&plan_at_create, "constraints"), "2");
        assert_eq!(
            value(&plan_at_create, "constraints_json"),
            "[\"branch selector\",\"commit selector\"]"
        );
        assert_eq!(value(&plan_at_create, "completion_rationale_json"), "null");

        let completed_plan = run(Cli::try_parse_from([
            "workvcs",
            "plan",
            "complete",
            store,
            "--branch",
            &branch,
            "--head",
            &value(&plan, "commit_id"),
            "--plan",
            &plan_id,
            "--plan-version",
            &value(&plan, "plan_entity_version_id"),
            "--completion-rationale",
            "snapshot confirms completion",
        ])
        .expect("parse plan complete"))
        .expect("complete plan");

        let plan_at_branch = run(Cli::try_parse_from([
            "workvcs", "plan", "show", store, "--branch", &branch, "--plan", &plan_id,
        ])
        .expect("parse plan show at branch"))
        .expect("show plan at branch");
        assert_eq!(
            value(&plan_at_branch, "commit_id"),
            value(&completed_plan, "commit_id")
        );
        assert_eq!(value(&plan_at_branch, "status"), "completed");
        assert_eq!(
            value(&plan_at_branch, "plan_entity_version_id"),
            value(&completed_plan, "plan_entity_version_id")
        );
        assert_eq!(
            value(&plan_at_branch, "completion_rationale_json"),
            "\"snapshot confirms completion\""
        );
    }

    #[test]
    fn cli_lists_goal_and_plan_snapshots_at_branch_or_commit() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let path = tempdir.path().join("workvcs.sqlite");
        let store = path.to_str().expect("path text");

        run(
            Cli::try_parse_from(["workvcs", "init", store, "--display-name", "cli-store"])
                .expect("parse init"),
        )
        .expect("run init");
        let workspace = run(Cli::try_parse_from([
            "workvcs",
            "workspace",
            "create",
            store,
            "--display-name",
            "workspace",
        ])
        .expect("parse workspace"))
        .expect("create workspace");
        let branch = value(&workspace, "branch_id");
        let genesis = value(&workspace, "genesis_commit_id");

        let empty_goals =
            run(
                Cli::try_parse_from(["workvcs", "goal", "list", store, "--commit", &genesis])
                    .expect("parse empty goal list"),
            )
            .expect("list empty goals");
        assert_eq!(value(&empty_goals, "goals"), "0");

        let empty_plans =
            run(
                Cli::try_parse_from(["workvcs", "plan", "list", store, "--commit", &genesis])
                    .expect("parse empty plan list"),
            )
            .expect("list empty plans");
        assert_eq!(value(&empty_plans, "plans"), "0");

        let goal = run(Cli::try_parse_from([
            "workvcs",
            "goal",
            "create",
            store,
            "--branch",
            &branch,
            "--head",
            &genesis,
            "--description",
            "List goal snapshots",
        ])
        .expect("parse goal create"))
        .expect("create goal");
        let goal_id = value(&goal, "goal_entity_id");

        let goals_at_create = run(Cli::try_parse_from([
            "workvcs",
            "goal",
            "list",
            store,
            "--commit",
            &value(&goal, "commit_id"),
        ])
        .expect("parse goal list at commit"))
        .expect("list goals at commit");
        assert_eq!(value(&goals_at_create, "goals"), "1");
        assert_eq!(value(&goals_at_create, "goal.0.goal_entity_id"), goal_id);
        assert_eq!(value(&goals_at_create, "goal.0.status"), "active");

        let active_goals_at_create = run(Cli::try_parse_from([
            "workvcs",
            "goal",
            "list",
            store,
            "--commit",
            &value(&goal, "commit_id"),
            "--status",
            "active",
        ])
        .expect("parse active goal list at commit"))
        .expect("list active goals at commit");
        assert_eq!(value(&active_goals_at_create, "goals"), "1");
        assert_eq!(
            value(&active_goals_at_create, "goal.0.goal_entity_id"),
            goal_id
        );

        let limited_active_goals_at_create = run(Cli::try_parse_from([
            "workvcs",
            "goal",
            "list",
            store,
            "--commit",
            &value(&goal, "commit_id"),
            "--status",
            "active",
            "--limit",
            "1",
        ])
        .expect("parse limited active goal list at commit"))
        .expect("list limited active goals at commit");
        assert_eq!(value(&limited_active_goals_at_create, "goals"), "1");
        assert_eq!(
            value(&limited_active_goals_at_create, "goal.0.goal_entity_id"),
            goal_id
        );

        let zero_limit_goals_at_create = run(Cli::try_parse_from([
            "workvcs",
            "goal",
            "list",
            store,
            "--commit",
            &value(&goal, "commit_id"),
            "--limit",
            "0",
        ])
        .expect("parse zero-limit goal list at commit"));
        assert!(zero_limit_goals_at_create.is_err());

        let achieved_goals_at_create = run(Cli::try_parse_from([
            "workvcs",
            "goal",
            "list",
            store,
            "--commit",
            &value(&goal, "commit_id"),
            "--status",
            "achieved",
        ])
        .expect("parse achieved goal list at commit"))
        .expect("list achieved goals at commit");
        assert_eq!(value(&achieved_goals_at_create, "goals"), "0");

        let achieved_goal = run(Cli::try_parse_from([
            "workvcs",
            "goal",
            "achieve",
            store,
            "--branch",
            &branch,
            "--head",
            &value(&goal, "commit_id"),
            "--goal",
            &goal_id,
            "--goal-version",
            &value(&goal, "goal_entity_version_id"),
            "--rationale",
            "list sees terminal goal",
        ])
        .expect("parse goal achieve"))
        .expect("achieve goal");

        let goals_at_branch =
            run(
                Cli::try_parse_from(["workvcs", "goal", "list", store, "--branch", &branch])
                    .expect("parse goal list at branch"),
            )
            .expect("list goals at branch");
        assert_eq!(
            value(&goals_at_branch, "commit_id"),
            value(&achieved_goal, "commit_id")
        );
        assert_eq!(value(&goals_at_branch, "goals"), "1");
        assert_eq!(value(&goals_at_branch, "goal.0.status"), "achieved");
        assert_eq!(
            value(&goals_at_branch, "goal.0.terminal_rationale_json"),
            "\"list sees terminal goal\""
        );

        let achieved_goals_at_branch = run(Cli::try_parse_from([
            "workvcs", "goal", "list", store, "--branch", &branch, "--status", "achieved",
        ])
        .expect("parse achieved goal list at branch"))
        .expect("list achieved goals at branch");
        assert_eq!(value(&achieved_goals_at_branch, "goals"), "1");
        assert_eq!(
            value(&achieved_goals_at_branch, "goal.0.goal_entity_id"),
            goal_id
        );

        let active_goals_at_branch = run(Cli::try_parse_from([
            "workvcs", "goal", "list", store, "--branch", &branch, "--status", "active",
        ])
        .expect("parse active goal list at branch"))
        .expect("list active goals at branch");
        assert_eq!(value(&active_goals_at_branch, "goals"), "0");

        let plan = run(Cli::try_parse_from([
            "workvcs",
            "plan",
            "create",
            store,
            "--branch",
            &branch,
            "--head",
            &value(&achieved_goal, "commit_id"),
            "--description",
            "List plan snapshots",
            "--strategy",
            "Use list commands",
            "--constraint",
            "current branch",
        ])
        .expect("parse plan create"))
        .expect("create plan");
        let plan_id = value(&plan, "plan_entity_id");

        let plans_at_create = run(Cli::try_parse_from([
            "workvcs",
            "plan",
            "list",
            store,
            "--commit",
            &value(&plan, "commit_id"),
        ])
        .expect("parse plan list at commit"))
        .expect("list plans at commit");
        assert_eq!(value(&plans_at_create, "plans"), "1");
        assert_eq!(value(&plans_at_create, "plan.0.plan_entity_id"), plan_id);
        assert_eq!(value(&plans_at_create, "plan.0.status"), "active");
        assert_eq!(value(&plans_at_create, "plan.0.constraints"), "1");

        let active_plans_at_create = run(Cli::try_parse_from([
            "workvcs",
            "plan",
            "list",
            store,
            "--commit",
            &value(&plan, "commit_id"),
            "--status",
            "active",
        ])
        .expect("parse active plan list at commit"))
        .expect("list active plans at commit");
        assert_eq!(value(&active_plans_at_create, "plans"), "1");
        assert_eq!(
            value(&active_plans_at_create, "plan.0.plan_entity_id"),
            plan_id
        );

        let limited_active_plans_at_create = run(Cli::try_parse_from([
            "workvcs",
            "plan",
            "list",
            store,
            "--commit",
            &value(&plan, "commit_id"),
            "--status",
            "active",
            "--limit",
            "1",
        ])
        .expect("parse limited active plan list at commit"))
        .expect("list limited active plans at commit");
        assert_eq!(value(&limited_active_plans_at_create, "plans"), "1");
        assert_eq!(
            value(&limited_active_plans_at_create, "plan.0.plan_entity_id"),
            plan_id
        );

        let zero_limit_plans_at_create = run(Cli::try_parse_from([
            "workvcs",
            "plan",
            "list",
            store,
            "--commit",
            &value(&plan, "commit_id"),
            "--limit",
            "0",
        ])
        .expect("parse zero-limit plan list at commit"));
        assert!(zero_limit_plans_at_create.is_err());

        let completed_plans_at_create = run(Cli::try_parse_from([
            "workvcs",
            "plan",
            "list",
            store,
            "--commit",
            &value(&plan, "commit_id"),
            "--status",
            "completed",
        ])
        .expect("parse completed plan list at commit"))
        .expect("list completed plans at commit");
        assert_eq!(value(&completed_plans_at_create, "plans"), "0");

        let completed_plan = run(Cli::try_parse_from([
            "workvcs",
            "plan",
            "complete",
            store,
            "--branch",
            &branch,
            "--head",
            &value(&plan, "commit_id"),
            "--plan",
            &plan_id,
            "--plan-version",
            &value(&plan, "plan_entity_version_id"),
            "--completion-rationale",
            "list sees completed plan",
        ])
        .expect("parse plan complete"))
        .expect("complete plan");

        let plans_at_branch =
            run(
                Cli::try_parse_from(["workvcs", "plan", "list", store, "--branch", &branch])
                    .expect("parse plan list at branch"),
            )
            .expect("list plans at branch");
        assert_eq!(
            value(&plans_at_branch, "commit_id"),
            value(&completed_plan, "commit_id")
        );
        assert_eq!(value(&plans_at_branch, "plans"), "1");
        assert_eq!(value(&plans_at_branch, "plan.0.status"), "completed");
        assert_eq!(
            value(&plans_at_branch, "plan.0.completion_rationale_json"),
            "\"list sees completed plan\""
        );

        let completed_plans_at_branch = run(Cli::try_parse_from([
            "workvcs",
            "plan",
            "list",
            store,
            "--branch",
            &branch,
            "--status",
            "completed",
        ])
        .expect("parse completed plan list at branch"))
        .expect("list completed plans at branch");
        assert_eq!(value(&completed_plans_at_branch, "plans"), "1");
        assert_eq!(
            value(&completed_plans_at_branch, "plan.0.plan_entity_id"),
            plan_id
        );

        let active_plans_at_branch = run(Cli::try_parse_from([
            "workvcs", "plan", "list", store, "--branch", &branch, "--status", "active",
        ])
        .expect("parse active plan list at branch"))
        .expect("list active plans at branch");
        assert_eq!(value(&active_plans_at_branch, "plans"), "0");

        let superseded_plans_at_branch = run(Cli::try_parse_from([
            "workvcs",
            "plan",
            "list",
            store,
            "--branch",
            &branch,
            "--status",
            "superseded",
        ])
        .expect("parse superseded plan list at branch"))
        .expect("list superseded plans at branch");
        assert_eq!(value(&superseded_plans_at_branch, "plans"), "0");
    }

    #[test]
    fn cli_shows_binds_and_associates_resources() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let path = tempdir.path().join("workvcs.sqlite");
        let store = path.to_str().expect("path text");

        run(
            Cli::try_parse_from(["workvcs", "init", store, "--display-name", "cli-store"])
                .expect("parse init"),
        )
        .expect("run init");
        let workspace = run(Cli::try_parse_from([
            "workvcs",
            "workspace",
            "create",
            store,
            "--display-name",
            "workspace",
        ])
        .expect("parse workspace"))
        .expect("create workspace");
        let workspace_id = value(&workspace, "workspace_id");

        let resource = run(Cli::try_parse_from([
            "workvcs",
            "resource",
            "create",
            store,
            "--kind",
            "git-worktree",
        ])
        .expect("parse resource create"))
        .expect("create resource");
        let resource_id = value(&resource, "resource_id");

        let resource_before_bind = run(Cli::try_parse_from([
            "workvcs",
            "resource",
            "show",
            store,
            "--resource",
            &resource_id,
        ])
        .expect("parse resource show before bind"))
        .expect("show resource before bind");
        assert_eq!(
            value(&resource_before_bind, "resource_kind"),
            "git-worktree"
        );
        assert_eq!(value(&resource_before_bind, "binding_present"), "false");
        assert_eq!(value(&resource_before_bind, "workspace_associations"), "0");

        let binding = run(Cli::try_parse_from([
            "workvcs",
            "resource",
            "bind",
            store,
            "--resource",
            &resource_id,
            "--adapter-kind",
            "git",
            "--locator",
            "file:///repo",
            "--binding-config-json",
            r#"{"branch":"main"}"#,
        ])
        .expect("parse resource bind"))
        .expect("bind resource");
        assert_eq!(value(&binding, "resource_id"), resource_id);
        assert_eq!(value(&binding, "adapter_kind"), "git");
        assert_eq!(value(&binding, "locator"), "file:///repo");
        assert_eq!(
            value(&binding, "binding_config_json"),
            r#"{"branch":"main"}"#
        );

        let association = run(Cli::try_parse_from([
            "workvcs",
            "resource",
            "associate-workspace",
            store,
            "--workspace",
            &workspace_id,
            "--resource",
            &resource_id,
            "--metadata-json",
            r#"{"role":"primary"}"#,
        ])
        .expect("parse workspace resource association"))
        .expect("associate resource");
        assert_eq!(value(&association, "workspace_id"), workspace_id);
        assert_eq!(value(&association, "resource_id"), resource_id);
        assert_eq!(
            value(&association, "metadata_json"),
            r#"{"role":"primary"}"#
        );

        let resource_after_association = run(Cli::try_parse_from([
            "workvcs",
            "resource",
            "show",
            store,
            "--resource",
            &resource_id,
        ])
        .expect("parse resource show after association"))
        .expect("show resource after association");
        assert_eq!(
            value(&resource_after_association, "binding_present"),
            "true"
        );
        assert_eq!(
            value(&resource_after_association, "binding.adapter_kind"),
            "git"
        );
        assert_eq!(
            value(&resource_after_association, "binding.binding_config_json"),
            r#"{"branch":"main"}"#
        );
        assert_eq!(
            value(&resource_after_association, "workspace_associations"),
            "1"
        );
        assert_eq!(
            value(
                &resource_after_association,
                "workspace_association.0.workspace_id"
            ),
            workspace_id
        );
        assert_eq!(
            value(
                &resource_after_association,
                "workspace_association.0.metadata_json"
            ),
            r#"{"role":"primary"}"#
        );

        let workspace_associations = run(Cli::try_parse_from([
            "workvcs",
            "resource",
            "workspace-association-list",
            store,
            "--workspace",
            &workspace_id,
        ])
        .expect("parse workspace association list"))
        .expect("list workspace associations");
        assert_eq!(value(&workspace_associations, "workspace_id"), workspace_id);
        assert_eq!(
            value(&workspace_associations, "workspace_associations"),
            "1"
        );
        assert_eq!(
            value(
                &workspace_associations,
                "workspace_association.0.resource_id"
            ),
            resource_id
        );
        assert_eq!(
            value(
                &workspace_associations,
                "workspace_association.0.metadata_json"
            ),
            r#"{"role":"primary"}"#
        );

        let second_resource = run(Cli::try_parse_from([
            "workvcs", "resource", "create", store, "--kind", "document",
        ])
        .expect("parse second resource create"))
        .expect("create second resource");
        let second_resource_id = value(&second_resource, "resource_id");

        let second_association = run(Cli::try_parse_from([
            "workvcs",
            "resource",
            "associate-workspace",
            store,
            "--workspace",
            &workspace_id,
            "--resource",
            &second_resource_id,
            "--metadata-json",
            r#"{"role":"reference"}"#,
        ])
        .expect("parse second workspace resource association"))
        .expect("associate second resource");
        assert_eq!(value(&second_association, "workspace_id"), workspace_id);
        assert_eq!(
            value(&second_association, "resource_id"),
            second_resource_id
        );

        let filtered_workspace_associations = run(Cli::try_parse_from([
            "workvcs",
            "resource",
            "workspace-association-list",
            store,
            "--workspace",
            &workspace_id,
            "--resource",
            &second_resource_id,
        ])
        .expect("parse filtered workspace association list"))
        .expect("list filtered workspace associations");
        assert_eq!(
            value(&filtered_workspace_associations, "workspace_id"),
            workspace_id
        );
        assert_eq!(
            value(&filtered_workspace_associations, "workspace_associations"),
            "1"
        );
        assert_eq!(
            value(
                &filtered_workspace_associations,
                "workspace_association.0.resource_id"
            ),
            second_resource_id
        );
        assert_eq!(
            value(
                &filtered_workspace_associations,
                "workspace_association.0.metadata_json"
            ),
            r#"{"role":"reference"}"#
        );

        let missing_workspace_associations = run(Cli::try_parse_from([
            "workvcs",
            "resource",
            "workspace-association-list",
            store,
            "--workspace",
            &workspace_id,
            "--resource",
            "018b4ed6-0e2f-7000-8000-000000000001",
        ])
        .expect("parse missing workspace association list"))
        .expect("list missing workspace associations");
        assert_eq!(
            value(&missing_workspace_associations, "workspace_associations"),
            "0"
        );

        let resources = run(Cli::try_parse_from(["workvcs", "resource", "list", store])
            .expect("parse resource list"))
        .expect("list resources");
        assert_eq!(value(&resources, "resources"), "2");
        let listed_ids = [
            value(&resources, "resource.0.resource_id"),
            value(&resources, "resource.1.resource_id"),
        ];
        assert!(listed_ids.contains(&resource_id));
        assert!(listed_ids.contains(&second_resource_id));

        let limited_resources =
            run(
                Cli::try_parse_from(["workvcs", "resource", "list", store, "--limit", "1"])
                    .expect("parse limited resource list"),
            )
            .expect("list limited resources");
        assert_eq!(value(&limited_resources, "resources"), "1");
        assert_ne!(value(&limited_resources, "resource.0.resource_id"), "");

        let zero_limit_resources =
            run(
                Cli::try_parse_from(["workvcs", "resource", "list", store, "--limit", "0"])
                    .expect("parse zero-limit resource list"),
            );
        assert!(zero_limit_resources.is_err());

        let bound_resources =
            run(
                Cli::try_parse_from(["workvcs", "resource", "list", store, "--bound", "true"])
                    .expect("parse bound resource list"),
            )
            .expect("list bound resources");
        assert_eq!(value(&bound_resources, "resources"), "1");
        assert_eq!(
            value(&bound_resources, "resource.0.resource_id"),
            resource_id
        );

        let unbound_resources =
            run(
                Cli::try_parse_from(["workvcs", "resource", "list", store, "--bound", "false"])
                    .expect("parse unbound resource list"),
            )
            .expect("list unbound resources");
        assert_eq!(value(&unbound_resources, "resources"), "1");
        assert_eq!(
            value(&unbound_resources, "resource.0.resource_id"),
            second_resource_id
        );

        let workspace_resources = run(Cli::try_parse_from([
            "workvcs",
            "resource",
            "list",
            store,
            "--workspace",
            &workspace_id,
        ])
        .expect("parse workspace resource list"))
        .expect("list workspace resources");
        assert_eq!(value(&workspace_resources, "resources"), "2");

        let missing_workspace_resources = run(Cli::try_parse_from([
            "workvcs",
            "resource",
            "list",
            store,
            "--workspace",
            "018b4ed6-0e2f-7000-8000-000000000002",
        ])
        .expect("parse missing workspace resource list"))
        .expect("list missing workspace resources");
        assert_eq!(value(&missing_workspace_resources, "resources"), "0");

        let filtered_resources = run(Cli::try_parse_from([
            "workvcs",
            "resource",
            "list",
            store,
            "--kind",
            "git-worktree",
            "--bound",
            "true",
        ])
        .expect("parse filtered resource list"))
        .expect("list filtered resources");
        assert_eq!(value(&filtered_resources, "resources"), "1");
        assert_eq!(
            value(&filtered_resources, "resource.0.resource_id"),
            resource_id
        );
        assert_eq!(
            value(&filtered_resources, "resource.0.resource_kind"),
            "git-worktree"
        );
        assert_eq!(
            value(&filtered_resources, "resource.0.binding_present"),
            "true"
        );
        assert_eq!(
            value(&filtered_resources, "resource.0.workspace_associations"),
            "1"
        );
    }

    #[test]
    fn cli_runs_resource_backed_verification_and_cache_workflow() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let path = tempdir.path().join("workvcs.sqlite");
        let store = path.to_str().expect("path text");
        let observation_file = tempdir.path().join("observation.bin");
        fs::write(&observation_file, b"baseline bytes").expect("write observation content");
        let observation_file_path = observation_file
            .to_str()
            .expect("observation file path text");
        let detail_file = tempdir.path().join("observation-detail.txt");
        fs::write(&detail_file, b"detail bytes").expect("write observation detail");
        let detail_file_path = detail_file.to_str().expect("detail file path text");

        run(
            Cli::try_parse_from(["workvcs", "init", store, "--display-name", "cli-store"])
                .expect("parse init"),
        )
        .expect("run init");
        let workspace = run(Cli::try_parse_from([
            "workvcs",
            "workspace",
            "create",
            store,
            "--display-name",
            "workspace",
        ])
        .expect("parse workspace"))
        .expect("create workspace");
        let workspace_id = value(&workspace, "workspace_id");
        let branch = value(&workspace, "branch_id");
        let mut head = value(&workspace, "genesis_commit_id");
        let session = run(Cli::try_parse_from([
            "workvcs",
            "session",
            "start",
            store,
            "--workspace",
            &workspace_id,
            "--branch",
            &branch,
            "--metadata-json",
            r#"{"purpose":"resource observation"}"#,
        ])
        .expect("parse session start"))
        .expect("start session");
        let session_id = value(&session, "session_id");

        let task = run(Cli::try_parse_from([
            "workvcs",
            "task",
            "create",
            store,
            "--branch",
            &branch,
            "--head",
            &head,
            "--description",
            "Ship resource-backed CLI workflow",
        ])
        .expect("parse task"))
        .expect("create task");
        let task_id = value(&task, "task_entity_id");
        head = value(&task, "commit_id");

        let criterion = run(Cli::try_parse_from([
            "workvcs",
            "ac",
            "create",
            store,
            "--branch",
            &branch,
            "--head",
            &head,
            "--task",
            &task_id,
            "--task-version",
            &value(&task, "task_entity_version_id"),
            "--local-key",
            "AC-1",
            "--statement",
            "The resource-backed CLI verification is applicable.",
        ])
        .expect("parse ac"))
        .expect("create ac");
        let criterion_id = value(&criterion, "acceptance_criterion_entity_id");
        let current_task_version = value(&criterion, "task_entity_version_id");
        head = value(&criterion, "commit_id");

        let resource =
            run(
                Cli::try_parse_from(["workvcs", "resource", "create", store, "--kind", "git"])
                    .expect("parse resource"),
            )
            .expect("create resource");
        let resource_id = value(&resource, "resource_id");
        let observation = run(Cli::try_parse_from([
            "workvcs",
            "resource",
            "observe",
            store,
            "--resource",
            &resource_id,
            "--adapter-kind",
            "git",
            "--adapter-schema-version",
            "1",
            "--content-file",
            observation_file_path,
            "--detail-content-file",
            detail_file_path,
            "--detail-media-type",
            "text/plain",
            "--detail-format-metadata-json",
            r#"{"encoding":"utf-8"}"#,
            "--source-session",
            &session_id,
        ])
        .expect("parse observe"))
        .expect("record observation");
        let observation_id = value(&observation, "observation_id");
        let fingerprint = value(&observation, "fingerprint");

        let shown_observation = run(Cli::try_parse_from([
            "workvcs",
            "resource",
            "observation-show",
            store,
            "--observation",
            &observation_id,
        ])
        .expect("parse observation show"))
        .expect("show observation");
        assert_eq!(value(&shown_observation, "observation_id"), observation_id);
        assert_eq!(value(&shown_observation, "resource_id"), resource_id);
        assert_eq!(value(&shown_observation, "adapter_kind"), "git");
        assert_eq!(value(&shown_observation, "adapter_schema_version"), "1");
        assert_eq!(value(&shown_observation, "fingerprint"), fingerprint);
        assert_eq!(value(&shown_observation, "summary_json"), "{}");
        assert_eq!(value(&shown_observation, "detail_content_present"), "true");
        assert_ne!(value(&shown_observation, "detail.content_digest"), "");
        assert_eq!(value(&shown_observation, "detail.size_bytes"), "12");
        assert_eq!(value(&shown_observation, "detail.media_type"), "text/plain");
        assert_eq!(
            value(&shown_observation, "detail.format_metadata_json"),
            r#"{"encoding":"utf-8"}"#
        );
        assert_eq!(value(&shown_observation, "source_session_id"), session_id);

        let duplicate_fingerprint_source = Cli::try_parse_from([
            "workvcs",
            "resource",
            "observe",
            store,
            "--resource",
            &resource_id,
            "--adapter-kind",
            "git",
            "--adapter-schema-version",
            "1",
            "--content",
            "baseline bytes",
            "--content-file",
            observation_file_path,
        ]);
        assert!(duplicate_fingerprint_source.is_err());

        let duplicate_detail_source = Cli::try_parse_from([
            "workvcs",
            "resource",
            "observe",
            store,
            "--resource",
            &resource_id,
            "--adapter-kind",
            "git",
            "--adapter-schema-version",
            "1",
            "--content-file",
            observation_file_path,
            "--detail-content",
            "detail bytes",
            "--detail-content-file",
            detail_file_path,
        ]);
        assert!(duplicate_detail_source.is_err());

        let observations =
            run(
                Cli::try_parse_from(["workvcs", "resource", "observation-list", store])
                    .expect("parse observation list"),
            )
            .expect("list observations");
        assert_eq!(value(&observations, "observations"), "1");
        assert_eq!(
            value(&observations, "observation.0.observation_id"),
            observation_id
        );
        assert_eq!(
            value(&observations, "observation.0.resource_id"),
            resource_id
        );
        assert_eq!(value(&observations, "observation.0.adapter_kind"), "git");
        assert_eq!(
            value(&observations, "observation.0.fingerprint"),
            fingerprint
        );
        assert_eq!(
            value(&observations, "observation.0.source_session_id"),
            session_id
        );

        let resource_observations = run(Cli::try_parse_from([
            "workvcs",
            "resource",
            "observation-list",
            store,
            "--resource",
            &resource_id,
        ])
        .expect("parse resource-filtered observation list"))
        .expect("list resource-filtered observations");
        assert_eq!(value(&resource_observations, "observations"), "1");
        assert_eq!(
            value(&resource_observations, "observation.0.observation_id"),
            observation_id
        );

        let limited_observations = run(Cli::try_parse_from([
            "workvcs",
            "resource",
            "observation-list",
            store,
            "--resource",
            &resource_id,
            "--limit",
            "1",
        ])
        .expect("parse limited observation list"))
        .expect("list limited observations");
        assert_eq!(value(&limited_observations, "observations"), "1");
        assert_eq!(
            value(&limited_observations, "observation.0.observation_id"),
            observation_id
        );

        let zero_limit_observations = run(Cli::try_parse_from([
            "workvcs",
            "resource",
            "observation-list",
            store,
            "--limit",
            "0",
        ])
        .expect("parse zero-limit observation list"));
        assert!(zero_limit_observations.is_err());

        let adapter_observations = run(Cli::try_parse_from([
            "workvcs",
            "resource",
            "observation-list",
            store,
            "--adapter-kind",
            "git",
        ])
        .expect("parse adapter-filtered observation list"))
        .expect("list adapter-filtered observations");
        assert_eq!(value(&adapter_observations, "observations"), "1");
        assert_eq!(
            value(&adapter_observations, "observation.0.observation_id"),
            observation_id
        );

        let schema_version_observations = run(Cli::try_parse_from([
            "workvcs",
            "resource",
            "observation-list",
            store,
            "--adapter-schema-version",
            "1",
        ])
        .expect("parse schema-version-filtered observation list"))
        .expect("list schema-version-filtered observations");
        assert_eq!(value(&schema_version_observations, "observations"), "1");
        assert_eq!(
            value(&schema_version_observations, "observation.0.observation_id"),
            observation_id
        );

        let session_observations = run(Cli::try_parse_from([
            "workvcs",
            "resource",
            "observation-list",
            store,
            "--source-session",
            &session_id,
        ])
        .expect("parse source-session-filtered observation list"))
        .expect("list source-session-filtered observations");
        assert_eq!(value(&session_observations, "observations"), "1");
        assert_eq!(
            value(&session_observations, "observation.0.observation_id"),
            observation_id
        );
        assert_eq!(
            value(&session_observations, "observation.0.source_session_id"),
            session_id
        );

        let combined_observations = run(Cli::try_parse_from([
            "workvcs",
            "resource",
            "observation-list",
            store,
            "--resource",
            &resource_id,
            "--adapter-kind",
            "git",
            "--adapter-schema-version",
            "1",
            "--source-session",
            &session_id,
        ])
        .expect("parse combined observation list"))
        .expect("list combined observations");
        assert_eq!(value(&combined_observations, "observations"), "1");
        assert_eq!(
            value(&combined_observations, "observation.0.observation_id"),
            observation_id
        );

        let missing_observations = run(Cli::try_parse_from([
            "workvcs",
            "resource",
            "observation-list",
            store,
            "--adapter-kind",
            "http",
        ])
        .expect("parse missing observation list"))
        .expect("list missing observations");
        assert_eq!(value(&missing_observations, "observations"), "0");

        let missing_schema_observations = run(Cli::try_parse_from([
            "workvcs",
            "resource",
            "observation-list",
            store,
            "--adapter-schema-version",
            "2",
        ])
        .expect("parse missing schema-version observation list"))
        .expect("list missing schema-version observations");
        assert_eq!(value(&missing_schema_observations, "observations"), "0");

        let verification = run(Cli::try_parse_from([
            "workvcs",
            "verification",
            "record",
            store,
            "--branch",
            &branch,
            "--head",
            &head,
            "--acceptance-criterion",
            &criterion_id,
            "--result",
            "passed",
            "--method",
            "manual-review",
            "--resource",
            &resource_id,
            "--adapter-kind",
            "git",
            "--adapter-schema-version",
            "1",
            "--scope-kind",
            "path",
            "--scope-schema-version",
            "1",
            "--scope-payload-json",
            "{\"path\":\"src/lib.rs\"}",
            "--baseline-fingerprint",
            &fingerprint,
            "--baseline-observation",
            &observation_id,
        ])
        .expect("parse verification"))
        .expect("record verification");
        let verification_id = value(&verification, "verification_entity_id");
        head = value(&verification, "commit_id");

        let missing_cache = run(Cli::try_parse_from([
            "workvcs",
            "verification",
            "cache-show",
            store,
            "--branch",
            &branch,
            "--verification",
            &verification_id,
        ])
        .expect("parse missing cache show"))
        .expect("show missing cache");
        assert_eq!(value(&missing_cache, "cache_found"), "false");

        let stale = run(Cli::try_parse_from([
            "workvcs",
            "ac",
            "status",
            store,
            "--branch",
            &branch,
            "--criterion",
            &criterion_id,
        ])
        .expect("parse stale status"))
        .expect("stale status");
        assert_eq!(stale, "status=stale\n");

        let cache = run(Cli::try_parse_from([
            "workvcs",
            "verification",
            "cache-record",
            store,
            "--branch",
            &branch,
            "--head",
            &head,
            "--verification",
            &verification_id,
            "--adapter-kind",
            "git",
            "--adapter-schema-version",
            "1",
            "--scope-schema-version",
            "1",
            "--observation-status",
            "observed",
            "--observed-fingerprint",
            &fingerprint,
            "--observation",
            &observation_id,
        ])
        .expect("parse cache"))
        .expect("record cache");
        assert!(cache.contains("applicability=applicable"));
        assert!(cache.contains("reason_code=all_basis_applicable"));

        let shown_cache = run(Cli::try_parse_from([
            "workvcs",
            "verification",
            "cache-show",
            store,
            "--branch",
            &branch,
            "--verification",
            &verification_id,
        ])
        .expect("parse cache show"))
        .expect("show cache");
        assert_eq!(value(&shown_cache, "cache_found"), "true");
        assert_eq!(
            value(&shown_cache, "evaluated_commit_id"),
            value(&verification, "commit_id")
        );
        assert_eq!(value(&shown_cache, "applicability"), "applicable");
        assert_eq!(value(&shown_cache, "reason_code"), "all_basis_applicable");
        assert_eq!(value(&shown_cache, "detail_json"), "{}");
        assert_eq!(value(&shown_cache, "resource_stamps"), "1");
        assert_eq!(
            value(&shown_cache, "resource_stamp.0.resource_basis_ordinal"),
            "0"
        );
        assert_eq!(value(&shown_cache, "resource_stamp.0.adapter_kind"), "git");
        assert_eq!(
            value(&shown_cache, "resource_stamp.0.observation_status"),
            "observed"
        );
        assert_eq!(
            value(&shown_cache, "resource_stamp.0.observed_fingerprint"),
            fingerprint
        );
        assert_eq!(
            value(&shown_cache, "resource_stamp.0.observation_id"),
            observation_id
        );

        let cache_list = run(Cli::try_parse_from([
            "workvcs",
            "verification",
            "cache-list",
            store,
            "--branch",
            &branch,
        ])
        .expect("parse cache list"))
        .expect("list caches");
        assert_eq!(value(&cache_list, "branch_id"), branch);
        assert_eq!(value(&cache_list, "caches"), "1");
        assert_eq!(
            value(&cache_list, "cache.0.verification_entity_id"),
            verification_id
        );
        assert_eq!(
            value(&cache_list, "cache.0.evaluated_commit_id"),
            value(&verification, "commit_id")
        );
        assert_eq!(value(&cache_list, "cache.0.applicability"), "applicable");
        assert_eq!(
            value(&cache_list, "cache.0.reason_code"),
            "all_basis_applicable"
        );
        assert_eq!(value(&cache_list, "cache.0.resource_stamps"), "1");

        let cache_list_by_verification = run(Cli::try_parse_from([
            "workvcs",
            "verification",
            "cache-list",
            store,
            "--branch",
            &branch,
            "--verification",
            &verification_id,
        ])
        .expect("parse verification-filtered cache list"))
        .expect("list verification-filtered caches");
        assert_eq!(value(&cache_list_by_verification, "caches"), "1");
        assert_eq!(
            value(
                &cache_list_by_verification,
                "cache.0.verification_entity_id"
            ),
            verification_id
        );

        let cache_list_by_applicability = run(Cli::try_parse_from([
            "workvcs",
            "verification",
            "cache-list",
            store,
            "--branch",
            &branch,
            "--applicability",
            "applicable",
        ])
        .expect("parse applicability-filtered cache list"))
        .expect("list applicability-filtered caches");
        assert_eq!(value(&cache_list_by_applicability, "caches"), "1");
        assert_eq!(
            value(
                &cache_list_by_applicability,
                "cache.0.verification_entity_id"
            ),
            verification_id
        );

        let cache_list_by_reason = run(Cli::try_parse_from([
            "workvcs",
            "verification",
            "cache-list",
            store,
            "--branch",
            &branch,
            "--reason-code",
            "all_basis_applicable",
        ])
        .expect("parse reason-filtered cache list"))
        .expect("list reason-filtered caches");
        assert_eq!(value(&cache_list_by_reason, "caches"), "1");
        assert_eq!(
            value(&cache_list_by_reason, "cache.0.verification_entity_id"),
            verification_id
        );

        let limited_cache_list = run(Cli::try_parse_from([
            "workvcs",
            "verification",
            "cache-list",
            store,
            "--branch",
            &branch,
            "--limit",
            "1",
        ])
        .expect("parse limited cache list"))
        .expect("list limited caches");
        assert_eq!(value(&limited_cache_list, "caches"), "1");
        assert_eq!(
            value(&limited_cache_list, "cache.0.verification_entity_id"),
            verification_id
        );

        let zero_limit_cache_list = run(Cli::try_parse_from([
            "workvcs",
            "verification",
            "cache-list",
            store,
            "--branch",
            &branch,
            "--limit",
            "0",
        ])
        .expect("parse zero-limit cache list"));
        assert!(zero_limit_cache_list.is_err());

        let missing_reason_cache_list = run(Cli::try_parse_from([
            "workvcs",
            "verification",
            "cache-list",
            store,
            "--branch",
            &branch,
            "--reason-code",
            "resource_stale",
        ])
        .expect("parse missing reason cache list"))
        .expect("list missing reason caches");
        assert_eq!(value(&missing_reason_cache_list, "caches"), "0");

        let missing_cache_list = run(Cli::try_parse_from([
            "workvcs",
            "verification",
            "cache-list",
            store,
            "--branch",
            &branch,
            "--applicability",
            "stale",
        ])
        .expect("parse missing cache list"))
        .expect("list missing caches");
        assert_eq!(value(&missing_cache_list, "caches"), "0");

        let verified = run(Cli::try_parse_from([
            "workvcs",
            "ac",
            "status",
            store,
            "--branch",
            &branch,
            "--criterion",
            &criterion_id,
        ])
        .expect("parse verified"))
        .expect("verified status");
        assert_eq!(verified, "status=verified\n");

        let transition = run(Cli::try_parse_from([
            "workvcs",
            "task",
            "transition",
            store,
            "--branch",
            &branch,
            "--head",
            &head,
            "--task",
            &task_id,
            "--task-version",
            &current_task_version,
            "--status",
            "done",
            "--outcome",
            "completed",
        ])
        .expect("parse transition"))
        .expect("transition task");
        assert!(transition.contains("status=done"));
    }

    #[test]
    fn cli_runs_runtime_runnable_and_claim_workflow() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let path = tempdir.path().join("workvcs.sqlite");
        let store = path.to_str().expect("path text");

        run(
            Cli::try_parse_from(["workvcs", "init", store, "--display-name", "cli-store"])
                .expect("parse init"),
        )
        .expect("run init");
        let workspace = run(Cli::try_parse_from([
            "workvcs",
            "workspace",
            "create",
            store,
            "--display-name",
            "workspace",
        ])
        .expect("parse workspace"))
        .expect("create workspace");
        let workspace_id = value(&workspace, "workspace_id");
        let branch = value(&workspace, "branch_id");
        let head = value(&workspace, "genesis_commit_id");

        let task = run(Cli::try_parse_from([
            "workvcs",
            "task",
            "create",
            store,
            "--branch",
            &branch,
            "--head",
            &head,
            "--description",
            "Claim runnable task",
        ])
        .expect("parse task"))
        .expect("create task");
        let task_id = value(&task, "task_entity_id");

        let session = run(Cli::try_parse_from([
            "workvcs",
            "session",
            "start",
            store,
            "--workspace",
            &workspace_id,
            "--branch",
            &branch,
        ])
        .expect("parse session start"))
        .expect("start session");
        let session_id = value(&session, "session_id");
        assert!(session.contains("lifecycle_state=active"));

        let context =
            run(
                Cli::try_parse_from(["workvcs", "context", store, "--session", &session_id])
                    .expect("parse context"),
            )
            .expect("context overview");
        assert!(context.contains("lifecycle_state=active"));
        assert!(context.contains(&format!("workspace_id={workspace_id}")));
        assert!(context.contains(&format!("branch_id={branch}")));
        assert!(context.contains("context_workspaces=1"));
        assert!(context.contains("runnable_candidates=1"));
        assert!(context.contains("knowledge=0"));
        assert!(context.contains("records=0"));
        assert!(context.contains("record_relations=0"));
        assert!(context.contains(&format!("candidate.0.task_entity_id={task_id}")));

        let runnable = run(Cli::try_parse_from([
            "workvcs",
            "runnable",
            "tasks",
            store,
            "--session",
            &session_id,
        ])
        .expect("parse runnable"))
        .expect("runnable tasks");
        assert!(runnable.contains("candidates=1"));
        assert!(runnable.contains(&format!("candidate.0.task_entity_id={task_id}")));
        assert!(runnable.contains("candidate.0.runnable=true"));
        assert!(runnable.contains("candidate.0.claim=unclaimed"));

        let runnable_by_task = run(Cli::try_parse_from([
            "workvcs",
            "runnable",
            "tasks",
            store,
            "--session",
            &session_id,
            "--task",
            &task_id,
        ])
        .expect("parse runnable by task"))
        .expect("runnable tasks by task");
        assert_eq!(value(&runnable_by_task, "candidates"), "1");
        assert_eq!(
            value(&runnable_by_task, "candidate.0.task_entity_id"),
            task_id
        );

        let runnable_by_status = run(Cli::try_parse_from([
            "workvcs",
            "runnable",
            "tasks",
            store,
            "--session",
            &session_id,
            "--status",
            "pending",
            "--runnable",
            "true",
        ])
        .expect("parse runnable by status"))
        .expect("runnable tasks by status");
        assert_eq!(value(&runnable_by_status, "candidates"), "1");
        assert_eq!(
            value(&runnable_by_status, "candidate.0.task_entity_id"),
            task_id
        );

        let non_runnable = run(Cli::try_parse_from([
            "workvcs",
            "runnable",
            "tasks",
            store,
            "--session",
            &session_id,
            "--runnable",
            "false",
        ])
        .expect("parse non-runnable tasks"))
        .expect("non-runnable tasks");
        assert_eq!(value(&non_runnable, "candidates"), "0");

        let missing_runnable_task = run(Cli::try_parse_from([
            "workvcs",
            "runnable",
            "tasks",
            store,
            "--session",
            &session_id,
            "--task",
            "018b4ed6-0e2f-7000-8000-000000000004",
        ])
        .expect("parse missing runnable task"))
        .expect("missing runnable task");
        assert_eq!(value(&missing_runnable_task, "candidates"), "0");

        let claim = run(Cli::try_parse_from([
            "workvcs",
            "claim",
            "task",
            store,
            "--session",
            &session_id,
            "--task",
            &task_id,
        ])
        .expect("parse claim"))
        .expect("claim task");
        let claim_id = value(&claim, "claim_id");
        assert!(claim.contains("mode=exclusive"));
        assert!(claim.contains("lifecycle_state=active"));

        let claimed_runnable = run(Cli::try_parse_from([
            "workvcs",
            "runnable",
            "tasks",
            store,
            "--session",
            &session_id,
        ])
        .expect("parse claimed runnable"))
        .expect("claimed runnable tasks");
        assert!(
            claimed_runnable.contains(&format!("candidate.0.claim=claimed_by_session:{claim_id}"))
        );

        let release = run(Cli::try_parse_from([
            "workvcs",
            "claim",
            "release",
            store,
            "--session",
            &session_id,
            "--claim",
            &claim_id,
        ])
        .expect("parse release"))
        .expect("release claim");
        assert!(release.contains("lifecycle_state=released"));

        let ended = run(Cli::try_parse_from([
            "workvcs",
            "session",
            "end",
            store,
            "--session",
            &session_id,
        ])
        .expect("parse session end"))
        .expect("end session");
        assert!(ended.contains("lifecycle_state=ended"));
    }

    #[test]
    fn cli_shows_and_lists_session_active_claims() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let path = tempdir.path().join("workvcs.sqlite");
        let store = path.to_str().expect("path text");

        run(
            Cli::try_parse_from(["workvcs", "init", store, "--display-name", "cli-store"])
                .expect("parse init"),
        )
        .expect("run init");
        let workspace = run(Cli::try_parse_from([
            "workvcs",
            "workspace",
            "create",
            store,
            "--display-name",
            "workspace",
        ])
        .expect("parse workspace"))
        .expect("create workspace");
        let workspace_id = value(&workspace, "workspace_id");
        let branch = value(&workspace, "branch_id");
        let head = value(&workspace, "genesis_commit_id");
        let task = run(Cli::try_parse_from([
            "workvcs",
            "task",
            "create",
            store,
            "--branch",
            &branch,
            "--head",
            &head,
            "--description",
            "Observable claim task",
        ])
        .expect("parse task"))
        .expect("create task");
        let task_id = value(&task, "task_entity_id");
        let task_head = value(&task, "commit_id");
        let session = run(Cli::try_parse_from([
            "workvcs",
            "session",
            "start",
            store,
            "--workspace",
            &workspace_id,
            "--branch",
            &branch,
        ])
        .expect("parse session start"))
        .expect("start session");
        let session_id = value(&session, "session_id");

        let claim = run(Cli::try_parse_from([
            "workvcs",
            "claim",
            "task",
            store,
            "--session",
            &session_id,
            "--task",
            &task_id,
        ])
        .expect("parse claim"))
        .expect("claim task");
        let claim_id = value(&claim, "claim_id");

        let shown =
            run(
                Cli::try_parse_from(["workvcs", "claim", "show", store, "--claim", &claim_id])
                    .expect("parse claim show"),
            )
            .expect("claim show");
        assert_eq!(value(&shown, "claim_id"), claim_id);
        assert_eq!(value(&shown, "session_id"), session_id);
        assert_eq!(value(&shown, "task_entity_id"), task_id);
        assert_eq!(value(&shown, "mode"), "exclusive");
        assert_eq!(value(&shown, "lifecycle_state"), "active");

        let listed =
            run(
                Cli::try_parse_from(["workvcs", "claim", "list", store, "--session", &session_id])
                    .expect("parse claim list"),
            )
            .expect("claim list");
        assert_eq!(value(&listed, "claims"), "1");
        assert_eq!(value(&listed, "claim.0.claim_id"), claim_id);
        assert_eq!(value(&listed, "claim.0.task_entity_id"), task_id);
        assert_eq!(value(&listed, "claim.0.lifecycle_state"), "active");

        let listed_by_task = run(Cli::try_parse_from([
            "workvcs",
            "claim",
            "list",
            store,
            "--session",
            &session_id,
            "--task",
            &task_id,
        ])
        .expect("parse claim list by task"))
        .expect("claim list by task");
        assert_eq!(value(&listed_by_task, "claims"), "1");
        assert_eq!(value(&listed_by_task, "claim.0.claim_id"), claim_id);

        let listed_by_exclusive_mode = run(Cli::try_parse_from([
            "workvcs",
            "claim",
            "list",
            store,
            "--session",
            &session_id,
            "--mode",
            "exclusive",
        ])
        .expect("parse claim list by exclusive mode"))
        .expect("claim list by exclusive mode");
        assert_eq!(value(&listed_by_exclusive_mode, "claims"), "1");

        let listed_by_shared_mode = run(Cli::try_parse_from([
            "workvcs",
            "claim",
            "list",
            store,
            "--session",
            &session_id,
            "--mode",
            "shared",
        ])
        .expect("parse claim list by shared mode"))
        .expect("claim list by shared mode");
        assert_eq!(value(&listed_by_shared_mode, "claims"), "0");

        let listed_by_combined_filters = run(Cli::try_parse_from([
            "workvcs",
            "claim",
            "list",
            store,
            "--session",
            &session_id,
            "--task",
            &task_id,
            "--mode",
            "exclusive",
        ])
        .expect("parse claim list by combined filters"))
        .expect("claim list by combined filters");
        assert_eq!(value(&listed_by_combined_filters, "claims"), "1");

        let guard = run(Cli::try_parse_from([
            "workvcs",
            "claim",
            "guard",
            store,
            "--session",
            &session_id,
            "--task",
            &task_id,
        ])
        .expect("parse claim guard"))
        .expect("claim guard");
        assert_eq!(value(&guard, "allowed"), "true");
        assert_eq!(value(&guard, "reason"), "owned_exclusive_claim");
        assert_eq!(value(&guard, "active_claims"), "1");
        assert_eq!(value(&guard, "active_claim.0.claim_id"), claim_id);
        let failed = run(Cli::try_parse_from([
            "workvcs",
            "task",
            "transition",
            store,
            "--branch",
            &branch,
            "--head",
            &task_head,
            "--task",
            &task_id,
            "--task-version",
            &value(&task, "task_entity_version_id"),
            "--status",
            "failed",
            "--session",
            &session_id,
        ])
        .expect("parse guarded terminal transition"))
        .expect("guarded terminal transition");
        assert_eq!(value(&failed, "status"), "failed");

        run(Cli::try_parse_from([
            "workvcs",
            "claim",
            "release",
            store,
            "--session",
            &session_id,
            "--claim",
            &claim_id,
        ])
        .expect("parse release"))
        .expect("release claim");
        let empty =
            run(
                Cli::try_parse_from(["workvcs", "claim", "list", store, "--session", &session_id])
                    .expect("parse empty claim list"),
            )
            .expect("empty claim list");
        assert_eq!(value(&empty, "claims"), "0");
        let released =
            run(
                Cli::try_parse_from(["workvcs", "claim", "show", store, "--claim", &claim_id])
                    .expect("parse released claim show"),
            )
            .expect("released claim show");
        assert_eq!(value(&released, "lifecycle_state"), "released");
    }

    #[test]
    fn cli_claim_task_accepts_explicit_shared_mode() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let path = tempdir.path().join("workvcs.sqlite");
        let store = path.to_str().expect("path text");

        run(
            Cli::try_parse_from(["workvcs", "init", store, "--display-name", "cli-store"])
                .expect("parse init"),
        )
        .expect("run init");
        let workspace = run(Cli::try_parse_from([
            "workvcs",
            "workspace",
            "create",
            store,
            "--display-name",
            "workspace",
        ])
        .expect("parse workspace"))
        .expect("create workspace");
        let workspace_id = value(&workspace, "workspace_id");
        let branch = value(&workspace, "branch_id");
        let head = value(&workspace, "genesis_commit_id");
        let task = run(Cli::try_parse_from([
            "workvcs",
            "task",
            "create",
            store,
            "--branch",
            &branch,
            "--head",
            &head,
            "--description",
            "Shared CLI claim task",
        ])
        .expect("parse task"))
        .expect("create task");
        let task_id = value(&task, "task_entity_id");

        let first_session = run(Cli::try_parse_from([
            "workvcs",
            "session",
            "start",
            store,
            "--workspace",
            &workspace_id,
            "--branch",
            &branch,
        ])
        .expect("parse first session"))
        .expect("start first session");
        let first_session_id = value(&first_session, "session_id");
        let second_session = run(Cli::try_parse_from([
            "workvcs",
            "session",
            "start",
            store,
            "--workspace",
            &workspace_id,
            "--branch",
            &branch,
        ])
        .expect("parse second session"))
        .expect("start second session");
        let second_session_id = value(&second_session, "session_id");

        let first_claim = run(Cli::try_parse_from([
            "workvcs",
            "claim",
            "task",
            store,
            "--session",
            &first_session_id,
            "--task",
            &task_id,
            "--mode",
            "shared",
        ])
        .expect("parse first shared claim"))
        .expect("first shared claim");
        assert!(first_claim.contains("mode=shared"));
        assert!(first_claim.contains("lifecycle_state=active"));
        let second_claim = run(Cli::try_parse_from([
            "workvcs",
            "claim",
            "task",
            store,
            "--session",
            &second_session_id,
            "--task",
            &task_id,
            "--mode",
            "shared",
        ])
        .expect("parse second shared claim"))
        .expect("second shared claim");
        assert!(second_claim.contains("mode=shared"));

        let runnable = run(Cli::try_parse_from([
            "workvcs",
            "runnable",
            "tasks",
            store,
            "--session",
            &first_session_id,
        ])
        .expect("parse runnable"))
        .expect("runnable with shared claim");
        assert!(runnable.contains("candidate.0.claim=shared:true:"));
    }

    #[test]
    fn cli_claim_next_accepts_explicit_shared_mode() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let path = tempdir.path().join("workvcs.sqlite");
        let store = path.to_str().expect("path text");

        run(
            Cli::try_parse_from(["workvcs", "init", store, "--display-name", "cli-store"])
                .expect("parse init"),
        )
        .expect("run init");
        let workspace = run(Cli::try_parse_from([
            "workvcs",
            "workspace",
            "create",
            store,
            "--display-name",
            "workspace",
        ])
        .expect("parse workspace"))
        .expect("create workspace");
        let workspace_id = value(&workspace, "workspace_id");
        let branch = value(&workspace, "branch_id");
        let head = value(&workspace, "genesis_commit_id");
        let task = run(Cli::try_parse_from([
            "workvcs",
            "task",
            "create",
            store,
            "--branch",
            &branch,
            "--head",
            &head,
            "--description",
            "Shared CLI claim next task",
        ])
        .expect("parse task"))
        .expect("create task");
        let task_id = value(&task, "task_entity_id");

        let first_session = run(Cli::try_parse_from([
            "workvcs",
            "session",
            "start",
            store,
            "--workspace",
            &workspace_id,
            "--branch",
            &branch,
        ])
        .expect("parse first session"))
        .expect("start first session");
        let first_session_id = value(&first_session, "session_id");
        let second_session = run(Cli::try_parse_from([
            "workvcs",
            "session",
            "start",
            store,
            "--workspace",
            &workspace_id,
            "--branch",
            &branch,
        ])
        .expect("parse second session"))
        .expect("start second session");
        let second_session_id = value(&second_session, "session_id");

        run(Cli::try_parse_from([
            "workvcs",
            "claim",
            "task",
            store,
            "--session",
            &first_session_id,
            "--task",
            &task_id,
            "--mode",
            "shared",
        ])
        .expect("parse first shared claim"))
        .expect("first shared claim");

        let claimed = run(Cli::try_parse_from([
            "workvcs",
            "claim",
            "next",
            store,
            "--session",
            &second_session_id,
            "--mode",
            "shared",
        ])
        .expect("parse shared claim next"))
        .expect("shared claim next");
        assert_eq!(value(&claimed, "selected"), "true");
        assert_eq!(value(&claimed, "task_entity_id"), task_id);
        assert_eq!(value(&claimed, "mode"), "shared");

        let runnable = run(Cli::try_parse_from([
            "workvcs",
            "runnable",
            "tasks",
            store,
            "--session",
            &second_session_id,
        ])
        .expect("parse runnable"))
        .expect("runnable after shared claim next");
        assert!(runnable.contains("candidate.0.claim=shared:true:"));
    }

    #[test]
    fn cli_context_includes_current_record_summary() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let path = tempdir.path().join("workvcs.sqlite");
        let store = path.to_str().expect("path text");

        run(
            Cli::try_parse_from(["workvcs", "init", store, "--display-name", "cli-store"])
                .expect("parse init"),
        )
        .expect("run init");
        let workspace = run(Cli::try_parse_from([
            "workvcs",
            "workspace",
            "create",
            store,
            "--display-name",
            "workspace",
        ])
        .expect("parse workspace"))
        .expect("create workspace");
        let workspace_id = value(&workspace, "workspace_id");
        let branch = value(&workspace, "branch_id");
        let head = value(&workspace, "genesis_commit_id");

        let finding = run(Cli::try_parse_from([
            "workvcs",
            "record",
            "finding",
            store,
            "--branch",
            &branch,
            "--head",
            &head,
            "--statement",
            "The context resolver should expose current findings",
        ])
        .expect("parse finding"))
        .expect("create finding");

        let session = run(Cli::try_parse_from([
            "workvcs",
            "session",
            "start",
            store,
            "--workspace",
            &workspace_id,
            "--branch",
            &branch,
        ])
        .expect("parse session start"))
        .expect("start session");
        let session_id = value(&session, "session_id");

        let context =
            run(
                Cli::try_parse_from(["workvcs", "context", store, "--session", &session_id])
                    .expect("parse context"),
            )
            .expect("context overview");
        assert!(context.contains("knowledge=0"));
        assert!(context.contains("records=1"));
        assert!(context.contains("record_relations=0"));
        assert_eq!(
            value(&context, "context_record.0.record_entity_id"),
            value(&finding, "record_entity_id")
        );
        assert!(context.contains("context_record.0.record_kind=finding"));
        assert!(context.contains("context_record.0.record_status=active"));
        assert!(
            context.contains(
                "context_record.0.record_statement_json=\"The context resolver should expose current findings\""
            )
        );
    }

    #[test]
    fn cli_context_includes_current_active_knowledge_summary() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let path = tempdir.path().join("workvcs.sqlite");
        let store = path.to_str().expect("path text");

        run(
            Cli::try_parse_from(["workvcs", "init", store, "--display-name", "cli-store"])
                .expect("parse init"),
        )
        .expect("run init");
        let workspace = run(Cli::try_parse_from([
            "workvcs",
            "workspace",
            "create",
            store,
            "--display-name",
            "workspace",
        ])
        .expect("parse workspace"))
        .expect("create workspace");
        let workspace_id = value(&workspace, "workspace_id");
        let branch = value(&workspace, "branch_id");
        let head = value(&workspace, "genesis_commit_id");

        let active = run(Cli::try_parse_from([
            "workvcs",
            "knowledge",
            "create",
            store,
            "--branch",
            &branch,
            "--head",
            &head,
            "--statement",
            "Serialized writes make SQLite sufficient for V0.1",
            "--scope-json",
            "{\"kind\":\"workspace\"}",
        ])
        .expect("parse active knowledge"))
        .expect("create active knowledge");
        let stale = run(Cli::try_parse_from([
            "workvcs",
            "knowledge",
            "create",
            store,
            "--branch",
            &branch,
            "--head",
            &value(&active, "commit_id"),
            "--statement",
            "Legacy cache keys do not need schema versions",
        ])
        .expect("parse stale knowledge"))
        .expect("create stale knowledge");
        let invalidated = run(Cli::try_parse_from([
            "workvcs",
            "knowledge",
            "invalidate",
            store,
            "--branch",
            &branch,
            "--head",
            &value(&stale, "commit_id"),
            "--knowledge",
            &value(&stale, "knowledge_entity_id"),
            "--knowledge-version",
            &value(&stale, "knowledge_entity_version_id"),
            "--rationale",
            "Schema versions are part of cache keys",
        ])
        .expect("parse invalidate knowledge"))
        .expect("invalidate knowledge");

        let session = run(Cli::try_parse_from([
            "workvcs",
            "session",
            "start",
            store,
            "--workspace",
            &workspace_id,
            "--branch",
            &branch,
        ])
        .expect("parse session start"))
        .expect("start session");
        let session_id = value(&session, "session_id");

        let context =
            run(
                Cli::try_parse_from(["workvcs", "context", store, "--session", &session_id])
                    .expect("parse context"),
            )
            .expect("context overview");
        assert_eq!(
            value(&context, "head_commit_id"),
            value(&invalidated, "commit_id")
        );
        assert_eq!(value(&context, "knowledge"), "1");
        assert_eq!(
            value(&context, "context_knowledge.0.knowledge_entity_id"),
            value(&active, "knowledge_entity_id")
        );
        assert!(context.contains(
            "context_knowledge.0.knowledge_statement_json=\"Serialized writes make SQLite sufficient for V0.1\""
        ));
        assert!(
            context.contains("context_knowledge.0.knowledge_scope_json={\"kind\":\"workspace\"}")
        );
        assert!(!context.contains("Legacy cache keys do not need schema versions"));
    }

    #[test]
    fn cli_context_includes_current_record_relation_summary() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let path = tempdir.path().join("workvcs.sqlite");
        let store = path.to_str().expect("path text");

        run(
            Cli::try_parse_from(["workvcs", "init", store, "--display-name", "cli-store"])
                .expect("parse init"),
        )
        .expect("run init");
        let workspace = run(Cli::try_parse_from([
            "workvcs",
            "workspace",
            "create",
            store,
            "--display-name",
            "workspace",
        ])
        .expect("parse workspace"))
        .expect("create workspace");
        let workspace_id = value(&workspace, "workspace_id");
        let branch = value(&workspace, "branch_id");
        let head = value(&workspace, "genesis_commit_id");

        let decision = run(Cli::try_parse_from([
            "workvcs",
            "record",
            "decision",
            store,
            "--branch",
            &branch,
            "--head",
            &head,
            "--statement",
            "Use serialized writes",
        ])
        .expect("parse decision"))
        .expect("create decision");
        let finding = run(Cli::try_parse_from([
            "workvcs",
            "record",
            "finding",
            store,
            "--branch",
            &branch,
            "--head",
            &value(&decision, "commit_id"),
            "--statement",
            "Benchmarks support serialized writes",
        ])
        .expect("parse finding"))
        .expect("create finding");
        let relation = run(Cli::try_parse_from([
            "workvcs",
            "record",
            "link-supports",
            store,
            "--branch",
            &branch,
            "--head",
            &value(&finding, "commit_id"),
            "--source-record",
            &value(&finding, "record_entity_id"),
            "--target-record",
            &value(&decision, "record_entity_id"),
            "--rationale",
            "Finding supports the decision",
        ])
        .expect("parse link supports"))
        .expect("link supports");

        let session = run(Cli::try_parse_from([
            "workvcs",
            "session",
            "start",
            store,
            "--workspace",
            &workspace_id,
            "--branch",
            &branch,
        ])
        .expect("parse session start"))
        .expect("start session");
        let session_id = value(&session, "session_id");

        let context =
            run(
                Cli::try_parse_from(["workvcs", "context", store, "--session", &session_id])
                    .expect("parse context"),
            )
            .expect("context overview");
        assert!(context.contains("records=2"));
        assert!(context.contains("record_relations=1"));
        assert_eq!(
            value(&context, "context_record_relation.0.relation_id"),
            value(&relation, "relation_id")
        );
        assert!(context.contains("context_record_relation.0.relation_type=supports"));
        assert_eq!(
            value(
                &context,
                "context_record_relation.0.source_record_entity_id"
            ),
            value(&finding, "record_entity_id")
        );
        assert_eq!(
            value(
                &context,
                "context_record_relation.0.target_record_entity_id"
            ),
            value(&decision, "record_entity_id")
        );
    }

    #[test]
    fn cli_context_includes_current_record_knowledge_relation_summary() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let path = tempdir.path().join("workvcs.sqlite");
        let store = path.to_str().expect("path text");

        run(
            Cli::try_parse_from(["workvcs", "init", store, "--display-name", "cli-store"])
                .expect("parse init"),
        )
        .expect("run init");
        let workspace = run(Cli::try_parse_from([
            "workvcs",
            "workspace",
            "create",
            store,
            "--display-name",
            "workspace",
        ])
        .expect("parse workspace"))
        .expect("create workspace");
        let workspace_id = value(&workspace, "workspace_id");
        let branch = value(&workspace, "branch_id");

        let knowledge = run(Cli::try_parse_from([
            "workvcs",
            "knowledge",
            "create",
            store,
            "--branch",
            &branch,
            "--head",
            &value(&workspace, "genesis_commit_id"),
            "--statement",
            "Context overview exposes Record-to-Knowledge support",
        ])
        .expect("parse knowledge create"))
        .expect("create knowledge");
        let finding = run(Cli::try_parse_from([
            "workvcs",
            "record",
            "finding",
            store,
            "--branch",
            &branch,
            "--head",
            &value(&knowledge, "commit_id"),
            "--statement",
            "The context output includes Record-to-Knowledge relations",
        ])
        .expect("parse finding"))
        .expect("create finding");
        let relation = run(Cli::try_parse_from([
            "workvcs",
            "record",
            "link-supports-knowledge",
            store,
            "--branch",
            &branch,
            "--head",
            &value(&finding, "commit_id"),
            "--source-record",
            &value(&finding, "record_entity_id"),
            "--target-knowledge",
            &value(&knowledge, "knowledge_entity_id"),
            "--rationale",
            "The Finding supports the Knowledge statement",
        ])
        .expect("parse link supports knowledge"))
        .expect("link supports knowledge");

        let session = run(Cli::try_parse_from([
            "workvcs",
            "session",
            "start",
            store,
            "--workspace",
            &workspace_id,
            "--branch",
            &branch,
        ])
        .expect("parse session start"))
        .expect("start session");
        let session_id = value(&session, "session_id");

        let context =
            run(
                Cli::try_parse_from(["workvcs", "context", store, "--session", &session_id])
                    .expect("parse context"),
            )
            .expect("context overview");
        assert!(context.contains("record_relations=0"));
        assert!(context.contains("record_knowledge_relations=1"));
        assert_eq!(
            value(&context, "context_record_knowledge_relation.0.relation_id"),
            value(&relation, "relation_id")
        );
        assert!(context.contains("context_record_knowledge_relation.0.relation_type=supports"));
        assert_eq!(
            value(
                &context,
                "context_record_knowledge_relation.0.source_record_entity_id"
            ),
            value(&finding, "record_entity_id")
        );
        assert_eq!(
            value(
                &context,
                "context_record_knowledge_relation.0.target_knowledge_entity_id"
            ),
            value(&knowledge, "knowledge_entity_id")
        );
    }

    #[test]
    fn cli_context_includes_current_knowledge_relation_summary() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let path = tempdir.path().join("workvcs.sqlite");
        let store = path.to_str().expect("path text");

        run(
            Cli::try_parse_from(["workvcs", "init", store, "--display-name", "cli-store"])
                .expect("parse init"),
        )
        .expect("run init");
        let workspace = run(Cli::try_parse_from([
            "workvcs",
            "workspace",
            "create",
            store,
            "--display-name",
            "workspace",
        ])
        .expect("parse workspace"))
        .expect("create workspace");
        let workspace_id = value(&workspace, "workspace_id");
        let branch = value(&workspace, "branch_id");

        let prior = run(Cli::try_parse_from([
            "workvcs",
            "knowledge",
            "create",
            store,
            "--branch",
            &branch,
            "--head",
            &value(&workspace, "genesis_commit_id"),
            "--statement",
            "Use the old context summary format",
        ])
        .expect("parse prior knowledge create"))
        .expect("create prior knowledge");
        let replacement = run(Cli::try_parse_from([
            "workvcs",
            "knowledge",
            "create",
            store,
            "--branch",
            &branch,
            "--head",
            &value(&prior, "commit_id"),
            "--statement",
            "Use the scoped context summary format",
        ])
        .expect("parse replacement knowledge create"))
        .expect("create replacement knowledge");
        let superseded = run(Cli::try_parse_from([
            "workvcs",
            "knowledge",
            "supersede",
            store,
            "--branch",
            &branch,
            "--head",
            &value(&replacement, "commit_id"),
            "--knowledge",
            &value(&prior, "knowledge_entity_id"),
            "--knowledge-version",
            &value(&prior, "knowledge_entity_version_id"),
            "--rationale",
            "The context summary format was replaced",
        ])
        .expect("parse knowledge supersede"))
        .expect("supersede knowledge");
        let relation = run(Cli::try_parse_from([
            "workvcs",
            "knowledge",
            "link-supersedes",
            store,
            "--branch",
            &branch,
            "--head",
            &value(&superseded, "commit_id"),
            "--replacement-knowledge",
            &value(&replacement, "knowledge_entity_id"),
            "--prior-knowledge",
            &value(&prior, "knowledge_entity_id"),
            "--rationale",
            "The new Knowledge replaces the prior statement",
        ])
        .expect("parse link supersedes"))
        .expect("link supersedes");

        let session = run(Cli::try_parse_from([
            "workvcs",
            "session",
            "start",
            store,
            "--workspace",
            &workspace_id,
            "--branch",
            &branch,
        ])
        .expect("parse session start"))
        .expect("start session");
        let session_id = value(&session, "session_id");

        let context =
            run(
                Cli::try_parse_from(["workvcs", "context", store, "--session", &session_id])
                    .expect("parse context"),
            )
            .expect("context overview");
        assert!(context.contains("knowledge=1"));
        assert!(context.contains("knowledge_relations=1"));
        assert_eq!(
            value(&context, "context_knowledge_relation.0.relation_id"),
            value(&relation, "relation_id")
        );
        assert!(context.contains("context_knowledge_relation.0.relation_type=supersedes"));
        assert_eq!(
            value(
                &context,
                "context_knowledge_relation.0.replacement_knowledge_entity_id"
            ),
            value(&replacement, "knowledge_entity_id")
        );
        assert_eq!(
            value(
                &context,
                "context_knowledge_relation.0.prior_knowledge_entity_id"
            ),
            value(&prior, "knowledge_entity_id")
        );

        let next = run(
            Cli::try_parse_from(["workvcs", "next", store, "--session", &session_id])
                .expect("parse next"),
        )
        .expect("next work");
        assert!(next.contains("selected=false"));
        assert!(next.contains("context_knowledge=1"));
        assert!(next.contains("context_knowledge_relations=1"));
    }

    #[test]
    fn cli_context_includes_knowledge_exposure_provenance_relations() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let path = tempdir.path().join("workvcs.sqlite");
        let store = path.to_str().expect("path text");

        run(
            Cli::try_parse_from(["workvcs", "init", store, "--display-name", "cli-store"])
                .expect("parse init"),
        )
        .expect("run init");
        let workspace = run(Cli::try_parse_from([
            "workvcs",
            "workspace",
            "create",
            store,
            "--display-name",
            "workspace",
        ])
        .expect("parse workspace"))
        .expect("create workspace");
        let workspace_id = value(&workspace, "workspace_id");
        let branch = value(&workspace, "branch_id");
        let source = run(Cli::try_parse_from([
            "workvcs",
            "knowledge",
            "create",
            store,
            "--branch",
            &branch,
            "--head",
            &value(&workspace, "genesis_commit_id"),
            "--statement",
            "Reusable context exposure knowledge",
        ])
        .expect("parse source knowledge create"))
        .expect("create source knowledge");
        let knowledge_space = run(Cli::try_parse_from([
            "workvcs",
            "store",
            "knowledge-space-create",
            store,
            "--name",
            "Research",
        ])
        .expect("parse knowledge-space-create"))
        .expect("create knowledge space");
        let exposure = run(Cli::try_parse_from([
            "workvcs",
            "store",
            "knowledge-exposure-create-local",
            store,
            "--knowledge-space",
            &value(&knowledge_space, "knowledge_space_id"),
            "--workspace",
            &workspace_id,
            "--knowledge",
            &value(&source, "knowledge_entity_id"),
            "--knowledge-version",
            &value(&source, "knowledge_entity_version_id"),
        ])
        .expect("parse exposure create"))
        .expect("create exposure");
        let adoption = run(Cli::try_parse_from([
            "workvcs",
            "store",
            "knowledge-exposure-adopt",
            store,
            "--branch",
            &branch,
            "--head",
            &value(&source, "commit_id"),
            "--exposure",
            &value(&exposure, "exposure_id"),
            "--rationale",
            "adopt current exposure",
        ])
        .expect("parse adoption"))
        .expect("adopt exposure");
        let session = run(Cli::try_parse_from([
            "workvcs",
            "session",
            "start",
            store,
            "--workspace",
            &workspace_id,
            "--branch",
            &branch,
        ])
        .expect("parse session start"))
        .expect("start session");
        let session_id = value(&session, "session_id");

        let context =
            run(
                Cli::try_parse_from(["workvcs", "context", store, "--session", &session_id])
                    .expect("parse context"),
            )
            .expect("context overview");
        assert_eq!(value(&context, "knowledge"), "2");
        assert_eq!(value(&context, "knowledge_exposure_relations"), "1");
        assert_eq!(
            value(
                &context,
                "context_knowledge_exposure_relation.0.relation_id"
            ),
            value(&adoption, "relation_id")
        );
        assert_eq!(
            value(
                &context,
                "context_knowledge_exposure_relation.0.relation_kind"
            ),
            "knowledge_exposure_derived_from"
        );
        assert_eq!(
            value(&context, "context_knowledge_exposure_relation.0.direction"),
            "outgoing"
        );
        assert_eq!(
            value(
                &context,
                "context_knowledge_exposure_relation.0.source_knowledge_entity_id"
            ),
            value(&adoption, "adopted_knowledge_entity_id")
        );
        assert_eq!(
            value(
                &context,
                "context_knowledge_exposure_relation.0.target_exposure_id"
            ),
            value(&exposure, "exposure_id")
        );

        let next = run(
            Cli::try_parse_from(["workvcs", "next", store, "--session", &session_id])
                .expect("parse next"),
        )
        .expect("next work");
        assert!(next.contains("context_knowledge=2"));
        assert!(next.contains("context_knowledge_exposure_relations=1"));
    }

    #[test]
    fn cli_runs_next_workflow() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let path = tempdir.path().join("workvcs.sqlite");
        let store = path.to_str().expect("path text");

        run(
            Cli::try_parse_from(["workvcs", "init", store, "--display-name", "cli-store"])
                .expect("parse init"),
        )
        .expect("run init");
        let workspace = run(Cli::try_parse_from([
            "workvcs",
            "workspace",
            "create",
            store,
            "--display-name",
            "workspace",
        ])
        .expect("parse workspace"))
        .expect("create workspace");
        let workspace_id = value(&workspace, "workspace_id");
        let branch = value(&workspace, "branch_id");
        let head = value(&workspace, "genesis_commit_id");

        let task = run(Cli::try_parse_from([
            "workvcs",
            "task",
            "create",
            store,
            "--branch",
            &branch,
            "--head",
            &head,
            "--description",
            "Next workflow task",
        ])
        .expect("parse task"))
        .expect("create task");
        let task_id = value(&task, "task_entity_id");

        let session = run(Cli::try_parse_from([
            "workvcs",
            "session",
            "start",
            store,
            "--workspace",
            &workspace_id,
            "--branch",
            &branch,
        ])
        .expect("parse session start"))
        .expect("start session");
        let session_id = value(&session, "session_id");

        let next = run(
            Cli::try_parse_from(["workvcs", "next", store, "--session", &session_id])
                .expect("parse next"),
        )
        .expect("next work");
        let claim_id = value(&next, "claim_id");
        assert!(next.contains("selected=true"));
        assert!(next.contains(&format!("task_entity_id={task_id}")));
        assert!(next.contains(&format!("context_focus_entity_id={task_id}")));
        assert!(next.contains("context_runnable_candidates=1"));
        assert!(next.contains("context_knowledge=0"));

        let runnable = run(Cli::try_parse_from([
            "workvcs",
            "runnable",
            "tasks",
            store,
            "--session",
            &session_id,
        ])
        .expect("parse runnable"))
        .expect("runnable after next");
        assert!(runnable.contains(&format!("candidate.0.claim=claimed_by_session:{claim_id}")));
    }

    #[test]
    fn cli_next_accepts_explicit_shared_mode() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let path = tempdir.path().join("workvcs.sqlite");
        let store = path.to_str().expect("path text");

        run(
            Cli::try_parse_from(["workvcs", "init", store, "--display-name", "cli-store"])
                .expect("parse init"),
        )
        .expect("run init");
        let workspace = run(Cli::try_parse_from([
            "workvcs",
            "workspace",
            "create",
            store,
            "--display-name",
            "workspace",
        ])
        .expect("parse workspace"))
        .expect("create workspace");
        let workspace_id = value(&workspace, "workspace_id");
        let branch = value(&workspace, "branch_id");
        let head = value(&workspace, "genesis_commit_id");
        let task = run(Cli::try_parse_from([
            "workvcs",
            "task",
            "create",
            store,
            "--branch",
            &branch,
            "--head",
            &head,
            "--description",
            "Shared next workflow task",
        ])
        .expect("parse task"))
        .expect("create task");
        let task_id = value(&task, "task_entity_id");

        let first_session = run(Cli::try_parse_from([
            "workvcs",
            "session",
            "start",
            store,
            "--workspace",
            &workspace_id,
            "--branch",
            &branch,
        ])
        .expect("parse first session"))
        .expect("start first session");
        let first_session_id = value(&first_session, "session_id");
        let second_session = run(Cli::try_parse_from([
            "workvcs",
            "session",
            "start",
            store,
            "--workspace",
            &workspace_id,
            "--branch",
            &branch,
        ])
        .expect("parse second session"))
        .expect("start second session");
        let second_session_id = value(&second_session, "session_id");

        run(Cli::try_parse_from([
            "workvcs",
            "claim",
            "task",
            store,
            "--session",
            &first_session_id,
            "--task",
            &task_id,
            "--mode",
            "shared",
        ])
        .expect("parse first shared claim"))
        .expect("first shared claim");

        let next = run(Cli::try_parse_from([
            "workvcs",
            "next",
            store,
            "--session",
            &second_session_id,
            "--mode",
            "shared",
        ])
        .expect("parse shared next"))
        .expect("shared next");
        assert!(next.contains("selected=true"));
        assert_eq!(value(&next, "task_entity_id"), task_id);
        assert_eq!(value(&next, "mode"), "shared");
        assert_eq!(value(&next, "context_focus_entity_id"), task_id);
    }

    #[test]
    fn cli_runs_record_finding_workflow() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let path = tempdir.path().join("workvcs.sqlite");
        let store = path.to_str().expect("path text");

        run(
            Cli::try_parse_from(["workvcs", "init", store, "--display-name", "cli-store"])
                .expect("parse init"),
        )
        .expect("run init");
        let workspace = run(Cli::try_parse_from([
            "workvcs",
            "workspace",
            "create",
            store,
            "--display-name",
            "workspace",
        ])
        .expect("parse workspace"))
        .expect("create workspace");
        let branch = value(&workspace, "branch_id");
        let head = value(&workspace, "genesis_commit_id");

        let record = run(Cli::try_parse_from([
            "workvcs",
            "record",
            "finding",
            store,
            "--branch",
            &branch,
            "--head",
            &head,
            "--statement",
            "Schema validation has no drift",
            "--scope-json",
            r#"{"subject":"schema"}"#,
        ])
        .expect("parse record finding"))
        .expect("create finding record");

        assert!(record.contains("record_kind=finding"));
        assert!(record.contains("record_status=active"));
        assert!(record.contains("record_entity_id="));
        assert!(record.contains("commit_id="));
    }

    #[test]
    fn cli_runs_record_assumption_workflow() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let path = tempdir.path().join("workvcs.sqlite");
        let store = path.to_str().expect("path text");

        run(
            Cli::try_parse_from(["workvcs", "init", store, "--display-name", "cli-store"])
                .expect("parse init"),
        )
        .expect("run init");
        let workspace = run(Cli::try_parse_from([
            "workvcs",
            "workspace",
            "create",
            store,
            "--display-name",
            "workspace",
        ])
        .expect("parse workspace"))
        .expect("create workspace");
        let branch = value(&workspace, "branch_id");
        let head = value(&workspace, "genesis_commit_id");

        let record = run(Cli::try_parse_from([
            "workvcs",
            "record",
            "assumption",
            store,
            "--branch",
            &branch,
            "--head",
            &head,
            "--statement",
            "Serialized writes are sufficient",
        ])
        .expect("parse record assumption"))
        .expect("create assumption record");

        assert!(record.contains("record_kind=assumption"));
        assert!(record.contains("record_status=unverified"));
        assert!(record.contains("record_entity_id="));

        let transition = run(Cli::try_parse_from([
            "workvcs",
            "record",
            "assumption-status",
            store,
            "--branch",
            &branch,
            "--head",
            &value(&record, "commit_id"),
            "--record",
            &value(&record, "record_entity_id"),
            "--record-version",
            &value(&record, "record_entity_version_id"),
            "--status",
            "validated",
            "--rationale",
            "Confirmed by local validation",
        ])
        .expect("parse assumption status"))
        .expect("transition assumption");
        assert!(transition.contains("previous_record_status=unverified"));
        assert!(transition.contains("record_status=validated"));
    }

    #[test]
    fn cli_runs_record_decision_workflow() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let path = tempdir.path().join("workvcs.sqlite");
        let store = path.to_str().expect("path text");

        run(
            Cli::try_parse_from(["workvcs", "init", store, "--display-name", "cli-store"])
                .expect("parse init"),
        )
        .expect("run init");
        let workspace = run(Cli::try_parse_from([
            "workvcs",
            "workspace",
            "create",
            store,
            "--display-name",
            "workspace",
        ])
        .expect("parse workspace"))
        .expect("create workspace");
        let branch = value(&workspace, "branch_id");
        let head = value(&workspace, "genesis_commit_id");

        let record = run(Cli::try_parse_from([
            "workvcs",
            "record",
            "decision",
            store,
            "--branch",
            &branch,
            "--head",
            &head,
            "--statement",
            "Use SQLite for V0.1 storage",
            "--scope-json",
            r#"{"decision_scope":"storage"}"#,
        ])
        .expect("parse record decision"))
        .expect("create decision record");

        assert!(record.contains("record_kind=decision"));
        assert!(record.contains("record_status=active"));
        assert!(record.contains("record_entity_id="));
    }

    #[test]
    fn cli_runs_record_decision_lifecycle_workflow() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let path = tempdir.path().join("workvcs.sqlite");
        let store = path.to_str().expect("path text");

        run(
            Cli::try_parse_from(["workvcs", "init", store, "--display-name", "cli-store"])
                .expect("parse init"),
        )
        .expect("run init");
        let workspace = run(Cli::try_parse_from([
            "workvcs",
            "workspace",
            "create",
            store,
            "--display-name",
            "workspace",
        ])
        .expect("parse workspace"))
        .expect("create workspace");
        let branch = value(&workspace, "branch_id");

        let decision = run(Cli::try_parse_from([
            "workvcs",
            "record",
            "decision",
            store,
            "--branch",
            &branch,
            "--head",
            &value(&workspace, "genesis_commit_id"),
            "--statement",
            "Use serialized writes",
        ])
        .expect("parse decision"))
        .expect("create decision");
        let superseded = run(Cli::try_parse_from([
            "workvcs",
            "record",
            "decision-status",
            store,
            "--branch",
            &branch,
            "--head",
            &value(&decision, "commit_id"),
            "--record",
            &value(&decision, "record_entity_id"),
            "--record-version",
            &value(&decision, "record_entity_version_id"),
            "--status",
            "superseded",
            "--rationale",
            "A newer decision replaces this one",
        ])
        .expect("parse decision status"))
        .expect("transition decision");

        assert!(superseded.contains("record_kind=decision"));
        assert!(superseded.contains("previous_record_status=active"));
        assert!(superseded.contains("record_status=superseded"));

        let shown = run(Cli::try_parse_from([
            "workvcs",
            "record",
            "show",
            store,
            "--commit",
            &value(&superseded, "commit_id"),
            "--record",
            &value(&superseded, "record_entity_id"),
        ])
        .expect("parse record show"))
        .expect("show decision");
        assert!(shown.contains("record_status=superseded"));
    }

    #[test]
    fn cli_supersedes_decision_record_atomically() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let path = tempdir.path().join("workvcs.sqlite");
        let store = path.to_str().expect("path text");

        run(
            Cli::try_parse_from(["workvcs", "init", store, "--display-name", "cli-store"])
                .expect("parse init"),
        )
        .expect("run init");
        let workspace = run(Cli::try_parse_from([
            "workvcs",
            "workspace",
            "create",
            store,
            "--display-name",
            "workspace",
        ])
        .expect("parse workspace"))
        .expect("create workspace");
        let branch = value(&workspace, "branch_id");

        let prior = run(Cli::try_parse_from([
            "workvcs",
            "record",
            "decision",
            store,
            "--branch",
            &branch,
            "--head",
            &value(&workspace, "genesis_commit_id"),
            "--statement",
            "Use optimistic writes",
        ])
        .expect("parse prior decision"))
        .expect("create prior decision");
        let finding = run(Cli::try_parse_from([
            "workvcs",
            "record",
            "finding",
            store,
            "--branch",
            &branch,
            "--head",
            &value(&prior, "commit_id"),
            "--statement",
            "Concurrent write tests fail without serialization",
        ])
        .expect("parse finding"))
        .expect("create finding");
        let replacement = run(Cli::try_parse_from([
            "workvcs",
            "record",
            "decision",
            store,
            "--branch",
            &branch,
            "--head",
            &value(&finding, "commit_id"),
            "--statement",
            "Use serialized writes",
        ])
        .expect("parse replacement decision"))
        .expect("create replacement decision");
        let superseded = run(Cli::try_parse_from([
            "workvcs",
            "record",
            "supersede-decision",
            store,
            "--branch",
            &branch,
            "--head",
            &value(&replacement, "commit_id"),
            "--replacement-record",
            &value(&replacement, "record_entity_id"),
            "--prior-record",
            &value(&prior, "record_entity_id"),
            "--prior-record-version",
            &value(&prior, "record_entity_version_id"),
            "--because-record",
            &value(&finding, "record_entity_id"),
            "--rationale",
            "Serialized writes supersede optimistic writes",
        ])
        .expect("parse supersede decision"))
        .expect("supersede decision");

        assert!(superseded.contains("prior_record_status=superseded"));
        assert!(superseded.contains("relation_type=supersedes"));
        assert_eq!(
            value(&superseded, "causal_record_entity_id"),
            value(&finding, "record_entity_id")
        );
        assert_eq!(value(&superseded, "causal_relation_type"), "derived_from");
        assert!(!value(&superseded, "causal_relation_id").is_empty());

        let shown_changeset = run(Cli::try_parse_from([
            "workvcs",
            "changeset",
            "show",
            store,
            "--changeset",
            &value(&superseded, "changeset_id"),
        ])
        .expect("parse changeset show"))
        .expect("show changeset");
        assert_eq!(value(&shown_changeset, "causal_anchors"), "1");

        let anchors = run(Cli::try_parse_from([
            "workvcs",
            "changeset",
            "anchors",
            store,
            "--changeset",
            &value(&superseded, "changeset_id"),
        ])
        .expect("parse changeset anchors"))
        .expect("list changeset anchors");
        assert_eq!(value(&anchors, "causal_anchors"), "1");
        assert_eq!(value(&anchors, "anchor[0].ordinal"), "0");
        assert_eq!(
            value(&anchors, "anchor[0].object_id"),
            value(&finding, "record_entity_id")
        );
        assert_eq!(value(&anchors, "anchor[0].object_kind"), "entity");

        let doctor = run(Cli::try_parse_from(["workvcs", "doctor", store]).expect("parse doctor"))
            .expect("run doctor");
        assert!(doctor.contains("checked_changeset_causal_anchors=1"));

        let shown = run(Cli::try_parse_from([
            "workvcs",
            "record",
            "show",
            store,
            "--commit",
            &value(&superseded, "commit_id"),
            "--record",
            &value(&prior, "record_entity_id"),
        ])
        .expect("parse prior show"))
        .expect("show prior");
        assert!(shown.contains("record_status=superseded"));

        let superseded_records = run(Cli::try_parse_from([
            "workvcs",
            "record",
            "list",
            store,
            "--commit",
            &value(&superseded, "commit_id"),
            "--status",
            "superseded",
        ])
        .expect("parse superseded record list"))
        .expect("list superseded records");
        assert!(superseded_records.contains("records=1"));
        assert_eq!(
            value(&superseded_records, "record.0.record_entity_id"),
            value(&prior, "record_entity_id")
        );

        let active_decisions = run(Cli::try_parse_from([
            "workvcs",
            "record",
            "list",
            store,
            "--commit",
            &value(&superseded, "commit_id"),
            "--kind",
            "decision",
            "--status",
            "active",
        ])
        .expect("parse active decision list"))
        .expect("list active decisions");
        assert!(active_decisions.contains("records=1"));
        assert_eq!(
            value(&active_decisions, "record.0.record_entity_id"),
            value(&replacement, "record_entity_id")
        );

        let listed = run(Cli::try_parse_from([
            "workvcs",
            "record",
            "relation-list",
            store,
            "--commit",
            &value(&superseded, "commit_id"),
            "--type",
            "supersedes",
        ])
        .expect("parse relation list"))
        .expect("list supersedes");
        assert!(listed.contains("relations=1"));
        assert_eq!(
            value(&listed, "relation.0.relation_id"),
            value(&superseded, "relation_id")
        );

        let derived = run(Cli::try_parse_from([
            "workvcs",
            "record",
            "relation-list",
            store,
            "--commit",
            &value(&superseded, "commit_id"),
            "--type",
            "derived_from",
        ])
        .expect("parse derived relation list"))
        .expect("list derived_from");
        assert!(derived.contains("relations=1"));
        assert_eq!(
            value(&derived, "relation.0.relation_id"),
            value(&superseded, "causal_relation_id")
        );

        let why_prior = run(Cli::try_parse_from([
            "workvcs",
            "why",
            store,
            "--commit",
            &value(&superseded, "commit_id"),
            "--entity",
            &value(&prior, "record_entity_id"),
        ])
        .expect("parse why prior"))
        .expect("why prior");
        assert!(why_prior.contains("relation.0.relation_kind=record_supersedes"));
        assert!(why_prior.contains("relation.0.direction=incoming"));
    }

    #[test]
    fn cli_links_record_derived_from_source_record() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let path = tempdir.path().join("workvcs.sqlite");
        let store = path.to_str().expect("path text");

        run(
            Cli::try_parse_from(["workvcs", "init", store, "--display-name", "cli-store"])
                .expect("parse init"),
        )
        .expect("run init");
        let workspace = run(Cli::try_parse_from([
            "workvcs",
            "workspace",
            "create",
            store,
            "--display-name",
            "workspace",
        ])
        .expect("parse workspace"))
        .expect("create workspace");
        let branch = value(&workspace, "branch_id");

        let finding = run(Cli::try_parse_from([
            "workvcs",
            "record",
            "finding",
            store,
            "--branch",
            &branch,
            "--head",
            &value(&workspace, "genesis_commit_id"),
            "--statement",
            "Concurrent write tests fail without serialization",
        ])
        .expect("parse finding"))
        .expect("create finding");
        let decision = run(Cli::try_parse_from([
            "workvcs",
            "record",
            "decision",
            store,
            "--branch",
            &branch,
            "--head",
            &value(&finding, "commit_id"),
            "--statement",
            "Use serialized writes",
        ])
        .expect("parse decision"))
        .expect("create decision");
        let relation = run(Cli::try_parse_from([
            "workvcs",
            "record",
            "link-derived-from",
            store,
            "--branch",
            &branch,
            "--head",
            &value(&decision, "commit_id"),
            "--result-record",
            &value(&decision, "record_entity_id"),
            "--source-record",
            &value(&finding, "record_entity_id"),
            "--rationale",
            "Decision was derived from the finding",
        ])
        .expect("parse link derived_from"))
        .expect("link derived_from");

        assert!(relation.contains("relation_type=derived_from"));
        assert_eq!(
            value(&relation, "source_record_entity_id"),
            value(&decision, "record_entity_id")
        );
        assert_eq!(
            value(&relation, "target_record_entity_id"),
            value(&finding, "record_entity_id")
        );

        let listed = run(Cli::try_parse_from([
            "workvcs",
            "record",
            "relation-list",
            store,
            "--commit",
            &value(&relation, "commit_id"),
            "--type",
            "derived_from",
        ])
        .expect("parse relation list"))
        .expect("list derived_from");
        assert!(listed.contains("relations=1"));

        let why_decision = run(Cli::try_parse_from([
            "workvcs",
            "why",
            store,
            "--commit",
            &value(&relation, "commit_id"),
            "--entity",
            &value(&decision, "record_entity_id"),
        ])
        .expect("parse why decision"))
        .expect("why decision");
        assert!(why_decision.contains("relation.0.relation_kind=record_derived_from"));
        assert!(why_decision.contains("relation.0.direction=outgoing"));
    }

    #[test]
    fn cli_runs_question_and_risk_record_workflows() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let path = tempdir.path().join("workvcs.sqlite");
        let store = path.to_str().expect("path text");

        run(
            Cli::try_parse_from(["workvcs", "init", store, "--display-name", "cli-store"])
                .expect("parse init"),
        )
        .expect("run init");
        let workspace = run(Cli::try_parse_from([
            "workvcs",
            "workspace",
            "create",
            store,
            "--display-name",
            "workspace",
        ])
        .expect("parse workspace"))
        .expect("create workspace");
        let branch = value(&workspace, "branch_id");
        let head = value(&workspace, "genesis_commit_id");

        let question = run(Cli::try_parse_from([
            "workvcs",
            "record",
            "question",
            store,
            "--branch",
            &branch,
            "--head",
            &head,
            "--statement",
            "Which replay invariant should be exposed next?",
        ])
        .expect("parse record question"))
        .expect("create question record");
        assert!(question.contains("record_kind=question"));
        assert!(question.contains("record_status=active"));

        let risk = run(Cli::try_parse_from([
            "workvcs",
            "record",
            "risk",
            store,
            "--branch",
            &branch,
            "--head",
            &value(&question, "commit_id"),
            "--statement",
            "Manual ordering remains unresolved",
        ])
        .expect("parse record risk"))
        .expect("create risk record");
        assert!(risk.contains("record_kind=risk"));
        assert!(risk.contains("record_status=active"));
    }

    #[test]
    fn cli_lists_record_workflow() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let path = tempdir.path().join("workvcs.sqlite");
        let store = path.to_str().expect("path text");

        run(
            Cli::try_parse_from(["workvcs", "init", store, "--display-name", "cli-store"])
                .expect("parse init"),
        )
        .expect("run init");
        let workspace = run(Cli::try_parse_from([
            "workvcs",
            "workspace",
            "create",
            store,
            "--display-name",
            "workspace",
        ])
        .expect("parse workspace"))
        .expect("create workspace");
        let branch = value(&workspace, "branch_id");
        let head = value(&workspace, "genesis_commit_id");

        let finding = run(Cli::try_parse_from([
            "workvcs",
            "record",
            "finding",
            store,
            "--branch",
            &branch,
            "--head",
            &head,
            "--statement",
            "Schema validation has no drift",
            "--scope-json",
            "{\"local_ref\":\"root\",\"kind\":\"workspace\"}",
        ])
        .expect("parse finding"))
        .expect("create finding");
        let assumption = run(Cli::try_parse_from([
            "workvcs",
            "record",
            "assumption",
            store,
            "--branch",
            &branch,
            "--head",
            &value(&finding, "commit_id"),
            "--statement",
            "Serialized writes are sufficient",
            "--scope-json",
            "{\"kind\":\"module\",\"local_ref\":\"core\"}",
        ])
        .expect("parse assumption"))
        .expect("create assumption");

        let list = run(Cli::try_parse_from([
            "workvcs",
            "record",
            "list",
            store,
            "--commit",
            &value(&assumption, "commit_id"),
        ])
        .expect("parse record list"))
        .expect("list records");
        assert_eq!(value(&list, "records"), "2");
        assert!(list.contains("record_kind=finding"));
        assert!(list.contains("record_kind=assumption"));
        assert!(list.contains("record_status=active"));
        assert!(list.contains("record_status=unverified"));

        let branch_list =
            run(
                Cli::try_parse_from(["workvcs", "record", "list", store, "--branch", &branch])
                    .expect("parse branch record list"),
            )
            .expect("list records at branch head");
        assert_eq!(value(&branch_list, "records"), "2");
        assert!(branch_list.contains("record_kind=finding"));
        assert!(branch_list.contains("record_kind=assumption"));

        let filtered = run(Cli::try_parse_from([
            "workvcs",
            "record",
            "list",
            store,
            "--commit",
            &value(&assumption, "commit_id"),
            "--kind",
            "assumption",
            "--scope-json",
            "{\"local_ref\":\"core\",\"kind\":\"module\"}",
        ])
        .expect("parse filtered record list"))
        .expect("list filtered records");
        assert_eq!(value(&filtered, "records"), "1");
        assert!(filtered.contains("record_kind=assumption"));
        assert!(!filtered.contains("record_kind=finding"));

        let scope_filtered = run(Cli::try_parse_from([
            "workvcs",
            "record",
            "list",
            store,
            "--branch",
            &branch,
            "--scope-json",
            "{\"kind\":\"workspace\",\"local_ref\":\"root\"}",
        ])
        .expect("parse scope-filtered record list"))
        .expect("list scope-filtered records");
        assert_eq!(value(&scope_filtered, "records"), "1");
        assert!(scope_filtered.contains("record_kind=finding"));
        assert!(!scope_filtered.contains("record_kind=assumption"));

        let statement_filtered = run(Cli::try_parse_from([
            "workvcs",
            "record",
            "list",
            store,
            "--branch",
            &branch,
            "--statement-contains",
            "validation",
        ])
        .expect("parse statement-filtered record list"))
        .expect("list statement-filtered records");
        assert_eq!(value(&statement_filtered, "records"), "1");
        assert!(statement_filtered.contains("record_kind=finding"));
        assert!(!statement_filtered.contains("record_kind=assumption"));
    }

    #[test]
    fn cli_shows_record_workflow() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let path = tempdir.path().join("workvcs.sqlite");
        let store = path.to_str().expect("path text");

        run(
            Cli::try_parse_from(["workvcs", "init", store, "--display-name", "cli-store"])
                .expect("parse init"),
        )
        .expect("run init");
        let workspace = run(Cli::try_parse_from([
            "workvcs",
            "workspace",
            "create",
            store,
            "--display-name",
            "workspace",
        ])
        .expect("parse workspace"))
        .expect("create workspace");
        let branch = value(&workspace, "branch_id");
        let head = value(&workspace, "genesis_commit_id");

        let record = run(Cli::try_parse_from([
            "workvcs",
            "record",
            "finding",
            store,
            "--branch",
            &branch,
            "--head",
            &head,
            "--statement",
            "Line one\nLine two",
            "--scope-json",
            r#"{"b":2,"a":1}"#,
        ])
        .expect("parse finding"))
        .expect("create finding");
        let show = run(Cli::try_parse_from([
            "workvcs",
            "record",
            "show",
            store,
            "--commit",
            &value(&record, "commit_id"),
            "--record",
            &value(&record, "record_entity_id"),
        ])
        .expect("parse record show"))
        .expect("show record");

        assert!(show.contains("record_kind=finding"));
        assert!(show.contains("record_status=active"));
        assert!(show.contains("record_statement_json=\"Line one\\nLine two\""));
        assert!(show.contains("record_scope_json={\"a\":1,\"b\":2}"));
        assert_eq!(
            value(&show, "record_entity_version_id"),
            value(&record, "record_entity_version_id")
        );

        let branch_show = run(Cli::try_parse_from([
            "workvcs",
            "record",
            "show",
            store,
            "--branch",
            &branch,
            "--record",
            &value(&record, "record_entity_id"),
        ])
        .expect("parse branch record show"))
        .expect("show record at branch head");
        assert_eq!(
            value(&branch_show, "record_entity_version_id"),
            value(&record, "record_entity_version_id")
        );
    }

    #[test]
    fn cli_runs_attempt_record_workflow() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let path = tempdir.path().join("workvcs.sqlite");
        let store = path.to_str().expect("path text");

        run(
            Cli::try_parse_from(["workvcs", "init", store, "--display-name", "cli-store"])
                .expect("parse init"),
        )
        .expect("run init");
        let workspace = run(Cli::try_parse_from([
            "workvcs",
            "workspace",
            "create",
            store,
            "--display-name",
            "workspace",
        ])
        .expect("parse workspace"))
        .expect("create workspace");
        let branch = value(&workspace, "branch_id");
        let head = value(&workspace, "genesis_commit_id");

        let record = run(Cli::try_parse_from([
            "workvcs",
            "record",
            "attempt",
            store,
            "--branch",
            &branch,
            "--head",
            &head,
            "--statement",
            "Run the next validation command",
        ])
        .expect("parse attempt"))
        .expect("create attempt");
        assert!(record.contains("record_kind=attempt"));
        assert!(record.contains("record_status=running"));

        let list = run(Cli::try_parse_from([
            "workvcs",
            "record",
            "list",
            store,
            "--commit",
            &value(&record, "commit_id"),
            "--kind",
            "attempt",
        ])
        .expect("parse attempt list"))
        .expect("list attempt records");
        assert_eq!(value(&list, "records"), "1");
        assert!(list.contains("record_kind=attempt"));
        assert!(list.contains("record_status=running"));

        let show = run(Cli::try_parse_from([
            "workvcs",
            "record",
            "show",
            store,
            "--commit",
            &value(&record, "commit_id"),
            "--record",
            &value(&record, "record_entity_id"),
        ])
        .expect("parse attempt show"))
        .expect("show attempt record");
        assert!(show.contains("record_kind=attempt"));
        assert!(show.contains("record_status=running"));
        assert!(show.contains("record_statement_json=\"Run the next validation command\""));

        let terminal = run(Cli::try_parse_from([
            "workvcs",
            "record",
            "attempt-status",
            store,
            "--branch",
            &branch,
            "--head",
            &value(&record, "commit_id"),
            "--record",
            &value(&record, "record_entity_id"),
            "--record-version",
            &value(&record, "record_entity_version_id"),
            "--status",
            "failed",
            "--rationale",
            "Validation command failed",
        ])
        .expect("parse attempt status"))
        .expect("transition attempt");
        assert!(terminal.contains("previous_record_status=running"));
        assert!(terminal.contains("record_status=failed"));

        let terminal_show = run(Cli::try_parse_from([
            "workvcs",
            "record",
            "show",
            store,
            "--commit",
            &value(&terminal, "commit_id"),
            "--record",
            &value(&terminal, "record_entity_id"),
        ])
        .expect("parse terminal attempt show"))
        .expect("show terminal attempt record");
        assert!(terminal_show.contains("record_kind=attempt"));
        assert!(terminal_show.contains("record_status=failed"));
    }

    #[test]
    fn cli_runs_handoff_record_workflow() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let path = tempdir.path().join("workvcs.sqlite");
        let store = path.to_str().expect("path text");

        run(
            Cli::try_parse_from(["workvcs", "init", store, "--display-name", "cli-store"])
                .expect("parse init"),
        )
        .expect("run init");
        let workspace = run(Cli::try_parse_from([
            "workvcs",
            "workspace",
            "create",
            store,
            "--display-name",
            "workspace",
        ])
        .expect("parse workspace"))
        .expect("create workspace");
        let branch = value(&workspace, "branch_id");
        let head = value(&workspace, "genesis_commit_id");

        let record = run(Cli::try_parse_from([
            "workvcs",
            "record",
            "handoff",
            store,
            "--branch",
            &branch,
            "--head",
            &head,
            "--statement",
            "Continue with the next runnable task",
            "--scope-json",
            r#"{"focus":"task:next"}"#,
        ])
        .expect("parse handoff"))
        .expect("create handoff");
        assert!(record.contains("record_kind=handoff"));
        assert!(record.contains("record_status=active"));

        let list = run(Cli::try_parse_from([
            "workvcs",
            "record",
            "list",
            store,
            "--commit",
            &value(&record, "commit_id"),
            "--kind",
            "handoff",
        ])
        .expect("parse handoff list"))
        .expect("list handoff records");
        assert_eq!(value(&list, "records"), "1");
        assert!(list.contains("record_kind=handoff"));

        let show = run(Cli::try_parse_from([
            "workvcs",
            "record",
            "show",
            store,
            "--commit",
            &value(&record, "commit_id"),
            "--record",
            &value(&record, "record_entity_id"),
        ])
        .expect("parse handoff show"))
        .expect("show handoff");
        assert!(show.contains("record_kind=handoff"));
        assert!(show.contains("record_scope_json={\"focus\":\"task:next\"}"));
    }

    #[test]
    fn cli_links_finding_to_invalidated_assumption() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let path = tempdir.path().join("workvcs.sqlite");
        let store = path.to_str().expect("path text");

        run(
            Cli::try_parse_from(["workvcs", "init", store, "--display-name", "cli-store"])
                .expect("parse init"),
        )
        .expect("run init");
        let workspace = run(Cli::try_parse_from([
            "workvcs",
            "workspace",
            "create",
            store,
            "--display-name",
            "workspace",
        ])
        .expect("parse workspace"))
        .expect("create workspace");
        let branch = value(&workspace, "branch_id");
        let head = value(&workspace, "genesis_commit_id");

        let assumption = run(Cli::try_parse_from([
            "workvcs",
            "record",
            "assumption",
            store,
            "--branch",
            &branch,
            "--head",
            &head,
            "--statement",
            "Serialized writes are sufficient",
        ])
        .expect("parse assumption"))
        .expect("create assumption");
        let finding = run(Cli::try_parse_from([
            "workvcs",
            "record",
            "finding",
            store,
            "--branch",
            &branch,
            "--head",
            &value(&assumption, "commit_id"),
            "--statement",
            "Concurrent writer test failed",
        ])
        .expect("parse finding"))
        .expect("create finding");
        let invalidated = run(Cli::try_parse_from([
            "workvcs",
            "record",
            "assumption-status",
            store,
            "--branch",
            &branch,
            "--head",
            &value(&finding, "commit_id"),
            "--record",
            &value(&assumption, "record_entity_id"),
            "--record-version",
            &value(&assumption, "record_entity_version_id"),
            "--status",
            "invalidated",
            "--rationale",
            "Concurrent writer test failed",
        ])
        .expect("parse assumption status"))
        .expect("invalidate assumption");

        let relation = run(Cli::try_parse_from([
            "workvcs",
            "record",
            "link-invalidates",
            store,
            "--branch",
            &branch,
            "--head",
            &value(&invalidated, "commit_id"),
            "--source-record",
            &value(&finding, "record_entity_id"),
            "--target-record",
            &value(&assumption, "record_entity_id"),
            "--rationale",
            "Finding invalidates the assumption",
        ])
        .expect("parse link invalidates"))
        .expect("link invalidates");
        assert!(relation.contains("relation_type=invalidates"));
        assert_eq!(
            value(&relation, "source_record_entity_id"),
            value(&finding, "record_entity_id")
        );
        assert_eq!(
            value(&relation, "target_record_entity_id"),
            value(&assumption, "record_entity_id")
        );

        let state = run(Cli::try_parse_from([
            "workvcs",
            "show-at",
            store,
            "--commit",
            &value(&relation, "commit_id"),
        ])
        .expect("parse show-at"))
        .expect("show relation commit");
        assert!(state.contains("relations=1"));
        assert!(state.contains(&format!("relation={}", value(&relation, "relation_id"))));

        let listed = run(Cli::try_parse_from([
            "workvcs",
            "record",
            "relation-list",
            store,
            "--commit",
            &value(&relation, "commit_id"),
            "--type",
            "invalidates",
            "--source-record",
            &value(&finding, "record_entity_id"),
            "--target-record",
            &value(&assumption, "record_entity_id"),
        ])
        .expect("parse relation list"))
        .expect("list record relations");
        assert!(listed.contains("relations=1"));
        assert_eq!(
            value(&listed, "relation.0.relation_id"),
            value(&relation, "relation_id")
        );
        assert!(listed.contains("relation.0.relation_type=invalidates"));
        assert_eq!(
            value(&listed, "relation.0.source_record_entity_id"),
            value(&finding, "record_entity_id")
        );
        assert_eq!(
            value(&listed, "relation.0.target_record_entity_id"),
            value(&assumption, "record_entity_id")
        );

        let why_finding = run(Cli::try_parse_from([
            "workvcs",
            "why",
            store,
            "--commit",
            &value(&relation, "commit_id"),
            "--entity",
            &value(&finding, "record_entity_id"),
        ])
        .expect("parse why finding"))
        .expect("why finding");
        assert!(why_finding.contains("target_kind=commit"));
        assert!(why_finding.contains("subject_kind=entity"));
        assert!(why_finding.contains("relation_edges=1"));
        assert!(why_finding.contains("relation.0.relation_kind=record_invalidates"));
        assert!(why_finding.contains("relation.0.direction=outgoing"));
        assert!(why_finding.contains("relation.0.source_entity_kind=record"));
        assert!(why_finding.contains("relation.0.target_entity_kind=record"));
        assert_eq!(
            value(&why_finding, "relation.0.relation_id"),
            value(&relation, "relation_id")
        );

        let why_assumption = run(Cli::try_parse_from([
            "workvcs",
            "why",
            store,
            "--branch",
            &branch,
            "--entity",
            &value(&assumption, "record_entity_id"),
        ])
        .expect("parse why assumption"))
        .expect("why assumption");
        assert!(why_assumption.contains("target_kind=branch_head"));
        assert!(why_assumption.contains("relation.0.relation_kind=record_invalidates"));
        assert!(why_assumption.contains("relation.0.direction=incoming"));
        assert_eq!(
            value(&why_assumption, "relation.0.relation_id"),
            value(&relation, "relation_id")
        );
    }

    #[test]
    fn cli_links_finding_to_validated_assumption() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let path = tempdir.path().join("workvcs.sqlite");
        let store = path.to_str().expect("path text");

        run(
            Cli::try_parse_from(["workvcs", "init", store, "--display-name", "cli-store"])
                .expect("parse init"),
        )
        .expect("run init");
        let workspace = run(Cli::try_parse_from([
            "workvcs",
            "workspace",
            "create",
            store,
            "--display-name",
            "workspace",
        ])
        .expect("parse workspace"))
        .expect("create workspace");
        let branch = value(&workspace, "branch_id");
        let head = value(&workspace, "genesis_commit_id");

        let assumption = run(Cli::try_parse_from([
            "workvcs",
            "record",
            "assumption",
            store,
            "--branch",
            &branch,
            "--head",
            &head,
            "--statement",
            "Serialized writes are sufficient",
        ])
        .expect("parse assumption"))
        .expect("create assumption");
        let finding = run(Cli::try_parse_from([
            "workvcs",
            "record",
            "finding",
            store,
            "--branch",
            &branch,
            "--head",
            &value(&assumption, "commit_id"),
            "--statement",
            "Concurrent writer test passed",
        ])
        .expect("parse finding"))
        .expect("create finding");
        let validated = run(Cli::try_parse_from([
            "workvcs",
            "record",
            "assumption-status",
            store,
            "--branch",
            &branch,
            "--head",
            &value(&finding, "commit_id"),
            "--record",
            &value(&assumption, "record_entity_id"),
            "--record-version",
            &value(&assumption, "record_entity_version_id"),
            "--status",
            "validated",
            "--rationale",
            "Concurrent writer test passed",
        ])
        .expect("parse assumption status"))
        .expect("validate assumption");

        let relation = run(Cli::try_parse_from([
            "workvcs",
            "record",
            "link-validates",
            store,
            "--branch",
            &branch,
            "--head",
            &value(&validated, "commit_id"),
            "--source-record",
            &value(&finding, "record_entity_id"),
            "--target-record",
            &value(&assumption, "record_entity_id"),
            "--rationale",
            "Finding validates the assumption",
        ])
        .expect("parse link validates"))
        .expect("link validates");
        assert!(relation.contains("relation_type=validates"));

        let listed = run(Cli::try_parse_from([
            "workvcs",
            "record",
            "relation-list",
            store,
            "--commit",
            &value(&relation, "commit_id"),
            "--type",
            "validates",
        ])
        .expect("parse relation list"))
        .expect("list record relations");
        assert!(listed.contains("relations=1"));
        assert_eq!(
            value(&listed, "relation.0.relation_id"),
            value(&relation, "relation_id")
        );
        assert!(listed.contains("relation.0.relation_type=validates"));

        let why_assumption = run(Cli::try_parse_from([
            "workvcs",
            "why",
            store,
            "--commit",
            &value(&relation, "commit_id"),
            "--entity",
            &value(&assumption, "record_entity_id"),
        ])
        .expect("parse why assumption"))
        .expect("why assumption");
        assert!(why_assumption.contains("relation_edges=1"));
        assert!(why_assumption.contains("relation.0.relation_kind=record_validates"));
        assert!(why_assumption.contains("relation.0.direction=incoming"));
    }

    #[test]
    fn cli_links_finding_to_supported_decision() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let path = tempdir.path().join("workvcs.sqlite");
        let store = path.to_str().expect("path text");

        run(
            Cli::try_parse_from(["workvcs", "init", store, "--display-name", "cli-store"])
                .expect("parse init"),
        )
        .expect("run init");
        let workspace = run(Cli::try_parse_from([
            "workvcs",
            "workspace",
            "create",
            store,
            "--display-name",
            "workspace",
        ])
        .expect("parse workspace"))
        .expect("create workspace");
        let branch = value(&workspace, "branch_id");
        let head = value(&workspace, "genesis_commit_id");

        let decision = run(Cli::try_parse_from([
            "workvcs",
            "record",
            "decision",
            store,
            "--branch",
            &branch,
            "--head",
            &head,
            "--statement",
            "Use serialized writes",
        ])
        .expect("parse decision"))
        .expect("create decision");
        let finding = run(Cli::try_parse_from([
            "workvcs",
            "record",
            "finding",
            store,
            "--branch",
            &branch,
            "--head",
            &value(&decision, "commit_id"),
            "--statement",
            "Concurrent write tests are flaky without serialization",
        ])
        .expect("parse finding"))
        .expect("create finding");

        let relation = run(Cli::try_parse_from([
            "workvcs",
            "record",
            "link-supports",
            store,
            "--branch",
            &branch,
            "--head",
            &value(&finding, "commit_id"),
            "--source-record",
            &value(&finding, "record_entity_id"),
            "--target-record",
            &value(&decision, "record_entity_id"),
            "--rationale",
            "Finding supports the decision",
        ])
        .expect("parse link supports"))
        .expect("link supports");
        assert!(relation.contains("relation_type=supports"));

        let listed = run(Cli::try_parse_from([
            "workvcs",
            "record",
            "relation-list",
            store,
            "--commit",
            &value(&relation, "commit_id"),
            "--type",
            "supports",
        ])
        .expect("parse relation list"))
        .expect("list record relations");
        assert!(listed.contains("relations=1"));
        assert_eq!(
            value(&listed, "relation.0.relation_id"),
            value(&relation, "relation_id")
        );
        assert!(listed.contains("relation.0.relation_type=supports"));

        let branch_listed = run(Cli::try_parse_from([
            "workvcs",
            "record",
            "relation-list",
            store,
            "--branch",
            &branch,
            "--type",
            "supports",
        ])
        .expect("parse branch relation list"))
        .expect("list record relations at branch head");
        assert!(branch_listed.contains("relations=1"));
        assert_eq!(
            value(&branch_listed, "relation.0.relation_id"),
            value(&relation, "relation_id")
        );

        let shown = run(Cli::try_parse_from([
            "workvcs",
            "record",
            "relation-show",
            store,
            "--commit",
            &value(&relation, "commit_id"),
            "--relation",
            &value(&relation, "relation_id"),
        ])
        .expect("parse relation show"))
        .expect("show record relation");
        assert_eq!(
            value(&shown, "relation_id"),
            value(&relation, "relation_id")
        );
        assert!(shown.contains("relation_type=supports"));
        assert_eq!(
            value(&shown, "source_record_entity_id"),
            value(&finding, "record_entity_id")
        );
        assert_eq!(
            value(&shown, "target_record_entity_id"),
            value(&decision, "record_entity_id")
        );

        let branch_shown = run(Cli::try_parse_from([
            "workvcs",
            "record",
            "relation-show",
            store,
            "--branch",
            &branch,
            "--relation",
            &value(&relation, "relation_id"),
        ])
        .expect("parse branch relation show"))
        .expect("show record relation at branch head");
        assert_eq!(
            value(&branch_shown, "relation_id"),
            value(&relation, "relation_id")
        );

        let why_decision = run(Cli::try_parse_from([
            "workvcs",
            "why",
            store,
            "--commit",
            &value(&relation, "commit_id"),
            "--entity",
            &value(&decision, "record_entity_id"),
        ])
        .expect("parse why decision"))
        .expect("why decision");
        assert!(why_decision.contains("relation_edges=1"));
        assert!(why_decision.contains("relation.0.relation_kind=record_supports"));
        assert!(why_decision.contains("relation.0.direction=incoming"));
    }

    #[test]
    fn cli_links_finding_to_contradicted_decision() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let path = tempdir.path().join("workvcs.sqlite");
        let store = path.to_str().expect("path text");

        run(
            Cli::try_parse_from(["workvcs", "init", store, "--display-name", "cli-store"])
                .expect("parse init"),
        )
        .expect("run init");
        let workspace = run(Cli::try_parse_from([
            "workvcs",
            "workspace",
            "create",
            store,
            "--display-name",
            "workspace",
        ])
        .expect("parse workspace"))
        .expect("create workspace");
        let branch = value(&workspace, "branch_id");
        let head = value(&workspace, "genesis_commit_id");

        let decision = run(Cli::try_parse_from([
            "workvcs",
            "record",
            "decision",
            store,
            "--branch",
            &branch,
            "--head",
            &head,
            "--statement",
            "Use optimistic writes",
        ])
        .expect("parse decision"))
        .expect("create decision");
        let finding = run(Cli::try_parse_from([
            "workvcs",
            "record",
            "finding",
            store,
            "--branch",
            &branch,
            "--head",
            &value(&decision, "commit_id"),
            "--statement",
            "Concurrent write tests fail without serialization",
        ])
        .expect("parse finding"))
        .expect("create finding");

        let relation = run(Cli::try_parse_from([
            "workvcs",
            "record",
            "link-contradicts",
            store,
            "--branch",
            &branch,
            "--head",
            &value(&finding, "commit_id"),
            "--source-record",
            &value(&finding, "record_entity_id"),
            "--target-record",
            &value(&decision, "record_entity_id"),
            "--rationale",
            "Finding contradicts the decision",
        ])
        .expect("parse link contradicts"))
        .expect("link contradicts");
        assert!(relation.contains("relation_type=contradicts"));

        let listed = run(Cli::try_parse_from([
            "workvcs",
            "record",
            "relation-list",
            store,
            "--commit",
            &value(&relation, "commit_id"),
            "--type",
            "contradicts",
        ])
        .expect("parse relation list"))
        .expect("list record relations");
        assert!(listed.contains("relations=1"));
        assert_eq!(
            value(&listed, "relation.0.relation_id"),
            value(&relation, "relation_id")
        );
        assert!(listed.contains("relation.0.relation_type=contradicts"));

        let why_decision = run(Cli::try_parse_from([
            "workvcs",
            "why",
            store,
            "--commit",
            &value(&relation, "commit_id"),
            "--entity",
            &value(&decision, "record_entity_id"),
        ])
        .expect("parse why decision"))
        .expect("why decision");
        assert!(why_decision.contains("relation_edges=1"));
        assert!(why_decision.contains("relation.0.relation_kind=record_contradicts"));
        assert!(why_decision.contains("relation.0.direction=incoming"));
    }

    #[test]
    fn cli_links_custom_related_records_with_label() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let path = tempdir.path().join("workvcs.sqlite");
        let store = path.to_str().expect("path text");

        run(
            Cli::try_parse_from(["workvcs", "init", store, "--display-name", "cli-store"])
                .expect("parse init"),
        )
        .expect("run init");
        let workspace = run(Cli::try_parse_from([
            "workvcs",
            "workspace",
            "create",
            store,
            "--display-name",
            "workspace",
        ])
        .expect("parse workspace"))
        .expect("create workspace");
        let branch = value(&workspace, "branch_id");
        let head = value(&workspace, "genesis_commit_id");

        let finding = run(Cli::try_parse_from([
            "workvcs",
            "record",
            "finding",
            store,
            "--branch",
            &branch,
            "--head",
            &head,
            "--statement",
            "Concurrent write tests fail without serialization",
        ])
        .expect("parse finding"))
        .expect("create finding");
        let decision = run(Cli::try_parse_from([
            "workvcs",
            "record",
            "decision",
            store,
            "--branch",
            &branch,
            "--head",
            &value(&finding, "commit_id"),
            "--statement",
            "Use serialized writes",
        ])
        .expect("parse decision"))
        .expect("create decision");

        let relation = run(Cli::try_parse_from([
            "workvcs",
            "record",
            "link-related-to",
            store,
            "--branch",
            &branch,
            "--head",
            &value(&decision, "commit_id"),
            "--source-record",
            &value(&finding, "record_entity_id"),
            "--target-record",
            &value(&decision, "record_entity_id"),
            "--label",
            "caused_by",
            "--rationale",
            "Finding caused the decision",
        ])
        .expect("parse link related_to"))
        .expect("link related_to");
        assert!(relation.contains("relation_type=related_to"));
        assert!(relation.contains("relation_label=caused_by"));

        let listed = run(Cli::try_parse_from([
            "workvcs",
            "record",
            "relation-list",
            store,
            "--commit",
            &value(&relation, "commit_id"),
            "--type",
            "related_to",
            "--label",
            "caused_by",
        ])
        .expect("parse relation list"))
        .expect("list related_to relations");
        assert!(listed.contains("relations=1"));
        assert_eq!(
            value(&listed, "relation.0.relation_id"),
            value(&relation, "relation_id")
        );
        assert!(listed.contains("relation.0.relation_label=caused_by"));

        let shown = run(Cli::try_parse_from([
            "workvcs",
            "record",
            "relation-show",
            store,
            "--commit",
            &value(&relation, "commit_id"),
            "--relation",
            &value(&relation, "relation_id"),
        ])
        .expect("parse relation show"))
        .expect("show related_to relation");
        assert!(shown.contains("relation_type=related_to"));
        assert!(shown.contains("relation_label=caused_by"));

        let why_decision = run(Cli::try_parse_from([
            "workvcs",
            "why",
            store,
            "--commit",
            &value(&relation, "commit_id"),
            "--entity",
            &value(&decision, "record_entity_id"),
        ])
        .expect("parse why decision"))
        .expect("why decision");
        assert!(why_decision.contains("relation.0.relation_kind=record_related_to"));
        assert!(why_decision.contains("relation.0.relation_label=caused_by"));
    }

    #[test]
    fn cli_removes_record_relation_from_current_projection() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let path = tempdir.path().join("workvcs.sqlite");
        let store = path.to_str().expect("path text");

        run(
            Cli::try_parse_from(["workvcs", "init", store, "--display-name", "cli-store"])
                .expect("parse init"),
        )
        .expect("run init");
        let workspace = run(Cli::try_parse_from([
            "workvcs",
            "workspace",
            "create",
            store,
            "--display-name",
            "workspace",
        ])
        .expect("parse workspace"))
        .expect("create workspace");
        let branch = value(&workspace, "branch_id");
        let head = value(&workspace, "genesis_commit_id");

        let decision = run(Cli::try_parse_from([
            "workvcs",
            "record",
            "decision",
            store,
            "--branch",
            &branch,
            "--head",
            &head,
            "--statement",
            "Use serialized writes",
        ])
        .expect("parse decision"))
        .expect("create decision");
        let finding = run(Cli::try_parse_from([
            "workvcs",
            "record",
            "finding",
            store,
            "--branch",
            &branch,
            "--head",
            &value(&decision, "commit_id"),
            "--statement",
            "The benchmark no longer supports this decision",
        ])
        .expect("parse finding"))
        .expect("create finding");
        let relation = run(Cli::try_parse_from([
            "workvcs",
            "record",
            "link-supports",
            store,
            "--branch",
            &branch,
            "--head",
            &value(&finding, "commit_id"),
            "--source-record",
            &value(&finding, "record_entity_id"),
            "--target-record",
            &value(&decision, "record_entity_id"),
            "--rationale",
            "Finding initially supports the decision",
        ])
        .expect("parse link supports"))
        .expect("link supports");

        let removed = run(Cli::try_parse_from([
            "workvcs",
            "record",
            "relation-remove",
            store,
            "--branch",
            &branch,
            "--head",
            &value(&relation, "commit_id"),
            "--relation",
            &value(&relation, "relation_id"),
            "--relation-version",
            &value(&relation, "relation_version_id"),
            "--rationale",
            "Finding no longer supports the decision",
        ])
        .expect("parse relation remove"))
        .expect("remove relation");
        assert!(removed.contains("relation_type=supports"));
        assert_eq!(
            value(&removed, "relation_id"),
            value(&relation, "relation_id")
        );
        assert_eq!(
            value(&removed, "previous_relation_version_id"),
            value(&relation, "relation_version_id")
        );

        let listed_after = run(Cli::try_parse_from([
            "workvcs",
            "record",
            "relation-list",
            store,
            "--commit",
            &value(&removed, "commit_id"),
        ])
        .expect("parse relation list"))
        .expect("list after removal");
        assert!(listed_after.contains("relations=0"));

        let show_after = run(Cli::try_parse_from([
            "workvcs",
            "record",
            "relation-show",
            store,
            "--commit",
            &value(&removed, "commit_id"),
            "--relation",
            &value(&relation, "relation_id"),
        ])
        .expect("parse relation show"))
        .expect_err("removed relation should not be current");
        assert!(show_after.to_string().contains("is not present"));

        let historical = run(Cli::try_parse_from([
            "workvcs",
            "record",
            "relation-show",
            store,
            "--commit",
            &value(&relation, "commit_id"),
            "--relation",
            &value(&relation, "relation_id"),
        ])
        .expect("parse historical relation show"))
        .expect("historical relation remains visible");
        assert!(historical.contains("relation_type=supports"));
        assert_eq!(
            value(&historical, "relation_version_id"),
            value(&relation, "relation_version_id")
        );

        let restored = run(Cli::try_parse_from([
            "workvcs",
            "record",
            "relation-restore",
            store,
            "--branch",
            &branch,
            "--head",
            &value(&removed, "commit_id"),
            "--relation",
            &value(&relation, "relation_id"),
            "--relation-version",
            &value(&relation, "relation_version_id"),
            "--rationale",
            "Restore support relation",
        ])
        .expect("parse relation restore"))
        .expect("restore relation");
        assert!(restored.contains("relation_type=supports"));
        assert_eq!(
            value(&restored, "relation_id"),
            value(&relation, "relation_id")
        );
        assert_eq!(
            value(&restored, "relation_version_id"),
            value(&relation, "relation_version_id")
        );

        let listed_restored = run(Cli::try_parse_from([
            "workvcs",
            "record",
            "relation-list",
            store,
            "--commit",
            &value(&restored, "commit_id"),
        ])
        .expect("parse relation list after restore"))
        .expect("list after restore");
        assert!(listed_restored.contains("relations=1"));
        assert_eq!(
            value(&listed_restored, "relation.0.relation_id"),
            value(&relation, "relation_id")
        );
    }

    #[test]
    fn cli_creates_and_queries_knowledge() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let path = tempdir.path().join("workvcs.sqlite");
        let store = path.to_str().expect("path text");

        run(
            Cli::try_parse_from(["workvcs", "init", store, "--display-name", "cli-store"])
                .expect("parse init"),
        )
        .expect("run init");
        let workspace = run(Cli::try_parse_from([
            "workvcs",
            "workspace",
            "create",
            store,
            "--display-name",
            "workspace",
        ])
        .expect("parse workspace"))
        .expect("create workspace");
        let branch = value(&workspace, "branch_id");
        let head = value(&workspace, "genesis_commit_id");

        let knowledge = run(Cli::try_parse_from([
            "workvcs",
            "knowledge",
            "create",
            store,
            "--branch",
            &branch,
            "--head",
            &head,
            "--statement",
            "SQLite is sufficient under serialized writes",
            "--scope-json",
            "{\"local_ref\":\"root\",\"kind\":\"workspace\"}",
            "--provenance-json",
            "{\"source\":\"cli\"}",
        ])
        .expect("parse knowledge create"))
        .expect("create knowledge");
        assert_eq!(value(&knowledge, "knowledge_status"), "active");
        assert!(
            knowledge.contains(
                "knowledge_statement_json=\"SQLite is sufficient under serialized writes\""
            )
        );
        assert!(
            knowledge
                .contains("knowledge_scope_json={\"kind\":\"workspace\",\"local_ref\":\"root\"}")
        );
        assert!(knowledge.contains("knowledge_provenance_json={\"source\":\"cli\"}"));

        let other_knowledge = run(Cli::try_parse_from([
            "workvcs",
            "knowledge",
            "create",
            store,
            "--branch",
            &branch,
            "--head",
            &value(&knowledge, "commit_id"),
            "--statement",
            "BLAKE3 state digests are stable for module projections",
            "--scope-json",
            "{\"kind\":\"module\",\"local_ref\":\"core\"}",
        ])
        .expect("parse other knowledge create"))
        .expect("create other knowledge");

        let why = run(Cli::try_parse_from([
            "workvcs",
            "why",
            store,
            "--branch",
            &branch,
            "--entity",
            &value(&knowledge, "knowledge_entity_id"),
        ])
        .expect("parse why knowledge"))
        .expect("why knowledge");
        assert_eq!(value(&why, "subject_kind"), "entity");
        assert_eq!(value(&why, "subject_entity_kind"), "knowledge");
        assert_eq!(
            value(&why, "subject_entity_version_id"),
            value(&knowledge, "knowledge_entity_version_id")
        );
        assert_eq!(value(&why, "relation_edges"), "0");

        let show = run(Cli::try_parse_from([
            "workvcs",
            "knowledge",
            "show",
            store,
            "--branch",
            &branch,
            "--knowledge",
            &value(&knowledge, "knowledge_entity_id"),
        ])
        .expect("parse knowledge show"))
        .expect("show knowledge");
        assert_eq!(
            value(&show, "knowledge_entity_version_id"),
            value(&knowledge, "knowledge_entity_version_id")
        );
        assert!(
            show.contains("knowledge_scope_json={\"kind\":\"workspace\",\"local_ref\":\"root\"}")
        );

        let listed = run(Cli::try_parse_from([
            "workvcs",
            "knowledge",
            "list",
            store,
            "--branch",
            &branch,
            "--status",
            "active",
            "--scope-json",
            "{\"kind\":\"workspace\",\"local_ref\":\"root\"}",
            "--statement-contains",
            "serialized",
        ])
        .expect("parse knowledge list"))
        .expect("list knowledge");
        assert_eq!(value(&listed, "knowledge"), "1");
        assert_eq!(
            value(&listed, "knowledge.0.knowledge_entity_id"),
            value(&knowledge, "knowledge_entity_id")
        );

        let invalidated = run(Cli::try_parse_from([
            "workvcs",
            "knowledge",
            "invalidate",
            store,
            "--branch",
            &branch,
            "--head",
            &value(&other_knowledge, "commit_id"),
            "--knowledge",
            &value(&knowledge, "knowledge_entity_id"),
            "--knowledge-version",
            &value(&knowledge, "knowledge_entity_version_id"),
            "--rationale",
            "Later evidence invalidated the statement",
        ])
        .expect("parse knowledge invalidate"))
        .expect("invalidate knowledge");
        assert_eq!(value(&invalidated, "previous_knowledge_status"), "active");
        assert_eq!(value(&invalidated, "knowledge_status"), "invalidated");

        let current = run(Cli::try_parse_from([
            "workvcs",
            "knowledge",
            "show",
            store,
            "--branch",
            &branch,
            "--knowledge",
            &value(&knowledge, "knowledge_entity_id"),
        ])
        .expect("parse current knowledge show"))
        .expect("show current knowledge");
        assert_eq!(value(&current, "knowledge_status"), "invalidated");

        let historical_active = run(Cli::try_parse_from([
            "workvcs",
            "knowledge",
            "show",
            store,
            "--commit",
            &value(&knowledge, "commit_id"),
            "--knowledge",
            &value(&knowledge, "knowledge_entity_id"),
        ])
        .expect("parse historical knowledge show"))
        .expect("show historical knowledge");
        assert_eq!(value(&historical_active, "knowledge_status"), "active");

        let active_after_invalidation = run(Cli::try_parse_from([
            "workvcs",
            "knowledge",
            "list",
            store,
            "--branch",
            &branch,
            "--status",
            "active",
        ])
        .expect("parse active knowledge list"))
        .expect("list active knowledge");
        assert_eq!(value(&active_after_invalidation, "knowledge"), "1");
        assert_eq!(
            value(
                &active_after_invalidation,
                "knowledge.0.knowledge_entity_id"
            ),
            value(&other_knowledge, "knowledge_entity_id")
        );

        let invalidated_list = run(Cli::try_parse_from([
            "workvcs",
            "knowledge",
            "list",
            store,
            "--branch",
            &branch,
            "--status",
            "invalidated",
        ])
        .expect("parse invalidated knowledge list"))
        .expect("list invalidated knowledge");
        assert_eq!(value(&invalidated_list, "knowledge"), "1");

        let empty =
            run(
                Cli::try_parse_from(["workvcs", "knowledge", "list", store, "--commit", &head])
                    .expect("parse historical knowledge list"),
            )
            .expect("list historical knowledge");
        assert_eq!(value(&empty, "knowledge"), "0");
    }

    #[test]
    fn cli_supersedes_knowledge() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let path = tempdir.path().join("workvcs.sqlite");
        let store = path.to_str().expect("path text");

        run(
            Cli::try_parse_from(["workvcs", "init", store, "--display-name", "cli-store"])
                .expect("parse init"),
        )
        .expect("run init");
        let workspace = run(Cli::try_parse_from([
            "workvcs",
            "workspace",
            "create",
            store,
            "--display-name",
            "workspace",
        ])
        .expect("parse workspace"))
        .expect("create workspace");
        let branch = value(&workspace, "branch_id");

        let knowledge = run(Cli::try_parse_from([
            "workvcs",
            "knowledge",
            "create",
            store,
            "--branch",
            &branch,
            "--head",
            &value(&workspace, "genesis_commit_id"),
            "--statement",
            "Use the old context summary format",
        ])
        .expect("parse knowledge create"))
        .expect("create knowledge");

        let superseded = run(Cli::try_parse_from([
            "workvcs",
            "knowledge",
            "supersede",
            store,
            "--branch",
            &branch,
            "--head",
            &value(&knowledge, "commit_id"),
            "--knowledge",
            &value(&knowledge, "knowledge_entity_id"),
            "--knowledge-version",
            &value(&knowledge, "knowledge_entity_version_id"),
            "--rationale",
            "The context summary format was replaced",
        ])
        .expect("parse knowledge supersede"))
        .expect("supersede knowledge");
        assert_eq!(value(&superseded, "previous_knowledge_status"), "active");
        assert_eq!(value(&superseded, "knowledge_status"), "superseded");

        let superseded_list = run(Cli::try_parse_from([
            "workvcs",
            "knowledge",
            "list",
            store,
            "--branch",
            &branch,
            "--status",
            "superseded",
        ])
        .expect("parse superseded knowledge list"))
        .expect("list superseded knowledge");
        assert_eq!(value(&superseded_list, "knowledge"), "1");
        assert_eq!(
            value(&superseded_list, "knowledge.0.knowledge_entity_id"),
            value(&knowledge, "knowledge_entity_id")
        );

        let active_list = run(Cli::try_parse_from([
            "workvcs",
            "knowledge",
            "list",
            store,
            "--branch",
            &branch,
            "--status",
            "active",
        ])
        .expect("parse active knowledge list"))
        .expect("list active knowledge");
        assert_eq!(value(&active_list, "knowledge"), "0");
    }

    #[test]
    fn cli_links_knowledge_supersession() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let path = tempdir.path().join("workvcs.sqlite");
        let store = path.to_str().expect("path text");

        run(
            Cli::try_parse_from(["workvcs", "init", store, "--display-name", "cli-store"])
                .expect("parse init"),
        )
        .expect("run init");
        let workspace = run(Cli::try_parse_from([
            "workvcs",
            "workspace",
            "create",
            store,
            "--display-name",
            "workspace",
        ])
        .expect("parse workspace"))
        .expect("create workspace");
        let branch = value(&workspace, "branch_id");

        let prior = run(Cli::try_parse_from([
            "workvcs",
            "knowledge",
            "create",
            store,
            "--branch",
            &branch,
            "--head",
            &value(&workspace, "genesis_commit_id"),
            "--statement",
            "Use the old context summary format",
        ])
        .expect("parse prior knowledge create"))
        .expect("create prior knowledge");
        let replacement = run(Cli::try_parse_from([
            "workvcs",
            "knowledge",
            "create",
            store,
            "--branch",
            &branch,
            "--head",
            &value(&prior, "commit_id"),
            "--statement",
            "Use the scoped context summary format",
        ])
        .expect("parse replacement knowledge create"))
        .expect("create replacement knowledge");
        let superseded = run(Cli::try_parse_from([
            "workvcs",
            "knowledge",
            "supersede",
            store,
            "--branch",
            &branch,
            "--head",
            &value(&replacement, "commit_id"),
            "--knowledge",
            &value(&prior, "knowledge_entity_id"),
            "--knowledge-version",
            &value(&prior, "knowledge_entity_version_id"),
            "--rationale",
            "The scoped context summary format replaced it",
        ])
        .expect("parse knowledge supersede"))
        .expect("supersede prior knowledge");

        let relation = run(Cli::try_parse_from([
            "workvcs",
            "knowledge",
            "link-supersedes",
            store,
            "--branch",
            &branch,
            "--head",
            &value(&superseded, "commit_id"),
            "--replacement-knowledge",
            &value(&replacement, "knowledge_entity_id"),
            "--prior-knowledge",
            &value(&prior, "knowledge_entity_id"),
            "--rationale",
            "The replacement Knowledge supersedes the prior statement",
        ])
        .expect("parse knowledge link supersedes"))
        .expect("link knowledge supersedes");
        assert_eq!(value(&relation, "relation_type"), "supersedes");
        assert_eq!(
            value(&relation, "replacement_knowledge_entity_id"),
            value(&replacement, "knowledge_entity_id")
        );
        assert_eq!(
            value(&relation, "prior_knowledge_entity_id"),
            value(&prior, "knowledge_entity_id")
        );

        let listed = run(Cli::try_parse_from([
            "workvcs",
            "knowledge",
            "relation-list",
            store,
            "--commit",
            &value(&relation, "commit_id"),
            "--prior-knowledge",
            &value(&prior, "knowledge_entity_id"),
        ])
        .expect("parse knowledge relation list"))
        .expect("list knowledge relations");
        assert_eq!(value(&listed, "relations"), "1");
        assert_eq!(
            value(&listed, "relation.0.relation_id"),
            value(&relation, "relation_id")
        );
        assert_eq!(value(&listed, "relation.0.relation_type"), "supersedes");
        assert_eq!(
            value(&listed, "relation.0.replacement_knowledge_entity_id"),
            value(&replacement, "knowledge_entity_id")
        );
        assert_eq!(
            value(&listed, "relation.0.prior_knowledge_entity_id"),
            value(&prior, "knowledge_entity_id")
        );

        let shown = run(Cli::try_parse_from([
            "workvcs",
            "knowledge",
            "relation-show",
            store,
            "--commit",
            &value(&relation, "commit_id"),
            "--relation",
            &value(&relation, "relation_id"),
        ])
        .expect("parse knowledge relation show"))
        .expect("show knowledge relation");
        assert_eq!(
            value(&shown, "relation_version_id"),
            value(&relation, "relation_version_id")
        );
        assert_eq!(
            value(&shown, "replacement_knowledge_entity_id"),
            value(&replacement, "knowledge_entity_id")
        );
        assert_eq!(
            value(&shown, "prior_knowledge_entity_id"),
            value(&prior, "knowledge_entity_id")
        );

        let why = run(Cli::try_parse_from([
            "workvcs",
            "why",
            store,
            "--commit",
            &value(&relation, "commit_id"),
            "--entity",
            &value(&prior, "knowledge_entity_id"),
        ])
        .expect("parse why prior knowledge"))
        .expect("why prior knowledge");
        assert_eq!(value(&why, "subject_entity_kind"), "knowledge");
        assert_eq!(value(&why, "relation_edges"), "1");
        assert_eq!(
            value(&why, "relation.0.relation_kind"),
            "knowledge_supersedes"
        );
        assert_eq!(value(&why, "relation.0.direction"), "incoming");
        assert_eq!(value(&why, "relation.0.source_entity_kind"), "knowledge");
        assert_eq!(value(&why, "relation.0.target_entity_kind"), "knowledge");

        let removed = run(Cli::try_parse_from([
            "workvcs",
            "knowledge",
            "relation-remove",
            store,
            "--branch",
            &branch,
            "--head",
            &value(&relation, "commit_id"),
            "--relation",
            &value(&relation, "relation_id"),
            "--relation-version",
            &value(&relation, "relation_version_id"),
            "--rationale",
            "The lineage edge was recorded against the wrong prior Knowledge",
        ])
        .expect("parse knowledge relation remove"))
        .expect("remove knowledge relation");
        assert_eq!(value(&removed, "relation_type"), "supersedes");
        assert_eq!(
            value(&removed, "previous_relation_version_id"),
            value(&relation, "relation_version_id")
        );

        let after_remove = run(Cli::try_parse_from([
            "workvcs",
            "knowledge",
            "relation-list",
            store,
            "--branch",
            &branch,
        ])
        .expect("parse knowledge relation list after remove"))
        .expect("list knowledge relations after remove");
        assert_eq!(value(&after_remove, "relations"), "0");

        let why_after_remove = run(Cli::try_parse_from([
            "workvcs",
            "why",
            store,
            "--branch",
            &branch,
            "--entity",
            &value(&prior, "knowledge_entity_id"),
        ])
        .expect("parse why after knowledge relation remove"))
        .expect("why after knowledge relation remove");
        assert_eq!(value(&why_after_remove, "relation_edges"), "0");

        let restored = run(Cli::try_parse_from([
            "workvcs",
            "knowledge",
            "relation-restore",
            store,
            "--branch",
            &branch,
            "--head",
            &value(&removed, "commit_id"),
            "--relation",
            &value(&relation, "relation_id"),
            "--relation-version",
            &value(&relation, "relation_version_id"),
            "--rationale",
            "Restore the corrected Knowledge lineage edge",
        ])
        .expect("parse knowledge relation restore"))
        .expect("restore knowledge relation");
        assert_eq!(value(&restored, "relation_type"), "supersedes");
        assert_eq!(
            value(&restored, "relation_version_id"),
            value(&relation, "relation_version_id")
        );
        assert_eq!(
            value(&restored, "relation_state_digest"),
            value(&relation, "relation_state_digest")
        );

        let after_restore = run(Cli::try_parse_from([
            "workvcs",
            "knowledge",
            "relation-list",
            store,
            "--branch",
            &branch,
        ])
        .expect("parse knowledge relation list after restore"))
        .expect("list knowledge relations after restore");
        assert_eq!(value(&after_restore, "relations"), "1");

        let why_after_restore = run(Cli::try_parse_from([
            "workvcs",
            "why",
            store,
            "--branch",
            &branch,
            "--entity",
            &value(&prior, "knowledge_entity_id"),
        ])
        .expect("parse why after knowledge relation restore"))
        .expect("why after knowledge relation restore");
        assert_eq!(value(&why_after_restore, "relation_edges"), "1");
        assert_eq!(
            value(&why_after_restore, "relation.0.relation_kind"),
            "knowledge_supersedes"
        );
    }

    #[test]
    fn cli_links_record_support_to_knowledge() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let path = tempdir.path().join("workvcs.sqlite");
        let store = path.to_str().expect("path text");

        run(
            Cli::try_parse_from(["workvcs", "init", store, "--display-name", "cli-store"])
                .expect("parse init"),
        )
        .expect("run init");
        let workspace = run(Cli::try_parse_from([
            "workvcs",
            "workspace",
            "create",
            store,
            "--display-name",
            "workspace",
        ])
        .expect("parse workspace"))
        .expect("create workspace");
        let branch = value(&workspace, "branch_id");

        let knowledge = run(Cli::try_parse_from([
            "workvcs",
            "knowledge",
            "create",
            store,
            "--branch",
            &branch,
            "--head",
            &value(&workspace, "genesis_commit_id"),
            "--statement",
            "Context summaries expose active Knowledge",
        ])
        .expect("parse knowledge create"))
        .expect("create knowledge");
        let finding = run(Cli::try_parse_from([
            "workvcs",
            "record",
            "finding",
            store,
            "--branch",
            &branch,
            "--head",
            &value(&knowledge, "commit_id"),
            "--statement",
            "The context command prints context_knowledge rows",
        ])
        .expect("parse finding"))
        .expect("create finding");

        let relation = run(Cli::try_parse_from([
            "workvcs",
            "record",
            "link-supports-knowledge",
            store,
            "--branch",
            &branch,
            "--head",
            &value(&finding, "commit_id"),
            "--source-record",
            &value(&finding, "record_entity_id"),
            "--target-knowledge",
            &value(&knowledge, "knowledge_entity_id"),
            "--rationale",
            "The observed CLI output supports the reusable statement",
        ])
        .expect("parse supports knowledge"))
        .expect("support knowledge");
        assert_eq!(value(&relation, "relation_type"), "supports");
        assert_eq!(
            value(&relation, "target_knowledge_entity_id"),
            value(&knowledge, "knowledge_entity_id")
        );

        let record_only = run(Cli::try_parse_from([
            "workvcs",
            "record",
            "relation-list",
            store,
            "--commit",
            &value(&relation, "commit_id"),
            "--type",
            "supports",
        ])
        .expect("parse record-only relation list"))
        .expect("list record-only relations");
        assert_eq!(value(&record_only, "relations"), "0");

        let why = run(Cli::try_parse_from([
            "workvcs",
            "why",
            store,
            "--commit",
            &value(&relation, "commit_id"),
            "--entity",
            &value(&knowledge, "knowledge_entity_id"),
        ])
        .expect("parse why knowledge"))
        .expect("why knowledge");
        assert_eq!(value(&why, "subject_entity_kind"), "knowledge");
        assert_eq!(value(&why, "relation_edges"), "1");
        assert_eq!(value(&why, "relation.0.relation_kind"), "record_supports");
        assert_eq!(value(&why, "relation.0.direction"), "incoming");
        assert_eq!(value(&why, "relation.0.source_entity_kind"), "record");
        assert_eq!(value(&why, "relation.0.target_entity_kind"), "knowledge");
    }

    #[test]
    fn cli_links_record_invalidation_to_knowledge() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let path = tempdir.path().join("workvcs.sqlite");
        let store = path.to_str().expect("path text");

        run(
            Cli::try_parse_from(["workvcs", "init", store, "--display-name", "cli-store"])
                .expect("parse init"),
        )
        .expect("run init");
        let workspace = run(Cli::try_parse_from([
            "workvcs",
            "workspace",
            "create",
            store,
            "--display-name",
            "workspace",
        ])
        .expect("parse workspace"))
        .expect("create workspace");
        let branch = value(&workspace, "branch_id");

        let knowledge = run(Cli::try_parse_from([
            "workvcs",
            "knowledge",
            "create",
            store,
            "--branch",
            &branch,
            "--head",
            &value(&workspace, "genesis_commit_id"),
            "--statement",
            "Context summaries always include inactive Knowledge",
        ])
        .expect("parse knowledge create"))
        .expect("create knowledge");
        let finding = run(Cli::try_parse_from([
            "workvcs",
            "record",
            "finding",
            store,
            "--branch",
            &branch,
            "--head",
            &value(&knowledge, "commit_id"),
            "--statement",
            "Invalidated Knowledge is excluded from context",
        ])
        .expect("parse finding"))
        .expect("create finding");
        let invalidated = run(Cli::try_parse_from([
            "workvcs",
            "knowledge",
            "invalidate",
            store,
            "--branch",
            &branch,
            "--head",
            &value(&finding, "commit_id"),
            "--knowledge",
            &value(&knowledge, "knowledge_entity_id"),
            "--knowledge-version",
            &value(&knowledge, "knowledge_entity_version_id"),
            "--rationale",
            "Context only includes active Knowledge",
        ])
        .expect("parse knowledge invalidate"))
        .expect("invalidate knowledge");

        let relation = run(Cli::try_parse_from([
            "workvcs",
            "record",
            "link-invalidates-knowledge",
            store,
            "--branch",
            &branch,
            "--head",
            &value(&invalidated, "commit_id"),
            "--source-record",
            &value(&finding, "record_entity_id"),
            "--target-knowledge",
            &value(&knowledge, "knowledge_entity_id"),
            "--rationale",
            "The Finding records why the Knowledge was invalidated",
        ])
        .expect("parse invalidates knowledge"))
        .expect("invalidate knowledge relation");
        assert_eq!(value(&relation, "relation_type"), "invalidates");

        let why = run(Cli::try_parse_from([
            "workvcs",
            "why",
            store,
            "--commit",
            &value(&relation, "commit_id"),
            "--entity",
            &value(&knowledge, "knowledge_entity_id"),
        ])
        .expect("parse why knowledge"))
        .expect("why knowledge");
        assert_eq!(value(&why, "subject_entity_kind"), "knowledge");
        assert_eq!(
            value(&why, "subject_entity_version_id"),
            value(&invalidated, "knowledge_entity_version_id")
        );
        assert_eq!(value(&why, "relation_edges"), "1");
        assert_eq!(
            value(&why, "relation.0.relation_kind"),
            "record_invalidates"
        );
        assert_eq!(value(&why, "relation.0.direction"), "incoming");
        assert_eq!(value(&why, "relation.0.target_entity_kind"), "knowledge");
    }

    #[test]
    fn cli_links_record_validation_to_knowledge() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let path = tempdir.path().join("workvcs.sqlite");
        let store = path.to_str().expect("path text");

        run(
            Cli::try_parse_from(["workvcs", "init", store, "--display-name", "cli-store"])
                .expect("parse init"),
        )
        .expect("run init");
        let workspace = run(Cli::try_parse_from([
            "workvcs",
            "workspace",
            "create",
            store,
            "--display-name",
            "workspace",
        ])
        .expect("parse workspace"))
        .expect("create workspace");
        let branch = value(&workspace, "branch_id");

        let knowledge = run(Cli::try_parse_from([
            "workvcs",
            "knowledge",
            "create",
            store,
            "--branch",
            &branch,
            "--head",
            &value(&workspace, "genesis_commit_id"),
            "--statement",
            "Context summaries include active Knowledge",
        ])
        .expect("parse knowledge create"))
        .expect("create knowledge");
        let finding = run(Cli::try_parse_from([
            "workvcs",
            "record",
            "finding",
            store,
            "--branch",
            &branch,
            "--head",
            &value(&knowledge, "commit_id"),
            "--statement",
            "The current context command includes active Knowledge rows",
        ])
        .expect("parse finding"))
        .expect("create finding");

        let relation = run(Cli::try_parse_from([
            "workvcs",
            "record",
            "link-validates-knowledge",
            store,
            "--branch",
            &branch,
            "--head",
            &value(&finding, "commit_id"),
            "--source-record",
            &value(&finding, "record_entity_id"),
            "--target-knowledge",
            &value(&knowledge, "knowledge_entity_id"),
            "--rationale",
            "The Finding validates the reusable Knowledge statement",
        ])
        .expect("parse validates knowledge"))
        .expect("validate knowledge relation");
        assert_eq!(value(&relation, "relation_type"), "validates");

        let why = run(Cli::try_parse_from([
            "workvcs",
            "why",
            store,
            "--commit",
            &value(&relation, "commit_id"),
            "--entity",
            &value(&knowledge, "knowledge_entity_id"),
        ])
        .expect("parse why knowledge"))
        .expect("why knowledge");
        assert_eq!(value(&why, "subject_entity_kind"), "knowledge");
        assert_eq!(value(&why, "relation_edges"), "1");
        assert_eq!(value(&why, "relation.0.relation_kind"), "record_validates");
        assert_eq!(value(&why, "relation.0.direction"), "incoming");
        assert_eq!(value(&why, "relation.0.target_entity_kind"), "knowledge");
    }

    #[test]
    fn cli_links_record_contradiction_to_knowledge() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let path = tempdir.path().join("workvcs.sqlite");
        let store = path.to_str().expect("path text");

        run(
            Cli::try_parse_from(["workvcs", "init", store, "--display-name", "cli-store"])
                .expect("parse init"),
        )
        .expect("run init");
        let workspace = run(Cli::try_parse_from([
            "workvcs",
            "workspace",
            "create",
            store,
            "--display-name",
            "workspace",
        ])
        .expect("parse workspace"))
        .expect("create workspace");
        let branch = value(&workspace, "branch_id");

        let knowledge = run(Cli::try_parse_from([
            "workvcs",
            "knowledge",
            "create",
            store,
            "--branch",
            &branch,
            "--head",
            &value(&workspace, "genesis_commit_id"),
            "--statement",
            "All validation output is deterministic",
        ])
        .expect("parse knowledge create"))
        .expect("create knowledge");
        let finding = run(Cli::try_parse_from([
            "workvcs",
            "record",
            "finding",
            store,
            "--branch",
            &branch,
            "--head",
            &value(&knowledge, "commit_id"),
            "--statement",
            "Validation output includes timestamps from external tools",
        ])
        .expect("parse finding"))
        .expect("create finding");

        let relation = run(Cli::try_parse_from([
            "workvcs",
            "record",
            "link-contradicts-knowledge",
            store,
            "--branch",
            &branch,
            "--head",
            &value(&finding, "commit_id"),
            "--source-record",
            &value(&finding, "record_entity_id"),
            "--target-knowledge",
            &value(&knowledge, "knowledge_entity_id"),
            "--rationale",
            "The Finding contradicts the Knowledge statement",
        ])
        .expect("parse contradicts knowledge"))
        .expect("contradict knowledge relation");
        assert_eq!(value(&relation, "relation_type"), "contradicts");

        let why = run(Cli::try_parse_from([
            "workvcs",
            "why",
            store,
            "--commit",
            &value(&relation, "commit_id"),
            "--entity",
            &value(&knowledge, "knowledge_entity_id"),
        ])
        .expect("parse why knowledge"))
        .expect("why knowledge");
        assert_eq!(value(&why, "subject_entity_kind"), "knowledge");
        assert_eq!(value(&why, "relation_edges"), "1");
        assert_eq!(
            value(&why, "relation.0.relation_kind"),
            "record_contradicts"
        );
        assert_eq!(value(&why, "relation.0.direction"), "incoming");
        assert_eq!(value(&why, "relation.0.target_entity_kind"), "knowledge");
    }

    #[test]
    fn cli_lists_record_knowledge_relations() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let path = tempdir.path().join("workvcs.sqlite");
        let store = path.to_str().expect("path text");

        run(
            Cli::try_parse_from(["workvcs", "init", store, "--display-name", "cli-store"])
                .expect("parse init"),
        )
        .expect("run init");
        let workspace = run(Cli::try_parse_from([
            "workvcs",
            "workspace",
            "create",
            store,
            "--display-name",
            "workspace",
        ])
        .expect("parse workspace"))
        .expect("create workspace");
        let branch = value(&workspace, "branch_id");

        let knowledge = run(Cli::try_parse_from([
            "workvcs",
            "knowledge",
            "create",
            store,
            "--branch",
            &branch,
            "--head",
            &value(&workspace, "genesis_commit_id"),
            "--statement",
            "Context overviews include active reusable Knowledge",
        ])
        .expect("parse knowledge create"))
        .expect("create knowledge");
        let finding = run(Cli::try_parse_from([
            "workvcs",
            "record",
            "finding",
            store,
            "--branch",
            &branch,
            "--head",
            &value(&knowledge, "commit_id"),
            "--statement",
            "The context overview output contains active Knowledge rows",
        ])
        .expect("parse finding"))
        .expect("create finding");
        let relation = run(Cli::try_parse_from([
            "workvcs",
            "record",
            "link-supports-knowledge",
            store,
            "--branch",
            &branch,
            "--head",
            &value(&finding, "commit_id"),
            "--source-record",
            &value(&finding, "record_entity_id"),
            "--target-knowledge",
            &value(&knowledge, "knowledge_entity_id"),
            "--rationale",
            "The Finding supports the reusable Knowledge statement",
        ])
        .expect("parse supports knowledge"))
        .expect("support knowledge relation");

        let listed = run(Cli::try_parse_from([
            "workvcs",
            "record",
            "knowledge-relation-list",
            store,
            "--commit",
            &value(&relation, "commit_id"),
            "--type",
            "supports",
            "--source-record",
            &value(&finding, "record_entity_id"),
            "--target-knowledge",
            &value(&knowledge, "knowledge_entity_id"),
        ])
        .expect("parse record knowledge relation list"))
        .expect("list record knowledge relations");
        assert_eq!(value(&listed, "relations"), "1");
        assert_eq!(
            value(&listed, "relation.0.relation_id"),
            value(&relation, "relation_id")
        );
        assert_eq!(value(&listed, "relation.0.relation_type"), "supports");
        assert_eq!(
            value(&listed, "relation.0.source_record_entity_id"),
            value(&finding, "record_entity_id")
        );
        assert_eq!(
            value(&listed, "relation.0.target_knowledge_entity_id"),
            value(&knowledge, "knowledge_entity_id")
        );
    }

    #[test]
    fn cli_shows_record_knowledge_relation() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let path = tempdir.path().join("workvcs.sqlite");
        let store = path.to_str().expect("path text");

        run(
            Cli::try_parse_from(["workvcs", "init", store, "--display-name", "cli-store"])
                .expect("parse init"),
        )
        .expect("run init");
        let workspace = run(Cli::try_parse_from([
            "workvcs",
            "workspace",
            "create",
            store,
            "--display-name",
            "workspace",
        ])
        .expect("parse workspace"))
        .expect("create workspace");
        let branch = value(&workspace, "branch_id");

        let knowledge = run(Cli::try_parse_from([
            "workvcs",
            "knowledge",
            "create",
            store,
            "--branch",
            &branch,
            "--head",
            &value(&workspace, "genesis_commit_id"),
            "--statement",
            "Why output includes Record-to-Knowledge relation details",
        ])
        .expect("parse knowledge create"))
        .expect("create knowledge");
        let finding = run(Cli::try_parse_from([
            "workvcs",
            "record",
            "finding",
            store,
            "--branch",
            &branch,
            "--head",
            &value(&knowledge, "commit_id"),
            "--statement",
            "The relation id can be shown directly",
        ])
        .expect("parse finding"))
        .expect("create finding");
        let relation = run(Cli::try_parse_from([
            "workvcs",
            "record",
            "link-validates-knowledge",
            store,
            "--branch",
            &branch,
            "--head",
            &value(&finding, "commit_id"),
            "--source-record",
            &value(&finding, "record_entity_id"),
            "--target-knowledge",
            &value(&knowledge, "knowledge_entity_id"),
            "--rationale",
            "The Finding validates the Knowledge relation details",
        ])
        .expect("parse validates knowledge"))
        .expect("validate knowledge relation");

        let shown = run(Cli::try_parse_from([
            "workvcs",
            "record",
            "knowledge-relation-show",
            store,
            "--commit",
            &value(&relation, "commit_id"),
            "--relation",
            &value(&relation, "relation_id"),
        ])
        .expect("parse record knowledge relation show"))
        .expect("show record knowledge relation");
        assert_eq!(
            value(&shown, "relation_id"),
            value(&relation, "relation_id")
        );
        assert_eq!(value(&shown, "relation_type"), "validates");
        assert_eq!(
            value(&shown, "source_record_entity_id"),
            value(&finding, "record_entity_id")
        );
        assert_eq!(
            value(&shown, "target_knowledge_entity_id"),
            value(&knowledge, "knowledge_entity_id")
        );
    }

    #[test]
    fn cli_removes_record_knowledge_relation() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let path = tempdir.path().join("workvcs.sqlite");
        let store = path.to_str().expect("path text");

        run(
            Cli::try_parse_from(["workvcs", "init", store, "--display-name", "cli-store"])
                .expect("parse init"),
        )
        .expect("run init");
        let workspace = run(Cli::try_parse_from([
            "workvcs",
            "workspace",
            "create",
            store,
            "--display-name",
            "workspace",
        ])
        .expect("parse workspace"))
        .expect("create workspace");
        let branch = value(&workspace, "branch_id");

        let knowledge = run(Cli::try_parse_from([
            "workvcs",
            "knowledge",
            "create",
            store,
            "--branch",
            &branch,
            "--head",
            &value(&workspace, "genesis_commit_id"),
            "--statement",
            "The active context always includes this Knowledge",
        ])
        .expect("parse knowledge create"))
        .expect("create knowledge");
        let finding = run(Cli::try_parse_from([
            "workvcs",
            "record",
            "finding",
            store,
            "--branch",
            &branch,
            "--head",
            &value(&knowledge, "commit_id"),
            "--statement",
            "The active context does not include this Knowledge",
        ])
        .expect("parse finding"))
        .expect("create finding");
        let relation = run(Cli::try_parse_from([
            "workvcs",
            "record",
            "link-contradicts-knowledge",
            store,
            "--branch",
            &branch,
            "--head",
            &value(&finding, "commit_id"),
            "--source-record",
            &value(&finding, "record_entity_id"),
            "--target-knowledge",
            &value(&knowledge, "knowledge_entity_id"),
            "--rationale",
            "The Finding contradicts the Knowledge statement",
        ])
        .expect("parse contradicts knowledge"))
        .expect("contradict knowledge relation");

        let removed = run(Cli::try_parse_from([
            "workvcs",
            "record",
            "knowledge-relation-remove",
            store,
            "--branch",
            &branch,
            "--head",
            &value(&relation, "commit_id"),
            "--relation",
            &value(&relation, "relation_id"),
            "--relation-version",
            &value(&relation, "relation_version_id"),
            "--rationale",
            "The contradiction edge was entered by mistake",
        ])
        .expect("parse record knowledge relation remove"))
        .expect("remove record knowledge relation");
        assert_eq!(
            value(&removed, "relation_id"),
            value(&relation, "relation_id")
        );
        assert_eq!(value(&removed, "relation_type"), "contradicts");
        assert_eq!(
            value(&removed, "target_knowledge_entity_id"),
            value(&knowledge, "knowledge_entity_id")
        );

        let listed = run(Cli::try_parse_from([
            "workvcs",
            "record",
            "knowledge-relation-list",
            store,
            "--commit",
            &value(&removed, "commit_id"),
            "--target-knowledge",
            &value(&knowledge, "knowledge_entity_id"),
        ])
        .expect("parse record knowledge relation list"))
        .expect("list record knowledge relations");
        assert_eq!(value(&listed, "relations"), "0");
    }

    #[test]
    fn cli_restores_record_knowledge_relation() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let path = tempdir.path().join("workvcs.sqlite");
        let store = path.to_str().expect("path text");

        run(
            Cli::try_parse_from(["workvcs", "init", store, "--display-name", "cli-store"])
                .expect("parse init"),
        )
        .expect("run init");
        let workspace = run(Cli::try_parse_from([
            "workvcs",
            "workspace",
            "create",
            store,
            "--display-name",
            "workspace",
        ])
        .expect("parse workspace"))
        .expect("create workspace");
        let branch = value(&workspace, "branch_id");

        let knowledge = run(Cli::try_parse_from([
            "workvcs",
            "knowledge",
            "create",
            store,
            "--branch",
            &branch,
            "--head",
            &value(&workspace, "genesis_commit_id"),
            "--statement",
            "CLI can restore a Record-to-Knowledge relation",
        ])
        .expect("parse knowledge create"))
        .expect("create knowledge");
        let finding = run(Cli::try_parse_from([
            "workvcs",
            "record",
            "finding",
            store,
            "--branch",
            &branch,
            "--head",
            &value(&knowledge, "commit_id"),
            "--statement",
            "The relation should be restored after review",
        ])
        .expect("parse finding"))
        .expect("create finding");
        let relation = run(Cli::try_parse_from([
            "workvcs",
            "record",
            "link-supports-knowledge",
            store,
            "--branch",
            &branch,
            "--head",
            &value(&finding, "commit_id"),
            "--source-record",
            &value(&finding, "record_entity_id"),
            "--target-knowledge",
            &value(&knowledge, "knowledge_entity_id"),
            "--rationale",
            "The Finding supports the Knowledge statement",
        ])
        .expect("parse supports knowledge"))
        .expect("support knowledge relation");
        let removed = run(Cli::try_parse_from([
            "workvcs",
            "record",
            "knowledge-relation-remove",
            store,
            "--branch",
            &branch,
            "--head",
            &value(&relation, "commit_id"),
            "--relation",
            &value(&relation, "relation_id"),
            "--relation-version",
            &value(&relation, "relation_version_id"),
            "--rationale",
            "Remove relation before restoring",
        ])
        .expect("parse record knowledge relation remove"))
        .expect("remove record knowledge relation");

        let restored = run(Cli::try_parse_from([
            "workvcs",
            "record",
            "knowledge-relation-restore",
            store,
            "--branch",
            &branch,
            "--head",
            &value(&removed, "commit_id"),
            "--relation",
            &value(&relation, "relation_id"),
            "--relation-version",
            &value(&relation, "relation_version_id"),
            "--rationale",
            "Restore relation after review",
        ])
        .expect("parse record knowledge relation restore"))
        .expect("restore record knowledge relation");
        assert_eq!(
            value(&restored, "relation_id"),
            value(&relation, "relation_id")
        );
        assert_eq!(
            value(&restored, "relation_version_id"),
            value(&relation, "relation_version_id")
        );

        let listed = run(Cli::try_parse_from([
            "workvcs",
            "record",
            "knowledge-relation-list",
            store,
            "--commit",
            &value(&restored, "commit_id"),
            "--target-knowledge",
            &value(&knowledge, "knowledge_entity_id"),
        ])
        .expect("parse record knowledge relation list"))
        .expect("list record knowledge relations");
        assert_eq!(value(&listed, "relations"), "1");
        assert_eq!(
            value(&listed, "relation.0.relation_id"),
            value(&relation, "relation_id")
        );
    }

    #[test]
    fn cli_starts_merge_attempt_without_advancing_target_branch() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let path = tempdir.path().join("workvcs.sqlite");
        let store = path.to_str().expect("path text");

        run(
            Cli::try_parse_from(["workvcs", "init", store, "--display-name", "cli-store"])
                .expect("parse init"),
        )
        .expect("run init");
        let workspace = run(Cli::try_parse_from([
            "workvcs",
            "workspace",
            "create",
            store,
            "--display-name",
            "workspace",
        ])
        .expect("parse workspace"))
        .expect("create workspace");
        let workspace_id = value(&workspace, "workspace_id");
        let target_branch = value(&workspace, "branch_id");
        let genesis_head = value(&workspace, "genesis_commit_id");

        let base = run(Cli::try_parse_from([
            "workvcs",
            "task",
            "create",
            store,
            "--branch",
            &target_branch,
            "--head",
            &genesis_head,
            "--description",
            "Base task",
        ])
        .expect("parse base task"))
        .expect("create base task");
        let base_commit = value(&base, "commit_id");

        let source = run(Cli::try_parse_from([
            "workvcs",
            "branch",
            "fork",
            store,
            "--from-branch",
            &target_branch,
            "--name",
            "source",
        ])
        .expect("parse source branch"))
        .expect("fork source branch");
        let source_branch = value(&source, "branch_id");

        let target = run(Cli::try_parse_from([
            "workvcs",
            "task",
            "create",
            store,
            "--branch",
            &target_branch,
            "--head",
            &base_commit,
            "--description",
            "Target work",
        ])
        .expect("parse target task"))
        .expect("create target task");
        let target_head = value(&target, "commit_id");

        let source_task = run(Cli::try_parse_from([
            "workvcs",
            "task",
            "create",
            store,
            "--branch",
            &source_branch,
            "--head",
            &base_commit,
            "--description",
            "Source work",
        ])
        .expect("parse source task"))
        .expect("create source task");
        let source_head = value(&source_task, "commit_id");

        let session = run(Cli::try_parse_from([
            "workvcs",
            "session",
            "start",
            store,
            "--workspace",
            &workspace_id,
            "--branch",
            &target_branch,
        ])
        .expect("parse session start"))
        .expect("start session");
        let session_id = value(&session, "session_id");

        let merge = run(Cli::try_parse_from([
            "workvcs",
            "merge",
            "start",
            store,
            "--target-branch",
            &target_branch,
            "--source-branch",
            &source_branch,
            "--session",
            &session_id,
        ])
        .expect("parse merge start"))
        .expect("start merge");

        assert_eq!(value(&merge, "workspace_id"), workspace_id);
        assert_eq!(value(&merge, "target_branch_id"), target_branch);
        assert_eq!(value(&merge, "source_branch_id"), source_branch);
        assert_eq!(value(&merge, "merge_base_commit_id"), base_commit);
        assert_eq!(value(&merge, "target_head_commit_id"), target_head);
        assert_eq!(value(&merge, "source_head_commit_id"), source_head);
        assert_eq!(value(&merge, "origin_session_id"), session_id);
        assert_eq!(value(&merge, "runtime_state"), "active");

        let head = run(Cli::try_parse_from([
            "workvcs",
            "branch",
            "head",
            store,
            "--branch",
            &target_branch,
        ])
        .expect("parse target head"))
        .expect("target head");
        assert_eq!(value(&head, "head_commit_id"), target_head);
    }

    #[test]
    fn cli_aborts_merge_attempt_without_advancing_target_branch() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let path = tempdir.path().join("workvcs.sqlite");
        let store = path.to_str().expect("path text");

        run(
            Cli::try_parse_from(["workvcs", "init", store, "--display-name", "cli-store"])
                .expect("parse init"),
        )
        .expect("run init");
        let workspace = run(Cli::try_parse_from([
            "workvcs",
            "workspace",
            "create",
            store,
            "--display-name",
            "workspace",
        ])
        .expect("parse workspace"))
        .expect("create workspace");
        let workspace_id = value(&workspace, "workspace_id");
        let target_branch = value(&workspace, "branch_id");
        let genesis_head = value(&workspace, "genesis_commit_id");

        let base = run(Cli::try_parse_from([
            "workvcs",
            "task",
            "create",
            store,
            "--branch",
            &target_branch,
            "--head",
            &genesis_head,
            "--description",
            "Base task",
        ])
        .expect("parse base task"))
        .expect("create base task");
        let base_commit = value(&base, "commit_id");

        let source = run(Cli::try_parse_from([
            "workvcs",
            "branch",
            "fork",
            store,
            "--from-branch",
            &target_branch,
            "--name",
            "source",
        ])
        .expect("parse source branch"))
        .expect("fork source branch");
        let source_branch = value(&source, "branch_id");

        let target = run(Cli::try_parse_from([
            "workvcs",
            "task",
            "create",
            store,
            "--branch",
            &target_branch,
            "--head",
            &base_commit,
            "--description",
            "Target work",
        ])
        .expect("parse target task"))
        .expect("create target task");
        let target_head = value(&target, "commit_id");

        let source_task = run(Cli::try_parse_from([
            "workvcs",
            "task",
            "create",
            store,
            "--branch",
            &source_branch,
            "--head",
            &base_commit,
            "--description",
            "Source work",
        ])
        .expect("parse source task"))
        .expect("create source task");
        let source_head = value(&source_task, "commit_id");

        let session = run(Cli::try_parse_from([
            "workvcs",
            "session",
            "start",
            store,
            "--workspace",
            &workspace_id,
            "--branch",
            &target_branch,
        ])
        .expect("parse session start"))
        .expect("start session");
        let session_id = value(&session, "session_id");

        let merge = run(Cli::try_parse_from([
            "workvcs",
            "merge",
            "start",
            store,
            "--target-branch",
            &target_branch,
            "--source-branch",
            &source_branch,
            "--session",
            &session_id,
        ])
        .expect("parse merge start"))
        .expect("start merge");
        let merge_id = value(&merge, "merge_id");

        let aborted = run(Cli::try_parse_from([
            "workvcs",
            "merge",
            "abort",
            store,
            "--merge",
            &merge_id,
            "--session",
            &session_id,
            "--detail-json",
            "{\"reason\":\"no longer needed\"}",
        ])
        .expect("parse merge abort"))
        .expect("abort merge");

        assert_eq!(value(&aborted, "merge_id"), merge_id);
        assert_eq!(value(&aborted, "workspace_id"), workspace_id);
        assert_eq!(value(&aborted, "target_branch_id"), target_branch);
        assert_eq!(value(&aborted, "source_branch_id"), source_branch);
        assert_eq!(value(&aborted, "merge_base_commit_id"), base_commit);
        assert_eq!(value(&aborted, "target_head_commit_id"), target_head);
        assert_eq!(value(&aborted, "source_head_commit_id"), source_head);
        assert_eq!(value(&aborted, "origin_session_id"), session_id);
        assert_eq!(value(&aborted, "abort_session_id"), session_id);
        assert_eq!(value(&aborted, "runtime_state"), "aborted");

        let restarted = run(Cli::try_parse_from([
            "workvcs",
            "merge",
            "start",
            store,
            "--target-branch",
            &target_branch,
            "--source-branch",
            &source_branch,
        ])
        .expect("parse replacement merge"))
        .expect("start replacement merge");
        assert_ne!(value(&restarted, "merge_id"), merge_id);
        assert_eq!(value(&restarted, "merge_base_commit_id"), base_commit);

        let head = run(Cli::try_parse_from([
            "workvcs",
            "branch",
            "head",
            store,
            "--branch",
            &target_branch,
        ])
        .expect("parse target head"))
        .expect("target head");
        assert_eq!(value(&head, "head_commit_id"), target_head);
    }

    #[test]
    fn cli_shows_and_lists_merge_attempts() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let path = tempdir.path().join("workvcs.sqlite");
        let store = path.to_str().expect("path text");

        run(
            Cli::try_parse_from(["workvcs", "init", store, "--display-name", "cli-store"])
                .expect("parse init"),
        )
        .expect("run init");
        let workspace = run(Cli::try_parse_from([
            "workvcs",
            "workspace",
            "create",
            store,
            "--display-name",
            "workspace",
        ])
        .expect("parse workspace"))
        .expect("create workspace");
        let workspace_id = value(&workspace, "workspace_id");
        let target_branch = value(&workspace, "branch_id");
        let genesis_head = value(&workspace, "genesis_commit_id");

        let base = run(Cli::try_parse_from([
            "workvcs",
            "task",
            "create",
            store,
            "--branch",
            &target_branch,
            "--head",
            &genesis_head,
            "--description",
            "Base task",
        ])
        .expect("parse base task"))
        .expect("create base task");
        let base_commit = value(&base, "commit_id");

        let source = run(Cli::try_parse_from([
            "workvcs",
            "branch",
            "fork",
            store,
            "--from-branch",
            &target_branch,
            "--name",
            "source",
        ])
        .expect("parse source branch"))
        .expect("fork source branch");
        let source_branch = value(&source, "branch_id");

        let target = run(Cli::try_parse_from([
            "workvcs",
            "task",
            "create",
            store,
            "--branch",
            &target_branch,
            "--head",
            &base_commit,
            "--description",
            "Target work",
        ])
        .expect("parse target task"))
        .expect("create target task");
        let target_head = value(&target, "commit_id");

        let source_task = run(Cli::try_parse_from([
            "workvcs",
            "task",
            "create",
            store,
            "--branch",
            &source_branch,
            "--head",
            &base_commit,
            "--description",
            "Source work",
        ])
        .expect("parse source task"))
        .expect("create source task");
        let source_task_id = value(&source_task, "task_entity_id");
        let source_head = value(&source_task, "commit_id");

        let merge = run(Cli::try_parse_from([
            "workvcs",
            "merge",
            "start",
            store,
            "--target-branch",
            &target_branch,
            "--source-branch",
            &source_branch,
        ])
        .expect("parse merge start"))
        .expect("start merge");
        let merge_id = value(&merge, "merge_id");

        let shown =
            run(
                Cli::try_parse_from(["workvcs", "merge", "show", store, "--merge", &merge_id])
                    .expect("parse merge show"),
            )
            .expect("show merge");
        assert_eq!(value(&shown, "merge_id"), merge_id);
        assert_eq!(value(&shown, "workspace_id"), workspace_id);
        assert_eq!(value(&shown, "target_branch_id"), target_branch);
        assert_eq!(value(&shown, "source_branch_id"), source_branch);
        assert_eq!(value(&shown, "merge_base_commit_id"), base_commit);
        assert_eq!(value(&shown, "target_head_commit_id"), target_head);
        assert_eq!(value(&shown, "source_head_commit_id"), source_head);
        assert_eq!(value(&shown, "runtime_state"), "active");
        assert_eq!(value(&shown, "items"), "1");
        assert_eq!(value(&shown, "item.0.classification"), "AUTO");
        assert_eq!(value(&shown, "item.0.subject_kind"), "entity");
        assert_eq!(value(&shown, "item.0.subject_id"), source_task_id);
        assert_eq!(value(&shown, "item.0.resolution"), "none");
        assert_eq!(value(&shown, "outcome"), "none");
        let merge_item_id = value(&shown, "item.0.merge_item_id");

        let resolved = run(Cli::try_parse_from([
            "workvcs",
            "merge",
            "resolve",
            store,
            "--item",
            &merge_item_id,
            "--kind",
            "theirs",
            "--rationale-json",
            "{\"reason\":\"take source\"}",
        ])
        .expect("parse merge resolve"))
        .expect("resolve merge item");
        assert_eq!(value(&resolved, "merge_id"), merge_id);
        assert_eq!(value(&resolved, "merge_item_id"), merge_item_id);
        assert_eq!(value(&resolved, "resolution"), "theirs");
        assert_eq!(value(&resolved, "resolution.kind"), "theirs");
        assert_eq!(value(&resolved, "resolution.custom_payload_json"), "none");

        let shown =
            run(
                Cli::try_parse_from(["workvcs", "merge", "show", store, "--merge", &merge_id])
                    .expect("parse resolved merge show"),
            )
            .expect("show resolved merge");
        assert_eq!(value(&shown, "item.0.resolution"), "theirs");
        assert_eq!(value(&shown, "item.0.resolution.kind"), "theirs");
        assert_eq!(
            value(&shown, "item.0.resolution.rationale_json"),
            "{\"reason\":\"take source\"}"
        );

        let frozen =
            run(
                Cli::try_parse_from(["workvcs", "merge", "freeze", store, "--merge", &merge_id])
                    .expect("parse merge freeze"),
            )
            .expect("freeze merge resolutions");
        assert_eq!(value(&frozen, "merge_id"), merge_id);
        assert_eq!(value(&frozen, "workspace_id"), workspace_id);
        assert_eq!(value(&frozen, "frozen_items"), "1");

        let active = run(Cli::try_parse_from([
            "workvcs",
            "merge",
            "list",
            store,
            "--workspace",
            &workspace_id,
        ])
        .expect("parse active merge list"))
        .expect("list active merges");
        assert_eq!(value(&active, "merges"), "1");
        assert_eq!(value(&active, "merge.0.merge_id"), merge_id);
        assert_eq!(value(&active, "merge.0.runtime_state"), "active");
        assert_eq!(value(&active, "merge.0.items"), "1");

        let active_by_runtime = run(Cli::try_parse_from([
            "workvcs",
            "merge",
            "list",
            store,
            "--workspace",
            &workspace_id,
            "--runtime-state",
            "active",
        ])
        .expect("parse active merge list by runtime"))
        .expect("list active merges by runtime");
        assert_eq!(value(&active_by_runtime, "merges"), "1");
        assert_eq!(value(&active_by_runtime, "merge.0.merge_id"), merge_id);

        let active_by_outcome = run(Cli::try_parse_from([
            "workvcs",
            "merge",
            "list",
            store,
            "--workspace",
            &workspace_id,
            "--outcome",
            "none",
        ])
        .expect("parse active merge list by outcome"))
        .expect("list active merges by outcome");
        assert_eq!(value(&active_by_outcome, "merges"), "1");
        assert_eq!(value(&active_by_outcome, "merge.0.merge_id"), merge_id);

        let continued = run(Cli::try_parse_from([
            "workvcs",
            "merge",
            "continue",
            store,
            "--merge",
            &merge_id,
            "--detail-json",
            "{\"reason\":\"complete merge\"}",
        ])
        .expect("parse merge continue"))
        .expect("continue merge");
        assert_eq!(value(&continued, "merge_id"), merge_id);
        assert_eq!(value(&continued, "workspace_id"), workspace_id);
        assert_eq!(value(&continued, "target_branch_id"), target_branch);
        assert_eq!(value(&continued, "source_branch_id"), source_branch);
        assert_eq!(value(&continued, "runtime_state"), "completed");
        assert_eq!(value(&continued, "continued_by_session_id"), "none");
        let result_commit = value(&continued, "result_commit_id");

        let head = run(Cli::try_parse_from([
            "workvcs",
            "branch",
            "head",
            store,
            "--branch",
            &target_branch,
        ])
        .expect("parse continued branch head"))
        .expect("continued branch head");
        assert_eq!(value(&head, "head_commit_id"), result_commit);

        let hidden = run(Cli::try_parse_from([
            "workvcs",
            "merge",
            "list",
            store,
            "--workspace",
            &workspace_id,
        ])
        .expect("parse hidden merge list"))
        .expect("list active merges after continue");
        assert_eq!(value(&hidden, "merges"), "0");

        let closed =
            run(
                Cli::try_parse_from(["workvcs", "merge", "show", store, "--merge", &merge_id])
                    .expect("parse closed merge show"),
            )
            .expect("show closed merge");
        assert_eq!(value(&closed, "runtime_state"), "completed");
        assert_eq!(value(&closed, "items"), "1");
        assert_eq!(value(&closed, "outcome"), "completed");
        assert_eq!(value(&closed, "outcome.kind"), "completed");
        assert_eq!(value(&closed, "outcome.result_commit_id"), result_commit);
        assert_eq!(
            value(&closed, "outcome.detail_json"),
            "{\"reason\":\"complete merge\"}"
        );

        let all = run(Cli::try_parse_from([
            "workvcs",
            "merge",
            "list",
            store,
            "--workspace",
            &workspace_id,
            "--target-branch",
            &target_branch,
            "--include-closed",
        ])
        .expect("parse all merge list"))
        .expect("list closed merges");
        assert_eq!(value(&all, "merges"), "1");
        assert_eq!(value(&all, "merge.0.merge_id"), merge_id);
        assert_eq!(value(&all, "merge.0.runtime_state"), "completed");
        assert_eq!(value(&all, "merge.0.items"), "1");
        assert_eq!(value(&all, "merge.0.outcome"), "completed");

        let completed_by_runtime = run(Cli::try_parse_from([
            "workvcs",
            "merge",
            "list",
            store,
            "--workspace",
            &workspace_id,
            "--include-closed",
            "--runtime-state",
            "completed",
        ])
        .expect("parse completed merge list by runtime"))
        .expect("list completed merges by runtime");
        assert_eq!(value(&completed_by_runtime, "merges"), "1");
        assert_eq!(value(&completed_by_runtime, "merge.0.merge_id"), merge_id);

        let completed_by_outcome = run(Cli::try_parse_from([
            "workvcs",
            "merge",
            "list",
            store,
            "--workspace",
            &workspace_id,
            "--include-closed",
            "--outcome",
            "completed",
        ])
        .expect("parse completed merge list by outcome"))
        .expect("list completed merges by outcome");
        assert_eq!(value(&completed_by_outcome, "merges"), "1");
        assert_eq!(value(&completed_by_outcome, "merge.0.merge_id"), merge_id);

        let missing_by_runtime = run(Cli::try_parse_from([
            "workvcs",
            "merge",
            "list",
            store,
            "--workspace",
            &workspace_id,
            "--include-closed",
            "--runtime-state",
            "aborted",
        ])
        .expect("parse missing merge list by runtime"))
        .expect("list missing merges by runtime");
        assert_eq!(value(&missing_by_runtime, "merges"), "0");
    }

    fn value(output: &str, key: &str) -> String {
        output
            .lines()
            .find_map(|line| line.strip_prefix(&format!("{key}=")))
            .unwrap_or_else(|| panic!("missing {key} in output:\n{output}"))
            .to_owned()
    }
}
