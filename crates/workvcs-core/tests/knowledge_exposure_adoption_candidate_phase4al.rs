use std::path::{Path, PathBuf};
use tempfile::TempDir;
use workvcs_core::{
    CanonicalValue, Engine, KnowledgeCreateOptions, KnowledgeExposureAdoptionCandidateOptions,
    KnowledgeExposureCreateLocalOptions, KnowledgeExposureWithdrawOptions,
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
        StoreInitOptions::new("phase4al-adoption-candidate-store").expect("store options"),
    )
    .expect("init engine");
    let workspace = engine
        .create_workspace(WorkspaceInitOptions::new("workspace").expect("workspace options"))
        .expect("create workspace");
    (engine, workspace)
}

fn object_string(value: &CanonicalValue, key: &str) -> String {
    let CanonicalValue::Object(entries) = value else {
        panic!("expected object, found {value:?}");
    };
    let Some((_, CanonicalValue::String(value))) =
        entries.iter().find(|(entry_key, _)| entry_key == key)
    else {
        panic!("missing string field {key} in {value:?}");
    };
    value.clone()
}

#[test]
fn adoption_candidate_preserves_source_version_state_and_provenance() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let store_id = engine.store_info().expect("store info").store_id;
    let scope = CanonicalValue::object(vec![(
        "kind".to_owned(),
        CanonicalValue::String("workspace".to_owned()),
    )])
    .expect("scope");
    let provenance = CanonicalValue::object(vec![(
        "source".to_owned(),
        CanonicalValue::String("test-fixture".to_owned()),
    )])
    .expect("provenance");
    let knowledge = engine
        .create_knowledge(
            KnowledgeCreateOptions::new(
                workspace.initial_branch_id,
                workspace.genesis_commit_id,
                "Reusable adoption candidate knowledge",
            )
            .expect("knowledge options")
            .with_scope(scope.clone())
            .expect("knowledge scope")
            .with_provenance(provenance.clone())
            .expect("knowledge provenance"),
        )
        .expect("create knowledge");
    let knowledge_space = engine
        .create_knowledge_space(KnowledgeSpaceCreateOptions::new("Research").expect("space name"))
        .expect("create knowledge space")
        .knowledge_space;
    let exposure = engine
        .create_local_knowledge_exposure(
            KnowledgeExposureCreateLocalOptions::new(
                knowledge_space.knowledge_space_id,
                workspace.workspace_id,
                knowledge.knowledge_entity_id,
                knowledge.knowledge_entity_version_id,
            )
            .expect("exposure options"),
        )
        .expect("create exposure")
        .exposure;

    let candidate = engine
        .knowledge_exposure_adoption_candidate(KnowledgeExposureAdoptionCandidateOptions::new(
            exposure.exposure_id,
        ))
        .expect("adoption candidate")
        .candidate;

    assert_eq!(candidate.exposure, exposure);
    assert_eq!(candidate.source_store_id, store_id);
    assert_eq!(
        candidate.source_knowledge_state_digest,
        knowledge.knowledge_state_digest
    );
    assert_eq!(
        candidate.source_knowledge_state.statement,
        "Reusable adoption candidate knowledge"
    );
    assert_eq!(candidate.source_knowledge_state.scope, scope);
    assert_eq!(candidate.source_knowledge_state.provenance, provenance);
    assert_eq!(
        object_string(&candidate.adoption_provenance, "adopted_from_exposure_id"),
        exposure.exposure_id.to_string()
    );
    assert_eq!(
        object_string(&candidate.adoption_provenance, "source_store_id"),
        store_id.to_string()
    );
    assert_eq!(
        object_string(
            &candidate.adoption_provenance,
            "source_knowledge_entity_version_id"
        ),
        knowledge.knowledge_entity_version_id.to_string()
    );
    assert_eq!(
        object_string(
            &candidate.adoption_provenance,
            "source_knowledge_state_digest"
        ),
        knowledge.knowledge_state_digest.to_string()
    );
}

#[test]
fn adoption_candidate_rejects_withdrawn_exposure() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let knowledge = engine
        .create_knowledge(
            KnowledgeCreateOptions::new(
                workspace.initial_branch_id,
                workspace.genesis_commit_id,
                "Withdrawn exposure knowledge",
            )
            .expect("knowledge options"),
        )
        .expect("create knowledge");
    let knowledge_space = engine
        .create_knowledge_space(KnowledgeSpaceCreateOptions::new("Research").expect("space name"))
        .expect("create knowledge space")
        .knowledge_space;
    let exposure = engine
        .create_local_knowledge_exposure(
            KnowledgeExposureCreateLocalOptions::new(
                knowledge_space.knowledge_space_id,
                workspace.workspace_id,
                knowledge.knowledge_entity_id,
                knowledge.knowledge_entity_version_id,
            )
            .expect("exposure options"),
        )
        .expect("create exposure")
        .exposure;
    engine
        .withdraw_knowledge_exposure(
            KnowledgeExposureWithdrawOptions::new(exposure.exposure_id, exposure.transition_id)
                .expect("withdraw options"),
        )
        .expect("withdraw exposure");

    let error = engine
        .knowledge_exposure_adoption_candidate(KnowledgeExposureAdoptionCandidateOptions::new(
            exposure.exposure_id,
        ))
        .expect_err("withdrawn exposure cannot be an adoption candidate");

    assert!(error.to_string().contains("not active"));
}
