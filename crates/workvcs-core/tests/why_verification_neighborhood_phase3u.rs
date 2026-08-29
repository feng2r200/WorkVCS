use rusqlite::{Connection, params};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use tempfile::TempDir;
use workvcs_core::{
    AcceptanceCriterionClassification, AcceptanceCriterionCreateCommit,
    AcceptanceCriterionCreateOptions, BranchId, CommitId, Engine, EntityId, ErrorCategory,
    ErrorCode, EventId, RelationId, StoreInitOptions, TaskCreateOptions, VerificationCreateCommit,
    VerificationCreateOptions, VerificationRequirementCreateCommit,
    VerificationRequirementCreateOptions, VerificationResult, VerificationTarget,
    WhyDeferredRelationFamily, WhyEntityKind, WhyQueryOptions, WhyQueryResult, WhyQueryTarget,
    WhyRelationDirection, WhyRelationEdge, WhyRelationKind, WorkspaceInfo, WorkspaceInitOptions,
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
    event: i64,
}

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
        StoreInitOptions::new("phase3u-why-verification-neighborhood-store")
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

fn create_required_criterion(engine: &mut Engine, workspace: &WorkspaceInfo) -> CriterionFixture {
    let task = engine
        .create_task(
            TaskCreateOptions::new(
                workspace.initial_branch_id,
                workspace.genesis_commit_id,
                "Implement verified why behavior",
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
                "The why query explains verification judgments.",
                AcceptanceCriterionClassification::Required,
            )
            .expect("criterion options"),
        )
        .expect("create acceptance criterion");
    CriterionFixture { criterion }
}

fn create_requirement(
    engine: &mut Engine,
    workspace: &WorkspaceInfo,
    expected_head_commit_id: CommitId,
    criterion: &AcceptanceCriterionCreateCommit,
    local_key: &str,
) -> VerificationRequirementCreateCommit {
    engine
        .create_verification_requirement(
            VerificationRequirementCreateOptions::new(
                workspace.initial_branch_id,
                expected_head_commit_id,
                criterion.acceptance_criterion_entity_id,
                criterion.acceptance_criterion_entity_version_id,
                local_key,
                "The required command output is preserved.",
            )
            .expect("requirement options"),
        )
        .expect("create verification requirement")
}

fn create_verification(
    engine: &mut Engine,
    workspace: &WorkspaceInfo,
    expected_head_commit_id: CommitId,
    target: VerificationTarget,
    result: VerificationResult,
) -> VerificationCreateCommit {
    engine
        .create_verification(
            VerificationCreateOptions::new(
                workspace.initial_branch_id,
                expected_head_commit_id,
                target,
                result,
            )
            .expect("verification options"),
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

fn why_commit(engine: &Engine, commit_id: CommitId, subject_entity_id: EntityId) -> WhyQueryResult {
    engine
        .why(WhyQueryOptions::new(
            WhyQueryTarget::commit(commit_id),
            subject_entity_id,
        ))
        .expect("why commit")
}

fn edge_facts(
    why: &WhyQueryResult,
) -> BTreeSet<(
    WhyRelationKind,
    WhyRelationDirection,
    EntityId,
    WhyEntityKind,
    EntityId,
    WhyEntityKind,
)> {
    why.relation_edges
        .iter()
        .map(|edge| {
            (
                edge.relation_kind,
                edge.direction,
                edge.source_entity_id,
                edge.source_kind,
                edge.target_entity_id,
                edge.target_kind,
            )
        })
        .collect()
}

fn edge_order_key(
    edge: &WhyRelationEdge,
) -> (
    WhyRelationKind,
    WhyRelationDirection,
    WhyEntityKind,
    EntityId,
    WhyEntityKind,
    EntityId,
    RelationId,
) {
    (
        edge.relation_kind,
        edge.direction,
        edge.source_kind,
        edge.source_entity_id,
        edge.target_kind,
        edge.target_entity_id,
        edge.relation_id,
    )
}

fn assert_edges_are_sorted(edges: &[WhyRelationEdge]) {
    for pair in edges.windows(2) {
        assert!(edge_order_key(&pair[0]) <= edge_order_key(&pair[1]));
    }
}

fn assert_deferred_families(why: &WhyQueryResult) {
    assert_eq!(
        why.deferred_relation_families,
        vec![
            WhyDeferredRelationFamily::Evolution,
            WhyDeferredRelationFamily::Epistemic,
            WhyDeferredRelationFamily::VerificationEvidence,
        ]
    );
}

#[test]
fn why_reports_direct_acceptance_criterion_verification_from_both_endpoints() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let fixture = create_required_criterion(&mut engine, &workspace);
    let verification = create_verification(
        &mut engine,
        &workspace,
        fixture.criterion.commit_id,
        VerificationTarget::AcceptanceCriterion(fixture.criterion.acceptance_criterion_entity_id),
        VerificationResult::Passed,
    );

    let criterion_why = why_branch_head(
        &engine,
        workspace.initial_branch_id,
        fixture.criterion.acceptance_criterion_entity_id,
    );
    let verification_why = why_branch_head(
        &engine,
        workspace.initial_branch_id,
        verification.verification_entity_id,
    );

    assert_eq!(criterion_why.target.commit_id, verification.commit_id);
    assert_eq!(
        criterion_why.target.target,
        WhyQueryTarget::branch_head(workspace.initial_branch_id)
    );
    assert_eq!(
        criterion_why.subject_entity_version_id,
        fixture.criterion.acceptance_criterion_entity_version_id
    );
    assert_deferred_families(&criterion_why);
    assert_eq!(
        edge_facts(&criterion_why),
        BTreeSet::from([(
            WhyRelationKind::Verifies,
            WhyRelationDirection::Incoming,
            verification.verification_entity_id,
            WhyEntityKind::Verification,
            fixture.criterion.acceptance_criterion_entity_id,
            WhyEntityKind::AcceptanceCriterion,
        )])
    );

    assert_eq!(
        verification_why.subject_entity_version_id,
        verification.verification_entity_version_id
    );
    assert_deferred_families(&verification_why);
    assert_eq!(
        edge_facts(&verification_why),
        BTreeSet::from([(
            WhyRelationKind::Verifies,
            WhyRelationDirection::Outgoing,
            verification.verification_entity_id,
            WhyEntityKind::Verification,
            fixture.criterion.acceptance_criterion_entity_id,
            WhyEntityKind::AcceptanceCriterion,
        )])
    );
}

#[test]
fn why_reports_verification_requirement_target_relation() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let fixture = create_required_criterion(&mut engine, &workspace);
    let requirement = create_requirement(
        &mut engine,
        &workspace,
        fixture.criterion.commit_id,
        &fixture.criterion,
        "VR-1",
    );
    let verification = create_verification(
        &mut engine,
        &workspace,
        requirement.commit_id,
        VerificationTarget::VerificationRequirement(requirement.verification_requirement_entity_id),
        VerificationResult::Passed,
    );

    let requirement_why = why_branch_head(
        &engine,
        workspace.initial_branch_id,
        requirement.verification_requirement_entity_id,
    );

    assert_eq!(
        requirement_why.subject_entity_version_id,
        requirement.verification_requirement_entity_version_id
    );
    assert_deferred_families(&requirement_why);
    assert_eq!(
        edge_facts(&requirement_why),
        BTreeSet::from([(
            WhyRelationKind::Verifies,
            WhyRelationDirection::Incoming,
            verification.verification_entity_id,
            WhyEntityKind::Verification,
            requirement.verification_requirement_entity_id,
            WhyEntityKind::VerificationRequirement,
        )])
    );
}

#[test]
fn why_commit_selector_reads_historical_verification_neighborhood() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let fixture = create_required_criterion(&mut engine, &workspace);
    let before = why_commit(
        &engine,
        fixture.criterion.commit_id,
        fixture.criterion.acceptance_criterion_entity_id,
    );
    let verification = create_verification(
        &mut engine,
        &workspace,
        fixture.criterion.commit_id,
        VerificationTarget::AcceptanceCriterion(fixture.criterion.acceptance_criterion_entity_id),
        VerificationResult::Passed,
    );
    let after = why_commit(
        &engine,
        verification.commit_id,
        fixture.criterion.acceptance_criterion_entity_id,
    );

    assert!(before.relation_edges.is_empty());
    assert_eq!(before.target.commit_id, fixture.criterion.commit_id);
    assert_eq!(after.target.commit_id, verification.commit_id);
    assert_eq!(
        edge_facts(&after),
        BTreeSet::from([(
            WhyRelationKind::Verifies,
            WhyRelationDirection::Incoming,
            verification.verification_entity_id,
            WhyEntityKind::Verification,
            fixture.criterion.acceptance_criterion_entity_id,
            WhyEntityKind::AcceptanceCriterion,
        )])
    );
}

#[test]
fn why_verification_edges_are_sorted_read_only_and_ignore_projection_and_event_noise() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let fixture = create_required_criterion(&mut engine, &workspace);
    let failed = create_verification(
        &mut engine,
        &workspace,
        fixture.criterion.commit_id,
        VerificationTarget::AcceptanceCriterion(fixture.criterion.acceptance_criterion_entity_id),
        VerificationResult::Failed,
    );
    let passed = create_verification(
        &mut engine,
        &workspace,
        failed.commit_id,
        VerificationTarget::AcceptanceCriterion(fixture.criterion.acceptance_criterion_entity_id),
        VerificationResult::Passed,
    );
    let connection = raw_connection(&path);
    let branch_id = workspace.initial_branch_id.raw_bytes();
    let genesis_commit_id = workspace.genesis_commit_id.raw_bytes();
    let event_id = EventId::new_v7().raw_bytes();
    connection
        .execute(
            "INSERT INTO branch_projection_state(
                branch_id,
                projection_status,
                projected_commit_id,
                projection_state_digest,
                updated_at_us
             )
             VALUES (?1, 'complete', ?2, ?3, 7)",
            params![&branch_id[..], &genesis_commit_id[..], &[8_u8; 32][..]],
        )
        .expect("insert projection noise");
    connection
        .execute(
            "INSERT INTO event(
                event_id,
                workspace_id,
                changeset_id,
                session_id,
                event_kind,
                occurred_at_us,
                payload_json
             )
             VALUES (?1, ?2, NULL, NULL, 'why.verification.noise', 8, '{}')",
            params![&event_id[..], &workspace.workspace_id.raw_bytes()[..]],
        )
        .expect("insert event noise");
    let before_counts = query_counts(&connection);
    let before_head = branch_head(&connection, workspace.initial_branch_id);

    let why = why_branch_head(
        &engine,
        workspace.initial_branch_id,
        fixture.criterion.acceptance_criterion_entity_id,
    );

    assert_eq!(why.target.commit_id, passed.commit_id);
    assert_eq!(why.relation_edges.len(), 2);
    assert_eq!(
        edge_facts(&why),
        BTreeSet::from([
            (
                WhyRelationKind::Verifies,
                WhyRelationDirection::Incoming,
                failed.verification_entity_id,
                WhyEntityKind::Verification,
                fixture.criterion.acceptance_criterion_entity_id,
                WhyEntityKind::AcceptanceCriterion,
            ),
            (
                WhyRelationKind::Verifies,
                WhyRelationDirection::Incoming,
                passed.verification_entity_id,
                WhyEntityKind::Verification,
                fixture.criterion.acceptance_criterion_entity_id,
                WhyEntityKind::AcceptanceCriterion,
            ),
        ])
    );
    assert_edges_are_sorted(&why.relation_edges);
    assert_eq!(query_counts(&connection), before_counts);
    assert_eq!(
        branch_head(&connection, workspace.initial_branch_id),
        before_head
    );
}

#[test]
fn why_rejects_malformed_current_verifies_relation() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let fixture = create_required_criterion(&mut engine, &workspace);
    let verification = create_verification(
        &mut engine,
        &workspace,
        fixture.criterion.commit_id,
        VerificationTarget::AcceptanceCriterion(fixture.criterion.acceptance_criterion_entity_id),
        VerificationResult::Passed,
    );
    let connection = raw_connection(&path);
    connection
        .execute(
            "UPDATE relation
             SET relation_discriminator = 'malformed'
             WHERE object_id = ?1",
            params![&verification.verifies_relation_id.raw_bytes()[..]],
        )
        .expect("corrupt verifies relation discriminator");

    let error = engine
        .why(WhyQueryOptions::new(
            WhyQueryTarget::commit(verification.commit_id),
            fixture.criterion.acceptance_criterion_entity_id,
        ))
        .expect_err("malformed verifies relation should be rejected");

    assert_eq!(error.code(), ErrorCode::TaskInvalid);
    assert_eq!(error.category(), ErrorCategory::Task);
}

#[test]
fn why_rejects_verifies_relation_with_noncurrent_endpoint() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let fixture = create_required_criterion(&mut engine, &workspace);
    let first_verification = create_verification(
        &mut engine,
        &workspace,
        fixture.criterion.commit_id,
        VerificationTarget::AcceptanceCriterion(fixture.criterion.acceptance_criterion_entity_id),
        VerificationResult::Passed,
    );
    let later_requirement = create_requirement(
        &mut engine,
        &workspace,
        first_verification.commit_id,
        &fixture.criterion,
        "VR-1",
    );
    let later_verification = create_verification(
        &mut engine,
        &workspace,
        later_requirement.commit_id,
        VerificationTarget::VerificationRequirement(
            later_requirement.verification_requirement_entity_id,
        ),
        VerificationResult::Failed,
    );
    let connection = raw_connection(&path);
    connection
        .execute(
            "UPDATE relation
             SET source_object_id = ?1
             WHERE object_id = ?2",
            params![
                &later_verification.verification_entity_id.raw_bytes()[..],
                &first_verification.verifies_relation_id.raw_bytes()[..]
            ],
        )
        .expect("corrupt verifies source endpoint");

    let source_error = engine
        .why(WhyQueryOptions::new(
            WhyQueryTarget::commit(first_verification.commit_id),
            fixture.criterion.acceptance_criterion_entity_id,
        ))
        .expect_err("noncurrent verifies source should be rejected");

    assert_eq!(source_error.code(), ErrorCode::TaskInvalid);
    assert_eq!(source_error.category(), ErrorCategory::Task);

    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let fixture = create_required_criterion(&mut engine, &workspace);
    let first_verification = create_verification(
        &mut engine,
        &workspace,
        fixture.criterion.commit_id,
        VerificationTarget::AcceptanceCriterion(fixture.criterion.acceptance_criterion_entity_id),
        VerificationResult::Passed,
    );
    let later_requirement = create_requirement(
        &mut engine,
        &workspace,
        first_verification.commit_id,
        &fixture.criterion,
        "VR-1",
    );
    let connection = raw_connection(&path);
    connection
        .execute(
            "UPDATE relation
             SET target_object_id = ?1
             WHERE object_id = ?2",
            params![
                &later_requirement
                    .verification_requirement_entity_id
                    .raw_bytes()[..],
                &first_verification.verifies_relation_id.raw_bytes()[..]
            ],
        )
        .expect("corrupt verifies target endpoint");

    let target_error = engine
        .why(WhyQueryOptions::new(
            WhyQueryTarget::commit(first_verification.commit_id),
            first_verification.verification_entity_id,
        ))
        .expect_err("noncurrent verifies target should be rejected");

    assert_eq!(target_error.code(), ErrorCode::TaskInvalid);
    assert_eq!(target_error.category(), ErrorCategory::Task);
}
