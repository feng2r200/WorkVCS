use super::task::{
    ACCEPTANCE_CRITERION_ENTITY_KIND, TASK_ENTITY_KIND, VERIFICATION_REQUIREMENT_ENTITY_KIND,
};
use crate::canonical::{
    CanonicalValue, WorkState, canonical_bytes, content_object_digest, entity_version_digest,
    parse_canonical_json, relation_version_digest, work_state_mapping_digest,
};
use crate::error::{Result, WorkVcsError, storage_error};
use crate::identity::{
    BranchId, ChangeSetId, CheckpointId, CommitId, Digest, EntityId, EntityVersionId, EventId,
    ExposureId, ExposureTransitionId, ImportId, KnowledgeSpaceId, OperationId, RelationId,
    RelationVersionId, SessionId, StoreId, WorkspaceId,
};
use crate::store::{StoreConnection, StoreInfo, StoreManifest, current_epoch_micros};
use rusqlite::{OptionalExtension, Transaction, TransactionBehavior, params};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use uuid::Uuid;

const BUNDLE_EXPORT_MANIFEST_PROFILE: &str = "workvcs-local-export-manifest-v1";
const BUNDLE_EXPORT_MANIFEST_VERSION: i64 = 1;
const BUNDLE_PAYLOAD_INDEX_PROFILE: &str = "workvcs-local-payload-index-v1";
const BUNDLE_PAYLOAD_INDEX_VERSION: i64 = 1;
const BUNDLE_PAYLOAD_MEDIA_TYPE: &str = "application/json";
const BUNDLE_IMPORT_PROFILE: &str = "workvcs-local-payload-directory-v1";
const RELATION_OBJECT_KIND: &str = "relation";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BundleExportOptions {
    commit_id: CommitId,
}

impl BundleExportOptions {
    pub fn for_commit(commit_id: CommitId) -> Self {
        Self { commit_id }
    }

    pub fn commit_id(self) -> CommitId {
        self.commit_id
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BundlePayloadExportOptions {
    commit_id: CommitId,
}

impl BundlePayloadExportOptions {
    pub fn for_commit(commit_id: CommitId) -> Self {
        Self { commit_id }
    }

    pub fn commit_id(self) -> CommitId {
        self.commit_id
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BundlePayloadValidationOptions {
    commit_id: CommitId,
    manifest_bytes: Vec<u8>,
    payload_index_bytes: Vec<u8>,
    payloads: Vec<BundlePayloadInput>,
}

impl BundlePayloadValidationOptions {
    pub fn from_parts(
        commit_id: CommitId,
        manifest_bytes: impl Into<Vec<u8>>,
        payload_index_bytes: impl Into<Vec<u8>>,
        payloads: Vec<BundlePayloadInput>,
    ) -> Result<Self> {
        let manifest_bytes = manifest_bytes.into();
        if manifest_bytes.is_empty() {
            return Err(WorkVcsError::QueryInvalid(
                "bundle manifest bytes cannot be empty".to_owned(),
            ));
        }
        let payload_index_bytes = payload_index_bytes.into();
        if payload_index_bytes.is_empty() {
            return Err(WorkVcsError::QueryInvalid(
                "bundle payload index bytes cannot be empty".to_owned(),
            ));
        }
        Ok(Self {
            commit_id,
            manifest_bytes,
            payload_index_bytes,
            payloads,
        })
    }

    pub fn commit_id(&self) -> CommitId {
        self.commit_id
    }

    fn manifest_bytes(&self) -> &[u8] {
        &self.manifest_bytes
    }

    fn payload_index_bytes(&self) -> &[u8] {
        &self.payload_index_bytes
    }

    fn payloads(&self) -> &[BundlePayloadInput] {
        &self.payloads
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BundleImportPreflightOptions {
    manifest_bytes: Vec<u8>,
    payload_index_bytes: Vec<u8>,
    payloads: Vec<BundlePayloadInput>,
}

impl BundleImportPreflightOptions {
    pub fn from_parts(
        manifest_bytes: impl Into<Vec<u8>>,
        payload_index_bytes: impl Into<Vec<u8>>,
        payloads: Vec<BundlePayloadInput>,
    ) -> Result<Self> {
        let manifest_bytes = manifest_bytes.into();
        if manifest_bytes.is_empty() {
            return Err(WorkVcsError::QueryInvalid(
                "bundle manifest bytes cannot be empty".to_owned(),
            ));
        }
        let payload_index_bytes = payload_index_bytes.into();
        if payload_index_bytes.is_empty() {
            return Err(WorkVcsError::QueryInvalid(
                "bundle payload index bytes cannot be empty".to_owned(),
            ));
        }
        Ok(Self {
            manifest_bytes,
            payload_index_bytes,
            payloads,
        })
    }

    fn manifest_bytes(&self) -> &[u8] {
        &self.manifest_bytes
    }

    fn payload_index_bytes(&self) -> &[u8] {
        &self.payload_index_bytes
    }

    fn payloads(&self) -> &[BundlePayloadInput] {
        &self.payloads
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BundleImportAttemptOptions {
    manifest_bytes: Vec<u8>,
    payload_index_bytes: Vec<u8>,
    payloads: Vec<BundlePayloadInput>,
    origin_session_id: Option<SessionId>,
}

impl BundleImportAttemptOptions {
    pub fn from_parts(
        manifest_bytes: impl Into<Vec<u8>>,
        payload_index_bytes: impl Into<Vec<u8>>,
        payloads: Vec<BundlePayloadInput>,
    ) -> Result<Self> {
        let manifest_bytes = manifest_bytes.into();
        if manifest_bytes.is_empty() {
            return Err(WorkVcsError::QueryInvalid(
                "bundle manifest bytes cannot be empty".to_owned(),
            ));
        }
        let payload_index_bytes = payload_index_bytes.into();
        if payload_index_bytes.is_empty() {
            return Err(WorkVcsError::QueryInvalid(
                "bundle payload index bytes cannot be empty".to_owned(),
            ));
        }
        Ok(Self {
            manifest_bytes,
            payload_index_bytes,
            payloads,
            origin_session_id: None,
        })
    }

    pub fn with_origin_session_id(mut self, origin_session_id: SessionId) -> Self {
        self.origin_session_id = Some(origin_session_id);
        self
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BundleImportApplyOptions {
    manifest_bytes: Vec<u8>,
    payload_index_bytes: Vec<u8>,
    payloads: Vec<BundlePayloadInput>,
    origin_session_id: Option<SessionId>,
}

impl BundleImportApplyOptions {
    pub fn from_parts(
        manifest_bytes: impl Into<Vec<u8>>,
        payload_index_bytes: impl Into<Vec<u8>>,
        payloads: Vec<BundlePayloadInput>,
    ) -> Result<Self> {
        let manifest_bytes = manifest_bytes.into();
        if manifest_bytes.is_empty() {
            return Err(WorkVcsError::QueryInvalid(
                "bundle manifest bytes cannot be empty".to_owned(),
            ));
        }
        let payload_index_bytes = payload_index_bytes.into();
        if payload_index_bytes.is_empty() {
            return Err(WorkVcsError::QueryInvalid(
                "bundle payload index bytes cannot be empty".to_owned(),
            ));
        }
        Ok(Self {
            manifest_bytes,
            payload_index_bytes,
            payloads,
            origin_session_id: None,
        })
    }

    pub fn with_origin_session_id(mut self, origin_session_id: SessionId) -> Self {
        self.origin_session_id = Some(origin_session_id);
        self
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BundleExportManifest {
    pub manifest_profile: String,
    pub manifest_version: i64,
    pub store_id: StoreId,
    pub workspace_id: WorkspaceId,
    pub commit_id: CommitId,
    pub state_digest: Digest,
    pub manifest_digest: Digest,
    pub manifest_size_bytes: i64,
    pub entity_count: usize,
    pub relation_count: usize,
    pub commit_count: usize,
    pub exported_branch_heads: Vec<BundleBranchHeadRef>,
    pub entity_versions: Vec<BundleEntityVersionRef>,
    pub acceptance_criterion_identities: Vec<BundleAcceptanceCriterionIdentityRef>,
    pub verification_requirement_identities: Vec<BundleVerificationRequirementIdentityRef>,
    pub relation_versions: Vec<BundleRelationVersionRef>,
    pub knowledge_spaces: Vec<BundleKnowledgeSpaceRef>,
    pub knowledge_exposures: Vec<BundleKnowledgeExposureRef>,
    pub knowledge_exposure_local_sources: Vec<BundleKnowledgeExposureLocalSourceRef>,
    pub knowledge_exposure_transitions: Vec<BundleKnowledgeExposureTransitionRef>,
    pub knowledge_exposure_source_statuses: Vec<BundleKnowledgeExposureSourceStatusRef>,
    pub entity_membership_changes: Vec<BundleEntityMembershipChangeRef>,
    pub relation_membership_changes: Vec<BundleRelationMembershipChangeRef>,
    pub checkpoint_candidates: Vec<BundleCheckpointCandidate>,
    pub manifest: CanonicalValue,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BundleManifestValidationOptions {
    commit_id: CommitId,
    manifest_bytes: Vec<u8>,
}

impl BundleManifestValidationOptions {
    pub fn from_bytes(commit_id: CommitId, manifest_bytes: impl Into<Vec<u8>>) -> Result<Self> {
        let manifest_bytes = manifest_bytes.into();
        if manifest_bytes.is_empty() {
            return Err(WorkVcsError::QueryInvalid(
                "bundle manifest bytes cannot be empty".to_owned(),
            ));
        }
        Ok(Self {
            commit_id,
            manifest_bytes,
        })
    }

    pub fn commit_id(&self) -> CommitId {
        self.commit_id
    }

    fn manifest_bytes(&self) -> &[u8] {
        &self.manifest_bytes
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BundleManifestValidationResult {
    pub commit_id: CommitId,
    pub valid: bool,
    pub expected_manifest_digest: Digest,
    pub actual_manifest_digest: Digest,
    pub actual_manifest_size_bytes: i64,
    pub problem: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BundlePayloadExport {
    pub manifest: BundleExportManifest,
    pub manifest_bytes: Vec<u8>,
    pub payload_index: CanonicalValue,
    pub payload_index_bytes: Vec<u8>,
    pub payload_index_digest: Digest,
    pub payload_index_size_bytes: i64,
    pub payload_files: Vec<BundlePayloadFile>,
    pub payload_references: Vec<BundlePayloadReference>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BundlePayloadFile {
    pub relative_path: String,
    pub content_digest: Digest,
    pub size_bytes: i64,
    pub media_type: String,
    pub bytes: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BundlePayloadReference {
    pub role: String,
    pub relative_path: String,
    pub content_digest: Digest,
    pub size_bytes: i64,
    pub owner: CanonicalValue,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BundlePayloadInput {
    pub relative_path: String,
    pub bytes: Vec<u8>,
}

impl BundlePayloadInput {
    pub fn new(relative_path: impl Into<String>, bytes: impl Into<Vec<u8>>) -> Result<Self> {
        let relative_path = relative_path.into();
        validate_payload_relative_path(&relative_path)?;
        let bytes = bytes.into();
        if bytes.is_empty() {
            return Err(WorkVcsError::QueryInvalid(
                "bundle payload bytes cannot be empty".to_owned(),
            ));
        }
        Ok(Self {
            relative_path,
            bytes,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BundlePayloadValidationResult {
    pub commit_id: CommitId,
    pub valid: bool,
    pub expected_manifest_digest: Digest,
    pub actual_manifest_digest: Digest,
    pub expected_payload_index_digest: Digest,
    pub actual_payload_index_digest: Digest,
    pub expected_payload_files: usize,
    pub actual_payload_files: usize,
    pub expected_payload_references: usize,
    pub problem: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BundleImportPreflightResult {
    pub valid: bool,
    pub format_compatible: bool,
    pub source_store_id: Option<StoreId>,
    pub target_workspace_id: Option<WorkspaceId>,
    pub target_commit_id: Option<CommitId>,
    pub target_state_digest: Option<Digest>,
    pub source_store_relation: String,
    pub incoming_commit_present: bool,
    pub import_required: bool,
    pub can_apply: bool,
    pub action: String,
    pub manifest_digest: Digest,
    pub payload_index_digest: Digest,
    pub payload_files: usize,
    pub payload_references: usize,
    pub exported_branch_heads: usize,
    pub branch_heads_already_present: usize,
    pub branch_heads_missing: usize,
    pub branch_heads_fast_forward: usize,
    pub branch_heads_diverged: usize,
    pub problem: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BundleImportAttemptResult {
    pub import_id: Option<ImportId>,
    pub recorded: bool,
    pub bundle_digest: Digest,
    pub import_profile: String,
    pub started_at_us: Option<i64>,
    pub completed_at_us: Option<i64>,
    pub outcome: String,
    pub preflight: BundleImportPreflightResult,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BundleImportApplyResult {
    pub import_id: Option<ImportId>,
    pub applied: bool,
    pub bundle_digest: Digest,
    pub import_profile: String,
    pub started_at_us: Option<i64>,
    pub completed_at_us: Option<i64>,
    pub outcome: String,
    pub preflight: BundleImportPreflightResult,
    pub imported_commits: usize,
    pub imported_entity_versions: usize,
    pub imported_acceptance_criterion_identities: usize,
    pub imported_verification_requirement_identities: usize,
    pub imported_relation_versions: usize,
    pub updated_branch_heads: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BundleImportAttemptListOptions {
    limit: usize,
    source_store_id: Option<StoreId>,
    bundle_digest: Option<Digest>,
}

impl BundleImportAttemptListOptions {
    pub fn new() -> Self {
        Self {
            limit: 50,
            source_store_id: None,
            bundle_digest: None,
        }
    }

    pub fn with_limit(mut self, limit: usize) -> Result<Self> {
        if limit == 0 {
            return Err(WorkVcsError::QueryInvalid(
                "bundle import attempt list limit must be positive".to_owned(),
            ));
        }
        self.limit = limit;
        Ok(self)
    }

    pub fn with_source_store_id(mut self, source_store_id: StoreId) -> Self {
        self.source_store_id = Some(source_store_id);
        self
    }

    pub fn with_bundle_digest(mut self, bundle_digest: Digest) -> Self {
        self.bundle_digest = Some(bundle_digest);
        self
    }

    fn limit(self) -> usize {
        self.limit
    }

    fn source_store_id(self) -> Option<StoreId> {
        self.source_store_id
    }

    fn bundle_digest(self) -> Option<Digest> {
        self.bundle_digest
    }
}

impl Default for BundleImportAttemptListOptions {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BundleImportAttemptListResult {
    pub attempts: Vec<BundleImportAttemptSnapshot>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BundleImportAttemptSnapshot {
    pub import_id: ImportId,
    pub source_store_id: StoreId,
    pub bundle_digest: Digest,
    pub import_profile: String,
    pub origin_session_id: Option<SessionId>,
    pub started_at_us: i64,
    pub outcome: Option<BundleImportAttemptOutcomeSnapshot>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BundleImportAttemptOutcomeSnapshot {
    pub outcome: String,
    pub completed_at_us: i64,
    pub detail: CanonicalValue,
    pub detail_digest: Digest,
    pub detail_size_bytes: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BundleCheckpointCandidate {
    pub checkpoint_id: CheckpointId,
    pub content_digest: Digest,
    pub content_size_bytes: i64,
    pub checkpoint_format_version: i64,
    pub media_type: Option<String>,
    pub usability_state: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BundleBranchHeadRef {
    pub workspace_id: WorkspaceId,
    pub branch_id: BranchId,
    pub name: String,
    pub head_commit_id: CommitId,
    pub head_state_digest: Digest,
    pub lifecycle_state: String,
    pub created_at_us: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BundleEntityVersionRef {
    pub entity_id: EntityId,
    pub entity_version_id: EntityVersionId,
    pub entity_kind: String,
    pub state_schema_version: i64,
    pub state_digest: Digest,
    pub state_json_digest: Digest,
    pub state_json_size_bytes: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BundleAcceptanceCriterionIdentityRef {
    pub entity_id: EntityId,
    pub owner_entity_id: EntityId,
    pub local_key: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BundleVerificationRequirementIdentityRef {
    pub entity_id: EntityId,
    pub owner_entity_id: EntityId,
    pub local_key: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BundleRelationVersionRef {
    pub relation_id: RelationId,
    pub relation_version_id: RelationVersionId,
    pub relation_type: String,
    pub source_object_id: String,
    pub target_object_id: String,
    pub relation_discriminator: String,
    pub state_schema_version: i64,
    pub state_digest: Digest,
    pub metadata_json_digest: Digest,
    pub metadata_json_size_bytes: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BundleKnowledgeSpaceRef {
    pub knowledge_space_id: KnowledgeSpaceId,
    pub name: String,
    pub created_at_us: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BundleKnowledgeExposureRef {
    pub exposure_id: ExposureId,
    pub knowledge_space_id: KnowledgeSpaceId,
    pub created_at_us: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BundleKnowledgeExposureLocalSourceRef {
    pub exposure_id: ExposureId,
    pub source_workspace_id: WorkspaceId,
    pub source_knowledge_entity_id: EntityId,
    pub source_knowledge_entity_version_id: EntityVersionId,
    pub source_knowledge_state_digest: Digest,
    pub source_knowledge_state_json_digest: Digest,
    pub source_knowledge_state_json_size_bytes: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BundleKnowledgeExposureTransitionRef {
    pub exposure_id: ExposureId,
    pub transition_id: ExposureTransitionId,
    pub previous_transition_id: Option<ExposureTransitionId>,
    pub lifecycle_status: String,
    pub changed_at_us: i64,
    pub event_id: Option<EventId>,
    pub detail_digest: Digest,
    pub detail_size_bytes: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BundleKnowledgeExposureSourceStatusRef {
    pub exposure_id: ExposureId,
    pub source_status: String,
    pub checked_at_us: i64,
    pub detail_digest: Digest,
    pub detail_size_bytes: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BundleEntityMembershipChangeRef {
    pub changeset_id: ChangeSetId,
    pub operation_id: OperationId,
    pub ordinal: i64,
    pub entity_id: EntityId,
    pub before_entity_version_id: Option<EntityVersionId>,
    pub after_entity_version_id: Option<EntityVersionId>,
    pub field_delta_digest: Digest,
    pub field_delta_size_bytes: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BundleRelationMembershipChangeRef {
    pub changeset_id: ChangeSetId,
    pub operation_id: OperationId,
    pub ordinal: i64,
    pub relation_id: RelationId,
    pub before_relation_version_id: Option<RelationVersionId>,
    pub after_relation_version_id: Option<RelationVersionId>,
    pub field_delta_digest: Digest,
    pub field_delta_size_bytes: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct BundleCommitRef {
    workspace_id: WorkspaceId,
    commit_id: CommitId,
    changeset_id: ChangeSetId,
    commit_kind: String,
    state_digest: Digest,
    committed_at_us: i64,
    operation_type: String,
    operation_schema_version: i64,
    changeset_created_at_us: i64,
    operation_payload_digest: Digest,
    rationale_digest: Digest,
    change_operation_count: i64,
    parents: Vec<BundleCommitParentRef>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct BundleCommitParentRef {
    parent_ordinal: i64,
    parent_role: String,
    parent_commit_id: CommitId,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct BundlePayloadCandidate {
    role: String,
    owner: CanonicalValue,
    bytes: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct BundleManifestSummary {
    source_store_id: StoreId,
    store_format_version: i64,
    schema_version: i64,
    object_store_format_version: i64,
    id_scheme: String,
    digest_algorithm: String,
    canonical_json_profile: String,
    workspace_id: WorkspaceId,
    commit_id: CommitId,
    state_digest: Digest,
    commits: Vec<BundleCommitSummary>,
    exported_branch_heads: Vec<BundleBranchHeadSummary>,
    same_store_apply_supported: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct BundleCommitSummary {
    commit_id: CommitId,
    state_digest: Digest,
    parent_commit_ids: Vec<CommitId>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct BundleBranchHeadSummary {
    workspace_id: WorkspaceId,
    branch_id: BranchId,
    name: String,
    head_commit_id: CommitId,
    head_state_digest: Digest,
    lifecycle_state: String,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct BundleBranchPreflightSummary {
    exported_branch_heads: usize,
    already_present: usize,
    missing: usize,
    fast_forward: usize,
    diverged: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct BundlePayloadIndexSummary {
    manifest_digest: Digest,
    manifest_size_bytes: i64,
    workspace_id: WorkspaceId,
    commit_id: CommitId,
    state_digest: Digest,
    payloads: Vec<BundlePayloadFileRef>,
    reference_count: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct BundlePayloadFileRef {
    relative_path: String,
    content_digest: Digest,
    size_bytes: i64,
    media_type: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct BundleSameStoreApplyDocument {
    source_store_id: StoreId,
    workspace_id: WorkspaceId,
    commit_id: CommitId,
    state_digest: Digest,
    commits: Vec<BundleCommitRef>,
    exported_branch_heads: Vec<BundleBranchHeadSummary>,
    entity_versions: Vec<BundleEntityVersionRef>,
    acceptance_criterion_identities: Vec<BundleAcceptanceCriterionIdentityRef>,
    verification_requirement_identities: Vec<BundleVerificationRequirementIdentityRef>,
    relation_versions: Vec<BundleRelationVersionRef>,
    entity_membership_changes: Vec<BundleEntityMembershipChangeRef>,
    relation_membership_changes: Vec<BundleRelationMembershipChangeRef>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct BundlePayloadLookupEntry {
    relative_path: String,
    content_digest: Digest,
    size_bytes: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct BundlePayloadLookup {
    by_role_owner: BTreeMap<(String, Vec<u8>), BundlePayloadLookupEntry>,
    bytes_by_path: BTreeMap<String, Vec<u8>>,
}

struct ImportAttemptOutcomeInsert<'a> {
    import_id: ImportId,
    source_store_id: StoreId,
    bundle_digest: Digest,
    import_profile: &'a str,
    origin_session_id: Option<SessionId>,
    now_us: i64,
    outcome: &'a str,
    detail_json: &'a str,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct CheckedBundleDirectory {
    manifest: BundleManifestSummary,
    payload_index: BundlePayloadIndexSummary,
}

pub(crate) fn export_bundle_manifest(
    connection: &StoreConnection,
    store_info: &StoreInfo,
    options: BundleExportOptions,
) -> Result<BundleExportManifest> {
    connection.verify_foreign_keys()?;
    let replayed = super::state_at(connection, options.commit_id())?;
    let replayed_state_digest = work_state_mapping_digest(&replayed.state);
    if replayed_state_digest != replayed.state_digest {
        return Err(WorkVcsError::ReplayInvalid(format!(
            "commit {} replayed WorkState digest does not match export target",
            replayed.commit_id
        )));
    }

    let mut commits = commit_closure_refs(connection, options.commit_id())?;
    for commit in &commits {
        if commit.workspace_id != replayed.workspace_id {
            return Err(WorkVcsError::QueryInvalid(format!(
                "commit {} belongs to workspace {}, not export workspace {}",
                commit.commit_id, commit.workspace_id, replayed.workspace_id
            )));
        }
    }
    commits.sort_by(|left, right| {
        left.committed_at_us
            .cmp(&right.committed_at_us)
            .then_with(|| left.commit_id.cmp(&right.commit_id))
    });
    let exported_branch_heads =
        exported_branch_head_refs(connection, replayed.workspace_id, &commits)?;

    let entity_membership_changes = entity_membership_change_refs(connection, &commits)?;
    let relation_membership_changes = relation_membership_change_refs(connection, &commits)?;
    let entity_versions = entity_version_closure_refs(
        connection,
        replayed.workspace_id,
        &replayed.state,
        &entity_membership_changes,
    )?;
    let acceptance_criterion_identities =
        acceptance_criterion_identity_refs(connection, &entity_versions)?;
    let verification_requirement_identities =
        verification_requirement_identity_refs(connection, &entity_versions)?;
    let relation_versions = relation_version_closure_refs(
        connection,
        replayed.workspace_id,
        &replayed.state,
        &relation_membership_changes,
    )?;
    let knowledge_exposures = knowledge_exposure_closure_refs(connection, &relation_versions)?;
    let knowledge_spaces = knowledge_space_closure_refs(connection, &knowledge_exposures)?;
    let knowledge_exposure_local_sources =
        knowledge_exposure_local_source_refs(connection, &knowledge_exposures)?;
    let knowledge_exposure_transitions =
        knowledge_exposure_transition_refs(connection, &knowledge_exposures)?;
    let knowledge_exposure_source_statuses =
        knowledge_exposure_source_status_refs(connection, &knowledge_exposures)?;

    let mut checkpoint_candidates = super::checkpoints(
        connection,
        super::CheckpointListOptions::for_commit(options.commit_id()),
    )?
    .checkpoints
    .into_iter()
    .map(|checkpoint| BundleCheckpointCandidate {
        checkpoint_id: checkpoint.checkpoint_id,
        content_digest: checkpoint.content_digest,
        content_size_bytes: checkpoint.content_size_bytes,
        checkpoint_format_version: checkpoint.checkpoint_format_version,
        media_type: checkpoint.media_type,
        usability_state: checkpoint.usability_state,
    })
    .collect::<Vec<_>>();
    checkpoint_candidates.sort_by_key(|checkpoint| checkpoint.checkpoint_id.raw_bytes());

    let manifest = manifest_value(BundleManifestValueInput {
        store_info,
        replayed: &replayed,
        commits: &commits,
        exported_branch_heads: &exported_branch_heads,
        entity_versions: &entity_versions,
        acceptance_criterion_identities: &acceptance_criterion_identities,
        verification_requirement_identities: &verification_requirement_identities,
        relation_versions: &relation_versions,
        knowledge_spaces: &knowledge_spaces,
        knowledge_exposures: &knowledge_exposures,
        knowledge_exposure_local_sources: &knowledge_exposure_local_sources,
        knowledge_exposure_transitions: &knowledge_exposure_transitions,
        knowledge_exposure_source_statuses: &knowledge_exposure_source_statuses,
        entity_membership_changes: &entity_membership_changes,
        relation_membership_changes: &relation_membership_changes,
        checkpoint_candidates: &checkpoint_candidates,
    })?;
    let manifest_bytes = canonical_bytes(&manifest)?;
    let manifest_digest = content_object_digest(&manifest_bytes);
    let manifest_size_bytes = usize_to_i64("manifest_size_bytes", manifest_bytes.len())?;

    Ok(BundleExportManifest {
        manifest_profile: BUNDLE_EXPORT_MANIFEST_PROFILE.to_owned(),
        manifest_version: BUNDLE_EXPORT_MANIFEST_VERSION,
        store_id: store_info.store_id,
        workspace_id: replayed.workspace_id,
        commit_id: replayed.commit_id,
        state_digest: replayed.state_digest,
        manifest_digest,
        manifest_size_bytes,
        entity_count: replayed.state.entities().len(),
        relation_count: replayed.state.relations().len(),
        commit_count: commits.len(),
        exported_branch_heads,
        entity_versions,
        acceptance_criterion_identities,
        verification_requirement_identities,
        relation_versions,
        knowledge_spaces,
        knowledge_exposures,
        knowledge_exposure_local_sources,
        knowledge_exposure_transitions,
        knowledge_exposure_source_statuses,
        entity_membership_changes,
        relation_membership_changes,
        checkpoint_candidates,
        manifest,
    })
}

pub(crate) fn validate_bundle_manifest(
    connection: &StoreConnection,
    store_info: &StoreInfo,
    options: BundleManifestValidationOptions,
) -> Result<BundleManifestValidationResult> {
    connection.verify_foreign_keys()?;
    let expected = export_bundle_manifest(
        connection,
        store_info,
        BundleExportOptions::for_commit(options.commit_id()),
    )?;
    let actual_manifest_digest = content_object_digest(options.manifest_bytes());
    let actual_manifest_size_bytes =
        usize_to_i64("actual_manifest_size_bytes", options.manifest_bytes().len())?;
    let problem = bundle_manifest_validation_problem(&expected, options.manifest_bytes())?;
    Ok(BundleManifestValidationResult {
        commit_id: options.commit_id(),
        valid: problem.is_none(),
        expected_manifest_digest: expected.manifest_digest,
        actual_manifest_digest,
        actual_manifest_size_bytes,
        problem,
    })
}

pub(crate) fn export_bundle_payloads(
    connection: &StoreConnection,
    store_info: &StoreInfo,
    options: BundlePayloadExportOptions,
) -> Result<BundlePayloadExport> {
    connection.verify_foreign_keys()?;
    let manifest = export_bundle_manifest(
        connection,
        store_info,
        BundleExportOptions::for_commit(options.commit_id()),
    )?;
    let manifest_bytes = canonical_bytes(&manifest.manifest)?;
    if content_object_digest(&manifest_bytes) != manifest.manifest_digest {
        return Err(WorkVcsError::QueryInvalid(
            "bundle manifest digest does not match canonical manifest bytes".to_owned(),
        ));
    }

    let mut commits = commit_closure_refs(connection, options.commit_id())?;
    commits.sort_by(|left, right| {
        left.committed_at_us
            .cmp(&right.committed_at_us)
            .then_with(|| left.commit_id.cmp(&right.commit_id))
    });

    let mut candidates = Vec::new();
    for commit in &commits {
        load_changeset_payload_candidates(connection, commit, &mut candidates)?;
        load_change_operation_payload_candidates(connection, commit, &mut candidates)?;
        load_entity_membership_change_payload_candidates(connection, commit, &mut candidates)?;
        load_relation_membership_change_payload_candidates(connection, commit, &mut candidates)?;
    }
    for entity_version in &manifest.entity_versions {
        load_entity_version_payload_candidate(connection, entity_version, &mut candidates)?;
    }
    for relation_version in &manifest.relation_versions {
        load_relation_version_payload_candidate(connection, relation_version, &mut candidates)?;
    }
    for source in &manifest.knowledge_exposure_local_sources {
        load_knowledge_exposure_source_knowledge_payload_candidate(
            connection,
            source,
            &mut candidates,
        )?;
    }
    for transition in &manifest.knowledge_exposure_transitions {
        load_knowledge_exposure_transition_detail_payload_candidate(
            connection,
            transition,
            &mut candidates,
        )?;
    }
    for source_status in &manifest.knowledge_exposure_source_statuses {
        load_knowledge_exposure_source_status_detail_payload_candidate(
            connection,
            source_status,
            &mut candidates,
        )?;
    }

    build_payload_export(manifest, manifest_bytes, candidates)
}

pub(crate) fn validate_bundle_payloads(
    connection: &StoreConnection,
    store_info: &StoreInfo,
    options: BundlePayloadValidationOptions,
) -> Result<BundlePayloadValidationResult> {
    connection.verify_foreign_keys()?;
    let expected = export_bundle_payloads(
        connection,
        store_info,
        BundlePayloadExportOptions::for_commit(options.commit_id()),
    )?;
    let actual_manifest_digest = content_object_digest(options.manifest_bytes());
    let actual_payload_index_digest = content_object_digest(options.payload_index_bytes());
    let problem = bundle_payload_validation_problem(&expected, &options)?;

    Ok(BundlePayloadValidationResult {
        commit_id: options.commit_id(),
        valid: problem.is_none(),
        expected_manifest_digest: expected.manifest.manifest_digest,
        actual_manifest_digest,
        expected_payload_index_digest: expected.payload_index_digest,
        actual_payload_index_digest,
        expected_payload_files: expected.payload_files.len(),
        actual_payload_files: options.payloads().len(),
        expected_payload_references: expected.payload_references.len(),
        problem,
    })
}

pub(crate) fn preflight_bundle_import(
    connection: &StoreConnection,
    store_info: &StoreInfo,
    options: BundleImportPreflightOptions,
) -> Result<BundleImportPreflightResult> {
    connection.verify_foreign_keys()?;
    let manifest_digest = content_object_digest(options.manifest_bytes());
    let payload_index_digest = content_object_digest(options.payload_index_bytes());
    let payload_files = options.payloads().len();

    let checked = match validate_bundle_directory_artifact(&options) {
        Ok(checked) => checked,
        Err(problem) => {
            return Ok(BundleImportPreflightResult {
                valid: false,
                format_compatible: false,
                source_store_id: None,
                target_workspace_id: None,
                target_commit_id: None,
                target_state_digest: None,
                source_store_relation: "unknown".to_owned(),
                incoming_commit_present: false,
                import_required: false,
                can_apply: false,
                action: "invalid_bundle_directory".to_owned(),
                manifest_digest,
                payload_index_digest,
                payload_files,
                payload_references: 0,
                exported_branch_heads: 0,
                branch_heads_already_present: 0,
                branch_heads_missing: 0,
                branch_heads_fast_forward: 0,
                branch_heads_diverged: 0,
                problem: Some(problem),
            });
        }
    };

    let format_compatible = bundle_format_compatible(store_info, &checked.manifest);
    let branch_preflight = bundle_branch_preflight_summary(connection, &checked.manifest)?;
    let existing_commit_digest =
        load_commit_state_digest_optional(connection, checked.manifest.commit_id)?;
    if let Some(existing_digest) = existing_commit_digest
        && existing_digest != checked.manifest.state_digest
    {
        return Ok(BundleImportPreflightResult {
            valid: false,
            format_compatible,
            source_store_id: Some(checked.manifest.source_store_id),
            target_workspace_id: Some(checked.manifest.workspace_id),
            target_commit_id: Some(checked.manifest.commit_id),
            target_state_digest: Some(checked.manifest.state_digest),
            source_store_relation: source_store_relation(
                store_info.store_id,
                checked.manifest.source_store_id,
            ),
            incoming_commit_present: true,
            import_required: false,
            can_apply: false,
            action: "local_commit_digest_conflict".to_owned(),
            manifest_digest,
            payload_index_digest,
            payload_files,
            payload_references: checked.payload_index.reference_count,
            exported_branch_heads: branch_preflight.exported_branch_heads,
            branch_heads_already_present: branch_preflight.already_present,
            branch_heads_missing: branch_preflight.missing,
            branch_heads_fast_forward: branch_preflight.fast_forward,
            branch_heads_diverged: branch_preflight.diverged,
            problem: Some(format!(
                "local commit {} exists with a different state digest",
                checked.manifest.commit_id
            )),
        });
    }

    let source_store_relation =
        source_store_relation(store_info.store_id, checked.manifest.source_store_id);
    let incoming_commit_present = existing_commit_digest.is_some();
    let same_store_fast_forward_ready = source_store_relation == "same_store"
        && !incoming_commit_present
        && checked.manifest.same_store_apply_supported
        && branch_preflight.exported_branch_heads > 0
        && branch_preflight.fast_forward > 0
        && branch_preflight.missing == 0
        && branch_preflight.diverged == 0;
    let (import_required, can_apply, action) = if !format_compatible {
        (true, false, "incompatible_store_format")
    } else if source_store_relation == "same_store" && incoming_commit_present {
        (false, false, "already_present")
    } else if same_store_fast_forward_ready {
        (true, true, "same_store_fast_forward_ready")
    } else if source_store_relation == "same_store" {
        (true, false, "same_store_import_not_implemented")
    } else {
        (true, false, "external_store_import_not_implemented")
    };

    Ok(BundleImportPreflightResult {
        valid: true,
        format_compatible,
        source_store_id: Some(checked.manifest.source_store_id),
        target_workspace_id: Some(checked.manifest.workspace_id),
        target_commit_id: Some(checked.manifest.commit_id),
        target_state_digest: Some(checked.manifest.state_digest),
        source_store_relation,
        incoming_commit_present,
        import_required,
        can_apply,
        action: action.to_owned(),
        manifest_digest,
        payload_index_digest,
        payload_files,
        payload_references: checked.payload_index.reference_count,
        exported_branch_heads: branch_preflight.exported_branch_heads,
        branch_heads_already_present: branch_preflight.already_present,
        branch_heads_missing: branch_preflight.missing,
        branch_heads_fast_forward: branch_preflight.fast_forward,
        branch_heads_diverged: branch_preflight.diverged,
        problem: None,
    })
}

pub(crate) fn record_bundle_import_attempt(
    connection: &mut StoreConnection,
    store_info: &StoreInfo,
    options: BundleImportAttemptOptions,
) -> Result<BundleImportAttemptResult> {
    connection.verify_foreign_keys()?;
    let preflight_options = BundleImportPreflightOptions::from_parts(
        options.manifest_bytes.clone(),
        options.payload_index_bytes.clone(),
        options.payloads.clone(),
    )?;
    let preflight = preflight_bundle_import(connection, store_info, preflight_options)?;
    let bundle_digest = preflight.payload_index_digest;
    let import_profile = BUNDLE_IMPORT_PROFILE.to_owned();

    if !preflight.valid {
        return Ok(BundleImportAttemptResult {
            import_id: None,
            recorded: false,
            bundle_digest,
            import_profile,
            started_at_us: None,
            completed_at_us: None,
            outcome: preflight.action.clone(),
            preflight,
        });
    }

    let source_store_id = preflight.source_store_id.ok_or_else(|| {
        WorkVcsError::QueryInvalid("valid bundle preflight must include source_store_id".to_owned())
    })?;
    let import_id = ImportId::new_v7();
    let now_us = current_epoch_micros()?;
    let outcome = preflight.action.clone();
    let detail_json = bundle_import_attempt_detail_json(&preflight)?;

    let import_id_bytes = import_id.raw_bytes();
    let source_store_id_bytes = source_store_id.raw_bytes();
    let bundle_digest_bytes = *bundle_digest.as_bytes();
    let origin_session_id_bytes = options.origin_session_id.map(|id| id.raw_bytes());

    let transaction = connection
        .inner_mut()
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(storage_error)?;
    transaction
        .execute(
            "INSERT INTO import_attempt(
                import_id,
                source_store_id,
                bundle_digest,
                import_profile,
                origin_session_id,
                started_at_us
             )
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                &import_id_bytes[..],
                &source_store_id_bytes[..],
                &bundle_digest_bytes[..],
                import_profile,
                origin_session_id_bytes.as_ref().map(|bytes| &bytes[..]),
                now_us
            ],
        )
        .map_err(storage_error)?;
    transaction
        .execute(
            "INSERT INTO import_attempt_outcome(
                import_id,
                outcome,
                completed_at_us,
                detail_json
             )
             VALUES (?1, ?2, ?3, ?4)",
            params![&import_id_bytes[..], outcome, now_us, detail_json],
        )
        .map_err(storage_error)?;
    transaction.commit().map_err(storage_error)?;

    Ok(BundleImportAttemptResult {
        import_id: Some(import_id),
        recorded: true,
        bundle_digest,
        import_profile: BUNDLE_IMPORT_PROFILE.to_owned(),
        started_at_us: Some(now_us),
        completed_at_us: Some(now_us),
        outcome: preflight.action.clone(),
        preflight,
    })
}

pub(crate) fn apply_bundle_import(
    connection: &mut StoreConnection,
    store_info: &StoreInfo,
    options: BundleImportApplyOptions,
) -> Result<BundleImportApplyResult> {
    connection.verify_foreign_keys()?;
    let preflight_options = BundleImportPreflightOptions::from_parts(
        options.manifest_bytes.clone(),
        options.payload_index_bytes.clone(),
        options.payloads.clone(),
    )?;
    let preflight = preflight_bundle_import(connection, store_info, preflight_options)?;
    let bundle_digest = preflight.payload_index_digest;
    let import_profile = BUNDLE_IMPORT_PROFILE.to_owned();

    if !preflight.can_apply || preflight.action != "same_store_fast_forward_ready" {
        return Ok(BundleImportApplyResult {
            import_id: None,
            applied: false,
            bundle_digest,
            import_profile,
            started_at_us: None,
            completed_at_us: None,
            outcome: preflight.action.clone(),
            preflight,
            imported_commits: 0,
            imported_entity_versions: 0,
            imported_acceptance_criterion_identities: 0,
            imported_verification_requirement_identities: 0,
            imported_relation_versions: 0,
            updated_branch_heads: 0,
        });
    }

    let manifest_value =
        parse_fixed_point_canonical_json("bundle manifest", &options.manifest_bytes)
            .map_err(WorkVcsError::QueryInvalid)?;
    let payload_index_value =
        parse_fixed_point_canonical_json("bundle payload index", &options.payload_index_bytes)
            .map_err(WorkVcsError::QueryInvalid)?;
    let document = parse_bundle_same_store_apply_document(&manifest_value)?;
    let payload_lookup = BundlePayloadLookup::from_parts(&payload_index_value, &options.payloads)?;

    let source_store_id = preflight.source_store_id.ok_or_else(|| {
        WorkVcsError::QueryInvalid(
            "applicable bundle preflight must include source_store_id".to_owned(),
        )
    })?;
    if document.source_store_id != source_store_id
        || Some(document.workspace_id) != preflight.target_workspace_id
        || Some(document.commit_id) != preflight.target_commit_id
        || Some(document.state_digest) != preflight.target_state_digest
    {
        return Err(WorkVcsError::QueryInvalid(
            "applicable bundle manifest target changed after preflight".to_owned(),
        ));
    }
    let import_id = ImportId::new_v7();
    let now_us = current_epoch_micros()?;
    let transaction = connection
        .inner_mut()
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(storage_error)?;

    let imported_entity_versions =
        apply_entity_versions(&transaction, &document, &payload_lookup, now_us)?;
    let imported_acceptance_criterion_identities =
        apply_acceptance_criterion_identities(&transaction, &document)?;
    let imported_verification_requirement_identities =
        apply_verification_requirement_identities(&transaction, &document)?;
    let imported_relation_versions =
        apply_relation_versions(&transaction, &document, &payload_lookup, now_us)?;
    let imported_commits = apply_commit_closure(&transaction, &document, &payload_lookup)?;
    let updated_branch_heads =
        apply_same_store_branch_fast_forwards(&transaction, &document, now_us)?;

    let outcome = "same_store_fast_forward_applied".to_owned();
    let detail_json = bundle_import_apply_detail_json(
        &preflight,
        imported_commits,
        imported_entity_versions,
        imported_acceptance_criterion_identities,
        imported_verification_requirement_identities,
        imported_relation_versions,
        updated_branch_heads,
    )?;
    insert_import_attempt_outcome(
        &transaction,
        ImportAttemptOutcomeInsert {
            import_id,
            source_store_id,
            bundle_digest,
            import_profile: &import_profile,
            origin_session_id: options.origin_session_id,
            now_us,
            outcome: &outcome,
            detail_json: &detail_json,
        },
    )?;
    transaction.commit().map_err(storage_error)?;

    Ok(BundleImportApplyResult {
        import_id: Some(import_id),
        applied: true,
        bundle_digest,
        import_profile,
        started_at_us: Some(now_us),
        completed_at_us: Some(now_us),
        outcome,
        preflight,
        imported_commits,
        imported_entity_versions,
        imported_acceptance_criterion_identities,
        imported_verification_requirement_identities,
        imported_relation_versions,
        updated_branch_heads,
    })
}

pub(crate) fn bundle_import_attempt(
    connection: &StoreConnection,
    import_id: ImportId,
) -> Result<BundleImportAttemptSnapshot> {
    connection.verify_foreign_keys()?;
    load_bundle_import_attempt_snapshot(connection, import_id)?
        .ok_or_else(|| WorkVcsError::QueryInvalid(format!("ImportAttempt {import_id} not found")))
}

pub(crate) fn bundle_import_attempts(
    connection: &StoreConnection,
    options: BundleImportAttemptListOptions,
) -> Result<BundleImportAttemptListResult> {
    connection.verify_foreign_keys()?;
    let limit = usize_to_i64("bundle import attempt list limit", options.limit())?;
    let source_store_id_bytes = options.source_store_id().map(|id| id.raw_bytes());
    let bundle_digest_bytes = options.bundle_digest().map(|digest| *digest.as_bytes());
    let mut statement = connection
        .inner()
        .prepare(
            "SELECT import_id
             FROM import_attempt
             WHERE (?2 IS NULL OR source_store_id = ?2)
               AND (?3 IS NULL OR bundle_digest = ?3)
             ORDER BY started_at_us DESC, import_id DESC
             LIMIT ?1",
        )
        .map_err(storage_error)?;
    let rows = statement
        .query_map(
            params![
                limit,
                source_store_id_bytes.as_ref().map(|bytes| &bytes[..]),
                bundle_digest_bytes.as_ref().map(|bytes| &bytes[..])
            ],
            |row| row.get::<_, Vec<u8>>(0),
        )
        .map_err(storage_error)?;

    let mut attempts = Vec::new();
    for row in rows {
        let import_id = decode_import_id("import_attempt.import_id", row.map_err(storage_error)?)?;
        attempts.push(bundle_import_attempt(connection, import_id)?);
    }
    Ok(BundleImportAttemptListResult { attempts })
}

fn commit_closure_refs(
    connection: &StoreConnection,
    root_commit_id: CommitId,
) -> Result<Vec<BundleCommitRef>> {
    let mut stack = vec![root_commit_id];
    let mut seen = HashSet::new();
    let mut commit_ids = Vec::new();

    while let Some(commit_id) = stack.pop() {
        if !seen.insert(commit_id.raw_bytes()) {
            continue;
        }
        let parents = load_commit_parent_refs(connection, commit_id)?;
        for parent in &parents {
            stack.push(parent.parent_commit_id);
        }
        commit_ids.push(commit_id);
    }

    commit_ids
        .into_iter()
        .map(|commit_id| load_commit_ref(connection, commit_id))
        .collect()
}

fn load_commit_ref(connection: &StoreConnection, commit_id: CommitId) -> Result<BundleCommitRef> {
    let row = connection
        .inner()
        .query_row(
            "SELECT workstate_commit.workspace_id,
                    workstate_commit.changeset_id,
                    workstate_commit.commit_kind,
                    workstate_commit.state_digest,
                    workstate_commit.committed_at_us,
                    changeset.operation_type,
                    changeset.operation_schema_version,
                    changeset.operation_payload_json,
                    changeset.rationale_json,
                    changeset.created_at_us,
                    (
                        SELECT count(*)
                        FROM change_operation
                        WHERE change_operation.changeset_id = changeset.changeset_id
                    )
             FROM workstate_commit
             JOIN changeset
               ON changeset.workspace_id = workstate_commit.workspace_id
              AND changeset.changeset_id = workstate_commit.changeset_id
             WHERE workstate_commit.commit_id = ?1",
            params![&commit_id.raw_bytes()[..]],
            |row| {
                Ok((
                    row.get::<_, Vec<u8>>(0)?,
                    row.get::<_, Vec<u8>>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, Vec<u8>>(3)?,
                    row.get::<_, i64>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, i64>(6)?,
                    row.get::<_, String>(7)?,
                    row.get::<_, String>(8)?,
                    row.get::<_, i64>(9)?,
                    row.get::<_, i64>(10)?,
                ))
            },
        )
        .optional()
        .map_err(storage_error)?;

    let Some((
        workspace_id,
        changeset_id,
        commit_kind,
        state_digest,
        committed_at_us,
        operation_type,
        operation_schema_version,
        operation_payload_json,
        rationale_json,
        changeset_created_at_us,
        change_operation_count,
    )) = row
    else {
        return Err(WorkVcsError::QueryInvalid(format!(
            "commit {commit_id} does not exist or lacks its ChangeSet"
        )));
    };

    validate_stored_text("workstate_commit.commit_kind", &commit_kind)?;
    validate_stored_text("changeset.operation_type", &operation_type)?;
    validate_nonnegative_i64("workstate_commit.committed_at_us", committed_at_us)?;
    validate_nonnegative_i64("changeset.created_at_us", changeset_created_at_us)?;
    validate_positive_i64(
        "changeset.operation_schema_version",
        operation_schema_version,
    )?;
    validate_nonnegative_i64("change_operation count", change_operation_count)?;
    validate_canonical_json_text("changeset.operation_payload_json", &operation_payload_json)?;
    validate_canonical_json_text("changeset.rationale_json", &rationale_json)?;

    Ok(BundleCommitRef {
        workspace_id: decode_workspace_id("workstate_commit.workspace_id", workspace_id)?,
        commit_id,
        changeset_id: decode_changeset_id("workstate_commit.changeset_id", changeset_id)?,
        commit_kind,
        state_digest: decode_digest("workstate_commit.state_digest", state_digest)?,
        committed_at_us,
        operation_type,
        operation_schema_version,
        changeset_created_at_us,
        operation_payload_digest: content_object_digest(operation_payload_json.as_bytes()),
        rationale_digest: content_object_digest(rationale_json.as_bytes()),
        change_operation_count,
        parents: load_commit_parent_refs(connection, commit_id)?,
    })
}

fn load_commit_parent_refs(
    connection: &StoreConnection,
    commit_id: CommitId,
) -> Result<Vec<BundleCommitParentRef>> {
    let mut statement = connection
        .inner()
        .prepare(
            "SELECT parent_ordinal, parent_role, parent_commit_id
             FROM commit_parent
             WHERE commit_id = ?1
             ORDER BY parent_ordinal, parent_commit_id",
        )
        .map_err(storage_error)?;
    let rows = statement
        .query_map(params![&commit_id.raw_bytes()[..]], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Vec<u8>>(2)?,
            ))
        })
        .map_err(storage_error)?;

    let mut parents = Vec::new();
    for row in rows {
        let (parent_ordinal, parent_role, parent_commit_id) = row.map_err(storage_error)?;
        validate_nonnegative_i64("commit_parent.parent_ordinal", parent_ordinal)?;
        validate_stored_text("commit_parent.parent_role", &parent_role)?;
        parents.push(BundleCommitParentRef {
            parent_ordinal,
            parent_role,
            parent_commit_id: decode_commit_id("commit_parent.parent_commit_id", parent_commit_id)?,
        });
    }
    Ok(parents)
}

fn exported_branch_head_refs(
    connection: &StoreConnection,
    workspace_id: WorkspaceId,
    commits: &[BundleCommitRef],
) -> Result<Vec<BundleBranchHeadRef>> {
    let commit_ids = commits
        .iter()
        .map(|commit| commit.commit_id)
        .collect::<BTreeSet<_>>();
    let mut statement = connection
        .inner()
        .prepare(
            "SELECT branch.branch_id,
                    branch.name,
                    branch.head_commit_id,
                    branch.lifecycle_state,
                    branch.created_at_us,
                    workstate_commit.state_digest
             FROM branch
             JOIN workstate_commit
               ON workstate_commit.workspace_id = branch.workspace_id
              AND workstate_commit.commit_id = branch.head_commit_id
             WHERE branch.workspace_id = ?1
             ORDER BY branch.name, branch.branch_id",
        )
        .map_err(storage_error)?;
    let rows = statement
        .query_map(params![&workspace_id.raw_bytes()[..]], |row| {
            Ok((
                row.get::<_, Vec<u8>>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Vec<u8>>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, i64>(4)?,
                row.get::<_, Vec<u8>>(5)?,
            ))
        })
        .map_err(storage_error)?;

    let mut refs = Vec::new();
    for row in rows {
        let (branch_id, name, head_commit_id, lifecycle_state, created_at_us, state_digest) =
            row.map_err(storage_error)?;
        let head_commit_id = decode_commit_id("branch.head_commit_id", head_commit_id)?;
        if !commit_ids.contains(&head_commit_id) {
            continue;
        }
        validate_stored_text("branch.name", &name)?;
        validate_stored_text("branch.lifecycle_state", &lifecycle_state)?;
        validate_positive_i64("branch.created_at_us", created_at_us)?;
        refs.push(BundleBranchHeadRef {
            workspace_id,
            branch_id: decode_branch_id("branch.branch_id", branch_id)?,
            name,
            head_commit_id,
            head_state_digest: decode_digest("workstate_commit.state_digest", state_digest)?,
            lifecycle_state,
            created_at_us,
        });
    }
    Ok(refs)
}

fn entity_version_closure_refs(
    connection: &StoreConnection,
    workspace_id: WorkspaceId,
    state: &WorkState,
    membership_changes: &[BundleEntityMembershipChangeRef],
) -> Result<Vec<BundleEntityVersionRef>> {
    let mut refs = BTreeSet::new();
    refs.extend(sorted_entities(state));
    for change in membership_changes {
        if let Some(entity_version_id) = change.before_entity_version_id {
            refs.insert((change.entity_id, entity_version_id));
        }
        if let Some(entity_version_id) = change.after_entity_version_id {
            refs.insert((change.entity_id, entity_version_id));
        }
    }
    refs.into_iter()
        .map(|(entity_id, entity_version_id)| {
            load_entity_version_ref(connection, workspace_id, entity_id, entity_version_id)
        })
        .collect()
}

fn load_entity_version_ref(
    connection: &StoreConnection,
    workspace_id: WorkspaceId,
    entity_id: EntityId,
    entity_version_id: EntityVersionId,
) -> Result<BundleEntityVersionRef> {
    let row = connection
        .inner()
        .query_row(
            "SELECT entity.workspace_id,
                    entity.entity_kind,
                    entity_version.state_schema_version,
                    entity_version.state_json,
                    entity_version.state_digest
             FROM entity_version
             JOIN entity
               ON entity.object_id = entity_version.entity_id
             WHERE entity_version.entity_id = ?1
               AND entity_version.entity_version_id = ?2",
            params![
                &entity_id.raw_bytes()[..],
                &entity_version_id.raw_bytes()[..]
            ],
            |row| {
                Ok((
                    row.get::<_, Vec<u8>>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, Vec<u8>>(4)?,
                ))
            },
        )
        .optional()
        .map_err(storage_error)?;

    let Some((stored_workspace_id, entity_kind, state_schema_version, state_json, state_digest)) =
        row
    else {
        return Err(WorkVcsError::QueryInvalid(format!(
            "EntityVersion {entity_version_id} for entity {entity_id} does not exist"
        )));
    };
    let stored_workspace_id = decode_workspace_id("entity.workspace_id", stored_workspace_id)?;
    if stored_workspace_id != workspace_id {
        return Err(WorkVcsError::QueryInvalid(format!(
            "entity {entity_id} belongs to workspace {stored_workspace_id}, not export workspace {workspace_id}"
        )));
    }
    validate_stored_text("entity.entity_kind", &entity_kind)?;
    validate_positive_i64("entity_version.state_schema_version", state_schema_version)?;
    let state_digest = decode_digest("entity_version.state_digest", state_digest)?;
    let value = validate_canonical_json_value("entity_version.state_json", &state_json)?;
    let actual_digest = entity_version_digest(&value)?;
    if actual_digest != state_digest {
        return Err(WorkVcsError::QueryInvalid(format!(
            "EntityVersion {entity_version_id} digest does not match state JSON"
        )));
    }

    Ok(BundleEntityVersionRef {
        entity_id,
        entity_version_id,
        entity_kind,
        state_schema_version,
        state_digest,
        state_json_digest: content_object_digest(state_json.as_bytes()),
        state_json_size_bytes: usize_to_i64("entity_version.state_json size", state_json.len())?,
    })
}

fn acceptance_criterion_identity_refs(
    connection: &StoreConnection,
    entity_versions: &[BundleEntityVersionRef],
) -> Result<Vec<BundleAcceptanceCriterionIdentityRef>> {
    let entity_ids = typed_entity_ids(entity_versions, ACCEPTANCE_CRITERION_ENTITY_KIND);
    entity_ids
        .into_iter()
        .map(|entity_id| load_acceptance_criterion_identity_ref(connection, entity_id))
        .collect()
}

fn load_acceptance_criterion_identity_ref(
    connection: &StoreConnection,
    entity_id: EntityId,
) -> Result<BundleAcceptanceCriterionIdentityRef> {
    let row = connection
        .inner()
        .query_row(
            "SELECT owner_entity_id, local_key
             FROM acceptance_criterion_identity
             WHERE entity_id = ?1",
            params![&entity_id.raw_bytes()[..]],
            |row| Ok((row.get::<_, Vec<u8>>(0)?, row.get::<_, String>(1)?)),
        )
        .optional()
        .map_err(storage_error)?;
    let Some((owner_entity_id, local_key)) = row else {
        return Err(WorkVcsError::QueryInvalid(format!(
            "AcceptanceCriterion identity for entity {entity_id} does not exist"
        )));
    };
    validate_stored_text("acceptance_criterion_identity.local_key", &local_key)?;
    Ok(BundleAcceptanceCriterionIdentityRef {
        entity_id,
        owner_entity_id: decode_entity_id(
            "acceptance_criterion_identity.owner_entity_id",
            owner_entity_id,
        )?,
        local_key,
    })
}

fn verification_requirement_identity_refs(
    connection: &StoreConnection,
    entity_versions: &[BundleEntityVersionRef],
) -> Result<Vec<BundleVerificationRequirementIdentityRef>> {
    let entity_ids = typed_entity_ids(entity_versions, VERIFICATION_REQUIREMENT_ENTITY_KIND);
    entity_ids
        .into_iter()
        .map(|entity_id| load_verification_requirement_identity_ref(connection, entity_id))
        .collect()
}

fn load_verification_requirement_identity_ref(
    connection: &StoreConnection,
    entity_id: EntityId,
) -> Result<BundleVerificationRequirementIdentityRef> {
    let row = connection
        .inner()
        .query_row(
            "SELECT owner_entity_id, local_key
             FROM verification_requirement_identity
             WHERE entity_id = ?1",
            params![&entity_id.raw_bytes()[..]],
            |row| Ok((row.get::<_, Vec<u8>>(0)?, row.get::<_, String>(1)?)),
        )
        .optional()
        .map_err(storage_error)?;
    let Some((owner_entity_id, local_key)) = row else {
        return Err(WorkVcsError::QueryInvalid(format!(
            "VerificationRequirement identity for entity {entity_id} does not exist"
        )));
    };
    validate_stored_text("verification_requirement_identity.local_key", &local_key)?;
    Ok(BundleVerificationRequirementIdentityRef {
        entity_id,
        owner_entity_id: decode_entity_id(
            "verification_requirement_identity.owner_entity_id",
            owner_entity_id,
        )?,
        local_key,
    })
}

fn typed_entity_ids(
    entity_versions: &[BundleEntityVersionRef],
    entity_kind: &str,
) -> Vec<EntityId> {
    entity_versions
        .iter()
        .filter_map(|entity_version| {
            (entity_version.entity_kind == entity_kind).then_some(entity_version.entity_id)
        })
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn relation_version_closure_refs(
    connection: &StoreConnection,
    workspace_id: WorkspaceId,
    state: &WorkState,
    membership_changes: &[BundleRelationMembershipChangeRef],
) -> Result<Vec<BundleRelationVersionRef>> {
    let mut refs = BTreeSet::new();
    refs.extend(sorted_relations(state));
    for change in membership_changes {
        if let Some(relation_version_id) = change.before_relation_version_id {
            refs.insert((change.relation_id, relation_version_id));
        }
        if let Some(relation_version_id) = change.after_relation_version_id {
            refs.insert((change.relation_id, relation_version_id));
        }
    }
    refs.into_iter()
        .map(|(relation_id, relation_version_id)| {
            load_relation_version_ref(connection, workspace_id, relation_id, relation_version_id)
        })
        .collect()
}

fn load_relation_version_ref(
    connection: &StoreConnection,
    workspace_id: WorkspaceId,
    relation_id: RelationId,
    relation_version_id: RelationVersionId,
) -> Result<BundleRelationVersionRef> {
    let row = connection
        .inner()
        .query_row(
            "SELECT relation.workspace_id,
                    relation.relation_type,
                    relation.source_object_id,
                    relation.target_object_id,
                    relation.relation_discriminator,
                    relation_version.state_schema_version,
                    relation_version.metadata_json,
                    relation_version.state_digest
             FROM relation_version
             JOIN relation
               ON relation.object_id = relation_version.relation_id
             WHERE relation_version.relation_id = ?1
               AND relation_version.relation_version_id = ?2",
            params![
                &relation_id.raw_bytes()[..],
                &relation_version_id.raw_bytes()[..]
            ],
            |row| {
                Ok((
                    row.get::<_, Vec<u8>>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Vec<u8>>(2)?,
                    row.get::<_, Vec<u8>>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, i64>(5)?,
                    row.get::<_, String>(6)?,
                    row.get::<_, Vec<u8>>(7)?,
                ))
            },
        )
        .optional()
        .map_err(storage_error)?;

    let Some((
        stored_workspace_id,
        relation_type,
        source_object_id,
        target_object_id,
        relation_discriminator,
        state_schema_version,
        metadata_json,
        state_digest,
    )) = row
    else {
        return Err(WorkVcsError::QueryInvalid(format!(
            "RelationVersion {relation_version_id} for relation {relation_id} does not exist"
        )));
    };
    let stored_workspace_id = decode_workspace_id("relation.workspace_id", stored_workspace_id)?;
    if stored_workspace_id != workspace_id {
        return Err(WorkVcsError::QueryInvalid(format!(
            "relation {relation_id} belongs to workspace {stored_workspace_id}, not export workspace {workspace_id}"
        )));
    }
    validate_stored_text("relation.relation_type", &relation_type)?;
    validate_stored_text_allow_empty("relation.relation_discriminator", &relation_discriminator)?;
    validate_positive_i64(
        "relation_version.state_schema_version",
        state_schema_version,
    )?;
    let state_digest = decode_digest("relation_version.state_digest", state_digest)?;
    let value = validate_canonical_json_value("relation_version.metadata_json", &metadata_json)?;
    let actual_digest = relation_version_digest(&value)?;
    if actual_digest != state_digest {
        return Err(WorkVcsError::QueryInvalid(format!(
            "RelationVersion {relation_version_id} digest does not match metadata JSON"
        )));
    }

    Ok(BundleRelationVersionRef {
        relation_id,
        relation_version_id,
        relation_type,
        source_object_id: decode_object_id_text("relation.source_object_id", source_object_id)?,
        target_object_id: decode_object_id_text("relation.target_object_id", target_object_id)?,
        relation_discriminator,
        state_schema_version,
        state_digest,
        metadata_json_digest: content_object_digest(metadata_json.as_bytes()),
        metadata_json_size_bytes: usize_to_i64(
            "relation_version.metadata_json size",
            metadata_json.len(),
        )?,
    })
}

fn knowledge_exposure_closure_refs(
    connection: &StoreConnection,
    relation_versions: &[BundleRelationVersionRef],
) -> Result<Vec<BundleKnowledgeExposureRef>> {
    let mut exposure_ids = BTreeSet::new();
    for relation_version in relation_versions {
        collect_knowledge_exposure_endpoint(
            connection,
            "relation.source_object_id",
            &relation_version.source_object_id,
            &mut exposure_ids,
        )?;
        collect_knowledge_exposure_endpoint(
            connection,
            "relation.target_object_id",
            &relation_version.target_object_id,
            &mut exposure_ids,
        )?;
    }
    exposure_ids
        .into_iter()
        .map(|exposure_id| load_knowledge_exposure_ref(connection, exposure_id))
        .collect()
}

fn collect_knowledge_exposure_endpoint(
    connection: &StoreConnection,
    label: &str,
    object_id: &str,
    exposure_ids: &mut BTreeSet<ExposureId>,
) -> Result<()> {
    let exposure_id = ExposureId::parse_canonical(object_id)
        .map_err(|error| WorkVcsError::QueryInvalid(format!("{label}: {error}")))?;
    let present = connection
        .inner()
        .query_row(
            "SELECT 1
             FROM knowledge_exposure
             WHERE exposure_id = ?1",
            params![&exposure_id.raw_bytes()[..]],
            |_| Ok(()),
        )
        .optional()
        .map_err(storage_error)?;
    if present.is_some() {
        exposure_ids.insert(exposure_id);
    }
    Ok(())
}

fn load_knowledge_exposure_ref(
    connection: &StoreConnection,
    exposure_id: ExposureId,
) -> Result<BundleKnowledgeExposureRef> {
    let row = connection
        .inner()
        .query_row(
            "SELECT object_identity.object_kind,
                    knowledge_exposure.knowledge_space_id,
                    knowledge_exposure.created_at_us
             FROM knowledge_exposure
             JOIN object_identity
               ON object_identity.object_id = knowledge_exposure.exposure_id
             WHERE knowledge_exposure.exposure_id = ?1",
            params![&exposure_id.raw_bytes()[..]],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, Vec<u8>>(1)?,
                    row.get::<_, i64>(2)?,
                ))
            },
        )
        .optional()
        .map_err(storage_error)?;
    let Some((object_kind, knowledge_space_id, created_at_us)) = row else {
        return Err(WorkVcsError::QueryInvalid(format!(
            "KnowledgeExposure {exposure_id} does not exist"
        )));
    };
    validate_object_kind_exact(
        "knowledge_exposure.object_kind",
        &object_kind,
        "knowledge_exposure",
    )?;
    validate_positive_i64("knowledge_exposure.created_at_us", created_at_us)?;
    Ok(BundleKnowledgeExposureRef {
        exposure_id,
        knowledge_space_id: decode_knowledge_space_id(
            "knowledge_exposure.knowledge_space_id",
            knowledge_space_id,
        )?,
        created_at_us,
    })
}

fn knowledge_space_closure_refs(
    connection: &StoreConnection,
    exposures: &[BundleKnowledgeExposureRef],
) -> Result<Vec<BundleKnowledgeSpaceRef>> {
    let mut knowledge_space_ids = BTreeSet::new();
    for exposure in exposures {
        knowledge_space_ids.insert(exposure.knowledge_space_id);
    }
    knowledge_space_ids
        .into_iter()
        .map(|knowledge_space_id| load_knowledge_space_ref(connection, knowledge_space_id))
        .collect()
}

fn load_knowledge_space_ref(
    connection: &StoreConnection,
    knowledge_space_id: KnowledgeSpaceId,
) -> Result<BundleKnowledgeSpaceRef> {
    let row = connection
        .inner()
        .query_row(
            "SELECT object_identity.object_kind,
                    knowledge_space.name,
                    knowledge_space.created_at_us
             FROM knowledge_space
             JOIN object_identity
               ON object_identity.object_id = knowledge_space.knowledge_space_id
             WHERE knowledge_space.knowledge_space_id = ?1",
            params![&knowledge_space_id.raw_bytes()[..]],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                ))
            },
        )
        .optional()
        .map_err(storage_error)?;
    let Some((object_kind, name, created_at_us)) = row else {
        return Err(WorkVcsError::QueryInvalid(format!(
            "KnowledgeSpace {knowledge_space_id} does not exist"
        )));
    };
    validate_object_kind_exact(
        "knowledge_space.object_kind",
        &object_kind,
        "knowledge_space",
    )?;
    validate_stored_text("knowledge_space.name", &name)?;
    validate_positive_i64("knowledge_space.created_at_us", created_at_us)?;
    Ok(BundleKnowledgeSpaceRef {
        knowledge_space_id,
        name,
        created_at_us,
    })
}

fn knowledge_exposure_local_source_refs(
    connection: &StoreConnection,
    exposures: &[BundleKnowledgeExposureRef],
) -> Result<Vec<BundleKnowledgeExposureLocalSourceRef>> {
    exposures
        .iter()
        .map(|exposure| load_knowledge_exposure_local_source_ref(connection, exposure.exposure_id))
        .collect()
}

fn load_knowledge_exposure_local_source_ref(
    connection: &StoreConnection,
    exposure_id: ExposureId,
) -> Result<BundleKnowledgeExposureLocalSourceRef> {
    let row = connection
        .inner()
        .query_row(
            "SELECT knowledge_exposure_local_source.workspace_id,
                    knowledge_exposure_local_source.knowledge_entity_id,
                    knowledge_exposure_local_source.knowledge_entity_version_id,
                    entity.workspace_id,
                    entity.entity_kind,
                    entity_version.state_json,
                    entity_version.state_digest
             FROM knowledge_exposure_local_source
             JOIN entity
               ON entity.object_id = knowledge_exposure_local_source.knowledge_entity_id
             JOIN entity_version
               ON entity_version.entity_id = knowledge_exposure_local_source.knowledge_entity_id
              AND entity_version.entity_version_id = knowledge_exposure_local_source.knowledge_entity_version_id
             WHERE knowledge_exposure_local_source.exposure_id = ?1",
            params![&exposure_id.raw_bytes()[..]],
            |row| {
                Ok((
                    row.get::<_, Vec<u8>>(0)?,
                    row.get::<_, Vec<u8>>(1)?,
                    row.get::<_, Vec<u8>>(2)?,
                    row.get::<_, Vec<u8>>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, Vec<u8>>(6)?,
                ))
            },
        )
        .optional()
        .map_err(storage_error)?;
    let Some((
        source_workspace_id,
        source_knowledge_entity_id,
        source_knowledge_entity_version_id,
        entity_workspace_id,
        entity_kind,
        state_json,
        state_digest,
    )) = row
    else {
        return Err(WorkVcsError::QueryInvalid(format!(
            "KnowledgeExposure {exposure_id} local source does not exist"
        )));
    };
    validate_object_kind_exact(
        "knowledge_exposure_local_source.entity_kind",
        &entity_kind,
        "knowledge",
    )?;
    let source_workspace_id = decode_workspace_id(
        "knowledge_exposure_local_source.workspace_id",
        source_workspace_id,
    )?;
    let entity_workspace_id = decode_workspace_id("entity.workspace_id", entity_workspace_id)?;
    if source_workspace_id != entity_workspace_id {
        return Err(WorkVcsError::QueryInvalid(format!(
            "KnowledgeExposure {exposure_id} source workspace does not match source entity workspace"
        )));
    }
    let source_knowledge_state_digest = decode_digest("entity_version.state_digest", state_digest)?;
    let value = validate_canonical_json_value(
        "knowledge_exposure_local_source.source_knowledge_state_json",
        &state_json,
    )?;
    let actual_digest = entity_version_digest(&value)?;
    if actual_digest != source_knowledge_state_digest {
        return Err(WorkVcsError::QueryInvalid(format!(
            "KnowledgeExposure {exposure_id} source KnowledgeVersion digest does not match state JSON"
        )));
    }

    Ok(BundleKnowledgeExposureLocalSourceRef {
        exposure_id,
        source_workspace_id,
        source_knowledge_entity_id: decode_entity_id(
            "knowledge_exposure_local_source.knowledge_entity_id",
            source_knowledge_entity_id,
        )?,
        source_knowledge_entity_version_id: decode_entity_version_id(
            "knowledge_exposure_local_source.knowledge_entity_version_id",
            source_knowledge_entity_version_id,
        )?,
        source_knowledge_state_digest,
        source_knowledge_state_json_digest: content_object_digest(state_json.as_bytes()),
        source_knowledge_state_json_size_bytes: usize_to_i64(
            "knowledge_exposure_local_source source state_json size",
            state_json.len(),
        )?,
    })
}

fn knowledge_exposure_transition_refs(
    connection: &StoreConnection,
    exposures: &[BundleKnowledgeExposureRef],
) -> Result<Vec<BundleKnowledgeExposureTransitionRef>> {
    let mut refs = Vec::new();
    for exposure in exposures {
        refs.extend(load_knowledge_exposure_transition_refs(
            connection,
            exposure.exposure_id,
        )?);
    }
    Ok(refs)
}

fn load_knowledge_exposure_transition_refs(
    connection: &StoreConnection,
    exposure_id: ExposureId,
) -> Result<Vec<BundleKnowledgeExposureTransitionRef>> {
    let mut statement = connection
        .inner()
        .prepare(
            "SELECT transition_id,
                    previous_transition_id,
                    lifecycle_status,
                    changed_at_us,
                    event_id,
                    detail_json
             FROM knowledge_exposure_transition
             WHERE exposure_id = ?1
             ORDER BY changed_at_us, transition_id",
        )
        .map_err(storage_error)?;
    let rows = statement
        .query_map(params![&exposure_id.raw_bytes()[..]], |row| {
            Ok((
                row.get::<_, Vec<u8>>(0)?,
                row.get::<_, Option<Vec<u8>>>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, i64>(3)?,
                row.get::<_, Option<Vec<u8>>>(4)?,
                row.get::<_, String>(5)?,
            ))
        })
        .map_err(storage_error)?;

    let mut refs = Vec::new();
    for row in rows {
        let (
            transition_id,
            previous_transition_id,
            lifecycle_status,
            changed_at_us,
            event_id,
            detail_json,
        ) = row.map_err(storage_error)?;
        validate_knowledge_exposure_lifecycle_status(
            "knowledge_exposure_transition.lifecycle_status",
            &lifecycle_status,
        )?;
        validate_positive_i64("knowledge_exposure_transition.changed_at_us", changed_at_us)?;
        validate_canonical_json_text("knowledge_exposure_transition.detail_json", &detail_json)?;
        refs.push(BundleKnowledgeExposureTransitionRef {
            exposure_id,
            transition_id: decode_exposure_transition_id(
                "knowledge_exposure_transition.transition_id",
                transition_id,
            )?,
            previous_transition_id: decode_optional_exposure_transition_id(
                "knowledge_exposure_transition.previous_transition_id",
                previous_transition_id,
            )?,
            lifecycle_status,
            changed_at_us,
            event_id: decode_optional_event_id("knowledge_exposure_transition.event_id", event_id)?,
            detail_digest: content_object_digest(detail_json.as_bytes()),
            detail_size_bytes: usize_to_i64(
                "knowledge_exposure_transition.detail_json size",
                detail_json.len(),
            )?,
        });
    }
    if refs.is_empty() {
        return Err(WorkVcsError::QueryInvalid(format!(
            "KnowledgeExposure {exposure_id} has no transition history"
        )));
    }
    Ok(refs)
}

fn knowledge_exposure_source_status_refs(
    connection: &StoreConnection,
    exposures: &[BundleKnowledgeExposureRef],
) -> Result<Vec<BundleKnowledgeExposureSourceStatusRef>> {
    exposures
        .iter()
        .map(|exposure| load_knowledge_exposure_source_status_ref(connection, exposure.exposure_id))
        .collect()
}

fn load_knowledge_exposure_source_status_ref(
    connection: &StoreConnection,
    exposure_id: ExposureId,
) -> Result<BundleKnowledgeExposureSourceStatusRef> {
    let row = connection
        .inner()
        .query_row(
            "SELECT source_status,
                    checked_at_us,
                    detail_json
             FROM knowledge_exposure_source_status
             WHERE exposure_id = ?1",
            params![&exposure_id.raw_bytes()[..]],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, String>(2)?,
                ))
            },
        )
        .optional()
        .map_err(storage_error)?;
    let Some((source_status, checked_at_us, detail_json)) = row else {
        return Err(WorkVcsError::QueryInvalid(format!(
            "KnowledgeExposure {exposure_id} has no source-status projection"
        )));
    };
    validate_knowledge_exposure_source_status(
        "knowledge_exposure_source_status.source_status",
        &source_status,
    )?;
    validate_positive_i64(
        "knowledge_exposure_source_status.checked_at_us",
        checked_at_us,
    )?;
    validate_canonical_json_text("knowledge_exposure_source_status.detail_json", &detail_json)?;
    Ok(BundleKnowledgeExposureSourceStatusRef {
        exposure_id,
        source_status,
        checked_at_us,
        detail_digest: content_object_digest(detail_json.as_bytes()),
        detail_size_bytes: usize_to_i64(
            "knowledge_exposure_source_status.detail_json size",
            detail_json.len(),
        )?,
    })
}

fn entity_membership_change_refs(
    connection: &StoreConnection,
    commits: &[BundleCommitRef],
) -> Result<Vec<BundleEntityMembershipChangeRef>> {
    let mut refs = Vec::new();
    for commit in commits {
        refs.extend(load_entity_membership_change_refs(connection, commit)?);
    }
    Ok(refs)
}

fn load_entity_membership_change_refs(
    connection: &StoreConnection,
    commit: &BundleCommitRef,
) -> Result<Vec<BundleEntityMembershipChangeRef>> {
    let mut statement = connection
        .inner()
        .prepare(
            "SELECT change_operation.operation_id,
                    change_operation.ordinal,
                    change_operation.subject_family,
                    change_operation.subject_object_id,
                    entity_membership_change.entity_id,
                    entity_membership_change.before_entity_version_id,
                    entity_membership_change.after_entity_version_id,
                    entity_membership_change.field_delta_json
             FROM entity_membership_change
             JOIN change_operation
               ON change_operation.operation_id = entity_membership_change.operation_id
              AND change_operation.subject_object_id = entity_membership_change.entity_id
             WHERE change_operation.changeset_id = ?1
             ORDER BY change_operation.ordinal, change_operation.operation_id",
        )
        .map_err(storage_error)?;
    let rows = statement
        .query_map(params![&commit.changeset_id.raw_bytes()[..]], |row| {
            Ok((
                row.get::<_, Vec<u8>>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, Vec<u8>>(3)?,
                row.get::<_, Vec<u8>>(4)?,
                row.get::<_, Option<Vec<u8>>>(5)?,
                row.get::<_, Option<Vec<u8>>>(6)?,
                row.get::<_, String>(7)?,
            ))
        })
        .map_err(storage_error)?;

    let mut refs = Vec::new();
    for row in rows {
        let (
            operation_id,
            ordinal,
            subject_family,
            subject_object_id,
            entity_id,
            before_entity_version_id,
            after_entity_version_id,
            field_delta_json,
        ) = row.map_err(storage_error)?;
        validate_subject_family_exact(
            "change_operation.subject_family",
            &subject_family,
            "entity",
        )?;
        validate_nonnegative_i64("change_operation.ordinal", ordinal)?;
        let operation_id = decode_operation_id("change_operation.operation_id", operation_id)?;
        let subject_entity_id =
            decode_entity_id("change_operation.subject_object_id", subject_object_id)?;
        let entity_id = decode_entity_id("entity_membership_change.entity_id", entity_id)?;
        if subject_entity_id != entity_id {
            return Err(WorkVcsError::QueryInvalid(format!(
                "entity membership operation {operation_id} subject does not match entity id"
            )));
        }
        let before_entity_version_id = decode_optional_entity_version_id(
            "entity_membership_change.before_entity_version_id",
            before_entity_version_id,
        )?;
        let after_entity_version_id = decode_optional_entity_version_id(
            "entity_membership_change.after_entity_version_id",
            after_entity_version_id,
        )?;
        validate_membership_transition(
            "entity_membership_change",
            before_entity_version_id,
            after_entity_version_id,
        )?;
        validate_canonical_json_text(
            "entity_membership_change.field_delta_json",
            &field_delta_json,
        )?;
        refs.push(BundleEntityMembershipChangeRef {
            changeset_id: commit.changeset_id,
            operation_id,
            ordinal,
            entity_id,
            before_entity_version_id,
            after_entity_version_id,
            field_delta_digest: content_object_digest(field_delta_json.as_bytes()),
            field_delta_size_bytes: usize_to_i64(
                "entity_membership_change.field_delta_json size",
                field_delta_json.len(),
            )?,
        });
    }
    Ok(refs)
}

fn relation_membership_change_refs(
    connection: &StoreConnection,
    commits: &[BundleCommitRef],
) -> Result<Vec<BundleRelationMembershipChangeRef>> {
    let mut refs = Vec::new();
    for commit in commits {
        refs.extend(load_relation_membership_change_refs(connection, commit)?);
    }
    Ok(refs)
}

fn load_relation_membership_change_refs(
    connection: &StoreConnection,
    commit: &BundleCommitRef,
) -> Result<Vec<BundleRelationMembershipChangeRef>> {
    let mut statement = connection
        .inner()
        .prepare(
            "SELECT change_operation.operation_id,
                    change_operation.ordinal,
                    change_operation.subject_family,
                    change_operation.subject_object_id,
                    relation_membership_change.relation_id,
                    relation_membership_change.before_relation_version_id,
                    relation_membership_change.after_relation_version_id,
                    relation_membership_change.field_delta_json
             FROM relation_membership_change
             JOIN change_operation
               ON change_operation.operation_id = relation_membership_change.operation_id
              AND change_operation.subject_object_id = relation_membership_change.relation_id
             WHERE change_operation.changeset_id = ?1
             ORDER BY change_operation.ordinal, change_operation.operation_id",
        )
        .map_err(storage_error)?;
    let rows = statement
        .query_map(params![&commit.changeset_id.raw_bytes()[..]], |row| {
            Ok((
                row.get::<_, Vec<u8>>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, Vec<u8>>(3)?,
                row.get::<_, Vec<u8>>(4)?,
                row.get::<_, Option<Vec<u8>>>(5)?,
                row.get::<_, Option<Vec<u8>>>(6)?,
                row.get::<_, String>(7)?,
            ))
        })
        .map_err(storage_error)?;

    let mut refs = Vec::new();
    for row in rows {
        let (
            operation_id,
            ordinal,
            subject_family,
            subject_object_id,
            relation_id,
            before_relation_version_id,
            after_relation_version_id,
            field_delta_json,
        ) = row.map_err(storage_error)?;
        validate_subject_family_exact(
            "change_operation.subject_family",
            &subject_family,
            "relation",
        )?;
        validate_nonnegative_i64("change_operation.ordinal", ordinal)?;
        let operation_id = decode_operation_id("change_operation.operation_id", operation_id)?;
        let subject_relation_id =
            decode_relation_id("change_operation.subject_object_id", subject_object_id)?;
        let relation_id =
            decode_relation_id("relation_membership_change.relation_id", relation_id)?;
        if subject_relation_id != relation_id {
            return Err(WorkVcsError::QueryInvalid(format!(
                "relation membership operation {operation_id} subject does not match relation id"
            )));
        }
        let before_relation_version_id = decode_optional_relation_version_id(
            "relation_membership_change.before_relation_version_id",
            before_relation_version_id,
        )?;
        let after_relation_version_id = decode_optional_relation_version_id(
            "relation_membership_change.after_relation_version_id",
            after_relation_version_id,
        )?;
        validate_membership_transition(
            "relation_membership_change",
            before_relation_version_id,
            after_relation_version_id,
        )?;
        validate_canonical_json_text(
            "relation_membership_change.field_delta_json",
            &field_delta_json,
        )?;
        refs.push(BundleRelationMembershipChangeRef {
            changeset_id: commit.changeset_id,
            operation_id,
            ordinal,
            relation_id,
            before_relation_version_id,
            after_relation_version_id,
            field_delta_digest: content_object_digest(field_delta_json.as_bytes()),
            field_delta_size_bytes: usize_to_i64(
                "relation_membership_change.field_delta_json size",
                field_delta_json.len(),
            )?,
        });
    }
    Ok(refs)
}

fn load_entity_membership_change_payload_candidates(
    connection: &StoreConnection,
    commit: &BundleCommitRef,
    candidates: &mut Vec<BundlePayloadCandidate>,
) -> Result<()> {
    let mut statement = connection
        .inner()
        .prepare(
            "SELECT change_operation.operation_id,
                    change_operation.ordinal,
                    entity_membership_change.entity_id,
                    entity_membership_change.field_delta_json
             FROM entity_membership_change
             JOIN change_operation
               ON change_operation.operation_id = entity_membership_change.operation_id
              AND change_operation.subject_object_id = entity_membership_change.entity_id
             WHERE change_operation.changeset_id = ?1
             ORDER BY change_operation.ordinal, change_operation.operation_id",
        )
        .map_err(storage_error)?;
    let rows = statement
        .query_map(params![&commit.changeset_id.raw_bytes()[..]], |row| {
            Ok((
                row.get::<_, Vec<u8>>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, Vec<u8>>(2)?,
                row.get::<_, String>(3)?,
            ))
        })
        .map_err(storage_error)?;

    for row in rows {
        let (operation_id, ordinal, entity_id, field_delta_json) = row.map_err(storage_error)?;
        let operation_id = decode_operation_id("change_operation.operation_id", operation_id)?;
        validate_nonnegative_i64("change_operation.ordinal", ordinal)?;
        let entity_id = decode_entity_id("entity_membership_change.entity_id", entity_id)?;
        push_canonical_payload(
            candidates,
            "entity_membership_field_delta",
            CanonicalValue::object(vec![
                string_field("changeset_id", commit.changeset_id.to_string()),
                string_field("operation_id", operation_id.to_string()),
                integer_field("ordinal", ordinal)?,
                string_field("entity_id", entity_id.to_string()),
            ])?,
            "entity_membership_change.field_delta_json",
            field_delta_json,
        )?;
    }
    Ok(())
}

fn load_relation_membership_change_payload_candidates(
    connection: &StoreConnection,
    commit: &BundleCommitRef,
    candidates: &mut Vec<BundlePayloadCandidate>,
) -> Result<()> {
    let mut statement = connection
        .inner()
        .prepare(
            "SELECT change_operation.operation_id,
                    change_operation.ordinal,
                    relation_membership_change.relation_id,
                    relation_membership_change.field_delta_json
             FROM relation_membership_change
             JOIN change_operation
               ON change_operation.operation_id = relation_membership_change.operation_id
              AND change_operation.subject_object_id = relation_membership_change.relation_id
             WHERE change_operation.changeset_id = ?1
             ORDER BY change_operation.ordinal, change_operation.operation_id",
        )
        .map_err(storage_error)?;
    let rows = statement
        .query_map(params![&commit.changeset_id.raw_bytes()[..]], |row| {
            Ok((
                row.get::<_, Vec<u8>>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, Vec<u8>>(2)?,
                row.get::<_, String>(3)?,
            ))
        })
        .map_err(storage_error)?;

    for row in rows {
        let (operation_id, ordinal, relation_id, field_delta_json) = row.map_err(storage_error)?;
        let operation_id = decode_operation_id("change_operation.operation_id", operation_id)?;
        validate_nonnegative_i64("change_operation.ordinal", ordinal)?;
        let relation_id =
            decode_relation_id("relation_membership_change.relation_id", relation_id)?;
        push_canonical_payload(
            candidates,
            "relation_membership_field_delta",
            CanonicalValue::object(vec![
                string_field("changeset_id", commit.changeset_id.to_string()),
                string_field("operation_id", operation_id.to_string()),
                integer_field("ordinal", ordinal)?,
                string_field("relation_id", relation_id.to_string()),
            ])?,
            "relation_membership_change.field_delta_json",
            field_delta_json,
        )?;
    }
    Ok(())
}

fn load_changeset_payload_candidates(
    connection: &StoreConnection,
    commit: &BundleCommitRef,
    candidates: &mut Vec<BundlePayloadCandidate>,
) -> Result<()> {
    let row = connection
        .inner()
        .query_row(
            "SELECT operation_payload_json, rationale_json
             FROM changeset
             WHERE changeset_id = ?1",
            params![&commit.changeset_id.raw_bytes()[..]],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
        )
        .optional()
        .map_err(storage_error)?;

    let Some((operation_payload_json, rationale_json)) = row else {
        return Err(WorkVcsError::QueryInvalid(format!(
            "ChangeSet {} does not exist",
            commit.changeset_id
        )));
    };
    push_canonical_payload(
        candidates,
        "changeset_operation_payload",
        CanonicalValue::object(vec![
            string_field("commit_id", commit.commit_id.to_string()),
            string_field("changeset_id", commit.changeset_id.to_string()),
        ])?,
        "changeset.operation_payload_json",
        operation_payload_json,
    )?;
    push_canonical_payload(
        candidates,
        "changeset_rationale",
        CanonicalValue::object(vec![
            string_field("commit_id", commit.commit_id.to_string()),
            string_field("changeset_id", commit.changeset_id.to_string()),
        ])?,
        "changeset.rationale_json",
        rationale_json,
    )
}

fn load_change_operation_payload_candidates(
    connection: &StoreConnection,
    commit: &BundleCommitRef,
    candidates: &mut Vec<BundlePayloadCandidate>,
) -> Result<()> {
    let mut statement = connection
        .inner()
        .prepare(
            "SELECT operation_id,
                    ordinal,
                    subject_family,
                    subject_object_id,
                    operation_payload_json
             FROM change_operation
             WHERE changeset_id = ?1
             ORDER BY ordinal, operation_id",
        )
        .map_err(storage_error)?;
    let rows = statement
        .query_map(params![&commit.changeset_id.raw_bytes()[..]], |row| {
            Ok((
                row.get::<_, Vec<u8>>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, Vec<u8>>(3)?,
                row.get::<_, String>(4)?,
            ))
        })
        .map_err(storage_error)?;

    for row in rows {
        let (operation_id, ordinal, subject_family, subject_object_id, operation_payload_json) =
            row.map_err(storage_error)?;
        let operation_id = decode_operation_id("change_operation.operation_id", operation_id)?;
        validate_nonnegative_i64("change_operation.ordinal", ordinal)?;
        validate_subject_family("change_operation.subject_family", &subject_family)?;
        let subject_object_id =
            decode_object_id_text("change_operation.subject_object_id", subject_object_id)?;
        push_canonical_payload(
            candidates,
            "change_operation_payload",
            CanonicalValue::object(vec![
                string_field("changeset_id", commit.changeset_id.to_string()),
                string_field("operation_id", operation_id.to_string()),
                integer_field("ordinal", ordinal)?,
                string_field("subject_family", subject_family),
                string_field("subject_object_id", subject_object_id),
            ])?,
            "change_operation.operation_payload_json",
            operation_payload_json,
        )?;
    }
    Ok(())
}

fn load_entity_version_payload_candidate(
    connection: &StoreConnection,
    entity_version: &BundleEntityVersionRef,
    candidates: &mut Vec<BundlePayloadCandidate>,
) -> Result<()> {
    let state_json = connection
        .inner()
        .query_row(
            "SELECT state_json
             FROM entity_version
             WHERE entity_id = ?1
               AND entity_version_id = ?2",
            params![
                &entity_version.entity_id.raw_bytes()[..],
                &entity_version.entity_version_id.raw_bytes()[..]
            ],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(storage_error)?
        .ok_or_else(|| {
            WorkVcsError::QueryInvalid(format!(
                "EntityVersion {} for entity {} does not exist",
                entity_version.entity_version_id, entity_version.entity_id
            ))
        })?;
    if content_object_digest(state_json.as_bytes()) != entity_version.state_json_digest {
        return Err(WorkVcsError::QueryInvalid(format!(
            "EntityVersion {} raw state JSON digest changed during payload export",
            entity_version.entity_version_id
        )));
    }
    push_canonical_payload(
        candidates,
        "entity_version_state",
        CanonicalValue::object(vec![
            string_field("entity_id", entity_version.entity_id.to_string()),
            string_field(
                "entity_version_id",
                entity_version.entity_version_id.to_string(),
            ),
        ])?,
        "entity_version.state_json",
        state_json,
    )
}

fn load_relation_version_payload_candidate(
    connection: &StoreConnection,
    relation_version: &BundleRelationVersionRef,
    candidates: &mut Vec<BundlePayloadCandidate>,
) -> Result<()> {
    let metadata_json = connection
        .inner()
        .query_row(
            "SELECT metadata_json
             FROM relation_version
             WHERE relation_id = ?1
               AND relation_version_id = ?2",
            params![
                &relation_version.relation_id.raw_bytes()[..],
                &relation_version.relation_version_id.raw_bytes()[..]
            ],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(storage_error)?
        .ok_or_else(|| {
            WorkVcsError::QueryInvalid(format!(
                "RelationVersion {} for relation {} does not exist",
                relation_version.relation_version_id, relation_version.relation_id
            ))
        })?;
    if content_object_digest(metadata_json.as_bytes()) != relation_version.metadata_json_digest {
        return Err(WorkVcsError::QueryInvalid(format!(
            "RelationVersion {} raw metadata JSON digest changed during payload export",
            relation_version.relation_version_id
        )));
    }
    push_canonical_payload(
        candidates,
        "relation_version_metadata",
        CanonicalValue::object(vec![
            string_field("relation_id", relation_version.relation_id.to_string()),
            string_field(
                "relation_version_id",
                relation_version.relation_version_id.to_string(),
            ),
        ])?,
        "relation_version.metadata_json",
        metadata_json,
    )
}

fn load_knowledge_exposure_source_knowledge_payload_candidate(
    connection: &StoreConnection,
    source: &BundleKnowledgeExposureLocalSourceRef,
    candidates: &mut Vec<BundlePayloadCandidate>,
) -> Result<()> {
    let state_json = connection
        .inner()
        .query_row(
            "SELECT state_json
             FROM entity_version
             WHERE entity_id = ?1
               AND entity_version_id = ?2",
            params![
                &source.source_knowledge_entity_id.raw_bytes()[..],
                &source.source_knowledge_entity_version_id.raw_bytes()[..]
            ],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(storage_error)?
        .ok_or_else(|| {
            WorkVcsError::QueryInvalid(format!(
                "KnowledgeExposure {} source KnowledgeVersion {} for entity {} does not exist",
                source.exposure_id,
                source.source_knowledge_entity_version_id,
                source.source_knowledge_entity_id
            ))
        })?;
    if content_object_digest(state_json.as_bytes()) != source.source_knowledge_state_json_digest {
        return Err(WorkVcsError::QueryInvalid(format!(
            "KnowledgeExposure {} source KnowledgeVersion raw state JSON digest changed during payload export",
            source.exposure_id
        )));
    }
    push_canonical_payload(
        candidates,
        "knowledge_exposure_source_knowledge_state",
        CanonicalValue::object(vec![
            string_field("exposure_id", source.exposure_id.to_string()),
            string_field(
                "source_workspace_id",
                source.source_workspace_id.to_string(),
            ),
            string_field(
                "source_knowledge_entity_id",
                source.source_knowledge_entity_id.to_string(),
            ),
            string_field(
                "source_knowledge_entity_version_id",
                source.source_knowledge_entity_version_id.to_string(),
            ),
        ])?,
        "knowledge_exposure_local_source.source_knowledge_state_json",
        state_json,
    )
}

fn load_knowledge_exposure_transition_detail_payload_candidate(
    connection: &StoreConnection,
    transition: &BundleKnowledgeExposureTransitionRef,
    candidates: &mut Vec<BundlePayloadCandidate>,
) -> Result<()> {
    let detail_json = connection
        .inner()
        .query_row(
            "SELECT detail_json
             FROM knowledge_exposure_transition
             WHERE exposure_id = ?1
               AND transition_id = ?2",
            params![
                &transition.exposure_id.raw_bytes()[..],
                &transition.transition_id.raw_bytes()[..]
            ],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(storage_error)?
        .ok_or_else(|| {
            WorkVcsError::QueryInvalid(format!(
                "KnowledgeExposure {} transition {} does not exist",
                transition.exposure_id, transition.transition_id
            ))
        })?;
    if content_object_digest(detail_json.as_bytes()) != transition.detail_digest {
        return Err(WorkVcsError::QueryInvalid(format!(
            "KnowledgeExposure {} transition {} detail JSON digest changed during payload export",
            transition.exposure_id, transition.transition_id
        )));
    }
    push_canonical_payload(
        candidates,
        "knowledge_exposure_transition_detail",
        CanonicalValue::object(vec![
            string_field("exposure_id", transition.exposure_id.to_string()),
            string_field("transition_id", transition.transition_id.to_string()),
        ])?,
        "knowledge_exposure_transition.detail_json",
        detail_json,
    )
}

fn load_knowledge_exposure_source_status_detail_payload_candidate(
    connection: &StoreConnection,
    source_status: &BundleKnowledgeExposureSourceStatusRef,
    candidates: &mut Vec<BundlePayloadCandidate>,
) -> Result<()> {
    let detail_json = connection
        .inner()
        .query_row(
            "SELECT detail_json
             FROM knowledge_exposure_source_status
             WHERE exposure_id = ?1",
            params![&source_status.exposure_id.raw_bytes()[..]],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(storage_error)?
        .ok_or_else(|| {
            WorkVcsError::QueryInvalid(format!(
                "KnowledgeExposure {} source-status projection does not exist",
                source_status.exposure_id
            ))
        })?;
    if content_object_digest(detail_json.as_bytes()) != source_status.detail_digest {
        return Err(WorkVcsError::QueryInvalid(format!(
            "KnowledgeExposure {} source-status detail JSON digest changed during payload export",
            source_status.exposure_id
        )));
    }
    push_canonical_payload(
        candidates,
        "knowledge_exposure_source_status_detail",
        CanonicalValue::object(vec![string_field(
            "exposure_id",
            source_status.exposure_id.to_string(),
        )])?,
        "knowledge_exposure_source_status.detail_json",
        detail_json,
    )
}

fn push_canonical_payload(
    candidates: &mut Vec<BundlePayloadCandidate>,
    role: &str,
    owner: CanonicalValue,
    label: &str,
    json: String,
) -> Result<()> {
    validate_canonical_json_value(label, &json)?;
    candidates.push(BundlePayloadCandidate {
        role: role.to_owned(),
        owner,
        bytes: json.into_bytes(),
    });
    Ok(())
}

fn build_payload_export(
    manifest: BundleExportManifest,
    manifest_bytes: Vec<u8>,
    candidates: Vec<BundlePayloadCandidate>,
) -> Result<BundlePayloadExport> {
    let mut payload_files_by_digest: BTreeMap<Digest, BundlePayloadFile> = BTreeMap::new();
    let mut payload_references = Vec::new();

    for candidate in candidates {
        let content_digest = content_object_digest(&candidate.bytes);
        let size_bytes = usize_to_i64("bundle payload size", candidate.bytes.len())?;
        let relative_path = format!("payloads/{content_digest}.json");
        if let Some(existing) = payload_files_by_digest.get(&content_digest) {
            if existing.bytes != candidate.bytes {
                return Err(WorkVcsError::QueryInvalid(format!(
                    "bundle payload digest collision at {content_digest}"
                )));
            }
        } else {
            payload_files_by_digest.insert(
                content_digest,
                BundlePayloadFile {
                    relative_path: relative_path.clone(),
                    content_digest,
                    size_bytes,
                    media_type: BUNDLE_PAYLOAD_MEDIA_TYPE.to_owned(),
                    bytes: candidate.bytes.clone(),
                },
            );
        }
        payload_references.push(BundlePayloadReference {
            role: candidate.role,
            relative_path,
            content_digest,
            size_bytes,
            owner: candidate.owner,
        });
    }

    let payload_files = payload_files_by_digest.into_values().collect::<Vec<_>>();
    let payload_index = payload_index_value(
        &manifest,
        &manifest_bytes,
        &payload_files,
        &payload_references,
    )?;
    let payload_index_bytes = canonical_bytes(&payload_index)?;
    let payload_index_digest = content_object_digest(&payload_index_bytes);
    let payload_index_size_bytes =
        usize_to_i64("payload_index_size_bytes", payload_index_bytes.len())?;

    Ok(BundlePayloadExport {
        manifest,
        manifest_bytes,
        payload_index,
        payload_index_bytes,
        payload_index_digest,
        payload_index_size_bytes,
        payload_files,
        payload_references,
    })
}

fn bundle_payload_validation_problem(
    expected: &BundlePayloadExport,
    options: &BundlePayloadValidationOptions,
) -> Result<Option<String>> {
    if let Some(problem) =
        bundle_manifest_validation_problem(&expected.manifest, options.manifest_bytes())?
    {
        return Ok(Some(format!("bundle manifest is invalid: {problem}")));
    }

    let payload_index = match parse_canonical_json(options.payload_index_bytes()) {
        Ok(value) => value,
        Err(error) => {
            return Ok(Some(format!(
                "bundle payload index JSON is invalid: {error}"
            )));
        }
    };
    let payload_index_bytes = canonical_bytes(&payload_index)?;
    if payload_index_bytes != options.payload_index_bytes() {
        return Ok(Some(
            "bundle payload index bytes are not fixed-point canonical JSON".to_owned(),
        ));
    }
    if payload_index_bytes != expected.payload_index_bytes {
        return Ok(Some(
            "bundle payload index content does not match expected export payload index".to_owned(),
        ));
    }

    let mut expected_by_path = expected
        .payload_files
        .iter()
        .map(|payload| (payload.relative_path.clone(), payload))
        .collect::<HashMap<_, _>>();
    let mut seen_paths = HashSet::new();
    for payload in options.payloads() {
        validate_payload_relative_path(&payload.relative_path)?;
        if !seen_paths.insert(payload.relative_path.clone()) {
            return Ok(Some(format!(
                "bundle payload path {} appears more than once",
                payload.relative_path
            )));
        }
        let Some(expected_payload) = expected_by_path.remove(&payload.relative_path) else {
            return Ok(Some(format!(
                "bundle payload path {} is not expected by payload index",
                payload.relative_path
            )));
        };
        let actual_digest = content_object_digest(&payload.bytes);
        if actual_digest != expected_payload.content_digest {
            return Ok(Some(format!(
                "bundle payload {} digest does not match expected content digest",
                payload.relative_path
            )));
        }
        if payload.bytes != expected_payload.bytes {
            return Ok(Some(format!(
                "bundle payload {} bytes do not match expected export payload",
                payload.relative_path
            )));
        }
    }
    if let Some(missing_path) = expected_by_path.keys().min() {
        return Ok(Some(format!(
            "bundle payload path {missing_path} is missing"
        )));
    }
    Ok(None)
}

fn validate_bundle_directory_artifact(
    options: &BundleImportPreflightOptions,
) -> std::result::Result<CheckedBundleDirectory, String> {
    let manifest_value =
        parse_fixed_point_canonical_json("bundle manifest", options.manifest_bytes())?;
    let manifest = parse_bundle_manifest_summary(&manifest_value)?;
    let payload_index_value =
        parse_fixed_point_canonical_json("bundle payload index", options.payload_index_bytes())?;
    let payload_index = parse_bundle_payload_index_summary(&payload_index_value)?;

    let manifest_digest = content_object_digest(options.manifest_bytes());
    if payload_index.manifest_digest != manifest_digest {
        return Err(
            "bundle payload index manifest digest does not match manifest bytes".to_owned(),
        );
    }
    if usize_to_i64("bundle manifest size", options.manifest_bytes().len())
        .map_err(|error| error.to_string())?
        != payload_index.manifest_size_bytes
    {
        return Err("bundle payload index manifest size does not match manifest bytes".to_owned());
    }
    if payload_index.workspace_id != manifest.workspace_id
        || payload_index.commit_id != manifest.commit_id
        || payload_index.state_digest != manifest.state_digest
    {
        return Err("bundle payload index target does not match manifest target".to_owned());
    }

    let mut expected_by_path = BTreeMap::new();
    for payload in &payload_index.payloads {
        let path_digest = payload_digest_from_relative_path(&payload.relative_path)
            .map_err(|error| error.to_string())?;
        if path_digest != payload.content_digest {
            return Err(format!(
                "bundle payload path {} does not match declared content digest",
                payload.relative_path
            ));
        }
        if payload.media_type != BUNDLE_PAYLOAD_MEDIA_TYPE {
            return Err(format!(
                "bundle payload path {} has unsupported media type {}",
                payload.relative_path, payload.media_type
            ));
        }
        if expected_by_path
            .insert(payload.relative_path.clone(), payload)
            .is_some()
        {
            return Err(format!(
                "bundle payload index path {} appears more than once",
                payload.relative_path
            ));
        }
    }

    let mut seen_paths = HashSet::new();
    for payload in options.payloads() {
        validate_payload_relative_path(&payload.relative_path)
            .map_err(|error| error.to_string())?;
        if !seen_paths.insert(payload.relative_path.clone()) {
            return Err(format!(
                "bundle payload path {} appears more than once",
                payload.relative_path
            ));
        }
        let Some(expected_payload) = expected_by_path.remove(&payload.relative_path) else {
            return Err(format!(
                "bundle payload path {} is not listed by payload index",
                payload.relative_path
            ));
        };
        let actual_digest = content_object_digest(&payload.bytes);
        if actual_digest != expected_payload.content_digest {
            return Err(format!(
                "bundle payload {} digest does not match payload index",
                payload.relative_path
            ));
        }
        if usize_to_i64("bundle payload size", payload.bytes.len())
            .map_err(|error| error.to_string())?
            != expected_payload.size_bytes
        {
            return Err(format!(
                "bundle payload {} size does not match payload index",
                payload.relative_path
            ));
        }
        parse_fixed_point_canonical_json("bundle payload", &payload.bytes)?;
    }
    if let Some(missing_path) = expected_by_path.keys().next() {
        return Err(format!("bundle payload path {missing_path} is missing"));
    }

    Ok(CheckedBundleDirectory {
        manifest,
        payload_index,
    })
}

fn bundle_manifest_validation_problem(
    expected: &BundleExportManifest,
    manifest_bytes: &[u8],
) -> Result<Option<String>> {
    let actual = match parse_canonical_json(manifest_bytes) {
        Ok(value) => value,
        Err(error) => {
            return Ok(Some(format!("bundle manifest JSON is invalid: {error}")));
        }
    };
    let reencoded = canonical_bytes(&actual)?;
    if reencoded != manifest_bytes {
        return Ok(Some(
            "bundle manifest bytes are not fixed-point canonical JSON".to_owned(),
        ));
    }
    let expected_bytes = canonical_bytes(&expected.manifest)?;
    if reencoded != expected_bytes {
        return Ok(Some(
            "bundle manifest content does not match expected export manifest".to_owned(),
        ));
    }
    Ok(None)
}

struct BundleManifestValueInput<'a> {
    store_info: &'a StoreInfo,
    replayed: &'a super::ReplayedState,
    commits: &'a [BundleCommitRef],
    exported_branch_heads: &'a [BundleBranchHeadRef],
    entity_versions: &'a [BundleEntityVersionRef],
    acceptance_criterion_identities: &'a [BundleAcceptanceCriterionIdentityRef],
    verification_requirement_identities: &'a [BundleVerificationRequirementIdentityRef],
    relation_versions: &'a [BundleRelationVersionRef],
    knowledge_spaces: &'a [BundleKnowledgeSpaceRef],
    knowledge_exposures: &'a [BundleKnowledgeExposureRef],
    knowledge_exposure_local_sources: &'a [BundleKnowledgeExposureLocalSourceRef],
    knowledge_exposure_transitions: &'a [BundleKnowledgeExposureTransitionRef],
    knowledge_exposure_source_statuses: &'a [BundleKnowledgeExposureSourceStatusRef],
    entity_membership_changes: &'a [BundleEntityMembershipChangeRef],
    relation_membership_changes: &'a [BundleRelationMembershipChangeRef],
    checkpoint_candidates: &'a [BundleCheckpointCandidate],
}

fn manifest_value(input: BundleManifestValueInput<'_>) -> Result<CanonicalValue> {
    CanonicalValue::object(vec![
        string_field("bundle_manifest_profile", BUNDLE_EXPORT_MANIFEST_PROFILE),
        integer_field("bundle_manifest_version", BUNDLE_EXPORT_MANIFEST_VERSION)?,
        (
            "export_limits".to_owned(),
            CanonicalValue::object(vec![
                (
                    "contains_bundle_container".to_owned(),
                    CanonicalValue::Bool(false),
                ),
                (
                    "contains_import_attempt".to_owned(),
                    CanonicalValue::Bool(false),
                ),
                (
                    "contains_payload_bytes".to_owned(),
                    CanonicalValue::Bool(false),
                ),
                (
                    "contains_remote_transport".to_owned(),
                    CanonicalValue::Bool(false),
                ),
            ])?,
        ),
        (
            "store".to_owned(),
            store_manifest_value(input.store_info.store_id, &input.store_info.manifest)?,
        ),
        (
            "target".to_owned(),
            CanonicalValue::object(vec![
                string_field("workspace_id", input.replayed.workspace_id.to_string()),
                string_field("commit_id", input.replayed.commit_id.to_string()),
                string_field("state_digest", input.replayed.state_digest.to_string()),
            ])?,
        ),
        (
            "commit_closure".to_owned(),
            CanonicalValue::Array(
                input
                    .commits
                    .iter()
                    .map(commit_ref_value)
                    .collect::<Result<Vec<_>>>()?,
            ),
        ),
        (
            "exported_branch_heads".to_owned(),
            CanonicalValue::Array(
                input
                    .exported_branch_heads
                    .iter()
                    .map(branch_head_ref_value)
                    .collect::<Result<Vec<_>>>()?,
            ),
        ),
        (
            "work_state".to_owned(),
            work_state_manifest_value(&input.replayed.state)?,
        ),
        (
            "entity_versions".to_owned(),
            CanonicalValue::Array(
                input
                    .entity_versions
                    .iter()
                    .map(entity_version_ref_value)
                    .collect::<Result<Vec<_>>>()?,
            ),
        ),
        (
            "acceptance_criterion_identities".to_owned(),
            CanonicalValue::Array(
                input
                    .acceptance_criterion_identities
                    .iter()
                    .map(acceptance_criterion_identity_ref_value)
                    .collect::<Result<Vec<_>>>()?,
            ),
        ),
        (
            "verification_requirement_identities".to_owned(),
            CanonicalValue::Array(
                input
                    .verification_requirement_identities
                    .iter()
                    .map(verification_requirement_identity_ref_value)
                    .collect::<Result<Vec<_>>>()?,
            ),
        ),
        (
            "relation_versions".to_owned(),
            CanonicalValue::Array(
                input
                    .relation_versions
                    .iter()
                    .map(relation_version_ref_value)
                    .collect::<Result<Vec<_>>>()?,
            ),
        ),
        (
            "knowledge_spaces".to_owned(),
            CanonicalValue::Array(
                input
                    .knowledge_spaces
                    .iter()
                    .map(knowledge_space_ref_value)
                    .collect::<Result<Vec<_>>>()?,
            ),
        ),
        (
            "knowledge_exposures".to_owned(),
            CanonicalValue::Array(
                input
                    .knowledge_exposures
                    .iter()
                    .map(knowledge_exposure_ref_value)
                    .collect::<Result<Vec<_>>>()?,
            ),
        ),
        (
            "knowledge_exposure_local_sources".to_owned(),
            CanonicalValue::Array(
                input
                    .knowledge_exposure_local_sources
                    .iter()
                    .map(knowledge_exposure_local_source_ref_value)
                    .collect::<Result<Vec<_>>>()?,
            ),
        ),
        (
            "knowledge_exposure_transitions".to_owned(),
            CanonicalValue::Array(
                input
                    .knowledge_exposure_transitions
                    .iter()
                    .map(knowledge_exposure_transition_ref_value)
                    .collect::<Result<Vec<_>>>()?,
            ),
        ),
        (
            "knowledge_exposure_source_statuses".to_owned(),
            CanonicalValue::Array(
                input
                    .knowledge_exposure_source_statuses
                    .iter()
                    .map(knowledge_exposure_source_status_ref_value)
                    .collect::<Result<Vec<_>>>()?,
            ),
        ),
        (
            "entity_membership_changes".to_owned(),
            CanonicalValue::Array(
                input
                    .entity_membership_changes
                    .iter()
                    .map(entity_membership_change_ref_value)
                    .collect::<Result<Vec<_>>>()?,
            ),
        ),
        (
            "relation_membership_changes".to_owned(),
            CanonicalValue::Array(
                input
                    .relation_membership_changes
                    .iter()
                    .map(relation_membership_change_ref_value)
                    .collect::<Result<Vec<_>>>()?,
            ),
        ),
        (
            "checkpoint_candidates".to_owned(),
            CanonicalValue::Array(
                input
                    .checkpoint_candidates
                    .iter()
                    .map(checkpoint_candidate_value)
                    .collect::<Result<Vec<_>>>()?,
            ),
        ),
    ])
}

fn store_manifest_value(store_id: StoreId, manifest: &StoreManifest) -> Result<CanonicalValue> {
    CanonicalValue::object(vec![
        string_field("store_id", store_id.to_string()),
        integer_field("store_format_version", manifest.store_format_version)?,
        integer_field("schema_version", manifest.schema_version)?,
        integer_field(
            "object_store_format_version",
            manifest.object_store_format_version,
        )?,
        string_field("id_scheme", manifest.id_scheme.clone()),
        string_field("digest_algorithm", manifest.digest_algorithm.clone()),
        string_field(
            "canonical_json_profile",
            manifest.canonical_json_profile.clone(),
        ),
    ])
}

fn commit_ref_value(commit: &BundleCommitRef) -> Result<CanonicalValue> {
    CanonicalValue::object(vec![
        string_field("workspace_id", commit.workspace_id.to_string()),
        string_field("commit_id", commit.commit_id.to_string()),
        string_field("changeset_id", commit.changeset_id.to_string()),
        string_field("commit_kind", commit.commit_kind.clone()),
        string_field("state_digest", commit.state_digest.to_string()),
        integer_field("committed_at_us", commit.committed_at_us)?,
        string_field("operation_type", commit.operation_type.clone()),
        integer_field("operation_schema_version", commit.operation_schema_version)?,
        integer_field("changeset_created_at_us", commit.changeset_created_at_us)?,
        string_field(
            "operation_payload_digest",
            commit.operation_payload_digest.to_string(),
        ),
        string_field("rationale_digest", commit.rationale_digest.to_string()),
        integer_field("change_operation_count", commit.change_operation_count)?,
        (
            "parents".to_owned(),
            CanonicalValue::Array(
                commit
                    .parents
                    .iter()
                    .map(commit_parent_ref_value)
                    .collect::<Result<Vec<_>>>()?,
            ),
        ),
    ])
}

fn commit_parent_ref_value(parent: &BundleCommitParentRef) -> Result<CanonicalValue> {
    CanonicalValue::object(vec![
        integer_field("parent_ordinal", parent.parent_ordinal)?,
        string_field("parent_role", parent.parent_role.clone()),
        string_field("parent_commit_id", parent.parent_commit_id.to_string()),
    ])
}

fn branch_head_ref_value(branch: &BundleBranchHeadRef) -> Result<CanonicalValue> {
    CanonicalValue::object(vec![
        string_field("workspace_id", branch.workspace_id.to_string()),
        string_field("branch_id", branch.branch_id.to_string()),
        string_field("name", branch.name.clone()),
        string_field("head_commit_id", branch.head_commit_id.to_string()),
        string_field("head_state_digest", branch.head_state_digest.to_string()),
        string_field("lifecycle_state", branch.lifecycle_state.clone()),
        integer_field("created_at_us", branch.created_at_us)?,
    ])
}

fn work_state_manifest_value(state: &WorkState) -> Result<CanonicalValue> {
    CanonicalValue::object(vec![
        integer_field(
            "entity_count",
            usize_to_i64("entity_count", state.entities().len())?,
        )?,
        (
            "entities".to_owned(),
            CanonicalValue::Array(
                sorted_entities(state)
                    .into_iter()
                    .map(entity_ref_value)
                    .collect::<Result<Vec<_>>>()?,
            ),
        ),
        integer_field(
            "relation_count",
            usize_to_i64("relation_count", state.relations().len())?,
        )?,
        (
            "relations".to_owned(),
            CanonicalValue::Array(
                sorted_relations(state)
                    .into_iter()
                    .map(relation_ref_value)
                    .collect::<Result<Vec<_>>>()?,
            ),
        ),
    ])
}

fn sorted_entities(state: &WorkState) -> Vec<(EntityId, EntityVersionId)> {
    let mut entries = state.entities().to_vec();
    entries.sort_by_key(|(entity_id, _)| entity_id.raw_bytes());
    entries
}

fn sorted_relations(state: &WorkState) -> Vec<(RelationId, RelationVersionId)> {
    let mut entries = state.relations().to_vec();
    entries.sort_by_key(|(relation_id, _)| relation_id.raw_bytes());
    entries
}

fn entity_ref_value(entry: (EntityId, EntityVersionId)) -> Result<CanonicalValue> {
    let (entity_id, entity_version_id) = entry;
    CanonicalValue::object(vec![
        string_field("entity_id", entity_id.to_string()),
        string_field("entity_version_id", entity_version_id.to_string()),
    ])
}

fn relation_ref_value(entry: (RelationId, RelationVersionId)) -> Result<CanonicalValue> {
    let (relation_id, relation_version_id) = entry;
    CanonicalValue::object(vec![
        string_field("relation_id", relation_id.to_string()),
        string_field("relation_version_id", relation_version_id.to_string()),
    ])
}

fn entity_version_ref_value(entity_version: &BundleEntityVersionRef) -> Result<CanonicalValue> {
    CanonicalValue::object(vec![
        string_field("entity_id", entity_version.entity_id.to_string()),
        string_field(
            "entity_version_id",
            entity_version.entity_version_id.to_string(),
        ),
        string_field("entity_kind", entity_version.entity_kind.clone()),
        integer_field("state_schema_version", entity_version.state_schema_version)?,
        string_field("state_digest", entity_version.state_digest.to_string()),
        string_field(
            "state_json_digest",
            entity_version.state_json_digest.to_string(),
        ),
        integer_field(
            "state_json_size_bytes",
            entity_version.state_json_size_bytes,
        )?,
    ])
}

fn acceptance_criterion_identity_ref_value(
    identity: &BundleAcceptanceCriterionIdentityRef,
) -> Result<CanonicalValue> {
    validate_portable_text(
        "acceptance_criterion_identity.local_key",
        &identity.local_key,
    )
    .map_err(WorkVcsError::QueryInvalid)?;
    CanonicalValue::object(vec![
        string_field("entity_id", identity.entity_id.to_string()),
        string_field("owner_entity_id", identity.owner_entity_id.to_string()),
        string_field("local_key", identity.local_key.clone()),
    ])
}

fn verification_requirement_identity_ref_value(
    identity: &BundleVerificationRequirementIdentityRef,
) -> Result<CanonicalValue> {
    validate_portable_text(
        "verification_requirement_identity.local_key",
        &identity.local_key,
    )
    .map_err(WorkVcsError::QueryInvalid)?;
    CanonicalValue::object(vec![
        string_field("entity_id", identity.entity_id.to_string()),
        string_field("owner_entity_id", identity.owner_entity_id.to_string()),
        string_field("local_key", identity.local_key.clone()),
    ])
}

fn relation_version_ref_value(
    relation_version: &BundleRelationVersionRef,
) -> Result<CanonicalValue> {
    CanonicalValue::object(vec![
        string_field("relation_id", relation_version.relation_id.to_string()),
        string_field(
            "relation_version_id",
            relation_version.relation_version_id.to_string(),
        ),
        string_field("relation_type", relation_version.relation_type.clone()),
        string_field(
            "source_object_id",
            relation_version.source_object_id.clone(),
        ),
        string_field(
            "target_object_id",
            relation_version.target_object_id.clone(),
        ),
        string_field(
            "relation_discriminator",
            relation_version.relation_discriminator.clone(),
        ),
        integer_field(
            "state_schema_version",
            relation_version.state_schema_version,
        )?,
        string_field("state_digest", relation_version.state_digest.to_string()),
        string_field(
            "metadata_json_digest",
            relation_version.metadata_json_digest.to_string(),
        ),
        integer_field(
            "metadata_json_size_bytes",
            relation_version.metadata_json_size_bytes,
        )?,
    ])
}

fn knowledge_space_ref_value(knowledge_space: &BundleKnowledgeSpaceRef) -> Result<CanonicalValue> {
    CanonicalValue::object(vec![
        string_field(
            "knowledge_space_id",
            knowledge_space.knowledge_space_id.to_string(),
        ),
        string_field("name", knowledge_space.name.clone()),
        integer_field("created_at_us", knowledge_space.created_at_us)?,
    ])
}

fn knowledge_exposure_ref_value(exposure: &BundleKnowledgeExposureRef) -> Result<CanonicalValue> {
    CanonicalValue::object(vec![
        string_field("exposure_id", exposure.exposure_id.to_string()),
        string_field(
            "knowledge_space_id",
            exposure.knowledge_space_id.to_string(),
        ),
        integer_field("created_at_us", exposure.created_at_us)?,
    ])
}

fn knowledge_exposure_local_source_ref_value(
    source: &BundleKnowledgeExposureLocalSourceRef,
) -> Result<CanonicalValue> {
    CanonicalValue::object(vec![
        string_field("exposure_id", source.exposure_id.to_string()),
        string_field(
            "source_workspace_id",
            source.source_workspace_id.to_string(),
        ),
        string_field(
            "source_knowledge_entity_id",
            source.source_knowledge_entity_id.to_string(),
        ),
        string_field(
            "source_knowledge_entity_version_id",
            source.source_knowledge_entity_version_id.to_string(),
        ),
        string_field(
            "source_knowledge_state_digest",
            source.source_knowledge_state_digest.to_string(),
        ),
        string_field(
            "source_knowledge_state_json_digest",
            source.source_knowledge_state_json_digest.to_string(),
        ),
        integer_field(
            "source_knowledge_state_json_size_bytes",
            source.source_knowledge_state_json_size_bytes,
        )?,
    ])
}

fn knowledge_exposure_transition_ref_value(
    transition: &BundleKnowledgeExposureTransitionRef,
) -> Result<CanonicalValue> {
    CanonicalValue::object(vec![
        string_field("exposure_id", transition.exposure_id.to_string()),
        string_field("transition_id", transition.transition_id.to_string()),
        optional_display_field("previous_transition_id", transition.previous_transition_id),
        string_field("lifecycle_status", transition.lifecycle_status.clone()),
        integer_field("changed_at_us", transition.changed_at_us)?,
        optional_display_field("event_id", transition.event_id),
        string_field("detail_digest", transition.detail_digest.to_string()),
        integer_field("detail_size_bytes", transition.detail_size_bytes)?,
    ])
}

fn knowledge_exposure_source_status_ref_value(
    source_status: &BundleKnowledgeExposureSourceStatusRef,
) -> Result<CanonicalValue> {
    CanonicalValue::object(vec![
        string_field("exposure_id", source_status.exposure_id.to_string()),
        string_field("source_status", source_status.source_status.clone()),
        integer_field("checked_at_us", source_status.checked_at_us)?,
        string_field("detail_digest", source_status.detail_digest.to_string()),
        integer_field("detail_size_bytes", source_status.detail_size_bytes)?,
    ])
}

fn entity_membership_change_ref_value(
    change: &BundleEntityMembershipChangeRef,
) -> Result<CanonicalValue> {
    CanonicalValue::object(vec![
        string_field("changeset_id", change.changeset_id.to_string()),
        string_field("operation_id", change.operation_id.to_string()),
        integer_field("ordinal", change.ordinal)?,
        string_field("entity_id", change.entity_id.to_string()),
        optional_entity_version_field("before_entity_version_id", change.before_entity_version_id),
        optional_entity_version_field("after_entity_version_id", change.after_entity_version_id),
        string_field("field_delta_digest", change.field_delta_digest.to_string()),
        integer_field("field_delta_size_bytes", change.field_delta_size_bytes)?,
    ])
}

fn relation_membership_change_ref_value(
    change: &BundleRelationMembershipChangeRef,
) -> Result<CanonicalValue> {
    CanonicalValue::object(vec![
        string_field("changeset_id", change.changeset_id.to_string()),
        string_field("operation_id", change.operation_id.to_string()),
        integer_field("ordinal", change.ordinal)?,
        string_field("relation_id", change.relation_id.to_string()),
        optional_relation_version_field(
            "before_relation_version_id",
            change.before_relation_version_id,
        ),
        optional_relation_version_field(
            "after_relation_version_id",
            change.after_relation_version_id,
        ),
        string_field("field_delta_digest", change.field_delta_digest.to_string()),
        integer_field("field_delta_size_bytes", change.field_delta_size_bytes)?,
    ])
}

fn optional_entity_version_field(
    name: &str,
    value: Option<EntityVersionId>,
) -> (String, CanonicalValue) {
    (
        name.to_owned(),
        value
            .map(|id| CanonicalValue::String(id.to_string()))
            .unwrap_or(CanonicalValue::Null),
    )
}

fn optional_relation_version_field(
    name: &str,
    value: Option<RelationVersionId>,
) -> (String, CanonicalValue) {
    (
        name.to_owned(),
        value
            .map(|id| CanonicalValue::String(id.to_string()))
            .unwrap_or(CanonicalValue::Null),
    )
}

fn checkpoint_candidate_value(checkpoint: &BundleCheckpointCandidate) -> Result<CanonicalValue> {
    CanonicalValue::object(vec![
        string_field("checkpoint_id", checkpoint.checkpoint_id.to_string()),
        string_field("content_digest", checkpoint.content_digest.to_string()),
        integer_field("content_size_bytes", checkpoint.content_size_bytes)?,
        integer_field(
            "checkpoint_format_version",
            checkpoint.checkpoint_format_version,
        )?,
        (
            "media_type".to_owned(),
            checkpoint
                .media_type
                .clone()
                .map(CanonicalValue::String)
                .unwrap_or(CanonicalValue::Null),
        ),
        string_field("usability_state", checkpoint.usability_state.clone()),
    ])
}

fn payload_index_value(
    manifest: &BundleExportManifest,
    manifest_bytes: &[u8],
    payload_files: &[BundlePayloadFile],
    payload_references: &[BundlePayloadReference],
) -> Result<CanonicalValue> {
    CanonicalValue::object(vec![
        string_field("bundle_payload_index_profile", BUNDLE_PAYLOAD_INDEX_PROFILE),
        integer_field("bundle_payload_index_version", BUNDLE_PAYLOAD_INDEX_VERSION)?,
        (
            "manifest".to_owned(),
            CanonicalValue::object(vec![
                string_field("path", "manifest.json"),
                string_field("digest", manifest.manifest_digest.to_string()),
                integer_field(
                    "size_bytes",
                    usize_to_i64("manifest_size_bytes", manifest_bytes.len())?,
                )?,
            ])?,
        ),
        (
            "target".to_owned(),
            CanonicalValue::object(vec![
                string_field("workspace_id", manifest.workspace_id.to_string()),
                string_field("commit_id", manifest.commit_id.to_string()),
                string_field("state_digest", manifest.state_digest.to_string()),
            ])?,
        ),
        integer_field(
            "payload_count",
            usize_to_i64("payload_count", payload_files.len())?,
        )?,
        integer_field(
            "reference_count",
            usize_to_i64("reference_count", payload_references.len())?,
        )?,
        (
            "payloads".to_owned(),
            CanonicalValue::Array(
                payload_files
                    .iter()
                    .map(payload_file_value)
                    .collect::<Result<Vec<_>>>()?,
            ),
        ),
        (
            "references".to_owned(),
            CanonicalValue::Array(
                payload_references
                    .iter()
                    .map(payload_reference_value)
                    .collect::<Result<Vec<_>>>()?,
            ),
        ),
    ])
}

fn payload_file_value(payload: &BundlePayloadFile) -> Result<CanonicalValue> {
    CanonicalValue::object(vec![
        string_field("path", payload.relative_path.clone()),
        string_field("content_digest", payload.content_digest.to_string()),
        integer_field("size_bytes", payload.size_bytes)?,
        string_field("media_type", payload.media_type.clone()),
    ])
}

fn payload_reference_value(reference: &BundlePayloadReference) -> Result<CanonicalValue> {
    CanonicalValue::object(vec![
        string_field("role", reference.role.clone()),
        string_field("path", reference.relative_path.clone()),
        string_field("content_digest", reference.content_digest.to_string()),
        integer_field("size_bytes", reference.size_bytes)?,
        ("owner".to_owned(), reference.owner.clone()),
    ])
}

fn bundle_import_attempt_detail_json(preflight: &BundleImportPreflightResult) -> Result<String> {
    let problem = preflight
        .problem
        .clone()
        .map(CanonicalValue::String)
        .unwrap_or(CanonicalValue::Null);
    let value = CanonicalValue::object(vec![
        ("valid".to_owned(), CanonicalValue::Bool(preflight.valid)),
        (
            "format_compatible".to_owned(),
            CanonicalValue::Bool(preflight.format_compatible),
        ),
        optional_display_field("source_store_id", preflight.source_store_id),
        optional_display_field("target_workspace_id", preflight.target_workspace_id),
        optional_display_field("target_commit_id", preflight.target_commit_id),
        optional_display_field("target_state_digest", preflight.target_state_digest),
        string_field(
            "source_store_relation",
            preflight.source_store_relation.clone(),
        ),
        (
            "incoming_commit_present".to_owned(),
            CanonicalValue::Bool(preflight.incoming_commit_present),
        ),
        (
            "import_required".to_owned(),
            CanonicalValue::Bool(preflight.import_required),
        ),
        (
            "can_apply".to_owned(),
            CanonicalValue::Bool(preflight.can_apply),
        ),
        string_field("action", preflight.action.clone()),
        string_field("manifest_digest", preflight.manifest_digest.to_string()),
        string_field(
            "payload_index_digest",
            preflight.payload_index_digest.to_string(),
        ),
        integer_field(
            "payload_files",
            usize_to_i64("payload_files", preflight.payload_files)?,
        )?,
        integer_field(
            "payload_references",
            usize_to_i64("payload_references", preflight.payload_references)?,
        )?,
        integer_field(
            "exported_branch_heads",
            usize_to_i64("exported_branch_heads", preflight.exported_branch_heads)?,
        )?,
        integer_field(
            "branch_heads_already_present",
            usize_to_i64(
                "branch_heads_already_present",
                preflight.branch_heads_already_present,
            )?,
        )?,
        integer_field(
            "branch_heads_missing",
            usize_to_i64("branch_heads_missing", preflight.branch_heads_missing)?,
        )?,
        integer_field(
            "branch_heads_fast_forward",
            usize_to_i64(
                "branch_heads_fast_forward",
                preflight.branch_heads_fast_forward,
            )?,
        )?,
        integer_field(
            "branch_heads_diverged",
            usize_to_i64("branch_heads_diverged", preflight.branch_heads_diverged)?,
        )?,
        ("problem".to_owned(), problem),
    ])?;
    let bytes = canonical_bytes(&value)?;
    String::from_utf8(bytes).map_err(|error| {
        WorkVcsError::CanonicalEncodingInvalid(format!(
            "bundle import attempt detail JSON was not UTF-8: {error}"
        ))
    })
}

fn bundle_import_apply_detail_json(
    preflight: &BundleImportPreflightResult,
    imported_commits: usize,
    imported_entity_versions: usize,
    imported_acceptance_criterion_identities: usize,
    imported_verification_requirement_identities: usize,
    imported_relation_versions: usize,
    updated_branch_heads: usize,
) -> Result<String> {
    let value = CanonicalValue::object(vec![
        ("valid".to_owned(), CanonicalValue::Bool(preflight.valid)),
        (
            "format_compatible".to_owned(),
            CanonicalValue::Bool(preflight.format_compatible),
        ),
        optional_display_field("source_store_id", preflight.source_store_id),
        optional_display_field("target_workspace_id", preflight.target_workspace_id),
        optional_display_field("target_commit_id", preflight.target_commit_id),
        optional_display_field("target_state_digest", preflight.target_state_digest),
        string_field(
            "source_store_relation",
            preflight.source_store_relation.clone(),
        ),
        (
            "incoming_commit_present".to_owned(),
            CanonicalValue::Bool(preflight.incoming_commit_present),
        ),
        (
            "import_required".to_owned(),
            CanonicalValue::Bool(preflight.import_required),
        ),
        (
            "can_apply".to_owned(),
            CanonicalValue::Bool(preflight.can_apply),
        ),
        string_field("action", preflight.action.clone()),
        integer_field(
            "imported_commits",
            usize_to_i64("imported_commits", imported_commits)?,
        )?,
        integer_field(
            "imported_entity_versions",
            usize_to_i64("imported_entity_versions", imported_entity_versions)?,
        )?,
        integer_field(
            "imported_acceptance_criterion_identities",
            usize_to_i64(
                "imported_acceptance_criterion_identities",
                imported_acceptance_criterion_identities,
            )?,
        )?,
        integer_field(
            "imported_verification_requirement_identities",
            usize_to_i64(
                "imported_verification_requirement_identities",
                imported_verification_requirement_identities,
            )?,
        )?,
        integer_field(
            "imported_relation_versions",
            usize_to_i64("imported_relation_versions", imported_relation_versions)?,
        )?,
        integer_field(
            "updated_branch_heads",
            usize_to_i64("updated_branch_heads", updated_branch_heads)?,
        )?,
    ])?;
    let bytes = canonical_bytes(&value)?;
    String::from_utf8(bytes).map_err(|error| {
        WorkVcsError::CanonicalEncodingInvalid(format!(
            "bundle import apply detail JSON was not UTF-8: {error}"
        ))
    })
}

fn insert_import_attempt_outcome(
    transaction: &Transaction<'_>,
    row: ImportAttemptOutcomeInsert<'_>,
) -> Result<()> {
    let import_id_bytes = row.import_id.raw_bytes();
    let source_store_id_bytes = row.source_store_id.raw_bytes();
    let bundle_digest_bytes = *row.bundle_digest.as_bytes();
    let origin_session_id_bytes = row.origin_session_id.map(|id| id.raw_bytes());
    transaction
        .execute(
            "INSERT INTO import_attempt(
                import_id,
                source_store_id,
                bundle_digest,
                import_profile,
                origin_session_id,
                started_at_us
             )
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                &import_id_bytes[..],
                &source_store_id_bytes[..],
                &bundle_digest_bytes[..],
                row.import_profile,
                origin_session_id_bytes.as_ref().map(|bytes| &bytes[..]),
                row.now_us
            ],
        )
        .map_err(storage_error)?;
    transaction
        .execute(
            "INSERT INTO import_attempt_outcome(
                import_id,
                outcome,
                completed_at_us,
                detail_json
             )
             VALUES (?1, ?2, ?3, ?4)",
            params![
                &import_id_bytes[..],
                row.outcome,
                row.now_us,
                row.detail_json
            ],
        )
        .map_err(storage_error)?;
    Ok(())
}

fn apply_entity_versions(
    transaction: &Transaction<'_>,
    document: &BundleSameStoreApplyDocument,
    payload_lookup: &BundlePayloadLookup,
    now_us: i64,
) -> Result<usize> {
    let mut imported = 0;
    for entity_version in &document.entity_versions {
        if !same_store_entity_kind_supported(&entity_version.entity_kind) {
            return Err(WorkVcsError::QueryInvalid(format!(
                "bundle entity {} kind {} is outside same-Store typed-entity apply scope",
                entity_version.entity_id, entity_version.entity_kind
            )));
        }
        let owner = CanonicalValue::object(vec![
            string_field("entity_id", entity_version.entity_id.to_string()),
            string_field(
                "entity_version_id",
                entity_version.entity_version_id.to_string(),
            ),
        ])?;
        let state_json = payload_lookup.required_json(
            "entity_version_state",
            owner,
            Some(entity_version.state_json_digest),
            Some(entity_version.state_json_size_bytes),
        )?;
        let state_value =
            validate_canonical_json_value("bundle entity_version.state_json", &state_json)?;
        if entity_version_digest(&state_value)? != entity_version.state_digest {
            return Err(WorkVcsError::ImmutableImportInvalid(format!(
                "bundle EntityVersion {} state digest does not match payload",
                entity_version.entity_version_id
            )));
        }

        ensure_entity_identity(
            transaction,
            document.workspace_id,
            entity_version.entity_id,
            &entity_version.entity_kind,
            entity_created_at_us(document, entity_version.entity_id).unwrap_or(now_us),
        )?;
        if ensure_entity_version_row(transaction, entity_version, &state_json)? {
            imported += 1;
        }
    }
    Ok(imported)
}

fn same_store_entity_kind_supported(entity_kind: &str) -> bool {
    matches!(
        entity_kind,
        TASK_ENTITY_KIND | ACCEPTANCE_CRITERION_ENTITY_KIND | VERIFICATION_REQUIREMENT_ENTITY_KIND
    )
}

fn apply_acceptance_criterion_identities(
    transaction: &Transaction<'_>,
    document: &BundleSameStoreApplyDocument,
) -> Result<usize> {
    let mut imported = 0;
    for identity in &document.acceptance_criterion_identities {
        require_entity_kind(
            transaction,
            identity.entity_id,
            ACCEPTANCE_CRITERION_ENTITY_KIND,
        )?;
        require_entity_kind(transaction, identity.owner_entity_id, TASK_ENTITY_KIND)?;
        if ensure_acceptance_criterion_identity_row(transaction, identity)? {
            imported += 1;
        }
    }
    Ok(imported)
}

fn apply_verification_requirement_identities(
    transaction: &Transaction<'_>,
    document: &BundleSameStoreApplyDocument,
) -> Result<usize> {
    let mut imported = 0;
    for identity in &document.verification_requirement_identities {
        require_entity_kind(
            transaction,
            identity.entity_id,
            VERIFICATION_REQUIREMENT_ENTITY_KIND,
        )?;
        require_entity_kind(
            transaction,
            identity.owner_entity_id,
            ACCEPTANCE_CRITERION_ENTITY_KIND,
        )?;
        if ensure_verification_requirement_identity_row(transaction, identity)? {
            imported += 1;
        }
    }
    Ok(imported)
}

fn apply_relation_versions(
    transaction: &Transaction<'_>,
    document: &BundleSameStoreApplyDocument,
    payload_lookup: &BundlePayloadLookup,
    now_us: i64,
) -> Result<usize> {
    for relation_version in &document.relation_versions {
        ensure_relation_object_identity(
            transaction,
            relation_version.relation_id,
            relation_created_at_us(document, relation_version.relation_id).unwrap_or(now_us),
        )?;
    }

    let mut imported = 0;
    for relation_version in &document.relation_versions {
        let owner = CanonicalValue::object(vec![
            string_field("relation_id", relation_version.relation_id.to_string()),
            string_field(
                "relation_version_id",
                relation_version.relation_version_id.to_string(),
            ),
        ])?;
        let metadata_json = payload_lookup.required_json(
            "relation_version_metadata",
            owner,
            Some(relation_version.metadata_json_digest),
            Some(relation_version.metadata_json_size_bytes),
        )?;
        let metadata_value =
            validate_canonical_json_value("bundle relation_version.metadata_json", &metadata_json)?;
        if relation_version_digest(&metadata_value)? != relation_version.state_digest {
            return Err(WorkVcsError::ImmutableImportInvalid(format!(
                "bundle RelationVersion {} state digest does not match payload",
                relation_version.relation_version_id
            )));
        }
        ensure_relation_row(transaction, document.workspace_id, relation_version)?;
        if ensure_relation_version_row(transaction, relation_version, &metadata_json)? {
            imported += 1;
        }
    }
    Ok(imported)
}

fn apply_commit_closure(
    transaction: &Transaction<'_>,
    document: &BundleSameStoreApplyDocument,
    payload_lookup: &BundlePayloadLookup,
) -> Result<usize> {
    let mut commits = document.commits.iter().collect::<Vec<_>>();
    commits.sort_by(|left, right| {
        left.committed_at_us
            .cmp(&right.committed_at_us)
            .then_with(|| left.commit_id.cmp(&right.commit_id))
    });

    let mut imported = 0;
    for commit in commits {
        if local_commit_present(transaction, commit.commit_id, commit.state_digest)? {
            continue;
        }
        if commit.workspace_id != document.workspace_id {
            return Err(WorkVcsError::QueryInvalid(format!(
                "bundle commit {} workspace does not match target workspace",
                commit.commit_id
            )));
        }
        let entity_membership_changes = document
            .entity_membership_changes
            .iter()
            .filter(|change| change.changeset_id == commit.changeset_id)
            .collect::<Vec<_>>();
        let relation_membership_changes = document
            .relation_membership_changes
            .iter()
            .filter(|change| change.changeset_id == commit.changeset_id)
            .collect::<Vec<_>>();
        let membership_count =
            i64::try_from(entity_membership_changes.len() + relation_membership_changes.len())
                .map_err(|_| {
                    WorkVcsError::QueryInvalid(
                        "bundle membership change count does not fit i64".to_owned(),
                    )
                })?;
        if membership_count != commit.change_operation_count {
            return Err(WorkVcsError::QueryInvalid(format!(
                "bundle commit {} operation count does not match membership changes",
                commit.commit_id
            )));
        }

        let changeset_owner = CanonicalValue::object(vec![
            string_field("commit_id", commit.commit_id.to_string()),
            string_field("changeset_id", commit.changeset_id.to_string()),
        ])?;
        let changeset_operation_payload_json = payload_lookup.required_json(
            "changeset_operation_payload",
            changeset_owner.clone(),
            Some(commit.operation_payload_digest),
            None,
        )?;
        let rationale_json = payload_lookup.required_json(
            "changeset_rationale",
            changeset_owner,
            Some(commit.rationale_digest),
            None,
        )?;
        insert_changeset_row(
            transaction,
            document.workspace_id,
            commit,
            &changeset_operation_payload_json,
            &rationale_json,
        )?;

        for change in entity_membership_changes {
            insert_task_change_operation_and_membership(
                transaction,
                commit,
                change,
                payload_lookup,
            )?;
        }
        for change in relation_membership_changes {
            insert_relation_change_operation_and_membership(
                transaction,
                commit,
                change,
                payload_lookup,
            )?;
        }
        insert_workstate_commit_row(transaction, commit)?;
        for parent in &commit.parents {
            insert_commit_parent_row(transaction, commit.commit_id, parent)?;
        }
        imported += 1;
    }
    Ok(imported)
}

fn apply_same_store_branch_fast_forwards(
    transaction: &Transaction<'_>,
    document: &BundleSameStoreApplyDocument,
    now_us: i64,
) -> Result<usize> {
    let mut updated = 0;
    for branch in &document.exported_branch_heads {
        let (local_workspace_id, local_head_commit_id, local_head_state_digest) =
            load_branch_head_for_apply(transaction, branch.branch_id)?;
        if local_workspace_id != document.workspace_id
            || branch.workspace_id != document.workspace_id
        {
            return Err(WorkVcsError::QueryInvalid(format!(
                "bundle branch {} workspace does not match target workspace",
                branch.branch_id
            )));
        }
        if branch.head_commit_id == local_head_commit_id {
            continue;
        }
        if manifest_commit_digest_for_document(document, local_head_commit_id)
            != Some(local_head_state_digest)
            || !manifest_commit_is_descendant_for_document(
                document,
                branch.head_commit_id,
                local_head_commit_id,
            )
        {
            return Err(WorkVcsError::QueryInvalid(format!(
                "bundle branch {} is no longer a fast-forward",
                branch.branch_id
            )));
        }
        let branch_id_bytes = branch.branch_id.raw_bytes();
        let workspace_id_bytes = branch.workspace_id.raw_bytes();
        let old_head_bytes = local_head_commit_id.raw_bytes();
        let new_head_bytes = branch.head_commit_id.raw_bytes();
        let affected = transaction
            .execute(
                "UPDATE branch
                 SET head_commit_id = ?1
                 WHERE branch_id = ?2
                   AND workspace_id = ?3
                   AND head_commit_id = ?4",
                params![
                    &new_head_bytes[..],
                    &branch_id_bytes[..],
                    &workspace_id_bytes[..],
                    &old_head_bytes[..],
                ],
            )
            .map_err(storage_error)?;
        if affected != 1 {
            return Err(WorkVcsError::QueryInvalid(format!(
                "bundle branch {} fast-forward compare-and-swap failed",
                branch.branch_id
            )));
        }
        super::mark_branch_projection_not_materialized(transaction, branch.branch_id, now_us)?;
        updated += 1;
    }
    Ok(updated)
}

fn ensure_entity_identity(
    transaction: &Transaction<'_>,
    workspace_id: WorkspaceId,
    entity_id: EntityId,
    entity_kind: &str,
    created_at_us: i64,
) -> Result<()> {
    let entity_id_bytes = entity_id.raw_bytes();
    let existing_object_kind = transaction
        .query_row(
            "SELECT object_kind
             FROM object_identity
             WHERE object_id = ?1",
            params![&entity_id_bytes[..]],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(storage_error)?;
    match existing_object_kind {
        Some(object_kind) if object_kind == "entity" => {}
        Some(object_kind) => {
            return Err(WorkVcsError::ImmutableImportInvalid(format!(
                "object {} exists with kind {}, not entity",
                entity_id, object_kind
            )));
        }
        None => {
            transaction
                .execute(
                    "INSERT INTO object_identity(object_id, object_kind, created_at_us)
                     VALUES (?1, 'entity', ?2)",
                    params![&entity_id_bytes[..], created_at_us],
                )
                .map_err(storage_error)?;
        }
    }

    let existing_entity = transaction
        .query_row(
            "SELECT workspace_id, entity_kind
             FROM entity
             WHERE object_id = ?1",
            params![&entity_id_bytes[..]],
            |row| Ok((row.get::<_, Vec<u8>>(0)?, row.get::<_, String>(1)?)),
        )
        .optional()
        .map_err(storage_error)?;
    let workspace_id_bytes = workspace_id.raw_bytes();
    match existing_entity {
        Some((existing_workspace_id, existing_entity_kind)) => {
            let existing_workspace_id =
                decode_workspace_id("entity.workspace_id", existing_workspace_id)?;
            if existing_workspace_id != workspace_id || existing_entity_kind != entity_kind {
                return Err(WorkVcsError::ImmutableImportInvalid(format!(
                    "entity {} exists with different workspace or kind",
                    entity_id
                )));
            }
        }
        None => {
            transaction
                .execute(
                    "INSERT INTO entity(object_id, workspace_id, entity_kind)
                     VALUES (?1, ?2, ?3)",
                    params![&entity_id_bytes[..], &workspace_id_bytes[..], entity_kind],
                )
                .map_err(storage_error)?;
        }
    }
    Ok(())
}

fn require_entity_kind(
    transaction: &Transaction<'_>,
    entity_id: EntityId,
    expected_entity_kind: &str,
) -> Result<()> {
    let entity_id_bytes = entity_id.raw_bytes();
    let actual = transaction
        .query_row(
            "SELECT entity_kind
             FROM entity
             WHERE object_id = ?1",
            params![&entity_id_bytes[..]],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(storage_error)?
        .ok_or_else(|| {
            WorkVcsError::ImmutableImportInvalid(format!(
                "entity {entity_id} is missing before typed identity import"
            ))
        })?;
    if actual != expected_entity_kind {
        return Err(WorkVcsError::ImmutableImportInvalid(format!(
            "entity {entity_id} has kind {actual}, expected {expected_entity_kind}"
        )));
    }
    Ok(())
}

fn ensure_acceptance_criterion_identity_row(
    transaction: &Transaction<'_>,
    identity: &BundleAcceptanceCriterionIdentityRef,
) -> Result<bool> {
    ensure_typed_identity_row(
        transaction,
        "acceptance_criterion_identity",
        "AcceptanceCriterion",
        identity.entity_id,
        identity.owner_entity_id,
        &identity.local_key,
    )
}

fn ensure_verification_requirement_identity_row(
    transaction: &Transaction<'_>,
    identity: &BundleVerificationRequirementIdentityRef,
) -> Result<bool> {
    ensure_typed_identity_row(
        transaction,
        "verification_requirement_identity",
        "VerificationRequirement",
        identity.entity_id,
        identity.owner_entity_id,
        &identity.local_key,
    )
}

fn ensure_typed_identity_row(
    transaction: &Transaction<'_>,
    table_name: &str,
    label: &str,
    entity_id: EntityId,
    owner_entity_id: EntityId,
    local_key: &str,
) -> Result<bool> {
    validate_portable_text(&format!("{table_name}.local_key"), local_key)
        .map_err(WorkVcsError::ImmutableImportInvalid)?;
    let entity_id_bytes = entity_id.raw_bytes();
    let owner_entity_id_bytes = owner_entity_id.raw_bytes();
    let existing = transaction
        .query_row(
            &format!(
                "SELECT owner_entity_id, local_key
                 FROM {table_name}
                 WHERE entity_id = ?1"
            ),
            params![&entity_id_bytes[..]],
            |row| Ok((row.get::<_, Vec<u8>>(0)?, row.get::<_, String>(1)?)),
        )
        .optional()
        .map_err(storage_error)?;
    if let Some((existing_owner_entity_id, existing_local_key)) = existing {
        let existing_owner_entity_id = decode_entity_id(
            &format!("{table_name}.owner_entity_id"),
            existing_owner_entity_id,
        )?;
        if existing_owner_entity_id != owner_entity_id || existing_local_key != local_key {
            return Err(WorkVcsError::ImmutableImportInvalid(format!(
                "{label} identity for entity {entity_id} exists with different content"
            )));
        }
        return Ok(false);
    }

    let existing_for_local_key = transaction
        .query_row(
            &format!(
                "SELECT entity_id
                 FROM {table_name}
                 WHERE owner_entity_id = ?1
                   AND local_key = ?2"
            ),
            params![&owner_entity_id_bytes[..], local_key],
            |row| row.get::<_, Vec<u8>>(0),
        )
        .optional()
        .map_err(storage_error)?;
    if let Some(existing_entity_id) = existing_for_local_key {
        let existing_entity_id =
            decode_entity_id(&format!("{table_name}.entity_id"), existing_entity_id)?;
        return Err(WorkVcsError::ImmutableImportInvalid(format!(
            "{label} owner {owner_entity_id} local key {local_key:?} already belongs to entity {existing_entity_id}"
        )));
    }

    transaction
        .execute(
            &format!(
                "INSERT INTO {table_name}(entity_id, owner_entity_id, local_key)
                 VALUES (?1, ?2, ?3)"
            ),
            params![&entity_id_bytes[..], &owner_entity_id_bytes[..], local_key],
        )
        .map_err(storage_error)?;
    Ok(true)
}

fn ensure_relation_object_identity(
    transaction: &Transaction<'_>,
    relation_id: RelationId,
    created_at_us: i64,
) -> Result<()> {
    let relation_id_bytes = relation_id.raw_bytes();
    let existing_object_kind = transaction
        .query_row(
            "SELECT object_kind
             FROM object_identity
             WHERE object_id = ?1",
            params![&relation_id_bytes[..]],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(storage_error)?;
    match existing_object_kind {
        Some(object_kind) if object_kind == RELATION_OBJECT_KIND => Ok(()),
        Some(object_kind) => Err(WorkVcsError::ImmutableImportInvalid(format!(
            "object {} exists with kind {}, not relation",
            relation_id, object_kind
        ))),
        None => {
            transaction
                .execute(
                    "INSERT INTO object_identity(object_id, object_kind, created_at_us)
                     VALUES (?1, ?2, ?3)",
                    params![&relation_id_bytes[..], RELATION_OBJECT_KIND, created_at_us],
                )
                .map_err(storage_error)?;
            Ok(())
        }
    }
}

fn ensure_relation_row(
    transaction: &Transaction<'_>,
    workspace_id: WorkspaceId,
    relation_version: &BundleRelationVersionRef,
) -> Result<()> {
    let relation_id_bytes = relation_version.relation_id.raw_bytes();
    let workspace_id_bytes = workspace_id.raw_bytes();
    let source_object_id_bytes = parse_object_id_bytes(
        "bundle relation_version.source_object_id",
        &relation_version.source_object_id,
    )?;
    let target_object_id_bytes = parse_object_id_bytes(
        "bundle relation_version.target_object_id",
        &relation_version.target_object_id,
    )?;
    require_object_identity(
        transaction,
        "bundle relation source_object_id",
        &source_object_id_bytes,
    )?;
    require_object_identity(
        transaction,
        "bundle relation target_object_id",
        &target_object_id_bytes,
    )?;

    let existing = transaction
        .query_row(
            "SELECT workspace_id,
                    relation_type,
                    source_object_id,
                    target_object_id,
                    relation_discriminator
             FROM relation
             WHERE object_id = ?1",
            params![&relation_id_bytes[..]],
            |row| {
                Ok((
                    row.get::<_, Vec<u8>>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Vec<u8>>(2)?,
                    row.get::<_, Vec<u8>>(3)?,
                    row.get::<_, String>(4)?,
                ))
            },
        )
        .optional()
        .map_err(storage_error)?;
    if let Some((
        existing_workspace_id,
        existing_relation_type,
        existing_source_object_id,
        existing_target_object_id,
        existing_relation_discriminator,
    )) = existing
    {
        let existing_workspace_id =
            decode_workspace_id("relation.workspace_id", existing_workspace_id)?;
        let existing_source_object_id =
            decode_16("relation.source_object_id", existing_source_object_id)?;
        let existing_target_object_id =
            decode_16("relation.target_object_id", existing_target_object_id)?;
        if existing_workspace_id != workspace_id
            || existing_relation_type != relation_version.relation_type
            || existing_source_object_id != source_object_id_bytes
            || existing_target_object_id != target_object_id_bytes
            || existing_relation_discriminator != relation_version.relation_discriminator
        {
            return Err(WorkVcsError::ImmutableImportInvalid(format!(
                "relation {} exists with different content",
                relation_version.relation_id
            )));
        }
        return Ok(());
    }

    let existing_for_identity = transaction
        .query_row(
            "SELECT object_id
             FROM relation
             WHERE workspace_id = ?1
               AND relation_type = ?2
               AND source_object_id = ?3
               AND target_object_id = ?4
               AND relation_discriminator = ?5",
            params![
                &workspace_id_bytes[..],
                relation_version.relation_type,
                &source_object_id_bytes[..],
                &target_object_id_bytes[..],
                relation_version.relation_discriminator,
            ],
            |row| row.get::<_, Vec<u8>>(0),
        )
        .optional()
        .map_err(storage_error)?;
    if let Some(existing_relation_id) = existing_for_identity {
        let existing_relation_id = decode_relation_id("relation.object_id", existing_relation_id)?;
        return Err(WorkVcsError::ImmutableImportInvalid(format!(
            "relation identity already belongs to relation {existing_relation_id}, not {}",
            relation_version.relation_id
        )));
    }

    transaction
        .execute(
            "INSERT INTO relation(
                object_id,
                workspace_id,
                relation_type,
                source_object_id,
                target_object_id,
                relation_discriminator
             )
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                &relation_id_bytes[..],
                &workspace_id_bytes[..],
                relation_version.relation_type,
                &source_object_id_bytes[..],
                &target_object_id_bytes[..],
                relation_version.relation_discriminator,
            ],
        )
        .map_err(storage_error)?;
    Ok(())
}

fn require_object_identity(
    transaction: &Transaction<'_>,
    label: &str,
    object_id_bytes: &[u8; 16],
) -> Result<()> {
    let present = transaction
        .query_row(
            "SELECT 1
             FROM object_identity
             WHERE object_id = ?1",
            params![&object_id_bytes[..]],
            |_| Ok(()),
        )
        .optional()
        .map_err(storage_error)?;
    if present.is_some() {
        Ok(())
    } else {
        Err(WorkVcsError::ImmutableImportInvalid(format!(
            "{label} is missing from object_identity"
        )))
    }
}

fn ensure_relation_version_row(
    transaction: &Transaction<'_>,
    relation_version: &BundleRelationVersionRef,
    metadata_json: &str,
) -> Result<bool> {
    let relation_id_bytes = relation_version.relation_id.raw_bytes();
    let relation_version_id_bytes = relation_version.relation_version_id.raw_bytes();
    let existing = transaction
        .query_row(
            "SELECT relation_id, state_schema_version, metadata_json, state_digest
             FROM relation_version
             WHERE relation_version_id = ?1",
            params![&relation_version_id_bytes[..]],
            |row| {
                Ok((
                    row.get::<_, Vec<u8>>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, Vec<u8>>(3)?,
                ))
            },
        )
        .optional()
        .map_err(storage_error)?;
    if let Some((
        existing_relation_id,
        state_schema_version,
        existing_metadata_json,
        state_digest,
    )) = existing
    {
        let existing_relation_id =
            decode_relation_id("relation_version.relation_id", existing_relation_id)?;
        let state_digest = decode_digest("relation_version.state_digest", state_digest)?;
        if existing_relation_id != relation_version.relation_id
            || state_schema_version != relation_version.state_schema_version
            || existing_metadata_json != metadata_json
            || state_digest != relation_version.state_digest
        {
            return Err(WorkVcsError::ImmutableImportInvalid(format!(
                "RelationVersion {} exists with different content",
                relation_version.relation_version_id
            )));
        }
        return Ok(false);
    }

    let state_digest_bytes = *relation_version.state_digest.as_bytes();
    transaction
        .execute(
            "INSERT INTO relation_version(
                relation_version_id,
                relation_id,
                state_schema_version,
                metadata_json,
                state_digest
             )
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                &relation_version_id_bytes[..],
                &relation_id_bytes[..],
                relation_version.state_schema_version,
                metadata_json,
                &state_digest_bytes[..],
            ],
        )
        .map_err(storage_error)?;
    Ok(true)
}

fn ensure_entity_version_row(
    transaction: &Transaction<'_>,
    entity_version: &BundleEntityVersionRef,
    state_json: &str,
) -> Result<bool> {
    let entity_version_id_bytes = entity_version.entity_version_id.raw_bytes();
    let entity_id_bytes = entity_version.entity_id.raw_bytes();
    let existing = transaction
        .query_row(
            "SELECT entity_id, state_schema_version, state_json, state_digest
             FROM entity_version
             WHERE entity_version_id = ?1",
            params![&entity_version_id_bytes[..]],
            |row| {
                Ok((
                    row.get::<_, Vec<u8>>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, Vec<u8>>(3)?,
                ))
            },
        )
        .optional()
        .map_err(storage_error)?;
    if let Some((existing_entity_id, state_schema_version, existing_state_json, state_digest)) =
        existing
    {
        let existing_entity_id = decode_entity_id("entity_version.entity_id", existing_entity_id)?;
        let state_digest = decode_digest("entity_version.state_digest", state_digest)?;
        if existing_entity_id != entity_version.entity_id
            || state_schema_version != entity_version.state_schema_version
            || existing_state_json != state_json
            || state_digest != entity_version.state_digest
        {
            return Err(WorkVcsError::ImmutableImportInvalid(format!(
                "EntityVersion {} exists with different content",
                entity_version.entity_version_id
            )));
        }
        return Ok(false);
    }

    let state_digest_bytes = *entity_version.state_digest.as_bytes();
    transaction
        .execute(
            "INSERT INTO entity_version(
                entity_version_id,
                entity_id,
                state_schema_version,
                state_json,
                state_digest
             )
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                &entity_version_id_bytes[..],
                &entity_id_bytes[..],
                entity_version.state_schema_version,
                state_json,
                &state_digest_bytes[..],
            ],
        )
        .map_err(storage_error)?;
    Ok(true)
}

fn local_commit_present(
    transaction: &Transaction<'_>,
    commit_id: CommitId,
    expected_state_digest: Digest,
) -> Result<bool> {
    let commit_id_bytes = commit_id.raw_bytes();
    let existing = transaction
        .query_row(
            "SELECT state_digest
             FROM workstate_commit
             WHERE commit_id = ?1",
            params![&commit_id_bytes[..]],
            |row| row.get::<_, Vec<u8>>(0),
        )
        .optional()
        .map_err(storage_error)?;
    let Some(existing) = existing else {
        return Ok(false);
    };
    let state_digest = decode_digest("workstate_commit.state_digest", existing)?;
    if state_digest != expected_state_digest {
        return Err(WorkVcsError::ImmutableImportInvalid(format!(
            "local commit {} exists with a different state digest",
            commit_id
        )));
    }
    Ok(true)
}

fn insert_changeset_row(
    transaction: &Transaction<'_>,
    workspace_id: WorkspaceId,
    commit: &BundleCommitRef,
    operation_payload_json: &str,
    rationale_json: &str,
) -> Result<()> {
    validate_canonical_json_text(
        "bundle changeset operation_payload_json",
        operation_payload_json,
    )?;
    validate_canonical_json_text("bundle changeset rationale_json", rationale_json)?;
    let changeset_id_bytes = commit.changeset_id.raw_bytes();
    let workspace_id_bytes = workspace_id.raw_bytes();
    transaction
        .execute(
            "INSERT INTO changeset(
                changeset_id,
                workspace_id,
                operation_type,
                operation_schema_version,
                operation_payload_json,
                rationale_json,
                origin_session_id,
                created_at_us
             )
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, NULL, ?7)",
            params![
                &changeset_id_bytes[..],
                &workspace_id_bytes[..],
                commit.operation_type,
                commit.operation_schema_version,
                operation_payload_json,
                rationale_json,
                commit.changeset_created_at_us,
            ],
        )
        .map_err(storage_error)?;
    Ok(())
}

fn insert_task_change_operation_and_membership(
    transaction: &Transaction<'_>,
    commit: &BundleCommitRef,
    change: &BundleEntityMembershipChangeRef,
    payload_lookup: &BundlePayloadLookup,
) -> Result<()> {
    let operation_owner = CanonicalValue::object(vec![
        string_field("changeset_id", commit.changeset_id.to_string()),
        string_field("operation_id", change.operation_id.to_string()),
        integer_field("ordinal", change.ordinal)?,
        string_field("subject_family", "entity"),
        string_field("subject_object_id", change.entity_id.to_string()),
    ])?;
    let operation_payload_json =
        payload_lookup.required_json("change_operation_payload", operation_owner, None, None)?;
    validate_canonical_json_text(
        "bundle change_operation.operation_payload_json",
        &operation_payload_json,
    )?;

    let operation_id_bytes = change.operation_id.raw_bytes();
    let changeset_id_bytes = commit.changeset_id.raw_bytes();
    let entity_id_bytes = change.entity_id.raw_bytes();
    transaction
        .execute(
            "INSERT INTO change_operation(
                operation_id,
                changeset_id,
                ordinal,
                subject_family,
                subject_object_id,
                operation_payload_json
             )
             VALUES (?1, ?2, ?3, 'entity', ?4, ?5)",
            params![
                &operation_id_bytes[..],
                &changeset_id_bytes[..],
                change.ordinal,
                &entity_id_bytes[..],
                operation_payload_json,
            ],
        )
        .map_err(storage_error)?;

    let field_delta_owner = CanonicalValue::object(vec![
        string_field("changeset_id", commit.changeset_id.to_string()),
        string_field("operation_id", change.operation_id.to_string()),
        integer_field("ordinal", change.ordinal)?,
        string_field("entity_id", change.entity_id.to_string()),
    ])?;
    let field_delta_json = payload_lookup.required_json(
        "entity_membership_field_delta",
        field_delta_owner,
        Some(change.field_delta_digest),
        Some(change.field_delta_size_bytes),
    )?;
    validate_canonical_json_text(
        "bundle entity_membership_change.field_delta_json",
        &field_delta_json,
    )?;

    let before_entity_version_id_bytes = change.before_entity_version_id.map(|id| id.raw_bytes());
    let after_entity_version_id_bytes = change.after_entity_version_id.map(|id| id.raw_bytes());
    transaction
        .execute(
            "INSERT INTO entity_membership_change(
                operation_id,
                entity_id,
                before_entity_version_id,
                after_entity_version_id,
                field_delta_json
             )
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                &operation_id_bytes[..],
                &entity_id_bytes[..],
                before_entity_version_id_bytes
                    .as_ref()
                    .map(|bytes| &bytes[..]),
                after_entity_version_id_bytes
                    .as_ref()
                    .map(|bytes| &bytes[..]),
                field_delta_json,
            ],
        )
        .map_err(storage_error)?;
    Ok(())
}

fn insert_relation_change_operation_and_membership(
    transaction: &Transaction<'_>,
    commit: &BundleCommitRef,
    change: &BundleRelationMembershipChangeRef,
    payload_lookup: &BundlePayloadLookup,
) -> Result<()> {
    let operation_owner = CanonicalValue::object(vec![
        string_field("changeset_id", commit.changeset_id.to_string()),
        string_field("operation_id", change.operation_id.to_string()),
        integer_field("ordinal", change.ordinal)?,
        string_field("subject_family", "relation"),
        string_field("subject_object_id", change.relation_id.to_string()),
    ])?;
    let operation_payload_json =
        payload_lookup.required_json("change_operation_payload", operation_owner, None, None)?;
    validate_canonical_json_text(
        "bundle change_operation.operation_payload_json",
        &operation_payload_json,
    )?;

    let operation_id_bytes = change.operation_id.raw_bytes();
    let changeset_id_bytes = commit.changeset_id.raw_bytes();
    let relation_id_bytes = change.relation_id.raw_bytes();
    transaction
        .execute(
            "INSERT INTO change_operation(
                operation_id,
                changeset_id,
                ordinal,
                subject_family,
                subject_object_id,
                operation_payload_json
             )
             VALUES (?1, ?2, ?3, 'relation', ?4, ?5)",
            params![
                &operation_id_bytes[..],
                &changeset_id_bytes[..],
                change.ordinal,
                &relation_id_bytes[..],
                operation_payload_json,
            ],
        )
        .map_err(storage_error)?;

    let field_delta_owner = CanonicalValue::object(vec![
        string_field("changeset_id", commit.changeset_id.to_string()),
        string_field("operation_id", change.operation_id.to_string()),
        integer_field("ordinal", change.ordinal)?,
        string_field("relation_id", change.relation_id.to_string()),
    ])?;
    let field_delta_json = payload_lookup.required_json(
        "relation_membership_field_delta",
        field_delta_owner,
        Some(change.field_delta_digest),
        Some(change.field_delta_size_bytes),
    )?;
    validate_canonical_json_text(
        "bundle relation_membership_change.field_delta_json",
        &field_delta_json,
    )?;

    let before_relation_version_id_bytes =
        change.before_relation_version_id.map(|id| id.raw_bytes());
    let after_relation_version_id_bytes = change.after_relation_version_id.map(|id| id.raw_bytes());
    transaction
        .execute(
            "INSERT INTO relation_membership_change(
                operation_id,
                relation_id,
                before_relation_version_id,
                after_relation_version_id,
                field_delta_json
             )
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                &operation_id_bytes[..],
                &relation_id_bytes[..],
                before_relation_version_id_bytes
                    .as_ref()
                    .map(|bytes| &bytes[..]),
                after_relation_version_id_bytes
                    .as_ref()
                    .map(|bytes| &bytes[..]),
                field_delta_json,
            ],
        )
        .map_err(storage_error)?;
    Ok(())
}

fn insert_workstate_commit_row(
    transaction: &Transaction<'_>,
    commit: &BundleCommitRef,
) -> Result<()> {
    let commit_id_bytes = commit.commit_id.raw_bytes();
    let workspace_id_bytes = commit.workspace_id.raw_bytes();
    let changeset_id_bytes = commit.changeset_id.raw_bytes();
    let state_digest_bytes = *commit.state_digest.as_bytes();
    transaction
        .execute(
            "INSERT INTO workstate_commit(
                commit_id,
                workspace_id,
                changeset_id,
                commit_kind,
                state_digest,
                committed_at_us
             )
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                &commit_id_bytes[..],
                &workspace_id_bytes[..],
                &changeset_id_bytes[..],
                commit.commit_kind,
                &state_digest_bytes[..],
                commit.committed_at_us,
            ],
        )
        .map_err(storage_error)?;
    Ok(())
}

fn insert_commit_parent_row(
    transaction: &Transaction<'_>,
    commit_id: CommitId,
    parent: &BundleCommitParentRef,
) -> Result<()> {
    let commit_id_bytes = commit_id.raw_bytes();
    let parent_commit_id_bytes = parent.parent_commit_id.raw_bytes();
    transaction
        .execute(
            "INSERT INTO commit_parent(
                commit_id,
                parent_ordinal,
                parent_role,
                parent_commit_id
             )
             VALUES (?1, ?2, ?3, ?4)",
            params![
                &commit_id_bytes[..],
                parent.parent_ordinal,
                parent.parent_role,
                &parent_commit_id_bytes[..],
            ],
        )
        .map_err(storage_error)?;
    Ok(())
}

fn load_branch_head_for_apply(
    transaction: &Transaction<'_>,
    branch_id: BranchId,
) -> Result<(WorkspaceId, CommitId, Digest)> {
    let branch_id_bytes = branch_id.raw_bytes();
    let (workspace_id, head_commit_id, state_digest) = transaction
        .query_row(
            "SELECT branch.workspace_id,
                    branch.head_commit_id,
                    workstate_commit.state_digest
             FROM branch
             JOIN workstate_commit
               ON workstate_commit.workspace_id = branch.workspace_id
              AND workstate_commit.commit_id = branch.head_commit_id
             WHERE branch.branch_id = ?1",
            params![&branch_id_bytes[..]],
            |row| {
                Ok((
                    row.get::<_, Vec<u8>>(0)?,
                    row.get::<_, Vec<u8>>(1)?,
                    row.get::<_, Vec<u8>>(2)?,
                ))
            },
        )
        .optional()
        .map_err(storage_error)?
        .ok_or_else(|| WorkVcsError::QueryInvalid(format!("branch {branch_id} does not exist")))?;
    Ok((
        decode_workspace_id("branch.workspace_id", workspace_id)?,
        decode_commit_id("branch.head_commit_id", head_commit_id)?,
        decode_digest("workstate_commit.state_digest", state_digest)?,
    ))
}

fn entity_created_at_us(
    document: &BundleSameStoreApplyDocument,
    entity_id: EntityId,
) -> Option<i64> {
    document
        .entity_membership_changes
        .iter()
        .filter_map(|change| {
            (change.entity_id == entity_id && change.before_entity_version_id.is_none())
                .then(|| {
                    document.commits.iter().find_map(|commit| {
                        (commit.changeset_id == change.changeset_id)
                            .then_some(commit.changeset_created_at_us)
                    })
                })
                .flatten()
        })
        .min()
}

fn relation_created_at_us(
    document: &BundleSameStoreApplyDocument,
    relation_id: RelationId,
) -> Option<i64> {
    document
        .relation_membership_changes
        .iter()
        .filter_map(|change| {
            (change.relation_id == relation_id && change.before_relation_version_id.is_none())
                .then(|| {
                    document.commits.iter().find_map(|commit| {
                        (commit.changeset_id == change.changeset_id)
                            .then_some(commit.changeset_created_at_us)
                    })
                })
                .flatten()
        })
        .min()
}

fn manifest_commit_digest_for_document(
    document: &BundleSameStoreApplyDocument,
    commit_id: CommitId,
) -> Option<Digest> {
    document
        .commits
        .iter()
        .find_map(|commit| (commit.commit_id == commit_id).then_some(commit.state_digest))
}

fn manifest_commit_is_descendant_for_document(
    document: &BundleSameStoreApplyDocument,
    descendant: CommitId,
    ancestor: CommitId,
) -> bool {
    if descendant == ancestor {
        return true;
    }
    let parents_by_commit = document
        .commits
        .iter()
        .map(|commit| (commit.commit_id, commit.parents.as_slice()))
        .collect::<BTreeMap<_, _>>();
    let mut stack = vec![descendant];
    let mut seen = BTreeSet::new();
    while let Some(commit_id) = stack.pop() {
        if !seen.insert(commit_id) {
            continue;
        }
        let Some(parents) = parents_by_commit.get(&commit_id) else {
            continue;
        };
        for parent in *parents {
            if parent.parent_commit_id == ancestor {
                return true;
            }
            stack.push(parent.parent_commit_id);
        }
    }
    false
}

fn optional_display_field<T: std::fmt::Display>(
    name: &str,
    value: Option<T>,
) -> (String, CanonicalValue) {
    (
        name.to_owned(),
        value
            .map(|value| CanonicalValue::String(value.to_string()))
            .unwrap_or(CanonicalValue::Null),
    )
}

fn parse_fixed_point_canonical_json(
    label: &str,
    bytes: &[u8],
) -> std::result::Result<CanonicalValue, String> {
    let value =
        parse_canonical_json(bytes).map_err(|error| format!("{label} JSON is invalid: {error}"))?;
    let reencoded =
        canonical_bytes(&value).map_err(|error| format!("{label} encoding failed: {error}"))?;
    if reencoded != bytes {
        return Err(format!("{label} bytes are not fixed-point canonical JSON"));
    }
    Ok(value)
}

fn parse_bundle_manifest_summary(
    value: &CanonicalValue,
) -> std::result::Result<BundleManifestSummary, String> {
    expect_string_field(
        value,
        "bundle manifest",
        "bundle_manifest_profile",
        BUNDLE_EXPORT_MANIFEST_PROFILE,
    )?;
    expect_integer_field(
        value,
        "bundle manifest",
        "bundle_manifest_version",
        BUNDLE_EXPORT_MANIFEST_VERSION,
    )?;
    let store = object_field_ref(value, "bundle manifest", "store")?;
    let target = object_field_ref(value, "bundle manifest", "target")?;
    let commits = array_field_ref(value, "bundle manifest", "commit_closure")?
        .iter()
        .map(parse_bundle_commit_summary)
        .collect::<std::result::Result<Vec<_>, _>>()?;
    let exported_branch_heads = array_field_ref(value, "bundle manifest", "exported_branch_heads")?
        .iter()
        .map(parse_bundle_branch_head_summary)
        .collect::<std::result::Result<Vec<_>, _>>()?;
    let same_store_apply_supported = bundle_manifest_supports_same_store_apply(value)?;
    let manifest = BundleManifestSummary {
        source_store_id: parse_store_id_field(store, "bundle manifest store", "store_id")?,
        store_format_version: integer_field_value(
            store,
            "bundle manifest store",
            "store_format_version",
        )?,
        schema_version: integer_field_value(store, "bundle manifest store", "schema_version")?,
        object_store_format_version: integer_field_value(
            store,
            "bundle manifest store",
            "object_store_format_version",
        )?,
        id_scheme: string_field_value(store, "bundle manifest store", "id_scheme")?.to_owned(),
        digest_algorithm: string_field_value(store, "bundle manifest store", "digest_algorithm")?
            .to_owned(),
        canonical_json_profile: string_field_value(
            store,
            "bundle manifest store",
            "canonical_json_profile",
        )?
        .to_owned(),
        workspace_id: parse_workspace_id_field(target, "bundle manifest target", "workspace_id")?,
        commit_id: parse_commit_id_field(target, "bundle manifest target", "commit_id")?,
        state_digest: parse_digest_field(target, "bundle manifest target", "state_digest")?,
        commits,
        exported_branch_heads,
        same_store_apply_supported,
    };
    validate_bundle_manifest_summary_integrity(&manifest)?;
    Ok(manifest)
}

fn bundle_manifest_supports_same_store_apply(
    value: &CanonicalValue,
) -> std::result::Result<bool, String> {
    let unsupported_array_fields = [
        "knowledge_spaces",
        "knowledge_exposures",
        "knowledge_exposure_local_sources",
        "knowledge_exposure_transitions",
        "knowledge_exposure_source_statuses",
        "checkpoint_candidates",
    ];
    for field in unsupported_array_fields {
        if !array_field_ref(value, "bundle manifest", field)?.is_empty() {
            return Ok(false);
        }
    }
    let mut expected_acceptance_criterion_ids = BTreeSet::new();
    let mut expected_verification_requirement_ids = BTreeSet::new();
    let mut entity_kinds_by_id = BTreeMap::new();
    for entity_version in array_field_ref(value, "bundle manifest", "entity_versions")? {
        let entity_id = parse_entity_id_field(
            entity_version,
            "bundle manifest entity version",
            "entity_id",
        )?;
        let entity_kind = string_field_value(
            entity_version,
            "bundle manifest entity version",
            "entity_kind",
        )?;
        if let Some(existing_kind) = entity_kinds_by_id.insert(entity_id, entity_kind.to_owned())
            && existing_kind != entity_kind
        {
            return Err(format!(
                "bundle manifest entity {entity_id} has inconsistent entity kinds"
            ));
        }
        match entity_kind {
            TASK_ENTITY_KIND => {}
            ACCEPTANCE_CRITERION_ENTITY_KIND => {
                expected_acceptance_criterion_ids.insert(entity_id);
            }
            VERIFICATION_REQUIREMENT_ENTITY_KIND => {
                expected_verification_requirement_ids.insert(entity_id);
            }
            _ => return Ok(false),
        }
    }
    let acceptance_criterion_identities =
        optional_array_field_ref(value, "bundle manifest", "acceptance_criterion_identities")?
            .iter()
            .map(|identity| {
                parse_bundle_acceptance_criterion_identity_ref(identity)
                    .map_err(|error| error.to_string())
            })
            .collect::<std::result::Result<Vec<_>, _>>()?;
    let acceptance_criterion_ids =
        unique_acceptance_criterion_identity_ids(&acceptance_criterion_identities)
            .map_err(|error| error.to_string())?;
    let verification_requirement_identities = optional_array_field_ref(
        value,
        "bundle manifest",
        "verification_requirement_identities",
    )?
    .iter()
    .map(|identity| {
        parse_bundle_verification_requirement_identity_ref(identity)
            .map_err(|error| error.to_string())
    })
    .collect::<std::result::Result<Vec<_>, _>>()?;
    let verification_requirement_ids =
        unique_verification_requirement_identity_ids(&verification_requirement_identities)
            .map_err(|error| error.to_string())?;
    Ok(
        acceptance_criterion_ids == expected_acceptance_criterion_ids
            && verification_requirement_ids == expected_verification_requirement_ids,
    )
}

fn validate_bundle_manifest_summary_integrity(
    manifest: &BundleManifestSummary,
) -> std::result::Result<(), String> {
    let mut commits_by_id = BTreeMap::new();
    for commit in &manifest.commits {
        if commits_by_id
            .insert(commit.commit_id, commit.state_digest)
            .is_some()
        {
            return Err(format!(
                "bundle manifest commit {} appears more than once",
                commit.commit_id
            ));
        }
    }
    match commits_by_id.get(&manifest.commit_id) {
        Some(state_digest) if *state_digest == manifest.state_digest => {}
        Some(_) => {
            return Err(
                "bundle manifest target commit state digest does not match commit closure"
                    .to_owned(),
            );
        }
        None => {
            return Err("bundle manifest target commit is missing from commit closure".to_owned());
        }
    }

    for commit in &manifest.commits {
        for parent_commit_id in &commit.parent_commit_ids {
            if !commits_by_id.contains_key(parent_commit_id) {
                return Err(format!(
                    "bundle manifest commit {} parent {} is missing from commit closure",
                    commit.commit_id, parent_commit_id
                ));
            }
        }
    }

    let mut branch_ids = BTreeSet::new();
    for branch in &manifest.exported_branch_heads {
        if !branch_ids.insert(branch.branch_id) {
            return Err(format!(
                "bundle manifest branch {} appears more than once",
                branch.branch_id
            ));
        }
        if branch.workspace_id != manifest.workspace_id {
            return Err(format!(
                "bundle manifest branch {} workspace does not match target workspace",
                branch.branch_id
            ));
        }
        match commits_by_id.get(&branch.head_commit_id) {
            Some(state_digest) if *state_digest == branch.head_state_digest => {}
            Some(_) => {
                return Err(format!(
                    "bundle manifest branch {} head digest does not match commit closure",
                    branch.branch_id
                ));
            }
            None => {
                return Err(format!(
                    "bundle manifest branch {} head commit is missing from commit closure",
                    branch.branch_id
                ));
            }
        }
    }
    Ok(())
}

fn parse_bundle_commit_summary(
    value: &CanonicalValue,
) -> std::result::Result<BundleCommitSummary, String> {
    let parents = array_field_ref(value, "bundle manifest commit", "parents")?
        .iter()
        .map(|parent| {
            parse_commit_id_field(parent, "bundle manifest commit parent", "parent_commit_id")
        })
        .collect::<std::result::Result<Vec<_>, _>>()?;
    Ok(BundleCommitSummary {
        commit_id: parse_commit_id_field(value, "bundle manifest commit", "commit_id")?,
        state_digest: parse_digest_field(value, "bundle manifest commit", "state_digest")?,
        parent_commit_ids: parents,
    })
}

fn parse_bundle_branch_head_summary(
    value: &CanonicalValue,
) -> std::result::Result<BundleBranchHeadSummary, String> {
    let name = string_field_value(value, "bundle manifest branch head", "name")?.to_owned();
    validate_portable_text("bundle manifest branch head name", &name)?;
    let lifecycle_state =
        string_field_value(value, "bundle manifest branch head", "lifecycle_state")?.to_owned();
    validate_portable_text(
        "bundle manifest branch head lifecycle_state",
        &lifecycle_state,
    )?;
    let created_at_us = integer_field_value(value, "bundle manifest branch head", "created_at_us")?;
    if created_at_us <= 0 {
        return Err("bundle manifest branch head created_at_us must be positive".to_owned());
    }
    Ok(BundleBranchHeadSummary {
        workspace_id: parse_workspace_id_field(
            value,
            "bundle manifest branch head",
            "workspace_id",
        )?,
        branch_id: parse_branch_id_field(value, "bundle manifest branch head", "branch_id")?,
        name,
        head_commit_id: parse_commit_id_field(
            value,
            "bundle manifest branch head",
            "head_commit_id",
        )?,
        head_state_digest: parse_digest_field(
            value,
            "bundle manifest branch head",
            "head_state_digest",
        )?,
        lifecycle_state,
    })
}

fn parse_bundle_payload_index_summary(
    value: &CanonicalValue,
) -> std::result::Result<BundlePayloadIndexSummary, String> {
    expect_string_field(
        value,
        "bundle payload index",
        "bundle_payload_index_profile",
        BUNDLE_PAYLOAD_INDEX_PROFILE,
    )?;
    expect_integer_field(
        value,
        "bundle payload index",
        "bundle_payload_index_version",
        BUNDLE_PAYLOAD_INDEX_VERSION,
    )?;
    let manifest = object_field_ref(value, "bundle payload index", "manifest")?;
    let target = object_field_ref(value, "bundle payload index", "target")?;
    let payloads = array_field_ref(value, "bundle payload index", "payloads")?
        .iter()
        .map(parse_bundle_payload_file_ref)
        .collect::<std::result::Result<Vec<_>, _>>()?;
    let reference_count = usize::try_from(integer_field_value(
        value,
        "bundle payload index",
        "reference_count",
    )?)
    .map_err(|_| "bundle payload index reference_count cannot be negative".to_owned())?;
    let payload_count = usize::try_from(integer_field_value(
        value,
        "bundle payload index",
        "payload_count",
    )?)
    .map_err(|_| "bundle payload index payload_count cannot be negative".to_owned())?;
    if payload_count != payloads.len() {
        return Err("bundle payload index payload_count does not match payload array".to_owned());
    }
    let reference_array_count = array_field_ref(value, "bundle payload index", "references")?.len();
    if reference_count != reference_array_count {
        return Err(
            "bundle payload index reference_count does not match reference array".to_owned(),
        );
    }
    Ok(BundlePayloadIndexSummary {
        manifest_digest: parse_digest_field(manifest, "bundle payload index manifest", "digest")?,
        manifest_size_bytes: integer_field_value(
            manifest,
            "bundle payload index manifest",
            "size_bytes",
        )?,
        workspace_id: parse_workspace_id_field(
            target,
            "bundle payload index target",
            "workspace_id",
        )?,
        commit_id: parse_commit_id_field(target, "bundle payload index target", "commit_id")?,
        state_digest: parse_digest_field(target, "bundle payload index target", "state_digest")?,
        payloads,
        reference_count,
    })
}

fn parse_bundle_payload_file_ref(
    value: &CanonicalValue,
) -> std::result::Result<BundlePayloadFileRef, String> {
    let relative_path = string_field_value(value, "bundle payload file", "path")?.to_owned();
    validate_payload_relative_path(&relative_path).map_err(|error| error.to_string())?;
    let size_bytes = integer_field_value(value, "bundle payload file", "size_bytes")?;
    if size_bytes < 0 {
        return Err(format!(
            "bundle payload file {relative_path} size_bytes cannot be negative"
        ));
    }
    Ok(BundlePayloadFileRef {
        relative_path,
        content_digest: parse_digest_field(value, "bundle payload file", "content_digest")?,
        size_bytes,
        media_type: string_field_value(value, "bundle payload file", "media_type")?.to_owned(),
    })
}

impl BundlePayloadLookup {
    fn from_parts(payload_index: &CanonicalValue, payloads: &[BundlePayloadInput]) -> Result<Self> {
        let mut bytes_by_path = BTreeMap::new();
        for payload in payloads {
            if bytes_by_path
                .insert(payload.relative_path.clone(), payload.bytes.clone())
                .is_some()
            {
                return Err(WorkVcsError::QueryInvalid(format!(
                    "bundle payload path {} appears more than once",
                    payload.relative_path
                )));
            }
        }

        let mut by_role_owner = BTreeMap::new();
        for reference in array_field_ref(payload_index, "bundle payload index", "references")
            .map_err(WorkVcsError::QueryInvalid)?
        {
            let role = string_field_value(reference, "bundle payload reference", "role")
                .map_err(WorkVcsError::QueryInvalid)?
                .to_owned();
            validate_portable_text("bundle payload reference role", &role)
                .map_err(WorkVcsError::QueryInvalid)?;
            let relative_path = string_field_value(reference, "bundle payload reference", "path")
                .map_err(WorkVcsError::QueryInvalid)?
                .to_owned();
            let content_digest =
                parse_digest_field(reference, "bundle payload reference", "content_digest")
                    .map_err(WorkVcsError::QueryInvalid)?;
            let size_bytes =
                integer_field_value(reference, "bundle payload reference", "size_bytes")
                    .map_err(WorkVcsError::QueryInvalid)?;
            let owner = object_field_ref(reference, "bundle payload reference", "owner")
                .map_err(WorkVcsError::QueryInvalid)?;
            let owner_bytes = canonical_bytes(owner)?;
            let entry = BundlePayloadLookupEntry {
                relative_path,
                content_digest,
                size_bytes,
            };
            if by_role_owner
                .insert((role.clone(), owner_bytes), entry)
                .is_some()
            {
                return Err(WorkVcsError::QueryInvalid(format!(
                    "bundle payload reference role {role} owner appears more than once"
                )));
            }
        }
        Ok(Self {
            by_role_owner,
            bytes_by_path,
        })
    }

    fn required_json(
        &self,
        role: &str,
        owner: CanonicalValue,
        expected_digest: Option<Digest>,
        expected_size_bytes: Option<i64>,
    ) -> Result<String> {
        let owner_bytes = canonical_bytes(&owner)?;
        let entry = self
            .by_role_owner
            .get(&(role.to_owned(), owner_bytes))
            .ok_or_else(|| {
                WorkVcsError::QueryInvalid(format!("bundle payload reference {role} is missing"))
            })?;
        if let Some(expected_digest) = expected_digest
            && entry.content_digest != expected_digest
        {
            return Err(WorkVcsError::QueryInvalid(format!(
                "bundle payload reference {role} digest does not match manifest"
            )));
        }
        if let Some(expected_size_bytes) = expected_size_bytes
            && entry.size_bytes != expected_size_bytes
        {
            return Err(WorkVcsError::QueryInvalid(format!(
                "bundle payload reference {role} size does not match manifest"
            )));
        }
        let bytes = self
            .bytes_by_path
            .get(&entry.relative_path)
            .ok_or_else(|| {
                WorkVcsError::QueryInvalid(format!(
                    "bundle payload {} is missing",
                    entry.relative_path
                ))
            })?;
        if content_object_digest(bytes) != entry.content_digest {
            return Err(WorkVcsError::QueryInvalid(format!(
                "bundle payload {} digest does not match reference",
                entry.relative_path
            )));
        }
        if usize_to_i64("bundle payload size", bytes.len())? != entry.size_bytes {
            return Err(WorkVcsError::QueryInvalid(format!(
                "bundle payload {} size does not match reference",
                entry.relative_path
            )));
        }
        String::from_utf8(bytes.clone()).map_err(|error| {
            WorkVcsError::CanonicalEncodingInvalid(format!(
                "bundle payload {} was not UTF-8: {error}",
                entry.relative_path
            ))
        })
    }
}

fn parse_bundle_same_store_apply_document(
    value: &CanonicalValue,
) -> Result<BundleSameStoreApplyDocument> {
    let summary = parse_bundle_manifest_summary(value).map_err(WorkVcsError::QueryInvalid)?;
    if !summary.same_store_apply_supported {
        return Err(WorkVcsError::QueryInvalid(
            "bundle manifest is outside same-Store task-only apply scope".to_owned(),
        ));
    }
    let commits = array_field_ref(value, "bundle manifest", "commit_closure")
        .map_err(WorkVcsError::QueryInvalid)?
        .iter()
        .map(parse_bundle_commit_ref)
        .collect::<Result<Vec<_>>>()?;
    let entity_versions = array_field_ref(value, "bundle manifest", "entity_versions")
        .map_err(WorkVcsError::QueryInvalid)?
        .iter()
        .map(parse_bundle_entity_version_ref)
        .collect::<Result<Vec<_>>>()?;
    let acceptance_criterion_identities =
        optional_array_field_ref(value, "bundle manifest", "acceptance_criterion_identities")
            .map_err(WorkVcsError::QueryInvalid)?
            .iter()
            .map(parse_bundle_acceptance_criterion_identity_ref)
            .collect::<Result<Vec<_>>>()?;
    let verification_requirement_identities = optional_array_field_ref(
        value,
        "bundle manifest",
        "verification_requirement_identities",
    )
    .map_err(WorkVcsError::QueryInvalid)?
    .iter()
    .map(parse_bundle_verification_requirement_identity_ref)
    .collect::<Result<Vec<_>>>()?;
    let relation_versions = array_field_ref(value, "bundle manifest", "relation_versions")
        .map_err(WorkVcsError::QueryInvalid)?
        .iter()
        .map(parse_bundle_relation_version_ref)
        .collect::<Result<Vec<_>>>()?;
    let entity_membership_changes =
        array_field_ref(value, "bundle manifest", "entity_membership_changes")
            .map_err(WorkVcsError::QueryInvalid)?
            .iter()
            .map(parse_bundle_entity_membership_change_ref)
            .collect::<Result<Vec<_>>>()?;
    let relation_membership_changes =
        array_field_ref(value, "bundle manifest", "relation_membership_changes")
            .map_err(WorkVcsError::QueryInvalid)?
            .iter()
            .map(parse_bundle_relation_membership_change_ref)
            .collect::<Result<Vec<_>>>()?;

    let document = BundleSameStoreApplyDocument {
        source_store_id: summary.source_store_id,
        workspace_id: summary.workspace_id,
        commit_id: summary.commit_id,
        state_digest: summary.state_digest,
        commits,
        exported_branch_heads: summary.exported_branch_heads,
        entity_versions,
        acceptance_criterion_identities,
        verification_requirement_identities,
        relation_versions,
        entity_membership_changes,
        relation_membership_changes,
    };
    validate_same_store_apply_identity_coverage(&document)?;
    validate_same_store_apply_relation_coverage(&document)?;
    Ok(document)
}

fn parse_bundle_commit_ref(value: &CanonicalValue) -> Result<BundleCommitRef> {
    let commit_kind = string_field_value(value, "bundle manifest commit", "commit_kind")
        .map_err(WorkVcsError::QueryInvalid)?
        .to_owned();
    validate_portable_text("bundle manifest commit_kind", &commit_kind)
        .map_err(WorkVcsError::QueryInvalid)?;
    let operation_type = string_field_value(value, "bundle manifest commit", "operation_type")
        .map_err(WorkVcsError::QueryInvalid)?
        .to_owned();
    validate_portable_text("bundle manifest operation_type", &operation_type)
        .map_err(WorkVcsError::QueryInvalid)?;
    let operation_schema_version =
        integer_field_value(value, "bundle manifest commit", "operation_schema_version")
            .map_err(WorkVcsError::QueryInvalid)?;
    validate_positive_i64(
        "bundle manifest commit operation_schema_version",
        operation_schema_version,
    )?;
    let committed_at_us = integer_field_value(value, "bundle manifest commit", "committed_at_us")
        .map_err(WorkVcsError::QueryInvalid)?;
    validate_nonnegative_i64("bundle manifest commit committed_at_us", committed_at_us)?;
    let changeset_created_at_us =
        integer_field_value(value, "bundle manifest commit", "changeset_created_at_us")
            .map_err(WorkVcsError::QueryInvalid)?;
    validate_nonnegative_i64(
        "bundle manifest commit changeset_created_at_us",
        changeset_created_at_us,
    )?;
    let change_operation_count =
        integer_field_value(value, "bundle manifest commit", "change_operation_count")
            .map_err(WorkVcsError::QueryInvalid)?;
    validate_nonnegative_i64(
        "bundle manifest commit change_operation_count",
        change_operation_count,
    )?;
    let parents = array_field_ref(value, "bundle manifest commit", "parents")
        .map_err(WorkVcsError::QueryInvalid)?
        .iter()
        .map(parse_bundle_commit_parent_ref)
        .collect::<Result<Vec<_>>>()?;

    Ok(BundleCommitRef {
        workspace_id: parse_workspace_id_field(value, "bundle manifest commit", "workspace_id")
            .map_err(WorkVcsError::QueryInvalid)?,
        commit_id: parse_commit_id_field(value, "bundle manifest commit", "commit_id")
            .map_err(WorkVcsError::QueryInvalid)?,
        changeset_id: parse_changeset_id_field(value, "bundle manifest commit", "changeset_id")
            .map_err(WorkVcsError::QueryInvalid)?,
        commit_kind,
        state_digest: parse_digest_field(value, "bundle manifest commit", "state_digest")
            .map_err(WorkVcsError::QueryInvalid)?,
        committed_at_us,
        operation_type,
        operation_schema_version,
        changeset_created_at_us,
        operation_payload_digest: parse_digest_field(
            value,
            "bundle manifest commit",
            "operation_payload_digest",
        )
        .map_err(WorkVcsError::QueryInvalid)?,
        rationale_digest: parse_digest_field(value, "bundle manifest commit", "rationale_digest")
            .map_err(WorkVcsError::QueryInvalid)?,
        change_operation_count,
        parents,
    })
}

fn parse_bundle_commit_parent_ref(value: &CanonicalValue) -> Result<BundleCommitParentRef> {
    let parent_ordinal =
        integer_field_value(value, "bundle manifest commit parent", "parent_ordinal")
            .map_err(WorkVcsError::QueryInvalid)?;
    validate_nonnegative_i64("bundle manifest commit parent_ordinal", parent_ordinal)?;
    let parent_role = string_field_value(value, "bundle manifest commit parent", "parent_role")
        .map_err(WorkVcsError::QueryInvalid)?
        .to_owned();
    validate_portable_text("bundle manifest commit parent_role", &parent_role)
        .map_err(WorkVcsError::QueryInvalid)?;
    Ok(BundleCommitParentRef {
        parent_ordinal,
        parent_role,
        parent_commit_id: parse_commit_id_field(
            value,
            "bundle manifest commit parent",
            "parent_commit_id",
        )
        .map_err(WorkVcsError::QueryInvalid)?,
    })
}

fn parse_bundle_entity_version_ref(value: &CanonicalValue) -> Result<BundleEntityVersionRef> {
    let entity_kind = string_field_value(value, "bundle manifest entity version", "entity_kind")
        .map_err(WorkVcsError::QueryInvalid)?
        .to_owned();
    validate_portable_text("bundle manifest entity_kind", &entity_kind)
        .map_err(WorkVcsError::QueryInvalid)?;
    let state_schema_version = integer_field_value(
        value,
        "bundle manifest entity version",
        "state_schema_version",
    )
    .map_err(WorkVcsError::QueryInvalid)?;
    validate_positive_i64(
        "bundle manifest entity version state_schema_version",
        state_schema_version,
    )?;
    let state_json_size_bytes = integer_field_value(
        value,
        "bundle manifest entity version",
        "state_json_size_bytes",
    )
    .map_err(WorkVcsError::QueryInvalid)?;
    validate_nonnegative_i64(
        "bundle manifest entity version state_json_size_bytes",
        state_json_size_bytes,
    )?;
    Ok(BundleEntityVersionRef {
        entity_id: parse_entity_id_field(value, "bundle manifest entity version", "entity_id")
            .map_err(WorkVcsError::QueryInvalid)?,
        entity_version_id: parse_entity_version_id_field(
            value,
            "bundle manifest entity version",
            "entity_version_id",
        )
        .map_err(WorkVcsError::QueryInvalid)?,
        entity_kind,
        state_schema_version,
        state_digest: parse_digest_field(value, "bundle manifest entity version", "state_digest")
            .map_err(WorkVcsError::QueryInvalid)?,
        state_json_digest: parse_digest_field(
            value,
            "bundle manifest entity version",
            "state_json_digest",
        )
        .map_err(WorkVcsError::QueryInvalid)?,
        state_json_size_bytes,
    })
}

fn parse_bundle_acceptance_criterion_identity_ref(
    value: &CanonicalValue,
) -> Result<BundleAcceptanceCriterionIdentityRef> {
    let local_key = string_field_value(
        value,
        "bundle manifest acceptance criterion identity",
        "local_key",
    )
    .map_err(WorkVcsError::QueryInvalid)?
    .to_owned();
    validate_portable_text("bundle manifest acceptance criterion local_key", &local_key)
        .map_err(WorkVcsError::QueryInvalid)?;
    Ok(BundleAcceptanceCriterionIdentityRef {
        entity_id: parse_entity_id_field(
            value,
            "bundle manifest acceptance criterion identity",
            "entity_id",
        )
        .map_err(WorkVcsError::QueryInvalid)?,
        owner_entity_id: parse_entity_id_field(
            value,
            "bundle manifest acceptance criterion identity",
            "owner_entity_id",
        )
        .map_err(WorkVcsError::QueryInvalid)?,
        local_key,
    })
}

fn parse_bundle_verification_requirement_identity_ref(
    value: &CanonicalValue,
) -> Result<BundleVerificationRequirementIdentityRef> {
    let local_key = string_field_value(
        value,
        "bundle manifest verification requirement identity",
        "local_key",
    )
    .map_err(WorkVcsError::QueryInvalid)?
    .to_owned();
    validate_portable_text(
        "bundle manifest verification requirement local_key",
        &local_key,
    )
    .map_err(WorkVcsError::QueryInvalid)?;
    Ok(BundleVerificationRequirementIdentityRef {
        entity_id: parse_entity_id_field(
            value,
            "bundle manifest verification requirement identity",
            "entity_id",
        )
        .map_err(WorkVcsError::QueryInvalid)?,
        owner_entity_id: parse_entity_id_field(
            value,
            "bundle manifest verification requirement identity",
            "owner_entity_id",
        )
        .map_err(WorkVcsError::QueryInvalid)?,
        local_key,
    })
}

fn validate_same_store_apply_identity_coverage(
    document: &BundleSameStoreApplyDocument,
) -> Result<()> {
    let mut entity_kinds_by_id = BTreeMap::new();
    let mut expected_acceptance_criterion_ids = BTreeSet::new();
    let mut expected_verification_requirement_ids = BTreeSet::new();
    for entity_version in &document.entity_versions {
        if let Some(existing_kind) =
            entity_kinds_by_id.insert(entity_version.entity_id, entity_version.entity_kind.clone())
            && existing_kind != entity_version.entity_kind
        {
            return Err(WorkVcsError::QueryInvalid(format!(
                "bundle manifest entity {} has inconsistent entity kinds",
                entity_version.entity_id
            )));
        }
        match entity_version.entity_kind.as_str() {
            TASK_ENTITY_KIND => {}
            ACCEPTANCE_CRITERION_ENTITY_KIND => {
                expected_acceptance_criterion_ids.insert(entity_version.entity_id);
            }
            VERIFICATION_REQUIREMENT_ENTITY_KIND => {
                expected_verification_requirement_ids.insert(entity_version.entity_id);
            }
            _ => {
                return Err(WorkVcsError::QueryInvalid(format!(
                    "bundle entity {} kind {} is outside same-Store typed-entity apply scope",
                    entity_version.entity_id, entity_version.entity_kind
                )));
            }
        }
    }
    let acceptance_criterion_ids =
        unique_acceptance_criterion_identity_ids(&document.acceptance_criterion_identities)?;
    let verification_requirement_ids = unique_verification_requirement_identity_ids(
        &document.verification_requirement_identities,
    )?;
    if acceptance_criterion_ids != expected_acceptance_criterion_ids {
        return Err(WorkVcsError::QueryInvalid(
            "bundle acceptance criterion identities do not cover acceptance criterion entity versions"
                .to_owned(),
        ));
    }
    if verification_requirement_ids != expected_verification_requirement_ids {
        return Err(WorkVcsError::QueryInvalid(
            "bundle verification requirement identities do not cover verification requirement entity versions"
                .to_owned(),
        ));
    }
    Ok(())
}

fn unique_acceptance_criterion_identity_ids(
    identities: &[BundleAcceptanceCriterionIdentityRef],
) -> Result<BTreeSet<EntityId>> {
    let mut ids = BTreeSet::new();
    let mut by_owner_local_key = BTreeMap::new();
    for identity in identities {
        if !ids.insert(identity.entity_id) {
            return Err(WorkVcsError::QueryInvalid(format!(
                "bundle acceptance criterion identity {} appears more than once",
                identity.entity_id
            )));
        }
        if let Some(existing_entity_id) = by_owner_local_key.insert(
            (identity.owner_entity_id, identity.local_key.clone()),
            identity.entity_id,
        ) && existing_entity_id != identity.entity_id
        {
            return Err(WorkVcsError::QueryInvalid(format!(
                "bundle acceptance criterion identity owner {} local key {:?} maps to multiple entities",
                identity.owner_entity_id, identity.local_key
            )));
        }
    }
    Ok(ids)
}

fn unique_verification_requirement_identity_ids(
    identities: &[BundleVerificationRequirementIdentityRef],
) -> Result<BTreeSet<EntityId>> {
    let mut ids = BTreeSet::new();
    let mut by_owner_local_key = BTreeMap::new();
    for identity in identities {
        if !ids.insert(identity.entity_id) {
            return Err(WorkVcsError::QueryInvalid(format!(
                "bundle verification requirement identity {} appears more than once",
                identity.entity_id
            )));
        }
        if let Some(existing_entity_id) = by_owner_local_key.insert(
            (identity.owner_entity_id, identity.local_key.clone()),
            identity.entity_id,
        ) && existing_entity_id != identity.entity_id
        {
            return Err(WorkVcsError::QueryInvalid(format!(
                "bundle verification requirement identity owner {} local key {:?} maps to multiple entities",
                identity.owner_entity_id, identity.local_key
            )));
        }
    }
    Ok(ids)
}

fn validate_same_store_apply_relation_coverage(
    document: &BundleSameStoreApplyDocument,
) -> Result<()> {
    let mut relation_versions = BTreeSet::new();
    let mut relation_shapes = BTreeMap::new();
    for relation_version in &document.relation_versions {
        if !relation_versions.insert((
            relation_version.relation_id,
            relation_version.relation_version_id,
        )) {
            return Err(WorkVcsError::QueryInvalid(format!(
                "bundle relation version {} for relation {} appears more than once",
                relation_version.relation_version_id, relation_version.relation_id
            )));
        }
        let shape = (
            relation_version.relation_type.clone(),
            relation_version.source_object_id.clone(),
            relation_version.target_object_id.clone(),
            relation_version.relation_discriminator.clone(),
        );
        if let Some(existing_shape) =
            relation_shapes.insert(relation_version.relation_id, shape.clone())
            && existing_shape != shape
        {
            return Err(WorkVcsError::QueryInvalid(format!(
                "bundle relation {} has inconsistent relation identity fields",
                relation_version.relation_id
            )));
        }
    }
    for change in &document.relation_membership_changes {
        if let Some(relation_version_id) = change.before_relation_version_id
            && !relation_versions.contains(&(change.relation_id, relation_version_id))
        {
            return Err(WorkVcsError::QueryInvalid(format!(
                "bundle relation membership change {} before version {} is missing from relation_versions",
                change.operation_id, relation_version_id
            )));
        }
        if let Some(relation_version_id) = change.after_relation_version_id
            && !relation_versions.contains(&(change.relation_id, relation_version_id))
        {
            return Err(WorkVcsError::QueryInvalid(format!(
                "bundle relation membership change {} after version {} is missing from relation_versions",
                change.operation_id, relation_version_id
            )));
        }
    }
    Ok(())
}

fn parse_bundle_relation_version_ref(value: &CanonicalValue) -> Result<BundleRelationVersionRef> {
    let relation_type =
        string_field_value(value, "bundle manifest relation version", "relation_type")
            .map_err(WorkVcsError::QueryInvalid)?
            .to_owned();
    validate_portable_text("bundle manifest relation_type", &relation_type)
        .map_err(WorkVcsError::QueryInvalid)?;
    let source_object_id = string_field_value(
        value,
        "bundle manifest relation version",
        "source_object_id",
    )
    .map_err(WorkVcsError::QueryInvalid)?
    .to_owned();
    parse_object_id_bytes(
        "bundle manifest relation version source_object_id",
        &source_object_id,
    )?;
    let target_object_id = string_field_value(
        value,
        "bundle manifest relation version",
        "target_object_id",
    )
    .map_err(WorkVcsError::QueryInvalid)?
    .to_owned();
    parse_object_id_bytes(
        "bundle manifest relation version target_object_id",
        &target_object_id,
    )?;
    let relation_discriminator = string_field_value(
        value,
        "bundle manifest relation version",
        "relation_discriminator",
    )
    .map_err(WorkVcsError::QueryInvalid)?
    .to_owned();
    validate_stored_text_allow_empty(
        "bundle manifest relation_discriminator",
        &relation_discriminator,
    )?;
    let state_schema_version = integer_field_value(
        value,
        "bundle manifest relation version",
        "state_schema_version",
    )
    .map_err(WorkVcsError::QueryInvalid)?;
    validate_positive_i64(
        "bundle manifest relation version state_schema_version",
        state_schema_version,
    )?;
    let metadata_json_size_bytes = integer_field_value(
        value,
        "bundle manifest relation version",
        "metadata_json_size_bytes",
    )
    .map_err(WorkVcsError::QueryInvalid)?;
    validate_nonnegative_i64(
        "bundle manifest relation version metadata_json_size_bytes",
        metadata_json_size_bytes,
    )?;
    Ok(BundleRelationVersionRef {
        relation_id: parse_relation_id_field(
            value,
            "bundle manifest relation version",
            "relation_id",
        )
        .map_err(WorkVcsError::QueryInvalid)?,
        relation_version_id: parse_relation_version_id_field(
            value,
            "bundle manifest relation version",
            "relation_version_id",
        )
        .map_err(WorkVcsError::QueryInvalid)?,
        relation_type,
        source_object_id,
        target_object_id,
        relation_discriminator,
        state_schema_version,
        state_digest: parse_digest_field(value, "bundle manifest relation version", "state_digest")
            .map_err(WorkVcsError::QueryInvalid)?,
        metadata_json_digest: parse_digest_field(
            value,
            "bundle manifest relation version",
            "metadata_json_digest",
        )
        .map_err(WorkVcsError::QueryInvalid)?,
        metadata_json_size_bytes,
    })
}

fn parse_bundle_relation_membership_change_ref(
    value: &CanonicalValue,
) -> Result<BundleRelationMembershipChangeRef> {
    let ordinal = integer_field_value(
        value,
        "bundle manifest relation membership change",
        "ordinal",
    )
    .map_err(WorkVcsError::QueryInvalid)?;
    validate_nonnegative_i64(
        "bundle manifest relation membership change ordinal",
        ordinal,
    )?;
    let field_delta_size_bytes = integer_field_value(
        value,
        "bundle manifest relation membership change",
        "field_delta_size_bytes",
    )
    .map_err(WorkVcsError::QueryInvalid)?;
    validate_nonnegative_i64(
        "bundle manifest relation membership change field_delta_size_bytes",
        field_delta_size_bytes,
    )?;
    let before_relation_version_id = parse_optional_relation_version_field(
        value,
        "bundle manifest relation membership change",
        "before_relation_version_id",
    )?;
    let after_relation_version_id = parse_optional_relation_version_field(
        value,
        "bundle manifest relation membership change",
        "after_relation_version_id",
    )?;
    validate_membership_transition(
        "bundle manifest relation membership change",
        before_relation_version_id,
        after_relation_version_id,
    )?;
    Ok(BundleRelationMembershipChangeRef {
        changeset_id: parse_changeset_id_field(
            value,
            "bundle manifest relation membership change",
            "changeset_id",
        )
        .map_err(WorkVcsError::QueryInvalid)?,
        operation_id: parse_operation_id_field(
            value,
            "bundle manifest relation membership change",
            "operation_id",
        )
        .map_err(WorkVcsError::QueryInvalid)?,
        ordinal,
        relation_id: parse_relation_id_field(
            value,
            "bundle manifest relation membership change",
            "relation_id",
        )
        .map_err(WorkVcsError::QueryInvalid)?,
        before_relation_version_id,
        after_relation_version_id,
        field_delta_digest: parse_digest_field(
            value,
            "bundle manifest relation membership change",
            "field_delta_digest",
        )
        .map_err(WorkVcsError::QueryInvalid)?,
        field_delta_size_bytes,
    })
}

fn parse_bundle_entity_membership_change_ref(
    value: &CanonicalValue,
) -> Result<BundleEntityMembershipChangeRef> {
    let ordinal = integer_field_value(value, "bundle manifest entity membership change", "ordinal")
        .map_err(WorkVcsError::QueryInvalid)?;
    validate_nonnegative_i64("bundle manifest entity membership change ordinal", ordinal)?;
    let field_delta_size_bytes = integer_field_value(
        value,
        "bundle manifest entity membership change",
        "field_delta_size_bytes",
    )
    .map_err(WorkVcsError::QueryInvalid)?;
    validate_nonnegative_i64(
        "bundle manifest entity membership change field_delta_size_bytes",
        field_delta_size_bytes,
    )?;
    Ok(BundleEntityMembershipChangeRef {
        changeset_id: parse_changeset_id_field(
            value,
            "bundle manifest entity membership change",
            "changeset_id",
        )
        .map_err(WorkVcsError::QueryInvalid)?,
        operation_id: parse_operation_id_field(
            value,
            "bundle manifest entity membership change",
            "operation_id",
        )
        .map_err(WorkVcsError::QueryInvalid)?,
        ordinal,
        entity_id: parse_entity_id_field(
            value,
            "bundle manifest entity membership change",
            "entity_id",
        )
        .map_err(WorkVcsError::QueryInvalid)?,
        before_entity_version_id: parse_optional_entity_version_field(
            value,
            "bundle manifest entity membership change",
            "before_entity_version_id",
        )?,
        after_entity_version_id: parse_optional_entity_version_field(
            value,
            "bundle manifest entity membership change",
            "after_entity_version_id",
        )?,
        field_delta_digest: parse_digest_field(
            value,
            "bundle manifest entity membership change",
            "field_delta_digest",
        )
        .map_err(WorkVcsError::QueryInvalid)?,
        field_delta_size_bytes,
    })
}

fn object_field_ref<'a>(
    value: &'a CanonicalValue,
    label: &str,
    field: &str,
) -> std::result::Result<&'a CanonicalValue, String> {
    let fields = match value {
        CanonicalValue::Object(fields) => fields,
        _ => return Err(format!("{label} must be an object")),
    };
    fields
        .iter()
        .find_map(|(candidate, value)| (candidate == field).then_some(value))
        .ok_or_else(|| format!("{label} missing field {field}"))
}

fn array_field_ref<'a>(
    value: &'a CanonicalValue,
    label: &str,
    field: &str,
) -> std::result::Result<&'a [CanonicalValue], String> {
    match object_field_ref(value, label, field)? {
        CanonicalValue::Array(values) => Ok(values),
        _ => Err(format!("{label} field {field} must be an array")),
    }
}

fn optional_array_field_ref<'a>(
    value: &'a CanonicalValue,
    label: &str,
    field: &str,
) -> std::result::Result<&'a [CanonicalValue], String> {
    let fields = match value {
        CanonicalValue::Object(fields) => fields,
        _ => return Err(format!("{label} must be an object")),
    };
    let Some(value) = fields
        .iter()
        .find_map(|(candidate, value)| (candidate == field).then_some(value))
    else {
        return Ok(&[]);
    };
    match value {
        CanonicalValue::Array(values) => Ok(values),
        _ => Err(format!("{label} field {field} must be an array")),
    }
}

fn string_field_value<'a>(
    value: &'a CanonicalValue,
    label: &str,
    field: &str,
) -> std::result::Result<&'a str, String> {
    match object_field_ref(value, label, field)? {
        CanonicalValue::String(value) => Ok(value),
        _ => Err(format!("{label} field {field} must be a string")),
    }
}

fn integer_field_value(
    value: &CanonicalValue,
    label: &str,
    field: &str,
) -> std::result::Result<i64, String> {
    match object_field_ref(value, label, field)? {
        CanonicalValue::Integer(value) => Ok(value.get()),
        _ => Err(format!("{label} field {field} must be an integer")),
    }
}

fn expect_string_field(
    value: &CanonicalValue,
    label: &str,
    field: &str,
    expected: &str,
) -> std::result::Result<(), String> {
    let actual = string_field_value(value, label, field)?;
    if actual == expected {
        Ok(())
    } else {
        Err(format!(
            "{label} field {field} expected {expected}, found {actual}"
        ))
    }
}

fn expect_integer_field(
    value: &CanonicalValue,
    label: &str,
    field: &str,
    expected: i64,
) -> std::result::Result<(), String> {
    let actual = integer_field_value(value, label, field)?;
    if actual == expected {
        Ok(())
    } else {
        Err(format!(
            "{label} field {field} expected {expected}, found {actual}"
        ))
    }
}

fn parse_store_id_field(
    value: &CanonicalValue,
    label: &str,
    field: &str,
) -> std::result::Result<StoreId, String> {
    StoreId::parse_canonical(string_field_value(value, label, field)?)
        .map_err(|error| error.to_string())
}

fn parse_branch_id_field(
    value: &CanonicalValue,
    label: &str,
    field: &str,
) -> std::result::Result<BranchId, String> {
    BranchId::parse_canonical(string_field_value(value, label, field)?)
        .map_err(|error| error.to_string())
}

fn parse_workspace_id_field(
    value: &CanonicalValue,
    label: &str,
    field: &str,
) -> std::result::Result<WorkspaceId, String> {
    WorkspaceId::parse_canonical(string_field_value(value, label, field)?)
        .map_err(|error| error.to_string())
}

fn parse_commit_id_field(
    value: &CanonicalValue,
    label: &str,
    field: &str,
) -> std::result::Result<CommitId, String> {
    CommitId::parse_canonical(string_field_value(value, label, field)?)
        .map_err(|error| error.to_string())
}

fn parse_changeset_id_field(
    value: &CanonicalValue,
    label: &str,
    field: &str,
) -> std::result::Result<ChangeSetId, String> {
    ChangeSetId::parse_canonical(string_field_value(value, label, field)?)
        .map_err(|error| error.to_string())
}

fn parse_operation_id_field(
    value: &CanonicalValue,
    label: &str,
    field: &str,
) -> std::result::Result<OperationId, String> {
    OperationId::parse_canonical(string_field_value(value, label, field)?)
        .map_err(|error| error.to_string())
}

fn parse_entity_id_field(
    value: &CanonicalValue,
    label: &str,
    field: &str,
) -> std::result::Result<EntityId, String> {
    EntityId::parse_canonical(string_field_value(value, label, field)?)
        .map_err(|error| error.to_string())
}

fn parse_entity_version_id_field(
    value: &CanonicalValue,
    label: &str,
    field: &str,
) -> std::result::Result<EntityVersionId, String> {
    EntityVersionId::parse_canonical(string_field_value(value, label, field)?)
        .map_err(|error| error.to_string())
}

fn parse_relation_id_field(
    value: &CanonicalValue,
    label: &str,
    field: &str,
) -> std::result::Result<RelationId, String> {
    RelationId::parse_canonical(string_field_value(value, label, field)?)
        .map_err(|error| error.to_string())
}

fn parse_relation_version_id_field(
    value: &CanonicalValue,
    label: &str,
    field: &str,
) -> std::result::Result<RelationVersionId, String> {
    RelationVersionId::parse_canonical(string_field_value(value, label, field)?)
        .map_err(|error| error.to_string())
}

fn parse_optional_entity_version_field(
    value: &CanonicalValue,
    label: &str,
    field: &str,
) -> Result<Option<EntityVersionId>> {
    match object_field_ref(value, label, field).map_err(WorkVcsError::QueryInvalid)? {
        CanonicalValue::Null => Ok(None),
        CanonicalValue::String(value) => EntityVersionId::parse_canonical(value)
            .map(Some)
            .map_err(|error| WorkVcsError::QueryInvalid(error.to_string())),
        _ => Err(WorkVcsError::QueryInvalid(format!(
            "{label} field {field} must be null or string"
        ))),
    }
}

fn parse_optional_relation_version_field(
    value: &CanonicalValue,
    label: &str,
    field: &str,
) -> Result<Option<RelationVersionId>> {
    match object_field_ref(value, label, field).map_err(WorkVcsError::QueryInvalid)? {
        CanonicalValue::Null => Ok(None),
        CanonicalValue::String(value) => RelationVersionId::parse_canonical(value)
            .map(Some)
            .map_err(|error| WorkVcsError::QueryInvalid(error.to_string())),
        _ => Err(WorkVcsError::QueryInvalid(format!(
            "{label} field {field} must be null or string"
        ))),
    }
}

fn parse_digest_field(
    value: &CanonicalValue,
    label: &str,
    field: &str,
) -> std::result::Result<Digest, String> {
    Digest::from_hex(string_field_value(value, label, field)?).map_err(|error| error.to_string())
}

fn parse_object_id_bytes(label: &str, value: &str) -> Result<[u8; 16]> {
    let uuid = Uuid::parse_str(value)
        .map_err(|error| WorkVcsError::QueryInvalid(format!("{label}: {error}")))?;
    if uuid.get_version_num() != 7 || uuid.hyphenated().to_string() != value {
        return Err(WorkVcsError::QueryInvalid(format!(
            "{label} must be a canonical lowercase hyphenated UUIDv7"
        )));
    }
    Ok(*uuid.as_bytes())
}

fn validate_portable_text(label: &str, value: &str) -> std::result::Result<(), String> {
    if value.is_empty() {
        return Err(format!("{label} cannot be empty"));
    }
    if value.chars().any(char::is_control) {
        return Err(format!("{label} cannot contain control characters"));
    }
    Ok(())
}

fn string_field(name: &str, value: impl Into<String>) -> (String, CanonicalValue) {
    (name.to_owned(), CanonicalValue::String(value.into()))
}

fn integer_field(name: &str, value: i64) -> Result<(String, CanonicalValue)> {
    Ok((name.to_owned(), CanonicalValue::safe_integer(value)?))
}

fn usize_to_i64(label: &str, value: usize) -> Result<i64> {
    i64::try_from(value)
        .map_err(|_| WorkVcsError::QueryInvalid(format!("{label} does not fit i64")))
}

fn validate_canonical_json_text(label: &str, input: &str) -> Result<()> {
    validate_canonical_json_value(label, input).map(|_| ())
}

fn validate_canonical_json_value(label: &str, input: &str) -> Result<CanonicalValue> {
    let value = parse_canonical_json(input.as_bytes())?;
    let bytes = canonical_bytes(&value)?;
    let reencoded = String::from_utf8(bytes).map_err(|error| {
        WorkVcsError::CanonicalEncodingInvalid(format!("canonical JSON was not UTF-8: {error}"))
    })?;
    if reencoded != input {
        return Err(WorkVcsError::QueryInvalid(format!(
            "{label} is not canonical JSON"
        )));
    }
    Ok(value)
}

fn validate_stored_text(label: &str, value: &str) -> Result<()> {
    if value.is_empty() {
        return Err(WorkVcsError::QueryInvalid(format!(
            "{label} cannot be empty"
        )));
    }
    if value.chars().any(char::is_control) {
        return Err(WorkVcsError::QueryInvalid(format!(
            "{label} cannot contain control characters"
        )));
    }
    Ok(())
}

fn validate_stored_text_allow_empty(label: &str, value: &str) -> Result<()> {
    if value.chars().any(char::is_control) {
        return Err(WorkVcsError::QueryInvalid(format!(
            "{label} cannot contain control characters"
        )));
    }
    Ok(())
}

fn validate_object_kind_exact(label: &str, actual: &str, expected: &str) -> Result<()> {
    validate_stored_text(label, actual)?;
    if actual == expected {
        Ok(())
    } else {
        Err(WorkVcsError::QueryInvalid(format!(
            "{label} expected {expected}, found {actual}"
        )))
    }
}

fn validate_knowledge_exposure_lifecycle_status(label: &str, value: &str) -> Result<()> {
    match value {
        "active" | "withdrawn" => Ok(()),
        _ => Err(WorkVcsError::QueryInvalid(format!(
            "{label} must be active or withdrawn"
        ))),
    }
}

fn validate_knowledge_exposure_source_status(label: &str, value: &str) -> Result<()> {
    match value {
        "current" | "stale" | "unknown" | "unresolved" => Ok(()),
        _ => Err(WorkVcsError::QueryInvalid(format!(
            "{label} must be current, stale, unknown, or unresolved"
        ))),
    }
}

fn validate_nonnegative_i64(label: &str, value: i64) -> Result<()> {
    if value < 0 {
        return Err(WorkVcsError::QueryInvalid(format!(
            "{label} must be non-negative"
        )));
    }
    Ok(())
}

fn validate_positive_i64(label: &str, value: i64) -> Result<()> {
    if value <= 0 {
        return Err(WorkVcsError::QueryInvalid(format!(
            "{label} must be positive"
        )));
    }
    Ok(())
}

fn validate_subject_family(label: &str, value: &str) -> Result<()> {
    match value {
        "entity" | "relation" => Ok(()),
        _ => Err(WorkVcsError::QueryInvalid(format!(
            "{label} must be entity or relation"
        ))),
    }
}

fn validate_subject_family_exact(label: &str, value: &str, expected: &str) -> Result<()> {
    validate_subject_family(label, value)?;
    if value == expected {
        Ok(())
    } else {
        Err(WorkVcsError::QueryInvalid(format!(
            "{label} must be {expected}"
        )))
    }
}

fn validate_membership_transition<T: Eq>(
    label: &str,
    before: Option<T>,
    after: Option<T>,
) -> Result<()> {
    if before.is_none() && after.is_none() {
        return Err(WorkVcsError::QueryInvalid(format!(
            "{label} must have a before or after version"
        )));
    }
    if before == after {
        return Err(WorkVcsError::QueryInvalid(format!(
            "{label} before and after versions must differ"
        )));
    }
    Ok(())
}

fn validate_payload_relative_path(relative_path: &str) -> Result<()> {
    payload_digest_from_relative_path(relative_path).map(|_| ())
}

fn payload_digest_from_relative_path(relative_path: &str) -> Result<Digest> {
    let Some(digest_hex) = relative_path
        .strip_prefix("payloads/")
        .and_then(|value| value.strip_suffix(".json"))
    else {
        return Err(WorkVcsError::QueryInvalid(
            "bundle payload path must match payloads/<digest>.json".to_owned(),
        ));
    };
    Digest::from_hex(digest_hex).map_err(|error| {
        WorkVcsError::QueryInvalid(format!("bundle payload path digest is invalid: {error}"))
    })
}

fn bundle_format_compatible(store_info: &StoreInfo, manifest: &BundleManifestSummary) -> bool {
    manifest.store_format_version == store_info.manifest.store_format_version
        && manifest.schema_version == store_info.manifest.schema_version
        && manifest.object_store_format_version == store_info.manifest.object_store_format_version
        && manifest.id_scheme == store_info.manifest.id_scheme
        && manifest.digest_algorithm == store_info.manifest.digest_algorithm
        && manifest.canonical_json_profile == store_info.manifest.canonical_json_profile
}

fn bundle_branch_preflight_summary(
    connection: &StoreConnection,
    manifest: &BundleManifestSummary,
) -> Result<BundleBranchPreflightSummary> {
    let mut summary = BundleBranchPreflightSummary {
        exported_branch_heads: manifest.exported_branch_heads.len(),
        ..BundleBranchPreflightSummary::default()
    };
    for branch in &manifest.exported_branch_heads {
        if branch.workspace_id != manifest.workspace_id {
            summary.diverged += 1;
            continue;
        }
        let Some((local_workspace_id, local_head_commit_id, local_head_state_digest)) =
            load_local_branch_head_for_preflight(connection, branch.branch_id)?
        else {
            summary.missing += 1;
            continue;
        };
        if local_workspace_id != branch.workspace_id {
            summary.diverged += 1;
            continue;
        }
        if local_head_commit_id == branch.head_commit_id {
            if local_head_state_digest == branch.head_state_digest {
                summary.already_present += 1;
            } else {
                summary.diverged += 1;
            }
            continue;
        }
        let local_digest_matches_manifest =
            manifest_commit_digest(manifest, local_head_commit_id) == Some(local_head_state_digest);
        if local_digest_matches_manifest
            && manifest_commit_is_descendant(manifest, branch.head_commit_id, local_head_commit_id)
        {
            summary.fast_forward += 1;
        } else {
            summary.diverged += 1;
        }
    }
    Ok(summary)
}

fn load_local_branch_head_for_preflight(
    connection: &StoreConnection,
    branch_id: BranchId,
) -> Result<Option<(WorkspaceId, CommitId, Digest)>> {
    let row = connection
        .inner()
        .query_row(
            "SELECT branch.workspace_id,
                    branch.name,
                    branch.head_commit_id,
                    branch.lifecycle_state,
                    workstate_commit.state_digest
             FROM branch
             JOIN workstate_commit
               ON workstate_commit.workspace_id = branch.workspace_id
              AND workstate_commit.commit_id = branch.head_commit_id
             WHERE branch.branch_id = ?1",
            params![&branch_id.raw_bytes()[..]],
            |row| {
                Ok((
                    row.get::<_, Vec<u8>>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Vec<u8>>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, Vec<u8>>(4)?,
                ))
            },
        )
        .optional()
        .map_err(storage_error)?;
    let Some((workspace_id, name, head_commit_id, lifecycle_state, state_digest)) = row else {
        return Ok(None);
    };
    validate_stored_text("branch.name", &name)?;
    validate_stored_text("branch.lifecycle_state", &lifecycle_state)?;
    Ok(Some((
        decode_workspace_id("branch.workspace_id", workspace_id)?,
        decode_commit_id("branch.head_commit_id", head_commit_id)?,
        decode_digest("workstate_commit.state_digest", state_digest)?,
    )))
}

fn manifest_commit_digest(manifest: &BundleManifestSummary, commit_id: CommitId) -> Option<Digest> {
    manifest
        .commits
        .iter()
        .find_map(|commit| (commit.commit_id == commit_id).then_some(commit.state_digest))
}

fn manifest_commit_is_descendant(
    manifest: &BundleManifestSummary,
    descendant: CommitId,
    ancestor: CommitId,
) -> bool {
    if descendant == ancestor {
        return true;
    }
    let parents_by_commit = manifest
        .commits
        .iter()
        .map(|commit| (commit.commit_id, commit.parent_commit_ids.as_slice()))
        .collect::<BTreeMap<_, _>>();
    let mut stack = vec![descendant];
    let mut seen = BTreeSet::new();
    while let Some(commit_id) = stack.pop() {
        if !seen.insert(commit_id) {
            continue;
        }
        let Some(parents) = parents_by_commit.get(&commit_id) else {
            continue;
        };
        for parent in *parents {
            if *parent == ancestor {
                return true;
            }
            stack.push(*parent);
        }
    }
    false
}

fn source_store_relation(local_store_id: StoreId, source_store_id: StoreId) -> String {
    if local_store_id == source_store_id {
        "same_store".to_owned()
    } else {
        "external_store".to_owned()
    }
}

fn load_commit_state_digest_optional(
    connection: &StoreConnection,
    commit_id: CommitId,
) -> Result<Option<Digest>> {
    connection
        .inner()
        .query_row(
            "SELECT state_digest
             FROM workstate_commit
             WHERE commit_id = ?1",
            params![&commit_id.raw_bytes()[..]],
            |row| row.get::<_, Vec<u8>>(0),
        )
        .optional()
        .map_err(storage_error)?
        .map(|bytes| decode_digest("workstate_commit.state_digest", bytes))
        .transpose()
}

fn load_bundle_import_attempt_snapshot(
    connection: &StoreConnection,
    import_id: ImportId,
) -> Result<Option<BundleImportAttemptSnapshot>> {
    let row = connection
        .inner()
        .query_row(
            "SELECT source_store_id,
                    bundle_digest,
                    import_profile,
                    origin_session_id,
                    started_at_us
             FROM import_attempt
             WHERE import_id = ?1",
            params![&import_id.raw_bytes()[..]],
            |row| {
                Ok((
                    row.get::<_, Vec<u8>>(0)?,
                    row.get::<_, Vec<u8>>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, Option<Vec<u8>>>(3)?,
                    row.get::<_, i64>(4)?,
                ))
            },
        )
        .optional()
        .map_err(storage_error)?;
    let Some((source_store_id, bundle_digest, import_profile, origin_session_id, started_at_us)) =
        row
    else {
        return Ok(None);
    };
    validate_stored_text("import_attempt.import_profile", &import_profile)?;
    validate_positive_i64("import_attempt.started_at_us", started_at_us)?;
    Ok(Some(BundleImportAttemptSnapshot {
        import_id,
        source_store_id: decode_store_id("import_attempt.source_store_id", source_store_id)?,
        bundle_digest: decode_digest("import_attempt.bundle_digest", bundle_digest)?,
        import_profile,
        origin_session_id: decode_optional_session_id(
            "import_attempt.origin_session_id",
            origin_session_id,
        )?,
        started_at_us,
        outcome: load_bundle_import_attempt_outcome(connection, import_id)?,
    }))
}

fn load_bundle_import_attempt_outcome(
    connection: &StoreConnection,
    import_id: ImportId,
) -> Result<Option<BundleImportAttemptOutcomeSnapshot>> {
    let row = connection
        .inner()
        .query_row(
            "SELECT outcome,
                    completed_at_us,
                    detail_json
             FROM import_attempt_outcome
             WHERE import_id = ?1",
            params![&import_id.raw_bytes()[..]],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, String>(2)?,
                ))
            },
        )
        .optional()
        .map_err(storage_error)?;
    let Some((outcome, completed_at_us, detail_json)) = row else {
        return Ok(None);
    };
    validate_stored_text("import_attempt_outcome.outcome", &outcome)?;
    validate_positive_i64("import_attempt_outcome.completed_at_us", completed_at_us)?;
    let detail = validate_canonical_json_value("import_attempt_outcome.detail_json", &detail_json)?;
    Ok(Some(BundleImportAttemptOutcomeSnapshot {
        outcome,
        completed_at_us,
        detail,
        detail_digest: content_object_digest(detail_json.as_bytes()),
        detail_size_bytes: usize_to_i64(
            "import_attempt_outcome.detail_json size",
            detail_json.len(),
        )?,
    }))
}

fn decode_store_id(column: &str, bytes: Vec<u8>) -> Result<StoreId> {
    let bytes = decode_16(column, bytes)?;
    StoreId::from_bytes(bytes)
        .map_err(|error| WorkVcsError::QueryInvalid(format!("{column}: {error}")))
}

fn decode_branch_id(column: &str, bytes: Vec<u8>) -> Result<BranchId> {
    let bytes = decode_16(column, bytes)?;
    BranchId::from_bytes(bytes)
        .map_err(|error| WorkVcsError::QueryInvalid(format!("{column}: {error}")))
}

fn decode_workspace_id(column: &str, bytes: Vec<u8>) -> Result<WorkspaceId> {
    let bytes = decode_16(column, bytes)?;
    WorkspaceId::from_bytes(bytes)
        .map_err(|error| WorkVcsError::QueryInvalid(format!("{column}: {error}")))
}

fn decode_import_id(column: &str, bytes: Vec<u8>) -> Result<ImportId> {
    let bytes = decode_16(column, bytes)?;
    ImportId::from_bytes(bytes)
        .map_err(|error| WorkVcsError::QueryInvalid(format!("{column}: {error}")))
}

fn decode_commit_id(column: &str, bytes: Vec<u8>) -> Result<CommitId> {
    let bytes = decode_16(column, bytes)?;
    CommitId::from_bytes(bytes)
        .map_err(|error| WorkVcsError::QueryInvalid(format!("{column}: {error}")))
}

fn decode_changeset_id(column: &str, bytes: Vec<u8>) -> Result<ChangeSetId> {
    let bytes = decode_16(column, bytes)?;
    ChangeSetId::from_bytes(bytes)
        .map_err(|error| WorkVcsError::QueryInvalid(format!("{column}: {error}")))
}

fn decode_operation_id(column: &str, bytes: Vec<u8>) -> Result<OperationId> {
    let bytes = decode_16(column, bytes)?;
    OperationId::from_bytes(bytes)
        .map_err(|error| WorkVcsError::QueryInvalid(format!("{column}: {error}")))
}

fn decode_entity_id(column: &str, bytes: Vec<u8>) -> Result<EntityId> {
    let bytes = decode_16(column, bytes)?;
    EntityId::from_bytes(bytes)
        .map_err(|error| WorkVcsError::QueryInvalid(format!("{column}: {error}")))
}

fn decode_relation_id(column: &str, bytes: Vec<u8>) -> Result<RelationId> {
    let bytes = decode_16(column, bytes)?;
    RelationId::from_bytes(bytes)
        .map_err(|error| WorkVcsError::QueryInvalid(format!("{column}: {error}")))
}

fn decode_knowledge_space_id(column: &str, bytes: Vec<u8>) -> Result<KnowledgeSpaceId> {
    let bytes = decode_16(column, bytes)?;
    KnowledgeSpaceId::from_bytes(bytes)
        .map_err(|error| WorkVcsError::QueryInvalid(format!("{column}: {error}")))
}

fn decode_exposure_transition_id(column: &str, bytes: Vec<u8>) -> Result<ExposureTransitionId> {
    let bytes = decode_16(column, bytes)?;
    ExposureTransitionId::from_bytes(bytes)
        .map_err(|error| WorkVcsError::QueryInvalid(format!("{column}: {error}")))
}

fn decode_entity_version_id(column: &str, bytes: Vec<u8>) -> Result<EntityVersionId> {
    let bytes = decode_16(column, bytes)?;
    EntityVersionId::from_bytes(bytes)
        .map_err(|error| WorkVcsError::QueryInvalid(format!("{column}: {error}")))
}

fn decode_optional_entity_version_id(
    column: &str,
    bytes: Option<Vec<u8>>,
) -> Result<Option<EntityVersionId>> {
    bytes
        .map(|bytes| {
            let bytes = decode_16(column, bytes)?;
            EntityVersionId::from_bytes(bytes)
                .map_err(|error| WorkVcsError::QueryInvalid(format!("{column}: {error}")))
        })
        .transpose()
}

fn decode_optional_relation_version_id(
    column: &str,
    bytes: Option<Vec<u8>>,
) -> Result<Option<RelationVersionId>> {
    bytes
        .map(|bytes| {
            let bytes = decode_16(column, bytes)?;
            RelationVersionId::from_bytes(bytes)
                .map_err(|error| WorkVcsError::QueryInvalid(format!("{column}: {error}")))
        })
        .transpose()
}

fn decode_optional_exposure_transition_id(
    column: &str,
    bytes: Option<Vec<u8>>,
) -> Result<Option<ExposureTransitionId>> {
    bytes
        .map(|bytes| {
            let bytes = decode_16(column, bytes)?;
            ExposureTransitionId::from_bytes(bytes)
                .map_err(|error| WorkVcsError::QueryInvalid(format!("{column}: {error}")))
        })
        .transpose()
}

fn decode_optional_event_id(column: &str, bytes: Option<Vec<u8>>) -> Result<Option<EventId>> {
    bytes
        .map(|bytes| {
            let bytes = decode_16(column, bytes)?;
            EventId::from_bytes(bytes)
                .map_err(|error| WorkVcsError::QueryInvalid(format!("{column}: {error}")))
        })
        .transpose()
}

fn decode_optional_session_id(column: &str, bytes: Option<Vec<u8>>) -> Result<Option<SessionId>> {
    bytes
        .map(|bytes| {
            let bytes = decode_16(column, bytes)?;
            SessionId::from_bytes(bytes)
                .map_err(|error| WorkVcsError::QueryInvalid(format!("{column}: {error}")))
        })
        .transpose()
}

fn decode_object_id_text(column: &str, bytes: Vec<u8>) -> Result<String> {
    let bytes = decode_16(column, bytes)?;
    let uuid = Uuid::from_bytes(bytes);
    if uuid.get_version_num() != 7 {
        return Err(WorkVcsError::QueryInvalid(format!(
            "{column} is not a UUIDv7 value"
        )));
    }
    Ok(uuid.hyphenated().to_string())
}

fn decode_digest(column: &str, bytes: Vec<u8>) -> Result<Digest> {
    let bytes = decode_32(column, bytes)?;
    Ok(Digest::from_bytes(bytes))
}

fn decode_16(column: &str, bytes: Vec<u8>) -> Result<[u8; 16]> {
    bytes.try_into().map_err(|bytes: Vec<u8>| {
        WorkVcsError::QueryInvalid(format!("{column} must be 16 bytes, found {}", bytes.len()))
    })
}

fn decode_32(column: &str, bytes: Vec<u8>) -> Result<[u8; 32]> {
    bytes.try_into().map_err(|bytes: Vec<u8>| {
        WorkVcsError::QueryInvalid(format!("{column} must be 32 bytes, found {}", bytes.len()))
    })
}
