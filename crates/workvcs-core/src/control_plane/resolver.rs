use super::model::{
    LocatorAssurance, LocatorAuthority, LocatorEvidence, LocatorKey, ProjectRegistryV2,
    ResolutionContext, invalid,
};
use crate::ProjectRefId;
use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResolutionStatus {
    Resolved,
    Unbound,
    Unresolved,
    Conflict,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResolutionRank {
    ExplicitProjectRef,
    SemanticProject,
    Repository,
    Cwd,
}

impl ResolutionRank {
    fn from_authority(authority: LocatorAuthority) -> Self {
        match authority {
            LocatorAuthority::SemanticProject => Self::SemanticProject,
            LocatorAuthority::Repository => Self::Repository,
            LocatorAuthority::Cwd => Self::Cwd,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResolutionDiagnostic {
    ProjectRefNotFound,
    ContextMismatch,
    OwnershipConflict,
    RepositoryFallback,
    CwdFallback,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ResolutionBasis {
    ExplicitProjectRef {
        project_ref_id: ProjectRefId,
    },
    Locator {
        rank: ResolutionRank,
        project_ref_id: Option<ProjectRefId>,
        evidence: LocatorEvidence,
    },
}

impl ResolutionBasis {
    pub fn project_ref_id(&self) -> Option<ProjectRefId> {
        match self {
            Self::ExplicitProjectRef { project_ref_id } => Some(*project_ref_id),
            Self::Locator { project_ref_id, .. } => *project_ref_id,
        }
    }

    pub fn rank(&self) -> ResolutionRank {
        match self {
            Self::ExplicitProjectRef { .. } => ResolutionRank::ExplicitProjectRef,
            Self::Locator { rank, .. } => *rank,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResolutionResult {
    status: ResolutionStatus,
    primary_project_ref: Option<ProjectRefId>,
    primary_basis: Option<ResolutionBasis>,
    related_project_refs: Vec<ProjectRefId>,
    unmapped_locators: Vec<LocatorEvidence>,
    diagnostics: Vec<ResolutionDiagnostic>,
}

impl ResolutionResult {
    pub fn status(&self) -> ResolutionStatus {
        self.status
    }

    pub fn primary_project_ref(&self) -> Option<ProjectRefId> {
        self.primary_project_ref
    }

    pub fn primary_basis(&self) -> Option<&ResolutionBasis> {
        self.primary_basis.as_ref()
    }

    pub fn related_project_refs(&self) -> &[ProjectRefId] {
        &self.related_project_refs
    }

    pub fn unmapped_locators(&self) -> &[LocatorEvidence] {
        &self.unmapped_locators
    }

    pub fn diagnostics(&self) -> &[ResolutionDiagnostic] {
        &self.diagnostics
    }

    pub(crate) fn validate(&self) -> Result<()> {
        if let Some(ResolutionBasis::Locator {
            rank,
            project_ref_id: _,
            evidence,
        }) = &self.primary_basis
        {
            evidence.validate()?;
            if *rank != ResolutionRank::from_authority(evidence.authority) {
                return invalid("locator resolution basis rank must match its evidence authority");
            }
        }
        match self.status {
            ResolutionStatus::Resolved => {
                if self.primary_project_ref.is_none() || self.primary_basis.is_none() {
                    return invalid(
                        "resolved result requires primary_project_ref and primary_basis",
                    );
                }
                if self
                    .primary_basis
                    .as_ref()
                    .and_then(ResolutionBasis::project_ref_id)
                    != self.primary_project_ref
                {
                    return invalid("resolved result primary basis must name primary_project_ref");
                }
            }
            ResolutionStatus::Unbound => {
                if self.primary_project_ref.is_some() {
                    return invalid("unbound result must not name a primary ProjectRef");
                }
                let Some(ResolutionBasis::Locator {
                    project_ref_id,
                    evidence,
                    ..
                }) = &self.primary_basis
                else {
                    return invalid("unbound result requires an unmapped locator basis");
                };
                if project_ref_id.is_some() {
                    return invalid("unbound locator basis must not name a ProjectRef");
                }
                if !self.unmapped_locators.contains(evidence) {
                    return invalid("unbound primary basis must be retained in unmapped_locators");
                }
            }
            ResolutionStatus::Unresolved | ResolutionStatus::Conflict => {
                if self.primary_project_ref.is_some() || self.primary_basis.is_some() {
                    return invalid("unresolved or conflict result must not name a primary owner");
                }
            }
        }
        require_strictly_sorted(&self.related_project_refs, "related_project_refs")?;
        if self
            .primary_project_ref
            .is_some_and(|primary| self.related_project_refs.contains(&primary))
        {
            return invalid("related_project_refs must not repeat the primary ProjectRef");
        }
        let mut unmapped_keys = BTreeSet::new();
        for evidence in &self.unmapped_locators {
            evidence.validate()?;
            if !unmapped_keys.insert(evidence.key()) {
                return invalid("unmapped_locators must not contain duplicate locator keys");
            }
        }
        for pair in self.unmapped_locators.windows(2) {
            if evidence_sort_key(&pair[0]) >= evidence_sort_key(&pair[1]) {
                return invalid("unmapped_locators must use stable resolver order");
            }
        }
        require_strictly_sorted(&self.diagnostics, "resolution diagnostics")?;
        Ok(())
    }
}

#[derive(Clone)]
struct Candidate {
    evidence: LocatorEvidence,
    project_ref_id: Option<ProjectRefId>,
}

pub fn resolve_project(
    registry: &ProjectRegistryV2,
    context: &ResolutionContext,
) -> Result<ResolutionResult> {
    registry.validate()?;
    let mut evidence = context.all_evidence(registry.registry_id())?;
    evidence.sort_by_key(evidence_sort_key);
    evidence.dedup_by(|left, right| left.key() == right.key());

    let mapped = evidence
        .iter()
        .cloned()
        .map(|evidence| Candidate {
            project_ref_id: registry.active_identity_project(&evidence.key()),
            evidence,
        })
        .collect::<Vec<_>>();
    resolve_candidates(context, mapped, |project_ref_id| {
        registry.contains_project(project_ref_id)
    })
}

pub fn resolve_unbound_project(
    context: &ResolutionContext,
    path_namespace: impl Into<String>,
) -> Result<ResolutionResult> {
    let mut evidence = context.all_evidence_in_namespace(path_namespace)?;
    evidence.sort_by_key(evidence_sort_key);
    evidence.dedup_by(|left, right| left.key() == right.key());
    let mapped = evidence
        .into_iter()
        .map(|evidence| Candidate {
            evidence,
            project_ref_id: None,
        })
        .collect::<Vec<_>>();
    resolve_candidates(context, mapped, |_| false)
}

fn resolve_candidates<F>(
    context: &ResolutionContext,
    mapped: Vec<Candidate>,
    project_exists: F,
) -> Result<ResolutionResult>
where
    F: Fn(ProjectRefId) -> bool,
{
    let unmapped_locators = mapped
        .iter()
        .filter(|candidate| candidate.project_ref_id.is_none())
        .map(|candidate| candidate.evidence.clone())
        .collect::<Vec<_>>();

    if let Some(project_ref_id) = context.project_ref_id {
        if project_exists(project_ref_id) {
            let result = ResolutionResult {
                status: ResolutionStatus::Resolved,
                primary_project_ref: Some(project_ref_id),
                primary_basis: Some(ResolutionBasis::ExplicitProjectRef { project_ref_id }),
                related_project_refs: related_projects(&mapped, Some(project_ref_id)),
                unmapped_locators,
                diagnostics: Vec::new(),
            };
            result.validate()?;
            return Ok(result);
        }
        let result = ResolutionResult {
            status: ResolutionStatus::Conflict,
            primary_project_ref: None,
            primary_basis: None,
            related_project_refs: related_projects(&mapped, None),
            unmapped_locators,
            diagnostics: vec![ResolutionDiagnostic::ProjectRefNotFound],
        };
        result.validate()?;
        return Ok(result);
    }

    let mut diagnostics = mismatch_diagnostics(&mapped);
    let eligible = mapped
        .iter()
        .filter(|candidate| candidate.evidence.assurance != LocatorAssurance::Observed)
        .cloned()
        .collect::<Vec<_>>();
    let Some(highest_rank) = eligible
        .iter()
        .map(|candidate| candidate.evidence.authority.rank())
        .min()
    else {
        let result = ResolutionResult {
            status: ResolutionStatus::Unresolved,
            primary_project_ref: None,
            primary_basis: None,
            related_project_refs: related_projects(&mapped, None),
            unmapped_locators,
            diagnostics,
        };
        result.validate()?;
        return Ok(result);
    };

    let ranked = eligible
        .into_iter()
        .filter(|candidate| candidate.evidence.authority.rank() == highest_rank)
        .collect::<Vec<_>>();
    let mut highest_provider_assurance = BTreeMap::<(String, String), u8>::new();
    for candidate in &ranked {
        highest_provider_assurance
            .entry((
                candidate.evidence.provider().to_owned(),
                candidate.evidence.namespace().to_owned(),
            ))
            .and_modify(|priority| {
                *priority = (*priority).max(candidate.evidence.assurance.priority());
            })
            .or_insert(candidate.evidence.assurance.priority());
    }
    let mut winners_by_key = BTreeMap::<LocatorKey, Candidate>::new();
    for candidate in ranked.into_iter().filter(|candidate| {
        highest_provider_assurance.get(&(
            candidate.evidence.provider().to_owned(),
            candidate.evidence.namespace().to_owned(),
        )) == Some(&candidate.evidence.assurance.priority())
    }) {
        winners_by_key
            .entry(candidate.evidence.key())
            .or_insert(candidate);
    }
    let winners = winners_by_key.into_values().collect::<Vec<_>>();
    let distinct_projects = winners
        .iter()
        .map(|candidate| candidate.project_ref_id)
        .collect::<BTreeSet<_>>();

    let authority = winners[0].evidence.authority;
    match authority {
        LocatorAuthority::Repository => diagnostics.push(ResolutionDiagnostic::RepositoryFallback),
        LocatorAuthority::Cwd => diagnostics.push(ResolutionDiagnostic::CwdFallback),
        LocatorAuthority::SemanticProject => {}
    }

    let (status, primary_project_ref, primary_basis) = if distinct_projects.len() == 1
        && (distinct_projects.first().is_some_and(Option::is_some) || winners.len() == 1)
    {
        let project_ref_id = *distinct_projects.first().expect("winners are not empty");
        let basis = ResolutionBasis::Locator {
            rank: ResolutionRank::from_authority(authority),
            project_ref_id,
            evidence: winners[0].evidence.clone(),
        };
        match project_ref_id {
            Some(project_ref_id) => (
                ResolutionStatus::Resolved,
                Some(project_ref_id),
                Some(basis),
            ),
            None => (ResolutionStatus::Unbound, None, Some(basis)),
        }
    } else {
        diagnostics.push(ResolutionDiagnostic::OwnershipConflict);
        (ResolutionStatus::Conflict, None, None)
    };
    diagnostics.sort();
    diagnostics.dedup();
    let result = ResolutionResult {
        status,
        primary_project_ref,
        primary_basis,
        related_project_refs: related_projects(&mapped, primary_project_ref),
        unmapped_locators,
        diagnostics,
    };
    result.validate()?;
    Ok(result)
}

fn mismatch_diagnostics(candidates: &[Candidate]) -> Vec<ResolutionDiagnostic> {
    let semantic = candidates
        .iter()
        .filter(|candidate| candidate.evidence.authority == LocatorAuthority::SemanticProject)
        .collect::<Vec<_>>();
    for authoritative in semantic
        .iter()
        .filter(|candidate| candidate.evidence.assurance == LocatorAssurance::Authoritative)
    {
        if semantic.iter().any(|derived| {
            derived.evidence.assurance == LocatorAssurance::VerifiedDerived
                && derived.evidence.provider() == authoritative.evidence.provider()
                && derived.evidence.namespace() == authoritative.evidence.namespace()
                && derived.evidence.key() != authoritative.evidence.key()
        }) {
            return vec![ResolutionDiagnostic::ContextMismatch];
        }
    }
    Vec::new()
}

fn related_projects(
    candidates: &[Candidate],
    primary_project_ref: Option<ProjectRefId>,
) -> Vec<ProjectRefId> {
    candidates
        .iter()
        .filter_map(|candidate| candidate.project_ref_id)
        .filter(|project_ref_id| Some(*project_ref_id) != primary_project_ref)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn evidence_sort_key(evidence: &LocatorEvidence) -> (u8, std::cmp::Reverse<u8>, LocatorKey) {
    (
        evidence.authority.rank(),
        std::cmp::Reverse(evidence.assurance.priority()),
        evidence.key(),
    )
}

fn require_strictly_sorted<T: Ord + std::fmt::Debug>(values: &[T], label: &str) -> Result<()> {
    for pair in values.windows(2) {
        if pair[0] >= pair[1] {
            return invalid(format!(
                "{label} must be strictly sorted; found {:?} before {:?}",
                pair[0], pair[1]
            ));
        }
    }
    Ok(())
}
