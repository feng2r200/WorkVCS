use std::path::{Path, PathBuf};
use tempfile::TempDir;
use workvcs_core::{
    CanonicalValue, Engine, RecordCreateOptions, RecordKind, RecordListOptions, RecordStatus,
    StoreInitOptions, WorkspaceInfo, WorkspaceInitOptions,
};

fn store_path() -> (TempDir, PathBuf) {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let path = tempdir.path().join("workvcs.sqlite");
    (tempdir, path)
}

fn init_engine(path: &Path) -> Engine {
    Engine::init(
        path,
        StoreInitOptions::new("phase3au-handoff-record-store").expect("store options"),
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
fn handoff_record_create_commits_active_record() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let scope = CanonicalValue::object(vec![(
        "focus".to_owned(),
        CanonicalValue::String("task:next".to_owned()),
    )])
    .expect("scope");

    let record = engine
        .create_record(
            RecordCreateOptions::handoff(
                workspace.initial_branch_id,
                workspace.genesis_commit_id,
                "Continue with the next runnable task",
            )
            .expect("handoff options")
            .with_scope(scope.clone())
            .expect("handoff scope"),
        )
        .expect("create handoff record");

    assert_eq!(record.state.kind, RecordKind::Handoff);
    assert_eq!(record.state.status, RecordStatus::Active);
    assert_eq!(record.state.scope, scope);

    let loaded = engine
        .record_at(record.commit_id, record.record_entity_id)
        .expect("record at commit");
    assert_eq!(loaded.state, record.state);

    let handoffs = engine
        .records_at(RecordListOptions::new(record.commit_id).with_kind(RecordKind::Handoff))
        .expect("handoff records");
    assert_eq!(handoffs.records.len(), 1);
    assert_eq!(
        handoffs.records[0].record_entity_id,
        record.record_entity_id
    );
}
