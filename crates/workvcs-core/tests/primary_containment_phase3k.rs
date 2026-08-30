use rusqlite::{Connection, params};
use std::path::{Path, PathBuf};
use tempfile::TempDir;
use workvcs_core::{
    BranchId, CanonicalValue, ClaimTaskOptions, CommitId, Digest, Engine, EntityTransitionOptions,
    ErrorCategory, ErrorCode, PlanCreateOptions, PlanSnapshot, PrimaryContainmentCreateOptions,
    PrimaryContainmentEndpointKind, PrimaryContainmentSnapshot, RelationId, RelationVersionId,
    SessionStartOptions, StoreInitOptions, TaskCreateOptions, TaskSchedulingRelationCreateOptions,
    TaskSnapshot, WorkspaceInfo, WorkspaceInitOptions, canonical_bytes, relation_version_digest,
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
        StoreInitOptions::new("phase3k-containment-store").expect("store options"),
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

fn record_state(title: &str) -> CanonicalValue {
    CanonicalValue::object(vec![(
        "title".to_owned(),
        CanonicalValue::String(title.to_owned()),
    )])
    .expect("record state")
}

fn rationale(reason: &str) -> CanonicalValue {
    CanonicalValue::object(vec![(
        "reason".to_owned(),
        CanonicalValue::String(reason.to_owned()),
    )])
    .expect("rationale")
}

fn create_plan_snapshot(
    engine: &mut Engine,
    workspace: &WorkspaceInfo,
    expected_head_commit_id: CommitId,
    description: &str,
) -> PlanSnapshot {
    let plan = engine
        .create_plan(
            PlanCreateOptions::new(
                workspace.initial_branch_id,
                expected_head_commit_id,
                description,
                "Keep the structure explicit",
            )
            .expect("plan options"),
        )
        .expect("create plan");
    engine
        .plan_at(plan.commit_id, plan.plan_entity_id)
        .expect("plan snapshot")
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

#[test]
fn plan_to_task_primary_containment_persists_and_replays_after_reopen() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let plan = create_plan_snapshot(
        &mut engine,
        &workspace,
        workspace.genesis_commit_id,
        "Persistence plan",
    );
    let task = create_task_snapshot(
        &mut engine,
        &workspace,
        plan.commit_id,
        "Benchmark storage shape",
    );
    let connection = raw_connection(&path);
    let before_counts = history_counts(&connection);
    let before_head = branch_head(&connection, workspace.initial_branch_id);

    let commit = engine
        .create_primary_containment(
            PrimaryContainmentCreateOptions::new(
                workspace.initial_branch_id,
                task.commit_id,
                plan.plan_entity_id,
                task.task_entity_id,
            )
            .expect("containment options")
            .with_rationale(rationale("phase3k structure")),
        )
        .expect("create primary containment");

    assert_eq!(before_head, task.commit_id);
    assert_eq!(commit.workspace_id, workspace.workspace_id);
    assert_eq!(commit.branch_id, workspace.initial_branch_id);
    assert_eq!(commit.previous_head_commit_id, task.commit_id);
    assert_eq!(commit.parent_entity_id, plan.plan_entity_id);
    assert_eq!(commit.parent_kind, PrimaryContainmentEndpointKind::Plan);
    assert_eq!(commit.child_entity_id, task.task_entity_id);
    assert_eq!(commit.child_kind, PrimaryContainmentEndpointKind::Task);
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
        .primary_containment_relations_at(commit.commit_id)
        .expect("primary containment relations");
    assert_eq!(relations.len(), 1);
    assert_primary_snapshot(&relations[0], &workspace, commit.commit_id, &commit);

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
    assert_eq!(relation_type, "contains");
    assert_eq!(relation_discriminator, "primary");
    assert_eq!(state_schema_version, 1);
    assert_eq!(metadata_json, "{}");
    assert_eq!(digest_from_blob(state_digest), commit.relation_state_digest);
    assert_eq!(operation_type, "primary_containment.create");
    assert_eq!(operation_schema_version, 1);
    assert_eq!(changeset_payload_json, expected_relation_payload);
    assert_eq!(subject_family, "relation");
    assert_eq!(operation_payload_json, expected_relation_payload);
    assert_eq!(event_kind, "primary_containment.created");
    assert_eq!(event_payload_json, expected_relation_payload);
    drop(connection);
    drop(engine);

    let reopened = Engine::open(&path).expect("reopen engine");
    assert_eq!(
        reopened
            .primary_containment_relations_at(commit.commit_id)
            .expect("reopened relations"),
        relations
    );
}

#[test]
fn actor_session_claim_guard_protects_task_primary_containment_without_partial_rows() {
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

    let plan = create_plan_snapshot(
        &mut engine,
        &workspace,
        workspace.genesis_commit_id,
        "Owned containment plan",
    );
    let task = create_task_snapshot(&mut engine, &workspace, plan.commit_id, "Owned child task");
    engine
        .claim_task(ClaimTaskOptions::new(
            first_session.session_id,
            task.task_entity_id,
        ))
        .expect("claim owned task");
    let allowed = engine
        .create_primary_containment(
            PrimaryContainmentCreateOptions::new(
                workspace.initial_branch_id,
                task.commit_id,
                plan.plan_entity_id,
                task.task_entity_id,
            )
            .expect("allowed containment options")
            .with_actor_session(first_session.session_id),
        )
        .expect("owner creates primary containment");
    assert_eq!(allowed.child_entity_id, task.task_entity_id);

    let blocked_plan = create_plan_snapshot(
        &mut engine,
        &workspace,
        allowed.commit_id,
        "Blocked containment plan",
    );
    let blocked_task = create_task_snapshot(
        &mut engine,
        &workspace,
        blocked_plan.commit_id,
        "Blocked child task",
    );
    engine
        .claim_task(ClaimTaskOptions::new(
            first_session.session_id,
            blocked_task.task_entity_id,
        ))
        .expect("claim blocked task");
    let connection = raw_connection(&path);
    let before_blocked = history_counts(&connection);
    let before_head = branch_head(&connection, workspace.initial_branch_id);

    let blocked = engine
        .create_primary_containment(
            PrimaryContainmentCreateOptions::new(
                workspace.initial_branch_id,
                blocked_task.commit_id,
                blocked_plan.plan_entity_id,
                blocked_task.task_entity_id,
            )
            .expect("blocked containment options")
            .with_actor_session(second_session.session_id),
        )
        .expect_err("other session containment rejected");
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
fn plan_children_can_mix_plans_and_tasks_while_task_can_contain_task() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let parent_plan = create_plan_snapshot(
        &mut engine,
        &workspace,
        workspace.genesis_commit_id,
        "Parent plan",
    );
    let child_plan =
        create_plan_snapshot(&mut engine, &workspace, parent_plan.commit_id, "Child plan");
    let child_task =
        create_task_snapshot(&mut engine, &workspace, child_plan.commit_id, "Child task");
    let parent_task =
        create_task_snapshot(&mut engine, &workspace, child_task.commit_id, "Parent task");
    let subtask = create_task_snapshot(
        &mut engine,
        &workspace,
        parent_task.commit_id,
        "Task subtask",
    );

    let plan_to_plan = engine
        .create_primary_containment(
            PrimaryContainmentCreateOptions::new(
                workspace.initial_branch_id,
                subtask.commit_id,
                parent_plan.plan_entity_id,
                child_plan.plan_entity_id,
            )
            .expect("plan to plan options"),
        )
        .expect("plan contains plan");
    let plan_to_task = engine
        .create_primary_containment(
            PrimaryContainmentCreateOptions::new(
                workspace.initial_branch_id,
                plan_to_plan.commit_id,
                parent_plan.plan_entity_id,
                child_task.task_entity_id,
            )
            .expect("plan to task options"),
        )
        .expect("plan contains task");
    let task_to_task = engine
        .create_primary_containment(
            PrimaryContainmentCreateOptions::new(
                workspace.initial_branch_id,
                plan_to_task.commit_id,
                parent_task.task_entity_id,
                subtask.task_entity_id,
            )
            .expect("task to task options"),
        )
        .expect("task contains task");

    let relations = engine
        .primary_containment_relations_at(task_to_task.commit_id)
        .expect("primary containment relations");
    assert_eq!(relations.len(), 3);
    assert_eq!(
        relations
            .iter()
            .filter(|relation| relation.parent_entity_id == parent_plan.plan_entity_id)
            .count(),
        2
    );
    assert!(relations.iter().any(|relation| {
        relation.parent_entity_id == parent_task.task_entity_id
            && relation.parent_kind == PrimaryContainmentEndpointKind::Task
            && relation.child_entity_id == subtask.task_entity_id
            && relation.child_kind == PrimaryContainmentEndpointKind::Task
    }));
}

#[test]
fn scheduling_relations_are_independent_from_primary_containment() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let plan = create_plan_snapshot(
        &mut engine,
        &workspace,
        workspace.genesis_commit_id,
        "Execution plan",
    );
    let first = create_task_snapshot(&mut engine, &workspace, plan.commit_id, "First task");
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
        .expect("create ordered_before");
    let contains = engine
        .create_primary_containment(
            PrimaryContainmentCreateOptions::new(
                workspace.initial_branch_id,
                order.commit_id,
                plan.plan_entity_id,
                first.task_entity_id,
            )
            .expect("containment options"),
        )
        .expect("create containment");

    let containment = engine
        .primary_containment_relations_at(contains.commit_id)
        .expect("containment relations");
    assert_eq!(containment.len(), 1);
    assert_eq!(containment[0].relation_id, contains.relation_id);

    let scheduling = engine
        .task_scheduling_relations_at(contains.commit_id)
        .expect("scheduling relations");
    assert_eq!(scheduling.len(), 1);
    assert_eq!(scheduling[0].relation_id, order.relation_id);
}

#[test]
fn duplicate_parent_self_edge_and_unsupported_direction_are_rejected_without_partial_rows() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let first_plan = create_plan_snapshot(
        &mut engine,
        &workspace,
        workspace.genesis_commit_id,
        "First plan",
    );
    let second_plan =
        create_plan_snapshot(&mut engine, &workspace, first_plan.commit_id, "Second plan");
    let child_task =
        create_task_snapshot(&mut engine, &workspace, second_plan.commit_id, "Child task");

    let connection = raw_connection(&path);
    let before_self_counts = history_counts(&connection);
    let before_self_head = branch_head(&connection, workspace.initial_branch_id);
    let self_edge = PrimaryContainmentCreateOptions::new(
        workspace.initial_branch_id,
        child_task.commit_id,
        child_task.task_entity_id,
        child_task.task_entity_id,
    )
    .expect_err("self edge options should fail");
    assert_eq!(self_edge.code(), ErrorCode::RelationInvalid);
    assert_counts_and_head_unchanged(
        &path,
        &before_self_counts,
        workspace.initial_branch_id,
        before_self_head,
    );

    let unsupported_direction = engine
        .create_primary_containment(
            PrimaryContainmentCreateOptions::new(
                workspace.initial_branch_id,
                child_task.commit_id,
                child_task.task_entity_id,
                second_plan.plan_entity_id,
            )
            .expect("task to plan options"),
        )
        .expect_err("task cannot contain plan");
    assert_eq!(unsupported_direction.code(), ErrorCode::RelationInvalid);
    assert_eq!(unsupported_direction.category(), ErrorCategory::Relation);

    let first_contains = engine
        .create_primary_containment(
            PrimaryContainmentCreateOptions::new(
                workspace.initial_branch_id,
                child_task.commit_id,
                first_plan.plan_entity_id,
                child_task.task_entity_id,
            )
            .expect("first parent options"),
        )
        .expect("first parent");
    let connection = raw_connection(&path);
    let before_duplicate_counts = history_counts(&connection);
    let before_duplicate_head = branch_head(&connection, workspace.initial_branch_id);

    let duplicate_same_parent = engine
        .create_primary_containment(
            PrimaryContainmentCreateOptions::new(
                workspace.initial_branch_id,
                first_contains.commit_id,
                first_plan.plan_entity_id,
                child_task.task_entity_id,
            )
            .expect("duplicate options"),
        )
        .expect_err("duplicate same parent");
    assert_eq!(duplicate_same_parent.code(), ErrorCode::RelationInvalid);
    assert_counts_and_head_unchanged(
        &path,
        &before_duplicate_counts,
        workspace.initial_branch_id,
        before_duplicate_head,
    );

    let duplicate_other_parent = engine
        .create_primary_containment(
            PrimaryContainmentCreateOptions::new(
                workspace.initial_branch_id,
                first_contains.commit_id,
                second_plan.plan_entity_id,
                child_task.task_entity_id,
            )
            .expect("other parent options"),
        )
        .expect_err("duplicate other parent");
    assert_eq!(duplicate_other_parent.code(), ErrorCode::RelationInvalid);
    assert_counts_and_head_unchanged(
        &path,
        &before_duplicate_counts,
        workspace.initial_branch_id,
        before_duplicate_head,
    );
}

#[test]
fn containment_cycle_is_rejected_without_partial_rows() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let first = create_plan_snapshot(
        &mut engine,
        &workspace,
        workspace.genesis_commit_id,
        "First plan",
    );
    let second = create_plan_snapshot(&mut engine, &workspace, first.commit_id, "Second plan");
    let third = create_plan_snapshot(&mut engine, &workspace, second.commit_id, "Third plan");
    let first_contains_second = engine
        .create_primary_containment(
            PrimaryContainmentCreateOptions::new(
                workspace.initial_branch_id,
                third.commit_id,
                first.plan_entity_id,
                second.plan_entity_id,
            )
            .expect("first edge options"),
        )
        .expect("first edge");
    let second_contains_third = engine
        .create_primary_containment(
            PrimaryContainmentCreateOptions::new(
                workspace.initial_branch_id,
                first_contains_second.commit_id,
                second.plan_entity_id,
                third.plan_entity_id,
            )
            .expect("second edge options"),
        )
        .expect("second edge");
    let connection = raw_connection(&path);
    let before_counts = history_counts(&connection);
    let before_head = branch_head(&connection, workspace.initial_branch_id);

    let cycle = engine
        .create_primary_containment(
            PrimaryContainmentCreateOptions::new(
                workspace.initial_branch_id,
                second_contains_third.commit_id,
                third.plan_entity_id,
                first.plan_entity_id,
            )
            .expect("cycle options"),
        )
        .expect_err("cycle should be rejected");
    assert_eq!(cycle.code(), ErrorCode::RelationInvalid);
    assert!(cycle.to_string().contains("containment cycle"));
    assert_eq!(history_counts(&connection), before_counts);
    assert_eq!(
        branch_head(&connection, workspace.initial_branch_id),
        before_head
    );
}

#[test]
fn unsupported_and_cross_workspace_endpoints_are_rejected_without_partial_rows() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let plan = create_plan_snapshot(&mut engine, &workspace, workspace.genesis_commit_id, "Plan");
    let task = create_task_snapshot(&mut engine, &workspace, plan.commit_id, "Task");
    let record = engine
        .commit_entity_transition(
            EntityTransitionOptions::create(
                workspace.initial_branch_id,
                task.commit_id,
                "generic_record",
                record_state("not containable"),
            )
            .expect("record options"),
        )
        .expect("record transition");
    let connection = raw_connection(&path);
    let before_record_counts = history_counts(&connection);
    let before_record_head = branch_head(&connection, workspace.initial_branch_id);

    let unsupported = engine
        .create_primary_containment(
            PrimaryContainmentCreateOptions::new(
                workspace.initial_branch_id,
                record.commit_id,
                plan.plan_entity_id,
                record.entity_id,
            )
            .expect("unsupported endpoint options"),
        )
        .expect_err("record endpoint rejected");
    assert_eq!(unsupported.code(), ErrorCode::RelationInvalid);
    assert_counts_and_head_unchanged(
        &path,
        &before_record_counts,
        workspace.initial_branch_id,
        before_record_head,
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
        .create_primary_containment(
            PrimaryContainmentCreateOptions::new(
                workspace.initial_branch_id,
                record.commit_id,
                plan.plan_entity_id,
                other_task.task_entity_id,
            )
            .expect("cross workspace options"),
        )
        .expect_err("cross workspace rejected");
    assert_eq!(cross_workspace.code(), ErrorCode::RelationInvalid);
    assert_counts_and_head_unchanged(
        &path,
        &before_cross_counts,
        workspace.initial_branch_id,
        before_cross_head,
    );
}

#[test]
fn stale_head_rolls_back_without_partial_containment_rows() {
    let (_tempdir, path) = store_path();
    let (mut first_engine, workspace) = create_workspace(&path);
    let plan = create_plan_snapshot(
        &mut first_engine,
        &workspace,
        workspace.genesis_commit_id,
        "Plan",
    );
    let task = create_task_snapshot(&mut first_engine, &workspace, plan.commit_id, "Task");
    let mut second_engine = Engine::open(&path).expect("second engine");

    let first_seen_head = first_engine
        .branch_head(workspace.initial_branch_id)
        .expect("first branch head")
        .head_commit_id;
    let second_seen_head = second_engine
        .branch_head(workspace.initial_branch_id)
        .expect("second branch head")
        .head_commit_id;
    assert_eq!(first_seen_head, task.commit_id);
    assert_eq!(second_seen_head, task.commit_id);

    let winner = first_engine
        .create_primary_containment(
            PrimaryContainmentCreateOptions::new(
                workspace.initial_branch_id,
                first_seen_head,
                plan.plan_entity_id,
                task.task_entity_id,
            )
            .expect("winner options"),
        )
        .expect("winner containment");

    let connection = raw_connection(&path);
    let before_loser = history_counts(&connection);
    drop(connection);

    let error = second_engine
        .create_primary_containment(
            PrimaryContainmentCreateOptions::new(
                workspace.initial_branch_id,
                second_seen_head,
                plan.plan_entity_id,
                task.task_entity_id,
            )
            .expect("loser options"),
        )
        .expect_err("stale loser");
    assert_eq!(error.code(), ErrorCode::BranchHeadConflict);
    assert_eq!(error.category(), ErrorCategory::Mutation);
    assert!(error.retryable());

    let connection = raw_connection(&path);
    assert_eq!(history_counts(&connection), before_loser);
    assert_eq!(
        branch_head(&connection, workspace.initial_branch_id),
        winner.commit_id
    );
}

fn assert_primary_snapshot(
    relation: &PrimaryContainmentSnapshot,
    workspace: &WorkspaceInfo,
    commit_id: CommitId,
    commit: &workvcs_core::PrimaryContainmentCreateCommit,
) {
    assert_eq!(relation.workspace_id, workspace.workspace_id);
    assert_eq!(relation.commit_id, commit_id);
    assert_eq!(relation.relation_id, commit.relation_id);
    assert_eq!(relation.relation_version_id, commit.relation_version_id);
    assert_eq!(relation.parent_entity_id, commit.parent_entity_id);
    assert_eq!(relation.parent_kind, commit.parent_kind);
    assert_eq!(relation.child_entity_id, commit.child_entity_id);
    assert_eq!(relation.child_kind, commit.child_kind);
    assert_eq!(
        relation.state_digest,
        relation_version_digest(&empty_relation_state()).expect("relation digest")
    );
}
