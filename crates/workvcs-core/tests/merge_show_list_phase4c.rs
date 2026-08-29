use tempfile::TempDir;
use workvcs_core::{
    BranchForkOptions, BranchId, CanonicalValue, CommitId, Engine, ErrorCode, MergeAbortOptions,
    MergeListOptions, MergeOutcome, MergeStartOptions, SessionStartOptions, StoreInitOptions,
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
        StoreInitOptions::new("phase4c-merge-store").expect("store options"),
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
) -> CommitId {
    engine
        .create_task(
            TaskCreateOptions::new(branch_id, head_commit_id, description).expect("task options"),
        )
        .expect("create task")
        .commit_id
}

fn detail() -> CanonicalValue {
    CanonicalValue::object(vec![(
        "reason".to_owned(),
        CanonicalValue::String("superseded by another attempt".to_owned()),
    )])
    .expect("detail")
}

#[test]
fn show_and_list_merge_attempts_track_active_and_closed_states() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);

    let base_commit = create_task(
        &mut engine,
        workspace.initial_branch_id,
        workspace.genesis_commit_id,
        "shared merge base",
    );
    let source_branch = engine
        .fork_branch(
            BranchForkOptions::from_branch(workspace.initial_branch_id, "source")
                .expect("fork options"),
        )
        .expect("fork source branch");
    let target_head = create_task(
        &mut engine,
        workspace.initial_branch_id,
        base_commit,
        "target branch work",
    );
    let source_head = create_task(
        &mut engine,
        source_branch.branch_id,
        base_commit,
        "source branch work",
    );
    let session = engine
        .start_session(
            SessionStartOptions::new(workspace.workspace_id, workspace.initial_branch_id)
                .expect("session options"),
        )
        .expect("start session");
    let first = engine
        .start_merge(
            MergeStartOptions::new(workspace.initial_branch_id, source_branch.branch_id)
                .with_origin_session_id(session.session_id),
        )
        .expect("start merge");

    let active = engine.merge_attempt(first.merge_id).expect("show active");
    assert_eq!(active.merge_id, first.merge_id);
    assert_eq!(active.workspace_id, workspace.workspace_id);
    assert_eq!(active.target_branch_id, workspace.initial_branch_id);
    assert_eq!(active.source_branch_id, source_branch.branch_id);
    assert_eq!(active.merge_base_commit_id, base_commit);
    assert_eq!(active.target_head_commit_id, target_head);
    assert_eq!(active.source_head_commit_id, source_head);
    assert_eq!(active.origin_session_id, Some(session.session_id));
    assert_eq!(active.runtime_state.as_str(), "active");
    assert!(active.outcome.is_none());

    let active_list = engine
        .merge_attempts(MergeListOptions::new(workspace.workspace_id))
        .expect("list active");
    assert_eq!(active_list.merges.len(), 1);
    assert_eq!(active_list.merges[0].merge_id, first.merge_id);

    let target_list = engine
        .merge_attempts(
            MergeListOptions::new(workspace.workspace_id)
                .with_target_branch(workspace.initial_branch_id),
        )
        .expect("list target active");
    assert_eq!(target_list.merges.len(), 1);

    let detail = detail();
    engine
        .abort_merge(
            MergeAbortOptions::new(first.merge_id)
                .expect("abort options")
                .with_detail(detail.clone())
                .expect("detail"),
        )
        .expect("abort first merge");

    let hidden_closed = engine
        .merge_attempts(MergeListOptions::new(workspace.workspace_id))
        .expect("closed hidden");
    assert!(hidden_closed.merges.is_empty());

    let closed = engine.merge_attempt(first.merge_id).expect("show closed");
    assert_eq!(closed.runtime_state.as_str(), "aborted");
    let outcome = closed.outcome.expect("closed outcome");
    assert_eq!(outcome.outcome, MergeOutcome::Aborted);
    assert!(outcome.result_commit_id.is_none());
    assert_eq!(outcome.detail, detail);

    let replacement = engine
        .start_merge(MergeStartOptions::new(
            workspace.initial_branch_id,
            source_branch.branch_id,
        ))
        .expect("replacement merge");
    let all = engine
        .merge_attempts(MergeListOptions::new(workspace.workspace_id).include_closed())
        .expect("list all");
    assert_eq!(all.merges.len(), 2);
    assert_eq!(all.merges[0].merge_id, first.merge_id);
    assert_eq!(all.merges[0].runtime_state.as_str(), "aborted");
    assert_eq!(all.merges[1].merge_id, replacement.merge_id);
    assert_eq!(all.merges[1].runtime_state.as_str(), "active");

    let missing = engine
        .merge_attempt(workvcs_core::MergeId::new_v7())
        .expect_err("missing merge should fail");
    assert_eq!(missing.code(), ErrorCode::WorkspaceInvalid);
}
