use rusqlite::{Connection, params};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use tempfile::TempDir;
use workvcs_core::{
    AcceptanceCriterionClassification, AcceptanceCriterionCreateCommit,
    AcceptanceCriterionCreateOptions, BranchId, CanonicalValue, CommitId, Engine, EntityId,
    ErrorCategory, ErrorCode, EvidenceCreateOptions, EvidenceId, ResolvedWhyQuerySubject,
    StoreInitOptions, TaskCreateOptions, VerificationCreateCommit, VerificationCreateOptions,
    VerificationResult, VerificationTarget, WhyEntityKind, WhyQueryOptions, WhyQueryResult,
    WhyQueryTarget, WhyRelationDirection, WhyRelationEdge, WhyRelationEndpoint, WhyRelationKind,
    WorkspaceInfo, WorkspaceInitOptions,
};

#[derive(Clone, Debug, PartialEq, Eq)]
struct QueryCounts {
    branch_projection_state: i64,
    changeset: i64,
    change_operation: i64,
    workstate_commit: i64,
    relation: i64,
    relation_version: i64,
    entity_version: i64,
    evidence: i64,
    content_object: i64,
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
        StoreInitOptions::new("phase3w-why-evidence-neighborhood-store").expect("store options"),
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

fn query_counts(connection: &Connection) -> QueryCounts {
    QueryCounts {
        branch_projection_state: count_rows(connection, "branch_projection_state"),
        changeset: count_rows(connection, "changeset"),
        change_operation: count_rows(connection, "change_operation"),
        workstate_commit: count_rows(connection, "workstate_commit"),
        relation: count_rows(connection, "relation"),
        relation_version: count_rows(connection, "relation_version"),
        entity_version: count_rows(connection, "entity_version"),
        evidence: count_rows(connection, "evidence"),
        content_object: count_rows(connection, "content_object"),
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

fn create_required_criterion(
    engine: &mut Engine,
    workspace: &WorkspaceInfo,
) -> AcceptanceCriterionCreateCommit {
    let task = engine
        .create_task(
            TaskCreateOptions::new(
                workspace.initial_branch_id,
                workspace.genesis_commit_id,
                "Explain verification evidence",
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
                "The verification exposes its evidence in why.",
                AcceptanceCriterionClassification::Required,
            )
            .expect("criterion options"),
        )
        .expect("create acceptance criterion")
}

fn create_evidence(engine: &mut Engine, label: &str) -> EvidenceId {
    engine
        .create_evidence(
            EvidenceCreateOptions::new("command_output", object(vec![("label", string(label))]))
                .expect("evidence options"),
        )
        .expect("create evidence")
        .evidence_id
}

fn create_verification(
    engine: &mut Engine,
    workspace: &WorkspaceInfo,
    expected_head_commit_id: CommitId,
    target_entity_id: EntityId,
    evidence_ids: Vec<EvidenceId>,
) -> VerificationCreateCommit {
    engine
        .create_verification(
            VerificationCreateOptions::new(
                workspace.initial_branch_id,
                expected_head_commit_id,
                VerificationTarget::AcceptanceCriterion(target_entity_id),
                VerificationResult::Passed,
            )
            .expect("verification options")
            .with_evidence(evidence_ids)
            .expect("verification evidence"),
        )
        .expect("create verification")
}

fn why_branch_head(
    engine: &Engine,
    branch_id: BranchId,
    subject_entity_id: EntityId,
) -> WhyQueryResult {
    engine
        .why(WhyQueryOptions::new(
            WhyQueryTarget::branch_head(branch_id),
            subject_entity_id,
        ))
        .expect("why branch head")
}

fn why_evidence_branch_head(
    engine: &Engine,
    branch_id: BranchId,
    evidence_id: EvidenceId,
) -> WhyQueryResult {
    engine
        .why(WhyQueryOptions::for_evidence(
            WhyQueryTarget::branch_head(branch_id),
            evidence_id,
        ))
        .expect("why evidence branch head")
}

fn why_evidence_commit(
    engine: &Engine,
    commit_id: CommitId,
    evidence_id: EvidenceId,
) -> WhyQueryResult {
    engine
        .why(WhyQueryOptions::for_evidence(
            WhyQueryTarget::commit(commit_id),
            evidence_id,
        ))
        .expect("why evidence commit")
}

fn entity_endpoint(entity_id: EntityId, entity_kind: WhyEntityKind) -> WhyRelationEndpoint {
    WhyRelationEndpoint::entity(entity_id, entity_kind)
}

fn evidence_endpoint(evidence_id: EvidenceId) -> WhyRelationEndpoint {
    WhyRelationEndpoint::evidence(evidence_id)
}

fn edge_facts(
    why: &WhyQueryResult,
) -> BTreeSet<(
    WhyRelationKind,
    WhyRelationDirection,
    WhyRelationEndpoint,
    WhyRelationEndpoint,
)> {
    why.relation_edges
        .iter()
        .map(|edge| (edge.relation_kind, edge.direction, edge.source, edge.target))
        .collect()
}

fn edge_order_key(
    edge: &WhyRelationEdge,
) -> (
    WhyRelationKind,
    WhyRelationDirection,
    WhyRelationEndpoint,
    WhyRelationEndpoint,
    workvcs_core::RelationId,
) {
    (
        edge.relation_kind,
        edge.direction,
        edge.source,
        edge.target,
        edge.relation_id,
    )
}

fn assert_edges_are_sorted(edges: &[WhyRelationEdge]) {
    for pair in edges.windows(2) {
        assert!(edge_order_key(&pair[0]) <= edge_order_key(&pair[1]));
    }
}

fn assert_deferred_families(why: &WhyQueryResult) {
    assert!(why.deferred_relation_families.is_empty());
}

#[test]
fn why_verification_subject_reports_evidenced_by_edges_to_evidence_endpoints() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let criterion = create_required_criterion(&mut engine, &workspace);
    let first_evidence = create_evidence(&mut engine, "first");
    let second_evidence = create_evidence(&mut engine, "second");
    let verification = create_verification(
        &mut engine,
        &workspace,
        criterion.commit_id,
        criterion.acceptance_criterion_entity_id,
        vec![second_evidence, first_evidence],
    );

    let why = why_branch_head(
        &engine,
        workspace.initial_branch_id,
        verification.verification_entity_id,
    );

    assert_eq!(
        why.subject,
        ResolvedWhyQuerySubject::Entity {
            entity_id: verification.verification_entity_id,
            entity_version_id: verification.verification_entity_version_id,
            entity_kind: WhyEntityKind::Verification,
        }
    );
    assert_deferred_families(&why);
    assert_eq!(
        edge_facts(&why),
        BTreeSet::from([
            (
                WhyRelationKind::Verifies,
                WhyRelationDirection::Outgoing,
                entity_endpoint(
                    verification.verification_entity_id,
                    WhyEntityKind::Verification,
                ),
                entity_endpoint(
                    criterion.acceptance_criterion_entity_id,
                    WhyEntityKind::AcceptanceCriterion,
                ),
            ),
            (
                WhyRelationKind::EvidencedBy,
                WhyRelationDirection::Outgoing,
                entity_endpoint(
                    verification.verification_entity_id,
                    WhyEntityKind::Verification,
                ),
                evidence_endpoint(first_evidence),
            ),
            (
                WhyRelationKind::EvidencedBy,
                WhyRelationDirection::Outgoing,
                entity_endpoint(
                    verification.verification_entity_id,
                    WhyEntityKind::Verification,
                ),
                evidence_endpoint(second_evidence),
            ),
        ])
    );
    assert_edges_are_sorted(&why.relation_edges);
}

#[test]
fn why_evidence_subject_reports_current_verifications_using_that_evidence() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let criterion = create_required_criterion(&mut engine, &workspace);
    let evidence = create_evidence(&mut engine, "shared");
    let first = create_verification(
        &mut engine,
        &workspace,
        criterion.commit_id,
        criterion.acceptance_criterion_entity_id,
        vec![evidence],
    );
    let second = create_verification(
        &mut engine,
        &workspace,
        first.commit_id,
        criterion.acceptance_criterion_entity_id,
        vec![evidence],
    );

    let why = why_evidence_branch_head(&engine, workspace.initial_branch_id, evidence);

    assert_eq!(why.target.commit_id, second.commit_id);
    assert_eq!(
        why.subject,
        ResolvedWhyQuerySubject::Evidence {
            evidence_id: evidence
        }
    );
    assert_deferred_families(&why);
    assert_eq!(
        edge_facts(&why),
        BTreeSet::from([
            (
                WhyRelationKind::EvidencedBy,
                WhyRelationDirection::Incoming,
                entity_endpoint(first.verification_entity_id, WhyEntityKind::Verification),
                evidence_endpoint(evidence),
            ),
            (
                WhyRelationKind::EvidencedBy,
                WhyRelationDirection::Incoming,
                entity_endpoint(second.verification_entity_id, WhyEntityKind::Verification),
                evidence_endpoint(evidence),
            ),
        ])
    );
    assert_edges_are_sorted(&why.relation_edges);
}

#[test]
fn why_evidence_subject_is_read_only() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let criterion = create_required_criterion(&mut engine, &workspace);
    let evidence = create_evidence(&mut engine, "read only");
    let verification = create_verification(
        &mut engine,
        &workspace,
        criterion.commit_id,
        criterion.acceptance_criterion_entity_id,
        vec![evidence],
    );
    let connection = raw_connection(&path);
    let before_counts = query_counts(&connection);
    let before_head = branch_head(&connection, workspace.initial_branch_id);

    let why = why_evidence_branch_head(&engine, workspace.initial_branch_id, evidence);

    assert_eq!(why.target.commit_id, verification.commit_id);
    assert_eq!(why.relation_edges.len(), 1);
    assert_eq!(query_counts(&connection), before_counts);
    assert_eq!(
        branch_head(&connection, workspace.initial_branch_id),
        before_head
    );
}

#[test]
fn why_evidence_subject_uses_selected_workstate_commit_not_future_edges() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let criterion = create_required_criterion(&mut engine, &workspace);
    let evidence = create_evidence(&mut engine, "future edge");

    let before = why_evidence_commit(&engine, criterion.commit_id, evidence);
    let verification = create_verification(
        &mut engine,
        &workspace,
        criterion.commit_id,
        criterion.acceptance_criterion_entity_id,
        vec![evidence],
    );
    let after = why_evidence_commit(&engine, verification.commit_id, evidence);

    assert!(before.relation_edges.is_empty());
    assert_eq!(
        before.subject,
        ResolvedWhyQuerySubject::Evidence {
            evidence_id: evidence
        }
    );
    assert_eq!(
        edge_facts(&after),
        BTreeSet::from([(
            WhyRelationKind::EvidencedBy,
            WhyRelationDirection::Incoming,
            entity_endpoint(
                verification.verification_entity_id,
                WhyEntityKind::Verification,
            ),
            evidence_endpoint(evidence),
        )])
    );
}

#[test]
fn why_rejects_missing_evidence_subject() {
    let (_tempdir, path) = store_path();
    let (engine, workspace) = create_workspace(&path);
    let missing = EvidenceId::new_v7();

    let error = engine
        .why(WhyQueryOptions::for_evidence(
            WhyQueryTarget::branch_head(workspace.initial_branch_id),
            missing,
        ))
        .expect_err("missing evidence subject should be rejected");

    assert_eq!(error.code(), ErrorCode::EvidenceNotFound);
    assert_eq!(error.category(), ErrorCategory::Evidence);
}

#[test]
fn why_rejects_corrupted_current_evidenced_by_relation() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let criterion = create_required_criterion(&mut engine, &workspace);
    let evidence = create_evidence(&mut engine, "target kind corruption");
    let verification = create_verification(
        &mut engine,
        &workspace,
        criterion.commit_id,
        criterion.acceptance_criterion_entity_id,
        vec![evidence],
    );
    let connection = raw_connection(&path);
    connection
        .execute(
            "UPDATE relation
             SET target_object_id = ?1
             WHERE object_id = ?2",
            params![
                &criterion.acceptance_criterion_entity_id.raw_bytes()[..],
                &verification.evidenced_by_relations[0]
                    .relation_id
                    .raw_bytes()[..]
            ],
        )
        .expect("corrupt evidenced_by target");

    let target_error = engine
        .why(WhyQueryOptions::new(
            WhyQueryTarget::commit(verification.commit_id),
            verification.verification_entity_id,
        ))
        .expect_err("corrupted evidenced_by target should be rejected");

    assert_eq!(target_error.code(), ErrorCode::TaskInvalid);
    assert_eq!(target_error.category(), ErrorCategory::Task);

    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let criterion = create_required_criterion(&mut engine, &workspace);
    let evidence = create_evidence(&mut engine, "source currency corruption");
    let first_verification = create_verification(
        &mut engine,
        &workspace,
        criterion.commit_id,
        criterion.acceptance_criterion_entity_id,
        vec![evidence],
    );
    let later_verification = create_verification(
        &mut engine,
        &workspace,
        first_verification.commit_id,
        criterion.acceptance_criterion_entity_id,
        Vec::new(),
    );
    let connection = raw_connection(&path);
    connection
        .execute(
            "UPDATE relation
             SET source_object_id = ?1
             WHERE object_id = ?2",
            params![
                &later_verification.verification_entity_id.raw_bytes()[..],
                &first_verification.evidenced_by_relations[0]
                    .relation_id
                    .raw_bytes()[..]
            ],
        )
        .expect("corrupt evidenced_by source");

    let source_error = engine
        .why(WhyQueryOptions::for_evidence(
            WhyQueryTarget::commit(first_verification.commit_id),
            evidence,
        ))
        .expect_err("noncurrent evidenced_by source should be rejected");

    assert_eq!(source_error.code(), ErrorCode::TaskInvalid);
    assert_eq!(source_error.category(), ErrorCategory::Task);
}
