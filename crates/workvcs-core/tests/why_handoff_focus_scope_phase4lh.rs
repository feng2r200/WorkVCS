use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use tempfile::TempDir;
use workvcs_core::{
    CanonicalValue, CommitId, Engine, EntityId, RecordCreateCommit, RecordCreateOptions,
    ResolvedWhyQuerySubject, SessionId, StoreInitOptions, TaskCreateCommit, TaskCreateOptions,
    WhyEntityKind, WhyQueryOptions, WhyQueryResult, WhyQueryTarget, WhyRelationDirection,
    WhyRelationEndpoint, WhyScopeLinkKind, WorkspaceInfo, WorkspaceInitOptions,
};

fn store_path() -> (TempDir, PathBuf) {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let path = tempdir.path().join("workvcs.sqlite");
    (tempdir, path)
}

fn init_engine(path: &Path) -> Engine {
    Engine::init(
        path,
        StoreInitOptions::new("phase4lh-why-handoff-focus-store").expect("store options"),
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

fn create_task(engine: &mut Engine, workspace: &WorkspaceInfo) -> TaskCreateCommit {
    engine
        .create_task(
            TaskCreateOptions::new(
                workspace.initial_branch_id,
                workspace.genesis_commit_id,
                "Continue focused handoff",
            )
            .expect("task options"),
        )
        .expect("create task")
}

fn focused_handoff_scope(task_entity_id: EntityId) -> CanonicalValue {
    CanonicalValue::object(vec![
        (
            "focus_entity_id".to_owned(),
            CanonicalValue::String(task_entity_id.to_string()),
        ),
        (
            "handoff_scope_schema_version".to_owned(),
            CanonicalValue::safe_integer(1).expect("safe integer"),
        ),
        ("session_diff_id".to_owned(), CanonicalValue::Null),
        (
            "session_id".to_owned(),
            CanonicalValue::String(SessionId::new_v7().to_string()),
        ),
        (
            "session_lifecycle_state".to_owned(),
            CanonicalValue::String("ended".to_owned()),
        ),
    ])
    .expect("focused handoff scope")
}

fn create_focused_handoff(
    engine: &mut Engine,
    workspace: &WorkspaceInfo,
    task: &TaskCreateCommit,
) -> RecordCreateCommit {
    engine
        .create_record(
            RecordCreateOptions::handoff(
                workspace.initial_branch_id,
                task.commit_id,
                "Continue focused implementation",
            )
            .expect("handoff options")
            .with_scope(focused_handoff_scope(task.task_entity_id))
            .expect("handoff scope"),
        )
        .expect("create focused handoff")
}

fn create_generic_handoff(
    engine: &mut Engine,
    workspace: &WorkspaceInfo,
    head_commit_id: CommitId,
) -> RecordCreateCommit {
    let generic_scope = CanonicalValue::object(vec![(
        "focus".to_owned(),
        CanonicalValue::String("task:next".to_owned()),
    )])
    .expect("generic scope");
    engine
        .create_record(
            RecordCreateOptions::handoff(
                workspace.initial_branch_id,
                head_commit_id,
                "Generic handoff note",
            )
            .expect("handoff options")
            .with_scope(generic_scope)
            .expect("generic handoff scope"),
        )
        .expect("create generic handoff")
}

fn create_handoff_with_scope(
    engine: &mut Engine,
    workspace: &WorkspaceInfo,
    head_commit_id: CommitId,
    statement: &str,
    scope: CanonicalValue,
) -> RecordCreateCommit {
    engine
        .create_record(
            RecordCreateOptions::handoff(workspace.initial_branch_id, head_commit_id, statement)
                .expect("handoff options")
                .with_scope(scope)
                .expect("handoff scope"),
        )
        .expect("create handoff with scope")
}

fn why_commit(engine: &Engine, commit_id: CommitId, subject_entity_id: EntityId) -> WhyQueryResult {
    engine
        .why(WhyQueryOptions::for_entity(
            WhyQueryTarget::commit(commit_id),
            subject_entity_id,
        ))
        .expect("why commit")
}

fn endpoint(entity_id: EntityId, entity_kind: WhyEntityKind) -> WhyRelationEndpoint {
    WhyRelationEndpoint::entity(entity_id, entity_kind)
}

fn scope_link_facts(
    why: &WhyQueryResult,
) -> BTreeSet<(
    WhyScopeLinkKind,
    WhyRelationDirection,
    WhyRelationEndpoint,
    WhyRelationEndpoint,
)> {
    why.scope_links
        .iter()
        .map(|link| (link.link_kind, link.direction, link.source, link.target))
        .collect()
}

#[test]
fn why_reports_focused_handoff_scope_link_for_both_endpoints() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let task = create_task(&mut engine, &workspace);
    let handoff = create_focused_handoff(&mut engine, &workspace, &task);

    let handoff_why = why_commit(&engine, handoff.commit_id, handoff.record_entity_id);
    assert_eq!(
        handoff_why.subject,
        ResolvedWhyQuerySubject::Entity {
            entity_id: handoff.record_entity_id,
            entity_version_id: handoff.record_entity_version_id,
            entity_kind: WhyEntityKind::Record,
        }
    );
    assert!(handoff_why.relation_edges.is_empty());
    assert_eq!(
        scope_link_facts(&handoff_why),
        BTreeSet::from([(
            WhyScopeLinkKind::HandoffFocus,
            WhyRelationDirection::Outgoing,
            endpoint(handoff.record_entity_id, WhyEntityKind::Record),
            endpoint(task.task_entity_id, WhyEntityKind::Task),
        )])
    );
    assert_eq!(
        handoff_why.scope_links[0].source_entity_version_id,
        handoff.record_entity_version_id
    );
    assert_eq!(
        handoff_why.scope_links[0].target_entity_version_id,
        task.task_entity_version_id
    );
    assert_eq!(
        handoff_why.scope_links[0].source_state_digest,
        handoff.record_state_digest
    );
    assert!(handoff_why.deferred_relation_families.is_empty());

    let task_why = why_commit(&engine, handoff.commit_id, task.task_entity_id);
    assert_eq!(
        task_why.subject,
        ResolvedWhyQuerySubject::Entity {
            entity_id: task.task_entity_id,
            entity_version_id: task.task_entity_version_id,
            entity_kind: WhyEntityKind::Task,
        }
    );
    assert!(task_why.relation_edges.is_empty());
    assert_eq!(
        scope_link_facts(&task_why),
        BTreeSet::from([(
            WhyScopeLinkKind::HandoffFocus,
            WhyRelationDirection::Incoming,
            endpoint(handoff.record_entity_id, WhyEntityKind::Record),
            endpoint(task.task_entity_id, WhyEntityKind::Task),
        )])
    );
    assert_eq!(
        task_why.scope_links[0].source_entity_version_id,
        handoff.record_entity_version_id
    );
    assert_eq!(
        task_why.scope_links[0].target_entity_version_id,
        task.task_entity_version_id
    );
    assert_eq!(
        task_why.scope_links[0].source_state_digest,
        handoff.record_state_digest
    );
    assert!(task_why.deferred_relation_families.is_empty());
}

#[test]
fn why_does_not_infer_scope_links_from_generic_handoff_scope() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let generic_handoff =
        create_generic_handoff(&mut engine, &workspace, workspace.genesis_commit_id);

    let why = why_commit(
        &engine,
        generic_handoff.commit_id,
        generic_handoff.record_entity_id,
    );
    assert!(why.relation_edges.is_empty());
    assert!(why.scope_links.is_empty());
    assert!(why.deferred_relation_families.is_empty());
}

#[test]
fn why_skips_unrecognized_handoff_scope_without_blocking_unrelated_subjects() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let task = create_task(&mut engine, &workspace);
    let malformed_v1_scope = CanonicalValue::object(vec![(
        "handoff_scope_schema_version".to_owned(),
        CanonicalValue::safe_integer(1).expect("safe integer"),
    )])
    .expect("malformed v1 scope");
    let malformed_handoff = create_handoff_with_scope(
        &mut engine,
        &workspace,
        task.commit_id,
        "Malformed focused handoff",
        malformed_v1_scope,
    );
    let future_scope = CanonicalValue::object(vec![
        (
            "focus_entity_id".to_owned(),
            CanonicalValue::String(task.task_entity_id.to_string()),
        ),
        (
            "handoff_scope_schema_version".to_owned(),
            CanonicalValue::safe_integer(2).expect("safe integer"),
        ),
    ])
    .expect("future scope");
    let future_handoff = create_handoff_with_scope(
        &mut engine,
        &workspace,
        malformed_handoff.commit_id,
        "Future focused handoff",
        future_scope,
    );

    let why = why_commit(&engine, future_handoff.commit_id, task.task_entity_id);
    assert!(why.relation_edges.is_empty());
    assert!(why.scope_links.is_empty());
    assert!(why.deferred_relation_families.is_empty());
}
