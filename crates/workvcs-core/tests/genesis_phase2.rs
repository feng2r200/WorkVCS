use rusqlite::{Connection, Params, params};
use std::path::{Path, PathBuf};
use tempfile::TempDir;
use workvcs_core::{
    Engine, ErrorCode, StoreInitOptions, WorkState, WorkspaceId, WorkspaceInfo,
    WorkspaceInitOptions, work_state_mapping_digest,
};

fn store_path() -> (TempDir, PathBuf) {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let path = tempdir.path().join("workvcs.sqlite");
    (tempdir, path)
}

fn init_engine(path: &Path) -> Engine {
    Engine::init(
        path,
        StoreInitOptions::new("phase2-genesis-store").expect("store options"),
    )
    .expect("init engine")
}

fn raw_connection(path: &Path) -> Connection {
    let connection = Connection::open(path).expect("raw connection");
    connection
        .pragma_update(None, "foreign_keys", "ON")
        .expect("enable raw foreign keys");
    connection
}

fn count<P: Params>(connection: &Connection, sql: &str, params: P) -> i64 {
    connection
        .query_row(sql, params, |row| row.get(0))
        .expect("count query")
}

fn assert_no_foreign_key_violations(connection: &Connection) {
    let mut statement = connection
        .prepare("PRAGMA foreign_key_check")
        .expect("foreign_key_check statement");
    let mut rows = statement.query([]).expect("foreign_key_check rows");
    assert!(
        rows.next().expect("foreign_key_check result").is_none(),
        "foreign_key_check returned at least one violation"
    );
}

fn assert_genesis_shape(connection: &Connection, info: &WorkspaceInfo) {
    let workspace_id = info.workspace_id.raw_bytes();
    let commit_id = info.genesis_commit_id.raw_bytes();
    let changeset_id = info.genesis_changeset_id.raw_bytes();
    let branch_id = info.initial_branch_id.raw_bytes();

    assert_eq!(count(connection, "SELECT count(*) FROM workspace", []), 1);
    assert_eq!(count(connection, "SELECT count(*) FROM changeset", []), 1);
    assert_eq!(
        count(connection, "SELECT count(*) FROM workstate_commit", []),
        1
    );
    assert_eq!(count(connection, "SELECT count(*) FROM branch", []), 1);
    assert_eq!(count(connection, "SELECT count(*) FROM event", []), 1);
    assert_eq!(
        count(connection, "SELECT count(*) FROM commit_parent", []),
        0
    );
    assert_eq!(
        count(connection, "SELECT count(*) FROM change_operation", []),
        0
    );
    assert_eq!(
        count(
            connection,
            "SELECT count(*)
             FROM workspace
             WHERE workspace_id = ?1
               AND genesis_commit_id = ?2",
            params![&workspace_id[..], &commit_id[..]],
        ),
        1
    );
    assert_eq!(
        count(
            connection,
            "SELECT count(*)
             FROM changeset
             WHERE changeset_id = ?1
               AND workspace_id = ?2
               AND operation_type = 'workspace.genesis'
               AND operation_schema_version = 1
               AND operation_payload_json = '{}'
               AND rationale_json = '{}'
               AND origin_session_id IS NULL",
            params![&changeset_id[..], &workspace_id[..]],
        ),
        1
    );
    assert_eq!(
        count(
            connection,
            "SELECT count(*)
             FROM workstate_commit
             WHERE commit_id = ?1
               AND workspace_id = ?2
               AND changeset_id = ?3
               AND commit_kind = 'genesis'
               AND state_digest = ?4",
            params![
                &commit_id[..],
                &workspace_id[..],
                &changeset_id[..],
                &info.state_digest.as_bytes()[..]
            ],
        ),
        1
    );
    assert_eq!(
        count(
            connection,
            "SELECT count(*)
             FROM branch
             WHERE branch_id = ?1
               AND workspace_id = ?2
               AND name = ?3
               AND head_commit_id = ?4
               AND lifecycle_state = 'active'",
            params![
                &branch_id[..],
                &workspace_id[..],
                &info.initial_branch_name,
                &commit_id[..]
            ],
        ),
        1
    );
    assert_eq!(
        count(
            connection,
            "SELECT count(*)
             FROM event
             WHERE workspace_id = ?1
               AND changeset_id = ?2
               AND session_id IS NULL
               AND event_kind = 'workspace.initialized'
               AND payload_json = '{}'",
            params![&workspace_id[..], &changeset_id[..]],
        ),
        1
    );
    assert_no_foreign_key_violations(connection);
}

#[test]
fn creates_workspace_genesis_and_reopens_it() {
    let (_tempdir, path) = store_path();
    let mut engine = init_engine(&path);
    let workspace = engine
        .create_workspace(WorkspaceInitOptions::new("alpha").expect("workspace options"))
        .expect("create workspace");

    assert_eq!(workspace.display_name, "alpha");
    assert_eq!(workspace.initial_branch_name, "main");
    assert_eq!(
        workspace.state_digest,
        work_state_mapping_digest(&WorkState::empty())
    );
    assert_eq!(
        engine
            .workspace_info(workspace.workspace_id)
            .expect("workspace info"),
        workspace
    );

    let connection = raw_connection(&path);
    assert_genesis_shape(&connection, &workspace);
    drop(connection);
    drop(engine);

    let reopened = Engine::open(&path).expect("reopen engine");
    assert_eq!(
        reopened
            .workspace_info(workspace.workspace_id)
            .expect("reopened workspace info"),
        workspace
    );
}

#[test]
fn custom_initial_branch_name_is_persisted() {
    let (_tempdir, path) = store_path();
    let mut engine = init_engine(&path);
    let workspace = engine
        .create_workspace(
            WorkspaceInitOptions::new("beta")
                .expect("workspace options")
                .with_initial_branch_name("Main/team")
                .expect("branch name"),
        )
        .expect("create workspace");

    assert_eq!(workspace.initial_branch_name, "Main/team");
    assert_eq!(
        engine
            .workspace_info(workspace.workspace_id)
            .expect("workspace info"),
        workspace
    );
}

#[test]
fn branch_name_validation_rejects_non_canonical_names() {
    for invalid in ["", " main", "main ", "feature\nx", "feature\u{7f}x", "a\0b"] {
        let error = WorkspaceInitOptions::new("invalid")
            .expect("workspace options")
            .with_initial_branch_name(invalid)
            .unwrap_err();
        assert_eq!(error.code(), ErrorCode::WorkspaceInvalid);
    }
}

#[test]
fn workspace_info_reports_not_found() {
    let (_tempdir, path) = store_path();
    let engine = init_engine(&path);

    let error = engine
        .workspace_info(WorkspaceId::new_v7())
        .expect_err("missing workspace");
    assert_eq!(error.code(), ErrorCode::WorkspaceNotFound);
}

#[test]
fn workspace_info_rejects_missing_initial_branch_shape() {
    let (_tempdir, path) = store_path();
    let mut engine = init_engine(&path);
    let workspace = engine
        .create_workspace(WorkspaceInitOptions::new("shape").expect("workspace options"))
        .expect("create workspace");

    let connection = raw_connection(&path);
    let branch_id = workspace.initial_branch_id.raw_bytes();
    connection
        .execute(
            "DELETE FROM branch WHERE branch_id = ?1",
            params![&branch_id[..]],
        )
        .expect("delete branch");
    drop(connection);

    let error = engine
        .workspace_info(workspace.workspace_id)
        .expect_err("invalid genesis shape");
    assert_eq!(error.code(), ErrorCode::StoreBootstrapInvalid);
}

#[test]
fn workspace_info_rejects_non_empty_workstate_digest() {
    let (_tempdir, path) = store_path();
    let mut engine = init_engine(&path);
    let workspace = engine
        .create_workspace(WorkspaceInitOptions::new("digest").expect("workspace options"))
        .expect("create workspace");

    let connection = raw_connection(&path);
    let commit_id = workspace.genesis_commit_id.raw_bytes();
    connection
        .execute(
            "UPDATE workstate_commit SET state_digest = ?1 WHERE commit_id = ?2",
            params![&[7_u8; 32][..], &commit_id[..]],
        )
        .expect("corrupt digest");
    drop(connection);

    let error = engine
        .workspace_info(workspace.workspace_id)
        .expect_err("invalid digest");
    assert_eq!(error.code(), ErrorCode::StoreBootstrapInvalid);
}

#[test]
fn workspace_info_rejects_corrupted_genesis_json_payloads() {
    let (_tempdir, path) = store_path();
    let mut engine = init_engine(&path);
    let workspace = engine
        .create_workspace(WorkspaceInitOptions::new("payload").expect("workspace options"))
        .expect("create workspace");

    let connection = raw_connection(&path);
    let changeset_id = workspace.genesis_changeset_id.raw_bytes();
    connection
        .execute(
            "UPDATE changeset SET operation_payload_json = ?1 WHERE changeset_id = ?2",
            params![r#"{"unexpected":true}"#, &changeset_id[..]],
        )
        .expect("corrupt operation payload");
    drop(connection);

    let error = engine
        .workspace_info(workspace.workspace_id)
        .expect_err("invalid operation payload");
    assert_eq!(error.code(), ErrorCode::StoreBootstrapInvalid);
}

#[test]
fn workspace_info_rejects_missing_canonical_provenance_event() {
    let (_tempdir, path) = store_path();
    let mut engine = init_engine(&path);
    let workspace = engine
        .create_workspace(WorkspaceInitOptions::new("event").expect("workspace options"))
        .expect("create workspace");

    let connection = raw_connection(&path);
    let changeset_id = workspace.genesis_changeset_id.raw_bytes();
    connection
        .execute(
            "UPDATE event SET payload_json = ?1 WHERE changeset_id = ?2",
            params![r#"{"unexpected":true}"#, &changeset_id[..]],
        )
        .expect("corrupt event payload");
    drop(connection);

    let error = engine
        .workspace_info(workspace.workspace_id)
        .expect_err("missing canonical event");
    assert_eq!(error.code(), ErrorCode::StoreBootstrapInvalid);
}
