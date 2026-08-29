use rusqlite::{Connection, params};
use std::path::{Path, PathBuf};
use tempfile::TempDir;
use workvcs_core::{
    Engine, ErrorCode, KnowledgeCreateOptions, KnowledgeRelationCreateCommit,
    KnowledgeRelationCreateOptions, KnowledgeRelationListOptions, KnowledgeRelationRemoveOptions,
    KnowledgeTransitionOptions, StoreInitOptions, WhyQueryOptions, WhyQueryTarget, WorkspaceInfo,
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
        StoreInitOptions::new("phase3ci-knowledge-relation-removal-store").expect("store options"),
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
fn knowledge_relation_remove_drops_edge_from_current_workstate_without_deleting_history() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let relation = create_supersedes_relation(&mut engine, &workspace);

    let removed = engine
        .remove_knowledge_relation(
            KnowledgeRelationRemoveOptions::new(
                workspace.initial_branch_id,
                relation.commit_id,
                relation.relation_id,
                relation.relation_version_id,
                "The supersession lineage was recorded against the wrong prior Knowledge",
            )
            .expect("remove options"),
        )
        .expect("remove knowledge relation");

    assert_eq!(removed.workspace_id, workspace.workspace_id);
    assert_eq!(removed.previous_head_commit_id, relation.commit_id);
    assert_eq!(removed.relation_id, relation.relation_id);
    assert_eq!(
        removed.previous_relation_version_id,
        relation.relation_version_id
    );
    assert_eq!(
        removed.replacement_knowledge_entity_id,
        relation.replacement_knowledge_entity_id
    );
    assert_eq!(
        removed.prior_knowledge_entity_id,
        relation.prior_knowledge_entity_id
    );

    let current = engine
        .knowledge_relations_at(KnowledgeRelationListOptions::new(removed.commit_id))
        .expect("current knowledge relations");
    assert!(current.relations.is_empty());

    let missing = engine
        .knowledge_relation_at(removed.commit_id, relation.relation_id)
        .expect_err("removed relation should not be present");
    assert_eq!(missing.code(), ErrorCode::KnowledgeNotFound);

    let historical = engine
        .knowledge_relation_at(relation.commit_id, relation.relation_id)
        .expect("historical knowledge relation");
    assert_eq!(historical.relation_id, relation.relation_id);

    let why = engine
        .why(WhyQueryOptions::for_entity(
            WhyQueryTarget::commit(removed.commit_id),
            relation.prior_knowledge_entity_id,
        ))
        .expect("why after removal");
    assert!(why.relation_edges.is_empty());

    let connection = Connection::open(&path).expect("open sqlite");
    let operation_type: String = connection
        .query_row(
            "SELECT operation_type FROM changeset WHERE changeset_id = ?1",
            params![&removed.changeset_id.raw_bytes()[..]],
            |row| row.get(0),
        )
        .expect("knowledge relation removal changeset");
    assert_eq!(operation_type, "knowledge.relation.remove");
}
