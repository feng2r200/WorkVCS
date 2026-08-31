use rusqlite::{Connection, params};
use std::path::{Path, PathBuf};
use tempfile::TempDir;
use workvcs_core::{
    CanonicalValue, ClaimForceTakeoverOptions, ClaimGuardOptions, ClaimGuardReason, ClaimId,
    ClaimLifecycleState, ClaimListOptions, ClaimMode, ClaimReleaseOptions, ClaimTaskOptions,
    ClaimTransferOptions, Engine, EntityId, ErrorCategory, ErrorCode, SessionEndOptions,
    SessionStartOptions, StoreInitOptions, TaskCreateOptions, TaskSnapshot, WorkspaceInfo,
    WorkspaceInitOptions, canonical_bytes,
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
        StoreInitOptions::new("phase3f-claim-store").expect("store options"),
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

fn canonical_json(value: &CanonicalValue) -> String {
    String::from_utf8(canonical_bytes(value).expect("canonical bytes")).expect("utf8")
}

fn claim_created_payload(
    claim_id: ClaimId,
    session_id: workvcs_core::SessionId,
    workspace: &WorkspaceInfo,
    task_entity_id: EntityId,
) -> String {
    claim_created_payload_with_mode(claim_id, session_id, workspace, task_entity_id, "exclusive")
}

fn claim_created_payload_with_mode(
    claim_id: ClaimId,
    session_id: workvcs_core::SessionId,
    workspace: &WorkspaceInfo,
    task_entity_id: EntityId,
    mode: &str,
) -> String {
    canonical_json(
        &CanonicalValue::object(vec![
            (
                "branch_id".to_owned(),
                CanonicalValue::String(workspace.initial_branch_id.to_string()),
            ),
            (
                "claim_id".to_owned(),
                CanonicalValue::String(claim_id.to_string()),
            ),
            (
                "lifecycle_state".to_owned(),
                CanonicalValue::String("active".to_owned()),
            ),
            ("mode".to_owned(), CanonicalValue::String(mode.to_owned())),
            (
                "session_id".to_owned(),
                CanonicalValue::String(session_id.to_string()),
            ),
            (
                "task_entity_id".to_owned(),
                CanonicalValue::String(task_entity_id.to_string()),
            ),
            (
                "workspace_id".to_owned(),
                CanonicalValue::String(workspace.workspace_id.to_string()),
            ),
        ])
        .expect("payload"),
    )
}

fn claim_released_payload(
    claim_id: ClaimId,
    session_id: workvcs_core::SessionId,
    workspace: &WorkspaceInfo,
    task_entity_id: EntityId,
) -> String {
    canonical_json(
        &CanonicalValue::object(vec![
            (
                "branch_id".to_owned(),
                CanonicalValue::String(workspace.initial_branch_id.to_string()),
            ),
            (
                "claim_id".to_owned(),
                CanonicalValue::String(claim_id.to_string()),
            ),
            (
                "lifecycle_state".to_owned(),
                CanonicalValue::String("released".to_owned()),
            ),
            (
                "mode".to_owned(),
                CanonicalValue::String("exclusive".to_owned()),
            ),
            (
                "session_id".to_owned(),
                CanonicalValue::String(session_id.to_string()),
            ),
            (
                "task_entity_id".to_owned(),
                CanonicalValue::String(task_entity_id.to_string()),
            ),
            (
                "workspace_id".to_owned(),
                CanonicalValue::String(workspace.workspace_id.to_string()),
            ),
        ])
        .expect("payload"),
    )
}

fn claim_transferred_payload(
    previous_claim_id: ClaimId,
    claim_id: ClaimId,
    from_session_id: workvcs_core::SessionId,
    to_session_id: workvcs_core::SessionId,
    workspace: &WorkspaceInfo,
    task_entity_id: EntityId,
    previous_last_activity_at_us: i64,
) -> String {
    canonical_json(
        &CanonicalValue::object(vec![
            (
                "branch_id".to_owned(),
                CanonicalValue::String(workspace.initial_branch_id.to_string()),
            ),
            (
                "claim_id".to_owned(),
                CanonicalValue::String(claim_id.to_string()),
            ),
            (
                "from_session_id".to_owned(),
                CanonicalValue::String(from_session_id.to_string()),
            ),
            (
                "lifecycle_state".to_owned(),
                CanonicalValue::String("active".to_owned()),
            ),
            (
                "mode".to_owned(),
                CanonicalValue::String("exclusive".to_owned()),
            ),
            (
                "previous_claim_id".to_owned(),
                CanonicalValue::String(previous_claim_id.to_string()),
            ),
            (
                "previous_last_activity_at_us".to_owned(),
                CanonicalValue::safe_integer(previous_last_activity_at_us).expect("safe integer"),
            ),
            (
                "reason".to_owned(),
                CanonicalValue::String("transfer".to_owned()),
            ),
            (
                "task_entity_id".to_owned(),
                CanonicalValue::String(task_entity_id.to_string()),
            ),
            (
                "to_session_id".to_owned(),
                CanonicalValue::String(to_session_id.to_string()),
            ),
            (
                "workspace_id".to_owned(),
                CanonicalValue::String(workspace.workspace_id.to_string()),
            ),
        ])
        .expect("payload"),
    )
}

struct ClaimForceTakenOverPayload<'a> {
    previous_claim_id: ClaimId,
    claim_id: ClaimId,
    previous_session_id: workvcs_core::SessionId,
    session_id: workvcs_core::SessionId,
    workspace: &'a WorkspaceInfo,
    task_entity_id: EntityId,
    previous_last_activity_at_us: i64,
    rationale: &'a str,
}

fn claim_force_taken_over_payload(payload: ClaimForceTakenOverPayload<'_>) -> String {
    canonical_json(
        &CanonicalValue::object(vec![
            (
                "branch_id".to_owned(),
                CanonicalValue::String(payload.workspace.initial_branch_id.to_string()),
            ),
            (
                "claim_id".to_owned(),
                CanonicalValue::String(payload.claim_id.to_string()),
            ),
            (
                "lifecycle_state".to_owned(),
                CanonicalValue::String("active".to_owned()),
            ),
            (
                "mode".to_owned(),
                CanonicalValue::String("exclusive".to_owned()),
            ),
            (
                "previous_claim_id".to_owned(),
                CanonicalValue::String(payload.previous_claim_id.to_string()),
            ),
            (
                "previous_last_activity_at_us".to_owned(),
                CanonicalValue::safe_integer(payload.previous_last_activity_at_us)
                    .expect("safe integer"),
            ),
            (
                "previous_session_id".to_owned(),
                CanonicalValue::String(payload.previous_session_id.to_string()),
            ),
            (
                "previous_session_lifecycle_state".to_owned(),
                CanonicalValue::String("active".to_owned()),
            ),
            (
                "rationale".to_owned(),
                CanonicalValue::String(payload.rationale.to_owned()),
            ),
            (
                "reason".to_owned(),
                CanonicalValue::String("force".to_owned()),
            ),
            (
                "session_id".to_owned(),
                CanonicalValue::String(payload.session_id.to_string()),
            ),
            (
                "task_entity_id".to_owned(),
                CanonicalValue::String(payload.task_entity_id.to_string()),
            ),
            (
                "workspace_id".to_owned(),
                CanonicalValue::String(payload.workspace.workspace_id.to_string()),
            ),
        ])
        .expect("payload"),
    )
}

fn event_count(
    connection: &Connection,
    session_id: workvcs_core::SessionId,
    event_kind: &str,
) -> i64 {
    let session_id = session_id.raw_bytes();
    connection
        .query_row(
            "SELECT count(*)
             FROM event
             WHERE session_id = ?1
               AND changeset_id IS NULL
               AND event_kind = ?2",
            params![&session_id[..], event_kind],
            |row| row.get(0),
        )
        .expect("event count")
}

fn latest_event_payload(
    connection: &Connection,
    session_id: workvcs_core::SessionId,
    event_kind: &str,
) -> String {
    let session_id = session_id.raw_bytes();
    connection
        .query_row(
            "SELECT payload_json
             FROM event
             WHERE session_id = ?1
               AND event_kind = ?2
             ORDER BY occurred_at_us DESC
             LIMIT 1",
            params![&session_id[..], event_kind],
            |row| row.get(0),
        )
        .expect("event payload")
}

fn event_payloads(
    connection: &Connection,
    session_id: workvcs_core::SessionId,
    event_kind: &str,
) -> Vec<String> {
    let session_id = session_id.raw_bytes();
    connection
        .prepare(
            "SELECT payload_json
             FROM event
             WHERE session_id = ?1
               AND event_kind = ?2
             ORDER BY payload_json",
        )
        .expect("prepare event payloads")
        .query_map(params![&session_id[..], event_kind], |row| row.get(0))
        .expect("query event payloads")
        .collect::<rusqlite::Result<Vec<_>>>()
        .expect("event payloads")
}

fn create_task_and_session(
    engine: &mut Engine,
    workspace: &WorkspaceInfo,
    task_description: &str,
) -> (TaskSnapshot, workvcs_core::SessionId) {
    let task = engine
        .create_task(
            TaskCreateOptions::new(
                workspace.initial_branch_id,
                workspace.genesis_commit_id,
                task_description,
            )
            .expect("task options"),
        )
        .expect("create task");
    let branch_head = engine
        .branch_head(workspace.initial_branch_id)
        .expect("branch head");
    let task_snapshot = engine
        .task_at(branch_head.head_commit_id, task.task_entity_id)
        .expect("task snapshot");
    let started = engine
        .start_session(
            SessionStartOptions::new(workspace.workspace_id, workspace.initial_branch_id)
                .expect("session options"),
        )
        .expect("start session");
    (task_snapshot, started.session_id)
}

fn create_task_snapshot(
    engine: &mut Engine,
    workspace: &WorkspaceInfo,
    expected_head_commit_id: workvcs_core::CommitId,
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
    let branch_head = engine
        .branch_head(workspace.initial_branch_id)
        .expect("branch head");
    engine
        .task_at(branch_head.head_commit_id, task.task_entity_id)
        .expect("task snapshot")
}

#[test]
fn claim_task_persists_exclusive_runtime_without_task_or_workstate_mutation() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let (task_snapshot, session_id) =
        create_task_and_session(&mut engine, &workspace, "Claim runtime task");
    let connection = raw_connection(&path);
    let before = runtime_counts(&connection);
    let before_head = engine
        .branch_head(workspace.initial_branch_id)
        .expect("branch head");
    let before_session = engine
        .session_snapshot(session_id)
        .expect("session before claim");

    let claimed = engine
        .claim_task(ClaimTaskOptions::new(
            session_id,
            task_snapshot.task_entity_id,
        ))
        .expect("claim task");

    assert_eq!(claimed.session_id, session_id);
    assert_eq!(claimed.workspace_id, workspace.workspace_id);
    assert_eq!(claimed.branch_id, workspace.initial_branch_id);
    assert_eq!(claimed.task_entity_id, task_snapshot.task_entity_id);
    assert_eq!(claimed.mode, ClaimMode::Exclusive);
    assert!(claimed.claimed_at_us >= before_session.started_at_us);
    assert_eq!(claimed.state.claim_id, claimed.claim_id);
    assert_eq!(claimed.state.lifecycle_state, ClaimLifecycleState::Active);
    assert_eq!(claimed.state.session_id, session_id);
    assert_eq!(claimed.state.workspace_id, workspace.workspace_id);
    assert_eq!(claimed.state.branch_id, workspace.initial_branch_id);
    assert_eq!(claimed.state.task_entity_id, task_snapshot.task_entity_id);
    assert_eq!(claimed.state.mode, ClaimMode::Exclusive);
    assert_eq!(claimed.state.created_at_us, claimed.claimed_at_us);
    assert_eq!(
        claimed.state.last_activity_at_us,
        Some(claimed.claimed_at_us)
    );

    let after = runtime_counts(&connection);
    assert_eq!(after.object_identity, before.object_identity + 1);
    assert_eq!(after.claim, before.claim + 1);
    assert_eq!(after.claim_runtime, before.claim_runtime + 1);
    assert_eq!(after.changeset, before.changeset);
    assert_eq!(after.change_operation, before.change_operation);
    assert_eq!(after.workstate_commit, before.workstate_commit);
    assert_eq!(after.event, before.event + 1);

    let after_head = engine
        .branch_head(workspace.initial_branch_id)
        .expect("branch head after claim");
    assert_eq!(after_head.head_commit_id, before_head.head_commit_id);
    assert_eq!(after_head.state_digest, before_head.state_digest);
    let after_task = engine
        .task_at(after_head.head_commit_id, task_snapshot.task_entity_id)
        .expect("task after claim");
    assert_eq!(after_task, task_snapshot);
    let after_session = engine
        .session_snapshot(session_id)
        .expect("session after claim");
    assert_eq!(
        after_session.last_activity_at_us,
        Some(claimed.claimed_at_us)
    );

    assert_eq!(event_count(&connection, session_id, "claim.created"), 1);
    assert_eq!(
        latest_event_payload(&connection, session_id, "claim.created"),
        claim_created_payload(
            claimed.claim_id,
            session_id,
            &workspace,
            task_snapshot.task_entity_id
        )
    );

    drop(engine);
    let reopened = Engine::open(&path).expect("reopen engine");
    let snapshot = reopened
        .claim_snapshot(claimed.claim_id)
        .expect("claim snapshot");
    assert_eq!(snapshot, claimed.state);
}

#[test]
fn active_claims_for_session_lists_only_current_claim_runtime() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let (task_snapshot, session_id) =
        create_task_and_session(&mut engine, &workspace, "List active claim target");

    let empty = engine
        .active_claims_for_session(ClaimListOptions::for_session(session_id))
        .expect("initial active claims");
    assert_eq!(empty.session_id, session_id);
    assert!(empty.claims.is_empty());

    let claimed = engine
        .claim_task(ClaimTaskOptions::new(
            session_id,
            task_snapshot.task_entity_id,
        ))
        .expect("claim task");
    let active = engine
        .active_claims_for_session(ClaimListOptions::for_session(session_id))
        .expect("active claims");
    assert_eq!(active.session_id, session_id);
    assert_eq!(active.claims, vec![claimed.state.clone()]);

    let released = engine
        .release_claim(ClaimReleaseOptions::new(session_id, claimed.claim_id))
        .expect("release claim");
    let after_release = engine
        .active_claims_for_session(ClaimListOptions::for_session(session_id))
        .expect("active claims after release");
    assert!(after_release.claims.is_empty());
    assert_eq!(
        engine
            .claim_snapshot(claimed.claim_id)
            .expect("released claim snapshot"),
        released.state
    );
}

#[test]
fn task_claim_guard_allows_unclaimed_or_owned_exclusive_and_rejects_other_exclusive() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let task_snapshot = create_task_snapshot(
        &mut engine,
        &workspace,
        workspace.genesis_commit_id,
        "Guard exclusive claim target",
    );
    let first_session = engine
        .start_session(
            SessionStartOptions::new(workspace.workspace_id, workspace.initial_branch_id)
                .expect("first session options"),
        )
        .expect("start first session");
    let second_session = engine
        .start_session(
            SessionStartOptions::new(workspace.workspace_id, workspace.initial_branch_id)
                .expect("second session options"),
        )
        .expect("start second session");

    let unclaimed = engine
        .task_claim_guard(ClaimGuardOptions::terminal_task_mutation(
            first_session.session_id,
            task_snapshot.task_entity_id,
        ))
        .expect("unclaimed guard");
    assert!(unclaimed.allowed);
    assert_eq!(unclaimed.reason, ClaimGuardReason::Unclaimed);
    assert!(unclaimed.active_claims.is_empty());

    let claimed = engine
        .claim_task(ClaimTaskOptions::new(
            first_session.session_id,
            task_snapshot.task_entity_id,
        ))
        .expect("exclusive claim");
    let owner = engine
        .task_claim_guard(ClaimGuardOptions::terminal_task_mutation(
            first_session.session_id,
            task_snapshot.task_entity_id,
        ))
        .expect("owner guard");
    assert!(owner.allowed);
    assert_eq!(owner.reason, ClaimGuardReason::OwnedExclusiveClaim);
    assert_eq!(owner.active_claims, vec![claimed.state.clone()]);

    let other = engine
        .task_claim_guard(ClaimGuardOptions::terminal_task_mutation(
            second_session.session_id,
            task_snapshot.task_entity_id,
        ))
        .expect("other guard");
    assert!(!other.allowed);
    assert_eq!(
        other.reason,
        ClaimGuardReason::ExclusiveClaimOwnedByOtherSession
    );
    assert_eq!(other.active_claims, vec![claimed.state]);
}

#[test]
fn task_claim_guard_requires_unique_shared_claimant_for_protected_action() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let task_snapshot = create_task_snapshot(
        &mut engine,
        &workspace,
        workspace.genesis_commit_id,
        "Guard shared claim target",
    );
    let first_session = engine
        .start_session(
            SessionStartOptions::new(workspace.workspace_id, workspace.initial_branch_id)
                .expect("first session options"),
        )
        .expect("start first session");
    let second_session = engine
        .start_session(
            SessionStartOptions::new(workspace.workspace_id, workspace.initial_branch_id)
                .expect("second session options"),
        )
        .expect("start second session");

    engine
        .claim_task(
            ClaimTaskOptions::new(first_session.session_id, task_snapshot.task_entity_id)
                .with_mode(ClaimMode::Shared),
        )
        .expect("first shared claim");
    let unique = engine
        .task_claim_guard(ClaimGuardOptions::structural_task_mutation(
            first_session.session_id,
            task_snapshot.task_entity_id,
        ))
        .expect("unique shared guard");
    assert!(unique.allowed);
    assert_eq!(unique.reason, ClaimGuardReason::UniqueSharedClaimant);
    assert_eq!(unique.active_claims.len(), 1);

    let outsider = engine
        .task_claim_guard(ClaimGuardOptions::structural_task_mutation(
            second_session.session_id,
            task_snapshot.task_entity_id,
        ))
        .expect("outsider shared guard");
    assert!(!outsider.allowed);
    assert_eq!(
        outsider.reason,
        ClaimGuardReason::SharedClaimSetDoesNotIncludeSession
    );

    engine
        .claim_task(
            ClaimTaskOptions::new(second_session.session_id, task_snapshot.task_entity_id)
                .with_mode(ClaimMode::Shared),
        )
        .expect("second shared claim");
    let non_unique = engine
        .task_claim_guard(ClaimGuardOptions::structural_task_mutation(
            first_session.session_id,
            task_snapshot.task_entity_id,
        ))
        .expect("non-unique shared guard");
    assert!(!non_unique.allowed);
    assert_eq!(non_unique.reason, ClaimGuardReason::NonUniqueSharedClaimSet);
    assert_eq!(non_unique.active_claims.len(), 2);
}

#[test]
fn two_engine_claim_conflict_allows_one_active_claim() {
    let (_tempdir, path) = store_path();
    let (mut first_engine, workspace) = create_workspace(&path);
    let mut second_engine = Engine::open(&path).expect("second engine");
    let task_snapshot = create_task_snapshot(
        &mut first_engine,
        &workspace,
        workspace.genesis_commit_id,
        "Two engine claim target",
    );
    let first_session = first_engine
        .start_session(
            SessionStartOptions::new(workspace.workspace_id, workspace.initial_branch_id)
                .expect("session options"),
        )
        .expect("first session");
    let second_session = second_engine
        .start_session(
            SessionStartOptions::new(workspace.workspace_id, workspace.initial_branch_id)
                .expect("session options"),
        )
        .expect("second session");

    let claimed = first_engine
        .claim_task(ClaimTaskOptions::new(
            first_session.session_id,
            task_snapshot.task_entity_id,
        ))
        .expect("first claim");
    let connection = raw_connection(&path);
    let before_conflict = runtime_counts(&connection);

    let error = second_engine
        .claim_task(ClaimTaskOptions::new(
            second_session.session_id,
            task_snapshot.task_entity_id,
        ))
        .expect_err("second engine claim should fail");
    assert_eq!(error.code(), ErrorCode::ClaimInvalid);
    assert_eq!(error.category(), ErrorCategory::Runtime);
    assert_eq!(runtime_counts(&connection), before_conflict);
    assert_eq!(
        first_engine
            .claim_snapshot(claimed.claim_id)
            .expect("first claim snapshot")
            .lifecycle_state,
        ClaimLifecycleState::Active
    );
}

#[test]
fn claim_task_rejects_conflicting_active_exclusive_claim_without_partial_rows() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let (task_snapshot, first_session_id) =
        create_task_and_session(&mut engine, &workspace, "Exclusive claim target");
    let second_session = engine
        .start_session(
            SessionStartOptions::new(workspace.workspace_id, workspace.initial_branch_id)
                .expect("session options"),
        )
        .expect("second session");

    engine
        .claim_task(ClaimTaskOptions::new(
            first_session_id,
            task_snapshot.task_entity_id,
        ))
        .expect("first claim");
    let connection = raw_connection(&path);
    let before_conflict = runtime_counts(&connection);

    let error = engine
        .claim_task(ClaimTaskOptions::new(
            second_session.session_id,
            task_snapshot.task_entity_id,
        ))
        .expect_err("second active exclusive claim should fail");
    assert_eq!(error.code(), ErrorCode::ClaimInvalid);
    assert_eq!(error.category(), ErrorCategory::Runtime);
    assert_eq!(runtime_counts(&connection), before_conflict);
}

#[test]
fn shared_claims_coexist_and_block_exclusive_or_duplicate_shared_claims() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let task_snapshot = create_task_snapshot(
        &mut engine,
        &workspace,
        workspace.genesis_commit_id,
        "Shared claim target",
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
    let third_session = engine
        .start_session(
            SessionStartOptions::new(workspace.workspace_id, workspace.initial_branch_id)
                .expect("third session options"),
        )
        .expect("third session");

    let first_shared = engine
        .claim_task(
            ClaimTaskOptions::new(first_session.session_id, task_snapshot.task_entity_id)
                .with_mode(ClaimMode::Shared),
        )
        .expect("first shared claim");
    let second_shared = engine
        .claim_task(
            ClaimTaskOptions::new(second_session.session_id, task_snapshot.task_entity_id)
                .with_mode(ClaimMode::Shared),
        )
        .expect("second shared claim");

    assert_eq!(first_shared.mode, ClaimMode::Shared);
    assert_eq!(first_shared.state.mode, ClaimMode::Shared);
    assert_eq!(second_shared.mode, ClaimMode::Shared);
    assert_eq!(second_shared.state.mode, ClaimMode::Shared);
    assert_ne!(first_shared.claim_id, second_shared.claim_id);

    let connection = raw_connection(&path);
    assert_eq!(
        event_count(&connection, first_session.session_id, "claim.created"),
        1
    );
    assert_eq!(
        latest_event_payload(&connection, first_session.session_id, "claim.created"),
        claim_created_payload_with_mode(
            first_shared.claim_id,
            first_session.session_id,
            &workspace,
            task_snapshot.task_entity_id,
            "shared",
        )
    );
    assert_eq!(
        event_count(&connection, second_session.session_id, "claim.created"),
        1
    );
    assert_eq!(
        latest_event_payload(&connection, second_session.session_id, "claim.created"),
        claim_created_payload_with_mode(
            second_shared.claim_id,
            second_session.session_id,
            &workspace,
            task_snapshot.task_entity_id,
            "shared",
        )
    );

    let before_duplicate = runtime_counts(&connection);
    let duplicate_shared = engine
        .claim_task(
            ClaimTaskOptions::new(first_session.session_id, task_snapshot.task_entity_id)
                .with_mode(ClaimMode::Shared),
        )
        .expect_err("same session duplicate shared claim should fail");
    assert_eq!(duplicate_shared.code(), ErrorCode::ClaimInvalid);
    assert_eq!(duplicate_shared.category(), ErrorCategory::Runtime);
    assert_eq!(runtime_counts(&connection), before_duplicate);

    let exclusive_conflict = engine
        .claim_task(ClaimTaskOptions::new(
            third_session.session_id,
            task_snapshot.task_entity_id,
        ))
        .expect_err("exclusive claim should not join active shared set");
    assert_eq!(exclusive_conflict.code(), ErrorCode::ClaimInvalid);
    assert_eq!(exclusive_conflict.category(), ErrorCategory::Runtime);
    assert_eq!(runtime_counts(&connection), before_duplicate);
}

#[test]
fn active_exclusive_claim_blocks_shared_claim_without_partial_rows() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let (task_snapshot, first_session_id) =
        create_task_and_session(&mut engine, &workspace, "Exclusive blocks shared claim");
    let second_session = engine
        .start_session(
            SessionStartOptions::new(workspace.workspace_id, workspace.initial_branch_id)
                .expect("second session options"),
        )
        .expect("second session");

    engine
        .claim_task(ClaimTaskOptions::new(
            first_session_id,
            task_snapshot.task_entity_id,
        ))
        .expect("exclusive claim");
    let connection = raw_connection(&path);
    let before_conflict = runtime_counts(&connection);

    let error = engine
        .claim_task(
            ClaimTaskOptions::new(second_session.session_id, task_snapshot.task_entity_id)
                .with_mode(ClaimMode::Shared),
        )
        .expect_err("shared claim should fail behind active exclusive claim");
    assert_eq!(error.code(), ErrorCode::ClaimInvalid);
    assert_eq!(error.category(), ErrorCategory::Runtime);
    assert_eq!(runtime_counts(&connection), before_conflict);
}

#[test]
fn release_claim_projects_released_occurrence_without_workstate_mutation() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let (task_snapshot, session_id) =
        create_task_and_session(&mut engine, &workspace, "Release claim target");
    let claimed = engine
        .claim_task(ClaimTaskOptions::new(
            session_id,
            task_snapshot.task_entity_id,
        ))
        .expect("claim task");
    let connection = raw_connection(&path);
    let before_release = runtime_counts(&connection);
    let before_head = engine
        .branch_head(workspace.initial_branch_id)
        .expect("branch head before release");

    let released = engine
        .release_claim(ClaimReleaseOptions::new(session_id, claimed.claim_id))
        .expect("release claim");

    assert_eq!(released.claim_id, claimed.claim_id);
    assert_eq!(released.session_id, session_id);
    assert!(released.released_at_us >= claimed.claimed_at_us);
    assert_eq!(released.state.claim_id, claimed.claim_id);
    assert_eq!(
        released.state.lifecycle_state,
        ClaimLifecycleState::Released
    );
    assert_eq!(released.state.session_id, session_id);
    assert_eq!(released.state.workspace_id, workspace.workspace_id);
    assert_eq!(released.state.branch_id, workspace.initial_branch_id);
    assert_eq!(released.state.task_entity_id, task_snapshot.task_entity_id);
    assert_eq!(released.state.mode, ClaimMode::Exclusive);
    assert_eq!(released.state.created_at_us, claimed.claimed_at_us);
    assert_eq!(released.state.last_activity_at_us, None);

    let after_release = runtime_counts(&connection);
    assert_eq!(
        after_release.object_identity,
        before_release.object_identity
    );
    assert_eq!(
        after_release.session_runtime,
        before_release.session_runtime
    );
    assert_eq!(after_release.claim, before_release.claim);
    assert_eq!(
        after_release.claim_runtime,
        before_release.claim_runtime - 1
    );
    assert_eq!(after_release.session_diff, before_release.session_diff);
    assert_eq!(after_release.changeset, before_release.changeset);
    assert_eq!(
        after_release.change_operation,
        before_release.change_operation
    );
    assert_eq!(
        after_release.workstate_commit,
        before_release.workstate_commit
    );
    assert_eq!(after_release.event, before_release.event + 1);

    let after_head = engine
        .branch_head(workspace.initial_branch_id)
        .expect("branch head after release");
    assert_eq!(after_head.head_commit_id, before_head.head_commit_id);
    assert_eq!(after_head.state_digest, before_head.state_digest);
    let after_task = engine
        .task_at(after_head.head_commit_id, task_snapshot.task_entity_id)
        .expect("task after release");
    assert_eq!(after_task, task_snapshot);
    let after_session = engine
        .session_snapshot(session_id)
        .expect("session after release");
    assert_eq!(
        after_session.last_activity_at_us,
        Some(released.released_at_us)
    );

    assert_eq!(event_count(&connection, session_id, "claim.released"), 1);
    assert_eq!(
        latest_event_payload(&connection, session_id, "claim.released"),
        claim_released_payload(
            claimed.claim_id,
            session_id,
            &workspace,
            task_snapshot.task_entity_id,
        )
    );

    drop(engine);
    let reopened = Engine::open(&path).expect("reopen engine");
    let snapshot = reopened
        .claim_snapshot(claimed.claim_id)
        .expect("released claim snapshot");
    assert_eq!(snapshot, released.state);
}

#[test]
fn transfer_claim_replaces_active_occurrence_without_workstate_mutation() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let (task_snapshot, from_session_id) =
        create_task_and_session(&mut engine, &workspace, "Transfer claim target");
    let to_session = engine
        .start_session(
            SessionStartOptions::new(workspace.workspace_id, workspace.initial_branch_id)
                .expect("session options"),
        )
        .expect("to session");
    let claimed = engine
        .claim_task(ClaimTaskOptions::new(
            from_session_id,
            task_snapshot.task_entity_id,
        ))
        .expect("claim task");
    let connection = raw_connection(&path);
    let before = runtime_counts(&connection);
    let before_head = engine
        .branch_head(workspace.initial_branch_id)
        .expect("branch head before transfer");

    let transferred = engine
        .transfer_claim(ClaimTransferOptions::new(
            from_session_id,
            to_session.session_id,
            claimed.claim_id,
        ))
        .expect("transfer claim");

    assert_eq!(transferred.previous_claim_id, claimed.claim_id);
    assert_ne!(transferred.claim_id, claimed.claim_id);
    assert_eq!(transferred.from_session_id, from_session_id);
    assert_eq!(transferred.to_session_id, to_session.session_id);
    assert!(transferred.transferred_at_us >= claimed.claimed_at_us);
    assert_eq!(
        transferred.previous_last_activity_at_us,
        claimed.claimed_at_us
    );
    assert_eq!(
        transferred.previous_state.lifecycle_state,
        ClaimLifecycleState::Released
    );
    assert_eq!(transferred.previous_state.last_activity_at_us, None);
    assert_eq!(
        transferred.state.lifecycle_state,
        ClaimLifecycleState::Active
    );
    assert_eq!(transferred.state.session_id, to_session.session_id);
    assert_eq!(transferred.state.workspace_id, workspace.workspace_id);
    assert_eq!(transferred.state.branch_id, workspace.initial_branch_id);
    assert_eq!(
        transferred.state.task_entity_id,
        task_snapshot.task_entity_id
    );
    assert_eq!(transferred.state.mode, ClaimMode::Exclusive);

    let after = runtime_counts(&connection);
    assert_eq!(after.object_identity, before.object_identity + 1);
    assert_eq!(after.claim, before.claim + 1);
    assert_eq!(after.claim_runtime, before.claim_runtime);
    assert_eq!(after.session_diff, before.session_diff);
    assert_eq!(after.changeset, before.changeset);
    assert_eq!(after.change_operation, before.change_operation);
    assert_eq!(after.workstate_commit, before.workstate_commit);
    assert_eq!(after.event, before.event + 1);

    let after_head = engine
        .branch_head(workspace.initial_branch_id)
        .expect("branch head after transfer");
    assert_eq!(after_head.head_commit_id, before_head.head_commit_id);
    assert_eq!(after_head.state_digest, before_head.state_digest);
    assert!(
        engine
            .active_claims_for_session(ClaimListOptions::for_session(from_session_id))
            .expect("from claims")
            .claims
            .is_empty()
    );
    let to_claims = engine
        .active_claims_for_session(ClaimListOptions::for_session(to_session.session_id))
        .expect("to claims");
    assert_eq!(to_claims.claims, vec![transferred.state.clone()]);

    assert_eq!(
        event_count(&connection, from_session_id, "claim.transferred"),
        1
    );
    assert_eq!(
        latest_event_payload(&connection, from_session_id, "claim.transferred"),
        claim_transferred_payload(
            claimed.claim_id,
            transferred.claim_id,
            from_session_id,
            to_session.session_id,
            &workspace,
            task_snapshot.task_entity_id,
            transferred.previous_last_activity_at_us,
        )
    );
    let source_guard = engine
        .task_claim_guard(ClaimGuardOptions::terminal_task_mutation(
            from_session_id,
            task_snapshot.task_entity_id,
        ))
        .expect("source guard");
    assert_eq!(
        source_guard.reason,
        ClaimGuardReason::ExclusiveClaimOwnedByOtherSession
    );
    let target_guard = engine
        .task_claim_guard(ClaimGuardOptions::terminal_task_mutation(
            to_session.session_id,
            task_snapshot.task_entity_id,
        ))
        .expect("target guard");
    assert_eq!(target_guard.reason, ClaimGuardReason::OwnedExclusiveClaim);
}

#[test]
fn force_takeover_claim_requires_rationale_and_records_prior_claim() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let (task_snapshot, previous_session_id) =
        create_task_and_session(&mut engine, &workspace, "Force takeover claim target");
    let taking_session = engine
        .start_session(
            SessionStartOptions::new(workspace.workspace_id, workspace.initial_branch_id)
                .expect("session options"),
        )
        .expect("taking session");
    let claimed = engine
        .claim_task(ClaimTaskOptions::new(
            previous_session_id,
            task_snapshot.task_entity_id,
        ))
        .expect("claim task");
    let connection = raw_connection(&path);
    let before = runtime_counts(&connection);
    let empty_rationale =
        ClaimForceTakeoverOptions::new(taking_session.session_id, claimed.claim_id, "   ")
            .expect_err("empty rationale should fail");
    assert_eq!(empty_rationale.code(), ErrorCode::ClaimInvalid);
    assert_eq!(empty_rationale.category(), ErrorCategory::Runtime);
    assert_eq!(runtime_counts(&connection), before);
    let before_head = engine
        .branch_head(workspace.initial_branch_id)
        .expect("branch head before takeover");

    let rationale = "previous agent is unavailable";
    let taken_over = engine
        .force_takeover_claim(
            ClaimForceTakeoverOptions::new(taking_session.session_id, claimed.claim_id, rationale)
                .expect("takeover options"),
        )
        .expect("force takeover");

    assert_eq!(taken_over.previous_claim_id, claimed.claim_id);
    assert_ne!(taken_over.claim_id, claimed.claim_id);
    assert_eq!(taken_over.previous_session_id, previous_session_id);
    assert_eq!(taken_over.session_id, taking_session.session_id);
    assert_eq!(
        taken_over.previous_last_activity_at_us,
        claimed.claimed_at_us
    );
    assert_eq!(taken_over.rationale, rationale);
    assert_eq!(
        taken_over.previous_state.lifecycle_state,
        ClaimLifecycleState::Released
    );
    assert_eq!(taken_over.previous_state.last_activity_at_us, None);
    assert_eq!(
        taken_over.state.lifecycle_state,
        ClaimLifecycleState::Active
    );
    assert_eq!(taken_over.state.session_id, taking_session.session_id);
    assert_eq!(taken_over.state.workspace_id, workspace.workspace_id);
    assert_eq!(taken_over.state.branch_id, workspace.initial_branch_id);
    assert_eq!(
        taken_over.state.task_entity_id,
        task_snapshot.task_entity_id
    );
    assert_eq!(taken_over.state.mode, ClaimMode::Exclusive);

    let after = runtime_counts(&connection);
    assert_eq!(after.object_identity, before.object_identity + 1);
    assert_eq!(after.claim, before.claim + 1);
    assert_eq!(after.claim_runtime, before.claim_runtime);
    assert_eq!(after.session_diff, before.session_diff);
    assert_eq!(after.changeset, before.changeset);
    assert_eq!(after.change_operation, before.change_operation);
    assert_eq!(after.workstate_commit, before.workstate_commit);
    assert_eq!(after.event, before.event + 1);

    let after_head = engine
        .branch_head(workspace.initial_branch_id)
        .expect("branch head after takeover");
    assert_eq!(after_head.head_commit_id, before_head.head_commit_id);
    assert_eq!(after_head.state_digest, before_head.state_digest);
    assert_eq!(
        event_count(
            &connection,
            taking_session.session_id,
            "claim.force_taken_over"
        ),
        1
    );
    assert_eq!(
        latest_event_payload(
            &connection,
            taking_session.session_id,
            "claim.force_taken_over"
        ),
        claim_force_taken_over_payload(ClaimForceTakenOverPayload {
            previous_claim_id: claimed.claim_id,
            claim_id: taken_over.claim_id,
            previous_session_id,
            session_id: taking_session.session_id,
            workspace: &workspace,
            task_entity_id: task_snapshot.task_entity_id,
            previous_last_activity_at_us: taken_over.previous_last_activity_at_us,
            rationale,
        })
    );
}

#[test]
fn release_claim_rejects_non_owner_or_inactive_claim_without_partial_rows() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let (task_snapshot, first_session_id) =
        create_task_and_session(&mut engine, &workspace, "Release ownership target");
    let second_session = engine
        .start_session(
            SessionStartOptions::new(workspace.workspace_id, workspace.initial_branch_id)
                .expect("session options"),
        )
        .expect("second session");
    let claimed = engine
        .claim_task(ClaimTaskOptions::new(
            first_session_id,
            task_snapshot.task_entity_id,
        ))
        .expect("claim task");
    let connection = raw_connection(&path);
    let before_non_owner = runtime_counts(&connection);

    let non_owner = engine
        .release_claim(ClaimReleaseOptions::new(
            second_session.session_id,
            claimed.claim_id,
        ))
        .expect_err("non owner release should fail");
    assert_eq!(non_owner.code(), ErrorCode::ClaimInvalid);
    assert_eq!(non_owner.category(), ErrorCategory::Runtime);
    assert_eq!(runtime_counts(&connection), before_non_owner);

    engine
        .release_claim(ClaimReleaseOptions::new(first_session_id, claimed.claim_id))
        .expect("release claim");
    let after_release = runtime_counts(&connection);
    let duplicate_release = engine
        .release_claim(ClaimReleaseOptions::new(first_session_id, claimed.claim_id))
        .expect_err("duplicate release should fail");
    assert_eq!(duplicate_release.code(), ErrorCode::ClaimInvalid);
    assert_eq!(duplicate_release.category(), ErrorCategory::Runtime);
    assert_eq!(runtime_counts(&connection), after_release);
}

#[test]
fn end_session_releases_owned_claim_runtime_and_records_claim_event() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let (task_snapshot, session_id) =
        create_task_and_session(&mut engine, &workspace, "SessionEnd releases claim");
    let claimed = engine
        .claim_task(ClaimTaskOptions::new(
            session_id,
            task_snapshot.task_entity_id,
        ))
        .expect("claim task");
    let connection = raw_connection(&path);
    let before_end = runtime_counts(&connection);

    let ended = engine
        .end_session(SessionEndOptions::new(session_id).expect("end options"))
        .expect("end session");

    assert!(ended.ended_at_us >= claimed.claimed_at_us);
    let after_end = runtime_counts(&connection);
    assert_eq!(after_end.object_identity, before_end.object_identity + 1);
    assert_eq!(after_end.session_runtime, before_end.session_runtime - 1);
    assert_eq!(after_end.claim, before_end.claim);
    assert_eq!(after_end.claim_runtime, before_end.claim_runtime - 1);
    assert_eq!(after_end.session_diff, before_end.session_diff + 1);
    assert_eq!(after_end.changeset, before_end.changeset);
    assert_eq!(after_end.change_operation, before_end.change_operation);
    assert_eq!(after_end.workstate_commit, before_end.workstate_commit);
    assert_eq!(after_end.event, before_end.event + 2);
    assert_eq!(event_count(&connection, session_id, "claim.released"), 1);
    assert_eq!(event_count(&connection, session_id, "session.ended"), 1);
    assert_eq!(
        latest_event_payload(&connection, session_id, "claim.released"),
        claim_released_payload(
            claimed.claim_id,
            session_id,
            &workspace,
            task_snapshot.task_entity_id,
        )
    );

    let released_claim = engine
        .claim_snapshot(claimed.claim_id)
        .expect("claim snapshot after session end");
    assert_eq!(
        released_claim.lifecycle_state,
        ClaimLifecycleState::Released
    );
    assert_eq!(released_claim.last_activity_at_us, None);
}

#[test]
fn end_session_releases_multiple_owned_claims_atomically() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let first_task = create_task_snapshot(
        &mut engine,
        &workspace,
        workspace.genesis_commit_id,
        "First owned claim",
    );
    let first_head = engine
        .branch_head(workspace.initial_branch_id)
        .expect("first branch head")
        .head_commit_id;
    let second_task =
        create_task_snapshot(&mut engine, &workspace, first_head, "Second owned claim");
    let started = engine
        .start_session(
            SessionStartOptions::new(workspace.workspace_id, workspace.initial_branch_id)
                .expect("session options"),
        )
        .expect("start session");
    let first_claim = engine
        .claim_task(ClaimTaskOptions::new(
            started.session_id,
            first_task.task_entity_id,
        ))
        .expect("first claim");
    let second_claim = engine
        .claim_task(ClaimTaskOptions::new(
            started.session_id,
            second_task.task_entity_id,
        ))
        .expect("second claim");
    let connection = raw_connection(&path);
    let before_end = runtime_counts(&connection);

    engine
        .end_session(SessionEndOptions::new(started.session_id).expect("end options"))
        .expect("end session");

    let after_end = runtime_counts(&connection);
    assert_eq!(after_end.claim, before_end.claim);
    assert_eq!(after_end.claim_runtime, before_end.claim_runtime - 2);
    assert_eq!(after_end.event, before_end.event + 3);
    assert_eq!(
        event_count(&connection, started.session_id, "claim.released"),
        2
    );
    assert_eq!(
        event_payloads(&connection, started.session_id, "claim.released"),
        {
            let mut expected = vec![
                claim_released_payload(
                    first_claim.claim_id,
                    started.session_id,
                    &workspace,
                    first_task.task_entity_id,
                ),
                claim_released_payload(
                    second_claim.claim_id,
                    started.session_id,
                    &workspace,
                    second_task.task_entity_id,
                ),
            ];
            expected.sort();
            expected
        }
    );
    assert_eq!(
        engine
            .claim_snapshot(first_claim.claim_id)
            .expect("first claim")
            .lifecycle_state,
        ClaimLifecycleState::Released
    );
    assert_eq!(
        engine
            .claim_snapshot(second_claim.claim_id)
            .expect("second claim")
            .lifecycle_state,
        ClaimLifecycleState::Released
    );
}

#[test]
fn claim_task_rejects_invalid_session_or_missing_task_without_partial_rows() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let (task_snapshot, session_id) =
        create_task_and_session(&mut engine, &workspace, "Invalid claim inputs");
    let connection = raw_connection(&path);
    let before = runtime_counts(&connection);

    let missing_session = engine
        .claim_task(ClaimTaskOptions::new(
            workvcs_core::SessionId::new_v7(),
            task_snapshot.task_entity_id,
        ))
        .expect_err("missing session should fail");
    assert_eq!(missing_session.code(), ErrorCode::SessionNotFound);
    assert_eq!(missing_session.category(), ErrorCategory::Runtime);

    let missing_task = engine
        .claim_task(ClaimTaskOptions::new(session_id, EntityId::new_v7()))
        .expect_err("missing task should fail");
    assert_eq!(missing_task.code(), ErrorCode::TaskNotFound);
    assert_eq!(missing_task.category(), ErrorCategory::Task);

    engine
        .end_session(SessionEndOptions::new(session_id).expect("end options"))
        .expect("end session");
    let after_end = runtime_counts(&connection);
    let inactive_session = engine
        .claim_task(ClaimTaskOptions::new(
            session_id,
            task_snapshot.task_entity_id,
        ))
        .expect_err("inactive session should fail");
    assert_eq!(inactive_session.code(), ErrorCode::SessionInvalid);
    assert_eq!(inactive_session.category(), ErrorCategory::Runtime);

    assert_eq!(before.claim, 0);
    assert_eq!(before.claim_runtime, 0);
    assert_eq!(runtime_counts(&connection), after_end);
}

#[test]
fn claim_snapshot_rejects_unknown_claim() {
    let (_tempdir, path) = store_path();
    let (engine, _workspace) = create_workspace(&path);

    let error = engine
        .claim_snapshot(ClaimId::new_v7())
        .expect_err("unknown claim should fail");
    assert_eq!(error.code(), ErrorCode::ClaimNotFound);
    assert_eq!(error.category(), ErrorCategory::Runtime);
}

#[test]
fn claim_task_rejects_non_task_entity_at_active_head_without_runtime_rows() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let task = engine
        .create_task(
            TaskCreateOptions::new(
                workspace.initial_branch_id,
                workspace.genesis_commit_id,
                "Task with criterion",
            )
            .expect("task options"),
        )
        .expect("create task");
    let criterion = engine
        .create_acceptance_criterion(
            workvcs_core::AcceptanceCriterionCreateOptions::new(
                workspace.initial_branch_id,
                task.commit_id,
                task.task_entity_id,
                task.task_entity_version_id,
                "AC-claim",
                "A non-task entity cannot be claimed as a task.",
                workvcs_core::AcceptanceCriterionClassification::Required,
            )
            .expect("criterion options"),
        )
        .expect("create criterion");
    let started = engine
        .start_session(
            SessionStartOptions::new(workspace.workspace_id, workspace.initial_branch_id)
                .expect("session options"),
        )
        .expect("start session");
    let connection = raw_connection(&path);
    let before = runtime_counts(&connection);

    let error = engine
        .claim_task(ClaimTaskOptions::new(
            started.session_id,
            criterion.acceptance_criterion_entity_id,
        ))
        .expect_err("non-task entity should fail");
    assert_eq!(error.code(), ErrorCode::TaskNotFound);
    assert_eq!(error.category(), ErrorCategory::Task);
    assert_eq!(runtime_counts(&connection), before);
}

#[test]
fn claim_task_rejects_wrong_branch_without_runtime_rows() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let task = engine
        .create_task(
            TaskCreateOptions::new(
                workspace.initial_branch_id,
                workspace.genesis_commit_id,
                "Branch-scoped claim",
            )
            .expect("task options"),
        )
        .expect("create task");
    let other_workspace = engine
        .create_workspace(WorkspaceInitOptions::new("other").expect("workspace options"))
        .expect("other workspace");
    let other_session = engine
        .start_session(
            SessionStartOptions::new(
                other_workspace.workspace_id,
                other_workspace.initial_branch_id,
            )
            .expect("session options"),
        )
        .expect("other session");
    let connection = raw_connection(&path);
    let before = runtime_counts(&connection);

    let error = engine
        .claim_task(ClaimTaskOptions::new(
            other_session.session_id,
            task.task_entity_id,
        ))
        .expect_err("task absent from other branch should fail");
    assert_eq!(error.code(), ErrorCode::TaskNotFound);
    assert_eq!(error.category(), ErrorCategory::Task);
    assert_eq!(runtime_counts(&connection), before);
}

#[test]
fn claim_task_rejects_inactive_branch_without_runtime_rows() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let (task_snapshot, session_id) =
        create_task_and_session(&mut engine, &workspace, "Inactive branch claim");
    let connection = raw_connection(&path);
    let before = runtime_counts(&connection);
    let branch_id = workspace.initial_branch_id.raw_bytes();
    connection
        .execute(
            "UPDATE branch
             SET lifecycle_state = 'closed'
             WHERE branch_id = ?1",
            params![&branch_id[..]],
        )
        .expect("close branch");

    let error = engine
        .claim_task(ClaimTaskOptions::new(
            session_id,
            task_snapshot.task_entity_id,
        ))
        .expect_err("inactive branch should fail");
    assert_eq!(error.code(), ErrorCode::ClaimInvalid);
    assert_eq!(error.category(), ErrorCategory::Runtime);
    assert_eq!(runtime_counts(&connection), before);
}
