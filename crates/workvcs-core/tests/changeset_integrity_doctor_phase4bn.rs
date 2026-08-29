use rusqlite::{Connection, params};
use std::path::Path;
use tempfile::TempDir;
use workvcs_core::{
    ChangeSetId, Engine, EntityId, ErrorCategory, OperationId, StoreInitOptions, TaskCreateOptions,
    WorkspaceInfo, WorkspaceInitOptions,
};

fn create_store() -> (TempDir, Engine, WorkspaceInfo) {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let path = tempdir.path().join("workvcs.sqlite");
    let mut engine = Engine::init(
        &path,
        StoreInitOptions::new("phase4bn-changeset-integrity-store").expect("store options"),
    )
    .expect("init engine");
    let workspace = engine
        .create_workspace(WorkspaceInitOptions::new("workspace").expect("workspace options"))
        .expect("create workspace");
    (tempdir, engine, workspace)
}

fn raw_connection(path: &Path) -> Connection {
    let connection = Connection::open(path).expect("raw connection");
    connection
        .pragma_update(None, "foreign_keys", "ON")
        .expect("enable raw foreign keys");
    connection
}

#[test]
fn doctor_counts_changesets_and_change_operations() {
    let (_tempdir, mut engine, workspace) = create_store();
    engine
        .create_task(
            TaskCreateOptions::new(
                workspace.initial_branch_id,
                workspace.genesis_commit_id,
                "changeset integrity task",
            )
            .expect("task options"),
        )
        .expect("create task");

    let report = engine.validate_integrity().expect("integrity");
    assert_eq!(report.checked_branches, 1);
    assert_eq!(report.checked_commits, 2);
    assert_eq!(report.checked_changesets, 2);
    assert_eq!(report.checked_change_operations, 1);
    assert_eq!(report.checked_events, 2);
}

#[test]
fn doctor_validates_changeset_payload_fixed_point() {
    let (tempdir, mut engine, workspace) = create_store();
    let task = engine
        .create_task(
            TaskCreateOptions::new(
                workspace.initial_branch_id,
                workspace.genesis_commit_id,
                "corrupt changeset payload",
            )
            .expect("task options"),
        )
        .expect("create task");

    raw_connection(tempdir.path().join("workvcs.sqlite").as_path())
        .execute(
            "UPDATE changeset
             SET operation_payload_json = ?1
             WHERE changeset_id = ?2",
            params!["{\"z\":1,\"a\":2}", &task.changeset_id.raw_bytes()[..],],
        )
        .expect("corrupt changeset payload");

    let error = engine
        .validate_integrity()
        .expect_err("non-canonical changeset payload fails integrity");
    assert_eq!(error.category(), ErrorCategory::Integrity);
    assert!(
        error
            .to_string()
            .contains("changeset.operation_payload_json is not canonical fixed-point JSON")
    );
}

#[test]
fn doctor_validates_change_operation_payload_fixed_point() {
    let (tempdir, engine, workspace) = create_store();
    let changeset_id = ChangeSetId::new_v7();
    let operation_id = OperationId::new_v7();
    let entity_id = EntityId::new_v7();

    let connection = raw_connection(tempdir.path().join("workvcs.sqlite").as_path());
    connection
        .execute(
            "INSERT INTO object_identity(object_id, object_kind, created_at_us)
             VALUES(?1, 'entity', 1)",
            params![&entity_id.raw_bytes()[..]],
        )
        .expect("insert raw object identity");
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
             VALUES(?1, ?2, 'phase4bn.raw', 1, '{}', '{}', NULL, 1)",
            params![
                &changeset_id.raw_bytes()[..],
                &workspace.workspace_id.raw_bytes()[..],
            ],
        )
        .expect("insert raw changeset");
    connection
        .execute(
            "INSERT INTO change_operation(
                operation_id,
                changeset_id,
                ordinal,
                subject_family,
                subject_object_id,
                operation_payload_json
             )
             VALUES(?1, ?2, 0, 'entity', ?3, ?4)",
            params![
                &operation_id.raw_bytes()[..],
                &changeset_id.raw_bytes()[..],
                &entity_id.raw_bytes()[..],
                "{\"z\":1,\"a\":2}",
            ],
        )
        .expect("insert corrupt change operation");

    let error = engine
        .validate_integrity()
        .expect_err("non-canonical change operation payload fails integrity");
    assert_eq!(error.category(), ErrorCategory::Integrity);
    assert!(
        error
            .to_string()
            .contains("change_operation.operation_payload_json is not canonical fixed-point JSON")
    );
}
