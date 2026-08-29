use std::path::{Path, PathBuf};
use tempfile::TempDir;
use workvcs_core::{
    Engine, ErrorCategory, ErrorCode, RecordCreateOptions, RecordKind, RecordListOptions,
    RecordStatus, RecordTransitionOptions, StoreInitOptions, WorkspaceInfo, WorkspaceInitOptions,
};

fn store_path() -> (TempDir, PathBuf) {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let path = tempdir.path().join("workvcs.sqlite");
    (tempdir, path)
}

fn init_engine(path: &Path) -> Engine {
    Engine::init(
        path,
        StoreInitOptions::new("phase3be-decision-lifecycle-store").expect("store options"),
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
fn decision_records_transition_from_active_to_terminal_statuses() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let mut head = workspace.genesis_commit_id;

    for (statement, target_status) in [
        (
            "Replace serialized-write decision",
            RecordStatus::Superseded,
        ),
        (
            "Withdraw unsafe migration decision",
            RecordStatus::Withdrawn,
        ),
    ] {
        let decision = engine
            .create_record(
                RecordCreateOptions::decision(workspace.initial_branch_id, head, statement)
                    .expect("decision options"),
            )
            .expect("create decision");
        let transition = engine
            .transition_record(
                decision_transition_options(
                    target_status,
                    workspace.initial_branch_id,
                    decision.commit_id,
                    decision.record_entity_id,
                    decision.record_entity_version_id,
                )
                .expect("decision transition options"),
            )
            .expect("transition decision");

        assert_eq!(transition.state.kind, RecordKind::Decision);
        assert_eq!(transition.previous_state.status, RecordStatus::Active);
        assert_eq!(transition.state.status, target_status);

        let loaded = engine
            .record_at(transition.commit_id, transition.record_entity_id)
            .expect("record at terminal commit");
        assert_eq!(loaded.state.status, target_status);

        head = transition.commit_id;
    }

    let decisions = engine
        .records_at(RecordListOptions::new(head).with_kind(RecordKind::Decision))
        .expect("decision records");
    assert_eq!(decisions.records.len(), 2);
    assert!(
        decisions
            .records
            .iter()
            .any(|record| record.state.status == RecordStatus::Superseded)
    );
    assert!(
        decisions
            .records
            .iter()
            .any(|record| record.state.status == RecordStatus::Withdrawn)
    );
}

#[test]
fn terminal_decision_record_cannot_transition_again() {
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
    let terminal = engine
        .transition_record(
            RecordTransitionOptions::supersede_decision(
                workspace.initial_branch_id,
                decision.commit_id,
                decision.record_entity_id,
                decision.record_entity_version_id,
                "A newer decision replaces this one",
            )
            .expect("superseded transition"),
        )
        .expect("supersede decision");

    let error = engine
        .transition_record(
            RecordTransitionOptions::withdraw_decision(
                workspace.initial_branch_id,
                terminal.commit_id,
                terminal.record_entity_id,
                terminal.record_entity_version_id,
                "Trying to change a terminal decision",
            )
            .expect("withdraw options"),
        )
        .expect_err("terminal decision transition should fail");

    assert_eq!(error.code(), ErrorCode::RecordInvalid);
    assert_eq!(error.category(), ErrorCategory::Record);
    assert!(
        error
            .to_string()
            .contains("decision transition Superseded -> Withdrawn is not allowed")
    );
}

#[test]
fn non_decision_record_does_not_use_decision_lifecycle() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let finding = engine
        .create_record(
            RecordCreateOptions::finding(
                workspace.initial_branch_id,
                workspace.genesis_commit_id,
                "A finding is not a decision",
            )
            .expect("finding options"),
        )
        .expect("create finding");

    let error = engine
        .transition_record(
            RecordTransitionOptions::withdraw_decision(
                workspace.initial_branch_id,
                finding.commit_id,
                finding.record_entity_id,
                finding.record_entity_version_id,
                "Not applicable",
            )
            .expect("withdraw options"),
        )
        .expect_err("finding decision transition should fail");

    assert_eq!(error.code(), ErrorCode::RecordInvalid);
    assert_eq!(error.category(), ErrorCategory::Record);
}

fn decision_transition_options(
    status: RecordStatus,
    branch_id: workvcs_core::BranchId,
    head_id: workvcs_core::CommitId,
    record_id: workvcs_core::EntityId,
    record_version_id: workvcs_core::EntityVersionId,
) -> workvcs_core::Result<RecordTransitionOptions> {
    match status {
        RecordStatus::Superseded => RecordTransitionOptions::supersede_decision(
            branch_id,
            head_id,
            record_id,
            record_version_id,
            "Decision superseded by a newer choice",
        ),
        RecordStatus::Withdrawn => RecordTransitionOptions::withdraw_decision(
            branch_id,
            head_id,
            record_id,
            record_version_id,
            "Decision withdrawn after review",
        ),
        other => panic!("unexpected decision terminal status {other:?}"),
    }
}
