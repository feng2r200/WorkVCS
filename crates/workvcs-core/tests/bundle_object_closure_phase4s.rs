use tempfile::TempDir;
use workvcs_core::{
    BranchId, BundleExportOptions, CanonicalValue, CommitId, Engine, StoreInitOptions,
    TaskCreateCommit, TaskCreateOptions, TaskSchedulingRelationCreateCommit,
    TaskSchedulingRelationCreateOptions, WorkspaceInfo, WorkspaceInitOptions,
};

fn store_path() -> (TempDir, std::path::PathBuf) {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let path = tempdir.path().join("workvcs.sqlite");
    (tempdir, path)
}

fn init_engine(path: &std::path::Path) -> Engine {
    Engine::init(
        path,
        StoreInitOptions::new("phase4s-bundle-store").expect("store options"),
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

fn create_dependency(
    engine: &mut Engine,
    branch_id: BranchId,
    head_commit_id: CommitId,
    dependent: &TaskCreateCommit,
    prerequisite: &TaskCreateCommit,
) -> TaskSchedulingRelationCreateCommit {
    engine
        .create_task_scheduling_relation(
            TaskSchedulingRelationCreateOptions::depends_on(
                branch_id,
                head_commit_id,
                dependent.task_entity_id,
                prerequisite.task_entity_id,
            )
            .expect("dependency options"),
        )
        .expect("create dependency")
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
fn bundle_manifest_includes_current_entity_and_relation_version_closure() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let prerequisite = create_task(
        &mut engine,
        workspace.initial_branch_id,
        workspace.genesis_commit_id,
        "Bundle prerequisite",
    );
    let dependent = create_task(
        &mut engine,
        workspace.initial_branch_id,
        prerequisite.commit_id,
        "Bundle dependent",
    );
    let dependency = create_dependency(
        &mut engine,
        workspace.initial_branch_id,
        dependent.commit_id,
        &dependent,
        &prerequisite,
    );

    let manifest = engine
        .export_bundle_manifest(BundleExportOptions::for_commit(dependency.commit_id))
        .expect("export bundle manifest");

    assert_eq!(manifest.entity_count, 2);
    assert_eq!(manifest.relation_count, 1);
    assert_eq!(manifest.entity_versions.len(), 2);
    assert_eq!(manifest.relation_versions.len(), 1);
    assert!(
        manifest
            .entity_versions
            .iter()
            .all(|entity| entity.entity_kind == "task"
                && entity.state_schema_version == 1
                && entity.state_json_size_bytes > 0)
    );

    let relation = &manifest.relation_versions[0];
    assert_eq!(relation.relation_id, dependency.relation_id);
    assert_eq!(relation.relation_version_id, dependency.relation_version_id);
    assert_eq!(relation.relation_type, dependency.relation_type.as_str());
    assert_eq!(
        relation.source_object_id,
        dependency.source_task_entity_id.to_string()
    );
    assert_eq!(
        relation.target_object_id,
        dependency.target_task_entity_id.to_string()
    );
    assert_eq!(relation.state_digest, dependency.relation_state_digest);
    assert!(relation.metadata_json_size_bytes > 0);

    let CanonicalValue::Array(entity_versions) =
        object_field(&manifest.manifest, "entity_versions")
    else {
        panic!("entity_versions must be an array");
    };
    let CanonicalValue::Array(relation_versions) =
        object_field(&manifest.manifest, "relation_versions")
    else {
        panic!("relation_versions must be an array");
    };
    assert_eq!(entity_versions.len(), 2);
    assert_eq!(relation_versions.len(), 1);
}
