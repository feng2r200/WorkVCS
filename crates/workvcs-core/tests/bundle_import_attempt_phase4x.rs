use rusqlite::{Connection, params};
use tempfile::TempDir;
use workvcs_core::{
    BranchId, BundleImportAttemptOptions, BundlePayloadExport, BundlePayloadExportOptions,
    BundlePayloadInput, CanonicalValue, CommitId, Digest, Engine, StoreInitOptions,
    TaskCreateCommit, TaskCreateOptions, WorkspaceInfo, WorkspaceInitOptions,
    content_object_digest, parse_canonical_json,
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

fn create_workspace(path: &std::path::Path, display_name: &str) -> (Engine, WorkspaceInfo) {
    let mut engine = init_engine(path, display_name);
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

fn payload_inputs(export: &BundlePayloadExport) -> Vec<BundlePayloadInput> {
    export
        .payload_files
        .iter()
        .map(|payload| {
            BundlePayloadInput::new(payload.relative_path.clone(), payload.bytes.clone())
                .expect("payload input")
        })
        .collect()
}

fn import_options(export: &BundlePayloadExport) -> BundleImportAttemptOptions {
    BundleImportAttemptOptions::from_parts(
        export.manifest_bytes.clone(),
        export.payload_index_bytes.clone(),
        payload_inputs(export),
    )
    .expect("import attempt options")
}

fn raw_connection(path: &std::path::Path) -> Connection {
    let connection = Connection::open(path).expect("raw connection");
    connection
        .pragma_update(None, "foreign_keys", "ON")
        .expect("enable raw foreign keys");
    connection
}

fn import_attempt_count(connection: &Connection) -> i64 {
    connection
        .query_row("SELECT count(*) FROM import_attempt", [], |row| row.get(0))
        .expect("import_attempt count")
}

fn import_outcome_count(connection: &Connection) -> i64 {
    connection
        .query_row("SELECT count(*) FROM import_attempt_outcome", [], |row| {
            row.get(0)
        })
        .expect("import_attempt_outcome count")
}

fn digest_from_blob(bytes: Vec<u8>) -> Digest {
    Digest::from_bytes(bytes.try_into().expect("digest bytes"))
}

#[test]
fn bundle_import_attempt_records_valid_same_store_preflight_outcome() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path, "phase4x-source-store");
    let task = create_task(
        &mut engine,
        workspace.initial_branch_id,
        workspace.genesis_commit_id,
        "Record import attempt",
    );
    let export = engine
        .export_bundle_payloads(BundlePayloadExportOptions::for_commit(task.commit_id))
        .expect("export bundle payloads");

    let result = engine
        .record_bundle_import_attempt(import_options(&export))
        .expect("record bundle import attempt");

    assert!(result.recorded);
    assert_eq!(result.import_profile, "workvcs-local-payload-directory-v2");
    assert_eq!(result.outcome, "already_present");
    assert_eq!(result.bundle_digest, result.preflight.payload_index_digest);
    assert_eq!(
        result.bundle_digest,
        content_object_digest(&export.payload_index_bytes)
    );
    assert!(result.started_at_us.is_some());
    assert_eq!(result.completed_at_us, result.started_at_us);
    assert_eq!(
        result.preflight.source_store_id,
        Some(export.manifest.store_id)
    );
    assert!(result.preflight.incoming_commit_present);
    assert!(!result.preflight.import_required);

    let import_id = result.import_id.expect("import id");
    let connection = raw_connection(&path);
    assert_eq!(import_attempt_count(&connection), 1);
    assert_eq!(import_outcome_count(&connection), 1);

    let import_id_bytes = import_id.raw_bytes();
    let row = connection
        .query_row(
            "SELECT source_store_id, bundle_digest, import_profile, started_at_us
             FROM import_attempt
             WHERE import_id = ?1",
            params![&import_id_bytes[..]],
            |row| {
                Ok((
                    row.get::<_, Vec<u8>>(0)?,
                    row.get::<_, Vec<u8>>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, i64>(3)?,
                ))
            },
        )
        .expect("import attempt row");
    assert_eq!(row.0, export.manifest.store_id.raw_bytes());
    assert_eq!(digest_from_blob(row.1), result.bundle_digest);
    assert_eq!(row.2, "workvcs-local-payload-directory-v2");
    assert_eq!(Some(row.3), result.started_at_us);

    let outcome = connection
        .query_row(
            "SELECT outcome, completed_at_us, detail_json
             FROM import_attempt_outcome
             WHERE import_id = ?1",
            params![&import_id_bytes[..]],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, String>(2)?,
                ))
            },
        )
        .expect("import attempt outcome row");
    assert_eq!(outcome.0, "already_present");
    assert_eq!(Some(outcome.1), result.completed_at_us);
    let detail = parse_canonical_json(outcome.2.as_bytes()).expect("detail JSON");
    let CanonicalValue::Object(fields) = detail else {
        panic!("detail JSON must be an object");
    };
    assert!(fields.contains(&(
        "action".to_owned(),
        CanonicalValue::String("already_present".to_owned())
    )));
    assert!(fields.contains(&("valid".to_owned(), CanonicalValue::Bool(true))));
}

#[test]
fn bundle_import_attempt_does_not_record_invalid_directory() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path, "phase4x-invalid-store");
    let task = create_task(
        &mut engine,
        workspace.initial_branch_id,
        workspace.genesis_commit_id,
        "Invalid import attempt",
    );
    let export = engine
        .export_bundle_payloads(BundlePayloadExportOptions::for_commit(task.commit_id))
        .expect("export bundle payloads");
    let mut payloads = payload_inputs(&export);
    payloads[0].bytes = br#"{"drifted":true}"#.to_vec();
    let options = BundleImportAttemptOptions::from_parts(
        export.manifest_bytes.clone(),
        export.payload_index_bytes.clone(),
        payloads,
    )
    .expect("import attempt options");

    let result = engine
        .record_bundle_import_attempt(options)
        .expect("record bundle import attempt");

    assert!(!result.recorded);
    assert_eq!(result.import_id, None);
    assert_eq!(result.started_at_us, None);
    assert_eq!(result.completed_at_us, None);
    assert_eq!(result.outcome, "invalid_bundle_directory");
    assert!(!result.preflight.valid);

    let connection = raw_connection(&path);
    assert_eq!(import_attempt_count(&connection), 0);
    assert_eq!(import_outcome_count(&connection), 0);
}

#[test]
fn bundle_import_attempt_records_external_store_preflight_outcome() {
    let (_source_tempdir, source_path) = store_path();
    let (mut source, workspace) = create_workspace(&source_path, "phase4x-source-store");
    let task = create_task(
        &mut source,
        workspace.initial_branch_id,
        workspace.genesis_commit_id,
        "External import attempt",
    );
    let export = source
        .export_bundle_payloads(BundlePayloadExportOptions::for_commit(task.commit_id))
        .expect("export bundle payloads");

    let (_target_tempdir, target_path) = store_path();
    let mut target = init_engine(&target_path, "phase4x-target-store");
    let result = target
        .record_bundle_import_attempt(import_options(&export))
        .expect("record external import attempt");

    assert!(result.recorded);
    assert_eq!(result.outcome, "external_store_import_not_implemented");
    assert_eq!(result.preflight.source_store_relation, "external_store");
    assert!(result.preflight.import_required);
    assert!(!result.preflight.can_apply);

    let connection = raw_connection(&target_path);
    assert_eq!(import_attempt_count(&connection), 1);
    let import_id = result.import_id.expect("import id");
    let import_id_bytes = import_id.raw_bytes();
    let stored_source_store_id = connection
        .query_row(
            "SELECT source_store_id FROM import_attempt WHERE import_id = ?1",
            params![&import_id_bytes[..]],
            |row| row.get::<_, Vec<u8>>(0),
        )
        .expect("stored source store id");
    assert_eq!(stored_source_store_id, export.manifest.store_id.raw_bytes());
}
