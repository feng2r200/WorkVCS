use rusqlite::{Connection, params};
use std::path::{Path, PathBuf};
use tempfile::TempDir;
use workvcs_core::{
    BranchForkOptions, BranchId, CanonicalValue, CommitId, Engine, ErrorCategory, ErrorCode,
    MergeAbortOptions, MergeStartOptions, SessionId, SessionStartOptions, StoreInitOptions,
    TaskCreateOptions, WorkspaceInfo, WorkspaceInitOptions, canonical_bytes,
};

fn store_path() -> (TempDir, PathBuf) {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let path = tempdir.path().join("workvcs.sqlite");
    (tempdir, path)
}

fn init_engine(path: &Path) -> Engine {
    Engine::init(
        path,
        StoreInitOptions::new("phase4b-merge-store").expect("store options"),
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

fn detail() -> CanonicalValue {
    CanonicalValue::object(vec![(
        "reason".to_owned(),
        CanonicalValue::String("no longer needed".to_owned()),
    )])
    .expect("detail")
}

fn canonical_json(value: &CanonicalValue) -> String {
    String::from_utf8(canonical_bytes(value).expect("canonical bytes")).expect("utf8")
}

#[test]
fn abort_merge_records_outcome_without_moving_branch_or_creating_commit() {
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
    let merge = engine
        .start_merge(
            MergeStartOptions::new(workspace.initial_branch_id, source_branch.branch_id)
                .with_origin_session_id(session.session_id),
        )
        .expect("start merge");

    let connection = raw_connection(&path);
    let before_outcomes = count_rows(&connection, "merge_attempt_outcome");
    let before_commits = count_rows(&connection, "workstate_commit");
    let before_events = count_rows(&connection, "event");
    drop(connection);

    let detail = detail();
    let aborted = engine
        .abort_merge(
            MergeAbortOptions::new(merge.merge_id)
                .expect("abort options")
                .with_abort_session_id(session.session_id)
                .with_detail(detail.clone())
                .expect("detail"),
        )
        .expect("abort merge");

    assert_eq!(aborted.merge_id, merge.merge_id);
    assert_eq!(aborted.workspace_id, workspace.workspace_id);
    assert_eq!(aborted.target_branch_id, workspace.initial_branch_id);
    assert_eq!(aborted.source_branch_id, source_branch.branch_id);
    assert_eq!(aborted.merge_base_commit_id, base_commit);
    assert_eq!(aborted.target_head_commit_id, target_head);
    assert_eq!(aborted.source_head_commit_id, source_head);
    assert_eq!(aborted.origin_session_id, Some(session.session_id));
    assert_eq!(aborted.abort_session_id, Some(session.session_id));
    assert_eq!(aborted.runtime_state.as_str(), "aborted");

    let duplicate_abort = engine
        .abort_merge(MergeAbortOptions::new(merge.merge_id).expect("abort options"))
        .expect_err("aborting an already aborted merge should fail");
    assert_eq!(duplicate_abort.code(), ErrorCode::WorkspaceInvalid);
    assert_eq!(duplicate_abort.category(), ErrorCategory::Workspace);

    let restarted = engine
        .start_merge(MergeStartOptions::new(
            workspace.initial_branch_id,
            source_branch.branch_id,
        ))
        .expect("start replacement merge");
    assert_ne!(restarted.merge_id, merge.merge_id);
    assert_eq!(restarted.merge_base_commit_id, base_commit);

    let invalid_detail = MergeAbortOptions::new(restarted.merge_id)
        .expect("abort options")
        .with_detail(CanonicalValue::Array(Vec::new()))
        .expect_err("merge abort detail must be an object");
    assert_eq!(invalid_detail.code(), ErrorCode::WorkspaceInvalid);

    let connection = raw_connection(&path);
    assert_eq!(
        count_rows(&connection, "merge_attempt_outcome"),
        before_outcomes + 1
    );
    assert_eq!(count_rows(&connection, "workstate_commit"), before_commits);
    assert_eq!(count_rows(&connection, "event"), before_events + 2);
    assert_eq!(
        branch_head(&connection, workspace.initial_branch_id),
        target_head
    );
    assert_eq!(
        session_last_activity(&connection, session.session_id),
        aborted.aborted_at_us
    );

    let merge_id = aborted.merge_id.raw_bytes();
    let stored = connection
        .query_row(
            "SELECT merge_runtime.runtime_json,
                    merge_attempt_outcome.outcome,
                    merge_attempt_outcome.result_commit_id,
                    merge_attempt_outcome.completed_at_us,
                    merge_attempt_outcome.detail_json,
                    event.event_kind,
                    event.payload_json
             FROM merge_attempt
             JOIN merge_runtime
               ON merge_runtime.merge_id = merge_attempt.merge_id
             JOIN merge_attempt_outcome
               ON merge_attempt_outcome.merge_id = merge_attempt.merge_id
             JOIN event
               ON event.event_id = ?2
             WHERE merge_attempt.merge_id = ?1",
            params![&merge_id[..], &aborted.event_id.raw_bytes()[..]],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<Vec<u8>>>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, String>(6)?,
                ))
            },
        )
        .expect("stored aborted merge");
    assert_eq!(stored.0, r#"{"lifecycle_state":"aborted"}"#);
    assert_eq!(stored.1, "aborted");
    assert!(stored.2.is_none());
    assert_eq!(stored.3, aborted.aborted_at_us);
    assert_eq!(stored.4, canonical_json(&detail));
    assert_eq!(stored.5, "merge.aborted");
    assert_eq!(
        stored.6,
        format!(
            "{{\"abort_session_id\":\"{}\",\"detail\":{},\"lifecycle_state\":\"aborted\",\"merge_base_commit_id\":\"{}\",\"merge_id\":\"{}\",\"origin_session_id\":\"{}\",\"outcome\":\"aborted\",\"source_branch_id\":\"{}\",\"source_head_commit_id\":\"{}\",\"target_branch_id\":\"{}\",\"target_head_commit_id\":\"{}\",\"workspace_id\":\"{}\"}}",
            session.session_id,
            canonical_json(&detail),
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
