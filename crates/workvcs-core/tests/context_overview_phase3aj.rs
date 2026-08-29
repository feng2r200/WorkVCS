use std::path::{Path, PathBuf};
use tempfile::TempDir;
use workvcs_core::{
    ContextOverviewOptions, Engine, ErrorCategory, ErrorCode, SessionEndOptions,
    SessionFocusOptions, SessionStartOptions, StoreInitOptions, TaskCreateOptions, WorkspaceInfo,
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
        StoreInitOptions::new("phase3aj-context-overview-store").expect("store options"),
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
fn context_overview_returns_active_anchor_focus_and_runnable_summary() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let task = engine
        .create_task(
            TaskCreateOptions::new(
                workspace.initial_branch_id,
                workspace.genesis_commit_id,
                "Focused context task",
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
    engine
        .set_session_focus(SessionFocusOptions::new(
            session.session_id,
            task.task_entity_id,
        ))
        .expect("set focus");

    let context = engine
        .context_overview(ContextOverviewOptions::new(session.session_id))
        .expect("context overview");

    assert_eq!(context.session.session_id, session.session_id);
    assert_eq!(context.branch.workspace_id, workspace.workspace_id);
    assert_eq!(context.branch.branch_id, workspace.initial_branch_id);
    assert_eq!(context.branch.head_commit_id, task.commit_id);
    assert_eq!(context.branch.state_digest, task.work_state_digest);
    assert_eq!(
        context.session.context_workspaces,
        vec![workspace.workspace_id]
    );
    assert_eq!(
        context
            .session
            .focus
            .as_ref()
            .expect("session focus")
            .focus_entity_id,
        task.task_entity_id
    );
    assert_eq!(context.runnable_tasks.candidates.len(), 1);
    assert_eq!(
        context.runnable_tasks.candidates[0].task.task_entity_id,
        task.task_entity_id
    );
    assert!(context.runnable_tasks.candidates[0].runnable);
}

#[test]
fn context_overview_is_read_only() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let task = engine
        .create_task(
            TaskCreateOptions::new(
                workspace.initial_branch_id,
                workspace.genesis_commit_id,
                "Read only context task",
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
    let before = engine
        .branch_head(workspace.initial_branch_id)
        .expect("branch before context");

    let context = engine
        .context_overview(ContextOverviewOptions::new(session.session_id))
        .expect("context overview");

    let after = engine
        .branch_head(workspace.initial_branch_id)
        .expect("branch after context");
    assert_eq!(after, before);
    assert_eq!(context.branch.head_commit_id, task.commit_id);
}

#[test]
fn context_overview_rejects_ended_session() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let session = engine
        .start_session(
            SessionStartOptions::new(workspace.workspace_id, workspace.initial_branch_id)
                .expect("session options"),
        )
        .expect("start session");
    engine
        .end_session(SessionEndOptions::new(session.session_id).expect("end options"))
        .expect("end session");

    let error = engine
        .context_overview(ContextOverviewOptions::new(session.session_id))
        .expect_err("ended session context");
    assert_eq!(error.code(), ErrorCode::SessionInvalid);
    assert_eq!(error.category(), ErrorCategory::Runtime);
}
