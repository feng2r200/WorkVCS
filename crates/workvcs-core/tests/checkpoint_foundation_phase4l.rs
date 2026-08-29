use rusqlite::{Connection, params};
use tempfile::TempDir;
use workvcs_core::{
    BranchId, CanonicalValue, CheckpointCreateOptions, CheckpointListOptions, CommitId, Digest,
    Engine, StoreInitOptions, TaskCreateCommit, TaskCreateOptions, WorkspaceInfo,
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
        StoreInitOptions::new("phase4l-checkpoint-store").expect("store options"),
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

fn raw_connection(path: &std::path::Path) -> Connection {
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

fn content_object_count(connection: &Connection, digest: Digest) -> i64 {
    connection
        .query_row(
            "SELECT count(*)
             FROM content_object
             WHERE content_digest = ?1",
            params![&digest.as_bytes()[..]],
            |row| row.get(0),
        )
        .expect("content object count")
}

fn overwrite_checkpoint_state_digest(
    connection: &Connection,
    checkpoint_id: workvcs_core::CheckpointId,
    state_digest: Digest,
) {
    connection
        .execute(
            "UPDATE checkpoint
             SET state_digest = ?2
             WHERE checkpoint_id = ?1",
            params![&checkpoint_id.raw_bytes()[..], &state_digest.as_bytes()[..]],
        )
        .expect("overwrite checkpoint state digest");
}

#[test]
fn checkpoint_records_replay_verified_work_state_digest() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let task = create_task(
        &mut engine,
        workspace.initial_branch_id,
        workspace.genesis_commit_id,
        "Checkpoint target",
    );

    let created = engine
        .create_checkpoint(CheckpointCreateOptions::new(task.commit_id))
        .expect("create checkpoint");

    assert_eq!(created.checkpoint.workspace_id, workspace.workspace_id);
    assert_eq!(created.checkpoint.commit_id, task.commit_id);
    assert_eq!(created.checkpoint.state_digest, task.work_state_digest);
    assert_eq!(created.checkpoint.checkpoint_format_version, 1);
    assert_eq!(
        created.checkpoint.media_type.as_deref(),
        Some("application/vnd.workvcs.workstate-checkpoint+json")
    );
    assert_eq!(created.checkpoint.usability_state, "usable");
    assert_eq!(created.entity_count, 1);
    assert_eq!(created.relation_count, 0);
    assert_ne!(
        created.checkpoint.content_digest,
        created.checkpoint.state_digest
    );

    let CanonicalValue::Object(status_detail) = &created.checkpoint.status_detail else {
        panic!("status detail is an object");
    };
    assert!(status_detail.iter().any(|(key, value)| key == "validation"
        && value == &CanonicalValue::String("replayed_work_state_digest_match".to_owned())));
}

#[test]
fn same_commit_reuses_canonical_checkpoint_content_digest() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);

    let first = engine
        .create_checkpoint(CheckpointCreateOptions::new(workspace.genesis_commit_id))
        .expect("first checkpoint");
    let second = engine
        .create_checkpoint(CheckpointCreateOptions::new(workspace.genesis_commit_id))
        .expect("second checkpoint");

    assert_ne!(
        first.checkpoint.checkpoint_id,
        second.checkpoint.checkpoint_id
    );
    assert_eq!(
        first.checkpoint.content_digest,
        second.checkpoint.content_digest
    );
    assert_eq!(
        first.checkpoint.state_digest,
        second.checkpoint.state_digest
    );
    assert_eq!(first.entity_count, 0);
    assert_eq!(second.entity_count, 0);

    let connection = raw_connection(&path);
    assert_eq!(count_rows(&connection, "checkpoint"), 2);
    assert_eq!(count_rows(&connection, "content_object"), 1);
    assert_eq!(
        content_object_count(&connection, first.checkpoint.content_digest),
        1
    );
}

#[test]
fn checkpoint_content_changes_when_work_state_changes() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);

    let genesis_checkpoint = engine
        .create_checkpoint(CheckpointCreateOptions::new(workspace.genesis_commit_id))
        .expect("genesis checkpoint");
    let task = create_task(
        &mut engine,
        workspace.initial_branch_id,
        workspace.genesis_commit_id,
        "Changed WorkState",
    );
    let task_checkpoint = engine
        .create_checkpoint(CheckpointCreateOptions::new(task.commit_id))
        .expect("task checkpoint");

    assert_ne!(
        genesis_checkpoint.checkpoint.state_digest,
        task_checkpoint.checkpoint.state_digest
    );
    assert_ne!(
        genesis_checkpoint.checkpoint.content_digest,
        task_checkpoint.checkpoint.content_digest
    );

    let connection = raw_connection(&path);
    assert_eq!(count_rows(&connection, "checkpoint"), 2);
    assert_eq!(count_rows(&connection, "content_object"), 2);
}

#[test]
fn checkpoint_show_loads_saved_metadata() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let created = engine
        .create_checkpoint(CheckpointCreateOptions::new(workspace.genesis_commit_id))
        .expect("create checkpoint");
    drop(engine);

    let reopened = Engine::open(&path).expect("reopen engine");
    let loaded = reopened
        .checkpoint(created.checkpoint.checkpoint_id)
        .expect("load checkpoint");

    assert_eq!(loaded, created.checkpoint);
}

#[test]
fn checkpoint_validation_confirms_rebuildable_digest() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let task = create_task(
        &mut engine,
        workspace.initial_branch_id,
        workspace.genesis_commit_id,
        "Validate checkpoint",
    );
    let created = engine
        .create_checkpoint(CheckpointCreateOptions::new(task.commit_id))
        .expect("create checkpoint");

    let validation = engine
        .validate_checkpoint(created.checkpoint.checkpoint_id)
        .expect("validate checkpoint");

    assert!(validation.valid);
    assert_eq!(validation.problem, None);
    assert_eq!(validation.checkpoint.usability_state, "usable");
    assert_eq!(validation.expected_state_digest, task.work_state_digest);
    assert_eq!(
        validation.expected_content_digest,
        created.checkpoint.content_digest
    );
    assert_eq!(
        validation.expected_content_size_bytes,
        created.checkpoint.content_size_bytes
    );
    assert!(validation.checkpoint.last_validated_at_us >= created.checkpoint.last_validated_at_us);
    let CanonicalValue::Object(status_detail) = &validation.checkpoint.status_detail else {
        panic!("status detail is an object");
    };
    assert!(
        status_detail
            .iter()
            .any(|(key, value)| key == "valid" && value == &CanonicalValue::Bool(true))
    );
}

#[test]
fn checkpoint_validation_marks_state_digest_drift_invalid() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let task = create_task(
        &mut engine,
        workspace.initial_branch_id,
        workspace.genesis_commit_id,
        "Drift checkpoint",
    );
    let created = engine
        .create_checkpoint(CheckpointCreateOptions::new(task.commit_id))
        .expect("create checkpoint");
    let wrong_state_digest = Digest::raw(b"wrong checkpoint state digest");
    {
        let connection = raw_connection(&path);
        overwrite_checkpoint_state_digest(
            &connection,
            created.checkpoint.checkpoint_id,
            wrong_state_digest,
        );
    }

    let validation = engine
        .validate_checkpoint(created.checkpoint.checkpoint_id)
        .expect("validate checkpoint");

    assert!(!validation.valid);
    assert_eq!(validation.checkpoint.usability_state, "invalid");
    assert_eq!(validation.expected_state_digest, task.work_state_digest);
    assert_eq!(
        validation.expected_content_digest,
        created.checkpoint.content_digest
    );
    assert!(
        validation
            .problem
            .as_deref()
            .expect("problem")
            .contains("state digest")
    );

    let loaded = engine
        .checkpoint(created.checkpoint.checkpoint_id)
        .expect("load checkpoint");
    assert_eq!(loaded.usability_state, "invalid");
}

#[test]
fn checkpoints_list_returns_only_requested_commit_checkpoints() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let task = create_task(
        &mut engine,
        workspace.initial_branch_id,
        workspace.genesis_commit_id,
        "List checkpoint target",
    );
    let first = engine
        .create_checkpoint(CheckpointCreateOptions::new(task.commit_id))
        .expect("first task checkpoint");
    let second = engine
        .create_checkpoint(CheckpointCreateOptions::new(task.commit_id))
        .expect("second task checkpoint");
    engine
        .create_checkpoint(CheckpointCreateOptions::new(workspace.genesis_commit_id))
        .expect("genesis checkpoint");

    let listed = engine
        .checkpoints(CheckpointListOptions::for_commit(task.commit_id))
        .expect("list checkpoints");

    assert_eq!(listed.commit_id, task.commit_id);
    assert_eq!(listed.checkpoints.len(), 2);
    assert!(
        listed
            .checkpoints
            .iter()
            .any(|checkpoint| checkpoint.checkpoint_id == first.checkpoint.checkpoint_id)
    );
    assert!(
        listed
            .checkpoints
            .iter()
            .any(|checkpoint| checkpoint.checkpoint_id == second.checkpoint.checkpoint_id)
    );
    assert!(
        listed
            .checkpoints
            .iter()
            .all(|checkpoint| checkpoint.commit_id == task.commit_id)
    );
}
