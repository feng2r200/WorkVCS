use std::path::{Path, PathBuf};
use tempfile::TempDir;
use workvcs_core::{
    Engine, KnowledgeCreateOptions, RecordCreateOptions, RecordKnowledgeRelationCreateOptions,
    RecordKnowledgeRelationListOptions, RecordKnowledgeRelationRemoveOptions, StoreInitOptions,
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
        StoreInitOptions::new("phase3bz-record-knowledge-relation-removal-store")
            .expect("store options"),
    )
    .expect("init engine");
    let workspace = engine
        .create_workspace(WorkspaceInitOptions::new("workspace").expect("workspace options"))
        .expect("create workspace");
    (engine, workspace)
}

#[test]
fn record_knowledge_relation_remove_removes_current_edge() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let knowledge = engine
        .create_knowledge(
            KnowledgeCreateOptions::new(
                workspace.initial_branch_id,
                workspace.genesis_commit_id,
                "Context overviews show Knowledge relation evidence",
            )
            .expect("knowledge options"),
        )
        .expect("create knowledge");
    let finding = engine
        .create_record(
            RecordCreateOptions::finding(
                workspace.initial_branch_id,
                knowledge.commit_id,
                "The relation evidence is obsolete",
            )
            .expect("finding options"),
        )
        .expect("create finding");
    let relation = engine
        .create_record_knowledge_relation(
            RecordKnowledgeRelationCreateOptions::supports(
                workspace.initial_branch_id,
                finding.commit_id,
                finding.record_entity_id,
                knowledge.knowledge_entity_id,
                "The Finding supports the Knowledge statement",
            )
            .expect("supports knowledge options"),
        )
        .expect("support knowledge relation");

    let removed = engine
        .remove_record_knowledge_relation(
            RecordKnowledgeRelationRemoveOptions::new(
                workspace.initial_branch_id,
                relation.commit_id,
                relation.relation_id,
                relation.relation_version_id,
                "The Knowledge support edge was created by mistake",
            )
            .expect("remove options"),
        )
        .expect("remove record knowledge relation");
    assert_eq!(removed.previous_head_commit_id, relation.commit_id);
    assert_eq!(removed.relation_id, relation.relation_id);
    assert_eq!(
        removed.previous_relation_version_id,
        relation.relation_version_id
    );
    assert_eq!(removed.relation_type, relation.relation_type);
    assert_eq!(
        removed.source_record_entity_id,
        relation.source_record_entity_id
    );
    assert_eq!(
        removed.target_knowledge_entity_id,
        relation.target_knowledge_entity_id
    );

    let historical = engine
        .record_knowledge_relation_at(relation.commit_id, relation.relation_id)
        .expect("historical relation remains visible");
    assert_eq!(historical.relation_id, relation.relation_id);

    let current = engine
        .record_knowledge_relations_at(RecordKnowledgeRelationListOptions::new(removed.commit_id))
        .expect("list after removal");
    assert!(current.relations.is_empty());
    assert!(
        engine
            .record_knowledge_relation_at(removed.commit_id, relation.relation_id)
            .is_err()
    );
}
