use std::path::{Path, PathBuf};
use tempfile::TempDir;
use workvcs_core::{
    Engine, ErrorCategory, ErrorCode, RecordCreateOptions, RecordStatus, RecordTransitionOptions,
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
        StoreInitOptions::new("phase3an-assumption-transition-store").expect("store options"),
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
fn assumption_record_allows_confirmed_transition_graph() {
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

    let validated = engine
        .transition_record(
            RecordTransitionOptions::validate_assumption(
                workspace.initial_branch_id,
                assumption.commit_id,
                assumption.record_entity_id,
                assumption.record_entity_version_id,
                "Validated by runtime test coverage",
            )
            .expect("validate options"),
        )
        .expect("validate assumption");
    assert_eq!(validated.previous_state.status, RecordStatus::Unverified);
    assert_eq!(validated.state.status, RecordStatus::Validated);

    let invalidated = engine
        .transition_record(
            RecordTransitionOptions::invalidate_assumption(
                workspace.initial_branch_id,
                validated.commit_id,
                validated.record_entity_id,
                validated.record_entity_version_id,
                "Later evidence changed the condition",
            )
            .expect("invalidate options"),
        )
        .expect("invalidate assumption");
    assert_eq!(invalidated.previous_state.status, RecordStatus::Validated);
    assert_eq!(invalidated.state.status, RecordStatus::Invalidated);
}

#[test]
fn invalidated_assumption_cannot_be_revalidated() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let assumption = engine
        .create_record(
            RecordCreateOptions::assumption(
                workspace.initial_branch_id,
                workspace.genesis_commit_id,
                "A temporary assumption",
            )
            .expect("assumption options"),
        )
        .expect("create assumption");
    let invalidated = engine
        .transition_record(
            RecordTransitionOptions::invalidate_assumption(
                workspace.initial_branch_id,
                assumption.commit_id,
                assumption.record_entity_id,
                assumption.record_entity_version_id,
                "Counterexample found",
            )
            .expect("invalidate options"),
        )
        .expect("invalidate assumption");

    let error = engine
        .transition_record(
            RecordTransitionOptions::validate_assumption(
                workspace.initial_branch_id,
                invalidated.commit_id,
                invalidated.record_entity_id,
                invalidated.record_entity_version_id,
                "Trying to revalidate",
            )
            .expect("validate options"),
        )
        .expect_err("invalidated assumption revalidation");
    assert_eq!(error.code(), ErrorCode::RecordInvalid);
    assert_eq!(error.category(), ErrorCategory::Record);
}

#[test]
fn finding_record_does_not_use_assumption_lifecycle() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let finding = engine
        .create_record(
            RecordCreateOptions::finding(
                workspace.initial_branch_id,
                workspace.genesis_commit_id,
                "A finding is not an assumption",
            )
            .expect("finding options"),
        )
        .expect("create finding");

    let error = engine
        .transition_record(
            RecordTransitionOptions::validate_assumption(
                workspace.initial_branch_id,
                finding.commit_id,
                finding.record_entity_id,
                finding.record_entity_version_id,
                "Not applicable",
            )
            .expect("validate options"),
        )
        .expect_err("finding assumption transition");
    assert_eq!(error.code(), ErrorCode::RecordInvalid);
    assert_eq!(error.category(), ErrorCategory::Record);
}
