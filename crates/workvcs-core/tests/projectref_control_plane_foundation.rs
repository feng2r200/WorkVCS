use serde_json::{Value, json};
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Barrier, mpsc};
use std::thread;
use std::time::Duration;
use tempfile::tempdir;
use workvcs_core::canonical::{canonical_bytes, parse_canonical_json};
use workvcs_core::control_plane::{
    CanonicalPath, CaptureAdmissionOutcome, CaptureIntent, CaptureJournal, ControlPlaneDigest,
    JournalQuiescenceLock, LocatorAssurance, LocatorAuthority, LocatorEvidence,
    MAX_SEMANTIC_PAYLOAD_BYTES, PathLocatorEvidence, ProjectMaturity, ProjectRegistryJournalAlias,
    ProjectRegistryV2, ResolutionContext, ResolutionDiagnostic, ResolutionMode, ResolutionRank,
    ResolutionStatus, StrongerLocatorAttachment, StrongerLocatorAttachmentBasis,
    StrongerLocatorAttachmentOutcome, UtcTimestamp, project_registry_journal_quiescence_lock_path,
    resolve_project,
};
use workvcs_core::{
    BranchId, CaptureGroupId, CaptureId, ErrorCode, ProjectLinkId, ProjectLocatorId, ProjectRefId,
    RegistryId, RegistryObservationId, StoreId, WorkspaceId,
};

const NOW: &str = "2026-09-23T08:00:00Z";

struct RegistryFixture {
    registry: ProjectRegistryV2,
    semantic_project: ProjectRefId,
    repository_project: ProjectRefId,
    cwd_project: ProjectRefId,
    repository_path: String,
    cwd_path: String,
}

fn digest(label: &str) -> ControlPlaneDigest {
    ControlPlaneDigest::raw(label.as_bytes())
}

fn sorted_by_id(mut values: Vec<Value>, field: &str) -> Vec<Value> {
    values.sort_by(|left, right| {
        left[field]
            .as_str()
            .expect("left id")
            .cmp(right[field].as_str().expect("right id"))
    });
    values
}

fn registry_fixture() -> RegistryFixture {
    let registry_id = RegistryId::new_v7();
    let semantic_project = ProjectRefId::new_v7();
    let repository_project = ProjectRefId::new_v7();
    let cwd_project = ProjectRefId::new_v7();
    let repository_path = "/tmp/workvcs-projectref-foundation/repository/.git".to_owned();
    let cwd_path = "/tmp/workvcs-projectref-foundation/cwd".to_owned();
    let namespace = format!("registry:{registry_id}");

    let projects = sorted_by_id(
        vec![
            json!({
                "project_ref_id": semantic_project,
                "maturity": "established",
                "display_name": "Semantic Project",
                "created_at": NOW,
                "created_by": "semantic_locator"
            }),
            json!({
                "project_ref_id": repository_project,
                "maturity": "established",
                "display_name": "Repository Project",
                "created_at": NOW,
                "created_by": "migration"
            }),
            json!({
                "project_ref_id": cwd_project,
                "maturity": "provisional",
                "display_name": null,
                "created_at": NOW,
                "created_by": "cwd_first_write"
            }),
        ],
        "project_ref_id",
    );
    let locators = sorted_by_id(
        vec![
            json!({
                "locator_id": ProjectLocatorId::new_v7(),
                "project_ref_id": semantic_project,
                "role": "identity",
                "authority": "semantic_project",
                "provider": "chatgpt",
                "namespace": "account-a",
                "kind": "project_id",
                "normalized_value": "g-p-1",
                "assurance": "authoritative",
                "source_adapter": "test-adapter-v1",
                "evidence_digest": digest("semantic-g-p-1"),
                "observed_at": NOW,
                "state": "active"
            }),
            json!({
                "locator_id": ProjectLocatorId::new_v7(),
                "project_ref_id": repository_project,
                "role": "identity",
                "authority": "repository",
                "provider": "git",
                "namespace": namespace,
                "kind": "git_common_dir",
                "normalized_value": repository_path,
                "assurance": "verified_derived",
                "source_adapter": "workvcs-git-v1",
                "evidence_digest": digest("repository-path"),
                "observed_at": NOW,
                "state": "active"
            }),
            json!({
                "locator_id": ProjectLocatorId::new_v7(),
                "project_ref_id": cwd_project,
                "role": "identity",
                "authority": "cwd",
                "provider": "filesystem",
                "namespace": namespace,
                "kind": "canonical_directory",
                "normalized_value": cwd_path,
                "assurance": "verified_derived",
                "source_adapter": "workvcs-filesystem-v1",
                "evidence_digest": digest("cwd-path"),
                "observed_at": NOW,
                "state": "active"
            }),
        ],
        "locator_id",
    );
    let value = json!({
        "version": 2,
        "registry_id": registry_id,
        "revision": 1,
        "projects": projects,
        "locators": locators,
        "bindings": [],
        "links": [],
        "observations": [],
        "migration": null
    });
    let registry = ProjectRegistryV2::from_json_bytes(
        &serde_json::to_vec_pretty(&value).expect("registry JSON"),
    )
    .expect("valid registry v2");
    RegistryFixture {
        registry,
        semantic_project,
        repository_project,
        cwd_project,
        repository_path,
        cwd_path,
    }
}

fn registry_with_cwd_binding(fixture: &RegistryFixture) -> ProjectRegistryV2 {
    let mut value: Value =
        serde_json::from_slice(&fixture.registry.canonical_json_bytes().unwrap()).unwrap();
    value["bindings"] = json!([{
        "project_ref_id": fixture.cwd_project,
        "store_path": "/tmp/workvcs-projectref-foundation/stores/cwd.sqlite",
        "store_id": StoreId::new_v7(),
        "workspace_id": WorkspaceId::new_v7(),
        "branch_id": BranchId::new_v7(),
        "bound_at": NOW,
        "binding_source": "first_write"
    }]);
    ProjectRegistryV2::from_json_bytes(&serde_json::to_vec(&value).unwrap()).unwrap()
}

fn semantic_evidence(value: &str, assurance: LocatorAssurance) -> LocatorEvidence {
    semantic_evidence_for("chatgpt", "account-a", value, assurance)
}

fn semantic_evidence_for(
    provider: &str,
    namespace: &str,
    value: &str,
    assurance: LocatorAssurance,
) -> LocatorEvidence {
    LocatorEvidence::new(
        LocatorAuthority::SemanticProject,
        provider,
        namespace,
        "project_id",
        value,
        assurance,
        "test-adapter-v1",
        digest(&format!(
            "semantic-{provider}-{namespace}-{value}-{assurance:?}"
        )),
    )
    .expect("semantic evidence")
}

fn path_evidence(path: &str, label: &str) -> PathLocatorEvidence {
    PathLocatorEvidence::new(
        CanonicalPath::parse(path).expect("canonical path"),
        "test-path-adapter-v1",
        digest(label),
    )
    .expect("path evidence")
}

#[test]
fn registry_v2_is_strict_sorted_and_canonical() {
    let fixture = registry_fixture();
    let canonical = fixture
        .registry
        .canonical_json_bytes()
        .expect("canonical registry");
    let reparsed = ProjectRegistryV2::from_json_bytes(&canonical).expect("reparse registry");
    assert_eq!(reparsed, fixture.registry);
    assert!(!canonical.ends_with(b"\n"));
    assert!(
        fixture
            .registry
            .stored_json_bytes()
            .unwrap()
            .ends_with(b"\n")
    );

    let mut unknown: Value = serde_json::from_slice(&canonical).unwrap();
    unknown["unexpected"] = json!(true);
    let error = ProjectRegistryV2::from_json_bytes(&serde_json::to_vec(&unknown).unwrap())
        .expect_err("unknown field must fail");
    assert_eq!(error.code(), ErrorCode::ControlPlaneInvalid);

    let mut missing_migration: Value = serde_json::from_slice(&canonical).unwrap();
    missing_migration
        .as_object_mut()
        .unwrap()
        .remove("migration");
    assert!(
        ProjectRegistryV2::from_json_bytes(&serde_json::to_vec(&missing_migration).unwrap())
            .is_err()
    );
    let directly_deserialized: ProjectRegistryV2 =
        serde_json::from_value(missing_migration).expect("shape remains deserializable");
    assert!(directly_deserialized.validate().is_err());

    let mut missing_display_name: Value = serde_json::from_slice(&canonical).unwrap();
    missing_display_name["projects"][0]
        .as_object_mut()
        .unwrap()
        .remove("display_name");
    assert!(
        ProjectRegistryV2::from_json_bytes(&serde_json::to_vec(&missing_display_name).unwrap())
            .is_err()
    );

    let mut unsorted: Value = serde_json::from_slice(&canonical).unwrap();
    unsorted["projects"].as_array_mut().unwrap().reverse();
    assert!(ProjectRegistryV2::from_json_bytes(&serde_json::to_vec(&unsorted).unwrap()).is_err());
}

#[test]
fn registry_v2_rejects_duplicate_active_identity_and_invalid_maturity() {
    let fixture = registry_fixture();
    let mut value: Value =
        serde_json::from_slice(&fixture.registry.canonical_json_bytes().unwrap()).unwrap();
    let duplicate = json!({
        "locator_id": ProjectLocatorId::new_v7(),
        "project_ref_id": fixture.repository_project,
        "role": "identity",
        "authority": "semantic_project",
        "provider": "chatgpt",
        "namespace": "account-a",
        "kind": "project_id",
        "normalized_value": "g-p-1",
        "assurance": "verified_derived",
        "source_adapter": "test-adapter-v1",
        "evidence_digest": digest("duplicate-claim"),
        "observed_at": NOW,
        "state": "active"
    });
    let locators = value["locators"].as_array_mut().unwrap();
    locators.push(duplicate);
    locators.sort_by(|left, right| {
        left["locator_id"]
            .as_str()
            .unwrap()
            .cmp(right["locator_id"].as_str().unwrap())
    });
    let error = ProjectRegistryV2::from_json_bytes(&serde_json::to_vec(&value).unwrap())
        .expect_err("duplicate active identity must fail");
    assert!(error.to_string().contains("claimed by ProjectRefs"));

    let mut maturity: Value =
        serde_json::from_slice(&fixture.registry.canonical_json_bytes().unwrap()).unwrap();
    let project = maturity["projects"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|project| project["project_ref_id"] == json!(fixture.cwd_project))
        .unwrap();
    project["maturity"] = json!("established");
    let error = ProjectRegistryV2::from_json_bytes(&serde_json::to_vec(&maturity).unwrap())
        .expect_err("CWD-only project cannot be established");
    assert!(error.to_string().contains("requires explicit creation"));
}

#[test]
fn resolver_orders_explicit_semantic_repository_and_cwd() {
    let fixture = registry_fixture();
    let semantic = semantic_evidence("g-p-1", LocatorAssurance::Authoritative);
    let git = path_evidence(&fixture.repository_path, "git");
    let cwd = path_evidence(&fixture.cwd_path, "cwd");

    let explicit = ResolutionContext::new(ResolutionMode::ReadOnly)
        .with_project_ref(fixture.cwd_project)
        .with_locator_evidence(semantic.clone())
        .with_git_common_dir(git.clone())
        .with_cwd(cwd.clone());
    let result = resolve_project(&fixture.registry, &explicit).expect("explicit resolution");
    assert_eq!(result.status(), ResolutionStatus::Resolved);
    assert_eq!(result.primary_project_ref(), Some(fixture.cwd_project));
    assert_eq!(
        result.primary_basis().unwrap().rank(),
        ResolutionRank::ExplicitProjectRef
    );
    assert_eq!(
        result.related_project_refs(),
        &[fixture.semantic_project, fixture.repository_project]
            .into_iter()
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>()
    );

    let semantic_context = ResolutionContext::new(ResolutionMode::ReadOnly)
        .with_locator_evidence(semantic)
        .with_git_common_dir(git)
        .with_cwd(cwd);
    let result = resolve_project(&fixture.registry, &semantic_context).expect("semantic owner");
    assert_eq!(result.primary_project_ref(), Some(fixture.semantic_project));
    assert_eq!(
        result.primary_basis().unwrap().rank(),
        ResolutionRank::SemanticProject
    );
    assert!(
        !result
            .diagnostics()
            .contains(&ResolutionDiagnostic::RepositoryFallback)
    );

    let git_only = ResolutionContext::new(ResolutionMode::ReadOnly)
        .with_git_common_dir(path_evidence(&fixture.repository_path, "git-only"))
        .with_cwd(path_evidence(&fixture.cwd_path, "cwd-lower"));
    let result = resolve_project(&fixture.registry, &git_only).expect("Git fallback");
    assert_eq!(
        result.primary_project_ref(),
        Some(fixture.repository_project)
    );
    assert!(
        result
            .diagnostics()
            .contains(&ResolutionDiagnostic::RepositoryFallback)
    );

    let cwd_only = ResolutionContext::new(ResolutionMode::ReadOnly)
        .with_cwd(path_evidence(&fixture.cwd_path, "cwd-only"));
    let result = resolve_project(&fixture.registry, &cwd_only).expect("CWD fallback");
    assert_eq!(result.primary_project_ref(), Some(fixture.cwd_project));
    assert!(
        result
            .diagnostics()
            .contains(&ResolutionDiagnostic::CwdFallback)
    );
}

#[test]
fn resolver_blocks_lower_fallback_and_fails_closed_on_conflict() {
    let fixture = registry_fixture();
    let unbound = ResolutionContext::new(ResolutionMode::DurableWrite)
        .with_locator_evidence(semantic_evidence(
            "g-p-unbound",
            LocatorAssurance::Authoritative,
        ))
        .with_git_common_dir(path_evidence(&fixture.repository_path, "bound-git"));
    let result = resolve_project(&fixture.registry, &unbound).expect("unbound semantic owner");
    assert_eq!(result.status(), ResolutionStatus::Unbound);
    assert_eq!(result.primary_project_ref(), None);
    assert_eq!(
        result.primary_basis().unwrap().rank(),
        ResolutionRank::SemanticProject
    );
    assert_eq!(result.related_project_refs(), &[fixture.repository_project]);

    let mismatch = ResolutionContext::new(ResolutionMode::ReadOnly)
        .with_locator_evidence(semantic_evidence(
            "g-p-authoritative",
            LocatorAssurance::Authoritative,
        ))
        .with_locator_evidence(semantic_evidence(
            "g-p-1",
            LocatorAssurance::VerifiedDerived,
        ))
        .with_git_common_dir(path_evidence(&fixture.repository_path, "mismatch-git"));
    let result = resolve_project(&fixture.registry, &mismatch).expect("mismatch resolution");
    assert_eq!(result.status(), ResolutionStatus::Unbound);
    assert!(
        result
            .diagnostics()
            .contains(&ResolutionDiagnostic::ContextMismatch)
    );
    assert!(
        result
            .related_project_refs()
            .contains(&fixture.semantic_project)
    );

    let conflict = ResolutionContext::new(ResolutionMode::ReadOnly)
        .with_locator_evidence(semantic_evidence("g-p-1", LocatorAssurance::Authoritative))
        .with_locator_evidence(semantic_evidence(
            "g-p-other",
            LocatorAssurance::Authoritative,
        ));
    let result = resolve_project(&fixture.registry, &conflict).expect("conflict result");
    assert_eq!(result.status(), ResolutionStatus::Conflict);
    assert_eq!(result.primary_project_ref(), None);
    assert!(
        result
            .diagnostics()
            .contains(&ResolutionDiagnostic::OwnershipConflict)
    );

    let cross_provider = ResolutionContext::new(ResolutionMode::ReadOnly)
        .with_locator_evidence(semantic_evidence("g-p-1", LocatorAssurance::Authoritative))
        .with_locator_evidence(semantic_evidence_for(
            "notion",
            "workspace-b",
            "project-b",
            LocatorAssurance::VerifiedDerived,
        ));
    let result =
        resolve_project(&fixture.registry, &cross_provider).expect("cross-provider conflict");
    assert_eq!(result.status(), ResolutionStatus::Conflict);
    assert!(
        result
            .diagnostics()
            .contains(&ResolutionDiagnostic::OwnershipConflict)
    );

    let multiple_unbound = ResolutionContext::new(ResolutionMode::ReadOnly)
        .with_locator_evidence(semantic_evidence(
            "g-p-unbound-a",
            LocatorAssurance::Authoritative,
        ))
        .with_locator_evidence(semantic_evidence(
            "g-p-unbound-b",
            LocatorAssurance::Authoritative,
        ));
    let result = resolve_project(&fixture.registry, &multiple_unbound).expect("unbound conflict");
    assert_eq!(result.status(), ResolutionStatus::Conflict);
    assert!(
        result
            .diagnostics()
            .contains(&ResolutionDiagnostic::OwnershipConflict)
    );

    let missing = ResolutionContext::new(ResolutionMode::ReadOnly)
        .with_project_ref(ProjectRefId::new_v7())
        .with_git_common_dir(path_evidence(&fixture.repository_path, "must-not-fallback"));
    let result = resolve_project(&fixture.registry, &missing).expect("missing explicit result");
    assert_eq!(result.status(), ResolutionStatus::Conflict);
    assert_eq!(
        result.diagnostics(),
        &[ResolutionDiagnostic::ProjectRefNotFound]
    );
}

#[test]
fn two_semantic_projects_sharing_repository_and_display_name_remain_distinct() {
    let fixture = registry_fixture();
    let second_project = ProjectRefId::new_v7();
    let mut value: Value =
        serde_json::from_slice(&fixture.registry.canonical_json_bytes().unwrap()).unwrap();
    for project in value["projects"].as_array_mut().unwrap().iter_mut() {
        if project["project_ref_id"] == json!(fixture.semantic_project) {
            project["display_name"] = json!("Shared Label");
        }
    }
    value["projects"].as_array_mut().unwrap().push(json!({
        "project_ref_id": second_project,
        "maturity": "established",
        "display_name": "Shared Label",
        "created_at": NOW,
        "created_by": "semantic_locator"
    }));
    value["projects"]
        .as_array_mut()
        .unwrap()
        .sort_by(|left, right| {
            left["project_ref_id"]
                .as_str()
                .unwrap()
                .cmp(right["project_ref_id"].as_str().unwrap())
        });
    value["locators"].as_array_mut().unwrap().push(json!({
        "locator_id": ProjectLocatorId::new_v7(),
        "project_ref_id": second_project,
        "role": "identity",
        "authority": "semantic_project",
        "provider": "chatgpt",
        "namespace": "account-a",
        "kind": "project_id",
        "normalized_value": "g-p-2",
        "assurance": "authoritative",
        "source_adapter": "test-adapter-v1",
        "evidence_digest": digest("semantic-g-p-2"),
        "observed_at": NOW,
        "state": "active"
    }));
    value["locators"]
        .as_array_mut()
        .unwrap()
        .sort_by(|left, right| {
            left["locator_id"]
                .as_str()
                .unwrap()
                .cmp(right["locator_id"].as_str().unwrap())
        });
    let registry =
        ProjectRegistryV2::from_json_bytes(&serde_json::to_vec(&value).unwrap()).unwrap();
    for (semantic_value, expected) in [
        ("g-p-1", fixture.semantic_project),
        ("g-p-2", second_project),
    ] {
        let context = ResolutionContext::new(ResolutionMode::ReadOnly)
            .with_locator_evidence(semantic_evidence(
                semantic_value,
                LocatorAssurance::Authoritative,
            ))
            .with_git_common_dir(path_evidence(&fixture.repository_path, "shared-repository"));
        let result = resolve_project(&registry, &context).unwrap();
        assert_eq!(result.primary_project_ref(), Some(expected));
        assert!(
            result
                .related_project_refs()
                .contains(&fixture.repository_project)
        );
    }
}

#[test]
fn stronger_locator_attachment_promotes_without_moving_target_and_reuses_exact_key() {
    let fixture = registry_fixture();
    let registry = registry_with_cwd_binding(&fixture);
    let original_binding = registry
        .binding(fixture.cwd_project)
        .expect("CWD project binding")
        .clone();
    let evidence = semantic_evidence("g-p-promoted", LocatorAssurance::Authoritative);
    let attachment = StrongerLocatorAttachment::new(
        fixture.cwd_project,
        evidence.clone(),
        UtcTimestamp::parse(NOW).unwrap(),
        StrongerLocatorAttachmentBasis::DeterministicAdapterProof(
            evidence.evidence_digest().clone(),
        ),
    )
    .unwrap();

    let attached = registry
        .attach_stronger_identity_locator(attachment.clone())
        .unwrap();
    assert_eq!(
        attached.outcome(),
        StrongerLocatorAttachmentOutcome::Attached
    );
    assert_eq!(attached.project_ref_id(), fixture.cwd_project);
    assert_eq!(attached.registry().revision(), registry.revision() + 1);
    assert_eq!(
        attached
            .registry()
            .project(fixture.cwd_project)
            .unwrap()
            .maturity(),
        ProjectMaturity::Established
    );
    assert_eq!(
        attached.registry().binding(fixture.cwd_project),
        Some(&original_binding)
    );

    let replay = attached
        .registry()
        .attach_stronger_identity_locator(attachment)
        .unwrap();
    assert_eq!(replay.outcome(), StrongerLocatorAttachmentOutcome::Reused);
    assert_eq!(replay.locator_id(), attached.locator_id());
    assert_eq!(replay.registry(), attached.registry());
}

#[test]
fn stronger_locator_attachment_rejects_another_projects_claim_without_change() {
    let fixture = registry_fixture();
    let registry = registry_with_cwd_binding(&fixture);
    let claimed = semantic_evidence("g-p-1", LocatorAssurance::Authoritative);
    let attachment = StrongerLocatorAttachment::new(
        fixture.cwd_project,
        claimed,
        UtcTimestamp::parse(NOW).unwrap(),
        StrongerLocatorAttachmentBasis::ExplicitProjectSelection,
    )
    .unwrap();
    let before = registry.canonical_json_bytes().unwrap();

    let error = registry
        .attach_stronger_identity_locator(attachment)
        .expect_err("claimed locator must not be reassigned");
    assert_eq!(error.code(), ErrorCode::LocatorAlreadyClaimed);
    assert_eq!(registry.canonical_json_bytes().unwrap(), before);
}

fn payload_digest(payload: &Value) -> ControlPlaneDigest {
    let raw = serde_json::to_vec(payload).unwrap();
    let canonical = parse_canonical_json(&raw).unwrap();
    ControlPlaneDigest::raw(&canonical_bytes(&canonical).unwrap())
}

fn intent_value(
    fixture: &RegistryFixture,
    capture_id: CaptureId,
    idempotency_key: &str,
    payload: Value,
) -> Value {
    let context = ResolutionContext::new(ResolutionMode::DurableWrite)
        .with_locator_evidence(semantic_evidence(
            "g-p-unbound",
            LocatorAssurance::Authoritative,
        ))
        .with_git_common_dir(path_evidence(&fixture.repository_path, "journal-git"));
    let result = resolve_project(&fixture.registry, &context).expect("journal resolution");
    assert_eq!(result.status(), ResolutionStatus::Unbound);
    json!({
        "journal_version": 1,
        "capture_id": capture_id,
        "idempotency_key": idempotency_key,
        "created_at": NOW,
        "value_reason": "Preserve an accepted implementation finding before routing",
        "payload_kind": "cognition_v2",
        "semantic_payload": payload,
        "payload_digest": payload_digest(&payload),
        "resolution_context": context,
        "initial_resolution": result,
        "capture_group": null
    })
}

fn parse_intent(value: &Value) -> CaptureIntent {
    CaptureIntent::from_json_bytes(&serde_json::to_vec_pretty(value).unwrap())
        .expect("valid capture intent")
}

#[test]
fn journal_atomically_installs_and_idempotently_reuses_intent() {
    let fixture = registry_fixture();
    let temp = tempdir().unwrap();
    let journal =
        CaptureJournal::for_standalone_root(temp.path().join("capture-journal/v1")).unwrap();
    let payload = json!({"records": [{"local_id": "finding-1", "statement": "value"}]});
    let first = parse_intent(&intent_value(
        &fixture,
        CaptureId::new_v7(),
        "journal-replay-key",
        payload.clone(),
    ));
    let admitted = journal.admit(&first).expect("first admission");
    assert_eq!(admitted.outcome(), CaptureAdmissionOutcome::Created);
    assert!(admitted.intent_path().is_file());
    let stored = fs::read(admitted.intent_path()).unwrap();
    assert!(stored.ends_with(b"\n"));
    assert_eq!(
        &stored[..stored.len() - 1],
        first.canonical_json_bytes().unwrap()
    );
    assert_eq!(journal.load(first.capture_id()).unwrap(), first);

    let replay = parse_intent(&intent_value(
        &fixture,
        CaptureId::new_v7(),
        "journal-replay-key",
        payload,
    ));
    let reused = journal.admit(&replay).expect("idempotent replay");
    assert_eq!(reused.outcome(), CaptureAdmissionOutcome::Reused);
    assert_eq!(reused.capture_id(), first.capture_id());
    assert_eq!(
        fs::read_dir(journal.root().join("intents"))
            .unwrap()
            .filter_map(|entry| entry.ok())
            .filter(
                |entry| entry.path().extension().and_then(|value| value.to_str()) == Some("json")
            )
            .count(),
        1
    );
}

#[test]
fn journal_serializes_concurrent_same_key_admission() {
    let fixture = registry_fixture();
    let temp = tempdir().unwrap();
    let journal = Arc::new(
        CaptureJournal::for_standalone_root(temp.path().join("capture-journal/v1")).unwrap(),
    );
    let payload = json!({"records": [{"local_id": "finding-1", "statement": "same"}]});
    let left = Arc::new(parse_intent(&intent_value(
        &fixture,
        CaptureId::new_v7(),
        "concurrent-key",
        payload.clone(),
    )));
    let right = Arc::new(parse_intent(&intent_value(
        &fixture,
        CaptureId::new_v7(),
        "concurrent-key",
        payload,
    )));
    let barrier = Arc::new(Barrier::new(3));
    let handles = [left, right].map(|intent| {
        let journal = Arc::clone(&journal);
        let barrier = Arc::clone(&barrier);
        thread::spawn(move || {
            barrier.wait();
            journal.admit(&intent).expect("concurrent admission")
        })
    });
    barrier.wait();
    let outcomes = handles
        .into_iter()
        .map(|handle| handle.join().unwrap().outcome())
        .collect::<Vec<_>>();
    assert!(outcomes.contains(&CaptureAdmissionOutcome::Created));
    assert!(outcomes.contains(&CaptureAdmissionOutcome::Reused));
}

#[test]
fn journal_registry_quiescence_identity_is_alias_independent_and_check_precedes_layout() {
    let fixture = registry_fixture();
    let temp = tempdir().unwrap();
    let registry_dir = temp.path().join("registry");
    fs::create_dir_all(&registry_dir).unwrap();
    let registry_path = registry_dir.join("project-bindings.json");
    fs::write(
        &registry_path,
        fixture.registry.stored_json_bytes().unwrap(),
    )
    .unwrap();
    let registry_path = fs::canonicalize(registry_path).unwrap();
    let lock_path = project_registry_journal_quiescence_lock_path(&registry_path).unwrap();
    let home_root = registry_path.parent().unwrap().join("capture-journal/v1");
    let sidecar_root =
        PathBuf::from(format!("{}.d", registry_path.display())).join("capture-journal/v1");
    let home = CaptureJournal::for_project_registry(
        &registry_path,
        ProjectRegistryJournalAlias::StandardRegistryHome,
    )
    .unwrap();
    let sidecar = CaptureJournal::for_project_registry(
        &registry_path,
        ProjectRegistryJournalAlias::RegistrySidecar,
    )
    .unwrap();
    let nonstandard_registry = registry_dir.join("custom-registry.json");
    fs::write(
        &nonstandard_registry,
        fixture.registry.stored_json_bytes().unwrap(),
    )
    .unwrap();
    let nonstandard_registry = fs::canonicalize(nonstandard_registry).unwrap();
    let unsupported_home = CaptureJournal::for_project_registry(
        &nonstandard_registry,
        ProjectRegistryJournalAlias::StandardRegistryHome,
    )
    .expect_err("standard-home alias must not accept an arbitrary registry layout");
    assert_eq!(unsupported_home.code(), ErrorCode::ControlPlaneInvalid);
    assert_eq!(home.root(), home_root);
    assert_eq!(sidecar.root(), sidecar_root);
    assert_eq!(home.quiescence_lock_path(), sidecar.quiescence_lock_path());
    assert_eq!(home.quiescence_lock_path(), lock_path);

    let intent = parse_intent(&intent_value(
        &fixture,
        CaptureId::new_v7(),
        "post-lock-check",
        json!({"records": [{"statement": "must not persist"}]}),
    ));
    let missing_check = home
        .admit(&intent)
        .expect_err("registry-coupled journal must require an explicit post-lock check");
    assert_eq!(missing_check.code(), ErrorCode::ControlPlaneInvalid);
    assert!(!home_root.exists());
    assert!(!lock_path.exists());
    let expected_digest = fixture.registry.digest().unwrap();
    let stale_identity = home
        .admit_for_project_registry_with_check(
            &intent,
            fixture.registry.revision() + 1,
            &expected_digest,
            |_| panic!("additional checks cannot bypass the mandatory registry identity check"),
        )
        .expect_err("stale registry identity must block before caller checks");
    assert_eq!(stale_identity.code(), ErrorCode::ControlPlaneInvalid);
    assert!(!home_root.exists());
    assert!(!lock_path.exists());
    let wrong_digest = digest("wrong-registry-snapshot");
    let stale_digest = home
        .admit_for_project_registry_with_check(
            &intent,
            fixture.registry.revision(),
            &wrong_digest,
            |_| panic!("additional checks cannot bypass the mandatory registry digest check"),
        )
        .expect_err("stale registry digest must block before caller checks");
    assert_eq!(stale_digest.code(), ErrorCode::ControlPlaneInvalid);
    assert!(!home_root.exists());
    assert!(!lock_path.exists());
    let changed_during_check = home
        .admit_for_project_registry_with_check(
            &intent,
            fixture.registry.revision(),
            &expected_digest,
            |_| {
                fs::write(&registry_path, b"{\"version\":1}\n").unwrap();
                Ok(())
            },
        )
        .expect_err("a caller check that changes the registry must not permit admission");
    assert_eq!(changed_during_check.code(), ErrorCode::ControlPlaneInvalid);
    assert!(!home_root.exists());
    assert!(!lock_path.exists());
    fs::write(
        &registry_path,
        fixture.registry.stored_json_bytes().unwrap(),
    )
    .unwrap();
    let error = home
        .admit_for_project_registry_with_check(
            &intent,
            fixture.registry.revision(),
            &expected_digest,
            |_| {
                assert!(lock_path.is_file(), "check must execute under the lock");
                assert!(
                    !home_root.exists(),
                    "journal layout must not exist before the post-lock check passes"
                );
                Err(workvcs_core::WorkVcsError::ControlPlaneInvalid(
                    "routing changed while waiting for journal quiescence".to_owned(),
                ))
            },
        )
        .expect_err("failed post-lock check must prevent admission");
    assert_eq!(error.code(), ErrorCode::ControlPlaneInvalid);
    assert!(!home_root.exists());
    assert!(!lock_path.exists());
}

#[test]
fn journal_orphan_quiescence_lock_fails_closed_until_exact_fixture_recovery() {
    let fixture = registry_fixture();
    let temp = tempdir().unwrap();
    let registry_path = temp.path().join("project-bindings.json");
    fs::write(
        &registry_path,
        fixture.registry.stored_json_bytes().unwrap(),
    )
    .unwrap();
    let registry_path = fs::canonicalize(registry_path).unwrap();
    let lock_path = project_registry_journal_quiescence_lock_path(&registry_path).unwrap();
    fs::write(&lock_path, b"orphaned-fixture-lock\n").unwrap();
    let journal = Arc::new(
        CaptureJournal::for_project_registry(
            &registry_path,
            ProjectRegistryJournalAlias::StandardRegistryHome,
        )
        .unwrap(),
    );
    let intent = Arc::new(parse_intent(&intent_value(
        &fixture,
        CaptureId::new_v7(),
        "orphan-recovery",
        json!({"records": [{"statement": "recover after exact removal"}]}),
    )));
    let (started_tx, started_rx) = mpsc::channel();
    let (result_tx, result_rx) = mpsc::channel();
    let expected_revision = fixture.registry.revision();
    let expected_digest = fixture.registry.digest().unwrap();
    let handle = {
        let journal = Arc::clone(&journal);
        let intent = Arc::clone(&intent);
        thread::spawn(move || {
            started_tx.send(()).unwrap();
            result_tx
                .send(journal.admit_for_project_registry(
                    &intent,
                    expected_revision,
                    &expected_digest,
                ))
                .unwrap();
        })
    };
    started_rx.recv().unwrap();
    assert!(
        result_rx.recv_timeout(Duration::from_millis(100)).is_err(),
        "an unresolved orphan lock must not be stolen automatically"
    );
    fs::remove_file(&lock_path).expect("fixture-only exact orphan recovery");
    let admitted = result_rx
        .recv_timeout(Duration::from_secs(2))
        .expect("admission should resume after exact recovery")
        .expect("admission succeeds after exact recovery");
    assert_eq!(admitted.outcome(), CaptureAdmissionOutcome::Created);
    handle.join().unwrap();
    assert!(!lock_path.exists());
}

#[test]
fn journal_quiescence_guard_never_removes_a_replacement_owner() {
    let temp = tempdir().unwrap();
    let registry_path = temp.path().join("project-bindings.json");
    fs::write(&registry_path, b"fixture registry identity\n").unwrap();
    let registry_path = fs::canonicalize(registry_path).unwrap();
    let lock_path = project_registry_journal_quiescence_lock_path(&registry_path).unwrap();
    let first = JournalQuiescenceLock::acquire(&lock_path).unwrap();
    fs::remove_file(&lock_path).expect("simulate lost first lock path");
    let second = JournalQuiescenceLock::acquire(&lock_path).unwrap();

    drop(first);
    second
        .verify_owned()
        .expect("old guard must preserve replacement owner");
    assert!(lock_path.is_file());
    drop(second);
    assert!(!lock_path.exists());
}

#[test]
fn journal_conflict_and_preinstall_failures_leave_no_extra_intent() {
    let fixture = registry_fixture();
    let temp = tempdir().unwrap();
    let journal =
        CaptureJournal::for_standalone_root(temp.path().join("capture-journal/v1")).unwrap();
    let first = parse_intent(&intent_value(
        &fixture,
        CaptureId::new_v7(),
        "conflict-key",
        json!({"records": [{"statement": "first"}]}),
    ));
    journal.admit(&first).unwrap();
    let conflict = parse_intent(&intent_value(
        &fixture,
        CaptureId::new_v7(),
        "conflict-key",
        json!({"records": [{"statement": "different"}]}),
    ));
    let error = journal.admit(&conflict).expect_err("payload conflict");
    assert_eq!(error.code(), ErrorCode::CaptureIdempotencyConflict);

    let duplicate = parse_intent(&intent_value(
        &fixture,
        CaptureId::new_v7(),
        "conflict-key",
        json!({"records": [{"statement": "first"}]}),
    ));
    fs::write(
        journal.intent_path(duplicate.capture_id()),
        duplicate.stored_json_bytes().unwrap(),
    )
    .unwrap();
    let error = journal
        .admit(&duplicate)
        .expect_err("corrupt duplicate idempotency keys must fail closed");
    assert_eq!(error.code(), ErrorCode::CaptureNotPersisted);

    let blocked_root = temp.path().join("not-a-directory");
    fs::write(&blocked_root, b"file").unwrap();
    let blocked = CaptureJournal::for_standalone_root(blocked_root.clone()).unwrap();
    let error = blocked.admit(&conflict).expect_err("layout failure");
    assert_eq!(error.code(), ErrorCode::CaptureNotPersisted);
    assert!(!blocked_root.join("intents").exists());
}

#[test]
fn capture_intent_rejects_secrets_oversize_and_invalid_group_before_write() {
    let fixture = registry_fixture();
    let mut missing_capture_group = intent_value(
        &fixture,
        CaptureId::new_v7(),
        "missing-capture-group",
        json!({"records": []}),
    );
    missing_capture_group
        .as_object_mut()
        .unwrap()
        .remove("capture_group");
    let error =
        CaptureIntent::from_json_bytes(&serde_json::to_vec(&missing_capture_group).unwrap())
            .expect_err("capture_group must be present even when null");
    assert_eq!(error.code(), ErrorCode::ControlPlaneInvalid);
    let directly_deserialized: CaptureIntent =
        serde_json::from_value(missing_capture_group).expect("shape remains deserializable");
    assert!(directly_deserialized.validate().is_err());

    let secret = intent_value(
        &fixture,
        CaptureId::new_v7(),
        "secret-key",
        json!({"nested": {"clientSecret": "must-not-persist"}}),
    );
    let error = CaptureIntent::from_json_bytes(&serde_json::to_vec(&secret).unwrap())
        .expect_err("secret field");
    assert_eq!(error.code(), ErrorCode::ControlPlaneInvalid);

    let oversized = intent_value(
        &fixture,
        CaptureId::new_v7(),
        "oversized-key",
        json!({"text": "x".repeat(MAX_SEMANTIC_PAYLOAD_BYTES + 1)}),
    );
    let error = CaptureIntent::from_json_bytes(&serde_json::to_vec(&oversized).unwrap())
        .expect_err("oversized payload");
    assert_eq!(error.code(), ErrorCode::ControlPlaneInvalid);

    let resolved_context = ResolutionContext::new(ResolutionMode::DurableWrite)
        .with_locator_evidence(semantic_evidence("g-p-1", LocatorAssurance::Authoritative));
    let resolved = resolve_project(&fixture.registry, &resolved_context).unwrap();
    let payload = json!({"records": [{"local_id": "record-1", "statement": "group"}]});
    let invalid_group = json!({
        "journal_version": 1,
        "capture_id": CaptureId::new_v7(),
        "idempotency_key": "invalid-group",
        "created_at": NOW,
        "value_reason": "Cross-project association",
        "payload_kind": "cognition_v2",
        "semantic_payload": payload,
        "payload_digest": payload_digest(&payload),
        "resolution_context": resolved_context,
        "initial_resolution": resolved,
        "capture_group": {
            "capture_group_id": CaptureGroupId::new_v7(),
            "primary_project_ref": fixture.semantic_project,
            "primary_locator_evidence_digest": null,
            "canonical_record_local_id": "record-1",
            "members": [
                {
                    "project_ref_id": fixture.semantic_project,
                    "role": "primary",
                    "relation": "primary",
                    "delivery_mode": "canonical"
                },
                {
                    "project_ref_id": fixture.repository_project,
                    "role": "artifact_repository",
                    "relation": "artifact_repository",
                    "delivery_mode": "canonical"
                }
            ]
        }
    });
    let error = CaptureIntent::from_json_bytes(&serde_json::to_vec(&invalid_group).unwrap())
        .expect_err("two canonical deliveries");
    assert!(
        error
            .to_string()
            .contains("exactly one canonical primary member")
    );

    let mut missing_primary = invalid_group;
    missing_primary["capture_group"]
        .as_object_mut()
        .unwrap()
        .remove("primary_locator_evidence_digest");
    let error = CaptureIntent::from_json_bytes(&serde_json::to_vec(&missing_primary).unwrap())
        .expect_err("nullable capture group fields must be present");
    assert_eq!(error.code(), ErrorCode::ControlPlaneInvalid);
}

#[test]
fn registry_accepts_full_empty_families_with_typed_ids() {
    let registry_id = RegistryId::new_v7();
    let project_ref = ProjectRefId::new_v7();
    let store_id = StoreId::new_v7();
    let workspace_id = WorkspaceId::new_v7();
    let branch_id = BranchId::new_v7();
    let namespace = format!("registry:{registry_id}");
    let value = json!({
        "version": 2,
        "registry_id": registry_id,
        "revision": 7,
        "projects": [{
            "project_ref_id": project_ref,
            "maturity": "established",
            "display_name": "Explicit Project",
            "created_at": NOW,
            "created_by": "explicit"
        }],
        "locators": [{
            "locator_id": ProjectLocatorId::new_v7(),
            "project_ref_id": project_ref,
            "role": "identity",
            "authority": "repository",
            "provider": "git",
            "namespace": namespace,
            "kind": "git_common_dir",
            "normalized_value": "/tmp/explicit/.git",
            "assurance": "verified_derived",
            "source_adapter": "workvcs-git-v1",
            "evidence_digest": digest("explicit-git"),
            "observed_at": NOW,
            "state": "active"
        }],
        "bindings": [{
            "project_ref_id": project_ref,
            "store_path": "/tmp/workvcs-store.sqlite",
            "store_id": store_id,
            "workspace_id": workspace_id,
            "branch_id": branch_id,
            "bound_at": NOW,
            "binding_source": "explicit"
        }],
        "links": [],
        "observations": [],
        "migration": null
    });
    let registry = ProjectRegistryV2::from_json_bytes(&serde_json::to_vec(&value).unwrap())
        .expect("complete registry");
    assert_eq!(registry.registry_id(), registry_id);
    assert_eq!(registry.bindings()[0].store_id(), store_id);

    let _unused_future_ids = (ProjectLinkId::new_v7(), RegistryObservationId::new_v7());
}
