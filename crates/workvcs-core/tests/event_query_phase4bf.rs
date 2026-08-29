use tempfile::TempDir;
use workvcs_core::{
    Engine, EventListOptions, SessionStartOptions, StoreInitOptions, TaskCreateOptions,
    WorkspaceInfo, WorkspaceInitOptions, content_object_digest,
};

fn create_store() -> (TempDir, Engine, WorkspaceInfo) {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let path = tempdir.path().join("workvcs.sqlite");
    let mut engine = Engine::init(
        &path,
        StoreInitOptions::new("phase4bf-event-query-store").expect("store options"),
    )
    .expect("init engine");
    let workspace = engine
        .create_workspace(WorkspaceInitOptions::new("workspace").expect("workspace options"))
        .expect("create workspace");
    (tempdir, engine, workspace)
}

#[test]
fn events_are_queryable_by_changeset_session_and_workspace() {
    let (_tempdir, mut engine, workspace) = create_store();
    let task = engine
        .create_task(
            TaskCreateOptions::new(
                workspace.initial_branch_id,
                workspace.genesis_commit_id,
                "event-query task",
            )
            .expect("task options"),
        )
        .expect("create task");
    let session = engine
        .start_session(
            SessionStartOptions::new(workspace.workspace_id, workspace.initial_branch_id)
                .expect("session options"),
        )
        .expect("start session");

    let changeset_events = engine
        .events(EventListOptions::for_changeset(task.changeset_id))
        .expect("changeset events");
    assert_eq!(changeset_events.events.len(), 1);
    let task_event = &changeset_events.events[0];
    assert_eq!(task_event.workspace_id, Some(workspace.workspace_id));
    assert_eq!(task_event.changeset_id, Some(task.changeset_id));
    assert_eq!(task_event.session_id, None);
    assert_eq!(task_event.event_kind, "entity.transitioned");
    assert_eq!(
        task_event.payload_digest,
        content_object_digest(task_event.payload_json.as_bytes())
    );
    assert_eq!(
        task_event.payload_size_bytes,
        i64::try_from(task_event.payload_json.len()).expect("payload size")
    );

    let shown = engine.event(task_event.event_id).expect("show event");
    assert_eq!(shown, *task_event);

    let session_events = engine
        .events(EventListOptions::for_session(session.session_id))
        .expect("session events");
    assert_eq!(session_events.events.len(), 1);
    assert_eq!(
        session_events.events[0].session_id,
        Some(session.session_id)
    );
    assert_eq!(session_events.events[0].event_kind, "session.started");

    let workspace_events = engine
        .events(
            EventListOptions::for_workspace(workspace.workspace_id)
                .with_limit(2)
                .unwrap(),
        )
        .expect("workspace events");
    assert_eq!(workspace_events.events.len(), 2);
    assert!(
        workspace_events
            .events
            .windows(2)
            .all(|events| events[0].occurred_at_us <= events[1].occurred_at_us)
    );
}

#[test]
fn event_list_rejects_zero_limit() {
    let (_tempdir, _engine, workspace) = create_store();
    let result = EventListOptions::for_workspace(workspace.workspace_id).with_limit(0);
    assert!(result.is_err());
}
