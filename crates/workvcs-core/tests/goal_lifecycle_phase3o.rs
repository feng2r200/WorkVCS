use rusqlite::{Connection, params};
use std::path::{Path, PathBuf};
use tempfile::TempDir;
use workvcs_core::{
    BranchId, CanonicalValue, CommitId, Digest, Engine, EntityVersionId, ErrorCategory, ErrorCode,
    GoalCreateOptions, GoalState, GoalStatus, GoalTransitionOptions, StoreInitOptions,
    TaskCreateOptions, WorkspaceInfo, WorkspaceInitOptions, canonical_bytes, entity_version_digest,
};

#[derive(Clone, Debug, PartialEq, Eq)]
struct HistoryCounts {
    object_identity: i64,
    entity: i64,
    entity_version: i64,
    changeset: i64,
    change_operation: i64,
    entity_membership_change: i64,
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
        StoreInitOptions::new("phase3o-goal-lifecycle-store").expect("store options"),
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
        entity_version: count_rows(connection, "entity_version"),
        changeset: count_rows(connection, "changeset"),
        change_operation: count_rows(connection, "change_operation"),
        entity_membership_change: count_rows(connection, "entity_membership_change"),
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

fn digest_from_blob(bytes: Vec<u8>) -> Digest {
    Digest::from_bytes(bytes.try_into().expect("digest bytes"))
}

fn canonical_json(value: &CanonicalValue) -> String {
    String::from_utf8(canonical_bytes(value).expect("canonical bytes")).expect("canonical json")
}

fn goal_state_json(description: &str, status: GoalStatus, rationale: Option<&str>) -> String {
    let state = GoalState {
        description: description.to_owned(),
        status,
        terminal_rationale: rationale.map(str::to_owned),
    };
    canonical_json(&state.to_canonical_value().expect("canonical goal state"))
}

fn rationale(reason: &str) -> CanonicalValue {
    CanonicalValue::object(vec![(
        "reason".to_owned(),
        CanonicalValue::String(reason.to_owned()),
    )])
    .expect("rationale")
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

fn corrupt_goal_state(
    connection: &Connection,
    entity_version_id: EntityVersionId,
    status: GoalStatus,
    terminal_rationale: CanonicalValue,
) -> Digest {
    let corrupted = CanonicalValue::object(vec![
        (
            "description".to_owned(),
            CanonicalValue::String("Corrupted goal".to_owned()),
        ),
        ("plan_refs".to_owned(), CanonicalValue::Array(Vec::new())),
        (
            "status".to_owned(),
            CanonicalValue::String(status.as_str().to_owned()),
        ),
        ("subgoals".to_owned(), CanonicalValue::Array(Vec::new())),
        ("terminal_rationale".to_owned(), terminal_rationale),
        ("work_refs".to_owned(), CanonicalValue::Array(Vec::new())),
    ])
    .expect("corrupted goal state");
    let state_json = canonical_json(&corrupted);
    let digest = entity_version_digest(&corrupted).expect("entity digest");
    connection
        .execute(
            "UPDATE entity_version
             SET state_json = ?1,
                 state_digest = ?2
             WHERE entity_version_id = ?3",
            params![
                state_json,
                &digest.as_bytes()[..],
                &entity_version_id.raw_bytes()[..]
            ],
        )
        .expect("corrupt goal state");
    digest
}

#[test]
fn goal_can_be_achieved_and_reopened_with_historical_snapshots() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);

    let created = engine
        .create_goal(
            GoalCreateOptions::new(
                workspace.initial_branch_id,
                workspace.genesis_commit_id,
                "Deliver deterministic state transitions",
            )
            .expect("goal options"),
        )
        .expect("create goal");
    let achieved = engine
        .transition_goal(
            GoalTransitionOptions::achieve(
                workspace.initial_branch_id,
                created.commit_id,
                created.goal_entity_id,
                created.goal_entity_version_id,
                "All required verification passed",
            )
            .expect("achieve options"),
        )
        .expect("achieve goal");
    let reopened = engine
        .transition_goal(
            GoalTransitionOptions::reopen(
                workspace.initial_branch_id,
                achieved.commit_id,
                achieved.goal_entity_id,
                achieved.goal_entity_version_id,
                "A new dependency invalidated the result",
            )
            .expect("reopen options"),
        )
        .expect("reopen goal");

    assert_eq!(achieved.workspace_id, workspace.workspace_id);
    assert_eq!(achieved.previous_head_commit_id, created.commit_id);
    assert_eq!(
        achieved.previous_goal_entity_version_id,
        created.goal_entity_version_id
    );
    assert_eq!(achieved.goal_entity_id, created.goal_entity_id);
    assert_ne!(
        achieved.goal_entity_version_id,
        created.goal_entity_version_id
    );
    assert_eq!(achieved.previous_state.status, GoalStatus::Active);
    assert_eq!(achieved.state.status, GoalStatus::Achieved);
    assert_eq!(
        achieved.state.terminal_rationale.as_deref(),
        Some("All required verification passed")
    );

    assert_eq!(reopened.goal_entity_id, created.goal_entity_id);
    assert_eq!(reopened.previous_state, achieved.state);
    assert_eq!(reopened.state.status, GoalStatus::Active);
    assert_eq!(reopened.state.terminal_rationale, None);

    let created_snapshot = engine
        .goal_at(created.commit_id, created.goal_entity_id)
        .expect("created snapshot");
    let achieved_snapshot = engine
        .goal_at(achieved.commit_id, achieved.goal_entity_id)
        .expect("achieved snapshot");
    let reopened_snapshot = engine
        .goal_at(reopened.commit_id, reopened.goal_entity_id)
        .expect("reopened snapshot");
    assert_eq!(created_snapshot.state.status, GoalStatus::Active);
    assert_eq!(created_snapshot.state.terminal_rationale, None);
    assert_eq!(achieved_snapshot.state, achieved.state);
    assert_eq!(reopened_snapshot.state, reopened.state);

    let connection = raw_connection(&path);
    assert_eq!(
        history_counts(&connection),
        HistoryCounts {
            object_identity: 1,
            entity: 1,
            entity_version: 3,
            changeset: 4,
            change_operation: 3,
            entity_membership_change: 3,
            workstate_commit: 4,
            commit_parent: 3,
            event: 4,
        }
    );
    assert_eq!(
        branch_head(&connection, workspace.initial_branch_id),
        reopened.commit_id
    );
    drop(connection);
    drop(engine);

    let reopened_engine = Engine::open(&path).expect("reopen engine");
    assert_eq!(
        reopened_engine
            .goal_at(reopened.commit_id, reopened.goal_entity_id)
            .expect("reopened engine snapshot"),
        reopened_snapshot
    );
}

#[test]
fn goal_abandon_transition_uses_entity_transition_storage_shape_and_rationale() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);

    let created = engine
        .create_goal(
            GoalCreateOptions::new(
                workspace.initial_branch_id,
                workspace.genesis_commit_id,
                "Retire unsupported strategy",
            )
            .expect("goal options"),
        )
        .expect("create goal");
    let abandoned = engine
        .transition_goal(
            GoalTransitionOptions::abandon(
                workspace.initial_branch_id,
                created.commit_id,
                created.goal_entity_id,
                created.goal_entity_version_id,
                "The strategy no longer matches confirmed scope",
            )
            .expect("abandon options"),
        )
        .expect("abandon goal");

    assert_eq!(abandoned.state.status, GoalStatus::Abandoned);
    assert_eq!(
        abandoned.state.terminal_rationale.as_deref(),
        Some("The strategy no longer matches confirmed scope")
    );

    let connection = raw_connection(&path);
    let goal_entity_id = abandoned.goal_entity_id.raw_bytes();
    let goal_entity_version_id = abandoned.goal_entity_version_id.raw_bytes();
    let (
        object_kind,
        entity_kind,
        state_schema_version,
        state_json,
        state_digest,
        operation_type,
        changeset_rationale_json,
        subject_family,
    ) = connection
        .query_row(
            "SELECT object_identity.object_kind,
                    entity.entity_kind,
                    entity_version.state_schema_version,
                    entity_version.state_json,
                    entity_version.state_digest,
                    changeset.operation_type,
                    changeset.rationale_json,
                    change_operation.subject_family
             FROM entity
             JOIN object_identity
               ON object_identity.object_id = entity.object_id
             JOIN entity_version
               ON entity_version.entity_id = entity.object_id
             JOIN entity_membership_change
               ON entity_membership_change.after_entity_version_id = entity_version.entity_version_id
             JOIN change_operation
               ON change_operation.operation_id = entity_membership_change.operation_id
             JOIN changeset
               ON changeset.changeset_id = change_operation.changeset_id
             WHERE entity.object_id = ?1
               AND entity_version.entity_version_id = ?2",
            params![&goal_entity_id[..], &goal_entity_version_id[..]],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, Vec<u8>>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, String>(6)?,
                    row.get::<_, String>(7)?,
                ))
            },
        )
        .expect("goal transition storage shape");

    assert_eq!(object_kind, "entity");
    assert_eq!(entity_kind, "goal");
    assert_eq!(state_schema_version, 1);
    assert_eq!(
        state_json,
        goal_state_json(
            "Retire unsupported strategy",
            GoalStatus::Abandoned,
            Some("The strategy no longer matches confirmed scope")
        )
    );
    assert_eq!(digest_from_blob(state_digest), abandoned.goal_state_digest);
    assert_eq!(operation_type, "entity.transition");
    assert_eq!(
        changeset_rationale_json,
        canonical_json(&rationale("The strategy no longer matches confirmed scope"))
    );
    assert_eq!(subject_family, "entity");
}

#[test]
fn abandoned_goal_can_be_reopened_and_clears_terminal_rationale() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);

    let created = engine
        .create_goal(
            GoalCreateOptions::new(
                workspace.initial_branch_id,
                workspace.genesis_commit_id,
                "Restore abandoned Goal later",
            )
            .expect("goal options"),
        )
        .expect("create goal");
    let abandoned = engine
        .transition_goal(
            GoalTransitionOptions::abandon(
                workspace.initial_branch_id,
                created.commit_id,
                created.goal_entity_id,
                created.goal_entity_version_id,
                "No longer fits the active plan",
            )
            .expect("abandon options"),
        )
        .expect("abandon goal");
    let reopened = engine
        .transition_goal(
            GoalTransitionOptions::reopen(
                workspace.initial_branch_id,
                abandoned.commit_id,
                abandoned.goal_entity_id,
                abandoned.goal_entity_version_id,
                "Scope changed and the Goal is relevant again",
            )
            .expect("reopen abandoned options"),
        )
        .expect("reopen abandoned goal");

    assert_eq!(abandoned.state.status, GoalStatus::Abandoned);
    assert_eq!(
        abandoned.state.terminal_rationale.as_deref(),
        Some("No longer fits the active plan")
    );
    assert_eq!(reopened.previous_state, abandoned.state);
    assert_eq!(reopened.state.status, GoalStatus::Active);
    assert_eq!(reopened.state.terminal_rationale, None);

    let abandoned_snapshot = engine
        .goal_at(abandoned.commit_id, abandoned.goal_entity_id)
        .expect("abandoned snapshot");
    let reopened_snapshot = engine
        .goal_at(reopened.commit_id, reopened.goal_entity_id)
        .expect("reopened snapshot");
    assert_eq!(abandoned_snapshot.state, abandoned.state);
    assert_eq!(reopened_snapshot.state, reopened.state);

    let connection = raw_connection(&path);
    assert_eq!(
        history_counts(&connection),
        HistoryCounts {
            object_identity: 1,
            entity: 1,
            entity_version: 3,
            changeset: 4,
            change_operation: 3,
            entity_membership_change: 3,
            workstate_commit: 4,
            commit_parent: 3,
            event: 4,
        }
    );
    assert_eq!(
        branch_head(&connection, workspace.initial_branch_id),
        reopened.commit_id
    );
}

#[test]
fn goal_transition_rejects_invalid_lifecycle_edges_without_history_rows() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);

    let created = engine
        .create_goal(
            GoalCreateOptions::new(
                workspace.initial_branch_id,
                workspace.genesis_commit_id,
                "Reject invalid Goal transitions",
            )
            .expect("goal options"),
        )
        .expect("create goal");

    let empty_reason = GoalTransitionOptions::achieve(
        workspace.initial_branch_id,
        created.commit_id,
        created.goal_entity_id,
        created.goal_entity_version_id,
        "   ",
    )
    .expect_err("empty terminal rationale");
    assert_eq!(empty_reason.code(), ErrorCode::GoalInvalid);
    assert_eq!(empty_reason.category(), ErrorCategory::Goal);

    let connection = raw_connection(&path);
    let before_invalid = history_counts(&connection);
    let before_head = branch_head(&connection, workspace.initial_branch_id);
    drop(connection);

    let active_to_active = engine
        .transition_goal(
            GoalTransitionOptions::new(
                workspace.initial_branch_id,
                created.commit_id,
                created.goal_entity_id,
                created.goal_entity_version_id,
                GoalStatus::Active,
                "Nothing changed",
            )
            .expect("active options"),
        )
        .expect_err("active to active rejected");
    assert_eq!(active_to_active.code(), ErrorCode::GoalInvalid);
    assert_counts_and_head_unchanged(
        &path,
        &before_invalid,
        workspace.initial_branch_id,
        before_head,
    );

    let empty_operation_rationale = engine
        .transition_goal(
            GoalTransitionOptions::achieve(
                workspace.initial_branch_id,
                created.commit_id,
                created.goal_entity_id,
                created.goal_entity_version_id,
                "Terminal reason exists",
            )
            .expect("achieve options")
            .with_rationale(CanonicalValue::object(Vec::new()).expect("empty object")),
        )
        .expect_err("empty operation rationale rejected");
    assert_eq!(empty_operation_rationale.code(), ErrorCode::GoalInvalid);
    assert_counts_and_head_unchanged(
        &path,
        &before_invalid,
        workspace.initial_branch_id,
        before_head,
    );

    let achieved = engine
        .transition_goal(
            GoalTransitionOptions::achieve(
                workspace.initial_branch_id,
                created.commit_id,
                created.goal_entity_id,
                created.goal_entity_version_id,
                "Achieved for now",
            )
            .expect("achieve options"),
        )
        .expect("achieve goal");
    let connection = raw_connection(&path);
    let before_terminal_invalid = history_counts(&connection);
    let before_terminal_head = branch_head(&connection, workspace.initial_branch_id);
    drop(connection);

    let terminal_to_terminal = engine
        .transition_goal(
            GoalTransitionOptions::abandon(
                workspace.initial_branch_id,
                achieved.commit_id,
                achieved.goal_entity_id,
                achieved.goal_entity_version_id,
                "Change directly to abandoned",
            )
            .expect("abandon options"),
        )
        .expect_err("terminal to terminal rejected");
    assert_eq!(terminal_to_terminal.code(), ErrorCode::GoalInvalid);
    assert_counts_and_head_unchanged(
        &path,
        &before_terminal_invalid,
        workspace.initial_branch_id,
        before_terminal_head,
    );

    let terminal_noop = engine
        .transition_goal(
            GoalTransitionOptions::new(
                workspace.initial_branch_id,
                achieved.commit_id,
                achieved.goal_entity_id,
                achieved.goal_entity_version_id,
                GoalStatus::Achieved,
                "Still achieved",
            )
            .expect("terminal noop options"),
        )
        .expect_err("terminal noop rejected");
    assert_eq!(terminal_noop.code(), ErrorCode::GoalInvalid);
    assert_counts_and_head_unchanged(
        &path,
        &before_terminal_invalid,
        workspace.initial_branch_id,
        before_terminal_head,
    );

    let second = engine
        .create_goal(
            GoalCreateOptions::new(
                workspace.initial_branch_id,
                achieved.commit_id,
                "Reject abandoned terminal edges",
            )
            .expect("second goal options"),
        )
        .expect("second goal");
    let abandoned = engine
        .transition_goal(
            GoalTransitionOptions::abandon(
                workspace.initial_branch_id,
                second.commit_id,
                second.goal_entity_id,
                second.goal_entity_version_id,
                "No longer applicable",
            )
            .expect("abandon second options"),
        )
        .expect("abandon second goal");
    let connection = raw_connection(&path);
    let before_abandoned_invalid = history_counts(&connection);
    let before_abandoned_head = branch_head(&connection, workspace.initial_branch_id);
    drop(connection);

    let abandoned_to_achieved = engine
        .transition_goal(
            GoalTransitionOptions::achieve(
                workspace.initial_branch_id,
                abandoned.commit_id,
                abandoned.goal_entity_id,
                abandoned.goal_entity_version_id,
                "Direct terminal switch",
            )
            .expect("abandoned to achieved options"),
        )
        .expect_err("abandoned to achieved rejected");
    assert_eq!(abandoned_to_achieved.code(), ErrorCode::GoalInvalid);
    assert_counts_and_head_unchanged(
        &path,
        &before_abandoned_invalid,
        workspace.initial_branch_id,
        before_abandoned_head,
    );

    let abandoned_noop = engine
        .transition_goal(
            GoalTransitionOptions::new(
                workspace.initial_branch_id,
                abandoned.commit_id,
                abandoned.goal_entity_id,
                abandoned.goal_entity_version_id,
                GoalStatus::Abandoned,
                "Still abandoned",
            )
            .expect("abandoned noop options"),
        )
        .expect_err("abandoned noop rejected");
    assert_eq!(abandoned_noop.code(), ErrorCode::GoalInvalid);
    assert_counts_and_head_unchanged(
        &path,
        &before_abandoned_invalid,
        workspace.initial_branch_id,
        before_abandoned_head,
    );
}

#[test]
fn goal_transition_rejects_stale_version_stale_head_and_non_goal_without_partial_rows() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);

    let created = engine
        .create_goal(
            GoalCreateOptions::new(
                workspace.initial_branch_id,
                workspace.genesis_commit_id,
                "Protect transition CAS",
            )
            .expect("goal options"),
        )
        .expect("create goal");
    let achieved = engine
        .transition_goal(
            GoalTransitionOptions::achieve(
                workspace.initial_branch_id,
                created.commit_id,
                created.goal_entity_id,
                created.goal_entity_version_id,
                "The target is satisfied",
            )
            .expect("achieve options"),
        )
        .expect("achieve goal");

    let connection = raw_connection(&path);
    let before_invalid = history_counts(&connection);
    let before_head = branch_head(&connection, workspace.initial_branch_id);
    drop(connection);

    let stale_version = engine
        .transition_goal(
            GoalTransitionOptions::reopen(
                workspace.initial_branch_id,
                achieved.commit_id,
                achieved.goal_entity_id,
                created.goal_entity_version_id,
                "Retry with stale entity version",
            )
            .expect("reopen options"),
        )
        .expect_err("stale goal version rejected");
    assert_eq!(stale_version.code(), ErrorCode::GoalInvalid);
    assert_counts_and_head_unchanged(
        &path,
        &before_invalid,
        workspace.initial_branch_id,
        before_head,
    );

    let task = engine
        .create_task(
            TaskCreateOptions::new(
                workspace.initial_branch_id,
                achieved.commit_id,
                "Non-goal entity",
            )
            .expect("task options"),
        )
        .expect("create task");
    let connection = raw_connection(&path);
    let before_non_goal = history_counts(&connection);
    let before_non_goal_head = branch_head(&connection, workspace.initial_branch_id);
    drop(connection);

    let non_goal = engine
        .transition_goal(
            GoalTransitionOptions::achieve(
                workspace.initial_branch_id,
                task.commit_id,
                task.task_entity_id,
                task.task_entity_version_id,
                "Wrong semantic family",
            )
            .expect("achieve options"),
        )
        .expect_err("non-goal entity rejected");
    assert_eq!(non_goal.code(), ErrorCode::GoalNotFound);
    assert_counts_and_head_unchanged(
        &path,
        &before_non_goal,
        workspace.initial_branch_id,
        before_non_goal_head,
    );

    let mut second_engine = Engine::open(&path).expect("second engine");
    let branch_mover = second_engine
        .create_goal(
            GoalCreateOptions::new(
                workspace.initial_branch_id,
                task.commit_id,
                "Move the branch head",
            )
            .expect("branch mover options"),
        )
        .expect("branch mover");
    let connection = raw_connection(&path);
    let before_stale_head = history_counts(&connection);
    let current_head = branch_head(&connection, workspace.initial_branch_id);
    assert_eq!(current_head, branch_mover.commit_id);
    drop(connection);

    let stale_head = engine
        .transition_goal(
            GoalTransitionOptions::reopen(
                workspace.initial_branch_id,
                achieved.commit_id,
                achieved.goal_entity_id,
                achieved.goal_entity_version_id,
                "Retry after branch moved",
            )
            .expect("stale head options"),
        )
        .expect_err("stale branch head rejected");
    assert_eq!(stale_head.code(), ErrorCode::BranchHeadConflict);
    assert_eq!(stale_head.category(), ErrorCategory::Mutation);
    assert_counts_and_head_unchanged(
        &path,
        &before_stale_head,
        workspace.initial_branch_id,
        branch_mover.commit_id,
    );
}

#[test]
fn goal_state_requires_status_and_terminal_rationale_consistency() {
    let active_with_rationale = GoalState {
        description: "Active must not keep terminal reason".to_owned(),
        status: GoalStatus::Active,
        terminal_rationale: Some("stale reason".to_owned()),
    }
    .to_canonical_value()
    .expect_err("active with terminal rationale rejected");
    assert_eq!(active_with_rationale.code(), ErrorCode::GoalInvalid);

    let achieved_without_rationale = GoalState {
        description: "Terminal needs reason".to_owned(),
        status: GoalStatus::Achieved,
        terminal_rationale: None,
    }
    .to_canonical_value()
    .expect_err("terminal without rationale rejected");
    assert_eq!(achieved_without_rationale.code(), ErrorCode::GoalInvalid);

    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let created = engine
        .create_goal(
            GoalCreateOptions::new(
                workspace.initial_branch_id,
                workspace.genesis_commit_id,
                "Reject corrupted terminal state",
            )
            .expect("goal options"),
        )
        .expect("create goal");
    let connection = raw_connection(&path);
    corrupt_goal_state(
        &connection,
        created.goal_entity_version_id,
        GoalStatus::Achieved,
        CanonicalValue::Null,
    );
    drop(connection);

    let corrupted = engine
        .goal_at(created.commit_id, created.goal_entity_id)
        .expect_err("corrupted goal state rejected");
    assert_eq!(corrupted.code(), ErrorCode::GoalInvalid);
}
