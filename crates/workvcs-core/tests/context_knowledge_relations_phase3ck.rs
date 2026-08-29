use std::path::{Path, PathBuf};
use tempfile::TempDir;
use workvcs_core::{
    ContextOverviewOptions, Engine, KnowledgeCreateOptions, KnowledgeRelationCreateOptions,
    KnowledgeTransitionOptions, RecordRelationType, SessionStartOptions, StoreInitOptions,
    WorkspaceInfo, WorkspaceInitOptions,
};

fn store_path() -> (TempDir, PathBuf) {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let path = tempdir.path().join("workvcs.sqlite");
    (tempdir, path)
}

fn create_workspace(path: &Path) -> (Engine, WorkspaceInfo) {
    let mut engine = Engine::init(
        path,
        StoreInitOptions::new("phase3ck-context-knowledge-relations-store").expect("store options"),
    )
    .expect("init engine");
    let workspace = engine
        .create_workspace(WorkspaceInitOptions::new("workspace").expect("workspace options"))
        .expect("create workspace");
    (engine, workspace)
}

#[test]
fn context_overview_includes_current_knowledge_relations() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let prior = engine
        .create_knowledge(
            KnowledgeCreateOptions::new(
                workspace.initial_branch_id,
                workspace.genesis_commit_id,
                "Use the old context summary format",
            )
            .expect("prior knowledge options"),
        )
        .expect("create prior knowledge");
    let replacement = engine
        .create_knowledge(
            KnowledgeCreateOptions::new(
                workspace.initial_branch_id,
                prior.commit_id,
                "Use the scoped context summary format",
            )
            .expect("replacement knowledge options"),
        )
        .expect("create replacement knowledge");
    let superseded = engine
        .transition_knowledge(
            KnowledgeTransitionOptions::supersede(
                workspace.initial_branch_id,
                replacement.commit_id,
                prior.knowledge_entity_id,
                prior.knowledge_entity_version_id,
                "The scoped context summary format replaced it",
            )
            .expect("supersede prior options"),
        )
        .expect("supersede prior knowledge");
    let relation = engine
        .create_knowledge_relation(
            KnowledgeRelationCreateOptions::supersedes(
                workspace.initial_branch_id,
                superseded.commit_id,
                replacement.knowledge_entity_id,
                prior.knowledge_entity_id,
                "The new Knowledge replaces the prior statement",
            )
            .expect("knowledge relation options"),
        )
        .expect("create knowledge relation");
    let session = engine
        .start_session(
            SessionStartOptions::new(workspace.workspace_id, workspace.initial_branch_id)
                .expect("session options"),
        )
        .expect("start session");

    let context = engine
        .context_overview(ContextOverviewOptions::new(session.session_id))
        .expect("context overview");

    assert_eq!(context.knowledge.knowledge.len(), 1);
    assert_eq!(
        context.knowledge.knowledge[0].knowledge_entity_id,
        replacement.knowledge_entity_id
    );
    assert_eq!(
        context.knowledge_relations.workspace_id,
        workspace.workspace_id
    );
    assert_eq!(context.knowledge_relations.commit_id, relation.commit_id);
    assert_eq!(context.knowledge_relations.relations.len(), 1);
    assert_eq!(
        context.knowledge_relations.relations[0].relation_id,
        relation.relation_id
    );
    assert_eq!(
        context.knowledge_relations.relations[0].relation_type,
        RecordRelationType::Supersedes
    );
    assert_eq!(
        context.knowledge_relations.relations[0].replacement_knowledge_entity_id,
        replacement.knowledge_entity_id
    );
    assert_eq!(
        context.knowledge_relations.relations[0].prior_knowledge_entity_id,
        prior.knowledge_entity_id
    );
}
