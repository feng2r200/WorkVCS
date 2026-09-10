use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use tempfile::TempDir;
use workvcs_core::{
    AcceptanceCriterionClassification, AcceptanceCriterionCreateCommit,
    AcceptanceCriterionCreateOptions, AuthorizationReceiptConsumeManifest,
    AuthorizationReceiptConsumeOptions, AuthorizationReceiptIssueManifest,
    AuthorizationReceiptIssueOptions, CanonicalValue, ClaimTaskOptions, CloseoutInspectCategory,
    CloseoutInspectOptions, CloseoutInspectRuntimeGapCategory, Digest, Engine, EntityId,
    GoalCreateCommit, GoalCreateOptions, PlanCreateCommit, PlanCreateOptions,
    PrimaryContainmentCreateOptions, RecordCreateOptions, SessionFocusOptions, SessionId,
    SessionStartOptions, StoreInitOptions, TaskCreateCommit, TaskCreateOptions, WorkspaceInfo,
    WorkspaceInitOptions,
};

fn store_path() -> (TempDir, PathBuf) {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let path = tempdir.path().join("workvcs.sqlite");
    (tempdir, path)
}

fn create_workspace(path: &Path) -> (Engine, WorkspaceInfo) {
    let mut engine = Engine::init(
        path,
        StoreInitOptions::new("closeout-runtime-summary-p0-store").expect("store options"),
    )
    .expect("init engine");
    let workspace = engine
        .create_workspace(WorkspaceInitOptions::new("workspace").expect("workspace options"))
        .expect("create workspace");
    (engine, workspace)
}

fn create_task(
    engine: &mut Engine,
    workspace: &WorkspaceInfo,
    expected_head_commit_id: workvcs_core::CommitId,
    description: &str,
) -> TaskCreateCommit {
    engine
        .create_task(
            TaskCreateOptions::new(
                workspace.initial_branch_id,
                expected_head_commit_id,
                description,
            )
            .expect("task options"),
        )
        .expect("create task")
}

fn create_plan(
    engine: &mut Engine,
    workspace: &WorkspaceInfo,
    expected_head_commit_id: workvcs_core::CommitId,
) -> PlanCreateCommit {
    engine
        .create_plan(
            PlanCreateOptions::new(
                workspace.initial_branch_id,
                expected_head_commit_id,
                "Runtime summary plan",
                "Keep runtime links mechanical",
            )
            .expect("plan options"),
        )
        .expect("create plan")
}

fn create_goal(
    engine: &mut Engine,
    workspace: &WorkspaceInfo,
    expected_head_commit_id: workvcs_core::CommitId,
) -> GoalCreateCommit {
    engine
        .create_goal(
            GoalCreateOptions::new(
                workspace.initial_branch_id,
                expected_head_commit_id,
                "Runtime summary goal",
            )
            .expect("goal options"),
        )
        .expect("create goal")
}

fn create_acceptance_criterion(
    engine: &mut Engine,
    workspace: &WorkspaceInfo,
    task_entity_id: EntityId,
    expected_head_commit_id: workvcs_core::CommitId,
    expected_task_entity_version_id: workvcs_core::EntityVersionId,
    local_key: &str,
) -> AcceptanceCriterionCreateCommit {
    engine
        .create_acceptance_criterion(
            AcceptanceCriterionCreateOptions::new(
                workspace.initial_branch_id,
                expected_head_commit_id,
                task_entity_id,
                expected_task_entity_version_id,
                local_key,
                format!("{local_key} must be checked."),
                AcceptanceCriterionClassification::Required,
            )
            .expect("acceptance criterion options"),
        )
        .expect("create acceptance criterion")
}

fn digest(label: &str) -> String {
    Digest::domain_separated("workvcs.test.closeout-runtime-summary", label.as_bytes()).to_string()
}

fn now_us() -> i64 {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time after epoch");
    i64::try_from(duration.as_micros()).expect("now fits i64")
}

#[derive(Clone, Copy)]
struct ReceiptTarget<'a> {
    entity_id: EntityId,
    entity_kind: &'a str,
    entity_version_id: workvcs_core::EntityVersionId,
    state_digest: Digest,
}

fn issue_manifest_json(
    target: ReceiptTarget<'_>,
    key: &str,
    action: &str,
    expected_head_commit_id: workvcs_core::CommitId,
    expected_state_digest: Digest,
    authority_ref: &str,
    authority_digest: &str,
    expires_at_us: Option<i64>,
) -> String {
    let expires = expires_at_us
        .map(|value| format!(r#","expires_at_us":{value}"#))
        .unwrap_or_default();
    format!(
        r#"{{
  "schema_version": 1,
  "idempotency_key": "{key}",
  "expected_head_commit_id": "{expected_head_commit_id}",
  "expected_state_digest": "{expected_state_digest}",
  "target": {{
    "entity_id": "{target_entity_id}",
    "entity_kind": "{target_entity_kind}",
    "expected_version_id": "{target_version_id}",
    "expected_state_digest": "{target_state_digest}"
  }},
  "action": "{action}",
  "contract_digest_domain": "work-governance/action-contract/v1",
  "contract_digest": "{contract_digest}",
  "authority_ref": {{
    "kind": "user-session",
    "ref": "{authority_ref}",
    "authority_digest": "{authority_digest}"
  }}{expires},
  "rationale": {{"why":"mechanical closeout runtime summary test"}}
}}"#,
        target_entity_id = target.entity_id,
        target_entity_kind = target.entity_kind,
        target_version_id = target.entity_version_id,
        target_state_digest = target.state_digest,
        contract_digest = digest("contract"),
    )
}

fn parse_issue_manifest(json: &str) -> AuthorizationReceiptIssueManifest {
    AuthorizationReceiptIssueManifest::from_json_bytes(json.as_bytes())
        .expect("parse receipt issue manifest")
}

fn consume_manifest_json(
    target: ReceiptTarget<'_>,
    issued: &workvcs_core::AuthorizationReceiptIssueResult,
    key: &str,
    expected_head_commit_id: workvcs_core::CommitId,
    expected_state_digest: Digest,
) -> String {
    format!(
        r#"{{
  "schema_version": 1,
  "idempotency_key": "{key}",
  "expected_head_commit_id": "{expected_head_commit_id}",
  "expected_state_digest": "{expected_state_digest}",
  "receipt_record_entity_id": "{receipt}",
  "expected_receipt_entity_version_id": "{receipt_version}",
  "expected_receipt_state_digest": "{receipt_digest}",
  "target": {{
    "entity_id": "{target_entity_id}",
    "entity_kind": "{target_entity_kind}",
    "expected_version_id": "{target_version_id}",
    "expected_state_digest": "{target_state_digest}"
  }},
  "action": "{action}",
  "contract_digest": "{contract}",
  "contract_digest_domain": "work-governance/action-contract/v1",
  "rationale": {{"why":"consume binding"}}
}}"#,
        receipt = issued.receipt.entity_id,
        receipt_version = issued.receipt.entity_version_id,
        receipt_digest = issued.receipt.state_digest,
        target_entity_id = target.entity_id,
        target_entity_kind = target.entity_kind,
        target_version_id = target.entity_version_id,
        target_state_digest = target.state_digest,
        action = issued.receipt.binding.action,
        contract = digest("contract"),
    )
}

fn parse_consume_manifest(json: &str) -> AuthorizationReceiptConsumeManifest {
    AuthorizationReceiptConsumeManifest::from_json_bytes(json.as_bytes())
        .expect("parse receipt consume manifest")
}

fn focused_handoff_scope(
    focus_entity_id: EntityId,
    session_id: Option<SessionId>,
) -> CanonicalValue {
    let session = session_id
        .map(|id| CanonicalValue::String(id.to_string()))
        .unwrap_or(CanonicalValue::Null);
    CanonicalValue::object(vec![
        (
            "focus_entity_id".to_owned(),
            CanonicalValue::String(focus_entity_id.to_string()),
        ),
        (
            "handoff_scope_schema_version".to_owned(),
            CanonicalValue::safe_integer(1).expect("safe integer"),
        ),
        ("session_diff_id".to_owned(), CanonicalValue::Null),
        ("session_id".to_owned(), session),
        (
            "session_lifecycle_state".to_owned(),
            CanonicalValue::String("ended".to_owned()),
        ),
    ])
    .expect("focused handoff scope")
}

fn create_handoff(
    engine: &mut Engine,
    workspace: &WorkspaceInfo,
    expected_head_commit_id: workvcs_core::CommitId,
    focus_entity_id: EntityId,
    session_id: Option<SessionId>,
) -> workvcs_core::RecordCreateCommit {
    engine
        .create_record(
            RecordCreateOptions::handoff(
                workspace.initial_branch_id,
                expected_head_commit_id,
                "Focused handoff",
            )
            .expect("handoff options")
            .with_scope(focused_handoff_scope(focus_entity_id, session_id))
            .expect("handoff scope"),
        )
        .expect("create handoff")
}

fn create_handoff_with_scope(
    engine: &mut Engine,
    workspace: &WorkspaceInfo,
    expected_head_commit_id: workvcs_core::CommitId,
    statement: &str,
    scope: CanonicalValue,
) -> workvcs_core::RecordCreateCommit {
    engine
        .create_record(
            RecordCreateOptions::handoff(
                workspace.initial_branch_id,
                expected_head_commit_id,
                statement,
            )
            .expect("handoff options")
            .with_scope(scope)
            .expect("handoff scope"),
        )
        .expect("create handoff with scope")
}

#[test]
fn task_runtime_summary_reports_exact_runtime_links_and_redacted_receipts() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let task = create_task(
        &mut engine,
        &workspace,
        workspace.genesis_commit_id,
        "Runtime linked task",
    );
    let unrelated_task = create_task(
        &mut engine,
        &workspace,
        task.commit_id,
        "Unrelated runtime task",
    );
    let focused_session = engine
        .start_session(
            SessionStartOptions::new(workspace.workspace_id, workspace.initial_branch_id)
                .expect("session options"),
        )
        .expect("start focused session");
    engine
        .set_session_focus(
            SessionFocusOptions::new(focused_session.session_id, task.task_entity_id).with_path(
                vec![workvcs_core::SessionFocusPathEntry::new(
                    task.task_entity_id,
                    None,
                )],
            ),
        )
        .expect("set focused session");
    engine
        .claim_task(ClaimTaskOptions::new(
            focused_session.session_id,
            task.task_entity_id,
        ))
        .expect("claim focused task");
    let unrelated_session = engine
        .start_session(
            SessionStartOptions::new(workspace.workspace_id, workspace.initial_branch_id)
                .expect("session options"),
        )
        .expect("start unrelated session");
    engine
        .set_session_focus(SessionFocusOptions::new(
            unrelated_session.session_id,
            unrelated_task.task_entity_id,
        ))
        .expect("set unrelated focus");
    engine
        .claim_task(ClaimTaskOptions::new(
            unrelated_session.session_id,
            unrelated_task.task_entity_id,
        ))
        .expect("claim unrelated task");

    let handoff = create_handoff(
        &mut engine,
        &workspace,
        unrelated_task.commit_id,
        task.task_entity_id,
        Some(focused_session.session_id),
    );
    let unrelated_handoff = create_handoff(
        &mut engine,
        &workspace,
        handoff.commit_id,
        unrelated_task.task_entity_id,
        Some(unrelated_session.session_id),
    );

    let raw_authority_ref = "user:session/raw-secret-ref-must-not-appear";
    let task_target = ReceiptTarget {
        entity_id: task.task_entity_id,
        entity_kind: "task",
        entity_version_id: task.task_entity_version_id,
        state_digest: task.task_state_digest,
    };
    let consumed_issue = engine
        .issue_authorization_receipt(AuthorizationReceiptIssueOptions::new(
            workspace.initial_branch_id,
            parse_issue_manifest(&issue_manifest_json(
                task_target,
                "closeout-consumed",
                "deploy",
                unrelated_handoff.commit_id,
                unrelated_handoff.work_state_digest,
                raw_authority_ref,
                &digest("authority-consumed"),
                None,
            )),
        ))
        .expect("issue consumed receipt");
    let expires_at_us = now_us() + 1_000_000;
    let expired_issue = engine
        .issue_authorization_receipt(AuthorizationReceiptIssueOptions::new(
            workspace.initial_branch_id,
            parse_issue_manifest(&issue_manifest_json(
                task_target,
                "closeout-expired",
                "archive",
                consumed_issue.commit_id,
                consumed_issue.work_state_digest,
                raw_authority_ref,
                &digest("authority-expired"),
                Some(expires_at_us),
            )),
        ))
        .expect("issue expiring receipt");
    thread::sleep(Duration::from_millis(1100));
    let head_before_consume = engine
        .branch_head(workspace.initial_branch_id)
        .expect("head before consume");
    let consume_manifest = parse_consume_manifest(&consume_manifest_json(
        task_target,
        &consumed_issue,
        "closeout-consume",
        head_before_consume.head_commit_id,
        head_before_consume.state_digest,
    ));
    engine
        .consume_authorization_receipt(AuthorizationReceiptConsumeOptions::new(
            workspace.initial_branch_id,
            consume_manifest,
        ))
        .expect("consume first receipt");
    drop(engine);

    let readonly = Engine::open_readonly(&path).expect("open readonly");
    let expired_commit = readonly
        .commit(expired_issue.commit_id)
        .expect("expired issue commit");
    let explicit_after_wallclock = readonly
        .closeout_inspect_task(CloseoutInspectOptions::for_task_at_commit(
            expired_issue.commit_id,
            task.task_entity_id,
        ))
        .expect("inspect explicit expired issue commit");
    let explicit_runtime = &explicit_after_wallclock.runtime_summary;
    assert_eq!(
        explicit_runtime.receipt_evaluation_at_us,
        expired_commit.committed_at_us
    );
    assert!(
        explicit_runtime
            .authorization_receipts
            .iter()
            .all(|receipt| receipt.evaluated_at_us == expired_commit.committed_at_us)
    );
    let explicit_archive = explicit_runtime
        .authorization_receipts
        .iter()
        .find(|receipt| receipt.action == "archive")
        .expect("archive receipt at explicit commit");
    assert_eq!(explicit_archive.mechanical_status, "active");
    let stable_receipts =
        serde_json::to_value(&explicit_runtime.authorization_receipts).expect("stable receipts");
    thread::sleep(Duration::from_millis(10));
    let explicit_after_second_wallclock = readonly
        .closeout_inspect_task(CloseoutInspectOptions::for_task_at_commit(
            expired_issue.commit_id,
            task.task_entity_id,
        ))
        .expect("inspect explicit expired issue commit again");
    assert_eq!(
        serde_json::to_value(
            &explicit_after_second_wallclock
                .runtime_summary
                .authorization_receipts
        )
        .expect("stable receipts again"),
        stable_receipts
    );

    let projection = readonly
        .closeout_inspect_task(CloseoutInspectOptions::for_task_on_branch(
            workspace.initial_branch_id,
            task.task_entity_id,
        ))
        .expect("inspect task runtime summary");
    let runtime = &projection.runtime_summary;
    let branch_head = readonly
        .branch_head(workspace.initial_branch_id)
        .expect("branch head");
    let branch_head_commit = readonly
        .commit(branch_head.head_commit_id)
        .expect("branch head commit");
    assert_eq!(
        runtime.receipt_evaluation_at_us,
        branch_head_commit.committed_at_us
    );

    assert_eq!(runtime.counts.sessions_total, 1);
    assert_eq!(runtime.sessions.len(), 1);
    assert_eq!(runtime.sessions[0].session_id, focused_session.session_id);
    assert_eq!(runtime.sessions[0].focus_entity_id, task.task_entity_id);
    assert_eq!(runtime.sessions[0].focus_path_len, 1);
    assert_eq!(runtime.counts.claims_total, 1);
    assert_eq!(runtime.claims.len(), 1);
    assert_eq!(runtime.claims[0].session_id, focused_session.session_id);
    assert_eq!(runtime.claims[0].task_entity_id, task.task_entity_id);
    assert_eq!(runtime.counts.handoffs_total, 1);
    assert_eq!(runtime.handoffs.len(), 1);
    assert_eq!(
        runtime.handoffs[0].handoff_record_entity_id,
        handoff.record_entity_id
    );
    assert_eq!(
        runtime.handoffs[0].session_id,
        Some(focused_session.session_id)
    );
    assert_eq!(runtime.counts.authorization_receipts_total, 2);
    assert_eq!(runtime.authorization_receipts.len(), 2);
    assert!(
        runtime
            .authorization_receipts
            .iter()
            .all(|receipt| receipt.evaluated_at_us == branch_head_commit.committed_at_us)
    );
    let statuses = runtime
        .authorization_receipts
        .iter()
        .map(|receipt| receipt.mechanical_status.as_str())
        .collect::<Vec<_>>();
    assert!(statuses.contains(&"consumed"));
    assert!(statuses.contains(&"expired"));
    assert!(
        runtime
            .authorization_receipts
            .iter()
            .all(|receipt| receipt.authority_ref_kind == "user-session")
    );

    let serialized = serde_json::to_string(&projection).expect("serialize projection");
    assert!(
        !serialized.contains(raw_authority_ref),
        "closeout runtime summary leaked raw authority ref"
    );
    assert!(
        !serialized.contains("authorized"),
        "runtime summary must not claim authorization sufficiency"
    );
}

#[test]
fn receipt_runtime_status_uses_source_commit_time_and_marks_stale_digest() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let task = create_task(
        &mut engine,
        &workspace,
        workspace.genesis_commit_id,
        "Receipt stale task",
    );
    let initial_target = ReceiptTarget {
        entity_id: task.task_entity_id,
        entity_kind: "task",
        entity_version_id: task.task_entity_version_id,
        state_digest: task.task_state_digest,
    };
    let issued = engine
        .issue_authorization_receipt(AuthorizationReceiptIssueOptions::new(
            workspace.initial_branch_id,
            parse_issue_manifest(&issue_manifest_json(
                initial_target,
                "closeout-stale",
                "deploy",
                task.commit_id,
                task.work_state_digest,
                "user:session/stale",
                &digest("authority-stale"),
                None,
            )),
        ))
        .expect("issue stale candidate receipt");
    let criterion = create_acceptance_criterion(
        &mut engine,
        &workspace,
        task.task_entity_id,
        issued.commit_id,
        task.task_entity_version_id,
        "AC-stale",
    );
    drop(engine);

    let readonly = Engine::open_readonly(&path).expect("open readonly");
    let projection = readonly
        .closeout_inspect_task(CloseoutInspectOptions::for_task_on_branch(
            workspace.initial_branch_id,
            task.task_entity_id,
        ))
        .expect("inspect stale branch");
    let head_commit = readonly
        .commit(criterion.commit_id)
        .expect("criterion commit timestamp");

    assert_eq!(
        projection.runtime_summary.receipt_evaluation_at_us,
        head_commit.committed_at_us
    );
    assert_eq!(
        projection
            .runtime_summary
            .authorization_receipts
            .first()
            .expect("stale receipt")
            .evaluated_at_us,
        head_commit.committed_at_us
    );
    assert_eq!(
        projection.runtime_summary.authorization_receipts[0].mechanical_status,
        "stale"
    );
}

#[test]
fn runtime_summary_is_budgeted_and_stable_for_handoffs() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let task = create_task(
        &mut engine,
        &workspace,
        workspace.genesis_commit_id,
        "Budgeted handoff task",
    );
    let first = create_handoff(
        &mut engine,
        &workspace,
        task.commit_id,
        task.task_entity_id,
        None,
    );
    let second = create_handoff(
        &mut engine,
        &workspace,
        first.commit_id,
        task.task_entity_id,
        None,
    );
    create_handoff(
        &mut engine,
        &workspace,
        second.commit_id,
        task.task_entity_id,
        None,
    );
    drop(engine);

    let readonly = Engine::open_readonly(&path).expect("open readonly");
    let projection = readonly
        .closeout_inspect_task(
            CloseoutInspectOptions::for_task_on_branch(
                workspace.initial_branch_id,
                task.task_entity_id,
            )
            .with_budget(2)
            .expect("budget"),
        )
        .expect("inspect budgeted runtime");

    assert_eq!(projection.runtime_summary.counts.handoffs_total, 3);
    assert_eq!(projection.runtime_summary.handoffs.len(), 2);
    assert!(projection.truncated);
    let handoff_omission = projection
        .omitted
        .iter()
        .find(|item| item.category == CloseoutInspectCategory::Handoffs)
        .expect("handoff omission");
    assert_eq!(handoff_omission.count, 1);
    let handoff_ids = projection
        .runtime_summary
        .handoffs
        .iter()
        .map(|item| item.handoff_record_entity_id)
        .collect::<Vec<_>>();
    let mut expected = vec![first.record_entity_id, second.record_entity_id];
    expected.sort();
    assert_eq!(handoff_ids, expected);
}

#[test]
fn runtime_summary_mixed_budget_shares_projection_runtime_and_gap_capacity() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let task = create_task(
        &mut engine,
        &workspace,
        workspace.genesis_commit_id,
        "Mixed budget task",
    );
    let criterion = create_acceptance_criterion(
        &mut engine,
        &workspace,
        task.task_entity_id,
        task.commit_id,
        task.task_entity_version_id,
        "AC-budget",
    );
    let focused_session = engine
        .start_session(
            SessionStartOptions::new(workspace.workspace_id, workspace.initial_branch_id)
                .expect("session options"),
        )
        .expect("start focused session");
    engine
        .set_session_focus(SessionFocusOptions::new(
            focused_session.session_id,
            task.task_entity_id,
        ))
        .expect("set focused session");
    engine
        .claim_task(ClaimTaskOptions::new(
            focused_session.session_id,
            task.task_entity_id,
        ))
        .expect("claim focused task");
    let valid_handoff = create_handoff(
        &mut engine,
        &workspace,
        criterion.commit_id,
        task.task_entity_id,
        Some(focused_session.session_id),
    );
    let invalid_scope = CanonicalValue::object(vec![
        (
            "focus_entity_id".to_owned(),
            CanonicalValue::String(task.task_entity_id.to_string()),
        ),
        (
            "handoff_scope_schema_version".to_owned(),
            CanonicalValue::safe_integer(1).expect("safe integer"),
        ),
        (
            "session_id".to_owned(),
            CanonicalValue::String("not-a-session-id".to_owned()),
        ),
    ])
    .expect("invalid session scope");
    let invalid_handoff = create_handoff_with_scope(
        &mut engine,
        &workspace,
        valid_handoff.commit_id,
        "Invalid session handoff",
        invalid_scope,
    );
    let current_target = ReceiptTarget {
        entity_id: task.task_entity_id,
        entity_kind: "task",
        entity_version_id: criterion.task_entity_version_id,
        state_digest: criterion.task_state_digest,
    };
    engine
        .issue_authorization_receipt(AuthorizationReceiptIssueOptions::new(
            workspace.initial_branch_id,
            parse_issue_manifest(&issue_manifest_json(
                current_target,
                "mixed-budget-receipt",
                "deploy",
                invalid_handoff.commit_id,
                invalid_handoff.work_state_digest,
                "user:session/mixed-budget",
                &digest("authority-mixed-budget"),
                None,
            )),
        ))
        .expect("issue mixed budget receipt");
    drop(engine);

    let readonly = Engine::open_readonly(&path).expect("open readonly");
    let projection = readonly
        .closeout_inspect_task(
            CloseoutInspectOptions::for_task_on_branch(
                workspace.initial_branch_id,
                task.task_entity_id,
            )
            .with_budget(5)
            .expect("budget"),
        )
        .expect("inspect mixed budget");

    assert_eq!(projection.counts.acceptance_criteria_total, 1);
    assert_eq!(projection.acceptance_criteria.len(), 1);
    assert_eq!(projection.runtime_summary.counts.sessions_total, 1);
    assert_eq!(projection.runtime_summary.sessions.len(), 1);
    assert_eq!(projection.runtime_summary.counts.claims_total, 1);
    assert_eq!(projection.runtime_summary.claims.len(), 1);
    assert_eq!(projection.runtime_summary.counts.handoffs_total, 1);
    assert_eq!(projection.runtime_summary.handoffs.len(), 1);
    assert_eq!(
        projection
            .runtime_summary
            .counts
            .authorization_receipts_total,
        1
    );
    assert_eq!(projection.runtime_summary.authorization_receipts.len(), 1);
    assert_eq!(projection.runtime_summary.counts.gaps_total, 1);
    assert!(projection.runtime_summary.gaps.is_empty());
    assert!(projection.truncated);
    let runtime_gap_omission = projection
        .omitted
        .iter()
        .find(|item| item.category == CloseoutInspectCategory::RuntimeGaps)
        .expect("runtime gap omitted by shared budget");
    assert_eq!(runtime_gap_omission.count, 1);
}

#[test]
fn handoff_scope_failures_report_gaps_without_raw_scope_or_items() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let task = create_task(
        &mut engine,
        &workspace,
        workspace.genesis_commit_id,
        "Handoff gap task",
    );
    let valid_minimal_scope = CanonicalValue::object(vec![
        (
            "focus_entity_id".to_owned(),
            CanonicalValue::String(task.task_entity_id.to_string()),
        ),
        (
            "handoff_scope_schema_version".to_owned(),
            CanonicalValue::safe_integer(1).expect("safe integer"),
        ),
    ])
    .expect("valid minimal scope");
    let valid = create_handoff_with_scope(
        &mut engine,
        &workspace,
        task.commit_id,
        "Valid minimal handoff",
        valid_minimal_scope,
    );
    let malformed_v1_scope = CanonicalValue::object(vec![(
        "handoff_scope_schema_version".to_owned(),
        CanonicalValue::safe_integer(1).expect("safe integer"),
    )])
    .expect("malformed v1 scope");
    let malformed = create_handoff_with_scope(
        &mut engine,
        &workspace,
        valid.commit_id,
        "Malformed v1 handoff",
        malformed_v1_scope,
    );
    let future_scope = CanonicalValue::object(vec![
        (
            "focus_entity_id".to_owned(),
            CanonicalValue::String(task.task_entity_id.to_string()),
        ),
        (
            "handoff_scope_schema_version".to_owned(),
            CanonicalValue::safe_integer(2).expect("safe integer"),
        ),
    ])
    .expect("future scope");
    let future = create_handoff_with_scope(
        &mut engine,
        &workspace,
        malformed.commit_id,
        "Future handoff",
        future_scope,
    );
    let invalid_session_scope = CanonicalValue::object(vec![
        (
            "focus_entity_id".to_owned(),
            CanonicalValue::String(task.task_entity_id.to_string()),
        ),
        (
            "handoff_scope_schema_version".to_owned(),
            CanonicalValue::safe_integer(1).expect("safe integer"),
        ),
        (
            "session_id".to_owned(),
            CanonicalValue::String("not-a-session-id".to_owned()),
        ),
    ])
    .expect("invalid session scope");
    let invalid_session = create_handoff_with_scope(
        &mut engine,
        &workspace,
        future.commit_id,
        "Invalid session handoff",
        invalid_session_scope,
    );
    drop(engine);

    let readonly = Engine::open_readonly(&path).expect("open readonly");
    let projection = readonly
        .closeout_inspect_task(CloseoutInspectOptions::for_task_on_branch(
            workspace.initial_branch_id,
            task.task_entity_id,
        ))
        .expect("inspect handoff gaps");

    assert_eq!(projection.runtime_summary.counts.handoffs_total, 1);
    assert_eq!(
        projection.runtime_summary.handoffs[0].handoff_record_entity_id,
        valid.record_entity_id
    );
    assert_eq!(projection.runtime_summary.counts.gaps_total, 3);
    assert!(projection.runtime_summary.gaps.iter().all(|gap| {
        gap.category == CloseoutInspectRuntimeGapCategory::Handoffs
            && (gap
                .message
                .contains(&malformed.record_entity_id.to_string())
                || gap.message.contains(&future.record_entity_id.to_string())
                || gap
                    .message
                    .contains(&invalid_session.record_entity_id.to_string()))
    }));
    let codes = projection
        .runtime_summary
        .gaps
        .iter()
        .map(|gap| gap.code.as_str())
        .collect::<Vec<_>>();
    assert!(codes.contains(&"handoff_focus_entity_id_missing"));
    assert!(codes.contains(&"handoff_scope_schema_unsupported"));
    assert!(codes.contains(&"handoff_session_id_invalid"));
    let serialized = serde_json::to_string(&projection).expect("serialize projection");
    assert!(!serialized.contains("not-a-session-id"));
}

#[test]
fn commit_source_keeps_workstate_provenance_and_marks_runtime_gaps() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let task = create_task(
        &mut engine,
        &workspace,
        workspace.genesis_commit_id,
        "Commit source task",
    );
    let handoff = create_handoff(
        &mut engine,
        &workspace,
        task.commit_id,
        task.task_entity_id,
        None,
    );
    let target = ReceiptTarget {
        entity_id: task.task_entity_id,
        entity_kind: "task",
        entity_version_id: task.task_entity_version_id,
        state_digest: task.task_state_digest,
    };
    let issued = engine
        .issue_authorization_receipt(AuthorizationReceiptIssueOptions::new(
            workspace.initial_branch_id,
            parse_issue_manifest(&issue_manifest_json(
                target,
                "commit-source-receipt",
                "deploy",
                handoff.commit_id,
                handoff.work_state_digest,
                "user:session/commit-source",
                &digest("authority-commit"),
                None,
            )),
        ))
        .expect("issue receipt");
    drop(engine);

    let readonly = Engine::open_readonly(&path).expect("open readonly");
    let before_receipt = readonly
        .closeout_inspect_task(CloseoutInspectOptions::for_task_at_commit(
            handoff.commit_id,
            task.task_entity_id,
        ))
        .expect("inspect before receipt commit");
    assert_eq!(before_receipt.runtime_summary.counts.handoffs_total, 1);
    assert_eq!(
        before_receipt
            .runtime_summary
            .counts
            .authorization_receipts_total,
        0
    );
    assert_eq!(before_receipt.runtime_summary.counts.gaps_total, 2);
    assert!(before_receipt.runtime_summary.gaps.iter().any(|gap| {
        gap.category == CloseoutInspectRuntimeGapCategory::Sessions
            && gap.code == "runtime_sessions_unavailable_for_commit_source"
    }));

    let at_receipt = readonly
        .closeout_inspect_task(CloseoutInspectOptions::for_task_at_commit(
            issued.commit_id,
            task.task_entity_id,
        ))
        .expect("inspect at receipt commit");
    assert_eq!(at_receipt.runtime_summary.counts.handoffs_total, 1);
    assert_eq!(
        at_receipt
            .runtime_summary
            .counts
            .authorization_receipts_total,
        1
    );
    assert_eq!(
        at_receipt.runtime_summary.authorization_receipts[0].mechanical_status,
        "active"
    );
}

#[test]
fn plan_and_goal_runtime_links_are_exact_target_only() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let plan = create_plan(&mut engine, &workspace, workspace.genesis_commit_id);
    let task = create_task(
        &mut engine,
        &workspace,
        plan.commit_id,
        "Child task under plan",
    );
    let contains = engine
        .create_primary_containment(
            PrimaryContainmentCreateOptions::new(
                workspace.initial_branch_id,
                task.commit_id,
                plan.plan_entity_id,
                task.task_entity_id,
            )
            .expect("containment options"),
        )
        .expect("plan contains task");
    let goal = create_goal(&mut engine, &workspace, contains.commit_id);
    let plan_session = engine
        .start_session(
            SessionStartOptions::new(workspace.workspace_id, workspace.initial_branch_id)
                .expect("session options"),
        )
        .expect("start plan session");
    engine
        .set_session_focus(SessionFocusOptions::new(
            plan_session.session_id,
            plan.plan_entity_id,
        ))
        .expect("set plan focus");
    let task_session = engine
        .start_session(
            SessionStartOptions::new(workspace.workspace_id, workspace.initial_branch_id)
                .expect("session options"),
        )
        .expect("start task session");
    engine
        .set_session_focus(SessionFocusOptions::new(
            task_session.session_id,
            task.task_entity_id,
        ))
        .expect("set task focus");
    let goal_handoff = create_handoff(
        &mut engine,
        &workspace,
        goal.commit_id,
        goal.goal_entity_id,
        None,
    );
    drop(engine);

    let readonly = Engine::open_readonly(&path).expect("open readonly");
    let plan_projection = readonly
        .closeout_inspect_plan(CloseoutInspectOptions::for_plan_on_branch(
            workspace.initial_branch_id,
            plan.plan_entity_id,
        ))
        .expect("inspect plan runtime");
    assert_eq!(plan_projection.runtime_summary.counts.sessions_total, 1);
    assert_eq!(
        plan_projection.runtime_summary.sessions[0].session_id,
        plan_session.session_id
    );
    assert_eq!(plan_projection.runtime_summary.counts.claims_total, 0);

    let goal_projection = readonly
        .closeout_inspect_goal(CloseoutInspectOptions::for_goal_on_branch(
            workspace.initial_branch_id,
            goal.goal_entity_id,
        ))
        .expect("inspect goal runtime");
    assert_eq!(goal_projection.runtime_summary.counts.handoffs_total, 1);
    assert_eq!(
        goal_projection.runtime_summary.handoffs[0].handoff_record_entity_id,
        goal_handoff.record_entity_id
    );
    assert_eq!(goal_projection.runtime_summary.counts.sessions_total, 0);
}
