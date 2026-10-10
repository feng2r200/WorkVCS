use tempfile::TempDir;
use workvcs_core::{
    Engine, GoalCreateOptions, GoalTransitionOptions, PlanCreateOptions, PlanTransitionOptions,
    PrimaryContainmentCreateOptions, StoreInitOptions, StructuralReferenceCreateOptions,
    TaskCreateOptions, WorkspaceInitOptions,
};

fn fixture() -> (TempDir, Engine, workvcs_core::WorkspaceInfo) {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let path = tempdir.path().join("workvcs.sqlite");
    let mut engine = Engine::init(
        &path,
        StoreInitOptions::new("currentness-p0").expect("store options"),
    )
    .expect("init Store");
    let workspace = engine
        .create_workspace(WorkspaceInitOptions::new("workspace").expect("workspace options"))
        .expect("create workspace");
    (tempdir, engine, workspace)
}

#[test]
fn currentness_uses_rooted_fixed_point_and_orphan_reference_cycles_do_not_self_activate() {
    let (_tempdir, mut engine, workspace) = fixture();
    let branch = workspace.initial_branch_id;
    let standalone_plan = engine
        .create_plan(
            PlanCreateOptions::new(
                branch,
                workspace.genesis_commit_id,
                "Standalone Plan",
                "Standalone strategy",
            )
            .expect("standalone Plan options"),
        )
        .expect("create standalone Plan");
    let standalone_task = engine
        .create_task(
            TaskCreateOptions::new(branch, standalone_plan.commit_id, "Standalone Task")
                .expect("standalone Task options"),
        )
        .expect("create standalone Task");
    let original_goal = engine
        .create_goal(
            GoalCreateOptions::new(branch, standalone_task.commit_id, "Original Goal")
                .expect("Goal options"),
        )
        .expect("create Goal");
    let plan_a = engine
        .create_plan(
            PlanCreateOptions::new(branch, original_goal.commit_id, "Plan A", "Strategy A")
                .expect("Plan A options"),
        )
        .expect("create Plan A");
    let goal_contains_a = engine
        .create_primary_containment(
            PrimaryContainmentCreateOptions::new(
                branch,
                plan_a.commit_id,
                original_goal.goal_entity_id,
                plan_a.plan_entity_id,
            )
            .expect("Goal contains A options"),
        )
        .expect("Goal contains A");
    let plan_b = engine
        .create_plan(
            PlanCreateOptions::new(branch, goal_contains_a.commit_id, "Plan B", "Strategy B")
                .expect("Plan B options"),
        )
        .expect("create Plan B");
    let goal_contains_b = engine
        .create_primary_containment(
            PrimaryContainmentCreateOptions::new(
                branch,
                plan_b.commit_id,
                original_goal.goal_entity_id,
                plan_b.plan_entity_id,
            )
            .expect("Goal contains B options"),
        )
        .expect("Goal contains B");
    let task_a = engine
        .create_task(
            TaskCreateOptions::new(branch, goal_contains_b.commit_id, "Plan A task")
                .expect("Plan A Task options"),
        )
        .expect("create Plan A Task");
    let plan_contains_task = engine
        .create_primary_containment(
            PrimaryContainmentCreateOptions::new(
                branch,
                task_a.commit_id,
                plan_a.plan_entity_id,
                task_a.task_entity_id,
            )
            .expect("Plan contains Task options"),
        )
        .expect("Plan contains Task");
    let a_references_b = engine
        .create_structural_reference(
            StructuralReferenceCreateOptions::new(
                branch,
                plan_contains_task.commit_id,
                plan_a.plan_entity_id,
                plan_b.plan_entity_id,
            )
            .expect("A references B options"),
        )
        .expect("A references B");
    let b_references_a = engine
        .create_structural_reference(
            StructuralReferenceCreateOptions::new(
                branch,
                a_references_b.commit_id,
                plan_b.plan_entity_id,
                plan_a.plan_entity_id,
            )
            .expect("B references A options"),
        )
        .expect("B references A");

    let rooted = engine
        .current_scope_at(b_references_a.commit_id)
        .expect("rooted current scope");
    assert!(rooted.current_goals.contains(&original_goal.goal_entity_id));
    assert!(rooted.current_plans.contains(&plan_a.plan_entity_id));
    assert!(rooted.current_plans.contains(&plan_b.plan_entity_id));
    assert!(rooted.current_tasks.contains(&task_a.task_entity_id));
    assert!(
        rooted
            .current_plans
            .contains(&standalone_plan.plan_entity_id)
    );
    assert!(
        rooted
            .current_tasks
            .contains(&standalone_task.task_entity_id)
    );

    let abandoned = engine
        .transition_goal(
            GoalTransitionOptions::abandon(
                branch,
                b_references_a.commit_id,
                original_goal.goal_entity_id,
                original_goal.goal_entity_version_id,
                "retire the original execution root",
            )
            .expect("abandon Goal options"),
        )
        .expect("abandon Goal");
    let orphaned = engine
        .current_scope_at(abandoned.commit_id)
        .expect("orphaned current scope");
    assert!(
        !orphaned
            .current_goals
            .contains(&original_goal.goal_entity_id)
    );
    assert!(!orphaned.current_plans.contains(&plan_a.plan_entity_id));
    assert!(!orphaned.current_plans.contains(&plan_b.plan_entity_id));
    assert!(!orphaned.current_tasks.contains(&task_a.task_entity_id));
    assert!(
        orphaned
            .current_plans
            .contains(&standalone_plan.plan_entity_id)
    );
    assert!(
        orphaned
            .current_tasks
            .contains(&standalone_task.task_entity_id)
    );

    let carrier_goal = engine
        .create_goal(
            GoalCreateOptions::new(branch, abandoned.commit_id, "External current carrier")
                .expect("carrier Goal options"),
        )
        .expect("create carrier Goal");
    let carrier_reference = engine
        .create_structural_reference(
            StructuralReferenceCreateOptions::new(
                branch,
                carrier_goal.commit_id,
                carrier_goal.goal_entity_id,
                plan_a.plan_entity_id,
            )
            .expect("carrier reference options"),
        )
        .expect("carrier references old Plan");
    let carried = engine
        .current_scope_at(carrier_reference.commit_id)
        .expect("externally carried scope");
    assert!(carried.current_plans.contains(&plan_a.plan_entity_id));
    assert!(carried.current_plans.contains(&plan_b.plan_entity_id));
    assert!(carried.current_tasks.contains(&task_a.task_entity_id));

    let completed_a = engine
        .transition_plan(
            PlanTransitionOptions::complete(
                branch,
                carrier_reference.commit_id,
                plan_a.plan_entity_id,
                plan_a.plan_entity_version_id,
            )
            .expect("complete externally carried Plan options"),
        )
        .expect("complete externally carried Plan");
    let stopped = engine
        .current_scope_at(completed_a.commit_id)
        .expect("terminal Plan stops propagation");
    assert!(!stopped.current_plans.contains(&plan_a.plan_entity_id));
    assert!(!stopped.current_plans.contains(&plan_b.plan_entity_id));
    assert!(!stopped.current_tasks.contains(&task_a.task_entity_id));
}
