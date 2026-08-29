use std::path::{Path, PathBuf};
use tempfile::TempDir;
use workvcs_core::{
    CanonicalValue, Engine, ErrorCode, KnowledgeCreateOptions, KnowledgeListOptions,
    StoreInitOptions, WorkspaceInfo, WorkspaceInitOptions,
};

fn store_path() -> (TempDir, PathBuf) {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let path = tempdir.path().join("workvcs.sqlite");
    (tempdir, path)
}

fn create_workspace(path: &Path) -> (Engine, WorkspaceInfo) {
    let mut engine = Engine::init(
        path,
        StoreInitOptions::new("phase3cd-knowledge-scope-filter-store").expect("store options"),
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
fn knowledge_list_filters_by_scope_using_canonical_semantics() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);

    let workspace_scope = object(vec![
        ("local_ref", CanonicalValue::String("root".to_owned())),
        ("kind", CanonicalValue::String("workspace".to_owned())),
    ]);
    let module_scope = object(vec![
        ("kind", CanonicalValue::String("module".to_owned())),
        ("local_ref", CanonicalValue::String("core".to_owned())),
    ]);

    let workspace_knowledge = engine
        .create_knowledge(
            KnowledgeCreateOptions::new(
                workspace.initial_branch_id,
                workspace.genesis_commit_id,
                "SQLite is sufficient under serialized writes",
            )
            .expect("workspace knowledge options")
            .with_scope(workspace_scope)
            .expect("workspace scope"),
        )
        .expect("create workspace knowledge");

    let module_knowledge = engine
        .create_knowledge(
            KnowledgeCreateOptions::new(
                workspace.initial_branch_id,
                workspace_knowledge.commit_id,
                "BLAKE3 state digests are stable for module projections",
            )
            .expect("module knowledge options")
            .with_scope(module_scope)
            .expect("module scope"),
        )
        .expect("create module knowledge");

    let equivalent_workspace_scope = object(vec![
        ("kind", CanonicalValue::String("workspace".to_owned())),
        ("local_ref", CanonicalValue::String("root".to_owned())),
    ]);
    let filtered = engine
        .knowledges_at(
            KnowledgeListOptions::new(module_knowledge.commit_id)
                .with_scope(equivalent_workspace_scope)
                .expect("scope filter"),
        )
        .expect("filtered knowledge list");

    assert_eq!(filtered.knowledge.len(), 1);
    assert_eq!(
        filtered.knowledge[0].knowledge_entity_id,
        workspace_knowledge.knowledge_entity_id
    );

    let combined = engine
        .knowledges_at(
            KnowledgeListOptions::new(module_knowledge.commit_id)
                .with_scope(object(vec![
                    ("local_ref", CanonicalValue::String("core".to_owned())),
                    ("kind", CanonicalValue::String("module".to_owned())),
                ]))
                .expect("module scope filter")
                .with_statement_contains("module")
                .expect("statement filter"),
        )
        .expect("combined filtered knowledge list");

    assert_eq!(combined.knowledge.len(), 1);
    assert_eq!(
        combined.knowledge[0].knowledge_entity_id,
        module_knowledge.knowledge_entity_id
    );

    let absent = engine
        .knowledges_at(
            KnowledgeListOptions::new(module_knowledge.commit_id)
                .with_scope(object(vec![(
                    "kind",
                    CanonicalValue::String("session".to_owned()),
                )]))
                .expect("absent scope filter"),
        )
        .expect("absent filtered knowledge list");

    assert!(absent.knowledge.is_empty());
}

#[test]
fn knowledge_scope_filter_requires_object_scope() {
    let (_tempdir, path) = store_path();
    let (_engine, workspace) = create_workspace(&path);

    let error = KnowledgeListOptions::new(workspace.genesis_commit_id)
        .with_scope(CanonicalValue::String("workspace".to_owned()))
        .expect_err("non-object scope should be rejected");

    assert_eq!(error.code(), ErrorCode::KnowledgeInvalid);
}
