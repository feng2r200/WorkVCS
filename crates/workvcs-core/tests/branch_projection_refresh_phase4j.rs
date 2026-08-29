use rusqlite::{Connection, params};
use tempfile::TempDir;
use workvcs_core::{
    BranchId, BranchProjectionRefreshOptions, BranchProjectionStatus, CommitId, Engine, ErrorCode,
    StoreInitOptions, TaskCreateCommit, TaskCreateOptions, TaskSchedulingRelationCreateCommit,
    TaskSchedulingRelationCreateOptions, TaskSchedulingRelationType, WorkspaceInfo,
    WorkspaceInitOptions,
};

fn store_path() -> (TempDir, std::path::PathBuf) {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let path = tempdir.path().join("workvcs.sqlite");
    (tempdir, path)
}

fn init_engine(path: &std::path::Path) -> Engine {
    Engine::init(
        path,
        StoreInitOptions::new("phase4j-projection-store").expect("store options"),
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

fn create_dependency(
    engine: &mut Engine,
    branch_id: BranchId,
    head_commit_id: CommitId,
    dependent: &TaskCreateCommit,
    prerequisite: &TaskCreateCommit,
) -> TaskSchedulingRelationCreateCommit {
    engine
        .create_task_scheduling_relation(
            TaskSchedulingRelationCreateOptions::depends_on(
                branch_id,
                head_commit_id,
                dependent.task_entity_id,
                prerequisite.task_entity_id,
            )
            .expect("dependency options"),
        )
        .expect("create dependency")
}

fn raw_connection(path: &std::path::Path) -> Connection {
    let connection = Connection::open(path).expect("raw connection");
    connection
        .pragma_update(None, "foreign_keys", "ON")
        .expect("enable raw foreign keys");
    connection
}

fn count_branch_rows(connection: &Connection, table: &str, branch_id: BranchId) -> i64 {
    connection
        .query_row(
            &format!("SELECT count(*) FROM {table} WHERE branch_id = ?1"),
            params![&branch_id.raw_bytes()[..]],
            |row| row.get(0),
        )
        .expect("count projection rows")
}

fn projection_status(connection: &Connection, branch_id: BranchId) -> String {
    connection
        .query_row(
            "SELECT projection_status
             FROM branch_projection_state
             WHERE branch_id = ?1",
            params![&branch_id.raw_bytes()[..]],
            |row| row.get(0),
        )
        .expect("projection status")
}

fn relation_current_type(connection: &Connection, branch_id: BranchId) -> String {
    connection
        .query_row(
            "SELECT relation_type
             FROM branch_relation_current
             WHERE branch_id = ?1",
            params![&branch_id.raw_bytes()[..]],
            |row| row.get(0),
        )
        .expect("relation current type")
}

#[test]
fn refresh_materializes_current_entities_and_relations() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let prerequisite = create_task(
        &mut engine,
        workspace.initial_branch_id,
        workspace.genesis_commit_id,
        "Build prerequisite",
    );
    let dependent = create_task(
        &mut engine,
        workspace.initial_branch_id,
        prerequisite.commit_id,
        "Run dependent work",
    );
    let dependency = create_dependency(
        &mut engine,
        workspace.initial_branch_id,
        dependent.commit_id,
        &dependent,
        &prerequisite,
    );
    let head = engine
        .branch_head(workspace.initial_branch_id)
        .expect("branch head");

    let before = engine
        .branch_projection(workspace.initial_branch_id)
        .expect("projection before refresh");
    assert_eq!(before.status, BranchProjectionStatus::NotMaterialized);
    assert!(!before.is_current());

    let refreshed = engine
        .refresh_branch_projection(BranchProjectionRefreshOptions::new(
            workspace.initial_branch_id,
        ))
        .expect("refresh projection");
    assert_eq!(refreshed.workspace_id, workspace.workspace_id);
    assert_eq!(refreshed.projected_commit_id, dependency.commit_id);
    assert_eq!(refreshed.projection_state_digest, head.state_digest);
    assert_eq!(refreshed.entity_count, 2);
    assert_eq!(refreshed.relation_count, 1);

    let snapshot = engine
        .branch_projection(workspace.initial_branch_id)
        .expect("projection after refresh");
    assert_eq!(snapshot.status, BranchProjectionStatus::Complete);
    assert!(snapshot.is_current());
    assert_eq!(snapshot.projected_commit_id, Some(dependency.commit_id));
    assert_eq!(snapshot.projection_state_digest, Some(head.state_digest));
    assert_eq!(snapshot.entity_count, 2);
    assert_eq!(snapshot.relation_count, 1);

    let connection = raw_connection(&path);
    assert_eq!(
        projection_status(&connection, workspace.initial_branch_id),
        "complete"
    );
    assert_eq!(
        count_branch_rows(
            &connection,
            "branch_entity_current",
            workspace.initial_branch_id
        ),
        2
    );
    assert_eq!(
        count_branch_rows(
            &connection,
            "branch_relation_current",
            workspace.initial_branch_id
        ),
        1
    );
    assert_eq!(
        relation_current_type(&connection, workspace.initial_branch_id),
        TaskSchedulingRelationType::DependsOn.as_str()
    );
}

#[test]
fn projection_reports_not_materialized_after_branch_head_moves() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let refreshed = engine
        .refresh_branch_projection(BranchProjectionRefreshOptions::new(
            workspace.initial_branch_id,
        ))
        .expect("refresh genesis projection");
    assert_eq!(refreshed.projected_commit_id, workspace.genesis_commit_id);
    assert_eq!(refreshed.entity_count, 0);
    assert_eq!(refreshed.relation_count, 0);

    let task = create_task(
        &mut engine,
        workspace.initial_branch_id,
        workspace.genesis_commit_id,
        "Move head",
    );
    let not_materialized = engine
        .branch_projection(workspace.initial_branch_id)
        .expect("not materialized projection");
    assert_eq!(
        not_materialized.status,
        BranchProjectionStatus::NotMaterialized
    );
    assert!(!not_materialized.is_current());
    assert_eq!(not_materialized.projected_commit_id, None);
    assert_eq!(not_materialized.projection_state_digest, None);
    assert_eq!(not_materialized.head_commit_id, task.commit_id);
    assert_eq!(not_materialized.entity_count, 0);
    assert_eq!(not_materialized.relation_count, 0);
}

#[test]
fn refresh_replaces_prior_rows_after_head_moves() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let first = create_task(
        &mut engine,
        workspace.initial_branch_id,
        workspace.genesis_commit_id,
        "First task",
    );
    engine
        .refresh_branch_projection(BranchProjectionRefreshOptions::new(
            workspace.initial_branch_id,
        ))
        .expect("refresh first projection");

    let second = create_task(
        &mut engine,
        workspace.initial_branch_id,
        first.commit_id,
        "Second task",
    );
    let refreshed = engine
        .refresh_branch_projection(BranchProjectionRefreshOptions::new(
            workspace.initial_branch_id,
        ))
        .expect("refresh second projection");
    assert_eq!(refreshed.projected_commit_id, second.commit_id);
    assert_eq!(refreshed.entity_count, 2);
    assert_eq!(refreshed.relation_count, 0);

    let connection = raw_connection(&path);
    assert_eq!(
        count_branch_rows(
            &connection,
            "branch_entity_current",
            workspace.initial_branch_id
        ),
        2
    );
    assert_eq!(
        count_branch_rows(
            &connection,
            "branch_relation_current",
            workspace.initial_branch_id
        ),
        0
    );
}

#[test]
fn refresh_rejects_unknown_branch() {
    let (_tempdir, path) = store_path();
    let mut engine = init_engine(&path);

    let error = engine
        .refresh_branch_projection(BranchProjectionRefreshOptions::new(BranchId::new_v7()))
        .expect_err("unknown branch");
    assert_eq!(error.code(), ErrorCode::QueryInvalid);
}
