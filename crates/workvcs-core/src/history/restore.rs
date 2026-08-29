use crate::canonical::{CanonicalValue, WorkState, canonical_bytes, work_state_mapping_digest};
use crate::error::{Result, WorkVcsError, storage_error};
use crate::identity::{
    BranchId, ChangeSetId, CommitId, Digest, EntityId, EntityVersionId, EventId, OperationId,
    RelationId, RelationVersionId, WorkspaceId,
};
use crate::store::{StoreConnection, current_epoch_micros};
use rusqlite::{OptionalExtension, Transaction, TransactionBehavior, params};
use std::collections::{BTreeMap, BTreeSet};

const ACTIVE_BRANCH_LIFECYCLE_STATE: &str = "active";
const EMPTY_FIELD_DELTA: &str = "{}";
const NORMAL_COMMIT_KIND: &str = "normal";
const PRIMARY_PARENT_ROLE: &str = "primary";
const RESTORE_EVENT_KIND: &str = "workstate.restored";
const RESTORE_OPERATION_SCHEMA_VERSION: i64 = 1;
const RESTORE_OPERATION_TYPE: &str = "workstate.restore";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkStateRestoreOptions {
    branch_id: BranchId,
    expected_head_commit_id: CommitId,
    target_commit_id: CommitId,
    rationale: CanonicalValue,
}

impl WorkStateRestoreOptions {
    pub fn new(
        branch_id: BranchId,
        expected_head_commit_id: CommitId,
        target_commit_id: CommitId,
    ) -> Result<Self> {
        Ok(Self {
            branch_id,
            expected_head_commit_id,
            target_commit_id,
            rationale: CanonicalValue::object(Vec::new())?,
        })
    }

    pub fn with_rationale(mut self, rationale: CanonicalValue) -> Result<Self> {
        require_object_value("restore rationale", &rationale)?;
        self.rationale = rationale;
        Ok(self)
    }

    pub fn branch_id(&self) -> BranchId {
        self.branch_id
    }

    pub fn expected_head_commit_id(&self) -> CommitId {
        self.expected_head_commit_id
    }

    pub fn target_commit_id(&self) -> CommitId {
        self.target_commit_id
    }

    pub fn rationale(&self) -> &CanonicalValue {
        &self.rationale
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkStateRestoreCommit {
    pub workspace_id: WorkspaceId,
    pub branch_id: BranchId,
    pub previous_head_commit_id: CommitId,
    pub target_commit_id: CommitId,
    pub commit_id: CommitId,
    pub changeset_id: ChangeSetId,
    pub operation_count: usize,
    pub work_state_digest: Digest,
}

pub(crate) fn restore_work_state(
    connection: &mut StoreConnection,
    options: &WorkStateRestoreOptions,
) -> Result<WorkStateRestoreCommit> {
    connection.verify_foreign_keys()?;

    let prepared = prepare_restore(connection, options)?;
    let now_us = current_epoch_micros()?;
    let changeset_id = ChangeSetId::new_v7();
    let commit_id = CommitId::new_v7();
    let payload_json = restore_payload_json(&RestorePayload {
        workspace_id: prepared.workspace_id,
        branch_id: options.branch_id(),
        previous_head_commit_id: options.expected_head_commit_id(),
        target_commit_id: options.target_commit_id(),
        commit_id,
        operation_count: prepared.operations.len(),
    })?;
    let rationale_json = canonical_json_string(options.rationale())?;
    let event_id = EventId::new_v7();

    let transaction = connection
        .inner_mut()
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(storage_error)?;
    let branch = load_active_branch(&transaction, options.branch_id())?;
    if branch.workspace_id != prepared.workspace_id {
        return Err(WorkVcsError::WorkspaceInvalid(format!(
            "branch {} belongs to workspace {}, but restore target {} belongs to workspace {}",
            options.branch_id(),
            branch.workspace_id,
            options.target_commit_id(),
            prepared.workspace_id
        )));
    }
    if branch.head_commit_id != options.expected_head_commit_id() {
        return Err(WorkVcsError::BranchHeadConflict(format!(
            "branch {} expected head {}, found {}",
            options.branch_id(),
            options.expected_head_commit_id(),
            branch.head_commit_id
        )));
    }

    write_restore(
        &transaction,
        options,
        &prepared,
        &RestoreWrite {
            now_us,
            changeset_id,
            commit_id,
            event_id,
            payload_json: &payload_json,
            rationale_json: &rationale_json,
        },
    )?;
    transaction.commit().map_err(storage_error)?;

    Ok(WorkStateRestoreCommit {
        workspace_id: prepared.workspace_id,
        branch_id: options.branch_id(),
        previous_head_commit_id: options.expected_head_commit_id(),
        target_commit_id: options.target_commit_id(),
        commit_id,
        changeset_id,
        operation_count: prepared.operations.len(),
        work_state_digest: prepared.work_state_digest,
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct BranchRow {
    workspace_id: WorkspaceId,
    head_commit_id: CommitId,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct PreparedRestore {
    workspace_id: WorkspaceId,
    work_state_digest: Digest,
    operations: Vec<PreparedRestoreOperation>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct PreparedRestoreOperation {
    operation_id: OperationId,
    ordinal: i64,
    subject: RestoreSubject,
    operation_payload_json: String,
    membership_change: RestoreMembershipChange,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RestoreSubject {
    Entity(EntityId),
    Relation(RelationId),
}

impl RestoreSubject {
    fn kind(self) -> &'static str {
        match self {
            Self::Entity(_) => "entity",
            Self::Relation(_) => "relation",
        }
    }

    fn object_id_bytes(self) -> [u8; 16] {
        match self {
            Self::Entity(entity_id) => entity_id.raw_bytes(),
            Self::Relation(relation_id) => relation_id.raw_bytes(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RestoreMembershipChange {
    Entity {
        entity_id: EntityId,
        before_entity_version_id: Option<EntityVersionId>,
        after_entity_version_id: Option<EntityVersionId>,
    },
    Relation {
        relation_id: RelationId,
        before_relation_version_id: Option<RelationVersionId>,
        after_relation_version_id: Option<RelationVersionId>,
    },
}

fn prepare_restore(
    connection: &StoreConnection,
    options: &WorkStateRestoreOptions,
) -> Result<PreparedRestore> {
    let current = super::state_at(connection, options.expected_head_commit_id())?;
    let target = super::state_at(connection, options.target_commit_id())?;
    if current.workspace_id != target.workspace_id {
        return Err(WorkVcsError::WorkspaceInvalid(format!(
            "restore current head {} belongs to workspace {}, but target {} belongs to workspace {}",
            options.expected_head_commit_id(),
            current.workspace_id,
            options.target_commit_id(),
            target.workspace_id
        )));
    }
    if current.state == target.state {
        return Err(WorkVcsError::WorkspaceInvalid(format!(
            "restore target {} already matches current head {}",
            options.target_commit_id(),
            options.expected_head_commit_id()
        )));
    }

    let mut operations = Vec::new();
    prepare_entity_restore_operations(&current.state, &target.state, &mut operations)?;
    prepare_relation_restore_operations(&current.state, &target.state, &mut operations)?;
    let work_state_digest = work_state_mapping_digest(&target.state);
    Ok(PreparedRestore {
        workspace_id: current.workspace_id,
        work_state_digest,
        operations,
    })
}

fn prepare_entity_restore_operations(
    current: &WorkState,
    target: &WorkState,
    operations: &mut Vec<PreparedRestoreOperation>,
) -> Result<()> {
    let current_entities = entity_map(current.entities());
    let target_entities = entity_map(target.entities());
    let mut entity_ids = BTreeSet::new();
    entity_ids.extend(current_entities.keys().copied());
    entity_ids.extend(target_entities.keys().copied());
    for entity_id in entity_ids {
        let before = current_entities.get(&entity_id).copied();
        let after = target_entities.get(&entity_id).copied();
        if before == after {
            continue;
        }
        operations.push(PreparedRestoreOperation {
            operation_id: OperationId::new_v7(),
            ordinal: operations.len() as i64,
            subject: RestoreSubject::Entity(entity_id),
            operation_payload_json: entity_membership_payload_json(entity_id, before, after)?,
            membership_change: RestoreMembershipChange::Entity {
                entity_id,
                before_entity_version_id: before,
                after_entity_version_id: after,
            },
        });
    }
    Ok(())
}

fn prepare_relation_restore_operations(
    current: &WorkState,
    target: &WorkState,
    operations: &mut Vec<PreparedRestoreOperation>,
) -> Result<()> {
    let current_relations = relation_map(current.relations());
    let target_relations = relation_map(target.relations());
    let mut relation_ids = BTreeSet::new();
    relation_ids.extend(current_relations.keys().copied());
    relation_ids.extend(target_relations.keys().copied());
    for relation_id in relation_ids {
        let before = current_relations.get(&relation_id).copied();
        let after = target_relations.get(&relation_id).copied();
        if before == after {
            continue;
        }
        if before.is_some() && after.is_some() {
            return Err(WorkVcsError::ReplayUnsupported(format!(
                "restore relation version update for {relation_id} is deferred"
            )));
        }
        operations.push(PreparedRestoreOperation {
            operation_id: OperationId::new_v7(),
            ordinal: operations.len() as i64,
            subject: RestoreSubject::Relation(relation_id),
            operation_payload_json: relation_membership_payload_json(relation_id, before, after)?,
            membership_change: RestoreMembershipChange::Relation {
                relation_id,
                before_relation_version_id: before,
                after_relation_version_id: after,
            },
        });
    }
    Ok(())
}

fn write_restore(
    transaction: &Transaction<'_>,
    options: &WorkStateRestoreOptions,
    prepared: &PreparedRestore,
    write: &RestoreWrite<'_>,
) -> Result<()> {
    let workspace_id_bytes = prepared.workspace_id.raw_bytes();
    let changeset_id_bytes = write.changeset_id.raw_bytes();
    let commit_id_bytes = write.commit_id.raw_bytes();
    let branch_id_bytes = options.branch_id().raw_bytes();
    let previous_head_commit_id_bytes = options.expected_head_commit_id().raw_bytes();
    let work_state_digest_bytes = prepared.work_state_digest.as_bytes();
    let event_id_bytes = write.event_id.raw_bytes();

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
                RESTORE_OPERATION_TYPE,
                RESTORE_OPERATION_SCHEMA_VERSION,
                write.payload_json,
                write.rationale_json,
                write.now_us
            ],
        )
        .map_err(storage_error)?;
    insert_restore_operations(transaction, write.changeset_id, &prepared.operations)?;
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
                write.now_us
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
                &previous_head_commit_id_bytes[..]
            ],
        )
        .map_err(storage_error)?;
    let moved = transaction
        .execute(
            "UPDATE branch
             SET head_commit_id = ?1
             WHERE branch_id = ?2
               AND head_commit_id = ?3",
            params![
                &commit_id_bytes[..],
                &branch_id_bytes[..],
                &previous_head_commit_id_bytes[..]
            ],
        )
        .map_err(storage_error)?;
    if moved != 1 {
        return Err(WorkVcsError::BranchHeadConflict(format!(
            "branch {} head changed before restore commit {} could be installed",
            options.branch_id(),
            write.commit_id
        )));
    }
    super::mark_branch_projection_not_materialized(transaction, options.branch_id(), write.now_us)?;
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
                &event_id_bytes[..],
                &workspace_id_bytes[..],
                &changeset_id_bytes[..],
                RESTORE_EVENT_KIND,
                write.now_us,
                write.payload_json
            ],
        )
        .map_err(storage_error)?;
    Ok(())
}

struct RestoreWrite<'a> {
    now_us: i64,
    changeset_id: ChangeSetId,
    commit_id: CommitId,
    event_id: EventId,
    payload_json: &'a str,
    rationale_json: &'a str,
}

fn insert_restore_operations(
    transaction: &Transaction<'_>,
    changeset_id: ChangeSetId,
    operations: &[PreparedRestoreOperation],
) -> Result<()> {
    let changeset_id_bytes = changeset_id.raw_bytes();
    for operation in operations {
        let operation_id_bytes = operation.operation_id.raw_bytes();
        let subject_object_id_bytes = operation.subject.object_id_bytes();
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
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    &operation_id_bytes[..],
                    &changeset_id_bytes[..],
                    operation.ordinal,
                    operation.subject.kind(),
                    &subject_object_id_bytes[..],
                    operation.operation_payload_json
                ],
            )
            .map_err(storage_error)?;
        match operation.membership_change {
            RestoreMembershipChange::Entity {
                entity_id,
                before_entity_version_id,
                after_entity_version_id,
            } => insert_entity_membership_change(
                transaction,
                operation.operation_id,
                entity_id,
                before_entity_version_id,
                after_entity_version_id,
            )?,
            RestoreMembershipChange::Relation {
                relation_id,
                before_relation_version_id,
                after_relation_version_id,
            } => insert_relation_membership_change(
                transaction,
                operation.operation_id,
                relation_id,
                before_relation_version_id,
                after_relation_version_id,
            )?,
        }
    }
    Ok(())
}

fn insert_entity_membership_change(
    transaction: &Transaction<'_>,
    operation_id: OperationId,
    entity_id: EntityId,
    before_entity_version_id: Option<EntityVersionId>,
    after_entity_version_id: Option<EntityVersionId>,
) -> Result<()> {
    let operation_id_bytes = operation_id.raw_bytes();
    let entity_id_bytes = entity_id.raw_bytes();
    let before_entity_version_id_bytes =
        before_entity_version_id.map(|version_id| version_id.raw_bytes());
    let after_entity_version_id_bytes =
        after_entity_version_id.map(|version_id| version_id.raw_bytes());
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
                &operation_id_bytes[..],
                &entity_id_bytes[..],
                before_entity_version_id_bytes
                    .as_ref()
                    .map(|bytes| &bytes[..]),
                after_entity_version_id_bytes
                    .as_ref()
                    .map(|bytes| &bytes[..]),
                EMPTY_FIELD_DELTA
            ],
        )
        .map_err(storage_error)?;
    Ok(())
}

fn insert_relation_membership_change(
    transaction: &Transaction<'_>,
    operation_id: OperationId,
    relation_id: RelationId,
    before_relation_version_id: Option<RelationVersionId>,
    after_relation_version_id: Option<RelationVersionId>,
) -> Result<()> {
    let operation_id_bytes = operation_id.raw_bytes();
    let relation_id_bytes = relation_id.raw_bytes();
    let before_relation_version_id_bytes =
        before_relation_version_id.map(|version_id| version_id.raw_bytes());
    let after_relation_version_id_bytes =
        after_relation_version_id.map(|version_id| version_id.raw_bytes());
    transaction
        .execute(
            "INSERT INTO relation_membership_change(
                operation_id,
                relation_id,
                before_relation_version_id,
                after_relation_version_id,
                field_delta_json
             )
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                &operation_id_bytes[..],
                &relation_id_bytes[..],
                before_relation_version_id_bytes
                    .as_ref()
                    .map(|bytes| &bytes[..]),
                after_relation_version_id_bytes
                    .as_ref()
                    .map(|bytes| &bytes[..]),
                EMPTY_FIELD_DELTA
            ],
        )
        .map_err(storage_error)?;
    Ok(())
}

struct RestorePayload {
    workspace_id: WorkspaceId,
    branch_id: BranchId,
    previous_head_commit_id: CommitId,
    target_commit_id: CommitId,
    commit_id: CommitId,
    operation_count: usize,
}

fn restore_payload_json(payload: &RestorePayload) -> Result<String> {
    canonical_json_string(&CanonicalValue::object(vec![
        (
            "branch_id".to_owned(),
            CanonicalValue::String(payload.branch_id.to_string()),
        ),
        (
            "commit_id".to_owned(),
            CanonicalValue::String(payload.commit_id.to_string()),
        ),
        (
            "operation_count".to_owned(),
            canonical_count("restore operation count", payload.operation_count)?,
        ),
        (
            "previous_head_commit_id".to_owned(),
            CanonicalValue::String(payload.previous_head_commit_id.to_string()),
        ),
        (
            "target_commit_id".to_owned(),
            CanonicalValue::String(payload.target_commit_id.to_string()),
        ),
        (
            "workspace_id".to_owned(),
            CanonicalValue::String(payload.workspace_id.to_string()),
        ),
    ])?)
}

fn entity_membership_payload_json(
    entity_id: EntityId,
    before_entity_version_id: Option<EntityVersionId>,
    after_entity_version_id: Option<EntityVersionId>,
) -> Result<String> {
    let before_value = match before_entity_version_id {
        Some(version_id) => CanonicalValue::String(version_id.to_string()),
        None => CanonicalValue::Null,
    };
    let after_value = match after_entity_version_id {
        Some(version_id) => CanonicalValue::String(version_id.to_string()),
        None => CanonicalValue::Null,
    };
    canonical_json_string(&CanonicalValue::object(vec![
        ("after_entity_version_id".to_owned(), after_value),
        ("before_entity_version_id".to_owned(), before_value),
        (
            "entity_id".to_owned(),
            CanonicalValue::String(entity_id.to_string()),
        ),
    ])?)
}

fn relation_membership_payload_json(
    relation_id: RelationId,
    before_relation_version_id: Option<RelationVersionId>,
    after_relation_version_id: Option<RelationVersionId>,
) -> Result<String> {
    let before_value = match before_relation_version_id {
        Some(version_id) => CanonicalValue::String(version_id.to_string()),
        None => CanonicalValue::Null,
    };
    let after_value = match after_relation_version_id {
        Some(version_id) => CanonicalValue::String(version_id.to_string()),
        None => CanonicalValue::Null,
    };
    canonical_json_string(&CanonicalValue::object(vec![
        ("after_relation_version_id".to_owned(), after_value),
        ("before_relation_version_id".to_owned(), before_value),
        (
            "relation_id".to_owned(),
            CanonicalValue::String(relation_id.to_string()),
        ),
    ])?)
}

fn canonical_count(label: &str, count: usize) -> Result<CanonicalValue> {
    let count = i64::try_from(count)
        .map_err(|_| WorkVcsError::WorkspaceInvalid(format!("{label} does not fit into i64")))?;
    CanonicalValue::safe_integer(count)
}

fn canonical_json_string(value: &CanonicalValue) -> Result<String> {
    String::from_utf8(canonical_bytes(value)?).map_err(|error| {
        WorkVcsError::CanonicalEncodingInvalid(format!("canonical JSON was not UTF-8: {error}"))
    })
}

fn require_object_value(label: &str, value: &CanonicalValue) -> Result<()> {
    match value {
        CanonicalValue::Object(_) => Ok(()),
        _ => Err(WorkVcsError::WorkspaceInvalid(format!(
            "{label} must be an object"
        ))),
    }
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
        return Err(WorkVcsError::WorkspaceInvalid(format!(
            "branch {branch_id} has lifecycle state {lifecycle_state:?}"
        )));
    }
    Ok(BranchRow {
        workspace_id: decode_workspace_id("branch.workspace_id", workspace_id)?,
        head_commit_id: decode_commit_id("branch.head_commit_id", head_commit_id)?,
    })
}

fn entity_map(entries: &[(EntityId, EntityVersionId)]) -> BTreeMap<EntityId, EntityVersionId> {
    entries.iter().copied().collect()
}

fn relation_map(
    entries: &[(RelationId, RelationVersionId)],
) -> BTreeMap<RelationId, RelationVersionId> {
    entries.iter().copied().collect()
}

fn decode_workspace_id(column: &str, bytes: Vec<u8>) -> Result<WorkspaceId> {
    let bytes = decode_16(column, bytes)?;
    WorkspaceId::from_bytes(bytes).map_err(|error| {
        WorkVcsError::WorkspaceInvalid(format!("{column} is not a UUIDv7 value: {error}"))
    })
}

fn decode_commit_id(column: &str, bytes: Vec<u8>) -> Result<CommitId> {
    let bytes = decode_16(column, bytes)?;
    CommitId::from_bytes(bytes).map_err(|error| {
        WorkVcsError::WorkspaceInvalid(format!("{column} is not a UUIDv7 value: {error}"))
    })
}

fn decode_16(column: &str, bytes: Vec<u8>) -> Result<[u8; 16]> {
    bytes.try_into().map_err(|bytes: Vec<u8>| {
        WorkVcsError::WorkspaceInvalid(format!("{column} must be 16 bytes, found {}", bytes.len()))
    })
}
