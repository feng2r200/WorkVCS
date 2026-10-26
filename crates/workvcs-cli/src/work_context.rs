//! Resolve an explicit work selection inside an already verified Project owner.

use clap::Args;
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};
use workvcs_core::control_plane::CaptureWorkContext;
use workvcs_core::{
    BranchId, CanonicalValue, CommitId, CurrentnessCarrierKind, Digest, Engine, EntityId,
    EvidenceId, EvidenceListOptions, EvidenceSnapshot, KnowledgeStatus, RecordKind, Result,
    SemanticSnapshot, SemanticSnapshotOptions, SessionId, SessionLifecycleState, WorkApplicability,
    WorkVcsError, WorkspaceId, is_project_fact_scope, parse_canonical_json,
    resolve_work_applicability,
};

#[derive(Clone, Debug, Default, Args)]
pub(crate) struct WorkSelectionArgs {
    /// Existing native Session whose active Branch should be used.
    #[arg(long)]
    pub session: Option<String>,
    /// Explicit existing Work Branch, independent of the Git branch.
    #[arg(long)]
    pub branch: Option<String>,
    /// Existing Task to recover or associate with new standalone records.
    #[arg(long)]
    pub task: Option<String>,
    /// Structural subset selector for work-specific record scope.
    #[arg(long)]
    pub scope_json: Option<String>,
}

#[derive(Clone, Debug)]
pub(crate) struct ResolvedWorkSelection {
    pub context: CaptureWorkContext,
    pub applicability: WorkApplicability,
}

impl WorkSelectionArgs {
    pub fn is_explicit(&self) -> bool {
        self.session.is_some()
            || self.branch.is_some()
            || self.task.is_some()
            || self.scope_json.is_some()
    }

    pub fn resolve(
        &self,
        engine: &Engine,
        workspace_id: WorkspaceId,
        default_branch_id: BranchId,
    ) -> Result<ResolvedWorkSelection> {
        let session_id = self
            .session
            .as_deref()
            .map(SessionId::parse_canonical)
            .transpose()?;
        let session = session_id
            .map(|id| engine.session_snapshot(id))
            .transpose()?;
        if let Some(session) = &session
            && (session.lifecycle_state != SessionLifecycleState::Active
                || session.active_workspace_id != Some(workspace_id))
        {
            return Err(WorkVcsError::QueryInvalid(
                "selected Session must be active in the resolved Project Workspace".to_owned(),
            ));
        }
        let explicit_branch = self
            .branch
            .as_deref()
            .map(BranchId::parse_canonical)
            .transpose()?;
        let session_branch = session.as_ref().and_then(|value| value.active_branch_id);
        if let (Some(explicit), Some(active)) = (explicit_branch, session_branch)
            && explicit != active
        {
            return Err(WorkVcsError::QueryInvalid(
                "explicit Work Branch conflicts with the selected Session; switch the Session explicitly before continuing".to_owned(),
            ));
        }
        let branch_id = session_branch
            .or(explicit_branch)
            .unwrap_or(default_branch_id);
        let branch = engine.branch_head(branch_id)?;
        if branch.workspace_id != workspace_id || branch.lifecycle_state != "active" {
            return Err(WorkVcsError::QueryInvalid(
                "selected Work Branch must be active in the resolved Project Workspace".to_owned(),
            ));
        }
        let task = self
            .task
            .as_deref()
            .map(EntityId::parse_canonical)
            .transpose()?;
        let tasks = engine.tasks_at(branch.head_commit_id)?;
        let focus = session
            .as_ref()
            .and_then(|value| value.focus.as_ref().map(|focus| focus.focus_entity_id));
        let scope = self
            .scope_json
            .as_deref()
            .map(|json| {
                let value = parse_canonical_json(json.as_bytes())?;
                if !matches!(&value, CanonicalValue::Object(entries) if !entries.is_empty()) {
                    return Err(WorkVcsError::QueryInvalid(
                        "--scope-json must be a nonempty object identifying the current work"
                            .to_owned(),
                    ));
                }
                Ok(value)
            })
            .transpose()?;
        let task_ids = tasks.iter().map(|task| task.task_entity_id).collect();
        let applicability = resolve_work_applicability(scope.as_ref(), task, focus, &task_ids)?;
        if let Some(entity) = applicability.entity_id()
            && !task_ids.contains(&entity)
            && !engine
                .plans_at(branch.head_commit_id)?
                .iter()
                .any(|plan| plan.plan_entity_id == entity)
            && !engine
                .goals_at(branch.head_commit_id)?
                .iter()
                .any(|goal| goal.goal_entity_id == entity)
        {
            return Err(WorkVcsError::QueryInvalid(format!(
                "selected work entity {entity} is absent from the selected Branch"
            )));
        }
        Ok(ResolvedWorkSelection {
            context: CaptureWorkContext {
                workspace_id,
                branch_id,
                session_id,
                focus_entity_id: focus,
            },
            applicability,
        })
    }
}

pub(crate) fn merge_scope_value(
    target: &mut CanonicalValue,
    addition: &CanonicalValue,
) -> Result<()> {
    match (target, addition) {
        (CanonicalValue::Object(target), CanonicalValue::Object(addition)) => {
            for (key, value) in addition {
                if let Some((_, current)) = target.iter_mut().find(|(candidate, _)| candidate == key) {
                    merge_scope_value(current, value)?;
                } else {
                    target.push((key.clone(), value.clone()));
                }
            }
            target.sort_by(|left, right| left.0.cmp(&right.0));
            Ok(())
        }
        (current, addition) if current == addition => Ok(()),
        _ => Err(WorkVcsError::QueryInvalid(
            "record scope conflicts with the explicit work selection; correct the scope before admission".to_owned(),
        )),
    }
}

pub(crate) fn context_basis_digest(
    discovery: &super::ProjectDiscovery,
    selected: &ResolvedWorkSelection,
    profile: super::RecallProfileArg,
    snapshot: &SemanticSnapshot,
    evidence: &EvidenceProjection,
) -> Result<Digest> {
    let projected_versions = versions(snapshot).into_iter().map(|((category, id), version)| {
        serde_json::json!({"category": category, "id": id, "version_id": version})
    }).collect::<Vec<_>>();
    let value = serde_json::json!({
        "project_ref_id": discovery.project_ref_id,
        "store_id": discovery.binding.store_id,
        "work_context": selected.context,
        "record_scope": selected.applicability.selector.as_ref().map(canonical_json_value).transpose()?,
        "profile": profile.as_str(),
        "base_commit_id": snapshot.commit_id,
        "projected_versions": projected_versions,
        "projected_evidence": evidence.basis,
    });
    let bytes = serde_json::to_vec(&value).map_err(|error| {
        WorkVcsError::CanonicalEncodingInvalid(format!("cannot encode recovery selection: {error}"))
    })?;
    Ok(Digest::domain_separated(
        "workvcs.recovery-basis.v1",
        &bytes,
    ))
}

#[derive(Default)]
pub(crate) struct EvidenceProjection {
    pub items: Vec<EvidenceSnapshot>,
    basis: BTreeMap<EvidenceId, serde_json::Value>,
}

#[derive(Deserialize)]
struct CapturedEvidenceEnvelope {
    workspace_id: WorkspaceId,
    branch_id: BranchId,
    commit_id: CommitId,
    evidence: Vec<CapturedEvidenceReference>,
}

#[derive(Deserialize)]
struct CapturedEvidenceReference {
    evidence_id: EvidenceId,
}

/// Follow immutable commit ancestry instead of treating an origin Branch as permanent ownership.
fn captured_evidence_provenance(
    engine: &Engine,
    snapshot: &SemanticSnapshot,
) -> Result<BTreeMap<EvidenceId, BTreeMap<CommitId, serde_json::Value>>> {
    let mut result = BTreeMap::<EvidenceId, BTreeMap<CommitId, serde_json::Value>>::new();
    let mut pending = vec![snapshot.commit_id];
    let mut visited = BTreeSet::new();
    while let Some(id) = pending.pop() {
        if !visited.insert(id) {
            continue;
        }
        let commit = engine.commit(id)?;
        if commit.workspace_id != snapshot.workspace_id {
            return Err(WorkVcsError::IntegrityInvalid(
                "Evidence ancestry crosses Workspace ownership".to_owned(),
            ));
        }
        pending.extend(commit.parents.iter().map(|parent| parent.parent_commit_id));
        if !matches!(
            commit.operation_type.as_str(),
            "cognition.capture" | "plan.admit" | "plan.evolve"
        ) {
            continue;
        }
        let changeset = engine.changeset(commit.changeset_id)?;
        let payload: CapturedEvidenceEnvelope =
            serde_json::from_str(&changeset.operation_payload_json).map_err(|error| {
                WorkVcsError::IntegrityInvalid(format!(
                    "invalid immutable cognition Evidence provenance at {id}: {error}"
                ))
            })?;
        if payload.workspace_id != snapshot.workspace_id || payload.commit_id != id {
            return Err(WorkVcsError::IntegrityInvalid(
                "cognition Evidence provenance contradicts its commit".to_owned(),
            ));
        }
        for evidence in payload.evidence {
            result.entry(evidence.evidence_id).or_default().insert(
                id,
                serde_json::json!({
                    "commit_id": id,
                    "changeset_id": commit.changeset_id,
                    "payload_digest": changeset.operation_payload_digest,
                    "workspace_id": payload.workspace_id,
                    "origin_branch_id": payload.branch_id,
                }),
            );
        }
    }
    Ok(result)
}

/// Recover immutable evidence only through declared scope or current typed citations.
pub(crate) fn project_evidence(
    engine: &Engine,
    snapshot: &SemanticSnapshot,
    selected: &ResolvedWorkSelection,
    profile: super::RecallProfileArg,
) -> Result<EvidenceProjection> {
    if profile != super::RecallProfileArg::Retrospective {
        return Ok(EvidenceProjection::default());
    }
    let scoped = selected.applicability.selector.is_some();
    let capture_provenance = captured_evidence_provenance(engine, snapshot)?;
    // Evidence identities are immutable and native capture publishes their introducing
    // commit atomically. Read the identity list first, then introduction identities:
    // a concurrent capture can never masquerade as standalone evidence at an older head.
    let evidence_items = engine.evidences(EvidenceListOptions::all())?.evidences;
    let introduced = engine
        .evidence_introductions()?
        .into_iter()
        .map(|(id, _)| id)
        .collect::<BTreeSet<_>>();
    let mut citations = BTreeMap::<EvidenceId, Vec<serde_json::Value>>::new();
    let current = engine.current_scope_at(snapshot.commit_id)?;
    let containment = engine
        .primary_containment_relations_at(snapshot.commit_id)?
        .into_iter()
        .filter(|relation| {
            current.has_relation_carrier(
                relation.child_entity_id,
                CurrentnessCarrierKind::PrimaryContainment,
                relation.parent_entity_id,
                relation.relation_id,
                relation.relation_version_id,
            )
        })
        .collect::<Vec<_>>();
    let references = engine
        .structural_references_at(snapshot.commit_id)?
        .into_iter()
        .filter(|reference| {
            current.has_relation_carrier(
                reference.target_entity_id,
                CurrentnessCarrierKind::StructuralReference,
                reference.referrer_entity_id,
                reference.relation_id,
                reference.relation_version_id,
            )
        })
        .collect::<Vec<_>>();
    let selected_entities = selected
        .applicability
        .selected_entities(&containment, &references);
    {
        let mut targets = BTreeMap::new();
        for task in &snapshot.tasks {
            if scoped
                && !selected.applicability.matches_scope(
                    &workvcs_core::task_work_scope(task.task_entity_id),
                    selected_entities.as_ref(),
                )
            {
                continue;
            }
            for criterion_ref in &task.state.acceptance_criteria {
                let criterion = engine.acceptance_criterion_at(
                    snapshot.commit_id,
                    criterion_ref.acceptance_criterion_entity_id,
                )?;
                if criterion.task_entity_id != task.task_entity_id
                    || criterion.local_key != criterion_ref.local_key
                {
                    return Err(WorkVcsError::IntegrityInvalid(
                        "selected Task acceptance reference does not match its current criterion"
                            .to_owned(),
                    ));
                }
                let qualification = serde_json::json!({
                    "task_id": task.task_entity_id,
                    "task_version_id": task.task_entity_version_id,
                    "criterion_id": criterion.acceptance_criterion_entity_id,
                    "criterion_version_id": criterion.acceptance_criterion_entity_version_id,
                });
                targets.insert(
                    criterion.acceptance_criterion_entity_id,
                    qualification.clone(),
                );
                for requirement_ref in &criterion.state.verification_requirements {
                    let requirement = engine.verification_requirement_at(
                        snapshot.commit_id,
                        requirement_ref.verification_requirement_entity_id,
                    )?;
                    if requirement.acceptance_criterion_entity_id
                        != criterion.acceptance_criterion_entity_id
                        || requirement.local_key != requirement_ref.local_key
                    {
                        return Err(WorkVcsError::IntegrityInvalid("selected acceptance reference does not match its current verification requirement".to_owned()));
                    }
                    targets.insert(requirement.verification_requirement_entity_id, serde_json::json!({
                        "criterion": qualification,
                        "requirement_id": requirement.verification_requirement_entity_id,
                        "requirement_version_id": requirement.verification_requirement_entity_version_id,
                    }));
                }
            }
        }
        let state = engine.state_at(snapshot.commit_id)?;
        let versions = state
            .state
            .entities()
            .iter()
            .copied()
            .collect::<BTreeMap<_, _>>();
        let mut verifications = engine.verifications_at(snapshot.commit_id)?;
        verifications.sort_by_key(|verification| verification.verification_entity_id);
        for verification in verifications {
            if !targets.contains_key(&verification.target.entity_id())
                || !verification
                    .state
                    .semantic_dependencies
                    .iter()
                    .all(|dependency| {
                        versions.get(&dependency.entity_id) == Some(&dependency.entity_version_id)
                    })
            {
                continue;
            }
            for relation in &verification.evidenced_by_relations {
                citations
                    .entry(relation.evidence_id)
                    .or_default()
                    .push(serde_json::json!({
                        "verification_id": verification.verification_entity_id,
                        "verification_version_id": verification.verification_entity_version_id,
                        "verification_digest": verification.state_digest,
                        "verifies_relation_id": verification.verifies_relation_id,
                        "verifies_relation_version_id": verification.verifies_relation_version_id,
                        "evidence_relation_id": relation.relation_id,
                        "evidence_relation_version_id": relation.relation_version_id,
                            "target_qualification": targets.get(&verification.target.entity_id()),
                    }));
            }
        }
    }
    let mut projection = EvidenceProjection::default();
    for evidence in evidence_items {
        if introduced.contains(&evidence.evidence_id)
            && !capture_provenance.contains_key(&evidence.evidence_id)
        {
            continue;
        }
        let declared_scope = match &evidence.metadata {
            CanonicalValue::Object(fields) => fields
                .iter()
                .find(|(key, _)| key == "work_scope")
                .map(|(_, scope)| scope),
            _ => None,
        };
        let scope_matches = if let Some(scope) = declared_scope {
            if !matches!(scope, CanonicalValue::Object(fields) if !fields.is_empty()) {
                return Err(WorkVcsError::QueryInvalid(format!(
                    "Evidence {} metadata.work_scope must be a nonempty scope object",
                    evidence.evidence_id
                )));
            }
            workvcs_core::validate_work_scope(scope)?;
            let CanonicalValue::Object(fields) = scope else {
                unreachable!()
            };
            let owner = fields
                .iter()
                .find(|(key, _)| key == "workspace_id")
                .map(|(_, value)| value);
            let branch = fields
                .iter()
                .find(|(key, _)| key == "branch_id")
                .map(|(_, value)| value);
            let owner_matches = match (owner, branch) {
                (Some(CanonicalValue::String(owner)), Some(CanonicalValue::String(branch))) => {
                    WorkspaceId::parse_canonical(owner)? == selected.context.workspace_id
                        && BranchId::parse_canonical(branch)? == selected.context.branch_id
                }
                (None, None) => false,
                _ => {
                    return Err(WorkVcsError::QueryInvalid(format!(
                        "Evidence {} metadata.work_scope requires canonical workspace_id and branch_id together",
                        evidence.evidence_id
                    )));
                }
            };
            let inherited_owner_matches = capture_provenance
                .get(&evidence.evidence_id)
                .is_some_and(|captures| match (owner, branch) {
                    (None, None) => true,
                    (Some(CanonicalValue::String(owner)), Some(CanonicalValue::String(branch))) => {
                        captures.values().any(|capture| {
                            capture["workspace_id"].as_str() == Some(owner.as_str())
                                && capture["origin_branch_id"].as_str() == Some(branch.as_str())
                        })
                    }
                    _ => false,
                });
            if capture_provenance.contains_key(&evidence.evidence_id)
                && owner.is_some()
                && !inherited_owner_matches
            {
                return Err(WorkVcsError::IntegrityInvalid(format!(
                    "Evidence {} work scope contradicts its immutable introduction provenance",
                    evidence.evidence_id
                )));
            }
            (owner_matches || inherited_owner_matches)
                && (is_project_fact_scope(scope)
                    || selected
                        .applicability
                        .matches_scope(scope, selected_entities.as_ref()))
        } else {
            false
        };
        let evidence_citations = citations.remove(&evidence.evidence_id).unwrap_or_default();
        let captures = capture_provenance.get(&evidence.evidence_id);
        if !scope_matches && evidence_citations.is_empty() && (scoped || captures.is_none()) {
            continue;
        }
        projection.basis.insert(
            evidence.evidence_id,
            serde_json::json!({
                "captured_at_us": evidence.captured_at_us,
                "declared_scope": declared_scope.map(canonical_json_value).transpose()?,
                "citations": evidence_citations,
                "captures": captures,
            }),
        );
        projection.items.push(evidence);
    }
    Ok(projection)
}

/// Apply task ownership before lifecycle ordering or output-budget decisions.
pub(crate) fn project_snapshot(
    engine: &Engine,
    snapshot: &mut SemanticSnapshot,
    selected: &ResolvedWorkSelection,
    profile: super::RecallProfileArg,
) -> Result<()> {
    let current = engine.current_scope_at(snapshot.commit_id)?;
    if selected.applicability.selector.is_some() {
        let containment = engine
            .primary_containment_relations_at(snapshot.commit_id)?
            .into_iter()
            .filter(|relation| {
                current.has_relation_carrier(
                    relation.child_entity_id,
                    CurrentnessCarrierKind::PrimaryContainment,
                    relation.parent_entity_id,
                    relation.relation_id,
                    relation.relation_version_id,
                )
            })
            .collect::<Vec<_>>();
        let references = engine
            .structural_references_at(snapshot.commit_id)?
            .into_iter()
            .filter(|reference| {
                current.has_relation_carrier(
                    reference.target_entity_id,
                    CurrentnessCarrierKind::StructuralReference,
                    reference.referrer_entity_id,
                    reference.relation_id,
                    reference.relation_version_id,
                )
            })
            .collect::<Vec<_>>();
        let selected_entities = selected
            .applicability
            .selected_entities(&containment, &references);
        snapshot.records.retain(|record| {
            selected
                .applicability
                .matches_scope(&record.state.scope, selected_entities.as_ref())
                || (record.state.kind == RecordKind::Finding
                    && is_project_fact_scope(&record.state.scope))
        });
        snapshot.knowledge.retain(|knowledge| {
            selected
                .applicability
                .matches_knowledge_scope(&knowledge.state.scope, selected_entities.as_ref())
        });
        if let Some(selected_entities) = selected_entities {
            let mut relevant = selected_entities.clone();
            loop {
                let before = relevant.len();
                for relation in &containment {
                    if relevant.contains(&relation.child_entity_id) {
                        relevant.insert(relation.parent_entity_id);
                    }
                }
                for reference in &references {
                    if relevant.contains(&reference.target_entity_id) {
                        relevant.insert(reference.referrer_entity_id);
                    }
                }
                if relevant.len() == before {
                    break;
                }
            }
            snapshot
                .tasks
                .retain(|value| selected_entities.contains(&value.task_entity_id));
            snapshot
                .plans
                .retain(|value| relevant.contains(&value.plan_entity_id));
            snapshot
                .goals
                .retain(|value| relevant.contains(&value.goal_entity_id));
        }
        let records = snapshot
            .records
            .iter()
            .map(|value| value.record_entity_id)
            .collect::<BTreeSet<_>>();
        let knowledge = snapshot
            .knowledge
            .iter()
            .map(|value| value.knowledge_entity_id)
            .collect::<BTreeSet<_>>();
        snapshot.record_relations.retain(|value| {
            records.contains(&value.source_record_entity_id)
                && records.contains(&value.target_record_entity_id)
        });
        snapshot.record_knowledge_relations.retain(|value| {
            records.contains(&value.source_record_entity_id)
                && knowledge.contains(&value.target_knowledge_entity_id)
        });
        snapshot.knowledge_relations.retain(|value| {
            knowledge.contains(&value.prior_knowledge_entity_id)
                && knowledge.contains(&value.replacement_knowledge_entity_id)
        });
    }
    if profile != super::RecallProfileArg::Retrospective {
        snapshot
            .goals
            .retain(|value| current.current_goals.contains(&value.goal_entity_id));
        snapshot
            .plans
            .retain(|value| current.current_plans.contains(&value.plan_entity_id));
        snapshot
            .tasks
            .retain(|value| current.current_tasks.contains(&value.task_entity_id));
        snapshot
            .knowledge
            .retain(|value| value.state.status == KnowledgeStatus::Active);
    }
    snapshot.records.retain(|value| match profile {
        super::RecallProfileArg::Brief => super::record_status_is_current(value.state.status),
        super::RecallProfileArg::Handoff => {
            super::record_status_is_current(value.state.status)
                || value.state.kind == RecordKind::Attempt
        }
        super::RecallProfileArg::Retrospective => true,
    });
    Ok(())
}

fn versions(snapshot: &SemanticSnapshot) -> BTreeMap<(String, String), String> {
    let mut result = BTreeMap::new();
    macro_rules! collect {
        ($field:ident, $category:literal, $id:ident, $version:ident) => {
            for value in &snapshot.$field {
                result.insert(
                    ($category.to_owned(), value.$id.to_string()),
                    value.$version.to_string(),
                );
            }
        };
    }
    collect!(goals, "goal", goal_entity_id, goal_entity_version_id);
    collect!(plans, "plan", plan_entity_id, plan_entity_version_id);
    collect!(tasks, "task", task_entity_id, task_entity_version_id);
    collect!(
        records,
        "record",
        record_entity_id,
        record_entity_version_id
    );
    collect!(
        knowledge,
        "knowledge",
        knowledge_entity_id,
        knowledge_entity_version_id
    );
    collect!(
        record_relations,
        "record_relation",
        relation_id,
        relation_version_id
    );
    collect!(
        record_knowledge_relations,
        "record_knowledge_relation",
        relation_id,
        relation_version_id
    );
    collect!(
        knowledge_relations,
        "knowledge_relation",
        relation_id,
        relation_version_id
    );
    result
}

/// Produce semantic changes against an exact ancestor; runtime remains a live read.
pub(crate) struct RecoveryBaseline {
    pub commit_id: CommitId,
    pub context_digest: Digest,
}

pub(crate) fn apply_delta(
    engine: &Engine,
    snapshot: &mut SemanticSnapshot,
    selected: &ResolvedWorkSelection,
    profile: super::RecallProfileArg,
    baseline: RecoveryBaseline,
    discovery: &super::ProjectDiscovery,
    evidence: &mut EvidenceProjection,
) -> Result<Vec<(String, String)>> {
    let since = baseline.commit_id;
    let mut pending = vec![snapshot.commit_id];
    let mut visited = BTreeSet::new();
    let mut ancestor = false;
    while let Some(commit_id) = pending.pop() {
        if !visited.insert(commit_id) {
            continue;
        }
        if commit_id == since {
            ancestor = true;
            break;
        }
        let commit = engine.commit(commit_id)?;
        pending.extend(commit.parents.iter().map(|parent| parent.parent_commit_id));
    }
    if !ancestor {
        return Err(WorkVcsError::QueryInvalid(
            "delta basis must be an ancestor of the selected Branch head; recover a fresh context after changing work".to_owned(),
        ));
    }
    let mut options = SemanticSnapshotOptions::new(since);
    if profile != super::RecallProfileArg::Brief {
        options = options
            .with_record_relations()
            .with_record_knowledge_relations();
    }
    if profile == super::RecallProfileArg::Retrospective {
        options = options.with_knowledge_relations();
    }
    let mut previous = engine.semantic_snapshot(options)?;
    project_snapshot(engine, &mut previous, selected, profile)?;
    let previous_evidence = project_evidence(engine, &previous, selected, profile)?;
    if context_basis_digest(discovery, selected, profile, &previous, &previous_evidence)?
        != baseline.context_digest
    {
        return Err(WorkVcsError::QueryInvalid(
            "recovery selection changed or the exact cached commit and projected versions do not match; recover a fresh context before applying a delta".to_owned(),
        ));
    }
    let before = versions(&previous);
    let current = versions(snapshot);
    let mut removed = before
        .keys()
        .filter(|key| !current.contains_key(*key))
        .cloned()
        .collect::<Vec<_>>();
    removed.extend(
        previous_evidence
            .basis
            .keys()
            .filter(|id| !evidence.basis.contains_key(id))
            .map(|id| ("evidence".to_owned(), id.to_string())),
    );
    evidence.items.retain(|item| {
        previous_evidence.basis.get(&item.evidence_id) != evidence.basis.get(&item.evidence_id)
    });
    macro_rules! changed {
        ($field:ident, $category:literal, $id:ident, $version:ident) => {
            snapshot.$field.retain(|value| {
                before.get(&($category.to_owned(), value.$id.to_string()))
                    != Some(&value.$version.to_string())
            });
        };
    }
    changed!(goals, "goal", goal_entity_id, goal_entity_version_id);
    changed!(plans, "plan", plan_entity_id, plan_entity_version_id);
    changed!(tasks, "task", task_entity_id, task_entity_version_id);
    changed!(
        records,
        "record",
        record_entity_id,
        record_entity_version_id
    );
    changed!(
        knowledge,
        "knowledge",
        knowledge_entity_id,
        knowledge_entity_version_id
    );
    changed!(
        record_relations,
        "record_relation",
        relation_id,
        relation_version_id
    );
    changed!(
        record_knowledge_relations,
        "record_knowledge_relation",
        relation_id,
        relation_version_id
    );
    changed!(
        knowledge_relations,
        "knowledge_relation",
        relation_id,
        relation_version_id
    );
    Ok(removed)
}

pub(crate) fn tag_cognition_payload(
    payload: &mut serde_json::Value,
    selector: &CanonicalValue,
    context: &CaptureWorkContext,
) -> Result<()> {
    for field in ["records", "knowledge"] {
        if let Some(items) = payload
            .get_mut(field)
            .and_then(serde_json::Value::as_array_mut)
        {
            for item in items {
                let object = item.as_object_mut().ok_or_else(|| {
                    WorkVcsError::QueryInvalid("cognition item must be an object".to_owned())
                })?;
                let original = object
                    .get("scope")
                    .cloned()
                    .unwrap_or_else(|| serde_json::json!({}));
                let mut current =
                    parse_canonical_json(&serde_json::to_vec(&original).map_err(|error| {
                        WorkVcsError::CanonicalEncodingInvalid(error.to_string())
                    })?)?;
                if is_project_fact_scope(&current) {
                    continue;
                }
                merge_scope_value(&mut current, selector)?;
                object.insert("scope".to_owned(), canonical_json_value(&current)?);
            }
        }
    }
    if let Some(items) = payload
        .get_mut("evidence")
        .and_then(serde_json::Value::as_array_mut)
    {
        for item in items {
            let object = item.as_object_mut().ok_or_else(|| {
                WorkVcsError::QueryInvalid("cognition Evidence must be an object".to_owned())
            })?;
            let metadata = object
                .entry("metadata")
                .or_insert_with(|| serde_json::json!({}));
            let metadata = metadata.as_object_mut().ok_or_else(|| {
                WorkVcsError::QueryInvalid("Evidence metadata must be an object".to_owned())
            })?;
            let original = metadata
                .get("work_scope")
                .cloned()
                .unwrap_or_else(|| serde_json::json!({}));
            let mut current = parse_canonical_json(
                &serde_json::to_vec(&original)
                    .map_err(|error| WorkVcsError::CanonicalEncodingInvalid(error.to_string()))?,
            )?;
            if !is_project_fact_scope(&current) {
                merge_scope_value(&mut current, selector)?;
            }
            let owner = parse_canonical_json(
                &serde_json::to_vec(&serde_json::json!({
                    "workspace_id": context.workspace_id,
                    "branch_id": context.branch_id,
                }))
                .map_err(|error| WorkVcsError::CanonicalEncodingInvalid(error.to_string()))?,
            )?;
            merge_scope_value(&mut current, &owner)?;
            metadata.insert("work_scope".to_owned(), canonical_json_value(&current)?);
        }
    }
    Ok(())
}

pub(crate) fn canonical_json_value(value: &CanonicalValue) -> Result<serde_json::Value> {
    serde_json::from_slice(&workvcs_core::canonical_bytes(value)?).map_err(|error| {
        WorkVcsError::CanonicalEncodingInvalid(format!(
            "cannot decode canonical JSON value: {error}"
        ))
    })
}
