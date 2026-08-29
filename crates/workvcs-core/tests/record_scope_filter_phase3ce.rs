use std::path::{Path, PathBuf};
use tempfile::TempDir;
use workvcs_core::{
    CanonicalValue, Engine, ErrorCode, RecordCreateOptions, RecordKind, RecordListOptions,
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
        StoreInitOptions::new("phase3ce-record-scope-filter-store").expect("store options"),
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
fn record_list_filters_by_scope_using_canonical_semantics() {
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

    let finding = engine
        .create_record(
            RecordCreateOptions::finding(
                workspace.initial_branch_id,
                workspace.genesis_commit_id,
                "Schema validation has no drift",
            )
            .expect("finding options")
            .with_scope(workspace_scope)
            .expect("workspace scope"),
        )
        .expect("create finding");

    let assumption = engine
        .create_record(
            RecordCreateOptions::assumption(
                workspace.initial_branch_id,
                finding.commit_id,
                "Serialized writes are sufficient",
            )
            .expect("assumption options")
            .with_scope(module_scope)
            .expect("module scope"),
        )
        .expect("create assumption");

    let equivalent_workspace_scope = object(vec![
        ("kind", CanonicalValue::String("workspace".to_owned())),
        ("local_ref", CanonicalValue::String("root".to_owned())),
    ]);
    let scoped = engine
        .records_at(
            RecordListOptions::new(assumption.commit_id)
                .with_scope(equivalent_workspace_scope)
                .expect("scope filter"),
        )
        .expect("scope-filtered records");

    assert_eq!(scoped.records.len(), 1);
    assert_eq!(scoped.records[0].record_entity_id, finding.record_entity_id);

    let combined = engine
        .records_at(
            RecordListOptions::new(assumption.commit_id)
                .with_kind(RecordKind::Assumption)
                .with_scope(object(vec![
                    ("local_ref", CanonicalValue::String("core".to_owned())),
                    ("kind", CanonicalValue::String("module".to_owned())),
                ]))
                .expect("module scope filter")
                .with_statement_contains("writes")
                .expect("statement filter"),
        )
        .expect("combined-filtered records");

    assert_eq!(combined.records.len(), 1);
    assert_eq!(
        combined.records[0].record_entity_id,
        assumption.record_entity_id
    );

    let absent = engine
        .records_at(
            RecordListOptions::new(assumption.commit_id)
                .with_scope(object(vec![(
                    "kind",
                    CanonicalValue::String("session".to_owned()),
                )]))
                .expect("absent scope filter"),
        )
        .expect("absent scope-filtered records");

    assert!(absent.records.is_empty());
}

#[test]
fn record_scope_filter_requires_object_scope() {
    let (_tempdir, path) = store_path();
    let (_engine, workspace) = create_workspace(&path);

    let error = RecordListOptions::new(workspace.genesis_commit_id)
        .with_scope(CanonicalValue::String("workspace".to_owned()))
        .expect_err("non-object scope should be rejected");

    assert_eq!(error.code(), ErrorCode::RecordInvalid);
}
