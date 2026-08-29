use rusqlite::{Connection, params};
use std::path::{Path, PathBuf};
use tempfile::TempDir;
use workvcs_core::{
    AcceptanceCriterionClassification, AcceptanceCriterionCreateCommit,
    AcceptanceCriterionCreateOptions, BranchId, CanonicalValue, CommitId, Engine, ErrorCategory,
    ErrorCode, ResourceCreateOptions, ResourceId, ResourceObservationCreateOptions,
    StoreInitOptions, TaskCreateOptions, VerificationCreateCommit, VerificationCreateOptions,
    VerificationResourceBasis, VerificationResult, VerificationTarget, WorkspaceInfo,
    WorkspaceInitOptions, canonical_bytes, content_object_digest,
};

struct CriterionFixture {
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
        StoreInitOptions::new("phase3y-verification-resource-basis-store").expect("store options"),
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

fn canonical_json(value: &CanonicalValue) -> String {
    String::from_utf8(canonical_bytes(value).expect("canonical bytes")).expect("canonical JSON")
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

fn create_required_criterion(engine: &mut Engine, workspace: &WorkspaceInfo) -> CriterionFixture {
    let task = engine
        .create_task(
            TaskCreateOptions::new(
                workspace.initial_branch_id,
                workspace.genesis_commit_id,
                "Implement verification resource basis",
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
                "The verification records resource basis.",
                AcceptanceCriterionClassification::Required,
            )
            .expect("criterion options"),
        )
        .expect("create acceptance criterion");
    CriterionFixture { criterion }
}

fn create_resource_basis(engine: &mut Engine, label: &str) -> VerificationResourceBasis {
    let resource = engine
        .create_resource(ResourceCreateOptions::new("git").expect("resource options"))
        .expect("create resource");
    let fingerprint = content_object_digest(format!("fingerprint:{label}").as_bytes());
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
        object(vec![("z", string("tail")), ("path", string("src/lib.rs"))]),
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

#[test]
fn verification_creation_records_resource_basis_in_state_basis_and_rows() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let fixture = create_required_criterion(&mut engine, &workspace);
    let resource_basis = create_resource_basis(&mut engine, "baseline");

    let verification = create_verification_with_basis(
        &mut engine,
        &workspace,
        &fixture.criterion,
        vec![resource_basis.clone()],
    );

    assert_eq!(
        verification.state.resource_basis,
        vec![resource_basis.clone()]
    );
    let snapshot = engine
        .verification_at(verification.commit_id, verification.verification_entity_id)
        .expect("verification snapshot");
    assert_eq!(snapshot.state.resource_basis, vec![resource_basis.clone()]);

    let connection = raw_connection(&path);
    let row = connection
        .query_row(
            "SELECT ordinal,
                    resource_id,
                    adapter_kind,
                    adapter_schema_version,
                    scope_kind,
                    scope_schema_version,
                    scope_payload_json,
                    baseline_observation_id,
                    baseline_fingerprint
             FROM verification_resource_basis
             WHERE verification_entity_id = ?1",
            params![&verification.verification_entity_id.raw_bytes()[..]],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, Vec<u8>>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, i64>(5)?,
                    row.get::<_, String>(6)?,
                    row.get::<_, Option<Vec<u8>>>(7)?,
                    row.get::<_, Vec<u8>>(8)?,
                ))
            },
        )
        .expect("resource basis row");
    assert_eq!(row.0, 0);
    assert_eq!(row.1, resource_basis.resource_id.raw_bytes().to_vec());
    assert_eq!(row.2, "git");
    assert_eq!(row.3, 1);
    assert_eq!(row.4, "path");
    assert_eq!(row.5, 1);
    assert_eq!(row.6, canonical_json(&resource_basis.scope_payload));
    assert_eq!(
        row.7.expect("baseline observation"),
        resource_basis
            .baseline_observation_id
            .expect("basis observation")
            .raw_bytes()
            .to_vec()
    );
    assert_eq!(
        row.8,
        resource_basis.baseline_fingerprint.as_bytes().to_vec()
    );

    let basis_json = connection
        .query_row(
            "SELECT basis_json
             FROM verification_basis
             WHERE verification_entity_id = ?1",
            params![&verification.verification_entity_id.raw_bytes()[..]],
            |row| row.get::<_, String>(0),
        )
        .expect("basis json");
    assert!(basis_json.contains("\"resource_basis\""));
    assert!(basis_json.contains(&resource_basis.baseline_fingerprint.to_hex()));
}

#[test]
fn resource_basis_scope_payload_order_is_canonicalized() {
    let resource_id = ResourceId::new_v7();
    let fingerprint = content_object_digest(b"scope order fingerprint");
    let left = VerificationResourceBasis::new(
        resource_id,
        "git",
        1,
        "path",
        1,
        object(vec![("z", string("tail")), ("path", string("src"))]),
        fingerprint,
    )
    .expect("left basis");
    let right = VerificationResourceBasis::new(
        resource_id,
        "git",
        1,
        "path",
        1,
        object(vec![("path", string("src")), ("z", string("tail"))]),
        fingerprint,
    )
    .expect("right basis");

    assert_eq!(left, right);
    assert_eq!(
        canonical_json(&left.scope_payload),
        "{\"path\":\"src\",\"z\":\"tail\"}"
    );
}

#[test]
fn verification_without_resource_basis_preserves_work_state_only_basis() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let fixture = create_required_criterion(&mut engine, &workspace);
    let verification =
        create_verification_with_basis(&mut engine, &workspace, &fixture.criterion, Vec::new());

    assert!(verification.state.resource_basis.is_empty());
    let snapshot = engine
        .verification_at(verification.commit_id, verification.verification_entity_id)
        .expect("verification snapshot");
    assert!(snapshot.state.resource_basis.is_empty());

    let connection = raw_connection(&path);
    assert_eq!(count_rows(&connection, "verification_resource_basis"), 0);
    let basis_json = connection
        .query_row(
            "SELECT basis_json
             FROM verification_basis
             WHERE verification_entity_id = ?1",
            params![&verification.verification_entity_id.raw_bytes()[..]],
            |row| row.get::<_, String>(0),
        )
        .expect("basis json");
    assert!(basis_json.contains("\"resource_basis\":[]"));
}

#[test]
fn missing_resource_basis_resource_is_rejected_before_commit() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let fixture = create_required_criterion(&mut engine, &workspace);
    let connection = raw_connection(&path);
    let before_head = branch_head(&connection, workspace.initial_branch_id);
    let missing_resource = ResourceId::new_v7();
    let basis = VerificationResourceBasis::new(
        missing_resource,
        "git",
        1,
        "path",
        1,
        object(vec![("path", string("missing"))]),
        content_object_digest(b"missing resource basis"),
    )
    .expect("basis with missing resource id");

    let error = engine
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
            .with_resource_basis(vec![basis])
            .expect("resource basis options"),
        )
        .expect_err("missing resource should be rejected");

    assert_eq!(error.code(), ErrorCode::ResourceNotFound);
    assert_eq!(error.category(), ErrorCategory::Resource);
    assert_eq!(
        branch_head(&connection, workspace.initial_branch_id),
        before_head
    );
    assert_eq!(count_rows(&connection, "verification_resource_basis"), 0);
}

#[test]
fn baseline_observation_must_match_resource_adapter_and_fingerprint() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let fixture = create_required_criterion(&mut engine, &workspace);
    let resource = engine
        .create_resource(ResourceCreateOptions::new("git").expect("resource options"))
        .expect("create resource");
    let other_resource = engine
        .create_resource(ResourceCreateOptions::new("git").expect("other resource options"))
        .expect("create other resource");
    let fingerprint = content_object_digest(b"observed fingerprint");
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

    let mismatched_resource = VerificationResourceBasis::new(
        other_resource.resource_id,
        "git",
        1,
        "path",
        1,
        object(vec![("path", string("src"))]),
        fingerprint,
    )
    .expect("mismatched resource basis")
    .with_baseline_observation_id(observation.observation_id)
    .expect("basis observation");
    let error = engine
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
            .with_resource_basis(vec![mismatched_resource])
            .expect("resource basis options"),
        )
        .expect_err("mismatched resource should be rejected");
    assert_eq!(error.code(), ErrorCode::TaskInvalid);

    let wrong_fingerprint = VerificationResourceBasis::new(
        resource.resource_id,
        "git",
        1,
        "path",
        1,
        object(vec![("path", string("src"))]),
        content_object_digest(b"wrong fingerprint"),
    )
    .expect("wrong fingerprint basis")
    .with_baseline_observation_id(observation.observation_id)
    .expect("basis observation");
    let error = engine
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
            .with_resource_basis(vec![wrong_fingerprint])
            .expect("resource basis options"),
        )
        .expect_err("wrong fingerprint should be rejected");
    assert_eq!(error.code(), ErrorCode::TaskInvalid);

    let wrong_adapter_schema_version = VerificationResourceBasis::new(
        resource.resource_id,
        "git",
        2,
        "path",
        1,
        object(vec![("path", string("src"))]),
        fingerprint,
    )
    .expect("wrong adapter schema version basis")
    .with_baseline_observation_id(observation.observation_id)
    .expect("basis observation");
    let error = engine
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
            .with_resource_basis(vec![wrong_adapter_schema_version])
            .expect("resource basis options"),
        )
        .expect_err("wrong adapter schema version should be rejected");
    assert_eq!(error.code(), ErrorCode::TaskInvalid);

    let wrong_adapter = VerificationResourceBasis::new(
        resource.resource_id,
        "filesystem",
        1,
        "path",
        1,
        object(vec![("path", string("src"))]),
        fingerprint,
    )
    .expect("wrong adapter basis")
    .with_baseline_observation_id(observation.observation_id)
    .expect("basis observation");
    let error = engine
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
            .with_resource_basis(vec![wrong_adapter])
            .expect("resource basis options"),
        )
        .expect_err("wrong adapter should be rejected");
    assert_eq!(error.code(), ErrorCode::TaskInvalid);
}

#[test]
fn resource_basis_validation_rejects_invalid_scope_contract() {
    let non_object_scope = VerificationResourceBasis::new(
        ResourceId::new_v7(),
        "git",
        1,
        "path",
        1,
        CanonicalValue::String("not an object".to_owned()),
        content_object_digest(b"fingerprint"),
    )
    .expect_err("scope payload must be object");
    assert_eq!(non_object_scope.code(), ErrorCode::TaskInvalid);

    let invalid_adapter_version = VerificationResourceBasis::new(
        ResourceId::new_v7(),
        "git",
        0,
        "path",
        1,
        object(Vec::new()),
        content_object_digest(b"fingerprint"),
    )
    .expect_err("adapter schema version must be positive");
    assert_eq!(invalid_adapter_version.code(), ErrorCode::TaskInvalid);

    let invalid_scope_kind = VerificationResourceBasis::new(
        ResourceId::new_v7(),
        "git",
        1,
        " path ",
        1,
        object(Vec::new()),
        content_object_digest(b"fingerprint"),
    )
    .expect_err("scope kind must not have surrounding whitespace");
    assert_eq!(invalid_scope_kind.code(), ErrorCode::TaskInvalid);
}

#[test]
fn verification_readback_accepts_legacy_two_field_basis_json_when_resource_basis_is_empty() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let fixture = create_required_criterion(&mut engine, &workspace);
    let verification =
        create_verification_with_basis(&mut engine, &workspace, &fixture.criterion, Vec::new());
    let dependency = verification
        .state
        .semantic_dependencies
        .first()
        .expect("semantic dependency");
    let legacy_basis_json = format!(
        "{{\"semantic_dependencies\":[{{\"entity_id\":\"{}\",\"entity_version_id\":\"{}\"}}],\"verified_at_commit_id\":\"{}\"}}",
        dependency.entity_id,
        dependency.entity_version_id,
        verification.state.verified_at_commit_id
    );
    let connection = raw_connection(&path);
    connection
        .execute(
            "UPDATE verification_basis
             SET basis_json = ?1
             WHERE verification_entity_id = ?2",
            params![
                legacy_basis_json,
                &verification.verification_entity_id.raw_bytes()[..]
            ],
        )
        .expect("write legacy basis json");

    let snapshot = engine
        .verification_at(verification.commit_id, verification.verification_entity_id)
        .expect("legacy two-field basis json remains readable");
    assert!(snapshot.state.resource_basis.is_empty());
}

#[test]
fn verification_readback_rejects_corrupted_resource_basis_rows() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let fixture = create_required_criterion(&mut engine, &workspace);
    let resource_basis = create_resource_basis(&mut engine, "corrupt");
    let verification = create_verification_with_basis(
        &mut engine,
        &workspace,
        &fixture.criterion,
        vec![resource_basis],
    );
    let connection = raw_connection(&path);
    connection
        .execute(
            "UPDATE verification_resource_basis
             SET scope_payload_json = '{\"z\":1,\"a\":2}'
             WHERE verification_entity_id = ?1",
            params![&verification.verification_entity_id.raw_bytes()[..]],
        )
        .expect("corrupt resource basis scope payload");

    let error = engine
        .verification_at(verification.commit_id, verification.verification_entity_id)
        .expect_err("noncanonical resource basis scope payload should be rejected");
    assert_eq!(error.code(), ErrorCode::TaskInvalid);
}
