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
    Mutation,
    Plan,
    Query,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErrorCode {
    CanonicalEncodingInvalid,
    DigestInvalid,
    EvidenceInvalid,
    EvidenceNotFound,
    IdentityInvalid,
    ImmutableImportInvalid,
    IntegrityInvalid,
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
    QueryInvalid,
    QueryUnsupported,
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

    #[error("query invalid: {0}")]
    QueryInvalid(String),

    #[error("query unsupported: {0}")]
    QueryUnsupported(String),

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
            Self::QueryInvalid(_) => ErrorCode::QueryInvalid,
            Self::QueryUnsupported(_) => ErrorCode::QueryUnsupported,
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
            Self::BranchHeadConflict(_)
            | Self::BranchNotFound(_)
            | Self::EntityNotFound(_)
            | Self::EntityTransitionInvalid(_) => ErrorCategory::Mutation,
            Self::GoalInvalid(_) | Self::GoalNotFound(_) => ErrorCategory::Goal,
            Self::PlanInvalid(_) | Self::PlanNotFound(_) => ErrorCategory::Plan,
            Self::QueryInvalid(_) | Self::QueryUnsupported(_) => ErrorCategory::Query,
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
