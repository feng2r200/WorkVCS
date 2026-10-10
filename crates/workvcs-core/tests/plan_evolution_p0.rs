use rusqlite::Connection;
use std::path::{Path, PathBuf};
use tempfile::TempDir;
use workvcs_core::control_plane::{
    CaptureAdmissionOutcome, CaptureIntent, CaptureJournal, CapturePayloadKind, ResolutionContext,
    ResolutionMode, ResolutionResult, UtcTimestamp, admitted_legacy_plan_evolution_options,
};
use workvcs_core::{
    CaptureId, ClaimListOptions, ClaimTaskOptions, Engine, ErrorCode, GoalCreateOptions,
    HistoryQueryOptions, PlanAdmissionManifest, PlanAdmissionOptions, PlanCreateOptions,
    PlanEvolutionManifest, PlanEvolutionOptions, PlanEvolutionOutcome, PlanStatus,
    PlanTaskDispositionAuditOptions, PlanTransitionOptions, PrimaryContainmentCreateOptions,
    RelationId, RelationVersionId, RunnableTasksOptions, SessionStartOptions, StoreInitOptions,
    TaskCreateOptions, TaskSchedulingRelationCreateOptions, WhyQueryOptions, WhyQueryTarget,
    WhyRelationKind, WorkspaceInfo, WorkspaceInitOptions,
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
        StoreInitOptions::new("p0-plan-evolution-store").expect("store options"),
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

fn admit_manifest_json(workspace: &WorkspaceInfo, key: &str) -> String {
    format!(
        r#"{{
  "schema_version": 1,
  "idempotency_key": "{key}",
  "expected_head_commit_id": "{head}",
  "expected_state_digest": "{state_digest}",
  "goal": {{"mode": "create", "description": "Cutover goal"}},
  "plan": {{
    "description": "Original Plan",
    "strategy": "Original Strategy",
    "constraints": ["Keep scoped"]
  }},
  "tasks": [],
  "records": [],
  "evidence": [],
  "rationale": {{"source": "admit-test"}}
}}"#,
        head = workspace.genesis_commit_id,
        state_digest = workspace.state_digest
    )
}

fn admit_manifest_with_prior_context_json(workspace: &WorkspaceInfo, key: &str) -> String {
    format!(
        r#"{{
  "schema_version": 1,
  "idempotency_key": "{key}",
  "expected_head_commit_id": "{head}",
  "expected_state_digest": "{state_digest}",
  "goal": {{"mode": "create", "description": "Cutover goal"}},
  "plan": {{
    "description": "Original Plan",
    "strategy": "Original Strategy",
    "constraints": ["Keep scoped"]
  }},
  "tasks": [
    {{
      "local_id": "task-prior",
      "description": "Prior task remains on old Plan",
      "acceptance_criteria": [
        {{
          "local_id": "ac-prior",
          "statement": "Prior AC remains under prior task",
          "verification_requirements": [
            {{"local_id": "vr-prior", "statement": "Prior VR remains under prior AC"}}
          ]
        }}
      ]
    }}
  ],
  "records": [
    {{
      "local_id": "record-prior",
      "kind": "finding",
      "statement": "Prior record is not copied by supersede",
      "scope": {{"source": "prior-context"}}
    }}
  ],
  "evidence": [
    {{
      "local_id": "evidence-prior",
      "kind": "terminal-output",
      "metadata": {{"command": "prior command"}}
    }}
  ],
  "rationale": {{"source": "admit-prior-context-test"}}
}}"#,
        head = workspace.genesis_commit_id,
        state_digest = workspace.state_digest
    )
}

fn parse_admit(json: &str) -> PlanAdmissionManifest {
    PlanAdmissionManifest::from_json_bytes(json.as_bytes()).expect("parse admission manifest")
}

fn parse_evolve(json: &str) -> PlanEvolutionManifest {
    PlanEvolutionManifest::from_json_bytes(json.as_bytes()).expect("parse evolution manifest")
}

fn admit_base_plan(
    engine: &mut Engine,
    workspace: &WorkspaceInfo,
    key: &str,
) -> workvcs_core::PlanAdmissionResult {
    engine
        .admit_plan(PlanAdmissionOptions::new(
            workspace.initial_branch_id,
            parse_admit(&admit_manifest_json(workspace, key)),
        ))
        .expect("admit base plan")
}

fn evolve_manifest_json(
    admission: &workvcs_core::PlanAdmissionResult,
    key: &str,
    description: Option<&str>,
) -> String {
    let plan_update = description
        .map(|description| {
            format!(
                r#""plan": {{"description": "{description}"}},
"#
            )
        })
        .unwrap_or_else(|| {
            r#""plan": {},
"#
            .to_owned()
        });
    format!(
        r#"{{
  "mode": "in_place",
  "schema_version": 1,
  "idempotency_key": "{key}",
  "expected_head_commit_id": "{head}",
  "expected_state_digest": "{state_digest}",
  "target_plan_entity_id": "{plan_id}",
  "expected_plan_entity_version_id": "{plan_version}",
  "expected_plan_state_digest": "{plan_digest}",
  {plan_update}  "tasks": [
    {{
      "local_id": "task-2",
      "description": "Carry prior context forward",
      "priority": 2,
      "acceptance_criteria": [
        {{
          "local_id": "ac-2",
          "statement": "Evolution writes one semantic commit",
          "classification": "required",
          "verification_requirements": [
            {{
              "local_id": "vr-2",
              "statement": "Replay and doctor accept plan.evolve"
            }}
          ]
        }}
      ]
    }}
  ],
  "records": [
    {{
      "local_id": "finding-2",
      "kind": "finding",
      "statement": "Existing discovery remains useful",
      "scope": {{"source": "prior-context"}}
    }}
  ],
  "evidence": [
    {{
      "local_id": "evidence-2",
      "kind": "terminal-output",
      "metadata": {{"command": "cargo test -p workvcs-core plan_evolution_p0"}}
    }}
  ],
  "rationale": {{"source": "evolve-test"}}
}}"#,
        head = admission.commit_id,
        state_digest = admission.work_state_digest,
        plan_id = admission.plan.entity_id,
        plan_version = admission.plan.entity_version_id,
        plan_digest = admission.plan.state_digest
    )
}

fn evolve_manifest_with_plan_update_json(
    head: impl std::fmt::Display,
    state_digest: impl std::fmt::Display,
    plan_id: impl std::fmt::Display,
    plan_version: impl std::fmt::Display,
    plan_digest: impl std::fmt::Display,
    key: &str,
    plan_update_json: &str,
) -> String {
    format!(
        r#"{{
  "mode": "in_place",
  "schema_version": 1,
  "idempotency_key": "{key}",
  "expected_head_commit_id": "{head}",
  "expected_state_digest": "{state_digest}",
  "target_plan_entity_id": "{plan_id}",
  "expected_plan_entity_version_id": "{plan_version}",
  "expected_plan_state_digest": "{plan_digest}",
  "plan": {plan_update_json},
  "tasks": [],
  "records": [],
  "evidence": [],
  "rationale": {{"source": "presence-boundary"}}
}}"#
    )
}

fn goal_plan_relation(
    engine: &Engine,
    commit_id: impl Into<workvcs_core::CommitId>,
    goal_entity_id: impl Into<workvcs_core::EntityId>,
    plan_entity_id: impl Into<workvcs_core::EntityId>,
) -> workvcs_core::PrimaryContainmentSnapshot {
    let commit_id = commit_id.into();
    let goal_entity_id = goal_entity_id.into();
    let plan_entity_id = plan_entity_id.into();
    engine
        .primary_containment_relations_at(commit_id)
        .expect("primary containment")
        .into_iter()
        .find(|relation| {
            relation.parent_entity_id == goal_entity_id
                && relation.child_entity_id == plan_entity_id
        })
        .expect("goal plan relation")
}

fn supersede_manifest_json(
    admission: &workvcs_core::PlanAdmissionResult,
    key: &str,
    replacement_strategy: &str,
    constraints_json: &str,
) -> String {
    let dispositions = admission
        .tasks
        .iter()
        .map(|task| {
            format!(
                r#"{{"task_entity_id":"{task_id}","expected_task_entity_version_id":"{task_version}","expected_task_state_digest":"{task_digest}","action":"retain_original","return_condition":"preserve historical task for explicit reassessment","rationale":{{"assessment":"retain without implicit migration"}}}}"#,
                task_id = task.entity_id,
                task_version = task.entity_version_id,
                task_digest = task.state_digest,
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    format!(
        r#"{{
  "mode": "supersede",
  "schema_version": 2,
  "idempotency_key": "{key}",
  "expected_head_commit_id": "{head}",
  "expected_state_digest": "{state_digest}",
  "target_plan_entity_id": "{plan_id}",
  "expected_plan_entity_version_id": "{plan_version}",
  "expected_plan_state_digest": "{plan_digest}",
  "expected_parent": {{
    "kind": "goal",
    "entity_id": "{goal_id}",
    "expected_entity_version_id": "{goal_version}",
    "containment_relation_id": "__RELATION_ID__",
    "containment_relation_version_id": "__RELATION_VERSION__"
  }},
  "plan": {{
    "description": "Replacement Plan",
    "strategy": "{replacement_strategy}",
    "constraints": {constraints_json}
  }},
  "tasks": [
    {{
      "local_id": "task-supersede",
      "description": "Execute replacement plan",
      "acceptance_criteria": [
        {{
          "local_id": "ac-supersede",
          "statement": "Supersede writes lineage",
          "verification_requirements": [
            {{"local_id": "vr-supersede", "statement": "why reports plan_supersedes"}}
          ]
        }}
      ]
    }}
  ],
  "task_dispositions": [{dispositions}],
  "nested_plan_continuations": [],
  "records": [
    {{
      "local_id": "finding-supersede",
      "kind": "finding",
      "statement": "Supersede preserves old Plan lineage",
      "scope": {{"source": "supersede-test"}}
    }}
  ],
  "evidence": [
    {{
      "local_id": "evidence-supersede",
      "kind": "terminal-output",
      "metadata": {{"command": "cargo test -p workvcs-core --test plan_evolution_p0"}}
    }}
  ],
  "rationale": {{"kind": "strategy-change", "why": "replacement strategy differs"}}
}}"#,
        head = admission.commit_id,
        state_digest = admission.work_state_digest,
        plan_id = admission.plan.entity_id,
        plan_version = admission.plan.entity_version_id,
        plan_digest = admission.plan.state_digest,
        goal_id = admission.goal.entity_id,
        goal_version = admission.goal.entity_version_id,
    )
}

fn supersede_manifest_for_admission(
    engine: &Engine,
    admission: &workvcs_core::PlanAdmissionResult,
    key: &str,
    replacement_strategy: &str,
    constraints_json: &str,
) -> String {
    let relation = goal_plan_relation(
        engine,
        admission.commit_id,
        admission.goal.entity_id,
        admission.plan.entity_id,
    );
    supersede_manifest_json(admission, key, replacement_strategy, constraints_json)
        .replace("__RELATION_ID__", &relation.relation_id.to_string())
        .replace(
            "__RELATION_VERSION__",
            &relation.relation_version_id.to_string(),
        )
}

fn legacy_supersede_manifest_for_admission(
    engine: &Engine,
    admission: &workvcs_core::PlanAdmissionResult,
    key: &str,
) -> String {
    let relation = goal_plan_relation(
        engine,
        admission.commit_id,
        admission.goal.entity_id,
        admission.plan.entity_id,
    );
    format!(
        r#"{{
  "mode": "supersede",
  "schema_version": 1,
  "idempotency_key": "{key}",
  "expected_head_commit_id": "{head}",
  "expected_state_digest": "{state_digest}",
  "target_plan_entity_id": "{plan_id}",
  "expected_plan_entity_version_id": "{plan_version}",
  "expected_plan_state_digest": "{plan_digest}",
  "expected_goal_entity_id": "{goal_id}",
  "expected_goal_entity_version_id": "{goal_version}",
  "expected_goal_plan_relation_id": "{relation_id}",
  "expected_goal_plan_relation_version_id": "{relation_version}",
  "plan": {{
    "description": "Legacy replacement",
    "strategy": "Legacy replacement strategy",
    "constraints": {{"mode":"carry_all"}}
  }},
  "tasks": [],
  "records": [],
  "evidence": [],
  "rationale": {{"kind":"admitted-legacy-recovery"}}
}}"#,
        head = admission.commit_id,
        state_digest = admission.work_state_digest,
        plan_id = admission.plan.entity_id,
        plan_version = admission.plan.entity_version_id,
        plan_digest = admission.plan.state_digest,
        goal_id = admission.goal.entity_id,
        goal_version = admission.goal.entity_version_id,
        relation_id = relation.relation_id,
        relation_version = relation.relation_version_id,
    )
}

fn disposition_fixture_manifest_json(workspace: &WorkspaceInfo, key: &str) -> String {
    format!(
        r#"{{
  "schema_version": 1,
  "idempotency_key": "{key}",
  "expected_head_commit_id": "{head}",
  "expected_state_digest": "{state_digest}",
  "goal": {{"mode": "create", "description": "Repair carrier goal"}},
  "plan": {{
    "description": "Retired plan needing task assessment",
    "strategy": "Preserve each task according to evidence",
    "constraints": []
  }},
  "tasks": [
    {{"local_id": "umbrella", "description": "Umbrella remains active"}},
    {{"local_id": "mcp", "description": "MCP remains blocked"}},
    {{"local_id": "review", "description": "Review remains pending"}},
    {{"local_id": "deferred", "description": "Deferred task stays historical"}}
  ],
  "records": [],
  "evidence": [],
  "rationale": {{"source": "task-disposition-fixture"}}
}}"#,
        head = workspace.genesis_commit_id,
        state_digest = workspace.state_digest,
    )
}

fn repair_manifest_json(
    key: &str,
    head: impl std::fmt::Display,
    work_state_digest: impl std::fmt::Display,
    retired_plan: &workvcs_core::PlanSnapshot,
    carrier_goal: Option<&workvcs_core::GoalSnapshot>,
    tasks: &[workvcs_core::PlanAdmissionTaskResult],
    requested_dispositions: &[(&str, &str, &str)],
) -> String {
    let task = |local_id: &str| {
        tasks
            .iter()
            .find(|task| task.local_id == local_id)
            .unwrap_or_else(|| panic!("missing fixture task {local_id}"))
    };
    let item = |local_id: &str, action: &str, return_condition: &str| {
        let task = task(local_id);
        format!(
            r#"{{
      "task_entity_id": "{task_id}",
      "expected_task_entity_version_id": "{task_version}",
      "expected_task_state_digest": "{task_digest}",
      "action": "{action}",
      "return_condition": "{return_condition}",
      "rationale": {{"assessment": "{local_id}"}}
    }}"#,
            task_id = task.entity_id,
            task_version = task.entity_version_id,
            task_digest = task.state_digest,
        )
    };
    let dispositions = requested_dispositions
        .iter()
        .map(|(local_id, action, return_condition)| item(local_id, action, return_condition))
        .collect::<Vec<_>>()
        .join(",\n");
    let carrier = carrier_goal
        .map(|goal| {
            format!(
                r#""carrier": {{
    "kind": "goal",
    "entity_id": "{goal_id}",
    "expected_entity_version_id": "{goal_version}",
    "expected_state_digest": "{goal_digest}"
  }},"#,
                goal_id = goal.goal_entity_id,
                goal_version = goal.goal_entity_version_id,
                goal_digest = goal.state_digest,
            )
        })
        .unwrap_or_default();
    format!(
        r#"{{
  "mode": "task_disposition",
  "schema_version": 2,
  "idempotency_key": "{key}",
  "expected_head_commit_id": "{head}",
  "expected_state_digest": "{work_state_digest}",
  "target_plan_entity_id": "{plan_id}",
  "expected_plan_entity_version_id": "{plan_version}",
  "expected_plan_state_digest": "{plan_digest}",
  {carrier}
  "tasks": [],
  "task_dispositions": [
{dispositions}
  ],
  "rationale": {{"kind": "retired-plan-repair", "source": "reviewed-assessment"}}
}}"#,
        plan_id = retired_plan.plan_entity_id,
        plan_version = retired_plan.plan_entity_version_id,
        plan_digest = retired_plan.state_digest,
    )
}

fn retired_fixture_with_dependency(
    path: &Path,
    key: &str,
) -> (
    Engine,
    WorkspaceInfo,
    workvcs_core::PlanAdmissionResult,
    workvcs_core::PlanSnapshot,
    workvcs_core::GoalSnapshot,
    workvcs_core::CommitId,
    workvcs_core::Digest,
) {
    let (mut engine, workspace) = create_workspace(path);
    let admission = engine
        .admit_plan(PlanAdmissionOptions::new(
            workspace.initial_branch_id,
            parse_admit(&disposition_fixture_manifest_json(&workspace, key)),
        ))
        .expect("admit dependency fixture");
    let umbrella = admission
        .tasks
        .iter()
        .find(|task| task.local_id == "umbrella")
        .expect("umbrella task");
    let review = admission
        .tasks
        .iter()
        .find(|task| task.local_id == "review")
        .expect("review task");
    let abandoned = engine
        .transition_plan(
            PlanTransitionOptions::abandon(
                workspace.initial_branch_id,
                admission.commit_id,
                admission.plan.entity_id,
                admission.plan.entity_version_id,
                "retire fixture plan",
            )
            .expect("abandon fixture options"),
        )
        .expect("abandon fixture plan");
    let external = engine
        .create_task(
            TaskCreateOptions::new(
                workspace.initial_branch_id,
                abandoned.commit_id,
                "Existing current dependent",
            )
            .expect("external task options"),
        )
        .expect("create external dependent");
    let internal_dependency = engine
        .create_task_scheduling_relation(
            TaskSchedulingRelationCreateOptions::depends_on(
                workspace.initial_branch_id,
                external.commit_id,
                review.entity_id,
                umbrella.entity_id,
            )
            .expect("internal dependency options"),
        )
        .expect("create internal dependency");
    let external_dependency = engine
        .create_task_scheduling_relation(
            TaskSchedulingRelationCreateOptions::depends_on(
                workspace.initial_branch_id,
                internal_dependency.commit_id,
                external.task_entity_id,
                umbrella.entity_id,
            )
            .expect("external dependency options"),
        )
        .expect("create external dependency");
    let retired = engine
        .plan_at(external_dependency.commit_id, admission.plan.entity_id)
        .expect("retired fixture plan");
    let carrier = engine
        .goal_at(external_dependency.commit_id, admission.goal.entity_id)
        .expect("fixture carrier goal");
    (
        engine,
        workspace,
        admission,
        retired,
        carrier,
        external_dependency.commit_id,
        external_dependency.work_state_digest,
    )
}

#[test]
fn plan_evolution_in_place_updates_plan_and_appends_prior_context_atomically() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let connection = raw_connection(&path);
    let admission = admit_base_plan(&mut engine, &workspace, "p0-evolve-success-admit");
    let before = history_counts(&connection);
    let manifest = parse_evolve(&evolve_manifest_json(
        &admission,
        "p0-evolve-success",
        Some("Evolved Plan"),
    ));

    let result = engine
        .evolve_plan(PlanEvolutionOptions::new(
            workspace.initial_branch_id,
            manifest,
        ))
        .expect("evolve plan");

    assert_eq!(result.mode, "in_place");
    assert_eq!(result.outcome, PlanEvolutionOutcome::Created);
    assert_eq!(result.previous_head_commit_id, admission.commit_id);
    assert_eq!(result.goal_entity_id, Some(admission.goal.entity_id));
    assert_eq!(result.plan.entity_id, admission.plan.entity_id);
    assert_eq!(
        result.plan.previous_entity_version_id,
        admission.plan.entity_version_id
    );
    assert_eq!(
        result.plan.previous_state_digest,
        admission.plan.state_digest
    );
    assert_eq!(result.tasks.len(), 1);
    assert_eq!(result.tasks[0].acceptance_criteria.len(), 1);
    assert_eq!(
        result.tasks[0].acceptance_criteria[0]
            .verification_requirements
            .len(),
        1
    );
    assert_eq!(result.records.len(), 1);
    assert_eq!(result.evidence.len(), 1);

    let head = engine
        .branch_head(workspace.initial_branch_id)
        .expect("branch head");
    assert_eq!(head.head_commit_id, result.commit_id);
    assert_eq!(head.state_digest, result.work_state_digest);

    let old_plan = engine
        .plan_at(admission.commit_id, admission.plan.entity_id)
        .expect("old plan");
    assert_eq!(old_plan.state.description, "Original Plan");
    let new_plan = engine
        .plan_at(result.commit_id, result.plan.entity_id)
        .expect("new plan");
    assert_eq!(new_plan.state.description, "Evolved Plan");
    assert_eq!(new_plan.state.strategy, "Original Strategy");
    assert_eq!(new_plan.state.constraints, vec!["Keep scoped".to_owned()]);
    engine
        .task_at(result.commit_id, result.tasks[0].entity_id)
        .expect("task visible");
    engine
        .record_at(result.commit_id, result.records[0].entity_id)
        .expect("record visible");
    engine
        .evidence(result.evidence[0].evidence_id)
        .expect("evidence visible");
    let containment = engine
        .primary_containment_relations_at(result.commit_id)
        .expect("containment");
    assert!(containment.iter().any(|relation| relation.parent_entity_id
        == admission.goal.entity_id
        && relation.child_entity_id == admission.plan.entity_id));
    assert!(containment.iter().any(|relation| relation.parent_entity_id
        == admission.plan.entity_id
        && relation.child_entity_id == result.tasks[0].entity_id));

    let after = history_counts(&connection);
    assert_eq!(after.changeset, before.changeset + 1);
    assert_eq!(after.workstate_commit, before.workstate_commit + 1);
    assert_eq!(after.commit_parent, before.commit_parent + 1);
    assert_eq!(after.event, before.event + 1);
    assert_eq!(after.entity_version, before.entity_version + 5);
    assert_eq!(after.entity, before.entity + 4);
    assert_eq!(after.relation, before.relation + 1);
    assert_eq!(after.evidence, before.evidence + 1);
    assert_eq!(after.change_operation, before.change_operation + 6);

    let integrity = engine.validate_integrity().expect("integrity");
    assert_eq!(integrity.checked_changesets, 3);
}

#[test]
fn plan_evolution_supersede_replaces_plan_with_lineage_atomically() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let connection = raw_connection(&path);
    let admission = admit_base_plan(&mut engine, &workspace, "p0-supersede-success-admit");
    let before = history_counts(&connection);
    let manifest = parse_evolve(&supersede_manifest_for_admission(
        &engine,
        &admission,
        "p0-supersede-success",
        "Replacement Strategy",
        r#"{"mode":"carry_all"}"#,
    ));

    let result = engine
        .evolve_plan(PlanEvolutionOptions::new(
            workspace.initial_branch_id,
            manifest,
        ))
        .expect("supersede plan");

    assert_eq!(result.mode, "supersede");
    assert_eq!(result.outcome, PlanEvolutionOutcome::Created);
    assert_eq!(result.previous_head_commit_id, admission.commit_id);
    assert_eq!(result.goal_entity_id, Some(admission.goal.entity_id));
    assert_eq!(result.plan.entity_id, admission.plan.entity_id);
    assert_eq!(
        result.plan.previous_entity_version_id,
        admission.plan.entity_version_id
    );
    let new_plan = result.new_plan.as_ref().expect("new replacement plan");
    let goal_contains = result
        .goal_contains_relation
        .as_ref()
        .expect("goal contains relation");
    let supersedes = result
        .supersedes_relation
        .as_ref()
        .expect("supersedes relation");
    assert_eq!(goal_contains.source_entity_id, admission.goal.entity_id);
    assert_eq!(goal_contains.target_entity_id, new_plan.entity_id);
    assert_eq!(supersedes.source_entity_id, new_plan.entity_id);
    assert_eq!(supersedes.target_entity_id, admission.plan.entity_id);
    assert_eq!(result.tasks.len(), 1);
    assert_eq!(result.records.len(), 1);
    assert_eq!(result.evidence.len(), 1);

    let head = engine
        .branch_head(workspace.initial_branch_id)
        .expect("branch head");
    assert_eq!(head.head_commit_id, result.commit_id);
    assert_eq!(head.state_digest, result.work_state_digest);
    let old_plan = engine
        .plan_at(result.commit_id, admission.plan.entity_id)
        .expect("old plan superseded");
    assert_eq!(old_plan.state.status, PlanStatus::Superseded);
    assert_eq!(old_plan.state.strategy, "Original Strategy");
    let replacement = engine
        .plan_at(result.commit_id, new_plan.entity_id)
        .expect("replacement plan");
    assert_eq!(replacement.state.status, PlanStatus::Active);
    assert_eq!(replacement.state.strategy, "Replacement Strategy");
    assert_eq!(
        replacement.state.constraints,
        vec!["Keep scoped".to_owned()]
    );

    let containment = engine
        .primary_containment_relations_at(result.commit_id)
        .expect("containment");
    assert!(containment.iter().any(|relation| relation.parent_entity_id
        == admission.goal.entity_id
        && relation.child_entity_id == admission.plan.entity_id));
    assert!(containment.iter().any(|relation| relation.parent_entity_id
        == admission.goal.entity_id
        && relation.child_entity_id == new_plan.entity_id));
    assert!(
        containment
            .iter()
            .any(|relation| relation.parent_entity_id == new_plan.entity_id
                && relation.child_entity_id == result.tasks[0].entity_id)
    );

    let why_new = engine
        .why(WhyQueryOptions::new(
            WhyQueryTarget::commit(result.commit_id),
            new_plan.entity_id,
        ))
        .expect("why replacement plan");
    assert!(
        why_new
            .relation_edges
            .iter()
            .any(|edge| edge.relation_id == supersedes.relation_id)
    );
    let why_old = engine
        .why(WhyQueryOptions::new(
            WhyQueryTarget::commit(result.commit_id),
            admission.plan.entity_id,
        ))
        .expect("why prior plan");
    assert!(
        why_old
            .relation_edges
            .iter()
            .any(|edge| edge.relation_id == supersedes.relation_id)
    );
    let history = engine
        .history(HistoryQueryOptions::from_branch(
            workspace.initial_branch_id,
        ))
        .expect("history");
    assert_eq!(history.entries[0].commit_id, result.commit_id);
    assert_eq!(history.entries[0].operation_type, "plan.evolve");

    let after = history_counts(&connection);
    assert_eq!(after.changeset, before.changeset + 1);
    assert_eq!(after.workstate_commit, before.workstate_commit + 1);
    assert_eq!(after.commit_parent, before.commit_parent + 1);
    assert_eq!(after.event, before.event + 1);
    assert_eq!(after.entity_version, before.entity_version + 6);
    assert_eq!(after.entity, before.entity + 5);
    assert_eq!(after.relation, before.relation + 3);
    assert_eq!(after.relation_version, before.relation_version + 3);
    assert_eq!(after.evidence, before.evidence + 1);
    assert_eq!(after.change_operation, before.change_operation + 9);
    let integrity = engine.validate_integrity().expect("integrity");
    assert_eq!(integrity.checked_changesets, 3);
}

#[test]
fn legacy_supersede_requires_admitted_recovery_but_exact_committed_reuse_is_read_only() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let admission = admit_base_plan(&mut engine, &workspace, "p0-legacy-gate-admit");
    let manifest_json =
        legacy_supersede_manifest_for_admission(&engine, &admission, "p0-legacy-gate");
    let manifest = parse_evolve(&manifest_json);
    let connection = raw_connection(&path);
    let before_fresh = history_counts(&connection);

    let fresh = engine.evolve_plan(PlanEvolutionOptions::new(
        workspace.initial_branch_id,
        manifest.clone(),
    ));
    assert!(matches!(
        fresh,
        Err(error) if error.code() == ErrorCode::PlanInvalid
    ));
    assert_eq!(history_counts(&connection), before_fresh);
    assert_eq!(
        engine
            .branch_head(workspace.initial_branch_id)
            .expect("head after rejected legacy authoring")
            .head_commit_id,
        admission.commit_id
    );

    let resolution: ResolutionResult = serde_json::from_value(serde_json::json!({
        "status": "unresolved",
        "primary_project_ref": null,
        "primary_basis": null,
        "related_project_refs": [],
        "unmapped_locators": [],
        "diagnostics": []
    }))
    .expect("legacy recovery resolution");
    let intent = CaptureIntent::new(
        CaptureId::new_v7(),
        "p0-legacy-gate",
        UtcTimestamp::parse("2026-10-10T00:00:00Z").expect("legacy recovery time"),
        "recover an immutable admitted schema 1 supersede intent",
        CapturePayloadKind::PlanEvolveV1,
        serde_json::from_str(&manifest_json).expect("legacy recovery payload"),
        ResolutionContext::new(ResolutionMode::DurableWrite),
        resolution,
        None,
    )
    .expect("legacy recovery intent");
    let journal =
        CaptureJournal::for_standalone_root(_tempdir.path().join("capture-journal").join("v1"))
            .expect("legacy recovery journal");
    let rejected = journal.admit(&intent);
    assert!(matches!(
        rejected,
        Err(error) if error.code() == ErrorCode::PlanInvalid
    ));
    let rejected_json = journal.admit_json(
        &intent
            .canonical_json_bytes()
            .expect("canonical legacy intent"),
    );
    assert!(matches!(
        rejected_json,
        Err(error) if error.code() == ErrorCode::PlanInvalid
    ));
    assert!(!journal.intent_path(intent.capture_id()).exists());

    std::fs::create_dir_all(journal.root().join("intents"))
        .expect("create historical journal intent directory");
    std::fs::write(
        journal.intent_path(intent.capture_id()),
        intent.stored_json_bytes().expect("historical intent bytes"),
    )
    .expect("pre-store immutable historical legacy intent bytes");
    let reuse = journal
        .admit(&intent)
        .expect("reuse immutable historical legacy intent");
    assert_eq!(reuse.outcome(), CaptureAdmissionOutcome::Reused);
    let authority = journal
        .inspect_authority(intent.capture_id())
        .expect("inspect admitted legacy recovery authority");
    let recovered = engine
        .evolve_plan(
            admitted_legacy_plan_evolution_options(
                &authority,
                workspace.initial_branch_id,
                manifest.clone(),
            )
            .expect("admitted legacy recovery options"),
        )
        .expect("execute already-admitted legacy recovery");
    assert_eq!(recovered.outcome, PlanEvolutionOutcome::Created);
    engine
        .validate_integrity()
        .expect("legacy recovery replay integrity");
    let before_reuse = history_counts(&connection);
    let reused = engine
        .evolve_plan(PlanEvolutionOptions::new(
            workspace.initial_branch_id,
            manifest,
        ))
        .expect("exact committed legacy reuse");
    assert_eq!(reused.outcome, PlanEvolutionOutcome::Reused);
    assert_eq!(reused.commit_id, recovered.commit_id);
    assert_eq!(history_counts(&connection), before_reuse);
}

#[test]
fn why_explains_task_supersedes_lineage_from_schema2_disposition() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let admission = engine
        .admit_plan(PlanAdmissionOptions::new(
            workspace.initial_branch_id,
            parse_admit(&admit_manifest_with_prior_context_json(
                &workspace,
                "p0-task-supersedes-admit",
            )),
        ))
        .expect("admit task supersedes fixture");
    let manifest_json = supersede_manifest_for_admission(
        &engine,
        &admission,
        "p0-task-supersedes",
        "Replacement strategy with explicit task mapping",
        r#"{"mode":"carry_all"}"#,
    )
    .replace(
        r#""action":"retain_original","return_condition""#,
        r#""action":"supersede","replacement_task_local_id":"task-supersede","return_condition""#,
    );
    let result = engine
        .evolve_plan(PlanEvolutionOptions::new(
            workspace.initial_branch_id,
            parse_evolve(&manifest_json),
        ))
        .expect("supersede task with exact mapping");
    let disposition = result
        .task_dispositions
        .iter()
        .find(|item| item.task_entity_id == admission.tasks[0].entity_id)
        .expect("task supersede disposition");
    let replacement_task = disposition
        .replacement_task_entity_id
        .expect("replacement task identity");
    let relation = result
        .disposition_relations
        .iter()
        .find(|relation| relation.target_entity_id == admission.tasks[0].entity_id)
        .expect("task supersedes relation");
    assert_eq!(relation.source_entity_id, replacement_task);

    for subject in [admission.tasks[0].entity_id, replacement_task] {
        let why = engine
            .why(WhyQueryOptions::new(
                WhyQueryTarget::commit(result.commit_id),
                subject,
            ))
            .expect("why task supersedes");
        assert!(why.relation_edges.iter().any(|edge| {
            edge.relation_id == relation.relation_id
                && edge.relation_kind == WhyRelationKind::TaskSupersedes
        }));
    }
}

#[test]
fn plan_evolution_supersede_does_not_migrate_prior_plan_children() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let prior_admission = engine
        .admit_plan(PlanAdmissionOptions::new(
            workspace.initial_branch_id,
            parse_admit(&admit_manifest_with_prior_context_json(
                &workspace,
                "p0-supersede-prior-context-admit",
            )),
        ))
        .expect("admit prior context plan");
    let prior_task = prior_admission.tasks.first().expect("prior task");
    let prior_ac = prior_task
        .acceptance_criteria
        .first()
        .expect("prior acceptance criterion");
    let prior_vr = prior_ac
        .verification_requirements
        .first()
        .expect("prior verification requirement");
    let prior_record = prior_admission.records.first().expect("prior record");
    let prior_evidence = prior_admission.evidence.first().expect("prior evidence");

    let result = engine
        .evolve_plan(PlanEvolutionOptions::new(
            workspace.initial_branch_id,
            parse_evolve(&supersede_manifest_for_admission(
                &engine,
                &prior_admission,
                "p0-supersede-prior-context",
                "Replacement Strategy",
                r#"{"mode":"carry_all"}"#,
            )),
        ))
        .expect("supersede prior context plan");
    let new_plan = result.new_plan.as_ref().expect("replacement plan");

    let containment = engine
        .primary_containment_relations_at(result.commit_id)
        .expect("containment");
    let old_plan_children = containment
        .iter()
        .filter(|relation| relation.parent_entity_id == prior_admission.plan.entity_id)
        .map(|relation| relation.child_entity_id)
        .collect::<Vec<_>>();
    assert_eq!(old_plan_children, vec![prior_task.entity_id]);
    let new_plan_children = containment
        .iter()
        .filter(|relation| relation.parent_entity_id == new_plan.entity_id)
        .map(|relation| relation.child_entity_id)
        .collect::<Vec<_>>();
    assert_eq!(new_plan_children, vec![result.tasks[0].entity_id]);

    engine
        .task_at(result.commit_id, prior_task.entity_id)
        .expect("prior task still visible");
    engine
        .acceptance_criterion_at(result.commit_id, prior_ac.entity_id)
        .expect("prior acceptance criterion still visible");
    engine
        .verification_requirement_at(result.commit_id, prior_vr.entity_id)
        .expect("prior verification requirement still visible");
    engine
        .record_at(result.commit_id, prior_record.entity_id)
        .expect("prior record still visible");
    engine
        .evidence(prior_evidence.evidence_id)
        .expect("prior evidence still visible");
    assert_ne!(result.tasks[0].entity_id, prior_task.entity_id);
    assert_ne!(
        result.tasks[0].acceptance_criteria[0].entity_id,
        prior_ac.entity_id
    );
    assert_ne!(
        result.tasks[0].acceptance_criteria[0].verification_requirements[0].entity_id,
        prior_vr.entity_id
    );
    assert_ne!(result.records[0].entity_id, prior_record.entity_id);
    assert_ne!(result.evidence[0].evidence_id, prior_evidence.evidence_id);
}

#[test]
fn plan_evolution_supersede_replace_constraints_and_idempotency() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let admission = admit_base_plan(&mut engine, &workspace, "p0-supersede-replay-admit");
    let manifest_json = supersede_manifest_for_admission(
        &engine,
        &admission,
        "p0-supersede-replay",
        "Replacement Strategy",
        r#"{"mode":"replace","values":["New constraint"]}"#,
    );

    let first = engine
        .evolve_plan(PlanEvolutionOptions::new(
            workspace.initial_branch_id,
            parse_evolve(&manifest_json),
        ))
        .expect("first supersede");
    let replacement = engine
        .plan_at(
            first.commit_id,
            first.new_plan.as_ref().expect("new plan").entity_id,
        )
        .expect("replacement plan");
    assert_eq!(
        replacement.state.constraints,
        vec!["New constraint".to_owned()]
    );
    drop(engine);

    let mut reopened = Engine::open(&path).expect("reopen engine");
    let connection = raw_connection(&path);
    let before_replay = history_counts(&connection);
    let replay = reopened
        .evolve_plan(PlanEvolutionOptions::new(
            workspace.initial_branch_id,
            parse_evolve(&manifest_json),
        ))
        .expect("replay supersede");
    assert_eq!(replay.outcome, PlanEvolutionOutcome::Reused);
    assert_eq!(replay.commit_id, first.commit_id);
    assert_eq!(replay.new_plan, first.new_plan);
    assert_eq!(history_counts(&connection), before_replay);

    let conflict_manifest = supersede_manifest_for_admission(
        &reopened,
        &admission,
        "p0-supersede-replay",
        "Different Replacement Strategy",
        r#"{"mode":"replace","values":["New constraint"]}"#,
    );
    let before_conflict = history_counts(&connection);
    let conflict = reopened.evolve_plan(PlanEvolutionOptions::new(
        workspace.initial_branch_id,
        parse_evolve(&conflict_manifest),
    ));
    assert!(matches!(
        conflict,
        Err(error) if error.code() == ErrorCode::PlanInvalid
    ));
    assert_eq!(history_counts(&connection), before_conflict);
    assert_eq!(
        reopened
            .branch_head(workspace.initial_branch_id)
            .expect("branch head after conflict")
            .head_commit_id,
        first.commit_id
    );
}

#[test]
fn plan_evolution_supersede_fails_closed_for_strategy_relation_status_and_stale() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let connection = raw_connection(&path);
    let admission = admit_base_plan(&mut engine, &workspace, "p0-supersede-failure-admit");

    let same_strategy = supersede_manifest_for_admission(
        &engine,
        &admission,
        "p0-supersede-same-strategy",
        "Original Strategy",
        r#"{"mode":"carry_all"}"#,
    );
    let before_same = history_counts(&connection);
    let same_result = engine.evolve_plan(PlanEvolutionOptions::new(
        workspace.initial_branch_id,
        parse_evolve(&same_strategy),
    ));
    assert!(matches!(
        same_result,
        Err(error) if error.code() == ErrorCode::PlanInvalid
    ));
    assert_eq!(history_counts(&connection), before_same);

    let expected_relation = goal_plan_relation(
        &engine,
        admission.commit_id,
        admission.goal.entity_id,
        admission.plan.entity_id,
    );
    let wrong_relation = supersede_manifest_for_admission(
        &engine,
        &admission,
        "p0-supersede-wrong-relation",
        "Replacement Strategy",
        r#"{"mode":"carry_all"}"#,
    )
    .replace(
        &format!(
            r#""containment_relation_id": "{}""#,
            expected_relation.relation_id
        ),
        &format!(r#""containment_relation_id": "{}""#, RelationId::new_v7()),
    );
    let before_wrong_relation = history_counts(&connection);
    let wrong_relation_result = engine.evolve_plan(PlanEvolutionOptions::new(
        workspace.initial_branch_id,
        parse_evolve(&wrong_relation),
    ));
    assert!(matches!(
        wrong_relation_result,
        Err(error) if error.code() == ErrorCode::RelationInvalid
    ));
    assert_eq!(history_counts(&connection), before_wrong_relation);

    let bad_digest = supersede_manifest_for_admission(
        &engine,
        &admission,
        "p0-supersede-bad-digest",
        "Replacement Strategy",
        r#"{"mode":"carry_all"}"#,
    )
    .replace(
        &format!(
            r#""expected_plan_state_digest": "{}""#,
            admission.plan.state_digest
        ),
        r#""expected_plan_state_digest": "0000000000000000000000000000000000000000000000000000000000000000""#,
    );
    let before_bad_digest = history_counts(&connection);
    let bad_digest_result = engine.evolve_plan(PlanEvolutionOptions::new(
        workspace.initial_branch_id,
        parse_evolve(&bad_digest),
    ));
    assert!(matches!(
        bad_digest_result,
        Err(error) if error.code() == ErrorCode::DigestInvalid
    ));
    assert_eq!(history_counts(&connection), before_bad_digest);

    let first = engine
        .evolve_plan(PlanEvolutionOptions::new(
            workspace.initial_branch_id,
            parse_evolve(&supersede_manifest_for_admission(
                &engine,
                &admission,
                "p0-supersede-first",
                "Replacement Strategy",
                r#"{"mode":"carry_all"}"#,
            )),
        ))
        .expect("first supersede");
    let before_stale = history_counts(&connection);
    let stale = engine.evolve_plan(PlanEvolutionOptions::new(
        workspace.initial_branch_id,
        parse_evolve(&supersede_manifest_for_admission(
            &engine,
            &admission,
            "p0-supersede-stale",
            "Another Replacement Strategy",
            r#"{"mode":"carry_all"}"#,
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

    let (mut status_engine, status_workspace) =
        create_workspace(&path.with_extension("supersede-status.sqlite"));
    let status_admission = admit_base_plan(
        &mut status_engine,
        &status_workspace,
        "p0-supersede-status-admit",
    );
    let completed = status_engine
        .transition_plan(
            PlanTransitionOptions::complete(
                status_workspace.initial_branch_id,
                status_admission.commit_id,
                status_admission.plan.entity_id,
                status_admission.plan.entity_version_id,
            )
            .expect("complete options"),
        )
        .expect("complete plan");
    let status_connection = raw_connection(&path.with_extension("supersede-status.sqlite"));
    let status_before = history_counts(&status_connection);
    let status_manifest = supersede_manifest_for_admission(
        &status_engine,
        &workvcs_core::PlanAdmissionResult {
            commit_id: completed.commit_id,
            work_state_digest: completed.work_state_digest,
            plan: workvcs_core::PlanAdmissionEntityResult {
                entity_id: completed.plan_entity_id,
                entity_version_id: completed.plan_entity_version_id,
                state_digest: completed.plan_state_digest,
            },
            ..status_admission
        },
        "p0-supersede-status",
        "Replacement Strategy",
        r#"{"mode":"carry_all"}"#,
    );
    let status_result = status_engine.evolve_plan(PlanEvolutionOptions::new(
        status_workspace.initial_branch_id,
        parse_evolve(&status_manifest),
    ));
    assert!(matches!(
        status_result,
        Err(error) if error.code() == ErrorCode::PlanInvalid
    ));
    assert_eq!(history_counts(&status_connection), status_before);

    let (mut orphan_engine, orphan_workspace) =
        create_workspace(&path.with_extension("supersede-orphan.sqlite"));
    let orphan_goal = orphan_engine
        .create_goal(
            GoalCreateOptions::new(
                orphan_workspace.initial_branch_id,
                orphan_workspace.genesis_commit_id,
                "Orphan goal",
            )
            .expect("orphan goal options"),
        )
        .expect("create orphan goal");
    let orphan_plan = orphan_engine
        .create_plan(
            PlanCreateOptions::new(
                orphan_workspace.initial_branch_id,
                orphan_goal.commit_id,
                "Orphan Plan",
                "Original Strategy",
            )
            .expect("orphan plan options"),
        )
        .expect("create orphan plan");
    let orphan_connection = raw_connection(&path.with_extension("supersede-orphan.sqlite"));
    let orphan_before = history_counts(&orphan_connection);
    let orphan_manifest = format!(
        r#"{{
  "mode": "supersede",
  "schema_version": 2,
  "idempotency_key": "p0-supersede-orphan",
  "expected_head_commit_id": "{head}",
  "expected_state_digest": "{state_digest}",
  "target_plan_entity_id": "{plan_id}",
  "expected_plan_entity_version_id": "{plan_version}",
  "expected_plan_state_digest": "{plan_digest}",
  "expected_parent": {{
    "kind": "goal",
    "entity_id": "{goal_id}",
    "expected_entity_version_id": "{goal_version}",
    "containment_relation_id": "{relation_id}",
    "containment_relation_version_id": "{relation_version}"
  }},
  "plan": {{
    "description": "Replacement Plan",
    "strategy": "Replacement Strategy",
    "constraints": {{"mode":"carry_all"}}
  }},
  "tasks": [],
  "task_dispositions": [],
  "nested_plan_continuations": [],
  "records": [],
  "evidence": [],
  "rationale": {{"kind": "strategy-change"}}
}}"#,
        head = orphan_plan.commit_id,
        state_digest = orphan_plan.work_state_digest,
        plan_id = orphan_plan.plan_entity_id,
        plan_version = orphan_plan.plan_entity_version_id,
        plan_digest = orphan_plan.plan_state_digest,
        goal_id = orphan_goal.goal_entity_id,
        goal_version = orphan_goal.goal_entity_version_id,
        relation_id = RelationId::new_v7(),
        relation_version = RelationVersionId::new_v7(),
    );
    let orphan_result = orphan_engine.evolve_plan(PlanEvolutionOptions::new(
        orphan_workspace.initial_branch_id,
        parse_evolve(&orphan_manifest),
    ));
    assert!(matches!(
        orphan_result,
        Err(error) if error.code() == ErrorCode::RelationInvalid
    ));
    assert_eq!(history_counts(&orphan_connection), orphan_before);

    let (mut multi_goal_engine, multi_goal_workspace) =
        create_workspace(&path.with_extension("supersede-multi-goal.sqlite"));
    let multi_goal_admission = admit_base_plan(
        &mut multi_goal_engine,
        &multi_goal_workspace,
        "p0-supersede-multi-goal-admit",
    );
    let second_goal = multi_goal_engine
        .create_goal(
            GoalCreateOptions::new(
                multi_goal_workspace.initial_branch_id,
                multi_goal_admission.commit_id,
                "Second goal",
            )
            .expect("second goal options"),
        )
        .expect("create second goal");
    let multi_goal_connection = raw_connection(&path.with_extension("supersede-multi-goal.sqlite"));
    let multi_goal_before = history_counts(&multi_goal_connection);
    let duplicate_parent = multi_goal_engine.create_primary_containment(
        PrimaryContainmentCreateOptions::new(
            multi_goal_workspace.initial_branch_id,
            second_goal.commit_id,
            second_goal.goal_entity_id,
            multi_goal_admission.plan.entity_id,
        )
        .expect("duplicate parent options"),
    );
    assert!(matches!(
        duplicate_parent,
        Err(error) if error.code() == ErrorCode::RelationInvalid
    ));
    assert_eq!(history_counts(&multi_goal_connection), multi_goal_before);
}

#[test]
fn schema2_nested_plan_continuation_covers_subtree_and_missing_frontier_is_zero_write() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let admission = admit_base_plan(&mut engine, &workspace, "p0-nested-continuation-admit");
    let nested_plan = engine
        .create_plan(
            PlanCreateOptions::new(
                workspace.initial_branch_id,
                admission.commit_id,
                "Nested active Plan",
                "Nested strategy",
            )
            .expect("nested Plan options"),
        )
        .expect("create nested Plan");
    let root_contains_nested = engine
        .create_primary_containment(
            PrimaryContainmentCreateOptions::new(
                workspace.initial_branch_id,
                nested_plan.commit_id,
                admission.plan.entity_id,
                nested_plan.plan_entity_id,
            )
            .expect("root contains nested options"),
        )
        .expect("root contains nested Plan");
    let nested_task = engine
        .create_task(
            TaskCreateOptions::new(
                workspace.initial_branch_id,
                root_contains_nested.commit_id,
                "Nested task preserved by the explicit frontier",
            )
            .expect("nested Task options"),
        )
        .expect("create nested Task");
    let nested_contains_task = engine
        .create_primary_containment(
            PrimaryContainmentCreateOptions::new(
                workspace.initial_branch_id,
                nested_task.commit_id,
                nested_plan.plan_entity_id,
                nested_task.task_entity_id,
            )
            .expect("nested contains Task options"),
        )
        .expect("nested contains Task");
    let updated_admission = workvcs_core::PlanAdmissionResult {
        commit_id: nested_contains_task.commit_id,
        work_state_digest: nested_contains_task.work_state_digest,
        ..admission.clone()
    };
    let missing_frontier = supersede_manifest_for_admission(
        &engine,
        &updated_admission,
        "p0-nested-continuation-missing",
        "Replacement strategy without a frontier",
        r#"{"mode":"carry_all"}"#,
    );
    let connection = raw_connection(&path);
    let before_missing = history_counts(&connection);
    let missing = engine.evolve_plan(PlanEvolutionOptions::new(
        workspace.initial_branch_id,
        parse_evolve(&missing_frontier),
    ));
    assert!(matches!(
        missing,
        Err(error) if error.code() == ErrorCode::PlanInvalid
    ));
    assert_eq!(history_counts(&connection), before_missing);

    let continuation = format!(
        r#"{{"plan_entity_id":"{}","expected_plan_entity_version_id":"{}","expected_plan_state_digest":"{}"}}"#,
        nested_plan.plan_entity_id,
        nested_plan.plan_entity_version_id,
        nested_plan.plan_state_digest,
    );
    let manifest = supersede_manifest_for_admission(
        &engine,
        &updated_admission,
        "p0-nested-continuation",
        "Replacement strategy with an explicit nested frontier",
        r#"{"mode":"carry_all"}"#,
    )
    .replace(
        r#""nested_plan_continuations": []"#,
        &format!(r#""nested_plan_continuations": [{continuation}]"#),
    );
    let result = engine
        .evolve_plan(PlanEvolutionOptions::new(
            workspace.initial_branch_id,
            parse_evolve(&manifest),
        ))
        .expect("continue nested Plan subtree");
    assert_eq!(result.task_dispositions.len(), admission.tasks.len());
    assert!(result.task_dispositions.iter().all(|item| {
        item.task_entity_id != nested_task.task_entity_id
            && item.action.as_str() == "retain_original"
    }));
    let new_plan_id = result
        .new_plan
        .as_ref()
        .expect("replacement Plan")
        .entity_id;
    assert!(result.disposition_relations.iter().any(|relation| {
        relation.source_entity_id == new_plan_id
            && relation.target_entity_id == nested_plan.plan_entity_id
    }));
    let scope = engine
        .current_scope_at(result.commit_id)
        .expect("current scope after nested continuation");
    assert!(scope.current_plans.contains(&new_plan_id));
    assert!(scope.current_plans.contains(&nested_plan.plan_entity_id));
    assert!(scope.current_tasks.contains(&nested_task.task_entity_id));
    assert!(!scope.current_plans.contains(&admission.plan.entity_id));
    engine
        .validate_integrity()
        .expect("nested continuation integrity");
}

#[test]
fn schema2_supersedes_nested_target_under_guarded_plan_parent() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let admission = admit_base_plan(&mut engine, &workspace, "p0-nested-target-admit");
    let nested_plan = engine
        .create_plan(
            PlanCreateOptions::new(
                workspace.initial_branch_id,
                admission.commit_id,
                "Nested target Plan",
                "Nested original strategy",
            )
            .expect("nested target options"),
        )
        .expect("create nested target");
    let parent_relation = engine
        .create_primary_containment(
            PrimaryContainmentCreateOptions::new(
                workspace.initial_branch_id,
                nested_plan.commit_id,
                admission.plan.entity_id,
                nested_plan.plan_entity_id,
            )
            .expect("nested target containment options"),
        )
        .expect("contain nested target");
    let nested_task = engine
        .create_task(
            TaskCreateOptions::new(
                workspace.initial_branch_id,
                parent_relation.commit_id,
                "Nested target task",
            )
            .expect("nested target Task options"),
        )
        .expect("create nested target Task");
    let task_relation = engine
        .create_primary_containment(
            PrimaryContainmentCreateOptions::new(
                workspace.initial_branch_id,
                nested_task.commit_id,
                nested_plan.plan_entity_id,
                nested_task.task_entity_id,
            )
            .expect("nested target Task containment options"),
        )
        .expect("contain nested target Task");
    let manifest = format!(
        r#"{{
  "mode":"supersede",
  "schema_version":2,
  "idempotency_key":"p0-nested-target-supersede",
  "expected_head_commit_id":"{head}",
  "expected_state_digest":"{state_digest}",
  "target_plan_entity_id":"{target_plan}",
  "expected_plan_entity_version_id":"{target_version}",
  "expected_plan_state_digest":"{target_digest}",
  "expected_parent":{{
    "kind":"plan",
    "entity_id":"{parent_plan}",
    "expected_entity_version_id":"{parent_version}",
    "containment_relation_id":"{parent_relation_id}",
    "containment_relation_version_id":"{parent_relation_version}"
  }},
  "plan":{{"description":"Nested replacement Plan","strategy":"Nested replacement strategy","constraints":{{"mode":"carry_all"}}}},
  "tasks":[],
  "task_dispositions":[{{
    "task_entity_id":"{task_id}",
    "expected_task_entity_version_id":"{task_version}",
    "expected_task_state_digest":"{task_digest}",
    "action":"retain_original",
    "return_condition":"retain the exact nested task for later reassessment",
    "rationale":{{"assessment":"do not invent a replacement"}}
  }}],
  "nested_plan_continuations":[],
  "records":[],
  "evidence":[],
  "rationale":{{"source":"nested-target-test"}}
}}"#,
        head = task_relation.commit_id,
        state_digest = task_relation.work_state_digest,
        target_plan = nested_plan.plan_entity_id,
        target_version = nested_plan.plan_entity_version_id,
        target_digest = nested_plan.plan_state_digest,
        parent_plan = admission.plan.entity_id,
        parent_version = admission.plan.entity_version_id,
        parent_relation_id = parent_relation.relation_id,
        parent_relation_version = parent_relation.relation_version_id,
        task_id = nested_task.task_entity_id,
        task_version = nested_task.task_entity_version_id,
        task_digest = nested_task.task_state_digest,
    );
    let result = engine
        .evolve_plan(PlanEvolutionOptions::new(
            workspace.initial_branch_id,
            parse_evolve(&manifest),
        ))
        .expect("supersede nested target");
    let replacement = result.new_plan.as_ref().expect("nested replacement");
    let new_parent_relation = result
        .goal_contains_relation
        .as_ref()
        .expect("replacement containment");
    assert_eq!(
        new_parent_relation.source_entity_id,
        admission.plan.entity_id
    );
    assert_eq!(new_parent_relation.target_entity_id, replacement.entity_id);
    let scope = engine.current_scope_at(result.commit_id).unwrap();
    assert!(scope.current_plans.contains(&admission.plan.entity_id));
    assert!(scope.current_plans.contains(&replacement.entity_id));
    assert!(!scope.current_plans.contains(&nested_plan.plan_entity_id));
    assert!(!scope.current_tasks.contains(&nested_task.task_entity_id));
    engine
        .validate_integrity()
        .expect("nested target replacement integrity");
}

#[test]
fn active_claim_blocks_terminal_disposition_but_continue_preserves_identity_and_scope() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let admission = engine
        .admit_plan(PlanAdmissionOptions::new(
            workspace.initial_branch_id,
            parse_admit(&admit_manifest_with_prior_context_json(
                &workspace,
                "p0-active-claim-admit",
            )),
        ))
        .expect("admit active claim fixture");
    let task = admission.tasks.first().expect("admitted Task");
    let session = engine
        .start_session(
            SessionStartOptions::new(workspace.workspace_id, workspace.initial_branch_id)
                .expect("session options"),
        )
        .expect("start session");
    let claim = engine
        .claim_task(ClaimTaskOptions::new(session.session_id, task.entity_id))
        .expect("claim current Task");
    let connection = raw_connection(&path);
    let cancel_manifest = supersede_manifest_for_admission(
        &engine,
        &admission,
        "p0-active-claim-cancel",
        "Replacement strategy attempting cancellation",
        r#"{"mode":"carry_all"}"#,
    )
    .replace(r#""action":"retain_original""#, r#""action":"cancel""#);
    let before_cancel = history_counts(&connection);
    let cancelled = engine.evolve_plan(PlanEvolutionOptions::new(
        workspace.initial_branch_id,
        parse_evolve(&cancel_manifest),
    ));
    assert!(matches!(
        cancelled,
        Err(error) if error.code() == ErrorCode::ClaimInvalid
    ));
    assert_eq!(history_counts(&connection), before_cancel);
    assert_eq!(
        engine
            .branch_head(workspace.initial_branch_id)
            .expect("head after rejected claim conflict")
            .head_commit_id,
        admission.commit_id
    );

    let continue_manifest = supersede_manifest_for_admission(
        &engine,
        &admission,
        "p0-active-claim-continue",
        "Replacement strategy preserving the claimed Task",
        r#"{"mode":"carry_all"}"#,
    )
    .replace(r#""action":"retain_original""#, r#""action":"continue""#);
    let continued = engine
        .evolve_plan(PlanEvolutionOptions::new(
            workspace.initial_branch_id,
            parse_evolve(&continue_manifest),
        ))
        .expect("continue claimed Task under replacement Plan");
    let disposition = continued
        .task_dispositions
        .iter()
        .find(|item| item.task_entity_id == task.entity_id)
        .expect("continued Task assessment");
    assert_eq!(disposition.entity_version_id, task.entity_version_id);
    assert_eq!(disposition.state_digest, task.state_digest);
    assert!(disposition.post_current);
    let scope = engine.current_scope_at(continued.commit_id).unwrap();
    assert!(scope.current_tasks.contains(&task.entity_id));
    let active_claims = engine
        .active_claims_for_session(ClaimListOptions::for_session(session.session_id))
        .expect("active claims after continuation");
    assert_eq!(active_claims.claims.len(), 1);
    assert_eq!(active_claims.claims[0].claim_id, claim.claim_id);
    assert_eq!(active_claims.claims[0].task_entity_id, task.entity_id);
}

#[test]
fn plan_evolution_plan_update_presence_boundaries_are_explicit() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let connection = raw_connection(&path);
    let admission = admit_base_plan(&mut engine, &workspace, "p0-evolve-presence-admit");

    let before_null = history_counts(&connection);
    let null_manifest = evolve_manifest_with_plan_update_json(
        admission.commit_id,
        admission.work_state_digest,
        admission.plan.entity_id,
        admission.plan.entity_version_id,
        admission.plan.state_digest,
        "p0-evolve-null-description",
        r#"{"description": null}"#,
    );
    let null_result = PlanEvolutionManifest::from_json_bytes(null_manifest.as_bytes());
    assert!(matches!(
        null_result,
        Err(error) if error.code() == ErrorCode::PlanInvalid
    ));
    assert_eq!(history_counts(&connection), before_null);
    assert_eq!(
        engine
            .branch_head(workspace.initial_branch_id)
            .expect("branch head after null")
            .head_commit_id,
        admission.commit_id
    );

    for (key, plan_update) in [
        ("p0-evolve-empty-description", r#"{"description": ""}"#),
        ("p0-evolve-empty-strategy", r#"{"strategy": ""}"#),
    ] {
        let before_invalid = history_counts(&connection);
        let invalid = engine.evolve_plan(PlanEvolutionOptions::new(
            workspace.initial_branch_id,
            parse_evolve(&evolve_manifest_with_plan_update_json(
                admission.commit_id,
                admission.work_state_digest,
                admission.plan.entity_id,
                admission.plan.entity_version_id,
                admission.plan.state_digest,
                key,
                plan_update,
            )),
        ));
        assert!(matches!(
            invalid,
            Err(error) if error.code() == ErrorCode::PlanInvalid
        ));
        assert_eq!(history_counts(&connection), before_invalid);
        assert_eq!(
            engine
                .branch_head(workspace.initial_branch_id)
                .expect("branch head after invalid text")
                .head_commit_id,
            admission.commit_id
        );
    }

    let clear_constraints = engine
        .evolve_plan(PlanEvolutionOptions::new(
            workspace.initial_branch_id,
            parse_evolve(&evolve_manifest_with_plan_update_json(
                admission.commit_id,
                admission.work_state_digest,
                admission.plan.entity_id,
                admission.plan.entity_version_id,
                admission.plan.state_digest,
                "p0-evolve-clear-constraints",
                r#"{"constraints": []}"#,
            )),
        ))
        .expect("clear constraints");
    let evolved_plan = engine
        .plan_at(clear_constraints.commit_id, admission.plan.entity_id)
        .expect("evolved plan");
    assert_eq!(evolved_plan.state.description, "Original Plan");
    assert_eq!(evolved_plan.state.strategy, "Original Strategy");
    assert!(evolved_plan.state.constraints.is_empty());
    let old_plan = engine
        .plan_at(admission.commit_id, admission.plan.entity_id)
        .expect("old plan");
    assert_eq!(old_plan.state.constraints, vec!["Keep scoped".to_owned()]);
}

#[test]
fn plan_evolution_replays_same_payload_after_reopen_and_rejects_key_conflict() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let admission = admit_base_plan(&mut engine, &workspace, "p0-evolve-replay-admit");
    let manifest_json = evolve_manifest_json(&admission, "p0-evolve-replay", Some("Replay Plan"));

    let first = engine
        .evolve_plan(PlanEvolutionOptions::new(
            workspace.initial_branch_id,
            parse_evolve(&manifest_json),
        ))
        .expect("first evolve");
    drop(engine);

    let mut reopened = Engine::open(&path).expect("reopen engine");
    let connection = raw_connection(&path);
    let before_replay = history_counts(&connection);
    let replay = reopened
        .evolve_plan(PlanEvolutionOptions::new(
            workspace.initial_branch_id,
            parse_evolve(&manifest_json),
        ))
        .expect("replay evolve");
    let after_replay = history_counts(&connection);
    let head = reopened
        .branch_head(workspace.initial_branch_id)
        .expect("branch head after replay");

    assert_eq!(replay.outcome, PlanEvolutionOutcome::Reused);
    assert_eq!(replay.commit_id, first.commit_id);
    assert_eq!(replay.changeset_id, first.changeset_id);
    assert_eq!(head.head_commit_id, first.commit_id);
    assert_eq!(before_replay, after_replay);

    let before_conflict = history_counts(&connection);
    let conflict = reopened.evolve_plan(PlanEvolutionOptions::new(
        workspace.initial_branch_id,
        parse_evolve(&evolve_manifest_json(
            &admission,
            "p0-evolve-replay",
            Some("Different payload"),
        )),
    ));
    assert!(matches!(
        conflict,
        Err(error) if error.code() == ErrorCode::PlanInvalid
    ));
    assert_eq!(history_counts(&connection), before_conflict);
    assert_eq!(
        reopened
            .branch_head(workspace.initial_branch_id)
            .expect("branch head after conflict")
            .head_commit_id,
        first.commit_id
    );
}

#[test]
fn plan_evolution_fails_closed_for_stale_digest_status_and_missing_containment() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let connection = raw_connection(&path);
    let admission = admit_base_plan(&mut engine, &workspace, "p0-evolve-failure-admit");

    let bad_digest_json = evolve_manifest_json(&admission, "p0-evolve-bad-digest", Some("Bad"))
        .replace(
            &format!(
                r#""expected_plan_state_digest": "{}""#,
                admission.plan.state_digest
            ),
            r#""expected_plan_state_digest": "0000000000000000000000000000000000000000000000000000000000000000""#,
        );
    let before_bad_digest = history_counts(&connection);
    let bad_digest = engine.evolve_plan(PlanEvolutionOptions::new(
        workspace.initial_branch_id,
        parse_evolve(&bad_digest_json),
    ));
    assert!(matches!(
        bad_digest,
        Err(error) if error.code() == ErrorCode::DigestInvalid
    ));
    assert_eq!(history_counts(&connection), before_bad_digest);

    let first = engine
        .evolve_plan(PlanEvolutionOptions::new(
            workspace.initial_branch_id,
            parse_evolve(&evolve_manifest_json(
                &admission,
                "p0-evolve-first",
                Some("First"),
            )),
        ))
        .expect("first evolve");
    let before_stale = history_counts(&connection);
    let stale = engine.evolve_plan(PlanEvolutionOptions::new(
        workspace.initial_branch_id,
        parse_evolve(&evolve_manifest_json(
            &admission,
            "p0-evolve-stale",
            Some("Stale"),
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

    let (mut status_engine, status_workspace) =
        create_workspace(&path.with_extension("status.sqlite"));
    let status_admission = admit_base_plan(
        &mut status_engine,
        &status_workspace,
        "p0-evolve-status-admit",
    );
    let completed = status_engine
        .transition_plan(
            PlanTransitionOptions::complete(
                status_workspace.initial_branch_id,
                status_admission.commit_id,
                status_admission.plan.entity_id,
                status_admission.plan.entity_version_id,
            )
            .expect("complete options"),
        )
        .expect("complete plan");
    let status_connection = raw_connection(&path.with_extension("status.sqlite"));
    let status_before = history_counts(&status_connection);
    let status_json = evolve_manifest_json(
        &workvcs_core::PlanAdmissionResult {
            commit_id: completed.commit_id,
            work_state_digest: completed.work_state_digest,
            plan: workvcs_core::PlanAdmissionEntityResult {
                entity_id: completed.plan_entity_id,
                entity_version_id: completed.plan_entity_version_id,
                state_digest: completed.plan_state_digest,
            },
            ..status_admission
        },
        "p0-evolve-status",
        Some("Completed plan cannot evolve"),
    );
    let status_result = status_engine.evolve_plan(PlanEvolutionOptions::new(
        status_workspace.initial_branch_id,
        parse_evolve(&status_json),
    ));
    assert!(matches!(
        status_result,
        Err(error) if error.code() == ErrorCode::PlanInvalid
    ));
    assert_eq!(history_counts(&status_connection), status_before);

    let (mut orphan_engine, orphan_workspace) =
        create_workspace(&path.with_extension("orphan.sqlite"));
    let orphan_plan = orphan_engine
        .create_plan(
            PlanCreateOptions::new(
                orphan_workspace.initial_branch_id,
                orphan_workspace.genesis_commit_id,
                "Orphan Plan",
                "No goal containment",
            )
            .expect("orphan plan options"),
        )
        .expect("create orphan plan");
    let orphan_connection = raw_connection(&path.with_extension("orphan.sqlite"));
    let orphan_before = history_counts(&orphan_connection);
    let orphan_manifest = format!(
        r#"{{
  "mode": "in_place",
  "schema_version": 1,
  "idempotency_key": "p0-evolve-orphan",
  "expected_head_commit_id": "{head}",
  "expected_state_digest": "{state_digest}",
  "target_plan_entity_id": "{plan_id}",
  "expected_plan_entity_version_id": "{plan_version}",
  "expected_plan_state_digest": "{plan_digest}",
  "plan": {{"description": "Still orphaned"}},
  "tasks": [],
  "records": [],
  "evidence": [],
  "rationale": {{}}
}}"#,
        head = orphan_plan.commit_id,
        state_digest = orphan_plan.work_state_digest,
        plan_id = orphan_plan.plan_entity_id,
        plan_version = orphan_plan.plan_entity_version_id,
        plan_digest = orphan_plan.plan_state_digest
    );
    let orphan_result = orphan_engine.evolve_plan(PlanEvolutionOptions::new(
        orphan_workspace.initial_branch_id,
        parse_evolve(&orphan_manifest),
    ));
    assert!(matches!(
        orphan_result,
        Err(error) if error.code() == ErrorCode::RelationInvalid
    ));
    assert_eq!(history_counts(&orphan_connection), orphan_before);
}

#[test]
fn retired_plan_repair_continues_three_tasks_to_goal_and_retains_one_historical() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let admission = engine
        .admit_plan(PlanAdmissionOptions::new(
            workspace.initial_branch_id,
            parse_admit(&disposition_fixture_manifest_json(
                &workspace,
                "p0-disposition-admit",
            )),
        ))
        .expect("admit disposition fixture");
    let initial_tasks = admission.tasks.clone();
    let initial_by_local_id = initial_tasks
        .iter()
        .map(|task| (task.local_id.as_str(), task))
        .collect::<std::collections::BTreeMap<_, _>>();

    let abandoned = engine
        .transition_plan(
            PlanTransitionOptions::abandon(
                workspace.initial_branch_id,
                admission.commit_id,
                admission.plan.entity_id,
                admission.plan.entity_version_id,
                "the plan is retired but its task evidence still requires assessment",
            )
            .expect("abandon options"),
        )
        .expect("abandon plan");
    let external = engine
        .create_task(
            TaskCreateOptions::new(
                workspace.initial_branch_id,
                abandoned.commit_id,
                "Current downstream roadmap task",
            )
            .expect("external task options"),
        )
        .expect("create external task");

    let umbrella = initial_by_local_id["umbrella"];
    let mcp = initial_by_local_id["mcp"];
    let review = initial_by_local_id["review"];
    let deferred = initial_by_local_id["deferred"];
    let review_umbrella = engine
        .create_task_scheduling_relation(
            TaskSchedulingRelationCreateOptions::depends_on(
                workspace.initial_branch_id,
                external.commit_id,
                review.entity_id,
                umbrella.entity_id,
            )
            .expect("review umbrella dependency"),
        )
        .expect("create review umbrella dependency");
    let review_mcp = engine
        .create_task_scheduling_relation(
            TaskSchedulingRelationCreateOptions::depends_on(
                workspace.initial_branch_id,
                review_umbrella.commit_id,
                review.entity_id,
                mcp.entity_id,
            )
            .expect("review mcp dependency"),
        )
        .expect("create review mcp dependency");
    let deferred_review = engine
        .create_task_scheduling_relation(
            TaskSchedulingRelationCreateOptions::depends_on(
                workspace.initial_branch_id,
                review_mcp.commit_id,
                deferred.entity_id,
                review.entity_id,
            )
            .expect("deferred review dependency"),
        )
        .expect("create deferred review dependency");
    let external_review = engine
        .create_task_scheduling_relation(
            TaskSchedulingRelationCreateOptions::depends_on(
                workspace.initial_branch_id,
                deferred_review.commit_id,
                external.task_entity_id,
                review.entity_id,
            )
            .expect("external review dependency"),
        )
        .expect("create external review dependency");

    let retired_plan = engine
        .plan_at(external_review.commit_id, admission.plan.entity_id)
        .expect("retired plan snapshot");
    let carrier_goal = engine
        .goal_at(external_review.commit_id, admission.goal.entity_id)
        .expect("carrier goal snapshot");
    let manifest_json = repair_manifest_json(
        "p0-disposition-repair",
        external_review.commit_id,
        external_review.work_state_digest,
        &retired_plan,
        Some(&carrier_goal),
        &initial_tasks,
        &[
            ("umbrella", "continue", "remains carried by the active goal"),
            ("mcp", "continue", "remains carried by the active goal"),
            ("review", "continue", "remains carried by the active goal"),
            (
                "deferred",
                "retain_original",
                "reassess only after its prerequisite review completes",
            ),
        ],
    );
    let manifest = parse_evolve(&manifest_json);
    let result = engine
        .evolve_plan(PlanEvolutionOptions::new(
            workspace.initial_branch_id,
            manifest.clone(),
        ))
        .expect("repair retired plan tasks");

    assert_eq!(result.mode, "task_disposition");
    assert!(!result.plan_changed);
    assert_eq!(
        result.plan.entity_version_id,
        retired_plan.plan_entity_version_id
    );
    assert_eq!(result.plan.state_digest, retired_plan.state_digest);
    assert_eq!(result.disposition_relations.len(), 3);
    assert!(result.disposition_relations.iter().all(|relation| {
        relation.relation_type == "references"
            && relation.source_entity_id == carrier_goal.goal_entity_id
    }));
    assert!(result.tasks.is_empty());
    assert_eq!(result.task_dispositions.len(), 4);
    for disposition in &result.task_dispositions {
        let initial = initial_tasks
            .iter()
            .find(|task| task.entity_id == disposition.task_entity_id)
            .expect("initial task");
        assert_eq!(
            disposition.previous_entity_version_id,
            initial.entity_version_id
        );
        assert_eq!(disposition.previous_state_digest, initial.state_digest);
        assert_eq!(disposition.entity_version_id, initial.entity_version_id);
        assert_eq!(disposition.state_digest, initial.state_digest);
        assert!(disposition.replacement_task_entity_id.is_none());
        if disposition.task_entity_id == deferred.entity_id {
            assert!(!disposition.post_current);
        } else {
            assert!(disposition.post_current);
        }
    }

    let post_scope = engine
        .current_scope_at(result.commit_id)
        .expect("post repair current scope");
    assert!(post_scope.current_tasks.contains(&umbrella.entity_id));
    assert!(post_scope.current_tasks.contains(&mcp.entity_id));
    assert!(post_scope.current_tasks.contains(&review.entity_id));
    assert!(!post_scope.current_tasks.contains(&deferred.entity_id));
    assert!(post_scope.current_tasks.contains(&external.task_entity_id));
    for initial in &initial_tasks {
        let post = engine
            .task_at(result.commit_id, initial.entity_id)
            .expect("post repair task");
        assert_eq!(post.task_entity_version_id, initial.entity_version_id);
        assert_eq!(post.state_digest, initial.state_digest);
    }

    let audit = engine
        .plan_task_disposition_audit(PlanTaskDispositionAuditOptions::new(
            workspace.initial_branch_id,
            admission.plan.entity_id,
        ))
        .expect("audit repaired plan");
    assert!(audit.retired_plan_orphan);
    assert_eq!(audit.tasks.len(), 4);
    let deferred_audit = audit
        .tasks
        .iter()
        .find(|task| task.task_entity_id == deferred.entity_id)
        .expect("deferred audit row");
    assert!(!deferred_audit.current);
    let review_audit = audit
        .tasks
        .iter()
        .find(|task| task.task_entity_id == review.entity_id)
        .expect("review audit row");
    assert!(
        review_audit
            .external_dependents
            .iter()
            .any(
                |dependent| dependent.task_entity_id == external.task_entity_id
                    && dependent.current
            )
    );

    let connection = raw_connection(&path);
    let before_retry = history_counts(&connection);
    let replay = engine
        .evolve_plan(PlanEvolutionOptions::new(
            workspace.initial_branch_id,
            manifest,
        ))
        .expect("exact repair retry");
    assert_eq!(replay.outcome, PlanEvolutionOutcome::Reused);
    assert_eq!(replay.commit_id, result.commit_id);
    assert_eq!(history_counts(&connection), before_retry);
}

#[test]
fn retain_dependency_fence_distinguishes_existing_blocker_from_new_regression() {
    let (_mixed_tempdir, mixed_path) = store_path();
    let (
        mut mixed_engine,
        mixed_workspace,
        mixed_admission,
        mixed_plan,
        mixed_carrier,
        mixed_head,
        mixed_digest,
    ) = retired_fixture_with_dependency(&mixed_path, "p0-mixed-dependency-admit");
    let mixed_json = repair_manifest_json(
        "p0-mixed-dependency-repair",
        mixed_head,
        mixed_digest,
        &mixed_plan,
        Some(&mixed_carrier),
        &mixed_admission.tasks,
        &[
            ("umbrella", "retain_original", "keep historical"),
            ("review", "continue", "restore current review"),
            ("mcp", "retain_original", "keep historical"),
            ("deferred", "retain_original", "keep historical"),
        ],
    );
    let mixed_connection = raw_connection(&mixed_path);
    let before_mixed = history_counts(&mixed_connection);
    let mixed = mixed_engine.evolve_plan(PlanEvolutionOptions::new(
        mixed_workspace.initial_branch_id,
        parse_evolve(&mixed_json),
    ));
    assert!(matches!(
        mixed,
        Err(error) if error.code() == ErrorCode::RelationInvalid
    ));
    assert_eq!(history_counts(&mixed_connection), before_mixed);
    assert_eq!(
        mixed_engine
            .branch_head(mixed_workspace.initial_branch_id)
            .expect("mixed branch head")
            .head_commit_id,
        mixed_head
    );

    let (_retain_tempdir, retain_path) = store_path();
    let (
        mut retain_engine,
        retain_workspace,
        retain_admission,
        retain_plan,
        _retain_carrier,
        retain_head,
        retain_digest,
    ) = retired_fixture_with_dependency(&retain_path, "p0-existing-dependency-admit");
    let retain_json = repair_manifest_json(
        "p0-existing-dependency-assessment",
        retain_head,
        retain_digest,
        &retain_plan,
        None,
        &retain_admission.tasks,
        &[
            (
                "umbrella",
                "retain_original",
                "existing blocker remains explicit",
            ),
            ("review", "retain_original", "keep historical"),
            ("mcp", "retain_original", "keep historical"),
            ("deferred", "retain_original", "keep historical"),
        ],
    );
    let retain_manifest = parse_evolve(&retain_json);
    let retained = retain_engine
        .evolve_plan(PlanEvolutionOptions::new(
            retain_workspace.initial_branch_id,
            retain_manifest.clone(),
        ))
        .expect("record existing blocked dependency assessment");
    assert!(retained.disposition_relations.is_empty());
    let retained_umbrella = retained
        .task_dispositions
        .iter()
        .find(|item| {
            item.task_entity_id
                == retain_admission
                    .tasks
                    .iter()
                    .find(|task| task.local_id == "umbrella")
                    .expect("retained umbrella")
                    .entity_id
        })
        .expect("umbrella assessment");
    assert!(!retained_umbrella.post_current);
    assert!(
        retained_umbrella
            .risks
            .contains(&"existing_current_dependency_on_noncurrent_task".to_owned())
    );
    drop(retain_engine);
    let mut reopened = Engine::open(&retain_path).expect("reopen all-retain Store");
    reopened
        .validate_integrity()
        .expect("all-retain Store replay integrity");
    let reopened_head = reopened
        .branch_head(retain_workspace.initial_branch_id)
        .expect("all-retain branch head");
    assert_eq!(reopened_head.head_commit_id, retained.commit_id);
    assert_eq!(reopened_head.state_digest, retained.work_state_digest);
    let reopened_scope = reopened
        .current_scope_at(retained.commit_id)
        .expect("all-retain current scope");
    assert!(
        retain_admission
            .tasks
            .iter()
            .all(|task| !reopened_scope.current_tasks.contains(&task.entity_id))
    );
    for initial in &retain_admission.tasks {
        let post = reopened
            .task_at(retained.commit_id, initial.entity_id)
            .expect("all-retain task guard");
        assert_eq!(post.task_entity_version_id, initial.entity_version_id);
        assert_eq!(post.state_digest, initial.state_digest);
    }
    let reopened_audit = reopened
        .plan_task_disposition_audit(PlanTaskDispositionAuditOptions::new(
            retain_workspace.initial_branch_id,
            retain_admission.plan.entity_id,
        ))
        .expect("all-retain audit after reopen");
    assert!(reopened_audit.retired_plan_orphan);
    let exact_retain = reopened
        .evolve_plan(PlanEvolutionOptions::new(
            retain_workspace.initial_branch_id,
            retain_manifest,
        ))
        .expect("all-retain exact retry after reopen");
    assert_eq!(exact_retain.outcome, PlanEvolutionOutcome::Reused);
    assert_eq!(exact_retain.task_dispositions, retained.task_dispositions);

    let (_continued_tempdir, continued_path) = store_path();
    let (
        mut continued_engine,
        continued_workspace,
        continued_admission,
        continued_plan,
        continued_carrier,
        continued_head,
        continued_digest,
    ) = retired_fixture_with_dependency(&continued_path, "p0-continued-dependency-admit");
    let continued_json = repair_manifest_json(
        "p0-continued-dependency-repair",
        continued_head,
        continued_digest,
        &continued_plan,
        Some(&continued_carrier),
        &continued_admission.tasks,
        &[
            ("umbrella", "continue", "restore prerequisite currentness"),
            ("review", "continue", "restore dependent currentness"),
            ("mcp", "retain_original", "keep historical"),
            ("deferred", "retain_original", "keep historical"),
        ],
    );
    let continued = continued_engine
        .evolve_plan(PlanEvolutionOptions::new(
            continued_workspace.initial_branch_id,
            parse_evolve(&continued_json),
        ))
        .expect("continue prerequisite and dependent together");
    assert_eq!(continued.disposition_relations.len(), 2);
    assert!(
        continued
            .task_dispositions
            .iter()
            .filter(|item| item.action.as_str() == "continue")
            .all(|item| item.post_current)
    );
}

#[test]
fn pure_historical_retain_preserves_existing_stale_claim_as_diagnostic() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let admission = engine
        .admit_plan(PlanAdmissionOptions::new(
            workspace.initial_branch_id,
            parse_admit(&disposition_fixture_manifest_json(
                &workspace,
                "p0-stale-claim-retain-admit",
            )),
        ))
        .expect("admit stale-claim fixture");
    let claimed_task = admission
        .tasks
        .iter()
        .find(|task| task.local_id == "umbrella")
        .expect("claimed Task");
    let session = engine
        .start_session(
            SessionStartOptions::new(workspace.workspace_id, workspace.initial_branch_id)
                .expect("session options"),
        )
        .expect("start session");
    let claim = engine
        .claim_task(ClaimTaskOptions::new(
            session.session_id,
            claimed_task.entity_id,
        ))
        .expect("claim current Task");
    let abandoned = engine
        .transition_plan(
            PlanTransitionOptions::abandon(
                workspace.initial_branch_id,
                admission.commit_id,
                admission.plan.entity_id,
                admission.plan.entity_version_id,
                "retire the claimed Task scope",
            )
            .expect("abandon options"),
        )
        .expect("abandon claimed Plan");
    let retired_plan = engine
        .plan_at(abandoned.commit_id, admission.plan.entity_id)
        .expect("retired Plan snapshot");
    let manifest = parse_evolve(&repair_manifest_json(
        "p0-stale-claim-retain-assessment",
        abandoned.commit_id,
        abandoned.work_state_digest,
        &retired_plan,
        None,
        &admission.tasks,
        &[
            (
                "umbrella",
                "retain_original",
                "preserve stale ownership diagnostic",
            ),
            ("review", "retain_original", "keep historical"),
            ("mcp", "retain_original", "keep historical"),
            ("deferred", "retain_original", "keep historical"),
        ],
    ));
    let claims_before = engine
        .active_claims_for_session(ClaimListOptions::for_session(session.session_id))
        .expect("claims before retain");
    let retained = engine
        .evolve_plan(PlanEvolutionOptions::new(
            workspace.initial_branch_id,
            manifest,
        ))
        .expect("record pure historical retain assessment");
    let claimed_assessment = retained
        .task_dispositions
        .iter()
        .find(|item| item.task_entity_id == claimed_task.entity_id)
        .expect("claimed Task assessment");
    assert!(!claimed_assessment.post_current);
    assert!(
        claimed_assessment
            .risks
            .contains(&"existing_active_claim_on_noncurrent_task".to_owned())
    );
    for initial in &admission.tasks {
        let post = engine
            .task_at(retained.commit_id, initial.entity_id)
            .expect("retained Task snapshot");
        assert_eq!(post.task_entity_version_id, initial.entity_version_id);
        assert_eq!(post.state_digest, initial.state_digest);
    }
    let claims_after = engine
        .active_claims_for_session(ClaimListOptions::for_session(session.session_id))
        .expect("claims after retain");
    assert_eq!(claims_after.claims, claims_before.claims);
    assert_eq!(claims_after.claims[0].claim_id, claim.claim_id);
    let audit = engine
        .plan_task_disposition_audit(PlanTaskDispositionAuditOptions::new(
            workspace.initial_branch_id,
            admission.plan.entity_id,
        ))
        .expect("audit stale claim after retain");
    let audited_claim = audit
        .tasks
        .iter()
        .find(|task| task.task_entity_id == claimed_task.entity_id)
        .expect("audited claimed Task");
    assert_eq!(audited_claim.active_claims, 1);
    assert!(audited_claim.stale_scope_claim);
    assert!(!audited_claim.current);
    let runnable = engine
        .runnable_tasks(RunnableTasksOptions::new(session.session_id))
        .expect("runnable projection after retain");
    assert!(
        runnable
            .candidates
            .iter()
            .all(|candidate| candidate.task.task_entity_id != claimed_task.entity_id)
    );
}

#[test]
fn v2_exact_lookup_rejects_missing_durable_assessment_fields_without_writes() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace, admission, retired_plan, _carrier, head, state_digest) =
        retired_fixture_with_dependency(&path, "p0-v2-strict-admit");
    let manifest = parse_evolve(&repair_manifest_json(
        "p0-v2-strict-assessment",
        head,
        state_digest,
        &retired_plan,
        None,
        &admission.tasks,
        &[
            ("umbrella", "retain_original", "keep historical"),
            ("review", "retain_original", "keep historical"),
            ("mcp", "retain_original", "keep historical"),
            ("deferred", "retain_original", "keep historical"),
        ],
    ));
    let options = PlanEvolutionOptions::new(workspace.initial_branch_id, manifest);
    let created = engine
        .evolve_plan(options.clone())
        .expect("create strict v2 assessment");
    let exact = engine
        .find_plan_evolution_result(options.clone())
        .expect("full v2 exact lookup")
        .expect("full v2 exact result");
    assert_eq!(exact.outcome, PlanEvolutionOutcome::Reused);
    assert_eq!(exact.task_dispositions, created.task_dispositions);

    let connection = raw_connection(&path);
    let original: String = connection
        .query_row(
            "SELECT operation_payload_json FROM changeset WHERE operation_type = 'plan.evolve' AND operation_schema_version = 2",
            [],
            |row| row.get(0),
        )
        .expect("v2 operation payload");
    let before = history_counts(&connection);
    for field in ["plan_changed", "task_dispositions", "disposition_relations"] {
        let mut value: serde_json::Value =
            serde_json::from_str(&original).expect("decode v2 payload");
        value
            .as_object_mut()
            .expect("v2 object")
            .remove(field)
            .expect("remove required field");
        connection
            .execute(
                "UPDATE changeset SET operation_payload_json = ?1 WHERE operation_type = 'plan.evolve' AND operation_schema_version = 2",
                [serde_json::to_string(&value).expect("encode damaged payload")],
            )
            .expect("damage v2 payload");
        let lookup = engine.find_plan_evolution_result(options.clone());
        assert!(matches!(
            lookup,
            Err(error) if error.code() == ErrorCode::PlanInvalid
        ));
        let reopened = Engine::open(&path).expect("open damaged v2 Store for integrity check");
        assert!(reopened.validate_integrity().is_err());
        assert_eq!(history_counts(&connection), before);
        connection
            .execute(
                "UPDATE changeset SET operation_payload_json = ?1 WHERE operation_type = 'plan.evolve' AND operation_schema_version = 2",
                [&original],
            )
            .expect("restore v2 payload");
    }

    let mut missing_risks: serde_json::Value =
        serde_json::from_str(&original).expect("decode v2 payload for risks");
    missing_risks
        .get_mut("task_dispositions")
        .and_then(serde_json::Value::as_array_mut)
        .and_then(|values| values.first_mut())
        .and_then(serde_json::Value::as_object_mut)
        .expect("first task disposition")
        .remove("risks")
        .expect("remove risks");
    connection
        .execute(
            "UPDATE changeset SET operation_payload_json = ?1 WHERE operation_type = 'plan.evolve' AND operation_schema_version = 2",
            [serde_json::to_string(&missing_risks).expect("encode missing risks payload")],
        )
        .expect("remove v2 risks");
    let lookup = engine.find_plan_evolution_result(options.clone());
    assert!(matches!(
        lookup,
        Err(error) if error.code() == ErrorCode::PlanInvalid
    ));
    let reopened = Engine::open(&path).expect("open missing-risks Store for integrity check");
    assert!(reopened.validate_integrity().is_err());
    assert_eq!(history_counts(&connection), before);
    connection
        .execute(
            "UPDATE changeset SET operation_payload_json = ?1 WHERE operation_type = 'plan.evolve' AND operation_schema_version = 2",
            [&original],
        )
        .expect("restore v2 payload after risks check");

    let mut damaged_payloads = Vec::new();
    let mut plan_version: serde_json::Value =
        serde_json::from_str(&original).expect("decode plan version tamper");
    plan_version["plan"]["entity_version_id"] =
        serde_json::Value::String(admission.plan.entity_version_id.to_string());
    damaged_payloads.push(plan_version);
    let mut plan_digest: serde_json::Value =
        serde_json::from_str(&original).expect("decode plan digest tamper");
    plan_digest["plan"]["state_digest"] = serde_json::Value::String("00".repeat(32));
    damaged_payloads.push(plan_digest);
    let mut task_version: serde_json::Value =
        serde_json::from_str(&original).expect("decode task version tamper");
    task_version["task_dispositions"][0]["entity_version_id"] =
        serde_json::Value::String(admission.plan.entity_version_id.to_string());
    damaged_payloads.push(task_version);
    let mut task_digest: serde_json::Value =
        serde_json::from_str(&original).expect("decode task digest tamper");
    task_digest["task_dispositions"][0]["state_digest"] =
        serde_json::Value::String("00".repeat(32));
    damaged_payloads.push(task_digest);
    let mut replacement_local: serde_json::Value =
        serde_json::from_str(&original).expect("decode replacement local tamper");
    replacement_local["task_dispositions"][0]["replacement_task_local_id"] =
        serde_json::Value::String("invented".to_owned());
    damaged_payloads.push(replacement_local);
    let mut replacement_id: serde_json::Value =
        serde_json::from_str(&original).expect("decode replacement id tamper");
    replacement_id["task_dispositions"][0]["replacement_task_entity_id"] =
        serde_json::Value::String(admission.plan.entity_id.to_string());
    damaged_payloads.push(replacement_id);
    let mut records: serde_json::Value =
        serde_json::from_str(&original).expect("decode records tamper");
    records["records"] = serde_json::json!([{"local_id":"fake"}]);
    damaged_payloads.push(records);
    let mut evidence: serde_json::Value =
        serde_json::from_str(&original).expect("decode evidence tamper");
    evidence["evidence"] = serde_json::json!([{"local_id":"fake"}]);
    damaged_payloads.push(evidence);
    let mut envelope: serde_json::Value =
        serde_json::from_str(&original).expect("decode envelope tamper");
    envelope["work_state_digest"] = serde_json::Value::String("00".repeat(32));
    damaged_payloads.push(envelope);

    let original_value: serde_json::Value =
        serde_json::from_str(&original).expect("decode v2 payload for paired tamper");
    let original_plan = original_value["plan"].clone();
    let first_task = original_value["task_dispositions"][0].clone();
    let second_task = original_value["task_dispositions"][1].clone();

    let mut plan_entity_swap = original_value.clone();
    plan_entity_swap["plan"]["entity_id"] = first_task["task_entity_id"].clone();
    plan_entity_swap["plan"]["previous_entity_version_id"] =
        first_task["previous_entity_version_id"].clone();
    plan_entity_swap["plan"]["entity_version_id"] = first_task["entity_version_id"].clone();
    plan_entity_swap["plan"]["previous_state_digest"] = first_task["previous_state_digest"].clone();
    plan_entity_swap["plan"]["state_digest"] = first_task["state_digest"].clone();
    damaged_payloads.push(plan_entity_swap);

    let mut paired_plan_guard = original_value.clone();
    paired_plan_guard["plan"]["previous_entity_version_id"] =
        first_task["previous_entity_version_id"].clone();
    paired_plan_guard["plan"]["entity_version_id"] = first_task["entity_version_id"].clone();
    paired_plan_guard["plan"]["previous_state_digest"] =
        first_task["previous_state_digest"].clone();
    paired_plan_guard["plan"]["state_digest"] = first_task["state_digest"].clone();
    damaged_payloads.push(paired_plan_guard);

    let mut task_entity_swap = original_value.clone();
    task_entity_swap["task_dispositions"][0]["task_entity_id"] = original_plan["entity_id"].clone();
    task_entity_swap["task_dispositions"][0]["previous_entity_version_id"] =
        original_plan["previous_entity_version_id"].clone();
    task_entity_swap["task_dispositions"][0]["entity_version_id"] =
        original_plan["entity_version_id"].clone();
    task_entity_swap["task_dispositions"][0]["previous_state_digest"] =
        original_plan["previous_state_digest"].clone();
    task_entity_swap["task_dispositions"][0]["state_digest"] =
        original_plan["state_digest"].clone();
    damaged_payloads.push(task_entity_swap);

    let mut paired_task_guard = original_value.clone();
    paired_task_guard["task_dispositions"][0]["previous_entity_version_id"] =
        second_task["previous_entity_version_id"].clone();
    paired_task_guard["task_dispositions"][0]["entity_version_id"] =
        second_task["entity_version_id"].clone();
    paired_task_guard["task_dispositions"][0]["previous_state_digest"] =
        second_task["previous_state_digest"].clone();
    paired_task_guard["task_dispositions"][0]["state_digest"] = second_task["state_digest"].clone();
    damaged_payloads.push(paired_task_guard);

    let mut duplicate_tasks = original_value;
    duplicate_tasks["task_dispositions"][1] = first_task;
    damaged_payloads.push(duplicate_tasks);

    for damaged in damaged_payloads {
        connection
            .execute(
                "UPDATE changeset SET operation_payload_json = ?1 WHERE operation_type = 'plan.evolve' AND operation_schema_version = 2",
                [serde_json::to_string(&damaged).expect("encode damaged v2 payload")],
            )
            .expect("install damaged v2 payload");
        let lookup = engine.find_plan_evolution_result(options.clone());
        assert!(matches!(
            lookup,
            Err(error)
                if matches!(error.code(), ErrorCode::PlanInvalid | ErrorCode::DigestInvalid)
        ));
        let reopened = Engine::open(&path).expect("open tampered Store for integrity check");
        assert!(reopened.validate_integrity().is_err());
        assert_eq!(history_counts(&connection), before);
    }
    connection
        .execute(
            "UPDATE changeset SET operation_payload_json = ?1 WHERE operation_type = 'plan.evolve' AND operation_schema_version = 2",
            [&original],
        )
        .expect("restore valid v2 payload after paired tamper checks");
    engine
        .validate_integrity()
        .expect("valid state-preserving payload remains replayable");
    let exact = engine
        .find_plan_evolution_result(options)
        .expect("valid state-preserving exact lookup")
        .expect("valid state-preserving exact result");
    assert_eq!(exact.outcome, PlanEvolutionOutcome::Reused);
}
