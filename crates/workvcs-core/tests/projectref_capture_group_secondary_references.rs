use serde_json::json;
use std::fs;
use tempfile::TempDir;
use workvcs_core::control_plane::{
    CanonicalPath, CaptureCompletedPayload, CaptureEventAppendOutcome, CaptureEventPayload,
    CaptureGroupIntent, CaptureGroupResolvedPayload, CaptureIntent, CaptureJournal,
    CapturePayloadKind, CaptureRecoveryState, ControlPlaneDigest, DeliveryAppliedPayload,
    LocatorAssurance, LocatorAuthority, LocatorEvidence, ProjectBindingReadyPayload,
    ProjectMaturity, ReferenceAppliedPayload, ResolutionContext, ResolutionMode,
    ResolutionRecordedPayload, ResolutionResult, StoredProjectionState, UtcTimestamp,
    build_primary_delivery_receipt, prepare_primary_delivery,
};
use workvcs_core::{
    CaptureGroupId, CaptureId, CognitionCaptureOptions, Engine, EntityId, EntityVersionId,
    FindingRecordCorrectionOptions, ProjectRefId, RecordCreateOptions, RegistryId, StoreInfo,
    StoreInitOptions, WorkspaceInfo, WorkspaceInitOptions,
};

const NOW: &str = "2026-09-27T12:00:00Z";

struct GroupFixture {
    _tempdir: TempDir,
    engine: Engine,
    store: StoreInfo,
    workspace: WorkspaceInfo,
    primary: ProjectRefId,
    secondaries: Vec<ProjectRefId>,
    intent: CaptureIntent,
    journal: CaptureJournal,
}

fn digest(label: &str) -> ControlPlaneDigest {
    ControlPlaneDigest::raw(label.as_bytes())
}

fn evidence(label: &str) -> LocatorEvidence {
    LocatorEvidence::new(
        LocatorAuthority::SemanticProject,
        "fixture-tool",
        "isolated:round-4",
        "project_id",
        label,
        LocatorAssurance::Authoritative,
        "round-4-fixture/v1",
        digest(label),
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

fn fixture(secondary_count: usize) -> GroupFixture {
    let tempdir = tempfile::tempdir().unwrap();
    let store_path = tempdir.path().join("primary.sqlite");
    let mut engine = Engine::init(
        &store_path,
        StoreInitOptions::new("round-4-primary").unwrap(),
    )
    .unwrap();
    let workspace = engine
        .create_workspace(WorkspaceInitOptions::new("primary").unwrap())
        .unwrap();
    let store = engine.store_info().unwrap();
    let canonical_store = fs::canonicalize(&store_path).unwrap();
    let primary = ProjectRefId::new_v7();
    let owner_evidence = evidence("round-4-primary");
    let resolution = resolved(primary, &owner_evidence);
    let context = ResolutionContext::new(ResolutionMode::DurableWrite)
        .with_project_ref(primary)
        .with_locator_evidence(owner_evidence.clone());
    let mut secondaries = (0..secondary_count)
        .map(|_| ProjectRefId::new_v7())
        .collect::<Vec<_>>();
    secondaries.sort();
    let mut members = vec![json!({
        "project_ref_id": primary,
        "role": "primary",
        "relation": "canonical_owner",
        "delivery_mode": "canonical"
    })];
    members.extend(secondaries.iter().map(|secondary| {
        json!({
            "project_ref_id": secondary,
            "role": "related",
            "relation": "related_context",
            "delivery_mode": "immutable_reference"
        })
    }));
    let capture_group = serde_json::from_value::<CaptureGroupIntent>(json!({
        "capture_group_id": CaptureGroupId::new_v7(),
        "primary_project_ref": primary,
        "primary_locator_evidence_digest": null,
        "canonical_record_local_id": "finding",
        "members": members
    }))
    .unwrap();
    let capture_id = CaptureId::new_v7();
    let intent = CaptureIntent::new(
        capture_id,
        format!("round-4-group-{capture_id}"),
        UtcTimestamp::parse(NOW).unwrap(),
        "Make one canonical Record discoverable from secondary ProjectRefs",
        CapturePayloadKind::CognitionV2,
        json!({
            "records": [{
                "local_id": "finding",
                "kind": "finding",
                "statement": "One mutable authority has immutable cross-project references"
            }],
            "knowledge": [],
            "evidence": [],
            "relations": [],
            "rationale": {"round": 4}
        }),
        context,
        resolution.clone(),
        Some(capture_group),
    )
    .unwrap();
    let journal =
        CaptureJournal::for_standalone_root(tempdir.path().join("capture-journal/v1")).unwrap();
    journal.admit(&intent).unwrap();
    let registry_id = RegistryId::new_v7();
    journal
        .append_event_authority_only(
            capture_id,
            UtcTimestamp::parse(NOW).unwrap(),
            CaptureEventPayload::ResolutionRecorded(
                ResolutionRecordedPayload::new(
                    registry_id,
                    1,
                    digest("round-4-registry"),
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
                    registry_id,
                    1,
                    digest("round-4-registry"),
                    primary,
                    ProjectMaturity::Established,
                    workvcs_core::ProjectLocatorId::new_v7(),
                    owner_evidence,
                    CanonicalPath::parse(canonical_store.display().to_string()).unwrap(),
                    store.store_id,
                    workspace.workspace_id,
                    workspace.initial_branch_id,
                )
                .unwrap(),
            ),
        )
        .unwrap();
    GroupFixture {
        _tempdir: tempdir,
        engine,
        store,
        workspace,
        primary,
        secondaries,
        intent,
        journal,
    }
}

fn deliver_primary(fixture: &mut GroupFixture) -> DeliveryAppliedPayload {
    let head = fixture
        .engine
        .branch_head(fixture.workspace.initial_branch_id)
        .unwrap();
    let prepared = prepare_primary_delivery(
        &fixture.intent,
        fixture.primary,
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
            fixture.intent.capture_id(),
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
            fixture.intent.capture_id(),
            UtcTimestamp::parse(NOW).unwrap(),
            CaptureEventPayload::DeliveryApplied(receipt.clone()),
        )
        .unwrap();
    receipt
}

#[test]
fn pending_references_retry_only_missing_and_secondary_recall_is_authoritative() {
    let mut fixture = fixture(2);
    let receipt = deliver_primary(&mut fixture);
    let capture_id = fixture.intent.capture_id();
    let canonical = receipt.canonical_record_ref().unwrap().clone();
    let group_id = fixture
        .journal
        .inspect_projection(capture_id)
        .unwrap()
        .projection()
        .capture_group()
        .unwrap()
        .capture_group_id();

    let pending = fixture.journal.inspect_projection(capture_id).unwrap();
    assert_eq!(
        pending.projection().recovery_state(),
        CaptureRecoveryState::PendingReferences
    );
    assert_eq!(
        pending
            .projection()
            .capture_group()
            .unwrap()
            .pending_reference_project_refs(),
        fixture.secondaries
    );

    let first_payload = ReferenceAppliedPayload::new(
        group_id,
        fixture.secondaries[0],
        canonical.clone(),
        "related_context",
    )
    .unwrap();
    fixture
        .journal
        .append_event_authority_only(
            capture_id,
            UtcTimestamp::parse(NOW).unwrap(),
            CaptureEventPayload::ReferenceApplied(first_payload.clone()),
        )
        .unwrap();
    let replay = fixture
        .journal
        .append_event_authority_only(
            capture_id,
            UtcTimestamp::parse("2026-09-27T12:01:00Z").unwrap(),
            CaptureEventPayload::ReferenceApplied(first_payload),
        )
        .unwrap();
    assert_eq!(replay.outcome(), CaptureEventAppendOutcome::Reused);
    assert_eq!(replay.projection().event_count(), 5);
    assert_eq!(
        replay
            .projection()
            .capture_group()
            .unwrap()
            .pending_reference_project_refs(),
        vec![fixture.secondaries[1]]
    );

    let first_recall = fixture
        .journal
        .recall_secondary_project(fixture.secondaries[0])
        .unwrap();
    assert_eq!(first_recall.len(), 1);
    assert_eq!(first_recall[0].canonical_record_ref(), &canonical);
    assert!(!first_recall[0].capture_completed());
    assert!(
        fixture
            .journal
            .recall_secondary_project(fixture.secondaries[1])
            .unwrap()
            .is_empty()
    );

    fixture
        .journal
        .append_event_authority_only(
            capture_id,
            UtcTimestamp::parse("2026-09-27T12:02:00Z").unwrap(),
            CaptureEventPayload::ReferenceApplied(
                ReferenceAppliedPayload::new(
                    group_id,
                    fixture.secondaries[1],
                    canonical.clone(),
                    "related_context",
                )
                .unwrap(),
            ),
        )
        .unwrap();
    let all_referenced = fixture.journal.inspect_projection(capture_id).unwrap();
    assert_eq!(
        all_referenced.projection().recovery_state(),
        CaptureRecoveryState::Completed
    );
    assert!(
        all_referenced
            .projection()
            .capture_group()
            .unwrap()
            .completion_receipt()
            .is_none()
    );

    fixture
        .journal
        .append_event_authority_only(
            capture_id,
            UtcTimestamp::parse("2026-09-27T12:03:00Z").unwrap(),
            CaptureEventPayload::CaptureCompleted(
                CaptureCompletedPayload::new(
                    group_id,
                    canonical.clone(),
                    fixture.secondaries.clone(),
                )
                .unwrap(),
            ),
        )
        .unwrap();
    let rebuilt = fixture.journal.rebuild_projection(capture_id).unwrap();
    assert_eq!(rebuilt.projection().event_count(), 7);
    assert_eq!(
        rebuilt.projection().recovery_state(),
        CaptureRecoveryState::Completed
    );

    let head = fixture
        .engine
        .branch_head(fixture.workspace.initial_branch_id)
        .unwrap();
    let replacement = fixture
        .engine
        .create_record(
            RecordCreateOptions::finding(
                fixture.workspace.initial_branch_id,
                head.head_commit_id,
                "The canonical conclusion now has a later project-local correction",
            )
            .unwrap(),
        )
        .unwrap();
    fixture
        .engine
        .correct_finding_record(
            FindingRecordCorrectionOptions::supersede(
                fixture.workspace.initial_branch_id,
                replacement.commit_id,
                replacement.record_entity_id,
                EntityId::parse_canonical(canonical.record_id()).unwrap(),
                EntityVersionId::parse_canonical(canonical.record_version_id()).unwrap(),
                "Later canonical evolution must not rewrite an existing external reference",
            )
            .unwrap(),
        )
        .unwrap();
    fs::remove_file(fixture.journal.projection_path(capture_id)).unwrap();
    let authority_only = fixture.journal.inspect_projection(capture_id).unwrap();
    assert_eq!(authority_only.stored_state(), StoredProjectionState::Absent);
    assert_eq!(authority_only.projection(), rebuilt.projection());
    for secondary in &fixture.secondaries {
        let recalled = fixture
            .journal
            .recall_secondary_project(*secondary)
            .unwrap();
        assert_eq!(recalled.len(), 1);
        assert_eq!(recalled[0].canonical_record_ref(), &canonical);
        assert!(recalled[0].capture_completed());
    }
}

#[test]
fn invalid_secondary_transition_fails_before_event_install() {
    let mut fixture = fixture(1);
    let receipt = deliver_primary(&mut fixture);
    let capture_id = fixture.intent.capture_id();
    let before = fixture.journal.inspect_projection(capture_id).unwrap();
    let group_id = before
        .projection()
        .capture_group()
        .unwrap()
        .capture_group_id();
    let error = fixture
        .journal
        .append_event_authority_only(
            capture_id,
            UtcTimestamp::parse(NOW).unwrap(),
            CaptureEventPayload::ReferenceApplied(
                ReferenceAppliedPayload::new(
                    group_id,
                    ProjectRefId::new_v7(),
                    receipt.canonical_record_ref().unwrap().clone(),
                    "related_context",
                )
                .unwrap(),
            ),
        )
        .expect_err("a non-member secondary reference must fail");
    assert!(error.to_string().contains("outside the CaptureGroup"));
    let after = fixture.journal.inspect_projection(capture_id).unwrap();
    assert_eq!(
        after.projection().event_count(),
        before.projection().event_count()
    );
    assert_eq!(
        after.projection().last_event_digest(),
        before.projection().last_event_digest()
    );

    let changed_owner_evidence = evidence("round-4-changed-owner");
    let changed_owner = ProjectRefId::new_v7();
    let error = fixture
        .journal
        .append_event_authority_only(
            capture_id,
            UtcTimestamp::parse("2026-09-27T12:01:00Z").unwrap(),
            CaptureEventPayload::ResolutionRecorded(
                ResolutionRecordedPayload::new(
                    RegistryId::new_v7(),
                    2,
                    digest("round-4-retarget-registry"),
                    resolved(changed_owner, &changed_owner_evidence),
                )
                .unwrap(),
            ),
        )
        .expect_err("a canonical CaptureGroup must not be retargeted");
    assert!(error.to_string().contains("cannot supersede"));
    let final_projection = fixture.journal.inspect_projection(capture_id).unwrap();
    assert_eq!(
        final_projection.projection().event_count(),
        before.projection().event_count()
    );
}

#[test]
fn unresolved_group_adds_one_canonical_member_from_matching_locator_evidence() {
    let tempdir = tempfile::tempdir().unwrap();
    let primary = ProjectRefId::new_v7();
    let owner_evidence = evidence("round-4-unbound-primary");
    let context = ResolutionContext::new(ResolutionMode::DurableWrite)
        .with_locator_evidence(owner_evidence.clone());
    let unbound: ResolutionResult = serde_json::from_value(json!({
        "status": "unbound",
        "primary_project_ref": null,
        "primary_basis": {
            "kind": "locator",
            "rank": "semantic_project",
            "project_ref_id": null,
            "evidence": owner_evidence
        },
        "related_project_refs": [],
        "unmapped_locators": [owner_evidence],
        "diagnostics": []
    }))
    .unwrap();
    let group_id = CaptureGroupId::new_v7();
    let group = serde_json::from_value::<CaptureGroupIntent>(json!({
        "capture_group_id": group_id,
        "primary_project_ref": null,
        "primary_locator_evidence_digest": owner_evidence.evidence_digest(),
        "canonical_record_local_id": "finding",
        "members": []
    }))
    .unwrap();
    let capture_id = CaptureId::new_v7();
    let intent = CaptureIntent::new(
        capture_id,
        "round-4-unbound-group",
        UtcTimestamp::parse(NOW).unwrap(),
        "Resolve one canonical owner without rewriting the intent",
        CapturePayloadKind::CognitionV2,
        json!({
            "records": [{
                "local_id": "finding",
                "kind": "finding",
                "statement": "The group primary is event-derived"
            }]
        }),
        context,
        unbound,
        Some(group),
    )
    .unwrap();
    let journal =
        CaptureJournal::for_standalone_root(tempdir.path().join("capture-journal/v1")).unwrap();
    journal.admit(&intent).unwrap();
    let resolved = resolved(primary, &owner_evidence);
    journal
        .append_event_authority_only(
            capture_id,
            UtcTimestamp::parse(NOW).unwrap(),
            CaptureEventPayload::ResolutionRecorded(
                ResolutionRecordedPayload::new(
                    RegistryId::new_v7(),
                    2,
                    digest("round-4-resolved-registry"),
                    resolved,
                )
                .unwrap(),
            ),
        )
        .unwrap();
    let payload = CaptureGroupResolvedPayload::new(
        group_id,
        primary,
        owner_evidence.evidence_digest().clone(),
        "canonical_owner",
    )
    .unwrap();
    let applied = journal
        .append_event_authority_only(
            capture_id,
            UtcTimestamp::parse(NOW).unwrap(),
            CaptureEventPayload::CaptureGroupResolved(payload.clone()),
        )
        .unwrap();
    let group = applied.projection().capture_group().unwrap();
    assert_eq!(group.resolved_primary_project_ref(), Some(primary));
    assert_eq!(group.members().len(), 1);
    assert_eq!(group.members()[0].project_ref_id(), primary);
    let replay = journal
        .append_event_authority_only(
            capture_id,
            UtcTimestamp::parse("2026-09-27T12:04:00Z").unwrap(),
            CaptureEventPayload::CaptureGroupResolved(payload),
        )
        .unwrap();
    assert_eq!(replay.outcome(), CaptureEventAppendOutcome::Reused);

    let wrong = CaptureGroupResolvedPayload::new(
        group_id,
        ProjectRefId::new_v7(),
        owner_evidence.evidence_digest().clone(),
        "canonical_owner",
    )
    .unwrap();
    let before_count = replay.projection().event_count();
    assert!(
        journal
            .append_event_authority_only(
                capture_id,
                UtcTimestamp::parse("2026-09-27T12:05:00Z").unwrap(),
                CaptureEventPayload::CaptureGroupResolved(wrong),
            )
            .is_err()
    );
    assert_eq!(
        journal
            .inspect_projection(capture_id)
            .unwrap()
            .projection()
            .event_count(),
        before_count
    );
}

#[test]
fn capture_group_rejects_a_second_primary_role_before_admission() {
    let primary = ProjectRefId::new_v7();
    let secondary = ProjectRefId::new_v7();
    let owner_evidence = evidence("round-4-primary-role");
    let resolution = resolved(primary, &owner_evidence);
    let context = ResolutionContext::new(ResolutionMode::DurableWrite)
        .with_project_ref(primary)
        .with_locator_evidence(owner_evidence);
    let group = serde_json::from_value::<CaptureGroupIntent>(json!({
        "capture_group_id": CaptureGroupId::new_v7(),
        "primary_project_ref": primary,
        "primary_locator_evidence_digest": null,
        "canonical_record_local_id": "finding",
        "members": [
            {
                "project_ref_id": primary,
                "role": "primary",
                "relation": "canonical_owner",
                "delivery_mode": "canonical"
            },
            {
                "project_ref_id": secondary,
                "role": "primary",
                "relation": "related_context",
                "delivery_mode": "immutable_reference"
            }
        ]
    }))
    .unwrap();
    let error = CaptureIntent::new(
        CaptureId::new_v7(),
        "round-4-invalid-primary-role",
        UtcTimestamp::parse(NOW).unwrap(),
        "Reject two primary roles before durable admission",
        CapturePayloadKind::CognitionV2,
        json!({
            "records": [{
                "local_id": "finding",
                "kind": "finding",
                "statement": "Only one CaptureGroup member may be primary"
            }]
        }),
        context,
        resolution,
        Some(group),
    )
    .expect_err("a second primary role must fail validation");
    assert!(error.to_string().contains("exactly one primary role"));
}
