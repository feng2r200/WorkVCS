use std::path::{Path, PathBuf};
use tempfile::TempDir;
use workvcs_core::{
    Engine, KnowledgeCreateOptions, KnowledgeRelationCreateOptions, KnowledgeTransitionOptions,
    StoreInitOptions, WorkspaceInfo, WorkspaceInitOptions,
};

fn store_path() -> (TempDir, PathBuf) {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let path = tempdir.path().join("workvcs.sqlite");
    (tempdir, path)
}

fn create_workspace(path: &Path) -> (Engine, WorkspaceInfo) {
    let mut engine = Engine::init(
        path,
        StoreInitOptions::new("phase3ch-knowledge-relation-show-store").expect("store options"),
    )
    .expect("init engine");
    let workspace = engine
        .create_workspace(WorkspaceInitOptions::new("workspace").expect("workspace options"))
        .expect("create workspace");
    (engine, workspace)
}

#[test]
fn knowledge_relation_at_shows_current_supersession_edge() {
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

    let snapshot = engine
        .knowledge_relation_at(relation.commit_id, relation.relation_id)
        .expect("knowledge relation snapshot");
    assert_eq!(snapshot.workspace_id, workspace.workspace_id);
    assert_eq!(snapshot.commit_id, relation.commit_id);
    assert_eq!(snapshot.relation_id, relation.relation_id);
    assert_eq!(snapshot.relation_version_id, relation.relation_version_id);
    assert_eq!(
        snapshot.replacement_knowledge_entity_id,
        replacement.knowledge_entity_id
    );
    assert_eq!(
        snapshot.prior_knowledge_entity_id,
        prior.knowledge_entity_id
    );
    assert_eq!(snapshot.state_digest, relation.relation_state_digest);
}
