use super::session::{self, SessionFocus};
use crate::error::{Result, WorkVcsError, storage_error};
use crate::history::{
    self, PrimaryContainmentEndpointKind, PrimaryContainmentSnapshot,
    TaskSchedulingRelationSnapshot, TaskSchedulingRelationType, TaskSnapshot, TaskStatus,
};
use crate::identity::{BranchId, ClaimId, CommitId, EntityId, SessionId, WorkspaceId};
use crate::store::StoreConnection;
use rusqlite::params;
use std::collections::{BTreeMap, BTreeSet, HashSet};

const ACTIVE_BRANCH_LIFECYCLE_STATE: &str = "active";
const EXCLUSIVE_CLAIM_MODE: &str = "exclusive";
const SHARED_CLAIM_MODE: &str = "shared";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RunnableTasksOptions {
    session_id: SessionId,
    workspace_wide_scope: bool,
}

impl RunnableTasksOptions {
    pub fn new(session_id: SessionId) -> Self {
        Self {
            session_id,
            workspace_wide_scope: false,
        }
    }

    pub fn session_id(&self) -> SessionId {
        self.session_id
    }

    pub(crate) fn with_workspace_wide_scope(mut self) -> Self {
        self.workspace_wide_scope = true;
        self
    }

    fn workspace_wide_scope(&self) -> bool {
        self.workspace_wide_scope
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
    Shared {
        claim_ids: Vec<ClaimId>,
        session_ids: Vec<SessionId>,
        claimed_by_session: bool,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RunnableTaskBlockedReason {
    LifecycleIneligible,
    DependencyBlocked,
    ClaimBlocked,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ProjectionAnchor {
    workspace_id: WorkspaceId,
    branch_id: BranchId,
    head_commit_id: CommitId,
    focus: Option<SessionFocus>,
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

#[derive(Clone, Debug, PartialEq, Eq)]
struct CandidateScope {
    task_entity_ids: Option<BTreeSet<EntityId>>,
    focused: bool,
}

impl CandidateScope {
    fn workspace_wide() -> Self {
        Self {
            task_entity_ids: None,
            focused: false,
        }
    }

    fn focused(task_entity_ids: BTreeSet<EntityId>) -> Self {
        Self {
            task_entity_ids: Some(task_entity_ids),
            focused: true,
        }
    }

    fn includes(&self, task_entity_id: EntityId) -> bool {
        self.task_entity_ids
            .as_ref()
            .is_none_or(|task_entity_ids| task_entity_ids.contains(&task_entity_id))
    }
}

pub(crate) fn runnable_tasks(
    connection: &StoreConnection,
    options: &RunnableTasksOptions,
) -> Result<RunnableTasksProjection> {
    connection.verify_foreign_keys()?;
    let anchor = projection_anchor(connection, options.session_id())?;
    let all_tasks = history::tasks_at(connection, anchor.head_commit_id)?;
    let scheduling_relations =
        history::task_scheduling_relations_at(connection, anchor.head_commit_id)?;
    let dependency_readiness = dependency_readiness_by_task(&all_tasks, &scheduling_relations)?;
    let candidate_scope = if options.workspace_wide_scope() {
        CandidateScope::workspace_wide()
    } else {
        candidate_scope_for_focus(connection, anchor.head_commit_id, anchor.focus.as_ref())?
    };
    let claim_coordination = load_active_claim_coordination(
        connection,
        anchor.workspace_id,
        anchor.branch_id,
        options.session_id(),
    )?;
    let mut candidates = all_tasks
        .into_iter()
        .filter(|task| candidate_scope.includes(task.task_entity_id))
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
    let manual_order = manual_order_rank_by_task(&candidates, &scheduling_relations)?;
    sort_candidates(&mut candidates, &manual_order);
    ensure_projection_anchor_unchanged(connection, options.session_id(), &anchor)?;

    Ok(RunnableTasksProjection {
        session_id: options.session_id(),
        workspace_id: anchor.workspace_id,
        branch_id: anchor.branch_id,
        head_commit_id: anchor.head_commit_id,
        deferred_dimensions: deferred_dimensions(candidate_scope.focused),
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
    let claim_blocked = match &claim_coordination {
        RunnableTaskClaimCoordination::ClaimedByOtherSession { .. } => true,
        RunnableTaskClaimCoordination::Shared {
            claimed_by_session, ..
        } => !claimed_by_session,
        RunnableTaskClaimCoordination::Unclaimed
        | RunnableTaskClaimCoordination::ClaimedBySession { .. } => false,
    };
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

fn candidate_scope_for_focus(
    connection: &StoreConnection,
    commit_id: CommitId,
    focus: Option<&SessionFocus>,
) -> Result<CandidateScope> {
    let Some(focus) = focus else {
        return Ok(CandidateScope::workspace_wide());
    };

    let relations = history::primary_containment_relations_at(connection, commit_id)?;
    validate_primary_containment_graph(&relations)?;
    validate_focus_path(focus, &relations)?;
    let focus_kind = focus_kind_at(connection, commit_id, focus.focus_entity_id)?;
    Ok(CandidateScope::focused(focused_task_entity_ids(
        focus.focus_entity_id,
        focus_kind,
        &relations,
    )))
}

fn focus_kind_at(
    connection: &StoreConnection,
    commit_id: CommitId,
    focus_entity_id: EntityId,
) -> Result<PrimaryContainmentEndpointKind> {
    match history::goal_at(connection, commit_id, focus_entity_id) {
        Ok(_) => Ok(PrimaryContainmentEndpointKind::Goal),
        Err(goal_error) => match history::plan_at(connection, commit_id, focus_entity_id) {
            Ok(_) => Ok(PrimaryContainmentEndpointKind::Plan),
            Err(plan_error) => match history::task_at(connection, commit_id, focus_entity_id) {
                Ok(_) => Ok(PrimaryContainmentEndpointKind::Task),
                Err(task_error) => Err(WorkVcsError::SessionInvalid(format!(
                    "focus entity {focus_entity_id} is not a current Goal, Plan, or Task at branch head {commit_id}: {goal_error}; {plan_error}; {task_error}"
                ))),
            },
        },
    }
}

fn focused_task_entity_ids(
    focus_entity_id: EntityId,
    focus_kind: PrimaryContainmentEndpointKind,
    relations: &[PrimaryContainmentSnapshot],
) -> BTreeSet<EntityId> {
    let mut task_entity_ids = BTreeSet::new();
    if focus_kind == PrimaryContainmentEndpointKind::Task {
        task_entity_ids.insert(focus_entity_id);
    }

    let mut stack = vec![focus_entity_id];
    let mut visited = HashSet::new();
    while let Some(parent_entity_id) = stack.pop() {
        if !visited.insert(parent_entity_id) {
            continue;
        }
        for relation in relations
            .iter()
            .filter(|relation| relation.parent_entity_id == parent_entity_id)
        {
            if relation.child_kind == PrimaryContainmentEndpointKind::Task {
                task_entity_ids.insert(relation.child_entity_id);
            }
            stack.push(relation.child_entity_id);
        }
    }

    task_entity_ids
}

fn validate_focus_path(
    focus: &SessionFocus,
    relations: &[PrimaryContainmentSnapshot],
) -> Result<()> {
    if focus.path.is_empty() {
        return Ok(());
    }
    if focus.path.last().map(|entry| entry.path_entity_id) != Some(focus.focus_entity_id) {
        return Err(WorkVcsError::SessionInvalid(format!(
            "focused runnable projection requires focus path to end at focus entity {}",
            focus.focus_entity_id
        )));
    }

    let relation_by_id = relations
        .iter()
        .map(|relation| (relation.relation_id, relation))
        .collect::<BTreeMap<_, _>>();
    let mut seen_path_entities = HashSet::new();
    for (index, entry) in focus.path.iter().enumerate() {
        if !seen_path_entities.insert(entry.path_entity_id) {
            return Err(WorkVcsError::SessionInvalid(format!(
                "focused runnable projection path repeats entity {}",
                entry.path_entity_id
            )));
        }
        if index == 0 {
            if entry.incoming_relation_id.is_some() {
                return Err(WorkVcsError::SessionInvalid(
                    "first focused runnable projection path entry cannot have an incoming relation"
                        .to_owned(),
                ));
            }
            continue;
        }

        let incoming_relation_id = entry.incoming_relation_id.ok_or_else(|| {
            WorkVcsError::SessionInvalid(format!(
                "focused runnable projection path entry {} has no incoming primary containment relation",
                entry.path_entity_id
            ))
        })?;
        let relation = relation_by_id.get(&incoming_relation_id).ok_or_else(|| {
            WorkVcsError::SessionInvalid(format!(
                "focused runnable projection path relation {incoming_relation_id} is not a current primary containment relation"
            ))
        })?;
        let previous_entity_id = focus.path[index - 1].path_entity_id;
        if relation.parent_entity_id != previous_entity_id
            || relation.child_entity_id != entry.path_entity_id
        {
            return Err(WorkVcsError::SessionInvalid(format!(
                "focused runnable projection path relation {incoming_relation_id} does not connect {previous_entity_id} to {}",
                entry.path_entity_id
            )));
        }
    }

    Ok(())
}

fn validate_primary_containment_graph(relations: &[PrimaryContainmentSnapshot]) -> Result<()> {
    let mut parent_by_child = BTreeMap::new();
    let mut children_by_parent = BTreeMap::<EntityId, Vec<EntityId>>::new();
    for relation in relations {
        if relation.parent_entity_id == relation.child_entity_id {
            return Err(WorkVcsError::RelationInvalid(format!(
                "current primary containment relation {} is a self edge",
                relation.relation_id
            )));
        }
        if let Some(existing_parent) =
            parent_by_child.insert(relation.child_entity_id, relation.parent_entity_id)
        {
            return Err(WorkVcsError::RelationInvalid(format!(
                "entity {} has multiple current primary containment parents: {existing_parent} and {}",
                relation.child_entity_id, relation.parent_entity_id
            )));
        }
        children_by_parent
            .entry(relation.parent_entity_id)
            .or_default()
            .push(relation.child_entity_id);
    }

    let mut visiting = HashSet::new();
    let mut visited = HashSet::new();
    for entity_id in children_by_parent.keys().copied().collect::<Vec<_>>() {
        validate_primary_containment_acyclic(
            entity_id,
            &children_by_parent,
            &mut visiting,
            &mut visited,
        )?;
    }

    Ok(())
}

fn validate_primary_containment_acyclic(
    entity_id: EntityId,
    children_by_parent: &BTreeMap<EntityId, Vec<EntityId>>,
    visiting: &mut HashSet<EntityId>,
    visited: &mut HashSet<EntityId>,
) -> Result<()> {
    if visited.contains(&entity_id) {
        return Ok(());
    }
    if !visiting.insert(entity_id) {
        return Err(WorkVcsError::RelationInvalid(format!(
            "current primary containment graph contains a cycle at entity {entity_id}"
        )));
    }
    if let Some(children) = children_by_parent.get(&entity_id) {
        for child_entity_id in children {
            validate_primary_containment_acyclic(
                *child_entity_id,
                children_by_parent,
                visiting,
                visited,
            )?;
        }
    }
    visiting.remove(&entity_id);
    visited.insert(entity_id);
    Ok(())
}

fn sort_candidates(
    candidates: &mut [RunnableTaskCandidate],
    manual_order: &BTreeMap<EntityId, usize>,
) {
    candidates.sort_by(|left, right| {
        right
            .runnable
            .cmp(&left.runnable)
            .then_with(|| {
                manual_order[&left.task.task_entity_id]
                    .cmp(&manual_order[&right.task.task_entity_id])
            })
            .then_with(|| left.task.task_entity_id.cmp(&right.task.task_entity_id))
    });
}

fn manual_order_rank_by_task(
    candidates: &[RunnableTaskCandidate],
    relations: &[TaskSchedulingRelationSnapshot],
) -> Result<BTreeMap<EntityId, usize>> {
    let candidate_ids = candidates
        .iter()
        .map(|candidate| candidate.task.task_entity_id)
        .collect::<BTreeSet<_>>();
    let mut outgoing = BTreeMap::<EntityId, BTreeSet<EntityId>>::new();
    let mut indegree = candidate_ids
        .iter()
        .map(|task_entity_id| (*task_entity_id, 0usize))
        .collect::<BTreeMap<_, _>>();

    for relation in relations {
        if relation.relation_type != TaskSchedulingRelationType::OrderedBefore
            || !candidate_ids.contains(&relation.source_task_entity_id)
            || !candidate_ids.contains(&relation.target_task_entity_id)
        {
            continue;
        }
        if outgoing
            .entry(relation.source_task_entity_id)
            .or_default()
            .insert(relation.target_task_entity_id)
        {
            *indegree
                .get_mut(&relation.target_task_entity_id)
                .expect("candidate indegree must exist") += 1;
        }
    }

    let mut ready = indegree
        .iter()
        .filter_map(|(task_entity_id, degree)| (*degree == 0).then_some(*task_entity_id))
        .collect::<BTreeSet<_>>();
    let mut ranks = BTreeMap::new();
    while let Some(task_entity_id) = ready.iter().next().copied() {
        ready.remove(&task_entity_id);
        ranks.insert(task_entity_id, ranks.len());
        if let Some(targets) = outgoing.get(&task_entity_id) {
            for target_task_entity_id in targets {
                let degree = indegree
                    .get_mut(target_task_entity_id)
                    .expect("candidate indegree must exist");
                *degree -= 1;
                if *degree == 0 {
                    ready.insert(*target_task_entity_id);
                }
            }
        }
    }

    if ranks.len() != candidate_ids.len() {
        let cycle_task_entity_id = candidate_ids
            .into_iter()
            .find(|task_entity_id| !ranks.contains_key(task_entity_id))
            .expect("cycle task candidate");
        return Err(WorkVcsError::RelationInvalid(format!(
            "current ordered_before graph contains a cycle at task {cycle_task_entity_id}"
        )));
    }
    Ok(ranks)
}

fn projection_anchor(
    connection: &StoreConnection,
    session_id: SessionId,
) -> Result<ProjectionAnchor> {
    let snapshot = session::session_snapshot(connection, session_id)?;
    if snapshot.lifecycle_state != session::SessionLifecycleState::Active {
        return Err(WorkVcsError::SessionInvalid(format!(
            "session {session_id} is not active"
        )));
    }
    let active_workspace_id = snapshot.active_workspace_id.ok_or_else(|| {
        WorkVcsError::SessionInvalid(format!(
            "active session {session_id} has no active workspace"
        ))
    })?;
    let active_branch_id = snapshot.active_branch_id.ok_or_else(|| {
        WorkVcsError::SessionInvalid(format!("active session {session_id} has no active branch"))
    })?;
    let branch_head = history::branch_head(connection, active_branch_id)?;
    if branch_head.workspace_id != active_workspace_id {
        return Err(WorkVcsError::SessionInvalid(format!(
            "session {session_id} active branch {} belongs to workspace {}, not active workspace {}",
            active_branch_id, branch_head.workspace_id, active_workspace_id
        )));
    }
    if branch_head.lifecycle_state != ACTIVE_BRANCH_LIFECYCLE_STATE {
        return Err(WorkVcsError::SessionInvalid(format!(
            "session {session_id} active branch {} has lifecycle state {:?}",
            active_branch_id, branch_head.lifecycle_state
        )));
    }

    Ok(ProjectionAnchor {
        workspace_id: active_workspace_id,
        branch_id: active_branch_id,
        head_commit_id: branch_head.head_commit_id,
        focus: snapshot.focus,
    })
}

fn ensure_projection_anchor_unchanged(
    connection: &StoreConnection,
    session_id: SessionId,
    anchor: &ProjectionAnchor,
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
    if refreshed.focus != anchor.focus {
        return Err(WorkVcsError::SessionInvalid(format!(
            "session {session_id} focus changed before runnable projection completed"
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

    let mut claims_by_task = BTreeMap::<EntityId, Vec<ActiveClaimRow>>::new();
    for row in rows {
        let (claim_id, session_id, task_entity_id, mode) = row.map_err(storage_error)?;
        let claim_id = decode_claim_id("claim.claim_id", claim_id)?;
        let session_id = decode_session_id("claim.session_id", session_id)?;
        let task_entity_id = decode_entity_id("claim.task_entity_id", task_entity_id)?;
        let mode = parse_active_claim_mode(&mode)?;
        claims_by_task
            .entry(task_entity_id)
            .or_default()
            .push(ActiveClaimRow {
                claim_id,
                session_id,
                mode,
            });
    }
    let mut claims = BTreeMap::new();
    for (task_entity_id, mut active_claims) in claims_by_task {
        active_claims.sort_by_key(|claim| claim.claim_id);
        let coordination = active_claim_coordination(
            task_entity_id,
            branch_id,
            requesting_session_id,
            &active_claims,
        )?;
        claims.insert(task_entity_id, coordination);
    }
    Ok(claims)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ActiveClaimMode {
    Exclusive,
    Shared,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ActiveClaimRow {
    claim_id: ClaimId,
    session_id: SessionId,
    mode: ActiveClaimMode,
}

fn active_claim_coordination(
    task_entity_id: EntityId,
    branch_id: BranchId,
    requesting_session_id: SessionId,
    active_claims: &[ActiveClaimRow],
) -> Result<RunnableTaskClaimCoordination> {
    let mut exclusive_claim = None;
    let mut shared_claim_ids = Vec::new();
    let mut shared_session_ids = Vec::new();
    let mut seen_shared_sessions = BTreeSet::new();

    for active_claim in active_claims {
        match active_claim.mode {
            ActiveClaimMode::Exclusive => {
                if exclusive_claim.replace(active_claim).is_some() {
                    return Err(WorkVcsError::ClaimInvalid(format!(
                        "task {task_entity_id} has more than one active exclusive claim on branch {branch_id}"
                    )));
                }
            }
            ActiveClaimMode::Shared => {
                if !seen_shared_sessions.insert(active_claim.session_id) {
                    return Err(WorkVcsError::ClaimInvalid(format!(
                        "task {task_entity_id} has multiple active shared claims for session {} on branch {branch_id}",
                        active_claim.session_id
                    )));
                }
                shared_claim_ids.push(active_claim.claim_id);
                shared_session_ids.push(active_claim.session_id);
            }
        }
    }

    if let Some(exclusive_claim) = exclusive_claim {
        if !shared_claim_ids.is_empty() {
            return Err(WorkVcsError::ClaimInvalid(format!(
                "task {task_entity_id} has mixed active exclusive and shared claims on branch {branch_id}"
            )));
        }
        return Ok(if exclusive_claim.session_id == requesting_session_id {
            RunnableTaskClaimCoordination::ClaimedBySession {
                claim_id: exclusive_claim.claim_id,
            }
        } else {
            RunnableTaskClaimCoordination::ClaimedByOtherSession {
                claim_id: exclusive_claim.claim_id,
                session_id: exclusive_claim.session_id,
            }
        });
    }

    if shared_claim_ids.is_empty() {
        return Ok(RunnableTaskClaimCoordination::Unclaimed);
    }
    Ok(RunnableTaskClaimCoordination::Shared {
        claimed_by_session: shared_session_ids.contains(&requesting_session_id),
        claim_ids: shared_claim_ids,
        session_ids: shared_session_ids,
    })
}

fn parse_active_claim_mode(value: &str) -> Result<ActiveClaimMode> {
    match value {
        EXCLUSIVE_CLAIM_MODE => Ok(ActiveClaimMode::Exclusive),
        SHARED_CLAIM_MODE => Ok(ActiveClaimMode::Shared),
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

fn deferred_dimensions(focused: bool) -> Vec<RunnableTaskProjectionDimension> {
    let mut dimensions = Vec::new();
    if !focused {
        dimensions.push(RunnableTaskProjectionDimension::ActiveScopePlanPath);
        dimensions.push(RunnableTaskProjectionDimension::ExecutableTaskDescendants);
    }
    dimensions
}
