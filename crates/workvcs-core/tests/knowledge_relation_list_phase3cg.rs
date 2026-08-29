use std::path::{Path, PathBuf};
use tempfile::TempDir;
use workvcs_core::{
    Engine, KnowledgeCreateOptions, KnowledgeRelationCreateCommit, KnowledgeRelationCreateOptions,
    KnowledgeRelationListOptions, KnowledgeTransitionOptions, StoreInitOptions, WorkspaceInfo,
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
        StoreInitOptions::new("phase3cg-knowledge-relation-list-store").expect("store options"),
    )
    .expect("init engine");
    let workspace = engine
        .create_workspace(WorkspaceInitOptions::new("workspace").expect("workspace options"))
        .expect("create workspace");
    (engine, workspace)
}

fn create_supersedes_relation(
    engine: &mut Engine,
    workspace: &WorkspaceInfo,
) -> KnowledgeRelationCreateCommit {
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

    engine
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
        .expect("create knowledge relation")
}

#[test]
fn knowledge_relation_list_returns_current_supersession_edges_with_filters() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let relation = create_supersedes_relation(&mut engine, &workspace);

    let all = engine
        .knowledge_relations_at(KnowledgeRelationListOptions::new(relation.commit_id))
        .expect("all knowledge relations");
    assert_eq!(all.workspace_id, workspace.workspace_id);
    assert_eq!(all.commit_id, relation.commit_id);
    assert_eq!(all.relations.len(), 1);
    assert_eq!(all.relations[0].relation_id, relation.relation_id);

    let by_replacement = engine
        .knowledge_relations_at(
            KnowledgeRelationListOptions::new(relation.commit_id)
                .with_replacement_knowledge(relation.replacement_knowledge_entity_id),
        )
        .expect("replacement-filtered knowledge relations");
    assert_eq!(by_replacement.relations.len(), 1);
    assert_eq!(
        by_replacement.relations[0].relation_id,
        relation.relation_id
    );

    let by_prior = engine
        .knowledge_relations_at(
            KnowledgeRelationListOptions::new(relation.commit_id)
                .with_prior_knowledge(relation.prior_knowledge_entity_id),
        )
        .expect("prior-filtered knowledge relations");
    assert_eq!(by_prior.relations.len(), 1);
    assert_eq!(by_prior.relations[0].relation_id, relation.relation_id);

    let absent = engine
        .knowledge_relations_at(
            KnowledgeRelationListOptions::new(relation.commit_id)
                .with_replacement_knowledge(relation.prior_knowledge_entity_id),
        )
        .expect("absent knowledge relations");
    assert!(absent.relations.is_empty());
}
