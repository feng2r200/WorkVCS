use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;
use workvcs_core::{
    AcceptanceCriterionClassification, AcceptanceCriterionCreateOptions, BundleImportApplyOptions,
    BundleImportPreflightOptions, BundlePayloadExport, BundlePayloadExportOptions,
    BundlePayloadInput, CommitId, Engine, StoreInitOptions, TaskCreateCommit, TaskCreateOptions,
    VerificationRequirementCreateOptions, WorkspaceInfo, WorkspaceInitOptions,
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
        StoreInitOptions::new("phase4ax-bundle-typed-identity-apply-store").expect("store options"),
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
fn bundle_apply_restores_acceptance_and_verification_requirement_identities() {
    let (_tempdir, source_path, old_path) = store_paths();
    let (mut engine, workspace) = create_workspace(&source_path);
    let task = create_task(
        &mut engine,
        &workspace,
        workspace.genesis_commit_id,
        "task with criteria",
    );
    drop(engine);
    fs::copy(&source_path, &old_path).expect("copy old store");

    let mut source_engine = Engine::open(&source_path).expect("open source store");
    let acceptance = source_engine
        .create_acceptance_criterion(
            AcceptanceCriterionCreateOptions::new(
                workspace.initial_branch_id,
                task.commit_id,
                task.task_entity_id,
                task.task_entity_version_id,
                "ac.done",
                "Done means the task has an independently checkable result.",
                AcceptanceCriterionClassification::Required,
            )
            .expect("acceptance options"),
        )
        .expect("create acceptance criterion");
    let requirement = source_engine
        .create_verification_requirement(
            VerificationRequirementCreateOptions::new(
                workspace.initial_branch_id,
                acceptance.commit_id,
                acceptance.acceptance_criterion_entity_id,
                acceptance.acceptance_criterion_entity_version_id,
                "vr.manual-review",
                "A reviewer can reproduce the result from the stored evidence.",
            )
            .expect("verification requirement options"),
        )
        .expect("create verification requirement");
    let export = source_engine
        .export_bundle_payloads(BundlePayloadExportOptions::for_commit(
            requirement.commit_id,
        ))
        .expect("export payloads");

    assert_eq!(export.manifest.acceptance_criterion_identities.len(), 1);
    assert_eq!(
        export.manifest.acceptance_criterion_identities[0].entity_id,
        acceptance.acceptance_criterion_entity_id
    );
    assert_eq!(
        export.manifest.acceptance_criterion_identities[0].owner_entity_id,
        task.task_entity_id
    );
    assert_eq!(
        export.manifest.acceptance_criterion_identities[0].local_key,
        "ac.done"
    );
    assert_eq!(export.manifest.verification_requirement_identities.len(), 1);
    assert_eq!(
        export.manifest.verification_requirement_identities[0].entity_id,
        requirement.verification_requirement_entity_id
    );
    assert_eq!(
        export.manifest.verification_requirement_identities[0].owner_entity_id,
        acceptance.acceptance_criterion_entity_id
    );
    assert_eq!(
        export.manifest.verification_requirement_identities[0].local_key,
        "vr.manual-review"
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
    assert_eq!(applied.imported_commits, 2);
    assert_eq!(applied.imported_entity_versions, 4);
    assert_eq!(applied.imported_acceptance_criterion_identities, 1);
    assert_eq!(applied.imported_verification_requirement_identities, 1);
    assert_eq!(applied.updated_branch_heads, 1);

    let imported_acceptance = old_engine
        .acceptance_criterion_at(
            requirement.commit_id,
            acceptance.acceptance_criterion_entity_id,
        )
        .expect("imported acceptance criterion");
    assert_eq!(imported_acceptance.local_key, "ac.done");
    assert_eq!(imported_acceptance.task_entity_id, task.task_entity_id);
    assert_eq!(
        imported_acceptance.acceptance_criterion_entity_version_id,
        requirement.acceptance_criterion_entity_version_id
    );

    let imported_requirement = old_engine
        .verification_requirement_at(
            requirement.commit_id,
            requirement.verification_requirement_entity_id,
        )
        .expect("imported verification requirement");
    assert_eq!(imported_requirement.local_key, "vr.manual-review");
    assert_eq!(
        imported_requirement.acceptance_criterion_entity_id,
        acceptance.acceptance_criterion_entity_id
    );
    assert_eq!(
        imported_requirement.verification_requirement_entity_version_id,
        requirement.verification_requirement_entity_version_id
    );
}
