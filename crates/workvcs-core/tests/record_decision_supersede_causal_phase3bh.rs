use std::path::{Path, PathBuf};
use tempfile::TempDir;
use workvcs_core::{
    DecisionRecordSupersedeOptions, Engine, RecordCreateOptions, RecordRelationListOptions,
    RecordRelationType, StoreInitOptions, WhyQueryOptions, WhyQueryTarget, WhyRelationKind,
    WorkspaceInfo, WorkspaceInitOptions,
};

fn store_path() -> (TempDir, PathBuf) {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let path = tempdir.path().join("workvcs.sqlite");
    (tempdir, path)
}

fn init_engine(path: &Path) -> Engine {
    Engine::init(
        path,
        StoreInitOptions::new("phase3bh-decision-supersede-causal-store").expect("store options"),
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
fn supersede_decision_can_attach_causal_source_record() {
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
    let replacement = engine
        .create_record(
            RecordCreateOptions::decision(
                workspace.initial_branch_id,
                finding.commit_id,
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
                "Finding caused the replacement decision",
            )
            .expect("supersede options")
            .with_causal_record(finding.record_entity_id),
        )
        .expect("supersede decision");

    assert_eq!(
        superseded.causal_record_entity_id,
        Some(finding.record_entity_id)
    );
    assert!(superseded.causal_relation_operation_id.is_some());
    assert!(superseded.causal_relation_id.is_some());
    assert!(superseded.causal_relation_version_id.is_some());
    assert!(superseded.causal_relation_state_digest.is_some());

    let supersedes = engine
        .record_relations_at(
            RecordRelationListOptions::new(superseded.commit_id)
                .with_relation_type(RecordRelationType::Supersedes),
        )
        .expect("supersedes relations");
    assert_eq!(supersedes.relations.len(), 1);
    assert_eq!(supersedes.relations[0].relation_id, superseded.relation_id);

    let derived = engine
        .record_relations_at(
            RecordRelationListOptions::new(superseded.commit_id)
                .with_relation_type(RecordRelationType::DerivedFrom),
        )
        .expect("derived_from relations");
    assert_eq!(derived.relations.len(), 1);
    assert_eq!(
        derived.relations[0].relation_id,
        superseded.causal_relation_id.unwrap()
    );
    assert_eq!(
        derived.relations[0].source_record_entity_id,
        replacement.record_entity_id
    );
    assert_eq!(
        derived.relations[0].target_record_entity_id,
        finding.record_entity_id
    );

    let why_replacement = engine
        .why(WhyQueryOptions::new(
            WhyQueryTarget::commit(superseded.commit_id),
            replacement.record_entity_id,
        ))
        .expect("why replacement");
    assert!(
        why_replacement
            .relation_edges
            .iter()
            .any(|edge| edge.relation_kind == WhyRelationKind::RecordSupersedes)
    );
    assert!(
        why_replacement
            .relation_edges
            .iter()
            .any(|edge| edge.relation_kind == WhyRelationKind::RecordDerivedFrom)
    );
}

#[test]
fn supersede_decision_rejects_self_causal_anchor() {
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

    let error = engine
        .supersede_decision_record(
            DecisionRecordSupersedeOptions::new(
                workspace.initial_branch_id,
                replacement.commit_id,
                replacement.record_entity_id,
                prior.record_entity_id,
                prior.record_entity_version_id,
                "Self causal anchor is invalid",
            )
            .expect("supersede options")
            .with_causal_record(replacement.record_entity_id),
        )
        .expect_err("self causal anchor should fail");

    assert!(error.to_string().contains("cannot derive from itself"));
}
