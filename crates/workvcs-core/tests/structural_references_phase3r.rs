use rusqlite::{Connection, params};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use tempfile::TempDir;
use workvcs_core::{
    BranchId, CommitId, Engine, EntityId, ErrorCategory, ErrorCode, GoalCreateOptions,
    GoalSnapshot, PlanCreateOptions, PlanSnapshot, RunnableTaskProjectionDimension,
    RunnableTasksOptions, SessionFocusOptions, SessionFocusPathEntry, SessionStartOptions,
    StoreInitOptions, StructuralReferenceCreateCommit, StructuralReferenceCreateOptions,
    StructuralReferenceEndpointKind, StructuralReferenceSnapshot, TaskCreateOptions, TaskSnapshot,
    WorkspaceInfo, WorkspaceInitOptions,
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
        StoreInitOptions::new("phase3r-structural-reference-store").expect("store options"),
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
                "Keep structural references explicit",
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

fn references_by_edge(
    references: &[StructuralReferenceSnapshot],
) -> BTreeMap<(EntityId, EntityId), &StructuralReferenceSnapshot> {
    references
        .iter()
        .map(|reference| {
            (
                (reference.referrer_entity_id, reference.target_entity_id),
                reference,
            )
        })
        .collect()
}

fn reference_edge_set(
    references: &[StructuralReferenceSnapshot],
) -> BTreeSet<(EntityId, EntityId)> {
    references
        .iter()
        .map(|reference| (reference.referrer_entity_id, reference.target_entity_id))
        .collect()
}

#[test]
fn structural_reference_pairs_persist_and_replay_after_reopen() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let goal = create_goal_snapshot(
        &mut engine,
        &workspace,
        workspace.genesis_commit_id,
        "Deliver WorkVCS V0.1",
    );
    let first_plan = create_plan_snapshot(
        &mut engine,
        &workspace,
        goal.commit_id,
        "Primary implementation plan",
    );
    let second_plan = create_plan_snapshot(
        &mut engine,
        &workspace,
        first_plan.commit_id,
        "Alternative plan",
    );
    let task = create_task_snapshot(
        &mut engine,
        &workspace,
        second_plan.commit_id,
        "Shared executable task",
    );
    let connection = raw_connection(&path);
    let before_counts = history_counts(&connection);

    let goal_references_plan = reference(
        &mut engine,
        &workspace,
        task.commit_id,
        goal.goal_entity_id,
        first_plan.plan_entity_id,
    );
    let goal_references_task = reference(
        &mut engine,
        &workspace,
        goal_references_plan.commit_id,
        goal.goal_entity_id,
        task.task_entity_id,
    );
    let plan_references_plan = reference(
        &mut engine,
        &workspace,
        goal_references_task.commit_id,
        first_plan.plan_entity_id,
        second_plan.plan_entity_id,
    );
    let first_plan_references_task = reference(
        &mut engine,
        &workspace,
        plan_references_plan.commit_id,
        first_plan.plan_entity_id,
        task.task_entity_id,
    );
    let second_plan_references_same_task = reference(
        &mut engine,
        &workspace,
        first_plan_references_task.commit_id,
        second_plan.plan_entity_id,
        task.task_entity_id,
    );

    let after_counts = history_counts(&connection);
    assert_relation_create_delta(&before_counts, &after_counts, 5);

    let references = engine
        .structural_references_at(second_plan_references_same_task.commit_id)
        .expect("structural references");
    assert_eq!(
        reference_edge_set(&references),
        BTreeSet::from([
            (goal.goal_entity_id, first_plan.plan_entity_id),
            (goal.goal_entity_id, task.task_entity_id),
            (first_plan.plan_entity_id, second_plan.plan_entity_id),
            (first_plan.plan_entity_id, task.task_entity_id),
            (second_plan.plan_entity_id, task.task_entity_id),
        ])
    );
    let by_edge = references_by_edge(&references);
    assert_eq!(
        by_edge
            .get(&(goal.goal_entity_id, first_plan.plan_entity_id))
            .expect("goal to plan")
            .referrer_kind,
        StructuralReferenceEndpointKind::Goal
    );
    assert_eq!(
        by_edge
            .get(&(goal.goal_entity_id, task.task_entity_id))
            .expect("goal to task")
            .target_kind,
        StructuralReferenceEndpointKind::Task
    );
    assert_eq!(
        by_edge
            .get(&(first_plan.plan_entity_id, second_plan.plan_entity_id))
            .expect("plan to plan")
            .target_kind,
        StructuralReferenceEndpointKind::Plan
    );
    assert_eq!(
        by_edge
            .get(&(second_plan.plan_entity_id, task.task_entity_id))
            .expect("second plan to task")
            .target_kind,
        StructuralReferenceEndpointKind::Task
    );

    let reopened = Engine::open(&path).expect("reopen engine");
    let reopened_references = reopened
        .structural_references_at(second_plan_references_same_task.commit_id)
        .expect("reopened structural references");
    assert_eq!(references, reopened_references);
}

#[test]
fn structural_references_do_not_drive_primary_containment_or_runnable_scope() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let referrer_plan = create_plan_snapshot(
        &mut engine,
        &workspace,
        workspace.genesis_commit_id,
        "Referrer plan",
    );
    let referenced_task = create_task_snapshot(
        &mut engine,
        &workspace,
        referrer_plan.commit_id,
        "Referenced task",
    );
    let structural_reference = reference(
        &mut engine,
        &workspace,
        referenced_task.commit_id,
        referrer_plan.plan_entity_id,
        referenced_task.task_entity_id,
    );
    let started = engine
        .start_session(
            SessionStartOptions::new(workspace.workspace_id, workspace.initial_branch_id)
                .expect("session options"),
        )
        .expect("start session");
    engine
        .set_session_focus(
            SessionFocusOptions::new(started.session_id, referrer_plan.plan_entity_id).with_path(
                vec![SessionFocusPathEntry::new(
                    referrer_plan.plan_entity_id,
                    None,
                )],
            ),
        )
        .expect("set plan focus");
    let connection = raw_connection(&path);
    let before_counts = history_counts(&connection);
    let before_head = branch_head(&connection, workspace.initial_branch_id);

    let containment = engine
        .primary_containment_relations_at(structural_reference.commit_id)
        .expect("primary containment");
    let projection = engine
        .runnable_tasks(RunnableTasksOptions::new(started.session_id))
        .expect("focused runnable projection");

    assert!(containment.is_empty());
    assert_eq!(projection.head_commit_id, structural_reference.commit_id);
    assert_eq!(
        projection.deferred_dimensions,
        vec![
            RunnableTaskProjectionDimension::ExplicitManualOrder,
            RunnableTaskProjectionDimension::FinalEqualCandidateTieBreaker,
        ]
    );
    assert!(projection.candidates.is_empty());
    assert_eq!(history_counts(&connection), before_counts);
    assert_eq!(
        branch_head(&connection, workspace.initial_branch_id),
        before_head
    );
}

#[test]
fn invalid_reference_edges_are_rejected_without_partial_rows() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let goal = create_goal_snapshot(
        &mut engine,
        &workspace,
        workspace.genesis_commit_id,
        "Reference boundary goal",
    );
    let plan = create_plan_snapshot(&mut engine, &workspace, goal.commit_id, "Reference plan");
    let task = create_task_snapshot(&mut engine, &workspace, plan.commit_id, "Reference task");
    let valid = reference(
        &mut engine,
        &workspace,
        task.commit_id,
        plan.plan_entity_id,
        task.task_entity_id,
    );

    let connection = raw_connection(&path);
    let before_counts = history_counts(&connection);
    let before_head = branch_head(&connection, workspace.initial_branch_id);
    let duplicate = engine
        .create_structural_reference(
            StructuralReferenceCreateOptions::new(
                workspace.initial_branch_id,
                valid.commit_id,
                plan.plan_entity_id,
                task.task_entity_id,
            )
            .expect("duplicate options"),
        )
        .expect_err("duplicate reference should be rejected");
    assert_eq!(duplicate.code(), ErrorCode::RelationInvalid);
    assert_eq!(duplicate.category(), ErrorCategory::Relation);
    assert_counts_and_head_unchanged(
        &path,
        &before_counts,
        workspace.initial_branch_id,
        before_head,
    );

    let self_edge = StructuralReferenceCreateOptions::new(
        workspace.initial_branch_id,
        valid.commit_id,
        plan.plan_entity_id,
        plan.plan_entity_id,
    )
    .expect_err("self reference should be rejected");
    assert_eq!(self_edge.code(), ErrorCode::RelationInvalid);
    assert_eq!(self_edge.category(), ErrorCategory::Relation);
    assert_counts_and_head_unchanged(
        &path,
        &before_counts,
        workspace.initial_branch_id,
        before_head,
    );

    let task_referrer = engine
        .create_structural_reference(
            StructuralReferenceCreateOptions::new(
                workspace.initial_branch_id,
                valid.commit_id,
                task.task_entity_id,
                plan.plan_entity_id,
            )
            .expect("task referrer options"),
        )
        .expect_err("task referrer should be rejected");
    assert_eq!(task_referrer.code(), ErrorCode::RelationInvalid);
    assert_eq!(task_referrer.category(), ErrorCategory::Relation);
    assert_counts_and_head_unchanged(
        &path,
        &before_counts,
        workspace.initial_branch_id,
        before_head,
    );

    let goal_target = engine
        .create_structural_reference(
            StructuralReferenceCreateOptions::new(
                workspace.initial_branch_id,
                valid.commit_id,
                plan.plan_entity_id,
                goal.goal_entity_id,
            )
            .expect("goal target options"),
        )
        .expect_err("goal target should be rejected");
    assert_eq!(goal_target.code(), ErrorCode::RelationInvalid);
    assert_eq!(goal_target.category(), ErrorCategory::Relation);
    assert_counts_and_head_unchanged(
        &path,
        &before_counts,
        workspace.initial_branch_id,
        before_head,
    );
}

#[test]
fn stale_head_and_noncurrent_reference_endpoint_roll_back_without_partial_rows() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let plan = create_plan_snapshot(
        &mut engine,
        &workspace,
        workspace.genesis_commit_id,
        "Stale reference plan",
    );
    let first_task = create_task_snapshot(&mut engine, &workspace, plan.commit_id, "First task");
    let second_task =
        create_task_snapshot(&mut engine, &workspace, first_task.commit_id, "Second task");
    let connection = raw_connection(&path);
    let before_counts = history_counts(&connection);
    let before_head = branch_head(&connection, workspace.initial_branch_id);

    let stale = engine
        .create_structural_reference(
            StructuralReferenceCreateOptions::new(
                workspace.initial_branch_id,
                first_task.commit_id,
                plan.plan_entity_id,
                first_task.task_entity_id,
            )
            .expect("stale options"),
        )
        .expect_err("stale head should be rejected");
    assert_eq!(stale.code(), ErrorCode::BranchHeadConflict);
    assert_counts_and_head_unchanged(
        &path,
        &before_counts,
        workspace.initial_branch_id,
        before_head,
    );

    let noncurrent = engine
        .create_structural_reference(
            StructuralReferenceCreateOptions::new(
                workspace.initial_branch_id,
                first_task.commit_id,
                plan.plan_entity_id,
                second_task.task_entity_id,
            )
            .expect("noncurrent options"),
        )
        .expect_err("noncurrent endpoint should be rejected");
    assert_eq!(noncurrent.code(), ErrorCode::RelationInvalid);
    assert_eq!(noncurrent.category(), ErrorCategory::Relation);
    assert_counts_and_head_unchanged(
        &path,
        &before_counts,
        workspace.initial_branch_id,
        before_head,
    );
}

#[test]
fn structural_reference_readback_rejects_malformed_discriminator() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let plan = create_plan_snapshot(
        &mut engine,
        &workspace,
        workspace.genesis_commit_id,
        "Malformed readback plan",
    );
    let task = create_task_snapshot(&mut engine, &workspace, plan.commit_id, "Referenced task");
    let structural_reference = reference(
        &mut engine,
        &workspace,
        task.commit_id,
        plan.plan_entity_id,
        task.task_entity_id,
    );
    let connection = raw_connection(&path);
    connection
        .execute(
            "UPDATE relation
             SET relation_discriminator = 'unexpected'
             WHERE object_id = ?1",
            params![&structural_reference.relation_id.raw_bytes()[..]],
        )
        .expect("corrupt relation discriminator");

    let error = engine
        .structural_references_at(structural_reference.commit_id)
        .expect_err("malformed reference discriminator should be rejected");

    assert_eq!(error.code(), ErrorCode::RelationInvalid);
    assert_eq!(error.category(), ErrorCategory::Relation);
}
