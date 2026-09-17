use rusqlite::{Connection, params};
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;
use workvcs_core::{
    CanonicalValue, Engine, ErrorCode, EvidenceContentInput, EvidenceCreateOptions,
    LocalObjectV2MigrationMode, LocalObjectV2MigrationOptions, StoreId, StoreInitOptions,
    StoreManifest,
};

const LEGACY_BACKEND: &str = "workvcs.local-object-v1";
const CURRENT_BACKEND: &str = "workvcs.local-object-v2";

struct LegacyFixture {
    _tempdir: TempDir,
    store_path: PathBuf,
    evidence_id: workvcs_core::EvidenceId,
    old_path: PathBuf,
    new_path: PathBuf,
}

#[test]
fn preflights_and_applies_one_way_local_object_v2_cutover() {
    let fixture = legacy_store_with_content(b"migration payload");

    let open_error = match Engine::open(&fixture.store_path) {
        Ok(_) => panic!("legacy Store unexpectedly opened"),
        Err(error) => error,
    };
    assert_eq!(open_error.code(), ErrorCode::StoreCompatibilityUnsupported);

    let preflight = Engine::migrate_local_object_store_v2(
        &fixture.store_path,
        LocalObjectV2MigrationOptions::preflight().with_expected_local_objects(1),
    )
    .expect("migration preflight");
    assert_eq!(preflight.mode, LocalObjectV2MigrationMode::Preflight);
    assert_eq!(preflight.local_objects, 1);
    assert_eq!(preflight.preexisting_target_objects, 0);
    assert!(fixture.old_path.is_file());
    assert!(!fixture.new_path.exists());

    let applied = Engine::migrate_local_object_store_v2(
        &fixture.store_path,
        LocalObjectV2MigrationOptions::apply().with_expected_local_objects(1),
    )
    .expect("apply migration");
    assert_eq!(applied.mode, LocalObjectV2MigrationMode::Applied);
    assert!(applied.migration_id.is_some());
    assert_eq!(applied.store_info.manifest.object_store_format_version, 2);
    assert!(fixture.old_path.is_file(), "migration retains v1 input");
    assert!(fixture.new_path.is_file());

    let engine = Engine::open(&fixture.store_path).expect("open migrated Store");
    assert_eq!(
        engine.validate_local_content_storage().expect("validate"),
        1
    );
    assert_eq!(
        engine
            .read_evidence_content(fixture.evidence_id, 0)
            .expect("read migrated evidence")
            .raw_bytes,
        b"migration payload"
    );
    drop(engine);

    let repeated = Engine::migrate_local_object_store_v2(
        &fixture.store_path,
        LocalObjectV2MigrationOptions::apply().with_expected_local_objects(1),
    )
    .expect("idempotent current validation");
    assert_eq!(repeated.mode, LocalObjectV2MigrationMode::AlreadyCurrent);
    assert!(repeated.migration_id.is_none());

    let connection = raw_connection(&fixture.store_path);
    let backend: String = connection
        .query_row(
            "SELECT storage_backend FROM content_storage_location",
            [],
            |row| row.get(0),
        )
        .expect("storage backend");
    assert_eq!(backend, CURRENT_BACKEND);
    assert_eq!(count_rows(&connection, "store_migration_attempt"), 1);
    assert_eq!(count_rows(&connection, "store_migration_outcome"), 1);
}

#[test]
fn applies_cutover_to_store_without_local_objects() {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let store_path = tempdir.path().join("workvcs.sqlite");
    let engine = Engine::init(
        &store_path,
        StoreInitOptions::new("objectless").expect("options"),
    )
    .expect("init");
    let store_id = engine.store_info().expect("store info").store_id;
    drop(engine);
    downgrade_manifest(&store_path, store_id);

    let applied = Engine::migrate_local_object_store_v2(
        &store_path,
        LocalObjectV2MigrationOptions::apply().with_expected_local_objects(0),
    )
    .expect("apply objectless migration");
    assert_eq!(applied.mode, LocalObjectV2MigrationMode::Applied);
    assert_eq!(applied.local_objects, 0);
    assert_eq!(applied.store_info.manifest.object_store_format_version, 2);
    Engine::open(&store_path).expect("open migrated objectless Store");
}

#[test]
fn preflight_rejects_corrupt_legacy_content_without_mutating_store() {
    let fixture = legacy_store_with_content(b"valid before corruption");
    fs::write(&fixture.old_path, b"corrupt").expect("corrupt legacy object");

    let error = Engine::migrate_local_object_store_v2(
        &fixture.store_path,
        LocalObjectV2MigrationOptions::preflight().with_expected_local_objects(1),
    )
    .unwrap_err();
    assert_eq!(error.code(), ErrorCode::EvidenceInvalid);
    assert!(!fixture.new_path.exists());

    let connection = raw_connection(&fixture.store_path);
    let object_version: i64 = connection
        .query_row(
            "SELECT object_store_format_version FROM store_manifest",
            [],
            |row| row.get(0),
        )
        .expect("object version");
    assert_eq!(object_version, 1);
    assert_eq!(count_rows(&connection, "store_migration_attempt"), 0);
}

fn legacy_store_with_content(raw_bytes: &[u8]) -> LegacyFixture {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let store_path = tempdir.path().join("workvcs.sqlite");
    let mut engine = Engine::init(
        &store_path,
        StoreInitOptions::new("legacy fixture").expect("options"),
    )
    .expect("init");
    let store_id = engine.store_info().expect("store info").store_id;
    let evidence = engine
        .create_evidence(
            EvidenceCreateOptions::new(
                "command_output",
                CanonicalValue::object(Vec::new()).expect("metadata"),
            )
            .expect("evidence options")
            .with_contents(vec![
                EvidenceContentInput::from_raw_bytes("stdout", raw_bytes)
                    .expect("raw evidence content"),
            ])
            .expect("contents"),
        )
        .expect("create evidence");
    let content = &evidence.contents[0];
    let digest_text = content.content_digest.to_string();
    let new_locator = &content.storage_locations[0].locator;
    let new_path = store_path.parent().expect("store parent").join(new_locator);
    let old_relative = PathBuf::from(".workvcs-objects")
        .join(store_id.to_string())
        .join("sha256")
        .join(&digest_text[..2])
        .join(&digest_text);
    let old_path = store_path
        .parent()
        .expect("store parent")
        .join(&old_relative);
    fs::create_dir_all(old_path.parent().expect("old object parent"))
        .expect("create legacy object directory");
    fs::rename(&new_path, &old_path).expect("move object to legacy locator");
    drop(engine);

    let connection = raw_connection(&store_path);
    let updated = connection
        .execute(
            "UPDATE content_storage_location
             SET storage_backend = ?2, locator = ?3
             WHERE content_digest = ?1 AND storage_backend = ?4",
            params![
                &content.content_digest.as_bytes()[..],
                LEGACY_BACKEND,
                old_relative.to_str().expect("legacy locator"),
                CURRENT_BACKEND,
            ],
        )
        .expect("downgrade storage location");
    assert_eq!(updated, 1);
    drop(connection);
    downgrade_manifest(&store_path, store_id);

    LegacyFixture {
        _tempdir: tempdir,
        store_path,
        evidence_id: evidence.evidence_id,
        old_path,
        new_path,
    }
}

fn downgrade_manifest(store_path: &Path, store_id: StoreId) {
    let mut legacy_manifest = StoreManifest::current();
    legacy_manifest.object_store_format_version = 1;
    let manifest_json = legacy_manifest
        .canonical_manifest_json()
        .expect("legacy manifest json");
    let connection = raw_connection(store_path);
    let store_id_bytes = store_id.raw_bytes();
    let updated = connection
        .execute(
            "UPDATE store_manifest
             SET object_store_format_version = 1, manifest_json = ?2
             WHERE store_id = ?1",
            params![&store_id_bytes[..], manifest_json],
        )
        .expect("downgrade manifest");
    assert_eq!(updated, 1);
}

fn raw_connection(path: &Path) -> Connection {
    let connection = Connection::open(path).expect("raw connection");
    connection
        .pragma_update(None, "foreign_keys", "ON")
        .expect("enable foreign keys");
    connection
}

fn count_rows(connection: &Connection, table: &str) -> i64 {
    connection
        .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
            row.get(0)
        })
        .expect("count rows")
}
