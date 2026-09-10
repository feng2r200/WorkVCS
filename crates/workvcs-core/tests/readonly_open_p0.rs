use rusqlite::Connection;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use tempfile::TempDir;
use workvcs_core::{
    BranchProjectionRefreshOptions, BranchProjectionStatus, CanonicalValue, Engine,
    EntityTransitionCommit, EntityTransitionOptions, ErrorCode, HistoryQueryOptions,
    StoreInitOptions, WorkspaceInfo, WorkspaceInitOptions,
};

#[derive(Debug, Clone, PartialEq, Eq)]
struct FileSnapshot {
    exists: bool,
    len: Option<u64>,
    modified: Option<SystemTime>,
}

struct StoreFixture {
    workspace: WorkspaceInfo,
    transition: EntityTransitionCommit,
}

fn store_path() -> (TempDir, PathBuf) {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let path = tempdir.path().join("workvcs.sqlite");
    (tempdir, path)
}

fn sqlite_sidecar_path(path: &Path, suffix: &str) -> PathBuf {
    let mut raw = OsString::from(path.as_os_str());
    raw.push(suffix);
    PathBuf::from(raw)
}

fn sqlite_file_paths(path: &Path) -> [PathBuf; 3] {
    [
        path.to_path_buf(),
        sqlite_sidecar_path(path, "-wal"),
        sqlite_sidecar_path(path, "-shm"),
    ]
}

fn file_snapshot(path: &Path) -> FileSnapshot {
    match std::fs::metadata(path) {
        Ok(metadata) => FileSnapshot {
            exists: true,
            len: Some(metadata.len()),
            modified: Some(metadata.modified().expect("modified time")),
        },
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => FileSnapshot {
            exists: false,
            len: None,
            modified: None,
        },
        Err(error) => panic!("snapshot {path:?}: {error}"),
    }
}

fn sqlite_file_snapshots(path: &Path) -> Vec<(PathBuf, FileSnapshot)> {
    sqlite_file_paths(path)
        .into_iter()
        .map(|path| {
            let snapshot = file_snapshot(&path);
            (path, snapshot)
        })
        .collect()
}

fn assert_sqlite_files_unchanged(path: &Path, before: &[(PathBuf, FileSnapshot)]) {
    let after = sqlite_file_snapshots(path);
    assert_eq!(after, before, "SQLite main/WAL/SHM metadata changed");
}

fn init_engine(path: &Path) -> Engine {
    Engine::init(
        path,
        StoreInitOptions::new("readonly-p0-store").expect("store options"),
    )
    .expect("init engine")
}

fn workspace_options() -> WorkspaceInitOptions {
    WorkspaceInitOptions::new("readonly-p0-workspace").expect("workspace options")
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

fn create_current_store(path: &Path) -> StoreFixture {
    let mut engine = init_engine(path);
    let workspace = engine
        .create_workspace(workspace_options())
        .expect("create workspace");
    let transition = engine
        .commit_entity_transition(
            EntityTransitionOptions::create(
                workspace.initial_branch_id,
                workspace.genesis_commit_id,
                "readonly_record",
                record_state("readonly", "open"),
            )
            .expect("transition options"),
        )
        .expect("commit transition");
    let head = engine
        .branch_head(workspace.initial_branch_id)
        .expect("branch head");
    assert_eq!(head.head_commit_id, transition.commit_id);

    let refreshed = engine
        .refresh_branch_projection(BranchProjectionRefreshOptions::new(
            workspace.initial_branch_id,
        ))
        .expect("refresh projection");
    assert_eq!(refreshed.projected_commit_id, transition.commit_id);

    StoreFixture {
        workspace,
        transition,
    }
}

fn raw_connection(path: &Path) -> Connection {
    let connection = Connection::open(path).expect("raw connection");
    connection
        .pragma_update(None, "foreign_keys", "ON")
        .expect("enable raw foreign keys");
    connection
}

fn open_wal_connection_with_sidecars(path: &Path) -> Connection {
    let connection = raw_connection(path);
    let journal_mode: String = connection
        .query_row("PRAGMA journal_mode=WAL", [], |row| row.get(0))
        .expect("enable WAL journal mode");
    assert_eq!(journal_mode.to_ascii_lowercase(), "wal");
    connection
        .pragma_update(None, "wal_autocheckpoint", 0_i64)
        .expect("disable wal autocheckpoint");
    connection
        .execute(
            r#"UPDATE store SET metadata_json = '{"wal_sidecar_test":true}'"#,
            [],
        )
        .expect("write WAL frame");

    let snapshots = sqlite_file_snapshots(path);
    assert!(snapshots[1].1.exists, "expected WAL sidecar to exist");
    assert!(snapshots[2].1.exists, "expected SHM sidecar to exist");
    connection
}

fn assert_open_readonly_error_code(path: &Path, expected: ErrorCode) {
    match Engine::open_readonly(path) {
        Ok(_) => panic!("Engine::open_readonly unexpectedly succeeded"),
        Err(error) => assert_eq!(error.code(), expected, "{error}"),
    }
}

#[test]
fn readonly_open_supports_reads_rejects_writes_and_preserves_sqlite_files() {
    let (_tempdir, path) = store_path();
    let fixture = create_current_store(&path);
    let before = sqlite_file_snapshots(&path);

    let mut readonly = Engine::open_readonly(&path).expect("open readonly");
    let info = readonly.store_info().expect("store info");
    assert_eq!(info.display_name, "readonly-p0-store");

    let head = readonly
        .branch_head(fixture.workspace.initial_branch_id)
        .expect("branch head");
    assert_eq!(head.head_commit_id, fixture.transition.commit_id);

    let state = readonly
        .state_at(fixture.transition.commit_id)
        .expect("state at transition");
    assert_eq!(state.commit_id, fixture.transition.commit_id);
    assert_eq!(state.state.entities().len(), 1);

    let commit = readonly
        .commit(fixture.transition.commit_id)
        .expect("commit snapshot");
    assert_eq!(commit.commit_id, fixture.transition.commit_id);

    let history = readonly
        .history(HistoryQueryOptions::from_branch(
            fixture.workspace.initial_branch_id,
        ))
        .expect("history");
    assert_eq!(history.entries.len(), 2);

    let projection = readonly
        .branch_projection(fixture.workspace.initial_branch_id)
        .expect("branch projection");
    assert_eq!(projection.status, BranchProjectionStatus::Complete);
    assert_eq!(
        projection.projected_commit_id,
        Some(fixture.transition.commit_id)
    );

    readonly.validate_integrity().expect("integrity");

    let write_error = readonly
        .commit_entity_transition(
            EntityTransitionOptions::create(
                fixture.workspace.initial_branch_id,
                fixture.transition.commit_id,
                "readonly_record",
                record_state("readonly", "blocked"),
            )
            .expect("write transition options"),
        )
        .expect_err("readonly write must fail");
    assert_eq!(
        write_error.code(),
        ErrorCode::StorageFailure,
        "{write_error}"
    );

    let after_failed_write = readonly
        .branch_head(fixture.workspace.initial_branch_id)
        .expect("branch head after failed write");
    assert_eq!(after_failed_write, head);
    drop(readonly);

    assert_sqlite_files_unchanged(&path, &before);
}

#[test]
fn readonly_open_preserves_existing_wal_and_shm_sidecars() {
    let (_tempdir, path) = store_path();
    let fixture = create_current_store(&path);
    let wal_connection = open_wal_connection_with_sidecars(&path);
    let before = sqlite_file_snapshots(&path);

    let readonly = Engine::open_readonly(&path).expect("open readonly with wal");
    let head = readonly
        .branch_head(fixture.workspace.initial_branch_id)
        .expect("branch head");
    assert_eq!(head.head_commit_id, fixture.transition.commit_id);
    readonly.validate_integrity().expect("integrity");
    drop(readonly);

    assert_sqlite_files_unchanged(&path, &before);
    drop(wal_connection);
}

#[test]
fn readonly_open_missing_store_does_not_create_database_or_sidecars() {
    let (_tempdir, path) = store_path();
    let before = sqlite_file_snapshots(&path);

    assert_open_readonly_error_code(&path, ErrorCode::StorageFailure);

    assert_sqlite_files_unchanged(&path, &before);
}

#[test]
fn readonly_open_empty_store_rejects_without_changing_file() {
    let (_tempdir, path) = store_path();
    std::fs::File::create(&path).expect("empty sqlite file");
    let before = sqlite_file_snapshots(&path);

    assert_open_readonly_error_code(&path, ErrorCode::StoreCompatibilityUnsupported);

    assert_sqlite_files_unchanged(&path, &before);
}

#[test]
fn readonly_open_rejects_migration_required_store_without_migrating() {
    let (_tempdir, path) = store_path();
    create_current_store(&path);
    {
        let connection = raw_connection(&path);
        connection
            .execute("DROP INDEX idx_context_packet_snapshot_session_created", [])
            .expect("drop context packet session index");
        connection
            .execute("DROP INDEX idx_context_packet_snapshot_branch_head", [])
            .expect("drop context packet branch index");
        connection
            .execute("DROP TABLE context_packet_snapshot", [])
            .expect("drop context packet snapshot table");
    }
    let before = sqlite_file_snapshots(&path);

    assert_open_readonly_error_code(&path, ErrorCode::StoreBootstrapInvalid);

    assert_sqlite_files_unchanged(&path, &before);
    let connection = raw_connection(&path);
    let context_packet_object_count: i64 = connection
        .query_row(
            "SELECT count(*)
             FROM sqlite_schema
             WHERE name IN (
                 'context_packet_snapshot',
                 'idx_context_packet_snapshot_branch_head',
                 'idx_context_packet_snapshot_session_created'
             )",
            [],
            |row| row.get(0),
        )
        .expect("count context packet snapshot objects");
    assert_eq!(context_packet_object_count, 0);
}

#[test]
fn readonly_open_corrupt_store_rejects_without_changing_file() {
    let (_tempdir, path) = store_path();
    std::fs::write(&path, b"not a sqlite database").expect("write corrupt store");
    let before = sqlite_file_snapshots(&path);

    assert_open_readonly_error_code(&path, ErrorCode::StorageFailure);

    assert_sqlite_files_unchanged(&path, &before);
}
