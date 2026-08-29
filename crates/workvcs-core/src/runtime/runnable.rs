use super::session;
use crate::error::{Result, WorkVcsError, storage_error};
use crate::history::{
    self, TaskSchedulingRelationSnapshot, TaskSchedulingRelationType, TaskSnapshot, TaskStatus,
};
use crate::identity::{BranchId, ClaimId, CommitId, EntityId, SessionId, WorkspaceId};
use crate::store::StoreConnection;
use rusqlite::params;
use std::collections::{BTreeMap, BTreeSet, HashSet};

const ACTIVE_BRANCH_LIFECYCLE_STATE: &str = "active";
const EXCLUSIVE_CLAIM_MODE: &str = "exclusive";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RunnableTasksOptions {
    session_id: SessionId,
}

impl RunnableTasksOptions {
    pub fn new(session_id: SessionId) -> Self {
        Self { session_id }
    }

    pub fn session_id(&self) -> SessionId {
        self.session_id
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RunnableTasksProjection {
    pub session_id: SessionId,
    pub workspace_id: WorkspaceId,
    pub branch_id: BranchId,
    pub head_commit_id: CommitId,
    pub deferred_dimensions: Vec<RunnableTaskProjectionDimension>,
    pub candidates: Vec<RunnableTaskCandidate>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RunnableTaskProjectionDimension {
    ActiveScopePlanPath,
    ExecutableTaskDescendants,
    DependencyReadiness,
    ExplicitManualOrder,
    FinalEqualCandidateTieBreaker,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RunnableTaskCandidate {
    pub task: TaskSnapshot,
    pub lifecycle_eligible: bool,
    pub dependency_ready: bool,
    pub unsatisfied_dependency_entity_ids: Vec<EntityId>,
    pub claim_coordination: RunnableTaskClaimCoordination,
    pub runnable: bool,
    pub blocked_reasons: Vec<RunnableTaskBlockedReason>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RunnableTaskClaimCoordination {
    Unclaimed,
    ClaimedBySession {
        claim_id: ClaimId,
    },
    ClaimedByOtherSession {
        claim_id: ClaimId,
        session_id: SessionId,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RunnableTaskBlockedReason {
    LifecycleIneligible,
    DependencyBlocked,
    ClaimBlocked,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ProjectionAnchor {
    workspace_id: WorkspaceId,
    branch_id: BranchId,
    head_commit_id: CommitId,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct DependencyReadiness {
    unsatisfied_dependency_entity_ids: Vec<EntityId>,
}

impl DependencyReadiness {
    fn satisfied() -> Self {
        Self {
            unsatisfied_dependency_entity_ids: Vec::new(),
        }
    }

    fn is_ready(&self) -> bool {
        self.unsatisfied_dependency_entity_ids.is_empty()
    }
}

pub(crate) fn runnable_tasks(
    connection: &StoreConnection,
    options: &RunnableTasksOptions,
) -> Result<RunnableTasksProjection> {
    connection.verify_foreign_keys()?;
    let anchor = projection_anchor(connection, options.session_id())?;
    let tasks = history::tasks_at(connection, anchor.head_commit_id)?;
    let scheduling_relations =
        history::task_scheduling_relations_at(connection, anchor.head_commit_id)?;
    let dependency_readiness = dependency_readiness_by_task(&tasks, &scheduling_relations)?;
    let claim_coordination = load_active_claim_coordination(
        connection,
        anchor.workspace_id,
        anchor.branch_id,
        options.session_id(),
    )?;
    let mut candidates = tasks
        .into_iter()
        .map(|task| {
            let dependency_readiness = dependency_readiness
                .get(&task.task_entity_id)
                .cloned()
                .unwrap_or_else(DependencyReadiness::satisfied);
            let coordination = claim_coordination
                .get(&task.task_entity_id)
                .cloned()
                .unwrap_or(RunnableTaskClaimCoordination::Unclaimed);
            candidate_with_claims(task, dependency_readiness, coordination)
        })
        .collect::<Vec<_>>();
    sort_candidates(&mut candidates);
    ensure_projection_anchor_unchanged(connection, options.session_id(), anchor)?;

    Ok(RunnableTasksProjection {
        session_id: options.session_id(),
        workspace_id: anchor.workspace_id,
        branch_id: anchor.branch_id,
        head_commit_id: anchor.head_commit_id,
        deferred_dimensions: deferred_dimensions(),
        candidates,
    })
}

fn candidate_with_claims(
    task: TaskSnapshot,
    dependency_readiness: DependencyReadiness,
    claim_coordination: RunnableTaskClaimCoordination,
) -> RunnableTaskCandidate {
    let lifecycle_eligible = lifecycle_eligible(task.state.status);
    let dependency_ready = dependency_readiness.is_ready();
    let claim_blocked = matches!(
        claim_coordination,
        RunnableTaskClaimCoordination::ClaimedByOtherSession { .. }
    );
    let mut blocked_reasons = Vec::new();
    if !lifecycle_eligible {
        blocked_reasons.push(RunnableTaskBlockedReason::LifecycleIneligible);
    }
    if !dependency_ready {
        blocked_reasons.push(RunnableTaskBlockedReason::DependencyBlocked);
    }
    if claim_blocked {
        blocked_reasons.push(RunnableTaskBlockedReason::ClaimBlocked);
    }

    RunnableTaskCandidate {
        task,
        lifecycle_eligible,
        dependency_ready,
        unsatisfied_dependency_entity_ids: dependency_readiness.unsatisfied_dependency_entity_ids,
        claim_coordination,
        runnable: lifecycle_eligible && dependency_ready && !claim_blocked,
        blocked_reasons,
    }
}

fn lifecycle_eligible(status: TaskStatus) -> bool {
    matches!(status, TaskStatus::Pending | TaskStatus::InProgress)
}

fn dependency_readiness_by_task(
    tasks: &[TaskSnapshot],
    relations: &[TaskSchedulingRelationSnapshot],
) -> Result<BTreeMap<EntityId, DependencyReadiness>> {
    let task_statuses = tasks
        .iter()
        .map(|task| (task.task_entity_id, task.state.status))
        .collect::<BTreeMap<_, _>>();
    let mut graph = BTreeMap::<EntityId, Vec<EntityId>>::new();

    for relation in relations {
        if relation.relation_type != TaskSchedulingRelationType::DependsOn {
            continue;
        }
        if !task_statuses.contains_key(&relation.source_task_entity_id) {
            return Err(WorkVcsError::TaskInvalid(format!(
                "depends_on relation {} source task {} is not present in runnable projection",
                relation.relation_id, relation.source_task_entity_id
            )));
        }
        if !task_statuses.contains_key(&relation.target_task_entity_id) {
            return Err(WorkVcsError::TaskInvalid(format!(
                "depends_on relation {} target task {} is not present in runnable projection",
                relation.relation_id, relation.target_task_entity_id
            )));
        }
        graph
            .entry(relation.source_task_entity_id)
            .or_default()
            .push(relation.target_task_entity_id);
    }
    for prerequisites in graph.values_mut() {
        prerequisites.sort();
        prerequisites.dedup();
    }

    let mut readiness = BTreeMap::new();
    for task in tasks {
        let dependencies = dependency_closure(task.task_entity_id, &graph, &task_statuses)?;
        let unsatisfied_dependency_entity_ids = dependencies
            .into_iter()
            .filter(|dependency_entity_id| {
                task_statuses.get(dependency_entity_id).copied() != Some(TaskStatus::Done)
            })
            .collect();
        readiness.insert(
            task.task_entity_id,
            DependencyReadiness {
                unsatisfied_dependency_entity_ids,
            },
        );
    }
    Ok(readiness)
}

fn dependency_closure(
    task_entity_id: EntityId,
    graph: &BTreeMap<EntityId, Vec<EntityId>>,
    task_statuses: &BTreeMap<EntityId, TaskStatus>,
) -> Result<BTreeSet<EntityId>> {
    let mut collected = BTreeSet::new();
    let mut active_path = HashSet::new();
    let mut complete = HashSet::new();
    collect_dependency_closure(
        task_entity_id,
        graph,
        task_statuses,
        &mut active_path,
        &mut complete,
        &mut collected,
    )?;
    Ok(collected)
}

fn collect_dependency_closure(
    task_entity_id: EntityId,
    graph: &BTreeMap<EntityId, Vec<EntityId>>,
    task_statuses: &BTreeMap<EntityId, TaskStatus>,
    active_path: &mut HashSet<EntityId>,
    complete: &mut HashSet<EntityId>,
    collected: &mut BTreeSet<EntityId>,
) -> Result<()> {
    if complete.contains(&task_entity_id) {
        return Ok(());
    }
    if !active_path.insert(task_entity_id) {
        return Err(WorkVcsError::TaskInvalid(format!(
            "current depends_on graph contains a dependency cycle at task {task_entity_id}"
        )));
    }

    if let Some(prerequisites) = graph.get(&task_entity_id) {
        for prerequisite in prerequisites {
            if !task_statuses.contains_key(prerequisite) {
                return Err(WorkVcsError::TaskInvalid(format!(
                    "depends_on prerequisite task {prerequisite} is not present in runnable projection"
                )));
            }
            collected.insert(*prerequisite);
            collect_dependency_closure(
                *prerequisite,
                graph,
                task_statuses,
                active_path,
                complete,
                collected,
            )?;
        }
    }

    active_path.remove(&task_entity_id);
    complete.insert(task_entity_id);
    Ok(())
}

fn sort_candidates(candidates: &mut [RunnableTaskCandidate]) {
    candidates.sort_by(|left, right| {
        right
            .runnable
            .cmp(&left.runnable)
            .then_with(|| left.task.task_entity_id.cmp(&right.task.task_entity_id))
    });
}

fn projection_anchor(
    connection: &StoreConnection,
    session_id: SessionId,
) -> Result<ProjectionAnchor> {
    let active = session::active_session_projection(connection, session_id)?;
    let branch_head = history::branch_head(connection, active.active_branch_id)?;
    if branch_head.workspace_id != active.active_workspace_id {
        return Err(WorkVcsError::SessionInvalid(format!(
            "session {session_id} active branch {} belongs to workspace {}, not active workspace {}",
            active.active_branch_id, branch_head.workspace_id, active.active_workspace_id
        )));
    }
    if branch_head.lifecycle_state != ACTIVE_BRANCH_LIFECYCLE_STATE {
        return Err(WorkVcsError::SessionInvalid(format!(
            "session {session_id} active branch {} has lifecycle state {:?}",
            active.active_branch_id, branch_head.lifecycle_state
        )));
    }

    Ok(ProjectionAnchor {
        workspace_id: active.active_workspace_id,
        branch_id: active.active_branch_id,
        head_commit_id: branch_head.head_commit_id,
    })
}

fn ensure_projection_anchor_unchanged(
    connection: &StoreConnection,
    session_id: SessionId,
    anchor: ProjectionAnchor,
) -> Result<()> {
    let refreshed = projection_anchor(connection, session_id)?;
    if refreshed.workspace_id != anchor.workspace_id || refreshed.branch_id != anchor.branch_id {
        return Err(WorkVcsError::SessionInvalid(format!(
            "session {session_id} active target changed before runnable projection completed"
        )));
    }
    if refreshed.head_commit_id != anchor.head_commit_id {
        return Err(WorkVcsError::BranchHeadConflict(format!(
            "branch {} head changed before runnable projection completed",
            anchor.branch_id
        )));
    }
    Ok(())
}

fn load_active_claim_coordination(
    connection: &StoreConnection,
    workspace_id: WorkspaceId,
    branch_id: BranchId,
    requesting_session_id: SessionId,
) -> Result<BTreeMap<EntityId, RunnableTaskClaimCoordination>> {
    let mut statement = connection
        .inner()
        .prepare(
            "SELECT claim.claim_id,
                    claim.session_id,
                    claim.task_entity_id,
                    claim.mode
             FROM claim_runtime
             INNER JOIN claim ON claim.claim_id = claim_runtime.claim_id
             WHERE claim.workspace_id = ?1
               AND claim.branch_id = ?2
             ORDER BY claim.task_entity_id, claim.claim_id",
        )
        .map_err(storage_error)?;
    let rows = statement
        .query_map(
            params![&workspace_id.raw_bytes()[..], &branch_id.raw_bytes()[..]],
            |row| {
                Ok((
                    row.get::<_, Vec<u8>>(0)?,
                    row.get::<_, Vec<u8>>(1)?,
                    row.get::<_, Vec<u8>>(2)?,
                    row.get::<_, String>(3)?,
                ))
            },
        )
        .map_err(storage_error)?;

    let mut claims = BTreeMap::new();
    for row in rows {
        let (claim_id, session_id, task_entity_id, mode) = row.map_err(storage_error)?;
        let claim_id = decode_claim_id("claim.claim_id", claim_id)?;
        let session_id = decode_session_id("claim.session_id", session_id)?;
        let task_entity_id = decode_entity_id("claim.task_entity_id", task_entity_id)?;
        validate_claim_mode(&mode)?;
        let coordination = if session_id == requesting_session_id {
            RunnableTaskClaimCoordination::ClaimedBySession { claim_id }
        } else {
            RunnableTaskClaimCoordination::ClaimedByOtherSession {
                claim_id,
                session_id,
            }
        };
        if claims.insert(task_entity_id, coordination).is_some() {
            return Err(WorkVcsError::ClaimInvalid(format!(
                "task {task_entity_id} has more than one active claim on branch {branch_id}"
            )));
        }
    }
    Ok(claims)
}

fn validate_claim_mode(value: &str) -> Result<()> {
    match value {
        EXCLUSIVE_CLAIM_MODE => Ok(()),
        "shared" => Err(WorkVcsError::ClaimInvalid(
            "shared claim mode is not supported by Phase 3G runnable projection".to_owned(),
        )),
        other => Err(WorkVcsError::ClaimInvalid(format!(
            "claim mode {other:?} is not supported"
        ))),
    }
}

fn decode_session_id(column: &str, bytes: Vec<u8>) -> Result<SessionId> {
    let bytes = decode_16(column, bytes)?;
    SessionId::from_bytes(bytes).map_err(|error| {
        WorkVcsError::ClaimInvalid(format!("{column} is not a UUIDv7 value: {error}"))
    })
}

fn decode_claim_id(column: &str, bytes: Vec<u8>) -> Result<ClaimId> {
    let bytes = decode_16(column, bytes)?;
    ClaimId::from_bytes(bytes).map_err(|error| {
        WorkVcsError::ClaimInvalid(format!("{column} is not a UUIDv7 value: {error}"))
    })
}

fn decode_entity_id(column: &str, bytes: Vec<u8>) -> Result<EntityId> {
    let bytes = decode_16(column, bytes)?;
    EntityId::from_bytes(bytes).map_err(|error| {
        WorkVcsError::ClaimInvalid(format!("{column} is not a UUIDv7 value: {error}"))
    })
}

fn decode_16(column: &str, bytes: Vec<u8>) -> Result<[u8; 16]> {
    bytes.try_into().map_err(|bytes: Vec<u8>| {
        WorkVcsError::ClaimInvalid(format!("{column} must be 16 bytes, found {}", bytes.len()))
    })
}

fn deferred_dimensions() -> Vec<RunnableTaskProjectionDimension> {
    vec![
        RunnableTaskProjectionDimension::ActiveScopePlanPath,
        RunnableTaskProjectionDimension::ExecutableTaskDescendants,
        RunnableTaskProjectionDimension::ExplicitManualOrder,
        RunnableTaskProjectionDimension::FinalEqualCandidateTieBreaker,
    ]
}
