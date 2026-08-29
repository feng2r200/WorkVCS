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
    Finding,
}

impl RecordKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Finding => "finding",
        }
    }

    fn parse(value: &str) -> Result<Self> {
        match value {
            "finding" => Ok(Self::Finding),
            other => Err(WorkVcsError::RecordInvalid(format!(
                "record kind {other:?} is not implemented by the Phase 3AL semantic API"
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
}

impl RecordStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
        }
    }

    fn parse(value: &str) -> Result<Self> {
        match value {
            "active" => Ok(Self::Active),
            other => Err(WorkVcsError::RecordInvalid(format!(
                "record status {other:?} is not in the Phase 3AL lifecycle vocabulary"
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
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecordCreateOptions {
    branch_id: BranchId,
    expected_head_commit_id: CommitId,
    state: RecordState,
    rationale: CanonicalValue,
}

impl RecordCreateOptions {
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
}

fn validate_statement(value: &str) -> Result<()> {
    if value.trim().is_empty() {
        return Err(WorkVcsError::RecordInvalid(
            "record statement must not be empty".to_owned(),
        ));
    }
    Ok(())
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
