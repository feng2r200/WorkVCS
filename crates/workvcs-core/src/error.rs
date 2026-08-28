use thiserror::Error;

pub type Result<T> = std::result::Result<T, WorkVcsError>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErrorCategory {
    Canonical,
    Identity,
    Import,
    Replay,
    Store,
    Storage,
    Time,
    Workspace,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErrorCode {
    CanonicalEncodingInvalid,
    DigestInvalid,
    IdentityInvalid,
    ImmutableImportInvalid,
    CommitNotFound,
    ReplayInvalid,
    ReplayUnsupported,
    StoreAlreadyInitialized,
    StoreBootstrapInvalid,
    StoreCompatibilityUnsupported,
    StorageFailure,
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

    #[error("identity invalid: {0}")]
    IdentityInvalid(String),

    #[error("immutable import fixed-point validation failed: {0}")]
    ImmutableImportInvalid(String),

    #[error("commit not found: {0}")]
    CommitNotFound(String),

    #[error("replay invalid: {0}")]
    ReplayInvalid(String),

    #[error("replay unsupported: {0}")]
    ReplayUnsupported(String),

    #[error("store already initialized: {0}")]
    StoreAlreadyInitialized(String),

    #[error("store bootstrap invalid: {0}")]
    StoreBootstrapInvalid(String),

    #[error("store compatibility unsupported: {0}")]
    StoreCompatibilityUnsupported(String),

    #[error("storage failure: {0}")]
    StorageFailure(String),

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
            Self::IdentityInvalid(_) => ErrorCode::IdentityInvalid,
            Self::ImmutableImportInvalid(_) => ErrorCode::ImmutableImportInvalid,
            Self::CommitNotFound(_) => ErrorCode::CommitNotFound,
            Self::ReplayInvalid(_) => ErrorCode::ReplayInvalid,
            Self::ReplayUnsupported(_) => ErrorCode::ReplayUnsupported,
            Self::StoreAlreadyInitialized(_) => ErrorCode::StoreAlreadyInitialized,
            Self::StoreBootstrapInvalid(_) => ErrorCode::StoreBootstrapInvalid,
            Self::StoreCompatibilityUnsupported(_) => ErrorCode::StoreCompatibilityUnsupported,
            Self::StorageFailure(_) => ErrorCode::StorageFailure,
            Self::TimeInvalid(_) => ErrorCode::TimeInvalid,
            Self::WorkspaceInvalid(_) => ErrorCode::WorkspaceInvalid,
            Self::WorkspaceNotFound(_) => ErrorCode::WorkspaceNotFound,
        }
    }

    pub fn category(&self) -> ErrorCategory {
        match self {
            Self::CanonicalEncodingInvalid(_) | Self::DigestInvalid(_) => ErrorCategory::Canonical,
            Self::IdentityInvalid(_) => ErrorCategory::Identity,
            Self::ImmutableImportInvalid(_) => ErrorCategory::Import,
            Self::CommitNotFound(_) | Self::ReplayInvalid(_) | Self::ReplayUnsupported(_) => {
                ErrorCategory::Replay
            }
            Self::StoreAlreadyInitialized(_)
            | Self::StoreBootstrapInvalid(_)
            | Self::StoreCompatibilityUnsupported(_) => ErrorCategory::Store,
            Self::StorageFailure(_) => ErrorCategory::Storage,
            Self::TimeInvalid(_) => ErrorCategory::Time,
            Self::WorkspaceInvalid(_) | Self::WorkspaceNotFound(_) => ErrorCategory::Workspace,
        }
    }

    pub fn retryable(&self) -> bool {
        false
    }
}

pub(crate) fn storage_error(error: rusqlite::Error) -> WorkVcsError {
    WorkVcsError::StorageFailure(format!("sqlite operation failed: {error}"))
}
