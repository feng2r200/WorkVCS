use tempfile::TempDir;
use workvcs_core::{
    Engine, StoreInitOptions, TaskCreateCommit, TaskCreateOptions, WorkspaceInfo,
    WorkspaceInitOptions, content_object_digest,
};

fn create_store() -> (TempDir, Engine, WorkspaceInfo) {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let path = tempdir.path().join("workvcs.sqlite");
    let mut engine = Engine::init(
        &path,
        StoreInitOptions::new("phase4bl-changeset-query-store").expect("store options"),
    )
    .expect("init engine");
    let workspace = engine
        .create_workspace(WorkspaceInitOptions::new("workspace").expect("workspace options"))
        .expect("create workspace");
    (tempdir, engine, workspace)
}

fn create_task(engine: &mut Engine, workspace: &WorkspaceInfo) -> TaskCreateCommit {
    engine
        .create_task(
            TaskCreateOptions::new(
                workspace.initial_branch_id,
                workspace.genesis_commit_id,
                "inspect changeset",
            )
            .expect("task options"),
        )
        .expect("create task")
}

#[test]
fn changeset_snapshot_exposes_payloads_counts_and_commit_ref() {
    let (_tempdir, mut engine, workspace) = create_store();
    let task = create_task(&mut engine, &workspace);

    let snapshot = engine.changeset(task.changeset_id).expect("changeset");

    assert_eq!(snapshot.workspace_id, workspace.workspace_id);
    assert_eq!(snapshot.changeset_id, task.changeset_id);
    assert_eq!(snapshot.operation_type, "entity.transition");
    assert_eq!(snapshot.operation_schema_version, 1);
    assert_eq!(snapshot.origin_session_id, None);
    assert_eq!(snapshot.change_operation_count, 1);
    assert_eq!(snapshot.event_count, 1);
    assert_eq!(
        snapshot.operation_payload_digest,
        content_object_digest(snapshot.operation_payload_json.as_bytes())
    );
    assert_eq!(
        snapshot.operation_payload_size_bytes,
        i64::try_from(snapshot.operation_payload_json.len()).expect("payload size")
    );
    assert_eq!(
        snapshot.rationale_digest,
        content_object_digest(snapshot.rationale_json.as_bytes())
    );
    assert_eq!(
        snapshot.rationale_size_bytes,
        i64::try_from(snapshot.rationale_json.len()).expect("rationale size")
    );
    assert_eq!(snapshot.commits.len(), 1);
    assert_eq!(snapshot.commits[0].commit_id, task.commit_id);
    assert_eq!(snapshot.commits[0].commit_kind, "normal");
    assert_eq!(snapshot.commits[0].state_digest, task.work_state_digest);
}

#[test]
fn genesis_changeset_snapshot_exposes_genesis_commit_ref() {
    let (_tempdir, engine, workspace) = create_store();

    let snapshot = engine
        .changeset(workspace.genesis_changeset_id)
        .expect("genesis changeset");

    assert_eq!(snapshot.workspace_id, workspace.workspace_id);
    assert_eq!(snapshot.changeset_id, workspace.genesis_changeset_id);
    assert_eq!(snapshot.operation_type, "workspace.genesis");
    assert_eq!(snapshot.operation_schema_version, 1);
    assert_eq!(snapshot.change_operation_count, 0);
    assert_eq!(snapshot.event_count, 1);
    assert_eq!(snapshot.commits.len(), 1);
    assert_eq!(snapshot.commits[0].commit_id, workspace.genesis_commit_id);
    assert_eq!(snapshot.commits[0].commit_kind, "genesis");
    assert_eq!(snapshot.commits[0].state_digest, workspace.state_digest);
}
