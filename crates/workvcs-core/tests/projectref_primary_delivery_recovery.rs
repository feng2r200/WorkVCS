use serde_json::json;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;
use workvcs_core::control_plane::{
    CanonicalPath, CaptureEventAppendOutcome, CaptureEventPayload, CaptureGroupIntent,
    CaptureIntent, CaptureJournal, CapturePayloadKind, CaptureRecoveryState, ControlPlaneDigest,
    DeliveryAppliedPayload, DeliveryMode, DeliveryStartedPayload, LocatorAssurance,
    LocatorAuthority, LocatorEvidence, ProjectBindingReadyPayload, ProjectMaturity,
    ResolutionContext, ResolutionMode, ResolutionRecordedPayload, ResolutionResult, UtcTimestamp,
    build_primary_delivery_receipt, prepare_primary_delivery,
};
use workvcs_core::{
    CaptureId, CognitionCaptureOptions, CognitionCaptureOutcome, DeliveryId, Engine,
    HistoryQueryOptions, ProjectRefId, RegistryId, StoreInfo, StoreInitOptions, WorkspaceInfo,
    WorkspaceInitOptions,
};

const NOW: &str = "2026-09-27T10:00:00Z";

struct DeliveryFixture {
    _tempdir: TempDir,
    store_path: PathBuf,
    engine: Engine,
    store: StoreInfo,
    workspace: WorkspaceInfo,
    project_ref_id: ProjectRefId,
    intent: CaptureIntent,
    journal: CaptureJournal,
}

fn digest(label: &str) -> ControlPlaneDigest {
    ControlPlaneDigest::raw(label.as_bytes())
}

fn locator_evidence(project_ref_id: ProjectRefId) -> LocatorEvidence {
    LocatorEvidence::new(
        LocatorAuthority::SemanticProject,
        "chatgpt",
        "isolated:round-3",
        "project_id",
        format!("fixture-{project_ref_id}"),
        LocatorAssurance::Authoritative,
        "round-3-fixture/v1",
        digest("round-3 semantic owner"),
    )
    .unwrap()
}

fn resolved(project_ref_id: ProjectRefId, evidence: &LocatorEvidence) -> ResolutionResult {
    serde_json::from_value(json!({
        "status": "resolved",
        "primary_project_ref": project_ref_id,
        "primary_basis": {
            "kind": "locator",
            "rank": "semantic_project",
            "project_ref_id": project_ref_id,
            "evidence": evidence
        },
        "related_project_refs": [],
        "unmapped_locators": [],
        "diagnostics": []
    }))
    .unwrap()
}

fn fixture(with_secondary_reference: bool) -> DeliveryFixture {
    let tempdir = tempfile::tempdir().unwrap();
    let store_path = tempdir.path().join("primary.sqlite");
    let mut engine = Engine::init(
        &store_path,
        StoreInitOptions::new("round-3-primary").unwrap(),
    )
    .unwrap();
    let workspace = engine
        .create_workspace(WorkspaceInitOptions::new("primary").unwrap())
        .unwrap();
    let store = engine.store_info().unwrap();
    let store_path = fs::canonicalize(&store_path).unwrap();
    let project_ref_id = ProjectRefId::new_v7();
    let evidence = locator_evidence(project_ref_id);
    let context = ResolutionContext::new(ResolutionMode::DurableWrite)
        .with_project_ref(project_ref_id)
        .with_locator_evidence(evidence.clone());
    let resolution = resolved(project_ref_id, &evidence);
    let capture_group = with_secondary_reference.then(|| {
        let secondary = ProjectRefId::new_v7();
        serde_json::from_value::<CaptureGroupIntent>(json!({
            "capture_group_id": workvcs_core::CaptureGroupId::new_v7(),
            "primary_project_ref": project_ref_id,
            "primary_locator_evidence_digest": null,
            "canonical_record_local_id": "finding",
            "members": [
                {
                    "project_ref_id": project_ref_id,
                    "role": "primary",
                    "relation": "canonical_owner",
                    "delivery_mode": "canonical"
                },
                {
                    "project_ref_id": secondary,
                    "role": "related",
                    "relation": "related_context",
                    "delivery_mode": "immutable_reference"
                }
            ]
        }))
        .unwrap()
    });
    let capture_id = CaptureId::new_v7();
    let intent = CaptureIntent::new(
        capture_id,
        format!("round-3-primary-{capture_id}"),
        UtcTimestamp::parse(NOW).unwrap(),
        "Preserve the verified primary delivery result",
        CapturePayloadKind::CognitionV2,
        json!({
            "records": [{
                "local_id": "finding",
                "kind": "finding",
                "statement": "A Store commit without a journal receipt is recoverable",
                "scope": {"round": 3}
            }],
            "knowledge": [{
                "local_id": "knowledge",
                "statement": "Replay the same target idempotency identity",
                "scope": {"round": 3},
                "provenance": {"record": "finding"}
            }],
            "evidence": [{
                "local_id": "evidence",
                "kind": "fixture-probe",
                "metadata": {"fault_window": "commit_before_receipt"}
            }],
            "relations": [{
                "local_id": "supports",
                "type": "supports",
                "source_local_id": "finding",
                "target_local_id": "knowledge",
                "rationale": "The fixture proves the recovery rule"
            }],
            "rationale": {"round": 3}
        }),
        context,
        resolution.clone(),
        capture_group,
    )
    .unwrap();
    let journal =
        CaptureJournal::for_standalone_root(tempdir.path().join("capture-journal/v1")).unwrap();
    journal.admit(&intent).unwrap();
    journal
        .append_event_authority_only(
            capture_id,
            UtcTimestamp::parse(NOW).unwrap(),
            CaptureEventPayload::ResolutionRecorded(
                ResolutionRecordedPayload::new(
                    RegistryId::new_v7(),
                    1,
                    digest("round-3 registry"),
                    resolution,
                )
                .unwrap(),
            ),
        )
        .unwrap();
    journal
        .append_event_authority_only(
            capture_id,
            UtcTimestamp::parse(NOW).unwrap(),
            CaptureEventPayload::ProjectBindingReady(
                ProjectBindingReadyPayload::new(
                    RegistryId::new_v7(),
                    1,
                    digest("round-3 registry"),
                    project_ref_id,
                    ProjectMaturity::Established,
                    workvcs_core::ProjectLocatorId::new_v7(),
                    evidence,
                    CanonicalPath::parse(store_path.display().to_string()).unwrap(),
                    store.store_id,
                    workspace.workspace_id,
                    workspace.initial_branch_id,
                )
                .unwrap(),
            ),
        )
        .unwrap();
    DeliveryFixture {
        _tempdir: tempdir,
        store_path,
        engine,
        store,
        workspace,
        project_ref_id,
        intent,
        journal,
    }
}

fn assert_commit_before_receipt_recovery(with_secondary_reference: bool) {
    let mut fixture = fixture(with_secondary_reference);
    let capture_id = fixture.intent.capture_id();
    let initial = fixture.journal.inspect_projection(capture_id).unwrap();
    assert_eq!(
        initial.projection().recovery_state(),
        CaptureRecoveryState::PendingPrimary
    );

    let head = fixture
        .engine
        .branch_head(fixture.workspace.initial_branch_id)
        .unwrap();
    let prepared = prepare_primary_delivery(
        &fixture.intent,
        fixture.project_ref_id,
        fixture.store.store_id,
        fixture.workspace.workspace_id,
        fixture.workspace.initial_branch_id,
        head.head_commit_id,
        head.state_digest,
        None,
    )
    .unwrap();
    let started = prepared.started().clone();
    assert_eq!(
        started.target_idempotency_key(),
        format!(
            "projectref-primary:{capture_id}:{}:{}:{}:{}",
            fixture.project_ref_id,
            fixture.store.store_id,
            fixture.workspace.workspace_id,
            fixture.workspace.initial_branch_id
        )
    );
    fixture
        .journal
        .append_event_authority_only(
            capture_id,
            UtcTimestamp::parse(NOW).unwrap(),
            CaptureEventPayload::DeliveryStarted(started.clone()),
        )
        .unwrap();
    let created = fixture
        .engine
        .capture_cognition(CognitionCaptureOptions::new(
            fixture.workspace.initial_branch_id,
            started.expected_head_commit_id(),
            started.expected_state_digest(),
            prepared.into_manifest().expect("cognition manifest"),
        ))
        .unwrap();
    assert_eq!(created.outcome, CognitionCaptureOutcome::Created);

    let no_receipt = fixture.journal.inspect_projection(capture_id).unwrap();
    assert_eq!(
        no_receipt.projection().recovery_state(),
        CaptureRecoveryState::PendingPrimary
    );
    assert!(no_receipt.projection().primary_delivery().is_none());

    let current = fixture
        .engine
        .branch_head(fixture.workspace.initial_branch_id)
        .unwrap();
    let replay = prepare_primary_delivery(
        &fixture.intent,
        fixture.project_ref_id,
        fixture.store.store_id,
        fixture.workspace.workspace_id,
        fixture.workspace.initial_branch_id,
        current.head_commit_id,
        current.state_digest,
        no_receipt.projection().delivery_started(),
    )
    .unwrap();
    assert_eq!(replay.started(), &started);
    let reused = fixture
        .engine
        .capture_cognition(CognitionCaptureOptions::new(
            fixture.workspace.initial_branch_id,
            started.expected_head_commit_id(),
            started.expected_state_digest(),
            replay.into_manifest().expect("cognition manifest"),
        ))
        .unwrap();
    assert_eq!(reused.outcome, CognitionCaptureOutcome::Reused);
    assert_eq!(reused.commit_id, created.commit_id);

    let receipt = build_primary_delivery_receipt(&fixture.intent, &started, &reused).unwrap();
    assert!(receipt.reused());
    assert_eq!(receipt.result_objects().len(), 4);
    assert_eq!(
        receipt.canonical_record_ref().is_some(),
        with_secondary_reference
    );
    let applied = fixture
        .journal
        .append_event_authority_only(
            capture_id,
            UtcTimestamp::parse(NOW).unwrap(),
            CaptureEventPayload::DeliveryApplied(receipt.clone()),
        )
        .unwrap();
    assert_eq!(applied.outcome(), CaptureEventAppendOutcome::Created);
    let expected_state = if with_secondary_reference {
        CaptureRecoveryState::PendingReferences
    } else {
        CaptureRecoveryState::Completed
    };
    assert_eq!(applied.projection().recovery_state(), expected_state);
    assert_eq!(
        applied.projection().requires_secondary_references(),
        with_secondary_reference
    );

    let projection = fixture.journal.rebuild_projection(capture_id).unwrap();
    let expected_bytes = projection.projection().stored_json_bytes().unwrap();
    assert_eq!(
        fs::read(projection.projection_path()).unwrap(),
        expected_bytes
    );
    fs::remove_file(projection.projection_path()).unwrap();
    let rebuilt = fixture.journal.rebuild_projection(capture_id).unwrap();
    assert_eq!(
        rebuilt.projection().stored_json_bytes().unwrap(),
        expected_bytes
    );
    let replayed_event = fixture
        .journal
        .append_event_authority_only(
            capture_id,
            UtcTimestamp::parse(NOW).unwrap(),
            CaptureEventPayload::DeliveryApplied(receipt),
        )
        .unwrap();
    assert_eq!(replayed_event.outcome(), CaptureEventAppendOutcome::Reused);
    assert_eq!(replayed_event.projection().event_count(), 4);

    let history = fixture
        .engine
        .history(HistoryQueryOptions::from_branch(
            fixture.workspace.initial_branch_id,
        ))
        .unwrap();
    assert_eq!(history.entries.len(), 2, "target commit must not duplicate");
    assert_eq!(
        fs::canonicalize(&fixture.store_path).unwrap(),
        fixture.store_path
    );
}

#[test]
fn commit_before_receipt_recovery_reuses_one_primary_result() {
    assert_commit_before_receipt_recovery(false);
}

#[test]
fn primary_receipt_exposes_canonical_record_and_waits_for_references() {
    assert_commit_before_receipt_recovery(true);
}

fn deliver_primary_once(fixture: &mut DeliveryFixture) -> DeliveryAppliedPayload {
    let capture_id = fixture.intent.capture_id();
    let head = fixture
        .engine
        .branch_head(fixture.workspace.initial_branch_id)
        .unwrap();
    let prepared = prepare_primary_delivery(
        &fixture.intent,
        fixture.project_ref_id,
        fixture.store.store_id,
        fixture.workspace.workspace_id,
        fixture.workspace.initial_branch_id,
        head.head_commit_id,
        head.state_digest,
        None,
    )
    .unwrap();
    let started = prepared.started().clone();
    fixture
        .journal
        .append_event_authority_only(
            capture_id,
            UtcTimestamp::parse(NOW).unwrap(),
            CaptureEventPayload::DeliveryStarted(started.clone()),
        )
        .unwrap();
    let result = fixture
        .engine
        .capture_cognition(CognitionCaptureOptions::new(
            fixture.workspace.initial_branch_id,
            started.expected_head_commit_id(),
            started.expected_state_digest(),
            prepared.into_manifest().expect("cognition manifest"),
        ))
        .unwrap();
    let receipt = build_primary_delivery_receipt(&fixture.intent, &started, &result).unwrap();
    fixture
        .journal
        .append_event_authority_only(
            capture_id,
            UtcTimestamp::parse(NOW).unwrap(),
            CaptureEventPayload::DeliveryApplied(receipt.clone()),
        )
        .unwrap();
    receipt
}

#[test]
fn registry_revalidation_preserves_receipt_only_for_the_exact_target() {
    let mut fixture = fixture(false);
    let capture_id = fixture.intent.capture_id();
    let receipt = deliver_primary_once(&mut fixture);
    let evidence = locator_evidence(fixture.project_ref_id);
    let resolution = resolved(fixture.project_ref_id, &evidence);
    let registry_id = RegistryId::new_v7();
    let revision_two_digest = digest("round-3 registry revision 2");

    let resolution_refresh = fixture
        .journal
        .append_event_authority_only(
            capture_id,
            UtcTimestamp::parse(NOW).unwrap(),
            CaptureEventPayload::ResolutionRecorded(
                ResolutionRecordedPayload::new(
                    registry_id,
                    2,
                    revision_two_digest.clone(),
                    resolution.clone(),
                )
                .unwrap(),
            ),
        )
        .unwrap();
    assert_eq!(
        resolution_refresh.projection().recovery_state(),
        CaptureRecoveryState::PendingProject
    );
    assert!(
        resolution_refresh
            .projection()
            .project_binding_ready()
            .is_none()
    );
    assert_eq!(
        resolution_refresh.projection().primary_delivery(),
        Some(&receipt)
    );

    let same_target = fixture
        .journal
        .append_event_authority_only(
            capture_id,
            UtcTimestamp::parse(NOW).unwrap(),
            CaptureEventPayload::ProjectBindingReady(
                ProjectBindingReadyPayload::new(
                    registry_id,
                    2,
                    revision_two_digest,
                    fixture.project_ref_id,
                    ProjectMaturity::Established,
                    workvcs_core::ProjectLocatorId::new_v7(),
                    evidence.clone(),
                    CanonicalPath::parse(fixture.store_path.display().to_string()).unwrap(),
                    fixture.store.store_id,
                    fixture.workspace.workspace_id,
                    fixture.workspace.initial_branch_id,
                )
                .unwrap(),
            ),
        )
        .unwrap();
    assert_eq!(
        same_target.projection().recovery_state(),
        CaptureRecoveryState::Completed
    );
    assert_eq!(same_target.projection().primary_delivery(), Some(&receipt));
    assert_eq!(
        fixture
            .engine
            .history(HistoryQueryOptions::from_branch(
                fixture.workspace.initial_branch_id,
            ))
            .unwrap()
            .entries
            .len(),
        2,
        "same-target revalidation must not redeliver"
    );

    let revision_three_digest = digest("round-3 registry revision 3");
    fixture
        .journal
        .append_event_authority_only(
            capture_id,
            UtcTimestamp::parse(NOW).unwrap(),
            CaptureEventPayload::ResolutionRecorded(
                ResolutionRecordedPayload::new(
                    registry_id,
                    3,
                    revision_three_digest.clone(),
                    resolution,
                )
                .unwrap(),
            ),
        )
        .unwrap();
    let alternate_path = fixture._tempdir.path().join("alternate.sqlite");
    let mut alternate_engine = Engine::init(
        &alternate_path,
        StoreInitOptions::new("round-3-alternate").unwrap(),
    )
    .unwrap();
    let alternate_workspace = alternate_engine
        .create_workspace(WorkspaceInitOptions::new("alternate").unwrap())
        .unwrap();
    let alternate_store = alternate_engine.store_info().unwrap();
    let changed_target = fixture
        .journal
        .append_event_authority_only(
            capture_id,
            UtcTimestamp::parse(NOW).unwrap(),
            CaptureEventPayload::ProjectBindingReady(
                ProjectBindingReadyPayload::new(
                    registry_id,
                    3,
                    revision_three_digest,
                    fixture.project_ref_id,
                    ProjectMaturity::Established,
                    workvcs_core::ProjectLocatorId::new_v7(),
                    evidence,
                    CanonicalPath::parse(
                        fs::canonicalize(&alternate_path)
                            .unwrap()
                            .display()
                            .to_string(),
                    )
                    .unwrap(),
                    alternate_store.store_id,
                    alternate_workspace.workspace_id,
                    alternate_workspace.initial_branch_id,
                )
                .unwrap(),
            ),
        )
        .unwrap();
    assert_eq!(
        changed_target.projection().recovery_state(),
        CaptureRecoveryState::PendingPrimary
    );
    assert!(changed_target.projection().delivery_started().is_none());
    assert!(changed_target.projection().primary_delivery().is_none());
    assert_eq!(
        alternate_engine
            .history(HistoryQueryOptions::from_branch(
                alternate_workspace.initial_branch_id,
            ))
            .unwrap()
            .entries
            .len(),
        1,
        "rebinding must not write until an explicit delivery is attempted"
    );
}

#[test]
fn invalid_delivery_transition_is_rejected_before_event_install() {
    let mut fixture = fixture(false);
    deliver_primary_once(&mut fixture);
    let capture_id = fixture.intent.capture_id();
    let before = fixture.journal.inspect_projection(capture_id).unwrap();
    assert_eq!(before.projection().event_count(), 4);
    let head = fixture
        .engine
        .branch_head(fixture.workspace.initial_branch_id)
        .unwrap();
    let conflicting_start = DeliveryStartedPayload::new(
        DeliveryId::new_v7(),
        DeliveryMode::Canonical,
        fixture.project_ref_id,
        fixture.store.store_id,
        fixture.workspace.workspace_id,
        fixture.workspace.initial_branch_id,
        head.head_commit_id,
        head.state_digest,
        "round-3-conflicting-start",
        workvcs_core::Digest::raw(b"conflicting manifest"),
    )
    .unwrap();
    assert!(
        fixture
            .journal
            .append_event_authority_only(
                capture_id,
                UtcTimestamp::parse(NOW).unwrap(),
                CaptureEventPayload::DeliveryStarted(conflicting_start),
            )
            .is_err()
    );
    let after = fixture.journal.inspect_projection(capture_id).unwrap();
    assert_eq!(after.projection().event_count(), 4);
    assert_eq!(
        after.projection().recovery_state(),
        CaptureRecoveryState::Completed
    );
}

#[test]
fn capture_group_canonical_record_is_preflighted_before_target_write() {
    let fixture = fixture(true);
    let mut group_value = serde_json::to_value(fixture.intent.capture_group().unwrap()).unwrap();
    group_value
        .as_object_mut()
        .unwrap()
        .insert("canonical_record_local_id".to_owned(), json!("knowledge"));
    let invalid_group = serde_json::from_value::<CaptureGroupIntent>(group_value).unwrap();
    let invalid_intent = CaptureIntent::new(
        CaptureId::new_v7(),
        "round-3-invalid-canonical-record",
        UtcTimestamp::parse(NOW).unwrap(),
        "Reject a non-Record canonical target before target mutation",
        fixture.intent.payload_kind(),
        fixture.intent.semantic_payload().clone(),
        fixture.intent.resolution_context().clone(),
        fixture.intent.initial_resolution().clone(),
        Some(invalid_group),
    )
    .unwrap();
    let head = fixture
        .engine
        .branch_head(fixture.workspace.initial_branch_id)
        .unwrap();
    assert!(
        prepare_primary_delivery(
            &invalid_intent,
            fixture.project_ref_id,
            fixture.store.store_id,
            fixture.workspace.workspace_id,
            fixture.workspace.initial_branch_id,
            head.head_commit_id,
            head.state_digest,
            None,
        )
        .is_err()
    );
    assert_eq!(
        fixture
            .engine
            .history(HistoryQueryOptions::from_branch(
                fixture.workspace.initial_branch_id,
            ))
            .unwrap()
            .entries
            .len(),
        1
    );
}

#[test]
fn stale_legacy_manifest_is_detected_without_rewriting_the_intent() {
    let fixture = fixture(false);
    let stale_payload = json!({
        "schema_version": 1,
        "idempotency_key": "legacy-stale-core",
        "expected_head_commit_id": workvcs_core::CommitId::new_v7(),
        "expected_state_digest": workvcs_core::Digest::raw(b"stale state"),
        "records": [{
            "local_id": "finding",
            "kind": "finding",
            "statement": "Preserve this exact manifest"
        }],
        "knowledge": [],
        "evidence": [],
        "relations": [],
        "rationale": {}
    });
    let legacy = CaptureIntent::new(
        CaptureId::new_v7(),
        "legacy-stale-intent",
        UtcTimestamp::parse(NOW).unwrap(),
        "Require an explicit legacy upgrade",
        CapturePayloadKind::LegacyCognitionV1,
        stale_payload,
        fixture.intent.resolution_context().clone(),
        fixture.intent.initial_resolution().clone(),
        None,
    )
    .unwrap();
    let before = legacy.canonical_json_bytes().unwrap();
    let prepared = prepare_primary_delivery(
        &legacy,
        fixture.project_ref_id,
        fixture.store.store_id,
        fixture.workspace.workspace_id,
        fixture.workspace.initial_branch_id,
        fixture.workspace.genesis_commit_id,
        fixture.workspace.state_digest,
        None,
    )
    .unwrap();
    assert!(prepared.legacy_manifest_upgrade_required());
    assert_eq!(legacy.canonical_json_bytes().unwrap(), before);
    assert_eq!(
        fixture
            .engine
            .history(HistoryQueryOptions::from_branch(
                fixture.workspace.initial_branch_id,
            ))
            .unwrap()
            .entries
            .len(),
        1
    );
}
