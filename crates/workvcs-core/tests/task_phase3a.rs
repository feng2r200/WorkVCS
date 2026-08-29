use rusqlite::{Connection, params};
use std::path::{Path, PathBuf};
use tempfile::TempDir;
use workvcs_core::{
    BranchId, CanonicalValue, CommitId, Digest, Engine, EntityTransitionOptions, EntityVersionId,
    ErrorCategory, ErrorCode, StoreInitOptions, TaskCreateOptions, TaskState, TaskStatus,
    WorkspaceInfo, WorkspaceInitOptions, canonical_bytes, entity_version_digest,
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
        StoreInitOptions::new("phase3-task-store").expect("store options"),
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

fn record_state(title: &str) -> CanonicalValue {
    CanonicalValue::object(vec![(
        "title".to_owned(),
        CanonicalValue::String(title.to_owned()),
    )])
    .expect("record state")
}

fn task_state_json(description: &str, priority: i64) -> String {
    let state = TaskState::pending(description, priority).expect("task state");
    String::from_utf8(
        canonical_bytes(&state.to_canonical_value().expect("canonical task state"))
            .expect("canonical bytes"),
    )
    .expect("task json")
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

fn corrupt_task_state_shape(connection: &Connection, entity_version_id: EntityVersionId) -> Digest {
    let corrupted = CanonicalValue::object(vec![
        (
            "acceptance_criteria".to_owned(),
            CanonicalValue::Array(Vec::new()),
        ),
        ("child_order".to_owned(), CanonicalValue::Array(Vec::new())),
        (
            "description".to_owned(),
            CanonicalValue::String("shape".to_owned()),
        ),
        ("outcome".to_owned(), CanonicalValue::Null),
        (
            "priority".to_owned(),
            CanonicalValue::safe_integer(0).expect("priority"),
        ),
    ])
    .expect("corrupted task state");
    let state_json =
        String::from_utf8(canonical_bytes(&corrupted).expect("canonical bytes")).expect("json");
    let digest = entity_version_digest(&corrupted).expect("entity digest");
    connection
        .execute(
            "UPDATE entity_version
             SET state_json = ?1,
                 state_digest = ?2
             WHERE entity_version_id = ?3",
            params![
                state_json,
                &digest.as_bytes()[..],
                &entity_version_id.raw_bytes()[..]
            ],
        )
        .expect("corrupt task state shape");
    digest
}

#[test]
fn create_task_persists_pending_task_and_reads_it_after_reopen() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);

    let commit = engine
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

    assert_eq!(commit.workspace_id, workspace.workspace_id);
    assert_eq!(commit.branch_id, workspace.initial_branch_id);
    assert_eq!(commit.previous_head_commit_id, workspace.genesis_commit_id);
    assert_eq!(commit.state.description, "Evaluate SQLite history");
    assert_eq!(commit.state.status, TaskStatus::Pending);
    assert_eq!(commit.state.outcome, None);
    assert_eq!(commit.state.priority, 7);

    let snapshot = engine
        .task_at(commit.commit_id, commit.task_entity_id)
        .expect("task snapshot");
    assert_eq!(snapshot.workspace_id, workspace.workspace_id);
    assert_eq!(snapshot.commit_id, commit.commit_id);
    assert_eq!(snapshot.task_entity_id, commit.task_entity_id);
    assert_eq!(
        snapshot.task_entity_version_id,
        commit.task_entity_version_id
    );
    assert_eq!(snapshot.state_digest, commit.task_state_digest);
    assert_eq!(snapshot.state, commit.state);

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

    let task_entity_id = commit.task_entity_id.raw_bytes();
    let task_entity_version_id = commit.task_entity_version_id.raw_bytes();
    let (
        object_kind,
        entity_kind,
        state_schema_version,
        state_json,
        state_digest,
        operation_type,
        subject_family,
    ) = connection
        .query_row(
            "SELECT object_identity.object_kind,
                    entity.entity_kind,
                    entity_version.state_schema_version,
                    entity_version.state_json,
                    entity_version.state_digest,
                    changeset.operation_type,
                    change_operation.subject_family
             FROM entity
             JOIN object_identity
               ON object_identity.object_id = entity.object_id
             JOIN entity_version
               ON entity_version.entity_id = entity.object_id
             JOIN entity_membership_change
               ON entity_membership_change.after_entity_version_id = entity_version.entity_version_id
             JOIN change_operation
               ON change_operation.operation_id = entity_membership_change.operation_id
             JOIN changeset
               ON changeset.changeset_id = change_operation.changeset_id
             WHERE entity.object_id = ?1
               AND entity_version.entity_version_id = ?2",
            params![&task_entity_id[..], &task_entity_version_id[..]],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, Vec<u8>>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, String>(6)?,
                ))
            },
        )
        .expect("task storage shape");
    assert_eq!(object_kind, "entity");
    assert_eq!(entity_kind, "task");
    assert_eq!(state_schema_version, 1);
    assert_eq!(state_json, task_state_json("Evaluate SQLite history", 7));
    assert_eq!(digest_from_blob(state_digest), commit.task_state_digest);
    assert_eq!(operation_type, "entity.transition");
    assert_eq!(subject_family, "entity");
    drop(connection);
    drop(engine);

    let reopened = Engine::open(&path).expect("reopen engine");
    let reopened_snapshot = reopened
        .task_at(commit.commit_id, commit.task_entity_id)
        .expect("reopened task snapshot");
    assert_eq!(reopened_snapshot, snapshot);
}

#[test]
fn task_at_uses_historical_work_state_not_branch_head() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);

    let first = engine
        .create_task(
            TaskCreateOptions::new(
                workspace.initial_branch_id,
                workspace.genesis_commit_id,
                "First task",
            )
            .expect("first options"),
        )
        .expect("first task");
    let second = engine
        .create_task(
            TaskCreateOptions::new(workspace.initial_branch_id, first.commit_id, "Second task")
                .expect("second options"),
        )
        .expect("second task");

    let missing_at_genesis = engine
        .task_at(workspace.genesis_commit_id, first.task_entity_id)
        .expect_err("task is absent at genesis");
    assert_eq!(missing_at_genesis.code(), ErrorCode::TaskNotFound);
    assert_eq!(missing_at_genesis.category(), ErrorCategory::Task);

    let first_snapshot = engine
        .task_at(first.commit_id, first.task_entity_id)
        .expect("first historical task");
    assert_eq!(first_snapshot.state.description, "First task");
    let first_from_head = engine
        .task_at(second.commit_id, first.task_entity_id)
        .expect("first task still present at head");
    assert_eq!(first_from_head.commit_id, second.commit_id);
    assert_eq!(
        first_from_head.task_entity_version_id,
        first_snapshot.task_entity_version_id
    );
    assert_eq!(first_from_head.state_digest, first_snapshot.state_digest);
    assert_eq!(first_from_head.state, first_snapshot.state);

    let head_after_read = engine
        .branch_head(workspace.initial_branch_id)
        .expect("branch head after task_at");
    assert_eq!(head_after_read.head_commit_id, second.commit_id);
}

#[test]
fn task_create_rejects_invalid_input_without_history_rows() {
    let (_tempdir, path) = store_path();
    let (_engine, workspace) = create_workspace(&path);

    let connection = raw_connection(&path);
    let before = history_counts(&connection);
    assert_eq!(
        branch_head(&connection, workspace.initial_branch_id),
        workspace.genesis_commit_id
    );
    drop(connection);

    let empty = TaskCreateOptions::new(
        workspace.initial_branch_id,
        workspace.genesis_commit_id,
        "   ",
    )
    .expect_err("empty task description");
    assert_eq!(empty.code(), ErrorCode::TaskInvalid);
    assert_eq!(empty.category(), ErrorCategory::Task);

    let unsafe_priority = TaskCreateOptions::new(
        workspace.initial_branch_id,
        workspace.genesis_commit_id,
        "Unsafe priority",
    )
    .expect("task options")
    .with_priority(9_007_199_254_740_992)
    .expect_err("unsafe priority");
    assert_eq!(unsafe_priority.code(), ErrorCode::TaskInvalid);
    assert_eq!(unsafe_priority.category(), ErrorCategory::Task);

    let connection = raw_connection(&path);
    assert_eq!(history_counts(&connection), before);
    assert_eq!(
        branch_head(&connection, workspace.initial_branch_id),
        workspace.genesis_commit_id
    );
}

#[test]
fn task_create_reuses_branch_cas_and_rolls_back_stale_loser() {
    let (_tempdir, path) = store_path();
    let (mut first_engine, workspace) = create_workspace(&path);
    let mut second_engine = Engine::open(&path).expect("second engine");

    let first_seen_head = first_engine
        .branch_head(workspace.initial_branch_id)
        .expect("first branch head")
        .head_commit_id;
    let second_seen_head = second_engine
        .branch_head(workspace.initial_branch_id)
        .expect("second branch head")
        .head_commit_id;
    assert_eq!(first_seen_head, workspace.genesis_commit_id);
    assert_eq!(second_seen_head, workspace.genesis_commit_id);

    let winner = first_engine
        .create_task(
            TaskCreateOptions::new(workspace.initial_branch_id, first_seen_head, "Winner")
                .expect("winner options"),
        )
        .expect("winner task");

    let connection = raw_connection(&path);
    let before_loser = history_counts(&connection);
    drop(connection);

    let error = second_engine
        .create_task(
            TaskCreateOptions::new(workspace.initial_branch_id, second_seen_head, "Loser")
                .expect("loser options"),
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
    drop(connection);

    let report = second_engine.validate_integrity().expect("integrity");
    assert_eq!(report.checked_branches, 1);
    assert_eq!(report.checked_commits, 2);
    assert_eq!(report.checked_events, 2);
}

#[test]
fn task_at_rejects_non_task_entity_and_invalid_task_state_shape() {
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
    let non_task = engine
        .task_at(record.commit_id, record.entity_id)
        .expect_err("record is not a task");
    assert_eq!(non_task.code(), ErrorCode::TaskNotFound);
    assert_eq!(non_task.category(), ErrorCategory::Task);

    let task = engine
        .create_task(
            TaskCreateOptions::new(workspace.initial_branch_id, record.commit_id, "Shape")
                .expect("task options"),
        )
        .expect("task");
    let connection = raw_connection(&path);
    let corrupted_digest = corrupt_task_state_shape(&connection, task.task_entity_version_id);
    drop(connection);

    let replayed = engine.show_at(task.commit_id).expect("shape still replays");
    assert_eq!(replayed.commit_id, task.commit_id);
    assert_eq!(
        engine
            .validate_integrity()
            .expect("integrity")
            .checked_commits,
        3
    );

    let invalid_shape = engine
        .task_at(task.commit_id, task.task_entity_id)
        .expect_err("invalid task state shape");
    assert_eq!(invalid_shape.code(), ErrorCode::TaskInvalid);
    assert_eq!(invalid_shape.category(), ErrorCategory::Task);
    assert_ne!(corrupted_digest, task.task_state_digest);
}

#[test]
fn task_state_serializes_only_confirmed_phase3a_shape() {
    let state = TaskState::pending("Serialize me", 3).expect("task state");
    let value = state.to_canonical_value().expect("canonical value");
    let encoded = String::from_utf8(canonical_bytes(&value).expect("canonical bytes"))
        .expect("canonical json");

    assert_eq!(
        encoded,
        r#"{"acceptance_criteria":[],"child_order":[],"description":"Serialize me","outcome":null,"priority":3,"status":"pending"}"#
    );
    assert_eq!(TaskStatus::Pending.as_str(), "pending");
    assert_eq!(TaskStatus::InProgress.as_str(), "in_progress");
    assert_eq!(TaskStatus::Blocked.as_str(), "blocked");
    assert_eq!(TaskStatus::Done.as_str(), "done");
    assert_eq!(TaskStatus::Failed.as_str(), "failed");
    assert_eq!(TaskStatus::Cancelled.as_str(), "cancelled");
    assert_eq!(TaskStatus::Superseded.as_str(), "superseded");
}
