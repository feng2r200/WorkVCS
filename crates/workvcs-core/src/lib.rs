pub mod canonical;
pub mod engine;
pub mod error;
mod history;
pub mod identity;
pub mod store;

pub use canonical::{
    CanonicalValue, WorkState, canonical_bytes, content_object_digest, entity_version_digest,
    parse_canonical_json, relation_version_digest, validate_import_fixed_point,
    work_state_mapping_digest,
};
pub use engine::Engine;
pub use error::{ErrorCategory, ErrorCode, Result, WorkVcsError};
pub use history::{WorkspaceInfo, WorkspaceInitOptions};
pub use identity::{
    BranchId, ChangeSetId, CommitId, Digest, EntityId, EntityVersionId, EventId, RelationId,
    RelationVersionId, StoreId, WorkspaceId,
};
pub use store::{StoreInfo, StoreInitOptions, StoreManifest};
