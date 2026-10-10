use super::containment::{PrimaryContainmentEndpointKind, primary_containment_relations_at};
use super::currentness::{
    CurrentScopeOverrides, CurrentnessCarrierKind, CurrentnessEdge, CurrentnessNodeStatus,
    current_scope_at, current_scope_at_with_overrides,
};
use super::goal::{GOAL_ENTITY_KIND, GoalStatus, goal_at};
use super::plan::{PLAN_ENTITY_KIND, PlanState, PlanStatus, plan_at};
use super::record::{RECORD_ENTITY_KIND, RecordKind, RecordState};
use super::reference::structural_references_at;
use super::state_at;
use super::task::{
    ACCEPTANCE_CRITERION_ENTITY_KIND, AcceptanceCriterionClassification, AcceptanceCriterionState,
    AcceptanceCriterionVerificationRequirementRef, TASK_ENTITY_KIND, TaskAcceptanceCriterionRef,
    TaskState, TaskStatus, VERIFICATION_REQUIREMENT_ENTITY_KIND, VerificationRequirementState,
    task_at, task_scheduling_relations_at,
};
use crate::canonical::{
    CanonicalValue, WorkState, canonical_bytes, entity_version_digest, parse_canonical_json,
    relation_version_digest, work_state_mapping_digest,
};
use crate::error::{Result, WorkVcsError, storage_error};
use crate::identity::{
    BranchId, ChangeSetId, CommitId, Digest, EntityId, EntityVersionId, EventId, EvidenceId,
    OperationId, RelationId, RelationVersionId, WorkspaceId,
};
use crate::store::{StoreConnection, current_epoch_micros};
use rusqlite::{Connection, OptionalExtension, Transaction, TransactionBehavior, params};
use serde::{Deserialize, Deserializer, de};
use std::collections::{BTreeMap, BTreeSet};

const ACTIVE_BRANCH_LIFECYCLE_STATE: &str = "active";
const ADMISSION_EVENT_KIND: &str = "plan.admitted";
pub(crate) const ADMISSION_OPERATION_SCHEMA_VERSION: i64 = 1;
pub(crate) const ADMISSION_OPERATION_TYPE: &str = "plan.admit";
const CONTAINS_RELATION_TYPE: &str = "contains";
const EMPTY_FIELD_DELTA: &str = "{}";
const ENTITY_OBJECT_KIND: &str = "entity";
const EVOLUTION_EVENT_KIND: &str = "plan.evolved";
pub(crate) const EVOLUTION_OPERATION_SCHEMA_VERSION: i64 = 1;
pub(crate) const EVOLUTION_OPERATION_SCHEMA_VERSION_V2: i64 = 2;
pub(crate) const EVOLUTION_OPERATION_TYPE: &str = "plan.evolve";
const EVIDENCE_OBJECT_KIND: &str = "evidence";
const NORMAL_COMMIT_KIND: &str = "normal";
const PLAN_SUPERSEDES_RELATION_DISCRIMINATOR: &str = "plan";
const PLAN_SUPERSEDES_RELATION_TYPE: &str = "supersedes";
const PRIMARY_CONTAINMENT_DISCRIMINATOR: &str = "primary";
const PRIMARY_PARENT_ROLE: &str = "primary";
const RELATION_OBJECT_KIND: &str = "relation";
const RELATION_STATE_SCHEMA_VERSION: i64 = 1;
const STATE_SCHEMA_VERSION: i64 = 1;
const PAYLOAD_DIGEST_DOMAIN: &str = "workvcs.plan-admit-manifest.v1";
const EVOLUTION_PAYLOAD_DIGEST_DOMAIN: &str = "workvcs.plan-evolve-manifest.v1";
const EVOLUTION_V2_PAYLOAD_DIGEST_DOMAIN: &str = "workvcs.plan-evolve-manifest.v2";
const REFERENCES_RELATION_TYPE: &str = "references";
const TASK_SUPERSEDES_RELATION_DISCRIMINATOR: &str = "task";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlanAdmissionOptions {
    branch_id: BranchId,
    manifest: PlanAdmissionManifest,
}

impl PlanAdmissionOptions {
    pub fn new(branch_id: BranchId, manifest: PlanAdmissionManifest) -> Self {
        Self {
            branch_id,
            manifest,
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PlanAdmissionManifest {
    pub schema_version: i64,
    pub idempotency_key: String,
    pub expected_head_commit_id: String,
    #[serde(default)]
    pub expected_state_digest: Option<String>,
    pub goal: PlanAdmissionGoalManifest,
    pub plan: PlanAdmissionPlanManifest,
    #[serde(default)]
    pub tasks: Vec<PlanAdmissionTaskManifest>,
    #[serde(default)]
    pub records: Vec<PlanAdmissionRecordManifest>,
    #[serde(default)]
    pub evidence: Vec<PlanAdmissionEvidenceManifest>,
    #[serde(default = "empty_object")]
    pub rationale: CanonicalValue,
}

impl PlanAdmissionManifest {
    pub fn from_json_bytes(bytes: &[u8]) -> Result<Self> {
        let value = parse_canonical_json(bytes).map_err(|error| {
            WorkVcsError::PlanInvalid(format!(
                "plan admission manifest is not valid JSON: {error}"
            ))
        })?;
        let canonical = canonical_bytes(&value).map_err(plan_invalid_from)?;
        serde_json::from_slice(&canonical).map_err(|error| {
            WorkVcsError::PlanInvalid(format!(
                "plan admission manifest has invalid shape: {error}"
            ))
        })
    }

    pub fn idempotency_key(&self) -> &str {
        &self.idempotency_key
    }

    pub fn expected_head_commit_id(&self) -> Result<CommitId> {
        CommitId::parse_canonical(&self.expected_head_commit_id)
    }

    pub fn expected_state_digest(&self) -> Result<Option<Digest>> {
        self.expected_state_digest
            .as_deref()
            .map(Digest::from_hex)
            .transpose()
    }

    pub fn payload_digest(&self) -> Result<Digest> {
        let value = manifest_to_canonical_value(self)?;
        Ok(Digest::domain_separated(
            PAYLOAD_DIGEST_DOMAIN,
            &canonical_bytes(&value)?,
        ))
    }

    pub fn validate_for_delivery(&self) -> Result<()> {
        validate_plan_admission_manifest_for_delivery(self)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlanEvolutionOptions {
    branch_id: BranchId,
    manifest: PlanEvolutionManifest,
    allow_legacy_supersede_recovery: bool,
}

impl PlanEvolutionOptions {
    pub fn new(branch_id: BranchId, manifest: PlanEvolutionManifest) -> Self {
        Self {
            branch_id,
            manifest,
            allow_legacy_supersede_recovery: false,
        }
    }

    pub(crate) fn for_admitted_legacy_recovery(
        branch_id: BranchId,
        manifest: PlanEvolutionManifest,
    ) -> Self {
        Self {
            branch_id,
            manifest,
            allow_legacy_supersede_recovery: true,
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(tag = "mode", rename_all = "snake_case", deny_unknown_fields)]
#[allow(clippy::large_enum_variant)]
pub enum PlanEvolutionManifest {
    InPlace(PlanEvolutionInPlaceManifest),
    Supersede(PlanEvolutionSupersedeManifest),
    TaskDisposition(PlanTaskDispositionRepairManifest),
}

impl PlanEvolutionManifest {
    pub fn from_json_bytes(bytes: &[u8]) -> Result<Self> {
        let value = parse_canonical_json(bytes).map_err(|error| {
            WorkVcsError::PlanInvalid(format!(
                "plan evolution manifest is not valid JSON: {error}"
            ))
        })?;
        let canonical = canonical_bytes(&value).map_err(plan_invalid_from)?;
        serde_json::from_slice(&canonical).map_err(|error| {
            WorkVcsError::PlanInvalid(format!(
                "plan evolution manifest has invalid shape: {error}"
            ))
        })
    }

    pub fn mode(&self) -> &'static str {
        match self {
            Self::InPlace(_) => "in_place",
            Self::Supersede(_) => "supersede",
            Self::TaskDisposition(_) => "task_disposition",
        }
    }

    pub fn idempotency_key(&self) -> &str {
        match self {
            Self::InPlace(manifest) => &manifest.idempotency_key,
            Self::Supersede(manifest) => &manifest.idempotency_key,
            Self::TaskDisposition(manifest) => &manifest.idempotency_key,
        }
    }

    pub fn expected_head_commit_id(&self) -> Result<CommitId> {
        let value = match self {
            Self::InPlace(manifest) => &manifest.expected_head_commit_id,
            Self::Supersede(manifest) => &manifest.expected_head_commit_id,
            Self::TaskDisposition(manifest) => &manifest.expected_head_commit_id,
        };
        CommitId::parse_canonical(value)
    }

    pub fn expected_state_digest(&self) -> Result<Option<Digest>> {
        let value = match self {
            Self::InPlace(manifest) => manifest.expected_state_digest.as_deref(),
            Self::Supersede(manifest) => manifest.expected_state_digest.as_deref(),
            Self::TaskDisposition(manifest) => manifest.expected_state_digest.as_deref(),
        };
        value.map(Digest::from_hex).transpose()
    }

    pub fn payload_digest(&self) -> Result<Digest> {
        let value = evolution_manifest_to_canonical_value(self)?;
        let domain = match self {
            Self::Supersede(manifest) if manifest.schema_version == 2 => {
                EVOLUTION_V2_PAYLOAD_DIGEST_DOMAIN
            }
            Self::TaskDisposition(_) => EVOLUTION_V2_PAYLOAD_DIGEST_DOMAIN,
            _ => EVOLUTION_PAYLOAD_DIGEST_DOMAIN,
        };
        Ok(Digest::domain_separated(domain, &canonical_bytes(&value)?))
    }

    pub fn schema_version(&self) -> i64 {
        match self {
            Self::InPlace(manifest) => manifest.schema_version,
            Self::Supersede(manifest) => manifest.schema_version,
            Self::TaskDisposition(manifest) => manifest.schema_version,
        }
    }

    pub fn payload_kind_version(&self) -> i64 {
        match self {
            Self::Supersede(manifest) if manifest.schema_version == 2 => 2,
            Self::TaskDisposition(_) => 2,
            _ => 1,
        }
    }

    pub fn validate_for_delivery(&self) -> Result<()> {
        validate_plan_evolution_manifest_for_delivery(self)
    }

    pub fn validate_for_authoring(&self) -> Result<()> {
        self.validate_for_delivery()?;
        if matches!(self, Self::Supersede(manifest) if manifest.schema_version == 1) {
            return Err(WorkVcsError::PlanInvalid(
                "fresh Plan supersede authoring requires schema_version 2; schema_version 1 is accepted only for exact legacy reuse or recovery"
                    .to_owned(),
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PlanEvolutionInPlaceManifest {
    pub schema_version: i64,
    pub idempotency_key: String,
    pub expected_head_commit_id: String,
    #[serde(default)]
    pub expected_state_digest: Option<String>,
    pub target_plan_entity_id: String,
    pub expected_plan_entity_version_id: String,
    pub expected_plan_state_digest: String,
    #[serde(default)]
    pub plan: PlanEvolutionPlanUpdateManifest,
    #[serde(default)]
    pub tasks: Vec<PlanAdmissionTaskManifest>,
    #[serde(default)]
    pub records: Vec<PlanAdmissionRecordManifest>,
    #[serde(default)]
    pub evidence: Vec<PlanAdmissionEvidenceManifest>,
    #[serde(default = "empty_object")]
    pub rationale: CanonicalValue,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PlanEvolutionSupersedeManifest {
    pub schema_version: i64,
    pub idempotency_key: String,
    pub expected_head_commit_id: String,
    #[serde(default)]
    pub expected_state_digest: Option<String>,
    pub target_plan_entity_id: String,
    pub expected_plan_entity_version_id: String,
    pub expected_plan_state_digest: String,
    #[serde(default)]
    pub expected_goal_entity_id: Option<String>,
    #[serde(default)]
    pub expected_goal_entity_version_id: Option<String>,
    #[serde(default)]
    pub expected_goal_plan_relation_id: Option<String>,
    #[serde(default)]
    pub expected_goal_plan_relation_version_id: Option<String>,
    #[serde(default)]
    pub expected_parent: Option<PlanEvolutionExpectedParentManifest>,
    pub plan: PlanEvolutionSupersedePlanManifest,
    #[serde(default)]
    pub tasks: Vec<PlanAdmissionTaskManifest>,
    #[serde(default)]
    pub records: Vec<PlanAdmissionRecordManifest>,
    #[serde(default)]
    pub evidence: Vec<PlanAdmissionEvidenceManifest>,
    #[serde(default)]
    pub task_dispositions: Vec<PlanTaskDispositionManifest>,
    #[serde(default)]
    pub nested_plan_continuations: Vec<NestedPlanContinuationManifest>,
    pub rationale: CanonicalValue,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum PlanEvolutionExpectedParentManifest {
    Goal {
        entity_id: String,
        expected_entity_version_id: String,
        containment_relation_id: String,
        containment_relation_version_id: String,
    },
    Plan {
        entity_id: String,
        expected_entity_version_id: String,
        containment_relation_id: String,
        containment_relation_version_id: String,
    },
    None,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PlanTaskDispositionAction {
    Supersede,
    Cancel,
    Continue,
    RetainOriginal,
}

impl PlanTaskDispositionAction {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Supersede => "supersede",
            Self::Cancel => "cancel",
            Self::Continue => "continue",
            Self::RetainOriginal => "retain_original",
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PlanTaskDispositionManifest {
    pub task_entity_id: String,
    pub expected_task_entity_version_id: String,
    pub expected_task_state_digest: String,
    pub action: PlanTaskDispositionAction,
    #[serde(default)]
    pub replacement_task_local_id: Option<String>,
    pub return_condition: String,
    pub rationale: CanonicalValue,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct NestedPlanContinuationManifest {
    pub plan_entity_id: String,
    pub expected_plan_entity_version_id: String,
    pub expected_plan_state_digest: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PlanTaskDispositionRepairManifest {
    pub schema_version: i64,
    pub idempotency_key: String,
    pub expected_head_commit_id: String,
    #[serde(default)]
    pub expected_state_digest: Option<String>,
    pub target_plan_entity_id: String,
    pub expected_plan_entity_version_id: String,
    pub expected_plan_state_digest: String,
    #[serde(default)]
    pub carrier: Option<PlanTaskDispositionCarrierManifest>,
    #[serde(default)]
    pub tasks: Vec<PlanAdmissionTaskManifest>,
    pub task_dispositions: Vec<PlanTaskDispositionManifest>,
    pub rationale: CanonicalValue,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum PlanTaskDispositionCarrierManifest {
    Goal {
        entity_id: String,
        expected_entity_version_id: String,
        expected_state_digest: String,
    },
    Plan {
        entity_id: String,
        expected_entity_version_id: String,
        expected_state_digest: String,
    },
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PlanEvolutionSupersedePlanManifest {
    pub description: String,
    pub strategy: String,
    pub constraints: PlanEvolutionSupersedeConstraintsManifest,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(tag = "mode", rename_all = "snake_case", deny_unknown_fields)]
pub enum PlanEvolutionSupersedeConstraintsManifest {
    CarryAll,
    Replace { values: Vec<String> },
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PlanEvolutionPlanUpdateManifest {
    pub description: Option<String>,
    pub strategy: Option<String>,
    pub constraints: Option<Vec<String>>,
}

impl PlanEvolutionPlanUpdateManifest {
    fn is_empty(&self) -> bool {
        self.description.is_none() && self.strategy.is_none() && self.constraints.is_none()
    }
}

impl<'de> Deserialize<'de> for PlanEvolutionPlanUpdateManifest {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = serde_json::Value::deserialize(deserializer)?;
        let object = value
            .as_object()
            .ok_or_else(|| de::Error::custom("plan evolution plan update must be an object"))?;
        let mut update = Self::default();
        for (key, value) in object {
            match key.as_str() {
                "description" => {
                    update.description =
                        Some(serde_json::from_value(value.clone()).map_err(de::Error::custom)?);
                }
                "strategy" => {
                    update.strategy =
                        Some(serde_json::from_value(value.clone()).map_err(de::Error::custom)?);
                }
                "constraints" => {
                    update.constraints =
                        Some(serde_json::from_value(value.clone()).map_err(de::Error::custom)?);
                }
                other => {
                    return Err(de::Error::unknown_field(
                        other,
                        &["description", "strategy", "constraints"],
                    ));
                }
            }
        }
        Ok(update)
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(tag = "mode", rename_all = "snake_case", deny_unknown_fields)]
pub enum PlanAdmissionGoalManifest {
    Create {
        description: String,
    },
    Existing {
        entity_id: String,
        #[serde(default)]
        expected_entity_version_id: Option<String>,
    },
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PlanAdmissionPlanManifest {
    pub description: String,
    pub strategy: String,
    #[serde(default)]
    pub constraints: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PlanAdmissionTaskManifest {
    pub local_id: String,
    pub description: String,
    #[serde(default)]
    pub priority: Option<i64>,
    #[serde(default)]
    pub acceptance_criteria: Vec<PlanAdmissionAcceptanceCriterionManifest>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PlanAdmissionAcceptanceCriterionManifest {
    pub local_id: String,
    pub statement: String,
    #[serde(default = "default_acceptance_classification")]
    pub classification: String,
    #[serde(default)]
    pub verification_requirements: Vec<PlanAdmissionVerificationRequirementManifest>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PlanAdmissionVerificationRequirementManifest {
    pub local_id: String,
    pub statement: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PlanAdmissionRecordManifest {
    pub local_id: String,
    pub kind: String,
    pub statement: String,
    #[serde(default = "empty_object")]
    pub scope: CanonicalValue,
}

impl PlanAdmissionRecordManifest {
    pub(crate) fn canonical_kind(&self) -> Result<&'static str> {
        Ok(parse_record_kind(&self.kind)?.as_str())
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PlanAdmissionEvidenceManifest {
    pub local_id: String,
    pub kind: String,
    #[serde(default = "empty_object")]
    pub metadata: CanonicalValue,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlanAdmissionOutcome {
    Created,
    Reused,
}

impl PlanAdmissionOutcome {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Created => "created",
            Self::Reused => "reused",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlanAdmissionResult {
    pub outcome: PlanAdmissionOutcome,
    pub workspace_id: WorkspaceId,
    pub branch_id: BranchId,
    pub previous_head_commit_id: CommitId,
    pub commit_id: CommitId,
    pub changeset_id: ChangeSetId,
    pub idempotency_key: String,
    pub payload_digest: Digest,
    pub work_state_digest: Digest,
    pub goal: PlanAdmissionGoalResult,
    pub plan: PlanAdmissionEntityResult,
    pub tasks: Vec<PlanAdmissionTaskResult>,
    pub records: Vec<PlanAdmissionRecordResult>,
    pub evidence: Vec<PlanAdmissionEvidenceResult>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlanAdmissionGoalResult {
    pub created: bool,
    pub entity_id: EntityId,
    pub entity_version_id: EntityVersionId,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlanAdmissionEntityResult {
    pub entity_id: EntityId,
    pub entity_version_id: EntityVersionId,
    pub state_digest: Digest,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlanAdmissionTaskResult {
    pub local_id: String,
    pub entity_id: EntityId,
    pub entity_version_id: EntityVersionId,
    pub state_digest: Digest,
    pub acceptance_criteria: Vec<PlanAdmissionAcceptanceCriterionResult>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlanAdmissionAcceptanceCriterionResult {
    pub local_id: String,
    pub entity_id: EntityId,
    pub entity_version_id: EntityVersionId,
    pub state_digest: Digest,
    pub verification_requirements: Vec<PlanAdmissionVerificationRequirementResult>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlanAdmissionVerificationRequirementResult {
    pub local_id: String,
    pub entity_id: EntityId,
    pub entity_version_id: EntityVersionId,
    pub state_digest: Digest,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlanAdmissionRecordResult {
    pub local_id: String,
    pub kind: RecordKind,
    pub entity_id: EntityId,
    pub entity_version_id: EntityVersionId,
    pub state_digest: Digest,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlanAdmissionEvidenceResult {
    pub local_id: String,
    pub evidence_id: EvidenceId,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlanEvolutionOutcome {
    Created,
    Reused,
}

impl PlanEvolutionOutcome {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Created => "created",
            Self::Reused => "reused",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlanEvolutionResult {
    pub mode: String,
    pub plan_changed: bool,
    pub outcome: PlanEvolutionOutcome,
    pub workspace_id: WorkspaceId,
    pub branch_id: BranchId,
    pub previous_head_commit_id: CommitId,
    pub commit_id: CommitId,
    pub changeset_id: ChangeSetId,
    pub idempotency_key: String,
    pub payload_digest: Digest,
    pub work_state_digest: Digest,
    pub goal_entity_id: Option<EntityId>,
    pub parent_entity_id: Option<EntityId>,
    pub parent_kind: Option<PrimaryContainmentEndpointKind>,
    pub plan: PlanEvolutionPlanResult,
    pub new_plan: Option<PlanAdmissionEntityResult>,
    pub goal_contains_relation: Option<PlanEvolutionRelationResult>,
    pub supersedes_relation: Option<PlanEvolutionRelationResult>,
    pub task_dispositions: Vec<PlanTaskDispositionResult>,
    pub disposition_relations: Vec<PlanEvolutionRelationResult>,
    pub tasks: Vec<PlanAdmissionTaskResult>,
    pub records: Vec<PlanAdmissionRecordResult>,
    pub evidence: Vec<PlanAdmissionEvidenceResult>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlanTaskDispositionResult {
    pub task_entity_id: EntityId,
    pub previous_entity_version_id: EntityVersionId,
    pub previous_state_digest: Digest,
    pub action: PlanTaskDispositionAction,
    pub replacement_task_local_id: Option<String>,
    pub replacement_task_entity_id: Option<EntityId>,
    pub entity_version_id: EntityVersionId,
    pub state_digest: Digest,
    pub post_current: bool,
    pub post_carriers: Vec<String>,
    pub return_condition: String,
    pub risks: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlanEvolutionPlanResult {
    pub entity_id: EntityId,
    pub previous_entity_version_id: EntityVersionId,
    pub previous_state_digest: Digest,
    pub entity_version_id: EntityVersionId,
    pub state_digest: Digest,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlanEvolutionRelationResult {
    pub relation_type: String,
    pub relation_id: RelationId,
    pub relation_version_id: RelationVersionId,
    pub state_digest: Digest,
    pub source_entity_id: EntityId,
    pub target_entity_id: EntityId,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlanSupersedesRelationSnapshot {
    pub workspace_id: WorkspaceId,
    pub commit_id: CommitId,
    pub relation_id: RelationId,
    pub relation_version_id: RelationVersionId,
    pub replacement_plan_entity_id: EntityId,
    pub prior_plan_entity_id: EntityId,
    pub state_digest: Digest,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TaskSupersedesRelationSnapshot {
    pub workspace_id: WorkspaceId,
    pub commit_id: CommitId,
    pub relation_id: RelationId,
    pub relation_version_id: RelationVersionId,
    pub replacement_task_entity_id: EntityId,
    pub prior_task_entity_id: EntityId,
    pub state_digest: Digest,
}

#[derive(Clone, Debug)]
struct PreparedAdmission {
    branch_id: BranchId,
    previous_head_commit_id: CommitId,
    expected_state_digest: Option<Digest>,
    idempotency_key: String,
    payload_digest: Digest,
    rationale: CanonicalValue,
    goal: PreparedGoal,
    plan: PreparedEntity,
    tasks: Vec<PreparedTask>,
    records: Vec<PreparedRecord>,
    evidence: Vec<PreparedEvidence>,
}

#[derive(Clone, Debug)]
struct PreparedEvolution {
    mode: &'static str,
    operation_schema_version: i64,
    plan_changed: bool,
    branch_id: BranchId,
    previous_head_commit_id: CommitId,
    expected_state_digest: Option<Digest>,
    idempotency_key: String,
    payload_digest: Digest,
    rationale: CanonicalValue,
    goal_entity_id: Option<EntityId>,
    parent_entity_id: Option<EntityId>,
    parent_kind: Option<PrimaryContainmentEndpointKind>,
    previous_plan_entity_version_id: EntityVersionId,
    previous_plan_state_digest: Digest,
    plan: PreparedEntity,
    task_parent_entity_id: EntityId,
    carrier_entity_id: Option<EntityId>,
    carrier_kind: Option<PrimaryContainmentEndpointKind>,
    expected_carrier_entity_version_id: Option<EntityVersionId>,
    expected_carrier_state_digest: Option<Digest>,
    new_plan: Option<PreparedEntity>,
    expected_goal_entity_version_id: Option<EntityVersionId>,
    expected_goal_plan_relation_id: Option<RelationId>,
    expected_goal_plan_relation_version_id: Option<RelationVersionId>,
    goal_contains_relation: Option<PreparedRelation>,
    supersedes_relation: Option<PreparedRelation>,
    task_dispositions: Vec<PreparedTaskDisposition>,
    disposition_relations: Vec<PreparedRelation>,
    tasks: Vec<PreparedTask>,
    records: Vec<PreparedRecord>,
    evidence: Vec<PreparedEvidence>,
}

#[derive(Clone, Debug)]
struct PreparedTaskDisposition {
    task_entity_id: EntityId,
    previous_entity_version_id: EntityVersionId,
    previous_state_digest: Digest,
    action: PlanTaskDispositionAction,
    replacement_task_local_id: Option<String>,
    replacement_task_entity_id: Option<EntityId>,
    next_entity: Option<PreparedEntity>,
    return_condition: String,
    rationale: CanonicalValue,
    pre_current: bool,
    post_current: bool,
    post_carriers: Vec<String>,
    risks: Vec<String>,
}

#[derive(Clone, Debug)]
struct PreparedNestedPlanContinuation {
    plan_entity_id: EntityId,
    expected_plan_entity_version_id: EntityVersionId,
    expected_plan_state_digest: Digest,
}

#[derive(Clone, Debug)]
enum PreparedGoal {
    Existing {
        entity_id: EntityId,
        entity_version_id: EntityVersionId,
    },
    Create(PreparedEntity),
}

#[derive(Clone, Debug)]
pub(super) struct PreparedEntity {
    pub(super) entity_id: EntityId,
    pub(super) entity_version_id: EntityVersionId,
    pub(super) entity_kind: &'static str,
    pub(super) state: CanonicalValue,
    pub(super) state_digest: Digest,
}

#[derive(Clone, Debug)]
pub(super) struct PreparedTask {
    pub(super) local_id: String,
    pub(super) entity: PreparedEntity,
    pub(super) acceptance_criteria: Vec<PreparedAcceptanceCriterion>,
}

#[derive(Clone, Debug)]
pub(super) struct PreparedAcceptanceCriterion {
    pub(super) local_id: String,
    pub(super) entity: PreparedEntity,
    pub(super) verification_requirements: Vec<PreparedVerificationRequirement>,
}

#[derive(Clone, Debug)]
pub(super) struct PreparedVerificationRequirement {
    pub(super) local_id: String,
    pub(super) entity: PreparedEntity,
}

#[derive(Clone, Debug)]
pub(super) struct PreparedRecord {
    pub(super) local_id: String,
    pub(super) kind: RecordKind,
    pub(super) entity: PreparedEntity,
}

#[derive(Clone, Debug)]
pub(super) struct PreparedEvidence {
    pub(super) local_id: String,
    pub(super) evidence_id: EvidenceId,
    pub(super) evidence_kind: String,
    pub(super) metadata: CanonicalValue,
}

#[derive(Clone, Debug)]
pub(super) struct PreparedRelation {
    pub(super) relation_id: RelationId,
    pub(super) relation_version_id: RelationVersionId,
    pub(super) relation_type: String,
    pub(super) relation_discriminator: String,
    pub(super) source_entity_id: EntityId,
    pub(super) target_entity_id: EntityId,
    pub(super) state_json: String,
    pub(super) state_digest: Digest,
}

#[derive(Clone, Debug)]
pub(super) struct BranchRow {
    pub(super) workspace_id: WorkspaceId,
    pub(super) head_commit_id: CommitId,
}

pub(crate) fn admit_plan(
    connection: &mut StoreConnection,
    options: &PlanAdmissionOptions,
) -> Result<PlanAdmissionResult> {
    connection.verify_foreign_keys()?;
    let prepared = prepare_admission(connection, options)?;
    let now_us = current_epoch_micros()?;
    let changeset_id = ChangeSetId::new_v7();
    let commit_id = CommitId::new_v7();
    let parent = state_at(connection, prepared.previous_head_commit_id)?;

    let transaction = connection
        .inner_mut()
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(storage_error)?;
    let branch = load_active_branch(&transaction, prepared.branch_id)?;
    if let Some(reused) = find_idempotent_admission(
        &transaction,
        branch.workspace_id,
        prepared.branch_id,
        &prepared.idempotency_key,
        prepared.payload_digest,
    )? {
        return Ok(reused);
    }
    if branch.head_commit_id != prepared.previous_head_commit_id {
        return Err(WorkVcsError::BranchHeadConflict(format!(
            "branch {} expected head {}, found {}",
            prepared.branch_id, prepared.previous_head_commit_id, branch.head_commit_id
        )));
    }

    if parent.workspace_id != branch.workspace_id {
        return Err(WorkVcsError::PlanInvalid(format!(
            "branch {} belongs to workspace {}, but expected head {} belongs to workspace {}",
            prepared.branch_id,
            branch.workspace_id,
            prepared.previous_head_commit_id,
            parent.workspace_id
        )));
    }
    if let Some(expected_state_digest) = prepared.expected_state_digest
        && parent.state_digest != expected_state_digest
    {
        return Err(WorkVcsError::DigestInvalid(format!(
            "expected head {} state digest {} does not match expected {}",
            prepared.previous_head_commit_id, parent.state_digest, expected_state_digest
        )));
    }

    let relation_state = CanonicalValue::object(Vec::new())?;
    let relation_state_json = canonical_json_string(&relation_state)?;
    let relation_state_digest = relation_version_digest(&relation_state)?;
    let relations = prepare_relations(&prepared, &relation_state_json, relation_state_digest)?;
    let next_state = admission_work_state(&parent.state, &prepared, &relations)?;
    let work_state_digest = work_state_mapping_digest(&next_state);
    let operation_payload = admission_payload_value(
        branch.workspace_id,
        prepared.branch_id,
        prepared.previous_head_commit_id,
        commit_id,
        changeset_id,
        work_state_digest,
        &prepared,
        &relations,
    )?;
    let operation_payload_json = canonical_json_string(&operation_payload)?;
    let rationale_json = canonical_json_string(&prepared.rationale)?;

    let mut result = result_from_payload(&operation_payload)?;
    result.outcome = PlanAdmissionOutcome::Created;

    write_admission(
        &transaction,
        branch.workspace_id,
        changeset_id,
        commit_id,
        now_us,
        work_state_digest,
        &operation_payload_json,
        &rationale_json,
        &prepared,
        &relations,
    )?;
    move_branch_head(
        &transaction,
        prepared.branch_id,
        prepared.previous_head_commit_id,
        commit_id,
        now_us,
    )?;
    transaction.commit().map_err(storage_error)?;
    Ok(result)
}

pub(crate) fn evolve_plan(
    connection: &mut StoreConnection,
    options: &PlanEvolutionOptions,
) -> Result<PlanEvolutionResult> {
    connection.verify_foreign_keys()?;
    let branch = super::query::branch_head(connection, options.branch_id)?;
    if let Some(reused) = find_idempotent_evolution(
        connection.inner(),
        branch.workspace_id,
        options.branch_id,
        options.manifest.idempotency_key(),
        options.manifest.payload_digest()?,
    )? {
        let parent = state_at(connection, reused.previous_head_commit_id)?;
        validate_idempotent_evolution_result(connection.inner(), &parent.state, &reused)?;
        return Ok(reused);
    }
    if matches!(
        &options.manifest,
        PlanEvolutionManifest::Supersede(manifest) if manifest.schema_version == 1
    ) && !options.allow_legacy_supersede_recovery
    {
        return Err(WorkVcsError::PlanInvalid(
            "fresh schema_version 1 Plan supersede is prohibited; only exact committed reuse or already-admitted legacy recovery may execute"
                .to_owned(),
        ));
    }
    let prepared = prepare_evolution(connection, options)?;
    let now_us = current_epoch_micros()?;
    let changeset_id = ChangeSetId::new_v7();
    let commit_id = CommitId::new_v7();
    let parent = state_at(connection, prepared.previous_head_commit_id)?;

    let transaction = connection
        .inner_mut()
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(storage_error)?;
    let branch = load_active_branch(&transaction, prepared.branch_id)?;
    if let Some(reused) = find_idempotent_evolution(
        &transaction,
        branch.workspace_id,
        prepared.branch_id,
        &prepared.idempotency_key,
        prepared.payload_digest,
    )? {
        validate_idempotent_evolution_result(&transaction, &parent.state, &reused)?;
        return Ok(reused);
    }
    if branch.head_commit_id != prepared.previous_head_commit_id {
        return Err(WorkVcsError::BranchHeadConflict(format!(
            "branch {} expected head {}, found {}",
            prepared.branch_id, prepared.previous_head_commit_id, branch.head_commit_id
        )));
    }
    if parent.workspace_id != branch.workspace_id {
        return Err(WorkVcsError::PlanInvalid(format!(
            "branch {} belongs to workspace {}, but expected head {} belongs to workspace {}",
            prepared.branch_id,
            branch.workspace_id,
            prepared.previous_head_commit_id,
            parent.workspace_id
        )));
    }
    let head_state_digest =
        load_commit_state_digest(&transaction, prepared.previous_head_commit_id)?;
    if head_state_digest != parent.state_digest {
        return Err(WorkVcsError::PlanInvalid(format!(
            "expected head {} replayed state digest {} does not match stored digest {}",
            prepared.previous_head_commit_id, parent.state_digest, head_state_digest
        )));
    }
    if let Some(expected_state_digest) = prepared.expected_state_digest
        && parent.state_digest != expected_state_digest
    {
        return Err(WorkVcsError::DigestInvalid(format!(
            "expected head {} state digest {} does not match expected {}",
            prepared.previous_head_commit_id, parent.state_digest, expected_state_digest
        )));
    }
    validate_plan_version_for_evolution(&transaction, branch.workspace_id, &prepared)?;
    validate_supersede_goal_and_relation_for_evolution(
        &transaction,
        branch.workspace_id,
        &parent.state,
        &prepared,
    )?;
    validate_repair_carrier_for_evolution(&transaction, branch.workspace_id, &prepared)?;
    validate_task_dispositions_for_evolution(&transaction, branch.workspace_id, &prepared)?;

    let relation_state = CanonicalValue::object(Vec::new())?;
    let relation_state_json = canonical_json_string(&relation_state)?;
    let relation_state_digest = relation_version_digest(&relation_state)?;
    let relations = prepare_evolution_relations(
        &prepared,
        &prepared.tasks,
        &relation_state_json,
        relation_state_digest,
    );
    let next_state = evolution_work_state(&parent.state, &prepared, &relations)?;
    let work_state_digest = work_state_mapping_digest(&next_state);
    let operation_payload = evolution_payload_value(
        branch.workspace_id,
        prepared.branch_id,
        prepared.previous_head_commit_id,
        commit_id,
        changeset_id,
        work_state_digest,
        &prepared,
        &relations,
    )?;
    let operation_payload_json = canonical_json_string(&operation_payload)?;
    let rationale_json = canonical_json_string(&prepared.rationale)?;

    let mut result = evolution_result_from_payload(&operation_payload)?;
    result.outcome = PlanEvolutionOutcome::Created;

    write_evolution(
        &transaction,
        branch.workspace_id,
        changeset_id,
        commit_id,
        now_us,
        work_state_digest,
        &operation_payload_json,
        &rationale_json,
        &prepared,
        &relations,
    )?;
    move_branch_head(
        &transaction,
        prepared.branch_id,
        prepared.previous_head_commit_id,
        commit_id,
        now_us,
    )?;
    transaction.commit().map_err(storage_error)?;
    Ok(result)
}

fn validate_task_dispositions_for_evolution(
    transaction: &Transaction<'_>,
    workspace_id: WorkspaceId,
    prepared: &PreparedEvolution,
) -> Result<()> {
    for disposition in &prepared.task_dispositions {
        let row = transaction
            .query_row(
                "SELECT entity.workspace_id, entity.entity_kind, entity_version.state_digest
                 FROM entity
                 JOIN entity_version ON entity_version.entity_id = entity.object_id
                 WHERE entity.object_id = ?1 AND entity_version.entity_version_id = ?2",
                params![
                    &disposition.task_entity_id.raw_bytes()[..],
                    &disposition.previous_entity_version_id.raw_bytes()[..]
                ],
                |row| {
                    Ok((
                        row.get::<_, Vec<u8>>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, Vec<u8>>(2)?,
                    ))
                },
            )
            .optional()
            .map_err(storage_error)?;
        let Some((actual_workspace, kind, digest)) = row else {
            return Err(WorkVcsError::TaskNotFound(format!(
                "guarded task {} version {} does not exist",
                disposition.task_entity_id, disposition.previous_entity_version_id
            )));
        };
        if decode_workspace_id("entity.workspace_id", actual_workspace)? != workspace_id
            || kind != TASK_ENTITY_KIND
        {
            return Err(WorkVcsError::TaskInvalid(format!(
                "guarded task {} is not a Task in workspace {workspace_id}",
                disposition.task_entity_id
            )));
        }
        if decode_digest("entity_version.state_digest", digest)?
            != disposition.previous_state_digest
        {
            return Err(WorkVcsError::DigestInvalid(format!(
                "guarded task {} state digest changed before disposition",
                disposition.task_entity_id
            )));
        }
        let active_claims =
            active_claim_count(transaction, prepared.branch_id, disposition.task_entity_id)?;
        if active_claims > 0
            && (matches!(
                disposition.action,
                PlanTaskDispositionAction::Supersede | PlanTaskDispositionAction::Cancel
            ) || (disposition.action == PlanTaskDispositionAction::RetainOriginal
                && disposition.pre_current
                && !disposition.post_current))
        {
            return Err(WorkVcsError::ClaimInvalid(format!(
                "task {} acquired {active_claims} active claim(s) before {} disposition could commit",
                disposition.task_entity_id,
                disposition.action.as_str()
            )));
        }
    }
    Ok(())
}

pub(crate) fn find_plan_admission_result(
    connection: &StoreConnection,
    options: &PlanAdmissionOptions,
) -> Result<Option<PlanAdmissionResult>> {
    let branch = super::query::branch_head(connection, options.branch_id)?;
    find_idempotent_admission(
        connection.inner(),
        branch.workspace_id,
        options.branch_id,
        options.manifest.idempotency_key(),
        options.manifest.payload_digest()?,
    )
}

pub(crate) fn find_plan_evolution_result(
    connection: &StoreConnection,
    options: &PlanEvolutionOptions,
) -> Result<Option<PlanEvolutionResult>> {
    let branch = super::query::branch_head(connection, options.branch_id)?;
    let result = find_idempotent_evolution(
        connection.inner(),
        branch.workspace_id,
        options.branch_id,
        options.manifest.idempotency_key(),
        options.manifest.payload_digest()?,
    )?;
    if let Some(result) = result.as_ref() {
        let parent = state_at(connection, result.previous_head_commit_id)?;
        validate_idempotent_evolution_result(connection.inner(), &parent.state, result)?;
    }
    Ok(result)
}

fn validate_plan_admission_manifest_for_delivery(manifest: &PlanAdmissionManifest) -> Result<()> {
    if manifest.schema_version != 1 {
        return Err(WorkVcsError::PlanInvalid(format!(
            "plan admission manifest schema_version {} is not supported",
            manifest.schema_version
        )));
    }
    validate_idempotency_key(&manifest.idempotency_key)?;
    require_object("plan admission rationale", &manifest.rationale)?;
    validate_unique_local_ids("task", manifest.tasks.iter().map(|task| &task.local_id))?;
    validate_unique_local_ids(
        "record",
        manifest.records.iter().map(|record| &record.local_id),
    )?;
    validate_unique_local_ids(
        "evidence",
        manifest.evidence.iter().map(|evidence| &evidence.local_id),
    )?;
    for task in &manifest.tasks {
        validate_unique_local_ids(
            "acceptance criterion",
            task.acceptance_criteria
                .iter()
                .map(|criterion| &criterion.local_id),
        )?;
        for criterion in &task.acceptance_criteria {
            validate_unique_local_ids(
                "verification requirement",
                criterion
                    .verification_requirements
                    .iter()
                    .map(|requirement| &requirement.local_id),
            )?;
        }
    }

    manifest.expected_head_commit_id()?;
    manifest.expected_state_digest()?;
    manifest.payload_digest()?;
    match &manifest.goal {
        PlanAdmissionGoalManifest::Create { description } => {
            super::goal::GoalState::active(description.clone())?.to_canonical_value()?;
        }
        PlanAdmissionGoalManifest::Existing {
            entity_id,
            expected_entity_version_id,
        } => {
            EntityId::parse_canonical(entity_id)?;
            if let Some(expected_entity_version_id) = expected_entity_version_id {
                EntityVersionId::parse_canonical(expected_entity_version_id)?;
            }
        }
    }
    let mut plan = super::plan::PlanState::active(
        manifest.plan.description.clone(),
        manifest.plan.strategy.clone(),
    )?;
    plan.constraints = manifest.plan.constraints.clone();
    plan.to_canonical_value()?;
    prepare_tasks(&manifest.tasks)?;
    prepare_records(&manifest.records)?;
    prepare_evidence(&manifest.evidence)?;
    Ok(())
}

fn validate_plan_evolution_manifest_for_delivery(manifest: &PlanEvolutionManifest) -> Result<()> {
    match manifest {
        PlanEvolutionManifest::InPlace(manifest) => {
            if manifest.schema_version != 1 {
                return Err(WorkVcsError::PlanInvalid(format!(
                    "plan evolution manifest schema_version {} is not supported",
                    manifest.schema_version
                )));
            }
            validate_idempotency_key(&manifest.idempotency_key)?;
            require_object("plan evolution rationale", &manifest.rationale)?;
            validate_evolution_append_manifests(
                &manifest.tasks,
                &manifest.records,
                &manifest.evidence,
            )?;
            if manifest.plan.is_empty()
                && manifest.tasks.is_empty()
                && manifest.records.is_empty()
                && manifest.evidence.is_empty()
            {
                return Err(WorkVcsError::PlanInvalid(
                    "plan evolution manifest must update the Plan or append at least one object"
                        .to_owned(),
                ));
            }
            CommitId::parse_canonical(&manifest.expected_head_commit_id)?;
            manifest
                .expected_state_digest
                .as_deref()
                .map(Digest::from_hex)
                .transpose()?;
            EntityId::parse_canonical(&manifest.target_plan_entity_id)?;
            EntityVersionId::parse_canonical(&manifest.expected_plan_entity_version_id)?;
            Digest::from_hex(&manifest.expected_plan_state_digest)?;
            let mut candidate = PlanState::active("validation", "validation")?;
            if let Some(description) = &manifest.plan.description {
                candidate.description = description.clone();
            }
            if let Some(strategy) = &manifest.plan.strategy {
                candidate.strategy = strategy.clone();
            }
            if let Some(constraints) = &manifest.plan.constraints {
                candidate.constraints = constraints.clone();
            }
            candidate.to_canonical_value()?;
            prepare_tasks(&manifest.tasks)?;
            prepare_records(&manifest.records)?;
            prepare_evidence(&manifest.evidence)?;
        }
        PlanEvolutionManifest::Supersede(manifest) => {
            if !matches!(manifest.schema_version, 1 | 2) {
                return Err(WorkVcsError::PlanInvalid(format!(
                    "plan evolution manifest schema_version {} is not supported",
                    manifest.schema_version
                )));
            }
            validate_idempotency_key(&manifest.idempotency_key)?;
            require_non_empty_object("plan supersede rationale", &manifest.rationale)?;
            validate_evolution_append_manifests(
                &manifest.tasks,
                &manifest.records,
                &manifest.evidence,
            )?;
            CommitId::parse_canonical(&manifest.expected_head_commit_id)?;
            manifest
                .expected_state_digest
                .as_deref()
                .map(Digest::from_hex)
                .transpose()?;
            EntityId::parse_canonical(&manifest.target_plan_entity_id)?;
            EntityVersionId::parse_canonical(&manifest.expected_plan_entity_version_id)?;
            Digest::from_hex(&manifest.expected_plan_state_digest)?;
            validate_supersede_parent_shape(manifest)?;
            validate_task_disposition_manifest(manifest)?;
            validate_nested_plan_continuation_manifest(manifest)?;
            let mut candidate = PlanState::active(
                manifest.plan.description.clone(),
                manifest.plan.strategy.clone(),
            )?;
            if let PlanEvolutionSupersedeConstraintsManifest::Replace { values } =
                &manifest.plan.constraints
            {
                candidate.constraints = values.clone();
            }
            candidate.to_canonical_value()?;
            prepare_tasks(&manifest.tasks)?;
            prepare_records(&manifest.records)?;
            prepare_evidence(&manifest.evidence)?;
        }
        PlanEvolutionManifest::TaskDisposition(manifest) => {
            if manifest.schema_version != 2 {
                return Err(WorkVcsError::PlanInvalid(format!(
                    "task disposition repair manifest schema_version {} is not supported",
                    manifest.schema_version
                )));
            }
            validate_idempotency_key(&manifest.idempotency_key)?;
            require_non_empty_object("task disposition repair rationale", &manifest.rationale)?;
            validate_evolution_append_manifests(&manifest.tasks, &[], &[])?;
            CommitId::parse_canonical(&manifest.expected_head_commit_id)?;
            manifest
                .expected_state_digest
                .as_deref()
                .map(Digest::from_hex)
                .transpose()?;
            EntityId::parse_canonical(&manifest.target_plan_entity_id)?;
            EntityVersionId::parse_canonical(&manifest.expected_plan_entity_version_id)?;
            Digest::from_hex(&manifest.expected_plan_state_digest)?;
            validate_task_disposition_items(&manifest.tasks, &manifest.task_dispositions)?;
            if let Some(carrier) = &manifest.carrier {
                match carrier {
                    PlanTaskDispositionCarrierManifest::Goal {
                        entity_id,
                        expected_entity_version_id,
                        expected_state_digest,
                    }
                    | PlanTaskDispositionCarrierManifest::Plan {
                        entity_id,
                        expected_entity_version_id,
                        expected_state_digest,
                    } => {
                        EntityId::parse_canonical(entity_id)?;
                        EntityVersionId::parse_canonical(expected_entity_version_id)?;
                        Digest::from_hex(expected_state_digest)?;
                    }
                }
            }
            let needs_carrier = !manifest.tasks.is_empty()
                || manifest.task_dispositions.iter().any(|item| {
                    matches!(
                        item.action,
                        PlanTaskDispositionAction::Supersede | PlanTaskDispositionAction::Continue
                    )
                });
            if needs_carrier && manifest.carrier.is_none() {
                return Err(WorkVcsError::PlanInvalid(
                    "task disposition repair requires a current carrier for continue or replacement tasks"
                        .to_owned(),
                ));
            }
            prepare_tasks(&manifest.tasks)?;
        }
    }
    manifest.payload_digest()?;
    Ok(())
}

fn validate_supersede_parent_shape(manifest: &PlanEvolutionSupersedeManifest) -> Result<()> {
    if manifest.schema_version == 1 {
        if manifest.expected_parent.is_some()
            || !manifest.task_dispositions.is_empty()
            || !manifest.nested_plan_continuations.is_empty()
        {
            return Err(WorkVcsError::PlanInvalid(
                "schema_version 1 plan supersede cannot carry schema_version 2 disposition fields"
                    .to_owned(),
            ));
        }
        EntityId::parse_canonical(manifest.expected_goal_entity_id.as_deref().ok_or_else(
            || {
                WorkVcsError::PlanInvalid(
                    "schema_version 1 supersede requires expected_goal_entity_id".to_owned(),
                )
            },
        )?)?;
        EntityVersionId::parse_canonical(
            manifest
                .expected_goal_entity_version_id
                .as_deref()
                .ok_or_else(|| {
                    WorkVcsError::PlanInvalid(
                        "schema_version 1 supersede requires expected_goal_entity_version_id"
                            .to_owned(),
                    )
                })?,
        )?;
        RelationId::parse_canonical(
            manifest
                .expected_goal_plan_relation_id
                .as_deref()
                .ok_or_else(|| {
                    WorkVcsError::PlanInvalid(
                        "schema_version 1 supersede requires expected_goal_plan_relation_id"
                            .to_owned(),
                    )
                })?,
        )?;
        RelationVersionId::parse_canonical(
            manifest
                .expected_goal_plan_relation_version_id
                .as_deref()
                .ok_or_else(|| {
                    WorkVcsError::PlanInvalid(
                        "schema_version 1 supersede requires expected_goal_plan_relation_version_id"
                            .to_owned(),
                    )
                })?,
        )?;
        return Ok(());
    }

    if manifest.expected_goal_entity_id.is_some()
        || manifest.expected_goal_entity_version_id.is_some()
        || manifest.expected_goal_plan_relation_id.is_some()
        || manifest.expected_goal_plan_relation_version_id.is_some()
    {
        return Err(WorkVcsError::PlanInvalid(
            "schema_version 2 supersede uses expected_parent and cannot carry schema_version 1 Goal parent fields"
                .to_owned(),
        ));
    }
    let parent = manifest.expected_parent.as_ref().ok_or_else(|| {
        WorkVcsError::PlanInvalid("schema_version 2 supersede requires expected_parent".to_owned())
    })?;
    match parent {
        PlanEvolutionExpectedParentManifest::Goal {
            entity_id,
            expected_entity_version_id,
            containment_relation_id,
            containment_relation_version_id,
        }
        | PlanEvolutionExpectedParentManifest::Plan {
            entity_id,
            expected_entity_version_id,
            containment_relation_id,
            containment_relation_version_id,
        } => {
            EntityId::parse_canonical(entity_id)?;
            EntityVersionId::parse_canonical(expected_entity_version_id)?;
            RelationId::parse_canonical(containment_relation_id)?;
            RelationVersionId::parse_canonical(containment_relation_version_id)?;
        }
        PlanEvolutionExpectedParentManifest::None => {}
    }
    Ok(())
}

fn validate_task_disposition_manifest(manifest: &PlanEvolutionSupersedeManifest) -> Result<()> {
    validate_task_disposition_items(&manifest.tasks, &manifest.task_dispositions)
}

fn validate_task_disposition_items(
    tasks: &[PlanAdmissionTaskManifest],
    dispositions: &[PlanTaskDispositionManifest],
) -> Result<()> {
    let mut task_ids = BTreeSet::new();
    let task_local_ids = tasks
        .iter()
        .map(|task| task.local_id.as_str())
        .collect::<BTreeSet<_>>();
    let mut replacement_local_ids = BTreeSet::new();
    for disposition in dispositions {
        let task_id = EntityId::parse_canonical(&disposition.task_entity_id)?;
        if !task_ids.insert(task_id) {
            return Err(WorkVcsError::PlanInvalid(format!(
                "task disposition repeats task {task_id}"
            )));
        }
        EntityVersionId::parse_canonical(&disposition.expected_task_entity_version_id)?;
        Digest::from_hex(&disposition.expected_task_state_digest)?;
        validate_stored_text(
            "task disposition return_condition",
            &disposition.return_condition,
        )?;
        require_non_empty_object("task disposition rationale", &disposition.rationale)?;
        match disposition.action {
            PlanTaskDispositionAction::Supersede => {
                let local_id = disposition.replacement_task_local_id.as_deref().ok_or_else(|| {
                    WorkVcsError::PlanInvalid(format!(
                        "supersede disposition for task {task_id} requires replacement_task_local_id"
                    ))
                })?;
                if !task_local_ids.contains(local_id) {
                    return Err(WorkVcsError::PlanInvalid(format!(
                        "supersede disposition for task {task_id} maps to missing replacement task local_id {local_id:?}"
                    )));
                }
                if !replacement_local_ids.insert(local_id) {
                    return Err(WorkVcsError::PlanInvalid(format!(
                        "replacement task local_id {local_id:?} is mapped by more than one old task"
                    )));
                }
            }
            PlanTaskDispositionAction::Cancel
            | PlanTaskDispositionAction::Continue
            | PlanTaskDispositionAction::RetainOriginal => {
                if disposition.replacement_task_local_id.is_some() {
                    return Err(WorkVcsError::PlanInvalid(format!(
                        "{} disposition for task {task_id} cannot carry replacement_task_local_id",
                        disposition.action.as_str()
                    )));
                }
            }
        }
    }
    Ok(())
}

fn validate_nested_plan_continuation_manifest(
    manifest: &PlanEvolutionSupersedeManifest,
) -> Result<()> {
    let mut plan_ids = BTreeSet::new();
    for continuation in &manifest.nested_plan_continuations {
        let plan_id = EntityId::parse_canonical(&continuation.plan_entity_id)?;
        if !plan_ids.insert(plan_id) {
            return Err(WorkVcsError::PlanInvalid(format!(
                "nested plan continuation repeats plan {plan_id}"
            )));
        }
        EntityVersionId::parse_canonical(&continuation.expected_plan_entity_version_id)?;
        Digest::from_hex(&continuation.expected_plan_state_digest)?;
    }
    Ok(())
}

fn prepare_evolution(
    connection: &StoreConnection,
    options: &PlanEvolutionOptions,
) -> Result<PreparedEvolution> {
    match &options.manifest {
        PlanEvolutionManifest::InPlace(manifest) => {
            prepare_in_place_evolution(connection, options, manifest)
        }
        PlanEvolutionManifest::Supersede(manifest) => {
            prepare_supersede_evolution(connection, options, manifest)
        }
        PlanEvolutionManifest::TaskDisposition(manifest) => {
            prepare_task_disposition_repair(connection, options, manifest)
        }
    }
}

fn validate_evolution_append_manifests(
    tasks: &[PlanAdmissionTaskManifest],
    records: &[PlanAdmissionRecordManifest],
    evidence: &[PlanAdmissionEvidenceManifest],
) -> Result<()> {
    validate_unique_local_ids("task", tasks.iter().map(|task| &task.local_id))?;
    validate_unique_local_ids("record", records.iter().map(|record| &record.local_id))?;
    validate_unique_local_ids(
        "evidence",
        evidence.iter().map(|evidence| &evidence.local_id),
    )?;
    for task in tasks {
        validate_unique_local_ids(
            "acceptance criterion",
            task.acceptance_criteria
                .iter()
                .map(|criterion| &criterion.local_id),
        )?;
        for criterion in &task.acceptance_criteria {
            validate_unique_local_ids(
                "verification requirement",
                criterion
                    .verification_requirements
                    .iter()
                    .map(|requirement| &requirement.local_id),
            )?;
        }
    }
    Ok(())
}

fn prepare_in_place_evolution(
    connection: &StoreConnection,
    options: &PlanEvolutionOptions,
    manifest: &PlanEvolutionInPlaceManifest,
) -> Result<PreparedEvolution> {
    if manifest.schema_version != 1 {
        return Err(WorkVcsError::PlanInvalid(format!(
            "plan evolution manifest schema_version {} is not supported",
            manifest.schema_version
        )));
    }
    validate_idempotency_key(&manifest.idempotency_key)?;
    require_object("plan evolution rationale", &manifest.rationale)?;
    validate_evolution_append_manifests(&manifest.tasks, &manifest.records, &manifest.evidence)?;
    if manifest.plan.is_empty()
        && manifest.tasks.is_empty()
        && manifest.records.is_empty()
        && manifest.evidence.is_empty()
    {
        return Err(WorkVcsError::PlanInvalid(
            "plan evolution manifest must update the Plan or append at least one object".to_owned(),
        ));
    }

    let manifest_value = evolution_manifest_to_canonical_value(&options.manifest)?;
    let payload_digest = Digest::domain_separated(
        EVOLUTION_PAYLOAD_DIGEST_DOMAIN,
        &canonical_bytes(&manifest_value)?,
    );
    let previous_head_commit_id = CommitId::parse_canonical(&manifest.expected_head_commit_id)?;
    let expected_state_digest = manifest
        .expected_state_digest
        .as_deref()
        .map(Digest::from_hex)
        .transpose()?;
    let target_plan_entity_id = EntityId::parse_canonical(&manifest.target_plan_entity_id)?;
    let expected_plan_entity_version_id =
        EntityVersionId::parse_canonical(&manifest.expected_plan_entity_version_id)?;
    let expected_plan_state_digest = Digest::from_hex(&manifest.expected_plan_state_digest)?;
    let current_plan = plan_at(connection, previous_head_commit_id, target_plan_entity_id)?;
    if current_plan.plan_entity_version_id != expected_plan_entity_version_id {
        return Err(WorkVcsError::PlanInvalid(format!(
            "plan entity {} expected version {}, found {} at commit {}",
            target_plan_entity_id,
            expected_plan_entity_version_id,
            current_plan.plan_entity_version_id,
            previous_head_commit_id
        )));
    }
    if current_plan.state_digest != expected_plan_state_digest {
        return Err(WorkVcsError::DigestInvalid(format!(
            "plan entity {} state digest {} does not match expected {} at commit {}",
            target_plan_entity_id,
            current_plan.state_digest,
            expected_plan_state_digest,
            previous_head_commit_id
        )));
    }
    if current_plan.state.status != PlanStatus::Active {
        return Err(WorkVcsError::PlanInvalid(format!(
            "plan entity {} is {}, not active",
            target_plan_entity_id, current_plan.state.status
        )));
    }
    let goal_entity_id =
        resolve_single_goal_parent(connection, previous_head_commit_id, target_plan_entity_id)?;
    let next_plan_state = apply_plan_update(current_plan.state, &manifest.plan)?;
    let plan = prepared_existing_entity(
        PLAN_ENTITY_KIND,
        target_plan_entity_id,
        next_plan_state.to_canonical_value()?,
    )?;
    let tasks = prepare_tasks(&manifest.tasks)?;
    let records = prepare_records(&manifest.records)?;
    let evidence = prepare_evidence(&manifest.evidence)?;

    Ok(PreparedEvolution {
        mode: "in_place",
        operation_schema_version: 1,
        plan_changed: true,
        branch_id: options.branch_id,
        previous_head_commit_id,
        expected_state_digest,
        idempotency_key: manifest.idempotency_key.clone(),
        payload_digest,
        rationale: manifest.rationale.clone(),
        goal_entity_id: Some(goal_entity_id),
        parent_entity_id: Some(goal_entity_id),
        parent_kind: Some(PrimaryContainmentEndpointKind::Goal),
        previous_plan_entity_version_id: expected_plan_entity_version_id,
        previous_plan_state_digest: expected_plan_state_digest,
        task_parent_entity_id: target_plan_entity_id,
        carrier_entity_id: None,
        carrier_kind: None,
        expected_carrier_entity_version_id: None,
        expected_carrier_state_digest: None,
        plan,
        new_plan: None,
        expected_goal_entity_version_id: None,
        expected_goal_plan_relation_id: None,
        expected_goal_plan_relation_version_id: None,
        goal_contains_relation: None,
        supersedes_relation: None,
        task_dispositions: Vec::new(),
        disposition_relations: Vec::new(),
        tasks,
        records,
        evidence,
    })
}

fn prepare_supersede_evolution(
    connection: &StoreConnection,
    options: &PlanEvolutionOptions,
    manifest: &PlanEvolutionSupersedeManifest,
) -> Result<PreparedEvolution> {
    match manifest.schema_version {
        1 => prepare_supersede_evolution_v1(connection, options, manifest),
        2 => prepare_supersede_evolution_v2(connection, options, manifest),
        other => Err(WorkVcsError::PlanInvalid(format!(
            "plan evolution manifest schema_version {other} is not supported"
        ))),
    }
}

fn prepare_supersede_evolution_v1(
    connection: &StoreConnection,
    options: &PlanEvolutionOptions,
    manifest: &PlanEvolutionSupersedeManifest,
) -> Result<PreparedEvolution> {
    if manifest.schema_version != 1 {
        return Err(WorkVcsError::PlanInvalid(format!(
            "plan evolution manifest schema_version {} is not supported",
            manifest.schema_version
        )));
    }
    validate_idempotency_key(&manifest.idempotency_key)?;
    require_non_empty_object("plan supersede rationale", &manifest.rationale)?;
    validate_evolution_append_manifests(&manifest.tasks, &manifest.records, &manifest.evidence)?;

    let manifest_value = evolution_manifest_to_canonical_value(&options.manifest)?;
    let payload_digest = Digest::domain_separated(
        EVOLUTION_PAYLOAD_DIGEST_DOMAIN,
        &canonical_bytes(&manifest_value)?,
    );
    let previous_head_commit_id = CommitId::parse_canonical(&manifest.expected_head_commit_id)?;
    let expected_state_digest = manifest
        .expected_state_digest
        .as_deref()
        .map(Digest::from_hex)
        .transpose()?;
    let target_plan_entity_id = EntityId::parse_canonical(&manifest.target_plan_entity_id)?;
    let expected_plan_entity_version_id =
        EntityVersionId::parse_canonical(&manifest.expected_plan_entity_version_id)?;
    let expected_plan_state_digest = Digest::from_hex(&manifest.expected_plan_state_digest)?;
    let expected_goal_entity_id = EntityId::parse_canonical(
        manifest
            .expected_goal_entity_id
            .as_deref()
            .expect("validated schema 1 goal id"),
    )?;
    let expected_goal_entity_version_id = EntityVersionId::parse_canonical(
        manifest
            .expected_goal_entity_version_id
            .as_deref()
            .expect("validated schema 1 goal version"),
    )?;
    let expected_goal_plan_relation_id = RelationId::parse_canonical(
        manifest
            .expected_goal_plan_relation_id
            .as_deref()
            .expect("validated schema 1 relation id"),
    )?;
    let expected_goal_plan_relation_version_id = RelationVersionId::parse_canonical(
        manifest
            .expected_goal_plan_relation_version_id
            .as_deref()
            .expect("validated schema 1 relation version"),
    )?;

    let current_plan = plan_at(connection, previous_head_commit_id, target_plan_entity_id)?;
    if current_plan.plan_entity_version_id != expected_plan_entity_version_id {
        return Err(WorkVcsError::PlanInvalid(format!(
            "plan entity {} expected version {}, found {} at commit {}",
            target_plan_entity_id,
            expected_plan_entity_version_id,
            current_plan.plan_entity_version_id,
            previous_head_commit_id
        )));
    }
    if current_plan.state_digest != expected_plan_state_digest {
        return Err(WorkVcsError::DigestInvalid(format!(
            "plan entity {} state digest {} does not match expected {} at commit {}",
            target_plan_entity_id,
            current_plan.state_digest,
            expected_plan_state_digest,
            previous_head_commit_id
        )));
    }
    if current_plan.state.status != PlanStatus::Active {
        return Err(WorkVcsError::PlanInvalid(format!(
            "plan entity {} is {}, not active",
            target_plan_entity_id, current_plan.state.status
        )));
    }
    let goal_entity_id =
        resolve_single_goal_parent(connection, previous_head_commit_id, target_plan_entity_id)?;
    if goal_entity_id != expected_goal_entity_id {
        return Err(WorkVcsError::RelationInvalid(format!(
            "plan entity {target_plan_entity_id} primary goal parent is {goal_entity_id}, not expected {expected_goal_entity_id}"
        )));
    }
    let goal = goal_at(connection, previous_head_commit_id, expected_goal_entity_id)?;
    if goal.goal_entity_version_id != expected_goal_entity_version_id {
        return Err(WorkVcsError::PlanInvalid(format!(
            "goal entity {} expected version {}, found {} at commit {}",
            expected_goal_entity_id,
            expected_goal_entity_version_id,
            goal.goal_entity_version_id,
            previous_head_commit_id
        )));
    }
    if goal.state.status != GoalStatus::Active {
        return Err(WorkVcsError::PlanInvalid(format!(
            "goal entity {} is {}, not active",
            expected_goal_entity_id, goal.state.status
        )));
    }

    if manifest.plan.strategy == current_plan.state.strategy {
        return Err(WorkVcsError::PlanInvalid(
            "plan supersede replacement strategy must differ from the prior Plan strategy"
                .to_owned(),
        ));
    }
    let replacement_constraints = match &manifest.plan.constraints {
        PlanEvolutionSupersedeConstraintsManifest::CarryAll => {
            current_plan.state.constraints.clone()
        }
        PlanEvolutionSupersedeConstraintsManifest::Replace { values } => values.clone(),
    };
    let mut old_state = current_plan.state.clone();
    old_state.status = PlanStatus::Superseded;
    old_state.completion_rationale = None;
    let old_plan = prepared_existing_entity(
        PLAN_ENTITY_KIND,
        target_plan_entity_id,
        old_state.to_canonical_value()?,
    )?;
    let mut replacement_state = PlanState::active(
        manifest.plan.description.clone(),
        manifest.plan.strategy.clone(),
    )?;
    replacement_state.constraints = replacement_constraints;
    let new_plan = prepared_entity(PLAN_ENTITY_KIND, replacement_state.to_canonical_value()?)?;
    let tasks = prepare_tasks(&manifest.tasks)?;
    let records = prepare_records(&manifest.records)?;
    let evidence = prepare_evidence(&manifest.evidence)?;
    let relation_state = CanonicalValue::object(Vec::new())?;
    let relation_state_json = canonical_json_string(&relation_state)?;
    let relation_state_digest = relation_version_digest(&relation_state)?;
    let goal_contains_relation = PreparedRelation {
        relation_id: RelationId::new_v7(),
        relation_version_id: RelationVersionId::new_v7(),
        relation_type: CONTAINS_RELATION_TYPE.to_owned(),
        relation_discriminator: PRIMARY_CONTAINMENT_DISCRIMINATOR.to_owned(),
        source_entity_id: expected_goal_entity_id,
        target_entity_id: new_plan.entity_id,
        state_json: relation_state_json.clone(),
        state_digest: relation_state_digest,
    };
    let supersedes_relation = PreparedRelation {
        relation_id: RelationId::new_v7(),
        relation_version_id: RelationVersionId::new_v7(),
        relation_type: PLAN_SUPERSEDES_RELATION_TYPE.to_owned(),
        relation_discriminator: PLAN_SUPERSEDES_RELATION_DISCRIMINATOR.to_owned(),
        source_entity_id: new_plan.entity_id,
        target_entity_id: target_plan_entity_id,
        state_json: relation_state_json,
        state_digest: relation_state_digest,
    };
    Ok(PreparedEvolution {
        mode: "supersede",
        operation_schema_version: 1,
        plan_changed: true,
        branch_id: options.branch_id,
        previous_head_commit_id,
        expected_state_digest,
        idempotency_key: manifest.idempotency_key.clone(),
        payload_digest,
        rationale: manifest.rationale.clone(),
        goal_entity_id: Some(expected_goal_entity_id),
        parent_entity_id: Some(expected_goal_entity_id),
        parent_kind: Some(PrimaryContainmentEndpointKind::Goal),
        previous_plan_entity_version_id: expected_plan_entity_version_id,
        previous_plan_state_digest: expected_plan_state_digest,
        task_parent_entity_id: new_plan.entity_id,
        carrier_entity_id: None,
        carrier_kind: None,
        expected_carrier_entity_version_id: None,
        expected_carrier_state_digest: None,
        plan: old_plan,
        new_plan: Some(new_plan),
        expected_goal_entity_version_id: Some(expected_goal_entity_version_id),
        expected_goal_plan_relation_id: Some(expected_goal_plan_relation_id),
        expected_goal_plan_relation_version_id: Some(expected_goal_plan_relation_version_id),
        goal_contains_relation: Some(goal_contains_relation),
        supersedes_relation: Some(supersedes_relation),
        task_dispositions: Vec::new(),
        disposition_relations: Vec::new(),
        tasks,
        records,
        evidence,
    })
}

fn prepare_supersede_evolution_v2(
    connection: &StoreConnection,
    options: &PlanEvolutionOptions,
    manifest: &PlanEvolutionSupersedeManifest,
) -> Result<PreparedEvolution> {
    validate_idempotency_key(&manifest.idempotency_key)?;
    require_non_empty_object("plan supersede rationale", &manifest.rationale)?;
    validate_evolution_append_manifests(&manifest.tasks, &manifest.records, &manifest.evidence)?;
    validate_supersede_parent_shape(manifest)?;
    validate_task_disposition_manifest(manifest)?;
    validate_nested_plan_continuation_manifest(manifest)?;

    let manifest_value = evolution_manifest_to_canonical_value(&options.manifest)?;
    let payload_digest = Digest::domain_separated(
        EVOLUTION_V2_PAYLOAD_DIGEST_DOMAIN,
        &canonical_bytes(&manifest_value)?,
    );
    let previous_head_commit_id = CommitId::parse_canonical(&manifest.expected_head_commit_id)?;
    let expected_state_digest = manifest
        .expected_state_digest
        .as_deref()
        .map(Digest::from_hex)
        .transpose()?;
    let target_plan_entity_id = EntityId::parse_canonical(&manifest.target_plan_entity_id)?;
    let expected_plan_entity_version_id =
        EntityVersionId::parse_canonical(&manifest.expected_plan_entity_version_id)?;
    let expected_plan_state_digest = Digest::from_hex(&manifest.expected_plan_state_digest)?;
    let current_plan = plan_at(connection, previous_head_commit_id, target_plan_entity_id)?;
    if current_plan.plan_entity_version_id != expected_plan_entity_version_id {
        return Err(WorkVcsError::PlanInvalid(format!(
            "plan entity {target_plan_entity_id} expected version {expected_plan_entity_version_id}, found {} at commit {previous_head_commit_id}",
            current_plan.plan_entity_version_id
        )));
    }
    if current_plan.state_digest != expected_plan_state_digest {
        return Err(WorkVcsError::DigestInvalid(format!(
            "plan entity {target_plan_entity_id} state digest {} does not match expected {expected_plan_state_digest} at commit {previous_head_commit_id}",
            current_plan.state_digest
        )));
    }
    if current_plan.state.status != PlanStatus::Active {
        return Err(WorkVcsError::PlanInvalid(format!(
            "plan entity {target_plan_entity_id} is {}, not active",
            current_plan.state.status
        )));
    }
    if manifest.plan.strategy == current_plan.state.strategy {
        return Err(WorkVcsError::PlanInvalid(
            "plan supersede replacement strategy must differ from the prior Plan strategy"
                .to_owned(),
        ));
    }

    let before_current = current_scope_at(connection, previous_head_commit_id)?;
    if !before_current
        .current_plans
        .contains(&target_plan_entity_id)
    {
        return Err(WorkVcsError::PlanInvalid(format!(
            "plan entity {target_plan_entity_id} is active but not current at commit {previous_head_commit_id}"
        )));
    }
    let containment = primary_containment_relations_at(connection, previous_head_commit_id)?;
    let (
        parent_entity_id,
        parent_kind,
        parent_version_id,
        parent_relation_id,
        parent_relation_version_id,
    ) = validate_v2_parent_guard(
        connection,
        previous_head_commit_id,
        target_plan_entity_id,
        manifest
            .expected_parent
            .as_ref()
            .expect("validated v2 parent"),
        &containment,
        &before_current,
    )?;

    let replacement_constraints = match &manifest.plan.constraints {
        PlanEvolutionSupersedeConstraintsManifest::CarryAll => {
            current_plan.state.constraints.clone()
        }
        PlanEvolutionSupersedeConstraintsManifest::Replace { values } => values.clone(),
    };
    let mut retired_state = current_plan.state.clone();
    retired_state.status = PlanStatus::Superseded;
    retired_state.completion_rationale = None;
    let old_plan = prepared_existing_entity(
        PLAN_ENTITY_KIND,
        target_plan_entity_id,
        retired_state.to_canonical_value()?,
    )?;
    let mut replacement_state = PlanState::active(
        manifest.plan.description.clone(),
        manifest.plan.strategy.clone(),
    )?;
    replacement_state.constraints = replacement_constraints;
    let new_plan = prepared_entity(PLAN_ENTITY_KIND, replacement_state.to_canonical_value()?)?;
    let tasks = prepare_tasks(&manifest.tasks)?;
    let task_by_local_id = tasks
        .iter()
        .map(|task| (task.local_id.as_str(), task.entity.entity_id))
        .collect::<BTreeMap<_, _>>();
    let records = prepare_records(&manifest.records)?;
    let evidence = prepare_evidence(&manifest.evidence)?;

    let nested_plan_continuations = manifest
        .nested_plan_continuations
        .iter()
        .map(|item| {
            Ok(PreparedNestedPlanContinuation {
                plan_entity_id: EntityId::parse_canonical(&item.plan_entity_id)?,
                expected_plan_entity_version_id: EntityVersionId::parse_canonical(
                    &item.expected_plan_entity_version_id,
                )?,
                expected_plan_state_digest: Digest::from_hex(&item.expected_plan_state_digest)?,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let affected_tasks = collect_affected_tasks(
        connection,
        previous_head_commit_id,
        target_plan_entity_id,
        &containment,
        &nested_plan_continuations,
    )?;
    let disposition_ids = manifest
        .task_dispositions
        .iter()
        .map(|item| EntityId::parse_canonical(&item.task_entity_id))
        .collect::<Result<BTreeSet<_>>>()?;
    let affected_ids = affected_tasks.keys().copied().collect::<BTreeSet<_>>();
    if disposition_ids != affected_ids {
        let missing = affected_ids
            .difference(&disposition_ids)
            .map(ToString::to_string)
            .collect::<Vec<_>>();
        let extra = disposition_ids
            .difference(&affected_ids)
            .map(ToString::to_string)
            .collect::<Vec<_>>();
        return Err(WorkVcsError::PlanInvalid(format!(
            "task dispositions must exactly cover affected nonterminal tasks; missing=[{}] extra=[{}]",
            missing.join(","),
            extra.join(",")
        )));
    }

    let mut task_dispositions = Vec::new();
    for item in &manifest.task_dispositions {
        let task_entity_id = EntityId::parse_canonical(&item.task_entity_id)?;
        let snapshot = affected_tasks
            .get(&task_entity_id)
            .expect("exact coverage checked");
        let expected_version =
            EntityVersionId::parse_canonical(&item.expected_task_entity_version_id)?;
        let expected_digest = Digest::from_hex(&item.expected_task_state_digest)?;
        if snapshot.task_entity_version_id != expected_version {
            return Err(WorkVcsError::TaskInvalid(format!(
                "task {task_entity_id} expected version {expected_version}, found {}",
                snapshot.task_entity_version_id
            )));
        }
        if snapshot.state_digest != expected_digest {
            return Err(WorkVcsError::DigestInvalid(format!(
                "task {task_entity_id} expected state digest {expected_digest}, found {}",
                snapshot.state_digest
            )));
        }
        let replacement_task_entity_id = item
            .replacement_task_local_id
            .as_deref()
            .map(|local_id| task_by_local_id[local_id]);
        let next_entity = match item.action {
            PlanTaskDispositionAction::Supersede | PlanTaskDispositionAction::Cancel => {
                let mut state = snapshot.state.clone();
                state.status = if item.action == PlanTaskDispositionAction::Supersede {
                    TaskStatus::Superseded
                } else {
                    TaskStatus::Cancelled
                };
                Some(prepared_existing_entity(
                    TASK_ENTITY_KIND,
                    task_entity_id,
                    state.to_canonical_value()?,
                )?)
            }
            PlanTaskDispositionAction::Continue | PlanTaskDispositionAction::RetainOriginal => None,
        };
        task_dispositions.push(PreparedTaskDisposition {
            task_entity_id,
            previous_entity_version_id: expected_version,
            previous_state_digest: expected_digest,
            action: item.action,
            replacement_task_local_id: item.replacement_task_local_id.clone(),
            replacement_task_entity_id,
            next_entity,
            return_condition: item.return_condition.clone(),
            rationale: item.rationale.clone(),
            pre_current: before_current.current_tasks.contains(&task_entity_id),
            post_current: false,
            post_carriers: Vec::new(),
            risks: Vec::new(),
        });
    }

    let relation_state = CanonicalValue::object(Vec::new())?;
    let relation_state_json = canonical_json_string(&relation_state)?;
    let relation_state_digest = relation_version_digest(&relation_state)?;
    let parent_contains_relation = parent_entity_id.map(|parent_id| PreparedRelation {
        relation_id: RelationId::new_v7(),
        relation_version_id: RelationVersionId::new_v7(),
        relation_type: CONTAINS_RELATION_TYPE.to_owned(),
        relation_discriminator: PRIMARY_CONTAINMENT_DISCRIMINATOR.to_owned(),
        source_entity_id: parent_id,
        target_entity_id: new_plan.entity_id,
        state_json: relation_state_json.clone(),
        state_digest: relation_state_digest,
    });
    let supersedes_relation = PreparedRelation {
        relation_id: RelationId::new_v7(),
        relation_version_id: RelationVersionId::new_v7(),
        relation_type: PLAN_SUPERSEDES_RELATION_TYPE.to_owned(),
        relation_discriminator: PLAN_SUPERSEDES_RELATION_DISCRIMINATOR.to_owned(),
        source_entity_id: new_plan.entity_id,
        target_entity_id: target_plan_entity_id,
        state_json: relation_state_json,
        state_digest: relation_state_digest,
    };
    let mut disposition_relations = Vec::new();
    for disposition in &task_dispositions {
        let (relation_type, relation_discriminator, source_entity_id) = match disposition.action {
            PlanTaskDispositionAction::Supersede => (
                PLAN_SUPERSEDES_RELATION_TYPE,
                TASK_SUPERSEDES_RELATION_DISCRIMINATOR,
                disposition
                    .replacement_task_entity_id
                    .expect("validated replacement task"),
            ),
            PlanTaskDispositionAction::Continue => {
                (REFERENCES_RELATION_TYPE, "", new_plan.entity_id)
            }
            PlanTaskDispositionAction::Cancel | PlanTaskDispositionAction::RetainOriginal => {
                continue;
            }
        };
        disposition_relations.push(PreparedRelation {
            relation_id: RelationId::new_v7(),
            relation_version_id: RelationVersionId::new_v7(),
            relation_type: relation_type.to_owned(),
            relation_discriminator: relation_discriminator.to_owned(),
            source_entity_id,
            target_entity_id: disposition.task_entity_id,
            state_json: canonical_json_string(&CanonicalValue::object(Vec::new())?)?,
            state_digest: relation_state_digest,
        });
    }
    for continuation in &nested_plan_continuations {
        disposition_relations.push(PreparedRelation {
            relation_id: RelationId::new_v7(),
            relation_version_id: RelationVersionId::new_v7(),
            relation_type: REFERENCES_RELATION_TYPE.to_owned(),
            relation_discriminator: String::new(),
            source_entity_id: new_plan.entity_id,
            target_entity_id: continuation.plan_entity_id,
            state_json: canonical_json_string(&CanonicalValue::object(Vec::new())?)?,
            state_digest: relation_state_digest,
        });
    }

    let mut prepared = PreparedEvolution {
        mode: "supersede",
        operation_schema_version: 2,
        plan_changed: true,
        branch_id: options.branch_id,
        previous_head_commit_id,
        expected_state_digest,
        idempotency_key: manifest.idempotency_key.clone(),
        payload_digest,
        rationale: manifest.rationale.clone(),
        goal_entity_id: match parent_kind {
            Some(PrimaryContainmentEndpointKind::Goal) => parent_entity_id,
            _ => None,
        },
        parent_entity_id,
        parent_kind,
        previous_plan_entity_version_id: expected_plan_entity_version_id,
        previous_plan_state_digest: expected_plan_state_digest,
        task_parent_entity_id: new_plan.entity_id,
        carrier_entity_id: None,
        carrier_kind: None,
        expected_carrier_entity_version_id: None,
        expected_carrier_state_digest: None,
        plan: old_plan,
        new_plan: Some(new_plan),
        expected_goal_entity_version_id: parent_version_id,
        expected_goal_plan_relation_id: parent_relation_id,
        expected_goal_plan_relation_version_id: parent_relation_version_id,
        goal_contains_relation: parent_contains_relation,
        supersedes_relation: Some(supersedes_relation),
        task_dispositions,
        disposition_relations,
        tasks,
        records,
        evidence,
    };
    evaluate_post_disposition_currentness(connection, &mut prepared, &before_current)?;
    Ok(prepared)
}

fn prepare_task_disposition_repair(
    connection: &StoreConnection,
    options: &PlanEvolutionOptions,
    manifest: &PlanTaskDispositionRepairManifest,
) -> Result<PreparedEvolution> {
    if manifest.schema_version != 2 {
        return Err(WorkVcsError::PlanInvalid(format!(
            "task disposition repair manifest schema_version {} is not supported",
            manifest.schema_version
        )));
    }
    validate_idempotency_key(&manifest.idempotency_key)?;
    require_non_empty_object("task disposition repair rationale", &manifest.rationale)?;
    validate_evolution_append_manifests(&manifest.tasks, &[], &[])?;
    validate_task_disposition_items(&manifest.tasks, &manifest.task_dispositions)?;

    let manifest_value = evolution_manifest_to_canonical_value(&options.manifest)?;
    let payload_digest = Digest::domain_separated(
        EVOLUTION_V2_PAYLOAD_DIGEST_DOMAIN,
        &canonical_bytes(&manifest_value)?,
    );
    let previous_head_commit_id = CommitId::parse_canonical(&manifest.expected_head_commit_id)?;
    let expected_state_digest = manifest
        .expected_state_digest
        .as_deref()
        .map(Digest::from_hex)
        .transpose()?;
    let target_plan_entity_id = EntityId::parse_canonical(&manifest.target_plan_entity_id)?;
    let expected_plan_entity_version_id =
        EntityVersionId::parse_canonical(&manifest.expected_plan_entity_version_id)?;
    let expected_plan_state_digest = Digest::from_hex(&manifest.expected_plan_state_digest)?;
    let current_plan = plan_at(connection, previous_head_commit_id, target_plan_entity_id)?;
    if current_plan.plan_entity_version_id != expected_plan_entity_version_id {
        return Err(WorkVcsError::PlanInvalid(format!(
            "repair target Plan {target_plan_entity_id} expected version {expected_plan_entity_version_id}, found {}",
            current_plan.plan_entity_version_id
        )));
    }
    if current_plan.state_digest != expected_plan_state_digest {
        return Err(WorkVcsError::DigestInvalid(format!(
            "repair target Plan {target_plan_entity_id} expected state digest {expected_plan_state_digest}, found {}",
            current_plan.state_digest
        )));
    }
    if !matches!(
        current_plan.state.status,
        PlanStatus::Superseded | PlanStatus::Abandoned
    ) {
        return Err(WorkVcsError::PlanInvalid(format!(
            "task disposition repair requires a superseded or abandoned Plan; {target_plan_entity_id} is {}",
            current_plan.state.status
        )));
    }

    let before_current = current_scope_at(connection, previous_head_commit_id)?;
    if before_current
        .current_plans
        .contains(&target_plan_entity_id)
    {
        return Err(WorkVcsError::IntegrityInvalid(format!(
            "retired repair target Plan {target_plan_entity_id} is unexpectedly current"
        )));
    }
    let (carrier_entity_id, carrier_kind, carrier_version_id, carrier_state_digest) = match manifest
        .carrier
        .as_ref()
    {
        Some(PlanTaskDispositionCarrierManifest::Goal {
            entity_id,
            expected_entity_version_id,
            expected_state_digest,
        }) => {
            let entity_id = EntityId::parse_canonical(entity_id)?;
            let expected_version_id = EntityVersionId::parse_canonical(expected_entity_version_id)?;
            let expected_digest = Digest::from_hex(expected_state_digest)?;
            let goal = goal_at(connection, previous_head_commit_id, entity_id)?;
            if goal.goal_entity_version_id != expected_version_id
                || goal.state_digest != expected_digest
                || goal.state.status != GoalStatus::Active
                || !before_current.current_goals.contains(&entity_id)
            {
                return Err(WorkVcsError::PlanInvalid(format!(
                    "repair carrier Goal {entity_id} does not match the guarded current active version"
                )));
            }
            (
                Some(entity_id),
                Some(PrimaryContainmentEndpointKind::Goal),
                Some(expected_version_id),
                Some(expected_digest),
            )
        }
        Some(PlanTaskDispositionCarrierManifest::Plan {
            entity_id,
            expected_entity_version_id,
            expected_state_digest,
        }) => {
            let entity_id = EntityId::parse_canonical(entity_id)?;
            let expected_version_id = EntityVersionId::parse_canonical(expected_entity_version_id)?;
            let expected_digest = Digest::from_hex(expected_state_digest)?;
            let carrier = plan_at(connection, previous_head_commit_id, entity_id)?;
            if carrier.plan_entity_version_id != expected_version_id
                || carrier.state_digest != expected_digest
                || carrier.state.status != PlanStatus::Active
                || !before_current.current_plans.contains(&entity_id)
            {
                return Err(WorkVcsError::PlanInvalid(format!(
                    "repair carrier Plan {entity_id} does not match the guarded current active version"
                )));
            }
            (
                Some(entity_id),
                Some(PrimaryContainmentEndpointKind::Plan),
                Some(expected_version_id),
                Some(expected_digest),
            )
        }
        None => (None, None, None, None),
    };
    let needs_carrier = !manifest.tasks.is_empty()
        || manifest.task_dispositions.iter().any(|item| {
            matches!(
                item.action,
                PlanTaskDispositionAction::Supersede | PlanTaskDispositionAction::Continue
            )
        });
    if needs_carrier && carrier_entity_id.is_none() {
        return Err(WorkVcsError::PlanInvalid(
            "task disposition repair requires a current carrier for continue or replacement tasks"
                .to_owned(),
        ));
    }

    let containment = primary_containment_relations_at(connection, previous_head_commit_id)?;
    let affected_tasks = collect_repair_affected_tasks(
        connection,
        previous_head_commit_id,
        target_plan_entity_id,
        &containment,
    )?;
    let disposition_ids = manifest
        .task_dispositions
        .iter()
        .map(|item| EntityId::parse_canonical(&item.task_entity_id))
        .collect::<Result<BTreeSet<_>>>()?;
    let affected_ids = affected_tasks.keys().copied().collect::<BTreeSet<_>>();
    if disposition_ids != affected_ids {
        let missing = affected_ids
            .difference(&disposition_ids)
            .map(ToString::to_string)
            .collect::<Vec<_>>();
        let extra = disposition_ids
            .difference(&affected_ids)
            .map(ToString::to_string)
            .collect::<Vec<_>>();
        return Err(WorkVcsError::PlanInvalid(format!(
            "repair task dispositions must exactly cover affected nonterminal tasks; missing=[{}] extra=[{}]",
            missing.join(","),
            extra.join(",")
        )));
    }

    let tasks = prepare_tasks(&manifest.tasks)?;
    let task_by_local_id = tasks
        .iter()
        .map(|task| (task.local_id.as_str(), task.entity.entity_id))
        .collect::<BTreeMap<_, _>>();
    let mut task_dispositions = Vec::new();
    for item in &manifest.task_dispositions {
        let task_entity_id = EntityId::parse_canonical(&item.task_entity_id)?;
        let snapshot = affected_tasks
            .get(&task_entity_id)
            .expect("exact repair coverage checked");
        let expected_version =
            EntityVersionId::parse_canonical(&item.expected_task_entity_version_id)?;
        let expected_digest = Digest::from_hex(&item.expected_task_state_digest)?;
        if snapshot.task_entity_version_id != expected_version {
            return Err(WorkVcsError::TaskInvalid(format!(
                "repair task {task_entity_id} expected version {expected_version}, found {}",
                snapshot.task_entity_version_id
            )));
        }
        if snapshot.state_digest != expected_digest {
            return Err(WorkVcsError::DigestInvalid(format!(
                "repair task {task_entity_id} expected state digest {expected_digest}, found {}",
                snapshot.state_digest
            )));
        }
        let replacement_task_entity_id = item
            .replacement_task_local_id
            .as_deref()
            .map(|local_id| task_by_local_id[local_id]);
        let next_entity = match item.action {
            PlanTaskDispositionAction::Supersede | PlanTaskDispositionAction::Cancel => {
                let mut state = snapshot.state.clone();
                state.status = if item.action == PlanTaskDispositionAction::Supersede {
                    TaskStatus::Superseded
                } else {
                    TaskStatus::Cancelled
                };
                Some(prepared_existing_entity(
                    TASK_ENTITY_KIND,
                    task_entity_id,
                    state.to_canonical_value()?,
                )?)
            }
            PlanTaskDispositionAction::Continue | PlanTaskDispositionAction::RetainOriginal => None,
        };
        task_dispositions.push(PreparedTaskDisposition {
            task_entity_id,
            previous_entity_version_id: expected_version,
            previous_state_digest: expected_digest,
            action: item.action,
            replacement_task_local_id: item.replacement_task_local_id.clone(),
            replacement_task_entity_id,
            next_entity,
            return_condition: item.return_condition.clone(),
            rationale: item.rationale.clone(),
            pre_current: before_current.current_tasks.contains(&task_entity_id),
            post_current: false,
            post_carriers: Vec::new(),
            risks: Vec::new(),
        });
    }

    let relation_state = CanonicalValue::object(Vec::new())?;
    let relation_state_json = canonical_json_string(&relation_state)?;
    let relation_state_digest = relation_version_digest(&relation_state)?;
    let mut disposition_relations = Vec::new();
    for disposition in &task_dispositions {
        let (relation_type, relation_discriminator, source_entity_id) = match disposition.action {
            PlanTaskDispositionAction::Supersede => (
                PLAN_SUPERSEDES_RELATION_TYPE,
                TASK_SUPERSEDES_RELATION_DISCRIMINATOR,
                disposition
                    .replacement_task_entity_id
                    .expect("validated repair replacement task"),
            ),
            PlanTaskDispositionAction::Continue => (
                REFERENCES_RELATION_TYPE,
                "",
                carrier_entity_id.expect("continue repair has carrier"),
            ),
            PlanTaskDispositionAction::Cancel | PlanTaskDispositionAction::RetainOriginal => {
                continue;
            }
        };
        disposition_relations.push(PreparedRelation {
            relation_id: RelationId::new_v7(),
            relation_version_id: RelationVersionId::new_v7(),
            relation_type: relation_type.to_owned(),
            relation_discriminator: relation_discriminator.to_owned(),
            source_entity_id,
            target_entity_id: disposition.task_entity_id,
            state_json: relation_state_json.clone(),
            state_digest: relation_state_digest,
        });
    }

    let plan_state = current_plan.state.to_canonical_value()?;
    let unchanged_plan = PreparedEntity {
        entity_id: target_plan_entity_id,
        entity_version_id: expected_plan_entity_version_id,
        entity_kind: PLAN_ENTITY_KIND,
        state: plan_state,
        state_digest: expected_plan_state_digest,
    };
    let mut prepared = PreparedEvolution {
        mode: "task_disposition",
        operation_schema_version: 2,
        plan_changed: false,
        branch_id: options.branch_id,
        previous_head_commit_id,
        expected_state_digest,
        idempotency_key: manifest.idempotency_key.clone(),
        payload_digest,
        rationale: manifest.rationale.clone(),
        goal_entity_id: if carrier_kind == Some(PrimaryContainmentEndpointKind::Goal) {
            carrier_entity_id
        } else {
            None
        },
        parent_entity_id: carrier_entity_id,
        parent_kind: carrier_kind,
        previous_plan_entity_version_id: expected_plan_entity_version_id,
        previous_plan_state_digest: expected_plan_state_digest,
        task_parent_entity_id: carrier_entity_id.unwrap_or(target_plan_entity_id),
        carrier_entity_id,
        carrier_kind,
        expected_carrier_entity_version_id: carrier_version_id,
        expected_carrier_state_digest: carrier_state_digest,
        plan: unchanged_plan,
        new_plan: None,
        expected_goal_entity_version_id: None,
        expected_goal_plan_relation_id: None,
        expected_goal_plan_relation_version_id: None,
        goal_contains_relation: None,
        supersedes_relation: None,
        task_dispositions,
        disposition_relations,
        tasks,
        records: Vec::new(),
        evidence: Vec::new(),
    };
    evaluate_post_disposition_currentness(connection, &mut prepared, &before_current)?;
    Ok(prepared)
}

fn collect_repair_affected_tasks(
    connection: &StoreConnection,
    commit_id: CommitId,
    target_plan_entity_id: EntityId,
    containment: &[super::PrimaryContainmentSnapshot],
) -> Result<BTreeMap<EntityId, super::TaskSnapshot>> {
    let mut children = BTreeMap::<EntityId, Vec<(EntityId, PrimaryContainmentEndpointKind)>>::new();
    for relation in containment {
        children
            .entry(relation.parent_entity_id)
            .or_default()
            .push((relation.child_entity_id, relation.child_kind));
    }
    let mut seen = BTreeSet::new();
    let mut affected = BTreeMap::new();
    let mut stack = children
        .get(&target_plan_entity_id)
        .cloned()
        .unwrap_or_default();
    while let Some((entity_id, kind)) = stack.pop() {
        if !seen.insert(entity_id) {
            continue;
        }
        match kind {
            PrimaryContainmentEndpointKind::Goal => {
                return Err(WorkVcsError::IntegrityInvalid(format!(
                    "repair Plan scope contains unexpected Goal child {entity_id}"
                )));
            }
            PrimaryContainmentEndpointKind::Plan => {
                plan_at(connection, commit_id, entity_id)?;
                if let Some(descendants) = children.get(&entity_id) {
                    stack.extend(descendants.iter().copied());
                }
            }
            PrimaryContainmentEndpointKind::Task => {
                let snapshot = task_at(connection, commit_id, entity_id)?;
                if !snapshot.state.status.is_terminal() {
                    affected.insert(entity_id, snapshot);
                }
                if let Some(descendants) = children.get(&entity_id) {
                    stack.extend(descendants.iter().copied());
                }
            }
        }
    }
    Ok(affected)
}

type ValidatedParentGuard = (
    Option<EntityId>,
    Option<PrimaryContainmentEndpointKind>,
    Option<EntityVersionId>,
    Option<RelationId>,
    Option<RelationVersionId>,
);

fn validate_v2_parent_guard(
    connection: &StoreConnection,
    commit_id: CommitId,
    target_plan_entity_id: EntityId,
    expected: &PlanEvolutionExpectedParentManifest,
    containment: &[super::PrimaryContainmentSnapshot],
    current: &super::CurrentScopeProjection,
) -> Result<ValidatedParentGuard> {
    let parents = containment
        .iter()
        .filter(|relation| relation.child_entity_id == target_plan_entity_id)
        .collect::<Vec<_>>();
    match expected {
        PlanEvolutionExpectedParentManifest::None => {
            if !parents.is_empty() {
                return Err(WorkVcsError::RelationInvalid(format!(
                    "standalone plan {target_plan_entity_id} unexpectedly has {} primary parent relation(s)",
                    parents.len()
                )));
            }
            Ok((None, None, None, None, None))
        }
        PlanEvolutionExpectedParentManifest::Goal {
            entity_id,
            expected_entity_version_id,
            containment_relation_id,
            containment_relation_version_id,
        }
        | PlanEvolutionExpectedParentManifest::Plan {
            entity_id,
            expected_entity_version_id,
            containment_relation_id,
            containment_relation_version_id,
        } => {
            if parents.len() != 1 {
                return Err(WorkVcsError::RelationInvalid(format!(
                    "plan {target_plan_entity_id} expected one primary parent, found {}",
                    parents.len()
                )));
            }
            let entity_id = EntityId::parse_canonical(entity_id)?;
            let expected_version = EntityVersionId::parse_canonical(expected_entity_version_id)?;
            let relation_id = RelationId::parse_canonical(containment_relation_id)?;
            let relation_version_id =
                RelationVersionId::parse_canonical(containment_relation_version_id)?;
            let relation = parents[0];
            if relation.parent_entity_id != entity_id
                || relation.relation_id != relation_id
                || relation.relation_version_id != relation_version_id
            {
                return Err(WorkVcsError::RelationInvalid(format!(
                    "plan {target_plan_entity_id} primary parent relation does not match expected parent guards"
                )));
            }
            let kind = match expected {
                PlanEvolutionExpectedParentManifest::Goal { .. } => {
                    let parent = goal_at(connection, commit_id, entity_id)?;
                    if parent.goal_entity_version_id != expected_version
                        || parent.state.status != GoalStatus::Active
                    {
                        return Err(WorkVcsError::PlanInvalid(format!(
                            "expected Goal parent {entity_id} is not the guarded active version"
                        )));
                    }
                    if !current.current_goals.contains(&entity_id) {
                        return Err(WorkVcsError::PlanInvalid(format!(
                            "expected Goal parent {entity_id} is not current"
                        )));
                    }
                    PrimaryContainmentEndpointKind::Goal
                }
                PlanEvolutionExpectedParentManifest::Plan { .. } => {
                    let parent = plan_at(connection, commit_id, entity_id)?;
                    if parent.plan_entity_version_id != expected_version
                        || parent.state.status != PlanStatus::Active
                    {
                        return Err(WorkVcsError::PlanInvalid(format!(
                            "expected Plan parent {entity_id} is not the guarded active version"
                        )));
                    }
                    if !current.current_plans.contains(&entity_id) {
                        return Err(WorkVcsError::PlanInvalid(format!(
                            "expected Plan parent {entity_id} is not current"
                        )));
                    }
                    PrimaryContainmentEndpointKind::Plan
                }
                PlanEvolutionExpectedParentManifest::None => unreachable!(),
            };
            Ok((
                Some(entity_id),
                Some(kind),
                Some(expected_version),
                Some(relation_id),
                Some(relation_version_id),
            ))
        }
    }
}

fn collect_affected_tasks(
    connection: &StoreConnection,
    commit_id: CommitId,
    target_plan_entity_id: EntityId,
    containment: &[super::PrimaryContainmentSnapshot],
    continuations: &[PreparedNestedPlanContinuation],
) -> Result<BTreeMap<EntityId, super::TaskSnapshot>> {
    let continuation_ids = continuations
        .iter()
        .map(|item| item.plan_entity_id)
        .collect::<BTreeSet<_>>();
    let mut children = BTreeMap::<EntityId, Vec<(EntityId, PrimaryContainmentEndpointKind)>>::new();
    for relation in containment {
        children
            .entry(relation.parent_entity_id)
            .or_default()
            .push((relation.child_entity_id, relation.child_kind));
    }
    let mut seen = BTreeSet::new();
    let mut seen_continuations = BTreeSet::new();
    let mut affected = BTreeMap::new();
    let mut stack = children
        .get(&target_plan_entity_id)
        .cloned()
        .unwrap_or_default();
    while let Some((entity_id, kind)) = stack.pop() {
        if !seen.insert(entity_id) {
            continue;
        }
        match kind {
            PrimaryContainmentEndpointKind::Goal => {
                return Err(WorkVcsError::IntegrityInvalid(format!(
                    "plan scope contains unexpected Goal child {entity_id}"
                )));
            }
            PrimaryContainmentEndpointKind::Plan => {
                let snapshot = plan_at(connection, commit_id, entity_id)?;
                if continuation_ids.contains(&entity_id) {
                    let guard = continuations
                        .iter()
                        .find(|item| item.plan_entity_id == entity_id)
                        .expect("known continuation");
                    if snapshot.plan_entity_version_id != guard.expected_plan_entity_version_id
                        || snapshot.state_digest != guard.expected_plan_state_digest
                        || snapshot.state.status != PlanStatus::Active
                    {
                        return Err(WorkVcsError::PlanInvalid(format!(
                            "nested continuation plan {entity_id} does not match its guarded active state"
                        )));
                    }
                    seen_continuations.insert(entity_id);
                    continue;
                }
                if snapshot.state.status == PlanStatus::Active {
                    return Err(WorkVcsError::PlanInvalid(format!(
                        "active nested plan {entity_id} must be continued explicitly or superseded independently first"
                    )));
                }
                // Terminal nested Plans delimit non-current historical scope.
                continue;
            }
            PrimaryContainmentEndpointKind::Task => {
                let snapshot = task_at(connection, commit_id, entity_id)?;
                if !snapshot.state.status.is_terminal() {
                    affected.insert(entity_id, snapshot);
                }
                if let Some(descendants) = children.get(&entity_id) {
                    stack.extend(descendants.iter().copied());
                }
            }
        }
    }
    if seen_continuations != continuation_ids {
        let missing = continuation_ids
            .difference(&seen_continuations)
            .map(ToString::to_string)
            .collect::<Vec<_>>();
        return Err(WorkVcsError::PlanInvalid(format!(
            "nested plan continuations are outside the target Plan subtree: [{}]",
            missing.join(",")
        )));
    }
    Ok(affected)
}

fn evaluate_post_disposition_currentness(
    connection: &StoreConnection,
    prepared: &mut PreparedEvolution,
    before: &super::CurrentScopeProjection,
) -> Result<()> {
    let mut overrides = CurrentScopeOverrides::default();
    if prepared.plan_changed && prepared.mode == "supersede" {
        overrides.entity_statuses.insert(
            prepared.plan.entity_id,
            CurrentnessNodeStatus::Plan(PlanStatus::Superseded),
        );
    }
    if let Some(new_plan) = &prepared.new_plan {
        overrides.entity_statuses.insert(
            new_plan.entity_id,
            CurrentnessNodeStatus::Plan(PlanStatus::Active),
        );
    }
    if let Some(relation) = &prepared.goal_contains_relation {
        overrides.added_edges.push(CurrentnessEdge {
            source: relation.source_entity_id,
            target: relation.target_entity_id,
            relation_id: relation.relation_id,
            relation_version_id: relation.relation_version_id,
            carrier_kind: CurrentnessCarrierKind::PrimaryContainment,
        });
    }
    for task in &prepared.tasks {
        overrides.entity_statuses.insert(
            task.entity.entity_id,
            CurrentnessNodeStatus::Task(TaskStatus::Pending),
        );
        overrides.added_edges.push(CurrentnessEdge {
            source: prepared.task_parent_entity_id,
            target: task.entity.entity_id,
            relation_id: RelationId::new_v7(),
            relation_version_id: RelationVersionId::new_v7(),
            carrier_kind: CurrentnessCarrierKind::PrimaryContainment,
        });
    }
    for disposition in &prepared.task_dispositions {
        match disposition.action {
            PlanTaskDispositionAction::Supersede => {
                overrides.entity_statuses.insert(
                    disposition.task_entity_id,
                    CurrentnessNodeStatus::Task(TaskStatus::Superseded),
                );
            }
            PlanTaskDispositionAction::Cancel => {
                overrides.entity_statuses.insert(
                    disposition.task_entity_id,
                    CurrentnessNodeStatus::Task(TaskStatus::Cancelled),
                );
            }
            PlanTaskDispositionAction::Continue => {}
            PlanTaskDispositionAction::RetainOriginal => {}
        }
    }
    for relation in &prepared.disposition_relations {
        if relation.relation_type == REFERENCES_RELATION_TYPE {
            overrides.added_edges.push(CurrentnessEdge {
                source: relation.source_entity_id,
                target: relation.target_entity_id,
                relation_id: relation.relation_id,
                relation_version_id: relation.relation_version_id,
                carrier_kind: CurrentnessCarrierKind::StructuralReference,
            });
        }
    }
    let post =
        current_scope_at_with_overrides(connection, prepared.previous_head_commit_id, &overrides)?;
    let references = structural_references_at(connection, prepared.previous_head_commit_id)?;
    let scheduling = task_scheduling_relations_at(connection, prepared.previous_head_commit_id)?;
    let affected_ids = prepared
        .task_dispositions
        .iter()
        .map(|item| item.task_entity_id)
        .collect::<BTreeSet<_>>();
    for disposition in &mut prepared.task_dispositions {
        disposition.post_current = post.current_tasks.contains(&disposition.task_entity_id);
        disposition.post_carriers = post
            .carriers_for(disposition.task_entity_id)
            .iter()
            .map(|carrier| {
                format!(
                    "{}:{}:{}",
                    carrier.kind.as_str(),
                    carrier
                        .source_entity_id
                        .map(|id| id.to_string())
                        .unwrap_or_else(|| "none".to_owned()),
                    carrier
                        .relation_id
                        .map(|id| id.to_string())
                        .unwrap_or_else(|| "none".to_owned())
                )
            })
            .collect();
        let active_claims = active_claim_count(
            connection.inner(),
            prepared.branch_id,
            disposition.task_entity_id,
        )?;
        if active_claims > 0 {
            match disposition.action {
                PlanTaskDispositionAction::Supersede | PlanTaskDispositionAction::Cancel => {
                    return Err(WorkVcsError::ClaimInvalid(format!(
                        "task {} has {active_claims} active claim(s); {} disposition is fenced",
                        disposition.task_entity_id,
                        disposition.action.as_str()
                    )));
                }
                PlanTaskDispositionAction::RetainOriginal
                    if before.current_tasks.contains(&disposition.task_entity_id)
                        && !disposition.post_current =>
                {
                    return Err(WorkVcsError::ClaimInvalid(format!(
                        "retain_original would leave claimed task {} outside current scope",
                        disposition.task_entity_id
                    )));
                }
                PlanTaskDispositionAction::RetainOriginal if !disposition.post_current => {
                    disposition
                        .risks
                        .push("existing_active_claim_on_noncurrent_task".to_owned());
                }
                PlanTaskDispositionAction::Continue | PlanTaskDispositionAction::RetainOriginal => {
                }
            }
        }
        if disposition.action == PlanTaskDispositionAction::Continue && !disposition.post_current {
            return Err(WorkVcsError::IntegrityInvalid(format!(
                "continue disposition did not preserve current task {}",
                disposition.task_entity_id
            )));
        }
        if matches!(
            disposition.action,
            PlanTaskDispositionAction::Supersede | PlanTaskDispositionAction::Cancel
        ) {
            for reference in references
                .iter()
                .filter(|reference| reference.target_entity_id == disposition.task_entity_id)
            {
                let source_current = match reference.referrer_kind {
                    super::StructuralReferenceEndpointKind::Goal => {
                        post.current_goals.contains(&reference.referrer_entity_id)
                    }
                    super::StructuralReferenceEndpointKind::Plan => {
                        post.current_plans.contains(&reference.referrer_entity_id)
                    }
                    super::StructuralReferenceEndpointKind::Task => false,
                };
                if source_current {
                    return Err(WorkVcsError::RelationInvalid(format!(
                        "task {} is referenced by surviving current {} {}; {} disposition is fenced",
                        disposition.task_entity_id,
                        reference.referrer_kind,
                        reference.referrer_entity_id,
                        disposition.action.as_str()
                    )));
                }
            }
        }
        for dependency in scheduling.iter().filter(|relation| {
            relation.relation_type == super::TaskSchedulingRelationType::DependsOn
                && relation.target_task_entity_id == disposition.task_entity_id
        }) {
            if post
                .current_tasks
                .contains(&dependency.source_task_entity_id)
                && (!disposition.post_current
                    || matches!(
                        disposition.action,
                        PlanTaskDispositionAction::Supersede | PlanTaskDispositionAction::Cancel
                    ))
            {
                if disposition.action == PlanTaskDispositionAction::RetainOriginal
                    && !before.current_tasks.contains(&disposition.task_entity_id)
                    && !disposition.post_current
                    && before
                        .current_tasks
                        .contains(&dependency.source_task_entity_id)
                {
                    disposition
                        .risks
                        .push("existing_current_dependency_on_noncurrent_task".to_owned());
                    continue;
                }
                return Err(WorkVcsError::RelationInvalid(format!(
                    "current dependent task {} still depends_on task {}; disposition would leave the dependency permanently unsatisfied",
                    dependency.source_task_entity_id, disposition.task_entity_id
                )));
            }
        }
        if disposition.action == PlanTaskDispositionAction::RetainOriginal
            && before.current_tasks.contains(&disposition.task_entity_id)
            && !disposition.post_current
        {
            // This is valid only when no live owner or current dependent remains.
            // The durable assessment makes the historical/deferred result explicit.
        }
    }
    // Ensure no duplicate/extra disposition can evade the stable-identity walk.
    if affected_ids.len() != prepared.task_dispositions.len() {
        return Err(WorkVcsError::PlanInvalid(
            "task dispositions are not unique by stable Task identity".to_owned(),
        ));
    }
    Ok(())
}

fn active_claim_count(
    connection: &Connection,
    branch_id: BranchId,
    task_entity_id: EntityId,
) -> Result<i64> {
    connection
        .query_row(
            "SELECT count(*) FROM claim_runtime INNER JOIN claim ON claim.claim_id = claim_runtime.claim_id WHERE claim.branch_id = ?1 AND claim.task_entity_id = ?2",
            params![&branch_id.raw_bytes()[..], &task_entity_id.raw_bytes()[..]],
            |row| row.get(0),
        )
        .map_err(storage_error)
}

fn prepare_admission(
    connection: &StoreConnection,
    options: &PlanAdmissionOptions,
) -> Result<PreparedAdmission> {
    let manifest = &options.manifest;
    if manifest.schema_version != 1 {
        return Err(WorkVcsError::PlanInvalid(format!(
            "plan admission manifest schema_version {} is not supported",
            manifest.schema_version
        )));
    }
    validate_idempotency_key(&manifest.idempotency_key)?;
    require_object("plan admission rationale", &manifest.rationale)?;
    validate_unique_local_ids("task", manifest.tasks.iter().map(|task| &task.local_id))?;
    validate_unique_local_ids(
        "record",
        manifest.records.iter().map(|record| &record.local_id),
    )?;
    validate_unique_local_ids(
        "evidence",
        manifest.evidence.iter().map(|evidence| &evidence.local_id),
    )?;
    for task in &manifest.tasks {
        validate_unique_local_ids(
            "acceptance criterion",
            task.acceptance_criteria
                .iter()
                .map(|criterion| &criterion.local_id),
        )?;
        for criterion in &task.acceptance_criteria {
            validate_unique_local_ids(
                "verification requirement",
                criterion
                    .verification_requirements
                    .iter()
                    .map(|requirement| &requirement.local_id),
            )?;
        }
    }

    let manifest_value = manifest_to_canonical_value(manifest)?;
    let payload_digest =
        Digest::domain_separated(PAYLOAD_DIGEST_DOMAIN, &canonical_bytes(&manifest_value)?);
    let previous_head_commit_id = CommitId::parse_canonical(&manifest.expected_head_commit_id)?;
    let expected_state_digest = manifest
        .expected_state_digest
        .as_deref()
        .map(Digest::from_hex)
        .transpose()?;

    let goal = match &manifest.goal {
        PlanAdmissionGoalManifest::Create { description } => {
            let state = super::goal::GoalState::active(description.clone())?;
            PreparedGoal::Create(prepared_entity(
                GOAL_ENTITY_KIND,
                state.to_canonical_value()?,
            )?)
        }
        PlanAdmissionGoalManifest::Existing {
            entity_id,
            expected_entity_version_id,
        } => {
            let entity_id = EntityId::parse_canonical(entity_id)?;
            let snapshot = goal_at(connection, previous_head_commit_id, entity_id)?;
            if snapshot.state.status != GoalStatus::Active {
                return Err(WorkVcsError::GoalInvalid(format!(
                    "existing goal {entity_id} is {}, not active",
                    snapshot.state.status
                )));
            }
            if let Some(expected_entity_version_id) = expected_entity_version_id {
                let expected_entity_version_id =
                    EntityVersionId::parse_canonical(expected_entity_version_id)?;
                if snapshot.goal_entity_version_id != expected_entity_version_id {
                    return Err(WorkVcsError::GoalInvalid(format!(
                        "existing goal {entity_id} expected version {}, found {} at commit {}",
                        expected_entity_version_id,
                        snapshot.goal_entity_version_id,
                        previous_head_commit_id
                    )));
                }
            }
            PreparedGoal::Existing {
                entity_id,
                entity_version_id: snapshot.goal_entity_version_id,
            }
        }
    };

    let mut plan_state = super::plan::PlanState::active(
        manifest.plan.description.clone(),
        manifest.plan.strategy.clone(),
    )?;
    plan_state.constraints = manifest.plan.constraints.clone();
    let plan = prepared_entity(PLAN_ENTITY_KIND, plan_state.to_canonical_value()?)?;
    let tasks = prepare_tasks(&manifest.tasks)?;
    let records = prepare_records(&manifest.records)?;
    let evidence = prepare_evidence(&manifest.evidence)?;

    Ok(PreparedAdmission {
        branch_id: options.branch_id,
        previous_head_commit_id,
        expected_state_digest,
        idempotency_key: manifest.idempotency_key.clone(),
        payload_digest,
        rationale: manifest.rationale.clone(),
        goal,
        plan,
        tasks,
        records,
        evidence,
    })
}

pub(super) fn prepare_tasks(
    manifest_tasks: &[PlanAdmissionTaskManifest],
) -> Result<Vec<PreparedTask>> {
    let mut tasks = Vec::new();
    for task in manifest_tasks {
        let acceptance_criteria = prepare_acceptance_criteria(&task.acceptance_criteria)?;
        let refs = acceptance_criteria
            .iter()
            .map(|criterion| {
                TaskAcceptanceCriterionRef::new(
                    criterion.local_id.clone(),
                    criterion.entity.entity_id,
                )
            })
            .collect::<Result<Vec<_>>>()?;
        let mut state = TaskState::pending(task.description.clone(), task.priority.unwrap_or(0))?;
        state.acceptance_criteria = refs;
        tasks.push(PreparedTask {
            local_id: task.local_id.clone(),
            entity: prepared_entity(TASK_ENTITY_KIND, state.to_canonical_value()?)?,
            acceptance_criteria,
        });
    }
    Ok(tasks)
}

fn prepare_acceptance_criteria(
    manifest_criteria: &[PlanAdmissionAcceptanceCriterionManifest],
) -> Result<Vec<PreparedAcceptanceCriterion>> {
    let mut criteria = Vec::new();
    for criterion in manifest_criteria {
        let verification_requirements =
            prepare_verification_requirements(&criterion.verification_requirements)?;
        let requirement_refs = verification_requirements
            .iter()
            .map(|requirement| {
                AcceptanceCriterionVerificationRequirementRef::new(
                    requirement.local_id.clone(),
                    requirement.entity.entity_id,
                )
            })
            .collect::<Result<Vec<_>>>()?;
        let classification = parse_acceptance_classification(&criterion.classification)?;
        let mut state = AcceptanceCriterionState::new(criterion.statement.clone(), classification)?;
        state.verification_requirements = requirement_refs;
        criteria.push(PreparedAcceptanceCriterion {
            local_id: criterion.local_id.clone(),
            entity: prepared_entity(
                ACCEPTANCE_CRITERION_ENTITY_KIND,
                state.to_canonical_value()?,
            )?,
            verification_requirements,
        });
    }
    Ok(criteria)
}

fn prepare_verification_requirements(
    manifest_requirements: &[PlanAdmissionVerificationRequirementManifest],
) -> Result<Vec<PreparedVerificationRequirement>> {
    manifest_requirements
        .iter()
        .map(|requirement| {
            let state = VerificationRequirementState::new(requirement.statement.clone())?;
            Ok(PreparedVerificationRequirement {
                local_id: requirement.local_id.clone(),
                entity: prepared_entity(
                    VERIFICATION_REQUIREMENT_ENTITY_KIND,
                    state.to_canonical_value()?,
                )?,
            })
        })
        .collect()
}

pub(super) fn prepare_records(
    manifest_records: &[PlanAdmissionRecordManifest],
) -> Result<Vec<PreparedRecord>> {
    manifest_records
        .iter()
        .map(|record| {
            let kind = parse_record_kind(&record.kind)?;
            require_object("record scope", &record.scope)?;
            let state = record_state(kind, record.statement.clone(), record.scope.clone())?;
            Ok(PreparedRecord {
                local_id: record.local_id.clone(),
                kind,
                entity: prepared_entity(RECORD_ENTITY_KIND, state.to_canonical_value()?)?,
            })
        })
        .collect()
}

pub(super) fn prepare_evidence(
    manifest_evidence: &[PlanAdmissionEvidenceManifest],
) -> Result<Vec<PreparedEvidence>> {
    manifest_evidence
        .iter()
        .map(|evidence| {
            validate_stored_text("evidence kind", &evidence.kind)?;
            require_object("evidence metadata", &evidence.metadata)?;
            Ok(PreparedEvidence {
                local_id: evidence.local_id.clone(),
                evidence_id: EvidenceId::new_v7(),
                evidence_kind: evidence.kind.clone(),
                metadata: evidence.metadata.clone(),
            })
        })
        .collect()
}

pub(super) fn prepared_entity(
    entity_kind: &'static str,
    state: CanonicalValue,
) -> Result<PreparedEntity> {
    let state_digest = entity_version_digest(&state)?;
    Ok(PreparedEntity {
        entity_id: EntityId::new_v7(),
        entity_version_id: EntityVersionId::new_v7(),
        entity_kind,
        state,
        state_digest,
    })
}

fn prepared_existing_entity(
    entity_kind: &'static str,
    entity_id: EntityId,
    state: CanonicalValue,
) -> Result<PreparedEntity> {
    let state_digest = entity_version_digest(&state)?;
    Ok(PreparedEntity {
        entity_id,
        entity_version_id: EntityVersionId::new_v7(),
        entity_kind,
        state,
        state_digest,
    })
}

fn prepare_relations(
    prepared: &PreparedAdmission,
    state_json: &str,
    state_digest: Digest,
) -> Result<Vec<PreparedRelation>> {
    let goal_entity_id = match &prepared.goal {
        PreparedGoal::Existing { entity_id, .. } => *entity_id,
        PreparedGoal::Create(entity) => entity.entity_id,
    };
    let mut relations = vec![PreparedRelation {
        relation_id: RelationId::new_v7(),
        relation_version_id: RelationVersionId::new_v7(),
        relation_type: CONTAINS_RELATION_TYPE.to_owned(),
        relation_discriminator: PRIMARY_CONTAINMENT_DISCRIMINATOR.to_owned(),
        source_entity_id: goal_entity_id,
        target_entity_id: prepared.plan.entity_id,
        state_json: state_json.to_owned(),
        state_digest,
    }];
    for task in &prepared.tasks {
        relations.push(PreparedRelation {
            relation_id: RelationId::new_v7(),
            relation_version_id: RelationVersionId::new_v7(),
            relation_type: CONTAINS_RELATION_TYPE.to_owned(),
            relation_discriminator: PRIMARY_CONTAINMENT_DISCRIMINATOR.to_owned(),
            source_entity_id: prepared.plan.entity_id,
            target_entity_id: task.entity.entity_id,
            state_json: state_json.to_owned(),
            state_digest,
        });
    }
    Ok(relations)
}

fn prepare_evolution_relations(
    prepared: &PreparedEvolution,
    tasks: &[PreparedTask],
    state_json: &str,
    state_digest: Digest,
) -> Vec<PreparedRelation> {
    let task_parent_entity_id = prepared.task_parent_entity_id;
    let mut relations = Vec::new();
    if let Some(relation) = &prepared.goal_contains_relation {
        relations.push(relation.clone());
    }
    if let Some(relation) = &prepared.supersedes_relation {
        relations.push(relation.clone());
    }
    relations.extend(prepared.disposition_relations.iter().cloned());
    relations.extend(tasks.iter().map(|task| PreparedRelation {
        relation_id: RelationId::new_v7(),
        relation_version_id: RelationVersionId::new_v7(),
        relation_type: CONTAINS_RELATION_TYPE.to_owned(),
        relation_discriminator: PRIMARY_CONTAINMENT_DISCRIMINATOR.to_owned(),
        source_entity_id: task_parent_entity_id,
        target_entity_id: task.entity.entity_id,
        state_json: state_json.to_owned(),
        state_digest,
    }));
    relations
}

fn admission_work_state(
    parent: &WorkState,
    prepared: &PreparedAdmission,
    relations: &[PreparedRelation],
) -> Result<WorkState> {
    let mut entities = parent
        .entities()
        .iter()
        .copied()
        .collect::<BTreeMap<_, _>>();
    if let PreparedGoal::Create(goal) = &prepared.goal {
        insert_entity_mapping(&mut entities, goal)?;
    }
    insert_entity_mapping(&mut entities, &prepared.plan)?;
    for task in &prepared.tasks {
        insert_entity_mapping(&mut entities, &task.entity)?;
        for criterion in &task.acceptance_criteria {
            insert_entity_mapping(&mut entities, &criterion.entity)?;
            for requirement in &criterion.verification_requirements {
                insert_entity_mapping(&mut entities, &requirement.entity)?;
            }
        }
    }
    for record in &prepared.records {
        insert_entity_mapping(&mut entities, &record.entity)?;
    }

    let mut relation_map = parent
        .relations()
        .iter()
        .copied()
        .collect::<BTreeMap<_, _>>();
    for relation in relations {
        if relation_map
            .insert(relation.relation_id, relation.relation_version_id)
            .is_some()
        {
            return Err(WorkVcsError::RelationInvalid(format!(
                "admission generated duplicate relation id {}",
                relation.relation_id
            )));
        }
    }
    WorkState::new(entities, relation_map)
}

fn evolution_work_state(
    parent: &WorkState,
    prepared: &PreparedEvolution,
    relations: &[PreparedRelation],
) -> Result<WorkState> {
    let mut entities = parent
        .entities()
        .iter()
        .copied()
        .collect::<BTreeMap<_, _>>();
    match entities.get(&prepared.plan.entity_id) {
        Some(current) if *current == prepared.previous_plan_entity_version_id => {}
        Some(current) => {
            return Err(WorkVcsError::PlanInvalid(format!(
                "plan entity {} expected WorkState version {}, found {}",
                prepared.plan.entity_id, prepared.previous_plan_entity_version_id, current
            )));
        }
        None => {
            return Err(WorkVcsError::PlanNotFound(format!(
                "plan entity {} is not present at commit {}",
                prepared.plan.entity_id, prepared.previous_head_commit_id
            )));
        }
    }
    if prepared.plan_changed {
        entities.insert(prepared.plan.entity_id, prepared.plan.entity_version_id);
    }
    if let Some(new_plan) = &prepared.new_plan {
        insert_entity_mapping(&mut entities, new_plan)?;
    }
    for disposition in &prepared.task_dispositions {
        if let Some(next_entity) = &disposition.next_entity {
            match entities.get(&disposition.task_entity_id) {
                Some(current) if *current == disposition.previous_entity_version_id => {
                    entities.insert(disposition.task_entity_id, next_entity.entity_version_id);
                }
                Some(current) => {
                    return Err(WorkVcsError::TaskInvalid(format!(
                        "task {} expected WorkState version {}, found {}",
                        disposition.task_entity_id, disposition.previous_entity_version_id, current
                    )));
                }
                None => {
                    return Err(WorkVcsError::TaskNotFound(format!(
                        "task {} is absent from target head {}",
                        disposition.task_entity_id, prepared.previous_head_commit_id
                    )));
                }
            }
        }
    }
    for task in &prepared.tasks {
        insert_entity_mapping(&mut entities, &task.entity)?;
        for criterion in &task.acceptance_criteria {
            insert_entity_mapping(&mut entities, &criterion.entity)?;
            for requirement in &criterion.verification_requirements {
                insert_entity_mapping(&mut entities, &requirement.entity)?;
            }
        }
    }
    for record in &prepared.records {
        insert_entity_mapping(&mut entities, &record.entity)?;
    }

    let mut relation_map = parent
        .relations()
        .iter()
        .copied()
        .collect::<BTreeMap<_, _>>();
    for relation in relations {
        if relation_map
            .insert(relation.relation_id, relation.relation_version_id)
            .is_some()
        {
            return Err(WorkVcsError::RelationInvalid(format!(
                "evolution generated duplicate relation id {}",
                relation.relation_id
            )));
        }
    }
    WorkState::new(entities, relation_map)
}

fn insert_entity_mapping(
    entities: &mut BTreeMap<EntityId, EntityVersionId>,
    entity: &PreparedEntity,
) -> Result<()> {
    if entities
        .insert(entity.entity_id, entity.entity_version_id)
        .is_some()
    {
        return Err(WorkVcsError::EntityTransitionInvalid(format!(
            "admission generated duplicate entity id {}",
            entity.entity_id
        )));
    }
    Ok(())
}

// The arguments mirror the immutable changeset envelope at this transaction boundary.
#[allow(clippy::too_many_arguments)]
fn write_admission(
    transaction: &Transaction<'_>,
    workspace_id: WorkspaceId,
    changeset_id: ChangeSetId,
    commit_id: CommitId,
    now_us: i64,
    work_state_digest: Digest,
    operation_payload_json: &str,
    rationale_json: &str,
    prepared: &PreparedAdmission,
    relations: &[PreparedRelation],
) -> Result<()> {
    for evidence in &prepared.evidence {
        write_evidence(transaction, evidence, now_us)?;
    }
    if let PreparedGoal::Create(goal) = &prepared.goal {
        write_entity(transaction, workspace_id, goal, now_us)?;
    }
    write_entity(transaction, workspace_id, &prepared.plan, now_us)?;
    for task in &prepared.tasks {
        write_entity(transaction, workspace_id, &task.entity, now_us)?;
        for criterion in &task.acceptance_criteria {
            write_entity(transaction, workspace_id, &criterion.entity, now_us)?;
            write_acceptance_criterion_identity(
                transaction,
                criterion.entity.entity_id,
                task.entity.entity_id,
                &criterion.local_id,
            )?;
            for requirement in &criterion.verification_requirements {
                write_entity(transaction, workspace_id, &requirement.entity, now_us)?;
                write_verification_requirement_identity(
                    transaction,
                    requirement.entity.entity_id,
                    criterion.entity.entity_id,
                    &requirement.local_id,
                )?;
            }
        }
    }
    for record in &prepared.records {
        write_entity(transaction, workspace_id, &record.entity, now_us)?;
    }
    for relation in relations {
        write_relation(transaction, workspace_id, relation, now_us)?;
    }

    let changeset_id_bytes = changeset_id.raw_bytes();
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
                ADMISSION_OPERATION_TYPE,
                ADMISSION_OPERATION_SCHEMA_VERSION,
                operation_payload_json,
                rationale_json,
                now_us
            ],
        )
        .map_err(storage_error)?;

    let mut ordinal = 0_i64;
    if let PreparedGoal::Create(goal) = &prepared.goal {
        write_entity_change_operation(transaction, changeset_id, ordinal, goal)?;
        ordinal += 1;
    }
    write_entity_change_operation(transaction, changeset_id, ordinal, &prepared.plan)?;
    ordinal += 1;
    for task in &prepared.tasks {
        write_entity_change_operation(transaction, changeset_id, ordinal, &task.entity)?;
        ordinal += 1;
        for criterion in &task.acceptance_criteria {
            write_entity_change_operation(transaction, changeset_id, ordinal, &criterion.entity)?;
            ordinal += 1;
            for requirement in &criterion.verification_requirements {
                write_entity_change_operation(
                    transaction,
                    changeset_id,
                    ordinal,
                    &requirement.entity,
                )?;
                ordinal += 1;
            }
        }
    }
    for record in &prepared.records {
        write_entity_change_operation(transaction, changeset_id, ordinal, &record.entity)?;
        ordinal += 1;
    }
    for relation in relations {
        write_relation_change_operation(transaction, changeset_id, ordinal, relation)?;
        ordinal += 1;
    }

    let commit_id_bytes = commit_id.raw_bytes();
    let work_state_digest_bytes = work_state_digest.as_bytes();
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
                NORMAL_COMMIT_KIND,
                &work_state_digest_bytes[..],
                now_us
            ],
        )
        .map_err(storage_error)?;
    transaction
        .execute(
            "INSERT INTO commit_parent(
                commit_id,
                parent_ordinal,
                parent_role,
                parent_commit_id
             )
             VALUES (?1, 0, ?2, ?3)",
            params![
                &commit_id_bytes[..],
                PRIMARY_PARENT_ROLE,
                &prepared.previous_head_commit_id.raw_bytes()[..]
            ],
        )
        .map_err(storage_error)?;
    transaction
        .execute(
            "INSERT INTO event(
                event_id,
                workspace_id,
                changeset_id,
                session_id,
                event_kind,
                occurred_at_us,
                payload_json
             )
             VALUES (?1, ?2, ?3, NULL, ?4, ?5, ?6)",
            params![
                &EventId::new_v7().raw_bytes()[..],
                &workspace_id_bytes[..],
                &changeset_id_bytes[..],
                ADMISSION_EVENT_KIND,
                now_us,
                operation_payload_json
            ],
        )
        .map_err(storage_error)?;
    Ok(())
}

// The arguments mirror the immutable changeset envelope at this transaction boundary.
#[allow(clippy::too_many_arguments)]
fn write_evolution(
    transaction: &Transaction<'_>,
    workspace_id: WorkspaceId,
    changeset_id: ChangeSetId,
    commit_id: CommitId,
    now_us: i64,
    work_state_digest: Digest,
    operation_payload_json: &str,
    rationale_json: &str,
    prepared: &PreparedEvolution,
    relations: &[PreparedRelation],
) -> Result<()> {
    for evidence in &prepared.evidence {
        write_evidence(transaction, evidence, now_us)?;
    }
    if prepared.plan_changed {
        write_entity_version(transaction, &prepared.plan)?;
    }
    for disposition in &prepared.task_dispositions {
        if let Some(next_entity) = &disposition.next_entity {
            write_entity_version(transaction, next_entity)?;
        }
    }
    if let Some(new_plan) = &prepared.new_plan {
        write_entity(transaction, workspace_id, new_plan, now_us)?;
    }
    for task in &prepared.tasks {
        write_entity(transaction, workspace_id, &task.entity, now_us)?;
        for criterion in &task.acceptance_criteria {
            write_entity(transaction, workspace_id, &criterion.entity, now_us)?;
            write_acceptance_criterion_identity(
                transaction,
                criterion.entity.entity_id,
                task.entity.entity_id,
                &criterion.local_id,
            )?;
            for requirement in &criterion.verification_requirements {
                write_entity(transaction, workspace_id, &requirement.entity, now_us)?;
                write_verification_requirement_identity(
                    transaction,
                    requirement.entity.entity_id,
                    criterion.entity.entity_id,
                    &requirement.local_id,
                )?;
            }
        }
    }
    for record in &prepared.records {
        write_entity(transaction, workspace_id, &record.entity, now_us)?;
    }
    for relation in relations {
        write_relation(transaction, workspace_id, relation, now_us)?;
    }

    let changeset_id_bytes = changeset_id.raw_bytes();
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
                EVOLUTION_OPERATION_TYPE,
                prepared.operation_schema_version,
                operation_payload_json,
                rationale_json,
                now_us
            ],
        )
        .map_err(storage_error)?;

    let mut ordinal = 0_i64;
    if prepared.plan_changed {
        write_entity_transition_change_operation(
            transaction,
            changeset_id,
            ordinal,
            prepared.plan.entity_id,
            Some(prepared.previous_plan_entity_version_id),
            Some(prepared.plan.entity_version_id),
        )?;
        ordinal += 1;
    }
    for disposition in &prepared.task_dispositions {
        if let Some(next_entity) = &disposition.next_entity {
            write_entity_transition_change_operation(
                transaction,
                changeset_id,
                ordinal,
                disposition.task_entity_id,
                Some(disposition.previous_entity_version_id),
                Some(next_entity.entity_version_id),
            )?;
            ordinal += 1;
        }
    }
    if let Some(new_plan) = &prepared.new_plan {
        write_entity_change_operation(transaction, changeset_id, ordinal, new_plan)?;
        ordinal += 1;
    }
    for task in &prepared.tasks {
        write_entity_change_operation(transaction, changeset_id, ordinal, &task.entity)?;
        ordinal += 1;
        for criterion in &task.acceptance_criteria {
            write_entity_change_operation(transaction, changeset_id, ordinal, &criterion.entity)?;
            ordinal += 1;
            for requirement in &criterion.verification_requirements {
                write_entity_change_operation(
                    transaction,
                    changeset_id,
                    ordinal,
                    &requirement.entity,
                )?;
                ordinal += 1;
            }
        }
    }
    for record in &prepared.records {
        write_entity_change_operation(transaction, changeset_id, ordinal, &record.entity)?;
        ordinal += 1;
    }
    for relation in relations {
        write_relation_change_operation(transaction, changeset_id, ordinal, relation)?;
        ordinal += 1;
    }

    let commit_id_bytes = commit_id.raw_bytes();
    let work_state_digest_bytes = work_state_digest.as_bytes();
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
                NORMAL_COMMIT_KIND,
                &work_state_digest_bytes[..],
                now_us
            ],
        )
        .map_err(storage_error)?;
    transaction
        .execute(
            "INSERT INTO commit_parent(
                commit_id,
                parent_ordinal,
                parent_role,
                parent_commit_id
             )
             VALUES (?1, 0, ?2, ?3)",
            params![
                &commit_id_bytes[..],
                PRIMARY_PARENT_ROLE,
                &prepared.previous_head_commit_id.raw_bytes()[..]
            ],
        )
        .map_err(storage_error)?;
    transaction
        .execute(
            "INSERT INTO event(
                event_id,
                workspace_id,
                changeset_id,
                session_id,
                event_kind,
                occurred_at_us,
                payload_json
             )
             VALUES (?1, ?2, ?3, NULL, ?4, ?5, ?6)",
            params![
                &EventId::new_v7().raw_bytes()[..],
                &workspace_id_bytes[..],
                &changeset_id_bytes[..],
                EVOLUTION_EVENT_KIND,
                now_us,
                operation_payload_json
            ],
        )
        .map_err(storage_error)?;
    Ok(())
}

pub(super) fn write_entity(
    transaction: &Transaction<'_>,
    workspace_id: WorkspaceId,
    entity: &PreparedEntity,
    now_us: i64,
) -> Result<()> {
    let entity_id_bytes = entity.entity_id.raw_bytes();
    let workspace_id_bytes = workspace_id.raw_bytes();
    transaction
        .execute(
            "INSERT INTO object_identity(object_id, object_kind, created_at_us)
             VALUES (?1, ?2, ?3)",
            params![&entity_id_bytes[..], ENTITY_OBJECT_KIND, now_us],
        )
        .map_err(storage_error)?;
    transaction
        .execute(
            "INSERT INTO entity(object_id, workspace_id, entity_kind)
             VALUES (?1, ?2, ?3)",
            params![
                &entity_id_bytes[..],
                &workspace_id_bytes[..],
                entity.entity_kind
            ],
        )
        .map_err(storage_error)?;
    write_entity_version(transaction, entity)?;
    if entity.entity_kind == RECORD_ENTITY_KIND {
        let state = record_state_kind(&entity.state)?;
        transaction
            .execute(
                "INSERT INTO record_identity(entity_id, record_kind)
                 VALUES (?1, ?2)",
                params![&entity_id_bytes[..], state.as_str()],
            )
            .map_err(storage_error)?;
    }
    Ok(())
}

pub(super) fn write_entity_version(
    transaction: &Transaction<'_>,
    entity: &PreparedEntity,
) -> Result<()> {
    let entity_version_id_bytes = entity.entity_version_id.raw_bytes();
    let entity_id_bytes = entity.entity_id.raw_bytes();
    let state_json = canonical_json_string(&entity.state)?;
    let state_digest_bytes = entity.state_digest.as_bytes();
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
                STATE_SCHEMA_VERSION,
                state_json,
                &state_digest_bytes[..]
            ],
        )
        .map_err(storage_error)?;
    Ok(())
}

pub(super) fn write_acceptance_criterion_identity(
    transaction: &Transaction<'_>,
    entity_id: EntityId,
    owner_entity_id: EntityId,
    local_key: &str,
) -> Result<()> {
    transaction
        .execute(
            "INSERT INTO acceptance_criterion_identity(entity_id, owner_entity_id, local_key)
             VALUES (?1, ?2, ?3)",
            params![
                &entity_id.raw_bytes()[..],
                &owner_entity_id.raw_bytes()[..],
                local_key
            ],
        )
        .map_err(storage_error)?;
    Ok(())
}

pub(super) fn write_verification_requirement_identity(
    transaction: &Transaction<'_>,
    entity_id: EntityId,
    owner_entity_id: EntityId,
    local_key: &str,
) -> Result<()> {
    transaction
        .execute(
            "INSERT INTO verification_requirement_identity(entity_id, owner_entity_id, local_key)
             VALUES (?1, ?2, ?3)",
            params![
                &entity_id.raw_bytes()[..],
                &owner_entity_id.raw_bytes()[..],
                local_key
            ],
        )
        .map_err(storage_error)?;
    Ok(())
}

pub(super) fn write_relation(
    transaction: &Transaction<'_>,
    workspace_id: WorkspaceId,
    relation: &PreparedRelation,
    now_us: i64,
) -> Result<()> {
    let relation_id_bytes = relation.relation_id.raw_bytes();
    let workspace_id_bytes = workspace_id.raw_bytes();
    let relation_version_id_bytes = relation.relation_version_id.raw_bytes();
    transaction
        .execute(
            "INSERT INTO object_identity(object_id, object_kind, created_at_us)
             VALUES (?1, ?2, ?3)",
            params![&relation_id_bytes[..], RELATION_OBJECT_KIND, now_us],
        )
        .map_err(storage_error)?;
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
                relation.relation_type.as_str(),
                &relation.source_entity_id.raw_bytes()[..],
                &relation.target_entity_id.raw_bytes()[..],
                relation.relation_discriminator.as_str()
            ],
        )
        .map_err(storage_error)?;
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
                RELATION_STATE_SCHEMA_VERSION,
                relation.state_json,
                &relation.state_digest.as_bytes()[..]
            ],
        )
        .map_err(storage_error)?;
    Ok(())
}

pub(super) fn write_evidence(
    transaction: &Transaction<'_>,
    evidence: &PreparedEvidence,
    captured_at_us: i64,
) -> Result<()> {
    let evidence_id_bytes = evidence.evidence_id.raw_bytes();
    let metadata_json = canonical_json_string(&evidence.metadata)?;
    transaction
        .execute(
            "INSERT INTO object_identity(object_id, object_kind, created_at_us)
             VALUES (?1, ?2, ?3)",
            params![&evidence_id_bytes[..], EVIDENCE_OBJECT_KIND, captured_at_us],
        )
        .map_err(storage_error)?;
    transaction
        .execute(
            "INSERT INTO evidence(
                evidence_id,
                evidence_kind,
                captured_at_us,
                source_session_id,
                metadata_json
             )
             VALUES (?1, ?2, ?3, NULL, ?4)",
            params![
                &evidence_id_bytes[..],
                evidence.evidence_kind,
                captured_at_us,
                metadata_json
            ],
        )
        .map_err(storage_error)?;
    Ok(())
}

pub(super) fn write_entity_change_operation(
    transaction: &Transaction<'_>,
    changeset_id: ChangeSetId,
    ordinal: i64,
    entity: &PreparedEntity,
) -> Result<()> {
    write_entity_transition_change_operation(
        transaction,
        changeset_id,
        ordinal,
        entity.entity_id,
        None,
        Some(entity.entity_version_id),
    )
}

pub(super) fn write_entity_transition_change_operation(
    transaction: &Transaction<'_>,
    changeset_id: ChangeSetId,
    ordinal: i64,
    entity_id: EntityId,
    before_entity_version_id: Option<EntityVersionId>,
    after_entity_version_id: Option<EntityVersionId>,
) -> Result<()> {
    let operation_id = OperationId::new_v7();
    let before_entity_version_id_bytes = before_entity_version_id.map(|id| id.raw_bytes().to_vec());
    let after_entity_version_id_bytes = after_entity_version_id.map(|id| id.raw_bytes().to_vec());
    let payload = entity_transition_payload_value(
        entity_id,
        before_entity_version_id,
        after_entity_version_id,
    )?;
    let payload_json = canonical_json_string(&payload)?;
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
                &operation_id.raw_bytes()[..],
                &changeset_id.raw_bytes()[..],
                ordinal,
                &entity_id.raw_bytes()[..],
                payload_json
            ],
        )
        .map_err(storage_error)?;
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
                &operation_id.raw_bytes()[..],
                &entity_id.raw_bytes()[..],
                before_entity_version_id_bytes.as_deref(),
                after_entity_version_id_bytes.as_deref(),
                EMPTY_FIELD_DELTA
            ],
        )
        .map_err(storage_error)?;
    Ok(())
}

pub(super) fn write_relation_change_operation(
    transaction: &Transaction<'_>,
    changeset_id: ChangeSetId,
    ordinal: i64,
    relation: &PreparedRelation,
) -> Result<()> {
    write_relation_transition_change_operation(
        transaction,
        changeset_id,
        ordinal,
        relation.relation_id,
        None,
        Some(relation.relation_version_id),
    )
}

pub(super) fn write_relation_transition_change_operation(
    transaction: &Transaction<'_>,
    changeset_id: ChangeSetId,
    ordinal: i64,
    relation_id: RelationId,
    before_relation_version_id: Option<RelationVersionId>,
    after_relation_version_id: Option<RelationVersionId>,
) -> Result<()> {
    let operation_id = OperationId::new_v7();
    let before_relation_version_id_bytes =
        before_relation_version_id.map(|id| id.raw_bytes().to_vec());
    let after_relation_version_id_bytes =
        after_relation_version_id.map(|id| id.raw_bytes().to_vec());
    let payload = relation_transition_payload_value(
        relation_id,
        before_relation_version_id,
        after_relation_version_id,
    )?;
    let payload_json = canonical_json_string(&payload)?;
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
                &operation_id.raw_bytes()[..],
                &changeset_id.raw_bytes()[..],
                ordinal,
                &relation_id.raw_bytes()[..],
                payload_json
            ],
        )
        .map_err(storage_error)?;
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
                &operation_id.raw_bytes()[..],
                &relation_id.raw_bytes()[..],
                before_relation_version_id_bytes.as_deref(),
                after_relation_version_id_bytes.as_deref(),
                EMPTY_FIELD_DELTA
            ],
        )
        .map_err(storage_error)?;
    Ok(())
}

pub(super) fn move_branch_head(
    transaction: &Transaction<'_>,
    branch_id: BranchId,
    expected_head_commit_id: CommitId,
    commit_id: CommitId,
    updated_at_us: i64,
) -> Result<()> {
    let moved = transaction
        .execute(
            "UPDATE branch
             SET head_commit_id = ?1
             WHERE branch_id = ?2
               AND head_commit_id = ?3",
            params![
                &commit_id.raw_bytes()[..],
                &branch_id.raw_bytes()[..],
                &expected_head_commit_id.raw_bytes()[..]
            ],
        )
        .map_err(storage_error)?;
    if moved != 1 {
        return Err(WorkVcsError::BranchHeadConflict(format!(
            "branch {branch_id} head changed before commit {commit_id} could be installed"
        )));
    }
    super::mark_branch_projection_not_materialized(transaction, branch_id, updated_at_us)?;
    Ok(())
}

fn find_idempotent_admission(
    connection: &Connection,
    workspace_id: WorkspaceId,
    branch_id: BranchId,
    idempotency_key: &str,
    payload_digest: Digest,
) -> Result<Option<PlanAdmissionResult>> {
    let mut statement = connection
        .prepare(
            "SELECT changeset.operation_payload_json
             FROM changeset
             JOIN workstate_commit
               ON workstate_commit.changeset_id = changeset.changeset_id
             WHERE changeset.workspace_id = ?1
               AND changeset.operation_type = ?2
             ORDER BY changeset.created_at_us, changeset.changeset_id",
        )
        .map_err(storage_error)?;
    let rows = statement
        .query_map(
            params![&workspace_id.raw_bytes()[..], ADMISSION_OPERATION_TYPE],
            |row| row.get::<_, String>(0),
        )
        .map_err(storage_error)?;
    let mut matched = None;
    for row in rows {
        let payload_json = row.map_err(storage_error)?;
        let value: serde_json::Value = serde_json::from_str(&payload_json).map_err(|error| {
            WorkVcsError::PlanInvalid(format!("stored plan admission payload is invalid: {error}"))
        })?;
        if value
            .get("idempotency_key")
            .and_then(|value| value.as_str())
            != Some(idempotency_key)
        {
            continue;
        }
        let stored_branch = required_json_str(&value, "branch_id")?;
        if stored_branch != branch_id.to_string() {
            return Err(WorkVcsError::PlanInvalid(format!(
                "idempotency key {idempotency_key:?} already belongs to branch {stored_branch}, not {branch_id}"
            )));
        }
        let stored_digest = Digest::from_hex(required_json_str(&value, "payload_digest")?)?;
        if stored_digest != payload_digest {
            return Err(WorkVcsError::PlanInvalid(format!(
                "idempotency key {idempotency_key:?} was already used with payload digest {stored_digest}, not {payload_digest}"
            )));
        }
        let result = result_from_json_value(&value, PlanAdmissionOutcome::Reused)?;
        if matched.replace(result).is_some() {
            return Err(WorkVcsError::PlanInvalid(format!(
                "idempotency key {idempotency_key:?} has multiple matching admission payloads"
            )));
        }
    }
    Ok(matched)
}

fn find_idempotent_evolution(
    connection: &Connection,
    workspace_id: WorkspaceId,
    branch_id: BranchId,
    idempotency_key: &str,
    payload_digest: Digest,
) -> Result<Option<PlanEvolutionResult>> {
    let mut statement = connection
        .prepare(
            "SELECT changeset.operation_schema_version, changeset.operation_payload_json
             FROM changeset
             JOIN workstate_commit
               ON workstate_commit.changeset_id = changeset.changeset_id
             WHERE changeset.workspace_id = ?1
               AND changeset.operation_type = ?2
             ORDER BY changeset.created_at_us, changeset.changeset_id",
        )
        .map_err(storage_error)?;
    let rows = statement
        .query_map(
            params![&workspace_id.raw_bytes()[..], EVOLUTION_OPERATION_TYPE],
            |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?)),
        )
        .map_err(storage_error)?;
    let mut matched = None;
    for row in rows {
        let (operation_schema_version, payload_json) = row.map_err(storage_error)?;
        let value: serde_json::Value = serde_json::from_str(&payload_json).map_err(|error| {
            WorkVcsError::PlanInvalid(format!("stored plan evolution payload is invalid: {error}"))
        })?;
        if value
            .get("idempotency_key")
            .and_then(|value| value.as_str())
            != Some(idempotency_key)
        {
            continue;
        }
        let stored_branch = required_json_str(&value, "branch_id")?;
        if stored_branch != branch_id.to_string() {
            return Err(WorkVcsError::PlanInvalid(format!(
                "idempotency key {idempotency_key:?} already belongs to branch {stored_branch}, not {branch_id}"
            )));
        }
        let stored_digest = Digest::from_hex(required_json_str(&value, "payload_digest")?)?;
        if stored_digest != payload_digest {
            return Err(WorkVcsError::PlanInvalid(format!(
                "idempotency key {idempotency_key:?} was already used with payload digest {stored_digest}, not {payload_digest}"
            )));
        }
        let payload_schema_version = required_json_i64(&value, "schema_version")?;
        if payload_schema_version != operation_schema_version {
            return Err(WorkVcsError::PlanInvalid(format!(
                "stored plan evolution payload schema_version {payload_schema_version} does not match changeset operation_schema_version {operation_schema_version}"
            )));
        }
        let result = evolution_result_from_json_value(&value, PlanEvolutionOutcome::Reused)?;
        if matched.replace(result).is_some() {
            return Err(WorkVcsError::PlanInvalid(format!(
                "idempotency key {idempotency_key:?} has multiple matching evolution payloads"
            )));
        }
    }
    Ok(matched)
}

fn validate_idempotent_evolution_result(
    connection: &Connection,
    parent_state: &WorkState,
    result: &PlanEvolutionResult,
) -> Result<()> {
    let (operation_schema_version, payload_json, change_operation_count) = connection
        .query_row(
            "SELECT changeset.operation_schema_version,
                    changeset.operation_payload_json,
                    COUNT(change_operation.operation_id)
             FROM changeset
             LEFT JOIN change_operation
               ON change_operation.changeset_id = changeset.changeset_id
             WHERE changeset.changeset_id = ?1
             GROUP BY changeset.changeset_id",
            params![&result.changeset_id.raw_bytes()[..]],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                ))
            },
        )
        .optional()
        .map_err(storage_error)?
        .ok_or_else(|| {
            WorkVcsError::PlanInvalid(format!(
                "stored Plan evolution ChangeSet {} does not exist",
                result.changeset_id
            ))
        })?;
    if operation_schema_version == EVOLUTION_OPERATION_SCHEMA_VERSION_V2
        && change_operation_count == 0
    {
        validate_state_preserving_task_disposition_payload_json(
            connection,
            parent_state,
            &payload_json,
            result.workspace_id,
            result.changeset_id,
            result.commit_id,
            result.previous_head_commit_id,
        )?;
    }
    Ok(())
}

fn load_commit_state_digest(transaction: &Transaction<'_>, commit_id: CommitId) -> Result<Digest> {
    let row = transaction
        .query_row(
            "SELECT state_digest
             FROM workstate_commit
             WHERE commit_id = ?1",
            params![&commit_id.raw_bytes()[..]],
            |row| row.get::<_, Vec<u8>>(0),
        )
        .optional()
        .map_err(storage_error)?;
    let Some(state_digest) = row else {
        return Err(WorkVcsError::CommitNotFound(format!(
            "commit {commit_id} does not exist"
        )));
    };
    decode_digest("workstate_commit.state_digest", state_digest)
}

fn validate_plan_version_for_evolution(
    transaction: &Transaction<'_>,
    workspace_id: WorkspaceId,
    prepared: &PreparedEvolution,
) -> Result<()> {
    let row = transaction
        .query_row(
            "SELECT entity.workspace_id,
                    entity.entity_kind,
                    entity_version.state_json,
                    entity_version.state_digest
             FROM entity
             JOIN entity_version
               ON entity_version.entity_id = entity.object_id
             WHERE entity.object_id = ?1
               AND entity_version.entity_version_id = ?2",
            params![
                &prepared.plan.entity_id.raw_bytes()[..],
                &prepared.previous_plan_entity_version_id.raw_bytes()[..]
            ],
            |row| {
                Ok((
                    row.get::<_, Vec<u8>>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, Vec<u8>>(3)?,
                ))
            },
        )
        .optional()
        .map_err(storage_error)?;
    let Some((entity_workspace_id, entity_kind, state_json, state_digest)) = row else {
        return Err(WorkVcsError::PlanNotFound(format!(
            "plan entity {} version {} does not exist",
            prepared.plan.entity_id, prepared.previous_plan_entity_version_id
        )));
    };
    let entity_workspace_id = decode_workspace_id("entity.workspace_id", entity_workspace_id)?;
    if entity_workspace_id != workspace_id {
        return Err(WorkVcsError::PlanInvalid(format!(
            "plan entity {} belongs to workspace {}, not {}",
            prepared.plan.entity_id, entity_workspace_id, workspace_id
        )));
    }
    if entity_kind != PLAN_ENTITY_KIND {
        return Err(WorkVcsError::PlanNotFound(format!(
            "entity {} has kind {entity_kind:?}, not {PLAN_ENTITY_KIND:?}",
            prepared.plan.entity_id
        )));
    }
    let state_digest = decode_digest("entity_version.state_digest", state_digest)?;
    if state_digest != prepared.previous_plan_state_digest {
        return Err(WorkVcsError::DigestInvalid(format!(
            "plan entity {} state digest {} does not match expected {}",
            prepared.plan.entity_id, state_digest, prepared.previous_plan_state_digest
        )));
    }
    let value = parse_canonical_json(state_json.as_bytes()).map_err(plan_invalid_from)?;
    let CanonicalValue::Object(entries) = value else {
        return Err(WorkVcsError::PlanInvalid(
            "plan state must be a canonical object".to_owned(),
        ));
    };
    let status = entries
        .iter()
        .find_map(|(key, value)| (key == "status").then_some(value))
        .and_then(|value| match value {
            CanonicalValue::String(value) => Some(value.as_str()),
            _ => None,
        })
        .ok_or_else(|| WorkVcsError::PlanInvalid("plan state is missing status".to_owned()))?;
    let status_allowed = if prepared.mode == "task_disposition" {
        matches!(status, "superseded" | "abandoned")
    } else {
        status == PlanStatus::Active.as_str()
    };
    if !status_allowed {
        return Err(WorkVcsError::PlanInvalid(format!(
            "plan entity {} has invalid status {status:?} for {} operation",
            prepared.plan.entity_id, prepared.mode
        )));
    }
    Ok(())
}

fn validate_supersede_goal_and_relation_for_evolution(
    transaction: &Transaction<'_>,
    workspace_id: WorkspaceId,
    parent_state: &WorkState,
    prepared: &PreparedEvolution,
) -> Result<()> {
    if prepared.mode == "task_disposition" {
        return Ok(());
    }
    let Some(expected_parent_entity_version_id) = prepared.expected_goal_entity_version_id else {
        if prepared.operation_schema_version == 2 {
            let relations = load_plan_primary_containment_from_state(
                transaction,
                workspace_id,
                parent_state,
                prepared.plan.entity_id,
            )?;
            if !relations.is_empty() {
                return Err(WorkVcsError::RelationInvalid(format!(
                    "standalone plan {} gained a primary parent before supersede",
                    prepared.plan.entity_id
                )));
            }
        }
        return Ok(());
    };
    let expected_goal_plan_relation_id = prepared
        .expected_goal_plan_relation_id
        .expect("supersede relation id prepared with goal version");
    let expected_goal_plan_relation_version_id = prepared
        .expected_goal_plan_relation_version_id
        .expect("supersede relation version prepared with goal version");
    let parent_entity_id = prepared
        .parent_entity_id
        .expect("parent id accompanies version guard");
    let parent_kind = prepared
        .parent_kind
        .expect("parent kind accompanies version guard");
    validate_active_parent_version_for_supersede(
        transaction,
        workspace_id,
        parent_entity_id,
        expected_parent_entity_version_id,
        parent_kind,
    )?;
    let relations = load_plan_primary_containment_from_state(
        transaction,
        workspace_id,
        parent_state,
        prepared.plan.entity_id,
    )?;
    match relations.as_slice() {
        [relation] => {
            if relation.source_entity_id != parent_entity_id {
                return Err(WorkVcsError::RelationInvalid(format!(
                    "plan entity {} primary parent is {}, not expected {}",
                    prepared.plan.entity_id, relation.source_entity_id, parent_entity_id
                )));
            }
            if relation.relation_id != expected_goal_plan_relation_id
                || relation.relation_version_id != expected_goal_plan_relation_version_id
            {
                return Err(WorkVcsError::RelationInvalid(format!(
                    "plan entity {} expected goal containment relation {} version {}, found {} version {}",
                    prepared.plan.entity_id,
                    expected_goal_plan_relation_id,
                    expected_goal_plan_relation_version_id,
                    relation.relation_id,
                    relation.relation_version_id
                )));
            }
            Ok(())
        }
        [] => Err(WorkVcsError::RelationInvalid(format!(
            "plan entity {} has no primary goal containment at commit {}",
            prepared.plan.entity_id, prepared.previous_head_commit_id
        ))),
        relations => Err(WorkVcsError::RelationInvalid(format!(
            "plan entity {} has {} primary containment parents at commit {}",
            prepared.plan.entity_id,
            relations.len(),
            prepared.previous_head_commit_id
        ))),
    }
}

fn validate_repair_carrier_for_evolution(
    transaction: &Transaction<'_>,
    workspace_id: WorkspaceId,
    prepared: &PreparedEvolution,
) -> Result<()> {
    if prepared.mode != "task_disposition" {
        return Ok(());
    }
    let Some(carrier_entity_id) = prepared.carrier_entity_id else {
        return Ok(());
    };
    let carrier_kind = prepared.carrier_kind.expect("repair carrier kind");
    let carrier_version_id = prepared
        .expected_carrier_entity_version_id
        .expect("repair carrier version");
    validate_active_parent_version_for_supersede(
        transaction,
        workspace_id,
        carrier_entity_id,
        carrier_version_id,
        carrier_kind,
    )?;
    let digest = transaction
        .query_row(
            "SELECT state_digest FROM entity_version WHERE entity_id = ?1 AND entity_version_id = ?2",
            params![
                &carrier_entity_id.raw_bytes()[..],
                &carrier_version_id.raw_bytes()[..]
            ],
            |row| row.get::<_, Vec<u8>>(0),
        )
        .optional()
        .map_err(storage_error)?
        .ok_or_else(|| {
            WorkVcsError::PlanInvalid(format!(
                "repair carrier {carrier_entity_id} version {carrier_version_id} disappeared"
            ))
        })?;
    let digest = decode_digest("entity_version.state_digest", digest)?;
    if Some(digest) != prepared.expected_carrier_state_digest {
        return Err(WorkVcsError::DigestInvalid(format!(
            "repair carrier {carrier_entity_id} state digest changed before commit"
        )));
    }
    Ok(())
}

#[derive(Clone, Copy, Debug)]
struct RelationGuardSnapshot {
    relation_id: RelationId,
    relation_version_id: RelationVersionId,
    source_entity_id: EntityId,
}

fn load_plan_primary_containment_from_state(
    transaction: &Transaction<'_>,
    workspace_id: WorkspaceId,
    state: &WorkState,
    target_plan_entity_id: EntityId,
) -> Result<Vec<RelationGuardSnapshot>> {
    let mut relations = Vec::new();
    for (relation_id, relation_version_id) in state.relations() {
        let Some(relation) = load_relation_guard_snapshot(
            transaction,
            workspace_id,
            *relation_id,
            *relation_version_id,
        )?
        else {
            continue;
        };
        if relation.relation_type != CONTAINS_RELATION_TYPE
            || relation.relation_discriminator != PRIMARY_CONTAINMENT_DISCRIMINATOR
            || relation.target_entity_id != target_plan_entity_id
        {
            continue;
        }
        validate_entity_kind_for_relation(
            transaction,
            relation.target_entity_id,
            PLAN_ENTITY_KIND,
        )?;
        relations.push(RelationGuardSnapshot {
            relation_id: *relation_id,
            relation_version_id: *relation_version_id,
            source_entity_id: relation.source_entity_id,
        });
    }
    Ok(relations)
}

struct LoadedRelationGuard {
    relation_type: String,
    relation_discriminator: String,
    source_entity_id: EntityId,
    target_entity_id: EntityId,
}

fn load_relation_guard_snapshot(
    transaction: &Transaction<'_>,
    workspace_id: WorkspaceId,
    relation_id: RelationId,
    relation_version_id: RelationVersionId,
) -> Result<Option<LoadedRelationGuard>> {
    let row = transaction
        .query_row(
            "SELECT object_identity.object_kind,
                    relation.workspace_id,
                    relation.relation_type,
                    relation.source_object_id,
                    relation.target_object_id,
                    relation.relation_discriminator,
                    relation_version.state_schema_version,
                    relation_version.metadata_json,
                    relation_version.state_digest
             FROM relation
             JOIN object_identity
               ON object_identity.object_id = relation.object_id
             JOIN relation_version
               ON relation_version.relation_id = relation.object_id
             WHERE relation.object_id = ?1
               AND relation_version.relation_version_id = ?2",
            params![
                &relation_id.raw_bytes()[..],
                &relation_version_id.raw_bytes()[..]
            ],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, Vec<u8>>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, Vec<u8>>(3)?,
                    row.get::<_, Vec<u8>>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, i64>(6)?,
                    row.get::<_, String>(7)?,
                    row.get::<_, Vec<u8>>(8)?,
                ))
            },
        )
        .optional()
        .map_err(storage_error)?;
    let Some((
        object_kind,
        relation_workspace_id,
        relation_type,
        source_entity_id,
        target_entity_id,
        relation_discriminator,
        state_schema_version,
        metadata_json,
        state_digest,
    )) = row
    else {
        return Err(WorkVcsError::RelationInvalid(format!(
            "relation {relation_id} version {relation_version_id} does not exist"
        )));
    };
    if object_kind != RELATION_OBJECT_KIND {
        return Err(WorkVcsError::RelationInvalid(format!(
            "relation {relation_id} has object kind {object_kind:?}"
        )));
    }
    let relation_workspace_id =
        decode_workspace_id("relation.workspace_id", relation_workspace_id)?;
    if relation_workspace_id != workspace_id {
        return Err(WorkVcsError::RelationInvalid(format!(
            "relation {relation_id} belongs to workspace {relation_workspace_id}, not {workspace_id}"
        )));
    }
    if state_schema_version != RELATION_STATE_SCHEMA_VERSION {
        return Err(WorkVcsError::RelationInvalid(format!(
            "relation {relation_id} version {relation_version_id} has state schema version {state_schema_version}"
        )));
    }
    let state_digest = decode_digest("relation_version.state_digest", state_digest)?;
    let metadata_value =
        parse_canonical_json(metadata_json.as_bytes()).map_err(relation_invalid_from)?;
    if metadata_value != CanonicalValue::object(Vec::new())? {
        return Err(WorkVcsError::RelationInvalid(format!(
            "relation {relation_id} version {relation_version_id} state must be canonical empty object"
        )));
    }
    let actual = relation_version_digest(&metadata_value).map_err(relation_invalid_from)?;
    if actual != state_digest {
        return Err(WorkVcsError::RelationInvalid(format!(
            "relation {relation_id} version {relation_version_id} digest does not match metadata JSON"
        )));
    }
    Ok(Some(LoadedRelationGuard {
        relation_type,
        relation_discriminator,
        source_entity_id: decode_entity_id("relation.source_object_id", source_entity_id)?,
        target_entity_id: decode_entity_id("relation.target_object_id", target_entity_id)?,
    }))
}

fn validate_active_parent_version_for_supersede(
    transaction: &Transaction<'_>,
    workspace_id: WorkspaceId,
    parent_entity_id: EntityId,
    parent_entity_version_id: EntityVersionId,
    parent_kind: PrimaryContainmentEndpointKind,
) -> Result<()> {
    let row = transaction
        .query_row(
            "SELECT entity.workspace_id,
                    entity.entity_kind,
                    entity_version.state_json
             FROM entity
             JOIN entity_version
               ON entity_version.entity_id = entity.object_id
             WHERE entity.object_id = ?1
               AND entity_version.entity_version_id = ?2",
            params![
                &parent_entity_id.raw_bytes()[..],
                &parent_entity_version_id.raw_bytes()[..]
            ],
            |row| {
                Ok((
                    row.get::<_, Vec<u8>>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            },
        )
        .optional()
        .map_err(storage_error)?;
    let Some((entity_workspace_id, entity_kind, state_json)) = row else {
        return Err(WorkVcsError::PlanInvalid(format!(
            "parent entity {parent_entity_id} version {parent_entity_version_id} does not exist"
        )));
    };
    let entity_workspace_id = decode_workspace_id("entity.workspace_id", entity_workspace_id)?;
    if entity_workspace_id != workspace_id {
        return Err(WorkVcsError::PlanInvalid(format!(
            "parent entity {parent_entity_id} belongs to workspace {entity_workspace_id}, not {workspace_id}"
        )));
    }
    if entity_kind != parent_kind.as_str() {
        return Err(WorkVcsError::PlanInvalid(format!(
            "entity {parent_entity_id} has kind {entity_kind:?}, not {:?}",
            parent_kind.as_str()
        )));
    }
    let value = parse_canonical_json(state_json.as_bytes()).map_err(plan_invalid_from)?;
    let CanonicalValue::Object(entries) = value else {
        return Err(WorkVcsError::PlanInvalid(
            "parent state must be a canonical object".to_owned(),
        ));
    };
    let status = entries
        .iter()
        .find_map(|(key, value)| (key == "status").then_some(value))
        .and_then(|value| match value {
            CanonicalValue::String(value) => Some(value.as_str()),
            _ => None,
        })
        .ok_or_else(|| WorkVcsError::PlanInvalid("parent state is missing status".to_owned()))?;
    let expected_status = match parent_kind {
        PrimaryContainmentEndpointKind::Goal => GoalStatus::Active.as_str(),
        PrimaryContainmentEndpointKind::Plan => PlanStatus::Active.as_str(),
        PrimaryContainmentEndpointKind::Task => {
            return Err(WorkVcsError::PlanInvalid(
                "Task cannot be a Plan primary parent".to_owned(),
            ));
        }
    };
    if status != expected_status {
        return Err(WorkVcsError::PlanInvalid(format!(
            "parent entity {parent_entity_id} is {status}, not active"
        )));
    }
    Ok(())
}

fn validate_entity_kind_for_relation(
    transaction: &Transaction<'_>,
    entity_id: EntityId,
    expected_kind: &str,
) -> Result<()> {
    let kind = transaction
        .query_row(
            "SELECT entity_kind
             FROM entity
             WHERE object_id = ?1",
            params![&entity_id.raw_bytes()[..]],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(storage_error)?
        .ok_or_else(|| {
            WorkVcsError::RelationInvalid(format!(
                "relation endpoint entity {entity_id} is missing"
            ))
        })?;
    if kind != expected_kind {
        return Err(WorkVcsError::RelationInvalid(format!(
            "relation endpoint entity {entity_id} has kind {kind:?}, not {expected_kind:?}"
        )));
    }
    Ok(())
}

pub(crate) fn plan_supersedes_relations_at(
    connection: &StoreConnection,
    commit_id: CommitId,
) -> Result<Vec<PlanSupersedesRelationSnapshot>> {
    let replayed = state_at(connection, commit_id)?;
    let mut relations = Vec::new();
    for (relation_id, relation_version_id) in replayed.state.relations() {
        let Some(relation) = load_plan_supersedes_relation_version(
            connection,
            replayed.workspace_id,
            commit_id,
            *relation_id,
            *relation_version_id,
        )?
        else {
            continue;
        };
        relations.push(relation);
    }
    relations.sort_by(|left, right| {
        left.replacement_plan_entity_id
            .cmp(&right.replacement_plan_entity_id)
            .then_with(|| left.prior_plan_entity_id.cmp(&right.prior_plan_entity_id))
            .then_with(|| left.relation_id.cmp(&right.relation_id))
    });
    Ok(relations)
}

fn load_plan_supersedes_relation_version(
    connection: &StoreConnection,
    workspace_id: WorkspaceId,
    commit_id: CommitId,
    relation_id: RelationId,
    relation_version_id: RelationVersionId,
) -> Result<Option<PlanSupersedesRelationSnapshot>> {
    let row = connection
        .inner()
        .query_row(
            "SELECT object_identity.object_kind,
                    relation.workspace_id,
                    relation.relation_type,
                    relation.source_object_id,
                    relation.target_object_id,
                    relation.relation_discriminator,
                    relation_version.state_schema_version,
                    relation_version.metadata_json,
                    relation_version.state_digest
             FROM relation
             JOIN object_identity
               ON object_identity.object_id = relation.object_id
             JOIN relation_version
               ON relation_version.relation_id = relation.object_id
             WHERE relation.object_id = ?1
               AND relation_version.relation_version_id = ?2",
            params![
                &relation_id.raw_bytes()[..],
                &relation_version_id.raw_bytes()[..]
            ],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, Vec<u8>>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, Vec<u8>>(3)?,
                    row.get::<_, Vec<u8>>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, i64>(6)?,
                    row.get::<_, String>(7)?,
                    row.get::<_, Vec<u8>>(8)?,
                ))
            },
        )
        .optional()
        .map_err(storage_error)?;
    let Some((
        object_kind,
        relation_workspace_id,
        relation_type,
        replacement_plan_entity_id,
        prior_plan_entity_id,
        relation_discriminator,
        state_schema_version,
        metadata_json,
        state_digest,
    )) = row
    else {
        return Err(WorkVcsError::RelationInvalid(format!(
            "plan supersedes relation {relation_id} version {relation_version_id} does not exist"
        )));
    };
    if object_kind != RELATION_OBJECT_KIND {
        return Err(WorkVcsError::RelationInvalid(format!(
            "plan supersedes relation {relation_id} has object kind {object_kind:?}"
        )));
    }
    let relation_workspace_id =
        decode_workspace_id("relation.workspace_id", relation_workspace_id)?;
    if relation_workspace_id != workspace_id {
        return Err(WorkVcsError::RelationInvalid(format!(
            "plan supersedes relation {relation_id} belongs to workspace {relation_workspace_id}, not {workspace_id}"
        )));
    }
    if relation_type != PLAN_SUPERSEDES_RELATION_TYPE
        || relation_discriminator != PLAN_SUPERSEDES_RELATION_DISCRIMINATOR
    {
        return Ok(None);
    }
    if state_schema_version != RELATION_STATE_SCHEMA_VERSION {
        return Err(WorkVcsError::RelationInvalid(format!(
            "plan supersedes relation {relation_id} version {relation_version_id} has state schema version {state_schema_version}"
        )));
    }
    let state_digest = decode_digest("relation_version.state_digest", state_digest)?;
    let metadata_value = crate::canonical::parse_canonical_json(metadata_json.as_bytes())
        .map_err(relation_invalid_from)?;
    if metadata_value != CanonicalValue::object(Vec::new())? {
        return Err(WorkVcsError::RelationInvalid(format!(
            "plan supersedes relation {relation_id} version {relation_version_id} state must be canonical empty object"
        )));
    }
    let actual = relation_version_digest(&metadata_value).map_err(relation_invalid_from)?;
    if actual != state_digest {
        return Err(WorkVcsError::RelationInvalid(format!(
            "plan supersedes relation {relation_id} version {relation_version_id} digest does not match metadata JSON"
        )));
    }
    let replacement_plan_entity_id =
        decode_entity_id("relation.source_object_id", replacement_plan_entity_id)?;
    let prior_plan_entity_id = decode_entity_id("relation.target_object_id", prior_plan_entity_id)?;
    let replacement = plan_at(connection, commit_id, replacement_plan_entity_id)
        .map_err(relation_invalid_from)?;
    let prior =
        plan_at(connection, commit_id, prior_plan_entity_id).map_err(relation_invalid_from)?;
    if replacement.workspace_id != workspace_id || prior.workspace_id != workspace_id {
        return Err(WorkVcsError::RelationInvalid(format!(
            "plan supersedes relation {relation_id} endpoints must belong to workspace {workspace_id}"
        )));
    }
    Ok(Some(PlanSupersedesRelationSnapshot {
        workspace_id,
        commit_id,
        relation_id,
        relation_version_id,
        replacement_plan_entity_id,
        prior_plan_entity_id,
        state_digest,
    }))
}

pub(crate) fn task_supersedes_relations_at(
    connection: &StoreConnection,
    commit_id: CommitId,
) -> Result<Vec<TaskSupersedesRelationSnapshot>> {
    let replayed = state_at(connection, commit_id)?;
    let mut relations = Vec::new();
    for (relation_id, relation_version_id) in replayed.state.relations() {
        let row = connection
            .inner()
            .query_row(
                "SELECT object_identity.object_kind,
                        relation.workspace_id,
                        relation.relation_type,
                        relation.source_object_id,
                        relation.target_object_id,
                        relation.relation_discriminator,
                        relation_version.state_schema_version,
                        relation_version.metadata_json,
                        relation_version.state_digest
                 FROM relation
                 JOIN object_identity ON object_identity.object_id = relation.object_id
                 JOIN relation_version ON relation_version.relation_id = relation.object_id
                 WHERE relation.object_id = ?1 AND relation_version.relation_version_id = ?2",
                params![
                    &relation_id.raw_bytes()[..],
                    &relation_version_id.raw_bytes()[..]
                ],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, Vec<u8>>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, Vec<u8>>(3)?,
                        row.get::<_, Vec<u8>>(4)?,
                        row.get::<_, String>(5)?,
                        row.get::<_, i64>(6)?,
                        row.get::<_, String>(7)?,
                        row.get::<_, Vec<u8>>(8)?,
                    ))
                },
            )
            .optional()
            .map_err(storage_error)?
            .ok_or_else(|| {
                WorkVcsError::RelationInvalid(format!(
                    "task supersedes relation {relation_id} version {relation_version_id} does not exist"
                ))
            })?;
        let (
            object_kind,
            relation_workspace_id,
            relation_type,
            replacement_task_entity_id,
            prior_task_entity_id,
            relation_discriminator,
            state_schema_version,
            metadata_json,
            state_digest,
        ) = row;
        if relation_type != PLAN_SUPERSEDES_RELATION_TYPE
            || relation_discriminator != TASK_SUPERSEDES_RELATION_DISCRIMINATOR
        {
            continue;
        }
        if object_kind != RELATION_OBJECT_KIND {
            return Err(WorkVcsError::RelationInvalid(format!(
                "task supersedes relation {relation_id} has object kind {object_kind:?}"
            )));
        }
        let relation_workspace_id =
            decode_workspace_id("relation.workspace_id", relation_workspace_id)?;
        if relation_workspace_id != replayed.workspace_id {
            return Err(WorkVcsError::RelationInvalid(format!(
                "task supersedes relation {relation_id} belongs to workspace {relation_workspace_id}, not {}",
                replayed.workspace_id
            )));
        }
        if state_schema_version != RELATION_STATE_SCHEMA_VERSION {
            return Err(WorkVcsError::RelationInvalid(format!(
                "task supersedes relation {relation_id} version {relation_version_id} has state schema version {state_schema_version}"
            )));
        }
        let state_digest = decode_digest("relation_version.state_digest", state_digest)?;
        let metadata_value = crate::canonical::parse_canonical_json(metadata_json.as_bytes())
            .map_err(relation_invalid_from)?;
        if metadata_value != CanonicalValue::object(Vec::new())?
            || relation_version_digest(&metadata_value).map_err(relation_invalid_from)?
                != state_digest
        {
            return Err(WorkVcsError::RelationInvalid(format!(
                "task supersedes relation {relation_id} version {relation_version_id} has invalid state"
            )));
        }
        let replacement_task_entity_id =
            decode_entity_id("relation.source_object_id", replacement_task_entity_id)?;
        let prior_task_entity_id =
            decode_entity_id("relation.target_object_id", prior_task_entity_id)?;
        let replacement = task_at(connection, commit_id, replacement_task_entity_id)
            .map_err(relation_invalid_from)?;
        let prior =
            task_at(connection, commit_id, prior_task_entity_id).map_err(relation_invalid_from)?;
        if replacement.workspace_id != replayed.workspace_id
            || prior.workspace_id != replayed.workspace_id
        {
            return Err(WorkVcsError::RelationInvalid(format!(
                "task supersedes relation {relation_id} endpoints must belong to workspace {}",
                replayed.workspace_id
            )));
        }
        relations.push(TaskSupersedesRelationSnapshot {
            workspace_id: replayed.workspace_id,
            commit_id,
            relation_id: *relation_id,
            relation_version_id: *relation_version_id,
            replacement_task_entity_id,
            prior_task_entity_id,
            state_digest,
        });
    }
    relations.sort_by_key(|relation| {
        (
            relation.replacement_task_entity_id,
            relation.prior_task_entity_id,
            relation.relation_id,
        )
    });
    Ok(relations)
}

fn resolve_single_goal_parent(
    connection: &StoreConnection,
    commit_id: CommitId,
    plan_entity_id: EntityId,
) -> Result<EntityId> {
    let matches = super::primary_containment_relations_at(connection, commit_id)?
        .into_iter()
        .filter(|relation| relation.child_entity_id == plan_entity_id)
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [relation] if relation.parent_kind == PrimaryContainmentEndpointKind::Goal => {
            Ok(relation.parent_entity_id)
        }
        [relation] => Err(WorkVcsError::RelationInvalid(format!(
            "plan entity {plan_entity_id} primary parent {} has kind {}, not goal",
            relation.parent_entity_id, relation.parent_kind
        ))),
        [] => Err(WorkVcsError::RelationInvalid(format!(
            "plan entity {plan_entity_id} has no primary goal containment at commit {commit_id}"
        ))),
        relations => Err(WorkVcsError::RelationInvalid(format!(
            "plan entity {plan_entity_id} has {} primary containment parents at commit {commit_id}",
            relations.len()
        ))),
    }
}

fn apply_plan_update(
    mut state: PlanState,
    update: &PlanEvolutionPlanUpdateManifest,
) -> Result<PlanState> {
    if let Some(description) = &update.description {
        state.description = description.clone();
    }
    if let Some(strategy) = &update.strategy {
        state.strategy = strategy.clone();
    }
    if let Some(constraints) = &update.constraints {
        state.constraints = constraints.clone();
    }
    state.to_canonical_value()?;
    Ok(state)
}

// Canonical payload fields stay explicit so their ordering and provenance are reviewable.
#[allow(clippy::too_many_arguments)]
fn admission_payload_value(
    workspace_id: WorkspaceId,
    branch_id: BranchId,
    previous_head_commit_id: CommitId,
    commit_id: CommitId,
    changeset_id: ChangeSetId,
    work_state_digest: Digest,
    prepared: &PreparedAdmission,
    relations: &[PreparedRelation],
) -> Result<CanonicalValue> {
    let operations = operation_values(prepared, relations)?;
    CanonicalValue::object(vec![
        (
            "schema_version".to_owned(),
            CanonicalValue::safe_integer(1)?,
        ),
        (
            "operation".to_owned(),
            CanonicalValue::String(ADMISSION_OPERATION_TYPE.to_owned()),
        ),
        (
            "idempotency_key".to_owned(),
            CanonicalValue::String(prepared.idempotency_key.clone()),
        ),
        (
            "payload_digest".to_owned(),
            CanonicalValue::String(prepared.payload_digest.to_hex()),
        ),
        (
            "workspace_id".to_owned(),
            CanonicalValue::String(workspace_id.to_string()),
        ),
        (
            "branch_id".to_owned(),
            CanonicalValue::String(branch_id.to_string()),
        ),
        (
            "previous_head_commit_id".to_owned(),
            CanonicalValue::String(previous_head_commit_id.to_string()),
        ),
        (
            "commit_id".to_owned(),
            CanonicalValue::String(commit_id.to_string()),
        ),
        (
            "changeset_id".to_owned(),
            CanonicalValue::String(changeset_id.to_string()),
        ),
        (
            "work_state_digest".to_owned(),
            CanonicalValue::String(work_state_digest.to_hex()),
        ),
        ("goal".to_owned(), goal_payload_value(&prepared.goal)?),
        (
            "plan".to_owned(),
            entity_payload_value(&prepared.plan, None)?,
        ),
        ("tasks".to_owned(), task_payload_values(&prepared.tasks)?),
        (
            "records".to_owned(),
            record_payload_values(&prepared.records)?,
        ),
        (
            "evidence".to_owned(),
            evidence_payload_values(&prepared.evidence)?,
        ),
        ("operations".to_owned(), CanonicalValue::Array(operations)),
    ])
}

fn operation_values(
    prepared: &PreparedAdmission,
    relations: &[PreparedRelation],
) -> Result<Vec<CanonicalValue>> {
    let mut values = Vec::new();
    if let PreparedGoal::Create(goal) = &prepared.goal {
        values.push(entity_operation_value(goal)?);
    }
    values.push(entity_operation_value(&prepared.plan)?);
    for task in &prepared.tasks {
        values.push(entity_operation_value(&task.entity)?);
        for criterion in &task.acceptance_criteria {
            values.push(entity_operation_value(&criterion.entity)?);
            for requirement in &criterion.verification_requirements {
                values.push(entity_operation_value(&requirement.entity)?);
            }
        }
    }
    for record in &prepared.records {
        values.push(entity_operation_value(&record.entity)?);
    }
    for relation in relations {
        values.push(relation_operation_value(relation)?);
    }
    Ok(values)
}

fn goal_payload_value(goal: &PreparedGoal) -> Result<CanonicalValue> {
    match goal {
        PreparedGoal::Existing {
            entity_id,
            entity_version_id,
        } => CanonicalValue::object(vec![
            ("created".to_owned(), CanonicalValue::Bool(false)),
            (
                "entity_id".to_owned(),
                CanonicalValue::String(entity_id.to_string()),
            ),
            (
                "entity_version_id".to_owned(),
                CanonicalValue::String(entity_version_id.to_string()),
            ),
        ]),
        PreparedGoal::Create(entity) => entity_payload_value(entity, Some(true)),
    }
}

fn entity_payload_value(entity: &PreparedEntity, created: Option<bool>) -> Result<CanonicalValue> {
    let mut entries = vec![
        (
            "entity_id".to_owned(),
            CanonicalValue::String(entity.entity_id.to_string()),
        ),
        (
            "entity_version_id".to_owned(),
            CanonicalValue::String(entity.entity_version_id.to_string()),
        ),
        (
            "state_digest".to_owned(),
            CanonicalValue::String(entity.state_digest.to_hex()),
        ),
    ];
    if let Some(created) = created {
        entries.push(("created".to_owned(), CanonicalValue::Bool(created)));
    }
    CanonicalValue::object(entries)
}

fn task_payload_values(tasks: &[PreparedTask]) -> Result<CanonicalValue> {
    tasks
        .iter()
        .map(|task| {
            let mut value = match entity_payload_value(&task.entity, None)? {
                CanonicalValue::Object(entries) => entries,
                _ => unreachable!("entity payload is object"),
            };
            value.push((
                "local_id".to_owned(),
                CanonicalValue::String(task.local_id.clone()),
            ));
            value.push((
                "acceptance_criteria".to_owned(),
                acceptance_payload_values(&task.acceptance_criteria)?,
            ));
            CanonicalValue::object(value)
        })
        .collect::<Result<Vec<_>>>()
        .map(CanonicalValue::Array)
}

fn acceptance_payload_values(criteria: &[PreparedAcceptanceCriterion]) -> Result<CanonicalValue> {
    criteria
        .iter()
        .map(|criterion| {
            let mut value = match entity_payload_value(&criterion.entity, None)? {
                CanonicalValue::Object(entries) => entries,
                _ => unreachable!("entity payload is object"),
            };
            value.push((
                "local_id".to_owned(),
                CanonicalValue::String(criterion.local_id.clone()),
            ));
            value.push((
                "verification_requirements".to_owned(),
                verification_requirement_payload_values(&criterion.verification_requirements)?,
            ));
            CanonicalValue::object(value)
        })
        .collect::<Result<Vec<_>>>()
        .map(CanonicalValue::Array)
}

fn verification_requirement_payload_values(
    requirements: &[PreparedVerificationRequirement],
) -> Result<CanonicalValue> {
    requirements
        .iter()
        .map(|requirement| {
            let mut value = match entity_payload_value(&requirement.entity, None)? {
                CanonicalValue::Object(entries) => entries,
                _ => unreachable!("entity payload is object"),
            };
            value.push((
                "local_id".to_owned(),
                CanonicalValue::String(requirement.local_id.clone()),
            ));
            CanonicalValue::object(value)
        })
        .collect::<Result<Vec<_>>>()
        .map(CanonicalValue::Array)
}

fn record_payload_values(records: &[PreparedRecord]) -> Result<CanonicalValue> {
    records
        .iter()
        .map(|record| {
            let mut value = match entity_payload_value(&record.entity, None)? {
                CanonicalValue::Object(entries) => entries,
                _ => unreachable!("entity payload is object"),
            };
            value.push((
                "local_id".to_owned(),
                CanonicalValue::String(record.local_id.clone()),
            ));
            value.push((
                "kind".to_owned(),
                CanonicalValue::String(record.kind.as_str().to_owned()),
            ));
            CanonicalValue::object(value)
        })
        .collect::<Result<Vec<_>>>()
        .map(CanonicalValue::Array)
}

fn evidence_payload_values(evidence: &[PreparedEvidence]) -> Result<CanonicalValue> {
    evidence
        .iter()
        .map(|evidence| {
            CanonicalValue::object(vec![
                (
                    "local_id".to_owned(),
                    CanonicalValue::String(evidence.local_id.clone()),
                ),
                (
                    "evidence_id".to_owned(),
                    CanonicalValue::String(evidence.evidence_id.to_string()),
                ),
            ])
        })
        .collect::<Result<Vec<_>>>()
        .map(CanonicalValue::Array)
}

// Canonical payload fields stay explicit so their ordering and provenance are reviewable.
#[allow(clippy::too_many_arguments)]
fn evolution_payload_value(
    workspace_id: WorkspaceId,
    branch_id: BranchId,
    previous_head_commit_id: CommitId,
    commit_id: CommitId,
    changeset_id: ChangeSetId,
    work_state_digest: Digest,
    prepared: &PreparedEvolution,
    relations: &[PreparedRelation],
) -> Result<CanonicalValue> {
    let operations = evolution_operation_values(prepared, relations)?;
    let mut entries = vec![
        (
            "schema_version".to_owned(),
            CanonicalValue::safe_integer(prepared.operation_schema_version)?,
        ),
        (
            "operation".to_owned(),
            CanonicalValue::String(EVOLUTION_OPERATION_TYPE.to_owned()),
        ),
        (
            "mode".to_owned(),
            CanonicalValue::String(prepared.mode.to_owned()),
        ),
        (
            "idempotency_key".to_owned(),
            CanonicalValue::String(prepared.idempotency_key.clone()),
        ),
        (
            "payload_digest".to_owned(),
            CanonicalValue::String(prepared.payload_digest.to_hex()),
        ),
        (
            "workspace_id".to_owned(),
            CanonicalValue::String(workspace_id.to_string()),
        ),
        (
            "branch_id".to_owned(),
            CanonicalValue::String(branch_id.to_string()),
        ),
        (
            "previous_head_commit_id".to_owned(),
            CanonicalValue::String(previous_head_commit_id.to_string()),
        ),
        (
            "commit_id".to_owned(),
            CanonicalValue::String(commit_id.to_string()),
        ),
        (
            "changeset_id".to_owned(),
            CanonicalValue::String(changeset_id.to_string()),
        ),
        (
            "work_state_digest".to_owned(),
            CanonicalValue::String(work_state_digest.to_hex()),
        ),
        (
            "goal_entity_id".to_owned(),
            prepared
                .goal_entity_id
                .map(|id| CanonicalValue::String(id.to_string()))
                .unwrap_or(CanonicalValue::Null),
        ),
        ("plan".to_owned(), evolution_plan_payload_value(prepared)?),
        (
            "new_plan".to_owned(),
            match &prepared.new_plan {
                Some(plan) => entity_payload_value(plan, Some(true))?,
                None => CanonicalValue::Null,
            },
        ),
        (
            "goal_contains_relation".to_owned(),
            match &prepared.goal_contains_relation {
                Some(relation) => evolution_relation_payload_value(relation)?,
                None => CanonicalValue::Null,
            },
        ),
        (
            "supersedes_relation".to_owned(),
            match &prepared.supersedes_relation {
                Some(relation) => evolution_relation_payload_value(relation)?,
                None => CanonicalValue::Null,
            },
        ),
        ("tasks".to_owned(), task_payload_values(&prepared.tasks)?),
        (
            "records".to_owned(),
            record_payload_values(&prepared.records)?,
        ),
        (
            "evidence".to_owned(),
            evidence_payload_values(&prepared.evidence)?,
        ),
        ("operations".to_owned(), CanonicalValue::Array(operations)),
    ];
    if prepared.operation_schema_version == 2 {
        entries.push((
            "plan_changed".to_owned(),
            CanonicalValue::Bool(prepared.plan_changed),
        ));
        entries.push((
            "parent_entity_id".to_owned(),
            prepared
                .parent_entity_id
                .map(|id| CanonicalValue::String(id.to_string()))
                .unwrap_or(CanonicalValue::Null),
        ));
        entries.push((
            "parent_kind".to_owned(),
            prepared
                .parent_kind
                .map(|kind| CanonicalValue::String(kind.as_str().to_owned()))
                .unwrap_or(CanonicalValue::Null),
        ));
        entries.push((
            "task_dispositions".to_owned(),
            task_disposition_payload_values(&prepared.task_dispositions)?,
        ));
        entries.push((
            "disposition_relations".to_owned(),
            CanonicalValue::Array(
                prepared
                    .disposition_relations
                    .iter()
                    .map(evolution_disposition_relation_payload_value)
                    .collect::<Result<Vec<_>>>()?,
            ),
        ));
    }
    CanonicalValue::object(entries)
}

fn task_disposition_payload_values(
    dispositions: &[PreparedTaskDisposition],
) -> Result<CanonicalValue> {
    dispositions
        .iter()
        .map(|item| {
            let (entity_version_id, state_digest) = item
                .next_entity
                .as_ref()
                .map(|entity| (entity.entity_version_id, entity.state_digest))
                .unwrap_or((item.previous_entity_version_id, item.previous_state_digest));
            CanonicalValue::object(vec![
                (
                    "task_entity_id".to_owned(),
                    CanonicalValue::String(item.task_entity_id.to_string()),
                ),
                (
                    "previous_entity_version_id".to_owned(),
                    CanonicalValue::String(item.previous_entity_version_id.to_string()),
                ),
                (
                    "previous_state_digest".to_owned(),
                    CanonicalValue::String(item.previous_state_digest.to_hex()),
                ),
                (
                    "action".to_owned(),
                    CanonicalValue::String(item.action.as_str().to_owned()),
                ),
                (
                    "replacement_task_local_id".to_owned(),
                    item.replacement_task_local_id
                        .as_ref()
                        .map(|value| CanonicalValue::String(value.clone()))
                        .unwrap_or(CanonicalValue::Null),
                ),
                (
                    "replacement_task_entity_id".to_owned(),
                    item.replacement_task_entity_id
                        .map(|value| CanonicalValue::String(value.to_string()))
                        .unwrap_or(CanonicalValue::Null),
                ),
                (
                    "entity_version_id".to_owned(),
                    CanonicalValue::String(entity_version_id.to_string()),
                ),
                (
                    "state_digest".to_owned(),
                    CanonicalValue::String(state_digest.to_hex()),
                ),
                (
                    "post_current".to_owned(),
                    CanonicalValue::Bool(item.post_current),
                ),
                (
                    "post_carriers".to_owned(),
                    CanonicalValue::Array(
                        item.post_carriers
                            .iter()
                            .cloned()
                            .map(CanonicalValue::String)
                            .collect(),
                    ),
                ),
                (
                    "return_condition".to_owned(),
                    CanonicalValue::String(item.return_condition.clone()),
                ),
                (
                    "risks".to_owned(),
                    CanonicalValue::Array(
                        item.risks
                            .iter()
                            .cloned()
                            .map(CanonicalValue::String)
                            .collect(),
                    ),
                ),
                ("rationale".to_owned(), item.rationale.clone()),
            ])
        })
        .collect::<Result<Vec<_>>>()
        .map(CanonicalValue::Array)
}

fn evolution_plan_payload_value(prepared: &PreparedEvolution) -> Result<CanonicalValue> {
    CanonicalValue::object(vec![
        (
            "entity_id".to_owned(),
            CanonicalValue::String(prepared.plan.entity_id.to_string()),
        ),
        (
            "previous_entity_version_id".to_owned(),
            CanonicalValue::String(prepared.previous_plan_entity_version_id.to_string()),
        ),
        (
            "previous_state_digest".to_owned(),
            CanonicalValue::String(prepared.previous_plan_state_digest.to_hex()),
        ),
        (
            "entity_version_id".to_owned(),
            CanonicalValue::String(prepared.plan.entity_version_id.to_string()),
        ),
        (
            "state_digest".to_owned(),
            CanonicalValue::String(prepared.plan.state_digest.to_hex()),
        ),
    ])
}

fn evolution_relation_payload_value(relation: &PreparedRelation) -> Result<CanonicalValue> {
    CanonicalValue::object(vec![
        (
            "relation_id".to_owned(),
            CanonicalValue::String(relation.relation_id.to_string()),
        ),
        (
            "relation_version_id".to_owned(),
            CanonicalValue::String(relation.relation_version_id.to_string()),
        ),
        (
            "state_digest".to_owned(),
            CanonicalValue::String(relation.state_digest.to_hex()),
        ),
        (
            "source_entity_id".to_owned(),
            CanonicalValue::String(relation.source_entity_id.to_string()),
        ),
        (
            "target_entity_id".to_owned(),
            CanonicalValue::String(relation.target_entity_id.to_string()),
        ),
    ])
}

fn evolution_disposition_relation_payload_value(
    relation: &PreparedRelation,
) -> Result<CanonicalValue> {
    let CanonicalValue::Object(mut entries) = evolution_relation_payload_value(relation)? else {
        unreachable!("relation payload is an object")
    };
    entries.push((
        "relation_type".to_owned(),
        CanonicalValue::String(relation.relation_type.clone()),
    ));
    CanonicalValue::object(entries)
}

fn evolution_operation_values(
    prepared: &PreparedEvolution,
    relations: &[PreparedRelation],
) -> Result<Vec<CanonicalValue>> {
    let mut values = Vec::new();
    if prepared.plan_changed {
        values.push(entity_transition_payload_value(
            prepared.plan.entity_id,
            Some(prepared.previous_plan_entity_version_id),
            Some(prepared.plan.entity_version_id),
        )?);
    }
    for disposition in &prepared.task_dispositions {
        if let Some(next_entity) = &disposition.next_entity {
            values.push(entity_transition_payload_value(
                disposition.task_entity_id,
                Some(disposition.previous_entity_version_id),
                Some(next_entity.entity_version_id),
            )?);
        }
    }
    if let Some(new_plan) = &prepared.new_plan {
        values.push(entity_operation_value(new_plan)?);
    }
    for task in &prepared.tasks {
        values.push(entity_operation_value(&task.entity)?);
        for criterion in &task.acceptance_criteria {
            values.push(entity_operation_value(&criterion.entity)?);
            for requirement in &criterion.verification_requirements {
                values.push(entity_operation_value(&requirement.entity)?);
            }
        }
    }
    for record in &prepared.records {
        values.push(entity_operation_value(&record.entity)?);
    }
    for relation in relations {
        values.push(relation_operation_value(relation)?);
    }
    Ok(values)
}

fn entity_operation_value(entity: &PreparedEntity) -> Result<CanonicalValue> {
    entity_transition_payload_value(entity.entity_id, None, Some(entity.entity_version_id))
}

fn relation_operation_value(relation: &PreparedRelation) -> Result<CanonicalValue> {
    relation_transition_payload_value(
        relation.relation_id,
        None,
        Some(relation.relation_version_id),
    )
}

fn result_from_payload(value: &CanonicalValue) -> Result<PlanAdmissionResult> {
    let json = canonical_json_string(value)?;
    let value: serde_json::Value = serde_json::from_str(&json).map_err(|error| {
        WorkVcsError::PlanInvalid(format!("plan admission payload cannot be decoded: {error}"))
    })?;
    result_from_json_value(&value, PlanAdmissionOutcome::Created)
}

fn result_from_json_value(
    value: &serde_json::Value,
    outcome: PlanAdmissionOutcome,
) -> Result<PlanAdmissionResult> {
    Ok(PlanAdmissionResult {
        outcome,
        workspace_id: WorkspaceId::parse_canonical(required_json_str(value, "workspace_id")?)?,
        branch_id: BranchId::parse_canonical(required_json_str(value, "branch_id")?)?,
        previous_head_commit_id: CommitId::parse_canonical(required_json_str(
            value,
            "previous_head_commit_id",
        )?)?,
        commit_id: CommitId::parse_canonical(required_json_str(value, "commit_id")?)?,
        changeset_id: ChangeSetId::parse_canonical(required_json_str(value, "changeset_id")?)?,
        idempotency_key: required_json_str(value, "idempotency_key")?.to_owned(),
        payload_digest: Digest::from_hex(required_json_str(value, "payload_digest")?)?,
        work_state_digest: Digest::from_hex(required_json_str(value, "work_state_digest")?)?,
        goal: parse_goal_result(required_json_object(value, "goal")?)?,
        plan: parse_entity_result(required_json_object(value, "plan")?)?,
        tasks: parse_task_results(required_json_array(value, "tasks")?)?,
        records: parse_record_results(required_json_array(value, "records")?)?,
        evidence: parse_evidence_results(required_json_array(value, "evidence")?)?,
    })
}

fn parse_goal_result(value: &serde_json::Value) -> Result<PlanAdmissionGoalResult> {
    Ok(PlanAdmissionGoalResult {
        created: value
            .get("created")
            .and_then(|value| value.as_bool())
            .unwrap_or(false),
        entity_id: EntityId::parse_canonical(required_json_str(value, "entity_id")?)?,
        entity_version_id: EntityVersionId::parse_canonical(required_json_str(
            value,
            "entity_version_id",
        )?)?,
    })
}

fn parse_entity_result(value: &serde_json::Value) -> Result<PlanAdmissionEntityResult> {
    Ok(PlanAdmissionEntityResult {
        entity_id: EntityId::parse_canonical(required_json_str(value, "entity_id")?)?,
        entity_version_id: EntityVersionId::parse_canonical(required_json_str(
            value,
            "entity_version_id",
        )?)?,
        state_digest: Digest::from_hex(required_json_str(value, "state_digest")?)?,
    })
}

fn parse_task_results(values: &[serde_json::Value]) -> Result<Vec<PlanAdmissionTaskResult>> {
    values
        .iter()
        .map(|value| {
            let entity = parse_entity_result(value)?;
            Ok(PlanAdmissionTaskResult {
                local_id: required_json_str(value, "local_id")?.to_owned(),
                entity_id: entity.entity_id,
                entity_version_id: entity.entity_version_id,
                state_digest: entity.state_digest,
                acceptance_criteria: parse_acceptance_results(required_json_array(
                    value,
                    "acceptance_criteria",
                )?)?,
            })
        })
        .collect()
}

fn parse_acceptance_results(
    values: &[serde_json::Value],
) -> Result<Vec<PlanAdmissionAcceptanceCriterionResult>> {
    values
        .iter()
        .map(|value| {
            let entity = parse_entity_result(value)?;
            Ok(PlanAdmissionAcceptanceCriterionResult {
                local_id: required_json_str(value, "local_id")?.to_owned(),
                entity_id: entity.entity_id,
                entity_version_id: entity.entity_version_id,
                state_digest: entity.state_digest,
                verification_requirements: parse_verification_requirement_results(
                    required_json_array(value, "verification_requirements")?,
                )?,
            })
        })
        .collect()
}

fn parse_verification_requirement_results(
    values: &[serde_json::Value],
) -> Result<Vec<PlanAdmissionVerificationRequirementResult>> {
    values
        .iter()
        .map(|value| {
            let entity = parse_entity_result(value)?;
            Ok(PlanAdmissionVerificationRequirementResult {
                local_id: required_json_str(value, "local_id")?.to_owned(),
                entity_id: entity.entity_id,
                entity_version_id: entity.entity_version_id,
                state_digest: entity.state_digest,
            })
        })
        .collect()
}

fn parse_record_results(values: &[serde_json::Value]) -> Result<Vec<PlanAdmissionRecordResult>> {
    values
        .iter()
        .map(|value| {
            let entity = parse_entity_result(value)?;
            Ok(PlanAdmissionRecordResult {
                local_id: required_json_str(value, "local_id")?.to_owned(),
                kind: parse_record_kind(required_json_str(value, "kind")?)?,
                entity_id: entity.entity_id,
                entity_version_id: entity.entity_version_id,
                state_digest: entity.state_digest,
            })
        })
        .collect()
}

fn parse_evidence_results(
    values: &[serde_json::Value],
) -> Result<Vec<PlanAdmissionEvidenceResult>> {
    values
        .iter()
        .map(|value| {
            Ok(PlanAdmissionEvidenceResult {
                local_id: required_json_str(value, "local_id")?.to_owned(),
                evidence_id: EvidenceId::parse_canonical(required_json_str(value, "evidence_id")?)?,
            })
        })
        .collect()
}

fn evolution_result_from_payload(value: &CanonicalValue) -> Result<PlanEvolutionResult> {
    let json = canonical_json_string(value)?;
    let value: serde_json::Value = serde_json::from_str(&json).map_err(|error| {
        WorkVcsError::PlanInvalid(format!("plan evolution payload cannot be decoded: {error}"))
    })?;
    evolution_result_from_json_value(&value, PlanEvolutionOutcome::Created)
}

fn evolution_result_from_json_value(
    value: &serde_json::Value,
    outcome: PlanEvolutionOutcome,
) -> Result<PlanEvolutionResult> {
    let schema_version = required_json_i64(value, "schema_version")?;
    let mode = required_json_str(value, "mode")?.to_owned();
    let (plan_changed, parent_entity_id, parent_kind, task_dispositions, disposition_relations) =
        match schema_version {
            1 => {
                if !matches!(mode.as_str(), "in_place" | "supersede") {
                    return Err(WorkVcsError::PlanInvalid(format!(
                        "schema 1 evolution payload has invalid mode {mode:?}"
                    )));
                }
                (true, None, None, Vec::new(), Vec::new())
            }
            2 => {
                let plan_changed = value
                    .get("plan_changed")
                    .and_then(|value| value.as_bool())
                    .ok_or_else(|| {
                        WorkVcsError::PlanInvalid(
                            "schema 2 evolution payload missing boolean plan_changed".to_owned(),
                        )
                    })?;
                match mode.as_str() {
                    "supersede" if plan_changed => {}
                    "task_disposition" if !plan_changed => {}
                    "supersede" | "task_disposition" => {
                        return Err(WorkVcsError::PlanInvalid(format!(
                            "schema 2 evolution mode {mode:?} is inconsistent with plan_changed={plan_changed}"
                        )));
                    }
                    _ => {
                        return Err(WorkVcsError::PlanInvalid(format!(
                            "schema 2 evolution payload has invalid mode {mode:?}"
                        )));
                    }
                }
                let parent_entity_id = required_optional_json_str(value, "parent_entity_id")?
                    .map(EntityId::parse_canonical)
                    .transpose()?;
                let parent_kind = required_optional_json_str(value, "parent_kind")?
                    .map(parse_primary_containment_kind)
                    .transpose()?;
                if parent_entity_id.is_some() != parent_kind.is_some() {
                    return Err(WorkVcsError::PlanInvalid(
                        "schema 2 evolution parent_entity_id and parent_kind must both be null or both be present"
                            .to_owned(),
                    ));
                }
                let task_dispositions = parse_task_disposition_results(
                    required_json_array(value, "task_dispositions")?,
                    true,
                )?;
                let disposition_relations = required_json_array(value, "disposition_relations")?
                    .iter()
                    .map(parse_evolution_relation_result)
                    .collect::<Result<Vec<_>>>()?;
                (
                    plan_changed,
                    parent_entity_id,
                    parent_kind,
                    task_dispositions,
                    disposition_relations,
                )
            }
            other => {
                return Err(WorkVcsError::PlanInvalid(format!(
                    "plan evolution payload schema_version {other} is not supported"
                )));
            }
        };
    Ok(PlanEvolutionResult {
        mode,
        plan_changed,
        outcome,
        workspace_id: WorkspaceId::parse_canonical(required_json_str(value, "workspace_id")?)?,
        branch_id: BranchId::parse_canonical(required_json_str(value, "branch_id")?)?,
        previous_head_commit_id: CommitId::parse_canonical(required_json_str(
            value,
            "previous_head_commit_id",
        )?)?,
        commit_id: CommitId::parse_canonical(required_json_str(value, "commit_id")?)?,
        changeset_id: ChangeSetId::parse_canonical(required_json_str(value, "changeset_id")?)?,
        idempotency_key: required_json_str(value, "idempotency_key")?.to_owned(),
        payload_digest: Digest::from_hex(required_json_str(value, "payload_digest")?)?,
        work_state_digest: Digest::from_hex(required_json_str(value, "work_state_digest")?)?,
        goal_entity_id: optional_json_str(value, "goal_entity_id")?
            .map(EntityId::parse_canonical)
            .transpose()?,
        parent_entity_id,
        parent_kind,
        plan: parse_evolution_plan_result(required_json_object(value, "plan")?)?,
        new_plan: parse_optional_entity_result(value.get("new_plan"))?,
        goal_contains_relation: parse_optional_evolution_relation_result(
            value.get("goal_contains_relation"),
        )?
        .map(|mut relation| {
            relation.relation_type = CONTAINS_RELATION_TYPE.to_owned();
            relation
        }),
        supersedes_relation: parse_optional_evolution_relation_result(
            value.get("supersedes_relation"),
        )?
        .map(|mut relation| {
            relation.relation_type = PLAN_SUPERSEDES_RELATION_TYPE.to_owned();
            relation
        }),
        task_dispositions,
        disposition_relations,
        tasks: parse_task_results(required_json_array(value, "tasks")?)?,
        records: parse_record_results(required_json_array(value, "records")?)?,
        evidence: parse_evidence_results(required_json_array(value, "evidence")?)?,
    })
}

pub(super) fn validate_state_preserving_task_disposition_payload_json(
    connection: &Connection,
    parent_state: &WorkState,
    payload_json: &str,
    expected_workspace_id: WorkspaceId,
    expected_changeset_id: ChangeSetId,
    expected_commit_id: CommitId,
    expected_previous_head_commit_id: CommitId,
) -> Result<()> {
    let value: serde_json::Value = serde_json::from_str(payload_json).map_err(|error| {
        WorkVcsError::PlanInvalid(format!(
            "state-preserving task disposition payload is invalid JSON: {error}"
        ))
    })?;
    let result = evolution_result_from_json_value(&value, PlanEvolutionOutcome::Reused)?;
    if required_json_str(&value, "operation")? != EVOLUTION_OPERATION_TYPE
        || result.workspace_id != expected_workspace_id
        || result.changeset_id != expected_changeset_id
        || result.commit_id != expected_commit_id
        || result.previous_head_commit_id != expected_previous_head_commit_id
        || result.work_state_digest != work_state_mapping_digest(parent_state)
    {
        return Err(WorkVcsError::PlanInvalid(
            "state-preserving task disposition envelope does not match replayed commit".to_owned(),
        ));
    }
    if result.mode != "task_disposition" || result.plan_changed {
        return Err(WorkVcsError::PlanInvalid(
            "empty plan.evolve operations require schema 2 task_disposition with plan_changed=false"
                .to_owned(),
        ));
    }
    if result.task_dispositions.is_empty()
        || result
            .task_dispositions
            .iter()
            .any(|item| item.action != PlanTaskDispositionAction::RetainOriginal)
    {
        return Err(WorkVcsError::PlanInvalid(
            "state-preserving task disposition requires one or more retain_original assessments"
                .to_owned(),
        ));
    }
    if result.plan.previous_entity_version_id != result.plan.entity_version_id
        || result.plan.previous_state_digest != result.plan.state_digest
    {
        return Err(WorkVcsError::PlanInvalid(
            "state-preserving task disposition changed the retired Plan guard".to_owned(),
        ));
    }
    validate_state_preserving_entity_binding(
        connection,
        parent_state,
        expected_workspace_id,
        result.plan.entity_id,
        result.plan.entity_version_id,
        result.plan.state_digest,
        PLAN_ENTITY_KIND,
    )?;
    let mut task_ids = BTreeSet::new();
    if result.task_dispositions.iter().any(|item| {
        item.previous_entity_version_id != item.entity_version_id
            || item.previous_state_digest != item.state_digest
            || item.replacement_task_local_id.is_some()
            || item.replacement_task_entity_id.is_some()
    }) {
        return Err(WorkVcsError::PlanInvalid(
            "retain_original task disposition contains a Task delta or replacement".to_owned(),
        ));
    }
    for item in &result.task_dispositions {
        if !task_ids.insert(item.task_entity_id) {
            return Err(WorkVcsError::PlanInvalid(format!(
                "state-preserving task disposition repeats Task {}",
                item.task_entity_id
            )));
        }
        validate_state_preserving_entity_binding(
            connection,
            parent_state,
            expected_workspace_id,
            item.task_entity_id,
            item.entity_version_id,
            item.state_digest,
            TASK_ENTITY_KIND,
        )?;
    }
    for item in required_json_array(&value, "task_dispositions")? {
        if required_optional_json_str(item, "replacement_task_local_id")?.is_some()
            || required_optional_json_str(item, "replacement_task_entity_id")?.is_some()
        {
            return Err(WorkVcsError::PlanInvalid(
                "retain_original replacement fields must be explicit null".to_owned(),
            ));
        }
        required_json_object(item, "rationale")?;
    }
    if !result.tasks.is_empty()
        || !result.records.is_empty()
        || !result.evidence.is_empty()
        || !result.disposition_relations.is_empty()
        || result.new_plan.is_some()
        || result.goal_contains_relation.is_some()
        || result.supersedes_relation.is_some()
    {
        return Err(WorkVcsError::PlanInvalid(
            "state-preserving task disposition payload contains a declared state change".to_owned(),
        ));
    }
    if !required_json_array(&value, "operations")?.is_empty() {
        return Err(WorkVcsError::PlanInvalid(
            "state-preserving task disposition payload operations must be empty".to_owned(),
        ));
    }
    Ok(())
}

fn validate_state_preserving_entity_binding(
    connection: &Connection,
    parent_state: &WorkState,
    workspace_id: WorkspaceId,
    entity_id: EntityId,
    entity_version_id: EntityVersionId,
    expected_state_digest: Digest,
    expected_kind: &str,
) -> Result<()> {
    let mapped_version = parent_state
        .entities()
        .iter()
        .find_map(|(mapped_id, mapped_version)| {
            (*mapped_id == entity_id).then_some(*mapped_version)
        })
        .ok_or_else(|| {
            WorkVcsError::PlanInvalid(format!(
                "state-preserving assessment entity {entity_id} is absent from the parent WorkState"
            ))
        })?;
    if mapped_version != entity_version_id {
        return Err(WorkVcsError::PlanInvalid(format!(
            "state-preserving assessment entity {entity_id} version {entity_version_id} does not match parent WorkState version {mapped_version}"
        )));
    }
    let row = connection
        .query_row(
            "SELECT entity.workspace_id, entity.entity_kind, entity_version.state_digest
             FROM entity
             JOIN entity_version ON entity_version.entity_id = entity.object_id
             WHERE entity.object_id = ?1 AND entity_version.entity_version_id = ?2",
            params![
                &entity_id.raw_bytes()[..],
                &entity_version_id.raw_bytes()[..]
            ],
            |row| {
                Ok((
                    row.get::<_, Vec<u8>>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Vec<u8>>(2)?,
                ))
            },
        )
        .optional()
        .map_err(storage_error)?
        .ok_or_else(|| {
            WorkVcsError::PlanInvalid(format!(
                "state-preserving assessment entity {entity_id} version {entity_version_id} does not exist"
            ))
        })?;
    let (actual_workspace, actual_kind, actual_digest) = row;
    if decode_workspace_id("entity.workspace_id", actual_workspace)? != workspace_id
        || actual_kind != expected_kind
    {
        return Err(WorkVcsError::PlanInvalid(format!(
            "state-preserving assessment entity {entity_id} is not a {expected_kind} in workspace {workspace_id}"
        )));
    }
    let actual_digest = decode_digest("entity_version.state_digest", actual_digest)?;
    if actual_digest != expected_state_digest {
        return Err(WorkVcsError::DigestInvalid(format!(
            "state-preserving assessment entity {entity_id} digest {expected_state_digest} does not match stored digest {actual_digest}"
        )));
    }
    Ok(())
}

fn parse_task_disposition_results(
    values: &[serde_json::Value],
    require_complete_v2: bool,
) -> Result<Vec<PlanTaskDispositionResult>> {
    values
        .iter()
        .map(|value| {
            if require_complete_v2 {
                required_json_object(value, "rationale")?;
            }
            let action = match required_json_str(value, "action")? {
                "supersede" => PlanTaskDispositionAction::Supersede,
                "cancel" => PlanTaskDispositionAction::Cancel,
                "continue" => PlanTaskDispositionAction::Continue,
                "retain_original" => PlanTaskDispositionAction::RetainOriginal,
                other => {
                    return Err(WorkVcsError::PlanInvalid(format!(
                        "evolution payload has unknown task disposition action {other:?}"
                    )));
                }
            };
            let post_carriers = required_json_array(value, "post_carriers")?
                .iter()
                .map(|value| {
                    value.as_str().map(str::to_owned).ok_or_else(|| {
                        WorkVcsError::PlanInvalid(
                            "task disposition post_carriers must contain strings".to_owned(),
                        )
                    })
                })
                .collect::<Result<Vec<_>>>()?;
            let risks = if require_complete_v2 {
                required_json_array(value, "risks")?
                    .iter()
                    .map(|value| {
                        value.as_str().map(str::to_owned).ok_or_else(|| {
                            WorkVcsError::PlanInvalid(
                                "task disposition risks must contain strings".to_owned(),
                            )
                        })
                    })
                    .collect::<Result<Vec<_>>>()?
            } else {
                value
                    .get("risks")
                    .and_then(|value| value.as_array())
                    .map(|values| {
                        values
                            .iter()
                            .map(|value| {
                                value.as_str().map(str::to_owned).ok_or_else(|| {
                                    WorkVcsError::PlanInvalid(
                                        "task disposition risks must contain strings".to_owned(),
                                    )
                                })
                            })
                            .collect::<Result<Vec<_>>>()
                    })
                    .transpose()?
                    .unwrap_or_default()
            };
            Ok(PlanTaskDispositionResult {
                task_entity_id: EntityId::parse_canonical(required_json_str(
                    value,
                    "task_entity_id",
                )?)?,
                previous_entity_version_id: EntityVersionId::parse_canonical(required_json_str(
                    value,
                    "previous_entity_version_id",
                )?)?,
                previous_state_digest: Digest::from_hex(required_json_str(
                    value,
                    "previous_state_digest",
                )?)?,
                action,
                replacement_task_local_id: if require_complete_v2 {
                    required_optional_json_str(value, "replacement_task_local_id")?
                } else {
                    optional_json_str(value, "replacement_task_local_id")?
                }
                .map(str::to_owned),
                replacement_task_entity_id: if require_complete_v2 {
                    required_optional_json_str(value, "replacement_task_entity_id")?
                } else {
                    optional_json_str(value, "replacement_task_entity_id")?
                }
                .map(EntityId::parse_canonical)
                .transpose()?,
                entity_version_id: EntityVersionId::parse_canonical(required_json_str(
                    value,
                    "entity_version_id",
                )?)?,
                state_digest: Digest::from_hex(required_json_str(value, "state_digest")?)?,
                post_current: value
                    .get("post_current")
                    .and_then(|value| value.as_bool())
                    .ok_or_else(|| {
                        WorkVcsError::PlanInvalid(
                            "task disposition missing boolean post_current".to_owned(),
                        )
                    })?,
                post_carriers,
                return_condition: required_json_str(value, "return_condition")?.to_owned(),
                risks,
            })
        })
        .collect()
}

fn parse_evolution_relation_result(
    value: &serde_json::Value,
) -> Result<PlanEvolutionRelationResult> {
    Ok(PlanEvolutionRelationResult {
        relation_type: required_json_str(value, "relation_type")?.to_owned(),
        relation_id: RelationId::parse_canonical(required_json_str(value, "relation_id")?)?,
        relation_version_id: RelationVersionId::parse_canonical(required_json_str(
            value,
            "relation_version_id",
        )?)?,
        state_digest: Digest::from_hex(required_json_str(value, "state_digest")?)?,
        source_entity_id: EntityId::parse_canonical(required_json_str(value, "source_entity_id")?)?,
        target_entity_id: EntityId::parse_canonical(required_json_str(value, "target_entity_id")?)?,
    })
}

fn parse_primary_containment_kind(value: &str) -> Result<PrimaryContainmentEndpointKind> {
    match value {
        "goal" => Ok(PrimaryContainmentEndpointKind::Goal),
        "plan" => Ok(PrimaryContainmentEndpointKind::Plan),
        "task" => Ok(PrimaryContainmentEndpointKind::Task),
        other => Err(WorkVcsError::PlanInvalid(format!(
            "evolution payload has invalid parent kind {other:?}"
        ))),
    }
}

fn optional_json_str<'a>(value: &'a serde_json::Value, field: &str) -> Result<Option<&'a str>> {
    match value.get(field) {
        None | Some(serde_json::Value::Null) => Ok(None),
        Some(value) => value.as_str().map(Some).ok_or_else(|| {
            WorkVcsError::PlanInvalid(format!(
                "admission payload field {field} must be string or null"
            ))
        }),
    }
}

fn required_optional_json_str<'a>(
    value: &'a serde_json::Value,
    field: &str,
) -> Result<Option<&'a str>> {
    if value.get(field).is_none() {
        return Err(WorkVcsError::PlanInvalid(format!(
            "evolution payload missing nullable field {field}"
        )));
    }
    optional_json_str(value, field)
}

fn required_json_i64(value: &serde_json::Value, field: &str) -> Result<i64> {
    value
        .get(field)
        .and_then(|value| value.as_i64())
        .ok_or_else(|| {
            WorkVcsError::PlanInvalid(format!("plan evolution payload missing integer {field}"))
        })
}

fn parse_evolution_plan_result(value: &serde_json::Value) -> Result<PlanEvolutionPlanResult> {
    Ok(PlanEvolutionPlanResult {
        entity_id: EntityId::parse_canonical(required_json_str(value, "entity_id")?)?,
        previous_entity_version_id: EntityVersionId::parse_canonical(required_json_str(
            value,
            "previous_entity_version_id",
        )?)?,
        previous_state_digest: Digest::from_hex(required_json_str(
            value,
            "previous_state_digest",
        )?)?,
        entity_version_id: EntityVersionId::parse_canonical(required_json_str(
            value,
            "entity_version_id",
        )?)?,
        state_digest: Digest::from_hex(required_json_str(value, "state_digest")?)?,
    })
}

fn parse_optional_entity_result(
    value: Option<&serde_json::Value>,
) -> Result<Option<PlanAdmissionEntityResult>> {
    match value {
        Some(value) if value.is_null() => Ok(None),
        Some(value) => parse_entity_result(value).map(Some),
        None => Ok(None),
    }
}

fn parse_optional_evolution_relation_result(
    value: Option<&serde_json::Value>,
) -> Result<Option<PlanEvolutionRelationResult>> {
    match value {
        Some(value) if value.is_null() => Ok(None),
        Some(value) => Ok(Some(PlanEvolutionRelationResult {
            relation_type: String::new(),
            relation_id: RelationId::parse_canonical(required_json_str(value, "relation_id")?)?,
            relation_version_id: RelationVersionId::parse_canonical(required_json_str(
                value,
                "relation_version_id",
            )?)?,
            state_digest: Digest::from_hex(required_json_str(value, "state_digest")?)?,
            source_entity_id: EntityId::parse_canonical(required_json_str(
                value,
                "source_entity_id",
            )?)?,
            target_entity_id: EntityId::parse_canonical(required_json_str(
                value,
                "target_entity_id",
            )?)?,
        })),
        None => Ok(None),
    }
}

fn required_json_object<'a>(
    value: &'a serde_json::Value,
    field: &str,
) -> Result<&'a serde_json::Value> {
    value
        .get(field)
        .filter(|value| value.is_object())
        .ok_or_else(|| {
            WorkVcsError::PlanInvalid(format!("admission payload missing object {field}"))
        })
}

fn required_json_array<'a>(
    value: &'a serde_json::Value,
    field: &str,
) -> Result<&'a [serde_json::Value]> {
    value
        .get(field)
        .and_then(|value| value.as_array())
        .map(Vec::as_slice)
        .ok_or_else(|| {
            WorkVcsError::PlanInvalid(format!("admission payload missing array {field}"))
        })
}

fn required_json_str<'a>(value: &'a serde_json::Value, field: &str) -> Result<&'a str> {
    value
        .get(field)
        .and_then(|value| value.as_str())
        .ok_or_else(|| {
            WorkVcsError::PlanInvalid(format!("admission payload missing string {field}"))
        })
}

pub(super) fn load_active_branch(
    transaction: &Transaction<'_>,
    branch_id: BranchId,
) -> Result<BranchRow> {
    let row = transaction
        .query_row(
            "SELECT workspace_id, head_commit_id, lifecycle_state
             FROM branch
             WHERE branch_id = ?1",
            params![&branch_id.raw_bytes()[..]],
            |row| {
                Ok((
                    row.get::<_, Vec<u8>>(0)?,
                    row.get::<_, Vec<u8>>(1)?,
                    row.get::<_, String>(2)?,
                ))
            },
        )
        .optional()
        .map_err(storage_error)?;
    let Some((workspace_id, head_commit_id, lifecycle_state)) = row else {
        return Err(WorkVcsError::BranchNotFound(format!(
            "branch {branch_id} does not exist"
        )));
    };
    if lifecycle_state != ACTIVE_BRANCH_LIFECYCLE_STATE {
        return Err(WorkVcsError::PlanInvalid(format!(
            "branch {branch_id} has lifecycle state {lifecycle_state:?}"
        )));
    }
    Ok(BranchRow {
        workspace_id: decode_workspace_id("branch.workspace_id", workspace_id)?,
        head_commit_id: decode_commit_id("branch.head_commit_id", head_commit_id)?,
    })
}

fn entity_transition_payload_value(
    entity_id: EntityId,
    before_entity_version_id: Option<EntityVersionId>,
    after_entity_version_id: Option<EntityVersionId>,
) -> Result<CanonicalValue> {
    let before_value = before_entity_version_id
        .map(|id| CanonicalValue::String(id.to_string()))
        .unwrap_or(CanonicalValue::Null);
    let after_value = after_entity_version_id
        .map(|id| CanonicalValue::String(id.to_string()))
        .unwrap_or(CanonicalValue::Null);
    CanonicalValue::object(vec![
        ("after_entity_version_id".to_owned(), after_value),
        ("before_entity_version_id".to_owned(), before_value),
        (
            "entity_id".to_owned(),
            CanonicalValue::String(entity_id.to_string()),
        ),
    ])
    .map_err(plan_invalid_from)
}

fn relation_transition_payload_value(
    relation_id: RelationId,
    before_relation_version_id: Option<RelationVersionId>,
    after_relation_version_id: Option<RelationVersionId>,
) -> Result<CanonicalValue> {
    let before_value = before_relation_version_id
        .map(|id| CanonicalValue::String(id.to_string()))
        .unwrap_or(CanonicalValue::Null);
    let after_value = after_relation_version_id
        .map(|id| CanonicalValue::String(id.to_string()))
        .unwrap_or(CanonicalValue::Null);
    CanonicalValue::object(vec![
        ("after_relation_version_id".to_owned(), after_value),
        ("before_relation_version_id".to_owned(), before_value),
        (
            "relation_id".to_owned(),
            CanonicalValue::String(relation_id.to_string()),
        ),
    ])
    .map_err(plan_invalid_from)
}

fn manifest_to_canonical_value(manifest: &PlanAdmissionManifest) -> Result<CanonicalValue> {
    let mut entries = vec![
        (
            "schema_version".to_owned(),
            CanonicalValue::safe_integer(manifest.schema_version)?,
        ),
        (
            "idempotency_key".to_owned(),
            CanonicalValue::String(manifest.idempotency_key.clone()),
        ),
        (
            "expected_head_commit_id".to_owned(),
            CanonicalValue::String(manifest.expected_head_commit_id.clone()),
        ),
        ("goal".to_owned(), manifest_goal_value(&manifest.goal)?),
        ("plan".to_owned(), manifest_plan_value(&manifest.plan)?),
        (
            "tasks".to_owned(),
            CanonicalValue::Array(
                manifest
                    .tasks
                    .iter()
                    .map(manifest_task_value)
                    .collect::<Result<Vec<_>>>()?,
            ),
        ),
        (
            "records".to_owned(),
            CanonicalValue::Array(
                manifest
                    .records
                    .iter()
                    .map(manifest_record_value)
                    .collect::<Result<Vec<_>>>()?,
            ),
        ),
        (
            "evidence".to_owned(),
            CanonicalValue::Array(
                manifest
                    .evidence
                    .iter()
                    .map(manifest_evidence_value)
                    .collect::<Result<Vec<_>>>()?,
            ),
        ),
        ("rationale".to_owned(), manifest.rationale.clone()),
    ];
    if let Some(expected_state_digest) = &manifest.expected_state_digest {
        entries.push((
            "expected_state_digest".to_owned(),
            CanonicalValue::String(expected_state_digest.clone()),
        ));
    }
    CanonicalValue::object(entries)
}

fn evolution_manifest_to_canonical_value(
    manifest: &PlanEvolutionManifest,
) -> Result<CanonicalValue> {
    match manifest {
        PlanEvolutionManifest::InPlace(manifest) => {
            let mut entries = vec![
                (
                    "mode".to_owned(),
                    CanonicalValue::String("in_place".to_owned()),
                ),
                (
                    "schema_version".to_owned(),
                    CanonicalValue::safe_integer(manifest.schema_version)?,
                ),
                (
                    "idempotency_key".to_owned(),
                    CanonicalValue::String(manifest.idempotency_key.clone()),
                ),
                (
                    "expected_head_commit_id".to_owned(),
                    CanonicalValue::String(manifest.expected_head_commit_id.clone()),
                ),
                (
                    "target_plan_entity_id".to_owned(),
                    CanonicalValue::String(manifest.target_plan_entity_id.clone()),
                ),
                (
                    "expected_plan_entity_version_id".to_owned(),
                    CanonicalValue::String(manifest.expected_plan_entity_version_id.clone()),
                ),
                (
                    "expected_plan_state_digest".to_owned(),
                    CanonicalValue::String(manifest.expected_plan_state_digest.clone()),
                ),
                (
                    "plan".to_owned(),
                    manifest_plan_update_value(&manifest.plan)?,
                ),
                (
                    "tasks".to_owned(),
                    CanonicalValue::Array(
                        manifest
                            .tasks
                            .iter()
                            .map(manifest_task_value)
                            .collect::<Result<Vec<_>>>()?,
                    ),
                ),
                (
                    "records".to_owned(),
                    CanonicalValue::Array(
                        manifest
                            .records
                            .iter()
                            .map(manifest_record_value)
                            .collect::<Result<Vec<_>>>()?,
                    ),
                ),
                (
                    "evidence".to_owned(),
                    CanonicalValue::Array(
                        manifest
                            .evidence
                            .iter()
                            .map(manifest_evidence_value)
                            .collect::<Result<Vec<_>>>()?,
                    ),
                ),
                ("rationale".to_owned(), manifest.rationale.clone()),
            ];
            if let Some(expected_state_digest) = &manifest.expected_state_digest {
                entries.push((
                    "expected_state_digest".to_owned(),
                    CanonicalValue::String(expected_state_digest.clone()),
                ));
            }
            CanonicalValue::object(entries)
        }
        PlanEvolutionManifest::Supersede(manifest) => {
            let mut entries = vec![
                (
                    "mode".to_owned(),
                    CanonicalValue::String("supersede".to_owned()),
                ),
                (
                    "schema_version".to_owned(),
                    CanonicalValue::safe_integer(manifest.schema_version)?,
                ),
                (
                    "idempotency_key".to_owned(),
                    CanonicalValue::String(manifest.idempotency_key.clone()),
                ),
                (
                    "expected_head_commit_id".to_owned(),
                    CanonicalValue::String(manifest.expected_head_commit_id.clone()),
                ),
                (
                    "target_plan_entity_id".to_owned(),
                    CanonicalValue::String(manifest.target_plan_entity_id.clone()),
                ),
                (
                    "expected_plan_entity_version_id".to_owned(),
                    CanonicalValue::String(manifest.expected_plan_entity_version_id.clone()),
                ),
                (
                    "expected_plan_state_digest".to_owned(),
                    CanonicalValue::String(manifest.expected_plan_state_digest.clone()),
                ),
                (
                    "plan".to_owned(),
                    manifest_supersede_plan_value(&manifest.plan)?,
                ),
                (
                    "tasks".to_owned(),
                    CanonicalValue::Array(
                        manifest
                            .tasks
                            .iter()
                            .map(manifest_task_value)
                            .collect::<Result<Vec<_>>>()?,
                    ),
                ),
                (
                    "records".to_owned(),
                    CanonicalValue::Array(
                        manifest
                            .records
                            .iter()
                            .map(manifest_record_value)
                            .collect::<Result<Vec<_>>>()?,
                    ),
                ),
                (
                    "evidence".to_owned(),
                    CanonicalValue::Array(
                        manifest
                            .evidence
                            .iter()
                            .map(manifest_evidence_value)
                            .collect::<Result<Vec<_>>>()?,
                    ),
                ),
                ("rationale".to_owned(), manifest.rationale.clone()),
            ];
            if manifest.schema_version == 1 {
                entries.extend([
                    (
                        "expected_goal_entity_id".to_owned(),
                        CanonicalValue::String(
                            manifest
                                .expected_goal_entity_id
                                .clone()
                                .expect("validated schema 1 goal id"),
                        ),
                    ),
                    (
                        "expected_goal_entity_version_id".to_owned(),
                        CanonicalValue::String(
                            manifest
                                .expected_goal_entity_version_id
                                .clone()
                                .expect("validated schema 1 goal version"),
                        ),
                    ),
                    (
                        "expected_goal_plan_relation_id".to_owned(),
                        CanonicalValue::String(
                            manifest
                                .expected_goal_plan_relation_id
                                .clone()
                                .expect("validated schema 1 relation id"),
                        ),
                    ),
                    (
                        "expected_goal_plan_relation_version_id".to_owned(),
                        CanonicalValue::String(
                            manifest
                                .expected_goal_plan_relation_version_id
                                .clone()
                                .expect("validated schema 1 relation version"),
                        ),
                    ),
                ]);
            } else {
                entries.push((
                    "expected_parent".to_owned(),
                    manifest_expected_parent_value(
                        manifest
                            .expected_parent
                            .as_ref()
                            .expect("validated schema 2 parent"),
                    )?,
                ));
                entries.push((
                    "task_dispositions".to_owned(),
                    CanonicalValue::Array(
                        manifest
                            .task_dispositions
                            .iter()
                            .map(manifest_task_disposition_value)
                            .collect::<Result<Vec<_>>>()?,
                    ),
                ));
                entries.push((
                    "nested_plan_continuations".to_owned(),
                    CanonicalValue::Array(
                        manifest
                            .nested_plan_continuations
                            .iter()
                            .map(manifest_nested_plan_continuation_value)
                            .collect::<Result<Vec<_>>>()?,
                    ),
                ));
            }
            if let Some(expected_state_digest) = &manifest.expected_state_digest {
                entries.push((
                    "expected_state_digest".to_owned(),
                    CanonicalValue::String(expected_state_digest.clone()),
                ));
            }
            CanonicalValue::object(entries)
        }
        PlanEvolutionManifest::TaskDisposition(manifest) => {
            let mut entries = vec![
                (
                    "mode".to_owned(),
                    CanonicalValue::String("task_disposition".to_owned()),
                ),
                (
                    "schema_version".to_owned(),
                    CanonicalValue::safe_integer(manifest.schema_version)?,
                ),
                (
                    "idempotency_key".to_owned(),
                    CanonicalValue::String(manifest.idempotency_key.clone()),
                ),
                (
                    "expected_head_commit_id".to_owned(),
                    CanonicalValue::String(manifest.expected_head_commit_id.clone()),
                ),
                (
                    "target_plan_entity_id".to_owned(),
                    CanonicalValue::String(manifest.target_plan_entity_id.clone()),
                ),
                (
                    "expected_plan_entity_version_id".to_owned(),
                    CanonicalValue::String(manifest.expected_plan_entity_version_id.clone()),
                ),
                (
                    "expected_plan_state_digest".to_owned(),
                    CanonicalValue::String(manifest.expected_plan_state_digest.clone()),
                ),
                (
                    "carrier".to_owned(),
                    manifest
                        .carrier
                        .as_ref()
                        .map(manifest_disposition_carrier_value)
                        .transpose()?
                        .unwrap_or(CanonicalValue::Null),
                ),
                (
                    "tasks".to_owned(),
                    CanonicalValue::Array(
                        manifest
                            .tasks
                            .iter()
                            .map(manifest_task_value)
                            .collect::<Result<Vec<_>>>()?,
                    ),
                ),
                (
                    "task_dispositions".to_owned(),
                    CanonicalValue::Array(
                        manifest
                            .task_dispositions
                            .iter()
                            .map(manifest_task_disposition_value)
                            .collect::<Result<Vec<_>>>()?,
                    ),
                ),
                ("rationale".to_owned(), manifest.rationale.clone()),
            ];
            if let Some(expected_state_digest) = &manifest.expected_state_digest {
                entries.push((
                    "expected_state_digest".to_owned(),
                    CanonicalValue::String(expected_state_digest.clone()),
                ));
            }
            CanonicalValue::object(entries)
        }
    }
}

fn manifest_disposition_carrier_value(
    carrier: &PlanTaskDispositionCarrierManifest,
) -> Result<CanonicalValue> {
    let (kind, entity_id, expected_entity_version_id, expected_state_digest) = match carrier {
        PlanTaskDispositionCarrierManifest::Goal {
            entity_id,
            expected_entity_version_id,
            expected_state_digest,
        } => (
            "goal",
            entity_id,
            expected_entity_version_id,
            expected_state_digest,
        ),
        PlanTaskDispositionCarrierManifest::Plan {
            entity_id,
            expected_entity_version_id,
            expected_state_digest,
        } => (
            "plan",
            entity_id,
            expected_entity_version_id,
            expected_state_digest,
        ),
    };
    CanonicalValue::object(vec![
        ("kind".to_owned(), CanonicalValue::String(kind.to_owned())),
        (
            "entity_id".to_owned(),
            CanonicalValue::String(entity_id.clone()),
        ),
        (
            "expected_entity_version_id".to_owned(),
            CanonicalValue::String(expected_entity_version_id.clone()),
        ),
        (
            "expected_state_digest".to_owned(),
            CanonicalValue::String(expected_state_digest.clone()),
        ),
    ])
}

fn manifest_expected_parent_value(
    parent: &PlanEvolutionExpectedParentManifest,
) -> Result<CanonicalValue> {
    match parent {
        PlanEvolutionExpectedParentManifest::None => CanonicalValue::object(vec![(
            "kind".to_owned(),
            CanonicalValue::String("none".to_owned()),
        )]),
        PlanEvolutionExpectedParentManifest::Goal {
            entity_id,
            expected_entity_version_id,
            containment_relation_id,
            containment_relation_version_id,
        }
        | PlanEvolutionExpectedParentManifest::Plan {
            entity_id,
            expected_entity_version_id,
            containment_relation_id,
            containment_relation_version_id,
        } => {
            let kind = if matches!(parent, PlanEvolutionExpectedParentManifest::Goal { .. }) {
                "goal"
            } else {
                "plan"
            };
            CanonicalValue::object(vec![
                ("kind".to_owned(), CanonicalValue::String(kind.to_owned())),
                (
                    "entity_id".to_owned(),
                    CanonicalValue::String(entity_id.clone()),
                ),
                (
                    "expected_entity_version_id".to_owned(),
                    CanonicalValue::String(expected_entity_version_id.clone()),
                ),
                (
                    "containment_relation_id".to_owned(),
                    CanonicalValue::String(containment_relation_id.clone()),
                ),
                (
                    "containment_relation_version_id".to_owned(),
                    CanonicalValue::String(containment_relation_version_id.clone()),
                ),
            ])
        }
    }
}

fn manifest_task_disposition_value(item: &PlanTaskDispositionManifest) -> Result<CanonicalValue> {
    let mut entries = vec![
        (
            "task_entity_id".to_owned(),
            CanonicalValue::String(item.task_entity_id.clone()),
        ),
        (
            "expected_task_entity_version_id".to_owned(),
            CanonicalValue::String(item.expected_task_entity_version_id.clone()),
        ),
        (
            "expected_task_state_digest".to_owned(),
            CanonicalValue::String(item.expected_task_state_digest.clone()),
        ),
        (
            "action".to_owned(),
            CanonicalValue::String(item.action.as_str().to_owned()),
        ),
        (
            "return_condition".to_owned(),
            CanonicalValue::String(item.return_condition.clone()),
        ),
        ("rationale".to_owned(), item.rationale.clone()),
    ];
    if let Some(local_id) = &item.replacement_task_local_id {
        entries.push((
            "replacement_task_local_id".to_owned(),
            CanonicalValue::String(local_id.clone()),
        ));
    }
    CanonicalValue::object(entries)
}

fn manifest_nested_plan_continuation_value(
    item: &NestedPlanContinuationManifest,
) -> Result<CanonicalValue> {
    CanonicalValue::object(vec![
        (
            "plan_entity_id".to_owned(),
            CanonicalValue::String(item.plan_entity_id.clone()),
        ),
        (
            "expected_plan_entity_version_id".to_owned(),
            CanonicalValue::String(item.expected_plan_entity_version_id.clone()),
        ),
        (
            "expected_plan_state_digest".to_owned(),
            CanonicalValue::String(item.expected_plan_state_digest.clone()),
        ),
    ])
}

fn manifest_plan_update_value(update: &PlanEvolutionPlanUpdateManifest) -> Result<CanonicalValue> {
    let mut entries = Vec::new();
    if let Some(description) = &update.description {
        entries.push((
            "description".to_owned(),
            CanonicalValue::String(description.clone()),
        ));
    }
    if let Some(strategy) = &update.strategy {
        entries.push((
            "strategy".to_owned(),
            CanonicalValue::String(strategy.clone()),
        ));
    }
    if let Some(constraints) = &update.constraints {
        entries.push((
            "constraints".to_owned(),
            CanonicalValue::Array(
                constraints
                    .iter()
                    .cloned()
                    .map(CanonicalValue::String)
                    .collect(),
            ),
        ));
    }
    CanonicalValue::object(entries)
}

fn manifest_supersede_plan_value(
    plan: &PlanEvolutionSupersedePlanManifest,
) -> Result<CanonicalValue> {
    CanonicalValue::object(vec![
        (
            "description".to_owned(),
            CanonicalValue::String(plan.description.clone()),
        ),
        (
            "strategy".to_owned(),
            CanonicalValue::String(plan.strategy.clone()),
        ),
        (
            "constraints".to_owned(),
            manifest_supersede_constraints_value(&plan.constraints)?,
        ),
    ])
}

fn manifest_supersede_constraints_value(
    constraints: &PlanEvolutionSupersedeConstraintsManifest,
) -> Result<CanonicalValue> {
    match constraints {
        PlanEvolutionSupersedeConstraintsManifest::CarryAll => CanonicalValue::object(vec![(
            "mode".to_owned(),
            CanonicalValue::String("carry_all".to_owned()),
        )]),
        PlanEvolutionSupersedeConstraintsManifest::Replace { values } => {
            CanonicalValue::object(vec![
                (
                    "mode".to_owned(),
                    CanonicalValue::String("replace".to_owned()),
                ),
                (
                    "values".to_owned(),
                    CanonicalValue::Array(
                        values.iter().cloned().map(CanonicalValue::String).collect(),
                    ),
                ),
            ])
        }
    }
}

fn manifest_goal_value(goal: &PlanAdmissionGoalManifest) -> Result<CanonicalValue> {
    match goal {
        PlanAdmissionGoalManifest::Create { description } => CanonicalValue::object(vec![
            (
                "mode".to_owned(),
                CanonicalValue::String("create".to_owned()),
            ),
            (
                "description".to_owned(),
                CanonicalValue::String(description.clone()),
            ),
        ]),
        PlanAdmissionGoalManifest::Existing {
            entity_id,
            expected_entity_version_id,
        } => {
            let mut entries = vec![
                (
                    "mode".to_owned(),
                    CanonicalValue::String("existing".to_owned()),
                ),
                (
                    "entity_id".to_owned(),
                    CanonicalValue::String(entity_id.clone()),
                ),
            ];
            if let Some(expected_entity_version_id) = expected_entity_version_id {
                entries.push((
                    "expected_entity_version_id".to_owned(),
                    CanonicalValue::String(expected_entity_version_id.clone()),
                ));
            }
            CanonicalValue::object(entries)
        }
    }
}

fn manifest_plan_value(plan: &PlanAdmissionPlanManifest) -> Result<CanonicalValue> {
    CanonicalValue::object(vec![
        (
            "description".to_owned(),
            CanonicalValue::String(plan.description.clone()),
        ),
        (
            "strategy".to_owned(),
            CanonicalValue::String(plan.strategy.clone()),
        ),
        (
            "constraints".to_owned(),
            CanonicalValue::Array(
                plan.constraints
                    .iter()
                    .cloned()
                    .map(CanonicalValue::String)
                    .collect(),
            ),
        ),
    ])
}

fn manifest_task_value(task: &PlanAdmissionTaskManifest) -> Result<CanonicalValue> {
    let mut entries = vec![
        (
            "local_id".to_owned(),
            CanonicalValue::String(task.local_id.clone()),
        ),
        (
            "description".to_owned(),
            CanonicalValue::String(task.description.clone()),
        ),
        (
            "acceptance_criteria".to_owned(),
            CanonicalValue::Array(
                task.acceptance_criteria
                    .iter()
                    .map(manifest_acceptance_value)
                    .collect::<Result<Vec<_>>>()?,
            ),
        ),
    ];
    if let Some(priority) = task.priority {
        entries.push((
            "priority".to_owned(),
            CanonicalValue::safe_integer(priority)?,
        ));
    }
    CanonicalValue::object(entries)
}

fn manifest_acceptance_value(
    criterion: &PlanAdmissionAcceptanceCriterionManifest,
) -> Result<CanonicalValue> {
    CanonicalValue::object(vec![
        (
            "local_id".to_owned(),
            CanonicalValue::String(criterion.local_id.clone()),
        ),
        (
            "statement".to_owned(),
            CanonicalValue::String(criterion.statement.clone()),
        ),
        (
            "classification".to_owned(),
            CanonicalValue::String(criterion.classification.clone()),
        ),
        (
            "verification_requirements".to_owned(),
            CanonicalValue::Array(
                criterion
                    .verification_requirements
                    .iter()
                    .map(manifest_requirement_value)
                    .collect::<Result<Vec<_>>>()?,
            ),
        ),
    ])
}

fn manifest_requirement_value(
    requirement: &PlanAdmissionVerificationRequirementManifest,
) -> Result<CanonicalValue> {
    CanonicalValue::object(vec![
        (
            "local_id".to_owned(),
            CanonicalValue::String(requirement.local_id.clone()),
        ),
        (
            "statement".to_owned(),
            CanonicalValue::String(requirement.statement.clone()),
        ),
    ])
}

fn manifest_record_value(record: &PlanAdmissionRecordManifest) -> Result<CanonicalValue> {
    CanonicalValue::object(vec![
        (
            "local_id".to_owned(),
            CanonicalValue::String(record.local_id.clone()),
        ),
        (
            "kind".to_owned(),
            CanonicalValue::String(record.kind.clone()),
        ),
        (
            "statement".to_owned(),
            CanonicalValue::String(record.statement.clone()),
        ),
        ("scope".to_owned(), record.scope.clone()),
    ])
}

fn manifest_evidence_value(evidence: &PlanAdmissionEvidenceManifest) -> Result<CanonicalValue> {
    CanonicalValue::object(vec![
        (
            "local_id".to_owned(),
            CanonicalValue::String(evidence.local_id.clone()),
        ),
        (
            "kind".to_owned(),
            CanonicalValue::String(evidence.kind.clone()),
        ),
        ("metadata".to_owned(), evidence.metadata.clone()),
    ])
}

fn record_state(kind: RecordKind, statement: String, scope: CanonicalValue) -> Result<RecordState> {
    let state = match kind {
        RecordKind::Assumption => RecordState::assumption(statement)?,
        RecordKind::Attempt => RecordState::attempt(statement)?,
        RecordKind::Decision => RecordState::decision(statement)?,
        RecordKind::Finding => RecordState::finding(statement)?,
        RecordKind::Handoff => RecordState::handoff(statement)?,
        RecordKind::Question => RecordState::question(statement)?,
        RecordKind::Risk => RecordState::risk(statement)?,
        RecordKind::AuthorizationReceipt => {
            return Err(WorkVcsError::RecordInvalid(
                "authorization_receipt records must use the receipt issue API".to_owned(),
            ));
        }
    };
    state.with_scope(scope)
}

fn parse_record_kind(value: &str) -> Result<RecordKind> {
    match value {
        "assumption" => Ok(RecordKind::Assumption),
        "attempt" => Ok(RecordKind::Attempt),
        "authorization_receipt" => Ok(RecordKind::AuthorizationReceipt),
        "decision" => Ok(RecordKind::Decision),
        "finding" => Ok(RecordKind::Finding),
        "handoff" => Ok(RecordKind::Handoff),
        "question" | "unknown" => Ok(RecordKind::Question),
        "risk" => Ok(RecordKind::Risk),
        other => Err(WorkVcsError::RecordInvalid(format!(
            "record kind {other:?} is not implemented by plan admission"
        ))),
    }
}

fn parse_acceptance_classification(value: &str) -> Result<AcceptanceCriterionClassification> {
    match value {
        "required" => Ok(AcceptanceCriterionClassification::Required),
        "optional" => Ok(AcceptanceCriterionClassification::Optional),
        other => Err(WorkVcsError::TaskInvalid(format!(
            "acceptance criterion classification {other:?} is not in the confirmed vocabulary"
        ))),
    }
}

fn validate_idempotency_key(value: &str) -> Result<()> {
    validate_stored_text("idempotency key", value)?;
    if value.len() > 256 {
        return Err(WorkVcsError::PlanInvalid(
            "idempotency key must be at most 256 bytes".to_owned(),
        ));
    }
    Ok(())
}

fn validate_unique_local_ids<'a>(
    label: &str,
    values: impl IntoIterator<Item = &'a String>,
) -> Result<()> {
    let mut seen = BTreeSet::new();
    for value in values {
        validate_stored_text(label, value)?;
        if !seen.insert(value.clone()) {
            return Err(WorkVcsError::PlanInvalid(format!(
                "duplicate {label} local_id {value:?} in plan admission manifest"
            )));
        }
    }
    Ok(())
}

fn validate_stored_text(label: &str, value: &str) -> Result<()> {
    if value.trim().is_empty() {
        return Err(WorkVcsError::PlanInvalid(format!(
            "{label} must not be empty"
        )));
    }
    Ok(())
}

fn require_object(label: &str, value: &CanonicalValue) -> Result<()> {
    if matches!(value, CanonicalValue::Object(_)) {
        Ok(())
    } else {
        Err(WorkVcsError::PlanInvalid(format!(
            "{label} must be an object"
        )))
    }
}

fn require_non_empty_object(label: &str, value: &CanonicalValue) -> Result<()> {
    match value {
        CanonicalValue::Object(entries) if !entries.is_empty() => Ok(()),
        CanonicalValue::Object(_) => Err(WorkVcsError::PlanInvalid(format!(
            "{label} must not be an empty object"
        ))),
        _ => Err(WorkVcsError::PlanInvalid(format!(
            "{label} must be an object"
        ))),
    }
}

fn record_state_kind(value: &CanonicalValue) -> Result<RecordKind> {
    let CanonicalValue::Object(entries) = value else {
        return Err(WorkVcsError::RecordInvalid(
            "record state must be an object".to_owned(),
        ));
    };
    let Some(CanonicalValue::String(kind)) = entries
        .iter()
        .find_map(|(key, value)| (key == "kind").then_some(value))
    else {
        return Err(WorkVcsError::RecordInvalid(
            "record state missing kind".to_owned(),
        ));
    };
    parse_record_kind(kind)
}

fn canonical_json_string(value: &CanonicalValue) -> Result<String> {
    String::from_utf8(canonical_bytes(value)?).map_err(|error| {
        WorkVcsError::CanonicalEncodingInvalid(format!("canonical JSON is not UTF-8: {error}"))
    })
}

fn empty_object() -> CanonicalValue {
    CanonicalValue::object(Vec::new()).expect("empty canonical object")
}

fn default_acceptance_classification() -> String {
    "required".to_owned()
}

fn decode_workspace_id(column: &str, bytes: Vec<u8>) -> Result<WorkspaceId> {
    let bytes = decode_16(column, bytes)?;
    WorkspaceId::from_bytes(bytes).map_err(plan_invalid_from)
}

fn decode_commit_id(column: &str, bytes: Vec<u8>) -> Result<CommitId> {
    let bytes = decode_16(column, bytes)?;
    CommitId::from_bytes(bytes).map_err(plan_invalid_from)
}

fn decode_entity_id(column: &str, bytes: Vec<u8>) -> Result<EntityId> {
    let bytes = decode_16(column, bytes)?;
    EntityId::from_bytes(bytes).map_err(plan_invalid_from)
}

fn decode_digest(column: &str, bytes: Vec<u8>) -> Result<Digest> {
    let bytes: [u8; 32] = bytes.try_into().map_err(|bytes: Vec<u8>| {
        WorkVcsError::PlanInvalid(format!("{column} must be 32 bytes, found {}", bytes.len()))
    })?;
    Ok(Digest::from_bytes(bytes))
}

fn decode_16(column: &str, bytes: Vec<u8>) -> Result<[u8; 16]> {
    bytes.try_into().map_err(|bytes: Vec<u8>| {
        WorkVcsError::PlanInvalid(format!("{column} must be 16 bytes, found {}", bytes.len()))
    })
}

fn plan_invalid_from(error: WorkVcsError) -> WorkVcsError {
    WorkVcsError::PlanInvalid(error.to_string())
}

fn relation_invalid_from(error: WorkVcsError) -> WorkVcsError {
    WorkVcsError::RelationInvalid(error.to_string())
}
