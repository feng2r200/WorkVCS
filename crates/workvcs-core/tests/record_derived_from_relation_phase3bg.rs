use std::path::{Path, PathBuf};
use tempfile::TempDir;
use workvcs_core::{
    Engine, RecordCreateOptions, RecordRelationCreateOptions, RecordRelationListOptions,
    RecordRelationType, StoreInitOptions, WhyEntityKind, WhyQueryOptions, WhyQueryTarget,
    WhyRelationDirection, WhyRelationEndpoint, WhyRelationKind, WorkspaceInfo,
    WorkspaceInitOptions,
};

fn store_path() -> (TempDir, PathBuf) {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let path = tempdir.path().join("workvcs.sqlite");
    (tempdir, path)
}

fn init_engine(path: &Path) -> Engine {
    Engine::init(
        path,
        StoreInitOptions::new("phase3bg-record-derived-from-store").expect("store options"),
    )
    .expect("init engine")
}

fn create_workspace(path: &Path) -> (Engine, WorkspaceInfo) {
    let mut engine = init_engine(path);
    let workspace = engine
        .create_workspace(WorkspaceInitOptions::new("workspace").expect("workspace options"))
        .expect("create workspace");
    (engine, workspace)
}

#[test]
fn record_can_derive_from_another_record() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let finding = engine
        .create_record(
            RecordCreateOptions::finding(
                workspace.initial_branch_id,
                workspace.genesis_commit_id,
                "Concurrent write tests fail without serialization",
            )
            .expect("finding options"),
        )
        .expect("create finding");
    let decision = engine
        .create_record(
            RecordCreateOptions::decision(
                workspace.initial_branch_id,
                finding.commit_id,
                "Use serialized writes",
            )
            .expect("decision options"),
        )
        .expect("create decision");

    let relation = engine
        .create_record_relation(
            RecordRelationCreateOptions::derived_from(
                workspace.initial_branch_id,
                decision.commit_id,
                decision.record_entity_id,
                finding.record_entity_id,
                "Decision was derived from the finding",
            )
            .expect("relation options"),
        )
        .expect("create derived_from relation");

    assert_eq!(relation.relation_type, RecordRelationType::DerivedFrom);
    assert_eq!(relation.source_record_entity_id, decision.record_entity_id);
    assert_eq!(relation.target_record_entity_id, finding.record_entity_id);

    let listed = engine
        .record_relations_at(
            RecordRelationListOptions::new(relation.commit_id)
                .with_relation_type(RecordRelationType::DerivedFrom),
        )
        .expect("list derived_from");
    assert_eq!(listed.relations.len(), 1);
    assert_eq!(listed.relations[0].relation_id, relation.relation_id);

    let why_decision = engine
        .why(WhyQueryOptions::new(
            WhyQueryTarget::commit(relation.commit_id),
            decision.record_entity_id,
        ))
        .expect("why decision");
    assert_eq!(why_decision.relation_edges.len(), 1);
    assert_eq!(
        why_decision.relation_edges[0].relation_kind,
        WhyRelationKind::RecordDerivedFrom
    );
    assert_eq!(
        why_decision.relation_edges[0].direction,
        WhyRelationDirection::Outgoing
    );
    assert_eq!(
        why_decision.relation_edges[0].source,
        WhyRelationEndpoint::entity(decision.record_entity_id, WhyEntityKind::Record)
    );
    assert_eq!(
        why_decision.relation_edges[0].target,
        WhyRelationEndpoint::entity(finding.record_entity_id, WhyEntityKind::Record)
    );

    let duplicate = engine
        .create_record_relation(
            RecordRelationCreateOptions::derived_from(
                workspace.initial_branch_id,
                relation.commit_id,
                decision.record_entity_id,
                finding.record_entity_id,
                "Duplicate derived_from relation",
            )
            .expect("duplicate options"),
        )
        .expect_err("duplicate derived_from should fail");
    assert!(duplicate.to_string().contains("already exists"));
}
