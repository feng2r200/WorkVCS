use rusqlite::{Connection, params};
use std::path::{Path, PathBuf};
use tempfile::TempDir;
use workvcs_core::{
    AcceptanceCriterionClassification, AcceptanceCriterionCreateCommit,
    AcceptanceCriterionCreateOptions, BranchId, CanonicalValue, CommitId, Engine, EntityId,
    ErrorCategory, ErrorCode, EvidenceContentInput, EvidenceCreateOptions, EvidenceId,
    StoreInitOptions, TaskCreateOptions, VerificationCreateCommit, VerificationCreateOptions,
    VerificationResult, VerificationTarget, WhyDeferredRelationFamily, WhyQueryOptions,
    WhyQueryTarget, WhyRelationKind, WorkspaceInfo, WorkspaceInitOptions, content_object_digest,
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
        StoreInitOptions::new("phase3v-verification-evidence-closure-store")
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

fn metadata(label: &str) -> CanonicalValue {
    object(vec![("label", string(label))])
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
                "Implement verification evidence closure",
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
                "The verification records immutable evidence.",
                AcceptanceCriterionClassification::Required,
            )
            .expect("criterion options"),
        )
        .expect("create acceptance criterion");
    CriterionFixture { criterion }
}

fn create_evidence(engine: &mut Engine, label: &str) -> EvidenceId {
    engine
        .create_evidence(
            EvidenceCreateOptions::new("command_output", metadata(label))
                .expect("evidence options"),
        )
        .expect("create evidence")
        .evidence_id
}

fn create_verification(
    engine: &mut Engine,
    workspace: &WorkspaceInfo,
    expected_head_commit_id: CommitId,
    target: VerificationTarget,
    result: VerificationResult,
    evidence_ids: Vec<EvidenceId>,
) -> VerificationCreateCommit {
    engine
        .create_verification(
            VerificationCreateOptions::new(
                workspace.initial_branch_id,
                expected_head_commit_id,
                target,
                result,
            )
            .expect("verification options")
            .with_evidence(evidence_ids)
            .expect("verification evidence"),
        )
        .expect("create verification")
}

fn evidenced_by_relation_targets(
    connection: &Connection,
    verification_entity_id: EntityId,
) -> Vec<EvidenceId> {
    let mut statement = connection
        .prepare(
            "SELECT target_object_id
             FROM relation
             WHERE relation_type = 'evidenced_by'
               AND source_object_id = ?1
             ORDER BY target_object_id",
        )
        .expect("prepare evidenced_by targets");
    statement
        .query_map(params![&verification_entity_id.raw_bytes()[..]], |row| {
            row.get::<_, Vec<u8>>(0)
        })
        .expect("query evidenced_by targets")
        .map(|row| {
            EvidenceId::from_bytes(row.expect("target bytes").try_into().expect("evidence id"))
                .expect("evidence id")
        })
        .collect()
}

fn changeset_operation_count(
    connection: &Connection,
    changeset_id: workvcs_core::ChangeSetId,
) -> i64 {
    connection
        .query_row(
            "SELECT count(*) FROM change_operation WHERE changeset_id = ?1",
            params![&changeset_id.raw_bytes()[..]],
            |row| row.get(0),
        )
        .expect("changeset operation count")
}

#[test]
fn evidence_can_be_metadata_only_or_reference_raw_byte_content() {
    let (_tempdir, path) = store_path();
    let mut engine = init_engine(&path);
    let raw_output = b"phase 3v passed\n";
    let content = EvidenceContentInput::from_raw_bytes("stdout", raw_output)
        .expect("raw content")
        .with_media_type("text/plain")
        .expect("media type")
        .with_format_metadata(object(vec![("encoding", string("utf-8"))]))
        .expect("format metadata");

    let evidence = engine
        .create_evidence(
            EvidenceCreateOptions::new("command_output", metadata("cargo test"))
                .expect("evidence options")
                .with_contents(vec![content])
                .expect("contents"),
        )
        .expect("create evidence");
    let snapshot = engine.evidence(evidence.evidence_id).expect("evidence");
    let connection = raw_connection(&path);

    assert_eq!(snapshot.evidence_kind, "command_output");
    assert_eq!(snapshot.metadata, metadata("cargo test"));
    assert_eq!(snapshot.contents.len(), 1);
    assert_eq!(snapshot.contents[0].ordinal, 0);
    assert_eq!(snapshot.contents[0].role, "stdout");
    assert_eq!(
        snapshot.contents[0].content_digest,
        content_object_digest(raw_output)
    );
    assert_eq!(snapshot.contents[0].size_bytes, raw_output.len() as i64);
    assert_eq!(
        snapshot.contents[0].media_type.as_deref(),
        Some("text/plain")
    );
    assert_eq!(
        snapshot.contents[0].format_metadata,
        object(vec![("encoding", string("utf-8"))])
    );
    assert_eq!(count_rows(&connection, "workstate_commit"), 0);
    assert_eq!(count_rows(&connection, "content_storage_location"), 0);

    let metadata_only = engine
        .create_evidence(
            EvidenceCreateOptions::new("external_reference", metadata("manual transcript"))
                .expect("metadata-only options"),
        )
        .expect("metadata-only evidence");
    assert!(metadata_only.contents.is_empty());
}

#[test]
fn content_object_digest_metadata_is_reused_or_rejected_on_conflict() {
    let (_tempdir, path) = store_path();
    let mut engine = init_engine(&path);
    let digest = content_object_digest(b"same bytes");
    let first_content = EvidenceContentInput::from_digest("stdout", digest, 10)
        .expect("digest content")
        .with_media_type("text/plain")
        .expect("media");
    let second_content = EvidenceContentInput::from_digest("stdout", digest, 10)
        .expect("digest content")
        .with_media_type("text/plain")
        .expect("media");

    engine
        .create_evidence(
            EvidenceCreateOptions::new("command_output", metadata("first"))
                .expect("first options")
                .with_contents(vec![first_content])
                .expect("first contents"),
        )
        .expect("first evidence");
    engine
        .create_evidence(
            EvidenceCreateOptions::new("command_output", metadata("second"))
                .expect("second options")
                .with_contents(vec![second_content])
                .expect("second contents"),
        )
        .expect("second evidence");
    let connection = raw_connection(&path);
    assert_eq!(count_rows(&connection, "content_object"), 1);

    let conflicting_content = EvidenceContentInput::from_digest("stdout", digest, 10)
        .expect("conflicting content")
        .with_media_type("application/json")
        .expect("conflicting media");
    let error = engine
        .create_evidence(
            EvidenceCreateOptions::new("command_output", metadata("conflict"))
                .expect("conflict options")
                .with_contents(vec![conflicting_content])
                .expect("conflict contents"),
        )
        .expect_err("conflicting content metadata should be rejected");

    assert_eq!(error.code(), ErrorCode::EvidenceInvalid);
    assert_eq!(error.category(), ErrorCategory::Evidence);
    assert_eq!(count_rows(&connection, "evidence"), 2);
}

#[test]
fn verification_creation_records_evidenced_by_closure_atomically() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let fixture = create_required_criterion(&mut engine, &workspace);
    let first_evidence = create_evidence(&mut engine, "first");
    let second_evidence = create_evidence(&mut engine, "second");
    let verification = create_verification(
        &mut engine,
        &workspace,
        fixture.criterion.commit_id,
        VerificationTarget::AcceptanceCriterion(fixture.criterion.acceptance_criterion_entity_id),
        VerificationResult::Passed,
        vec![second_evidence, first_evidence],
    );
    let connection = raw_connection(&path);

    let mut expected_evidence = vec![first_evidence, second_evidence];
    expected_evidence.sort();
    assert_eq!(
        verification
            .state
            .evidence
            .iter()
            .map(|entry| entry.evidence_id)
            .collect::<Vec<_>>(),
        expected_evidence
    );
    assert_eq!(verification.evidenced_by_relations.len(), 2);
    assert_eq!(
        evidenced_by_relation_targets(&connection, verification.verification_entity_id),
        expected_evidence
    );
    assert_eq!(
        engine
            .state_at(verification.commit_id)
            .expect("state after verification")
            .state
            .relations()
            .len(),
        3
    );
    assert_eq!(
        changeset_operation_count(&connection, verification.changeset_id),
        4
    );

    let snapshot = engine
        .verification_at(verification.commit_id, verification.verification_entity_id)
        .expect("verification snapshot");
    assert_eq!(snapshot.state.evidence, verification.state.evidence);
    assert_eq!(snapshot.evidenced_by_relations.len(), 2);
    assert_eq!(
        engine
            .acceptance_criterion_effective_status(
                verification.commit_id,
                fixture.criterion.acceptance_criterion_entity_id,
            )
            .expect("effective status")
            .as_str(),
        "verified"
    );

    let why = engine
        .why(WhyQueryOptions::new(
            WhyQueryTarget::commit(verification.commit_id),
            verification.verification_entity_id,
        ))
        .expect("why skips evidence endpoints");
    assert_eq!(
        why.relation_edges
            .iter()
            .filter(|edge| edge.relation_kind == WhyRelationKind::Verifies)
            .count(),
        1
    );
    assert_eq!(
        why.deferred_relation_families,
        vec![
            WhyDeferredRelationFamily::Evolution,
            WhyDeferredRelationFamily::Epistemic,
        ]
    );
}

#[test]
fn one_evidence_object_can_be_reused_by_multiple_verifications() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let fixture = create_required_criterion(&mut engine, &workspace);
    let evidence = create_evidence(&mut engine, "shared command output");
    let first = create_verification(
        &mut engine,
        &workspace,
        fixture.criterion.commit_id,
        VerificationTarget::AcceptanceCriterion(fixture.criterion.acceptance_criterion_entity_id),
        VerificationResult::Passed,
        vec![evidence],
    );
    let second = create_verification(
        &mut engine,
        &workspace,
        first.commit_id,
        VerificationTarget::AcceptanceCriterion(fixture.criterion.acceptance_criterion_entity_id),
        VerificationResult::Passed,
        vec![evidence],
    );
    let connection = raw_connection(&path);
    let relation_count: i64 = connection
        .query_row(
            "SELECT count(*)
             FROM relation
             WHERE relation_type = 'evidenced_by'
               AND target_object_id = ?1",
            params![&evidence.raw_bytes()[..]],
            |row| row.get(0),
        )
        .expect("evidence relation count");

    assert_ne!(first.verification_entity_id, second.verification_entity_id);
    assert_eq!(relation_count, 2);
}

#[test]
fn duplicate_or_missing_evidence_is_rejected_before_verification_commit() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let fixture = create_required_criterion(&mut engine, &workspace);
    let evidence = create_evidence(&mut engine, "single evidence");
    let duplicate_options = VerificationCreateOptions::new(
        workspace.initial_branch_id,
        fixture.criterion.commit_id,
        VerificationTarget::AcceptanceCriterion(fixture.criterion.acceptance_criterion_entity_id),
        VerificationResult::Passed,
    )
    .expect("verification options")
    .with_evidence(vec![evidence, evidence])
    .expect_err("duplicate evidence should be rejected");
    assert_eq!(duplicate_options.code(), ErrorCode::TaskInvalid);

    let connection = raw_connection(&path);
    let before_head = branch_head(&connection, workspace.initial_branch_id);
    let missing = EvidenceId::new_v7();
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
            .expect("missing options")
            .with_evidence(vec![missing])
            .expect("missing evidence id is syntactically valid"),
        )
        .expect_err("missing evidence should be rejected");

    assert_eq!(error.code(), ErrorCode::EvidenceNotFound);
    assert_eq!(error.category(), ErrorCategory::Evidence);
    assert_eq!(
        branch_head(&connection, workspace.initial_branch_id),
        before_head
    );
    assert_eq!(count_rows(&connection, "workstate_commit"), 3);
}

#[test]
fn verification_readback_rejects_corrupted_evidence_closure() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let fixture = create_required_criterion(&mut engine, &workspace);
    let original_evidence = create_evidence(&mut engine, "original");
    let other_evidence = create_evidence(&mut engine, "other");
    let verification = create_verification(
        &mut engine,
        &workspace,
        fixture.criterion.commit_id,
        VerificationTarget::AcceptanceCriterion(fixture.criterion.acceptance_criterion_entity_id),
        VerificationResult::Passed,
        vec![original_evidence],
    );
    let relation_id = verification.evidenced_by_relations[0].relation_id;
    let connection = raw_connection(&path);
    connection
        .execute(
            "UPDATE relation
             SET target_object_id = ?1
             WHERE object_id = ?2",
            params![
                &other_evidence.raw_bytes()[..],
                &relation_id.raw_bytes()[..]
            ],
        )
        .expect("corrupt evidenced_by target");

    let snapshot_error = engine
        .verification_at(verification.commit_id, verification.verification_entity_id)
        .expect_err("corrupted closure should be rejected");
    assert_eq!(snapshot_error.code(), ErrorCode::TaskInvalid);

    let status_error = engine
        .acceptance_criterion_effective_status(
            verification.commit_id,
            fixture.criterion.acceptance_criterion_entity_id,
        )
        .expect_err("effective status should reject corrupted closure");
    assert_eq!(status_error.code(), ErrorCode::TaskInvalid);
}
