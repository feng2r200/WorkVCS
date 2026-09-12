use rusqlite::Connection;
use std::path::{Path, PathBuf};
use tempfile::TempDir;
use workvcs_core::{
    AcceptanceCriterionClassification, AcceptanceCriterionCreateCommit,
    AcceptanceCriterionCreateOptions, AcceptanceCriterionEffectiveStatus,
    ApplicabilityResourceObservationStatus, CanonicalValue, Engine, ErrorCode,
    EvidenceContentInput, EvidenceCreateOptions, ResourceCreateOptions,
    ResourceObservationCreateOptions, ResourceObservationDetailInput, StoreInitOptions,
    TaskCreateCommit, TaskCreateOptions, TaskStatus, TaskTransitionOptions,
    VerificationApplicability, VerificationApplicabilityRefreshOptions, VerificationCreateOptions,
    VerificationResourceBasis, VerificationResult, VerificationTarget, VerifyOptions,
    VerifyResourceObservationInput, WorkspaceInfo, WorkspaceInitOptions, content_object_digest,
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
        StoreInitOptions::new("phase4ky-verify-wrapper-store").expect("store options"),
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

fn metadata(label: &str) -> CanonicalValue {
    object(vec![("label", string(label))])
}

fn create_required_criterion(engine: &mut Engine, workspace: &WorkspaceInfo) -> CriterionFixture {
    let task = engine
        .create_task(
            TaskCreateOptions::new(
                workspace.initial_branch_id,
                workspace.genesis_commit_id,
                "Dogfood deterministic verification wrapper",
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
                "The verification wrapper records evidence and current resource applicability.",
                AcceptanceCriterionClassification::Required,
            )
            .expect("criterion options"),
        )
        .expect("create acceptance criterion");
    CriterionFixture { task, criterion }
}

fn evidence_options(label: &str) -> EvidenceCreateOptions {
    EvidenceCreateOptions::new("command_output", metadata(label))
        .expect("evidence options")
        .with_contents(vec![
            EvidenceContentInput::from_raw_bytes("stdout", format!("{label}: ok").as_bytes())
                .expect("evidence content"),
        ])
        .expect("evidence contents")
}

#[test]
fn verify_wrapper_records_resource_observation_verification_and_applicable_cache() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let fixture = create_required_criterion(&mut engine, &workspace);
    let resource = engine
        .create_resource(ResourceCreateOptions::new("filesystem").expect("resource options"))
        .expect("create resource");
    let fingerprint = content_object_digest(b"src/lib.rs verified content");
    let observation = ResourceObservationCreateOptions::new(
        resource.resource_id,
        "filesystem",
        1,
        fingerprint,
        metadata("current source snapshot"),
    )
    .expect("observation options")
    .with_detail_content(
        ResourceObservationDetailInput::from_raw_bytes(b"src/lib.rs digest input")
            .expect("detail content"),
    );
    let resource_observation = VerifyResourceObservationInput::new(
        observation,
        "path",
        1,
        object(vec![("path", string("src/lib.rs"))]),
    )
    .expect("verify resource observation");

    let result = engine
        .verify(
            VerifyOptions::new(
                workspace.initial_branch_id,
                fixture.criterion.commit_id,
                VerificationTarget::AcceptanceCriterion(
                    fixture.criterion.acceptance_criterion_entity_id,
                ),
                VerificationResult::Passed,
                evidence_options("cargo test"),
            )
            .expect("verify options")
            .with_method(metadata("cargo test -p workvcs-core"))
            .expect("verify method")
            .with_resource_observation(resource_observation)
            .with_cache_detail(metadata("phase4ky wrapper cache"))
            .expect("cache detail"),
        )
        .expect("verify wrapper");

    let observation = result
        .resource_observation
        .as_ref()
        .expect("resource observation");
    let cache = result
        .applicability_cache
        .as_ref()
        .expect("applicability cache");
    assert_eq!(result.evidence.evidence_kind, "command_output");
    assert_eq!(result.evidence.contents.len(), 1);
    assert_eq!(result.evidence.contents[0].storage_locations.len(), 1);
    assert_eq!(
        engine
            .read_evidence_content(result.evidence.evidence_id, 0)
            .expect("read verification evidence content")
            .raw_bytes,
        b"cargo test: ok"
    );
    assert_eq!(
        engine
            .validate_local_content_storage()
            .expect("validate local evidence content"),
        1
    );
    assert_eq!(observation.resource_id, resource.resource_id);
    assert_eq!(observation.state.fingerprint, fingerprint);
    assert_eq!(
        result.verification.previous_head_commit_id,
        fixture.criterion.commit_id
    );
    assert_eq!(result.verification.state.evidence.len(), 1);
    assert_eq!(result.verification.evidenced_by_relations.len(), 1);
    assert_eq!(result.verification.state.resource_basis.len(), 1);
    assert_eq!(cache.evaluated_commit_id, result.verification.commit_id);
    assert_eq!(cache.applicability, VerificationApplicability::Applicable);
    assert_eq!(cache.reason_code, "all_basis_applicable");
    assert_eq!(cache.resource_stamps.len(), 1);
    assert_eq!(
        cache.resource_stamps[0].observation_status,
        ApplicabilityResourceObservationStatus::Observed
    );
    assert_eq!(
        cache.resource_stamps[0].observed_fingerprint,
        Some(fingerprint)
    );
    assert_eq!(
        cache.resource_stamps[0].observation_id,
        Some(observation.observation_id)
    );

    assert_eq!(
        engine
            .acceptance_criterion_effective_status_for_branch(
                workspace.initial_branch_id,
                fixture.criterion.acceptance_criterion_entity_id,
            )
            .expect("branch AC status"),
        AcceptanceCriterionEffectiveStatus::Verified
    );
    engine
        .transition_task(
            TaskTransitionOptions::new(
                workspace.initial_branch_id,
                result.verification.commit_id,
                fixture.task.task_entity_id,
                fixture.criterion.task_entity_version_id,
                TaskStatus::Done,
            )
            .expect("done options")
            .with_outcome("verified through wrapper")
            .expect("done outcome"),
        )
        .expect("verified wrapper satisfies required AC");

    let connection = raw_connection(&path);
    assert_eq!(count_rows(&connection, "evidence"), 1);
    assert_eq!(count_rows(&connection, "evidence_content"), 1);
    assert_eq!(count_rows(&connection, "resource_observation"), 1);
    assert_eq!(
        count_rows(&connection, "verification_applicability_cache"),
        1
    );
    assert_eq!(count_rows(&connection, "applicability_resource_stamp"), 1);
}

#[test]
fn resource_backed_verification_cache_refresh_recovers_after_later_branch_head() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let fixture = create_required_criterion(&mut engine, &workspace);
    let resource = engine
        .create_resource(ResourceCreateOptions::new("filesystem").expect("resource options"))
        .expect("create resource");
    let fingerprint = content_object_digest(b"src/lib.rs verified content");
    let observation = ResourceObservationCreateOptions::new(
        resource.resource_id,
        "filesystem",
        1,
        fingerprint,
        metadata("current source snapshot"),
    )
    .expect("observation options");
    let resource_observation = VerifyResourceObservationInput::new(
        observation,
        "path",
        1,
        object(vec![("path", string("src/lib.rs"))]),
    )
    .expect("verify resource observation");

    let result = engine
        .verify(
            VerifyOptions::new(
                workspace.initial_branch_id,
                fixture.criterion.commit_id,
                VerificationTarget::AcceptanceCriterion(
                    fixture.criterion.acceptance_criterion_entity_id,
                ),
                VerificationResult::Passed,
                evidence_options("cargo test"),
            )
            .expect("verify options")
            .with_resource_observation(resource_observation),
        )
        .expect("verify wrapper");
    let original_cache = result.applicability_cache.as_ref().expect("initial cache");
    let observation_id = result
        .resource_observation
        .as_ref()
        .expect("resource observation")
        .observation_id;
    assert_eq!(
        original_cache.evaluated_commit_id,
        result.verification.commit_id
    );
    assert_eq!(
        original_cache.applicability,
        VerificationApplicability::Applicable
    );

    let advanced = engine
        .transition_task(
            TaskTransitionOptions::new(
                workspace.initial_branch_id,
                result.verification.commit_id,
                fixture.task.task_entity_id,
                fixture.criterion.task_entity_version_id,
                TaskStatus::Done,
            )
            .expect("done options")
            .with_outcome("verified before later head advance")
            .expect("done outcome"),
        )
        .expect("advance branch");
    assert_eq!(
        engine
            .acceptance_criterion_effective_status_for_branch(
                workspace.initial_branch_id,
                fixture.criterion.acceptance_criterion_entity_id,
            )
            .expect("branch AC status after advance"),
        AcceptanceCriterionEffectiveStatus::Stale
    );
    let mismatched_expected_head = engine
        .refresh_verification_applicability(
            VerificationApplicabilityRefreshOptions::new(
                workspace.initial_branch_id,
                result.verification.verification_entity_id,
            )
            .expect("refresh options")
            .with_expected_evaluated_commit_id(result.verification.commit_id),
        )
        .expect_err("mismatched expected head should fail before refresh");
    assert_eq!(
        mismatched_expected_head.code(),
        ErrorCode::BranchHeadConflict
    );
    let unchanged_cache = engine
        .verification_applicability_cache(
            workspace.initial_branch_id,
            result.verification.verification_entity_id,
        )
        .expect("read cache after failed refresh")
        .expect("old cache");
    assert_eq!(
        unchanged_cache.evaluated_commit_id,
        result.verification.commit_id
    );
    assert_eq!(
        engine
            .acceptance_criterion_effective_status_for_branch(
                workspace.initial_branch_id,
                fixture.criterion.acceptance_criterion_entity_id,
            )
            .expect("branch AC status after failed refresh"),
        AcceptanceCriterionEffectiveStatus::Stale
    );

    let refreshed = engine
        .refresh_verification_applicability(
            VerificationApplicabilityRefreshOptions::new(
                workspace.initial_branch_id,
                result.verification.verification_entity_id,
            )
            .expect("refresh options")
            .with_detail(metadata("phase4kz refresh"))
            .expect("refresh detail"),
        )
        .expect("refresh applicability");
    assert_eq!(refreshed.evaluated_commit_id, advanced.commit_id);
    assert_eq!(
        refreshed.applicability,
        VerificationApplicability::Applicable
    );
    assert_eq!(refreshed.reason_code, "all_basis_applicable");
    assert_eq!(refreshed.resource_stamps.len(), 1);
    assert_eq!(
        refreshed.resource_stamps[0].observed_fingerprint,
        Some(fingerprint)
    );
    assert_eq!(
        refreshed.resource_stamps[0].observation_id,
        Some(observation_id)
    );
    assert_eq!(
        engine
            .acceptance_criterion_effective_status_for_branch(
                workspace.initial_branch_id,
                fixture.criterion.acceptance_criterion_entity_id,
            )
            .expect("branch AC status after refresh"),
        AcceptanceCriterionEffectiveStatus::Verified
    );

    let connection = raw_connection(&path);
    assert_eq!(
        count_rows(&connection, "verification_applicability_cache"),
        1
    );
    assert_eq!(count_rows(&connection, "applicability_resource_stamp"), 1);
}

#[test]
fn cache_refresh_rejects_resource_basis_without_baseline_observation() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let fixture = create_required_criterion(&mut engine, &workspace);
    let resource = engine
        .create_resource(ResourceCreateOptions::new("filesystem").expect("resource options"))
        .expect("create resource");
    let fingerprint = content_object_digest(b"manual resource baseline");
    let resource_basis = VerificationResourceBasis::new(
        resource.resource_id,
        "filesystem",
        1,
        "path",
        1,
        object(vec![("path", string("src/lib.rs"))]),
        fingerprint,
    )
    .expect("resource basis without baseline observation");
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
            .with_method(metadata("manual resource assertion"))
            .expect("verification method")
            .with_resource_basis(vec![resource_basis])
            .expect("resource basis"),
        )
        .expect("create verification");
    let connection = raw_connection(&path);
    let before_caches = count_rows(&connection, "verification_applicability_cache");
    let before_stamps = count_rows(&connection, "applicability_resource_stamp");

    let error = engine
        .refresh_verification_applicability(
            VerificationApplicabilityRefreshOptions::new(
                workspace.initial_branch_id,
                verification.verification_entity_id,
            )
            .expect("refresh options"),
        )
        .expect_err("missing baseline observation should reject refresh");

    assert_eq!(error.code(), ErrorCode::TaskInvalid);
    assert!(
        error
            .to_string()
            .contains("cannot be refreshed from baseline because it has no baseline observation")
    );
    assert_eq!(
        count_rows(&connection, "verification_applicability_cache"),
        before_caches
    );
    assert_eq!(
        count_rows(&connection, "applicability_resource_stamp"),
        before_stamps
    );
}

#[test]
fn verify_wrapper_without_resource_keeps_work_state_only_verification_path() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let fixture = create_required_criterion(&mut engine, &workspace);

    let result = engine
        .verify(
            VerifyOptions::new(
                workspace.initial_branch_id,
                fixture.criterion.commit_id,
                VerificationTarget::AcceptanceCriterion(
                    fixture.criterion.acceptance_criterion_entity_id,
                ),
                VerificationResult::Passed,
                evidence_options("manual review"),
            )
            .expect("verify options"),
        )
        .expect("verify wrapper");

    assert!(result.resource_observation.is_none());
    assert!(result.applicability_cache.is_none());
    assert_eq!(result.verification.state.evidence.len(), 1);
    assert!(result.verification.state.resource_basis.is_empty());
    assert_eq!(
        engine
            .acceptance_criterion_effective_status(
                result.verification.commit_id,
                fixture.criterion.acceptance_criterion_entity_id,
            )
            .expect("AC status"),
        AcceptanceCriterionEffectiveStatus::Verified
    );

    let connection = raw_connection(&path);
    assert_eq!(count_rows(&connection, "evidence"), 1);
    assert_eq!(count_rows(&connection, "resource_observation"), 0);
    assert_eq!(
        count_rows(&connection, "verification_applicability_cache"),
        0
    );
}

#[test]
fn verify_wrapper_rejects_orphan_cache_detail_before_partial_writes() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let fixture = create_required_criterion(&mut engine, &workspace);
    let connection = raw_connection(&path);
    let before_evidence = count_rows(&connection, "evidence");
    let before_observations = count_rows(&connection, "resource_observation");
    let before_verifications = count_rows(&connection, "entity");
    let before_caches = count_rows(&connection, "verification_applicability_cache");

    let error = engine
        .verify(
            VerifyOptions::new(
                workspace.initial_branch_id,
                fixture.criterion.commit_id,
                VerificationTarget::AcceptanceCriterion(
                    fixture.criterion.acceptance_criterion_entity_id,
                ),
                VerificationResult::Passed,
                evidence_options("orphan cache detail"),
            )
            .expect("verify options")
            .with_cache_detail(metadata("must not be silently discarded"))
            .expect("cache detail"),
        )
        .expect_err("cache detail without resource observation should fail");

    assert_eq!(error.code(), ErrorCode::TaskInvalid);
    assert_eq!(count_rows(&connection, "evidence"), before_evidence);
    assert_eq!(
        count_rows(&connection, "resource_observation"),
        before_observations
    );
    assert_eq!(count_rows(&connection, "entity"), before_verifications);
    assert_eq!(
        count_rows(&connection, "verification_applicability_cache"),
        before_caches
    );
}

#[test]
fn verify_wrapper_rejects_stale_head_before_partial_evidence_or_observation_writes() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let fixture = create_required_criterion(&mut engine, &workspace);
    let resource = engine
        .create_resource(ResourceCreateOptions::new("filesystem").expect("resource options"))
        .expect("create resource");
    let connection = raw_connection(&path);
    let before_evidence = count_rows(&connection, "evidence");
    let before_observations = count_rows(&connection, "resource_observation");
    let before_verifications = count_rows(&connection, "entity");
    let before_caches = count_rows(&connection, "verification_applicability_cache");
    let fingerprint = content_object_digest(b"stale head resource state");
    let observation = ResourceObservationCreateOptions::new(
        resource.resource_id,
        "filesystem",
        1,
        fingerprint,
        metadata("stale head snapshot"),
    )
    .expect("observation options");
    let resource_observation = VerifyResourceObservationInput::new(
        observation,
        "path",
        1,
        object(vec![("path", string("src/lib.rs"))]),
    )
    .expect("verify resource observation");

    let error = engine
        .verify(
            VerifyOptions::new(
                workspace.initial_branch_id,
                workspace.genesis_commit_id,
                VerificationTarget::AcceptanceCriterion(
                    fixture.criterion.acceptance_criterion_entity_id,
                ),
                VerificationResult::Passed,
                evidence_options("should not persist"),
            )
            .expect("verify options")
            .with_resource_observation(resource_observation),
        )
        .expect_err("stale head should fail before writes");

    assert_eq!(error.code(), ErrorCode::BranchHeadConflict);
    assert_eq!(count_rows(&connection, "evidence"), before_evidence);
    assert_eq!(
        count_rows(&connection, "resource_observation"),
        before_observations
    );
    assert_eq!(count_rows(&connection, "entity"), before_verifications);
    assert_eq!(
        count_rows(&connection, "verification_applicability_cache"),
        before_caches
    );
}
