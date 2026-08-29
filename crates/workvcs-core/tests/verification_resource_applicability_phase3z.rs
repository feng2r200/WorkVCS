use rusqlite::Connection;
use std::path::{Path, PathBuf};
use tempfile::TempDir;
use workvcs_core::{
    AcceptanceCriterionClassification, AcceptanceCriterionCreateCommit,
    AcceptanceCriterionCreateOptions, AcceptanceCriterionEffectiveStatus, CanonicalValue, Engine,
    ErrorCode, ResourceCreateOptions, ResourceObservationCreateOptions, StoreInitOptions,
    TaskCreateCommit, TaskCreateOptions, TaskStatus, TaskTransitionOptions,
    VerificationCreateOptions, VerificationRequirementCreateOptions, VerificationResourceBasis,
    VerificationResult, VerificationTarget, WorkspaceInfo, WorkspaceInitOptions,
    content_object_digest,
};

struct CriterionFixture {
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
        StoreInitOptions::new("phase3z-resource-applicability-unknown-store")
            .expect("store options"),
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

fn object(entries: Vec<(&str, CanonicalValue)>) -> CanonicalValue {
    CanonicalValue::object(
        entries
            .into_iter()
            .map(|(key, value)| (key.to_owned(), value))
            .collect(),
    )
    .expect("canonical object")
}

fn string(value: &str) -> CanonicalValue {
    CanonicalValue::String(value.to_owned())
}

fn count_rows(connection: &Connection, table: &str) -> i64 {
    connection
        .query_row(&format!("SELECT count(*) FROM {table}"), [], |row| {
            row.get(0)
        })
        .expect("count rows")
}

fn create_required_criterion(engine: &mut Engine, workspace: &WorkspaceInfo) -> CriterionFixture {
    let task = engine
        .create_task(
            TaskCreateOptions::new(
                workspace.initial_branch_id,
                workspace.genesis_commit_id,
                "Implement resource applicability projection",
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
                "Resource-backed verification must not be assumed current.",
                AcceptanceCriterionClassification::Required,
            )
            .expect("criterion options"),
        )
        .expect("create acceptance criterion");
    CriterionFixture { task, criterion }
}

fn create_resource_basis(engine: &mut Engine) -> VerificationResourceBasis {
    let resource = engine
        .create_resource(ResourceCreateOptions::new("git").expect("resource options"))
        .expect("create resource");
    let fingerprint = content_object_digest(b"resource applicability baseline");
    let observation = engine
        .record_resource_observation(
            ResourceObservationCreateOptions::new(
                resource.resource_id,
                "git",
                1,
                fingerprint,
                object(vec![("label", string("baseline"))]),
            )
            .expect("observation options"),
        )
        .expect("record observation");
    VerificationResourceBasis::new(
        resource.resource_id,
        "git",
        1,
        "path",
        1,
        object(vec![("path", string("src/lib.rs"))]),
        fingerprint,
    )
    .expect("resource basis")
    .with_baseline_observation_id(observation.observation_id)
    .expect("observation basis")
}

#[test]
fn resource_basis_without_comparison_evidence_projects_to_stale_effective_status() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let fixture = create_required_criterion(&mut engine, &workspace);
    let resource_basis = create_resource_basis(&mut engine);
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
            .with_resource_basis(vec![resource_basis])
            .expect("resource basis"),
        )
        .expect("create verification");

    assert_eq!(
        engine
            .acceptance_criterion_effective_status(
                verification.commit_id,
                fixture.criterion.acceptance_criterion_entity_id,
            )
            .expect("effective status"),
        AcceptanceCriterionEffectiveStatus::Stale
    );
    let blocked = engine
        .transition_task(
            TaskTransitionOptions::new(
                workspace.initial_branch_id,
                verification.commit_id,
                fixture.task.task_entity_id,
                fixture.criterion.task_entity_version_id,
                TaskStatus::Done,
            )
            .expect("done options")
            .with_outcome("done")
            .expect("outcome"),
        )
        .expect_err("unknown resource applicability should not satisfy mandatory AC");
    assert_eq!(blocked.code(), ErrorCode::TaskInvalid);
    assert!(blocked.to_string().contains("stale"));

    let connection = raw_connection(&path);
    assert_eq!(
        count_rows(&connection, "verification_applicability_cache"),
        0
    );
    assert_eq!(count_rows(&connection, "applicability_resource_stamp"), 0);
}

#[test]
fn resource_unknown_requirement_verification_keeps_parent_criterion_stale() {
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
                "Prove the resource-backed condition.",
            )
            .expect("requirement options"),
        )
        .expect("create requirement");
    let resource_basis = create_resource_basis(&mut engine);
    let verification = engine
        .create_verification(
            VerificationCreateOptions::new(
                workspace.initial_branch_id,
                requirement.commit_id,
                VerificationTarget::VerificationRequirement(
                    requirement.verification_requirement_entity_id,
                ),
                VerificationResult::Passed,
            )
            .expect("verification options")
            .with_resource_basis(vec![resource_basis])
            .expect("resource basis"),
        )
        .expect("create verification");

    assert_eq!(
        engine
            .acceptance_criterion_effective_status(
                verification.commit_id,
                fixture.criterion.acceptance_criterion_entity_id,
            )
            .expect("effective status"),
        AcceptanceCriterionEffectiveStatus::Stale
    );
    let blocked = engine
        .transition_task(
            TaskTransitionOptions::new(
                workspace.initial_branch_id,
                verification.commit_id,
                fixture.task.task_entity_id,
                fixture.criterion.task_entity_version_id,
                TaskStatus::Done,
            )
            .expect("done options")
            .with_outcome("done")
            .expect("outcome"),
        )
        .expect_err("unknown requirement verification should not satisfy mandatory AC");
    assert_eq!(blocked.code(), ErrorCode::TaskInvalid);
    assert!(blocked.to_string().contains("stale"));
}

#[test]
fn work_state_only_passed_verification_remains_applicable() {
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
            .expect("verification options"),
        )
        .expect("create verification");

    assert_eq!(
        engine
            .acceptance_criterion_effective_status(
                verification.commit_id,
                fixture.criterion.acceptance_criterion_entity_id,
            )
            .expect("effective status"),
        AcceptanceCriterionEffectiveStatus::Verified
    );
}
