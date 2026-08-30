use rusqlite::Connection;
use std::path::{Path, PathBuf};
use tempfile::TempDir;
use workvcs_core::{
    ClaimLifecycleState, ClaimNextOptions, CommitId, Engine, EntityId,
    RunnableTaskClaimCoordination, RunnableTasksOptions, SessionId, SessionLifecycleState,
    SessionStartOptions, StoreInitOptions, TaskCreateCommit, TaskCreateOptions, WorkspaceInfo,
    WorkspaceInitOptions,
};

#[derive(Clone, Debug, PartialEq, Eq)]
struct RuntimeCounts {
    object_identity: i64,
    claim: i64,
    claim_runtime: i64,
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
        StoreInitOptions::new("phase3ah-claim-next-store").expect("store options"),
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
        object_identity: count_rows(connection, "object_identity"),
        claim: count_rows(connection, "claim"),
        claim_runtime: count_rows(connection, "claim_runtime"),
        session_focus: count_rows(connection, "session_focus"),
        session_focus_path: count_rows(connection, "session_focus_path"),
        changeset: count_rows(connection, "changeset"),
        workstate_commit: count_rows(connection, "workstate_commit"),
        event: count_rows(connection, "event"),
    }
}

fn create_task(
    engine: &mut Engine,
    workspace: &WorkspaceInfo,
    head: CommitId,
    description: &str,
) -> TaskCreateCommit {
    engine
        .create_task(
            TaskCreateOptions::new(workspace.initial_branch_id, head, description)
                .expect("task options"),
        )
        .expect("create task")
}

fn first_unclaimed_runnable(engine: &Engine, session_id: SessionId) -> Option<EntityId> {
    engine
        .runnable_tasks(RunnableTasksOptions::new(session_id))
        .expect("runnable projection")
        .candidates
        .into_iter()
        .find(|candidate| {
            candidate.runnable
                && matches!(
                    candidate.claim_coordination,
                    RunnableTaskClaimCoordination::Unclaimed
                )
        })
        .map(|candidate| candidate.task.task_entity_id)
}

#[test]
fn claim_next_selects_unclaimed_runnable_task_and_focuses_session() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let first = create_task(
        &mut engine,
        &workspace,
        workspace.genesis_commit_id,
        "First runnable",
    );
    let _second = create_task(&mut engine, &workspace, first.commit_id, "Second runnable");
    let started = engine
        .start_session(
            SessionStartOptions::new(workspace.workspace_id, workspace.initial_branch_id)
                .expect("session options"),
        )
        .expect("start session");

    let expected_first =
        first_unclaimed_runnable(&engine, started.session_id).expect("first runnable");
    let connection = raw_connection(&path);
    let before = runtime_counts(&connection);
    drop(connection);

    let claimed = engine
        .claim_next_task(ClaimNextOptions::new(started.session_id))
        .expect("claim next");
    let selected = claimed.selected.expect("selected claim");
    assert_eq!(claimed.session_id, started.session_id);
    assert_eq!(claimed.workspace_id, workspace.workspace_id);
    assert_eq!(claimed.branch_id, workspace.initial_branch_id);
    assert_eq!(selected.task_entity_id, expected_first);
    assert_eq!(selected.state.lifecycle_state, ClaimLifecycleState::Active);

    let session = engine
        .session_snapshot(started.session_id)
        .expect("session snapshot");
    assert_eq!(session.lifecycle_state, SessionLifecycleState::Active);
    assert_eq!(
        session.focus.expect("claim-next focus").focus_entity_id,
        expected_first
    );

    let connection = raw_connection(&path);
    let after = runtime_counts(&connection);
    assert_eq!(after.object_identity, before.object_identity + 1);
    assert_eq!(after.claim, before.claim + 1);
    assert_eq!(after.claim_runtime, before.claim_runtime + 1);
    assert_eq!(after.session_focus, before.session_focus + 1);
    assert_eq!(after.session_focus_path, before.session_focus_path);
    assert_eq!(after.changeset, before.changeset);
    assert_eq!(after.workstate_commit, before.workstate_commit);
    assert_eq!(after.event, before.event + 2);
}

#[test]
fn claim_next_uses_entity_id_as_final_stable_tiebreaker() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let first = create_task(
        &mut engine,
        &workspace,
        workspace.genesis_commit_id,
        "Equal candidate A",
    );
    let second = create_task(
        &mut engine,
        &workspace,
        first.commit_id,
        "Equal candidate B",
    );
    let third = create_task(
        &mut engine,
        &workspace,
        second.commit_id,
        "Equal candidate C",
    );
    let started = engine
        .start_session(
            SessionStartOptions::new(workspace.workspace_id, workspace.initial_branch_id)
                .expect("session options"),
        )
        .expect("start session");
    let expected = [
        first.task_entity_id,
        second.task_entity_id,
        third.task_entity_id,
    ]
    .into_iter()
    .min()
    .expect("expected minimum task id");

    let claimed = engine
        .claim_next_task(ClaimNextOptions::new(started.session_id))
        .expect("claim next");
    let selected = claimed.selected.expect("selected claim");

    assert_eq!(selected.task_entity_id, expected);
}

#[test]
fn claim_next_skips_tasks_already_claimed_by_same_session() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let first = create_task(
        &mut engine,
        &workspace,
        workspace.genesis_commit_id,
        "First runnable",
    );
    let _second = create_task(&mut engine, &workspace, first.commit_id, "Second runnable");
    let started = engine
        .start_session(
            SessionStartOptions::new(workspace.workspace_id, workspace.initial_branch_id)
                .expect("session options"),
        )
        .expect("start session");

    let first_expected =
        first_unclaimed_runnable(&engine, started.session_id).expect("first runnable");
    let first_claim = engine
        .claim_next_task(ClaimNextOptions::new(started.session_id))
        .expect("first claim next")
        .selected
        .expect("first selected");
    assert_eq!(first_claim.task_entity_id, first_expected);

    engine
        .clear_session_focus(started.session_id)
        .expect("clear first focus");
    let second_expected =
        first_unclaimed_runnable(&engine, started.session_id).expect("second runnable");
    assert_ne!(second_expected, first_claim.task_entity_id);
    let second_claim = engine
        .claim_next_task(ClaimNextOptions::new(started.session_id))
        .expect("second claim next")
        .selected
        .expect("second selected");
    assert_eq!(second_claim.task_entity_id, second_expected);

    engine
        .clear_session_focus(started.session_id)
        .expect("clear second focus");
    let connection = raw_connection(&path);
    let before_empty = runtime_counts(&connection);
    drop(connection);
    let empty = engine
        .claim_next_task(ClaimNextOptions::new(started.session_id))
        .expect("empty claim next");
    assert!(empty.selected.is_none());
    assert_eq!(empty.inspected_candidates, 2);
    let connection = raw_connection(&path);
    assert_eq!(runtime_counts(&connection), before_empty);
}

#[test]
fn claim_next_reports_empty_projection_without_mutation() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let started = engine
        .start_session(
            SessionStartOptions::new(workspace.workspace_id, workspace.initial_branch_id)
                .expect("session options"),
        )
        .expect("start session");
    let connection = raw_connection(&path);
    let before = runtime_counts(&connection);
    drop(connection);

    let next = engine
        .claim_next_task(ClaimNextOptions::new(started.session_id))
        .expect("claim next empty");

    assert!(next.selected.is_none());
    assert_eq!(next.inspected_candidates, 0);
    let connection = raw_connection(&path);
    assert_eq!(runtime_counts(&connection), before);
}
