use std::path::{Path, PathBuf};
use tempfile::TempDir;
use workvcs_core::{
    ContextOverviewOptions, Engine, ErrorCategory, ErrorCode, KnowledgeCreateOptions,
    KnowledgeTransitionOptions, RecordCreateOptions, RecordRelationCreateOptions,
    RecordRelationType, SessionEndOptions, SessionFocusOptions, SessionStartOptions,
    StoreInitOptions, TaskCreateOptions, WorkspaceInfo, WorkspaceInitOptions,
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
    assert!(context.knowledge.knowledge.is_empty());
    assert!(context.records.records.is_empty());
    assert!(context.record_relations.relations.is_empty());
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
    assert!(context.knowledge.knowledge.is_empty());
    assert!(context.records.records.is_empty());
    assert!(context.record_relations.relations.is_empty());
}

#[test]
fn context_overview_includes_current_record_summaries() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let finding = engine
        .create_record(
            RecordCreateOptions::finding(
                workspace.initial_branch_id,
                workspace.genesis_commit_id,
                "The current context needs explicit findings",
            )
            .expect("finding options"),
        )
        .expect("create finding");
    let session = engine
        .start_session(
            SessionStartOptions::new(workspace.workspace_id, workspace.initial_branch_id)
                .expect("session options"),
        )
        .expect("start session");

    let context = engine
        .context_overview(ContextOverviewOptions::new(session.session_id))
        .expect("context overview");

    assert_eq!(context.records.workspace_id, workspace.workspace_id);
    assert_eq!(context.records.commit_id, finding.commit_id);
    assert_eq!(context.records.records.len(), 1);
    assert_eq!(
        context.records.records[0].record_entity_id,
        finding.record_entity_id
    );
    assert_eq!(context.records.records[0].state.kind, finding.state.kind);
    assert_eq!(
        context.records.records[0].state.status,
        finding.state.status
    );
    assert_eq!(
        context.records.records[0].state.statement,
        finding.state.statement
    );
    assert!(context.knowledge.knowledge.is_empty());
    assert!(context.record_relations.relations.is_empty());
}

#[test]
fn context_overview_includes_active_knowledge_summaries() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let active = engine
        .create_knowledge(
            KnowledgeCreateOptions::new(
                workspace.initial_branch_id,
                workspace.genesis_commit_id,
                "Serialized writes make SQLite sufficient for V0.1",
            )
            .expect("active knowledge options"),
        )
        .expect("create active knowledge");
    let stale = engine
        .create_knowledge(
            KnowledgeCreateOptions::new(
                workspace.initial_branch_id,
                active.commit_id,
                "Legacy cache keys do not need schema versions",
            )
            .expect("stale knowledge options"),
        )
        .expect("create stale knowledge");
    let invalidated = engine
        .transition_knowledge(
            KnowledgeTransitionOptions::invalidate(
                workspace.initial_branch_id,
                stale.commit_id,
                stale.knowledge_entity_id,
                stale.knowledge_entity_version_id,
                "Schema versions are part of cache keys",
            )
            .expect("invalidate options"),
        )
        .expect("invalidate stale knowledge");
    let session = engine
        .start_session(
            SessionStartOptions::new(workspace.workspace_id, workspace.initial_branch_id)
                .expect("session options"),
        )
        .expect("start session");

    let context = engine
        .context_overview(ContextOverviewOptions::new(session.session_id))
        .expect("context overview");

    assert_eq!(context.knowledge.workspace_id, workspace.workspace_id);
    assert_eq!(context.knowledge.commit_id, invalidated.commit_id);
    assert_eq!(context.knowledge.knowledge.len(), 1);
    assert_eq!(
        context.knowledge.knowledge[0].knowledge_entity_id,
        active.knowledge_entity_id
    );
    assert_eq!(
        context.knowledge.knowledge[0].state.statement,
        active.state.statement
    );
    assert!(context.records.records.is_empty());
    assert!(context.record_relations.relations.is_empty());
}

#[test]
fn context_overview_includes_current_record_relation_summaries() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let decision = engine
        .create_record(
            RecordCreateOptions::decision(
                workspace.initial_branch_id,
                workspace.genesis_commit_id,
                "Use serialized writes",
            )
            .expect("decision options"),
        )
        .expect("create decision");
    let finding = engine
        .create_record(
            RecordCreateOptions::finding(
                workspace.initial_branch_id,
                decision.commit_id,
                "Benchmarks support serialized writes",
            )
            .expect("finding options"),
        )
        .expect("create finding");
    let relation = engine
        .create_record_relation(
            RecordRelationCreateOptions::supports(
                workspace.initial_branch_id,
                finding.commit_id,
                finding.record_entity_id,
                decision.record_entity_id,
                "Finding supports decision",
            )
            .expect("relation options"),
        )
        .expect("create supports relation");
    let session = engine
        .start_session(
            SessionStartOptions::new(workspace.workspace_id, workspace.initial_branch_id)
                .expect("session options"),
        )
        .expect("start session");

    let context = engine
        .context_overview(ContextOverviewOptions::new(session.session_id))
        .expect("context overview");

    assert_eq!(context.records.records.len(), 2);
    assert_eq!(
        context.record_relations.workspace_id,
        workspace.workspace_id
    );
    assert_eq!(context.record_relations.commit_id, relation.commit_id);
    assert_eq!(context.record_relations.relations.len(), 1);
    assert_eq!(
        context.record_relations.relations[0].relation_id,
        relation.relation_id
    );
    assert_eq!(
        context.record_relations.relations[0].relation_type,
        RecordRelationType::Supports
    );
    assert_eq!(
        context.record_relations.relations[0].source_record_entity_id,
        finding.record_entity_id
    );
    assert_eq!(
        context.record_relations.relations[0].target_record_entity_id,
        decision.record_entity_id
    );
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
