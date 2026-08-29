use std::path::{Path, PathBuf};
use tempfile::TempDir;
use workvcs_core::{
    CanonicalValue, Engine, KnowledgeCreateOptions, KnowledgeExposureCreateLocalOptions,
    KnowledgeExposureRefreshSourceStatusOptions, KnowledgeExposureSourceStatus,
    KnowledgeSpaceCreateOptions, KnowledgeTransitionOptions, StoreInitOptions, WorkspaceInfo,
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
        StoreInitOptions::new("phase4ag-knowledge-exposure-store").expect("store options"),
    )
    .expect("init engine");
    let workspace = engine
        .create_workspace(WorkspaceInitOptions::new("workspace").expect("workspace options"))
        .expect("create workspace");
    (engine, workspace)
}

fn detail_integer(detail: &CanonicalValue, key: &str) -> i64 {
    let CanonicalValue::Object(entries) = detail else {
        panic!("detail is not an object");
    };
    let (_, value) = entries
        .iter()
        .find(|(entry_key, _)| entry_key == key)
        .unwrap_or_else(|| panic!("missing detail key {key}"));
    let CanonicalValue::Integer(value) = value else {
        panic!("detail key {key} is not an integer");
    };
    value.get()
}

#[test]
fn refresh_source_status_keeps_current_when_source_version_is_at_branch_head() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let knowledge = engine
        .create_knowledge(
            KnowledgeCreateOptions::new(
                workspace.initial_branch_id,
                workspace.genesis_commit_id,
                "Reusable claim for source status",
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

    let refreshed = engine
        .refresh_knowledge_exposure_source_status(KnowledgeExposureRefreshSourceStatusOptions::new(
            exposure.exposure_id,
        ))
        .expect("refresh source status")
        .exposure;

    assert_eq!(refreshed.exposure_id, exposure.exposure_id);
    assert_eq!(
        refreshed.source_status.source_status,
        KnowledgeExposureSourceStatus::Current
    );
    assert_eq!(
        detail_integer(&refreshed.source_status.detail, "checked_branch_heads"),
        1
    );
    assert_eq!(
        detail_integer(&refreshed.source_status.detail, "matching_branch_heads"),
        1
    );
    assert_eq!(
        detail_integer(&refreshed.source_status.detail, "drifted_branch_heads"),
        0
    );
    assert_eq!(
        detail_integer(&refreshed.source_status.detail, "missing_branch_heads"),
        0
    );
}

#[test]
fn refresh_source_status_marks_stale_when_branch_head_has_new_source_version() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let knowledge = engine
        .create_knowledge(
            KnowledgeCreateOptions::new(
                workspace.initial_branch_id,
                workspace.genesis_commit_id,
                "Reusable claim that later changes",
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

    let invalidated = engine
        .transition_knowledge(
            KnowledgeTransitionOptions::invalidate(
                workspace.initial_branch_id,
                knowledge.commit_id,
                knowledge.knowledge_entity_id,
                knowledge.knowledge_entity_version_id,
                "source claim changed",
            )
            .expect("transition options"),
        )
        .expect("invalidate knowledge");
    assert_ne!(
        invalidated.knowledge_entity_version_id,
        exposure.source.knowledge_entity_version_id
    );

    let refreshed = engine
        .refresh_knowledge_exposure_source_status(KnowledgeExposureRefreshSourceStatusOptions::new(
            exposure.exposure_id,
        ))
        .expect("refresh source status")
        .exposure;

    assert_eq!(
        refreshed.source.knowledge_entity_version_id,
        exposure.source.knowledge_entity_version_id
    );
    assert_eq!(
        refreshed.source_status.source_status,
        KnowledgeExposureSourceStatus::Stale
    );
    assert_eq!(
        detail_integer(&refreshed.source_status.detail, "checked_branch_heads"),
        1
    );
    assert_eq!(
        detail_integer(&refreshed.source_status.detail, "matching_branch_heads"),
        0
    );
    assert_eq!(
        detail_integer(&refreshed.source_status.detail, "drifted_branch_heads"),
        1
    );
    assert_eq!(
        detail_integer(&refreshed.source_status.detail, "missing_branch_heads"),
        0
    );
}
