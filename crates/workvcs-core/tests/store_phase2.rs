use rusqlite::{Connection, params};
use std::path::{Path, PathBuf};
use tempfile::TempDir;
use workvcs_core::store::{
    CANONICAL_JSON_PROFILE, DIGEST_ALGORITHM, ID_SCHEME, OBJECT_STORE_FORMAT_VERSION,
    SCHEMA_VERSION, STORE_FORMAT_VERSION,
};
use workvcs_core::{Engine, ErrorCode, StoreId, StoreInitOptions, StoreManifest, WorkVcsError};

fn store_path() -> (TempDir, PathBuf) {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let path = tempdir.path().join("workvcs.sqlite");
    (tempdir, path)
}

fn init_store(path: &Path) -> workvcs_core::Result<workvcs_core::StoreInfo> {
    let engine = Engine::init(path, StoreInitOptions::new("phase2-store")?)?;
    engine.store_info()
}

fn raw_connection(path: &Path) -> Connection {
    let connection = Connection::open(path).expect("raw connection");
    connection
        .pragma_update(None, "foreign_keys", "ON")
        .expect("enable raw foreign keys");
    connection
}

fn raw_connection_ignoring_checks(path: &Path) -> Connection {
    let connection = raw_connection(path);
    connection
        .pragma_update(None, "ignore_check_constraints", "ON")
        .expect("ignore check constraints");
    connection
}

fn assert_open_error_code(path: &Path, expected: ErrorCode) {
    match Engine::open(path) {
        Ok(_) => panic!("Engine::open unexpectedly succeeded"),
        Err(error) => assert_eq!(error.code(), expected, "{error}"),
    }
}

#[test]
fn initializes_and_reopens_store_manifest() {
    let (_tempdir, path) = store_path();
    let info = init_store(&path).expect("init store");

    assert_eq!(info.display_name, "phase2-store");
    assert_eq!(info.manifest.store_format_version, STORE_FORMAT_VERSION);
    assert_eq!(info.manifest.schema_version, SCHEMA_VERSION);
    assert_eq!(
        info.manifest.object_store_format_version,
        OBJECT_STORE_FORMAT_VERSION
    );
    assert_eq!(info.manifest.id_scheme, ID_SCHEME);
    assert_eq!(info.manifest.digest_algorithm, DIGEST_ALGORITHM);
    assert_eq!(info.manifest.canonical_json_profile, CANONICAL_JSON_PROFILE);

    let reopened = Engine::open(&path).expect("reopen store");
    assert_eq!(reopened.store_info().expect("store info"), info);
}

#[test]
fn init_rejects_existing_schema_objects() {
    let (_tempdir, path) = store_path();
    {
        let connection = Connection::open(&path).expect("raw connection");
        connection
            .execute("CREATE TABLE unrelated(id INTEGER PRIMARY KEY)", [])
            .expect("create unrelated table");
    }

    match Engine::init(&path, StoreInitOptions::new("existing").expect("options")) {
        Ok(_) => panic!("Engine::init unexpectedly succeeded"),
        Err(error) => assert_eq!(error.code(), ErrorCode::StoreAlreadyInitialized),
    }
}

#[test]
fn open_rejects_application_id_mismatch() {
    let (_tempdir, path) = store_path();
    init_store(&path).expect("init store");

    let connection = raw_connection(&path);
    connection
        .pragma_update(None, "application_id", 123_i64)
        .expect("change application id");
    drop(connection);

    assert_open_error_code(&path, ErrorCode::StoreCompatibilityUnsupported);
}

#[test]
fn open_rejects_missing_schema_structure_even_with_application_id() {
    let (_tempdir, path) = store_path();
    let connection = Connection::open(&path).expect("raw connection");
    connection
        .pragma_update(None, "application_id", 1_465_271_123_i64)
        .expect("set application id");
    drop(connection);

    assert_open_error_code(&path, ErrorCode::StoreBootstrapInvalid);
}

#[test]
fn open_rejects_schema_object_drift_with_same_table_count() {
    let (_tempdir, path) = store_path();
    init_store(&path).expect("init store");

    let connection = raw_connection(&path);
    connection
        .execute("DROP INDEX uq_external_object_ref_object", [])
        .expect("drop frozen index");
    connection
        .execute(
            "CREATE INDEX schema_drift_padding ON store(created_at_us)",
            [],
        )
        .expect("create padding index");
    drop(connection);

    assert_open_error_code(&path, ErrorCode::StoreBootstrapInvalid);
}

#[test]
fn explicit_context_packet_snapshot_schema_migration_recovers_pre_4ls_store() {
    let (_tempdir, path) = store_path();
    init_store(&path).expect("init store");

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

    assert_open_error_code(&path, ErrorCode::StoreBootstrapInvalid);

    let migrated = Engine::migrate_context_packet_snapshot_schema(&path)
        .expect("migrate context packet snapshot schema");
    assert!(migrated.migrated);
    assert_eq!(migrated.added_schema_objects.len(), 3);
    assert!(
        migrated
            .added_schema_objects
            .contains(&"table:context_packet_snapshot".to_owned())
    );
    assert!(
        migrated
            .added_schema_objects
            .contains(&"index:idx_context_packet_snapshot_branch_head".to_owned())
    );
    assert!(
        migrated
            .added_schema_objects
            .contains(&"index:idx_context_packet_snapshot_session_created".to_owned())
    );
    let migration = migrated
        .migration
        .as_ref()
        .expect("migration record exists");
    assert_eq!(migration.from_schema_version, SCHEMA_VERSION);
    assert_eq!(migration.to_schema_version, SCHEMA_VERSION);
    assert_eq!(migration.tool_version, "workvcs-context-packet-snapshot-v1");
    assert_eq!(
        migration
            .outcome
            .as_ref()
            .expect("migration outcome")
            .outcome,
        "completed"
    );

    Engine::open(&path).expect("reopen migrated store");
    let no_op = Engine::migrate_context_packet_snapshot_schema(&path)
        .expect("idempotent context packet snapshot schema migration");
    assert!(!no_op.migrated);
    assert!(no_op.added_schema_objects.is_empty());
    assert!(no_op.migration.is_none());
}

#[test]
fn open_rejects_missing_manifest() {
    let (_tempdir, path) = store_path();
    init_store(&path).expect("init store");

    let connection = raw_connection(&path);
    connection
        .execute("DELETE FROM store_manifest", [])
        .expect("delete manifest");
    drop(connection);

    assert_open_error_code(&path, ErrorCode::StoreBootstrapInvalid);
}

#[test]
fn open_rejects_duplicate_store_manifest_pair() {
    let (_tempdir, path) = store_path();
    init_store(&path).expect("init store");

    let second_store_id = StoreId::new_v7();
    let second_store_id_bytes = second_store_id.raw_bytes();
    let manifest = StoreManifest::current();
    let manifest_json = manifest
        .canonical_manifest_json()
        .expect("canonical manifest json");
    let connection = raw_connection(&path);
    connection
        .execute(
            "INSERT INTO store(store_id, created_at_us, display_name, metadata_json)
             VALUES (?1, ?2, ?3, ?4)",
            params![&second_store_id_bytes[..], 2000_i64, "duplicate", "{}"],
        )
        .expect("insert second store");
    connection
        .execute(
            "INSERT INTO store_manifest(
                store_id,
                store_format_version,
                schema_version,
                object_store_format_version,
                id_scheme,
                digest_algorithm,
                canonical_json_profile,
                created_at_us,
                manifest_json
             )
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                &second_store_id_bytes[..],
                manifest.store_format_version,
                manifest.schema_version,
                manifest.object_store_format_version,
                manifest.id_scheme,
                manifest.digest_algorithm,
                manifest.canonical_json_profile,
                2001_i64,
                manifest_json
            ],
        )
        .expect("insert second manifest");
    drop(connection);

    assert_open_error_code(&path, ErrorCode::StoreBootstrapInvalid);
}

#[test]
fn open_rejects_newer_store_format() {
    let (_tempdir, path) = store_path();
    init_store(&path).expect("init store");

    let connection = raw_connection(&path);
    connection
        .execute(
            "UPDATE store_manifest SET store_format_version = ?1",
            [STORE_FORMAT_VERSION + 1],
        )
        .expect("update store format");
    drop(connection);

    assert_open_error_code(&path, ErrorCode::StoreCompatibilityUnsupported);
}

#[test]
fn open_rejects_older_store_format() {
    let (_tempdir, path) = store_path();
    init_store(&path).expect("init store");

    let connection = raw_connection_ignoring_checks(&path);
    connection
        .execute(
            "UPDATE store_manifest SET store_format_version = ?1",
            [STORE_FORMAT_VERSION - 1],
        )
        .expect("update store format");
    drop(connection);

    assert_open_error_code(&path, ErrorCode::StoreCompatibilityUnsupported);
}

#[test]
fn open_rejects_schema_version_mismatch() {
    let (_tempdir, path) = store_path();
    init_store(&path).expect("init store");

    let connection = raw_connection_ignoring_checks(&path);
    connection
        .execute(
            "UPDATE store_manifest SET schema_version = ?1",
            [SCHEMA_VERSION + 1],
        )
        .expect("update schema version");
    drop(connection);

    assert_open_error_code(&path, ErrorCode::StoreCompatibilityUnsupported);
}

#[test]
fn open_rejects_newer_object_store_format() {
    let (_tempdir, path) = store_path();
    init_store(&path).expect("init store");

    let connection = raw_connection(&path);
    connection
        .execute(
            "UPDATE store_manifest SET object_store_format_version = ?1",
            [OBJECT_STORE_FORMAT_VERSION + 1],
        )
        .expect("update object store format");
    drop(connection);

    assert_open_error_code(&path, ErrorCode::StoreCompatibilityUnsupported);
}

#[test]
fn open_rejects_legacy_object_store_format_without_runtime_fallback() {
    let (_tempdir, path) = store_path();
    init_store(&path).expect("init store");

    let mut legacy_manifest = StoreManifest::current();
    legacy_manifest.object_store_format_version = OBJECT_STORE_FORMAT_VERSION - 1;
    let legacy_manifest_json = legacy_manifest
        .canonical_manifest_json()
        .expect("legacy manifest json");
    let connection = raw_connection(&path);
    connection
        .execute(
            "UPDATE store_manifest
             SET object_store_format_version = ?1, manifest_json = ?2",
            params![
                legacy_manifest.object_store_format_version,
                legacy_manifest_json,
            ],
        )
        .expect("update object store format");
    drop(connection);

    assert_open_error_code(&path, ErrorCode::StoreCompatibilityUnsupported);
}

#[test]
fn open_rejects_id_scheme_mismatch() {
    let (_tempdir, path) = store_path();
    init_store(&path).expect("init store");

    let connection = raw_connection_ignoring_checks(&path);
    connection
        .execute("UPDATE store_manifest SET id_scheme = ?1", ["uuidv4-text"])
        .expect("update id scheme");
    drop(connection);

    assert_open_error_code(&path, ErrorCode::StoreCompatibilityUnsupported);
}

#[test]
fn open_rejects_digest_algorithm_mismatch() {
    let (_tempdir, path) = store_path();
    init_store(&path).expect("init store");

    let connection = raw_connection_ignoring_checks(&path);
    connection
        .execute(
            "UPDATE store_manifest SET digest_algorithm = ?1",
            ["sha2-256"],
        )
        .expect("update digest algorithm");
    drop(connection);

    assert_open_error_code(&path, ErrorCode::StoreCompatibilityUnsupported);
}

#[test]
fn open_rejects_canonical_profile_mismatch() {
    let (_tempdir, path) = store_path();
    init_store(&path).expect("init store");

    let connection = raw_connection(&path);
    connection
        .execute(
            "UPDATE store_manifest SET canonical_json_profile = ?1",
            ["workvcs-canonical-json-v0.1"],
        )
        .expect("update canonical profile");
    drop(connection);

    assert_open_error_code(&path, ErrorCode::StoreCompatibilityUnsupported);
}

#[test]
fn open_rejects_noncanonical_manifest_json() {
    let (_tempdir, path) = store_path();
    init_store(&path).expect("init store");

    let connection = raw_connection(&path);
    connection
        .execute(
            "UPDATE store_manifest SET manifest_json = ?1",
            [r#"{"store_format_version":1}"#],
        )
        .expect("update manifest json");
    drop(connection);

    assert_open_error_code(&path, ErrorCode::StoreBootstrapInvalid);
}

#[test]
fn manifest_validator_rejects_unsupported_values_not_representable_by_schema_checks() {
    let mut manifest = StoreManifest::current();
    manifest.schema_version = SCHEMA_VERSION + 1;
    assert_eq!(
        manifest.validate_supported().unwrap_err().code(),
        ErrorCode::StoreCompatibilityUnsupported
    );

    let mut manifest = StoreManifest::current();
    manifest.store_format_version = STORE_FORMAT_VERSION - 1;
    assert_eq!(
        manifest.validate_supported().unwrap_err().code(),
        ErrorCode::StoreCompatibilityUnsupported
    );

    let mut manifest = StoreManifest::current();
    manifest.object_store_format_version = OBJECT_STORE_FORMAT_VERSION + 1;
    assert_eq!(
        manifest.validate_supported().unwrap_err().code(),
        ErrorCode::StoreCompatibilityUnsupported
    );

    let mut manifest = StoreManifest::current();
    manifest.id_scheme = "uuidv4-text".to_owned();
    assert_eq!(
        manifest.validate_supported().unwrap_err().code(),
        ErrorCode::StoreCompatibilityUnsupported
    );

    let mut manifest = StoreManifest::current();
    manifest.digest_algorithm = "sha2-256".to_owned();
    assert_eq!(
        manifest.validate_supported().unwrap_err().code(),
        ErrorCode::StoreCompatibilityUnsupported
    );
}

#[test]
fn public_errors_remain_structured() {
    let error = WorkVcsError::StoreBootstrapInvalid("example".to_owned());

    assert_eq!(error.code(), ErrorCode::StoreBootstrapInvalid);
    assert!(!error.retryable());
}
