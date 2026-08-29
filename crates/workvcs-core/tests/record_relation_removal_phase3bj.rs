use std::path::{Path, PathBuf};
use tempfile::TempDir;
use workvcs_core::{
    Engine, RecordCreateCommit, RecordCreateOptions, RecordRelationCreateCommit,
    RecordRelationCreateOptions, RecordRelationListOptions, RecordRelationRemoveOptions,
    RecordRelationType, RelationVersionId, StoreInitOptions, WorkspaceInfo, WorkspaceInitOptions,
};

fn store_path() -> (TempDir, PathBuf) {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let path = tempdir.path().join("workvcs.sqlite");
    (tempdir, path)
}

fn init_engine(path: &Path) -> Engine {
    Engine::init(
        path,
        StoreInitOptions::new("phase3bj-record-relation-removal-store").expect("store options"),
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

fn supported_decision(
    engine: &mut Engine,
    workspace: &WorkspaceInfo,
) -> (
    RecordCreateCommit,
    RecordCreateCommit,
    RecordRelationCreateCommit,
) {
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
    let relation = engine
        .create_record_relation(
            RecordRelationCreateOptions::supports(
                workspace.initial_branch_id,
                finding.commit_id,
                finding.record_entity_id,
                decision.record_entity_id,
                "Finding supports the decision",
            )
            .expect("relation options"),
        )
        .expect("create supports relation");
    (decision, finding, relation)
}

#[test]
fn removing_record_relation_drops_current_edge_and_preserves_history() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let (decision, finding, relation) = supported_decision(&mut engine, &workspace);

    let listed_before = engine
        .record_relations_at(
            RecordRelationListOptions::new(relation.commit_id)
                .with_relation_type(RecordRelationType::Supports),
        )
        .expect("list relation before removal");
    assert_eq!(listed_before.relations.len(), 1);
    assert_eq!(listed_before.relations[0].relation_id, relation.relation_id);

    let removed = engine
        .remove_record_relation(
            RecordRelationRemoveOptions::new(
                workspace.initial_branch_id,
                relation.commit_id,
                relation.relation_id,
                relation.relation_version_id,
                "Finding no longer supports the decision",
            )
            .expect("remove options"),
        )
        .expect("remove relation");
    assert_eq!(removed.workspace_id, workspace.workspace_id);
    assert_eq!(removed.previous_head_commit_id, relation.commit_id);
    assert_eq!(removed.relation_id, relation.relation_id);
    assert_eq!(
        removed.previous_relation_version_id,
        relation.relation_version_id
    );
    assert_eq!(removed.relation_type, RecordRelationType::Supports);
    assert_eq!(removed.source_record_entity_id, finding.record_entity_id);
    assert_eq!(removed.target_record_entity_id, decision.record_entity_id);

    let replayed_after = engine.state_at(removed.commit_id).expect("replay removal");
    assert_eq!(replayed_after.state_digest, removed.work_state_digest);
    assert!(replayed_after.state.relations().is_empty());

    let listed_after = engine
        .record_relations_at(RecordRelationListOptions::new(removed.commit_id))
        .expect("list after removal");
    assert!(listed_after.relations.is_empty());

    let absent_after = engine
        .record_relation_at(removed.commit_id, relation.relation_id)
        .expect_err("removed relation should not be current");
    assert!(absent_after.to_string().contains("is not present"));

    let historical = engine
        .record_relation_at(relation.commit_id, relation.relation_id)
        .expect("historical relation remains queryable");
    assert_eq!(historical.relation_id, relation.relation_id);
    assert_eq!(historical.relation_version_id, relation.relation_version_id);
}

#[test]
fn record_relation_remove_rejects_stale_version_without_moving_branch() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let (_decision, _finding, relation) = supported_decision(&mut engine, &workspace);
    let stale_relation_version_id = RelationVersionId::new_v7();

    let stale = engine
        .remove_record_relation(
            RecordRelationRemoveOptions::new(
                workspace.initial_branch_id,
                relation.commit_id,
                relation.relation_id,
                stale_relation_version_id,
                "Stale remove should fail",
            )
            .expect("remove options"),
        )
        .expect_err("stale relation version should be rejected");
    assert!(stale.to_string().contains("expected version"));

    let branch = engine
        .branch_head(workspace.initial_branch_id)
        .expect("branch head");
    assert_eq!(branch.head_commit_id, relation.commit_id);
}

#[test]
fn record_relation_remove_rejects_absent_relation_without_moving_branch() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let (_decision, _finding, relation) = supported_decision(&mut engine, &workspace);
    let removed = engine
        .remove_record_relation(
            RecordRelationRemoveOptions::new(
                workspace.initial_branch_id,
                relation.commit_id,
                relation.relation_id,
                relation.relation_version_id,
                "Finding no longer supports the decision",
            )
            .expect("remove options"),
        )
        .expect("remove relation");

    let absent = engine
        .remove_record_relation(
            RecordRelationRemoveOptions::new(
                workspace.initial_branch_id,
                removed.commit_id,
                relation.relation_id,
                relation.relation_version_id,
                "Second remove should fail",
            )
            .expect("remove options"),
        )
        .expect_err("absent relation should be rejected");
    assert!(absent.to_string().contains("is not present"));

    let branch = engine
        .branch_head(workspace.initial_branch_id)
        .expect("branch head");
    assert_eq!(branch.head_commit_id, removed.commit_id);
}
