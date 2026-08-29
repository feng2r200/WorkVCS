use std::path::{Path, PathBuf};
use tempfile::TempDir;
use workvcs_core::{
    Engine, RecordCreateOptions, RecordRelationCreateOptions, RecordRelationType,
    RecordTransitionOptions, StoreInitOptions, WorkspaceInfo, WorkspaceInitOptions,
};

fn store_path() -> (TempDir, PathBuf) {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let path = tempdir.path().join("workvcs.sqlite");
    (tempdir, path)
}

fn init_engine(path: &Path) -> Engine {
    Engine::init(
        path,
        StoreInitOptions::new("phase3av-record-invalidates-store").expect("store options"),
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
fn finding_can_link_to_invalidated_assumption() {
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
                "Concurrent writer test failed",
            )
            .expect("finding options"),
        )
        .expect("create finding");

    let err = engine
        .create_record_relation(
            RecordRelationCreateOptions::invalidates(
                workspace.initial_branch_id,
                finding.commit_id,
                finding.record_entity_id,
                assumption.record_entity_id,
                "Finding invalidates the assumption",
            )
            .expect("relation options"),
        )
        .expect_err("uninvalidated assumption should not be linkable");
    assert!(
        err.to_string()
            .contains("target Assumption must be invalidated")
    );

    let invalidated = engine
        .transition_record(
            RecordTransitionOptions::invalidate_assumption(
                workspace.initial_branch_id,
                finding.commit_id,
                assumption.record_entity_id,
                assumption.record_entity_version_id,
                "Concurrent writer test failed",
            )
            .expect("invalidate options"),
        )
        .expect("invalidate assumption");
    let relation = engine
        .create_record_relation(
            RecordRelationCreateOptions::invalidates(
                workspace.initial_branch_id,
                invalidated.commit_id,
                finding.record_entity_id,
                assumption.record_entity_id,
                "Finding invalidates the assumption",
            )
            .expect("relation options"),
        )
        .expect("create invalidates relation");

    assert_eq!(relation.workspace_id, workspace.workspace_id);
    assert_eq!(relation.relation_type, RecordRelationType::Invalidates);
    assert_eq!(relation.source_record_entity_id, finding.record_entity_id);
    assert_eq!(
        relation.target_record_entity_id,
        assumption.record_entity_id
    );

    let state = engine
        .state_at(relation.commit_id)
        .expect("replay relation commit");
    assert!(
        state
            .state
            .relations()
            .iter()
            .any(|(relation_id, relation_version_id)| {
                *relation_id == relation.relation_id
                    && *relation_version_id == relation.relation_version_id
            })
    );

    let duplicate = engine
        .create_record_relation(
            RecordRelationCreateOptions::invalidates(
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
