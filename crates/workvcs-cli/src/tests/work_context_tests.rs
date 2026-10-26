use super::*;

fn create_recovery_evidence(
    fixture: &ProjectBindingFixture,
    metadata: &str,
    session: Option<&str>,
) -> EvidenceId {
    let mut engine = Engine::open(&fixture.store).unwrap();
    let mut metadata: serde_json::Value = serde_json::from_str(metadata).unwrap();
    if let Some(scope) = metadata
        .get_mut("work_scope")
        .and_then(serde_json::Value::as_object_mut)
    {
        scope
            .entry("workspace_id")
            .or_insert_with(|| serde_json::json!(fixture.workspace_id));
        scope
            .entry("branch_id")
            .or_insert_with(|| serde_json::json!(fixture.branch));
    }
    let mut options = EvidenceCreateOptions::new(
        "recovery-validation",
        parse_canonical_json(&serde_json::to_vec(&metadata).unwrap()).unwrap(),
    )
    .unwrap();
    if let Some(session) = session {
        options = options.with_source_session_id(SessionId::parse_canonical(session).unwrap());
    }
    engine
        .create_evidence(options)
        .expect("persist actual Evidence")
        .evidence_id
}

#[test]
fn retrospective_evidence_is_scoped_before_budget_and_invalidates_changed_store_basis() {
    run_cli_test_with_large_stack(
        "work-context-evidence-scope",
        assert_evidence_scope_and_delta,
    );
}

fn assert_evidence_scope_and_delta() {
    let fixture = prepared_v2_fixture();
    let task_a = create_task(
        &fixture,
        &fixture.branch,
        &branch_head(&fixture, &fixture.branch),
        "Evidence Task A",
    );
    let task_a_id = value(&task_a, "task_entity_id");
    let task_b = create_task(
        &fixture,
        &fixture.branch,
        &branch_head(&fixture, &fixture.branch),
        "Evidence Task B",
    );
    let task_b_id = value(&task_b, "task_entity_id");
    let session_b = start_session(&fixture, &fixture.branch);
    run(Cli::try_parse_from([
        "workvcs",
        "session",
        "focus-set",
        &fixture.store,
        "--session",
        &session_b,
        "--focus",
        &task_b_id,
    ])
    .unwrap())
    .unwrap();
    create_recovery_evidence(
        &fixture,
        &format!(
            r#"{{"work_scope":{{"task_id":"{task_a_id}"}},"marker":"TASK_A_SCOPE_EVIDENCE"}}"#
        ),
        None,
    );
    create_recovery_evidence(
        &fixture,
        r#"{"marker":"UNRELATED_TASK_B_EVIDENCE"}"#,
        Some(&session_b),
    );
    create_recovery_evidence(
        &fixture,
        &format!(
            r#"{{"scope":{{"task_id":"{task_a_id}"}},"marker":"UNCLASSIFIED_SCOPE_EVIDENCE"}}"#
        ),
        None,
    );
    let selected = ["--task", task_a_id.as_str(), "--profile", "retrospective"];
    let baseline = run_recall(&fixture, &selected, 200).unwrap();
    assert_eq!(value(&baseline, "evidence_total"), "1");
    assert!(baseline.contains("TASK_A_SCOPE_EVIDENCE"));
    assert!(!baseline.contains("UNRELATED_TASK_B_EVIDENCE"));
    assert!(!baseline.contains("UNCLASSIFIED_SCOPE_EVIDENCE"));
    let head = value(&baseline, "head_commit_id");
    let digest = value(&baseline, "context_digest");
    let delta_args = [
        "--task",
        task_a_id.as_str(),
        "--profile",
        "retrospective",
        "--since-commit",
        head.as_str(),
        "--expected-context-digest",
        digest.as_str(),
    ];
    let unchanged = run_recall(&fixture, &delta_args, 200).unwrap();
    assert_eq!(value(&unchanged, "evidence_total"), "0");
    assert_eq!(value(&unchanged, "context_digest"), digest);
    create_recovery_evidence(
        &fixture,
        &format!(
            r#"{{"work_scope":{{"task_id":"{task_b_id}"}},"marker":"LATER_TASK_B_EVIDENCE"}}"#
        ),
        Some(&session_b),
    );
    let unrelated = run_recall(&fixture, &delta_args, 200).unwrap();
    assert_eq!(value(&unrelated, "evidence_total"), "0");
    assert_eq!(value(&unrelated, "context_digest"), digest);
    create_recovery_evidence(
        &fixture,
        &format!(
            r#"{{"work_scope":{{"task_id":"{task_a_id}"}},"marker":"LATER_TASK_A_EVIDENCE"}}"#
        ),
        None,
    );
    assert!(
        run_recall(&fixture, &delta_args, 200)
            .unwrap_err()
            .to_string()
            .contains("cached commit")
    );
    let fresh = run_recall(&fixture, &selected, 200).unwrap();
    assert_eq!(value(&fresh, "evidence_total"), "2");
    assert_ne!(value(&fresh, "context_digest"), digest);
    let overview = run_recall(&fixture, &["--profile", "retrospective"], 200).unwrap();
    assert!(!overview.contains("UNRELATED_TASK_B_EVIDENCE"));
    assert!(overview.contains("LATER_TASK_B_EVIDENCE"));
}

#[test]
fn routed_capture_preserves_evidence_scope_and_separates_forked_work_branches() {
    run_cli_test_with_large_stack("work-context-routed-evidence", assert_routed_evidence_scope);
}

fn assert_routed_evidence_scope() {
    let fixture = prepared_v2_fixture();
    let task = create_task(
        &fixture,
        &fixture.branch,
        &branch_head(&fixture, &fixture.branch),
        "Shared ancestor Task",
    );
    let task_id = value(&task, "task_entity_id");
    let other_branch = fork_branch(&fixture, &fixture.branch, "independent-evidence");
    let baseline = run_recall(
        &fixture,
        &["--task", &task_id, "--profile", "retrospective"],
        200,
    )
    .unwrap();
    let manifest = write_v2_capture_manifest(
        &fixture,
        "scoped-evidence.json",
        "scoped-evidence-key",
        "SCOPED_EVIDENCE_CAPTURE_RECORD",
    );
    let mut payload: serde_json::Value =
        serde_json::from_slice(&fs::read(&manifest).unwrap()).unwrap();
    payload["evidence"] = serde_json::json!([{ "local_id":"result-evidence", "kind":"validation", "metadata":{"marker":"ROUTED_TASK_EVIDENCE"}}]);
    fs::write(&manifest, serde_json::to_vec(&payload).unwrap()).unwrap();
    let capture_args = vec![
        "workvcs".to_owned(),
        "capture".to_owned(),
        "--cwd".to_owned(),
        fixture.project_text.clone(),
        "--registry".to_owned(),
        fixture.registry.clone(),
        "--branch".to_owned(),
        fixture.branch.clone(),
        "--task".to_owned(),
        task_id.clone(),
        "--value-reason".to_owned(),
        "Persist actual typed work Evidence".to_owned(),
        "--manifest".to_owned(),
        path_text(&manifest),
        "--deliver-existing-binding".to_owned(),
    ];
    let captured = run_owned(capture_args.clone()).expect("capture Task-scoped real Evidence");
    let delta = run_recall(
        &fixture,
        &[
            "--task",
            &task_id,
            "--profile",
            "retrospective",
            "--since-commit",
            &value(&baseline, "head_commit_id"),
            "--expected-context-digest",
            &value(&baseline, "context_digest"),
        ],
        200,
    )
    .unwrap();
    assert_eq!(value(&delta, "evidence_total"), "1");
    assert!(delta.contains("ROUTED_TASK_EVIDENCE"));
    assert!(delta.contains("SCOPED_EVIDENCE_CAPTURE_RECORD"));
    let first = run_recall(
        &fixture,
        &[
            "--branch",
            &fixture.branch,
            "--task",
            &task_id,
            "--profile",
            "retrospective",
        ],
        200,
    )
    .unwrap();
    assert!(first.contains("ROUTED_TASK_EVIDENCE"));
    assert_eq!(value(&first, "evidence_total"), "1");
    assert!(first.contains(&format!(r#""workspace_id":"{}""#, fixture.workspace_id)));
    assert!(first.contains(&format!(r#""branch_id":"{}""#, fixture.branch)));
    let replay = run_owned(capture_args).unwrap();
    assert_eq!(
        value(&captured, "target_commit_id"),
        value(&replay, "target_commit_id")
    );
    let inherited = fork_branch(&fixture, &fixture.branch, "inherited-evidence");
    let inherited_recall = run_recall(
        &fixture,
        &[
            "--branch",
            &inherited,
            "--task",
            &task_id,
            "--profile",
            "retrospective",
        ],
        200,
    )
    .unwrap();
    assert!(inherited_recall.contains("ROUTED_TASK_EVIDENCE"));
    for selection in [
        vec![
            "--branch",
            other_branch.as_str(),
            "--task",
            task_id.as_str(),
            "--profile",
            "retrospective",
        ],
        vec![
            "--branch",
            other_branch.as_str(),
            "--profile",
            "retrospective",
        ],
    ] {
        let other = run_recall(&fixture, &selection, 200).unwrap();
        assert_eq!(value(&other, "evidence_total"), "0");
        assert!(!other.contains("ROUTED_TASK_EVIDENCE"));
    }
    payload["idempotency_key"] = serde_json::json!("conflicting-evidence-owner-key");
    payload["evidence"][0]["metadata"]["work_scope"] =
        serde_json::json!({"branch_id":other_branch});
    fs::write(&manifest, serde_json::to_vec(&payload).unwrap()).unwrap();
    let before = branch_head(&fixture, &fixture.branch);
    let failure = run_owned(vec![
        "workvcs".to_owned(),
        "capture".to_owned(),
        "--cwd".to_owned(),
        fixture.project_text.clone(),
        "--registry".to_owned(),
        fixture.registry.clone(),
        "--task".to_owned(),
        task_id,
        "--value-reason".to_owned(),
        "Reject contradictory Evidence owner".to_owned(),
        "--manifest".to_owned(),
        path_text(&manifest),
    ])
    .unwrap_err();
    assert!(failure.to_string().contains("scope conflicts"));
    assert_eq!(branch_head(&fixture, &fixture.branch), before);
}

#[test]
fn scoped_retrospective_recovers_evidence_through_current_typed_verification() {
    run_cli_test_with_large_stack(
        "work-context-evidence-verification",
        assert_typed_evidence_citation,
    );
}

#[test]
fn default_capture_evidence_follows_commit_ancestry_and_excludes_another_workspace() {
    run_cli_test_with_large_stack(
        "work-context-default-evidence",
        assert_default_evidence_ancestry,
    );
}

fn assert_default_evidence_ancestry() {
    let fixture = prepared_v2_fixture();
    let before_branch = fork_branch(&fixture, &fixture.branch, "before-default-evidence");
    let manifest = write_v2_capture_manifest(
        &fixture,
        "default-evidence.json",
        "default-evidence-key",
        "DEFAULT_EVIDENCE_RECORD",
    );
    let mut payload: serde_json::Value =
        serde_json::from_slice(&fs::read(&manifest).unwrap()).unwrap();
    payload["evidence"] = serde_json::json!([{ "local_id":"default-proof", "kind":"validation", "metadata":{"marker":"DEFAULT_COMMITTED_EVIDENCE"}}]);
    fs::write(&manifest, serde_json::to_vec(&payload).unwrap()).unwrap();
    let args = vec![
        "workvcs".to_owned(),
        "capture".to_owned(),
        "--cwd".to_owned(),
        fixture.project_text.clone(),
        "--registry".to_owned(),
        fixture.registry.clone(),
        "--value-reason".to_owned(),
        "Persist actual default-Branch Evidence".to_owned(),
        "--manifest".to_owned(),
        path_text(&manifest),
        "--deliver-existing-binding".to_owned(),
    ];
    let first = run_owned(args.clone()).unwrap();
    let replay = run_owned(args).unwrap();
    assert_eq!(
        value(&first, "target_commit_id"),
        value(&replay, "target_commit_id")
    );
    let inherited_branch = fork_branch(&fixture, &fixture.branch, "after-default-evidence");
    let mut engine = Engine::open(&fixture.store).unwrap();
    let other = engine
        .create_workspace(WorkspaceInitOptions::new("Other real Workspace").unwrap())
        .unwrap();
    drop(engine);
    create_recovery_evidence(
        &fixture,
        &format!(
            r#"{{"work_scope":{{"workspace_id":"{}","branch_id":"{}","work":"other-project"}},"marker":"OTHER_WORKSPACE_EVIDENCE"}}"#,
            other.workspace_id, other.initial_branch_id
        ),
        None,
    );
    for branch in [&fixture.branch, &inherited_branch] {
        let recalled = run_recall(
            &fixture,
            &["--branch", branch, "--profile", "retrospective"],
            200,
        )
        .unwrap();
        assert!(recalled.contains("DEFAULT_COMMITTED_EVIDENCE"));
        assert!(!recalled.contains("OTHER_WORKSPACE_EVIDENCE"));
    }
    let before = run_recall(
        &fixture,
        &["--branch", &before_branch, "--profile", "retrospective"],
        200,
    )
    .unwrap();
    assert!(!before.contains("DEFAULT_COMMITTED_EVIDENCE"));
    assert!(!before.contains("OTHER_WORKSPACE_EVIDENCE"));
}

#[test]
fn routed_plan_admission_and_evolution_preserve_no_plan_path_scope() {
    run_cli_test_with_large_stack("work-context-plan-scope", assert_routed_plan_scope);
}

fn assert_routed_plan_scope() {
    let fixture = prepared_v2_fixture();
    let head = run(Cli::try_parse_from([
        "workvcs",
        "branch",
        "head",
        &fixture.store,
        "--branch",
        &fixture.branch,
    ])
    .unwrap())
    .unwrap();
    let mut payload: serde_json::Value = serde_json::from_str(&minimal_admit_manifest(
        &value(&head, "head_commit_id"),
        &value(&head, "state_digest"),
        "scoped-plan-admit-key",
    ))
    .unwrap();
    payload["records"] = serde_json::json!([{ "local_id":"scope-record", "kind":"finding", "statement":"SCOPED_PLAN_RECORD", "scope":{} }]);
    payload["evidence"] = serde_json::json!([{ "local_id":"scope-evidence", "kind":"validation", "metadata":{"marker":"SCOPED_PLAN_EVIDENCE"} }]);
    let admit_path = fixture._tempdir.path().join("scoped-plan-admit.json");
    fs::write(&admit_path, serde_json::to_vec(&payload).unwrap()).unwrap();
    let scope = r#"{"work":"scoped-plan-work","path":"src/x.rs"}"#;
    let before_admit = run_recall(
        &fixture,
        &["--scope-json", scope, "--profile", "retrospective"],
        200,
    )
    .unwrap();
    let admit_args = vec![
        "workvcs".to_owned(),
        "plan".to_owned(),
        "admit".to_owned(),
        "--cwd".to_owned(),
        fixture.project_text.clone(),
        "--registry".to_owned(),
        fixture.registry.clone(),
        "--scope-json".to_owned(),
        scope.to_owned(),
        "--manifest".to_owned(),
        path_text(&admit_path),
    ];
    let admitted = run_owned(admit_args.clone()).unwrap();
    let replay = run_owned(admit_args).unwrap();
    assert_eq!(value(&admitted, "commit_id"), value(&replay, "commit_id"));
    let admit_delta = run_recall(
        &fixture,
        &[
            "--scope-json",
            scope,
            "--profile",
            "retrospective",
            "--since-commit",
            &value(&before_admit, "head_commit_id"),
            "--expected-context-digest",
            &value(&before_admit, "context_digest"),
        ],
        200,
    )
    .unwrap();
    assert_eq!(value(&admit_delta, "evidence_total"), "1");
    assert!(admit_delta.contains("SCOPED_PLAN_EVIDENCE"));
    let recalled = run_recall(
        &fixture,
        &["--scope-json", scope, "--profile", "retrospective"],
        200,
    )
    .unwrap();
    assert!(recalled.contains("SCOPED_PLAN_RECORD"));
    assert!(recalled.contains("SCOPED_PLAN_EVIDENCE"));
    let before_evolve = recalled.clone();
    let unrelated = run_recall(
        &fixture,
        &[
            "--scope-json",
            r#"{"work":"other-plan-work","path":"src/x.rs"}"#,
            "--profile",
            "retrospective",
        ],
        200,
    )
    .unwrap();
    assert!(!unrelated.contains("SCOPED_PLAN_RECORD"));
    assert!(!unrelated.contains("SCOPED_PLAN_EVIDENCE"));
    let evolve_path = fixture._tempdir.path().join("scoped-plan-evolve.json");
    let mut evolve: serde_json::Value = serde_json::from_str(
        &supersede_manifest_for_cli_admission(&fixture, &admitted, "scoped-plan-evolve-key", 2),
    )
    .unwrap();
    evolve["records"] = serde_json::json!([{ "local_id":"scope-evolve-record", "kind":"finding", "statement":"SCOPED_EVOLVED_PLAN_RECORD", "scope":{} }]);
    evolve["evidence"] = serde_json::json!([{ "local_id":"scope-evolve-evidence", "kind":"validation", "metadata":{"marker":"SCOPED_EVOLVED_PLAN_EVIDENCE"} }]);
    fs::write(&evolve_path, serde_json::to_vec(&evolve).unwrap()).unwrap();
    let evolve_args = vec![
        "workvcs".to_owned(),
        "plan".to_owned(),
        "evolve".to_owned(),
        "--cwd".to_owned(),
        fixture.project_text.clone(),
        "--registry".to_owned(),
        fixture.registry.clone(),
        "--scope-json".to_owned(),
        scope.to_owned(),
        "--manifest".to_owned(),
        path_text(&evolve_path),
    ];
    let evolved = run_owned(evolve_args.clone()).unwrap();
    let replay = run_owned(evolve_args).unwrap();
    assert_eq!(value(&evolved, "commit_id"), value(&replay, "commit_id"));
    let evolve_delta = run_recall(
        &fixture,
        &[
            "--scope-json",
            scope,
            "--profile",
            "retrospective",
            "--since-commit",
            &value(&before_evolve, "head_commit_id"),
            "--expected-context-digest",
            &value(&before_evolve, "context_digest"),
        ],
        200,
    )
    .unwrap();
    assert_eq!(value(&evolve_delta, "evidence_total"), "1");
    assert!(evolve_delta.contains("SCOPED_EVOLVED_PLAN_EVIDENCE"));
    assert!(!evolve_delta.contains("\"marker\":\"SCOPED_PLAN_EVIDENCE\""));
    let recalled = run_recall(
        &fixture,
        &["--scope-json", scope, "--profile", "retrospective"],
        200,
    )
    .unwrap();
    assert!(recalled.contains("SCOPED_EVOLVED_PLAN_RECORD"));
    assert!(recalled.contains("SCOPED_EVOLVED_PLAN_EVIDENCE"));
    let wrong_path = run_recall(
        &fixture,
        &[
            "--scope-json",
            r#"{"work":"scoped-plan-work","path":"src/y.rs"}"#,
            "--profile",
            "retrospective",
        ],
        200,
    )
    .unwrap();
    assert!(!wrong_path.contains("SCOPED_PLAN_EVIDENCE"));
    assert!(!wrong_path.contains("SCOPED_EVOLVED_PLAN_EVIDENCE"));
}

fn assert_typed_evidence_citation() {
    let fixture = prepared_v2_fixture();
    let task = create_task(
        &fixture,
        &fixture.branch,
        &branch_head(&fixture, &fixture.branch),
        "Typed citation Task",
    );
    let task_id = EntityId::parse_canonical(&value(&task, "task_entity_id")).unwrap();
    let evidence_id = create_recovery_evidence(
        &fixture,
        r#"{"marker":"CURRENT_TYPED_CITATION_EVIDENCE"}"#,
        None,
    );
    let mut engine = Engine::open(&fixture.store).unwrap();
    let branch = BranchId::parse_canonical(&fixture.branch).unwrap();
    let head = engine.branch_head(branch).unwrap().head_commit_id;
    let task = engine.task_at(head, task_id).unwrap();
    let criterion = engine
        .create_acceptance_criterion(
            AcceptanceCriterionCreateOptions::new(
                branch,
                head,
                task_id,
                task.task_entity_version_id,
                "result",
                "Typed citation evidence is recoverable",
                AcceptanceCriterionClassification::Required,
            )
            .unwrap(),
        )
        .unwrap();
    engine
        .create_verification(
            VerificationCreateOptions::new(
                branch,
                criterion.commit_id,
                VerificationTarget::AcceptanceCriterion(criterion.acceptance_criterion_entity_id),
                VerificationResult::Passed,
            )
            .unwrap()
            .with_evidence([evidence_id])
            .unwrap(),
        )
        .unwrap();
    drop(engine);
    let id = task_id.to_string();
    let selected = ["--task", id.as_str(), "--profile", "retrospective"];
    let baseline = run_recall(&fixture, &selected, 200).unwrap();
    assert_eq!(value(&baseline, "evidence_total"), "1");
    assert!(baseline.contains("CURRENT_TYPED_CITATION_EVIDENCE"));
    let head = value(&baseline, "head_commit_id");
    let digest = value(&baseline, "context_digest");
    let unchanged = run_recall(
        &fixture,
        &[
            "--task",
            &id,
            "--profile",
            "retrospective",
            "--since-commit",
            &head,
            "--expected-context-digest",
            &digest,
        ],
        200,
    )
    .unwrap();
    assert_eq!(value(&unchanged, "evidence_total"), "0");
    let task_scoped_path = format!(r#"{{"task_id":"{id}","path":"src/x.rs"}}"#);
    let path_selected = run_recall(
        &fixture,
        &[
            "--scope-json",
            &task_scoped_path,
            "--profile",
            "retrospective",
        ],
        200,
    )
    .unwrap();
    assert_eq!(
        value(&path_selected, "evidence_total"),
        "0",
        "a Task citation does not prove a file path association"
    );
}

fn prepared_v2_fixture() -> ProjectBindingFixture {
    let fixture = create_project_binding_fixture(false);
    migrate_fixture_registry_to_v2(&fixture, None);
    activate_fixture_v2_durable_operations(&fixture);
    fixture
}

fn run_owned(args: Vec<String>) -> Result<String> {
    run(Cli::try_parse_from(args).expect("parse CLI arguments"))
}

fn fork_branch(fixture: &ProjectBindingFixture, source: &str, name: &str) -> String {
    let output = run(Cli::try_parse_from([
        "workvcs",
        "branch",
        "fork",
        &fixture.store,
        "--from-branch",
        source,
        "--name",
        name,
    ])
    .expect("parse Branch fork"))
    .expect("fork Branch");
    value(&output, "branch_id")
}

fn start_session(fixture: &ProjectBindingFixture, branch: &str) -> String {
    let output = run(Cli::try_parse_from([
        "workvcs",
        "session",
        "start",
        &fixture.store,
        "--workspace",
        &fixture.workspace_id,
        "--branch",
        branch,
    ])
    .expect("parse Session start"))
    .expect("start Session");
    value(&output, "session_id")
}

fn branch_head(fixture: &ProjectBindingFixture, branch: &str) -> String {
    let output = run(Cli::try_parse_from([
        "workvcs",
        "branch",
        "head",
        &fixture.store,
        "--branch",
        branch,
    ])
    .expect("parse Branch head"))
    .expect("read Branch head");
    value(&output, "head_commit_id")
}

fn create_task(
    fixture: &ProjectBindingFixture,
    branch: &str,
    head: &str,
    description: &str,
) -> String {
    run(Cli::try_parse_from([
        "workvcs",
        "task",
        "create",
        &fixture.store,
        "--branch",
        branch,
        "--head",
        head,
        "--description",
        description,
    ])
    .expect("parse Task create"))
    .expect("create Task")
}

fn create_finding(
    fixture: &ProjectBindingFixture,
    branch: &str,
    head: &str,
    statement: &str,
    scope_json: &str,
) -> String {
    run(Cli::try_parse_from([
        "workvcs",
        "record",
        "finding",
        &fixture.store,
        "--branch",
        branch,
        "--head",
        head,
        "--statement",
        statement,
        "--scope-json",
        scope_json,
    ])
    .expect("parse Finding create"))
    .expect("create Finding")
}

fn create_knowledge(
    fixture: &ProjectBindingFixture,
    head: &str,
    statement: &str,
    scope: &str,
) -> String {
    run(Cli::try_parse_from([
        "workvcs",
        "knowledge",
        "create",
        &fixture.store,
        "--branch",
        &fixture.branch,
        "--head",
        head,
        "--statement",
        statement,
        "--scope-json",
        scope,
    ])
    .unwrap())
    .expect("create actual scoped Knowledge")
}

fn run_recall(
    fixture: &ProjectBindingFixture,
    selection_args: &[&str],
    budget: usize,
) -> Result<String> {
    let mut args = vec![
        "workvcs".to_owned(),
        "recall".to_owned(),
        "--cwd".to_owned(),
        fixture.project_text.clone(),
        "--registry".to_owned(),
        fixture.registry.clone(),
        "--budget-items".to_owned(),
        budget.to_string(),
    ];
    args.extend(selection_args.iter().map(|value| (*value).to_owned()));
    run_owned(args)
}

fn run_session_capture(
    fixture: &ProjectBindingFixture,
    session: &str,
    manifest: &Path,
    deliver: bool,
) -> Result<String> {
    let mut args = vec![
        "workvcs".to_owned(),
        "capture".to_owned(),
        "--cwd".to_owned(),
        fixture.project_text.clone(),
        "--registry".to_owned(),
        fixture.registry.clone(),
        "--session".to_owned(),
        session.to_owned(),
        "--value-reason".to_owned(),
        "Persist selected Session work".to_owned(),
        "--manifest".to_owned(),
        path_text(manifest),
    ];
    if deliver {
        args.push("--deliver-existing-binding".to_owned());
    }
    run_owned(args)
}

#[test]
fn compound_path_scope_is_preserved_across_recall_native_context_and_capture() {
    run_cli_test_with_large_stack("work-context-compound-path", assert_compound_path_scope);
}

fn assert_compound_path_scope() {
    let fixture = prepared_v2_fixture();
    let task_a = create_task(
        &fixture,
        &fixture.branch,
        &fixture.genesis_commit_id,
        "Path selected Task A",
    );
    let task_a_id = value(&task_a, "task_entity_id");
    let task_b = create_task(
        &fixture,
        &fixture.branch,
        &value(&task_a, "commit_id"),
        "Path unrelated Task B",
    );
    let task_b_id = value(&task_b, "task_entity_id");
    let mut head = value(&task_b, "commit_id");
    for (statement, scope) in [
        (
            "SHARED_PATH_X_KNOWLEDGE",
            r#"{"path":"src/x.rs"}"#.to_owned(),
        ),
        (
            "SHARED_PATH_Y_KNOWLEDGE",
            r#"{"path":"src/y.rs"}"#.to_owned(),
        ),
        (
            "TASK_A_PREFIX_X_KNOWLEDGE",
            format!(r#"{{"task_id":"{task_a_id}","path_prefix":"src/x.rs"}}"#),
        ),
        (
            "TASK_A_PATH_Y_KNOWLEDGE",
            format!(r#"{{"task_id":"{task_a_id}","path":"src/y.rs"}}"#),
        ),
        (
            "TASK_B_PATH_X_KNOWLEDGE",
            format!(r#"{{"task_id":"{task_b_id}","path":"src/x.rs"}}"#),
        ),
        (
            "TASK_A_NESTED_PATH_X_KNOWLEDGE",
            format!(r#"{{"task_id":"{task_a_id}","scope_payload":{{"path":"src/x.rs"}}}}"#),
        ),
        (
            "TASK_A_NESTED_PATH_Y_KNOWLEDGE",
            format!(r#"{{"task_id":"{task_a_id}","scope_payload":{{"path":"src/y.rs"}}}}"#),
        ),
    ] {
        head = value(
            &create_knowledge(&fixture, &head, statement, &scope),
            "commit_id",
        );
    }
    for (statement, task, path) in [
        ("TASK_A_PATH_X_RECORD", &task_a_id, "src/x.rs"),
        ("TASK_A_PATH_Y_RECORD", &task_a_id, "src/y.rs"),
        ("TASK_B_PATH_X_RECORD", &task_b_id, "src/x.rs"),
    ] {
        head = value(
            &create_finding(
                &fixture,
                &fixture.branch,
                &head,
                statement,
                &format!(r#"{{"task_id":"{task}","path":"{path}"}}"#),
            ),
            "commit_id",
        );
    }
    let session = start_session(&fixture, &fixture.branch);
    run(Cli::try_parse_from([
        "workvcs",
        "session",
        "focus-set",
        &fixture.store,
        "--session",
        &session,
        "--focus",
        &task_a_id,
    ])
    .unwrap())
    .expect("focus Task A");
    let path_scope = r#"{"path":"src/x.rs"}"#;
    let recalled = run_recall(
        &fixture,
        &["--task", &task_a_id, "--scope-json", path_scope],
        30,
    )
    .expect("compound Task/path Recall");
    let context = run(Cli::try_parse_from([
        "workvcs",
        "context",
        &fixture.store,
        "--session",
        &session,
        "--scope-json",
        path_scope,
        "--profile",
        "brief",
        "--budget-items",
        "30",
    ])
    .unwrap())
    .expect("compound Focus/path Context");
    for output in [&recalled, &context] {
        for selected in [
            "SHARED_PATH_X_KNOWLEDGE",
            "TASK_A_PREFIX_X_KNOWLEDGE",
            "TASK_A_NESTED_PATH_X_KNOWLEDGE",
            "TASK_A_PATH_X_RECORD",
        ] {
            assert!(
                output.contains(selected),
                "selected path item missing: {selected}"
            );
        }
        for unrelated in [
            "SHARED_PATH_Y_KNOWLEDGE",
            "TASK_A_PATH_Y_KNOWLEDGE",
            "TASK_A_NESTED_PATH_Y_KNOWLEDGE",
            "TASK_B_PATH_X_KNOWLEDGE",
            "TASK_A_PATH_Y_RECORD",
            "TASK_B_PATH_X_RECORD",
        ] {
            assert!(
                !output.contains(unrelated),
                "unrelated path item leaked: {unrelated}"
            );
        }
    }
    let nested_scope = r#"{"scope_payload":{"path":"src/x.rs"}}"#;
    let nested_recall = run_recall(
        &fixture,
        &["--task", &task_a_id, "--scope-json", nested_scope],
        30,
    )
    .expect("nested path selector Recall");
    let nested_context = run(Cli::try_parse_from([
        "workvcs",
        "context",
        &fixture.store,
        "--session",
        &session,
        "--scope-json",
        nested_scope,
        "--profile",
        "brief",
        "--budget-items",
        "30",
    ])
    .unwrap())
    .expect("nested path selector Context");
    for output in [&nested_recall, &nested_context] {
        for selected in [
            "SHARED_PATH_X_KNOWLEDGE",
            "TASK_A_PREFIX_X_KNOWLEDGE",
            "TASK_A_NESTED_PATH_X_KNOWLEDGE",
        ] {
            assert!(output.contains(selected), "nested selector lost {selected}");
        }
        for unrelated in [
            "SHARED_PATH_Y_KNOWLEDGE",
            "TASK_A_PATH_Y_KNOWLEDGE",
            "TASK_A_NESTED_PATH_Y_KNOWLEDGE",
            "TASK_B_PATH_X_KNOWLEDGE",
        ] {
            assert!(
                !output.contains(unrelated),
                "nested selector leaked {unrelated}"
            );
        }
    }
    let overview_recall = run_recall(&fixture, &["--scope-json", path_scope], 30)
        .expect("path-only Project overview");
    let overview_session = start_session(&fixture, &fixture.branch);
    let overview_context = run(Cli::try_parse_from([
        "workvcs",
        "context",
        &fixture.store,
        "--session",
        &overview_session,
        "--scope-json",
        path_scope,
        "--profile",
        "normal",
        "--budget-items",
        "30",
    ])
    .unwrap())
    .expect("path-only Context without Task Focus");
    for output in [&overview_recall, &overview_context] {
        assert!(output.contains("TASK_A_PATH_X_RECORD"));
        assert!(output.contains("TASK_B_PATH_X_RECORD"));
        assert!(!output.contains("TASK_A_PATH_Y_RECORD"));
        assert!(!output.contains("SHARED_PATH_Y_KNOWLEDGE"));
    }
    assert_eq!(value(&overview_recall, "tasks_total"), "2");
    assert_eq!(value(&overview_recall, "selected_focus_entity_id"), "none");
    assert!(
        run_recall(&fixture, &["--scope-json", r#"{"paths":[]}"#], 30)
            .expect_err("an empty file selector must fail at input")
            .to_string()
            .contains("nonempty path strings")
    );
    let manifest = write_v2_capture_manifest(
        &fixture,
        "compound-capture.json",
        "compound-capture-key",
        "COMPOUND_SCOPE_CAPTURE_RECORD",
    );
    let captured = run(Cli::try_parse_from([
        "workvcs",
        "capture",
        "--cwd",
        &fixture.project_text,
        "--registry",
        &fixture.registry,
        "--task",
        &task_a_id,
        "--scope-json",
        path_scope,
        "--value-reason",
        "Persist actual compound work selection",
        "--manifest",
        &path_text(&manifest),
        "--deliver-existing-binding",
    ])
    .unwrap())
    .expect("compound-scope capture");
    let engine = Engine::open_readonly(&fixture.store).unwrap();
    let records = engine
        .records_at(RecordListOptions::new(
            CommitId::parse_canonical(&value(&captured, "target_commit_id")).unwrap(),
        ))
        .unwrap();
    let record = records
        .records
        .iter()
        .find(|record| record.state.statement == "COMPOUND_SCOPE_CAPTURE_RECORD")
        .expect("captured scoped record");
    assert_eq!(
        canonical_cli_json("captured compound scope", &record.state.scope).unwrap(),
        format!(r#"{{"path":"src/x.rs","task_id":"{task_a_id}"}}"#)
    );
}

#[test]
fn task_and_no_plan_recall_filter_before_budget_and_keep_only_shared_project_facts() {
    run_cli_test_with_large_stack(
        "work-context-filter-before-budget",
        assert_task_and_no_plan_recall_filter_before_budget_and_keep_only_shared_project_facts,
    );
}

fn assert_task_and_no_plan_recall_filter_before_budget_and_keep_only_shared_project_facts() {
    let fixture = prepared_v2_fixture();
    let task_a = create_task(
        &fixture,
        &fixture.branch,
        &fixture.genesis_commit_id,
        "Selected task A",
    );
    let task_a_id = value(&task_a, "task_entity_id");
    let task_b = create_task(
        &fixture,
        &fixture.branch,
        &value(&task_a, "commit_id"),
        "Unrelated task B",
    );
    let task_b_id = value(&task_b, "task_entity_id");
    let criterion_b = run(Cli::try_parse_from([
        "workvcs",
        "ac",
        "create",
        &fixture.store,
        "--branch",
        &fixture.branch,
        "--head",
        &value(&task_b, "commit_id"),
        "--task",
        &task_b_id,
        "--task-version",
        &value(&task_b, "task_entity_version_id"),
        "--local-key",
        "task-b-acceptance",
        "--statement",
        "UNRELATED_TASK_B_ACCEPTANCE_MARKER",
    ])
    .unwrap())
    .expect("create actual Task B acceptance criterion");
    let requirement_b = run(Cli::try_parse_from([
        "workvcs",
        "vr",
        "create",
        &fixture.store,
        "--branch",
        &fixture.branch,
        "--head",
        &value(&criterion_b, "commit_id"),
        "--criterion",
        &value(&criterion_b, "acceptance_criterion_entity_id"),
        "--criterion-version",
        &value(&criterion_b, "acceptance_criterion_entity_version_id"),
        "--local-key",
        "task-b-verification",
        "--statement",
        "UNRELATED_TASK_B_VERIFICATION_MARKER",
    ])
    .unwrap())
    .expect("create actual Task B verification requirement");
    let task_a_scope = format!(r#"{{"task_id":"{task_a_id}"}}"#);
    let task_b_scope = format!(r#"{{"task_id":"{task_b_id}"}}"#);
    let selected_record = create_finding(
        &fixture,
        &fixture.branch,
        &value(&requirement_b, "commit_id"),
        "SELECTED_TASK_RECORD_MARKER",
        &task_a_scope,
    );
    let project_fact = create_finding(
        &fixture,
        &fixture.branch,
        &value(&selected_record, "commit_id"),
        "SHARED_PROJECT_FACT_MARKER",
        r#"{"applicability":"project_fact"}"#,
    );
    let mut head = value(&project_fact, "commit_id");
    for index in 0..5 {
        let unrelated = create_finding(
            &fixture,
            &fixture.branch,
            &head,
            &format!("UNRELATED_TASK_RECORD_MARKER_{index}"),
            &task_b_scope,
        );
        head = value(&unrelated, "commit_id");
    }

    let other_session = start_session(&fixture, &fixture.branch);
    run(Cli::try_parse_from([
        "workvcs",
        "session",
        "focus-set",
        &fixture.store,
        "--session",
        &other_session,
        "--focus",
        &task_b_id,
    ])
    .expect("parse unrelated Session focus"))
    .expect("focus unrelated Task");
    run(Cli::try_parse_from([
        "workvcs",
        "claim",
        "task",
        &fixture.store,
        "--session",
        &other_session,
        "--task",
        &task_b_id,
    ])
    .expect("parse unrelated Task claim"))
    .expect("claim unrelated Task");

    let selected_session = start_session(&fixture, &fixture.branch);
    run(Cli::try_parse_from([
        "workvcs",
        "session",
        "focus-set",
        &fixture.store,
        "--session",
        &selected_session,
        "--focus",
        &task_a_id,
    ])
    .expect("parse selected Session focus"))
    .expect("focus selected Task");
    run(Cli::try_parse_from([
        "workvcs",
        "claim",
        "task",
        &fixture.store,
        "--session",
        &selected_session,
        "--task",
        &task_a_id,
    ])
    .expect("parse selected Task claim"))
    .expect("claim selected Task");

    let task_recall =
        run_recall(&fixture, &["--task", &task_a_id], 5).expect("recall selected Task");
    assert_eq!(value(&task_recall, "tasks_total"), "1");
    assert_eq!(value(&task_recall, "sessions_total"), "1");
    assert_eq!(value(&task_recall, "records_total"), "2");
    assert_eq!(value(&task_recall, "claims_total"), "1");
    assert_eq!(value(&task_recall, "returned_items"), "5");
    assert!(task_recall.contains("SELECTED_TASK_RECORD_MARKER"));
    assert!(task_recall.contains("SHARED_PROJECT_FACT_MARKER"));
    assert!(!task_recall.contains("UNRELATED_TASK_RECORD_MARKER"));
    assert!(!task_recall.contains(&task_b_id));
    assert!(task_recall.contains(&selected_session));
    assert!(!task_recall.contains(&other_session));
    assert!(task_recall.contains("applicability=selected_work"));
    assert!(task_recall.contains("applicability=project_fact"));
    assert!(task_recall.contains("action_authority=none"));

    let conflicting_recall = run_recall(
        &fixture,
        &[
            "--session",
            &selected_session,
            "--scope-json",
            &task_b_scope,
        ],
        30,
    )
    .expect_err("scope Task must agree with Session Focus before any projection");
    assert!(
        conflicting_recall
            .to_string()
            .contains("conflicts with Session Focus")
    );
    let conflicting_native = run(Cli::try_parse_from([
        "workvcs",
        "context",
        &fixture.store,
        "--session",
        &selected_session,
        "--scope-json",
        &task_b_scope,
    ])
    .unwrap())
    .expect_err("native context must reject Task/Focus conflict");
    assert!(
        conflicting_native
            .to_string()
            .contains("conflicts with Session Focus")
    );
    let conflicting_resume = run(Cli::try_parse_from([
        "workvcs",
        "resume",
        "--cwd",
        &fixture.project_text,
        "--registry",
        &fixture.registry,
        "--session",
        &selected_session,
        "--scope-json",
        &task_b_scope,
    ])
    .unwrap())
    .expect_err("resume must reject Task/Focus conflict");
    assert!(
        conflicting_resume
            .to_string()
            .contains("conflicts with Session Focus")
    );

    let no_plan_a = create_finding(
        &fixture,
        &fixture.branch,
        &head,
        "NO_PLAN_A_RECORD_MARKER",
        r#"{"work":"no-plan-a"}"#,
    );
    let no_plan_b = create_finding(
        &fixture,
        &fixture.branch,
        &value(&no_plan_a, "commit_id"),
        "NO_PLAN_B_RECORD_MARKER",
        r#"{"work":"no-plan-b"}"#,
    );
    let later_fact = create_finding(
        &fixture,
        &fixture.branch,
        &value(&no_plan_b, "commit_id"),
        "LATER_SHARED_PROJECT_FACT_MARKER",
        r#"{"applicability":"project_fact"}"#,
    );
    assert_eq!(
        branch_head(&fixture, &fixture.branch),
        value(&later_fact, "commit_id")
    );
    let no_plan_recall = run_recall(&fixture, &["--scope-json", r#"{"work":"no-plan-a"}"#], 3)
        .expect("recall selected No-Plan work");
    assert_eq!(value(&no_plan_recall, "records_total"), "3");
    assert!(no_plan_recall.contains("NO_PLAN_A_RECORD_MARKER"));
    assert!(no_plan_recall.contains("SHARED_PROJECT_FACT_MARKER"));
    assert!(no_plan_recall.contains("LATER_SHARED_PROJECT_FACT_MARKER"));
    assert!(!no_plan_recall.contains("NO_PLAN_B_RECORD_MARKER"));

    let focused_session = start_session(&fixture, &fixture.branch);
    run(Cli::try_parse_from([
        "workvcs",
        "session",
        "focus-set",
        &fixture.store,
        "--session",
        &focused_session,
        "--focus",
        &task_a_id,
    ])
    .unwrap())
    .expect("focus selected Task");
    let focused_context = run(Cli::try_parse_from([
        "workvcs",
        "context",
        &fixture.store,
        "--session",
        &focused_session,
        "--profile",
        "brief",
        "--budget-items",
        "30",
    ])
    .unwrap())
    .expect("collect focused native context");
    assert_eq!(value(&focused_context, "focus_entity_id"), task_a_id);
    assert!(focused_context.contains("SELECTED_TASK_RECORD_MARKER"));
    assert!(focused_context.contains("SHARED_PROJECT_FACT_MARKER"));
    assert!(!focused_context.contains("UNRELATED_TASK_RECORD_MARKER"));

    let focused_resume = run(Cli::try_parse_from([
        "workvcs",
        "resume",
        "--cwd",
        &fixture.project_text,
        "--registry",
        &fixture.registry,
        "--session",
        &focused_session,
        "--budget-items",
        "30",
    ])
    .unwrap())
    .expect("resume focused Project Session");
    assert_eq!(value(&focused_resume, "focus_entity_id"), task_a_id);
    assert!(focused_resume.contains("SELECTED_TASK_RECORD_MARKER"));
    assert!(focused_resume.contains("SHARED_PROJECT_FACT_MARKER"));
    assert!(!focused_resume.contains("UNRELATED_TASK_RECORD_MARKER"));

    let scoped_context = run(Cli::try_parse_from([
        "workvcs",
        "context",
        &fixture.store,
        "--session",
        &other_session,
        "--profile",
        "brief",
        "--budget-items",
        "30",
        "--scope-json",
        r#"{"work":"no-plan-a"}"#,
    ])
    .unwrap())
    .expect("collect explicitly scoped native context");
    assert!(scoped_context.contains("NO_PLAN_A_RECORD_MARKER"));
    assert!(scoped_context.contains("SHARED_PROJECT_FACT_MARKER"));
    assert!(scoped_context.contains("LATER_SHARED_PROJECT_FACT_MARKER"));
    assert!(!scoped_context.contains("NO_PLAN_B_RECORD_MARKER"));
    for unrelated in [
        "Unrelated task B",
        "UNRELATED_TASK_B_ACCEPTANCE_MARKER",
        "UNRELATED_TASK_B_VERIFICATION_MARKER",
        "category=current_task",
        "category=task_readiness",
        "category=acceptance_criterion",
        "category=verification_requirement",
        "category=blocked_dependency",
    ] {
        assert!(
            !scoped_context.contains(unrelated),
            "No-Plan context leaked {unrelated}"
        );
    }
    assert!(scoped_context.contains("applicability=project_fact action_authority=none"));

    let scoped_resume = run(Cli::try_parse_from([
        "workvcs",
        "resume",
        "--cwd",
        &fixture.project_text,
        "--registry",
        &fixture.registry,
        "--session",
        &other_session,
        "--scope-json",
        r#"{"work":"no-plan-a"}"#,
        "--budget-items",
        "30",
    ])
    .unwrap())
    .expect("resume explicitly scoped Project Session");
    assert!(scoped_resume.contains("NO_PLAN_A_RECORD_MARKER"));
    assert!(scoped_resume.contains("SHARED_PROJECT_FACT_MARKER"));
    assert!(scoped_resume.contains("LATER_SHARED_PROJECT_FACT_MARKER"));
    assert!(!scoped_resume.contains("NO_PLAN_B_RECORD_MARKER"));
    for unrelated in [
        "Unrelated task B",
        "UNRELATED_TASK_B_ACCEPTANCE_MARKER",
        "UNRELATED_TASK_B_VERIFICATION_MARKER",
        "category=current_task",
        "category=task_readiness",
        "category=acceptance_criterion",
        "category=verification_requirement",
        "category=blocked_dependency",
    ] {
        assert!(
            !scoped_resume.contains(unrelated),
            "No-Plan resume leaked {unrelated}"
        );
    }
    let scoped_recall = run_recall(
        &fixture,
        &[
            "--session",
            &other_session,
            "--scope-json",
            r#"{"work":"no-plan-a"}"#,
        ],
        30,
    )
    .expect("explicit No-Plan Session recall");
    assert_eq!(value(&scoped_recall, "tasks_total"), "0");
    assert_eq!(value(&scoped_recall, "claims_total"), "0");
    assert_eq!(value(&scoped_recall, "selected_focus_entity_id"), "none");
    assert_eq!(value(&scoped_recall, "runtime_focus_entity_id"), task_b_id);
    assert!(!scoped_recall.contains("Unrelated task B"));
    let task_b_context = run(Cli::try_parse_from([
        "workvcs",
        "context",
        &fixture.store,
        "--session",
        &other_session,
        "--profile",
        "brief",
        "--budget-items",
        "30",
    ])
    .unwrap())
    .expect("Task B positive control");
    assert!(task_b_context.contains("UNRELATED_TASK_B_ACCEPTANCE_MARKER"));
    assert!(task_b_context.contains("UNRELATED_TASK_B_VERIFICATION_MARKER"));
    assert!(task_b_context.contains("category=task_readiness"));
}

#[test]
fn context_and_resume_filter_by_session_focus_and_explicit_scope() {
    run_cli_test_with_large_stack(
        "work-context-native-focus-filter",
        assert_context_and_resume_filter_by_session_focus_and_explicit_scope,
    );
}

fn assert_context_and_resume_filter_by_session_focus_and_explicit_scope() {
    let fixture = prepared_v2_fixture();
    let task_a = create_task(
        &fixture,
        &fixture.branch,
        &fixture.genesis_commit_id,
        "Focused Task A",
    );
    let task_a_id = value(&task_a, "task_entity_id");
    let task_b = create_task(
        &fixture,
        &fixture.branch,
        &value(&task_a, "commit_id"),
        "Unrelated Task B",
    );
    let task_b_id = value(&task_b, "task_entity_id");
    let focused = create_finding(
        &fixture,
        &fixture.branch,
        &value(&task_b, "commit_id"),
        "FOCUSED_CONTEXT_MARKER",
        &format!(r#"{{"task_id":"{task_a_id}"}}"#),
    );
    let unrelated = create_finding(
        &fixture,
        &fixture.branch,
        &value(&focused, "commit_id"),
        "UNRELATED_CONTEXT_MARKER",
        &format!(r#"{{"task_id":"{task_b_id}"}}"#),
    );
    let scoped = create_finding(
        &fixture,
        &fixture.branch,
        &value(&unrelated, "commit_id"),
        "EXPLICIT_SCOPE_CONTEXT_MARKER",
        r#"{"work":"native-scope-a"}"#,
    );
    let wrong_scope = create_finding(
        &fixture,
        &fixture.branch,
        &value(&scoped, "commit_id"),
        "WRONG_SCOPE_CONTEXT_MARKER",
        r#"{"work":"native-scope-b"}"#,
    );
    let project_fact = create_finding(
        &fixture,
        &fixture.branch,
        &value(&wrong_scope, "commit_id"),
        "CONTEXT_PROJECT_FACT_MARKER",
        r#"{"applicability":"project_fact"}"#,
    );
    assert_eq!(
        branch_head(&fixture, &fixture.branch),
        value(&project_fact, "commit_id")
    );

    let focused_session = start_session(&fixture, &fixture.branch);
    run(Cli::try_parse_from([
        "workvcs",
        "session",
        "focus-set",
        &fixture.store,
        "--session",
        &focused_session,
        "--focus",
        &task_a_id,
    ])
    .unwrap())
    .expect("focus Task A");
    let focused_context = run(Cli::try_parse_from([
        "workvcs",
        "context",
        &fixture.store,
        "--session",
        &focused_session,
        "--profile",
        "brief",
        "--budget-items",
        "30",
    ])
    .unwrap())
    .expect("focused context");
    assert_eq!(value(&focused_context, "focus_entity_id"), task_a_id);
    assert!(focused_context.contains("FOCUSED_CONTEXT_MARKER"));
    assert!(focused_context.contains("CONTEXT_PROJECT_FACT_MARKER"));
    assert!(!focused_context.contains("UNRELATED_CONTEXT_MARKER"));

    let focused_resume = run(Cli::try_parse_from([
        "workvcs",
        "resume",
        "--cwd",
        &fixture.project_text,
        "--registry",
        &fixture.registry,
        "--session",
        &focused_session,
        "--budget-items",
        "30",
    ])
    .unwrap())
    .expect("focused resume");
    assert_eq!(value(&focused_resume, "focus_entity_id"), task_a_id);
    assert!(focused_resume.contains("FOCUSED_CONTEXT_MARKER"));
    assert!(focused_resume.contains("CONTEXT_PROJECT_FACT_MARKER"));
    assert!(!focused_resume.contains("UNRELATED_CONTEXT_MARKER"));

    let scoped_session = start_session(&fixture, &fixture.branch);
    let scoped_context = run(Cli::try_parse_from([
        "workvcs",
        "context",
        &fixture.store,
        "--session",
        &scoped_session,
        "--profile",
        "brief",
        "--budget-items",
        "30",
        "--scope-json",
        r#"{"work":"native-scope-a"}"#,
    ])
    .unwrap())
    .expect("explicitly scoped context");
    assert!(scoped_context.contains("EXPLICIT_SCOPE_CONTEXT_MARKER"));
    assert!(scoped_context.contains("CONTEXT_PROJECT_FACT_MARKER"));
    assert!(!scoped_context.contains("WRONG_SCOPE_CONTEXT_MARKER"));

    let scoped_resume = run(Cli::try_parse_from([
        "workvcs",
        "resume",
        "--cwd",
        &fixture.project_text,
        "--registry",
        &fixture.registry,
        "--session",
        &scoped_session,
        "--scope-json",
        r#"{"work":"native-scope-a"}"#,
        "--budget-items",
        "30",
    ])
    .unwrap())
    .expect("explicitly scoped resume");
    assert!(scoped_resume.contains("EXPLICIT_SCOPE_CONTEXT_MARKER"));
    assert!(scoped_resume.contains("CONTEXT_PROJECT_FACT_MARKER"));
    assert!(!scoped_resume.contains("WRONG_SCOPE_CONTEXT_MARKER"));
}

#[test]
fn session_selected_branch_routes_plan_capture_recall_and_resume_then_rejects_cross_context_reuse()
{
    run_cli_test_with_large_stack(
        "work-context-session-branch-routing",
        assert_session_selected_branch_routes_plan_capture_recall_and_resume_then_rejects_cross_context_reuse,
    );
}

fn assert_session_selected_branch_routes_plan_capture_recall_and_resume_then_rejects_cross_context_reuse()
 {
    let fixture = prepared_v2_fixture();
    let default_head_before = branch_head(&fixture, &fixture.branch);
    let work_branch = fork_branch(&fixture, &fixture.branch, "session-work");
    let work_head = run(Cli::try_parse_from([
        "workvcs",
        "branch",
        "head",
        &fixture.store,
        "--branch",
        &work_branch,
    ])
    .unwrap())
    .unwrap();
    let session = start_session(&fixture, &work_branch);

    let admit_path = fixture._tempdir.path().join("session-plan-admit.json");
    fs::write(
        &admit_path,
        minimal_admit_manifest(
            &value(&work_head, "head_commit_id"),
            &value(&work_head, "state_digest"),
            "session-plan-admit-key",
        ),
    )
    .unwrap();
    let admitted = run(Cli::try_parse_from([
        "workvcs",
        "plan",
        "admit",
        "--cwd",
        &fixture.project_text,
        "--registry",
        &fixture.registry,
        "--session",
        &session,
        "--manifest",
        &path_text(&admit_path),
    ])
    .unwrap())
    .expect("admit Plan on Session Branch");
    assert_eq!(value(&admitted, "branch_id"), work_branch);
    assert_eq!(value(&admitted, "durable_route"), "projectref_journal");

    let evolve_path = fixture._tempdir.path().join("session-plan-evolve.json");
    fs::write(
        &evolve_path,
        supersede_manifest_for_cli_admission(&fixture, &admitted, "session-plan-evolve-key", 2),
    )
    .unwrap();
    let evolved = run(Cli::try_parse_from([
        "workvcs",
        "plan",
        "evolve",
        "--cwd",
        &fixture.project_text,
        "--registry",
        &fixture.registry,
        "--session",
        &session,
        "--manifest",
        &path_text(&evolve_path),
    ])
    .unwrap())
    .expect("evolve Plan on Session Branch");
    assert_eq!(value(&evolved, "branch_id"), work_branch);
    assert_eq!(value(&evolved, "durable_route"), "projectref_journal");
    let child_task = create_task(
        &fixture,
        &work_branch,
        &value(&evolved, "commit_id"),
        "Goal child delivery Task",
    );
    let child_task_id = value(&child_task, "task_entity_id");
    let attached = run(Cli::try_parse_from([
        "workvcs",
        "task",
        "contain",
        &fixture.store,
        "--branch",
        &work_branch,
        "--head",
        &value(&child_task, "commit_id"),
        "--parent",
        &value(&evolved, "new_plan_entity_id"),
        "--child",
        &child_task_id,
    ])
    .unwrap())
    .expect("attach actual Task to the current Plan");
    create_finding(
        &fixture,
        &work_branch,
        &value(&attached, "commit_id"),
        "GOAL_CHILD_TASK_RECORD_MARKER",
        &format!(r#"{{"task_id":"{child_task_id}"}}"#),
    );
    let goal_id = value(&admitted, "goal_entity_id");
    run(Cli::try_parse_from([
        "workvcs",
        "session",
        "focus-set",
        &fixture.store,
        "--session",
        &session,
        "--focus",
        &goal_id,
    ])
    .unwrap())
    .expect("select the final Goal for continued Plan work");

    let capture_path = write_v2_capture_manifest(
        &fixture,
        "session-capture.json",
        "session-capture-key",
        "SESSION_BRANCH_CAPTURE_MARKER",
    );
    let captured = run_session_capture(&fixture, &session, &capture_path, true)
        .expect("deliver Session-selected capture");
    assert_eq!(value(&captured, "delivery_status"), "completed");
    assert_eq!(
        value(&captured, "target_commit_id"),
        branch_head(&fixture, &work_branch)
    );
    let capture_id = value(&captured, "capture_id");

    let recalled =
        run_recall(&fixture, &["--session", &session], 20).expect("recall Session-selected Branch");
    assert_eq!(value(&recalled, "branch_id"), work_branch);
    assert_eq!(value(&recalled, "selected_session_id"), session);
    assert_eq!(value(&recalled, "goals_total"), "1");
    assert_eq!(value(&recalled, "plans_total"), "1");
    assert_ne!(value(&recalled, "tasks_total"), "0");
    assert!(recalled.contains("SESSION_BRANCH_CAPTURE_MARKER"));
    assert!(recalled.contains("GOAL_CHILD_TASK_RECORD_MARKER"));
    assert!(recalled.contains("Goal child delivery Task"));
    let plan_session = start_session(&fixture, &work_branch);
    let current_plan_id = value(&evolved, "new_plan_entity_id");
    run(Cli::try_parse_from([
        "workvcs",
        "session",
        "focus-set",
        &fixture.store,
        "--session",
        &plan_session,
        "--focus",
        &current_plan_id,
    ])
    .unwrap())
    .expect("focus a related Plan Session without a Task Claim");
    let other_goal = run(Cli::try_parse_from([
        "workvcs",
        "goal",
        "create",
        &fixture.store,
        "--branch",
        &work_branch,
        "--head",
        &branch_head(&fixture, &work_branch),
        "--description",
        "Unrelated independent Goal",
    ])
    .unwrap())
    .expect("create independent Goal on the same Branch");
    let other_goal_session = start_session(&fixture, &work_branch);
    run(Cli::try_parse_from([
        "workvcs",
        "session",
        "focus-set",
        &fixture.store,
        "--session",
        &other_goal_session,
        "--focus",
        &value(&other_goal, "goal_entity_id"),
    ])
    .unwrap())
    .expect("focus unrelated Goal Session");
    let goal_selected = run_recall(
        &fixture,
        &[
            "--branch",
            &work_branch,
            "--scope-json",
            &format!(r#"{{"focus_entity_id":"{goal_id}"}}"#),
        ],
        30,
    )
    .expect("Goal selector without Session override");
    assert!(goal_selected.contains(&session));
    assert!(goal_selected.contains(&plan_session));
    assert!(!goal_selected.contains(&other_goal_session));
    assert_eq!(value(&goal_selected, "sessions_total"), "2");
    let plan_selected = run_recall(
        &fixture,
        &[
            "--branch",
            &work_branch,
            "--scope-json",
            &format!(r#"{{"focus_entity_id":"{current_plan_id}"}}"#),
        ],
        30,
    )
    .expect("Plan selector without Session override");
    assert!(plan_selected.contains(&plan_session));
    assert!(!plan_selected.contains(&other_goal_session));

    let resumed = run(Cli::try_parse_from([
        "workvcs",
        "resume",
        "--cwd",
        &fixture.project_text,
        "--registry",
        &fixture.registry,
        "--session",
        &session,
        "--budget-items",
        "30",
    ])
    .unwrap())
    .expect("resume Session-selected non-default Branch");
    assert_eq!(value(&resumed, "branch_id"), work_branch);
    assert_eq!(value(&resumed, "session_id"), session);
    assert!(resumed.contains("SESSION_BRANCH_CAPTURE_MARKER"));
    assert!(resumed.contains("GOAL_CHILD_TASK_RECORD_MARKER"));
    assert_eq!(branch_head(&fixture, &fixture.branch), default_head_before);

    run(Cli::try_parse_from([
        "workvcs",
        "session",
        "switch",
        &fixture.store,
        "--session",
        &session,
        "--workspace",
        &fixture.workspace_id,
        "--branch",
        &fixture.branch,
    ])
    .unwrap())
    .expect("switch Session to default Branch");

    let status = run(Cli::try_parse_from([
        "workvcs",
        "project",
        "operation-recovery",
        "--status",
        "--capture-id",
        &capture_id,
        "--registry",
        &fixture.registry,
    ])
    .unwrap())
    .expect("read completed receipt after Session switch");
    assert_eq!(value(&status, "effective_recovery_state"), "completed");
    assert_eq!(value(&status, "delivery_receipt"), "true");
    assert_ne!(value(&status, "target_commit_id"), "none");

    let conflict = run_session_capture(&fixture, &session, &capture_path, true)
        .expect_err("same idempotency key cannot cross work context");
    assert!(
        conflict.to_string().contains("idempotency")
            || conflict.to_string().contains("work context")
    );
    assert_eq!(branch_head(&fixture, &fixture.branch), default_head_before);
}

#[test]
fn admitted_old_session_context_cannot_write_after_session_switch() {
    run_cli_test_with_large_stack(
        "work-context-session-guard",
        assert_admitted_old_session_context_cannot_write_after_session_switch,
    );
}

fn assert_admitted_old_session_context_cannot_write_after_session_switch() {
    let fixture = prepared_v2_fixture();
    let work_branch = fork_branch(&fixture, &fixture.branch, "pending-session-work");
    let session = start_session(&fixture, &work_branch);
    let manifest = write_v2_capture_manifest(
        &fixture,
        "pending-session-capture.json",
        "pending-session-capture-key",
        "PENDING_OLD_SESSION_CONTEXT_MARKER",
    );
    let admitted = run_session_capture(&fixture, &session, &manifest, false)
        .expect("admit without target delivery");
    let capture_id = value(&admitted, "capture_id");
    let default_before = branch_head(&fixture, &fixture.branch);
    let work_before = branch_head(&fixture, &work_branch);

    run(Cli::try_parse_from([
        "workvcs",
        "session",
        "switch",
        &fixture.store,
        "--session",
        &session,
        "--workspace",
        &fixture.workspace_id,
        "--branch",
        &fixture.branch,
    ])
    .unwrap())
    .expect("switch Session before recovery");
    let status = inspect_project_capture_recovery(
        Some(fixture.registry_path.clone()),
        CaptureId::parse_canonical(&capture_id).unwrap(),
    )
    .expect("inspect pending operation");
    let apply = run(Cli::try_parse_from([
        "workvcs",
        "project",
        "operation-recovery",
        "--apply",
        "--capture-id",
        &capture_id,
        "--registry",
        &fixture.registry,
        "--expected-registry-digest",
        &value(&status, "registry_digest"),
        "--expected-projection-digest",
        &value(&status, "projection_digest"),
    ])
    .unwrap());
    let error = apply.expect_err("stale Session work context must fail before Store write");
    assert!(
        error.to_string().contains("Session")
            || error.to_string().contains("session")
            || error.to_string().contains("work context"),
        "unexpected stale-context error: {error}"
    );
    assert_eq!(branch_head(&fixture, &fixture.branch), default_before);
    assert_eq!(branch_head(&fixture, &work_branch), work_before);
    let after = inspect_project_capture_recovery(
        Some(fixture.registry_path.clone()),
        CaptureId::parse_canonical(&capture_id).unwrap(),
    )
    .expect("inspect after rejected recovery");
    assert_eq!(value(&after, "delivery_receipt"), "false");
    assert_eq!(value(&after, "capture_completed_receipt"), "false");
}

#[test]
fn recall_delta_reports_real_changes_and_removals_and_rejects_scope_or_ancestry_drift() {
    run_cli_test_with_large_stack(
        "work-context-incremental-recall",
        assert_recall_delta_reports_real_changes_and_removals_and_rejects_scope_or_ancestry_drift,
    );
}

fn assert_recall_delta_reports_real_changes_and_removals_and_rejects_scope_or_ancestry_drift() {
    let fixture = prepared_v2_fixture();
    let scope = r#"{"work":"delta-a"}"#;
    let target = create_finding(
        &fixture,
        &fixture.branch,
        &fixture.genesis_commit_id,
        "DELTA_REMOVED_TARGET_MARKER",
        scope,
    );
    let base = run_recall(&fixture, &["--scope-json", scope], 20).expect("base recall");
    let base_head = value(&base, "head_commit_id");
    let context_digest = value(&base, "context_digest");
    let target_id = value(&target, "record_entity_id");

    let cause = create_finding(
        &fixture,
        &fixture.branch,
        &base_head,
        "DELTA_NEW_CAUSE_MARKER",
        scope,
    );
    let invalidated = run(Cli::try_parse_from([
        "workvcs",
        "record",
        "invalidate-finding",
        &fixture.store,
        "--branch",
        &fixture.branch,
        "--head",
        &value(&cause, "commit_id"),
        "--because-record",
        &value(&cause, "record_entity_id"),
        "--target-record",
        &target_id,
        "--target-record-version",
        &value(&target, "record_entity_version_id"),
        "--rationale",
        "new evidence invalidates the earlier scoped finding",
    ])
    .unwrap())
    .expect("invalidate prior Finding");

    let delta = run_recall(
        &fixture,
        &[
            "--scope-json",
            scope,
            "--since-commit",
            &base_head,
            "--expected-context-digest",
            &context_digest,
        ],
        20,
    )
    .expect("incremental recall");
    assert_eq!(value(&delta, "delta_from_commit"), base_head);
    assert_eq!(value(&delta, "removed_items"), "1");
    assert_eq!(value(&delta, "removed.0.category"), "record");
    assert_eq!(value(&delta, "removed.0.id"), target_id);
    assert!(delta.contains("DELTA_NEW_CAUSE_MARKER"));
    assert!(!delta.contains("DELTA_REMOVED_TARGET_MARKER"));
    assert_ne!(value(&delta, "context_digest"), context_digest);
    let wrong_cached_commit = run_recall(
        &fixture,
        &[
            "--scope-json",
            scope,
            "--since-commit",
            &value(&cause, "commit_id"),
            "--expected-context-digest",
            &context_digest,
        ],
        20,
    )
    .expect_err("a different valid ancestor cannot reuse the older cache digest");
    assert!(
        wrong_cached_commit
            .to_string()
            .contains("exact cached commit")
    );

    let scope_drift = run_recall(
        &fixture,
        &[
            "--scope-json",
            r#"{"work":"delta-b"}"#,
            "--since-commit",
            &base_head,
            "--expected-context-digest",
            &context_digest,
        ],
        20,
    )
    .expect_err("selection digest must reject changed scope");
    assert!(
        scope_drift
            .to_string()
            .contains("recovery selection changed")
    );

    let fork = fork_branch(&fixture, &fixture.branch, "delta-non-ancestor");
    let fork_record = create_finding(
        &fixture,
        &fork,
        &value(&invalidated, "commit_id"),
        "FORK_ONLY_DELTA_MARKER",
        scope,
    );
    let non_ancestor = run_recall(
        &fixture,
        &[
            "--scope-json",
            scope,
            "--since-commit",
            &value(&fork_record, "commit_id"),
            "--expected-context-digest",
            &context_digest,
        ],
        20,
    )
    .expect_err("non-ancestor delta basis must reject");
    assert!(non_ancestor.to_string().contains("must be an ancestor"));
}

#[test]
fn capture_validate_is_zero_write_and_reports_the_exact_invalid_field_path() {
    run_cli_test_with_large_stack(
        "work-context-capture-validate",
        assert_capture_validate_is_zero_write_and_reports_the_exact_invalid_field_path,
    );
}

fn assert_capture_validate_is_zero_write_and_reports_the_exact_invalid_field_path() {
    let fixture = create_project_binding_fixture(false);
    let valid = write_v2_capture_manifest(
        &fixture,
        "validate-only.json",
        "validate-only-key",
        "VALIDATE_ONLY_MARKER",
    );
    let store_before = fs::read(&fixture.store_path).expect("Store bytes before validation");
    let registry_before = fs::read(&fixture.registry_path).expect("registry before validation");
    let entries_before = directory_entry_names(fixture._tempdir.path());
    let output = run(Cli::try_parse_from([
        "workvcs",
        "capture-validate",
        "--manifest",
        &path_text(&valid),
    ])
    .unwrap())
    .expect("validate manifest without project state");
    assert_eq!(value(&output, "action"), "capture_validate");
    assert_eq!(value(&output, "valid"), "true");
    assert_eq!(value(&output, "read_only"), "true");
    assert_eq!(value(&output, "registry_read"), "false");
    assert_eq!(value(&output, "journal_written"), "false");
    assert_eq!(value(&output, "store_opened"), "false");

    let invalid = fixture._tempdir.path().join("invalid-capture.json");
    fs::write(
        &invalid,
        r#"{"schema_version":1,"idempotency_key":"invalid-field-path","records":[{"local_id":7,"kind":"finding","statement":"bad"}],"knowledge":[],"evidence":[],"relations":[],"rationale":{}}"#,
    )
    .unwrap();
    let failure = run(Cli::try_parse_from([
        "workvcs",
        "capture-validate",
        "--manifest",
        &path_text(&invalid),
    ])
    .unwrap())
    .expect_err("invalid field type must fail validation");
    let message = failure.to_string();
    assert!(message.contains("records[0].local_id"), "{message}");
    assert!(message.contains("string"), "{message}");
    assert_eq!(fs::read(&fixture.store_path).unwrap(), store_before);
    assert_eq!(fs::read(&fixture.registry_path).unwrap(), registry_before);
    let entries_after = directory_entry_names(fixture._tempdir.path());
    assert_eq!(
        entries_after
            .iter()
            .filter(|entry| *entry != "invalid-capture.json")
            .cloned()
            .collect::<Vec<_>>(),
        entries_before
    );
}
