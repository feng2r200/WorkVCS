use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;
use workvcs_core::{
    BundleImportPreflightOptions, BundleManifestValidationOptions, BundlePayloadExport,
    BundlePayloadExportOptions, BundlePayloadInput, BundlePayloadValidationOptions, CanonicalValue,
    DecisionRecordSupersedeCommit, DecisionRecordSupersedeOptions, Engine, RecordCreateOptions,
    StoreInitOptions, WorkspaceInfo, WorkspaceInitOptions, parse_canonical_json,
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
        StoreInitOptions::new("phase4br-bundle-changeset-anchor-export-store")
            .expect("store options"),
    )
    .expect("init engine");
    let workspace = engine
        .create_workspace(WorkspaceInitOptions::new("workspace").expect("workspace options"))
        .expect("create workspace");
    (engine, workspace)
}

fn create_supersede_with_causal_anchor(
    engine: &mut Engine,
    workspace: &WorkspaceInfo,
) -> (DecisionRecordSupersedeCommit, String) {
    let prior = engine
        .create_record(
            RecordCreateOptions::decision(
                workspace.initial_branch_id,
                workspace.genesis_commit_id,
                "Use optimistic writes",
            )
            .expect("prior decision options"),
        )
        .expect("create prior decision");
    let finding = engine
        .create_record(
            RecordCreateOptions::finding(
                workspace.initial_branch_id,
                prior.commit_id,
                "Concurrent write tests fail without serialization",
            )
            .expect("finding options"),
        )
        .expect("create finding");
    let replacement = engine
        .create_record(
            RecordCreateOptions::decision(
                workspace.initial_branch_id,
                finding.commit_id,
                "Use serialized writes",
            )
            .expect("replacement decision options"),
        )
        .expect("create replacement decision");

    let superseded = engine
        .supersede_decision_record(
            DecisionRecordSupersedeOptions::new(
                workspace.initial_branch_id,
                replacement.commit_id,
                replacement.record_entity_id,
                prior.record_entity_id,
                prior.record_entity_version_id,
                "Finding caused the replacement decision",
            )
            .expect("supersede options")
            .with_causal_record(finding.record_entity_id),
        )
        .expect("supersede decision");

    (superseded, finding.record_entity_id.to_string())
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

fn validation_options(export: &BundlePayloadExport) -> BundlePayloadValidationOptions {
    BundlePayloadValidationOptions::from_parts(
        export.manifest.commit_id,
        export.manifest_bytes.clone(),
        export.payload_index_bytes.clone(),
        payload_inputs(export),
    )
    .expect("validation options")
}

fn manifest_array_len(value: &CanonicalValue, field: &str) -> usize {
    let CanonicalValue::Object(fields) = value else {
        panic!("manifest must be an object");
    };
    let Some((_, value)) = fields.iter().find(|(name, _)| name == field) else {
        panic!("manifest missing {field}");
    };
    let CanonicalValue::Array(values) = value else {
        panic!("manifest field {field} must be an array");
    };
    values.len()
}

#[test]
fn bundle_export_includes_changeset_causal_anchors() {
    let (_tempdir, source_path, _old_path) = store_paths();
    let (mut engine, workspace) = create_workspace(&source_path);
    let (superseded, finding_record_id) =
        create_supersede_with_causal_anchor(&mut engine, &workspace);

    let export = engine
        .export_bundle_payloads(BundlePayloadExportOptions::for_commit(superseded.commit_id))
        .expect("export bundle payloads");

    assert_eq!(export.manifest.changeset_causal_anchors.len(), 1);
    let anchor = &export.manifest.changeset_causal_anchors[0];
    assert_eq!(anchor.changeset_id, superseded.changeset_id);
    assert_eq!(anchor.ordinal, 0);
    assert_eq!(anchor.anchor_object_id, finding_record_id);
    assert_eq!(anchor.anchor_object_kind, "entity");

    let manifest = parse_canonical_json(&export.manifest_bytes).expect("parse manifest");
    assert_eq!(manifest_array_len(&manifest, "changeset_causal_anchors"), 1);

    let manifest_validation = engine
        .validate_bundle_manifest(
            BundleManifestValidationOptions::from_bytes(
                superseded.commit_id,
                export.manifest_bytes.clone(),
            )
            .expect("manifest validation options"),
        )
        .expect("validate manifest");
    assert!(
        manifest_validation.valid,
        "{:?}",
        manifest_validation.problem
    );

    let payload_validation = engine
        .validate_bundle_payloads(validation_options(&export))
        .expect("validate payload directory");
    assert!(payload_validation.valid, "{:?}", payload_validation.problem);
}

#[test]
fn bundle_preflight_defers_anchor_bundle_until_apply_preserves_anchors() {
    let (_tempdir, source_path, old_path) = store_paths();
    let (mut engine, workspace) = create_workspace(&source_path);
    let prior = engine
        .create_record(
            RecordCreateOptions::decision(
                workspace.initial_branch_id,
                workspace.genesis_commit_id,
                "Use optimistic writes",
            )
            .expect("prior decision options"),
        )
        .expect("create prior decision");
    let finding = engine
        .create_record(
            RecordCreateOptions::finding(
                workspace.initial_branch_id,
                prior.commit_id,
                "Concurrent write tests fail without serialization",
            )
            .expect("finding options"),
        )
        .expect("create finding");
    let replacement = engine
        .create_record(
            RecordCreateOptions::decision(
                workspace.initial_branch_id,
                finding.commit_id,
                "Use serialized writes",
            )
            .expect("replacement decision options"),
        )
        .expect("create replacement decision");
    drop(engine);
    fs::copy(&source_path, &old_path).expect("copy old store");

    let mut source_engine = Engine::open(&source_path).expect("open source store");
    let superseded = source_engine
        .supersede_decision_record(
            DecisionRecordSupersedeOptions::new(
                workspace.initial_branch_id,
                replacement.commit_id,
                replacement.record_entity_id,
                prior.record_entity_id,
                prior.record_entity_version_id,
                "Finding caused the replacement decision",
            )
            .expect("supersede options")
            .with_causal_record(finding.record_entity_id),
        )
        .expect("supersede decision");
    let export = source_engine
        .export_bundle_payloads(BundlePayloadExportOptions::for_commit(superseded.commit_id))
        .expect("export payloads");

    let old_engine = Engine::open(&old_path).expect("open old store");
    let preflight = old_engine
        .preflight_bundle_import(preflight_options(&export))
        .expect("preflight bundle import");

    assert!(preflight.valid, "{:?}", preflight.problem);
    assert_eq!(preflight.source_store_relation, "same_store");
    assert!(preflight.import_required);
    assert!(!preflight.can_apply);
    assert_eq!(preflight.action, "same_store_import_not_implemented");
    assert_eq!(preflight.exported_branch_heads, 1);
    assert_eq!(preflight.branch_heads_fast_forward, 1);
}
