use rusqlite::{Connection, params};
use std::path::{Path, PathBuf};
use tempfile::TempDir;
use workvcs_core::{
    BranchId, CanonicalValue, ChangeSetId, CommitId, Digest, Engine, EntityTransitionOptions,
    EntityVersionId, ErrorCategory, ErrorCode, OperationId, RelationId, StoreInitOptions,
    WorkState, WorkspaceInfo, WorkspaceInitOptions, entity_version_digest,
};

fn store_path() -> (TempDir, PathBuf) {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let path = tempdir.path().join("workvcs.sqlite");
    (tempdir, path)
}

fn init_engine(path: &Path) -> Engine {
    Engine::init(
        path,
        StoreInitOptions::new("phase2-entity-store").expect("store options"),
    )
    .expect("init engine")
}

fn create_workspace(path: &Path) -> (Engine, WorkspaceInfo) {
    let mut engine = init_engine(path);
    let workspace = engine
        .create_workspace(WorkspaceInitOptions::new("workspace").expect("workspace options"))
        .expect("create workspace");
    (engine, workspace)
}

fn raw_connection(path: &Path) -> Connection {
    let connection = Connection::open(path).expect("raw connection");
    connection
        .pragma_update(None, "foreign_keys", "ON")
        .expect("enable raw foreign keys");
    connection
}

fn record_state(title: &str, status: &str) -> CanonicalValue {
    CanonicalValue::object(vec![
        ("title".to_owned(), CanonicalValue::String(title.to_owned())),
        (
            "status".to_owned(),
            CanonicalValue::String(status.to_owned()),
        ),
    ])
    .expect("record state")
}

fn swapped_state() -> CanonicalValue {
    CanonicalValue::object(vec![
        ("b".to_owned(), CanonicalValue::String("two".to_owned())),
        (
            "a".to_owned(),
            CanonicalValue::safe_integer(1).expect("safe integer"),
        ),
    ])
    .expect("swapped state")
}

fn canonical_state() -> CanonicalValue {
    CanonicalValue::object(vec![
        (
            "a".to_owned(),
            CanonicalValue::safe_integer(1).expect("safe integer"),
        ),
        ("b".to_owned(), CanonicalValue::String("two".to_owned())),
    ])
    .expect("canonical state")
}

fn branch_head(connection: &Connection, branch_id: BranchId) -> CommitId {
    let branch_id_bytes = branch_id.raw_bytes();
    let bytes = connection
        .query_row(
            "SELECT head_commit_id FROM branch WHERE branch_id = ?1",
            params![&branch_id_bytes[..]],
            |row| row.get::<_, Vec<u8>>(0),
        )
        .expect("branch head");
    CommitId::from_bytes(bytes.try_into().expect("commit id bytes")).expect("commit id")
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct HistoryCounts {
    object_identity: i64,
    entity: i64,
    entity_version: i64,
    changeset: i64,
    change_operation: i64,
    entity_membership_change: i64,
    workstate_commit: i64,
    commit_parent: i64,
    event: i64,
}

fn history_counts(connection: &Connection) -> HistoryCounts {
    HistoryCounts {
        object_identity: count_rows(connection, "object_identity"),
        entity: count_rows(connection, "entity"),
        entity_version: count_rows(connection, "entity_version"),
        changeset: count_rows(connection, "changeset"),
        change_operation: count_rows(connection, "change_operation"),
        entity_membership_change: count_rows(connection, "entity_membership_change"),
        workstate_commit: count_rows(connection, "workstate_commit"),
        commit_parent: count_rows(connection, "commit_parent"),
        event: count_rows(connection, "event"),
    }
}

fn count_rows(connection: &Connection, table: &str) -> i64 {
    connection
        .query_row(&format!("SELECT count(*) FROM {table}"), [], |row| {
            row.get::<_, i64>(0)
        })
        .expect("row count")
}

fn digest_from_blob(bytes: Vec<u8>) -> Digest {
    Digest::from_bytes(bytes.try_into().expect("digest bytes"))
}

#[test]
fn commits_two_entity_transitions_and_replays_history_after_reopen() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);

    let first = engine
        .commit_entity_transition(
            EntityTransitionOptions::create(
                workspace.initial_branch_id,
                workspace.genesis_commit_id,
                "record",
                record_state("draft", "open"),
            )
            .expect("create options"),
        )
        .expect("first entity transition");
    let second = engine
        .commit_entity_transition(
            EntityTransitionOptions::update(
                workspace.initial_branch_id,
                first.commit_id,
                first.entity_id,
                first.entity_version_id,
                record_state("draft", "done"),
            )
            .expect("update options"),
        )
        .expect("second entity transition");

    let connection = raw_connection(&path);
    assert_eq!(
        branch_head(&connection, workspace.initial_branch_id),
        second.commit_id
    );
    drop(connection);

    let first_replayed = engine.state_at(first.commit_id).expect("first state");
    assert_eq!(
        first_replayed.state.entities(),
        &[(first.entity_id, first.entity_version_id)]
    );
    let second_replayed = engine.state_at(second.commit_id).expect("second state");
    assert_eq!(
        second_replayed.state.entities(),
        &[(first.entity_id, second.entity_version_id)]
    );
    assert_eq!(second_replayed.state_digest, second.work_state_digest);

    drop(engine);
    let reopened = Engine::open(&path).expect("reopen engine");
    let reopened_state = reopened
        .state_at(second.commit_id)
        .expect("reopened second state");
    assert_eq!(reopened_state.state, second_replayed.state);
    assert_eq!(reopened_state.state_digest, second.work_state_digest);
}

#[test]
fn entity_transition_writes_atomic_history_rows() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);

    let commit = engine
        .commit_entity_transition(
            EntityTransitionOptions::create(
                workspace.initial_branch_id,
                workspace.genesis_commit_id,
                "record",
                record_state("row", "open"),
            )
            .expect("create options"),
        )
        .expect("entity transition");

    let connection = raw_connection(&path);
    assert_eq!(
        history_counts(&connection),
        HistoryCounts {
            object_identity: 1,
            entity: 1,
            entity_version: 1,
            changeset: 2,
            change_operation: 1,
            entity_membership_change: 1,
            workstate_commit: 2,
            commit_parent: 1,
            event: 2,
        }
    );
    assert_eq!(
        branch_head(&connection, workspace.initial_branch_id),
        commit.commit_id
    );
}

#[test]
fn entity_state_is_stored_as_canonical_json_and_digest() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);

    let commit = engine
        .commit_entity_transition(
            EntityTransitionOptions::create(
                workspace.initial_branch_id,
                workspace.genesis_commit_id,
                "record",
                swapped_state(),
            )
            .expect("create options"),
        )
        .expect("entity transition");

    let connection = raw_connection(&path);
    let entity_version_id = commit.entity_version_id.raw_bytes();
    let (state_json, state_digest) = connection
        .query_row(
            "SELECT state_json, state_digest
             FROM entity_version
             WHERE entity_version_id = ?1",
            params![&entity_version_id[..]],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, Vec<u8>>(1)?)),
        )
        .expect("entity version");

    assert_eq!(state_json, r#"{"a":1,"b":"two"}"#);
    assert_eq!(
        digest_from_blob(state_digest),
        entity_version_digest(&canonical_state()).expect("entity digest")
    );
    assert_eq!(
        commit.entity_state_digest,
        entity_version_digest(&canonical_state()).expect("entity digest")
    );
}

#[test]
fn projection_rows_do_not_drive_mutation_or_state_at() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let first = engine
        .commit_entity_transition(
            EntityTransitionOptions::create(
                workspace.initial_branch_id,
                workspace.genesis_commit_id,
                "record",
                record_state("projection", "open"),
            )
            .expect("create options"),
        )
        .expect("first transition");

    let connection = raw_connection(&path);
    let branch_id = workspace.initial_branch_id.raw_bytes();
    let first_commit_id = first.commit_id.raw_bytes();
    let entity_id = first.entity_id.raw_bytes();
    let first_entity_version_id = first.entity_version_id.raw_bytes();
    connection
        .execute(
            "INSERT INTO branch_projection_state(
                branch_id,
                projection_status,
                projected_commit_id,
                projection_state_digest,
                updated_at_us
             )
             VALUES (?1, 'complete', ?2, ?3, 3)",
            params![&branch_id[..], &first_commit_id[..], &[9_u8; 32][..]],
        )
        .expect("insert inconsistent projection state");
    connection
        .execute(
            "INSERT INTO branch_entity_current(branch_id, entity_id, entity_version_id)
             VALUES (?1, ?2, ?3)",
            params![&branch_id[..], &entity_id[..], &first_entity_version_id[..]],
        )
        .expect("insert stale current projection");
    drop(connection);

    let second = engine
        .commit_entity_transition(
            EntityTransitionOptions::update(
                workspace.initial_branch_id,
                first.commit_id,
                first.entity_id,
                first.entity_version_id,
                record_state("projection", "done"),
            )
            .expect("update options"),
        )
        .expect("second transition");

    let replayed = engine.state_at(second.commit_id).expect("second state");
    assert_eq!(
        replayed.state.entities(),
        &[(first.entity_id, second.entity_version_id)]
    );
}

#[test]
fn stale_expected_head_fails_without_partial_history() {
    let (_tempdir, path) = store_path();
    let (mut current_engine, workspace) = create_workspace(&path);
    let mut stale_engine = Engine::open(&path).expect("second engine");

    current_engine
        .commit_entity_transition(
            EntityTransitionOptions::create(
                workspace.initial_branch_id,
                workspace.genesis_commit_id,
                "record",
                record_state("winner", "open"),
            )
            .expect("create options"),
        )
        .expect("winning transition");

    let connection = raw_connection(&path);
    let before = history_counts(&connection);
    drop(connection);

    let error = stale_engine
        .commit_entity_transition(
            EntityTransitionOptions::create(
                workspace.initial_branch_id,
                workspace.genesis_commit_id,
                "record",
                record_state("stale", "open"),
            )
            .expect("stale options"),
        )
        .expect_err("stale head conflict");

    assert_eq!(error.code(), ErrorCode::BranchHeadConflict);
    assert_eq!(error.category(), ErrorCategory::Mutation);
    assert!(error.retryable());

    let connection = raw_connection(&path);
    assert_eq!(history_counts(&connection), before);
}

#[test]
fn wrong_expected_entity_version_rolls_back_without_partial_history() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let first = engine
        .commit_entity_transition(
            EntityTransitionOptions::create(
                workspace.initial_branch_id,
                workspace.genesis_commit_id,
                "record",
                record_state("first", "open"),
            )
            .expect("create options"),
        )
        .expect("first transition");

    let connection = raw_connection(&path);
    let before = history_counts(&connection);
    drop(connection);

    let error = engine
        .commit_entity_transition(
            EntityTransitionOptions::update(
                workspace.initial_branch_id,
                first.commit_id,
                first.entity_id,
                EntityVersionId::new_v7(),
                record_state("bad", "done"),
            )
            .expect("bad update options"),
        )
        .expect_err("wrong version precondition");

    assert_eq!(error.code(), ErrorCode::EntityTransitionInvalid);

    let connection = raw_connection(&path);
    assert_eq!(history_counts(&connection), before);
    assert_eq!(
        branch_head(&connection, workspace.initial_branch_id),
        first.commit_id
    );
}

#[test]
fn missing_branch_is_a_structured_error() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);

    let error = engine
        .commit_entity_transition(
            EntityTransitionOptions::create(
                BranchId::new_v7(),
                workspace.genesis_commit_id,
                "record",
                record_state("missing", "open"),
            )
            .expect("create options"),
        )
        .expect_err("missing branch");

    assert_eq!(error.code(), ErrorCode::BranchNotFound);
    assert_eq!(error.category(), ErrorCategory::Mutation);
}

#[test]
fn replay_rejects_corrupted_entity_version_digest() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let commit = engine
        .commit_entity_transition(
            EntityTransitionOptions::create(
                workspace.initial_branch_id,
                workspace.genesis_commit_id,
                "record",
                record_state("digest", "open"),
            )
            .expect("create options"),
        )
        .expect("entity transition");

    let connection = raw_connection(&path);
    let entity_version_id = commit.entity_version_id.raw_bytes();
    connection
        .execute(
            "UPDATE entity_version
             SET state_digest = ?1
             WHERE entity_version_id = ?2",
            params![&[7_u8; 32][..], &entity_version_id[..]],
        )
        .expect("corrupt entity version digest");
    drop(connection);

    let error = engine
        .state_at(commit.commit_id)
        .expect_err("corrupted entity version digest");

    assert_eq!(error.code(), ErrorCode::ReplayInvalid);
}

#[test]
fn replay_rejects_corrupted_change_operation_payload() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let commit = engine
        .commit_entity_transition(
            EntityTransitionOptions::create(
                workspace.initial_branch_id,
                workspace.genesis_commit_id,
                "record",
                record_state("payload", "open"),
            )
            .expect("create options"),
        )
        .expect("entity transition");

    let connection = raw_connection(&path);
    let operation_id = commit.operation_id.raw_bytes();
    connection
        .execute(
            "UPDATE change_operation
             SET operation_payload_json = ?1
             WHERE operation_id = ?2",
            params![r#"{"unexpected":true}"#, &operation_id[..]],
        )
        .expect("corrupt operation payload");
    drop(connection);

    let error = engine
        .state_at(commit.commit_id)
        .expect_err("corrupted operation payload");

    assert_eq!(error.code(), ErrorCode::ReplayInvalid);
}

#[test]
fn replay_rejects_corrupted_changeset_payload() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let commit = engine
        .commit_entity_transition(
            EntityTransitionOptions::create(
                workspace.initial_branch_id,
                workspace.genesis_commit_id,
                "record",
                record_state("changeset", "open"),
            )
            .expect("create options"),
        )
        .expect("entity transition");

    let connection = raw_connection(&path);
    let changeset_id = commit.changeset_id.raw_bytes();
    connection
        .execute(
            "UPDATE changeset
             SET operation_payload_json = ?1
             WHERE changeset_id = ?2",
            params![r#"{"unexpected":true}"#, &changeset_id[..]],
        )
        .expect("corrupt changeset payload");
    drop(connection);

    let error = engine
        .state_at(commit.commit_id)
        .expect_err("corrupted changeset payload");

    assert_eq!(error.code(), ErrorCode::ReplayInvalid);
}

#[test]
fn relation_change_operation_without_membership_is_invalid() {
    let (_tempdir, path) = store_path();
    let (engine, workspace) = create_workspace(&path);
    let connection = raw_connection(&path);

    let workspace_id = workspace.workspace_id.raw_bytes();
    let genesis_commit_id = workspace.genesis_commit_id.raw_bytes();
    let relation_id = RelationId::new_v7().raw_bytes();
    let changeset_id = ChangeSetId::new_v7().raw_bytes();
    let commit_id = CommitId::new_v7();
    let commit_id_bytes = commit_id.raw_bytes();
    let operation_id = OperationId::new_v7().raw_bytes();
    connection
        .execute(
            "INSERT INTO object_identity(object_id, object_kind, created_at_us)
             VALUES (?1, 'relation', 1)",
            params![&relation_id[..]],
        )
        .expect("insert relation identity");
    connection
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
             VALUES (?1, ?2, 'entity.transition', 1, '{}', '{}', NULL, 2)",
            params![&changeset_id[..], &workspace_id[..]],
        )
        .expect("insert relation changeset");
    connection
        .execute(
            "INSERT INTO change_operation(
                operation_id,
                changeset_id,
                ordinal,
                subject_family,
                subject_object_id,
                operation_payload_json
             )
             VALUES (?1, ?2, 0, 'relation', ?3, '{}')",
            params![&operation_id[..], &changeset_id[..], &relation_id[..]],
        )
        .expect("insert relation operation");
    connection
        .execute(
            "INSERT INTO workstate_commit(
                commit_id,
                workspace_id,
                changeset_id,
                commit_kind,
                state_digest,
                committed_at_us
             )
             VALUES (?1, ?2, ?3, 'normal', ?4, 3)",
            params![
                &commit_id_bytes[..],
                &workspace_id[..],
                &changeset_id[..],
                &workspace.state_digest.as_bytes()[..]
            ],
        )
        .expect("insert normal commit");
    connection
        .execute(
            "INSERT INTO commit_parent(
                commit_id,
                parent_ordinal,
                parent_role,
                parent_commit_id
             )
             VALUES (?1, 0, 'primary', ?2)",
            params![&commit_id_bytes[..], &genesis_commit_id[..]],
        )
        .expect("insert parent");
    drop(connection);

    let error = engine
        .state_at(commit_id)
        .expect_err("relation replay invalid without membership row");

    assert_eq!(error.code(), ErrorCode::ReplayInvalid);
}

#[test]
fn entity_kind_validation_rejects_non_canonical_names() {
    let (_tempdir, path) = store_path();
    let (_engine, workspace) = create_workspace(&path);

    let error = EntityTransitionOptions::create(
        workspace.initial_branch_id,
        workspace.genesis_commit_id,
        " record ",
        record_state("kind", "open"),
    )
    .expect_err("invalid entity kind");

    assert_eq!(error.code(), ErrorCode::EntityTransitionInvalid);
}

#[test]
fn committed_work_state_digest_is_order_independent() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);

    let first = engine
        .commit_entity_transition(
            EntityTransitionOptions::create(
                workspace.initial_branch_id,
                workspace.genesis_commit_id,
                "record",
                record_state("first", "open"),
            )
            .expect("first options"),
        )
        .expect("first transition");
    let second = engine
        .commit_entity_transition(
            EntityTransitionOptions::create(
                workspace.initial_branch_id,
                first.commit_id,
                "record",
                record_state("second", "open"),
            )
            .expect("second options"),
        )
        .expect("second transition");

    let forward = WorkState::new(
        [
            (first.entity_id, first.entity_version_id),
            (second.entity_id, second.entity_version_id),
        ],
        [],
    )
    .expect("forward work state");
    let reverse = WorkState::new(
        [
            (second.entity_id, second.entity_version_id),
            (first.entity_id, first.entity_version_id),
        ],
        [],
    )
    .expect("reverse work state");

    assert_eq!(
        second.work_state_digest,
        workvcs_core::work_state_mapping_digest(&forward)
    );
    assert_eq!(
        second.work_state_digest,
        workvcs_core::work_state_mapping_digest(&reverse)
    );
}
