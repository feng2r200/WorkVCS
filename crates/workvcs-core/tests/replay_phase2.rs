use rusqlite::{Connection, params};
use std::path::{Path, PathBuf};
use tempfile::TempDir;
use workvcs_core::{
    ChangeSetId, CommitId, Engine, EntityId, ErrorCode, EventId, StoreInitOptions, WorkState,
    WorkspaceInfo, WorkspaceInitOptions, work_state_mapping_digest,
};

fn store_path() -> (TempDir, PathBuf) {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let path = tempdir.path().join("workvcs.sqlite");
    (tempdir, path)
}

fn init_engine(path: &Path) -> Engine {
    Engine::init(
        path,
        StoreInitOptions::new("phase2-replay-store").expect("store options"),
    )
    .expect("init engine")
}

fn raw_connection(path: &Path) -> Connection {
    let connection = Connection::open(path).expect("raw connection");
    connection
        .pragma_update(None, "foreign_keys", "ON")
        .expect("enable raw foreign keys");
    connection
}

fn create_workspace(path: &Path, name: &str) -> (Engine, WorkspaceInfo) {
    let mut engine = init_engine(path);
    let workspace = engine
        .create_workspace(WorkspaceInitOptions::new(name).expect("workspace options"))
        .expect("create workspace");
    (engine, workspace)
}

fn insert_normal_commit_after_genesis(
    connection: &Connection,
    workspace: &WorkspaceInfo,
) -> CommitId {
    let workspace_id = workspace.workspace_id.raw_bytes();
    let genesis_commit_id = workspace.genesis_commit_id.raw_bytes();
    let changeset_id = ChangeSetId::new_v7().raw_bytes();
    let commit_id = CommitId::new_v7();
    let commit_id_bytes = commit_id.raw_bytes();

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
             VALUES (?1, ?2, 'test.normal', 1, '{}', '{}', NULL, 1)",
            params![&changeset_id[..], &workspace_id[..]],
        )
        .expect("insert normal changeset");
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
             VALUES (?1, ?2, ?3, 'normal', ?4, 2)",
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
        .expect("insert normal parent");

    commit_id
}

fn insert_merge_commit_after_genesis(
    connection: &Connection,
    workspace: &WorkspaceInfo,
) -> CommitId {
    let workspace_id = workspace.workspace_id.raw_bytes();
    let primary_parent_id = workspace.genesis_commit_id.raw_bytes();
    let secondary_parent_id = insert_normal_commit_after_genesis(connection, workspace).raw_bytes();
    let changeset_id = ChangeSetId::new_v7().raw_bytes();
    let commit_id = CommitId::new_v7();
    let commit_id_bytes = commit_id.raw_bytes();

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
             VALUES (?1, ?2, 'test.merge', 1, '{}', '{}', NULL, 3)",
            params![&changeset_id[..], &workspace_id[..]],
        )
        .expect("insert merge changeset");
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
             VALUES (?1, ?2, ?3, 'merge', ?4, 4)",
            params![
                &commit_id_bytes[..],
                &workspace_id[..],
                &changeset_id[..],
                &workspace.state_digest.as_bytes()[..]
            ],
        )
        .expect("insert merge commit");
    connection
        .execute(
            "INSERT INTO commit_parent(
                commit_id,
                parent_ordinal,
                parent_role,
                parent_commit_id
             )
             VALUES (?1, 0, 'primary', ?2)",
            params![&commit_id_bytes[..], &primary_parent_id[..]],
        )
        .expect("insert merge primary parent");
    connection
        .execute(
            "INSERT INTO commit_parent(
                commit_id,
                parent_ordinal,
                parent_role,
                parent_commit_id
             )
             VALUES (?1, 1, 'secondary', ?2)",
            params![&commit_id_bytes[..], &secondary_parent_id[..]],
        )
        .expect("insert merge secondary parent");

    commit_id
}

#[test]
fn state_at_replays_genesis_to_empty_work_state() {
    let (_tempdir, path) = store_path();
    let (engine, workspace) = create_workspace(&path, "alpha");

    let replayed = engine
        .state_at(workspace.genesis_commit_id)
        .expect("state_at genesis");

    assert_eq!(replayed.workspace_id, workspace.workspace_id);
    assert_eq!(replayed.commit_id, workspace.genesis_commit_id);
    assert_eq!(replayed.state, WorkState::empty());
    assert_eq!(replayed.state_digest, workspace.state_digest);
    assert_eq!(
        replayed.state_digest,
        work_state_mapping_digest(&WorkState::empty())
    );
}

#[test]
fn state_at_replays_genesis_after_reopen() {
    let (_tempdir, path) = store_path();
    let (engine, workspace) = create_workspace(&path, "reopen");
    drop(engine);

    let reopened = Engine::open(&path).expect("reopen engine");
    let replayed = reopened
        .state_at(workspace.genesis_commit_id)
        .expect("state_at reopened genesis");

    assert_eq!(replayed.state, WorkState::empty());
    assert_eq!(replayed.state_digest, workspace.state_digest);
}

#[test]
fn state_at_does_not_use_events_or_projection_as_replay_truth() {
    let (_tempdir, path) = store_path();
    let (engine, workspace) = create_workspace(&path, "independent");

    let connection = raw_connection(&path);
    let branch_id = workspace.initial_branch_id.raw_bytes();
    let workspace_id = workspace.workspace_id.raw_bytes();
    let commit_id = workspace.genesis_commit_id.raw_bytes();
    let changeset_id = workspace.genesis_changeset_id.raw_bytes();
    connection
        .execute(
            "UPDATE event SET payload_json = ?1 WHERE changeset_id = ?2",
            params![r#"{"not":"replay truth"}"#, &changeset_id[..]],
        )
        .expect("corrupt event payload");
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
            params![&branch_id[..], &commit_id[..], &[9_u8; 32][..]],
        )
        .expect("insert inconsistent projection");
    drop(connection);

    let replayed = engine
        .state_at(workspace.genesis_commit_id)
        .expect("state_at ignores event and projection");

    assert_eq!(replayed.workspace_id.raw_bytes(), workspace_id);
    assert_eq!(replayed.state, WorkState::empty());
    assert_eq!(replayed.state_digest, workspace.state_digest);
}

#[test]
fn state_at_reports_missing_commit() {
    let (_tempdir, path) = store_path();
    let engine = init_engine(&path);

    let error = engine
        .state_at(CommitId::new_v7())
        .expect_err("missing commit");

    assert_eq!(error.code(), ErrorCode::CommitNotFound);
}

#[test]
fn state_at_rejects_genesis_with_parent() {
    let (_tempdir, path) = store_path();
    let (engine, workspace) = create_workspace(&path, "parent");

    let connection = raw_connection(&path);
    let genesis_commit_id = workspace.genesis_commit_id.raw_bytes();
    let parent_commit_id = insert_normal_commit_after_genesis(&connection, &workspace).raw_bytes();
    connection
        .execute(
            "INSERT INTO commit_parent(
                commit_id,
                parent_ordinal,
                parent_role,
                parent_commit_id
             )
             VALUES (?1, 1, 'secondary', ?2)",
            params![&genesis_commit_id[..], &parent_commit_id[..]],
        )
        .expect("insert invalid genesis parent");
    drop(connection);

    let error = engine
        .state_at(workspace.genesis_commit_id)
        .expect_err("genesis parent is invalid");

    assert_eq!(error.code(), ErrorCode::ReplayInvalid);
}

#[test]
fn state_at_rejects_genesis_with_change_operation() {
    let (_tempdir, path) = store_path();
    let (engine, workspace) = create_workspace(&path, "operation");

    let connection = raw_connection(&path);
    let changeset_id = workspace.genesis_changeset_id.raw_bytes();
    let object_id = EntityId::new_v7().raw_bytes();
    let operation_id = EventId::new_v7().raw_bytes();
    connection
        .execute(
            "INSERT INTO object_identity(object_id, object_kind, created_at_us)
             VALUES (?1, 'entity', 1)",
            params![&object_id[..]],
        )
        .expect("insert object identity");
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
             VALUES (?1, ?2, 0, 'entity', ?3, '{}')",
            params![&operation_id[..], &changeset_id[..], &object_id[..]],
        )
        .expect("insert invalid operation");
    drop(connection);

    let error = engine
        .state_at(workspace.genesis_commit_id)
        .expect_err("genesis change operation is invalid");

    assert_eq!(error.code(), ErrorCode::ReplayInvalid);
}

#[test]
fn state_at_rejects_genesis_digest_mismatch() {
    let (_tempdir, path) = store_path();
    let (engine, workspace) = create_workspace(&path, "digest");

    let connection = raw_connection(&path);
    let commit_id = workspace.genesis_commit_id.raw_bytes();
    connection
        .execute(
            "UPDATE workstate_commit SET state_digest = ?1 WHERE commit_id = ?2",
            params![&[7_u8; 32][..], &commit_id[..]],
        )
        .expect("corrupt digest");
    drop(connection);

    let error = engine
        .state_at(workspace.genesis_commit_id)
        .expect_err("digest mismatch");

    assert_eq!(error.code(), ErrorCode::ReplayInvalid);
}

#[test]
fn state_at_rejects_wrong_genesis_operation_type() {
    let (_tempdir, path) = store_path();
    let (engine, workspace) = create_workspace(&path, "operation-type");

    let connection = raw_connection(&path);
    let changeset_id = workspace.genesis_changeset_id.raw_bytes();
    connection
        .execute(
            "UPDATE changeset SET operation_type = ?1 WHERE changeset_id = ?2",
            params!["test.not-genesis", &changeset_id[..]],
        )
        .expect("corrupt operation type");
    drop(connection);

    let error = engine
        .state_at(workspace.genesis_commit_id)
        .expect_err("operation type mismatch");

    assert_eq!(error.code(), ErrorCode::ReplayInvalid);
}

#[test]
fn state_at_rejects_corrupted_genesis_payload_json() {
    let (_tempdir, path) = store_path();
    let (engine, workspace) = create_workspace(&path, "payload");

    let connection = raw_connection(&path);
    let changeset_id = workspace.genesis_changeset_id.raw_bytes();
    connection
        .execute(
            "UPDATE changeset SET operation_payload_json = ?1 WHERE changeset_id = ?2",
            params![r#"{"unexpected":true}"#, &changeset_id[..]],
        )
        .expect("corrupt operation payload");
    drop(connection);

    let error = engine
        .state_at(workspace.genesis_commit_id)
        .expect_err("operation payload mismatch");

    assert_eq!(error.code(), ErrorCode::ReplayInvalid);
}

#[test]
fn state_at_rejects_corrupted_genesis_rationale_json() {
    let (_tempdir, path) = store_path();
    let (engine, workspace) = create_workspace(&path, "rationale");

    let connection = raw_connection(&path);
    let changeset_id = workspace.genesis_changeset_id.raw_bytes();
    connection
        .execute(
            "UPDATE changeset SET rationale_json = ?1 WHERE changeset_id = ?2",
            params![r#"{"unexpected":true}"#, &changeset_id[..]],
        )
        .expect("corrupt rationale");
    drop(connection);

    let error = engine
        .state_at(workspace.genesis_commit_id)
        .expect_err("rationale mismatch");

    assert_eq!(error.code(), ErrorCode::ReplayInvalid);
}

#[test]
fn state_at_marks_unsupported_normal_operations_unsupported() {
    let (_tempdir, path) = store_path();
    let (engine, workspace) = create_workspace(&path, "normal");

    let connection = raw_connection(&path);
    let normal_commit_id = insert_normal_commit_after_genesis(&connection, &workspace);
    drop(connection);

    let error = engine
        .state_at(normal_commit_id)
        .expect_err("unsupported normal operation");

    assert_eq!(error.code(), ErrorCode::ReplayUnsupported);
}

#[test]
fn state_at_marks_merge_commits_unsupported_for_this_slice() {
    let (_tempdir, path) = store_path();
    let (engine, workspace) = create_workspace(&path, "merge");

    let connection = raw_connection(&path);
    let merge_commit_id = insert_merge_commit_after_genesis(&connection, &workspace);
    drop(connection);

    let error = engine
        .state_at(merge_commit_id)
        .expect_err("merge replay not implemented");

    assert_eq!(error.code(), ErrorCode::ReplayUnsupported);
}
