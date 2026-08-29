use crate::canonical::{
    CanonicalValue, WorkState, canonical_bytes, content_object_digest, entity_version_digest,
    parse_canonical_json, relation_version_digest, work_state_mapping_digest,
};
use crate::error::{Result, WorkVcsError, storage_error};
use crate::identity::{
    ChangeSetId, CheckpointId, CommitId, Digest, EntityId, EntityVersionId, ImportId, OperationId,
    RelationId, RelationVersionId, SessionId, StoreId, WorkspaceId,
};
use crate::store::{StoreConnection, StoreInfo, StoreManifest, current_epoch_micros};
use rusqlite::{OptionalExtension, TransactionBehavior, params};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use uuid::Uuid;

const BUNDLE_EXPORT_MANIFEST_PROFILE: &str = "workvcs-local-export-manifest-v1";
const BUNDLE_EXPORT_MANIFEST_VERSION: i64 = 1;
const BUNDLE_PAYLOAD_INDEX_PROFILE: &str = "workvcs-local-payload-index-v1";
const BUNDLE_PAYLOAD_INDEX_VERSION: i64 = 1;
const BUNDLE_PAYLOAD_MEDIA_TYPE: &str = "application/json";
const BUNDLE_IMPORT_PROFILE: &str = "workvcs-local-payload-directory-v1";

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
    pub entity_versions: Vec<BundleEntityVersionRef>,
    pub relation_versions: Vec<BundleRelationVersionRef>,
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
pub struct BundleCheckpointCandidate {
    pub checkpoint_id: CheckpointId,
    pub content_digest: Digest,
    pub content_size_bytes: i64,
    pub checkpoint_format_version: i64,
    pub media_type: Option<String>,
    pub usability_state: String,
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

    let entity_membership_changes = entity_membership_change_refs(connection, &commits)?;
    let relation_membership_changes = relation_membership_change_refs(connection, &commits)?;
    let entity_versions = entity_version_closure_refs(
        connection,
        replayed.workspace_id,
        &replayed.state,
        &entity_membership_changes,
    )?;
    let relation_versions = relation_version_closure_refs(
        connection,
        replayed.workspace_id,
        &replayed.state,
        &relation_membership_changes,
    )?;

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
        entity_versions: &entity_versions,
        relation_versions: &relation_versions,
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
        entity_versions,
        relation_versions,
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
                problem: Some(problem),
            });
        }
    };

    let format_compatible = bundle_format_compatible(store_info, &checked.manifest);
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
            problem: Some(format!(
                "local commit {} exists with a different state digest",
                checked.manifest.commit_id
            )),
        });
    }

    let source_store_relation =
        source_store_relation(store_info.store_id, checked.manifest.source_store_id);
    let incoming_commit_present = existing_commit_digest.is_some();
    let (import_required, can_apply, action) = if !format_compatible {
        (true, false, "incompatible_store_format")
    } else if source_store_relation == "same_store" && incoming_commit_present {
        (false, false, "already_present")
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
    entity_versions: &'a [BundleEntityVersionRef],
    relation_versions: &'a [BundleRelationVersionRef],
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
        ("problem".to_owned(), problem),
    ])?;
    let bytes = canonical_bytes(&value)?;
    String::from_utf8(bytes).map_err(|error| {
        WorkVcsError::CanonicalEncodingInvalid(format!(
            "bundle import attempt detail JSON was not UTF-8: {error}"
        ))
    })
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
    Ok(BundleManifestSummary {
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

fn parse_digest_field(
    value: &CanonicalValue,
    label: &str,
    field: &str,
) -> std::result::Result<Digest, String> {
    Digest::from_hex(string_field_value(value, label, field)?).map_err(|error| error.to_string())
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

fn decode_workspace_id(column: &str, bytes: Vec<u8>) -> Result<WorkspaceId> {
    let bytes = decode_16(column, bytes)?;
    WorkspaceId::from_bytes(bytes)
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
