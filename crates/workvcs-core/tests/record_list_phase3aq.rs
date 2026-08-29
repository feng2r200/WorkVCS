use std::path::{Path, PathBuf};
use tempfile::TempDir;
use workvcs_core::{
    Engine, RecordCreateOptions, RecordKind, RecordListOptions, RecordStatus,
    RecordTransitionOptions, StoreInitOptions, TaskCreateOptions, WorkspaceInfo,
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
        StoreInitOptions::new("phase3aq-record-list-store").expect("store options"),
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
fn records_at_lists_current_records_and_filters_by_kind() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);

    let task = engine
        .create_task(
            TaskCreateOptions::new(
                workspace.initial_branch_id,
                workspace.genesis_commit_id,
                "Build the record list projection",
            )
            .expect("task options"),
        )
        .expect("create task");
    let empty = engine
        .records_at(RecordListOptions::new(task.commit_id))
        .expect("records after only task");
    assert_eq!(empty.workspace_id, workspace.workspace_id);
    assert_eq!(empty.records, Vec::new());

    let finding = engine
        .create_record(
            RecordCreateOptions::finding(
                workspace.initial_branch_id,
                task.commit_id,
                "Schema validation has no drift",
            )
            .expect("finding options"),
        )
        .expect("create finding");
    let assumption = engine
        .create_record(
            RecordCreateOptions::assumption(
                workspace.initial_branch_id,
                finding.commit_id,
                "Serialized writes are sufficient",
            )
            .expect("assumption options"),
        )
        .expect("create assumption");
    let transition = engine
        .transition_record(
            RecordTransitionOptions::validate_assumption(
                workspace.initial_branch_id,
                assumption.commit_id,
                assumption.record_entity_id,
                assumption.record_entity_version_id,
                "Confirmed by local validation",
            )
            .expect("transition options"),
        )
        .expect("validate assumption");
    let decision = engine
        .create_record(
            RecordCreateOptions::decision(
                workspace.initial_branch_id,
                transition.commit_id,
                "Expose record listing as a thin tool command",
            )
            .expect("decision options"),
        )
        .expect("create decision");

    let after_finding = engine
        .records_at(RecordListOptions::new(finding.commit_id))
        .expect("records after finding");
    assert_eq!(after_finding.records.len(), 1);
    assert_eq!(after_finding.records[0].state.kind, RecordKind::Finding);

    let all = engine
        .records_at(RecordListOptions::new(decision.commit_id))
        .expect("records at final commit");
    assert_eq!(all.workspace_id, workspace.workspace_id);
    assert_eq!(all.commit_id, decision.commit_id);
    assert_eq!(all.records.len(), 3);
    assert!(
        !all.records
            .iter()
            .any(|record| record.record_entity_id == task.task_entity_id)
    );
    assert!(
        all.records
            .windows(2)
            .all(|records| records[0].record_entity_id <= records[1].record_entity_id)
    );

    let mut kinds = all
        .records
        .iter()
        .map(|record| record.state.kind.as_str())
        .collect::<Vec<_>>();
    kinds.sort_unstable();
    assert_eq!(kinds, ["assumption", "decision", "finding"]);

    let assumptions = engine
        .records_at(RecordListOptions::new(decision.commit_id).with_kind(RecordKind::Assumption))
        .expect("assumption records");
    assert_eq!(assumptions.records.len(), 1);
    assert_eq!(
        assumptions.records[0].record_entity_id,
        assumption.record_entity_id
    );
    assert_eq!(
        assumptions.records[0].record_entity_version_id,
        transition.record_entity_version_id
    );
    assert_eq!(assumptions.records[0].state.status, RecordStatus::Validated);

    let containing_validation = engine
        .records_at(
            RecordListOptions::new(decision.commit_id)
                .with_statement_contains("validation")
                .expect("statement filter"),
        )
        .expect("statement-filtered records");
    assert_eq!(containing_validation.records.len(), 1);
    assert_eq!(
        containing_validation.records[0].record_entity_id,
        finding.record_entity_id
    );

    let combined = engine
        .records_at(
            RecordListOptions::new(decision.commit_id)
                .with_kind(RecordKind::Assumption)
                .with_status(RecordStatus::Validated)
                .with_statement_contains("writes")
                .expect("combined statement filter"),
        )
        .expect("combined-filtered records");
    assert_eq!(combined.records.len(), 1);
    assert_eq!(
        combined.records[0].record_entity_id,
        assumption.record_entity_id
    );

    let empty = engine
        .records_at(
            RecordListOptions::new(decision.commit_id)
                .with_statement_contains("not present")
                .expect("missing statement filter"),
        )
        .expect("empty statement-filtered records");
    assert!(empty.records.is_empty());

    let empty_filter = RecordListOptions::new(decision.commit_id)
        .with_statement_contains(" ")
        .expect_err("empty statement filter should fail");
    assert!(empty_filter.to_string().contains("must not be empty"));
}
