use super::migration::ProjectRegistryV1;
use super::model::{
    CanonicalPath, CaptureGroupIntent, ControlPlaneDigest, LocatorAssurance, LocatorEvidence,
    NonEmptyString, Nullable, ProjectMaturity, ProjectRegistryV2, ResolutionContext, UtcTimestamp,
    canonicalize_serializable, invalid,
};
use super::resolver::{ResolutionResult, ResolutionStatus};
use super::safety::contains_secret_bearing_field;
use crate::canonical::{CanonicalValue, canonical_bytes, parse_canonical_json};
use crate::error::{Result, WorkVcsError};
use crate::{
    BranchId, CaptureGroupId, CaptureId, ChangeSetId, CommitId, DeliveryId, Digest, EntityId,
    EntityVersionId, EventId, EvidenceId, ProjectLocatorId, ProjectRefId, RegistryId, RelationId,
    RelationVersionId, StoreId, WorkspaceId,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeSet;
use std::ffi::OsString;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const JOURNAL_VERSION: u64 = 1;
const JOURNAL_LOCK_TIMEOUT: Duration = Duration::from_secs(2);
const JOURNAL_LOCK_RETRY: Duration = Duration::from_millis(10);
const STANDARD_PROJECT_REGISTRY_FILE: &str = "project-bindings.json";
static JOURNAL_LOCK_NONCE: AtomicU64 = AtomicU64::new(0);
const MAX_IDEMPOTENCY_KEY_BYTES: usize = 512;
const MAX_VALUE_REASON_BYTES: usize = 2_048;
pub const MAX_SEMANTIC_PAYLOAD_BYTES: usize = 64 * 1_024;
pub const MAX_CAPTURE_INTENT_BYTES: usize = 256 * 1_024;
pub const MAX_CAPTURE_EVENT_BYTES: usize = 128 * 1_024;
pub const MAX_CAPTURE_PROJECTION_BYTES: usize = 256 * 1_024;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapturePayloadKind {
    CognitionV2,
    LegacyCognitionV1,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResolutionRecordedPayload {
    registry_id: RegistryId,
    registry_revision: u64,
    registry_digest: ControlPlaneDigest,
    resolution: ResolutionResult,
}

impl ResolutionRecordedPayload {
    pub fn new(
        registry_id: RegistryId,
        registry_revision: u64,
        registry_digest: ControlPlaneDigest,
        resolution: ResolutionResult,
    ) -> Result<Self> {
        let payload = Self {
            registry_id,
            registry_revision,
            registry_digest,
            resolution,
        };
        payload.validate()?;
        Ok(payload)
    }

    pub fn registry_id(&self) -> RegistryId {
        self.registry_id
    }

    pub fn registry_revision(&self) -> u64 {
        self.registry_revision
    }

    pub fn registry_digest(&self) -> &ControlPlaneDigest {
        &self.registry_digest
    }

    pub fn resolution(&self) -> &ResolutionResult {
        &self.resolution
    }

    fn validate(&self) -> Result<()> {
        self.resolution.validate()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectBindingReadyPayload {
    registry_id: RegistryId,
    registry_revision: u64,
    registry_digest: ControlPlaneDigest,
    project_ref_id: ProjectRefId,
    project_maturity: ProjectMaturity,
    locator_id: ProjectLocatorId,
    locator_evidence: LocatorEvidence,
    store_path: CanonicalPath,
    store_id: StoreId,
    workspace_id: WorkspaceId,
    branch_id: BranchId,
}

impl ProjectBindingReadyPayload {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        registry_id: RegistryId,
        registry_revision: u64,
        registry_digest: ControlPlaneDigest,
        project_ref_id: ProjectRefId,
        project_maturity: ProjectMaturity,
        locator_id: ProjectLocatorId,
        locator_evidence: LocatorEvidence,
        store_path: CanonicalPath,
        store_id: StoreId,
        workspace_id: WorkspaceId,
        branch_id: BranchId,
    ) -> Result<Self> {
        let payload = Self {
            registry_id,
            registry_revision,
            registry_digest,
            project_ref_id,
            project_maturity,
            locator_id,
            locator_evidence,
            store_path,
            store_id,
            workspace_id,
            branch_id,
        };
        payload.validate()?;
        Ok(payload)
    }

    pub fn registry_id(&self) -> RegistryId {
        self.registry_id
    }

    pub fn registry_revision(&self) -> u64 {
        self.registry_revision
    }

    pub fn registry_digest(&self) -> &ControlPlaneDigest {
        &self.registry_digest
    }

    pub fn project_ref_id(&self) -> ProjectRefId {
        self.project_ref_id
    }

    pub fn project_maturity(&self) -> ProjectMaturity {
        self.project_maturity
    }

    pub fn locator_id(&self) -> ProjectLocatorId {
        self.locator_id
    }

    pub fn locator_evidence(&self) -> &LocatorEvidence {
        &self.locator_evidence
    }

    pub fn store_path(&self) -> &CanonicalPath {
        &self.store_path
    }

    pub fn store_id(&self) -> StoreId {
        self.store_id
    }

    pub fn workspace_id(&self) -> WorkspaceId {
        self.workspace_id
    }

    pub fn branch_id(&self) -> BranchId {
        self.branch_id
    }

    fn validate(&self) -> Result<()> {
        self.locator_evidence.validate()?;
        if self.locator_evidence.assurance() == LocatorAssurance::Observed {
            return invalid("project_binding_ready requires eligible identity locator evidence");
        }
        match (self.project_maturity, self.locator_evidence.authority()) {
            (
                ProjectMaturity::Provisional | ProjectMaturity::Established,
                super::model::LocatorAuthority::Cwd,
            )
            | (
                ProjectMaturity::Established,
                super::model::LocatorAuthority::SemanticProject
                | super::model::LocatorAuthority::Repository,
            ) => Ok(()),
            _ => invalid(
                "project_binding_ready maturity must be established for semantic/repository ownership; CWD ownership may be provisional or reference an already established ProjectRef",
            ),
        }
    }

    fn matches_delivery_target(&self, started: &DeliveryStartedPayload) -> bool {
        self.project_ref_id == started.project_ref_id()
            && self.store_id == started.store_id()
            && self.workspace_id == started.workspace_id()
            && self.branch_id == started.branch_id()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeliveryMode {
    Canonical,
    ImmutableReference,
}

impl DeliveryMode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Canonical => "canonical",
            Self::ImmutableReference => "immutable_reference",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeliveryStartedPayload {
    delivery_id: DeliveryId,
    delivery_mode: DeliveryMode,
    project_ref_id: ProjectRefId,
    store_id: StoreId,
    workspace_id: WorkspaceId,
    branch_id: BranchId,
    expected_head_commit_id: CommitId,
    expected_state_digest: Digest,
    target_idempotency_key: NonEmptyString,
    target_manifest_digest: Digest,
}

impl DeliveryStartedPayload {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        delivery_id: DeliveryId,
        delivery_mode: DeliveryMode,
        project_ref_id: ProjectRefId,
        store_id: StoreId,
        workspace_id: WorkspaceId,
        branch_id: BranchId,
        expected_head_commit_id: CommitId,
        expected_state_digest: Digest,
        target_idempotency_key: impl Into<String>,
        target_manifest_digest: Digest,
    ) -> Result<Self> {
        let payload = Self {
            delivery_id,
            delivery_mode,
            project_ref_id,
            store_id,
            workspace_id,
            branch_id,
            expected_head_commit_id,
            expected_state_digest,
            target_idempotency_key: NonEmptyString::new(
                target_idempotency_key,
                "delivery target idempotency key",
            )?,
            target_manifest_digest,
        };
        payload.validate()?;
        Ok(payload)
    }

    pub fn delivery_id(&self) -> DeliveryId {
        self.delivery_id
    }

    pub fn delivery_mode(&self) -> DeliveryMode {
        self.delivery_mode
    }

    pub fn project_ref_id(&self) -> ProjectRefId {
        self.project_ref_id
    }

    pub fn store_id(&self) -> StoreId {
        self.store_id
    }

    pub fn workspace_id(&self) -> WorkspaceId {
        self.workspace_id
    }

    pub fn branch_id(&self) -> BranchId {
        self.branch_id
    }

    pub fn expected_head_commit_id(&self) -> CommitId {
        self.expected_head_commit_id
    }

    pub fn expected_state_digest(&self) -> Digest {
        self.expected_state_digest
    }

    pub fn target_idempotency_key(&self) -> &str {
        self.target_idempotency_key.as_str()
    }

    pub fn target_manifest_digest(&self) -> Digest {
        self.target_manifest_digest
    }

    fn validate(&self) -> Result<()> {
        if self.delivery_mode != DeliveryMode::Canonical {
            return invalid("primary delivery_started must use canonical delivery mode");
        }
        if self.target_idempotency_key.as_str().len() > 256 {
            return invalid("delivery target idempotency key must be at most 256 bytes");
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeliveryResultObject {
    local_id: NonEmptyString,
    object_kind: NonEmptyString,
    logical_object_id: NonEmptyString,
    immutable_version_id: NonEmptyString,
    version_digest: Digest,
}

impl DeliveryResultObject {
    pub fn new(
        local_id: impl Into<String>,
        object_kind: impl Into<String>,
        logical_object_id: impl Into<String>,
        immutable_version_id: impl Into<String>,
        version_digest: Digest,
    ) -> Result<Self> {
        let object = Self {
            local_id: NonEmptyString::new(local_id, "delivery result local_id")?,
            object_kind: NonEmptyString::new(object_kind, "delivery result object_kind")?,
            logical_object_id: NonEmptyString::new(
                logical_object_id,
                "delivery result logical_object_id",
            )?,
            immutable_version_id: NonEmptyString::new(
                immutable_version_id,
                "delivery result immutable_version_id",
            )?,
            version_digest,
        };
        object.validate()?;
        Ok(object)
    }

    pub fn local_id(&self) -> &str {
        self.local_id.as_str()
    }

    pub fn object_kind(&self) -> &str {
        self.object_kind.as_str()
    }

    pub fn logical_object_id(&self) -> &str {
        self.logical_object_id.as_str()
    }

    pub fn immutable_version_id(&self) -> &str {
        self.immutable_version_id.as_str()
    }

    pub fn version_digest(&self) -> Digest {
        self.version_digest
    }

    fn validate(&self) -> Result<()> {
        let logical = self.logical_object_id.as_str();
        let version = self.immutable_version_id.as_str();
        let parse_error = |field: &str, error: WorkVcsError| {
            WorkVcsError::ControlPlaneInvalid(format!(
                "delivery result {} {field} is invalid: {error}",
                self.object_kind.as_str()
            ))
        };
        match self.object_kind.as_str() {
            kind if kind
                .strip_prefix("record:")
                .is_some_and(|suffix| !suffix.is_empty()) =>
            {
                EntityId::parse_canonical(logical)
                    .map_err(|error| parse_error("logical_object_id", error))?;
                EntityVersionId::parse_canonical(version)
                    .map_err(|error| parse_error("immutable_version_id", error))?;
            }
            "knowledge" => {
                EntityId::parse_canonical(logical)
                    .map_err(|error| parse_error("logical_object_id", error))?;
                EntityVersionId::parse_canonical(version)
                    .map_err(|error| parse_error("immutable_version_id", error))?;
            }
            "evidence" => {
                EvidenceId::parse_canonical(logical)
                    .map_err(|error| parse_error("logical_object_id", error))?;
                EvidenceId::parse_canonical(version)
                    .map_err(|error| parse_error("immutable_version_id", error))?;
                if logical != version {
                    return invalid(
                        "delivery Evidence logical_object_id and immutable_version_id must match",
                    );
                }
            }
            kind if kind
                .strip_prefix("relation:")
                .is_some_and(|suffix| !suffix.is_empty()) =>
            {
                RelationId::parse_canonical(logical)
                    .map_err(|error| parse_error("logical_object_id", error))?;
                RelationVersionId::parse_canonical(version)
                    .map_err(|error| parse_error("immutable_version_id", error))?;
            }
            kind => {
                return invalid(format!(
                    "delivery result object_kind {kind:?} is not supported"
                ));
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CanonicalRecordRef {
    project_ref_id: ProjectRefId,
    store_id: StoreId,
    workspace_id: WorkspaceId,
    record_id: NonEmptyString,
    record_version_id: NonEmptyString,
    version_digest: Digest,
}

impl CanonicalRecordRef {
    pub fn new(
        project_ref_id: ProjectRefId,
        store_id: StoreId,
        workspace_id: WorkspaceId,
        record_id: impl Into<String>,
        record_version_id: impl Into<String>,
        version_digest: Digest,
    ) -> Result<Self> {
        let reference = Self {
            project_ref_id,
            store_id,
            workspace_id,
            record_id: NonEmptyString::new(record_id, "canonical record id")?,
            record_version_id: NonEmptyString::new(
                record_version_id,
                "canonical record version id",
            )?,
            version_digest,
        };
        reference.validate()?;
        Ok(reference)
    }

    pub fn project_ref_id(&self) -> ProjectRefId {
        self.project_ref_id
    }

    pub fn store_id(&self) -> StoreId {
        self.store_id
    }

    pub fn workspace_id(&self) -> WorkspaceId {
        self.workspace_id
    }

    pub fn record_id(&self) -> &str {
        self.record_id.as_str()
    }

    pub fn record_version_id(&self) -> &str {
        self.record_version_id.as_str()
    }

    pub fn version_digest(&self) -> Digest {
        self.version_digest
    }

    fn validate(&self) -> Result<()> {
        EntityId::parse_canonical(self.record_id.as_str()).map_err(|error| {
            WorkVcsError::ControlPlaneInvalid(format!("canonical record id is invalid: {error}"))
        })?;
        EntityVersionId::parse_canonical(self.record_version_id.as_str()).map_err(|error| {
            WorkVcsError::ControlPlaneInvalid(format!(
                "canonical record version id is invalid: {error}"
            ))
        })?;
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeliveryAppliedPayload {
    delivery_id: DeliveryId,
    delivery_mode: DeliveryMode,
    project_ref_id: ProjectRefId,
    store_id: StoreId,
    workspace_id: WorkspaceId,
    branch_id: BranchId,
    commit_id: CommitId,
    changeset_id: ChangeSetId,
    state_digest: Digest,
    result_objects: Vec<DeliveryResultObject>,
    reused: bool,
    #[serde(default)]
    canonical_record_ref: Nullable<CanonicalRecordRef>,
}

impl DeliveryAppliedPayload {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        delivery_id: DeliveryId,
        project_ref_id: ProjectRefId,
        store_id: StoreId,
        workspace_id: WorkspaceId,
        branch_id: BranchId,
        commit_id: CommitId,
        changeset_id: ChangeSetId,
        state_digest: Digest,
        result_objects: Vec<DeliveryResultObject>,
        reused: bool,
        canonical_record_ref: Option<CanonicalRecordRef>,
    ) -> Result<Self> {
        let payload = Self {
            delivery_id,
            delivery_mode: DeliveryMode::Canonical,
            project_ref_id,
            store_id,
            workspace_id,
            branch_id,
            commit_id,
            changeset_id,
            state_digest,
            result_objects,
            reused,
            canonical_record_ref: Nullable::present(canonical_record_ref),
        };
        payload.validate()?;
        Ok(payload)
    }

    pub fn delivery_id(&self) -> DeliveryId {
        self.delivery_id
    }

    pub fn delivery_mode(&self) -> DeliveryMode {
        self.delivery_mode
    }

    pub fn project_ref_id(&self) -> ProjectRefId {
        self.project_ref_id
    }

    pub fn store_id(&self) -> StoreId {
        self.store_id
    }

    pub fn workspace_id(&self) -> WorkspaceId {
        self.workspace_id
    }

    pub fn branch_id(&self) -> BranchId {
        self.branch_id
    }

    pub fn commit_id(&self) -> CommitId {
        self.commit_id
    }

    pub fn changeset_id(&self) -> ChangeSetId {
        self.changeset_id
    }

    pub fn state_digest(&self) -> Digest {
        self.state_digest
    }

    pub fn result_objects(&self) -> &[DeliveryResultObject] {
        &self.result_objects
    }

    pub fn reused(&self) -> bool {
        self.reused
    }

    pub fn canonical_record_ref(&self) -> Option<&CanonicalRecordRef> {
        self.canonical_record_ref.as_ref()
    }

    fn validate(&self) -> Result<()> {
        if self.delivery_mode != DeliveryMode::Canonical {
            return invalid("primary delivery_applied must use canonical delivery mode");
        }
        if !self.canonical_record_ref.is_present() {
            return invalid("delivery_applied canonical_record_ref is required even when null");
        }
        if self.result_objects.is_empty() {
            return invalid("delivery_applied requires at least one result object");
        }
        let mut local_ids = BTreeSet::new();
        let mut immutable_objects = BTreeSet::new();
        let mut has_canonical_content = false;
        for object in &self.result_objects {
            object.validate()?;
            if !local_ids.insert(object.local_id.as_str()) {
                return invalid(format!(
                    "delivery_applied result local_id {:?} is duplicated",
                    object.local_id.as_str()
                ));
            }
            if !immutable_objects.insert((
                object.object_kind.as_str(),
                object.logical_object_id.as_str(),
                object.immutable_version_id.as_str(),
            )) {
                return invalid(format!(
                    "delivery_applied immutable result {}:{}:{} is duplicated",
                    object.object_kind.as_str(),
                    object.logical_object_id.as_str(),
                    object.immutable_version_id.as_str()
                ));
            }
            has_canonical_content |= object.object_kind.as_str() == "knowledge"
                || object.object_kind.as_str().starts_with("record:");
        }
        if !has_canonical_content {
            return invalid("delivery_applied requires at least one Record or Knowledge result");
        }
        if let Some(reference) = self.canonical_record_ref.as_ref() {
            reference.validate()?;
            if reference.project_ref_id != self.project_ref_id
                || reference.store_id != self.store_id
                || reference.workspace_id != self.workspace_id
            {
                return invalid("canonical record reference target must match delivery receipt");
            }
            let matching = self.result_objects.iter().any(|object| {
                object.object_kind.as_str().starts_with("record:")
                    && object.logical_object_id.as_str() == reference.record_id.as_str()
                    && object.immutable_version_id.as_str() == reference.record_version_id.as_str()
                    && object.version_digest == reference.version_digest
            });
            if !matching {
                return invalid("canonical record reference must name a delivered Record result");
            }
        }
        Ok(())
    }

    fn matches_started(&self, started: &DeliveryStartedPayload) -> bool {
        self.delivery_id == started.delivery_id
            && self.delivery_mode == started.delivery_mode
            && self.project_ref_id == started.project_ref_id
            && self.store_id == started.store_id
            && self.workspace_id == started.workspace_id
            && self.branch_id == started.branch_id
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeliveryFailureCode {
    LegacyManifestUpgradeRequired,
}

impl DeliveryFailureCode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::LegacyManifestUpgradeRequired => "legacy_manifest_upgrade_required",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeliveryFailedPayload {
    delivery_id: DeliveryId,
    delivery_mode: DeliveryMode,
    project_ref_id: ProjectRefId,
    store_id: StoreId,
    workspace_id: WorkspaceId,
    branch_id: BranchId,
    failure_code: DeliveryFailureCode,
    recovery_action: NonEmptyString,
}

impl DeliveryFailedPayload {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        delivery_id: DeliveryId,
        project_ref_id: ProjectRefId,
        store_id: StoreId,
        workspace_id: WorkspaceId,
        branch_id: BranchId,
        failure_code: DeliveryFailureCode,
        recovery_action: impl Into<String>,
    ) -> Result<Self> {
        let payload = Self {
            delivery_id,
            delivery_mode: DeliveryMode::Canonical,
            project_ref_id,
            store_id,
            workspace_id,
            branch_id,
            failure_code,
            recovery_action: NonEmptyString::new(recovery_action, "delivery recovery action")?,
        };
        payload.validate()?;
        Ok(payload)
    }

    pub fn delivery_id(&self) -> DeliveryId {
        self.delivery_id
    }

    pub fn failure_code(&self) -> DeliveryFailureCode {
        self.failure_code
    }

    pub fn recovery_action(&self) -> &str {
        self.recovery_action.as_str()
    }

    fn validate(&self) -> Result<()> {
        if self.delivery_mode != DeliveryMode::Canonical {
            return invalid("primary delivery_failed must use canonical delivery mode");
        }
        Ok(())
    }

    fn matches_started(&self, started: &DeliveryStartedPayload) -> bool {
        self.delivery_id == started.delivery_id
            && self.delivery_mode == started.delivery_mode
            && self.project_ref_id == started.project_ref_id
            && self.store_id == started.store_id
            && self.workspace_id == started.workspace_id
            && self.branch_id == started.branch_id
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CaptureGroupResolvedPayload {
    capture_group_id: CaptureGroupId,
    primary_project_ref: ProjectRefId,
    primary_locator_evidence_digest: ControlPlaneDigest,
    relation: NonEmptyString,
}

impl CaptureGroupResolvedPayload {
    pub fn new(
        capture_group_id: CaptureGroupId,
        primary_project_ref: ProjectRefId,
        primary_locator_evidence_digest: ControlPlaneDigest,
        relation: impl Into<String>,
    ) -> Result<Self> {
        let payload = Self {
            capture_group_id,
            primary_project_ref,
            primary_locator_evidence_digest,
            relation: NonEmptyString::new(relation, "capture group primary relation")?,
        };
        payload.validate()?;
        Ok(payload)
    }

    pub fn capture_group_id(&self) -> CaptureGroupId {
        self.capture_group_id
    }

    pub fn primary_project_ref(&self) -> ProjectRefId {
        self.primary_project_ref
    }

    pub fn primary_locator_evidence_digest(&self) -> &ControlPlaneDigest {
        &self.primary_locator_evidence_digest
    }

    pub fn relation(&self) -> &str {
        self.relation.as_str()
    }

    fn validate(&self) -> Result<()> {
        if self.relation.as_str().len() > 256 {
            return invalid("capture group primary relation must be at most 256 bytes");
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReferenceAppliedPayload {
    capture_group_id: CaptureGroupId,
    secondary_project_ref: ProjectRefId,
    canonical_record_ref: CanonicalRecordRef,
    relation: NonEmptyString,
}

impl ReferenceAppliedPayload {
    pub fn new(
        capture_group_id: CaptureGroupId,
        secondary_project_ref: ProjectRefId,
        canonical_record_ref: CanonicalRecordRef,
        relation: impl Into<String>,
    ) -> Result<Self> {
        let payload = Self {
            capture_group_id,
            secondary_project_ref,
            canonical_record_ref,
            relation: NonEmptyString::new(relation, "secondary reference relation")?,
        };
        payload.validate()?;
        Ok(payload)
    }

    pub fn capture_group_id(&self) -> CaptureGroupId {
        self.capture_group_id
    }

    pub fn secondary_project_ref(&self) -> ProjectRefId {
        self.secondary_project_ref
    }

    pub fn canonical_record_ref(&self) -> &CanonicalRecordRef {
        &self.canonical_record_ref
    }

    pub fn relation(&self) -> &str {
        self.relation.as_str()
    }

    fn validate(&self) -> Result<()> {
        self.canonical_record_ref.validate()?;
        if self.secondary_project_ref == self.canonical_record_ref.project_ref_id() {
            return invalid("secondary reference must not target the canonical ProjectRef");
        }
        if self.relation.as_str().len() > 256 {
            return invalid("secondary reference relation must be at most 256 bytes");
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CaptureCompletedPayload {
    capture_group_id: CaptureGroupId,
    canonical_record_ref: CanonicalRecordRef,
    secondary_project_refs: Vec<ProjectRefId>,
}

impl CaptureCompletedPayload {
    pub fn new(
        capture_group_id: CaptureGroupId,
        canonical_record_ref: CanonicalRecordRef,
        secondary_project_refs: Vec<ProjectRefId>,
    ) -> Result<Self> {
        let payload = Self {
            capture_group_id,
            canonical_record_ref,
            secondary_project_refs,
        };
        payload.validate()?;
        Ok(payload)
    }

    pub fn capture_group_id(&self) -> CaptureGroupId {
        self.capture_group_id
    }

    pub fn canonical_record_ref(&self) -> &CanonicalRecordRef {
        &self.canonical_record_ref
    }

    pub fn secondary_project_refs(&self) -> &[ProjectRefId] {
        &self.secondary_project_refs
    }

    fn validate(&self) -> Result<()> {
        self.canonical_record_ref.validate()?;
        if self
            .secondary_project_refs
            .windows(2)
            .any(|pair| pair[0] >= pair[1])
        {
            return invalid(
                "capture_completed secondary_project_refs must be strictly sorted and unique",
            );
        }
        if self
            .secondary_project_refs
            .contains(&self.canonical_record_ref.project_ref_id())
        {
            return invalid(
                "capture_completed must not list the canonical ProjectRef as secondary",
            );
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "event_kind", content = "payload", rename_all = "snake_case")]
pub enum CaptureEventPayload {
    ResolutionRecorded(ResolutionRecordedPayload),
    ProjectBindingReady(ProjectBindingReadyPayload),
    CaptureGroupResolved(CaptureGroupResolvedPayload),
    DeliveryStarted(DeliveryStartedPayload),
    DeliveryApplied(DeliveryAppliedPayload),
    DeliveryFailed(DeliveryFailedPayload),
    ReferenceApplied(ReferenceAppliedPayload),
    CaptureCompleted(CaptureCompletedPayload),
}

impl CaptureEventPayload {
    fn validate(&self) -> Result<()> {
        match self {
            Self::ResolutionRecorded(payload) => payload.validate(),
            Self::ProjectBindingReady(payload) => payload.validate(),
            Self::CaptureGroupResolved(payload) => payload.validate(),
            Self::DeliveryStarted(payload) => payload.validate(),
            Self::DeliveryApplied(payload) => payload.validate(),
            Self::DeliveryFailed(payload) => payload.validate(),
            Self::ReferenceApplied(payload) => payload.validate(),
            Self::CaptureCompleted(payload) => payload.validate(),
        }
    }

    fn kind_name(&self) -> &'static str {
        match self {
            Self::ResolutionRecorded(_) => "resolution_recorded",
            Self::ProjectBindingReady(_) => "project_binding_ready",
            Self::CaptureGroupResolved(_) => "capture_group_resolved",
            Self::DeliveryStarted(_) => "delivery_started",
            Self::DeliveryApplied(_) => "delivery_applied",
            Self::DeliveryFailed(_) => "delivery_failed",
            Self::ReferenceApplied(_) => "reference_applied",
            Self::CaptureCompleted(_) => "capture_completed",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CaptureEvent {
    journal_version: u64,
    capture_id: CaptureId,
    sequence: u64,
    event_id: EventId,
    occurred_at: UtcTimestamp,
    #[serde(default)]
    previous_event_digest: Nullable<ControlPlaneDigest>,
    payload_digest: ControlPlaneDigest,
    #[serde(flatten)]
    payload: CaptureEventPayload,
}

impl CaptureEvent {
    fn new(
        capture_id: CaptureId,
        sequence: u64,
        event_id: EventId,
        occurred_at: UtcTimestamp,
        previous_event_digest: Option<ControlPlaneDigest>,
        payload: CaptureEventPayload,
    ) -> Result<Self> {
        let payload_digest = ControlPlaneDigest::raw(&canonicalize_serializable(
            &payload,
            "capture event payload",
        )?);
        let event = Self {
            journal_version: JOURNAL_VERSION,
            capture_id,
            sequence,
            event_id,
            occurred_at,
            previous_event_digest: Nullable::present(previous_event_digest),
            payload_digest,
            payload,
        };
        event.validate()?;
        Ok(event)
    }

    pub fn from_json_bytes(input: &[u8]) -> Result<Self> {
        if input.len() > MAX_CAPTURE_EVENT_BYTES {
            return invalid(format!(
                "capture event is {} bytes; maximum is {MAX_CAPTURE_EVENT_BYTES}",
                input.len()
            ));
        }
        parse_canonical_json(input).map_err(|error| {
            WorkVcsError::ControlPlaneInvalid(format!(
                "capture event is not strict canonical-domain JSON: {error}"
            ))
        })?;
        let event: Self = serde_json::from_slice(input).map_err(|error| {
            WorkVcsError::ControlPlaneInvalid(format!(
                "capture event has an invalid shape: {error}"
            ))
        })?;
        event.validate()?;
        Ok(event)
    }

    pub fn validate(&self) -> Result<()> {
        if self.journal_version != JOURNAL_VERSION {
            return invalid(format!(
                "capture event journal_version {} is not supported",
                self.journal_version
            ));
        }
        if self.sequence == 0 {
            return invalid("capture event sequence must start at 1");
        }
        if !self.previous_event_digest.is_present() {
            return invalid("capture event previous_event_digest is required even when null");
        }
        if self.sequence == 1 && self.previous_event_digest.as_ref().is_some() {
            return invalid("first capture event must have null previous_event_digest");
        }
        if self.sequence > 1 && self.previous_event_digest.as_ref().is_none() {
            return invalid("capture event after sequence 1 requires previous_event_digest");
        }
        self.payload.validate()?;
        let actual_payload_digest = ControlPlaneDigest::raw(&canonicalize_serializable(
            &self.payload,
            "capture event payload",
        )?);
        if actual_payload_digest != self.payload_digest {
            return invalid(format!(
                "capture event payload digest {actual_payload_digest} does not match declared {}",
                self.payload_digest
            ));
        }
        let canonical_size = canonicalize_serializable(self, "capture event")?.len();
        if canonical_size > MAX_CAPTURE_EVENT_BYTES {
            return invalid(format!(
                "canonical capture event is {canonical_size} bytes; maximum is {MAX_CAPTURE_EVENT_BYTES}"
            ));
        }
        Ok(())
    }

    pub fn capture_id(&self) -> CaptureId {
        self.capture_id
    }

    pub fn sequence(&self) -> u64 {
        self.sequence
    }

    pub fn event_id(&self) -> EventId {
        self.event_id
    }

    pub fn occurred_at(&self) -> &UtcTimestamp {
        &self.occurred_at
    }

    pub fn previous_event_digest(&self) -> Option<&ControlPlaneDigest> {
        self.previous_event_digest.as_ref()
    }

    pub fn payload(&self) -> &CaptureEventPayload {
        &self.payload
    }

    pub fn canonical_json_bytes(&self) -> Result<Vec<u8>> {
        self.validate()?;
        canonicalize_serializable(self, "capture event")
    }

    pub fn stored_json_bytes(&self) -> Result<Vec<u8>> {
        let mut bytes = self.canonical_json_bytes()?;
        bytes.push(b'\n');
        Ok(bytes)
    }

    pub fn digest(&self) -> Result<ControlPlaneDigest> {
        Ok(ControlPlaneDigest::raw(&self.canonical_json_bytes()?))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CaptureRecoveryState {
    PendingResolution,
    PendingProject,
    PendingPrimary,
    PendingReferences,
    LegacyManifestUpgradeRequired,
    Completed,
}

impl CaptureRecoveryState {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::PendingResolution => "pending_resolution",
            Self::PendingProject => "pending_project",
            Self::PendingPrimary => "pending_primary",
            Self::PendingReferences => "pending_references",
            Self::LegacyManifestUpgradeRequired => "legacy_manifest_upgrade_required",
            Self::Completed => "completed",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImmutableSecondaryReference {
    capture_group_id: CaptureGroupId,
    secondary_project_ref: ProjectRefId,
    canonical_record_ref: CanonicalRecordRef,
    relation: NonEmptyString,
    observed_at: UtcTimestamp,
}

impl ImmutableSecondaryReference {
    fn from_event(payload: &ReferenceAppliedPayload, observed_at: UtcTimestamp) -> Result<Self> {
        let reference = Self {
            capture_group_id: payload.capture_group_id(),
            secondary_project_ref: payload.secondary_project_ref(),
            canonical_record_ref: payload.canonical_record_ref().clone(),
            relation: NonEmptyString::new(payload.relation(), "secondary reference relation")?,
            observed_at,
        };
        reference.validate()?;
        Ok(reference)
    }

    pub fn capture_group_id(&self) -> CaptureGroupId {
        self.capture_group_id
    }

    pub fn secondary_project_ref(&self) -> ProjectRefId {
        self.secondary_project_ref
    }

    pub fn canonical_record_ref(&self) -> &CanonicalRecordRef {
        &self.canonical_record_ref
    }

    pub fn relation(&self) -> &str {
        self.relation.as_str()
    }

    pub fn observed_at(&self) -> &UtcTimestamp {
        &self.observed_at
    }

    fn validate(&self) -> Result<()> {
        self.canonical_record_ref.validate()?;
        if self.secondary_project_ref == self.canonical_record_ref.project_ref_id() {
            return invalid(
                "immutable secondary reference must not target the canonical ProjectRef",
            );
        }
        if self.relation.as_str().len() > 256 {
            return invalid("immutable secondary reference relation must be at most 256 bytes");
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CaptureGroupMemberProjection {
    project_ref_id: ProjectRefId,
    role: super::model::CaptureGroupMemberRole,
    relation: NonEmptyString,
    delivery_mode: super::model::CaptureGroupMemberDelivery,
    #[serde(default)]
    reference: Nullable<ImmutableSecondaryReference>,
}

impl CaptureGroupMemberProjection {
    fn from_intent(member: &super::model::CaptureGroupMember) -> Result<Self> {
        let projection = Self {
            project_ref_id: member.project_ref_id(),
            role: member.role(),
            relation: NonEmptyString::new(member.relation(), "capture group member relation")?,
            delivery_mode: member.delivery_mode(),
            reference: Nullable::present(None),
        };
        projection.validate()?;
        Ok(projection)
    }

    fn resolved_primary(payload: &CaptureGroupResolvedPayload) -> Result<Self> {
        let projection = Self {
            project_ref_id: payload.primary_project_ref(),
            role: super::model::CaptureGroupMemberRole::Primary,
            relation: NonEmptyString::new(payload.relation(), "capture group primary relation")?,
            delivery_mode: super::model::CaptureGroupMemberDelivery::Canonical,
            reference: Nullable::present(None),
        };
        projection.validate()?;
        Ok(projection)
    }

    pub fn project_ref_id(&self) -> ProjectRefId {
        self.project_ref_id
    }

    pub fn role(&self) -> super::model::CaptureGroupMemberRole {
        self.role
    }

    pub fn relation(&self) -> &str {
        self.relation.as_str()
    }

    pub fn delivery_mode(&self) -> super::model::CaptureGroupMemberDelivery {
        self.delivery_mode
    }

    pub fn reference(&self) -> Option<&ImmutableSecondaryReference> {
        self.reference.as_ref()
    }

    fn validate(&self) -> Result<()> {
        if !self.reference.is_present() {
            return invalid("capture group member reference is required even when null");
        }
        if self.relation.as_str().len() > 256 {
            return invalid("capture group member relation must be at most 256 bytes");
        }
        match (self.delivery_mode, self.reference.as_ref()) {
            (super::model::CaptureGroupMemberDelivery::ImmutableReference, Some(reference)) => {
                reference.validate()?;
                if reference.secondary_project_ref() != self.project_ref_id
                    || reference.relation() != self.relation.as_str()
                {
                    return invalid(
                        "capture group member reference must match its ProjectRef and relation",
                    );
                }
            }
            (super::model::CaptureGroupMemberDelivery::ImmutableReference, None)
            | (super::model::CaptureGroupMemberDelivery::Canonical, None)
            | (super::model::CaptureGroupMemberDelivery::None, None) => {}
            (_, Some(_)) => {
                return invalid(
                    "only an immutable_reference capture group member may retain a reference",
                );
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CaptureGroupProjection {
    capture_id: CaptureId,
    capture_group_id: CaptureGroupId,
    #[serde(default)]
    resolved_primary_project_ref: Nullable<ProjectRefId>,
    #[serde(default)]
    primary_locator_evidence_digest: Nullable<ControlPlaneDigest>,
    #[serde(default)]
    canonical_record_ref: Nullable<CanonicalRecordRef>,
    members: Vec<CaptureGroupMemberProjection>,
    unresolved_related_locators: Vec<LocatorEvidence>,
    #[serde(default)]
    completion_receipt: Nullable<CaptureCompletedPayload>,
}

impl CaptureGroupProjection {
    pub fn capture_id(&self) -> CaptureId {
        self.capture_id
    }

    pub fn capture_group_id(&self) -> CaptureGroupId {
        self.capture_group_id
    }

    pub fn resolved_primary_project_ref(&self) -> Option<ProjectRefId> {
        self.resolved_primary_project_ref.copied()
    }

    pub fn primary_locator_evidence_digest(&self) -> Option<&ControlPlaneDigest> {
        self.primary_locator_evidence_digest.as_ref()
    }

    pub fn canonical_record_ref(&self) -> Option<&CanonicalRecordRef> {
        self.canonical_record_ref.as_ref()
    }

    pub fn members(&self) -> &[CaptureGroupMemberProjection] {
        &self.members
    }

    pub fn unresolved_related_locators(&self) -> &[LocatorEvidence] {
        &self.unresolved_related_locators
    }

    pub fn completion_receipt(&self) -> Option<&CaptureCompletedPayload> {
        self.completion_receipt.as_ref()
    }

    pub fn required_reference_count(&self) -> usize {
        self.members
            .iter()
            .filter(|member| {
                member.delivery_mode == super::model::CaptureGroupMemberDelivery::ImmutableReference
            })
            .count()
    }

    pub fn applied_reference_count(&self) -> usize {
        self.members
            .iter()
            .filter(|member| member.reference.as_ref().is_some())
            .count()
    }

    pub fn pending_reference_project_refs(&self) -> Vec<ProjectRefId> {
        self.members
            .iter()
            .filter(|member| {
                member.delivery_mode == super::model::CaptureGroupMemberDelivery::ImmutableReference
                    && member.reference.as_ref().is_none()
            })
            .map(CaptureGroupMemberProjection::project_ref_id)
            .collect()
    }

    pub fn member(&self, project_ref_id: ProjectRefId) -> Option<&CaptureGroupMemberProjection> {
        self.members
            .binary_search_by_key(
                &project_ref_id,
                CaptureGroupMemberProjection::project_ref_id,
            )
            .ok()
            .map(|index| &self.members[index])
    }

    fn validate(&self) -> Result<()> {
        if !self.resolved_primary_project_ref.is_present()
            || !self.primary_locator_evidence_digest.is_present()
            || !self.canonical_record_ref.is_present()
            || !self.completion_receipt.is_present()
        {
            return invalid("capture group projection nullable fields are required even when null");
        }
        if self
            .members
            .windows(2)
            .any(|pair| pair[0].project_ref_id >= pair[1].project_ref_id)
        {
            return invalid("capture group projection members must be strictly sorted and unique");
        }
        for member in &self.members {
            member.validate()?;
        }
        let canonical_members = self
            .members
            .iter()
            .filter(|member| {
                member.delivery_mode == super::model::CaptureGroupMemberDelivery::Canonical
            })
            .collect::<Vec<_>>();
        let primary_role_members = self
            .members
            .iter()
            .filter(|member| member.role == super::model::CaptureGroupMemberRole::Primary)
            .collect::<Vec<_>>();
        match self.resolved_primary_project_ref.copied() {
            Some(primary)
                if canonical_members.len() == 1
                    && canonical_members[0].project_ref_id == primary
                    && canonical_members[0].role
                        == super::model::CaptureGroupMemberRole::Primary
                    && primary_role_members.len() == 1
                    && primary_role_members[0].project_ref_id == primary => {}
            Some(_) => {
                return invalid(
                    "resolved capture group projection requires one canonical primary-role member",
                );
            }
            None if canonical_members.is_empty() && primary_role_members.is_empty() => {}
            None => {
                return invalid(
                    "unresolved capture group projection must not have a primary role or canonical member",
                );
            }
        }
        if let Some(reference) = self.canonical_record_ref.as_ref() {
            reference.validate()?;
            if self.resolved_primary_project_ref.copied() != Some(reference.project_ref_id()) {
                return invalid(
                    "capture group canonical record reference must match its resolved primary",
                );
            }
        }
        let canonical_reference = self.canonical_record_ref.as_ref();
        for member in &self.members {
            if let Some(reference) = member.reference.as_ref()
                && canonical_reference != Some(reference.canonical_record_ref())
            {
                return invalid(
                    "every immutable secondary reference must pin the current canonical record reference",
                );
            }
        }
        let mut locator_keys = BTreeSet::new();
        for evidence in &self.unresolved_related_locators {
            evidence.validate()?;
            if !locator_keys.insert((
                evidence.provider(),
                evidence.namespace(),
                evidence.kind(),
                evidence.normalized_value(),
            )) {
                return invalid(
                    "capture group unresolved related locators must not contain duplicates",
                );
            }
        }
        if let Some(completion) = self.completion_receipt.as_ref() {
            completion.validate()?;
            if completion.capture_group_id() != self.capture_group_id
                || canonical_reference != Some(completion.canonical_record_ref())
            {
                return invalid(
                    "capture group completion receipt must match its group and canonical record",
                );
            }
            let applied = self
                .members
                .iter()
                .filter(|member| member.reference.as_ref().is_some())
                .map(CaptureGroupMemberProjection::project_ref_id)
                .collect::<Vec<_>>();
            if completion.secondary_project_refs() != applied {
                return invalid(
                    "capture group completion receipt must enumerate every applied secondary reference",
                );
            }
            if self.required_reference_count() != self.applied_reference_count() {
                return invalid(
                    "capture group completion receipt requires every requested secondary reference",
                );
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SecondaryProjectAssociation {
    capture_id: CaptureId,
    capture_group_id: CaptureGroupId,
    primary_project_ref: ProjectRefId,
    secondary_project_ref: ProjectRefId,
    canonical_record_ref: CanonicalRecordRef,
    relation: String,
    observed_at: UtcTimestamp,
    capture_completed: bool,
}

impl SecondaryProjectAssociation {
    pub fn capture_id(&self) -> CaptureId {
        self.capture_id
    }

    pub fn capture_group_id(&self) -> CaptureGroupId {
        self.capture_group_id
    }

    pub fn primary_project_ref(&self) -> ProjectRefId {
        self.primary_project_ref
    }

    pub fn secondary_project_ref(&self) -> ProjectRefId {
        self.secondary_project_ref
    }

    pub fn canonical_record_ref(&self) -> &CanonicalRecordRef {
        &self.canonical_record_ref
    }

    pub fn relation(&self) -> &str {
        &self.relation
    }

    pub fn observed_at(&self) -> &UtcTimestamp {
        &self.observed_at
    }

    pub fn capture_completed(&self) -> bool {
        self.capture_completed
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CaptureProjection {
    journal_version: u64,
    capture_id: CaptureId,
    intent_digest: ControlPlaneDigest,
    event_count: u64,
    last_event_sequence: u64,
    #[serde(default)]
    last_event_digest: Nullable<ControlPlaneDigest>,
    recovery_state: CaptureRecoveryState,
    latest_resolution: ResolutionResult,
    #[serde(default)]
    project_binding_ready: Nullable<ProjectBindingReadyPayload>,
    requires_secondary_references: bool,
    #[serde(default)]
    capture_group: Nullable<CaptureGroupProjection>,
    #[serde(default)]
    delivery_started: Nullable<DeliveryStartedPayload>,
    #[serde(default)]
    primary_delivery: Nullable<DeliveryAppliedPayload>,
    #[serde(default)]
    delivery_failure: Nullable<DeliveryFailedPayload>,
}

impl CaptureProjection {
    fn from_authority(intent: &CaptureIntent, events: &[CaptureEvent]) -> Result<Self> {
        intent.validate()?;
        let intent_digest = ControlPlaneDigest::raw(&intent.canonical_json_bytes()?);
        let mut latest_resolution = intent.initial_resolution().clone();
        let mut project_binding_ready = None;
        let mut capture_group = intent
            .capture_group()
            .map(|group| {
                let mut members = group
                    .members()
                    .iter()
                    .map(CaptureGroupMemberProjection::from_intent)
                    .collect::<Result<Vec<_>>>()?;
                members.sort_by_key(CaptureGroupMemberProjection::project_ref_id);
                let projection = CaptureGroupProjection {
                    capture_id: intent.capture_id(),
                    capture_group_id: group.capture_group_id(),
                    resolved_primary_project_ref: Nullable::present(group.primary_project_ref()),
                    primary_locator_evidence_digest: Nullable::present(
                        group.primary_locator_evidence_digest().cloned(),
                    ),
                    canonical_record_ref: Nullable::present(None),
                    members,
                    unresolved_related_locators: latest_resolution.unmapped_locators().to_vec(),
                    completion_receipt: Nullable::present(None),
                };
                projection.validate()?;
                Ok(projection)
            })
            .transpose()?;
        let requires_secondary_references = capture_group
            .as_ref()
            .is_some_and(|group| group.required_reference_count() > 0);
        let mut delivery_started: Option<DeliveryStartedPayload> = None;
        let mut primary_delivery: Option<DeliveryAppliedPayload> = None;
        let mut delivery_failure: Option<DeliveryFailedPayload> = None;
        for event in events {
            match event.payload() {
                CaptureEventPayload::ResolutionRecorded(payload) => {
                    latest_resolution = payload.resolution().clone();
                    if let Some(group) = capture_group.as_ref()
                        && group.canonical_record_ref().is_some()
                        && (latest_resolution.status() != ResolutionStatus::Resolved
                            || latest_resolution.primary_project_ref()
                                != group.resolved_primary_project_ref())
                    {
                        return invalid(format!(
                            "resolution_recorded event {} cannot supersede a CaptureGroup canonical authority",
                            event.event_id()
                        ));
                    }
                    if let Some(group) = capture_group.as_ref()
                        && latest_resolution.status() == ResolutionStatus::Resolved
                        && group.resolved_primary_project_ref().is_some()
                        && latest_resolution.primary_project_ref()
                            != group.resolved_primary_project_ref()
                    {
                        return invalid(format!(
                            "resolution_recorded event {} conflicts with the CaptureGroup primary",
                            event.event_id()
                        ));
                    }
                    if let Some(group) = capture_group.as_mut() {
                        group.unresolved_related_locators =
                            latest_resolution.unmapped_locators().to_vec();
                    }
                    project_binding_ready = None;
                    let retains_delivery = delivery_started.as_ref().is_some_and(|started| {
                        latest_resolution.status() == ResolutionStatus::Resolved
                            && latest_resolution.primary_project_ref()
                                == Some(started.project_ref_id())
                    });
                    if !retains_delivery {
                        delivery_started = None;
                        primary_delivery = None;
                        delivery_failure = None;
                    }
                }
                CaptureEventPayload::ProjectBindingReady(payload) => {
                    if latest_resolution.status() != ResolutionStatus::Resolved
                        || latest_resolution.primary_project_ref() != Some(payload.project_ref_id())
                    {
                        return invalid(format!(
                            "project_binding_ready event {} does not match the latest resolved owner",
                            event.event_id()
                        ));
                    }
                    let retains_delivery = delivery_started
                        .as_ref()
                        .is_some_and(|started| payload.matches_delivery_target(started));
                    if capture_group
                        .as_ref()
                        .is_some_and(|group| group.canonical_record_ref().is_some())
                        && !retains_delivery
                    {
                        return invalid(format!(
                            "project_binding_ready event {} cannot retarget a CaptureGroup after canonical delivery",
                            event.event_id()
                        ));
                    }
                    if !retains_delivery {
                        delivery_started = None;
                        primary_delivery = None;
                        delivery_failure = None;
                    }
                    project_binding_ready = Some(payload.clone());
                }
                CaptureEventPayload::CaptureGroupResolved(payload) => {
                    let group = capture_group.as_mut().ok_or_else(|| {
                        WorkVcsError::ControlPlaneInvalid(format!(
                            "capture_group_resolved event {} has no CaptureGroup intent",
                            event.event_id()
                        ))
                    })?;
                    if group.capture_group_id() != payload.capture_group_id() {
                        return invalid(format!(
                            "capture_group_resolved event {} does not match the CaptureGroup identity",
                            event.event_id()
                        ));
                    }
                    if group.resolved_primary_project_ref().is_some() {
                        return invalid(format!(
                            "capture_group_resolved event {} attempts to replace an existing primary",
                            event.event_id()
                        ));
                    }
                    if latest_resolution.status() != ResolutionStatus::Resolved
                        || latest_resolution.primary_project_ref()
                            != Some(payload.primary_project_ref())
                    {
                        return invalid(format!(
                            "capture_group_resolved event {} does not match the latest resolved owner",
                            event.event_id()
                        ));
                    }
                    if group.primary_locator_evidence_digest()
                        != Some(payload.primary_locator_evidence_digest())
                    {
                        return invalid(format!(
                            "capture_group_resolved event {} does not match the admitted primary locator evidence",
                            event.event_id()
                        ));
                    }
                    let basis_digest = latest_resolution.primary_basis().and_then(|basis| {
                        if let super::resolver::ResolutionBasis::Locator { evidence, .. } = basis {
                            Some(evidence.evidence_digest())
                        } else {
                            None
                        }
                    });
                    if basis_digest != Some(payload.primary_locator_evidence_digest()) {
                        return invalid(format!(
                            "capture_group_resolved event {} is not backed by the latest primary locator evidence",
                            event.event_id()
                        ));
                    }
                    if group.member(payload.primary_project_ref()).is_some() {
                        return invalid(format!(
                            "capture_group_resolved event {} would replace an existing member role",
                            event.event_id()
                        ));
                    }
                    group.resolved_primary_project_ref =
                        Nullable::present(Some(payload.primary_project_ref()));
                    group
                        .members
                        .push(CaptureGroupMemberProjection::resolved_primary(payload)?);
                    group
                        .members
                        .sort_by_key(CaptureGroupMemberProjection::project_ref_id);
                    group.validate()?;
                }
                CaptureEventPayload::DeliveryStarted(payload) => {
                    let binding = project_binding_ready.as_ref().ok_or_else(|| {
                        WorkVcsError::ControlPlaneInvalid(format!(
                            "delivery_started event {} has no current project binding receipt",
                            event.event_id()
                        ))
                    })?;
                    if latest_resolution.status() != ResolutionStatus::Resolved
                        || latest_resolution.primary_project_ref() != Some(payload.project_ref_id())
                        || binding.project_ref_id() != payload.project_ref_id()
                        || binding.store_id() != payload.store_id()
                        || binding.workspace_id() != payload.workspace_id()
                        || binding.branch_id() != payload.branch_id()
                    {
                        return invalid(format!(
                            "delivery_started event {} does not match the current resolved binding",
                            event.event_id()
                        ));
                    }
                    if capture_group.as_ref().is_some_and(|group| {
                        group.resolved_primary_project_ref() != Some(payload.project_ref_id())
                    }) {
                        return invalid(format!(
                            "delivery_started event {} does not match the capture group primary",
                            event.event_id()
                        ));
                    }
                    if delivery_started.is_some() {
                        return invalid(format!(
                            "delivery_started event {} overlaps an unfinished primary delivery",
                            event.event_id()
                        ));
                    }
                    delivery_started = Some(payload.clone());
                    delivery_failure = None;
                }
                CaptureEventPayload::DeliveryApplied(payload) => {
                    let started = delivery_started.as_ref().ok_or_else(|| {
                        WorkVcsError::ControlPlaneInvalid(format!(
                            "delivery_applied event {} has no preceding delivery_started event",
                            event.event_id()
                        ))
                    })?;
                    if !payload.matches_started(started) {
                        return invalid(format!(
                            "delivery_applied event {} does not match its delivery_started target",
                            event.event_id()
                        ));
                    }
                    if payload.commit_id() == started.expected_head_commit_id() {
                        return invalid(format!(
                            "delivery_applied event {} did not advance the target commit",
                            event.event_id()
                        ));
                    }
                    if intent.capture_group().is_some() != payload.canonical_record_ref().is_some()
                    {
                        return invalid(format!(
                            "delivery_applied event {} canonical record reference does not match capture group presence",
                            event.event_id()
                        ));
                    }
                    if primary_delivery.is_some() {
                        return invalid(format!(
                            "delivery_applied event {} duplicates the canonical primary receipt",
                            event.event_id()
                        ));
                    }
                    if let Some(group) = capture_group.as_mut() {
                        let reference = payload.canonical_record_ref().ok_or_else(|| {
                            WorkVcsError::ControlPlaneInvalid(
                                "CaptureGroup primary delivery is missing canonical_record_ref"
                                    .to_owned(),
                            )
                        })?;
                        if group.resolved_primary_project_ref() != Some(reference.project_ref_id())
                        {
                            return invalid(format!(
                                "delivery_applied event {} canonical Record does not match the CaptureGroup primary",
                                event.event_id()
                            ));
                        }
                        if group.canonical_record_ref().is_some() {
                            return invalid(format!(
                                "delivery_applied event {} attempts a second CaptureGroup canonical authority",
                                event.event_id()
                            ));
                        }
                        group.canonical_record_ref = Nullable::present(Some(reference.clone()));
                    }
                    primary_delivery = Some(payload.clone());
                    delivery_failure = None;
                }
                CaptureEventPayload::DeliveryFailed(payload) => {
                    let started = delivery_started.as_ref().ok_or_else(|| {
                        WorkVcsError::ControlPlaneInvalid(format!(
                            "delivery_failed event {} has no preceding delivery_started event",
                            event.event_id()
                        ))
                    })?;
                    if primary_delivery.is_some() || !payload.matches_started(started) {
                        return invalid(format!(
                            "delivery_failed event {} does not match an unapplied primary delivery",
                            event.event_id()
                        ));
                    }
                    delivery_failure = Some(payload.clone());
                }
                CaptureEventPayload::ReferenceApplied(payload) => {
                    let group = capture_group.as_mut().ok_or_else(|| {
                        WorkVcsError::ControlPlaneInvalid(format!(
                            "reference_applied event {} has no CaptureGroup intent",
                            event.event_id()
                        ))
                    })?;
                    if group.capture_group_id() != payload.capture_group_id() {
                        return invalid(format!(
                            "reference_applied event {} does not match the CaptureGroup identity",
                            event.event_id()
                        ));
                    }
                    let canonical = primary_delivery
                        .as_ref()
                        .and_then(DeliveryAppliedPayload::canonical_record_ref)
                        .ok_or_else(|| {
                            WorkVcsError::ControlPlaneInvalid(format!(
                                "reference_applied event {} has no current canonical delivery receipt",
                                event.event_id()
                            ))
                        })?;
                    if payload.canonical_record_ref() != canonical {
                        return invalid(format!(
                            "reference_applied event {} is not pinned to the exact canonical Record receipt",
                            event.event_id()
                        ));
                    }
                    let member = group
                        .members
                        .iter_mut()
                        .find(|member| {
                            member.project_ref_id() == payload.secondary_project_ref()
                        })
                        .ok_or_else(|| {
                            WorkVcsError::ControlPlaneInvalid(format!(
                                "reference_applied event {} targets a ProjectRef outside the CaptureGroup",
                                event.event_id()
                            ))
                        })?;
                    if member.delivery_mode()
                        != super::model::CaptureGroupMemberDelivery::ImmutableReference
                        || member.relation() != payload.relation()
                    {
                        return invalid(format!(
                            "reference_applied event {} does not match the requested member delivery",
                            event.event_id()
                        ));
                    }
                    if member.reference().is_some() {
                        return invalid(format!(
                            "reference_applied event {} duplicates a secondary reference receipt",
                            event.event_id()
                        ));
                    }
                    member.reference =
                        Nullable::present(Some(ImmutableSecondaryReference::from_event(
                            payload,
                            event.occurred_at().clone(),
                        )?));
                }
                CaptureEventPayload::CaptureCompleted(payload) => {
                    let group = capture_group.as_mut().ok_or_else(|| {
                        WorkVcsError::ControlPlaneInvalid(format!(
                            "capture_completed event {} has no CaptureGroup intent",
                            event.event_id()
                        ))
                    })?;
                    if group.capture_group_id() != payload.capture_group_id()
                        || group.canonical_record_ref() != Some(payload.canonical_record_ref())
                    {
                        return invalid(format!(
                            "capture_completed event {} does not match the CaptureGroup canonical authority",
                            event.event_id()
                        ));
                    }
                    if group.required_reference_count() != group.applied_reference_count() {
                        return invalid(format!(
                            "capture_completed event {} precedes required secondary references",
                            event.event_id()
                        ));
                    }
                    let applied = group
                        .members
                        .iter()
                        .filter(|member| member.reference().is_some())
                        .map(CaptureGroupMemberProjection::project_ref_id)
                        .collect::<Vec<_>>();
                    if payload.secondary_project_refs() != applied {
                        return invalid(format!(
                            "capture_completed event {} does not enumerate the applied secondary references",
                            event.event_id()
                        ));
                    }
                    if group.completion_receipt().is_some() {
                        return invalid(format!(
                            "capture_completed event {} duplicates the completion receipt",
                            event.event_id()
                        ));
                    }
                    group.completion_receipt = Nullable::present(Some(payload.clone()));
                }
            }
        }
        let recovery_state = match latest_resolution.status() {
            ResolutionStatus::Unresolved | ResolutionStatus::Conflict => {
                CaptureRecoveryState::PendingResolution
            }
            ResolutionStatus::Unbound => CaptureRecoveryState::PendingProject,
            ResolutionStatus::Resolved => {
                if project_binding_ready.as_ref().is_some_and(|binding| {
                    Some(binding.project_ref_id()) == latest_resolution.primary_project_ref()
                }) {
                    if capture_group
                        .as_ref()
                        .is_some_and(|group| group.resolved_primary_project_ref().is_none())
                    {
                        CaptureRecoveryState::PendingProject
                    } else if primary_delivery.is_some() {
                        if capture_group.as_ref().is_some_and(|group| {
                            group.required_reference_count() != group.applied_reference_count()
                        }) {
                            CaptureRecoveryState::PendingReferences
                        } else {
                            CaptureRecoveryState::Completed
                        }
                    } else if delivery_failure.as_ref().is_some_and(|failure| {
                        failure.failure_code() == DeliveryFailureCode::LegacyManifestUpgradeRequired
                    }) {
                        CaptureRecoveryState::LegacyManifestUpgradeRequired
                    } else {
                        CaptureRecoveryState::PendingPrimary
                    }
                } else {
                    CaptureRecoveryState::PendingProject
                }
            }
        };
        let event_count = u64::try_from(events.len())
            .map_err(|_| WorkVcsError::ControlPlaneInvalid("too many capture events".to_owned()))?;
        let projection = Self {
            journal_version: JOURNAL_VERSION,
            capture_id: intent.capture_id(),
            intent_digest,
            event_count,
            last_event_sequence: events.last().map(CaptureEvent::sequence).unwrap_or(0),
            last_event_digest: Nullable::present(
                events.last().map(CaptureEvent::digest).transpose()?,
            ),
            recovery_state,
            latest_resolution,
            project_binding_ready: Nullable::present(project_binding_ready),
            requires_secondary_references,
            capture_group: Nullable::present(capture_group),
            delivery_started: Nullable::present(delivery_started),
            primary_delivery: Nullable::present(primary_delivery),
            delivery_failure: Nullable::present(delivery_failure),
        };
        projection.validate()?;
        Ok(projection)
    }

    pub fn from_json_bytes(input: &[u8]) -> Result<Self> {
        if input.len() > MAX_CAPTURE_PROJECTION_BYTES {
            return invalid(format!(
                "capture projection is {} bytes; maximum is {MAX_CAPTURE_PROJECTION_BYTES}",
                input.len()
            ));
        }
        parse_canonical_json(input).map_err(|error| {
            WorkVcsError::ControlPlaneInvalid(format!(
                "capture projection is not strict canonical-domain JSON: {error}"
            ))
        })?;
        let projection: Self = serde_json::from_slice(input).map_err(|error| {
            WorkVcsError::ControlPlaneInvalid(format!(
                "capture projection has an invalid shape: {error}"
            ))
        })?;
        projection.validate()?;
        Ok(projection)
    }

    pub fn validate(&self) -> Result<()> {
        if self.journal_version != JOURNAL_VERSION {
            return invalid(format!(
                "capture projection journal_version {} is not supported",
                self.journal_version
            ));
        }
        if !self.last_event_digest.is_present()
            || !self.project_binding_ready.is_present()
            || !self.capture_group.is_present()
            || !self.delivery_started.is_present()
            || !self.primary_delivery.is_present()
            || !self.delivery_failure.is_present()
        {
            return invalid(
                "capture projection nullable fields are required even when their values are null",
            );
        }
        if (self.event_count == 0)
            != (self.last_event_sequence == 0 && self.last_event_digest.as_ref().is_none())
        {
            return invalid("capture projection last-event fields do not match event_count");
        }
        if self.event_count != self.last_event_sequence {
            return invalid("capture projection event_count must equal last_event_sequence");
        }
        self.latest_resolution.validate()?;
        if self.requires_secondary_references
            != self
                .capture_group
                .as_ref()
                .is_some_and(|group| group.required_reference_count() > 0)
        {
            return invalid(
                "capture projection secondary-reference flag does not match CaptureGroup members",
            );
        }
        if let Some(group) = self.capture_group.as_ref() {
            group.validate()?;
            if group.capture_id() != self.capture_id
                || group.unresolved_related_locators() != self.latest_resolution.unmapped_locators()
            {
                return invalid(
                    "capture group projection must match its capture and latest unresolved locator evidence",
                );
            }
            if self.latest_resolution.status() == ResolutionStatus::Resolved
                && group.resolved_primary_project_ref().is_some()
                && group.resolved_primary_project_ref()
                    != self.latest_resolution.primary_project_ref()
            {
                return invalid(
                    "capture group projection primary conflicts with the latest resolved owner",
                );
            }
        }
        if let Some(binding) = self.project_binding_ready.as_ref() {
            binding.validate()?;
            if self.latest_resolution.status() != ResolutionStatus::Resolved
                || self.latest_resolution.primary_project_ref() != Some(binding.project_ref_id())
            {
                return invalid(
                    "capture projection binding must match its latest resolved ProjectRef",
                );
            }
        }
        if let Some(started) = self.delivery_started.as_ref() {
            started.validate()?;
            if self.latest_resolution.status() != ResolutionStatus::Resolved
                || self.latest_resolution.primary_project_ref() != Some(started.project_ref_id())
            {
                return invalid(
                    "capture projection retained delivery must match its latest resolved ProjectRef",
                );
            }
            if self
                .project_binding_ready
                .as_ref()
                .is_some_and(|binding| !binding.matches_delivery_target(started))
            {
                return invalid(
                    "capture projection delivery target must match its binding receipt",
                );
            }
        }
        if let Some(delivery) = self.primary_delivery.as_ref() {
            delivery.validate()?;
            let started = self.delivery_started.as_ref().ok_or_else(|| {
                WorkVcsError::ControlPlaneInvalid(
                    "capture projection primary receipt requires delivery_started".to_owned(),
                )
            })?;
            if !delivery.matches_started(started) {
                return invalid(
                    "capture projection primary receipt does not match delivery_started",
                );
            }
            if delivery.commit_id() == started.expected_head_commit_id() {
                return invalid(
                    "capture projection primary receipt must advance the target commit",
                );
            }
            match (self.capture_group.as_ref(), delivery.canonical_record_ref()) {
                (Some(group), Some(reference))
                    if group.canonical_record_ref() == Some(reference) => {}
                (None, None) => {}
                _ => {
                    return invalid(
                        "capture projection primary receipt and CaptureGroup canonical reference do not match",
                    );
                }
            }
        } else if self
            .capture_group
            .as_ref()
            .is_some_and(|group| group.canonical_record_ref().is_some())
        {
            return invalid(
                "capture group canonical reference requires a current primary delivery receipt",
            );
        }
        if let Some(failure) = self.delivery_failure.as_ref() {
            failure.validate()?;
            let started = self.delivery_started.as_ref().ok_or_else(|| {
                WorkVcsError::ControlPlaneInvalid(
                    "capture projection delivery failure requires delivery_started".to_owned(),
                )
            })?;
            if self.primary_delivery.as_ref().is_some() || !failure.matches_started(started) {
                return invalid(
                    "capture projection delivery failure must match an unapplied delivery_started",
                );
            }
        }
        let expected_state = match self.latest_resolution.status() {
            ResolutionStatus::Unresolved | ResolutionStatus::Conflict => {
                CaptureRecoveryState::PendingResolution
            }
            ResolutionStatus::Unbound => CaptureRecoveryState::PendingProject,
            ResolutionStatus::Resolved if self.project_binding_ready.as_ref().is_some() => {
                if self
                    .capture_group
                    .as_ref()
                    .is_some_and(|group| group.resolved_primary_project_ref().is_none())
                {
                    CaptureRecoveryState::PendingProject
                } else if self.primary_delivery.as_ref().is_some() {
                    if self.capture_group.as_ref().is_some_and(|group| {
                        group.required_reference_count() != group.applied_reference_count()
                    }) {
                        CaptureRecoveryState::PendingReferences
                    } else {
                        CaptureRecoveryState::Completed
                    }
                } else if self.delivery_failure.as_ref().is_some_and(|failure| {
                    failure.failure_code() == DeliveryFailureCode::LegacyManifestUpgradeRequired
                }) {
                    CaptureRecoveryState::LegacyManifestUpgradeRequired
                } else {
                    CaptureRecoveryState::PendingPrimary
                }
            }
            ResolutionStatus::Resolved => CaptureRecoveryState::PendingProject,
        };
        if self.recovery_state != expected_state {
            return invalid(format!(
                "capture projection state {} does not match authority state {}",
                self.recovery_state.as_str(),
                expected_state.as_str()
            ));
        }
        let canonical_size = canonicalize_serializable(self, "capture projection")?.len();
        if canonical_size > MAX_CAPTURE_PROJECTION_BYTES {
            return invalid(format!(
                "canonical capture projection is {canonical_size} bytes; maximum is {MAX_CAPTURE_PROJECTION_BYTES}"
            ));
        }
        Ok(())
    }

    pub fn capture_id(&self) -> CaptureId {
        self.capture_id
    }

    pub fn intent_digest(&self) -> &ControlPlaneDigest {
        &self.intent_digest
    }

    pub fn event_count(&self) -> u64 {
        self.event_count
    }

    pub fn last_event_sequence(&self) -> u64 {
        self.last_event_sequence
    }

    pub fn last_event_digest(&self) -> Option<&ControlPlaneDigest> {
        self.last_event_digest.as_ref()
    }

    pub fn recovery_state(&self) -> CaptureRecoveryState {
        self.recovery_state
    }

    pub fn latest_resolution(&self) -> &ResolutionResult {
        &self.latest_resolution
    }

    pub fn project_binding_ready(&self) -> Option<&ProjectBindingReadyPayload> {
        self.project_binding_ready.as_ref()
    }

    pub fn requires_secondary_references(&self) -> bool {
        self.requires_secondary_references
    }

    pub fn capture_group(&self) -> Option<&CaptureGroupProjection> {
        self.capture_group.as_ref()
    }

    pub fn delivery_started(&self) -> Option<&DeliveryStartedPayload> {
        self.delivery_started.as_ref()
    }

    pub fn primary_delivery(&self) -> Option<&DeliveryAppliedPayload> {
        self.primary_delivery.as_ref()
    }

    pub fn delivery_failure(&self) -> Option<&DeliveryFailedPayload> {
        self.delivery_failure.as_ref()
    }

    pub fn canonical_json_bytes(&self) -> Result<Vec<u8>> {
        self.validate()?;
        canonicalize_serializable(self, "capture projection")
    }

    pub fn stored_json_bytes(&self) -> Result<Vec<u8>> {
        let mut bytes = self.canonical_json_bytes()?;
        bytes.push(b'\n');
        Ok(bytes)
    }

    pub fn digest(&self) -> Result<ControlPlaneDigest> {
        Ok(ControlPlaneDigest::raw(&self.canonical_json_bytes()?))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StoredProjectionState {
    Absent,
    Current,
    Stale,
    Invalid,
}

impl StoredProjectionState {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Absent => "absent",
            Self::Current => "current",
            Self::Stale => "stale",
            Self::Invalid => "invalid",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CaptureProjectionInspection {
    projection: CaptureProjection,
    stored_state: StoredProjectionState,
    stored_issue: Option<String>,
}

impl CaptureProjectionInspection {
    pub fn projection(&self) -> &CaptureProjection {
        &self.projection
    }

    pub fn stored_state(&self) -> StoredProjectionState {
        self.stored_state
    }

    pub fn stored_issue(&self) -> Option<&str> {
        self.stored_issue.as_deref()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CaptureEventAppendOutcome {
    Created,
    Reused,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CaptureEventAppendResult {
    outcome: CaptureEventAppendOutcome,
    event: CaptureEvent,
    projection: CaptureProjection,
    event_path: PathBuf,
}

impl CaptureEventAppendResult {
    pub fn outcome(&self) -> CaptureEventAppendOutcome {
        self.outcome
    }

    pub fn event(&self) -> &CaptureEvent {
        &self.event
    }

    pub fn projection(&self) -> &CaptureProjection {
        &self.projection
    }

    pub fn event_path(&self) -> &Path {
        &self.event_path
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CaptureProjectionWriteOutcome {
    Created,
    Replaced,
    Reused,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CaptureProjectionWriteResult {
    outcome: CaptureProjectionWriteOutcome,
    projection: CaptureProjection,
    projection_path: PathBuf,
}

impl CaptureProjectionWriteResult {
    pub fn outcome(&self) -> CaptureProjectionWriteOutcome {
        self.outcome
    }

    pub fn projection(&self) -> &CaptureProjection {
        &self.projection
    }

    pub fn projection_path(&self) -> &Path {
        &self.projection_path
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CaptureIntent {
    journal_version: u64,
    capture_id: CaptureId,
    idempotency_key: NonEmptyString,
    created_at: UtcTimestamp,
    value_reason: NonEmptyString,
    payload_kind: CapturePayloadKind,
    semantic_payload: Value,
    payload_digest: ControlPlaneDigest,
    resolution_context: ResolutionContext,
    initial_resolution: ResolutionResult,
    #[serde(default)]
    capture_group: Nullable<CaptureGroupIntent>,
}

impl CaptureIntent {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        capture_id: CaptureId,
        idempotency_key: impl Into<String>,
        created_at: UtcTimestamp,
        value_reason: impl Into<String>,
        payload_kind: CapturePayloadKind,
        semantic_payload: Value,
        resolution_context: ResolutionContext,
        initial_resolution: ResolutionResult,
        capture_group: Option<CaptureGroupIntent>,
    ) -> Result<Self> {
        let payload_digest =
            ControlPlaneDigest::raw(&canonical_semantic_payload(&semantic_payload)?);
        let intent = Self {
            journal_version: JOURNAL_VERSION,
            capture_id,
            idempotency_key: NonEmptyString::new(
                idempotency_key,
                "capture intent idempotency_key",
            )?,
            created_at,
            value_reason: NonEmptyString::new(value_reason, "capture intent value_reason")?,
            payload_kind,
            semantic_payload,
            payload_digest,
            resolution_context,
            initial_resolution,
            capture_group: Nullable::present(capture_group),
        };
        intent.validate()?;
        Ok(intent)
    }

    pub fn from_json_bytes(input: &[u8]) -> Result<Self> {
        if input.len() > MAX_CAPTURE_INTENT_BYTES {
            return invalid(format!(
                "capture intent is {} bytes; maximum is {MAX_CAPTURE_INTENT_BYTES}",
                input.len()
            ));
        }
        parse_canonical_json(input).map_err(|error| {
            WorkVcsError::ControlPlaneInvalid(format!(
                "capture intent is not strict canonical-domain JSON: {error}"
            ))
        })?;
        let intent: Self = serde_json::from_slice(input).map_err(|error| {
            WorkVcsError::ControlPlaneInvalid(format!(
                "capture intent has an invalid shape: {error}"
            ))
        })?;
        intent.validate()?;
        Ok(intent)
    }

    pub fn validate(&self) -> Result<()> {
        if self.journal_version != JOURNAL_VERSION {
            return invalid(format!(
                "capture intent journal_version {} is not supported",
                self.journal_version
            ));
        }
        if !self.capture_group.is_present() {
            return invalid("capture intent capture_group is required even when null");
        }
        if self.idempotency_key.as_str().len() > MAX_IDEMPOTENCY_KEY_BYTES {
            return invalid(format!(
                "capture intent idempotency_key exceeds {MAX_IDEMPOTENCY_KEY_BYTES} bytes"
            ));
        }
        if self.value_reason.as_str().len() > MAX_VALUE_REASON_BYTES {
            return invalid(format!(
                "capture intent value_reason exceeds {MAX_VALUE_REASON_BYTES} bytes"
            ));
        }
        let payload_bytes = canonical_semantic_payload(&self.semantic_payload)?;
        if payload_bytes.len() > MAX_SEMANTIC_PAYLOAD_BYTES {
            return invalid(format!(
                "semantic payload is {} bytes; maximum is {MAX_SEMANTIC_PAYLOAD_BYTES}",
                payload_bytes.len()
            ));
        }
        if contains_secret_bearing_field(&self.semantic_payload) {
            return invalid(
                "semantic payload contains a known secret-bearing field; redact it before admission",
            );
        }
        let actual_digest = ControlPlaneDigest::raw(&payload_bytes);
        if actual_digest != self.payload_digest {
            return invalid(format!(
                "semantic payload digest {} does not match declared {}",
                actual_digest, self.payload_digest
            ));
        }
        self.initial_resolution.validate()?;
        if let Some(group) = self.capture_group.as_ref() {
            group.validate()?;
            match (
                self.initial_resolution.status(),
                self.initial_resolution.primary_project_ref(),
                group.primary_project_ref(),
            ) {
                (ResolutionStatus::Resolved, Some(resolved), Some(primary))
                    if resolved == primary => {}
                (ResolutionStatus::Resolved, _, _) => {
                    return invalid("resolved capture group primary must match initial resolution");
                }
                (_, _, Some(_)) => {
                    return invalid(
                        "capture group must not name a primary ProjectRef when initial resolution is not resolved",
                    );
                }
                _ => {}
            }
            if let Some(primary_evidence_digest) = group.primary_locator_evidence_digest()
                && !self
                    .initial_resolution
                    .unmapped_locators()
                    .iter()
                    .any(|evidence| evidence.evidence_digest() == primary_evidence_digest)
            {
                return invalid(
                    "unresolved capture group primary evidence digest must be retained in initial_resolution.unmapped_locators",
                );
            }
        }
        let canonical_size = canonicalize_serializable(self, "capture intent")?.len();
        if canonical_size > MAX_CAPTURE_INTENT_BYTES {
            return invalid(format!(
                "canonical capture intent is {canonical_size} bytes; maximum is {MAX_CAPTURE_INTENT_BYTES}"
            ));
        }
        Ok(())
    }

    pub fn canonical_json_bytes(&self) -> Result<Vec<u8>> {
        self.validate()?;
        canonicalize_serializable(self, "capture intent")
    }

    pub fn stored_json_bytes(&self) -> Result<Vec<u8>> {
        let mut bytes = self.canonical_json_bytes()?;
        bytes.push(b'\n');
        Ok(bytes)
    }

    pub fn capture_id(&self) -> CaptureId {
        self.capture_id
    }

    pub fn idempotency_key(&self) -> &str {
        self.idempotency_key.as_str()
    }

    pub fn payload_digest(&self) -> &ControlPlaneDigest {
        &self.payload_digest
    }

    pub fn payload_kind(&self) -> CapturePayloadKind {
        self.payload_kind
    }

    pub fn semantic_payload(&self) -> &Value {
        &self.semantic_payload
    }

    pub fn created_at(&self) -> &UtcTimestamp {
        &self.created_at
    }

    pub fn value_reason(&self) -> &str {
        self.value_reason.as_str()
    }

    pub fn resolution_context(&self) -> &ResolutionContext {
        &self.resolution_context
    }

    pub fn initial_resolution(&self) -> &ResolutionResult {
        &self.initial_resolution
    }

    pub fn capture_group(&self) -> Option<&CaptureGroupIntent> {
        self.capture_group.as_ref()
    }
}

fn canonical_semantic_payload(payload: &Value) -> Result<Vec<u8>> {
    let raw = serde_json::to_vec(payload).map_err(|error| {
        WorkVcsError::ControlPlaneInvalid(format!("cannot serialize semantic payload: {error}"))
    })?;
    let canonical = parse_canonical_json(&raw).map_err(|error| {
        WorkVcsError::ControlPlaneInvalid(format!("semantic payload is invalid: {error}"))
    })?;
    if !matches!(canonical, CanonicalValue::Object(_)) {
        return invalid("semantic payload must be a JSON object");
    }
    canonical_bytes(&canonical).map_err(|error| {
        WorkVcsError::ControlPlaneInvalid(format!(
            "cannot encode canonical semantic payload: {error}"
        ))
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CaptureAdmissionOutcome {
    Created,
    Reused,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CaptureAdmissionResult {
    outcome: CaptureAdmissionOutcome,
    capture_id: CaptureId,
    payload_digest: ControlPlaneDigest,
    intent_path: PathBuf,
}

impl CaptureAdmissionResult {
    pub fn outcome(&self) -> CaptureAdmissionOutcome {
        self.outcome
    }

    pub fn capture_id(&self) -> CaptureId {
        self.capture_id
    }

    pub fn payload_digest(&self) -> &ControlPlaneDigest {
        &self.payload_digest
    }

    pub fn intent_path(&self) -> &Path {
        &self.intent_path
    }
}

#[derive(Clone, Debug)]
pub struct CaptureJournal {
    root: PathBuf,
    quiescence_lock_path: PathBuf,
    registry_path: Option<PathBuf>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProjectRegistryJournalAlias {
    StandardRegistryHome,
    RegistrySidecar,
}

impl CaptureJournal {
    pub fn for_standalone_root(root: impl Into<PathBuf>) -> Result<Self> {
        let root = root.into();
        if !root.is_absolute() {
            return invalid("capture journal root must be absolute");
        }
        let quiescence_lock_path = root.join("locks").join("intent-admission.lock");
        Ok(Self {
            root,
            quiescence_lock_path,
            registry_path: None,
        })
    }

    fn with_quiescence_lock(
        root: impl Into<PathBuf>,
        quiescence_lock_path: impl Into<PathBuf>,
        registry_path: PathBuf,
    ) -> Result<Self> {
        let root = root.into();
        let quiescence_lock_path = quiescence_lock_path.into();
        if !root.is_absolute() {
            return invalid("capture journal root must be absolute");
        }
        if !quiescence_lock_path.is_absolute() {
            return invalid("capture journal quiescence lock path must be absolute");
        }
        Ok(Self {
            root,
            quiescence_lock_path,
            registry_path: Some(registry_path),
        })
    }

    pub fn for_project_registry(
        canonical_registry_path: &Path,
        alias: ProjectRegistryJournalAlias,
    ) -> Result<Self> {
        let quiescence_lock_path =
            project_registry_journal_quiescence_lock_path(canonical_registry_path)?;
        let root = match alias {
            ProjectRegistryJournalAlias::StandardRegistryHome => {
                if canonical_registry_path.file_name()
                    != Some(std::ffi::OsStr::new(STANDARD_PROJECT_REGISTRY_FILE))
                {
                    return invalid(format!(
                        "standard-home capture journal alias requires registry file name {STANDARD_PROJECT_REGISTRY_FILE}"
                    ));
                }
                canonical_registry_path
                    .parent()
                    .expect("canonical registry path has parent")
                    .join("capture-journal")
                    .join("v1")
            }
            ProjectRegistryJournalAlias::RegistrySidecar => {
                let mut sidecar = OsString::from(canonical_registry_path.as_os_str());
                sidecar.push(".d");
                PathBuf::from(sidecar).join("capture-journal").join("v1")
            }
        };
        Self::with_quiescence_lock(
            root,
            quiescence_lock_path,
            canonical_registry_path.to_path_buf(),
        )
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn quiescence_lock_path(&self) -> &Path {
        &self.quiescence_lock_path
    }

    pub fn admit_json(&self, input: &[u8]) -> Result<CaptureAdmissionResult> {
        let intent = CaptureIntent::from_json_bytes(input)?;
        self.admit(&intent)
    }

    pub fn admit(&self, intent: &CaptureIntent) -> Result<CaptureAdmissionResult> {
        if self.registry_path.is_some() {
            return invalid(
                "registry-coupled capture journal requires exact post-lock registry eligibility validation",
            );
        }
        self.admit_after_lock_check(intent, || Ok(()))
    }

    pub fn admit_for_project_registry(
        &self,
        intent: &CaptureIntent,
        expected_registry_revision: u64,
        expected_registry_digest: &ControlPlaneDigest,
    ) -> Result<CaptureAdmissionResult> {
        self.admit_for_project_registry_with_check(
            intent,
            expected_registry_revision,
            expected_registry_digest,
            |_| Ok(()),
        )
    }

    pub fn admit_for_project_registry_v1(
        &self,
        intent: &CaptureIntent,
        expected_registry_digest: &ControlPlaneDigest,
    ) -> Result<CaptureAdmissionResult> {
        self.admit_for_project_registry_v1_with_check(intent, expected_registry_digest, |_| Ok(()))
    }

    pub fn admit_for_project_registry_v1_with_check<F>(
        &self,
        intent: &CaptureIntent,
        expected_registry_digest: &ControlPlaneDigest,
        additional_check: F,
    ) -> Result<CaptureAdmissionResult>
    where
        F: FnOnce(&ProjectRegistryV1) -> Result<()>,
    {
        let registry_path = self.registry_path.as_deref().ok_or_else(|| {
            WorkVcsError::ControlPlaneInvalid(
                "standalone capture journal cannot perform registry-v1 routed admission".to_owned(),
            )
        })?;
        self.admit_after_lock_check(intent, || {
            let registry =
                Self::read_expected_project_registry_v1(registry_path, expected_registry_digest)?;
            additional_check(&registry)?;
            Self::read_expected_project_registry_v1(registry_path, expected_registry_digest)?;
            Ok(())
        })
    }

    fn read_expected_project_registry_v1(
        registry_path: &Path,
        expected_registry_digest: &ControlPlaneDigest,
    ) -> Result<ProjectRegistryV1> {
        let bytes = fs::read(registry_path).map_err(|error| {
            not_persisted(format!(
                "cannot read registry v1 {} while journal quiescence is held: {error}",
                registry_path.display()
            ))
        })?;
        let registry = ProjectRegistryV1::from_json_bytes(&bytes).map_err(|error| {
            WorkVcsError::ControlPlaneInvalid(format!(
                "project registry {} is not eligible for v1 journal admission while journal quiescence is held: {error}",
                registry_path.display()
            ))
        })?;
        let actual_digest = registry.source_digest()?;
        if &actual_digest != expected_registry_digest {
            return Err(WorkVcsError::ControlPlaneInvalid(format!(
                "registry v1 changed before journal admission: expected digest {expected_registry_digest}, found {actual_digest}"
            )));
        }
        Ok(registry)
    }

    pub fn admit_for_project_registry_with_check<F>(
        &self,
        intent: &CaptureIntent,
        expected_registry_revision: u64,
        expected_registry_digest: &ControlPlaneDigest,
        additional_check: F,
    ) -> Result<CaptureAdmissionResult>
    where
        F: FnOnce(&ProjectRegistryV2) -> Result<()>,
    {
        let registry_path = self.registry_path.as_deref().ok_or_else(|| {
            WorkVcsError::ControlPlaneInvalid(
                "standalone capture journal cannot perform ProjectRef registry admission"
                    .to_owned(),
            )
        })?;
        self.admit_after_lock_check(intent, || {
            let registry = Self::read_expected_project_registry(
                registry_path,
                expected_registry_revision,
                expected_registry_digest,
            )?;
            additional_check(&registry)?;
            Self::read_expected_project_registry(
                registry_path,
                expected_registry_revision,
                expected_registry_digest,
            )?;
            Ok(())
        })
    }

    fn read_expected_project_registry(
        registry_path: &Path,
        expected_registry_revision: u64,
        expected_registry_digest: &ControlPlaneDigest,
    ) -> Result<ProjectRegistryV2> {
        let bytes = fs::read(registry_path).map_err(|error| {
            not_persisted(format!(
                "cannot read ProjectRef registry {} while journal quiescence is held: {error}",
                registry_path.display()
            ))
        })?;
        let registry = ProjectRegistryV2::from_json_bytes(&bytes).map_err(|error| {
            WorkVcsError::ControlPlaneInvalid(format!(
                "ProjectRef registry {} is not eligible for v2 journal admission while journal quiescence is held: {error}",
                registry_path.display()
            ))
        })?;
        let actual_digest = registry.digest()?;
        if registry.revision() != expected_registry_revision
            || &actual_digest != expected_registry_digest
        {
            return Err(WorkVcsError::ControlPlaneInvalid(format!(
                "ProjectRef registry changed before journal admission: expected revision {expected_registry_revision} digest {expected_registry_digest}, found revision {} digest {actual_digest}",
                registry.revision()
            )));
        }
        Ok(registry)
    }

    fn admit_after_lock_check<F>(
        &self,
        intent: &CaptureIntent,
        check: F,
    ) -> Result<CaptureAdmissionResult>
    where
        F: FnOnce() -> Result<()>,
    {
        intent.validate()?;
        self.ensure_lock_parent()?;
        let _lock = JournalQuiescenceLock::acquire(&self.quiescence_lock_path)?;
        check()?;
        self.ensure_layout()?;

        if let Some((existing, path)) = self.find_idempotency_key(intent.idempotency_key())? {
            if existing.payload_digest() == intent.payload_digest()
                && existing.capture_group() == intent.capture_group()
            {
                return Ok(CaptureAdmissionResult {
                    outcome: CaptureAdmissionOutcome::Reused,
                    capture_id: existing.capture_id(),
                    payload_digest: existing.payload_digest().clone(),
                    intent_path: path,
                });
            }
            return Err(WorkVcsError::CaptureIdempotencyConflict(format!(
                "idempotency key {:?} already belongs to capture {} with payload {} and its admitted CaptureGroup",
                intent.idempotency_key(),
                existing.capture_id(),
                existing.payload_digest()
            )));
        }

        let target = self.intent_path(intent.capture_id());
        if target.exists() {
            let existing = self.load_path(&target)?;
            if existing.idempotency_key() == intent.idempotency_key()
                && existing.payload_digest() == intent.payload_digest()
                && existing.capture_group() == intent.capture_group()
            {
                return Ok(CaptureAdmissionResult {
                    outcome: CaptureAdmissionOutcome::Reused,
                    capture_id: existing.capture_id(),
                    payload_digest: existing.payload_digest().clone(),
                    intent_path: target,
                });
            }
            return Err(WorkVcsError::CaptureIdempotencyConflict(format!(
                "capture id {} already exists with different admitted content",
                intent.capture_id()
            )));
        }

        let stored = intent.stored_json_bytes()?;
        let temp = self.unique_temp_path(intent.capture_id())?;
        let write_result = (|| -> Result<()> {
            let mut file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&temp)
                .map_err(|error| {
                    not_persisted(format!(
                        "cannot create capture intent temp file {}: {error}",
                        temp.display()
                    ))
                })?;
            file.write_all(&stored).map_err(|error| {
                not_persisted(format!(
                    "cannot write capture intent temp file {}: {error}",
                    temp.display()
                ))
            })?;
            file.sync_all().map_err(|error| {
                not_persisted(format!(
                    "cannot sync capture intent temp file {}: {error}",
                    temp.display()
                ))
            })?;
            drop(file);
            fs::hard_link(&temp, &target).map_err(|error| {
                if error.kind() == std::io::ErrorKind::AlreadyExists {
                    WorkVcsError::CaptureIdempotencyConflict(format!(
                        "capture id {} was installed concurrently; existing immutable intent was not replaced",
                        intent.capture_id()
                    ))
                } else {
                    not_persisted(format!(
                        "cannot atomically install capture intent {} from {}: {error}",
                        target.display(),
                        temp.display()
                    ))
                }
            })?;
            fs::remove_file(&temp).map_err(|error| {
                not_persisted(format!(
                    "capture intent {} was installed but temp-link cleanup {} failed: {error}",
                    target.display(),
                    temp.display()
                ))
            })?;
            sync_directory(&self.intents_dir()).map_err(|error| {
                not_persisted(format!(
                    "cannot sync capture intents directory {}: {error}",
                    self.intents_dir().display()
                ))
            })?;
            Ok(())
        })();
        if write_result.is_err() {
            let _ = fs::remove_file(&temp);
        }
        write_result?;

        Ok(CaptureAdmissionResult {
            outcome: CaptureAdmissionOutcome::Created,
            capture_id: intent.capture_id(),
            payload_digest: intent.payload_digest().clone(),
            intent_path: target,
        })
    }

    pub fn load(&self, capture_id: CaptureId) -> Result<CaptureIntent> {
        self.load_path(&self.intent_path(capture_id))
    }

    pub fn recall_secondary_project(
        &self,
        project_ref_id: ProjectRefId,
    ) -> Result<Vec<SecondaryProjectAssociation>> {
        let mut associations = Vec::new();
        for path in self.intent_paths_readonly()? {
            let intent = self.load_path(&path)?;
            let file_capture_id = path
                .file_stem()
                .and_then(|value| value.to_str())
                .ok_or_else(|| {
                    not_persisted(format!(
                        "capture intent path {} has no canonical UTF-8 stem",
                        path.display()
                    ))
                })
                .and_then(|value| {
                    CaptureId::parse_canonical(value).map_err(|error| {
                        not_persisted(format!(
                            "capture intent path {} has invalid capture ID: {error}",
                            path.display()
                        ))
                    })
                })?;
            if file_capture_id != intent.capture_id() {
                return Err(not_persisted(format!(
                    "capture intent {} identity does not match its file name",
                    path.display()
                )));
            }
            let events = self.load_events(intent.capture_id())?;
            let projection = CaptureProjection::from_authority(&intent, &events)?;
            let Some(group) = projection.capture_group() else {
                continue;
            };
            let Some(member) = group.member(project_ref_id) else {
                continue;
            };
            let Some(reference) = member.reference() else {
                continue;
            };
            let primary_project_ref = group.resolved_primary_project_ref().ok_or_else(|| {
                WorkVcsError::ControlPlaneInvalid(format!(
                    "capture {} has a secondary reference without a resolved CaptureGroup primary",
                    intent.capture_id()
                ))
            })?;
            associations.push(SecondaryProjectAssociation {
                capture_id: intent.capture_id(),
                capture_group_id: group.capture_group_id(),
                primary_project_ref,
                secondary_project_ref: project_ref_id,
                canonical_record_ref: reference.canonical_record_ref().clone(),
                relation: reference.relation().to_owned(),
                observed_at: reference.observed_at().clone(),
                capture_completed: group.completion_receipt().is_some(),
            });
        }
        associations
            .sort_by_key(|association| (association.capture_group_id, association.capture_id));
        Ok(associations)
    }

    pub fn inspect_projection(&self, capture_id: CaptureId) -> Result<CaptureProjectionInspection> {
        let intent = self.load(capture_id)?;
        let events = self.load_events(capture_id)?;
        let projection = CaptureProjection::from_authority(&intent, &events)?;
        let projection_path = self.projection_path(capture_id);
        let (stored_state, stored_issue) = match fs::symlink_metadata(&projection_path) {
            Ok(metadata) => {
                if metadata.file_type().is_symlink() || !metadata.is_file() {
                    return Err(not_persisted(format!(
                        "capture projection {} must be a regular non-symlink file",
                        projection_path.display()
                    )));
                }
                let stored = fs::read(&projection_path).map_err(|error| {
                    not_persisted(format!(
                        "cannot read capture projection {}: {error}",
                        projection_path.display()
                    ))
                })?;
                match CaptureProjection::from_json_bytes(&stored) {
                    Ok(_) if stored == projection.stored_json_bytes()? => {
                        (StoredProjectionState::Current, None)
                    }
                    Ok(_) => (
                        StoredProjectionState::Stale,
                        Some(
                            "stored projection does not match immutable intent and events"
                                .to_owned(),
                        ),
                    ),
                    Err(error) => (StoredProjectionState::Invalid, Some(error.to_string())),
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                (StoredProjectionState::Absent, None)
            }
            Err(error) => {
                return Err(not_persisted(format!(
                    "cannot inspect capture projection {}: {error}",
                    projection_path.display()
                )));
            }
        };
        Ok(CaptureProjectionInspection {
            projection,
            stored_state,
            stored_issue,
        })
    }

    pub fn append_event_authority_only(
        &self,
        capture_id: CaptureId,
        occurred_at: UtcTimestamp,
        payload: CaptureEventPayload,
    ) -> Result<CaptureEventAppendResult> {
        payload.validate()?;
        self.ensure_layout()?;
        let _lock = JournalQuiescenceLock::acquire(&self.capture_lock_path(capture_id))?;
        let intent = self.load(capture_id)?;
        let mut events = self.load_events(capture_id)?;
        if let Some(existing) = events
            .iter()
            .rev()
            .find(|event| event.payload().kind_name() == payload.kind_name())
            .filter(|event| event.payload() == &payload)
            .cloned()
        {
            let projection = CaptureProjection::from_authority(&intent, &events)?;
            return Ok(CaptureEventAppendResult {
                outcome: CaptureEventAppendOutcome::Reused,
                event_path: self.event_path(&existing),
                event: existing,
                projection,
            });
        }

        let sequence = u64::try_from(events.len())
            .map_err(|_| WorkVcsError::ControlPlaneInvalid("too many capture events".to_owned()))?
            .checked_add(1)
            .ok_or_else(|| {
                WorkVcsError::ControlPlaneInvalid("capture event sequence overflow".to_owned())
            })?;
        let previous_event_digest = events.last().map(CaptureEvent::digest).transpose()?;
        let event = CaptureEvent::new(
            capture_id,
            sequence,
            EventId::new_v7(),
            occurred_at,
            previous_event_digest,
            payload,
        )?;
        events.push(event.clone());
        let projection = CaptureProjection::from_authority(&intent, &events)?;
        let event_dir = self.capture_events_dir(capture_id);
        ensure_regular_directory(&event_dir, "capture event directory", true)?;
        let target = self.event_path(&event);
        let temp = self.unique_event_temp_path(&event)?;
        let stored = event.stored_json_bytes()?;
        let mut event_installed = false;
        let write_result = (|| -> Result<()> {
            let mut file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&temp)
                .map_err(|error| {
                    not_persisted(format!(
                        "cannot create capture event temp file {}: {error}",
                        temp.display()
                    ))
                })?;
            file.write_all(&stored).map_err(|error| {
                not_persisted(format!(
                    "cannot write capture event temp file {}: {error}",
                    temp.display()
                ))
            })?;
            file.sync_all().map_err(|error| {
                not_persisted(format!(
                    "cannot sync capture event temp file {}: {error}",
                    temp.display()
                ))
            })?;
            drop(file);
            fs::hard_link(&temp, &target).map_err(|error| {
                if error.kind() == std::io::ErrorKind::AlreadyExists {
                    WorkVcsError::CaptureIdempotencyConflict(format!(
                        "capture event target {} already exists; immutable event was not replaced",
                        target.display()
                    ))
                } else {
                    not_persisted(format!(
                        "cannot atomically install capture event {} from {}: {error}",
                        target.display(),
                        temp.display()
                    ))
                }
            })?;
            event_installed = true;
            fs::remove_file(&temp).map_err(|error| {
                not_persisted(format!(
                    "capture event {} was installed but temp-link cleanup {} failed: {error}",
                    target.display(),
                    temp.display()
                ))
            })?;
            sync_directory(&event_dir).map_err(|error| {
                not_persisted(format!(
                    "cannot sync capture event directory {}: {error}",
                    event_dir.display()
                ))
            })?;
            Ok(())
        })();
        if write_result.is_err() {
            let _ = fs::remove_file(&temp);
        }
        if let Err(error) = write_result {
            if event_installed {
                return Err(WorkVcsError::CaptureRecoveryInstallIndeterminate(format!(
                    "capture event {} was installed or may already be durable; inspect recovery status before retry; cause: {error}",
                    target.display()
                )));
            }
            return Err(error);
        }
        Ok(CaptureEventAppendResult {
            outcome: CaptureEventAppendOutcome::Created,
            event,
            projection,
            event_path: target,
        })
    }

    pub fn append_event(
        &self,
        capture_id: CaptureId,
        occurred_at: UtcTimestamp,
        payload: CaptureEventPayload,
    ) -> Result<CaptureEventAppendResult> {
        let result = self.append_event_authority_only(capture_id, occurred_at, payload)?;
        self.rebuild_projection(capture_id)?;
        Ok(result)
    }

    pub fn rebuild_projection(
        &self,
        capture_id: CaptureId,
    ) -> Result<CaptureProjectionWriteResult> {
        self.ensure_layout()?;
        let _lock = JournalQuiescenceLock::acquire(&self.capture_lock_path(capture_id))?;
        let intent = self.load(capture_id)?;
        let events = self.load_events(capture_id)?;
        let projection = CaptureProjection::from_authority(&intent, &events)?;
        self.write_projection_locked(projection)
    }

    pub fn intent_path(&self, capture_id: CaptureId) -> PathBuf {
        self.intents_dir().join(format!("{capture_id}.json"))
    }

    pub fn projection_path(&self, capture_id: CaptureId) -> PathBuf {
        self.projections_dir().join(format!("{capture_id}.json"))
    }

    fn write_projection_locked(
        &self,
        projection: CaptureProjection,
    ) -> Result<CaptureProjectionWriteResult> {
        let target = self.projection_path(projection.capture_id());
        let stored = projection.stored_json_bytes()?;
        let outcome = match fs::symlink_metadata(&target) {
            Ok(metadata) => {
                if metadata.file_type().is_symlink() || !metadata.is_file() {
                    return Err(not_persisted(format!(
                        "capture projection {} must be a regular non-symlink file",
                        target.display()
                    )));
                }
                let existing = fs::read(&target).map_err(|error| {
                    not_persisted(format!(
                        "cannot read capture projection {}: {error}",
                        target.display()
                    ))
                })?;
                if existing == stored {
                    return Ok(CaptureProjectionWriteResult {
                        outcome: CaptureProjectionWriteOutcome::Reused,
                        projection,
                        projection_path: target,
                    });
                }
                CaptureProjectionWriteOutcome::Replaced
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                CaptureProjectionWriteOutcome::Created
            }
            Err(error) => {
                return Err(not_persisted(format!(
                    "cannot inspect capture projection {}: {error}",
                    target.display()
                )));
            }
        };
        let temp = self.unique_projection_temp_path(projection.capture_id())?;
        let parent = self.projections_dir();
        let write_result = (|| -> Result<()> {
            let mut file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&temp)
                .map_err(|error| {
                    not_persisted(format!(
                        "cannot create capture projection temp file {}: {error}",
                        temp.display()
                    ))
                })?;
            file.write_all(&stored).map_err(|error| {
                not_persisted(format!(
                    "cannot write capture projection temp file {}: {error}",
                    temp.display()
                ))
            })?;
            file.sync_all().map_err(|error| {
                not_persisted(format!(
                    "cannot sync capture projection temp file {}: {error}",
                    temp.display()
                ))
            })?;
            drop(file);
            fs::rename(&temp, &target).map_err(|error| {
                not_persisted(format!(
                    "cannot atomically replace capture projection {} from {}: {error}",
                    target.display(),
                    temp.display()
                ))
            })?;
            sync_directory(&parent).map_err(|error| {
                not_persisted(format!(
                    "cannot sync capture projection directory {}: {error}",
                    parent.display()
                ))
            })?;
            Ok(())
        })();
        if write_result.is_err() {
            let _ = fs::remove_file(&temp);
        }
        write_result?;
        Ok(CaptureProjectionWriteResult {
            outcome,
            projection,
            projection_path: target,
        })
    }

    fn load_events(&self, capture_id: CaptureId) -> Result<Vec<CaptureEvent>> {
        let directory = self.capture_events_dir(capture_id);
        let metadata = match fs::symlink_metadata(&directory) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(error) => {
                return Err(not_persisted(format!(
                    "cannot inspect capture event directory {}: {error}",
                    directory.display()
                )));
            }
        };
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            return Err(not_persisted(format!(
                "capture event directory {} must be a regular non-symlink directory",
                directory.display()
            )));
        }
        let mut entries = Vec::new();
        for entry in fs::read_dir(&directory).map_err(|error| {
            not_persisted(format!(
                "cannot read capture event directory {}: {error}",
                directory.display()
            ))
        })? {
            let entry = entry.map_err(|error| {
                not_persisted(format!(
                    "cannot inspect an entry in capture event directory {}: {error}",
                    directory.display()
                ))
            })?;
            let path = entry.path();
            let metadata = fs::symlink_metadata(&path).map_err(|error| {
                not_persisted(format!(
                    "cannot inspect capture event entry {}: {error}",
                    path.display()
                ))
            })?;
            if metadata.file_type().is_symlink() || !metadata.is_file() {
                return Err(not_persisted(format!(
                    "capture event entry {} must be a regular non-symlink file",
                    path.display()
                )));
            }
            let file_name = path
                .file_name()
                .and_then(|value| value.to_str())
                .ok_or_else(|| {
                    not_persisted(format!(
                        "capture event entry {} has a non-UTF-8 file name",
                        path.display()
                    ))
                })?;
            let stem = file_name.strip_suffix(".json").ok_or_else(|| {
                not_persisted(format!(
                    "capture event entry {} does not use the required .json suffix",
                    path.display()
                ))
            })?;
            let (sequence_text, event_id_text) = stem.split_once('-').ok_or_else(|| {
                not_persisted(format!(
                    "capture event entry {} does not use <sequence>-<event-id>.json",
                    path.display()
                ))
            })?;
            let sequence = sequence_text.parse::<u64>().map_err(|error| {
                not_persisted(format!(
                    "capture event entry {} has invalid sequence: {error}",
                    path.display()
                ))
            })?;
            if sequence_text != format!("{sequence:020}") {
                return Err(not_persisted(format!(
                    "capture event entry {} sequence must be zero-padded to 20 digits",
                    path.display()
                )));
            }
            let event_id = EventId::parse_canonical(event_id_text).map_err(|error| {
                not_persisted(format!(
                    "capture event entry {} has invalid event ID: {error}",
                    path.display()
                ))
            })?;
            entries.push((sequence, event_id, path));
        }
        entries.sort_by_key(|(sequence, _, _)| *sequence);
        let mut events = Vec::with_capacity(entries.len());
        let mut previous_digest = None;
        for (index, (sequence, event_id, path)) in entries.into_iter().enumerate() {
            let expected_sequence = u64::try_from(index).map_err(|_| {
                WorkVcsError::ControlPlaneInvalid("too many capture events".to_owned())
            })? + 1;
            if sequence != expected_sequence {
                return Err(not_persisted(format!(
                    "capture event stream for {} has gap or reordering: expected sequence {expected_sequence}, found {sequence}",
                    capture_id
                )));
            }
            let bytes = fs::read(&path).map_err(|error| {
                not_persisted(format!(
                    "cannot read capture event {}: {error}",
                    path.display()
                ))
            })?;
            let event = CaptureEvent::from_json_bytes(&bytes).map_err(|error| {
                not_persisted(format!(
                    "capture event {} is invalid: {error}",
                    path.display()
                ))
            })?;
            if bytes != event.stored_json_bytes()? {
                return Err(not_persisted(format!(
                    "capture event {} is not stored in canonical journal form",
                    path.display()
                )));
            }
            if event.capture_id() != capture_id
                || event.sequence() != sequence
                || event.event_id() != event_id
            {
                return Err(not_persisted(format!(
                    "capture event {} identity does not match its directory or file name",
                    path.display()
                )));
            }
            if event.previous_event_digest() != previous_digest.as_ref() {
                return Err(not_persisted(format!(
                    "capture event {} previous_event_digest does not match the preceding immutable event",
                    path.display()
                )));
            }
            previous_digest = Some(event.digest()?);
            events.push(event);
        }
        Ok(events)
    }

    fn intent_paths_readonly(&self) -> Result<Vec<PathBuf>> {
        let root_metadata = match fs::symlink_metadata(&self.root) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(error) => {
                return Err(not_persisted(format!(
                    "cannot inspect capture journal root {}: {error}",
                    self.root.display()
                )));
            }
        };
        if root_metadata.file_type().is_symlink() || !root_metadata.is_dir() {
            return Err(not_persisted(format!(
                "capture journal root {} must be a regular non-symlink directory",
                self.root.display()
            )));
        }
        let directory = self.intents_dir();
        let metadata = match fs::symlink_metadata(&directory) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(error) => {
                return Err(not_persisted(format!(
                    "cannot inspect capture intents directory {}: {error}",
                    directory.display()
                )));
            }
        };
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            return Err(not_persisted(format!(
                "capture intents directory {} must be a regular non-symlink directory",
                directory.display()
            )));
        }
        let mut paths = Vec::new();
        for entry in fs::read_dir(&directory).map_err(|error| {
            not_persisted(format!(
                "cannot read capture intents directory {}: {error}",
                directory.display()
            ))
        })? {
            let entry = entry.map_err(|error| {
                not_persisted(format!(
                    "cannot inspect an entry in capture intents directory {}: {error}",
                    directory.display()
                ))
            })?;
            let path = entry.path();
            let metadata = fs::symlink_metadata(&path).map_err(|error| {
                not_persisted(format!(
                    "cannot inspect capture intent entry {}: {error}",
                    path.display()
                ))
            })?;
            if metadata.file_type().is_symlink() || !metadata.is_file() {
                return Err(not_persisted(format!(
                    "capture intent entry {} must be a regular non-symlink file",
                    path.display()
                )));
            }
            let file_name = path
                .file_name()
                .and_then(|value| value.to_str())
                .ok_or_else(|| {
                    not_persisted(format!(
                        "capture intent entry {} has a non-UTF-8 file name",
                        path.display()
                    ))
                })?;
            if file_name.starts_with('.') && file_name.ends_with(".tmp") {
                return Err(not_persisted(format!(
                    "capture intent temp entry {} makes secondary recall indeterminate",
                    path.display()
                )));
            }
            let Some(stem) = file_name.strip_suffix(".json") else {
                return Err(not_persisted(format!(
                    "capture intent entry {} does not use the required .json suffix",
                    path.display()
                )));
            };
            CaptureId::parse_canonical(stem).map_err(|error| {
                not_persisted(format!(
                    "capture intent entry {} has invalid capture ID: {error}",
                    path.display()
                ))
            })?;
            paths.push(path);
        }
        paths.sort();
        Ok(paths)
    }

    fn event_path(&self, event: &CaptureEvent) -> PathBuf {
        self.capture_events_dir(event.capture_id()).join(format!(
            "{:020}-{}.json",
            event.sequence(),
            event.event_id()
        ))
    }

    fn capture_events_dir(&self, capture_id: CaptureId) -> PathBuf {
        self.events_dir().join(capture_id.to_string())
    }

    fn capture_lock_path(&self, capture_id: CaptureId) -> PathBuf {
        self.locks_dir().join(format!("{capture_id}.lock"))
    }

    fn ensure_layout(&self) -> Result<()> {
        ensure_regular_directory(&self.root, "capture journal root", true)?;
        for directory in [
            self.intents_dir(),
            self.events_dir(),
            self.projections_dir(),
            self.locks_dir(),
        ] {
            ensure_regular_directory(&directory, "capture journal directory", true)?;
        }
        sync_directory(&self.root).map_err(|error| {
            not_persisted(format!(
                "cannot sync capture journal root {}: {error}",
                self.root.display()
            ))
        })
    }

    fn ensure_lock_parent(&self) -> Result<()> {
        let parent = self.quiescence_lock_path.parent().ok_or_else(|| {
            not_persisted(format!(
                "capture journal quiescence lock path {} has no parent directory",
                self.quiescence_lock_path.display()
            ))
        })?;
        fs::create_dir_all(parent).map_err(|error| {
            not_persisted(format!(
                "cannot create capture journal quiescence lock directory {}: {error}",
                parent.display()
            ))
        })?;
        if !parent.is_dir() {
            return Err(not_persisted(format!(
                "capture journal quiescence lock parent {} is not a directory",
                parent.display()
            )));
        }
        Ok(())
    }

    fn find_idempotency_key(&self, key: &str) -> Result<Option<(CaptureIntent, PathBuf)>> {
        let mut paths = Vec::new();
        for entry in fs::read_dir(self.intents_dir()).map_err(|error| {
            not_persisted(format!(
                "cannot read capture intents directory {}: {error}",
                self.intents_dir().display()
            ))
        })? {
            let entry = entry.map_err(|error| {
                not_persisted(format!(
                    "cannot inspect an entry in capture intents directory {}: {error}",
                    self.intents_dir().display()
                ))
            })?;
            let path = entry.path();
            if path.extension().and_then(|value| value.to_str()) == Some("json") {
                paths.push(path);
            }
        }
        paths.sort();
        let mut found: Option<(CaptureIntent, PathBuf)> = None;
        for path in paths {
            let intent = self.load_path(&path)?;
            if intent.idempotency_key() == key {
                if let Some((existing, existing_path)) = &found {
                    return Err(not_persisted(format!(
                        "capture journal contains duplicate idempotency key {key:?} in {} and {} (captures {} and {})",
                        existing_path.display(),
                        path.display(),
                        existing.capture_id(),
                        intent.capture_id()
                    )));
                }
                found = Some((intent, path));
            }
        }
        Ok(found)
    }

    fn load_path(&self, path: &Path) -> Result<CaptureIntent> {
        let metadata = fs::symlink_metadata(path).map_err(|error| {
            not_persisted(format!(
                "cannot inspect capture intent {}: {error}",
                path.display()
            ))
        })?;
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return Err(not_persisted(format!(
                "capture intent {} must be a regular non-symlink file",
                path.display()
            )));
        }
        let bytes = fs::read(path).map_err(|error| {
            not_persisted(format!(
                "cannot read capture intent {}: {error}",
                path.display()
            ))
        })?;
        let intent = CaptureIntent::from_json_bytes(&bytes).map_err(|error| {
            not_persisted(format!(
                "capture intent {} is invalid: {error}",
                path.display()
            ))
        })?;
        if bytes != intent.stored_json_bytes()? {
            return Err(not_persisted(format!(
                "capture intent {} is not stored in canonical journal form",
                path.display()
            )));
        }
        Ok(intent)
    }

    fn unique_temp_path(&self, capture_id: CaptureId) -> Result<PathBuf> {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|error| not_persisted(format!("system clock is before UNIX epoch: {error}")))?
            .as_nanos();
        Ok(self.intents_dir().join(format!(
            ".{capture_id}.{}.{}.tmp",
            std::process::id(),
            nanos
        )))
    }

    fn unique_event_temp_path(&self, event: &CaptureEvent) -> Result<PathBuf> {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|error| not_persisted(format!("system clock is before UNIX epoch: {error}")))?
            .as_nanos();
        Ok(self.capture_events_dir(event.capture_id()).join(format!(
            ".{:020}-{}.{}.{}.tmp",
            event.sequence(),
            event.event_id(),
            std::process::id(),
            nanos
        )))
    }

    fn unique_projection_temp_path(&self, capture_id: CaptureId) -> Result<PathBuf> {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|error| not_persisted(format!("system clock is before UNIX epoch: {error}")))?
            .as_nanos();
        Ok(self.projections_dir().join(format!(
            ".{capture_id}.{}.{}.tmp",
            std::process::id(),
            nanos
        )))
    }

    fn intents_dir(&self) -> PathBuf {
        self.root.join("intents")
    }

    fn events_dir(&self) -> PathBuf {
        self.root.join("events")
    }

    fn projections_dir(&self) -> PathBuf {
        self.root.join("projections")
    }

    fn locks_dir(&self) -> PathBuf {
        self.root.join("locks")
    }
}

pub fn project_registry_journal_quiescence_lock_path(registry_path: &Path) -> Result<PathBuf> {
    if !registry_path.is_absolute() {
        return invalid("project registry path for journal quiescence must be absolute");
    }
    let canonical_registry_path = fs::canonicalize(registry_path).map_err(|error| {
        WorkVcsError::ControlPlaneInvalid(format!(
            "project registry path {} cannot establish a journal quiescence identity: {error}",
            registry_path.display()
        ))
    })?;
    if canonical_registry_path != registry_path {
        return invalid(format!(
            "project registry path {} must be canonical before deriving journal quiescence identity; canonical path is {}",
            registry_path.display(),
            canonical_registry_path.display()
        ));
    }
    let file_name = registry_path.file_name().ok_or_else(|| {
        WorkVcsError::ControlPlaneInvalid(format!(
            "project registry path {} has no file name",
            registry_path.display()
        ))
    })?;
    let mut lock_name = OsString::from(file_name);
    lock_name.push(".journal-quiescence.lock");
    Ok(registry_path.with_file_name(lock_name))
}

pub struct JournalQuiescenceLock {
    path: PathBuf,
    owner: Vec<u8>,
}

impl JournalQuiescenceLock {
    pub fn acquire(path: &Path) -> Result<Self> {
        if !path.is_absolute() {
            return invalid("journal quiescence lock path must be absolute");
        }
        let parent = path.parent().ok_or_else(|| {
            not_persisted(format!(
                "journal quiescence lock path {} has no parent directory",
                path.display()
            ))
        })?;
        let parent_metadata = fs::symlink_metadata(parent).map_err(|error| {
            not_persisted(format!(
                "cannot inspect journal quiescence lock parent {}: {error}",
                parent.display()
            ))
        })?;
        if parent_metadata.file_type().is_symlink() || !parent_metadata.is_dir() {
            return Err(not_persisted(format!(
                "journal quiescence lock parent {} must be a regular non-symlink directory",
                parent.display()
            )));
        }
        let created_at_nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|error| not_persisted(format!("system clock is before UNIX epoch: {error}")))?
            .as_nanos();
        let nonce = JOURNAL_LOCK_NONCE.fetch_add(1, Ordering::Relaxed);
        let owner = format!(
            "pid={}\ncreated_at_nanos={created_at_nanos}\nnonce={nonce}\n",
            std::process::id()
        )
        .into_bytes();
        let started = Instant::now();
        loop {
            match OpenOptions::new().write(true).create_new(true).open(path) {
                Ok(mut file) => {
                    let initialize = (|| -> Result<()> {
                        file.write_all(&owner).map_err(|error| {
                            not_persisted(format!(
                                "cannot write journal quiescence lock {}: {error}",
                                path.display()
                            ))
                        })?;
                        file.sync_all().map_err(|error| {
                            not_persisted(format!(
                                "cannot sync journal quiescence lock {}: {error}",
                                path.display()
                            ))
                        })?;
                        Ok(())
                    })();
                    if let Err(error) = initialize {
                        drop(file);
                        if fs::read(path).is_ok_and(|actual| actual == owner) {
                            let _ = fs::remove_file(path);
                        }
                        return Err(error);
                    }
                    return Ok(Self {
                        path: path.to_path_buf(),
                        owner: owner.clone(),
                    });
                }
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                    if started.elapsed() >= JOURNAL_LOCK_TIMEOUT {
                        return Err(not_persisted(format!(
                            "journal quiescence lock {} is already held; refusing admission or rollback until the exact lock owner is resolved",
                            path.display()
                        )));
                    }
                    thread::sleep(JOURNAL_LOCK_RETRY);
                }
                Err(error) => {
                    return Err(not_persisted(format!(
                        "cannot acquire journal quiescence lock {}: {error}",
                        path.display()
                    )));
                }
            }
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn verify_owned(&self) -> Result<()> {
        let metadata = fs::symlink_metadata(&self.path).map_err(|error| {
            not_persisted(format!(
                "cannot verify journal quiescence lock {}: {error}",
                self.path.display()
            ))
        })?;
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return Err(not_persisted(format!(
                "journal quiescence lock {} is no longer a regular non-symlink file",
                self.path.display()
            )));
        }
        let actual = fs::read(&self.path).map_err(|error| {
            not_persisted(format!(
                "cannot read journal quiescence lock {}: {error}",
                self.path.display()
            ))
        })?;
        if actual != self.owner {
            return Err(not_persisted(format!(
                "journal quiescence lock {} ownership changed while held",
                self.path.display()
            )));
        }
        Ok(())
    }
}

impl Drop for JournalQuiescenceLock {
    fn drop(&mut self) {
        if self.verify_owned().is_ok() {
            let _ = fs::remove_file(&self.path);
        }
    }
}

fn not_persisted(message: impl Into<String>) -> WorkVcsError {
    WorkVcsError::CaptureNotPersisted(message.into())
}

fn ensure_regular_directory(path: &Path, label: &str, create: bool) -> Result<()> {
    if create {
        fs::create_dir_all(path).map_err(|error| {
            not_persisted(format!("cannot create {label} {}: {error}", path.display()))
        })?;
    }
    let metadata = fs::symlink_metadata(path).map_err(|error| {
        not_persisted(format!(
            "cannot inspect {label} {}: {error}",
            path.display()
        ))
    })?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(not_persisted(format!(
            "{label} {} must be a regular non-symlink directory",
            path.display()
        )));
    }
    Ok(())
}

#[cfg(unix)]
fn sync_directory(path: &Path) -> std::io::Result<()> {
    File::open(path)?.sync_all()
}

#[cfg(not(unix))]
fn sync_directory(_path: &Path) -> std::io::Result<()> {
    Ok(())
}
