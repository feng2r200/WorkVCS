use super::entity::{
    ENTITY_TRANSITION_OPERATION_SCHEMA_VERSION, ENTITY_TRANSITION_OPERATION_TYPE,
    canonical_json_string, entity_transition_payload_value,
};
use super::evidence::{EVIDENCE_OBJECT_KIND, require_evidence_exists};
use super::goal::GOAL_ENTITY_KIND;
use super::knowledge::KNOWLEDGE_ENTITY_KIND;
use super::plan::PLAN_ENTITY_KIND;
use super::record::RECORD_ENTITY_KIND;
use super::resource::{resource, resource_observation};
use super::{EntityTransitionOptions, branch_head, commit_entity_transition, state_at};
use crate::canonical::{
    CanonicalValue, ImportDigestDomain, WorkState, entity_version_digest, parse_canonical_json,
    relation_version_digest, validate_import_fixed_point, work_state_mapping_digest,
};
use crate::error::{Result, WorkVcsError, storage_error};
use crate::identity::{
    BranchId, ChangeSetId, CommitId, Digest, EntityId, EntityVersionId, EventId, EvidenceId,
    OperationId, RelationId, RelationVersionId, ResourceId, ResourceObservationId, SessionId,
    WorkspaceId,
};
use crate::store::{StoreConnection, current_epoch_micros};
use rusqlite::{OptionalExtension, Transaction, TransactionBehavior, params};
use std::collections::{BTreeMap, HashSet};
use std::fmt;

pub(crate) const ACCEPTANCE_CRITERION_ENTITY_KIND: &str = "acceptance_criterion";
pub(crate) const TASK_ENTITY_KIND: &str = "task";
pub(crate) const VERIFICATION_ENTITY_KIND: &str = "verification";
pub(crate) const VERIFICATION_REQUIREMENT_ENTITY_KIND: &str = "verification_requirement";

const ACTIVE_BRANCH_LIFECYCLE_STATE: &str = "active";
const EMPTY_FIELD_DELTA: &str = "{}";
const ENTITY_OBJECT_KIND: &str = "entity";
const ENTITY_TRANSITION_EVENT_KIND: &str = "entity.transitioned";
const NORMAL_COMMIT_KIND: &str = "normal";
const PRIMARY_PARENT_ROLE: &str = "primary";
const RELATION_OBJECT_KIND: &str = "relation";
const RELATION_STATE_SCHEMA_VERSION: i64 = 1;
const ACCEPTANCE_CRITERION_STATE_SCHEMA_VERSION: i64 = 1;
const TASK_STATE_SCHEMA_VERSION: i64 = 1;
const TASK_SCHEDULING_RELATION_CREATE_EVENT_KIND: &str = "task.scheduling_relation.created";
const TASK_SCHEDULING_RELATION_CREATE_OPERATION_SCHEMA_VERSION: i64 = 1;
const TASK_SCHEDULING_RELATION_CREATE_OPERATION_TYPE: &str = "task.scheduling_relation.create";
const VERIFICATION_BASIS_SCHEMA_VERSION: i64 = 1;
const VERIFICATION_RECORD_EVENT_KIND: &str = "verification.recorded";
const VERIFICATION_RECORD_OPERATION_SCHEMA_VERSION: i64 = 1;
const VERIFICATION_RECORD_OPERATION_TYPE: &str = "verification.record";
const VERIFICATION_REQUIREMENT_STATE_SCHEMA_VERSION: i64 = 1;
const VERIFICATION_STATE_SCHEMA_VERSION: i64 = 1;
const DEPENDS_ON_RELATION_TYPE: &str = "depends_on";
const EVIDENCED_BY_RELATION_TYPE: &str = "evidenced_by";
const ORDERED_BEFORE_RELATION_TYPE: &str = "ordered_before";
const VERIFIES_RELATION_TYPE: &str = "verifies";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TaskStatus {
    Pending,
    InProgress,
    Blocked,
    Done,
    Failed,
    Cancelled,
    Superseded,
}

impl TaskStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::InProgress => "in_progress",
            Self::Blocked => "blocked",
            Self::Done => "done",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
            Self::Superseded => "superseded",
        }
    }

    fn parse(value: &str) -> Result<Self> {
        match value {
            "pending" => Ok(Self::Pending),
            "in_progress" => Ok(Self::InProgress),
            "blocked" => Ok(Self::Blocked),
            "done" => Ok(Self::Done),
            "failed" => Ok(Self::Failed),
            "cancelled" => Ok(Self::Cancelled),
            "superseded" => Ok(Self::Superseded),
            other => Err(WorkVcsError::TaskInvalid(format!(
                "task status {other:?} is not in the confirmed lifecycle vocabulary"
            ))),
        }
    }

    fn is_non_terminal(self) -> bool {
        matches!(self, Self::Pending | Self::InProgress | Self::Blocked)
    }

    pub fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::Done | Self::Failed | Self::Cancelled | Self::Superseded
        )
    }

    fn allows_rationale_reentry(self) -> bool {
        matches!(self, Self::Done | Self::Failed | Self::Cancelled)
    }
}

impl fmt::Display for TaskStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TaskState {
    pub description: String,
    pub status: TaskStatus,
    pub outcome: Option<String>,
    pub priority: i64,
    pub acceptance_criteria: Vec<TaskAcceptanceCriterionRef>,
}

impl TaskState {
    pub fn pending(description: impl Into<String>, priority: i64) -> Result<Self> {
        let description = description.into();
        validate_description(&description)?;
        validate_priority(priority)?;
        Ok(Self {
            description,
            status: TaskStatus::Pending,
            outcome: None,
            priority,
            acceptance_criteria: Vec::new(),
        })
    }

    pub fn to_canonical_value(&self) -> Result<CanonicalValue> {
        validate_description(&self.description)?;
        validate_priority(self.priority)?;
        validate_acceptance_criterion_refs(&self.acceptance_criteria)?;
        if let Some(outcome) = &self.outcome {
            validate_outcome(outcome)?;
        }
        let outcome = self
            .outcome
            .as_ref()
            .map(|value| CanonicalValue::String(value.clone()))
            .unwrap_or(CanonicalValue::Null);
        let mut acceptance_criteria = self.acceptance_criteria.clone();
        acceptance_criteria.sort_by(|left, right| left.local_key.cmp(&right.local_key));
        let acceptance_criteria = acceptance_criteria
            .iter()
            .map(TaskAcceptanceCriterionRef::to_canonical_value)
            .collect::<Result<Vec<_>>>()?;

        CanonicalValue::object(vec![
            (
                "acceptance_criteria".to_owned(),
                CanonicalValue::Array(acceptance_criteria),
            ),
            ("child_order".to_owned(), CanonicalValue::Array(Vec::new())),
            (
                "description".to_owned(),
                CanonicalValue::String(self.description.clone()),
            ),
            ("outcome".to_owned(), outcome),
            (
                "priority".to_owned(),
                CanonicalValue::safe_integer(self.priority).map_err(task_invalid_from)?,
            ),
            (
                "status".to_owned(),
                CanonicalValue::String(self.status.as_str().to_owned()),
            ),
        ])
    }

    fn transition(
        &self,
        next_status: TaskStatus,
        outcome_update: &TaskOutcomeUpdate,
        rationale: &CanonicalValue,
    ) -> Result<Self> {
        validate_lifecycle_transition(self.status, next_status, rationale)?;
        let outcome = outcome_update.apply(self.outcome.as_ref())?;
        let next = Self {
            description: self.description.clone(),
            status: next_status,
            outcome,
            priority: self.priority,
            acceptance_criteria: self.acceptance_criteria.clone(),
        };
        if next == *self {
            return Err(WorkVcsError::TaskInvalid(
                "task transition must change status or outcome".to_owned(),
            ));
        }
        Ok(next)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TaskAcceptanceCriterionRef {
    pub local_key: String,
    pub acceptance_criterion_entity_id: EntityId,
}

impl TaskAcceptanceCriterionRef {
    pub fn new(
        local_key: impl Into<String>,
        acceptance_criterion_entity_id: EntityId,
    ) -> Result<Self> {
        let local_key = local_key.into();
        validate_local_key("acceptance criterion local key", &local_key)?;
        Ok(Self {
            local_key,
            acceptance_criterion_entity_id,
        })
    }

    fn to_canonical_value(&self) -> Result<CanonicalValue> {
        validate_local_key("acceptance criterion local key", &self.local_key)?;
        CanonicalValue::object(vec![
            (
                "entity_id".to_owned(),
                CanonicalValue::String(self.acceptance_criterion_entity_id.to_string()),
            ),
            (
                "local_key".to_owned(),
                CanonicalValue::String(self.local_key.clone()),
            ),
        ])
        .map_err(task_invalid_from)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AcceptanceCriterionVerificationRequirementRef {
    pub local_key: String,
    pub verification_requirement_entity_id: EntityId,
}

impl AcceptanceCriterionVerificationRequirementRef {
    pub fn new(
        local_key: impl Into<String>,
        verification_requirement_entity_id: EntityId,
    ) -> Result<Self> {
        let local_key = local_key.into();
        validate_local_key("verification requirement local key", &local_key)?;
        Ok(Self {
            local_key,
            verification_requirement_entity_id,
        })
    }

    fn to_canonical_value(&self) -> Result<CanonicalValue> {
        validate_local_key("verification requirement local key", &self.local_key)?;
        CanonicalValue::object(vec![
            (
                "entity_id".to_owned(),
                CanonicalValue::String(self.verification_requirement_entity_id.to_string()),
            ),
            (
                "local_key".to_owned(),
                CanonicalValue::String(self.local_key.clone()),
            ),
        ])
        .map_err(task_invalid_from)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AcceptanceCriterionClassification {
    Required,
    Optional,
}

impl AcceptanceCriterionClassification {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Required => "required",
            Self::Optional => "optional",
        }
    }

    fn parse(value: &str) -> Result<Self> {
        match value {
            "required" => Ok(Self::Required),
            "optional" => Ok(Self::Optional),
            other => Err(WorkVcsError::TaskInvalid(format!(
                "acceptance criterion classification {other:?} is not in the confirmed vocabulary"
            ))),
        }
    }
}

impl fmt::Display for AcceptanceCriterionClassification {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AcceptanceCriterionState {
    pub statement: String,
    pub classification: AcceptanceCriterionClassification,
    pub verification_requirements: Vec<AcceptanceCriterionVerificationRequirementRef>,
}

impl AcceptanceCriterionState {
    pub fn new(
        statement: impl Into<String>,
        classification: AcceptanceCriterionClassification,
    ) -> Result<Self> {
        let statement = statement.into();
        validate_acceptance_criterion_statement(&statement)?;
        Ok(Self {
            statement,
            classification,
            verification_requirements: Vec::new(),
        })
    }

    fn with_verification_requirements(
        mut self,
        verification_requirements: Vec<AcceptanceCriterionVerificationRequirementRef>,
    ) -> Result<Self> {
        validate_verification_requirement_refs(&verification_requirements)?;
        self.verification_requirements = verification_requirements;
        Ok(self)
    }

    pub fn to_canonical_value(&self) -> Result<CanonicalValue> {
        validate_acceptance_criterion_statement(&self.statement)?;
        validate_verification_requirement_refs(&self.verification_requirements)?;
        let mut verification_requirements = self.verification_requirements.clone();
        verification_requirements.sort_by(|left, right| left.local_key.cmp(&right.local_key));
        let verification_requirements = verification_requirements
            .iter()
            .map(AcceptanceCriterionVerificationRequirementRef::to_canonical_value)
            .collect::<Result<Vec<_>>>()?;
        CanonicalValue::object(vec![
            (
                "classification".to_owned(),
                CanonicalValue::String(self.classification.as_str().to_owned()),
            ),
            (
                "statement".to_owned(),
                CanonicalValue::String(self.statement.clone()),
            ),
            (
                "verification_requirements".to_owned(),
                CanonicalValue::Array(verification_requirements),
            ),
        ])
        .map_err(task_invalid_from)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerificationRequirementState {
    pub statement: String,
}

impl VerificationRequirementState {
    pub fn new(statement: impl Into<String>) -> Result<Self> {
        let statement = statement.into();
        validate_verification_requirement_statement(&statement)?;
        Ok(Self { statement })
    }

    pub fn to_canonical_value(&self) -> Result<CanonicalValue> {
        validate_verification_requirement_statement(&self.statement)?;
        CanonicalValue::object(vec![(
            "statement".to_owned(),
            CanonicalValue::String(self.statement.clone()),
        )])
        .map_err(task_invalid_from)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VerificationResult {
    Passed,
    Failed,
    Inconclusive,
}

impl VerificationResult {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Passed => "passed",
            Self::Failed => "failed",
            Self::Inconclusive => "inconclusive",
        }
    }

    fn parse(value: &str) -> Result<Self> {
        match value {
            "passed" => Ok(Self::Passed),
            "failed" => Ok(Self::Failed),
            "inconclusive" => Ok(Self::Inconclusive),
            other => Err(WorkVcsError::TaskInvalid(format!(
                "verification result {other:?} is not in the confirmed vocabulary"
            ))),
        }
    }
}

impl fmt::Display for VerificationResult {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VerificationTarget {
    AcceptanceCriterion(EntityId),
    VerificationRequirement(EntityId),
}

impl VerificationTarget {
    pub fn entity_id(self) -> EntityId {
        match self {
            Self::AcceptanceCriterion(entity_id) | Self::VerificationRequirement(entity_id) => {
                entity_id
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerificationSemanticDependency {
    pub entity_id: EntityId,
    pub entity_version_id: EntityVersionId,
}

impl VerificationSemanticDependency {
    pub fn new(entity_id: EntityId, entity_version_id: EntityVersionId) -> Self {
        Self {
            entity_id,
            entity_version_id,
        }
    }

    fn to_canonical_value(&self) -> Result<CanonicalValue> {
        CanonicalValue::object(vec![
            (
                "entity_id".to_owned(),
                CanonicalValue::String(self.entity_id.to_string()),
            ),
            (
                "entity_version_id".to_owned(),
                CanonicalValue::String(self.entity_version_id.to_string()),
            ),
        ])
        .map_err(task_invalid_from)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VerificationEvidenceRef {
    pub evidence_id: EvidenceId,
}

impl VerificationEvidenceRef {
    pub fn new(evidence_id: EvidenceId) -> Self {
        Self { evidence_id }
    }

    fn to_canonical_value(self) -> Result<CanonicalValue> {
        CanonicalValue::object(vec![(
            "evidence_id".to_owned(),
            CanonicalValue::String(self.evidence_id.to_string()),
        )])
        .map_err(task_invalid_from)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerificationResourceBasis {
    pub resource_id: ResourceId,
    pub adapter_kind: String,
    pub adapter_schema_version: i64,
    pub scope_kind: String,
    pub scope_schema_version: i64,
    pub scope_payload: CanonicalValue,
    pub baseline_observation_id: Option<ResourceObservationId>,
    pub baseline_fingerprint: Digest,
}

impl VerificationResourceBasis {
    pub fn new(
        resource_id: ResourceId,
        adapter_kind: impl Into<String>,
        adapter_schema_version: i64,
        scope_kind: impl Into<String>,
        scope_schema_version: i64,
        scope_payload: CanonicalValue,
        baseline_fingerprint: Digest,
    ) -> Result<Self> {
        let scope_payload = normalize_verification_resource_scope_payload(scope_payload)?;
        let basis = Self {
            resource_id,
            adapter_kind: adapter_kind.into(),
            adapter_schema_version,
            scope_kind: scope_kind.into(),
            scope_schema_version,
            scope_payload,
            baseline_observation_id: None,
            baseline_fingerprint,
        };
        validate_verification_resource_basis_entry(&basis)?;
        Ok(basis)
    }

    pub fn with_baseline_observation_id(
        mut self,
        baseline_observation_id: ResourceObservationId,
    ) -> Result<Self> {
        self.baseline_observation_id = Some(baseline_observation_id);
        validate_verification_resource_basis_entry(&self)?;
        Ok(self)
    }

    fn to_canonical_value(&self) -> Result<CanonicalValue> {
        validate_verification_resource_basis_entry(self)?;
        let baseline_observation_id = self
            .baseline_observation_id
            .map(|id| CanonicalValue::String(id.to_string()))
            .unwrap_or(CanonicalValue::Null);
        CanonicalValue::object(vec![
            (
                "adapter_kind".to_owned(),
                CanonicalValue::String(self.adapter_kind.clone()),
            ),
            (
                "adapter_schema_version".to_owned(),
                CanonicalValue::safe_integer(self.adapter_schema_version)
                    .map_err(task_invalid_from)?,
            ),
            (
                "baseline_fingerprint".to_owned(),
                CanonicalValue::String(self.baseline_fingerprint.to_hex()),
            ),
            (
                "baseline_observation_id".to_owned(),
                baseline_observation_id,
            ),
            (
                "resource_id".to_owned(),
                CanonicalValue::String(self.resource_id.to_string()),
            ),
            (
                "scope_kind".to_owned(),
                CanonicalValue::String(self.scope_kind.clone()),
            ),
            ("scope_payload".to_owned(), self.scope_payload.clone()),
            (
                "scope_schema_version".to_owned(),
                CanonicalValue::safe_integer(self.scope_schema_version)
                    .map_err(task_invalid_from)?,
            ),
        ])
        .map_err(task_invalid_from)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerificationState {
    pub result: VerificationResult,
    pub method: CanonicalValue,
    pub verified_at_commit_id: CommitId,
    pub semantic_dependencies: Vec<VerificationSemanticDependency>,
    pub evidence: Vec<VerificationEvidenceRef>,
    pub resource_basis: Vec<VerificationResourceBasis>,
}

impl VerificationState {
    pub fn new(
        result: VerificationResult,
        method: CanonicalValue,
        verified_at_commit_id: CommitId,
        semantic_dependencies: Vec<VerificationSemanticDependency>,
    ) -> Result<Self> {
        Self::new_with_evidence(
            result,
            method,
            verified_at_commit_id,
            semantic_dependencies,
            Vec::new(),
        )
    }

    pub fn new_with_evidence(
        result: VerificationResult,
        method: CanonicalValue,
        verified_at_commit_id: CommitId,
        semantic_dependencies: Vec<VerificationSemanticDependency>,
        evidence: Vec<VerificationEvidenceRef>,
    ) -> Result<Self> {
        Self::new_with_evidence_and_resource_basis(
            result,
            method,
            verified_at_commit_id,
            semantic_dependencies,
            evidence,
            Vec::new(),
        )
    }

    pub fn new_with_evidence_and_resource_basis(
        result: VerificationResult,
        method: CanonicalValue,
        verified_at_commit_id: CommitId,
        semantic_dependencies: Vec<VerificationSemanticDependency>,
        evidence: Vec<VerificationEvidenceRef>,
        resource_basis: Vec<VerificationResourceBasis>,
    ) -> Result<Self> {
        validate_verification_method(&method)?;
        validate_verification_semantic_dependencies(&semantic_dependencies)?;
        validate_verification_evidence_refs(&evidence)?;
        validate_verification_resource_basis(&resource_basis)?;
        let mut evidence = evidence;
        evidence.sort_by_key(|evidence| evidence.evidence_id.raw_bytes());
        Ok(Self {
            result,
            method,
            verified_at_commit_id,
            semantic_dependencies,
            evidence,
            resource_basis,
        })
    }

    pub fn to_canonical_value(&self) -> Result<CanonicalValue> {
        validate_verification_method(&self.method)?;
        validate_verification_semantic_dependencies(&self.semantic_dependencies)?;
        validate_verification_evidence_refs(&self.evidence)?;
        validate_verification_resource_basis(&self.resource_basis)?;
        let mut dependencies = self.semantic_dependencies.clone();
        dependencies.sort_by(|left, right| {
            left.entity_id
                .raw_bytes()
                .cmp(&right.entity_id.raw_bytes())
                .then_with(|| {
                    left.entity_version_id
                        .raw_bytes()
                        .cmp(&right.entity_version_id.raw_bytes())
                })
        });
        let dependencies = dependencies
            .iter()
            .map(VerificationSemanticDependency::to_canonical_value)
            .collect::<Result<Vec<_>>>()?;
        let mut evidence = self.evidence.clone();
        evidence.sort_by_key(|evidence| evidence.evidence_id.raw_bytes());
        let evidence = evidence
            .iter()
            .copied()
            .map(VerificationEvidenceRef::to_canonical_value)
            .collect::<Result<Vec<_>>>()?;
        let resource_basis = self
            .resource_basis
            .iter()
            .map(VerificationResourceBasis::to_canonical_value)
            .collect::<Result<Vec<_>>>()?;
        CanonicalValue::object(vec![
            (
                "basis".to_owned(),
                CanonicalValue::object(vec![
                    (
                        "resource_basis".to_owned(),
                        CanonicalValue::Array(resource_basis),
                    ),
                    (
                        "semantic_dependencies".to_owned(),
                        CanonicalValue::Array(dependencies),
                    ),
                    (
                        "verified_at_commit_id".to_owned(),
                        CanonicalValue::String(self.verified_at_commit_id.to_string()),
                    ),
                ])?,
            ),
            ("evidence".to_owned(), CanonicalValue::Array(evidence)),
            ("method".to_owned(), self.method.clone()),
            (
                "result".to_owned(),
                CanonicalValue::String(self.result.as_str().to_owned()),
            ),
        ])
        .map_err(task_invalid_from)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AcceptanceCriterionEffectiveStatus {
    Unverified,
    Verified,
    Failed,
    Stale,
    Conflicted,
}

impl AcceptanceCriterionEffectiveStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Unverified => "unverified",
            Self::Verified => "verified",
            Self::Failed => "failed",
            Self::Stale => "stale",
            Self::Conflicted => "conflicted",
        }
    }
}

impl fmt::Display for AcceptanceCriterionEffectiveStatus {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VerificationApplicability {
    Applicable,
    Stale,
    Unknown,
}

impl VerificationApplicability {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Applicable => "applicable",
            Self::Stale => "stale",
            Self::Unknown => "unknown",
        }
    }

    fn parse(value: &str) -> Result<Self> {
        match value {
            "applicable" => Ok(Self::Applicable),
            "stale" => Ok(Self::Stale),
            "unknown" => Ok(Self::Unknown),
            other => Err(WorkVcsError::TaskInvalid(format!(
                "verification applicability {other:?} is not in the confirmed vocabulary"
            ))),
        }
    }
}

impl fmt::Display for VerificationApplicability {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ApplicabilityResourceObservationStatus {
    Observed,
    Unavailable,
    Error,
}

impl ApplicabilityResourceObservationStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Observed => "observed",
            Self::Unavailable => "unavailable",
            Self::Error => "error",
        }
    }

    fn parse(value: &str) -> Result<Self> {
        match value {
            "observed" => Ok(Self::Observed),
            "unavailable" => Ok(Self::Unavailable),
            "error" => Ok(Self::Error),
            other => Err(WorkVcsError::TaskInvalid(format!(
                "applicability resource observation status {other:?} is not in the confirmed vocabulary"
            ))),
        }
    }
}

impl fmt::Display for ApplicabilityResourceObservationStatus {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ApplicabilityResourceStampInput {
    pub resource_basis_ordinal: i64,
    pub adapter_kind: String,
    pub adapter_schema_version: i64,
    pub scope_schema_version: i64,
    pub observation_status: ApplicabilityResourceObservationStatus,
    pub observed_fingerprint: Option<Digest>,
    pub observation_id: Option<ResourceObservationId>,
}

impl ApplicabilityResourceStampInput {
    pub fn observed(
        resource_basis_ordinal: i64,
        adapter_kind: impl Into<String>,
        adapter_schema_version: i64,
        scope_schema_version: i64,
        observed_fingerprint: Digest,
    ) -> Result<Self> {
        let stamp = Self {
            resource_basis_ordinal,
            adapter_kind: adapter_kind.into(),
            adapter_schema_version,
            scope_schema_version,
            observation_status: ApplicabilityResourceObservationStatus::Observed,
            observed_fingerprint: Some(observed_fingerprint),
            observation_id: None,
        };
        validate_applicability_resource_stamp_input(&stamp)?;
        Ok(stamp)
    }

    pub fn unavailable(
        resource_basis_ordinal: i64,
        adapter_kind: impl Into<String>,
        adapter_schema_version: i64,
        scope_schema_version: i64,
    ) -> Result<Self> {
        Self::unobserved(
            resource_basis_ordinal,
            adapter_kind,
            adapter_schema_version,
            scope_schema_version,
            ApplicabilityResourceObservationStatus::Unavailable,
        )
    }

    pub fn error(
        resource_basis_ordinal: i64,
        adapter_kind: impl Into<String>,
        adapter_schema_version: i64,
        scope_schema_version: i64,
    ) -> Result<Self> {
        Self::unobserved(
            resource_basis_ordinal,
            adapter_kind,
            adapter_schema_version,
            scope_schema_version,
            ApplicabilityResourceObservationStatus::Error,
        )
    }

    pub fn with_observation_id(mut self, observation_id: ResourceObservationId) -> Result<Self> {
        self.observation_id = Some(observation_id);
        validate_applicability_resource_stamp_input(&self)?;
        Ok(self)
    }

    fn unobserved(
        resource_basis_ordinal: i64,
        adapter_kind: impl Into<String>,
        adapter_schema_version: i64,
        scope_schema_version: i64,
        observation_status: ApplicabilityResourceObservationStatus,
    ) -> Result<Self> {
        let stamp = Self {
            resource_basis_ordinal,
            adapter_kind: adapter_kind.into(),
            adapter_schema_version,
            scope_schema_version,
            observation_status,
            observed_fingerprint: None,
            observation_id: None,
        };
        validate_applicability_resource_stamp_input(&stamp)?;
        Ok(stamp)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ApplicabilityResourceStampSnapshot {
    pub resource_basis_ordinal: i64,
    pub adapter_kind: String,
    pub adapter_schema_version: i64,
    pub scope_schema_version: i64,
    pub observation_status: ApplicabilityResourceObservationStatus,
    pub observed_fingerprint: Option<Digest>,
    pub observation_id: Option<ResourceObservationId>,
    pub observed_at_us: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerificationApplicabilityRecordOptions {
    branch_id: BranchId,
    verification_entity_id: EntityId,
    evaluated_commit_id: CommitId,
    resource_stamps: Vec<ApplicabilityResourceStampInput>,
    detail: CanonicalValue,
}

impl VerificationApplicabilityRecordOptions {
    pub fn new(
        branch_id: BranchId,
        verification_entity_id: EntityId,
        evaluated_commit_id: CommitId,
    ) -> Result<Self> {
        Ok(Self {
            branch_id,
            verification_entity_id,
            evaluated_commit_id,
            resource_stamps: Vec::new(),
            detail: CanonicalValue::object(Vec::new())?,
        })
    }

    pub fn with_resource_stamps<I>(mut self, resource_stamps: I) -> Result<Self>
    where
        I: IntoIterator<Item = ApplicabilityResourceStampInput>,
    {
        self.resource_stamps = resource_stamps.into_iter().collect();
        self.resource_stamps
            .sort_by_key(|stamp| stamp.resource_basis_ordinal);
        validate_applicability_resource_stamp_inputs(&self.resource_stamps)?;
        Ok(self)
    }

    pub fn with_detail(mut self, detail: CanonicalValue) -> Result<Self> {
        require_object("verification applicability detail", &detail)?;
        self.detail = detail;
        Ok(self)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerificationApplicabilityCacheSnapshot {
    pub branch_id: BranchId,
    pub verification_entity_id: EntityId,
    pub evaluated_commit_id: CommitId,
    pub applicability: VerificationApplicability,
    pub reason_code: String,
    pub detail: CanonicalValue,
    pub evaluated_at_us: i64,
    pub resource_stamps: Vec<ApplicabilityResourceStampSnapshot>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerificationApplicabilityCacheListOptions {
    branch_id: BranchId,
    verification_entity_id: Option<EntityId>,
    applicability: Option<VerificationApplicability>,
}

impl VerificationApplicabilityCacheListOptions {
    pub fn new(branch_id: BranchId) -> Self {
        Self {
            branch_id,
            verification_entity_id: None,
            applicability: None,
        }
    }

    pub fn with_verification_entity_id(mut self, verification_entity_id: EntityId) -> Self {
        self.verification_entity_id = Some(verification_entity_id);
        self
    }

    pub fn with_applicability(mut self, applicability: VerificationApplicability) -> Self {
        self.applicability = Some(applicability);
        self
    }

    fn branch_id(&self) -> BranchId {
        self.branch_id
    }

    fn verification_entity_id(&self) -> Option<EntityId> {
        self.verification_entity_id
    }

    fn applicability(&self) -> Option<VerificationApplicability> {
        self.applicability
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerificationApplicabilityCacheListResult {
    pub branch_id: BranchId,
    pub caches: Vec<VerificationApplicabilityCacheSnapshot>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TaskCreateOptions {
    branch_id: BranchId,
    expected_head_commit_id: CommitId,
    state: TaskState,
    rationale: CanonicalValue,
}

impl TaskCreateOptions {
    pub fn new(
        branch_id: BranchId,
        expected_head_commit_id: CommitId,
        description: impl Into<String>,
    ) -> Result<Self> {
        Ok(Self {
            branch_id,
            expected_head_commit_id,
            state: TaskState::pending(description, 0)?,
            rationale: CanonicalValue::object(Vec::new())?,
        })
    }

    pub fn with_priority(mut self, priority: i64) -> Result<Self> {
        validate_priority(priority)?;
        self.state.priority = priority;
        Ok(self)
    }

    pub fn with_rationale(mut self, rationale: CanonicalValue) -> Self {
        self.rationale = rationale;
        self
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TaskCreateCommit {
    pub workspace_id: WorkspaceId,
    pub branch_id: BranchId,
    pub previous_head_commit_id: CommitId,
    pub commit_id: CommitId,
    pub changeset_id: ChangeSetId,
    pub operation_id: OperationId,
    pub task_entity_id: EntityId,
    pub task_entity_version_id: EntityVersionId,
    pub task_state_digest: Digest,
    pub work_state_digest: Digest,
    pub state: TaskState,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TaskTransitionOptions {
    branch_id: BranchId,
    expected_head_commit_id: CommitId,
    task_entity_id: EntityId,
    expected_task_entity_version_id: EntityVersionId,
    next_status: TaskStatus,
    outcome_update: TaskOutcomeUpdate,
    rationale: CanonicalValue,
    actor_session_id: Option<SessionId>,
}

impl TaskTransitionOptions {
    pub fn new(
        branch_id: BranchId,
        expected_head_commit_id: CommitId,
        task_entity_id: EntityId,
        expected_task_entity_version_id: EntityVersionId,
        next_status: TaskStatus,
    ) -> Result<Self> {
        if next_status == TaskStatus::Superseded {
            return Err(WorkVcsError::TaskInvalid(
                "ordinary task transition to superseded requires supersession-aware resolution"
                    .to_owned(),
            ));
        }
        Ok(Self {
            branch_id,
            expected_head_commit_id,
            task_entity_id,
            expected_task_entity_version_id,
            next_status,
            outcome_update: TaskOutcomeUpdate::Preserve,
            rationale: CanonicalValue::object(Vec::new())?,
            actor_session_id: None,
        })
    }

    pub fn with_outcome(mut self, outcome: impl Into<String>) -> Result<Self> {
        let outcome = outcome.into();
        validate_outcome(&outcome)?;
        self.outcome_update = TaskOutcomeUpdate::Set(outcome);
        Ok(self)
    }

    pub fn clear_outcome(mut self) -> Self {
        self.outcome_update = TaskOutcomeUpdate::Clear;
        self
    }

    pub fn with_rationale(mut self, rationale: CanonicalValue) -> Self {
        self.rationale = rationale;
        self
    }

    pub fn with_actor_session(mut self, actor_session_id: SessionId) -> Self {
        self.actor_session_id = Some(actor_session_id);
        self
    }

    pub fn actor_session_id(&self) -> Option<SessionId> {
        self.actor_session_id
    }

    pub fn branch_id(&self) -> BranchId {
        self.branch_id
    }

    pub fn expected_head_commit_id(&self) -> CommitId {
        self.expected_head_commit_id
    }

    pub fn task_entity_id(&self) -> EntityId {
        self.task_entity_id
    }

    pub fn next_status(&self) -> TaskStatus {
        self.next_status
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum TaskOutcomeUpdate {
    Preserve,
    Set(String),
    Clear,
}

impl TaskOutcomeUpdate {
    fn apply(&self, current: Option<&String>) -> Result<Option<String>> {
        match self {
            Self::Preserve => Ok(current.cloned()),
            Self::Set(value) => {
                validate_outcome(value)?;
                Ok(Some(value.clone()))
            }
            Self::Clear => Ok(None),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TaskTransitionCommit {
    pub workspace_id: WorkspaceId,
    pub branch_id: BranchId,
    pub previous_head_commit_id: CommitId,
    pub commit_id: CommitId,
    pub changeset_id: ChangeSetId,
    pub operation_id: OperationId,
    pub task_entity_id: EntityId,
    pub previous_task_entity_version_id: EntityVersionId,
    pub task_entity_version_id: EntityVersionId,
    pub task_state_digest: Digest,
    pub work_state_digest: Digest,
    pub previous_state: TaskState,
    pub state: TaskState,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum TaskSchedulingRelationType {
    DependsOn,
    OrderedBefore,
}

impl TaskSchedulingRelationType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::DependsOn => DEPENDS_ON_RELATION_TYPE,
            Self::OrderedBefore => ORDERED_BEFORE_RELATION_TYPE,
        }
    }

    fn parse(value: &str) -> Option<Self> {
        match value {
            DEPENDS_ON_RELATION_TYPE => Some(Self::DependsOn),
            ORDERED_BEFORE_RELATION_TYPE => Some(Self::OrderedBefore),
            _ => None,
        }
    }
}

impl fmt::Display for TaskSchedulingRelationType {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TaskSchedulingRelationCreateOptions {
    branch_id: BranchId,
    expected_head_commit_id: CommitId,
    relation_type: TaskSchedulingRelationType,
    source_task_entity_id: EntityId,
    target_task_entity_id: EntityId,
    rationale: CanonicalValue,
    actor_session_id: Option<SessionId>,
}

impl TaskSchedulingRelationCreateOptions {
    pub fn new(
        branch_id: BranchId,
        expected_head_commit_id: CommitId,
        relation_type: TaskSchedulingRelationType,
        source_task_entity_id: EntityId,
        target_task_entity_id: EntityId,
    ) -> Result<Self> {
        Ok(Self {
            branch_id,
            expected_head_commit_id,
            relation_type,
            source_task_entity_id,
            target_task_entity_id,
            rationale: CanonicalValue::object(Vec::new())?,
            actor_session_id: None,
        })
    }

    pub fn depends_on(
        branch_id: BranchId,
        expected_head_commit_id: CommitId,
        dependent_task_entity_id: EntityId,
        prerequisite_task_entity_id: EntityId,
    ) -> Result<Self> {
        Self::new(
            branch_id,
            expected_head_commit_id,
            TaskSchedulingRelationType::DependsOn,
            dependent_task_entity_id,
            prerequisite_task_entity_id,
        )
    }

    pub fn ordered_before(
        branch_id: BranchId,
        expected_head_commit_id: CommitId,
        earlier_task_entity_id: EntityId,
        later_task_entity_id: EntityId,
    ) -> Result<Self> {
        Self::new(
            branch_id,
            expected_head_commit_id,
            TaskSchedulingRelationType::OrderedBefore,
            earlier_task_entity_id,
            later_task_entity_id,
        )
    }

    pub fn with_rationale(mut self, rationale: CanonicalValue) -> Self {
        self.rationale = rationale;
        self
    }

    pub fn with_actor_session(mut self, actor_session_id: SessionId) -> Self {
        self.actor_session_id = Some(actor_session_id);
        self
    }

    pub fn actor_session_id(&self) -> Option<SessionId> {
        self.actor_session_id
    }

    pub fn branch_id(&self) -> BranchId {
        self.branch_id
    }

    pub fn expected_head_commit_id(&self) -> CommitId {
        self.expected_head_commit_id
    }

    pub fn source_task_entity_id(&self) -> EntityId {
        self.source_task_entity_id
    }

    pub fn target_task_entity_id(&self) -> EntityId {
        self.target_task_entity_id
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TaskSchedulingRelationCreateCommit {
    pub workspace_id: WorkspaceId,
    pub branch_id: BranchId,
    pub previous_head_commit_id: CommitId,
    pub commit_id: CommitId,
    pub changeset_id: ChangeSetId,
    pub operation_id: OperationId,
    pub relation_id: RelationId,
    pub relation_version_id: RelationVersionId,
    pub relation_type: TaskSchedulingRelationType,
    pub source_task_entity_id: EntityId,
    pub target_task_entity_id: EntityId,
    pub relation_state_digest: Digest,
    pub work_state_digest: Digest,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AcceptanceCriterionCreateOptions {
    branch_id: BranchId,
    expected_head_commit_id: CommitId,
    task_entity_id: EntityId,
    expected_task_entity_version_id: EntityVersionId,
    local_key: String,
    state: AcceptanceCriterionState,
    rationale: CanonicalValue,
}

impl AcceptanceCriterionCreateOptions {
    pub fn new(
        branch_id: BranchId,
        expected_head_commit_id: CommitId,
        task_entity_id: EntityId,
        expected_task_entity_version_id: EntityVersionId,
        local_key: impl Into<String>,
        statement: impl Into<String>,
        classification: AcceptanceCriterionClassification,
    ) -> Result<Self> {
        let local_key = local_key.into();
        validate_local_key("acceptance criterion local key", &local_key)?;
        Ok(Self {
            branch_id,
            expected_head_commit_id,
            task_entity_id,
            expected_task_entity_version_id,
            local_key,
            state: AcceptanceCriterionState::new(statement, classification)?,
            rationale: CanonicalValue::object(Vec::new())?,
        })
    }

    pub fn with_rationale(mut self, rationale: CanonicalValue) -> Self {
        self.rationale = rationale;
        self
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AcceptanceCriterionCreateCommit {
    pub workspace_id: WorkspaceId,
    pub branch_id: BranchId,
    pub previous_head_commit_id: CommitId,
    pub commit_id: CommitId,
    pub changeset_id: ChangeSetId,
    pub acceptance_criterion_operation_id: OperationId,
    pub task_operation_id: OperationId,
    pub task_entity_id: EntityId,
    pub previous_task_entity_version_id: EntityVersionId,
    pub task_entity_version_id: EntityVersionId,
    pub task_state_digest: Digest,
    pub acceptance_criterion_entity_id: EntityId,
    pub acceptance_criterion_entity_version_id: EntityVersionId,
    pub acceptance_criterion_state_digest: Digest,
    pub work_state_digest: Digest,
    pub local_key: String,
    pub state: AcceptanceCriterionState,
    pub task_state: TaskState,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AcceptanceCriterionRevisionOptions {
    branch_id: BranchId,
    expected_head_commit_id: CommitId,
    acceptance_criterion_entity_id: EntityId,
    expected_acceptance_criterion_entity_version_id: EntityVersionId,
    state: AcceptanceCriterionState,
    rationale: CanonicalValue,
}

impl AcceptanceCriterionRevisionOptions {
    pub fn new(
        branch_id: BranchId,
        expected_head_commit_id: CommitId,
        acceptance_criterion_entity_id: EntityId,
        expected_acceptance_criterion_entity_version_id: EntityVersionId,
        statement: impl Into<String>,
        classification: AcceptanceCriterionClassification,
    ) -> Result<Self> {
        Ok(Self {
            branch_id,
            expected_head_commit_id,
            acceptance_criterion_entity_id,
            expected_acceptance_criterion_entity_version_id,
            state: AcceptanceCriterionState::new(statement, classification)?,
            rationale: CanonicalValue::object(Vec::new())?,
        })
    }

    pub fn with_rationale(mut self, rationale: CanonicalValue) -> Self {
        self.rationale = rationale;
        self
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AcceptanceCriterionRevisionCommit {
    pub workspace_id: WorkspaceId,
    pub branch_id: BranchId,
    pub previous_head_commit_id: CommitId,
    pub commit_id: CommitId,
    pub changeset_id: ChangeSetId,
    pub operation_id: OperationId,
    pub task_entity_id: EntityId,
    pub acceptance_criterion_entity_id: EntityId,
    pub previous_acceptance_criterion_entity_version_id: EntityVersionId,
    pub acceptance_criterion_entity_version_id: EntityVersionId,
    pub acceptance_criterion_state_digest: Digest,
    pub work_state_digest: Digest,
    pub local_key: String,
    pub previous_state: AcceptanceCriterionState,
    pub state: AcceptanceCriterionState,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TaskSnapshot {
    pub workspace_id: WorkspaceId,
    pub commit_id: CommitId,
    pub task_entity_id: EntityId,
    pub task_entity_version_id: EntityVersionId,
    pub state_digest: Digest,
    pub state: TaskState,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TaskSchedulingRelationSnapshot {
    pub workspace_id: WorkspaceId,
    pub commit_id: CommitId,
    pub relation_id: RelationId,
    pub relation_version_id: RelationVersionId,
    pub relation_type: TaskSchedulingRelationType,
    pub source_task_entity_id: EntityId,
    pub target_task_entity_id: EntityId,
    pub state_digest: Digest,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AcceptanceCriterionSnapshot {
    pub workspace_id: WorkspaceId,
    pub commit_id: CommitId,
    pub task_entity_id: EntityId,
    pub local_key: String,
    pub acceptance_criterion_entity_id: EntityId,
    pub acceptance_criterion_entity_version_id: EntityVersionId,
    pub state_digest: Digest,
    pub state: AcceptanceCriterionState,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerificationRequirementCreateOptions {
    branch_id: BranchId,
    expected_head_commit_id: CommitId,
    acceptance_criterion_entity_id: EntityId,
    expected_acceptance_criterion_entity_version_id: EntityVersionId,
    local_key: String,
    state: VerificationRequirementState,
    rationale: CanonicalValue,
}

impl VerificationRequirementCreateOptions {
    pub fn new(
        branch_id: BranchId,
        expected_head_commit_id: CommitId,
        acceptance_criterion_entity_id: EntityId,
        expected_acceptance_criterion_entity_version_id: EntityVersionId,
        local_key: impl Into<String>,
        statement: impl Into<String>,
    ) -> Result<Self> {
        let local_key = local_key.into();
        validate_local_key("verification requirement local key", &local_key)?;
        Ok(Self {
            branch_id,
            expected_head_commit_id,
            acceptance_criterion_entity_id,
            expected_acceptance_criterion_entity_version_id,
            local_key,
            state: VerificationRequirementState::new(statement)?,
            rationale: CanonicalValue::object(Vec::new())?,
        })
    }

    pub fn with_rationale(mut self, rationale: CanonicalValue) -> Self {
        self.rationale = rationale;
        self
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerificationRequirementCreateCommit {
    pub workspace_id: WorkspaceId,
    pub branch_id: BranchId,
    pub previous_head_commit_id: CommitId,
    pub commit_id: CommitId,
    pub changeset_id: ChangeSetId,
    pub verification_requirement_operation_id: OperationId,
    pub acceptance_criterion_operation_id: OperationId,
    pub acceptance_criterion_entity_id: EntityId,
    pub previous_acceptance_criterion_entity_version_id: EntityVersionId,
    pub acceptance_criterion_entity_version_id: EntityVersionId,
    pub acceptance_criterion_state_digest: Digest,
    pub verification_requirement_entity_id: EntityId,
    pub verification_requirement_entity_version_id: EntityVersionId,
    pub verification_requirement_state_digest: Digest,
    pub work_state_digest: Digest,
    pub local_key: String,
    pub state: VerificationRequirementState,
    pub acceptance_criterion_state: AcceptanceCriterionState,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerificationRequirementRevisionOptions {
    branch_id: BranchId,
    expected_head_commit_id: CommitId,
    verification_requirement_entity_id: EntityId,
    expected_verification_requirement_entity_version_id: EntityVersionId,
    state: VerificationRequirementState,
    rationale: CanonicalValue,
}

impl VerificationRequirementRevisionOptions {
    pub fn new(
        branch_id: BranchId,
        expected_head_commit_id: CommitId,
        verification_requirement_entity_id: EntityId,
        expected_verification_requirement_entity_version_id: EntityVersionId,
        statement: impl Into<String>,
    ) -> Result<Self> {
        Ok(Self {
            branch_id,
            expected_head_commit_id,
            verification_requirement_entity_id,
            expected_verification_requirement_entity_version_id,
            state: VerificationRequirementState::new(statement)?,
            rationale: CanonicalValue::object(Vec::new())?,
        })
    }

    pub fn with_rationale(mut self, rationale: CanonicalValue) -> Self {
        self.rationale = rationale;
        self
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerificationRequirementRevisionCommit {
    pub workspace_id: WorkspaceId,
    pub branch_id: BranchId,
    pub previous_head_commit_id: CommitId,
    pub commit_id: CommitId,
    pub changeset_id: ChangeSetId,
    pub operation_id: OperationId,
    pub acceptance_criterion_entity_id: EntityId,
    pub local_key: String,
    pub verification_requirement_entity_id: EntityId,
    pub previous_verification_requirement_entity_version_id: EntityVersionId,
    pub verification_requirement_entity_version_id: EntityVersionId,
    pub verification_requirement_state_digest: Digest,
    pub work_state_digest: Digest,
    pub previous_state: VerificationRequirementState,
    pub state: VerificationRequirementState,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerificationRequirementSnapshot {
    pub workspace_id: WorkspaceId,
    pub commit_id: CommitId,
    pub acceptance_criterion_entity_id: EntityId,
    pub local_key: String,
    pub verification_requirement_entity_id: EntityId,
    pub verification_requirement_entity_version_id: EntityVersionId,
    pub state_digest: Digest,
    pub state: VerificationRequirementState,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerificationCreateOptions {
    branch_id: BranchId,
    expected_head_commit_id: CommitId,
    target: VerificationTarget,
    result: VerificationResult,
    method: CanonicalValue,
    evidence: Vec<VerificationEvidenceRef>,
    resource_basis: Vec<VerificationResourceBasis>,
    rationale: CanonicalValue,
}

impl VerificationCreateOptions {
    pub fn new(
        branch_id: BranchId,
        expected_head_commit_id: CommitId,
        target: VerificationTarget,
        result: VerificationResult,
    ) -> Result<Self> {
        Ok(Self {
            branch_id,
            expected_head_commit_id,
            target,
            result,
            method: CanonicalValue::object(Vec::new())?,
            evidence: Vec::new(),
            resource_basis: Vec::new(),
            rationale: CanonicalValue::object(Vec::new())?,
        })
    }

    pub fn with_method(mut self, method: CanonicalValue) -> Result<Self> {
        validate_verification_method(&method)?;
        self.method = method;
        Ok(self)
    }

    pub fn with_evidence<I>(mut self, evidence_ids: I) -> Result<Self>
    where
        I: IntoIterator<Item = EvidenceId>,
    {
        let mut evidence = evidence_ids
            .into_iter()
            .map(VerificationEvidenceRef::new)
            .collect::<Vec<_>>();
        evidence.sort_by_key(|evidence| evidence.evidence_id.raw_bytes());
        validate_verification_evidence_refs(&evidence)?;
        self.evidence = evidence;
        Ok(self)
    }

    pub fn with_resource_basis<I>(mut self, resource_basis: I) -> Result<Self>
    where
        I: IntoIterator<Item = VerificationResourceBasis>,
    {
        self.resource_basis = resource_basis.into_iter().collect();
        validate_verification_resource_basis(&self.resource_basis)?;
        Ok(self)
    }

    pub fn with_rationale(mut self, rationale: CanonicalValue) -> Self {
        self.rationale = rationale;
        self
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerificationEvidenceRelationCreate {
    pub evidence_id: EvidenceId,
    pub relation_operation_id: OperationId,
    pub relation_id: RelationId,
    pub relation_version_id: RelationVersionId,
    pub relation_state_digest: Digest,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerificationEvidenceRelationSnapshot {
    pub evidence_id: EvidenceId,
    pub relation_id: RelationId,
    pub relation_version_id: RelationVersionId,
    pub state_digest: Digest,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerificationCreateCommit {
    pub workspace_id: WorkspaceId,
    pub branch_id: BranchId,
    pub previous_head_commit_id: CommitId,
    pub commit_id: CommitId,
    pub changeset_id: ChangeSetId,
    pub verification_operation_id: OperationId,
    pub verifies_relation_operation_id: OperationId,
    pub verification_entity_id: EntityId,
    pub verification_entity_version_id: EntityVersionId,
    pub verification_state_digest: Digest,
    pub verifies_relation_id: RelationId,
    pub verifies_relation_version_id: RelationVersionId,
    pub verifies_relation_state_digest: Digest,
    pub evidenced_by_relations: Vec<VerificationEvidenceRelationCreate>,
    pub work_state_digest: Digest,
    pub target: VerificationTarget,
    pub state: VerificationState,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerificationSnapshot {
    pub workspace_id: WorkspaceId,
    pub commit_id: CommitId,
    pub verification_entity_id: EntityId,
    pub verification_entity_version_id: EntityVersionId,
    pub state_digest: Digest,
    pub verifies_relation_id: RelationId,
    pub verifies_relation_version_id: RelationVersionId,
    pub verifies_relation_state_digest: Digest,
    pub evidenced_by_relations: Vec<VerificationEvidenceRelationSnapshot>,
    pub target: VerificationTarget,
    pub state: VerificationState,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct VerificationRelationSnapshot {
    pub(crate) workspace_id: WorkspaceId,
    pub(crate) commit_id: CommitId,
    pub(crate) relation_id: RelationId,
    pub(crate) relation_version_id: RelationVersionId,
    pub(crate) source_verification_entity_id: EntityId,
    pub(crate) target: VerificationTarget,
    pub(crate) state_digest: Digest,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct VerificationEvidenceRelationAtSnapshot {
    pub(crate) workspace_id: WorkspaceId,
    pub(crate) commit_id: CommitId,
    pub(crate) relation_id: RelationId,
    pub(crate) relation_version_id: RelationVersionId,
    pub(crate) source_verification_entity_id: EntityId,
    pub(crate) evidence_id: EvidenceId,
    pub(crate) state_digest: Digest,
}

pub(crate) fn create_task(
    connection: &mut StoreConnection,
    options: &TaskCreateOptions,
) -> Result<TaskCreateCommit> {
    let entity_options = EntityTransitionOptions::create(
        options.branch_id,
        options.expected_head_commit_id,
        TASK_ENTITY_KIND,
        options.state.to_canonical_value()?,
    )?
    .with_rationale(options.rationale.clone());
    let commit = commit_entity_transition(connection, &entity_options)?;

    Ok(TaskCreateCommit {
        workspace_id: commit.workspace_id,
        branch_id: commit.branch_id,
        previous_head_commit_id: commit.previous_head_commit_id,
        commit_id: commit.commit_id,
        changeset_id: commit.changeset_id,
        operation_id: commit.operation_id,
        task_entity_id: commit.entity_id,
        task_entity_version_id: commit.entity_version_id,
        task_state_digest: commit.entity_state_digest,
        work_state_digest: commit.work_state_digest,
        state: options.state.clone(),
    })
}

pub(crate) fn reject_reserved_semantic_entity_transition(
    connection: &StoreConnection,
    options: &EntityTransitionOptions,
) -> Result<()> {
    if let Some(entity_kind) = options.created_entity_kind()
        && is_reserved_semantic_entity_kind(entity_kind)
    {
        return Err(reserved_semantic_entity_transition_error(entity_kind));
    }

    if let Some((entity_id, _)) = options.update_subject()
        && let Some(entity_kind) = load_entity_kind_for_public_boundary(connection, entity_id)?
        && is_reserved_semantic_entity_kind(&entity_kind)
    {
        return Err(reserved_semantic_entity_transition_error(&entity_kind));
    }

    Ok(())
}

pub(crate) fn transition_task(
    connection: &mut StoreConnection,
    options: &TaskTransitionOptions,
) -> Result<TaskTransitionCommit> {
    let current = task_at(
        connection,
        options.expected_head_commit_id,
        options.task_entity_id,
    )?;
    if current.task_entity_version_id != options.expected_task_entity_version_id {
        return Err(WorkVcsError::TaskInvalid(format!(
            "task entity {} expected version {}, found {} at commit {}",
            options.task_entity_id,
            options.expected_task_entity_version_id,
            current.task_entity_version_id,
            options.expected_head_commit_id
        )));
    }
    if options.next_status == TaskStatus::Done {
        require_mandatory_acceptance_criteria_verified(
            connection,
            options.branch_id,
            options.expected_head_commit_id,
            &current,
        )?;
    }

    let next_state = current.state.transition(
        options.next_status,
        &options.outcome_update,
        &options.rationale,
    )?;
    let entity_options = EntityTransitionOptions::update(
        options.branch_id,
        options.expected_head_commit_id,
        options.task_entity_id,
        options.expected_task_entity_version_id,
        next_state.to_canonical_value()?,
    )?
    .with_rationale(options.rationale.clone());
    let commit = commit_entity_transition(connection, &entity_options)?;

    Ok(TaskTransitionCommit {
        workspace_id: commit.workspace_id,
        branch_id: commit.branch_id,
        previous_head_commit_id: commit.previous_head_commit_id,
        commit_id: commit.commit_id,
        changeset_id: commit.changeset_id,
        operation_id: commit.operation_id,
        task_entity_id: commit.entity_id,
        previous_task_entity_version_id: options.expected_task_entity_version_id,
        task_entity_version_id: commit.entity_version_id,
        task_state_digest: commit.entity_state_digest,
        work_state_digest: commit.work_state_digest,
        previous_state: current.state,
        state: next_state,
    })
}

pub(crate) fn create_acceptance_criterion(
    connection: &mut StoreConnection,
    options: &AcceptanceCriterionCreateOptions,
) -> Result<AcceptanceCriterionCreateCommit> {
    connection.verify_foreign_keys()?;

    let parent = state_at(connection, options.expected_head_commit_id)?;
    let current_task = task_at(
        connection,
        options.expected_head_commit_id,
        options.task_entity_id,
    )?;
    if current_task.task_entity_version_id != options.expected_task_entity_version_id {
        return Err(WorkVcsError::TaskInvalid(format!(
            "task entity {} expected version {}, found {} at commit {}",
            options.task_entity_id,
            options.expected_task_entity_version_id,
            current_task.task_entity_version_id,
            options.expected_head_commit_id
        )));
    }
    if current_task
        .state
        .acceptance_criteria
        .iter()
        .any(|criterion| criterion.local_key == options.local_key)
    {
        return Err(WorkVcsError::TaskInvalid(format!(
            "task entity {} already has acceptance criterion local key {:?}",
            options.task_entity_id, options.local_key
        )));
    }
    if current_task.state.status == TaskStatus::Done
        && options.state.classification == AcceptanceCriterionClassification::Required
    {
        return Err(WorkVcsError::TaskInvalid(
            "mandatory acceptance criteria cannot be added to a done task without a dedicated semantic operation"
                .to_owned(),
        ));
    }

    let acceptance_criterion_entity_id = EntityId::new_v7();
    let acceptance_criterion_entity_version_id = EntityVersionId::new_v7();
    let task_entity_version_id = EntityVersionId::new_v7();
    let changeset_id = ChangeSetId::new_v7();
    let commit_id = CommitId::new_v7();
    let acceptance_criterion_operation_id = OperationId::new_v7();
    let task_operation_id = OperationId::new_v7();
    let now_us = current_epoch_micros()?;

    let mut task_state = current_task.state.clone();
    task_state
        .acceptance_criteria
        .push(TaskAcceptanceCriterionRef::new(
            options.local_key.clone(),
            acceptance_criterion_entity_id,
        )?);
    task_state.acceptance_criteria.sort_by(|left, right| {
        left.local_key.cmp(&right.local_key).then_with(|| {
            left.acceptance_criterion_entity_id
                .cmp(&right.acceptance_criterion_entity_id)
        })
    });
    validate_acceptance_criterion_refs(&task_state.acceptance_criteria)?;

    let acceptance_criterion_state_value = options.state.to_canonical_value()?;
    let acceptance_criterion_state_json = canonical_json_string(&acceptance_criterion_state_value)?;
    let acceptance_criterion_state_digest =
        entity_version_digest(&acceptance_criterion_state_value)?;
    let task_state_value = task_state.to_canonical_value()?;
    let task_state_json = canonical_json_string(&task_state_value)?;
    let task_state_digest = entity_version_digest(&task_state_value)?;
    let next_work_state = work_state_after_acceptance_criterion_create(
        &parent.state,
        options.task_entity_id,
        options.expected_task_entity_version_id,
        task_entity_version_id,
        acceptance_criterion_entity_id,
        acceptance_criterion_entity_version_id,
    )?;
    let work_state_digest = work_state_mapping_digest(&next_work_state);
    let acceptance_criterion_payload_value = entity_transition_payload_value(
        acceptance_criterion_entity_id,
        None,
        acceptance_criterion_entity_version_id,
    )?;
    let task_payload_value = entity_transition_payload_value(
        options.task_entity_id,
        Some(options.expected_task_entity_version_id),
        task_entity_version_id,
    )?;
    let acceptance_criterion_payload_json =
        canonical_json_string(&acceptance_criterion_payload_value)?;
    let task_payload_json = canonical_json_string(&task_payload_value)?;
    let changeset_payload_json = canonical_json_string(&CanonicalValue::object(vec![(
        "operations".to_owned(),
        CanonicalValue::Array(vec![acceptance_criterion_payload_value, task_payload_value]),
    )])?)?;
    let rationale_json = canonical_json_string(&options.rationale)?;

    let transaction = connection
        .inner_mut()
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(storage_error)?;
    let branch = load_active_branch(&transaction, options.branch_id)?;
    if branch.head_commit_id != options.expected_head_commit_id {
        return Err(WorkVcsError::BranchHeadConflict(format!(
            "branch {} expected head {}, found {}",
            options.branch_id, options.expected_head_commit_id, branch.head_commit_id
        )));
    }
    if branch.workspace_id != parent.workspace_id {
        return Err(WorkVcsError::TaskInvalid(format!(
            "branch {} belongs to workspace {}, but expected head {} belongs to workspace {}",
            options.branch_id,
            branch.workspace_id,
            options.expected_head_commit_id,
            parent.workspace_id
        )));
    }
    ensure_acceptance_criterion_local_key_available(
        &transaction,
        options.task_entity_id,
        &options.local_key,
    )?;
    write_acceptance_criterion_create(
        &transaction,
        &AcceptanceCriterionCreateRows {
            workspace_id: branch.workspace_id,
            expected_head_commit_id: options.expected_head_commit_id,
            task_entity_id: options.task_entity_id,
            previous_task_entity_version_id: options.expected_task_entity_version_id,
            task_entity_version_id,
            task_state_json,
            task_state_digest,
            acceptance_criterion_entity_id,
            acceptance_criterion_entity_version_id,
            acceptance_criterion_state_json,
            acceptance_criterion_state_digest,
            local_key: options.local_key.clone(),
            changeset_id,
            commit_id,
            acceptance_criterion_operation_id,
            task_operation_id,
            acceptance_criterion_payload_json,
            task_payload_json,
            changeset_payload_json,
            rationale_json,
            work_state_digest,
            now_us,
        },
    )?;
    move_branch_head(
        &transaction,
        options.branch_id,
        options.expected_head_commit_id,
        commit_id,
        now_us,
    )?;
    transaction.commit().map_err(storage_error)?;

    Ok(AcceptanceCriterionCreateCommit {
        workspace_id: branch.workspace_id,
        branch_id: options.branch_id,
        previous_head_commit_id: options.expected_head_commit_id,
        commit_id,
        changeset_id,
        acceptance_criterion_operation_id,
        task_operation_id,
        task_entity_id: options.task_entity_id,
        previous_task_entity_version_id: options.expected_task_entity_version_id,
        task_entity_version_id,
        task_state_digest,
        acceptance_criterion_entity_id,
        acceptance_criterion_entity_version_id,
        acceptance_criterion_state_digest,
        work_state_digest,
        local_key: options.local_key.clone(),
        state: options.state.clone(),
        task_state,
    })
}

pub(crate) fn revise_acceptance_criterion(
    connection: &mut StoreConnection,
    options: &AcceptanceCriterionRevisionOptions,
) -> Result<AcceptanceCriterionRevisionCommit> {
    let current = acceptance_criterion_at(
        connection,
        options.expected_head_commit_id,
        options.acceptance_criterion_entity_id,
    )?;
    if current.acceptance_criterion_entity_version_id
        != options.expected_acceptance_criterion_entity_version_id
    {
        return Err(WorkVcsError::TaskInvalid(format!(
            "acceptance criterion entity {} expected version {}, found {} at commit {}",
            options.acceptance_criterion_entity_id,
            options.expected_acceptance_criterion_entity_version_id,
            current.acceptance_criterion_entity_version_id,
            options.expected_head_commit_id
        )));
    }
    let task = task_at(
        connection,
        options.expected_head_commit_id,
        current.task_entity_id,
    )?;
    require_task_references_acceptance_criterion(&task.state, &current)?;
    if task.state.status == TaskStatus::Done
        && options.state.classification == AcceptanceCriterionClassification::Required
    {
        return Err(WorkVcsError::TaskInvalid(
            "mandatory acceptance criteria cannot be revised under a done task without a dedicated semantic operation"
                .to_owned(),
        ));
    }
    let next_state = options
        .state
        .clone()
        .with_verification_requirements(current.state.verification_requirements.clone())?;
    if current.state == next_state {
        return Err(WorkVcsError::TaskInvalid(
            "acceptance criterion revision must change statement or classification".to_owned(),
        ));
    }

    let entity_options = EntityTransitionOptions::update(
        options.branch_id,
        options.expected_head_commit_id,
        options.acceptance_criterion_entity_id,
        options.expected_acceptance_criterion_entity_version_id,
        next_state.to_canonical_value()?,
    )?
    .with_rationale(options.rationale.clone());
    let commit = commit_entity_transition(connection, &entity_options)?;

    Ok(AcceptanceCriterionRevisionCommit {
        workspace_id: commit.workspace_id,
        branch_id: commit.branch_id,
        previous_head_commit_id: commit.previous_head_commit_id,
        commit_id: commit.commit_id,
        changeset_id: commit.changeset_id,
        operation_id: commit.operation_id,
        task_entity_id: current.task_entity_id,
        acceptance_criterion_entity_id: commit.entity_id,
        previous_acceptance_criterion_entity_version_id: options
            .expected_acceptance_criterion_entity_version_id,
        acceptance_criterion_entity_version_id: commit.entity_version_id,
        acceptance_criterion_state_digest: commit.entity_state_digest,
        work_state_digest: commit.work_state_digest,
        local_key: current.local_key,
        previous_state: current.state,
        state: next_state,
    })
}

pub(crate) fn task_at(
    connection: &StoreConnection,
    commit_id: CommitId,
    task_entity_id: EntityId,
) -> Result<TaskSnapshot> {
    let replayed = state_at(connection, commit_id)?;
    let Some(task_entity_version_id) =
        replayed
            .state
            .entities()
            .iter()
            .find_map(|(entity_id, entity_version_id)| {
                (*entity_id == task_entity_id).then_some(*entity_version_id)
            })
    else {
        return Err(WorkVcsError::TaskNotFound(format!(
            "task entity {task_entity_id} is not present at commit {commit_id}"
        )));
    };

    let loaded = load_task_version(
        connection,
        replayed.workspace_id,
        task_entity_id,
        task_entity_version_id,
    )?;
    Ok(TaskSnapshot {
        workspace_id: replayed.workspace_id,
        commit_id,
        task_entity_id,
        task_entity_version_id,
        state_digest: loaded.state_digest,
        state: loaded.state,
    })
}

pub(crate) fn tasks_at(
    connection: &StoreConnection,
    commit_id: CommitId,
) -> Result<Vec<TaskSnapshot>> {
    let replayed = state_at(connection, commit_id)?;
    let mut tasks = Vec::new();

    for (entity_id, entity_version_id) in replayed.state.entities() {
        match load_entity_kind_for_public_boundary(connection, *entity_id)? {
            Some(entity_kind) if entity_kind == TASK_ENTITY_KIND => {
                let loaded = load_task_version(
                    connection,
                    replayed.workspace_id,
                    *entity_id,
                    *entity_version_id,
                )?;
                tasks.push(TaskSnapshot {
                    workspace_id: replayed.workspace_id,
                    commit_id,
                    task_entity_id: *entity_id,
                    task_entity_version_id: *entity_version_id,
                    state_digest: loaded.state_digest,
                    state: loaded.state,
                });
            }
            Some(_) => {}
            None => {
                return Err(WorkVcsError::TaskInvalid(format!(
                    "WorkState at commit {commit_id} references missing entity {entity_id}"
                )));
            }
        }
    }

    tasks.sort_by_key(|task| task.task_entity_id);
    Ok(tasks)
}

pub(crate) fn create_task_scheduling_relation(
    connection: &mut StoreConnection,
    options: &TaskSchedulingRelationCreateOptions,
) -> Result<TaskSchedulingRelationCreateCommit> {
    connection.verify_foreign_keys()?;
    if options.source_task_entity_id == options.target_task_entity_id {
        return Err(WorkVcsError::TaskInvalid(format!(
            "task scheduling relation {} cannot use the same task {} as both source and target",
            options.relation_type, options.source_task_entity_id
        )));
    }

    let parent = state_at(connection, options.expected_head_commit_id)?;
    let source = task_at(
        connection,
        options.expected_head_commit_id,
        options.source_task_entity_id,
    )?;
    let target = task_at(
        connection,
        options.expected_head_commit_id,
        options.target_task_entity_id,
    )?;
    if source.workspace_id != parent.workspace_id || target.workspace_id != parent.workspace_id {
        return Err(WorkVcsError::TaskInvalid(format!(
            "task scheduling relation endpoints must belong to workspace {}",
            parent.workspace_id
        )));
    }
    if options.relation_type == TaskSchedulingRelationType::DependsOn {
        ensure_dependency_create_is_acyclic(
            connection,
            options.expected_head_commit_id,
            parent.workspace_id,
            &parent.state,
            options.source_task_entity_id,
            options.target_task_entity_id,
        )?;
    }

    let relation_id = RelationId::new_v7();
    let relation_version_id = RelationVersionId::new_v7();
    let changeset_id = ChangeSetId::new_v7();
    let commit_id = CommitId::new_v7();
    let operation_id = OperationId::new_v7();
    let now_us = current_epoch_micros()?;

    let relation_state_value = CanonicalValue::object(Vec::new())?;
    let relation_state_json = canonical_json_string(&relation_state_value)?;
    let relation_state_digest = relation_version_digest(&relation_state_value)?;
    let next_work_state = work_state_after_task_scheduling_relation_create(
        &parent.state,
        relation_id,
        relation_version_id,
    )?;
    let work_state_digest = work_state_mapping_digest(&next_work_state);
    let relation_payload_value =
        relation_transition_payload_value(relation_id, None, relation_version_id)?;
    let relation_payload_json = canonical_json_string(&relation_payload_value)?;
    let rationale_json = canonical_json_string(&options.rationale)?;

    let transaction = connection
        .inner_mut()
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(storage_error)?;
    let branch = load_active_branch(&transaction, options.branch_id)?;
    if branch.head_commit_id != options.expected_head_commit_id {
        return Err(WorkVcsError::BranchHeadConflict(format!(
            "branch {} expected head {}, found {}",
            options.branch_id, options.expected_head_commit_id, branch.head_commit_id
        )));
    }
    if branch.workspace_id != parent.workspace_id {
        return Err(WorkVcsError::TaskInvalid(format!(
            "branch {} belongs to workspace {}, but expected head {} belongs to workspace {}",
            options.branch_id,
            branch.workspace_id,
            options.expected_head_commit_id,
            parent.workspace_id
        )));
    }
    ensure_task_scheduling_relation_logical_key_available(
        &transaction,
        branch.workspace_id,
        options.relation_type,
        options.source_task_entity_id,
        options.target_task_entity_id,
    )?;
    write_task_scheduling_relation_create(
        &transaction,
        &TaskSchedulingRelationCreateRows {
            workspace_id: branch.workspace_id,
            expected_head_commit_id: options.expected_head_commit_id,
            relation_id,
            relation_version_id,
            relation_state_json,
            relation_state_digest,
            relation_type: options.relation_type,
            source_task_entity_id: options.source_task_entity_id,
            target_task_entity_id: options.target_task_entity_id,
            changeset_id,
            commit_id,
            operation_id,
            relation_payload_json,
            rationale_json,
            work_state_digest,
            now_us,
        },
    )?;
    move_branch_head(
        &transaction,
        options.branch_id,
        options.expected_head_commit_id,
        commit_id,
        now_us,
    )?;
    transaction.commit().map_err(storage_error)?;

    Ok(TaskSchedulingRelationCreateCommit {
        workspace_id: branch.workspace_id,
        branch_id: options.branch_id,
        previous_head_commit_id: options.expected_head_commit_id,
        commit_id,
        changeset_id,
        operation_id,
        relation_id,
        relation_version_id,
        relation_type: options.relation_type,
        source_task_entity_id: options.source_task_entity_id,
        target_task_entity_id: options.target_task_entity_id,
        relation_state_digest,
        work_state_digest,
    })
}

pub(crate) fn task_scheduling_relations_at(
    connection: &StoreConnection,
    commit_id: CommitId,
) -> Result<Vec<TaskSchedulingRelationSnapshot>> {
    let replayed = state_at(connection, commit_id)?;
    let mut relations = Vec::new();

    for (relation_id, relation_version_id) in replayed.state.relations() {
        let Some(relation) = load_task_scheduling_relation_version(
            connection,
            replayed.workspace_id,
            commit_id,
            *relation_id,
            *relation_version_id,
        )?
        else {
            continue;
        };
        relations.push(TaskSchedulingRelationSnapshot {
            workspace_id: replayed.workspace_id,
            commit_id,
            relation_id: relation.relation_id,
            relation_version_id: relation.relation_version_id,
            relation_type: relation.relation_type,
            source_task_entity_id: relation.source_task_entity_id,
            target_task_entity_id: relation.target_task_entity_id,
            state_digest: relation.state_digest,
        });
    }

    relations.sort_by(|left, right| {
        left.relation_type
            .cmp(&right.relation_type)
            .then_with(|| left.source_task_entity_id.cmp(&right.source_task_entity_id))
            .then_with(|| left.target_task_entity_id.cmp(&right.target_task_entity_id))
            .then_with(|| left.relation_id.cmp(&right.relation_id))
    });
    Ok(relations)
}

pub(crate) fn verification_relations_at(
    connection: &StoreConnection,
    commit_id: CommitId,
) -> Result<Vec<VerificationRelationSnapshot>> {
    let replayed = state_at(connection, commit_id)?;
    let mut relations = Vec::new();

    for (relation_id, relation_version_id) in replayed.state.relations() {
        let Some(relation) = load_verifies_relation_version(
            connection,
            replayed.workspace_id,
            *relation_id,
            *relation_version_id,
        )?
        else {
            continue;
        };
        validate_current_verifies_relation_endpoints(
            connection,
            replayed.workspace_id,
            &replayed.state,
            &relation,
        )?;
        relations.push(VerificationRelationSnapshot {
            workspace_id: replayed.workspace_id,
            commit_id,
            relation_id: relation.relation_id,
            relation_version_id: relation.relation_version_id,
            source_verification_entity_id: relation.source_verification_entity_id,
            target: relation.target,
            state_digest: relation.state_digest,
        });
    }

    relations.sort_by(|left, right| {
        left.source_verification_entity_id
            .cmp(&right.source_verification_entity_id)
            .then_with(|| {
                verification_target_sort_key(left.target)
                    .cmp(&verification_target_sort_key(right.target))
            })
            .then_with(|| left.relation_id.cmp(&right.relation_id))
    });
    Ok(relations)
}

pub(crate) fn verification_evidence_relations_at(
    connection: &StoreConnection,
    commit_id: CommitId,
) -> Result<Vec<VerificationEvidenceRelationAtSnapshot>> {
    let replayed = state_at(connection, commit_id)?;
    let mut relations = Vec::new();

    for (relation_id, relation_version_id) in replayed.state.relations() {
        let Some(relation) = load_evidenced_by_relation_version(
            connection,
            replayed.workspace_id,
            *relation_id,
            *relation_version_id,
        )?
        else {
            continue;
        };
        validate_current_evidenced_by_relation_source(
            connection,
            replayed.workspace_id,
            &replayed.state,
            &relation,
        )?;
        relations.push(VerificationEvidenceRelationAtSnapshot {
            workspace_id: replayed.workspace_id,
            commit_id,
            relation_id: relation.relation_id,
            relation_version_id: relation.relation_version_id,
            source_verification_entity_id: relation.source_verification_entity_id,
            evidence_id: relation.evidence_id,
            state_digest: relation.state_digest,
        });
    }

    relations.sort_by(|left, right| {
        left.source_verification_entity_id
            .cmp(&right.source_verification_entity_id)
            .then_with(|| left.evidence_id.cmp(&right.evidence_id))
            .then_with(|| left.relation_id.cmp(&right.relation_id))
    });
    Ok(relations)
}

fn validate_current_verifies_relation_endpoints(
    connection: &StoreConnection,
    workspace_id: WorkspaceId,
    state: &WorkState,
    relation: &LoadedVerifiesRelationVersion,
) -> Result<()> {
    let source_version_id = current_verifies_endpoint_version_id(
        state,
        relation.relation_id,
        "source verification",
        relation.source_verification_entity_id,
    )?;
    load_verification_version(
        connection,
        workspace_id,
        relation.source_verification_entity_id,
        source_version_id,
    )?;

    let target_entity_id = relation.target.entity_id();
    let target_version_id = current_verifies_endpoint_version_id(
        state,
        relation.relation_id,
        "target",
        target_entity_id,
    )?;
    match relation.target {
        VerificationTarget::AcceptanceCriterion(acceptance_criterion_entity_id) => {
            load_acceptance_criterion_version(
                connection,
                workspace_id,
                acceptance_criterion_entity_id,
                target_version_id,
            )?;
        }
        VerificationTarget::VerificationRequirement(verification_requirement_entity_id) => {
            load_verification_requirement_version(
                connection,
                workspace_id,
                verification_requirement_entity_id,
                target_version_id,
            )?;
        }
    }

    Ok(())
}

fn validate_current_evidenced_by_relation_source(
    connection: &StoreConnection,
    workspace_id: WorkspaceId,
    state: &WorkState,
    relation: &LoadedEvidenceRelationVersion,
) -> Result<()> {
    let source_version_id = current_relation_entity_endpoint_version_id(
        EVIDENCED_BY_RELATION_TYPE,
        state,
        relation.relation_id,
        "source verification",
        relation.source_verification_entity_id,
    )?;
    load_verification_version(
        connection,
        workspace_id,
        relation.source_verification_entity_id,
        source_version_id,
    )?;
    Ok(())
}

fn current_verifies_endpoint_version_id(
    state: &WorkState,
    relation_id: RelationId,
    endpoint_role: &str,
    entity_id: EntityId,
) -> Result<EntityVersionId> {
    current_relation_entity_endpoint_version_id(
        VERIFIES_RELATION_TYPE,
        state,
        relation_id,
        endpoint_role,
        entity_id,
    )
}

fn current_relation_entity_endpoint_version_id(
    relation_type: &str,
    state: &WorkState,
    relation_id: RelationId,
    endpoint_role: &str,
    entity_id: EntityId,
) -> Result<EntityVersionId> {
    state
        .entities()
        .iter()
        .find_map(|(current_entity_id, entity_version_id)| {
            (*current_entity_id == entity_id).then_some(*entity_version_id)
        })
        .ok_or_else(|| {
            WorkVcsError::TaskInvalid(format!(
                "{relation_type} relation {relation_id} {endpoint_role} entity {entity_id} is not present in WorkState"
            ))
        })
}

pub(crate) fn acceptance_criterion_at(
    connection: &StoreConnection,
    commit_id: CommitId,
    acceptance_criterion_entity_id: EntityId,
) -> Result<AcceptanceCriterionSnapshot> {
    let replayed = state_at(connection, commit_id)?;
    let Some(acceptance_criterion_entity_version_id) =
        replayed
            .state
            .entities()
            .iter()
            .find_map(|(entity_id, entity_version_id)| {
                (*entity_id == acceptance_criterion_entity_id).then_some(*entity_version_id)
            })
    else {
        return Err(WorkVcsError::TaskNotFound(format!(
            "acceptance criterion entity {acceptance_criterion_entity_id} is not present at commit {commit_id}"
        )));
    };

    let loaded = load_acceptance_criterion_version(
        connection,
        replayed.workspace_id,
        acceptance_criterion_entity_id,
        acceptance_criterion_entity_version_id,
    )?;
    Ok(AcceptanceCriterionSnapshot {
        workspace_id: replayed.workspace_id,
        commit_id,
        task_entity_id: loaded.task_entity_id,
        local_key: loaded.local_key,
        acceptance_criterion_entity_id,
        acceptance_criterion_entity_version_id,
        state_digest: loaded.state_digest,
        state: loaded.state,
    })
}

pub(crate) fn acceptance_criteria_at(
    connection: &StoreConnection,
    commit_id: CommitId,
) -> Result<Vec<AcceptanceCriterionSnapshot>> {
    let replayed = state_at(connection, commit_id)?;
    let mut criteria = Vec::new();

    for (entity_id, entity_version_id) in replayed.state.entities() {
        match load_entity_kind_for_public_boundary(connection, *entity_id)? {
            Some(entity_kind) if entity_kind == ACCEPTANCE_CRITERION_ENTITY_KIND => {
                let loaded = load_acceptance_criterion_version(
                    connection,
                    replayed.workspace_id,
                    *entity_id,
                    *entity_version_id,
                )?;
                criteria.push(AcceptanceCriterionSnapshot {
                    workspace_id: replayed.workspace_id,
                    commit_id,
                    task_entity_id: loaded.task_entity_id,
                    local_key: loaded.local_key,
                    acceptance_criterion_entity_id: *entity_id,
                    acceptance_criterion_entity_version_id: *entity_version_id,
                    state_digest: loaded.state_digest,
                    state: loaded.state,
                });
            }
            Some(_) => {}
            None => {
                return Err(WorkVcsError::TaskInvalid(format!(
                    "WorkState at commit {commit_id} references missing entity {entity_id}"
                )));
            }
        }
    }

    criteria.sort_by(|left, right| {
        left.task_entity_id
            .cmp(&right.task_entity_id)
            .then_with(|| left.local_key.cmp(&right.local_key))
            .then_with(|| {
                left.acceptance_criterion_entity_id
                    .cmp(&right.acceptance_criterion_entity_id)
            })
    });
    Ok(criteria)
}

pub(crate) fn create_verification_requirement(
    connection: &mut StoreConnection,
    options: &VerificationRequirementCreateOptions,
) -> Result<VerificationRequirementCreateCommit> {
    connection.verify_foreign_keys()?;

    let parent = state_at(connection, options.expected_head_commit_id)?;
    let current_criterion = acceptance_criterion_at(
        connection,
        options.expected_head_commit_id,
        options.acceptance_criterion_entity_id,
    )?;
    if current_criterion.acceptance_criterion_entity_version_id
        != options.expected_acceptance_criterion_entity_version_id
    {
        return Err(WorkVcsError::TaskInvalid(format!(
            "acceptance criterion entity {} expected version {}, found {} at commit {}",
            options.acceptance_criterion_entity_id,
            options.expected_acceptance_criterion_entity_version_id,
            current_criterion.acceptance_criterion_entity_version_id,
            options.expected_head_commit_id
        )));
    }
    if current_criterion
        .state
        .verification_requirements
        .iter()
        .any(|requirement| requirement.local_key == options.local_key)
    {
        return Err(WorkVcsError::TaskInvalid(format!(
            "acceptance criterion entity {} already has verification requirement local key {:?}",
            options.acceptance_criterion_entity_id, options.local_key
        )));
    }
    let owner_task = task_at(
        connection,
        options.expected_head_commit_id,
        current_criterion.task_entity_id,
    )?;
    require_task_references_acceptance_criterion(&owner_task.state, &current_criterion)?;
    if owner_task.state.status == TaskStatus::Done
        && current_criterion.state.classification == AcceptanceCriterionClassification::Required
    {
        return Err(WorkVcsError::TaskInvalid(
            "verification requirements cannot be added to a required acceptance criterion under a done task"
                .to_owned(),
        ));
    }

    let verification_requirement_entity_id = EntityId::new_v7();
    let verification_requirement_entity_version_id = EntityVersionId::new_v7();
    let acceptance_criterion_entity_version_id = EntityVersionId::new_v7();
    let changeset_id = ChangeSetId::new_v7();
    let commit_id = CommitId::new_v7();
    let verification_requirement_operation_id = OperationId::new_v7();
    let acceptance_criterion_operation_id = OperationId::new_v7();
    let now_us = current_epoch_micros()?;

    let mut acceptance_criterion_state = current_criterion.state.clone();
    acceptance_criterion_state.verification_requirements.push(
        AcceptanceCriterionVerificationRequirementRef::new(
            options.local_key.clone(),
            verification_requirement_entity_id,
        )?,
    );
    acceptance_criterion_state
        .verification_requirements
        .sort_by(|left, right| {
            left.local_key.cmp(&right.local_key).then_with(|| {
                left.verification_requirement_entity_id
                    .cmp(&right.verification_requirement_entity_id)
            })
        });
    validate_verification_requirement_refs(&acceptance_criterion_state.verification_requirements)?;

    let verification_requirement_state_value = options.state.to_canonical_value()?;
    let verification_requirement_state_json =
        canonical_json_string(&verification_requirement_state_value)?;
    let verification_requirement_state_digest =
        entity_version_digest(&verification_requirement_state_value)?;
    let acceptance_criterion_state_value = acceptance_criterion_state.to_canonical_value()?;
    let acceptance_criterion_state_json = canonical_json_string(&acceptance_criterion_state_value)?;
    let acceptance_criterion_state_digest =
        entity_version_digest(&acceptance_criterion_state_value)?;
    let next_work_state = work_state_after_verification_requirement_create(
        &parent.state,
        options.acceptance_criterion_entity_id,
        options.expected_acceptance_criterion_entity_version_id,
        acceptance_criterion_entity_version_id,
        verification_requirement_entity_id,
        verification_requirement_entity_version_id,
    )?;
    let work_state_digest = work_state_mapping_digest(&next_work_state);
    let verification_requirement_payload_value = entity_transition_payload_value(
        verification_requirement_entity_id,
        None,
        verification_requirement_entity_version_id,
    )?;
    let acceptance_criterion_payload_value = entity_transition_payload_value(
        options.acceptance_criterion_entity_id,
        Some(options.expected_acceptance_criterion_entity_version_id),
        acceptance_criterion_entity_version_id,
    )?;
    let verification_requirement_payload_json =
        canonical_json_string(&verification_requirement_payload_value)?;
    let acceptance_criterion_payload_json =
        canonical_json_string(&acceptance_criterion_payload_value)?;
    let changeset_payload_json = canonical_json_string(&CanonicalValue::object(vec![(
        "operations".to_owned(),
        CanonicalValue::Array(vec![
            verification_requirement_payload_value,
            acceptance_criterion_payload_value,
        ]),
    )])?)?;
    let rationale_json = canonical_json_string(&options.rationale)?;

    let transaction = connection
        .inner_mut()
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(storage_error)?;
    let branch = load_active_branch(&transaction, options.branch_id)?;
    if branch.head_commit_id != options.expected_head_commit_id {
        return Err(WorkVcsError::BranchHeadConflict(format!(
            "branch {} expected head {}, found {}",
            options.branch_id, options.expected_head_commit_id, branch.head_commit_id
        )));
    }
    if branch.workspace_id != parent.workspace_id {
        return Err(WorkVcsError::TaskInvalid(format!(
            "branch {} belongs to workspace {}, but expected head {} belongs to workspace {}",
            options.branch_id,
            branch.workspace_id,
            options.expected_head_commit_id,
            parent.workspace_id
        )));
    }
    ensure_verification_requirement_local_key_available(
        &transaction,
        options.acceptance_criterion_entity_id,
        &options.local_key,
    )?;
    write_verification_requirement_create(
        &transaction,
        &VerificationRequirementCreateRows {
            workspace_id: branch.workspace_id,
            expected_head_commit_id: options.expected_head_commit_id,
            acceptance_criterion_entity_id: options.acceptance_criterion_entity_id,
            previous_acceptance_criterion_entity_version_id: options
                .expected_acceptance_criterion_entity_version_id,
            acceptance_criterion_entity_version_id,
            acceptance_criterion_state_json,
            acceptance_criterion_state_digest,
            verification_requirement_entity_id,
            verification_requirement_entity_version_id,
            verification_requirement_state_json,
            verification_requirement_state_digest,
            local_key: options.local_key.clone(),
            changeset_id,
            commit_id,
            verification_requirement_operation_id,
            acceptance_criterion_operation_id,
            verification_requirement_payload_json,
            acceptance_criterion_payload_json,
            changeset_payload_json,
            rationale_json,
            work_state_digest,
            now_us,
        },
    )?;
    move_branch_head(
        &transaction,
        options.branch_id,
        options.expected_head_commit_id,
        commit_id,
        now_us,
    )?;
    transaction.commit().map_err(storage_error)?;

    Ok(VerificationRequirementCreateCommit {
        workspace_id: branch.workspace_id,
        branch_id: options.branch_id,
        previous_head_commit_id: options.expected_head_commit_id,
        commit_id,
        changeset_id,
        verification_requirement_operation_id,
        acceptance_criterion_operation_id,
        acceptance_criterion_entity_id: options.acceptance_criterion_entity_id,
        previous_acceptance_criterion_entity_version_id: options
            .expected_acceptance_criterion_entity_version_id,
        acceptance_criterion_entity_version_id,
        acceptance_criterion_state_digest,
        verification_requirement_entity_id,
        verification_requirement_entity_version_id,
        verification_requirement_state_digest,
        work_state_digest,
        local_key: options.local_key.clone(),
        state: options.state.clone(),
        acceptance_criterion_state,
    })
}

pub(crate) fn revise_verification_requirement(
    connection: &mut StoreConnection,
    options: &VerificationRequirementRevisionOptions,
) -> Result<VerificationRequirementRevisionCommit> {
    let current = verification_requirement_at(
        connection,
        options.expected_head_commit_id,
        options.verification_requirement_entity_id,
    )?;
    if current.verification_requirement_entity_version_id
        != options.expected_verification_requirement_entity_version_id
    {
        return Err(WorkVcsError::TaskInvalid(format!(
            "verification requirement entity {} expected version {}, found {} at commit {}",
            options.verification_requirement_entity_id,
            options.expected_verification_requirement_entity_version_id,
            current.verification_requirement_entity_version_id,
            options.expected_head_commit_id
        )));
    }
    let criterion = acceptance_criterion_at(
        connection,
        options.expected_head_commit_id,
        current.acceptance_criterion_entity_id,
    )?;
    require_acceptance_criterion_references_verification_requirement(&criterion.state, &current)?;
    let task = task_at(
        connection,
        options.expected_head_commit_id,
        criterion.task_entity_id,
    )?;
    require_task_references_acceptance_criterion(&task.state, &criterion)?;
    if task.state.status == TaskStatus::Done
        && criterion.state.classification == AcceptanceCriterionClassification::Required
    {
        return Err(WorkVcsError::TaskInvalid(
            "verification requirements cannot be revised under a required acceptance criterion on a done task"
                .to_owned(),
        ));
    }
    if current.state == options.state {
        return Err(WorkVcsError::TaskInvalid(
            "verification requirement revision must change statement".to_owned(),
        ));
    }

    let entity_options = EntityTransitionOptions::update(
        options.branch_id,
        options.expected_head_commit_id,
        options.verification_requirement_entity_id,
        options.expected_verification_requirement_entity_version_id,
        options.state.to_canonical_value()?,
    )?
    .with_rationale(options.rationale.clone());
    let commit = commit_entity_transition(connection, &entity_options)?;

    Ok(VerificationRequirementRevisionCommit {
        workspace_id: commit.workspace_id,
        branch_id: commit.branch_id,
        previous_head_commit_id: commit.previous_head_commit_id,
        commit_id: commit.commit_id,
        changeset_id: commit.changeset_id,
        operation_id: commit.operation_id,
        acceptance_criterion_entity_id: current.acceptance_criterion_entity_id,
        local_key: current.local_key,
        verification_requirement_entity_id: commit.entity_id,
        previous_verification_requirement_entity_version_id: options
            .expected_verification_requirement_entity_version_id,
        verification_requirement_entity_version_id: commit.entity_version_id,
        verification_requirement_state_digest: commit.entity_state_digest,
        work_state_digest: commit.work_state_digest,
        previous_state: current.state,
        state: options.state.clone(),
    })
}

pub(crate) fn verification_requirement_at(
    connection: &StoreConnection,
    commit_id: CommitId,
    verification_requirement_entity_id: EntityId,
) -> Result<VerificationRequirementSnapshot> {
    let replayed = state_at(connection, commit_id)?;
    let Some(verification_requirement_entity_version_id) = replayed
        .state
        .entities()
        .iter()
        .find_map(|(entity_id, entity_version_id)| {
            (*entity_id == verification_requirement_entity_id).then_some(*entity_version_id)
        })
    else {
        return Err(WorkVcsError::TaskNotFound(format!(
            "verification requirement entity {verification_requirement_entity_id} is not present at commit {commit_id}"
        )));
    };

    let loaded = load_verification_requirement_version(
        connection,
        replayed.workspace_id,
        verification_requirement_entity_id,
        verification_requirement_entity_version_id,
    )?;
    Ok(VerificationRequirementSnapshot {
        workspace_id: replayed.workspace_id,
        commit_id,
        acceptance_criterion_entity_id: loaded.acceptance_criterion_entity_id,
        local_key: loaded.local_key,
        verification_requirement_entity_id,
        verification_requirement_entity_version_id,
        state_digest: loaded.state_digest,
        state: loaded.state,
    })
}

pub(crate) fn verification_requirements_at(
    connection: &StoreConnection,
    commit_id: CommitId,
) -> Result<Vec<VerificationRequirementSnapshot>> {
    let replayed = state_at(connection, commit_id)?;
    let mut requirements = Vec::new();

    for (entity_id, entity_version_id) in replayed.state.entities() {
        match load_entity_kind_for_public_boundary(connection, *entity_id)? {
            Some(entity_kind) if entity_kind == VERIFICATION_REQUIREMENT_ENTITY_KIND => {
                let loaded = load_verification_requirement_version(
                    connection,
                    replayed.workspace_id,
                    *entity_id,
                    *entity_version_id,
                )?;
                requirements.push(VerificationRequirementSnapshot {
                    workspace_id: replayed.workspace_id,
                    commit_id,
                    acceptance_criterion_entity_id: loaded.acceptance_criterion_entity_id,
                    local_key: loaded.local_key,
                    verification_requirement_entity_id: *entity_id,
                    verification_requirement_entity_version_id: *entity_version_id,
                    state_digest: loaded.state_digest,
                    state: loaded.state,
                });
            }
            Some(_) => {}
            None => {
                return Err(WorkVcsError::TaskInvalid(format!(
                    "WorkState at commit {commit_id} references missing entity {entity_id}"
                )));
            }
        }
    }

    requirements.sort_by(|left, right| {
        left.acceptance_criterion_entity_id
            .cmp(&right.acceptance_criterion_entity_id)
            .then_with(|| left.local_key.cmp(&right.local_key))
            .then_with(|| {
                left.verification_requirement_entity_id
                    .cmp(&right.verification_requirement_entity_id)
            })
    });
    Ok(requirements)
}

pub(crate) fn create_verification(
    connection: &mut StoreConnection,
    options: &VerificationCreateOptions,
) -> Result<VerificationCreateCommit> {
    connection.verify_foreign_keys()?;
    validate_verification_method(&options.method)?;
    validate_verification_evidence_refs(&options.evidence)?;
    validate_verification_resource_basis(&options.resource_basis)?;
    for evidence in &options.evidence {
        require_evidence_exists(connection, evidence.evidence_id)?;
    }
    for resource_basis in &options.resource_basis {
        require_verification_resource_basis_references(connection, resource_basis)?;
    }

    let parent = state_at(connection, options.expected_head_commit_id)?;
    let semantic_dependencies = verification_semantic_dependencies_for_target(
        connection,
        options.expected_head_commit_id,
        options.target,
    )?;

    let verification_entity_id = EntityId::new_v7();
    let verification_entity_version_id = EntityVersionId::new_v7();
    let verifies_relation_id = RelationId::new_v7();
    let verifies_relation_version_id = RelationVersionId::new_v7();
    let changeset_id = ChangeSetId::new_v7();
    let commit_id = CommitId::new_v7();
    let verification_operation_id = OperationId::new_v7();
    let verifies_relation_operation_id = OperationId::new_v7();
    let now_us = current_epoch_micros()?;

    let verification_state = VerificationState::new_with_evidence_and_resource_basis(
        options.result,
        options.method.clone(),
        options.expected_head_commit_id,
        semantic_dependencies,
        options.evidence.clone(),
        options.resource_basis.clone(),
    )?;
    let verification_state_value = verification_state.to_canonical_value()?;
    let verification_state_json = canonical_json_string(&verification_state_value)?;
    let verification_state_digest = entity_version_digest(&verification_state_value)?;
    let relation_state_value = CanonicalValue::object(Vec::new())?;
    let verifies_relation_state_json = canonical_json_string(&relation_state_value)?;
    let verifies_relation_state_digest = relation_version_digest(&relation_state_value)?;
    let evidenced_by_relation_state_json = verifies_relation_state_json.clone();
    let evidenced_by_relation_state_digest = verifies_relation_state_digest;
    let basis_json = canonical_json_string(&verification_basis_value(
        verification_state.verified_at_commit_id,
        &verification_state.semantic_dependencies,
        &verification_state.resource_basis,
    )?)?;
    let mut evidenced_by_relations = Vec::new();
    let mut evidenced_by_payload_values = Vec::new();
    for evidence in &verification_state.evidence {
        let relation_id = RelationId::new_v7();
        let relation_version_id = RelationVersionId::new_v7();
        let relation_operation_id = OperationId::new_v7();
        let relation_payload_value =
            relation_transition_payload_value(relation_id, None, relation_version_id)?;
        let relation_payload_json = canonical_json_string(&relation_payload_value)?;
        evidenced_by_payload_values.push(relation_payload_value);
        evidenced_by_relations.push(VerificationEvidenceRelationRows {
            evidence_id: evidence.evidence_id,
            relation_operation_id,
            relation_id,
            relation_version_id,
            relation_state_json: evidenced_by_relation_state_json.clone(),
            relation_state_digest: evidenced_by_relation_state_digest,
            relation_payload_json,
        });
    }
    let next_work_state = work_state_after_verification_create(
        &parent.state,
        verification_entity_id,
        verification_entity_version_id,
        verifies_relation_id,
        verifies_relation_version_id,
        &evidenced_by_relations,
    )?;
    let work_state_digest = work_state_mapping_digest(&next_work_state);
    let verification_payload_value = entity_transition_payload_value(
        verification_entity_id,
        None,
        verification_entity_version_id,
    )?;
    let verifies_relation_payload_value = relation_transition_payload_value(
        verifies_relation_id,
        None,
        verifies_relation_version_id,
    )?;
    let verification_payload_json = canonical_json_string(&verification_payload_value)?;
    let verifies_relation_payload_json = canonical_json_string(&verifies_relation_payload_value)?;
    let mut operation_payload_values =
        vec![verification_payload_value, verifies_relation_payload_value];
    operation_payload_values.extend(evidenced_by_payload_values);
    let changeset_payload_json = canonical_json_string(&CanonicalValue::object(vec![(
        "operations".to_owned(),
        CanonicalValue::Array(operation_payload_values),
    )])?)?;
    let rationale_json = canonical_json_string(&options.rationale)?;

    let transaction = connection
        .inner_mut()
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(storage_error)?;
    let branch = load_active_branch(&transaction, options.branch_id)?;
    if branch.head_commit_id != options.expected_head_commit_id {
        return Err(WorkVcsError::BranchHeadConflict(format!(
            "branch {} expected head {}, found {}",
            options.branch_id, options.expected_head_commit_id, branch.head_commit_id
        )));
    }
    if branch.workspace_id != parent.workspace_id {
        return Err(WorkVcsError::TaskInvalid(format!(
            "branch {} belongs to workspace {}, but expected head {} belongs to workspace {}",
            options.branch_id,
            branch.workspace_id,
            options.expected_head_commit_id,
            parent.workspace_id
        )));
    }
    write_verification_create(
        &transaction,
        &VerificationCreateRows {
            workspace_id: branch.workspace_id,
            expected_head_commit_id: options.expected_head_commit_id,
            verification_entity_id,
            verification_entity_version_id,
            verification_state_json,
            verification_state_digest,
            verifies_relation_id,
            verifies_relation_version_id,
            verifies_relation_state_json,
            verifies_relation_state_digest,
            evidenced_by_relations: evidenced_by_relations.clone(),
            target: options.target,
            basis_json,
            semantic_dependencies: verification_state.semantic_dependencies.clone(),
            resource_basis: verification_state.resource_basis.clone(),
            changeset_id,
            commit_id,
            verification_operation_id,
            verifies_relation_operation_id,
            verification_payload_json,
            verifies_relation_payload_json,
            changeset_payload_json,
            rationale_json,
            work_state_digest,
            now_us,
        },
    )?;
    move_branch_head(
        &transaction,
        options.branch_id,
        options.expected_head_commit_id,
        commit_id,
        now_us,
    )?;
    transaction.commit().map_err(storage_error)?;

    Ok(VerificationCreateCommit {
        workspace_id: branch.workspace_id,
        branch_id: options.branch_id,
        previous_head_commit_id: options.expected_head_commit_id,
        commit_id,
        changeset_id,
        verification_operation_id,
        verifies_relation_operation_id,
        verification_entity_id,
        verification_entity_version_id,
        verification_state_digest,
        verifies_relation_id,
        verifies_relation_version_id,
        verifies_relation_state_digest,
        evidenced_by_relations: evidenced_by_relations
            .into_iter()
            .map(|relation| VerificationEvidenceRelationCreate {
                evidence_id: relation.evidence_id,
                relation_operation_id: relation.relation_operation_id,
                relation_id: relation.relation_id,
                relation_version_id: relation.relation_version_id,
                relation_state_digest: relation.relation_state_digest,
            })
            .collect(),
        work_state_digest,
        target: options.target,
        state: verification_state,
    })
}

pub(crate) fn verification_at(
    connection: &StoreConnection,
    commit_id: CommitId,
    verification_entity_id: EntityId,
) -> Result<VerificationSnapshot> {
    let replayed = state_at(connection, commit_id)?;
    let Some(verification_entity_version_id) =
        replayed
            .state
            .entities()
            .iter()
            .find_map(|(entity_id, entity_version_id)| {
                (*entity_id == verification_entity_id).then_some(*entity_version_id)
            })
    else {
        return Err(WorkVcsError::TaskNotFound(format!(
            "verification entity {verification_entity_id} is not present at commit {commit_id}"
        )));
    };

    let loaded = load_verification_version(
        connection,
        replayed.workspace_id,
        verification_entity_id,
        verification_entity_version_id,
    )?;
    let defining_relation = load_current_verifies_relation_for_source(
        connection,
        replayed.workspace_id,
        &replayed.state,
        verification_entity_id,
    )?;
    let evidenced_by_relations = load_current_evidenced_by_relations_for_source(
        connection,
        replayed.workspace_id,
        &replayed.state,
        verification_entity_id,
    )?;
    require_verification_evidence_closure(
        verification_entity_id,
        &loaded.state.evidence,
        &evidenced_by_relations,
    )?;
    Ok(VerificationSnapshot {
        workspace_id: replayed.workspace_id,
        commit_id,
        verification_entity_id,
        verification_entity_version_id,
        state_digest: loaded.state_digest,
        verifies_relation_id: defining_relation.relation_id,
        verifies_relation_version_id: defining_relation.relation_version_id,
        verifies_relation_state_digest: defining_relation.state_digest,
        evidenced_by_relations,
        target: defining_relation.target,
        state: loaded.state,
    })
}

pub(crate) fn verifications_at(
    connection: &StoreConnection,
    commit_id: CommitId,
) -> Result<Vec<VerificationSnapshot>> {
    let replayed = state_at(connection, commit_id)?;
    let mut verifications = Vec::new();

    for (entity_id, entity_version_id) in replayed.state.entities() {
        match load_entity_kind_for_public_boundary(connection, *entity_id)? {
            Some(entity_kind) if entity_kind == VERIFICATION_ENTITY_KIND => {
                let loaded = load_verification_version(
                    connection,
                    replayed.workspace_id,
                    *entity_id,
                    *entity_version_id,
                )?;
                let defining_relation = load_current_verifies_relation_for_source(
                    connection,
                    replayed.workspace_id,
                    &replayed.state,
                    *entity_id,
                )?;
                let evidenced_by_relations = load_current_evidenced_by_relations_for_source(
                    connection,
                    replayed.workspace_id,
                    &replayed.state,
                    *entity_id,
                )?;
                require_verification_evidence_closure(
                    *entity_id,
                    &loaded.state.evidence,
                    &evidenced_by_relations,
                )?;
                verifications.push(VerificationSnapshot {
                    workspace_id: replayed.workspace_id,
                    commit_id,
                    verification_entity_id: *entity_id,
                    verification_entity_version_id: *entity_version_id,
                    state_digest: loaded.state_digest,
                    verifies_relation_id: defining_relation.relation_id,
                    verifies_relation_version_id: defining_relation.relation_version_id,
                    verifies_relation_state_digest: defining_relation.state_digest,
                    evidenced_by_relations,
                    target: defining_relation.target,
                    state: loaded.state,
                });
            }
            Some(_) => {}
            None => {
                return Err(WorkVcsError::TaskInvalid(format!(
                    "WorkState at commit {commit_id} references missing entity {entity_id}"
                )));
            }
        }
    }

    verifications.sort_by_key(|verification| {
        (
            verification_target_sort_key(verification.target),
            verification.verification_entity_id,
        )
    });
    Ok(verifications)
}

pub(crate) fn record_verification_applicability(
    connection: &mut StoreConnection,
    options: &VerificationApplicabilityRecordOptions,
) -> Result<VerificationApplicabilityCacheSnapshot> {
    connection.verify_foreign_keys()?;
    require_object("verification applicability detail", &options.detail)?;
    validate_applicability_resource_stamp_inputs(&options.resource_stamps)?;

    let replayed = state_at(connection, options.evaluated_commit_id)?;
    let verification = verification_at(
        connection,
        options.evaluated_commit_id,
        options.verification_entity_id,
    )?;
    validate_applicability_resource_stamp_inputs_against_basis(
        connection,
        &verification.state.resource_basis,
        &options.resource_stamps,
    )?;
    let (applicability, reason_code) = compute_recorded_verification_applicability(
        &replayed.state,
        &verification.state,
        &options.resource_stamps,
    )?;
    validate_local_key("verification applicability reason_code", reason_code)?;
    let detail_json = canonical_json_string(&options.detail)?;
    let evaluated_at_us = current_epoch_micros()?;

    let transaction = connection
        .inner_mut()
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(storage_error)?;
    let branch = load_active_branch(&transaction, options.branch_id)?;
    if branch.workspace_id != verification.workspace_id {
        return Err(WorkVcsError::TaskInvalid(format!(
            "branch {} belongs to workspace {}, but verification {} belongs to workspace {}",
            options.branch_id,
            branch.workspace_id,
            options.verification_entity_id,
            verification.workspace_id
        )));
    }
    if branch.head_commit_id != options.evaluated_commit_id {
        return Err(WorkVcsError::BranchHeadConflict(format!(
            "branch {} expected head {}, found {}",
            options.branch_id, options.evaluated_commit_id, branch.head_commit_id
        )));
    }
    write_verification_applicability_cache(
        &transaction,
        options,
        applicability,
        reason_code,
        &detail_json,
        evaluated_at_us,
    )?;
    transaction.commit().map_err(storage_error)?;

    verification_applicability_cache(
        connection,
        options.branch_id,
        options.verification_entity_id,
    )?
    .ok_or_else(|| {
        WorkVcsError::TaskInvalid(format!(
            "verification applicability cache for branch {} verification {} was not persisted",
            options.branch_id, options.verification_entity_id
        ))
    })
}

pub(crate) fn verification_applicability_cache(
    connection: &StoreConnection,
    branch_id: BranchId,
    verification_entity_id: EntityId,
) -> Result<Option<VerificationApplicabilityCacheSnapshot>> {
    let row = connection
        .inner()
        .query_row(
            "SELECT evaluated_commit_id,
                    applicability,
                    reason_code,
                    detail_json,
                    evaluated_at_us
             FROM verification_applicability_cache
             WHERE branch_id = ?1
               AND verification_entity_id = ?2",
            params![
                &branch_id.raw_bytes()[..],
                &verification_entity_id.raw_bytes()[..]
            ],
            |row| {
                Ok((
                    row.get::<_, Vec<u8>>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, i64>(4)?,
                ))
            },
        )
        .optional()
        .map_err(storage_error)?;

    let Some((evaluated_commit_id, applicability, reason_code, detail_json, evaluated_at_us)) = row
    else {
        return Ok(None);
    };
    let evaluated_commit_id = decode_commit_id(
        "verification_applicability_cache.evaluated_commit_id",
        evaluated_commit_id,
    )?;
    let applicability = VerificationApplicability::parse(&applicability)?;
    validate_local_key("verification_applicability_cache.reason_code", &reason_code)?;
    let detail =
        parse_canonical_object_json("verification_applicability_cache.detail_json", &detail_json)?;
    if evaluated_at_us < 0 {
        return Err(WorkVcsError::TaskInvalid(format!(
            "verification applicability cache for branch {branch_id} verification {verification_entity_id} has negative evaluated_at_us"
        )));
    }

    let replayed = state_at(connection, evaluated_commit_id)?;
    let verification = verification_at(connection, evaluated_commit_id, verification_entity_id)?;
    let resource_stamps =
        load_applicability_resource_stamps(connection, branch_id, verification_entity_id)?;
    validate_applicability_resource_stamp_snapshots_against_basis(
        connection,
        &verification.state.resource_basis,
        &resource_stamps,
    )?;
    let (expected_applicability, expected_reason_code) = compute_cached_verification_applicability(
        &replayed.state,
        &verification.state,
        &resource_stamps,
    )?;
    if applicability != expected_applicability || reason_code != expected_reason_code {
        return Err(WorkVcsError::TaskInvalid(format!(
            "verification applicability cache for branch {branch_id} verification {verification_entity_id} does not match basis stamps"
        )));
    }

    Ok(Some(VerificationApplicabilityCacheSnapshot {
        branch_id,
        verification_entity_id,
        evaluated_commit_id,
        applicability,
        reason_code,
        detail,
        evaluated_at_us,
        resource_stamps,
    }))
}

pub(crate) fn verification_applicability_caches(
    connection: &StoreConnection,
    options: &VerificationApplicabilityCacheListOptions,
) -> Result<VerificationApplicabilityCacheListResult> {
    let verification_entity_ids =
        verification_applicability_cache_verification_ids(connection, options)?;
    let mut caches = Vec::new();
    for verification_entity_id in verification_entity_ids {
        if let Some(cache) = verification_applicability_cache(
            connection,
            options.branch_id(),
            verification_entity_id,
        )? {
            caches.push(cache);
        }
    }
    Ok(VerificationApplicabilityCacheListResult {
        branch_id: options.branch_id(),
        caches,
    })
}

fn verification_applicability_cache_verification_ids(
    connection: &StoreConnection,
    options: &VerificationApplicabilityCacheListOptions,
) -> Result<Vec<EntityId>> {
    let branch_id = options.branch_id().raw_bytes();
    let mut verification_entity_ids = Vec::new();
    match (options.verification_entity_id(), options.applicability()) {
        (Some(verification_entity_id), Some(applicability)) => {
            let verification_entity_id = verification_entity_id.raw_bytes();
            let mut statement = connection
                .inner()
                .prepare(
                    "SELECT verification_entity_id
                     FROM verification_applicability_cache
                     WHERE branch_id = ?1
                       AND verification_entity_id = ?2
                       AND applicability = ?3
                     ORDER BY evaluated_at_us, verification_entity_id",
                )
                .map_err(storage_error)?;
            let rows = statement
                .query_map(
                    params![
                        &branch_id[..],
                        &verification_entity_id[..],
                        applicability.as_str()
                    ],
                    |row| row.get::<_, Vec<u8>>(0),
                )
                .map_err(storage_error)?;
            for row in rows {
                verification_entity_ids.push(decode_entity_id(
                    "verification_applicability_cache.verification_entity_id",
                    row.map_err(storage_error)?,
                )?);
            }
        }
        (Some(verification_entity_id), None) => {
            let verification_entity_id = verification_entity_id.raw_bytes();
            let mut statement = connection
                .inner()
                .prepare(
                    "SELECT verification_entity_id
                     FROM verification_applicability_cache
                     WHERE branch_id = ?1
                       AND verification_entity_id = ?2
                     ORDER BY evaluated_at_us, verification_entity_id",
                )
                .map_err(storage_error)?;
            let rows = statement
                .query_map(
                    params![&branch_id[..], &verification_entity_id[..]],
                    |row| row.get::<_, Vec<u8>>(0),
                )
                .map_err(storage_error)?;
            for row in rows {
                verification_entity_ids.push(decode_entity_id(
                    "verification_applicability_cache.verification_entity_id",
                    row.map_err(storage_error)?,
                )?);
            }
        }
        (None, Some(applicability)) => {
            let mut statement = connection
                .inner()
                .prepare(
                    "SELECT verification_entity_id
                     FROM verification_applicability_cache
                     WHERE branch_id = ?1
                       AND applicability = ?2
                     ORDER BY evaluated_at_us, verification_entity_id",
                )
                .map_err(storage_error)?;
            let rows = statement
                .query_map(params![&branch_id[..], applicability.as_str()], |row| {
                    row.get::<_, Vec<u8>>(0)
                })
                .map_err(storage_error)?;
            for row in rows {
                verification_entity_ids.push(decode_entity_id(
                    "verification_applicability_cache.verification_entity_id",
                    row.map_err(storage_error)?,
                )?);
            }
        }
        (None, None) => {
            let mut statement = connection
                .inner()
                .prepare(
                    "SELECT verification_entity_id
                     FROM verification_applicability_cache
                     WHERE branch_id = ?1
                     ORDER BY evaluated_at_us, verification_entity_id",
                )
                .map_err(storage_error)?;
            let rows = statement
                .query_map(params![&branch_id[..]], |row| row.get::<_, Vec<u8>>(0))
                .map_err(storage_error)?;
            for row in rows {
                verification_entity_ids.push(decode_entity_id(
                    "verification_applicability_cache.verification_entity_id",
                    row.map_err(storage_error)?,
                )?);
            }
        }
    }
    Ok(verification_entity_ids)
}

pub(crate) fn acceptance_criterion_effective_status(
    connection: &StoreConnection,
    commit_id: CommitId,
    acceptance_criterion_entity_id: EntityId,
) -> Result<AcceptanceCriterionEffectiveStatus> {
    acceptance_criterion_effective_status_at(
        connection,
        commit_id,
        None,
        acceptance_criterion_entity_id,
    )
}

pub(crate) fn acceptance_criterion_effective_status_for_branch(
    connection: &StoreConnection,
    branch_id: BranchId,
    acceptance_criterion_entity_id: EntityId,
) -> Result<AcceptanceCriterionEffectiveStatus> {
    let branch = branch_head(connection, branch_id)?;
    acceptance_criterion_effective_status_at(
        connection,
        branch.head_commit_id,
        Some(branch_id),
        acceptance_criterion_entity_id,
    )
}

fn acceptance_criterion_effective_status_for_branch_commit(
    connection: &StoreConnection,
    branch_id: BranchId,
    commit_id: CommitId,
    acceptance_criterion_entity_id: EntityId,
) -> Result<AcceptanceCriterionEffectiveStatus> {
    let branch = branch_head(connection, branch_id)?;
    if branch.head_commit_id != commit_id {
        return Err(WorkVcsError::BranchHeadConflict(format!(
            "branch {branch_id} expected head {commit_id}, found {}",
            branch.head_commit_id
        )));
    }
    acceptance_criterion_effective_status_at(
        connection,
        commit_id,
        Some(branch_id),
        acceptance_criterion_entity_id,
    )
}

fn acceptance_criterion_effective_status_at(
    connection: &StoreConnection,
    commit_id: CommitId,
    branch_id: Option<BranchId>,
    acceptance_criterion_entity_id: EntityId,
) -> Result<AcceptanceCriterionEffectiveStatus> {
    let criterion = acceptance_criterion_at(connection, commit_id, acceptance_criterion_entity_id)?;
    if criterion.state.verification_requirements.is_empty() {
        return effective_status_for_target(
            connection,
            commit_id,
            branch_id,
            VerificationTarget::AcceptanceCriterion(acceptance_criterion_entity_id),
        );
    }

    let mut combined = AcceptanceCriterionEffectiveStatus::Verified;
    for requirement_ref in &criterion.state.verification_requirements {
        let requirement = verification_requirement_at(
            connection,
            commit_id,
            requirement_ref.verification_requirement_entity_id,
        )?;
        if requirement.acceptance_criterion_entity_id != acceptance_criterion_entity_id
            || requirement.local_key != requirement_ref.local_key
        {
            return Err(WorkVcsError::TaskInvalid(format!(
                "acceptance criterion {} verification requirement reference {:?} does not match stored identity",
                acceptance_criterion_entity_id, requirement_ref.local_key
            )));
        }
        let status = effective_status_for_target(
            connection,
            commit_id,
            branch_id,
            VerificationTarget::VerificationRequirement(
                requirement_ref.verification_requirement_entity_id,
            ),
        )?;
        combined = combine_acceptance_criterion_status(combined, status);
    }
    Ok(combined)
}

struct LoadedTaskVersion {
    state_digest: Digest,
    state: TaskState,
}

struct LoadedAcceptanceCriterionVersion {
    task_entity_id: EntityId,
    local_key: String,
    state_digest: Digest,
    state: AcceptanceCriterionState,
}

struct LoadedVerificationRequirementVersion {
    acceptance_criterion_entity_id: EntityId,
    local_key: String,
    state_digest: Digest,
    state: VerificationRequirementState,
}

struct LoadedVerificationVersion {
    state_digest: Digest,
    state: VerificationState,
}

#[derive(Debug, PartialEq, Eq)]
struct ParsedVerificationBasis {
    verified_at_commit_id: CommitId,
    semantic_dependencies: Vec<VerificationSemanticDependency>,
    resource_basis: Vec<VerificationResourceBasis>,
}

struct LoadedVerifiesRelationVersion {
    relation_id: RelationId,
    relation_version_id: RelationVersionId,
    source_verification_entity_id: EntityId,
    state_digest: Digest,
    target: VerificationTarget,
}

struct LoadedEvidenceRelationVersion {
    relation_id: RelationId,
    relation_version_id: RelationVersionId,
    source_verification_entity_id: EntityId,
    evidence_id: EvidenceId,
    state_digest: Digest,
}

struct LoadedTaskSchedulingRelationVersion {
    relation_id: RelationId,
    relation_version_id: RelationVersionId,
    relation_type: TaskSchedulingRelationType,
    source_task_entity_id: EntityId,
    target_task_entity_id: EntityId,
    state_digest: Digest,
}

struct BranchRow {
    workspace_id: WorkspaceId,
    head_commit_id: CommitId,
}

struct AcceptanceCriterionCreateRows {
    workspace_id: WorkspaceId,
    expected_head_commit_id: CommitId,
    task_entity_id: EntityId,
    previous_task_entity_version_id: EntityVersionId,
    task_entity_version_id: EntityVersionId,
    task_state_json: String,
    task_state_digest: Digest,
    acceptance_criterion_entity_id: EntityId,
    acceptance_criterion_entity_version_id: EntityVersionId,
    acceptance_criterion_state_json: String,
    acceptance_criterion_state_digest: Digest,
    local_key: String,
    changeset_id: ChangeSetId,
    commit_id: CommitId,
    acceptance_criterion_operation_id: OperationId,
    task_operation_id: OperationId,
    acceptance_criterion_payload_json: String,
    task_payload_json: String,
    changeset_payload_json: String,
    rationale_json: String,
    work_state_digest: Digest,
    now_us: i64,
}

struct VerificationRequirementCreateRows {
    workspace_id: WorkspaceId,
    expected_head_commit_id: CommitId,
    acceptance_criterion_entity_id: EntityId,
    previous_acceptance_criterion_entity_version_id: EntityVersionId,
    acceptance_criterion_entity_version_id: EntityVersionId,
    acceptance_criterion_state_json: String,
    acceptance_criterion_state_digest: Digest,
    verification_requirement_entity_id: EntityId,
    verification_requirement_entity_version_id: EntityVersionId,
    verification_requirement_state_json: String,
    verification_requirement_state_digest: Digest,
    local_key: String,
    changeset_id: ChangeSetId,
    commit_id: CommitId,
    verification_requirement_operation_id: OperationId,
    acceptance_criterion_operation_id: OperationId,
    verification_requirement_payload_json: String,
    acceptance_criterion_payload_json: String,
    changeset_payload_json: String,
    rationale_json: String,
    work_state_digest: Digest,
    now_us: i64,
}

struct VerificationCreateRows {
    workspace_id: WorkspaceId,
    expected_head_commit_id: CommitId,
    verification_entity_id: EntityId,
    verification_entity_version_id: EntityVersionId,
    verification_state_json: String,
    verification_state_digest: Digest,
    verifies_relation_id: RelationId,
    verifies_relation_version_id: RelationVersionId,
    verifies_relation_state_json: String,
    verifies_relation_state_digest: Digest,
    evidenced_by_relations: Vec<VerificationEvidenceRelationRows>,
    target: VerificationTarget,
    basis_json: String,
    semantic_dependencies: Vec<VerificationSemanticDependency>,
    resource_basis: Vec<VerificationResourceBasis>,
    changeset_id: ChangeSetId,
    commit_id: CommitId,
    verification_operation_id: OperationId,
    verifies_relation_operation_id: OperationId,
    verification_payload_json: String,
    verifies_relation_payload_json: String,
    changeset_payload_json: String,
    rationale_json: String,
    work_state_digest: Digest,
    now_us: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct VerificationEvidenceRelationRows {
    evidence_id: EvidenceId,
    relation_operation_id: OperationId,
    relation_id: RelationId,
    relation_version_id: RelationVersionId,
    relation_state_json: String,
    relation_state_digest: Digest,
    relation_payload_json: String,
}

struct TaskSchedulingRelationCreateRows {
    workspace_id: WorkspaceId,
    expected_head_commit_id: CommitId,
    relation_id: RelationId,
    relation_version_id: RelationVersionId,
    relation_state_json: String,
    relation_state_digest: Digest,
    relation_type: TaskSchedulingRelationType,
    source_task_entity_id: EntityId,
    target_task_entity_id: EntityId,
    changeset_id: ChangeSetId,
    commit_id: CommitId,
    operation_id: OperationId,
    relation_payload_json: String,
    rationale_json: String,
    work_state_digest: Digest,
    now_us: i64,
}

fn load_task_version(
    connection: &StoreConnection,
    workspace_id: WorkspaceId,
    task_entity_id: EntityId,
    task_entity_version_id: EntityVersionId,
) -> Result<LoadedTaskVersion> {
    let row = connection
        .inner()
        .query_row(
            "SELECT object_identity.object_kind,
                    entity.workspace_id,
                    entity.entity_kind,
                    entity_version.state_schema_version,
                    entity_version.state_json,
                    entity_version.state_digest
             FROM entity
             JOIN object_identity
               ON object_identity.object_id = entity.object_id
             JOIN entity_version
               ON entity_version.entity_id = entity.object_id
             WHERE entity.object_id = ?1
               AND entity_version.entity_version_id = ?2",
            params![
                &task_entity_id.raw_bytes()[..],
                &task_entity_version_id.raw_bytes()[..]
            ],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, Vec<u8>>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, Vec<u8>>(5)?,
                ))
            },
        )
        .optional()
        .map_err(storage_error)?;

    let Some((
        object_kind,
        entity_workspace_id,
        entity_kind,
        state_schema_version,
        state_json,
        state_digest,
    )) = row
    else {
        return Err(WorkVcsError::TaskNotFound(format!(
            "task entity {task_entity_id} version {task_entity_version_id} does not exist"
        )));
    };
    if object_kind != ENTITY_OBJECT_KIND {
        return Err(WorkVcsError::TaskInvalid(format!(
            "task entity {task_entity_id} has object kind {object_kind:?}"
        )));
    }
    if entity_kind != TASK_ENTITY_KIND {
        return Err(WorkVcsError::TaskNotFound(format!(
            "entity {task_entity_id} has kind {entity_kind:?}, not {TASK_ENTITY_KIND:?}"
        )));
    }
    if state_schema_version != TASK_STATE_SCHEMA_VERSION {
        return Err(WorkVcsError::TaskInvalid(format!(
            "task entity {task_entity_id} version {task_entity_version_id} has state schema version {state_schema_version}"
        )));
    }
    let entity_workspace_id = decode_workspace_id("entity.workspace_id", entity_workspace_id)?;
    if entity_workspace_id != workspace_id {
        return Err(WorkVcsError::TaskInvalid(format!(
            "task entity {task_entity_id} belongs to workspace {entity_workspace_id}, not {workspace_id}"
        )));
    }

    let state_digest = decode_digest("entity_version.state_digest", state_digest)?;
    validate_import_fixed_point(
        state_json.as_bytes(),
        state_digest,
        ImportDigestDomain::EntityVersion,
    )
    .map_err(|error| {
        WorkVcsError::TaskInvalid(format!(
            "task entity {task_entity_id} version {task_entity_version_id} fixed-point validation failed: {error}"
        ))
    })?;
    let value = parse_canonical_json(state_json.as_bytes()).map_err(task_invalid_from)?;
    let actual = entity_version_digest(&value).map_err(task_invalid_from)?;
    if actual != state_digest {
        return Err(WorkVcsError::TaskInvalid(format!(
            "task entity {task_entity_id} version {task_entity_version_id} digest does not match state JSON"
        )));
    }
    Ok(LoadedTaskVersion {
        state_digest,
        state: parse_task_state(value)?,
    })
}

fn load_acceptance_criterion_version(
    connection: &StoreConnection,
    workspace_id: WorkspaceId,
    acceptance_criterion_entity_id: EntityId,
    acceptance_criterion_entity_version_id: EntityVersionId,
) -> Result<LoadedAcceptanceCriterionVersion> {
    let row = connection
        .inner()
        .query_row(
            "SELECT object_identity.object_kind,
                    entity.workspace_id,
                    entity.entity_kind,
                    acceptance_criterion_identity.owner_entity_id,
                    acceptance_criterion_identity.local_key,
                    owner_entity.workspace_id,
                    owner_entity.entity_kind,
                    entity_version.state_schema_version,
                    entity_version.state_json,
                    entity_version.state_digest
             FROM entity
             JOIN object_identity
               ON object_identity.object_id = entity.object_id
             JOIN acceptance_criterion_identity
               ON acceptance_criterion_identity.entity_id = entity.object_id
             JOIN entity AS owner_entity
               ON owner_entity.object_id = acceptance_criterion_identity.owner_entity_id
             JOIN entity_version
               ON entity_version.entity_id = entity.object_id
             WHERE entity.object_id = ?1
               AND entity_version.entity_version_id = ?2",
            params![
                &acceptance_criterion_entity_id.raw_bytes()[..],
                &acceptance_criterion_entity_version_id.raw_bytes()[..]
            ],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, Vec<u8>>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, Vec<u8>>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, Vec<u8>>(5)?,
                    row.get::<_, String>(6)?,
                    row.get::<_, i64>(7)?,
                    row.get::<_, String>(8)?,
                    row.get::<_, Vec<u8>>(9)?,
                ))
            },
        )
        .optional()
        .map_err(storage_error)?;

    let Some((
        object_kind,
        entity_workspace_id,
        entity_kind,
        owner_entity_id,
        local_key,
        owner_workspace_id,
        owner_entity_kind,
        state_schema_version,
        state_json,
        state_digest,
    )) = row
    else {
        return Err(WorkVcsError::TaskNotFound(format!(
            "acceptance criterion entity {acceptance_criterion_entity_id} version {acceptance_criterion_entity_version_id} does not exist"
        )));
    };
    if object_kind != ENTITY_OBJECT_KIND {
        return Err(WorkVcsError::TaskInvalid(format!(
            "acceptance criterion entity {acceptance_criterion_entity_id} has object kind {object_kind:?}"
        )));
    }
    if entity_kind != ACCEPTANCE_CRITERION_ENTITY_KIND {
        return Err(WorkVcsError::TaskNotFound(format!(
            "entity {acceptance_criterion_entity_id} has kind {entity_kind:?}, not {ACCEPTANCE_CRITERION_ENTITY_KIND:?}"
        )));
    }
    if owner_entity_kind != TASK_ENTITY_KIND {
        return Err(WorkVcsError::TaskInvalid(format!(
            "acceptance criterion entity {acceptance_criterion_entity_id} owner has kind {owner_entity_kind:?}, not {TASK_ENTITY_KIND:?}"
        )));
    }
    if state_schema_version != ACCEPTANCE_CRITERION_STATE_SCHEMA_VERSION {
        return Err(WorkVcsError::TaskInvalid(format!(
            "acceptance criterion entity {acceptance_criterion_entity_id} version {acceptance_criterion_entity_version_id} has state schema version {state_schema_version}"
        )));
    }
    validate_local_key("acceptance criterion local key", &local_key)?;

    let entity_workspace_id = decode_workspace_id("entity.workspace_id", entity_workspace_id)?;
    if entity_workspace_id != workspace_id {
        return Err(WorkVcsError::TaskInvalid(format!(
            "acceptance criterion entity {acceptance_criterion_entity_id} belongs to workspace {entity_workspace_id}, not {workspace_id}"
        )));
    }
    let owner_workspace_id = decode_workspace_id("owner_entity.workspace_id", owner_workspace_id)?;
    if owner_workspace_id != workspace_id {
        return Err(WorkVcsError::TaskInvalid(format!(
            "acceptance criterion entity {acceptance_criterion_entity_id} owner belongs to workspace {owner_workspace_id}, not {workspace_id}"
        )));
    }

    let state_digest = decode_digest("entity_version.state_digest", state_digest)?;
    validate_import_fixed_point(
        state_json.as_bytes(),
        state_digest,
        ImportDigestDomain::EntityVersion,
    )
    .map_err(|error| {
        WorkVcsError::TaskInvalid(format!(
            "acceptance criterion entity {acceptance_criterion_entity_id} version {acceptance_criterion_entity_version_id} fixed-point validation failed: {error}"
        ))
    })?;
    let value = parse_canonical_json(state_json.as_bytes()).map_err(task_invalid_from)?;
    let actual = entity_version_digest(&value).map_err(task_invalid_from)?;
    if actual != state_digest {
        return Err(WorkVcsError::TaskInvalid(format!(
            "acceptance criterion entity {acceptance_criterion_entity_id} version {acceptance_criterion_entity_version_id} digest does not match state JSON"
        )));
    }

    Ok(LoadedAcceptanceCriterionVersion {
        task_entity_id: decode_entity_id(
            "acceptance_criterion_identity.owner_entity_id",
            owner_entity_id,
        )?,
        local_key,
        state_digest,
        state: parse_acceptance_criterion_state(value)?,
    })
}

fn load_verification_requirement_version(
    connection: &StoreConnection,
    workspace_id: WorkspaceId,
    verification_requirement_entity_id: EntityId,
    verification_requirement_entity_version_id: EntityVersionId,
) -> Result<LoadedVerificationRequirementVersion> {
    let row = connection
        .inner()
        .query_row(
            "SELECT object_identity.object_kind,
                    entity.workspace_id,
                    entity.entity_kind,
                    verification_requirement_identity.owner_entity_id,
                    verification_requirement_identity.local_key,
                    owner_entity.workspace_id,
                    owner_entity.entity_kind,
                    entity_version.state_schema_version,
                    entity_version.state_json,
                    entity_version.state_digest
             FROM entity
             JOIN object_identity
               ON object_identity.object_id = entity.object_id
             JOIN verification_requirement_identity
               ON verification_requirement_identity.entity_id = entity.object_id
             JOIN entity AS owner_entity
               ON owner_entity.object_id = verification_requirement_identity.owner_entity_id
             JOIN entity_version
               ON entity_version.entity_id = entity.object_id
             WHERE entity.object_id = ?1
               AND entity_version.entity_version_id = ?2",
            params![
                &verification_requirement_entity_id.raw_bytes()[..],
                &verification_requirement_entity_version_id.raw_bytes()[..]
            ],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, Vec<u8>>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, Vec<u8>>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, Vec<u8>>(5)?,
                    row.get::<_, String>(6)?,
                    row.get::<_, i64>(7)?,
                    row.get::<_, String>(8)?,
                    row.get::<_, Vec<u8>>(9)?,
                ))
            },
        )
        .optional()
        .map_err(storage_error)?;

    let Some((
        object_kind,
        entity_workspace_id,
        entity_kind,
        owner_entity_id,
        local_key,
        owner_workspace_id,
        owner_entity_kind,
        state_schema_version,
        state_json,
        state_digest,
    )) = row
    else {
        return Err(WorkVcsError::TaskNotFound(format!(
            "verification requirement entity {verification_requirement_entity_id} version {verification_requirement_entity_version_id} does not exist"
        )));
    };
    if object_kind != ENTITY_OBJECT_KIND {
        return Err(WorkVcsError::TaskInvalid(format!(
            "verification requirement entity {verification_requirement_entity_id} has object kind {object_kind:?}"
        )));
    }
    if entity_kind != VERIFICATION_REQUIREMENT_ENTITY_KIND {
        return Err(WorkVcsError::TaskNotFound(format!(
            "entity {verification_requirement_entity_id} has kind {entity_kind:?}, not {VERIFICATION_REQUIREMENT_ENTITY_KIND:?}"
        )));
    }
    if owner_entity_kind != ACCEPTANCE_CRITERION_ENTITY_KIND {
        return Err(WorkVcsError::TaskInvalid(format!(
            "verification requirement entity {verification_requirement_entity_id} owner has kind {owner_entity_kind:?}, not {ACCEPTANCE_CRITERION_ENTITY_KIND:?}"
        )));
    }
    if state_schema_version != VERIFICATION_REQUIREMENT_STATE_SCHEMA_VERSION {
        return Err(WorkVcsError::TaskInvalid(format!(
            "verification requirement entity {verification_requirement_entity_id} version {verification_requirement_entity_version_id} has state schema version {state_schema_version}"
        )));
    }
    validate_local_key("verification requirement local key", &local_key)?;

    let entity_workspace_id = decode_workspace_id("entity.workspace_id", entity_workspace_id)?;
    if entity_workspace_id != workspace_id {
        return Err(WorkVcsError::TaskInvalid(format!(
            "verification requirement entity {verification_requirement_entity_id} belongs to workspace {entity_workspace_id}, not {workspace_id}"
        )));
    }
    let owner_workspace_id = decode_workspace_id("owner_entity.workspace_id", owner_workspace_id)?;
    if owner_workspace_id != workspace_id {
        return Err(WorkVcsError::TaskInvalid(format!(
            "verification requirement entity {verification_requirement_entity_id} owner belongs to workspace {owner_workspace_id}, not {workspace_id}"
        )));
    }

    let state_digest = decode_digest("entity_version.state_digest", state_digest)?;
    validate_import_fixed_point(
        state_json.as_bytes(),
        state_digest,
        ImportDigestDomain::EntityVersion,
    )
    .map_err(|error| {
        WorkVcsError::TaskInvalid(format!(
            "verification requirement entity {verification_requirement_entity_id} version {verification_requirement_entity_version_id} fixed-point validation failed: {error}"
        ))
    })?;
    let value = parse_canonical_json(state_json.as_bytes()).map_err(task_invalid_from)?;
    let actual = entity_version_digest(&value).map_err(task_invalid_from)?;
    if actual != state_digest {
        return Err(WorkVcsError::TaskInvalid(format!(
            "verification requirement entity {verification_requirement_entity_id} version {verification_requirement_entity_version_id} digest does not match state JSON"
        )));
    }

    Ok(LoadedVerificationRequirementVersion {
        acceptance_criterion_entity_id: decode_entity_id(
            "verification_requirement_identity.owner_entity_id",
            owner_entity_id,
        )?,
        local_key,
        state_digest,
        state: parse_verification_requirement_state(value)?,
    })
}

fn load_verification_version(
    connection: &StoreConnection,
    workspace_id: WorkspaceId,
    verification_entity_id: EntityId,
    verification_entity_version_id: EntityVersionId,
) -> Result<LoadedVerificationVersion> {
    let row = connection
        .inner()
        .query_row(
            "SELECT object_identity.object_kind,
                    entity.workspace_id,
                    entity.entity_kind,
                    entity_version.state_schema_version,
                    entity_version.state_json,
                    entity_version.state_digest,
                    verification_basis.verified_at_commit_id,
                    verification_basis.basis_schema_version,
                    verification_basis.basis_json
             FROM entity
             JOIN object_identity
               ON object_identity.object_id = entity.object_id
             JOIN entity_version
               ON entity_version.entity_id = entity.object_id
             JOIN verification_basis
               ON verification_basis.verification_entity_id = entity.object_id
             WHERE entity.object_id = ?1
               AND entity_version.entity_version_id = ?2",
            params![
                &verification_entity_id.raw_bytes()[..],
                &verification_entity_version_id.raw_bytes()[..]
            ],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, Vec<u8>>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, Vec<u8>>(5)?,
                    row.get::<_, Vec<u8>>(6)?,
                    row.get::<_, i64>(7)?,
                    row.get::<_, String>(8)?,
                ))
            },
        )
        .optional()
        .map_err(storage_error)?;

    let Some((
        object_kind,
        entity_workspace_id,
        entity_kind,
        state_schema_version,
        state_json,
        state_digest,
        verified_at_commit_id,
        basis_schema_version,
        basis_json,
    )) = row
    else {
        return Err(WorkVcsError::TaskNotFound(format!(
            "verification entity {verification_entity_id} version {verification_entity_version_id} does not exist"
        )));
    };
    if object_kind != ENTITY_OBJECT_KIND {
        return Err(WorkVcsError::TaskInvalid(format!(
            "verification entity {verification_entity_id} has object kind {object_kind:?}"
        )));
    }
    if entity_kind != VERIFICATION_ENTITY_KIND {
        return Err(WorkVcsError::TaskNotFound(format!(
            "entity {verification_entity_id} has kind {entity_kind:?}, not {VERIFICATION_ENTITY_KIND:?}"
        )));
    }
    if state_schema_version != VERIFICATION_STATE_SCHEMA_VERSION {
        return Err(WorkVcsError::TaskInvalid(format!(
            "verification entity {verification_entity_id} version {verification_entity_version_id} has state schema version {state_schema_version}"
        )));
    }
    if basis_schema_version != VERIFICATION_BASIS_SCHEMA_VERSION {
        return Err(WorkVcsError::TaskInvalid(format!(
            "verification entity {verification_entity_id} basis has schema version {basis_schema_version}"
        )));
    }

    let entity_workspace_id = decode_workspace_id("entity.workspace_id", entity_workspace_id)?;
    if entity_workspace_id != workspace_id {
        return Err(WorkVcsError::TaskInvalid(format!(
            "verification entity {verification_entity_id} belongs to workspace {entity_workspace_id}, not {workspace_id}"
        )));
    }

    let state_digest = decode_digest("entity_version.state_digest", state_digest)?;
    validate_import_fixed_point(
        state_json.as_bytes(),
        state_digest,
        ImportDigestDomain::EntityVersion,
    )
    .map_err(|error| {
        WorkVcsError::TaskInvalid(format!(
            "verification entity {verification_entity_id} version {verification_entity_version_id} fixed-point validation failed: {error}"
        ))
    })?;
    let value = parse_canonical_json(state_json.as_bytes()).map_err(task_invalid_from)?;
    let actual = entity_version_digest(&value).map_err(task_invalid_from)?;
    if actual != state_digest {
        return Err(WorkVcsError::TaskInvalid(format!(
            "verification entity {verification_entity_id} version {verification_entity_version_id} digest does not match state JSON"
        )));
    }

    let dependencies = load_verification_semantic_dependencies(connection, verification_entity_id)?;
    let resource_basis = load_verification_resource_basis(connection, verification_entity_id)?;
    let state = parse_verification_state(value)?;
    let verified_at_commit_id = decode_commit_id(
        "verification_basis.verified_at_commit_id",
        verified_at_commit_id,
    )?;
    if state.verified_at_commit_id != verified_at_commit_id {
        return Err(WorkVcsError::TaskInvalid(format!(
            "verification entity {verification_entity_id} state basis commit does not match verification_basis"
        )));
    }
    if state.semantic_dependencies != dependencies {
        return Err(WorkVcsError::TaskInvalid(format!(
            "verification entity {verification_entity_id} state dependencies do not match verification_semantic_dependency rows"
        )));
    }
    if state.resource_basis != resource_basis {
        return Err(WorkVcsError::TaskInvalid(format!(
            "verification entity {verification_entity_id} state resource_basis does not match verification_resource_basis rows"
        )));
    }
    let basis_value = parse_canonical_json(basis_json.as_bytes()).map_err(task_invalid_from)?;
    let basis_json_actual = canonical_json_string(&basis_value)?;
    if basis_json_actual != basis_json {
        return Err(WorkVcsError::TaskInvalid(format!(
            "verification entity {verification_entity_id} basis_json is not canonical fixed-point JSON"
        )));
    }
    let parsed_basis = parse_verification_basis(basis_value)?;
    if parsed_basis.verified_at_commit_id != verified_at_commit_id
        || parsed_basis.semantic_dependencies != dependencies
        || parsed_basis.resource_basis != resource_basis
    {
        return Err(WorkVcsError::TaskInvalid(format!(
            "verification entity {verification_entity_id} basis_json does not match structured basis rows"
        )));
    }
    let expected_basis_json = canonical_json_string(&verification_basis_value(
        verified_at_commit_id,
        &dependencies,
        &resource_basis,
    )?)?;
    let legacy_expected_basis_json = canonical_json_string(&legacy_verification_basis_value(
        verified_at_commit_id,
        &dependencies,
    )?)?;
    if basis_json != expected_basis_json
        && !(resource_basis.is_empty() && basis_json == legacy_expected_basis_json)
    {
        return Err(WorkVcsError::TaskInvalid(format!(
            "verification entity {verification_entity_id} basis_json does not match dependency rows"
        )));
    }

    Ok(LoadedVerificationVersion {
        state_digest,
        state,
    })
}

fn load_verification_semantic_dependencies(
    connection: &StoreConnection,
    verification_entity_id: EntityId,
) -> Result<Vec<VerificationSemanticDependency>> {
    let mut statement = connection
        .inner()
        .prepare(
            "SELECT ordinal, dependency_entity_id, expected_entity_version_id
             FROM verification_semantic_dependency
             WHERE verification_entity_id = ?1
             ORDER BY ordinal",
        )
        .map_err(storage_error)?;
    let rows = statement
        .query_map(params![&verification_entity_id.raw_bytes()[..]], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, Vec<u8>>(1)?,
                row.get::<_, Vec<u8>>(2)?,
            ))
        })
        .map_err(storage_error)?;

    let mut dependencies = Vec::new();
    for row in rows {
        let (ordinal, dependency_entity_id, expected_entity_version_id) =
            row.map_err(storage_error)?;
        if ordinal != dependencies.len() as i64 {
            return Err(WorkVcsError::TaskInvalid(format!(
                "verification entity {verification_entity_id} semantic dependency ordinals are not contiguous"
            )));
        }
        dependencies.push(VerificationSemanticDependency::new(
            decode_entity_id(
                "verification_semantic_dependency.dependency_entity_id",
                dependency_entity_id,
            )?,
            decode_entity_version_id(
                "verification_semantic_dependency.expected_entity_version_id",
                expected_entity_version_id,
            )?,
        ));
    }
    validate_verification_semantic_dependencies(&dependencies)?;
    require_verification_semantic_dependencies_canonical_order(&dependencies)?;
    Ok(dependencies)
}

fn load_verification_resource_basis(
    connection: &StoreConnection,
    verification_entity_id: EntityId,
) -> Result<Vec<VerificationResourceBasis>> {
    let mut statement = connection
        .inner()
        .prepare(
            "SELECT ordinal,
                    resource_id,
                    adapter_kind,
                    adapter_schema_version,
                    scope_kind,
                    scope_schema_version,
                    scope_payload_json,
                    baseline_observation_id,
                    baseline_fingerprint
             FROM verification_resource_basis
             WHERE verification_entity_id = ?1
             ORDER BY ordinal",
        )
        .map_err(storage_error)?;
    let rows = statement
        .query_map(params![&verification_entity_id.raw_bytes()[..]], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, Vec<u8>>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, i64>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, i64>(5)?,
                row.get::<_, String>(6)?,
                row.get::<_, Option<Vec<u8>>>(7)?,
                row.get::<_, Vec<u8>>(8)?,
            ))
        })
        .map_err(storage_error)?;

    let mut resource_basis = Vec::new();
    for row in rows {
        let (
            ordinal,
            resource_id,
            adapter_kind,
            adapter_schema_version,
            scope_kind,
            scope_schema_version,
            scope_payload_json,
            baseline_observation_id,
            baseline_fingerprint,
        ) = row.map_err(storage_error)?;
        if ordinal != resource_basis.len() as i64 {
            return Err(WorkVcsError::TaskInvalid(format!(
                "verification entity {verification_entity_id} resource basis ordinals are not contiguous"
            )));
        }
        let scope_payload = parse_verification_resource_scope_payload_json(&scope_payload_json)?;
        let mut basis = VerificationResourceBasis::new(
            decode_resource_id("verification_resource_basis.resource_id", resource_id)?,
            adapter_kind,
            adapter_schema_version,
            scope_kind,
            scope_schema_version,
            scope_payload,
            decode_digest(
                "verification_resource_basis.baseline_fingerprint",
                baseline_fingerprint,
            )?,
        )?;
        basis.baseline_observation_id = baseline_observation_id
            .map(|bytes| {
                decode_resource_observation_id(
                    "verification_resource_basis.baseline_observation_id",
                    bytes,
                )
            })
            .transpose()?;
        validate_verification_resource_basis_entry(&basis)?;
        require_verification_resource_basis_references(connection, &basis)?;
        resource_basis.push(basis);
    }
    validate_verification_resource_basis(&resource_basis)?;
    Ok(resource_basis)
}

fn load_applicability_resource_stamps(
    connection: &StoreConnection,
    branch_id: BranchId,
    verification_entity_id: EntityId,
) -> Result<Vec<ApplicabilityResourceStampSnapshot>> {
    let mut statement = connection
        .inner()
        .prepare(
            "SELECT resource_basis_ordinal,
                    adapter_kind,
                    adapter_schema_version,
                    scope_schema_version,
                    observation_status,
                    observed_fingerprint,
                    observation_id,
                    observed_at_us
             FROM applicability_resource_stamp
             WHERE branch_id = ?1
               AND verification_entity_id = ?2
             ORDER BY resource_basis_ordinal",
        )
        .map_err(storage_error)?;
    let rows = statement
        .query_map(
            params![
                &branch_id.raw_bytes()[..],
                &verification_entity_id.raw_bytes()[..]
            ],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, Option<Vec<u8>>>(5)?,
                    row.get::<_, Option<Vec<u8>>>(6)?,
                    row.get::<_, i64>(7)?,
                ))
            },
        )
        .map_err(storage_error)?;

    let mut stamps = Vec::new();
    for row in rows {
        let (
            resource_basis_ordinal,
            adapter_kind,
            adapter_schema_version,
            scope_schema_version,
            observation_status,
            observed_fingerprint,
            observation_id,
            observed_at_us,
        ) = row.map_err(storage_error)?;
        if resource_basis_ordinal != stamps.len() as i64 {
            return Err(WorkVcsError::TaskInvalid(format!(
                "applicability resource stamps for branch {branch_id} verification {verification_entity_id} are not contiguous"
            )));
        }
        let stamp = ApplicabilityResourceStampSnapshot {
            resource_basis_ordinal,
            adapter_kind,
            adapter_schema_version,
            scope_schema_version,
            observation_status: ApplicabilityResourceObservationStatus::parse(&observation_status)?,
            observed_fingerprint: observed_fingerprint
                .map(|bytes| {
                    decode_digest("applicability_resource_stamp.observed_fingerprint", bytes)
                })
                .transpose()?,
            observation_id: observation_id
                .map(|bytes| {
                    decode_resource_observation_id(
                        "applicability_resource_stamp.observation_id",
                        bytes,
                    )
                })
                .transpose()?,
            observed_at_us,
        };
        validate_applicability_resource_stamp_snapshot(&stamp)?;
        stamps.push(stamp);
    }
    Ok(stamps)
}

fn load_verifies_relation_version(
    connection: &StoreConnection,
    workspace_id: WorkspaceId,
    relation_id: RelationId,
    relation_version_id: RelationVersionId,
) -> Result<Option<LoadedVerifiesRelationVersion>> {
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
                    relation_version.state_digest,
                    source_entity.entity_kind,
                    target_entity.entity_kind
             FROM relation
             JOIN object_identity
               ON object_identity.object_id = relation.object_id
             JOIN relation_version
               ON relation_version.relation_id = relation.object_id
             LEFT JOIN entity AS source_entity
               ON source_entity.object_id = relation.source_object_id
             LEFT JOIN entity AS target_entity
               ON target_entity.object_id = relation.target_object_id
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
                    row.get::<_, Option<String>>(9)?,
                    row.get::<_, Option<String>>(10)?,
                ))
            },
        )
        .optional()
        .map_err(storage_error)?;

    let Some((
        object_kind,
        relation_workspace_id,
        relation_type,
        source_object_id,
        target_object_id,
        relation_discriminator,
        state_schema_version,
        metadata_json,
        state_digest,
        source_entity_kind,
        target_entity_kind,
    )) = row
    else {
        return Err(WorkVcsError::TaskNotFound(format!(
            "verifies relation {relation_id} version {relation_version_id} does not exist"
        )));
    };
    if object_kind != RELATION_OBJECT_KIND {
        return Err(WorkVcsError::TaskInvalid(format!(
            "verifies relation {relation_id} has object kind {object_kind:?}"
        )));
    }
    let relation_workspace_id =
        decode_workspace_id("relation.workspace_id", relation_workspace_id)?;
    if relation_workspace_id != workspace_id {
        return Err(WorkVcsError::TaskInvalid(format!(
            "verifies relation {relation_id} belongs to workspace {relation_workspace_id}, not {workspace_id}"
        )));
    }
    if relation_type != VERIFIES_RELATION_TYPE {
        return Ok(None);
    }
    let source_entity_kind = source_entity_kind.ok_or_else(|| {
        WorkVcsError::TaskInvalid(format!(
            "verifies relation {relation_id} source is not an entity"
        ))
    })?;
    let target_entity_kind = target_entity_kind.ok_or_else(|| {
        WorkVcsError::TaskInvalid(format!(
            "verifies relation {relation_id} target is not an entity"
        ))
    })?;
    if !relation_discriminator.is_empty() {
        return Err(WorkVcsError::TaskInvalid(format!(
            "verifies relation {relation_id} has non-empty discriminator {relation_discriminator:?}"
        )));
    }
    if state_schema_version != RELATION_STATE_SCHEMA_VERSION {
        return Err(WorkVcsError::TaskInvalid(format!(
            "verifies relation {relation_id} version {relation_version_id} has state schema version {state_schema_version}"
        )));
    }
    if source_entity_kind != VERIFICATION_ENTITY_KIND {
        return Err(WorkVcsError::TaskInvalid(format!(
            "verifies relation {relation_id} source has kind {source_entity_kind:?}, not {VERIFICATION_ENTITY_KIND:?}"
        )));
    }
    let target_entity_id = decode_entity_id("relation.target_object_id", target_object_id)?;
    let target = match target_entity_kind.as_str() {
        ACCEPTANCE_CRITERION_ENTITY_KIND => {
            VerificationTarget::AcceptanceCriterion(target_entity_id)
        }
        VERIFICATION_REQUIREMENT_ENTITY_KIND => {
            VerificationTarget::VerificationRequirement(target_entity_id)
        }
        other => {
            return Err(WorkVcsError::TaskInvalid(format!(
                "verifies relation {relation_id} target has unsupported kind {other:?}"
            )));
        }
    };

    let state_digest = decode_digest("relation_version.state_digest", state_digest)?;
    validate_import_fixed_point(
        metadata_json.as_bytes(),
        state_digest,
        ImportDigestDomain::RelationVersion,
    )
    .map_err(|error| {
        WorkVcsError::TaskInvalid(format!(
            "verifies relation {relation_id} version {relation_version_id} fixed-point validation failed: {error}"
        ))
    })?;
    let metadata_value =
        parse_canonical_json(metadata_json.as_bytes()).map_err(task_invalid_from)?;
    let actual = relation_version_digest(&metadata_value).map_err(task_invalid_from)?;
    if actual != state_digest {
        return Err(WorkVcsError::TaskInvalid(format!(
            "verifies relation {relation_id} version {relation_version_id} digest does not match metadata JSON"
        )));
    }

    Ok(Some(LoadedVerifiesRelationVersion {
        relation_id,
        relation_version_id,
        source_verification_entity_id: decode_entity_id(
            "relation.source_object_id",
            source_object_id,
        )?,
        state_digest,
        target,
    }))
}

fn load_evidenced_by_relation_version(
    connection: &StoreConnection,
    workspace_id: WorkspaceId,
    relation_id: RelationId,
    relation_version_id: RelationVersionId,
) -> Result<Option<LoadedEvidenceRelationVersion>> {
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
                    relation_version.state_digest,
                    source_entity.entity_kind,
                    target_object.object_kind
             FROM relation
             JOIN object_identity
               ON object_identity.object_id = relation.object_id
             JOIN relation_version
               ON relation_version.relation_id = relation.object_id
             JOIN entity AS source_entity
               ON source_entity.object_id = relation.source_object_id
             JOIN object_identity AS target_object
               ON target_object.object_id = relation.target_object_id
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
                    row.get::<_, String>(9)?,
                    row.get::<_, String>(10)?,
                ))
            },
        )
        .optional()
        .map_err(storage_error)?;

    let Some((
        object_kind,
        relation_workspace_id,
        relation_type,
        source_object_id,
        target_object_id,
        relation_discriminator,
        state_schema_version,
        metadata_json,
        state_digest,
        source_entity_kind,
        target_object_kind,
    )) = row
    else {
        return Err(WorkVcsError::TaskNotFound(format!(
            "evidenced_by relation {relation_id} version {relation_version_id} does not exist"
        )));
    };
    if object_kind != RELATION_OBJECT_KIND {
        return Err(WorkVcsError::TaskInvalid(format!(
            "evidenced_by relation {relation_id} has object kind {object_kind:?}"
        )));
    }
    let relation_workspace_id =
        decode_workspace_id("relation.workspace_id", relation_workspace_id)?;
    if relation_workspace_id != workspace_id {
        return Err(WorkVcsError::TaskInvalid(format!(
            "evidenced_by relation {relation_id} belongs to workspace {relation_workspace_id}, not {workspace_id}"
        )));
    }
    if relation_type != EVIDENCED_BY_RELATION_TYPE {
        return Ok(None);
    }
    if !relation_discriminator.is_empty() {
        return Err(WorkVcsError::TaskInvalid(format!(
            "evidenced_by relation {relation_id} has non-empty discriminator {relation_discriminator:?}"
        )));
    }
    if state_schema_version != RELATION_STATE_SCHEMA_VERSION {
        return Err(WorkVcsError::TaskInvalid(format!(
            "evidenced_by relation {relation_id} version {relation_version_id} has state schema version {state_schema_version}"
        )));
    }
    if source_entity_kind != VERIFICATION_ENTITY_KIND {
        return Err(WorkVcsError::TaskInvalid(format!(
            "evidenced_by relation {relation_id} source has kind {source_entity_kind:?}, not {VERIFICATION_ENTITY_KIND:?}"
        )));
    }
    if target_object_kind != EVIDENCE_OBJECT_KIND {
        return Err(WorkVcsError::TaskInvalid(format!(
            "evidenced_by relation {relation_id} target has object kind {target_object_kind:?}, not {EVIDENCE_OBJECT_KIND:?}"
        )));
    }

    let state_digest = decode_digest("relation_version.state_digest", state_digest)?;
    validate_import_fixed_point(
        metadata_json.as_bytes(),
        state_digest,
        ImportDigestDomain::RelationVersion,
    )
    .map_err(|error| {
        WorkVcsError::TaskInvalid(format!(
            "evidenced_by relation {relation_id} version {relation_version_id} fixed-point validation failed: {error}"
        ))
    })?;
    let metadata_value =
        parse_canonical_json(metadata_json.as_bytes()).map_err(task_invalid_from)?;
    let actual = relation_version_digest(&metadata_value).map_err(task_invalid_from)?;
    if actual != state_digest {
        return Err(WorkVcsError::TaskInvalid(format!(
            "evidenced_by relation {relation_id} version {relation_version_id} digest does not match metadata JSON"
        )));
    }

    let evidence_id = decode_evidence_id("relation.target_object_id", target_object_id)?;
    require_evidence_exists(connection, evidence_id)?;

    Ok(Some(LoadedEvidenceRelationVersion {
        relation_id,
        relation_version_id,
        source_verification_entity_id: decode_entity_id(
            "relation.source_object_id",
            source_object_id,
        )?,
        evidence_id,
        state_digest,
    }))
}

fn load_task_scheduling_relation_version(
    connection: &StoreConnection,
    workspace_id: WorkspaceId,
    commit_id: CommitId,
    relation_id: RelationId,
    relation_version_id: RelationVersionId,
) -> Result<Option<LoadedTaskSchedulingRelationVersion>> {
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
        source_object_id,
        target_object_id,
        relation_discriminator,
        state_schema_version,
        metadata_json,
        state_digest,
    )) = row
    else {
        return Err(WorkVcsError::TaskNotFound(format!(
            "task scheduling relation {relation_id} version {relation_version_id} does not exist"
        )));
    };
    if object_kind != RELATION_OBJECT_KIND {
        return Err(WorkVcsError::TaskInvalid(format!(
            "task scheduling relation {relation_id} has object kind {object_kind:?}"
        )));
    }
    let relation_workspace_id =
        decode_workspace_id("relation.workspace_id", relation_workspace_id)?;
    if relation_workspace_id != workspace_id {
        return Err(WorkVcsError::TaskInvalid(format!(
            "task scheduling relation {relation_id} belongs to workspace {relation_workspace_id}, not {workspace_id}"
        )));
    }

    let Some(relation_type) = TaskSchedulingRelationType::parse(&relation_type) else {
        return Ok(None);
    };
    if !relation_discriminator.is_empty() {
        return Err(WorkVcsError::TaskInvalid(format!(
            "task scheduling relation {relation_id} has non-empty discriminator {relation_discriminator:?}"
        )));
    }
    if state_schema_version != RELATION_STATE_SCHEMA_VERSION {
        return Err(WorkVcsError::TaskInvalid(format!(
            "task scheduling relation {relation_id} version {relation_version_id} has state schema version {state_schema_version}"
        )));
    }

    let state_digest = decode_digest("relation_version.state_digest", state_digest)?;
    validate_import_fixed_point(
        metadata_json.as_bytes(),
        state_digest,
        ImportDigestDomain::RelationVersion,
    )
    .map_err(|error| {
        WorkVcsError::TaskInvalid(format!(
            "task scheduling relation {relation_id} version {relation_version_id} fixed-point validation failed: {error}"
        ))
    })?;
    let metadata_value =
        parse_canonical_json(metadata_json.as_bytes()).map_err(task_invalid_from)?;
    if metadata_value != CanonicalValue::object(Vec::new())? {
        return Err(WorkVcsError::TaskInvalid(format!(
            "task scheduling relation {relation_id} version {relation_version_id} state must be canonical empty object"
        )));
    }
    let actual = relation_version_digest(&metadata_value).map_err(task_invalid_from)?;
    if actual != state_digest {
        return Err(WorkVcsError::TaskInvalid(format!(
            "task scheduling relation {relation_id} version {relation_version_id} digest does not match metadata JSON"
        )));
    }

    let source_task_entity_id = decode_entity_id("relation.source_object_id", source_object_id)?;
    let target_task_entity_id = decode_entity_id("relation.target_object_id", target_object_id)?;
    let source = task_at(connection, commit_id, source_task_entity_id)?;
    let target = task_at(connection, commit_id, target_task_entity_id)?;
    if source.workspace_id != workspace_id || target.workspace_id != workspace_id {
        return Err(WorkVcsError::TaskInvalid(format!(
            "task scheduling relation {relation_id} endpoints must belong to workspace {workspace_id}"
        )));
    }

    Ok(Some(LoadedTaskSchedulingRelationVersion {
        relation_id,
        relation_version_id,
        relation_type,
        source_task_entity_id,
        target_task_entity_id,
        state_digest,
    }))
}

fn load_current_verifies_relation_for_source(
    connection: &StoreConnection,
    workspace_id: WorkspaceId,
    state: &WorkState,
    verification_entity_id: EntityId,
) -> Result<LoadedVerifiesRelationVersion> {
    let mut matches = Vec::new();
    for (relation_id, relation_version_id) in state.relations() {
        let Some(relation) = load_verifies_relation_version(
            connection,
            workspace_id,
            *relation_id,
            *relation_version_id,
        )?
        else {
            continue;
        };
        if relation.source_verification_entity_id == verification_entity_id {
            matches.push(relation);
        }
    }
    match matches.len() {
        1 => Ok(matches.remove(0)),
        0 => Err(WorkVcsError::TaskInvalid(format!(
            "verification entity {verification_entity_id} has no current defining verifies relation"
        ))),
        count => Err(WorkVcsError::TaskInvalid(format!(
            "verification entity {verification_entity_id} has {count} current defining verifies relations"
        ))),
    }
}

fn load_current_evidenced_by_relations_for_source(
    connection: &StoreConnection,
    workspace_id: WorkspaceId,
    state: &WorkState,
    verification_entity_id: EntityId,
) -> Result<Vec<VerificationEvidenceRelationSnapshot>> {
    let mut matches = Vec::new();
    for (relation_id, relation_version_id) in state.relations() {
        let Some(relation) = load_evidenced_by_relation_version(
            connection,
            workspace_id,
            *relation_id,
            *relation_version_id,
        )?
        else {
            continue;
        };
        if relation.source_verification_entity_id == verification_entity_id {
            matches.push(VerificationEvidenceRelationSnapshot {
                evidence_id: relation.evidence_id,
                relation_id: relation.relation_id,
                relation_version_id: relation.relation_version_id,
                state_digest: relation.state_digest,
            });
        }
    }
    matches.sort_by(|left, right| {
        left.evidence_id
            .cmp(&right.evidence_id)
            .then_with(|| left.relation_id.cmp(&right.relation_id))
    });
    Ok(matches)
}

fn require_verification_evidence_closure(
    verification_entity_id: EntityId,
    expected: &[VerificationEvidenceRef],
    relations: &[VerificationEvidenceRelationSnapshot],
) -> Result<()> {
    let expected = expected
        .iter()
        .map(|entry| entry.evidence_id)
        .collect::<Vec<_>>();
    let actual = relations
        .iter()
        .map(|relation| relation.evidence_id)
        .collect::<Vec<_>>();
    if expected == actual {
        Ok(())
    } else {
        Err(WorkVcsError::TaskInvalid(format!(
            "verification entity {verification_entity_id} evidence state does not match current evidenced_by relations"
        )))
    }
}

fn verification_target_sort_key(target: VerificationTarget) -> (u8, EntityId) {
    match target {
        VerificationTarget::AcceptanceCriterion(entity_id) => (0, entity_id),
        VerificationTarget::VerificationRequirement(entity_id) => (1, entity_id),
    }
}

fn load_active_branch(transaction: &Transaction<'_>, branch_id: BranchId) -> Result<BranchRow> {
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
        return Err(WorkVcsError::TaskInvalid(format!(
            "branch {branch_id} has lifecycle state {lifecycle_state:?}"
        )));
    }

    Ok(BranchRow {
        workspace_id: decode_workspace_id("branch.workspace_id", workspace_id)?,
        head_commit_id: decode_commit_id("branch.head_commit_id", head_commit_id)?,
    })
}

fn ensure_acceptance_criterion_local_key_available(
    transaction: &Transaction<'_>,
    task_entity_id: EntityId,
    local_key: &str,
) -> Result<()> {
    let existing = transaction
        .query_row(
            "SELECT count(*)
             FROM acceptance_criterion_identity
             WHERE owner_entity_id = ?1
               AND local_key = ?2",
            params![&task_entity_id.raw_bytes()[..], local_key],
            |row| row.get::<_, i64>(0),
        )
        .map_err(storage_error)?;
    if existing == 0 {
        Ok(())
    } else {
        Err(WorkVcsError::TaskInvalid(format!(
            "task entity {task_entity_id} already has acceptance criterion local key {local_key:?}"
        )))
    }
}

fn ensure_verification_requirement_local_key_available(
    transaction: &Transaction<'_>,
    acceptance_criterion_entity_id: EntityId,
    local_key: &str,
) -> Result<()> {
    let existing = transaction
        .query_row(
            "SELECT count(*)
             FROM verification_requirement_identity
             WHERE owner_entity_id = ?1
               AND local_key = ?2",
            params![&acceptance_criterion_entity_id.raw_bytes()[..], local_key],
            |row| row.get::<_, i64>(0),
        )
        .map_err(storage_error)?;
    if existing == 0 {
        Ok(())
    } else {
        Err(WorkVcsError::TaskInvalid(format!(
            "acceptance criterion entity {acceptance_criterion_entity_id} already has verification requirement local key {local_key:?}"
        )))
    }
}

fn ensure_task_scheduling_relation_logical_key_available(
    transaction: &Transaction<'_>,
    workspace_id: WorkspaceId,
    relation_type: TaskSchedulingRelationType,
    source_task_entity_id: EntityId,
    target_task_entity_id: EntityId,
) -> Result<()> {
    let existing = transaction
        .query_row(
            "SELECT count(*)
             FROM relation
             WHERE workspace_id = ?1
               AND relation_type = ?2
               AND source_object_id = ?3
               AND target_object_id = ?4
               AND relation_discriminator = ''",
            params![
                &workspace_id.raw_bytes()[..],
                relation_type.as_str(),
                &source_task_entity_id.raw_bytes()[..],
                &target_task_entity_id.raw_bytes()[..],
            ],
            |row| row.get::<_, i64>(0),
        )
        .map_err(storage_error)?;
    if existing == 0 {
        Ok(())
    } else {
        Err(WorkVcsError::TaskInvalid(format!(
            "task scheduling relation {relation_type} from {source_task_entity_id} to {target_task_entity_id} already exists in workspace {workspace_id}"
        )))
    }
}

fn work_state_after_acceptance_criterion_create(
    parent_state: &WorkState,
    task_entity_id: EntityId,
    expected_task_entity_version_id: EntityVersionId,
    next_task_entity_version_id: EntityVersionId,
    acceptance_criterion_entity_id: EntityId,
    acceptance_criterion_entity_version_id: EntityVersionId,
) -> Result<WorkState> {
    let mut entities = parent_state
        .entities()
        .iter()
        .copied()
        .collect::<BTreeMap<_, _>>();
    match entities.insert(task_entity_id, next_task_entity_version_id) {
        Some(current) if current == expected_task_entity_version_id => {}
        Some(current) => {
            return Err(WorkVcsError::TaskInvalid(format!(
                "task entity {task_entity_id} expected parent version {expected_task_entity_version_id}, found {current}"
            )));
        }
        None => {
            return Err(WorkVcsError::TaskNotFound(format!(
                "task entity {task_entity_id} is not present in the parent WorkState"
            )));
        }
    }
    if entities
        .insert(
            acceptance_criterion_entity_id,
            acceptance_criterion_entity_version_id,
        )
        .is_some()
    {
        return Err(WorkVcsError::TaskInvalid(format!(
            "acceptance criterion entity {acceptance_criterion_entity_id} was expected to be absent before creation"
        )));
    }

    WorkState::new(entities, parent_state.relations().to_vec()).map_err(task_invalid_from)
}

fn work_state_after_verification_requirement_create(
    parent_state: &WorkState,
    acceptance_criterion_entity_id: EntityId,
    expected_acceptance_criterion_entity_version_id: EntityVersionId,
    next_acceptance_criterion_entity_version_id: EntityVersionId,
    verification_requirement_entity_id: EntityId,
    verification_requirement_entity_version_id: EntityVersionId,
) -> Result<WorkState> {
    let mut entities = parent_state
        .entities()
        .iter()
        .copied()
        .collect::<BTreeMap<_, _>>();
    match entities.insert(
        acceptance_criterion_entity_id,
        next_acceptance_criterion_entity_version_id,
    ) {
        Some(current) if current == expected_acceptance_criterion_entity_version_id => {}
        Some(current) => {
            return Err(WorkVcsError::TaskInvalid(format!(
                "acceptance criterion entity {acceptance_criterion_entity_id} expected parent version {expected_acceptance_criterion_entity_version_id}, found {current}"
            )));
        }
        None => {
            return Err(WorkVcsError::TaskNotFound(format!(
                "acceptance criterion entity {acceptance_criterion_entity_id} is not present in the parent WorkState"
            )));
        }
    }
    if entities
        .insert(
            verification_requirement_entity_id,
            verification_requirement_entity_version_id,
        )
        .is_some()
    {
        return Err(WorkVcsError::TaskInvalid(format!(
            "verification requirement entity {verification_requirement_entity_id} was expected to be absent before creation"
        )));
    }

    WorkState::new(entities, parent_state.relations().to_vec()).map_err(task_invalid_from)
}

fn work_state_after_verification_create(
    parent_state: &WorkState,
    verification_entity_id: EntityId,
    verification_entity_version_id: EntityVersionId,
    verifies_relation_id: RelationId,
    verifies_relation_version_id: RelationVersionId,
    evidenced_by_relations: &[VerificationEvidenceRelationRows],
) -> Result<WorkState> {
    let mut entities = parent_state
        .entities()
        .iter()
        .copied()
        .collect::<BTreeMap<_, _>>();
    if entities
        .insert(verification_entity_id, verification_entity_version_id)
        .is_some()
    {
        return Err(WorkVcsError::TaskInvalid(format!(
            "verification entity {verification_entity_id} was expected to be absent before creation"
        )));
    }

    let mut relations = parent_state
        .relations()
        .iter()
        .copied()
        .collect::<BTreeMap<_, _>>();
    if relations
        .insert(verifies_relation_id, verifies_relation_version_id)
        .is_some()
    {
        return Err(WorkVcsError::TaskInvalid(format!(
            "verifies relation {verifies_relation_id} was expected to be absent before creation"
        )));
    }
    for relation in evidenced_by_relations {
        if relations
            .insert(relation.relation_id, relation.relation_version_id)
            .is_some()
        {
            return Err(WorkVcsError::TaskInvalid(format!(
                "evidenced_by relation {} was expected to be absent before creation",
                relation.relation_id
            )));
        }
    }

    WorkState::new(entities, relations).map_err(task_invalid_from)
}

fn work_state_after_task_scheduling_relation_create(
    parent_state: &WorkState,
    relation_id: RelationId,
    relation_version_id: RelationVersionId,
) -> Result<WorkState> {
    let entities = parent_state.entities().to_vec();
    let mut relations = parent_state
        .relations()
        .iter()
        .copied()
        .collect::<BTreeMap<_, _>>();
    if relations.insert(relation_id, relation_version_id).is_some() {
        return Err(WorkVcsError::TaskInvalid(format!(
            "task scheduling relation {relation_id} was expected to be absent before creation"
        )));
    }

    WorkState::new(entities, relations).map_err(task_invalid_from)
}

fn write_acceptance_criterion_create(
    transaction: &Transaction<'_>,
    rows: &AcceptanceCriterionCreateRows,
) -> Result<()> {
    let workspace_id_bytes = rows.workspace_id.raw_bytes();
    let task_entity_id_bytes = rows.task_entity_id.raw_bytes();
    let previous_task_entity_version_id_bytes = rows.previous_task_entity_version_id.raw_bytes();
    let task_entity_version_id_bytes = rows.task_entity_version_id.raw_bytes();
    let task_state_digest_bytes = rows.task_state_digest.as_bytes();
    let acceptance_criterion_entity_id_bytes = rows.acceptance_criterion_entity_id.raw_bytes();
    let acceptance_criterion_entity_version_id_bytes =
        rows.acceptance_criterion_entity_version_id.raw_bytes();
    let acceptance_criterion_state_digest_bytes = rows.acceptance_criterion_state_digest.as_bytes();
    let changeset_id_bytes = rows.changeset_id.raw_bytes();
    let commit_id_bytes = rows.commit_id.raw_bytes();
    let acceptance_criterion_operation_id_bytes =
        rows.acceptance_criterion_operation_id.raw_bytes();
    let task_operation_id_bytes = rows.task_operation_id.raw_bytes();
    let parent_commit_id_bytes = rows.expected_head_commit_id.raw_bytes();
    let work_state_digest_bytes = rows.work_state_digest.as_bytes();

    transaction
        .execute(
            "INSERT INTO object_identity(object_id, object_kind, created_at_us)
             VALUES (?1, ?2, ?3)",
            params![
                &acceptance_criterion_entity_id_bytes[..],
                ENTITY_OBJECT_KIND,
                rows.now_us
            ],
        )
        .map_err(storage_error)?;
    transaction
        .execute(
            "INSERT INTO entity(object_id, workspace_id, entity_kind)
             VALUES (?1, ?2, ?3)",
            params![
                &acceptance_criterion_entity_id_bytes[..],
                &workspace_id_bytes[..],
                ACCEPTANCE_CRITERION_ENTITY_KIND
            ],
        )
        .map_err(storage_error)?;
    transaction
        .execute(
            "INSERT INTO acceptance_criterion_identity(entity_id, owner_entity_id, local_key)
             VALUES (?1, ?2, ?3)",
            params![
                &acceptance_criterion_entity_id_bytes[..],
                &task_entity_id_bytes[..],
                rows.local_key
            ],
        )
        .map_err(storage_error)?;
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
                &acceptance_criterion_entity_version_id_bytes[..],
                &acceptance_criterion_entity_id_bytes[..],
                ACCEPTANCE_CRITERION_STATE_SCHEMA_VERSION,
                rows.acceptance_criterion_state_json,
                &acceptance_criterion_state_digest_bytes[..]
            ],
        )
        .map_err(storage_error)?;
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
                &task_entity_version_id_bytes[..],
                &task_entity_id_bytes[..],
                TASK_STATE_SCHEMA_VERSION,
                rows.task_state_json,
                &task_state_digest_bytes[..]
            ],
        )
        .map_err(storage_error)?;
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
                ENTITY_TRANSITION_OPERATION_TYPE,
                ENTITY_TRANSITION_OPERATION_SCHEMA_VERSION,
                rows.changeset_payload_json,
                rows.rationale_json,
                rows.now_us
            ],
        )
        .map_err(storage_error)?;
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
             VALUES (?1, ?2, 0, 'entity', ?3, ?4)",
            params![
                &acceptance_criterion_operation_id_bytes[..],
                &changeset_id_bytes[..],
                &acceptance_criterion_entity_id_bytes[..],
                rows.acceptance_criterion_payload_json
            ],
        )
        .map_err(storage_error)?;
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
             VALUES (?1, ?2, 1, 'entity', ?3, ?4)",
            params![
                &task_operation_id_bytes[..],
                &changeset_id_bytes[..],
                &task_entity_id_bytes[..],
                rows.task_payload_json
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
             VALUES (?1, ?2, NULL, ?3, ?4)",
            params![
                &acceptance_criterion_operation_id_bytes[..],
                &acceptance_criterion_entity_id_bytes[..],
                &acceptance_criterion_entity_version_id_bytes[..],
                EMPTY_FIELD_DELTA
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
                &task_operation_id_bytes[..],
                &task_entity_id_bytes[..],
                &previous_task_entity_version_id_bytes[..],
                &task_entity_version_id_bytes[..],
                EMPTY_FIELD_DELTA
            ],
        )
        .map_err(storage_error)?;
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
                rows.now_us
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
                &parent_commit_id_bytes[..]
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
                ENTITY_TRANSITION_EVENT_KIND,
                rows.now_us,
                rows.changeset_payload_json
            ],
        )
        .map_err(storage_error)?;

    Ok(())
}

fn write_verification_requirement_create(
    transaction: &Transaction<'_>,
    rows: &VerificationRequirementCreateRows,
) -> Result<()> {
    let workspace_id_bytes = rows.workspace_id.raw_bytes();
    let acceptance_criterion_entity_id_bytes = rows.acceptance_criterion_entity_id.raw_bytes();
    let previous_acceptance_criterion_entity_version_id_bytes = rows
        .previous_acceptance_criterion_entity_version_id
        .raw_bytes();
    let acceptance_criterion_entity_version_id_bytes =
        rows.acceptance_criterion_entity_version_id.raw_bytes();
    let acceptance_criterion_state_digest_bytes = rows.acceptance_criterion_state_digest.as_bytes();
    let verification_requirement_entity_id_bytes =
        rows.verification_requirement_entity_id.raw_bytes();
    let verification_requirement_entity_version_id_bytes =
        rows.verification_requirement_entity_version_id.raw_bytes();
    let verification_requirement_state_digest_bytes =
        rows.verification_requirement_state_digest.as_bytes();
    let changeset_id_bytes = rows.changeset_id.raw_bytes();
    let commit_id_bytes = rows.commit_id.raw_bytes();
    let verification_requirement_operation_id_bytes =
        rows.verification_requirement_operation_id.raw_bytes();
    let acceptance_criterion_operation_id_bytes =
        rows.acceptance_criterion_operation_id.raw_bytes();
    let parent_commit_id_bytes = rows.expected_head_commit_id.raw_bytes();
    let work_state_digest_bytes = rows.work_state_digest.as_bytes();

    transaction
        .execute(
            "INSERT INTO object_identity(object_id, object_kind, created_at_us)
             VALUES (?1, ?2, ?3)",
            params![
                &verification_requirement_entity_id_bytes[..],
                ENTITY_OBJECT_KIND,
                rows.now_us
            ],
        )
        .map_err(storage_error)?;
    transaction
        .execute(
            "INSERT INTO entity(object_id, workspace_id, entity_kind)
             VALUES (?1, ?2, ?3)",
            params![
                &verification_requirement_entity_id_bytes[..],
                &workspace_id_bytes[..],
                VERIFICATION_REQUIREMENT_ENTITY_KIND
            ],
        )
        .map_err(storage_error)?;
    transaction
        .execute(
            "INSERT INTO verification_requirement_identity(entity_id, owner_entity_id, local_key)
             VALUES (?1, ?2, ?3)",
            params![
                &verification_requirement_entity_id_bytes[..],
                &acceptance_criterion_entity_id_bytes[..],
                rows.local_key
            ],
        )
        .map_err(storage_error)?;
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
                &verification_requirement_entity_version_id_bytes[..],
                &verification_requirement_entity_id_bytes[..],
                VERIFICATION_REQUIREMENT_STATE_SCHEMA_VERSION,
                rows.verification_requirement_state_json,
                &verification_requirement_state_digest_bytes[..]
            ],
        )
        .map_err(storage_error)?;
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
                &acceptance_criterion_entity_version_id_bytes[..],
                &acceptance_criterion_entity_id_bytes[..],
                ACCEPTANCE_CRITERION_STATE_SCHEMA_VERSION,
                rows.acceptance_criterion_state_json,
                &acceptance_criterion_state_digest_bytes[..]
            ],
        )
        .map_err(storage_error)?;
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
                ENTITY_TRANSITION_OPERATION_TYPE,
                ENTITY_TRANSITION_OPERATION_SCHEMA_VERSION,
                rows.changeset_payload_json,
                rows.rationale_json,
                rows.now_us
            ],
        )
        .map_err(storage_error)?;
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
             VALUES (?1, ?2, 0, 'entity', ?3, ?4)",
            params![
                &verification_requirement_operation_id_bytes[..],
                &changeset_id_bytes[..],
                &verification_requirement_entity_id_bytes[..],
                rows.verification_requirement_payload_json
            ],
        )
        .map_err(storage_error)?;
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
             VALUES (?1, ?2, 1, 'entity', ?3, ?4)",
            params![
                &acceptance_criterion_operation_id_bytes[..],
                &changeset_id_bytes[..],
                &acceptance_criterion_entity_id_bytes[..],
                rows.acceptance_criterion_payload_json
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
             VALUES (?1, ?2, NULL, ?3, ?4)",
            params![
                &verification_requirement_operation_id_bytes[..],
                &verification_requirement_entity_id_bytes[..],
                &verification_requirement_entity_version_id_bytes[..],
                EMPTY_FIELD_DELTA
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
                &acceptance_criterion_operation_id_bytes[..],
                &acceptance_criterion_entity_id_bytes[..],
                &previous_acceptance_criterion_entity_version_id_bytes[..],
                &acceptance_criterion_entity_version_id_bytes[..],
                EMPTY_FIELD_DELTA
            ],
        )
        .map_err(storage_error)?;
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
                rows.now_us
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
                &parent_commit_id_bytes[..]
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
                ENTITY_TRANSITION_EVENT_KIND,
                rows.now_us,
                rows.changeset_payload_json
            ],
        )
        .map_err(storage_error)?;

    Ok(())
}

fn write_verification_create(
    transaction: &Transaction<'_>,
    rows: &VerificationCreateRows,
) -> Result<()> {
    let workspace_id_bytes = rows.workspace_id.raw_bytes();
    let verification_entity_id_bytes = rows.verification_entity_id.raw_bytes();
    let verification_entity_version_id_bytes = rows.verification_entity_version_id.raw_bytes();
    let verification_state_digest_bytes = rows.verification_state_digest.as_bytes();
    let verifies_relation_id_bytes = rows.verifies_relation_id.raw_bytes();
    let verifies_relation_version_id_bytes = rows.verifies_relation_version_id.raw_bytes();
    let verifies_relation_state_digest_bytes = rows.verifies_relation_state_digest.as_bytes();
    let target_entity_id_bytes = rows.target.entity_id().raw_bytes();
    let changeset_id_bytes = rows.changeset_id.raw_bytes();
    let commit_id_bytes = rows.commit_id.raw_bytes();
    let verification_operation_id_bytes = rows.verification_operation_id.raw_bytes();
    let verifies_relation_operation_id_bytes = rows.verifies_relation_operation_id.raw_bytes();
    let parent_commit_id_bytes = rows.expected_head_commit_id.raw_bytes();
    let work_state_digest_bytes = rows.work_state_digest.as_bytes();

    transaction
        .execute(
            "INSERT INTO object_identity(object_id, object_kind, created_at_us)
             VALUES (?1, ?2, ?3)",
            params![
                &verification_entity_id_bytes[..],
                ENTITY_OBJECT_KIND,
                rows.now_us
            ],
        )
        .map_err(storage_error)?;
    transaction
        .execute(
            "INSERT INTO entity(object_id, workspace_id, entity_kind)
             VALUES (?1, ?2, ?3)",
            params![
                &verification_entity_id_bytes[..],
                &workspace_id_bytes[..],
                VERIFICATION_ENTITY_KIND
            ],
        )
        .map_err(storage_error)?;
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
                &verification_entity_version_id_bytes[..],
                &verification_entity_id_bytes[..],
                VERIFICATION_STATE_SCHEMA_VERSION,
                rows.verification_state_json,
                &verification_state_digest_bytes[..]
            ],
        )
        .map_err(storage_error)?;
    transaction
        .execute(
            "INSERT INTO verification_basis(
                verification_entity_id,
                verified_at_commit_id,
                basis_schema_version,
                basis_json
             )
             VALUES (?1, ?2, ?3, ?4)",
            params![
                &verification_entity_id_bytes[..],
                &parent_commit_id_bytes[..],
                VERIFICATION_BASIS_SCHEMA_VERSION,
                rows.basis_json
            ],
        )
        .map_err(storage_error)?;
    for (ordinal, dependency) in rows.semantic_dependencies.iter().enumerate() {
        transaction
            .execute(
                "INSERT INTO verification_semantic_dependency(
                    verification_entity_id,
                    ordinal,
                    dependency_entity_id,
                    expected_entity_version_id
                 )
                 VALUES (?1, ?2, ?3, ?4)",
                params![
                    &verification_entity_id_bytes[..],
                    ordinal as i64,
                    &dependency.entity_id.raw_bytes()[..],
                    &dependency.entity_version_id.raw_bytes()[..]
                ],
            )
            .map_err(storage_error)?;
    }
    for (ordinal, resource_basis) in rows.resource_basis.iter().enumerate() {
        let scope_payload_json = canonical_json_string(&resource_basis.scope_payload)?;
        let baseline_observation_id_bytes = resource_basis
            .baseline_observation_id
            .map(|id| id.raw_bytes());
        transaction
            .execute(
                "INSERT INTO verification_resource_basis(
                    verification_entity_id,
                    ordinal,
                    resource_id,
                    adapter_kind,
                    adapter_schema_version,
                    scope_kind,
                    scope_schema_version,
                    scope_payload_json,
                    baseline_observation_id,
                    baseline_fingerprint
                 )
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![
                    &verification_entity_id_bytes[..],
                    ordinal as i64,
                    &resource_basis.resource_id.raw_bytes()[..],
                    resource_basis.adapter_kind.as_str(),
                    resource_basis.adapter_schema_version,
                    resource_basis.scope_kind.as_str(),
                    resource_basis.scope_schema_version,
                    scope_payload_json,
                    baseline_observation_id_bytes
                        .as_ref()
                        .map(|bytes| &bytes[..]),
                    &resource_basis.baseline_fingerprint.as_bytes()[..],
                ],
            )
            .map_err(storage_error)?;
    }
    transaction
        .execute(
            "INSERT INTO object_identity(object_id, object_kind, created_at_us)
             VALUES (?1, ?2, ?3)",
            params![
                &verifies_relation_id_bytes[..],
                RELATION_OBJECT_KIND,
                rows.now_us
            ],
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
             VALUES (?1, ?2, ?3, ?4, ?5, '')",
            params![
                &verifies_relation_id_bytes[..],
                &workspace_id_bytes[..],
                VERIFIES_RELATION_TYPE,
                &verification_entity_id_bytes[..],
                &target_entity_id_bytes[..]
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
                &verifies_relation_version_id_bytes[..],
                &verifies_relation_id_bytes[..],
                RELATION_STATE_SCHEMA_VERSION,
                rows.verifies_relation_state_json,
                &verifies_relation_state_digest_bytes[..]
            ],
        )
        .map_err(storage_error)?;
    for relation in &rows.evidenced_by_relations {
        let relation_id_bytes = relation.relation_id.raw_bytes();
        let relation_version_id_bytes = relation.relation_version_id.raw_bytes();
        let relation_state_digest_bytes = relation.relation_state_digest.as_bytes();
        let evidence_id_bytes = relation.evidence_id.raw_bytes();
        transaction
            .execute(
                "INSERT INTO object_identity(object_id, object_kind, created_at_us)
                 VALUES (?1, ?2, ?3)",
                params![&relation_id_bytes[..], RELATION_OBJECT_KIND, rows.now_us],
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
                 VALUES (?1, ?2, ?3, ?4, ?5, '')",
                params![
                    &relation_id_bytes[..],
                    &workspace_id_bytes[..],
                    EVIDENCED_BY_RELATION_TYPE,
                    &verification_entity_id_bytes[..],
                    &evidence_id_bytes[..]
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
                    relation.relation_state_json,
                    &relation_state_digest_bytes[..]
                ],
            )
            .map_err(storage_error)?;
    }
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
                VERIFICATION_RECORD_OPERATION_TYPE,
                VERIFICATION_RECORD_OPERATION_SCHEMA_VERSION,
                rows.changeset_payload_json,
                rows.rationale_json,
                rows.now_us
            ],
        )
        .map_err(storage_error)?;
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
             VALUES (?1, ?2, 0, 'entity', ?3, ?4)",
            params![
                &verification_operation_id_bytes[..],
                &changeset_id_bytes[..],
                &verification_entity_id_bytes[..],
                rows.verification_payload_json
            ],
        )
        .map_err(storage_error)?;
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
             VALUES (?1, ?2, 1, 'relation', ?3, ?4)",
            params![
                &verifies_relation_operation_id_bytes[..],
                &changeset_id_bytes[..],
                &verifies_relation_id_bytes[..],
                rows.verifies_relation_payload_json
            ],
        )
        .map_err(storage_error)?;
    for (index, relation) in rows.evidenced_by_relations.iter().enumerate() {
        let relation_id_bytes = relation.relation_id.raw_bytes();
        let operation_id_bytes = relation.relation_operation_id.raw_bytes();
        let ordinal = i64::try_from(index + 2).map_err(|_| {
            WorkVcsError::TaskInvalid("evidenced_by operation ordinal is too large".to_owned())
        })?;
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
                    ordinal,
                    &relation_id_bytes[..],
                    relation.relation_payload_json
                ],
            )
            .map_err(storage_error)?;
    }
    transaction
        .execute(
            "INSERT INTO entity_membership_change(
                operation_id,
                entity_id,
                before_entity_version_id,
                after_entity_version_id,
                field_delta_json
             )
             VALUES (?1, ?2, NULL, ?3, ?4)",
            params![
                &verification_operation_id_bytes[..],
                &verification_entity_id_bytes[..],
                &verification_entity_version_id_bytes[..],
                EMPTY_FIELD_DELTA
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
             VALUES (?1, ?2, NULL, ?3, ?4)",
            params![
                &verifies_relation_operation_id_bytes[..],
                &verifies_relation_id_bytes[..],
                &verifies_relation_version_id_bytes[..],
                EMPTY_FIELD_DELTA
            ],
        )
        .map_err(storage_error)?;
    for relation in &rows.evidenced_by_relations {
        let relation_id_bytes = relation.relation_id.raw_bytes();
        let relation_version_id_bytes = relation.relation_version_id.raw_bytes();
        let operation_id_bytes = relation.relation_operation_id.raw_bytes();
        transaction
            .execute(
                "INSERT INTO relation_membership_change(
                    operation_id,
                    relation_id,
                    before_relation_version_id,
                    after_relation_version_id,
                    field_delta_json
                 )
                 VALUES (?1, ?2, NULL, ?3, ?4)",
                params![
                    &operation_id_bytes[..],
                    &relation_id_bytes[..],
                    &relation_version_id_bytes[..],
                    EMPTY_FIELD_DELTA
                ],
            )
            .map_err(storage_error)?;
    }
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
                rows.now_us
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
                &parent_commit_id_bytes[..]
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
                VERIFICATION_RECORD_EVENT_KIND,
                rows.now_us,
                rows.changeset_payload_json
            ],
        )
        .map_err(storage_error)?;

    Ok(())
}

fn write_verification_applicability_cache(
    transaction: &Transaction<'_>,
    options: &VerificationApplicabilityRecordOptions,
    applicability: VerificationApplicability,
    reason_code: &str,
    detail_json: &str,
    evaluated_at_us: i64,
) -> Result<()> {
    let branch_id_bytes = options.branch_id.raw_bytes();
    let verification_entity_id_bytes = options.verification_entity_id.raw_bytes();
    let evaluated_commit_id_bytes = options.evaluated_commit_id.raw_bytes();

    transaction
        .execute(
            "INSERT INTO verification_applicability_cache(
                branch_id,
                verification_entity_id,
                evaluated_commit_id,
                applicability,
                reason_code,
                detail_json,
                evaluated_at_us
             )
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT(branch_id, verification_entity_id) DO UPDATE SET
                evaluated_commit_id = excluded.evaluated_commit_id,
                applicability = excluded.applicability,
                reason_code = excluded.reason_code,
                detail_json = excluded.detail_json,
                evaluated_at_us = excluded.evaluated_at_us",
            params![
                &branch_id_bytes[..],
                &verification_entity_id_bytes[..],
                &evaluated_commit_id_bytes[..],
                applicability.as_str(),
                reason_code,
                detail_json,
                evaluated_at_us
            ],
        )
        .map_err(storage_error)?;
    transaction
        .execute(
            "DELETE FROM applicability_resource_stamp
             WHERE branch_id = ?1
               AND verification_entity_id = ?2",
            params![&branch_id_bytes[..], &verification_entity_id_bytes[..]],
        )
        .map_err(storage_error)?;
    for stamp in &options.resource_stamps {
        let observed_fingerprint_bytes = stamp
            .observed_fingerprint
            .map(|fingerprint| *fingerprint.as_bytes());
        let observation_id_bytes = stamp.observation_id.map(|id| id.raw_bytes());
        transaction
            .execute(
                "INSERT INTO applicability_resource_stamp(
                    branch_id,
                    verification_entity_id,
                    resource_basis_ordinal,
                    adapter_kind,
                    adapter_schema_version,
                    scope_schema_version,
                    observation_status,
                    observed_fingerprint,
                    observation_id,
                    observed_at_us
                 )
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![
                    &branch_id_bytes[..],
                    &verification_entity_id_bytes[..],
                    stamp.resource_basis_ordinal,
                    stamp.adapter_kind.as_str(),
                    stamp.adapter_schema_version,
                    stamp.scope_schema_version,
                    stamp.observation_status.as_str(),
                    observed_fingerprint_bytes.as_ref().map(|bytes| &bytes[..]),
                    observation_id_bytes.as_ref().map(|bytes| &bytes[..]),
                    evaluated_at_us
                ],
            )
            .map_err(storage_error)?;
    }
    Ok(())
}

fn write_task_scheduling_relation_create(
    transaction: &Transaction<'_>,
    rows: &TaskSchedulingRelationCreateRows,
) -> Result<()> {
    let workspace_id_bytes = rows.workspace_id.raw_bytes();
    let relation_id_bytes = rows.relation_id.raw_bytes();
    let relation_version_id_bytes = rows.relation_version_id.raw_bytes();
    let relation_state_digest_bytes = rows.relation_state_digest.as_bytes();
    let source_task_entity_id_bytes = rows.source_task_entity_id.raw_bytes();
    let target_task_entity_id_bytes = rows.target_task_entity_id.raw_bytes();
    let changeset_id_bytes = rows.changeset_id.raw_bytes();
    let commit_id_bytes = rows.commit_id.raw_bytes();
    let operation_id_bytes = rows.operation_id.raw_bytes();
    let parent_commit_id_bytes = rows.expected_head_commit_id.raw_bytes();
    let work_state_digest_bytes = rows.work_state_digest.as_bytes();

    transaction
        .execute(
            "INSERT INTO object_identity(object_id, object_kind, created_at_us)
             VALUES (?1, ?2, ?3)",
            params![&relation_id_bytes[..], RELATION_OBJECT_KIND, rows.now_us],
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
             VALUES (?1, ?2, ?3, ?4, ?5, '')",
            params![
                &relation_id_bytes[..],
                &workspace_id_bytes[..],
                rows.relation_type.as_str(),
                &source_task_entity_id_bytes[..],
                &target_task_entity_id_bytes[..]
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
                rows.relation_state_json,
                &relation_state_digest_bytes[..]
            ],
        )
        .map_err(storage_error)?;
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
                TASK_SCHEDULING_RELATION_CREATE_OPERATION_TYPE,
                TASK_SCHEDULING_RELATION_CREATE_OPERATION_SCHEMA_VERSION,
                rows.relation_payload_json,
                rows.rationale_json,
                rows.now_us
            ],
        )
        .map_err(storage_error)?;
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
             VALUES (?1, ?2, 0, 'relation', ?3, ?4)",
            params![
                &operation_id_bytes[..],
                &changeset_id_bytes[..],
                &relation_id_bytes[..],
                rows.relation_payload_json
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
             VALUES (?1, ?2, NULL, ?3, ?4)",
            params![
                &operation_id_bytes[..],
                &relation_id_bytes[..],
                &relation_version_id_bytes[..],
                EMPTY_FIELD_DELTA
            ],
        )
        .map_err(storage_error)?;
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
                rows.now_us
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
                &parent_commit_id_bytes[..]
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
                TASK_SCHEDULING_RELATION_CREATE_EVENT_KIND,
                rows.now_us,
                rows.relation_payload_json
            ],
        )
        .map_err(storage_error)?;

    Ok(())
}

fn move_branch_head(
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
    if moved == 1 {
        super::mark_branch_projection_not_materialized(transaction, branch_id, updated_at_us)?;
        Ok(())
    } else {
        Err(WorkVcsError::BranchHeadConflict(format!(
            "branch {branch_id} head changed before commit {commit_id} could be installed"
        )))
    }
}

fn load_entity_kind_for_public_boundary(
    connection: &StoreConnection,
    entity_id: EntityId,
) -> Result<Option<String>> {
    connection
        .inner()
        .query_row(
            "SELECT entity_kind
             FROM entity
             WHERE object_id = ?1",
            params![&entity_id.raw_bytes()[..]],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(storage_error)
}

fn is_reserved_semantic_entity_kind(entity_kind: &str) -> bool {
    matches!(
        entity_kind,
        GOAL_ENTITY_KIND
            | KNOWLEDGE_ENTITY_KIND
            | PLAN_ENTITY_KIND
            | TASK_ENTITY_KIND
            | RECORD_ENTITY_KIND
            | ACCEPTANCE_CRITERION_ENTITY_KIND
            | VERIFICATION_REQUIREMENT_ENTITY_KIND
            | VERIFICATION_ENTITY_KIND
    )
}

fn reserved_semantic_entity_transition_error(entity_kind: &str) -> WorkVcsError {
    WorkVcsError::EntityTransitionInvalid(format!(
        "entity kind {entity_kind:?} must use its Engine semantic API"
    ))
}

fn parse_task_state(value: CanonicalValue) -> Result<TaskState> {
    let CanonicalValue::Object(entries) = value else {
        return Err(WorkVcsError::TaskInvalid(
            "task state must be a canonical object".to_owned(),
        ));
    };
    if entries.len() != 6 {
        return Err(WorkVcsError::TaskInvalid(format!(
            "task state must contain exactly 6 fields, found {}",
            entries.len()
        )));
    }

    let mut acceptance_criteria = None;
    let mut child_order = None;
    let mut description = None;
    let mut outcome = None;
    let mut priority = None;
    let mut status = None;

    for (key, value) in entries {
        match key.as_str() {
            "acceptance_criteria" => {
                acceptance_criteria = Some(parse_task_acceptance_criteria(value)?);
            }
            "child_order" => {
                require_empty_array("child_order", &value)?;
                child_order = Some(());
            }
            "description" => {
                let value = require_string("description", value)?;
                validate_description(&value)?;
                description = Some(value);
            }
            "outcome" => {
                outcome = Some(match value {
                    CanonicalValue::Null => None,
                    CanonicalValue::String(value) => {
                        validate_outcome(&value)?;
                        Some(value)
                    }
                    _ => {
                        return Err(WorkVcsError::TaskInvalid(
                            "outcome must be null or a string".to_owned(),
                        ));
                    }
                });
            }
            "priority" => {
                let CanonicalValue::Integer(value) = value else {
                    return Err(WorkVcsError::TaskInvalid(
                        "priority must be a JSON safe integer".to_owned(),
                    ));
                };
                priority = Some(value.get());
            }
            "status" => {
                let value = require_string("status", value)?;
                status = Some(TaskStatus::parse(&value)?);
            }
            other => {
                return Err(WorkVcsError::TaskInvalid(format!(
                    "task state contains unsupported field {other:?}"
                )));
            }
        }
    }

    let acceptance_criteria = acceptance_criteria.ok_or_else(|| {
        WorkVcsError::TaskInvalid("task state is missing acceptance_criteria".to_owned())
    })?;
    child_order
        .ok_or_else(|| WorkVcsError::TaskInvalid("task state is missing child_order".to_owned()))?;

    Ok(TaskState {
        description: description.ok_or_else(|| {
            WorkVcsError::TaskInvalid("task state is missing description".to_owned())
        })?,
        status: status
            .ok_or_else(|| WorkVcsError::TaskInvalid("task state is missing status".to_owned()))?,
        outcome: outcome
            .ok_or_else(|| WorkVcsError::TaskInvalid("task state is missing outcome".to_owned()))?,
        priority: priority.ok_or_else(|| {
            WorkVcsError::TaskInvalid("task state is missing priority".to_owned())
        })?,
        acceptance_criteria,
    })
}

fn parse_task_acceptance_criteria(
    value: CanonicalValue,
) -> Result<Vec<TaskAcceptanceCriterionRef>> {
    let CanonicalValue::Array(values) = value else {
        return Err(WorkVcsError::TaskInvalid(
            "acceptance_criteria must be an array".to_owned(),
        ));
    };
    let mut criteria = Vec::with_capacity(values.len());
    for value in values {
        let CanonicalValue::Object(entries) = value else {
            return Err(WorkVcsError::TaskInvalid(
                "acceptance_criteria entries must be canonical objects".to_owned(),
            ));
        };
        if entries.len() != 2 {
            return Err(WorkVcsError::TaskInvalid(format!(
                "acceptance_criteria entries must contain exactly 2 fields, found {}",
                entries.len()
            )));
        }

        let mut entity_id = None;
        let mut local_key = None;
        for (key, value) in entries {
            match key.as_str() {
                "entity_id" => {
                    let value = require_string("acceptance_criteria[].entity_id", value)?;
                    entity_id = Some(EntityId::parse_canonical(&value).map_err(task_invalid_from)?);
                }
                "local_key" => {
                    let value = require_string("acceptance_criteria[].local_key", value)?;
                    validate_local_key("acceptance criterion local key", &value)?;
                    local_key = Some(value);
                }
                other => {
                    return Err(WorkVcsError::TaskInvalid(format!(
                        "acceptance_criteria entry contains unsupported field {other:?}"
                    )));
                }
            }
        }
        criteria.push(TaskAcceptanceCriterionRef::new(
            local_key.ok_or_else(|| {
                WorkVcsError::TaskInvalid(
                    "acceptance_criteria entry is missing local_key".to_owned(),
                )
            })?,
            entity_id.ok_or_else(|| {
                WorkVcsError::TaskInvalid(
                    "acceptance_criteria entry is missing entity_id".to_owned(),
                )
            })?,
        )?);
    }
    validate_acceptance_criterion_refs(&criteria)?;
    require_acceptance_criteria_canonical_order(&criteria)?;
    Ok(criteria)
}

fn parse_acceptance_criterion_state(value: CanonicalValue) -> Result<AcceptanceCriterionState> {
    let CanonicalValue::Object(entries) = value else {
        return Err(WorkVcsError::TaskInvalid(
            "acceptance criterion state must be a canonical object".to_owned(),
        ));
    };
    if entries.len() != 3 {
        return Err(WorkVcsError::TaskInvalid(format!(
            "acceptance criterion state must contain exactly 3 fields, found {}",
            entries.len()
        )));
    }

    let mut classification = None;
    let mut statement = None;
    let mut verification_requirements = None;
    for (key, value) in entries {
        match key.as_str() {
            "classification" => {
                let value = require_string("classification", value)?;
                classification = Some(AcceptanceCriterionClassification::parse(&value)?);
            }
            "statement" => {
                let value = require_string("statement", value)?;
                validate_acceptance_criterion_statement(&value)?;
                statement = Some(value);
            }
            "verification_requirements" => {
                verification_requirements =
                    Some(parse_acceptance_criterion_verification_requirements(value)?);
            }
            other => {
                return Err(WorkVcsError::TaskInvalid(format!(
                    "acceptance criterion state contains unsupported field {other:?}"
                )));
            }
        }
    }
    let verification_requirements = verification_requirements.ok_or_else(|| {
        WorkVcsError::TaskInvalid(
            "acceptance criterion state is missing verification_requirements".to_owned(),
        )
    })?;
    AcceptanceCriterionState::new(
        statement.ok_or_else(|| {
            WorkVcsError::TaskInvalid("acceptance criterion state is missing statement".to_owned())
        })?,
        classification.ok_or_else(|| {
            WorkVcsError::TaskInvalid(
                "acceptance criterion state is missing classification".to_owned(),
            )
        })?,
    )
    .and_then(|state| state.with_verification_requirements(verification_requirements))
}

fn parse_acceptance_criterion_verification_requirements(
    value: CanonicalValue,
) -> Result<Vec<AcceptanceCriterionVerificationRequirementRef>> {
    let CanonicalValue::Array(values) = value else {
        return Err(WorkVcsError::TaskInvalid(
            "verification_requirements must be an array".to_owned(),
        ));
    };
    let mut requirements = Vec::with_capacity(values.len());
    for value in values {
        let CanonicalValue::Object(entries) = value else {
            return Err(WorkVcsError::TaskInvalid(
                "verification_requirements entries must be canonical objects".to_owned(),
            ));
        };
        if entries.len() != 2 {
            return Err(WorkVcsError::TaskInvalid(format!(
                "verification_requirements entries must contain exactly 2 fields, found {}",
                entries.len()
            )));
        }

        let mut entity_id = None;
        let mut local_key = None;
        for (key, value) in entries {
            match key.as_str() {
                "entity_id" => {
                    let value = require_string("verification_requirements[].entity_id", value)?;
                    entity_id = Some(EntityId::parse_canonical(&value).map_err(task_invalid_from)?);
                }
                "local_key" => {
                    let value = require_string("verification_requirements[].local_key", value)?;
                    validate_local_key("verification requirement local key", &value)?;
                    local_key = Some(value);
                }
                other => {
                    return Err(WorkVcsError::TaskInvalid(format!(
                        "verification_requirements entry contains unsupported field {other:?}"
                    )));
                }
            }
        }
        requirements.push(AcceptanceCriterionVerificationRequirementRef::new(
            local_key.ok_or_else(|| {
                WorkVcsError::TaskInvalid(
                    "verification_requirements entry is missing local_key".to_owned(),
                )
            })?,
            entity_id.ok_or_else(|| {
                WorkVcsError::TaskInvalid(
                    "verification_requirements entry is missing entity_id".to_owned(),
                )
            })?,
        )?);
    }
    validate_verification_requirement_refs(&requirements)?;
    require_verification_requirements_canonical_order(&requirements)?;
    Ok(requirements)
}

fn require_task_references_acceptance_criterion(
    task_state: &TaskState,
    criterion: &AcceptanceCriterionSnapshot,
) -> Result<()> {
    let referenced = task_state.acceptance_criteria.iter().any(|candidate| {
        candidate.local_key == criterion.local_key
            && candidate.acceptance_criterion_entity_id == criterion.acceptance_criterion_entity_id
    });
    if referenced {
        Ok(())
    } else {
        Err(WorkVcsError::TaskInvalid(format!(
            "task entity {} does not reference acceptance criterion {}/{}",
            criterion.task_entity_id, criterion.local_key, criterion.acceptance_criterion_entity_id
        )))
    }
}

fn require_acceptance_criterion_references_verification_requirement(
    criterion_state: &AcceptanceCriterionState,
    requirement: &VerificationRequirementSnapshot,
) -> Result<()> {
    let referenced = criterion_state
        .verification_requirements
        .iter()
        .any(|candidate| {
            candidate.local_key == requirement.local_key
                && candidate.verification_requirement_entity_id
                    == requirement.verification_requirement_entity_id
        });
    if referenced {
        Ok(())
    } else {
        Err(WorkVcsError::TaskInvalid(format!(
            "acceptance criterion entity {} does not reference verification requirement {}/{}",
            requirement.acceptance_criterion_entity_id,
            requirement.local_key,
            requirement.verification_requirement_entity_id
        )))
    }
}

fn parse_verification_requirement_state(
    value: CanonicalValue,
) -> Result<VerificationRequirementState> {
    let CanonicalValue::Object(entries) = value else {
        return Err(WorkVcsError::TaskInvalid(
            "verification requirement state must be a canonical object".to_owned(),
        ));
    };
    if entries.len() != 1 {
        return Err(WorkVcsError::TaskInvalid(format!(
            "verification requirement state must contain exactly 1 field, found {}",
            entries.len()
        )));
    }

    let mut statement = None;
    for (key, value) in entries {
        match key.as_str() {
            "statement" => {
                let value = require_string("statement", value)?;
                validate_verification_requirement_statement(&value)?;
                statement = Some(value);
            }
            other => {
                return Err(WorkVcsError::TaskInvalid(format!(
                    "verification requirement state contains unsupported field {other:?}"
                )));
            }
        }
    }

    VerificationRequirementState::new(statement.ok_or_else(|| {
        WorkVcsError::TaskInvalid("verification requirement state is missing statement".to_owned())
    })?)
}

fn parse_verification_state(value: CanonicalValue) -> Result<VerificationState> {
    let CanonicalValue::Object(entries) = value else {
        return Err(WorkVcsError::TaskInvalid(
            "verification state must be a canonical object".to_owned(),
        ));
    };
    if entries.len() != 4 {
        return Err(WorkVcsError::TaskInvalid(format!(
            "verification state must contain exactly 4 fields, found {}",
            entries.len()
        )));
    }

    let mut basis = None;
    let mut evidence = None;
    let mut method = None;
    let mut result = None;
    for (key, value) in entries {
        match key.as_str() {
            "basis" => {
                basis = Some(parse_verification_basis(value)?);
            }
            "evidence" => {
                evidence = Some(parse_verification_evidence_refs(value)?);
            }
            "method" => {
                validate_verification_method(&value)?;
                method = Some(value);
            }
            "result" => {
                let value = require_string("result", value)?;
                result = Some(VerificationResult::parse(&value)?);
            }
            other => {
                return Err(WorkVcsError::TaskInvalid(format!(
                    "verification state contains unsupported field {other:?}"
                )));
            }
        }
    }
    let evidence = evidence.ok_or_else(|| {
        WorkVcsError::TaskInvalid("verification state is missing evidence".to_owned())
    })?;
    let basis = basis.ok_or_else(|| {
        WorkVcsError::TaskInvalid("verification state is missing basis".to_owned())
    })?;
    VerificationState::new_with_evidence_and_resource_basis(
        result.ok_or_else(|| {
            WorkVcsError::TaskInvalid("verification state is missing result".to_owned())
        })?,
        method.ok_or_else(|| {
            WorkVcsError::TaskInvalid("verification state is missing method".to_owned())
        })?,
        basis.verified_at_commit_id,
        basis.semantic_dependencies,
        evidence,
        basis.resource_basis,
    )
}

fn parse_verification_evidence_refs(value: CanonicalValue) -> Result<Vec<VerificationEvidenceRef>> {
    let CanonicalValue::Array(entries) = value else {
        return Err(WorkVcsError::TaskInvalid(
            "verification evidence must be an array".to_owned(),
        ));
    };
    let mut evidence = Vec::new();
    for entry in entries {
        let CanonicalValue::Object(fields) = entry else {
            return Err(WorkVcsError::TaskInvalid(
                "verification evidence entry must be an object".to_owned(),
            ));
        };
        if fields.len() != 1 {
            return Err(WorkVcsError::TaskInvalid(format!(
                "verification evidence entry must contain exactly 1 field, found {}",
                fields.len()
            )));
        }
        let mut evidence_id = None;
        for (key, value) in fields {
            match key.as_str() {
                "evidence_id" => {
                    let value = require_string("evidence_id", value)?;
                    evidence_id = Some(EvidenceId::parse_canonical(&value).map_err(|error| {
                        WorkVcsError::TaskInvalid(format!(
                            "verification evidence_id is not canonical: {error}"
                        ))
                    })?);
                }
                other => {
                    return Err(WorkVcsError::TaskInvalid(format!(
                        "verification evidence entry contains unsupported field {other:?}"
                    )));
                }
            }
        }
        evidence.push(VerificationEvidenceRef::new(evidence_id.ok_or_else(
            || {
                WorkVcsError::TaskInvalid(
                    "verification evidence entry is missing evidence_id".to_owned(),
                )
            },
        )?));
    }
    validate_verification_evidence_refs(&evidence)?;
    Ok(evidence)
}

fn parse_verification_basis(value: CanonicalValue) -> Result<ParsedVerificationBasis> {
    let CanonicalValue::Object(entries) = value else {
        return Err(WorkVcsError::TaskInvalid(
            "verification basis must be a canonical object".to_owned(),
        ));
    };
    if !(2..=3).contains(&entries.len()) {
        return Err(WorkVcsError::TaskInvalid(format!(
            "verification basis must contain 2 or 3 fields, found {}",
            entries.len()
        )));
    }

    let mut resource_basis = None;
    let mut semantic_dependencies = None;
    let mut verified_at_commit_id = None;
    for (key, value) in entries {
        match key.as_str() {
            "resource_basis" => {
                resource_basis = Some(parse_verification_resource_basis(value)?);
            }
            "semantic_dependencies" => {
                semantic_dependencies = Some(parse_verification_semantic_dependencies(value)?);
            }
            "verified_at_commit_id" => {
                let value = require_string("verified_at_commit_id", value)?;
                verified_at_commit_id =
                    Some(CommitId::parse_canonical(&value).map_err(task_invalid_from)?);
            }
            other => {
                return Err(WorkVcsError::TaskInvalid(format!(
                    "verification basis contains unsupported field {other:?}"
                )));
            }
        }
    }

    Ok(ParsedVerificationBasis {
        verified_at_commit_id: verified_at_commit_id.ok_or_else(|| {
            WorkVcsError::TaskInvalid(
                "verification basis is missing verified_at_commit_id".to_owned(),
            )
        })?,
        semantic_dependencies: semantic_dependencies.ok_or_else(|| {
            WorkVcsError::TaskInvalid(
                "verification basis is missing semantic_dependencies".to_owned(),
            )
        })?,
        resource_basis: resource_basis.unwrap_or_default(),
    })
}

fn parse_verification_resource_basis(
    value: CanonicalValue,
) -> Result<Vec<VerificationResourceBasis>> {
    let CanonicalValue::Array(values) = value else {
        return Err(WorkVcsError::TaskInvalid(
            "resource_basis must be an array".to_owned(),
        ));
    };
    let mut entries = Vec::with_capacity(values.len());
    for value in values {
        let CanonicalValue::Object(fields) = value else {
            return Err(WorkVcsError::TaskInvalid(
                "resource_basis entries must be canonical objects".to_owned(),
            ));
        };
        if fields.len() != 8 {
            return Err(WorkVcsError::TaskInvalid(format!(
                "resource_basis entries must contain exactly 8 fields, found {}",
                fields.len()
            )));
        }
        let mut adapter_kind = None;
        let mut adapter_schema_version = None;
        let mut baseline_fingerprint = None;
        let mut baseline_observation_id = None;
        let mut resource_id = None;
        let mut scope_kind = None;
        let mut scope_payload = None;
        let mut scope_schema_version = None;
        for (key, value) in fields {
            match key.as_str() {
                "adapter_kind" => {
                    let value = require_string("resource_basis[].adapter_kind", value)?;
                    validate_local_key("resource_basis adapter_kind", &value)?;
                    adapter_kind = Some(value);
                }
                "adapter_schema_version" => {
                    adapter_schema_version = Some(require_positive_i64(
                        "resource_basis[].adapter_schema_version",
                        value,
                    )?);
                }
                "baseline_fingerprint" => {
                    let value = require_string("resource_basis[].baseline_fingerprint", value)?;
                    baseline_fingerprint =
                        Some(Digest::from_hex(&value).map_err(task_invalid_from)?);
                }
                "baseline_observation_id" => {
                    baseline_observation_id = Some(match value {
                        CanonicalValue::Null => None,
                        CanonicalValue::String(value) => Some(
                            ResourceObservationId::parse_canonical(&value)
                                .map_err(task_invalid_from)?,
                        ),
                        _ => {
                            return Err(WorkVcsError::TaskInvalid(
                                "resource_basis[].baseline_observation_id must be null or a canonical UUID"
                                    .to_owned(),
                            ));
                        }
                    });
                }
                "resource_id" => {
                    let value = require_string("resource_basis[].resource_id", value)?;
                    resource_id =
                        Some(ResourceId::parse_canonical(&value).map_err(task_invalid_from)?);
                }
                "scope_kind" => {
                    let value = require_string("resource_basis[].scope_kind", value)?;
                    validate_local_key("resource_basis scope_kind", &value)?;
                    scope_kind = Some(value);
                }
                "scope_payload" => {
                    require_verification_resource_scope_payload(&value)?;
                    scope_payload = Some(value);
                }
                "scope_schema_version" => {
                    scope_schema_version = Some(require_positive_i64(
                        "resource_basis[].scope_schema_version",
                        value,
                    )?);
                }
                other => {
                    return Err(WorkVcsError::TaskInvalid(format!(
                        "resource_basis entry contains unsupported field {other:?}"
                    )));
                }
            }
        }
        let mut basis = VerificationResourceBasis::new(
            resource_id.ok_or_else(|| {
                WorkVcsError::TaskInvalid("resource_basis entry is missing resource_id".to_owned())
            })?,
            adapter_kind.ok_or_else(|| {
                WorkVcsError::TaskInvalid("resource_basis entry is missing adapter_kind".to_owned())
            })?,
            adapter_schema_version.ok_or_else(|| {
                WorkVcsError::TaskInvalid(
                    "resource_basis entry is missing adapter_schema_version".to_owned(),
                )
            })?,
            scope_kind.ok_or_else(|| {
                WorkVcsError::TaskInvalid("resource_basis entry is missing scope_kind".to_owned())
            })?,
            scope_schema_version.ok_or_else(|| {
                WorkVcsError::TaskInvalid(
                    "resource_basis entry is missing scope_schema_version".to_owned(),
                )
            })?,
            scope_payload.ok_or_else(|| {
                WorkVcsError::TaskInvalid(
                    "resource_basis entry is missing scope_payload".to_owned(),
                )
            })?,
            baseline_fingerprint.ok_or_else(|| {
                WorkVcsError::TaskInvalid(
                    "resource_basis entry is missing baseline_fingerprint".to_owned(),
                )
            })?,
        )?;
        basis.baseline_observation_id = baseline_observation_id.ok_or_else(|| {
            WorkVcsError::TaskInvalid(
                "resource_basis entry is missing baseline_observation_id".to_owned(),
            )
        })?;
        validate_verification_resource_basis_entry(&basis)?;
        entries.push(basis);
    }
    validate_verification_resource_basis(&entries)?;
    Ok(entries)
}

fn parse_verification_semantic_dependencies(
    value: CanonicalValue,
) -> Result<Vec<VerificationSemanticDependency>> {
    let CanonicalValue::Array(values) = value else {
        return Err(WorkVcsError::TaskInvalid(
            "semantic_dependencies must be an array".to_owned(),
        ));
    };
    let mut dependencies = Vec::with_capacity(values.len());
    for value in values {
        let CanonicalValue::Object(entries) = value else {
            return Err(WorkVcsError::TaskInvalid(
                "semantic_dependencies entries must be canonical objects".to_owned(),
            ));
        };
        if entries.len() != 2 {
            return Err(WorkVcsError::TaskInvalid(format!(
                "semantic_dependencies entries must contain exactly 2 fields, found {}",
                entries.len()
            )));
        }

        let mut entity_id = None;
        let mut entity_version_id = None;
        for (key, value) in entries {
            match key.as_str() {
                "entity_id" => {
                    let value = require_string("semantic_dependencies[].entity_id", value)?;
                    entity_id = Some(EntityId::parse_canonical(&value).map_err(task_invalid_from)?);
                }
                "entity_version_id" => {
                    let value = require_string("semantic_dependencies[].entity_version_id", value)?;
                    entity_version_id =
                        Some(EntityVersionId::parse_canonical(&value).map_err(task_invalid_from)?);
                }
                other => {
                    return Err(WorkVcsError::TaskInvalid(format!(
                        "semantic_dependencies entry contains unsupported field {other:?}"
                    )));
                }
            }
        }
        dependencies.push(VerificationSemanticDependency::new(
            entity_id.ok_or_else(|| {
                WorkVcsError::TaskInvalid(
                    "semantic_dependencies entry is missing entity_id".to_owned(),
                )
            })?,
            entity_version_id.ok_or_else(|| {
                WorkVcsError::TaskInvalid(
                    "semantic_dependencies entry is missing entity_version_id".to_owned(),
                )
            })?,
        ));
    }
    validate_verification_semantic_dependencies(&dependencies)?;
    require_verification_semantic_dependencies_canonical_order(&dependencies)?;
    Ok(dependencies)
}

fn ensure_dependency_create_is_acyclic(
    connection: &StoreConnection,
    commit_id: CommitId,
    workspace_id: WorkspaceId,
    state: &WorkState,
    dependent_task_entity_id: EntityId,
    prerequisite_task_entity_id: EntityId,
) -> Result<()> {
    let graph = current_dependency_graph(connection, commit_id, workspace_id, state)?;
    ensure_dependency_graph_is_acyclic(&graph)?;
    if dependency_path_exists(
        &graph,
        prerequisite_task_entity_id,
        dependent_task_entity_id,
    ) {
        return Err(WorkVcsError::TaskInvalid(format!(
            "depends_on relation from {dependent_task_entity_id} to {prerequisite_task_entity_id} would create a dependency cycle"
        )));
    }
    Ok(())
}

fn current_dependency_graph(
    connection: &StoreConnection,
    commit_id: CommitId,
    workspace_id: WorkspaceId,
    state: &WorkState,
) -> Result<BTreeMap<EntityId, Vec<EntityId>>> {
    let mut graph = BTreeMap::<EntityId, Vec<EntityId>>::new();
    for (relation_id, relation_version_id) in state.relations() {
        let Some(relation) = load_task_scheduling_relation_version(
            connection,
            workspace_id,
            commit_id,
            *relation_id,
            *relation_version_id,
        )?
        else {
            continue;
        };
        if relation.relation_type == TaskSchedulingRelationType::DependsOn {
            graph
                .entry(relation.source_task_entity_id)
                .or_default()
                .push(relation.target_task_entity_id);
        }
    }
    for prerequisites in graph.values_mut() {
        prerequisites.sort();
        prerequisites.dedup();
    }
    Ok(graph)
}

fn ensure_dependency_graph_is_acyclic(graph: &BTreeMap<EntityId, Vec<EntityId>>) -> Result<()> {
    let mut visiting = HashSet::new();
    let mut visited = HashSet::new();
    for node in graph.keys().copied() {
        if dependency_dfs_has_cycle(node, graph, &mut visiting, &mut visited) {
            return Err(WorkVcsError::TaskInvalid(
                "current depends_on graph already contains a dependency cycle".to_owned(),
            ));
        }
    }
    Ok(())
}

fn dependency_dfs_has_cycle(
    node: EntityId,
    graph: &BTreeMap<EntityId, Vec<EntityId>>,
    visiting: &mut HashSet<EntityId>,
    visited: &mut HashSet<EntityId>,
) -> bool {
    if visited.contains(&node) {
        return false;
    }
    if !visiting.insert(node) {
        return true;
    }
    if let Some(next_nodes) = graph.get(&node) {
        for next in next_nodes {
            if dependency_dfs_has_cycle(*next, graph, visiting, visited) {
                return true;
            }
        }
    }
    visiting.remove(&node);
    visited.insert(node);
    false
}

fn dependency_path_exists(
    graph: &BTreeMap<EntityId, Vec<EntityId>>,
    start: EntityId,
    target: EntityId,
) -> bool {
    let mut seen = HashSet::new();
    let mut stack = vec![start];
    while let Some(node) = stack.pop() {
        if !seen.insert(node) {
            continue;
        }
        if node == target {
            return true;
        }
        if let Some(next_nodes) = graph.get(&node) {
            stack.extend(next_nodes.iter().copied());
        }
    }
    false
}

fn verification_semantic_dependencies_for_target(
    connection: &StoreConnection,
    commit_id: CommitId,
    target: VerificationTarget,
) -> Result<Vec<VerificationSemanticDependency>> {
    match target {
        VerificationTarget::AcceptanceCriterion(acceptance_criterion_entity_id) => {
            let criterion =
                acceptance_criterion_at(connection, commit_id, acceptance_criterion_entity_id)?;
            if !criterion.state.verification_requirements.is_empty() {
                return Err(WorkVcsError::TaskInvalid(format!(
                    "acceptance criterion {acceptance_criterion_entity_id} has verification requirements; verification must target a requirement"
                )));
            }
            Ok(vec![VerificationSemanticDependency::new(
                acceptance_criterion_entity_id,
                criterion.acceptance_criterion_entity_version_id,
            )])
        }
        VerificationTarget::VerificationRequirement(verification_requirement_entity_id) => {
            let requirement = verification_requirement_at(
                connection,
                commit_id,
                verification_requirement_entity_id,
            )?;
            let criterion = acceptance_criterion_at(
                connection,
                commit_id,
                requirement.acceptance_criterion_entity_id,
            )?;
            require_acceptance_criterion_references_verification_requirement(
                &criterion.state,
                &requirement,
            )?;
            Ok(vec![VerificationSemanticDependency::new(
                verification_requirement_entity_id,
                requirement.verification_requirement_entity_version_id,
            )])
        }
    }
}

fn effective_status_for_target(
    connection: &StoreConnection,
    commit_id: CommitId,
    branch_id: Option<BranchId>,
    target: VerificationTarget,
) -> Result<AcceptanceCriterionEffectiveStatus> {
    let replayed = state_at(connection, commit_id)?;
    let verifications = load_current_verifications_for_target(
        connection,
        commit_id,
        replayed.workspace_id,
        &replayed.state,
        target,
    )?;
    let mut has_applicable_passed = false;
    let mut has_applicable_failed = false;
    let mut has_stale_or_unknown = false;

    for verification in verifications {
        match verification_applicability_for_effective_status(
            connection,
            branch_id,
            commit_id,
            &replayed.state,
            &verification,
        )? {
            VerificationApplicability::Applicable => match verification.state.result {
                VerificationResult::Passed => has_applicable_passed = true,
                VerificationResult::Failed => has_applicable_failed = true,
                VerificationResult::Inconclusive => {}
            },
            VerificationApplicability::Stale | VerificationApplicability::Unknown => {
                has_stale_or_unknown = true;
            }
        }
    }

    match (
        has_applicable_passed,
        has_applicable_failed,
        has_stale_or_unknown,
    ) {
        (true, true, _) => Ok(AcceptanceCriterionEffectiveStatus::Conflicted),
        (_, true, _) => Ok(AcceptanceCriterionEffectiveStatus::Failed),
        (true, false, _) => Ok(AcceptanceCriterionEffectiveStatus::Verified),
        (false, false, true) => Ok(AcceptanceCriterionEffectiveStatus::Stale),
        (false, false, false) => Ok(AcceptanceCriterionEffectiveStatus::Unverified),
    }
}

fn load_current_verifications_for_target(
    connection: &StoreConnection,
    commit_id: CommitId,
    workspace_id: WorkspaceId,
    state: &WorkState,
    target: VerificationTarget,
) -> Result<Vec<VerificationSnapshot>> {
    let mut verifications = Vec::new();
    for (relation_id, relation_version_id) in state.relations() {
        let Some(relation) = load_verifies_relation_version(
            connection,
            workspace_id,
            *relation_id,
            *relation_version_id,
        )?
        else {
            continue;
        };
        if relation.target != target {
            continue;
        }
        let Some(verification_entity_version_id) =
            state
                .entities()
                .iter()
                .find_map(|(entity_id, entity_version_id)| {
                    (*entity_id == relation.source_verification_entity_id)
                        .then_some(*entity_version_id)
                })
        else {
            return Err(WorkVcsError::TaskInvalid(format!(
                "verifies relation {} source verification {} is not present in WorkState",
                relation.relation_id, relation.source_verification_entity_id
            )));
        };
        let loaded = load_verification_version(
            connection,
            workspace_id,
            relation.source_verification_entity_id,
            verification_entity_version_id,
        )?;
        let evidenced_by_relations = load_current_evidenced_by_relations_for_source(
            connection,
            workspace_id,
            state,
            relation.source_verification_entity_id,
        )?;
        require_verification_evidence_closure(
            relation.source_verification_entity_id,
            &loaded.state.evidence,
            &evidenced_by_relations,
        )?;
        verifications.push(VerificationSnapshot {
            workspace_id,
            commit_id,
            verification_entity_id: relation.source_verification_entity_id,
            verification_entity_version_id,
            state_digest: loaded.state_digest,
            verifies_relation_id: relation.relation_id,
            verifies_relation_version_id: relation.relation_version_id,
            verifies_relation_state_digest: relation.state_digest,
            evidenced_by_relations,
            target: relation.target,
            state: loaded.state,
        });
    }
    Ok(verifications)
}

fn verification_applicability(
    state: &WorkState,
    verification: &VerificationState,
) -> VerificationApplicability {
    if !work_state_basis_is_applicable(state, &verification.semantic_dependencies) {
        return VerificationApplicability::Stale;
    }
    if !verification.resource_basis.is_empty() {
        return VerificationApplicability::Unknown;
    }
    VerificationApplicability::Applicable
}

fn verification_applicability_for_effective_status(
    connection: &StoreConnection,
    branch_id: Option<BranchId>,
    commit_id: CommitId,
    state: &WorkState,
    verification: &VerificationSnapshot,
) -> Result<VerificationApplicability> {
    let base_applicability = verification_applicability(state, &verification.state);
    if base_applicability == VerificationApplicability::Stale
        || verification.state.resource_basis.is_empty()
    {
        return Ok(base_applicability);
    }

    let Some(branch_id) = branch_id else {
        return Ok(base_applicability);
    };
    let Some(cache) = verification_applicability_cache(
        connection,
        branch_id,
        verification.verification_entity_id,
    )?
    else {
        return Ok(VerificationApplicability::Unknown);
    };
    if cache.evaluated_commit_id == commit_id {
        Ok(cache.applicability)
    } else {
        Ok(VerificationApplicability::Unknown)
    }
}

fn work_state_basis_is_applicable(
    state: &WorkState,
    dependencies: &[VerificationSemanticDependency],
) -> bool {
    dependencies.iter().all(|dependency| {
        state
            .entities()
            .iter()
            .find_map(|(entity_id, entity_version_id)| {
                (*entity_id == dependency.entity_id).then_some(*entity_version_id)
            })
            == Some(dependency.entity_version_id)
    })
}

struct ResourceStampView<'a> {
    resource_basis_ordinal: i64,
    adapter_kind: &'a str,
    adapter_schema_version: i64,
    scope_schema_version: i64,
    observation_status: ApplicabilityResourceObservationStatus,
    observed_fingerprint: Option<Digest>,
    observation_id: Option<ResourceObservationId>,
}

impl<'a> From<&'a ApplicabilityResourceStampInput> for ResourceStampView<'a> {
    fn from(stamp: &'a ApplicabilityResourceStampInput) -> Self {
        Self {
            resource_basis_ordinal: stamp.resource_basis_ordinal,
            adapter_kind: &stamp.adapter_kind,
            adapter_schema_version: stamp.adapter_schema_version,
            scope_schema_version: stamp.scope_schema_version,
            observation_status: stamp.observation_status,
            observed_fingerprint: stamp.observed_fingerprint,
            observation_id: stamp.observation_id,
        }
    }
}

impl<'a> From<&'a ApplicabilityResourceStampSnapshot> for ResourceStampView<'a> {
    fn from(stamp: &'a ApplicabilityResourceStampSnapshot) -> Self {
        Self {
            resource_basis_ordinal: stamp.resource_basis_ordinal,
            adapter_kind: &stamp.adapter_kind,
            adapter_schema_version: stamp.adapter_schema_version,
            scope_schema_version: stamp.scope_schema_version,
            observation_status: stamp.observation_status,
            observed_fingerprint: stamp.observed_fingerprint,
            observation_id: stamp.observation_id,
        }
    }
}

fn compute_recorded_verification_applicability(
    state: &WorkState,
    verification: &VerificationState,
    resource_stamps: &[ApplicabilityResourceStampInput],
) -> Result<(VerificationApplicability, &'static str)> {
    if !work_state_basis_is_applicable(state, &verification.semantic_dependencies) {
        return Ok((VerificationApplicability::Stale, "work_state_stale"));
    }
    compute_resource_stamp_applicability(
        &verification.resource_basis,
        resource_stamps.iter().map(ResourceStampView::from),
    )
}

fn compute_cached_verification_applicability(
    state: &WorkState,
    verification: &VerificationState,
    resource_stamps: &[ApplicabilityResourceStampSnapshot],
) -> Result<(VerificationApplicability, &'static str)> {
    if !work_state_basis_is_applicable(state, &verification.semantic_dependencies) {
        return Ok((VerificationApplicability::Stale, "work_state_stale"));
    }
    compute_resource_stamp_applicability(
        &verification.resource_basis,
        resource_stamps.iter().map(ResourceStampView::from),
    )
}

fn compute_resource_stamp_applicability<'a>(
    resource_basis: &[VerificationResourceBasis],
    resource_stamps: impl Iterator<Item = ResourceStampView<'a>>,
) -> Result<(VerificationApplicability, &'static str)> {
    if resource_basis.is_empty() {
        return Ok((
            VerificationApplicability::Applicable,
            "work_state_applicable",
        ));
    }

    let mut has_unavailable = false;
    let mut has_error = false;
    for (basis, stamp) in resource_basis.iter().zip(resource_stamps) {
        match stamp.observation_status {
            ApplicabilityResourceObservationStatus::Observed => {
                let observed_fingerprint = stamp.observed_fingerprint.ok_or_else(|| {
                    WorkVcsError::TaskInvalid(
                        "observed applicability resource stamp is missing observed_fingerprint"
                            .to_owned(),
                    )
                })?;
                if observed_fingerprint != basis.baseline_fingerprint {
                    return Ok((VerificationApplicability::Stale, "resource_drift"));
                }
            }
            ApplicabilityResourceObservationStatus::Unavailable => {
                has_unavailable = true;
            }
            ApplicabilityResourceObservationStatus::Error => {
                has_error = true;
            }
        }
    }
    if has_error {
        Ok((VerificationApplicability::Unknown, "resource_error"))
    } else if has_unavailable {
        Ok((VerificationApplicability::Unknown, "resource_unavailable"))
    } else {
        Ok((
            VerificationApplicability::Applicable,
            "all_basis_applicable",
        ))
    }
}

fn combine_acceptance_criterion_status(
    left: AcceptanceCriterionEffectiveStatus,
    right: AcceptanceCriterionEffectiveStatus,
) -> AcceptanceCriterionEffectiveStatus {
    use AcceptanceCriterionEffectiveStatus::{Conflicted, Failed, Stale, Unverified, Verified};
    match (left, right) {
        (Conflicted, _) | (_, Conflicted) => Conflicted,
        (Failed, _) | (_, Failed) => Failed,
        (Stale, _) | (_, Stale) => Stale,
        (Unverified, _) | (_, Unverified) => Unverified,
        (Verified, Verified) => Verified,
    }
}

fn relation_transition_payload_value(
    relation_id: RelationId,
    before_relation_version_id: Option<RelationVersionId>,
    after_relation_version_id: RelationVersionId,
) -> Result<CanonicalValue> {
    let before_value = match before_relation_version_id {
        Some(version_id) => CanonicalValue::String(version_id.to_string()),
        None => CanonicalValue::Null,
    };
    CanonicalValue::object(vec![
        (
            "after_relation_version_id".to_owned(),
            CanonicalValue::String(after_relation_version_id.to_string()),
        ),
        ("before_relation_version_id".to_owned(), before_value),
        (
            "relation_id".to_owned(),
            CanonicalValue::String(relation_id.to_string()),
        ),
    ])
    .map_err(task_invalid_from)
}

fn verification_basis_value(
    verified_at_commit_id: CommitId,
    semantic_dependencies: &[VerificationSemanticDependency],
    resource_basis: &[VerificationResourceBasis],
) -> Result<CanonicalValue> {
    validate_verification_semantic_dependencies(semantic_dependencies)?;
    validate_verification_resource_basis(resource_basis)?;
    let mut semantic_dependencies = semantic_dependencies.to_vec();
    semantic_dependencies.sort_by(|left, right| {
        left.entity_id
            .raw_bytes()
            .cmp(&right.entity_id.raw_bytes())
            .then_with(|| {
                left.entity_version_id
                    .raw_bytes()
                    .cmp(&right.entity_version_id.raw_bytes())
            })
    });
    let dependencies = semantic_dependencies
        .iter()
        .map(VerificationSemanticDependency::to_canonical_value)
        .collect::<Result<Vec<_>>>()?;
    let resource_basis = resource_basis
        .iter()
        .map(VerificationResourceBasis::to_canonical_value)
        .collect::<Result<Vec<_>>>()?;
    CanonicalValue::object(vec![
        (
            "resource_basis".to_owned(),
            CanonicalValue::Array(resource_basis),
        ),
        (
            "semantic_dependencies".to_owned(),
            CanonicalValue::Array(dependencies),
        ),
        (
            "verified_at_commit_id".to_owned(),
            CanonicalValue::String(verified_at_commit_id.to_string()),
        ),
    ])
    .map_err(task_invalid_from)
}

fn legacy_verification_basis_value(
    verified_at_commit_id: CommitId,
    semantic_dependencies: &[VerificationSemanticDependency],
) -> Result<CanonicalValue> {
    validate_verification_semantic_dependencies(semantic_dependencies)?;
    let mut semantic_dependencies = semantic_dependencies.to_vec();
    semantic_dependencies.sort_by(|left, right| {
        left.entity_id
            .raw_bytes()
            .cmp(&right.entity_id.raw_bytes())
            .then_with(|| {
                left.entity_version_id
                    .raw_bytes()
                    .cmp(&right.entity_version_id.raw_bytes())
            })
    });
    let dependencies = semantic_dependencies
        .iter()
        .map(VerificationSemanticDependency::to_canonical_value)
        .collect::<Result<Vec<_>>>()?;
    CanonicalValue::object(vec![
        (
            "semantic_dependencies".to_owned(),
            CanonicalValue::Array(dependencies),
        ),
        (
            "verified_at_commit_id".to_owned(),
            CanonicalValue::String(verified_at_commit_id.to_string()),
        ),
    ])
    .map_err(task_invalid_from)
}

fn require_mandatory_acceptance_criteria_verified(
    connection: &StoreConnection,
    branch_id: BranchId,
    commit_id: CommitId,
    task: &TaskSnapshot,
) -> Result<()> {
    for criterion_ref in &task.state.acceptance_criteria {
        let criterion = acceptance_criterion_at(
            connection,
            commit_id,
            criterion_ref.acceptance_criterion_entity_id,
        )?;
        if criterion.task_entity_id != task.task_entity_id
            || criterion.local_key != criterion_ref.local_key
        {
            return Err(WorkVcsError::TaskInvalid(format!(
                "task entity {} acceptance criterion reference {:?} does not match stored identity",
                task.task_entity_id, criterion_ref.local_key
            )));
        }
        if criterion.state.classification == AcceptanceCriterionClassification::Required {
            let status = acceptance_criterion_effective_status_for_branch_commit(
                connection,
                branch_id,
                commit_id,
                criterion.acceptance_criterion_entity_id,
            )?;
            if status != AcceptanceCriterionEffectiveStatus::Verified {
                return Err(WorkVcsError::TaskInvalid(format!(
                    "mandatory acceptance criterion {} is {status}; expected verified",
                    criterion.local_key
                )));
            }
        }
    }
    Ok(())
}

fn require_empty_array(label: &str, value: &CanonicalValue) -> Result<()> {
    let CanonicalValue::Array(values) = value else {
        return Err(WorkVcsError::TaskInvalid(format!(
            "{label} must be an array"
        )));
    };
    if values.is_empty() {
        Ok(())
    } else {
        Err(WorkVcsError::TaskInvalid(format!(
            "{label} is deferred in the current implementation slice and must be empty"
        )))
    }
}

fn require_object(label: &str, value: &CanonicalValue) -> Result<()> {
    match value {
        CanonicalValue::Object(_) => Ok(()),
        _ => Err(WorkVcsError::TaskInvalid(format!(
            "{label} must be a canonical JSON object"
        ))),
    }
}

fn parse_canonical_object_json(label: &str, input: &str) -> Result<CanonicalValue> {
    let value = parse_canonical_json(input.as_bytes()).map_err(task_invalid_from)?;
    require_object(label, &value)?;
    let encoded = canonical_json_string(&value)?;
    if encoded != input {
        return Err(WorkVcsError::TaskInvalid(format!(
            "{label} is not canonical fixed-point JSON"
        )));
    }
    Ok(value)
}

fn require_string(label: &str, value: CanonicalValue) -> Result<String> {
    let CanonicalValue::String(value) = value else {
        return Err(WorkVcsError::TaskInvalid(format!(
            "{label} must be a string"
        )));
    };
    Ok(value)
}

fn validate_acceptance_criterion_refs(criteria: &[TaskAcceptanceCriterionRef]) -> Result<()> {
    let mut local_keys = HashSet::new();
    let mut entity_ids = HashSet::new();
    for criterion in criteria {
        validate_local_key("acceptance criterion local key", &criterion.local_key)?;
        if !local_keys.insert(criterion.local_key.as_str()) {
            return Err(WorkVcsError::TaskInvalid(format!(
                "task state contains duplicate acceptance criterion local key {:?}",
                criterion.local_key
            )));
        }
        if !entity_ids.insert(criterion.acceptance_criterion_entity_id.raw_bytes()) {
            return Err(WorkVcsError::TaskInvalid(format!(
                "task state contains duplicate acceptance criterion entity id {}",
                criterion.acceptance_criterion_entity_id
            )));
        }
    }
    Ok(())
}

fn validate_verification_requirement_refs(
    requirements: &[AcceptanceCriterionVerificationRequirementRef],
) -> Result<()> {
    let mut local_keys = HashSet::new();
    let mut entity_ids = HashSet::new();
    for requirement in requirements {
        validate_local_key("verification requirement local key", &requirement.local_key)?;
        if !local_keys.insert(requirement.local_key.as_str()) {
            return Err(WorkVcsError::TaskInvalid(format!(
                "acceptance criterion state contains duplicate verification requirement local key {:?}",
                requirement.local_key
            )));
        }
        if !entity_ids.insert(requirement.verification_requirement_entity_id.raw_bytes()) {
            return Err(WorkVcsError::TaskInvalid(format!(
                "acceptance criterion state contains duplicate verification requirement entity id {}",
                requirement.verification_requirement_entity_id
            )));
        }
    }
    Ok(())
}

fn require_acceptance_criteria_canonical_order(
    criteria: &[TaskAcceptanceCriterionRef],
) -> Result<()> {
    for pair in criteria.windows(2) {
        let [left, right] = pair else {
            continue;
        };
        if left.local_key >= right.local_key {
            return Err(WorkVcsError::TaskInvalid(
                "task acceptance_criteria must be sorted by local_key".to_owned(),
            ));
        }
    }
    Ok(())
}

fn require_verification_requirements_canonical_order(
    requirements: &[AcceptanceCriterionVerificationRequirementRef],
) -> Result<()> {
    for pair in requirements.windows(2) {
        let [left, right] = pair else {
            continue;
        };
        if left.local_key >= right.local_key {
            return Err(WorkVcsError::TaskInvalid(
                "acceptance criterion verification_requirements must be sorted by local_key"
                    .to_owned(),
            ));
        }
    }
    Ok(())
}

fn validate_local_key(label: &str, value: &str) -> Result<()> {
    if value.trim().is_empty() {
        return Err(WorkVcsError::TaskInvalid(format!(
            "{label} must not be empty"
        )));
    }
    if value.trim() != value {
        return Err(WorkVcsError::TaskInvalid(format!(
            "{label} must not have leading or trailing whitespace"
        )));
    }
    if value
        .as_bytes()
        .iter()
        .any(|byte| *byte == b'\0' || *byte < 0x20 || *byte == 0x7f)
    {
        return Err(WorkVcsError::TaskInvalid(format!(
            "{label} must not contain NUL or ASCII control characters"
        )));
    }
    Ok(())
}

fn validate_acceptance_criterion_statement(value: &str) -> Result<()> {
    if value.trim().is_empty() {
        return Err(WorkVcsError::TaskInvalid(
            "acceptance criterion statement must not be empty".to_owned(),
        ));
    }
    if value.contains('\0') {
        return Err(WorkVcsError::TaskInvalid(
            "acceptance criterion statement must not contain NUL".to_owned(),
        ));
    }
    Ok(())
}

fn validate_verification_requirement_statement(value: &str) -> Result<()> {
    if value.trim().is_empty() {
        return Err(WorkVcsError::TaskInvalid(
            "verification requirement statement must not be empty".to_owned(),
        ));
    }
    if value.contains('\0') {
        return Err(WorkVcsError::TaskInvalid(
            "verification requirement statement must not contain NUL".to_owned(),
        ));
    }
    Ok(())
}

fn validate_verification_method(value: &CanonicalValue) -> Result<()> {
    match value {
        CanonicalValue::Object(_) => Ok(()),
        _ => Err(WorkVcsError::TaskInvalid(
            "verification method must be a canonical object descriptor".to_owned(),
        )),
    }
}

fn validate_verification_semantic_dependencies(
    dependencies: &[VerificationSemanticDependency],
) -> Result<()> {
    if dependencies.is_empty() {
        return Err(WorkVcsError::TaskInvalid(
            "verification semantic_dependencies must not be empty".to_owned(),
        ));
    }
    let mut entity_ids = HashSet::new();
    for dependency in dependencies {
        if !entity_ids.insert(dependency.entity_id.raw_bytes()) {
            return Err(WorkVcsError::TaskInvalid(format!(
                "verification semantic_dependencies contains duplicate entity id {}",
                dependency.entity_id
            )));
        }
    }
    Ok(())
}

fn validate_verification_evidence_refs(evidence: &[VerificationEvidenceRef]) -> Result<()> {
    let mut evidence_ids = HashSet::new();
    for entry in evidence {
        if !evidence_ids.insert(entry.evidence_id.raw_bytes()) {
            return Err(WorkVcsError::TaskInvalid(format!(
                "verification evidence contains duplicate evidence id {}",
                entry.evidence_id
            )));
        }
    }
    Ok(())
}

fn validate_verification_resource_basis(
    resource_basis: &[VerificationResourceBasis],
) -> Result<()> {
    for entry in resource_basis {
        validate_verification_resource_basis_entry(entry)?;
    }
    Ok(())
}

fn validate_verification_resource_basis_entry(
    resource_basis: &VerificationResourceBasis,
) -> Result<()> {
    validate_local_key(
        "verification resource basis adapter_kind",
        &resource_basis.adapter_kind,
    )?;
    validate_positive_i64(
        "verification resource basis adapter_schema_version",
        resource_basis.adapter_schema_version,
    )?;
    validate_local_key(
        "verification resource basis scope_kind",
        &resource_basis.scope_kind,
    )?;
    validate_positive_i64(
        "verification resource basis scope_schema_version",
        resource_basis.scope_schema_version,
    )?;
    require_verification_resource_scope_payload(&resource_basis.scope_payload)?;
    Ok(())
}

fn require_verification_resource_basis_references(
    connection: &StoreConnection,
    resource_basis: &VerificationResourceBasis,
) -> Result<()> {
    resource(connection, resource_basis.resource_id)?;
    if let Some(observation_id) = resource_basis.baseline_observation_id {
        let observation = resource_observation(connection, observation_id)?;
        if observation.resource_id != resource_basis.resource_id {
            return Err(WorkVcsError::TaskInvalid(format!(
                "verification resource basis observation {observation_id} belongs to resource {}, not {}",
                observation.resource_id, resource_basis.resource_id
            )));
        }
        if observation.adapter_kind != resource_basis.adapter_kind {
            return Err(WorkVcsError::TaskInvalid(format!(
                "verification resource basis observation {observation_id} adapter kind {:?} does not match basis adapter kind {:?}",
                observation.adapter_kind, resource_basis.adapter_kind
            )));
        }
        if observation.adapter_schema_version != resource_basis.adapter_schema_version {
            return Err(WorkVcsError::TaskInvalid(format!(
                "verification resource basis observation {observation_id} adapter schema version {} does not match basis adapter schema version {}",
                observation.adapter_schema_version, resource_basis.adapter_schema_version
            )));
        }
        if observation.fingerprint != resource_basis.baseline_fingerprint {
            return Err(WorkVcsError::TaskInvalid(format!(
                "verification resource basis observation {observation_id} fingerprint does not match baseline_fingerprint"
            )));
        }
    }
    Ok(())
}

fn validate_applicability_resource_stamp_inputs(
    stamps: &[ApplicabilityResourceStampInput],
) -> Result<()> {
    let mut ordinals = HashSet::new();
    for stamp in stamps {
        validate_applicability_resource_stamp_view(ResourceStampView::from(stamp))?;
        if !ordinals.insert(stamp.resource_basis_ordinal) {
            return Err(WorkVcsError::TaskInvalid(format!(
                "applicability resource stamps contain duplicate resource basis ordinal {}",
                stamp.resource_basis_ordinal
            )));
        }
    }
    Ok(())
}

fn validate_applicability_resource_stamp_input(
    stamp: &ApplicabilityResourceStampInput,
) -> Result<()> {
    validate_applicability_resource_stamp_view(ResourceStampView::from(stamp))
}

fn validate_applicability_resource_stamp_snapshot(
    stamp: &ApplicabilityResourceStampSnapshot,
) -> Result<()> {
    validate_applicability_resource_stamp_view(ResourceStampView::from(stamp))?;
    if stamp.observed_at_us < 0 {
        return Err(WorkVcsError::TaskInvalid(
            "applicability resource stamp observed_at_us must be non-negative".to_owned(),
        ));
    }
    Ok(())
}

fn validate_applicability_resource_stamp_view(stamp: ResourceStampView<'_>) -> Result<()> {
    if stamp.resource_basis_ordinal < 0 {
        return Err(WorkVcsError::TaskInvalid(
            "applicability resource stamp ordinal must be non-negative".to_owned(),
        ));
    }
    validate_local_key(
        "applicability resource stamp adapter_kind",
        stamp.adapter_kind,
    )?;
    validate_positive_i64(
        "applicability resource stamp adapter_schema_version",
        stamp.adapter_schema_version,
    )?;
    validate_positive_i64(
        "applicability resource stamp scope_schema_version",
        stamp.scope_schema_version,
    )?;
    match stamp.observation_status {
        ApplicabilityResourceObservationStatus::Observed => {
            if stamp.observed_fingerprint.is_none() {
                return Err(WorkVcsError::TaskInvalid(
                    "observed applicability resource stamp requires observed_fingerprint"
                        .to_owned(),
                ));
            }
        }
        ApplicabilityResourceObservationStatus::Unavailable
        | ApplicabilityResourceObservationStatus::Error => {
            if stamp.observed_fingerprint.is_some() || stamp.observation_id.is_some() {
                return Err(WorkVcsError::TaskInvalid(
                    "unavailable/error applicability resource stamp must not contain observed data"
                        .to_owned(),
                ));
            }
        }
    }
    Ok(())
}

fn validate_applicability_resource_stamp_inputs_against_basis(
    connection: &StoreConnection,
    resource_basis: &[VerificationResourceBasis],
    stamps: &[ApplicabilityResourceStampInput],
) -> Result<()> {
    validate_applicability_resource_stamps_against_basis(
        connection,
        resource_basis,
        stamps.iter().map(ResourceStampView::from),
    )
}

fn validate_applicability_resource_stamp_snapshots_against_basis(
    connection: &StoreConnection,
    resource_basis: &[VerificationResourceBasis],
    stamps: &[ApplicabilityResourceStampSnapshot],
) -> Result<()> {
    validate_applicability_resource_stamps_against_basis(
        connection,
        resource_basis,
        stamps.iter().map(ResourceStampView::from),
    )
}

fn validate_applicability_resource_stamps_against_basis<'a>(
    connection: &StoreConnection,
    resource_basis: &[VerificationResourceBasis],
    stamps: impl Iterator<Item = ResourceStampView<'a>>,
) -> Result<()> {
    let stamps = stamps.collect::<Vec<_>>();
    if stamps.len() != resource_basis.len() {
        return Err(WorkVcsError::TaskInvalid(format!(
            "applicability resource stamp count {} does not match verification resource basis count {}",
            stamps.len(),
            resource_basis.len()
        )));
    }
    for (index, (basis, stamp)) in resource_basis.iter().zip(stamps.iter()).enumerate() {
        if stamp.resource_basis_ordinal != index as i64 {
            return Err(WorkVcsError::TaskInvalid(format!(
                "applicability resource stamp ordinal {} does not match expected ordinal {index}",
                stamp.resource_basis_ordinal
            )));
        }
        if stamp.adapter_kind != basis.adapter_kind {
            return Err(WorkVcsError::TaskInvalid(format!(
                "applicability resource stamp ordinal {index} adapter kind {:?} does not match basis adapter kind {:?}",
                stamp.adapter_kind, basis.adapter_kind
            )));
        }
        if stamp.adapter_schema_version != basis.adapter_schema_version {
            return Err(WorkVcsError::TaskInvalid(format!(
                "applicability resource stamp ordinal {index} adapter schema version {} does not match basis adapter schema version {}",
                stamp.adapter_schema_version, basis.adapter_schema_version
            )));
        }
        if stamp.scope_schema_version != basis.scope_schema_version {
            return Err(WorkVcsError::TaskInvalid(format!(
                "applicability resource stamp ordinal {index} scope schema version {} does not match basis scope schema version {}",
                stamp.scope_schema_version, basis.scope_schema_version
            )));
        }
        if let Some(observation_id) = stamp.observation_id {
            let observation = resource_observation(connection, observation_id)?;
            if observation.resource_id != basis.resource_id {
                return Err(WorkVcsError::TaskInvalid(format!(
                    "applicability resource stamp observation {observation_id} belongs to resource {}, not {}",
                    observation.resource_id, basis.resource_id
                )));
            }
            if observation.adapter_kind != stamp.adapter_kind {
                return Err(WorkVcsError::TaskInvalid(format!(
                    "applicability resource stamp observation {observation_id} adapter kind {:?} does not match stamp adapter kind {:?}",
                    observation.adapter_kind, stamp.adapter_kind
                )));
            }
            if observation.adapter_schema_version != stamp.adapter_schema_version {
                return Err(WorkVcsError::TaskInvalid(format!(
                    "applicability resource stamp observation {observation_id} adapter schema version {} does not match stamp adapter schema version {}",
                    observation.adapter_schema_version, stamp.adapter_schema_version
                )));
            }
            if Some(observation.fingerprint) != stamp.observed_fingerprint {
                return Err(WorkVcsError::TaskInvalid(format!(
                    "applicability resource stamp observation {observation_id} fingerprint does not match observed_fingerprint"
                )));
            }
        }
    }
    Ok(())
}

fn require_verification_semantic_dependencies_canonical_order(
    dependencies: &[VerificationSemanticDependency],
) -> Result<()> {
    for pair in dependencies.windows(2) {
        let [left, right] = pair else {
            continue;
        };
        if left.entity_id.raw_bytes() >= right.entity_id.raw_bytes() {
            return Err(WorkVcsError::TaskInvalid(
                "verification semantic_dependencies must be sorted by raw entity id".to_owned(),
            ));
        }
    }
    Ok(())
}

fn validate_positive_i64(label: &str, value: i64) -> Result<()> {
    if value > 0 {
        Ok(())
    } else {
        Err(WorkVcsError::TaskInvalid(format!(
            "{label} must be positive"
        )))
    }
}

fn require_positive_i64(label: &str, value: CanonicalValue) -> Result<i64> {
    let CanonicalValue::Integer(value) = value else {
        return Err(WorkVcsError::TaskInvalid(format!(
            "{label} must be a JSON safe integer"
        )));
    };
    validate_positive_i64(label, value.get())?;
    Ok(value.get())
}

fn require_verification_resource_scope_payload(value: &CanonicalValue) -> Result<()> {
    match value {
        CanonicalValue::Object(_) => Ok(()),
        _ => Err(WorkVcsError::TaskInvalid(
            "verification resource basis scope_payload must be a canonical JSON object".to_owned(),
        )),
    }
}

fn normalize_verification_resource_scope_payload(value: CanonicalValue) -> Result<CanonicalValue> {
    require_verification_resource_scope_payload(&value)?;
    let encoded = canonical_json_string(&value)?;
    let normalized = parse_canonical_json(encoded.as_bytes()).map_err(task_invalid_from)?;
    require_verification_resource_scope_payload(&normalized)?;
    Ok(normalized)
}

fn parse_verification_resource_scope_payload_json(input: &str) -> Result<CanonicalValue> {
    let value = parse_canonical_json(input.as_bytes()).map_err(task_invalid_from)?;
    require_verification_resource_scope_payload(&value)?;
    let encoded = canonical_json_string(&value)?;
    if encoded != input {
        return Err(WorkVcsError::TaskInvalid(
            "verification_resource_basis.scope_payload_json is not canonical fixed-point JSON"
                .to_owned(),
        ));
    }
    Ok(value)
}

fn validate_description(value: &str) -> Result<()> {
    if value.trim().is_empty() {
        return Err(WorkVcsError::TaskInvalid(
            "task description must not be empty".to_owned(),
        ));
    }
    if value.contains('\0') {
        return Err(WorkVcsError::TaskInvalid(
            "task description must not contain NUL".to_owned(),
        ));
    }
    Ok(())
}

fn validate_outcome(value: &str) -> Result<()> {
    if value.contains('\0') {
        return Err(WorkVcsError::TaskInvalid(
            "task outcome must not contain NUL".to_owned(),
        ));
    }
    Ok(())
}

fn validate_lifecycle_transition(
    current_status: TaskStatus,
    next_status: TaskStatus,
    rationale: &CanonicalValue,
) -> Result<()> {
    if next_status == TaskStatus::Superseded {
        return Err(WorkVcsError::TaskInvalid(
            "ordinary task transition to superseded requires supersession-aware resolution"
                .to_owned(),
        ));
    }
    if current_status == TaskStatus::Superseded {
        return Err(WorkVcsError::TaskInvalid(
            "ordinary task transition from superseded requires supersession-aware resolution"
                .to_owned(),
        ));
    }
    if current_status.allows_rationale_reentry() && next_status.is_non_terminal() {
        require_non_empty_rationale_object(rationale)?;
        return Ok(());
    }
    if current_status.allows_rationale_reentry() && current_status != next_status {
        return Err(WorkVcsError::TaskInvalid(format!(
            "terminal task status {current_status} cannot transition directly to {next_status}"
        )));
    }
    if next_status == TaskStatus::Cancelled && current_status != TaskStatus::Cancelled {
        require_non_empty_rationale_object(rationale)?;
    }
    Ok(())
}

fn require_non_empty_rationale_object(value: &CanonicalValue) -> Result<()> {
    match value {
        CanonicalValue::Object(entries) if !entries.is_empty() => Ok(()),
        CanonicalValue::Object(_) => Err(WorkVcsError::TaskInvalid(
            "terminal task re-entry requires a non-empty rationale object".to_owned(),
        )),
        _ => Err(WorkVcsError::TaskInvalid(
            "terminal task re-entry requires a rationale object".to_owned(),
        )),
    }
}

fn validate_priority(value: i64) -> Result<()> {
    CanonicalValue::safe_integer(value)
        .map(|_| ())
        .map_err(task_invalid_from)
}

fn task_invalid_from(error: WorkVcsError) -> WorkVcsError {
    WorkVcsError::TaskInvalid(error.to_string())
}

fn decode_workspace_id(column: &str, bytes: Vec<u8>) -> Result<WorkspaceId> {
    let bytes = decode_16(column, bytes)?;
    WorkspaceId::from_bytes(bytes).map_err(task_invalid_from)
}

fn decode_commit_id(column: &str, bytes: Vec<u8>) -> Result<CommitId> {
    let bytes = decode_16(column, bytes)?;
    CommitId::from_bytes(bytes).map_err(task_invalid_from)
}

fn decode_entity_id(column: &str, bytes: Vec<u8>) -> Result<EntityId> {
    let bytes = decode_16(column, bytes)?;
    EntityId::from_bytes(bytes).map_err(task_invalid_from)
}

fn decode_evidence_id(column: &str, bytes: Vec<u8>) -> Result<EvidenceId> {
    let bytes = decode_16(column, bytes)?;
    EvidenceId::from_bytes(bytes).map_err(task_invalid_from)
}

fn decode_resource_id(column: &str, bytes: Vec<u8>) -> Result<ResourceId> {
    let bytes = decode_16(column, bytes)?;
    ResourceId::from_bytes(bytes).map_err(task_invalid_from)
}

fn decode_resource_observation_id(column: &str, bytes: Vec<u8>) -> Result<ResourceObservationId> {
    let bytes = decode_16(column, bytes)?;
    ResourceObservationId::from_bytes(bytes).map_err(task_invalid_from)
}

fn decode_entity_version_id(column: &str, bytes: Vec<u8>) -> Result<EntityVersionId> {
    let bytes = decode_16(column, bytes)?;
    EntityVersionId::from_bytes(bytes).map_err(task_invalid_from)
}

fn decode_16(column: &str, bytes: Vec<u8>) -> Result<[u8; 16]> {
    bytes.try_into().map_err(|bytes: Vec<u8>| {
        WorkVcsError::TaskInvalid(format!("{column} must be 16 bytes, found {}", bytes.len()))
    })
}

fn decode_digest(column: &str, bytes: Vec<u8>) -> Result<Digest> {
    let bytes = bytes.try_into().map_err(|bytes: Vec<u8>| {
        WorkVcsError::TaskInvalid(format!("{column} must be 32 bytes, found {}", bytes.len()))
    })?;
    Ok(Digest::from_bytes(bytes))
}
