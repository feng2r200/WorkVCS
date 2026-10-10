use super::{
    CurrentScopeProjection, PlanStatus, PrimaryContainmentEndpointKind,
    StructuralReferenceEndpointKind, TaskSchedulingRelationType, TaskStatus, branch_head,
    current_scope_at, plan_at, primary_containment_relations_at, structural_references_at, task_at,
    task_scheduling_relations_at,
};
use crate::error::{Result, WorkVcsError, storage_error};
use crate::identity::{
    BranchId, CommitId, Digest, EntityId, EntityVersionId, RelationId, RelationVersionId,
    WorkspaceId,
};
use crate::store::StoreConnection;
use rusqlite::params;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlanTaskDispositionAuditOptions {
    branch_id: BranchId,
    plan_entity_id: EntityId,
}

impl PlanTaskDispositionAuditOptions {
    pub fn new(branch_id: BranchId, plan_entity_id: EntityId) -> Self {
        Self {
            branch_id,
            plan_entity_id,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlanTaskDispositionAuditResult {
    pub workspace_id: WorkspaceId,
    pub branch_id: BranchId,
    pub commit_id: CommitId,
    pub plan_entity_id: EntityId,
    pub plan_entity_version_id: EntityVersionId,
    pub plan_state_digest: Digest,
    pub plan_status: PlanStatus,
    pub plan_current: bool,
    pub retired_plan_orphan: bool,
    pub tasks: Vec<PlanTaskDispositionAuditTask>,
    pub nested_plans: Vec<PlanTaskDispositionAuditNestedPlan>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlanTaskDispositionAuditTask {
    pub task_entity_id: EntityId,
    pub task_entity_version_id: EntityVersionId,
    pub state_digest: Digest,
    pub status: TaskStatus,
    pub current: bool,
    pub terminal_history: bool,
    pub currentness_carriers: Vec<String>,
    pub reverse_references: Vec<PlanTaskDispositionAuditReference>,
    pub external_dependents: Vec<PlanTaskDispositionAuditDependent>,
    pub ordered_before_hints: Vec<PlanTaskDispositionAuditDependent>,
    pub active_claims: i64,
    pub stale_scope_claim: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlanTaskDispositionAuditNestedPlan {
    pub plan_entity_id: EntityId,
    pub plan_entity_version_id: EntityVersionId,
    pub state_digest: Digest,
    pub status: PlanStatus,
    pub current: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlanTaskDispositionAuditReference {
    pub relation_id: RelationId,
    pub relation_version_id: RelationVersionId,
    pub source_entity_id: EntityId,
    pub source_kind: StructuralReferenceEndpointKind,
    pub source_current: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlanTaskDispositionAuditDependent {
    pub relation_id: RelationId,
    pub relation_version_id: RelationVersionId,
    pub task_entity_id: EntityId,
    pub current: bool,
    pub external_to_target_scope: bool,
}

pub(crate) fn plan_task_disposition_audit(
    connection: &StoreConnection,
    options: &PlanTaskDispositionAuditOptions,
) -> Result<PlanTaskDispositionAuditResult> {
    connection.verify_foreign_keys()?;
    let head = branch_head(connection, options.branch_id)?;
    let plan = plan_at(connection, head.head_commit_id, options.plan_entity_id)?;
    let current = current_scope_at(connection, head.head_commit_id)?;
    let containment = primary_containment_relations_at(connection, head.head_commit_id)?;
    let (task_ids, nested_plan_ids) =
        bounded_plan_descendants(options.plan_entity_id, &containment)?;
    let references = structural_references_at(connection, head.head_commit_id)?;
    let scheduling = task_scheduling_relations_at(connection, head.head_commit_id)?;

    let mut tasks = Vec::new();
    for task_entity_id in &task_ids {
        let task = task_at(connection, head.head_commit_id, *task_entity_id)?;
        let current_task = current.current_tasks.contains(task_entity_id);
        let active_claims = connection
            .inner()
            .query_row(
                "SELECT count(*) FROM claim_runtime INNER JOIN claim ON claim.claim_id = claim_runtime.claim_id WHERE claim.branch_id = ?1 AND claim.task_entity_id = ?2",
                params![
                    &options.branch_id.raw_bytes()[..],
                    &task_entity_id.raw_bytes()[..]
                ],
                |row| row.get::<_, i64>(0),
            )
            .map_err(storage_error)?;
        let reverse_references = references
            .iter()
            .filter(|reference| reference.target_entity_id == *task_entity_id)
            .map(|reference| PlanTaskDispositionAuditReference {
                relation_id: reference.relation_id,
                relation_version_id: reference.relation_version_id,
                source_entity_id: reference.referrer_entity_id,
                source_kind: reference.referrer_kind,
                source_current: reference_source_current(&current, reference),
            })
            .collect::<Vec<_>>();
        let external_dependents = scheduling
            .iter()
            .filter(|relation| {
                relation.relation_type == TaskSchedulingRelationType::DependsOn
                    && relation.target_task_entity_id == *task_entity_id
            })
            .map(|relation| PlanTaskDispositionAuditDependent {
                relation_id: relation.relation_id,
                relation_version_id: relation.relation_version_id,
                task_entity_id: relation.source_task_entity_id,
                current: current
                    .current_tasks
                    .contains(&relation.source_task_entity_id),
                external_to_target_scope: !task_ids.contains(&relation.source_task_entity_id),
            })
            .collect::<Vec<_>>();
        let ordered_before_hints = scheduling
            .iter()
            .filter(|relation| {
                relation.relation_type == TaskSchedulingRelationType::OrderedBefore
                    && (relation.source_task_entity_id == *task_entity_id
                        || relation.target_task_entity_id == *task_entity_id)
            })
            .map(|relation| {
                let other = if relation.source_task_entity_id == *task_entity_id {
                    relation.target_task_entity_id
                } else {
                    relation.source_task_entity_id
                };
                PlanTaskDispositionAuditDependent {
                    relation_id: relation.relation_id,
                    relation_version_id: relation.relation_version_id,
                    task_entity_id: other,
                    current: current.current_tasks.contains(&other),
                    external_to_target_scope: !task_ids.contains(&other),
                }
            })
            .collect::<Vec<_>>();
        tasks.push(PlanTaskDispositionAuditTask {
            task_entity_id: *task_entity_id,
            task_entity_version_id: task.task_entity_version_id,
            state_digest: task.state_digest,
            status: task.state.status,
            current: current_task,
            terminal_history: task.state.status.is_terminal(),
            currentness_carriers: carrier_strings(&current, *task_entity_id),
            reverse_references,
            external_dependents,
            ordered_before_hints,
            active_claims,
            stale_scope_claim: active_claims > 0 && !current_task,
        });
    }
    tasks.sort_by_key(|task| task.task_entity_id);

    let mut nested_plans = Vec::new();
    for plan_entity_id in nested_plan_ids {
        let nested = plan_at(connection, head.head_commit_id, plan_entity_id)?;
        nested_plans.push(PlanTaskDispositionAuditNestedPlan {
            plan_entity_id,
            plan_entity_version_id: nested.plan_entity_version_id,
            state_digest: nested.state_digest,
            status: nested.state.status,
            current: current.current_plans.contains(&plan_entity_id),
        });
    }
    nested_plans.sort_by_key(|nested| nested.plan_entity_id);
    let plan_current = current.current_plans.contains(&options.plan_entity_id);
    let retired_plan_orphan = matches!(
        plan.state.status,
        PlanStatus::Superseded | PlanStatus::Abandoned
    ) && tasks
        .iter()
        .any(|task| !task.terminal_history && !task.current);
    Ok(PlanTaskDispositionAuditResult {
        workspace_id: head.workspace_id,
        branch_id: options.branch_id,
        commit_id: head.head_commit_id,
        plan_entity_id: options.plan_entity_id,
        plan_entity_version_id: plan.plan_entity_version_id,
        plan_state_digest: plan.state_digest,
        plan_status: plan.state.status,
        plan_current,
        retired_plan_orphan,
        tasks,
        nested_plans,
    })
}

fn bounded_plan_descendants(
    root: EntityId,
    containment: &[super::PrimaryContainmentSnapshot],
) -> Result<(BTreeSet<EntityId>, BTreeSet<EntityId>)> {
    let mut children = BTreeMap::<EntityId, Vec<(EntityId, PrimaryContainmentEndpointKind)>>::new();
    for relation in containment {
        children
            .entry(relation.parent_entity_id)
            .or_default()
            .push((relation.child_entity_id, relation.child_kind));
    }
    let mut visited = BTreeSet::new();
    let mut tasks = BTreeSet::new();
    let mut plans = BTreeSet::new();
    let mut stack = children.get(&root).cloned().unwrap_or_default();
    while let Some((entity_id, kind)) = stack.pop() {
        if !visited.insert(entity_id) {
            continue;
        }
        match kind {
            PrimaryContainmentEndpointKind::Goal => {
                return Err(WorkVcsError::IntegrityInvalid(format!(
                    "Plan {root} scope contains Goal child {entity_id}"
                )));
            }
            PrimaryContainmentEndpointKind::Plan => {
                plans.insert(entity_id);
            }
            PrimaryContainmentEndpointKind::Task => {
                tasks.insert(entity_id);
            }
        }
        if let Some(descendants) = children.get(&entity_id) {
            stack.extend(descendants.iter().copied());
        }
    }
    Ok((tasks, plans))
}

fn reference_source_current(
    current: &CurrentScopeProjection,
    reference: &super::StructuralReferenceSnapshot,
) -> bool {
    match reference.referrer_kind {
        StructuralReferenceEndpointKind::Goal => current
            .current_goals
            .contains(&reference.referrer_entity_id),
        StructuralReferenceEndpointKind::Plan => current
            .current_plans
            .contains(&reference.referrer_entity_id),
        StructuralReferenceEndpointKind::Task => false,
    }
}

fn carrier_strings(current: &CurrentScopeProjection, entity_id: EntityId) -> Vec<String> {
    current
        .carriers_for(entity_id)
        .iter()
        .map(|carrier| {
            format!(
                "{}:{}:{}",
                carrier.kind.as_str(),
                carrier
                    .source_entity_id
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| "none".to_owned()),
                carrier
                    .relation_id
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| "none".to_owned())
            )
        })
        .collect()
}
