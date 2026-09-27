use super::journal::{
    CanonicalRecordRef, CaptureGroupProjection, CaptureIntent, CapturePayloadKind,
    DeliveryAppliedPayload, DeliveryMode, DeliveryResultObject, DeliveryStartedPayload,
};
use crate::canonical::{CanonicalValue, canonical_bytes, relation_version_digest};
use crate::error::{Result, WorkVcsError};
use crate::{
    BranchId, CognitionCaptureManifest, CognitionCaptureOutcome, CognitionCaptureResult, CommitId,
    DeliveryId, Digest, ProjectRefId, StoreId, WorkspaceId,
};
use serde_json::{Map, Value};

const TARGET_IDEMPOTENCY_PREFIX: &str = "projectref-primary";
const EVIDENCE_RECEIPT_DIGEST_DOMAIN: &str = "workvcs.delivery-evidence-receipt.v1";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PreparedPrimaryDelivery {
    started: DeliveryStartedPayload,
    manifest: CognitionCaptureManifest,
    legacy_manifest_upgrade_required: bool,
}

impl PreparedPrimaryDelivery {
    pub fn started(&self) -> &DeliveryStartedPayload {
        &self.started
    }

    pub fn manifest(&self) -> &CognitionCaptureManifest {
        &self.manifest
    }

    pub fn into_manifest(self) -> CognitionCaptureManifest {
        self.manifest
    }

    pub fn legacy_manifest_upgrade_required(&self) -> bool {
        self.legacy_manifest_upgrade_required
    }
}

#[allow(clippy::too_many_arguments)]
pub fn prepare_primary_delivery(
    intent: &CaptureIntent,
    project_ref_id: ProjectRefId,
    store_id: StoreId,
    workspace_id: WorkspaceId,
    branch_id: BranchId,
    current_head_commit_id: CommitId,
    current_state_digest: Digest,
    existing_started: Option<&DeliveryStartedPayload>,
) -> Result<PreparedPrimaryDelivery> {
    prepare_primary_delivery_inner(
        intent,
        intent
            .capture_group()
            .map(|group| group.primary_project_ref()),
        false,
        project_ref_id,
        store_id,
        workspace_id,
        branch_id,
        current_head_commit_id,
        current_state_digest,
        existing_started,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn prepare_primary_delivery_from_projection(
    intent: &CaptureIntent,
    capture_group: Option<&CaptureGroupProjection>,
    project_ref_id: ProjectRefId,
    store_id: StoreId,
    workspace_id: WorkspaceId,
    branch_id: BranchId,
    current_head_commit_id: CommitId,
    current_state_digest: Digest,
    existing_started: Option<&DeliveryStartedPayload>,
) -> Result<PreparedPrimaryDelivery> {
    if intent.capture_group().is_some() != capture_group.is_some() {
        return Err(WorkVcsError::ControlPlaneInvalid(
            "capture projection and intent disagree about CaptureGroup presence".to_owned(),
        ));
    }
    if let Some(group) = capture_group
        && (group.capture_id() != intent.capture_id()
            || intent.capture_group().is_some_and(|intent_group| {
                intent_group.capture_group_id() != group.capture_group_id()
            }))
    {
        return Err(WorkVcsError::ControlPlaneInvalid(
            "capture projection CaptureGroup identity does not match the intent".to_owned(),
        ));
    }
    prepare_primary_delivery_inner(
        intent,
        capture_group.map(|group| group.resolved_primary_project_ref()),
        capture_group.is_some_and(|group| group.canonical_record_ref().is_some()),
        project_ref_id,
        store_id,
        workspace_id,
        branch_id,
        current_head_commit_id,
        current_state_digest,
        existing_started,
    )
}

#[allow(clippy::too_many_arguments)]
fn prepare_primary_delivery_inner(
    intent: &CaptureIntent,
    effective_group_primary: Option<Option<ProjectRefId>>,
    canonical_already_delivered: bool,
    project_ref_id: ProjectRefId,
    store_id: StoreId,
    workspace_id: WorkspaceId,
    branch_id: BranchId,
    current_head_commit_id: CommitId,
    current_state_digest: Digest,
    existing_started: Option<&DeliveryStartedPayload>,
) -> Result<PreparedPrimaryDelivery> {
    if effective_group_primary.is_some_and(|primary| primary != Some(project_ref_id)) {
        return Err(WorkVcsError::ControlPlaneInvalid(
            "capture group primary is unresolved or does not match the delivery target; capture_group_resolved is required before primary delivery"
                .to_owned(),
        ));
    }
    if canonical_already_delivered {
        return Err(WorkVcsError::ControlPlaneInvalid(
            "capture group already has a canonical Record reference; a second primary delivery is forbidden"
                .to_owned(),
        ));
    }

    let (delivery_id, expected_head_commit_id, expected_state_digest, target_idempotency_key) =
        match existing_started {
            Some(started) => {
                if started.delivery_mode() != DeliveryMode::Canonical
                    || started.project_ref_id() != project_ref_id
                    || started.store_id() != store_id
                    || started.workspace_id() != workspace_id
                    || started.branch_id() != branch_id
                {
                    return Err(WorkVcsError::ControlPlaneInvalid(
                        "existing delivery_started target does not match the current primary binding"
                            .to_owned(),
                    ));
                }
                (
                    started.delivery_id(),
                    started.expected_head_commit_id(),
                    started.expected_state_digest(),
                    started.target_idempotency_key().to_owned(),
                )
            }
            None => (
                DeliveryId::new_v7(),
                current_head_commit_id,
                current_state_digest,
                match intent.payload_kind() {
                    CapturePayloadKind::CognitionV2 => format!(
                        "{TARGET_IDEMPOTENCY_PREFIX}:{}:{project_ref_id}:{store_id}:{workspace_id}:{branch_id}",
                        intent.capture_id(),
                    ),
                    CapturePayloadKind::LegacyCognitionV1 => {
                        legacy_manifest(intent)?.idempotency_key
                    }
                },
            ),
        };

    let manifest = match intent.payload_kind() {
        CapturePayloadKind::CognitionV2 => cognition_v2_manifest(
            intent,
            &target_idempotency_key,
            expected_head_commit_id,
            expected_state_digest,
        )?,
        CapturePayloadKind::LegacyCognitionV1 => legacy_manifest(intent)?,
    };
    if let Some(group) = intent.capture_group()
        && !manifest
            .records
            .iter()
            .any(|record| record.local_id == group.canonical_record_local_id())
    {
        return Err(WorkVcsError::ControlPlaneInvalid(format!(
            "capture group canonical_record_local_id {:?} must name a Record in the target manifest",
            group.canonical_record_local_id()
        )));
    }
    let expected_head_text = expected_head_commit_id.to_string();
    let expected_state_text = expected_state_digest.to_hex();
    let legacy_manifest_upgrade_required = intent.payload_kind()
        == CapturePayloadKind::LegacyCognitionV1
        && (manifest.expected_head_commit_id.as_deref() != Some(expected_head_text.as_str())
            || manifest.expected_state_digest.as_deref() != Some(expected_state_text.as_str()));
    let started = DeliveryStartedPayload::new(
        delivery_id,
        DeliveryMode::Canonical,
        project_ref_id,
        store_id,
        workspace_id,
        branch_id,
        expected_head_commit_id,
        expected_state_digest,
        target_idempotency_key,
        manifest.payload_digest(),
    )?;
    if let Some(existing) = existing_started
        && existing != &started
    {
        return Err(WorkVcsError::ControlPlaneInvalid(
            "reconstructed primary delivery does not match the durable delivery_started event"
                .to_owned(),
        ));
    }
    Ok(PreparedPrimaryDelivery {
        started,
        manifest,
        legacy_manifest_upgrade_required,
    })
}

fn cognition_v2_manifest(
    intent: &CaptureIntent,
    target_idempotency_key: &str,
    expected_head_commit_id: CommitId,
    expected_state_digest: Digest,
) -> Result<CognitionCaptureManifest> {
    let mut object = intent
        .semantic_payload()
        .as_object()
        .cloned()
        .ok_or_else(|| {
            WorkVcsError::ControlPlaneInvalid(
                "cognition_v2 semantic payload must be an object".to_owned(),
            )
        })?;
    for reserved in [
        "schema_version",
        "idempotency_key",
        "expected_head_commit_id",
        "expected_state_digest",
    ] {
        if object.contains_key(reserved) {
            return Err(WorkVcsError::ControlPlaneInvalid(format!(
                "cognition_v2 semantic payload must not contain target field {reserved}"
            )));
        }
    }
    object.insert("schema_version".to_owned(), Value::from(1));
    object.insert(
        "idempotency_key".to_owned(),
        Value::String(target_idempotency_key.to_owned()),
    );
    object.insert(
        "expected_head_commit_id".to_owned(),
        Value::String(expected_head_commit_id.to_string()),
    );
    object.insert(
        "expected_state_digest".to_owned(),
        Value::String(expected_state_digest.to_hex()),
    );
    object
        .entry("rationale".to_owned())
        .or_insert_with(|| Value::Object(Map::new()));
    parse_manifest(Value::Object(object))
}

fn legacy_manifest(intent: &CaptureIntent) -> Result<CognitionCaptureManifest> {
    parse_manifest(intent.semantic_payload().clone())
}

fn parse_manifest(value: Value) -> Result<CognitionCaptureManifest> {
    let bytes = serde_json::to_vec(&value).map_err(|error| {
        WorkVcsError::ControlPlaneInvalid(format!(
            "cannot serialize routed cognition manifest: {error}"
        ))
    })?;
    CognitionCaptureManifest::from_json_bytes(&bytes).map_err(|error| {
        WorkVcsError::ControlPlaneInvalid(format!(
            "routed cognition payload is not deliverable: {error}"
        ))
    })
}

pub fn build_primary_delivery_receipt(
    intent: &CaptureIntent,
    started: &DeliveryStartedPayload,
    result: &CognitionCaptureResult,
) -> Result<DeliveryAppliedPayload> {
    if result.workspace_id != started.workspace_id()
        || result.branch_id != started.branch_id()
        || result.idempotency_key != started.target_idempotency_key()
        || result.payload_digest != started.target_manifest_digest()
    {
        return Err(WorkVcsError::ControlPlaneInvalid(
            "target capture result does not match the durable delivery_started identity".to_owned(),
        ));
    }
    let target_manifest = match intent.payload_kind() {
        CapturePayloadKind::CognitionV2 => cognition_v2_manifest(
            intent,
            started.target_idempotency_key(),
            started.expected_head_commit_id(),
            started.expected_state_digest(),
        )?,
        CapturePayloadKind::LegacyCognitionV1 => legacy_manifest(intent)?,
    };
    if target_manifest.payload_digest() != started.target_manifest_digest() {
        return Err(WorkVcsError::ControlPlaneInvalid(
            "reconstructed target manifest digest does not match delivery_started".to_owned(),
        ));
    }

    let mut objects = Vec::new();
    for item in &result.records {
        objects.push(DeliveryResultObject::new(
            item.local_id.clone(),
            format!("record:{}", item.kind),
            item.entity_id.to_string(),
            item.entity_version_id.to_string(),
            item.state_digest,
        )?);
    }
    for item in &result.knowledge {
        objects.push(DeliveryResultObject::new(
            item.local_id.clone(),
            "knowledge",
            item.entity_id.to_string(),
            item.entity_version_id.to_string(),
            item.state_digest,
        )?);
    }
    for item in &result.evidence {
        let manifest_item = target_manifest
            .evidence
            .iter()
            .find(|candidate| candidate.local_id == item.local_id)
            .ok_or_else(|| {
                WorkVcsError::ControlPlaneInvalid(format!(
                    "delivered Evidence local_id {:?} is absent from the target manifest",
                    item.local_id
                ))
            })?;
        let canonical = CanonicalValue::object(vec![
            (
                "kind".to_owned(),
                CanonicalValue::String(manifest_item.kind.clone()),
            ),
            ("metadata".to_owned(), manifest_item.metadata.clone()),
        ])?;
        let digest = Digest::domain_separated(
            EVIDENCE_RECEIPT_DIGEST_DOMAIN,
            &canonical_bytes(&canonical).map_err(|error| {
                WorkVcsError::ControlPlaneInvalid(format!(
                    "cannot encode delivered Evidence receipt digest: {error}"
                ))
            })?,
        );
        objects.push(DeliveryResultObject::new(
            item.local_id.clone(),
            "evidence",
            item.evidence_id.to_string(),
            item.evidence_id.to_string(),
            digest,
        )?);
    }
    let relation_digest = relation_version_digest(&CanonicalValue::object(Vec::new())?)?;
    for item in &result.relations {
        objects.push(DeliveryResultObject::new(
            item.local_id.clone(),
            format!("relation:{}", item.relation_type),
            item.relation_id.to_string(),
            item.relation_version_id.to_string(),
            relation_digest,
        )?);
    }

    let canonical_record_ref = intent
        .capture_group()
        .map(|group| {
            let record = result
                .records
                .iter()
                .find(|item| item.local_id == group.canonical_record_local_id())
                .ok_or_else(|| {
                    WorkVcsError::ControlPlaneInvalid(format!(
                        "capture group canonical_record_local_id {:?} is not a delivered Record",
                        group.canonical_record_local_id()
                    ))
                })?;
            CanonicalRecordRef::new(
                started.project_ref_id(),
                started.store_id(),
                started.workspace_id(),
                record.entity_id.to_string(),
                record.entity_version_id.to_string(),
                record.state_digest,
            )
        })
        .transpose()?;

    DeliveryAppliedPayload::new(
        started.delivery_id(),
        started.project_ref_id(),
        started.store_id(),
        result.workspace_id,
        result.branch_id,
        result.commit_id,
        result.changeset_id,
        result.work_state_digest,
        objects,
        result.outcome == CognitionCaptureOutcome::Reused,
        canonical_record_ref,
    )
}
