use std::path::{Path, PathBuf};
use tempfile::TempDir;
use workvcs_core::{
    CommitId, Engine, KnowledgeCreateCommit, KnowledgeCreateOptions,
    KnowledgeExposureCreateLocalOptions, KnowledgeExposureDerivedFromRelationCreateOptions,
    KnowledgeSpaceCreateOptions, StoreInitOptions, WorkspaceInfo, WorkspaceInitOptions,
};

fn store_path() -> (TempDir, PathBuf) {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let path = tempdir.path().join("workvcs.sqlite");
    (tempdir, path)
}

fn create_workspace(path: &Path) -> (Engine, WorkspaceInfo) {
    let mut engine = Engine::init(
        path,
        StoreInitOptions::new("phase4am-derived-from-relation-store").expect("store options"),
    )
    .expect("init engine");
    let workspace = engine
        .create_workspace(WorkspaceInitOptions::new("workspace").expect("workspace options"))
        .expect("create workspace");
    (engine, workspace)
}

fn create_knowledge(
    engine: &mut Engine,
    workspace: &WorkspaceInfo,
    head: CommitId,
    statement: &str,
) -> KnowledgeCreateCommit {
    engine
        .create_knowledge(
            KnowledgeCreateOptions::new(workspace.initial_branch_id, head, statement)
                .expect("knowledge options"),
        )
        .expect("create knowledge")
}

#[test]
fn creates_replayable_knowledge_derived_from_exposure_relation() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let source_knowledge = create_knowledge(
        &mut engine,
        &workspace,
        workspace.genesis_commit_id,
        "Reusable source knowledge",
    );
    let adopted_knowledge = create_knowledge(
        &mut engine,
        &workspace,
        source_knowledge.commit_id,
        "Workspace-local adopted knowledge",
    );
    let knowledge_space = engine
        .create_knowledge_space(KnowledgeSpaceCreateOptions::new("Research").expect("space name"))
        .expect("create knowledge space")
        .knowledge_space;
    let exposure = engine
        .create_local_knowledge_exposure(
            KnowledgeExposureCreateLocalOptions::new(
                knowledge_space.knowledge_space_id,
                workspace.workspace_id,
                source_knowledge.knowledge_entity_id,
                source_knowledge.knowledge_entity_version_id,
            )
            .expect("exposure options"),
        )
        .expect("create exposure")
        .exposure;

    let relation = engine
        .create_knowledge_exposure_derived_from_relation(
            KnowledgeExposureDerivedFromRelationCreateOptions::new(
                workspace.initial_branch_id,
                adopted_knowledge.commit_id,
                adopted_knowledge.knowledge_entity_id,
                exposure.exposure_id,
                "adopted from exposed source",
            )
            .expect("relation options"),
        )
        .expect("create derived_from relation");

    assert_eq!(relation.workspace_id, workspace.workspace_id);
    assert_eq!(relation.branch_id, workspace.initial_branch_id);
    assert_eq!(
        relation.previous_head_commit_id,
        adopted_knowledge.commit_id
    );
    assert_eq!(relation.relation_type, "derived_from");
    assert_eq!(
        relation.knowledge_entity_id,
        adopted_knowledge.knowledge_entity_id
    );
    assert_eq!(relation.exposure_id, exposure.exposure_id);

    let branch = engine
        .branch_head(workspace.initial_branch_id)
        .expect("branch head");
    assert_eq!(branch.head_commit_id, relation.commit_id);
    let replayed = engine
        .state_at(relation.commit_id)
        .expect("replay relation");
    assert!(
        replayed
            .state
            .relations()
            .iter()
            .any(|(relation_id, relation_version_id)| {
                *relation_id == relation.relation_id
                    && *relation_version_id == relation.relation_version_id
            })
    );
    assert_eq!(replayed.state_digest, relation.work_state_digest);
}

#[test]
fn rejects_duplicate_knowledge_derived_from_exposure_relation() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let source_knowledge = create_knowledge(
        &mut engine,
        &workspace,
        workspace.genesis_commit_id,
        "Reusable source knowledge",
    );
    let adopted_knowledge = create_knowledge(
        &mut engine,
        &workspace,
        source_knowledge.commit_id,
        "Workspace-local adopted knowledge",
    );
    let knowledge_space = engine
        .create_knowledge_space(KnowledgeSpaceCreateOptions::new("Research").expect("space name"))
        .expect("create knowledge space")
        .knowledge_space;
    let exposure = engine
        .create_local_knowledge_exposure(
            KnowledgeExposureCreateLocalOptions::new(
                knowledge_space.knowledge_space_id,
                workspace.workspace_id,
                source_knowledge.knowledge_entity_id,
                source_knowledge.knowledge_entity_version_id,
            )
            .expect("exposure options"),
        )
        .expect("create exposure")
        .exposure;
    let relation = engine
        .create_knowledge_exposure_derived_from_relation(
            KnowledgeExposureDerivedFromRelationCreateOptions::new(
                workspace.initial_branch_id,
                adopted_knowledge.commit_id,
                adopted_knowledge.knowledge_entity_id,
                exposure.exposure_id,
                "adopted from exposed source",
            )
            .expect("relation options"),
        )
        .expect("create derived_from relation");

    let error = engine
        .create_knowledge_exposure_derived_from_relation(
            KnowledgeExposureDerivedFromRelationCreateOptions::new(
                workspace.initial_branch_id,
                relation.commit_id,
                adopted_knowledge.knowledge_entity_id,
                exposure.exposure_id,
                "duplicate adoption source",
            )
            .expect("duplicate relation options"),
        )
        .expect_err("duplicate relation rejected");

    assert!(error.to_string().contains("already exists"));
}
