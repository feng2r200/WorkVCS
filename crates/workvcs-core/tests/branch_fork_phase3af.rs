use rusqlite::{Connection, params};
use std::path::{Path, PathBuf};
use tempfile::TempDir;
use workvcs_core::{
    BranchForkOptions, BranchId, CanonicalValue, CommitId, Engine, EntityTransitionCommit,
    EntityTransitionOptions, ErrorCategory, ErrorCode, HistoryQueryOptions, StoreInitOptions,
    WorkspaceInfo, WorkspaceInitOptions,
};

fn store_path() -> (TempDir, PathBuf) {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let path = tempdir.path().join("workvcs.sqlite");
    (tempdir, path)
}

fn init_engine(path: &Path) -> Engine {
    Engine::init(
        path,
        StoreInitOptions::new("phase3af-branch-store").expect("store options"),
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

fn create_record(
    engine: &mut Engine,
    branch_id: BranchId,
    head_commit_id: CommitId,
    title: &str,
) -> EntityTransitionCommit {
    engine
        .commit_entity_transition(
            EntityTransitionOptions::create(
                branch_id,
                head_commit_id,
                "generic_record",
                record_state(title, "open"),
            )
            .expect("create options"),
        )
        .expect("create record")
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

#[test]
fn fork_from_branch_is_o1_ref_and_source_moves_independently() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);

    let first = create_record(
        &mut engine,
        workspace.initial_branch_id,
        workspace.genesis_commit_id,
        "fork point",
    );

    let connection = raw_connection(&path);
    let before_branch_count = count_rows(&connection, "branch");
    let before_commit_count = count_rows(&connection, "workstate_commit");
    let before_changeset_count = count_rows(&connection, "changeset");
    let before_event_count = count_rows(&connection, "event");
    drop(connection);

    let fork = engine
        .fork_branch(
            BranchForkOptions::from_branch(workspace.initial_branch_id, "experiment")
                .expect("fork options"),
        )
        .expect("fork branch");

    assert_eq!(fork.workspace_id, workspace.workspace_id);
    assert_eq!(fork.name, "experiment");
    assert_eq!(fork.source_branch_id, Some(workspace.initial_branch_id));
    assert_eq!(fork.head_commit_id, first.commit_id);
    assert_eq!(fork.state_digest, first.work_state_digest);

    let source_head = engine
        .branch_head(workspace.initial_branch_id)
        .expect("source branch head");
    let fork_head = engine
        .branch_head(fork.branch_id)
        .expect("fork branch head");
    assert_eq!(source_head.head_commit_id, first.commit_id);
    assert_eq!(fork_head.head_commit_id, first.commit_id);
    assert_eq!(fork_head.state_digest, first.work_state_digest);

    let connection = raw_connection(&path);
    assert_eq!(count_rows(&connection, "branch"), before_branch_count + 1);
    assert_eq!(
        count_rows(&connection, "workstate_commit"),
        before_commit_count
    );
    assert_eq!(count_rows(&connection, "changeset"), before_changeset_count);
    assert_eq!(count_rows(&connection, "event"), before_event_count + 1);
    let event = connection
        .query_row(
            "SELECT event_kind, changeset_id, payload_json
             FROM event
             WHERE event_id = ?1",
            params![&fork.event_id.raw_bytes()[..]],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, Option<Vec<u8>>>(1)?,
                    row.get::<_, String>(2)?,
                ))
            },
        )
        .expect("branch event");
    assert_eq!(event.0, "branch.forked");
    assert!(event.1.is_none());
    assert_eq!(
        event.2,
        format!(
            "{{\"branch_id\":\"{}\",\"head_commit_id\":\"{}\",\"name\":\"experiment\",\"source_branch_id\":\"{}\",\"workspace_id\":\"{}\"}}",
            fork.branch_id, first.commit_id, workspace.initial_branch_id, workspace.workspace_id
        )
    );
    drop(connection);

    let second = create_record(
        &mut engine,
        workspace.initial_branch_id,
        first.commit_id,
        "source branch later",
    );
    assert_eq!(
        engine
            .branch_head(workspace.initial_branch_id)
            .expect("advanced source")
            .head_commit_id,
        second.commit_id
    );
    assert_eq!(
        engine
            .branch_head(fork.branch_id)
            .expect("fork stays at fork point")
            .head_commit_id,
        first.commit_id
    );
}

#[test]
fn fork_from_commit_starts_history_at_selected_historical_state() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let first = create_record(
        &mut engine,
        workspace.initial_branch_id,
        workspace.genesis_commit_id,
        "first",
    );
    let second = create_record(
        &mut engine,
        workspace.initial_branch_id,
        first.commit_id,
        "second",
    );

    let fork = engine
        .fork_branch(
            BranchForkOptions::from_commit(first.commit_id, "historical").expect("fork options"),
        )
        .expect("fork from commit");
    assert_eq!(fork.source_branch_id, None);
    assert_eq!(fork.head_commit_id, first.commit_id);
    assert_eq!(
        engine
            .branch_head(workspace.initial_branch_id)
            .expect("source head")
            .head_commit_id,
        second.commit_id
    );

    let history = engine
        .history(
            HistoryQueryOptions::from_branch(fork.branch_id)
                .with_limit(2)
                .expect("limit"),
        )
        .expect("fork history");
    assert_eq!(history.start_commit_id, first.commit_id);
    assert_eq!(history.entries.len(), 2);
    assert_eq!(history.entries[0].commit_id, first.commit_id);
    assert_eq!(history.entries[1].commit_id, workspace.genesis_commit_id);
}

#[test]
fn fork_rejects_duplicate_names_invalid_names_and_unknown_sources() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);

    let duplicate = engine
        .fork_branch(
            BranchForkOptions::from_branch(
                workspace.initial_branch_id,
                workspace.initial_branch_name.clone(),
            )
            .expect("duplicate name options"),
        )
        .expect_err("duplicate branch name");
    assert_eq!(duplicate.code(), ErrorCode::WorkspaceInvalid);
    assert_eq!(duplicate.category(), ErrorCategory::Workspace);

    let invalid = BranchForkOptions::from_branch(workspace.initial_branch_id, " invalid")
        .expect_err("invalid branch name");
    assert_eq!(invalid.code(), ErrorCode::WorkspaceInvalid);

    let unknown_branch = engine
        .fork_branch(
            BranchForkOptions::from_branch(BranchId::new_v7(), "unknown-branch")
                .expect("unknown branch options"),
        )
        .expect_err("unknown branch");
    assert_eq!(unknown_branch.code(), ErrorCode::BranchNotFound);

    let unknown_commit = engine
        .fork_branch(
            BranchForkOptions::from_commit(CommitId::new_v7(), "unknown-commit")
                .expect("unknown commit options"),
        )
        .expect_err("unknown commit");
    assert_eq!(unknown_commit.code(), ErrorCode::CommitNotFound);
}
