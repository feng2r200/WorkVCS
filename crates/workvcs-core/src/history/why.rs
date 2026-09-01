use super::goal::GOAL_ENTITY_KIND;
use super::knowledge::KNOWLEDGE_ENTITY_KIND;
use super::plan::PLAN_ENTITY_KIND;
use super::record::{RECORD_ENTITY_KIND, RecordKind, record_at};
use super::task::{
    ACCEPTANCE_CRITERION_ENTITY_KIND, TASK_ENTITY_KIND, VERIFICATION_ENTITY_KIND,
    VERIFICATION_REQUIREMENT_ENTITY_KIND,
};
use super::{
    HistoryQueryOptions, KnowledgeRelationListOptions, PrimaryContainmentEndpointKind,
    RecordKnowledgeRelationListOptions, RecordRelationListOptions, RecordRelationType,
    StructuralReferenceEndpointKind, VerificationTarget, branch_head, evidence, knowledge_exposure,
    knowledge_relations_at, primary_containment_relations_at, query_history,
    record_knowledge_relations_at, record_relations_at, state_at, structural_references_at,
    verification_evidence_relations_at, verification_relations_at,
};
use crate::canonical::CanonicalValue;
use crate::error::{Result, WorkVcsError};
use crate::identity::{
    BranchId, ChangeSetId, CommitId, Digest, EntityId, EntityVersionId, EvidenceId, ExposureId,
    RelationId, RelationVersionId, WorkspaceId,
};
use crate::store::StoreConnection;
use rusqlite::{OptionalExtension, params};

const ENTITY_OBJECT_KIND: &str = "entity";
const DERIVED_FROM_RELATION_TYPE: &str = "derived_from";
const KNOWLEDGE_EXPOSURE_OBJECT_KIND: &str = "knowledge_exposure";

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
    KnowledgeExposure(ExposureId),
}

impl WhyQuerySubject {
    pub fn entity(entity_id: EntityId) -> Self {
        Self::Entity(entity_id)
    }

    pub fn evidence(evidence_id: EvidenceId) -> Self {
        Self::Evidence(evidence_id)
    }

    pub fn knowledge_exposure(exposure_id: ExposureId) -> Self {
        Self::KnowledgeExposure(exposure_id)
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

    pub fn for_knowledge_exposure(target: WhyQueryTarget, subject_exposure_id: ExposureId) -> Self {
        Self {
            target,
            subject: WhyQuerySubject::KnowledgeExposure(subject_exposure_id),
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
            WhyQuerySubject::Evidence(_) | WhyQuerySubject::KnowledgeExposure(_) => None,
        }
    }

    pub fn subject_evidence_id(&self) -> Option<EvidenceId> {
        match self.subject {
            WhyQuerySubject::Entity(_) => None,
            WhyQuerySubject::Evidence(evidence_id) => Some(evidence_id),
            WhyQuerySubject::KnowledgeExposure(_) => None,
        }
    }

    pub fn subject_exposure_id(&self) -> Option<ExposureId> {
        match self.subject {
            WhyQuerySubject::Entity(_) | WhyQuerySubject::Evidence(_) => None,
            WhyQuerySubject::KnowledgeExposure(exposure_id) => Some(exposure_id),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WhyQueryResult {
    pub target: ResolvedWhyQueryTarget,
    pub subject: ResolvedWhyQuerySubject,
    pub relation_edges: Vec<WhyRelationEdge>,
    pub scope_links: Vec<WhyScopeLink>,
    pub causal_anchor_changesets: Vec<WhyCausalAnchorChangeSet>,
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
    KnowledgeExposure {
        exposure_id: ExposureId,
    },
}

impl ResolvedWhyQuerySubject {
    pub fn entity_id(self) -> Option<EntityId> {
        match self {
            Self::Entity { entity_id, .. } => Some(entity_id),
            Self::Evidence { .. } | Self::KnowledgeExposure { .. } => None,
        }
    }

    pub fn entity_version_id(self) -> Option<EntityVersionId> {
        match self {
            Self::Entity {
                entity_version_id, ..
            } => Some(entity_version_id),
            Self::Evidence { .. } | Self::KnowledgeExposure { .. } => None,
        }
    }

    pub fn entity_kind(self) -> Option<WhyEntityKind> {
        match self {
            Self::Entity { entity_kind, .. } => Some(entity_kind),
            Self::Evidence { .. } | Self::KnowledgeExposure { .. } => None,
        }
    }

    pub fn evidence_id(self) -> Option<EvidenceId> {
        match self {
            Self::Entity { .. } => None,
            Self::Evidence { evidence_id } => Some(evidence_id),
            Self::KnowledgeExposure { .. } => None,
        }
    }

    pub fn exposure_id(self) -> Option<ExposureId> {
        match self {
            Self::Entity { .. } | Self::Evidence { .. } => None,
            Self::KnowledgeExposure { exposure_id } => Some(exposure_id),
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
    KnowledgeExposureDerivedFrom,
    KnowledgeSupersedes,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum WhyScopeLinkKind {
    HandoffFocus,
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
    KnowledgeExposure {
        exposure_id: ExposureId,
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

    pub fn knowledge_exposure(exposure_id: ExposureId) -> Self {
        Self::KnowledgeExposure { exposure_id }
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WhyScopeLink {
    pub link_kind: WhyScopeLinkKind,
    pub direction: WhyRelationDirection,
    pub source: WhyRelationEndpoint,
    pub target: WhyRelationEndpoint,
    pub source_entity_version_id: EntityVersionId,
    pub target_entity_version_id: EntityVersionId,
    pub source_state_digest: Digest,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WhyCausalAnchorChangeSet {
    pub commit_id: CommitId,
    pub changeset_id: ChangeSetId,
    pub operation_type: String,
    pub operation_schema_version: i64,
    pub committed_at_us: i64,
    pub changeset_created_at_us: i64,
    pub anchor_object_id: EntityId,
    pub anchor_object_kind: String,
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
    for relation in record_knowledge_relations_at(
        connection,
        &RecordKnowledgeRelationListOptions::new(resolved.target.commit_id),
    )?
    .relations
    {
        let source =
            WhyRelationEndpoint::entity(relation.source_record_entity_id, WhyEntityKind::Record);
        let target = WhyRelationEndpoint::entity(
            relation.target_knowledge_entity_id,
            WhyEntityKind::Knowledge,
        );
        if endpoint_matches_subject(options.subject(), source)
            || endpoint_matches_subject(options.subject(), target)
        {
            relation_edges.push(WhyRelationEdge {
                relation_kind: record_relation_kind(relation.relation_type),
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
    for relation in knowledge_relations_at(
        connection,
        &KnowledgeRelationListOptions::new(resolved.target.commit_id),
    )?
    .relations
    {
        let source = WhyRelationEndpoint::entity(
            relation.replacement_knowledge_entity_id,
            WhyEntityKind::Knowledge,
        );
        let target = WhyRelationEndpoint::entity(
            relation.prior_knowledge_entity_id,
            WhyEntityKind::Knowledge,
        );
        if endpoint_matches_subject(options.subject(), source)
            || endpoint_matches_subject(options.subject(), target)
        {
            relation_edges.push(WhyRelationEdge {
                relation_kind: WhyRelationKind::KnowledgeSupersedes,
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
    for relation in knowledge_exposure_derived_from_relations_at(connection, &resolved)? {
        let source = WhyRelationEndpoint::entity(
            relation.source_knowledge_entity_id,
            WhyEntityKind::Knowledge,
        );
        let target = WhyRelationEndpoint::knowledge_exposure(relation.exposure_id);
        if endpoint_matches_subject(options.subject(), source)
            || endpoint_matches_subject(options.subject(), target)
        {
            relation_edges.push(WhyRelationEdge {
                relation_kind: WhyRelationKind::KnowledgeExposureDerivedFrom,
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
    let mut scope_links = handoff_focus_scope_links(connection, &resolved, options.subject())?;
    let causal_anchor_changesets =
        why_causal_anchor_changesets(connection, &resolved, options.subject())?;
    relation_edges.sort_by(|left, right| {
        left.relation_kind
            .cmp(&right.relation_kind)
            .then_with(|| left.direction.cmp(&right.direction))
            .then_with(|| left.source.cmp(&right.source))
            .then_with(|| left.target.cmp(&right.target))
            .then_with(|| left.relation_label.cmp(&right.relation_label))
            .then_with(|| left.relation_id.cmp(&right.relation_id))
    });
    scope_links.sort_by(|left, right| {
        left.link_kind
            .cmp(&right.link_kind)
            .then_with(|| left.direction.cmp(&right.direction))
            .then_with(|| left.source.cmp(&right.source))
            .then_with(|| left.target.cmp(&right.target))
            .then_with(|| {
                left.source_entity_version_id
                    .cmp(&right.source_entity_version_id)
            })
    });

    let deferred_relation_families = why_deferred_relation_families(&causal_anchor_changesets);

    Ok(WhyQueryResult {
        target: resolved.target,
        subject,
        relation_edges,
        scope_links,
        causal_anchor_changesets,
        deferred_relation_families,
    })
}

fn why_deferred_relation_families(
    causal_anchor_changesets: &[WhyCausalAnchorChangeSet],
) -> Vec<WhyDeferredRelationFamily> {
    let mut families = Vec::new();
    if !causal_anchor_changesets.is_empty() {
        families.push(WhyDeferredRelationFamily::Evolution);
    }
    families
}

fn why_causal_anchor_changesets(
    connection: &StoreConnection,
    resolved: &ResolvedWhyTargetWithState,
    subject: WhyQuerySubject,
) -> Result<Vec<WhyCausalAnchorChangeSet>> {
    let WhyQuerySubject::Entity(entity_id) = subject else {
        return Ok(Vec::new());
    };
    let mut anchors = Vec::new();
    let history = query_history(
        connection,
        &HistoryQueryOptions::from_commit(resolved.target.commit_id),
    )?;
    for entry in history.entries {
        if entry.workspace_id != resolved.target.workspace_id {
            continue;
        }
        if changeset_has_entity_causal_anchor(connection, entry.changeset_id, entity_id)? {
            anchors.push(WhyCausalAnchorChangeSet {
                commit_id: entry.commit_id,
                changeset_id: entry.changeset_id,
                operation_type: entry.operation_type,
                operation_schema_version: entry.operation_schema_version,
                committed_at_us: entry.committed_at_us,
                changeset_created_at_us: entry.changeset_created_at_us,
                anchor_object_id: entity_id,
                anchor_object_kind: ENTITY_OBJECT_KIND.to_owned(),
            });
        }
    }
    Ok(anchors)
}

fn changeset_has_entity_causal_anchor(
    connection: &StoreConnection,
    changeset_id: ChangeSetId,
    entity_id: EntityId,
) -> Result<bool> {
    let row = connection
        .inner()
        .query_row(
            "SELECT 1
             FROM changeset_causal_anchor
             JOIN object_identity
               ON object_identity.object_id = changeset_causal_anchor.anchor_object_id
             WHERE changeset_causal_anchor.changeset_id = ?1
               AND changeset_causal_anchor.anchor_object_id = ?2
               AND object_identity.object_kind = ?3
             LIMIT 1",
            params![
                &changeset_id.raw_bytes()[..],
                &entity_id.raw_bytes()[..],
                ENTITY_OBJECT_KIND
            ],
            |row| row.get::<_, i64>(0),
        )
        .optional()
        .map_err(crate::error::storage_error)?;
    Ok(row.is_some())
}

fn handoff_focus_scope_links(
    connection: &StoreConnection,
    resolved: &ResolvedWhyTargetWithState,
    subject: WhyQuerySubject,
) -> Result<Vec<WhyScopeLink>> {
    if !matches!(subject, WhyQuerySubject::Entity(_)) {
        return Ok(Vec::new());
    }
    let mut links = Vec::new();
    for (entity_id, entity_version_id) in resolved.state.entities() {
        if !is_record_entity(connection, resolved.target.workspace_id, *entity_id)? {
            continue;
        }
        let record = record_at(connection, resolved.target.commit_id, *entity_id)?;
        if record.state.kind != RecordKind::Handoff {
            continue;
        }
        let Some(focus_entity_id) = focused_handoff_scope_entity(&record.state.scope) else {
            continue;
        };
        let Some(focus_entity_version_id) =
            current_entity_version_id(&resolved.state, focus_entity_id)
        else {
            continue;
        };
        let focus_kind =
            load_subject_entity_kind(connection, resolved.target.workspace_id, focus_entity_id)?;
        let source = WhyRelationEndpoint::entity(record.record_entity_id, WhyEntityKind::Record);
        let target = WhyRelationEndpoint::entity(focus_entity_id, focus_kind);
        if endpoint_matches_subject(subject, source) || endpoint_matches_subject(subject, target) {
            links.push(WhyScopeLink {
                link_kind: WhyScopeLinkKind::HandoffFocus,
                direction: relation_direction(subject, source, target),
                source,
                target,
                source_entity_version_id: *entity_version_id,
                target_entity_version_id: focus_entity_version_id,
                source_state_digest: record.state_digest,
            });
        }
    }
    Ok(links)
}

fn is_record_entity(
    connection: &StoreConnection,
    workspace_id: WorkspaceId,
    entity_id: EntityId,
) -> Result<bool> {
    let row = connection
        .inner()
        .query_row(
            "SELECT entity_kind
             FROM entity
             WHERE object_id = ?1
               AND workspace_id = ?2",
            params![&entity_id.raw_bytes()[..], &workspace_id.raw_bytes()[..]],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(crate::error::storage_error)?;
    Ok(row.is_some_and(|entity_kind| entity_kind == RECORD_ENTITY_KIND))
}

fn current_entity_version_id(
    state: &crate::canonical::WorkState,
    entity_id: EntityId,
) -> Option<EntityVersionId> {
    state
        .entities()
        .iter()
        .find_map(|(current_entity_id, entity_version_id)| {
            (*current_entity_id == entity_id).then_some(*entity_version_id)
        })
}

fn focused_handoff_scope_entity(scope: &CanonicalValue) -> Option<EntityId> {
    let CanonicalValue::Object(entries) = scope else {
        return None;
    };
    let schema_version = handoff_scope_field(entries, "handoff_scope_schema_version")?;
    let CanonicalValue::Integer(schema_version) = schema_version else {
        return None;
    };
    if schema_version.get() != 1 {
        return None;
    }
    let value = handoff_scope_field(entries, "focus_entity_id")?;
    match value {
        CanonicalValue::String(value) => EntityId::parse_canonical(value).ok(),
        _ => None,
    }
}

fn handoff_scope_field<'a>(
    entries: &'a [(String, CanonicalValue)],
    key: &str,
) -> Option<&'a CanonicalValue> {
    entries
        .iter()
        .find_map(|(entry_key, value)| (entry_key == key).then_some(value))
}

struct KnowledgeExposureDerivedFromRelationRow {
    relation_id: RelationId,
    relation_version_id: RelationVersionId,
    source_knowledge_entity_id: EntityId,
    exposure_id: ExposureId,
    state_digest: Digest,
}

fn knowledge_exposure_derived_from_relations_at(
    connection: &StoreConnection,
    resolved: &ResolvedWhyTargetWithState,
) -> Result<Vec<KnowledgeExposureDerivedFromRelationRow>> {
    let mut relations = Vec::new();
    for (relation_id, relation_version_id) in resolved.state.relations() {
        if let Some(relation) = load_knowledge_exposure_derived_from_relation(
            connection,
            resolved.target.workspace_id,
            *relation_id,
            *relation_version_id,
        )? {
            relations.push(relation);
        }
    }
    Ok(relations)
}

fn load_knowledge_exposure_derived_from_relation(
    connection: &StoreConnection,
    workspace_id: WorkspaceId,
    relation_id: RelationId,
    relation_version_id: RelationVersionId,
) -> Result<Option<KnowledgeExposureDerivedFromRelationRow>> {
    let row = connection
        .inner()
        .query_row(
            "SELECT relation.source_object_id,
                    relation.target_object_id,
                    relation_version.state_digest
             FROM relation
             JOIN relation_version
               ON relation_version.relation_id = relation.object_id
             JOIN object_identity AS target_identity
               ON target_identity.object_id = relation.target_object_id
             JOIN knowledge_exposure
               ON knowledge_exposure.exposure_id = relation.target_object_id
             JOIN entity AS source_entity
               ON source_entity.object_id = relation.source_object_id
             WHERE relation.object_id = ?1
               AND relation_version.relation_version_id = ?2
               AND relation.workspace_id = ?3
               AND relation.relation_type = ?4
               AND relation.relation_discriminator = ''
               AND target_identity.object_kind = ?5
               AND source_entity.workspace_id = ?3
               AND source_entity.entity_kind = ?6",
            params![
                &relation_id.raw_bytes()[..],
                &relation_version_id.raw_bytes()[..],
                &workspace_id.raw_bytes()[..],
                DERIVED_FROM_RELATION_TYPE,
                KNOWLEDGE_EXPOSURE_OBJECT_KIND,
                KNOWLEDGE_ENTITY_KIND,
            ],
            |row| {
                Ok((
                    row.get::<_, Vec<u8>>(0)?,
                    row.get::<_, Vec<u8>>(1)?,
                    row.get::<_, Vec<u8>>(2)?,
                ))
            },
        )
        .optional()
        .map_err(crate::error::storage_error)?;
    let Some((source_knowledge_entity_id, exposure_id, state_digest)) = row else {
        return Ok(None);
    };
    Ok(Some(KnowledgeExposureDerivedFromRelationRow {
        relation_id,
        relation_version_id,
        source_knowledge_entity_id: decode_entity_id(
            "relation.source_object_id",
            source_knowledge_entity_id,
        )?,
        exposure_id: decode_exposure_id("relation.target_object_id", exposure_id)?,
        state_digest: decode_digest("relation_version.state_digest", state_digest)?,
    }))
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
        WhyQuerySubject::KnowledgeExposure(exposure_id) => {
            knowledge_exposure(connection, exposure_id)?;
            Ok(ResolvedWhyQuerySubject::KnowledgeExposure { exposure_id })
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
        (
            WhyQuerySubject::KnowledgeExposure(subject_exposure_id),
            WhyRelationEndpoint::KnowledgeExposure { exposure_id },
        ) => subject_exposure_id == exposure_id,
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

fn decode_entity_id(column: &str, bytes: Vec<u8>) -> Result<EntityId> {
    let bytes = decode_16(column, bytes)?;
    EntityId::from_bytes(bytes).map_err(|error| WorkVcsError::QueryInvalid(error.to_string()))
}

fn decode_exposure_id(column: &str, bytes: Vec<u8>) -> Result<ExposureId> {
    let bytes = decode_16(column, bytes)?;
    ExposureId::from_bytes(bytes).map_err(|error| WorkVcsError::QueryInvalid(error.to_string()))
}

fn decode_16(column: &str, bytes: Vec<u8>) -> Result<[u8; 16]> {
    bytes.try_into().map_err(|bytes: Vec<u8>| {
        WorkVcsError::QueryInvalid(format!("{column} must be 16 bytes, found {}", bytes.len()))
    })
}

fn decode_digest(column: &str, bytes: Vec<u8>) -> Result<Digest> {
    let bytes = bytes.try_into().map_err(|bytes: Vec<u8>| {
        WorkVcsError::QueryInvalid(format!("{column} must be 32 bytes, found {}", bytes.len()))
    })?;
    Ok(Digest::from_bytes(bytes))
}
