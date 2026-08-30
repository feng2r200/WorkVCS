use rusqlite::{Connection, params};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use tempfile::TempDir;
use workvcs_core::{
    AcceptanceCriterionClassification, AcceptanceCriterionCreateOptions, CanonicalValue,
    ClaimReleaseOptions, ClaimTaskOptions, CommitId, Engine, EntityId, ErrorCategory, ErrorCode,
    RunnableTaskBlockedReason, RunnableTaskCandidate, RunnableTaskClaimCoordination,
    RunnableTaskProjectionDimension, RunnableTasksOptions, SessionEndOptions, SessionId,
    SessionStartOptions, StoreInitOptions, TaskCreateOptions, TaskSnapshot, TaskStatus,
    TaskTransitionOptions, WorkspaceInfo, WorkspaceInitOptions,
};

#[derive(Clone, Debug, PartialEq, Eq)]
struct RuntimeCounts {
    object_identity: i64,
    session_runtime: i64,
    claim: i64,
    claim_runtime: i64,
    session_diff: i64,
    changeset: i64,
    change_operation: i64,
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
        StoreInitOptions::new("phase3g-runnable-store").expect("store options"),
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

fn runtime_counts(connection: &Connection) -> RuntimeCounts {
    RuntimeCounts {
        object_identity: count_rows(connection, "object_identity"),
        session_runtime: count_rows(connection, "session_runtime"),
        claim: count_rows(connection, "claim"),
        claim_runtime: count_rows(connection, "claim_runtime"),
        session_diff: count_rows(connection, "session_diff"),
        changeset: count_rows(connection, "changeset"),
        change_operation: count_rows(connection, "change_operation"),
        workstate_commit: count_rows(connection, "workstate_commit"),
        event: count_rows(connection, "event"),
    }
}

fn rationale(reason: &str) -> CanonicalValue {
    CanonicalValue::object(vec![(
        "reason".to_owned(),
        CanonicalValue::String(reason.to_owned()),
    )])
    .expect("rationale")
}

fn create_task_snapshot(
    engine: &mut Engine,
    workspace: &WorkspaceInfo,
    expected_head_commit_id: CommitId,
    task_description: &str,
    priority: i64,
) -> TaskSnapshot {
    let options = TaskCreateOptions::new(
        workspace.initial_branch_id,
        expected_head_commit_id,
        task_description,
    )
    .expect("task options")
    .with_priority(priority)
    .expect("priority");
    let task = engine.create_task(options).expect("create task");
    engine
        .task_at(task.commit_id, task.task_entity_id)
        .expect("task snapshot")
}

fn transition_task_snapshot(
    engine: &mut Engine,
    workspace: &WorkspaceInfo,
    current: &TaskSnapshot,
    next_status: TaskStatus,
    outcome: Option<&str>,
) -> TaskSnapshot {
    let mut options = TaskTransitionOptions::new(
        workspace.initial_branch_id,
        current.commit_id,
        current.task_entity_id,
        current.task_entity_version_id,
        next_status,
    )
    .expect("transition options");
    if let Some(outcome) = outcome {
        options = options.with_outcome(outcome).expect("outcome");
    }
    let task = engine
        .transition_task(options.with_rationale(rationale("phase3g transition")))
        .expect("transition task");
    engine
        .task_at(task.commit_id, task.task_entity_id)
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

#[test]
fn runnable_projection_lists_lifecycle_candidates_without_mutation() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let pending = create_task_snapshot(
        &mut engine,
        &workspace,
        workspace.genesis_commit_id,
        "Pending candidate",
        4,
    );
    let running_pending = create_task_snapshot(
        &mut engine,
        &workspace,
        pending.commit_id,
        "Running candidate",
        -10,
    );
    let running = transition_task_snapshot(
        &mut engine,
        &workspace,
        &running_pending,
        TaskStatus::InProgress,
        None,
    );
    let blocked_pending = create_task_snapshot(
        &mut engine,
        &workspace,
        running.commit_id,
        "Blocked candidate",
        100,
    );
    let blocked = transition_task_snapshot(
        &mut engine,
        &workspace,
        &blocked_pending,
        TaskStatus::Blocked,
        Some("waiting for an external decision"),
    );
    let criterion = engine
        .create_acceptance_criterion(
            AcceptanceCriterionCreateOptions::new(
                workspace.initial_branch_id,
                blocked.commit_id,
                pending.task_entity_id,
                pending.task_entity_version_id,
                "AC-runnable",
                "Runnable projection must not list non-task entities.",
                AcceptanceCriterionClassification::Required,
            )
            .expect("criterion options"),
        )
        .expect("create criterion");
    let branch_head = engine
        .branch_head(workspace.initial_branch_id)
        .expect("branch head");
    let started = engine
        .start_session(
            SessionStartOptions::new(workspace.workspace_id, workspace.initial_branch_id)
                .expect("session options"),
        )
        .expect("start session");
    let connection = raw_connection(&path);
    let before_counts = runtime_counts(&connection);
    let before_head = engine
        .branch_head(workspace.initial_branch_id)
        .expect("branch head before projection");
    let before_session = engine
        .session_snapshot(started.session_id)
        .expect("session before projection");

    let projection = engine
        .runnable_tasks(RunnableTasksOptions::new(started.session_id))
        .expect("runnable projection");

    assert_eq!(projection.session_id, started.session_id);
    assert_eq!(projection.workspace_id, workspace.workspace_id);
    assert_eq!(projection.branch_id, workspace.initial_branch_id);
    assert_eq!(projection.head_commit_id, branch_head.head_commit_id);
    assert_eq!(
        projection.deferred_dimensions,
        vec![
            RunnableTaskProjectionDimension::ActiveScopePlanPath,
            RunnableTaskProjectionDimension::ExecutableTaskDescendants,
        ]
    );
    assert_eq!(projection.candidates.len(), 3);
    assert!(
        projection
            .candidates
            .iter()
            .all(|candidate| candidate.task.task_entity_id
                != criterion.acceptance_criterion_entity_id)
    );
    assert!(
        projection
            .candidates
            .iter()
            .take(2)
            .all(|candidate| candidate.runnable)
    );
    assert!(
        projection
            .candidates
            .iter()
            .skip(2)
            .all(|candidate| !candidate.runnable)
    );

    let by_entity_id = candidates_by_entity_id(&projection.candidates);
    let pending_candidate = by_entity_id
        .get(&pending.task_entity_id)
        .expect("pending task candidate");
    assert_eq!(pending_candidate.task.state.status, TaskStatus::Pending);
    assert_eq!(pending_candidate.task.state.priority, 4);
    assert!(pending_candidate.lifecycle_eligible);
    assert!(pending_candidate.dependency_ready);
    assert!(
        pending_candidate
            .unsatisfied_dependency_entity_ids
            .is_empty()
    );
    assert!(pending_candidate.runnable);
    assert_eq!(
        pending_candidate.claim_coordination,
        RunnableTaskClaimCoordination::Unclaimed
    );
    assert!(pending_candidate.blocked_reasons.is_empty());

    let running_candidate = by_entity_id
        .get(&running.task_entity_id)
        .expect("running task candidate");
    assert_eq!(running_candidate.task.state.status, TaskStatus::InProgress);
    assert_eq!(running_candidate.task.state.priority, -10);
    assert!(running_candidate.lifecycle_eligible);
    assert!(running_candidate.dependency_ready);
    assert!(
        running_candidate
            .unsatisfied_dependency_entity_ids
            .is_empty()
    );
    assert!(running_candidate.runnable);
    assert_eq!(
        running_candidate.claim_coordination,
        RunnableTaskClaimCoordination::Unclaimed
    );
    assert!(running_candidate.blocked_reasons.is_empty());

    let blocked_candidate = by_entity_id
        .get(&blocked.task_entity_id)
        .expect("blocked task candidate");
    assert_eq!(blocked_candidate.task.state.status, TaskStatus::Blocked);
    assert_eq!(blocked_candidate.task.state.priority, 100);
    assert!(!blocked_candidate.lifecycle_eligible);
    assert!(blocked_candidate.dependency_ready);
    assert!(
        blocked_candidate
            .unsatisfied_dependency_entity_ids
            .is_empty()
    );
    assert!(!blocked_candidate.runnable);
    assert_eq!(
        blocked_candidate.claim_coordination,
        RunnableTaskClaimCoordination::Unclaimed
    );
    assert_eq!(
        blocked_candidate.blocked_reasons,
        vec![RunnableTaskBlockedReason::LifecycleIneligible]
    );

    let runnable_ids = projection
        .candidates
        .iter()
        .filter(|candidate| candidate.runnable)
        .map(|candidate| candidate.task.task_entity_id)
        .collect::<Vec<_>>();
    let mut expected_runnable_ids = runnable_ids.clone();
    expected_runnable_ids.sort();
    assert_eq!(runnable_ids, expected_runnable_ids);
    assert_eq!(runtime_counts(&connection), before_counts);
    assert_eq!(
        engine
            .branch_head(workspace.initial_branch_id)
            .expect("branch head after projection"),
        before_head
    );
    assert_eq!(
        engine
            .session_snapshot(started.session_id)
            .expect("session after projection"),
        before_session
    );
}

#[test]
fn runnable_projection_rejects_missing_inactive_session_or_inactive_branch_without_mutation() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let _task = create_task_snapshot(
        &mut engine,
        &workspace,
        workspace.genesis_commit_id,
        "Invalid projection inputs",
        0,
    );
    let missing_session = engine
        .runnable_tasks(RunnableTasksOptions::new(SessionId::new_v7()))
        .expect_err("missing session should fail");
    assert_eq!(missing_session.code(), ErrorCode::SessionNotFound);
    assert_eq!(missing_session.category(), ErrorCategory::Runtime);

    let ended_session = engine
        .start_session(
            SessionStartOptions::new(workspace.workspace_id, workspace.initial_branch_id)
                .expect("session options"),
        )
        .expect("ended session");
    engine
        .end_session(SessionEndOptions::new(ended_session.session_id).expect("end options"))
        .expect("end session");
    let connection = raw_connection(&path);
    let after_end_counts = runtime_counts(&connection);
    let inactive_session = engine
        .runnable_tasks(RunnableTasksOptions::new(ended_session.session_id))
        .expect_err("ended session should fail");
    assert_eq!(inactive_session.code(), ErrorCode::SessionInvalid);
    assert_eq!(inactive_session.category(), ErrorCategory::Runtime);
    assert_eq!(runtime_counts(&connection), after_end_counts);

    let active_session = engine
        .start_session(
            SessionStartOptions::new(workspace.workspace_id, workspace.initial_branch_id)
                .expect("session options"),
        )
        .expect("active session");
    let branch_id = workspace.initial_branch_id.raw_bytes();
    connection
        .execute(
            "UPDATE branch
             SET lifecycle_state = 'closed'
             WHERE branch_id = ?1",
            params![&branch_id[..]],
        )
        .expect("close branch");
    let before_inactive_branch = runtime_counts(&connection);
    let inactive_branch = engine
        .runnable_tasks(RunnableTasksOptions::new(active_session.session_id))
        .expect_err("inactive branch should fail");
    assert_eq!(inactive_branch.code(), ErrorCode::SessionInvalid);
    assert_eq!(inactive_branch.category(), ErrorCategory::Runtime);
    assert_eq!(runtime_counts(&connection), before_inactive_branch);
}

#[test]
fn runnable_projection_rejects_terminal_task_statuses() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let done_pending = create_task_snapshot(
        &mut engine,
        &workspace,
        workspace.genesis_commit_id,
        "Done terminal candidate",
        0,
    );
    let done = transition_task_snapshot(
        &mut engine,
        &workspace,
        &done_pending,
        TaskStatus::Done,
        Some("accepted"),
    );
    let failed_pending = create_task_snapshot(
        &mut engine,
        &workspace,
        done.commit_id,
        "Failed terminal candidate",
        0,
    );
    let failed = transition_task_snapshot(
        &mut engine,
        &workspace,
        &failed_pending,
        TaskStatus::Failed,
        Some("failed validation"),
    );
    let cancelled_pending = create_task_snapshot(
        &mut engine,
        &workspace,
        failed.commit_id,
        "Cancelled terminal candidate",
        0,
    );
    let cancelled = transition_task_snapshot(
        &mut engine,
        &workspace,
        &cancelled_pending,
        TaskStatus::Cancelled,
        Some("cancelled explicitly"),
    );
    let started = engine
        .start_session(
            SessionStartOptions::new(workspace.workspace_id, workspace.initial_branch_id)
                .expect("session options"),
        )
        .expect("start session");

    let projection = engine
        .runnable_tasks(RunnableTasksOptions::new(started.session_id))
        .expect("runnable projection");

    assert_eq!(projection.candidates.len(), 3);
    let by_entity_id = candidates_by_entity_id(&projection.candidates);
    for (snapshot, status) in [
        (&done, TaskStatus::Done),
        (&failed, TaskStatus::Failed),
        (&cancelled, TaskStatus::Cancelled),
    ] {
        let candidate = by_entity_id
            .get(&snapshot.task_entity_id)
            .expect("terminal candidate");
        assert_eq!(candidate.task.state.status, status);
        assert!(!candidate.lifecycle_eligible);
        assert!(candidate.dependency_ready);
        assert!(candidate.unsatisfied_dependency_entity_ids.is_empty());
        assert!(!candidate.runnable);
        assert_eq!(
            candidate.blocked_reasons,
            vec![RunnableTaskBlockedReason::LifecycleIneligible]
        );
    }
}

#[test]
fn runnable_projection_coordinates_active_and_released_claims_without_mutation() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let first_task = create_task_snapshot(
        &mut engine,
        &workspace,
        workspace.genesis_commit_id,
        "Claimed by first session",
        0,
    );
    let second_task = create_task_snapshot(
        &mut engine,
        &workspace,
        first_task.commit_id,
        "Still unclaimed",
        0,
    );
    let first_session = engine
        .start_session(
            SessionStartOptions::new(workspace.workspace_id, workspace.initial_branch_id)
                .expect("first session options"),
        )
        .expect("first session");
    let second_session = engine
        .start_session(
            SessionStartOptions::new(workspace.workspace_id, workspace.initial_branch_id)
                .expect("second session options"),
        )
        .expect("second session");
    let claimed = engine
        .claim_task(ClaimTaskOptions::new(
            first_session.session_id,
            first_task.task_entity_id,
        ))
        .expect("claim task");
    let connection = raw_connection(&path);
    let before_projection = runtime_counts(&connection);

    let first_projection = engine
        .runnable_tasks(RunnableTasksOptions::new(first_session.session_id))
        .expect("first projection");
    let second_projection = engine
        .runnable_tasks(RunnableTasksOptions::new(second_session.session_id))
        .expect("second projection");

    assert_eq!(runtime_counts(&connection), before_projection);
    let first_candidates = candidates_by_entity_id(&first_projection.candidates);
    let first_claimed_candidate = first_candidates
        .get(&first_task.task_entity_id)
        .expect("first session claimed candidate");
    assert!(first_claimed_candidate.lifecycle_eligible);
    assert!(first_claimed_candidate.dependency_ready);
    assert!(first_claimed_candidate.runnable);
    assert_eq!(
        first_claimed_candidate.claim_coordination,
        RunnableTaskClaimCoordination::ClaimedBySession {
            claim_id: claimed.claim_id
        }
    );
    assert!(first_claimed_candidate.blocked_reasons.is_empty());

    let first_unclaimed_candidate = first_candidates
        .get(&second_task.task_entity_id)
        .expect("first session unclaimed candidate");
    assert!(first_unclaimed_candidate.runnable);
    assert!(first_unclaimed_candidate.dependency_ready);
    assert_eq!(
        first_unclaimed_candidate.claim_coordination,
        RunnableTaskClaimCoordination::Unclaimed
    );

    let second_candidates = candidates_by_entity_id(&second_projection.candidates);
    let second_blocked_candidate = second_candidates
        .get(&first_task.task_entity_id)
        .expect("second session claim-blocked candidate");
    assert!(second_blocked_candidate.lifecycle_eligible);
    assert!(second_blocked_candidate.dependency_ready);
    assert!(!second_blocked_candidate.runnable);
    assert_eq!(
        second_blocked_candidate.claim_coordination,
        RunnableTaskClaimCoordination::ClaimedByOtherSession {
            claim_id: claimed.claim_id,
            session_id: first_session.session_id,
        }
    );
    assert_eq!(
        second_blocked_candidate.blocked_reasons,
        vec![RunnableTaskBlockedReason::ClaimBlocked]
    );

    engine
        .release_claim(ClaimReleaseOptions::new(
            first_session.session_id,
            claimed.claim_id,
        ))
        .expect("release claim");
    let after_release = runtime_counts(&connection);
    let second_after_release = engine
        .runnable_tasks(RunnableTasksOptions::new(second_session.session_id))
        .expect("second projection after release");
    assert_eq!(runtime_counts(&connection), after_release);
    let second_after_release_candidates = candidates_by_entity_id(&second_after_release.candidates);
    let released_candidate = second_after_release_candidates
        .get(&first_task.task_entity_id)
        .expect("released task candidate");
    assert!(released_candidate.runnable);
    assert!(released_candidate.dependency_ready);
    assert_eq!(
        released_candidate.claim_coordination,
        RunnableTaskClaimCoordination::Unclaimed
    );
    assert!(released_candidate.blocked_reasons.is_empty());
}
