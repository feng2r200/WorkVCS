use std::path::{Path, PathBuf};
use tempfile::TempDir;
use workvcs_core::{
    Engine, KnowledgeCreateOptions, RecordCreateOptions, RecordKnowledgeRelationCreateOptions,
    RecordKnowledgeRelationListOptions, RecordKnowledgeRelationRemoveOptions,
    RecordKnowledgeRelationRestoreOptions, StoreInitOptions, WorkspaceInfo, WorkspaceInitOptions,
};

fn store_path() -> (TempDir, PathBuf) {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let path = tempdir.path().join("workvcs.sqlite");
    (tempdir, path)
}

fn create_workspace(path: &Path) -> (Engine, WorkspaceInfo) {
    let mut engine = Engine::init(
        path,
        StoreInitOptions::new("phase3ca-record-knowledge-relation-restore-store")
            .expect("store options"),
    )
    .expect("init engine");
    let workspace = engine
        .create_workspace(WorkspaceInitOptions::new("workspace").expect("workspace options"))
        .expect("create workspace");
    (engine, workspace)
}

#[test]
fn record_knowledge_relation_restore_restores_removed_edge() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let knowledge = engine
        .create_knowledge(
            KnowledgeCreateOptions::new(
                workspace.initial_branch_id,
                workspace.genesis_commit_id,
                "A Finding supports this reusable Knowledge",
            )
            .expect("knowledge options"),
        )
        .expect("create knowledge");
    let finding = engine
        .create_record(
            RecordCreateOptions::finding(
                workspace.initial_branch_id,
                knowledge.commit_id,
                "The observed output supports the Knowledge",
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
                "Remove the mistaken relation",
            )
            .expect("remove options"),
        )
        .expect("remove record knowledge relation");
    assert!(
        engine
            .record_knowledge_relations_at(RecordKnowledgeRelationListOptions::new(
                removed.commit_id
            ))
            .expect("list after removal")
            .relations
            .is_empty()
    );

    let restored = engine
        .restore_record_knowledge_relation(
            RecordKnowledgeRelationRestoreOptions::new(
                workspace.initial_branch_id,
                removed.commit_id,
                relation.relation_id,
                relation.relation_version_id,
                "Restore the relation after review",
            )
            .expect("restore options"),
        )
        .expect("restore record knowledge relation");
    assert_eq!(restored.previous_head_commit_id, removed.commit_id);
    assert_eq!(restored.relation_id, relation.relation_id);
    assert_eq!(restored.relation_version_id, relation.relation_version_id);
    assert_eq!(restored.relation_type, relation.relation_type);
    assert_eq!(
        restored.source_record_entity_id,
        relation.source_record_entity_id
    );
    assert_eq!(
        restored.target_knowledge_entity_id,
        relation.target_knowledge_entity_id
    );
    assert_eq!(
        restored.relation_state_digest,
        relation.relation_state_digest
    );

    let current = engine
        .record_knowledge_relation_at(restored.commit_id, relation.relation_id)
        .expect("show restored relation");
    assert_eq!(current.relation_version_id, relation.relation_version_id);
    assert_eq!(current.state_digest, relation.relation_state_digest);
}
