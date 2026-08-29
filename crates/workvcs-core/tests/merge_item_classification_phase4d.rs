use rusqlite::{Connection, params};
use tempfile::TempDir;
use workvcs_core::{
    BranchForkOptions, BranchId, CommitId, Engine, EntityId, MergeItemClassification,
    MergeItemSubject, MergeStartOptions, RelationId, StoreInitOptions, TaskCreateCommit,
    TaskCreateOptions, TaskSchedulingRelationCreateCommit, TaskSchedulingRelationCreateOptions,
    TaskStatus, TaskTransitionCommit, TaskTransitionOptions, WorkspaceInfo, WorkspaceInitOptions,
};

fn store_path() -> (TempDir, std::path::PathBuf) {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let path = tempdir.path().join("workvcs.sqlite");
    (tempdir, path)
}

fn init_engine(path: &std::path::Path) -> Engine {
    Engine::init(
        path,
        StoreInitOptions::new("phase4d-merge-store").expect("store options"),
    )
    .expect("init engine")
}

fn create_workspace(path: &std::path::Path) -> (Engine, WorkspaceInfo) {
    let mut engine = init_engine(path);
    let workspace = engine
        .create_workspace(WorkspaceInitOptions::new("workspace").expect("workspace options"))
        .expect("create workspace");
    (engine, workspace)
}

fn create_task(
    engine: &mut Engine,
    branch_id: BranchId,
    head_commit_id: CommitId,
    description: &str,
) -> TaskCreateCommit {
    engine
        .create_task(
            TaskCreateOptions::new(branch_id, head_commit_id, description).expect("task options"),
        )
        .expect("create task")
}

fn transition_task(
    engine: &mut Engine,
    branch_id: BranchId,
    head_commit_id: CommitId,
    task: &TaskCreateCommit,
    next_status: TaskStatus,
) -> TaskTransitionCommit {
    engine
        .transition_task(
            TaskTransitionOptions::new(
                branch_id,
                head_commit_id,
                task.task_entity_id,
                task.task_entity_version_id,
                next_status,
            )
            .expect("transition options"),
        )
        .expect("transition task")
}

fn create_depends_on_relation(
    engine: &mut Engine,
    branch_id: BranchId,
    head_commit_id: CommitId,
    dependent: EntityId,
    prerequisite: EntityId,
) -> TaskSchedulingRelationCreateCommit {
    engine
        .create_task_scheduling_relation(
            TaskSchedulingRelationCreateOptions::depends_on(
                branch_id,
                head_commit_id,
                dependent,
                prerequisite,
            )
            .expect("relation options"),
        )
        .expect("create task scheduling relation")
}

fn raw_connection(path: &std::path::Path) -> Connection {
    let connection = Connection::open(path).expect("raw connection");
    connection
        .pragma_update(None, "foreign_keys", "ON")
        .expect("enable raw foreign keys");
    connection
}

fn count_merge_items(connection: &Connection, merge_id: workvcs_core::MergeId) -> i64 {
    let merge_id = merge_id.raw_bytes();
    connection
        .query_row(
            "SELECT count(*)
             FROM merge_item
             WHERE merge_id = ?1",
            params![&merge_id[..]],
            |row| row.get(0),
        )
        .expect("count merge items")
}

fn item_for_entity(
    items: &[workvcs_core::MergeItemSnapshot],
    entity_id: EntityId,
) -> &workvcs_core::MergeItemSnapshot {
    items
        .iter()
        .find(|item| item.subject == Some(MergeItemSubject::Entity(entity_id)))
        .unwrap_or_else(|| panic!("missing merge item for entity {entity_id}"))
}

fn item_for_relation(
    items: &[workvcs_core::MergeItemSnapshot],
    relation_id: RelationId,
) -> &workvcs_core::MergeItemSnapshot {
    items
        .iter()
        .find(|item| item.subject == Some(MergeItemSubject::Relation(relation_id)))
        .unwrap_or_else(|| panic!("missing merge item for relation {relation_id}"))
}

#[test]
fn start_merge_classifies_source_items_and_entity_conflicts() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);

    let shared = create_task(
        &mut engine,
        workspace.initial_branch_id,
        workspace.genesis_commit_id,
        "Shared task",
    );
    let dependent = create_task(
        &mut engine,
        workspace.initial_branch_id,
        shared.commit_id,
        "Dependent task",
    );
    let prerequisite = create_task(
        &mut engine,
        workspace.initial_branch_id,
        dependent.commit_id,
        "Prerequisite task",
    );
    let base_commit = prerequisite.commit_id;

    let source_branch = engine
        .fork_branch(
            BranchForkOptions::from_branch(workspace.initial_branch_id, "source")
                .expect("fork source options"),
        )
        .expect("fork source branch");

    let target_transition = transition_task(
        &mut engine,
        workspace.initial_branch_id,
        base_commit,
        &shared,
        TaskStatus::InProgress,
    );
    let target_only = create_task(
        &mut engine,
        workspace.initial_branch_id,
        target_transition.commit_id,
        "Target-only task",
    );

    let source_only = create_task(
        &mut engine,
        source_branch.branch_id,
        base_commit,
        "Source-only task",
    );
    let source_relation = create_depends_on_relation(
        &mut engine,
        source_branch.branch_id,
        source_only.commit_id,
        dependent.task_entity_id,
        prerequisite.task_entity_id,
    );
    let source_transition = transition_task(
        &mut engine,
        source_branch.branch_id,
        source_relation.commit_id,
        &shared,
        TaskStatus::Blocked,
    );

    let merge = engine
        .start_merge(MergeStartOptions::new(
            workspace.initial_branch_id,
            source_branch.branch_id,
        ))
        .expect("start merge");
    assert_eq!(merge.merge_base_commit_id, base_commit);
    assert_eq!(merge.target_head_commit_id, target_only.commit_id);
    assert_eq!(merge.source_head_commit_id, source_transition.commit_id);

    let snapshot = engine.merge_attempt(merge.merge_id).expect("show merge");
    assert_eq!(snapshot.items.len(), 3);
    for (expected_ordinal, item) in snapshot.items.iter().enumerate() {
        assert_eq!(item.ordinal, expected_ordinal as i64);
    }

    let conflict = item_for_entity(&snapshot.items, shared.task_entity_id);
    assert_eq!(conflict.classification, MergeItemClassification::Conflict);
    let auto_entity = item_for_entity(&snapshot.items, source_only.task_entity_id);
    assert_eq!(auto_entity.classification, MergeItemClassification::Auto);
    let auto_relation = item_for_relation(&snapshot.items, source_relation.relation_id);
    assert_eq!(auto_relation.classification, MergeItemClassification::Auto);
    assert!(
        snapshot
            .items
            .iter()
            .all(|item| item.subject != Some(MergeItemSubject::Entity(target_only.task_entity_id)))
    );

    let connection = raw_connection(&path);
    assert_eq!(count_merge_items(&connection, merge.merge_id), 3);
}
