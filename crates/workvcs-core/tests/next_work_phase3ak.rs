use std::path::{Path, PathBuf};
use tempfile::TempDir;
use workvcs_core::{
    ClaimMode, ClaimTaskOptions, Engine, NextWorkOptions, RunnableTaskClaimCoordination,
    SessionStartOptions, StoreInitOptions, TaskCreateOptions, TaskStatus, WorkspaceInfo,
    WorkspaceInitOptions,
};

fn store_path() -> (TempDir, PathBuf) {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let path = tempdir.path().join("workvcs.sqlite");
    (tempdir, path)
}

fn init_engine(path: &Path) -> Engine {
    Engine::init(
        path,
        StoreInitOptions::new("phase3ak-next-work-store").expect("store options"),
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

#[test]
fn next_work_claims_focuses_and_returns_context() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let task = engine
        .create_task(
            TaskCreateOptions::new(
                workspace.initial_branch_id,
                workspace.genesis_commit_id,
                "Next work task",
            )
            .expect("task options"),
        )
        .expect("create task");
    let session = engine
        .start_session(
            SessionStartOptions::new(workspace.workspace_id, workspace.initial_branch_id)
                .expect("session options"),
        )
        .expect("start session");

    let next = engine
        .next_work(NextWorkOptions::new(session.session_id))
        .expect("next work");
    let selected = next.claim_next.selected.as_ref().expect("selected work");

    assert_eq!(selected.task_entity_id, task.task_entity_id);
    assert_eq!(
        next.context
            .session
            .focus
            .as_ref()
            .expect("post-next focus")
            .focus_entity_id,
        task.task_entity_id
    );
    assert_eq!(next.context.branch.head_commit_id, task.commit_id);
    assert_eq!(next.context.runnable_tasks.candidates.len(), 1);
    assert_eq!(
        next.context.runnable_tasks.candidates[0].task.state.status,
        TaskStatus::Pending
    );
    assert!(matches!(
        next.context.runnable_tasks.candidates[0].claim_coordination,
        RunnableTaskClaimCoordination::ClaimedBySession { claim_id }
            if claim_id == selected.claim_id
    ));
}

#[test]
fn next_work_returns_context_without_selection_when_no_runnable_task_exists() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let session = engine
        .start_session(
            SessionStartOptions::new(workspace.workspace_id, workspace.initial_branch_id)
                .expect("session options"),
        )
        .expect("start session");

    let next = engine
        .next_work(NextWorkOptions::new(session.session_id))
        .expect("next work without tasks");

    assert!(next.claim_next.selected.is_none());
    assert_eq!(next.claim_next.inspected_candidates, 0);
    assert!(next.context.session.focus.is_none());
    assert_eq!(next.context.runnable_tasks.candidates.len(), 0);
    assert_eq!(
        next.context.branch.head_commit_id,
        workspace.genesis_commit_id
    );
}

#[test]
fn shared_next_work_joins_shared_claim_and_returns_updated_context() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let task = engine
        .create_task(
            TaskCreateOptions::new(
                workspace.initial_branch_id,
                workspace.genesis_commit_id,
                "Shared next work task",
            )
            .expect("task options"),
        )
        .expect("create task");
    let first_session = engine
        .start_session(
            SessionStartOptions::new(workspace.workspace_id, workspace.initial_branch_id)
                .expect("first session options"),
        )
        .expect("start first session");
    let second_session = engine
        .start_session(
            SessionStartOptions::new(workspace.workspace_id, workspace.initial_branch_id)
                .expect("second session options"),
        )
        .expect("start second session");
    let first_claim = engine
        .claim_task(
            ClaimTaskOptions::new(first_session.session_id, task.task_entity_id)
                .with_mode(ClaimMode::Shared),
        )
        .expect("first shared claim");

    let next = engine
        .next_work(NextWorkOptions::new(second_session.session_id).with_mode(ClaimMode::Shared))
        .expect("shared next work");
    let selected = next.claim_next.selected.as_ref().expect("selected work");

    assert_eq!(selected.task_entity_id, task.task_entity_id);
    assert_eq!(selected.mode, ClaimMode::Shared);
    assert_eq!(
        next.context
            .session
            .focus
            .as_ref()
            .expect("post-next focus")
            .focus_entity_id,
        task.task_entity_id
    );
    match &next.context.runnable_tasks.candidates[0].claim_coordination {
        RunnableTaskClaimCoordination::Shared {
            claim_ids,
            session_ids,
            claimed_by_session,
        } => {
            assert!(*claimed_by_session);
            assert!(claim_ids.contains(&first_claim.claim_id));
            assert!(claim_ids.contains(&selected.claim_id));
            assert!(session_ids.contains(&first_session.session_id));
            assert!(session_ids.contains(&second_session.session_id));
        }
        other => panic!("expected shared coordination, got {other:?}"),
    }
}
