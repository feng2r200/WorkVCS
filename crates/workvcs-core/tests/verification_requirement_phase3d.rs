use rusqlite::{Connection, params};
use std::path::{Path, PathBuf};
use tempfile::TempDir;
use workvcs_core::{
    AcceptanceCriterionClassification, AcceptanceCriterionCreateOptions, AcceptanceCriterionState,
    AcceptanceCriterionVerificationRequirementRef, BranchId, CanonicalValue, CommitId, Digest,
    Engine, EntityId, EntityTransitionOptions, ErrorCategory, ErrorCode, StoreInitOptions,
    TaskCreateOptions, VerificationRequirementCreateOptions,
    VerificationRequirementRevisionOptions, VerificationRequirementState, WorkspaceInfo,
    WorkspaceInitOptions, canonical_bytes, entity_version_digest,
};

#[derive(Clone, Debug, PartialEq, Eq)]
struct HistoryCounts {
    object_identity: i64,
    entity: i64,
    acceptance_criterion_identity: i64,
    verification_requirement_identity: i64,
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
        StoreInitOptions::new("phase3d-requirement-store").expect("store options"),
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
        verification_requirement_identity: count_rows(
            connection,
            "verification_requirement_identity",
        ),
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

fn acceptance_criterion_state_json(
    statement: &str,
    classification: AcceptanceCriterionClassification,
    verification_requirements: Vec<AcceptanceCriterionVerificationRequirementRef>,
) -> String {
    let state = AcceptanceCriterionState {
        statement: statement.to_owned(),
        classification,
        verification_requirements,
    };
    String::from_utf8(
        canonical_bytes(&state.to_canonical_value().expect("canonical ac state"))
            .expect("canonical bytes"),
    )
    .expect("ac json")
}

fn verification_requirement_state_json(statement: &str) -> String {
    let state = VerificationRequirementState::new(statement).expect("vr state");
    String::from_utf8(
        canonical_bytes(&state.to_canonical_value().expect("canonical vr state"))
            .expect("canonical bytes"),
    )
    .expect("vr json")
}

#[test]
fn create_verification_requirement_persists_identity_and_updates_ac_atomically() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let task = engine
        .create_task(
            TaskCreateOptions::new(
                workspace.initial_branch_id,
                workspace.genesis_commit_id,
                "Verify with requirements",
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
                "The store opens after process restart.",
                AcceptanceCriterionClassification::Required,
            )
            .expect("ac options"),
        )
        .expect("create acceptance criterion");

    let requirement = engine
        .create_verification_requirement(
            VerificationRequirementCreateOptions::new(
                workspace.initial_branch_id,
                criterion.commit_id,
                criterion.acceptance_criterion_entity_id,
                criterion.acceptance_criterion_entity_version_id,
                "VR-1",
                "The reopened store returns the same branch head.",
            )
            .expect("vr options")
            .with_rationale(rationale("define required coverage")),
        )
        .expect("create verification requirement");

    assert_eq!(requirement.workspace_id, workspace.workspace_id);
    assert_eq!(requirement.branch_id, workspace.initial_branch_id);
    assert_eq!(requirement.previous_head_commit_id, criterion.commit_id);
    assert_eq!(
        requirement.acceptance_criterion_entity_id,
        criterion.acceptance_criterion_entity_id
    );
    assert_eq!(
        requirement.previous_acceptance_criterion_entity_version_id,
        criterion.acceptance_criterion_entity_version_id
    );
    assert_eq!(requirement.local_key, "VR-1");
    assert_ne!(
        requirement.verification_requirement_operation_id,
        requirement.acceptance_criterion_operation_id
    );
    assert_eq!(
        requirement
            .acceptance_criterion_state
            .verification_requirements
            .len(),
        1
    );
    assert_eq!(
        requirement
            .acceptance_criterion_state
            .verification_requirements[0]
            .verification_requirement_entity_id,
        requirement.verification_requirement_entity_id
    );

    let old_criterion = engine
        .acceptance_criterion_at(
            criterion.commit_id,
            criterion.acceptance_criterion_entity_id,
        )
        .expect("historical criterion");
    assert!(old_criterion.state.verification_requirements.is_empty());
    let absent_at_criterion_commit = engine
        .verification_requirement_at(
            criterion.commit_id,
            requirement.verification_requirement_entity_id,
        )
        .expect_err("requirement absent before create commit");
    assert_eq!(absent_at_criterion_commit.code(), ErrorCode::TaskNotFound);

    let current_criterion = engine
        .acceptance_criterion_at(
            requirement.commit_id,
            criterion.acceptance_criterion_entity_id,
        )
        .expect("current criterion");
    assert_eq!(
        current_criterion.acceptance_criterion_entity_version_id,
        requirement.acceptance_criterion_entity_version_id
    );
    assert_eq!(
        current_criterion.state,
        requirement.acceptance_criterion_state
    );
    let snapshot = engine
        .verification_requirement_at(
            requirement.commit_id,
            requirement.verification_requirement_entity_id,
        )
        .expect("requirement snapshot");
    assert_eq!(snapshot.workspace_id, workspace.workspace_id);
    assert_eq!(
        snapshot.acceptance_criterion_entity_id,
        criterion.acceptance_criterion_entity_id
    );
    assert_eq!(snapshot.local_key, "VR-1");
    assert_eq!(
        snapshot.verification_requirement_entity_version_id,
        requirement.verification_requirement_entity_version_id
    );
    assert_eq!(
        snapshot.state_digest,
        requirement.verification_requirement_state_digest
    );
    assert_eq!(snapshot.state, requirement.state);

    let connection = raw_connection(&path);
    assert_eq!(
        history_counts(&connection),
        HistoryCounts {
            object_identity: 3,
            entity: 3,
            acceptance_criterion_identity: 1,
            verification_requirement_identity: 1,
            entity_version: 5,
            changeset: 4,
            change_operation: 5,
            entity_membership_change: 5,
            workstate_commit: 4,
            commit_parent: 3,
            event: 4,
        }
    );
    assert_eq!(
        branch_head(&connection, workspace.initial_branch_id),
        requirement.commit_id
    );

    let requirement_entity_id = requirement.verification_requirement_entity_id.raw_bytes();
    let requirement_version_id = requirement
        .verification_requirement_entity_version_id
        .raw_bytes();
    let criterion_version_id = requirement
        .acceptance_criterion_entity_version_id
        .raw_bytes();
    let (
        owner_entity_id,
        local_key,
        requirement_kind,
        requirement_state_json,
        requirement_state_digest,
        stored_criterion_state_json,
        criterion_state_digest,
    ) = connection
        .query_row(
            "SELECT verification_requirement_identity.owner_entity_id,
                    verification_requirement_identity.local_key,
                    entity.entity_kind,
                    requirement_version.state_json,
                    requirement_version.state_digest,
                    criterion_version.state_json,
                    criterion_version.state_digest
             FROM verification_requirement_identity
             JOIN entity
               ON entity.object_id = verification_requirement_identity.entity_id
             JOIN entity_version AS requirement_version
               ON requirement_version.entity_id = verification_requirement_identity.entity_id
             JOIN entity_version AS criterion_version
               ON criterion_version.entity_id = verification_requirement_identity.owner_entity_id
             WHERE verification_requirement_identity.entity_id = ?1
               AND requirement_version.entity_version_id = ?2
               AND criterion_version.entity_version_id = ?3",
            params![
                &requirement_entity_id[..],
                &requirement_version_id[..],
                &criterion_version_id[..]
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
        .expect("stored requirement and criterion");
    assert_eq!(
        owner_entity_id,
        criterion.acceptance_criterion_entity_id.raw_bytes()
    );
    assert_eq!(local_key, "VR-1");
    assert_eq!(requirement_kind, "verification_requirement");
    assert_eq!(
        requirement_state_json,
        verification_requirement_state_json("The reopened store returns the same branch head.")
    );
    assert_eq!(
        digest_from_blob(requirement_state_digest),
        requirement.verification_requirement_state_digest
    );
    assert_eq!(
        stored_criterion_state_json,
        acceptance_criterion_state_json(
            "The store opens after process restart.",
            AcceptanceCriterionClassification::Required,
            vec![
                AcceptanceCriterionVerificationRequirementRef::new(
                    "VR-1",
                    requirement.verification_requirement_entity_id,
                )
                .expect("requirement ref")
            ],
        )
    );
    assert_eq!(
        digest_from_blob(criterion_state_digest),
        requirement.acceptance_criterion_state_digest
    );

    let operations = connection
        .prepare(
            "SELECT ordinal, subject_object_id
             FROM change_operation
             WHERE changeset_id = ?1
             ORDER BY ordinal",
        )
        .expect("prepare operations")
        .query_map(params![&requirement.changeset_id.raw_bytes()[..]], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, Vec<u8>>(1)?))
        })
        .expect("query operations")
        .collect::<std::result::Result<Vec<_>, _>>()
        .expect("operations");
    assert_eq!(operations.len(), 2);
    assert_eq!(operations[0], (0, requirement_entity_id.to_vec()));
    assert_eq!(
        operations[1],
        (
            1,
            criterion
                .acceptance_criterion_entity_id
                .raw_bytes()
                .to_vec()
        )
    );
    drop(connection);

    let report = engine.validate_integrity().expect("integrity");
    assert_eq!(report.checked_branches, 1);
    assert_eq!(report.checked_commits, 4);
}

#[test]
fn verification_requirement_create_rejects_duplicate_local_key_without_partial_rows() {
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
    let criterion = engine
        .create_acceptance_criterion(
            AcceptanceCriterionCreateOptions::new(
                workspace.initial_branch_id,
                task.commit_id,
                task.task_entity_id,
                task.task_entity_version_id,
                "AC-1",
                "Criterion.",
                AcceptanceCriterionClassification::Optional,
            )
            .expect("ac options"),
        )
        .expect("criterion");
    let first = engine
        .create_verification_requirement(
            VerificationRequirementCreateOptions::new(
                workspace.initial_branch_id,
                criterion.commit_id,
                criterion.acceptance_criterion_entity_id,
                criterion.acceptance_criterion_entity_version_id,
                "VR-1",
                "First requirement.",
            )
            .expect("first options"),
        )
        .expect("first requirement");

    let connection = raw_connection(&path);
    let before = history_counts(&connection);
    drop(connection);

    let error = engine
        .create_verification_requirement(
            VerificationRequirementCreateOptions::new(
                workspace.initial_branch_id,
                first.commit_id,
                criterion.acceptance_criterion_entity_id,
                first.acceptance_criterion_entity_version_id,
                "VR-1",
                "Duplicate requirement.",
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
fn verification_requirement_create_reuses_branch_cas_and_rolls_back_stale_loser() {
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
    let criterion = first_engine
        .create_acceptance_criterion(
            AcceptanceCriterionCreateOptions::new(
                workspace.initial_branch_id,
                task.commit_id,
                task.task_entity_id,
                task.task_entity_version_id,
                "AC-1",
                "Criterion.",
                AcceptanceCriterionClassification::Optional,
            )
            .expect("criterion options"),
        )
        .expect("criterion");
    let first_seen_head = first_engine
        .branch_head(workspace.initial_branch_id)
        .expect("first branch head")
        .head_commit_id;
    let second_seen_head = second_engine
        .branch_head(workspace.initial_branch_id)
        .expect("second branch head")
        .head_commit_id;
    assert_eq!(first_seen_head, criterion.commit_id);
    assert_eq!(second_seen_head, criterion.commit_id);

    let winner = first_engine
        .create_verification_requirement(
            VerificationRequirementCreateOptions::new(
                workspace.initial_branch_id,
                first_seen_head,
                criterion.acceptance_criterion_entity_id,
                criterion.acceptance_criterion_entity_version_id,
                "VR-1",
                "Winning requirement.",
            )
            .expect("winner options"),
        )
        .expect("winner requirement");

    let connection = raw_connection(&path);
    let before_loser = history_counts(&connection);
    drop(connection);

    let error = second_engine
        .create_verification_requirement(
            VerificationRequirementCreateOptions::new(
                workspace.initial_branch_id,
                second_seen_head,
                criterion.acceptance_criterion_entity_id,
                criterion.acceptance_criterion_entity_version_id,
                "VR-2",
                "Stale requirement.",
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
fn verification_requirement_revision_preserves_identity_local_key_and_history() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let task = engine
        .create_task(
            TaskCreateOptions::new(
                workspace.initial_branch_id,
                workspace.genesis_commit_id,
                "Revise requirement",
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
                "Criterion.",
                AcceptanceCriterionClassification::Optional,
            )
            .expect("criterion options"),
        )
        .expect("criterion");
    let created = engine
        .create_verification_requirement(
            VerificationRequirementCreateOptions::new(
                workspace.initial_branch_id,
                criterion.commit_id,
                criterion.acceptance_criterion_entity_id,
                criterion.acceptance_criterion_entity_version_id,
                "VR-1",
                "Initial requirement.",
            )
            .expect("create options"),
        )
        .expect("create requirement");
    let revised = engine
        .revise_verification_requirement(
            VerificationRequirementRevisionOptions::new(
                workspace.initial_branch_id,
                created.commit_id,
                created.verification_requirement_entity_id,
                created.verification_requirement_entity_version_id,
                "Revised requirement.",
            )
            .expect("revision options")
            .with_rationale(rationale("tighten verification intent")),
        )
        .expect("revise requirement");

    assert_eq!(
        revised.verification_requirement_entity_id,
        created.verification_requirement_entity_id
    );
    assert_eq!(
        revised.acceptance_criterion_entity_id,
        criterion.acceptance_criterion_entity_id
    );
    assert_eq!(revised.local_key, "VR-1");
    assert_eq!(
        revised.previous_verification_requirement_entity_version_id,
        created.verification_requirement_entity_version_id
    );
    assert_ne!(
        revised.verification_requirement_entity_version_id,
        created.verification_requirement_entity_version_id
    );
    assert_eq!(revised.previous_state, created.state);
    assert_eq!(revised.state.statement, "Revised requirement.");

    let old_snapshot = engine
        .verification_requirement_at(
            created.commit_id,
            created.verification_requirement_entity_id,
        )
        .expect("old snapshot");
    let new_snapshot = engine
        .verification_requirement_at(
            revised.commit_id,
            revised.verification_requirement_entity_id,
        )
        .expect("new snapshot");
    assert_eq!(old_snapshot.state.statement, "Initial requirement.");
    assert_eq!(new_snapshot.state.statement, "Revised requirement.");

    let criterion_after_revision = engine
        .acceptance_criterion_at(revised.commit_id, criterion.acceptance_criterion_entity_id)
        .expect("criterion after vr revision");
    assert_eq!(
        criterion_after_revision.acceptance_criterion_entity_version_id,
        created.acceptance_criterion_entity_version_id
    );
    assert_eq!(
        criterion_after_revision
            .state
            .verification_requirements
            .len(),
        1
    );
    assert_eq!(
        criterion_after_revision.state.verification_requirements[0].local_key,
        "VR-1"
    );

    let connection = raw_connection(&path);
    let requirement_version_count = connection
        .query_row(
            "SELECT count(*)
             FROM entity_version
             WHERE entity_id = ?1",
            params![&created.verification_requirement_entity_id.raw_bytes()[..]],
            |row| row.get::<_, i64>(0),
        )
        .expect("requirement version count");
    assert_eq!(requirement_version_count, 2);
    assert_eq!(
        count_rows(&connection, "verification_requirement_identity"),
        1
    );
    drop(connection);

    let report = engine.validate_integrity().expect("integrity");
    assert_eq!(report.checked_commits, 5);
}

#[test]
fn generic_entity_transition_cannot_bypass_verification_requirement_semantics() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let task = engine
        .create_task(
            TaskCreateOptions::new(
                workspace.initial_branch_id,
                workspace.genesis_commit_id,
                "Reserved requirement entity",
            )
            .expect("task options"),
        )
        .expect("task");
    let criterion = engine
        .create_acceptance_criterion(
            AcceptanceCriterionCreateOptions::new(
                workspace.initial_branch_id,
                task.commit_id,
                task.task_entity_id,
                task.task_entity_version_id,
                "AC-1",
                "Criterion.",
                AcceptanceCriterionClassification::Optional,
            )
            .expect("criterion options"),
        )
        .expect("criterion");
    let requirement = engine
        .create_verification_requirement(
            VerificationRequirementCreateOptions::new(
                workspace.initial_branch_id,
                criterion.commit_id,
                criterion.acceptance_criterion_entity_id,
                criterion.acceptance_criterion_entity_version_id,
                "VR-1",
                "Requirement.",
            )
            .expect("requirement options"),
        )
        .expect("requirement");

    let connection = raw_connection(&path);
    let before = history_counts(&connection);
    drop(connection);

    let update_error = engine
        .commit_entity_transition(
            EntityTransitionOptions::update(
                workspace.initial_branch_id,
                requirement.commit_id,
                requirement.verification_requirement_entity_id,
                requirement.verification_requirement_entity_version_id,
                VerificationRequirementState::new("Bypass update.")
                    .expect("bypass state")
                    .to_canonical_value()
                    .expect("bypass value"),
            )
            .expect("generic update options"),
        )
        .expect_err("generic requirement update is reserved");
    assert_eq!(update_error.code(), ErrorCode::EntityTransitionInvalid);
    assert_eq!(update_error.category(), ErrorCategory::Mutation);

    let create_error = engine
        .commit_entity_transition(
            EntityTransitionOptions::create(
                workspace.initial_branch_id,
                requirement.commit_id,
                "verification_requirement",
                VerificationRequirementState::new("Orphan requirement.")
                    .expect("orphan state")
                    .to_canonical_value()
                    .expect("orphan value"),
            )
            .expect("generic create options"),
        )
        .expect_err("generic requirement create is reserved");
    assert_eq!(create_error.code(), ErrorCode::EntityTransitionInvalid);
    assert_eq!(create_error.category(), ErrorCategory::Mutation);

    let connection = raw_connection(&path);
    assert_eq!(history_counts(&connection), before);
    assert_eq!(
        branch_head(&connection, workspace.initial_branch_id),
        requirement.commit_id
    );
}

#[test]
fn acceptance_criterion_verification_requirements_are_serialized_in_local_key_order() {
    let first_id = EntityId::new_v7();
    let second_id = EntityId::new_v7();
    let state = AcceptanceCriterionState {
        statement: "Sorted requirements".to_owned(),
        classification: AcceptanceCriterionClassification::Required,
        verification_requirements: vec![
            AcceptanceCriterionVerificationRequirementRef::new("VR-2", second_id)
                .expect("second ref"),
            AcceptanceCriterionVerificationRequirementRef::new("VR-1", first_id)
                .expect("first ref"),
        ],
    };

    let encoded =
        String::from_utf8(canonical_bytes(&state.to_canonical_value().expect("ac state")).unwrap())
            .expect("ac json");
    assert_eq!(
        encoded,
        format!(
            r#"{{"classification":"required","statement":"Sorted requirements","verification_requirements":[{{"entity_id":"{}","local_key":"VR-1"}},{{"entity_id":"{}","local_key":"VR-2"}}]}}"#,
            first_id, second_id
        )
    );

    let reversed = AcceptanceCriterionState {
        statement: state.statement.clone(),
        classification: state.classification,
        verification_requirements: vec![
            AcceptanceCriterionVerificationRequirementRef::new("VR-1", first_id)
                .expect("first ref"),
            AcceptanceCriterionVerificationRequirementRef::new("VR-2", second_id)
                .expect("second ref"),
        ],
    };
    assert_eq!(
        canonical_bytes(&state.to_canonical_value().expect("state")).expect("state bytes"),
        canonical_bytes(&reversed.to_canonical_value().expect("reversed")).expect("reversed bytes")
    );
}

#[test]
fn verification_requirement_state_is_canonical_and_rejects_invalid_input() {
    let encoded =
        verification_requirement_state_json("The reopened store returns the same branch head.");
    assert_eq!(
        encoded,
        r#"{"statement":"The reopened store returns the same branch head."}"#
    );
    let value =
        VerificationRequirementState::new("The reopened store returns the same branch head.")
            .expect("state")
            .to_canonical_value()
            .expect("canonical");
    assert_eq!(
        entity_version_digest(&value).expect("digest"),
        entity_version_digest(&value).expect("same digest")
    );

    assert!(
        VerificationRequirementCreateOptions::new(
            BranchId::new_v7(),
            CommitId::new_v7(),
            EntityId::new_v7(),
            workvcs_core::EntityVersionId::new_v7(),
            " VR-1 ",
            "Requirement.",
        )
        .is_err()
    );
    assert!(VerificationRequirementState::new("   ").is_err());
    assert!(VerificationRequirementState::new("bad\0statement").is_err());
}
