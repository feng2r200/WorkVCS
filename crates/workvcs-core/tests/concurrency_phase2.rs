use rusqlite::{Connection, params};
use std::path::{Path, PathBuf};
use tempfile::TempDir;
use workvcs_core::{
    BranchId, CanonicalValue, CommitId, Engine, EntityId, EntityTransitionOptions, EntityVersionId,
    ErrorCategory, ErrorCode, HistoryQueryOptions, StoreInitOptions, WorkspaceInfo,
    WorkspaceInitOptions,
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
        StoreInitOptions::new("phase2-concurrency-store").expect("store options"),
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

fn record_state(title: &str, status: &str) -> CanonicalValue {
    CanonicalValue::object(vec![
        ("title".to_owned(), CanonicalValue::String(title.to_owned())),
        (
            "status".to_owned(),
            CanonicalValue::String(status.to_owned()),
        ),
    ])
    .expect("record state")
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

#[test]
fn two_engine_cas_race_allows_one_winner_and_rolls_back_stale_loser() {
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
        .commit_entity_transition(
            EntityTransitionOptions::create(
                workspace.initial_branch_id,
                first_seen_head,
                "record",
                record_state("winner", "open"),
            )
            .expect("winner options"),
        )
        .expect("winner commit");

    let connection = raw_connection(&path);
    let before_loser = history_counts(&connection);
    drop(connection);

    let error = second_engine
        .commit_entity_transition(
            EntityTransitionOptions::create(
                workspace.initial_branch_id,
                second_seen_head,
                "record",
                record_state("loser", "open"),
            )
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

    let history = second_engine
        .history(HistoryQueryOptions::from_branch(
            workspace.initial_branch_id,
        ))
        .expect("history after race");
    assert_eq!(history.entries.len(), 2);
    assert_eq!(history.entries[0].commit_id, winner.commit_id);
    assert_eq!(history.entries[1].commit_id, workspace.genesis_commit_id);

    let integrity = second_engine.validate_integrity().expect("integrity");
    assert_eq!(integrity.checked_branches, 1);
    assert_eq!(integrity.checked_commits, 2);
}

#[test]
fn invalid_update_subject_rolls_back_without_partial_history() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let first = engine
        .commit_entity_transition(
            EntityTransitionOptions::create(
                workspace.initial_branch_id,
                workspace.genesis_commit_id,
                "record",
                record_state("first", "open"),
            )
            .expect("create options"),
        )
        .expect("first transition");

    let connection = raw_connection(&path);
    let before = history_counts(&connection);
    drop(connection);

    let error = engine
        .commit_entity_transition(
            EntityTransitionOptions::update(
                workspace.initial_branch_id,
                first.commit_id,
                EntityId::new_v7(),
                EntityVersionId::new_v7(),
                record_state("invalid", "done"),
            )
            .expect("invalid update options"),
        )
        .expect_err("invalid update subject");
    assert_eq!(error.code(), ErrorCode::EntityTransitionInvalid);
    assert_eq!(error.category(), ErrorCategory::Mutation);

    let connection = raw_connection(&path);
    assert_eq!(history_counts(&connection), before);
    assert_eq!(
        branch_head(&connection, workspace.initial_branch_id),
        first.commit_id
    );
}

#[test]
fn invalid_semantic_input_is_rejected_without_history_rows() {
    let (_tempdir, path) = store_path();
    let (_engine, workspace) = create_workspace(&path);

    let connection = raw_connection(&path);
    let before = history_counts(&connection);
    assert_eq!(
        branch_head(&connection, workspace.initial_branch_id),
        workspace.genesis_commit_id
    );
    drop(connection);

    let error = EntityTransitionOptions::create(
        workspace.initial_branch_id,
        workspace.genesis_commit_id,
        " record ",
        record_state("invalid", "open"),
    )
    .expect_err("invalid semantic input");
    assert_eq!(error.code(), ErrorCode::EntityTransitionInvalid);
    assert_eq!(error.category(), ErrorCategory::Mutation);

    let connection = raw_connection(&path);
    assert_eq!(history_counts(&connection), before);
    assert_eq!(
        branch_head(&connection, workspace.initial_branch_id),
        workspace.genesis_commit_id
    );
}

#[test]
fn corrupted_expected_head_replay_fails_before_mutation_rows() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let first = engine
        .commit_entity_transition(
            EntityTransitionOptions::create(
                workspace.initial_branch_id,
                workspace.genesis_commit_id,
                "record",
                record_state("first", "open"),
            )
            .expect("create options"),
        )
        .expect("first transition");

    let connection = raw_connection(&path);
    connection
        .execute(
            "UPDATE workstate_commit
             SET state_digest = ?1
             WHERE commit_id = ?2",
            params![&[4_u8; 32][..], &first.commit_id.raw_bytes()[..]],
        )
        .expect("corrupt expected head digest");
    let before = history_counts(&connection);
    drop(connection);

    let error = engine
        .commit_entity_transition(
            EntityTransitionOptions::update(
                workspace.initial_branch_id,
                first.commit_id,
                first.entity_id,
                first.entity_version_id,
                record_state("second", "done"),
            )
            .expect("update options"),
        )
        .expect_err("corrupted expected head");
    assert_eq!(error.code(), ErrorCode::ReplayInvalid);
    assert_eq!(error.category(), ErrorCategory::Replay);

    let connection = raw_connection(&path);
    assert_eq!(history_counts(&connection), before);
    assert_eq!(
        branch_head(&connection, workspace.initial_branch_id),
        first.commit_id
    );

    let integrity_error = engine
        .validate_integrity()
        .expect_err("integrity catches corruption");
    assert_eq!(integrity_error.code(), ErrorCode::IntegrityInvalid);
    assert_eq!(integrity_error.category(), ErrorCategory::Integrity);
}
