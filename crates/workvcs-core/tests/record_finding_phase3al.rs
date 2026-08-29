use std::path::{Path, PathBuf};
use tempfile::TempDir;
use workvcs_core::{
    CanonicalValue, Engine, EntityTransitionOptions, ErrorCategory, ErrorCode, RecordCreateOptions,
    RecordKind, RecordStatus, StoreInitOptions, WorkspaceInfo, WorkspaceInitOptions,
};

fn store_path() -> (TempDir, PathBuf) {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let path = tempdir.path().join("workvcs.sqlite");
    (tempdir, path)
}

fn init_engine(path: &Path) -> Engine {
    Engine::init(
        path,
        StoreInitOptions::new("phase3al-record-finding-store").expect("store options"),
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
fn finding_record_create_commits_versioned_work_state() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let scope = CanonicalValue::object(vec![(
        "subject".to_owned(),
        CanonicalValue::String("schema-validation".to_owned()),
    )])
    .expect("scope object");

    let record = engine
        .create_record(
            RecordCreateOptions::finding(
                workspace.initial_branch_id,
                workspace.genesis_commit_id,
                "Schema validation has no drift",
            )
            .expect("finding options")
            .with_scope(scope.clone())
            .expect("scope"),
        )
        .expect("create finding");

    assert_eq!(record.workspace_id, workspace.workspace_id);
    assert_eq!(record.branch_id, workspace.initial_branch_id);
    assert_eq!(record.previous_head_commit_id, workspace.genesis_commit_id);
    assert_eq!(record.state.kind, RecordKind::Finding);
    assert_eq!(record.state.status, RecordStatus::Active);
    assert_eq!(record.state.scope, scope);

    let loaded = engine
        .record_at(record.commit_id, record.record_entity_id)
        .expect("record at commit");
    assert_eq!(
        loaded.record_entity_version_id,
        record.record_entity_version_id
    );
    assert_eq!(loaded.state_digest, record.record_state_digest);
    assert_eq!(loaded.state, record.state);

    let head = engine
        .branch_head(workspace.initial_branch_id)
        .expect("branch head");
    assert_eq!(head.head_commit_id, record.commit_id);
    assert_eq!(head.state_digest, record.work_state_digest);
}

#[test]
fn finding_record_rejects_empty_statement() {
    let (_tempdir, path) = store_path();
    let (_engine, workspace) = create_workspace(&path);

    let error = RecordCreateOptions::finding(
        workspace.initial_branch_id,
        workspace.genesis_commit_id,
        "   ",
    )
    .expect_err("empty statement");
    assert_eq!(error.code(), ErrorCode::RecordInvalid);
    assert_eq!(error.category(), ErrorCategory::Record);
}

#[test]
fn record_entity_kind_is_reserved_for_semantic_api() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let state = CanonicalValue::object(Vec::new()).expect("state");
    let options = EntityTransitionOptions::create(
        workspace.initial_branch_id,
        workspace.genesis_commit_id,
        "record",
        state,
    )
    .expect("generic entity transition");

    let error = engine
        .commit_entity_transition(options)
        .expect_err("reserved record kind");
    assert_eq!(error.code(), ErrorCode::EntityTransitionInvalid);
    assert_eq!(error.category(), ErrorCategory::Mutation);
}
