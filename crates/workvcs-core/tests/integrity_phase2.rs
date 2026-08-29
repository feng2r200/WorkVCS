use rusqlite::{Connection, params};
use std::path::{Path, PathBuf};
use tempfile::TempDir;
use workvcs_core::{
    CanonicalValue, ChangeSetId, CommitId, Engine, EntityTransitionCommit, EntityTransitionOptions,
    ErrorCategory, ErrorCode, EventId, StoreInitOptions, WorkspaceInfo, WorkspaceInitOptions,
};

fn store_path() -> (TempDir, PathBuf) {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let path = tempdir.path().join("workvcs.sqlite");
    (tempdir, path)
}

fn init_engine(path: &Path) -> Engine {
    Engine::init(
        path,
        StoreInitOptions::new("phase2-integrity-store").expect("store options"),
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

fn create_two_transitions(
    path: &Path,
) -> (
    Engine,
    WorkspaceInfo,
    EntityTransitionCommit,
    EntityTransitionCommit,
) {
    let (mut engine, workspace) = create_workspace(path);
    let first = engine
        .commit_entity_transition(
            EntityTransitionOptions::create(
                workspace.initial_branch_id,
                workspace.genesis_commit_id,
                "generic_record",
                record_state("first", "open"),
            )
            .expect("first options"),
        )
        .expect("first transition");
    let second = engine
        .commit_entity_transition(
            EntityTransitionOptions::update(
                workspace.initial_branch_id,
                first.commit_id,
                first.entity_id,
                first.entity_version_id,
                record_state("first", "done"),
            )
            .expect("second options"),
        )
        .expect("second transition");
    (engine, workspace, first, second)
}

fn raw_connection(path: &Path) -> Connection {
    let connection = Connection::open(path).expect("raw connection");
    connection
        .pragma_update(None, "foreign_keys", "ON")
        .expect("enable raw foreign keys");
    connection
}

fn raw_connection_without_foreign_keys(path: &Path) -> Connection {
    let connection = Connection::open(path).expect("raw connection");
    connection
        .pragma_update(None, "foreign_keys", "OFF")
        .expect("disable raw foreign keys");
    connection
}

fn assert_integrity_error(error: workvcs_core::WorkVcsError) {
    assert_eq!(error.code(), ErrorCode::IntegrityInvalid, "{error}");
    assert_eq!(error.category(), ErrorCategory::Integrity);
}

#[test]
fn validate_integrity_accepts_linear_history_and_ignores_projection_and_event_noise() {
    let (_tempdir, path) = store_path();
    let (engine, workspace, first, second) = create_two_transitions(&path);

    let connection = raw_connection(&path);
    let branch_id = workspace.initial_branch_id.raw_bytes();
    let first_commit_id = first.commit_id.raw_bytes();
    let event_id = EventId::new_v7().raw_bytes();
    connection
        .execute(
            "INSERT INTO branch_projection_state(
                branch_id,
                projection_status,
                projected_commit_id,
                projection_state_digest,
                updated_at_us
             )
             VALUES (?1, 'complete', ?2, ?3, 7)
             ON CONFLICT(branch_id) DO UPDATE SET
                projection_status = excluded.projection_status,
                projected_commit_id = excluded.projected_commit_id,
                projection_state_digest = excluded.projection_state_digest,
                updated_at_us = excluded.updated_at_us",
            params![&branch_id[..], &first_commit_id[..], &[8_u8; 32][..]],
        )
        .expect("insert projection noise");
    connection
        .execute(
            "INSERT INTO event(
                event_id,
                workspace_id,
                changeset_id,
                session_id,
                event_kind,
                occurred_at_us,
                payload_json
             )
             VALUES (?1, ?2, NULL, NULL, 'integrity.noise', 8, '{}')",
            params![&event_id[..], &workspace.workspace_id.raw_bytes()[..]],
        )
        .expect("insert event noise");
    drop(connection);

    let report = engine.validate_integrity().expect("integrity");
    assert_eq!(report.checked_branches, 1);
    assert_eq!(report.checked_commits, 3);

    let head = engine
        .branch_head(workspace.initial_branch_id)
        .expect("branch head");
    assert_eq!(head.head_commit_id, second.commit_id);
}

#[test]
fn validate_integrity_rejects_workstate_commit_digest_drift() {
    let (_tempdir, path) = store_path();
    let (engine, _workspace, _first, second) = create_two_transitions(&path);

    let connection = raw_connection(&path);
    connection
        .execute(
            "UPDATE workstate_commit
             SET state_digest = ?1
             WHERE commit_id = ?2",
            params![&[7_u8; 32][..], &second.commit_id.raw_bytes()[..]],
        )
        .expect("corrupt workstate digest");
    drop(connection);

    assert_integrity_error(
        engine
            .validate_integrity()
            .expect_err("workstate digest drift"),
    );
}

#[test]
fn validate_integrity_rejects_changeset_payload_drift() {
    let (_tempdir, path) = store_path();
    let (engine, _workspace, _first, second) = create_two_transitions(&path);

    let connection = raw_connection(&path);
    connection
        .execute(
            "UPDATE changeset
             SET operation_payload_json = ?1
             WHERE changeset_id = ?2",
            params![
                r#"{"unexpected":true}"#,
                &second.changeset_id.raw_bytes()[..]
            ],
        )
        .expect("corrupt changeset payload");
    drop(connection);

    assert_integrity_error(
        engine
            .validate_integrity()
            .expect_err("changeset payload drift"),
    );
}

#[test]
fn validate_integrity_rejects_change_operation_payload_drift() {
    let (_tempdir, path) = store_path();
    let (engine, _workspace, _first, second) = create_two_transitions(&path);

    let connection = raw_connection(&path);
    connection
        .execute(
            "UPDATE change_operation
             SET operation_payload_json = ?1
             WHERE operation_id = ?2",
            params![
                r#"{"unexpected":true}"#,
                &second.operation_id.raw_bytes()[..]
            ],
        )
        .expect("corrupt change operation payload");
    drop(connection);

    assert_integrity_error(
        engine
            .validate_integrity()
            .expect_err("change operation payload drift"),
    );
}

#[test]
fn validate_integrity_rejects_entity_version_digest_drift() {
    let (_tempdir, path) = store_path();
    let (engine, _workspace, _first, second) = create_two_transitions(&path);

    let connection = raw_connection(&path);
    connection
        .execute(
            "UPDATE entity_version
             SET state_digest = ?1
             WHERE entity_version_id = ?2",
            params![&[9_u8; 32][..], &second.entity_version_id.raw_bytes()[..]],
        )
        .expect("corrupt entity version digest");
    drop(connection);

    assert_integrity_error(
        engine
            .validate_integrity()
            .expect_err("entity version digest drift"),
    );
}

#[test]
fn validate_integrity_rejects_commit_parent_shape_drift() {
    let (_tempdir, path) = store_path();
    let (engine, workspace, _first, second) = create_two_transitions(&path);

    let connection = raw_connection(&path);
    connection
        .execute(
            "INSERT INTO commit_parent(
                commit_id,
                parent_ordinal,
                parent_role,
                parent_commit_id
             )
             VALUES (?1, 1, 'secondary', ?2)",
            params![
                &second.commit_id.raw_bytes()[..],
                &workspace.genesis_commit_id.raw_bytes()[..]
            ],
        )
        .expect("insert extra parent");
    drop(connection);

    assert_integrity_error(
        engine
            .validate_integrity()
            .expect_err("commit parent shape drift"),
    );
}

#[test]
fn validate_integrity_rejects_branch_head_workspace_shape_drift() {
    let (_tempdir, path) = store_path();
    let (engine, first_workspace, _first, second) = create_two_transitions(&path);
    let mut second_workspace_engine = Engine::open(&path).expect("second workspace engine");
    let second_workspace = second_workspace_engine
        .create_workspace(WorkspaceInitOptions::new("other").expect("workspace options"))
        .expect("second workspace");

    let connection = raw_connection_without_foreign_keys(&path);
    connection
        .execute(
            "UPDATE branch
             SET head_commit_id = ?1
             WHERE branch_id = ?2",
            params![
                &second_workspace.genesis_commit_id.raw_bytes()[..],
                &first_workspace.initial_branch_id.raw_bytes()[..]
            ],
        )
        .expect("corrupt branch head workspace shape");
    drop(connection);

    assert_integrity_error(
        engine
            .validate_integrity()
            .expect_err("branch head workspace drift"),
    );

    let shown = second_workspace_engine
        .show_at(second.commit_id)
        .expect("second commit still replays");
    assert_eq!(shown.commit_id, second.commit_id);
}

#[test]
fn validate_integrity_rejects_unbranched_corrupted_commit() {
    let (_tempdir, path) = store_path();
    let (engine, workspace) = create_workspace(&path);

    let connection = raw_connection(&path);
    let workspace_id = workspace.workspace_id.raw_bytes();
    let changeset_id = ChangeSetId::new_v7().raw_bytes();
    let commit_id = CommitId::new_v7().raw_bytes();
    connection
        .execute(
            "INSERT INTO changeset(
                changeset_id,
                workspace_id,
                operation_type,
                operation_schema_version,
                operation_payload_json,
                rationale_json,
                origin_session_id,
                created_at_us
             )
             VALUES (?1, ?2, 'entity.transition', 1, '{}', '{}', NULL, 2)",
            params![&changeset_id[..], &workspace_id[..]],
        )
        .expect("insert orphan changeset");
    connection
        .execute(
            "INSERT INTO workstate_commit(
                commit_id,
                workspace_id,
                changeset_id,
                commit_kind,
                state_digest,
                committed_at_us
             )
             VALUES (?1, ?2, ?3, 'normal', ?4, 3)",
            params![
                &commit_id[..],
                &workspace_id[..],
                &changeset_id[..],
                &workspace.state_digest.as_bytes()[..]
            ],
        )
        .expect("insert orphan normal commit");
    connection
        .execute(
            "INSERT INTO commit_parent(
                commit_id,
                parent_ordinal,
                parent_role,
                parent_commit_id
             )
             VALUES (?1, 0, 'primary', ?2)",
            params![&commit_id[..], &workspace.genesis_commit_id.raw_bytes()[..]],
        )
        .expect("insert orphan parent");
    drop(connection);

    assert_integrity_error(
        engine
            .validate_integrity()
            .expect_err("unbranched corrupted commit"),
    );
}
