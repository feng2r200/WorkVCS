use rusqlite::{Connection, params};
use std::path::{Path, PathBuf};
use tempfile::TempDir;
use workvcs_core::{
    BranchId, CommitId, Engine, EntityId, ErrorCategory, ErrorCode, GoalCreateOptions,
    GoalSnapshot, PlanCreateOptions, PlanSnapshot, PrimaryContainmentCreateOptions,
    PrimaryContainmentEndpointKind, PrimaryContainmentSnapshot, StoreInitOptions,
    TaskCreateOptions, TaskSnapshot, WorkspaceInfo, WorkspaceInitOptions,
};

#[derive(Clone, Debug, PartialEq, Eq)]
struct HistoryCounts {
    object_identity: i64,
    entity: i64,
    relation: i64,
    relation_version: i64,
    entity_version: i64,
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
        StoreInitOptions::new("phase3n-goal-containment-store").expect("store options"),
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
        relation: count_rows(connection, "relation"),
        relation_version: count_rows(connection, "relation_version"),
        entity_version: count_rows(connection, "entity_version"),
        changeset: count_rows(connection, "changeset"),
        change_operation: count_rows(connection, "change_operation"),
        entity_membership_change: count_rows(connection, "entity_membership_change"),
        relation_membership_change: count_rows(connection, "relation_membership_change"),
        workstate_commit: count_rows(connection, "workstate_commit"),
        commit_parent: count_rows(connection, "commit_parent"),
        event: count_rows(connection, "event"),
    }
}

fn branch_head(connection: &Connection, branch_id: BranchId) -> CommitId {
    let branch_id = branch_id.raw_bytes();
    let bytes = connection
        .query_row(
            "SELECT head_commit_id FROM branch WHERE branch_id = ?1",
            params![&branch_id[..]],
            |row| row.get::<_, Vec<u8>>(0),
        )
        .expect("branch head");
    CommitId::from_bytes(bytes.try_into().expect("commit id bytes")).expect("commit id")
}

fn assert_relation_create_delta(before: &HistoryCounts, after: &HistoryCounts, created: i64) {
    assert_eq!(after.object_identity, before.object_identity + created);
    assert_eq!(after.entity, before.entity);
    assert_eq!(after.relation, before.relation + created);
    assert_eq!(after.relation_version, before.relation_version + created);
    assert_eq!(after.entity_version, before.entity_version);
    assert_eq!(after.changeset, before.changeset + created);
    assert_eq!(after.change_operation, before.change_operation + created);
    assert_eq!(
        after.entity_membership_change,
        before.entity_membership_change
    );
    assert_eq!(
        after.relation_membership_change,
        before.relation_membership_change + created
    );
    assert_eq!(after.workstate_commit, before.workstate_commit + created);
    assert_eq!(after.commit_parent, before.commit_parent + created);
    assert_eq!(after.event, before.event + created);
}

fn assert_counts_and_head_unchanged(
    path: &Path,
    before_counts: &HistoryCounts,
    branch_id: BranchId,
    before_head: CommitId,
) {
    let connection = raw_connection(path);
    assert_eq!(history_counts(&connection), *before_counts);
    assert_eq!(branch_head(&connection, branch_id), before_head);
}

fn create_goal_snapshot(
    engine: &mut Engine,
    workspace: &WorkspaceInfo,
    expected_head_commit_id: CommitId,
    description: &str,
) -> GoalSnapshot {
    let goal = engine
        .create_goal(
            GoalCreateOptions::new(
                workspace.initial_branch_id,
                expected_head_commit_id,
                description,
            )
            .expect("goal options"),
        )
        .expect("create goal");
    engine
        .goal_at(goal.commit_id, goal.goal_entity_id)
        .expect("goal snapshot")
}

fn create_plan_snapshot(
    engine: &mut Engine,
    workspace: &WorkspaceInfo,
    expected_head_commit_id: CommitId,
    description: &str,
) -> PlanSnapshot {
    let plan = engine
        .create_plan(
            PlanCreateOptions::new(
                workspace.initial_branch_id,
                expected_head_commit_id,
                description,
                "Keep the structure explicit",
            )
            .expect("plan options"),
        )
        .expect("create plan");
    engine
        .plan_at(plan.commit_id, plan.plan_entity_id)
        .expect("plan snapshot")
}

fn create_task_snapshot(
    engine: &mut Engine,
    workspace: &WorkspaceInfo,
    expected_head_commit_id: CommitId,
    description: &str,
) -> TaskSnapshot {
    let task = engine
        .create_task(
            TaskCreateOptions::new(
                workspace.initial_branch_id,
                expected_head_commit_id,
                description,
            )
            .expect("task options"),
        )
        .expect("create task");
    engine
        .task_at(task.commit_id, task.task_entity_id)
        .expect("task snapshot")
}

fn relation_for(
    relations: &[PrimaryContainmentSnapshot],
    parent_entity_id: EntityId,
    child_entity_id: EntityId,
) -> &PrimaryContainmentSnapshot {
    relations
        .iter()
        .find(|relation| {
            relation.parent_entity_id == parent_entity_id
                && relation.child_entity_id == child_entity_id
        })
        .expect("primary containment relation")
}

#[test]
fn goal_primary_containment_pairs_persist_and_replay_after_reopen() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let root_goal = create_goal_snapshot(
        &mut engine,
        &workspace,
        workspace.genesis_commit_id,
        "Deliver WorkVCS V0.1",
    );
    let child_goal = create_goal_snapshot(
        &mut engine,
        &workspace,
        root_goal.commit_id,
        "Make semantic structure queryable",
    );
    let plan = create_plan_snapshot(
        &mut engine,
        &workspace,
        child_goal.commit_id,
        "Phase 3 implementation plan",
    );
    let task = create_task_snapshot(
        &mut engine,
        &workspace,
        plan.commit_id,
        "Wire Goal containment endpoints",
    );
    let connection = raw_connection(&path);
    let before_counts = history_counts(&connection);
    let before_head = branch_head(&connection, workspace.initial_branch_id);

    let goal_to_goal = engine
        .create_primary_containment(
            PrimaryContainmentCreateOptions::new(
                workspace.initial_branch_id,
                task.commit_id,
                root_goal.goal_entity_id,
                child_goal.goal_entity_id,
            )
            .expect("goal to goal options"),
        )
        .expect("goal contains goal");
    let goal_to_plan = engine
        .create_primary_containment(
            PrimaryContainmentCreateOptions::new(
                workspace.initial_branch_id,
                goal_to_goal.commit_id,
                root_goal.goal_entity_id,
                plan.plan_entity_id,
            )
            .expect("goal to plan options"),
        )
        .expect("goal contains plan");
    let goal_to_task = engine
        .create_primary_containment(
            PrimaryContainmentCreateOptions::new(
                workspace.initial_branch_id,
                goal_to_plan.commit_id,
                child_goal.goal_entity_id,
                task.task_entity_id,
            )
            .expect("goal to task options"),
        )
        .expect("goal contains task");

    assert_eq!(before_head, task.commit_id);
    assert_eq!(
        goal_to_goal.parent_kind,
        PrimaryContainmentEndpointKind::Goal
    );
    assert_eq!(
        goal_to_goal.child_kind,
        PrimaryContainmentEndpointKind::Goal
    );
    assert_eq!(
        goal_to_plan.parent_kind,
        PrimaryContainmentEndpointKind::Goal
    );
    assert_eq!(
        goal_to_plan.child_kind,
        PrimaryContainmentEndpointKind::Plan
    );
    assert_eq!(
        goal_to_task.parent_kind,
        PrimaryContainmentEndpointKind::Goal
    );
    assert_eq!(
        goal_to_task.child_kind,
        PrimaryContainmentEndpointKind::Task
    );

    let after_counts = history_counts(&connection);
    assert_relation_create_delta(&before_counts, &after_counts, 3);
    assert_eq!(
        branch_head(&connection, workspace.initial_branch_id),
        goal_to_task.commit_id
    );

    let relations = engine
        .primary_containment_relations_at(goal_to_task.commit_id)
        .expect("primary containment relations");
    assert_eq!(relations.len(), 3);
    let goal_relation = relation_for(
        &relations,
        root_goal.goal_entity_id,
        child_goal.goal_entity_id,
    );
    assert_eq!(
        goal_relation.parent_kind,
        PrimaryContainmentEndpointKind::Goal
    );
    assert_eq!(
        goal_relation.child_kind,
        PrimaryContainmentEndpointKind::Goal
    );
    let plan_relation = relation_for(&relations, root_goal.goal_entity_id, plan.plan_entity_id);
    assert_eq!(
        plan_relation.parent_kind,
        PrimaryContainmentEndpointKind::Goal
    );
    assert_eq!(
        plan_relation.child_kind,
        PrimaryContainmentEndpointKind::Plan
    );
    let task_relation = relation_for(&relations, child_goal.goal_entity_id, task.task_entity_id);
    assert_eq!(
        task_relation.parent_kind,
        PrimaryContainmentEndpointKind::Goal
    );
    assert_eq!(
        task_relation.child_kind,
        PrimaryContainmentEndpointKind::Task
    );
    drop(connection);
    drop(engine);

    let reopened = Engine::open(&path).expect("reopen engine");
    assert_eq!(
        reopened
            .primary_containment_relations_at(goal_to_task.commit_id)
            .expect("reopened primary containment relations"),
        relations
    );
}

#[test]
fn plan_and_task_cannot_contain_goal_without_partial_rows() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let goal = create_goal_snapshot(
        &mut engine,
        &workspace,
        workspace.genesis_commit_id,
        "Keep Goal direction authoritative",
    );
    let plan = create_plan_snapshot(&mut engine, &workspace, goal.commit_id, "Plan");
    let task = create_task_snapshot(&mut engine, &workspace, plan.commit_id, "Task");
    let connection = raw_connection(&path);
    let before_counts = history_counts(&connection);
    let before_head = branch_head(&connection, workspace.initial_branch_id);

    let plan_to_goal = engine
        .create_primary_containment(
            PrimaryContainmentCreateOptions::new(
                workspace.initial_branch_id,
                task.commit_id,
                plan.plan_entity_id,
                goal.goal_entity_id,
            )
            .expect("plan to goal options"),
        )
        .expect_err("plan cannot contain goal");
    assert_eq!(plan_to_goal.code(), ErrorCode::RelationInvalid);
    assert_eq!(plan_to_goal.category(), ErrorCategory::Relation);
    assert_counts_and_head_unchanged(
        &path,
        &before_counts,
        workspace.initial_branch_id,
        before_head,
    );

    let task_to_goal = engine
        .create_primary_containment(
            PrimaryContainmentCreateOptions::new(
                workspace.initial_branch_id,
                task.commit_id,
                task.task_entity_id,
                goal.goal_entity_id,
            )
            .expect("task to goal options"),
        )
        .expect_err("task cannot contain goal");
    assert_eq!(task_to_goal.code(), ErrorCode::RelationInvalid);
    assert_eq!(task_to_goal.category(), ErrorCategory::Relation);
    assert_counts_and_head_unchanged(
        &path,
        &before_counts,
        workspace.initial_branch_id,
        before_head,
    );
}

#[test]
fn goal_parent_participates_in_single_primary_parent_rule() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let goal = create_goal_snapshot(
        &mut engine,
        &workspace,
        workspace.genesis_commit_id,
        "Own the visible work tree",
    );
    let plan = create_plan_snapshot(&mut engine, &workspace, goal.commit_id, "Plan parent");
    let task = create_task_snapshot(&mut engine, &workspace, plan.commit_id, "Task child");
    let first_parent = engine
        .create_primary_containment(
            PrimaryContainmentCreateOptions::new(
                workspace.initial_branch_id,
                task.commit_id,
                goal.goal_entity_id,
                task.task_entity_id,
            )
            .expect("goal parent options"),
        )
        .expect("goal parent");
    let connection = raw_connection(&path);
    let before_counts = history_counts(&connection);
    let before_head = branch_head(&connection, workspace.initial_branch_id);

    let duplicate_parent = engine
        .create_primary_containment(
            PrimaryContainmentCreateOptions::new(
                workspace.initial_branch_id,
                first_parent.commit_id,
                plan.plan_entity_id,
                task.task_entity_id,
            )
            .expect("plan duplicate parent options"),
        )
        .expect_err("task already has primary parent");
    assert_eq!(duplicate_parent.code(), ErrorCode::RelationInvalid);
    assert!(
        duplicate_parent
            .to_string()
            .contains("already has primary containment parent")
    );
    assert_counts_and_head_unchanged(
        &path,
        &before_counts,
        workspace.initial_branch_id,
        before_head,
    );
}

#[test]
fn goal_containment_cycle_is_rejected_without_partial_rows() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let first = create_goal_snapshot(
        &mut engine,
        &workspace,
        workspace.genesis_commit_id,
        "First goal",
    );
    let second = create_goal_snapshot(&mut engine, &workspace, first.commit_id, "Second goal");
    let third = create_goal_snapshot(&mut engine, &workspace, second.commit_id, "Third goal");
    let first_contains_second = engine
        .create_primary_containment(
            PrimaryContainmentCreateOptions::new(
                workspace.initial_branch_id,
                third.commit_id,
                first.goal_entity_id,
                second.goal_entity_id,
            )
            .expect("first edge options"),
        )
        .expect("first edge");
    let second_contains_third = engine
        .create_primary_containment(
            PrimaryContainmentCreateOptions::new(
                workspace.initial_branch_id,
                first_contains_second.commit_id,
                second.goal_entity_id,
                third.goal_entity_id,
            )
            .expect("second edge options"),
        )
        .expect("second edge");
    let connection = raw_connection(&path);
    let before_counts = history_counts(&connection);
    let before_head = branch_head(&connection, workspace.initial_branch_id);

    let cycle = engine
        .create_primary_containment(
            PrimaryContainmentCreateOptions::new(
                workspace.initial_branch_id,
                second_contains_third.commit_id,
                third.goal_entity_id,
                first.goal_entity_id,
            )
            .expect("cycle options"),
        )
        .expect_err("cycle should be rejected");
    assert_eq!(cycle.code(), ErrorCode::RelationInvalid);
    assert!(cycle.to_string().contains("containment cycle"));
    assert_counts_and_head_unchanged(
        &path,
        &before_counts,
        workspace.initial_branch_id,
        before_head,
    );
}
