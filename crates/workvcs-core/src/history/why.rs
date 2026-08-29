use super::goal::GOAL_ENTITY_KIND;
use super::knowledge::KNOWLEDGE_ENTITY_KIND;
use super::plan::PLAN_ENTITY_KIND;
use super::record::RECORD_ENTITY_KIND;
use super::task::{
    ACCEPTANCE_CRITERION_ENTITY_KIND, TASK_ENTITY_KIND, VERIFICATION_ENTITY_KIND,
    VERIFICATION_REQUIREMENT_ENTITY_KIND,
};
use super::{
    PrimaryContainmentEndpointKind, RecordRelationListOptions, RecordRelationType,
    StructuralReferenceEndpointKind, VerificationTarget, branch_head, evidence,
    primary_containment_relations_at, record_relations_at, state_at, structural_references_at,
    verification_evidence_relations_at, verification_relations_at,
};
use crate::error::{Result, WorkVcsError};
use crate::identity::{
    BranchId, CommitId, Digest, EntityId, EntityVersionId, EvidenceId, RelationId,
    RelationVersionId, WorkspaceId,
};
use crate::store::StoreConnection;
use rusqlite::{OptionalExtension, params};

const ENTITY_OBJECT_KIND: &str = "entity";

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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WhyQuerySubject {
    Entity(EntityId),
    Evidence(EvidenceId),
}

impl WhyQuerySubject {
    pub fn entity(entity_id: EntityId) -> Self {
        Self::Entity(entity_id)
    }

    pub fn evidence(evidence_id: EvidenceId) -> Self {
        Self::Evidence(evidence_id)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WhyQueryOptions {
    target: WhyQueryTarget,
    subject: WhyQuerySubject,
}

impl WhyQueryOptions {
    pub fn new(target: WhyQueryTarget, subject_entity_id: EntityId) -> Self {
        Self {
            target,
            subject: WhyQuerySubject::Entity(subject_entity_id),
        }
    }

    pub fn for_entity(target: WhyQueryTarget, subject_entity_id: EntityId) -> Self {
        Self::new(target, subject_entity_id)
    }

    pub fn for_evidence(target: WhyQueryTarget, subject_evidence_id: EvidenceId) -> Self {
        Self {
            target,
            subject: WhyQuerySubject::Evidence(subject_evidence_id),
        }
    }

    pub fn target(&self) -> WhyQueryTarget {
        self.target
    }

    pub fn subject(&self) -> WhyQuerySubject {
        self.subject
    }

    pub fn subject_entity_id(&self) -> Option<EntityId> {
        match self.subject {
            WhyQuerySubject::Entity(entity_id) => Some(entity_id),
            WhyQuerySubject::Evidence(_) => None,
        }
    }

    pub fn subject_evidence_id(&self) -> Option<EvidenceId> {
        match self.subject {
            WhyQuerySubject::Entity(_) => None,
            WhyQuerySubject::Evidence(evidence_id) => Some(evidence_id),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WhyQueryResult {
    pub target: ResolvedWhyQueryTarget,
    pub subject: ResolvedWhyQuerySubject,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResolvedWhyQuerySubject {
    Entity {
        entity_id: EntityId,
        entity_version_id: EntityVersionId,
        entity_kind: WhyEntityKind,
    },
    Evidence {
        evidence_id: EvidenceId,
    },
}

impl ResolvedWhyQuerySubject {
    pub fn entity_id(self) -> Option<EntityId> {
        match self {
            Self::Entity { entity_id, .. } => Some(entity_id),
            Self::Evidence { .. } => None,
        }
    }

    pub fn entity_version_id(self) -> Option<EntityVersionId> {
        match self {
            Self::Entity {
                entity_version_id, ..
            } => Some(entity_version_id),
            Self::Evidence { .. } => None,
        }
    }

    pub fn entity_kind(self) -> Option<WhyEntityKind> {
        match self {
            Self::Entity { entity_kind, .. } => Some(entity_kind),
            Self::Evidence { .. } => None,
        }
    }

    pub fn evidence_id(self) -> Option<EvidenceId> {
        match self {
            Self::Entity { .. } => None,
            Self::Evidence { evidence_id } => Some(evidence_id),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum WhyRelationKind {
    PrimaryContainment,
    StructuralReference,
    Verifies,
    EvidencedBy,
    RecordContradicts,
    RecordDerivedFrom,
    RecordInvalidates,
    RecordRelatedTo,
    RecordSupports,
    RecordSupersedes,
    RecordValidates,
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
    Record,
    Knowledge,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum WhyRelationEndpoint {
    Entity {
        entity_kind: WhyEntityKind,
        entity_id: EntityId,
    },
    Evidence {
        evidence_id: EvidenceId,
    },
}

impl WhyRelationEndpoint {
    pub fn entity(entity_id: EntityId, entity_kind: WhyEntityKind) -> Self {
        Self::Entity {
            entity_kind,
            entity_id,
        }
    }

    pub fn evidence(evidence_id: EvidenceId) -> Self {
        Self::Evidence { evidence_id }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum WhyDeferredRelationFamily {
    Evolution,
    Epistemic,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WhyRelationEdge {
    pub relation_kind: WhyRelationKind,
    pub direction: WhyRelationDirection,
    pub relation_id: RelationId,
    pub relation_version_id: RelationVersionId,
    pub relation_label: Option<String>,
    pub source: WhyRelationEndpoint,
    pub target: WhyRelationEndpoint,
    pub state_digest: Digest,
}

pub(crate) fn explain_why(
    connection: &StoreConnection,
    options: &WhyQueryOptions,
) -> Result<WhyQueryResult> {
    let resolved = resolve_target(connection, options.target())?;
    let subject = resolve_subject(connection, &resolved, options.subject())?;

    let mut relation_edges = Vec::new();
    for relation in primary_containment_relations_at(connection, resolved.target.commit_id)? {
        let source =
            WhyRelationEndpoint::entity(relation.parent_entity_id, relation.parent_kind.into());
        let target =
            WhyRelationEndpoint::entity(relation.child_entity_id, relation.child_kind.into());
        if endpoint_matches_subject(options.subject(), source)
            || endpoint_matches_subject(options.subject(), target)
        {
            relation_edges.push(WhyRelationEdge {
                relation_kind: WhyRelationKind::PrimaryContainment,
                direction: relation_direction(options.subject(), source, target),
                relation_id: relation.relation_id,
                relation_version_id: relation.relation_version_id,
                relation_label: None,
                source,
                target,
                state_digest: relation.state_digest,
            });
        }
    }
    for relation in structural_references_at(connection, resolved.target.commit_id)? {
        let source =
            WhyRelationEndpoint::entity(relation.referrer_entity_id, relation.referrer_kind.into());
        let target =
            WhyRelationEndpoint::entity(relation.target_entity_id, relation.target_kind.into());
        if endpoint_matches_subject(options.subject(), source)
            || endpoint_matches_subject(options.subject(), target)
        {
            relation_edges.push(WhyRelationEdge {
                relation_kind: WhyRelationKind::StructuralReference,
                direction: relation_direction(options.subject(), source, target),
                relation_id: relation.relation_id,
                relation_version_id: relation.relation_version_id,
                relation_label: None,
                source,
                target,
                state_digest: relation.state_digest,
            });
        }
    }
    for relation in verification_relations_at(connection, resolved.target.commit_id)? {
        let target_entity_id = relation.target.entity_id();
        let source = WhyRelationEndpoint::entity(
            relation.source_verification_entity_id,
            WhyEntityKind::Verification,
        );
        let target = WhyRelationEndpoint::entity(target_entity_id, relation.target.into());
        if endpoint_matches_subject(options.subject(), source)
            || endpoint_matches_subject(options.subject(), target)
        {
            relation_edges.push(WhyRelationEdge {
                relation_kind: WhyRelationKind::Verifies,
                direction: relation_direction(options.subject(), source, target),
                relation_id: relation.relation_id,
                relation_version_id: relation.relation_version_id,
                relation_label: None,
                source,
                target,
                state_digest: relation.state_digest,
            });
        }
    }
    for relation in verification_evidence_relations_at(connection, resolved.target.commit_id)? {
        let source = WhyRelationEndpoint::entity(
            relation.source_verification_entity_id,
            WhyEntityKind::Verification,
        );
        let target = WhyRelationEndpoint::evidence(relation.evidence_id);
        if endpoint_matches_subject(options.subject(), source)
            || endpoint_matches_subject(options.subject(), target)
        {
            relation_edges.push(WhyRelationEdge {
                relation_kind: WhyRelationKind::EvidencedBy,
                direction: relation_direction(options.subject(), source, target),
                relation_id: relation.relation_id,
                relation_version_id: relation.relation_version_id,
                relation_label: None,
                source,
                target,
                state_digest: relation.state_digest,
            });
        }
    }
    for relation in record_relations_at(
        connection,
        &RecordRelationListOptions::new(resolved.target.commit_id),
    )?
    .relations
    {
        let source =
            WhyRelationEndpoint::entity(relation.source_record_entity_id, WhyEntityKind::Record);
        let target =
            WhyRelationEndpoint::entity(relation.target_record_entity_id, WhyEntityKind::Record);
        if endpoint_matches_subject(options.subject(), source)
            || endpoint_matches_subject(options.subject(), target)
        {
            relation_edges.push(WhyRelationEdge {
                relation_kind: record_relation_kind(relation.relation_type),
                direction: relation_direction(options.subject(), source, target),
                relation_id: relation.relation_id,
                relation_version_id: relation.relation_version_id,
                relation_label: relation.relation_label.clone(),
                source,
                target,
                state_digest: relation.state_digest,
            });
        }
    }
    relation_edges.sort_by(|left, right| {
        left.relation_kind
            .cmp(&right.relation_kind)
            .then_with(|| left.direction.cmp(&right.direction))
            .then_with(|| left.source.cmp(&right.source))
            .then_with(|| left.target.cmp(&right.target))
            .then_with(|| left.relation_label.cmp(&right.relation_label))
            .then_with(|| left.relation_id.cmp(&right.relation_id))
    });

    Ok(WhyQueryResult {
        target: resolved.target,
        subject,
        relation_edges,
        deferred_relation_families: vec![
            WhyDeferredRelationFamily::Evolution,
            WhyDeferredRelationFamily::Epistemic,
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

fn resolve_subject(
    connection: &StoreConnection,
    resolved: &ResolvedWhyTargetWithState,
    subject: WhyQuerySubject,
) -> Result<ResolvedWhyQuerySubject> {
    match subject {
        WhyQuerySubject::Entity(entity_id) => {
            let entity_version_id = resolved
                .state
                .entities()
                .iter()
                .find_map(|(current_entity_id, entity_version_id)| {
                    (*current_entity_id == entity_id).then_some(*entity_version_id)
                })
                .ok_or_else(|| {
                    WorkVcsError::QueryInvalid(format!(
                        "entity {entity_id} is not current in WorkState commit {}",
                        resolved.target.commit_id
                    ))
                })?;
            let entity_kind =
                load_subject_entity_kind(connection, resolved.target.workspace_id, entity_id)?;
            Ok(ResolvedWhyQuerySubject::Entity {
                entity_id,
                entity_version_id,
                entity_kind,
            })
        }
        WhyQuerySubject::Evidence(evidence_id) => {
            evidence(connection, evidence_id)?;
            Ok(ResolvedWhyQuerySubject::Evidence { evidence_id })
        }
    }
}

fn relation_direction(
    subject: WhyQuerySubject,
    source: WhyRelationEndpoint,
    target: WhyRelationEndpoint,
) -> WhyRelationDirection {
    debug_assert!(
        endpoint_matches_subject(subject, source) || endpoint_matches_subject(subject, target)
    );
    if endpoint_matches_subject(subject, source) {
        WhyRelationDirection::Outgoing
    } else {
        WhyRelationDirection::Incoming
    }
}

fn endpoint_matches_subject(subject: WhyQuerySubject, endpoint: WhyRelationEndpoint) -> bool {
    match (subject, endpoint) {
        (
            WhyQuerySubject::Entity(subject_entity_id),
            WhyRelationEndpoint::Entity { entity_id, .. },
        ) => subject_entity_id == entity_id,
        (
            WhyQuerySubject::Evidence(subject_evidence_id),
            WhyRelationEndpoint::Evidence { evidence_id },
        ) => subject_evidence_id == evidence_id,
        _ => false,
    }
}

fn load_subject_entity_kind(
    connection: &StoreConnection,
    workspace_id: WorkspaceId,
    entity_id: EntityId,
) -> Result<WhyEntityKind> {
    let row = connection
        .inner()
        .query_row(
            "SELECT object_identity.object_kind,
                    entity.workspace_id,
                    entity.entity_kind
             FROM entity
             JOIN object_identity
               ON object_identity.object_id = entity.object_id
             WHERE entity.object_id = ?1",
            params![&entity_id.raw_bytes()[..]],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, Vec<u8>>(1)?,
                    row.get::<_, String>(2)?,
                ))
            },
        )
        .optional()
        .map_err(crate::error::storage_error)?;

    let Some((object_kind, entity_workspace_id, entity_kind)) = row else {
        return Err(WorkVcsError::QueryInvalid(format!(
            "entity {entity_id} is present in WorkState but missing from entity table"
        )));
    };
    if object_kind != ENTITY_OBJECT_KIND {
        return Err(WorkVcsError::QueryInvalid(format!(
            "entity {entity_id} has object kind {object_kind:?}"
        )));
    }
    let entity_workspace_id = decode_workspace_id("entity.workspace_id", entity_workspace_id)?;
    if entity_workspace_id != workspace_id {
        return Err(WorkVcsError::QueryInvalid(format!(
            "entity {entity_id} belongs to workspace {entity_workspace_id}, not {workspace_id}"
        )));
    }
    parse_why_entity_kind(&entity_kind)
}

fn parse_why_entity_kind(entity_kind: &str) -> Result<WhyEntityKind> {
    match entity_kind {
        GOAL_ENTITY_KIND => Ok(WhyEntityKind::Goal),
        PLAN_ENTITY_KIND => Ok(WhyEntityKind::Plan),
        TASK_ENTITY_KIND => Ok(WhyEntityKind::Task),
        ACCEPTANCE_CRITERION_ENTITY_KIND => Ok(WhyEntityKind::AcceptanceCriterion),
        VERIFICATION_REQUIREMENT_ENTITY_KIND => Ok(WhyEntityKind::VerificationRequirement),
        VERIFICATION_ENTITY_KIND => Ok(WhyEntityKind::Verification),
        RECORD_ENTITY_KIND => Ok(WhyEntityKind::Record),
        KNOWLEDGE_ENTITY_KIND => Ok(WhyEntityKind::Knowledge),
        other => Err(WorkVcsError::QueryInvalid(format!(
            "entity kind {other:?} is not supported by why"
        ))),
    }
}

fn record_relation_kind(relation_type: RecordRelationType) -> WhyRelationKind {
    match relation_type {
        RecordRelationType::Contradicts => WhyRelationKind::RecordContradicts,
        RecordRelationType::DerivedFrom => WhyRelationKind::RecordDerivedFrom,
        RecordRelationType::Invalidates => WhyRelationKind::RecordInvalidates,
        RecordRelationType::RelatedTo => WhyRelationKind::RecordRelatedTo,
        RecordRelationType::Supports => WhyRelationKind::RecordSupports,
        RecordRelationType::Supersedes => WhyRelationKind::RecordSupersedes,
        RecordRelationType::Validates => WhyRelationKind::RecordValidates,
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

fn decode_workspace_id(column: &str, bytes: Vec<u8>) -> Result<WorkspaceId> {
    let bytes = decode_16(column, bytes)?;
    WorkspaceId::from_bytes(bytes).map_err(|error| WorkVcsError::QueryInvalid(error.to_string()))
}

fn decode_16(column: &str, bytes: Vec<u8>) -> Result<[u8; 16]> {
    bytes.try_into().map_err(|bytes: Vec<u8>| {
        WorkVcsError::QueryInvalid(format!("{column} must be 16 bytes, found {}", bytes.len()))
    })
}
