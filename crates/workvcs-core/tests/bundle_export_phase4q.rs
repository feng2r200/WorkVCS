use rusqlite::Connection;
use tempfile::TempDir;
use workvcs_core::{
    BranchId, BundleExportOptions, CanonicalValue, CheckpointCreateOptions, CommitId, Engine,
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
        StoreInitOptions::new("phase4q-bundle-store").expect("store options"),
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

fn raw_connection(path: &std::path::Path) -> Connection {
    let connection = Connection::open(path).expect("raw connection");
    connection
        .pragma_update(None, "foreign_keys", "ON")
        .expect("enable raw foreign keys");
    connection
}

fn count_rows(connection: &Connection, table: &str) -> i64 {
    connection
        .query_row(&format!("SELECT count(*) FROM {table}"), [], |row| {
            row.get(0)
        })
        .expect("count rows")
}

fn object_field<'a>(value: &'a CanonicalValue, key: &str) -> &'a CanonicalValue {
    let CanonicalValue::Object(fields) = value else {
        panic!("expected object, got {value:?}");
    };
    fields
        .iter()
        .find_map(|(candidate, value)| (candidate == key).then_some(value))
        .unwrap_or_else(|| panic!("missing key {key} in {value:?}"))
}

#[test]
fn bundle_export_manifest_includes_commit_closure_and_checkpoint_candidate() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let task = create_task(
        &mut engine,
        workspace.initial_branch_id,
        workspace.genesis_commit_id,
        "Bundle export target",
    );
    let checkpoint = engine
        .create_checkpoint(CheckpointCreateOptions::new(task.commit_id))
        .expect("create checkpoint");

    let manifest = engine
        .export_bundle_manifest(BundleExportOptions::for_commit(task.commit_id))
        .expect("export bundle manifest");

    assert_eq!(
        manifest.manifest_profile,
        "workvcs-local-export-manifest-v1"
    );
    assert_eq!(manifest.manifest_version, 1);
    assert_eq!(manifest.workspace_id, workspace.workspace_id);
    assert_eq!(manifest.commit_id, task.commit_id);
    assert_eq!(manifest.state_digest, task.work_state_digest);
    assert_eq!(manifest.entity_count, 1);
    assert_eq!(manifest.relation_count, 0);
    assert_eq!(manifest.commit_count, 2);
    assert_eq!(manifest.checkpoint_candidates.len(), 1);
    assert_eq!(
        manifest.checkpoint_candidates[0].checkpoint_id,
        checkpoint.checkpoint.checkpoint_id
    );
    assert_eq!(
        manifest.checkpoint_candidates[0].content_digest,
        checkpoint.checkpoint.content_digest
    );

    let encoded = canonical_bytes(&manifest.manifest).expect("canonical manifest bytes");
    assert_eq!(manifest.manifest_digest, content_object_digest(&encoded));
    assert_eq!(
        usize::try_from(manifest.manifest_size_bytes).expect("manifest size"),
        encoded.len()
    );

    let export_limits = object_field(&manifest.manifest, "export_limits");
    assert_eq!(
        object_field(export_limits, "contains_payload_bytes"),
        &CanonicalValue::Bool(false)
    );
    let CanonicalValue::Array(commits) = object_field(&manifest.manifest, "commit_closure") else {
        panic!("commit_closure must be an array");
    };
    assert_eq!(commits.len(), 2);
}

#[test]
fn bundle_export_manifest_is_read_only_and_repeatable() {
    let (_tempdir, path) = store_path();
    let (engine, workspace) = create_workspace(&path);

    let first = engine
        .export_bundle_manifest(BundleExportOptions::for_commit(workspace.genesis_commit_id))
        .expect("first export");
    let second = engine
        .export_bundle_manifest(BundleExportOptions::for_commit(workspace.genesis_commit_id))
        .expect("second export");

    assert_eq!(first.manifest_digest, second.manifest_digest);
    assert_eq!(first.manifest, second.manifest);

    let connection = raw_connection(&path);
    assert_eq!(count_rows(&connection, "import_attempt"), 0);
    assert_eq!(count_rows(&connection, "import_attempt_outcome"), 0);
    assert_eq!(count_rows(&connection, "store_lineage"), 0);
}

#[test]
fn bundle_export_manifest_changes_when_work_state_changes() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);

    let genesis_manifest = engine
        .export_bundle_manifest(BundleExportOptions::for_commit(workspace.genesis_commit_id))
        .expect("genesis export");
    let task = create_task(
        &mut engine,
        workspace.initial_branch_id,
        workspace.genesis_commit_id,
        "Changed export state",
    );
    let task_manifest = engine
        .export_bundle_manifest(BundleExportOptions::for_commit(task.commit_id))
        .expect("task export");

    assert_eq!(genesis_manifest.entity_count, 0);
    assert_eq!(genesis_manifest.commit_count, 1);
    assert_eq!(task_manifest.entity_count, 1);
    assert_eq!(task_manifest.commit_count, 2);
    assert_ne!(genesis_manifest.state_digest, task_manifest.state_digest);
    assert_ne!(
        genesis_manifest.manifest_digest,
        task_manifest.manifest_digest
    );
}
