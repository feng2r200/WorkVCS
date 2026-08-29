use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use tempfile::TempDir;
use workvcs_core::{
    BundleExportOptions, BundlePayloadExport, BundlePayloadExportOptions, BundlePayloadInput,
    BundlePayloadValidationOptions, CanonicalValue, CommitId, Engine, KnowledgeCreateCommit,
    KnowledgeCreateOptions, KnowledgeExposureAdoptOptions, KnowledgeExposureCreateLocalOptions,
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
        StoreInitOptions::new("phase4ar-bundle-knowledge-exposure-closure-store")
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
                "Reusable bundle source knowledge",
            )
            .expect("knowledge options")
            .with_scope(object(vec![(
                "domain",
                CanonicalValue::String("bundle".to_owned()),
            )]))
            .expect("knowledge scope")
            .with_provenance(object(vec![(
                "source",
                CanonicalValue::String("phase4ar".to_owned()),
            )]))
            .expect("knowledge provenance"),
        )
        .expect("create source knowledge")
}

fn object_field<'a>(value: &'a CanonicalValue, key: &str) -> &'a CanonicalValue {
    let CanonicalValue::Object(entries) = value else {
        panic!("expected canonical object");
    };
    entries
        .iter()
        .find_map(|(entry_key, entry_value)| (entry_key == key).then_some(entry_value))
        .unwrap_or_else(|| panic!("missing key {key}"))
}

fn array_len(value: &CanonicalValue, key: &str) -> usize {
    let CanonicalValue::Array(values) = object_field(value, key) else {
        panic!("{key} must be an array");
    };
    values.len()
}

fn role_counts(export: &BundlePayloadExport) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    for reference in &export.payload_references {
        *counts.entry(reference.role.clone()).or_insert(0) += 1;
    }
    counts
}

#[test]
fn bundle_manifest_and_payloads_include_knowledge_exposure_closure() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let source_knowledge =
        create_source_knowledge(&mut engine, &workspace, workspace.genesis_commit_id);
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
            .expect("exposure options")
            .with_detail(object(vec![(
                "reason",
                CanonicalValue::String("portable reuse".to_owned()),
            )]))
            .expect("exposure detail"),
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

    let manifest = engine
        .export_bundle_manifest(BundleExportOptions::for_commit(adoption.relation.commit_id))
        .expect("export bundle manifest");

    assert_eq!(manifest.knowledge_spaces.len(), 1);
    assert_eq!(
        manifest.knowledge_spaces[0].knowledge_space_id,
        knowledge_space.knowledge_space_id
    );
    assert_eq!(manifest.knowledge_spaces[0].name, "Research");
    assert_eq!(manifest.knowledge_exposures.len(), 1);
    assert_eq!(
        manifest.knowledge_exposures[0].exposure_id,
        exposure.exposure_id
    );
    assert_eq!(
        manifest.knowledge_exposures[0].knowledge_space_id,
        knowledge_space.knowledge_space_id
    );
    assert_eq!(manifest.knowledge_exposure_local_sources.len(), 1);
    let source = &manifest.knowledge_exposure_local_sources[0];
    assert_eq!(source.exposure_id, exposure.exposure_id);
    assert_eq!(source.source_workspace_id, workspace.workspace_id);
    assert_eq!(
        source.source_knowledge_entity_id,
        source_knowledge.knowledge_entity_id
    );
    assert_eq!(
        source.source_knowledge_entity_version_id,
        source_knowledge.knowledge_entity_version_id
    );
    assert_eq!(
        source.source_knowledge_state_digest,
        source_knowledge.knowledge_state_digest
    );
    assert!(source.source_knowledge_state_json_size_bytes > 0);
    assert_eq!(manifest.knowledge_exposure_transitions.len(), 1);
    assert_eq!(
        manifest.knowledge_exposure_transitions[0].transition_id,
        exposure.transition_id
    );
    assert_eq!(
        manifest.knowledge_exposure_transitions[0].lifecycle_status,
        "active"
    );
    assert_eq!(manifest.knowledge_exposure_source_statuses.len(), 1);
    assert_eq!(
        manifest.knowledge_exposure_source_statuses[0].source_status,
        "current"
    );
    assert_eq!(array_len(&manifest.manifest, "knowledge_spaces"), 1);
    assert_eq!(array_len(&manifest.manifest, "knowledge_exposures"), 1);
    assert_eq!(
        array_len(&manifest.manifest, "knowledge_exposure_local_sources"),
        1
    );
    assert_eq!(
        array_len(&manifest.manifest, "knowledge_exposure_transitions"),
        1
    );
    assert_eq!(
        array_len(&manifest.manifest, "knowledge_exposure_source_statuses"),
        1
    );

    let export = engine
        .export_bundle_payloads(BundlePayloadExportOptions::for_commit(
            adoption.relation.commit_id,
        ))
        .expect("export bundle payloads");
    let counts = role_counts(&export);
    assert_eq!(
        counts.get("knowledge_exposure_source_knowledge_state"),
        Some(&1)
    );
    assert_eq!(counts.get("knowledge_exposure_transition_detail"), Some(&1));
    assert_eq!(
        counts.get("knowledge_exposure_source_status_detail"),
        Some(&1)
    );
    assert!(export.payload_references.iter().any(|reference| {
        reference.role == "knowledge_exposure_transition_detail"
            && reference.content_digest == exposure.transition_detail_digest
    }));
    assert!(export.payload_references.iter().any(|reference| {
        reference.role == "knowledge_exposure_source_status_detail"
            && reference.content_digest == exposure.source_status.detail_digest
    }));

    let payload_inputs = export
        .payload_files
        .iter()
        .map(|payload| BundlePayloadInput::new(&payload.relative_path, payload.bytes.clone()))
        .collect::<workvcs_core::Result<Vec<_>>>()
        .expect("payload inputs");
    let validation = engine
        .validate_bundle_payloads(
            BundlePayloadValidationOptions::from_parts(
                adoption.relation.commit_id,
                export.manifest_bytes.clone(),
                export.payload_index_bytes.clone(),
                payload_inputs,
            )
            .expect("validation options"),
        )
        .expect("validate bundle payloads");
    assert!(validation.valid, "{:?}", validation.problem);
}
