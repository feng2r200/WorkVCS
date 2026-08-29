use super::{branch_head, state_at};
use crate::canonical::WorkState;
use crate::error::{Result, WorkVcsError};
use crate::identity::{
    BranchId, CommitId, Digest, EntityId, EntityVersionId, RelationId, RelationVersionId,
    WorkspaceId,
};
use crate::store::StoreConnection;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorkStateDiffTarget {
    BranchHead(BranchId),
    Commit(CommitId),
}

impl WorkStateDiffTarget {
    pub fn branch_head(branch_id: BranchId) -> Self {
        Self::BranchHead(branch_id)
    }

    pub fn commit(commit_id: CommitId) -> Self {
        Self::Commit(commit_id)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkStateDiffOptions {
    from: WorkStateDiffTarget,
    to: WorkStateDiffTarget,
}

impl WorkStateDiffOptions {
    pub fn new(from: WorkStateDiffTarget, to: WorkStateDiffTarget) -> Self {
        Self { from, to }
    }

    pub fn from(&self) -> WorkStateDiffTarget {
        self.from
    }

    pub fn to(&self) -> WorkStateDiffTarget {
        self.to
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkStateDiff {
    pub from: ResolvedWorkStateDiffTarget,
    pub to: ResolvedWorkStateDiffTarget,
    pub entity_changes: Vec<EntityVersionDiff>,
    pub relation_changes: Vec<RelationVersionDiff>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ResolvedWorkStateDiffTarget {
    pub target: WorkStateDiffTarget,
    pub workspace_id: WorkspaceId,
    pub commit_id: CommitId,
    pub state_digest: Digest,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorkStateDiffChangeKind {
    Added,
    Removed,
    Updated,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EntityVersionDiff {
    pub entity_id: EntityId,
    pub change_kind: WorkStateDiffChangeKind,
    pub before_entity_version_id: Option<EntityVersionId>,
    pub after_entity_version_id: Option<EntityVersionId>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RelationVersionDiff {
    pub relation_id: RelationId,
    pub change_kind: WorkStateDiffChangeKind,
    pub before_relation_version_id: Option<RelationVersionId>,
    pub after_relation_version_id: Option<RelationVersionId>,
}

pub(crate) fn diff_work_state(
    connection: &StoreConnection,
    options: &WorkStateDiffOptions,
) -> Result<WorkStateDiff> {
    let from = resolve_target(connection, options.from())?;
    let to = resolve_target(connection, options.to())?;
    if from.resolved.workspace_id != to.resolved.workspace_id {
        return Err(WorkVcsError::QueryInvalid(format!(
            "cannot diff WorkStates from different workspaces: {} and {}",
            from.resolved.workspace_id, to.resolved.workspace_id
        )));
    }

    Ok(WorkStateDiff {
        from: from.resolved,
        to: to.resolved,
        entity_changes: diff_entities(&from.state, &to.state),
        relation_changes: diff_relations(&from.state, &to.state),
    })
}

struct ResolvedTargetWithState {
    resolved: ResolvedWorkStateDiffTarget,
    state: WorkState,
}

fn resolve_target(
    connection: &StoreConnection,
    target: WorkStateDiffTarget,
) -> Result<ResolvedTargetWithState> {
    let commit_id = match target {
        WorkStateDiffTarget::BranchHead(branch_id) => {
            branch_head(connection, branch_id)?.head_commit_id
        }
        WorkStateDiffTarget::Commit(commit_id) => commit_id,
    };
    let replayed = state_at(connection, commit_id)?;

    Ok(ResolvedTargetWithState {
        resolved: ResolvedWorkStateDiffTarget {
            target,
            workspace_id: replayed.workspace_id,
            commit_id,
            state_digest: replayed.state_digest,
        },
        state: replayed.state,
    })
}

fn diff_entities(from: &WorkState, to: &WorkState) -> Vec<EntityVersionDiff> {
    let before = from.entities().iter().copied().collect::<BTreeMap<_, _>>();
    let after = to.entities().iter().copied().collect::<BTreeMap<_, _>>();
    let entity_ids = before
        .keys()
        .chain(after.keys())
        .copied()
        .collect::<BTreeSet<_>>();

    entity_ids
        .into_iter()
        .filter_map(|entity_id| {
            let before_entity_version_id = before.get(&entity_id).copied();
            let after_entity_version_id = after.get(&entity_id).copied();
            let change_kind = change_kind(before_entity_version_id, after_entity_version_id)?;
            Some(EntityVersionDiff {
                entity_id,
                change_kind,
                before_entity_version_id,
                after_entity_version_id,
            })
        })
        .collect()
}

fn diff_relations(from: &WorkState, to: &WorkState) -> Vec<RelationVersionDiff> {
    let before = from.relations().iter().copied().collect::<BTreeMap<_, _>>();
    let after = to.relations().iter().copied().collect::<BTreeMap<_, _>>();
    let relation_ids = before
        .keys()
        .chain(after.keys())
        .copied()
        .collect::<BTreeSet<_>>();

    relation_ids
        .into_iter()
        .filter_map(|relation_id| {
            let before_relation_version_id = before.get(&relation_id).copied();
            let after_relation_version_id = after.get(&relation_id).copied();
            let change_kind = change_kind(before_relation_version_id, after_relation_version_id)?;
            Some(RelationVersionDiff {
                relation_id,
                change_kind,
                before_relation_version_id,
                after_relation_version_id,
            })
        })
        .collect()
}

fn change_kind<T: Eq>(before: Option<T>, after: Option<T>) -> Option<WorkStateDiffChangeKind> {
    match (before, after) {
        (None, None) => None,
        (None, Some(_)) => Some(WorkStateDiffChangeKind::Added),
        (Some(_), None) => Some(WorkStateDiffChangeKind::Removed),
        (Some(before), Some(after)) if before != after => Some(WorkStateDiffChangeKind::Updated),
        (Some(_), Some(_)) => None,
    }
}
