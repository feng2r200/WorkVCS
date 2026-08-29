use std::path::{Path, PathBuf};
use tempfile::TempDir;
use workvcs_core::{
    BranchId, CommitId, Engine, EntityId, EntityVersionId, RecordCreateOptions, RecordKind,
    RecordListOptions, RecordStatus, RecordTransitionOptions, StoreInitOptions, WorkspaceInfo,
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
        StoreInitOptions::new("phase3at-attempt-transition-store").expect("store options"),
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
fn attempt_records_transition_from_running_to_terminal_statuses() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let mut head = workspace.genesis_commit_id;

    for (statement, target_status) in [
        ("Attempt succeeded", RecordStatus::Succeeded),
        ("Attempt failed", RecordStatus::Failed),
        ("Attempt inconclusive", RecordStatus::Inconclusive),
    ] {
        let attempt = engine
            .create_record(
                RecordCreateOptions::attempt(workspace.initial_branch_id, head, statement)
                    .expect("attempt options"),
            )
            .expect("create attempt");
        let transition = engine
            .transition_record(
                attempt_transition_options(
                    target_status,
                    workspace.initial_branch_id,
                    attempt.commit_id,
                    attempt.record_entity_id,
                    attempt.record_entity_version_id,
                )
                .expect("transition options"),
            )
            .expect("transition attempt");

        assert_eq!(transition.state.kind, RecordKind::Attempt);
        assert_eq!(transition.previous_state.status, RecordStatus::Running);
        assert_eq!(transition.state.status, target_status);

        let loaded = engine
            .record_at(transition.commit_id, transition.record_entity_id)
            .expect("record at terminal commit");
        assert_eq!(loaded.state.status, target_status);

        head = transition.commit_id;
    }

    let attempts = engine
        .records_at(RecordListOptions::new(head).with_kind(RecordKind::Attempt))
        .expect("attempt records");
    assert_eq!(attempts.records.len(), 3);
    assert!(
        attempts
            .records
            .iter()
            .any(|record| record.state.status == RecordStatus::Succeeded)
    );
    assert!(
        attempts
            .records
            .iter()
            .any(|record| record.state.status == RecordStatus::Failed)
    );
    assert!(
        attempts
            .records
            .iter()
            .any(|record| record.state.status == RecordStatus::Inconclusive)
    );
}

#[test]
fn terminal_attempt_record_cannot_transition_again() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);

    let attempt = engine
        .create_record(
            RecordCreateOptions::attempt(
                workspace.initial_branch_id,
                workspace.genesis_commit_id,
                "Run validation",
            )
            .expect("attempt options"),
        )
        .expect("create attempt");
    let terminal = engine
        .transition_record(
            RecordTransitionOptions::complete_attempt_succeeded(
                workspace.initial_branch_id,
                attempt.commit_id,
                attempt.record_entity_id,
                attempt.record_entity_version_id,
                "Validation passed",
            )
            .expect("succeeded transition"),
        )
        .expect("transition attempt");

    let err = engine
        .transition_record(
            RecordTransitionOptions::complete_attempt_failed(
                workspace.initial_branch_id,
                terminal.commit_id,
                terminal.record_entity_id,
                terminal.record_entity_version_id,
                "Tried to reopen terminal attempt",
            )
            .expect("failed transition options"),
        )
        .expect_err("terminal attempt should not transition again");

    assert!(
        err.to_string()
            .contains("attempt transition Succeeded -> Failed is not allowed")
    );
}

fn attempt_transition_options(
    status: RecordStatus,
    branch_id: BranchId,
    head_id: CommitId,
    record_id: EntityId,
    record_version_id: EntityVersionId,
) -> workvcs_core::Result<RecordTransitionOptions> {
    match status {
        RecordStatus::Succeeded => RecordTransitionOptions::complete_attempt_succeeded(
            branch_id,
            head_id,
            record_id,
            record_version_id,
            "Attempt succeeded",
        ),
        RecordStatus::Failed => RecordTransitionOptions::complete_attempt_failed(
            branch_id,
            head_id,
            record_id,
            record_version_id,
            "Attempt failed",
        ),
        RecordStatus::Inconclusive => RecordTransitionOptions::complete_attempt_inconclusive(
            branch_id,
            head_id,
            record_id,
            record_version_id,
            "Attempt inconclusive",
        ),
        other => panic!("unexpected attempt terminal status {other:?}"),
    }
}
