use std::path::{Path, PathBuf};
use tempfile::TempDir;
use workvcs_core::{
    Engine, KnowledgeCreateOptions, RecordCreateOptions, RecordKnowledgeRelationCreateOptions,
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
        StoreInitOptions::new("phase3by-record-knowledge-relation-show-store")
            .expect("store options"),
    )
    .expect("init engine");
    let workspace = engine
        .create_workspace(WorkspaceInitOptions::new("workspace").expect("workspace options"))
        .expect("create workspace");
    (engine, workspace)
}

#[test]
fn record_knowledge_relation_at_shows_current_edge() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let knowledge = engine
        .create_knowledge(
            KnowledgeCreateOptions::new(
                workspace.initial_branch_id,
                workspace.genesis_commit_id,
                "A local store can explain reusable Knowledge",
            )
            .expect("knowledge options"),
        )
        .expect("create knowledge");
    let finding = engine
        .create_record(
            RecordCreateOptions::finding(
                workspace.initial_branch_id,
                knowledge.commit_id,
                "The why command shows incoming Record-to-Knowledge edges",
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
                "The Finding supports the Knowledge explanation",
            )
            .expect("supports knowledge options"),
        )
        .expect("support knowledge relation");

    let before_relation =
        engine.record_knowledge_relation_at(finding.commit_id, relation.relation_id);
    assert!(before_relation.is_err());

    let shown = engine
        .record_knowledge_relation_at(relation.commit_id, relation.relation_id)
        .expect("show record knowledge relation");
    assert_eq!(shown.workspace_id, workspace.workspace_id);
    assert_eq!(shown.commit_id, relation.commit_id);
    assert_eq!(shown.relation_id, relation.relation_id);
    assert_eq!(shown.relation_version_id, relation.relation_version_id);
    assert_eq!(shown.relation_type, relation.relation_type);
    assert_eq!(
        shown.source_record_entity_id,
        relation.source_record_entity_id
    );
    assert_eq!(
        shown.target_knowledge_entity_id,
        relation.target_knowledge_entity_id
    );
    assert_eq!(shown.state_digest, relation.relation_state_digest);
}
