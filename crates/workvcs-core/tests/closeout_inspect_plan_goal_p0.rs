use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use tempfile::TempDir;
use workvcs_core::{
    CloseoutInspectCategory, CloseoutInspectNonExpandedCategory, CloseoutInspectOptions,
    CloseoutInspectSourceKind, CloseoutInspectStoreFileKind, CloseoutInspectTargetDigestStatus,
    CloseoutInspectTargetResolution, CommitId, Engine, EntityId, GoalCreateOptions, GoalSnapshot,
    PlanCreateOptions, PlanSnapshot, PrimaryContainmentCreateCommit,
    PrimaryContainmentCreateOptions, StoreInitOptions, TaskCreateOptions, TaskSnapshot,
    WorkspaceInfo, WorkspaceInitOptions,
};

#[derive(Debug, Clone, PartialEq, Eq)]
struct FileSnapshot {
    exists: bool,
    len: Option<u64>,
    modified: Option<SystemTime>,
}

fn store_path() -> (TempDir, PathBuf) {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let path = tempdir.path().join("workvcs.sqlite");
    (tempdir, path)
}

fn sqlite_sidecar_path(path: &Path, suffix: &str) -> PathBuf {
    let mut raw = OsString::from(path.as_os_str());
    raw.push(suffix);
    PathBuf::from(raw)
}

fn sqlite_file_paths(path: &Path) -> [PathBuf; 3] {
    [
        path.to_path_buf(),
        sqlite_sidecar_path(path, "-wal"),
        sqlite_sidecar_path(path, "-shm"),
    ]
}

fn file_snapshot(path: &Path) -> FileSnapshot {
    match std::fs::metadata(path) {
        Ok(metadata) => FileSnapshot {
            exists: true,
            len: Some(metadata.len()),
            modified: Some(metadata.modified().expect("modified time")),
        },
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => FileSnapshot {
            exists: false,
            len: None,
            modified: None,
        },
        Err(error) => panic!("snapshot {path:?}: {error}"),
    }
}

fn sqlite_file_snapshots(path: &Path) -> Vec<(PathBuf, FileSnapshot)> {
    sqlite_file_paths(path)
        .into_iter()
        .map(|path| {
            let snapshot = file_snapshot(&path);
            (path, snapshot)
        })
        .collect()
}

fn init_engine(path: &Path) -> Engine {
    Engine::init(
        path,
        StoreInitOptions::new("closeout-plan-goal-p0-store").expect("store options"),
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

fn create_goal(
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

fn create_plan(
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
                "Keep direct children inspectable",
            )
            .expect("plan options"),
        )
        .expect("create plan");
    engine
        .plan_at(plan.commit_id, plan.plan_entity_id)
        .expect("plan snapshot")
}

fn create_task(
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
        .expect("create containment")
}

#[test]
fn plan_projection_lists_direct_tasks_and_counts_child_plans_without_recursing() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let parent_plan = create_plan(
        &mut engine,
        &workspace,
        workspace.genesis_commit_id,
        "Parent",
    );
    let child_plan = create_plan(&mut engine, &workspace, parent_plan.commit_id, "Child plan");
    let nested_task = create_task(&mut engine, &workspace, child_plan.commit_id, "Nested task");
    let direct_task = create_task(
        &mut engine,
        &workspace,
        nested_task.commit_id,
        "Direct task",
    );
    let plan_to_plan = contain(
        &mut engine,
        &workspace,
        direct_task.commit_id,
        parent_plan.plan_entity_id,
        child_plan.plan_entity_id,
    );
    let child_plan_to_task = contain(
        &mut engine,
        &workspace,
        plan_to_plan.commit_id,
        child_plan.plan_entity_id,
        nested_task.task_entity_id,
    );
    let plan_to_task = contain(
        &mut engine,
        &workspace,
        child_plan_to_task.commit_id,
        parent_plan.plan_entity_id,
        direct_task.task_entity_id,
    );
    drop(engine);

    let readonly = Engine::open_readonly(&path).expect("open readonly");
    let projection = readonly
        .closeout_inspect_plan(CloseoutInspectOptions::for_plan_on_branch(
            workspace.initial_branch_id,
            parent_plan.plan_entity_id,
        ))
        .expect("inspect plan");

    assert_eq!(projection.source.kind, CloseoutInspectSourceKind::Branch);
    assert_eq!(projection.source.commit_id, plan_to_task.commit_id);
    assert_eq!(
        projection.target_resolution,
        CloseoutInspectTargetResolution::Found
    );
    assert!(projection.read_proof.stable);
    assert!(!projection.truncated);
    assert_eq!(
        projection.plan.as_ref().expect("plan").plan_entity_id,
        parent_plan.plan_entity_id
    );
    assert_eq!(projection.counts.direct_tasks_total, 1);
    assert_eq!(projection.counts.direct_child_plans_total, 1);
    assert_eq!(projection.direct_tasks.len(), 1);
    assert_eq!(
        projection.direct_tasks[0].task.task_entity_id,
        direct_task.task_entity_id
    );
    assert_eq!(
        projection.direct_tasks[0].containment.relation_id,
        plan_to_task.relation_id
    );
    assert!(
        projection
            .direct_tasks
            .iter()
            .all(|item| item.task.task_entity_id != nested_task.task_entity_id),
        "child Plan tasks must not be pulled into the parent Plan projection"
    );
    assert_eq!(projection.non_expanded.len(), 1);
    assert_eq!(
        projection.non_expanded[0].category,
        CloseoutInspectNonExpandedCategory::DirectChildPlans
    );
    assert_eq!(projection.non_expanded[0].count, 1);
    assert!(projection.omitted.is_empty());

    let serialized = serde_json::to_value(&projection).expect("serialize plan projection");
    assert!(serialized.get("read_proof").is_some());
    assert_eq!(
        serialized["non_expanded"][0]["category"],
        serde_json::Value::String("direct_child_plans".to_owned())
    );
    assert!(
        serialized.get("ready").is_none(),
        "mechanical projection must not judge readiness"
    );
    assert!(
        serialized.get("complete").is_none(),
        "mechanical projection must not judge completion"
    );
    assert!(
        serialized.get("authorized").is_none(),
        "mechanical projection must not judge authorization"
    );
}

#[test]
fn goal_projection_lists_direct_plans_and_tasks_counts_child_goals_without_recursing() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let root_goal = create_goal(&mut engine, &workspace, workspace.genesis_commit_id, "Root");
    let direct_plan = create_plan(&mut engine, &workspace, root_goal.commit_id, "Direct plan");
    let direct_task = create_task(
        &mut engine,
        &workspace,
        direct_plan.commit_id,
        "Direct task",
    );
    let child_goal = create_goal(&mut engine, &workspace, direct_task.commit_id, "Child goal");
    let nested_plan_task = create_task(
        &mut engine,
        &workspace,
        child_goal.commit_id,
        "Nested plan task",
    );
    let child_goal_task = create_task(
        &mut engine,
        &workspace,
        nested_plan_task.commit_id,
        "Child goal task",
    );
    let goal_to_plan = contain(
        &mut engine,
        &workspace,
        child_goal_task.commit_id,
        root_goal.goal_entity_id,
        direct_plan.plan_entity_id,
    );
    let plan_to_task = contain(
        &mut engine,
        &workspace,
        goal_to_plan.commit_id,
        direct_plan.plan_entity_id,
        nested_plan_task.task_entity_id,
    );
    let goal_to_task = contain(
        &mut engine,
        &workspace,
        plan_to_task.commit_id,
        root_goal.goal_entity_id,
        direct_task.task_entity_id,
    );
    let goal_to_goal = contain(
        &mut engine,
        &workspace,
        goal_to_task.commit_id,
        root_goal.goal_entity_id,
        child_goal.goal_entity_id,
    );
    let child_goal_to_task = contain(
        &mut engine,
        &workspace,
        goal_to_goal.commit_id,
        child_goal.goal_entity_id,
        child_goal_task.task_entity_id,
    );
    drop(engine);

    let readonly = Engine::open_readonly(&path).expect("open readonly");
    let projection = readonly
        .closeout_inspect_goal(CloseoutInspectOptions::for_goal_on_branch(
            workspace.initial_branch_id,
            root_goal.goal_entity_id,
        ))
        .expect("inspect goal");

    assert_eq!(projection.source.commit_id, child_goal_to_task.commit_id);
    assert_eq!(
        projection.target_resolution,
        CloseoutInspectTargetResolution::Found
    );
    assert!(projection.read_proof.stable);
    assert_eq!(
        projection.goal.as_ref().expect("goal").goal_entity_id,
        root_goal.goal_entity_id
    );
    assert_eq!(projection.counts.direct_plans_total, 1);
    assert_eq!(projection.counts.direct_tasks_total, 1);
    assert_eq!(projection.counts.direct_child_goals_total, 1);
    assert_eq!(projection.direct_plans.len(), 1);
    assert_eq!(
        projection.direct_plans[0].plan.plan_entity_id,
        direct_plan.plan_entity_id
    );
    assert_eq!(
        projection.direct_plans[0].containment.relation_id,
        goal_to_plan.relation_id
    );
    assert_eq!(projection.direct_tasks.len(), 1);
    assert_eq!(
        projection.direct_tasks[0].task.task_entity_id,
        direct_task.task_entity_id
    );
    assert!(
        projection
            .direct_tasks
            .iter()
            .all(
                |item| item.task.task_entity_id != nested_plan_task.task_entity_id
                    && item.task.task_entity_id != child_goal_task.task_entity_id
            ),
        "Goal projection must not recurse into direct Plans or child Goals"
    );
    assert_eq!(
        projection.non_expanded[0].category,
        CloseoutInspectNonExpandedCategory::DirectChildGoals
    );
    assert_eq!(projection.non_expanded[0].count, 1);
    assert!(!projection.truncated);
}

#[test]
fn plan_budget_is_stable_and_explicit_commit_differs_from_branch_head() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let plan = create_plan(
        &mut engine,
        &workspace,
        workspace.genesis_commit_id,
        "Budget plan",
    );
    let first = create_task(&mut engine, &workspace, plan.commit_id, "First");
    let second = create_task(&mut engine, &workspace, first.commit_id, "Second");
    let third = create_task(&mut engine, &workspace, second.commit_id, "Third");
    let first_edge = contain(
        &mut engine,
        &workspace,
        third.commit_id,
        plan.plan_entity_id,
        first.task_entity_id,
    );
    let second_edge = contain(
        &mut engine,
        &workspace,
        first_edge.commit_id,
        plan.plan_entity_id,
        second.task_entity_id,
    );
    contain(
        &mut engine,
        &workspace,
        second_edge.commit_id,
        plan.plan_entity_id,
        third.task_entity_id,
    );
    drop(engine);

    let readonly = Engine::open_readonly(&path).expect("open readonly");
    let historical = readonly
        .closeout_inspect_plan(CloseoutInspectOptions::for_plan_at_commit(
            plan.commit_id,
            plan.plan_entity_id,
        ))
        .expect("inspect historical plan");
    let current = readonly
        .closeout_inspect_plan(
            CloseoutInspectOptions::for_plan_on_branch(
                workspace.initial_branch_id,
                plan.plan_entity_id,
            )
            .with_budget(2)
            .expect("budget"),
        )
        .expect("inspect branch plan");

    assert_eq!(historical.source.kind, CloseoutInspectSourceKind::Commit);
    assert_eq!(historical.source.commit_id, plan.commit_id);
    assert!(historical.read_proof.before.branch_head.is_none());
    assert_eq!(historical.counts.direct_tasks_total, 0);
    assert!(historical.direct_tasks.is_empty());

    assert_eq!(current.source.kind, CloseoutInspectSourceKind::Branch);
    assert_eq!(current.counts.direct_tasks_total, 3);
    assert!(current.truncated);
    let mut expected_ids = vec![
        first.task_entity_id,
        second.task_entity_id,
        third.task_entity_id,
    ];
    expected_ids.sort();
    assert_eq!(
        current
            .direct_tasks
            .iter()
            .map(|item| item.task.task_entity_id)
            .collect::<Vec<_>>(),
        expected_ids.into_iter().take(2).collect::<Vec<_>>()
    );
    assert_eq!(current.omitted.len(), 1);
    assert_eq!(
        current.omitted[0].category,
        CloseoutInspectCategory::DirectTasks
    );
    assert_eq!(current.omitted[0].count, 1);

    let serialized = serde_json::to_value(&current).expect("serialize budgeted projection");
    assert_eq!(serialized["truncated"], serde_json::Value::Bool(true));
    assert_eq!(
        serialized["omitted"][0]["category"],
        serde_json::Value::String("direct_tasks".to_owned())
    );
    assert!(serialized.get("read_proof").is_some());
}

#[test]
fn plan_and_goal_resolution_distinguish_not_found_from_wrong_kind() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let task = create_task(
        &mut engine,
        &workspace,
        workspace.genesis_commit_id,
        "Wrong kind target",
    );
    drop(engine);

    let readonly = Engine::open_readonly(&path).expect("open readonly");
    let wrong_kind = readonly
        .closeout_inspect_plan(CloseoutInspectOptions::for_plan_on_branch(
            workspace.initial_branch_id,
            task.task_entity_id,
        ))
        .expect("wrong-kind plan target projects");
    assert_eq!(
        wrong_kind.target_resolution,
        CloseoutInspectTargetResolution::WrongKind
    );
    assert!(wrong_kind.plan.is_none());
    assert_eq!(wrong_kind.gaps[0].code, "target_wrong_kind");
    assert_eq!(
        wrong_kind.read_proof.before.target_digest_status,
        CloseoutInspectTargetDigestStatus::WrongKind
    );
    assert_eq!(wrong_kind.read_proof.before.target_state_digest, None);

    let not_found = readonly
        .closeout_inspect_goal(CloseoutInspectOptions::for_goal_on_branch(
            workspace.initial_branch_id,
            EntityId::new_v7(),
        ))
        .expect("missing goal target projects");
    assert_eq!(
        not_found.target_resolution,
        CloseoutInspectTargetResolution::TargetNotFound
    );
    assert!(not_found.goal.is_none());
    assert_eq!(not_found.gaps[0].code, "target_not_found");
    assert_eq!(
        not_found.read_proof.before.target_digest_status,
        CloseoutInspectTargetDigestStatus::TargetNotFound
    );
}

#[test]
fn readonly_plan_inspect_preserves_head_state_digest_and_sqlite_metadata() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let plan = create_plan(
        &mut engine,
        &workspace,
        workspace.genesis_commit_id,
        "Readonly plan",
    );
    drop(engine);
    let before_files = sqlite_file_snapshots(&path);

    let readonly = Engine::open_readonly(&path).expect("open readonly");
    let before_head = readonly
        .branch_head(workspace.initial_branch_id)
        .expect("branch head before inspect");
    let before_state = readonly
        .state_at(before_head.head_commit_id)
        .expect("state before inspect");
    let projection = readonly
        .closeout_inspect_plan(CloseoutInspectOptions::for_plan_on_branch(
            workspace.initial_branch_id,
            plan.plan_entity_id,
        ))
        .expect("inspect plan");

    assert_eq!(projection.source.state_digest, before_state.state_digest);
    assert!(projection.read_proof.stable);
    assert!(projection.read_proof.drift.is_empty());
    assert_eq!(
        projection
            .read_proof
            .before
            .branch_head
            .as_ref()
            .expect("branch proof before")
            .head_commit_id,
        before_head.head_commit_id
    );
    assert_eq!(
        projection.read_proof.before.target_state_digest,
        projection.plan.as_ref().map(|plan| plan.state_digest)
    );
    assert_eq!(projection.read_proof.before.store_files.len(), 3);
    assert_eq!(
        projection
            .read_proof
            .before
            .store_files
            .iter()
            .map(|metadata| metadata.kind)
            .collect::<Vec<_>>(),
        vec![
            CloseoutInspectStoreFileKind::Main,
            CloseoutInspectStoreFileKind::Wal,
            CloseoutInspectStoreFileKind::Shm,
        ]
    );

    let after_head = readonly
        .branch_head(workspace.initial_branch_id)
        .expect("branch head after inspect");
    let after_state = readonly
        .state_at(after_head.head_commit_id)
        .expect("state after inspect");
    assert_eq!(after_head, before_head);
    assert_eq!(after_state.state_digest, before_state.state_digest);
    drop(readonly);

    let after_files = sqlite_file_snapshots(&path);
    assert_eq!(after_files, before_files);
}
