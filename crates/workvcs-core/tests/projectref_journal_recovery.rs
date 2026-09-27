use serde_json::{Value, json};
use std::fs;
use tempfile::tempdir;
use workvcs_core::canonical::{canonical_bytes, parse_canonical_json};
use workvcs_core::control_plane::{
    CanonicalPath, CaptureEventAppendOutcome, CaptureEventPayload, CaptureIntent, CaptureJournal,
    CapturePayloadKind, CaptureProjectionWriteOutcome, CaptureRecoveryState, ControlPlaneDigest,
    FirstWriteProjectBinding, LocatorAssurance, LocatorAuthority, LocatorEvidence,
    ProjectBindingReadyPayload, ProjectBootstrapOutcome, ProjectCreatedBy, ProjectMaturity,
    ProjectRegistryV2, ResolutionContext, ResolutionMode, ResolutionRecordedPayload,
    ResolutionResult, StoredProjectionState, UtcTimestamp, resolve_unbound_project,
};
use workvcs_core::{
    BranchId, CaptureId, ProjectLocatorId, ProjectRefId, RegistryId, StoreId, WorkspaceId,
};

const NOW: &str = "2026-09-27T08:00:00Z";

fn digest(label: &str) -> ControlPlaneDigest {
    ControlPlaneDigest::raw(label.as_bytes())
}

fn semantic_evidence() -> LocatorEvidence {
    LocatorEvidence::new(
        LocatorAuthority::SemanticProject,
        "chatgpt",
        "account-a",
        "project_id",
        "g-p-round-2",
        LocatorAssurance::Authoritative,
        "fixture-adapter/v1",
        digest("semantic-owner"),
    )
    .expect("semantic evidence")
}

fn capture_intent(capture_id: CaptureId) -> CaptureIntent {
    let context = ResolutionContext::new(ResolutionMode::DurableWrite)
        .with_locator_evidence(semantic_evidence());
    let resolution =
        resolve_unbound_project(&context, "registry:fixture").expect("unbound fixture resolution");
    CaptureIntent::new(
        capture_id,
        "round-2-journal-recovery",
        UtcTimestamp::parse(NOW).unwrap(),
        "Preserve the accepted recovery contract",
        CapturePayloadKind::CognitionV2,
        json!({"records": [{"local_id": "finding", "statement": "recoverable"}]}),
        context,
        resolution,
        None,
    )
    .expect("capture intent")
}

fn resolved_result(project_ref_id: ProjectRefId) -> ResolutionResult {
    serde_json::from_value(json!({
        "status": "resolved",
        "primary_project_ref": project_ref_id,
        "primary_basis": {
            "kind": "locator",
            "rank": "semantic_project",
            "project_ref_id": project_ref_id,
            "evidence": semantic_evidence()
        },
        "related_project_refs": [],
        "unmapped_locators": [],
        "diagnostics": []
    }))
    .expect("resolved result shape")
}

fn event_paths(journal: &CaptureJournal, capture_id: CaptureId) -> Vec<std::path::PathBuf> {
    let mut paths = fs::read_dir(journal.root().join("events").join(capture_id.to_string()))
        .expect("event directory")
        .map(|entry| entry.expect("event entry").path())
        .collect::<Vec<_>>();
    paths.sort();
    paths
}

#[test]
fn immutable_events_rebuild_a_byte_equivalent_projection_and_replay_without_duplicates() {
    let temp = tempdir().unwrap();
    let journal =
        CaptureJournal::for_standalone_root(temp.path().join("capture-journal/v1")).unwrap();
    let capture_id = CaptureId::new_v7();
    let intent = capture_intent(capture_id);
    journal.admit(&intent).expect("admit intent");

    let initial = journal.inspect_projection(capture_id).unwrap();
    assert_eq!(initial.stored_state(), StoredProjectionState::Absent);
    assert_eq!(
        initial.projection().recovery_state(),
        CaptureRecoveryState::PendingProject
    );
    assert_eq!(initial.projection().event_count(), 0);

    let registry_id = RegistryId::new_v7();
    let unbound = ResolutionRecordedPayload::new(
        registry_id,
        1,
        digest("registry-before-bootstrap"),
        intent.initial_resolution().clone(),
    )
    .unwrap();
    let first = journal
        .append_event_authority_only(
            capture_id,
            UtcTimestamp::parse(NOW).unwrap(),
            CaptureEventPayload::ResolutionRecorded(unbound.clone()),
        )
        .unwrap();
    assert_eq!(first.outcome(), CaptureEventAppendOutcome::Created);
    assert_eq!(first.event().sequence(), 1);
    assert_eq!(first.event().previous_event_digest(), None);
    assert_eq!(
        journal
            .inspect_projection(capture_id)
            .unwrap()
            .stored_state(),
        StoredProjectionState::Absent
    );

    let replay = journal
        .append_event_authority_only(
            capture_id,
            UtcTimestamp::parse(NOW).unwrap(),
            CaptureEventPayload::ResolutionRecorded(unbound),
        )
        .unwrap();
    assert_eq!(replay.outcome(), CaptureEventAppendOutcome::Reused);
    assert_eq!(event_paths(&journal, capture_id).len(), 1);

    let project_ref_id = ProjectRefId::new_v7();
    let resolved = resolved_result(project_ref_id);
    journal
        .append_event_authority_only(
            capture_id,
            UtcTimestamp::parse("2026-09-27T08:00:01Z").unwrap(),
            CaptureEventPayload::ResolutionRecorded(
                ResolutionRecordedPayload::new(
                    registry_id,
                    2,
                    digest("registry-after-bootstrap"),
                    resolved,
                )
                .unwrap(),
            ),
        )
        .unwrap();
    let ready = ProjectBindingReadyPayload::new(
        registry_id,
        2,
        digest("registry-after-bootstrap"),
        project_ref_id,
        ProjectMaturity::Established,
        ProjectLocatorId::new_v7(),
        semantic_evidence(),
        CanonicalPath::parse(temp.path().join("stores/project.sqlite").to_str().unwrap()).unwrap(),
        StoreId::new_v7(),
        WorkspaceId::new_v7(),
        BranchId::new_v7(),
    )
    .unwrap();
    journal
        .append_event_authority_only(
            capture_id,
            UtcTimestamp::parse("2026-09-27T08:00:02Z").unwrap(),
            CaptureEventPayload::ProjectBindingReady(ready.clone()),
        )
        .unwrap();

    let written = journal.rebuild_projection(capture_id).unwrap();
    assert_eq!(written.outcome(), CaptureProjectionWriteOutcome::Created);
    assert_eq!(
        written.projection().recovery_state(),
        CaptureRecoveryState::PendingPrimary
    );
    assert_eq!(written.projection().event_count(), 3);
    assert_eq!(
        written
            .projection()
            .project_binding_ready()
            .unwrap()
            .project_ref_id(),
        project_ref_id
    );
    let original_projection_bytes = fs::read(written.projection_path()).unwrap();

    let exact_rebuild = journal.rebuild_projection(capture_id).unwrap();
    assert_eq!(
        exact_rebuild.outcome(),
        CaptureProjectionWriteOutcome::Reused
    );
    assert_eq!(
        fs::read(exact_rebuild.projection_path()).unwrap(),
        original_projection_bytes
    );

    fs::write(exact_rebuild.projection_path(), b"{not-json\n").unwrap();
    let invalid = journal.inspect_projection(capture_id).unwrap();
    assert_eq!(invalid.stored_state(), StoredProjectionState::Invalid);
    assert!(invalid.stored_issue().is_some());
    let repaired = journal.rebuild_projection(capture_id).unwrap();
    assert_eq!(repaired.outcome(), CaptureProjectionWriteOutcome::Replaced);
    assert_eq!(
        fs::read(repaired.projection_path()).unwrap(),
        original_projection_bytes
    );

    fs::remove_file(repaired.projection_path()).unwrap();
    let recovered = journal.rebuild_projection(capture_id).unwrap();
    assert_eq!(recovered.outcome(), CaptureProjectionWriteOutcome::Created);
    assert_eq!(
        fs::read(recovered.projection_path()).unwrap(),
        original_projection_bytes
    );

    let binding_replay = journal
        .append_event_authority_only(
            capture_id,
            UtcTimestamp::parse("2026-09-27T08:00:03Z").unwrap(),
            CaptureEventPayload::ProjectBindingReady(ready),
        )
        .unwrap();
    assert_eq!(binding_replay.outcome(), CaptureEventAppendOutcome::Reused);
    assert_eq!(event_paths(&journal, capture_id).len(), 3);

    let degraded = journal
        .append_event_authority_only(
            capture_id,
            UtcTimestamp::parse("2026-09-27T08:00:04Z").unwrap(),
            CaptureEventPayload::ResolutionRecorded(
                ResolutionRecordedPayload::new(
                    registry_id,
                    3,
                    digest("registry-owner-no-longer-bound"),
                    intent.initial_resolution().clone(),
                )
                .unwrap(),
            ),
        )
        .unwrap();
    assert_eq!(degraded.outcome(), CaptureEventAppendOutcome::Created);
    assert_eq!(degraded.projection().event_count(), 4);
    assert_eq!(
        degraded.projection().recovery_state(),
        CaptureRecoveryState::PendingProject
    );
    assert!(degraded.projection().project_binding_ready().is_none());
    let stale = journal.inspect_projection(capture_id).unwrap();
    assert_eq!(stale.stored_state(), StoredProjectionState::Stale);
    let rebuilt_degraded = journal.rebuild_projection(capture_id).unwrap();
    assert_eq!(
        rebuilt_degraded.projection().recovery_state(),
        CaptureRecoveryState::PendingProject
    );
}

#[test]
fn event_gap_reordering_and_payload_tampering_fail_closed() {
    let temp = tempdir().unwrap();
    let journal =
        CaptureJournal::for_standalone_root(temp.path().join("capture-journal/v1")).unwrap();
    let capture_id = CaptureId::new_v7();
    let intent = capture_intent(capture_id);
    journal.admit(&intent).unwrap();
    let registry_id = RegistryId::new_v7();
    for revision in [1, 2] {
        journal
            .append_event_authority_only(
                capture_id,
                UtcTimestamp::parse(NOW).unwrap(),
                CaptureEventPayload::ResolutionRecorded(
                    ResolutionRecordedPayload::new(
                        registry_id,
                        revision,
                        digest(&format!("registry-{revision}")),
                        intent.initial_resolution().clone(),
                    )
                    .unwrap(),
                ),
            )
            .unwrap();
    }
    let paths = event_paths(&journal, capture_id);
    assert_eq!(paths.len(), 2);

    let second_name = paths[1].file_name().unwrap().to_str().unwrap();
    let (_, suffix) = second_name.split_once('-').unwrap();
    let gap_path = paths[1].with_file_name(format!("{:020}-{suffix}", 3));
    fs::rename(&paths[1], &gap_path).unwrap();
    let gap = journal
        .inspect_projection(capture_id)
        .expect_err("sequence gap must fail closed");
    assert!(gap.to_string().contains("gap or reordering"));
    fs::rename(&gap_path, &paths[1]).unwrap();

    let mut tampered: Value =
        serde_json::from_slice(&fs::read(&paths[0]).expect("first event bytes")).unwrap();
    tampered["payload"]["registry_revision"] = json!(99);
    let canonical = parse_canonical_json(&serde_json::to_vec(&tampered).unwrap()).unwrap();
    let mut bytes = canonical_bytes(&canonical).unwrap();
    bytes.push(b'\n');
    fs::write(&paths[0], bytes).unwrap();
    let tamper = journal
        .inspect_projection(capture_id)
        .expect_err("payload tampering must fail closed");
    assert!(tamper.to_string().contains("payload digest"));
}

fn empty_registry() -> ProjectRegistryV2 {
    ProjectRegistryV2::from_json_bytes(
        &serde_json::to_vec(&json!({
            "version": 2,
            "registry_id": RegistryId::new_v7(),
            "revision": 1,
            "projects": [],
            "locators": [],
            "bindings": [],
            "links": [],
            "observations": [],
            "migration": null
        }))
        .unwrap(),
    )
    .unwrap()
}

#[test]
fn first_write_registry_convergence_creates_once_and_rejects_target_substitution() {
    let temp = tempdir().unwrap();
    let store_path =
        CanonicalPath::parse(temp.path().join("stores/project.sqlite").to_str().unwrap()).unwrap();
    let candidate = FirstWriteProjectBinding::new(
        semantic_evidence(),
        Some("Round 2 Project".to_owned()),
        store_path,
        StoreId::new_v7(),
        WorkspaceId::new_v7(),
        BranchId::new_v7(),
        UtcTimestamp::parse(NOW).unwrap(),
    )
    .unwrap();
    let registry = empty_registry();
    let created = registry
        .converge_first_write_binding(candidate.clone())
        .unwrap();
    assert_eq!(created.outcome(), ProjectBootstrapOutcome::Created);
    assert_eq!(created.registry().revision(), 2);
    let project = created
        .registry()
        .project(created.project_ref_id())
        .unwrap();
    assert_eq!(project.maturity(), ProjectMaturity::Established);
    assert_eq!(project.created_by(), ProjectCreatedBy::SemanticLocator);
    assert_eq!(created.registry().projects().len(), 1);
    assert_eq!(created.registry().locators().len(), 1);
    assert_eq!(created.registry().bindings().len(), 1);

    let replay = created
        .registry()
        .converge_first_write_binding(candidate.clone())
        .unwrap();
    assert_eq!(replay.outcome(), ProjectBootstrapOutcome::Reused);
    assert_eq!(replay.project_ref_id(), created.project_ref_id());
    assert_eq!(replay.locator_id(), created.locator_id());
    assert_eq!(replay.registry().revision(), 2);

    let substituted = FirstWriteProjectBinding::new(
        semantic_evidence(),
        Some("Round 2 Project".to_owned()),
        CanonicalPath::parse(temp.path().join("stores/other.sqlite").to_str().unwrap()).unwrap(),
        StoreId::new_v7(),
        WorkspaceId::new_v7(),
        BranchId::new_v7(),
        UtcTimestamp::parse(NOW).unwrap(),
    )
    .unwrap();
    let conflict = replay
        .registry()
        .converge_first_write_binding(substituted)
        .expect_err("claimed locator cannot change target");
    assert!(conflict.to_string().contains("different target binding"));
}

#[test]
fn repository_and_cwd_first_writes_preserve_maturity_and_creation_provenance() {
    let temp = tempdir().unwrap();
    for (authority, provider, kind, maturity, created_by) in [
        (
            LocatorAuthority::Repository,
            "git",
            "git_common_dir",
            ProjectMaturity::Established,
            ProjectCreatedBy::RepositoryFirstWrite,
        ),
        (
            LocatorAuthority::Cwd,
            "filesystem",
            "canonical_directory",
            ProjectMaturity::Provisional,
            ProjectCreatedBy::CwdFirstWrite,
        ),
    ] {
        let registry = empty_registry();
        let namespace = format!("registry:{}", registry.registry_id());
        let normalized_value = match authority {
            LocatorAuthority::Repository => temp.path().join("repo/.git"),
            LocatorAuthority::Cwd => temp.path().join("cwd"),
            LocatorAuthority::SemanticProject => unreachable!(),
        };
        let evidence = LocatorEvidence::new(
            authority,
            provider,
            namespace,
            kind,
            normalized_value.display().to_string(),
            LocatorAssurance::VerifiedDerived,
            "fixture-path-adapter/v1",
            digest(&format!("{authority:?}")),
        )
        .unwrap();
        let candidate = FirstWriteProjectBinding::new(
            evidence,
            None,
            CanonicalPath::parse(
                temp.path()
                    .join(format!("{provider}-store.sqlite"))
                    .display()
                    .to_string(),
            )
            .unwrap(),
            StoreId::new_v7(),
            WorkspaceId::new_v7(),
            BranchId::new_v7(),
            UtcTimestamp::parse(NOW).unwrap(),
        )
        .unwrap();
        let result = registry.converge_first_write_binding(candidate).unwrap();
        let project = result.registry().project(result.project_ref_id()).unwrap();
        assert_eq!(project.maturity(), maturity);
        assert_eq!(project.created_by(), created_by);
    }
}
