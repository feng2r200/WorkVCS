use rusqlite::{Connection, params};
use std::path::{Path, PathBuf};
use tempfile::TempDir;
use workvcs_core::{
    BranchId, CanonicalValue, CommitId, Digest, Engine, EntityTransitionOptions, EntityVersionId,
    ErrorCategory, ErrorCode, GoalCreateOptions, GoalState, GoalStatus, StoreInitOptions,
    TaskCreateOptions, WorkspaceInfo, WorkspaceInitOptions, canonical_bytes, entity_version_digest,
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
        StoreInitOptions::new("phase3m-goal-store").expect("store options"),
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

fn goal_state_json(description: &str) -> String {
    let state = GoalState::active(description).expect("goal state");
    String::from_utf8(
        canonical_bytes(&state.to_canonical_value().expect("canonical goal state"))
            .expect("canonical bytes"),
    )
    .expect("goal json")
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

fn corrupt_goal_state_shape(connection: &Connection, entity_version_id: EntityVersionId) -> Digest {
    let corrupted = CanonicalValue::object(vec![
        (
            "description".to_owned(),
            CanonicalValue::String("shape".to_owned()),
        ),
        ("plan_refs".to_owned(), CanonicalValue::Array(Vec::new())),
        (
            "status".to_owned(),
            CanonicalValue::String("active".to_owned()),
        ),
        ("subgoals".to_owned(), CanonicalValue::Array(Vec::new())),
        ("terminal_rationale".to_owned(), CanonicalValue::Null),
    ])
    .expect("corrupted goal state");
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
        .expect("corrupt goal state shape");
    digest
}

#[test]
fn create_goal_persists_active_goal_and_reads_it_after_reopen() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);

    let commit = engine
        .create_goal(
            GoalCreateOptions::new(
                workspace.initial_branch_id,
                workspace.genesis_commit_id,
                "Guarantee deterministic state transitions",
            )
            .expect("goal options"),
        )
        .expect("create goal");

    assert_eq!(commit.workspace_id, workspace.workspace_id);
    assert_eq!(commit.branch_id, workspace.initial_branch_id);
    assert_eq!(commit.previous_head_commit_id, workspace.genesis_commit_id);
    assert_eq!(
        commit.state.description,
        "Guarantee deterministic state transitions"
    );
    assert_eq!(commit.state.status, GoalStatus::Active);
    assert_eq!(commit.state.terminal_rationale, None);

    let snapshot = engine
        .goal_at(commit.commit_id, commit.goal_entity_id)
        .expect("goal snapshot");
    assert_eq!(snapshot.workspace_id, workspace.workspace_id);
    assert_eq!(snapshot.commit_id, commit.commit_id);
    assert_eq!(snapshot.goal_entity_id, commit.goal_entity_id);
    assert_eq!(
        snapshot.goal_entity_version_id,
        commit.goal_entity_version_id
    );
    assert_eq!(snapshot.state_digest, commit.goal_state_digest);
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

    let goal_entity_id = commit.goal_entity_id.raw_bytes();
    let goal_entity_version_id = commit.goal_entity_version_id.raw_bytes();
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
            params![&goal_entity_id[..], &goal_entity_version_id[..]],
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
        .expect("goal storage shape");
    assert_eq!(object_kind, "entity");
    assert_eq!(entity_kind, "goal");
    assert_eq!(state_schema_version, 1);
    assert_eq!(
        state_json,
        goal_state_json("Guarantee deterministic state transitions")
    );
    assert_eq!(digest_from_blob(state_digest), commit.goal_state_digest);
    assert_eq!(operation_type, "entity.transition");
    assert_eq!(subject_family, "entity");
    drop(connection);
    drop(engine);

    let reopened = Engine::open(&path).expect("reopen engine");
    let reopened_snapshot = reopened
        .goal_at(commit.commit_id, commit.goal_entity_id)
        .expect("reopened goal snapshot");
    assert_eq!(reopened_snapshot, snapshot);
}

#[test]
fn goal_at_uses_historical_work_state_not_branch_head() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);

    let goal = engine
        .create_goal(
            GoalCreateOptions::new(
                workspace.initial_branch_id,
                workspace.genesis_commit_id,
                "First goal",
            )
            .expect("goal options"),
        )
        .expect("first goal");
    let task = engine
        .create_task(
            TaskCreateOptions::new(workspace.initial_branch_id, goal.commit_id, "Later task")
                .expect("task options"),
        )
        .expect("later task");

    let missing_at_genesis = engine
        .goal_at(workspace.genesis_commit_id, goal.goal_entity_id)
        .expect_err("goal is absent at genesis");
    assert_eq!(missing_at_genesis.code(), ErrorCode::GoalNotFound);
    assert_eq!(missing_at_genesis.category(), ErrorCategory::Goal);

    let goal_snapshot = engine
        .goal_at(goal.commit_id, goal.goal_entity_id)
        .expect("goal historical snapshot");
    assert_eq!(goal_snapshot.state.description, "First goal");
    let goal_from_head = engine
        .goal_at(task.commit_id, goal.goal_entity_id)
        .expect("goal still present at head");
    assert_eq!(goal_from_head.commit_id, task.commit_id);
    assert_eq!(
        goal_from_head.goal_entity_version_id,
        goal_snapshot.goal_entity_version_id
    );
    assert_eq!(goal_from_head.state_digest, goal_snapshot.state_digest);
    assert_eq!(goal_from_head.state, goal_snapshot.state);

    let head_after_read = engine
        .branch_head(workspace.initial_branch_id)
        .expect("branch head after goal_at");
    assert_eq!(head_after_read.head_commit_id, task.commit_id);
}

#[test]
fn goal_create_rejects_invalid_input_without_history_rows() {
    let (_tempdir, path) = store_path();
    let (_engine, workspace) = create_workspace(&path);

    let connection = raw_connection(&path);
    let before = history_counts(&connection);
    assert_eq!(
        branch_head(&connection, workspace.initial_branch_id),
        workspace.genesis_commit_id
    );
    drop(connection);

    let empty_description = GoalCreateOptions::new(
        workspace.initial_branch_id,
        workspace.genesis_commit_id,
        "   ",
    )
    .expect_err("empty goal description");
    assert_eq!(empty_description.code(), ErrorCode::GoalInvalid);
    assert_eq!(empty_description.category(), ErrorCategory::Goal);

    let nul_description = GoalCreateOptions::new(
        workspace.initial_branch_id,
        workspace.genesis_commit_id,
        "bad\0description",
    )
    .expect_err("NUL goal description");
    assert_eq!(nul_description.code(), ErrorCode::GoalInvalid);
    assert_eq!(nul_description.category(), ErrorCategory::Goal);

    let connection = raw_connection(&path);
    assert_eq!(history_counts(&connection), before);
    assert_eq!(
        branch_head(&connection, workspace.initial_branch_id),
        workspace.genesis_commit_id
    );
}

#[test]
fn goal_create_reuses_branch_cas_and_rolls_back_stale_loser() {
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
        .create_goal(
            GoalCreateOptions::new(workspace.initial_branch_id, first_seen_head, "Winner")
                .expect("winner options"),
        )
        .expect("winner goal");

    let connection = raw_connection(&path);
    let before_loser = history_counts(&connection);
    drop(connection);

    let error = second_engine
        .create_goal(
            GoalCreateOptions::new(workspace.initial_branch_id, second_seen_head, "Loser")
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
}

#[test]
fn goal_at_rejects_non_goal_entity_and_invalid_goal_state_shape() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);

    let record = engine
        .commit_entity_transition(
            EntityTransitionOptions::create(
                workspace.initial_branch_id,
                workspace.genesis_commit_id,
                "generic_record",
                record_state("not a goal"),
            )
            .expect("record options"),
        )
        .expect("record transition");
    let non_goal = engine
        .goal_at(record.commit_id, record.entity_id)
        .expect_err("record is not a goal");
    assert_eq!(non_goal.code(), ErrorCode::GoalNotFound);
    assert_eq!(non_goal.category(), ErrorCategory::Goal);

    let goal = engine
        .create_goal(
            GoalCreateOptions::new(workspace.initial_branch_id, record.commit_id, "Shape")
                .expect("goal options"),
        )
        .expect("goal");
    let connection = raw_connection(&path);
    let corrupted_digest = corrupt_goal_state_shape(&connection, goal.goal_entity_version_id);
    drop(connection);

    let replayed = engine.show_at(goal.commit_id).expect("shape still replays");
    assert_eq!(replayed.commit_id, goal.commit_id);
    assert_eq!(
        engine
            .validate_integrity()
            .expect("integrity")
            .checked_commits,
        3
    );

    let invalid_shape = engine
        .goal_at(goal.commit_id, goal.goal_entity_id)
        .expect_err("invalid goal state shape");
    assert_eq!(invalid_shape.code(), ErrorCode::GoalInvalid);
    assert_eq!(invalid_shape.category(), ErrorCategory::Goal);
    assert_ne!(corrupted_digest, goal.goal_state_digest);
}

#[test]
fn generic_entity_transition_rejects_reserved_goal_kind() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);

    let create_error = engine
        .commit_entity_transition(
            EntityTransitionOptions::create(
                workspace.initial_branch_id,
                workspace.genesis_commit_id,
                "goal",
                record_state("bypass create"),
            )
            .expect("generic create options"),
        )
        .expect_err("reserved goal create");
    assert_eq!(create_error.code(), ErrorCode::EntityTransitionInvalid);
    assert_eq!(create_error.category(), ErrorCategory::Mutation);

    let goal = engine
        .create_goal(
            GoalCreateOptions::new(
                workspace.initial_branch_id,
                workspace.genesis_commit_id,
                "Semantic goal",
            )
            .expect("goal options"),
        )
        .expect("semantic goal");
    let update_error = engine
        .commit_entity_transition(
            EntityTransitionOptions::update(
                workspace.initial_branch_id,
                goal.commit_id,
                goal.goal_entity_id,
                goal.goal_entity_version_id,
                record_state("bypass update"),
            )
            .expect("generic update options"),
        )
        .expect_err("reserved goal update");
    assert_eq!(update_error.code(), ErrorCode::EntityTransitionInvalid);
    assert_eq!(update_error.category(), ErrorCategory::Mutation);
}

#[test]
fn goal_state_serializes_only_confirmed_phase3m_shape() {
    let state = GoalState::active("Serialize goal").expect("goal state");
    let value = state.to_canonical_value().expect("canonical value");
    let encoded = String::from_utf8(canonical_bytes(&value).expect("canonical bytes"))
        .expect("canonical json");

    assert_eq!(
        encoded,
        r#"{"description":"Serialize goal","plan_refs":[],"status":"active","subgoals":[],"terminal_rationale":null,"work_refs":[]}"#
    );
    assert_eq!(GoalStatus::Active.as_str(), "active");
    assert_eq!(GoalStatus::Achieved.as_str(), "achieved");
    assert_eq!(GoalStatus::Abandoned.as_str(), "abandoned");
}
