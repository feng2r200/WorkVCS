use serde::Deserialize;
use serde_json::{Value, json};
use std::fs;
use std::path::{Path, PathBuf};
use workvcs_core::control_plane::{
    BoundedAdapterContext, ContextLocatorInvocation, ContextLocatorProvider, ControlPlaneDigest,
    LocatorAssurance, LocatorAuthority, LocatorEvidence, LocatorEvidenceExplanation,
    MAX_ADAPTER_CONTEXT_BYTES, PathLocatorEvidence, ResolutionMode, UnifiedLocatorInput,
};
use workvcs_core::{ProjectRefId, Result, WorkVcsError, content_object_digest};

pub(crate) const CODEX_APP_PROJECT_ADAPTER_ID: &str = "codex-app-project-metadata/v1";
const CHATGPT_PROJECT_PROVIDER: &str = "chatgpt";
const CHATGPT_PROJECT_KIND: &str = "chatgpt";
const PROJECT_ID_KIND: &str = "project_id";
const PROJECT_MIRROR_DIR: &str = ".chatgpt-projects";
const REQUIRED_METADATA_SOURCES: [&str; 2] = ["codex_app.list_projects", "codex_app.read_thread"];

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct LocatorAdapterEnvelope {
    schema_version: u64,
    adapter_id: String,
    context: Value,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CodexAppProjectContext {
    codex_home: PathBuf,
    #[serde(default)]
    project_metadata: Option<CodexAppProjectMetadata>,
    #[serde(default)]
    mirror_path: Option<PathBuf>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CodexAppProjectMetadata {
    host_id: String,
    project_id: String,
    project_kind: String,
    thread_id: String,
    verification_sources: Vec<String>,
}

#[derive(Debug)]
struct CodexAppProjectLocatorProvider {
    context_digest: ControlPlaneDigest,
    evidence: Vec<LocatorEvidence>,
    explanations: Vec<LocatorEvidenceExplanation>,
}

impl CodexAppProjectLocatorProvider {
    fn from_context(context: &BoundedAdapterContext) -> Result<Self> {
        let parsed: CodexAppProjectContext = serde_json::from_slice(context.canonical_json_bytes())
            .map_err(|error| {
                WorkVcsError::ControlPlaneInvalid(format!(
                    "{CODEX_APP_PROJECT_ADAPTER_ID} context has an invalid shape: {error}"
                ))
            })?;
        let canonical_home = canonical_directory("Codex home", &parsed.codex_home)?;
        let home_text = utf8_path("canonical Codex home", &canonical_home)?;
        let namespace = codex_installation_namespace(home_text);
        let mut evidence = Vec::new();
        let mut explanations = Vec::new();

        if let Some(metadata) = parsed.project_metadata {
            validate_project_metadata(&metadata)?;
            let evidence_material = json!({
                "host_id": metadata.host_id,
                "project_id": metadata.project_id,
                "project_kind": metadata.project_kind,
                "thread_id": metadata.thread_id,
                "verification_sources": metadata.verification_sources,
            });
            let evidence_digest = canonical_value_digest(&evidence_material)?;
            evidence.push(LocatorEvidence::new(
                LocatorAuthority::SemanticProject,
                CHATGPT_PROJECT_PROVIDER,
                namespace.clone(),
                PROJECT_ID_KIND,
                evidence_material["project_id"]
                    .as_str()
                    .expect("validated metadata project_id"),
                LocatorAssurance::Authoritative,
                CODEX_APP_PROJECT_ADAPTER_ID,
                evidence_digest.clone(),
            )?);
            explanations.push(explanation(
                evidence_digest,
                json!({
                    "assurance": "authoritative",
                    "basis": "verified_task_project_metadata",
                    "project_id": evidence_material["project_id"],
                    "verification_sources": evidence_material["verification_sources"],
                }),
            )?);
        }

        if let Some(mirror_path) = parsed.mirror_path {
            let mirror = canonical_project_mirror(&canonical_home, &mirror_path)?;
            let mirror_text = utf8_path("canonical ChatGPT Project mirror", &mirror)?;
            let project_id = mirror
                .file_name()
                .and_then(|value| value.to_str())
                .ok_or_else(|| {
                    WorkVcsError::ControlPlaneInvalid(
                        "canonical ChatGPT Project mirror name is not valid UTF-8".to_owned(),
                    )
                })?;
            validate_chatgpt_project_id(project_id)?;
            let evidence_material = json!({
                "codex_home": home_text,
                "mirror_path": mirror_text,
                "project_id": project_id,
                "verification_sources": ["canonical_project_mirror_path"],
            });
            let evidence_digest = canonical_value_digest(&evidence_material)?;
            evidence.push(LocatorEvidence::new(
                LocatorAuthority::SemanticProject,
                CHATGPT_PROJECT_PROVIDER,
                namespace,
                PROJECT_ID_KIND,
                project_id,
                LocatorAssurance::VerifiedDerived,
                CODEX_APP_PROJECT_ADAPTER_ID,
                evidence_digest.clone(),
            )?);
            explanations.push(explanation(
                evidence_digest,
                json!({
                    "assurance": "verified_derived",
                    "basis": "canonical_project_mirror_path",
                    "project_id": project_id,
                }),
            )?);
        }

        Ok(Self {
            context_digest: context.digest().clone(),
            evidence,
            explanations,
        })
    }
}

impl ContextLocatorProvider for CodexAppProjectLocatorProvider {
    fn provider_id(&self) -> &str {
        CODEX_APP_PROJECT_ADAPTER_ID
    }

    fn locate(&self, context: &BoundedAdapterContext) -> Result<Vec<LocatorEvidence>> {
        if context.digest() != &self.context_digest {
            return Err(WorkVcsError::ControlPlaneInvalid(format!(
                "{CODEX_APP_PROJECT_ADAPTER_ID} was invoked with a different bounded context"
            )));
        }
        Ok(self.evidence.clone())
    }

    fn explain(&self, evidence_digest: &ControlPlaneDigest) -> Result<LocatorEvidenceExplanation> {
        self.explanations
            .iter()
            .find(|value| value.evidence_digest() == evidence_digest)
            .cloned()
            .ok_or_else(|| {
                WorkVcsError::ControlPlaneInvalid(format!(
                    "{CODEX_APP_PROJECT_ADAPTER_ID} has no explanation for evidence digest {evidence_digest}"
                ))
            })
    }
}

pub(crate) fn collect_locator_input(
    adapter_context_path: Option<&Path>,
    operation_mode: ResolutionMode,
    project_ref_id: Option<ProjectRefId>,
    verified_semantic_evidence: Vec<LocatorEvidence>,
    git_common_dir: Option<PathLocatorEvidence>,
    cwd: Option<PathLocatorEvidence>,
) -> Result<UnifiedLocatorInput> {
    let Some(path) = adapter_context_path else {
        return UnifiedLocatorInput::from_verified_evidence(
            operation_mode,
            project_ref_id,
            verified_semantic_evidence,
            git_common_dir,
            cwd,
        );
    };
    let metadata = fs::metadata(path).map_err(|error| {
        WorkVcsError::QueryInvalid(format!(
            "cannot inspect locator adapter context {}: {error}",
            path.display()
        ))
    })?;
    if metadata.len() > MAX_ADAPTER_CONTEXT_BYTES as u64 {
        return Err(WorkVcsError::ControlPlaneInvalid(format!(
            "locator adapter context {} is {} bytes; maximum is {MAX_ADAPTER_CONTEXT_BYTES}",
            path.display(),
            metadata.len()
        )));
    }
    let bytes = fs::read(path).map_err(|error| {
        WorkVcsError::QueryInvalid(format!(
            "cannot read locator adapter context {}: {error}",
            path.display()
        ))
    })?;
    let envelope_context = BoundedAdapterContext::from_json_bytes(&bytes)?;
    let envelope: LocatorAdapterEnvelope =
        serde_json::from_slice(envelope_context.canonical_json_bytes()).map_err(|error| {
            WorkVcsError::ControlPlaneInvalid(format!(
                "locator adapter context {} has an invalid envelope: {error}",
                path.display()
            ))
        })?;
    if envelope.schema_version != 1 {
        return Err(WorkVcsError::ControlPlaneInvalid(format!(
            "locator adapter context {} has unsupported schema_version {}; expected 1",
            path.display(),
            envelope.schema_version
        )));
    }
    if envelope.adapter_id != CODEX_APP_PROJECT_ADAPTER_ID {
        return Err(WorkVcsError::ControlPlaneInvalid(format!(
            "locator adapter context {} requests unsupported adapter_id {:?}; available adapter is {CODEX_APP_PROJECT_ADAPTER_ID}",
            path.display(),
            envelope.adapter_id
        )));
    }
    let context_bytes = serde_json::to_vec(&envelope.context).map_err(|error| {
        WorkVcsError::ControlPlaneInvalid(format!(
            "cannot encode bounded context for {}: {error}",
            envelope.adapter_id
        ))
    })?;
    let context = BoundedAdapterContext::from_json_bytes(&context_bytes)?;
    let provider = CodexAppProjectLocatorProvider::from_context(&context)?;
    let invocation = ContextLocatorInvocation::new(&provider, &context);
    UnifiedLocatorInput::collect_with_verified_evidence(
        operation_mode,
        project_ref_id,
        &[invocation],
        verified_semantic_evidence,
        git_common_dir,
        cwd,
    )
}

fn canonical_directory(label: &str, path: &Path) -> Result<PathBuf> {
    if !path.is_absolute() {
        return Err(WorkVcsError::ControlPlaneInvalid(format!(
            "{label} must be an absolute path"
        )));
    }
    let canonical = fs::canonicalize(path).map_err(|error| {
        WorkVcsError::ControlPlaneInvalid(format!(
            "cannot canonicalize {label} {}: {error}",
            path.display()
        ))
    })?;
    if !canonical.is_dir() {
        return Err(WorkVcsError::ControlPlaneInvalid(format!(
            "{label} {} is not a directory",
            canonical.display()
        )));
    }
    Ok(canonical)
}

fn canonical_project_mirror(codex_home: &Path, mirror_path: &Path) -> Result<PathBuf> {
    let mirror_root = canonical_directory(
        "ChatGPT Project mirror root",
        &codex_home.join(PROJECT_MIRROR_DIR),
    )?;
    let mirror = canonical_directory("ChatGPT Project mirror", mirror_path)?;
    if mirror.parent() != Some(mirror_root.as_path()) {
        return Err(WorkVcsError::ControlPlaneInvalid(format!(
            "ChatGPT Project mirror {} is not an immediate child of canonical mirror root {}",
            mirror.display(),
            mirror_root.display()
        )));
    }
    Ok(mirror)
}

fn validate_project_metadata(metadata: &CodexAppProjectMetadata) -> Result<()> {
    validate_identity_text("Codex host_id", &metadata.host_id, 128)?;
    validate_chatgpt_project_id(&metadata.project_id)?;
    if metadata.project_kind != CHATGPT_PROJECT_KIND {
        return Err(WorkVcsError::ControlPlaneInvalid(format!(
            "{CODEX_APP_PROJECT_ADAPTER_ID} supports project_kind {CHATGPT_PROJECT_KIND:?}, found {:?}",
            metadata.project_kind
        )));
    }
    validate_thread_id(&metadata.thread_id)?;
    let required = REQUIRED_METADATA_SOURCES.map(str::to_owned).to_vec();
    if metadata.verification_sources != required {
        return Err(WorkVcsError::ControlPlaneInvalid(format!(
            "{CODEX_APP_PROJECT_ADAPTER_ID} authoritative metadata requires the exact ordered verification_sources {:?}",
            REQUIRED_METADATA_SOURCES
        )));
    }
    Ok(())
}

fn validate_identity_text(label: &str, value: &str, maximum: usize) -> Result<()> {
    if value.is_empty()
        || value.len() > maximum
        || !value.is_ascii()
        || value.chars().any(char::is_whitespace)
    {
        return Err(WorkVcsError::ControlPlaneInvalid(format!(
            "{label} must be 1..={maximum} ASCII bytes without whitespace"
        )));
    }
    Ok(())
}

fn validate_chatgpt_project_id(value: &str) -> Result<()> {
    let suffix = value.strip_prefix("g-p-").ok_or_else(|| {
        WorkVcsError::ControlPlaneInvalid(
            "ChatGPT Project ID must use the canonical g-p- prefix".to_owned(),
        )
    })?;
    if suffix.len() != 32
        || !suffix
            .as_bytes()
            .iter()
            .all(|byte| byte.is_ascii_digit() || (*byte >= b'a' && *byte <= b'f'))
    {
        return Err(WorkVcsError::ControlPlaneInvalid(
            "ChatGPT Project ID must contain exactly 32 lowercase hexadecimal characters after g-p-"
                .to_owned(),
        ));
    }
    Ok(())
}

fn validate_thread_id(value: &str) -> Result<()> {
    if value.len() != 36
        || value.as_bytes().get(8) != Some(&b'-')
        || value.as_bytes().get(13) != Some(&b'-')
        || value.as_bytes().get(18) != Some(&b'-')
        || value.as_bytes().get(23) != Some(&b'-')
        || !value.as_bytes().iter().enumerate().all(|(index, byte)| {
            matches!(index, 8 | 13 | 18 | 23)
                || byte.is_ascii_digit()
                || (*byte >= b'a' && *byte <= b'f')
        })
    {
        return Err(WorkVcsError::ControlPlaneInvalid(
            "Codex thread_id must use the canonical lowercase UUID form".to_owned(),
        ));
    }
    Ok(())
}

fn codex_installation_namespace(canonical_home: &str) -> String {
    let digest = content_object_digest(format!("codex-home:{canonical_home}").as_bytes());
    format!("codex-local:{digest}")
}

fn canonical_value_digest(value: &Value) -> Result<ControlPlaneDigest> {
    let bytes = serde_json::to_vec(value).map_err(|error| {
        WorkVcsError::ControlPlaneInvalid(format!(
            "cannot encode locator evidence material: {error}"
        ))
    })?;
    let bounded = BoundedAdapterContext::from_json_bytes(&bytes)?;
    Ok(ControlPlaneDigest::raw(bounded.canonical_json_bytes()))
}

fn explanation(
    evidence_digest: ControlPlaneDigest,
    value: Value,
) -> Result<LocatorEvidenceExplanation> {
    let bytes = serde_json::to_vec(&value).map_err(|error| {
        WorkVcsError::ControlPlaneInvalid(format!(
            "cannot encode locator evidence explanation: {error}"
        ))
    })?;
    LocatorEvidenceExplanation::from_json_bytes(evidence_digest, &bytes)
}

fn utf8_path<'a>(label: &str, path: &'a Path) -> Result<&'a str> {
    path.to_str()
        .ok_or_else(|| WorkVcsError::ControlPlaneInvalid(format!("{label} is not valid UTF-8")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use workvcs_core::control_plane::{
        ResolutionDiagnostic, ResolutionStatus, resolve_unbound_project,
    };

    const FIXTURE_PROJECT_A: &str = "g-p-0123456789abcdef0123456789abcdef";
    const FIXTURE_PROJECT_B: &str = "g-p-fedcba9876543210fedcba9876543210";
    const FIXTURE_THREAD_ID: &str = "01900000-0000-7000-8000-000000000001";

    fn metadata(project_id: &str) -> Value {
        json!({
            "host_id": "local",
            "project_id": project_id,
            "project_kind": "chatgpt",
            "thread_id": FIXTURE_THREAD_ID,
            "verification_sources": REQUIRED_METADATA_SOURCES,
        })
    }

    fn adapter_file(
        root: &Path,
        project_metadata: Option<Value>,
        mirror_path: Option<&Path>,
    ) -> PathBuf {
        let value = json!({
            "schema_version": 1,
            "adapter_id": CODEX_APP_PROJECT_ADAPTER_ID,
            "context": {
                "codex_home": root,
                "project_metadata": project_metadata,
                "mirror_path": mirror_path,
            }
        });
        let path = root.join("adapter-context.json");
        fs::write(&path, serde_json::to_vec_pretty(&value).unwrap()).unwrap();
        path
    }

    #[test]
    fn accepted_installation_and_metadata_digest_contract_remains_reproducible() {
        assert_eq!(
            codex_installation_namespace("/Users/example/.codex"),
            "codex-local:2c8a758af16948c70ce1b58b3fc83518b38abe04afec760399fc124264066f40"
        );
        let digest = canonical_value_digest(&metadata(FIXTURE_PROJECT_A)).unwrap();
        assert_eq!(
            digest.to_string(),
            "blake3-256:492eed9c3e0c0032938b1b43d479d01d2e16366d3b97c5726ab08ebdcb1c5395"
        );
    }

    #[test]
    fn metadata_and_mirror_agree_and_remain_separately_explainable() {
        let tempdir = tempfile::tempdir().unwrap();
        let project_id = FIXTURE_PROJECT_A;
        let mirror = tempdir.path().join(PROJECT_MIRROR_DIR).join(project_id);
        fs::create_dir_all(&mirror).unwrap();
        let path = adapter_file(tempdir.path(), Some(metadata(project_id)), Some(&mirror));
        let input = collect_locator_input(
            Some(&path),
            ResolutionMode::ReadOnly,
            None,
            Vec::new(),
            None,
            None,
        )
        .unwrap();
        assert_eq!(input.provider_ids(), &[CODEX_APP_PROJECT_ADAPTER_ID]);
        assert_eq!(input.resolution_context().locator_evidence().len(), 2);
        assert_eq!(input.explanations().len(), 2);
        assert_eq!(
            input.resolution_context().locator_evidence()[0].assurance(),
            LocatorAssurance::Authoritative
        );
        assert_eq!(
            input.resolution_context().locator_evidence()[1].assurance(),
            LocatorAssurance::VerifiedDerived
        );
        let resolution =
            resolve_unbound_project(input.resolution_context(), "registry:fixture").unwrap();
        assert_eq!(resolution.status(), ResolutionStatus::Unbound);
        assert!(resolution.diagnostics().is_empty());
    }

    #[test]
    fn authoritative_metadata_wins_a_conflicting_mirror_with_mismatch_diagnostic() {
        let tempdir = tempfile::tempdir().unwrap();
        let authoritative = FIXTURE_PROJECT_A;
        let derived = FIXTURE_PROJECT_B;
        let mirror = tempdir.path().join(PROJECT_MIRROR_DIR).join(derived);
        fs::create_dir_all(&mirror).unwrap();
        let path = adapter_file(tempdir.path(), Some(metadata(authoritative)), Some(&mirror));
        let input = collect_locator_input(
            Some(&path),
            ResolutionMode::ReadOnly,
            None,
            Vec::new(),
            None,
            None,
        )
        .unwrap();
        let resolution =
            resolve_unbound_project(input.resolution_context(), "registry:fixture").unwrap();
        assert_eq!(resolution.status(), ResolutionStatus::Unbound);
        assert_eq!(
            resolution.diagnostics(),
            &[ResolutionDiagnostic::ContextMismatch]
        );
        assert_eq!(
            resolution.primary_basis().and_then(|basis| match basis {
                workvcs_core::control_plane::ResolutionBasis::Locator { evidence, .. } => {
                    Some(evidence.normalized_value())
                }
                _ => None,
            }),
            Some(authoritative)
        );
    }

    #[test]
    fn unavailable_metadata_degrades_and_explicit_bad_context_fails_closed() {
        let tempdir = tempfile::tempdir().unwrap();
        let path = adapter_file(tempdir.path(), None, None);
        let input = collect_locator_input(
            Some(&path),
            ResolutionMode::ReadOnly,
            None,
            Vec::new(),
            None,
            None,
        )
        .unwrap();
        assert!(input.resolution_context().locator_evidence().is_empty());
        assert_eq!(input.provider_ids(), &[CODEX_APP_PROJECT_ADAPTER_ID]);

        let outside = tempfile::tempdir().unwrap();
        let bad_project = outside.path().join(FIXTURE_PROJECT_A);
        fs::create_dir_all(&bad_project).unwrap();
        let bad_path = adapter_file(tempdir.path(), None, Some(&bad_project));
        let error = collect_locator_input(
            Some(&bad_path),
            ResolutionMode::ReadOnly,
            None,
            Vec::new(),
            None,
            None,
        )
        .unwrap_err();
        assert!(error.to_string().contains("Project mirror root"));
    }
}
