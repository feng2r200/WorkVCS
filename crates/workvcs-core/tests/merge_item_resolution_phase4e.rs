use rusqlite::{Connection, params};
use tempfile::TempDir;
use workvcs_core::{
    BranchForkOptions, BranchId, CanonicalValue, CommitId, Engine, MergeItemResolutionSnapshot,
    MergeResolutionKind, MergeResolveOptions, MergeStartOptions, SessionStartOptions,
    StoreInitOptions, TaskCreateOptions, WorkspaceInfo, WorkspaceInitOptions,
};

fn store_path() -> (TempDir, std::path::PathBuf) {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let path = tempdir.path().join("workvcs.sqlite");
    (tempdir, path)
}

fn init_engine(path: &std::path::Path) -> Engine {
    Engine::init(
        path,
        StoreInitOptions::new("phase4e-merge-store").expect("store options"),
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
) -> CommitId {
    engine
        .create_task(
            TaskCreateOptions::new(branch_id, head_commit_id, description).expect("task options"),
        )
        .expect("create task")
        .commit_id
}

fn detail(label: &str) -> CanonicalValue {
    CanonicalValue::object(vec![(
        "reason".to_owned(),
        CanonicalValue::String(label.to_owned()),
    )])
    .expect("canonical object")
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

fn stored_resolution_kind(
    connection: &Connection,
    merge_item_id: workvcs_core::MergeItemId,
) -> String {
    let merge_item_id = merge_item_id.raw_bytes();
    connection
        .query_row(
            "SELECT resolution_kind
             FROM merge_resolution_runtime
             WHERE merge_item_id = ?1",
            params![&merge_item_id[..]],
            |row| row.get(0),
        )
        .expect("stored resolution kind")
}

fn assert_resolution(
    snapshot: &MergeItemResolutionSnapshot,
    kind: MergeResolutionKind,
    custom_payload: Option<CanonicalValue>,
) {
    assert_eq!(snapshot.resolution_kind, kind);
    assert_eq!(snapshot.custom_payload, custom_payload);
}

#[test]
fn resolve_merge_item_records_and_updates_runtime_resolution_only() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);

    let base_commit = create_task(
        &mut engine,
        workspace.initial_branch_id,
        workspace.genesis_commit_id,
        "Base task",
    );
    let source_branch = engine
        .fork_branch(
            BranchForkOptions::from_branch(workspace.initial_branch_id, "source")
                .expect("fork options"),
        )
        .expect("fork source branch");
    create_task(
        &mut engine,
        source_branch.branch_id,
        base_commit,
        "Source task",
    );
    let merge = engine
        .start_merge(MergeStartOptions::new(
            workspace.initial_branch_id,
            source_branch.branch_id,
        ))
        .expect("start merge");
    let item_id = engine
        .merge_attempt(merge.merge_id)
        .expect("show merge")
        .items
        .first()
        .expect("merge item")
        .merge_item_id;
    let session = engine
        .start_session(
            SessionStartOptions::new(workspace.workspace_id, workspace.initial_branch_id)
                .expect("session options"),
        )
        .expect("start session");

    let resolved = engine
        .resolve_merge_item(
            MergeResolveOptions::theirs(item_id)
                .expect("theirs options")
                .with_resolved_by_session_id(session.session_id)
                .with_rationale(detail("take source"))
                .expect("rationale"),
        )
        .expect("resolve item");
    assert_eq!(resolved.merge_id, merge.merge_id);
    assert_eq!(resolved.merge_item_id, item_id);
    assert_eq!(resolved.workspace_id, workspace.workspace_id);
    assert_resolution(&resolved.resolution, MergeResolutionKind::Theirs, None);
    assert_eq!(
        resolved.resolution.resolved_by_session_id,
        Some(session.session_id)
    );

    let shown = engine.merge_attempt(merge.merge_id).expect("show resolved");
    assert_resolution(
        shown.items[0].resolution.as_ref().expect("item resolution"),
        MergeResolutionKind::Theirs,
        None,
    );

    let custom_payload = detail("custom value");
    let updated = engine
        .resolve_merge_item(
            MergeResolveOptions::custom(item_id, custom_payload.clone())
                .expect("custom options")
                .with_rationale(detail("customize"))
                .expect("rationale"),
        )
        .expect("update item resolution");
    assert_resolution(
        &updated.resolution,
        MergeResolutionKind::Custom,
        Some(custom_payload.clone()),
    );

    let shown = engine.merge_attempt(merge.merge_id).expect("show updated");
    assert_resolution(
        shown.items[0]
            .resolution
            .as_ref()
            .expect("updated resolution"),
        MergeResolutionKind::Custom,
        Some(custom_payload),
    );

    let connection = raw_connection(&path);
    assert_eq!(count_rows(&connection, "merge_resolution_runtime"), 1);
    assert_eq!(count_rows(&connection, "merge_resolution"), 0);
    assert_eq!(stored_resolution_kind(&connection, item_id), "custom");
}
