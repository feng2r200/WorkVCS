use std::path::{Path, PathBuf};
use tempfile::TempDir;
use workvcs_core::{
    BranchForkOptions, CommitId, Engine, ErrorCategory, ErrorCode, StoreInitOptions,
    TaskCreateOptions, WorkspaceId, WorkspaceInfo, WorkspaceInitOptions,
};

fn store_path() -> (TempDir, PathBuf) {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let path = tempdir.path().join("workvcs.sqlite");
    (tempdir, path)
}

fn init_engine(path: &Path) -> Engine {
    Engine::init(
        path,
        StoreInitOptions::new("phase3ai-branch-list-store").expect("store options"),
    )
    .expect("init engine")
}

fn create_workspace(path: &Path) -> (Engine, WorkspaceInfo) {
    let mut engine = init_engine(path);
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
) -> workvcs_core::TaskCreateCommit {
    engine
        .create_task(
            TaskCreateOptions::new(workspace.initial_branch_id, head, description)
                .expect("task options"),
        )
        .expect("create task")
}

#[test]
fn list_branches_returns_workspace_branches_with_current_heads() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let source_task = create_task(
        &mut engine,
        &workspace,
        workspace.genesis_commit_id,
        "Source branch task",
    );
    let alpha = engine
        .fork_branch(
            BranchForkOptions::from_branch(workspace.initial_branch_id, "alpha")
                .expect("alpha fork"),
        )
        .expect("fork alpha");
    let zeta = engine
        .fork_branch(
            BranchForkOptions::from_commit(workspace.genesis_commit_id, "zeta").expect("zeta fork"),
        )
        .expect("fork zeta");

    let branches = engine
        .list_branches(workspace.workspace_id)
        .expect("list branches");
    assert_eq!(branches.len(), 3);
    assert_eq!(
        branches
            .iter()
            .map(|branch| branch.name.as_str())
            .collect::<Vec<_>>(),
        vec!["alpha", workspace.initial_branch_name.as_str(), "zeta"]
    );
    assert!(branches.iter().all(|branch| {
        branch.workspace_id == workspace.workspace_id && branch.lifecycle_state == "active"
    }));

    let alpha_head = branches
        .iter()
        .find(|branch| branch.branch_id == alpha.branch_id)
        .expect("alpha head");
    assert_eq!(alpha_head.head_commit_id, source_task.commit_id);
    assert_eq!(alpha_head.head_changeset_id, source_task.changeset_id);
    assert_eq!(alpha_head.head_commit_kind, "normal");
    assert_eq!(alpha_head.head_operation_type, "entity.transition");
    assert_eq!(alpha_head.head_operation_schema_version, 1);
    assert_eq!(alpha_head.state_digest, source_task.work_state_digest);

    let zeta_head = branches
        .iter()
        .find(|branch| branch.branch_id == zeta.branch_id)
        .expect("zeta head");
    assert_eq!(zeta_head.head_commit_id, workspace.genesis_commit_id);
    assert_eq!(zeta_head.head_changeset_id, workspace.genesis_changeset_id);
    assert_eq!(zeta_head.head_commit_kind, "genesis");
    assert_eq!(zeta_head.head_operation_type, "workspace.genesis");
    assert_eq!(zeta_head.head_operation_schema_version, 1);
    assert_eq!(zeta_head.state_digest, workspace.state_digest);

    let later = create_task(
        &mut engine,
        &workspace,
        source_task.commit_id,
        "Advance source",
    );
    let updated = engine
        .list_branches(workspace.workspace_id)
        .expect("updated branch list");
    let source_head = updated
        .iter()
        .find(|branch| branch.branch_id == workspace.initial_branch_id)
        .expect("source head");
    assert_eq!(source_head.head_commit_id, later.commit_id);
    assert_eq!(source_head.head_changeset_id, later.changeset_id);
    let alpha_head = updated
        .iter()
        .find(|branch| branch.branch_id == alpha.branch_id)
        .expect("alpha head after source advance");
    assert_eq!(alpha_head.head_commit_id, source_task.commit_id);
}

#[test]
fn list_branches_rejects_unknown_workspace() {
    let (_tempdir, path) = store_path();
    let (engine, _workspace) = create_workspace(&path);

    let error = engine
        .list_branches(WorkspaceId::new_v7())
        .expect_err("unknown workspace");
    assert_eq!(error.code(), ErrorCode::WorkspaceNotFound);
    assert_eq!(error.category(), ErrorCategory::Workspace);
}

#[test]
fn branch_listing_is_read_only() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let task = create_task(
        &mut engine,
        &workspace,
        workspace.genesis_commit_id,
        "Read only branch list",
    );
    let before = engine
        .show_at(task.commit_id)
        .expect("state before list")
        .state_digest;

    let branches = engine
        .list_branches(workspace.workspace_id)
        .expect("list branches");

    assert_eq!(branches.len(), 1);
    let after = engine
        .show_at(task.commit_id)
        .expect("state after list")
        .state_digest;
    assert_eq!(after, before);
    assert_eq!(branches[0].head_commit_id, task.commit_id);
}
