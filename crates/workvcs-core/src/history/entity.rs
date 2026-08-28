use crate::canonical::{
    CanonicalValue, WorkState, canonical_bytes, entity_version_digest, work_state_mapping_digest,
};
use crate::error::{Result, WorkVcsError, storage_error};
use crate::identity::{
    BranchId, ChangeSetId, CommitId, Digest, EntityId, EntityVersionId, EventId, OperationId,
    WorkspaceId,
};
use crate::store::{StoreConnection, current_epoch_micros};
use rusqlite::{OptionalExtension, Transaction, TransactionBehavior, params};

pub(crate) const ENTITY_TRANSITION_OPERATION_SCHEMA_VERSION: i64 = 1;
pub(crate) const ENTITY_TRANSITION_OPERATION_TYPE: &str = "entity.transition";

const ACTIVE_BRANCH_LIFECYCLE_STATE: &str = "active";
const EMPTY_FIELD_DELTA: &str = "{}";
const ENTITY_OBJECT_KIND: &str = "entity";
const ENTITY_TRANSITION_EVENT_KIND: &str = "entity.transitioned";
const NORMAL_COMMIT_KIND: &str = "normal";
const PRIMARY_PARENT_ROLE: &str = "primary";
const STATE_SCHEMA_VERSION: i64 = 1;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EntityTransitionOptions {
    branch_id: BranchId,
    expected_head_commit_id: CommitId,
    subject: EntityTransitionSubject,
    state: CanonicalValue,
    rationale: CanonicalValue,
}

impl EntityTransitionOptions {
    pub fn create(
        branch_id: BranchId,
        expected_head_commit_id: CommitId,
        entity_kind: impl Into<String>,
        state: CanonicalValue,
    ) -> Result<Self> {
        let entity_kind = entity_kind.into();
        validate_entity_kind(&entity_kind)?;
        Ok(Self {
            branch_id,
            expected_head_commit_id,
            subject: EntityTransitionSubject::Create { entity_kind },
            state,
            rationale: CanonicalValue::object(Vec::new())?,
        })
    }

    pub fn update(
        branch_id: BranchId,
        expected_head_commit_id: CommitId,
        entity_id: EntityId,
        expected_entity_version_id: EntityVersionId,
        state: CanonicalValue,
    ) -> Result<Self> {
        Ok(Self {
            branch_id,
            expected_head_commit_id,
            subject: EntityTransitionSubject::Update {
                entity_id,
                expected_entity_version_id,
            },
            state,
            rationale: CanonicalValue::object(Vec::new())?,
        })
    }

    pub fn with_rationale(mut self, rationale: CanonicalValue) -> Self {
        self.rationale = rationale;
        self
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum EntityTransitionSubject {
    Create {
        entity_kind: String,
    },
    Update {
        entity_id: EntityId,
        expected_entity_version_id: EntityVersionId,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EntityTransitionCommit {
    pub workspace_id: WorkspaceId,
    pub branch_id: BranchId,
    pub previous_head_commit_id: CommitId,
    pub commit_id: CommitId,
    pub changeset_id: ChangeSetId,
    pub operation_id: OperationId,
    pub entity_id: EntityId,
    pub entity_version_id: EntityVersionId,
    pub entity_state_digest: Digest,
    pub work_state_digest: Digest,
}

pub(crate) fn commit_entity_transition(
    connection: &mut StoreConnection,
    options: &EntityTransitionOptions,
) -> Result<EntityTransitionCommit> {
    connection.verify_foreign_keys()?;

    let parent = super::state_at(connection, options.expected_head_commit_id)?;
    let now_us = current_epoch_micros()?;
    let changeset_id = ChangeSetId::new_v7();
    let commit_id = CommitId::new_v7();
    let operation_id = OperationId::new_v7();

    let transaction = connection
        .inner_mut()
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(storage_error)?;

    let branch = load_branch(&transaction, options.branch_id)?;
    if branch.head_commit_id != options.expected_head_commit_id {
        return Err(WorkVcsError::BranchHeadConflict(format!(
            "branch {} expected head {}, found {}",
            options.branch_id, options.expected_head_commit_id, branch.head_commit_id
        )));
    }
    if branch.workspace_id != parent.workspace_id {
        return Err(WorkVcsError::EntityTransitionInvalid(format!(
            "branch {} belongs to workspace {}, but expected head {} belongs to workspace {}",
            options.branch_id,
            branch.workspace_id,
            options.expected_head_commit_id,
            parent.workspace_id
        )));
    }

    let prepared = prepare_transition(&transaction, branch.workspace_id, &parent.state, options)?;
    write_transition(
        &transaction,
        &prepared,
        now_us,
        changeset_id,
        commit_id,
        operation_id,
        options,
    )?;

    let commit_id_bytes = commit_id.raw_bytes();
    let branch_id_bytes = options.branch_id.raw_bytes();
    let expected_head_bytes = options.expected_head_commit_id.raw_bytes();
    let moved = transaction
        .execute(
            "UPDATE branch
             SET head_commit_id = ?1
             WHERE branch_id = ?2
               AND head_commit_id = ?3",
            params![
                &commit_id_bytes[..],
                &branch_id_bytes[..],
                &expected_head_bytes[..]
            ],
        )
        .map_err(storage_error)?;
    if moved != 1 {
        return Err(WorkVcsError::BranchHeadConflict(format!(
            "branch {} head changed before commit {} could be installed",
            options.branch_id, commit_id
        )));
    }

    transaction.commit().map_err(storage_error)?;

    Ok(EntityTransitionCommit {
        workspace_id: branch.workspace_id,
        branch_id: options.branch_id,
        previous_head_commit_id: options.expected_head_commit_id,
        commit_id,
        changeset_id,
        operation_id,
        entity_id: prepared.entity_id,
        entity_version_id: prepared.entity_version_id,
        entity_state_digest: prepared.entity_state_digest,
        work_state_digest: prepared.work_state_digest,
    })
}

struct BranchRow {
    workspace_id: WorkspaceId,
    head_commit_id: CommitId,
}

struct PreparedTransition {
    workspace_id: WorkspaceId,
    entity_id: EntityId,
    entity_kind: String,
    before_entity_version_id: Option<EntityVersionId>,
    entity_version_id: EntityVersionId,
    entity_state_json: String,
    entity_state_digest: Digest,
    operation_payload_json: String,
    rationale_json: String,
    work_state_digest: Digest,
}

fn load_branch(transaction: &Transaction<'_>, branch_id: BranchId) -> Result<BranchRow> {
    let branch_id_bytes = branch_id.raw_bytes();
    let row = transaction
        .query_row(
            "SELECT workspace_id, head_commit_id, lifecycle_state
             FROM branch
             WHERE branch_id = ?1",
            params![&branch_id_bytes[..]],
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
        return Err(WorkVcsError::EntityTransitionInvalid(format!(
            "branch {branch_id} has lifecycle state {lifecycle_state:?}"
        )));
    }

    Ok(BranchRow {
        workspace_id: decode_workspace_id("branch.workspace_id", workspace_id)?,
        head_commit_id: decode_commit_id("branch.head_commit_id", head_commit_id)?,
    })
}

fn prepare_transition(
    transaction: &Transaction<'_>,
    workspace_id: WorkspaceId,
    parent_state: &WorkState,
    options: &EntityTransitionOptions,
) -> Result<PreparedTransition> {
    let entity_version_id = EntityVersionId::new_v7();
    let entity_state_json = canonical_json_string(&options.state)?;
    let entity_state_digest = entity_version_digest(&options.state)?;
    let rationale_json = canonical_json_string(&options.rationale)?;

    let (entity_id, entity_kind, before_entity_version_id) = match &options.subject {
        EntityTransitionSubject::Create { entity_kind } => {
            let entity_id = EntityId::new_v7();
            require_parent_entity_version(parent_state, entity_id, None)?;
            (entity_id, entity_kind.clone(), None)
        }
        EntityTransitionSubject::Update {
            entity_id,
            expected_entity_version_id,
        } => {
            require_parent_entity_version(
                parent_state,
                *entity_id,
                Some(*expected_entity_version_id),
            )?;
            let entity_kind = load_entity_kind(transaction, workspace_id, *entity_id)?;
            (*entity_id, entity_kind, Some(*expected_entity_version_id))
        }
    };

    let new_state = apply_entity_version(
        parent_state,
        entity_id,
        before_entity_version_id,
        entity_version_id,
    )?;
    let work_state_digest = work_state_mapping_digest(&new_state);
    let operation_payload_json =
        entity_transition_payload_json(entity_id, before_entity_version_id, entity_version_id)?;

    Ok(PreparedTransition {
        workspace_id,
        entity_id,
        entity_kind,
        before_entity_version_id,
        entity_version_id,
        entity_state_json,
        entity_state_digest,
        operation_payload_json,
        rationale_json,
        work_state_digest,
    })
}

fn write_transition(
    transaction: &Transaction<'_>,
    prepared: &PreparedTransition,
    now_us: i64,
    changeset_id: ChangeSetId,
    commit_id: CommitId,
    operation_id: OperationId,
    options: &EntityTransitionOptions,
) -> Result<()> {
    let workspace_id_bytes = prepared.workspace_id.raw_bytes();
    let entity_id_bytes = prepared.entity_id.raw_bytes();
    let entity_version_id_bytes = prepared.entity_version_id.raw_bytes();
    let entity_state_digest_bytes = prepared.entity_state_digest.as_bytes();
    let changeset_id_bytes = changeset_id.raw_bytes();
    let commit_id_bytes = commit_id.raw_bytes();
    let operation_id_bytes = operation_id.raw_bytes();
    let parent_commit_id_bytes = options.expected_head_commit_id.raw_bytes();
    let work_state_digest_bytes = prepared.work_state_digest.as_bytes();
    let before_entity_version_id_bytes = prepared
        .before_entity_version_id
        .map(|version_id| version_id.raw_bytes());
    let before_entity_version_id_param = before_entity_version_id_bytes
        .as_ref()
        .map(|bytes| &bytes[..]);

    if matches!(options.subject, EntityTransitionSubject::Create { .. }) {
        transaction
            .execute(
                "INSERT INTO object_identity(object_id, object_kind, created_at_us)
                 VALUES (?1, ?2, ?3)",
                params![&entity_id_bytes[..], ENTITY_OBJECT_KIND, now_us],
            )
            .map_err(storage_error)?;
        transaction
            .execute(
                "INSERT INTO entity(object_id, workspace_id, entity_kind)
                 VALUES (?1, ?2, ?3)",
                params![
                    &entity_id_bytes[..],
                    &workspace_id_bytes[..],
                    prepared.entity_kind
                ],
            )
            .map_err(storage_error)?;
    }

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
                &entity_version_id_bytes[..],
                &entity_id_bytes[..],
                STATE_SCHEMA_VERSION,
                prepared.entity_state_json,
                &entity_state_digest_bytes[..]
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
                prepared.operation_payload_json,
                prepared.rationale_json,
                now_us
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
                &operation_id_bytes[..],
                &changeset_id_bytes[..],
                &entity_id_bytes[..],
                prepared.operation_payload_json
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
                &operation_id_bytes[..],
                &entity_id_bytes[..],
                before_entity_version_id_param,
                &entity_version_id_bytes[..],
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
                now_us
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
                now_us,
                prepared.operation_payload_json
            ],
        )
        .map_err(storage_error)?;

    Ok(())
}

fn load_entity_kind(
    transaction: &Transaction<'_>,
    workspace_id: WorkspaceId,
    entity_id: EntityId,
) -> Result<String> {
    let entity_id_bytes = entity_id.raw_bytes();
    let row = transaction
        .query_row(
            "SELECT entity.workspace_id, entity.entity_kind, object_identity.object_kind
             FROM entity
             JOIN object_identity ON object_identity.object_id = entity.object_id
             WHERE entity.object_id = ?1",
            params![&entity_id_bytes[..]],
            |row| {
                Ok((
                    row.get::<_, Vec<u8>>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            },
        )
        .optional()
        .map_err(storage_error)?;

    let Some((entity_workspace_id, entity_kind, object_kind)) = row else {
        return Err(WorkVcsError::EntityNotFound(format!(
            "entity {entity_id} does not exist"
        )));
    };
    if object_kind != ENTITY_OBJECT_KIND {
        return Err(WorkVcsError::EntityTransitionInvalid(format!(
            "entity {entity_id} has object kind {object_kind:?}"
        )));
    }
    let entity_workspace_id = decode_workspace_id("entity.workspace_id", entity_workspace_id)?;
    if entity_workspace_id != workspace_id {
        return Err(WorkVcsError::EntityTransitionInvalid(format!(
            "entity {entity_id} belongs to workspace {entity_workspace_id}, not {workspace_id}"
        )));
    }
    validate_entity_kind(&entity_kind)?;
    Ok(entity_kind)
}

fn require_parent_entity_version(
    parent_state: &WorkState,
    entity_id: EntityId,
    expected_entity_version_id: Option<EntityVersionId>,
) -> Result<()> {
    let current = parent_state
        .entities()
        .iter()
        .find_map(|(candidate_entity_id, version_id)| {
            (*candidate_entity_id == entity_id).then_some(*version_id)
        });
    if current == expected_entity_version_id {
        Ok(())
    } else {
        Err(WorkVcsError::EntityTransitionInvalid(format!(
            "entity {entity_id} expected parent version {expected_entity_version_id:?}, found {current:?}"
        )))
    }
}

fn apply_entity_version(
    parent_state: &WorkState,
    entity_id: EntityId,
    before_entity_version_id: Option<EntityVersionId>,
    after_entity_version_id: EntityVersionId,
) -> Result<WorkState> {
    let mut entities = Vec::new();
    let mut replaced = false;
    for (candidate_entity_id, candidate_version_id) in parent_state.entities() {
        if *candidate_entity_id == entity_id {
            replaced = true;
            if Some(*candidate_version_id) != before_entity_version_id {
                return Err(WorkVcsError::EntityTransitionInvalid(format!(
                    "entity {entity_id} expected parent version {before_entity_version_id:?}, found {candidate_version_id}"
                )));
            }
            entities.push((*candidate_entity_id, after_entity_version_id));
        } else {
            entities.push((*candidate_entity_id, *candidate_version_id));
        }
    }
    if before_entity_version_id.is_none() {
        if replaced {
            return Err(WorkVcsError::EntityTransitionInvalid(format!(
                "entity {entity_id} was expected to be absent before creation"
            )));
        }
        entities.push((entity_id, after_entity_version_id));
    } else if !replaced {
        return Err(WorkVcsError::EntityTransitionInvalid(format!(
            "entity {entity_id} was expected to be present before update"
        )));
    }

    WorkState::new(entities, parent_state.relations().to_vec())
}

pub(crate) fn canonical_json_string(value: &CanonicalValue) -> Result<String> {
    String::from_utf8(canonical_bytes(value)?).map_err(|error| {
        WorkVcsError::CanonicalEncodingInvalid(format!("canonical JSON was not UTF-8: {error}"))
    })
}

pub(crate) fn entity_transition_payload_json(
    entity_id: EntityId,
    before_entity_version_id: Option<EntityVersionId>,
    after_entity_version_id: EntityVersionId,
) -> Result<String> {
    let before_value = match before_entity_version_id {
        Some(version_id) => CanonicalValue::String(version_id.to_string()),
        None => CanonicalValue::Null,
    };
    let payload = CanonicalValue::object(vec![
        (
            "after_entity_version_id".to_owned(),
            CanonicalValue::String(after_entity_version_id.to_string()),
        ),
        ("before_entity_version_id".to_owned(), before_value),
        (
            "entity_id".to_owned(),
            CanonicalValue::String(entity_id.to_string()),
        ),
    ])?;
    canonical_json_string(&payload)
}

fn validate_entity_kind(value: &str) -> Result<()> {
    if value.is_empty() {
        return Err(WorkVcsError::EntityTransitionInvalid(
            "entity kind must not be empty".to_owned(),
        ));
    }
    if value.trim() != value {
        return Err(WorkVcsError::EntityTransitionInvalid(
            "entity kind must not have leading or trailing whitespace".to_owned(),
        ));
    }
    if value
        .as_bytes()
        .iter()
        .any(|byte| *byte == b'\0' || *byte < 0x20 || *byte == 0x7f)
    {
        return Err(WorkVcsError::EntityTransitionInvalid(
            "entity kind must not contain NUL or ASCII control characters".to_owned(),
        ));
    }
    Ok(())
}

fn decode_workspace_id(column: &str, bytes: Vec<u8>) -> Result<WorkspaceId> {
    let bytes = decode_16(column, bytes)?;
    WorkspaceId::from_bytes(bytes).map_err(|error| {
        WorkVcsError::EntityTransitionInvalid(format!("{column} is not a UUIDv7 value: {error}"))
    })
}

fn decode_commit_id(column: &str, bytes: Vec<u8>) -> Result<CommitId> {
    let bytes = decode_16(column, bytes)?;
    CommitId::from_bytes(bytes).map_err(|error| {
        WorkVcsError::EntityTransitionInvalid(format!("{column} is not a UUIDv7 value: {error}"))
    })
}

fn decode_16(column: &str, bytes: Vec<u8>) -> Result<[u8; 16]> {
    bytes.try_into().map_err(|bytes: Vec<u8>| {
        WorkVcsError::EntityTransitionInvalid(format!(
            "{column} must be 16 bytes, found {}",
            bytes.len()
        ))
    })
}
