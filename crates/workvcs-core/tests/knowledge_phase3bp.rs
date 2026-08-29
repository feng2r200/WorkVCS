use std::path::{Path, PathBuf};
use tempfile::TempDir;
use workvcs_core::{
    CanonicalValue, Engine, EntityTransitionOptions, ErrorCode, KnowledgeCreateOptions,
    KnowledgeListOptions, KnowledgeStatus, StoreInitOptions, WorkspaceInfo, WorkspaceInitOptions,
};

fn store_path() -> (TempDir, PathBuf) {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let path = tempdir.path().join("workvcs.sqlite");
    (tempdir, path)
}

fn create_workspace(path: &Path) -> (Engine, WorkspaceInfo) {
    let mut engine = Engine::init(
        path,
        StoreInitOptions::new("phase3bp-knowledge-store").expect("store options"),
    )
    .expect("init engine");
    let workspace = engine
        .create_workspace(WorkspaceInitOptions::new("workspace").expect("workspace options"))
        .expect("create workspace");
    (engine, workspace)
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

#[test]
fn create_knowledge_persists_and_lists_workspace_knowledge() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let scope = object(vec![
        ("kind", CanonicalValue::String("task".to_owned())),
        ("local_ref", CanonicalValue::String("T-1".to_owned())),
    ]);
    let provenance = object(vec![(
        "source",
        CanonicalValue::String("phase-3bp-test".to_owned()),
    )]);

    let first = engine
        .create_knowledge(
            KnowledgeCreateOptions::new(
                workspace.initial_branch_id,
                workspace.genesis_commit_id,
                "SQLite is sufficient under serialized writes",
            )
            .expect("knowledge options")
            .with_scope(scope.clone())
            .expect("knowledge scope")
            .with_provenance(provenance.clone())
            .expect("knowledge provenance"),
        )
        .expect("create knowledge");

    assert_eq!(first.workspace_id, workspace.workspace_id);
    assert_eq!(first.branch_id, workspace.initial_branch_id);
    assert_eq!(first.previous_head_commit_id, workspace.genesis_commit_id);
    assert_eq!(first.state.status, KnowledgeStatus::Active);
    assert_eq!(
        first.state.statement,
        "SQLite is sufficient under serialized writes"
    );
    assert_eq!(first.state.scope, scope);
    assert_eq!(first.state.provenance, provenance);

    let snapshot = engine
        .knowledge_at(first.commit_id, first.knowledge_entity_id)
        .expect("knowledge snapshot");
    assert_eq!(snapshot.workspace_id, workspace.workspace_id);
    assert_eq!(snapshot.commit_id, first.commit_id);
    assert_eq!(snapshot.knowledge_entity_id, first.knowledge_entity_id);
    assert_eq!(
        snapshot.knowledge_entity_version_id,
        first.knowledge_entity_version_id
    );
    assert_eq!(snapshot.state_digest, first.knowledge_state_digest);
    assert_eq!(snapshot.state, first.state);

    let second = engine
        .create_knowledge(
            KnowledgeCreateOptions::new(
                workspace.initial_branch_id,
                first.commit_id,
                "BLAKE3 state digests are stored as lowercase hex externally",
            )
            .expect("second knowledge options"),
        )
        .expect("create second knowledge");

    let historical = engine
        .knowledges_at(KnowledgeListOptions::new(first.commit_id))
        .expect("historical knowledge list");
    assert_eq!(historical.knowledge.len(), 1);
    assert_eq!(
        historical.knowledge[0].knowledge_entity_id,
        first.knowledge_entity_id
    );

    let current = engine
        .knowledges_at(KnowledgeListOptions::new(second.commit_id))
        .expect("current knowledge list");
    assert_eq!(current.knowledge.len(), 2);

    let filtered = engine
        .knowledges_at(
            KnowledgeListOptions::new(second.commit_id)
                .with_statement_contains("SQLite")
                .expect("statement filter"),
        )
        .expect("filtered knowledge list");
    assert_eq!(filtered.knowledge.len(), 1);
    assert_eq!(
        filtered.knowledge[0].knowledge_entity_id,
        first.knowledge_entity_id
    );

    let active = engine
        .knowledges_at(
            KnowledgeListOptions::new(second.commit_id).with_status(KnowledgeStatus::Active),
        )
        .expect("active knowledge list");
    assert_eq!(active.knowledge.len(), 2);

    let missing = engine
        .knowledge_at(workspace.genesis_commit_id, first.knowledge_entity_id)
        .expect_err("knowledge should not exist at genesis");
    assert_eq!(missing.code(), ErrorCode::KnowledgeNotFound);

    let reopened = Engine::open(&path).expect("reopen engine");
    let reopened_snapshot = reopened
        .knowledge_at(second.commit_id, second.knowledge_entity_id)
        .expect("reopened knowledge snapshot");
    assert_eq!(reopened_snapshot.state, second.state);
}

#[test]
fn knowledge_validation_rejects_empty_filters_and_non_object_state_parts() {
    let (_tempdir, path) = store_path();
    let (_engine, workspace) = create_workspace(&path);

    let empty = KnowledgeCreateOptions::new(
        workspace.initial_branch_id,
        workspace.genesis_commit_id,
        " ",
    )
    .expect_err("empty statement should fail");
    assert_eq!(empty.code(), ErrorCode::KnowledgeInvalid);

    let non_object_scope = KnowledgeCreateOptions::new(
        workspace.initial_branch_id,
        workspace.genesis_commit_id,
        "Reusable statement",
    )
    .expect("knowledge options")
    .with_scope(CanonicalValue::String("task".to_owned()))
    .expect_err("non-object scope should fail");
    assert_eq!(non_object_scope.code(), ErrorCode::KnowledgeInvalid);

    let non_object_provenance = KnowledgeCreateOptions::new(
        workspace.initial_branch_id,
        workspace.genesis_commit_id,
        "Reusable statement",
    )
    .expect("knowledge options")
    .with_provenance(CanonicalValue::String("source".to_owned()))
    .expect_err("non-object provenance should fail");
    assert_eq!(non_object_provenance.code(), ErrorCode::KnowledgeInvalid);

    let empty_filter = KnowledgeListOptions::new(workspace.genesis_commit_id)
        .with_statement_contains(" ")
        .expect_err("empty statement filter should fail");
    assert_eq!(empty_filter.code(), ErrorCode::KnowledgeInvalid);
}

#[test]
fn knowledge_kind_requires_semantic_api() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let raw_state = object(vec![
        ("provenance", object(Vec::new())),
        ("scope", object(Vec::new())),
        (
            "statement",
            CanonicalValue::String("Use the semantic API".to_owned()),
        ),
        ("status", CanonicalValue::String("active".to_owned())),
    ]);

    let options = EntityTransitionOptions::create(
        workspace.initial_branch_id,
        workspace.genesis_commit_id,
        "knowledge",
        raw_state,
    )
    .expect("raw entity transition options");
    let error = engine
        .commit_entity_transition(options)
        .expect_err("reserved knowledge kind should require semantic API");
    assert_eq!(error.code(), ErrorCode::EntityTransitionInvalid);
}
