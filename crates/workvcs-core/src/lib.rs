pub mod canonical;
pub mod engine;
pub mod error;
mod history;
pub mod identity;
mod runtime;
pub mod store;

pub use canonical::{
    CanonicalValue, WorkState, canonical_bytes, content_object_digest, entity_version_digest,
    parse_canonical_json, relation_version_digest, validate_import_fixed_point,
    work_state_mapping_digest,
};
pub use engine::Engine;
pub use error::{ErrorCategory, ErrorCode, Result, WorkVcsError};
pub use history::{
    AcceptanceCriterionClassification, AcceptanceCriterionCreateCommit,
    AcceptanceCriterionCreateOptions, AcceptanceCriterionEffectiveStatus,
    AcceptanceCriterionRevisionCommit, AcceptanceCriterionRevisionOptions,
    AcceptanceCriterionSnapshot, AcceptanceCriterionState,
    AcceptanceCriterionVerificationRequirementRef, ApplicabilityResourceObservationStatus,
    ApplicabilityResourceStampInput, ApplicabilityResourceStampSnapshot, BranchForkOptions,
    BranchForkResult, BranchForkSource, BranchHead, EntityTransitionCommit,
    EntityTransitionOptions, EntityVersionDiff, EvidenceContentInput, EvidenceContentSnapshot,
    EvidenceCreateOptions, EvidenceCreateResult, EvidenceSnapshot, GoalCreateCommit,
    GoalCreateOptions, GoalSnapshot, GoalState, GoalStatus, GoalTransitionCommit,
    GoalTransitionOptions, HistoryEntry, HistoryQueryOptions, HistoryQueryResult, HistoryStart,
    IntegrityReport, PlanCreateCommit, PlanCreateOptions, PlanSnapshot, PlanState, PlanStatus,
    PlanTransitionCommit, PlanTransitionOptions, PrimaryContainmentCreateCommit,
    PrimaryContainmentCreateOptions, PrimaryContainmentEndpointKind, PrimaryContainmentSnapshot,
    RelationVersionDiff, ReplayedState, ResolvedWhyQuerySubject, ResolvedWhyQueryTarget,
    ResolvedWorkStateDiffTarget, ResourceBindOptions, ResourceBindResult, ResourceBindingSnapshot,
    ResourceCreateOptions, ResourceCreateResult, ResourceObservationCreateOptions,
    ResourceObservationCreateResult, ResourceObservationDetailInput,
    ResourceObservationDetailSnapshot, ResourceObservationSnapshot, ResourceSnapshot,
    StructuralReferenceCreateCommit, StructuralReferenceCreateOptions,
    StructuralReferenceEndpointKind, StructuralReferenceSnapshot, TaskAcceptanceCriterionRef,
    TaskCreateCommit, TaskCreateOptions, TaskSchedulingRelationCreateCommit,
    TaskSchedulingRelationCreateOptions, TaskSchedulingRelationSnapshot,
    TaskSchedulingRelationType, TaskSnapshot, TaskState, TaskStatus, TaskTransitionCommit,
    TaskTransitionOptions, VerificationApplicability, VerificationApplicabilityCacheSnapshot,
    VerificationApplicabilityRecordOptions, VerificationCreateCommit, VerificationCreateOptions,
    VerificationEvidenceRef, VerificationEvidenceRelationCreate,
    VerificationEvidenceRelationSnapshot, VerificationRequirementCreateCommit,
    VerificationRequirementCreateOptions, VerificationRequirementRevisionCommit,
    VerificationRequirementRevisionOptions, VerificationRequirementSnapshot,
    VerificationRequirementState, VerificationResourceBasis, VerificationResult,
    VerificationSemanticDependency, VerificationSnapshot, VerificationState, VerificationTarget,
    WhyDeferredRelationFamily, WhyEntityKind, WhyQueryOptions, WhyQueryResult, WhyQuerySubject,
    WhyQueryTarget, WhyRelationDirection, WhyRelationEdge, WhyRelationEndpoint, WhyRelationKind,
    WorkStateDiff, WorkStateDiffChangeKind, WorkStateDiffOptions, WorkStateDiffTarget,
    WorkspaceInfo, WorkspaceInitOptions, WorkspaceResourceAssociationOptions,
    WorkspaceResourceAssociationResult, WorkspaceResourceAssociationSnapshot,
};
pub use identity::{
    BranchId, ChangeSetId, ClaimId, CommitId, Digest, EntityId, EntityVersionId, EventId,
    EvidenceId, OperationId, RelationId, RelationVersionId, ResourceId, ResourceObservationId,
    SessionDiffId, SessionId, StoreId, WorkspaceId,
};
pub use runtime::{
    ClaimLifecycleState, ClaimMode, ClaimReleaseOptions, ClaimReleaseResult, ClaimSnapshot,
    ClaimTaskOptions, ClaimTaskResult,
};
pub use runtime::{
    RunnableTaskBlockedReason, RunnableTaskCandidate, RunnableTaskClaimCoordination,
    RunnableTaskProjectionDimension, RunnableTasksOptions, RunnableTasksProjection,
};
pub use runtime::{
    SessionEndOptions, SessionEndResult, SessionFocus, SessionFocusOptions, SessionFocusPathEntry,
    SessionFocusUpdateResult, SessionLifecycleState, SessionSnapshot, SessionStartOptions,
    SessionStartResult, SessionSwitchOptions, SessionSwitchResult,
};
pub use store::{StoreInfo, StoreInitOptions, StoreManifest};
