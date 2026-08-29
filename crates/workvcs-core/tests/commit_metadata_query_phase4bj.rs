use tempfile::TempDir;
use workvcs_core::{
    BranchForkOptions, BranchId, CanonicalValue, CommitId, Engine, MergeContinueOptions,
    MergeFreezeResolutionsOptions, MergeResolveOptions, MergeStartOptions, StoreInitOptions,
    TaskCreateCommit, TaskCreateOptions, WorkspaceInfo, WorkspaceInitOptions,
};

fn create_store() -> (TempDir, Engine, WorkspaceInfo) {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let path = tempdir.path().join("workvcs.sqlite");
    let mut engine = Engine::init(
        &path,
        StoreInitOptions::new("phase4bj-commit-query-store").expect("store options"),
    )
    .expect("init engine");
    let workspace = engine
        .create_workspace(WorkspaceInitOptions::new("workspace").expect("workspace options"))
        .expect("create workspace");
    (tempdir, engine, workspace)
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

fn detail(reason: &str) -> CanonicalValue {
    CanonicalValue::object(vec![(
        "reason".to_owned(),
        CanonicalValue::String(reason.to_owned()),
    )])
    .expect("detail")
}

#[test]
fn commit_snapshot_exposes_all_merge_parents() {
    let (_tempdir, mut engine, workspace) = create_store();

    let base = create_task(
        &mut engine,
        workspace.initial_branch_id,
        workspace.genesis_commit_id,
        "shared base",
    );
    let source_branch = engine
        .fork_branch(
            BranchForkOptions::from_branch(workspace.initial_branch_id, "source")
                .expect("fork options"),
        )
        .expect("fork source");
    let target = create_task(
        &mut engine,
        workspace.initial_branch_id,
        base.commit_id,
        "target work",
    );
    let source = create_task(
        &mut engine,
        source_branch.branch_id,
        base.commit_id,
        "source work",
    );

    let merge = engine
        .start_merge(MergeStartOptions::new(
            workspace.initial_branch_id,
            source_branch.branch_id,
        ))
        .expect("start merge");
    let item_id = engine
        .merge_attempt(merge.merge_id)
        .expect("show merge")
        .items
        .first()
        .expect("merge item")
        .merge_item_id;
    engine
        .resolve_merge_item(
            MergeResolveOptions::theirs(item_id)
                .expect("resolve options")
                .with_rationale(detail("take source"))
                .expect("rationale"),
        )
        .expect("resolve");
    engine
        .freeze_merge_resolutions(MergeFreezeResolutionsOptions::new(merge.merge_id))
        .expect("freeze");
    let continued = engine
        .continue_merge(
            MergeContinueOptions::new(merge.merge_id)
                .expect("continue options")
                .with_detail(detail("complete merge"))
                .expect("detail"),
        )
        .expect("continue");

    let snapshot = engine
        .commit(continued.result_commit_id)
        .expect("commit snapshot");

    assert_eq!(snapshot.workspace_id, workspace.workspace_id);
    assert_eq!(snapshot.commit_id, continued.result_commit_id);
    assert_eq!(snapshot.changeset_id, continued.changeset_id);
    assert_eq!(snapshot.commit_kind, "merge");
    assert_eq!(snapshot.operation_type, "merge.continue");
    assert_eq!(snapshot.operation_schema_version, 1);
    assert_eq!(snapshot.origin_session_id, None);
    assert_eq!(snapshot.parents.len(), 2);
    assert_eq!(snapshot.parents[0].parent_ordinal, 0);
    assert_eq!(snapshot.parents[0].parent_role, "primary");
    assert_eq!(snapshot.parents[0].parent_commit_id, target.commit_id);
    assert_eq!(snapshot.parents[1].parent_ordinal, 1);
    assert_eq!(snapshot.parents[1].parent_role, "secondary");
    assert_eq!(snapshot.parents[1].parent_commit_id, source.commit_id);
}
