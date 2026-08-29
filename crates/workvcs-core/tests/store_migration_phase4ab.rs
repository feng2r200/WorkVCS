use tempfile::TempDir;
use workvcs_core::{
    CanonicalValue, Engine, StoreInitOptions, StoreMigrationListOptions,
    StoreMigrationRecordOptions, WorkVcsError,
};

fn store_path() -> (TempDir, std::path::PathBuf) {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let path = tempdir.path().join("workvcs.sqlite");
    (tempdir, path)
}

fn init_engine(path: &std::path::Path, display_name: &str) -> Engine {
    Engine::init(
        path,
        StoreInitOptions::new(display_name).expect("store options"),
    )
    .expect("init engine")
}

fn detail_json(label: &str) -> CanonicalValue {
    CanonicalValue::object(vec![
        ("label".to_owned(), CanonicalValue::String(label.to_owned())),
        (
            "schema_delta".to_owned(),
            CanonicalValue::object(vec![]).expect("delta"),
        ),
    ])
    .expect("detail")
}

#[test]
fn store_migration_records_and_reads_outcome_detail() {
    let (_tempdir, path) = store_path();
    let mut engine = init_engine(&path, "phase4ab-migration-store");

    let recorded = engine
        .record_store_migration(
            StoreMigrationRecordOptions::new(
                1,
                2,
                1,
                2,
                "workvcs-test-migrator/0.1",
                "completed",
                detail_json("first migration"),
            )
            .expect("migration options"),
        )
        .expect("record migration");

    assert_eq!(recorded.migration.from_store_format_version, 1);
    assert_eq!(recorded.migration.to_store_format_version, 2);
    assert_eq!(recorded.migration.from_schema_version, 1);
    assert_eq!(recorded.migration.to_schema_version, 2);
    assert_eq!(recorded.migration.tool_version, "workvcs-test-migrator/0.1");
    assert!(recorded.migration.started_at_us > 0);
    let outcome = recorded.migration.outcome.as_ref().expect("outcome");
    assert_eq!(outcome.outcome, "completed");
    assert!(outcome.completed_at_us > 0);
    assert!(outcome.detail_size_bytes > 0);

    let shown = engine
        .store_migration(recorded.migration.migration_id)
        .expect("show migration");
    assert_eq!(shown, recorded.migration);
}

#[test]
fn store_migration_list_is_newest_first_and_limitable() {
    let (_tempdir, path) = store_path();
    let mut engine = init_engine(&path, "phase4ab-migration-list");

    let first = engine
        .record_store_migration(
            StoreMigrationRecordOptions::new(
                1,
                2,
                1,
                2,
                "workvcs-test-migrator/0.1",
                "completed",
                detail_json("first"),
            )
            .expect("first options"),
        )
        .expect("first migration");
    let second = engine
        .record_store_migration(
            StoreMigrationRecordOptions::new(
                2,
                3,
                2,
                3,
                "workvcs-test-migrator/0.2",
                "failed",
                detail_json("second"),
            )
            .expect("second options"),
        )
        .expect("second migration");

    let listed = engine
        .store_migrations(StoreMigrationListOptions::new())
        .expect("list migrations");
    assert_eq!(listed.migrations.len(), 2);
    assert_eq!(listed.migrations[0], second.migration);
    assert_eq!(listed.migrations[1], first.migration);

    let limited = engine
        .store_migrations(
            StoreMigrationListOptions::new()
                .with_limit(1)
                .expect("limit"),
        )
        .expect("limited migrations");
    assert_eq!(limited.migrations, vec![second.migration]);
}

#[test]
fn store_migration_rejects_invalid_versions_text_and_detail() {
    let bad_version =
        StoreMigrationRecordOptions::new(0, 1, 1, 1, "tool", "completed", detail_json("bad"));
    assert!(matches!(bad_version, Err(WorkVcsError::QueryInvalid(_))));

    let bad_tool =
        StoreMigrationRecordOptions::new(1, 1, 1, 1, "", "completed", detail_json("bad"));
    assert!(matches!(bad_tool, Err(WorkVcsError::QueryInvalid(_))));

    let bad_detail = StoreMigrationRecordOptions::new(
        1,
        1,
        1,
        1,
        "tool",
        "completed",
        CanonicalValue::String("not an object".to_owned()),
    );
    assert!(matches!(bad_detail, Err(WorkVcsError::QueryInvalid(_))));
}

#[test]
fn store_migration_list_rejects_zero_limit() {
    let error = StoreMigrationListOptions::new()
        .with_limit(0)
        .expect_err("zero limit must fail");

    assert!(matches!(error, WorkVcsError::QueryInvalid(_)));
}
