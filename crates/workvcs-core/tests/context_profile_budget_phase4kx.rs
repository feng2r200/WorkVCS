use std::path::{Path, PathBuf};
use tempfile::TempDir;
use workvcs_core::{
    AcceptanceCriterionClassification, AcceptanceCriterionCreateOptions, ContextItemCategory,
    ContextPacketOptions, ContextPriority, ContextProfile, Engine, ErrorCode, GoalCreateOptions,
    KnowledgeCreateOptions, PlanCreateOptions, PrimaryContainmentCreateOptions,
    RecordCreateOptions, RecordTransitionOptions, SessionId, SessionStartOptions, StoreInitOptions,
    TaskCreateOptions, TaskSchedulingRelationCreateOptions, VerificationRequirementCreateOptions,
    WorkspaceInfo, WorkspaceInitOptions,
};

fn store_path() -> (TempDir, PathBuf) {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let path = tempdir.path().join("workvcs.sqlite");
    (tempdir, path)
}

fn init_engine(path: &Path) -> Engine {
    Engine::init(
        path,
        StoreInitOptions::new("phase4kx-context-profile-budget-store").expect("store options"),
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

fn contains_category(packet: &workvcs_core::ContextPacket, category: ContextItemCategory) -> bool {
    packet.items.iter().any(|item| item.category == category)
}

#[test]
fn context_packet_profiles_filter_categories_deterministically() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let branch_id = workspace.initial_branch_id;
    let mut head = workspace.genesis_commit_id;

    let task = engine
        .create_task(
            TaskCreateOptions::new(branch_id, head, "Implement context packet")
                .expect("task options"),
        )
        .expect("create task");
    head = task.commit_id;

    let decision = engine
        .create_record(
            RecordCreateOptions::decision(branch_id, head, "Use deterministic packet profiles")
                .expect("decision options"),
        )
        .expect("create decision");
    head = decision.commit_id;

    let assumption = engine
        .create_record(
            RecordCreateOptions::assumption(branch_id, head, "Agents need bounded context")
                .expect("assumption options"),
        )
        .expect("create assumption");
    head = assumption.commit_id;

    let attempt = engine
        .create_record(
            RecordCreateOptions::attempt(branch_id, head, "Prototype string truncation")
                .expect("attempt options"),
        )
        .expect("create attempt");
    head = attempt.commit_id;
    let failed_attempt = engine
        .transition_record(
            RecordTransitionOptions::complete_attempt_failed(
                branch_id,
                head,
                attempt.record_entity_id,
                attempt.record_entity_version_id,
                "Item omission preserves packet structure",
            )
            .expect("failed attempt options"),
        )
        .expect("fail attempt");
    head = failed_attempt.commit_id;

    let finding = engine
        .create_record(
            RecordCreateOptions::finding(branch_id, head, "Budget summaries are inspectable")
                .expect("finding options"),
        )
        .expect("create finding");
    head = finding.commit_id;

    let knowledge = engine
        .create_knowledge(
            KnowledgeCreateOptions::new(branch_id, head, "Context profile names are V1 vocabulary")
                .expect("knowledge options"),
        )
        .expect("create knowledge");
    head = knowledge.commit_id;

    let handoff = engine
        .create_record(
            RecordCreateOptions::handoff(branch_id, head, "Continue with verify wrapper next")
                .expect("handoff options"),
        )
        .expect("create handoff");
    head = handoff.commit_id;

    let risk = engine
        .create_record(
            RecordCreateOptions::risk(branch_id, head, "Do not claim full context resolver yet")
                .expect("risk options"),
        )
        .expect("create risk");

    let session = engine
        .start_session(
            SessionStartOptions::new(workspace.workspace_id, workspace.initial_branch_id)
                .expect("session options"),
        )
        .expect("start session");

    let brief = engine
        .context_packet(
            ContextPacketOptions::new(session.session_id).with_profile(ContextProfile::Brief),
        )
        .expect("brief context packet");
    let normal = engine
        .context_packet(ContextPacketOptions::new(session.session_id))
        .expect("normal context packet");
    let full = engine
        .context_packet(
            ContextPacketOptions::new(session.session_id).with_profile(ContextProfile::Full),
        )
        .expect("full context packet");

    assert_eq!(normal.profile, ContextProfile::Normal);
    assert_eq!(brief.budget_items, None);
    assert_eq!(brief.omission_summary.total, 0);
    assert_eq!(normal.omission_summary.total, 0);
    assert_eq!(full.omission_summary.total, 0);
    assert_eq!(full.envelope.head_commit_id, risk.commit_id);

    assert!(contains_category(
        &brief,
        ContextItemCategory::SessionAnchor
    ));
    assert!(contains_category(
        &brief,
        ContextItemCategory::BranchOverview
    ));
    assert!(contains_category(&brief, ContextItemCategory::CurrentTask));
    assert!(contains_category(
        &brief,
        ContextItemCategory::TaskReadiness
    ));
    assert!(contains_category(
        &brief,
        ContextItemCategory::ActiveDecision
    ));
    assert!(contains_category(
        &brief,
        ContextItemCategory::ActiveAssumption
    ));
    assert!(contains_category(
        &brief,
        ContextItemCategory::FailedAttempt
    ));
    assert!(!contains_category(&brief, ContextItemCategory::Finding));
    assert!(!contains_category(
        &brief,
        ContextItemCategory::ScopedKnowledge
    ));
    assert!(!contains_category(
        &brief,
        ContextItemCategory::RelevantHandoff
    ));
    assert!(!contains_category(
        &brief,
        ContextItemCategory::OlderProvenance
    ));

    assert!(contains_category(&normal, ContextItemCategory::Finding));
    assert!(contains_category(
        &normal,
        ContextItemCategory::ScopedKnowledge
    ));
    assert!(contains_category(
        &normal,
        ContextItemCategory::RelevantHandoff
    ));
    assert!(!contains_category(
        &normal,
        ContextItemCategory::OlderProvenance
    ));

    assert!(contains_category(
        &full,
        ContextItemCategory::OlderProvenance
    ));
    assert!(normal.items.len() > brief.items.len());
    assert!(full.items.len() > normal.items.len());
}

#[test]
fn context_packet_budget_omits_whole_low_priority_items() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let branch_id = workspace.initial_branch_id;
    let mut head = workspace.genesis_commit_id;

    let task = engine
        .create_task(
            TaskCreateOptions::new(branch_id, head, "Budgeted context task").expect("task options"),
        )
        .expect("create task");
    head = task.commit_id;
    let finding = engine
        .create_record(
            RecordCreateOptions::finding(branch_id, head, "Low-priority finding")
                .expect("finding options"),
        )
        .expect("create finding");
    head = finding.commit_id;
    let knowledge = engine
        .create_knowledge(
            KnowledgeCreateOptions::new(branch_id, head, "Scoped knowledge can be omitted")
                .expect("knowledge options"),
        )
        .expect("create knowledge");
    head = knowledge.commit_id;
    let risk = engine
        .create_record(
            RecordCreateOptions::risk(branch_id, head, "Full-only provenance")
                .expect("risk options"),
        )
        .expect("create risk");

    let session = engine
        .start_session(
            SessionStartOptions::new(workspace.workspace_id, workspace.initial_branch_id)
                .expect("session options"),
        )
        .expect("start session");

    let packet = engine
        .context_packet(
            ContextPacketOptions::new(session.session_id)
                .with_profile(ContextProfile::Full)
                .with_budget_items(3)
                .expect("budgeted context options"),
        )
        .expect("budgeted context packet");

    assert_eq!(packet.items.len(), 3);
    assert_eq!(packet.available_items, 8);
    assert_eq!(packet.omission_summary.total, 5);
    assert_eq!(packet.envelope.head_commit_id, risk.commit_id);
    assert!(
        packet
            .items
            .iter()
            .all(|item| item.priority == ContextPriority::P0)
    );
    assert!(
        packet
            .omission_summary
            .by_priority
            .iter()
            .any(|bucket| bucket.priority == ContextPriority::P2 && bucket.omitted == 1)
    );
    assert!(
        packet
            .omission_summary
            .by_category
            .iter()
            .any(
                |bucket| bucket.category == ContextItemCategory::ScopedKnowledge
                    && bucket.omitted == 1
            )
    );
    assert!(
        packet
            .omission_summary
            .by_category
            .iter()
            .any(
                |bucket| bucket.category == ContextItemCategory::OlderProvenance
                    && bucket.omitted == 1
            )
    );
}

#[test]
fn context_packet_includes_current_task_verification_obligations() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let branch_id = workspace.initial_branch_id;
    let mut head = workspace.genesis_commit_id;

    let task = engine
        .create_task(
            TaskCreateOptions::new(branch_id, head, "Verify packet obligations")
                .expect("task options"),
        )
        .expect("create task");
    head = task.commit_id;
    let criterion = engine
        .create_acceptance_criterion(
            AcceptanceCriterionCreateOptions::new(
                branch_id,
                head,
                task.task_entity_id,
                task.task_entity_version_id,
                "ac-context",
                "Packet tells the agent what must be verified.",
                AcceptanceCriterionClassification::Required,
            )
            .expect("acceptance criterion options"),
        )
        .expect("create acceptance criterion");
    head = criterion.commit_id;
    let requirement = engine
        .create_verification_requirement(
            VerificationRequirementCreateOptions::new(
                branch_id,
                head,
                criterion.acceptance_criterion_entity_id,
                criterion.acceptance_criterion_entity_version_id,
                "vr-wrapper",
                "Run the verify wrapper with captured evidence.",
            )
            .expect("verification requirement options"),
        )
        .expect("create verification requirement");

    let session = engine
        .start_session(
            SessionStartOptions::new(workspace.workspace_id, workspace.initial_branch_id)
                .expect("session options"),
        )
        .expect("start session");

    let packet = engine
        .context_packet(
            ContextPacketOptions::new(session.session_id)
                .with_profile(ContextProfile::Brief)
                .with_budget_items(5)
                .expect("budgeted context options"),
        )
        .expect("budgeted context packet");

    assert_eq!(packet.envelope.head_commit_id, requirement.commit_id);
    assert_eq!(packet.available_items, 6);
    assert_eq!(packet.items.len(), 5);
    assert_eq!(packet.items[0].category, ContextItemCategory::SessionAnchor);
    assert_eq!(
        packet.items[1].category,
        ContextItemCategory::BranchOverview
    );
    assert_eq!(packet.items[2].category, ContextItemCategory::CurrentTask);
    assert_eq!(
        packet.items[3].category,
        ContextItemCategory::AcceptanceCriterion
    );
    assert_eq!(
        packet.items[3].subject.as_ref_string(),
        format!(
            "acceptance_criterion:{}",
            criterion.acceptance_criterion_entity_id
        )
    );
    assert!(packet.items[3].summary.contains("local_key=ac-context"));
    assert!(packet.items[3].summary.contains("classification=required"));
    assert!(packet.items[3].summary.contains("status=unverified"));
    assert!(packet.items[3].summary.contains("requirements=1"));
    assert!(
        packet.items[3]
            .summary
            .contains("Packet tells the agent what must be verified.")
    );
    assert_eq!(
        packet.items[4].category,
        ContextItemCategory::VerificationRequirement
    );
    assert_eq!(
        packet.items[4].subject.as_ref_string(),
        format!(
            "verification_requirement:{}",
            requirement.verification_requirement_entity_id
        )
    );
    assert!(packet.items[4].summary.contains(&format!(
        "criterion={}",
        criterion.acceptance_criterion_entity_id
    )));
    assert!(packet.items[4].summary.contains("local_key=vr-wrapper"));
    assert!(
        packet.items[4]
            .summary
            .contains("Run the verify wrapper with captured evidence.")
    );
    assert_eq!(packet.omission_summary.total, 1);
    assert!(
        packet
            .omission_summary
            .by_category
            .iter()
            .any(
                |bucket| bucket.category == ContextItemCategory::TaskReadiness
                    && bucket.omitted == 1
            )
    );
}

#[test]
fn context_packet_includes_goal_plan_path_for_current_tasks() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let branch_id = workspace.initial_branch_id;
    let mut head = workspace.genesis_commit_id;

    let goal = engine
        .create_goal(
            GoalCreateOptions::new(branch_id, head, "Ship the context resolver")
                .expect("goal options"),
        )
        .expect("create goal");
    head = goal.commit_id;
    let plan = engine
        .create_plan(
            PlanCreateOptions::new(
                branch_id,
                head,
                "Implement path packets",
                "Reuse current containment snapshots",
            )
            .expect("plan options"),
        )
        .expect("create plan");
    head = plan.commit_id;
    let task = engine
        .create_task(
            TaskCreateOptions::new(branch_id, head, "Expose path to the agent")
                .expect("task options"),
        )
        .expect("create task");
    head = task.commit_id;
    let goal_plan = engine
        .create_primary_containment(
            PrimaryContainmentCreateOptions::new(
                branch_id,
                head,
                goal.goal_entity_id,
                plan.plan_entity_id,
            )
            .expect("goal-plan containment options"),
        )
        .expect("create goal-plan containment");
    head = goal_plan.commit_id;
    let plan_task = engine
        .create_primary_containment(
            PrimaryContainmentCreateOptions::new(
                branch_id,
                head,
                plan.plan_entity_id,
                task.task_entity_id,
            )
            .expect("plan-task containment options"),
        )
        .expect("create plan-task containment");

    let session = engine
        .start_session(
            SessionStartOptions::new(workspace.workspace_id, workspace.initial_branch_id)
                .expect("session options"),
        )
        .expect("start session");

    let packet = engine
        .context_packet(
            ContextPacketOptions::new(session.session_id).with_profile(ContextProfile::Brief),
        )
        .expect("brief context packet");

    assert_eq!(packet.envelope.head_commit_id, plan_task.commit_id);
    assert_eq!(packet.available_items, 5);
    assert_eq!(packet.items[0].category, ContextItemCategory::SessionAnchor);
    assert_eq!(
        packet.items[1].category,
        ContextItemCategory::BranchOverview
    );
    assert_eq!(packet.items[2].category, ContextItemCategory::CurrentTask);
    let path_item = packet
        .items
        .iter()
        .find(|item| item.category == ContextItemCategory::GoalPlanPath)
        .expect("goal plan path item");
    assert_eq!(path_item.priority, ContextPriority::P1);
    assert_eq!(
        path_item.subject.as_ref_string(),
        format!("goal_plan_path:{}", task.task_entity_id)
    );
    assert!(
        path_item
            .summary
            .contains(&format!("task={}", task.task_entity_id))
    );
    assert!(
        path_item
            .summary
            .contains(&format!("goal:{}", goal.goal_entity_id))
    );
    assert!(path_item.summary.contains("status=active"));
    assert!(path_item.summary.contains("Ship the context resolver"));
    assert!(
        path_item
            .summary
            .contains(&format!("plan:{}", plan.plan_entity_id))
    );
    assert!(path_item.summary.contains("Implement path packets"));
    assert!(
        path_item
            .summary
            .contains(&format!("relation={}", goal_plan.relation_id))
    );
    assert!(
        path_item
            .summary
            .contains(&format!("relation={}", plan_task.relation_id))
    );

    let tight_packet = engine
        .context_packet(
            ContextPacketOptions::new(session.session_id)
                .with_profile(ContextProfile::Brief)
                .with_budget_items(3)
                .expect("budgeted context options"),
        )
        .expect("tight context packet");

    assert_eq!(tight_packet.items.len(), 3);
    assert!(
        !tight_packet
            .items
            .iter()
            .any(|item| item.category == ContextItemCategory::GoalPlanPath)
    );
    assert!(
        tight_packet
            .omission_summary
            .by_category
            .iter()
            .any(
                |bucket| bucket.category == ContextItemCategory::GoalPlanPath
                    && bucket.omitted == 1
            )
    );
}

#[test]
fn context_packet_explains_blocked_dependency_tasks() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let branch_id = workspace.initial_branch_id;
    let mut head = workspace.genesis_commit_id;

    let dependent = engine
        .create_task(
            TaskCreateOptions::new(branch_id, head, "Blocked dependent task")
                .expect("dependent task options"),
        )
        .expect("create dependent task");
    head = dependent.commit_id;
    let prerequisite = engine
        .create_task(
            TaskCreateOptions::new(branch_id, head, "Ready prerequisite task")
                .expect("prerequisite task options"),
        )
        .expect("create prerequisite task");
    head = prerequisite.commit_id;
    let relation = engine
        .create_task_scheduling_relation(
            TaskSchedulingRelationCreateOptions::depends_on(
                branch_id,
                head,
                dependent.task_entity_id,
                prerequisite.task_entity_id,
            )
            .expect("dependency options"),
        )
        .expect("create dependency");

    let session = engine
        .start_session(
            SessionStartOptions::new(workspace.workspace_id, workspace.initial_branch_id)
                .expect("session options"),
        )
        .expect("start session");

    let packet = engine
        .context_packet(
            ContextPacketOptions::new(session.session_id).with_profile(ContextProfile::Brief),
        )
        .expect("brief context packet");

    assert_eq!(packet.envelope.head_commit_id, relation.commit_id);
    let blocker = packet
        .items
        .iter()
        .find(|item| item.category == ContextItemCategory::BlockedDependency)
        .expect("blocked dependency item");
    assert_eq!(blocker.priority, ContextPriority::P2);
    assert_eq!(
        blocker.subject.as_ref_string(),
        format!(
            "blocked_dependency:{}:{}",
            dependent.task_entity_id, prerequisite.task_entity_id
        )
    );
    assert!(
        blocker
            .summary
            .contains(&format!("task={}", dependent.task_entity_id))
    );
    assert!(
        blocker
            .summary
            .contains(&format!("dependency={}", prerequisite.task_entity_id))
    );
    assert!(blocker.summary.contains("dependency_status=pending"));
    assert!(blocker.summary.contains("dependency_priority=0"));
    assert!(blocker.summary.contains("Ready prerequisite task"));
}

#[test]
fn context_packet_budget_preserves_runnable_order_within_current_tasks() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let branch_id = workspace.initial_branch_id;
    let mut head = workspace.genesis_commit_id;

    let dependent = engine
        .create_task(
            TaskCreateOptions::new(branch_id, head, "Blocked dependent task")
                .expect("dependent task options"),
        )
        .expect("create dependent task");
    head = dependent.commit_id;
    let prerequisite = engine
        .create_task(
            TaskCreateOptions::new(branch_id, head, "Ready prerequisite task")
                .expect("prerequisite task options"),
        )
        .expect("create prerequisite task");
    head = prerequisite.commit_id;
    let relation = engine
        .create_task_scheduling_relation(
            TaskSchedulingRelationCreateOptions::depends_on(
                branch_id,
                head,
                dependent.task_entity_id,
                prerequisite.task_entity_id,
            )
            .expect("dependency options"),
        )
        .expect("create dependency");

    let session = engine
        .start_session(
            SessionStartOptions::new(workspace.workspace_id, workspace.initial_branch_id)
                .expect("session options"),
        )
        .expect("start session");

    let packet = engine
        .context_packet(
            ContextPacketOptions::new(session.session_id)
                .with_profile(ContextProfile::Brief)
                .with_budget_items(3)
                .expect("budgeted context options"),
        )
        .expect("budgeted context packet");

    assert_eq!(packet.envelope.head_commit_id, relation.commit_id);
    assert_eq!(packet.items.len(), 3);
    assert_eq!(packet.items[2].category, ContextItemCategory::CurrentTask);
    assert_eq!(
        packet.items[2].subject.as_ref_string(),
        format!("task:{}", prerequisite.task_entity_id)
    );
    assert!(
        packet
            .omission_summary
            .by_category
            .iter()
            .any(
                |bucket| bucket.category == ContextItemCategory::CurrentTask && bucket.omitted == 1
            )
    );
    assert!(
        packet
            .omission_summary
            .by_category
            .iter()
            .any(
                |bucket| bucket.category == ContextItemCategory::BlockedDependency
                    && bucket.omitted == 1
            )
    );
}

#[test]
fn context_packet_rejects_zero_item_budget() {
    let (_tempdir, path) = store_path();
    let (_engine, _workspace) = create_workspace(&path);

    let error = ContextPacketOptions::new(SessionId::new_v7())
        .with_budget_items(0)
        .expect_err("zero context item budget should fail");

    assert_eq!(error.code(), ErrorCode::QueryInvalid);
}
