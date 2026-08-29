use super::entity::{canonical_json_string, entity_transition_payload_json};
use super::{EntityTransitionOptions, commit_entity_transition, state_at};
use crate::canonical::{
    CanonicalValue, ImportDigestDomain, WorkState, entity_version_digest, parse_canonical_json,
    relation_version_digest, validate_import_fixed_point, work_state_mapping_digest,
};
use crate::error::{Result, WorkVcsError, storage_error};
use crate::identity::{
    BranchId, ChangeSetId, CommitId, Digest, EntityId, EntityVersionId, EventId, OperationId,
    RelationId, RelationVersionId, WorkspaceId,
};
use crate::store::{StoreConnection, current_epoch_micros};
use rusqlite::{OptionalExtension, Transaction, TransactionBehavior, params};
use std::collections::BTreeMap;
use std::fmt;

pub(crate) const RECORD_ENTITY_KIND: &str = "record";
pub(crate) const RECORD_DECISION_SUPERSEDE_OPERATION_SCHEMA_VERSION: i64 = 1;
pub(crate) const RECORD_DECISION_SUPERSEDE_OPERATION_TYPE: &str = "record.decision.supersede";
pub(crate) const RECORD_RELATION_CREATE_OPERATION_SCHEMA_VERSION: i64 = 1;
pub(crate) const RECORD_RELATION_CREATE_OPERATION_TYPE: &str = "record.relation.create";
pub(crate) const RECORD_RELATION_REMOVE_OPERATION_SCHEMA_VERSION: i64 = 1;
pub(crate) const RECORD_RELATION_REMOVE_OPERATION_TYPE: &str = "record.relation.remove";
pub(crate) const RECORD_RELATION_RESTORE_OPERATION_SCHEMA_VERSION: i64 = 1;
pub(crate) const RECORD_RELATION_RESTORE_OPERATION_TYPE: &str = "record.relation.restore";

const ENTITY_OBJECT_KIND: &str = "entity";
const ACTIVE_BRANCH_LIFECYCLE_STATE: &str = "active";
const CONTRADICTS_RELATION_TYPE: &str = "contradicts";
const DERIVED_FROM_RELATION_TYPE: &str = "derived_from";
const EMPTY_FIELD_DELTA: &str = "{}";
const INVALIDATES_RELATION_TYPE: &str = "invalidates";
const NORMAL_COMMIT_KIND: &str = "normal";
const PRIMARY_PARENT_ROLE: &str = "primary";
const RECORD_STATE_SCHEMA_VERSION: i64 = 1;
const RECORD_RELATION_CREATE_EVENT_KIND: &str = "record.relation.created";
const RECORD_RELATION_REMOVE_EVENT_KIND: &str = "record.relation.removed";
const RECORD_RELATION_RESTORE_EVENT_KIND: &str = "record.relation.restored";
const RELATED_TO_RELATION_TYPE: &str = "related_to";
const RELATION_OBJECT_KIND: &str = "relation";
const RELATION_STATE_SCHEMA_VERSION: i64 = 1;
const SUPPORTS_RELATION_TYPE: &str = "supports";
const SUPERSEDES_RELATION_TYPE: &str = "supersedes";
const VALIDATES_RELATION_TYPE: &str = "validates";
const RECORD_DECISION_SUPERSEDE_EVENT_KIND: &str = "record.decision.superseded";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RecordKind {
    Assumption,
    Attempt,
    Decision,
    Finding,
    Handoff,
    Question,
    Risk,
}

impl RecordKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Assumption => "assumption",
            Self::Attempt => "attempt",
            Self::Decision => "decision",
            Self::Finding => "finding",
            Self::Handoff => "handoff",
            Self::Question => "question",
            Self::Risk => "risk",
        }
    }

    fn parse(value: &str) -> Result<Self> {
        match value {
            "assumption" => Ok(Self::Assumption),
            "attempt" => Ok(Self::Attempt),
            "decision" => Ok(Self::Decision),
            "finding" => Ok(Self::Finding),
            "handoff" => Ok(Self::Handoff),
            "question" => Ok(Self::Question),
            "risk" => Ok(Self::Risk),
            other => Err(WorkVcsError::RecordInvalid(format!(
                "record kind {other:?} is not implemented by the semantic Record API"
            ))),
        }
    }
}

impl fmt::Display for RecordKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RecordStatus {
    Active,
    Failed,
    Inconclusive,
    Invalidated,
    Running,
    Succeeded,
    Superseded,
    Unverified,
    Validated,
    Withdrawn,
}

impl RecordStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Failed => "failed",
            Self::Inconclusive => "inconclusive",
            Self::Invalidated => "invalidated",
            Self::Running => "running",
            Self::Succeeded => "succeeded",
            Self::Superseded => "superseded",
            Self::Unverified => "unverified",
            Self::Validated => "validated",
            Self::Withdrawn => "withdrawn",
        }
    }

    fn parse(value: &str) -> Result<Self> {
        match value {
            "active" => Ok(Self::Active),
            "failed" => Ok(Self::Failed),
            "inconclusive" => Ok(Self::Inconclusive),
            "invalidated" => Ok(Self::Invalidated),
            "running" => Ok(Self::Running),
            "succeeded" => Ok(Self::Succeeded),
            "superseded" => Ok(Self::Superseded),
            "unverified" => Ok(Self::Unverified),
            "validated" => Ok(Self::Validated),
            "withdrawn" => Ok(Self::Withdrawn),
            other => Err(WorkVcsError::RecordInvalid(format!(
                "record status {other:?} is not in the semantic Record lifecycle vocabulary"
            ))),
        }
    }
}

impl fmt::Display for RecordStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecordState {
    pub kind: RecordKind,
    pub statement: String,
    pub scope: CanonicalValue,
    pub status: RecordStatus,
}

impl RecordState {
    pub fn handoff(statement: impl Into<String>) -> Result<Self> {
        let statement = statement.into();
        validate_statement(&statement)?;
        Ok(Self {
            kind: RecordKind::Handoff,
            statement,
            scope: CanonicalValue::object(Vec::new())?,
            status: RecordStatus::Active,
        })
    }

    pub fn attempt(statement: impl Into<String>) -> Result<Self> {
        let statement = statement.into();
        validate_statement(&statement)?;
        Ok(Self {
            kind: RecordKind::Attempt,
            statement,
            scope: CanonicalValue::object(Vec::new())?,
            status: RecordStatus::Running,
        })
    }

    pub fn question(statement: impl Into<String>) -> Result<Self> {
        let statement = statement.into();
        validate_statement(&statement)?;
        Ok(Self {
            kind: RecordKind::Question,
            statement,
            scope: CanonicalValue::object(Vec::new())?,
            status: RecordStatus::Active,
        })
    }

    pub fn risk(statement: impl Into<String>) -> Result<Self> {
        let statement = statement.into();
        validate_statement(&statement)?;
        Ok(Self {
            kind: RecordKind::Risk,
            statement,
            scope: CanonicalValue::object(Vec::new())?,
            status: RecordStatus::Active,
        })
    }

    pub fn decision(statement: impl Into<String>) -> Result<Self> {
        let statement = statement.into();
        validate_statement(&statement)?;
        Ok(Self {
            kind: RecordKind::Decision,
            statement,
            scope: CanonicalValue::object(Vec::new())?,
            status: RecordStatus::Active,
        })
    }

    pub fn assumption(statement: impl Into<String>) -> Result<Self> {
        let statement = statement.into();
        validate_statement(&statement)?;
        Ok(Self {
            kind: RecordKind::Assumption,
            statement,
            scope: CanonicalValue::object(Vec::new())?,
            status: RecordStatus::Unverified,
        })
    }

    pub fn finding(statement: impl Into<String>) -> Result<Self> {
        let statement = statement.into();
        validate_statement(&statement)?;
        Ok(Self {
            kind: RecordKind::Finding,
            statement,
            scope: CanonicalValue::object(Vec::new())?,
            status: RecordStatus::Active,
        })
    }

    pub fn with_scope(mut self, scope: CanonicalValue) -> Result<Self> {
        require_object_value("record scope", &scope)?;
        self.scope = scope;
        Ok(self)
    }

    pub fn to_canonical_value(&self) -> Result<CanonicalValue> {
        validate_statement(&self.statement)?;
        require_object_value("record scope", &self.scope)?;
        validate_record_status_for_kind(self.kind, self.status)?;
        CanonicalValue::object(vec![
            (
                "kind".to_owned(),
                CanonicalValue::String(self.kind.as_str().to_owned()),
            ),
            ("scope".to_owned(), self.scope.clone()),
            (
                "statement".to_owned(),
                CanonicalValue::String(self.statement.clone()),
            ),
            (
                "status".to_owned(),
                CanonicalValue::String(self.status.as_str().to_owned()),
            ),
        ])
    }

    fn transition_assumption(
        &self,
        next_status: RecordStatus,
        rationale_text: &str,
    ) -> Result<Self> {
        if self.kind != RecordKind::Assumption {
            return Err(WorkVcsError::RecordInvalid(format!(
                "record kind {:?} does not use the Assumption lifecycle",
                self.kind
            )));
        }
        validate_transition_rationale(rationale_text)?;
        validate_assumption_lifecycle_transition(self.status, next_status)?;
        let next = Self {
            kind: self.kind,
            statement: self.statement.clone(),
            scope: self.scope.clone(),
            status: next_status,
        };
        if next == *self {
            return Err(WorkVcsError::RecordInvalid(
                "record transition must change status".to_owned(),
            ));
        }
        Ok(next)
    }

    fn transition_attempt(&self, next_status: RecordStatus, rationale_text: &str) -> Result<Self> {
        if self.kind != RecordKind::Attempt {
            return Err(WorkVcsError::RecordInvalid(format!(
                "record kind {:?} does not use the Attempt lifecycle",
                self.kind
            )));
        }
        validate_transition_rationale(rationale_text)?;
        validate_attempt_lifecycle_transition(self.status, next_status)?;
        let next = Self {
            kind: self.kind,
            statement: self.statement.clone(),
            scope: self.scope.clone(),
            status: next_status,
        };
        Ok(next)
    }

    fn transition_decision(&self, next_status: RecordStatus, rationale_text: &str) -> Result<Self> {
        if self.kind != RecordKind::Decision {
            return Err(WorkVcsError::RecordInvalid(format!(
                "record kind {:?} does not use the Decision lifecycle",
                self.kind
            )));
        }
        validate_transition_rationale(rationale_text)?;
        validate_decision_lifecycle_transition(self.status, next_status)?;
        let next = Self {
            kind: self.kind,
            statement: self.statement.clone(),
            scope: self.scope.clone(),
            status: next_status,
        };
        Ok(next)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecordCreateOptions {
    branch_id: BranchId,
    expected_head_commit_id: CommitId,
    state: RecordState,
    rationale: CanonicalValue,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecordTransitionOptions {
    branch_id: BranchId,
    expected_head_commit_id: CommitId,
    record_entity_id: EntityId,
    expected_record_entity_version_id: EntityVersionId,
    next_status: RecordStatus,
    rationale_text: String,
    rationale: CanonicalValue,
}

impl RecordTransitionOptions {
    pub fn complete_attempt_succeeded(
        branch_id: BranchId,
        expected_head_commit_id: CommitId,
        record_entity_id: EntityId,
        expected_record_entity_version_id: EntityVersionId,
        rationale: impl Into<String>,
    ) -> Result<Self> {
        Self::attempt_transition(
            branch_id,
            expected_head_commit_id,
            record_entity_id,
            expected_record_entity_version_id,
            RecordStatus::Succeeded,
            rationale,
        )
    }

    pub fn complete_attempt_failed(
        branch_id: BranchId,
        expected_head_commit_id: CommitId,
        record_entity_id: EntityId,
        expected_record_entity_version_id: EntityVersionId,
        rationale: impl Into<String>,
    ) -> Result<Self> {
        Self::attempt_transition(
            branch_id,
            expected_head_commit_id,
            record_entity_id,
            expected_record_entity_version_id,
            RecordStatus::Failed,
            rationale,
        )
    }

    pub fn complete_attempt_inconclusive(
        branch_id: BranchId,
        expected_head_commit_id: CommitId,
        record_entity_id: EntityId,
        expected_record_entity_version_id: EntityVersionId,
        rationale: impl Into<String>,
    ) -> Result<Self> {
        Self::attempt_transition(
            branch_id,
            expected_head_commit_id,
            record_entity_id,
            expected_record_entity_version_id,
            RecordStatus::Inconclusive,
            rationale,
        )
    }

    pub fn validate_assumption(
        branch_id: BranchId,
        expected_head_commit_id: CommitId,
        record_entity_id: EntityId,
        expected_record_entity_version_id: EntityVersionId,
        rationale: impl Into<String>,
    ) -> Result<Self> {
        Self::assumption_transition(
            branch_id,
            expected_head_commit_id,
            record_entity_id,
            expected_record_entity_version_id,
            RecordStatus::Validated,
            rationale,
        )
    }

    pub fn invalidate_assumption(
        branch_id: BranchId,
        expected_head_commit_id: CommitId,
        record_entity_id: EntityId,
        expected_record_entity_version_id: EntityVersionId,
        rationale: impl Into<String>,
    ) -> Result<Self> {
        Self::assumption_transition(
            branch_id,
            expected_head_commit_id,
            record_entity_id,
            expected_record_entity_version_id,
            RecordStatus::Invalidated,
            rationale,
        )
    }

    pub fn supersede_decision(
        branch_id: BranchId,
        expected_head_commit_id: CommitId,
        record_entity_id: EntityId,
        expected_record_entity_version_id: EntityVersionId,
        rationale: impl Into<String>,
    ) -> Result<Self> {
        Self::decision_transition(
            branch_id,
            expected_head_commit_id,
            record_entity_id,
            expected_record_entity_version_id,
            RecordStatus::Superseded,
            rationale,
        )
    }

    pub fn withdraw_decision(
        branch_id: BranchId,
        expected_head_commit_id: CommitId,
        record_entity_id: EntityId,
        expected_record_entity_version_id: EntityVersionId,
        rationale: impl Into<String>,
    ) -> Result<Self> {
        Self::decision_transition(
            branch_id,
            expected_head_commit_id,
            record_entity_id,
            expected_record_entity_version_id,
            RecordStatus::Withdrawn,
            rationale,
        )
    }

    fn assumption_transition(
        branch_id: BranchId,
        expected_head_commit_id: CommitId,
        record_entity_id: EntityId,
        expected_record_entity_version_id: EntityVersionId,
        next_status: RecordStatus,
        rationale: impl Into<String>,
    ) -> Result<Self> {
        let rationale_text = rationale.into();
        validate_transition_rationale(&rationale_text)?;
        Ok(Self {
            branch_id,
            expected_head_commit_id,
            record_entity_id,
            expected_record_entity_version_id,
            next_status,
            rationale: rationale_value(&rationale_text)?,
            rationale_text,
        })
    }

    fn decision_transition(
        branch_id: BranchId,
        expected_head_commit_id: CommitId,
        record_entity_id: EntityId,
        expected_record_entity_version_id: EntityVersionId,
        next_status: RecordStatus,
        rationale: impl Into<String>,
    ) -> Result<Self> {
        let rationale_text = rationale.into();
        validate_transition_rationale(&rationale_text)?;
        Ok(Self {
            branch_id,
            expected_head_commit_id,
            record_entity_id,
            expected_record_entity_version_id,
            next_status,
            rationale: rationale_value(&rationale_text)?,
            rationale_text,
        })
    }

    fn attempt_transition(
        branch_id: BranchId,
        expected_head_commit_id: CommitId,
        record_entity_id: EntityId,
        expected_record_entity_version_id: EntityVersionId,
        next_status: RecordStatus,
        rationale: impl Into<String>,
    ) -> Result<Self> {
        let rationale_text = rationale.into();
        validate_transition_rationale(&rationale_text)?;
        Ok(Self {
            branch_id,
            expected_head_commit_id,
            record_entity_id,
            expected_record_entity_version_id,
            next_status,
            rationale: rationale_value(&rationale_text)?,
            rationale_text,
        })
    }

    pub fn with_rationale(mut self, rationale: CanonicalValue) -> Self {
        self.rationale = rationale;
        self
    }
}

impl RecordCreateOptions {
    pub fn handoff(
        branch_id: BranchId,
        expected_head_commit_id: CommitId,
        statement: impl Into<String>,
    ) -> Result<Self> {
        Ok(Self {
            branch_id,
            expected_head_commit_id,
            state: RecordState::handoff(statement)?,
            rationale: CanonicalValue::object(Vec::new())?,
        })
    }

    pub fn attempt(
        branch_id: BranchId,
        expected_head_commit_id: CommitId,
        statement: impl Into<String>,
    ) -> Result<Self> {
        Ok(Self {
            branch_id,
            expected_head_commit_id,
            state: RecordState::attempt(statement)?,
            rationale: CanonicalValue::object(Vec::new())?,
        })
    }

    pub fn question(
        branch_id: BranchId,
        expected_head_commit_id: CommitId,
        statement: impl Into<String>,
    ) -> Result<Self> {
        Ok(Self {
            branch_id,
            expected_head_commit_id,
            state: RecordState::question(statement)?,
            rationale: CanonicalValue::object(Vec::new())?,
        })
    }

    pub fn risk(
        branch_id: BranchId,
        expected_head_commit_id: CommitId,
        statement: impl Into<String>,
    ) -> Result<Self> {
        Ok(Self {
            branch_id,
            expected_head_commit_id,
            state: RecordState::risk(statement)?,
            rationale: CanonicalValue::object(Vec::new())?,
        })
    }

    pub fn decision(
        branch_id: BranchId,
        expected_head_commit_id: CommitId,
        statement: impl Into<String>,
    ) -> Result<Self> {
        Ok(Self {
            branch_id,
            expected_head_commit_id,
            state: RecordState::decision(statement)?,
            rationale: CanonicalValue::object(Vec::new())?,
        })
    }

    pub fn assumption(
        branch_id: BranchId,
        expected_head_commit_id: CommitId,
        statement: impl Into<String>,
    ) -> Result<Self> {
        Ok(Self {
            branch_id,
            expected_head_commit_id,
            state: RecordState::assumption(statement)?,
            rationale: CanonicalValue::object(Vec::new())?,
        })
    }

    pub fn finding(
        branch_id: BranchId,
        expected_head_commit_id: CommitId,
        statement: impl Into<String>,
    ) -> Result<Self> {
        Ok(Self {
            branch_id,
            expected_head_commit_id,
            state: RecordState::finding(statement)?,
            rationale: CanonicalValue::object(Vec::new())?,
        })
    }

    pub fn with_scope(mut self, scope: CanonicalValue) -> Result<Self> {
        self.state = self.state.with_scope(scope)?;
        Ok(self)
    }

    pub fn with_rationale(mut self, rationale: CanonicalValue) -> Self {
        self.rationale = rationale;
        self
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecordCreateCommit {
    pub workspace_id: WorkspaceId,
    pub branch_id: BranchId,
    pub previous_head_commit_id: CommitId,
    pub commit_id: CommitId,
    pub changeset_id: ChangeSetId,
    pub operation_id: OperationId,
    pub record_entity_id: EntityId,
    pub record_entity_version_id: EntityVersionId,
    pub record_state_digest: Digest,
    pub work_state_digest: Digest,
    pub state: RecordState,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecordTransitionCommit {
    pub workspace_id: WorkspaceId,
    pub branch_id: BranchId,
    pub previous_head_commit_id: CommitId,
    pub commit_id: CommitId,
    pub changeset_id: ChangeSetId,
    pub operation_id: OperationId,
    pub record_entity_id: EntityId,
    pub previous_record_entity_version_id: EntityVersionId,
    pub record_entity_version_id: EntityVersionId,
    pub record_state_digest: Digest,
    pub work_state_digest: Digest,
    pub previous_state: RecordState,
    pub state: RecordState,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecordSnapshot {
    pub workspace_id: WorkspaceId,
    pub commit_id: CommitId,
    pub record_entity_id: EntityId,
    pub record_entity_version_id: EntityVersionId,
    pub state_digest: Digest,
    pub state: RecordState,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecordListOptions {
    commit_id: CommitId,
    kind: Option<RecordKind>,
    status: Option<RecordStatus>,
    statement_contains: Option<String>,
}

impl RecordListOptions {
    pub fn new(commit_id: CommitId) -> Self {
        Self {
            commit_id,
            kind: None,
            status: None,
            statement_contains: None,
        }
    }

    pub fn with_kind(mut self, kind: RecordKind) -> Self {
        self.kind = Some(kind);
        self
    }

    pub fn with_status(mut self, status: RecordStatus) -> Self {
        self.status = Some(status);
        self
    }

    pub fn with_statement_contains(mut self, fragment: impl Into<String>) -> Result<Self> {
        let fragment = fragment.into();
        if fragment.trim().is_empty() {
            return Err(WorkVcsError::RecordInvalid(
                "record statement filter must not be empty".to_owned(),
            ));
        }
        self.statement_contains = Some(fragment);
        Ok(self)
    }

    pub fn commit_id(&self) -> CommitId {
        self.commit_id
    }

    pub fn kind(&self) -> Option<RecordKind> {
        self.kind
    }

    pub fn status(&self) -> Option<RecordStatus> {
        self.status
    }

    pub fn statement_contains(&self) -> Option<&str> {
        self.statement_contains.as_deref()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecordListResult {
    pub workspace_id: WorkspaceId,
    pub commit_id: CommitId,
    pub records: Vec<RecordSnapshot>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum RecordRelationType {
    Contradicts,
    DerivedFrom,
    Invalidates,
    RelatedTo,
    Supersedes,
    Supports,
    Validates,
}

impl RecordRelationType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Contradicts => CONTRADICTS_RELATION_TYPE,
            Self::DerivedFrom => DERIVED_FROM_RELATION_TYPE,
            Self::Invalidates => INVALIDATES_RELATION_TYPE,
            Self::RelatedTo => RELATED_TO_RELATION_TYPE,
            Self::Supersedes => SUPERSEDES_RELATION_TYPE,
            Self::Supports => SUPPORTS_RELATION_TYPE,
            Self::Validates => VALIDATES_RELATION_TYPE,
        }
    }

    fn parse(value: &str) -> Option<Self> {
        match value {
            CONTRADICTS_RELATION_TYPE => Some(Self::Contradicts),
            DERIVED_FROM_RELATION_TYPE => Some(Self::DerivedFrom),
            INVALIDATES_RELATION_TYPE => Some(Self::Invalidates),
            RELATED_TO_RELATION_TYPE => Some(Self::RelatedTo),
            SUPERSEDES_RELATION_TYPE => Some(Self::Supersedes),
            SUPPORTS_RELATION_TYPE => Some(Self::Supports),
            VALIDATES_RELATION_TYPE => Some(Self::Validates),
            _ => None,
        }
    }
}

impl fmt::Display for RecordRelationType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecordRelationCreateOptions {
    branch_id: BranchId,
    expected_head_commit_id: CommitId,
    relation_type: RecordRelationType,
    source_record_entity_id: EntityId,
    target_record_entity_id: EntityId,
    relation_label: Option<String>,
    rationale: CanonicalValue,
}

impl RecordRelationCreateOptions {
    pub fn contradicts(
        branch_id: BranchId,
        expected_head_commit_id: CommitId,
        source_record_entity_id: EntityId,
        target_record_entity_id: EntityId,
        rationale: impl Into<String>,
    ) -> Result<Self> {
        let rationale = rationale.into();
        validate_transition_rationale(&rationale)?;
        Ok(Self {
            branch_id,
            expected_head_commit_id,
            relation_type: RecordRelationType::Contradicts,
            source_record_entity_id,
            target_record_entity_id,
            relation_label: None,
            rationale: rationale_value(&rationale)?,
        })
    }

    pub fn invalidates(
        branch_id: BranchId,
        expected_head_commit_id: CommitId,
        source_record_entity_id: EntityId,
        target_record_entity_id: EntityId,
        rationale: impl Into<String>,
    ) -> Result<Self> {
        let rationale = rationale.into();
        validate_transition_rationale(&rationale)?;
        Ok(Self {
            branch_id,
            expected_head_commit_id,
            relation_type: RecordRelationType::Invalidates,
            source_record_entity_id,
            target_record_entity_id,
            relation_label: None,
            rationale: rationale_value(&rationale)?,
        })
    }

    pub fn derived_from(
        branch_id: BranchId,
        expected_head_commit_id: CommitId,
        result_record_entity_id: EntityId,
        source_record_entity_id: EntityId,
        rationale: impl Into<String>,
    ) -> Result<Self> {
        let rationale = rationale.into();
        validate_transition_rationale(&rationale)?;
        Ok(Self {
            branch_id,
            expected_head_commit_id,
            relation_type: RecordRelationType::DerivedFrom,
            source_record_entity_id: result_record_entity_id,
            target_record_entity_id: source_record_entity_id,
            relation_label: None,
            rationale: rationale_value(&rationale)?,
        })
    }

    pub fn related_to(
        branch_id: BranchId,
        expected_head_commit_id: CommitId,
        source_record_entity_id: EntityId,
        target_record_entity_id: EntityId,
        label: impl Into<String>,
        rationale: impl Into<String>,
    ) -> Result<Self> {
        let label = normalize_relation_label(&label.into())?;
        let rationale = rationale.into();
        validate_transition_rationale(&rationale)?;
        Ok(Self {
            branch_id,
            expected_head_commit_id,
            relation_type: RecordRelationType::RelatedTo,
            source_record_entity_id,
            target_record_entity_id,
            relation_label: Some(label),
            rationale: rationale_value(&rationale)?,
        })
    }

    pub fn supports(
        branch_id: BranchId,
        expected_head_commit_id: CommitId,
        source_record_entity_id: EntityId,
        target_record_entity_id: EntityId,
        rationale: impl Into<String>,
    ) -> Result<Self> {
        let rationale = rationale.into();
        validate_transition_rationale(&rationale)?;
        Ok(Self {
            branch_id,
            expected_head_commit_id,
            relation_type: RecordRelationType::Supports,
            source_record_entity_id,
            target_record_entity_id,
            relation_label: None,
            rationale: rationale_value(&rationale)?,
        })
    }

    pub fn validates(
        branch_id: BranchId,
        expected_head_commit_id: CommitId,
        source_record_entity_id: EntityId,
        target_record_entity_id: EntityId,
        rationale: impl Into<String>,
    ) -> Result<Self> {
        let rationale = rationale.into();
        validate_transition_rationale(&rationale)?;
        Ok(Self {
            branch_id,
            expected_head_commit_id,
            relation_type: RecordRelationType::Validates,
            source_record_entity_id,
            target_record_entity_id,
            relation_label: None,
            rationale: rationale_value(&rationale)?,
        })
    }

    pub fn with_rationale(mut self, rationale: CanonicalValue) -> Self {
        self.rationale = rationale;
        self
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecordRelationCreateCommit {
    pub workspace_id: WorkspaceId,
    pub branch_id: BranchId,
    pub previous_head_commit_id: CommitId,
    pub commit_id: CommitId,
    pub changeset_id: ChangeSetId,
    pub operation_id: OperationId,
    pub relation_id: RelationId,
    pub relation_version_id: RelationVersionId,
    pub relation_type: RecordRelationType,
    pub relation_label: Option<String>,
    pub source_record_entity_id: EntityId,
    pub target_record_entity_id: EntityId,
    pub relation_state_digest: Digest,
    pub work_state_digest: Digest,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DecisionRecordSupersedeOptions {
    branch_id: BranchId,
    expected_head_commit_id: CommitId,
    replacement_record_entity_id: EntityId,
    prior_record_entity_id: EntityId,
    expected_prior_record_entity_version_id: EntityVersionId,
    causal_record_entity_id: Option<EntityId>,
    rationale_text: String,
    rationale: CanonicalValue,
}

impl DecisionRecordSupersedeOptions {
    pub fn new(
        branch_id: BranchId,
        expected_head_commit_id: CommitId,
        replacement_record_entity_id: EntityId,
        prior_record_entity_id: EntityId,
        expected_prior_record_entity_version_id: EntityVersionId,
        rationale: impl Into<String>,
    ) -> Result<Self> {
        let rationale_text = rationale.into();
        validate_transition_rationale(&rationale_text)?;
        Ok(Self {
            branch_id,
            expected_head_commit_id,
            replacement_record_entity_id,
            prior_record_entity_id,
            expected_prior_record_entity_version_id,
            causal_record_entity_id: None,
            rationale: rationale_value(&rationale_text)?,
            rationale_text,
        })
    }

    pub fn with_causal_record(mut self, causal_record_entity_id: EntityId) -> Self {
        self.causal_record_entity_id = Some(causal_record_entity_id);
        self
    }

    pub fn with_rationale(mut self, rationale: CanonicalValue) -> Self {
        self.rationale = rationale;
        self
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DecisionRecordSupersedeCommit {
    pub workspace_id: WorkspaceId,
    pub branch_id: BranchId,
    pub previous_head_commit_id: CommitId,
    pub commit_id: CommitId,
    pub changeset_id: ChangeSetId,
    pub prior_record_operation_id: OperationId,
    pub relation_operation_id: OperationId,
    pub replacement_record_entity_id: EntityId,
    pub prior_record_entity_id: EntityId,
    pub previous_prior_record_entity_version_id: EntityVersionId,
    pub prior_record_entity_version_id: EntityVersionId,
    pub prior_record_state_digest: Digest,
    pub relation_id: RelationId,
    pub relation_version_id: RelationVersionId,
    pub relation_state_digest: Digest,
    pub causal_record_entity_id: Option<EntityId>,
    pub causal_relation_operation_id: Option<OperationId>,
    pub causal_relation_id: Option<RelationId>,
    pub causal_relation_version_id: Option<RelationVersionId>,
    pub causal_relation_state_digest: Option<Digest>,
    pub work_state_digest: Digest,
    pub previous_prior_state: RecordState,
    pub prior_state: RecordState,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecordRelationRemoveOptions {
    branch_id: BranchId,
    expected_head_commit_id: CommitId,
    relation_id: RelationId,
    expected_relation_version_id: RelationVersionId,
    rationale: CanonicalValue,
}

impl RecordRelationRemoveOptions {
    pub fn new(
        branch_id: BranchId,
        expected_head_commit_id: CommitId,
        relation_id: RelationId,
        expected_relation_version_id: RelationVersionId,
        rationale: impl Into<String>,
    ) -> Result<Self> {
        let rationale = rationale.into();
        validate_transition_rationale(&rationale)?;
        Ok(Self {
            branch_id,
            expected_head_commit_id,
            relation_id,
            expected_relation_version_id,
            rationale: rationale_value(&rationale)?,
        })
    }

    pub fn with_rationale(mut self, rationale: CanonicalValue) -> Self {
        self.rationale = rationale;
        self
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecordRelationRemoveCommit {
    pub workspace_id: WorkspaceId,
    pub branch_id: BranchId,
    pub previous_head_commit_id: CommitId,
    pub commit_id: CommitId,
    pub changeset_id: ChangeSetId,
    pub operation_id: OperationId,
    pub relation_id: RelationId,
    pub previous_relation_version_id: RelationVersionId,
    pub relation_type: RecordRelationType,
    pub relation_label: Option<String>,
    pub source_record_entity_id: EntityId,
    pub target_record_entity_id: EntityId,
    pub work_state_digest: Digest,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecordRelationRestoreOptions {
    branch_id: BranchId,
    expected_head_commit_id: CommitId,
    relation_id: RelationId,
    relation_version_id: RelationVersionId,
    rationale: CanonicalValue,
}

impl RecordRelationRestoreOptions {
    pub fn new(
        branch_id: BranchId,
        expected_head_commit_id: CommitId,
        relation_id: RelationId,
        relation_version_id: RelationVersionId,
        rationale: impl Into<String>,
    ) -> Result<Self> {
        let rationale = rationale.into();
        validate_transition_rationale(&rationale)?;
        Ok(Self {
            branch_id,
            expected_head_commit_id,
            relation_id,
            relation_version_id,
            rationale: rationale_value(&rationale)?,
        })
    }

    pub fn with_rationale(mut self, rationale: CanonicalValue) -> Self {
        self.rationale = rationale;
        self
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecordRelationRestoreCommit {
    pub workspace_id: WorkspaceId,
    pub branch_id: BranchId,
    pub previous_head_commit_id: CommitId,
    pub commit_id: CommitId,
    pub changeset_id: ChangeSetId,
    pub operation_id: OperationId,
    pub relation_id: RelationId,
    pub relation_version_id: RelationVersionId,
    pub relation_type: RecordRelationType,
    pub relation_label: Option<String>,
    pub source_record_entity_id: EntityId,
    pub target_record_entity_id: EntityId,
    pub relation_state_digest: Digest,
    pub work_state_digest: Digest,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecordRelationListOptions {
    commit_id: CommitId,
    relation_type: Option<RecordRelationType>,
    relation_label: Option<String>,
    source_record_entity_id: Option<EntityId>,
    target_record_entity_id: Option<EntityId>,
}

impl RecordRelationListOptions {
    pub fn new(commit_id: CommitId) -> Self {
        Self {
            commit_id,
            relation_type: None,
            relation_label: None,
            source_record_entity_id: None,
            target_record_entity_id: None,
        }
    }

    pub fn with_relation_type(mut self, relation_type: RecordRelationType) -> Self {
        self.relation_type = Some(relation_type);
        self
    }

    pub fn with_relation_label(mut self, label: impl Into<String>) -> Result<Self> {
        self.relation_label = Some(normalize_relation_label(&label.into())?);
        Ok(self)
    }

    pub fn with_source_record(mut self, record_entity_id: EntityId) -> Self {
        self.source_record_entity_id = Some(record_entity_id);
        self
    }

    pub fn with_target_record(mut self, record_entity_id: EntityId) -> Self {
        self.target_record_entity_id = Some(record_entity_id);
        self
    }

    pub fn commit_id(&self) -> CommitId {
        self.commit_id
    }

    pub fn relation_type(&self) -> Option<RecordRelationType> {
        self.relation_type
    }

    pub fn relation_label(&self) -> Option<&str> {
        self.relation_label.as_deref()
    }

    pub fn source_record_entity_id(&self) -> Option<EntityId> {
        self.source_record_entity_id
    }

    pub fn target_record_entity_id(&self) -> Option<EntityId> {
        self.target_record_entity_id
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecordRelationSnapshot {
    pub workspace_id: WorkspaceId,
    pub commit_id: CommitId,
    pub relation_id: RelationId,
    pub relation_version_id: RelationVersionId,
    pub relation_type: RecordRelationType,
    pub relation_label: Option<String>,
    pub source_record_entity_id: EntityId,
    pub target_record_entity_id: EntityId,
    pub state_digest: Digest,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecordRelationListResult {
    pub workspace_id: WorkspaceId,
    pub commit_id: CommitId,
    pub relations: Vec<RecordRelationSnapshot>,
}

pub(crate) fn create_record(
    connection: &mut StoreConnection,
    options: &RecordCreateOptions,
) -> Result<RecordCreateCommit> {
    let entity_options = EntityTransitionOptions::create(
        options.branch_id,
        options.expected_head_commit_id,
        RECORD_ENTITY_KIND,
        options.state.to_canonical_value()?,
    )?
    .with_rationale(options.rationale.clone());
    let commit = commit_entity_transition(connection, &entity_options)?;

    Ok(RecordCreateCommit {
        workspace_id: commit.workspace_id,
        branch_id: commit.branch_id,
        previous_head_commit_id: commit.previous_head_commit_id,
        commit_id: commit.commit_id,
        changeset_id: commit.changeset_id,
        operation_id: commit.operation_id,
        record_entity_id: commit.entity_id,
        record_entity_version_id: commit.entity_version_id,
        record_state_digest: commit.entity_state_digest,
        work_state_digest: commit.work_state_digest,
        state: options.state.clone(),
    })
}

pub(crate) fn transition_record(
    connection: &mut StoreConnection,
    options: &RecordTransitionOptions,
) -> Result<RecordTransitionCommit> {
    require_non_empty_rationale_object(&options.rationale)?;
    let current = record_at(
        connection,
        options.expected_head_commit_id,
        options.record_entity_id,
    )?;
    if current.record_entity_version_id != options.expected_record_entity_version_id {
        return Err(WorkVcsError::RecordInvalid(format!(
            "record entity {} expected version {}, found {} at commit {}",
            options.record_entity_id,
            options.expected_record_entity_version_id,
            current.record_entity_version_id,
            options.expected_head_commit_id
        )));
    }

    let next_state = match current.state.kind {
        RecordKind::Assumption => current
            .state
            .transition_assumption(options.next_status, &options.rationale_text)?,
        RecordKind::Attempt => current
            .state
            .transition_attempt(options.next_status, &options.rationale_text)?,
        RecordKind::Decision => current
            .state
            .transition_decision(options.next_status, &options.rationale_text)?,
        kind => {
            return Err(WorkVcsError::RecordInvalid(format!(
                "record kind {kind:?} does not use a transition lifecycle in this slice"
            )));
        }
    };
    let entity_options = EntityTransitionOptions::update(
        options.branch_id,
        options.expected_head_commit_id,
        options.record_entity_id,
        options.expected_record_entity_version_id,
        next_state.to_canonical_value()?,
    )?
    .with_rationale(options.rationale.clone());
    let commit = commit_entity_transition(connection, &entity_options)?;

    Ok(RecordTransitionCommit {
        workspace_id: commit.workspace_id,
        branch_id: commit.branch_id,
        previous_head_commit_id: commit.previous_head_commit_id,
        commit_id: commit.commit_id,
        changeset_id: commit.changeset_id,
        operation_id: commit.operation_id,
        record_entity_id: commit.entity_id,
        previous_record_entity_version_id: options.expected_record_entity_version_id,
        record_entity_version_id: commit.entity_version_id,
        record_state_digest: commit.entity_state_digest,
        work_state_digest: commit.work_state_digest,
        previous_state: current.state,
        state: next_state,
    })
}

pub(crate) fn supersede_decision_record(
    connection: &mut StoreConnection,
    options: &DecisionRecordSupersedeOptions,
) -> Result<DecisionRecordSupersedeCommit> {
    connection.verify_foreign_keys()?;
    require_non_empty_rationale_object(&options.rationale)?;
    if options.replacement_record_entity_id == options.prior_record_entity_id {
        return Err(WorkVcsError::RecordInvalid(format!(
            "Decision Record {} cannot supersede itself",
            options.prior_record_entity_id
        )));
    }

    let parent = state_at(connection, options.expected_head_commit_id)?;
    let replacement = record_at(
        connection,
        options.expected_head_commit_id,
        options.replacement_record_entity_id,
    )?;
    let prior = record_at(
        connection,
        options.expected_head_commit_id,
        options.prior_record_entity_id,
    )?;
    let causal = match options.causal_record_entity_id {
        Some(causal_record_entity_id) => {
            if causal_record_entity_id == options.replacement_record_entity_id {
                return Err(WorkVcsError::RecordInvalid(format!(
                    "Decision Record {} cannot derive from itself",
                    options.replacement_record_entity_id
                )));
            }
            Some(record_at(
                connection,
                options.expected_head_commit_id,
                causal_record_entity_id,
            )?)
        }
        None => None,
    };
    if replacement.workspace_id != parent.workspace_id
        || prior.workspace_id != parent.workspace_id
        || causal
            .as_ref()
            .is_some_and(|causal| causal.workspace_id != parent.workspace_id)
    {
        return Err(WorkVcsError::RecordInvalid(format!(
            "Decision supersession endpoints must belong to workspace {}",
            parent.workspace_id
        )));
    }
    if prior.record_entity_version_id != options.expected_prior_record_entity_version_id {
        return Err(WorkVcsError::RecordInvalid(format!(
            "prior Decision Record entity {} expected version {}, found {} at commit {}",
            options.prior_record_entity_id,
            options.expected_prior_record_entity_version_id,
            prior.record_entity_version_id,
            options.expected_head_commit_id
        )));
    }
    validate_decision_supersede_endpoints(&replacement, &prior)?;
    if let Some(causal) = &causal {
        validate_record_relation_endpoints_for_create(
            RecordRelationType::DerivedFrom,
            &replacement,
            causal,
        )?;
    }

    let prior_state = prior
        .state
        .transition_decision(RecordStatus::Superseded, &options.rationale_text)?;
    let prior_record_entity_version_id = EntityVersionId::new_v7();
    let relation_id = RelationId::new_v7();
    let relation_version_id = RelationVersionId::new_v7();
    let causal_relation_id = options
        .causal_record_entity_id
        .map(|_| RelationId::new_v7());
    let causal_relation_version_id = options
        .causal_record_entity_id
        .map(|_| RelationVersionId::new_v7());
    let changeset_id = ChangeSetId::new_v7();
    let commit_id = CommitId::new_v7();
    let prior_record_operation_id = OperationId::new_v7();
    let relation_operation_id = OperationId::new_v7();
    let causal_relation_operation_id = options
        .causal_record_entity_id
        .map(|_| OperationId::new_v7());
    let now_us = current_epoch_micros()?;

    let prior_state_value = prior_state.to_canonical_value()?;
    let prior_state_json = canonical_json_string(&prior_state_value)?;
    let prior_record_state_digest = entity_version_digest(&prior_state_value)?;
    let relation_state_value = CanonicalValue::object(Vec::new())?;
    let relation_state_json = canonical_json_string(&relation_state_value)?;
    let relation_state_digest = relation_version_digest(&relation_state_value)?;
    let next_work_state = work_state_after_decision_supersede(
        &parent.state,
        options.prior_record_entity_id,
        options.expected_prior_record_entity_version_id,
        prior_record_entity_version_id,
        relation_id,
        relation_version_id,
        causal_relation_id.zip(causal_relation_version_id),
    )?;
    let work_state_digest = work_state_mapping_digest(&next_work_state);
    let prior_payload_json = entity_transition_payload_json(
        options.prior_record_entity_id,
        Some(options.expected_prior_record_entity_version_id),
        prior_record_entity_version_id,
    )?;
    let relation_payload_value =
        relation_transition_payload_value(relation_id, None, Some(relation_version_id))?;
    let relation_payload_json = canonical_json_string(&relation_payload_value)?;
    let causal_relation_payload_json = match causal_relation_id.zip(causal_relation_version_id) {
        Some((relation_id, relation_version_id)) => Some(canonical_json_string(
            &relation_transition_payload_value(relation_id, None, Some(relation_version_id))?,
        )?),
        None => None,
    };
    let changeset_payload_json = decision_supersede_payload_json(&DecisionSupersedePayload {
        replacement_record_entity_id: options.replacement_record_entity_id,
        prior_record_entity_id: options.prior_record_entity_id,
        previous_prior_record_entity_version_id: options.expected_prior_record_entity_version_id,
        prior_record_entity_version_id,
        relation_id,
        relation_version_id,
        causal_record_entity_id: options.causal_record_entity_id,
        causal_relation_id,
        causal_relation_version_id,
    })?;
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
        return Err(WorkVcsError::RecordInvalid(format!(
            "branch {} belongs to workspace {}, but expected head {} belongs to workspace {}",
            options.branch_id,
            branch.workspace_id,
            options.expected_head_commit_id,
            parent.workspace_id
        )));
    }
    ensure_record_relation_logical_key_available(
        &transaction,
        branch.workspace_id,
        RecordRelationType::Supersedes,
        options.replacement_record_entity_id,
        options.prior_record_entity_id,
        "",
    )?;
    if let Some(causal_record_entity_id) = options.causal_record_entity_id {
        ensure_record_relation_logical_key_available(
            &transaction,
            branch.workspace_id,
            RecordRelationType::DerivedFrom,
            options.replacement_record_entity_id,
            causal_record_entity_id,
            "",
        )?;
    }
    write_decision_supersede(
        &transaction,
        &DecisionRecordSupersedeRows {
            workspace_id: branch.workspace_id,
            expected_head_commit_id: options.expected_head_commit_id,
            replacement_record_entity_id: options.replacement_record_entity_id,
            prior_record_entity_id: options.prior_record_entity_id,
            previous_prior_record_entity_version_id: options
                .expected_prior_record_entity_version_id,
            prior_record_entity_version_id,
            prior_state_json,
            prior_record_state_digest,
            relation_id,
            relation_version_id,
            relation_state_json,
            relation_state_digest,
            changeset_id,
            commit_id,
            prior_record_operation_id,
            relation_operation_id,
            prior_payload_json,
            relation_payload_json,
            causal_record_entity_id: options.causal_record_entity_id,
            causal_relation_id,
            causal_relation_version_id,
            causal_relation_operation_id,
            causal_relation_payload_json,
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
    )?;
    transaction.commit().map_err(storage_error)?;

    Ok(DecisionRecordSupersedeCommit {
        workspace_id: branch.workspace_id,
        branch_id: options.branch_id,
        previous_head_commit_id: options.expected_head_commit_id,
        commit_id,
        changeset_id,
        prior_record_operation_id,
        relation_operation_id,
        replacement_record_entity_id: options.replacement_record_entity_id,
        prior_record_entity_id: options.prior_record_entity_id,
        previous_prior_record_entity_version_id: options.expected_prior_record_entity_version_id,
        prior_record_entity_version_id,
        prior_record_state_digest,
        relation_id,
        relation_version_id,
        relation_state_digest,
        causal_record_entity_id: options.causal_record_entity_id,
        causal_relation_operation_id,
        causal_relation_id,
        causal_relation_version_id,
        causal_relation_state_digest: causal_relation_id.map(|_| relation_state_digest),
        work_state_digest,
        previous_prior_state: prior.state,
        prior_state,
    })
}

pub(crate) fn create_record_relation(
    connection: &mut StoreConnection,
    options: &RecordRelationCreateOptions,
) -> Result<RecordRelationCreateCommit> {
    connection.verify_foreign_keys()?;
    require_non_empty_rationale_object(&options.rationale)?;
    if options.source_record_entity_id == options.target_record_entity_id {
        return Err(WorkVcsError::RecordInvalid(format!(
            "record relation {} cannot use the same Record {} as both source and target",
            options.relation_type, options.source_record_entity_id
        )));
    }

    let parent = state_at(connection, options.expected_head_commit_id)?;
    let source = record_at(
        connection,
        options.expected_head_commit_id,
        options.source_record_entity_id,
    )?;
    let target = record_at(
        connection,
        options.expected_head_commit_id,
        options.target_record_entity_id,
    )?;
    if source.workspace_id != parent.workspace_id || target.workspace_id != parent.workspace_id {
        return Err(WorkVcsError::RecordInvalid(format!(
            "record relation endpoints must belong to workspace {}",
            parent.workspace_id
        )));
    }
    validate_record_relation_endpoints_for_create(options.relation_type, &source, &target)?;

    let relation_id = RelationId::new_v7();
    let relation_version_id = RelationVersionId::new_v7();
    let changeset_id = ChangeSetId::new_v7();
    let commit_id = CommitId::new_v7();
    let operation_id = OperationId::new_v7();
    let now_us = current_epoch_micros()?;

    let relation_state_value = CanonicalValue::object(Vec::new())?;
    let relation_state_json = canonical_json_string(&relation_state_value)?;
    let relation_state_digest = relation_version_digest(&relation_state_value)?;
    let next_work_state =
        work_state_after_record_relation_create(&parent.state, relation_id, relation_version_id)?;
    let work_state_digest = work_state_mapping_digest(&next_work_state);
    let relation_payload_value =
        relation_transition_payload_value(relation_id, None, Some(relation_version_id))?;
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
        return Err(WorkVcsError::RecordInvalid(format!(
            "branch {} belongs to workspace {}, but expected head {} belongs to workspace {}",
            options.branch_id,
            branch.workspace_id,
            options.expected_head_commit_id,
            parent.workspace_id
        )));
    }
    ensure_record_relation_logical_key_available(
        &transaction,
        branch.workspace_id,
        options.relation_type,
        options.source_record_entity_id,
        options.target_record_entity_id,
        relation_discriminator(options),
    )?;
    write_record_relation_create(
        &transaction,
        &RecordRelationCreateRows {
            workspace_id: branch.workspace_id,
            expected_head_commit_id: options.expected_head_commit_id,
            relation_id,
            relation_version_id,
            relation_state_json,
            relation_state_digest,
            relation_type: options.relation_type,
            relation_discriminator: relation_discriminator(options).to_owned(),
            source_record_entity_id: options.source_record_entity_id,
            target_record_entity_id: options.target_record_entity_id,
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
    )?;
    transaction.commit().map_err(storage_error)?;

    Ok(RecordRelationCreateCommit {
        workspace_id: branch.workspace_id,
        branch_id: options.branch_id,
        previous_head_commit_id: options.expected_head_commit_id,
        commit_id,
        changeset_id,
        operation_id,
        relation_id,
        relation_version_id,
        relation_type: options.relation_type,
        relation_label: options.relation_label.clone(),
        source_record_entity_id: options.source_record_entity_id,
        target_record_entity_id: options.target_record_entity_id,
        relation_state_digest,
        work_state_digest,
    })
}

pub(crate) fn remove_record_relation(
    connection: &mut StoreConnection,
    options: &RecordRelationRemoveOptions,
) -> Result<RecordRelationRemoveCommit> {
    connection.verify_foreign_keys()?;
    require_non_empty_rationale_object(&options.rationale)?;
    let parent = state_at(connection, options.expected_head_commit_id)?;
    let current_relation_version_id = parent
        .state
        .relations()
        .iter()
        .find_map(|(relation_id, relation_version_id)| {
            (*relation_id == options.relation_id).then_some(*relation_version_id)
        })
        .ok_or_else(|| {
            WorkVcsError::RecordNotFound(format!(
                "record relation {} is not present at commit {}",
                options.relation_id, options.expected_head_commit_id
            ))
        })?;
    if current_relation_version_id != options.expected_relation_version_id {
        return Err(WorkVcsError::RecordInvalid(format!(
            "record relation {} expected version {}, found {} at commit {}",
            options.relation_id,
            options.expected_relation_version_id,
            current_relation_version_id,
            options.expected_head_commit_id
        )));
    }

    let relation = record_relation_at(
        connection,
        options.expected_head_commit_id,
        options.relation_id,
    )?;
    let changeset_id = ChangeSetId::new_v7();
    let commit_id = CommitId::new_v7();
    let operation_id = OperationId::new_v7();
    let now_us = current_epoch_micros()?;
    let next_work_state = work_state_after_record_relation_remove(
        &parent.state,
        options.relation_id,
        options.expected_relation_version_id,
    )?;
    let work_state_digest = work_state_mapping_digest(&next_work_state);
    let relation_payload_value = relation_transition_payload_value(
        options.relation_id,
        Some(options.expected_relation_version_id),
        None,
    )?;
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
        return Err(WorkVcsError::RecordInvalid(format!(
            "branch {} belongs to workspace {}, but expected head {} belongs to workspace {}",
            options.branch_id,
            branch.workspace_id,
            options.expected_head_commit_id,
            parent.workspace_id
        )));
    }
    write_record_relation_remove(
        &transaction,
        &RecordRelationRemoveRows {
            workspace_id: branch.workspace_id,
            expected_head_commit_id: options.expected_head_commit_id,
            relation_id: options.relation_id,
            relation_version_id: options.expected_relation_version_id,
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
    )?;
    transaction.commit().map_err(storage_error)?;

    Ok(RecordRelationRemoveCommit {
        workspace_id: branch.workspace_id,
        branch_id: options.branch_id,
        previous_head_commit_id: options.expected_head_commit_id,
        commit_id,
        changeset_id,
        operation_id,
        relation_id: options.relation_id,
        previous_relation_version_id: options.expected_relation_version_id,
        relation_type: relation.relation_type,
        relation_label: relation.relation_label,
        source_record_entity_id: relation.source_record_entity_id,
        target_record_entity_id: relation.target_record_entity_id,
        work_state_digest,
    })
}

pub(crate) fn restore_record_relation(
    connection: &mut StoreConnection,
    options: &RecordRelationRestoreOptions,
) -> Result<RecordRelationRestoreCommit> {
    connection.verify_foreign_keys()?;
    require_non_empty_rationale_object(&options.rationale)?;
    let parent = state_at(connection, options.expected_head_commit_id)?;
    if parent
        .state
        .relations()
        .iter()
        .any(|(relation_id, _)| *relation_id == options.relation_id)
    {
        return Err(WorkVcsError::RecordInvalid(format!(
            "record relation {} is already present at commit {}",
            options.relation_id, options.expected_head_commit_id
        )));
    }

    let Some(relation) = load_record_relation_version(
        connection,
        parent.workspace_id,
        options.relation_id,
        options.relation_version_id,
    )?
    else {
        return Err(WorkVcsError::RecordInvalid(format!(
            "relation {} is not a semantic Record relation",
            options.relation_id
        )));
    };
    let source = record_snapshot_in_state(
        connection,
        parent.workspace_id,
        options.expected_head_commit_id,
        &parent.state,
        relation.source_record_entity_id,
    )?;
    let target = record_snapshot_in_state(
        connection,
        parent.workspace_id,
        options.expected_head_commit_id,
        &parent.state,
        relation.target_record_entity_id,
    )?;
    validate_record_relation_endpoints_for_projection(relation.relation_type, &source, &target)?;

    let changeset_id = ChangeSetId::new_v7();
    let commit_id = CommitId::new_v7();
    let operation_id = OperationId::new_v7();
    let now_us = current_epoch_micros()?;
    let next_work_state = work_state_after_record_relation_create(
        &parent.state,
        options.relation_id,
        options.relation_version_id,
    )?;
    let work_state_digest = work_state_mapping_digest(&next_work_state);
    let relation_payload_value = relation_transition_payload_value(
        options.relation_id,
        None,
        Some(options.relation_version_id),
    )?;
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
        return Err(WorkVcsError::RecordInvalid(format!(
            "branch {} belongs to workspace {}, but expected head {} belongs to workspace {}",
            options.branch_id,
            branch.workspace_id,
            options.expected_head_commit_id,
            parent.workspace_id
        )));
    }
    write_record_relation_restore(
        &transaction,
        &RecordRelationRestoreRows {
            workspace_id: branch.workspace_id,
            expected_head_commit_id: options.expected_head_commit_id,
            relation_id: options.relation_id,
            relation_version_id: options.relation_version_id,
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
    )?;
    transaction.commit().map_err(storage_error)?;

    Ok(RecordRelationRestoreCommit {
        workspace_id: branch.workspace_id,
        branch_id: options.branch_id,
        previous_head_commit_id: options.expected_head_commit_id,
        commit_id,
        changeset_id,
        operation_id,
        relation_id: options.relation_id,
        relation_version_id: options.relation_version_id,
        relation_type: relation.relation_type,
        relation_label: relation.relation_label,
        source_record_entity_id: relation.source_record_entity_id,
        target_record_entity_id: relation.target_record_entity_id,
        relation_state_digest: relation.state_digest,
        work_state_digest,
    })
}

pub(crate) fn records_at(
    connection: &StoreConnection,
    options: &RecordListOptions,
) -> Result<RecordListResult> {
    let commit_id = options.commit_id();
    let replayed = state_at(connection, commit_id)?;
    let mut records = Vec::new();

    for (entity_id, entity_version_id) in replayed.state.entities() {
        match load_entity_kind(connection, *entity_id)? {
            Some(entity_kind) if entity_kind == RECORD_ENTITY_KIND => {
                let loaded = load_record_version(
                    connection,
                    replayed.workspace_id,
                    *entity_id,
                    *entity_version_id,
                )?;
                if options.kind().is_none_or(|kind| loaded.state.kind == kind)
                    && options
                        .status()
                        .is_none_or(|status| loaded.state.status == status)
                    && options
                        .statement_contains()
                        .is_none_or(|fragment| loaded.state.statement.contains(fragment))
                {
                    records.push(RecordSnapshot {
                        workspace_id: replayed.workspace_id,
                        commit_id,
                        record_entity_id: *entity_id,
                        record_entity_version_id: *entity_version_id,
                        state_digest: loaded.state_digest,
                        state: loaded.state,
                    });
                }
            }
            Some(_) => {}
            None => {
                return Err(WorkVcsError::RecordInvalid(format!(
                    "WorkState at commit {commit_id} references missing entity {entity_id}"
                )));
            }
        }
    }

    records.sort_by_key(|record| record.record_entity_id);
    Ok(RecordListResult {
        workspace_id: replayed.workspace_id,
        commit_id,
        records,
    })
}

pub(crate) fn record_at(
    connection: &StoreConnection,
    commit_id: CommitId,
    record_entity_id: EntityId,
) -> Result<RecordSnapshot> {
    let replayed = state_at(connection, commit_id)?;
    let Some(record_entity_version_id) =
        replayed
            .state
            .entities()
            .iter()
            .find_map(|(entity_id, entity_version_id)| {
                (*entity_id == record_entity_id).then_some(*entity_version_id)
            })
    else {
        return Err(WorkVcsError::RecordNotFound(format!(
            "record entity {record_entity_id} is not present at commit {commit_id}"
        )));
    };

    let loaded = load_record_version(
        connection,
        replayed.workspace_id,
        record_entity_id,
        record_entity_version_id,
    )?;
    Ok(RecordSnapshot {
        workspace_id: replayed.workspace_id,
        commit_id,
        record_entity_id,
        record_entity_version_id,
        state_digest: loaded.state_digest,
        state: loaded.state,
    })
}

pub(crate) fn record_relations_at(
    connection: &StoreConnection,
    options: &RecordRelationListOptions,
) -> Result<RecordRelationListResult> {
    let commit_id = options.commit_id();
    let replayed = state_at(connection, commit_id)?;
    let mut relations = Vec::new();

    for (relation_id, relation_version_id) in replayed.state.relations() {
        let Some(relation) = load_record_relation_version(
            connection,
            replayed.workspace_id,
            *relation_id,
            *relation_version_id,
        )?
        else {
            continue;
        };
        if options
            .relation_type()
            .is_some_and(|relation_type| relation.relation_type != relation_type)
        {
            continue;
        }
        if options
            .relation_label()
            .is_some_and(|label| relation.relation_label.as_deref() != Some(label))
        {
            continue;
        }
        if options
            .source_record_entity_id()
            .is_some_and(|source_record_entity_id| {
                relation.source_record_entity_id != source_record_entity_id
            })
        {
            continue;
        }
        if options
            .target_record_entity_id()
            .is_some_and(|target_record_entity_id| {
                relation.target_record_entity_id != target_record_entity_id
            })
        {
            continue;
        }

        let source = record_snapshot_in_state(
            connection,
            replayed.workspace_id,
            commit_id,
            &replayed.state,
            relation.source_record_entity_id,
        )?;
        let target = record_snapshot_in_state(
            connection,
            replayed.workspace_id,
            commit_id,
            &replayed.state,
            relation.target_record_entity_id,
        )?;
        validate_record_relation_endpoints_for_projection(
            relation.relation_type,
            &source,
            &target,
        )?;

        relations.push(RecordRelationSnapshot {
            workspace_id: replayed.workspace_id,
            commit_id,
            relation_id: relation.relation_id,
            relation_version_id: relation.relation_version_id,
            relation_type: relation.relation_type,
            relation_label: relation.relation_label,
            source_record_entity_id: relation.source_record_entity_id,
            target_record_entity_id: relation.target_record_entity_id,
            state_digest: relation.state_digest,
        });
    }

    relations.sort_by(|left, right| {
        left.relation_type
            .cmp(&right.relation_type)
            .then_with(|| left.relation_label.cmp(&right.relation_label))
            .then_with(|| {
                left.source_record_entity_id
                    .cmp(&right.source_record_entity_id)
            })
            .then_with(|| {
                left.target_record_entity_id
                    .cmp(&right.target_record_entity_id)
            })
            .then_with(|| left.relation_id.cmp(&right.relation_id))
    });
    Ok(RecordRelationListResult {
        workspace_id: replayed.workspace_id,
        commit_id,
        relations,
    })
}

pub(crate) fn record_relation_at(
    connection: &StoreConnection,
    commit_id: CommitId,
    relation_id: RelationId,
) -> Result<RecordRelationSnapshot> {
    record_relations_at(connection, &RecordRelationListOptions::new(commit_id))?
        .relations
        .into_iter()
        .find(|relation| relation.relation_id == relation_id)
        .ok_or_else(|| {
            WorkVcsError::RecordNotFound(format!(
                "record relation {relation_id} is not present at commit {commit_id}"
            ))
        })
}

struct RecordRelationCreateRows {
    workspace_id: WorkspaceId,
    expected_head_commit_id: CommitId,
    relation_id: RelationId,
    relation_version_id: RelationVersionId,
    relation_state_json: String,
    relation_state_digest: Digest,
    relation_type: RecordRelationType,
    relation_discriminator: String,
    source_record_entity_id: EntityId,
    target_record_entity_id: EntityId,
    changeset_id: ChangeSetId,
    commit_id: CommitId,
    operation_id: OperationId,
    relation_payload_json: String,
    rationale_json: String,
    work_state_digest: Digest,
    now_us: i64,
}

struct RecordRelationRemoveRows {
    workspace_id: WorkspaceId,
    expected_head_commit_id: CommitId,
    relation_id: RelationId,
    relation_version_id: RelationVersionId,
    changeset_id: ChangeSetId,
    commit_id: CommitId,
    operation_id: OperationId,
    relation_payload_json: String,
    rationale_json: String,
    work_state_digest: Digest,
    now_us: i64,
}

struct RecordRelationRestoreRows {
    workspace_id: WorkspaceId,
    expected_head_commit_id: CommitId,
    relation_id: RelationId,
    relation_version_id: RelationVersionId,
    changeset_id: ChangeSetId,
    commit_id: CommitId,
    operation_id: OperationId,
    relation_payload_json: String,
    rationale_json: String,
    work_state_digest: Digest,
    now_us: i64,
}

struct DecisionRecordSupersedeRows {
    workspace_id: WorkspaceId,
    expected_head_commit_id: CommitId,
    replacement_record_entity_id: EntityId,
    prior_record_entity_id: EntityId,
    previous_prior_record_entity_version_id: EntityVersionId,
    prior_record_entity_version_id: EntityVersionId,
    prior_state_json: String,
    prior_record_state_digest: Digest,
    relation_id: RelationId,
    relation_version_id: RelationVersionId,
    relation_state_json: String,
    relation_state_digest: Digest,
    changeset_id: ChangeSetId,
    commit_id: CommitId,
    prior_record_operation_id: OperationId,
    relation_operation_id: OperationId,
    prior_payload_json: String,
    relation_payload_json: String,
    causal_record_entity_id: Option<EntityId>,
    causal_relation_id: Option<RelationId>,
    causal_relation_version_id: Option<RelationVersionId>,
    causal_relation_operation_id: Option<OperationId>,
    causal_relation_payload_json: Option<String>,
    changeset_payload_json: String,
    rationale_json: String,
    work_state_digest: Digest,
    now_us: i64,
}

struct LoadedRecordRelationVersion {
    relation_id: RelationId,
    relation_version_id: RelationVersionId,
    relation_type: RecordRelationType,
    relation_label: Option<String>,
    source_record_entity_id: EntityId,
    target_record_entity_id: EntityId,
    state_digest: Digest,
}

struct BranchRow {
    workspace_id: WorkspaceId,
    head_commit_id: CommitId,
}

fn validate_decision_supersede_endpoints(
    replacement: &RecordSnapshot,
    prior: &RecordSnapshot,
) -> Result<()> {
    if replacement.state.kind != RecordKind::Decision {
        return Err(WorkVcsError::RecordInvalid(format!(
            "supersedes replacement must be a Decision Record, found {}",
            replacement.state.kind
        )));
    }
    if replacement.state.status != RecordStatus::Active {
        return Err(WorkVcsError::RecordInvalid(format!(
            "supersedes replacement Decision must be active, found {}",
            replacement.state.status
        )));
    }
    if prior.state.kind != RecordKind::Decision {
        return Err(WorkVcsError::RecordInvalid(format!(
            "supersedes prior must be a Decision Record, found {}",
            prior.state.kind
        )));
    }
    if prior.state.status != RecordStatus::Active {
        return Err(WorkVcsError::RecordInvalid(format!(
            "supersedes prior Decision must be active, found {}",
            prior.state.status
        )));
    }
    Ok(())
}

fn validate_record_relation_endpoints_for_create(
    relation_type: RecordRelationType,
    source: &RecordSnapshot,
    target: &RecordSnapshot,
) -> Result<()> {
    match relation_type {
        RecordRelationType::Contradicts => {
            if source.state.kind != RecordKind::Finding {
                return Err(WorkVcsError::RecordInvalid(format!(
                    "contradicts source must be a Finding Record, found {}",
                    source.state.kind
                )));
            }
            if target.state.kind != RecordKind::Decision {
                return Err(WorkVcsError::RecordInvalid(format!(
                    "contradicts target must be a Decision Record, found {}",
                    target.state.kind
                )));
            }
            if target.state.status != RecordStatus::Active {
                return Err(WorkVcsError::RecordInvalid(format!(
                    "contradicts target Decision must be active, found {}",
                    target.state.status
                )));
            }
            Ok(())
        }
        RecordRelationType::DerivedFrom => Ok(()),
        RecordRelationType::Invalidates => {
            if source.state.kind != RecordKind::Finding {
                return Err(WorkVcsError::RecordInvalid(format!(
                    "invalidates source must be a Finding Record, found {}",
                    source.state.kind
                )));
            }
            if target.state.kind != RecordKind::Assumption {
                return Err(WorkVcsError::RecordInvalid(format!(
                    "invalidates target must be an Assumption Record, found {}",
                    target.state.kind
                )));
            }
            if target.state.status != RecordStatus::Invalidated {
                return Err(WorkVcsError::RecordInvalid(format!(
                    "invalidates target Assumption must be invalidated, found {}",
                    target.state.status
                )));
            }
            Ok(())
        }
        RecordRelationType::RelatedTo => Ok(()),
        RecordRelationType::Supersedes => validate_decision_supersede_endpoints(source, target),
        RecordRelationType::Supports => {
            if source.state.kind != RecordKind::Finding {
                return Err(WorkVcsError::RecordInvalid(format!(
                    "supports source must be a Finding Record, found {}",
                    source.state.kind
                )));
            }
            if target.state.kind != RecordKind::Decision {
                return Err(WorkVcsError::RecordInvalid(format!(
                    "supports target must be a Decision Record, found {}",
                    target.state.kind
                )));
            }
            if target.state.status != RecordStatus::Active {
                return Err(WorkVcsError::RecordInvalid(format!(
                    "supports target Decision must be active, found {}",
                    target.state.status
                )));
            }
            Ok(())
        }
        RecordRelationType::Validates => {
            if source.state.kind != RecordKind::Finding {
                return Err(WorkVcsError::RecordInvalid(format!(
                    "validates source must be a Finding Record, found {}",
                    source.state.kind
                )));
            }
            if target.state.kind != RecordKind::Assumption {
                return Err(WorkVcsError::RecordInvalid(format!(
                    "validates target must be an Assumption Record, found {}",
                    target.state.kind
                )));
            }
            if target.state.status != RecordStatus::Validated {
                return Err(WorkVcsError::RecordInvalid(format!(
                    "validates target Assumption must be validated, found {}",
                    target.state.status
                )));
            }
            Ok(())
        }
    }
}

fn validate_record_relation_endpoints_for_projection(
    relation_type: RecordRelationType,
    source: &RecordSnapshot,
    target: &RecordSnapshot,
) -> Result<()> {
    match relation_type {
        RecordRelationType::Contradicts => {
            if source.state.kind != RecordKind::Finding {
                return Err(WorkVcsError::RecordInvalid(format!(
                    "contradicts source must be a Finding Record, found {}",
                    source.state.kind
                )));
            }
            if target.state.kind != RecordKind::Decision {
                return Err(WorkVcsError::RecordInvalid(format!(
                    "contradicts target must be a Decision Record, found {}",
                    target.state.kind
                )));
            }
            Ok(())
        }
        RecordRelationType::DerivedFrom => Ok(()),
        RecordRelationType::Invalidates => {
            if source.state.kind != RecordKind::Finding {
                return Err(WorkVcsError::RecordInvalid(format!(
                    "invalidates source must be a Finding Record, found {}",
                    source.state.kind
                )));
            }
            if target.state.kind != RecordKind::Assumption {
                return Err(WorkVcsError::RecordInvalid(format!(
                    "invalidates target must be an Assumption Record, found {}",
                    target.state.kind
                )));
            }
            Ok(())
        }
        RecordRelationType::RelatedTo => Ok(()),
        RecordRelationType::Supersedes => {
            if source.state.kind != RecordKind::Decision {
                return Err(WorkVcsError::RecordInvalid(format!(
                    "supersedes replacement must be a Decision Record, found {}",
                    source.state.kind
                )));
            }
            if target.state.kind != RecordKind::Decision {
                return Err(WorkVcsError::RecordInvalid(format!(
                    "supersedes prior must be a Decision Record, found {}",
                    target.state.kind
                )));
            }
            Ok(())
        }
        RecordRelationType::Supports => {
            if source.state.kind != RecordKind::Finding {
                return Err(WorkVcsError::RecordInvalid(format!(
                    "supports source must be a Finding Record, found {}",
                    source.state.kind
                )));
            }
            if target.state.kind != RecordKind::Decision {
                return Err(WorkVcsError::RecordInvalid(format!(
                    "supports target must be a Decision Record, found {}",
                    target.state.kind
                )));
            }
            Ok(())
        }
        RecordRelationType::Validates => {
            if source.state.kind != RecordKind::Finding {
                return Err(WorkVcsError::RecordInvalid(format!(
                    "validates source must be a Finding Record, found {}",
                    source.state.kind
                )));
            }
            if target.state.kind != RecordKind::Assumption {
                return Err(WorkVcsError::RecordInvalid(format!(
                    "validates target must be an Assumption Record, found {}",
                    target.state.kind
                )));
            }
            Ok(())
        }
    }
}

fn ensure_record_relation_logical_key_available(
    transaction: &Transaction<'_>,
    workspace_id: WorkspaceId,
    relation_type: RecordRelationType,
    source_record_entity_id: EntityId,
    target_record_entity_id: EntityId,
    relation_discriminator: &str,
) -> Result<()> {
    let existing = transaction
        .query_row(
            "SELECT count(*)
             FROM relation
             WHERE workspace_id = ?1
               AND relation_type = ?2
               AND source_object_id = ?3
               AND target_object_id = ?4
               AND relation_discriminator = ?5",
            params![
                &workspace_id.raw_bytes()[..],
                relation_type.as_str(),
                &source_record_entity_id.raw_bytes()[..],
                &target_record_entity_id.raw_bytes()[..],
                relation_discriminator,
            ],
            |row| row.get::<_, i64>(0),
        )
        .map_err(storage_error)?;
    if existing == 0 {
        Ok(())
    } else {
        Err(WorkVcsError::RecordInvalid(format!(
            "record relation {relation_type} from {source_record_entity_id} to {target_record_entity_id} with label {relation_discriminator:?} already exists in workspace {workspace_id}"
        )))
    }
}

fn work_state_after_record_relation_create(
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
        return Err(WorkVcsError::RecordInvalid(format!(
            "record relation {relation_id} was expected to be absent before creation"
        )));
    }

    WorkState::new(entities, relations).map_err(record_invalid_from)
}

fn work_state_after_record_relation_remove(
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
    match relations.remove(&relation_id) {
        Some(current_relation_version_id) if current_relation_version_id == relation_version_id => {
            WorkState::new(entities, relations).map_err(record_invalid_from)
        }
        Some(current_relation_version_id) => Err(WorkVcsError::RecordInvalid(format!(
            "record relation {relation_id} expected version {relation_version_id}, found {current_relation_version_id}"
        ))),
        None => Err(WorkVcsError::RecordInvalid(format!(
            "record relation {relation_id} was expected to be present before removal"
        ))),
    }
}

fn work_state_after_decision_supersede(
    parent_state: &WorkState,
    prior_record_entity_id: EntityId,
    previous_prior_record_entity_version_id: EntityVersionId,
    prior_record_entity_version_id: EntityVersionId,
    relation_id: RelationId,
    relation_version_id: RelationVersionId,
    causal_relation: Option<(RelationId, RelationVersionId)>,
) -> Result<WorkState> {
    let mut entities = Vec::new();
    let mut replaced = false;
    for (entity_id, entity_version_id) in parent_state.entities() {
        if *entity_id == prior_record_entity_id {
            replaced = true;
            if *entity_version_id != previous_prior_record_entity_version_id {
                return Err(WorkVcsError::RecordInvalid(format!(
                    "prior Decision Record entity {prior_record_entity_id} expected parent version {previous_prior_record_entity_version_id}, found {entity_version_id}"
                )));
            }
            entities.push((*entity_id, prior_record_entity_version_id));
        } else {
            entities.push((*entity_id, *entity_version_id));
        }
    }
    if !replaced {
        return Err(WorkVcsError::RecordInvalid(format!(
            "prior Decision Record entity {prior_record_entity_id} was expected to be present"
        )));
    }

    let mut relations = parent_state
        .relations()
        .iter()
        .copied()
        .collect::<BTreeMap<_, _>>();
    if relations.insert(relation_id, relation_version_id).is_some() {
        return Err(WorkVcsError::RecordInvalid(format!(
            "supersedes relation {relation_id} was expected to be absent before creation"
        )));
    }
    if let Some((causal_relation_id, causal_relation_version_id)) = causal_relation
        && relations
            .insert(causal_relation_id, causal_relation_version_id)
            .is_some()
    {
        return Err(WorkVcsError::RecordInvalid(format!(
            "derived_from relation {causal_relation_id} was expected to be absent before creation"
        )));
    }

    WorkState::new(entities, relations).map_err(record_invalid_from)
}

fn record_snapshot_in_state(
    connection: &StoreConnection,
    workspace_id: WorkspaceId,
    commit_id: CommitId,
    state: &WorkState,
    record_entity_id: EntityId,
) -> Result<RecordSnapshot> {
    let Some(record_entity_version_id) =
        state
            .entities()
            .iter()
            .find_map(|(entity_id, entity_version_id)| {
                (*entity_id == record_entity_id).then_some(*entity_version_id)
            })
    else {
        return Err(WorkVcsError::RecordInvalid(format!(
            "record relation endpoint {record_entity_id} is not present at commit {commit_id}"
        )));
    };
    let loaded = load_record_version(
        connection,
        workspace_id,
        record_entity_id,
        record_entity_version_id,
    )?;
    Ok(RecordSnapshot {
        workspace_id,
        commit_id,
        record_entity_id,
        record_entity_version_id,
        state_digest: loaded.state_digest,
        state: loaded.state,
    })
}

fn write_record_relation_create(
    transaction: &Transaction<'_>,
    rows: &RecordRelationCreateRows,
) -> Result<()> {
    let workspace_id_bytes = rows.workspace_id.raw_bytes();
    let relation_id_bytes = rows.relation_id.raw_bytes();
    let relation_version_id_bytes = rows.relation_version_id.raw_bytes();
    let relation_state_digest_bytes = rows.relation_state_digest.as_bytes();
    let source_record_entity_id_bytes = rows.source_record_entity_id.raw_bytes();
    let target_record_entity_id_bytes = rows.target_record_entity_id.raw_bytes();
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
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                &relation_id_bytes[..],
                &workspace_id_bytes[..],
                rows.relation_type.as_str(),
                &source_record_entity_id_bytes[..],
                &target_record_entity_id_bytes[..],
                rows.relation_discriminator.as_str()
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
                RECORD_RELATION_CREATE_OPERATION_TYPE,
                RECORD_RELATION_CREATE_OPERATION_SCHEMA_VERSION,
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
                RECORD_RELATION_CREATE_EVENT_KIND,
                rows.now_us,
                rows.relation_payload_json
            ],
        )
        .map_err(storage_error)?;

    Ok(())
}

fn write_record_relation_remove(
    transaction: &Transaction<'_>,
    rows: &RecordRelationRemoveRows,
) -> Result<()> {
    let workspace_id_bytes = rows.workspace_id.raw_bytes();
    let relation_id_bytes = rows.relation_id.raw_bytes();
    let relation_version_id_bytes = rows.relation_version_id.raw_bytes();
    let changeset_id_bytes = rows.changeset_id.raw_bytes();
    let commit_id_bytes = rows.commit_id.raw_bytes();
    let operation_id_bytes = rows.operation_id.raw_bytes();
    let parent_commit_id_bytes = rows.expected_head_commit_id.raw_bytes();
    let work_state_digest_bytes = rows.work_state_digest.as_bytes();

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
                RECORD_RELATION_REMOVE_OPERATION_TYPE,
                RECORD_RELATION_REMOVE_OPERATION_SCHEMA_VERSION,
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
             VALUES (?1, ?2, ?3, NULL, ?4)",
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
                RECORD_RELATION_REMOVE_EVENT_KIND,
                rows.now_us,
                rows.relation_payload_json
            ],
        )
        .map_err(storage_error)?;

    Ok(())
}

fn write_record_relation_restore(
    transaction: &Transaction<'_>,
    rows: &RecordRelationRestoreRows,
) -> Result<()> {
    let workspace_id_bytes = rows.workspace_id.raw_bytes();
    let relation_id_bytes = rows.relation_id.raw_bytes();
    let relation_version_id_bytes = rows.relation_version_id.raw_bytes();
    let changeset_id_bytes = rows.changeset_id.raw_bytes();
    let commit_id_bytes = rows.commit_id.raw_bytes();
    let operation_id_bytes = rows.operation_id.raw_bytes();
    let parent_commit_id_bytes = rows.expected_head_commit_id.raw_bytes();
    let work_state_digest_bytes = rows.work_state_digest.as_bytes();

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
                RECORD_RELATION_RESTORE_OPERATION_TYPE,
                RECORD_RELATION_RESTORE_OPERATION_SCHEMA_VERSION,
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
                RECORD_RELATION_RESTORE_EVENT_KIND,
                rows.now_us,
                rows.relation_payload_json
            ],
        )
        .map_err(storage_error)?;

    Ok(())
}

fn write_decision_supersede(
    transaction: &Transaction<'_>,
    rows: &DecisionRecordSupersedeRows,
) -> Result<()> {
    let workspace_id_bytes = rows.workspace_id.raw_bytes();
    let replacement_record_entity_id_bytes = rows.replacement_record_entity_id.raw_bytes();
    let prior_record_entity_id_bytes = rows.prior_record_entity_id.raw_bytes();
    let previous_prior_record_entity_version_id_bytes =
        rows.previous_prior_record_entity_version_id.raw_bytes();
    let prior_record_entity_version_id_bytes = rows.prior_record_entity_version_id.raw_bytes();
    let prior_record_state_digest_bytes = rows.prior_record_state_digest.as_bytes();
    let relation_id_bytes = rows.relation_id.raw_bytes();
    let relation_version_id_bytes = rows.relation_version_id.raw_bytes();
    let relation_state_digest_bytes = rows.relation_state_digest.as_bytes();
    let changeset_id_bytes = rows.changeset_id.raw_bytes();
    let commit_id_bytes = rows.commit_id.raw_bytes();
    let prior_record_operation_id_bytes = rows.prior_record_operation_id.raw_bytes();
    let relation_operation_id_bytes = rows.relation_operation_id.raw_bytes();
    let parent_commit_id_bytes = rows.expected_head_commit_id.raw_bytes();
    let work_state_digest_bytes = rows.work_state_digest.as_bytes();
    let causal_relation = match (
        rows.causal_record_entity_id,
        rows.causal_relation_id,
        rows.causal_relation_version_id,
        rows.causal_relation_operation_id,
        rows.causal_relation_payload_json.as_ref(),
    ) {
        (
            Some(causal_record_entity_id),
            Some(causal_relation_id),
            Some(causal_relation_version_id),
            Some(causal_relation_operation_id),
            Some(causal_relation_payload_json),
        ) => Some((
            causal_record_entity_id,
            causal_relation_id,
            causal_relation_version_id,
            causal_relation_operation_id,
            causal_relation_payload_json,
        )),
        (None, None, None, None, None) => None,
        _ => {
            return Err(WorkVcsError::RecordInvalid(
                "Decision supersede causal relation rows are incomplete".to_owned(),
            ));
        }
    };

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
                &prior_record_entity_version_id_bytes[..],
                &prior_record_entity_id_bytes[..],
                RECORD_STATE_SCHEMA_VERSION,
                rows.prior_state_json,
                &prior_record_state_digest_bytes[..]
            ],
        )
        .map_err(storage_error)?;
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
                RecordRelationType::Supersedes.as_str(),
                &replacement_record_entity_id_bytes[..],
                &prior_record_entity_id_bytes[..]
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
    if let Some((
        causal_record_entity_id,
        causal_relation_id,
        causal_relation_version_id,
        _causal_relation_operation_id,
        _causal_relation_payload_json,
    )) = causal_relation
    {
        let causal_record_entity_id_bytes = causal_record_entity_id.raw_bytes();
        let causal_relation_id_bytes = causal_relation_id.raw_bytes();
        let causal_relation_version_id_bytes = causal_relation_version_id.raw_bytes();
        transaction
            .execute(
                "INSERT INTO object_identity(object_id, object_kind, created_at_us)
                 VALUES (?1, ?2, ?3)",
                params![
                    &causal_relation_id_bytes[..],
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
                    &causal_relation_id_bytes[..],
                    &workspace_id_bytes[..],
                    RecordRelationType::DerivedFrom.as_str(),
                    &replacement_record_entity_id_bytes[..],
                    &causal_record_entity_id_bytes[..]
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
                    &causal_relation_version_id_bytes[..],
                    &causal_relation_id_bytes[..],
                    RELATION_STATE_SCHEMA_VERSION,
                    rows.relation_state_json,
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
                RECORD_DECISION_SUPERSEDE_OPERATION_TYPE,
                RECORD_DECISION_SUPERSEDE_OPERATION_SCHEMA_VERSION,
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
                &prior_record_operation_id_bytes[..],
                &changeset_id_bytes[..],
                &prior_record_entity_id_bytes[..],
                rows.prior_payload_json
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
                &relation_operation_id_bytes[..],
                &changeset_id_bytes[..],
                &relation_id_bytes[..],
                rows.relation_payload_json
            ],
        )
        .map_err(storage_error)?;
    if let Some((
        _causal_record_entity_id,
        causal_relation_id,
        _causal_relation_version_id,
        causal_relation_operation_id,
        causal_relation_payload_json,
    )) = causal_relation
    {
        let causal_relation_id_bytes = causal_relation_id.raw_bytes();
        let causal_relation_operation_id_bytes = causal_relation_operation_id.raw_bytes();
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
                 VALUES (?1, ?2, 2, 'relation', ?3, ?4)",
                params![
                    &causal_relation_operation_id_bytes[..],
                    &changeset_id_bytes[..],
                    &causal_relation_id_bytes[..],
                    causal_relation_payload_json
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
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                &prior_record_operation_id_bytes[..],
                &prior_record_entity_id_bytes[..],
                &previous_prior_record_entity_version_id_bytes[..],
                &prior_record_entity_version_id_bytes[..],
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
                &relation_operation_id_bytes[..],
                &relation_id_bytes[..],
                &relation_version_id_bytes[..],
                EMPTY_FIELD_DELTA
            ],
        )
        .map_err(storage_error)?;
    if let Some((
        _causal_record_entity_id,
        causal_relation_id,
        causal_relation_version_id,
        causal_relation_operation_id,
        _causal_relation_payload_json,
    )) = causal_relation
    {
        let causal_relation_id_bytes = causal_relation_id.raw_bytes();
        let causal_relation_version_id_bytes = causal_relation_version_id.raw_bytes();
        let causal_relation_operation_id_bytes = causal_relation_operation_id.raw_bytes();
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
                    &causal_relation_operation_id_bytes[..],
                    &causal_relation_id_bytes[..],
                    &causal_relation_version_id_bytes[..],
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
                RECORD_DECISION_SUPERSEDE_EVENT_KIND,
                rows.now_us,
                rows.changeset_payload_json
            ],
        )
        .map_err(storage_error)?;

    Ok(())
}

fn load_active_branch(transaction: &Transaction<'_>, branch_id: BranchId) -> Result<BranchRow> {
    let branch_id_bytes = branch_id.raw_bytes();
    let row = transaction
        .query_row(
            "SELECT workspace_id, head_commit_id
             FROM branch
             WHERE branch_id = ?1
               AND lifecycle_state = ?2",
            params![&branch_id_bytes[..], ACTIVE_BRANCH_LIFECYCLE_STATE],
            |row| Ok((row.get::<_, Vec<u8>>(0)?, row.get::<_, Vec<u8>>(1)?)),
        )
        .optional()
        .map_err(storage_error)?;
    let Some((workspace_id, head_commit_id)) = row else {
        return Err(WorkVcsError::BranchNotFound(format!(
            "active branch {branch_id} does not exist"
        )));
    };
    Ok(BranchRow {
        workspace_id: decode_workspace_id("branch.workspace_id", workspace_id)?,
        head_commit_id: decode_commit_id("branch.head_commit_id", head_commit_id)?,
    })
}

fn move_branch_head(
    transaction: &Transaction<'_>,
    branch_id: BranchId,
    expected_head_commit_id: CommitId,
    commit_id: CommitId,
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
        Ok(())
    } else {
        Err(WorkVcsError::BranchHeadConflict(format!(
            "branch {branch_id} head changed before commit {commit_id} could be installed"
        )))
    }
}

fn relation_transition_payload_value(
    relation_id: RelationId,
    before_relation_version_id: Option<RelationVersionId>,
    after_relation_version_id: Option<RelationVersionId>,
) -> Result<CanonicalValue> {
    let before_value = match before_relation_version_id {
        Some(version_id) => CanonicalValue::String(version_id.to_string()),
        None => CanonicalValue::Null,
    };
    let after_value = match after_relation_version_id {
        Some(version_id) => CanonicalValue::String(version_id.to_string()),
        None => CanonicalValue::Null,
    };
    CanonicalValue::object(vec![
        ("after_relation_version_id".to_owned(), after_value),
        ("before_relation_version_id".to_owned(), before_value),
        (
            "relation_id".to_owned(),
            CanonicalValue::String(relation_id.to_string()),
        ),
    ])
    .map_err(record_invalid_from)
}

struct DecisionSupersedePayload {
    replacement_record_entity_id: EntityId,
    prior_record_entity_id: EntityId,
    previous_prior_record_entity_version_id: EntityVersionId,
    prior_record_entity_version_id: EntityVersionId,
    relation_id: RelationId,
    relation_version_id: RelationVersionId,
    causal_record_entity_id: Option<EntityId>,
    causal_relation_id: Option<RelationId>,
    causal_relation_version_id: Option<RelationVersionId>,
}

fn decision_supersede_payload_json(payload: &DecisionSupersedePayload) -> Result<String> {
    let has_causal_relation = payload.causal_relation_version_id.is_some();
    let causal_record_entity_id = optional_id_value(payload.causal_record_entity_id);
    let causal_relation_id = optional_id_value(payload.causal_relation_id);
    let causal_relation_version_id = optional_id_value(payload.causal_relation_version_id);
    canonical_json_string(
        &CanonicalValue::object(vec![
            (
                "causal_record_entity_id".to_owned(),
                causal_record_entity_id,
            ),
            ("causal_relation_id".to_owned(), causal_relation_id),
            (
                "causal_relation_type".to_owned(),
                if has_causal_relation {
                    CanonicalValue::String(RecordRelationType::DerivedFrom.as_str().to_owned())
                } else {
                    CanonicalValue::Null
                },
            ),
            (
                "causal_relation_version_id".to_owned(),
                causal_relation_version_id,
            ),
            (
                "prior_record_after_entity_version_id".to_owned(),
                CanonicalValue::String(payload.prior_record_entity_version_id.to_string()),
            ),
            (
                "prior_record_before_entity_version_id".to_owned(),
                CanonicalValue::String(payload.previous_prior_record_entity_version_id.to_string()),
            ),
            (
                "prior_record_entity_id".to_owned(),
                CanonicalValue::String(payload.prior_record_entity_id.to_string()),
            ),
            (
                "relation_id".to_owned(),
                CanonicalValue::String(payload.relation_id.to_string()),
            ),
            (
                "relation_type".to_owned(),
                CanonicalValue::String(RecordRelationType::Supersedes.as_str().to_owned()),
            ),
            (
                "relation_version_id".to_owned(),
                CanonicalValue::String(payload.relation_version_id.to_string()),
            ),
            (
                "replacement_record_entity_id".to_owned(),
                CanonicalValue::String(payload.replacement_record_entity_id.to_string()),
            ),
        ])
        .map_err(record_invalid_from)?,
    )
}

fn optional_id_value<T: fmt::Display>(value: Option<T>) -> CanonicalValue {
    value
        .map(|value| CanonicalValue::String(value.to_string()))
        .unwrap_or(CanonicalValue::Null)
}

struct LoadedRecordVersion {
    state_digest: Digest,
    state: RecordState,
}

fn load_record_version(
    connection: &StoreConnection,
    workspace_id: WorkspaceId,
    record_entity_id: EntityId,
    record_entity_version_id: EntityVersionId,
) -> Result<LoadedRecordVersion> {
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
                &record_entity_id.raw_bytes()[..],
                &record_entity_version_id.raw_bytes()[..]
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
        return Err(WorkVcsError::RecordNotFound(format!(
            "record entity {record_entity_id} version {record_entity_version_id} does not exist"
        )));
    };
    if object_kind != ENTITY_OBJECT_KIND {
        return Err(WorkVcsError::RecordInvalid(format!(
            "record entity {record_entity_id} has object kind {object_kind:?}"
        )));
    }
    if entity_kind != RECORD_ENTITY_KIND {
        return Err(WorkVcsError::RecordNotFound(format!(
            "entity {record_entity_id} has kind {entity_kind:?}, not {RECORD_ENTITY_KIND:?}"
        )));
    }
    if state_schema_version != RECORD_STATE_SCHEMA_VERSION {
        return Err(WorkVcsError::RecordInvalid(format!(
            "record entity {record_entity_id} version {record_entity_version_id} has state schema version {state_schema_version}"
        )));
    }
    let entity_workspace_id = decode_workspace_id("entity.workspace_id", entity_workspace_id)?;
    if entity_workspace_id != workspace_id {
        return Err(WorkVcsError::RecordInvalid(format!(
            "record entity {record_entity_id} belongs to workspace {entity_workspace_id}, not {workspace_id}"
        )));
    }

    let state_digest = decode_digest("entity_version.state_digest", state_digest)?;
    validate_import_fixed_point(
        state_json.as_bytes(),
        state_digest,
        ImportDigestDomain::EntityVersion,
    )
    .map_err(|error| {
        WorkVcsError::RecordInvalid(format!(
            "record entity {record_entity_id} version {record_entity_version_id} fixed-point validation failed: {error}"
        ))
    })?;
    let value = parse_canonical_json(state_json.as_bytes()).map_err(record_invalid_from)?;
    let actual = entity_version_digest(&value).map_err(record_invalid_from)?;
    if actual != state_digest {
        return Err(WorkVcsError::RecordInvalid(format!(
            "record entity {record_entity_id} version {record_entity_version_id} digest does not match state JSON"
        )));
    }

    Ok(LoadedRecordVersion {
        state_digest,
        state: parse_record_state(value)?,
    })
}

fn load_record_relation_version(
    connection: &StoreConnection,
    workspace_id: WorkspaceId,
    relation_id: RelationId,
    relation_version_id: RelationVersionId,
) -> Result<Option<LoadedRecordRelationVersion>> {
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
        return Err(WorkVcsError::RecordNotFound(format!(
            "record relation {relation_id} version {relation_version_id} does not exist"
        )));
    };
    if object_kind != RELATION_OBJECT_KIND {
        return Err(WorkVcsError::RecordInvalid(format!(
            "record relation {relation_id} has object kind {object_kind:?}"
        )));
    }
    let relation_workspace_id =
        decode_workspace_id("relation.workspace_id", relation_workspace_id)?;
    if relation_workspace_id != workspace_id {
        return Err(WorkVcsError::RecordInvalid(format!(
            "record relation {relation_id} belongs to workspace {relation_workspace_id}, not {workspace_id}"
        )));
    }

    let Some(relation_type) = RecordRelationType::parse(&relation_type) else {
        return Ok(None);
    };
    let relation_label = match relation_type {
        RecordRelationType::RelatedTo => Some(normalize_relation_label(&relation_discriminator)?),
        _ if relation_discriminator.is_empty() => None,
        _ => {
            return Err(WorkVcsError::RecordInvalid(format!(
                "record relation {relation_id} has non-empty discriminator {relation_discriminator:?}"
            )));
        }
    };
    if state_schema_version != RELATION_STATE_SCHEMA_VERSION {
        return Err(WorkVcsError::RecordInvalid(format!(
            "record relation {relation_id} version {relation_version_id} has state schema version {state_schema_version}"
        )));
    }

    let state_digest = decode_digest("relation_version.state_digest", state_digest)?;
    validate_import_fixed_point(
        metadata_json.as_bytes(),
        state_digest,
        ImportDigestDomain::RelationVersion,
    )
    .map_err(|error| {
        WorkVcsError::RecordInvalid(format!(
            "record relation {relation_id} version {relation_version_id} fixed-point validation failed: {error}"
        ))
    })?;
    let metadata_value =
        parse_canonical_json(metadata_json.as_bytes()).map_err(record_invalid_from)?;
    if metadata_value != CanonicalValue::object(Vec::new())? {
        return Err(WorkVcsError::RecordInvalid(format!(
            "record relation {relation_id} version {relation_version_id} state must be canonical empty object"
        )));
    }
    let actual = relation_version_digest(&metadata_value).map_err(record_invalid_from)?;
    if actual != state_digest {
        return Err(WorkVcsError::RecordInvalid(format!(
            "record relation {relation_id} version {relation_version_id} digest does not match metadata JSON"
        )));
    }

    Ok(Some(LoadedRecordRelationVersion {
        relation_id,
        relation_version_id,
        relation_type,
        relation_label,
        source_record_entity_id: decode_entity_id("relation.source_object_id", source_object_id)?,
        target_record_entity_id: decode_entity_id("relation.target_object_id", target_object_id)?,
        state_digest,
    }))
}

fn load_entity_kind(connection: &StoreConnection, entity_id: EntityId) -> Result<Option<String>> {
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

fn parse_record_state(value: CanonicalValue) -> Result<RecordState> {
    let CanonicalValue::Object(entries) = value else {
        return Err(WorkVcsError::RecordInvalid(
            "record state must be a canonical object".to_owned(),
        ));
    };
    if entries.len() != 4 {
        return Err(WorkVcsError::RecordInvalid(format!(
            "record state must contain exactly 4 fields, found {}",
            entries.len()
        )));
    }

    let mut kind = None;
    let mut scope = None;
    let mut statement = None;
    let mut status = None;

    for (key, value) in entries {
        match key.as_str() {
            "kind" => {
                let value = require_string("kind", value)?;
                kind = Some(RecordKind::parse(&value)?);
            }
            "scope" => {
                require_object_value("scope", &value)?;
                scope = Some(value);
            }
            "statement" => {
                let value = require_string("statement", value)?;
                validate_statement(&value)?;
                statement = Some(value);
            }
            "status" => {
                let value = require_string("status", value)?;
                status = Some(RecordStatus::parse(&value)?);
            }
            other => {
                return Err(WorkVcsError::RecordInvalid(format!(
                    "record state contains unsupported field {other:?}"
                )));
            }
        }
    }

    Ok(RecordState {
        kind: kind.ok_or_else(|| missing_field("kind"))?,
        scope: scope.ok_or_else(|| missing_field("scope"))?,
        statement: statement.ok_or_else(|| missing_field("statement"))?,
        status: status.ok_or_else(|| missing_field("status"))?,
    })
    .and_then(|state| {
        validate_record_status_for_kind(state.kind, state.status)?;
        Ok(state)
    })
}

fn validate_statement(value: &str) -> Result<()> {
    if value.trim().is_empty() {
        return Err(WorkVcsError::RecordInvalid(
            "record statement must not be empty".to_owned(),
        ));
    }
    Ok(())
}

fn validate_transition_rationale(value: &str) -> Result<()> {
    if value.trim().is_empty() {
        return Err(WorkVcsError::RecordInvalid(
            "record transition rationale must not be empty".to_owned(),
        ));
    }
    Ok(())
}

fn validate_assumption_lifecycle_transition(
    current: RecordStatus,
    next: RecordStatus,
) -> Result<()> {
    match (current, next) {
        (RecordStatus::Unverified, RecordStatus::Validated)
        | (RecordStatus::Unverified, RecordStatus::Invalidated)
        | (RecordStatus::Validated, RecordStatus::Invalidated) => Ok(()),
        (current, next) => Err(WorkVcsError::RecordInvalid(format!(
            "assumption transition {current:?} -> {next:?} is not allowed"
        ))),
    }
}

fn validate_attempt_lifecycle_transition(current: RecordStatus, next: RecordStatus) -> Result<()> {
    match (current, next) {
        (
            RecordStatus::Running,
            RecordStatus::Succeeded | RecordStatus::Failed | RecordStatus::Inconclusive,
        ) => Ok(()),
        (current, next) => Err(WorkVcsError::RecordInvalid(format!(
            "attempt transition {current:?} -> {next:?} is not allowed"
        ))),
    }
}

fn validate_decision_lifecycle_transition(current: RecordStatus, next: RecordStatus) -> Result<()> {
    match (current, next) {
        (RecordStatus::Active, RecordStatus::Superseded | RecordStatus::Withdrawn) => Ok(()),
        (current, next) => Err(WorkVcsError::RecordInvalid(format!(
            "decision transition {current:?} -> {next:?} is not allowed"
        ))),
    }
}

fn validate_record_status_for_kind(kind: RecordKind, status: RecordStatus) -> Result<()> {
    match (kind, status) {
        (
            RecordKind::Finding | RecordKind::Handoff | RecordKind::Question | RecordKind::Risk,
            RecordStatus::Active,
        )
        | (
            RecordKind::Decision,
            RecordStatus::Active | RecordStatus::Superseded | RecordStatus::Withdrawn,
        )
        | (
            RecordKind::Attempt,
            RecordStatus::Running
            | RecordStatus::Succeeded
            | RecordStatus::Failed
            | RecordStatus::Inconclusive,
        )
        | (
            RecordKind::Assumption,
            RecordStatus::Unverified | RecordStatus::Validated | RecordStatus::Invalidated,
        ) => Ok(()),
        (kind, status) => Err(WorkVcsError::RecordInvalid(format!(
            "record kind {kind:?} cannot use status {status:?}"
        ))),
    }
}

fn require_string(field: &str, value: CanonicalValue) -> Result<String> {
    match value {
        CanonicalValue::String(value) => Ok(value),
        other => Err(WorkVcsError::RecordInvalid(format!(
            "record field {field} must be a string, found {other:?}"
        ))),
    }
}

fn require_object_value(label: &str, value: &CanonicalValue) -> Result<()> {
    match value {
        CanonicalValue::Object(_) => Ok(()),
        other => Err(WorkVcsError::RecordInvalid(format!(
            "{label} must be a canonical object, found {other:?}"
        ))),
    }
}

fn missing_field(field: &str) -> WorkVcsError {
    WorkVcsError::RecordInvalid(format!("record state missing required field {field:?}"))
}

fn require_non_empty_rationale_object(value: &CanonicalValue) -> Result<()> {
    match value {
        CanonicalValue::Object(entries) if !entries.is_empty() => Ok(()),
        CanonicalValue::Object(_) => Err(WorkVcsError::RecordInvalid(
            "record transition rationale must not be empty".to_owned(),
        )),
        other => Err(WorkVcsError::RecordInvalid(format!(
            "record transition rationale must be a canonical object, found {other:?}"
        ))),
    }
}

fn rationale_value(reason: &str) -> Result<CanonicalValue> {
    validate_transition_rationale(reason)?;
    CanonicalValue::object(vec![(
        "reason".to_owned(),
        CanonicalValue::String(reason.to_owned()),
    )])
}

fn relation_discriminator(options: &RecordRelationCreateOptions) -> &str {
    options.relation_label.as_deref().unwrap_or("")
}

fn normalize_relation_label(value: &str) -> Result<String> {
    if value.trim().is_empty() {
        return Err(WorkVcsError::RecordInvalid(
            "record relation label must not be empty".to_owned(),
        ));
    }
    if value.trim() != value {
        return Err(WorkVcsError::RecordInvalid(
            "record relation label must not have leading or trailing whitespace".to_owned(),
        ));
    }
    if value.chars().any(char::is_control) {
        return Err(WorkVcsError::RecordInvalid(
            "record relation label must not contain control characters".to_owned(),
        ));
    }
    Ok(value.to_owned())
}

fn record_invalid_from(error: WorkVcsError) -> WorkVcsError {
    WorkVcsError::RecordInvalid(error.to_string())
}

fn decode_workspace_id(column: &str, bytes: Vec<u8>) -> Result<WorkspaceId> {
    let bytes = bytes.try_into().map_err(|bytes: Vec<u8>| {
        WorkVcsError::RecordInvalid(format!("{column} must be 16 bytes, found {}", bytes.len()))
    })?;
    WorkspaceId::from_bytes(bytes).map_err(|error| {
        WorkVcsError::RecordInvalid(format!("{column} is not a valid WorkspaceId: {error}"))
    })
}

fn decode_commit_id(column: &str, bytes: Vec<u8>) -> Result<CommitId> {
    let bytes = bytes.try_into().map_err(|bytes: Vec<u8>| {
        WorkVcsError::RecordInvalid(format!("{column} must be 16 bytes, found {}", bytes.len()))
    })?;
    CommitId::from_bytes(bytes).map_err(|error| {
        WorkVcsError::RecordInvalid(format!("{column} is not a valid CommitId: {error}"))
    })
}

fn decode_entity_id(column: &str, bytes: Vec<u8>) -> Result<EntityId> {
    let bytes = bytes.try_into().map_err(|bytes: Vec<u8>| {
        WorkVcsError::RecordInvalid(format!("{column} must be 16 bytes, found {}", bytes.len()))
    })?;
    EntityId::from_bytes(bytes).map_err(|error| {
        WorkVcsError::RecordInvalid(format!("{column} is not a valid EntityId: {error}"))
    })
}

fn decode_digest(column: &str, bytes: Vec<u8>) -> Result<Digest> {
    let bytes = bytes.try_into().map_err(|bytes: Vec<u8>| {
        WorkVcsError::RecordInvalid(format!("{column} must be 32 bytes, found {}", bytes.len()))
    })?;
    Ok(Digest::from_bytes(bytes))
}
