use std::path::{Path, PathBuf};
use tempfile::TempDir;
use workvcs_core::{
    DecisionRecordSupersedeOptions, Engine, ErrorCategory, ErrorCode, RecordCreateOptions,
    RecordRelationCreateOptions, RecordRelationListOptions, RecordRelationType, RecordStatus,
    StoreInitOptions, WhyEntityKind, WhyQueryOptions, WhyQueryTarget, WhyRelationDirection,
    WhyRelationEndpoint, WhyRelationKind, WorkspaceInfo, WorkspaceInitOptions,
};

fn store_path() -> (TempDir, PathBuf) {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let path = tempdir.path().join("workvcs.sqlite");
    (tempdir, path)
}

fn init_engine(path: &Path) -> Engine {
    Engine::init(
        path,
        StoreInitOptions::new("phase3bf-decision-supersedes-store").expect("store options"),
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
fn supersede_decision_atomically_marks_prior_and_creates_relation() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let prior = engine
        .create_record(
            RecordCreateOptions::decision(
                workspace.initial_branch_id,
                workspace.genesis_commit_id,
                "Use optimistic writes",
            )
            .expect("prior decision options"),
        )
        .expect("create prior decision");
    let finding = engine
        .create_record(
            RecordCreateOptions::finding(
                workspace.initial_branch_id,
                prior.commit_id,
                "Concurrent write tests fail without serialization",
            )
            .expect("finding options"),
        )
        .expect("create finding");
    let support = engine
        .create_record_relation(
            RecordRelationCreateOptions::supports(
                workspace.initial_branch_id,
                finding.commit_id,
                finding.record_entity_id,
                prior.record_entity_id,
                "Finding initially supports the old decision",
            )
            .expect("support relation options"),
        )
        .expect("create support relation");
    let replacement = engine
        .create_record(
            RecordCreateOptions::decision(
                workspace.initial_branch_id,
                support.commit_id,
                "Use serialized writes",
            )
            .expect("replacement decision options"),
        )
        .expect("create replacement decision");

    let superseded = engine
        .supersede_decision_record(
            DecisionRecordSupersedeOptions::new(
                workspace.initial_branch_id,
                replacement.commit_id,
                replacement.record_entity_id,
                prior.record_entity_id,
                prior.record_entity_version_id,
                "Serialized writes supersede optimistic writes",
            )
            .expect("supersede options"),
        )
        .expect("supersede decision");

    assert_eq!(
        superseded.replacement_record_entity_id,
        replacement.record_entity_id
    );
    assert_eq!(superseded.prior_record_entity_id, prior.record_entity_id);
    assert_eq!(
        superseded.previous_prior_record_entity_version_id,
        prior.record_entity_version_id
    );
    assert_eq!(superseded.previous_prior_state.status, RecordStatus::Active);
    assert_eq!(superseded.prior_state.status, RecordStatus::Superseded);

    let prior_after = engine
        .record_at(superseded.commit_id, prior.record_entity_id)
        .expect("prior after supersession");
    assert_eq!(prior_after.state.status, RecordStatus::Superseded);
    assert_eq!(
        prior_after.record_entity_version_id,
        superseded.prior_record_entity_version_id
    );
    let replacement_after = engine
        .record_at(superseded.commit_id, replacement.record_entity_id)
        .expect("replacement after supersession");
    assert_eq!(replacement_after.state.status, RecordStatus::Active);

    let supersedes = engine
        .record_relations_at(
            RecordRelationListOptions::new(superseded.commit_id)
                .with_relation_type(RecordRelationType::Supersedes),
        )
        .expect("list supersedes relations");
    assert_eq!(supersedes.relations.len(), 1);
    assert_eq!(supersedes.relations[0].relation_id, superseded.relation_id);
    assert_eq!(
        supersedes.relations[0].source_record_entity_id,
        replacement.record_entity_id
    );
    assert_eq!(
        supersedes.relations[0].target_record_entity_id,
        prior.record_entity_id
    );

    let support_after_supersede = engine
        .record_relations_at(
            RecordRelationListOptions::new(superseded.commit_id)
                .with_relation_type(RecordRelationType::Supports),
        )
        .expect("supports remains queryable after prior terminal state");
    assert_eq!(support_after_supersede.relations.len(), 1);
    assert_eq!(
        support_after_supersede.relations[0].relation_id,
        support.relation_id
    );

    let why_prior = engine
        .why(WhyQueryOptions::new(
            WhyQueryTarget::commit(superseded.commit_id),
            prior.record_entity_id,
        ))
        .expect("why prior");
    let supersedes_edge = why_prior
        .relation_edges
        .iter()
        .find(|edge| edge.relation_kind == WhyRelationKind::RecordSupersedes)
        .expect("supersedes why edge");
    assert_eq!(supersedes_edge.direction, WhyRelationDirection::Incoming);
    assert_eq!(
        supersedes_edge.source,
        WhyRelationEndpoint::entity(replacement.record_entity_id, WhyEntityKind::Record)
    );
    assert_eq!(
        supersedes_edge.target,
        WhyRelationEndpoint::entity(prior.record_entity_id, WhyEntityKind::Record)
    );
}

#[test]
fn supersede_decision_rejects_non_active_or_non_decision_endpoints() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let prior = engine
        .create_record(
            RecordCreateOptions::decision(
                workspace.initial_branch_id,
                workspace.genesis_commit_id,
                "Use optimistic writes",
            )
            .expect("prior decision options"),
        )
        .expect("create prior decision");
    let replacement = engine
        .create_record(
            RecordCreateOptions::decision(
                workspace.initial_branch_id,
                prior.commit_id,
                "Use serialized writes",
            )
            .expect("replacement decision options"),
        )
        .expect("create replacement decision");
    let superseded = engine
        .supersede_decision_record(
            DecisionRecordSupersedeOptions::new(
                workspace.initial_branch_id,
                replacement.commit_id,
                replacement.record_entity_id,
                prior.record_entity_id,
                prior.record_entity_version_id,
                "Serialized writes supersede optimistic writes",
            )
            .expect("supersede options"),
        )
        .expect("supersede decision");

    let repeat = engine
        .supersede_decision_record(
            DecisionRecordSupersedeOptions::new(
                workspace.initial_branch_id,
                superseded.commit_id,
                replacement.record_entity_id,
                prior.record_entity_id,
                superseded.prior_record_entity_version_id,
                "Trying to supersede an already superseded decision",
            )
            .expect("repeat options"),
        )
        .expect_err("terminal prior should fail");
    assert_eq!(repeat.code(), ErrorCode::RecordInvalid);
    assert_eq!(repeat.category(), ErrorCategory::Record);
    assert!(
        repeat
            .to_string()
            .contains("supersedes prior Decision must be active")
    );

    let finding = engine
        .create_record(
            RecordCreateOptions::finding(
                workspace.initial_branch_id,
                superseded.commit_id,
                "A finding is not a replacement decision",
            )
            .expect("finding options"),
        )
        .expect("create finding");
    let wrong_replacement = engine
        .supersede_decision_record(
            DecisionRecordSupersedeOptions::new(
                workspace.initial_branch_id,
                finding.commit_id,
                finding.record_entity_id,
                replacement.record_entity_id,
                replacement.record_entity_version_id,
                "Not applicable",
            )
            .expect("wrong replacement options"),
        )
        .expect_err("non-decision replacement should fail");
    assert_eq!(wrong_replacement.code(), ErrorCode::RecordInvalid);
    assert_eq!(wrong_replacement.category(), ErrorCategory::Record);
    assert!(
        wrong_replacement
            .to_string()
            .contains("replacement must be a Decision")
    );
}
