use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;
use workvcs_core::{
    AcceptanceCriterionClassification, AcceptanceCriterionCreateCommit,
    AcceptanceCriterionCreateOptions, BundleImportApplyOptions, BundleImportPreflightOptions,
    BundlePayloadExport, BundlePayloadExportOptions, BundlePayloadInput, CanonicalValue, CommitId,
    Engine, EvidenceContentInput, EvidenceCreateOptions, ResourceCreateOptions,
    ResourceObservationCreateOptions, ResourceObservationDetailInput, StoreInitOptions,
    TaskCreateCommit, TaskCreateOptions, VerificationCreateOptions,
    VerificationRequirementCreateOptions, VerificationResourceBasis, VerificationResult,
    VerificationTarget, WorkspaceInfo, WorkspaceInitOptions, content_object_digest,
};

fn store_paths() -> (TempDir, PathBuf, PathBuf) {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let source_path = tempdir.path().join("source.sqlite");
    let old_path = tempdir.path().join("old.sqlite");
    (tempdir, source_path, old_path)
}

#[test]
fn bundle_apply_imports_verified_at_commit_before_verification_basis() {
    let (_tempdir, source_path, old_path) = store_paths();
    let (mut engine, workspace) = create_workspace(&source_path);
    let task = create_task(
        &mut engine,
        &workspace,
        workspace.genesis_commit_id,
        "baseline task before copied target",
    );
    drop(engine);
    fs::copy(&source_path, &old_path).expect("copy old store");

    let mut source_engine = Engine::open(&source_path).expect("open source store");
    let acceptance = create_acceptance(&mut source_engine, &workspace, &task);
    let requirement = source_engine
        .create_verification_requirement(
            VerificationRequirementCreateOptions::new(
                workspace.initial_branch_id,
                acceptance.commit_id,
                acceptance.acceptance_criterion_entity_id,
                acceptance.acceptance_criterion_entity_version_id,
                "vr.imported-verified-at",
                "The bundle importer must import the verified-at commit before verification basis rows.",
            )
            .expect("verification requirement options"),
        )
        .expect("create verification requirement");
    let evidence = source_engine
        .create_evidence(
            EvidenceCreateOptions::new(
                "test-log",
                object(vec![(
                    "summary",
                    string("verification requirement evidence"),
                )]),
            )
            .expect("evidence options")
            .with_contents(vec![
                EvidenceContentInput::from_raw_bytes(
                    "log",
                    b"verification requirement evidence body",
                )
                .expect("evidence content"),
            ])
            .expect("evidence contents"),
        )
        .expect("create evidence");
    let verification = source_engine
        .create_verification(
            VerificationCreateOptions::new(
                workspace.initial_branch_id,
                requirement.commit_id,
                VerificationTarget::VerificationRequirement(
                    requirement.verification_requirement_entity_id,
                ),
                VerificationResult::Passed,
            )
            .expect("verification options")
            .with_evidence(vec![evidence.evidence_id])
            .expect("verification evidence"),
        )
        .expect("create verification");

    let export = source_engine
        .export_bundle_payloads(BundlePayloadExportOptions::for_commit(
            verification.commit_id,
        ))
        .expect("export payloads");
    assert_eq!(export.manifest.verification_bases.len(), 1);
    assert_eq!(
        export.manifest.verification_bases[0].verified_at_commit_id,
        requirement.commit_id
    );
    assert_eq!(export.manifest.verification_semantic_dependencies.len(), 1);

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
    assert_eq!(applied.outcome, "same_store_fast_forward_applied");
    assert_eq!(applied.imported_commits, 3);
    assert_eq!(applied.imported_entity_versions, 5);
    assert_eq!(applied.imported_content_objects, 1);
    assert_eq!(applied.imported_evidences, 1);
    assert_eq!(applied.imported_verification_bases, 1);
    assert_eq!(applied.imported_relation_versions, 2);
    assert_eq!(applied.updated_branch_heads, 1);

    let state = old_engine
        .show_at(verification.commit_id)
        .expect("show imported verification commit");
    assert_eq!(state.state_digest, verification.work_state_digest);
    let imported_verification = old_engine
        .verification_at(verification.commit_id, verification.verification_entity_id)
        .expect("imported verification");
    assert_eq!(
        imported_verification.state.verified_at_commit_id,
        requirement.commit_id
    );
    assert_eq!(
        imported_verification.target,
        VerificationTarget::VerificationRequirement(requirement.verification_requirement_entity_id)
    );
    assert_eq!(
        imported_verification.evidenced_by_relations[0].evidence_id,
        evidence.evidence_id
    );
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
        StoreInitOptions::new("phase4az-bundle-verification-object-apply-store")
            .expect("store options"),
    )
    .expect("init engine");
    let workspace = engine
        .create_workspace(WorkspaceInitOptions::new("workspace").expect("workspace options"))
        .expect("create workspace");
    (engine, workspace)
}

fn create_task(
    engine: &mut Engine,
    workspace: &WorkspaceInfo,
    head: CommitId,
    description: &str,
) -> TaskCreateCommit {
    engine
        .create_task(
            TaskCreateOptions::new(workspace.initial_branch_id, head, description)
                .expect("task options"),
        )
        .expect("create task")
}

fn create_acceptance(
    engine: &mut Engine,
    workspace: &WorkspaceInfo,
    task: &TaskCreateCommit,
) -> AcceptanceCriterionCreateCommit {
    engine
        .create_acceptance_criterion(
            AcceptanceCriterionCreateOptions::new(
                workspace.initial_branch_id,
                task.commit_id,
                task.task_entity_id,
                task.task_entity_version_id,
                "ac.verifiable",
                "The task has a verification record with evidence and resource basis.",
                AcceptanceCriterionClassification::Required,
            )
            .expect("acceptance options"),
        )
        .expect("create acceptance criterion")
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
fn bundle_apply_restores_verification_evidence_resource_and_basis() {
    let (_tempdir, source_path, old_path) = store_paths();
    let (mut engine, workspace) = create_workspace(&source_path);
    let task = create_task(
        &mut engine,
        &workspace,
        workspace.genesis_commit_id,
        "task with verification object closure",
    );
    let acceptance = create_acceptance(&mut engine, &workspace, &task);
    drop(engine);
    fs::copy(&source_path, &old_path).expect("copy old store");

    let mut source_engine = Engine::open(&source_path).expect("open source store");
    let resource = source_engine
        .create_resource(ResourceCreateOptions::new("git").expect("resource options"))
        .expect("create resource");
    let fingerprint = content_object_digest(b"phase4az-resource-fingerprint");
    let observation = source_engine
        .record_resource_observation(
            ResourceObservationCreateOptions::new(
                resource.resource_id,
                "git",
                1,
                fingerprint,
                object(vec![("path", string("crates/workvcs-core/src/lib.rs"))]),
            )
            .expect("observation options")
            .with_detail_content(
                ResourceObservationDetailInput::from_raw_bytes(b"resource observation detail")
                    .expect("observation detail"),
            ),
        )
        .expect("record resource observation");
    let evidence = source_engine
        .create_evidence(
            EvidenceCreateOptions::new(
                "test-log",
                object(vec![("summary", string("verification evidence"))]),
            )
            .expect("evidence options")
            .with_contents(vec![
                EvidenceContentInput::from_raw_bytes("log", b"verification evidence body")
                    .expect("evidence content"),
            ])
            .expect("evidence contents"),
        )
        .expect("create evidence");
    let resource_basis = VerificationResourceBasis::new(
        resource.resource_id,
        "git",
        1,
        "path",
        1,
        object(vec![("path", string("crates/workvcs-core/src/lib.rs"))]),
        fingerprint,
    )
    .expect("resource basis")
    .with_baseline_observation_id(observation.observation_id)
    .expect("resource basis observation");
    let verification = source_engine
        .create_verification(
            VerificationCreateOptions::new(
                workspace.initial_branch_id,
                acceptance.commit_id,
                VerificationTarget::AcceptanceCriterion(acceptance.acceptance_criterion_entity_id),
                VerificationResult::Passed,
            )
            .expect("verification options")
            .with_evidence(vec![evidence.evidence_id])
            .expect("verification evidence")
            .with_resource_basis(vec![resource_basis])
            .expect("verification resource basis"),
        )
        .expect("create verification");

    let export = source_engine
        .export_bundle_payloads(BundlePayloadExportOptions::for_commit(
            verification.commit_id,
        ))
        .expect("export payloads");
    assert_eq!(export.manifest.evidences.len(), 1);
    assert_eq!(export.manifest.evidence_contents.len(), 1);
    assert_eq!(export.manifest.resources.len(), 1);
    assert_eq!(export.manifest.resource_observations.len(), 1);
    assert_eq!(export.manifest.content_objects.len(), 2);
    assert_eq!(export.manifest.verification_bases.len(), 1);
    assert_eq!(export.manifest.verification_resource_bases.len(), 1);
    assert_eq!(export.manifest.verification_semantic_dependencies.len(), 1);
    assert_eq!(export.manifest.relation_versions.len(), 2);
    assert_eq!(
        export.manifest.evidences[0].evidence_id,
        evidence.evidence_id
    );
    assert_eq!(
        export.manifest.resources[0].resource_id,
        resource.resource_id
    );
    assert_eq!(
        export.manifest.resource_observations[0].observation_id,
        observation.observation_id
    );

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
    assert_eq!(applied.outcome, "same_store_fast_forward_applied");
    assert_eq!(applied.imported_commits, 1);
    assert_eq!(applied.imported_entity_versions, 1);
    assert_eq!(applied.imported_content_objects, 2);
    assert_eq!(applied.imported_evidences, 1);
    assert_eq!(applied.imported_resources, 1);
    assert_eq!(applied.imported_resource_observations, 1);
    assert_eq!(applied.imported_verification_bases, 1);
    assert_eq!(applied.imported_relation_versions, 2);
    assert_eq!(applied.updated_branch_heads, 1);

    let imported_evidence = old_engine
        .evidence(evidence.evidence_id)
        .expect("imported evidence");
    assert_eq!(imported_evidence.evidence_kind, "test-log");
    assert_eq!(imported_evidence.contents.len(), 1);
    let imported_resource = old_engine
        .resource(resource.resource_id)
        .expect("imported resource");
    assert_eq!(imported_resource.resource_kind, "git");
    let imported_observation = old_engine
        .resource_observation(observation.observation_id)
        .expect("imported resource observation");
    assert_eq!(imported_observation.resource_id, resource.resource_id);
    assert_eq!(imported_observation.fingerprint, fingerprint);

    let imported_verification = old_engine
        .verification_at(verification.commit_id, verification.verification_entity_id)
        .expect("imported verification");
    assert_eq!(
        imported_verification.state.result,
        VerificationResult::Passed
    );
    assert_eq!(imported_verification.state.evidence.len(), 1);
    assert_eq!(
        imported_verification.state.evidence[0].evidence_id,
        evidence.evidence_id
    );
    assert_eq!(imported_verification.state.resource_basis.len(), 1);
    assert_eq!(
        imported_verification.state.resource_basis[0].resource_id,
        resource.resource_id
    );
    assert_eq!(
        imported_verification.state.resource_basis[0].baseline_observation_id,
        Some(observation.observation_id)
    );
    assert_eq!(
        imported_verification.verifies_relation_id,
        verification.verifies_relation_id
    );
    assert_eq!(
        imported_verification.evidenced_by_relations[0].evidence_id,
        evidence.evidence_id
    );
}
