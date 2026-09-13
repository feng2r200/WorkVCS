use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;
use workvcs_core::{
    CanonicalValue, Engine, RecordCreateOptions, RecordCurrentnessAuditOptions,
    RecordCurrentnessClass, RecordKind, RecordStatus, RecordTransitionOptions, StoreInitOptions,
    WorkspaceInfo, WorkspaceInitOptions,
};

fn store_path() -> (TempDir, PathBuf) {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let path = tempdir.path().join("workvcs.sqlite");
    (tempdir, path)
}

fn create_workspace(path: &Path) -> (Engine, WorkspaceInfo) {
    let mut engine = Engine::init(
        path,
        StoreInitOptions::new("record-currentness-audit-store").expect("store options"),
    )
    .expect("init engine");
    let workspace = engine
        .create_workspace(WorkspaceInitOptions::new("workspace").expect("workspace options"))
        .expect("create workspace");
    (engine, workspace)
}

#[test]
fn audit_separates_open_obligations_from_optional_current_claims() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let branch = workspace.initial_branch_id;

    let risk = engine
        .create_record(
            RecordCreateOptions::risk(branch, workspace.genesis_commit_id, "Active risk")
                .expect("risk options"),
        )
        .expect("create risk");
    let question = engine
        .create_record(
            RecordCreateOptions::question(branch, risk.commit_id, "Answered question")
                .expect("question options"),
        )
        .expect("create question");
    let answered = engine
        .transition_record(
            RecordTransitionOptions::answer_question(
                branch,
                question.commit_id,
                question.record_entity_id,
                question.record_entity_version_id,
                "Current evidence answers it",
            )
            .expect("answer options"),
        )
        .expect("answer question");
    let finding = engine
        .create_record(
            RecordCreateOptions::finding(branch, answered.commit_id, "Current finding")
                .expect("finding options"),
        )
        .expect("create finding");
    let assumption = engine
        .create_record(
            RecordCreateOptions::assumption(branch, finding.commit_id, "Validated assumption")
                .expect("assumption options"),
        )
        .expect("create assumption");
    let validated = engine
        .transition_record(
            RecordTransitionOptions::validate_assumption(
                branch,
                assumption.commit_id,
                assumption.record_entity_id,
                assumption.record_entity_version_id,
                "A focused probe validated it",
            )
            .expect("validate options"),
        )
        .expect("validate assumption");
    let attempt = engine
        .create_record(
            RecordCreateOptions::attempt(branch, validated.commit_id, "Running attempt")
                .expect("attempt options"),
        )
        .expect("create attempt");

    let obligations = engine
        .record_currentness_audit(RecordCurrentnessAuditOptions::new(attempt.commit_id))
        .expect("audit open obligations");
    assert_eq!(obligations.counts.candidates_total, 2);
    assert_eq!(obligations.counts.open_obligations_total, 2);
    assert_eq!(obligations.counts.current_claims_total, 0);
    assert_eq!(obligations.counts.risks_total, 1);
    assert_eq!(obligations.counts.attempts_total, 1);
    assert!(obligations.candidates.iter().all(|candidate| {
        candidate.class == RecordCurrentnessClass::OpenObligation
            && matches!(
                (candidate.record.state.kind, candidate.record.state.status),
                (RecordKind::Risk, RecordStatus::Active)
                    | (RecordKind::Attempt, RecordStatus::Running)
            )
    }));
    assert_eq!(
        obligations.candidates[0].record.record_entity_id,
        attempt.record_entity_id
    );

    let all_candidates = engine
        .record_currentness_audit(
            RecordCurrentnessAuditOptions::new(attempt.commit_id).include_current_claims(),
        )
        .expect("audit current claims");
    assert_eq!(all_candidates.counts.candidates_total, 4);
    assert_eq!(all_candidates.counts.open_obligations_total, 2);
    assert_eq!(all_candidates.counts.current_claims_total, 2);
    assert_eq!(all_candidates.counts.findings_total, 1);
    assert_eq!(all_candidates.counts.assumptions_total, 1);
    assert!(
        !all_candidates
            .candidates
            .iter()
            .any(|candidate| { candidate.record.record_entity_id == question.record_entity_id })
    );
}

#[test]
fn audit_is_bounded_filterable_historical_and_read_only() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let branch = workspace.initial_branch_id;
    let selected_scope = CanonicalValue::object(vec![(
        "area".to_owned(),
        CanonicalValue::String("selected".to_owned()),
    )])
    .expect("selected scope");

    let first = engine
        .create_record(
            RecordCreateOptions::risk(branch, workspace.genesis_commit_id, "First scoped risk")
                .expect("risk options")
                .with_scope(selected_scope.clone())
                .expect("scoped risk options"),
        )
        .expect("create first risk");
    let second = engine
        .create_record(
            RecordCreateOptions::risk(branch, first.commit_id, "Second scoped risk")
                .expect("risk options")
                .with_scope(selected_scope.clone())
                .expect("scoped risk options"),
        )
        .expect("create second risk");
    let third = engine
        .create_record(
            RecordCreateOptions::question(branch, second.commit_id, "Unrelated question")
                .expect("question options"),
        )
        .expect("create question");
    let final_head = third.commit_id;
    drop(engine);

    let bytes_before = fs::read(&path).expect("read Store before audit");
    let readonly = Engine::open_readonly(&path).expect("open read-only engine");
    let bounded = readonly
        .record_currentness_audit(
            RecordCurrentnessAuditOptions::new(final_head)
                .with_kind(RecordKind::Risk)
                .with_scope(selected_scope)
                .expect("scope filter")
                .with_statement_contains("scoped")
                .expect("statement filter")
                .with_budget(1)
                .expect("bounded options"),
        )
        .expect("bounded audit");
    assert_eq!(bounded.counts.candidates_total, 2);
    assert_eq!(bounded.candidates.len(), 1);
    assert_eq!(bounded.omitted, 1);
    assert_eq!(
        bounded.candidates[0].record.record_entity_id,
        second.record_entity_id
    );

    let historical = readonly
        .record_currentness_audit(RecordCurrentnessAuditOptions::new(first.commit_id))
        .expect("historical audit");
    assert_eq!(historical.counts.candidates_total, 1);
    assert_eq!(
        historical.candidates[0].record.record_entity_id,
        first.record_entity_id
    );
    drop(readonly);
    assert_eq!(
        fs::read(&path).expect("read Store after audit"),
        bytes_before
    );

    assert!(
        RecordCurrentnessAuditOptions::new(final_head)
            .with_budget(0)
            .is_err()
    );
    assert!(
        RecordCurrentnessAuditOptions::new(final_head)
            .with_budget(201)
            .is_err()
    );
}
