use rusqlite::{Connection, params};
use std::path::{Path, PathBuf};
use tempfile::TempDir;
use workvcs_core::{
    BranchForkOptions, BranchId, CommitId, Engine, ErrorCategory, ErrorCode, MergeStartOptions,
    SessionId, SessionStartOptions, StoreInitOptions, TaskCreateOptions, WorkspaceInfo,
    WorkspaceInitOptions,
};

fn store_path() -> (TempDir, PathBuf) {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let path = tempdir.path().join("workvcs.sqlite");
    (tempdir, path)
}

fn init_engine(path: &Path) -> Engine {
    Engine::init(
        path,
        StoreInitOptions::new("phase4a-merge-store").expect("store options"),
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

fn create_task(
    engine: &mut Engine,
    branch_id: BranchId,
    head_commit_id: CommitId,
    description: &str,
) -> CommitId {
    engine
        .create_task(
            TaskCreateOptions::new(branch_id, head_commit_id, description).expect("task options"),
        )
        .expect("create task")
        .commit_id
}

fn raw_connection(path: &Path) -> Connection {
    let connection = Connection::open(path).expect("raw connection");
    connection
        .pragma_update(None, "foreign_keys", "ON")
        .expect("enable raw foreign keys");
    connection
}

fn count_rows(connection: &Connection, table: &str) -> i64 {
    connection
        .query_row(&format!("SELECT count(*) FROM {table}"), [], |row| {
            row.get(0)
        })
        .expect("count rows")
}

fn branch_head(connection: &Connection, branch_id: BranchId) -> CommitId {
    let branch_id = branch_id.raw_bytes();
    let bytes = connection
        .query_row(
            "SELECT head_commit_id
             FROM branch
             WHERE branch_id = ?1",
            params![&branch_id[..]],
            |row| row.get::<_, Vec<u8>>(0),
        )
        .expect("branch head");
    CommitId::from_bytes(bytes.try_into().expect("commit id bytes")).expect("commit id")
}

fn session_last_activity(connection: &Connection, session_id: SessionId) -> i64 {
    let session_id = session_id.raw_bytes();
    connection
        .query_row(
            "SELECT last_activity_at_us
             FROM session_runtime
             WHERE session_id = ?1",
            params![&session_id[..]],
            |row| row.get(0),
        )
        .expect("session activity")
}

#[test]
fn start_merge_captures_branch_heads_and_runtime_without_advancing_target() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);

    let base_commit = create_task(
        &mut engine,
        workspace.initial_branch_id,
        workspace.genesis_commit_id,
        "shared merge base",
    );
    let source_branch = engine
        .fork_branch(
            BranchForkOptions::from_branch(workspace.initial_branch_id, "source")
                .expect("fork options"),
        )
        .expect("fork source branch");
    let target_head = create_task(
        &mut engine,
        workspace.initial_branch_id,
        base_commit,
        "target branch work",
    );
    let source_head = create_task(
        &mut engine,
        source_branch.branch_id,
        base_commit,
        "source branch work",
    );
    let session = engine
        .start_session(
            SessionStartOptions::new(workspace.workspace_id, workspace.initial_branch_id)
                .expect("session options"),
        )
        .expect("start session");

    let connection = raw_connection(&path);
    let before_merge_attempts = count_rows(&connection, "merge_attempt");
    let before_merge_runtime = count_rows(&connection, "merge_runtime");
    let before_merge_items = count_rows(&connection, "merge_item");
    let before_workstate_commits = count_rows(&connection, "workstate_commit");
    let before_events = count_rows(&connection, "event");
    drop(connection);

    let merge = engine
        .start_merge(
            MergeStartOptions::new(workspace.initial_branch_id, source_branch.branch_id)
                .with_origin_session_id(session.session_id),
        )
        .expect("start merge");

    assert_eq!(merge.workspace_id, workspace.workspace_id);
    assert_eq!(merge.target_branch_id, workspace.initial_branch_id);
    assert_eq!(merge.source_branch_id, source_branch.branch_id);
    assert_eq!(merge.merge_base_commit_id, base_commit);
    assert_eq!(merge.target_head_commit_id, target_head);
    assert_eq!(merge.source_head_commit_id, source_head);
    assert_eq!(merge.origin_session_id, Some(session.session_id));
    assert_eq!(merge.runtime_state.as_str(), "active");

    let duplicate = engine
        .start_merge(MergeStartOptions::new(
            workspace.initial_branch_id,
            source_branch.branch_id,
        ))
        .expect_err("duplicate active merge should fail");
    assert_eq!(duplicate.code(), ErrorCode::WorkspaceInvalid);
    assert_eq!(duplicate.category(), ErrorCategory::Workspace);

    let same_branch = engine
        .start_merge(MergeStartOptions::new(
            source_branch.branch_id,
            source_branch.branch_id,
        ))
        .expect_err("same branch merge should fail");
    assert_eq!(same_branch.code(), ErrorCode::WorkspaceInvalid);

    let connection = raw_connection(&path);
    assert_eq!(
        count_rows(&connection, "merge_attempt"),
        before_merge_attempts + 1
    );
    assert_eq!(
        count_rows(&connection, "merge_runtime"),
        before_merge_runtime + 1
    );
    assert_eq!(count_rows(&connection, "merge_item"), before_merge_items);
    assert_eq!(
        count_rows(&connection, "workstate_commit"),
        before_workstate_commits
    );
    assert_eq!(count_rows(&connection, "event"), before_events + 1);
    assert_eq!(
        branch_head(&connection, workspace.initial_branch_id),
        target_head
    );
    assert_eq!(
        session_last_activity(&connection, session.session_id),
        merge.created_at_us
    );

    let merge_id = merge.merge_id.raw_bytes();
    let stored = connection
        .query_row(
            "SELECT object_identity.object_kind,
                    merge_runtime.runtime_json,
                    merge_attempt.merge_base_commit_id,
                    merge_attempt.target_head_commit_id,
                    merge_attempt.source_head_commit_id,
                    event.event_kind,
                    event.payload_json
             FROM merge_attempt
             JOIN object_identity
               ON object_identity.object_id = merge_attempt.merge_id
             JOIN merge_runtime
               ON merge_runtime.merge_id = merge_attempt.merge_id
             JOIN event
               ON event.event_id = ?2
             WHERE merge_attempt.merge_id = ?1",
            params![&merge_id[..], &merge.event_id.raw_bytes()[..]],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Vec<u8>>(2)?,
                    row.get::<_, Vec<u8>>(3)?,
                    row.get::<_, Vec<u8>>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, String>(6)?,
                ))
            },
        )
        .expect("stored merge attempt");
    assert_eq!(stored.0, "merge_attempt");
    assert_eq!(stored.1, r#"{"lifecycle_state":"active"}"#);
    assert_eq!(stored.2, base_commit.raw_bytes());
    assert_eq!(stored.3, target_head.raw_bytes());
    assert_eq!(stored.4, source_head.raw_bytes());
    assert_eq!(stored.5, "merge.started");
    assert_eq!(
        stored.6,
        format!(
            "{{\"lifecycle_state\":\"active\",\"merge_base_commit_id\":\"{}\",\"merge_id\":\"{}\",\"origin_session_id\":\"{}\",\"source_branch_id\":\"{}\",\"source_head_commit_id\":\"{}\",\"target_branch_id\":\"{}\",\"target_head_commit_id\":\"{}\",\"workspace_id\":\"{}\"}}",
            base_commit,
            merge.merge_id,
            session.session_id,
            source_branch.branch_id,
            source_head,
            workspace.initial_branch_id,
            target_head,
            workspace.workspace_id
        )
    );
}
