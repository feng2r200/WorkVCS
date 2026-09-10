use rusqlite::{Connection, params};
use std::path::{Path, PathBuf};
use tempfile::TempDir;
use workvcs_core::{
    AuthorizationReceiptConsumeManifest, AuthorizationReceiptConsumeOptions,
    AuthorizationReceiptIssueManifest, AuthorizationReceiptIssueOptions,
    AuthorizationReceiptListOptions, CanonicalValue, Digest, Engine, EntityTransitionOptions,
    ErrorCode, GoalCreateCommit, GoalCreateOptions, GoalTransitionOptions, HistoryQueryOptions,
    RecordStatus, StoreInitOptions, WorkspaceInfo, WorkspaceInitOptions,
};

#[derive(Clone, Debug, PartialEq, Eq)]
struct HistoryCounts {
    object_identity: i64,
    entity: i64,
    relation: i64,
    entity_version: i64,
    relation_version: i64,
    changeset: i64,
    change_operation: i64,
    entity_membership_change: i64,
    relation_membership_change: i64,
    workstate_commit: i64,
    commit_parent: i64,
    event: i64,
}

fn store_path() -> (TempDir, PathBuf) {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let path = tempdir.path().join("workvcs.sqlite");
    (tempdir, path)
}

fn create_workspace(path: &Path) -> (Engine, WorkspaceInfo) {
    let mut engine = Engine::init(
        path,
        StoreInitOptions::new("p0-authorization-receipt-store").expect("store options"),
    )
    .expect("init engine");
    let workspace = engine
        .create_workspace(WorkspaceInitOptions::new("workspace").expect("workspace options"))
        .expect("create workspace");
    (engine, workspace)
}

fn raw_connection(path: &Path) -> Connection {
    let connection = Connection::open(path).expect("raw connection");
    connection
        .pragma_update(None, "foreign_keys", "ON")
        .expect("enable raw foreign keys");
    connection
}

fn count_rows(connection: &Connection, table: &str) -> i64 {
    connection
        .query_row(&format!("SELECT count(*) FROM {table}"), [], |row| {
            row.get(0)
        })
        .expect("count rows")
}

fn history_counts(connection: &Connection) -> HistoryCounts {
    HistoryCounts {
        object_identity: count_rows(connection, "object_identity"),
        entity: count_rows(connection, "entity"),
        relation: count_rows(connection, "relation"),
        entity_version: count_rows(connection, "entity_version"),
        relation_version: count_rows(connection, "relation_version"),
        changeset: count_rows(connection, "changeset"),
        change_operation: count_rows(connection, "change_operation"),
        entity_membership_change: count_rows(connection, "entity_membership_change"),
        relation_membership_change: count_rows(connection, "relation_membership_change"),
        workstate_commit: count_rows(connection, "workstate_commit"),
        commit_parent: count_rows(connection, "commit_parent"),
        event: count_rows(connection, "event"),
    }
}

fn create_goal(
    engine: &mut Engine,
    workspace: &WorkspaceInfo,
    head: workvcs_core::CommitId,
) -> GoalCreateCommit {
    engine
        .create_goal(
            GoalCreateOptions::new(workspace.initial_branch_id, head, "Receipt target goal")
                .expect("goal options"),
        )
        .expect("create goal")
}

fn digest(label: &str) -> String {
    Digest::domain_separated("workvcs.test.authorization-receipt", label.as_bytes()).to_string()
}

fn receipt_manifest_json(
    goal: &GoalCreateCommit,
    key: &str,
    action: &str,
    expected_head_commit_id: String,
    expected_state_digest: String,
    target_entity_kind: &str,
    target_version_id: String,
    target_state_digest: String,
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
  "rationale": {{"why":"mechanical binding test"}}
}}"#,
        target_entity_id = goal.goal_entity_id,
        contract_digest = digest("contract"),
    )
}

fn parse_issue_manifest(json: &str) -> AuthorizationReceiptIssueManifest {
    AuthorizationReceiptIssueManifest::from_json_bytes(json.as_bytes())
        .expect("parse receipt issue manifest")
}

fn issue_manifest_for_goal(goal: &GoalCreateCommit, key: &str) -> String {
    receipt_manifest_json(
        goal,
        key,
        "deploy",
        goal.commit_id.to_string(),
        goal.work_state_digest.to_string(),
        "goal",
        goal.goal_entity_version_id.to_string(),
        goal.goal_state_digest.to_string(),
        "user:session/ref-original",
        &digest("authority"),
        None,
    )
}

fn consume_manifest_json(
    goal: &GoalCreateCommit,
    issued: &workvcs_core::AuthorizationReceiptIssueResult,
    key: &str,
) -> String {
    format!(
        r#"{{
  "schema_version": 1, "idempotency_key": "{key}",
  "expected_head_commit_id": "{head}", "expected_state_digest": "{state}",
  "receipt_record_entity_id": "{receipt}",
  "expected_receipt_entity_version_id": "{receipt_version}",
  "expected_receipt_state_digest": "{receipt_digest}",
  "target": {{"entity_id":"{target}","entity_kind":"goal","expected_version_id":"{target_version}","expected_state_digest":"{target_digest}"}},
  "action": "deploy", "contract_digest": "{contract}", "contract_digest_domain": "work-governance/action-contract/v1",
  "rationale": {{"why":"consume binding"}}
}}"#,
        head = issued.commit_id,
        state = issued.work_state_digest,
        receipt = issued.receipt.entity_id,
        receipt_version = issued.receipt.entity_version_id,
        receipt_digest = issued.receipt.state_digest,
        target = goal.goal_entity_id,
        target_version = goal.goal_entity_version_id,
        target_digest = goal.goal_state_digest,
        contract = digest("contract")
    )
}

fn parse_consume_manifest(json: &str) -> AuthorizationReceiptConsumeManifest {
    AuthorizationReceiptConsumeManifest::from_json_bytes(json.as_bytes())
        .expect("parse receipt consume manifest")
}

fn assert_unchanged(
    path: &Path,
    before: &HistoryCounts,
    branch_id: workvcs_core::BranchId,
    head: workvcs_core::CommitId,
) {
    let connection = raw_connection(path);
    assert_eq!(history_counts(&connection), *before);
    let head_bytes: Vec<u8> = connection
        .query_row(
            "SELECT head_commit_id FROM branch WHERE branch_id = ?1",
            params![&branch_id.raw_bytes()[..]],
            |row| row.get(0),
        )
        .expect("branch head");
    assert_eq!(
        workvcs_core::CommitId::from_bytes(head_bytes.try_into().expect("commit bytes"))
            .expect("commit id"),
        head
    );
}

#[test]
fn public_debug_surfaces_redact_authority_reference() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let goal = create_goal(&mut engine, &workspace, workspace.genesis_commit_id);
    let marker = "P2_RAW_AUTHORITY_REF_MARKER_token_password_credential_secret_DO_NOT_LEAK_7f0c";
    let manifest = parse_issue_manifest(&receipt_manifest_json(
        &goal,
        "receipt-debug-redaction",
        "deploy",
        goal.commit_id.to_string(),
        goal.work_state_digest.to_string(),
        "goal",
        goal.goal_entity_version_id.to_string(),
        goal.goal_state_digest.to_string(),
        marker,
        &digest("authority"),
        None,
    ));
    let list_options = AuthorizationReceiptListOptions::new(goal.commit_id)
        .with_target_entity_id(goal.goal_entity_id)
        .with_action("deploy")
        .expect("list action")
        .with_status(RecordStatus::Active);
    let surfaces = vec![
        (
            "authority_ref_manifest",
            format!("{:?}", &manifest.authority_ref),
        ),
        ("target_manifest", format!("{:?}", &manifest.target)),
        ("issue_manifest", format!("{manifest:?}")),
        (
            "issue_options",
            format!(
                "{:?}",
                AuthorizationReceiptIssueOptions::new(workspace.initial_branch_id, manifest)
            ),
        ),
        ("list_options", format!("{list_options:?}")),
    ];

    for (name, rendered) in surfaces {
        assert!(
            !rendered.contains(marker),
            "{name} Debug leaked raw authority_ref.ref: {rendered}"
        );
    }
}

#[test]
fn issue_show_list_replay_doctor_and_idempotent_reuse() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let goal = create_goal(&mut engine, &workspace, workspace.genesis_commit_id);
    let manifest_json = issue_manifest_for_goal(&goal, "receipt-reuse");
    let manifest = parse_issue_manifest(&manifest_json);
    let connection = raw_connection(&path);
    let before = history_counts(&connection);

    let issued = engine
        .issue_authorization_receipt(AuthorizationReceiptIssueOptions::new(
            workspace.initial_branch_id,
            manifest.clone(),
        ))
        .expect("issue receipt");
    assert_eq!(issued.receipt.status, RecordStatus::Active);
    assert_eq!(issued.receipt.binding.action, "deploy");
    assert_eq!(issued.receipt.binding.target_entity_id, goal.goal_entity_id);
    assert_eq!(issued.previous_head_commit_id, goal.commit_id);
    assert_ne!(issued.commit_id, goal.commit_id);

    let after = history_counts(&raw_connection(&path));
    assert_eq!(after.object_identity, before.object_identity + 2);
    assert_eq!(after.entity, before.entity + 1);
    assert_eq!(after.relation, before.relation + 1);
    assert_eq!(after.entity_version, before.entity_version + 1);
    assert_eq!(after.relation_version, before.relation_version + 1);
    assert_eq!(after.changeset, before.changeset + 1);
    assert_eq!(after.change_operation, before.change_operation + 2);
    assert_eq!(
        after.entity_membership_change,
        before.entity_membership_change + 1
    );
    assert_eq!(
        after.relation_membership_change,
        before.relation_membership_change + 1
    );
    assert_eq!(after.workstate_commit, before.workstate_commit + 1);
    assert_eq!(after.commit_parent, before.commit_parent + 1);
    assert_eq!(after.event, before.event + 1);

    let shown = engine
        .authorization_receipt_at(issued.commit_id, issued.receipt.entity_id)
        .expect("show receipt");
    assert_eq!(shown.binding.authority_ref_kind, "user-session");
    assert_eq!(
        shown.binding.authority_digest.to_string(),
        digest("authority")
    );

    let listed = engine
        .authorization_receipts_at(
            AuthorizationReceiptListOptions::new(issued.commit_id)
                .with_status(RecordStatus::Active)
                .with_target_entity_id(goal.goal_entity_id)
                .with_action("deploy")
                .expect("action filter"),
        )
        .expect("list receipts");
    assert_eq!(listed.receipts.len(), 1);
    assert_eq!(
        listed.receipts[0].receipt_entity_id,
        issued.receipt.entity_id
    );

    let history = engine
        .history(HistoryQueryOptions::from_commit(issued.commit_id))
        .expect("history");
    assert_eq!(
        history.entries[0].operation_type,
        "authorization_receipt.issue"
    );
    let replayed = engine.state_at(issued.commit_id).expect("replay receipt");
    assert_eq!(replayed.state_digest, issued.work_state_digest);
    let integrity = engine.validate_integrity().expect("doctor integrity");
    assert_eq!(integrity.checked_commits, 3);

    drop(engine);
    let mut reopened = Engine::open(&path).expect("reopen engine");
    let before_reuse = history_counts(&raw_connection(&path));
    let branch_head = reopened
        .branch_head(workspace.initial_branch_id)
        .expect("branch head")
        .head_commit_id;
    let reused = reopened
        .issue_authorization_receipt(AuthorizationReceiptIssueOptions::new(
            workspace.initial_branch_id,
            manifest,
        ))
        .expect("reuse receipt");
    assert!(matches!(
        reused.outcome,
        workvcs_core::AuthorizationReceiptOutcome::Reused
    ));
    assert_eq!(reused.commit_id, issued.commit_id);
    assert_unchanged(
        &path,
        &before_reuse,
        workspace.initial_branch_id,
        branch_head,
    );
}

#[test]
fn issue_rejects_conflicts_and_invalid_bindings_without_partial_writes() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let goal = create_goal(&mut engine, &workspace, workspace.genesis_commit_id);
    let manifest_json = issue_manifest_for_goal(&goal, "receipt-conflict");
    let issued = engine
        .issue_authorization_receipt(AuthorizationReceiptIssueOptions::new(
            workspace.initial_branch_id,
            parse_issue_manifest(&manifest_json),
        ))
        .expect("issue receipt");
    let before = history_counts(&raw_connection(&path));
    let head = engine
        .branch_head(workspace.initial_branch_id)
        .expect("branch head")
        .head_commit_id;

    let conflict_json = issue_manifest_for_goal(&goal, "receipt-conflict")
        .replace(r#""action": "deploy""#, r#""action": "publish""#);
    let conflict = engine.issue_authorization_receipt(AuthorizationReceiptIssueOptions::new(
        workspace.initial_branch_id,
        parse_issue_manifest(&conflict_json),
    ));
    assert!(conflict.is_err());
    assert_unchanged(&path, &before, workspace.initial_branch_id, head);

    let expired_json = receipt_manifest_json(
        &goal,
        "receipt-expired",
        "deploy",
        issued.commit_id.to_string(),
        issued.work_state_digest.to_string(),
        "goal",
        goal.goal_entity_version_id.to_string(),
        goal.goal_state_digest.to_string(),
        "user:session/ref-original",
        &digest("authority"),
        Some(1),
    );
    let expired = engine.issue_authorization_receipt(AuthorizationReceiptIssueOptions::new(
        workspace.initial_branch_id,
        parse_issue_manifest(&expired_json),
    ));
    assert!(expired.is_err());
    assert_unchanged(&path, &before, workspace.initial_branch_id, head);

    let stale_head_json = receipt_manifest_json(
        &goal,
        "receipt-stale-head",
        "deploy",
        goal.commit_id.to_string(),
        goal.work_state_digest.to_string(),
        "goal",
        goal.goal_entity_version_id.to_string(),
        goal.goal_state_digest.to_string(),
        "user:session/ref-original",
        &digest("authority"),
        None,
    );
    let stale_head = engine.issue_authorization_receipt(AuthorizationReceiptIssueOptions::new(
        workspace.initial_branch_id,
        parse_issue_manifest(&stale_head_json),
    ));
    assert!(stale_head.is_err());
    assert_unchanged(&path, &before, workspace.initial_branch_id, head);

    let wrong_kind_json = receipt_manifest_json(
        &goal,
        "receipt-wrong-kind",
        "deploy",
        issued.commit_id.to_string(),
        issued.work_state_digest.to_string(),
        "task",
        goal.goal_entity_version_id.to_string(),
        goal.goal_state_digest.to_string(),
        "user:session/ref-original",
        &digest("authority"),
        None,
    );
    let wrong_kind = engine.issue_authorization_receipt(AuthorizationReceiptIssueOptions::new(
        workspace.initial_branch_id,
        parse_issue_manifest(&wrong_kind_json),
    ));
    assert!(wrong_kind.is_err());
    assert_unchanged(&path, &before, workspace.initial_branch_id, head);

    let abandoned = engine
        .transition_goal(
            GoalTransitionOptions::abandon(
                workspace.initial_branch_id,
                issued.commit_id,
                goal.goal_entity_id,
                goal.goal_entity_version_id,
                "target drift",
            )
            .expect("abandon options"),
        )
        .expect("abandon goal");
    let drift_before = history_counts(&raw_connection(&path));
    let drift_json = receipt_manifest_json(
        &goal,
        "receipt-version-drift",
        "deploy",
        abandoned.commit_id.to_string(),
        abandoned.work_state_digest.to_string(),
        "goal",
        goal.goal_entity_version_id.to_string(),
        goal.goal_state_digest.to_string(),
        "user:session/ref-original",
        &digest("authority"),
        None,
    );
    let drift = engine.issue_authorization_receipt(AuthorizationReceiptIssueOptions::new(
        workspace.initial_branch_id,
        parse_issue_manifest(&drift_json),
    ));
    assert!(drift.is_err());
    assert_unchanged(
        &path,
        &drift_before,
        workspace.initial_branch_id,
        abandoned.commit_id,
    );
}

#[test]
fn consume_is_single_use_idempotent_and_rejects_scope_drift_without_writes() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let goal = create_goal(&mut engine, &workspace, workspace.genesis_commit_id);
    let issued = engine
        .issue_authorization_receipt(AuthorizationReceiptIssueOptions::new(
            workspace.initial_branch_id,
            parse_issue_manifest(&issue_manifest_for_goal(&goal, "consume-issue")),
        ))
        .expect("issue");
    let manifest_json = consume_manifest_json(&goal, &issued, "consume-key");
    let before = history_counts(&raw_connection(&path));
    let consumed = engine
        .consume_authorization_receipt(AuthorizationReceiptConsumeOptions::new(
            workspace.initial_branch_id,
            parse_consume_manifest(&manifest_json),
        ))
        .expect("consume");
    assert_eq!(consumed.receipt.status, RecordStatus::Consumed);
    let after = history_counts(&raw_connection(&path));
    assert_eq!(after.entity_version, before.entity_version + 1);
    assert_eq!(after.changeset, before.changeset + 1);
    assert_eq!(after.change_operation, before.change_operation + 1);
    assert_eq!(after.workstate_commit, before.workstate_commit + 1);
    assert_eq!(after.event, before.event + 1);
    assert_eq!(
        engine
            .authorization_receipt_at(consumed.commit_id, issued.receipt.entity_id)
            .expect("show")
            .status,
        RecordStatus::Consumed
    );
    drop(engine);
    let mut reopened = Engine::open(&path).expect("reopen");
    let reused = reopened
        .consume_authorization_receipt(AuthorizationReceiptConsumeOptions::new(
            workspace.initial_branch_id,
            parse_consume_manifest(&manifest_json),
        ))
        .expect("reuse consumed");
    assert!(matches!(
        reused.outcome,
        workvcs_core::AuthorizationReceiptConsumeOutcome::Reused
    ));
    assert_eq!(reused.commit_id, consumed.commit_id);
    let unchanged = history_counts(&raw_connection(&path));
    let new_key = manifest_json.replace("consume-key", "consume-other-key");
    assert!(
        reopened
            .consume_authorization_receipt(AuthorizationReceiptConsumeOptions::new(
                workspace.initial_branch_id,
                parse_consume_manifest(&new_key),
            ))
            .is_err()
    );
    assert_unchanged(
        &path,
        &unchanged,
        workspace.initial_branch_id,
        consumed.commit_id,
    );
    let wrong_action = manifest_json.replace(r#""action": "deploy""#, r#""action": "publish""#);
    assert!(
        reopened
            .consume_authorization_receipt(AuthorizationReceiptConsumeOptions::new(
                workspace.initial_branch_id,
                parse_consume_manifest(&wrong_action),
            ))
            .is_err()
    );
    assert_unchanged(
        &path,
        &unchanged,
        workspace.initial_branch_id,
        consumed.commit_id,
    );
}

#[test]
fn consume_ignores_orphan_changeset_with_matching_idempotency_key() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let goal = create_goal(&mut engine, &workspace, workspace.genesis_commit_id);
    let issued = engine
        .issue_authorization_receipt(AuthorizationReceiptIssueOptions::new(
            workspace.initial_branch_id,
            parse_issue_manifest(&issue_manifest_for_goal(&goal, "orphan-consume-issue")),
        ))
        .expect("issue");
    let manifest_json = consume_manifest_json(&goal, &issued, "orphan-consume-key");
    let orphan_changeset_id = workvcs_core::ChangeSetId::new_v7();
    raw_connection(&path)
        .execute(
            "INSERT INTO changeset(
                changeset_id, workspace_id, operation_type, operation_schema_version,
                operation_payload_json, rationale_json, origin_session_id, created_at_us
             ) VALUES (?1, ?2, 'authorization_receipt.consume', 1, ?3, '{}', NULL, 1)",
            params![
                &orphan_changeset_id.raw_bytes()[..],
                &workspace.workspace_id.raw_bytes()[..],
                r#"{"idempotency_key":"orphan-consume-key"}"#,
            ],
        )
        .expect("insert orphan changeset");

    // Existing integrity semantics permit an unattached, syntactically valid changeset.
    engine
        .validate_integrity()
        .expect("orphan changeset is auditable");
    let before = history_counts(&raw_connection(&path));
    let consumed = engine
        .consume_authorization_receipt(AuthorizationReceiptConsumeOptions::new(
            workspace.initial_branch_id,
            parse_consume_manifest(&manifest_json),
        ))
        .expect("orphan must not trigger replay or conflict");
    assert!(matches!(
        consumed.outcome,
        workvcs_core::AuthorizationReceiptConsumeOutcome::Consumed
    ));
    assert_eq!(consumed.receipt.status, RecordStatus::Consumed);
    let after = history_counts(&raw_connection(&path));
    assert_eq!(after.changeset, before.changeset + 1);
    assert_eq!(after.workstate_commit, before.workstate_commit + 1);
    assert_eq!(after.event, before.event + 1);
}

#[test]
fn manifest_rejects_secret_extra_field_and_empty_authority_without_partial_writes() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let goal = create_goal(&mut engine, &workspace, workspace.genesis_commit_id);
    let before = history_counts(&raw_connection(&path));

    let authority_digest = digest("authority");
    let secret_json = issue_manifest_for_goal(&goal, "receipt-secret-extra").replace(
        &format!(r#""authority_digest": "{authority_digest}""#),
        &format!(r#""token": "secret-token", "authority_digest": "{authority_digest}""#),
    );
    assert!(AuthorizationReceiptIssueManifest::from_json_bytes(secret_json.as_bytes()).is_err());
    assert_unchanged(&path, &before, workspace.initial_branch_id, goal.commit_id);

    let empty_ref_json = receipt_manifest_json(
        &goal,
        "receipt-empty-ref",
        "deploy",
        goal.commit_id.to_string(),
        goal.work_state_digest.to_string(),
        "goal",
        goal.goal_entity_version_id.to_string(),
        goal.goal_state_digest.to_string(),
        "",
        &digest("authority"),
        None,
    );
    let empty_ref = engine.issue_authorization_receipt(AuthorizationReceiptIssueOptions::new(
        workspace.initial_branch_id,
        parse_issue_manifest(&empty_ref_json),
    ));
    assert!(empty_ref.is_err());
    assert_unchanged(&path, &before, workspace.initial_branch_id, goal.commit_id);
}

#[test]
fn generic_entity_transition_cannot_forge_authorization_receipt_record() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let forged_state = CanonicalValue::object(vec![
        (
            "kind".to_owned(),
            CanonicalValue::String("authorization_receipt".to_owned()),
        ),
        (
            "scope".to_owned(),
            CanonicalValue::object(Vec::new()).expect("scope"),
        ),
        (
            "statement".to_owned(),
            CanonicalValue::String("forged receipt".to_owned()),
        ),
        (
            "status".to_owned(),
            CanonicalValue::String("consumed".to_owned()),
        ),
    ])
    .expect("forged state");
    let result = engine.commit_entity_transition(
        EntityTransitionOptions::create(
            workspace.initial_branch_id,
            workspace.genesis_commit_id,
            "record",
            forged_state,
        )
        .expect("entity transition options"),
    );
    let error = result.expect_err("generic record forge must fail");
    assert_eq!(error.code(), ErrorCode::EntityTransitionInvalid);
}
