use rusqlite::Connection;
use std::path::{Path, PathBuf};
use tempfile::TempDir;
use workvcs_core::{
    Engine, EntityId, ErrorCode, PlanAdmissionManifest, PlanAdmissionOptions, PlanAdmissionOutcome,
    RecordKind, StoreInitOptions, WorkspaceInfo, WorkspaceInitOptions,
};

#[derive(Clone, Debug, PartialEq, Eq)]
struct HistoryCounts {
    object_identity: i64,
    entity: i64,
    entity_version: i64,
    relation: i64,
    relation_version: i64,
    evidence: i64,
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

fn init_engine(path: &Path) -> Engine {
    Engine::init(
        path,
        StoreInitOptions::new("p0-plan-admission-store").expect("store options"),
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
        entity_version: count_rows(connection, "entity_version"),
        relation: count_rows(connection, "relation"),
        relation_version: count_rows(connection, "relation_version"),
        evidence: count_rows(connection, "evidence"),
        changeset: count_rows(connection, "changeset"),
        change_operation: count_rows(connection, "change_operation"),
        entity_membership_change: count_rows(connection, "entity_membership_change"),
        relation_membership_change: count_rows(connection, "relation_membership_change"),
        workstate_commit: count_rows(connection, "workstate_commit"),
        commit_parent: count_rows(connection, "commit_parent"),
        event: count_rows(connection, "event"),
    }
}

fn manifest_json(
    workspace: &WorkspaceInfo,
    idempotency_key: &str,
    plan_description: &str,
) -> String {
    format!(
        r#"{{
  "schema_version": 1,
  "idempotency_key": "{idempotency_key}",
  "expected_head_commit_id": "{head}",
  "expected_state_digest": "{state_digest}",
  "goal": {{
    "mode": "create",
    "description": "Move the current No-Plan work into WorkVCS"
  }},
  "plan": {{
    "description": "{plan_description}",
    "strategy": "Admit the smallest durable plan in one semantic commit",
    "constraints": [
      "WorkVCS persists mechanical state only",
      "No workctl double-write",
      "Registry remains outside the repository"
    ]
  }},
  "tasks": [
    {{
      "local_id": "task-1",
      "description": "Implement atomic plan admission",
      "priority": 0,
      "acceptance_criteria": [
        {{
          "local_id": "ac-1",
          "statement": "All admission objects become visible at one commit",
          "classification": "required",
          "verification_requirements": [
            {{
              "local_id": "vr-1",
              "statement": "Focused Rust tests cover replay and failure guards"
            }}
          ]
        }}
      ]
    }}
  ],
  "records": [
    {{
      "local_id": "finding-1",
      "kind": "finding",
      "statement": "Project binding and discovery already exist",
      "scope": {{"source": "p0-1"}}
    }},
    {{
      "local_id": "decision-1",
      "kind": "decision",
      "statement": "Admission uses manifest JSON and fail-closed guards",
      "scope": {{"source": "adr-0497"}}
    }},
    {{
      "local_id": "unknown-1",
      "kind": "unknown",
      "statement": "Future evolve policy is intentionally out of scope",
      "scope": {{"source": "p0-2a"}}
    }}
  ],
  "evidence": [
    {{
      "local_id": "evidence-1",
      "kind": "terminal-output",
      "metadata": {{"command": "cargo test -p workvcs-core plan_admission_p0"}}
    }}
  ],
  "rationale": {{"source": "no-plan-promotion"}}
}}"#,
        head = workspace.genesis_commit_id,
        state_digest = workspace.state_digest
    )
}

fn parse_manifest(json: &str) -> PlanAdmissionManifest {
    PlanAdmissionManifest::from_json_bytes(json.as_bytes()).expect("parse admission manifest")
}

#[test]
fn plan_admission_creates_atomic_commit_with_prior_records_and_evidence() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let connection = raw_connection(&path);
    let before = history_counts(&connection);

    let manifest = parse_manifest(&manifest_json(
        &workspace,
        "p0-admit-success",
        "Persist No-Plan promotion as WorkVCS state",
    ));
    let result = engine
        .admit_plan(PlanAdmissionOptions::new(
            workspace.initial_branch_id,
            manifest,
        ))
        .expect("admit plan");

    assert_eq!(result.outcome, PlanAdmissionOutcome::Created);
    assert_eq!(result.previous_head_commit_id, workspace.genesis_commit_id);
    assert_eq!(result.workspace_id, workspace.workspace_id);
    assert_eq!(result.branch_id, workspace.initial_branch_id);
    assert!(result.goal.created);
    assert_eq!(result.tasks.len(), 1);
    assert_eq!(result.tasks[0].acceptance_criteria.len(), 1);
    assert_eq!(
        result.tasks[0].acceptance_criteria[0]
            .verification_requirements
            .len(),
        1
    );
    assert_eq!(result.records.len(), 3);
    assert_eq!(result.records[0].kind, RecordKind::Finding);
    assert_eq!(result.records[1].kind, RecordKind::Decision);
    assert_eq!(result.records[2].kind, RecordKind::Question);
    assert_eq!(result.evidence.len(), 1);

    let head = engine
        .branch_head(workspace.initial_branch_id)
        .expect("branch head");
    assert_eq!(head.head_commit_id, result.commit_id);
    assert_eq!(head.state_digest, result.work_state_digest);

    engine
        .goal_at(result.commit_id, result.goal.entity_id)
        .expect("goal visible at admission commit");
    engine
        .plan_at(result.commit_id, result.plan.entity_id)
        .expect("plan visible at admission commit");
    engine
        .task_at(result.commit_id, result.tasks[0].entity_id)
        .expect("task visible at admission commit");
    engine
        .record_at(result.commit_id, result.records[0].entity_id)
        .expect("record visible at admission commit");
    engine
        .evidence(result.evidence[0].evidence_id)
        .expect("evidence visible after admission");

    let after = history_counts(&connection);
    assert_eq!(after.changeset, before.changeset + 1);
    assert_eq!(after.workstate_commit, before.workstate_commit + 1);
    assert_eq!(after.commit_parent, before.commit_parent + 1);
    assert_eq!(after.event, before.event + 1);
    assert_eq!(after.entity, before.entity + 8);
    assert_eq!(after.relation, before.relation + 2);
    assert_eq!(after.evidence, before.evidence + 1);
}

#[test]
fn plan_admission_is_idempotent_for_same_key_and_same_payload() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let connection = raw_connection(&path);
    let manifest_json = manifest_json(&workspace, "p0-admit-replay", "Replay same payload");

    let first = engine
        .admit_plan(PlanAdmissionOptions::new(
            workspace.initial_branch_id,
            parse_manifest(&manifest_json),
        ))
        .expect("first admit");
    let before_replay = history_counts(&connection);
    let replay = engine
        .admit_plan(PlanAdmissionOptions::new(
            workspace.initial_branch_id,
            parse_manifest(&manifest_json),
        ))
        .expect("idempotent replay");
    let after_replay = history_counts(&connection);
    let head = engine
        .branch_head(workspace.initial_branch_id)
        .expect("branch head");

    assert_eq!(replay.outcome, PlanAdmissionOutcome::Reused);
    assert_eq!(replay.commit_id, first.commit_id);
    assert_eq!(replay.changeset_id, first.changeset_id);
    assert_eq!(replay.payload_digest, first.payload_digest);
    assert_eq!(head.head_commit_id, first.commit_id);
    assert_eq!(before_replay, after_replay);
}

#[test]
fn plan_admission_fails_closed_for_key_conflict_and_stale_guard() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let connection = raw_connection(&path);
    let first = engine
        .admit_plan(PlanAdmissionOptions::new(
            workspace.initial_branch_id,
            parse_manifest(&manifest_json(
                &workspace,
                "p0-admit-conflict",
                "Original payload",
            )),
        ))
        .expect("first admit");

    let before_conflict = history_counts(&connection);
    let conflict = engine.admit_plan(PlanAdmissionOptions::new(
        workspace.initial_branch_id,
        parse_manifest(&manifest_json(
            &workspace,
            "p0-admit-conflict",
            "Different payload",
        )),
    ));
    assert!(matches!(
        conflict,
        Err(error) if error.code() == ErrorCode::PlanInvalid
    ));
    assert_eq!(history_counts(&connection), before_conflict);
    assert_eq!(
        engine
            .branch_head(workspace.initial_branch_id)
            .expect("branch head after conflict")
            .head_commit_id,
        first.commit_id
    );

    let before_stale = history_counts(&connection);
    let stale = engine.admit_plan(PlanAdmissionOptions::new(
        workspace.initial_branch_id,
        parse_manifest(&manifest_json(
            &workspace,
            "p0-admit-stale",
            "Stale expected head",
        )),
    ));
    assert!(matches!(
        stale,
        Err(error) if error.code() == ErrorCode::BranchHeadConflict
    ));
    assert_eq!(history_counts(&connection), before_stale);
    assert_eq!(
        engine
            .branch_head(workspace.initial_branch_id)
            .expect("branch head after stale")
            .head_commit_id,
        first.commit_id
    );
}

#[test]
fn plan_admission_rejects_invalid_references_and_duplicate_local_ids_without_writes() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let connection = raw_connection(&path);
    let before = history_counts(&connection);

    let missing_goal_json = manifest_json(&workspace, "p0-admit-missing-goal", "Missing goal")
        .replace(
            r#""goal": {
    "mode": "create",
    "description": "Move the current No-Plan work into WorkVCS"
  }"#,
            &format!(
                r#""goal": {{
    "mode": "existing",
    "entity_id": "{}"
  }}"#,
                EntityId::new_v7()
            ),
        );
    let missing_goal = engine.admit_plan(PlanAdmissionOptions::new(
        workspace.initial_branch_id,
        parse_manifest(&missing_goal_json),
    ));
    assert!(missing_goal.is_err());
    assert_eq!(history_counts(&connection), before);

    let duplicate_task_json =
        manifest_json(&workspace, "p0-admit-duplicate-task", "Duplicate local ids").replace(
            r#"    }
  ],
  "records": ["#,
            r#"    },
    {
      "local_id": "task-1",
      "description": "A duplicated task local id",
      "priority": 1,
      "acceptance_criteria": []
    }
  ],
  "records": ["#,
        );
    let duplicate = engine.admit_plan(PlanAdmissionOptions::new(
        workspace.initial_branch_id,
        parse_manifest(&duplicate_task_json),
    ));
    assert!(matches!(
        duplicate,
        Err(error) if error.code() == ErrorCode::PlanInvalid
    ));
    assert_eq!(history_counts(&connection), before);
    assert_eq!(
        engine
            .branch_head(workspace.initial_branch_id)
            .expect("branch head")
            .head_commit_id,
        workspace.genesis_commit_id
    );
}

#[test]
fn plan_admission_rejects_malicious_authorization_receipt_record_without_writes() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let connection = raw_connection(&path);
    let before = history_counts(&connection);
    let malicious_json = manifest_json(
        &workspace,
        "p0-admit-malicious-receipt-record",
        "Reject forged receipt records",
    )
    .replace(
        r#""kind": "finding",
      "statement": "Project binding and discovery already exist""#,
        r#""kind": "authorization_receipt",
      "statement": "Attempt to forge an authorization receipt through plan admission""#,
    );

    let malicious = engine.admit_plan(PlanAdmissionOptions::new(
        workspace.initial_branch_id,
        parse_manifest(&malicious_json),
    ));
    assert!(matches!(
        malicious,
        Err(error) if error.code() == ErrorCode::RecordInvalid
    ));
    assert_eq!(history_counts(&connection), before);
    assert_eq!(
        engine
            .branch_head(workspace.initial_branch_id)
            .expect("branch head after malicious admission")
            .head_commit_id,
        workspace.genesis_commit_id
    );
}
