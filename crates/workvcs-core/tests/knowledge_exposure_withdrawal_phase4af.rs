use std::path::{Path, PathBuf};
use tempfile::TempDir;
use workvcs_core::{
    CanonicalValue, Engine, ErrorCode, ExposureTransitionId, KnowledgeCreateOptions,
    KnowledgeExposureCreateLocalOptions, KnowledgeExposureLifecycleStatus,
    KnowledgeExposureListOptions, KnowledgeExposureSourceStatus, KnowledgeExposureWithdrawOptions,
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
        StoreInitOptions::new("phase4af-knowledge-exposure-store").expect("store options"),
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
fn withdraw_knowledge_exposure_updates_current_without_deleting_history() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let knowledge = engine
        .create_knowledge(
            KnowledgeCreateOptions::new(
                workspace.initial_branch_id,
                workspace.genesis_commit_id,
                "Reusable claim for withdrawal",
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
    let detail = object(vec![(
        "reason",
        CanonicalValue::String("retire-publication".to_owned()),
    )]);

    let withdrawn = engine
        .withdraw_knowledge_exposure(
            KnowledgeExposureWithdrawOptions::new(exposure.exposure_id, exposure.transition_id)
                .expect("withdraw options")
                .with_detail(detail.clone())
                .expect("withdraw detail"),
        )
        .expect("withdraw exposure")
        .exposure;

    assert_eq!(withdrawn.exposure_id, exposure.exposure_id);
    assert_eq!(
        withdrawn.lifecycle_status,
        KnowledgeExposureLifecycleStatus::Withdrawn
    );
    assert_ne!(withdrawn.transition_id, exposure.transition_id);
    assert_eq!(
        withdrawn.previous_transition_id,
        Some(exposure.transition_id)
    );
    assert_eq!(withdrawn.transition_detail, detail);
    assert_eq!(withdrawn.source, exposure.source);
    assert_eq!(
        withdrawn.source_status.source_status,
        KnowledgeExposureSourceStatus::Current
    );

    let shown = engine
        .knowledge_exposure(exposure.exposure_id)
        .expect("show withdrawn exposure");
    assert_eq!(shown, withdrawn);

    let active = engine
        .knowledge_exposures(
            KnowledgeExposureListOptions::new()
                .with_knowledge_space_id(knowledge_space.knowledge_space_id)
                .with_lifecycle_status(KnowledgeExposureLifecycleStatus::Active),
        )
        .expect("list active exposures");
    assert!(active.exposures.is_empty());

    let withdrawn_list = engine
        .knowledge_exposures(
            KnowledgeExposureListOptions::new()
                .with_knowledge_space_id(knowledge_space.knowledge_space_id)
                .with_lifecycle_status(KnowledgeExposureLifecycleStatus::Withdrawn),
        )
        .expect("list withdrawn exposures");
    assert_eq!(withdrawn_list.exposures, vec![withdrawn]);
}

#[test]
fn withdraw_knowledge_exposure_rejects_stale_transition_and_second_withdrawal() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let knowledge = engine
        .create_knowledge(
            KnowledgeCreateOptions::new(
                workspace.initial_branch_id,
                workspace.genesis_commit_id,
                "Reusable claim for transition checks",
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

    let stale = engine.withdraw_knowledge_exposure(
        KnowledgeExposureWithdrawOptions::new(exposure.exposure_id, ExposureTransitionId::new_v7())
            .expect("stale withdraw options"),
    );
    assert_eq!(
        stale.expect_err("stale transition must fail").code(),
        ErrorCode::QueryInvalid
    );

    let withdrawn = engine
        .withdraw_knowledge_exposure(
            KnowledgeExposureWithdrawOptions::new(exposure.exposure_id, exposure.transition_id)
                .expect("withdraw options"),
        )
        .expect("withdraw exposure")
        .exposure;
    let second = engine.withdraw_knowledge_exposure(
        KnowledgeExposureWithdrawOptions::new(exposure.exposure_id, withdrawn.transition_id)
            .expect("second withdraw options"),
    );
    assert_eq!(
        second.expect_err("second withdrawal must fail").code(),
        ErrorCode::QueryInvalid
    );
}

#[test]
fn withdraw_knowledge_exposure_requires_object_detail() {
    let options = KnowledgeExposureWithdrawOptions::new(
        workvcs_core::ExposureId::new_v7(),
        ExposureTransitionId::new_v7(),
    )
    .expect("withdraw options")
    .with_detail(CanonicalValue::String("not-object".to_owned()));

    assert_eq!(
        options.expect_err("non-object detail must fail").code(),
        ErrorCode::QueryInvalid
    );
}
