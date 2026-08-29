use tempfile::TempDir;
use workvcs_core::{
    BranchForkOptions, BranchId, BranchProjectionRefreshOptions, BranchProjectionStatus,
    CanonicalValue, CommitId, Engine, MergeContinueOptions, MergeFreezeResolutionsOptions,
    MergeResolveOptions, MergeStartOptions, StoreInitOptions, TaskCreateCommit, TaskCreateOptions,
    TaskSchedulingRelationCreateOptions, WorkStateRestoreOptions, WorkspaceInfo,
    WorkspaceInitOptions,
};

fn store_path() -> (TempDir, std::path::PathBuf) {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let path = tempdir.path().join("workvcs.sqlite");
    (tempdir, path)
}

fn init_engine(path: &std::path::Path) -> Engine {
    Engine::init(
        path,
        StoreInitOptions::new("phase4k-projection-invalidation-store").expect("store options"),
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

fn assert_projection_complete(
    engine: &Engine,
    branch_id: BranchId,
    projected_commit_id: CommitId,
    entity_count: usize,
    relation_count: usize,
) {
    let projection = engine
        .branch_projection(branch_id)
        .expect("current projection");
    assert_eq!(projection.status, BranchProjectionStatus::Complete);
    assert!(projection.is_current());
    assert_eq!(projection.projected_commit_id, Some(projected_commit_id));
    assert_eq!(projection.entity_count, entity_count);
    assert_eq!(projection.relation_count, relation_count);
}

fn assert_projection_not_materialized(
    engine: &Engine,
    branch_id: BranchId,
    head_commit_id: CommitId,
) {
    let projection = engine
        .branch_projection(branch_id)
        .expect("not materialized projection");
    assert_eq!(projection.status, BranchProjectionStatus::NotMaterialized);
    assert_eq!(
        projection.stored_status.as_deref(),
        Some("not_materialized")
    );
    assert!(!projection.is_current());
    assert_eq!(projection.head_commit_id, head_commit_id);
    assert_eq!(projection.projected_commit_id, None);
    assert_eq!(projection.projection_state_digest, None);
    assert_eq!(projection.entity_count, 0);
    assert_eq!(projection.relation_count, 0);
}

#[test]
fn entity_transition_invalidates_materialized_projection() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    engine
        .refresh_branch_projection(BranchProjectionRefreshOptions::new(
            workspace.initial_branch_id,
        ))
        .expect("refresh genesis projection");
    assert_projection_complete(
        &engine,
        workspace.initial_branch_id,
        workspace.genesis_commit_id,
        0,
        0,
    );

    let task = create_task(
        &mut engine,
        workspace.initial_branch_id,
        workspace.genesis_commit_id,
        "Invalidate projection",
    );
    assert_projection_not_materialized(&engine, workspace.initial_branch_id, task.commit_id);
}

#[test]
fn task_relation_create_invalidates_materialized_projection() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let prerequisite = create_task(
        &mut engine,
        workspace.initial_branch_id,
        workspace.genesis_commit_id,
        "Prerequisite",
    );
    let dependent = create_task(
        &mut engine,
        workspace.initial_branch_id,
        prerequisite.commit_id,
        "Dependent",
    );
    engine
        .refresh_branch_projection(BranchProjectionRefreshOptions::new(
            workspace.initial_branch_id,
        ))
        .expect("refresh task projection");
    assert_projection_complete(
        &engine,
        workspace.initial_branch_id,
        dependent.commit_id,
        2,
        0,
    );

    let relation = engine
        .create_task_scheduling_relation(
            TaskSchedulingRelationCreateOptions::depends_on(
                workspace.initial_branch_id,
                dependent.commit_id,
                dependent.task_entity_id,
                prerequisite.task_entity_id,
            )
            .expect("dependency options"),
        )
        .expect("create dependency");
    assert_projection_not_materialized(&engine, workspace.initial_branch_id, relation.commit_id);
}

#[test]
fn restore_invalidates_materialized_projection() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let task = create_task(
        &mut engine,
        workspace.initial_branch_id,
        workspace.genesis_commit_id,
        "Temporary task",
    );
    engine
        .refresh_branch_projection(BranchProjectionRefreshOptions::new(
            workspace.initial_branch_id,
        ))
        .expect("refresh task projection");
    assert_projection_complete(&engine, workspace.initial_branch_id, task.commit_id, 1, 0);

    let restored = engine
        .restore_work_state(
            WorkStateRestoreOptions::new(
                workspace.initial_branch_id,
                task.commit_id,
                workspace.genesis_commit_id,
            )
            .expect("restore options"),
        )
        .expect("restore");
    assert_projection_not_materialized(&engine, workspace.initial_branch_id, restored.commit_id);
}

#[test]
fn merge_continue_invalidates_target_branch_projection() {
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
    let _source_task = create_task(
        &mut engine,
        source_branch.branch_id,
        base.commit_id,
        "Source task",
    );
    engine
        .refresh_branch_projection(BranchProjectionRefreshOptions::new(
            workspace.initial_branch_id,
        ))
        .expect("refresh target projection");
    assert_projection_complete(
        &engine,
        workspace.initial_branch_id,
        target_task.commit_id,
        2,
        0,
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
    assert_projection_complete(
        &engine,
        workspace.initial_branch_id,
        target_task.commit_id,
        2,
        0,
    );

    let continued = engine
        .continue_merge(MergeContinueOptions::new(merge.merge_id).expect("continue options"))
        .expect("continue merge");
    assert_projection_not_materialized(
        &engine,
        workspace.initial_branch_id,
        continued.result_commit_id,
    );
}
