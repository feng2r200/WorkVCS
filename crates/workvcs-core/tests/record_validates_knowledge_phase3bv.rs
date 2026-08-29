use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use tempfile::TempDir;
use workvcs_core::{
    Engine, KnowledgeCreateOptions, RecordCreateOptions, RecordKnowledgeRelationCreateOptions,
    StoreInitOptions, WhyEntityKind, WhyQueryOptions, WhyQueryTarget, WhyRelationDirection,
    WhyRelationEndpoint, WhyRelationKind, WorkspaceInfo, WorkspaceInitOptions,
};

fn store_path() -> (TempDir, PathBuf) {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let path = tempdir.path().join("workvcs.sqlite");
    (tempdir, path)
}

fn create_workspace(path: &Path) -> (Engine, WorkspaceInfo) {
    let mut engine = Engine::init(
        path,
        StoreInitOptions::new("phase3bv-record-validates-knowledge-store").expect("store options"),
    )
    .expect("init engine");
    let workspace = engine
        .create_workspace(WorkspaceInitOptions::new("workspace").expect("workspace options"))
        .expect("create workspace");
    (engine, workspace)
}

#[test]
fn finding_can_validate_knowledge_and_explain_the_knowledge_subject() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let knowledge = engine
        .create_knowledge(
            KnowledgeCreateOptions::new(
                workspace.initial_branch_id,
                workspace.genesis_commit_id,
                "Context summaries include active Knowledge",
            )
            .expect("knowledge options"),
        )
        .expect("create knowledge");
    let finding = engine
        .create_record(
            RecordCreateOptions::finding(
                workspace.initial_branch_id,
                knowledge.commit_id,
                "The current context command includes active Knowledge rows",
            )
            .expect("finding options"),
        )
        .expect("create finding");

    let relation = engine
        .create_record_knowledge_relation(
            RecordKnowledgeRelationCreateOptions::validates(
                workspace.initial_branch_id,
                finding.commit_id,
                finding.record_entity_id,
                knowledge.knowledge_entity_id,
                "The Finding validates the reusable Knowledge statement",
            )
            .expect("validates knowledge options"),
        )
        .expect("validate knowledge relation");

    let why = engine
        .why(WhyQueryOptions::for_entity(
            WhyQueryTarget::commit(relation.commit_id),
            knowledge.knowledge_entity_id,
        ))
        .expect("why knowledge");
    assert_eq!(why.subject.entity_kind(), Some(WhyEntityKind::Knowledge));
    assert_eq!(
        why.relation_edges
            .iter()
            .map(|edge| { (edge.relation_kind, edge.direction, edge.source, edge.target,) })
            .collect::<BTreeSet<_>>(),
        BTreeSet::from([(
            WhyRelationKind::RecordValidates,
            WhyRelationDirection::Incoming,
            WhyRelationEndpoint::entity(finding.record_entity_id, WhyEntityKind::Record),
            WhyRelationEndpoint::entity(knowledge.knowledge_entity_id, WhyEntityKind::Knowledge),
        )])
    );
    assert_eq!(why.relation_edges[0].relation_id, relation.relation_id);
}
