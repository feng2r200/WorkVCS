use rusqlite::{Connection, params};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use tempfile::TempDir;
use workvcs_core::{
    BranchId, CanonicalValue, CommitId, Engine, EntityId, EntityTransitionCommit,
    EntityTransitionOptions, ErrorCategory, ErrorCode, EventId, PlanCreateOptions, PlanSnapshot,
    StoreInitOptions, StructuralReferenceCreateCommit, StructuralReferenceCreateOptions,
    TaskCreateOptions, TaskSnapshot, WorkStateDiff, WorkStateDiffChangeKind, WorkStateDiffOptions,
    WorkStateDiffTarget, WorkspaceInfo, WorkspaceInitOptions,
};

#[derive(Clone, Debug, PartialEq, Eq)]
struct QueryCounts {
    object_identity: i64,
    entity: i64,
    relation: i64,
    entity_version: i64,
    relation_version: i64,
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
        StoreInitOptions::new("phase3s-workstate-diff-store").expect("store options"),
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
        object_identity: count_rows(connection, "object_identity"),
        entity: count_rows(connection, "entity"),
        relation: count_rows(connection, "relation"),
        entity_version: count_rows(connection, "entity_version"),
        relation_version: count_rows(connection, "relation_version"),
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

fn record_state(title: &str, status: &str) -> CanonicalValue {
    CanonicalValue::object(vec![
        ("title".to_owned(), CanonicalValue::String(title.to_owned())),
        (
            "status".to_owned(),
            CanonicalValue::String(status.to_owned()),
        ),
    ])
    .expect("record state")
}

fn create_record(
    engine: &mut Engine,
    workspace: &WorkspaceInfo,
    expected_head_commit_id: CommitId,
    title: &str,
) -> EntityTransitionCommit {
    engine
        .commit_entity_transition(
            EntityTransitionOptions::create(
                workspace.initial_branch_id,
                expected_head_commit_id,
                "generic_record",
                record_state(title, "open"),
            )
            .expect("create options"),
        )
        .expect("create record")
}

fn update_record(
    engine: &mut Engine,
    workspace: &WorkspaceInfo,
    current: &EntityTransitionCommit,
    status: &str,
) -> EntityTransitionCommit {
    engine
        .commit_entity_transition(
            EntityTransitionOptions::update(
                workspace.initial_branch_id,
                current.commit_id,
                current.entity_id,
                current.entity_version_id,
                record_state("generic_record", status),
            )
            .expect("update options"),
        )
        .expect("update record")
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
                "Keep diff inputs explicit",
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
    description: &str,
) -> TaskSnapshot {
    let task = engine
        .create_task(
            TaskCreateOptions::new(
                workspace.initial_branch_id,
                expected_head_commit_id,
                description,
            )
            .expect("task options"),
        )
        .expect("create task");
    engine
        .task_at(task.commit_id, task.task_entity_id)
        .expect("task snapshot")
}

fn reference(
    engine: &mut Engine,
    workspace: &WorkspaceInfo,
    expected_head_commit_id: CommitId,
    referrer_entity_id: EntityId,
    target_entity_id: EntityId,
) -> StructuralReferenceCreateCommit {
    engine
        .create_structural_reference(
            StructuralReferenceCreateOptions::new(
                workspace.initial_branch_id,
                expected_head_commit_id,
                referrer_entity_id,
                target_entity_id,
            )
            .expect("reference options"),
        )
        .expect("create structural reference")
}

fn diff_commits(engine: &Engine, from: CommitId, to: CommitId) -> WorkStateDiff {
    engine
        .diff(WorkStateDiffOptions::new(
            WorkStateDiffTarget::commit(from),
            WorkStateDiffTarget::commit(to),
        ))
        .expect("diff commits")
}

#[test]
fn diff_reports_added_updated_and_removed_entity_versions() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let first = create_record(
        &mut engine,
        &workspace,
        workspace.genesis_commit_id,
        "generic_record",
    );
    let second = update_record(&mut engine, &workspace, &first, "done");

    let added = diff_commits(&engine, workspace.genesis_commit_id, first.commit_id);
    assert_eq!(added.from.commit_id, workspace.genesis_commit_id);
    assert_eq!(added.to.commit_id, first.commit_id);
    assert_eq!(added.from.workspace_id, workspace.workspace_id);
    assert_eq!(added.entity_changes.len(), 1);
    assert_eq!(added.entity_changes[0].entity_id, first.entity_id);
    assert_eq!(
        added.entity_changes[0].change_kind,
        WorkStateDiffChangeKind::Added
    );
    assert_eq!(added.entity_changes[0].before_entity_version_id, None);
    assert_eq!(
        added.entity_changes[0].after_entity_version_id,
        Some(first.entity_version_id)
    );
    assert!(added.relation_changes.is_empty());

    let updated = diff_commits(&engine, first.commit_id, second.commit_id);
    assert_eq!(updated.entity_changes.len(), 1);
    assert_eq!(updated.entity_changes[0].entity_id, first.entity_id);
    assert_eq!(
        updated.entity_changes[0].change_kind,
        WorkStateDiffChangeKind::Updated
    );
    assert_eq!(
        updated.entity_changes[0].before_entity_version_id,
        Some(first.entity_version_id)
    );
    assert_eq!(
        updated.entity_changes[0].after_entity_version_id,
        Some(second.entity_version_id)
    );
    assert!(updated.relation_changes.is_empty());

    let removed = diff_commits(&engine, first.commit_id, workspace.genesis_commit_id);
    assert_eq!(removed.entity_changes.len(), 1);
    assert_eq!(removed.entity_changes[0].entity_id, first.entity_id);
    assert_eq!(
        removed.entity_changes[0].change_kind,
        WorkStateDiffChangeKind::Removed
    );
    assert_eq!(
        removed.entity_changes[0].before_entity_version_id,
        Some(first.entity_version_id)
    );
    assert_eq!(removed.entity_changes[0].after_entity_version_id, None);
    assert!(removed.relation_changes.is_empty());
}

#[test]
fn diff_reports_added_and_removed_relation_versions_without_semantic_inference() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let plan = create_plan_snapshot(
        &mut engine,
        &workspace,
        workspace.genesis_commit_id,
        "Diff relation plan",
    );
    let task = create_task_snapshot(&mut engine, &workspace, plan.commit_id, "Referenced task");
    let structural_reference = reference(
        &mut engine,
        &workspace,
        task.commit_id,
        plan.plan_entity_id,
        task.task_entity_id,
    );
    let before_reference = task.commit_id;
    let after_reference = structural_reference.commit_id;

    let added = diff_commits(&engine, before_reference, after_reference);
    assert!(added.entity_changes.is_empty());
    assert_eq!(added.relation_changes.len(), 1);
    assert_eq!(
        added.relation_changes[0].relation_id,
        structural_reference.relation_id
    );
    assert_eq!(
        added.relation_changes[0].change_kind,
        WorkStateDiffChangeKind::Added
    );
    assert_eq!(added.relation_changes[0].before_relation_version_id, None);
    assert_eq!(
        added.relation_changes[0].after_relation_version_id,
        Some(structural_reference.relation_version_id)
    );

    let removed = diff_commits(&engine, after_reference, before_reference);
    assert!(removed.entity_changes.is_empty());
    assert_eq!(removed.relation_changes.len(), 1);
    assert_eq!(
        removed.relation_changes[0].relation_id,
        structural_reference.relation_id
    );
    assert_eq!(
        removed.relation_changes[0].change_kind,
        WorkStateDiffChangeKind::Removed
    );
    assert_eq!(
        removed.relation_changes[0].before_relation_version_id,
        Some(structural_reference.relation_version_id)
    );
    assert_eq!(removed.relation_changes[0].after_relation_version_id, None);
}

#[test]
fn branch_head_diff_is_read_only_and_ignores_projection_and_event_noise() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let first = create_record(
        &mut engine,
        &workspace,
        workspace.genesis_commit_id,
        "first",
    );
    let second = create_record(&mut engine, &workspace, first.commit_id, "second");
    let connection = raw_connection(&path);
    let branch_id = workspace.initial_branch_id.raw_bytes();
    let first_commit_id = first.commit_id.raw_bytes();
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
             VALUES (?1, 'complete', ?2, ?3, 7)
             ON CONFLICT(branch_id) DO UPDATE SET
                projection_status = excluded.projection_status,
                projected_commit_id = excluded.projected_commit_id,
                projection_state_digest = excluded.projection_state_digest,
                updated_at_us = excluded.updated_at_us",
            params![&branch_id[..], &first_commit_id[..], &[8_u8; 32][..]],
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
             VALUES (?1, ?2, NULL, NULL, 'diff.noise', 8, '{}')",
            params![&event_id[..], &workspace.workspace_id.raw_bytes()[..]],
        )
        .expect("insert event noise");
    let before_counts = query_counts(&connection);
    let before_head = branch_head(&connection, workspace.initial_branch_id);

    let diff = engine
        .diff(WorkStateDiffOptions::new(
            WorkStateDiffTarget::commit(first.commit_id),
            WorkStateDiffTarget::branch_head(workspace.initial_branch_id),
        ))
        .expect("branch head diff");

    assert_eq!(diff.from.commit_id, first.commit_id);
    assert_eq!(diff.to.commit_id, second.commit_id);
    assert_eq!(
        diff.to.target,
        WorkStateDiffTarget::branch_head(workspace.initial_branch_id)
    );
    let changed_entities = diff
        .entity_changes
        .iter()
        .map(|change| (change.entity_id, change.change_kind))
        .collect::<BTreeMap<_, _>>();
    assert_eq!(
        changed_entities,
        BTreeMap::from([(second.entity_id, WorkStateDiffChangeKind::Added)])
    );
    assert!(diff.relation_changes.is_empty());
    assert_eq!(query_counts(&connection), before_counts);
    assert_eq!(
        branch_head(&connection, workspace.initial_branch_id),
        before_head
    );
}

#[test]
fn diff_rejects_cross_workspace_and_unknown_branch() {
    let (_tempdir, path) = store_path();
    let (mut engine, first_workspace) = create_workspace(&path);
    let second_workspace = engine
        .create_workspace(WorkspaceInitOptions::new("other-workspace").expect("workspace options"))
        .expect("create second workspace");

    let cross_workspace = engine
        .diff(WorkStateDiffOptions::new(
            WorkStateDiffTarget::commit(first_workspace.genesis_commit_id),
            WorkStateDiffTarget::commit(second_workspace.genesis_commit_id),
        ))
        .expect_err("cross workspace diff should be rejected");
    assert_eq!(cross_workspace.code(), ErrorCode::QueryInvalid);
    assert_eq!(cross_workspace.category(), ErrorCategory::Query);

    let unknown_branch = engine
        .diff(WorkStateDiffOptions::new(
            WorkStateDiffTarget::branch_head(BranchId::new_v7()),
            WorkStateDiffTarget::commit(first_workspace.genesis_commit_id),
        ))
        .expect_err("unknown branch should be rejected");
    assert_eq!(unknown_branch.code(), ErrorCode::QueryInvalid);
    assert_eq!(unknown_branch.category(), ErrorCategory::Query);
}
