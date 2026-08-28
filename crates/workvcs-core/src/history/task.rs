use super::{EntityTransitionOptions, commit_entity_transition, state_at};
use crate::canonical::{
    CanonicalValue, ImportDigestDomain, entity_version_digest, parse_canonical_json,
    validate_import_fixed_point,
};
use crate::error::{Result, WorkVcsError, storage_error};
use crate::identity::{
    BranchId, ChangeSetId, CommitId, Digest, EntityId, EntityVersionId, OperationId, WorkspaceId,
};
use crate::store::StoreConnection;
use rusqlite::{OptionalExtension, params};
use std::fmt;

pub(crate) const TASK_ENTITY_KIND: &str = "task";

const ENTITY_OBJECT_KIND: &str = "entity";
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
        })
    }

    pub fn to_canonical_value(&self) -> Result<CanonicalValue> {
        validate_description(&self.description)?;
        validate_priority(self.priority)?;
        if let Some(outcome) = &self.outcome {
            validate_outcome(outcome)?;
        }
        let outcome = self
            .outcome
            .as_ref()
            .map(|value| CanonicalValue::String(value.clone()))
            .unwrap_or(CanonicalValue::Null);

        CanonicalValue::object(vec![
            (
                "acceptance_criteria".to_owned(),
                CanonicalValue::Array(Vec::new()),
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
pub struct TaskSnapshot {
    pub workspace_id: WorkspaceId,
    pub commit_id: CommitId,
    pub task_entity_id: EntityId,
    pub task_entity_version_id: EntityVersionId,
    pub state_digest: Digest,
    pub state: TaskState,
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

struct LoadedTaskVersion {
    state_digest: Digest,
    state: TaskState,
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
                require_empty_array("acceptance_criteria", &value)?;
                acceptance_criteria = Some(());
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

    acceptance_criteria.ok_or_else(|| {
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
    })
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
            "{label} is deferred in Phase 3A and must be empty"
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
