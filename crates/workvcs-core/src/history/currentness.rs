use super::{
    GoalStatus, PlanStatus, ReplayCache, TaskStatus, goals_at_with_cache, plans_at_with_cache,
    primary_containment_relations_at_with_cache, state_at_with_cache,
    structural_references_at_with_cache, tasks_at_with_cache,
};
use crate::error::Result;
use crate::identity::{CommitId, EntityId, RelationId, RelationVersionId, WorkspaceId};
use crate::store::StoreConnection;
use std::collections::{BTreeMap, BTreeSet, VecDeque};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum CurrentEntityKind {
    Goal,
    Plan,
    Task,
}

impl CurrentEntityKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Goal => "goal",
            Self::Plan => "plan",
            Self::Task => "task",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CurrentnessCarrierKind {
    ActiveGoalRoot,
    StandalonePlanRoot,
    StandaloneTaskRoot,
    PrimaryContainment,
    StructuralReference,
}

impl CurrentnessCarrierKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ActiveGoalRoot => "active_goal_root",
            Self::StandalonePlanRoot => "standalone_plan_root",
            Self::StandaloneTaskRoot => "standalone_task_root",
            Self::PrimaryContainment => "primary_containment",
            Self::StructuralReference => "structural_reference",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CurrentnessCarrier {
    pub kind: CurrentnessCarrierKind,
    pub source_entity_id: Option<EntityId>,
    pub relation_id: Option<RelationId>,
    pub relation_version_id: Option<RelationVersionId>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CurrentScopeProjection {
    pub workspace_id: WorkspaceId,
    pub commit_id: CommitId,
    pub current_goals: BTreeSet<EntityId>,
    pub current_plans: BTreeSet<EntityId>,
    pub current_tasks: BTreeSet<EntityId>,
    pub carriers: BTreeMap<EntityId, Vec<CurrentnessCarrier>>,
}

impl CurrentScopeProjection {
    pub fn is_current(&self, kind: CurrentEntityKind, entity_id: EntityId) -> bool {
        match kind {
            CurrentEntityKind::Goal => self.current_goals.contains(&entity_id),
            CurrentEntityKind::Plan => self.current_plans.contains(&entity_id),
            CurrentEntityKind::Task => self.current_tasks.contains(&entity_id),
        }
    }

    pub fn carriers_for(&self, entity_id: EntityId) -> &[CurrentnessCarrier] {
        self.carriers
            .get(&entity_id)
            .map(Vec::as_slice)
            .unwrap_or_default()
    }

    /// Returns whether the entity is on a path reached from a current root.
    /// Terminal Tasks are reachable structural containers even though they are
    /// not themselves current execution candidates.
    pub fn is_scope_reachable(&self, entity_id: EntityId) -> bool {
        self.carriers.contains_key(&entity_id)
    }

    pub fn has_relation_carrier(
        &self,
        target_entity_id: EntityId,
        kind: CurrentnessCarrierKind,
        source_entity_id: EntityId,
        relation_id: RelationId,
        relation_version_id: RelationVersionId,
    ) -> bool {
        self.carriers_for(target_entity_id).iter().any(|carrier| {
            carrier.kind == kind
                && carrier.source_entity_id == Some(source_entity_id)
                && carrier.relation_id == Some(relation_id)
                && carrier.relation_version_id == Some(relation_version_id)
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CurrentnessNodeStatus {
    Goal(GoalStatus),
    Plan(PlanStatus),
    Task(TaskStatus),
}

impl CurrentnessNodeStatus {
    fn kind(self) -> CurrentEntityKind {
        match self {
            Self::Goal(_) => CurrentEntityKind::Goal,
            Self::Plan(_) => CurrentEntityKind::Plan,
            Self::Task(_) => CurrentEntityKind::Task,
        }
    }

    fn eligible_current(self) -> bool {
        match self {
            Self::Goal(status) => status == GoalStatus::Active,
            Self::Plan(status) => status == PlanStatus::Active,
            Self::Task(status) => !status.is_terminal(),
        }
    }

    fn can_propagate_containment(self) -> bool {
        match self {
            Self::Goal(status) => status == GoalStatus::Active,
            Self::Plan(status) => status == PlanStatus::Active,
            // A terminal intermediate Task remains structural scope. It does not
            // become current itself, but it must not cut off live descendants.
            Self::Task(_) => true,
        }
    }

    fn can_propagate_reference(self) -> bool {
        matches!(
            self,
            Self::Goal(GoalStatus::Active) | Self::Plan(PlanStatus::Active)
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct CurrentnessEdge {
    pub(crate) source: EntityId,
    pub(crate) target: EntityId,
    pub(crate) relation_id: RelationId,
    pub(crate) relation_version_id: RelationVersionId,
    pub(crate) carrier_kind: CurrentnessCarrierKind,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct CurrentScopeOverrides {
    pub(crate) entity_statuses: BTreeMap<EntityId, CurrentnessNodeStatus>,
    pub(crate) added_edges: Vec<CurrentnessEdge>,
}

pub(crate) fn current_scope_at(
    connection: &StoreConnection,
    commit_id: CommitId,
) -> Result<CurrentScopeProjection> {
    current_scope_at_with_overrides(connection, commit_id, &CurrentScopeOverrides::default())
}

pub(crate) fn current_scope_at_with_overrides(
    connection: &StoreConnection,
    commit_id: CommitId,
    overrides: &CurrentScopeOverrides,
) -> Result<CurrentScopeProjection> {
    let mut replay_cache = ReplayCache::default();
    let goals = goals_at_with_cache(connection, commit_id, &mut replay_cache)?;
    let plans = plans_at_with_cache(connection, commit_id, &mut replay_cache)?;
    let tasks = tasks_at_with_cache(connection, commit_id, &mut replay_cache)?;
    let containment =
        primary_containment_relations_at_with_cache(connection, commit_id, &mut replay_cache)?;
    let references = structural_references_at_with_cache(connection, commit_id, &mut replay_cache)?;

    let workspace_id = state_at_with_cache(connection, commit_id, &mut replay_cache)?.workspace_id;

    let mut nodes = BTreeMap::new();
    for goal in goals {
        nodes.insert(
            goal.goal_entity_id,
            CurrentnessNodeStatus::Goal(goal.state.status),
        );
    }
    for plan in plans {
        nodes.insert(
            plan.plan_entity_id,
            CurrentnessNodeStatus::Plan(plan.state.status),
        );
    }
    for task in tasks {
        nodes.insert(
            task.task_entity_id,
            CurrentnessNodeStatus::Task(task.state.status),
        );
    }
    nodes.extend(
        overrides
            .entity_statuses
            .iter()
            .map(|(id, status)| (*id, *status)),
    );

    let mut primary_parent = BTreeMap::new();
    let mut containment_by_source = BTreeMap::<EntityId, Vec<CurrentnessEdge>>::new();
    for relation in containment {
        if !nodes.contains_key(&relation.parent_entity_id)
            || !nodes.contains_key(&relation.child_entity_id)
        {
            return Err(crate::WorkVcsError::IntegrityInvalid(format!(
                "primary containment relation {} has a missing endpoint",
                relation.relation_id
            )));
        }
        if let Some(existing) =
            primary_parent.insert(relation.child_entity_id, relation.parent_entity_id)
            && existing != relation.parent_entity_id
        {
            return Err(crate::WorkVcsError::IntegrityInvalid(format!(
                "entity {} has conflicting primary parents {existing} and {}",
                relation.child_entity_id, relation.parent_entity_id
            )));
        }
        containment_by_source
            .entry(relation.parent_entity_id)
            .or_default()
            .push(CurrentnessEdge {
                source: relation.parent_entity_id,
                target: relation.child_entity_id,
                relation_id: relation.relation_id,
                relation_version_id: relation.relation_version_id,
                carrier_kind: CurrentnessCarrierKind::PrimaryContainment,
            });
    }
    let mut references_by_source = BTreeMap::<EntityId, Vec<CurrentnessEdge>>::new();
    for relation in references {
        if !nodes.contains_key(&relation.referrer_entity_id)
            || !nodes.contains_key(&relation.target_entity_id)
        {
            return Err(crate::WorkVcsError::IntegrityInvalid(format!(
                "structural reference relation {} has a missing endpoint",
                relation.relation_id
            )));
        }
        references_by_source
            .entry(relation.referrer_entity_id)
            .or_default()
            .push(CurrentnessEdge {
                source: relation.referrer_entity_id,
                target: relation.target_entity_id,
                relation_id: relation.relation_id,
                relation_version_id: relation.relation_version_id,
                carrier_kind: CurrentnessCarrierKind::StructuralReference,
            });
    }
    for edge in &overrides.added_edges {
        if !nodes.contains_key(&edge.source) || !nodes.contains_key(&edge.target) {
            return Err(crate::WorkVcsError::IntegrityInvalid(format!(
                "currentness override relation {} has a missing endpoint",
                edge.relation_id
            )));
        }
        match edge.carrier_kind {
            CurrentnessCarrierKind::PrimaryContainment => {
                if let Some(existing) = primary_parent.insert(edge.target, edge.source)
                    && existing != edge.source
                {
                    return Err(crate::WorkVcsError::IntegrityInvalid(format!(
                        "entity {} has conflicting primary parents {existing} and {}",
                        edge.target, edge.source
                    )));
                }
                containment_by_source
                    .entry(edge.source)
                    .or_default()
                    .push(*edge);
            }
            CurrentnessCarrierKind::StructuralReference => {
                references_by_source
                    .entry(edge.source)
                    .or_default()
                    .push(*edge);
            }
            _ => {
                return Err(crate::WorkVcsError::IntegrityInvalid(format!(
                    "currentness override relation {} has non-relation carrier kind {}",
                    edge.relation_id,
                    edge.carrier_kind.as_str()
                )));
            }
        }
    }
    validate_containment_acyclic(&containment_by_source)?;

    let mut current_goals = BTreeSet::new();
    let mut current_plans = BTreeSet::new();
    let mut current_tasks = BTreeSet::new();
    let mut carriers = BTreeMap::<EntityId, Vec<CurrentnessCarrier>>::new();
    let mut reached = BTreeSet::new();
    let mut queue = VecDeque::new();

    for (&entity_id, &status) in &nodes {
        let root = match status {
            CurrentnessNodeStatus::Goal(GoalStatus::Active) => {
                Some(CurrentnessCarrierKind::ActiveGoalRoot)
            }
            CurrentnessNodeStatus::Plan(PlanStatus::Active)
                if !primary_parent.contains_key(&entity_id) =>
            {
                Some(CurrentnessCarrierKind::StandalonePlanRoot)
            }
            CurrentnessNodeStatus::Task(task_status)
                if !task_status.is_terminal() && !primary_parent.contains_key(&entity_id) =>
            {
                Some(CurrentnessCarrierKind::StandaloneTaskRoot)
            }
            _ => None,
        };
        if let Some(kind) = root {
            reached.insert(entity_id);
            queue.push_back(entity_id);
            carriers
                .entry(entity_id)
                .or_default()
                .push(CurrentnessCarrier {
                    kind,
                    source_entity_id: None,
                    relation_id: None,
                    relation_version_id: None,
                });
        }
    }

    while let Some(source) = queue.pop_front() {
        let Some(source_status) = nodes.get(&source).copied() else {
            continue;
        };
        if source_status.eligible_current() {
            match source_status.kind() {
                CurrentEntityKind::Goal => {
                    current_goals.insert(source);
                }
                CurrentEntityKind::Plan => {
                    current_plans.insert(source);
                }
                CurrentEntityKind::Task => {
                    current_tasks.insert(source);
                }
            }
        }

        if source_status.can_propagate_containment()
            && let Some(edges) = containment_by_source.get(&source)
        {
            propagate_edges(
                edges,
                source,
                &nodes,
                &mut reached,
                &mut queue,
                &mut carriers,
            );
        }
        if source_status.can_propagate_reference()
            && let Some(edges) = references_by_source.get(&source)
        {
            propagate_edges(
                edges,
                source,
                &nodes,
                &mut reached,
                &mut queue,
                &mut carriers,
            );
        }
    }

    for values in carriers.values_mut() {
        values.sort_by_key(|value| {
            (
                value.kind.as_str(),
                value.source_entity_id,
                value.relation_id,
                value.relation_version_id,
            )
        });
        values.dedup();
    }

    Ok(CurrentScopeProjection {
        workspace_id,
        commit_id,
        current_goals,
        current_plans,
        current_tasks,
        carriers,
    })
}

fn propagate_edges(
    edges: &[CurrentnessEdge],
    source: EntityId,
    nodes: &BTreeMap<EntityId, CurrentnessNodeStatus>,
    reached: &mut BTreeSet<EntityId>,
    queue: &mut VecDeque<EntityId>,
    carriers: &mut BTreeMap<EntityId, Vec<CurrentnessCarrier>>,
) {
    for edge in edges {
        let Some(target_status) = nodes.get(&edge.target).copied() else {
            continue;
        };
        // Terminal Goal/Plan nodes are retirement boundaries. Tasks are always
        // traversable so a terminal intermediate Task cannot hide live children.
        let traversable = match target_status {
            CurrentnessNodeStatus::Goal(status) => status == GoalStatus::Active,
            CurrentnessNodeStatus::Plan(status) => status == PlanStatus::Active,
            CurrentnessNodeStatus::Task(_) => true,
        };
        if !traversable {
            continue;
        }
        carriers
            .entry(edge.target)
            .or_default()
            .push(CurrentnessCarrier {
                kind: edge.carrier_kind,
                source_entity_id: Some(source),
                relation_id: Some(edge.relation_id),
                relation_version_id: Some(edge.relation_version_id),
            });
        if reached.insert(edge.target) {
            queue.push_back(edge.target);
        }
    }
}

fn validate_containment_acyclic(edges: &BTreeMap<EntityId, Vec<CurrentnessEdge>>) -> Result<()> {
    fn visit(
        node: EntityId,
        edges: &BTreeMap<EntityId, Vec<CurrentnessEdge>>,
        visiting: &mut BTreeSet<EntityId>,
        visited: &mut BTreeSet<EntityId>,
    ) -> Result<()> {
        if visited.contains(&node) {
            return Ok(());
        }
        if !visiting.insert(node) {
            return Err(crate::WorkVcsError::IntegrityInvalid(format!(
                "primary containment graph contains a cycle at entity {node}"
            )));
        }
        if let Some(children) = edges.get(&node) {
            for child in children {
                visit(child.target, edges, visiting, visited)?;
            }
        }
        visiting.remove(&node);
        visited.insert(node);
        Ok(())
    }

    let mut visiting = BTreeSet::new();
    let mut visited = BTreeSet::new();
    for node in edges.keys().copied().collect::<Vec<_>>() {
        visit(node, edges, &mut visiting, &mut visited)?;
    }
    Ok(())
}
