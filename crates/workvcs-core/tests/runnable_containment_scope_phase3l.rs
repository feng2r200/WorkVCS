use rusqlite::{Connection, params};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use tempfile::TempDir;
use workvcs_core::{
    BranchId, CommitId, Engine, EntityId, ErrorCategory, ErrorCode, PlanCreateOptions,
    PlanSnapshot, PrimaryContainmentCreateCommit, PrimaryContainmentCreateOptions,
    RunnableTaskCandidate, RunnableTaskProjectionDimension, RunnableTasksOptions,
    SessionFocusOptions, SessionFocusPathEntry, SessionStartOptions, StoreInitOptions,
    TaskCreateOptions, TaskSchedulingRelationCreateOptions, TaskSnapshot, TaskStatus,
    TaskTransitionOptions, WorkspaceInfo, WorkspaceInitOptions,
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
        StoreInitOptions::new("phase3l-runnable-containment-store").expect("store options"),
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
                "Keep the work scoped",
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
    task_description: &str,
) -> TaskSnapshot {
    let task = engine
        .create_task(
            TaskCreateOptions::new(
                workspace.initial_branch_id,
                expected_head_commit_id,
                task_description,
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

fn candidate_entity_ids(candidates: &[RunnableTaskCandidate]) -> Vec<EntityId> {
    candidates
        .iter()
        .map(|candidate| candidate.task.task_entity_id)
        .collect()
}

fn assert_focused_dimensions(dimensions: &[RunnableTaskProjectionDimension]) {
    assert_eq!(
        dimensions,
        vec![
            RunnableTaskProjectionDimension::ExplicitManualOrder,
            RunnableTaskProjectionDimension::FinalEqualCandidateTieBreaker,
        ]
    );
}

fn assert_workspace_wide_dimensions(dimensions: &[RunnableTaskProjectionDimension]) {
    assert_eq!(
        dimensions,
        vec![
            RunnableTaskProjectionDimension::ActiveScopePlanPath,
            RunnableTaskProjectionDimension::ExecutableTaskDescendants,
            RunnableTaskProjectionDimension::ExplicitManualOrder,
            RunnableTaskProjectionDimension::FinalEqualCandidateTieBreaker,
        ]
    );
}

#[test]
fn plan_focus_limits_projection_to_primary_containment_task_descendants() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let root_plan = create_plan_snapshot(
        &mut engine,
        &workspace,
        workspace.genesis_commit_id,
        "Root execution plan",
    );
    let subplan = create_plan_snapshot(&mut engine, &workspace, root_plan.commit_id, "Subplan");
    let scoped_task =
        create_task_snapshot(&mut engine, &workspace, subplan.commit_id, "Scoped task");
    let subtask = create_task_snapshot(&mut engine, &workspace, scoped_task.commit_id, "Subtask");
    let outside_task =
        create_task_snapshot(&mut engine, &workspace, subtask.commit_id, "Outside task");
    let root_contains_subplan = contain(
        &mut engine,
        &workspace,
        outside_task.commit_id,
        root_plan.plan_entity_id,
        subplan.plan_entity_id,
    );
    let subplan_contains_task = contain(
        &mut engine,
        &workspace,
        root_contains_subplan.commit_id,
        subplan.plan_entity_id,
        scoped_task.task_entity_id,
    );
    let task_contains_subtask = contain(
        &mut engine,
        &workspace,
        subplan_contains_task.commit_id,
        scoped_task.task_entity_id,
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
            SessionFocusOptions::new(started.session_id, root_plan.plan_entity_id).with_path(vec![
                SessionFocusPathEntry::new(root_plan.plan_entity_id, None),
            ]),
        )
        .expect("set plan focus");
    let connection = raw_connection(&path);
    let before_counts = mutation_counts(&connection);
    let before_head = branch_head(&connection, workspace.initial_branch_id);

    let projection = engine
        .runnable_tasks(RunnableTasksOptions::new(started.session_id))
        .expect("focused runnable projection");

    assert_eq!(projection.head_commit_id, task_contains_subtask.commit_id);
    assert_focused_dimensions(&projection.deferred_dimensions);
    assert_eq!(
        candidate_entity_ids(&projection.candidates),
        vec![scoped_task.task_entity_id, subtask.task_entity_id]
    );
    assert!(
        !projection
            .candidates
            .iter()
            .any(|candidate| { candidate.task.task_entity_id == outside_task.task_entity_id })
    );
    assert_eq!(mutation_counts(&connection), before_counts);
    assert_eq!(
        branch_head(&connection, workspace.initial_branch_id),
        before_head
    );
}

#[test]
fn task_focus_includes_focused_task_and_primary_containment_task_descendants() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let plan = create_plan_snapshot(
        &mut engine,
        &workspace,
        workspace.genesis_commit_id,
        "Focused task plan",
    );
    let focused_task = create_task_snapshot(&mut engine, &workspace, plan.commit_id, "Parent task");
    let child_task = create_task_snapshot(
        &mut engine,
        &workspace,
        focused_task.commit_id,
        "Child task",
    );
    let outside_task = create_task_snapshot(
        &mut engine,
        &workspace,
        child_task.commit_id,
        "Outside task",
    );
    let plan_contains_task = contain(
        &mut engine,
        &workspace,
        outside_task.commit_id,
        plan.plan_entity_id,
        focused_task.task_entity_id,
    );
    let task_contains_child = contain(
        &mut engine,
        &workspace,
        plan_contains_task.commit_id,
        focused_task.task_entity_id,
        child_task.task_entity_id,
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
                    SessionFocusPathEntry::new(plan.plan_entity_id, None),
                    SessionFocusPathEntry::new(
                        focused_task.task_entity_id,
                        Some(plan_contains_task.relation_id),
                    ),
                ],
            ),
        )
        .expect("set task focus");

    let projection = engine
        .runnable_tasks(RunnableTasksOptions::new(started.session_id))
        .expect("focused runnable projection");

    assert_eq!(projection.head_commit_id, task_contains_child.commit_id);
    assert_focused_dimensions(&projection.deferred_dimensions);
    assert_eq!(
        candidate_entity_ids(&projection.candidates),
        vec![focused_task.task_entity_id, child_task.task_entity_id]
    );
    assert!(
        !projection
            .candidates
            .iter()
            .any(|candidate| { candidate.task.task_entity_id == outside_task.task_entity_id })
    );
}

#[test]
fn malformed_focus_path_is_rejected_without_projection_writes() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let plan = create_plan_snapshot(
        &mut engine,
        &workspace,
        workspace.genesis_commit_id,
        "Malformed path plan",
    );
    let focused_task = create_task_snapshot(&mut engine, &workspace, plan.commit_id, "Focus task");
    let later_task = create_task_snapshot(
        &mut engine,
        &workspace,
        focused_task.commit_id,
        "Later task",
    );
    let plan_contains_task = contain(
        &mut engine,
        &workspace,
        later_task.commit_id,
        plan.plan_entity_id,
        focused_task.task_entity_id,
    );
    let order = engine
        .create_task_scheduling_relation(
            TaskSchedulingRelationCreateOptions::ordered_before(
                workspace.initial_branch_id,
                plan_contains_task.commit_id,
                focused_task.task_entity_id,
                later_task.task_entity_id,
            )
            .expect("order options"),
        )
        .expect("create ordered_before");
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
                    SessionFocusPathEntry::new(plan.plan_entity_id, None),
                    SessionFocusPathEntry::new(
                        focused_task.task_entity_id,
                        Some(order.relation_id),
                    ),
                ],
            ),
        )
        .expect("set malformed focus path");
    let connection = raw_connection(&path);
    let before_counts = mutation_counts(&connection);
    let before_head = branch_head(&connection, workspace.initial_branch_id);

    let error = engine
        .runnable_tasks(RunnableTasksOptions::new(started.session_id))
        .expect_err("malformed focus path should be rejected");

    assert_eq!(error.code(), ErrorCode::SessionInvalid);
    assert_eq!(error.category(), ErrorCategory::Runtime);
    assert_eq!(mutation_counts(&connection), before_counts);
    assert_eq!(
        branch_head(&connection, workspace.initial_branch_id),
        before_head
    );
}

#[test]
fn no_focus_retains_workspace_wide_projection_and_scope_deferred_metadata() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let first = create_task_snapshot(
        &mut engine,
        &workspace,
        workspace.genesis_commit_id,
        "First workspace task",
    );
    let second = create_task_snapshot(&mut engine, &workspace, first.commit_id, "Second task");
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
        candidate_entity_ids(&projection.candidates),
        vec![first.task_entity_id, second.task_entity_id]
    );
}

#[test]
fn scoped_task_can_be_blocked_by_out_of_scope_dependency() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let plan = create_plan_snapshot(
        &mut engine,
        &workspace,
        workspace.genesis_commit_id,
        "Dependency scope plan",
    );
    let scoped_task = create_task_snapshot(&mut engine, &workspace, plan.commit_id, "Scoped task");
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
        plan.plan_entity_id,
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
            SessionFocusOptions::new(started.session_id, plan.plan_entity_id)
                .with_path(vec![SessionFocusPathEntry::new(plan.plan_entity_id, None)]),
        )
        .expect("set plan focus");

    let blocked = engine
        .runnable_tasks(RunnableTasksOptions::new(started.session_id))
        .expect("blocked scoped projection");
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
        .expect("unblocked scoped projection");
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
