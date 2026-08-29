use super::entity::canonical_json_string;
use super::goal::{GOAL_ENTITY_KIND, goal_at};
use super::plan::{PLAN_ENTITY_KIND, plan_at};
use super::state_at;
use super::task::{TASK_ENTITY_KIND, task_at};
use crate::canonical::{
    CanonicalValue, ImportDigestDomain, WorkState, relation_version_digest,
    validate_import_fixed_point, work_state_mapping_digest,
};
use crate::error::{Result, WorkVcsError, storage_error};
use crate::identity::{
    BranchId, ChangeSetId, CommitId, Digest, EntityId, EventId, OperationId, RelationId,
    RelationVersionId, WorkspaceId,
};
use crate::store::{StoreConnection, current_epoch_micros};
use rusqlite::{OptionalExtension, Transaction, TransactionBehavior, params};
use std::collections::BTreeMap;
use std::fmt;

const ACTIVE_BRANCH_LIFECYCLE_STATE: &str = "active";
const EMPTY_FIELD_DELTA: &str = "{}";
const NORMAL_COMMIT_KIND: &str = "normal";
const PRIMARY_PARENT_ROLE: &str = "primary";
const REFERENCES_RELATION_TYPE: &str = "references";
const RELATION_OBJECT_KIND: &str = "relation";
const RELATION_STATE_SCHEMA_VERSION: i64 = 1;
const STRUCTURAL_REFERENCE_CREATE_EVENT_KIND: &str = "structural_reference.created";
const STRUCTURAL_REFERENCE_CREATE_OPERATION_SCHEMA_VERSION: i64 = 1;
pub(crate) const STRUCTURAL_REFERENCE_CREATE_OPERATION_TYPE: &str = "structural_reference.create";

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum StructuralReferenceEndpointKind {
    Goal,
    Plan,
    Task,
}

impl StructuralReferenceEndpointKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Goal => GOAL_ENTITY_KIND,
            Self::Plan => PLAN_ENTITY_KIND,
            Self::Task => TASK_ENTITY_KIND,
        }
    }

    fn parse(entity_kind: &str) -> Option<Self> {
        match entity_kind {
            GOAL_ENTITY_KIND => Some(Self::Goal),
            PLAN_ENTITY_KIND => Some(Self::Plan),
            TASK_ENTITY_KIND => Some(Self::Task),
            _ => None,
        }
    }
}

impl fmt::Display for StructuralReferenceEndpointKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StructuralReferenceCreateOptions {
    branch_id: BranchId,
    expected_head_commit_id: CommitId,
    referrer_entity_id: EntityId,
    target_entity_id: EntityId,
    rationale: CanonicalValue,
}

impl StructuralReferenceCreateOptions {
    pub fn new(
        branch_id: BranchId,
        expected_head_commit_id: CommitId,
        referrer_entity_id: EntityId,
        target_entity_id: EntityId,
    ) -> Result<Self> {
        if referrer_entity_id == target_entity_id {
            return Err(WorkVcsError::RelationInvalid(format!(
                "structural reference cannot use entity {referrer_entity_id} as both referrer and target"
            )));
        }
        Ok(Self {
            branch_id,
            expected_head_commit_id,
            referrer_entity_id,
            target_entity_id,
            rationale: CanonicalValue::object(Vec::new())?,
        })
    }

    pub fn with_rationale(mut self, rationale: CanonicalValue) -> Self {
        self.rationale = rationale;
        self
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StructuralReferenceCreateCommit {
    pub workspace_id: WorkspaceId,
    pub branch_id: BranchId,
    pub previous_head_commit_id: CommitId,
    pub commit_id: CommitId,
    pub changeset_id: ChangeSetId,
    pub operation_id: OperationId,
    pub relation_id: RelationId,
    pub relation_version_id: RelationVersionId,
    pub referrer_entity_id: EntityId,
    pub referrer_kind: StructuralReferenceEndpointKind,
    pub target_entity_id: EntityId,
    pub target_kind: StructuralReferenceEndpointKind,
    pub relation_state_digest: Digest,
    pub work_state_digest: Digest,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StructuralReferenceSnapshot {
    pub workspace_id: WorkspaceId,
    pub commit_id: CommitId,
    pub relation_id: RelationId,
    pub relation_version_id: RelationVersionId,
    pub referrer_entity_id: EntityId,
    pub referrer_kind: StructuralReferenceEndpointKind,
    pub target_entity_id: EntityId,
    pub target_kind: StructuralReferenceEndpointKind,
    pub state_digest: Digest,
}

pub(crate) fn create_structural_reference(
    connection: &mut StoreConnection,
    options: &StructuralReferenceCreateOptions,
) -> Result<StructuralReferenceCreateCommit> {
    connection.verify_foreign_keys()?;
    if options.referrer_entity_id == options.target_entity_id {
        return Err(WorkVcsError::RelationInvalid(format!(
            "structural reference cannot use entity {} as both referrer and target",
            options.referrer_entity_id
        )));
    }

    let parent = state_at(connection, options.expected_head_commit_id)?;
    let referrer = load_reference_endpoint(
        connection,
        options.expected_head_commit_id,
        options.referrer_entity_id,
    )?;
    let target = load_reference_endpoint(
        connection,
        options.expected_head_commit_id,
        options.target_entity_id,
    )?;
    if referrer.workspace_id != parent.workspace_id || target.workspace_id != parent.workspace_id {
        return Err(WorkVcsError::RelationInvalid(format!(
            "structural reference endpoints must belong to workspace {}",
            parent.workspace_id
        )));
    }
    validate_reference_kind_pair(
        referrer.kind,
        target.kind,
        options.referrer_entity_id,
        options.target_entity_id,
    )?;

    let relation_id = RelationId::new_v7();
    let relation_version_id = RelationVersionId::new_v7();
    let changeset_id = ChangeSetId::new_v7();
    let commit_id = CommitId::new_v7();
    let operation_id = OperationId::new_v7();
    let now_us = current_epoch_micros()?;

    let relation_state_value = CanonicalValue::object(Vec::new())?;
    let relation_state_json = canonical_json_string(&relation_state_value)?;
    let relation_state_digest = relation_version_digest(&relation_state_value)?;
    let next_work_state = work_state_after_structural_reference_create(
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
        return Err(WorkVcsError::RelationInvalid(format!(
            "branch {} belongs to workspace {}, but expected head {} belongs to workspace {}",
            options.branch_id,
            branch.workspace_id,
            options.expected_head_commit_id,
            parent.workspace_id
        )));
    }
    ensure_structural_reference_logical_key_available(
        &transaction,
        branch.workspace_id,
        options.referrer_entity_id,
        options.target_entity_id,
    )?;
    write_structural_reference_create(
        &transaction,
        &StructuralReferenceCreateRows {
            workspace_id: branch.workspace_id,
            expected_head_commit_id: options.expected_head_commit_id,
            relation_id,
            relation_version_id,
            relation_state_json,
            relation_state_digest,
            referrer_entity_id: options.referrer_entity_id,
            target_entity_id: options.target_entity_id,
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

    Ok(StructuralReferenceCreateCommit {
        workspace_id: branch.workspace_id,
        branch_id: options.branch_id,
        previous_head_commit_id: options.expected_head_commit_id,
        commit_id,
        changeset_id,
        operation_id,
        relation_id,
        relation_version_id,
        referrer_entity_id: options.referrer_entity_id,
        referrer_kind: referrer.kind,
        target_entity_id: options.target_entity_id,
        target_kind: target.kind,
        relation_state_digest,
        work_state_digest,
    })
}

pub(crate) fn structural_references_at(
    connection: &StoreConnection,
    commit_id: CommitId,
) -> Result<Vec<StructuralReferenceSnapshot>> {
    let replayed = state_at(connection, commit_id)?;
    let mut references = Vec::new();
    for (relation_id, relation_version_id) in replayed.state.relations() {
        let Some(reference) = load_structural_reference_version(
            connection,
            replayed.workspace_id,
            commit_id,
            *relation_id,
            *relation_version_id,
        )?
        else {
            continue;
        };
        references.push(reference);
    }

    references.sort_by(|left, right| {
        left.referrer_kind
            .cmp(&right.referrer_kind)
            .then_with(|| left.referrer_entity_id.cmp(&right.referrer_entity_id))
            .then_with(|| left.target_kind.cmp(&right.target_kind))
            .then_with(|| left.target_entity_id.cmp(&right.target_entity_id))
            .then_with(|| left.relation_id.cmp(&right.relation_id))
    });
    Ok(references)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ReferenceEndpoint {
    workspace_id: WorkspaceId,
    kind: StructuralReferenceEndpointKind,
}

struct StructuralReferenceCreateRows {
    workspace_id: WorkspaceId,
    expected_head_commit_id: CommitId,
    relation_id: RelationId,
    relation_version_id: RelationVersionId,
    relation_state_json: String,
    relation_state_digest: Digest,
    referrer_entity_id: EntityId,
    target_entity_id: EntityId,
    changeset_id: ChangeSetId,
    commit_id: CommitId,
    operation_id: OperationId,
    relation_payload_json: String,
    rationale_json: String,
    work_state_digest: Digest,
    now_us: i64,
}

struct BranchRow {
    workspace_id: WorkspaceId,
    head_commit_id: CommitId,
}

fn load_structural_reference_version(
    connection: &StoreConnection,
    workspace_id: WorkspaceId,
    commit_id: CommitId,
    relation_id: RelationId,
    relation_version_id: RelationVersionId,
) -> Result<Option<StructuralReferenceSnapshot>> {
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
        referrer_entity_id,
        target_entity_id,
        relation_discriminator,
        state_schema_version,
        metadata_json,
        state_digest,
    )) = row
    else {
        return Err(WorkVcsError::RelationInvalid(format!(
            "structural reference relation {relation_id} version {relation_version_id} does not exist"
        )));
    };
    if object_kind != RELATION_OBJECT_KIND {
        return Err(WorkVcsError::RelationInvalid(format!(
            "structural reference relation {relation_id} has object kind {object_kind:?}"
        )));
    }
    let relation_workspace_id =
        decode_workspace_id("relation.workspace_id", relation_workspace_id)?;
    if relation_workspace_id != workspace_id {
        return Err(WorkVcsError::RelationInvalid(format!(
            "structural reference relation {relation_id} belongs to workspace {relation_workspace_id}, not {workspace_id}"
        )));
    }
    if relation_type != REFERENCES_RELATION_TYPE {
        return Ok(None);
    }
    if !relation_discriminator.is_empty() {
        return Err(WorkVcsError::RelationInvalid(format!(
            "structural reference relation {relation_id} has non-empty discriminator {relation_discriminator:?}"
        )));
    }
    if state_schema_version != RELATION_STATE_SCHEMA_VERSION {
        return Err(WorkVcsError::RelationInvalid(format!(
            "structural reference relation {relation_id} version {relation_version_id} has state schema version {state_schema_version}"
        )));
    }

    let state_digest = decode_digest("relation_version.state_digest", state_digest)?;
    validate_import_fixed_point(
        metadata_json.as_bytes(),
        state_digest,
        ImportDigestDomain::RelationVersion,
    )
    .map_err(|error| {
        WorkVcsError::RelationInvalid(format!(
            "structural reference relation {relation_id} version {relation_version_id} fixed-point validation failed: {error}"
        ))
    })?;
    let metadata_value = crate::canonical::parse_canonical_json(metadata_json.as_bytes())
        .map_err(relation_invalid_from)?;
    if metadata_value != CanonicalValue::object(Vec::new())? {
        return Err(WorkVcsError::RelationInvalid(format!(
            "structural reference relation {relation_id} version {relation_version_id} state must be canonical empty object"
        )));
    }
    let actual = relation_version_digest(&metadata_value).map_err(relation_invalid_from)?;
    if actual != state_digest {
        return Err(WorkVcsError::RelationInvalid(format!(
            "structural reference relation {relation_id} version {relation_version_id} digest does not match metadata JSON"
        )));
    }

    let referrer_entity_id = decode_entity_id("relation.source_object_id", referrer_entity_id)?;
    let target_entity_id = decode_entity_id("relation.target_object_id", target_entity_id)?;
    let referrer = load_reference_endpoint(connection, commit_id, referrer_entity_id)?;
    let target = load_reference_endpoint(connection, commit_id, target_entity_id)?;
    if referrer.workspace_id != workspace_id || target.workspace_id != workspace_id {
        return Err(WorkVcsError::RelationInvalid(format!(
            "structural reference relation {relation_id} endpoints must belong to workspace {workspace_id}"
        )));
    }
    validate_reference_kind_pair(
        referrer.kind,
        target.kind,
        referrer_entity_id,
        target_entity_id,
    )?;

    Ok(Some(StructuralReferenceSnapshot {
        workspace_id,
        commit_id,
        relation_id,
        relation_version_id,
        referrer_entity_id,
        referrer_kind: referrer.kind,
        target_entity_id,
        target_kind: target.kind,
        state_digest,
    }))
}

fn load_reference_endpoint(
    connection: &StoreConnection,
    commit_id: CommitId,
    entity_id: EntityId,
) -> Result<ReferenceEndpoint> {
    let Some((workspace_id, entity_kind)) = load_current_entity_kind(connection, entity_id)? else {
        return Err(WorkVcsError::RelationInvalid(format!(
            "structural reference endpoint entity {entity_id} does not exist"
        )));
    };
    let Some(kind) = StructuralReferenceEndpointKind::parse(&entity_kind) else {
        return Err(WorkVcsError::RelationInvalid(format!(
            "structural reference endpoint {entity_id} has unsupported entity kind {entity_kind:?}"
        )));
    };

    match kind {
        StructuralReferenceEndpointKind::Goal => goal_at(connection, commit_id, entity_id)
            .map(|snapshot| ReferenceEndpoint {
                workspace_id: snapshot.workspace_id,
                kind,
            })
            .map_err(relation_invalid_from),
        StructuralReferenceEndpointKind::Plan => plan_at(connection, commit_id, entity_id)
            .map(|snapshot| ReferenceEndpoint {
                workspace_id: snapshot.workspace_id,
                kind,
            })
            .map_err(relation_invalid_from),
        StructuralReferenceEndpointKind::Task => task_at(connection, commit_id, entity_id)
            .map(|snapshot| ReferenceEndpoint {
                workspace_id: snapshot.workspace_id,
                kind,
            })
            .map_err(relation_invalid_from),
    }
    .and_then(|endpoint| {
        if endpoint.workspace_id == workspace_id {
            Ok(endpoint)
        } else {
            Err(WorkVcsError::RelationInvalid(format!(
                "structural reference endpoint {entity_id} workspace drifted from {workspace_id} to {}",
                endpoint.workspace_id
            )))
        }
    })
}

fn load_current_entity_kind(
    connection: &StoreConnection,
    entity_id: EntityId,
) -> Result<Option<(WorkspaceId, String)>> {
    let row = connection
        .inner()
        .query_row(
            "SELECT workspace_id, entity_kind
             FROM entity
             WHERE object_id = ?1",
            params![&entity_id.raw_bytes()[..]],
            |row| Ok((row.get::<_, Vec<u8>>(0)?, row.get::<_, String>(1)?)),
        )
        .optional()
        .map_err(storage_error)?;
    row.map(|(workspace_id, entity_kind)| {
        Ok((
            decode_workspace_id("entity.workspace_id", workspace_id)?,
            entity_kind,
        ))
    })
    .transpose()
}

fn validate_reference_kind_pair(
    referrer_kind: StructuralReferenceEndpointKind,
    target_kind: StructuralReferenceEndpointKind,
    referrer_entity_id: EntityId,
    target_entity_id: EntityId,
) -> Result<()> {
    match (referrer_kind, target_kind) {
        (StructuralReferenceEndpointKind::Goal, StructuralReferenceEndpointKind::Plan)
        | (StructuralReferenceEndpointKind::Goal, StructuralReferenceEndpointKind::Task)
        | (StructuralReferenceEndpointKind::Plan, StructuralReferenceEndpointKind::Plan)
        | (StructuralReferenceEndpointKind::Plan, StructuralReferenceEndpointKind::Task) => Ok(()),
        (StructuralReferenceEndpointKind::Goal, StructuralReferenceEndpointKind::Goal) => {
            Err(WorkVcsError::RelationInvalid(format!(
                "goal entity {referrer_entity_id} cannot structurally reference goal entity {target_entity_id} in Phase 3R"
            )))
        }
        (StructuralReferenceEndpointKind::Plan, StructuralReferenceEndpointKind::Goal) => {
            Err(WorkVcsError::RelationInvalid(format!(
                "plan entity {referrer_entity_id} cannot structurally reference goal entity {target_entity_id} in Phase 3R"
            )))
        }
        (StructuralReferenceEndpointKind::Task, _) => Err(WorkVcsError::RelationInvalid(format!(
            "task entity {referrer_entity_id} cannot be a structural reference referrer in Phase 3R"
        ))),
    }
}

fn ensure_structural_reference_logical_key_available(
    transaction: &Transaction<'_>,
    workspace_id: WorkspaceId,
    referrer_entity_id: EntityId,
    target_entity_id: EntityId,
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
                REFERENCES_RELATION_TYPE,
                &referrer_entity_id.raw_bytes()[..],
                &target_entity_id.raw_bytes()[..],
            ],
            |row| row.get::<_, i64>(0),
        )
        .map_err(storage_error)?;
    if existing == 0 {
        Ok(())
    } else {
        Err(WorkVcsError::RelationInvalid(format!(
            "structural reference from {referrer_entity_id} to {target_entity_id} already exists in workspace {workspace_id}"
        )))
    }
}

fn work_state_after_structural_reference_create(
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
        return Err(WorkVcsError::RelationInvalid(format!(
            "structural reference relation {relation_id} was expected to be absent before creation"
        )));
    }

    WorkState::new(entities, relations).map_err(relation_invalid_from)
}

fn write_structural_reference_create(
    transaction: &Transaction<'_>,
    rows: &StructuralReferenceCreateRows,
) -> Result<()> {
    let workspace_id_bytes = rows.workspace_id.raw_bytes();
    let relation_id_bytes = rows.relation_id.raw_bytes();
    let relation_version_id_bytes = rows.relation_version_id.raw_bytes();
    let relation_state_digest_bytes = rows.relation_state_digest.as_bytes();
    let referrer_entity_id_bytes = rows.referrer_entity_id.raw_bytes();
    let target_entity_id_bytes = rows.target_entity_id.raw_bytes();
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
                REFERENCES_RELATION_TYPE,
                &referrer_entity_id_bytes[..],
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
                STRUCTURAL_REFERENCE_CREATE_OPERATION_TYPE,
                STRUCTURAL_REFERENCE_CREATE_OPERATION_SCHEMA_VERSION,
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
                STRUCTURAL_REFERENCE_CREATE_EVENT_KIND,
                rows.now_us,
                rows.relation_payload_json
            ],
        )
        .map_err(storage_error)?;

    Ok(())
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
        return Err(WorkVcsError::RelationInvalid(format!(
            "branch {branch_id} has lifecycle state {lifecycle_state:?}"
        )));
    }

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
    .map_err(relation_invalid_from)
}

fn relation_invalid_from(error: WorkVcsError) -> WorkVcsError {
    WorkVcsError::RelationInvalid(error.to_string())
}

fn decode_workspace_id(column: &str, bytes: Vec<u8>) -> Result<WorkspaceId> {
    let bytes = decode_16(column, bytes)?;
    WorkspaceId::from_bytes(bytes).map_err(|error| {
        WorkVcsError::RelationInvalid(format!("{column} is not a UUIDv7 value: {error}"))
    })
}

fn decode_commit_id(column: &str, bytes: Vec<u8>) -> Result<CommitId> {
    let bytes = decode_16(column, bytes)?;
    CommitId::from_bytes(bytes).map_err(|error| {
        WorkVcsError::RelationInvalid(format!("{column} is not a UUIDv7 value: {error}"))
    })
}

fn decode_entity_id(column: &str, bytes: Vec<u8>) -> Result<EntityId> {
    let bytes = decode_16(column, bytes)?;
    EntityId::from_bytes(bytes).map_err(|error| {
        WorkVcsError::RelationInvalid(format!("{column} is not a UUIDv7 value: {error}"))
    })
}

fn decode_digest(column: &str, bytes: Vec<u8>) -> Result<Digest> {
    bytes
        .try_into()
        .map(Digest::from_bytes)
        .map_err(|bytes: Vec<u8>| {
            WorkVcsError::RelationInvalid(format!(
                "{column} must be 32 bytes, found {}",
                bytes.len()
            ))
        })
}

fn decode_16(column: &str, bytes: Vec<u8>) -> Result<[u8; 16]> {
    bytes.try_into().map_err(|bytes: Vec<u8>| {
        WorkVcsError::RelationInvalid(format!("{column} must be 16 bytes, found {}", bytes.len()))
    })
}
