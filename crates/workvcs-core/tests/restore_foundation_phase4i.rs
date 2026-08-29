use rusqlite::{Connection, params};
use tempfile::TempDir;
use workvcs_core::{
    BranchId, CommitId, Engine, ErrorCode, StoreInitOptions, TaskCreateCommit, TaskCreateOptions,
    WorkState, WorkStateRestoreOptions, WorkspaceInfo, WorkspaceInitOptions,
};

fn store_path() -> (TempDir, std::path::PathBuf) {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let path = tempdir.path().join("workvcs.sqlite");
    (tempdir, path)
}

fn init_engine(path: &std::path::Path) -> Engine {
    Engine::init(
        path,
        StoreInitOptions::new("phase4i-restore-store").expect("store options"),
    )
    .expect("init engine")
}

fn create_workspace(path: &std::path::Path) -> (Engine, WorkspaceInfo) {
    let mut engine = init_engine(path);
    let workspace = engine
        .create_workspace(WorkspaceInitOptions::new("workspace").expect("workspace options"))
        .expect("create workspace");
    (engine, workspace)
}

fn create_task(
    engine: &mut Engine,
    branch_id: BranchId,
    head_commit_id: CommitId,
    description: &str,
) -> TaskCreateCommit {
    engine
        .create_task(
            TaskCreateOptions::new(branch_id, head_commit_id, description).expect("task options"),
        )
        .expect("create task")
}

fn raw_connection(path: &std::path::Path) -> Connection {
    let connection = Connection::open(path).expect("raw connection");
    connection
        .pragma_update(None, "foreign_keys", "ON")
        .expect("enable raw foreign keys");
    connection
}

fn decode_commit_id(bytes: Vec<u8>) -> CommitId {
    CommitId::from_bytes(bytes.try_into().expect("commit id length")).expect("commit id")
}

fn parent_commit(connection: &Connection, commit_id: CommitId) -> CommitId {
    connection
        .query_row(
            "SELECT parent_commit_id
             FROM commit_parent
             WHERE commit_id = ?1
               AND parent_ordinal = 0
               AND parent_role = 'primary'",
            params![&commit_id.raw_bytes()[..]],
            |row| row.get::<_, Vec<u8>>(0).map(decode_commit_id),
        )
        .expect("parent commit")
}

fn changeset_operation_type(connection: &Connection, commit_id: CommitId) -> String {
    connection
        .query_row(
            "SELECT changeset.operation_type
             FROM workstate_commit
             JOIN changeset ON changeset.changeset_id = workstate_commit.changeset_id
             WHERE workstate_commit.commit_id = ?1",
            params![&commit_id.raw_bytes()[..]],
            |row| row.get(0),
        )
        .expect("operation type")
}

fn operation_count(connection: &Connection, commit_id: CommitId) -> i64 {
    connection
        .query_row(
            "SELECT count(*)
             FROM workstate_commit
             JOIN change_operation
               ON change_operation.changeset_id = workstate_commit.changeset_id
             WHERE workstate_commit.commit_id = ?1",
            params![&commit_id.raw_bytes()[..]],
            |row| row.get(0),
        )
        .expect("operation count")
}

#[test]
fn restore_work_state_creates_single_parent_commit_matching_historical_state() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);

    let first = create_task(
        &mut engine,
        workspace.initial_branch_id,
        workspace.genesis_commit_id,
        "First task",
    );
    let second = create_task(
        &mut engine,
        workspace.initial_branch_id,
        first.commit_id,
        "Second task",
    );

    let restored = engine
        .restore_work_state(
            WorkStateRestoreOptions::new(
                workspace.initial_branch_id,
                second.commit_id,
                workspace.genesis_commit_id,
            )
            .expect("restore options"),
        )
        .expect("restore to genesis");
    assert_eq!(restored.workspace_id, workspace.workspace_id);
    assert_eq!(restored.branch_id, workspace.initial_branch_id);
    assert_eq!(restored.previous_head_commit_id, second.commit_id);
    assert_eq!(restored.target_commit_id, workspace.genesis_commit_id);
    assert_eq!(restored.operation_count, 2);

    let branch_head = engine
        .branch_head(workspace.initial_branch_id)
        .expect("branch head");
    assert_eq!(branch_head.head_commit_id, restored.commit_id);

    let replayed = engine.state_at(restored.commit_id).expect("replay restore");
    assert_eq!(replayed.state, WorkState::new([], []).expect("empty state"));
    assert_eq!(replayed.state_digest, restored.work_state_digest);

    let connection = raw_connection(&path);
    assert_eq!(
        parent_commit(&connection, restored.commit_id),
        second.commit_id
    );
    assert_eq!(
        changeset_operation_type(&connection, restored.commit_id),
        "workstate.restore"
    );
    assert_eq!(operation_count(&connection, restored.commit_id), 2);
}

#[test]
fn restore_work_state_rejects_noop_restore() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);

    let error = engine
        .restore_work_state(
            WorkStateRestoreOptions::new(
                workspace.initial_branch_id,
                workspace.genesis_commit_id,
                workspace.genesis_commit_id,
            )
            .expect("restore options"),
        )
        .expect_err("no-op restore");
    assert_eq!(error.code(), ErrorCode::WorkspaceInvalid);
}

#[test]
fn restore_work_state_rejects_stale_expected_head() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);

    let first = create_task(
        &mut engine,
        workspace.initial_branch_id,
        workspace.genesis_commit_id,
        "First task",
    );
    let second = create_task(
        &mut engine,
        workspace.initial_branch_id,
        first.commit_id,
        "Second task",
    );

    let error = engine
        .restore_work_state(
            WorkStateRestoreOptions::new(
                workspace.initial_branch_id,
                first.commit_id,
                workspace.genesis_commit_id,
            )
            .expect("restore options"),
        )
        .expect_err("stale restore head");
    assert_eq!(error.code(), ErrorCode::BranchHeadConflict);

    let branch_head = engine
        .branch_head(workspace.initial_branch_id)
        .expect("branch head");
    assert_eq!(branch_head.head_commit_id, second.commit_id);
}
