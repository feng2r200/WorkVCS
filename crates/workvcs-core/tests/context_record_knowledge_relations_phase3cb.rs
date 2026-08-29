use std::path::{Path, PathBuf};
use tempfile::TempDir;
use workvcs_core::{
    ContextOverviewOptions, Engine, KnowledgeCreateOptions, RecordCreateOptions,
    RecordKnowledgeRelationCreateOptions, RecordRelationType, SessionStartOptions,
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
        StoreInitOptions::new("phase3cb-context-record-knowledge-relations-store")
            .expect("store options"),
    )
    .expect("init engine");
    let workspace = engine
        .create_workspace(WorkspaceInitOptions::new("workspace").expect("workspace options"))
        .expect("create workspace");
    (engine, workspace)
}

#[test]
fn context_overview_includes_record_knowledge_relations() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let knowledge = engine
        .create_knowledge(
            KnowledgeCreateOptions::new(
                workspace.initial_branch_id,
                workspace.genesis_commit_id,
                "Context overview exposes Record-to-Knowledge support",
            )
            .expect("knowledge options"),
        )
        .expect("create knowledge");
    let finding = engine
        .create_record(
            RecordCreateOptions::finding(
                workspace.initial_branch_id,
                knowledge.commit_id,
                "The context output includes Record-to-Knowledge relations",
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
    let session = engine
        .start_session(
            SessionStartOptions::new(workspace.workspace_id, workspace.initial_branch_id)
                .expect("session options"),
        )
        .expect("start session");

    let context = engine
        .context_overview(ContextOverviewOptions::new(session.session_id))
        .expect("context overview");

    assert!(context.record_relations.relations.is_empty());
    assert_eq!(
        context.record_knowledge_relations.workspace_id,
        workspace.workspace_id
    );
    assert_eq!(
        context.record_knowledge_relations.commit_id,
        relation.commit_id
    );
    assert_eq!(context.record_knowledge_relations.relations.len(), 1);
    assert_eq!(
        context.record_knowledge_relations.relations[0].relation_id,
        relation.relation_id
    );
    assert_eq!(
        context.record_knowledge_relations.relations[0].relation_type,
        RecordRelationType::Supports
    );
    assert_eq!(
        context.record_knowledge_relations.relations[0].source_record_entity_id,
        finding.record_entity_id
    );
    assert_eq!(
        context.record_knowledge_relations.relations[0].target_knowledge_entity_id,
        knowledge.knowledge_entity_id
    );
}
