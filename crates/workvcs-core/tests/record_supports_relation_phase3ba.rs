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
        StoreInitOptions::new("phase3ba-record-supports-store").expect("store options"),
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
fn finding_can_support_active_decision() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);

    let decision = engine
        .create_record(
            RecordCreateOptions::decision(
                workspace.initial_branch_id,
                workspace.genesis_commit_id,
                "Use serialized writes",
            )
            .expect("decision options"),
        )
        .expect("create decision");
    let finding = engine
        .create_record(
            RecordCreateOptions::finding(
                workspace.initial_branch_id,
                decision.commit_id,
                "Concurrent write tests are flaky without serialization",
            )
            .expect("finding options"),
        )
        .expect("create finding");
    let assumption = engine
        .create_record(
            RecordCreateOptions::assumption(
                workspace.initial_branch_id,
                finding.commit_id,
                "Serialized writes are sufficient",
            )
            .expect("assumption options"),
        )
        .expect("create assumption");

    let wrong_target = engine
        .create_record_relation(
            RecordRelationCreateOptions::supports(
                workspace.initial_branch_id,
                assumption.commit_id,
                finding.record_entity_id,
                assumption.record_entity_id,
                "Finding supports the decision",
            )
            .expect("relation options"),
        )
        .expect_err("supports should reject non-decision target");
    assert!(
        wrong_target
            .to_string()
            .contains("target must be a Decision")
    );

    let relation = engine
        .create_record_relation(
            RecordRelationCreateOptions::supports(
                workspace.initial_branch_id,
                assumption.commit_id,
                finding.record_entity_id,
                decision.record_entity_id,
                "Finding supports the decision",
            )
            .expect("relation options"),
        )
        .expect("create supports relation");

    assert_eq!(relation.workspace_id, workspace.workspace_id);
    assert_eq!(relation.relation_type, RecordRelationType::Supports);
    assert_eq!(relation.source_record_entity_id, finding.record_entity_id);
    assert_eq!(relation.target_record_entity_id, decision.record_entity_id);

    let relations = engine
        .record_relations_at(
            RecordRelationListOptions::new(relation.commit_id)
                .with_relation_type(RecordRelationType::Supports),
        )
        .expect("list supports relations");
    assert_eq!(relations.relations.len(), 1);
    assert_eq!(relations.relations[0].relation_id, relation.relation_id);

    let why = engine
        .why(WhyQueryOptions::new(
            WhyQueryTarget::commit(relation.commit_id),
            decision.record_entity_id,
        ))
        .expect("why supported decision");
    assert_eq!(why.relation_edges.len(), 1);
    assert_eq!(
        why.relation_edges[0].relation_kind,
        WhyRelationKind::RecordSupports
    );
    assert_eq!(
        why.relation_edges[0].direction,
        WhyRelationDirection::Incoming
    );
    assert_eq!(
        why.relation_edges[0].source,
        WhyRelationEndpoint::entity(finding.record_entity_id, WhyEntityKind::Record)
    );
    assert_eq!(
        why.relation_edges[0].target,
        WhyRelationEndpoint::entity(decision.record_entity_id, WhyEntityKind::Record)
    );

    let duplicate = engine
        .create_record_relation(
            RecordRelationCreateOptions::supports(
                workspace.initial_branch_id,
                relation.commit_id,
                finding.record_entity_id,
                decision.record_entity_id,
                "Duplicate relation",
            )
            .expect("duplicate options"),
        )
        .expect_err("duplicate relation should be rejected");
    assert!(duplicate.to_string().contains("already exists"));
}
