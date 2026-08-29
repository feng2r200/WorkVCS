use std::path::{Path, PathBuf};
use tempfile::TempDir;
use workvcs_core::{
    Engine, RecordCreateOptions, RecordKind, RecordStatus, StoreInitOptions, WorkspaceInfo,
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
        StoreInitOptions::new("phase3ap-record-question-risk-store").expect("store options"),
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
fn question_record_create_commits_active_record() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);

    let record = engine
        .create_record(
            RecordCreateOptions::question(
                workspace.initial_branch_id,
                workspace.genesis_commit_id,
                "Which replay invariant should be exposed next?",
            )
            .expect("question options"),
        )
        .expect("create question record");

    assert_eq!(record.state.kind, RecordKind::Question);
    assert_eq!(record.state.status, RecordStatus::Active);
    let loaded = engine
        .record_at(record.commit_id, record.record_entity_id)
        .expect("record at commit");
    assert_eq!(loaded.state, record.state);
}

#[test]
fn risk_record_create_commits_active_record() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);

    let record = engine
        .create_record(
            RecordCreateOptions::risk(
                workspace.initial_branch_id,
                workspace.genesis_commit_id,
                "Manual ordering remains unresolved",
            )
            .expect("risk options"),
        )
        .expect("create risk record");

    assert_eq!(record.state.kind, RecordKind::Risk);
    assert_eq!(record.state.status, RecordStatus::Active);
    let loaded = engine
        .record_at(record.commit_id, record.record_entity_id)
        .expect("record at commit");
    assert_eq!(loaded.state, record.state);
}
