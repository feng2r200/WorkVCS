use rusqlite::{Connection, params};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use tempfile::TempDir;
use workvcs_core::{
    BranchId, CommitId, Engine, EntityId, ErrorCategory, ErrorCode, EventId, GoalCreateOptions,
    GoalSnapshot, PlanCreateOptions, PlanSnapshot, PrimaryContainmentCreateCommit,
    PrimaryContainmentCreateOptions, StoreInitOptions, StructuralReferenceCreateCommit,
    StructuralReferenceCreateOptions, TaskCreateOptions, TaskSnapshot, WhyDeferredRelationFamily,
    WhyEntityKind, WhyQueryOptions, WhyQueryResult, WhyQueryTarget, WhyRelationDirection,
    WhyRelationEdge, WhyRelationKind, WorkspaceInfo, WorkspaceInitOptions,
};

#[derive(Clone, Debug, PartialEq, Eq)]
struct QueryCounts {
    branch_projection_state: i64,
    changeset: i64,
    change_operation: i64,
    workstate_commit: i64,
    relation: i64,
    relation_version: i64,
    entity_version: i64,
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
        StoreInitOptions::new("phase3t-why-structural-neighborhood-store").expect("store options"),
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

fn query_counts(connection: &Connection) -> QueryCounts {
    QueryCounts {
        branch_projection_state: count_rows(connection, "branch_projection_state"),
        changeset: count_rows(connection, "changeset"),
        change_operation: count_rows(connection, "change_operation"),
        workstate_commit: count_rows(connection, "workstate_commit"),
        relation: count_rows(connection, "relation"),
        relation_version: count_rows(connection, "relation_version"),
        entity_version: count_rows(connection, "entity_version"),
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
                "Keep why structural explanations explicit",
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

fn reference(
    engine: &mut Engine,
    workspace: &WorkspaceInfo,
    expected_head_commit_id: CommitId,
    referrer_entity_id: EntityId,
    target_entity_id: EntityId,
) -> StructuralReferenceCreateCommit {
    engine
        .create_structural_reference(
            StructuralReferenceCreateOptions::new(
                workspace.initial_branch_id,
                expected_head_commit_id,
                referrer_entity_id,
                target_entity_id,
            )
            .expect("reference options"),
        )
        .expect("create structural reference")
}

fn why_branch_head(
    engine: &Engine,
    branch_id: BranchId,
    subject_entity_id: EntityId,
) -> WhyQueryResult {
    engine
        .why(WhyQueryOptions::new(
            WhyQueryTarget::branch_head(branch_id),
            subject_entity_id,
        ))
        .expect("why branch head")
}

fn why_commit(engine: &Engine, commit_id: CommitId, subject_entity_id: EntityId) -> WhyQueryResult {
    engine
        .why(WhyQueryOptions::new(
            WhyQueryTarget::commit(commit_id),
            subject_entity_id,
        ))
        .expect("why commit")
}

fn edge_facts(
    why: &WhyQueryResult,
) -> BTreeSet<(
    WhyRelationKind,
    WhyRelationDirection,
    EntityId,
    WhyEntityKind,
    EntityId,
    WhyEntityKind,
)> {
    why.relation_edges
        .iter()
        .map(|edge| {
            (
                edge.relation_kind,
                edge.direction,
                edge.source_entity_id,
                edge.source_kind,
                edge.target_entity_id,
                edge.target_kind,
            )
        })
        .collect()
}

fn edge_order_key(
    edge: &WhyRelationEdge,
) -> (
    WhyRelationKind,
    WhyRelationDirection,
    WhyEntityKind,
    EntityId,
    WhyEntityKind,
    EntityId,
    workvcs_core::RelationId,
) {
    (
        edge.relation_kind,
        edge.direction,
        edge.source_kind,
        edge.source_entity_id,
        edge.target_kind,
        edge.target_entity_id,
        edge.relation_id,
    )
}

fn assert_edges_are_sorted(edges: &[WhyRelationEdge]) {
    for pair in edges.windows(2) {
        assert!(edge_order_key(&pair[0]) <= edge_order_key(&pair[1]));
    }
}

fn assert_deferred_families(why: &WhyQueryResult) {
    assert_eq!(
        why.deferred_relation_families,
        vec![
            WhyDeferredRelationFamily::Evolution,
            WhyDeferredRelationFamily::Epistemic,
            WhyDeferredRelationFamily::Verification,
        ]
    );
}

#[test]
fn why_reports_structural_neighborhood_for_plan_subject() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let goal = create_goal_snapshot(
        &mut engine,
        &workspace,
        workspace.genesis_commit_id,
        "Deliver WorkVCS V0.1",
    );
    let plan = create_plan_snapshot(&mut engine, &workspace, goal.commit_id, "Why query plan");
    let task = create_task_snapshot(&mut engine, &workspace, plan.commit_id, "Why query task");
    let goal_contains_plan = contain(
        &mut engine,
        &workspace,
        task.commit_id,
        goal.goal_entity_id,
        plan.plan_entity_id,
    );
    let plan_contains_task = contain(
        &mut engine,
        &workspace,
        goal_contains_plan.commit_id,
        plan.plan_entity_id,
        task.task_entity_id,
    );
    let plan_references_task = reference(
        &mut engine,
        &workspace,
        plan_contains_task.commit_id,
        plan.plan_entity_id,
        task.task_entity_id,
    );

    let why = why_branch_head(&engine, workspace.initial_branch_id, plan.plan_entity_id);

    assert_eq!(why.target.commit_id, plan_references_task.commit_id);
    assert_eq!(
        why.target.target,
        WhyQueryTarget::branch_head(workspace.initial_branch_id)
    );
    assert_eq!(why.target.workspace_id, workspace.workspace_id);
    assert_eq!(why.subject_entity_id, plan.plan_entity_id);
    assert_eq!(why.subject_entity_version_id, plan.plan_entity_version_id);
    assert_deferred_families(&why);
    assert_eq!(
        edge_facts(&why),
        BTreeSet::from([
            (
                WhyRelationKind::PrimaryContainment,
                WhyRelationDirection::Incoming,
                goal.goal_entity_id,
                WhyEntityKind::Goal,
                plan.plan_entity_id,
                WhyEntityKind::Plan,
            ),
            (
                WhyRelationKind::PrimaryContainment,
                WhyRelationDirection::Outgoing,
                plan.plan_entity_id,
                WhyEntityKind::Plan,
                task.task_entity_id,
                WhyEntityKind::Task,
            ),
            (
                WhyRelationKind::StructuralReference,
                WhyRelationDirection::Outgoing,
                plan.plan_entity_id,
                WhyEntityKind::Plan,
                task.task_entity_id,
                WhyEntityKind::Task,
            ),
        ])
    );
    assert_edges_are_sorted(&why.relation_edges);
}

#[test]
fn why_reports_incoming_references_for_task_subject_deterministically() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let goal = create_goal_snapshot(
        &mut engine,
        &workspace,
        workspace.genesis_commit_id,
        "Reference task from several scopes",
    );
    let first_plan = create_plan_snapshot(&mut engine, &workspace, goal.commit_id, "First plan");
    let second_plan =
        create_plan_snapshot(&mut engine, &workspace, first_plan.commit_id, "Second plan");
    let task = create_task_snapshot(
        &mut engine,
        &workspace,
        second_plan.commit_id,
        "Shared task",
    );
    let first_plan_contains_task = contain(
        &mut engine,
        &workspace,
        task.commit_id,
        first_plan.plan_entity_id,
        task.task_entity_id,
    );
    let goal_references_task = reference(
        &mut engine,
        &workspace,
        first_plan_contains_task.commit_id,
        goal.goal_entity_id,
        task.task_entity_id,
    );
    let first_plan_references_task = reference(
        &mut engine,
        &workspace,
        goal_references_task.commit_id,
        first_plan.plan_entity_id,
        task.task_entity_id,
    );
    reference(
        &mut engine,
        &workspace,
        first_plan_references_task.commit_id,
        second_plan.plan_entity_id,
        task.task_entity_id,
    );

    let why = why_branch_head(&engine, workspace.initial_branch_id, task.task_entity_id);

    assert_eq!(why.relation_edges.len(), 4);
    assert_eq!(why.subject_entity_id, task.task_entity_id);
    assert_eq!(why.subject_entity_version_id, task.task_entity_version_id);
    assert_deferred_families(&why);
    assert_eq!(
        edge_facts(&why),
        BTreeSet::from([
            (
                WhyRelationKind::PrimaryContainment,
                WhyRelationDirection::Incoming,
                first_plan.plan_entity_id,
                WhyEntityKind::Plan,
                task.task_entity_id,
                WhyEntityKind::Task,
            ),
            (
                WhyRelationKind::StructuralReference,
                WhyRelationDirection::Incoming,
                goal.goal_entity_id,
                WhyEntityKind::Goal,
                task.task_entity_id,
                WhyEntityKind::Task,
            ),
            (
                WhyRelationKind::StructuralReference,
                WhyRelationDirection::Incoming,
                first_plan.plan_entity_id,
                WhyEntityKind::Plan,
                task.task_entity_id,
                WhyEntityKind::Task,
            ),
            (
                WhyRelationKind::StructuralReference,
                WhyRelationDirection::Incoming,
                second_plan.plan_entity_id,
                WhyEntityKind::Plan,
                task.task_entity_id,
                WhyEntityKind::Task,
            ),
        ])
    );
    assert_edges_are_sorted(&why.relation_edges);
}

#[test]
fn why_commit_selector_reads_historical_structural_neighborhood() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let goal = create_goal_snapshot(
        &mut engine,
        &workspace,
        workspace.genesis_commit_id,
        "Historical why goal",
    );
    let plan = create_plan_snapshot(&mut engine, &workspace, goal.commit_id, "Historical plan");
    let task = create_task_snapshot(&mut engine, &workspace, plan.commit_id, "Historical task");
    let before = why_commit(&engine, task.commit_id, plan.plan_entity_id);
    let goal_contains_plan = contain(
        &mut engine,
        &workspace,
        task.commit_id,
        goal.goal_entity_id,
        plan.plan_entity_id,
    );

    let after = why_commit(&engine, goal_contains_plan.commit_id, plan.plan_entity_id);

    assert!(before.relation_edges.is_empty());
    assert_eq!(before.target.commit_id, task.commit_id);
    assert_eq!(after.target.commit_id, goal_contains_plan.commit_id);
    assert_eq!(after.relation_edges.len(), 1);
    assert_eq!(
        edge_facts(&after),
        BTreeSet::from([(
            WhyRelationKind::PrimaryContainment,
            WhyRelationDirection::Incoming,
            goal.goal_entity_id,
            WhyEntityKind::Goal,
            plan.plan_entity_id,
            WhyEntityKind::Plan,
        )])
    );
}

#[test]
fn why_is_read_only_and_ignores_projection_and_event_noise() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let goal = create_goal_snapshot(
        &mut engine,
        &workspace,
        workspace.genesis_commit_id,
        "Read-only why goal",
    );
    let plan = create_plan_snapshot(&mut engine, &workspace, goal.commit_id, "Read-only plan");
    let goal_contains_plan = contain(
        &mut engine,
        &workspace,
        plan.commit_id,
        goal.goal_entity_id,
        plan.plan_entity_id,
    );
    let connection = raw_connection(&path);
    let branch_id = workspace.initial_branch_id.raw_bytes();
    let genesis_commit_id = workspace.genesis_commit_id.raw_bytes();
    let event_id = EventId::new_v7().raw_bytes();
    connection
        .execute(
            "INSERT INTO branch_projection_state(
                branch_id,
                projection_status,
                projected_commit_id,
                projection_state_digest,
                updated_at_us
             )
             VALUES (?1, 'complete', ?2, ?3, 7)",
            params![&branch_id[..], &genesis_commit_id[..], &[8_u8; 32][..]],
        )
        .expect("insert projection noise");
    connection
        .execute(
            "INSERT INTO event(
                event_id,
                workspace_id,
                changeset_id,
                session_id,
                event_kind,
                occurred_at_us,
                payload_json
             )
             VALUES (?1, ?2, NULL, NULL, 'why.noise', 8, '{}')",
            params![&event_id[..], &workspace.workspace_id.raw_bytes()[..]],
        )
        .expect("insert event noise");
    let before_counts = query_counts(&connection);
    let before_head = branch_head(&connection, workspace.initial_branch_id);

    let why = why_branch_head(&engine, workspace.initial_branch_id, plan.plan_entity_id);

    assert_eq!(why.target.commit_id, goal_contains_plan.commit_id);
    assert_eq!(why.relation_edges.len(), 1);
    assert_eq!(query_counts(&connection), before_counts);
    assert_eq!(
        branch_head(&connection, workspace.initial_branch_id),
        before_head
    );
}

#[test]
fn why_rejects_noncurrent_subject_and_unknown_branch() {
    let (_tempdir, path) = store_path();
    let (mut engine, first_workspace) = create_workspace(&path);
    let second_workspace = engine
        .create_workspace(WorkspaceInitOptions::new("other-workspace").expect("workspace options"))
        .expect("create second workspace");
    let cross_workspace_plan = create_plan_snapshot(
        &mut engine,
        &second_workspace,
        second_workspace.genesis_commit_id,
        "Other workspace plan",
    );

    let noncurrent = engine
        .why(WhyQueryOptions::new(
            WhyQueryTarget::commit(first_workspace.genesis_commit_id),
            cross_workspace_plan.plan_entity_id,
        ))
        .expect_err("subject outside selected WorkState should be rejected");
    assert_eq!(noncurrent.code(), ErrorCode::QueryInvalid);
    assert_eq!(noncurrent.category(), ErrorCategory::Query);

    let unknown_branch = engine
        .why(WhyQueryOptions::new(
            WhyQueryTarget::branch_head(BranchId::new_v7()),
            cross_workspace_plan.plan_entity_id,
        ))
        .expect_err("unknown branch should be rejected");
    assert_eq!(unknown_branch.code(), ErrorCode::QueryInvalid);
    assert_eq!(unknown_branch.category(), ErrorCategory::Query);
}
