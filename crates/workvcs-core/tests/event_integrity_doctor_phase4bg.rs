use rusqlite::{Connection, params};
use std::path::Path;
use tempfile::TempDir;
use workvcs_core::{
    Engine, ErrorCategory, StoreInitOptions, TaskCreateOptions, WorkspaceInfo, WorkspaceInitOptions,
};

fn create_store() -> (TempDir, Engine, WorkspaceInfo) {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let path = tempdir.path().join("workvcs.sqlite");
    let mut engine = Engine::init(
        &path,
        StoreInitOptions::new("phase4bg-event-integrity-store").expect("store options"),
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
fn doctor_counts_and_validates_event_payloads() {
    let (tempdir, mut engine, workspace) = create_store();
    let task = engine
        .create_task(
            TaskCreateOptions::new(
                workspace.initial_branch_id,
                workspace.genesis_commit_id,
                "event integrity task",
            )
            .expect("task options"),
        )
        .expect("create task");

    let report = engine.validate_integrity().expect("integrity");
    assert_eq!(report.checked_branches, 1);
    assert_eq!(report.checked_commits, 2);
    assert_eq!(report.checked_events, 2);

    raw_connection(tempdir.path().join("workvcs.sqlite").as_path())
        .execute(
            "UPDATE event
             SET payload_json = ?1
             WHERE changeset_id = ?2",
            params!["{\"z\":1,\"a\":2}", &task.changeset_id.raw_bytes()[..],],
        )
        .expect("corrupt event payload");

    let error = engine
        .validate_integrity()
        .expect_err("non-canonical event payload fails integrity");
    assert_eq!(error.category(), ErrorCategory::Integrity);
    assert!(
        error
            .to_string()
            .contains("event.payload_json is not canonical fixed-point JSON")
    );
}
