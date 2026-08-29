use rusqlite::{Connection, OptionalExtension, params};
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;
use workvcs_core::{
    BundleImportApplyOptions, BundleImportPreflightOptions, BundlePayloadExport,
    BundlePayloadExportOptions, BundlePayloadInput, CanonicalValue, ChangeSetId, Engine,
    SessionEndOptions, SessionId, SessionLifecycleState, SessionStartOptions, StoreInitOptions,
    TaskCreateOptions, WorkspaceInfo, WorkspaceInitOptions,
};

fn store_paths() -> (TempDir, PathBuf, PathBuf) {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let source_path = tempdir.path().join("source.sqlite");
    let old_path = tempdir.path().join("old.sqlite");
    (tempdir, source_path, old_path)
}

fn object(entries: Vec<(&str, CanonicalValue)>) -> CanonicalValue {
    CanonicalValue::object(
        entries
            .into_iter()
            .map(|(key, value)| (key.to_owned(), value))
            .collect(),
    )
    .expect("canonical object")
}

fn string(value: &str) -> CanonicalValue {
    CanonicalValue::String(value.to_owned())
}

fn create_workspace(path: &Path) -> (Engine, WorkspaceInfo) {
    let mut engine = Engine::init(
        path,
        StoreInitOptions::new("phase4bb-bundle-changeset-session-provenance-store")
            .expect("store options"),
    )
    .expect("init engine");
    let workspace = engine
        .create_workspace(WorkspaceInitOptions::new("workspace").expect("workspace options"))
        .expect("create workspace");
    (engine, workspace)
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

fn preflight_options(export: &BundlePayloadExport) -> BundleImportPreflightOptions {
    BundleImportPreflightOptions::from_parts(
        export.manifest_bytes.clone(),
        export.payload_index_bytes.clone(),
        payload_inputs(export),
    )
    .expect("preflight options")
}

fn apply_options(export: &BundlePayloadExport) -> BundleImportApplyOptions {
    BundleImportApplyOptions::from_parts(
        export.manifest_bytes.clone(),
        export.payload_index_bytes.clone(),
        payload_inputs(export),
    )
    .expect("apply options")
}

fn set_changeset_origin_session(path: &Path, changeset_id: ChangeSetId, session_id: SessionId) {
    let connection = Connection::open(path).expect("open sqlite");
    let updated = connection
        .execute(
            "UPDATE changeset
             SET origin_session_id = ?1
             WHERE changeset_id = ?2",
            params![&session_id.raw_bytes()[..], &changeset_id.raw_bytes()[..],],
        )
        .expect("set changeset origin session");
    assert_eq!(updated, 1);
}

fn changeset_origin_session(path: &Path, changeset_id: ChangeSetId) -> Option<SessionId> {
    let connection = Connection::open(path).expect("open sqlite");
    let bytes = connection
        .query_row(
            "SELECT origin_session_id
             FROM changeset
             WHERE changeset_id = ?1",
            params![&changeset_id.raw_bytes()[..]],
            |row| row.get::<_, Option<Vec<u8>>>(0),
        )
        .optional()
        .expect("read changeset origin session")
        .flatten()?;
    let bytes = <[u8; 16]>::try_from(bytes).expect("16-byte session id");
    Some(SessionId::from_bytes(bytes).expect("session id"))
}

#[test]
fn bundle_apply_preserves_changeset_origin_session_id() {
    let (_tempdir, source_path, old_path) = store_paths();
    let (engine, workspace) = create_workspace(&source_path);
    drop(engine);
    fs::copy(&source_path, &old_path).expect("copy old store");

    let mut source_engine = Engine::open(&source_path).expect("open source store");
    let session = source_engine
        .start_session(
            SessionStartOptions::new(workspace.workspace_id, workspace.initial_branch_id)
                .expect("session options")
                .with_metadata(object(vec![("agent", string("phase-4bb"))]))
                .expect("session metadata"),
        )
        .expect("start session");
    let task = source_engine
        .create_task(
            TaskCreateOptions::new(
                workspace.initial_branch_id,
                workspace.genesis_commit_id,
                "task with origin session provenance",
            )
            .expect("task options"),
        )
        .expect("create task");
    set_changeset_origin_session(&source_path, task.changeset_id, session.session_id);
    let ended = source_engine
        .end_session(
            SessionEndOptions::new(session.session_id)
                .expect("session end options")
                .with_summary(object(vec![("result", string("done"))]))
                .expect("session summary"),
        )
        .expect("end session");

    let export = source_engine
        .export_bundle_payloads(BundlePayloadExportOptions::for_commit(task.commit_id))
        .expect("export payloads");
    assert_eq!(export.manifest.sessions.len(), 1);
    assert_eq!(export.manifest.session_diffs.len(), 1);
    assert_eq!(export.manifest.sessions[0].session_id, session.session_id);
    assert_eq!(
        export.manifest.session_diffs[0].session_diff_id,
        ended.session_diff_id
    );

    let mut old_engine = Engine::open(&old_path).expect("open old store");
    let before = old_engine
        .preflight_bundle_import(preflight_options(&export))
        .expect("preflight before apply");
    assert_eq!(before.action, "same_store_fast_forward_ready");
    assert!(before.can_apply);

    let applied = old_engine
        .apply_bundle_import(apply_options(&export))
        .expect("apply bundle import");

    assert!(applied.applied);
    assert_eq!(applied.imported_sessions, 1);
    assert_eq!(applied.imported_session_diffs, 1);
    assert_eq!(
        changeset_origin_session(&old_path, task.changeset_id),
        Some(session.session_id)
    );
    let imported_session = old_engine
        .session_snapshot(session.session_id)
        .expect("imported session snapshot");
    assert_eq!(
        imported_session.lifecycle_state,
        SessionLifecycleState::Ended
    );
    assert_eq!(
        imported_session.session_diff_id,
        Some(ended.session_diff_id)
    );
}
