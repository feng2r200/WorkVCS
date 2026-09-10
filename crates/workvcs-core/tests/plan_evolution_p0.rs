use rusqlite::Connection;
use std::path::{Path, PathBuf};
use tempfile::TempDir;
use workvcs_core::{
    Engine, ErrorCode, GoalCreateOptions, HistoryQueryOptions, PlanAdmissionManifest,
    PlanAdmissionOptions, PlanCreateOptions, PlanEvolutionManifest, PlanEvolutionOptions,
    PlanEvolutionOutcome, PlanStatus, PlanTransitionOptions, PrimaryContainmentCreateOptions,
    RelationId, RelationVersionId, StoreInitOptions, WhyQueryOptions, WhyQueryTarget,
    WorkspaceInfo, WorkspaceInitOptions,
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
    let relation = format!(
        r#""expected_goal_plan_relation_id": "{relation_id}",
  "expected_goal_plan_relation_version_id": "{relation_version}","#,
        relation_id = "__RELATION_ID__",
        relation_version = "__RELATION_VERSION__"
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
  {relation}
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
    assert_eq!(result.goal_entity_id, admission.goal.entity_id);
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
    assert_eq!(result.goal_entity_id, admission.goal.entity_id);
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
            r#""expected_goal_plan_relation_id": "{}""#,
            expected_relation.relation_id
        ),
        &format!(
            r#""expected_goal_plan_relation_id": "{}""#,
            RelationId::new_v7()
        ),
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
  "schema_version": 1,
  "idempotency_key": "p0-supersede-orphan",
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
    "description": "Replacement Plan",
    "strategy": "Replacement Strategy",
    "constraints": {{"mode":"carry_all"}}
  }},
  "tasks": [],
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
