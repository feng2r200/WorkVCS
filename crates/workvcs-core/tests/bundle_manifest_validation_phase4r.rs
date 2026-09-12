use tempfile::TempDir;
use workvcs_core::{
    BranchId, BundleExportOptions, BundleManifestValidationOptions, CommitId, Engine,
    StoreInitOptions, TaskCreateCommit, TaskCreateOptions, WorkspaceInfo, WorkspaceInitOptions,
    canonical_bytes, content_object_digest,
};

fn store_path() -> (TempDir, std::path::PathBuf) {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let path = tempdir.path().join("workvcs.sqlite");
    (tempdir, path)
}

fn init_engine(path: &std::path::Path) -> Engine {
    Engine::init(
        path,
        StoreInitOptions::new("phase4r-bundle-store").expect("store options"),
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

#[test]
fn bundle_manifest_validation_accepts_exact_exported_bytes() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let task = create_task(
        &mut engine,
        workspace.initial_branch_id,
        workspace.genesis_commit_id,
        "Validate exported bundle manifest",
    );
    let export = engine
        .export_bundle_manifest(BundleExportOptions::for_commit(task.commit_id))
        .expect("export manifest");
    let bytes = canonical_bytes(&export.manifest).expect("manifest bytes");

    let validation = engine
        .validate_bundle_manifest(
            BundleManifestValidationOptions::from_bytes(task.commit_id, bytes.clone())
                .expect("validation options"),
        )
        .expect("validate manifest");

    assert!(
        validation.valid,
        "unexpected validation problem: {:?}",
        validation.problem
    );
    assert_eq!(validation.problem, None);
    assert_eq!(validation.commit_id, task.commit_id);
    assert_eq!(validation.expected_manifest_digest, export.manifest_digest);
    assert_eq!(validation.actual_manifest_digest, export.manifest_digest);
    assert_eq!(
        validation.actual_manifest_digest,
        content_object_digest(&bytes)
    );
    assert_eq!(
        usize::try_from(validation.actual_manifest_size_bytes).expect("size"),
        bytes.len()
    );
}

#[test]
fn bundle_manifest_validation_rejects_manifest_content_drift() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let task = create_task(
        &mut engine,
        workspace.initial_branch_id,
        workspace.genesis_commit_id,
        "Drifted bundle manifest",
    );
    let export = engine
        .export_bundle_manifest(BundleExportOptions::for_commit(task.commit_id))
        .expect("export manifest");
    let mut json = String::from_utf8(canonical_bytes(&export.manifest).expect("manifest bytes"))
        .expect("manifest text");
    json = json.replace(
        "workvcs-local-export-manifest-v2",
        "workvcs-local-export-manifest-v3",
    );

    let validation = engine
        .validate_bundle_manifest(
            BundleManifestValidationOptions::from_bytes(task.commit_id, json.into_bytes())
                .expect("validation options"),
        )
        .expect("validate manifest");

    assert!(!validation.valid);
    assert_ne!(
        validation.expected_manifest_digest,
        validation.actual_manifest_digest
    );
    assert_eq!(
        validation.problem.as_deref(),
        Some("bundle manifest content does not match expected export manifest")
    );
}

#[test]
fn bundle_manifest_validation_rejects_noncanonical_manifest_bytes() {
    let (_tempdir, path) = store_path();
    let (engine, workspace) = create_workspace(&path);

    let validation = engine
        .validate_bundle_manifest(
            BundleManifestValidationOptions::from_bytes(
                workspace.genesis_commit_id,
                br#"{"z":1,"a":2}"#.to_vec(),
            )
            .expect("validation options"),
        )
        .expect("validate manifest");

    assert!(!validation.valid);
    assert_eq!(
        validation.problem.as_deref(),
        Some("bundle manifest bytes are not fixed-point canonical JSON")
    );
}
