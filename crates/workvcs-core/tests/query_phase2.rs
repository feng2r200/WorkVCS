use rusqlite::{Connection, params};
use std::path::{Path, PathBuf};
use tempfile::TempDir;
use workvcs_core::{
    BranchId, CanonicalValue, CommitId, Engine, EntityTransitionCommit, EntityTransitionOptions,
    ErrorCategory, ErrorCode, EventId, HistoryQueryOptions, StoreInitOptions, WorkspaceInfo,
    WorkspaceInitOptions,
};

fn store_path() -> (TempDir, PathBuf) {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let path = tempdir.path().join("workvcs.sqlite");
    (tempdir, path)
}

fn init_engine(path: &Path) -> Engine {
    Engine::init(
        path,
        StoreInitOptions::new("phase2-query-store").expect("store options"),
    )
    .expect("init engine")
}

fn create_workspace(path: &Path) -> (Engine, WorkspaceInfo) {
    let mut engine = init_engine(path);
    let workspace = engine
        .create_workspace(WorkspaceInitOptions::new("workspace").expect("workspace options"))
        .expect("create workspace");
    (engine, workspace)
}

fn record_state(title: &str, status: &str) -> CanonicalValue {
    CanonicalValue::object(vec![
        ("title".to_owned(), CanonicalValue::String(title.to_owned())),
        (
            "status".to_owned(),
            CanonicalValue::String(status.to_owned()),
        ),
    ])
    .expect("record state")
}

fn create_two_transitions(
    path: &Path,
) -> (
    Engine,
    WorkspaceInfo,
    EntityTransitionCommit,
    EntityTransitionCommit,
) {
    let (mut engine, workspace) = create_workspace(path);
    let first = engine
        .commit_entity_transition(
            EntityTransitionOptions::create(
                workspace.initial_branch_id,
                workspace.genesis_commit_id,
                "generic_record",
                record_state("first", "open"),
            )
            .expect("create options"),
        )
        .expect("first transition");
    let second = engine
        .commit_entity_transition(
            EntityTransitionOptions::update(
                workspace.initial_branch_id,
                first.commit_id,
                first.entity_id,
                first.entity_version_id,
                record_state("first", "done"),
            )
            .expect("update options"),
        )
        .expect("second transition");
    (engine, workspace, first, second)
}

fn raw_connection(path: &Path) -> Connection {
    let connection = Connection::open(path).expect("raw connection");
    connection
        .pragma_update(None, "foreign_keys", "ON")
        .expect("enable raw foreign keys");
    connection
}

#[test]
fn branch_head_reports_current_branch_head() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);

    let genesis_head = engine
        .branch_head(workspace.initial_branch_id)
        .expect("genesis branch head");
    assert_eq!(genesis_head.workspace_id, workspace.workspace_id);
    assert_eq!(genesis_head.branch_id, workspace.initial_branch_id);
    assert_eq!(genesis_head.name, workspace.initial_branch_name);
    assert_eq!(genesis_head.head_commit_id, workspace.genesis_commit_id);
    assert_eq!(
        genesis_head.head_changeset_id,
        workspace.genesis_changeset_id
    );
    assert_eq!(genesis_head.head_commit_kind, "genesis");
    assert_eq!(genesis_head.head_operation_type, "workspace.genesis");
    assert_eq!(genesis_head.head_operation_schema_version, 1);
    assert_eq!(genesis_head.lifecycle_state, "active");
    assert_eq!(genesis_head.state_digest, workspace.state_digest);

    let transition = engine
        .commit_entity_transition(
            EntityTransitionOptions::create(
                workspace.initial_branch_id,
                workspace.genesis_commit_id,
                "generic_record",
                record_state("head", "open"),
            )
            .expect("create options"),
        )
        .expect("transition");

    let updated_head = engine
        .branch_head(workspace.initial_branch_id)
        .expect("updated branch head");
    assert_eq!(updated_head.head_commit_id, transition.commit_id);
    assert_eq!(updated_head.head_changeset_id, transition.changeset_id);
    assert_eq!(updated_head.head_commit_kind, "normal");
    assert_eq!(updated_head.head_operation_type, "entity.transition");
    assert_eq!(updated_head.head_operation_schema_version, 1);
    assert_eq!(updated_head.state_digest, transition.work_state_digest);
}

#[test]
fn history_from_branch_lists_first_parent_chain_newest_first() {
    let (_tempdir, path) = store_path();
    let (engine, workspace, first, second) = create_two_transitions(&path);

    let history = engine
        .history(HistoryQueryOptions::from_branch(
            workspace.initial_branch_id,
        ))
        .expect("branch history");

    assert_eq!(history.start_commit_id, second.commit_id);
    assert_eq!(history.entries.len(), 3);
    assert_eq!(history.entries[0].commit_id, second.commit_id);
    assert_eq!(history.entries[0].parent_commit_id, Some(first.commit_id));
    assert_eq!(history.entries[0].operation_type, "entity.transition");
    assert_eq!(history.entries[1].commit_id, first.commit_id);
    assert_eq!(
        history.entries[1].parent_commit_id,
        Some(workspace.genesis_commit_id)
    );
    assert_eq!(history.entries[1].operation_type, "entity.transition");
    assert_eq!(history.entries[2].commit_id, workspace.genesis_commit_id);
    assert_eq!(history.entries[2].parent_commit_id, None);
    assert_eq!(history.entries[2].operation_type, "workspace.genesis");
}

#[test]
fn history_from_commit_can_be_limited() {
    let (_tempdir, path) = store_path();
    let (engine, workspace, first, second) = create_two_transitions(&path);

    let history = engine
        .history(
            HistoryQueryOptions::from_commit(second.commit_id)
                .with_limit(2)
                .expect("limit"),
        )
        .expect("limited history");

    assert_eq!(history.start_commit_id, second.commit_id);
    assert_eq!(history.entries.len(), 2);
    assert_eq!(history.entries[0].commit_id, second.commit_id);
    assert_eq!(history.entries[1].commit_id, first.commit_id);
    assert!(history.entries.iter().all(|entry| {
        entry.workspace_id == workspace.workspace_id
            && entry.operation_schema_version == 1
            && entry.committed_at_us > 0
            && entry.changeset_created_at_us > 0
    }));
}

#[test]
fn show_at_replays_without_mutating_branch_head() {
    let (_tempdir, path) = store_path();
    let (engine, workspace, first, second) = create_two_transitions(&path);

    let first_state = engine.show_at(first.commit_id).expect("show first");
    assert_eq!(
        first_state.state.entities(),
        &[(first.entity_id, first.entity_version_id)]
    );
    let head_after_show = engine
        .branch_head(workspace.initial_branch_id)
        .expect("branch head after show");
    assert_eq!(head_after_show.head_commit_id, second.commit_id);
}

#[test]
fn history_and_show_at_ignore_projection_and_events() {
    let (_tempdir, path) = store_path();
    let (engine, workspace, first, second) = create_two_transitions(&path);

    let connection = raw_connection(&path);
    let branch_id = workspace.initial_branch_id.raw_bytes();
    let first_commit_id = first.commit_id.raw_bytes();
    let event_id = EventId::new_v7().raw_bytes();
    connection
        .execute(
            "INSERT INTO branch_projection_state(
                branch_id,
                projection_status,
                projected_commit_id,
                projection_state_digest,
                updated_at_us
             )
             VALUES (?1, 'complete', ?2, ?3, 7)
             ON CONFLICT(branch_id) DO UPDATE SET
                projection_status = excluded.projection_status,
                projected_commit_id = excluded.projected_commit_id,
                projection_state_digest = excluded.projection_state_digest,
                updated_at_us = excluded.updated_at_us",
            params![&branch_id[..], &first_commit_id[..], &[8_u8; 32][..]],
        )
        .expect("insert corrupted projection state");
    connection
        .execute(
            "INSERT INTO event(
                event_id,
                workspace_id,
                changeset_id,
                session_id,
                event_kind,
                occurred_at_us,
                payload_json
             )
             VALUES (?1, ?2, NULL, NULL, 'query.noise', 8, '{}')",
            params![&event_id[..], &workspace.workspace_id.raw_bytes()[..]],
        )
        .expect("insert extra event");
    drop(connection);

    let head = engine
        .branch_head(workspace.initial_branch_id)
        .expect("branch head");
    assert_eq!(head.head_commit_id, second.commit_id);
    assert_eq!(head.state_digest, second.work_state_digest);

    let history = engine
        .history(HistoryQueryOptions::from_branch(
            workspace.initial_branch_id,
        ))
        .expect("history");
    assert_eq!(history.start_commit_id, second.commit_id);
    assert_eq!(history.entries.len(), 3);

    let shown = engine.show_at(second.commit_id).expect("show second");
    assert_eq!(
        shown.state.entities(),
        &[(first.entity_id, second.entity_version_id)]
    );
}

#[test]
fn history_unknown_branch_is_a_query_error() {
    let (_tempdir, path) = store_path();
    let (engine, _workspace) = create_workspace(&path);

    let error = engine
        .history(HistoryQueryOptions::from_branch(BranchId::new_v7()))
        .expect_err("unknown branch");

    assert_eq!(error.code(), ErrorCode::QueryInvalid);
    assert_eq!(error.category(), ErrorCategory::Query);
}

#[test]
fn history_unknown_commit_is_a_query_error() {
    let (_tempdir, path) = store_path();
    let (engine, _workspace) = create_workspace(&path);

    let error = engine
        .history(HistoryQueryOptions::from_commit(CommitId::new_v7()))
        .expect_err("unknown commit");

    assert_eq!(error.code(), ErrorCode::QueryInvalid);
    assert_eq!(error.category(), ErrorCategory::Query);
}

#[test]
fn history_rejects_corrupted_normal_parent_shape() {
    let (_tempdir, path) = store_path();
    let (engine, workspace, _first, second) = create_two_transitions(&path);

    let connection = raw_connection(&path);
    connection
        .execute(
            "INSERT INTO commit_parent(
                commit_id,
                parent_ordinal,
                parent_role,
                parent_commit_id
             )
             VALUES (?1, 1, 'secondary', ?2)",
            params![
                &second.commit_id.raw_bytes()[..],
                &workspace.genesis_commit_id.raw_bytes()[..]
            ],
        )
        .expect("insert extra parent");
    drop(connection);

    let error = engine
        .history(HistoryQueryOptions::from_commit(second.commit_id))
        .expect_err("corrupted parent shape");

    assert_eq!(error.code(), ErrorCode::QueryInvalid);
    assert_eq!(error.category(), ErrorCategory::Query);
}

#[test]
fn history_rejects_parent_cycles() {
    let (_tempdir, path) = store_path();
    let (engine, _workspace, first, second) = create_two_transitions(&path);

    let connection = raw_connection(&path);
    connection
        .execute(
            "UPDATE commit_parent
             SET parent_commit_id = ?1
             WHERE commit_id = ?2
               AND parent_ordinal = 0",
            params![
                &second.commit_id.raw_bytes()[..],
                &first.commit_id.raw_bytes()[..]
            ],
        )
        .expect("create parent cycle");
    drop(connection);

    let error = engine
        .history(HistoryQueryOptions::from_commit(second.commit_id))
        .expect_err("cycle");

    assert_eq!(error.code(), ErrorCode::QueryInvalid);
    assert_eq!(error.category(), ErrorCategory::Query);
}
