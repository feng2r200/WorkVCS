use std::fmt;

use thiserror::Error;

pub type Result<T> = std::result::Result<T, WorkVcsError>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErrorCategory {
    Canonical,
    Evidence,
    Goal,
    Identity,
    Import,
    Integrity,
    Knowledge,
    Mutation,
    Plan,
    Query,
    Record,
    Relation,
    Replay,
    Resource,
    Runtime,
    Store,
    Storage,
    Task,
    Time,
    Workspace,
}

impl ErrorCategory {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Canonical => "canonical",
            Self::Evidence => "evidence",
            Self::Goal => "goal",
            Self::Identity => "identity",
            Self::Import => "import",
            Self::Integrity => "integrity",
            Self::Knowledge => "knowledge",
            Self::Mutation => "mutation",
            Self::Plan => "plan",
            Self::Query => "query",
            Self::Record => "record",
            Self::Relation => "relation",
            Self::Replay => "replay",
            Self::Resource => "resource",
            Self::Runtime => "runtime",
            Self::Store => "store",
            Self::Storage => "storage",
            Self::Task => "task",
            Self::Time => "time",
            Self::Workspace => "workspace",
        }
    }
}

impl fmt::Display for ErrorCategory {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErrorCode {
    CanonicalEncodingInvalid,
    DigestInvalid,
    EvidenceInvalid,
    EvidenceNotFound,
    IdentityInvalid,
    ImmutableImportInvalid,
    IntegrityInvalid,
    KnowledgeInvalid,
    KnowledgeNotFound,
    MutationPostconditionFailed,
    CommitNotFound,
    BranchHeadConflict,
    BranchNotFound,
    ClaimInvalid,
    ClaimNotFound,
    EntityNotFound,
    EntityTransitionInvalid,
    GoalInvalid,
    GoalNotFound,
    PlanInvalid,
    PlanNotFound,
    ProjectBindingNotFound,
    QueryInvalid,
    QueryUnsupported,
    RecordInvalid,
    RecordNotFound,
    RelationInvalid,
    ResourceInvalid,
    ResourceNotFound,
    ResourceObservationNotFound,
    ReplayInvalid,
    ReplayUnsupported,
    SessionInvalid,
    SessionNotFound,
    StoreAlreadyInitialized,
    StoreBootstrapInvalid,
    StoreCompatibilityUnsupported,
    StorageFailure,
    TaskInvalid,
    TaskNotFound,
    TimeInvalid,
    WorkspaceInvalid,
    WorkspaceNotFound,
}

impl ErrorCode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::CanonicalEncodingInvalid => "canonical_encoding_invalid",
            Self::DigestInvalid => "digest_invalid",
            Self::EvidenceInvalid => "evidence_invalid",
            Self::EvidenceNotFound => "evidence_not_found",
            Self::IdentityInvalid => "identity_invalid",
            Self::ImmutableImportInvalid => "immutable_import_invalid",
            Self::IntegrityInvalid => "integrity_invalid",
            Self::KnowledgeInvalid => "knowledge_invalid",
            Self::KnowledgeNotFound => "knowledge_not_found",
            Self::MutationPostconditionFailed => "mutation_postcondition_failed",
            Self::CommitNotFound => "commit_not_found",
            Self::BranchHeadConflict => "branch_head_conflict",
            Self::BranchNotFound => "branch_not_found",
            Self::ClaimInvalid => "claim_invalid",
            Self::ClaimNotFound => "claim_not_found",
            Self::EntityNotFound => "entity_not_found",
            Self::EntityTransitionInvalid => "entity_transition_invalid",
            Self::GoalInvalid => "goal_invalid",
            Self::GoalNotFound => "goal_not_found",
            Self::PlanInvalid => "plan_invalid",
            Self::PlanNotFound => "plan_not_found",
            Self::ProjectBindingNotFound => "project_binding_not_found",
            Self::QueryInvalid => "query_invalid",
            Self::QueryUnsupported => "query_unsupported",
            Self::RecordInvalid => "record_invalid",
            Self::RecordNotFound => "record_not_found",
            Self::RelationInvalid => "relation_invalid",
            Self::ResourceInvalid => "resource_invalid",
            Self::ResourceNotFound => "resource_not_found",
            Self::ResourceObservationNotFound => "resource_observation_not_found",
            Self::ReplayInvalid => "replay_invalid",
            Self::ReplayUnsupported => "replay_unsupported",
            Self::SessionInvalid => "session_invalid",
            Self::SessionNotFound => "session_not_found",
            Self::StoreAlreadyInitialized => "store_already_initialized",
            Self::StoreBootstrapInvalid => "store_bootstrap_invalid",
            Self::StoreCompatibilityUnsupported => "store_compatibility_unsupported",
            Self::StorageFailure => "storage_failure",
            Self::TaskInvalid => "task_invalid",
            Self::TaskNotFound => "task_not_found",
            Self::TimeInvalid => "time_invalid",
            Self::WorkspaceInvalid => "workspace_invalid",
            Self::WorkspaceNotFound => "workspace_not_found",
        }
    }
}

impl fmt::Display for ErrorCode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum WorkVcsError {
    #[error("canonical encoding invalid: {0}")]
    CanonicalEncodingInvalid(String),

    #[error("digest invalid: {0}")]
    DigestInvalid(String),

    #[error("evidence invalid: {0}")]
    EvidenceInvalid(String),

    #[error("evidence not found: {0}")]
    EvidenceNotFound(String),

    #[error("identity invalid: {0}")]
    IdentityInvalid(String),

    #[error("immutable import fixed-point validation failed: {0}")]
    ImmutableImportInvalid(String),

    #[error("integrity invalid: {0}")]
    IntegrityInvalid(String),

    #[error("knowledge invalid: {0}")]
    KnowledgeInvalid(String),

    #[error("knowledge not found: {0}")]
    KnowledgeNotFound(String),

    #[error("mutation {operation} completed before result assertion failed: {message}")]
    MutationPostconditionFailed {
        operation: String,
        result: String,
        message: String,
    },

    #[error("commit not found: {0}")]
    CommitNotFound(String),

    #[error("branch head conflict: {0}")]
    BranchHeadConflict(String),

    #[error("branch not found: {0}")]
    BranchNotFound(String),

    #[error("claim invalid: {0}")]
    ClaimInvalid(String),

    #[error("claim not found: {0}")]
    ClaimNotFound(String),

    #[error("entity not found: {0}")]
    EntityNotFound(String),

    #[error("entity transition invalid: {0}")]
    EntityTransitionInvalid(String),

    #[error("goal invalid: {0}")]
    GoalInvalid(String),

    #[error("goal not found: {0}")]
    GoalNotFound(String),

    #[error("plan invalid: {0}")]
    PlanInvalid(String),

    #[error("plan not found: {0}")]
    PlanNotFound(String),

    #[error("project binding not found for {identity_kind} identity {identity} in {registry_path}")]
    ProjectBindingNotFound {
        identity_kind: String,
        identity: String,
        project_root: String,
        registry_path: String,
    },

    #[error("query invalid: {0}")]
    QueryInvalid(String),

    #[error("query unsupported: {0}")]
    QueryUnsupported(String),

    #[error("record invalid: {0}")]
    RecordInvalid(String),

    #[error("record not found: {0}")]
    RecordNotFound(String),

    #[error("relation invalid: {0}")]
    RelationInvalid(String),

    #[error("resource invalid: {0}")]
    ResourceInvalid(String),

    #[error("resource not found: {0}")]
    ResourceNotFound(String),

    #[error("resource observation not found: {0}")]
    ResourceObservationNotFound(String),

    #[error("replay invalid: {0}")]
    ReplayInvalid(String),

    #[error("replay unsupported: {0}")]
    ReplayUnsupported(String),

    #[error("session invalid: {0}")]
    SessionInvalid(String),

    #[error("session not found: {0}")]
    SessionNotFound(String),

    #[error("store already initialized: {0}")]
    StoreAlreadyInitialized(String),

    #[error("store bootstrap invalid: {0}")]
    StoreBootstrapInvalid(String),

    #[error("store compatibility unsupported: {0}")]
    StoreCompatibilityUnsupported(String),

    #[error("storage failure: {0}")]
    StorageFailure(String),

    #[error("task invalid: {0}")]
    TaskInvalid(String),

    #[error("task not found: {0}")]
    TaskNotFound(String),

    #[error("time invalid: {0}")]
    TimeInvalid(String),

    #[error("workspace invalid: {0}")]
    WorkspaceInvalid(String),

    #[error("workspace not found: {0}")]
    WorkspaceNotFound(String),
}

impl WorkVcsError {
    pub fn code(&self) -> ErrorCode {
        match self {
            Self::CanonicalEncodingInvalid(_) => ErrorCode::CanonicalEncodingInvalid,
            Self::DigestInvalid(_) => ErrorCode::DigestInvalid,
            Self::EvidenceInvalid(_) => ErrorCode::EvidenceInvalid,
            Self::EvidenceNotFound(_) => ErrorCode::EvidenceNotFound,
            Self::IdentityInvalid(_) => ErrorCode::IdentityInvalid,
            Self::ImmutableImportInvalid(_) => ErrorCode::ImmutableImportInvalid,
            Self::IntegrityInvalid(_) => ErrorCode::IntegrityInvalid,
            Self::KnowledgeInvalid(_) => ErrorCode::KnowledgeInvalid,
            Self::KnowledgeNotFound(_) => ErrorCode::KnowledgeNotFound,
            Self::MutationPostconditionFailed { .. } => ErrorCode::MutationPostconditionFailed,
            Self::CommitNotFound(_) => ErrorCode::CommitNotFound,
            Self::BranchHeadConflict(_) => ErrorCode::BranchHeadConflict,
            Self::BranchNotFound(_) => ErrorCode::BranchNotFound,
            Self::ClaimInvalid(_) => ErrorCode::ClaimInvalid,
            Self::ClaimNotFound(_) => ErrorCode::ClaimNotFound,
            Self::EntityNotFound(_) => ErrorCode::EntityNotFound,
            Self::EntityTransitionInvalid(_) => ErrorCode::EntityTransitionInvalid,
            Self::GoalInvalid(_) => ErrorCode::GoalInvalid,
            Self::GoalNotFound(_) => ErrorCode::GoalNotFound,
            Self::PlanInvalid(_) => ErrorCode::PlanInvalid,
            Self::PlanNotFound(_) => ErrorCode::PlanNotFound,
            Self::ProjectBindingNotFound { .. } => ErrorCode::ProjectBindingNotFound,
            Self::QueryInvalid(_) => ErrorCode::QueryInvalid,
            Self::QueryUnsupported(_) => ErrorCode::QueryUnsupported,
            Self::RecordInvalid(_) => ErrorCode::RecordInvalid,
            Self::RecordNotFound(_) => ErrorCode::RecordNotFound,
            Self::RelationInvalid(_) => ErrorCode::RelationInvalid,
            Self::ResourceInvalid(_) => ErrorCode::ResourceInvalid,
            Self::ResourceNotFound(_) => ErrorCode::ResourceNotFound,
            Self::ResourceObservationNotFound(_) => ErrorCode::ResourceObservationNotFound,
            Self::ReplayInvalid(_) => ErrorCode::ReplayInvalid,
            Self::ReplayUnsupported(_) => ErrorCode::ReplayUnsupported,
            Self::SessionInvalid(_) => ErrorCode::SessionInvalid,
            Self::SessionNotFound(_) => ErrorCode::SessionNotFound,
            Self::StoreAlreadyInitialized(_) => ErrorCode::StoreAlreadyInitialized,
            Self::StoreBootstrapInvalid(_) => ErrorCode::StoreBootstrapInvalid,
            Self::StoreCompatibilityUnsupported(_) => ErrorCode::StoreCompatibilityUnsupported,
            Self::StorageFailure(_) => ErrorCode::StorageFailure,
            Self::TaskInvalid(_) => ErrorCode::TaskInvalid,
            Self::TaskNotFound(_) => ErrorCode::TaskNotFound,
            Self::TimeInvalid(_) => ErrorCode::TimeInvalid,
            Self::WorkspaceInvalid(_) => ErrorCode::WorkspaceInvalid,
            Self::WorkspaceNotFound(_) => ErrorCode::WorkspaceNotFound,
        }
    }

    pub fn category(&self) -> ErrorCategory {
        match self {
            Self::CanonicalEncodingInvalid(_) | Self::DigestInvalid(_) => ErrorCategory::Canonical,
            Self::EvidenceInvalid(_) | Self::EvidenceNotFound(_) => ErrorCategory::Evidence,
            Self::IdentityInvalid(_) => ErrorCategory::Identity,
            Self::ImmutableImportInvalid(_) => ErrorCategory::Import,
            Self::IntegrityInvalid(_) => ErrorCategory::Integrity,
            Self::KnowledgeInvalid(_) | Self::KnowledgeNotFound(_) => ErrorCategory::Knowledge,
            Self::MutationPostconditionFailed { .. }
            | Self::BranchHeadConflict(_)
            | Self::BranchNotFound(_)
            | Self::EntityNotFound(_)
            | Self::EntityTransitionInvalid(_) => ErrorCategory::Mutation,
            Self::GoalInvalid(_) | Self::GoalNotFound(_) => ErrorCategory::Goal,
            Self::PlanInvalid(_) | Self::PlanNotFound(_) => ErrorCategory::Plan,
            Self::ProjectBindingNotFound { .. }
            | Self::QueryInvalid(_)
            | Self::QueryUnsupported(_) => ErrorCategory::Query,
            Self::RecordInvalid(_) | Self::RecordNotFound(_) => ErrorCategory::Record,
            Self::RelationInvalid(_) => ErrorCategory::Relation,
            Self::ResourceInvalid(_)
            | Self::ResourceNotFound(_)
            | Self::ResourceObservationNotFound(_) => ErrorCategory::Resource,
            Self::CommitNotFound(_) | Self::ReplayInvalid(_) | Self::ReplayUnsupported(_) => {
                ErrorCategory::Replay
            }
            Self::ClaimInvalid(_)
            | Self::ClaimNotFound(_)
            | Self::SessionInvalid(_)
            | Self::SessionNotFound(_) => ErrorCategory::Runtime,
            Self::StoreAlreadyInitialized(_)
            | Self::StoreBootstrapInvalid(_)
            | Self::StoreCompatibilityUnsupported(_) => ErrorCategory::Store,
            Self::StorageFailure(_) => ErrorCategory::Storage,
            Self::TaskInvalid(_) | Self::TaskNotFound(_) => ErrorCategory::Task,
            Self::TimeInvalid(_) => ErrorCategory::Time,
            Self::WorkspaceInvalid(_) | Self::WorkspaceNotFound(_) => ErrorCategory::Workspace,
        }
    }

    pub fn retryable(&self) -> bool {
        matches!(self, Self::BranchHeadConflict(_))
    }
}

pub(crate) fn storage_error(error: rusqlite::Error) -> WorkVcsError {
    WorkVcsError::StorageFailure(format!("sqlite operation failed: {error}"))
}

#[cfg(test)]
mod tests {
    use super::{ErrorCategory, ErrorCode, WorkVcsError};

    #[test]
    fn error_codes_render_stable_lower_snake_case() {
        assert_eq!(ErrorCode::QueryInvalid.as_str(), "query_invalid");
        assert_eq!(
            ErrorCode::ProjectBindingNotFound.as_str(),
            "project_binding_not_found"
        );
        assert_eq!(
            ErrorCode::BranchHeadConflict.as_str(),
            "branch_head_conflict"
        );
        assert_eq!(
            ErrorCode::ResourceObservationNotFound.as_str(),
            "resource_observation_not_found"
        );
        assert_eq!(
            ErrorCode::StoreCompatibilityUnsupported.as_str(),
            "store_compatibility_unsupported"
        );
        assert_eq!(
            ErrorCode::MutationPostconditionFailed.as_str(),
            "mutation_postcondition_failed"
        );
    }

    #[test]
    fn error_categories_render_stable_lower_snake_case() {
        assert_eq!(ErrorCategory::Query.as_str(), "query");
        assert_eq!(ErrorCategory::Runtime.as_str(), "runtime");
        assert_eq!(ErrorCategory::Resource.as_str(), "resource");
    }

    #[test]
    fn workvcs_error_exposes_code_category_and_retryability() {
        let query = WorkVcsError::QueryInvalid("missing selector".to_owned());
        assert_eq!(query.code().as_str(), "query_invalid");
        assert_eq!(query.category().as_str(), "query");
        assert!(!query.retryable());

        let conflict = WorkVcsError::BranchHeadConflict("moved head".to_owned());
        assert_eq!(conflict.code().as_str(), "branch_head_conflict");
        assert_eq!(conflict.category().as_str(), "mutation");
        assert!(conflict.retryable());

        let postcondition = WorkVcsError::MutationPostconditionFailed {
            operation: "claim.next".to_owned(),
            result: "selected=true\n".to_owned(),
            message: "expected selected false".to_owned(),
        };
        assert_eq!(
            postcondition.code().as_str(),
            "mutation_postcondition_failed"
        );
        assert_eq!(postcondition.category().as_str(), "mutation");
        assert!(!postcondition.retryable());
    }
}
