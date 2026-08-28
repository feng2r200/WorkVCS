mod entity;
mod genesis;
mod integrity;
mod query;
mod replay;
mod task;

pub use entity::{EntityTransitionCommit, EntityTransitionOptions};
pub use genesis::{WorkspaceInfo, WorkspaceInitOptions};
pub use integrity::IntegrityReport;
pub use query::{BranchHead, HistoryEntry, HistoryQueryOptions, HistoryQueryResult, HistoryStart};
pub use replay::ReplayedState;
pub use task::{
    AcceptanceCriterionClassification, AcceptanceCriterionCreateCommit,
    AcceptanceCriterionCreateOptions, AcceptanceCriterionEffectiveStatus,
    AcceptanceCriterionRevisionCommit, AcceptanceCriterionRevisionOptions,
    AcceptanceCriterionSnapshot, AcceptanceCriterionState,
    AcceptanceCriterionVerificationRequirementRef, TaskAcceptanceCriterionRef, TaskCreateCommit,
    TaskCreateOptions, TaskSnapshot, TaskState, TaskStatus, TaskTransitionCommit,
    TaskTransitionOptions, VerificationCreateCommit, VerificationCreateOptions,
    VerificationRequirementCreateCommit, VerificationRequirementCreateOptions,
    VerificationRequirementRevisionCommit, VerificationRequirementRevisionOptions,
    VerificationRequirementSnapshot, VerificationRequirementState, VerificationResult,
    VerificationSemanticDependency, VerificationSnapshot, VerificationState, VerificationTarget,
};

pub(crate) use entity::commit_entity_transition;
pub(crate) use genesis::{create_workspace, load_workspace_info};
pub(crate) use integrity::validate_integrity;
pub(crate) use query::{branch_head, query_history};
pub(crate) use replay::state_at;
pub(crate) use task::{
    acceptance_criterion_at, acceptance_criterion_effective_status, create_acceptance_criterion,
    create_task, create_verification, create_verification_requirement,
    reject_reserved_semantic_entity_transition, revise_acceptance_criterion,
    revise_verification_requirement, task_at, tasks_at, transition_task, verification_at,
    verification_requirement_at,
};
