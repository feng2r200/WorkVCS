use tempfile::TempDir;
use workvcs_core::{
    CanonicalValue, Engine, ExternalObjectId, ExternalObjectRefListOptions,
    ExternalObjectRefRecordOptions, ExternalVersionId, StoreId, StoreInitOptions, WorkVcsError,
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

fn descriptor(label: &str) -> CanonicalValue {
    CanonicalValue::object(vec![
        ("label".to_owned(), CanonicalValue::String(label.to_owned())),
        (
            "source".to_owned(),
            CanonicalValue::String("external-bundle".to_owned()),
        ),
    ])
    .expect("descriptor")
}

#[test]
fn external_object_ref_records_object_and_version_scopes() {
    let (_tempdir, path) = store_path();
    let mut engine = init_engine(&path, "phase4ac-external-ref-store");
    let external_store_id = StoreId::new_v7();
    let external_object_id = ExternalObjectId::new_v7();
    let external_version_ref = ExternalVersionId::new_v7();

    let object_ref = engine
        .record_external_object_ref(
            ExternalObjectRefRecordOptions::for_object(
                external_store_id,
                external_object_id,
                "knowledge",
                descriptor("object"),
            )
            .expect("object ref options"),
        )
        .expect("record object ref");
    assert!(object_ref.created);
    assert_eq!(object_ref.external_ref.external_store_id, external_store_id);
    assert_eq!(
        object_ref.external_ref.external_object_id,
        external_object_id
    );
    assert_eq!(object_ref.external_ref.reference_scope.as_str(), "object");
    assert_eq!(object_ref.external_ref.external_version_ref, None);

    let version_ref = engine
        .record_external_object_ref(
            ExternalObjectRefRecordOptions::for_version(
                external_store_id,
                external_object_id,
                "knowledge",
                external_version_ref,
                descriptor("version"),
            )
            .expect("version ref options"),
        )
        .expect("record version ref");
    assert!(version_ref.created);
    assert_eq!(version_ref.external_ref.reference_scope.as_str(), "version");
    assert_eq!(
        version_ref.external_ref.external_version_ref,
        Some(external_version_ref)
    );

    let shown = engine
        .external_object_ref(version_ref.external_ref.external_ref_id)
        .expect("show external ref");
    assert_eq!(shown, version_ref.external_ref);
}

#[test]
fn external_object_ref_record_is_idempotent_but_rejects_conflicting_descriptor() {
    let (_tempdir, path) = store_path();
    let mut engine = init_engine(&path, "phase4ac-idempotent");
    let external_store_id = StoreId::new_v7();
    let external_object_id = ExternalObjectId::new_v7();

    let options = ExternalObjectRefRecordOptions::for_object(
        external_store_id,
        external_object_id,
        "record",
        descriptor("same"),
    )
    .expect("external ref options");
    let first = engine
        .record_external_object_ref(options.clone())
        .expect("record first");
    let second = engine
        .record_external_object_ref(options)
        .expect("record idempotent");
    assert!(first.created);
    assert!(!second.created);
    assert_eq!(second.external_ref, first.external_ref);

    let conflict = engine.record_external_object_ref(
        ExternalObjectRefRecordOptions::for_object(
            external_store_id,
            external_object_id,
            "record",
            descriptor("different"),
        )
        .expect("conflicting options"),
    );
    assert!(matches!(conflict, Err(WorkVcsError::QueryInvalid(_))));
}

#[test]
fn external_object_ref_list_filters_by_store_kind_and_scope() {
    let (_tempdir, path) = store_path();
    let mut engine = init_engine(&path, "phase4ac-list");
    let first_store_id = StoreId::new_v7();
    let second_store_id = StoreId::new_v7();

    let first = engine
        .record_external_object_ref(
            ExternalObjectRefRecordOptions::for_object(
                first_store_id,
                ExternalObjectId::new_v7(),
                "knowledge",
                descriptor("first"),
            )
            .expect("first options"),
        )
        .expect("record first");
    let second = engine
        .record_external_object_ref(
            ExternalObjectRefRecordOptions::for_version(
                second_store_id,
                ExternalObjectId::new_v7(),
                "evidence",
                ExternalVersionId::new_v7(),
                descriptor("second"),
            )
            .expect("second options"),
        )
        .expect("record second");

    let by_store = engine
        .external_object_refs(
            ExternalObjectRefListOptions::new().with_external_store_id(first_store_id),
        )
        .expect("store filter");
    assert_eq!(by_store.external_refs, vec![first.external_ref.clone()]);

    let by_kind = engine
        .external_object_refs(
            ExternalObjectRefListOptions::new()
                .with_object_kind("evidence")
                .expect("kind filter"),
        )
        .expect("kind filter");
    assert_eq!(by_kind.external_refs, vec![second.external_ref.clone()]);

    let by_scope = engine
        .external_object_refs(
            ExternalObjectRefListOptions::new()
                .with_reference_scope(workvcs_core::ExternalObjectReferenceScope::Object),
        )
        .expect("scope filter");
    assert_eq!(by_scope.external_refs, vec![first.external_ref]);
}

#[test]
fn external_object_ref_rejects_local_store_non_object_descriptor_and_zero_limit() {
    let (_tempdir, path) = store_path();
    let mut engine = init_engine(&path, "phase4ac-invalid");
    let local_store_id = engine.store_info().expect("store info").store_id;

    let same_store = engine.record_external_object_ref(
        ExternalObjectRefRecordOptions::for_object(
            local_store_id,
            ExternalObjectId::new_v7(),
            "knowledge",
            descriptor("same-store"),
        )
        .expect("same-store options"),
    );
    assert!(matches!(same_store, Err(WorkVcsError::QueryInvalid(_))));

    let non_object = ExternalObjectRefRecordOptions::for_object(
        StoreId::new_v7(),
        ExternalObjectId::new_v7(),
        "knowledge",
        CanonicalValue::String("not an object".to_owned()),
    );
    assert!(matches!(non_object, Err(WorkVcsError::QueryInvalid(_))));

    let zero_limit = ExternalObjectRefListOptions::new()
        .with_limit(0)
        .expect_err("zero limit must fail");
    assert!(matches!(zero_limit, WorkVcsError::QueryInvalid(_)));
}
