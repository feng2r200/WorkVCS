use super::entity::{
    ENTITY_TRANSITION_OPERATION_SCHEMA_VERSION, ENTITY_TRANSITION_OPERATION_TYPE,
    canonical_json_string, entity_transition_payload_value,
};
use super::{EntityTransitionOptions, commit_entity_transition, state_at};
use crate::canonical::{
    CanonicalValue, ImportDigestDomain, WorkState, entity_version_digest, parse_canonical_json,
    validate_import_fixed_point, work_state_mapping_digest,
};
use crate::error::{Result, WorkVcsError, storage_error};
use crate::identity::{
    BranchId, ChangeSetId, CommitId, Digest, EntityId, EntityVersionId, EventId, OperationId,
    WorkspaceId,
};
use crate::store::{StoreConnection, current_epoch_micros};
use rusqlite::{OptionalExtension, Transaction, TransactionBehavior, params};
use std::collections::{BTreeMap, HashSet};
use std::fmt;

pub(crate) const ACCEPTANCE_CRITERION_ENTITY_KIND: &str = "acceptance_criterion";
pub(crate) const TASK_ENTITY_KIND: &str = "task";

const ACTIVE_BRANCH_LIFECYCLE_STATE: &str = "active";
const EMPTY_FIELD_DELTA: &str = "{}";
const ENTITY_OBJECT_KIND: &str = "entity";
const ENTITY_TRANSITION_EVENT_KIND: &str = "entity.transitioned";
const NORMAL_COMMIT_KIND: &str = "normal";
const PRIMARY_PARENT_ROLE: &str = "primary";
const ACCEPTANCE_CRITERION_STATE_SCHEMA_VERSION: i64 = 1;
const TASK_STATE_SCHEMA_VERSION: i64 = 1;

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
        validate_local_key(&local_key)?;
        Ok(Self {
            local_key,
            acceptance_criterion_entity_id,
        })
    }

    fn to_canonical_value(&self) -> Result<CanonicalValue> {
        validate_local_key(&self.local_key)?;
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
        })
    }

    pub fn to_canonical_value(&self) -> Result<CanonicalValue> {
        validate_acceptance_criterion_statement(&self.statement)?;
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
                CanonicalValue::Array(Vec::new()),
            ),
        ])
        .map_err(task_invalid_from)
    }
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
        validate_local_key(&local_key)?;
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
            "mandatory acceptance criteria cannot be added to a done task before Verification is implemented"
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
            "mandatory acceptance criteria cannot be revised under a done task before Verification is implemented"
                .to_owned(),
        ));
    }
    if current.state == options.state {
        return Err(WorkVcsError::TaskInvalid(
            "acceptance criterion revision must change statement or classification".to_owned(),
        ));
    }

    let entity_options = EntityTransitionOptions::update(
        options.branch_id,
        options.expected_head_commit_id,
        options.acceptance_criterion_entity_id,
        options.expected_acceptance_criterion_entity_version_id,
        options.state.to_canonical_value()?,
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
        state: options.state.clone(),
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
    validate_local_key(&local_key)?;

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
        TASK_ENTITY_KIND | ACCEPTANCE_CRITERION_ENTITY_KIND
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
                    validate_local_key(&value)?;
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
                require_empty_array("verification_requirements", &value)?;
                verification_requirements = Some(());
            }
            other => {
                return Err(WorkVcsError::TaskInvalid(format!(
                    "acceptance criterion state contains unsupported field {other:?}"
                )));
            }
        }
    }
    verification_requirements.ok_or_else(|| {
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

fn require_mandatory_acceptance_criteria_verified(
    connection: &StoreConnection,
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
            return Err(WorkVcsError::TaskInvalid(format!(
                "mandatory acceptance criterion {} is unverified; Verification projection is deferred",
                criterion.local_key
            )));
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
        validate_local_key(&criterion.local_key)?;
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

fn validate_local_key(value: &str) -> Result<()> {
    if value.trim().is_empty() {
        return Err(WorkVcsError::TaskInvalid(
            "acceptance criterion local key must not be empty".to_owned(),
        ));
    }
    if value.trim() != value {
        return Err(WorkVcsError::TaskInvalid(
            "acceptance criterion local key must not have leading or trailing whitespace"
                .to_owned(),
        ));
    }
    if value
        .as_bytes()
        .iter()
        .any(|byte| *byte == b'\0' || *byte < 0x20 || *byte == 0x7f)
    {
        return Err(WorkVcsError::TaskInvalid(
            "acceptance criterion local key must not contain NUL or ASCII control characters"
                .to_owned(),
        ));
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
