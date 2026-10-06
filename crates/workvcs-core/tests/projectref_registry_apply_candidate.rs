use serde_json::{Value, json};
use std::time::{Duration, UNIX_EPOCH};
use workvcs_core::control_plane::{
    CanonicalPath, ControlPlaneDigest, JournalAdmissionActivationCandidate,
    JournalAdmissionActivationScope, JournalAdmissionCapability, MigrationBindingValidation,
    MigrationOwnershipRepairManifest, ProjectRegistryV1, ProjectRegistryV2,
    RoutingActivationCandidate, RoutingActivationScope, UtcTimestamp,
    build_registry_v1_migration_preview, build_registry_v1_migration_preview_with_repairs,
    materialize_registry_v1_migration_candidate,
};
use workvcs_core::{BranchId, StoreId, WorkspaceId};

fn registry_value(shared_target: bool) -> Value {
    let store_id = StoreId::new_v7();
    let workspace_id = WorkspaceId::new_v7();
    let branch_id = BranchId::new_v7();
    let second_target = if shared_target {
        json!({
            "store_path": "/tmp/workvcs-registry-apply/store.sqlite",
            "store_id": store_id,
            "workspace_id": workspace_id,
            "branch_id": branch_id
        })
    } else {
        json!({
            "store_path": "/tmp/workvcs-registry-apply/second.sqlite",
            "store_id": StoreId::new_v7(),
            "workspace_id": WorkspaceId::new_v7(),
            "branch_id": BranchId::new_v7()
        })
    };
    json!({
        "version": 1,
        "bindings": [
            {
                "identity_kind": "cwd",
                "identity": "/tmp/workvcs-registry-apply/alpha",
                "root": "/tmp/workvcs-registry-apply/alpha",
                "store_path": "/tmp/workvcs-registry-apply/store.sqlite",
                "store_id": store_id,
                "workspace_id": workspace_id,
                "branch_id": branch_id
            },
            {
                "identity_kind": "git-common-dir",
                "identity": "/tmp/workvcs-registry-apply/beta/.git",
                "root": "/tmp/workvcs-registry-apply/beta",
                "store_path": second_target["store_path"],
                "store_id": second_target["store_id"],
                "workspace_id": second_target["workspace_id"],
                "branch_id": second_target["branch_id"]
            }
        ]
    })
}

fn registry(shared_target: bool) -> ProjectRegistryV1 {
    ProjectRegistryV1::from_json_bytes(
        &serde_json::to_vec_pretty(&registry_value(shared_target)).unwrap(),
    )
    .expect("registry v1")
}

fn passed_validations(registry: &ProjectRegistryV1) -> Vec<MigrationBindingValidation> {
    registry
        .bindings()
        .iter()
        .enumerate()
        .map(|(index, _)| MigrationBindingValidation::passed(index, index + 1))
        .collect()
}

fn fixed_time() -> UtcTimestamp {
    UtcTimestamp::parse("2026-09-24T12:34:56.123Z").unwrap()
}

fn backup_path() -> CanonicalPath {
    CanonicalPath::parse("/tmp/workvcs-registry-apply/project-bindings.json.v1.digest.bak").unwrap()
}

fn backup_digest() -> ControlPlaneDigest {
    ControlPlaneDigest::raw(b"exact raw v1 backup bytes")
}

fn repair_manifest(registry: &ProjectRegistryV1) -> MigrationOwnershipRepairManifest {
    let binding = &registry.bindings()[0];
    let value = json!({
        "schema_version": 1,
        "expected_source_digest": registry.source_digest().unwrap(),
        "repairs": [{
            "v1_binding_key": {
                "identity_kind": binding.identity_kind(),
                "identity": binding.identity()
            },
            "expected_target_digest": binding.target_digest().unwrap(),
            "semantic_locator": {
                "authority": "semantic_project",
                "provider": "chatgpt",
                "namespace": "local-installation:test",
                "kind": "project_id",
                "normalized_value": "g-p-apply-test",
                "assurance": "authoritative",
                "source_adapter": "test-project-metadata/v1",
                "evidence_digest": ControlPlaneDigest::raw(b"apply semantic evidence")
            },
            "historical_identity_disposition": "retire"
        }]
    });
    MigrationOwnershipRepairManifest::from_json_bytes(&serde_json::to_vec_pretty(&value).unwrap())
        .expect("repair manifest")
}

#[test]
fn utc_timestamp_from_system_time_is_canonical_across_calendar_boundaries() {
    assert_eq!(
        UtcTimestamp::from_system_time(UNIX_EPOCH).unwrap().as_str(),
        "1970-01-01T00:00:00Z"
    );
    let leap_day =
        UNIX_EPOCH + Duration::from_secs(1_709_164_800) + Duration::from_nanos(120_000_000);
    assert_eq!(
        UtcTimestamp::from_system_time(leap_day).unwrap().as_str(),
        "2024-02-29T00:00:00.12Z"
    );
}

#[test]
fn migration_candidate_preserves_targets_and_persists_one_to_one_receipts() {
    let source = registry(true);
    let preview =
        build_registry_v1_migration_preview(&source, &passed_validations(&source)).unwrap();
    let candidate = materialize_registry_v1_migration_candidate(
        &preview,
        &fixed_time(),
        &backup_path(),
        &backup_digest(),
    )
    .expect("candidate");

    assert_eq!(candidate.revision(), 1);
    assert_eq!(candidate.projects().len(), 2);
    assert_eq!(candidate.bindings().len(), 2);
    assert!(candidate.links().is_empty());
    assert_eq!(candidate.observations().len(), 1);
    assert_eq!(candidate.migration().unwrap().mappings().len(), 2);
    assert_eq!(
        candidate.migration().unwrap().source_digest(),
        preview.source_digest()
    );
    assert_eq!(
        candidate.migration().unwrap().preview_digest(),
        preview.preview_digest()
    );
    assert_eq!(
        candidate.migration().unwrap().backup_digest(),
        &backup_digest()
    );
    assert_eq!(
        candidate.migration().unwrap().backup_path().as_str(),
        backup_path().as_str()
    );

    for (mapping, receipt) in preview
        .mappings()
        .iter()
        .zip(candidate.migration().unwrap().mappings())
    {
        assert_eq!(
            receipt.v1_identity_kind(),
            mapping.v1_binding_key().identity_kind()
        );
        assert_eq!(
            receipt.v1_identity().as_str(),
            mapping.v1_binding_key().identity()
        );
        let binding = candidate
            .bindings()
            .iter()
            .find(|binding| binding.project_ref_id() == receipt.project_ref_id())
            .expect("mapped binding");
        assert_eq!(binding.store_path().as_str(), mapping.target().store_path());
        assert_eq!(binding.store_id().to_string(), mapping.target().store_id());
        assert_eq!(
            binding.workspace_id().to_string(),
            mapping.target().workspace_id()
        );
        assert_eq!(
            binding.branch_id().to_string(),
            mapping.target().branch_id()
        );
    }

    let canonical = candidate.canonical_json_bytes().unwrap();
    assert_eq!(
        ProjectRegistryV2::from_json_bytes(&canonical).unwrap(),
        candidate
    );
    assert_eq!(
        candidate.digest().unwrap(),
        ControlPlaneDigest::raw(&canonical)
    );
}

#[test]
fn read_routing_activation_candidate_is_exact_registry_snapshot_bound() {
    let source = registry(false);
    let preview =
        build_registry_v1_migration_preview(&source, &passed_validations(&source)).unwrap();
    let installed = materialize_registry_v1_migration_candidate(
        &preview,
        &fixed_time(),
        &backup_path(),
        &backup_digest(),
    )
    .expect("registry v2 candidate");
    let activation =
        RoutingActivationCandidate::for_registry(&installed).expect("activation candidate");

    assert_eq!(activation.activation_version(), 1);
    assert_eq!(
        activation.scope(),
        RoutingActivationScope::ProjectRefV2ReadRouting
    );
    assert_eq!(activation.registry_id(), installed.registry_id());
    assert_eq!(activation.registry_revision(), installed.revision());
    assert_eq!(activation.registry_digest(), &installed.digest().unwrap());
    assert_eq!(
        activation.digest().unwrap(),
        ControlPlaneDigest::raw(&activation.canonical_json_bytes().unwrap())
    );

    let reparsed = RoutingActivationCandidate::from_json_bytes(
        &activation.stored_json_bytes().expect("stored activation"),
    )
    .expect("reparse activation");
    assert_eq!(reparsed, activation);
    reparsed
        .validate_registry(&installed)
        .expect("matching registry");

    let mut changed: Value =
        serde_json::from_slice(&installed.canonical_json_bytes().unwrap()).unwrap();
    changed["revision"] = json!(installed.revision() + 1);
    let changed = ProjectRegistryV2::from_json_bytes(&serde_json::to_vec(&changed).unwrap())
        .expect("valid changed registry");
    assert!(reparsed.validate_registry(&changed).is_err());

    let mut unknown: Value =
        serde_json::from_slice(&activation.canonical_json_bytes().unwrap()).unwrap();
    unknown["unexpected"] = json!(true);
    assert!(
        RoutingActivationCandidate::from_json_bytes(&serde_json::to_vec(&unknown).unwrap())
            .is_err()
    );
}

#[test]
fn journal_admission_activation_candidate_is_separate_and_exact_snapshot_bound() {
    let source = registry(false);
    let preview =
        build_registry_v1_migration_preview(&source, &passed_validations(&source)).unwrap();
    let installed = materialize_registry_v1_migration_candidate(
        &preview,
        &fixed_time(),
        &backup_path(),
        &backup_digest(),
    )
    .expect("registry v2 candidate");
    let activation = JournalAdmissionActivationCandidate::for_registry(&installed)
        .expect("journal-admission activation candidate");

    assert_eq!(activation.activation_version(), 2);
    assert_eq!(
        activation.capabilities(),
        vec![
            JournalAdmissionCapability::CognitionCapture,
            JournalAdmissionCapability::PlanAdmit,
            JournalAdmissionCapability::PlanEvolve,
        ]
    );
    assert_eq!(
        activation.scope(),
        JournalAdmissionActivationScope::ProjectRefV2JournalAdmission
    );
    assert_eq!(activation.registry_id(), installed.registry_id());
    assert_eq!(activation.registry_revision(), installed.revision());
    assert_eq!(activation.registry_digest(), &installed.digest().unwrap());

    let reparsed = JournalAdmissionActivationCandidate::from_json_bytes(
        &activation.stored_json_bytes().unwrap(),
    )
    .expect("reparse journal-admission activation");
    assert_eq!(reparsed, activation);
    reparsed.validate_registry(&installed).unwrap();

    let mut legacy: Value =
        serde_json::from_slice(&activation.canonical_json_bytes().unwrap()).unwrap();
    legacy["activation_version"] = json!(1);
    legacy.as_object_mut().unwrap().remove("capabilities");
    let legacy =
        JournalAdmissionActivationCandidate::from_json_bytes(&serde_json::to_vec(&legacy).unwrap())
            .expect("legacy v1 marker remains readable");
    assert!(legacy.supports(JournalAdmissionCapability::CognitionCapture));
    assert!(!legacy.supports(JournalAdmissionCapability::PlanAdmit));
    assert!(legacy.is_strict_capability_predecessor_of(&activation));

    let mut lateral: Value =
        serde_json::from_slice(&activation.canonical_json_bytes().unwrap()).unwrap();
    lateral["capabilities"] = json!(["cognition_capture", "plan_admit"]);
    let lateral = JournalAdmissionActivationCandidate::from_json_bytes(
        &serde_json::to_vec(&lateral).unwrap(),
    )
    .expect("a valid partial v2 marker remains inspectable");
    assert!(!lateral.is_strict_capability_predecessor_of(&activation));

    assert!(
        JournalAdmissionActivationCandidate::from_json_bytes(
            &RoutingActivationCandidate::for_registry(&installed)
                .unwrap()
                .stored_json_bytes()
                .unwrap()
        )
        .is_err(),
        "the read-routing marker must not satisfy journal-admission activation"
    );
}

#[test]
fn repaired_candidate_activates_semantic_owner_and_retires_historical_path() {
    let source = registry(false);
    let manifest = repair_manifest(&source);
    let preview = build_registry_v1_migration_preview_with_repairs(
        &source,
        &passed_validations(&source),
        Some(&manifest),
    )
    .unwrap();
    let candidate = materialize_registry_v1_migration_candidate(
        &preview,
        &fixed_time(),
        &backup_path(),
        &backup_digest(),
    )
    .expect("repaired candidate");
    let value: Value = serde_json::from_slice(&candidate.canonical_json_bytes().unwrap()).unwrap();
    let receipt = candidate
        .migration()
        .unwrap()
        .mappings()
        .iter()
        .find(|mapping| mapping.v1_identity_kind() == "cwd")
        .unwrap();
    let project_id = receipt.project_ref_id().to_string();
    let locators = value["locators"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|locator| locator["project_ref_id"] == project_id)
        .collect::<Vec<_>>();
    assert_eq!(locators.len(), 2);
    let semantic = locators
        .iter()
        .find(|locator| locator["authority"] == "semantic_project")
        .unwrap();
    assert_eq!(semantic["state"], "active");
    assert_eq!(semantic["namespace"], "local-installation:test");
    assert_eq!(semantic["normalized_value"], "g-p-apply-test");
    assert_eq!(
        semantic["evidence_digest"],
        serde_json::to_value(ControlPlaneDigest::raw(b"apply semantic evidence")).unwrap()
    );
    let historical = locators
        .iter()
        .find(|locator| locator["authority"] == "cwd")
        .unwrap();
    assert_eq!(historical["state"], "retired");
    assert_eq!(
        historical["namespace"],
        format!("registry:{}", candidate.registry_id())
    );
    assert_eq!(
        candidate
            .projects()
            .iter()
            .find(|project| project.project_ref_id() == receipt.project_ref_id())
            .unwrap()
            .maturity(),
        workvcs_core::control_plane::ProjectMaturity::Established
    );
}

#[test]
fn migration_candidate_rejects_ineligible_preview_and_invalid_receipt_mapping() {
    let source = registry(false);
    let validations = vec![
        MigrationBindingValidation::failed(0, "fixture_invalid", "fixture is invalid").unwrap(),
        MigrationBindingValidation::passed(1, 0),
    ];
    let preview = build_registry_v1_migration_preview(&source, &validations).unwrap();
    assert!(
        materialize_registry_v1_migration_candidate(
            &preview,
            &fixed_time(),
            &backup_path(),
            &backup_digest(),
        )
        .is_err()
    );

    let valid_preview =
        build_registry_v1_migration_preview(&source, &passed_validations(&source)).unwrap();
    let candidate = materialize_registry_v1_migration_candidate(
        &valid_preview,
        &fixed_time(),
        &backup_path(),
        &backup_digest(),
    )
    .unwrap();
    let mut value: Value =
        serde_json::from_slice(&candidate.canonical_json_bytes().unwrap()).unwrap();
    value["migration"]["mappings"].as_array_mut().unwrap().pop();
    assert!(ProjectRegistryV2::from_json_bytes(&serde_json::to_vec(&value).unwrap()).is_err());
}
