use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;
use workvcs_core::{
    AcceptanceCriterionClassification, AcceptanceCriterionCreateCommit,
    AcceptanceCriterionCreateOptions, BundleImportApplyOptions, BundleImportPreflightOptions,
    BundlePayloadExport, BundlePayloadExportOptions, BundlePayloadInput, CanonicalValue, CommitId,
    Engine, EvidenceContentInput, EvidenceCreateOptions, ResourceCreateOptions,
    ResourceObservationCreateOptions, ResourceObservationDetailInput, SessionEndOptions,
    SessionLifecycleState, SessionStartOptions, StoreInitOptions, TaskCreateCommit,
    TaskCreateOptions, VerificationCreateOptions, VerificationResourceBasis, VerificationResult,
    VerificationTarget, WorkspaceInfo, WorkspaceInitOptions, content_object_digest,
};

fn store_paths() -> (TempDir, PathBuf, PathBuf) {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let source_path = tempdir.path().join("source.sqlite");
    let old_path = tempdir.path().join("old.sqlite");
    (tempdir, source_path, old_path)
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
        StoreInitOptions::new("phase4ba-bundle-session-provenance-apply-store")
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
                "ac.session-provenance",
                "The task has verification provenance tied to an ended session.",
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
fn bundle_apply_restores_ended_source_session_provenance() {
    let (_tempdir, source_path, old_path) = store_paths();
    let (mut engine, workspace) = create_workspace(&source_path);
    let task = create_task(
        &mut engine,
        &workspace,
        workspace.genesis_commit_id,
        "task with ended session provenance",
    );
    let acceptance = create_acceptance(&mut engine, &workspace, &task);
    drop(engine);
    fs::copy(&source_path, &old_path).expect("copy old store");

    let mut source_engine = Engine::open(&source_path).expect("open source store");
    let session = source_engine
        .start_session(
            SessionStartOptions::new(workspace.workspace_id, workspace.initial_branch_id)
                .expect("session options")
                .with_metadata(object(vec![("agent", string("phase-4ba"))]))
                .expect("session metadata"),
        )
        .expect("start session");
    let session_id = session.session_id;
    let resource = source_engine
        .create_resource(ResourceCreateOptions::new("git").expect("resource options"))
        .expect("create resource");
    let fingerprint = content_object_digest(b"phase4ba-resource-fingerprint");
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
            .with_source_session_id(session_id)
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
            .with_source_session_id(session_id)
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
    let ended = source_engine
        .end_session(
            SessionEndOptions::new(session_id)
                .expect("session end options")
                .with_summary(object(vec![("result", string("done"))]))
                .expect("session summary"),
        )
        .expect("end session");

    let export = source_engine
        .export_bundle_payloads(BundlePayloadExportOptions::for_commit(
            verification.commit_id,
        ))
        .expect("export payloads");
    assert_eq!(export.manifest.sessions.len(), 1);
    assert_eq!(export.manifest.session_diffs.len(), 1);
    assert_eq!(export.manifest.sessions[0].session_id, session_id);
    assert_eq!(
        export.manifest.session_diffs[0].session_diff_id,
        ended.session_diff_id
    );
    assert_eq!(
        export.manifest.evidences[0].source_session_id,
        Some(session_id)
    );
    assert_eq!(
        export.manifest.resource_observations[0].source_session_id,
        Some(session_id)
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
    assert_eq!(applied.imported_sessions, 1);
    assert_eq!(applied.imported_session_diffs, 1);
    assert_eq!(applied.imported_evidences, 1);
    assert_eq!(applied.imported_resource_observations, 1);

    let imported_session = old_engine
        .session_snapshot(session_id)
        .expect("imported ended session");
    assert_eq!(
        imported_session.lifecycle_state,
        SessionLifecycleState::Ended
    );
    assert_eq!(
        imported_session.session_diff_id,
        Some(ended.session_diff_id)
    );
    assert_eq!(imported_session.metadata, session.state.metadata);
    let imported_evidence = old_engine
        .evidence(evidence.evidence_id)
        .expect("imported evidence");
    assert_eq!(imported_evidence.source_session_id, Some(session_id));
    let imported_observation = old_engine
        .resource_observation(observation.observation_id)
        .expect("imported resource observation");
    assert_eq!(imported_observation.source_session_id, Some(session_id));
}
