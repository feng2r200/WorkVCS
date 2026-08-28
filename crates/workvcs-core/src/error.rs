use thiserror::Error;

pub type Result<T> = std::result::Result<T, WorkVcsError>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErrorCategory {
    Canonical,
    Identity,
    Import,
    Store,
    Storage,
    Time,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErrorCode {
    CanonicalEncodingInvalid,
    DigestInvalid,
    IdentityInvalid,
    ImmutableImportInvalid,
    StoreAlreadyInitialized,
    StoreBootstrapInvalid,
    StoreCompatibilityUnsupported,
    StorageFailure,
    TimeInvalid,
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
}

impl WorkVcsError {
    pub fn code(&self) -> ErrorCode {
        match self {
            Self::CanonicalEncodingInvalid(_) => ErrorCode::CanonicalEncodingInvalid,
            Self::DigestInvalid(_) => ErrorCode::DigestInvalid,
            Self::IdentityInvalid(_) => ErrorCode::IdentityInvalid,
            Self::ImmutableImportInvalid(_) => ErrorCode::ImmutableImportInvalid,
            Self::StoreAlreadyInitialized(_) => ErrorCode::StoreAlreadyInitialized,
            Self::StoreBootstrapInvalid(_) => ErrorCode::StoreBootstrapInvalid,
            Self::StoreCompatibilityUnsupported(_) => ErrorCode::StoreCompatibilityUnsupported,
            Self::StorageFailure(_) => ErrorCode::StorageFailure,
            Self::TimeInvalid(_) => ErrorCode::TimeInvalid,
        }
    }

    pub fn category(&self) -> ErrorCategory {
        match self {
            Self::CanonicalEncodingInvalid(_) | Self::DigestInvalid(_) => ErrorCategory::Canonical,
            Self::IdentityInvalid(_) => ErrorCategory::Identity,
            Self::ImmutableImportInvalid(_) => ErrorCategory::Import,
            Self::StoreAlreadyInitialized(_)
            | Self::StoreBootstrapInvalid(_)
            | Self::StoreCompatibilityUnsupported(_) => ErrorCategory::Store,
            Self::StorageFailure(_) => ErrorCategory::Storage,
            Self::TimeInvalid(_) => ErrorCategory::Time,
        }
    }

    pub fn retryable(&self) -> bool {
        false
    }
}

pub(crate) fn storage_error(error: rusqlite::Error) -> WorkVcsError {
    WorkVcsError::StorageFailure(format!("sqlite operation failed: {error}"))
}
