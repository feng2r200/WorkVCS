use std::path::{Path, PathBuf};
use tempfile::TempDir;
use workvcs_core::{
    Engine, KnowledgeCreateOptions, ResolvedWhyQuerySubject, StoreInitOptions, WhyEntityKind,
    WhyQueryOptions, WhyQueryTarget, WorkspaceInfo, WorkspaceInitOptions,
};

fn store_path() -> (TempDir, PathBuf) {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let path = tempdir.path().join("workvcs.sqlite");
    (tempdir, path)
}

fn create_workspace(path: &Path) -> (Engine, WorkspaceInfo) {
    let mut engine = Engine::init(
        path,
        StoreInitOptions::new("phase3bs-why-knowledge-subject-store").expect("store options"),
    )
    .expect("init engine");
    let workspace = engine
        .create_workspace(WorkspaceInitOptions::new("workspace").expect("workspace options"))
        .expect("create workspace");
    (engine, workspace)
}

#[test]
fn why_resolves_knowledge_subject_kind_without_relation_expansion() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let knowledge = engine
        .create_knowledge(
            KnowledgeCreateOptions::new(
                workspace.initial_branch_id,
                workspace.genesis_commit_id,
                "Context summaries expose active knowledge",
            )
            .expect("knowledge options"),
        )
        .expect("create knowledge");

    let why = engine
        .why(WhyQueryOptions::for_entity(
            WhyQueryTarget::branch_head(workspace.initial_branch_id),
            knowledge.knowledge_entity_id,
        ))
        .expect("why knowledge");

    assert_eq!(why.target.commit_id, knowledge.commit_id);
    assert_eq!(
        why.subject,
        ResolvedWhyQuerySubject::Entity {
            entity_id: knowledge.knowledge_entity_id,
            entity_version_id: knowledge.knowledge_entity_version_id,
            entity_kind: WhyEntityKind::Knowledge,
        }
    );
    assert!(why.relation_edges.is_empty());
    assert!(why.deferred_relation_families.is_empty());
}
