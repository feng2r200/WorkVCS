use super::model::{
    CanonicalPath, ControlPlaneDigest, LocatorAssurance, LocatorAuthority, LocatorEvidence,
    LocatorRole, LocatorState, ProjectMaturity, ProjectRegistryV2, UtcTimestamp,
    canonicalize_serializable, invalid,
};
use crate::canonical::parse_canonical_json;
use crate::error::{Result, WorkVcsError};
use crate::{
    BranchId, ProjectLocatorId, ProjectRefId, RegistryId, RegistryObservationId, StoreId,
    WorkspaceId,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

const REGISTRY_V1_VERSION: u64 = 1;
const REGISTRY_V2_VERSION: u64 = 2;
const MIGRATION_PREVIEW_VERSION: u64 = 1;
const MIGRATION_REPAIR_PREVIEW_VERSION: u64 = 2;
const MIGRATION_OWNERSHIP_REPAIR_SCHEMA_VERSION: u64 = 1;
const MIGRATION_SOURCE_ADAPTER: &str = "workvcs-registry-v1-migration";
pub const MAX_REGISTRY_V1_BYTES: usize = 1_024 * 1_024;
pub const MAX_MIGRATION_OWNERSHIP_REPAIR_MANIFEST_BYTES: usize = 64 * 1_024;
pub const MAX_MIGRATION_OWNERSHIP_REPAIRS: usize = 128;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectBindingV1 {
    identity_kind: String,
    identity: String,
    root: String,
    store_path: String,
    store_id: String,
    workspace_id: String,
    branch_id: String,
}

impl ProjectBindingV1 {
    pub fn identity_kind(&self) -> &str {
        &self.identity_kind
    }

    pub fn identity(&self) -> &str {
        &self.identity
    }

    pub fn root(&self) -> &str {
        &self.root
    }

    pub fn store_path(&self) -> &str {
        &self.store_path
    }

    pub fn store_id(&self) -> &str {
        &self.store_id
    }

    pub fn workspace_id(&self) -> &str {
        &self.workspace_id
    }

    pub fn branch_id(&self) -> &str {
        &self.branch_id
    }

    pub fn key(&self) -> V1BindingKey {
        V1BindingKey {
            identity_kind: self.identity_kind.clone(),
            identity: self.identity.clone(),
        }
    }

    pub fn target_digest(&self) -> Result<ControlPlaneDigest> {
        self.target().digest()
    }

    fn target(&self) -> MigrationTarget {
        MigrationTarget {
            store_path: self.store_path.clone(),
            store_id: self.store_id.clone(),
            workspace_id: self.workspace_id.clone(),
            branch_id: self.branch_id.clone(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectRegistryV1 {
    version: u64,
    bindings: Vec<ProjectBindingV1>,
}

impl ProjectRegistryV1 {
    pub fn from_json_bytes(input: &[u8]) -> Result<Self> {
        if input.len() > MAX_REGISTRY_V1_BYTES {
            return invalid(format!(
                "registry v1 is {} bytes; maximum is {MAX_REGISTRY_V1_BYTES}",
                input.len()
            ));
        }
        parse_canonical_json(input).map_err(|error| {
            WorkVcsError::ControlPlaneInvalid(format!(
                "registry v1 is not strict canonical-domain JSON: {error}"
            ))
        })?;
        let mut registry: Self = serde_json::from_slice(input).map_err(|error| {
            WorkVcsError::ControlPlaneInvalid(format!("registry v1 has an invalid shape: {error}"))
        })?;
        if registry.version != REGISTRY_V1_VERSION {
            return invalid(format!(
                "registry version {} is not supported for v1 migration preview",
                registry.version
            ));
        }
        registry.bindings.sort_by(|left, right| {
            (&left.identity_kind, &left.identity)
                .cmp(&(&right.identity_kind, &right.identity))
                .then_with(|| left.target().cmp(&right.target()))
                .then_with(|| left.root.cmp(&right.root))
        });
        Ok(registry)
    }

    pub fn version(&self) -> u64 {
        self.version
    }

    pub fn bindings(&self) -> &[ProjectBindingV1] {
        &self.bindings
    }

    pub fn canonical_json_bytes(&self) -> Result<Vec<u8>> {
        canonicalize_serializable(self, "registry v1")
    }

    pub fn source_digest(&self) -> Result<ControlPlaneDigest> {
        Ok(ControlPlaneDigest::raw(&self.canonical_json_bytes()?))
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct V1BindingKey {
    identity_kind: String,
    identity: String,
}

impl V1BindingKey {
    pub fn identity_kind(&self) -> &str {
        &self.identity_kind
    }

    pub fn identity(&self) -> &str {
        &self.identity
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MigrationHistoricalIdentityDisposition {
    Retire,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MigrationOwnershipRepair {
    v1_binding_key: V1BindingKey,
    expected_target_digest: ControlPlaneDigest,
    semantic_locator: LocatorEvidence,
    historical_identity_disposition: MigrationHistoricalIdentityDisposition,
}

impl MigrationOwnershipRepair {
    pub fn v1_binding_key(&self) -> &V1BindingKey {
        &self.v1_binding_key
    }

    pub fn expected_target_digest(&self) -> &ControlPlaneDigest {
        &self.expected_target_digest
    }

    pub fn semantic_locator(&self) -> &LocatorEvidence {
        &self.semantic_locator
    }

    pub fn historical_identity_disposition(&self) -> MigrationHistoricalIdentityDisposition {
        self.historical_identity_disposition
    }

    pub fn digest(&self) -> Result<ControlPlaneDigest> {
        Ok(ControlPlaneDigest::raw(&canonicalize_serializable(
            self,
            "migration ownership repair",
        )?))
    }

    fn validate(&self) -> Result<()> {
        if self.v1_binding_key.identity_kind.trim().is_empty()
            || self.v1_binding_key.identity.trim().is_empty()
        {
            return invalid("migration ownership repair requires a non-empty v1 binding key");
        }
        self.semantic_locator.validate()?;
        if self.semantic_locator.authority() != LocatorAuthority::SemanticProject {
            return invalid(
                "migration ownership repair requires semantic_project locator authority",
            );
        }
        if !matches!(
            self.semantic_locator.assurance(),
            LocatorAssurance::Authoritative | LocatorAssurance::VerifiedDerived
        ) {
            return invalid(
                "migration ownership repair requires authoritative or verified_derived semantic locator assurance",
            );
        }
        validate_repair_locator_scalar("provider", self.semantic_locator.provider(), 128)?;
        validate_repair_locator_scalar("namespace", self.semantic_locator.namespace(), 512)?;
        validate_repair_locator_scalar("kind", self.semantic_locator.kind(), 128)?;
        validate_repair_locator_scalar(
            "normalized_value",
            self.semantic_locator.normalized_value(),
            2 * 1_024,
        )?;
        validate_repair_locator_scalar(
            "source_adapter",
            self.semantic_locator.source_adapter(),
            256,
        )?;
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MigrationOwnershipRepairManifest {
    schema_version: u64,
    expected_source_digest: ControlPlaneDigest,
    repairs: Vec<MigrationOwnershipRepair>,
}

impl MigrationOwnershipRepairManifest {
    pub fn from_json_bytes(input: &[u8]) -> Result<Self> {
        if input.len() > MAX_MIGRATION_OWNERSHIP_REPAIR_MANIFEST_BYTES {
            return invalid(format!(
                "migration ownership repair manifest is {} bytes; maximum is {MAX_MIGRATION_OWNERSHIP_REPAIR_MANIFEST_BYTES}",
                input.len()
            ));
        }
        parse_canonical_json(input).map_err(|error| {
            WorkVcsError::ControlPlaneInvalid(format!(
                "migration ownership repair manifest is not strict canonical-domain JSON: {error}"
            ))
        })?;
        let mut manifest: Self = serde_json::from_slice(input).map_err(|error| {
            WorkVcsError::ControlPlaneInvalid(format!(
                "migration ownership repair manifest has an invalid shape: {error}"
            ))
        })?;
        manifest.repairs.sort_by(|left, right| {
            left.v1_binding_key
                .cmp(&right.v1_binding_key)
                .then_with(|| {
                    repair_semantic_locator_key(left.semantic_locator())
                        .cmp(&repair_semantic_locator_key(right.semantic_locator()))
                })
        });
        manifest.validate()?;
        Ok(manifest)
    }

    pub fn expected_source_digest(&self) -> &ControlPlaneDigest {
        &self.expected_source_digest
    }

    pub fn repairs(&self) -> &[MigrationOwnershipRepair] {
        &self.repairs
    }

    pub fn contains_binding(&self, key: &V1BindingKey) -> bool {
        self.repairs
            .binary_search_by(|repair| repair.v1_binding_key.cmp(key))
            .is_ok()
    }

    pub fn canonical_json_bytes(&self) -> Result<Vec<u8>> {
        canonicalize_serializable(self, "migration ownership repair manifest")
    }

    pub fn digest(&self) -> Result<ControlPlaneDigest> {
        Ok(ControlPlaneDigest::raw(&self.canonical_json_bytes()?))
    }

    pub fn validate_for_registry(&self, registry: &ProjectRegistryV1) -> Result<()> {
        let source_digest = registry.source_digest()?;
        if self.expected_source_digest != source_digest {
            return invalid(format!(
                "migration ownership repair expected source digest {} but registry digest is {}",
                self.expected_source_digest, source_digest
            ));
        }
        for repair in &self.repairs {
            let matching = registry
                .bindings
                .iter()
                .filter(|binding| binding.key() == repair.v1_binding_key)
                .collect::<Vec<_>>();
            if matching.len() != 1 {
                return invalid(format!(
                    "migration ownership repair binding {:?} {:?} matched {} v1 rows; expected exactly one",
                    repair.v1_binding_key.identity_kind,
                    repair.v1_binding_key.identity,
                    matching.len()
                ));
            }
            let target_digest = matching[0].target_digest()?;
            if repair.expected_target_digest != target_digest {
                return invalid(format!(
                    "migration ownership repair binding {:?} {:?} expected target digest {} but registry target digest is {}",
                    repair.v1_binding_key.identity_kind,
                    repair.v1_binding_key.identity,
                    repair.expected_target_digest,
                    target_digest
                ));
            }
        }
        Ok(())
    }

    fn validate(&self) -> Result<()> {
        if self.schema_version != MIGRATION_OWNERSHIP_REPAIR_SCHEMA_VERSION {
            return invalid(format!(
                "migration ownership repair schema version {} is not supported; expected {MIGRATION_OWNERSHIP_REPAIR_SCHEMA_VERSION}",
                self.schema_version
            ));
        }
        if self.repairs.is_empty() || self.repairs.len() > MAX_MIGRATION_OWNERSHIP_REPAIRS {
            return invalid(format!(
                "migration ownership repair manifest requires 1..={MAX_MIGRATION_OWNERSHIP_REPAIRS} repairs"
            ));
        }
        let mut binding_keys = BTreeSet::new();
        let mut semantic_keys = BTreeSet::new();
        for repair in &self.repairs {
            repair.validate()?;
            if !binding_keys.insert(repair.v1_binding_key.clone()) {
                return invalid(format!(
                    "migration ownership repair repeats v1 binding {:?} {:?}",
                    repair.v1_binding_key.identity_kind, repair.v1_binding_key.identity
                ));
            }
            let semantic_key = repair_semantic_locator_key(repair.semantic_locator());
            if !semantic_keys.insert(semantic_key) {
                return invalid(
                    "migration ownership repair assigns one semantic locator to multiple v1 bindings",
                );
            }
        }
        Ok(())
    }
}

fn repair_semantic_locator_key(locator: &LocatorEvidence) -> (String, String, String, String) {
    (
        locator.provider().to_owned(),
        locator.namespace().to_owned(),
        locator.kind().to_owned(),
        locator.normalized_value().to_owned(),
    )
}

fn validate_repair_locator_scalar(label: &str, value: &str, maximum_bytes: usize) -> Result<()> {
    if value.trim() != value
        || value.is_empty()
        || value.len() > maximum_bytes
        || value.chars().any(char::is_control)
    {
        return invalid(format!(
            "migration ownership repair semantic locator {label} must be trimmed, non-empty, control-free, and at most {maximum_bytes} bytes"
        ));
    }
    Ok(())
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct MigrationTarget {
    store_path: String,
    store_id: String,
    workspace_id: String,
    branch_id: String,
}

impl MigrationTarget {
    pub fn store_path(&self) -> &str {
        &self.store_path
    }

    pub fn store_id(&self) -> &str {
        &self.store_id
    }

    pub fn workspace_id(&self) -> &str {
        &self.workspace_id
    }

    pub fn branch_id(&self) -> &str {
        &self.branch_id
    }

    pub fn digest(&self) -> Result<ControlPlaneDigest> {
        Ok(ControlPlaneDigest::raw(&canonicalize_serializable(
            self,
            "migration target",
        )?))
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct MigrationValidationIssue {
    code: String,
    detail: String,
}

impl MigrationValidationIssue {
    pub fn new(code: impl Into<String>, detail: impl Into<String>) -> Result<Self> {
        let code = code.into();
        let detail = detail.into();
        if code.is_empty()
            || code.len() > 128
            || !code.chars().all(|character| {
                character.is_ascii_lowercase() || character.is_ascii_digit() || character == '_'
            })
        {
            return invalid(
                "migration validation issue code must be 1..=128 lowercase ASCII snake-case bytes",
            );
        }
        if detail.trim().is_empty() || detail.len() > 4 * 1_024 {
            return invalid(
                "migration validation issue detail must be non-empty and at most 4096 bytes",
            );
        }
        Ok(Self { code, detail })
    }

    pub fn code(&self) -> &str {
        &self.code
    }

    pub fn detail(&self) -> &str {
        &self.detail
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct MigrationBindingValidation {
    mapping_index: usize,
    local_content_objects_verified: Option<u64>,
    issues: Vec<MigrationValidationIssue>,
}

impl MigrationBindingValidation {
    pub fn passed(mapping_index: usize, local_content_objects_verified: usize) -> Self {
        Self {
            mapping_index,
            local_content_objects_verified: Some(local_content_objects_verified as u64),
            issues: Vec::new(),
        }
    }

    pub fn failed(
        mapping_index: usize,
        code: impl Into<String>,
        detail: impl Into<String>,
    ) -> Result<Self> {
        Ok(Self {
            mapping_index,
            local_content_objects_verified: None,
            issues: vec![MigrationValidationIssue::new(code, detail)?],
        })
    }

    pub fn mapping_index(&self) -> usize {
        self.mapping_index
    }

    pub fn local_content_objects_verified(&self) -> Option<u64> {
        self.local_content_objects_verified
    }

    pub fn issues(&self) -> &[MigrationValidationIssue] {
        &self.issues
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MigrationNamespaceStrategy {
    NewRegistryId,
    Explicit,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct MigrationLocatorPlan {
    role: LocatorRole,
    authority: LocatorAuthority,
    provider: String,
    namespace_strategy: MigrationNamespaceStrategy,
    #[serde(skip_serializing_if = "Option::is_none")]
    namespace: Option<String>,
    kind: String,
    normalized_value: String,
    assurance: LocatorAssurance,
    source_adapter: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    evidence_digest: Option<ControlPlaneDigest>,
    #[serde(skip_serializing_if = "Option::is_none")]
    state: Option<LocatorState>,
}

impl MigrationLocatorPlan {
    pub fn role(&self) -> LocatorRole {
        self.role
    }

    pub fn authority(&self) -> LocatorAuthority {
        self.authority
    }

    pub fn provider(&self) -> &str {
        &self.provider
    }

    pub fn kind(&self) -> &str {
        &self.kind
    }

    pub fn normalized_value(&self) -> &str {
        &self.normalized_value
    }

    pub fn namespace_strategy(&self) -> MigrationNamespaceStrategy {
        self.namespace_strategy
    }

    pub fn namespace(&self) -> Option<&str> {
        self.namespace.as_deref()
    }

    pub fn assurance(&self) -> LocatorAssurance {
        self.assurance
    }

    pub fn source_adapter(&self) -> &str {
        &self.source_adapter
    }

    pub fn evidence_digest(&self) -> Option<&ControlPlaneDigest> {
        self.evidence_digest.as_ref()
    }

    pub fn state(&self) -> Option<LocatorState> {
        self.state
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct MigrationPreviewValidation {
    valid: bool,
    local_content_objects_verified: Option<u64>,
    issues: Vec<MigrationValidationIssue>,
}

impl MigrationPreviewValidation {
    pub fn valid(&self) -> bool {
        self.valid
    }

    pub fn local_content_objects_verified(&self) -> Option<u64> {
        self.local_content_objects_verified
    }

    pub fn issues(&self) -> &[MigrationValidationIssue] {
        &self.issues
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct MigrationOwnershipRepairPreview {
    repair_digest: ControlPlaneDigest,
    expected_target_digest: ControlPlaneDigest,
    historical_identity_disposition: MigrationHistoricalIdentityDisposition,
}

impl MigrationOwnershipRepairPreview {
    pub fn repair_digest(&self) -> &ControlPlaneDigest {
        &self.repair_digest
    }

    pub fn expected_target_digest(&self) -> &ControlPlaneDigest {
        &self.expected_target_digest
    }

    pub fn historical_identity_disposition(&self) -> MigrationHistoricalIdentityDisposition {
        self.historical_identity_disposition
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct MigrationPreviewMapping {
    mapping_index: usize,
    v1_binding_key: V1BindingKey,
    root: String,
    planned_maturity: Option<ProjectMaturity>,
    identity_locator: Option<MigrationLocatorPlan>,
    #[serde(skip_serializing_if = "Option::is_none")]
    historical_identity_locator: Option<MigrationLocatorPlan>,
    root_context_locator: Option<MigrationLocatorPlan>,
    #[serde(skip_serializing_if = "Option::is_none")]
    ownership_repair: Option<MigrationOwnershipRepairPreview>,
    target: MigrationTarget,
    target_digest: ControlPlaneDigest,
    validation: MigrationPreviewValidation,
}

impl MigrationPreviewMapping {
    pub fn mapping_index(&self) -> usize {
        self.mapping_index
    }

    pub fn v1_binding_key(&self) -> &V1BindingKey {
        &self.v1_binding_key
    }

    pub fn root(&self) -> &str {
        &self.root
    }

    pub fn planned_maturity(&self) -> Option<ProjectMaturity> {
        self.planned_maturity
    }

    pub fn identity_locator(&self) -> Option<&MigrationLocatorPlan> {
        self.identity_locator.as_ref()
    }

    pub fn historical_identity_locator(&self) -> Option<&MigrationLocatorPlan> {
        self.historical_identity_locator.as_ref()
    }

    pub fn root_context_locator(&self) -> Option<&MigrationLocatorPlan> {
        self.root_context_locator.as_ref()
    }

    pub fn ownership_repair(&self) -> Option<&MigrationOwnershipRepairPreview> {
        self.ownership_repair.as_ref()
    }

    pub fn target(&self) -> &MigrationTarget {
        &self.target
    }

    pub fn target_digest(&self) -> &ControlPlaneDigest {
        &self.target_digest
    }

    pub fn validation(&self) -> &MigrationPreviewValidation {
        &self.validation
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct MigrationTargetCoincidence {
    target: MigrationTarget,
    target_digest: ControlPlaneDigest,
    mapping_indices: Vec<usize>,
    v1_binding_keys: Vec<V1BindingKey>,
}

impl MigrationTargetCoincidence {
    pub fn target(&self) -> &MigrationTarget {
        &self.target
    }

    pub fn target_digest(&self) -> &ControlPlaneDigest {
        &self.target_digest
    }

    pub fn mapping_indices(&self) -> &[usize] {
        &self.mapping_indices
    }

    pub fn v1_binding_keys(&self) -> &[V1BindingKey] {
        &self.v1_binding_keys
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct RegistryMigrationPreview {
    preview_version: u64,
    source_registry_version: u64,
    target_registry_version: u64,
    migration_required: bool,
    source_digest: ControlPlaneDigest,
    #[serde(skip_serializing_if = "Option::is_none")]
    repair_manifest_digest: Option<ControlPlaneDigest>,
    #[serde(skip_serializing_if = "usize_is_zero")]
    ownership_repairs: usize,
    mappings: Vec<MigrationPreviewMapping>,
    target_coincidences: Vec<MigrationTargetCoincidence>,
    apply_eligible: bool,
    preview_digest: ControlPlaneDigest,
}

impl RegistryMigrationPreview {
    pub fn preview_version(&self) -> u64 {
        self.preview_version
    }

    pub fn source_registry_version(&self) -> u64 {
        self.source_registry_version
    }

    pub fn target_registry_version(&self) -> u64 {
        self.target_registry_version
    }

    pub fn migration_required(&self) -> bool {
        self.migration_required
    }

    pub fn source_digest(&self) -> &ControlPlaneDigest {
        &self.source_digest
    }

    pub fn repair_manifest_digest(&self) -> Option<&ControlPlaneDigest> {
        self.repair_manifest_digest.as_ref()
    }

    pub fn ownership_repairs(&self) -> usize {
        self.ownership_repairs
    }

    pub fn mappings(&self) -> &[MigrationPreviewMapping] {
        &self.mappings
    }

    pub fn target_coincidences(&self) -> &[MigrationTargetCoincidence] {
        &self.target_coincidences
    }

    pub fn apply_eligible(&self) -> bool {
        self.apply_eligible
    }

    pub fn preview_digest(&self) -> &ControlPlaneDigest {
        &self.preview_digest
    }

    pub fn canonical_json_bytes(&self) -> Result<Vec<u8>> {
        let expected = PreviewDigestMaterial {
            preview_version: self.preview_version,
            source_registry_version: self.source_registry_version,
            target_registry_version: self.target_registry_version,
            migration_required: self.migration_required,
            source_digest: &self.source_digest,
            repair_manifest_digest: self.repair_manifest_digest.as_ref(),
            ownership_repairs: self.ownership_repairs,
            mappings: &self.mappings,
            target_coincidences: &self.target_coincidences,
            apply_eligible: self.apply_eligible,
        }
        .digest()?;
        if expected != self.preview_digest {
            return invalid(format!(
                "migration preview digest {} does not match recomputed {}",
                self.preview_digest, expected
            ));
        }
        canonicalize_serializable(self, "registry migration preview")
    }
}

#[derive(Serialize)]
struct PreviewDigestMaterial<'a> {
    preview_version: u64,
    source_registry_version: u64,
    target_registry_version: u64,
    migration_required: bool,
    source_digest: &'a ControlPlaneDigest,
    #[serde(skip_serializing_if = "Option::is_none")]
    repair_manifest_digest: Option<&'a ControlPlaneDigest>,
    #[serde(skip_serializing_if = "usize_is_zero")]
    ownership_repairs: usize,
    mappings: &'a [MigrationPreviewMapping],
    target_coincidences: &'a [MigrationTargetCoincidence],
    apply_eligible: bool,
}

fn usize_is_zero(value: &usize) -> bool {
    *value == 0
}

impl PreviewDigestMaterial<'_> {
    fn digest(&self) -> Result<ControlPlaneDigest> {
        Ok(ControlPlaneDigest::raw(&canonicalize_serializable(
            self,
            "registry migration preview digest material",
        )?))
    }
}

pub fn build_registry_v1_migration_preview(
    registry: &ProjectRegistryV1,
    validations: &[MigrationBindingValidation],
) -> Result<RegistryMigrationPreview> {
    build_registry_v1_migration_preview_with_repairs(registry, validations, None)
}

pub fn build_registry_v1_migration_preview_with_repairs(
    registry: &ProjectRegistryV1,
    validations: &[MigrationBindingValidation],
    repair_manifest: Option<&MigrationOwnershipRepairManifest>,
) -> Result<RegistryMigrationPreview> {
    if validations.len() != registry.bindings.len() {
        return invalid(format!(
            "migration preview requires one validation fact per binding; got {} for {} bindings",
            validations.len(),
            registry.bindings.len()
        ));
    }
    let validation_by_index = validations
        .iter()
        .map(|validation| (validation.mapping_index, validation))
        .collect::<BTreeMap<_, _>>();
    if validation_by_index.len() != validations.len()
        || validation_by_index
            .keys()
            .copied()
            .ne(0..registry.bindings.len())
    {
        return invalid("migration validation facts must use every mapping index exactly once");
    }
    if let Some(manifest) = repair_manifest {
        manifest.validate_for_registry(registry)?;
    }
    let repair_by_key = repair_manifest
        .into_iter()
        .flat_map(MigrationOwnershipRepairManifest::repairs)
        .map(|repair| (repair.v1_binding_key.clone(), repair))
        .collect::<BTreeMap<_, _>>();

    let duplicate_keys = registry
        .bindings
        .iter()
        .map(ProjectBindingV1::key)
        .fold(BTreeMap::<V1BindingKey, usize>::new(), |mut counts, key| {
            *counts.entry(key).or_default() += 1;
            counts
        })
        .into_iter()
        .filter_map(|(key, count)| (count > 1).then_some(key))
        .collect::<BTreeSet<_>>();

    let mut mappings = Vec::with_capacity(registry.bindings.len());
    for (mapping_index, binding) in registry.bindings.iter().enumerate() {
        let external = validation_by_index[&mapping_index];
        let mut issues = structural_issues(binding)?;
        if duplicate_keys.contains(&binding.key()) {
            issues.push(MigrationValidationIssue::new(
                "duplicate_v1_identity",
                format!(
                    "v1 identity {:?} {:?} appears more than once",
                    binding.identity_kind, binding.identity
                ),
            )?);
        }
        issues.extend(external.issues.iter().cloned());
        issues.sort();
        issues.dedup();
        let (mut planned_maturity, mut identity_locator, mut root_context_locator) =
            locator_plan(binding);
        let target = binding.target();
        let target_digest = target.digest()?;
        let mut historical_identity_locator = None;
        let mut ownership_repair = None;
        if let Some(repair) = repair_by_key.get(&binding.key()) {
            let prior_identity = identity_locator.take().ok_or_else(|| {
                WorkVcsError::ControlPlaneInvalid(format!(
                    "migration ownership repair cannot preserve unsupported v1 identity kind {:?}",
                    binding.identity_kind
                ))
            })?;
            historical_identity_locator =
                Some(locator_with_state(prior_identity, LocatorState::Retired));
            root_context_locator = root_context_locator
                .map(|locator| locator_with_state(locator, LocatorState::Retired));
            identity_locator = Some(semantic_repair_locator_plan(repair.semantic_locator()));
            planned_maturity = Some(ProjectMaturity::Established);
            ownership_repair = Some(MigrationOwnershipRepairPreview {
                repair_digest: repair.digest()?,
                expected_target_digest: repair.expected_target_digest.clone(),
                historical_identity_disposition: repair.historical_identity_disposition,
            });
        }
        mappings.push(MigrationPreviewMapping {
            mapping_index,
            v1_binding_key: binding.key(),
            root: binding.root.clone(),
            planned_maturity,
            identity_locator,
            historical_identity_locator,
            root_context_locator,
            ownership_repair,
            target,
            target_digest,
            validation: MigrationPreviewValidation {
                valid: issues.is_empty(),
                local_content_objects_verified: external.local_content_objects_verified,
                issues,
            },
        });
    }

    let mut target_groups = BTreeMap::<MigrationTarget, Vec<&MigrationPreviewMapping>>::new();
    for mapping in &mappings {
        target_groups
            .entry(mapping.target.clone())
            .or_default()
            .push(mapping);
    }
    let mut target_coincidences = target_groups
        .into_iter()
        .filter_map(|(target, group)| {
            (group.len() > 1).then(|| {
                let target_digest = target.digest()?;
                Ok(MigrationTargetCoincidence {
                    target,
                    target_digest,
                    mapping_indices: group.iter().map(|mapping| mapping.mapping_index).collect(),
                    v1_binding_keys: group
                        .iter()
                        .map(|mapping| mapping.v1_binding_key.clone())
                        .collect(),
                })
            })
        })
        .collect::<Result<Vec<_>>>()?;
    target_coincidences.sort_by(|left, right| {
        left.target_digest
            .to_string()
            .cmp(&right.target_digest.to_string())
    });

    let source_digest = registry.source_digest()?;
    let repair_manifest_digest = repair_manifest
        .map(MigrationOwnershipRepairManifest::digest)
        .transpose()?;
    let ownership_repairs = repair_by_key.len();
    let preview_version = if repair_manifest.is_some() {
        MIGRATION_REPAIR_PREVIEW_VERSION
    } else {
        MIGRATION_PREVIEW_VERSION
    };
    let apply_eligible = mappings.iter().all(|mapping| mapping.validation.valid);
    let digest = PreviewDigestMaterial {
        preview_version,
        source_registry_version: registry.version,
        target_registry_version: REGISTRY_V2_VERSION,
        migration_required: true,
        source_digest: &source_digest,
        repair_manifest_digest: repair_manifest_digest.as_ref(),
        ownership_repairs,
        mappings: &mappings,
        target_coincidences: &target_coincidences,
        apply_eligible,
    }
    .digest()?;
    Ok(RegistryMigrationPreview {
        preview_version,
        source_registry_version: registry.version,
        target_registry_version: REGISTRY_V2_VERSION,
        migration_required: true,
        source_digest,
        repair_manifest_digest,
        ownership_repairs,
        mappings,
        target_coincidences,
        apply_eligible,
        preview_digest: digest,
    })
}

#[derive(Serialize)]
struct MigratedLocatorEvidenceMaterial<'a> {
    source_digest: &'a ControlPlaneDigest,
    v1_binding_key: &'a V1BindingKey,
    role: LocatorRole,
    authority: LocatorAuthority,
    provider: &'a str,
    namespace: &'a str,
    kind: &'a str,
    normalized_value: &'a str,
    assurance: LocatorAssurance,
    source_adapter: &'a str,
}

pub fn materialize_registry_v1_migration_candidate(
    preview: &RegistryMigrationPreview,
    applied_at: &UtcTimestamp,
    backup_path: &CanonicalPath,
    backup_digest: &ControlPlaneDigest,
) -> Result<ProjectRegistryV2> {
    preview.canonical_json_bytes()?;
    if preview.source_registry_version != REGISTRY_V1_VERSION
        || preview.target_registry_version != REGISTRY_V2_VERSION
        || !preview.migration_required
    {
        return invalid("migration candidate requires a v1-to-v2 migration preview");
    }
    if !preview.apply_eligible {
        return invalid("migration candidate requires an apply-eligible preview");
    }

    let registry_id = RegistryId::new_v7();
    let path_namespace = format!("registry:{registry_id}");
    let project_ref_ids = preview
        .mappings
        .iter()
        .map(|_| ProjectRefId::new_v7())
        .collect::<Vec<_>>();

    let mut projects = Vec::with_capacity(preview.mappings.len());
    let mut locators = Vec::new();
    let mut bindings = Vec::with_capacity(preview.mappings.len());
    let mut mapping_receipts = Vec::with_capacity(preview.mappings.len());

    for (mapping, project_ref_id) in preview.mappings.iter().zip(&project_ref_ids) {
        let maturity = mapping.planned_maturity.ok_or_else(|| {
            WorkVcsError::ControlPlaneInvalid(format!(
                "migration mapping {} has no valid ProjectRef maturity",
                mapping.mapping_index
            ))
        })?;
        projects.push(json!({
            "project_ref_id": project_ref_id,
            "maturity": maturity,
            "display_name": null,
            "created_at": applied_at,
            "created_by": "migration"
        }));

        for locator in [
            mapping.identity_locator.as_ref(),
            mapping.historical_identity_locator.as_ref(),
            mapping.root_context_locator.as_ref(),
        ]
        .into_iter()
        .flatten()
        {
            locators.push(migration_locator_value(
                locator,
                mapping,
                *project_ref_id,
                ProjectLocatorId::new_v7(),
                &path_namespace,
                applied_at,
                &preview.source_digest,
            )?);
        }

        bindings.push(json!({
            "project_ref_id": project_ref_id,
            "store_path": mapping.target.store_path,
            "store_id": mapping.target.store_id,
            "workspace_id": mapping.target.workspace_id,
            "branch_id": mapping.target.branch_id,
            "bound_at": applied_at,
            "binding_source": "migration"
        }));
        mapping_receipts.push(json!({
            "v1_identity_kind": mapping.v1_binding_key.identity_kind,
            "v1_identity": mapping.v1_binding_key.identity,
            "project_ref_id": project_ref_id
        }));
    }

    let mut observations = Vec::with_capacity(preview.target_coincidences.len());
    for coincidence in &preview.target_coincidences {
        let mut referenced_projects = coincidence
            .mapping_indices
            .iter()
            .map(|index| {
                project_ref_ids.get(*index).copied().ok_or_else(|| {
                    WorkVcsError::ControlPlaneInvalid(format!(
                        "target coincidence refers to missing migration mapping {index}"
                    ))
                })
            })
            .collect::<Result<Vec<_>>>()?;
        referenced_projects.sort();
        referenced_projects.dedup();
        observations.push(json!({
            "observation_id": RegistryObservationId::new_v7(),
            "kind": "possible_shared_target",
            "project_refs": referenced_projects,
            "target_digest": coincidence.target_digest,
            "observed_at": applied_at,
            "source": "migration"
        }));
    }

    sort_json_values_by_string_field(&mut projects, "project_ref_id")?;
    sort_json_values_by_string_field(&mut locators, "locator_id")?;
    sort_json_values_by_string_field(&mut bindings, "project_ref_id")?;
    sort_json_values_by_string_field(&mut observations, "observation_id")?;

    let value = json!({
        "version": REGISTRY_V2_VERSION,
        "registry_id": registry_id,
        "revision": 1,
        "projects": projects,
        "locators": locators,
        "bindings": bindings,
        "links": [],
        "observations": observations,
        "migration": {
            "from_version": REGISTRY_V1_VERSION,
            "source_digest": preview.source_digest,
            "preview_digest": preview.preview_digest,
            "applied_at": applied_at,
            "backup_path": backup_path,
            "backup_digest": backup_digest,
            "mappings": mapping_receipts
        }
    });
    let encoded = serde_json::to_vec(&value).map_err(|error| {
        WorkVcsError::ControlPlaneInvalid(format!(
            "cannot encode registry v2 migration candidate: {error}"
        ))
    })?;
    ProjectRegistryV2::from_json_bytes(&encoded)
}

fn migration_locator_value(
    locator: &MigrationLocatorPlan,
    mapping: &MigrationPreviewMapping,
    project_ref_id: ProjectRefId,
    locator_id: ProjectLocatorId,
    path_namespace: &str,
    applied_at: &UtcTimestamp,
    source_digest: &ControlPlaneDigest,
) -> Result<Value> {
    let namespace = match locator.namespace_strategy {
        MigrationNamespaceStrategy::NewRegistryId => {
            if locator.namespace.is_some() {
                return invalid(
                    "new_registry_id migration locator must not carry an explicit namespace",
                );
            }
            path_namespace
        }
        MigrationNamespaceStrategy::Explicit => locator.namespace.as_deref().ok_or_else(|| {
            WorkVcsError::ControlPlaneInvalid(
                "explicit migration locator requires an explicit namespace".to_owned(),
            )
        })?,
    };
    let evidence_digest = locator
        .evidence_digest
        .clone()
        .unwrap_or(ControlPlaneDigest::raw(&canonicalize_serializable(
            &MigratedLocatorEvidenceMaterial {
                source_digest,
                v1_binding_key: &mapping.v1_binding_key,
                role: locator.role,
                authority: locator.authority,
                provider: &locator.provider,
                namespace,
                kind: &locator.kind,
                normalized_value: &locator.normalized_value,
                assurance: locator.assurance,
                source_adapter: &locator.source_adapter,
            },
            "migrated locator evidence",
        )?));
    Ok(json!({
        "locator_id": locator_id,
        "project_ref_id": project_ref_id,
        "role": locator.role,
        "authority": locator.authority,
        "provider": locator.provider,
        "namespace": namespace,
        "kind": locator.kind,
        "normalized_value": locator.normalized_value,
        "assurance": locator.assurance,
        "source_adapter": locator.source_adapter,
        "evidence_digest": evidence_digest,
        "observed_at": applied_at,
        "state": locator.state.unwrap_or(LocatorState::Active)
    }))
}

fn sort_json_values_by_string_field(values: &mut [Value], field: &str) -> Result<()> {
    for value in values.iter() {
        if value.get(field).and_then(Value::as_str).is_none() {
            return invalid(format!(
                "registry v2 candidate item is missing string field {field}"
            ));
        }
    }
    values.sort_by(|left, right| {
        left[field]
            .as_str()
            .expect("validated string field")
            .cmp(right[field].as_str().expect("validated string field"))
    });
    Ok(())
}

fn structural_issues(binding: &ProjectBindingV1) -> Result<Vec<MigrationValidationIssue>> {
    let mut issues = Vec::new();
    for (label, value) in [
        ("identity", binding.identity.as_str()),
        ("root", binding.root.as_str()),
        ("store_path", binding.store_path.as_str()),
        ("store_id", binding.store_id.as_str()),
        ("workspace_id", binding.workspace_id.as_str()),
        ("branch_id", binding.branch_id.as_str()),
    ] {
        if value.trim().is_empty() {
            issues.push(MigrationValidationIssue::new(
                "empty_v1_field",
                format!("v1 binding field {label} must not be empty"),
            )?);
        }
    }
    if !matches!(binding.identity_kind.as_str(), "git-common-dir" | "cwd") {
        issues.push(MigrationValidationIssue::new(
            "unknown_v1_identity_kind",
            format!(
                "v1 identity kind {:?} is not supported",
                binding.identity_kind
            ),
        )?);
    }
    for (code, label, value) in [
        (
            "invalid_v1_identity_path",
            "identity",
            binding.identity.as_str(),
        ),
        ("invalid_v1_root_path", "root", binding.root.as_str()),
        (
            "invalid_v1_store_path",
            "store_path",
            binding.store_path.as_str(),
        ),
    ] {
        if let Err(error) = CanonicalPath::parse(value) {
            issues.push(MigrationValidationIssue::new(
                code,
                format!("v1 {label} is invalid: {error}"),
            )?);
        }
    }
    if binding.identity_kind == "cwd" && binding.identity != binding.root {
        issues.push(MigrationValidationIssue::new(
            "cwd_identity_root_mismatch",
            "v1 cwd identity must equal its root",
        )?);
    }
    if let Err(error) = StoreId::parse_canonical(&binding.store_id) {
        issues.push(MigrationValidationIssue::new(
            "invalid_v1_store_id",
            format!("v1 store_id is invalid: {error}"),
        )?);
    }
    if let Err(error) = WorkspaceId::parse_canonical(&binding.workspace_id) {
        issues.push(MigrationValidationIssue::new(
            "invalid_v1_workspace_id",
            format!("v1 workspace_id is invalid: {error}"),
        )?);
    }
    if let Err(error) = BranchId::parse_canonical(&binding.branch_id) {
        issues.push(MigrationValidationIssue::new(
            "invalid_v1_branch_id",
            format!("v1 branch_id is invalid: {error}"),
        )?);
    }
    Ok(issues)
}

fn locator_plan(
    binding: &ProjectBindingV1,
) -> (
    Option<ProjectMaturity>,
    Option<MigrationLocatorPlan>,
    Option<MigrationLocatorPlan>,
) {
    let (maturity, authority, provider, kind) = match binding.identity_kind.as_str() {
        "git-common-dir" => (
            ProjectMaturity::Established,
            LocatorAuthority::Repository,
            "git",
            "git_common_dir",
        ),
        "cwd" => (
            ProjectMaturity::Provisional,
            LocatorAuthority::Cwd,
            "filesystem",
            "canonical_directory",
        ),
        _ => return (None, None, None),
    };
    let identity_locator = MigrationLocatorPlan {
        role: LocatorRole::Identity,
        authority,
        provider: provider.to_owned(),
        namespace_strategy: MigrationNamespaceStrategy::NewRegistryId,
        namespace: None,
        kind: kind.to_owned(),
        normalized_value: binding.identity.clone(),
        assurance: LocatorAssurance::VerifiedDerived,
        source_adapter: MIGRATION_SOURCE_ADAPTER.to_owned(),
        evidence_digest: None,
        state: None,
    };
    let root_context_locator = (binding.root != binding.identity).then(|| MigrationLocatorPlan {
        role: LocatorRole::Context,
        authority: LocatorAuthority::Cwd,
        provider: "filesystem".to_owned(),
        namespace_strategy: MigrationNamespaceStrategy::NewRegistryId,
        namespace: None,
        kind: "canonical_directory".to_owned(),
        normalized_value: binding.root.clone(),
        assurance: LocatorAssurance::VerifiedDerived,
        source_adapter: MIGRATION_SOURCE_ADAPTER.to_owned(),
        evidence_digest: None,
        state: None,
    });
    (Some(maturity), Some(identity_locator), root_context_locator)
}

fn locator_with_state(
    mut locator: MigrationLocatorPlan,
    state: LocatorState,
) -> MigrationLocatorPlan {
    locator.state = Some(state);
    locator
}

fn semantic_repair_locator_plan(evidence: &LocatorEvidence) -> MigrationLocatorPlan {
    MigrationLocatorPlan {
        role: LocatorRole::Identity,
        authority: LocatorAuthority::SemanticProject,
        provider: evidence.provider().to_owned(),
        namespace_strategy: MigrationNamespaceStrategy::Explicit,
        namespace: Some(evidence.namespace().to_owned()),
        kind: evidence.kind().to_owned(),
        normalized_value: evidence.normalized_value().to_owned(),
        assurance: evidence.assurance(),
        source_adapter: evidence.source_adapter().to_owned(),
        evidence_digest: Some(evidence.evidence_digest().clone()),
        state: Some(LocatorState::Active),
    }
}
