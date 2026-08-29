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

pub(crate) const RECORD_ENTITY_KIND: &str = "record";

const ENTITY_OBJECT_KIND: &str = "entity";
const RECORD_STATE_SCHEMA_VERSION: i64 = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RecordKind {
    Assumption,
    Finding,
}

impl RecordKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Assumption => "assumption",
            Self::Finding => "finding",
        }
    }

    fn parse(value: &str) -> Result<Self> {
        match value {
            "assumption" => Ok(Self::Assumption),
            "finding" => Ok(Self::Finding),
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
    Invalidated,
    Unverified,
    Validated,
}

impl RecordStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Invalidated => "invalidated",
            Self::Unverified => "unverified",
            Self::Validated => "validated",
        }
    }

    fn parse(value: &str) -> Result<Self> {
        match value {
            "active" => Ok(Self::Active),
            "invalidated" => Ok(Self::Invalidated),
            "unverified" => Ok(Self::Unverified),
            "validated" => Ok(Self::Validated),
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

    pub fn with_rationale(mut self, rationale: CanonicalValue) -> Self {
        self.rationale = rationale;
        self
    }
}

impl RecordCreateOptions {
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

    let next_state = current
        .state
        .transition_assumption(options.next_status, &options.rationale_text)?;
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

fn validate_record_status_for_kind(kind: RecordKind, status: RecordStatus) -> Result<()> {
    match (kind, status) {
        (RecordKind::Finding, RecordStatus::Active)
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

fn decode_digest(column: &str, bytes: Vec<u8>) -> Result<Digest> {
    let bytes = bytes.try_into().map_err(|bytes: Vec<u8>| {
        WorkVcsError::RecordInvalid(format!("{column} must be 32 bytes, found {}", bytes.len()))
    })?;
    Ok(Digest::from_bytes(bytes))
}
