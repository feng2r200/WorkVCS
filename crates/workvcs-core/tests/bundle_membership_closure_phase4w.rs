use std::collections::BTreeMap;
use tempfile::TempDir;
use workvcs_core::{
    BranchId, BundleExportOptions, BundlePayloadExport, BundlePayloadExportOptions, CanonicalValue,
    CommitId, Engine, StoreInitOptions, TaskCreateCommit, TaskCreateOptions,
    TaskSchedulingRelationCreateCommit, TaskSchedulingRelationCreateOptions, TaskStatus,
    TaskTransitionCommit, TaskTransitionOptions, WorkspaceInfo, WorkspaceInitOptions,
};

fn store_path() -> (TempDir, std::path::PathBuf) {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let path = tempdir.path().join("workvcs.sqlite");
    (tempdir, path)
}

fn init_engine(path: &std::path::Path) -> Engine {
    Engine::init(
        path,
        StoreInitOptions::new("phase4w-bundle-store").expect("store options"),
    )
    .expect("init engine")
}

fn create_workspace(path: &std::path::Path) -> (Engine, WorkspaceInfo) {
    let mut engine = init_engine(path);
    let workspace = engine
        .create_workspace(WorkspaceInitOptions::new("workspace").expect("workspace options"))
        .expect("create workspace");
    (engine, workspace)
}

fn create_task(
    engine: &mut Engine,
    branch_id: BranchId,
    head_commit_id: CommitId,
    description: &str,
) -> TaskCreateCommit {
    engine
        .create_task(
            TaskCreateOptions::new(branch_id, head_commit_id, description).expect("task options"),
        )
        .expect("create task")
}

fn transition_task(
    engine: &mut Engine,
    branch_id: BranchId,
    task: &TaskCreateCommit,
    next_status: TaskStatus,
) -> TaskTransitionCommit {
    engine
        .transition_task(
            TaskTransitionOptions::new(
                branch_id,
                task.commit_id,
                task.task_entity_id,
                task.task_entity_version_id,
                next_status,
            )
            .expect("transition options"),
        )
        .expect("transition task")
}

fn create_dependency(
    engine: &mut Engine,
    branch_id: BranchId,
    head_commit_id: CommitId,
    dependent: &TaskCreateCommit,
    prerequisite: &TaskCreateCommit,
) -> TaskSchedulingRelationCreateCommit {
    engine
        .create_task_scheduling_relation(
            TaskSchedulingRelationCreateOptions::depends_on(
                branch_id,
                head_commit_id,
                dependent.task_entity_id,
                prerequisite.task_entity_id,
            )
            .expect("dependency options"),
        )
        .expect("create dependency")
}

fn role_counts(export: &BundlePayloadExport) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    for reference in &export.payload_references {
        *counts.entry(reference.role.clone()).or_insert(0) += 1;
    }
    counts
}

fn array_len(value: &CanonicalValue, field: &str) -> usize {
    let CanonicalValue::Object(fields) = value else {
        panic!("manifest must be an object");
    };
    let Some((_, CanonicalValue::Array(values))) =
        fields.iter().find(|(candidate, _)| candidate == field)
    else {
        panic!("manifest field {field} must be an array");
    };
    values.len()
}

#[test]
fn entity_membership_changes_pull_before_versions_into_bundle_closure() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let created = create_task(
        &mut engine,
        workspace.initial_branch_id,
        workspace.genesis_commit_id,
        "Bundle entity membership closure",
    );
    let started = transition_task(
        &mut engine,
        workspace.initial_branch_id,
        &created,
        TaskStatus::InProgress,
    );

    let manifest = engine
        .export_bundle_manifest(BundleExportOptions::for_commit(started.commit_id))
        .expect("export bundle manifest");

    assert_eq!(manifest.commit_id, started.commit_id);
    assert_eq!(manifest.entity_count, 1);
    assert_eq!(manifest.relation_count, 0);
    assert_eq!(manifest.entity_membership_changes.len(), 2);
    assert_eq!(manifest.relation_membership_changes.len(), 0);
    assert_eq!(manifest.entity_versions.len(), 2);
    assert!(manifest.entity_versions.iter().any(|reference| {
        reference.entity_id == created.task_entity_id
            && reference.entity_version_id == created.task_entity_version_id
    }));
    assert!(manifest.entity_versions.iter().any(|reference| {
        reference.entity_id == started.task_entity_id
            && reference.entity_version_id == started.task_entity_version_id
    }));

    let transition_change = manifest
        .entity_membership_changes
        .iter()
        .find(|change| change.operation_id == started.operation_id)
        .expect("transition membership change");
    assert_eq!(
        transition_change.before_entity_version_id,
        Some(created.task_entity_version_id)
    );
    assert_eq!(
        transition_change.after_entity_version_id,
        Some(started.task_entity_version_id)
    );
    assert_eq!(
        array_len(&manifest.manifest, "entity_membership_changes"),
        2
    );

    let payloads = engine
        .export_bundle_payloads(BundlePayloadExportOptions::for_commit(started.commit_id))
        .expect("export bundle payloads");
    let counts = role_counts(&payloads);
    assert_eq!(counts.get("entity_membership_field_delta"), Some(&2));
    assert_eq!(counts.get("entity_version_state"), Some(&2));
    for change in &manifest.entity_membership_changes {
        assert!(payloads.payload_references.iter().any(|reference| {
            reference.role == "entity_membership_field_delta"
                && reference.content_digest == change.field_delta_digest
                && reference.relative_path == format!("payloads/{}.json", change.field_delta_digest)
        }));
    }
}

#[test]
fn relation_membership_changes_are_exported_with_payload_closure() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let prerequisite = create_task(
        &mut engine,
        workspace.initial_branch_id,
        workspace.genesis_commit_id,
        "Bundle prerequisite",
    );
    let dependent = create_task(
        &mut engine,
        workspace.initial_branch_id,
        prerequisite.commit_id,
        "Bundle dependent",
    );
    let dependency = create_dependency(
        &mut engine,
        workspace.initial_branch_id,
        dependent.commit_id,
        &dependent,
        &prerequisite,
    );

    let manifest = engine
        .export_bundle_manifest(BundleExportOptions::for_commit(dependency.commit_id))
        .expect("export bundle manifest");

    assert_eq!(manifest.commit_id, dependency.commit_id);
    assert_eq!(manifest.entity_membership_changes.len(), 2);
    assert_eq!(manifest.relation_membership_changes.len(), 1);
    assert_eq!(manifest.relation_versions.len(), 1);
    assert!(manifest.relation_versions.iter().any(|reference| {
        reference.relation_id == dependency.relation_id
            && reference.relation_version_id == dependency.relation_version_id
    }));
    let relation_change = &manifest.relation_membership_changes[0];
    assert_eq!(relation_change.operation_id, dependency.operation_id);
    assert_eq!(relation_change.relation_id, dependency.relation_id);
    assert_eq!(relation_change.before_relation_version_id, None);
    assert_eq!(
        relation_change.after_relation_version_id,
        Some(dependency.relation_version_id)
    );
    assert_eq!(
        array_len(&manifest.manifest, "relation_membership_changes"),
        1
    );

    let payloads = engine
        .export_bundle_payloads(BundlePayloadExportOptions::for_commit(dependency.commit_id))
        .expect("export bundle payloads");
    let counts = role_counts(&payloads);
    assert_eq!(counts.get("relation_membership_field_delta"), Some(&1));
    assert!(payloads.payload_references.iter().any(|reference| {
        reference.role == "relation_membership_field_delta"
            && reference.content_digest == relation_change.field_delta_digest
            && reference.relative_path
                == format!("payloads/{}.json", relation_change.field_delta_digest)
    }));
}
