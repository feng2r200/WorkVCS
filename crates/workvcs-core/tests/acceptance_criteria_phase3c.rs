use rusqlite::{Connection, params};
use std::path::{Path, PathBuf};
use tempfile::TempDir;
use workvcs_core::{
    AcceptanceCriterionClassification, AcceptanceCriterionCreateOptions,
    AcceptanceCriterionRevisionOptions, AcceptanceCriterionState, BranchId, CanonicalValue,
    CommitId, Digest, Engine, EntityId, EntityTransitionOptions, ErrorCategory, ErrorCode,
    StoreInitOptions, TaskAcceptanceCriterionRef, TaskCreateOptions, TaskState, TaskStatus,
    TaskTransitionOptions, WorkspaceInfo, WorkspaceInitOptions, canonical_bytes,
    entity_version_digest,
};

#[derive(Clone, Debug, PartialEq, Eq)]
struct HistoryCounts {
    object_identity: i64,
    entity: i64,
    acceptance_criterion_identity: i64,
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
        StoreInitOptions::new("phase3c-acceptance-store").expect("store options"),
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

fn history_counts(connection: &Connection) -> HistoryCounts {
    HistoryCounts {
        object_identity: count_rows(connection, "object_identity"),
        entity: count_rows(connection, "entity"),
        acceptance_criterion_identity: count_rows(connection, "acceptance_criterion_identity"),
        entity_version: count_rows(connection, "entity_version"),
        changeset: count_rows(connection, "changeset"),
        change_operation: count_rows(connection, "change_operation"),
        entity_membership_change: count_rows(connection, "entity_membership_change"),
        workstate_commit: count_rows(connection, "workstate_commit"),
        commit_parent: count_rows(connection, "commit_parent"),
        event: count_rows(connection, "event"),
    }
}

fn count_rows(connection: &Connection, table: &str) -> i64 {
    connection
        .query_row(&format!("SELECT count(*) FROM {table}"), [], |row| {
            row.get(0)
        })
        .expect("count rows")
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

fn rationale(reason: &str) -> CanonicalValue {
    CanonicalValue::object(vec![(
        "reason".to_owned(),
        CanonicalValue::String(reason.to_owned()),
    )])
    .expect("rationale")
}

fn task_state_json(
    description: &str,
    status: TaskStatus,
    outcome: Option<&str>,
    priority: i64,
    acceptance_criteria: Vec<TaskAcceptanceCriterionRef>,
) -> String {
    let state = TaskState {
        description: description.to_owned(),
        status,
        outcome: outcome.map(str::to_owned),
        priority,
        acceptance_criteria,
    };
    String::from_utf8(
        canonical_bytes(&state.to_canonical_value().expect("canonical task state"))
            .expect("canonical bytes"),
    )
    .expect("task json")
}

fn acceptance_criterion_state_json(
    statement: &str,
    classification: AcceptanceCriterionClassification,
) -> String {
    let state = AcceptanceCriterionState::new(statement, classification).expect("ac state");
    String::from_utf8(
        canonical_bytes(&state.to_canonical_value().expect("canonical ac state"))
            .expect("canonical bytes"),
    )
    .expect("ac json")
}

#[test]
fn create_acceptance_criterion_persists_identity_and_updates_task_atomically() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);

    let task = engine
        .create_task(
            TaskCreateOptions::new(
                workspace.initial_branch_id,
                workspace.genesis_commit_id,
                "Verify store reopen",
            )
            .expect("task options")
            .with_priority(4)
            .expect("priority"),
        )
        .expect("create task");
    let criterion = engine
        .create_acceptance_criterion(
            AcceptanceCriterionCreateOptions::new(
                workspace.initial_branch_id,
                task.commit_id,
                task.task_entity_id,
                task.task_entity_version_id,
                "AC-1",
                "The store opens after process restart.",
                AcceptanceCriterionClassification::Required,
            )
            .expect("ac options")
            .with_rationale(rationale("define acceptance")),
        )
        .expect("create acceptance criterion");

    assert_eq!(criterion.workspace_id, workspace.workspace_id);
    assert_eq!(criterion.branch_id, workspace.initial_branch_id);
    assert_eq!(criterion.previous_head_commit_id, task.commit_id);
    assert_eq!(criterion.task_entity_id, task.task_entity_id);
    assert_eq!(
        criterion.previous_task_entity_version_id,
        task.task_entity_version_id
    );
    assert_eq!(criterion.local_key, "AC-1");
    assert_eq!(
        criterion.state.classification,
        AcceptanceCriterionClassification::Required
    );
    assert_ne!(
        criterion.acceptance_criterion_operation_id,
        criterion.task_operation_id
    );
    assert_eq!(criterion.task_state.acceptance_criteria.len(), 1);
    assert_eq!(
        criterion.task_state.acceptance_criteria[0].acceptance_criterion_entity_id,
        criterion.acceptance_criterion_entity_id
    );

    let historical_task = engine
        .task_at(task.commit_id, task.task_entity_id)
        .expect("historical task");
    assert!(historical_task.state.acceptance_criteria.is_empty());
    let absent_at_task_commit = engine
        .acceptance_criterion_at(task.commit_id, criterion.acceptance_criterion_entity_id)
        .expect_err("criterion absent before create commit");
    assert_eq!(absent_at_task_commit.code(), ErrorCode::TaskNotFound);

    let current_task = engine
        .task_at(criterion.commit_id, task.task_entity_id)
        .expect("current task");
    assert_eq!(
        current_task.task_entity_version_id,
        criterion.task_entity_version_id
    );
    assert_eq!(current_task.state, criterion.task_state);
    let snapshot = engine
        .acceptance_criterion_at(
            criterion.commit_id,
            criterion.acceptance_criterion_entity_id,
        )
        .expect("criterion snapshot");
    assert_eq!(snapshot.workspace_id, workspace.workspace_id);
    assert_eq!(snapshot.task_entity_id, task.task_entity_id);
    assert_eq!(snapshot.local_key, "AC-1");
    assert_eq!(
        snapshot.acceptance_criterion_entity_version_id,
        criterion.acceptance_criterion_entity_version_id
    );
    assert_eq!(
        snapshot.state_digest,
        criterion.acceptance_criterion_state_digest
    );
    assert_eq!(snapshot.state, criterion.state);

    let connection = raw_connection(&path);
    assert_eq!(
        history_counts(&connection),
        HistoryCounts {
            object_identity: 2,
            entity: 2,
            acceptance_criterion_identity: 1,
            entity_version: 3,
            changeset: 3,
            change_operation: 3,
            entity_membership_change: 3,
            workstate_commit: 3,
            commit_parent: 2,
            event: 3,
        }
    );
    assert_eq!(
        branch_head(&connection, workspace.initial_branch_id),
        criterion.commit_id
    );

    let criterion_entity_id = criterion.acceptance_criterion_entity_id.raw_bytes();
    let criterion_version_id = criterion.acceptance_criterion_entity_version_id.raw_bytes();
    let task_version_id = criterion.task_entity_version_id.raw_bytes();
    let (
        owner_entity_id,
        local_key,
        criterion_kind,
        criterion_state_json,
        criterion_state_digest,
        stored_task_state_json,
        task_state_digest,
    ) = connection
        .query_row(
            "SELECT acceptance_criterion_identity.owner_entity_id,
                    acceptance_criterion_identity.local_key,
                    entity.entity_kind,
                    criterion_version.state_json,
                    criterion_version.state_digest,
                    task_version.state_json,
                    task_version.state_digest
             FROM acceptance_criterion_identity
             JOIN entity
               ON entity.object_id = acceptance_criterion_identity.entity_id
             JOIN entity_version AS criterion_version
               ON criterion_version.entity_id = acceptance_criterion_identity.entity_id
             JOIN entity_version AS task_version
               ON task_version.entity_id = acceptance_criterion_identity.owner_entity_id
             WHERE acceptance_criterion_identity.entity_id = ?1
               AND criterion_version.entity_version_id = ?2
               AND task_version.entity_version_id = ?3",
            params![
                &criterion_entity_id[..],
                &criterion_version_id[..],
                &task_version_id[..]
            ],
            |row| {
                Ok((
                    row.get::<_, Vec<u8>>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, Vec<u8>>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, Vec<u8>>(6)?,
                ))
            },
        )
        .expect("stored criterion and task");
    assert_eq!(owner_entity_id, task.task_entity_id.raw_bytes());
    assert_eq!(local_key, "AC-1");
    assert_eq!(criterion_kind, "acceptance_criterion");
    assert_eq!(
        criterion_state_json,
        acceptance_criterion_state_json(
            "The store opens after process restart.",
            AcceptanceCriterionClassification::Required
        )
    );
    assert_eq!(
        digest_from_blob(criterion_state_digest),
        criterion.acceptance_criterion_state_digest
    );
    assert_eq!(
        stored_task_state_json,
        task_state_json(
            "Verify store reopen",
            TaskStatus::Pending,
            None,
            4,
            vec![
                TaskAcceptanceCriterionRef::new("AC-1", criterion.acceptance_criterion_entity_id,)
                    .expect("criterion ref")
            ]
        )
    );
    assert_eq!(
        digest_from_blob(task_state_digest),
        criterion.task_state_digest
    );

    let operations = connection
        .prepare(
            "SELECT ordinal, subject_object_id
             FROM change_operation
             WHERE changeset_id = ?1
             ORDER BY ordinal",
        )
        .expect("prepare operations")
        .query_map(params![&criterion.changeset_id.raw_bytes()[..]], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, Vec<u8>>(1)?))
        })
        .expect("query operations")
        .collect::<std::result::Result<Vec<_>, _>>()
        .expect("operations");
    assert_eq!(operations.len(), 2);
    assert_eq!(operations[0], (0, criterion_entity_id.to_vec()));
    assert_eq!(operations[1], (1, task.task_entity_id.raw_bytes().to_vec()));
    drop(connection);

    let report = engine.validate_integrity().expect("integrity");
    assert_eq!(report.checked_branches, 1);
    assert_eq!(report.checked_commits, 3);
}

#[test]
fn acceptance_criterion_create_rejects_duplicate_local_key_without_partial_rows() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let task = engine
        .create_task(
            TaskCreateOptions::new(
                workspace.initial_branch_id,
                workspace.genesis_commit_id,
                "Task",
            )
            .expect("task options"),
        )
        .expect("create task");
    let first = engine
        .create_acceptance_criterion(
            AcceptanceCriterionCreateOptions::new(
                workspace.initial_branch_id,
                task.commit_id,
                task.task_entity_id,
                task.task_entity_version_id,
                "AC-1",
                "First criterion.",
                AcceptanceCriterionClassification::Optional,
            )
            .expect("first options"),
        )
        .expect("first criterion");

    let connection = raw_connection(&path);
    let before = history_counts(&connection);
    drop(connection);

    let error = engine
        .create_acceptance_criterion(
            AcceptanceCriterionCreateOptions::new(
                workspace.initial_branch_id,
                first.commit_id,
                task.task_entity_id,
                first.task_entity_version_id,
                "AC-1",
                "Duplicate criterion.",
                AcceptanceCriterionClassification::Optional,
            )
            .expect("duplicate options"),
        )
        .expect_err("duplicate local key");
    assert_eq!(error.code(), ErrorCode::TaskInvalid);
    assert_eq!(error.category(), ErrorCategory::Task);

    let connection = raw_connection(&path);
    assert_eq!(history_counts(&connection), before);
    assert_eq!(
        branch_head(&connection, workspace.initial_branch_id),
        first.commit_id
    );
}

#[test]
fn acceptance_criterion_create_reuses_branch_cas_and_rolls_back_stale_loser() {
    let (_tempdir, path) = store_path();
    let (mut first_engine, workspace) = create_workspace(&path);
    let mut second_engine = Engine::open(&path).expect("second engine");
    let task = first_engine
        .create_task(
            TaskCreateOptions::new(
                workspace.initial_branch_id,
                workspace.genesis_commit_id,
                "Race task",
            )
            .expect("task options"),
        )
        .expect("create task");
    let first_seen_head = first_engine
        .branch_head(workspace.initial_branch_id)
        .expect("first branch head")
        .head_commit_id;
    let second_seen_head = second_engine
        .branch_head(workspace.initial_branch_id)
        .expect("second branch head")
        .head_commit_id;
    assert_eq!(first_seen_head, task.commit_id);
    assert_eq!(second_seen_head, task.commit_id);

    let winner = first_engine
        .create_acceptance_criterion(
            AcceptanceCriterionCreateOptions::new(
                workspace.initial_branch_id,
                first_seen_head,
                task.task_entity_id,
                task.task_entity_version_id,
                "AC-1",
                "Winning criterion.",
                AcceptanceCriterionClassification::Optional,
            )
            .expect("winner options"),
        )
        .expect("winner criterion");

    let connection = raw_connection(&path);
    let before_loser = history_counts(&connection);
    drop(connection);

    let error = second_engine
        .create_acceptance_criterion(
            AcceptanceCriterionCreateOptions::new(
                workspace.initial_branch_id,
                second_seen_head,
                task.task_entity_id,
                task.task_entity_version_id,
                "AC-2",
                "Stale criterion.",
                AcceptanceCriterionClassification::Optional,
            )
            .expect("loser options"),
        )
        .expect_err("stale loser");
    assert_eq!(error.code(), ErrorCode::BranchHeadConflict);
    assert_eq!(error.category(), ErrorCategory::Mutation);
    assert!(error.retryable());

    let connection = raw_connection(&path);
    assert_eq!(history_counts(&connection), before_loser);
    assert_eq!(
        branch_head(&connection, workspace.initial_branch_id),
        winner.commit_id
    );
}

#[test]
fn generic_entity_transition_cannot_bypass_task_or_acceptance_criterion_semantics() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let task = engine
        .create_task(
            TaskCreateOptions::new(
                workspace.initial_branch_id,
                workspace.genesis_commit_id,
                "Reserved semantic entity",
            )
            .expect("task options"),
        )
        .expect("create task");
    let criterion = engine
        .create_acceptance_criterion(
            AcceptanceCriterionCreateOptions::new(
                workspace.initial_branch_id,
                task.commit_id,
                task.task_entity_id,
                task.task_entity_version_id,
                "AC-1",
                "Mandatory criterion.",
                AcceptanceCriterionClassification::Required,
            )
            .expect("criterion options"),
        )
        .expect("create criterion");

    let connection = raw_connection(&path);
    let before = history_counts(&connection);
    drop(connection);

    let illegal_done_state = TaskState {
        description: task.state.description,
        status: TaskStatus::Done,
        outcome: Some("forced".to_owned()),
        priority: task.state.priority,
        acceptance_criteria: criterion.task_state.acceptance_criteria.clone(),
    };
    let task_update_error = engine
        .commit_entity_transition(
            EntityTransitionOptions::update(
                workspace.initial_branch_id,
                criterion.commit_id,
                task.task_entity_id,
                criterion.task_entity_version_id,
                illegal_done_state
                    .to_canonical_value()
                    .expect("illegal task state"),
            )
            .expect("generic task update options"),
        )
        .expect_err("generic task update is reserved");
    assert_eq!(task_update_error.code(), ErrorCode::EntityTransitionInvalid);
    assert_eq!(task_update_error.category(), ErrorCategory::Mutation);

    let criterion_update_error = engine
        .commit_entity_transition(
            EntityTransitionOptions::update(
                workspace.initial_branch_id,
                criterion.commit_id,
                criterion.acceptance_criterion_entity_id,
                criterion.acceptance_criterion_entity_version_id,
                AcceptanceCriterionState::new(
                    "Bypass revision.",
                    AcceptanceCriterionClassification::Optional,
                )
                .expect("bypass state")
                .to_canonical_value()
                .expect("bypass value"),
            )
            .expect("generic criterion update options"),
        )
        .expect_err("generic criterion update is reserved");
    assert_eq!(
        criterion_update_error.code(),
        ErrorCode::EntityTransitionInvalid
    );
    assert_eq!(criterion_update_error.category(), ErrorCategory::Mutation);

    let criterion_create_error = engine
        .commit_entity_transition(
            EntityTransitionOptions::create(
                workspace.initial_branch_id,
                criterion.commit_id,
                "acceptance_criterion",
                AcceptanceCriterionState::new(
                    "Orphan criterion.",
                    AcceptanceCriterionClassification::Optional,
                )
                .expect("orphan state")
                .to_canonical_value()
                .expect("orphan value"),
            )
            .expect("generic criterion create options"),
        )
        .expect_err("generic criterion create is reserved");
    assert_eq!(
        criterion_create_error.code(),
        ErrorCode::EntityTransitionInvalid
    );
    assert_eq!(criterion_create_error.category(), ErrorCategory::Mutation);

    let task_create_error = engine
        .commit_entity_transition(
            EntityTransitionOptions::create(
                workspace.initial_branch_id,
                criterion.commit_id,
                "task",
                TaskState::pending("Generic task", 0)
                    .expect("generic task state")
                    .to_canonical_value()
                    .expect("generic task value"),
            )
            .expect("generic task create options"),
        )
        .expect_err("generic task create is reserved");
    assert_eq!(task_create_error.code(), ErrorCode::EntityTransitionInvalid);
    assert_eq!(task_create_error.category(), ErrorCategory::Mutation);

    let connection = raw_connection(&path);
    assert_eq!(history_counts(&connection), before);
    assert_eq!(
        branch_head(&connection, workspace.initial_branch_id),
        criterion.commit_id
    );
}

#[test]
fn acceptance_criterion_revision_preserves_identity_local_key_and_history() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let task = engine
        .create_task(
            TaskCreateOptions::new(
                workspace.initial_branch_id,
                workspace.genesis_commit_id,
                "Revise criterion",
            )
            .expect("task options"),
        )
        .expect("create task");
    let created = engine
        .create_acceptance_criterion(
            AcceptanceCriterionCreateOptions::new(
                workspace.initial_branch_id,
                task.commit_id,
                task.task_entity_id,
                task.task_entity_version_id,
                "AC-1",
                "Initial wording.",
                AcceptanceCriterionClassification::Optional,
            )
            .expect("create options"),
        )
        .expect("create criterion");
    let revised = engine
        .revise_acceptance_criterion(
            AcceptanceCriterionRevisionOptions::new(
                workspace.initial_branch_id,
                created.commit_id,
                created.acceptance_criterion_entity_id,
                created.acceptance_criterion_entity_version_id,
                "Revised wording.",
                AcceptanceCriterionClassification::Required,
            )
            .expect("revision options")
            .with_rationale(rationale("tighten acceptance")),
        )
        .expect("revise criterion");

    assert_eq!(
        revised.acceptance_criterion_entity_id,
        created.acceptance_criterion_entity_id
    );
    assert_eq!(revised.task_entity_id, task.task_entity_id);
    assert_eq!(revised.local_key, "AC-1");
    assert_eq!(
        revised.previous_acceptance_criterion_entity_version_id,
        created.acceptance_criterion_entity_version_id
    );
    assert_ne!(
        revised.acceptance_criterion_entity_version_id,
        created.acceptance_criterion_entity_version_id
    );
    assert_eq!(revised.previous_state, created.state);
    assert_eq!(
        revised.state.classification,
        AcceptanceCriterionClassification::Required
    );

    let old_snapshot = engine
        .acceptance_criterion_at(created.commit_id, created.acceptance_criterion_entity_id)
        .expect("old snapshot");
    let new_snapshot = engine
        .acceptance_criterion_at(revised.commit_id, revised.acceptance_criterion_entity_id)
        .expect("new snapshot");
    assert_eq!(old_snapshot.state.statement, "Initial wording.");
    assert_eq!(
        old_snapshot.state.classification,
        AcceptanceCriterionClassification::Optional
    );
    assert_eq!(new_snapshot.state.statement, "Revised wording.");
    assert_eq!(
        new_snapshot.state.classification,
        AcceptanceCriterionClassification::Required
    );

    let task_after_revision = engine
        .task_at(revised.commit_id, task.task_entity_id)
        .expect("task after ac revision");
    assert_eq!(
        task_after_revision.task_entity_version_id,
        created.task_entity_version_id
    );
    assert_eq!(task_after_revision.state.acceptance_criteria.len(), 1);
    assert_eq!(
        task_after_revision.state.acceptance_criteria[0].local_key,
        "AC-1"
    );

    let connection = raw_connection(&path);
    let criterion_version_count = connection
        .query_row(
            "SELECT count(*)
             FROM entity_version
             WHERE entity_id = ?1",
            params![&created.acceptance_criterion_entity_id.raw_bytes()[..]],
            |row| row.get::<_, i64>(0),
        )
        .expect("criterion version count");
    assert_eq!(criterion_version_count, 2);
    assert_eq!(count_rows(&connection, "acceptance_criterion_identity"), 1);
    drop(connection);

    let report = engine.validate_integrity().expect("integrity");
    assert_eq!(report.checked_commits, 4);
}

#[test]
fn mandatory_acceptance_criterion_blocks_done_without_verification_projection() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let task = engine
        .create_task(
            TaskCreateOptions::new(
                workspace.initial_branch_id,
                workspace.genesis_commit_id,
                "Mandatory gate",
            )
            .expect("task options"),
        )
        .expect("create task");
    let criterion = engine
        .create_acceptance_criterion(
            AcceptanceCriterionCreateOptions::new(
                workspace.initial_branch_id,
                task.commit_id,
                task.task_entity_id,
                task.task_entity_version_id,
                "AC-1",
                "Mandatory criterion.",
                AcceptanceCriterionClassification::Required,
            )
            .expect("criterion options"),
        )
        .expect("create criterion");

    let connection = raw_connection(&path);
    let before_done = history_counts(&connection);
    drop(connection);

    let error = engine
        .transition_task(
            TaskTransitionOptions::new(
                workspace.initial_branch_id,
                criterion.commit_id,
                task.task_entity_id,
                criterion.task_entity_version_id,
                TaskStatus::Done,
            )
            .expect("done options")
            .with_outcome("complete")
            .expect("outcome"),
        )
        .expect_err("mandatory criterion blocks done");
    assert_eq!(error.code(), ErrorCode::TaskInvalid);
    assert_eq!(error.category(), ErrorCategory::Task);
    assert!(error.to_string().contains("unverified"));

    let connection = raw_connection(&path);
    assert_eq!(history_counts(&connection), before_done);
    assert_eq!(
        branch_head(&connection, workspace.initial_branch_id),
        criterion.commit_id
    );
}

#[test]
fn optional_acceptance_criterion_allows_done_transition() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let task = engine
        .create_task(
            TaskCreateOptions::new(
                workspace.initial_branch_id,
                workspace.genesis_commit_id,
                "Optional gate",
            )
            .expect("task options"),
        )
        .expect("create task");
    let criterion = engine
        .create_acceptance_criterion(
            AcceptanceCriterionCreateOptions::new(
                workspace.initial_branch_id,
                task.commit_id,
                task.task_entity_id,
                task.task_entity_version_id,
                "AC-1",
                "Optional criterion.",
                AcceptanceCriterionClassification::Optional,
            )
            .expect("criterion options"),
        )
        .expect("create criterion");

    let done = engine
        .transition_task(
            TaskTransitionOptions::new(
                workspace.initial_branch_id,
                criterion.commit_id,
                task.task_entity_id,
                criterion.task_entity_version_id,
                TaskStatus::Done,
            )
            .expect("done options")
            .with_outcome("accepted")
            .expect("outcome"),
        )
        .expect("optional criterion allows done");
    assert_eq!(done.state.status, TaskStatus::Done);
    assert_eq!(done.state.outcome.as_deref(), Some("accepted"));
    assert_eq!(done.state.acceptance_criteria.len(), 1);
}

#[test]
fn task_acceptance_criteria_are_serialized_in_local_key_order() {
    let first_id = EntityId::new_v7();
    let second_id = EntityId::new_v7();
    let state = TaskState {
        description: "Sorted criteria".to_owned(),
        status: TaskStatus::Pending,
        outcome: None,
        priority: 0,
        acceptance_criteria: vec![
            TaskAcceptanceCriterionRef::new("AC-2", second_id).expect("second ref"),
            TaskAcceptanceCriterionRef::new("AC-1", first_id).expect("first ref"),
        ],
    };

    let encoded = String::from_utf8(
        canonical_bytes(&state.to_canonical_value().expect("task state")).unwrap(),
    )
    .expect("task json");
    assert_eq!(
        encoded,
        format!(
            r#"{{"acceptance_criteria":[{{"entity_id":"{}","local_key":"AC-1"}},{{"entity_id":"{}","local_key":"AC-2"}}],"child_order":[],"description":"Sorted criteria","outcome":null,"priority":0,"status":"pending"}}"#,
            first_id, second_id
        )
    );

    let reversed = TaskState {
        description: state.description.clone(),
        status: state.status,
        outcome: state.outcome.clone(),
        priority: state.priority,
        acceptance_criteria: vec![
            TaskAcceptanceCriterionRef::new("AC-1", first_id).expect("first ref"),
            TaskAcceptanceCriterionRef::new("AC-2", second_id).expect("second ref"),
        ],
    };
    assert_eq!(
        canonical_bytes(&state.to_canonical_value().expect("state")).expect("state bytes"),
        canonical_bytes(&reversed.to_canonical_value().expect("reversed")).expect("reversed bytes")
    );
}

#[test]
fn acceptance_criterion_state_is_canonical_and_rejects_invalid_input() {
    let encoded = acceptance_criterion_state_json(
        "Store integrity passes.",
        AcceptanceCriterionClassification::Required,
    );
    assert_eq!(
        encoded,
        r#"{"classification":"required","statement":"Store integrity passes.","verification_requirements":[]}"#
    );
    let value = AcceptanceCriterionState::new(
        "Store integrity passes.",
        AcceptanceCriterionClassification::Required,
    )
    .expect("state")
    .to_canonical_value()
    .expect("canonical");
    assert_eq!(
        entity_version_digest(&value).expect("digest"),
        entity_version_digest(&value).expect("same digest")
    );

    assert!(
        AcceptanceCriterionCreateOptions::new(
            BranchId::new_v7(),
            CommitId::new_v7(),
            EntityId::new_v7(),
            workvcs_core::EntityVersionId::new_v7(),
            " AC-1 ",
            "Statement.",
            AcceptanceCriterionClassification::Required,
        )
        .is_err()
    );
    assert!(
        AcceptanceCriterionState::new("   ", AcceptanceCriterionClassification::Optional).is_err()
    );
    assert!(
        AcceptanceCriterionState::new(
            "bad\0statement",
            AcceptanceCriterionClassification::Optional
        )
        .is_err()
    );
}
