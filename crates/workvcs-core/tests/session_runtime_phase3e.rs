use rusqlite::{Connection, params};
use std::path::{Path, PathBuf};
use tempfile::TempDir;
use workvcs_core::{
    AcceptanceCriterionClassification, AcceptanceCriterionCreateOptions, BranchId, CanonicalValue,
    Engine, EntityId, ErrorCategory, ErrorCode, RelationId, SessionEndOptions, SessionFocusOptions,
    SessionFocusPathEntry, SessionId, SessionLifecycleState, SessionStartOptions, StoreInitOptions,
    TaskCreateOptions, VerificationCreateOptions, VerificationResult, VerificationTarget,
    WorkspaceInfo, WorkspaceInitOptions, canonical_bytes,
};

#[derive(Clone, Debug, PartialEq, Eq)]
struct RuntimeCounts {
    object_identity: i64,
    session: i64,
    session_runtime: i64,
    session_context_workspace: i64,
    session_focus: i64,
    session_focus_path: i64,
    session_diff: i64,
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
        StoreInitOptions::new("phase3e-session-store").expect("store options"),
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
        session: count_rows(connection, "session"),
        session_runtime: count_rows(connection, "session_runtime"),
        session_context_workspace: count_rows(connection, "session_context_workspace"),
        session_focus: count_rows(connection, "session_focus"),
        session_focus_path: count_rows(connection, "session_focus_path"),
        session_diff: count_rows(connection, "session_diff"),
        changeset: count_rows(connection, "changeset"),
        workstate_commit: count_rows(connection, "workstate_commit"),
        event: count_rows(connection, "event"),
    }
}

fn metadata(agent: &str) -> CanonicalValue {
    CanonicalValue::object(vec![
        (
            "z_notes".to_owned(),
            CanonicalValue::String("canonical order probe".to_owned()),
        ),
        ("agent".to_owned(), CanonicalValue::String(agent.to_owned())),
    ])
    .expect("metadata")
}

fn canonical_json(value: &CanonicalValue) -> String {
    String::from_utf8(canonical_bytes(value).expect("canonical bytes")).expect("utf8")
}

fn event_count(connection: &Connection, session_id: SessionId, event_kind: &str) -> i64 {
    let session_id = session_id.raw_bytes();
    connection
        .query_row(
            "SELECT count(*)
             FROM event
             WHERE session_id = ?1
               AND changeset_id IS NULL
               AND event_kind = ?2",
            params![&session_id[..], event_kind],
            |row| row.get(0),
        )
        .expect("event count")
}

#[test]
fn start_session_persists_active_runtime_without_workstate_commit() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let connection = raw_connection(&path);
    let before = runtime_counts(&connection);
    let metadata = metadata("codex");

    let started = engine
        .start_session(
            SessionStartOptions::new(workspace.workspace_id, workspace.initial_branch_id)
                .expect("session options")
                .with_metadata(metadata.clone())
                .expect("metadata"),
        )
        .expect("start session");

    assert_eq!(started.workspace_id, workspace.workspace_id);
    assert_eq!(started.branch_id, workspace.initial_branch_id);
    assert_eq!(started.state.session_id, started.session_id);
    assert_eq!(started.state.lifecycle_state, SessionLifecycleState::Active);
    assert_eq!(started.state.started_at_us, started.started_at_us);
    assert_eq!(
        started.state.last_activity_at_us,
        Some(started.started_at_us)
    );
    assert_eq!(
        canonical_json(&started.state.metadata),
        canonical_json(&metadata)
    );
    assert_eq!(
        started.state.active_workspace_id,
        Some(workspace.workspace_id)
    );
    assert_eq!(
        started.state.active_branch_id,
        Some(workspace.initial_branch_id)
    );
    assert_eq!(
        started.state.context_workspaces,
        vec![workspace.workspace_id]
    );
    assert_eq!(started.state.focus, None);
    assert_eq!(started.state.session_diff_id, None);

    let after = runtime_counts(&connection);
    assert_eq!(after.object_identity, before.object_identity + 1);
    assert_eq!(after.session, before.session + 1);
    assert_eq!(after.session_runtime, before.session_runtime + 1);
    assert_eq!(
        after.session_context_workspace,
        before.session_context_workspace + 1
    );
    assert_eq!(after.session_focus, before.session_focus);
    assert_eq!(after.session_focus_path, before.session_focus_path);
    assert_eq!(after.session_diff, before.session_diff);
    assert_eq!(after.changeset, before.changeset);
    assert_eq!(after.workstate_commit, before.workstate_commit);
    assert_eq!(after.event, before.event + 1);

    let session_id = started.session_id.raw_bytes();
    let stored_metadata: String = connection
        .query_row(
            "SELECT metadata_json FROM session WHERE session_id = ?1",
            params![&session_id[..]],
            |row| row.get(0),
        )
        .expect("session metadata");
    assert_eq!(stored_metadata, canonical_json(&started.state.metadata));

    let stored_runtime: String = connection
        .query_row(
            "SELECT runtime_json FROM session_runtime WHERE session_id = ?1",
            params![&session_id[..]],
            |row| row.get(0),
        )
        .expect("session runtime");
    assert_eq!(stored_runtime, r#"{"lifecycle_state":"active"}"#);

    assert_eq!(
        event_count(&connection, started.session_id, "session.started"),
        1
    );

    drop(engine);
    let reopened = Engine::open(&path).expect("reopen engine");
    let snapshot = reopened
        .session_snapshot(started.session_id)
        .expect("session snapshot");
    assert_eq!(snapshot, started.state);
}

#[test]
fn start_session_rejects_invalid_inputs_without_runtime_rows() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let other_workspace = engine
        .create_workspace(WorkspaceInitOptions::new("other").expect("workspace options"))
        .expect("create other workspace");
    let connection = raw_connection(&path);
    let before = runtime_counts(&connection);

    let metadata_error =
        SessionStartOptions::new(workspace.workspace_id, workspace.initial_branch_id)
            .expect("session options")
            .with_metadata(CanonicalValue::Array(Vec::new()))
            .expect_err("non-object metadata should fail");
    assert_eq!(metadata_error.code(), ErrorCode::SessionInvalid);
    assert_eq!(metadata_error.category(), ErrorCategory::Runtime);

    let mismatch = engine
        .start_session(
            SessionStartOptions::new(workspace.workspace_id, other_workspace.initial_branch_id)
                .expect("session options"),
        )
        .expect_err("branch workspace mismatch should fail");
    assert_eq!(mismatch.code(), ErrorCode::SessionInvalid);
    assert_eq!(mismatch.category(), ErrorCategory::Runtime);

    let missing_branch = engine
        .start_session(
            SessionStartOptions::new(workspace.workspace_id, BranchId::new_v7())
                .expect("session options"),
        )
        .expect_err("missing branch should fail");
    assert_eq!(missing_branch.code(), ErrorCode::BranchNotFound);

    assert_eq!(runtime_counts(&connection), before);
}

#[test]
fn session_snapshot_rejects_unknown_session() {
    let (_tempdir, path) = store_path();
    let (engine, _workspace) = create_workspace(&path);

    let error = engine
        .session_snapshot(SessionId::new_v7())
        .expect_err("unknown session should fail");
    assert_eq!(error.code(), ErrorCode::SessionNotFound);
    assert_eq!(error.category(), ErrorCategory::Runtime);
}

#[test]
fn session_snapshot_rejects_active_session_missing_active_context_workspace() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let started = engine
        .start_session(
            SessionStartOptions::new(workspace.workspace_id, workspace.initial_branch_id)
                .expect("session options"),
        )
        .expect("start session");
    let connection = raw_connection(&path);
    let session_id = started.session_id.raw_bytes();
    connection
        .execute(
            "DELETE FROM session_context_workspace
             WHERE session_id = ?1",
            params![&session_id[..]],
        )
        .expect("corrupt context workspace");

    let error = engine
        .session_snapshot(started.session_id)
        .expect_err("missing active workspace context should fail");
    assert_eq!(error.code(), ErrorCode::SessionInvalid);
    assert_eq!(error.category(), ErrorCategory::Runtime);
}

#[test]
fn set_and_clear_focus_persist_structured_runtime_without_workstate_commit() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let task = engine
        .create_task(
            TaskCreateOptions::new(
                workspace.initial_branch_id,
                workspace.genesis_commit_id,
                "Focus runtime task",
            )
            .expect("task options"),
        )
        .expect("create task");
    let criterion = engine
        .create_acceptance_criterion(
            AcceptanceCriterionCreateOptions::new(
                workspace.initial_branch_id,
                task.commit_id,
                task.task_entity_id,
                task.task_entity_version_id,
                "AC-focus",
                "The session records structured focus.",
                AcceptanceCriterionClassification::Required,
            )
            .expect("ac options"),
        )
        .expect("create acceptance criterion");
    let verification = engine
        .create_verification(
            VerificationCreateOptions::new(
                workspace.initial_branch_id,
                criterion.commit_id,
                VerificationTarget::AcceptanceCriterion(criterion.acceptance_criterion_entity_id),
                VerificationResult::Passed,
            )
            .expect("verification options"),
        )
        .expect("create verification");
    let started = engine
        .start_session(
            SessionStartOptions::new(workspace.workspace_id, workspace.initial_branch_id)
                .expect("session options"),
        )
        .expect("start session");
    let connection = raw_connection(&path);
    let before = runtime_counts(&connection);
    let focus_path = vec![
        SessionFocusPathEntry::new(task.task_entity_id, None),
        SessionFocusPathEntry::new(
            criterion.acceptance_criterion_entity_id,
            Some(verification.verifies_relation_id),
        ),
    ];

    let focused = engine
        .set_session_focus(
            SessionFocusOptions::new(started.session_id, task.task_entity_id)
                .with_path(focus_path.clone()),
        )
        .expect("set focus");

    assert_eq!(focused.session_id, started.session_id);
    assert!(focused.occurred_at_us >= started.started_at_us);
    let focus = focused.state.focus.expect("focus snapshot");
    assert_eq!(focus.focus_entity_id, task.task_entity_id);
    assert_eq!(focus.path, focus_path);
    assert_eq!(focused.state.lifecycle_state, SessionLifecycleState::Active);
    assert_eq!(
        focused.state.active_workspace_id,
        Some(workspace.workspace_id)
    );
    assert_eq!(
        focused.state.active_branch_id,
        Some(workspace.initial_branch_id)
    );

    let after_focus = runtime_counts(&connection);
    assert_eq!(after_focus.session, before.session);
    assert_eq!(after_focus.session_runtime, before.session_runtime);
    assert_eq!(
        after_focus.session_context_workspace,
        before.session_context_workspace
    );
    assert_eq!(after_focus.session_focus, before.session_focus + 1);
    assert_eq!(
        after_focus.session_focus_path,
        before.session_focus_path + 2
    );
    assert_eq!(after_focus.session_diff, before.session_diff);
    assert_eq!(after_focus.changeset, before.changeset);
    assert_eq!(after_focus.workstate_commit, before.workstate_commit);
    assert_eq!(after_focus.event, before.event + 1);
    assert_eq!(
        event_count(&connection, started.session_id, "session.focus_set"),
        1
    );

    let session_id = started.session_id.raw_bytes();
    let rows = connection
        .prepare(
            "SELECT ordinal, path_entity_id, incoming_relation_id
             FROM session_focus_path
             WHERE session_id = ?1
             ORDER BY ordinal",
        )
        .expect("prepare focus path")
        .query_map(params![&session_id[..]], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, Vec<u8>>(1)?,
                row.get::<_, Option<Vec<u8>>>(2)?,
            ))
        })
        .expect("query focus path")
        .collect::<rusqlite::Result<Vec<_>>>()
        .expect("focus path rows");
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].0, 0);
    assert_eq!(rows[0].1, task.task_entity_id.raw_bytes());
    assert_eq!(rows[0].2, None);
    assert_eq!(rows[1].0, 1);
    assert_eq!(
        rows[1].1,
        criterion.acceptance_criterion_entity_id.raw_bytes()
    );
    assert_eq!(
        rows[1].2.as_ref().expect("relation id"),
        &verification.verifies_relation_id.raw_bytes()
    );

    let cleared = engine
        .clear_session_focus(started.session_id)
        .expect("clear focus");
    assert_eq!(cleared.session_id, started.session_id);
    assert!(cleared.occurred_at_us >= focused.occurred_at_us);
    assert_eq!(cleared.state.focus, None);
    assert_eq!(cleared.state.lifecycle_state, SessionLifecycleState::Active);

    let after_clear = runtime_counts(&connection);
    assert_eq!(after_clear.session, before.session);
    assert_eq!(after_clear.session_runtime, before.session_runtime);
    assert_eq!(
        after_clear.session_context_workspace,
        before.session_context_workspace
    );
    assert_eq!(after_clear.session_focus, before.session_focus);
    assert_eq!(after_clear.session_focus_path, before.session_focus_path);
    assert_eq!(after_clear.session_diff, before.session_diff);
    assert_eq!(after_clear.changeset, before.changeset);
    assert_eq!(after_clear.workstate_commit, before.workstate_commit);
    assert_eq!(after_clear.event, before.event + 2);
    assert_eq!(
        event_count(&connection, started.session_id, "session.focus_cleared"),
        1
    );
}

#[test]
fn set_focus_rejects_absent_entities_and_relations_without_partial_rows() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let task = engine
        .create_task(
            TaskCreateOptions::new(
                workspace.initial_branch_id,
                workspace.genesis_commit_id,
                "Reject invalid focus",
            )
            .expect("task options"),
        )
        .expect("create task");
    let started = engine
        .start_session(
            SessionStartOptions::new(workspace.workspace_id, workspace.initial_branch_id)
                .expect("session options"),
        )
        .expect("start session");
    let connection = raw_connection(&path);
    let before = runtime_counts(&connection);

    let missing_focus = engine
        .set_session_focus(SessionFocusOptions::new(
            started.session_id,
            EntityId::new_v7(),
        ))
        .expect_err("missing focus entity should fail");
    assert_eq!(missing_focus.code(), ErrorCode::SessionInvalid);
    assert_eq!(missing_focus.category(), ErrorCategory::Runtime);

    let missing_path_entity = engine
        .set_session_focus(
            SessionFocusOptions::new(started.session_id, task.task_entity_id)
                .with_path(vec![SessionFocusPathEntry::new(EntityId::new_v7(), None)]),
        )
        .expect_err("missing path entity should fail");
    assert_eq!(missing_path_entity.code(), ErrorCode::SessionInvalid);

    let missing_relation = engine
        .set_session_focus(
            SessionFocusOptions::new(started.session_id, task.task_entity_id).with_path(vec![
                SessionFocusPathEntry::new(task.task_entity_id, Some(RelationId::new_v7())),
            ]),
        )
        .expect_err("missing relation should fail");
    assert_eq!(missing_relation.code(), ErrorCode::SessionInvalid);

    assert_eq!(runtime_counts(&connection), before);
}

#[test]
fn end_session_creates_session_diff_and_cleans_runtime_without_workstate_commit() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let task = engine
        .create_task(
            TaskCreateOptions::new(
                workspace.initial_branch_id,
                workspace.genesis_commit_id,
                "End runtime task",
            )
            .expect("task options"),
        )
        .expect("create task");
    let started = engine
        .start_session(
            SessionStartOptions::new(workspace.workspace_id, workspace.initial_branch_id)
                .expect("session options"),
        )
        .expect("start session");
    engine
        .set_session_focus(SessionFocusOptions::new(
            started.session_id,
            task.task_entity_id,
        ))
        .expect("set focus");
    let connection = raw_connection(&path);
    let before = runtime_counts(&connection);
    let summary = CanonicalValue::object(vec![
        (
            "z_outcome".to_owned(),
            CanonicalValue::String("completed".to_owned()),
        ),
        (
            "notes".to_owned(),
            CanonicalValue::String("runtime boundary closed".to_owned()),
        ),
    ])
    .expect("summary");

    let ended = engine
        .end_session(
            SessionEndOptions::new(started.session_id)
                .expect("end options")
                .with_summary(summary.clone())
                .expect("summary"),
        )
        .expect("end session");

    assert_eq!(ended.session_id, started.session_id);
    assert!(ended.ended_at_us >= started.started_at_us);
    assert_eq!(canonical_json(&ended.summary), canonical_json(&summary));
    assert_eq!(ended.state.lifecycle_state, SessionLifecycleState::Ended);
    assert_eq!(ended.state.active_workspace_id, None);
    assert_eq!(ended.state.active_branch_id, None);
    assert_eq!(ended.state.context_workspaces, Vec::new());
    assert_eq!(ended.state.focus, None);
    assert_eq!(ended.state.last_activity_at_us, None);
    assert_eq!(ended.state.session_diff_id, Some(ended.session_diff_id));

    let after = runtime_counts(&connection);
    assert_eq!(after.object_identity, before.object_identity + 1);
    assert_eq!(after.session, before.session);
    assert_eq!(after.session_runtime, before.session_runtime - 1);
    assert_eq!(
        after.session_context_workspace,
        before.session_context_workspace - 1
    );
    assert_eq!(after.session_focus, before.session_focus - 1);
    assert_eq!(after.session_focus_path, before.session_focus_path);
    assert_eq!(after.session_diff, before.session_diff + 1);
    assert_eq!(after.changeset, before.changeset);
    assert_eq!(after.workstate_commit, before.workstate_commit);
    assert_eq!(after.event, before.event + 1);
    assert_eq!(
        event_count(&connection, started.session_id, "session.ended"),
        1
    );

    let session_id = started.session_id.raw_bytes();
    let stored_summary: String = connection
        .query_row(
            "SELECT summary_json
             FROM session_diff
             WHERE session_id = ?1",
            params![&session_id[..]],
            |row| row.get(0),
        )
        .expect("session diff summary");
    assert_eq!(stored_summary, canonical_json(&summary));

    let diff_snapshot = engine
        .session_diff(ended.session_diff_id)
        .expect("session diff snapshot");
    assert_eq!(diff_snapshot.session_diff_id, ended.session_diff_id);
    assert_eq!(diff_snapshot.session_id, started.session_id);
    assert!(diff_snapshot.created_at_us >= started.started_at_us);
    assert_eq!(
        canonical_json(&diff_snapshot.summary),
        canonical_json(&summary)
    );
    assert_eq!(diff_snapshot.detail_content_digest, None);

    drop(engine);
    let reopened = Engine::open(&path).expect("reopen engine");
    let snapshot = reopened
        .session_snapshot(started.session_id)
        .expect("ended snapshot");
    assert_eq!(snapshot, ended.state);
}

#[test]
fn end_session_rejects_invalid_summary_and_inactive_session_without_partial_rows() {
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

    let summary_error = SessionEndOptions::new(started.session_id)
        .expect("end options")
        .with_summary(CanonicalValue::Array(Vec::new()))
        .expect_err("non-object summary should fail");
    assert_eq!(summary_error.code(), ErrorCode::SessionInvalid);
    assert_eq!(summary_error.category(), ErrorCategory::Runtime);
    assert_eq!(runtime_counts(&connection), before);

    engine
        .end_session(SessionEndOptions::new(started.session_id).expect("end options"))
        .expect("end session");
    let after_end = runtime_counts(&connection);

    let inactive_error = engine
        .end_session(SessionEndOptions::new(started.session_id).expect("end options"))
        .expect_err("ending an inactive session should fail");
    assert_eq!(inactive_error.code(), ErrorCode::SessionInvalid);
    assert_eq!(inactive_error.category(), ErrorCategory::Runtime);
    assert_eq!(runtime_counts(&connection), after_end);
}
