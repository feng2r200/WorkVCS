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
    EntityVersionId, EventId, EvidenceId, PlanAdmissionGoalManifest, PlanAdmissionManifest,
    PlanAdmissionRecordManifest, PlanAdmissionTaskManifest, PlanEvolutionManifest,
    ProjectLocatorId, ProjectRefId, RegistryId, RelationId, RelationVersionId, StoreId,
    WorkspaceId,
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
const MAX_CAPTURE_EVENT_TIMESTAMP: &str = "9999-12-31T23:59:59.999999999Z";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapturePayloadKind {
    CognitionV2,
    LegacyCognitionV1,
    PlanAdmitV1,
    PlanEvolveV1,
}

impl CapturePayloadKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::CognitionV2 => "cognition_v2",
            Self::LegacyCognitionV1 => "legacy_cognition_v1",
            Self::PlanAdmitV1 => "plan_admit_v1",
            Self::PlanEvolveV1 => "plan_evolve_v1",
        }
    }

    pub const fn is_cognition(self) -> bool {
        matches!(self, Self::CognitionV2 | Self::LegacyCognitionV1)
    }
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
            "knowledge"
            | "goal"
            | "plan"
            | "task"
            | "acceptance_criterion"
            | "verification_requirement" => {
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    operation_payload_kind: Option<CapturePayloadKind>,
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
        Self::new_for_operation(
            delivery_id,
            project_ref_id,
            store_id,
            workspace_id,
            branch_id,
            commit_id,
            changeset_id,
            state_digest,
            result_objects,
            reused,
            None,
            canonical_record_ref,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn new_plan(
        operation_payload_kind: CapturePayloadKind,
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
    ) -> Result<Self> {
        if !matches!(
            operation_payload_kind,
            CapturePayloadKind::PlanAdmitV1 | CapturePayloadKind::PlanEvolveV1
        ) {
            return invalid("Plan delivery receipt requires a Plan operation payload kind");
        }
        Self::new_for_operation(
            delivery_id,
            project_ref_id,
            store_id,
            workspace_id,
            branch_id,
            commit_id,
            changeset_id,
            state_digest,
            result_objects,
            reused,
            Some(operation_payload_kind),
            None,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn new_for_operation(
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
        operation_payload_kind: Option<CapturePayloadKind>,
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
            operation_payload_kind,
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

    pub fn operation_payload_kind(&self) -> Option<CapturePayloadKind> {
        self.operation_payload_kind
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
        if self
            .operation_payload_kind
            .is_some_and(CapturePayloadKind::is_cognition)
        {
            return invalid(
                "delivery_applied operation_payload_kind may identify only a Plan operation",
            );
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
            has_canonical_content |= matches!(
                object.object_kind.as_str(),
                "knowledge"
                    | "goal"
                    | "plan"
                    | "task"
                    | "acceptance_criterion"
                    | "verification_requirement"
            ) || object.object_kind.as_str().starts_with("record:");
        }
        if !has_canonical_content {
            return invalid(
                "delivery_applied requires at least one canonical entity or Record result",
            );
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

    fn validate_for_payload_kind(&self, payload_kind: CapturePayloadKind) -> Result<()> {
        match payload_kind {
            CapturePayloadKind::CognitionV2 | CapturePayloadKind::LegacyCognitionV1 => {
                if self.operation_payload_kind.is_some() {
                    return invalid(format!(
                        "{} intent cannot carry a Plan operation receipt",
                        payload_kind.as_str()
                    ));
                }
                for object in &self.result_objects {
                    let kind = object.object_kind();
                    if matches!(
                        kind,
                        "goal"
                            | "plan"
                            | "task"
                            | "acceptance_criterion"
                            | "verification_requirement"
                    ) || matches!(kind, "relation:contains" | "relation:supersedes")
                    {
                        return invalid(format!(
                            "{} intent cannot carry Plan result object kind {kind:?}",
                            payload_kind.as_str()
                        ));
                    }
                }
            }
            CapturePayloadKind::PlanAdmitV1 | CapturePayloadKind::PlanEvolveV1 => {
                if self.operation_payload_kind != Some(payload_kind) {
                    return invalid(format!(
                        "{} delivery receipt requires operation_payload_kind {}",
                        payload_kind.as_str(),
                        payload_kind.as_str()
                    ));
                }
                if self.canonical_record_ref().is_some() {
                    return invalid(format!(
                        "{} delivery receipt cannot carry a CaptureGroup canonical Record reference",
                        payload_kind.as_str()
                    ));
                }
                if !self
                    .result_objects
                    .iter()
                    .any(|object| object.object_kind() == "plan")
                {
                    return invalid(format!(
                        "{} delivery receipt requires a Plan result",
                        payload_kind.as_str()
                    ));
                }
                for object in &self.result_objects {
                    let kind = object.object_kind();
                    let allowed = matches!(
                        kind,
                        "goal"
                            | "plan"
                            | "task"
                            | "acceptance_criterion"
                            | "verification_requirement"
                            | "evidence"
                            | "relation:contains"
                            | "relation:supersedes"
                    ) || kind.starts_with("record:");
                    if !allowed {
                        return invalid(format!(
                            "{} delivery receipt cannot carry result object kind {kind:?}",
                            payload_kind.as_str()
                        ));
                    }
                }
                if payload_kind == CapturePayloadKind::PlanAdmitV1
                    && self.result_objects.iter().any(|object| {
                        matches!(
                            object.object_kind(),
                            "relation:contains" | "relation:supersedes"
                        ) || object.local_id() == "new_plan"
                    })
                {
                    return invalid(
                        "plan_admit_v1 delivery receipt cannot carry Plan-evolution results",
                    );
                }
                if payload_kind == CapturePayloadKind::PlanEvolveV1
                    && self
                        .result_objects
                        .iter()
                        .any(|object| object.object_kind() == "goal")
                {
                    return invalid("plan_evolve_v1 delivery receipt cannot create a Goal result");
                }
            }
        }
        Ok(())
    }

    fn validate_for_intent(&self, intent: &CaptureIntent) -> Result<()> {
        self.validate_for_payload_kind(intent.payload_kind())?;
        match intent.payload_kind() {
            CapturePayloadKind::CognitionV2 | CapturePayloadKind::LegacyCognitionV1 => Ok(()),
            CapturePayloadKind::PlanAdmitV1 => {
                let manifest = parse_plan_admission_intent(intent)?;
                self.validate_exact_plan_result_shape(expected_admission_result_shape(&manifest)?)
            }
            CapturePayloadKind::PlanEvolveV1 => {
                let manifest = parse_plan_evolution_intent(intent)?;
                self.validate_exact_plan_result_shape(expected_evolution_result_shape(&manifest)?)
            }
        }
    }

    fn validate_exact_plan_result_shape(&self, expected: BTreeSet<(String, String)>) -> Result<()> {
        let operation_payload_kind = self.operation_payload_kind.ok_or_else(|| {
            WorkVcsError::ControlPlaneInvalid(
                "Plan result shape validation requires an operation payload kind".to_owned(),
            )
        })?;
        let actual = self
            .result_objects
            .iter()
            .map(|object| {
                (
                    object.local_id().to_owned(),
                    object.object_kind().to_owned(),
                )
            })
            .collect::<BTreeSet<_>>();
        if actual != expected {
            return invalid(format!(
                "{} delivery receipt result shape does not exactly match its admitted manifest",
                operation_payload_kind.as_str()
            ));
        }
        Ok(())
    }
}

fn parse_plan_admission_intent(intent: &CaptureIntent) -> Result<PlanAdmissionManifest> {
    let bytes = serde_json::to_vec(intent.semantic_payload()).map_err(|error| {
        WorkVcsError::ControlPlaneInvalid(format!(
            "cannot serialize admitted plan_admit_v1 payload: {error}"
        ))
    })?;
    PlanAdmissionManifest::from_json_bytes(&bytes).map_err(|error| {
        WorkVcsError::ControlPlaneInvalid(format!(
            "admitted plan_admit_v1 payload is not a Plan admission manifest: {error}"
        ))
    })
}

fn parse_plan_evolution_intent(intent: &CaptureIntent) -> Result<PlanEvolutionManifest> {
    let bytes = serde_json::to_vec(intent.semantic_payload()).map_err(|error| {
        WorkVcsError::ControlPlaneInvalid(format!(
            "cannot serialize admitted plan_evolve_v1 payload: {error}"
        ))
    })?;
    PlanEvolutionManifest::from_json_bytes(&bytes).map_err(|error| {
        WorkVcsError::ControlPlaneInvalid(format!(
            "admitted plan_evolve_v1 payload is not a Plan evolution manifest: {error}"
        ))
    })
}

fn expected_admission_result_shape(
    manifest: &PlanAdmissionManifest,
) -> Result<BTreeSet<(String, String)>> {
    let mut expected = BTreeSet::new();
    if matches!(manifest.goal, PlanAdmissionGoalManifest::Create { .. }) {
        expected.insert(("goal".to_owned(), "goal".to_owned()));
    }
    expected.insert(("plan".to_owned(), "plan".to_owned()));
    append_expected_manifest_results(
        &mut expected,
        &manifest.tasks,
        &manifest.records,
        manifest.evidence.len(),
    )?;
    Ok(expected)
}

fn expected_evolution_result_shape(
    manifest: &PlanEvolutionManifest,
) -> Result<BTreeSet<(String, String)>> {
    let mut expected = BTreeSet::from([("plan".to_owned(), "plan".to_owned())]);
    let (tasks, records, evidence_len, supersede) = match manifest {
        PlanEvolutionManifest::InPlace(manifest) => (
            &manifest.tasks,
            &manifest.records,
            manifest.evidence.len(),
            false,
        ),
        PlanEvolutionManifest::Supersede(manifest) => (
            &manifest.tasks,
            &manifest.records,
            manifest.evidence.len(),
            true,
        ),
    };
    if supersede {
        expected.insert(("new_plan".to_owned(), "plan".to_owned()));
    }
    append_expected_manifest_results(&mut expected, tasks, records, evidence_len)?;
    if supersede {
        expected.insert((
            "goal_contains_relation".to_owned(),
            "relation:contains".to_owned(),
        ));
        expected.insert((
            "supersedes_relation".to_owned(),
            "relation:supersedes".to_owned(),
        ));
    }
    Ok(expected)
}

fn append_expected_manifest_results(
    expected: &mut BTreeSet<(String, String)>,
    tasks: &[PlanAdmissionTaskManifest],
    records: &[PlanAdmissionRecordManifest],
    evidence_len: usize,
) -> Result<()> {
    for (task_index, task) in tasks.iter().enumerate() {
        expected.insert((format!("task.{task_index}"), "task".to_owned()));
        for (criterion_index, criterion) in task.acceptance_criteria.iter().enumerate() {
            expected.insert((
                format!("task.{task_index}.acceptance_criterion.{criterion_index}"),
                "acceptance_criterion".to_owned(),
            ));
            for requirement_index in 0..criterion.verification_requirements.len() {
                expected.insert((
                    format!(
                        "task.{task_index}.acceptance_criterion.{criterion_index}.verification_requirement.{requirement_index}"
                    ),
                    "verification_requirement".to_owned(),
                ));
            }
        }
    }
    for (index, record) in records.iter().enumerate() {
        expected.insert((
            format!("record.{index}"),
            format!("record:{}", record.canonical_kind()?),
        ));
    }
    for index in 0..evidence_len {
        expected.insert((format!("evidence.{index}"), "evidence".to_owned()));
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeliveryFailureCode {
    LegacyManifestUpgradeRequired,
    SemanticManifestInvalid,
    PlanTargetConflict,
    PlanManifestRejected,
    PlanReceiptTooLarge,
}

impl DeliveryFailureCode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::LegacyManifestUpgradeRequired => "legacy_manifest_upgrade_required",
            Self::SemanticManifestInvalid => "semantic_manifest_invalid",
            Self::PlanTargetConflict => "plan_target_conflict",
            Self::PlanManifestRejected => "plan_manifest_rejected",
            Self::PlanReceiptTooLarge => "plan_receipt_too_large",
        }
    }

    pub const fn recovery_action(self) -> &'static str {
        match self {
            Self::LegacyManifestUpgradeRequired => "upgrade_legacy_manifest",
            Self::SemanticManifestInvalid => "start_new_capture_with_corrected_payload",
            Self::PlanTargetConflict => "start_new_plan_operation_with_current_guards",
            Self::PlanManifestRejected => "start_new_plan_operation_with_corrected_manifest",
            Self::PlanReceiptTooLarge => "start_new_plan_operation_with_smaller_manifest",
        }
    }
}

fn recovery_state_for_delivery_failure(
    failure: Option<&DeliveryFailedPayload>,
) -> Option<CaptureRecoveryState> {
    failure.map(|failure| match failure.failure_code() {
        DeliveryFailureCode::LegacyManifestUpgradeRequired => {
            CaptureRecoveryState::LegacyManifestUpgradeRequired
        }
        DeliveryFailureCode::SemanticManifestInvalid => {
            CaptureRecoveryState::SemanticManifestInvalid
        }
        DeliveryFailureCode::PlanTargetConflict => CaptureRecoveryState::PlanTargetConflict,
        DeliveryFailureCode::PlanManifestRejected => CaptureRecoveryState::PlanManifestRejected,
        DeliveryFailureCode::PlanReceiptTooLarge => CaptureRecoveryState::PlanReceiptTooLarge,
    })
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
        if self.recovery_action.as_str() != self.failure_code.recovery_action() {
            return invalid(format!(
                "delivery failure {} requires recovery action {:?}, found {:?}",
                self.failure_code.as_str(),
                self.failure_code.recovery_action(),
                self.recovery_action.as_str()
            ));
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
pub enum OperationDisposition {
    Superseded,
    Abandoned,
}

impl OperationDisposition {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Superseded => "superseded",
            Self::Abandoned => "abandoned",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OperationDispositionRecordedPayload {
    disposition: OperationDisposition,
    #[serde(default)]
    successor_capture_id: Nullable<CaptureId>,
}

impl OperationDispositionRecordedPayload {
    pub fn new(
        disposition: OperationDisposition,
        successor_capture_id: Option<CaptureId>,
    ) -> Result<Self> {
        let payload = Self {
            disposition,
            successor_capture_id: Nullable::present(successor_capture_id),
        };
        payload.validate()?;
        Ok(payload)
    }

    pub fn disposition(&self) -> OperationDisposition {
        self.disposition
    }

    pub fn successor_capture_id(&self) -> Option<CaptureId> {
        self.successor_capture_id.copied()
    }

    fn validate(&self) -> Result<()> {
        if !self.successor_capture_id.is_present() {
            return invalid(
                "operation disposition successor_capture_id is required even when null",
            );
        }
        match (self.disposition, self.successor_capture_id()) {
            (OperationDisposition::Superseded, None) => {
                return invalid("superseded operation disposition requires a successor CaptureId");
            }
            (OperationDisposition::Abandoned, Some(_)) => {
                return invalid("abandoned operation disposition forbids a successor CaptureId");
            }
            _ => {}
        }
        Ok(())
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
    OperationDispositionRecorded(OperationDispositionRecordedPayload),
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
            Self::OperationDispositionRecorded(payload) => payload.validate(),
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
            Self::OperationDispositionRecorded(_) => "operation_disposition_recorded",
        }
    }

    fn validate_for_intent(&self, intent: &CaptureIntent) -> Result<()> {
        match self {
            Self::DeliveryApplied(payload) => {
                payload.validate_for_intent(intent)?;
            }
            Self::DeliveryFailed(payload) => {
                let valid_family = match payload.failure_code() {
                    DeliveryFailureCode::LegacyManifestUpgradeRequired => {
                        intent.payload_kind() == CapturePayloadKind::LegacyCognitionV1
                    }
                    DeliveryFailureCode::SemanticManifestInvalid => {
                        intent.payload_kind().is_cognition()
                    }
                    DeliveryFailureCode::PlanTargetConflict
                    | DeliveryFailureCode::PlanManifestRejected
                    | DeliveryFailureCode::PlanReceiptTooLarge => matches!(
                        intent.payload_kind(),
                        CapturePayloadKind::PlanAdmitV1 | CapturePayloadKind::PlanEvolveV1
                    ),
                };
                if !valid_family {
                    return invalid(format!(
                        "{} intent cannot carry delivery failure {}",
                        intent.payload_kind().as_str(),
                        payload.failure_code().as_str()
                    ));
                }
            }
            Self::CaptureGroupResolved(_)
            | Self::ReferenceApplied(_)
            | Self::CaptureCompleted(_)
                if !intent.payload_kind().is_cognition() =>
            {
                return invalid(format!(
                    "{} intent cannot carry CaptureGroup event {}",
                    intent.payload_kind().as_str(),
                    self.kind_name()
                ));
            }
            Self::OperationDispositionRecorded(payload)
                if payload.successor_capture_id() == Some(intent.capture_id()) =>
            {
                return invalid("operation disposition successor must differ from its CaptureId");
            }
            _ => {}
        }
        Ok(())
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
        let event = Self::candidate(
            capture_id,
            sequence,
            event_id,
            occurred_at,
            previous_event_digest,
            payload,
        )?;
        event.validate_size()?;
        Ok(event)
    }

    fn candidate(
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
        event.validate_structure()?;
        Ok(event)
    }

    fn worst_case_candidate(
        capture_id: CaptureId,
        sequence: u64,
        event_id: EventId,
        previous_event_digest: Option<ControlPlaneDigest>,
        payload: CaptureEventPayload,
    ) -> Result<Self> {
        Self::candidate(
            capture_id,
            sequence,
            event_id,
            UtcTimestamp::parse(MAX_CAPTURE_EVENT_TIMESTAMP)?,
            previous_event_digest,
            payload,
        )
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
        self.validate_structure()?;
        self.validate_size()
    }

    fn validate_structure(&self) -> Result<()> {
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
        Ok(())
    }

    fn validate_size(&self) -> Result<()> {
        let canonical_size = canonicalize_serializable(self, "capture event")?.len();
        if canonical_size > MAX_CAPTURE_EVENT_BYTES {
            return invalid(format!(
                "canonical capture event is {canonical_size} bytes; maximum is {MAX_CAPTURE_EVENT_BYTES}"
            ));
        }
        Ok(())
    }

    fn canonical_size(&self) -> Result<usize> {
        Ok(canonicalize_serializable(self, "capture event")?.len())
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
    SemanticManifestInvalid,
    PlanTargetConflict,
    PlanManifestRejected,
    PlanReceiptTooLarge,
    Completed,
    Superseded,
    Abandoned,
}

impl CaptureRecoveryState {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::PendingResolution => "pending_resolution",
            Self::PendingProject => "pending_project",
            Self::PendingPrimary => "pending_primary",
            Self::PendingReferences => "pending_references",
            Self::LegacyManifestUpgradeRequired => "legacy_manifest_upgrade_required",
            Self::SemanticManifestInvalid => "semantic_manifest_invalid",
            Self::PlanTargetConflict => "plan_target_conflict",
            Self::PlanManifestRejected => "plan_manifest_rejected",
            Self::PlanReceiptTooLarge => "plan_receipt_too_large",
            Self::Completed => "completed",
            Self::Superseded => "superseded",
            Self::Abandoned => "abandoned",
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
    #[serde(default)]
    operation_disposition: Nullable<OperationDispositionRecordedPayload>,
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
        let mut operation_disposition: Option<OperationDispositionRecordedPayload> = None;
        for event in events {
            event.payload().validate_for_intent(intent)?;
            if operation_disposition.is_some() {
                return invalid(format!(
                    "capture event {} follows terminal operation_disposition_recorded authority",
                    event.event_id()
                ));
            }
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
                    if delivery_failure.is_some() {
                        return invalid(format!(
                            "delivery_applied event {} cannot replace a terminal delivery_failed event",
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
                    if delivery_failure.is_some() {
                        return invalid(format!(
                            "delivery_failed event {} cannot replace a terminal delivery_failed event",
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
                CaptureEventPayload::OperationDispositionRecorded(payload) => {
                    let group_has_delivery_authority =
                        capture_group.as_ref().is_some_and(|group| {
                            group.canonical_record_ref().is_some()
                                || group.applied_reference_count() != 0
                                || group.completion_receipt().is_some()
                        });
                    if delivery_started.is_some()
                        || primary_delivery.is_some()
                        || delivery_failure.is_some()
                        || group_has_delivery_authority
                    {
                        return invalid(format!(
                            "operation_disposition_recorded event {} cannot terminate target-bearing authority",
                            event.event_id()
                        ));
                    }
                    operation_disposition = Some(payload.clone());
                }
            }
        }
        let recovery_state = if let Some(disposition) = operation_disposition.as_ref() {
            match disposition.disposition() {
                OperationDisposition::Superseded => CaptureRecoveryState::Superseded,
                OperationDisposition::Abandoned => CaptureRecoveryState::Abandoned,
            }
        } else {
            match latest_resolution.status() {
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
                        } else if let Some(recovery_state) =
                            recovery_state_for_delivery_failure(delivery_failure.as_ref())
                        {
                            recovery_state
                        } else {
                            CaptureRecoveryState::PendingPrimary
                        }
                    } else {
                        CaptureRecoveryState::PendingProject
                    }
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
            operation_disposition: Nullable::present(operation_disposition),
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
        // `operation_disposition` was added after projection v1 shipped.  A
        // missing value is accepted as the legacy spelling of null so the
        // disposable cache is reported stale and can be rebuilt from
        // immutable authority instead of being misclassified as corrupt.
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
        if let Some(disposition) = self.operation_disposition.as_ref() {
            disposition.validate()?;
            let group_has_delivery_authority = self.capture_group.as_ref().is_some_and(|group| {
                group.canonical_record_ref().is_some()
                    || group.applied_reference_count() != 0
                    || group.completion_receipt().is_some()
            });
            if self.delivery_started.as_ref().is_some()
                || self.primary_delivery.as_ref().is_some()
                || self.delivery_failure.as_ref().is_some()
                || group_has_delivery_authority
            {
                return invalid(
                    "capture projection disposition cannot coexist with target-bearing authority",
                );
            }
        }
        let expected_state = if let Some(disposition) = self.operation_disposition.as_ref() {
            match disposition.disposition() {
                OperationDisposition::Superseded => CaptureRecoveryState::Superseded,
                OperationDisposition::Abandoned => CaptureRecoveryState::Abandoned,
            }
        } else {
            match self.latest_resolution.status() {
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
                    } else if let Some(recovery_state) =
                        recovery_state_for_delivery_failure(self.delivery_failure.as_ref())
                    {
                        recovery_state
                    } else {
                        CaptureRecoveryState::PendingPrimary
                    }
                }
                ResolutionStatus::Resolved => CaptureRecoveryState::PendingProject,
            }
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

    pub fn operation_disposition(&self) -> Option<&OperationDispositionRecordedPayload> {
        self.operation_disposition.as_ref()
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

#[derive(Clone, Debug, PartialEq)]
pub struct CaptureAuthorityInspection {
    intent: CaptureIntent,
    events: Vec<CaptureEvent>,
    projection: CaptureProjection,
    stored_state: StoredProjectionState,
    stored_issue: Option<String>,
}

impl CaptureAuthorityInspection {
    pub fn intent(&self) -> &CaptureIntent {
        &self.intent
    }

    pub fn events(&self) -> &[CaptureEvent] {
        &self.events
    }

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
            if !self.payload_kind.is_cognition() {
                return invalid("CaptureGroup is supported only for cognition payloads");
            }
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
        let authority = self.inspect_authority(capture_id)?;
        Ok(CaptureProjectionInspection {
            projection: authority.projection,
            stored_state: authority.stored_state,
            stored_issue: authority.stored_issue,
        })
    }

    pub fn inspect_authority(&self, capture_id: CaptureId) -> Result<CaptureAuthorityInspection> {
        let intent = self.load(capture_id)?;
        let events = self.load_events(capture_id)?;
        let projection = CaptureProjection::from_authority(&intent, &events)?;
        let (stored_state, stored_issue) = self.inspect_stored_projection(&projection)?;
        Ok(CaptureAuthorityInspection {
            intent,
            events,
            projection,
            stored_state,
            stored_issue,
        })
    }

    pub fn inspect_all_authority(&self) -> Result<Vec<CaptureAuthorityInspection>> {
        let mut inspections = Vec::new();
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
            let (stored_state, stored_issue) = self.inspect_stored_projection(&projection)?;
            inspections.push(CaptureAuthorityInspection {
                intent,
                events,
                projection,
                stored_state,
                stored_issue,
            });
        }
        inspections.sort_by(|left, right| {
            left.intent
                .created_at()
                .as_str()
                .cmp(right.intent.created_at().as_str())
                .then_with(|| left.intent.capture_id().cmp(&right.intent.capture_id()))
        });
        Ok(inspections)
    }

    fn inspect_stored_projection(
        &self,
        projection: &CaptureProjection,
    ) -> Result<(StoredProjectionState, Option<String>)> {
        let capture_id = projection.capture_id();
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
        Ok((stored_state, stored_issue))
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
        payload.validate_for_intent(&intent)?;
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

    pub fn preflight_event_payload_size(
        &self,
        capture_id: CaptureId,
        payload: CaptureEventPayload,
    ) -> Result<usize> {
        payload.validate()?;
        self.ensure_layout()?;
        let _lock = JournalQuiescenceLock::acquire(&self.capture_lock_path(capture_id))?;
        let intent = self.load(capture_id)?;
        payload.validate_for_intent(&intent)?;
        let events = self.load_events(capture_id)?;
        let sequence = u64::try_from(events.len())
            .map_err(|_| WorkVcsError::ControlPlaneInvalid("too many capture events".to_owned()))?
            .checked_add(1)
            .ok_or_else(|| {
                WorkVcsError::ControlPlaneInvalid("capture event sequence overflow".to_owned())
            })?;
        let previous_event_digest = events.last().map(CaptureEvent::digest).transpose()?;
        CaptureEvent::worst_case_candidate(
            capture_id,
            sequence,
            EventId::new_v7(),
            previous_event_digest,
            payload,
        )?
        .canonical_size()
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::control_plane::{LocatorAuthority, ResolutionMode, resolve_unbound_project};

    fn typed_intent(payload_kind: CapturePayloadKind) -> CaptureIntent {
        typed_intent_with_payload(payload_kind, serde_json::json!({"schema_version": 1}))
    }

    fn typed_intent_with_payload(
        payload_kind: CapturePayloadKind,
        semantic_payload: serde_json::Value,
    ) -> CaptureIntent {
        let evidence = LocatorEvidence::new(
            LocatorAuthority::SemanticProject,
            "fixture-provider",
            "account-test",
            "project_id",
            "g-p-typed-journal-test",
            LocatorAssurance::Authoritative,
            "typed-journal-test/v1",
            ControlPlaneDigest::raw(b"typed-journal-owner"),
        )
        .expect("semantic locator evidence");
        let context =
            ResolutionContext::new(ResolutionMode::DurableWrite).with_locator_evidence(evidence);
        let resolution =
            resolve_unbound_project(&context, "registry:fixture").expect("fixture resolution");
        CaptureIntent::new(
            CaptureId::new_v7(),
            format!("{}:typed-journal-test", payload_kind.as_str()),
            UtcTimestamp::parse("2026-10-06T00:00:00Z").unwrap(),
            "prove typed journal event validation",
            payload_kind,
            semantic_payload,
            context,
            resolution,
            None,
        )
        .expect("typed intent")
    }

    fn entity_result_object(local_id: &str, object_kind: &str) -> DeliveryResultObject {
        DeliveryResultObject::new(
            local_id,
            object_kind,
            EntityId::new_v7().to_string(),
            EntityVersionId::new_v7().to_string(),
            Digest::domain_separated("workvcs.test.typed-journal.v1", local_id.as_bytes()),
        )
        .expect("typed result object")
    }

    fn delivery_receipt(result_objects: Vec<DeliveryResultObject>) -> DeliveryAppliedPayload {
        DeliveryAppliedPayload::new(
            DeliveryId::new_v7(),
            ProjectRefId::new_v7(),
            StoreId::new_v7(),
            WorkspaceId::new_v7(),
            BranchId::new_v7(),
            CommitId::new_v7(),
            ChangeSetId::new_v7(),
            Digest::domain_separated("workvcs.test.typed-journal.v1", b"state"),
            result_objects,
            false,
            None,
        )
        .expect("delivery receipt")
    }

    fn plan_delivery_receipt(
        payload_kind: CapturePayloadKind,
        result_objects: Vec<DeliveryResultObject>,
    ) -> DeliveryAppliedPayload {
        DeliveryAppliedPayload::new_plan(
            payload_kind,
            DeliveryId::new_v7(),
            ProjectRefId::new_v7(),
            StoreId::new_v7(),
            WorkspaceId::new_v7(),
            BranchId::new_v7(),
            CommitId::new_v7(),
            ChangeSetId::new_v7(),
            Digest::domain_separated("workvcs.test.typed-journal.v1", b"state"),
            result_objects,
            false,
        )
        .expect("Plan delivery receipt")
    }

    fn relation_result_object(local_id: &str, object_kind: &str) -> DeliveryResultObject {
        DeliveryResultObject::new(
            local_id,
            object_kind,
            RelationId::new_v7().to_string(),
            RelationVersionId::new_v7().to_string(),
            Digest::domain_separated("workvcs.test.typed-journal.v1", local_id.as_bytes()),
        )
        .expect("typed relation result object")
    }

    fn delivery_event_with_canonical_size(target_size: usize) -> CaptureEvent {
        let build = |local_id: String| {
            let object = DeliveryResultObject::new(
                local_id,
                "plan",
                EntityId::new_v7().to_string(),
                EntityVersionId::new_v7().to_string(),
                Digest::domain_separated("workvcs.test.event-boundary.v1", b"object"),
            )
            .expect("valid boundary result object");
            let payload = DeliveryAppliedPayload::new(
                DeliveryId::new_v7(),
                ProjectRefId::new_v7(),
                StoreId::new_v7(),
                WorkspaceId::new_v7(),
                BranchId::new_v7(),
                CommitId::new_v7(),
                ChangeSetId::new_v7(),
                Digest::domain_separated("workvcs.test.event-boundary.v1", b"state"),
                vec![object],
                false,
                None,
            )
            .expect("valid boundary delivery payload");
            CaptureEvent::worst_case_candidate(
                CaptureId::new_v7(),
                2,
                EventId::new_v7(),
                Some(ControlPlaneDigest::raw(b"previous event")),
                CaptureEventPayload::DeliveryApplied(payload),
            )
            .expect("valid boundary event candidate")
        };

        let base = build("x".to_owned());
        let base_size = base.canonical_size().expect("base event size");
        assert!(base_size <= target_size);
        let event = build("x".repeat(1 + target_size - base_size));
        assert_eq!(
            event.canonical_size().expect("target event size"),
            target_size
        );
        event
    }

    #[test]
    fn capture_event_limit_accepts_limit_minus_one_and_limit_but_rejects_limit_plus_one() {
        for size in [MAX_CAPTURE_EVENT_BYTES - 1, MAX_CAPTURE_EVENT_BYTES] {
            let event = delivery_event_with_canonical_size(size);
            event.validate_size().expect("event at or below limit");
            assert_eq!(event.occurred_at().as_str(), MAX_CAPTURE_EVENT_TIMESTAMP);
        }
        let event = delivery_event_with_canonical_size(MAX_CAPTURE_EVENT_BYTES + 1);
        let error = event.validate_size().expect_err("event above limit");
        assert!(error.to_string().contains("maximum is 131072"));
    }

    #[test]
    fn operation_disposition_is_terminal_and_idempotent_before_delivery() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let journal = CaptureJournal::for_standalone_root(tempdir.path().join("journal")).unwrap();
        let intent = typed_intent(CapturePayloadKind::CognitionV2);
        journal.admit(&intent).expect("admit intent");
        let payload = CaptureEventPayload::OperationDispositionRecorded(
            OperationDispositionRecordedPayload::new(OperationDisposition::Abandoned, None)
                .unwrap(),
        );

        let first = journal
            .append_event(
                intent.capture_id(),
                UtcTimestamp::parse("2026-10-08T00:00:00Z").unwrap(),
                payload.clone(),
            )
            .expect("record disposition");
        assert_eq!(first.outcome(), CaptureEventAppendOutcome::Created);
        assert_eq!(
            first.projection().recovery_state(),
            CaptureRecoveryState::Abandoned
        );
        assert_eq!(
            first
                .projection()
                .operation_disposition()
                .map(OperationDispositionRecordedPayload::disposition),
            Some(OperationDisposition::Abandoned)
        );

        let replay = journal
            .append_event(
                intent.capture_id(),
                UtcTimestamp::parse("2026-10-08T00:00:01Z").unwrap(),
                payload,
            )
            .expect("replay disposition");
        assert_eq!(replay.outcome(), CaptureEventAppendOutcome::Reused);
        assert_eq!(replay.projection().event_count(), 1);

        let later = CaptureEventPayload::ResolutionRecorded(
            ResolutionRecordedPayload::new(
                RegistryId::new_v7(),
                1,
                ControlPlaneDigest::raw(b"registry"),
                intent.initial_resolution().clone(),
            )
            .unwrap(),
        );
        let error = journal
            .append_event_authority_only(
                intent.capture_id(),
                UtcTimestamp::parse("2026-10-08T00:00:02Z").unwrap(),
                later,
            )
            .expect_err("event after disposition must fail");
        assert!(error.to_string().contains("follows terminal"));
        assert_eq!(journal.load_events(intent.capture_id()).unwrap().len(), 1);
    }

    #[test]
    fn operation_disposition_requires_an_exact_successor_contract() {
        let missing_successor =
            OperationDispositionRecordedPayload::new(OperationDisposition::Superseded, None)
                .expect_err("supersession requires a successor");
        assert!(
            missing_successor
                .to_string()
                .contains("requires a successor CaptureId")
        );

        let unexpected_successor = OperationDispositionRecordedPayload::new(
            OperationDisposition::Abandoned,
            Some(CaptureId::new_v7()),
        )
        .expect_err("abandonment forbids a successor");
        assert!(
            unexpected_successor
                .to_string()
                .contains("forbids a successor CaptureId")
        );

        let intent = typed_intent(CapturePayloadKind::CognitionV2);
        let self_successor = CaptureEventPayload::OperationDispositionRecorded(
            OperationDispositionRecordedPayload::new(
                OperationDisposition::Superseded,
                Some(intent.capture_id()),
            )
            .expect("structurally valid supersession"),
        );
        let self_error = self_successor
            .validate_for_intent(&intent)
            .expect_err("a capture cannot supersede itself");
        assert!(
            self_error
                .to_string()
                .contains("successor must differ from its CaptureId")
        );
    }

    #[test]
    fn legacy_projection_without_operation_disposition_is_stale_not_invalid() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let journal = CaptureJournal::for_standalone_root(tempdir.path().join("journal")).unwrap();
        let intent = typed_intent(CapturePayloadKind::CognitionV2);
        journal.admit(&intent).expect("admit intent");
        journal
            .rebuild_projection(intent.capture_id())
            .expect("write current projection");

        let inspection = journal
            .inspect_projection(intent.capture_id())
            .expect("inspect current projection");
        let mut legacy_value: Value = serde_json::from_slice(
            &inspection
                .projection()
                .canonical_json_bytes()
                .expect("canonical projection"),
        )
        .expect("projection JSON");
        legacy_value
            .as_object_mut()
            .expect("projection object")
            .remove("operation_disposition");
        let mut legacy_bytes = serde_json::to_vec(&legacy_value).expect("legacy projection JSON");
        legacy_bytes.push(b'\n');
        fs::write(journal.projection_path(intent.capture_id()), legacy_bytes)
            .expect("write legacy projection");

        let legacy = journal
            .inspect_projection(intent.capture_id())
            .expect("inspect legacy projection");
        assert_eq!(legacy.stored_state(), StoredProjectionState::Stale);
        assert_eq!(
            legacy.stored_issue(),
            Some("stored projection does not match immutable intent and events")
        );
    }

    #[test]
    fn event_size_preflight_envelope_matches_actual_max_precision_append() {
        assert!(UtcTimestamp::parse("2026-10-06T00:00:00.123456789Z").is_ok());
        let precision_error = UtcTimestamp::parse("2026-10-06T00:00:00.1234567890Z")
            .expect_err("timestamps above nanosecond precision must be rejected");
        assert!(
            precision_error
                .to_string()
                .contains("must not exceed nanosecond precision")
        );

        let tempdir = tempfile::tempdir().expect("tempdir");
        let journal = CaptureJournal::for_standalone_root(tempdir.path().join("journal")).unwrap();
        let intent = typed_intent(CapturePayloadKind::CognitionV2);
        journal.admit(&intent).expect("admit intent");
        let payload = CaptureEventPayload::ResolutionRecorded(
            ResolutionRecordedPayload::new(
                RegistryId::new_v7(),
                1,
                ControlPlaneDigest::raw(b"registry"),
                intent.initial_resolution().clone(),
            )
            .unwrap(),
        );
        let preflight_size = journal
            .preflight_event_payload_size(intent.capture_id(), payload.clone())
            .expect("preflight event size");
        let appended = journal
            .append_event_authority_only(
                intent.capture_id(),
                UtcTimestamp::parse(MAX_CAPTURE_EVENT_TIMESTAMP).unwrap(),
                payload,
            )
            .expect("append at the maximum supported timestamp precision");
        assert_eq!(
            appended.event().canonical_size().unwrap(),
            preflight_size,
            "preflight must use the type-level maximum timestamp envelope"
        );
    }

    #[test]
    fn typed_intents_reject_cross_family_receipts_and_failures() {
        let cognition = typed_intent(CapturePayloadKind::CognitionV2);
        let plan_admit = typed_intent(CapturePayloadKind::PlanAdmitV1);
        let plan_evolve = typed_intent(CapturePayloadKind::PlanEvolveV1);

        let plan_receipt = CaptureEventPayload::DeliveryApplied(plan_delivery_receipt(
            CapturePayloadKind::PlanAdmitV1,
            vec![entity_result_object("plan", "plan")],
        ));
        let cognition_error = plan_receipt
            .validate_for_intent(&cognition)
            .expect_err("cognition intent must reject Plan results");
        assert!(
            cognition_error
                .to_string()
                .contains("cannot carry a Plan operation receipt")
        );

        let cognition_receipt =
            CaptureEventPayload::DeliveryApplied(delivery_receipt(vec![entity_result_object(
                "finding",
                "knowledge",
            )]));
        let plan_error = cognition_receipt
            .validate_for_intent(&plan_admit)
            .expect_err("Plan intent must reject cognition results");
        assert!(
            plan_error
                .to_string()
                .contains("requires operation_payload_kind")
        );

        let admit_relation_receipt = CaptureEventPayload::DeliveryApplied(plan_delivery_receipt(
            CapturePayloadKind::PlanAdmitV1,
            vec![
                entity_result_object("plan", "plan"),
                relation_result_object("goal_plan", "relation:contains"),
            ],
        ));
        assert!(
            admit_relation_receipt
                .validate_for_intent(&plan_admit)
                .expect_err("Plan admission must reject evolution-only relations")
                .to_string()
                .contains("Plan-evolution results")
        );

        let evolve_goal_receipt = CaptureEventPayload::DeliveryApplied(plan_delivery_receipt(
            CapturePayloadKind::PlanEvolveV1,
            vec![
                entity_result_object("plan", "plan"),
                entity_result_object("goal", "goal"),
            ],
        ));
        assert!(
            evolve_goal_receipt
                .validate_for_intent(&plan_evolve)
                .expect_err("Plan evolution must reject Goal creation")
                .to_string()
                .contains("cannot create a Goal result")
        );

        let admit_as_evolve = CaptureEventPayload::DeliveryApplied(plan_delivery_receipt(
            CapturePayloadKind::PlanAdmitV1,
            vec![entity_result_object("plan", "plan")],
        ));
        assert!(
            admit_as_evolve
                .validate_for_intent(&plan_evolve)
                .expect_err("Plan admission receipt must not complete Plan evolution")
                .to_string()
                .contains("requires operation_payload_kind")
        );
        let evolve_as_admit = CaptureEventPayload::DeliveryApplied(plan_delivery_receipt(
            CapturePayloadKind::PlanEvolveV1,
            vec![entity_result_object("plan", "plan")],
        ));
        assert!(
            evolve_as_admit
                .validate_for_intent(&plan_admit)
                .expect_err("Plan evolution receipt must not complete Plan admission")
                .to_string()
                .contains("requires operation_payload_kind")
        );

        let wrong_action = DeliveryFailedPayload::new(
            DeliveryId::new_v7(),
            ProjectRefId::new_v7(),
            StoreId::new_v7(),
            WorkspaceId::new_v7(),
            BranchId::new_v7(),
            DeliveryFailureCode::PlanTargetConflict,
            DeliveryFailureCode::PlanManifestRejected.recovery_action(),
        )
        .expect_err("failure code and recovery action must be an exact pair");
        assert!(
            wrong_action
                .to_string()
                .contains("requires recovery action")
        );

        let plan_failure = CaptureEventPayload::DeliveryFailed(
            DeliveryFailedPayload::new(
                DeliveryId::new_v7(),
                ProjectRefId::new_v7(),
                StoreId::new_v7(),
                WorkspaceId::new_v7(),
                BranchId::new_v7(),
                DeliveryFailureCode::PlanTargetConflict,
                DeliveryFailureCode::PlanTargetConflict.recovery_action(),
            )
            .unwrap(),
        );
        assert!(
            plan_failure
                .validate_for_intent(&cognition)
                .expect_err("cognition intent must reject Plan failure codes")
                .to_string()
                .contains("cannot carry delivery failure")
        );
    }

    #[test]
    fn plan_receipt_shape_exactly_matches_admitted_operation_variant() {
        let head = CommitId::new_v7();
        let state_digest = Digest::domain_separated("workvcs.test.plan-shape.v1", b"state");
        let create_intent = typed_intent_with_payload(
            CapturePayloadKind::PlanAdmitV1,
            serde_json::json!({
                "schema_version": 1,
                "idempotency_key": "shape-admit-create",
                "expected_head_commit_id": head,
                "expected_state_digest": state_digest,
                "goal": {"mode": "create", "description": "Goal"},
                "plan": {"description": "Plan", "strategy": "Strategy", "constraints": []},
                "tasks": [],
                "records": [{
                    "local_id": "question",
                    "kind": "unknown",
                    "statement": "Normalize the historical alias",
                    "scope": {}
                }],
                "evidence": [],
                "rationale": {}
            }),
        );
        let missing_goal = CaptureEventPayload::DeliveryApplied(plan_delivery_receipt(
            CapturePayloadKind::PlanAdmitV1,
            vec![entity_result_object("plan", "plan")],
        ));
        assert!(
            missing_goal
                .validate_for_intent(&create_intent)
                .expect_err("create-goal admission requires the Goal result")
                .to_string()
                .contains("does not exactly match")
        );
        CaptureEventPayload::DeliveryApplied(plan_delivery_receipt(
            CapturePayloadKind::PlanAdmitV1,
            vec![
                entity_result_object("goal", "goal"),
                entity_result_object("plan", "plan"),
                entity_result_object("record.0", "record:question"),
            ],
        ))
        .validate_for_intent(&create_intent)
        .expect("exact create-goal admission result shape");
        let raw_alias_receipt = CaptureEventPayload::DeliveryApplied(plan_delivery_receipt(
            CapturePayloadKind::PlanAdmitV1,
            vec![
                entity_result_object("goal", "goal"),
                entity_result_object("plan", "plan"),
                entity_result_object("record.0", "record:unknown"),
            ],
        ));
        assert!(
            raw_alias_receipt
                .validate_for_intent(&create_intent)
                .expect_err("receipt shape must use the canonical question kind")
                .to_string()
                .contains("does not exactly match")
        );

        let supersede_intent = typed_intent_with_payload(
            CapturePayloadKind::PlanEvolveV1,
            serde_json::json!({
                "mode": "supersede",
                "schema_version": 1,
                "idempotency_key": "shape-evolve-supersede",
                "expected_head_commit_id": head,
                "expected_state_digest": state_digest,
                "target_plan_entity_id": EntityId::new_v7(),
                "expected_plan_entity_version_id": EntityVersionId::new_v7(),
                "expected_plan_state_digest": Digest::domain_separated("workvcs.test.plan-shape.v1", b"plan"),
                "expected_goal_entity_id": EntityId::new_v7(),
                "expected_goal_entity_version_id": EntityVersionId::new_v7(),
                "expected_goal_plan_relation_id": RelationId::new_v7(),
                "expected_goal_plan_relation_version_id": RelationVersionId::new_v7(),
                "plan": {
                    "description": "Replacement",
                    "strategy": "Replacement strategy",
                    "constraints": {"mode": "carry_all"}
                },
                "tasks": [],
                "records": [],
                "evidence": [],
                "rationale": {}
            }),
        );
        let incomplete_supersede = CaptureEventPayload::DeliveryApplied(plan_delivery_receipt(
            CapturePayloadKind::PlanEvolveV1,
            vec![entity_result_object("plan", "plan")],
        ));
        assert!(
            incomplete_supersede
                .validate_for_intent(&supersede_intent)
                .expect_err("supersede requires replacement Plan and both relations")
                .to_string()
                .contains("does not exactly match")
        );
        CaptureEventPayload::DeliveryApplied(plan_delivery_receipt(
            CapturePayloadKind::PlanEvolveV1,
            vec![
                entity_result_object("plan", "plan"),
                entity_result_object("new_plan", "plan"),
                relation_result_object("goal_contains_relation", "relation:contains"),
                relation_result_object("supersedes_relation", "relation:supersedes"),
            ],
        ))
        .validate_for_intent(&supersede_intent)
        .expect("exact supersede result shape");
    }
}
