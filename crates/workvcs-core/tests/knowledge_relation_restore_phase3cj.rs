use rusqlite::{Connection, params};
use std::path::{Path, PathBuf};
use tempfile::TempDir;
use workvcs_core::{
    Engine, ErrorCode, KnowledgeCreateOptions, KnowledgeRelationCreateCommit,
    KnowledgeRelationCreateOptions, KnowledgeRelationListOptions, KnowledgeRelationRemoveOptions,
    KnowledgeRelationRestoreOptions, KnowledgeTransitionOptions, StoreInitOptions, WhyQueryOptions,
    WhyQueryTarget, WorkspaceInfo, WorkspaceInitOptions,
};

fn store_path() -> (TempDir, PathBuf) {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let path = tempdir.path().join("workvcs.sqlite");
    (tempdir, path)
}

fn create_workspace(path: &Path) -> (Engine, WorkspaceInfo) {
    let mut engine = Engine::init(
        path,
        StoreInitOptions::new("phase3cj-knowledge-relation-restore-store").expect("store options"),
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
fn knowledge_relation_restore_restores_removed_edge_without_rewriting_history() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let relation = create_supersedes_relation(&mut engine, &workspace);

    let duplicate_restore = engine
        .restore_knowledge_relation(
            KnowledgeRelationRestoreOptions::new(
                workspace.initial_branch_id,
                relation.commit_id,
                relation.relation_id,
                relation.relation_version_id,
                "Restore the already-present Knowledge relation",
            )
            .expect("duplicate restore options"),
        )
        .expect_err("present knowledge relation cannot be restored again");
    assert_eq!(duplicate_restore.code(), ErrorCode::KnowledgeInvalid);

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
    assert!(
        engine
            .knowledge_relations_at(KnowledgeRelationListOptions::new(removed.commit_id))
            .expect("list after removal")
            .relations
            .is_empty()
    );

    let restored = engine
        .restore_knowledge_relation(
            KnowledgeRelationRestoreOptions::new(
                workspace.initial_branch_id,
                removed.commit_id,
                relation.relation_id,
                relation.relation_version_id,
                "Restore the corrected Knowledge lineage edge",
            )
            .expect("restore options"),
        )
        .expect("restore knowledge relation");
    assert_eq!(restored.previous_head_commit_id, removed.commit_id);
    assert_eq!(restored.relation_id, relation.relation_id);
    assert_eq!(restored.relation_version_id, relation.relation_version_id);
    assert_eq!(restored.relation_type, relation.relation_type);
    assert_eq!(
        restored.replacement_knowledge_entity_id,
        relation.replacement_knowledge_entity_id
    );
    assert_eq!(
        restored.prior_knowledge_entity_id,
        relation.prior_knowledge_entity_id
    );
    assert_eq!(
        restored.relation_state_digest,
        relation.relation_state_digest
    );

    let current = engine
        .knowledge_relation_at(restored.commit_id, relation.relation_id)
        .expect("show restored relation");
    assert_eq!(current.relation_version_id, relation.relation_version_id);
    assert_eq!(current.state_digest, relation.relation_state_digest);

    let why = engine
        .why(WhyQueryOptions::for_entity(
            WhyQueryTarget::commit(restored.commit_id),
            relation.prior_knowledge_entity_id,
        ))
        .expect("why after restore");
    assert_eq!(why.relation_edges.len(), 1);

    let historical = engine
        .knowledge_relation_at(relation.commit_id, relation.relation_id)
        .expect("historical knowledge relation remains readable");
    assert_eq!(historical.relation_version_id, relation.relation_version_id);

    let connection = Connection::open(&path).expect("open sqlite");
    let operation_type: String = connection
        .query_row(
            "SELECT operation_type FROM changeset WHERE changeset_id = ?1",
            params![&restored.changeset_id.raw_bytes()[..]],
            |row| row.get(0),
        )
        .expect("knowledge relation restore changeset");
    assert_eq!(operation_type, "knowledge.relation.restore");
}
