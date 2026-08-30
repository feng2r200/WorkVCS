use rusqlite::{Connection, params};
use std::path::{Path, PathBuf};
use tempfile::TempDir;
use workvcs_core::{
    AcceptanceCriterionClassification, AcceptanceCriterionCreateOptions,
    AcceptanceCriterionEffectiveStatus, BranchId, CanonicalValue, ClaimTaskOptions, CommitId,
    Digest, Engine, ErrorCategory, ErrorCode, RelationId, RelationVersionId, SessionStartOptions,
    StoreInitOptions, TaskCreateOptions, TaskSchedulingRelationCreateOptions,
    TaskSchedulingRelationSnapshot, TaskSchedulingRelationType, TaskSnapshot,
    VerificationCreateOptions, VerificationResult, VerificationTarget, WorkspaceInfo,
    WorkspaceInitOptions, canonical_bytes, relation_version_digest,
};

#[derive(Clone, Debug, PartialEq, Eq)]
struct HistoryCounts {
    object_identity: i64,
    entity: i64,
    relation: i64,
    relation_version: i64,
    entity_version: i64,
    changeset: i64,
    change_operation: i64,
    entity_membership_change: i64,
    relation_membership_change: i64,
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
        StoreInitOptions::new("phase3h-task-relation-store").expect("store options"),
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

fn history_counts(connection: &Connection) -> HistoryCounts {
    HistoryCounts {
        object_identity: count_rows(connection, "object_identity"),
        entity: count_rows(connection, "entity"),
        relation: count_rows(connection, "relation"),
        relation_version: count_rows(connection, "relation_version"),
        entity_version: count_rows(connection, "entity_version"),
        changeset: count_rows(connection, "changeset"),
        change_operation: count_rows(connection, "change_operation"),
        entity_membership_change: count_rows(connection, "entity_membership_change"),
        relation_membership_change: count_rows(connection, "relation_membership_change"),
        workstate_commit: count_rows(connection, "workstate_commit"),
        commit_parent: count_rows(connection, "commit_parent"),
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

fn digest_from_blob(bytes: Vec<u8>) -> Digest {
    Digest::from_bytes(bytes.try_into().expect("digest bytes"))
}

fn canonical_json(value: &CanonicalValue) -> String {
    String::from_utf8(canonical_bytes(value).expect("canonical bytes")).expect("canonical json")
}

fn empty_relation_state() -> CanonicalValue {
    CanonicalValue::object(Vec::new()).expect("empty relation state")
}

fn relation_payload_json(
    relation_id: RelationId,
    relation_version_id: RelationVersionId,
) -> String {
    canonical_json(
        &CanonicalValue::object(vec![
            (
                "after_relation_version_id".to_owned(),
                CanonicalValue::String(relation_version_id.to_string()),
            ),
            (
                "before_relation_version_id".to_owned(),
                CanonicalValue::Null,
            ),
            (
                "relation_id".to_owned(),
                CanonicalValue::String(relation_id.to_string()),
            ),
        ])
        .expect("relation payload"),
    )
}

fn rationale(reason: &str) -> CanonicalValue {
    CanonicalValue::object(vec![(
        "reason".to_owned(),
        CanonicalValue::String(reason.to_owned()),
    )])
    .expect("rationale")
}

fn create_task_snapshot(
    engine: &mut Engine,
    workspace: &WorkspaceInfo,
    expected_head_commit_id: CommitId,
    task_description: &str,
) -> TaskSnapshot {
    let task = engine
        .create_task(
            TaskCreateOptions::new(
                workspace.initial_branch_id,
                expected_head_commit_id,
                task_description,
            )
            .expect("task options"),
        )
        .expect("create task");
    engine
        .task_at(task.commit_id, task.task_entity_id)
        .expect("task snapshot")
}

fn assert_relation_create_delta(before: &HistoryCounts, after: &HistoryCounts) {
    assert_eq!(after.object_identity, before.object_identity + 1);
    assert_eq!(after.entity, before.entity);
    assert_eq!(after.relation, before.relation + 1);
    assert_eq!(after.relation_version, before.relation_version + 1);
    assert_eq!(after.entity_version, before.entity_version);
    assert_eq!(after.changeset, before.changeset + 1);
    assert_eq!(after.change_operation, before.change_operation + 1);
    assert_eq!(
        after.entity_membership_change,
        before.entity_membership_change
    );
    assert_eq!(
        after.relation_membership_change,
        before.relation_membership_change + 1
    );
    assert_eq!(after.workstate_commit, before.workstate_commit + 1);
    assert_eq!(after.commit_parent, before.commit_parent + 1);
    assert_eq!(after.event, before.event + 1);
}

fn assert_counts_and_head_unchanged(
    path: &Path,
    before_counts: &HistoryCounts,
    branch_id: BranchId,
    before_head: CommitId,
) {
    let connection = raw_connection(path);
    assert_eq!(history_counts(&connection), *before_counts);
    assert_eq!(branch_head(&connection, branch_id), before_head);
}

fn assert_relation_snapshot(
    relation: &TaskSchedulingRelationSnapshot,
    workspace: &WorkspaceInfo,
    commit_id: CommitId,
    relation_type: TaskSchedulingRelationType,
    source: &TaskSnapshot,
    target: &TaskSnapshot,
) {
    assert_eq!(relation.workspace_id, workspace.workspace_id);
    assert_eq!(relation.commit_id, commit_id);
    assert_eq!(relation.relation_type, relation_type);
    assert_eq!(relation.source_task_entity_id, source.task_entity_id);
    assert_eq!(relation.target_task_entity_id, target.task_entity_id);
    assert_eq!(
        relation.state_digest,
        relation_version_digest(&empty_relation_state()).expect("relation digest")
    );
}

#[test]
fn depends_on_relation_persists_and_replays_after_reopen() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let prerequisite = create_task_snapshot(
        &mut engine,
        &workspace,
        workspace.genesis_commit_id,
        "Build the prerequisite",
    );
    let dependent = create_task_snapshot(
        &mut engine,
        &workspace,
        prerequisite.commit_id,
        "Run the dependent work",
    );
    let connection = raw_connection(&path);
    let before_counts = history_counts(&connection);
    let before_head = branch_head(&connection, workspace.initial_branch_id);

    let commit = engine
        .create_task_scheduling_relation(
            TaskSchedulingRelationCreateOptions::depends_on(
                workspace.initial_branch_id,
                dependent.commit_id,
                dependent.task_entity_id,
                prerequisite.task_entity_id,
            )
            .expect("relation options")
            .with_rationale(rationale("phase3h dependency")),
        )
        .expect("create depends_on relation");

    assert_eq!(before_head, dependent.commit_id);
    assert_eq!(commit.workspace_id, workspace.workspace_id);
    assert_eq!(commit.branch_id, workspace.initial_branch_id);
    assert_eq!(commit.previous_head_commit_id, dependent.commit_id);
    assert_eq!(commit.relation_type, TaskSchedulingRelationType::DependsOn);
    assert_eq!(commit.source_task_entity_id, dependent.task_entity_id);
    assert_eq!(commit.target_task_entity_id, prerequisite.task_entity_id);
    assert_eq!(
        commit.relation_state_digest,
        relation_version_digest(&empty_relation_state()).expect("relation digest")
    );

    let after_counts = history_counts(&connection);
    assert_relation_create_delta(&before_counts, &after_counts);
    assert_eq!(
        branch_head(&connection, workspace.initial_branch_id),
        commit.commit_id
    );

    let replayed = engine.state_at(commit.commit_id).expect("replayed state");
    assert_eq!(
        replayed.state.relations(),
        &[(commit.relation_id, commit.relation_version_id)]
    );
    assert_eq!(replayed.state_digest, commit.work_state_digest);

    let relations = engine
        .task_scheduling_relations_at(commit.commit_id)
        .expect("task scheduling relations");
    assert_eq!(relations.len(), 1);
    assert_relation_snapshot(
        &relations[0],
        &workspace,
        commit.commit_id,
        TaskSchedulingRelationType::DependsOn,
        &dependent,
        &prerequisite,
    );

    let (
        object_kind,
        relation_type,
        relation_discriminator,
        state_schema_version,
        metadata_json,
        state_digest,
        operation_type,
        operation_schema_version,
        changeset_payload_json,
        subject_family,
        operation_payload_json,
        event_kind,
        event_payload_json,
    ) = connection
        .query_row(
            "SELECT object_identity.object_kind,
                    relation.relation_type,
                    relation.relation_discriminator,
                    relation_version.state_schema_version,
                    relation_version.metadata_json,
                    relation_version.state_digest,
                    changeset.operation_type,
                    changeset.operation_schema_version,
                    changeset.operation_payload_json,
                    change_operation.subject_family,
                    change_operation.operation_payload_json,
                    event.event_kind,
                    event.payload_json
             FROM relation
             JOIN object_identity
               ON object_identity.object_id = relation.object_id
             JOIN relation_version
               ON relation_version.relation_id = relation.object_id
             JOIN relation_membership_change
               ON relation_membership_change.after_relation_version_id = relation_version.relation_version_id
             JOIN change_operation
               ON change_operation.operation_id = relation_membership_change.operation_id
             JOIN changeset
               ON changeset.changeset_id = change_operation.changeset_id
             JOIN event
               ON event.changeset_id = changeset.changeset_id
             WHERE relation.object_id = ?1
               AND relation_version.relation_version_id = ?2",
            params![
                &commit.relation_id.raw_bytes()[..],
                &commit.relation_version_id.raw_bytes()[..]
            ],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, Vec<u8>>(5)?,
                    row.get::<_, String>(6)?,
                    row.get::<_, i64>(7)?,
                    row.get::<_, String>(8)?,
                    row.get::<_, String>(9)?,
                    row.get::<_, String>(10)?,
                    row.get::<_, String>(11)?,
                    row.get::<_, String>(12)?,
                ))
            },
        )
        .expect("relation storage shape");
    let expected_relation_payload =
        relation_payload_json(commit.relation_id, commit.relation_version_id);
    assert_eq!(object_kind, "relation");
    assert_eq!(relation_type, "depends_on");
    assert_eq!(relation_discriminator, "");
    assert_eq!(state_schema_version, 1);
    assert_eq!(metadata_json, "{}");
    assert_eq!(digest_from_blob(state_digest), commit.relation_state_digest);
    assert_eq!(operation_type, "task.scheduling_relation.create");
    assert_eq!(operation_schema_version, 1);
    assert_eq!(changeset_payload_json, expected_relation_payload);
    assert_eq!(subject_family, "relation");
    assert_eq!(operation_payload_json, expected_relation_payload);
    assert_eq!(event_kind, "task.scheduling_relation.created");
    assert_eq!(event_payload_json, expected_relation_payload);
    drop(connection);
    drop(engine);

    let reopened = Engine::open(&path).expect("reopen engine");
    assert_eq!(
        reopened
            .task_scheduling_relations_at(commit.commit_id)
            .expect("reopened relations"),
        relations
    );
}

#[test]
fn ordered_before_is_independent_from_depends_on() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let first = create_task_snapshot(
        &mut engine,
        &workspace,
        workspace.genesis_commit_id,
        "First task",
    );
    let second = create_task_snapshot(&mut engine, &workspace, first.commit_id, "Second task");

    let order = engine
        .create_task_scheduling_relation(
            TaskSchedulingRelationCreateOptions::ordered_before(
                workspace.initial_branch_id,
                second.commit_id,
                first.task_entity_id,
                second.task_entity_id,
            )
            .expect("order options"),
        )
        .expect("create ordered_before relation");
    let dependency = engine
        .create_task_scheduling_relation(
            TaskSchedulingRelationCreateOptions::depends_on(
                workspace.initial_branch_id,
                order.commit_id,
                first.task_entity_id,
                second.task_entity_id,
            )
            .expect("dependency options"),
        )
        .expect("create depends_on relation with same endpoints");

    let relations = engine
        .task_scheduling_relations_at(dependency.commit_id)
        .expect("relations at dependency commit");
    assert_eq!(relations.len(), 2);
    assert_relation_snapshot(
        &relations[0],
        &workspace,
        dependency.commit_id,
        TaskSchedulingRelationType::DependsOn,
        &first,
        &second,
    );
    assert_relation_snapshot(
        &relations[1],
        &workspace,
        dependency.commit_id,
        TaskSchedulingRelationType::OrderedBefore,
        &first,
        &second,
    );
    assert_eq!(relations[0].relation_id, dependency.relation_id);
    assert_eq!(relations[1].relation_id, order.relation_id);
}

#[test]
fn actor_session_claim_guard_protects_task_scheduling_relation_without_partial_rows() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let first_session = engine
        .start_session(
            SessionStartOptions::new(workspace.workspace_id, workspace.initial_branch_id)
                .expect("first session options"),
        )
        .expect("start first session");
    let second_session = engine
        .start_session(
            SessionStartOptions::new(workspace.workspace_id, workspace.initial_branch_id)
                .expect("second session options"),
        )
        .expect("start second session");

    let prerequisite = create_task_snapshot(
        &mut engine,
        &workspace,
        workspace.genesis_commit_id,
        "Owned prerequisite",
    );
    let dependent = create_task_snapshot(
        &mut engine,
        &workspace,
        prerequisite.commit_id,
        "Owned dependent",
    );
    engine
        .claim_task(ClaimTaskOptions::new(
            first_session.session_id,
            dependent.task_entity_id,
        ))
        .expect("claim owned dependent");
    let allowed = engine
        .create_task_scheduling_relation(
            TaskSchedulingRelationCreateOptions::depends_on(
                workspace.initial_branch_id,
                dependent.commit_id,
                dependent.task_entity_id,
                prerequisite.task_entity_id,
            )
            .expect("allowed relation options")
            .with_actor_session(first_session.session_id),
        )
        .expect("owner creates scheduling relation");
    assert_eq!(allowed.relation_type, TaskSchedulingRelationType::DependsOn);

    let blocked_source =
        create_task_snapshot(&mut engine, &workspace, allowed.commit_id, "Blocked source");
    let blocked_target = create_task_snapshot(
        &mut engine,
        &workspace,
        blocked_source.commit_id,
        "Blocked target",
    );
    engine
        .claim_task(ClaimTaskOptions::new(
            first_session.session_id,
            blocked_source.task_entity_id,
        ))
        .expect("claim blocked source");
    let connection = raw_connection(&path);
    let before_blocked = history_counts(&connection);
    let before_head = branch_head(&connection, workspace.initial_branch_id);

    let blocked = engine
        .create_task_scheduling_relation(
            TaskSchedulingRelationCreateOptions::ordered_before(
                workspace.initial_branch_id,
                blocked_target.commit_id,
                blocked_source.task_entity_id,
                blocked_target.task_entity_id,
            )
            .expect("blocked relation options")
            .with_actor_session(second_session.session_id),
        )
        .expect_err("other session scheduling relation rejected");
    assert_eq!(blocked.code(), ErrorCode::ClaimInvalid);
    assert_eq!(blocked.category(), ErrorCategory::Runtime);
    assert!(
        blocked
            .to_string()
            .contains("exclusive_claim_owned_by_other_session")
    );
    assert_eq!(history_counts(&connection), before_blocked);
    assert_eq!(
        branch_head(&connection, workspace.initial_branch_id),
        before_head
    );
}

#[test]
fn depends_on_cycle_is_rejected_without_partial_rows() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let first = create_task_snapshot(
        &mut engine,
        &workspace,
        workspace.genesis_commit_id,
        "First task",
    );
    let second = create_task_snapshot(&mut engine, &workspace, first.commit_id, "Second task");
    let third = create_task_snapshot(&mut engine, &workspace, second.commit_id, "Third task");
    let first_depends_on_second = engine
        .create_task_scheduling_relation(
            TaskSchedulingRelationCreateOptions::depends_on(
                workspace.initial_branch_id,
                third.commit_id,
                first.task_entity_id,
                second.task_entity_id,
            )
            .expect("first dependency options"),
        )
        .expect("create first dependency");
    let second_depends_on_third = engine
        .create_task_scheduling_relation(
            TaskSchedulingRelationCreateOptions::depends_on(
                workspace.initial_branch_id,
                first_depends_on_second.commit_id,
                second.task_entity_id,
                third.task_entity_id,
            )
            .expect("second dependency options"),
        )
        .expect("create second dependency");
    let connection = raw_connection(&path);
    let before_counts = history_counts(&connection);
    let before_head = branch_head(&connection, workspace.initial_branch_id);

    let cycle = engine
        .create_task_scheduling_relation(
            TaskSchedulingRelationCreateOptions::depends_on(
                workspace.initial_branch_id,
                second_depends_on_third.commit_id,
                third.task_entity_id,
                first.task_entity_id,
            )
            .expect("cycle options"),
        )
        .expect_err("dependency cycle should be rejected");
    assert_eq!(cycle.code(), ErrorCode::TaskInvalid);
    assert!(cycle.to_string().contains("dependency cycle"));

    assert_eq!(history_counts(&connection), before_counts);
    assert_eq!(
        branch_head(&connection, workspace.initial_branch_id),
        before_head
    );
}

#[test]
fn invalid_endpoints_are_rejected_without_partial_rows() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let first = create_task_snapshot(
        &mut engine,
        &workspace,
        workspace.genesis_commit_id,
        "First task",
    );
    let second = create_task_snapshot(&mut engine, &workspace, first.commit_id, "Second task");
    let connection = raw_connection(&path);
    let before_self_counts = history_counts(&connection);
    let before_self_head = branch_head(&connection, workspace.initial_branch_id);

    let self_edge = engine
        .create_task_scheduling_relation(
            TaskSchedulingRelationCreateOptions::depends_on(
                workspace.initial_branch_id,
                second.commit_id,
                first.task_entity_id,
                first.task_entity_id,
            )
            .expect("self edge options"),
        )
        .expect_err("self edge should be rejected");
    assert_eq!(self_edge.code(), ErrorCode::TaskInvalid);
    assert_counts_and_head_unchanged(
        &path,
        &before_self_counts,
        workspace.initial_branch_id,
        before_self_head,
    );

    let criterion = engine
        .create_acceptance_criterion(
            AcceptanceCriterionCreateOptions::new(
                workspace.initial_branch_id,
                second.commit_id,
                first.task_entity_id,
                first.task_entity_version_id,
                "AC-task-relation",
                "A non-task entity cannot be a scheduling endpoint.",
                AcceptanceCriterionClassification::Required,
            )
            .expect("criterion options"),
        )
        .expect("create acceptance criterion");
    let connection = raw_connection(&path);
    let before_non_task_counts = history_counts(&connection);
    let before_non_task_head = branch_head(&connection, workspace.initial_branch_id);
    let non_task = engine
        .create_task_scheduling_relation(
            TaskSchedulingRelationCreateOptions::depends_on(
                workspace.initial_branch_id,
                criterion.commit_id,
                first.task_entity_id,
                criterion.acceptance_criterion_entity_id,
            )
            .expect("non-task options"),
        )
        .expect_err("non-task endpoint should be rejected");
    assert_eq!(non_task.code(), ErrorCode::TaskNotFound);
    assert_counts_and_head_unchanged(
        &path,
        &before_non_task_counts,
        workspace.initial_branch_id,
        before_non_task_head,
    );

    let other_workspace = engine
        .create_workspace(WorkspaceInitOptions::new("other").expect("other workspace options"))
        .expect("create other workspace");
    let other_task = create_task_snapshot(
        &mut engine,
        &other_workspace,
        other_workspace.genesis_commit_id,
        "Other workspace task",
    );
    let connection = raw_connection(&path);
    let before_cross_counts = history_counts(&connection);
    let before_cross_head = branch_head(&connection, workspace.initial_branch_id);
    let cross_workspace = engine
        .create_task_scheduling_relation(
            TaskSchedulingRelationCreateOptions::depends_on(
                workspace.initial_branch_id,
                criterion.commit_id,
                first.task_entity_id,
                other_task.task_entity_id,
            )
            .expect("cross-workspace options"),
        )
        .expect_err("cross-workspace endpoint should be rejected");
    assert_eq!(cross_workspace.code(), ErrorCode::TaskNotFound);
    assert_counts_and_head_unchanged(
        &path,
        &before_cross_counts,
        workspace.initial_branch_id,
        before_cross_head,
    );
}

#[test]
fn duplicate_relation_and_stale_head_are_rejected_without_partial_rows() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let prerequisite = create_task_snapshot(
        &mut engine,
        &workspace,
        workspace.genesis_commit_id,
        "Prerequisite",
    );
    let dependent =
        create_task_snapshot(&mut engine, &workspace, prerequisite.commit_id, "Dependent");
    let dependency = engine
        .create_task_scheduling_relation(
            TaskSchedulingRelationCreateOptions::depends_on(
                workspace.initial_branch_id,
                dependent.commit_id,
                dependent.task_entity_id,
                prerequisite.task_entity_id,
            )
            .expect("dependency options"),
        )
        .expect("create dependency");
    let connection = raw_connection(&path);
    let before_duplicate_counts = history_counts(&connection);
    let before_duplicate_head = branch_head(&connection, workspace.initial_branch_id);

    let duplicate = engine
        .create_task_scheduling_relation(
            TaskSchedulingRelationCreateOptions::depends_on(
                workspace.initial_branch_id,
                dependency.commit_id,
                dependent.task_entity_id,
                prerequisite.task_entity_id,
            )
            .expect("duplicate options"),
        )
        .expect_err("duplicate relation should be rejected");
    assert_eq!(duplicate.code(), ErrorCode::TaskInvalid);
    assert!(duplicate.to_string().contains("already exists"));
    assert_counts_and_head_unchanged(
        &path,
        &before_duplicate_counts,
        workspace.initial_branch_id,
        before_duplicate_head,
    );

    let stale_head = dependency.commit_id;
    let unrelated = create_task_snapshot(&mut engine, &workspace, stale_head, "Unrelated");
    let connection = raw_connection(&path);
    let before_stale_counts = history_counts(&connection);
    let before_stale_head = branch_head(&connection, workspace.initial_branch_id);
    let stale = engine
        .create_task_scheduling_relation(
            TaskSchedulingRelationCreateOptions::ordered_before(
                workspace.initial_branch_id,
                stale_head,
                prerequisite.task_entity_id,
                dependent.task_entity_id,
            )
            .expect("stale options"),
        )
        .expect_err("stale head should be rejected");
    assert_eq!(stale.code(), ErrorCode::BranchHeadConflict);
    assert_eq!(unrelated.commit_id, before_stale_head);
    assert_counts_and_head_unchanged(
        &path,
        &before_stale_counts,
        workspace.initial_branch_id,
        before_stale_head,
    );
}

#[test]
fn scheduling_relations_do_not_pollute_verification_projection() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let prerequisite = create_task_snapshot(
        &mut engine,
        &workspace,
        workspace.genesis_commit_id,
        "Prerequisite",
    );
    let dependent =
        create_task_snapshot(&mut engine, &workspace, prerequisite.commit_id, "Dependent");
    let criterion = engine
        .create_acceptance_criterion(
            AcceptanceCriterionCreateOptions::new(
                workspace.initial_branch_id,
                dependent.commit_id,
                dependent.task_entity_id,
                dependent.task_entity_version_id,
                "AC-verification-projection",
                "Verification projection must ignore scheduling relations.",
                AcceptanceCriterionClassification::Required,
            )
            .expect("criterion options"),
        )
        .expect("create criterion");
    let verification = engine
        .create_verification(
            VerificationCreateOptions::new(
                workspace.initial_branch_id,
                criterion.commit_id,
                VerificationTarget::AcceptanceCriterion(criterion.acceptance_criterion_entity_id),
                VerificationResult::Passed,
            )
            .expect("verification options"),
        )
        .expect("record verification");
    let relation = engine
        .create_task_scheduling_relation(
            TaskSchedulingRelationCreateOptions::depends_on(
                workspace.initial_branch_id,
                verification.commit_id,
                dependent.task_entity_id,
                prerequisite.task_entity_id,
            )
            .expect("relation options"),
        )
        .expect("create scheduling relation");

    assert_eq!(
        engine
            .acceptance_criterion_effective_status(
                relation.commit_id,
                criterion.acceptance_criterion_entity_id,
            )
            .expect("effective status"),
        AcceptanceCriterionEffectiveStatus::Verified
    );
    assert_eq!(
        engine
            .verification_at(relation.commit_id, verification.verification_entity_id)
            .expect("verification snapshot")
            .target,
        VerificationTarget::AcceptanceCriterion(criterion.acceptance_criterion_entity_id)
    );
    assert_eq!(
        engine
            .task_scheduling_relations_at(relation.commit_id)
            .expect("scheduling relations")
            .len(),
        1
    );
}
