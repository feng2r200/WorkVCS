use std::path::{Path, PathBuf};
use tempfile::TempDir;
use workvcs_core::{
    CommitId, Engine, KnowledgeCreateCommit, KnowledgeCreateOptions,
    KnowledgeExposureCreateLocalOptions, KnowledgeExposureRefreshSourceStatusOptions,
    KnowledgeExposureWithdrawOptions, KnowledgeSpaceCreateOptions,
    KnowledgeSpaceHistoricalExposuresOptions, KnowledgeTransitionOptions, StoreInitOptions,
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
        StoreInitOptions::new("phase4aj-historical-exposure-store").expect("store options"),
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
fn historical_exposures_include_only_withdrawn_exposures() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let active_knowledge = create_knowledge(
        &mut engine,
        &workspace,
        workspace.genesis_commit_id,
        "Active current reusable knowledge",
    );
    let stale_knowledge = create_knowledge(
        &mut engine,
        &workspace,
        active_knowledge.commit_id,
        "Active stale reusable knowledge",
    );
    let withdrawn_knowledge = create_knowledge(
        &mut engine,
        &workspace,
        stale_knowledge.commit_id,
        "Withdrawn reusable knowledge",
    );
    let knowledge_space = engine
        .create_knowledge_space(KnowledgeSpaceCreateOptions::new("Research").expect("space name"))
        .expect("create knowledge space")
        .knowledge_space;

    let active_exposure = engine
        .create_local_knowledge_exposure(
            KnowledgeExposureCreateLocalOptions::new(
                knowledge_space.knowledge_space_id,
                workspace.workspace_id,
                active_knowledge.knowledge_entity_id,
                active_knowledge.knowledge_entity_version_id,
            )
            .expect("active exposure options"),
        )
        .expect("create active exposure")
        .exposure;
    let stale_exposure = engine
        .create_local_knowledge_exposure(
            KnowledgeExposureCreateLocalOptions::new(
                knowledge_space.knowledge_space_id,
                workspace.workspace_id,
                stale_knowledge.knowledge_entity_id,
                stale_knowledge.knowledge_entity_version_id,
            )
            .expect("stale exposure options"),
        )
        .expect("create stale exposure")
        .exposure;
    let withdrawn_exposure = engine
        .create_local_knowledge_exposure(
            KnowledgeExposureCreateLocalOptions::new(
                knowledge_space.knowledge_space_id,
                workspace.workspace_id,
                withdrawn_knowledge.knowledge_entity_id,
                withdrawn_knowledge.knowledge_entity_version_id,
            )
            .expect("withdrawn exposure options"),
        )
        .expect("create withdrawn exposure")
        .exposure;

    engine
        .transition_knowledge(
            KnowledgeTransitionOptions::invalidate(
                workspace.initial_branch_id,
                withdrawn_knowledge.commit_id,
                stale_knowledge.knowledge_entity_id,
                stale_knowledge.knowledge_entity_version_id,
                "source drift",
            )
            .expect("invalidate options"),
        )
        .expect("invalidate stale source");
    engine
        .refresh_knowledge_exposure_source_status(KnowledgeExposureRefreshSourceStatusOptions::new(
            stale_exposure.exposure_id,
        ))
        .expect("refresh stale source");
    let withdrawn = engine
        .withdraw_knowledge_exposure(
            KnowledgeExposureWithdrawOptions::new(
                withdrawn_exposure.exposure_id,
                withdrawn_exposure.transition_id,
            )
            .expect("withdraw options"),
        )
        .expect("withdraw exposure")
        .exposure;

    let historical = engine
        .knowledge_space_historical_exposures(KnowledgeSpaceHistoricalExposuresOptions::new(
            knowledge_space.knowledge_space_id,
        ))
        .expect("historical exposures");

    assert_eq!(
        historical.knowledge_space_id,
        knowledge_space.knowledge_space_id
    );
    assert_eq!(historical.exposures.len(), 1);
    assert_eq!(historical.exposures[0], withdrawn);
    assert_ne!(
        historical.exposures[0].exposure_id,
        active_exposure.exposure_id
    );
    assert_ne!(
        historical.exposures[0].exposure_id,
        stale_exposure.exposure_id
    );
    assert_eq!(
        historical.exposures[0].lifecycle_status.as_str(),
        "withdrawn"
    );
}
