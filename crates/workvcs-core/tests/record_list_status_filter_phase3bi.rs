use std::path::{Path, PathBuf};
use tempfile::TempDir;
use workvcs_core::{
    DecisionRecordSupersedeOptions, Engine, RecordCreateOptions, RecordKind, RecordListOptions,
    RecordStatus, StoreInitOptions, WorkspaceInfo, WorkspaceInitOptions,
};

fn store_path() -> (TempDir, PathBuf) {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let path = tempdir.path().join("workvcs.sqlite");
    (tempdir, path)
}

fn init_engine(path: &Path) -> Engine {
    Engine::init(
        path,
        StoreInitOptions::new("phase3bi-record-list-status-filter-store").expect("store options"),
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
fn record_list_can_filter_by_status_and_kind() {
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
    let assumption = engine
        .create_record(
            RecordCreateOptions::assumption(
                workspace.initial_branch_id,
                superseded.commit_id,
                "Serialized writes are sufficient",
            )
            .expect("assumption options"),
        )
        .expect("create assumption");

    let active = engine
        .records_at(RecordListOptions::new(assumption.commit_id).with_status(RecordStatus::Active))
        .expect("active records");
    assert!(
        active
            .records
            .iter()
            .any(|record| record.record_entity_id == replacement.record_entity_id)
    );
    assert!(
        !active
            .records
            .iter()
            .any(|record| record.record_entity_id == prior.record_entity_id)
    );

    let superseded_decisions = engine
        .records_at(
            RecordListOptions::new(assumption.commit_id)
                .with_kind(RecordKind::Decision)
                .with_status(RecordStatus::Superseded),
        )
        .expect("superseded decisions");
    assert_eq!(superseded_decisions.records.len(), 1);
    assert_eq!(
        superseded_decisions.records[0].record_entity_id,
        prior.record_entity_id
    );
}
