use rusqlite::Connection;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use tempfile::TempDir;
use workvcs_core::{
    AcceptanceCriterionClassification, AcceptanceCriterionCreateOptions,
    AcceptanceCriterionEffectiveStatus, ClaimTaskOptions, CommitId, Engine, EntityId, ErrorCode,
    RunnableTaskBlockedReason, RunnableTaskCandidate, RunnableTaskProjectionDimension,
    RunnableTasksOptions, SessionStartOptions, StoreInitOptions, TaskCreateOptions,
    TaskSchedulingRelationCreateOptions, TaskSnapshot, TaskStatus, TaskTransitionOptions,
    VerificationCreateOptions, VerificationResult, VerificationTarget, WorkspaceInfo,
    WorkspaceInitOptions,
};

#[derive(Clone, Debug, PartialEq, Eq)]
struct StoreCounts {
    object_identity: i64,
    entity: i64,
    relation: i64,
    relation_version: i64,
    session_runtime: i64,
    claim: i64,
    claim_runtime: i64,
    session_diff: i64,
    changeset: i64,
    change_operation: i64,
    entity_membership_change: i64,
    relation_membership_change: i64,
    workstate_commit: i64,
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
        StoreInitOptions::new("phase3i-runnable-dependency-store").expect("store options"),
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

fn store_counts(connection: &Connection) -> StoreCounts {
    StoreCounts {
        object_identity: count_rows(connection, "object_identity"),
        entity: count_rows(connection, "entity"),
        relation: count_rows(connection, "relation"),
        relation_version: count_rows(connection, "relation_version"),
        session_runtime: count_rows(connection, "session_runtime"),
        claim: count_rows(connection, "claim"),
        claim_runtime: count_rows(connection, "claim_runtime"),
        session_diff: count_rows(connection, "session_diff"),
        changeset: count_rows(connection, "changeset"),
        change_operation: count_rows(connection, "change_operation"),
        entity_membership_change: count_rows(connection, "entity_membership_change"),
        relation_membership_change: count_rows(connection, "relation_membership_change"),
        workstate_commit: count_rows(connection, "workstate_commit"),
        event: count_rows(connection, "event"),
    }
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

fn transition_task_snapshot(
    engine: &mut Engine,
    workspace: &WorkspaceInfo,
    expected_head_commit_id: CommitId,
    task: &TaskSnapshot,
    next_status: TaskStatus,
    outcome: &str,
) -> TaskSnapshot {
    let transitioned = engine
        .transition_task(
            TaskTransitionOptions::new(
                workspace.initial_branch_id,
                expected_head_commit_id,
                task.task_entity_id,
                task.task_entity_version_id,
                next_status,
            )
            .expect("transition options")
            .with_outcome(outcome)
            .expect("outcome"),
        )
        .expect("transition task");
    engine
        .task_at(transitioned.commit_id, transitioned.task_entity_id)
        .expect("transitioned task snapshot")
}

fn candidates_by_entity_id(
    candidates: &[RunnableTaskCandidate],
) -> BTreeMap<EntityId, &RunnableTaskCandidate> {
    candidates
        .iter()
        .map(|candidate| (candidate.task.task_entity_id, candidate))
        .collect()
}

fn assert_dependency_readiness_not_deferred(dimensions: &[RunnableTaskProjectionDimension]) {
    assert!(
        !dimensions.contains(&RunnableTaskProjectionDimension::DependencyReadiness),
        "dependency readiness should be implemented in Phase 3I"
    );
}

#[test]
fn dependency_blocks_until_prerequisite_done_without_mutation() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let prerequisite = create_task_snapshot(
        &mut engine,
        &workspace,
        workspace.genesis_commit_id,
        "Prerequisite",
    );
    let dependent =
        create_task_snapshot(&mut engine, &workspace, prerequisite.commit_id, "Dependent");
    let relation = engine
        .create_task_scheduling_relation(
            TaskSchedulingRelationCreateOptions::depends_on(
                workspace.initial_branch_id,
                dependent.commit_id,
                dependent.task_entity_id,
                prerequisite.task_entity_id,
            )
            .expect("relation options"),
        )
        .expect("create dependency");
    let session = engine
        .start_session(
            SessionStartOptions::new(workspace.workspace_id, workspace.initial_branch_id)
                .expect("session options"),
        )
        .expect("start session");
    let connection = raw_connection(&path);
    let before_projection = store_counts(&connection);

    let projection = engine
        .runnable_tasks(RunnableTasksOptions::new(session.session_id))
        .expect("runnable projection");

    assert_eq!(store_counts(&connection), before_projection);
    assert_dependency_readiness_not_deferred(&projection.deferred_dimensions);
    let candidates = candidates_by_entity_id(&projection.candidates);
    let prerequisite_candidate = candidates
        .get(&prerequisite.task_entity_id)
        .expect("prerequisite candidate");
    assert!(prerequisite_candidate.dependency_ready);
    assert!(prerequisite_candidate.runnable);

    let dependent_candidate = candidates
        .get(&dependent.task_entity_id)
        .expect("dependent candidate");
    assert!(dependent_candidate.lifecycle_eligible);
    assert!(!dependent_candidate.dependency_ready);
    assert_eq!(
        dependent_candidate.unsatisfied_dependency_entity_ids,
        vec![prerequisite.task_entity_id]
    );
    assert!(!dependent_candidate.runnable);
    assert_eq!(
        dependent_candidate.blocked_reasons,
        vec![RunnableTaskBlockedReason::DependencyBlocked]
    );

    let prerequisite_done = transition_task_snapshot(
        &mut engine,
        &workspace,
        relation.commit_id,
        &prerequisite,
        TaskStatus::Done,
        "prerequisite completed",
    );
    let before_ready_projection = store_counts(&connection);
    let ready_projection = engine
        .runnable_tasks(RunnableTasksOptions::new(session.session_id))
        .expect("ready projection");
    assert_eq!(store_counts(&connection), before_ready_projection);
    let ready_candidates = candidates_by_entity_id(&ready_projection.candidates);
    let ready_dependent = ready_candidates
        .get(&dependent.task_entity_id)
        .expect("ready dependent candidate");
    assert!(ready_dependent.dependency_ready);
    assert!(ready_dependent.unsatisfied_dependency_entity_ids.is_empty());
    assert!(ready_dependent.runnable);
    assert_eq!(prerequisite_done.state.status, TaskStatus::Done);
}

#[test]
fn dependency_readiness_uses_transitive_closure() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let root = create_task_snapshot(
        &mut engine,
        &workspace,
        workspace.genesis_commit_id,
        "Root prerequisite",
    );
    let middle = create_task_snapshot(&mut engine, &workspace, root.commit_id, "Middle task");
    let leaf = create_task_snapshot(&mut engine, &workspace, middle.commit_id, "Leaf task");
    let middle_depends_on_root = engine
        .create_task_scheduling_relation(
            TaskSchedulingRelationCreateOptions::depends_on(
                workspace.initial_branch_id,
                leaf.commit_id,
                middle.task_entity_id,
                root.task_entity_id,
            )
            .expect("middle dependency options"),
        )
        .expect("middle depends on root");
    let leaf_depends_on_middle = engine
        .create_task_scheduling_relation(
            TaskSchedulingRelationCreateOptions::depends_on(
                workspace.initial_branch_id,
                middle_depends_on_root.commit_id,
                leaf.task_entity_id,
                middle.task_entity_id,
            )
            .expect("leaf dependency options"),
        )
        .expect("leaf depends on middle");
    let session = engine
        .start_session(
            SessionStartOptions::new(workspace.workspace_id, workspace.initial_branch_id)
                .expect("session options"),
        )
        .expect("start session");

    let projection = engine
        .runnable_tasks(RunnableTasksOptions::new(session.session_id))
        .expect("runnable projection");
    let candidates = candidates_by_entity_id(&projection.candidates);
    let mut expected_leaf_unsatisfied = vec![root.task_entity_id, middle.task_entity_id];
    expected_leaf_unsatisfied.sort();
    assert_eq!(
        candidates
            .get(&leaf.task_entity_id)
            .expect("leaf candidate")
            .unsatisfied_dependency_entity_ids,
        expected_leaf_unsatisfied
    );

    let middle_done = transition_task_snapshot(
        &mut engine,
        &workspace,
        leaf_depends_on_middle.commit_id,
        &middle,
        TaskStatus::Done,
        "middle completed",
    );
    let still_blocked_projection = engine
        .runnable_tasks(RunnableTasksOptions::new(session.session_id))
        .expect("projection with root pending");
    let still_blocked = candidates_by_entity_id(&still_blocked_projection.candidates);
    assert_eq!(
        still_blocked
            .get(&leaf.task_entity_id)
            .expect("leaf candidate")
            .unsatisfied_dependency_entity_ids,
        vec![root.task_entity_id]
    );

    let _root_done = transition_task_snapshot(
        &mut engine,
        &workspace,
        middle_done.commit_id,
        &root,
        TaskStatus::Done,
        "root completed",
    );
    let ready_projection = engine
        .runnable_tasks(RunnableTasksOptions::new(session.session_id))
        .expect("ready projection");
    let ready = candidates_by_entity_id(&ready_projection.candidates);
    let leaf_candidate = ready.get(&leaf.task_entity_id).expect("leaf candidate");
    assert!(leaf_candidate.dependency_ready);
    assert!(leaf_candidate.runnable);
}

#[test]
fn blocked_prerequisite_is_unsatisfied_and_claim_block_can_coexist() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let prerequisite = create_task_snapshot(
        &mut engine,
        &workspace,
        workspace.genesis_commit_id,
        "Explicitly blocked prerequisite",
    );
    let dependent =
        create_task_snapshot(&mut engine, &workspace, prerequisite.commit_id, "Dependent");
    let blocked_prerequisite = transition_task_snapshot(
        &mut engine,
        &workspace,
        dependent.commit_id,
        &prerequisite,
        TaskStatus::Blocked,
        "waiting for external input",
    );
    let relation = engine
        .create_task_scheduling_relation(
            TaskSchedulingRelationCreateOptions::depends_on(
                workspace.initial_branch_id,
                blocked_prerequisite.commit_id,
                dependent.task_entity_id,
                prerequisite.task_entity_id,
            )
            .expect("relation options"),
        )
        .expect("create dependency");
    let owner_session = engine
        .start_session(
            SessionStartOptions::new(workspace.workspace_id, workspace.initial_branch_id)
                .expect("owner session options"),
        )
        .expect("owner session");
    let observer_session = engine
        .start_session(
            SessionStartOptions::new(workspace.workspace_id, workspace.initial_branch_id)
                .expect("observer session options"),
        )
        .expect("observer session");
    let claim = engine
        .claim_task(ClaimTaskOptions::new(
            owner_session.session_id,
            dependent.task_entity_id,
        ))
        .expect("claim dependent task");
    assert_eq!(
        relation.previous_head_commit_id,
        blocked_prerequisite.commit_id
    );

    let projection = engine
        .runnable_tasks(RunnableTasksOptions::new(observer_session.session_id))
        .expect("observer projection");
    let candidates = candidates_by_entity_id(&projection.candidates);
    let dependent_candidate = candidates
        .get(&dependent.task_entity_id)
        .expect("dependent candidate");
    assert!(dependent_candidate.lifecycle_eligible);
    assert!(!dependent_candidate.dependency_ready);
    assert_eq!(
        dependent_candidate.unsatisfied_dependency_entity_ids,
        vec![prerequisite.task_entity_id]
    );
    assert!(!dependent_candidate.runnable);
    assert_eq!(
        dependent_candidate.blocked_reasons,
        vec![
            RunnableTaskBlockedReason::DependencyBlocked,
            RunnableTaskBlockedReason::ClaimBlocked
        ]
    );
    assert_eq!(
        engine
            .claim_snapshot(claim.claim_id)
            .expect("claim snapshot")
            .task_entity_id,
        dependent.task_entity_id
    );
}

#[test]
fn ordered_before_and_verification_relations_do_not_block_readiness() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let first = create_task_snapshot(
        &mut engine,
        &workspace,
        workspace.genesis_commit_id,
        "First task",
    );
    let second = create_task_snapshot(&mut engine, &workspace, first.commit_id, "Second task");
    let criterion = engine
        .create_acceptance_criterion(
            AcceptanceCriterionCreateOptions::new(
                workspace.initial_branch_id,
                second.commit_id,
                first.task_entity_id,
                first.task_entity_version_id,
                "AC-ordered-verification",
                "Verification relations must coexist with scheduling relations.",
                AcceptanceCriterionClassification::Required,
            )
            .expect("criterion options"),
        )
        .expect("create acceptance criterion");
    let verification = engine
        .create_verification(
            VerificationCreateOptions::new(
                workspace.initial_branch_id,
                criterion.commit_id,
                VerificationTarget::AcceptanceCriterion(criterion.acceptance_criterion_entity_id),
                VerificationResult::Passed,
            )
            .expect("verification options"),
        )
        .expect("record verification");
    assert_eq!(
        engine
            .acceptance_criterion_effective_status(
                verification.commit_id,
                criterion.acceptance_criterion_entity_id,
            )
            .expect("effective status"),
        AcceptanceCriterionEffectiveStatus::Verified
    );
    let order = engine
        .create_task_scheduling_relation(
            TaskSchedulingRelationCreateOptions::ordered_before(
                workspace.initial_branch_id,
                verification.commit_id,
                first.task_entity_id,
                second.task_entity_id,
            )
            .expect("order relation options"),
        )
        .expect("create ordered relation");
    let session = engine
        .start_session(
            SessionStartOptions::new(workspace.workspace_id, workspace.initial_branch_id)
                .expect("session options"),
        )
        .expect("start session");

    let projection = engine
        .runnable_tasks(RunnableTasksOptions::new(session.session_id))
        .expect("runnable projection");
    let candidates = candidates_by_entity_id(&projection.candidates);
    for task in [&first, &second] {
        let candidate = candidates
            .get(&task.task_entity_id)
            .expect("task candidate");
        assert!(candidate.dependency_ready);
        assert!(candidate.unsatisfied_dependency_entity_ids.is_empty());
        assert!(candidate.runnable);
    }
    assert_eq!(
        engine
            .task_scheduling_relations_at(order.commit_id)
            .expect("scheduling relations")
            .len(),
        1
    );
}

#[test]
fn missing_session_still_rejects_before_reading_dependency_projection() {
    let (_tempdir, path) = store_path();
    let (engine, _workspace) = create_workspace(&path);
    let missing = engine
        .runnable_tasks(RunnableTasksOptions::new(workvcs_core::SessionId::new_v7()))
        .expect_err("missing session should be rejected");
    assert_eq!(missing.code(), ErrorCode::SessionNotFound);
}
