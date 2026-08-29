use rusqlite::{Connection, params};
use tempfile::TempDir;
use workvcs_core::{
    BranchForkOptions, BranchId, CanonicalValue, CommitId, Engine, ErrorCode, MergeContinueOptions,
    MergeFreezeResolutionsOptions, MergeListOptions, MergeOutcome, MergeResolutionKind,
    MergeResolveOptions, MergeRuntimeState, MergeStartOptions, StoreInitOptions, TaskCreateCommit,
    TaskCreateOptions, WorkState, WorkspaceInfo, WorkspaceInitOptions, work_state_mapping_digest,
};

fn store_path() -> (TempDir, std::path::PathBuf) {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let path = tempdir.path().join("workvcs.sqlite");
    (tempdir, path)
}

fn init_engine(path: &std::path::Path) -> Engine {
    Engine::init(
        path,
        StoreInitOptions::new("phase4h-merge-store").expect("store options"),
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
) -> TaskCreateCommit {
    engine
        .create_task(
            TaskCreateOptions::new(branch_id, head_commit_id, description).expect("task options"),
        )
        .expect("create task")
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

fn decode_commit_id(bytes: Vec<u8>) -> CommitId {
    CommitId::from_bytes(bytes.try_into().expect("commit id length")).expect("commit id")
}

fn commit_parents(connection: &Connection, commit_id: CommitId) -> Vec<(i64, String, CommitId)> {
    let mut statement = connection
        .prepare(
            "SELECT parent_ordinal, parent_role, parent_commit_id
             FROM commit_parent
             WHERE commit_id = ?1
             ORDER BY parent_ordinal",
        )
        .expect("prepare parents");
    statement
        .query_map(params![&commit_id.raw_bytes()[..]], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                decode_commit_id(row.get::<_, Vec<u8>>(2)?),
            ))
        })
        .expect("query parents")
        .collect::<Result<Vec<_>, _>>()
        .expect("parents")
}

fn changeset_operation_type(connection: &Connection, commit_id: CommitId) -> String {
    connection
        .query_row(
            "SELECT changeset.operation_type
             FROM workstate_commit
             JOIN changeset ON changeset.changeset_id = workstate_commit.changeset_id
             WHERE workstate_commit.commit_id = ?1",
            params![&commit_id.raw_bytes()[..]],
            |row| row.get(0),
        )
        .expect("operation type")
}

fn change_operation_count(connection: &Connection, commit_id: CommitId) -> i64 {
    connection
        .query_row(
            "SELECT count(*)
             FROM workstate_commit
             JOIN change_operation
               ON change_operation.changeset_id = workstate_commit.changeset_id
             WHERE workstate_commit.commit_id = ?1",
            params![&commit_id.raw_bytes()[..]],
            |row| row.get(0),
        )
        .expect("change operation count")
}

#[test]
fn continue_merge_applies_frozen_theirs_resolution_and_completes_attempt() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);

    let base = create_task(
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
        .expect("fork source");
    let target_task = create_task(
        &mut engine,
        workspace.initial_branch_id,
        base.commit_id,
        "Target task",
    );
    let source_task = create_task(
        &mut engine,
        source_branch.branch_id,
        base.commit_id,
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
    engine
        .resolve_merge_item(
            MergeResolveOptions::theirs(item_id)
                .expect("theirs options")
                .with_rationale(rationale("take source"))
                .expect("rationale"),
        )
        .expect("resolve theirs");
    engine
        .freeze_merge_resolutions(MergeFreezeResolutionsOptions::new(merge.merge_id))
        .expect("freeze");

    let continued = engine
        .continue_merge(
            MergeContinueOptions::new(merge.merge_id)
                .expect("continue options")
                .with_detail(rationale("complete merge"))
                .expect("detail"),
        )
        .expect("continue merge");
    assert_eq!(continued.merge_id, merge.merge_id);
    assert_eq!(continued.workspace_id, workspace.workspace_id);
    assert_eq!(continued.target_branch_id, workspace.initial_branch_id);
    assert_eq!(continued.source_branch_id, source_branch.branch_id);
    assert_eq!(continued.runtime_state, MergeRuntimeState::Completed);

    let branch_head = engine
        .branch_head(workspace.initial_branch_id)
        .expect("branch head");
    assert_eq!(branch_head.head_commit_id, continued.result_commit_id);

    let replayed = engine
        .state_at(continued.result_commit_id)
        .expect("replay merge commit");
    let expected_state = WorkState::new(
        [
            (base.task_entity_id, base.task_entity_version_id),
            (
                target_task.task_entity_id,
                target_task.task_entity_version_id,
            ),
            (
                source_task.task_entity_id,
                source_task.task_entity_version_id,
            ),
        ],
        [],
    )
    .expect("expected workstate");
    assert_eq!(replayed.state, expected_state);
    assert_eq!(
        continued.work_state_digest,
        work_state_mapping_digest(&expected_state)
    );
    assert_eq!(replayed.state_digest, continued.work_state_digest);

    let shown = engine
        .merge_attempt(merge.merge_id)
        .expect("show completed");
    assert_eq!(shown.runtime_state, MergeRuntimeState::Completed);
    let outcome = shown.outcome.expect("completed outcome");
    assert_eq!(outcome.outcome, MergeOutcome::Completed);
    assert_eq!(outcome.result_commit_id, Some(continued.result_commit_id));

    let active = engine
        .merge_attempts(MergeListOptions::new(workspace.workspace_id))
        .expect("active merges");
    assert!(active.merges.is_empty());
    let closed = engine
        .merge_attempts(MergeListOptions::new(workspace.workspace_id).include_closed())
        .expect("closed merges");
    assert_eq!(closed.merges.len(), 1);

    let connection = raw_connection(&path);
    assert_eq!(
        commit_parents(&connection, continued.result_commit_id),
        vec![
            (0, "primary".to_owned(), target_task.commit_id),
            (1, "secondary".to_owned(), source_task.commit_id),
        ]
    );
    assert_eq!(
        changeset_operation_type(&connection, continued.result_commit_id),
        "merge.continue"
    );
    assert_eq!(
        change_operation_count(&connection, continued.result_commit_id),
        1
    );
}

#[test]
fn continue_merge_requires_frozen_resolutions() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);

    let base = create_task(
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
        .expect("fork source");
    create_task(
        &mut engine,
        source_branch.branch_id,
        base.commit_id,
        "Source task",
    );
    let merge = engine
        .start_merge(MergeStartOptions::new(
            workspace.initial_branch_id,
            source_branch.branch_id,
        ))
        .expect("start merge");

    let error = engine
        .continue_merge(MergeContinueOptions::new(merge.merge_id).expect("continue options"))
        .expect_err("continue without freeze");
    assert_eq!(error.code(), ErrorCode::WorkspaceInvalid);
}

#[test]
fn continue_merge_with_all_ours_records_noop_merge_commit() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);

    let base = create_task(
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
        .expect("fork source");
    let target_task = create_task(
        &mut engine,
        workspace.initial_branch_id,
        base.commit_id,
        "Target task",
    );
    let source_task = create_task(
        &mut engine,
        source_branch.branch_id,
        base.commit_id,
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
    engine
        .resolve_merge_item(MergeResolveOptions::ours(item_id).expect("ours options"))
        .expect("resolve ours");
    engine
        .freeze_merge_resolutions(MergeFreezeResolutionsOptions::new(merge.merge_id))
        .expect("freeze");

    let continued = engine
        .continue_merge(MergeContinueOptions::new(merge.merge_id).expect("continue options"))
        .expect("continue all ours");
    let replayed = engine
        .state_at(continued.result_commit_id)
        .expect("replay no-op merge");
    assert_eq!(
        replayed.state,
        WorkState::new(
            [
                (base.task_entity_id, base.task_entity_version_id),
                (
                    target_task.task_entity_id,
                    target_task.task_entity_version_id,
                ),
            ],
            [],
        )
        .expect("target state")
    );
    assert!(!replayed.state.entities().contains(&(
        source_task.task_entity_id,
        source_task.task_entity_version_id
    )));

    let connection = raw_connection(&path);
    assert_eq!(
        commit_parents(&connection, continued.result_commit_id),
        vec![
            (0, "primary".to_owned(), target_task.commit_id),
            (1, "secondary".to_owned(), source_task.commit_id),
        ]
    );
    assert_eq!(
        change_operation_count(&connection, continued.result_commit_id),
        0
    );
}

#[test]
fn continue_merge_rejects_frozen_custom_resolution_for_this_slice() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);

    let base = create_task(
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
        .expect("fork source");
    create_task(
        &mut engine,
        source_branch.branch_id,
        base.commit_id,
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
    engine
        .resolve_merge_item(
            MergeResolveOptions::custom(item_id, rationale("custom"))
                .expect("custom options")
                .with_rationale(rationale("custom choice"))
                .expect("rationale"),
        )
        .expect("resolve custom");
    engine
        .freeze_merge_resolutions(MergeFreezeResolutionsOptions::new(merge.merge_id))
        .expect("freeze");

    let error = engine
        .continue_merge(MergeContinueOptions::new(merge.merge_id).expect("continue options"))
        .expect_err("custom continue is deferred");
    assert_eq!(error.code(), ErrorCode::WorkspaceInvalid);

    let shown = engine.merge_attempt(merge.merge_id).expect("show active");
    assert_eq!(shown.runtime_state, MergeRuntimeState::Active);
    assert_eq!(
        shown.items[0]
            .resolution
            .as_ref()
            .expect("runtime resolution")
            .resolution_kind,
        MergeResolutionKind::Custom
    );
}
