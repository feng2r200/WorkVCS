use super::journal::{
    CanonicalRecordRef, CaptureAuthorityInspection, CaptureGroupProjection, CaptureIntent,
    CapturePayloadKind, DeliveryAppliedPayload, DeliveryMode, DeliveryResultObject,
    DeliveryStartedPayload,
};
use crate::canonical::{
    CanonicalValue, canonical_bytes, entity_version_digest, relation_version_digest,
};
use crate::error::{Result, WorkVcsError};
use crate::{
    BranchId, ChangeSetId, CognitionCaptureManifest, CognitionCaptureOutcome,
    CognitionCaptureResult, CommitId, DeliveryId, Digest, Engine, EntityId, EntityVersionId,
    EvidenceId, GoalState, GoalStatus, PlanAdmissionGoalManifest, PlanAdmissionManifest,
    PlanAdmissionOutcome, PlanAdmissionResult, PlanEvolutionExpectedParentManifest,
    PlanEvolutionManifest, PlanEvolutionOptions, PlanEvolutionOutcome, PlanEvolutionResult,
    PlanStatus, ProjectRefId, RelationId, RelationVersionId, StoreId, WorkspaceId,
};
use serde_json::{Map, Value};

const TARGET_IDEMPOTENCY_PREFIX: &str = "projectref-primary";
const EVIDENCE_RECEIPT_DIGEST_DOMAIN: &str = "workvcs.delivery-evidence-receipt.v1";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PreparedPrimaryDelivery {
    started: DeliveryStartedPayload,
    operation: PreparedPrimaryOperation,
    legacy_manifest_upgrade_required: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[allow(clippy::large_enum_variant)]
pub enum PreparedPrimaryOperation {
    Cognition(CognitionCaptureManifest),
    PlanAdmission(PlanAdmissionManifest),
    PlanEvolution(PlanEvolutionManifest),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PlanDeliveryPreflight {
    Ready,
    TargetConflict(String),
    ManifestRejected(String),
}

pub fn admitted_legacy_plan_evolution_options(
    authority: &CaptureAuthorityInspection,
    branch_id: BranchId,
    delivered_manifest: PlanEvolutionManifest,
) -> Result<PlanEvolutionOptions> {
    let intent = authority.intent();
    if intent.payload_kind() != CapturePayloadKind::PlanEvolveV1 {
        return Err(WorkVcsError::ControlPlaneInvalid(
            "legacy Plan evolution recovery requires an admitted plan_evolve_v1 intent".to_owned(),
        ));
    }
    let Some(PreparedPrimaryOperation::PlanEvolution(admitted_manifest)) =
        parse_plan_operation(intent)?
    else {
        return Err(WorkVcsError::ControlPlaneInvalid(
            "legacy Plan evolution recovery intent does not carry a Plan evolution manifest"
                .to_owned(),
        ));
    };
    if admitted_manifest != delivered_manifest {
        return Err(WorkVcsError::ControlPlaneInvalid(
            "legacy Plan evolution recovery manifest does not match the admitted immutable intent"
                .to_owned(),
        ));
    }
    if !matches!(
        &admitted_manifest,
        PlanEvolutionManifest::Supersede(manifest) if manifest.schema_version == 1
    ) {
        return Err(WorkVcsError::ControlPlaneInvalid(
            "legacy Plan evolution recovery is limited to admitted schema_version 1 supersede intents"
                .to_owned(),
        ));
    }
    Ok(PlanEvolutionOptions::for_admitted_legacy_recovery(
        branch_id,
        admitted_manifest,
    ))
}

impl PreparedPrimaryDelivery {
    pub fn started(&self) -> &DeliveryStartedPayload {
        &self.started
    }

    pub fn operation(&self) -> &PreparedPrimaryOperation {
        &self.operation
    }

    pub fn into_operation(self) -> PreparedPrimaryOperation {
        self.operation
    }

    pub fn manifest(&self) -> Option<&CognitionCaptureManifest> {
        match &self.operation {
            PreparedPrimaryOperation::Cognition(manifest) => Some(manifest),
            PreparedPrimaryOperation::PlanAdmission(_)
            | PreparedPrimaryOperation::PlanEvolution(_) => None,
        }
    }

    pub fn into_manifest(self) -> Option<CognitionCaptureManifest> {
        match self.operation {
            PreparedPrimaryOperation::Cognition(manifest) => Some(manifest),
            PreparedPrimaryOperation::PlanAdmission(_)
            | PreparedPrimaryOperation::PlanEvolution(_) => None,
        }
    }

    pub fn legacy_manifest_upgrade_required(&self) -> bool {
        self.legacy_manifest_upgrade_required
    }
}

pub fn preflight_plan_delivery(
    engine: &Engine,
    operation: &PreparedPrimaryOperation,
    started: &DeliveryStartedPayload,
) -> Result<PlanDeliveryPreflight> {
    match operation {
        PreparedPrimaryOperation::PlanAdmission(manifest) => {
            if let Err(error) = manifest.validate_for_delivery() {
                return Ok(PlanDeliveryPreflight::ManifestRejected(error.to_string()));
            }
            if engine
                .find_plan_admission_result(crate::PlanAdmissionOptions::new(
                    started.branch_id(),
                    manifest.clone(),
                ))?
                .is_some()
            {
                return Ok(PlanDeliveryPreflight::Ready);
            }
        }
        PreparedPrimaryOperation::PlanEvolution(manifest) => {
            if let Err(error) = manifest.validate_for_delivery() {
                return Ok(PlanDeliveryPreflight::ManifestRejected(error.to_string()));
            }
            if engine
                .find_plan_evolution_result(crate::PlanEvolutionOptions::new(
                    started.branch_id(),
                    manifest.clone(),
                ))?
                .is_some()
            {
                return Ok(PlanDeliveryPreflight::Ready);
            }
        }
        PreparedPrimaryOperation::Cognition(_) => {
            return Err(WorkVcsError::ControlPlaneInvalid(
                "Plan delivery preflight requires a typed Plan operation".to_owned(),
            ));
        }
    }

    let head = engine.branch_head(started.branch_id())?;
    if head.head_commit_id != started.expected_head_commit_id() {
        return Ok(PlanDeliveryPreflight::TargetConflict(format!(
            "branch {} expected head {}, found {}",
            started.branch_id(),
            started.expected_head_commit_id(),
            head.head_commit_id
        )));
    }
    if head.state_digest != started.expected_state_digest() {
        return Ok(PlanDeliveryPreflight::TargetConflict(format!(
            "branch {} expected state digest {}, found {}",
            started.branch_id(),
            started.expected_state_digest(),
            head.state_digest
        )));
    }
    let state = engine.state_at(head.head_commit_id)?;
    if state.workspace_id != started.workspace_id() || state.state_digest != head.state_digest {
        return Err(WorkVcsError::IntegrityInvalid(format!(
            "Plan delivery target branch {} does not match its replayed workspace/state",
            started.branch_id()
        )));
    }

    match operation {
        PreparedPrimaryOperation::PlanAdmission(manifest) => {
            preflight_plan_admission_target(engine, manifest, &state)
        }
        PreparedPrimaryOperation::PlanEvolution(manifest) => {
            preflight_plan_evolution_target(engine, manifest, &state)
        }
        PreparedPrimaryOperation::Cognition(_) => unreachable!("checked above"),
    }
}

fn preflight_plan_admission_target(
    engine: &Engine,
    manifest: &PlanAdmissionManifest,
    state: &crate::ReplayedState,
) -> Result<PlanDeliveryPreflight> {
    let PlanAdmissionGoalManifest::Existing {
        entity_id,
        expected_entity_version_id,
    } = &manifest.goal
    else {
        return Ok(PlanDeliveryPreflight::Ready);
    };
    let goal_id = EntityId::parse_canonical(entity_id)?;
    if !state
        .state
        .entities()
        .iter()
        .any(|(entity_id, _)| *entity_id == goal_id)
    {
        return Ok(PlanDeliveryPreflight::TargetConflict(format!(
            "existing goal {goal_id} is absent from target head {}",
            state.commit_id
        )));
    }
    let goal = match engine.goal_at(state.commit_id, goal_id) {
        Ok(goal) => goal,
        Err(WorkVcsError::GoalNotFound(_)) => {
            return Ok(PlanDeliveryPreflight::TargetConflict(format!(
                "existing goal {goal_id} exists at target head {} but is not a Goal",
                state.commit_id
            )));
        }
        Err(error) => return Err(error),
    };
    if goal.state.status != GoalStatus::Active {
        return Ok(PlanDeliveryPreflight::TargetConflict(format!(
            "existing goal {goal_id} is {}, not active",
            goal.state.status
        )));
    }
    if let Some(expected_entity_version_id) = expected_entity_version_id {
        let expected_entity_version_id =
            EntityVersionId::parse_canonical(expected_entity_version_id)?;
        if goal.goal_entity_version_id != expected_entity_version_id {
            return Ok(PlanDeliveryPreflight::TargetConflict(format!(
                "existing goal {goal_id} expected version {expected_entity_version_id}, found {}",
                goal.goal_entity_version_id
            )));
        }
    }
    Ok(PlanDeliveryPreflight::Ready)
}

fn preflight_plan_evolution_target(
    engine: &Engine,
    manifest: &PlanEvolutionManifest,
    state: &crate::ReplayedState,
) -> Result<PlanDeliveryPreflight> {
    let (
        target_plan_entity_id,
        expected_plan_entity_version_id,
        expected_plan_state_digest,
        retired_target_allowed,
    ) = match manifest {
        PlanEvolutionManifest::InPlace(manifest) => (
            &manifest.target_plan_entity_id,
            &manifest.expected_plan_entity_version_id,
            &manifest.expected_plan_state_digest,
            false,
        ),
        PlanEvolutionManifest::Supersede(manifest) => (
            &manifest.target_plan_entity_id,
            &manifest.expected_plan_entity_version_id,
            &manifest.expected_plan_state_digest,
            false,
        ),
        PlanEvolutionManifest::TaskDisposition(manifest) => (
            &manifest.target_plan_entity_id,
            &manifest.expected_plan_entity_version_id,
            &manifest.expected_plan_state_digest,
            true,
        ),
    };
    let target_plan_entity_id = EntityId::parse_canonical(target_plan_entity_id)?;
    let expected_plan_entity_version_id =
        EntityVersionId::parse_canonical(expected_plan_entity_version_id)?;
    let expected_plan_state_digest = Digest::from_hex(expected_plan_state_digest)?;
    if !state
        .state
        .entities()
        .iter()
        .any(|(entity_id, _)| *entity_id == target_plan_entity_id)
    {
        return Ok(PlanDeliveryPreflight::TargetConflict(format!(
            "target Plan {target_plan_entity_id} is absent from target head {}",
            state.commit_id
        )));
    }
    let plan = match engine.plan_at(state.commit_id, target_plan_entity_id) {
        Ok(plan) => plan,
        Err(WorkVcsError::PlanNotFound(_)) => {
            return Ok(PlanDeliveryPreflight::TargetConflict(format!(
                "target Plan {target_plan_entity_id} exists at target head {} but is not a Plan",
                state.commit_id
            )));
        }
        Err(error) => return Err(error),
    };
    if plan.plan_entity_version_id != expected_plan_entity_version_id {
        return Ok(PlanDeliveryPreflight::TargetConflict(format!(
            "target Plan {target_plan_entity_id} expected version {expected_plan_entity_version_id}, found {}",
            plan.plan_entity_version_id
        )));
    }
    if plan.state_digest != expected_plan_state_digest {
        return Ok(PlanDeliveryPreflight::TargetConflict(format!(
            "target Plan {target_plan_entity_id} expected state digest {expected_plan_state_digest}, found {}",
            plan.state_digest
        )));
    }
    if (!retired_target_allowed && plan.state.status != PlanStatus::Active)
        || (retired_target_allowed
            && !matches!(
                plan.state.status,
                PlanStatus::Superseded | PlanStatus::Abandoned
            ))
    {
        return Ok(PlanDeliveryPreflight::TargetConflict(format!(
            "target Plan {target_plan_entity_id} has incompatible status {}",
            plan.state.status
        )));
    }

    let PlanEvolutionManifest::Supersede(manifest) = manifest else {
        return Ok(PlanDeliveryPreflight::Ready);
    };
    if manifest.plan.strategy == plan.state.strategy {
        return Ok(PlanDeliveryPreflight::ManifestRejected(
            "plan supersede replacement strategy must differ from the prior Plan strategy"
                .to_owned(),
        ));
    }
    let relations = engine.primary_containment_relations_at(state.commit_id)?;
    let mut plan_parents = relations
        .iter()
        .filter(|relation| relation.child_entity_id == target_plan_entity_id);
    let current_relation = plan_parents.next();
    if plan_parents.next().is_some() {
        return Err(WorkVcsError::IntegrityInvalid(format!(
            "target Plan {target_plan_entity_id} has multiple primary containment parents"
        )));
    }
    let (
        expected_parent,
        expected_parent_version_id,
        expected_relation_id,
        expected_relation_version_id,
    ) = if manifest.schema_version == 1 {
        (
            Some((
                "Goal",
                EntityId::parse_canonical(
                    manifest
                        .expected_goal_entity_id
                        .as_deref()
                        .expect("validated schema 1 Goal id"),
                )?,
            )),
            Some(EntityVersionId::parse_canonical(
                manifest
                    .expected_goal_entity_version_id
                    .as_deref()
                    .expect("validated schema 1 Goal version"),
            )?),
            Some(RelationId::parse_canonical(
                manifest
                    .expected_goal_plan_relation_id
                    .as_deref()
                    .expect("validated schema 1 Goal relation"),
            )?),
            Some(RelationVersionId::parse_canonical(
                manifest
                    .expected_goal_plan_relation_version_id
                    .as_deref()
                    .expect("validated schema 1 Goal relation version"),
            )?),
        )
    } else {
        match manifest
            .expected_parent
            .as_ref()
            .expect("validated schema 2 parent guard")
        {
            PlanEvolutionExpectedParentManifest::Goal {
                entity_id,
                expected_entity_version_id,
                containment_relation_id,
                containment_relation_version_id,
            } => (
                Some(("Goal", EntityId::parse_canonical(entity_id)?)),
                Some(EntityVersionId::parse_canonical(
                    expected_entity_version_id,
                )?),
                Some(RelationId::parse_canonical(containment_relation_id)?),
                Some(RelationVersionId::parse_canonical(
                    containment_relation_version_id,
                )?),
            ),
            PlanEvolutionExpectedParentManifest::Plan {
                entity_id,
                expected_entity_version_id,
                containment_relation_id,
                containment_relation_version_id,
            } => (
                Some(("Plan", EntityId::parse_canonical(entity_id)?)),
                Some(EntityVersionId::parse_canonical(
                    expected_entity_version_id,
                )?),
                Some(RelationId::parse_canonical(containment_relation_id)?),
                Some(RelationVersionId::parse_canonical(
                    containment_relation_version_id,
                )?),
            ),
            PlanEvolutionExpectedParentManifest::None => (None, None, None, None),
        }
    };
    let Some((expected_parent_kind, expected_parent_entity_id)) = expected_parent else {
        return Ok(if current_relation.is_none() {
            PlanDeliveryPreflight::Ready
        } else {
            PlanDeliveryPreflight::TargetConflict(format!(
                "standalone target Plan {target_plan_entity_id} gained a primary parent"
            ))
        });
    };
    let Some(current_relation) = current_relation else {
        return Ok(PlanDeliveryPreflight::TargetConflict(format!(
            "target Plan {target_plan_entity_id} has no current primary {expected_parent_kind} relation"
        )));
    };
    if current_relation.parent_entity_id != expected_parent_entity_id
        || Some(current_relation.relation_id) != expected_relation_id
        || Some(current_relation.relation_version_id) != expected_relation_version_id
    {
        return Ok(PlanDeliveryPreflight::TargetConflict(format!(
            "target Plan {target_plan_entity_id} primary {expected_parent_kind} relation no longer matches the manifest guards"
        )));
    }
    let expected_parent_version_id = expected_parent_version_id.expect("guarded parent version");
    match expected_parent_kind {
        "Goal" => {
            let goal = match engine.goal_at(state.commit_id, expected_parent_entity_id) {
                Ok(goal) => goal,
                Err(WorkVcsError::GoalNotFound(_)) => {
                    return Ok(PlanDeliveryPreflight::TargetConflict(format!(
                        "expected Goal {expected_parent_entity_id} is absent or has another kind"
                    )));
                }
                Err(error) => return Err(error),
            };
            if goal.goal_entity_version_id != expected_parent_version_id
                || goal.state.status != GoalStatus::Active
            {
                return Ok(PlanDeliveryPreflight::TargetConflict(format!(
                    "expected Goal {expected_parent_entity_id} version or active status no longer matches"
                )));
            }
        }
        "Plan" => {
            let parent = match engine.plan_at(state.commit_id, expected_parent_entity_id) {
                Ok(plan) => plan,
                Err(WorkVcsError::PlanNotFound(_)) => {
                    return Ok(PlanDeliveryPreflight::TargetConflict(format!(
                        "expected Plan {expected_parent_entity_id} is absent or has another kind"
                    )));
                }
                Err(error) => return Err(error),
            };
            if parent.plan_entity_version_id != expected_parent_version_id
                || parent.state.status != PlanStatus::Active
            {
                return Ok(PlanDeliveryPreflight::TargetConflict(format!(
                    "expected Plan {expected_parent_entity_id} version or active status no longer matches"
                )));
            }
        }
        _ => unreachable!("validated parent kind"),
    }
    Ok(PlanDeliveryPreflight::Ready)
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

    let plan_operation = parse_plan_operation(intent)?;
    let (operation_head, operation_state, operation_idempotency_key) = match plan_operation.as_ref()
    {
        Some(operation) => operation_target(operation)?,
        None => (None, None, String::new()),
    };
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
                operation_head.unwrap_or(current_head_commit_id),
                operation_state.unwrap_or(current_state_digest),
                match intent.payload_kind() {
                    CapturePayloadKind::CognitionV2 => format!(
                        "{TARGET_IDEMPOTENCY_PREFIX}:{}:{project_ref_id}:{store_id}:{workspace_id}:{branch_id}",
                        intent.capture_id(),
                    ),
                    CapturePayloadKind::LegacyCognitionV1 => {
                        legacy_manifest(intent)?.idempotency_key
                    }
                    CapturePayloadKind::PlanAdmitV1
                    | CapturePayloadKind::PlanEvolveV1
                    | CapturePayloadKind::PlanEvolveV2 => operation_idempotency_key.clone(),
                },
            ),
        };

    let operation = match plan_operation {
        None => PreparedPrimaryOperation::Cognition(match intent.payload_kind() {
            CapturePayloadKind::CognitionV2 => cognition_v2_manifest(
                intent,
                &target_idempotency_key,
                expected_head_commit_id,
                expected_state_digest,
            )?,
            CapturePayloadKind::LegacyCognitionV1 => legacy_manifest(intent)?,
            _ => unreachable!("non-plan operation has cognition payload kind"),
        }),
        Some(operation) => operation,
    };
    if let Some(expected_head) = operation_head
        && expected_head != expected_head_commit_id
    {
        return Err(WorkVcsError::ControlPlaneInvalid(
            "plan manifest expected_head_commit_id does not match durable delivery_started"
                .to_owned(),
        ));
    }
    if let Some(expected_state) = operation_state
        && expected_state != expected_state_digest
    {
        return Err(WorkVcsError::ControlPlaneInvalid(
            "plan manifest expected_state_digest does not match durable delivery_started"
                .to_owned(),
        ));
    }
    if !matches!(operation, PreparedPrimaryOperation::Cognition(_))
        && target_idempotency_key != operation_idempotency_key
    {
        return Err(WorkVcsError::ControlPlaneInvalid(
            "plan manifest idempotency key does not match durable delivery_started".to_owned(),
        ));
    }
    if let Some(group) = intent.capture_group()
        && !matches!(
            &operation,
            PreparedPrimaryOperation::Cognition(manifest)
                if manifest.records.iter().any(|record| record.local_id == group.canonical_record_local_id())
        )
    {
        return Err(WorkVcsError::ControlPlaneInvalid(format!(
            "capture group canonical_record_local_id {:?} must name a Record in the target manifest",
            group.canonical_record_local_id()
        )));
    }
    let expected_head_text = expected_head_commit_id.to_string();
    let expected_state_text = expected_state_digest.to_hex();
    let legacy_manifest_upgrade_required = matches!(
        &operation,
        PreparedPrimaryOperation::Cognition(manifest)
            if intent.payload_kind() == CapturePayloadKind::LegacyCognitionV1
                && (manifest.expected_head_commit_id.as_deref()
                    != Some(expected_head_text.as_str())
                    || manifest.expected_state_digest.as_deref()
                        != Some(expected_state_text.as_str()))
    );
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
        operation_digest(&operation)?,
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
        operation,
        legacy_manifest_upgrade_required,
    })
}

fn parse_plan_operation(intent: &CaptureIntent) -> Result<Option<PreparedPrimaryOperation>> {
    let bytes = serde_json::to_vec(intent.semantic_payload()).map_err(|error| {
        WorkVcsError::ControlPlaneInvalid(format!(
            "cannot serialize durable operation payload: {error}"
        ))
    })?;
    match intent.payload_kind() {
        CapturePayloadKind::CognitionV2 | CapturePayloadKind::LegacyCognitionV1 => Ok(None),
        CapturePayloadKind::PlanAdmitV1 => PlanAdmissionManifest::from_json_bytes(&bytes)
            .map(PreparedPrimaryOperation::PlanAdmission)
            .map(Some)
            .map_err(|error| {
                WorkVcsError::ControlPlaneInvalid(format!(
                    "routed plan admission payload is not deliverable: {error}"
                ))
            }),
        CapturePayloadKind::PlanEvolveV1 | CapturePayloadKind::PlanEvolveV2 => {
            let manifest = PlanEvolutionManifest::from_json_bytes(&bytes).map_err(|error| {
                WorkVcsError::ControlPlaneInvalid(format!(
                    "routed plan evolution payload is not deliverable: {error}"
                ))
            })?;
            let expected_kind = if manifest.payload_kind_version() == 2 {
                CapturePayloadKind::PlanEvolveV2
            } else {
                CapturePayloadKind::PlanEvolveV1
            };
            if intent.payload_kind() != expected_kind {
                return Err(WorkVcsError::ControlPlaneInvalid(format!(
                    "{} cannot carry a schema_version {} Plan evolution manifest",
                    intent.payload_kind().as_str(),
                    manifest.schema_version()
                )));
            }
            Ok(Some(PreparedPrimaryOperation::PlanEvolution(manifest)))
        }
    }
}

fn operation_target(
    operation: &PreparedPrimaryOperation,
) -> Result<(Option<CommitId>, Option<Digest>, String)> {
    match operation {
        PreparedPrimaryOperation::Cognition(_) => Ok((None, None, String::new())),
        PreparedPrimaryOperation::PlanAdmission(manifest) => Ok((
            Some(manifest.expected_head_commit_id()?),
            manifest.expected_state_digest()?,
            manifest.idempotency_key().to_owned(),
        )),
        PreparedPrimaryOperation::PlanEvolution(manifest) => Ok((
            Some(manifest.expected_head_commit_id()?),
            manifest.expected_state_digest()?,
            manifest.idempotency_key().to_owned(),
        )),
    }
}

fn operation_digest(operation: &PreparedPrimaryOperation) -> Result<Digest> {
    match operation {
        PreparedPrimaryOperation::Cognition(manifest) => Ok(manifest.payload_digest()),
        PreparedPrimaryOperation::PlanAdmission(manifest) => manifest.payload_digest(),
        PreparedPrimaryOperation::PlanEvolution(manifest) => manifest.payload_digest(),
    }
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
        CapturePayloadKind::PlanAdmitV1
        | CapturePayloadKind::PlanEvolveV1
        | CapturePayloadKind::PlanEvolveV2 => {
            return Err(WorkVcsError::ControlPlaneInvalid(
                "cognition delivery receipt cannot be built for a Plan payload".to_owned(),
            ));
        }
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

pub fn build_plan_admission_delivery_receipt(
    intent: &CaptureIntent,
    started: &DeliveryStartedPayload,
    result: &PlanAdmissionResult,
) -> Result<DeliveryAppliedPayload> {
    validate_operation_result_identity(
        started,
        result.workspace_id,
        result.branch_id,
        &result.idempotency_key,
        result.payload_digest,
    )?;
    let manifest = match parse_plan_operation(intent)? {
        Some(PreparedPrimaryOperation::PlanAdmission(manifest)) => manifest,
        _ => {
            return Err(WorkVcsError::ControlPlaneInvalid(
                "plan admission receipt requires a plan_admit_v1 intent".to_owned(),
            ));
        }
    };
    if manifest.payload_digest()? != started.target_manifest_digest() {
        return Err(WorkVcsError::ControlPlaneInvalid(
            "reconstructed plan admission digest does not match delivery_started".to_owned(),
        ));
    }

    let mut objects = Vec::new();
    if result.goal.created {
        let PlanAdmissionGoalManifest::Create { description } = &manifest.goal else {
            return Err(WorkVcsError::ControlPlaneInvalid(
                "created Plan admission Goal does not match a create-goal manifest".to_owned(),
            ));
        };
        let goal_digest =
            entity_version_digest(&GoalState::active(description.clone())?.to_canonical_value()?)?;
        objects.push(DeliveryResultObject::new(
            "goal",
            "goal",
            result.goal.entity_id.to_string(),
            result.goal.entity_version_id.to_string(),
            goal_digest,
        )?);
    }
    objects.push(DeliveryResultObject::new(
        "plan",
        "plan",
        result.plan.entity_id.to_string(),
        result.plan.entity_version_id.to_string(),
        result.plan.state_digest,
    )?);
    append_plan_task_objects(&mut objects, &result.tasks)?;
    append_plan_record_objects(&mut objects, &result.records)?;
    append_plan_evidence_objects(&mut objects, &result.evidence, &manifest.evidence)?;

    DeliveryAppliedPayload::new_plan(
        CapturePayloadKind::PlanAdmitV1,
        started.delivery_id(),
        started.project_ref_id(),
        started.store_id(),
        result.workspace_id,
        result.branch_id,
        result.commit_id,
        result.changeset_id,
        result.work_state_digest,
        objects,
        result.outcome == PlanAdmissionOutcome::Reused,
    )
}

pub fn build_plan_evolution_delivery_receipt(
    intent: &CaptureIntent,
    started: &DeliveryStartedPayload,
    result: &PlanEvolutionResult,
) -> Result<DeliveryAppliedPayload> {
    validate_operation_result_identity(
        started,
        result.workspace_id,
        result.branch_id,
        &result.idempotency_key,
        result.payload_digest,
    )?;
    let manifest = match parse_plan_operation(intent)? {
        Some(PreparedPrimaryOperation::PlanEvolution(manifest)) => manifest,
        _ => {
            return Err(WorkVcsError::ControlPlaneInvalid(
                "plan evolution receipt requires a typed plan_evolve intent".to_owned(),
            ));
        }
    };
    if manifest.payload_digest()? != started.target_manifest_digest() {
        return Err(WorkVcsError::ControlPlaneInvalid(
            "reconstructed plan evolution digest does not match delivery_started".to_owned(),
        ));
    }

    let mut objects = Vec::new();
    if result.plan_changed {
        objects.push(DeliveryResultObject::new(
            "plan",
            "plan",
            result.plan.entity_id.to_string(),
            result.plan.entity_version_id.to_string(),
            result.plan.state_digest,
        )?);
    }
    if let Some(plan) = &result.new_plan {
        objects.push(DeliveryResultObject::new(
            "new_plan",
            "plan",
            plan.entity_id.to_string(),
            plan.entity_version_id.to_string(),
            plan.state_digest,
        )?);
    }
    append_plan_task_objects(&mut objects, &result.tasks)?;
    append_plan_record_objects(&mut objects, &result.records)?;
    let evidence_manifest: &[crate::PlanAdmissionEvidenceManifest] = match &manifest {
        PlanEvolutionManifest::InPlace(manifest) => manifest.evidence.as_slice(),
        PlanEvolutionManifest::Supersede(manifest) => manifest.evidence.as_slice(),
        PlanEvolutionManifest::TaskDisposition(_) => &[],
    };
    append_plan_evidence_objects(&mut objects, &result.evidence, evidence_manifest)?;
    for (index, disposition) in result.task_dispositions.iter().enumerate() {
        if matches!(
            disposition.action,
            crate::PlanTaskDispositionAction::Supersede | crate::PlanTaskDispositionAction::Cancel
        ) {
            objects.push(DeliveryResultObject::new(
                format!("disposition.{index}.task"),
                "task",
                disposition.task_entity_id.to_string(),
                disposition.entity_version_id.to_string(),
                disposition.state_digest,
            )?);
        }
    }
    if let Some(relation) = &result.goal_contains_relation {
        objects.push(DeliveryResultObject::new(
            "goal_contains_relation",
            "relation:contains",
            relation.relation_id.to_string(),
            relation.relation_version_id.to_string(),
            relation.state_digest,
        )?);
    }
    if let Some(relation) = &result.supersedes_relation {
        objects.push(DeliveryResultObject::new(
            "supersedes_relation",
            "relation:supersedes",
            relation.relation_id.to_string(),
            relation.relation_version_id.to_string(),
            relation.state_digest,
        )?);
    }
    for (index, relation) in result.disposition_relations.iter().enumerate() {
        objects.push(DeliveryResultObject::new(
            format!("disposition_relation.{index}"),
            format!("relation:{}", relation.relation_type),
            relation.relation_id.to_string(),
            relation.relation_version_id.to_string(),
            relation.state_digest,
        )?);
    }

    DeliveryAppliedPayload::new_plan(
        intent.payload_kind(),
        started.delivery_id(),
        started.project_ref_id(),
        started.store_id(),
        result.workspace_id,
        result.branch_id,
        result.commit_id,
        result.changeset_id,
        result.work_state_digest,
        objects,
        result.outcome == PlanEvolutionOutcome::Reused,
    )
}

pub fn build_plan_delivery_receipt_preflight(
    intent: &CaptureIntent,
    started: &DeliveryStartedPayload,
) -> Result<DeliveryAppliedPayload> {
    let digest = started.target_manifest_digest();
    let mut objects = Vec::new();
    match parse_plan_operation(intent)? {
        Some(PreparedPrimaryOperation::PlanAdmission(manifest)) => {
            if matches!(manifest.goal, PlanAdmissionGoalManifest::Create { .. }) {
                append_placeholder_entity(&mut objects, "goal", "goal", digest)?;
            }
            append_placeholder_entity(&mut objects, "plan", "plan", digest)?;
            append_placeholder_manifest_objects(
                &mut objects,
                &manifest.tasks,
                &manifest.records,
                &manifest.evidence,
                digest,
            )?;
        }
        Some(PreparedPrimaryOperation::PlanEvolution(manifest)) => {
            let (tasks, records, evidence, supersede): (
                &[crate::PlanAdmissionTaskManifest],
                &[crate::PlanAdmissionRecordManifest],
                &[crate::PlanAdmissionEvidenceManifest],
                bool,
            ) = match &manifest {
                PlanEvolutionManifest::InPlace(manifest) => (
                    manifest.tasks.as_slice(),
                    manifest.records.as_slice(),
                    manifest.evidence.as_slice(),
                    false,
                ),
                PlanEvolutionManifest::Supersede(manifest) => (
                    manifest.tasks.as_slice(),
                    manifest.records.as_slice(),
                    manifest.evidence.as_slice(),
                    true,
                ),
                PlanEvolutionManifest::TaskDisposition(manifest) => {
                    (manifest.tasks.as_slice(), &[], &[], false)
                }
            };
            if !matches!(manifest, PlanEvolutionManifest::TaskDisposition(_)) {
                append_placeholder_entity(&mut objects, "plan", "plan", digest)?;
            }
            if supersede {
                append_placeholder_entity(&mut objects, "new_plan", "plan", digest)?;
            }
            append_placeholder_manifest_objects(&mut objects, tasks, records, evidence, digest)?;
            if supersede {
                let PlanEvolutionManifest::Supersede(supersede_manifest) = &manifest else {
                    unreachable!("supersede result uses supersede manifest")
                };
                let has_parent_relation = supersede_manifest.schema_version == 1
                    || !matches!(
                        supersede_manifest.expected_parent.as_ref(),
                        Some(PlanEvolutionExpectedParentManifest::None)
                    );
                if has_parent_relation {
                    append_placeholder_relation(
                        &mut objects,
                        "goal_contains_relation",
                        "relation:contains",
                        digest,
                    )?;
                }
                append_placeholder_relation(
                    &mut objects,
                    "supersedes_relation",
                    "relation:supersedes",
                    digest,
                )?;
                if supersede_manifest.schema_version == 2 {
                    let mut relation_index = 0_usize;
                    for (index, disposition) in
                        supersede_manifest.task_dispositions.iter().enumerate()
                    {
                        if matches!(
                            disposition.action,
                            crate::PlanTaskDispositionAction::Supersede
                                | crate::PlanTaskDispositionAction::Cancel
                        ) {
                            append_placeholder_entity(
                                &mut objects,
                                format!("disposition.{index}.task"),
                                "task",
                                digest,
                            )?;
                        }
                        let relation_kind = match disposition.action {
                            crate::PlanTaskDispositionAction::Supersede => {
                                Some("relation:supersedes")
                            }
                            crate::PlanTaskDispositionAction::Continue => {
                                Some("relation:references")
                            }
                            crate::PlanTaskDispositionAction::Cancel
                            | crate::PlanTaskDispositionAction::RetainOriginal => None,
                        };
                        if let Some(relation_kind) = relation_kind {
                            append_placeholder_relation(
                                &mut objects,
                                format!("disposition_relation.{relation_index}"),
                                relation_kind,
                                digest,
                            )?;
                            relation_index += 1;
                        }
                    }
                    for _ in &supersede_manifest.nested_plan_continuations {
                        append_placeholder_relation(
                            &mut objects,
                            format!("disposition_relation.{relation_index}"),
                            "relation:references",
                            digest,
                        )?;
                        relation_index += 1;
                    }
                }
            }
            if let PlanEvolutionManifest::TaskDisposition(repair) = &manifest {
                let mut relation_index = 0_usize;
                for (index, disposition) in repair.task_dispositions.iter().enumerate() {
                    if matches!(
                        disposition.action,
                        crate::PlanTaskDispositionAction::Supersede
                            | crate::PlanTaskDispositionAction::Cancel
                    ) {
                        append_placeholder_entity(
                            &mut objects,
                            format!("disposition.{index}.task"),
                            "task",
                            digest,
                        )?;
                    }
                    let relation_kind = match disposition.action {
                        crate::PlanTaskDispositionAction::Supersede => Some("relation:supersedes"),
                        crate::PlanTaskDispositionAction::Continue => Some("relation:references"),
                        crate::PlanTaskDispositionAction::Cancel
                        | crate::PlanTaskDispositionAction::RetainOriginal => None,
                    };
                    if let Some(relation_kind) = relation_kind {
                        append_placeholder_relation(
                            &mut objects,
                            format!("disposition_relation.{relation_index}"),
                            relation_kind,
                            digest,
                        )?;
                        relation_index += 1;
                    }
                }
            }
        }
        _ => {
            return Err(WorkVcsError::ControlPlaneInvalid(
                "Plan receipt preflight requires a typed Plan intent".to_owned(),
            ));
        }
    }

    DeliveryAppliedPayload::new_plan(
        intent.payload_kind(),
        started.delivery_id(),
        started.project_ref_id(),
        started.store_id(),
        started.workspace_id(),
        started.branch_id(),
        CommitId::new_v7(),
        ChangeSetId::new_v7(),
        digest,
        objects,
        false,
    )
}

fn append_placeholder_manifest_objects(
    objects: &mut Vec<DeliveryResultObject>,
    tasks: &[crate::PlanAdmissionTaskManifest],
    records: &[crate::PlanAdmissionRecordManifest],
    evidence: &[crate::PlanAdmissionEvidenceManifest],
    digest: Digest,
) -> Result<()> {
    for (task_index, task) in tasks.iter().enumerate() {
        append_placeholder_entity(objects, format!("task.{task_index}"), "task", digest)?;
        for (criterion_index, criterion) in task.acceptance_criteria.iter().enumerate() {
            append_placeholder_entity(
                objects,
                format!("task.{task_index}.acceptance_criterion.{criterion_index}"),
                "acceptance_criterion",
                digest,
            )?;
            for requirement_index in 0..criterion.verification_requirements.len() {
                append_placeholder_entity(
                    objects,
                    format!(
                        "task.{task_index}.acceptance_criterion.{criterion_index}.verification_requirement.{requirement_index}"
                    ),
                    "verification_requirement",
                    digest,
                )?;
            }
        }
    }
    for (index, record) in records.iter().enumerate() {
        append_placeholder_entity(
            objects,
            format!("record.{index}"),
            format!("record:{}", record.canonical_kind()?),
            digest,
        )?;
    }
    for index in 0..evidence.len() {
        let evidence_id = EvidenceId::new_v7();
        objects.push(DeliveryResultObject::new(
            format!("evidence.{index}"),
            "evidence",
            evidence_id.to_string(),
            evidence_id.to_string(),
            digest,
        )?);
    }
    Ok(())
}

fn append_placeholder_entity(
    objects: &mut Vec<DeliveryResultObject>,
    local_id: impl Into<String>,
    object_kind: impl Into<String>,
    digest: Digest,
) -> Result<()> {
    objects.push(DeliveryResultObject::new(
        local_id,
        object_kind,
        EntityId::new_v7().to_string(),
        EntityVersionId::new_v7().to_string(),
        digest,
    )?);
    Ok(())
}

fn append_placeholder_relation(
    objects: &mut Vec<DeliveryResultObject>,
    local_id: impl Into<String>,
    object_kind: impl Into<String>,
    digest: Digest,
) -> Result<()> {
    objects.push(DeliveryResultObject::new(
        local_id,
        object_kind,
        RelationId::new_v7().to_string(),
        RelationVersionId::new_v7().to_string(),
        digest,
    )?);
    Ok(())
}

fn validate_operation_result_identity(
    started: &DeliveryStartedPayload,
    workspace_id: WorkspaceId,
    branch_id: BranchId,
    idempotency_key: &str,
    payload_digest: Digest,
) -> Result<()> {
    if workspace_id != started.workspace_id()
        || branch_id != started.branch_id()
        || idempotency_key != started.target_idempotency_key()
        || payload_digest != started.target_manifest_digest()
    {
        return Err(WorkVcsError::ControlPlaneInvalid(
            "target Plan result does not match the durable delivery_started identity".to_owned(),
        ));
    }
    Ok(())
}

fn append_plan_task_objects(
    objects: &mut Vec<DeliveryResultObject>,
    tasks: &[crate::PlanAdmissionTaskResult],
) -> Result<()> {
    for (task_index, task) in tasks.iter().enumerate() {
        objects.push(DeliveryResultObject::new(
            format!("task.{task_index}"),
            "task",
            task.entity_id.to_string(),
            task.entity_version_id.to_string(),
            task.state_digest,
        )?);
        for (criterion_index, criterion) in task.acceptance_criteria.iter().enumerate() {
            objects.push(DeliveryResultObject::new(
                format!("task.{task_index}.acceptance_criterion.{criterion_index}"),
                "acceptance_criterion",
                criterion.entity_id.to_string(),
                criterion.entity_version_id.to_string(),
                criterion.state_digest,
            )?);
            for (requirement_index, requirement) in
                criterion.verification_requirements.iter().enumerate()
            {
                objects.push(DeliveryResultObject::new(
                    format!(
                        "task.{task_index}.acceptance_criterion.{criterion_index}.verification_requirement.{requirement_index}"
                    ),
                    "verification_requirement",
                    requirement.entity_id.to_string(),
                    requirement.entity_version_id.to_string(),
                    requirement.state_digest,
                )?);
            }
        }
    }
    Ok(())
}

fn append_plan_record_objects(
    objects: &mut Vec<DeliveryResultObject>,
    records: &[crate::PlanAdmissionRecordResult],
) -> Result<()> {
    for (index, record) in records.iter().enumerate() {
        objects.push(DeliveryResultObject::new(
            format!("record.{index}"),
            format!("record:{}", record.kind),
            record.entity_id.to_string(),
            record.entity_version_id.to_string(),
            record.state_digest,
        )?);
    }
    Ok(())
}

fn append_plan_evidence_objects(
    objects: &mut Vec<DeliveryResultObject>,
    evidence: &[crate::PlanAdmissionEvidenceResult],
    manifest: &[crate::PlanAdmissionEvidenceManifest],
) -> Result<()> {
    for (index, item) in evidence.iter().enumerate() {
        let manifest_item = manifest
            .iter()
            .find(|candidate| candidate.local_id == item.local_id)
            .ok_or_else(|| {
                WorkVcsError::ControlPlaneInvalid(format!(
                    "delivered Plan Evidence local_id {:?} is absent from the target manifest",
                    item.local_id
                ))
            })?;
        let digest = evidence_receipt_digest(&manifest_item.kind, &manifest_item.metadata)?;
        objects.push(DeliveryResultObject::new(
            format!("evidence.{index}"),
            "evidence",
            item.evidence_id.to_string(),
            item.evidence_id.to_string(),
            digest,
        )?);
    }
    Ok(())
}

fn evidence_receipt_digest(kind: &str, metadata: &CanonicalValue) -> Result<Digest> {
    let canonical = CanonicalValue::object(vec![
        ("kind".to_owned(), CanonicalValue::String(kind.to_owned())),
        ("metadata".to_owned(), metadata.clone()),
    ])?;
    Ok(Digest::domain_separated(
        EVIDENCE_RECEIPT_DIGEST_DOMAIN,
        &canonical_bytes(&canonical).map_err(|error| {
            WorkVcsError::ControlPlaneInvalid(format!(
                "cannot encode delivered Evidence receipt digest: {error}"
            ))
        })?,
    ))
}
