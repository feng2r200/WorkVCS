use std::path::{Path, PathBuf};
use tempfile::TempDir;
use workvcs_core::{
    Engine, RecordCreateCommit, RecordCreateOptions, RecordRelationCreateCommit,
    RecordRelationCreateOptions, RecordRelationListOptions, RecordRelationRemoveOptions,
    RecordRelationRestoreOptions, RecordRelationType, StoreInitOptions, WorkspaceInfo,
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
        StoreInitOptions::new("phase3bm-record-relation-restore-store").expect("store options"),
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
                "Benchmarks support serialized writes",
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
fn restore_record_relation_reinstalls_removed_edge() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let (decision, finding, relation) = supported_decision(&mut engine, &workspace);
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

    let restored = engine
        .restore_record_relation(
            RecordRelationRestoreOptions::new(
                workspace.initial_branch_id,
                removed.commit_id,
                relation.relation_id,
                relation.relation_version_id,
                "Restore support relation",
            )
            .expect("restore options"),
        )
        .expect("restore relation");
    assert_eq!(restored.workspace_id, workspace.workspace_id);
    assert_eq!(restored.previous_head_commit_id, removed.commit_id);
    assert_eq!(restored.relation_id, relation.relation_id);
    assert_eq!(restored.relation_version_id, relation.relation_version_id);
    assert_eq!(restored.relation_type, RecordRelationType::Supports);
    assert_eq!(restored.source_record_entity_id, finding.record_entity_id);
    assert_eq!(restored.target_record_entity_id, decision.record_entity_id);
    assert_eq!(
        restored.relation_state_digest,
        relation.relation_state_digest
    );

    let listed_after_remove = engine
        .record_relations_at(RecordRelationListOptions::new(removed.commit_id))
        .expect("list after removal");
    assert!(listed_after_remove.relations.is_empty());

    let listed_after_restore = engine
        .record_relations_at(
            RecordRelationListOptions::new(restored.commit_id)
                .with_relation_type(RecordRelationType::Supports),
        )
        .expect("list after restore");
    assert_eq!(listed_after_restore.relations.len(), 1);
    assert_eq!(
        listed_after_restore.relations[0].relation_id,
        relation.relation_id
    );
    assert_eq!(
        listed_after_restore.relations[0].relation_version_id,
        relation.relation_version_id
    );

    let replayed = engine.state_at(restored.commit_id).expect("replay restore");
    assert_eq!(replayed.state_digest, restored.work_state_digest);
    assert_eq!(replayed.state.relations().len(), 1);
}

#[test]
fn restore_record_relation_rejects_present_edge_without_moving_branch() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let (_decision, _finding, relation) = supported_decision(&mut engine, &workspace);

    let error = engine
        .restore_record_relation(
            RecordRelationRestoreOptions::new(
                workspace.initial_branch_id,
                relation.commit_id,
                relation.relation_id,
                relation.relation_version_id,
                "Restore already-present relation",
            )
            .expect("restore options"),
        )
        .expect_err("present relation should reject restore");
    assert!(error.to_string().contains("already present"));

    let branch = engine
        .branch_head(workspace.initial_branch_id)
        .expect("branch head");
    assert_eq!(branch.head_commit_id, relation.commit_id);
}
