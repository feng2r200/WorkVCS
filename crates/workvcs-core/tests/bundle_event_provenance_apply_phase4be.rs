use rusqlite::{Connection, params};
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;
use workvcs_core::{
    BundleImportApplyOptions, BundleImportPreflightOptions, BundlePayloadExport,
    BundlePayloadExportOptions, BundlePayloadInput, ChangeSetId, CommitId, Engine,
    StoreInitOptions, TaskCreateCommit, TaskCreateOptions, WorkspaceInfo, WorkspaceInitOptions,
};

fn store_paths() -> (TempDir, PathBuf, PathBuf) {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let source_path = tempdir.path().join("source.sqlite");
    let old_path = tempdir.path().join("old.sqlite");
    (tempdir, source_path, old_path)
}

fn create_workspace(path: &Path) -> (Engine, WorkspaceInfo) {
    let mut engine = Engine::init(
        path,
        StoreInitOptions::new("phase4be-bundle-event-apply-store").expect("store options"),
    )
    .expect("init engine");
    let workspace = engine
        .create_workspace(WorkspaceInitOptions::new("workspace").expect("workspace options"))
        .expect("create workspace");
    (engine, workspace)
}

fn create_task(
    engine: &mut Engine,
    workspace: &WorkspaceInfo,
    head: CommitId,
    description: &str,
) -> TaskCreateCommit {
    engine
        .create_task(
            TaskCreateOptions::new(workspace.initial_branch_id, head, description)
                .expect("task options"),
        )
        .expect("create task")
}

fn payload_inputs(export: &BundlePayloadExport) -> Vec<BundlePayloadInput> {
    export
        .payload_files
        .iter()
        .map(|payload| {
            BundlePayloadInput::new(payload.relative_path.clone(), payload.bytes.clone())
                .expect("payload input")
        })
        .collect()
}

fn preflight_options(export: &BundlePayloadExport) -> BundleImportPreflightOptions {
    BundleImportPreflightOptions::from_parts(
        export.manifest_bytes.clone(),
        export.payload_index_bytes.clone(),
        payload_inputs(export),
    )
    .expect("preflight options")
}

fn apply_options(export: &BundlePayloadExport) -> BundleImportApplyOptions {
    BundleImportApplyOptions::from_parts(
        export.manifest_bytes.clone(),
        export.payload_index_bytes.clone(),
        payload_inputs(export),
    )
    .expect("apply options")
}

fn raw_connection(path: &Path) -> Connection {
    let connection = Connection::open(path).expect("raw connection");
    connection
        .pragma_update(None, "foreign_keys", "ON")
        .expect("enable raw foreign keys");
    connection
}

fn event_for_changeset(path: &Path, changeset_id: ChangeSetId) -> (String, String) {
    raw_connection(path)
        .query_row(
            "SELECT event_kind, payload_json
             FROM event
             WHERE changeset_id = ?1",
            params![&changeset_id.raw_bytes()[..]],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
        )
        .expect("event for changeset")
}

#[test]
fn bundle_apply_restores_commit_event_provenance() {
    let (_tempdir, source_path, old_path) = store_paths();
    let (engine, workspace) = create_workspace(&source_path);
    drop(engine);
    fs::copy(&source_path, &old_path).expect("copy old store");

    let mut source_engine = Engine::open(&source_path).expect("open source store");
    let task = create_task(
        &mut source_engine,
        &workspace,
        workspace.genesis_commit_id,
        "task with portable event provenance",
    );
    let expected_event = event_for_changeset(&source_path, task.changeset_id);
    let export = source_engine
        .export_bundle_payloads(BundlePayloadExportOptions::for_commit(task.commit_id))
        .expect("export payloads");

    assert_eq!(export.manifest.events.len(), 2);
    assert!(export.payload_references.iter().any(|reference| {
        reference.role == "event_payload"
            && reference.content_digest == export.manifest.events[0].payload_digest
    }));

    let mut old_engine = Engine::open(&old_path).expect("open old store");
    let before = old_engine
        .preflight_bundle_import(preflight_options(&export))
        .expect("preflight before apply");
    assert_eq!(before.action, "same_store_fast_forward_ready");
    assert!(before.can_apply);

    let applied = old_engine
        .apply_bundle_import(apply_options(&export))
        .expect("apply bundle import");

    assert!(applied.applied);
    assert_eq!(applied.outcome, "same_store_fast_forward_applied");
    assert_eq!(applied.imported_commits, 1);
    assert_eq!(applied.imported_events, 1);
    assert_eq!(applied.updated_branch_heads, 1);
    assert_eq!(
        event_for_changeset(&old_path, task.changeset_id),
        expected_event
    );

    let state = old_engine
        .show_at(task.commit_id)
        .expect("replay imported task commit");
    assert_eq!(state.state_digest, task.work_state_digest);
}
