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

pub(crate) const GOAL_ENTITY_KIND: &str = "goal";

const ENTITY_OBJECT_KIND: &str = "entity";
const GOAL_STATE_SCHEMA_VERSION: i64 = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GoalStatus {
    Active,
    Achieved,
    Abandoned,
}

impl GoalStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Achieved => "achieved",
            Self::Abandoned => "abandoned",
        }
    }

    fn parse(value: &str) -> Result<Self> {
        match value {
            "active" => Ok(Self::Active),
            "achieved" => Ok(Self::Achieved),
            "abandoned" => Ok(Self::Abandoned),
            other => Err(WorkVcsError::GoalInvalid(format!(
                "goal status {other:?} is not in the confirmed lifecycle vocabulary"
            ))),
        }
    }
}

impl fmt::Display for GoalStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GoalState {
    pub description: String,
    pub status: GoalStatus,
    pub terminal_rationale: Option<String>,
}

impl GoalState {
    pub fn active(description: impl Into<String>) -> Result<Self> {
        let description = description.into();
        validate_description(&description)?;
        Ok(Self {
            description,
            status: GoalStatus::Active,
            terminal_rationale: None,
        })
    }

    pub fn to_canonical_value(&self) -> Result<CanonicalValue> {
        validate_description(&self.description)?;
        if let Some(terminal_rationale) = &self.terminal_rationale {
            validate_terminal_rationale(terminal_rationale)?;
        }

        let terminal_rationale = self
            .terminal_rationale
            .as_ref()
            .map(|value| CanonicalValue::String(value.clone()))
            .unwrap_or(CanonicalValue::Null);

        CanonicalValue::object(vec![
            (
                "description".to_owned(),
                CanonicalValue::String(self.description.clone()),
            ),
            ("plan_refs".to_owned(), CanonicalValue::Array(Vec::new())),
            (
                "status".to_owned(),
                CanonicalValue::String(self.status.as_str().to_owned()),
            ),
            ("subgoals".to_owned(), CanonicalValue::Array(Vec::new())),
            ("terminal_rationale".to_owned(), terminal_rationale),
            ("work_refs".to_owned(), CanonicalValue::Array(Vec::new())),
        ])
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GoalCreateOptions {
    branch_id: BranchId,
    expected_head_commit_id: CommitId,
    state: GoalState,
    rationale: CanonicalValue,
}

impl GoalCreateOptions {
    pub fn new(
        branch_id: BranchId,
        expected_head_commit_id: CommitId,
        description: impl Into<String>,
    ) -> Result<Self> {
        Ok(Self {
            branch_id,
            expected_head_commit_id,
            state: GoalState::active(description)?,
            rationale: CanonicalValue::object(Vec::new())?,
        })
    }

    pub fn with_rationale(mut self, rationale: CanonicalValue) -> Self {
        self.rationale = rationale;
        self
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GoalCreateCommit {
    pub workspace_id: WorkspaceId,
    pub branch_id: BranchId,
    pub previous_head_commit_id: CommitId,
    pub commit_id: CommitId,
    pub changeset_id: ChangeSetId,
    pub operation_id: OperationId,
    pub goal_entity_id: EntityId,
    pub goal_entity_version_id: EntityVersionId,
    pub goal_state_digest: Digest,
    pub work_state_digest: Digest,
    pub state: GoalState,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GoalSnapshot {
    pub workspace_id: WorkspaceId,
    pub commit_id: CommitId,
    pub goal_entity_id: EntityId,
    pub goal_entity_version_id: EntityVersionId,
    pub state_digest: Digest,
    pub state: GoalState,
}

pub(crate) fn create_goal(
    connection: &mut StoreConnection,
    options: &GoalCreateOptions,
) -> Result<GoalCreateCommit> {
    let entity_options = EntityTransitionOptions::create(
        options.branch_id,
        options.expected_head_commit_id,
        GOAL_ENTITY_KIND,
        options.state.to_canonical_value()?,
    )?
    .with_rationale(options.rationale.clone());
    let commit = commit_entity_transition(connection, &entity_options)?;

    Ok(GoalCreateCommit {
        workspace_id: commit.workspace_id,
        branch_id: commit.branch_id,
        previous_head_commit_id: commit.previous_head_commit_id,
        commit_id: commit.commit_id,
        changeset_id: commit.changeset_id,
        operation_id: commit.operation_id,
        goal_entity_id: commit.entity_id,
        goal_entity_version_id: commit.entity_version_id,
        goal_state_digest: commit.entity_state_digest,
        work_state_digest: commit.work_state_digest,
        state: options.state.clone(),
    })
}

pub(crate) fn goal_at(
    connection: &StoreConnection,
    commit_id: CommitId,
    goal_entity_id: EntityId,
) -> Result<GoalSnapshot> {
    let replayed = state_at(connection, commit_id)?;
    let Some(goal_entity_version_id) =
        replayed
            .state
            .entities()
            .iter()
            .find_map(|(entity_id, entity_version_id)| {
                (*entity_id == goal_entity_id).then_some(*entity_version_id)
            })
    else {
        return Err(WorkVcsError::GoalNotFound(format!(
            "goal entity {goal_entity_id} is not present at commit {commit_id}"
        )));
    };

    let loaded = load_goal_version(
        connection,
        replayed.workspace_id,
        goal_entity_id,
        goal_entity_version_id,
    )?;
    Ok(GoalSnapshot {
        workspace_id: replayed.workspace_id,
        commit_id,
        goal_entity_id,
        goal_entity_version_id,
        state_digest: loaded.state_digest,
        state: loaded.state,
    })
}

struct LoadedGoalVersion {
    state_digest: Digest,
    state: GoalState,
}

fn load_goal_version(
    connection: &StoreConnection,
    workspace_id: WorkspaceId,
    goal_entity_id: EntityId,
    goal_entity_version_id: EntityVersionId,
) -> Result<LoadedGoalVersion> {
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
                &goal_entity_id.raw_bytes()[..],
                &goal_entity_version_id.raw_bytes()[..]
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
        return Err(WorkVcsError::GoalNotFound(format!(
            "goal entity {goal_entity_id} version {goal_entity_version_id} does not exist"
        )));
    };
    if object_kind != ENTITY_OBJECT_KIND {
        return Err(WorkVcsError::GoalInvalid(format!(
            "goal entity {goal_entity_id} has object kind {object_kind:?}"
        )));
    }
    if entity_kind != GOAL_ENTITY_KIND {
        return Err(WorkVcsError::GoalNotFound(format!(
            "entity {goal_entity_id} has kind {entity_kind:?}, not {GOAL_ENTITY_KIND:?}"
        )));
    }
    if state_schema_version != GOAL_STATE_SCHEMA_VERSION {
        return Err(WorkVcsError::GoalInvalid(format!(
            "goal entity {goal_entity_id} version {goal_entity_version_id} has state schema version {state_schema_version}"
        )));
    }
    let entity_workspace_id = decode_workspace_id("entity.workspace_id", entity_workspace_id)?;
    if entity_workspace_id != workspace_id {
        return Err(WorkVcsError::GoalInvalid(format!(
            "goal entity {goal_entity_id} belongs to workspace {entity_workspace_id}, not {workspace_id}"
        )));
    }

    let state_digest = decode_digest("entity_version.state_digest", state_digest)?;
    validate_import_fixed_point(
        state_json.as_bytes(),
        state_digest,
        ImportDigestDomain::EntityVersion,
    )
    .map_err(|error| {
        WorkVcsError::GoalInvalid(format!(
            "goal entity {goal_entity_id} version {goal_entity_version_id} fixed-point validation failed: {error}"
        ))
    })?;
    let value = parse_canonical_json(state_json.as_bytes()).map_err(goal_invalid_from)?;
    let actual = entity_version_digest(&value).map_err(goal_invalid_from)?;
    if actual != state_digest {
        return Err(WorkVcsError::GoalInvalid(format!(
            "goal entity {goal_entity_id} version {goal_entity_version_id} digest does not match state JSON"
        )));
    }

    Ok(LoadedGoalVersion {
        state_digest,
        state: parse_goal_state(value)?,
    })
}

fn parse_goal_state(value: CanonicalValue) -> Result<GoalState> {
    let CanonicalValue::Object(entries) = value else {
        return Err(WorkVcsError::GoalInvalid(
            "goal state must be a canonical object".to_owned(),
        ));
    };
    if entries.len() != 6 {
        return Err(WorkVcsError::GoalInvalid(format!(
            "goal state must contain exactly 6 fields, found {}",
            entries.len()
        )));
    }

    let mut description = None;
    let mut plan_refs = None;
    let mut status = None;
    let mut subgoals = None;
    let mut terminal_rationale = None;
    let mut work_refs = None;

    for (key, value) in entries {
        match key.as_str() {
            "description" => {
                let value = require_string("description", value)?;
                validate_description(&value)?;
                description = Some(value);
            }
            "plan_refs" => {
                require_empty_array("plan_refs", &value)?;
                plan_refs = Some(());
            }
            "status" => {
                let value = require_string("status", value)?;
                status = Some(GoalStatus::parse(&value)?);
            }
            "subgoals" => {
                require_empty_array("subgoals", &value)?;
                subgoals = Some(());
            }
            "terminal_rationale" => {
                terminal_rationale = Some(match value {
                    CanonicalValue::Null => None,
                    CanonicalValue::String(value) => {
                        validate_terminal_rationale(&value)?;
                        Some(value)
                    }
                    _ => {
                        return Err(WorkVcsError::GoalInvalid(
                            "terminal_rationale must be null or a string".to_owned(),
                        ));
                    }
                });
            }
            "work_refs" => {
                require_empty_array("work_refs", &value)?;
                work_refs = Some(());
            }
            other => {
                return Err(WorkVcsError::GoalInvalid(format!(
                    "goal state contains unsupported field {other:?}"
                )));
            }
        }
    }

    plan_refs
        .ok_or_else(|| WorkVcsError::GoalInvalid("goal state is missing plan_refs".to_owned()))?;
    subgoals
        .ok_or_else(|| WorkVcsError::GoalInvalid("goal state is missing subgoals".to_owned()))?;
    work_refs
        .ok_or_else(|| WorkVcsError::GoalInvalid("goal state is missing work_refs".to_owned()))?;

    Ok(GoalState {
        description: description.ok_or_else(|| {
            WorkVcsError::GoalInvalid("goal state is missing description".to_owned())
        })?,
        status: status
            .ok_or_else(|| WorkVcsError::GoalInvalid("goal state is missing status".to_owned()))?,
        terminal_rationale: terminal_rationale.ok_or_else(|| {
            WorkVcsError::GoalInvalid("goal state is missing terminal_rationale".to_owned())
        })?,
    })
}

fn require_string(field: &str, value: CanonicalValue) -> Result<String> {
    match value {
        CanonicalValue::String(value) => Ok(value),
        _ => Err(WorkVcsError::GoalInvalid(format!(
            "{field} must be a string"
        ))),
    }
}

fn require_empty_array(field: &str, value: &CanonicalValue) -> Result<()> {
    match value {
        CanonicalValue::Array(values) if values.is_empty() => Ok(()),
        CanonicalValue::Array(_) => Err(WorkVcsError::GoalInvalid(format!(
            "{field} is deferred and must be an empty array in Phase 3M"
        ))),
        _ => Err(WorkVcsError::GoalInvalid(format!(
            "{field} must be an array"
        ))),
    }
}

fn validate_description(value: &str) -> Result<()> {
    validate_non_empty_text("goal description", value)
}

fn validate_terminal_rationale(value: &str) -> Result<()> {
    if value.contains('\0') {
        return Err(WorkVcsError::GoalInvalid(
            "goal terminal rationale must not contain NUL".to_owned(),
        ));
    }
    Ok(())
}

fn validate_non_empty_text(field: &str, value: &str) -> Result<()> {
    if value.trim().is_empty() {
        return Err(WorkVcsError::GoalInvalid(format!(
            "{field} must not be empty"
        )));
    }
    if value.contains('\0') {
        return Err(WorkVcsError::GoalInvalid(format!(
            "{field} must not contain NUL"
        )));
    }
    Ok(())
}

fn goal_invalid_from(error: WorkVcsError) -> WorkVcsError {
    WorkVcsError::GoalInvalid(error.to_string())
}

fn decode_workspace_id(column: &str, bytes: Vec<u8>) -> Result<WorkspaceId> {
    let bytes = decode_16(column, bytes)?;
    WorkspaceId::from_bytes(bytes).map_err(goal_invalid_from)
}

fn decode_digest(column: &str, bytes: Vec<u8>) -> Result<Digest> {
    let bytes: [u8; 32] = bytes.try_into().map_err(|bytes: Vec<u8>| {
        WorkVcsError::GoalInvalid(format!("{column} must be 32 bytes, found {}", bytes.len()))
    })?;
    Ok(Digest::from_bytes(bytes))
}

fn decode_16(column: &str, bytes: Vec<u8>) -> Result<[u8; 16]> {
    bytes.try_into().map_err(|bytes: Vec<u8>| {
        WorkVcsError::GoalInvalid(format!("{column} must be 16 bytes, found {}", bytes.len()))
    })
}
