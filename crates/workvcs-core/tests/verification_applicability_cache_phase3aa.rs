use rusqlite::{Connection, params};
use std::path::{Path, PathBuf};
use tempfile::TempDir;
use workvcs_core::{
    AcceptanceCriterionClassification, AcceptanceCriterionCreateCommit,
    AcceptanceCriterionCreateOptions, ApplicabilityResourceObservationStatus,
    ApplicabilityResourceStampInput, BranchId, CanonicalValue, Digest, Engine, EntityId, ErrorCode,
    ResourceCreateOptions, ResourceObservationCreateOptions, StoreInitOptions, TaskCreateOptions,
    VerificationApplicability, VerificationApplicabilityRecordOptions, VerificationCreateCommit,
    VerificationCreateOptions, VerificationResourceBasis, VerificationResult, VerificationTarget,
    WorkspaceInfo, WorkspaceInitOptions, content_object_digest,
};

fn store_path() -> (TempDir, PathBuf) {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let path = tempdir.path().join("workvcs.sqlite");
    (tempdir, path)
}

fn init_engine(path: &Path) -> Engine {
    Engine::init(
        path,
        StoreInitOptions::new("phase3aa-applicability-cache-stamp-foundation-store")
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

fn create_required_criterion(
    engine: &mut Engine,
    workspace: &WorkspaceInfo,
) -> AcceptanceCriterionCreateCommit {
    let task = engine
        .create_task(
            TaskCreateOptions::new(
                workspace.initial_branch_id,
                workspace.genesis_commit_id,
                "Implement applicability cache foundation",
            )
            .expect("task options"),
        )
        .expect("create task");
    engine
        .create_acceptance_criterion(
            AcceptanceCriterionCreateOptions::new(
                workspace.initial_branch_id,
                task.commit_id,
                task.task_entity_id,
                task.task_entity_version_id,
                "AC-1",
                "Verification applicability can be cached as derived state.",
                AcceptanceCriterionClassification::Required,
            )
            .expect("criterion options"),
        )
        .expect("create acceptance criterion")
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

fn create_verification_with_basis(
    engine: &mut Engine,
    workspace: &WorkspaceInfo,
    criterion: &AcceptanceCriterionCreateCommit,
    resource_basis: Vec<VerificationResourceBasis>,
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
            .with_resource_basis(resource_basis)
            .expect("resource basis options"),
        )
        .expect("create verification")
}

fn create_work_state_only_verification(
    engine: &mut Engine,
    workspace: &WorkspaceInfo,
    criterion: &AcceptanceCriterionCreateCommit,
) -> VerificationCreateCommit {
    engine
        .create_verification(
            VerificationCreateOptions::new(
                workspace.initial_branch_id,
                criterion.commit_id,
                VerificationTarget::AcceptanceCriterion(criterion.acceptance_criterion_entity_id),
                VerificationResult::Passed,
            )
            .expect("verification options"),
        )
        .expect("create verification")
}

fn observed_stamp(
    resource_basis: &VerificationResourceBasis,
    observed_fingerprint: Digest,
) -> ApplicabilityResourceStampInput {
    ApplicabilityResourceStampInput::observed(
        0,
        resource_basis.adapter_kind.clone(),
        resource_basis.adapter_schema_version,
        resource_basis.scope_schema_version,
        observed_fingerprint,
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

fn error_stamp(resource_basis: &VerificationResourceBasis) -> ApplicabilityResourceStampInput {
    ApplicabilityResourceStampInput::error(
        0,
        resource_basis.adapter_kind.clone(),
        resource_basis.adapter_schema_version,
        resource_basis.scope_schema_version,
    )
    .expect("error stamp")
}

fn record_options(
    workspace: &WorkspaceInfo,
    verification: &VerificationCreateCommit,
    stamps: Vec<ApplicabilityResourceStampInput>,
) -> VerificationApplicabilityRecordOptions {
    VerificationApplicabilityRecordOptions::new(
        workspace.initial_branch_id,
        verification.verification_entity_id,
        verification.commit_id,
    )
    .expect("record options")
    .with_resource_stamps(stamps)
    .expect("record stamps")
    .with_detail(object(vec![("source", string("phase3aa"))]))
    .expect("record detail")
}

fn corrupt_stamp_to_unavailable(
    connection: &Connection,
    branch_id: BranchId,
    verification_entity_id: EntityId,
) {
    connection
        .execute(
            "UPDATE applicability_resource_stamp
             SET observation_status = 'unavailable',
                 observed_fingerprint = NULL,
                 observation_id = NULL
             WHERE branch_id = ?1
               AND verification_entity_id = ?2
               AND resource_basis_ordinal = 0",
            params![
                &branch_id.raw_bytes()[..],
                &verification_entity_id.raw_bytes()[..]
            ],
        )
        .expect("corrupt stamp");
}

#[test]
fn records_and_reads_applicable_resource_stamp_cache_without_workstate_commit() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let criterion = create_required_criterion(&mut engine, &workspace);
    let resource_basis = create_resource_basis(&mut engine, "baseline");
    let verification = create_verification_with_basis(
        &mut engine,
        &workspace,
        &criterion,
        vec![resource_basis.clone()],
    );
    let connection = raw_connection(&path);
    let before_workstate_commits = count_rows(&connection, "workstate_commit");

    assert!(
        engine
            .verification_applicability_cache(
                workspace.initial_branch_id,
                verification.verification_entity_id,
            )
            .expect("missing cache")
            .is_none()
    );

    let baseline_observation_id = resource_basis
        .baseline_observation_id
        .expect("baseline observation");
    let stamp = observed_stamp(&resource_basis, resource_basis.baseline_fingerprint)
        .with_observation_id(baseline_observation_id)
        .expect("stamp observation");
    let snapshot = engine
        .record_verification_applicability(record_options(&workspace, &verification, vec![stamp]))
        .expect("record applicability");

    assert_eq!(snapshot.branch_id, workspace.initial_branch_id);
    assert_eq!(
        snapshot.verification_entity_id,
        verification.verification_entity_id
    );
    assert_eq!(snapshot.evaluated_commit_id, verification.commit_id);
    assert_eq!(
        snapshot.applicability,
        VerificationApplicability::Applicable
    );
    assert_eq!(snapshot.reason_code, "all_basis_applicable");
    assert_eq!(
        snapshot.detail,
        object(vec![("source", string("phase3aa"))])
    );
    assert_eq!(snapshot.resource_stamps.len(), 1);
    assert_eq!(
        snapshot.resource_stamps[0].observation_status,
        ApplicabilityResourceObservationStatus::Observed
    );
    assert_eq!(
        snapshot.resource_stamps[0].observed_fingerprint,
        Some(resource_basis.baseline_fingerprint)
    );
    assert_eq!(
        snapshot.resource_stamps[0].observation_id,
        Some(baseline_observation_id)
    );

    let readback = engine
        .verification_applicability_cache(
            workspace.initial_branch_id,
            verification.verification_entity_id,
        )
        .expect("read cache")
        .expect("cache");
    assert_eq!(readback, snapshot);
    assert_eq!(
        count_rows(&connection, "verification_applicability_cache"),
        1
    );
    assert_eq!(count_rows(&connection, "applicability_resource_stamp"), 1);
    assert_eq!(
        count_rows(&connection, "workstate_commit"),
        before_workstate_commits
    );
}

#[test]
fn records_stale_and_unknown_resource_stamp_results_by_replacing_cache() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let criterion = create_required_criterion(&mut engine, &workspace);
    let resource_basis = create_resource_basis(&mut engine, "baseline");
    let verification = create_verification_with_basis(
        &mut engine,
        &workspace,
        &criterion,
        vec![resource_basis.clone()],
    );
    let connection = raw_connection(&path);

    let drifted = content_object_digest(b"drifted external resource");
    let stale = engine
        .record_verification_applicability(record_options(
            &workspace,
            &verification,
            vec![observed_stamp(&resource_basis, drifted)],
        ))
        .expect("record stale applicability");
    assert_eq!(stale.applicability, VerificationApplicability::Stale);
    assert_eq!(stale.reason_code, "resource_drift");
    assert_eq!(
        count_rows(&connection, "verification_applicability_cache"),
        1
    );
    assert_eq!(count_rows(&connection, "applicability_resource_stamp"), 1);

    let unavailable = engine
        .record_verification_applicability(record_options(
            &workspace,
            &verification,
            vec![unavailable_stamp(&resource_basis)],
        ))
        .expect("record unavailable applicability");
    assert_eq!(
        unavailable.applicability,
        VerificationApplicability::Unknown
    );
    assert_eq!(unavailable.reason_code, "resource_unavailable");
    assert_eq!(
        count_rows(&connection, "verification_applicability_cache"),
        1
    );
    assert_eq!(count_rows(&connection, "applicability_resource_stamp"), 1);

    let errored = engine
        .record_verification_applicability(record_options(
            &workspace,
            &verification,
            vec![error_stamp(&resource_basis)],
        ))
        .expect("record error applicability");
    assert_eq!(errored.applicability, VerificationApplicability::Unknown);
    assert_eq!(errored.reason_code, "resource_error");
    assert_eq!(
        count_rows(&connection, "verification_applicability_cache"),
        1
    );
    assert_eq!(count_rows(&connection, "applicability_resource_stamp"), 1);
}

#[test]
fn rejects_invalid_stamps_without_partial_cache() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let criterion = create_required_criterion(&mut engine, &workspace);
    let resource_basis = create_resource_basis(&mut engine, "baseline");
    let verification = create_verification_with_basis(
        &mut engine,
        &workspace,
        &criterion,
        vec![resource_basis.clone()],
    );
    let connection = raw_connection(&path);

    let incomplete = engine
        .record_verification_applicability(record_options(&workspace, &verification, Vec::new()))
        .expect_err("missing stamp should be rejected");
    assert_eq!(incomplete.code(), ErrorCode::TaskInvalid);

    let wrong_adapter = ApplicabilityResourceStampInput::observed(
        0,
        "filesystem",
        resource_basis.adapter_schema_version,
        resource_basis.scope_schema_version,
        resource_basis.baseline_fingerprint,
    )
    .expect("wrong adapter stamp");
    let mismatch = engine
        .record_verification_applicability(record_options(
            &workspace,
            &verification,
            vec![wrong_adapter],
        ))
        .expect_err("adapter mismatch should be rejected");
    assert_eq!(mismatch.code(), ErrorCode::TaskInvalid);

    let other_resource = engine
        .create_resource(ResourceCreateOptions::new("git").expect("resource options"))
        .expect("create other resource");
    let other_fingerprint = content_object_digest(b"other observation");
    let other_observation = engine
        .record_resource_observation(
            ResourceObservationCreateOptions::new(
                other_resource.resource_id,
                "git",
                1,
                other_fingerprint,
                object(vec![("label", string("other"))]),
            )
            .expect("other observation options"),
        )
        .expect("record other observation");
    let wrong_observation = observed_stamp(&resource_basis, other_fingerprint)
        .with_observation_id(other_observation.observation_id)
        .expect("wrong observation stamp");
    let observation_mismatch = engine
        .record_verification_applicability(record_options(
            &workspace,
            &verification,
            vec![wrong_observation],
        ))
        .expect_err("observation mismatch should be rejected");
    assert_eq!(observation_mismatch.code(), ErrorCode::TaskInvalid);

    assert_eq!(
        count_rows(&connection, "verification_applicability_cache"),
        0
    );
    assert_eq!(count_rows(&connection, "applicability_resource_stamp"), 0);
}

#[test]
fn rejects_stale_branch_head_without_cache() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let criterion = create_required_criterion(&mut engine, &workspace);
    let resource_basis = create_resource_basis(&mut engine, "baseline");
    let verification = create_verification_with_basis(
        &mut engine,
        &workspace,
        &criterion,
        vec![resource_basis.clone()],
    );
    let connection = raw_connection(&path);

    engine
        .create_task(
            TaskCreateOptions::new(
                workspace.initial_branch_id,
                verification.commit_id,
                "Advance branch past the evaluated verification",
            )
            .expect("advance task options"),
        )
        .expect("advance branch");

    let stale_head = engine
        .record_verification_applicability(record_options(
            &workspace,
            &verification,
            vec![observed_stamp(
                &resource_basis,
                resource_basis.baseline_fingerprint,
            )],
        ))
        .expect_err("stale branch head should be rejected");
    assert_eq!(stale_head.code(), ErrorCode::BranchHeadConflict);
    assert_eq!(
        count_rows(&connection, "verification_applicability_cache"),
        0
    );
    assert_eq!(count_rows(&connection, "applicability_resource_stamp"), 0);
}

#[test]
fn readback_rejects_cache_rows_that_no_longer_match_stamps() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let criterion = create_required_criterion(&mut engine, &workspace);
    let resource_basis = create_resource_basis(&mut engine, "baseline");
    let verification = create_verification_with_basis(
        &mut engine,
        &workspace,
        &criterion,
        vec![resource_basis.clone()],
    );
    let connection = raw_connection(&path);
    engine
        .record_verification_applicability(record_options(
            &workspace,
            &verification,
            vec![observed_stamp(
                &resource_basis,
                resource_basis.baseline_fingerprint,
            )],
        ))
        .expect("record applicability");

    corrupt_stamp_to_unavailable(
        &connection,
        workspace.initial_branch_id,
        verification.verification_entity_id,
    );

    let corrupt = engine
        .verification_applicability_cache(
            workspace.initial_branch_id,
            verification.verification_entity_id,
        )
        .expect_err("corrupt cache should fail readback");
    assert_eq!(corrupt.code(), ErrorCode::TaskInvalid);
    assert!(corrupt.to_string().contains("does not match basis stamps"));
}

#[test]
fn work_state_only_cache_accepts_no_stamps_and_rejects_extra_stamp() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let criterion = create_required_criterion(&mut engine, &workspace);
    let verification = create_work_state_only_verification(&mut engine, &workspace, &criterion);
    let connection = raw_connection(&path);
    let before_workstate_commits = count_rows(&connection, "workstate_commit");

    let snapshot = engine
        .record_verification_applicability(record_options(&workspace, &verification, Vec::new()))
        .expect("record work-state-only applicability");
    assert_eq!(
        snapshot.applicability,
        VerificationApplicability::Applicable
    );
    assert_eq!(snapshot.reason_code, "work_state_applicable");
    assert!(snapshot.resource_stamps.is_empty());
    assert_eq!(
        count_rows(&connection, "verification_applicability_cache"),
        1
    );
    assert_eq!(count_rows(&connection, "applicability_resource_stamp"), 0);
    assert_eq!(
        count_rows(&connection, "workstate_commit"),
        before_workstate_commits
    );

    let extra_stamp = ApplicabilityResourceStampInput::observed(
        0,
        "git",
        1,
        1,
        content_object_digest(b"unneeded stamp"),
    )
    .expect("extra stamp");
    let rejected = engine
        .record_verification_applicability(record_options(
            &workspace,
            &verification,
            vec![extra_stamp],
        ))
        .expect_err("extra stamp should be rejected");
    assert_eq!(rejected.code(), ErrorCode::TaskInvalid);
    assert_eq!(
        engine
            .verification_applicability_cache(
                workspace.initial_branch_id,
                verification.verification_entity_id,
            )
            .expect("read existing cache")
            .expect("cache"),
        snapshot
    );
}
