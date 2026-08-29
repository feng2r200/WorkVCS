use std::path::{Path, PathBuf};
use tempfile::TempDir;
use workvcs_core::{
    CommitId, Engine, KnowledgeCreateCommit, KnowledgeCreateOptions,
    KnowledgeExposureCreateLocalOptions, KnowledgeExposureSourceStatus,
    KnowledgeExposureWithdrawOptions, KnowledgeSpaceCreateOptions,
    KnowledgeSpaceRefreshSourceStatusesOptions, KnowledgeTransitionOptions, StoreInitOptions,
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
        StoreInitOptions::new("phase4ak-source-status-refresh-store").expect("store options"),
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
fn refreshes_active_knowledge_space_exposure_source_statuses() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let current_knowledge = create_knowledge(
        &mut engine,
        &workspace,
        workspace.genesis_commit_id,
        "Current reusable knowledge",
    );
    let stale_knowledge = create_knowledge(
        &mut engine,
        &workspace,
        current_knowledge.commit_id,
        "Stale reusable knowledge",
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

    let current_exposure = engine
        .create_local_knowledge_exposure(
            KnowledgeExposureCreateLocalOptions::new(
                knowledge_space.knowledge_space_id,
                workspace.workspace_id,
                current_knowledge.knowledge_entity_id,
                current_knowledge.knowledge_entity_version_id,
            )
            .expect("current exposure options"),
        )
        .expect("create current exposure")
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
            .expect("invalidate stale options"),
        )
        .expect("invalidate stale source");
    engine
        .withdraw_knowledge_exposure(
            KnowledgeExposureWithdrawOptions::new(
                withdrawn_exposure.exposure_id,
                withdrawn_exposure.transition_id,
            )
            .expect("withdraw options"),
        )
        .expect("withdraw exposure");

    let refreshed = engine
        .refresh_knowledge_space_source_statuses(KnowledgeSpaceRefreshSourceStatusesOptions::new(
            knowledge_space.knowledge_space_id,
        ))
        .expect("refresh knowledge space source statuses");

    assert_eq!(
        refreshed.knowledge_space_id,
        knowledge_space.knowledge_space_id
    );
    assert_eq!(refreshed.refreshed_exposures.len(), 2);
    assert_eq!(refreshed.current_count, 1);
    assert_eq!(refreshed.stale_count, 1);
    assert_eq!(refreshed.unknown_count, 0);
    assert_eq!(refreshed.unresolved_count, 0);

    let current = refreshed
        .refreshed_exposures
        .iter()
        .find(|exposure| exposure.exposure_id == current_exposure.exposure_id)
        .expect("current exposure refreshed");
    assert_eq!(
        current.source_status.source_status,
        KnowledgeExposureSourceStatus::Current
    );

    let stale = refreshed
        .refreshed_exposures
        .iter()
        .find(|exposure| exposure.exposure_id == stale_exposure.exposure_id)
        .expect("stale exposure refreshed");
    assert_eq!(
        stale.source_status.source_status,
        KnowledgeExposureSourceStatus::Stale
    );
    assert!(
        refreshed
            .refreshed_exposures
            .iter()
            .all(|exposure| exposure.exposure_id != withdrawn_exposure.exposure_id)
    );
}
