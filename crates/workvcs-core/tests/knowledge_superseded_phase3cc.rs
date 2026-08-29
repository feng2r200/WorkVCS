use std::path::{Path, PathBuf};
use tempfile::TempDir;
use workvcs_core::{
    ContextOverviewOptions, Engine, KnowledgeCreateOptions, KnowledgeListOptions, KnowledgeStatus,
    KnowledgeTransitionOptions, SessionStartOptions, StoreInitOptions, WorkspaceInfo,
    WorkspaceInitOptions,
};

fn store_path() -> (TempDir, PathBuf) {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let path = tempdir.path().join("workvcs.sqlite");
    (tempdir, path)
}

fn create_workspace(path: &Path) -> (Engine, WorkspaceInfo) {
    let mut engine = Engine::init(
        path,
        StoreInitOptions::new("phase3cc-knowledge-superseded-store").expect("store options"),
    )
    .expect("init engine");
    let workspace = engine
        .create_workspace(WorkspaceInitOptions::new("workspace").expect("workspace options"))
        .expect("create workspace");
    (engine, workspace)
}

#[test]
fn active_knowledge_can_be_marked_superseded() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let knowledge = engine
        .create_knowledge(
            KnowledgeCreateOptions::new(
                workspace.initial_branch_id,
                workspace.genesis_commit_id,
                "Use the old context summary format",
            )
            .expect("knowledge options"),
        )
        .expect("create knowledge");

    let superseded = engine
        .transition_knowledge(
            KnowledgeTransitionOptions::supersede(
                workspace.initial_branch_id,
                knowledge.commit_id,
                knowledge.knowledge_entity_id,
                knowledge.knowledge_entity_version_id,
                "The context summary format was replaced",
            )
            .expect("supersede options"),
        )
        .expect("supersede knowledge");
    assert_eq!(superseded.previous_state.status, KnowledgeStatus::Active);
    assert_eq!(superseded.state.status, KnowledgeStatus::Superseded);

    let historical = engine
        .knowledge_at(knowledge.commit_id, knowledge.knowledge_entity_id)
        .expect("historical knowledge");
    assert_eq!(historical.state.status, KnowledgeStatus::Active);
    let current = engine
        .knowledge_at(superseded.commit_id, knowledge.knowledge_entity_id)
        .expect("current knowledge");
    assert_eq!(current.state.status, KnowledgeStatus::Superseded);

    let active = engine
        .knowledges_at(
            KnowledgeListOptions::new(superseded.commit_id).with_status(KnowledgeStatus::Active),
        )
        .expect("active knowledge list");
    assert!(active.knowledge.is_empty());
    let superseded_list = engine
        .knowledges_at(
            KnowledgeListOptions::new(superseded.commit_id)
                .with_status(KnowledgeStatus::Superseded),
        )
        .expect("superseded knowledge list");
    assert_eq!(superseded_list.knowledge.len(), 1);
    assert_eq!(
        superseded_list.knowledge[0].knowledge_entity_id,
        knowledge.knowledge_entity_id
    );

    let session = engine
        .start_session(
            SessionStartOptions::new(workspace.workspace_id, workspace.initial_branch_id)
                .expect("session options"),
        )
        .expect("start session");
    let context = engine
        .context_overview(ContextOverviewOptions::new(session.session_id))
        .expect("context overview");
    assert!(context.knowledge.knowledge.is_empty());
}

#[test]
fn superseded_knowledge_cannot_be_invalidated_again() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let knowledge = engine
        .create_knowledge(
            KnowledgeCreateOptions::new(
                workspace.initial_branch_id,
                workspace.genesis_commit_id,
                "Use the old context summary format",
            )
            .expect("knowledge options"),
        )
        .expect("create knowledge");
    let superseded = engine
        .transition_knowledge(
            KnowledgeTransitionOptions::supersede(
                workspace.initial_branch_id,
                knowledge.commit_id,
                knowledge.knowledge_entity_id,
                knowledge.knowledge_entity_version_id,
                "The context summary format was replaced",
            )
            .expect("supersede options"),
        )
        .expect("supersede knowledge");

    let invalidated = engine.transition_knowledge(
        KnowledgeTransitionOptions::invalidate(
            workspace.initial_branch_id,
            superseded.commit_id,
            knowledge.knowledge_entity_id,
            superseded.knowledge_entity_version_id,
            "Trying to invalidate superseded Knowledge",
        )
        .expect("invalidate options"),
    );
    assert!(invalidated.is_err());
}
