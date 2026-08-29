use rusqlite::{Connection, params};
use std::path::{Path, PathBuf};
use tempfile::TempDir;
use workvcs_core::{
    BranchForkOptions, BranchId, CanonicalValue, ClaimLifecycleState, ClaimTaskOptions, Engine,
    EntityId, ErrorCategory, ErrorCode, SessionEndOptions, SessionFocusOptions,
    SessionLifecycleState, SessionStartOptions, SessionSwitchOptions, StoreInitOptions,
    TaskCreateOptions, WorkspaceInfo, WorkspaceInitOptions,
};

#[derive(Clone, Debug, PartialEq, Eq)]
struct RuntimeCounts {
    claim_runtime: i64,
    session_context_workspace: i64,
    session_focus: i64,
    session_focus_path: i64,
    changeset: i64,
    workstate_commit: i64,
    event: i64,
}

fn store_path() -> (TempDir, PathBuf) {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let path = tempdir.path().join("workvcs.sqlite");
    (tempdir, path)
}

fn init_engine(path: &Path) -> Engine {
    Engine::init(
        path,
        StoreInitOptions::new("phase3ag-session-switch-store").expect("store options"),
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

fn count_rows(connection: &Connection, table: &str) -> i64 {
    connection
        .query_row(&format!("SELECT count(*) FROM {table}"), [], |row| {
            row.get(0)
        })
        .expect("count rows")
}

fn runtime_counts(connection: &Connection) -> RuntimeCounts {
    RuntimeCounts {
        claim_runtime: count_rows(connection, "claim_runtime"),
        session_context_workspace: count_rows(connection, "session_context_workspace"),
        session_focus: count_rows(connection, "session_focus"),
        session_focus_path: count_rows(connection, "session_focus_path"),
        changeset: count_rows(connection, "changeset"),
        workstate_commit: count_rows(connection, "workstate_commit"),
        event: count_rows(connection, "event"),
    }
}

fn event_count(connection: &Connection, event_kind: &str) -> i64 {
    connection
        .query_row(
            "SELECT count(*)
             FROM event
             WHERE event_kind = ?1
               AND changeset_id IS NULL",
            params![event_kind],
            |row| row.get(0),
        )
        .expect("event count")
}

fn metadata(label: &str) -> CanonicalValue {
    CanonicalValue::object(vec![(
        "label".to_owned(),
        CanonicalValue::String(label.to_owned()),
    )])
    .expect("metadata")
}

#[test]
fn switch_session_to_forked_branch_releases_old_claims_and_clears_focus() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let task = engine
        .create_task(
            TaskCreateOptions::new(
                workspace.initial_branch_id,
                workspace.genesis_commit_id,
                "Switchable task",
            )
            .expect("task options"),
        )
        .expect("create task");
    let fork = engine
        .fork_branch(
            BranchForkOptions::from_branch(workspace.initial_branch_id, "experiment")
                .expect("fork options"),
        )
        .expect("fork branch");
    let started = engine
        .start_session(
            SessionStartOptions::new(workspace.workspace_id, workspace.initial_branch_id)
                .expect("session options")
                .with_metadata(metadata("agent"))
                .expect("metadata"),
        )
        .expect("start session");
    let focused = engine
        .set_session_focus(SessionFocusOptions::new(
            started.session_id,
            task.task_entity_id,
        ))
        .expect("set focus");
    assert!(focused.state.focus.is_some());
    let claim = engine
        .claim_task(ClaimTaskOptions::new(
            started.session_id,
            task.task_entity_id,
        ))
        .expect("claim task");

    let connection = raw_connection(&path);
    let before = runtime_counts(&connection);
    drop(connection);

    let switched = engine
        .switch_session(SessionSwitchOptions::new(
            started.session_id,
            workspace.workspace_id,
            fork.branch_id,
        ))
        .expect("switch branch");

    assert_eq!(switched.previous_workspace_id, workspace.workspace_id);
    assert_eq!(switched.previous_branch_id, workspace.initial_branch_id);
    assert_eq!(switched.active_workspace_id, workspace.workspace_id);
    assert_eq!(switched.active_branch_id, fork.branch_id);
    assert_eq!(switched.released_claims, 1);
    assert_eq!(
        switched.state.lifecycle_state,
        SessionLifecycleState::Active
    );
    assert_eq!(
        switched.state.active_workspace_id,
        Some(workspace.workspace_id)
    );
    assert_eq!(switched.state.active_branch_id, Some(fork.branch_id));
    assert_eq!(switched.state.focus, None);
    assert!(switched.occurred_at_us >= started.started_at_us);

    let released = engine
        .claim_snapshot(claim.claim_id)
        .expect("claim snapshot");
    assert_eq!(released.lifecycle_state, ClaimLifecycleState::Released);
    assert_eq!(released.branch_id, workspace.initial_branch_id);
    assert_eq!(released.last_activity_at_us, None);

    let connection = raw_connection(&path);
    let after = runtime_counts(&connection);
    assert_eq!(after.claim_runtime, before.claim_runtime - 1);
    assert_eq!(
        after.session_context_workspace,
        before.session_context_workspace
    );
    assert_eq!(after.session_focus, before.session_focus - 1);
    assert_eq!(after.session_focus_path, before.session_focus_path);
    assert_eq!(after.changeset, before.changeset);
    assert_eq!(after.workstate_commit, before.workstate_commit);
    assert_eq!(after.event, before.event + 2);
    assert_eq!(event_count(&connection, "claim.released"), 1);
    assert_eq!(event_count(&connection, "session.switched"), 1);
}

#[test]
fn switch_session_can_set_new_focus_on_target_branch() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let task = engine
        .create_task(
            TaskCreateOptions::new(
                workspace.initial_branch_id,
                workspace.genesis_commit_id,
                "Focused fork task",
            )
            .expect("task options"),
        )
        .expect("create task");
    let fork = engine
        .fork_branch(
            BranchForkOptions::from_branch(workspace.initial_branch_id, "focused")
                .expect("fork options"),
        )
        .expect("fork branch");
    let started = engine
        .start_session(
            SessionStartOptions::new(workspace.workspace_id, workspace.initial_branch_id)
                .expect("session options"),
        )
        .expect("start session");

    let switched = engine
        .switch_session(
            SessionSwitchOptions::new(started.session_id, workspace.workspace_id, fork.branch_id)
                .with_focus(task.task_entity_id),
        )
        .expect("switch with focus");

    let focus = switched.state.focus.expect("focus");
    assert_eq!(focus.focus_entity_id, task.task_entity_id);
    assert!(focus.path.is_empty());
    assert_eq!(switched.released_claims, 0);
}

#[test]
fn switch_session_rejects_invalid_targets_without_changing_active_target() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let other_workspace = engine
        .create_workspace(WorkspaceInitOptions::new("other").expect("workspace options"))
        .expect("create other workspace");
    let started = engine
        .start_session(
            SessionStartOptions::new(workspace.workspace_id, workspace.initial_branch_id)
                .expect("session options"),
        )
        .expect("start session");

    let mismatch = engine
        .switch_session(SessionSwitchOptions::new(
            started.session_id,
            workspace.workspace_id,
            other_workspace.initial_branch_id,
        ))
        .expect_err("workspace mismatch");
    assert_eq!(mismatch.code(), ErrorCode::SessionInvalid);
    assert_eq!(mismatch.category(), ErrorCategory::Runtime);

    let unknown_branch = engine
        .switch_session(SessionSwitchOptions::new(
            started.session_id,
            workspace.workspace_id,
            BranchId::new_v7(),
        ))
        .expect_err("unknown branch");
    assert_eq!(unknown_branch.code(), ErrorCode::QueryInvalid);

    let missing_focus = engine
        .switch_session(
            SessionSwitchOptions::new(
                started.session_id,
                workspace.workspace_id,
                workspace.initial_branch_id,
            )
            .with_focus(EntityId::new_v7()),
        )
        .expect_err("missing focus");
    assert_eq!(missing_focus.code(), ErrorCode::SessionInvalid);

    let snapshot = engine
        .session_snapshot(started.session_id)
        .expect("unchanged session");
    assert_eq!(snapshot.active_workspace_id, Some(workspace.workspace_id));
    assert_eq!(snapshot.active_branch_id, Some(workspace.initial_branch_id));
}

#[test]
fn switch_session_rejects_ended_session() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let started = engine
        .start_session(
            SessionStartOptions::new(workspace.workspace_id, workspace.initial_branch_id)
                .expect("session options"),
        )
        .expect("start session");
    engine
        .end_session(SessionEndOptions::new(started.session_id).expect("end options"))
        .expect("end session");

    let error = engine
        .switch_session(SessionSwitchOptions::new(
            started.session_id,
            workspace.workspace_id,
            workspace.initial_branch_id,
        ))
        .expect_err("ended session");
    assert_eq!(error.code(), ErrorCode::SessionInvalid);
}
