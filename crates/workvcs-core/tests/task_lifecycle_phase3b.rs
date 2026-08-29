use rusqlite::{Connection, params};
use std::path::{Path, PathBuf};
use tempfile::TempDir;
use workvcs_core::{
    BranchId, CanonicalValue, CommitId, Digest, Engine, EntityTransitionOptions, EntityVersionId,
    ErrorCategory, ErrorCode, StoreInitOptions, TaskCreateOptions, TaskState, TaskStatus,
    TaskTransitionOptions, WorkspaceInfo, WorkspaceInitOptions, canonical_bytes,
};

#[derive(Clone, Debug, PartialEq, Eq)]
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

fn store_path() -> (TempDir, PathBuf) {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let path = tempdir.path().join("workvcs.sqlite");
    (tempdir, path)
}

fn init_engine(path: &Path) -> Engine {
    Engine::init(
        path,
        StoreInitOptions::new("phase3b-task-store").expect("store options"),
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

fn branch_head(connection: &Connection, branch_id: BranchId) -> CommitId {
    let branch_id = branch_id.raw_bytes();
    let bytes = connection
        .query_row(
            "SELECT head_commit_id FROM branch WHERE branch_id = ?1",
            params![&branch_id[..]],
            |row| row.get::<_, Vec<u8>>(0),
        )
        .expect("branch head");
    CommitId::from_bytes(bytes.try_into().expect("commit id bytes")).expect("commit id")
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
            row.get(0)
        })
        .expect("count rows")
}

fn digest_from_blob(bytes: Vec<u8>) -> Digest {
    Digest::from_bytes(bytes.try_into().expect("digest bytes"))
}

fn record_state(title: &str) -> CanonicalValue {
    CanonicalValue::object(vec![(
        "title".to_owned(),
        CanonicalValue::String(title.to_owned()),
    )])
    .expect("record state")
}

fn rationale(reason: &str) -> CanonicalValue {
    CanonicalValue::object(vec![(
        "reason".to_owned(),
        CanonicalValue::String(reason.to_owned()),
    )])
    .expect("rationale")
}

fn task_state_json(
    description: &str,
    status: TaskStatus,
    outcome: Option<&str>,
    priority: i64,
) -> String {
    let state = TaskState {
        description: description.to_owned(),
        status,
        outcome: outcome.map(str::to_owned),
        priority,
        acceptance_criteria: Vec::new(),
    };
    String::from_utf8(
        canonical_bytes(&state.to_canonical_value().expect("canonical task state"))
            .expect("canonical bytes"),
    )
    .expect("task json")
}

fn canonical_json(value: &CanonicalValue) -> String {
    String::from_utf8(canonical_bytes(value).expect("canonical bytes")).expect("json")
}

#[test]
fn task_transition_updates_status_and_outcome_with_historical_snapshots() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);

    let created = engine
        .create_task(
            TaskCreateOptions::new(
                workspace.initial_branch_id,
                workspace.genesis_commit_id,
                "Evaluate SQLite history",
            )
            .expect("task options")
            .with_priority(7)
            .expect("priority"),
        )
        .expect("create task");
    let started = engine
        .transition_task(
            TaskTransitionOptions::new(
                workspace.initial_branch_id,
                created.commit_id,
                created.task_entity_id,
                created.task_entity_version_id,
                TaskStatus::InProgress,
            )
            .expect("start options")
            .with_rationale(rationale("start execution")),
        )
        .expect("start task");
    let completed = engine
        .transition_task(
            TaskTransitionOptions::new(
                workspace.initial_branch_id,
                started.commit_id,
                started.task_entity_id,
                started.task_entity_version_id,
                TaskStatus::Done,
            )
            .expect("complete options")
            .with_outcome("accepted")
            .expect("outcome")
            .with_rationale(rationale("complete execution")),
        )
        .expect("complete task");

    assert_eq!(
        started.previous_task_entity_version_id,
        created.task_entity_version_id
    );
    assert_eq!(started.state.description, "Evaluate SQLite history");
    assert_eq!(started.state.status, TaskStatus::InProgress);
    assert_eq!(started.state.outcome, None);
    assert_eq!(started.state.priority, 7);
    assert_eq!(
        completed.previous_task_entity_version_id,
        started.task_entity_version_id
    );
    assert_eq!(completed.previous_state, started.state);
    assert_eq!(completed.state.status, TaskStatus::Done);
    assert_eq!(completed.state.outcome.as_deref(), Some("accepted"));
    assert_eq!(completed.state.priority, 7);

    let created_snapshot = engine
        .task_at(created.commit_id, created.task_entity_id)
        .expect("created snapshot");
    let started_snapshot = engine
        .task_at(started.commit_id, created.task_entity_id)
        .expect("started snapshot");
    let completed_snapshot = engine
        .task_at(completed.commit_id, created.task_entity_id)
        .expect("completed snapshot");

    assert_eq!(created_snapshot.state.status, TaskStatus::Pending);
    assert_eq!(created_snapshot.state.outcome, None);
    assert_eq!(started_snapshot.state.status, TaskStatus::InProgress);
    assert_eq!(started_snapshot.state.outcome, None);
    assert_eq!(
        completed_snapshot.task_entity_version_id,
        completed.task_entity_version_id
    );
    assert_eq!(completed_snapshot.state, completed.state);

    let connection = raw_connection(&path);
    assert_eq!(
        history_counts(&connection),
        HistoryCounts {
            object_identity: 1,
            entity: 1,
            entity_version: 3,
            changeset: 4,
            change_operation: 3,
            entity_membership_change: 3,
            workstate_commit: 4,
            commit_parent: 3,
            event: 4,
        }
    );
    assert_eq!(
        branch_head(&connection, workspace.initial_branch_id),
        completed.commit_id
    );
    drop(connection);
    drop(engine);

    let reopened = Engine::open(&path).expect("reopen engine");
    let reopened_snapshot = reopened
        .task_at(completed.commit_id, created.task_entity_id)
        .expect("reopened completed snapshot");
    assert_eq!(reopened_snapshot, completed_snapshot);
}

#[test]
fn task_transition_uses_entity_transition_storage_shape() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);

    let created = engine
        .create_task(
            TaskCreateOptions::new(
                workspace.initial_branch_id,
                workspace.genesis_commit_id,
                "Preserve task shape",
            )
            .expect("task options")
            .with_priority(5)
            .expect("priority"),
        )
        .expect("create task");
    let blocked = engine
        .transition_task(
            TaskTransitionOptions::new(
                workspace.initial_branch_id,
                created.commit_id,
                created.task_entity_id,
                created.task_entity_version_id,
                TaskStatus::Blocked,
            )
            .expect("block options")
            .with_outcome("waiting for decision")
            .expect("outcome")
            .with_rationale(rationale("block execution")),
        )
        .expect("block task");

    let connection = raw_connection(&path);
    let task_entity_version_id = blocked.task_entity_version_id.raw_bytes();
    let (
        entity_kind,
        state_schema_version,
        state_json,
        state_digest,
        operation_type,
        subject_family,
        before_entity_version_id,
        after_entity_version_id,
        field_delta_json,
        rationale_json,
    ) = connection
        .query_row(
            "SELECT entity.entity_kind,
                    entity_version.state_schema_version,
                    entity_version.state_json,
                    entity_version.state_digest,
                    changeset.operation_type,
                    change_operation.subject_family,
                    entity_membership_change.before_entity_version_id,
                    entity_membership_change.after_entity_version_id,
                    entity_membership_change.field_delta_json,
                    changeset.rationale_json
             FROM entity_version
             JOIN entity
               ON entity.object_id = entity_version.entity_id
             JOIN entity_membership_change
               ON entity_membership_change.after_entity_version_id = entity_version.entity_version_id
             JOIN change_operation
               ON change_operation.operation_id = entity_membership_change.operation_id
             JOIN changeset
               ON changeset.changeset_id = change_operation.changeset_id
             WHERE entity_version.entity_version_id = ?1",
            params![&task_entity_version_id[..]],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, Vec<u8>>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, Option<Vec<u8>>>(6)?,
                    row.get::<_, Vec<u8>>(7)?,
                    row.get::<_, String>(8)?,
                    row.get::<_, String>(9)?,
                ))
            },
        )
        .expect("transition storage shape");

    assert_eq!(entity_kind, "task");
    assert_eq!(state_schema_version, 1);
    assert_eq!(
        state_json,
        task_state_json(
            "Preserve task shape",
            TaskStatus::Blocked,
            Some("waiting for decision"),
            5,
        )
    );
    assert_eq!(digest_from_blob(state_digest), blocked.task_state_digest);
    assert_eq!(operation_type, "entity.transition");
    assert_eq!(subject_family, "entity");
    assert_eq!(
        before_entity_version_id.expect("before version"),
        created.task_entity_version_id.raw_bytes()
    );
    assert_eq!(
        after_entity_version_id,
        blocked.task_entity_version_id.raw_bytes()
    );
    assert_eq!(field_delta_json, "{}");
    assert_eq!(
        rationale_json,
        canonical_json(&rationale("block execution"))
    );
}

#[test]
fn task_transition_reuses_branch_cas_and_rolls_back_stale_loser() {
    let (_tempdir, path) = store_path();
    let (mut first_engine, workspace) = create_workspace(&path);
    let mut second_engine = Engine::open(&path).expect("second engine");

    let created = first_engine
        .create_task(
            TaskCreateOptions::new(
                workspace.initial_branch_id,
                workspace.genesis_commit_id,
                "Race task",
            )
            .expect("task options"),
        )
        .expect("create task");
    let first_seen_head = first_engine
        .branch_head(workspace.initial_branch_id)
        .expect("first branch head")
        .head_commit_id;
    let second_seen_head = second_engine
        .branch_head(workspace.initial_branch_id)
        .expect("second branch head")
        .head_commit_id;
    assert_eq!(first_seen_head, created.commit_id);
    assert_eq!(second_seen_head, created.commit_id);

    let winner = first_engine
        .transition_task(
            TaskTransitionOptions::new(
                workspace.initial_branch_id,
                first_seen_head,
                created.task_entity_id,
                created.task_entity_version_id,
                TaskStatus::InProgress,
            )
            .expect("winner options"),
        )
        .expect("winner transition");

    let connection = raw_connection(&path);
    let before_loser = history_counts(&connection);
    drop(connection);

    let error = second_engine
        .transition_task(
            TaskTransitionOptions::new(
                workspace.initial_branch_id,
                second_seen_head,
                created.task_entity_id,
                created.task_entity_version_id,
                TaskStatus::Done,
            )
            .expect("loser options")
            .with_outcome("stale")
            .expect("outcome"),
        )
        .expect_err("stale loser");
    assert_eq!(error.code(), ErrorCode::BranchHeadConflict);
    assert_eq!(error.category(), ErrorCategory::Mutation);
    assert!(error.retryable());

    let connection = raw_connection(&path);
    assert_eq!(history_counts(&connection), before_loser);
    assert_eq!(
        branch_head(&connection, workspace.initial_branch_id),
        winner.commit_id
    );
}

#[test]
fn task_transition_rejects_wrong_expected_task_version_without_history_rows() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);

    let created = engine
        .create_task(
            TaskCreateOptions::new(
                workspace.initial_branch_id,
                workspace.genesis_commit_id,
                "Wrong version",
            )
            .expect("task options"),
        )
        .expect("create task");
    let connection = raw_connection(&path);
    let before = history_counts(&connection);
    drop(connection);

    let wrong_version = EntityVersionId::new_v7();
    let error = engine
        .transition_task(
            TaskTransitionOptions::new(
                workspace.initial_branch_id,
                created.commit_id,
                created.task_entity_id,
                wrong_version,
                TaskStatus::InProgress,
            )
            .expect("transition options"),
        )
        .expect_err("wrong task version");
    assert_eq!(error.code(), ErrorCode::TaskInvalid);
    assert_eq!(error.category(), ErrorCategory::Task);

    let connection = raw_connection(&path);
    assert_eq!(history_counts(&connection), before);
    assert_eq!(
        branch_head(&connection, workspace.initial_branch_id),
        created.commit_id
    );
}

#[test]
fn task_transition_rejects_non_task_entity_without_history_rows() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);

    let record = engine
        .commit_entity_transition(
            EntityTransitionOptions::create(
                workspace.initial_branch_id,
                workspace.genesis_commit_id,
                "generic_record",
                record_state("not a task"),
            )
            .expect("record options"),
        )
        .expect("record transition");
    let connection = raw_connection(&path);
    let before = history_counts(&connection);
    drop(connection);

    let error = engine
        .transition_task(
            TaskTransitionOptions::new(
                workspace.initial_branch_id,
                record.commit_id,
                record.entity_id,
                record.entity_version_id,
                TaskStatus::InProgress,
            )
            .expect("transition options"),
        )
        .expect_err("record is not a task");
    assert_eq!(error.code(), ErrorCode::TaskNotFound);
    assert_eq!(error.category(), ErrorCategory::Task);

    let connection = raw_connection(&path);
    assert_eq!(history_counts(&connection), before);
    assert_eq!(
        branch_head(&connection, workspace.initial_branch_id),
        record.commit_id
    );
}

#[test]
fn task_transition_enforces_terminal_reentry_and_superseded_boundaries() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);

    let created = engine
        .create_task(
            TaskCreateOptions::new(
                workspace.initial_branch_id,
                workspace.genesis_commit_id,
                "Terminal task",
            )
            .expect("task options"),
        )
        .expect("create task");
    let done = engine
        .transition_task(
            TaskTransitionOptions::new(
                workspace.initial_branch_id,
                created.commit_id,
                created.task_entity_id,
                created.task_entity_version_id,
                TaskStatus::Done,
            )
            .expect("done options")
            .with_outcome("complete")
            .expect("outcome"),
        )
        .expect("complete task");

    let connection = raw_connection(&path);
    let before_invalid = history_counts(&connection);
    drop(connection);

    let missing_rationale = engine
        .transition_task(
            TaskTransitionOptions::new(
                workspace.initial_branch_id,
                done.commit_id,
                done.task_entity_id,
                done.task_entity_version_id,
                TaskStatus::InProgress,
            )
            .expect("reopen options"),
        )
        .expect_err("missing reentry rationale");
    assert_eq!(missing_rationale.code(), ErrorCode::TaskInvalid);
    assert_eq!(missing_rationale.category(), ErrorCategory::Task);

    let terminal_to_terminal = engine
        .transition_task(
            TaskTransitionOptions::new(
                workspace.initial_branch_id,
                done.commit_id,
                done.task_entity_id,
                done.task_entity_version_id,
                TaskStatus::Failed,
            )
            .expect("terminal options")
            .with_rationale(rationale("retry as failure")),
        )
        .expect_err("terminal to terminal");
    assert_eq!(terminal_to_terminal.code(), ErrorCode::TaskInvalid);
    assert_eq!(terminal_to_terminal.category(), ErrorCategory::Task);

    let superseded = TaskTransitionOptions::new(
        workspace.initial_branch_id,
        done.commit_id,
        done.task_entity_id,
        done.task_entity_version_id,
        TaskStatus::Superseded,
    )
    .expect_err("superseded transition is deferred");
    assert_eq!(superseded.code(), ErrorCode::TaskInvalid);
    assert_eq!(superseded.category(), ErrorCategory::Task);

    let connection = raw_connection(&path);
    assert_eq!(history_counts(&connection), before_invalid);
    assert_eq!(
        branch_head(&connection, workspace.initial_branch_id),
        done.commit_id
    );
    drop(connection);

    let reopened = engine
        .transition_task(
            TaskTransitionOptions::new(
                workspace.initial_branch_id,
                done.commit_id,
                done.task_entity_id,
                done.task_entity_version_id,
                TaskStatus::InProgress,
            )
            .expect("valid reopen options")
            .with_rationale(rationale("retry after review")),
        )
        .expect("valid reopen");
    assert_eq!(reopened.state.status, TaskStatus::InProgress);
    assert_eq!(reopened.state.outcome.as_deref(), Some("complete"));
}

#[test]
fn task_transition_requires_rationale_when_entering_cancelled() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);

    let created = engine
        .create_task(
            TaskCreateOptions::new(
                workspace.initial_branch_id,
                workspace.genesis_commit_id,
                "Cancel active work",
            )
            .expect("task options"),
        )
        .expect("create task");
    let started = engine
        .transition_task(
            TaskTransitionOptions::new(
                workspace.initial_branch_id,
                created.commit_id,
                created.task_entity_id,
                created.task_entity_version_id,
                TaskStatus::InProgress,
            )
            .expect("start options"),
        )
        .expect("start task");

    let connection = raw_connection(&path);
    let before_cancel = history_counts(&connection);
    drop(connection);

    let missing_rationale = engine
        .transition_task(
            TaskTransitionOptions::new(
                workspace.initial_branch_id,
                started.commit_id,
                started.task_entity_id,
                started.task_entity_version_id,
                TaskStatus::Cancelled,
            )
            .expect("cancel options"),
        )
        .expect_err("cancelled requires rationale");
    assert_eq!(missing_rationale.code(), ErrorCode::TaskInvalid);
    assert_eq!(missing_rationale.category(), ErrorCategory::Task);

    let connection = raw_connection(&path);
    assert_eq!(history_counts(&connection), before_cancel);
    assert_eq!(
        branch_head(&connection, workspace.initial_branch_id),
        started.commit_id
    );
    drop(connection);

    let cancelled = engine
        .transition_task(
            TaskTransitionOptions::new(
                workspace.initial_branch_id,
                started.commit_id,
                started.task_entity_id,
                started.task_entity_version_id,
                TaskStatus::Cancelled,
            )
            .expect("valid cancel options")
            .with_outcome("cancelled by operator")
            .expect("outcome")
            .with_rationale(rationale("explicit cancellation")),
        )
        .expect("cancel task");
    assert_eq!(cancelled.state.status, TaskStatus::Cancelled);
    assert_eq!(
        cancelled.state.outcome.as_deref(),
        Some("cancelled by operator")
    );
}

#[test]
fn task_transition_rejects_invalid_outcome_and_noop_without_history_rows() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);

    let created = engine
        .create_task(
            TaskCreateOptions::new(
                workspace.initial_branch_id,
                workspace.genesis_commit_id,
                "Invalid outcome",
            )
            .expect("task options"),
        )
        .expect("create task");
    let connection = raw_connection(&path);
    let before = history_counts(&connection);
    drop(connection);

    let invalid_outcome = TaskTransitionOptions::new(
        workspace.initial_branch_id,
        created.commit_id,
        created.task_entity_id,
        created.task_entity_version_id,
        TaskStatus::Blocked,
    )
    .expect("transition options")
    .with_outcome("bad\0outcome")
    .expect_err("invalid outcome");
    assert_eq!(invalid_outcome.code(), ErrorCode::TaskInvalid);
    assert_eq!(invalid_outcome.category(), ErrorCategory::Task);

    let noop = engine
        .transition_task(
            TaskTransitionOptions::new(
                workspace.initial_branch_id,
                created.commit_id,
                created.task_entity_id,
                created.task_entity_version_id,
                TaskStatus::Pending,
            )
            .expect("noop options"),
        )
        .expect_err("noop transition");
    assert_eq!(noop.code(), ErrorCode::TaskInvalid);
    assert_eq!(noop.category(), ErrorCategory::Task);

    let connection = raw_connection(&path);
    assert_eq!(history_counts(&connection), before);
    assert_eq!(
        branch_head(&connection, workspace.initial_branch_id),
        created.commit_id
    );
}

#[test]
fn task_transition_can_clear_outcome_without_changing_other_task_state() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);

    let created = engine
        .create_task(
            TaskCreateOptions::new(
                workspace.initial_branch_id,
                workspace.genesis_commit_id,
                "Clear outcome",
            )
            .expect("task options")
            .with_priority(3)
            .expect("priority"),
        )
        .expect("create task");
    let blocked = engine
        .transition_task(
            TaskTransitionOptions::new(
                workspace.initial_branch_id,
                created.commit_id,
                created.task_entity_id,
                created.task_entity_version_id,
                TaskStatus::Blocked,
            )
            .expect("block options")
            .with_outcome("waiting")
            .expect("outcome"),
        )
        .expect("block task");
    let unblocked = engine
        .transition_task(
            TaskTransitionOptions::new(
                workspace.initial_branch_id,
                blocked.commit_id,
                blocked.task_entity_id,
                blocked.task_entity_version_id,
                TaskStatus::InProgress,
            )
            .expect("unblock options")
            .clear_outcome(),
        )
        .expect("unblock task");

    assert_eq!(unblocked.state.description, "Clear outcome");
    assert_eq!(unblocked.state.status, TaskStatus::InProgress);
    assert_eq!(unblocked.state.outcome, None);
    assert_eq!(unblocked.state.priority, 3);
}
