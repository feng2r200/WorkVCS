use std::path::{Path, PathBuf};
use tempfile::TempDir;
use workvcs_core::{
    ContextItemCategory, ContextOverviewOptions, ContextPacketOptions, ContextProfile, Engine,
    ErrorCategory, ErrorCode, GoalCreateOptions, KnowledgeCreateOptions,
    KnowledgeTransitionOptions, PlanCreateOptions, PlanTransitionOptions,
    PrimaryContainmentCreateOptions, RecordCreateOptions, RecordRelationCreateOptions,
    RecordRelationType, SessionEndOptions, SessionFocusOptions, SessionStartOptions,
    StoreInitOptions, StructuralReferenceCreateOptions, TaskCreateOptions, TaskStatus,
    TaskTransitionOptions, WorkspaceInfo, WorkspaceInitOptions,
};

fn store_path() -> (TempDir, PathBuf) {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let path = tempdir.path().join("workvcs.sqlite");
    (tempdir, path)
}

fn init_engine(path: &Path) -> Engine {
    Engine::init(
        path,
        StoreInitOptions::new("phase3aj-context-overview-store").expect("store options"),
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
fn context_overview_returns_active_anchor_focus_and_runnable_summary() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let task = engine
        .create_task(
            TaskCreateOptions::new(
                workspace.initial_branch_id,
                workspace.genesis_commit_id,
                "Focused context task",
            )
            .expect("task options"),
        )
        .expect("create task");
    let session = engine
        .start_session(
            SessionStartOptions::new(workspace.workspace_id, workspace.initial_branch_id)
                .expect("session options"),
        )
        .expect("start session");
    engine
        .set_session_focus(SessionFocusOptions::new(
            session.session_id,
            task.task_entity_id,
        ))
        .expect("set focus");

    let context = engine
        .context_overview(ContextOverviewOptions::new(session.session_id))
        .expect("context overview");

    assert_eq!(context.session.session_id, session.session_id);
    assert_eq!(context.branch.workspace_id, workspace.workspace_id);
    assert_eq!(context.branch.branch_id, workspace.initial_branch_id);
    assert_eq!(context.branch.head_commit_id, task.commit_id);
    assert_eq!(context.branch.state_digest, task.work_state_digest);
    assert_eq!(
        context.session.context_workspaces,
        vec![workspace.workspace_id]
    );
    assert_eq!(
        context
            .session
            .focus
            .as_ref()
            .expect("session focus")
            .focus_entity_id,
        task.task_entity_id
    );
    assert_eq!(context.runnable_tasks.candidates.len(), 1);
    assert_eq!(
        context.runnable_tasks.candidates[0].task.task_entity_id,
        task.task_entity_id
    );
    assert!(context.runnable_tasks.candidates[0].runnable);
    assert!(context.knowledge.knowledge.is_empty());
    assert!(context.records.records.is_empty());
    assert!(context.record_relations.relations.is_empty());
}

#[test]
fn context_overview_is_read_only() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let task = engine
        .create_task(
            TaskCreateOptions::new(
                workspace.initial_branch_id,
                workspace.genesis_commit_id,
                "Read only context task",
            )
            .expect("task options"),
        )
        .expect("create task");
    let session = engine
        .start_session(
            SessionStartOptions::new(workspace.workspace_id, workspace.initial_branch_id)
                .expect("session options"),
        )
        .expect("start session");
    let before = engine
        .branch_head(workspace.initial_branch_id)
        .expect("branch before context");

    let context = engine
        .context_overview(ContextOverviewOptions::new(session.session_id))
        .expect("context overview");

    let after = engine
        .branch_head(workspace.initial_branch_id)
        .expect("branch after context");
    assert_eq!(after, before);
    assert_eq!(context.branch.head_commit_id, task.commit_id);
    assert!(context.knowledge.knowledge.is_empty());
    assert!(context.records.records.is_empty());
    assert!(context.record_relations.relations.is_empty());
}

#[test]
fn context_overview_includes_current_record_summaries() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let finding = engine
        .create_record(
            RecordCreateOptions::finding(
                workspace.initial_branch_id,
                workspace.genesis_commit_id,
                "The current context needs explicit findings",
            )
            .expect("finding options"),
        )
        .expect("create finding");
    let session = engine
        .start_session(
            SessionStartOptions::new(workspace.workspace_id, workspace.initial_branch_id)
                .expect("session options"),
        )
        .expect("start session");

    let context = engine
        .context_overview(ContextOverviewOptions::new(session.session_id))
        .expect("context overview");

    assert_eq!(context.records.workspace_id, workspace.workspace_id);
    assert_eq!(context.records.commit_id, finding.commit_id);
    assert_eq!(context.records.records.len(), 1);
    assert_eq!(
        context.records.records[0].record_entity_id,
        finding.record_entity_id
    );
    assert_eq!(context.records.records[0].state.kind, finding.state.kind);
    assert_eq!(
        context.records.records[0].state.status,
        finding.state.status
    );
    assert_eq!(
        context.records.records[0].state.statement,
        finding.state.statement
    );
    assert!(context.knowledge.knowledge.is_empty());
    assert!(context.record_relations.relations.is_empty());
}

#[test]
fn context_overview_includes_active_knowledge_summaries() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let active = engine
        .create_knowledge(
            KnowledgeCreateOptions::new(
                workspace.initial_branch_id,
                workspace.genesis_commit_id,
                "Serialized writes make SQLite sufficient for V0.1",
            )
            .expect("active knowledge options"),
        )
        .expect("create active knowledge");
    let stale = engine
        .create_knowledge(
            KnowledgeCreateOptions::new(
                workspace.initial_branch_id,
                active.commit_id,
                "Legacy cache keys do not need schema versions",
            )
            .expect("stale knowledge options"),
        )
        .expect("create stale knowledge");
    let invalidated = engine
        .transition_knowledge(
            KnowledgeTransitionOptions::invalidate(
                workspace.initial_branch_id,
                stale.commit_id,
                stale.knowledge_entity_id,
                stale.knowledge_entity_version_id,
                "Schema versions are part of cache keys",
            )
            .expect("invalidate options"),
        )
        .expect("invalidate stale knowledge");
    let session = engine
        .start_session(
            SessionStartOptions::new(workspace.workspace_id, workspace.initial_branch_id)
                .expect("session options"),
        )
        .expect("start session");

    let context = engine
        .context_overview(ContextOverviewOptions::new(session.session_id))
        .expect("context overview");

    assert_eq!(context.knowledge.workspace_id, workspace.workspace_id);
    assert_eq!(context.knowledge.commit_id, invalidated.commit_id);
    assert_eq!(context.knowledge.knowledge.len(), 1);
    assert_eq!(
        context.knowledge.knowledge[0].knowledge_entity_id,
        active.knowledge_entity_id
    );
    assert_eq!(
        context.knowledge.knowledge[0].state.statement,
        active.state.statement
    );
    assert!(context.records.records.is_empty());
    assert!(context.record_relations.relations.is_empty());
}

#[test]
fn context_overview_includes_current_record_relation_summaries() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let decision = engine
        .create_record(
            RecordCreateOptions::decision(
                workspace.initial_branch_id,
                workspace.genesis_commit_id,
                "Use serialized writes",
            )
            .expect("decision options"),
        )
        .expect("create decision");
    let finding = engine
        .create_record(
            RecordCreateOptions::finding(
                workspace.initial_branch_id,
                decision.commit_id,
                "Benchmarks support serialized writes",
            )
            .expect("finding options"),
        )
        .expect("create finding");
    let relation = engine
        .create_record_relation(
            RecordRelationCreateOptions::supports(
                workspace.initial_branch_id,
                finding.commit_id,
                finding.record_entity_id,
                decision.record_entity_id,
                "Finding supports decision",
            )
            .expect("relation options"),
        )
        .expect("create supports relation");
    let session = engine
        .start_session(
            SessionStartOptions::new(workspace.workspace_id, workspace.initial_branch_id)
                .expect("session options"),
        )
        .expect("start session");

    let context = engine
        .context_overview(ContextOverviewOptions::new(session.session_id))
        .expect("context overview");

    assert_eq!(context.records.records.len(), 2);
    assert_eq!(
        context.record_relations.workspace_id,
        workspace.workspace_id
    );
    assert_eq!(context.record_relations.commit_id, relation.commit_id);
    assert_eq!(context.record_relations.relations.len(), 1);
    assert_eq!(
        context.record_relations.relations[0].relation_id,
        relation.relation_id
    );
    assert_eq!(
        context.record_relations.relations[0].relation_type,
        RecordRelationType::Supports
    );
    assert_eq!(
        context.record_relations.relations[0].source_record_entity_id,
        finding.record_entity_id
    );
    assert_eq!(
        context.record_relations.relations[0].target_record_entity_id,
        decision.record_entity_id
    );
}

#[test]
fn context_overview_rejects_ended_session() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let session = engine
        .start_session(
            SessionStartOptions::new(workspace.workspace_id, workspace.initial_branch_id)
                .expect("session options"),
        )
        .expect("start session");
    engine
        .end_session(SessionEndOptions::new(session.session_id).expect("end options"))
        .expect("end session");

    let error = engine
        .context_overview(ContextOverviewOptions::new(session.session_id))
        .expect_err("ended session context");
    assert_eq!(error.code(), ErrorCode::SessionInvalid);
    assert_eq!(error.category(), ErrorCategory::Runtime);
}

#[test]
fn terminal_task_containers_keep_live_descendant_scope_and_context_paths() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let goal = engine
        .create_goal(
            GoalCreateOptions::new(
                workspace.initial_branch_id,
                workspace.genesis_commit_id,
                "Current goal",
            )
            .expect("goal options"),
        )
        .expect("create goal");
    let plan = engine
        .create_plan(
            PlanCreateOptions::new(
                workspace.initial_branch_id,
                goal.commit_id,
                "Current plan",
                "Exercise terminal container traversal",
            )
            .expect("plan options"),
        )
        .expect("create plan");
    let contained_container = engine
        .create_task(
            TaskCreateOptions::new(
                workspace.initial_branch_id,
                plan.commit_id,
                "Terminal containment container",
            )
            .expect("contained container options"),
        )
        .expect("create contained container");
    let contained_leaf = engine
        .create_task(
            TaskCreateOptions::new(
                workspace.initial_branch_id,
                contained_container.commit_id,
                "Live contained leaf",
            )
            .expect("contained leaf options"),
        )
        .expect("create contained leaf");
    let referenced_container = engine
        .create_task(
            TaskCreateOptions::new(
                workspace.initial_branch_id,
                contained_leaf.commit_id,
                "Terminal referenced container",
            )
            .expect("referenced container options"),
        )
        .expect("create referenced container");
    let referenced_leaf = engine
        .create_task(
            TaskCreateOptions::new(
                workspace.initial_branch_id,
                referenced_container.commit_id,
                "Live referenced leaf",
            )
            .expect("referenced leaf options"),
        )
        .expect("create referenced leaf");
    let goal_plan = engine
        .create_primary_containment(
            PrimaryContainmentCreateOptions::new(
                workspace.initial_branch_id,
                referenced_leaf.commit_id,
                goal.goal_entity_id,
                plan.plan_entity_id,
            )
            .expect("goal plan containment options"),
        )
        .expect("contain plan under goal");
    let plan_container = engine
        .create_primary_containment(
            PrimaryContainmentCreateOptions::new(
                workspace.initial_branch_id,
                goal_plan.commit_id,
                plan.plan_entity_id,
                contained_container.task_entity_id,
            )
            .expect("plan container options"),
        )
        .expect("contain terminal container under plan");
    let container_leaf = engine
        .create_primary_containment(
            PrimaryContainmentCreateOptions::new(
                workspace.initial_branch_id,
                plan_container.commit_id,
                contained_container.task_entity_id,
                contained_leaf.task_entity_id,
            )
            .expect("container leaf options"),
        )
        .expect("contain live leaf under terminal container");
    let referenced_container_leaf = engine
        .create_primary_containment(
            PrimaryContainmentCreateOptions::new(
                workspace.initial_branch_id,
                container_leaf.commit_id,
                referenced_container.task_entity_id,
                referenced_leaf.task_entity_id,
            )
            .expect("referenced container leaf options"),
        )
        .expect("contain referenced leaf");
    let goal_reference = engine
        .create_structural_reference(
            StructuralReferenceCreateOptions::new(
                workspace.initial_branch_id,
                referenced_container_leaf.commit_id,
                goal.goal_entity_id,
                referenced_container.task_entity_id,
            )
            .expect("goal reference options"),
        )
        .expect("reference terminal container from goal");
    let contained_done = engine
        .transition_task(
            TaskTransitionOptions::new(
                workspace.initial_branch_id,
                goal_reference.commit_id,
                contained_container.task_entity_id,
                contained_container.task_entity_version_id,
                TaskStatus::Done,
            )
            .expect("complete contained container options"),
        )
        .expect("complete contained container");
    let referenced_done = engine
        .transition_task(
            TaskTransitionOptions::new(
                workspace.initial_branch_id,
                contained_done.commit_id,
                referenced_container.task_entity_id,
                referenced_container.task_entity_version_id,
                TaskStatus::Done,
            )
            .expect("complete referenced container options"),
        )
        .expect("complete referenced container");
    let session = engine
        .start_session(
            SessionStartOptions::new(workspace.workspace_id, workspace.initial_branch_id)
                .expect("session options"),
        )
        .expect("start session");

    let scope = engine
        .current_scope_at(referenced_done.commit_id)
        .expect("current scope");
    assert!(
        !scope
            .current_tasks
            .contains(&contained_container.task_entity_id)
    );
    assert!(
        !scope
            .current_tasks
            .contains(&referenced_container.task_entity_id)
    );
    assert!(scope.current_tasks.contains(&contained_leaf.task_entity_id));
    assert!(
        scope
            .current_tasks
            .contains(&referenced_leaf.task_entity_id)
    );
    assert!(scope.is_scope_reachable(contained_container.task_entity_id));
    assert!(scope.is_scope_reachable(referenced_container.task_entity_id));
    assert!(
        scope
            .carriers_for(referenced_container.task_entity_id)
            .iter()
            .any(|carrier| carrier.relation_id == Some(goal_reference.relation_id))
    );

    let context = engine
        .context_overview(ContextOverviewOptions::new(session.session_id))
        .expect("context overview");
    assert_eq!(context.tasks.len(), 2);
    assert!(context.tasks.iter().all(|task| task.task_entity_id
        != contained_container.task_entity_id
        && task.task_entity_id != referenced_container.task_entity_id));
    assert_eq!(context.runnable_tasks.candidates.len(), 2);
    for relation_id in [
        goal_plan.relation_id,
        plan_container.relation_id,
        container_leaf.relation_id,
        referenced_container_leaf.relation_id,
    ] {
        assert!(
            context
                .primary_containment_relations
                .iter()
                .any(|relation| relation.relation_id == relation_id)
        );
    }
    assert!(
        context
            .structural_references
            .iter()
            .any(|reference| reference.relation_id == goal_reference.relation_id)
    );

    let packet = engine
        .context_packet(ContextPacketOptions::new(session.session_id))
        .expect("context packet");
    let path = packet
        .items
        .iter()
        .find(|item| {
            item.category == ContextItemCategory::GoalPlanPath
                && item
                    .summary
                    .contains(&contained_leaf.task_entity_id.to_string())
        })
        .expect("contained leaf goal plan path");
    assert!(path.summary.contains(&goal.goal_entity_id.to_string()));
    assert!(path.summary.contains(&plan.plan_entity_id.to_string()));
}

#[test]
fn context_keeps_only_actual_currentness_carrier_edges_for_externally_referenced_task() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let retired_plan = engine
        .create_plan(
            PlanCreateOptions::new(
                workspace.initial_branch_id,
                workspace.genesis_commit_id,
                "Retired parent Plan",
                "Retired strategy",
            )
            .expect("retired Plan options"),
        )
        .expect("create retired Plan");
    let task = engine
        .create_task(
            TaskCreateOptions::new(
                workspace.initial_branch_id,
                retired_plan.commit_id,
                "Task carried by an external Goal reference",
            )
            .expect("carried Task options"),
        )
        .expect("create carried Task");
    let retired_contains_task = engine
        .create_primary_containment(
            PrimaryContainmentCreateOptions::new(
                workspace.initial_branch_id,
                task.commit_id,
                retired_plan.plan_entity_id,
                task.task_entity_id,
            )
            .expect("retired containment options"),
        )
        .expect("retired Plan contains Task");
    let abandoned = engine
        .transition_plan(
            PlanTransitionOptions::abandon(
                workspace.initial_branch_id,
                retired_contains_task.commit_id,
                retired_plan.plan_entity_id,
                retired_plan.plan_entity_version_id,
                "retire the original Task parent",
            )
            .expect("abandon retired Plan options"),
        )
        .expect("abandon retired Plan");
    let carrier_goal = engine
        .create_goal(
            GoalCreateOptions::new(
                workspace.initial_branch_id,
                abandoned.commit_id,
                "Current external carrier",
            )
            .expect("carrier Goal options"),
        )
        .expect("create carrier Goal");
    let carrier_reference = engine
        .create_structural_reference(
            StructuralReferenceCreateOptions::new(
                workspace.initial_branch_id,
                carrier_goal.commit_id,
                carrier_goal.goal_entity_id,
                task.task_entity_id,
            )
            .expect("carrier reference options"),
        )
        .expect("carrier Goal references Task");
    let session = engine
        .start_session(
            SessionStartOptions::new(workspace.workspace_id, workspace.initial_branch_id)
                .expect("session options"),
        )
        .expect("start session");

    let scope = engine
        .current_scope_at(carrier_reference.commit_id)
        .expect("current scope");
    assert!(scope.current_tasks.contains(&task.task_entity_id));
    assert!(!scope.current_plans.contains(&retired_plan.plan_entity_id));
    assert!(!scope.is_scope_reachable(retired_plan.plan_entity_id));
    assert!(scope.has_relation_carrier(
        task.task_entity_id,
        workvcs_core::CurrentnessCarrierKind::StructuralReference,
        carrier_goal.goal_entity_id,
        carrier_reference.relation_id,
        carrier_reference.relation_version_id,
    ));
    assert!(!scope.has_relation_carrier(
        task.task_entity_id,
        workvcs_core::CurrentnessCarrierKind::PrimaryContainment,
        retired_plan.plan_entity_id,
        retired_contains_task.relation_id,
        retired_contains_task.relation_version_id,
    ));

    let context = engine
        .context_overview(ContextOverviewOptions::new(session.session_id))
        .expect("context overview");
    assert_eq!(context.tasks.len(), 1);
    assert_eq!(context.tasks[0].task_entity_id, task.task_entity_id);
    assert!(context.plans.is_empty());
    assert_eq!(context.runnable_tasks.candidates.len(), 1);
    assert!(
        !context
            .primary_containment_relations
            .iter()
            .any(|relation| relation.relation_id == retired_contains_task.relation_id)
    );
    assert!(
        context
            .structural_references
            .iter()
            .any(|reference| reference.relation_id == carrier_reference.relation_id)
    );

    for profile in [
        ContextProfile::Brief,
        ContextProfile::Normal,
        ContextProfile::Full,
    ] {
        let packet = engine
            .context_packet(ContextPacketOptions::new(session.session_id).with_profile(profile))
            .expect("carrier-only context packet");
        assert!(packet.items.iter().all(|item| {
            !item
                .summary
                .contains(&retired_plan.plan_entity_id.to_string())
        }));
    }
}
