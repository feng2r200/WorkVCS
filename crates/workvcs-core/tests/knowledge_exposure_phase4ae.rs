use std::path::{Path, PathBuf};
use tempfile::TempDir;
use workvcs_core::{
    CanonicalValue, Engine, EntityVersionId, ErrorCode, KnowledgeCreateCommit,
    KnowledgeCreateOptions, KnowledgeExposureCreateLocalOptions, KnowledgeExposureLifecycleStatus,
    KnowledgeExposureListOptions, KnowledgeExposureSourceStatus, KnowledgeSpaceCreateOptions,
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
        StoreInitOptions::new("phase4ae-knowledge-exposure-store").expect("store options"),
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

fn create_knowledge(engine: &mut Engine, workspace: &WorkspaceInfo) -> KnowledgeCreateCommit {
    engine
        .create_knowledge(
            KnowledgeCreateOptions::new(
                workspace.initial_branch_id,
                workspace.genesis_commit_id,
                "Reusable claim for the knowledge space",
            )
            .expect("knowledge options"),
        )
        .expect("create knowledge")
}

#[test]
fn local_knowledge_exposure_binds_exact_source_version_and_projects_current() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let knowledge = create_knowledge(&mut engine, &workspace);
    let knowledge_space = engine
        .create_knowledge_space(KnowledgeSpaceCreateOptions::new("Research").expect("space name"))
        .expect("create knowledge space")
        .knowledge_space;
    let detail = object(vec![(
        "reason",
        CanonicalValue::String("publish-for-reuse".to_owned()),
    )]);

    let created = engine
        .create_local_knowledge_exposure(
            KnowledgeExposureCreateLocalOptions::new(
                knowledge_space.knowledge_space_id,
                workspace.workspace_id,
                knowledge.knowledge_entity_id,
                knowledge.knowledge_entity_version_id,
            )
            .expect("exposure options")
            .with_detail(detail.clone())
            .expect("detail"),
        )
        .expect("create exposure");
    let exposure = created.exposure;

    assert_eq!(
        exposure.knowledge_space_id,
        knowledge_space.knowledge_space_id
    );
    assert_eq!(
        exposure.lifecycle_status,
        KnowledgeExposureLifecycleStatus::Active
    );
    assert!(exposure.previous_transition_id.is_none());
    assert_eq!(exposure.transition_detail, detail);
    assert_eq!(exposure.source.workspace_id, workspace.workspace_id);
    assert_eq!(
        exposure.source.knowledge_entity_id,
        knowledge.knowledge_entity_id
    );
    assert_eq!(
        exposure.source.knowledge_entity_version_id,
        knowledge.knowledge_entity_version_id
    );
    assert_eq!(
        exposure.source.knowledge_state_digest,
        knowledge.knowledge_state_digest
    );
    assert_eq!(
        exposure.source_status.source_status,
        KnowledgeExposureSourceStatus::Current
    );

    let shown = engine
        .knowledge_exposure(exposure.exposure_id)
        .expect("show exposure");
    assert_eq!(shown, exposure);

    let listed = engine
        .knowledge_exposures(
            KnowledgeExposureListOptions::new()
                .with_knowledge_space_id(knowledge_space.knowledge_space_id)
                .with_workspace_id(workspace.workspace_id)
                .with_knowledge_entity_id(knowledge.knowledge_entity_id)
                .with_lifecycle_status(KnowledgeExposureLifecycleStatus::Active)
                .with_source_status(KnowledgeExposureSourceStatus::Current),
        )
        .expect("list exposures");
    assert_eq!(listed.exposures, vec![exposure]);
}

#[test]
fn local_knowledge_exposure_rejects_duplicate_and_invalid_inputs() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let knowledge = create_knowledge(&mut engine, &workspace);
    let knowledge_space = engine
        .create_knowledge_space(KnowledgeSpaceCreateOptions::new("Research").expect("space name"))
        .expect("create knowledge space")
        .knowledge_space;
    let options = KnowledgeExposureCreateLocalOptions::new(
        knowledge_space.knowledge_space_id,
        workspace.workspace_id,
        knowledge.knowledge_entity_id,
        knowledge.knowledge_entity_version_id,
    )
    .expect("exposure options");
    engine
        .create_local_knowledge_exposure(options.clone())
        .expect("create exposure");

    let duplicate = engine.create_local_knowledge_exposure(options);
    assert_eq!(
        duplicate.expect_err("duplicate must fail").code(),
        ErrorCode::QueryInvalid
    );

    let non_object_detail = KnowledgeExposureCreateLocalOptions::new(
        knowledge_space.knowledge_space_id,
        workspace.workspace_id,
        knowledge.knowledge_entity_id,
        knowledge.knowledge_entity_version_id,
    )
    .expect("exposure options")
    .with_detail(CanonicalValue::String("not-object".to_owned()));
    assert_eq!(
        non_object_detail
            .expect_err("non-object detail must fail")
            .code(),
        ErrorCode::QueryInvalid
    );

    let missing_version = engine.create_local_knowledge_exposure(
        KnowledgeExposureCreateLocalOptions::new(
            knowledge_space.knowledge_space_id,
            workspace.workspace_id,
            knowledge.knowledge_entity_id,
            EntityVersionId::new_v7(),
        )
        .expect("missing version options"),
    );
    assert_eq!(
        missing_version
            .expect_err("missing version must fail")
            .code(),
        ErrorCode::KnowledgeNotFound
    );
}

#[test]
fn local_knowledge_exposure_rejects_wrong_workspace_and_zero_limit() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let other_workspace = engine
        .create_workspace(WorkspaceInitOptions::new("other").expect("workspace options"))
        .expect("create other workspace");
    let knowledge = create_knowledge(&mut engine, &workspace);
    let knowledge_space = engine
        .create_knowledge_space(KnowledgeSpaceCreateOptions::new("Research").expect("space name"))
        .expect("create knowledge space")
        .knowledge_space;

    let wrong_workspace = engine.create_local_knowledge_exposure(
        KnowledgeExposureCreateLocalOptions::new(
            knowledge_space.knowledge_space_id,
            other_workspace.workspace_id,
            knowledge.knowledge_entity_id,
            knowledge.knowledge_entity_version_id,
        )
        .expect("wrong workspace options"),
    );
    assert_eq!(
        wrong_workspace
            .expect_err("wrong workspace must fail")
            .code(),
        ErrorCode::KnowledgeInvalid
    );

    let zero_limit = KnowledgeExposureListOptions::new()
        .with_limit(0)
        .expect_err("zero limit must fail");
    assert_eq!(zero_limit.code(), ErrorCode::QueryInvalid);
}
