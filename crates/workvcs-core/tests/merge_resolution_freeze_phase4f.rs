use rusqlite::Connection;
use tempfile::TempDir;
use workvcs_core::{
    BranchForkOptions, BranchId, CanonicalValue, CommitId, Engine, ErrorCode,
    MergeFreezeResolutionsOptions, MergeResolutionKind, MergeResolveOptions, MergeStartOptions,
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
        StoreInitOptions::new("phase4f-merge-store").expect("store options"),
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

fn rationale(reason: &str) -> CanonicalValue {
    CanonicalValue::object(vec![(
        "reason".to_owned(),
        CanonicalValue::String(reason.to_owned()),
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

#[test]
fn freeze_merge_resolutions_requires_runtime_resolution_and_freezes_once() {
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

    let premature = engine
        .freeze_merge_resolutions(MergeFreezeResolutionsOptions::new(merge.merge_id))
        .expect_err("unresolved item should block freeze");
    assert_eq!(premature.code(), ErrorCode::WorkspaceInvalid);

    engine
        .resolve_merge_item(
            MergeResolveOptions::theirs(item_id)
                .expect("theirs options")
                .with_rationale(rationale("take source"))
                .expect("rationale"),
        )
        .expect("resolve item");

    let frozen = engine
        .freeze_merge_resolutions(MergeFreezeResolutionsOptions::new(merge.merge_id))
        .expect("freeze resolutions");
    assert_eq!(frozen.merge_id, merge.merge_id);
    assert_eq!(frozen.workspace_id, workspace.workspace_id);
    assert_eq!(frozen.frozen_items, 1);

    let shown = engine.merge_attempt(merge.merge_id).expect("show frozen");
    assert_eq!(
        shown.items[0]
            .resolution
            .as_ref()
            .expect("runtime resolution")
            .resolution_kind,
        MergeResolutionKind::Theirs
    );

    let duplicate = engine
        .freeze_merge_resolutions(MergeFreezeResolutionsOptions::new(merge.merge_id))
        .expect_err("duplicate freeze should fail");
    assert_eq!(duplicate.code(), ErrorCode::WorkspaceInvalid);

    let update = engine
        .resolve_merge_item(MergeResolveOptions::ours(item_id).expect("ours options"))
        .expect_err("frozen resolution should reject runtime updates");
    assert_eq!(update.code(), ErrorCode::WorkspaceInvalid);

    let connection = raw_connection(&path);
    assert_eq!(count_rows(&connection, "merge_resolution_runtime"), 1);
    assert_eq!(count_rows(&connection, "merge_resolution"), 1);
}
