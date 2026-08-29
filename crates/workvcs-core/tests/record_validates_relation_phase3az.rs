use std::path::{Path, PathBuf};
use tempfile::TempDir;
use workvcs_core::{
    Engine, RecordCreateOptions, RecordRelationCreateOptions, RecordRelationListOptions,
    RecordRelationType, RecordTransitionOptions, StoreInitOptions, WhyEntityKind, WhyQueryOptions,
    WhyQueryTarget, WhyRelationDirection, WhyRelationEndpoint, WhyRelationKind, WorkspaceInfo,
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
        StoreInitOptions::new("phase3az-record-validates-store").expect("store options"),
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
fn finding_can_link_to_validated_assumption() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);

    let assumption = engine
        .create_record(
            RecordCreateOptions::assumption(
                workspace.initial_branch_id,
                workspace.genesis_commit_id,
                "Serialized writes are sufficient",
            )
            .expect("assumption options"),
        )
        .expect("create assumption");
    let finding = engine
        .create_record(
            RecordCreateOptions::finding(
                workspace.initial_branch_id,
                assumption.commit_id,
                "Concurrent writer test passed",
            )
            .expect("finding options"),
        )
        .expect("create finding");

    let err = engine
        .create_record_relation(
            RecordRelationCreateOptions::validates(
                workspace.initial_branch_id,
                finding.commit_id,
                finding.record_entity_id,
                assumption.record_entity_id,
                "Finding validates the assumption",
            )
            .expect("relation options"),
        )
        .expect_err("unvalidated assumption should not be linkable");
    assert!(
        err.to_string()
            .contains("target Assumption must be validated")
    );

    let validated = engine
        .transition_record(
            RecordTransitionOptions::validate_assumption(
                workspace.initial_branch_id,
                finding.commit_id,
                assumption.record_entity_id,
                assumption.record_entity_version_id,
                "Concurrent writer test passed",
            )
            .expect("validate options"),
        )
        .expect("validate assumption");
    let relation = engine
        .create_record_relation(
            RecordRelationCreateOptions::validates(
                workspace.initial_branch_id,
                validated.commit_id,
                finding.record_entity_id,
                assumption.record_entity_id,
                "Finding validates the assumption",
            )
            .expect("relation options"),
        )
        .expect("create validates relation");

    assert_eq!(relation.workspace_id, workspace.workspace_id);
    assert_eq!(relation.relation_type, RecordRelationType::Validates);
    assert_eq!(relation.source_record_entity_id, finding.record_entity_id);
    assert_eq!(
        relation.target_record_entity_id,
        assumption.record_entity_id
    );

    let relations = engine
        .record_relations_at(
            RecordRelationListOptions::new(relation.commit_id)
                .with_relation_type(RecordRelationType::Validates),
        )
        .expect("list validates relations");
    assert_eq!(relations.relations.len(), 1);
    assert_eq!(relations.relations[0].relation_id, relation.relation_id);

    let why = engine
        .why(WhyQueryOptions::new(
            WhyQueryTarget::commit(relation.commit_id),
            assumption.record_entity_id,
        ))
        .expect("why validated assumption");
    assert_eq!(why.relation_edges.len(), 1);
    assert_eq!(
        why.relation_edges[0].relation_kind,
        WhyRelationKind::RecordValidates
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
        WhyRelationEndpoint::entity(assumption.record_entity_id, WhyEntityKind::Record)
    );

    let duplicate = engine
        .create_record_relation(
            RecordRelationCreateOptions::validates(
                workspace.initial_branch_id,
                relation.commit_id,
                finding.record_entity_id,
                assumption.record_entity_id,
                "Duplicate relation",
            )
            .expect("duplicate options"),
        )
        .expect_err("duplicate relation should be rejected");
    assert!(duplicate.to_string().contains("already exists"));
}
