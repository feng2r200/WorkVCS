use clap::{ArgGroup, Parser, Subcommand};
use std::fmt::Write as _;
use std::path::PathBuf;
use workvcs_core::{
    AcceptanceCriterionClassification, AcceptanceCriterionCreateCommit,
    AcceptanceCriterionCreateOptions, AcceptanceCriterionEffectiveStatus,
    ApplicabilityResourceObservationStatus, ApplicabilityResourceStampInput, BranchForkOptions,
    BranchForkResult, BranchHead, BranchId, BranchProjectionRefreshOptions,
    BranchProjectionRefreshResult, BranchProjectionSnapshot, CanonicalValue, ClaimId,
    ClaimLifecycleState, ClaimMode, ClaimNextOptions, ClaimNextResult, ClaimReleaseOptions,
    ClaimReleaseResult, ClaimTaskOptions, ClaimTaskResult, CommitId, ContextOverview,
    ContextOverviewOptions, DecisionRecordSupersedeCommit, DecisionRecordSupersedeOptions, Digest,
    Engine, EntityId, EntityVersionId, EvidenceId, HistoryEntry, HistoryQueryOptions,
    KnowledgeCreateCommit, KnowledgeCreateOptions, KnowledgeListOptions, KnowledgeListResult,
    KnowledgeRelationCreateCommit, KnowledgeRelationCreateOptions, KnowledgeRelationListOptions,
    KnowledgeRelationListResult, KnowledgeRelationRemoveCommit, KnowledgeRelationRemoveOptions,
    KnowledgeRelationRestoreCommit, KnowledgeRelationRestoreOptions, KnowledgeRelationSnapshot,
    KnowledgeSnapshot, KnowledgeStatus, KnowledgeTransitionCommit, KnowledgeTransitionOptions,
    MergeAbortOptions, MergeAbortResult, MergeAttemptSnapshot, MergeContinueOptions,
    MergeContinueResult, MergeFreezeResolutionsOptions, MergeFreezeResolutionsResult, MergeId,
    MergeItemId, MergeItemResolutionSnapshot, MergeItemSnapshot, MergeItemSubject,
    MergeListOptions, MergeListResult, MergeOutcomeSnapshot, MergeResolutionKind,
    MergeResolveOptions, MergeResolveResult, MergeStartOptions, MergeStartResult, NextWorkOptions,
    NextWorkResult, RecordCreateCommit, RecordCreateOptions, RecordKind,
    RecordKnowledgeRelationCreateCommit, RecordKnowledgeRelationCreateOptions,
    RecordKnowledgeRelationListOptions, RecordKnowledgeRelationListResult,
    RecordKnowledgeRelationRemoveCommit, RecordKnowledgeRelationRemoveOptions,
    RecordKnowledgeRelationRestoreCommit, RecordKnowledgeRelationRestoreOptions,
    RecordKnowledgeRelationSnapshot, RecordListOptions, RecordListResult,
    RecordRelationCreateCommit, RecordRelationCreateOptions, RecordRelationListOptions,
    RecordRelationListResult, RecordRelationRemoveCommit, RecordRelationRemoveOptions,
    RecordRelationRestoreCommit, RecordRelationRestoreOptions, RecordRelationSnapshot,
    RecordRelationType, RecordSnapshot, RecordStatus, RecordTransitionCommit,
    RecordTransitionOptions, RelationId, RelationVersionId, ReplayedState, ResolvedWhyQuerySubject,
    ResourceCreateOptions, ResourceCreateResult, ResourceId, ResourceObservationCreateOptions,
    ResourceObservationCreateResult, ResourceObservationId, Result, RunnableTaskBlockedReason,
    RunnableTaskCandidate, RunnableTaskClaimCoordination, RunnableTasksOptions,
    RunnableTasksProjection, SessionEndOptions, SessionEndResult, SessionId, SessionLifecycleState,
    SessionStartOptions, SessionStartResult, SessionSwitchOptions, SessionSwitchResult,
    StoreInitOptions, TaskCreateCommit, TaskCreateOptions, TaskStatus, TaskTransitionCommit,
    TaskTransitionOptions, VerificationApplicabilityCacheSnapshot,
    VerificationApplicabilityRecordOptions, VerificationCreateCommit, VerificationCreateOptions,
    VerificationRequirementCreateCommit, VerificationRequirementCreateOptions,
    VerificationResourceBasis, VerificationResult, VerificationTarget, WhyDeferredRelationFamily,
    WhyEntityKind, WhyQueryOptions, WhyQueryResult, WhyQueryTarget, WhyRelationDirection,
    WhyRelationEndpoint, WhyRelationKind, WorkState, WorkStateRestoreCommit,
    WorkStateRestoreOptions, WorkVcsError, WorkspaceInfo, WorkspaceInitOptions, canonical_bytes,
    content_object_digest, parse_canonical_json,
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
    ShowAt {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        commit: String,
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
            .args(["entity", "evidence"])
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
    Merge {
        #[command(subcommand)]
        command: MergeCommand,
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
}

#[derive(Debug, Subcommand)]
enum BranchCommand {
    List {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        workspace: String,
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
    Status {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        branch: String,

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
}

#[derive(Debug, Subcommand)]
enum ResourceCommand {
    Create {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        kind: String,
    },
    #[command(group(
        ArgGroup::new("resource-observation-fingerprint")
            .required(true)
            .multiple(false)
            .args(["fingerprint", "content"])
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

        #[arg(long, default_value = "{}")]
        summary_json: String,
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
    Next {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        session: String,
    },
    Task {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        session: String,

        #[arg(long)]
        task: String,
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
                "ok store_id={} schema_version={} canonical_json_profile={} checked_branches={} checked_commits={}\n",
                info.store_id,
                info.manifest.schema_version,
                info.manifest.canonical_json_profile,
                integrity.checked_branches,
                integrity.checked_commits
            ))
        }
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
        Command::ShowAt { store, commit } => {
            let engine = Engine::open(store)?;
            let state = engine.show_at(CommitId::parse_canonical(&commit)?)?;
            Ok(render_replayed_state(&state))
        }
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
        Command::Why {
            store,
            branch,
            commit,
            entity,
            evidence,
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
            let options = match (entity, evidence) {
                (Some(entity_id), None) => {
                    WhyQueryOptions::for_entity(target, EntityId::parse_canonical(&entity_id)?)
                }
                (None, Some(evidence_id)) => WhyQueryOptions::for_evidence(
                    target,
                    EvidenceId::parse_canonical(&evidence_id)?,
                ),
                _ => {
                    return Err(WorkVcsError::QueryInvalid(
                        "why requires exactly one of --entity or --evidence".to_owned(),
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
        Command::Branch {
            command: BranchCommand::List { store, workspace },
        } => {
            let engine = Engine::open(store)?;
            let branches =
                engine.list_branches(workvcs_core::WorkspaceId::parse_canonical(&workspace)?)?;
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
                TaskCommand::Transition {
                    store,
                    branch,
                    head,
                    task,
                    task_version,
                    status,
                    outcome,
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
            let transition = engine.transition_task(options)?;
            Ok(render_task_transition(&transition))
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
                AcceptanceCriterionCommand::Status {
                    store,
                    branch,
                    criterion,
                },
        } => {
            let engine = Engine::open(store)?;
            let status = engine.acceptance_criterion_effective_status_for_branch(
                BranchId::parse_canonical(&branch)?,
                EntityId::parse_canonical(&criterion)?,
            )?;
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
        Command::Resource {
            command: ResourceCommand::Create { store, kind },
        } => {
            let mut engine = Engine::open(store)?;
            let resource = engine.create_resource(ResourceCreateOptions::new(kind)?)?;
            Ok(render_resource_create(&resource))
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
                    summary_json,
                },
        } => {
            let mut engine = Engine::open(store)?;
            let fingerprint = fingerprint_from_cli(fingerprint, content)?;
            let observation =
                engine.record_resource_observation(ResourceObservationCreateOptions::new(
                    ResourceId::parse_canonical(&resource)?,
                    adapter_kind,
                    adapter_schema_version,
                    fingerprint,
                    parse_cli_object("resource observation summary", &summary_json)?,
                )?)?;
            Ok(render_resource_observation_create(&observation))
        }
        Command::Verification {
            command:
                VerificationCommand::Record {
                    store,
                    branch,
                    head,
                    result,
                    method,
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
            command: ClaimCommand::Next { store, session },
        } => {
            let mut engine = Engine::open(store)?;
            let claimed = engine
                .claim_next_task(ClaimNextOptions::new(SessionId::parse_canonical(&session)?))?;
            Ok(render_claim_next(&claimed))
        }
        Command::Claim {
            command:
                ClaimCommand::Task {
                    store,
                    session,
                    task,
                },
        } => {
            let mut engine = Engine::open(store)?;
            let claim = engine.claim_task(ClaimTaskOptions::new(
                SessionId::parse_canonical(&session)?,
                EntityId::parse_canonical(&task)?,
            ))?;
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
        Command::Next { store, session } => {
            let mut engine = Engine::open(store)?;
            let next =
                engine.next_work(NextWorkOptions::new(SessionId::parse_canonical(&session)?))?;
            Ok(render_next_work(&next))
        }
        Command::Runnable {
            command: RunnableCommand::Tasks { store, session },
        } => {
            let engine = Engine::open(store)?;
            let projection = engine.runnable_tasks(RunnableTasksOptions::new(
                SessionId::parse_canonical(&session)?,
            ))?;
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
            Ok(render_merge_list(&engine.merge_attempts(options)?)?)
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

fn fingerprint_from_cli(fingerprint: Option<String>, content: Option<String>) -> Result<Digest> {
    match (fingerprint, content) {
        (Some(fingerprint), None) => Digest::from_hex(&fingerprint),
        (None, Some(content)) => Ok(content_object_digest(content.as_bytes())),
        _ => Err(WorkVcsError::TaskInvalid(
            "expected exactly one fingerprint source".to_owned(),
        )),
    }
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
        "workspace_id={}\nbranch_id={}\nbranch_name={}\ngenesis_commit_id={}\ngenesis_changeset_id={}\nstate_digest={}\n",
        workspace.workspace_id,
        workspace.initial_branch_id,
        workspace.initial_branch_name,
        workspace.genesis_commit_id,
        workspace.genesis_changeset_id,
        workspace.state_digest
    )
}

fn render_branch_head(head: &BranchHead) -> String {
    format!(
        "workspace_id={}\nbranch_id={}\nbranch_name={}\nhead_commit_id={}\nlifecycle_state={}\nstate_digest={}\n",
        head.workspace_id,
        head.branch_id,
        head.name,
        head.head_commit_id,
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

fn render_resource_create(resource: &ResourceCreateResult) -> String {
    format!(
        "resource_id={}\nresource_kind={}\ncreated_at_us={}\n",
        resource.resource_id, resource.resource_kind, resource.created_at_us
    )
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
        "session_id={}\nlifecycle_state={}\nworkspace_id={}\nbranch_id={}\nbranch_name={}\nhead_commit_id={}\nstate_digest={}\nstarted_at_us={}\nlast_activity_at_us={}\nfocus_entity_id={}\nfocus_path_entries={}\ncontext_workspaces={}\nrunnable_candidates={}\nrunnable_ready={}\nknowledge={}\nknowledge_relations={}\nrecords={}\nrecord_relations={}\nrecord_knowledge_relations={}\n",
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
        "session_id={}\nworkspace_id={}\nbranch_id={}\nhead_commit_id={}\ninspected_candidates={}\nselected={}\ncontext_focus_entity_id={}\ncontext_workspaces={}\ncontext_runnable_candidates={}\ncontext_runnable_ready={}\ncontext_knowledge={}\ncontext_knowledge_relations={}\ncontext_records={}\ncontext_record_relations={}\ncontext_record_knowledge_relations={}\n",
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

fn claim_lifecycle_state(state: ClaimLifecycleState) -> &'static str {
    match state {
        ClaimLifecycleState::Active => "active",
        ClaimLifecycleState::Released => "released",
    }
}

fn claim_mode(mode: ClaimMode) -> &'static str {
    match mode {
        ClaimMode::Exclusive => "exclusive",
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
        "commit={} kind={} operation={} schema={} parent={} state_digest={}",
        entry.commit_id,
        entry.commit_kind,
        entry.operation_type,
        entry.operation_schema_version,
        parent,
        entry.state_digest
    );
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
                "history",
                "show-at",
                "restore",
                "why",
                "workspace",
                "branch",
                "knowledge",
                "task",
                "ac",
                "vr",
                "resource",
                "record",
                "session",
                "claim",
                "context",
                "next",
                "runnable",
                "verification",
                "projection",
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

        let branch_head =
            run(
                Cli::try_parse_from(["workvcs", "branch", "head", store, "--branch", &branch])
                    .expect("parse branch head"),
            )
            .expect("branch head");
        assert_eq!(value(&branch_head, "head_commit_id"), restore_commit);

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
    fn cli_runs_resource_backed_verification_and_cache_workflow() {
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
            "--content",
            "baseline bytes",
        ])
        .expect("parse observe"))
        .expect("record observation");
        let observation_id = value(&observation, "observation_id");
        let fingerprint = value(&observation, "fingerprint");

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
    }

    fn value(output: &str, key: &str) -> String {
        output
            .lines()
            .find_map(|line| line.strip_prefix(&format!("{key}=")))
            .unwrap_or_else(|| panic!("missing {key} in output:\n{output}"))
            .to_owned()
    }
}
