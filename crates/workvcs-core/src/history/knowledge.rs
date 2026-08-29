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

pub(crate) const KNOWLEDGE_ENTITY_KIND: &str = "knowledge";

const ENTITY_OBJECT_KIND: &str = "entity";
const KNOWLEDGE_STATE_SCHEMA_VERSION: i64 = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KnowledgeStatus {
    Active,
    Invalidated,
    Superseded,
}

impl KnowledgeStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Invalidated => "invalidated",
            Self::Superseded => "superseded",
        }
    }

    fn parse(value: &str) -> Result<Self> {
        match value {
            "active" => Ok(Self::Active),
            "invalidated" => Ok(Self::Invalidated),
            "superseded" => Ok(Self::Superseded),
            other => Err(WorkVcsError::KnowledgeInvalid(format!(
                "knowledge status {other:?} is not in the confirmed lifecycle vocabulary"
            ))),
        }
    }
}

impl fmt::Display for KnowledgeStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KnowledgeState {
    pub statement: String,
    pub scope: CanonicalValue,
    pub status: KnowledgeStatus,
    pub provenance: CanonicalValue,
}

impl KnowledgeState {
    pub fn active(statement: impl Into<String>) -> Result<Self> {
        let statement = statement.into();
        validate_statement(&statement)?;
        Ok(Self {
            statement,
            scope: CanonicalValue::object(Vec::new())?,
            status: KnowledgeStatus::Active,
            provenance: CanonicalValue::object(Vec::new())?,
        })
    }

    pub fn with_scope(mut self, scope: CanonicalValue) -> Result<Self> {
        require_object_value("knowledge scope", &scope)?;
        self.scope = scope;
        Ok(self)
    }

    pub fn with_provenance(mut self, provenance: CanonicalValue) -> Result<Self> {
        require_object_value("knowledge provenance", &provenance)?;
        self.provenance = provenance;
        Ok(self)
    }

    pub fn to_canonical_value(&self) -> Result<CanonicalValue> {
        validate_statement(&self.statement)?;
        require_object_value("knowledge scope", &self.scope)?;
        require_object_value("knowledge provenance", &self.provenance)?;
        CanonicalValue::object(vec![
            ("provenance".to_owned(), self.provenance.clone()),
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
        .map_err(knowledge_invalid_from)
    }

    fn transition(&self, next_status: KnowledgeStatus, rationale_text: &str) -> Result<Self> {
        validate_transition_rationale(rationale_text)?;
        validate_knowledge_lifecycle_transition(self.status, next_status)?;
        let next = Self {
            statement: self.statement.clone(),
            scope: self.scope.clone(),
            status: next_status,
            provenance: self.provenance.clone(),
        };
        if next == *self {
            return Err(WorkVcsError::KnowledgeInvalid(
                "knowledge transition must change status".to_owned(),
            ));
        }
        Ok(next)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KnowledgeCreateOptions {
    branch_id: BranchId,
    expected_head_commit_id: CommitId,
    state: KnowledgeState,
    rationale: CanonicalValue,
}

impl KnowledgeCreateOptions {
    pub fn new(
        branch_id: BranchId,
        expected_head_commit_id: CommitId,
        statement: impl Into<String>,
    ) -> Result<Self> {
        Ok(Self {
            branch_id,
            expected_head_commit_id,
            state: KnowledgeState::active(statement)?,
            rationale: CanonicalValue::object(Vec::new())?,
        })
    }

    pub fn with_scope(mut self, scope: CanonicalValue) -> Result<Self> {
        self.state = self.state.with_scope(scope)?;
        Ok(self)
    }

    pub fn with_provenance(mut self, provenance: CanonicalValue) -> Result<Self> {
        self.state = self.state.with_provenance(provenance)?;
        Ok(self)
    }

    pub fn with_rationale(mut self, rationale: CanonicalValue) -> Self {
        self.rationale = rationale;
        self
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KnowledgeTransitionOptions {
    branch_id: BranchId,
    expected_head_commit_id: CommitId,
    knowledge_entity_id: EntityId,
    expected_knowledge_entity_version_id: EntityVersionId,
    next_status: KnowledgeStatus,
    rationale_text: String,
    rationale: CanonicalValue,
}

impl KnowledgeTransitionOptions {
    pub fn invalidate(
        branch_id: BranchId,
        expected_head_commit_id: CommitId,
        knowledge_entity_id: EntityId,
        expected_knowledge_entity_version_id: EntityVersionId,
        rationale: impl Into<String>,
    ) -> Result<Self> {
        Self::new(
            branch_id,
            expected_head_commit_id,
            knowledge_entity_id,
            expected_knowledge_entity_version_id,
            KnowledgeStatus::Invalidated,
            rationale,
        )
    }

    pub fn supersede(
        branch_id: BranchId,
        expected_head_commit_id: CommitId,
        knowledge_entity_id: EntityId,
        expected_knowledge_entity_version_id: EntityVersionId,
        rationale: impl Into<String>,
    ) -> Result<Self> {
        Self::new(
            branch_id,
            expected_head_commit_id,
            knowledge_entity_id,
            expected_knowledge_entity_version_id,
            KnowledgeStatus::Superseded,
            rationale,
        )
    }

    fn new(
        branch_id: BranchId,
        expected_head_commit_id: CommitId,
        knowledge_entity_id: EntityId,
        expected_knowledge_entity_version_id: EntityVersionId,
        next_status: KnowledgeStatus,
        rationale: impl Into<String>,
    ) -> Result<Self> {
        let rationale_text = rationale.into();
        validate_transition_rationale(&rationale_text)?;
        Ok(Self {
            branch_id,
            expected_head_commit_id,
            knowledge_entity_id,
            expected_knowledge_entity_version_id,
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KnowledgeCreateCommit {
    pub workspace_id: WorkspaceId,
    pub branch_id: BranchId,
    pub previous_head_commit_id: CommitId,
    pub commit_id: CommitId,
    pub changeset_id: ChangeSetId,
    pub operation_id: OperationId,
    pub knowledge_entity_id: EntityId,
    pub knowledge_entity_version_id: EntityVersionId,
    pub knowledge_state_digest: Digest,
    pub work_state_digest: Digest,
    pub state: KnowledgeState,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KnowledgeTransitionCommit {
    pub workspace_id: WorkspaceId,
    pub branch_id: BranchId,
    pub previous_head_commit_id: CommitId,
    pub commit_id: CommitId,
    pub changeset_id: ChangeSetId,
    pub operation_id: OperationId,
    pub knowledge_entity_id: EntityId,
    pub previous_knowledge_entity_version_id: EntityVersionId,
    pub knowledge_entity_version_id: EntityVersionId,
    pub knowledge_state_digest: Digest,
    pub work_state_digest: Digest,
    pub previous_state: KnowledgeState,
    pub state: KnowledgeState,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KnowledgeSnapshot {
    pub workspace_id: WorkspaceId,
    pub commit_id: CommitId,
    pub knowledge_entity_id: EntityId,
    pub knowledge_entity_version_id: EntityVersionId,
    pub state_digest: Digest,
    pub state: KnowledgeState,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KnowledgeListOptions {
    commit_id: CommitId,
    status: Option<KnowledgeStatus>,
    statement_contains: Option<String>,
}

impl KnowledgeListOptions {
    pub fn new(commit_id: CommitId) -> Self {
        Self {
            commit_id,
            status: None,
            statement_contains: None,
        }
    }

    pub fn with_status(mut self, status: KnowledgeStatus) -> Self {
        self.status = Some(status);
        self
    }

    pub fn with_statement_contains(mut self, fragment: impl Into<String>) -> Result<Self> {
        let fragment = fragment.into();
        if fragment.trim().is_empty() {
            return Err(WorkVcsError::KnowledgeInvalid(
                "knowledge statement filter must not be empty".to_owned(),
            ));
        }
        self.statement_contains = Some(fragment);
        Ok(self)
    }

    pub fn commit_id(&self) -> CommitId {
        self.commit_id
    }

    pub fn status(&self) -> Option<KnowledgeStatus> {
        self.status
    }

    pub fn statement_contains(&self) -> Option<&str> {
        self.statement_contains.as_deref()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KnowledgeListResult {
    pub workspace_id: WorkspaceId,
    pub commit_id: CommitId,
    pub knowledge: Vec<KnowledgeSnapshot>,
}

pub(crate) fn create_knowledge(
    connection: &mut StoreConnection,
    options: &KnowledgeCreateOptions,
) -> Result<KnowledgeCreateCommit> {
    let entity_options = EntityTransitionOptions::create(
        options.branch_id,
        options.expected_head_commit_id,
        KNOWLEDGE_ENTITY_KIND,
        options.state.to_canonical_value()?,
    )?
    .with_rationale(options.rationale.clone());
    let commit = commit_entity_transition(connection, &entity_options)?;

    Ok(KnowledgeCreateCommit {
        workspace_id: commit.workspace_id,
        branch_id: commit.branch_id,
        previous_head_commit_id: commit.previous_head_commit_id,
        commit_id: commit.commit_id,
        changeset_id: commit.changeset_id,
        operation_id: commit.operation_id,
        knowledge_entity_id: commit.entity_id,
        knowledge_entity_version_id: commit.entity_version_id,
        knowledge_state_digest: commit.entity_state_digest,
        work_state_digest: commit.work_state_digest,
        state: options.state.clone(),
    })
}

pub(crate) fn transition_knowledge(
    connection: &mut StoreConnection,
    options: &KnowledgeTransitionOptions,
) -> Result<KnowledgeTransitionCommit> {
    require_non_empty_rationale_object(&options.rationale)?;
    let current = knowledge_at(
        connection,
        options.expected_head_commit_id,
        options.knowledge_entity_id,
    )?;
    if current.knowledge_entity_version_id != options.expected_knowledge_entity_version_id {
        return Err(WorkVcsError::KnowledgeInvalid(format!(
            "knowledge entity {} expected version {}, found {} at commit {}",
            options.knowledge_entity_id,
            options.expected_knowledge_entity_version_id,
            current.knowledge_entity_version_id,
            options.expected_head_commit_id
        )));
    }

    let next_state = current
        .state
        .transition(options.next_status, &options.rationale_text)?;
    let entity_options = EntityTransitionOptions::update(
        options.branch_id,
        options.expected_head_commit_id,
        options.knowledge_entity_id,
        options.expected_knowledge_entity_version_id,
        next_state.to_canonical_value()?,
    )?
    .with_rationale(options.rationale.clone());
    let commit = commit_entity_transition(connection, &entity_options)?;

    Ok(KnowledgeTransitionCommit {
        workspace_id: commit.workspace_id,
        branch_id: commit.branch_id,
        previous_head_commit_id: commit.previous_head_commit_id,
        commit_id: commit.commit_id,
        changeset_id: commit.changeset_id,
        operation_id: commit.operation_id,
        knowledge_entity_id: commit.entity_id,
        previous_knowledge_entity_version_id: options.expected_knowledge_entity_version_id,
        knowledge_entity_version_id: commit.entity_version_id,
        knowledge_state_digest: commit.entity_state_digest,
        work_state_digest: commit.work_state_digest,
        previous_state: current.state,
        state: next_state,
    })
}

pub(crate) fn knowledge_at(
    connection: &StoreConnection,
    commit_id: CommitId,
    knowledge_entity_id: EntityId,
) -> Result<KnowledgeSnapshot> {
    let replayed = state_at(connection, commit_id)?;
    let Some(knowledge_entity_version_id) =
        replayed
            .state
            .entities()
            .iter()
            .find_map(|(entity_id, entity_version_id)| {
                (*entity_id == knowledge_entity_id).then_some(*entity_version_id)
            })
    else {
        return Err(WorkVcsError::KnowledgeNotFound(format!(
            "knowledge entity {knowledge_entity_id} is not present at commit {commit_id}"
        )));
    };

    let loaded = load_knowledge_version(
        connection,
        replayed.workspace_id,
        knowledge_entity_id,
        knowledge_entity_version_id,
    )?;
    Ok(KnowledgeSnapshot {
        workspace_id: replayed.workspace_id,
        commit_id,
        knowledge_entity_id,
        knowledge_entity_version_id,
        state_digest: loaded.state_digest,
        state: loaded.state,
    })
}

pub(crate) fn knowledges_at(
    connection: &StoreConnection,
    options: &KnowledgeListOptions,
) -> Result<KnowledgeListResult> {
    let commit_id = options.commit_id();
    let replayed = state_at(connection, commit_id)?;
    let mut knowledge = Vec::new();

    for (entity_id, entity_version_id) in replayed.state.entities() {
        match load_entity_kind(connection, *entity_id)? {
            Some(entity_kind) if entity_kind == KNOWLEDGE_ENTITY_KIND => {
                let loaded = load_knowledge_version(
                    connection,
                    replayed.workspace_id,
                    *entity_id,
                    *entity_version_id,
                )?;
                if options
                    .status()
                    .is_none_or(|status| loaded.state.status == status)
                    && options
                        .statement_contains()
                        .is_none_or(|fragment| loaded.state.statement.contains(fragment))
                {
                    knowledge.push(KnowledgeSnapshot {
                        workspace_id: replayed.workspace_id,
                        commit_id,
                        knowledge_entity_id: *entity_id,
                        knowledge_entity_version_id: *entity_version_id,
                        state_digest: loaded.state_digest,
                        state: loaded.state,
                    });
                }
            }
            Some(_) => {}
            None => {
                return Err(WorkVcsError::KnowledgeInvalid(format!(
                    "WorkState at commit {commit_id} references missing entity {entity_id}"
                )));
            }
        }
    }

    Ok(KnowledgeListResult {
        workspace_id: replayed.workspace_id,
        commit_id,
        knowledge,
    })
}

struct LoadedKnowledgeVersion {
    state_digest: Digest,
    state: KnowledgeState,
}

fn load_knowledge_version(
    connection: &StoreConnection,
    workspace_id: WorkspaceId,
    knowledge_entity_id: EntityId,
    knowledge_entity_version_id: EntityVersionId,
) -> Result<LoadedKnowledgeVersion> {
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
                &knowledge_entity_id.raw_bytes()[..],
                &knowledge_entity_version_id.raw_bytes()[..]
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
        return Err(WorkVcsError::KnowledgeNotFound(format!(
            "knowledge entity {knowledge_entity_id} version {knowledge_entity_version_id} does not exist"
        )));
    };
    if object_kind != ENTITY_OBJECT_KIND {
        return Err(WorkVcsError::KnowledgeInvalid(format!(
            "knowledge entity {knowledge_entity_id} has object kind {object_kind:?}"
        )));
    }
    if entity_kind != KNOWLEDGE_ENTITY_KIND {
        return Err(WorkVcsError::KnowledgeNotFound(format!(
            "entity {knowledge_entity_id} has kind {entity_kind:?}, not {KNOWLEDGE_ENTITY_KIND:?}"
        )));
    }
    if state_schema_version != KNOWLEDGE_STATE_SCHEMA_VERSION {
        return Err(WorkVcsError::KnowledgeInvalid(format!(
            "knowledge entity {knowledge_entity_id} version {knowledge_entity_version_id} has state schema version {state_schema_version}"
        )));
    }
    let entity_workspace_id = decode_workspace_id("entity.workspace_id", entity_workspace_id)?;
    if entity_workspace_id != workspace_id {
        return Err(WorkVcsError::KnowledgeInvalid(format!(
            "knowledge entity {knowledge_entity_id} belongs to workspace {entity_workspace_id}, not {workspace_id}"
        )));
    }

    let state_digest = decode_digest("entity_version.state_digest", state_digest)?;
    validate_import_fixed_point(
        state_json.as_bytes(),
        state_digest,
        ImportDigestDomain::EntityVersion,
    )
    .map_err(|error| {
        WorkVcsError::KnowledgeInvalid(format!(
            "knowledge entity {knowledge_entity_id} version {knowledge_entity_version_id} fixed-point validation failed: {error}"
        ))
    })?;
    let value = parse_canonical_json(state_json.as_bytes()).map_err(knowledge_invalid_from)?;
    let actual = entity_version_digest(&value).map_err(knowledge_invalid_from)?;
    if actual != state_digest {
        return Err(WorkVcsError::KnowledgeInvalid(format!(
            "knowledge entity {knowledge_entity_id} version {knowledge_entity_version_id} digest does not match state JSON"
        )));
    }

    Ok(LoadedKnowledgeVersion {
        state_digest,
        state: parse_knowledge_state(value)?,
    })
}

fn parse_knowledge_state(value: CanonicalValue) -> Result<KnowledgeState> {
    let CanonicalValue::Object(entries) = value else {
        return Err(WorkVcsError::KnowledgeInvalid(
            "knowledge state must be a canonical object".to_owned(),
        ));
    };
    if entries.len() != 4 {
        return Err(WorkVcsError::KnowledgeInvalid(format!(
            "knowledge state must contain exactly 4 fields, found {}",
            entries.len()
        )));
    }

    let mut provenance = None;
    let mut scope = None;
    let mut statement = None;
    let mut status = None;

    for (key, value) in entries {
        match key.as_str() {
            "provenance" => {
                require_object_value("knowledge provenance", &value)?;
                provenance = Some(value);
            }
            "scope" => {
                require_object_value("knowledge scope", &value)?;
                scope = Some(value);
            }
            "statement" => {
                let value = require_string("statement", value)?;
                validate_statement(&value)?;
                statement = Some(value);
            }
            "status" => {
                let value = require_string("status", value)?;
                status = Some(KnowledgeStatus::parse(&value)?);
            }
            other => {
                return Err(WorkVcsError::KnowledgeInvalid(format!(
                    "knowledge state contains unsupported field {other:?}"
                )));
            }
        }
    }

    Ok(KnowledgeState {
        provenance: provenance.ok_or_else(|| missing_field("provenance"))?,
        scope: scope.ok_or_else(|| missing_field("scope"))?,
        statement: statement.ok_or_else(|| missing_field("statement"))?,
        status: status.ok_or_else(|| missing_field("status"))?,
    })
}

fn validate_statement(value: &str) -> Result<()> {
    if value.trim().is_empty() {
        return Err(WorkVcsError::KnowledgeInvalid(
            "knowledge statement must not be empty".to_owned(),
        ));
    }
    if value.contains('\0') {
        return Err(WorkVcsError::KnowledgeInvalid(
            "knowledge statement must not contain NUL".to_owned(),
        ));
    }
    Ok(())
}

fn validate_transition_rationale(value: &str) -> Result<()> {
    if value.trim().is_empty() {
        return Err(WorkVcsError::KnowledgeInvalid(
            "knowledge transition rationale must not be empty".to_owned(),
        ));
    }
    if value.contains('\0') {
        return Err(WorkVcsError::KnowledgeInvalid(
            "knowledge transition rationale must not contain NUL".to_owned(),
        ));
    }
    Ok(())
}

fn validate_knowledge_lifecycle_transition(
    current: KnowledgeStatus,
    next: KnowledgeStatus,
) -> Result<()> {
    match (current, next) {
        (KnowledgeStatus::Active, KnowledgeStatus::Invalidated) => Ok(()),
        (KnowledgeStatus::Active, KnowledgeStatus::Superseded) => Ok(()),
        (current, next) => Err(WorkVcsError::KnowledgeInvalid(format!(
            "knowledge transition {current:?} -> {next:?} is not allowed in this slice"
        ))),
    }
}

fn require_non_empty_rationale_object(value: &CanonicalValue) -> Result<()> {
    match value {
        CanonicalValue::Object(entries) if !entries.is_empty() => Ok(()),
        CanonicalValue::Object(_) => Err(WorkVcsError::KnowledgeInvalid(
            "knowledge lifecycle transition requires a non-empty rationale object".to_owned(),
        )),
        _ => Err(WorkVcsError::KnowledgeInvalid(
            "knowledge lifecycle transition requires a rationale object".to_owned(),
        )),
    }
}

fn rationale_value(reason: &str) -> Result<CanonicalValue> {
    validate_transition_rationale(reason)?;
    CanonicalValue::object(vec![(
        "reason".to_owned(),
        CanonicalValue::String(reason.to_owned()),
    )])
    .map_err(knowledge_invalid_from)
}

fn require_string(field: &str, value: CanonicalValue) -> Result<String> {
    match value {
        CanonicalValue::String(value) => Ok(value),
        other => Err(WorkVcsError::KnowledgeInvalid(format!(
            "knowledge field {field} must be a string, found {other:?}"
        ))),
    }
}

fn require_object_value(label: &str, value: &CanonicalValue) -> Result<()> {
    match value {
        CanonicalValue::Object(_) => Ok(()),
        other => Err(WorkVcsError::KnowledgeInvalid(format!(
            "{label} must be a canonical object, found {other:?}"
        ))),
    }
}

fn missing_field(field: &str) -> WorkVcsError {
    WorkVcsError::KnowledgeInvalid(format!("knowledge state is missing {field}"))
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

fn knowledge_invalid_from(error: WorkVcsError) -> WorkVcsError {
    WorkVcsError::KnowledgeInvalid(error.to_string())
}

fn decode_workspace_id(column: &str, bytes: Vec<u8>) -> Result<WorkspaceId> {
    let bytes = decode_16(column, bytes)?;
    WorkspaceId::from_bytes(bytes).map_err(knowledge_invalid_from)
}

fn decode_digest(column: &str, bytes: Vec<u8>) -> Result<Digest> {
    let bytes: [u8; 32] = bytes.try_into().map_err(|bytes: Vec<u8>| {
        WorkVcsError::KnowledgeInvalid(format!("{column} must be 32 bytes, found {}", bytes.len()))
    })?;
    Ok(Digest::from_bytes(bytes))
}

fn decode_16(column: &str, bytes: Vec<u8>) -> Result<[u8; 16]> {
    bytes.try_into().map_err(|bytes: Vec<u8>| {
        WorkVcsError::KnowledgeInvalid(format!("{column} must be 16 bytes, found {}", bytes.len()))
    })
}
