use rusqlite::{Connection, params};
use std::path::{Path, PathBuf};
use tempfile::TempDir;
use workvcs_core::{
    AcceptanceCriterionClassification, AcceptanceCriterionCreateCommit,
    AcceptanceCriterionCreateOptions, AcceptanceCriterionEffectiveStatus,
    AcceptanceCriterionRevisionOptions, CanonicalValue, CommitId, Engine, EntityId,
    EntityTransitionOptions, ErrorCode, StoreInitOptions, TaskCreateCommit, TaskCreateOptions,
    TaskStatus, TaskTransitionOptions, VerificationCreateOptions,
    VerificationRequirementCreateOptions, VerificationResult, VerificationSemanticDependency,
    VerificationTarget, WorkspaceInfo, WorkspaceInitOptions, relation_version_digest,
};

#[derive(Clone, Debug, PartialEq, Eq)]
struct HistoryCounts {
    object_identity: i64,
    entity: i64,
    acceptance_criterion_identity: i64,
    verification_requirement_identity: i64,
    relation: i64,
    relation_version: i64,
    entity_version: i64,
    changeset: i64,
    change_operation: i64,
    entity_membership_change: i64,
    relation_membership_change: i64,
    verification_basis: i64,
    verification_semantic_dependency: i64,
    workstate_commit: i64,
    commit_parent: i64,
    event: i64,
}

struct RequiredCriterionFixture {
    task: TaskCreateCommit,
    criterion: AcceptanceCriterionCreateCommit,
}

fn store_path() -> (TempDir, PathBuf) {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let path = tempdir.path().join("workvcs.sqlite");
    (tempdir, path)
}

fn init_engine(path: &Path) -> Engine {
    Engine::init(
        path,
        StoreInitOptions::new("phase3d-verification-store").expect("store options"),
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

fn create_required_criterion(
    engine: &mut Engine,
    workspace: &WorkspaceInfo,
) -> RequiredCriterionFixture {
    let task = engine
        .create_task(
            TaskCreateOptions::new(
                workspace.initial_branch_id,
                workspace.genesis_commit_id,
                "Implement a verified slice",
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
                "The slice has verified behavior.",
                AcceptanceCriterionClassification::Required,
            )
            .expect("ac options"),
        )
        .expect("create acceptance criterion");
    RequiredCriterionFixture { task, criterion }
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
        relation: count_rows(connection, "relation"),
        relation_version: count_rows(connection, "relation_version"),
        entity_version: count_rows(connection, "entity_version"),
        changeset: count_rows(connection, "changeset"),
        change_operation: count_rows(connection, "change_operation"),
        entity_membership_change: count_rows(connection, "entity_membership_change"),
        relation_membership_change: count_rows(connection, "relation_membership_change"),
        verification_basis: count_rows(connection, "verification_basis"),
        verification_semantic_dependency: count_rows(
            connection,
            "verification_semantic_dependency",
        ),
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

fn branch_head(connection: &Connection, workspace: &WorkspaceInfo) -> CommitId {
    let branch_id = workspace.initial_branch_id.raw_bytes();
    let bytes = connection
        .query_row(
            "SELECT head_commit_id FROM branch WHERE branch_id = ?1",
            params![&branch_id[..]],
            |row| row.get::<_, Vec<u8>>(0),
        )
        .expect("branch head");
    CommitId::from_bytes(bytes.try_into().expect("commit id bytes")).expect("commit id")
}

fn method(name: &str) -> CanonicalValue {
    CanonicalValue::object(vec![
        (
            "kind".to_owned(),
            CanonicalValue::String("manual".to_owned()),
        ),
        ("name".to_owned(), CanonicalValue::String(name.to_owned())),
    ])
    .expect("method")
}

fn outcome(value: &str) -> String {
    value.to_owned()
}

#[test]
fn direct_passed_verification_satisfies_mandatory_acceptance_criterion() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let fixture = create_required_criterion(&mut engine, &workspace);

    assert_eq!(
        engine
            .acceptance_criterion_effective_status(
                fixture.criterion.commit_id,
                fixture.criterion.acceptance_criterion_entity_id,
            )
            .expect("initial effective status"),
        AcceptanceCriterionEffectiveStatus::Unverified
    );
    let blocked = engine
        .transition_task(
            TaskTransitionOptions::new(
                workspace.initial_branch_id,
                fixture.criterion.commit_id,
                fixture.task.task_entity_id,
                fixture.criterion.task_entity_version_id,
                TaskStatus::Done,
            )
            .expect("done options")
            .with_outcome(outcome("completed"))
            .expect("done outcome"),
        )
        .expect_err("mandatory unverified criterion blocks done");
    assert_eq!(blocked.code(), ErrorCode::TaskInvalid);
    assert!(blocked.to_string().contains("unverified"));

    let verification = engine
        .create_verification(
            VerificationCreateOptions::new(
                workspace.initial_branch_id,
                fixture.criterion.commit_id,
                VerificationTarget::AcceptanceCriterion(
                    fixture.criterion.acceptance_criterion_entity_id,
                ),
                VerificationResult::Passed,
            )
            .expect("verification options")
            .with_method(method("reviewed passing test output"))
            .expect("method"),
        )
        .expect("record verification");

    assert_eq!(verification.workspace_id, workspace.workspace_id);
    assert_eq!(verification.branch_id, workspace.initial_branch_id);
    assert_eq!(
        verification.previous_head_commit_id,
        fixture.criterion.commit_id
    );
    assert_eq!(
        verification.target,
        VerificationTarget::AcceptanceCriterion(fixture.criterion.acceptance_criterion_entity_id)
    );
    assert_eq!(verification.state.result, VerificationResult::Passed);
    assert_eq!(
        verification.state.verified_at_commit_id,
        fixture.criterion.commit_id
    );
    assert_eq!(
        verification.state.semantic_dependencies,
        vec![VerificationSemanticDependency::new(
            fixture.criterion.acceptance_criterion_entity_id,
            fixture.criterion.acceptance_criterion_entity_version_id,
        )]
    );
    assert_ne!(
        verification.verification_operation_id,
        verification.verifies_relation_operation_id
    );

    let replayed = engine
        .state_at(verification.commit_id)
        .expect("replay after verification");
    assert_eq!(replayed.state.relations().len(), 1);
    assert_eq!(
        replayed.state.relations()[0],
        (
            verification.verifies_relation_id,
            verification.verifies_relation_version_id,
        )
    );

    let snapshot = engine
        .verification_at(verification.commit_id, verification.verification_entity_id)
        .expect("verification snapshot");
    assert_eq!(snapshot.target, verification.target);
    assert_eq!(snapshot.state, verification.state);
    assert_eq!(
        snapshot.verifies_relation_state_digest,
        relation_version_digest(&CanonicalValue::object(Vec::new()).expect("empty object"))
            .expect("relation digest")
    );

    let connection = raw_connection(&path);
    assert_eq!(
        history_counts(&connection),
        HistoryCounts {
            object_identity: 4,
            entity: 3,
            acceptance_criterion_identity: 1,
            verification_requirement_identity: 0,
            relation: 1,
            relation_version: 1,
            entity_version: 4,
            changeset: 4,
            change_operation: 5,
            entity_membership_change: 4,
            relation_membership_change: 1,
            verification_basis: 1,
            verification_semantic_dependency: 1,
            workstate_commit: 4,
            commit_parent: 3,
            event: 4,
        }
    );
    assert_eq!(branch_head(&connection, &workspace), verification.commit_id);
    drop(connection);

    assert_eq!(
        engine
            .acceptance_criterion_effective_status(
                verification.commit_id,
                fixture.criterion.acceptance_criterion_entity_id,
            )
            .expect("verified effective status"),
        AcceptanceCriterionEffectiveStatus::Verified
    );
    engine
        .transition_task(
            TaskTransitionOptions::new(
                workspace.initial_branch_id,
                verification.commit_id,
                fixture.task.task_entity_id,
                fixture.criterion.task_entity_version_id,
                TaskStatus::Done,
            )
            .expect("done options")
            .with_outcome(outcome("completed"))
            .expect("done outcome"),
        )
        .expect("verified mandatory criterion allows done");
}

#[test]
fn verification_requirements_require_each_requirement_to_be_verified() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let fixture = create_required_criterion(&mut engine, &workspace);
    let first_requirement = engine
        .create_verification_requirement(
            VerificationRequirementCreateOptions::new(
                workspace.initial_branch_id,
                fixture.criterion.commit_id,
                fixture.criterion.acceptance_criterion_entity_id,
                fixture.criterion.acceptance_criterion_entity_version_id,
                "VR-1",
                "Persist the expected rows.",
            )
            .expect("first requirement options"),
        )
        .expect("first requirement");
    let second_requirement = engine
        .create_verification_requirement(
            VerificationRequirementCreateOptions::new(
                workspace.initial_branch_id,
                first_requirement.commit_id,
                fixture.criterion.acceptance_criterion_entity_id,
                first_requirement.acceptance_criterion_entity_version_id,
                "VR-2",
                "Replay the resulting WorkState.",
            )
            .expect("second requirement options"),
        )
        .expect("second requirement");

    let direct_error = engine
        .create_verification(
            VerificationCreateOptions::new(
                workspace.initial_branch_id,
                second_requirement.commit_id,
                VerificationTarget::AcceptanceCriterion(
                    fixture.criterion.acceptance_criterion_entity_id,
                ),
                VerificationResult::Passed,
            )
            .expect("direct verification options"),
        )
        .expect_err("direct AC verification is invalid when requirements exist");
    assert_eq!(direct_error.code(), ErrorCode::TaskInvalid);
    assert!(
        direct_error
            .to_string()
            .contains("must target a requirement")
    );

    let first_verification = engine
        .create_verification(
            VerificationCreateOptions::new(
                workspace.initial_branch_id,
                second_requirement.commit_id,
                VerificationTarget::VerificationRequirement(
                    first_requirement.verification_requirement_entity_id,
                ),
                VerificationResult::Passed,
            )
            .expect("first verification options")
            .with_method(method("first requirement review"))
            .expect("method"),
        )
        .expect("verify first requirement");
    assert_eq!(
        engine
            .acceptance_criterion_effective_status(
                first_verification.commit_id,
                fixture.criterion.acceptance_criterion_entity_id,
            )
            .expect("partially verified status"),
        AcceptanceCriterionEffectiveStatus::Unverified
    );
    let blocked = engine
        .transition_task(
            TaskTransitionOptions::new(
                workspace.initial_branch_id,
                first_verification.commit_id,
                fixture.task.task_entity_id,
                fixture.criterion.task_entity_version_id,
                TaskStatus::Done,
            )
            .expect("done options")
            .with_outcome(outcome("completed"))
            .expect("done outcome"),
        )
        .expect_err("missing requirement verification blocks done");
    assert!(blocked.to_string().contains("unverified"));

    let second_verification = engine
        .create_verification(
            VerificationCreateOptions::new(
                workspace.initial_branch_id,
                first_verification.commit_id,
                VerificationTarget::VerificationRequirement(
                    second_requirement.verification_requirement_entity_id,
                ),
                VerificationResult::Passed,
            )
            .expect("second verification options")
            .with_method(method("second requirement review"))
            .expect("method"),
        )
        .expect("verify second requirement");
    assert_eq!(
        engine
            .acceptance_criterion_effective_status(
                second_verification.commit_id,
                fixture.criterion.acceptance_criterion_entity_id,
            )
            .expect("fully verified status"),
        AcceptanceCriterionEffectiveStatus::Verified
    );
    engine
        .transition_task(
            TaskTransitionOptions::new(
                workspace.initial_branch_id,
                second_verification.commit_id,
                fixture.task.task_entity_id,
                fixture.criterion.task_entity_version_id,
                TaskStatus::Done,
            )
            .expect("done options")
            .with_outcome(outcome("completed"))
            .expect("done outcome"),
        )
        .expect("all requirements verified allows done");
}

#[test]
fn acceptance_criterion_revision_preserves_requirements_and_cannot_reenable_direct_verification() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let fixture = create_required_criterion(&mut engine, &workspace);
    let requirement = engine
        .create_verification_requirement(
            VerificationRequirementCreateOptions::new(
                workspace.initial_branch_id,
                fixture.criterion.commit_id,
                fixture.criterion.acceptance_criterion_entity_id,
                fixture.criterion.acceptance_criterion_entity_version_id,
                "VR-1",
                "Replay includes the verification relation.",
            )
            .expect("requirement options"),
        )
        .expect("create requirement");
    let revised_criterion = engine
        .revise_acceptance_criterion(
            AcceptanceCriterionRevisionOptions::new(
                workspace.initial_branch_id,
                requirement.commit_id,
                fixture.criterion.acceptance_criterion_entity_id,
                requirement.acceptance_criterion_entity_version_id,
                "The slice has verified behavior after an AC wording revision.",
                AcceptanceCriterionClassification::Required,
            )
            .expect("revision options"),
        )
        .expect("revise acceptance criterion");
    assert_eq!(
        revised_criterion.state.verification_requirements,
        requirement
            .acceptance_criterion_state
            .verification_requirements
    );

    let direct_error = engine
        .create_verification(
            VerificationCreateOptions::new(
                workspace.initial_branch_id,
                revised_criterion.commit_id,
                VerificationTarget::AcceptanceCriterion(
                    fixture.criterion.acceptance_criterion_entity_id,
                ),
                VerificationResult::Passed,
            )
            .expect("direct verification options")
            .with_method(method("attempted direct AC verification"))
            .expect("method"),
        )
        .expect_err("AC requirements survive revision and reject direct verification");
    assert_eq!(direct_error.code(), ErrorCode::TaskInvalid);
    assert!(
        direct_error
            .to_string()
            .contains("must target a requirement")
    );

    assert_eq!(
        engine
            .acceptance_criterion_effective_status(
                revised_criterion.commit_id,
                fixture.criterion.acceptance_criterion_entity_id,
            )
            .expect("unverified status"),
        AcceptanceCriterionEffectiveStatus::Unverified
    );
    let blocked = engine
        .transition_task(
            TaskTransitionOptions::new(
                workspace.initial_branch_id,
                revised_criterion.commit_id,
                fixture.task.task_entity_id,
                fixture.criterion.task_entity_version_id,
                TaskStatus::Done,
            )
            .expect("done options")
            .with_outcome(outcome("completed"))
            .expect("done outcome"),
        )
        .expect_err("VR verification is still required after AC revision");
    assert!(blocked.to_string().contains("unverified"));

    let verified_requirement = engine
        .create_verification(
            VerificationCreateOptions::new(
                workspace.initial_branch_id,
                revised_criterion.commit_id,
                VerificationTarget::VerificationRequirement(
                    requirement.verification_requirement_entity_id,
                ),
                VerificationResult::Passed,
            )
            .expect("requirement verification options")
            .with_method(method("requirement verification"))
            .expect("method"),
        )
        .expect("verify requirement");
    assert_eq!(
        engine
            .acceptance_criterion_effective_status(
                verified_requirement.commit_id,
                fixture.criterion.acceptance_criterion_entity_id,
            )
            .expect("verified status"),
        AcceptanceCriterionEffectiveStatus::Verified
    );
    engine
        .transition_task(
            TaskTransitionOptions::new(
                workspace.initial_branch_id,
                verified_requirement.commit_id,
                fixture.task.task_entity_id,
                fixture.criterion.task_entity_version_id,
                TaskStatus::Done,
            )
            .expect("done options")
            .with_outcome(outcome("completed"))
            .expect("done outcome"),
        )
        .expect("requirement verification allows done after AC revision");
}

#[test]
fn failed_and_passed_applicable_judgments_project_to_conflicted() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let fixture = create_required_criterion(&mut engine, &workspace);

    let failed = engine
        .create_verification(
            VerificationCreateOptions::new(
                workspace.initial_branch_id,
                fixture.criterion.commit_id,
                VerificationTarget::AcceptanceCriterion(
                    fixture.criterion.acceptance_criterion_entity_id,
                ),
                VerificationResult::Failed,
            )
            .expect("failed verification options")
            .with_method(method("manual failure"))
            .expect("method"),
        )
        .expect("record failed verification");
    assert_eq!(
        engine
            .acceptance_criterion_effective_status(
                failed.commit_id,
                fixture.criterion.acceptance_criterion_entity_id,
            )
            .expect("failed status"),
        AcceptanceCriterionEffectiveStatus::Failed
    );

    let passed = engine
        .create_verification(
            VerificationCreateOptions::new(
                workspace.initial_branch_id,
                failed.commit_id,
                VerificationTarget::AcceptanceCriterion(
                    fixture.criterion.acceptance_criterion_entity_id,
                ),
                VerificationResult::Passed,
            )
            .expect("passed verification options")
            .with_method(method("manual pass"))
            .expect("method"),
        )
        .expect("record passed verification");
    assert_eq!(
        engine
            .acceptance_criterion_effective_status(
                passed.commit_id,
                fixture.criterion.acceptance_criterion_entity_id,
            )
            .expect("conflicted status"),
        AcceptanceCriterionEffectiveStatus::Conflicted
    );
    let blocked = engine
        .transition_task(
            TaskTransitionOptions::new(
                workspace.initial_branch_id,
                passed.commit_id,
                fixture.task.task_entity_id,
                fixture.criterion.task_entity_version_id,
                TaskStatus::Done,
            )
            .expect("done options")
            .with_outcome(outcome("completed"))
            .expect("done outcome"),
        )
        .expect_err("conflicted criterion blocks done");
    assert!(blocked.to_string().contains("conflicted"));
}

#[test]
fn verification_becomes_stale_when_recorded_semantic_dependency_changes() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let fixture = create_required_criterion(&mut engine, &workspace);
    let verification = engine
        .create_verification(
            VerificationCreateOptions::new(
                workspace.initial_branch_id,
                fixture.criterion.commit_id,
                VerificationTarget::AcceptanceCriterion(
                    fixture.criterion.acceptance_criterion_entity_id,
                ),
                VerificationResult::Passed,
            )
            .expect("verification options")
            .with_method(method("initial review"))
            .expect("method"),
        )
        .expect("record verification");
    assert_eq!(
        engine
            .acceptance_criterion_effective_status(
                verification.commit_id,
                fixture.criterion.acceptance_criterion_entity_id,
            )
            .expect("verified status"),
        AcceptanceCriterionEffectiveStatus::Verified
    );

    let revised = engine
        .revise_acceptance_criterion(
            AcceptanceCriterionRevisionOptions::new(
                workspace.initial_branch_id,
                verification.commit_id,
                fixture.criterion.acceptance_criterion_entity_id,
                fixture.criterion.acceptance_criterion_entity_version_id,
                "The slice has verified behavior after revision.",
                AcceptanceCriterionClassification::Required,
            )
            .expect("revision options"),
        )
        .expect("revise criterion");
    assert_eq!(
        engine
            .acceptance_criterion_effective_status(
                revised.commit_id,
                fixture.criterion.acceptance_criterion_entity_id,
            )
            .expect("stale status"),
        AcceptanceCriterionEffectiveStatus::Stale
    );
    let blocked = engine
        .transition_task(
            TaskTransitionOptions::new(
                workspace.initial_branch_id,
                revised.commit_id,
                fixture.task.task_entity_id,
                fixture.criterion.task_entity_version_id,
                TaskStatus::Done,
            )
            .expect("done options")
            .with_outcome(outcome("completed"))
            .expect("done outcome"),
        )
        .expect_err("stale criterion blocks done");
    assert!(blocked.to_string().contains("stale"));
}

#[test]
fn verification_create_rejects_invalid_target_and_invalid_method_without_partial_rows() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let fixture = create_required_criterion(&mut engine, &workspace);
    let connection = raw_connection(&path);
    let before = history_counts(&connection);
    drop(connection);

    let bad_method = VerificationCreateOptions::new(
        workspace.initial_branch_id,
        fixture.criterion.commit_id,
        VerificationTarget::AcceptanceCriterion(fixture.criterion.acceptance_criterion_entity_id),
        VerificationResult::Passed,
    )
    .expect("verification options")
    .with_method(CanonicalValue::Array(Vec::new()))
    .expect_err("method must be object");
    assert_eq!(bad_method.code(), ErrorCode::TaskInvalid);

    let missing_target = engine
        .create_verification(
            VerificationCreateOptions::new(
                workspace.initial_branch_id,
                fixture.criterion.commit_id,
                VerificationTarget::AcceptanceCriterion(EntityId::new_v7()),
                VerificationResult::Passed,
            )
            .expect("missing target options")
            .with_method(method("missing target"))
            .expect("method"),
        )
        .expect_err("target absent at commit");
    assert_eq!(missing_target.code(), ErrorCode::TaskNotFound);

    let connection = raw_connection(&path);
    assert_eq!(history_counts(&connection), before);
    assert_eq!(
        branch_head(&connection, &workspace),
        fixture.criterion.commit_id
    );
}

#[test]
fn generic_entity_transition_cannot_bypass_verification_semantics() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let fixture = create_required_criterion(&mut engine, &workspace);

    let create_error = engine
        .commit_entity_transition(
            EntityTransitionOptions::create(
                workspace.initial_branch_id,
                fixture.criterion.commit_id,
                "verification",
                CanonicalValue::object(Vec::new()).expect("empty state"),
            )
            .expect("generic create options"),
        )
        .expect_err("reserved create kind is rejected");
    assert_eq!(create_error.code(), ErrorCode::EntityTransitionInvalid);

    let verification = engine
        .create_verification(
            VerificationCreateOptions::new(
                workspace.initial_branch_id,
                fixture.criterion.commit_id,
                VerificationTarget::AcceptanceCriterion(
                    fixture.criterion.acceptance_criterion_entity_id,
                ),
                VerificationResult::Passed,
            )
            .expect("verification options")
            .with_method(method("approved"))
            .expect("method"),
        )
        .expect("record verification");
    let update_error = engine
        .commit_entity_transition(
            EntityTransitionOptions::update(
                workspace.initial_branch_id,
                verification.commit_id,
                verification.verification_entity_id,
                verification.verification_entity_version_id,
                CanonicalValue::object(Vec::new()).expect("replacement state"),
            )
            .expect("generic update options"),
        )
        .expect_err("reserved update kind is rejected");
    assert_eq!(update_error.code(), ErrorCode::EntityTransitionInvalid);
}
