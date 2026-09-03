use std::path::{Path, PathBuf};
use tempfile::TempDir;
use workvcs_core::{
    ChangeOperationSubject, Engine, KnowledgeCreateOptions, KnowledgeExposureAdoptOptions,
    KnowledgeExposureAdoptResult, KnowledgeExposureCreateLocalOptions, KnowledgeExposureSnapshot,
    KnowledgeSpaceCreateOptions, StoreInitOptions, WhyDeferredRelationFamily, WhyEntityKind,
    WhyEvolutionSubjectDetail, WhyQueryOptions, WhyQueryResult, WhyQueryTarget,
    WhyRelationDirection, WhyRelationEndpoint, WhyRelationKind, WorkspaceInfo,
    WorkspaceInitOptions,
};

fn store_path() -> (TempDir, PathBuf) {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let path = tempdir.path().join("workvcs.sqlite");
    (tempdir, path)
}

fn create_workspace(path: &Path) -> (Engine, WorkspaceInfo) {
    let mut engine = Engine::init(
        path,
        StoreInitOptions::new("phase4ao-why-knowledge-exposure-provenance-store")
            .expect("store options"),
    )
    .expect("init engine");
    let workspace = engine
        .create_workspace(WorkspaceInitOptions::new("workspace").expect("workspace options"))
        .expect("create workspace");
    (engine, workspace)
}

fn assert_adoption_provenance_evolution(
    why: &WhyQueryResult,
    adoption: &KnowledgeExposureAdoptResult,
    exposure: &KnowledgeExposureSnapshot,
) {
    assert_eq!(why.evolution_change_operations.len(), 1);
    assert_eq!(
        why.deferred_relation_families,
        vec![WhyDeferredRelationFamily::Evolution]
    );

    let operation = &why.evolution_change_operations[0];
    assert_eq!(operation.commit_id, adoption.relation.commit_id);
    assert_eq!(operation.changeset_id, adoption.relation.changeset_id);
    assert_eq!(
        operation.changeset_operation_type,
        "knowledge.relation.create"
    );
    assert_eq!(operation.changeset_operation_schema_version, 1);
    assert_eq!(operation.ordinal, 0);
    assert_eq!(operation.operation_id, adoption.relation.operation_id);
    assert_eq!(
        operation.subject,
        ChangeOperationSubject::Relation(adoption.relation.relation_id)
    );

    let Some(WhyEvolutionSubjectDetail::Relation(detail)) = &operation.subject_detail else {
        panic!("expected knowledge exposure derived_from relation subject detail")
    };
    assert_eq!(
        detail.relation_kind,
        WhyRelationKind::KnowledgeExposureDerivedFrom
    );
    assert_eq!(
        detail.relation_version_id,
        adoption.relation.relation_version_id
    );
    assert_eq!(detail.relation_label, None);
    assert_eq!(
        detail.source,
        WhyRelationEndpoint::entity(
            adoption.knowledge.knowledge_entity_id,
            WhyEntityKind::Knowledge
        )
    );
    assert_eq!(
        detail.target,
        WhyRelationEndpoint::knowledge_exposure(exposure.exposure_id)
    );
    assert_eq!(detail.state_digest, adoption.relation.relation_state_digest);
}

#[test]
fn why_shows_adopted_knowledge_exposure_provenance_edge() {
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
        .why(WhyQueryOptions::for_entity(
            WhyQueryTarget::commit(adoption.relation.commit_id),
            adoption.knowledge.knowledge_entity_id,
        ))
        .expect("why adopted knowledge");

    let edge = why
        .relation_edges
        .iter()
        .find(|edge| edge.relation_id == adoption.relation.relation_id)
        .expect("adoption provenance edge");
    assert_eq!(
        edge.relation_kind,
        WhyRelationKind::KnowledgeExposureDerivedFrom
    );
    assert_eq!(edge.direction, WhyRelationDirection::Outgoing);
    assert_eq!(
        edge.source,
        WhyRelationEndpoint::entity(
            adoption.knowledge.knowledge_entity_id,
            WhyEntityKind::Knowledge
        )
    );
    assert_eq!(
        edge.target,
        WhyRelationEndpoint::knowledge_exposure(exposure.exposure_id)
    );
    assert_eq!(
        edge.relation_version_id,
        adoption.relation.relation_version_id
    );
    assert_eq!(edge.state_digest, adoption.relation.relation_state_digest);
    assert_adoption_provenance_evolution(&why, &adoption, &exposure);

    let exposure_why = engine
        .why(WhyQueryOptions::for_knowledge_exposure(
            WhyQueryTarget::commit(adoption.relation.commit_id),
            exposure.exposure_id,
        ))
        .expect("why exposure");
    let exposure_edge = exposure_why
        .relation_edges
        .iter()
        .find(|edge| edge.relation_id == adoption.relation.relation_id)
        .expect("exposure provenance edge");
    assert_eq!(
        exposure_edge.relation_kind,
        WhyRelationKind::KnowledgeExposureDerivedFrom
    );
    assert_eq!(exposure_edge.direction, WhyRelationDirection::Incoming);
    assert_adoption_provenance_evolution(&exposure_why, &adoption, &exposure);
}
