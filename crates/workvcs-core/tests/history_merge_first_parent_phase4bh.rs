use tempfile::TempDir;
use workvcs_core::{
    BranchForkOptions, BranchId, CanonicalValue, CommitId, Engine, HistoryQueryOptions,
    MergeContinueOptions, MergeFreezeResolutionsOptions, MergeResolveOptions, MergeStartOptions,
    StoreInitOptions, TaskCreateCommit, TaskCreateOptions, WorkspaceInfo, WorkspaceInitOptions,
};

fn create_store() -> (TempDir, Engine, WorkspaceInfo) {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let path = tempdir.path().join("workvcs.sqlite");
    let mut engine = Engine::init(
        &path,
        StoreInitOptions::new("phase4bh-merge-history-store").expect("store options"),
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
fn history_follows_primary_parent_from_merge_commit() {
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

    let history = engine
        .history(HistoryQueryOptions::from_commit(continued.result_commit_id))
        .expect("history from merge");

    let commit_ids = history
        .entries
        .iter()
        .map(|entry| entry.commit_id)
        .collect::<Vec<_>>();
    assert_eq!(
        commit_ids,
        vec![
            continued.result_commit_id,
            target.commit_id,
            base.commit_id,
            workspace.genesis_commit_id,
        ]
    );
    assert_eq!(history.entries[0].commit_kind, "merge");
    assert_eq!(history.entries[0].parent_commit_id, Some(target.commit_id));
    assert!(!commit_ids.contains(&source.commit_id));

    let branch_history = engine
        .history(
            HistoryQueryOptions::from_branch(workspace.initial_branch_id)
                .with_limit(1)
                .unwrap(),
        )
        .expect("branch history from merge head");
    assert_eq!(
        branch_history.entries[0].commit_id,
        continued.result_commit_id
    );
}
