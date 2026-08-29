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

pub(crate) const PLAN_ENTITY_KIND: &str = "plan";

const ENTITY_OBJECT_KIND: &str = "entity";
const PLAN_STATE_SCHEMA_VERSION: i64 = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlanStatus {
    Active,
    Completed,
    Abandoned,
    Superseded,
}

impl PlanStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Completed => "completed",
            Self::Abandoned => "abandoned",
            Self::Superseded => "superseded",
        }
    }

    fn parse(value: &str) -> Result<Self> {
        match value {
            "active" => Ok(Self::Active),
            "completed" => Ok(Self::Completed),
            "abandoned" => Ok(Self::Abandoned),
            "superseded" => Ok(Self::Superseded),
            other => Err(WorkVcsError::PlanInvalid(format!(
                "plan status {other:?} is not in the confirmed lifecycle vocabulary"
            ))),
        }
    }
}

impl fmt::Display for PlanStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlanState {
    pub description: String,
    pub status: PlanStatus,
    pub constraints: Vec<String>,
    pub strategy: String,
    pub completion_rationale: Option<String>,
}

impl PlanState {
    pub fn active(description: impl Into<String>, strategy: impl Into<String>) -> Result<Self> {
        let description = description.into();
        let strategy = strategy.into();
        validate_description(&description)?;
        validate_strategy(&strategy)?;
        Ok(Self {
            description,
            status: PlanStatus::Active,
            constraints: Vec::new(),
            strategy,
            completion_rationale: None,
        })
    }

    pub fn to_canonical_value(&self) -> Result<CanonicalValue> {
        validate_description(&self.description)?;
        validate_strategy(&self.strategy)?;
        validate_constraints(&self.constraints)?;
        if let Some(completion_rationale) = &self.completion_rationale {
            validate_completion_rationale(completion_rationale)?;
        }

        let constraints = self
            .constraints
            .iter()
            .map(|value| CanonicalValue::String(value.clone()))
            .collect();
        let completion_rationale = self
            .completion_rationale
            .as_ref()
            .map(|value| CanonicalValue::String(value.clone()))
            .unwrap_or(CanonicalValue::Null);

        CanonicalValue::object(vec![
            ("assumptions".to_owned(), CanonicalValue::Array(Vec::new())),
            ("child_order".to_owned(), CanonicalValue::Array(Vec::new())),
            ("completion_rationale".to_owned(), completion_rationale),
            ("constraints".to_owned(), CanonicalValue::Array(constraints)),
            (
                "description".to_owned(),
                CanonicalValue::String(self.description.clone()),
            ),
            (
                "status".to_owned(),
                CanonicalValue::String(self.status.as_str().to_owned()),
            ),
            (
                "strategy".to_owned(),
                CanonicalValue::String(self.strategy.clone()),
            ),
            ("task_refs".to_owned(), CanonicalValue::Array(Vec::new())),
        ])
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlanCreateOptions {
    branch_id: BranchId,
    expected_head_commit_id: CommitId,
    state: PlanState,
    rationale: CanonicalValue,
}

impl PlanCreateOptions {
    pub fn new(
        branch_id: BranchId,
        expected_head_commit_id: CommitId,
        description: impl Into<String>,
        strategy: impl Into<String>,
    ) -> Result<Self> {
        Ok(Self {
            branch_id,
            expected_head_commit_id,
            state: PlanState::active(description, strategy)?,
            rationale: CanonicalValue::object(Vec::new())?,
        })
    }

    pub fn with_constraints<I, S>(mut self, constraints: I) -> Result<Self>
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let constraints = constraints.into_iter().map(Into::into).collect::<Vec<_>>();
        validate_constraints(&constraints)?;
        self.state.constraints = constraints;
        Ok(self)
    }

    pub fn with_rationale(mut self, rationale: CanonicalValue) -> Self {
        self.rationale = rationale;
        self
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlanCreateCommit {
    pub workspace_id: WorkspaceId,
    pub branch_id: BranchId,
    pub previous_head_commit_id: CommitId,
    pub commit_id: CommitId,
    pub changeset_id: ChangeSetId,
    pub operation_id: OperationId,
    pub plan_entity_id: EntityId,
    pub plan_entity_version_id: EntityVersionId,
    pub plan_state_digest: Digest,
    pub work_state_digest: Digest,
    pub state: PlanState,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlanSnapshot {
    pub workspace_id: WorkspaceId,
    pub commit_id: CommitId,
    pub plan_entity_id: EntityId,
    pub plan_entity_version_id: EntityVersionId,
    pub state_digest: Digest,
    pub state: PlanState,
}

pub(crate) fn create_plan(
    connection: &mut StoreConnection,
    options: &PlanCreateOptions,
) -> Result<PlanCreateCommit> {
    let entity_options = EntityTransitionOptions::create(
        options.branch_id,
        options.expected_head_commit_id,
        PLAN_ENTITY_KIND,
        options.state.to_canonical_value()?,
    )?
    .with_rationale(options.rationale.clone());
    let commit = commit_entity_transition(connection, &entity_options)?;

    Ok(PlanCreateCommit {
        workspace_id: commit.workspace_id,
        branch_id: commit.branch_id,
        previous_head_commit_id: commit.previous_head_commit_id,
        commit_id: commit.commit_id,
        changeset_id: commit.changeset_id,
        operation_id: commit.operation_id,
        plan_entity_id: commit.entity_id,
        plan_entity_version_id: commit.entity_version_id,
        plan_state_digest: commit.entity_state_digest,
        work_state_digest: commit.work_state_digest,
        state: options.state.clone(),
    })
}

pub(crate) fn plan_at(
    connection: &StoreConnection,
    commit_id: CommitId,
    plan_entity_id: EntityId,
) -> Result<PlanSnapshot> {
    let replayed = state_at(connection, commit_id)?;
    let Some(plan_entity_version_id) =
        replayed
            .state
            .entities()
            .iter()
            .find_map(|(entity_id, entity_version_id)| {
                (*entity_id == plan_entity_id).then_some(*entity_version_id)
            })
    else {
        return Err(WorkVcsError::PlanNotFound(format!(
            "plan entity {plan_entity_id} is not present at commit {commit_id}"
        )));
    };

    let loaded = load_plan_version(
        connection,
        replayed.workspace_id,
        plan_entity_id,
        plan_entity_version_id,
    )?;
    Ok(PlanSnapshot {
        workspace_id: replayed.workspace_id,
        commit_id,
        plan_entity_id,
        plan_entity_version_id,
        state_digest: loaded.state_digest,
        state: loaded.state,
    })
}

struct LoadedPlanVersion {
    state_digest: Digest,
    state: PlanState,
}

fn load_plan_version(
    connection: &StoreConnection,
    workspace_id: WorkspaceId,
    plan_entity_id: EntityId,
    plan_entity_version_id: EntityVersionId,
) -> Result<LoadedPlanVersion> {
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
                &plan_entity_id.raw_bytes()[..],
                &plan_entity_version_id.raw_bytes()[..]
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
        return Err(WorkVcsError::PlanNotFound(format!(
            "plan entity {plan_entity_id} version {plan_entity_version_id} does not exist"
        )));
    };
    if object_kind != ENTITY_OBJECT_KIND {
        return Err(WorkVcsError::PlanInvalid(format!(
            "plan entity {plan_entity_id} has object kind {object_kind:?}"
        )));
    }
    if entity_kind != PLAN_ENTITY_KIND {
        return Err(WorkVcsError::PlanNotFound(format!(
            "entity {plan_entity_id} has kind {entity_kind:?}, not {PLAN_ENTITY_KIND:?}"
        )));
    }
    if state_schema_version != PLAN_STATE_SCHEMA_VERSION {
        return Err(WorkVcsError::PlanInvalid(format!(
            "plan entity {plan_entity_id} version {plan_entity_version_id} has state schema version {state_schema_version}"
        )));
    }
    let entity_workspace_id = decode_workspace_id("entity.workspace_id", entity_workspace_id)?;
    if entity_workspace_id != workspace_id {
        return Err(WorkVcsError::PlanInvalid(format!(
            "plan entity {plan_entity_id} belongs to workspace {entity_workspace_id}, not {workspace_id}"
        )));
    }

    let state_digest = decode_digest("entity_version.state_digest", state_digest)?;
    validate_import_fixed_point(
        state_json.as_bytes(),
        state_digest,
        ImportDigestDomain::EntityVersion,
    )
    .map_err(|error| {
        WorkVcsError::PlanInvalid(format!(
            "plan entity {plan_entity_id} version {plan_entity_version_id} fixed-point validation failed: {error}"
        ))
    })?;
    let value = parse_canonical_json(state_json.as_bytes()).map_err(plan_invalid_from)?;
    let actual = entity_version_digest(&value).map_err(plan_invalid_from)?;
    if actual != state_digest {
        return Err(WorkVcsError::PlanInvalid(format!(
            "plan entity {plan_entity_id} version {plan_entity_version_id} digest does not match state JSON"
        )));
    }

    Ok(LoadedPlanVersion {
        state_digest,
        state: parse_plan_state(value)?,
    })
}

fn parse_plan_state(value: CanonicalValue) -> Result<PlanState> {
    let CanonicalValue::Object(entries) = value else {
        return Err(WorkVcsError::PlanInvalid(
            "plan state must be a canonical object".to_owned(),
        ));
    };
    if entries.len() != 8 {
        return Err(WorkVcsError::PlanInvalid(format!(
            "plan state must contain exactly 8 fields, found {}",
            entries.len()
        )));
    }

    let mut assumptions = None;
    let mut child_order = None;
    let mut completion_rationale = None;
    let mut constraints = None;
    let mut description = None;
    let mut status = None;
    let mut strategy = None;
    let mut task_refs = None;

    for (key, value) in entries {
        match key.as_str() {
            "assumptions" => {
                require_empty_array("assumptions", &value)?;
                assumptions = Some(());
            }
            "child_order" => {
                require_empty_array("child_order", &value)?;
                child_order = Some(());
            }
            "completion_rationale" => {
                completion_rationale = Some(match value {
                    CanonicalValue::Null => None,
                    CanonicalValue::String(value) => {
                        validate_completion_rationale(&value)?;
                        Some(value)
                    }
                    _ => {
                        return Err(WorkVcsError::PlanInvalid(
                            "completion_rationale must be null or a string".to_owned(),
                        ));
                    }
                });
            }
            "constraints" => {
                constraints = Some(parse_constraints(value)?);
            }
            "description" => {
                let value = require_string("description", value)?;
                validate_description(&value)?;
                description = Some(value);
            }
            "status" => {
                let value = require_string("status", value)?;
                status = Some(PlanStatus::parse(&value)?);
            }
            "strategy" => {
                let value = require_string("strategy", value)?;
                validate_strategy(&value)?;
                strategy = Some(value);
            }
            "task_refs" => {
                require_empty_array("task_refs", &value)?;
                task_refs = Some(());
            }
            other => {
                return Err(WorkVcsError::PlanInvalid(format!(
                    "plan state contains unsupported field {other:?}"
                )));
            }
        }
    }

    assumptions
        .ok_or_else(|| WorkVcsError::PlanInvalid("plan state is missing assumptions".to_owned()))?;
    child_order
        .ok_or_else(|| WorkVcsError::PlanInvalid("plan state is missing child_order".to_owned()))?;
    task_refs
        .ok_or_else(|| WorkVcsError::PlanInvalid("plan state is missing task_refs".to_owned()))?;

    Ok(PlanState {
        description: description.ok_or_else(|| {
            WorkVcsError::PlanInvalid("plan state is missing description".to_owned())
        })?,
        status: status
            .ok_or_else(|| WorkVcsError::PlanInvalid("plan state is missing status".to_owned()))?,
        constraints: constraints.ok_or_else(|| {
            WorkVcsError::PlanInvalid("plan state is missing constraints".to_owned())
        })?,
        strategy: strategy.ok_or_else(|| {
            WorkVcsError::PlanInvalid("plan state is missing strategy".to_owned())
        })?,
        completion_rationale: completion_rationale.ok_or_else(|| {
            WorkVcsError::PlanInvalid("plan state is missing completion_rationale".to_owned())
        })?,
    })
}

fn parse_constraints(value: CanonicalValue) -> Result<Vec<String>> {
    let CanonicalValue::Array(values) = value else {
        return Err(WorkVcsError::PlanInvalid(
            "constraints must be an array".to_owned(),
        ));
    };
    let mut constraints = Vec::with_capacity(values.len());
    for value in values {
        let value = require_string("constraints[]", value)?;
        validate_constraint(&value)?;
        constraints.push(value);
    }
    Ok(constraints)
}

fn require_string(field: &str, value: CanonicalValue) -> Result<String> {
    match value {
        CanonicalValue::String(value) => Ok(value),
        _ => Err(WorkVcsError::PlanInvalid(format!(
            "{field} must be a string"
        ))),
    }
}

fn require_empty_array(field: &str, value: &CanonicalValue) -> Result<()> {
    match value {
        CanonicalValue::Array(values) if values.is_empty() => Ok(()),
        CanonicalValue::Array(_) => Err(WorkVcsError::PlanInvalid(format!(
            "{field} is deferred and must be an empty array in Phase 3J"
        ))),
        _ => Err(WorkVcsError::PlanInvalid(format!(
            "{field} must be an array"
        ))),
    }
}

fn validate_description(value: &str) -> Result<()> {
    validate_non_empty_text("plan description", value)
}

fn validate_strategy(value: &str) -> Result<()> {
    validate_non_empty_text("plan strategy", value)
}

fn validate_constraints(values: &[String]) -> Result<()> {
    for value in values {
        validate_constraint(value)?;
    }
    Ok(())
}

fn validate_constraint(value: &str) -> Result<()> {
    validate_non_empty_text("plan constraint", value)
}

fn validate_completion_rationale(value: &str) -> Result<()> {
    if value.contains('\0') {
        return Err(WorkVcsError::PlanInvalid(
            "plan completion rationale must not contain NUL".to_owned(),
        ));
    }
    Ok(())
}

fn validate_non_empty_text(field: &str, value: &str) -> Result<()> {
    if value.trim().is_empty() {
        return Err(WorkVcsError::PlanInvalid(format!(
            "{field} must not be empty"
        )));
    }
    if value.contains('\0') {
        return Err(WorkVcsError::PlanInvalid(format!(
            "{field} must not contain NUL"
        )));
    }
    Ok(())
}

fn plan_invalid_from(error: WorkVcsError) -> WorkVcsError {
    WorkVcsError::PlanInvalid(error.to_string())
}

fn decode_workspace_id(column: &str, bytes: Vec<u8>) -> Result<WorkspaceId> {
    let bytes = decode_16(column, bytes)?;
    WorkspaceId::from_bytes(bytes).map_err(plan_invalid_from)
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
