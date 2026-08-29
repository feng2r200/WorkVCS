use std::path::{Path, PathBuf};
use tempfile::TempDir;
use workvcs_core::{
    Engine, KnowledgeCreateOptions, KnowledgeExposureAdoptOptions,
    KnowledgeExposureCreateLocalOptions, KnowledgeSpaceCreateOptions, StoreInitOptions,
    WhyQueryOptions, WhyQueryTarget, WhyRelationDirection, WhyRelationEndpoint, WhyRelationKind,
    WorkspaceInfo, WorkspaceInitOptions,
};

fn store_path() -> (TempDir, PathBuf) {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let path = tempdir.path().join("workvcs.sqlite");
    (tempdir, path)
}

fn create_workspace(path: &Path) -> (Engine, WorkspaceInfo) {
    let mut engine = Engine::init(
        path,
        StoreInitOptions::new("phase4ap-why-knowledge-exposure-subject-store")
            .expect("store options"),
    )
    .expect("init engine");
    let workspace = engine
        .create_workspace(WorkspaceInitOptions::new("workspace").expect("workspace options"))
        .expect("create workspace");
    (engine, workspace)
}

#[test]
fn why_exposure_subject_shows_incoming_adoption_edges() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let source_knowledge = engine
        .create_knowledge(
            KnowledgeCreateOptions::new(
                workspace.initial_branch_id,
                workspace.genesis_commit_id,
                "Reusable source knowledge",
            )
            .expect("knowledge options"),
        )
        .expect("create source knowledge");
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
    let adoption = engine
        .adopt_knowledge_exposure(
            KnowledgeExposureAdoptOptions::new(
                workspace.initial_branch_id,
                source_knowledge.commit_id,
                exposure.exposure_id,
                "adopt current exposure",
            )
            .expect("adoption options"),
        )
        .expect("adopt exposure");

    let why = engine
        .why(WhyQueryOptions::for_knowledge_exposure(
            WhyQueryTarget::commit(adoption.relation.commit_id),
            exposure.exposure_id,
        ))
        .expect("why exposure");

    let edge = why
        .relation_edges
        .iter()
        .find(|edge| edge.relation_id == adoption.relation.relation_id)
        .expect("incoming adoption edge");
    assert_eq!(
        edge.relation_kind,
        WhyRelationKind::KnowledgeExposureDerivedFrom
    );
    assert_eq!(edge.direction, WhyRelationDirection::Incoming);
    assert_eq!(
        edge.source,
        WhyRelationEndpoint::entity(
            adoption.knowledge.knowledge_entity_id,
            workvcs_core::WhyEntityKind::Knowledge
        )
    );
    assert_eq!(
        edge.target,
        WhyRelationEndpoint::knowledge_exposure(exposure.exposure_id)
    );
}
