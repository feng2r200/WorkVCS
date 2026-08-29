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
        StoreInitOptions::new("phase3bd-record-related-to-store").expect("store options"),
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
fn related_to_preserves_custom_label_as_logical_discriminator() {
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

    let empty_label = RecordRelationCreateOptions::related_to(
        workspace.initial_branch_id,
        decision.commit_id,
        finding.record_entity_id,
        decision.record_entity_id,
        " ",
        "Custom relation",
    )
    .expect_err("empty label should be rejected");
    assert!(empty_label.to_string().contains("label must not be empty"));

    let relation = engine
        .create_record_relation(
            RecordRelationCreateOptions::related_to(
                workspace.initial_branch_id,
                decision.commit_id,
                finding.record_entity_id,
                decision.record_entity_id,
                "caused_by",
                "Finding caused the decision",
            )
            .expect("relation options"),
        )
        .expect("create related_to relation");
    assert_eq!(relation.relation_type, RecordRelationType::RelatedTo);
    assert_eq!(relation.relation_label.as_deref(), Some("caused_by"));

    let same_label = engine
        .create_record_relation(
            RecordRelationCreateOptions::related_to(
                workspace.initial_branch_id,
                relation.commit_id,
                finding.record_entity_id,
                decision.record_entity_id,
                "caused_by",
                "Duplicate custom relation",
            )
            .expect("duplicate options"),
        )
        .expect_err("same related_to label should be unique");
    assert!(same_label.to_string().contains("already exists"));

    let second_label = engine
        .create_record_relation(
            RecordRelationCreateOptions::related_to(
                workspace.initial_branch_id,
                relation.commit_id,
                finding.record_entity_id,
                decision.record_entity_id,
                "mentioned_with",
                "A distinct custom relation",
            )
            .expect("second label options"),
        )
        .expect("create second related_to relation");

    let filtered = engine
        .record_relations_at(
            RecordRelationListOptions::new(second_label.commit_id)
                .with_relation_type(RecordRelationType::RelatedTo)
                .with_relation_label("caused_by")
                .expect("label filter"),
        )
        .expect("list by related_to label");
    assert_eq!(filtered.relations.len(), 1);
    assert_eq!(filtered.relations[0].relation_id, relation.relation_id);
    assert_eq!(
        filtered.relations[0].relation_label.as_deref(),
        Some("caused_by")
    );

    let shown = engine
        .record_relation_at(second_label.commit_id, relation.relation_id)
        .expect("show related_to");
    assert_eq!(shown.relation_label.as_deref(), Some("caused_by"));

    let why = engine
        .why(WhyQueryOptions::new(
            WhyQueryTarget::commit(second_label.commit_id),
            decision.record_entity_id,
        ))
        .expect("why related decision");
    let custom_edge = why
        .relation_edges
        .iter()
        .find(|edge| edge.relation_id == relation.relation_id)
        .expect("caused_by edge");
    assert_eq!(custom_edge.relation_kind, WhyRelationKind::RecordRelatedTo);
    assert_eq!(custom_edge.direction, WhyRelationDirection::Incoming);
    assert_eq!(custom_edge.relation_label.as_deref(), Some("caused_by"));
    assert_eq!(
        custom_edge.source,
        WhyRelationEndpoint::entity(finding.record_entity_id, WhyEntityKind::Record)
    );
    assert_eq!(
        custom_edge.target,
        WhyRelationEndpoint::entity(decision.record_entity_id, WhyEntityKind::Record)
    );
}
