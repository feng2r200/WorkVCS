use std::path::{Path, PathBuf};
use tempfile::TempDir;
use workvcs_core::{
    BranchForkOptions, BundleExportOptions, CanonicalValue, CommitId, Engine, StoreInitOptions,
    TaskCreateCommit, TaskCreateOptions, WorkspaceInfo, WorkspaceInitOptions,
};

fn store_path() -> (TempDir, PathBuf) {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let path = tempdir.path().join("workvcs.sqlite");
    (tempdir, path)
}

fn create_workspace(path: &Path) -> (Engine, WorkspaceInfo) {
    let mut engine = Engine::init(
        path,
        StoreInitOptions::new("phase4as-bundle-branch-head-closure-store").expect("store options"),
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

fn object_field<'a>(value: &'a CanonicalValue, key: &str) -> &'a CanonicalValue {
    let CanonicalValue::Object(entries) = value else {
        panic!("expected canonical object");
    };
    entries
        .iter()
        .find_map(|(entry_key, entry_value)| (entry_key == key).then_some(entry_value))
        .unwrap_or_else(|| panic!("missing key {key}"))
}

#[test]
fn bundle_manifest_includes_only_branch_heads_inside_commit_closure() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let first = create_task(
        &mut engine,
        &workspace,
        workspace.genesis_commit_id,
        "first branch export task",
    );
    let fork = engine
        .fork_branch(BranchForkOptions::from_commit(first.commit_id, "snapshot").expect("fork"))
        .expect("fork branch");
    let second = create_task(
        &mut engine,
        &workspace,
        first.commit_id,
        "second branch export task",
    );

    let first_manifest = engine
        .export_bundle_manifest(BundleExportOptions::for_commit(first.commit_id))
        .expect("export first commit");
    assert_eq!(first_manifest.exported_branch_heads.len(), 1);
    assert_eq!(
        first_manifest.exported_branch_heads[0].branch_id,
        fork.branch_id
    );
    assert_eq!(
        first_manifest.exported_branch_heads[0].head_commit_id,
        first.commit_id
    );
    assert_eq!(
        first_manifest.exported_branch_heads[0].head_state_digest,
        first.work_state_digest
    );

    let second_manifest = engine
        .export_bundle_manifest(BundleExportOptions::for_commit(second.commit_id))
        .expect("export second commit");
    assert_eq!(second_manifest.exported_branch_heads.len(), 2);
    assert!(second_manifest.exported_branch_heads.iter().any(|branch| {
        branch.branch_id == workspace.initial_branch_id
            && branch.head_commit_id == second.commit_id
            && branch.head_state_digest == second.work_state_digest
    }));
    assert!(second_manifest.exported_branch_heads.iter().any(|branch| {
        branch.branch_id == fork.branch_id
            && branch.head_commit_id == first.commit_id
            && branch.head_state_digest == first.work_state_digest
    }));
    let CanonicalValue::Array(branch_heads) =
        object_field(&second_manifest.manifest, "exported_branch_heads")
    else {
        panic!("exported_branch_heads must be an array");
    };
    assert_eq!(branch_heads.len(), 2);
}
