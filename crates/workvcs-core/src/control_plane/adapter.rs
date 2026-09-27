use super::model::{
    ControlPlaneDigest, LocatorAuthority, LocatorEvidence, PathLocatorEvidence, ResolutionContext,
    ResolutionMode, invalid,
};
use super::safety::contains_secret_bearing_field;
use crate::ProjectRefId;
use crate::canonical::{CanonicalValue, canonical_bytes, parse_canonical_json};
use crate::error::{Result, WorkVcsError};
use serde_json::Value;
use std::collections::BTreeSet;

pub const MAX_ADAPTER_CONTEXT_BYTES: usize = 64 * 1_024;
pub const MAX_ADAPTER_EXPLANATION_BYTES: usize = 4 * 1_024;
pub const MAX_ADAPTER_EVIDENCE_ITEMS: usize = 128;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BoundedAdapterContext {
    value: Value,
    canonical_bytes: Vec<u8>,
    digest: ControlPlaneDigest,
}

impl BoundedAdapterContext {
    pub fn from_json_bytes(input: &[u8]) -> Result<Self> {
        if input.len() > MAX_ADAPTER_CONTEXT_BYTES {
            return invalid(format!(
                "adapter context is {} bytes; maximum is {MAX_ADAPTER_CONTEXT_BYTES}",
                input.len()
            ));
        }
        let (value, canonical_bytes) =
            bounded_non_secret_object(input, MAX_ADAPTER_CONTEXT_BYTES, "adapter context")?;
        let digest = ControlPlaneDigest::raw(&canonical_bytes);
        Ok(Self {
            value,
            canonical_bytes,
            digest,
        })
    }

    pub fn value(&self) -> &Value {
        &self.value
    }

    pub fn canonical_json_bytes(&self) -> &[u8] {
        &self.canonical_bytes
    }

    pub fn digest(&self) -> &ControlPlaneDigest {
        &self.digest
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LocatorEvidenceExplanation {
    evidence_digest: ControlPlaneDigest,
    summary: Value,
    canonical_bytes: Vec<u8>,
}

impl LocatorEvidenceExplanation {
    pub fn from_json_bytes(evidence_digest: ControlPlaneDigest, input: &[u8]) -> Result<Self> {
        if input.len() > MAX_ADAPTER_EXPLANATION_BYTES {
            return invalid(format!(
                "adapter explanation is {} bytes; maximum is {MAX_ADAPTER_EXPLANATION_BYTES}",
                input.len()
            ));
        }
        let (summary, canonical_bytes) =
            bounded_non_secret_object(input, MAX_ADAPTER_EXPLANATION_BYTES, "adapter explanation")?;
        Ok(Self {
            evidence_digest,
            summary,
            canonical_bytes,
        })
    }

    pub fn evidence_digest(&self) -> &ControlPlaneDigest {
        &self.evidence_digest
    }

    pub fn summary(&self) -> &Value {
        &self.summary
    }

    pub fn canonical_json_bytes(&self) -> &[u8] {
        &self.canonical_bytes
    }
}

pub trait ContextLocatorProvider {
    fn provider_id(&self) -> &str;

    fn locate(&self, context: &BoundedAdapterContext) -> Result<Vec<LocatorEvidence>>;

    fn explain(&self, evidence_digest: &ControlPlaneDigest) -> Result<LocatorEvidenceExplanation>;
}

pub struct ContextLocatorInvocation<'a> {
    provider: &'a dyn ContextLocatorProvider,
    context: &'a BoundedAdapterContext,
}

impl<'a> ContextLocatorInvocation<'a> {
    pub fn new(
        provider: &'a dyn ContextLocatorProvider,
        context: &'a BoundedAdapterContext,
    ) -> Self {
        Self { provider, context }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnifiedLocatorInput {
    resolution_context: ResolutionContext,
    provider_ids: Vec<String>,
    explanations: Vec<LocatorEvidenceExplanation>,
}

impl UnifiedLocatorInput {
    #[allow(clippy::too_many_arguments)]
    pub fn collect(
        operation_mode: ResolutionMode,
        project_ref_id: Option<ProjectRefId>,
        invocations: &[ContextLocatorInvocation<'_>],
        git_common_dir: Option<PathLocatorEvidence>,
        cwd: Option<PathLocatorEvidence>,
    ) -> Result<Self> {
        Self::collect_with_verified_evidence(
            operation_mode,
            project_ref_id,
            invocations,
            Vec::new(),
            git_common_dir,
            cwd,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn collect_with_verified_evidence(
        operation_mode: ResolutionMode,
        project_ref_id: Option<ProjectRefId>,
        invocations: &[ContextLocatorInvocation<'_>],
        verified_semantic_evidence: Vec<LocatorEvidence>,
        git_common_dir: Option<PathLocatorEvidence>,
        cwd: Option<PathLocatorEvidence>,
    ) -> Result<Self> {
        let mut provider_id_set = BTreeSet::new();
        let mut evidence = Vec::new();
        let mut explanations = Vec::new();

        for invocation in invocations {
            let provider_id = invocation.provider.provider_id();
            validate_provider_id(provider_id)?;
            if !provider_id_set.insert(provider_id.to_owned()) {
                return invalid(format!(
                    "context locator provider {provider_id:?} was invoked more than once"
                ));
            }
            let located = invocation.provider.locate(invocation.context)?;
            if evidence.len().saturating_add(located.len()) > MAX_ADAPTER_EVIDENCE_ITEMS {
                return invalid(format!(
                    "adapter collection exceeds {MAX_ADAPTER_EVIDENCE_ITEMS} LocatorEvidence items"
                ));
            }
            for item in located {
                item.validate()?;
                if item.source_adapter() != provider_id {
                    return invalid(format!(
                        "adapter {provider_id:?} emitted LocatorEvidence for source_adapter {:?}",
                        item.source_adapter()
                    ));
                }
                if item.authority() != LocatorAuthority::SemanticProject {
                    return invalid(format!(
                        "adapter {provider_id:?} emitted non-semantic authority {:?}; Git and CWD use the dedicated verified path inputs",
                        item.authority()
                    ));
                }
                let explanation = invocation.provider.explain(item.evidence_digest())?;
                if explanation.evidence_digest() != item.evidence_digest() {
                    return invalid(format!(
                        "adapter {provider_id:?} explanation digest does not match its LocatorEvidence"
                    ));
                }
                explanations.push(explanation);
                evidence.push(item);
            }
        }

        if evidence
            .len()
            .saturating_add(verified_semantic_evidence.len())
            > MAX_ADAPTER_EVIDENCE_ITEMS
        {
            return invalid(format!(
                "adapter collection exceeds {MAX_ADAPTER_EVIDENCE_ITEMS} LocatorEvidence items"
            ));
        }
        for item in verified_semantic_evidence {
            item.validate()?;
            validate_provider_id(item.source_adapter())?;
            if item.authority() != LocatorAuthority::SemanticProject {
                return invalid(
                    "verified adapter input may contain only semantic locator evidence; Git and CWD use the dedicated verified path inputs",
                );
            }
            provider_id_set.insert(item.source_adapter().to_owned());
            evidence.push(item);
        }

        let provider_ids = provider_id_set.into_iter().collect();
        evidence.sort_by(|left, right| {
            adapter_evidence_sort_key(left).cmp(&adapter_evidence_sort_key(right))
        });
        evidence.dedup();
        explanations.sort_by(|left, right| {
            left.evidence_digest()
                .to_string()
                .cmp(&right.evidence_digest().to_string())
                .then_with(|| {
                    left.canonical_json_bytes()
                        .cmp(right.canonical_json_bytes())
                })
        });
        explanations.dedup();

        let mut resolution_context = ResolutionContext::new(operation_mode);
        if let Some(project_ref_id) = project_ref_id {
            resolution_context = resolution_context.with_project_ref(project_ref_id);
        }
        for item in evidence {
            resolution_context = resolution_context.with_locator_evidence(item);
        }
        if let Some(git_common_dir) = git_common_dir {
            resolution_context = resolution_context.with_git_common_dir(git_common_dir);
        }
        if let Some(cwd) = cwd {
            resolution_context = resolution_context.with_cwd(cwd);
        }

        Ok(Self {
            resolution_context,
            provider_ids,
            explanations,
        })
    }

    pub fn from_verified_evidence(
        operation_mode: ResolutionMode,
        project_ref_id: Option<ProjectRefId>,
        semantic_evidence: Vec<LocatorEvidence>,
        git_common_dir: Option<PathLocatorEvidence>,
        cwd: Option<PathLocatorEvidence>,
    ) -> Result<Self> {
        Self::collect_with_verified_evidence(
            operation_mode,
            project_ref_id,
            &[],
            semantic_evidence,
            git_common_dir,
            cwd,
        )
    }

    pub fn resolution_context(&self) -> &ResolutionContext {
        &self.resolution_context
    }

    pub fn provider_ids(&self) -> &[String] {
        &self.provider_ids
    }

    pub fn explanations(&self) -> &[LocatorEvidenceExplanation] {
        &self.explanations
    }
}

fn bounded_non_secret_object(
    input: &[u8],
    maximum_bytes: usize,
    label: &str,
) -> Result<(Value, Vec<u8>)> {
    let canonical = parse_canonical_json(input).map_err(|error| {
        WorkVcsError::ControlPlaneInvalid(format!("{label} is not strict JSON: {error}"))
    })?;
    if !matches!(canonical, CanonicalValue::Object(_)) {
        return invalid(format!("{label} must be a JSON object"));
    }
    let canonical_bytes = canonical_bytes(&canonical).map_err(|error| {
        WorkVcsError::ControlPlaneInvalid(format!("cannot encode canonical {label}: {error}"))
    })?;
    if canonical_bytes.len() > maximum_bytes {
        return invalid(format!(
            "canonical {label} is {} bytes; maximum is {maximum_bytes}",
            canonical_bytes.len()
        ));
    }
    let value: Value = serde_json::from_slice(&canonical_bytes).map_err(|error| {
        WorkVcsError::ControlPlaneInvalid(format!("cannot decode canonical {label}: {error}"))
    })?;
    if contains_secret_bearing_field(&value) {
        return invalid(format!(
            "{label} contains a known secret-bearing field; redact it before adapter use"
        ));
    }
    Ok((value, canonical_bytes))
}

fn validate_provider_id(provider_id: &str) -> Result<()> {
    if provider_id.is_empty()
        || provider_id.len() > 128
        || !provider_id.is_ascii()
        || provider_id.chars().any(char::is_whitespace)
    {
        return invalid(
            "context locator provider_id must be 1..=128 ASCII bytes without whitespace",
        );
    }
    Ok(())
}

fn adapter_evidence_sort_key(
    evidence: &LocatorEvidence,
) -> (u8, u8, &str, &str, &str, &str, &str, String) {
    (
        evidence.authority().rank(),
        u8::MAX - evidence.assurance().priority(),
        evidence.provider(),
        evidence.namespace(),
        evidence.kind(),
        evidence.normalized_value(),
        evidence.source_adapter(),
        evidence.evidence_digest().to_string(),
    )
}
