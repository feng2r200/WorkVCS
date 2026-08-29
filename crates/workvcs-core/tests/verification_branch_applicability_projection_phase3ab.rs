use rusqlite::Connection;
use std::path::{Path, PathBuf};
use tempfile::TempDir;
use workvcs_core::{
    AcceptanceCriterionClassification, AcceptanceCriterionCreateCommit,
    AcceptanceCriterionCreateOptions, AcceptanceCriterionEffectiveStatus,
    ApplicabilityResourceStampInput, CanonicalValue, Engine, ErrorCode, ResourceCreateOptions,
    ResourceObservationCreateOptions, StoreInitOptions, TaskCreateCommit, TaskCreateOptions,
    TaskStatus, TaskTransitionOptions, VerificationApplicabilityRecordOptions,
    VerificationCreateCommit, VerificationCreateOptions, VerificationRequirementCreateOptions,
    VerificationResourceBasis, VerificationResult, VerificationTarget, WorkspaceInfo,
    WorkspaceInitOptions, content_object_digest,
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
        StoreInitOptions::new("phase3ab-branch-aware-applicability-projection-store")
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
                "Implement branch-aware applicability projection",
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
                "Resource-backed verification can satisfy the branch gate.",
                AcceptanceCriterionClassification::Required,
            )
            .expect("criterion options"),
        )
        .expect("create acceptance criterion");
    CriterionFixture { task, criterion }
}

fn create_resource_basis(engine: &mut Engine, label: &str) -> VerificationResourceBasis {
    let resource = engine
        .create_resource(ResourceCreateOptions::new("git").expect("resource options"))
        .expect("create resource");
    let fingerprint = content_object_digest(format!("baseline:{label}").as_bytes());
    let observation = engine
        .record_resource_observation(
            ResourceObservationCreateOptions::new(
                resource.resource_id,
                "git",
                1,
                fingerprint,
                object(vec![("label", string(label))]),
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

fn observed_stamp(resource_basis: &VerificationResourceBasis) -> ApplicabilityResourceStampInput {
    ApplicabilityResourceStampInput::observed(
        0,
        resource_basis.adapter_kind.clone(),
        resource_basis.adapter_schema_version,
        resource_basis.scope_schema_version,
        resource_basis.baseline_fingerprint,
    )
    .expect("observed stamp")
}

fn unavailable_stamp(
    resource_basis: &VerificationResourceBasis,
) -> ApplicabilityResourceStampInput {
    ApplicabilityResourceStampInput::unavailable(
        0,
        resource_basis.adapter_kind.clone(),
        resource_basis.adapter_schema_version,
        resource_basis.scope_schema_version,
    )
    .expect("unavailable stamp")
}

fn record_applicability(
    engine: &mut Engine,
    workspace: &WorkspaceInfo,
    verification: &VerificationCreateCommit,
    stamps: Vec<ApplicabilityResourceStampInput>,
) {
    engine
        .record_verification_applicability(
            VerificationApplicabilityRecordOptions::new(
                workspace.initial_branch_id,
                verification.verification_entity_id,
                verification.commit_id,
            )
            .expect("record options")
            .with_resource_stamps(stamps)
            .expect("record stamps")
            .with_detail(object(vec![("source", string("phase3ab"))]))
            .expect("record detail"),
        )
        .expect("record applicability");
}

fn record_applicability_at_head(
    engine: &mut Engine,
    workspace: &WorkspaceInfo,
    verification: &VerificationCreateCommit,
    head: workvcs_core::CommitId,
    stamps: Vec<ApplicabilityResourceStampInput>,
) {
    engine
        .record_verification_applicability(
            VerificationApplicabilityRecordOptions::new(
                workspace.initial_branch_id,
                verification.verification_entity_id,
                head,
            )
            .expect("record options")
            .with_resource_stamps(stamps)
            .expect("record stamps")
            .with_detail(object(vec![("source", string("phase3ab"))]))
            .expect("record detail"),
        )
        .expect("record applicability");
}

fn create_direct_resource_verification(
    engine: &mut Engine,
    workspace: &WorkspaceInfo,
    criterion: &AcceptanceCriterionCreateCommit,
    resource_basis: VerificationResourceBasis,
) -> VerificationCreateCommit {
    engine
        .create_verification(
            VerificationCreateOptions::new(
                workspace.initial_branch_id,
                criterion.commit_id,
                VerificationTarget::AcceptanceCriterion(criterion.acceptance_criterion_entity_id),
                VerificationResult::Passed,
            )
            .expect("verification options")
            .with_resource_basis(vec![resource_basis])
            .expect("resource basis"),
        )
        .expect("create verification")
}

#[test]
fn branch_effective_status_consumes_current_applicable_cache_and_allows_done() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let fixture = create_required_criterion(&mut engine, &workspace);
    let resource_basis = create_resource_basis(&mut engine, "baseline");
    let verification = create_direct_resource_verification(
        &mut engine,
        &workspace,
        &fixture.criterion,
        resource_basis.clone(),
    );
    let connection = raw_connection(&path);
    let before_workstate_commits = count_rows(&connection, "workstate_commit");

    assert_eq!(
        engine
            .acceptance_criterion_effective_status_for_branch(
                workspace.initial_branch_id,
                fixture.criterion.acceptance_criterion_entity_id,
            )
            .expect("branch status without cache"),
        AcceptanceCriterionEffectiveStatus::Stale
    );

    record_applicability(
        &mut engine,
        &workspace,
        &verification,
        vec![observed_stamp(&resource_basis)],
    );
    assert_eq!(
        count_rows(&connection, "workstate_commit"),
        before_workstate_commits
    );
    assert_eq!(
        engine
            .acceptance_criterion_effective_status_for_branch(
                workspace.initial_branch_id,
                fixture.criterion.acceptance_criterion_entity_id,
            )
            .expect("branch status with cache"),
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
            .with_outcome("done")
            .expect("done outcome"),
        )
        .expect("current applicable cache should satisfy mandatory AC");
}

#[test]
fn branch_effective_status_ignores_cache_recorded_for_old_head() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let fixture = create_required_criterion(&mut engine, &workspace);
    let resource_basis = create_resource_basis(&mut engine, "baseline");
    let verification = create_direct_resource_verification(
        &mut engine,
        &workspace,
        &fixture.criterion,
        resource_basis.clone(),
    );
    record_applicability(
        &mut engine,
        &workspace,
        &verification,
        vec![observed_stamp(&resource_basis)],
    );

    let unrelated = engine
        .create_task(
            TaskCreateOptions::new(
                workspace.initial_branch_id,
                verification.commit_id,
                "Advance the branch after the cache was recorded",
            )
            .expect("advance options"),
        )
        .expect("advance branch");

    assert_eq!(
        engine
            .acceptance_criterion_effective_status_for_branch(
                workspace.initial_branch_id,
                fixture.criterion.acceptance_criterion_entity_id,
            )
            .expect("branch status with old cache"),
        AcceptanceCriterionEffectiveStatus::Stale
    );

    record_applicability_at_head(
        &mut engine,
        &workspace,
        &verification,
        unrelated.commit_id,
        vec![observed_stamp(&resource_basis)],
    );
    assert_eq!(
        engine
            .acceptance_criterion_effective_status_for_branch(
                workspace.initial_branch_id,
                fixture.criterion.acceptance_criterion_entity_id,
            )
            .expect("branch status with refreshed cache"),
        AcceptanceCriterionEffectiveStatus::Verified
    );
}

#[test]
fn branch_effective_status_consumes_requirement_target_cache() {
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
    let resource_basis = create_resource_basis(&mut engine, "requirement-baseline");
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
            .with_resource_basis(vec![resource_basis.clone()])
            .expect("resource basis"),
        )
        .expect("create verification");

    assert_eq!(
        engine
            .acceptance_criterion_effective_status_for_branch(
                workspace.initial_branch_id,
                fixture.criterion.acceptance_criterion_entity_id,
            )
            .expect("requirement status without cache"),
        AcceptanceCriterionEffectiveStatus::Stale
    );
    record_applicability(
        &mut engine,
        &workspace,
        &verification,
        vec![observed_stamp(&resource_basis)],
    );
    assert_eq!(
        engine
            .acceptance_criterion_effective_status_for_branch(
                workspace.initial_branch_id,
                fixture.criterion.acceptance_criterion_entity_id,
            )
            .expect("requirement status with cache"),
        AcceptanceCriterionEffectiveStatus::Verified
    );
}

#[test]
fn branch_unknown_resource_cache_still_blocks_done() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let fixture = create_required_criterion(&mut engine, &workspace);
    let resource_basis = create_resource_basis(&mut engine, "baseline");
    let verification = create_direct_resource_verification(
        &mut engine,
        &workspace,
        &fixture.criterion,
        resource_basis.clone(),
    );
    record_applicability(
        &mut engine,
        &workspace,
        &verification,
        vec![unavailable_stamp(&resource_basis)],
    );

    assert_eq!(
        engine
            .acceptance_criterion_effective_status_for_branch(
                workspace.initial_branch_id,
                fixture.criterion.acceptance_criterion_entity_id,
            )
            .expect("branch status with unknown cache"),
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
            .expect("done outcome"),
        )
        .expect_err("unknown cache should not satisfy mandatory AC");
    assert_eq!(blocked.code(), ErrorCode::TaskInvalid);
    assert!(blocked.to_string().contains("stale"));
}
