use tempfile::TempDir;
use workvcs_core::{
    Engine, KnowledgeSpaceCreateOptions, KnowledgeSpaceListOptions, StoreInitOptions, WorkVcsError,
};

fn store_path() -> (TempDir, std::path::PathBuf) {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let path = tempdir.path().join("workvcs.sqlite");
    (tempdir, path)
}

fn init_engine(path: &std::path::Path, display_name: &str) -> Engine {
    Engine::init(
        path,
        StoreInitOptions::new(display_name).expect("store options"),
    )
    .expect("init engine")
}

#[test]
fn knowledge_space_create_show_and_list_use_store_local_identity() {
    let (_tempdir, path) = store_path();
    let mut engine = init_engine(&path, "phase4ad-knowledge-space-store");

    let first = engine
        .create_knowledge_space(KnowledgeSpaceCreateOptions::new("Research").expect("first name"))
        .expect("create first knowledge space");
    let second = engine
        .create_knowledge_space(KnowledgeSpaceCreateOptions::new("research").expect("second name"))
        .expect("create case-distinct knowledge space");

    assert_ne!(
        first.knowledge_space.knowledge_space_id,
        second.knowledge_space.knowledge_space_id
    );
    assert_eq!(first.knowledge_space.name, "Research");
    assert_eq!(second.knowledge_space.name, "research");
    assert!(first.knowledge_space.created_at_us > 0);

    let shown = engine
        .knowledge_space(first.knowledge_space.knowledge_space_id)
        .expect("show knowledge space");
    assert_eq!(shown, first.knowledge_space);

    let listed = engine
        .knowledge_spaces(KnowledgeSpaceListOptions::new())
        .expect("list knowledge spaces");
    assert_eq!(listed.knowledge_spaces.len(), 2);
    assert_eq!(listed.knowledge_spaces[0], second.knowledge_space);
    assert_eq!(listed.knowledge_spaces[1], first.knowledge_space);
}

#[test]
fn knowledge_space_rejects_duplicate_and_invalid_names() {
    let (_tempdir, path) = store_path();
    let mut engine = init_engine(&path, "phase4ad-duplicates");

    engine
        .create_knowledge_space(KnowledgeSpaceCreateOptions::new("Research").expect("name"))
        .expect("create knowledge space");
    let duplicate =
        engine.create_knowledge_space(KnowledgeSpaceCreateOptions::new("Research").expect("name"));
    assert!(matches!(duplicate, Err(WorkVcsError::QueryInvalid(_))));

    let empty = KnowledgeSpaceCreateOptions::new("");
    assert!(matches!(empty, Err(WorkVcsError::QueryInvalid(_))));

    let padded = KnowledgeSpaceCreateOptions::new(" Research ");
    assert!(matches!(padded, Err(WorkVcsError::QueryInvalid(_))));

    let control = KnowledgeSpaceCreateOptions::new("Research\nSpace");
    assert!(matches!(control, Err(WorkVcsError::QueryInvalid(_))));
}

#[test]
fn knowledge_space_list_rejects_zero_limit() {
    let error = KnowledgeSpaceListOptions::new()
        .with_limit(0)
        .expect_err("zero limit must fail");

    assert!(matches!(error, WorkVcsError::QueryInvalid(_)));
}
