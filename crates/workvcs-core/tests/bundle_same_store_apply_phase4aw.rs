use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;
use workvcs_core::{
    BranchProjectionRefreshOptions, BranchProjectionStatus, BundleImportApplyOptions,
    BundleImportPreflightOptions, BundlePayloadExport, BundlePayloadExportOptions,
    BundlePayloadInput, CommitId, Engine, HistoryQueryOptions, StoreInitOptions, TaskCreateCommit,
    TaskCreateOptions, WorkspaceInfo, WorkspaceInitOptions,
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
        StoreInitOptions::new("phase4aw-bundle-same-store-apply-store").expect("store options"),
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

#[test]
fn bundle_apply_fast_forwards_same_store_task_branch() {
    let (_tempdir, source_path, old_path) = store_paths();
    let (mut engine, workspace) = create_workspace(&source_path);
    let first = create_task(
        &mut engine,
        &workspace,
        workspace.genesis_commit_id,
        "old branch head task",
    );
    drop(engine);
    fs::copy(&source_path, &old_path).expect("copy old store");

    let mut source_engine = Engine::open(&source_path).expect("open source store");
    let second = create_task(
        &mut source_engine,
        &workspace,
        first.commit_id,
        "exported branch head task",
    );
    let export = source_engine
        .export_bundle_payloads(BundlePayloadExportOptions::for_commit(second.commit_id))
        .expect("export payloads");

    let mut old_engine = Engine::open(&old_path).expect("open old store");
    old_engine
        .refresh_branch_projection(BranchProjectionRefreshOptions::new(
            workspace.initial_branch_id,
        ))
        .expect("refresh old projection");
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
    assert_eq!(applied.imported_entity_versions, 1);
    assert_eq!(applied.updated_branch_heads, 1);
    assert_ne!(applied.import_id, None);

    let history = old_engine
        .history(HistoryQueryOptions::from_branch(
            workspace.initial_branch_id,
        ))
        .expect("history after apply");
    assert_eq!(history.start_commit_id, second.commit_id);
    let state = old_engine
        .show_at(second.commit_id)
        .expect("replay imported commit");
    assert_eq!(state.commit_id, second.commit_id);
    assert_eq!(state.state_digest, second.work_state_digest);
    let projection = old_engine
        .branch_projection(workspace.initial_branch_id)
        .expect("projection after apply");
    assert_eq!(projection.status, BranchProjectionStatus::NotMaterialized);
    assert_eq!(projection.projected_commit_id, None);

    let after = old_engine
        .preflight_bundle_import(preflight_options(&export))
        .expect("preflight after apply");
    assert_eq!(after.action, "already_present");
    assert!(!after.import_required);
    assert_eq!(after.branch_heads_already_present, 1);
}
