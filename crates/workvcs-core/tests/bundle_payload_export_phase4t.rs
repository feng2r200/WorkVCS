use std::collections::BTreeMap;
use tempfile::TempDir;
use workvcs_core::{
    BranchId, BundlePayloadExport, BundlePayloadExportOptions, CommitId, Digest, Engine,
    StoreInitOptions, TaskCreateCommit, TaskCreateOptions, TaskSchedulingRelationCreateCommit,
    TaskSchedulingRelationCreateOptions, WorkspaceInfo, WorkspaceInitOptions, canonical_bytes,
    content_object_digest, parse_canonical_json,
};

fn store_path() -> (TempDir, std::path::PathBuf) {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let path = tempdir.path().join("workvcs.sqlite");
    (tempdir, path)
}

fn init_engine(path: &std::path::Path) -> Engine {
    Engine::init(
        path,
        StoreInitOptions::new("phase4t-bundle-store").expect("store options"),
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

fn payload_file_digest_map(export: &BundlePayloadExport) -> BTreeMap<Digest, Vec<u8>> {
    export
        .payload_files
        .iter()
        .map(|payload| {
            assert_eq!(
                payload.content_digest,
                content_object_digest(&payload.bytes)
            );
            assert_eq!(
                usize::try_from(payload.size_bytes).expect("payload size"),
                payload.bytes.len()
            );
            assert_eq!(
                payload.relative_path,
                format!("payloads/{}.json", payload.content_digest)
            );
            assert_eq!(payload.media_type, "application/json");
            parse_canonical_json(&payload.bytes).expect("payload must be canonical semantic JSON");
            (payload.content_digest, payload.bytes.clone())
        })
        .collect()
}

#[test]
fn bundle_payload_export_includes_canonical_json_payload_closure() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let prerequisite = create_task(
        &mut engine,
        workspace.initial_branch_id,
        workspace.genesis_commit_id,
        "Bundle payload prerequisite",
    );
    let dependent = create_task(
        &mut engine,
        workspace.initial_branch_id,
        prerequisite.commit_id,
        "Bundle payload dependent",
    );
    let dependency = create_dependency(
        &mut engine,
        workspace.initial_branch_id,
        dependent.commit_id,
        &dependent,
        &prerequisite,
    );

    let export = engine
        .export_bundle_payloads(BundlePayloadExportOptions::for_commit(dependency.commit_id))
        .expect("export bundle payloads");

    assert_eq!(export.manifest.commit_id, dependency.commit_id);
    assert_eq!(
        export.manifest_bytes,
        canonical_bytes(&export.manifest.manifest).expect("manifest bytes")
    );
    assert_eq!(
        export.manifest.manifest_digest,
        content_object_digest(&export.manifest_bytes)
    );
    assert_eq!(
        export.payload_index_digest,
        content_object_digest(&export.payload_index_bytes)
    );
    assert_eq!(
        usize::try_from(export.payload_index_size_bytes).expect("index size"),
        export.payload_index_bytes.len()
    );
    let reparsed_index =
        parse_canonical_json(&export.payload_index_bytes).expect("payload index JSON");
    assert_eq!(
        canonical_bytes(&reparsed_index).expect("reencoded index"),
        export.payload_index_bytes
    );

    let counts = role_counts(&export);
    assert_eq!(counts.get("changeset_operation_payload"), Some(&4));
    assert_eq!(counts.get("changeset_rationale"), Some(&4));
    assert_eq!(counts.get("change_operation_payload"), Some(&3));
    assert_eq!(counts.get("entity_version_state"), Some(&2));
    assert_eq!(counts.get("entity_membership_field_delta"), Some(&2));
    assert_eq!(counts.get("relation_version_metadata"), Some(&1));
    assert_eq!(counts.get("relation_membership_field_delta"), Some(&1));
    assert_eq!(export.payload_references.len(), 17);

    let payloads = payload_file_digest_map(&export);
    for change in &export.manifest.entity_membership_changes {
        assert!(payloads.contains_key(&change.field_delta_digest));
        assert!(export.payload_references.iter().any(|reference| {
            reference.role == "entity_membership_field_delta"
                && reference.content_digest == change.field_delta_digest
                && reference.relative_path == format!("payloads/{}.json", change.field_delta_digest)
        }));
    }
    for change in &export.manifest.relation_membership_changes {
        assert!(payloads.contains_key(&change.field_delta_digest));
        assert!(export.payload_references.iter().any(|reference| {
            reference.role == "relation_membership_field_delta"
                && reference.content_digest == change.field_delta_digest
                && reference.relative_path == format!("payloads/{}.json", change.field_delta_digest)
        }));
    }
    for entity in &export.manifest.entity_versions {
        assert!(payloads.contains_key(&entity.state_json_digest));
        assert!(export.payload_references.iter().any(|reference| {
            reference.role == "entity_version_state"
                && reference.content_digest == entity.state_json_digest
                && reference.relative_path == format!("payloads/{}.json", entity.state_json_digest)
        }));
    }
    for relation in &export.manifest.relation_versions {
        assert!(payloads.contains_key(&relation.metadata_json_digest));
        assert!(export.payload_references.iter().any(|reference| {
            reference.role == "relation_version_metadata"
                && reference.content_digest == relation.metadata_json_digest
                && reference.relative_path
                    == format!("payloads/{}.json", relation.metadata_json_digest)
        }));
    }
}
