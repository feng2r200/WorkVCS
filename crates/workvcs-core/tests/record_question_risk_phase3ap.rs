use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;
use workvcs_core::{
    BundleImportApplyOptions, BundleImportPreflightOptions, BundlePayloadExport,
    BundlePayloadExportOptions, BundlePayloadInput, Engine, RecordCreateOptions, RecordKind,
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

fn bundle_payload_inputs(export: &BundlePayloadExport) -> Vec<BundlePayloadInput> {
    export
        .payload_files
        .iter()
        .map(|payload| {
            BundlePayloadInput::new(payload.relative_path.clone(), payload.bytes.clone())
                .expect("payload input")
        })
        .collect()
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

#[test]
fn question_and_risk_support_only_their_terminal_lifecycles() {
    for (kind, terminal_status, options) in [
        (RecordKind::Question, RecordStatus::Answered, "answer"),
        (RecordKind::Question, RecordStatus::Deferred, "defer"),
        (RecordKind::Question, RecordStatus::Withdrawn, "withdraw"),
        (RecordKind::Risk, RecordStatus::Mitigated, "mitigate"),
        (RecordKind::Risk, RecordStatus::Invalidated, "invalidate"),
        (RecordKind::Risk, RecordStatus::Withdrawn, "withdraw"),
    ] {
        let (_tempdir, path) = store_path();
        let (mut engine, workspace) = create_workspace(&path);
        let created = engine
            .create_record(
                match kind {
                    RecordKind::Question => RecordCreateOptions::question(
                        workspace.initial_branch_id,
                        workspace.genesis_commit_id,
                        "lifecycle test question",
                    ),
                    RecordKind::Risk => RecordCreateOptions::risk(
                        workspace.initial_branch_id,
                        workspace.genesis_commit_id,
                        "lifecycle test risk",
                    ),
                    _ => unreachable!(),
                }
                .expect("create record"),
            )
            .expect("create commit");

        let transition = match (kind, options) {
            (RecordKind::Question, "answer") => RecordTransitionOptions::answer_question,
            (RecordKind::Question, "defer") => RecordTransitionOptions::defer_question,
            (RecordKind::Question, "withdraw") => RecordTransitionOptions::withdraw_question,
            (RecordKind::Risk, "mitigate") => RecordTransitionOptions::mitigate_risk,
            (RecordKind::Risk, "invalidate") => RecordTransitionOptions::invalidate_risk,
            (RecordKind::Risk, "withdraw") => RecordTransitionOptions::withdraw_risk,
            _ => unreachable!(),
        }(
            workspace.initial_branch_id,
            created.commit_id,
            created.record_entity_id,
            created.record_entity_version_id,
            "lifecycle rationale",
        )
        .expect("transition options");
        let terminal = engine
            .transition_record(transition)
            .expect("terminal transition");
        assert_eq!(terminal.state.status, terminal_status);
        assert_eq!(
            engine
                .record_at(terminal.commit_id, terminal.record_entity_id)
                .expect("current record")
                .state
                .status,
            terminal_status
        );

        let reopened = match kind {
            RecordKind::Question => RecordTransitionOptions::answer_question(
                workspace.initial_branch_id,
                terminal.commit_id,
                terminal.record_entity_id,
                terminal.record_entity_version_id,
                "reopen",
            ),
            RecordKind::Risk => RecordTransitionOptions::mitigate_risk(
                workspace.initial_branch_id,
                terminal.commit_id,
                terminal.record_entity_id,
                terminal.record_entity_version_id,
                "reopen",
            ),
            _ => unreachable!(),
        }
        .expect("reopen options");
        assert!(engine.transition_record(reopened).is_err());
    }
}

#[test]
fn question_and_risk_reject_wrong_kind_status_and_preserve_history() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let question = engine
        .create_record(
            RecordCreateOptions::question(
                workspace.initial_branch_id,
                workspace.genesis_commit_id,
                "history test question",
            )
            .expect("question options"),
        )
        .expect("create question");
    let wrong_kind = RecordTransitionOptions::mitigate_risk(
        workspace.initial_branch_id,
        question.commit_id,
        question.record_entity_id,
        question.record_entity_version_id,
        "wrong kind",
    )
    .expect("options are syntactically valid");
    assert!(engine.transition_record(wrong_kind).is_err());

    let answered = engine
        .transition_record(
            RecordTransitionOptions::answer_question(
                workspace.initial_branch_id,
                question.commit_id,
                question.record_entity_id,
                question.record_entity_version_id,
                "answered",
            )
            .expect("answer options"),
        )
        .expect("answer question");
    let historical = engine
        .record_at(question.commit_id, question.record_entity_id)
        .expect("historical record");
    assert_eq!(historical.state.status, RecordStatus::Active);
    assert_eq!(
        engine
            .record_at(answered.commit_id, answered.record_entity_id)
            .expect("head record")
            .state
            .status,
        RecordStatus::Answered
    );
    assert_eq!(RecordStatus::Answered.as_str(), "answered");
    assert_eq!(RecordStatus::Mitigated.as_str(), "mitigated");
}

#[test]
fn same_store_bundle_round_trip_preserves_terminal_question_and_risk() {
    let (tempdir, source_path) = store_path();
    let target_path = tempdir.path().join("target.sqlite");
    let (source, workspace) = create_workspace(&source_path);
    drop(source);
    fs::copy(&source_path, &target_path).expect("copy baseline Store");

    let mut source = Engine::open(&source_path).expect("open source Store");
    let question = source
        .create_record(
            RecordCreateOptions::question(
                workspace.initial_branch_id,
                workspace.genesis_commit_id,
                "Is terminal cognition portable?",
            )
            .expect("question options"),
        )
        .expect("create question");
    let answered = source
        .transition_record(
            RecordTransitionOptions::answer_question(
                workspace.initial_branch_id,
                question.commit_id,
                question.record_entity_id,
                question.record_entity_version_id,
                "The copied-target round trip answers it",
            )
            .expect("answer options"),
        )
        .expect("answer question");
    let risk = source
        .create_record(
            RecordCreateOptions::risk(
                workspace.initial_branch_id,
                answered.commit_id,
                "Terminal status might be lost during Bundle apply",
            )
            .expect("risk options"),
        )
        .expect("create risk");
    let mitigated = source
        .transition_record(
            RecordTransitionOptions::mitigate_risk(
                workspace.initial_branch_id,
                risk.commit_id,
                risk.record_entity_id,
                risk.record_entity_version_id,
                "The exact Bundle round trip preserves it",
            )
            .expect("mitigate options"),
        )
        .expect("mitigate risk");
    let export = source
        .export_bundle_payloads(BundlePayloadExportOptions::for_commit(mitigated.commit_id))
        .expect("export Bundle");
    let payloads = bundle_payload_inputs(&export);

    let mut target = Engine::open(&target_path).expect("open copied target Store");
    let preflight = target
        .preflight_bundle_import(
            BundleImportPreflightOptions::from_parts(
                export.manifest_bytes.clone(),
                export.payload_index_bytes.clone(),
                payloads.clone(),
            )
            .expect("preflight options"),
        )
        .expect("preflight Bundle");
    assert_eq!(preflight.action, "same_store_fast_forward_ready");
    assert!(preflight.can_apply);

    let applied = target
        .apply_bundle_import(
            BundleImportApplyOptions::from_parts(
                export.manifest_bytes,
                export.payload_index_bytes,
                payloads,
            )
            .expect("apply options"),
        )
        .expect("apply Bundle");
    assert_eq!(applied.outcome, "same_store_fast_forward_applied");
    assert_eq!(applied.imported_commits, 4);
    assert_eq!(applied.imported_entity_versions, 4);

    assert_eq!(
        target
            .record_at(mitigated.commit_id, question.record_entity_id)
            .expect("imported Question")
            .state
            .status,
        RecordStatus::Answered
    );
    assert_eq!(
        target
            .record_at(mitigated.commit_id, risk.record_entity_id)
            .expect("imported Risk")
            .state
            .status,
        RecordStatus::Mitigated
    );
    target.validate_integrity().expect("target Store integrity");
}
