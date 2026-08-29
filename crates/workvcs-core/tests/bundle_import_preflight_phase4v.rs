use tempfile::TempDir;
use workvcs_core::{
    BranchId, BundleImportPreflightOptions, BundlePayloadExport, BundlePayloadExportOptions,
    BundlePayloadInput, CommitId, Engine, StoreInitOptions, TaskCreateCommit, TaskCreateOptions,
    WorkspaceInfo, WorkspaceInitOptions,
};

fn store_path() -> (TempDir, std::path::PathBuf) {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let path = tempdir.path().join("workvcs.sqlite");
    (tempdir, path)
}

fn init_engine(path: &std::path::Path, display_name: &str) -> Engine {
    Engine::init(
        path,
        StoreInitOptions::new(display_name).expect("store options"),
    )
    .expect("init engine")
}

fn create_workspace(path: &std::path::Path, display_name: &str) -> (Engine, WorkspaceInfo) {
    let mut engine = init_engine(path, display_name);
    let workspace = engine
        .create_workspace(WorkspaceInitOptions::new("workspace").expect("workspace options"))
        .expect("create workspace");
    (engine, workspace)
}

fn create_task(
    engine: &mut Engine,
    branch_id: BranchId,
    head_commit_id: CommitId,
    description: &str,
) -> TaskCreateCommit {
    engine
        .create_task(
            TaskCreateOptions::new(branch_id, head_commit_id, description).expect("task options"),
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

#[test]
fn bundle_import_preflight_reports_current_store_export_as_already_present() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path, "phase4v-source-store");
    let task = create_task(
        &mut engine,
        workspace.initial_branch_id,
        workspace.genesis_commit_id,
        "Preflight current store",
    );
    let export = engine
        .export_bundle_payloads(BundlePayloadExportOptions::for_commit(task.commit_id))
        .expect("export bundle payloads");

    let preflight = engine
        .preflight_bundle_import(preflight_options(&export))
        .expect("preflight bundle import");

    assert!(preflight.valid, "{:?}", preflight.problem);
    assert_eq!(preflight.problem, None);
    assert!(preflight.format_compatible);
    assert_eq!(preflight.source_store_id, Some(export.manifest.store_id));
    assert_eq!(
        preflight.target_workspace_id,
        Some(export.manifest.workspace_id)
    );
    assert_eq!(preflight.target_commit_id, Some(task.commit_id));
    assert_eq!(preflight.source_store_relation, "same_store");
    assert!(preflight.incoming_commit_present);
    assert!(!preflight.import_required);
    assert!(!preflight.can_apply);
    assert_eq!(preflight.action, "already_present");
}

#[test]
fn bundle_import_preflight_reports_external_store_as_not_yet_applicable() {
    let (_source_tempdir, source_path) = store_path();
    let (mut source, workspace) = create_workspace(&source_path, "phase4v-source-store");
    let task = create_task(
        &mut source,
        workspace.initial_branch_id,
        workspace.genesis_commit_id,
        "Preflight external store",
    );
    let export = source
        .export_bundle_payloads(BundlePayloadExportOptions::for_commit(task.commit_id))
        .expect("export bundle payloads");
    let (_target_tempdir, target_path) = store_path();
    let target = init_engine(&target_path, "phase4v-target-store");

    let preflight = target
        .preflight_bundle_import(preflight_options(&export))
        .expect("preflight bundle import");

    assert!(preflight.valid, "{:?}", preflight.problem);
    assert!(preflight.format_compatible);
    assert_eq!(preflight.source_store_relation, "external_store");
    assert!(!preflight.incoming_commit_present);
    assert!(preflight.import_required);
    assert!(!preflight.can_apply);
    assert_eq!(preflight.action, "external_store_import_not_implemented");
}

#[test]
fn bundle_import_preflight_rejects_payload_byte_drift() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path, "phase4v-source-store");
    let task = create_task(
        &mut engine,
        workspace.initial_branch_id,
        workspace.genesis_commit_id,
        "Preflight drift",
    );
    let export = engine
        .export_bundle_payloads(BundlePayloadExportOptions::for_commit(task.commit_id))
        .expect("export bundle payloads");
    let mut payloads = payload_inputs(&export);
    payloads[0].bytes = br#"{"drifted":true}"#.to_vec();
    let options = BundleImportPreflightOptions::from_parts(
        export.manifest_bytes.clone(),
        export.payload_index_bytes.clone(),
        payloads,
    )
    .expect("preflight options");

    let preflight = engine
        .preflight_bundle_import(options)
        .expect("preflight bundle import");

    assert!(!preflight.valid);
    assert_eq!(preflight.action, "invalid_bundle_directory");
    assert!(
        preflight
            .problem
            .as_deref()
            .expect("problem")
            .contains("digest does not match payload index")
    );
}
