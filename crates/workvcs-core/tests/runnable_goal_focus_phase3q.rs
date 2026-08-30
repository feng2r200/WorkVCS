use rusqlite::{Connection, params};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use tempfile::TempDir;
use workvcs_core::{
    BranchId, CommitId, Engine, EntityId, ErrorCategory, ErrorCode, GoalCreateOptions,
    GoalSnapshot, PlanCreateOptions, PlanSnapshot, PrimaryContainmentCreateCommit,
    PrimaryContainmentCreateOptions, RunnableTaskCandidate, RunnableTaskProjectionDimension,
    RunnableTasksOptions, SessionFocusOptions, SessionFocusPathEntry, SessionStartOptions,
    StoreInitOptions, TaskCreateOptions, TaskSchedulingRelationCreateOptions, TaskSnapshot,
    TaskStatus, TaskTransitionOptions, WorkspaceInfo, WorkspaceInitOptions,
};

#[derive(Clone, Debug, PartialEq, Eq)]
struct MutationCounts {
    changeset: i64,
    change_operation: i64,
    workstate_commit: i64,
    relation: i64,
    relation_version: i64,
    session_focus: i64,
    session_focus_path: i64,
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
        StoreInitOptions::new("phase3q-goal-focus-store").expect("store options"),
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

fn mutation_counts(connection: &Connection) -> MutationCounts {
    MutationCounts {
        changeset: count_rows(connection, "changeset"),
        change_operation: count_rows(connection, "change_operation"),
        workstate_commit: count_rows(connection, "workstate_commit"),
        relation: count_rows(connection, "relation"),
        relation_version: count_rows(connection, "relation_version"),
        session_focus: count_rows(connection, "session_focus"),
        session_focus_path: count_rows(connection, "session_focus_path"),
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
                "Keep scoped runnable work explicit",
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

fn contain(
    engine: &mut Engine,
    workspace: &WorkspaceInfo,
    expected_head_commit_id: CommitId,
    parent_entity_id: EntityId,
    child_entity_id: EntityId,
) -> PrimaryContainmentCreateCommit {
    engine
        .create_primary_containment(
            PrimaryContainmentCreateOptions::new(
                workspace.initial_branch_id,
                expected_head_commit_id,
                parent_entity_id,
                child_entity_id,
            )
            .expect("containment options"),
        )
        .expect("create primary containment")
}

fn complete_task(
    engine: &mut Engine,
    workspace: &WorkspaceInfo,
    expected_head_commit_id: CommitId,
    task: &TaskSnapshot,
) -> TaskSnapshot {
    let completed = engine
        .transition_task(
            TaskTransitionOptions::new(
                workspace.initial_branch_id,
                expected_head_commit_id,
                task.task_entity_id,
                task.task_entity_version_id,
                TaskStatus::Done,
            )
            .expect("transition options"),
        )
        .expect("complete task");
    engine
        .task_at(completed.commit_id, completed.task_entity_id)
        .expect("completed task")
}

fn candidates_by_entity_id(
    candidates: &[RunnableTaskCandidate],
) -> BTreeMap<EntityId, &RunnableTaskCandidate> {
    candidates
        .iter()
        .map(|candidate| (candidate.task.task_entity_id, candidate))
        .collect()
}

fn candidate_entity_id_set(candidates: &[RunnableTaskCandidate]) -> BTreeSet<EntityId> {
    candidates
        .iter()
        .map(|candidate| candidate.task.task_entity_id)
        .collect()
}

fn assert_focused_dimensions(dimensions: &[RunnableTaskProjectionDimension]) {
    assert!(dimensions.is_empty());
}

fn assert_workspace_wide_dimensions(dimensions: &[RunnableTaskProjectionDimension]) {
    assert_eq!(
        dimensions,
        vec![
            RunnableTaskProjectionDimension::ActiveScopePlanPath,
            RunnableTaskProjectionDimension::ExecutableTaskDescendants,
        ]
    );
}

#[test]
fn goal_focus_returns_task_descendants_across_goal_plan_and_task_edges_without_projection_writes() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let root_goal = create_goal_snapshot(
        &mut engine,
        &workspace,
        workspace.genesis_commit_id,
        "Deliver WorkVCS V0.1",
    );
    let nested_goal = create_goal_snapshot(
        &mut engine,
        &workspace,
        root_goal.commit_id,
        "Make scoped work executable",
    );
    let plan = create_plan_snapshot(
        &mut engine,
        &workspace,
        nested_goal.commit_id,
        "Runnable scope plan",
    );
    let plan_task =
        create_task_snapshot(&mut engine, &workspace, plan.commit_id, "Plan scoped task");
    let direct_goal_task = create_task_snapshot(
        &mut engine,
        &workspace,
        plan_task.commit_id,
        "Direct goal task",
    );
    let subtask = create_task_snapshot(
        &mut engine,
        &workspace,
        direct_goal_task.commit_id,
        "Direct goal subtask",
    );
    let outside_task =
        create_task_snapshot(&mut engine, &workspace, subtask.commit_id, "Outside task");
    let root_contains_nested_goal = contain(
        &mut engine,
        &workspace,
        outside_task.commit_id,
        root_goal.goal_entity_id,
        nested_goal.goal_entity_id,
    );
    let nested_goal_contains_plan = contain(
        &mut engine,
        &workspace,
        root_contains_nested_goal.commit_id,
        nested_goal.goal_entity_id,
        plan.plan_entity_id,
    );
    let plan_contains_task = contain(
        &mut engine,
        &workspace,
        nested_goal_contains_plan.commit_id,
        plan.plan_entity_id,
        plan_task.task_entity_id,
    );
    let root_goal_contains_task = contain(
        &mut engine,
        &workspace,
        plan_contains_task.commit_id,
        root_goal.goal_entity_id,
        direct_goal_task.task_entity_id,
    );
    let task_contains_subtask = contain(
        &mut engine,
        &workspace,
        root_goal_contains_task.commit_id,
        direct_goal_task.task_entity_id,
        subtask.task_entity_id,
    );
    let started = engine
        .start_session(
            SessionStartOptions::new(workspace.workspace_id, workspace.initial_branch_id)
                .expect("session options"),
        )
        .expect("start session");
    engine
        .set_session_focus(
            SessionFocusOptions::new(started.session_id, root_goal.goal_entity_id).with_path(vec![
                SessionFocusPathEntry::new(root_goal.goal_entity_id, None),
            ]),
        )
        .expect("set goal focus");
    let connection = raw_connection(&path);
    let before_counts = mutation_counts(&connection);
    let before_head = branch_head(&connection, workspace.initial_branch_id);

    let projection = engine
        .runnable_tasks(RunnableTasksOptions::new(started.session_id))
        .expect("goal focused runnable projection");

    assert_eq!(projection.head_commit_id, task_contains_subtask.commit_id);
    assert_focused_dimensions(&projection.deferred_dimensions);
    assert_eq!(
        candidate_entity_id_set(&projection.candidates),
        BTreeSet::from([
            plan_task.task_entity_id,
            direct_goal_task.task_entity_id,
            subtask.task_entity_id,
        ])
    );
    assert!(
        !projection
            .candidates
            .iter()
            .any(|candidate| candidate.task.task_entity_id == outside_task.task_entity_id)
    );
    assert_eq!(mutation_counts(&connection), before_counts);
    assert_eq!(
        branch_head(&connection, workspace.initial_branch_id),
        before_head
    );
}

#[test]
fn goal_rooted_focus_path_validates_primary_containment_chain_to_task_focus() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let goal = create_goal_snapshot(
        &mut engine,
        &workspace,
        workspace.genesis_commit_id,
        "Focused goal",
    );
    let plan = create_plan_snapshot(&mut engine, &workspace, goal.commit_id, "Focused plan");
    let focused_task =
        create_task_snapshot(&mut engine, &workspace, plan.commit_id, "Focused task");
    let outside_task = create_task_snapshot(
        &mut engine,
        &workspace,
        focused_task.commit_id,
        "Outside task",
    );
    let goal_contains_plan = contain(
        &mut engine,
        &workspace,
        outside_task.commit_id,
        goal.goal_entity_id,
        plan.plan_entity_id,
    );
    let plan_contains_task = contain(
        &mut engine,
        &workspace,
        goal_contains_plan.commit_id,
        plan.plan_entity_id,
        focused_task.task_entity_id,
    );
    let started = engine
        .start_session(
            SessionStartOptions::new(workspace.workspace_id, workspace.initial_branch_id)
                .expect("session options"),
        )
        .expect("start session");
    engine
        .set_session_focus(
            SessionFocusOptions::new(started.session_id, focused_task.task_entity_id).with_path(
                vec![
                    SessionFocusPathEntry::new(goal.goal_entity_id, None),
                    SessionFocusPathEntry::new(
                        plan.plan_entity_id,
                        Some(goal_contains_plan.relation_id),
                    ),
                    SessionFocusPathEntry::new(
                        focused_task.task_entity_id,
                        Some(plan_contains_task.relation_id),
                    ),
                ],
            ),
        )
        .expect("set task focus under goal path");

    let projection = engine
        .runnable_tasks(RunnableTasksOptions::new(started.session_id))
        .expect("goal rooted task focus projection");

    assert_focused_dimensions(&projection.deferred_dimensions);
    assert_eq!(
        candidate_entity_id_set(&projection.candidates),
        BTreeSet::from([focused_task.task_entity_id])
    );
    assert!(
        !projection
            .candidates
            .iter()
            .any(|candidate| candidate.task.task_entity_id == outside_task.task_entity_id)
    );
}

#[test]
fn malformed_goal_focus_path_is_rejected_without_projection_writes() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let goal = create_goal_snapshot(
        &mut engine,
        &workspace,
        workspace.genesis_commit_id,
        "Malformed goal path",
    );
    let plan = create_plan_snapshot(&mut engine, &workspace, goal.commit_id, "Focused plan");
    let other_task = create_task_snapshot(&mut engine, &workspace, plan.commit_id, "Other task");
    let goal_contains_other_task = contain(
        &mut engine,
        &workspace,
        other_task.commit_id,
        goal.goal_entity_id,
        other_task.task_entity_id,
    );
    let started = engine
        .start_session(
            SessionStartOptions::new(workspace.workspace_id, workspace.initial_branch_id)
                .expect("session options"),
        )
        .expect("start session");
    engine
        .set_session_focus(
            SessionFocusOptions::new(started.session_id, plan.plan_entity_id).with_path(vec![
                SessionFocusPathEntry::new(goal.goal_entity_id, None),
                SessionFocusPathEntry::new(
                    plan.plan_entity_id,
                    Some(goal_contains_other_task.relation_id),
                ),
            ]),
        )
        .expect("set malformed goal focus path");
    let connection = raw_connection(&path);
    let before_counts = mutation_counts(&connection);
    let before_head = branch_head(&connection, workspace.initial_branch_id);

    let error = engine
        .runnable_tasks(RunnableTasksOptions::new(started.session_id))
        .expect_err("malformed goal rooted focus path should be rejected");

    assert_eq!(error.code(), ErrorCode::SessionInvalid);
    assert_eq!(error.category(), ErrorCategory::Runtime);
    assert_eq!(mutation_counts(&connection), before_counts);
    assert_eq!(
        branch_head(&connection, workspace.initial_branch_id),
        before_head
    );
}

#[test]
fn goal_scoped_task_can_be_blocked_by_out_of_scope_dependency() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let goal = create_goal_snapshot(
        &mut engine,
        &workspace,
        workspace.genesis_commit_id,
        "Dependency scoped goal",
    );
    let scoped_task = create_task_snapshot(&mut engine, &workspace, goal.commit_id, "Scoped task");
    let outside_prerequisite = create_task_snapshot(
        &mut engine,
        &workspace,
        scoped_task.commit_id,
        "Outside prerequisite",
    );
    let contains = contain(
        &mut engine,
        &workspace,
        outside_prerequisite.commit_id,
        goal.goal_entity_id,
        scoped_task.task_entity_id,
    );
    let dependency = engine
        .create_task_scheduling_relation(
            TaskSchedulingRelationCreateOptions::depends_on(
                workspace.initial_branch_id,
                contains.commit_id,
                scoped_task.task_entity_id,
                outside_prerequisite.task_entity_id,
            )
            .expect("dependency options"),
        )
        .expect("create dependency");
    let started = engine
        .start_session(
            SessionStartOptions::new(workspace.workspace_id, workspace.initial_branch_id)
                .expect("session options"),
        )
        .expect("start session");
    engine
        .set_session_focus(
            SessionFocusOptions::new(started.session_id, goal.goal_entity_id)
                .with_path(vec![SessionFocusPathEntry::new(goal.goal_entity_id, None)]),
        )
        .expect("set goal focus");

    let blocked = engine
        .runnable_tasks(RunnableTasksOptions::new(started.session_id))
        .expect("blocked goal scoped projection");
    let blocked_candidates = candidates_by_entity_id(&blocked.candidates);
    let scoped_candidate = blocked_candidates
        .get(&scoped_task.task_entity_id)
        .expect("scoped candidate");
    assert_eq!(blocked.head_commit_id, dependency.commit_id);
    assert_eq!(blocked.candidates.len(), 1);
    assert!(!blocked_candidates.contains_key(&outside_prerequisite.task_entity_id));
    assert!(!scoped_candidate.runnable);
    assert!(!scoped_candidate.dependency_ready);
    assert_eq!(
        scoped_candidate.unsatisfied_dependency_entity_ids,
        vec![outside_prerequisite.task_entity_id]
    );

    let completed_prerequisite = complete_task(
        &mut engine,
        &workspace,
        dependency.commit_id,
        &outside_prerequisite,
    );
    let unblocked = engine
        .runnable_tasks(RunnableTasksOptions::new(started.session_id))
        .expect("unblocked goal scoped projection");
    let unblocked_candidates = candidates_by_entity_id(&unblocked.candidates);
    let scoped_candidate = unblocked_candidates
        .get(&scoped_task.task_entity_id)
        .expect("scoped candidate after prerequisite completion");
    assert_eq!(unblocked.head_commit_id, completed_prerequisite.commit_id);
    assert_eq!(unblocked.candidates.len(), 1);
    assert!(scoped_candidate.runnable);
    assert!(scoped_candidate.dependency_ready);
    assert_eq!(
        scoped_candidate.unsatisfied_dependency_entity_ids,
        Vec::new()
    );
}

#[test]
fn no_focus_remains_workspace_wide_after_goal_focus_support() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let goal = create_goal_snapshot(
        &mut engine,
        &workspace,
        workspace.genesis_commit_id,
        "No focus goal",
    );
    let first_task = create_task_snapshot(&mut engine, &workspace, goal.commit_id, "First task");
    let second_task =
        create_task_snapshot(&mut engine, &workspace, first_task.commit_id, "Second task");
    let started = engine
        .start_session(
            SessionStartOptions::new(workspace.workspace_id, workspace.initial_branch_id)
                .expect("session options"),
        )
        .expect("start session");

    let projection = engine
        .runnable_tasks(RunnableTasksOptions::new(started.session_id))
        .expect("workspace-wide projection");

    assert_workspace_wide_dimensions(&projection.deferred_dimensions);
    assert_eq!(
        candidate_entity_id_set(&projection.candidates),
        BTreeSet::from([first_task.task_entity_id, second_task.task_entity_id])
    );
}
