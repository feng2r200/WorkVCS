use tempfile::TempDir;
use workvcs_core::{
    BranchId, BundleImportAttemptListOptions, BundleImportAttemptOptions, BundlePayloadExport,
    BundlePayloadExportOptions, BundlePayloadInput, CanonicalValue, CommitId, Engine,
    StoreInitOptions, TaskCreateCommit, TaskCreateOptions, WorkspaceInfo, WorkspaceInitOptions,
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

fn field<'a>(value: &'a CanonicalValue, name: &str) -> &'a CanonicalValue {
    let CanonicalValue::Object(fields) = value else {
        panic!("detail must be an object");
    };
    fields
        .iter()
        .find_map(|(candidate, value)| (candidate == name).then_some(value))
        .unwrap_or_else(|| panic!("missing detail field {name}"))
}

#[test]
fn bundle_import_attempt_show_reads_recorded_outcome_detail() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path, "phase4y-query-store");
    let task = create_task(
        &mut engine,
        workspace.initial_branch_id,
        workspace.genesis_commit_id,
        "Show recorded import attempt",
    );
    let export = engine
        .export_bundle_payloads(BundlePayloadExportOptions::for_commit(task.commit_id))
        .expect("export bundle payloads");
    let recorded = engine
        .record_bundle_import_attempt(import_options(&export))
        .expect("record import attempt");
    let import_id = recorded.import_id.expect("import id");

    let snapshot = engine
        .bundle_import_attempt(import_id)
        .expect("show import attempt");

    assert_eq!(snapshot.import_id, import_id);
    assert_eq!(snapshot.source_store_id, export.manifest.store_id);
    assert_eq!(snapshot.bundle_digest, recorded.bundle_digest);
    assert_eq!(
        snapshot.import_profile,
        "workvcs-local-payload-directory-v2"
    );
    assert_eq!(
        snapshot.started_at_us,
        recorded.started_at_us.expect("started")
    );
    let outcome = snapshot.outcome.expect("outcome");
    assert_eq!(outcome.outcome, "already_present");
    assert_eq!(
        outcome.completed_at_us,
        recorded.completed_at_us.expect("completed")
    );
    assert!(outcome.detail_size_bytes > 0);
    assert_eq!(
        field(&outcome.detail, "action"),
        &CanonicalValue::String("already_present".to_owned())
    );
    assert_eq!(field(&outcome.detail, "valid"), &CanonicalValue::Bool(true));
}

#[test]
fn bundle_import_attempt_list_is_newest_first_and_limitable() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path, "phase4y-list-store");
    let first = create_task(
        &mut engine,
        workspace.initial_branch_id,
        workspace.genesis_commit_id,
        "First import attempt",
    );
    let first_export = engine
        .export_bundle_payloads(BundlePayloadExportOptions::for_commit(first.commit_id))
        .expect("first export");
    let first_recorded = engine
        .record_bundle_import_attempt(import_options(&first_export))
        .expect("first record");
    let second = create_task(
        &mut engine,
        workspace.initial_branch_id,
        first.commit_id,
        "Second import attempt",
    );
    let second_export = engine
        .export_bundle_payloads(BundlePayloadExportOptions::for_commit(second.commit_id))
        .expect("second export");
    let second_recorded = engine
        .record_bundle_import_attempt(import_options(&second_export))
        .expect("second record");

    let listed = engine
        .bundle_import_attempts(BundleImportAttemptListOptions::new())
        .expect("list import attempts");
    assert_eq!(listed.attempts.len(), 2);
    assert_eq!(
        listed.attempts[0].import_id,
        second_recorded.import_id.unwrap()
    );
    assert_eq!(
        listed.attempts[1].import_id,
        first_recorded.import_id.unwrap()
    );

    let limited = engine
        .bundle_import_attempts(
            BundleImportAttemptListOptions::new()
                .with_limit(1)
                .expect("limit"),
        )
        .expect("limited import attempts");
    assert_eq!(limited.attempts.len(), 1);
    assert_eq!(
        limited.attempts[0].import_id,
        second_recorded.import_id.unwrap()
    );
}

#[test]
fn bundle_import_attempt_list_filters_by_source_store_and_bundle_digest() {
    let (_first_tempdir, first_path) = store_path();
    let (mut first_source, first_workspace) = create_workspace(&first_path, "phase4z-first-source");
    let first_task = create_task(
        &mut first_source,
        first_workspace.initial_branch_id,
        first_workspace.genesis_commit_id,
        "First external import",
    );
    let first_export = first_source
        .export_bundle_payloads(BundlePayloadExportOptions::for_commit(first_task.commit_id))
        .expect("first export");

    let (_second_tempdir, second_path) = store_path();
    let (mut second_source, second_workspace) =
        create_workspace(&second_path, "phase4z-second-source");
    let second_task = create_task(
        &mut second_source,
        second_workspace.initial_branch_id,
        second_workspace.genesis_commit_id,
        "Second external import",
    );
    let second_export = second_source
        .export_bundle_payloads(BundlePayloadExportOptions::for_commit(
            second_task.commit_id,
        ))
        .expect("second export");

    let (_target_tempdir, target_path) = store_path();
    let mut target = init_engine(&target_path, "phase4z-target-store");
    let first_recorded = target
        .record_bundle_import_attempt(import_options(&first_export))
        .expect("first import attempt");
    let second_recorded = target
        .record_bundle_import_attempt(import_options(&second_export))
        .expect("second import attempt");

    let first_source_only = target
        .bundle_import_attempts(
            BundleImportAttemptListOptions::new()
                .with_source_store_id(first_export.manifest.store_id),
        )
        .expect("source filtered imports");
    assert_eq!(first_source_only.attempts.len(), 1);
    assert_eq!(
        first_source_only.attempts[0].import_id,
        first_recorded.import_id.unwrap()
    );

    let second_bundle_only = target
        .bundle_import_attempts(
            BundleImportAttemptListOptions::new().with_bundle_digest(second_recorded.bundle_digest),
        )
        .expect("bundle digest filtered imports");
    assert_eq!(second_bundle_only.attempts.len(), 1);
    assert_eq!(
        second_bundle_only.attempts[0].import_id,
        second_recorded.import_id.unwrap()
    );

    let mismatched = target
        .bundle_import_attempts(
            BundleImportAttemptListOptions::new()
                .with_source_store_id(first_export.manifest.store_id)
                .with_bundle_digest(second_recorded.bundle_digest),
        )
        .expect("mismatched filters");
    assert_eq!(mismatched.attempts.len(), 0);
}

#[test]
fn bundle_import_attempt_list_rejects_zero_limit() {
    let error = BundleImportAttemptListOptions::new()
        .with_limit(0)
        .expect_err("zero limit must fail");
    assert!(error.to_string().contains("limit must be positive"));
}
