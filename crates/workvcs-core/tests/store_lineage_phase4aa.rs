use tempfile::TempDir;
use workvcs_core::{
    CanonicalValue, Engine, StoreId, StoreInitOptions, StoreLineageListOptions,
    StoreLineageRecordOptions, WorkVcsError, content_object_digest,
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

fn source_root_descriptor(path: &str) -> CanonicalValue {
    CanonicalValue::object(vec![
        ("path".to_owned(), CanonicalValue::String(path.to_owned())),
        (
            "profile".to_owned(),
            CanonicalValue::String("workvcs-local-payload-directory-v1".to_owned()),
        ),
    ])
    .expect("descriptor")
}

#[test]
fn store_lineage_records_and_reads_canonical_source_context() {
    let (_source_tempdir, source_path) = store_path();
    let source = init_engine(&source_path, "phase4aa-source");
    let source_store_id = source.store_info().expect("source info").store_id;

    let (_target_tempdir, target_path) = store_path();
    let mut target = init_engine(&target_path, "phase4aa-target");
    let source_bundle_digest = content_object_digest(b"phase4aa-bundle");
    let descriptor = source_root_descriptor("/tmp/source-store");

    let recorded = target
        .record_store_lineage(
            StoreLineageRecordOptions::new(source_store_id, "explicit_fork", descriptor.clone())
                .expect("lineage options")
                .with_source_bundle_digest(source_bundle_digest),
        )
        .expect("record store lineage");
    assert_eq!(recorded.lineage.source_store_id, source_store_id);
    assert_eq!(recorded.lineage.derivation_kind, "explicit_fork");
    assert_eq!(recorded.lineage.source_root_descriptor, descriptor);
    assert_eq!(
        recorded.lineage.source_bundle_digest,
        Some(source_bundle_digest)
    );
    assert!(recorded.lineage.created_at_us > 0);
    assert!(recorded.lineage.source_root_descriptor_size_bytes > 0);

    let shown = target
        .store_lineage(recorded.lineage.lineage_id)
        .expect("show store lineage");
    assert_eq!(shown, recorded.lineage);
}

#[test]
fn store_lineage_list_filters_by_source_kind_and_bundle_digest() {
    let first_source_id = StoreId::new_v7();
    let second_source_id = StoreId::new_v7();
    let first_digest = content_object_digest(b"phase4aa-first");
    let second_digest = content_object_digest(b"phase4aa-second");

    let (_tempdir, path) = store_path();
    let mut engine = init_engine(&path, "phase4aa-filter-target");
    let first = engine
        .record_store_lineage(
            StoreLineageRecordOptions::new(
                first_source_id,
                "explicit_fork",
                source_root_descriptor("first"),
            )
            .expect("first options")
            .with_source_bundle_digest(first_digest),
        )
        .expect("record first lineage");
    let second = engine
        .record_store_lineage(
            StoreLineageRecordOptions::new(
                second_source_id,
                "bundle_import",
                source_root_descriptor("second"),
            )
            .expect("second options")
            .with_source_bundle_digest(second_digest),
        )
        .expect("record second lineage");

    let by_source = engine
        .store_lineages(StoreLineageListOptions::new().with_source_store_id(first_source_id))
        .expect("source filter");
    assert_eq!(by_source.lineages, vec![first.lineage.clone()]);

    let by_kind = engine
        .store_lineages(
            StoreLineageListOptions::new()
                .with_derivation_kind("bundle_import")
                .expect("kind filter"),
        )
        .expect("kind filter");
    assert_eq!(by_kind.lineages, vec![second.lineage.clone()]);

    let by_digest = engine
        .store_lineages(StoreLineageListOptions::new().with_source_bundle_digest(first_digest))
        .expect("digest filter");
    assert_eq!(by_digest.lineages, vec![first.lineage.clone()]);

    let mismatched = engine
        .store_lineages(
            StoreLineageListOptions::new()
                .with_source_store_id(first_source_id)
                .with_source_bundle_digest(second_digest),
        )
        .expect("mismatched filters");
    assert!(mismatched.lineages.is_empty());
}

#[test]
fn store_lineage_rejects_same_store_source_and_non_object_descriptor() {
    let (_tempdir, path) = store_path();
    let mut engine = init_engine(&path, "phase4aa-same-store-target");
    let local_store_id = engine.store_info().expect("store info").store_id;

    let same_store = engine.record_store_lineage(
        StoreLineageRecordOptions::new(
            local_store_id,
            "explicit_fork",
            source_root_descriptor("same-store"),
        )
        .expect("lineage options"),
    );
    assert!(matches!(same_store, Err(WorkVcsError::QueryInvalid(_))));

    let non_object = StoreLineageRecordOptions::new(
        StoreId::new_v7(),
        "explicit_fork",
        CanonicalValue::String("not an object".to_owned()),
    );
    assert!(matches!(non_object, Err(WorkVcsError::QueryInvalid(_))));
}

#[test]
fn store_lineage_list_rejects_zero_limit() {
    let error = StoreLineageListOptions::new()
        .with_limit(0)
        .expect_err("zero limit must fail");

    assert!(matches!(error, WorkVcsError::QueryInvalid(_)));
}
