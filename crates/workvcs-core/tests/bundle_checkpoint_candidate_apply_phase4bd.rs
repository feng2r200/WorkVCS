use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;
use workvcs_core::{
    BundleImportApplyOptions, BundleImportPreflightOptions, BundlePayloadExport,
    BundlePayloadExportOptions, BundlePayloadInput, CheckpointCreateOptions,
    CheckpointLatestOptions, CommitId, Engine, StoreInitOptions, TaskCreateCommit,
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
        StoreInitOptions::new("phase4bd-bundle-checkpoint-apply-store").expect("store options"),
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
fn bundle_apply_restores_checkpoint_candidate_status_and_content_object() {
    let (_tempdir, source_path, old_path) = store_paths();
    let (engine, workspace) = create_workspace(&source_path);
    drop(engine);
    fs::copy(&source_path, &old_path).expect("copy old store");

    let mut source_engine = Engine::open(&source_path).expect("open source store");
    let task = create_task(
        &mut source_engine,
        &workspace,
        workspace.genesis_commit_id,
        "task with portable checkpoint",
    );
    let checkpoint = source_engine
        .create_checkpoint(CheckpointCreateOptions::new(task.commit_id))
        .expect("create checkpoint");
    let export = source_engine
        .export_bundle_payloads(BundlePayloadExportOptions::for_commit(task.commit_id))
        .expect("export payloads");

    assert_eq!(export.manifest.checkpoint_candidates.len(), 1);
    assert_eq!(
        export.manifest.checkpoint_candidates[0].checkpoint_id,
        checkpoint.checkpoint.checkpoint_id
    );
    assert!(export.manifest.content_objects.iter().any(|content| {
        content.content_digest == checkpoint.checkpoint.content_digest
            && content.size_bytes == checkpoint.checkpoint.content_size_bytes
    }));
    assert!(export.payload_references.iter().any(|reference| {
        reference.role == "checkpoint_status_detail"
            && reference.content_digest
                == export.manifest.checkpoint_candidates[0].status_detail_digest
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
    assert_eq!(applied.imported_entity_versions, 1);
    assert_eq!(applied.imported_content_objects, 1);
    assert_eq!(applied.imported_checkpoints, 1);
    assert_eq!(applied.imported_checkpoint_statuses, 1);
    assert_eq!(applied.updated_branch_heads, 1);

    let imported = old_engine
        .checkpoint(checkpoint.checkpoint.checkpoint_id)
        .expect("imported checkpoint");
    assert_eq!(imported, checkpoint.checkpoint);
    let latest = old_engine
        .latest_usable_checkpoint(CheckpointLatestOptions::usable_for_commit(task.commit_id))
        .expect("latest usable checkpoint");
    assert_eq!(latest.checkpoint, Some(imported));

    let state = old_engine
        .show_at(task.commit_id)
        .expect("replay imported task commit");
    assert_eq!(state.state_digest, task.work_state_digest);
}
