use rusqlite::{Connection, params};
use std::path::{Path, PathBuf};
use tempfile::TempDir;
use workvcs_core::{
    BranchId, CanonicalValue, CommitId, Digest, Engine, EntityVersionId, ErrorCategory, ErrorCode,
    PlanCreateOptions, PlanState, PlanStatus, PlanTransitionOptions, StoreInitOptions,
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
        StoreInitOptions::new("phase3p-plan-lifecycle-store").expect("store options"),
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

fn rationale(reason: &str) -> CanonicalValue {
    CanonicalValue::object(vec![(
        "reason".to_owned(),
        CanonicalValue::String(reason.to_owned()),
    )])
    .expect("rationale")
}

fn plan_state_json(
    description: &str,
    strategy: &str,
    constraints: Vec<&str>,
    status: PlanStatus,
    completion_rationale: Option<&str>,
) -> String {
    let mut state = PlanState::active(description, strategy).expect("plan state");
    state.constraints = constraints.into_iter().map(str::to_owned).collect();
    state.status = status;
    state.completion_rationale = completion_rationale.map(str::to_owned);
    canonical_json(&state.to_canonical_value().expect("canonical plan state"))
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

fn corrupt_plan_state(
    connection: &Connection,
    entity_version_id: EntityVersionId,
    status: PlanStatus,
    completion_rationale: CanonicalValue,
) -> Digest {
    let corrupted = CanonicalValue::object(vec![
        ("assumptions".to_owned(), CanonicalValue::Array(Vec::new())),
        ("child_order".to_owned(), CanonicalValue::Array(Vec::new())),
        ("completion_rationale".to_owned(), completion_rationale),
        ("constraints".to_owned(), CanonicalValue::Array(Vec::new())),
        (
            "description".to_owned(),
            CanonicalValue::String("Corrupted plan".to_owned()),
        ),
        (
            "status".to_owned(),
            CanonicalValue::String(status.as_str().to_owned()),
        ),
        (
            "strategy".to_owned(),
            CanonicalValue::String("Inspect the corrupted state".to_owned()),
        ),
        ("task_refs".to_owned(), CanonicalValue::Array(Vec::new())),
    ])
    .expect("corrupted plan state");
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
        .expect("corrupt plan state");
    digest
}

#[test]
fn plan_can_be_completed_and_reopened_with_historical_snapshots() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);

    let created = engine
        .create_plan(
            PlanCreateOptions::new(
                workspace.initial_branch_id,
                workspace.genesis_commit_id,
                "Complete persistence plan",
                "Use entity transition history",
            )
            .expect("plan options")
            .with_constraints(vec!["preserve schema", "keep public facade semantic"])
            .expect("constraints"),
        )
        .expect("create plan");
    let completed = engine
        .transition_plan(
            PlanTransitionOptions::complete(
                workspace.initial_branch_id,
                created.commit_id,
                created.plan_entity_id,
                created.plan_entity_version_id,
            )
            .expect("complete options")
            .with_completion_rationale("Implementation slice passed validation")
            .expect("completion rationale"),
        )
        .expect("complete plan");
    let reopened = engine
        .transition_plan(
            PlanTransitionOptions::reopen(
                workspace.initial_branch_id,
                completed.commit_id,
                completed.plan_entity_id,
                completed.plan_entity_version_id,
                "A later finding reopened the plan",
            )
            .expect("reopen options"),
        )
        .expect("reopen plan");

    assert_eq!(completed.workspace_id, workspace.workspace_id);
    assert_eq!(completed.previous_head_commit_id, created.commit_id);
    assert_eq!(
        completed.previous_plan_entity_version_id,
        created.plan_entity_version_id
    );
    assert_eq!(completed.plan_entity_id, created.plan_entity_id);
    assert_ne!(
        completed.plan_entity_version_id,
        created.plan_entity_version_id
    );
    assert_eq!(completed.previous_state.status, PlanStatus::Active);
    assert_eq!(completed.state.status, PlanStatus::Completed);
    assert_eq!(
        completed.state.completion_rationale.as_deref(),
        Some("Implementation slice passed validation")
    );
    assert_eq!(completed.state.constraints, created.state.constraints);
    assert_eq!(completed.state.strategy, created.state.strategy);

    assert_eq!(reopened.plan_entity_id, created.plan_entity_id);
    assert_eq!(reopened.previous_state, completed.state);
    assert_eq!(reopened.state.status, PlanStatus::Active);
    assert_eq!(reopened.state.completion_rationale, None);
    assert_eq!(reopened.state.constraints, created.state.constraints);
    assert_eq!(reopened.state.strategy, created.state.strategy);

    let created_snapshot = engine
        .plan_at(created.commit_id, created.plan_entity_id)
        .expect("created snapshot");
    let completed_snapshot = engine
        .plan_at(completed.commit_id, completed.plan_entity_id)
        .expect("completed snapshot");
    let reopened_snapshot = engine
        .plan_at(reopened.commit_id, reopened.plan_entity_id)
        .expect("reopened snapshot");
    assert_eq!(created_snapshot.state.status, PlanStatus::Active);
    assert_eq!(created_snapshot.state.completion_rationale, None);
    assert_eq!(completed_snapshot.state, completed.state);
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
            .plan_at(reopened.commit_id, reopened.plan_entity_id)
            .expect("reopened engine snapshot"),
        reopened_snapshot
    );
}

#[test]
fn plan_completion_uses_entity_transition_storage_shape_and_optional_rationale() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);

    let created = engine
        .create_plan(
            PlanCreateOptions::new(
                workspace.initial_branch_id,
                workspace.genesis_commit_id,
                "Persist completion shape",
                "Reuse entity transition rows",
            )
            .expect("plan options"),
        )
        .expect("create plan");
    let completed_without_rationale = engine
        .transition_plan(
            PlanTransitionOptions::complete(
                workspace.initial_branch_id,
                created.commit_id,
                created.plan_entity_id,
                created.plan_entity_version_id,
            )
            .expect("complete options"),
        )
        .expect("complete without rationale");
    assert_eq!(completed_without_rationale.state.completion_rationale, None);
    let reopened = engine
        .transition_plan(
            PlanTransitionOptions::reopen(
                workspace.initial_branch_id,
                completed_without_rationale.commit_id,
                completed_without_rationale.plan_entity_id,
                completed_without_rationale.plan_entity_version_id,
                "Need storage-shape completion rationale case",
            )
            .expect("reopen options"),
        )
        .expect("reopen plan");
    let completed = engine
        .transition_plan(
            PlanTransitionOptions::complete(
                workspace.initial_branch_id,
                reopened.commit_id,
                reopened.plan_entity_id,
                reopened.plan_entity_version_id,
            )
            .expect("complete options")
            .with_completion_rationale("The plan storage shape is stable")
            .expect("completion rationale"),
        )
        .expect("complete with rationale");

    let connection = raw_connection(&path);
    let plan_entity_id = completed.plan_entity_id.raw_bytes();
    let plan_entity_version_id = completed.plan_entity_version_id.raw_bytes();
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
            params![&plan_entity_id[..], &plan_entity_version_id[..]],
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
        .expect("plan transition storage shape");

    assert_eq!(object_kind, "entity");
    assert_eq!(entity_kind, "plan");
    assert_eq!(state_schema_version, 1);
    assert_eq!(
        state_json,
        plan_state_json(
            "Persist completion shape",
            "Reuse entity transition rows",
            Vec::new(),
            PlanStatus::Completed,
            Some("The plan storage shape is stable")
        )
    );
    assert_eq!(digest_from_blob(state_digest), completed.plan_state_digest);
    assert_eq!(operation_type, "entity.transition");
    assert_eq!(
        changeset_rationale_json,
        canonical_json(&rationale("The plan storage shape is stable"))
    );
    assert_eq!(subject_family, "entity");
}

#[test]
fn abandoned_plan_can_be_reopened_and_keeps_completion_rationale_clear() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);

    let created = engine
        .create_plan(
            PlanCreateOptions::new(
                workspace.initial_branch_id,
                workspace.genesis_commit_id,
                "Abandon obsolete plan",
                "Use a strategy that is no longer relevant",
            )
            .expect("plan options"),
        )
        .expect("create plan");
    let abandoned = engine
        .transition_plan(
            PlanTransitionOptions::abandon(
                workspace.initial_branch_id,
                created.commit_id,
                created.plan_entity_id,
                created.plan_entity_version_id,
                "The plan no longer matches the Goal",
            )
            .expect("abandon options"),
        )
        .expect("abandon plan");
    let reopened = engine
        .transition_plan(
            PlanTransitionOptions::reopen(
                workspace.initial_branch_id,
                abandoned.commit_id,
                abandoned.plan_entity_id,
                abandoned.plan_entity_version_id,
                "The plan became relevant again",
            )
            .expect("reopen abandoned options"),
        )
        .expect("reopen abandoned plan");

    assert_eq!(abandoned.state.status, PlanStatus::Abandoned);
    assert_eq!(abandoned.state.completion_rationale, None);
    assert_eq!(reopened.previous_state, abandoned.state);
    assert_eq!(reopened.state.status, PlanStatus::Active);
    assert_eq!(reopened.state.completion_rationale, None);

    let abandoned_snapshot = engine
        .plan_at(abandoned.commit_id, abandoned.plan_entity_id)
        .expect("abandoned snapshot");
    let reopened_snapshot = engine
        .plan_at(reopened.commit_id, reopened.plan_entity_id)
        .expect("reopened snapshot");
    assert_eq!(abandoned_snapshot.state, abandoned.state);
    assert_eq!(reopened_snapshot.state, reopened.state);
}

#[test]
fn plan_transition_rejects_invalid_lifecycle_edges_without_history_rows() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);

    let created = engine
        .create_plan(
            PlanCreateOptions::new(
                workspace.initial_branch_id,
                workspace.genesis_commit_id,
                "Reject invalid Plan transitions",
                "Keep transition graph narrow",
            )
            .expect("plan options"),
        )
        .expect("create plan");

    let empty_completion_rationale = PlanTransitionOptions::complete(
        workspace.initial_branch_id,
        created.commit_id,
        created.plan_entity_id,
        created.plan_entity_version_id,
    )
    .expect("complete options")
    .with_completion_rationale("   ")
    .expect_err("empty completion rationale");
    assert_eq!(empty_completion_rationale.code(), ErrorCode::PlanInvalid);

    let empty_abandon_rationale = PlanTransitionOptions::abandon(
        workspace.initial_branch_id,
        created.commit_id,
        created.plan_entity_id,
        created.plan_entity_version_id,
        "   ",
    )
    .expect_err("empty abandon rationale");
    assert_eq!(empty_abandon_rationale.code(), ErrorCode::PlanInvalid);

    let connection = raw_connection(&path);
    let before_invalid = history_counts(&connection);
    let before_head = branch_head(&connection, workspace.initial_branch_id);
    drop(connection);

    let active_to_active = engine
        .transition_plan(
            PlanTransitionOptions::new(
                workspace.initial_branch_id,
                created.commit_id,
                created.plan_entity_id,
                created.plan_entity_version_id,
                PlanStatus::Active,
            )
            .expect("active options")
            .with_transition_rationale("Nothing changed")
            .expect("active rationale"),
        )
        .expect_err("active to active rejected");
    assert_eq!(active_to_active.code(), ErrorCode::PlanInvalid);
    assert_counts_and_head_unchanged(
        &path,
        &before_invalid,
        workspace.initial_branch_id,
        before_head,
    );

    let active_to_superseded = engine
        .transition_plan(
            PlanTransitionOptions::new(
                workspace.initial_branch_id,
                created.commit_id,
                created.plan_entity_id,
                created.plan_entity_version_id,
                PlanStatus::Superseded,
            )
            .expect("superseded options")
            .with_transition_rationale("Supersession must use a relation")
            .expect("superseded rationale"),
        )
        .expect_err("ordinary superseded transition rejected");
    assert_eq!(active_to_superseded.code(), ErrorCode::PlanInvalid);
    assert_counts_and_head_unchanged(
        &path,
        &before_invalid,
        workspace.initial_branch_id,
        before_head,
    );

    let completed = engine
        .transition_plan(
            PlanTransitionOptions::complete(
                workspace.initial_branch_id,
                created.commit_id,
                created.plan_entity_id,
                created.plan_entity_version_id,
            )
            .expect("complete options"),
        )
        .expect("complete plan");
    let connection = raw_connection(&path);
    let before_completed_invalid = history_counts(&connection);
    let before_completed_head = branch_head(&connection, workspace.initial_branch_id);
    drop(connection);

    let completed_to_abandoned = engine
        .transition_plan(
            PlanTransitionOptions::abandon(
                workspace.initial_branch_id,
                completed.commit_id,
                completed.plan_entity_id,
                completed.plan_entity_version_id,
                "Direct terminal switch",
            )
            .expect("completed to abandoned options"),
        )
        .expect_err("completed to abandoned rejected");
    assert_eq!(completed_to_abandoned.code(), ErrorCode::PlanInvalid);
    assert_counts_and_head_unchanged(
        &path,
        &before_completed_invalid,
        workspace.initial_branch_id,
        before_completed_head,
    );

    let completed_noop = engine
        .transition_plan(
            PlanTransitionOptions::complete(
                workspace.initial_branch_id,
                completed.commit_id,
                completed.plan_entity_id,
                completed.plan_entity_version_id,
            )
            .expect("completed noop options"),
        )
        .expect_err("completed noop rejected");
    assert_eq!(completed_noop.code(), ErrorCode::PlanInvalid);
    assert_counts_and_head_unchanged(
        &path,
        &before_completed_invalid,
        workspace.initial_branch_id,
        before_completed_head,
    );

    let second = engine
        .create_plan(
            PlanCreateOptions::new(
                workspace.initial_branch_id,
                completed.commit_id,
                "Reject abandoned terminal edges",
                "Test abandoned side",
            )
            .expect("second plan options"),
        )
        .expect("second plan");
    let abandoned = engine
        .transition_plan(
            PlanTransitionOptions::abandon(
                workspace.initial_branch_id,
                second.commit_id,
                second.plan_entity_id,
                second.plan_entity_version_id,
                "No longer useful",
            )
            .expect("abandon options"),
        )
        .expect("abandon plan");
    let connection = raw_connection(&path);
    let before_abandoned_invalid = history_counts(&connection);
    let before_abandoned_head = branch_head(&connection, workspace.initial_branch_id);
    drop(connection);

    let abandoned_to_completed = engine
        .transition_plan(
            PlanTransitionOptions::complete(
                workspace.initial_branch_id,
                abandoned.commit_id,
                abandoned.plan_entity_id,
                abandoned.plan_entity_version_id,
            )
            .expect("abandoned to completed options")
            .with_completion_rationale("Direct terminal switch")
            .expect("completion rationale"),
        )
        .expect_err("abandoned to completed rejected");
    assert_eq!(abandoned_to_completed.code(), ErrorCode::PlanInvalid);
    assert_counts_and_head_unchanged(
        &path,
        &before_abandoned_invalid,
        workspace.initial_branch_id,
        before_abandoned_head,
    );

    let abandoned_noop = engine
        .transition_plan(
            PlanTransitionOptions::new(
                workspace.initial_branch_id,
                abandoned.commit_id,
                abandoned.plan_entity_id,
                abandoned.plan_entity_version_id,
                PlanStatus::Abandoned,
            )
            .expect("abandoned noop options")
            .with_transition_rationale("Still abandoned")
            .expect("abandoned noop rationale"),
        )
        .expect_err("abandoned noop rejected");
    assert_eq!(abandoned_noop.code(), ErrorCode::PlanInvalid);
    assert_counts_and_head_unchanged(
        &path,
        &before_abandoned_invalid,
        workspace.initial_branch_id,
        before_abandoned_head,
    );
}

#[test]
fn plan_transition_rejects_stale_version_stale_head_and_non_plan_without_partial_rows() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);

    let created = engine
        .create_plan(
            PlanCreateOptions::new(
                workspace.initial_branch_id,
                workspace.genesis_commit_id,
                "Protect Plan transition CAS",
                "Use expected versions and branch heads",
            )
            .expect("plan options"),
        )
        .expect("create plan");
    let completed = engine
        .transition_plan(
            PlanTransitionOptions::complete(
                workspace.initial_branch_id,
                created.commit_id,
                created.plan_entity_id,
                created.plan_entity_version_id,
            )
            .expect("complete options"),
        )
        .expect("complete plan");

    let connection = raw_connection(&path);
    let before_invalid = history_counts(&connection);
    let before_head = branch_head(&connection, workspace.initial_branch_id);
    drop(connection);

    let stale_version = engine
        .transition_plan(
            PlanTransitionOptions::reopen(
                workspace.initial_branch_id,
                completed.commit_id,
                completed.plan_entity_id,
                created.plan_entity_version_id,
                "Retry with stale entity version",
            )
            .expect("reopen options"),
        )
        .expect_err("stale plan version rejected");
    assert_eq!(stale_version.code(), ErrorCode::PlanInvalid);
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
                completed.commit_id,
                "Non-plan entity",
            )
            .expect("task options"),
        )
        .expect("create task");
    let connection = raw_connection(&path);
    let before_non_plan = history_counts(&connection);
    let before_non_plan_head = branch_head(&connection, workspace.initial_branch_id);
    drop(connection);

    let non_plan = engine
        .transition_plan(
            PlanTransitionOptions::complete(
                workspace.initial_branch_id,
                task.commit_id,
                task.task_entity_id,
                task.task_entity_version_id,
            )
            .expect("complete options"),
        )
        .expect_err("non-plan entity rejected");
    assert_eq!(non_plan.code(), ErrorCode::PlanNotFound);
    assert_counts_and_head_unchanged(
        &path,
        &before_non_plan,
        workspace.initial_branch_id,
        before_non_plan_head,
    );

    let mut second_engine = Engine::open(&path).expect("second engine");
    let branch_mover = second_engine
        .create_plan(
            PlanCreateOptions::new(
                workspace.initial_branch_id,
                task.commit_id,
                "Move the branch head",
                "Create an unrelated Plan",
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
        .transition_plan(
            PlanTransitionOptions::reopen(
                workspace.initial_branch_id,
                completed.commit_id,
                completed.plan_entity_id,
                completed.plan_entity_version_id,
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
fn plan_state_validates_completion_rationale_consistency_and_superseded_boundary() {
    let active_with_completion_rationale = PlanState {
        description: "Active plans cannot carry completion rationale".to_owned(),
        status: PlanStatus::Active,
        constraints: Vec::new(),
        strategy: "Reject stale terminal fields".to_owned(),
        completion_rationale: Some("stale rationale".to_owned()),
    }
    .to_canonical_value()
    .expect_err("active with completion rationale rejected");
    assert_eq!(
        active_with_completion_rationale.code(),
        ErrorCode::PlanInvalid
    );

    let abandoned_with_completion_rationale = PlanState {
        description: "Abandoned plans cannot carry completion rationale".to_owned(),
        status: PlanStatus::Abandoned,
        constraints: Vec::new(),
        strategy: "Reject wrong terminal field".to_owned(),
        completion_rationale: Some("wrong terminal rationale".to_owned()),
    }
    .to_canonical_value()
    .expect_err("abandoned with completion rationale rejected");
    assert_eq!(
        abandoned_with_completion_rationale.code(),
        ErrorCode::PlanInvalid
    );

    let empty_completion_rationale = PlanState {
        description: "Completed plan rationale must not be blank".to_owned(),
        status: PlanStatus::Completed,
        constraints: Vec::new(),
        strategy: "Reject blank rationale".to_owned(),
        completion_rationale: Some("   ".to_owned()),
    }
    .to_canonical_value()
    .expect_err("empty completion rationale rejected");
    assert_eq!(empty_completion_rationale.code(), ErrorCode::PlanInvalid);

    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let created = engine
        .create_plan(
            PlanCreateOptions::new(
                workspace.initial_branch_id,
                workspace.genesis_commit_id,
                "Reject corrupted completion rationale",
                "Readback should validate state consistency",
            )
            .expect("plan options"),
        )
        .expect("create plan");
    let connection = raw_connection(&path);
    corrupt_plan_state(
        &connection,
        created.plan_entity_version_id,
        PlanStatus::Active,
        CanonicalValue::String("stale reason".to_owned()),
    );
    drop(connection);

    let corrupted = engine
        .plan_at(created.commit_id, created.plan_entity_id)
        .expect_err("corrupted plan state rejected");
    assert_eq!(corrupted.code(), ErrorCode::PlanInvalid);

    let (_superseded_tempdir, superseded_path) = store_path();
    let (mut superseded_engine, superseded_workspace) = create_workspace(&superseded_path);
    let superseded_source = superseded_engine
        .create_plan(
            PlanCreateOptions::new(
                superseded_workspace.initial_branch_id,
                superseded_workspace.genesis_commit_id,
                "Superseded source",
                "Simulate imported superseded state",
            )
            .expect("superseded source options"),
        )
        .expect("create superseded source");
    let connection = raw_connection(&superseded_path);
    corrupt_plan_state(
        &connection,
        superseded_source.plan_entity_version_id,
        PlanStatus::Superseded,
        CanonicalValue::Null,
    );
    let before_superseded = history_counts(&connection);
    let before_superseded_head = branch_head(&connection, superseded_workspace.initial_branch_id);
    drop(connection);

    let superseded_reopen = superseded_engine
        .transition_plan(
            PlanTransitionOptions::reopen(
                superseded_workspace.initial_branch_id,
                superseded_source.commit_id,
                superseded_source.plan_entity_id,
                superseded_source.plan_entity_version_id,
                "Ordinary reopen should be rejected",
            )
            .expect("superseded reopen options"),
        )
        .expect_err("ordinary superseded reopen rejected");
    assert_eq!(superseded_reopen.code(), ErrorCode::PlanInvalid);
    assert_counts_and_head_unchanged(
        &superseded_path,
        &before_superseded,
        superseded_workspace.initial_branch_id,
        before_superseded_head,
    );
}
