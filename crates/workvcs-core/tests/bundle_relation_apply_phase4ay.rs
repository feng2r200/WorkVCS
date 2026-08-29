use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;
use workvcs_core::{
    BundleImportApplyOptions, BundleImportPreflightOptions, BundlePayloadExport,
    BundlePayloadExportOptions, BundlePayloadInput, CommitId, Engine, StoreInitOptions,
    TaskCreateCommit, TaskCreateOptions, TaskSchedulingRelationCreateCommit,
    TaskSchedulingRelationCreateOptions, WorkspaceInfo, WorkspaceInitOptions,
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
        StoreInitOptions::new("phase4ay-bundle-relation-apply-store").expect("store options"),
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

fn create_dependency(
    engine: &mut Engine,
    workspace: &WorkspaceInfo,
    head: CommitId,
    dependent: &TaskCreateCommit,
    prerequisite: &TaskCreateCommit,
) -> TaskSchedulingRelationCreateCommit {
    engine
        .create_task_scheduling_relation(
            TaskSchedulingRelationCreateOptions::depends_on(
                workspace.initial_branch_id,
                head,
                dependent.task_entity_id,
                prerequisite.task_entity_id,
            )
            .expect("relation options"),
        )
        .expect("create task dependency")
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
fn bundle_apply_restores_relation_versions_and_membership_changes() {
    let (_tempdir, source_path, old_path) = store_paths();
    let (mut engine, workspace) = create_workspace(&source_path);
    let prerequisite = create_task(
        &mut engine,
        &workspace,
        workspace.genesis_commit_id,
        "prerequisite task",
    );
    let dependent = create_task(
        &mut engine,
        &workspace,
        prerequisite.commit_id,
        "dependent task",
    );
    drop(engine);
    fs::copy(&source_path, &old_path).expect("copy old store");

    let mut source_engine = Engine::open(&source_path).expect("open source store");
    let relation = create_dependency(
        &mut source_engine,
        &workspace,
        dependent.commit_id,
        &dependent,
        &prerequisite,
    );
    let export = source_engine
        .export_bundle_payloads(BundlePayloadExportOptions::for_commit(relation.commit_id))
        .expect("export payloads");

    assert_eq!(export.manifest.relation_versions.len(), 1);
    assert_eq!(
        export.manifest.relation_versions[0].relation_id,
        relation.relation_id
    );
    assert_eq!(
        export.manifest.relation_versions[0].relation_version_id,
        relation.relation_version_id
    );
    assert_eq!(export.manifest.relation_membership_changes.len(), 1);
    assert_eq!(
        export.manifest.relation_membership_changes[0].relation_id,
        relation.relation_id
    );

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
    assert_eq!(applied.imported_entity_versions, 0);
    assert_eq!(applied.imported_relation_versions, 1);
    assert_eq!(applied.updated_branch_heads, 1);

    let state = old_engine
        .show_at(relation.commit_id)
        .expect("replay imported relation commit");
    assert!(
        state
            .state
            .relations()
            .contains(&(relation.relation_id, relation.relation_version_id))
    );
    assert_eq!(state.state_digest, relation.work_state_digest);
}
