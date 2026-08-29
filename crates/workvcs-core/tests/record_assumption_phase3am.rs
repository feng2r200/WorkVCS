use std::path::{Path, PathBuf};
use tempfile::TempDir;
use workvcs_core::{
    CanonicalValue, Engine, ErrorCategory, ErrorCode, RecordCreateOptions, RecordKind,
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
        StoreInitOptions::new("phase3am-record-assumption-store").expect("store options"),
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
fn assumption_record_create_commits_unverified_record() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let scope = CanonicalValue::object(vec![(
        "applies_to".to_owned(),
        CanonicalValue::String("phase-3-runtime".to_owned()),
    )])
    .expect("scope object");

    let record = engine
        .create_record(
            RecordCreateOptions::assumption(
                workspace.initial_branch_id,
                workspace.genesis_commit_id,
                "Serialized writes are sufficient",
            )
            .expect("assumption options")
            .with_scope(scope.clone())
            .expect("scope"),
        )
        .expect("create assumption");

    assert_eq!(record.state.kind, RecordKind::Assumption);
    assert_eq!(record.state.status, RecordStatus::Unverified);
    assert_eq!(record.state.scope, scope);

    let loaded = engine
        .record_at(record.commit_id, record.record_entity_id)
        .expect("record at commit");
    assert_eq!(loaded.state, record.state);
    assert_eq!(loaded.state_digest, record.record_state_digest);
}

#[test]
fn assumption_record_rejects_empty_statement() {
    let (_tempdir, path) = store_path();
    let (_engine, workspace) = create_workspace(&path);

    let error = RecordCreateOptions::assumption(
        workspace.initial_branch_id,
        workspace.genesis_commit_id,
        "",
    )
    .expect_err("empty statement");
    assert_eq!(error.code(), ErrorCode::RecordInvalid);
    assert_eq!(error.category(), ErrorCategory::Record);
}
