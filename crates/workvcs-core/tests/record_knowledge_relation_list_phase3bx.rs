use std::path::{Path, PathBuf};
use tempfile::TempDir;
use workvcs_core::{
    Engine, KnowledgeCreateOptions, KnowledgeTransitionOptions, RecordCreateOptions,
    RecordKnowledgeRelationCreateCommit, RecordKnowledgeRelationCreateOptions,
    RecordKnowledgeRelationListOptions, RecordRelationType, StoreInitOptions, WorkspaceInfo,
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
        StoreInitOptions::new("phase3bx-record-knowledge-relation-list-store")
            .expect("store options"),
    )
    .expect("init engine");
    let workspace = engine
        .create_workspace(WorkspaceInitOptions::new("workspace").expect("workspace options"))
        .expect("create workspace");
    (engine, workspace)
}

fn support_active_knowledge(
    engine: &mut Engine,
    workspace: &WorkspaceInfo,
    head: workvcs_core::CommitId,
) -> RecordKnowledgeRelationCreateCommit {
    let knowledge = engine
        .create_knowledge(
            KnowledgeCreateOptions::new(
                workspace.initial_branch_id,
                head,
                "Context overviews include active reusable Knowledge",
            )
            .expect("knowledge options"),
        )
        .expect("create knowledge");
    let finding = engine
        .create_record(
            RecordCreateOptions::finding(
                workspace.initial_branch_id,
                knowledge.commit_id,
                "The context overview output contains active Knowledge rows",
            )
            .expect("finding options"),
        )
        .expect("create finding");
    engine
        .create_record_knowledge_relation(
            RecordKnowledgeRelationCreateOptions::supports(
                workspace.initial_branch_id,
                finding.commit_id,
                finding.record_entity_id,
                knowledge.knowledge_entity_id,
                "The Finding supports the reusable Knowledge statement",
            )
            .expect("supports knowledge options"),
        )
        .expect("support knowledge relation")
}

fn invalidate_knowledge(
    engine: &mut Engine,
    workspace: &WorkspaceInfo,
    head: workvcs_core::CommitId,
) -> RecordKnowledgeRelationCreateCommit {
    let knowledge = engine
        .create_knowledge(
            KnowledgeCreateOptions::new(
                workspace.initial_branch_id,
                head,
                "Inactive Knowledge always appears in active context",
            )
            .expect("knowledge options"),
        )
        .expect("create knowledge");
    let finding = engine
        .create_record(
            RecordCreateOptions::finding(
                workspace.initial_branch_id,
                knowledge.commit_id,
                "Inactive Knowledge is excluded from active context",
            )
            .expect("finding options"),
        )
        .expect("create finding");
    let invalidated = engine
        .transition_knowledge(
            KnowledgeTransitionOptions::invalidate(
                workspace.initial_branch_id,
                finding.commit_id,
                knowledge.knowledge_entity_id,
                knowledge.knowledge_entity_version_id,
                "Finding invalidates the Knowledge statement",
            )
            .expect("invalidate knowledge options"),
        )
        .expect("invalidate knowledge");
    engine
        .create_record_knowledge_relation(
            RecordKnowledgeRelationCreateOptions::invalidates(
                workspace.initial_branch_id,
                invalidated.commit_id,
                finding.record_entity_id,
                knowledge.knowledge_entity_id,
                "The Finding explains the Knowledge invalidation",
            )
            .expect("invalidates knowledge options"),
        )
        .expect("invalidate knowledge relation")
}

#[test]
fn record_knowledge_relations_at_lists_current_edges_with_filters() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);

    let support = support_active_knowledge(&mut engine, &workspace, workspace.genesis_commit_id);
    let empty_before_second = engine
        .record_knowledge_relations_at(RecordKnowledgeRelationListOptions::new(support.commit_id))
        .expect("list support relation");
    assert_eq!(empty_before_second.relations.len(), 1);

    let invalidates = invalidate_knowledge(&mut engine, &workspace, support.commit_id);
    let all = engine
        .record_knowledge_relations_at(RecordKnowledgeRelationListOptions::new(
            invalidates.commit_id,
        ))
        .expect("list all record knowledge relations");
    assert_eq!(all.workspace_id, workspace.workspace_id);
    assert_eq!(all.commit_id, invalidates.commit_id);
    assert_eq!(all.relations.len(), 2);
    assert!(all.relations.windows(2).all(|window| {
        (
            window[0].relation_type,
            window[0].source_record_entity_id,
            window[0].target_knowledge_entity_id,
            window[0].relation_id,
        ) <= (
            window[1].relation_type,
            window[1].source_record_entity_id,
            window[1].target_knowledge_entity_id,
            window[1].relation_id,
        )
    }));

    let supports = engine
        .record_knowledge_relations_at(
            RecordKnowledgeRelationListOptions::new(invalidates.commit_id)
                .with_relation_type(RecordRelationType::Supports),
        )
        .expect("list supports relations");
    assert_eq!(supports.relations.len(), 1);
    assert_eq!(supports.relations[0].relation_id, support.relation_id);

    let source_filtered = engine
        .record_knowledge_relations_at(
            RecordKnowledgeRelationListOptions::new(invalidates.commit_id)
                .with_source_record(support.source_record_entity_id),
        )
        .expect("list by source record");
    assert_eq!(source_filtered.relations.len(), 1);
    assert_eq!(
        source_filtered.relations[0].relation_id,
        support.relation_id
    );

    let target_filtered = engine
        .record_knowledge_relations_at(
            RecordKnowledgeRelationListOptions::new(invalidates.commit_id)
                .with_target_knowledge(invalidates.target_knowledge_entity_id),
        )
        .expect("list by target knowledge");
    assert_eq!(target_filtered.relations.len(), 1);
    assert_eq!(
        target_filtered.relations[0].relation_id,
        invalidates.relation_id
    );

    let no_record_knowledge_contradictions = engine
        .record_knowledge_relations_at(
            RecordKnowledgeRelationListOptions::new(invalidates.commit_id)
                .with_relation_type(RecordRelationType::Contradicts),
        )
        .expect("list contradictions");
    assert!(no_record_knowledge_contradictions.relations.is_empty());
}
