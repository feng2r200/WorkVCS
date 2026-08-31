use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;
use workvcs_core::{
    BundleImportPreflightOptions, BundlePayloadExport, BundlePayloadExportOptions,
    BundlePayloadInput, CommitId, Engine, StoreInitOptions, TaskCreateCommit, TaskCreateOptions,
    WorkspaceInfo, WorkspaceInitOptions,
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
        StoreInitOptions::new("phase4at-bundle-branch-preflight-store").expect("store options"),
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

fn import_options(export: &BundlePayloadExport) -> BundleImportPreflightOptions {
    let payloads = export
        .payload_files
        .iter()
        .map(|payload| BundlePayloadInput::new(&payload.relative_path, payload.bytes.clone()))
        .collect::<workvcs_core::Result<Vec<_>>>()
        .expect("payload inputs");
    BundleImportPreflightOptions::from_parts(
        export.manifest_bytes.clone(),
        export.payload_index_bytes.clone(),
        payloads,
    )
    .expect("preflight options")
}

#[test]
fn bundle_import_preflight_classifies_same_store_branch_fast_forward() {
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

    let old_engine = Engine::open(&old_path).expect("open old store");
    let preflight = old_engine
        .preflight_bundle_import(import_options(&export))
        .expect("preflight bundle import");

    assert!(preflight.valid, "{:?}", preflight.problem);
    assert_eq!(preflight.source_store_relation, "same_store");
    assert!(preflight.import_required);
    assert!(preflight.can_apply);
    assert!(!preflight.incoming_commit_present);
    assert_eq!(preflight.action, "same_store_fast_forward_ready");
    assert_eq!(preflight.exported_branch_heads, 1);
    assert_eq!(preflight.branch_heads_already_present, 0);
    assert_eq!(preflight.branch_heads_missing, 0);
    assert_eq!(preflight.branch_heads_fast_forward, 1);
    assert_eq!(preflight.branch_heads_diverged, 0);
}

#[test]
fn bundle_import_preflight_classifies_same_store_branch_divergence() {
    let (_tempdir, source_path, old_path) = store_paths();
    let (mut engine, workspace) = create_workspace(&source_path);
    let first = create_task(
        &mut engine,
        &workspace,
        workspace.genesis_commit_id,
        "shared old branch head task",
    );
    drop(engine);
    fs::copy(&source_path, &old_path).expect("copy old store");

    let mut source_engine = Engine::open(&source_path).expect("open source store");
    let source_second = create_task(
        &mut source_engine,
        &workspace,
        first.commit_id,
        "source exported branch head task",
    );
    let export = source_engine
        .export_bundle_payloads(BundlePayloadExportOptions::for_commit(
            source_second.commit_id,
        ))
        .expect("export payloads");

    let mut old_engine = Engine::open(&old_path).expect("open old store");
    create_task(
        &mut old_engine,
        &workspace,
        first.commit_id,
        "local divergent branch head task",
    );

    let preflight = old_engine
        .preflight_bundle_import(import_options(&export))
        .expect("preflight bundle import");

    assert!(preflight.valid, "{:?}", preflight.problem);
    assert_eq!(preflight.source_store_relation, "same_store");
    assert!(preflight.import_required);
    assert!(!preflight.can_apply);
    assert!(!preflight.incoming_commit_present);
    assert_eq!(preflight.action, "same_store_divergence_detected");
    assert_eq!(preflight.exported_branch_heads, 1);
    assert_eq!(preflight.branch_heads_already_present, 0);
    assert_eq!(preflight.branch_heads_missing, 0);
    assert_eq!(preflight.branch_heads_fast_forward, 0);
    assert_eq!(preflight.branch_heads_diverged, 1);
    assert_eq!(preflight.problem, None);
}
