use tempfile::TempDir;
use workvcs_core::{
    BranchId, BundleImportPreflightOptions, BundlePayloadExport, BundlePayloadExportOptions,
    BundlePayloadInput, CommitId, Engine, StoreInitOptions, TaskCreateCommit, TaskCreateOptions,
    WorkspaceInfo, WorkspaceInitOptions, content_object_digest,
};

fn store_path() -> (TempDir, std::path::PathBuf) {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let path = tempdir.path().join("workvcs.sqlite");
    (tempdir, path)
}

fn create_workspace(path: &std::path::Path) -> (Engine, WorkspaceInfo) {
    let mut engine = Engine::init(
        path,
        StoreInitOptions::new("phase4au-bundle-branch-integrity-store").expect("store options"),
    )
    .expect("init engine");
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

fn corrupt_manifest_branch_head_digest(export: &BundlePayloadExport) -> (Vec<u8>, Vec<u8>) {
    let branch = export
        .manifest
        .exported_branch_heads
        .first()
        .expect("exported branch head");
    let manifest_json = String::from_utf8(export.manifest_bytes.clone()).expect("manifest utf8");
    let original_branch_digest = branch.head_state_digest.to_string();
    let branch_digest_needle = format!("\"head_state_digest\":\"{original_branch_digest}\"");
    assert!(manifest_json.contains(&branch_digest_needle));
    let branch_digest_replacement = format!("\"head_state_digest\":\"{}\"", "0".repeat(64));
    let corrupted_manifest_bytes = manifest_json
        .replace(&branch_digest_needle, &branch_digest_replacement)
        .into_bytes();

    let payload_index_json =
        String::from_utf8(export.payload_index_bytes.clone()).expect("payload index utf8");
    let original_manifest_digest = export.manifest.manifest_digest.to_string();
    let new_manifest_digest = content_object_digest(&corrupted_manifest_bytes).to_string();
    let manifest_digest_needle = format!("\"digest\":\"{original_manifest_digest}\"");
    assert!(payload_index_json.contains(&manifest_digest_needle));
    let manifest_digest_replacement = format!("\"digest\":\"{new_manifest_digest}\"");
    let corrupted_payload_index_bytes = payload_index_json
        .replace(&manifest_digest_needle, &manifest_digest_replacement)
        .into_bytes();

    (corrupted_manifest_bytes, corrupted_payload_index_bytes)
}

#[test]
fn bundle_import_preflight_rejects_branch_head_digest_drift_inside_manifest() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let task = create_task(
        &mut engine,
        workspace.initial_branch_id,
        workspace.genesis_commit_id,
        "Branch integrity preflight",
    );
    let export = engine
        .export_bundle_payloads(BundlePayloadExportOptions::for_commit(task.commit_id))
        .expect("export bundle payloads");
    let (corrupted_manifest_bytes, corrupted_payload_index_bytes) =
        corrupt_manifest_branch_head_digest(&export);
    let options = BundleImportPreflightOptions::from_parts(
        corrupted_manifest_bytes,
        corrupted_payload_index_bytes,
        payload_inputs(&export),
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
            .expect("preflight problem")
            .contains("head digest does not match commit closure")
    );
}
