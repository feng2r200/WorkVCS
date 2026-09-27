use serde_json::{Value, json};
use workvcs_core::control_plane::{
    BoundedAdapterContext, CanonicalPath, ContextLocatorInvocation, ContextLocatorProvider,
    ControlPlaneDigest, LocatorAssurance, LocatorAuthority, LocatorEvidence,
    LocatorEvidenceExplanation, LocatorRole, LocatorState, MigrationBindingValidation,
    MigrationHistoricalIdentityDisposition, MigrationNamespaceStrategy,
    MigrationOwnershipRepairManifest, PathLocatorEvidence, ProjectMaturity, ProjectRegistryV1,
    ResolutionMode, UnifiedLocatorInput, build_registry_v1_migration_preview,
    build_registry_v1_migration_preview_with_repairs,
};
use workvcs_core::{BranchId, ProjectRefId, Result, StoreId, WorkspaceId};

struct StaticProvider {
    provider_id: String,
    evidence: Vec<LocatorEvidence>,
    explanation_digest_override: Option<ControlPlaneDigest>,
}

impl StaticProvider {
    fn new(provider_id: &str, evidence: Vec<LocatorEvidence>) -> Self {
        Self {
            provider_id: provider_id.to_owned(),
            evidence,
            explanation_digest_override: None,
        }
    }
}

impl ContextLocatorProvider for StaticProvider {
    fn provider_id(&self) -> &str {
        &self.provider_id
    }

    fn locate(&self, _context: &BoundedAdapterContext) -> Result<Vec<LocatorEvidence>> {
        Ok(self.evidence.clone())
    }

    fn explain(&self, evidence_digest: &ControlPlaneDigest) -> Result<LocatorEvidenceExplanation> {
        LocatorEvidenceExplanation::from_json_bytes(
            self.explanation_digest_override
                .clone()
                .unwrap_or_else(|| evidence_digest.clone()),
            br#"{"basis":"verified project metadata"}"#,
        )
    }
}

fn semantic_evidence(
    adapter: &str,
    provider: &str,
    namespace: &str,
    project: &str,
) -> LocatorEvidence {
    LocatorEvidence::new(
        LocatorAuthority::SemanticProject,
        provider,
        namespace,
        "project_id",
        project,
        LocatorAssurance::Authoritative,
        adapter,
        ControlPlaneDigest::raw(format!("{adapter}:{provider}:{namespace}:{project}").as_bytes()),
    )
    .unwrap()
}

fn path_evidence(path: &str, adapter: &str) -> PathLocatorEvidence {
    PathLocatorEvidence::new(
        CanonicalPath::parse(path).unwrap(),
        adapter,
        ControlPlaneDigest::raw(format!("{adapter}:{path}").as_bytes()),
    )
    .unwrap()
}

#[test]
fn unified_adapter_input_is_tool_neutral_deterministic_and_bounded() {
    let chat = StaticProvider::new(
        "chat-provider/v1",
        vec![semantic_evidence(
            "chat-provider/v1",
            "chat",
            "account-a",
            "project-a",
        )],
    );
    let docs = StaticProvider::new(
        "docs-provider/v2",
        vec![semantic_evidence(
            "docs-provider/v2",
            "docs",
            "workspace-b",
            "project-b",
        )],
    );
    let chat_context = BoundedAdapterContext::from_json_bytes(
        br#"{"project":{"id":"project-a"},"source":"desktop"}"#,
    )
    .unwrap();
    let docs_context = BoundedAdapterContext::from_json_bytes(
        br#"{"container":{"id":"project-b"},"source":"api"}"#,
    )
    .unwrap();

    let forward = UnifiedLocatorInput::collect(
        ResolutionMode::ReadOnly,
        Some(ProjectRefId::new_v7()),
        &[
            ContextLocatorInvocation::new(&chat, &chat_context),
            ContextLocatorInvocation::new(&docs, &docs_context),
        ],
        Some(path_evidence("/tmp/repository/.git", "workvcs-git-v1")),
        Some(path_evidence("/tmp/repository", "workvcs-cwd-v1")),
    )
    .unwrap();
    let reverse = UnifiedLocatorInput::collect(
        ResolutionMode::ReadOnly,
        forward.resolution_context().project_ref_id(),
        &[
            ContextLocatorInvocation::new(&docs, &docs_context),
            ContextLocatorInvocation::new(&chat, &chat_context),
        ],
        Some(path_evidence("/tmp/repository/.git", "workvcs-git-v1")),
        Some(path_evidence("/tmp/repository", "workvcs-cwd-v1")),
    )
    .unwrap();

    assert_eq!(forward, reverse);
    assert_eq!(
        forward.provider_ids(),
        &["chat-provider/v1".to_owned(), "docs-provider/v2".to_owned()]
    );
    assert_eq!(forward.resolution_context().locator_evidence().len(), 2);
    assert_eq!(forward.explanations().len(), 2);
    assert!(forward.resolution_context().git_common_dir().is_some());
    assert!(forward.resolution_context().cwd().is_some());

    let secret = BoundedAdapterContext::from_json_bytes(
        br#"{"project_id":"project-a","clientSecret":"do-not-keep"}"#,
    )
    .expect_err("secret-bearing context must fail");
    assert!(secret.to_string().contains("secret-bearing"));
    let oversized = serde_json::to_vec(&json!({"text": "x".repeat(64 * 1_024)})).unwrap();
    assert!(BoundedAdapterContext::from_json_bytes(&oversized).is_err());

    let explanation_digest = ControlPlaneDigest::raw(b"explanation");
    assert!(
        LocatorEvidenceExplanation::from_json_bytes(
            explanation_digest.clone(),
            br#"{"token":"do-not-keep"}"#,
        )
        .is_err()
    );
    let oversized_explanation =
        serde_json::to_vec(&json!({"text": "x".repeat(4 * 1_024)})).unwrap();
    assert!(
        LocatorEvidenceExplanation::from_json_bytes(explanation_digest, &oversized_explanation,)
            .is_err()
    );

    let too_many = StaticProvider::new(
        "bulk-provider/v1",
        (0..129)
            .map(|index| {
                semantic_evidence(
                    "bulk-provider/v1",
                    "bulk",
                    "tenant-a",
                    &format!("project-{index}"),
                )
            })
            .collect(),
    );
    assert!(
        UnifiedLocatorInput::collect(
            ResolutionMode::ReadOnly,
            None,
            &[ContextLocatorInvocation::new(&too_many, &chat_context)],
            None,
            None,
        )
        .is_err()
    );
}

#[test]
fn verified_adapter_evidence_uses_the_same_unified_resolution_input() {
    let evidence = semantic_evidence(
        "generic-project-metadata/v1",
        "generic",
        "tenant-a",
        "project-a",
    );
    let input = UnifiedLocatorInput::from_verified_evidence(
        ResolutionMode::ReadOnly,
        None,
        vec![evidence.clone(), evidence],
        Some(path_evidence("/tmp/repository/.git", "workvcs-git/v1")),
        Some(path_evidence("/tmp/repository", "workvcs-cwd/v1")),
    )
    .expect("verified evidence input");

    assert_eq!(
        input.provider_ids(),
        &["generic-project-metadata/v1".to_owned()]
    );
    assert_eq!(input.resolution_context().locator_evidence().len(), 1);
    assert!(input.explanations().is_empty());

    let nonsemantic = LocatorEvidence::new(
        LocatorAuthority::Repository,
        "git",
        "registry:test",
        "git_common_dir",
        "/tmp/repository/.git",
        LocatorAssurance::VerifiedDerived,
        "generic-project-metadata/v1",
        ControlPlaneDigest::raw(b"repo"),
    )
    .unwrap();
    let error = UnifiedLocatorInput::from_verified_evidence(
        ResolutionMode::ReadOnly,
        None,
        vec![nonsemantic],
        None,
        None,
    )
    .expect_err("nonsemantic evidence must use the verified path channels");
    assert!(error.to_string().contains("only semantic"));
}

#[test]
fn invoked_and_already_verified_semantic_evidence_share_one_bounded_input() {
    let invoked_evidence = semantic_evidence(
        "desktop-project/v1",
        "desktop",
        "installation-a",
        "project-a",
    );
    let provider = StaticProvider::new("desktop-project/v1", vec![invoked_evidence.clone()]);
    let context = BoundedAdapterContext::from_json_bytes(br#"{"project":"project-a"}"#).unwrap();
    let verified = semantic_evidence(
        "orchestrator-project/v2",
        "orchestrator",
        "tenant-b",
        "project-b",
    );

    let input = UnifiedLocatorInput::collect_with_verified_evidence(
        ResolutionMode::ReadOnly,
        None,
        &[ContextLocatorInvocation::new(&provider, &context)],
        vec![verified],
        Some(path_evidence("/tmp/repository/.git", "workvcs-git/v1")),
        Some(path_evidence("/tmp/repository", "workvcs-cwd/v1")),
    )
    .expect("combined semantic locator input");

    assert_eq!(
        input.provider_ids(),
        &[
            "desktop-project/v1".to_owned(),
            "orchestrator-project/v2".to_owned()
        ]
    );
    assert_eq!(input.resolution_context().locator_evidence().len(), 2);
    assert_eq!(input.explanations().len(), 1);
    assert_eq!(
        input.explanations()[0].evidence_digest(),
        invoked_evidence.evidence_digest()
    );
}

#[test]
fn unified_adapter_input_rejects_provider_spoofing_nonsemantic_output_and_bad_explanation() {
    let context = BoundedAdapterContext::from_json_bytes(br#"{"id":"project-a"}"#).unwrap();
    let spoofed = StaticProvider::new(
        "declared-provider/v1",
        vec![semantic_evidence(
            "different-provider/v1",
            "chat",
            "account-a",
            "project-a",
        )],
    );
    let error = UnifiedLocatorInput::collect(
        ResolutionMode::ReadOnly,
        None,
        &[ContextLocatorInvocation::new(&spoofed, &context)],
        None,
        None,
    )
    .expect_err("source adapter spoofing must fail");
    assert!(error.to_string().contains("source_adapter"));

    let repository = LocatorEvidence::new(
        LocatorAuthority::Repository,
        "git",
        "registry:test",
        "git_common_dir",
        "/tmp/repository/.git",
        LocatorAssurance::VerifiedDerived,
        "repo-provider/v1",
        ControlPlaneDigest::raw(b"repo"),
    )
    .unwrap();
    let nonsemantic = StaticProvider::new("repo-provider/v1", vec![repository]);
    let error = UnifiedLocatorInput::collect(
        ResolutionMode::ReadOnly,
        None,
        &[ContextLocatorInvocation::new(&nonsemantic, &context)],
        None,
        None,
    )
    .expect_err("provider path evidence must use the dedicated input");
    assert!(error.to_string().contains("non-semantic"));

    let mut bad_explanation = StaticProvider::new(
        "chat-provider/v1",
        vec![semantic_evidence(
            "chat-provider/v1",
            "chat",
            "account-a",
            "project-a",
        )],
    );
    bad_explanation.explanation_digest_override = Some(ControlPlaneDigest::raw(b"wrong"));
    let error = UnifiedLocatorInput::collect(
        ResolutionMode::ReadOnly,
        None,
        &[ContextLocatorInvocation::new(&bad_explanation, &context)],
        None,
        None,
    )
    .expect_err("explanation must be bound to evidence digest");
    assert!(error.to_string().contains("explanation digest"));
}

fn v1_registry(bindings: Vec<Value>) -> ProjectRegistryV1 {
    ProjectRegistryV1::from_json_bytes(
        &serde_json::to_vec_pretty(&json!({"version": 1, "bindings": bindings})).unwrap(),
    )
    .unwrap()
}

fn binding(
    identity_kind: &str,
    identity: &str,
    root: &str,
    target: (&str, StoreId, WorkspaceId, BranchId),
) -> Value {
    json!({
        "identity_kind": identity_kind,
        "identity": identity,
        "root": root,
        "store_path": target.0,
        "store_id": target.1,
        "workspace_id": target.2,
        "branch_id": target.3
    })
}

fn repair_manifest_value(
    registry: &ProjectRegistryV1,
    binding_index: usize,
    provider: &str,
    namespace: &str,
    project: &str,
) -> Value {
    let binding = &registry.bindings()[binding_index];
    json!({
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
                "provider": provider,
                "namespace": namespace,
                "kind": "project_id",
                "normalized_value": project,
                "assurance": "authoritative",
                "source_adapter": "test-project-metadata/v1",
                "evidence_digest": ControlPlaneDigest::raw(
                    format!("{provider}:{namespace}:{project}").as_bytes()
                )
            },
            "historical_identity_disposition": "retire"
        }]
    })
}

fn repair_manifest(value: &Value) -> MigrationOwnershipRepairManifest {
    MigrationOwnershipRepairManifest::from_json_bytes(&serde_json::to_vec(value).unwrap()).unwrap()
}

#[test]
fn migration_preview_is_repeatable_sorted_and_excludes_generated_identity() {
    let target_a = (
        "/tmp/stores/a.sqlite",
        StoreId::new_v7(),
        WorkspaceId::new_v7(),
        BranchId::new_v7(),
    );
    let target_b = (
        "/tmp/stores/b.sqlite",
        StoreId::new_v7(),
        WorkspaceId::new_v7(),
        BranchId::new_v7(),
    );
    let first_registry = v1_registry(vec![
        binding("cwd", "/tmp/z", "/tmp/z", target_b),
        binding("git-common-dir", "/tmp/a/.git", "/tmp/a", target_a),
    ]);
    let reordered_registry = v1_registry(vec![
        binding("git-common-dir", "/tmp/a/.git", "/tmp/a", target_a),
        binding("cwd", "/tmp/z", "/tmp/z", target_b),
    ]);
    let validations = [
        MigrationBindingValidation::passed(0, 2),
        MigrationBindingValidation::passed(1, 0),
    ];

    let first = build_registry_v1_migration_preview(&first_registry, &validations).unwrap();
    let repeated = build_registry_v1_migration_preview(&first_registry, &validations).unwrap();
    let reordered = build_registry_v1_migration_preview(&reordered_registry, &validations).unwrap();
    assert_eq!(first, repeated);
    assert_eq!(first, reordered);
    assert_eq!(first.source_digest(), reordered.source_digest());
    assert_eq!(first.preview_digest(), reordered.preview_digest());
    assert!(first.apply_eligible());
    assert!(first.migration_required());
    assert_eq!(first.mappings().len(), 2);
    let repository = first
        .mappings()
        .iter()
        .find(|mapping| mapping.v1_binding_key().identity_kind() == "git-common-dir")
        .unwrap();
    assert_eq!(
        repository.planned_maturity(),
        Some(ProjectMaturity::Established)
    );
    assert_eq!(
        repository.identity_locator().unwrap().role(),
        LocatorRole::Identity
    );
    assert!(repository.root_context_locator().is_some());
    let canonical = String::from_utf8(first.canonical_json_bytes().unwrap()).unwrap();
    assert!(!canonical.contains("project_ref_id"));
    assert!(!canonical.contains("created_at"));
}

#[test]
fn migration_ownership_repair_replaces_identity_preserves_target_and_retires_history() {
    let target = (
        "/tmp/stores/hernes.sqlite",
        StoreId::new_v7(),
        WorkspaceId::new_v7(),
        BranchId::new_v7(),
    );
    let registry = v1_registry(vec![binding(
        "cwd",
        "/tmp/stale-hernes",
        "/tmp/stale-hernes",
        target,
    )]);
    let validations = [MigrationBindingValidation::passed(0, 8)];
    let raw = build_registry_v1_migration_preview(&registry, &validations).unwrap();
    let manifest = repair_manifest(&repair_manifest_value(
        &registry,
        0,
        "chat",
        "local-installation:abc123",
        "project-hernes",
    ));

    let repaired =
        build_registry_v1_migration_preview_with_repairs(&registry, &validations, Some(&manifest))
            .unwrap();
    let repeated =
        build_registry_v1_migration_preview_with_repairs(&registry, &validations, Some(&manifest))
            .unwrap();
    assert_eq!(repaired, repeated);
    assert_eq!(raw.preview_version(), 1);
    assert_eq!(repaired.preview_version(), 2);
    assert_eq!(repaired.source_digest(), raw.source_digest());
    assert_eq!(repaired.ownership_repairs(), 1);
    assert_eq!(
        repaired.repair_manifest_digest(),
        Some(&manifest.digest().unwrap())
    );
    assert!(repaired.apply_eligible());

    let raw_mapping = &raw.mappings()[0];
    let mapping = &repaired.mappings()[0];
    assert_eq!(mapping.target(), raw_mapping.target());
    assert_eq!(mapping.target_digest(), raw_mapping.target_digest());
    assert_eq!(
        mapping.planned_maturity(),
        Some(ProjectMaturity::Established)
    );
    let semantic = mapping.identity_locator().unwrap();
    assert_eq!(semantic.role(), LocatorRole::Identity);
    assert_eq!(semantic.authority(), LocatorAuthority::SemanticProject);
    assert_eq!(semantic.provider(), "chat");
    assert_eq!(
        semantic.namespace_strategy(),
        MigrationNamespaceStrategy::Explicit
    );
    assert_eq!(semantic.namespace(), Some("local-installation:abc123"));
    assert_eq!(semantic.normalized_value(), "project-hernes");
    assert_eq!(semantic.state(), Some(LocatorState::Active));
    let historical = mapping.historical_identity_locator().unwrap();
    assert_eq!(historical.authority(), LocatorAuthority::Cwd);
    assert_eq!(historical.normalized_value(), "/tmp/stale-hernes");
    assert_eq!(historical.state(), Some(LocatorState::Retired));
    assert!(mapping.root_context_locator().is_none());
    assert_eq!(
        mapping
            .ownership_repair()
            .unwrap()
            .historical_identity_disposition(),
        MigrationHistoricalIdentityDisposition::Retire
    );

    let raw_json = String::from_utf8(raw.canonical_json_bytes().unwrap()).unwrap();
    assert!(!raw_json.contains("repair_manifest_digest"));
    assert!(!raw_json.contains("ownership_repair"));
    assert!(!raw_json.contains("historical_identity_locator"));
}

#[test]
fn migration_ownership_repair_manifest_is_strict_bounded_and_registry_bound() {
    let first_target = (
        "/tmp/stores/a.sqlite",
        StoreId::new_v7(),
        WorkspaceId::new_v7(),
        BranchId::new_v7(),
    );
    let second_target = (
        "/tmp/stores/b.sqlite",
        StoreId::new_v7(),
        WorkspaceId::new_v7(),
        BranchId::new_v7(),
    );
    let registry = v1_registry(vec![
        binding("cwd", "/tmp/a", "/tmp/a", first_target),
        binding("cwd", "/tmp/b", "/tmp/b", second_target),
    ]);
    let valid = repair_manifest_value(&registry, 0, "chat", "tenant-a", "project-a");

    let mut unknown_field = valid.clone();
    unknown_field["unexpected"] = json!(true);
    assert!(
        MigrationOwnershipRepairManifest::from_json_bytes(
            &serde_json::to_vec(&unknown_field).unwrap()
        )
        .is_err()
    );

    let mut wrong_schema = valid.clone();
    wrong_schema["schema_version"] = json!(2);
    assert!(
        MigrationOwnershipRepairManifest::from_json_bytes(
            &serde_json::to_vec(&wrong_schema).unwrap()
        )
        .is_err()
    );

    let mut empty = valid.clone();
    empty["repairs"] = json!([]);
    assert!(
        MigrationOwnershipRepairManifest::from_json_bytes(&serde_json::to_vec(&empty).unwrap())
            .is_err()
    );

    let mut observed = valid.clone();
    observed["repairs"][0]["semantic_locator"]["assurance"] = json!("observed");
    assert!(
        MigrationOwnershipRepairManifest::from_json_bytes(&serde_json::to_vec(&observed).unwrap())
            .is_err()
    );

    let mut repository = valid.clone();
    repository["repairs"][0]["semantic_locator"]["authority"] = json!("repository");
    assert!(
        MigrationOwnershipRepairManifest::from_json_bytes(
            &serde_json::to_vec(&repository).unwrap()
        )
        .is_err()
    );

    let mut repeated_binding = valid.clone();
    repeated_binding["repairs"] = json!([valid["repairs"][0].clone(), valid["repairs"][0].clone()]);
    assert!(
        MigrationOwnershipRepairManifest::from_json_bytes(
            &serde_json::to_vec(&repeated_binding).unwrap()
        )
        .is_err()
    );

    let mut repeated_semantic = valid.clone();
    let mut second_repair =
        repair_manifest_value(&registry, 1, "chat", "tenant-a", "project-a")["repairs"][0].clone();
    second_repair["semantic_locator"]["evidence_digest"] =
        valid["repairs"][0]["semantic_locator"]["evidence_digest"].clone();
    repeated_semantic["repairs"] = json!([valid["repairs"][0].clone(), second_repair]);
    assert!(
        MigrationOwnershipRepairManifest::from_json_bytes(
            &serde_json::to_vec(&repeated_semantic).unwrap()
        )
        .is_err()
    );

    let mut stale_source = valid.clone();
    stale_source["expected_source_digest"] = json!(ControlPlaneDigest::raw(b"different registry"));
    let stale_source = repair_manifest(&stale_source);
    assert!(stale_source.validate_for_registry(&registry).is_err());

    let mut wrong_target = valid.clone();
    wrong_target["repairs"][0]["expected_target_digest"] =
        json!(ControlPlaneDigest::raw(b"different target"));
    let wrong_target = repair_manifest(&wrong_target);
    assert!(wrong_target.validate_for_registry(&registry).is_err());

    let mut missing_binding = valid;
    missing_binding["repairs"][0]["v1_binding_key"]["identity"] = json!("/tmp/missing");
    let missing_binding = repair_manifest(&missing_binding);
    assert!(missing_binding.validate_for_registry(&registry).is_err());

    assert!(
        MigrationOwnershipRepairManifest::from_json_bytes(&vec![b' '; 64 * 1_024 + 1]).is_err()
    );
}

#[test]
fn migration_ownership_repairs_are_canonically_ordered_and_retire_git_context() {
    let cwd_target = (
        "/tmp/stores/cwd.sqlite",
        StoreId::new_v7(),
        WorkspaceId::new_v7(),
        BranchId::new_v7(),
    );
    let git_target = (
        "/tmp/stores/git.sqlite",
        StoreId::new_v7(),
        WorkspaceId::new_v7(),
        BranchId::new_v7(),
    );
    let registry = v1_registry(vec![
        binding("cwd", "/tmp/cwd", "/tmp/cwd", cwd_target),
        binding(
            "git-common-dir",
            "/tmp/repository/.git",
            "/tmp/repository",
            git_target,
        ),
    ]);
    let first =
        repair_manifest_value(&registry, 0, "chat", "tenant-a", "project-a")["repairs"][0].clone();
    let second =
        repair_manifest_value(&registry, 1, "docs", "tenant-b", "project-b")["repairs"][0].clone();
    let forward = repair_manifest(&json!({
        "schema_version": 1,
        "expected_source_digest": registry.source_digest().unwrap(),
        "repairs": [first.clone(), second.clone()]
    }));
    let reverse = repair_manifest(&json!({
        "schema_version": 1,
        "expected_source_digest": registry.source_digest().unwrap(),
        "repairs": [second, first]
    }));
    assert_eq!(forward, reverse);
    assert_eq!(forward.digest().unwrap(), reverse.digest().unwrap());

    let validations = [
        MigrationBindingValidation::passed(0, 0),
        MigrationBindingValidation::passed(1, 0),
    ];
    let preview =
        build_registry_v1_migration_preview_with_repairs(&registry, &validations, Some(&forward))
            .unwrap();
    assert_eq!(preview.ownership_repairs(), 2);
    assert!(preview.apply_eligible());
    let git = preview
        .mappings()
        .iter()
        .find(|mapping| mapping.v1_binding_key().identity_kind() == "git-common-dir")
        .unwrap();
    assert_eq!(
        git.identity_locator().unwrap().authority(),
        LocatorAuthority::SemanticProject
    );
    assert_eq!(
        git.historical_identity_locator().unwrap().state(),
        Some(LocatorState::Retired)
    );
    assert_eq!(
        git.root_context_locator().unwrap().state(),
        Some(LocatorState::Retired)
    );
    assert_eq!(
        git.root_context_locator().unwrap().normalized_value(),
        "/tmp/repository"
    );
}

#[test]
fn migration_preview_reports_unknown_identity_invalid_target_and_duplicate_identity() {
    let target = (
        "/tmp/stores/missing.sqlite",
        StoreId::new_v7(),
        WorkspaceId::new_v7(),
        BranchId::new_v7(),
    );
    let registry = v1_registry(vec![
        binding("future-kind", "/tmp/a", "/tmp/a", target),
        binding("future-kind", "/tmp/a", "/tmp/a", target),
    ]);
    let validations = [
        MigrationBindingValidation::failed(
            0,
            "store_open_failed",
            "Store could not be opened read-only",
        )
        .unwrap(),
        MigrationBindingValidation::failed(
            1,
            "store_open_failed",
            "Store could not be opened read-only",
        )
        .unwrap(),
    ];
    let preview = build_registry_v1_migration_preview(&registry, &validations).unwrap();
    assert!(!preview.apply_eligible());
    assert_eq!(preview.target_coincidences().len(), 1);
    for mapping in preview.mappings() {
        assert!(mapping.identity_locator().is_none());
        let codes = mapping
            .validation()
            .issues()
            .iter()
            .map(|issue| issue.code())
            .collect::<Vec<_>>();
        assert!(codes.contains(&"unknown_v1_identity_kind"));
        assert!(codes.contains(&"duplicate_v1_identity"));
        assert!(codes.contains(&"store_open_failed"));
    }
}

#[test]
fn registry_v1_preview_input_is_strict_and_validation_facts_are_total() {
    let target = (
        "/tmp/stores/a.sqlite",
        StoreId::new_v7(),
        WorkspaceId::new_v7(),
        BranchId::new_v7(),
    );
    let value = json!({
        "version": 1,
        "bindings": [binding("cwd", "/tmp/a", "/tmp/a", target)],
        "unexpected": true
    });
    assert!(ProjectRegistryV1::from_json_bytes(&serde_json::to_vec(&value).unwrap()).is_err());

    let registry = v1_registry(Vec::new());
    let preview = build_registry_v1_migration_preview(&registry, &[]).unwrap();
    assert!(preview.apply_eligible());
    assert!(
        build_registry_v1_migration_preview(
            &v1_registry(vec![json!({
                "identity_kind": "cwd",
                "identity": "/tmp/a",
                "root": "/tmp/a",
                "store_path": "/tmp/a.sqlite",
                "store_id": StoreId::new_v7(),
                "workspace_id": WorkspaceId::new_v7(),
                "branch_id": BranchId::new_v7()
            })]),
            &[]
        )
        .is_err()
    );
}
