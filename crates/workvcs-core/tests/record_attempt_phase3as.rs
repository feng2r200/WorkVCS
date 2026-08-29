use std::path::{Path, PathBuf};
use tempfile::TempDir;
use workvcs_core::{
    CanonicalValue, Engine, RecordCreateOptions, RecordKind, RecordListOptions, RecordState,
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
        StoreInitOptions::new("phase3as-attempt-record-store").expect("store options"),
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
fn attempt_record_create_commits_running_record() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);

    let record = engine
        .create_record(
            RecordCreateOptions::attempt(
                workspace.initial_branch_id,
                workspace.genesis_commit_id,
                "Run the next validation command",
            )
            .expect("attempt options"),
        )
        .expect("create attempt record");

    assert_eq!(record.state.kind, RecordKind::Attempt);
    assert_eq!(record.state.status, RecordStatus::Running);

    let loaded = engine
        .record_at(record.commit_id, record.record_entity_id)
        .expect("record at commit");
    assert_eq!(loaded.state, record.state);

    let attempts = engine
        .records_at(RecordListOptions::new(record.commit_id).with_kind(RecordKind::Attempt))
        .expect("attempt records");
    assert_eq!(attempts.records.len(), 1);
    assert_eq!(
        attempts.records[0].record_entity_id,
        record.record_entity_id
    );
    assert_eq!(attempts.records[0].state.status, RecordStatus::Running);
}

#[test]
fn attempt_record_rejects_non_attempt_statuses() {
    let state = RecordState {
        kind: RecordKind::Attempt,
        statement: "Run the next validation command".to_owned(),
        scope: CanonicalValue::object(Vec::new()).expect("scope"),
        status: RecordStatus::Active,
    };

    assert!(state.to_canonical_value().is_err());
}
