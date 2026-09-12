use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;
use workvcs_core::{
    AcceptanceCriterionClassification, AcceptanceCriterionCreateOptions, BundleImportApplyOptions,
    BundleImportPreflightOptions, BundlePayloadExport, BundlePayloadExportOptions,
    BundlePayloadInput, CanonicalValue, Engine, EvidenceContentInput, EvidenceCreateOptions,
    StoreInitOptions, TaskCreateOptions, VerificationCreateOptions, VerificationResult,
    VerificationTarget, WorkspaceInfo, WorkspaceInitOptions, content_object_digest,
};

fn store_paths() -> (TempDir, PathBuf, PathBuf) {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let source_dir = tempdir.path().join("source");
    let target_dir = tempdir.path().join("target");
    fs::create_dir_all(&source_dir).expect("source directory");
    fs::create_dir_all(&target_dir).expect("target directory");
    (
        tempdir,
        source_dir.join("store.sqlite"),
        target_dir.join("store.sqlite"),
    )
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

fn create_workspace(path: &Path) -> (Engine, WorkspaceInfo) {
    let mut engine = Engine::init(
        path,
        StoreInitOptions::new("bundle-evidence-portability-store").expect("store options"),
    )
    .expect("init engine");
    let workspace = engine
        .create_workspace(
            WorkspaceInitOptions::new("evidence portability").expect("workspace options"),
        )
        .expect("create workspace");
    (engine, workspace)
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

#[test]
fn bundle_round_trip_restores_raw_evidence_and_preserves_digest_only_semantics() {
    let (_tempdir, source_path, target_path) = store_paths();
    let (mut engine, workspace) = create_workspace(&source_path);
    let task = engine
        .create_task(
            TaskCreateOptions::new(
                workspace.initial_branch_id,
                workspace.genesis_commit_id,
                "restore raw Evidence from a Bundle",
            )
            .expect("task options"),
        )
        .expect("create task");
    let criterion = engine
        .create_acceptance_criterion(
            AcceptanceCriterionCreateOptions::new(
                workspace.initial_branch_id,
                task.commit_id,
                task.task_entity_id,
                task.task_entity_version_id,
                "ac.portable-evidence",
                "Raw Evidence remains extractable after copied-Store Bundle apply.",
                AcceptanceCriterionClassification::Required,
            )
            .expect("criterion options"),
        )
        .expect("create criterion");
    drop(engine);
    fs::copy(&source_path, &target_path).expect("copy target baseline");

    let raw_body = b"portable verification evidence\n";
    let digest_only_bytes = b"remote-only";
    let mut source = Engine::open(&source_path).expect("open source");
    let evidence = source
        .create_evidence(
            EvidenceCreateOptions::new(
                "test-log",
                object(vec![(
                    "summary",
                    string("portable and digest-only content"),
                )]),
            )
            .expect("evidence options")
            .with_contents(vec![
                EvidenceContentInput::from_raw_bytes("stdout", raw_body).expect("raw Evidence"),
                EvidenceContentInput::from_raw_bytes("empty", b"").expect("empty Evidence"),
                EvidenceContentInput::from_digest(
                    "external",
                    content_object_digest(digest_only_bytes),
                    digest_only_bytes.len() as i64,
                )
                .expect("digest-only Evidence"),
            ])
            .expect("Evidence contents"),
        )
        .expect("create Evidence");
    let verification = source
        .create_verification(
            VerificationCreateOptions::new(
                workspace.initial_branch_id,
                criterion.commit_id,
                VerificationTarget::AcceptanceCriterion(criterion.acceptance_criterion_entity_id),
                VerificationResult::Passed,
            )
            .expect("verification options")
            .with_evidence(vec![evidence.evidence_id])
            .expect("verification Evidence"),
        )
        .expect("create verification");
    assert_eq!(
        source
            .validate_local_content_storage()
            .expect("validate source objects"),
        2
    );

    let export = source
        .export_bundle_payloads(BundlePayloadExportOptions::for_commit(
            verification.commit_id,
        ))
        .expect("export Bundle");
    assert_eq!(export.manifest.evidence_contents.len(), 3);
    assert_eq!(export.manifest.portable_evidence_contents.len(), 2);
    assert_eq!(
        export
            .payload_files
            .iter()
            .filter(|payload| payload.relative_path.starts_with("objects/"))
            .count(),
        2
    );

    let mut missing_body = payload_inputs(&export);
    let removed = missing_body
        .iter()
        .position(|payload| payload.relative_path.starts_with("objects/"))
        .expect("object payload");
    missing_body.remove(removed);
    let missing_preflight = source
        .preflight_bundle_import(
            BundleImportPreflightOptions::from_parts(
                export.manifest_bytes.clone(),
                export.payload_index_bytes.clone(),
                missing_body,
            )
            .expect("missing-body preflight options"),
        )
        .expect("missing-body preflight");
    assert!(!missing_preflight.valid);
    assert_eq!(missing_preflight.action, "invalid_bundle_directory");

    let mut tampered = payload_inputs(&export);
    tampered
        .iter_mut()
        .find(|payload| payload.relative_path.starts_with("objects/"))
        .expect("object payload")
        .bytes
        .push(b'!');
    let tampered_preflight = source
        .preflight_bundle_import(
            BundleImportPreflightOptions::from_parts(
                export.manifest_bytes.clone(),
                export.payload_index_bytes.clone(),
                tampered,
            )
            .expect("tampered preflight options"),
        )
        .expect("tampered preflight");
    assert!(!tampered_preflight.valid);
    assert_eq!(tampered_preflight.action, "invalid_bundle_directory");

    let mut target = Engine::open(&target_path).expect("open target");
    let preflight = target
        .preflight_bundle_import(
            BundleImportPreflightOptions::from_parts(
                export.manifest_bytes.clone(),
                export.payload_index_bytes.clone(),
                payload_inputs(&export),
            )
            .expect("preflight options"),
        )
        .expect("preflight");
    assert!(preflight.valid);
    assert!(preflight.can_apply);
    assert_eq!(preflight.portable_evidence_contents, 2);

    let applied = target
        .apply_bundle_import(
            BundleImportApplyOptions::from_parts(
                export.manifest_bytes.clone(),
                export.payload_index_bytes.clone(),
                payload_inputs(&export),
            )
            .expect("apply options"),
        )
        .expect("apply Bundle");
    assert!(applied.applied);
    assert_eq!(applied.imported_content_storage_locations, 2);
    assert_eq!(
        target
            .read_evidence_content(evidence.evidence_id, 0)
            .expect("read imported raw Evidence")
            .raw_bytes,
        raw_body
    );
    assert_eq!(
        target
            .read_evidence_content(evidence.evidence_id, 1)
            .expect("read imported empty Evidence")
            .raw_bytes,
        b""
    );
    assert!(
        target
            .read_evidence_content(evidence.evidence_id, 2)
            .is_err()
    );
    assert_eq!(
        target
            .validate_local_content_storage()
            .expect("validate target objects"),
        2
    );

    let source_snapshot = source
        .evidence(evidence.evidence_id)
        .expect("source Evidence snapshot");
    let source_locator = &source_snapshot.contents[0].storage_locations[0].locator;
    fs::remove_file(
        source_path
            .parent()
            .expect("source Store parent")
            .join(source_locator),
    )
    .expect("remove advertised source object");
    let unavailable = source
        .export_bundle_payloads(BundlePayloadExportOptions::for_commit(
            verification.commit_id,
        ))
        .expect_err("reject an advertised but missing source object");
    assert!(
        format!("{unavailable}").contains("is not readable"),
        "unexpected export error: {unavailable}"
    );
}
