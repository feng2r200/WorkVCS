use std::path::{Path, PathBuf};
use tempfile::TempDir;
use workvcs_core::{
    Engine, RecordCreateOptions, RecordRelationCreateOptions, StoreInitOptions, WorkspaceInfo,
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
        StoreInitOptions::new("phase3bc-record-relation-show-store").expect("store options"),
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
fn record_relation_at_shows_current_relation_by_id() {
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

    let shown = engine
        .record_relation_at(relation.commit_id, relation.relation_id)
        .expect("show relation");
    assert_eq!(shown.workspace_id, workspace.workspace_id);
    assert_eq!(shown.relation_id, relation.relation_id);
    assert_eq!(shown.relation_version_id, relation.relation_version_id);
    assert_eq!(shown.relation_type, relation.relation_type);
    assert_eq!(shown.source_record_entity_id, finding.record_entity_id);
    assert_eq!(shown.target_record_entity_id, decision.record_entity_id);
    assert_eq!(shown.state_digest, relation.relation_state_digest);

    let absent = engine
        .record_relation_at(finding.commit_id, relation.relation_id)
        .expect_err("relation should not be current before creation");
    assert!(absent.to_string().contains("is not present"));
}
