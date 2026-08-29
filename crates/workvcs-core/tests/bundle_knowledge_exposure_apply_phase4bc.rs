use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;
use workvcs_core::{
    BundleImportApplyOptions, BundleImportPreflightOptions, BundlePayloadExport,
    BundlePayloadExportOptions, BundlePayloadInput, CanonicalValue, CommitId, Engine,
    KnowledgeCreateCommit, KnowledgeCreateOptions, KnowledgeExposureAdoptOptions,
    KnowledgeExposureCreateLocalOptions, KnowledgeExposureLifecycleStatus,
    KnowledgeExposureSourceStatus, KnowledgeExposureWithdrawOptions, KnowledgeSpaceCreateOptions,
    StoreInitOptions, WorkspaceInfo, WorkspaceInitOptions,
};

fn store_paths() -> (TempDir, PathBuf, PathBuf) {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let source_path = tempdir.path().join("source.sqlite");
    let old_path = tempdir.path().join("old.sqlite");
    (tempdir, source_path, old_path)
}

fn create_workspace(path: &Path) -> (Engine, WorkspaceInfo) {
    let mut engine = Engine::init(
        path,
        StoreInitOptions::new("phase4bc-bundle-knowledge-exposure-apply-store")
            .expect("store options"),
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

fn string(value: &str) -> CanonicalValue {
    CanonicalValue::String(value.to_owned())
}

fn create_source_knowledge(
    engine: &mut Engine,
    workspace: &WorkspaceInfo,
    head: CommitId,
) -> KnowledgeCreateCommit {
    engine
        .create_knowledge(
            KnowledgeCreateOptions::new(
                workspace.initial_branch_id,
                head,
                "Reusable source knowledge for Bundle apply",
            )
            .expect("knowledge options")
            .with_scope(object(vec![("domain", string("bundle-apply"))]))
            .expect("knowledge scope")
            .with_provenance(object(vec![("source", string("phase4bc"))]))
            .expect("knowledge provenance"),
        )
        .expect("create source knowledge")
}

fn payload_inputs(export: &BundlePayloadExport) -> Vec<BundlePayloadInput> {
    export
        .payload_files
        .iter()
        .map(|payload| {
            BundlePayloadInput::new(payload.relative_path.clone(), payload.bytes.clone())
                .expect("payload input")
        })
        .collect()
}

fn preflight_options(export: &BundlePayloadExport) -> BundleImportPreflightOptions {
    BundleImportPreflightOptions::from_parts(
        export.manifest_bytes.clone(),
        export.payload_index_bytes.clone(),
        payload_inputs(export),
    )
    .expect("preflight options")
}

fn apply_options(export: &BundlePayloadExport) -> BundleImportApplyOptions {
    BundleImportApplyOptions::from_parts(
        export.manifest_bytes.clone(),
        export.payload_index_bytes.clone(),
        payload_inputs(export),
    )
    .expect("apply options")
}

#[test]
fn bundle_apply_restores_knowledge_exposure_closure_and_current_projection() {
    let (_tempdir, source_path, old_path) = store_paths();
    let (engine, workspace) = create_workspace(&source_path);
    drop(engine);
    fs::copy(&source_path, &old_path).expect("copy old store");

    let mut source_engine = Engine::open(&source_path).expect("open source store");
    let source_knowledge =
        create_source_knowledge(&mut source_engine, &workspace, workspace.genesis_commit_id);
    let knowledge_space = source_engine
        .create_knowledge_space(KnowledgeSpaceCreateOptions::new("Research").expect("space name"))
        .expect("create knowledge space")
        .knowledge_space;
    let created_exposure = source_engine
        .create_local_knowledge_exposure(
            KnowledgeExposureCreateLocalOptions::new(
                knowledge_space.knowledge_space_id,
                workspace.workspace_id,
                source_knowledge.knowledge_entity_id,
                source_knowledge.knowledge_entity_version_id,
            )
            .expect("exposure options")
            .with_detail(object(vec![("reason", string("portable reuse"))]))
            .expect("exposure detail"),
        )
        .expect("create exposure")
        .exposure;
    let adoption = source_engine
        .adopt_knowledge_exposure(
            KnowledgeExposureAdoptOptions::new(
                workspace.initial_branch_id,
                source_knowledge.commit_id,
                created_exposure.exposure_id,
                "adopt current exposure",
            )
            .expect("adoption options"),
        )
        .expect("adopt exposure");
    let withdrawn = source_engine
        .withdraw_knowledge_exposure(
            KnowledgeExposureWithdrawOptions::new(
                created_exposure.exposure_id,
                created_exposure.transition_id,
            )
            .expect("withdraw options")
            .with_detail(object(vec![("reason", string("retired"))]))
            .expect("withdraw detail"),
        )
        .expect("withdraw exposure")
        .exposure;

    let export = source_engine
        .export_bundle_payloads(BundlePayloadExportOptions::for_commit(
            adoption.relation.commit_id,
        ))
        .expect("export payloads");
    assert_eq!(export.manifest.knowledge_spaces.len(), 1);
    assert_eq!(export.manifest.knowledge_exposures.len(), 1);
    assert_eq!(export.manifest.knowledge_exposure_local_sources.len(), 1);
    assert_eq!(export.manifest.knowledge_exposure_transitions.len(), 2);
    assert_eq!(export.manifest.knowledge_exposure_source_statuses.len(), 1);

    let mut old_engine = Engine::open(&old_path).expect("open old store");
    let before = old_engine
        .preflight_bundle_import(preflight_options(&export))
        .expect("preflight before apply");
    assert_eq!(before.action, "same_store_fast_forward_ready");
    assert!(before.can_apply);

    let applied = old_engine
        .apply_bundle_import(apply_options(&export))
        .expect("apply bundle import");

    assert!(applied.applied);
    assert_eq!(applied.imported_knowledge_spaces, 1);
    assert_eq!(applied.imported_knowledge_exposures, 1);
    assert_eq!(applied.imported_knowledge_exposure_local_sources, 1);
    assert_eq!(applied.imported_knowledge_exposure_transitions, 2);
    assert_eq!(applied.imported_knowledge_exposure_source_statuses, 1);
    assert_eq!(applied.imported_relation_versions, 1);
    assert_eq!(applied.updated_branch_heads, 1);

    let imported_exposure = old_engine
        .knowledge_exposure(created_exposure.exposure_id)
        .expect("imported exposure");
    assert_eq!(imported_exposure, withdrawn);
    assert_eq!(
        imported_exposure.lifecycle_status,
        KnowledgeExposureLifecycleStatus::Withdrawn
    );
    assert_eq!(
        imported_exposure.source_status.source_status,
        KnowledgeExposureSourceStatus::Current
    );

    let state = old_engine
        .show_at(adoption.relation.commit_id)
        .expect("replay imported adoption relation commit");
    assert!(state.state.relations().contains(&(
        adoption.relation.relation_id,
        adoption.relation.relation_version_id
    )));
    assert_eq!(state.state_digest, adoption.relation.work_state_digest);
}
