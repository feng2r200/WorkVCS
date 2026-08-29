use super::{
    PrimaryContainmentEndpointKind, StructuralReferenceEndpointKind, VerificationTarget,
    branch_head, primary_containment_relations_at, state_at, structural_references_at,
    verification_relations_at,
};
use crate::error::{Result, WorkVcsError};
use crate::identity::{
    BranchId, CommitId, Digest, EntityId, EntityVersionId, RelationId, RelationVersionId,
    WorkspaceId,
};
use crate::store::StoreConnection;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WhyQueryTarget {
    BranchHead(BranchId),
    Commit(CommitId),
}

impl WhyQueryTarget {
    pub fn branch_head(branch_id: BranchId) -> Self {
        Self::BranchHead(branch_id)
    }

    pub fn commit(commit_id: CommitId) -> Self {
        Self::Commit(commit_id)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WhyQueryOptions {
    target: WhyQueryTarget,
    subject_entity_id: EntityId,
}

impl WhyQueryOptions {
    pub fn new(target: WhyQueryTarget, subject_entity_id: EntityId) -> Self {
        Self {
            target,
            subject_entity_id,
        }
    }

    pub fn target(&self) -> WhyQueryTarget {
        self.target
    }

    pub fn subject_entity_id(&self) -> EntityId {
        self.subject_entity_id
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WhyQueryResult {
    pub target: ResolvedWhyQueryTarget,
    pub subject_entity_id: EntityId,
    pub subject_entity_version_id: EntityVersionId,
    pub relation_edges: Vec<WhyRelationEdge>,
    pub deferred_relation_families: Vec<WhyDeferredRelationFamily>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ResolvedWhyQueryTarget {
    pub target: WhyQueryTarget,
    pub workspace_id: WorkspaceId,
    pub commit_id: CommitId,
    pub state_digest: Digest,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum WhyRelationKind {
    PrimaryContainment,
    StructuralReference,
    Verifies,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum WhyRelationDirection {
    Incoming,
    Outgoing,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum WhyEntityKind {
    Goal,
    Plan,
    Task,
    AcceptanceCriterion,
    VerificationRequirement,
    Verification,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum WhyDeferredRelationFamily {
    Evolution,
    Epistemic,
    VerificationEvidence,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WhyRelationEdge {
    pub relation_kind: WhyRelationKind,
    pub direction: WhyRelationDirection,
    pub relation_id: RelationId,
    pub relation_version_id: RelationVersionId,
    pub source_entity_id: EntityId,
    pub source_kind: WhyEntityKind,
    pub target_entity_id: EntityId,
    pub target_kind: WhyEntityKind,
    pub state_digest: Digest,
}

pub(crate) fn explain_why(
    connection: &StoreConnection,
    options: &WhyQueryOptions,
) -> Result<WhyQueryResult> {
    let resolved = resolve_target(connection, options.target())?;
    let subject_entity_version_id = resolved
        .state
        .entities()
        .iter()
        .find_map(|(entity_id, entity_version_id)| {
            (*entity_id == options.subject_entity_id()).then_some(*entity_version_id)
        })
        .ok_or_else(|| {
            WorkVcsError::QueryInvalid(format!(
                "entity {} is not current in WorkState commit {}",
                options.subject_entity_id(),
                resolved.target.commit_id
            ))
        })?;

    let mut relation_edges = Vec::new();
    for relation in primary_containment_relations_at(connection, resolved.target.commit_id)? {
        if relation.parent_entity_id == options.subject_entity_id()
            || relation.child_entity_id == options.subject_entity_id()
        {
            relation_edges.push(WhyRelationEdge {
                relation_kind: WhyRelationKind::PrimaryContainment,
                direction: relation_direction(
                    options.subject_entity_id(),
                    relation.parent_entity_id,
                    relation.child_entity_id,
                ),
                relation_id: relation.relation_id,
                relation_version_id: relation.relation_version_id,
                source_entity_id: relation.parent_entity_id,
                source_kind: relation.parent_kind.into(),
                target_entity_id: relation.child_entity_id,
                target_kind: relation.child_kind.into(),
                state_digest: relation.state_digest,
            });
        }
    }
    for relation in structural_references_at(connection, resolved.target.commit_id)? {
        if relation.referrer_entity_id == options.subject_entity_id()
            || relation.target_entity_id == options.subject_entity_id()
        {
            relation_edges.push(WhyRelationEdge {
                relation_kind: WhyRelationKind::StructuralReference,
                direction: relation_direction(
                    options.subject_entity_id(),
                    relation.referrer_entity_id,
                    relation.target_entity_id,
                ),
                relation_id: relation.relation_id,
                relation_version_id: relation.relation_version_id,
                source_entity_id: relation.referrer_entity_id,
                source_kind: relation.referrer_kind.into(),
                target_entity_id: relation.target_entity_id,
                target_kind: relation.target_kind.into(),
                state_digest: relation.state_digest,
            });
        }
    }
    for relation in verification_relations_at(connection, resolved.target.commit_id)? {
        let target_entity_id = relation.target.entity_id();
        if relation.source_verification_entity_id == options.subject_entity_id()
            || target_entity_id == options.subject_entity_id()
        {
            relation_edges.push(WhyRelationEdge {
                relation_kind: WhyRelationKind::Verifies,
                direction: relation_direction(
                    options.subject_entity_id(),
                    relation.source_verification_entity_id,
                    target_entity_id,
                ),
                relation_id: relation.relation_id,
                relation_version_id: relation.relation_version_id,
                source_entity_id: relation.source_verification_entity_id,
                source_kind: WhyEntityKind::Verification,
                target_entity_id,
                target_kind: relation.target.into(),
                state_digest: relation.state_digest,
            });
        }
    }
    relation_edges.sort_by(|left, right| {
        left.relation_kind
            .cmp(&right.relation_kind)
            .then_with(|| left.direction.cmp(&right.direction))
            .then_with(|| left.source_kind.cmp(&right.source_kind))
            .then_with(|| left.source_entity_id.cmp(&right.source_entity_id))
            .then_with(|| left.target_kind.cmp(&right.target_kind))
            .then_with(|| left.target_entity_id.cmp(&right.target_entity_id))
            .then_with(|| left.relation_id.cmp(&right.relation_id))
    });

    Ok(WhyQueryResult {
        target: resolved.target,
        subject_entity_id: options.subject_entity_id(),
        subject_entity_version_id,
        relation_edges,
        deferred_relation_families: vec![
            WhyDeferredRelationFamily::Evolution,
            WhyDeferredRelationFamily::Epistemic,
            WhyDeferredRelationFamily::VerificationEvidence,
        ],
    })
}

struct ResolvedWhyTargetWithState {
    target: ResolvedWhyQueryTarget,
    state: crate::canonical::WorkState,
}

fn resolve_target(
    connection: &StoreConnection,
    target: WhyQueryTarget,
) -> Result<ResolvedWhyTargetWithState> {
    let commit_id = match target {
        WhyQueryTarget::BranchHead(branch_id) => branch_head(connection, branch_id)?.head_commit_id,
        WhyQueryTarget::Commit(commit_id) => commit_id,
    };
    let replayed = state_at(connection, commit_id)?;

    Ok(ResolvedWhyTargetWithState {
        target: ResolvedWhyQueryTarget {
            target,
            workspace_id: replayed.workspace_id,
            commit_id,
            state_digest: replayed.state_digest,
        },
        state: replayed.state,
    })
}

fn relation_direction(
    subject_entity_id: EntityId,
    source_entity_id: EntityId,
    target_entity_id: EntityId,
) -> WhyRelationDirection {
    debug_assert!(subject_entity_id == source_entity_id || subject_entity_id == target_entity_id);
    if subject_entity_id == source_entity_id {
        WhyRelationDirection::Outgoing
    } else {
        WhyRelationDirection::Incoming
    }
}

impl From<PrimaryContainmentEndpointKind> for WhyEntityKind {
    fn from(value: PrimaryContainmentEndpointKind) -> Self {
        match value {
            PrimaryContainmentEndpointKind::Goal => Self::Goal,
            PrimaryContainmentEndpointKind::Plan => Self::Plan,
            PrimaryContainmentEndpointKind::Task => Self::Task,
        }
    }
}

impl From<StructuralReferenceEndpointKind> for WhyEntityKind {
    fn from(value: StructuralReferenceEndpointKind) -> Self {
        match value {
            StructuralReferenceEndpointKind::Goal => Self::Goal,
            StructuralReferenceEndpointKind::Plan => Self::Plan,
            StructuralReferenceEndpointKind::Task => Self::Task,
        }
    }
}

impl From<VerificationTarget> for WhyEntityKind {
    fn from(value: VerificationTarget) -> Self {
        match value {
            VerificationTarget::AcceptanceCriterion(_) => Self::AcceptanceCriterion,
            VerificationTarget::VerificationRequirement(_) => Self::VerificationRequirement,
        }
    }
}
