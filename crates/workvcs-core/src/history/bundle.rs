use crate::canonical::{
    CanonicalValue, WorkState, canonical_bytes, content_object_digest, entity_version_digest,
    parse_canonical_json, relation_version_digest, work_state_mapping_digest,
};
use crate::error::{Result, WorkVcsError, storage_error};
use crate::identity::{
    ChangeSetId, CheckpointId, CommitId, Digest, EntityId, EntityVersionId, RelationId,
    RelationVersionId, StoreId, WorkspaceId,
};
use crate::store::{StoreConnection, StoreInfo, StoreManifest};
use rusqlite::{OptionalExtension, params};
use std::collections::HashSet;
use uuid::Uuid;

const BUNDLE_EXPORT_MANIFEST_PROFILE: &str = "workvcs-local-export-manifest-v1";
const BUNDLE_EXPORT_MANIFEST_VERSION: i64 = 1;

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

    let entity_versions =
        current_entity_version_refs(connection, replayed.workspace_id, &replayed.state)?;
    let relation_versions =
        current_relation_version_refs(connection, replayed.workspace_id, &replayed.state)?;

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

    let manifest = manifest_value(
        store_info,
        &replayed,
        &commits,
        &entity_versions,
        &relation_versions,
        &checkpoint_candidates,
    )?;
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

fn current_entity_version_refs(
    connection: &StoreConnection,
    workspace_id: WorkspaceId,
    state: &WorkState,
) -> Result<Vec<BundleEntityVersionRef>> {
    sorted_entities(state)
        .into_iter()
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

fn current_relation_version_refs(
    connection: &StoreConnection,
    workspace_id: WorkspaceId,
    state: &WorkState,
) -> Result<Vec<BundleRelationVersionRef>> {
    sorted_relations(state)
        .into_iter()
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

fn manifest_value(
    store_info: &StoreInfo,
    replayed: &super::ReplayedState,
    commits: &[BundleCommitRef],
    entity_versions: &[BundleEntityVersionRef],
    relation_versions: &[BundleRelationVersionRef],
    checkpoint_candidates: &[BundleCheckpointCandidate],
) -> Result<CanonicalValue> {
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
            store_manifest_value(store_info.store_id, &store_info.manifest)?,
        ),
        (
            "target".to_owned(),
            CanonicalValue::object(vec![
                string_field("workspace_id", replayed.workspace_id.to_string()),
                string_field("commit_id", replayed.commit_id.to_string()),
                string_field("state_digest", replayed.state_digest.to_string()),
            ])?,
        ),
        (
            "commit_closure".to_owned(),
            CanonicalValue::Array(
                commits
                    .iter()
                    .map(commit_ref_value)
                    .collect::<Result<Vec<_>>>()?,
            ),
        ),
        (
            "work_state".to_owned(),
            work_state_manifest_value(&replayed.state)?,
        ),
        (
            "entity_versions".to_owned(),
            CanonicalValue::Array(
                entity_versions
                    .iter()
                    .map(entity_version_ref_value)
                    .collect::<Result<Vec<_>>>()?,
            ),
        ),
        (
            "relation_versions".to_owned(),
            CanonicalValue::Array(
                relation_versions
                    .iter()
                    .map(relation_version_ref_value)
                    .collect::<Result<Vec<_>>>()?,
            ),
        ),
        (
            "checkpoint_candidates".to_owned(),
            CanonicalValue::Array(
                checkpoint_candidates
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
