use tempfile::TempDir;
use workvcs_core::{
    BranchId, BundlePayloadExport, BundlePayloadExportOptions, BundlePayloadInput,
    BundlePayloadValidationOptions, CommitId, Engine, StoreInitOptions, TaskCreateCommit,
    TaskCreateOptions, WorkspaceInfo, WorkspaceInitOptions,
};

fn store_path() -> (TempDir, std::path::PathBuf) {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let path = tempdir.path().join("workvcs.sqlite");
    (tempdir, path)
}

fn init_engine(path: &std::path::Path) -> Engine {
    Engine::init(
        path,
        StoreInitOptions::new("phase4u-bundle-store").expect("store options"),
    )
    .expect("init engine")
}

fn create_workspace(path: &std::path::Path) -> (Engine, WorkspaceInfo) {
    let mut engine = init_engine(path);
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

fn validation_options(
    export: &BundlePayloadExport,
    payloads: Vec<BundlePayloadInput>,
) -> BundlePayloadValidationOptions {
    BundlePayloadValidationOptions::from_parts(
        export.manifest.commit_id,
        export.manifest_bytes.clone(),
        export.payload_index_bytes.clone(),
        payloads,
    )
    .expect("validation options")
}

#[test]
fn bundle_payload_validation_accepts_exact_exported_directory_parts() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let task = create_task(
        &mut engine,
        workspace.initial_branch_id,
        workspace.genesis_commit_id,
        "Validate payload export",
    );
    let export = engine
        .export_bundle_payloads(BundlePayloadExportOptions::for_commit(task.commit_id))
        .expect("export bundle payloads");

    let validation = engine
        .validate_bundle_payloads(validation_options(&export, payload_inputs(&export)))
        .expect("validate payload export");

    assert!(validation.valid, "{:?}", validation.problem);
    assert_eq!(validation.problem, None);
    assert_eq!(validation.commit_id, task.commit_id);
    assert_eq!(
        validation.expected_manifest_digest,
        export.manifest.manifest_digest
    );
    assert_eq!(
        validation.expected_payload_index_digest,
        export.payload_index_digest
    );
    assert_eq!(
        validation.expected_payload_files,
        export.payload_files.len()
    );
    assert_eq!(validation.actual_payload_files, export.payload_files.len());
    assert_eq!(
        validation.expected_payload_references,
        export.payload_references.len()
    );
}

#[test]
fn bundle_payload_validation_rejects_payload_byte_drift() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let task = create_task(
        &mut engine,
        workspace.initial_branch_id,
        workspace.genesis_commit_id,
        "Drift payload export",
    );
    let export = engine
        .export_bundle_payloads(BundlePayloadExportOptions::for_commit(task.commit_id))
        .expect("export bundle payloads");
    let mut payloads = payload_inputs(&export);
    payloads[0].bytes = br#"{"drifted":true}"#.to_vec();

    let validation = engine
        .validate_bundle_payloads(validation_options(&export, payloads))
        .expect("validate payload export");

    assert!(!validation.valid);
    assert!(
        validation
            .problem
            .as_deref()
            .expect("problem")
            .contains("digest does not match expected content digest")
    );
}

#[test]
fn bundle_payload_validation_rejects_missing_payload_file() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let task = create_task(
        &mut engine,
        workspace.initial_branch_id,
        workspace.genesis_commit_id,
        "Missing payload export",
    );
    let export = engine
        .export_bundle_payloads(BundlePayloadExportOptions::for_commit(task.commit_id))
        .expect("export bundle payloads");
    let mut payloads = payload_inputs(&export);
    payloads.pop();

    let validation = engine
        .validate_bundle_payloads(validation_options(&export, payloads))
        .expect("validate payload export");

    assert!(!validation.valid);
    assert!(
        validation
            .problem
            .as_deref()
            .expect("problem")
            .contains("is missing")
    );
}
