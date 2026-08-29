use super::containment::PRIMARY_CONTAINMENT_CREATE_OPERATION_TYPE;
use super::entity::{
    ENTITY_TRANSITION_OPERATION_SCHEMA_VERSION, ENTITY_TRANSITION_OPERATION_TYPE,
    canonical_json_string, entity_transition_payload_json,
};
use super::reference::STRUCTURAL_REFERENCE_CREATE_OPERATION_TYPE;
use crate::canonical::{
    CanonicalValue, ImportDigestDomain, WorkState, canonical_bytes, entity_version_digest,
    parse_canonical_json, relation_version_digest, validate_import_fixed_point,
    work_state_mapping_digest,
};
use crate::error::{Result, WorkVcsError, storage_error};
use crate::identity::{
    ChangeSetId, CommitId, Digest, EntityId, EntityVersionId, OperationId, RelationId,
    RelationVersionId, WorkspaceId,
};
use crate::store::StoreConnection;
use rusqlite::{OptionalExtension, Params, params};
use std::collections::{BTreeMap, HashSet};

const GENESIS_OPERATION_SCHEMA_VERSION: i64 = 1;
const GENESIS_OPERATION_TYPE: &str = "workspace.genesis";
const GENESIS_COMMIT_KIND: &str = "genesis";
const NORMAL_COMMIT_KIND: &str = "normal";
const MERGE_COMMIT_KIND: &str = "merge";
const PRIMARY_CONTAINMENT_CREATE_OPERATION_SCHEMA_VERSION: i64 = 1;
const RELATION_OBJECT_KIND: &str = "relation";
const RELATION_STATE_SCHEMA_VERSION: i64 = 1;
const STRUCTURAL_REFERENCE_CREATE_OPERATION_SCHEMA_VERSION: i64 = 1;
const TASK_SCHEDULING_RELATION_CREATE_OPERATION_SCHEMA_VERSION: i64 = 1;
const TASK_SCHEDULING_RELATION_CREATE_OPERATION_TYPE: &str = "task.scheduling_relation.create";
const VERIFICATION_RECORD_OPERATION_SCHEMA_VERSION: i64 = 1;
const VERIFICATION_RECORD_OPERATION_TYPE: &str = "verification.record";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReplayedState {
    pub workspace_id: WorkspaceId,
    pub commit_id: CommitId,
    pub state: WorkState,
    pub state_digest: Digest,
}

pub(crate) fn state_at(connection: &StoreConnection, commit_id: CommitId) -> Result<ReplayedState> {
    state_at_inner(connection, commit_id, &mut HashSet::new())
}

fn state_at_inner(
    connection: &StoreConnection,
    commit_id: CommitId,
    visiting: &mut HashSet<[u8; 16]>,
) -> Result<ReplayedState> {
    if !visiting.insert(commit_id.raw_bytes()) {
        return Err(WorkVcsError::ReplayInvalid(format!(
            "cycle detected while replaying commit {commit_id}"
        )));
    }
    let commit = load_commit(connection, commit_id)?;
    let replayed = match commit.commit_kind.as_str() {
        GENESIS_COMMIT_KIND => replay_genesis(connection, commit_id, commit),
        NORMAL_COMMIT_KIND => replay_normal(connection, commit_id, commit, visiting),
        MERGE_COMMIT_KIND => Err(WorkVcsError::ReplayUnsupported(format!(
            "{:?} WorkStateCommit replay is deferred until ChangeOperation replay is implemented",
            commit.commit_kind
        ))),
        other => Err(WorkVcsError::ReplayInvalid(format!(
            "unsupported WorkStateCommit kind {other:?}"
        ))),
    };
    visiting.remove(&commit_id.raw_bytes());
    replayed
}

struct CommitRow {
    workspace_id: WorkspaceId,
    changeset_id: ChangeSetId,
    commit_kind: String,
    state_digest: Digest,
}

fn load_commit(connection: &StoreConnection, commit_id: CommitId) -> Result<CommitRow> {
    let commit_id_bytes = commit_id.raw_bytes();
    let row = connection
        .inner()
        .query_row(
            "SELECT workspace_id, changeset_id, commit_kind, state_digest
             FROM workstate_commit
             WHERE commit_id = ?1",
            params![&commit_id_bytes[..]],
            |row| {
                Ok((
                    row.get::<_, Vec<u8>>(0)?,
                    row.get::<_, Vec<u8>>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, Vec<u8>>(3)?,
                ))
            },
        )
        .optional()
        .map_err(storage_error)?;

    let Some((workspace_id, changeset_id, commit_kind, state_digest)) = row else {
        return Err(WorkVcsError::CommitNotFound(format!(
            "commit {commit_id} does not exist"
        )));
    };

    Ok(CommitRow {
        workspace_id: decode_workspace_id("workstate_commit.workspace_id", workspace_id)?,
        changeset_id: decode_changeset_id("workstate_commit.changeset_id", changeset_id)?,
        commit_kind,
        state_digest: decode_digest("workstate_commit.state_digest", state_digest)?,
    })
}

fn replay_genesis(
    connection: &StoreConnection,
    commit_id: CommitId,
    commit: CommitRow,
) -> Result<ReplayedState> {
    validate_workspace_genesis_link(connection, commit.workspace_id, commit_id)?;
    validate_genesis_changeset(connection, commit.workspace_id, commit.changeset_id)?;
    require_count(
        connection,
        "Genesis commit parents",
        0,
        "SELECT count(*)
         FROM commit_parent
         WHERE commit_id = ?1",
        params![&commit_id.raw_bytes()[..]],
    )?;
    require_count(
        connection,
        "Genesis ChangeOperations",
        0,
        "SELECT count(*)
         FROM change_operation
         WHERE changeset_id = ?1",
        params![&commit.changeset_id.raw_bytes()[..]],
    )?;

    let state = WorkState::empty();
    let actual_digest = work_state_mapping_digest(&state);
    if commit.state_digest != actual_digest {
        return Err(WorkVcsError::ReplayInvalid(format!(
            "Genesis commit {commit_id} state digest does not match replayed empty WorkState"
        )));
    }

    Ok(ReplayedState {
        workspace_id: commit.workspace_id,
        commit_id,
        state,
        state_digest: actual_digest,
    })
}

fn validate_workspace_genesis_link(
    connection: &StoreConnection,
    workspace_id: WorkspaceId,
    commit_id: CommitId,
) -> Result<()> {
    require_count(
        connection,
        "Workspace Genesis link",
        1,
        "SELECT count(*)
         FROM workspace
         WHERE workspace_id = ?1
           AND genesis_commit_id = ?2",
        params![&workspace_id.raw_bytes()[..], &commit_id.raw_bytes()[..]],
    )
}

fn validate_genesis_changeset(
    connection: &StoreConnection,
    workspace_id: WorkspaceId,
    changeset_id: ChangeSetId,
) -> Result<()> {
    let row = connection
        .inner()
        .query_row(
            "SELECT operation_type, operation_schema_version, operation_payload_json, rationale_json
             FROM changeset
             WHERE workspace_id = ?1
               AND changeset_id = ?2",
            params![&workspace_id.raw_bytes()[..], &changeset_id.raw_bytes()[..]],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                ))
            },
        )
        .optional()
        .map_err(storage_error)?;

    let Some((operation_type, operation_schema_version, operation_payload_json, rationale_json)) =
        row
    else {
        return Err(WorkVcsError::ReplayInvalid(format!(
            "Genesis ChangeSet {changeset_id} does not exist"
        )));
    };
    if operation_type != GENESIS_OPERATION_TYPE {
        return Err(WorkVcsError::ReplayInvalid(format!(
            "Genesis ChangeSet {changeset_id} has operation type {operation_type:?}"
        )));
    }
    if operation_schema_version != GENESIS_OPERATION_SCHEMA_VERSION {
        return Err(WorkVcsError::ReplayInvalid(format!(
            "Genesis ChangeSet {changeset_id} has operation schema version {operation_schema_version}"
        )));
    }

    let empty_object_json = canonical_empty_object_json()?;
    if operation_payload_json != empty_object_json {
        return Err(WorkVcsError::ReplayInvalid(format!(
            "Genesis ChangeSet {changeset_id} operation payload is not canonical empty JSON"
        )));
    }
    if rationale_json != empty_object_json {
        return Err(WorkVcsError::ReplayInvalid(format!(
            "Genesis ChangeSet {changeset_id} rationale is not canonical empty JSON"
        )));
    }
    Ok(())
}

fn replay_normal(
    connection: &StoreConnection,
    commit_id: CommitId,
    commit: CommitRow,
    visiting: &mut HashSet<[u8; 16]>,
) -> Result<ReplayedState> {
    let parent_commit_id = load_normal_primary_parent(connection, commit_id)?;
    let parent = state_at_inner(connection, parent_commit_id, visiting)?;
    if parent.workspace_id != commit.workspace_id {
        return Err(WorkVcsError::ReplayInvalid(format!(
            "normal commit {commit_id} belongs to workspace {}, but parent {} belongs to workspace {}",
            commit.workspace_id, parent_commit_id, parent.workspace_id
        )));
    }

    let state = apply_entity_transition_changeset(
        connection,
        commit.workspace_id,
        commit.changeset_id,
        parent.state,
    )?;
    let actual_digest = work_state_mapping_digest(&state);
    if commit.state_digest != actual_digest {
        return Err(WorkVcsError::ReplayInvalid(format!(
            "normal commit {commit_id} state digest does not match replayed WorkState"
        )));
    }

    Ok(ReplayedState {
        workspace_id: commit.workspace_id,
        commit_id,
        state,
        state_digest: actual_digest,
    })
}

fn load_normal_primary_parent(
    connection: &StoreConnection,
    commit_id: CommitId,
) -> Result<CommitId> {
    require_count(
        connection,
        "Normal commit parents",
        1,
        "SELECT count(*)
         FROM commit_parent
         WHERE commit_id = ?1",
        params![&commit_id.raw_bytes()[..]],
    )?;

    let parent = connection
        .inner()
        .query_row(
            "SELECT parent_commit_id
             FROM commit_parent
             WHERE commit_id = ?1
               AND parent_ordinal = 0
               AND parent_role = 'primary'",
            params![&commit_id.raw_bytes()[..]],
            |row| row.get::<_, Vec<u8>>(0),
        )
        .optional()
        .map_err(storage_error)?;
    let Some(parent) = parent else {
        return Err(WorkVcsError::ReplayInvalid(format!(
            "normal commit {commit_id} does not have ordinal-0 primary parent"
        )));
    };
    decode_commit_id("commit_parent.parent_commit_id", parent)
}

fn apply_entity_transition_changeset(
    connection: &StoreConnection,
    workspace_id: WorkspaceId,
    changeset_id: ChangeSetId,
    parent_state: WorkState,
) -> Result<WorkState> {
    let changeset = validate_entity_transition_changeset(connection, workspace_id, changeset_id)?;
    let operations = load_change_operations(connection, changeset_id)?;
    if operations.is_empty() {
        return Err(WorkVcsError::ReplayInvalid(format!(
            "normal ChangeSet {changeset_id} has no ChangeOperations"
        )));
    }
    if operations.len() == 1
        && changeset.operation_payload_json != operations[0].operation_payload_json
    {
        return Err(WorkVcsError::ReplayInvalid(format!(
            "normal ChangeSet {changeset_id} payload does not match its single ChangeOperation"
        )));
    }

    let mut entities = parent_state
        .entities()
        .iter()
        .copied()
        .collect::<BTreeMap<_, _>>();
    let mut relations = parent_state
        .relations()
        .iter()
        .copied()
        .collect::<BTreeMap<_, _>>();
    for (expected_ordinal, operation) in operations.into_iter().enumerate() {
        if operation.ordinal != expected_ordinal as i64 {
            return Err(WorkVcsError::ReplayInvalid(format!(
                "ChangeOperation {} has non-contiguous ordinal {}",
                operation.operation_id, operation.ordinal
            )));
        }
        let subject_family = operation.subject_family.clone();
        match subject_family.as_str() {
            "entity" => {
                let subject_object_id = decode_entity_id(
                    "change_operation.subject_object_id",
                    operation.subject_object_id.clone(),
                )?;
                apply_entity_operation(
                    connection,
                    workspace_id,
                    &mut entities,
                    operation,
                    subject_object_id,
                )?;
            }
            "relation" => {
                let subject_object_id = decode_relation_id(
                    "change_operation.subject_object_id",
                    operation.subject_object_id.clone(),
                )?;
                apply_relation_operation(
                    connection,
                    workspace_id,
                    &mut relations,
                    operation,
                    subject_object_id,
                )?;
            }
            _ => {
                return Err(WorkVcsError::ReplayUnsupported(format!(
                    "ChangeOperation {} subject family {:?} is deferred",
                    operation.operation_id, operation.subject_family
                )));
            }
        }
    }

    WorkState::new(entities, relations).map_err(|error| {
        WorkVcsError::ReplayInvalid(format!("replayed WorkState is invalid: {error}"))
    })
}

fn validate_entity_transition_changeset(
    connection: &StoreConnection,
    workspace_id: WorkspaceId,
    changeset_id: ChangeSetId,
) -> Result<ChangeSetRow> {
    let row = connection
        .inner()
        .query_row(
            "SELECT operation_type, operation_schema_version, operation_payload_json, rationale_json
             FROM changeset
             WHERE workspace_id = ?1
               AND changeset_id = ?2",
            params![&workspace_id.raw_bytes()[..], &changeset_id.raw_bytes()[..]],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                ))
            },
        )
        .optional()
        .map_err(storage_error)?;

    let Some((operation_type, operation_schema_version, operation_payload_json, rationale_json)) =
        row
    else {
        return Err(WorkVcsError::ReplayInvalid(format!(
            "normal ChangeSet {changeset_id} does not exist"
        )));
    };
    match operation_type.as_str() {
        ENTITY_TRANSITION_OPERATION_TYPE => {
            if operation_schema_version != ENTITY_TRANSITION_OPERATION_SCHEMA_VERSION {
                return Err(WorkVcsError::ReplayUnsupported(format!(
                    "normal ChangeSet {changeset_id} operation schema version {operation_schema_version} is deferred"
                )));
            }
        }
        VERIFICATION_RECORD_OPERATION_TYPE => {
            if operation_schema_version != VERIFICATION_RECORD_OPERATION_SCHEMA_VERSION {
                return Err(WorkVcsError::ReplayUnsupported(format!(
                    "normal ChangeSet {changeset_id} operation schema version {operation_schema_version} is deferred"
                )));
            }
        }
        TASK_SCHEDULING_RELATION_CREATE_OPERATION_TYPE => {
            if operation_schema_version != TASK_SCHEDULING_RELATION_CREATE_OPERATION_SCHEMA_VERSION
            {
                return Err(WorkVcsError::ReplayUnsupported(format!(
                    "normal ChangeSet {changeset_id} operation schema version {operation_schema_version} is deferred"
                )));
            }
        }
        PRIMARY_CONTAINMENT_CREATE_OPERATION_TYPE => {
            if operation_schema_version != PRIMARY_CONTAINMENT_CREATE_OPERATION_SCHEMA_VERSION {
                return Err(WorkVcsError::ReplayUnsupported(format!(
                    "normal ChangeSet {changeset_id} operation schema version {operation_schema_version} is deferred"
                )));
            }
        }
        STRUCTURAL_REFERENCE_CREATE_OPERATION_TYPE => {
            if operation_schema_version != STRUCTURAL_REFERENCE_CREATE_OPERATION_SCHEMA_VERSION {
                return Err(WorkVcsError::ReplayUnsupported(format!(
                    "normal ChangeSet {changeset_id} operation schema version {operation_schema_version} is deferred"
                )));
            }
        }
        _ => {
            return Err(WorkVcsError::ReplayUnsupported(format!(
                "normal ChangeSet {changeset_id} operation type {operation_type:?} is deferred"
            )));
        }
    }
    validate_canonical_json_text("changeset.operation_payload_json", &operation_payload_json)?;
    validate_canonical_json_text("changeset.rationale_json", &rationale_json)?;
    Ok(ChangeSetRow {
        operation_payload_json,
    })
}

struct ChangeSetRow {
    operation_payload_json: String,
}

struct OperationRow {
    operation_id: OperationId,
    ordinal: i64,
    subject_family: String,
    subject_object_id: Vec<u8>,
    operation_payload_json: String,
}

fn load_change_operations(
    connection: &StoreConnection,
    changeset_id: ChangeSetId,
) -> Result<Vec<OperationRow>> {
    let mut statement = connection
        .inner()
        .prepare(
            "SELECT operation_id, ordinal, subject_family, subject_object_id, operation_payload_json
             FROM change_operation
             WHERE changeset_id = ?1
             ORDER BY ordinal",
        )
        .map_err(storage_error)?;
    let rows = statement
        .query_map(params![&changeset_id.raw_bytes()[..]], |row| {
            Ok((
                row.get::<_, Vec<u8>>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, Vec<u8>>(3)?,
                row.get::<_, String>(4)?,
            ))
        })
        .map_err(storage_error)?;

    let mut operations = Vec::new();
    for row in rows {
        let (operation_id, ordinal, subject_family, subject_object_id, operation_payload_json) =
            row.map_err(storage_error)?;
        operations.push(OperationRow {
            operation_id: decode_operation_id("change_operation.operation_id", operation_id)?,
            ordinal,
            subject_family,
            subject_object_id,
            operation_payload_json,
        });
    }
    Ok(operations)
}

fn apply_entity_operation(
    connection: &StoreConnection,
    workspace_id: WorkspaceId,
    entities: &mut BTreeMap<EntityId, EntityVersionId>,
    operation: OperationRow,
    subject_entity_id: EntityId,
) -> Result<()> {
    let change = load_entity_membership_change(connection, operation.operation_id)?;
    if change.entity_id != subject_entity_id {
        return Err(WorkVcsError::ReplayInvalid(format!(
            "Entity membership change {} targets {}, but ChangeOperation targets {}",
            operation.operation_id, change.entity_id, subject_entity_id
        )));
    }

    validate_canonical_json_text(
        "change_operation.operation_payload_json",
        &operation.operation_payload_json,
    )?;
    validate_canonical_json_text(
        "entity_membership_change.field_delta_json",
        &change.field_delta_json,
    )?;

    let expected_payload = entity_transition_payload_json(
        subject_entity_id,
        change.before_entity_version_id,
        change.after_entity_version_id.ok_or_else(|| {
            WorkVcsError::ReplayUnsupported(format!(
                "entity removal for {subject_entity_id} is deferred"
            ))
        })?,
    )?;
    if operation.operation_payload_json != expected_payload {
        return Err(WorkVcsError::ReplayInvalid(format!(
            "ChangeOperation {} payload does not match entity membership change",
            operation.operation_id
        )));
    }

    let current = entities.get(&subject_entity_id).copied();
    if current != change.before_entity_version_id {
        return Err(WorkVcsError::ReplayInvalid(format!(
            "entity {subject_entity_id} expected replay version {:?}, found {:?}",
            change.before_entity_version_id, current
        )));
    }

    let after_entity_version_id = change.after_entity_version_id.expect("checked above");
    validate_entity_version(
        connection,
        workspace_id,
        subject_entity_id,
        after_entity_version_id,
    )?;
    entities.insert(subject_entity_id, after_entity_version_id);
    Ok(())
}

fn apply_relation_operation(
    connection: &StoreConnection,
    workspace_id: WorkspaceId,
    relations: &mut BTreeMap<RelationId, RelationVersionId>,
    operation: OperationRow,
    subject_relation_id: RelationId,
) -> Result<()> {
    let change = load_relation_membership_change(connection, operation.operation_id)?;
    if change.relation_id != subject_relation_id {
        return Err(WorkVcsError::ReplayInvalid(format!(
            "Relation membership change {} targets {}, but ChangeOperation targets {}",
            operation.operation_id, change.relation_id, subject_relation_id
        )));
    }

    validate_canonical_json_text(
        "change_operation.operation_payload_json",
        &operation.operation_payload_json,
    )?;
    validate_canonical_json_text(
        "relation_membership_change.field_delta_json",
        &change.field_delta_json,
    )?;

    if change.before_relation_version_id.is_some() {
        return Err(WorkVcsError::ReplayUnsupported(format!(
            "relation update for {subject_relation_id} is deferred"
        )));
    }
    let after_relation_version_id = change.after_relation_version_id.ok_or_else(|| {
        WorkVcsError::ReplayUnsupported(format!(
            "relation removal for {subject_relation_id} is deferred"
        ))
    })?;
    let expected_payload =
        relation_transition_payload_json(subject_relation_id, None, after_relation_version_id)?;
    if operation.operation_payload_json != expected_payload {
        return Err(WorkVcsError::ReplayInvalid(format!(
            "ChangeOperation {} payload does not match relation membership change",
            operation.operation_id
        )));
    }

    let current = relations.get(&subject_relation_id).copied();
    if current.is_some() {
        return Err(WorkVcsError::ReplayInvalid(format!(
            "relation {subject_relation_id} was expected to be absent before creation, found {current:?}"
        )));
    }

    validate_relation_version(
        connection,
        workspace_id,
        subject_relation_id,
        after_relation_version_id,
    )?;
    relations.insert(subject_relation_id, after_relation_version_id);
    Ok(())
}

struct EntityMembershipChange {
    entity_id: EntityId,
    before_entity_version_id: Option<EntityVersionId>,
    after_entity_version_id: Option<EntityVersionId>,
    field_delta_json: String,
}

struct RelationMembershipChange {
    relation_id: RelationId,
    before_relation_version_id: Option<RelationVersionId>,
    after_relation_version_id: Option<RelationVersionId>,
    field_delta_json: String,
}

fn load_entity_membership_change(
    connection: &StoreConnection,
    operation_id: OperationId,
) -> Result<EntityMembershipChange> {
    let row = connection
        .inner()
        .query_row(
            "SELECT entity_id, before_entity_version_id, after_entity_version_id, field_delta_json
             FROM entity_membership_change
             WHERE operation_id = ?1",
            params![&operation_id.raw_bytes()[..]],
            |row| {
                Ok((
                    row.get::<_, Vec<u8>>(0)?,
                    row.get::<_, Option<Vec<u8>>>(1)?,
                    row.get::<_, Option<Vec<u8>>>(2)?,
                    row.get::<_, String>(3)?,
                ))
            },
        )
        .optional()
        .map_err(storage_error)?;

    let Some((entity_id, before_entity_version_id, after_entity_version_id, field_delta_json)) =
        row
    else {
        return Err(WorkVcsError::ReplayInvalid(format!(
            "ChangeOperation {operation_id} has no entity membership change"
        )));
    };
    Ok(EntityMembershipChange {
        entity_id: decode_entity_id("entity_membership_change.entity_id", entity_id)?,
        before_entity_version_id: decode_optional_entity_version_id(
            "entity_membership_change.before_entity_version_id",
            before_entity_version_id,
        )?,
        after_entity_version_id: decode_optional_entity_version_id(
            "entity_membership_change.after_entity_version_id",
            after_entity_version_id,
        )?,
        field_delta_json,
    })
}

fn load_relation_membership_change(
    connection: &StoreConnection,
    operation_id: OperationId,
) -> Result<RelationMembershipChange> {
    let row = connection
        .inner()
        .query_row(
            "SELECT relation_id, before_relation_version_id, after_relation_version_id, field_delta_json
             FROM relation_membership_change
             WHERE operation_id = ?1",
            params![&operation_id.raw_bytes()[..]],
            |row| {
                Ok((
                    row.get::<_, Vec<u8>>(0)?,
                    row.get::<_, Option<Vec<u8>>>(1)?,
                    row.get::<_, Option<Vec<u8>>>(2)?,
                    row.get::<_, String>(3)?,
                ))
            },
        )
        .optional()
        .map_err(storage_error)?;

    let Some((
        relation_id,
        before_relation_version_id,
        after_relation_version_id,
        field_delta_json,
    )) = row
    else {
        return Err(WorkVcsError::ReplayInvalid(format!(
            "ChangeOperation {operation_id} has no relation membership change"
        )));
    };
    Ok(RelationMembershipChange {
        relation_id: decode_relation_id("relation_membership_change.relation_id", relation_id)?,
        before_relation_version_id: decode_optional_relation_version_id(
            "relation_membership_change.before_relation_version_id",
            before_relation_version_id,
        )?,
        after_relation_version_id: decode_optional_relation_version_id(
            "relation_membership_change.after_relation_version_id",
            after_relation_version_id,
        )?,
        field_delta_json,
    })
}

fn validate_entity_version(
    connection: &StoreConnection,
    workspace_id: WorkspaceId,
    entity_id: EntityId,
    entity_version_id: EntityVersionId,
) -> Result<()> {
    let row = connection
        .inner()
        .query_row(
            "SELECT entity.workspace_id, entity_version.state_json, entity_version.state_digest
             FROM entity_version
             JOIN entity ON entity.object_id = entity_version.entity_id
             WHERE entity_version.entity_id = ?1
               AND entity_version.entity_version_id = ?2",
            params![
                &entity_id.raw_bytes()[..],
                &entity_version_id.raw_bytes()[..]
            ],
            |row| {
                Ok((
                    row.get::<_, Vec<u8>>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Vec<u8>>(2)?,
                ))
            },
        )
        .optional()
        .map_err(storage_error)?;

    let Some((entity_workspace_id, state_json, state_digest)) = row else {
        return Err(WorkVcsError::ReplayInvalid(format!(
            "EntityVersion {entity_version_id} for entity {entity_id} does not exist"
        )));
    };
    let entity_workspace_id = decode_workspace_id("entity.workspace_id", entity_workspace_id)?;
    if entity_workspace_id != workspace_id {
        return Err(WorkVcsError::ReplayInvalid(format!(
            "entity {entity_id} belongs to workspace {entity_workspace_id}, not {workspace_id}"
        )));
    }

    let state_digest = decode_digest("entity_version.state_digest", state_digest)?;
    validate_import_fixed_point(
        state_json.as_bytes(),
        state_digest,
        ImportDigestDomain::EntityVersion,
    )
    .map_err(|error| {
        WorkVcsError::ReplayInvalid(format!(
            "EntityVersion {entity_version_id} fixed-point validation failed: {error}"
        ))
    })?;

    let value = parse_canonical_json(state_json.as_bytes()).map_err(|error| {
        WorkVcsError::ReplayInvalid(format!("EntityVersion {entity_version_id}: {error}"))
    })?;
    let actual = entity_version_digest(&value).map_err(|error| {
        WorkVcsError::ReplayInvalid(format!("EntityVersion {entity_version_id}: {error}"))
    })?;
    if actual != state_digest {
        return Err(WorkVcsError::ReplayInvalid(format!(
            "EntityVersion {entity_version_id} digest does not match state JSON"
        )));
    }
    Ok(())
}

fn validate_relation_version(
    connection: &StoreConnection,
    workspace_id: WorkspaceId,
    relation_id: RelationId,
    relation_version_id: RelationVersionId,
) -> Result<()> {
    let row = connection
        .inner()
        .query_row(
            "SELECT object_identity.object_kind,
                    relation.workspace_id,
                    relation_version.state_schema_version,
                    relation_version.metadata_json,
                    relation_version.state_digest
             FROM relation
             JOIN object_identity ON object_identity.object_id = relation.object_id
             JOIN relation_version ON relation_version.relation_id = relation.object_id
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
                    row.get::<_, i64>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, Vec<u8>>(4)?,
                ))
            },
        )
        .optional()
        .map_err(storage_error)?;

    let Some((
        object_kind,
        relation_workspace_id,
        state_schema_version,
        metadata_json,
        state_digest,
    )) = row
    else {
        return Err(WorkVcsError::ReplayInvalid(format!(
            "RelationVersion {relation_version_id} for relation {relation_id} does not exist"
        )));
    };
    if object_kind != RELATION_OBJECT_KIND {
        return Err(WorkVcsError::ReplayInvalid(format!(
            "relation {relation_id} has object kind {object_kind:?}"
        )));
    }
    let relation_workspace_id =
        decode_workspace_id("relation.workspace_id", relation_workspace_id)?;
    if relation_workspace_id != workspace_id {
        return Err(WorkVcsError::ReplayInvalid(format!(
            "relation {relation_id} belongs to workspace {relation_workspace_id}, not {workspace_id}"
        )));
    }
    if state_schema_version != RELATION_STATE_SCHEMA_VERSION {
        return Err(WorkVcsError::ReplayUnsupported(format!(
            "RelationVersion {relation_version_id} state schema version {state_schema_version} is deferred"
        )));
    }

    let state_digest = decode_digest("relation_version.state_digest", state_digest)?;
    validate_import_fixed_point(
        metadata_json.as_bytes(),
        state_digest,
        ImportDigestDomain::RelationVersion,
    )
    .map_err(|error| {
        WorkVcsError::ReplayInvalid(format!(
            "RelationVersion {relation_version_id} fixed-point validation failed: {error}"
        ))
    })?;

    let value = parse_canonical_json(metadata_json.as_bytes()).map_err(|error| {
        WorkVcsError::ReplayInvalid(format!("RelationVersion {relation_version_id}: {error}"))
    })?;
    let actual = relation_version_digest(&value).map_err(|error| {
        WorkVcsError::ReplayInvalid(format!("RelationVersion {relation_version_id}: {error}"))
    })?;
    if actual != state_digest {
        return Err(WorkVcsError::ReplayInvalid(format!(
            "RelationVersion {relation_version_id} digest does not match metadata JSON"
        )));
    }
    Ok(())
}

fn validate_canonical_json_text(label: &str, value: &str) -> Result<CanonicalValue> {
    let parsed = parse_canonical_json(value.as_bytes())
        .map_err(|error| WorkVcsError::ReplayInvalid(format!("{label}: {error}")))?;
    let encoded = canonical_json_string(&parsed)
        .map_err(|error| WorkVcsError::ReplayInvalid(format!("{label}: {error}")))?;
    if encoded != value {
        return Err(WorkVcsError::ReplayInvalid(format!(
            "{label} is not canonical fixed-point JSON"
        )));
    }
    Ok(parsed)
}

fn canonical_empty_object_json() -> Result<String> {
    let value = CanonicalValue::object(Vec::new())?;
    String::from_utf8(canonical_bytes(&value)?).map_err(|error| {
        WorkVcsError::CanonicalEncodingInvalid(format!(
            "canonical empty object JSON was not UTF-8: {error}"
        ))
    })
}

fn relation_transition_payload_json(
    relation_id: RelationId,
    before_relation_version_id: Option<RelationVersionId>,
    after_relation_version_id: RelationVersionId,
) -> Result<String> {
    let before_value = match before_relation_version_id {
        Some(version_id) => CanonicalValue::String(version_id.to_string()),
        None => CanonicalValue::Null,
    };
    canonical_json_string(&CanonicalValue::object(vec![
        (
            "after_relation_version_id".to_owned(),
            CanonicalValue::String(after_relation_version_id.to_string()),
        ),
        ("before_relation_version_id".to_owned(), before_value),
        (
            "relation_id".to_owned(),
            CanonicalValue::String(relation_id.to_string()),
        ),
    ])?)
}

fn require_count<P: Params>(
    connection: &StoreConnection,
    label: &str,
    expected: i64,
    sql: &str,
    params: P,
) -> Result<()> {
    let actual = connection
        .inner()
        .query_row(sql, params, |row| row.get::<_, i64>(0))
        .map_err(storage_error)?;
    if actual == expected {
        Ok(())
    } else {
        Err(WorkVcsError::ReplayInvalid(format!(
            "{label} expected count {expected}, found {actual}"
        )))
    }
}

fn decode_workspace_id(column: &str, bytes: Vec<u8>) -> Result<WorkspaceId> {
    let bytes = decode_16(column, bytes)?;
    WorkspaceId::from_bytes(bytes).map_err(|error| {
        WorkVcsError::ReplayInvalid(format!("{column} is not a UUIDv7 value: {error}"))
    })
}

fn decode_changeset_id(column: &str, bytes: Vec<u8>) -> Result<ChangeSetId> {
    let bytes = decode_16(column, bytes)?;
    ChangeSetId::from_bytes(bytes).map_err(|error| {
        WorkVcsError::ReplayInvalid(format!("{column} is not a UUIDv7 value: {error}"))
    })
}

fn decode_commit_id(column: &str, bytes: Vec<u8>) -> Result<CommitId> {
    let bytes = decode_16(column, bytes)?;
    CommitId::from_bytes(bytes).map_err(|error| {
        WorkVcsError::ReplayInvalid(format!("{column} is not a UUIDv7 value: {error}"))
    })
}

fn decode_entity_id(column: &str, bytes: Vec<u8>) -> Result<EntityId> {
    let bytes = decode_16(column, bytes)?;
    EntityId::from_bytes(bytes).map_err(|error| {
        WorkVcsError::ReplayInvalid(format!("{column} is not a UUIDv7 value: {error}"))
    })
}

fn decode_relation_id(column: &str, bytes: Vec<u8>) -> Result<RelationId> {
    let bytes = decode_16(column, bytes)?;
    RelationId::from_bytes(bytes).map_err(|error| {
        WorkVcsError::ReplayInvalid(format!("{column} is not a UUIDv7 value: {error}"))
    })
}

fn decode_operation_id(column: &str, bytes: Vec<u8>) -> Result<OperationId> {
    let bytes = decode_16(column, bytes)?;
    OperationId::from_bytes(bytes).map_err(|error| {
        WorkVcsError::ReplayInvalid(format!("{column} is not a UUIDv7 value: {error}"))
    })
}

fn decode_optional_entity_version_id(
    column: &str,
    bytes: Option<Vec<u8>>,
) -> Result<Option<EntityVersionId>> {
    bytes
        .map(|value| decode_entity_version_id(column, value))
        .transpose()
}

fn decode_optional_relation_version_id(
    column: &str,
    bytes: Option<Vec<u8>>,
) -> Result<Option<RelationVersionId>> {
    bytes
        .map(|value| decode_relation_version_id(column, value))
        .transpose()
}

fn decode_entity_version_id(column: &str, bytes: Vec<u8>) -> Result<EntityVersionId> {
    let bytes = decode_16(column, bytes)?;
    EntityVersionId::from_bytes(bytes).map_err(|error| {
        WorkVcsError::ReplayInvalid(format!("{column} is not a UUIDv7 value: {error}"))
    })
}

fn decode_relation_version_id(column: &str, bytes: Vec<u8>) -> Result<RelationVersionId> {
    let bytes = decode_16(column, bytes)?;
    RelationVersionId::from_bytes(bytes).map_err(|error| {
        WorkVcsError::ReplayInvalid(format!("{column} is not a UUIDv7 value: {error}"))
    })
}

fn decode_16(column: &str, bytes: Vec<u8>) -> Result<[u8; 16]> {
    bytes.try_into().map_err(|bytes: Vec<u8>| {
        WorkVcsError::ReplayInvalid(format!("{column} must be 16 bytes, found {}", bytes.len()))
    })
}

fn decode_digest(column: &str, bytes: Vec<u8>) -> Result<Digest> {
    let bytes = bytes.try_into().map_err(|bytes: Vec<u8>| {
        WorkVcsError::ReplayInvalid(format!("{column} must be 32 bytes, found {}", bytes.len()))
    })?;
    Ok(Digest::from_bytes(bytes))
}
