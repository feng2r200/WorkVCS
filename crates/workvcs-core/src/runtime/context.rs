use super::runnable::{
    self, RunnableTaskCandidate, RunnableTaskClaimCoordination, RunnableTasksOptions,
    RunnableTasksProjection,
};
use super::session::{self, SessionLifecycleState, SessionSnapshot};
use crate::canonical::{CanonicalValue, canonical_bytes, parse_canonical_json};
use crate::error::{Result, WorkVcsError, storage_error};
use crate::history::{
    self, AcceptanceCriterionEffectiveStatus, AcceptanceCriterionSnapshot, BranchHead,
    ChangeSetSnapshot, GoalSnapshot, HistoryQueryOptions, KnowledgeListOptions,
    KnowledgeListResult, KnowledgeRelationListOptions, KnowledgeRelationListResult,
    KnowledgeStatus, PlanSnapshot, PrimaryContainmentEndpointKind, PrimaryContainmentSnapshot,
    RecordKind, RecordKnowledgeRelationListOptions, RecordKnowledgeRelationListResult,
    RecordListOptions, RecordListResult, RecordRelationListOptions, RecordRelationListResult,
    RecordRelationSnapshot, RecordSnapshot, RecordStatus, StructuralReferenceEndpointKind,
    StructuralReferenceSnapshot, TaskSnapshot, VerificationRequirementSnapshot,
    VerificationSnapshot, VerificationTarget, WhyRelationEdge, WhyRelationKind,
};
use crate::identity::{
    BranchId, ChangeSetId, CommitId, ContextPacketId, Digest, EntityId, RelationId, SessionId,
    WorkspaceId,
};
use crate::store::{StoreConnection, current_epoch_micros};
use rusqlite::{OptionalExtension, TransactionBehavior, params};
use std::collections::{BTreeMap, BTreeSet};
use std::num::NonZeroUsize;

const CONTEXT_PACKET_DIGEST_DOMAIN: &str = "context-packet-snapshot-v1";
const CONTEXT_PACKET_FORMAT: &str = "workvcs-context-packet-v1";
const CONTEXT_PACKET_FORMAT_VERSION: i64 = 1;
const CONTEXT_TRANSITION_RATIONALE_HISTORY_LIMIT: usize = 8;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContextOverviewOptions {
    session_id: SessionId,
}

impl ContextOverviewOptions {
    pub fn new(session_id: SessionId) -> Self {
        Self { session_id }
    }

    pub fn session_id(&self) -> SessionId {
        self.session_id
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContextProfile {
    Brief,
    Normal,
    Full,
}

impl ContextProfile {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Brief => "brief",
            Self::Normal => "normal",
            Self::Full => "full",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContextPacketOptions {
    session_id: SessionId,
    profile: ContextProfile,
    budget_items: Option<NonZeroUsize>,
    scope: Option<CanonicalValue>,
}

impl ContextPacketOptions {
    pub fn new(session_id: SessionId) -> Self {
        Self {
            session_id,
            profile: ContextProfile::Normal,
            budget_items: None,
            scope: None,
        }
    }

    pub fn with_profile(mut self, profile: ContextProfile) -> Self {
        self.profile = profile;
        self
    }

    pub fn with_budget_items(mut self, budget_items: usize) -> Result<Self> {
        let budget_items = NonZeroUsize::new(budget_items).ok_or_else(|| {
            WorkVcsError::QueryInvalid("context budget items must be greater than zero".to_owned())
        })?;
        self.budget_items = Some(budget_items);
        Ok(self)
    }

    pub fn with_scope(mut self, scope: CanonicalValue) -> Result<Self> {
        if !matches!(scope, CanonicalValue::Object(_)) {
            return Err(WorkVcsError::QueryInvalid(
                "context scope must be an object".to_owned(),
            ));
        }
        self.scope = Some(scope);
        Ok(self)
    }

    pub fn session_id(&self) -> SessionId {
        self.session_id
    }

    pub fn profile(&self) -> ContextProfile {
        self.profile
    }

    pub fn budget_items(&self) -> Option<NonZeroUsize> {
        self.budget_items
    }

    pub fn scope(&self) -> Option<&CanonicalValue> {
        self.scope.as_ref()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ContextPriority {
    P0,
    P1,
    P2,
    P3,
    P4,
    P5,
    P6,
    P7,
    P8,
    P9,
}

impl ContextPriority {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::P0 => "P0",
            Self::P1 => "P1",
            Self::P2 => "P2",
            Self::P3 => "P3",
            Self::P4 => "P4",
            Self::P5 => "P5",
            Self::P6 => "P6",
            Self::P7 => "P7",
            Self::P8 => "P8",
            Self::P9 => "P9",
        }
    }

    fn key(self) -> &'static str {
        match self {
            Self::P0 => "p0",
            Self::P1 => "p1",
            Self::P2 => "p2",
            Self::P3 => "p3",
            Self::P4 => "p4",
            Self::P5 => "p5",
            Self::P6 => "p6",
            Self::P7 => "p7",
            Self::P8 => "p8",
            Self::P9 => "p9",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ContextItemCategory {
    SessionAnchor,
    BranchOverview,
    CurrentTask,
    GoalPlanPath,
    AcceptanceCriterion,
    VerificationRequirement,
    TaskReadiness,
    BlockedDependency,
    DirectCausalChain,
    TransitionRationale,
    ActiveDecision,
    ActiveAssumption,
    FailedAttempt,
    Attempt,
    Finding,
    ScopedKnowledge,
    SessionContinuity,
    RelevantHandoff,
    OlderProvenance,
}

impl ContextItemCategory {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::SessionAnchor => "session_anchor",
            Self::BranchOverview => "branch_overview",
            Self::CurrentTask => "current_task",
            Self::GoalPlanPath => "goal_plan_path",
            Self::AcceptanceCriterion => "acceptance_criterion",
            Self::VerificationRequirement => "verification_requirement",
            Self::TaskReadiness => "task_readiness",
            Self::BlockedDependency => "blocked_dependency",
            Self::DirectCausalChain => "direct_causal_chain",
            Self::TransitionRationale => "transition_rationale",
            Self::ActiveDecision => "active_decision",
            Self::ActiveAssumption => "active_assumption",
            Self::FailedAttempt => "failed_attempt",
            Self::Attempt => "attempt",
            Self::Finding => "finding",
            Self::ScopedKnowledge => "scoped_knowledge",
            Self::SessionContinuity => "session_continuity",
            Self::RelevantHandoff => "relevant_handoff",
            Self::OlderProvenance => "older_provenance",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ContextItemSubject {
    Session {
        session_id: SessionId,
    },
    Branch {
        branch_id: BranchId,
        commit_id: CommitId,
    },
    Workspace {
        workspace_id: WorkspaceId,
    },
    Task {
        task_entity_id: EntityId,
    },
    GoalPlanPath {
        task_entity_id: EntityId,
    },
    AcceptanceCriterion {
        acceptance_criterion_entity_id: EntityId,
    },
    VerificationRequirement {
        verification_requirement_entity_id: EntityId,
    },
    TaskReadiness {
        task_entity_id: EntityId,
    },
    BlockedDependency {
        task_entity_id: EntityId,
        dependency_task_entity_id: EntityId,
    },
    Record {
        record_entity_id: EntityId,
    },
    Knowledge {
        knowledge_entity_id: EntityId,
    },
    Relation {
        relation_id: RelationId,
    },
    ChangeSet {
        changeset_id: ChangeSetId,
        commit_id: CommitId,
    },
}

impl ContextItemSubject {
    pub fn as_ref_string(&self) -> String {
        match self {
            Self::Session { session_id } => format!("session:{session_id}"),
            Self::Branch {
                branch_id,
                commit_id,
            } => format!("branch:{branch_id}@{commit_id}"),
            Self::Workspace { workspace_id } => format!("workspace:{workspace_id}"),
            Self::Task { task_entity_id } => format!("task:{task_entity_id}"),
            Self::GoalPlanPath { task_entity_id } => format!("goal_plan_path:{task_entity_id}"),
            Self::AcceptanceCriterion {
                acceptance_criterion_entity_id,
            } => format!("acceptance_criterion:{acceptance_criterion_entity_id}"),
            Self::VerificationRequirement {
                verification_requirement_entity_id,
            } => format!("verification_requirement:{verification_requirement_entity_id}"),
            Self::TaskReadiness { task_entity_id } => {
                format!("task_readiness:{task_entity_id}")
            }
            Self::BlockedDependency {
                task_entity_id,
                dependency_task_entity_id,
            } => format!("blocked_dependency:{task_entity_id}:{dependency_task_entity_id}"),
            Self::Record { record_entity_id } => format!("record:{record_entity_id}"),
            Self::Knowledge {
                knowledge_entity_id,
            } => format!("knowledge:{knowledge_entity_id}"),
            Self::Relation { relation_id } => format!("relation:{relation_id}"),
            Self::ChangeSet {
                changeset_id,
                commit_id,
            } => format!("changeset:{changeset_id}@{commit_id}"),
        }
    }

    fn key(&self) -> String {
        self.as_ref_string().replace([':', '@'], ".")
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContextItem {
    pub priority: ContextPriority,
    pub category: ContextItemCategory,
    pub subject: ContextItemSubject,
    pub item_key: String,
    pub summary: String,
}

impl ContextItem {
    fn new(
        priority: ContextPriority,
        category: ContextItemCategory,
        subject: ContextItemSubject,
        summary: impl Into<String>,
    ) -> Self {
        let item_key = format!("{}.{}.{}", priority.key(), category.as_str(), subject.key());
        Self {
            priority,
            category,
            subject,
            item_key,
            summary: summary.into(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContextOmissionBucket {
    pub priority: ContextPriority,
    pub omitted: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContextOmissionCategory {
    pub category: ContextItemCategory,
    pub omitted: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContextOmissionSummary {
    pub total: usize,
    pub by_priority: Vec<ContextOmissionBucket>,
    pub by_category: Vec<ContextOmissionCategory>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContextPacketEnvelope {
    pub session_id: SessionId,
    pub lifecycle_state: SessionLifecycleState,
    pub workspace_id: WorkspaceId,
    pub branch_id: BranchId,
    pub branch_name: String,
    pub head_commit_id: CommitId,
    pub state_digest: Digest,
    pub started_at_us: i64,
    pub last_activity_at_us: Option<i64>,
    pub focus_entity_id: Option<EntityId>,
}

impl ContextPacketEnvelope {
    fn from_overview(context: &ContextOverview) -> Self {
        Self {
            session_id: context.session.session_id,
            lifecycle_state: context.session.lifecycle_state,
            workspace_id: context.branch.workspace_id,
            branch_id: context.branch.branch_id,
            branch_name: context.branch.name.clone(),
            head_commit_id: context.branch.head_commit_id,
            state_digest: context.branch.state_digest,
            started_at_us: context.session.started_at_us,
            last_activity_at_us: context.session.last_activity_at_us,
            focus_entity_id: context
                .session
                .focus
                .as_ref()
                .map(|focus| focus.focus_entity_id),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContextPacket {
    pub envelope: ContextPacketEnvelope,
    pub profile: ContextProfile,
    pub budget_items: Option<NonZeroUsize>,
    pub scope: Option<CanonicalValue>,
    pub available_items: usize,
    pub items: Vec<ContextItem>,
    pub omission_summary: ContextOmissionSummary,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContextPacketSaveResult {
    pub snapshot: ContextPacketSnapshot,
    pub packet: ContextPacket,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContextPacketSnapshot {
    pub context_packet_id: ContextPacketId,
    pub session_id: SessionId,
    pub workspace_id: WorkspaceId,
    pub branch_id: BranchId,
    pub head_commit_id: CommitId,
    pub state_digest: Digest,
    pub profile: ContextProfile,
    pub budget_items: Option<NonZeroUsize>,
    pub scope: Option<CanonicalValue>,
    pub available_items: usize,
    pub item_count: usize,
    pub omitted_items: usize,
    pub packet_digest: Digest,
    pub packet_json: CanonicalValue,
    pub created_at_us: i64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ContextPacketListOptions {
    session_id: SessionId,
    limit: NonZeroUsize,
}

impl ContextPacketListOptions {
    pub fn for_session(session_id: SessionId) -> Self {
        Self {
            session_id,
            limit: NonZeroUsize::new(20).expect("default context packet list limit is non-zero"),
        }
    }

    pub fn with_limit(mut self, limit: usize) -> Result<Self> {
        let limit = NonZeroUsize::new(limit).ok_or_else(|| {
            WorkVcsError::QueryInvalid(
                "context packet list limit must be greater than zero".to_owned(),
            )
        })?;
        self.limit = limit;
        Ok(self)
    }

    pub fn session_id(self) -> SessionId {
        self.session_id
    }

    pub fn limit(self) -> NonZeroUsize {
        self.limit
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContextPacketListResult {
    pub session_id: SessionId,
    pub snapshots: Vec<ContextPacketSnapshot>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContextOverview {
    pub session: SessionSnapshot,
    pub branch: BranchHead,
    pub runnable_tasks: RunnableTasksProjection,
    pub tasks: Vec<TaskSnapshot>,
    pub plans: Vec<PlanSnapshot>,
    pub goals: Vec<GoalSnapshot>,
    pub primary_containment_relations: Vec<PrimaryContainmentSnapshot>,
    pub structural_references: Vec<StructuralReferenceSnapshot>,
    pub acceptance_criteria: Vec<ContextAcceptanceCriterionSnapshot>,
    pub verification_requirements: Vec<VerificationRequirementSnapshot>,
    pub knowledge: KnowledgeListResult,
    pub knowledge_relations: KnowledgeRelationListResult,
    pub knowledge_exposure_relations: Vec<WhyRelationEdge>,
    pub records: RecordListResult,
    pub record_relations: RecordRelationListResult,
    pub record_knowledge_relations: RecordKnowledgeRelationListResult,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContextAcceptanceCriterionSnapshot {
    pub snapshot: AcceptanceCriterionSnapshot,
    pub effective_status: AcceptanceCriterionEffectiveStatus,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ContextTransitionRationaleSnapshot {
    pub commit_id: CommitId,
    pub changeset_id: ChangeSetId,
    pub commit_kind: String,
    pub committed_at_us: i64,
    pub operation_type: String,
    pub operation_schema_version: i64,
    pub changeset_created_at_us: i64,
    pub rationale_json: String,
    pub rationale_digest: Digest,
    pub rationale_size_bytes: i64,
    pub origin_session_id: Option<SessionId>,
    pub change_operation_count: i64,
    pub causal_anchor_count: i64,
    pub event_count: i64,
}

pub(crate) fn context_overview(
    connection: &StoreConnection,
    options: &ContextOverviewOptions,
) -> Result<ContextOverview> {
    connection.verify_foreign_keys()?;
    let session = session::session_snapshot(connection, options.session_id())?;
    ensure_active_session(&session)?;
    let active_workspace_id = require_active_workspace_id(&session)?;
    let active_branch_id = session.active_branch_id.ok_or_else(|| {
        WorkVcsError::SessionInvalid(format!(
            "active session {} has no active branch",
            options.session_id()
        ))
    })?;
    let branch = history::branch_head(connection, active_branch_id)?;
    if branch.workspace_id != active_workspace_id {
        return Err(WorkVcsError::SessionInvalid(format!(
            "session {} active branch {} belongs to workspace {}, not active workspace {}",
            options.session_id(),
            active_branch_id,
            branch.workspace_id,
            active_workspace_id
        )));
    }

    let runnable_tasks =
        runnable::runnable_tasks(connection, &RunnableTasksOptions::new(options.session_id()))?;
    if runnable_tasks.workspace_id != active_workspace_id
        || runnable_tasks.branch_id != active_branch_id
        || runnable_tasks.head_commit_id != branch.head_commit_id
    {
        return Err(WorkVcsError::SessionInvalid(format!(
            "session {} context anchor changed while resolving overview",
            options.session_id()
        )));
    }
    let tasks = history::tasks_at(connection, branch.head_commit_id)?;
    if tasks.iter().any(|task| {
        task.workspace_id != active_workspace_id || task.commit_id != branch.head_commit_id
    }) {
        return Err(WorkVcsError::SessionInvalid(format!(
            "session {} task context anchor changed while resolving overview",
            options.session_id()
        )));
    }
    let plans = history::plans_at(connection, branch.head_commit_id)?;
    if plans.iter().any(|plan| {
        plan.workspace_id != active_workspace_id || plan.commit_id != branch.head_commit_id
    }) {
        return Err(WorkVcsError::SessionInvalid(format!(
            "session {} plan context anchor changed while resolving overview",
            options.session_id()
        )));
    }
    let goals = history::goals_at(connection, branch.head_commit_id)?;
    if goals.iter().any(|goal| {
        goal.workspace_id != active_workspace_id || goal.commit_id != branch.head_commit_id
    }) {
        return Err(WorkVcsError::SessionInvalid(format!(
            "session {} goal context anchor changed while resolving overview",
            options.session_id()
        )));
    }
    let primary_containment_relations =
        history::primary_containment_relations_at(connection, branch.head_commit_id)?;
    if primary_containment_relations.iter().any(|relation| {
        relation.workspace_id != active_workspace_id || relation.commit_id != branch.head_commit_id
    }) {
        return Err(WorkVcsError::SessionInvalid(format!(
            "session {} primary containment context anchor changed while resolving overview",
            options.session_id()
        )));
    }
    let structural_references =
        history::structural_references_at(connection, branch.head_commit_id)?;
    if structural_references.iter().any(|reference| {
        reference.workspace_id != active_workspace_id
            || reference.commit_id != branch.head_commit_id
    }) {
        return Err(WorkVcsError::SessionInvalid(format!(
            "session {} structural reference context anchor changed while resolving overview",
            options.session_id()
        )));
    }
    let acceptance_criteria = context_acceptance_criteria(
        connection,
        branch.head_commit_id,
        active_workspace_id,
        active_branch_id,
    )?;
    let verification_requirements =
        history::verification_requirements_at(connection, branch.head_commit_id)?;
    if verification_requirements.iter().any(|requirement| {
        requirement.workspace_id != active_workspace_id
            || requirement.commit_id != branch.head_commit_id
    }) {
        return Err(WorkVcsError::SessionInvalid(format!(
            "session {} verification requirement context anchor changed while resolving overview",
            options.session_id()
        )));
    }
    let records = history::records_at(connection, &RecordListOptions::new(branch.head_commit_id))?;
    if records.workspace_id != active_workspace_id || records.commit_id != branch.head_commit_id {
        return Err(WorkVcsError::SessionInvalid(format!(
            "session {} record context anchor changed while resolving overview",
            options.session_id()
        )));
    }
    let knowledge = history::knowledges_at(
        connection,
        &KnowledgeListOptions::new(branch.head_commit_id).with_status(KnowledgeStatus::Active),
    )?;
    if knowledge.workspace_id != active_workspace_id || knowledge.commit_id != branch.head_commit_id
    {
        return Err(WorkVcsError::SessionInvalid(format!(
            "session {} knowledge context anchor changed while resolving overview",
            options.session_id()
        )));
    }
    let knowledge_relations = history::knowledge_relations_at(
        connection,
        &KnowledgeRelationListOptions::new(branch.head_commit_id),
    )?;
    if knowledge_relations.workspace_id != active_workspace_id
        || knowledge_relations.commit_id != branch.head_commit_id
    {
        return Err(WorkVcsError::SessionInvalid(format!(
            "session {} knowledge relation context anchor changed while resolving overview",
            options.session_id()
        )));
    }
    let knowledge_exposure_relations =
        knowledge_exposure_relations_for_context(connection, &branch, &knowledge)?;
    let record_relations = history::record_relations_at(
        connection,
        &RecordRelationListOptions::new(branch.head_commit_id),
    )?;
    if record_relations.workspace_id != active_workspace_id
        || record_relations.commit_id != branch.head_commit_id
    {
        return Err(WorkVcsError::SessionInvalid(format!(
            "session {} record relation context anchor changed while resolving overview",
            options.session_id()
        )));
    }
    let record_knowledge_relations = history::record_knowledge_relations_at(
        connection,
        &RecordKnowledgeRelationListOptions::new(branch.head_commit_id),
    )?;
    if record_knowledge_relations.workspace_id != active_workspace_id
        || record_knowledge_relations.commit_id != branch.head_commit_id
    {
        return Err(WorkVcsError::SessionInvalid(format!(
            "session {} record knowledge relation context anchor changed while resolving overview",
            options.session_id()
        )));
    }
    Ok(ContextOverview {
        session,
        branch,
        runnable_tasks,
        tasks,
        plans,
        goals,
        primary_containment_relations,
        structural_references,
        acceptance_criteria,
        verification_requirements,
        knowledge,
        knowledge_relations,
        knowledge_exposure_relations,
        records,
        record_relations,
        record_knowledge_relations,
    })
}

pub(crate) fn context_packet(
    connection: &StoreConnection,
    options: &ContextPacketOptions,
) -> Result<ContextPacket> {
    let overview = context_overview(
        connection,
        &ContextOverviewOptions::new(options.session_id()),
    )?;
    let verifications = history::verifications_at(connection, overview.branch.head_commit_id)?;
    if verifications.iter().any(|verification| {
        verification.workspace_id != overview.branch.workspace_id
            || verification.commit_id != overview.branch.head_commit_id
    }) {
        return Err(WorkVcsError::SessionInvalid(format!(
            "session {} verification context anchor changed while resolving packet",
            options.session_id()
        )));
    }
    let transition_rationales = transition_rationales_for_context(
        connection,
        overview.branch.workspace_id,
        &overview.branch,
    )?;
    let workspace_runnable_tasks = if needs_workspace_runnable_context_for_focused_task(
        options.profile(),
        &overview,
    ) {
        let workspace_runnable_tasks = runnable::runnable_tasks(
            connection,
            &RunnableTasksOptions::new(options.session_id()).with_workspace_wide_scope(),
        )?;
        if workspace_runnable_tasks.workspace_id != overview.branch.workspace_id
            || workspace_runnable_tasks.branch_id != overview.branch.branch_id
            || workspace_runnable_tasks.head_commit_id != overview.branch.head_commit_id
        {
            return Err(WorkVcsError::SessionInvalid(format!(
                "session {} workspace-wide runnable context anchor changed while resolving packet",
                options.session_id()
            )));
        }
        Some(workspace_runnable_tasks)
    } else {
        None
    };
    let mut items = collect_context_items(
        &overview,
        &verifications,
        workspace_runnable_tasks.as_ref(),
        options.profile(),
        options.scope(),
        &transition_rationales,
    )?;
    items.sort_by_key(|item| item.priority);
    let available_items = items.len();
    let limit = options
        .budget_items()
        .map(NonZeroUsize::get)
        .unwrap_or(available_items);
    let omitted = if items.len() > limit {
        items.split_off(limit)
    } else {
        Vec::new()
    };
    let omission_summary = summarize_omitted_items(&omitted);

    Ok(ContextPacket {
        envelope: ContextPacketEnvelope::from_overview(&overview),
        profile: options.profile(),
        budget_items: options.budget_items(),
        scope: options.scope().cloned(),
        available_items,
        items,
        omission_summary,
    })
}

pub(crate) fn save_context_packet(
    connection: &mut StoreConnection,
    options: &ContextPacketOptions,
) -> Result<ContextPacketSaveResult> {
    connection.verify_foreign_keys()?;
    let packet = context_packet(connection, options)?;
    let packet_json = context_packet_value(&packet)?;
    let packet_json_bytes = canonical_bytes(&packet_json)?;
    let packet_json_text = String::from_utf8(packet_json_bytes.clone()).map_err(|error| {
        WorkVcsError::QueryInvalid(format!("context packet JSON was not UTF-8: {error}"))
    })?;
    let packet_digest = Digest::domain_separated(CONTEXT_PACKET_DIGEST_DOMAIN, &packet_json_bytes);
    let scope_json = packet
        .scope
        .as_ref()
        .map(canonical_object_json_text)
        .transpose()?;
    let context_packet_id = ContextPacketId::new_v7();
    let created_at_us = current_epoch_micros()?;
    let context_packet_id_bytes = context_packet_id.raw_bytes();
    let session_id_bytes = packet.envelope.session_id.raw_bytes();
    let workspace_id_bytes = packet.envelope.workspace_id.raw_bytes();
    let branch_id_bytes = packet.envelope.branch_id.raw_bytes();
    let head_commit_id_bytes = packet.envelope.head_commit_id.raw_bytes();
    let state_digest_bytes = packet.envelope.state_digest.as_bytes();
    let packet_digest_bytes = packet_digest.as_bytes();

    let transaction = connection
        .inner_mut()
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(storage_error)?;
    transaction
        .execute(
            "INSERT INTO context_packet_snapshot(
                context_packet_id,
                session_id,
                workspace_id,
                branch_id,
                head_commit_id,
                state_digest,
                profile,
                budget_items,
                scope_json,
                available_items,
                item_count,
                omitted_items,
                packet_digest,
                packet_json,
                created_at_us
             )
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
            params![
                &context_packet_id_bytes[..],
                &session_id_bytes[..],
                &workspace_id_bytes[..],
                &branch_id_bytes[..],
                &head_commit_id_bytes[..],
                &state_digest_bytes[..],
                packet.profile.as_str(),
                packet
                    .budget_items
                    .map(|value| usize_to_i64("context packet budget_items", value.get()))
                    .transpose()?,
                scope_json.as_deref(),
                usize_to_i64("context packet available_items", packet.available_items)?,
                usize_to_i64("context packet item_count", packet.items.len())?,
                usize_to_i64(
                    "context packet omitted_items",
                    packet.omission_summary.total
                )?,
                &packet_digest_bytes[..],
                packet_json_text,
                created_at_us
            ],
        )
        .map_err(storage_error)?;
    transaction.commit().map_err(storage_error)?;

    let snapshot = load_context_packet_snapshot(connection, context_packet_id)?;
    Ok(ContextPacketSaveResult { snapshot, packet })
}

pub(crate) fn context_packet_snapshot(
    connection: &StoreConnection,
    context_packet_id: ContextPacketId,
) -> Result<ContextPacketSnapshot> {
    connection.verify_foreign_keys()?;
    load_context_packet_snapshot(connection, context_packet_id)
}

pub(crate) fn context_packet_snapshots(
    connection: &StoreConnection,
    options: &ContextPacketListOptions,
) -> Result<ContextPacketListResult> {
    connection.verify_foreign_keys()?;
    let limit = i64::try_from(options.limit().get()).map_err(|_| {
        WorkVcsError::QueryInvalid("context packet list limit is too large".to_owned())
    })?;
    let session_id_bytes = options.session_id().raw_bytes();
    let mut statement = connection
        .inner()
        .prepare(
            "SELECT context_packet_id
             FROM context_packet_snapshot
             WHERE session_id = ?1
             ORDER BY created_at_us DESC, context_packet_id DESC
             LIMIT ?2",
        )
        .map_err(storage_error)?;
    let rows = statement
        .query_map(params![&session_id_bytes[..], limit], |row| {
            row.get::<_, Vec<u8>>(0)
        })
        .map_err(storage_error)?;

    let mut snapshots = Vec::new();
    for row in rows {
        let context_packet_id = decode_context_packet_id(
            "context_packet_snapshot.context_packet_id",
            row.map_err(storage_error)?,
        )?;
        snapshots.push(load_context_packet_snapshot(connection, context_packet_id)?);
    }
    Ok(ContextPacketListResult {
        session_id: options.session_id(),
        snapshots,
    })
}

fn load_context_packet_snapshot(
    connection: &StoreConnection,
    context_packet_id: ContextPacketId,
) -> Result<ContextPacketSnapshot> {
    let context_packet_id_bytes = context_packet_id.raw_bytes();
    let row = connection
        .inner()
        .query_row(
            "SELECT session_id,
                    workspace_id,
                    branch_id,
                    head_commit_id,
                    state_digest,
                    profile,
                    budget_items,
                    scope_json,
                    available_items,
                    item_count,
                    omitted_items,
                    packet_digest,
                    packet_json,
                    created_at_us
             FROM context_packet_snapshot
             WHERE context_packet_id = ?1",
            params![&context_packet_id_bytes[..]],
            |row| {
                Ok((
                    row.get::<_, Vec<u8>>(0)?,
                    row.get::<_, Vec<u8>>(1)?,
                    row.get::<_, Vec<u8>>(2)?,
                    row.get::<_, Vec<u8>>(3)?,
                    row.get::<_, Vec<u8>>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, Option<i64>>(6)?,
                    row.get::<_, Option<String>>(7)?,
                    row.get::<_, i64>(8)?,
                    row.get::<_, i64>(9)?,
                    row.get::<_, i64>(10)?,
                    row.get::<_, Vec<u8>>(11)?,
                    row.get::<_, String>(12)?,
                    row.get::<_, i64>(13)?,
                ))
            },
        )
        .optional()
        .map_err(storage_error)?;

    let Some((
        session_id,
        workspace_id,
        branch_id,
        head_commit_id,
        state_digest,
        profile,
        budget_items,
        scope_json,
        available_items,
        item_count,
        omitted_items,
        packet_digest,
        packet_json,
        created_at_us,
    )) = row
    else {
        return Err(WorkVcsError::QueryInvalid(format!(
            "context packet snapshot {context_packet_id} does not exist"
        )));
    };

    let packet_json_value =
        parse_canonical_object_json("context_packet_snapshot.packet_json", &packet_json)?;
    let scope = scope_json
        .map(|json| parse_canonical_object_json("context_packet_snapshot.scope_json", &json))
        .transpose()?;
    let budget_items = match budget_items {
        Some(value) => Some(non_zero_usize_from_i64(
            "context_packet_snapshot.budget_items",
            value,
        )?),
        None => None,
    };

    let session_id = decode_session_id("context_packet_snapshot.session_id", session_id)?;
    let workspace_id = decode_workspace_id("context_packet_snapshot.workspace_id", workspace_id)?;
    let branch_id = decode_branch_id("context_packet_snapshot.branch_id", branch_id)?;
    let head_commit_id =
        decode_commit_id("context_packet_snapshot.head_commit_id", head_commit_id)?;
    let state_digest = decode_digest("context_packet_snapshot.state_digest", state_digest)?;
    let profile = parse_context_profile("context_packet_snapshot.profile", &profile)?;
    let available_items =
        usize_from_i64("context_packet_snapshot.available_items", available_items)?;
    let item_count = usize_from_i64("context_packet_snapshot.item_count", item_count)?;
    let omitted_items = usize_from_i64("context_packet_snapshot.omitted_items", omitted_items)?;
    let packet_digest = decode_digest("context_packet_snapshot.packet_digest", packet_digest)?;
    let packet_json_bytes = canonical_bytes(&packet_json_value)?;
    let expected_packet_digest =
        Digest::domain_separated(CONTEXT_PACKET_DIGEST_DOMAIN, &packet_json_bytes);
    if packet_digest != expected_packet_digest {
        return Err(WorkVcsError::StorageFailure(format!(
            "context packet snapshot {context_packet_id} digest does not match packet_json"
        )));
    }
    validate_context_packet_snapshot_columns(
        context_packet_id,
        &packet_json_value,
        ContextPacketSnapshotColumnExpectations {
            session_id,
            workspace_id,
            branch_id,
            head_commit_id,
            state_digest,
            profile,
            budget_items,
            scope: scope.as_ref(),
            available_items,
            item_count,
            omitted_items,
        },
    )?;

    Ok(ContextPacketSnapshot {
        context_packet_id,
        session_id,
        workspace_id,
        branch_id,
        head_commit_id,
        state_digest,
        profile,
        budget_items,
        scope,
        available_items,
        item_count,
        omitted_items,
        packet_digest,
        packet_json: packet_json_value,
        created_at_us,
    })
}

struct ContextPacketSnapshotColumnExpectations<'a> {
    session_id: SessionId,
    workspace_id: WorkspaceId,
    branch_id: BranchId,
    head_commit_id: CommitId,
    state_digest: Digest,
    profile: ContextProfile,
    budget_items: Option<NonZeroUsize>,
    scope: Option<&'a CanonicalValue>,
    available_items: usize,
    item_count: usize,
    omitted_items: usize,
}

fn validate_context_packet_snapshot_columns(
    context_packet_id: ContextPacketId,
    packet_json: &CanonicalValue,
    expected: ContextPacketSnapshotColumnExpectations<'_>,
) -> Result<()> {
    let packet = canonical_object_fields("context_packet_snapshot.packet_json", packet_json)?;
    expect_packet_json_string(
        context_packet_id,
        packet,
        "context_packet_format",
        CONTEXT_PACKET_FORMAT,
    )?;
    expect_packet_json_integer(
        context_packet_id,
        packet,
        "context_packet_format_version",
        CONTEXT_PACKET_FORMAT_VERSION,
    )?;
    expect_packet_json_string(
        context_packet_id,
        packet,
        "profile",
        expected.profile.as_str(),
    )?;
    expect_packet_json_optional_usize(
        context_packet_id,
        packet,
        "budget_items",
        expected.budget_items,
    )?;
    expect_packet_json_scope(context_packet_id, packet, expected.scope)?;
    expect_packet_json_usize(
        context_packet_id,
        packet,
        "available_items",
        expected.available_items,
    )?;

    let items = canonical_array_field(context_packet_id, packet, "items")?;
    if items.len() != expected.item_count {
        return Err(context_packet_snapshot_mismatch(
            context_packet_id,
            "items length",
            expected.item_count.to_string(),
            items.len().to_string(),
        ));
    }

    let omission_summary = canonical_object_field(context_packet_id, packet, "omission_summary")?;
    expect_packet_json_usize(
        context_packet_id,
        omission_summary,
        "total",
        expected.omitted_items,
    )?;

    let envelope = canonical_object_field(context_packet_id, packet, "envelope")?;
    expect_packet_json_string(
        context_packet_id,
        envelope,
        "session_id",
        &expected.session_id.to_string(),
    )?;
    expect_packet_json_string(
        context_packet_id,
        envelope,
        "workspace_id",
        &expected.workspace_id.to_string(),
    )?;
    expect_packet_json_string(
        context_packet_id,
        envelope,
        "branch_id",
        &expected.branch_id.to_string(),
    )?;
    expect_packet_json_string(
        context_packet_id,
        envelope,
        "head_commit_id",
        &expected.head_commit_id.to_string(),
    )?;
    expect_packet_json_string(
        context_packet_id,
        envelope,
        "state_digest",
        &expected.state_digest.to_string(),
    )?;
    Ok(())
}

fn canonical_object_fields<'a>(
    label: &str,
    value: &'a CanonicalValue,
) -> Result<&'a [(String, CanonicalValue)]> {
    match value {
        CanonicalValue::Object(entries) => Ok(entries),
        _ => Err(WorkVcsError::StorageFailure(format!(
            "{label} must be an object"
        ))),
    }
}

fn canonical_object_field<'a>(
    context_packet_id: ContextPacketId,
    entries: &'a [(String, CanonicalValue)],
    field: &str,
) -> Result<&'a [(String, CanonicalValue)]> {
    let value = canonical_field(context_packet_id, entries, field)?;
    canonical_object_fields(
        &format!("context packet snapshot {context_packet_id} packet_json.{field}"),
        value,
    )
}

fn canonical_array_field<'a>(
    context_packet_id: ContextPacketId,
    entries: &'a [(String, CanonicalValue)],
    field: &str,
) -> Result<&'a [CanonicalValue]> {
    match canonical_field(context_packet_id, entries, field)? {
        CanonicalValue::Array(values) => Ok(values),
        _ => Err(WorkVcsError::StorageFailure(format!(
            "context packet snapshot {context_packet_id} packet_json.{field} must be an array"
        ))),
    }
}

fn canonical_field<'a>(
    context_packet_id: ContextPacketId,
    entries: &'a [(String, CanonicalValue)],
    field: &str,
) -> Result<&'a CanonicalValue> {
    entries
        .iter()
        .find_map(|(key, value)| (key == field).then_some(value))
        .ok_or_else(|| {
            WorkVcsError::StorageFailure(format!(
                "context packet snapshot {context_packet_id} packet_json missing field {field}"
            ))
        })
}

fn expect_packet_json_string(
    context_packet_id: ContextPacketId,
    entries: &[(String, CanonicalValue)],
    field: &str,
    expected: &str,
) -> Result<()> {
    match canonical_field(context_packet_id, entries, field)? {
        CanonicalValue::String(actual) if actual == expected => Ok(()),
        CanonicalValue::String(actual) => Err(context_packet_snapshot_mismatch(
            context_packet_id,
            field,
            expected.to_owned(),
            actual.clone(),
        )),
        _ => Err(WorkVcsError::StorageFailure(format!(
            "context packet snapshot {context_packet_id} packet_json.{field} must be a string"
        ))),
    }
}

fn expect_packet_json_integer(
    context_packet_id: ContextPacketId,
    entries: &[(String, CanonicalValue)],
    field: &str,
    expected: i64,
) -> Result<()> {
    match canonical_field(context_packet_id, entries, field)? {
        CanonicalValue::Integer(actual) if actual.get() == expected => Ok(()),
        CanonicalValue::Integer(actual) => Err(context_packet_snapshot_mismatch(
            context_packet_id,
            field,
            expected.to_string(),
            actual.get().to_string(),
        )),
        _ => Err(WorkVcsError::StorageFailure(format!(
            "context packet snapshot {context_packet_id} packet_json.{field} must be an integer"
        ))),
    }
}

fn expect_packet_json_usize(
    context_packet_id: ContextPacketId,
    entries: &[(String, CanonicalValue)],
    field: &str,
    expected: usize,
) -> Result<()> {
    expect_packet_json_integer(
        context_packet_id,
        entries,
        field,
        usize_to_i64(field, expected)?,
    )
}

fn expect_packet_json_optional_usize(
    context_packet_id: ContextPacketId,
    entries: &[(String, CanonicalValue)],
    field: &str,
    expected: Option<NonZeroUsize>,
) -> Result<()> {
    let expected_i64 = expected
        .map(|value| usize_to_i64(field, value.get()))
        .transpose()?;
    match (
        expected,
        canonical_field(context_packet_id, entries, field)?,
    ) {
        (None, CanonicalValue::Null) => Ok(()),
        (Some(_), CanonicalValue::Integer(actual)) if Some(actual.get()) == expected_i64 => Ok(()),
        (expected, actual) => Err(context_packet_snapshot_mismatch(
            context_packet_id,
            field,
            expected
                .map(|value| value.get().to_string())
                .unwrap_or_else(|| "null".to_owned()),
            context_packet_json_scalar(actual)?,
        )),
    }
}

fn expect_packet_json_scope(
    context_packet_id: ContextPacketId,
    entries: &[(String, CanonicalValue)],
    expected: Option<&CanonicalValue>,
) -> Result<()> {
    let actual = canonical_field(context_packet_id, entries, "scope")?;
    match (expected, actual) {
        (None, CanonicalValue::Null) => Ok(()),
        (Some(expected), actual) if expected == actual => Ok(()),
        (expected, actual) => Err(context_packet_snapshot_mismatch(
            context_packet_id,
            "scope",
            expected
                .map(context_packet_json_scalar)
                .transpose()?
                .unwrap_or_else(|| "null".to_owned()),
            context_packet_json_scalar(actual)?,
        )),
    }
}

fn context_packet_json_scalar(value: &CanonicalValue) -> Result<String> {
    canonical_json_text(value)
}

fn context_packet_snapshot_mismatch(
    context_packet_id: ContextPacketId,
    field: &str,
    expected: String,
    actual: String,
) -> WorkVcsError {
    WorkVcsError::StorageFailure(format!(
        "context packet snapshot {context_packet_id} {field} does not match packet_json: column={expected} packet_json={actual}"
    ))
}

fn context_acceptance_criteria(
    connection: &StoreConnection,
    commit_id: CommitId,
    workspace_id: WorkspaceId,
    branch_id: BranchId,
) -> Result<Vec<ContextAcceptanceCriterionSnapshot>> {
    let criteria = history::acceptance_criteria_at(connection, commit_id)?;
    criteria
        .into_iter()
        .map(|criterion| {
            if criterion.workspace_id != workspace_id || criterion.commit_id != commit_id {
                return Err(WorkVcsError::SessionInvalid(format!(
                    "acceptance criterion {} does not belong to context commit {}",
                    criterion.acceptance_criterion_entity_id, commit_id
                )));
            }
            let effective_status = history::acceptance_criterion_effective_status_for_branch(
                connection,
                branch_id,
                criterion.acceptance_criterion_entity_id,
            )?;
            Ok(ContextAcceptanceCriterionSnapshot {
                snapshot: criterion,
                effective_status,
            })
        })
        .collect()
}

fn transition_rationales_for_context(
    connection: &StoreConnection,
    workspace_id: WorkspaceId,
    branch: &BranchHead,
) -> Result<Vec<ContextTransitionRationaleSnapshot>> {
    let history = history::query_history(
        connection,
        &HistoryQueryOptions::from_commit(branch.head_commit_id)
            .with_limit(CONTEXT_TRANSITION_RATIONALE_HISTORY_LIMIT)?,
    )?;
    let mut rationales = Vec::new();
    for entry in history.entries {
        if entry.workspace_id != workspace_id {
            return Err(WorkVcsError::SessionInvalid(format!(
                "context commit {} belongs to workspace {}, not {}",
                entry.commit_id, entry.workspace_id, workspace_id
            )));
        }
        let changeset = history::changeset(connection, entry.changeset_id)?;
        if changeset.workspace_id != workspace_id {
            return Err(WorkVcsError::SessionInvalid(format!(
                "context ChangeSet {} belongs to workspace {}, not {}",
                entry.changeset_id, changeset.workspace_id, workspace_id
            )));
        }
        if !rationale_json_has_content(&changeset)? {
            continue;
        }
        rationales.push(ContextTransitionRationaleSnapshot {
            commit_id: entry.commit_id,
            changeset_id: entry.changeset_id,
            commit_kind: entry.commit_kind,
            committed_at_us: entry.committed_at_us,
            operation_type: changeset.operation_type,
            operation_schema_version: changeset.operation_schema_version,
            changeset_created_at_us: changeset.created_at_us,
            rationale_json: changeset.rationale_json,
            rationale_digest: changeset.rationale_digest,
            rationale_size_bytes: changeset.rationale_size_bytes,
            origin_session_id: changeset.origin_session_id,
            change_operation_count: changeset.change_operation_count,
            causal_anchor_count: changeset.causal_anchor_count,
            event_count: changeset.event_count,
        });
    }
    Ok(rationales)
}

fn rationale_json_has_content(changeset: &ChangeSetSnapshot) -> Result<bool> {
    let rationale = parse_canonical_json(changeset.rationale_json.as_bytes())?;
    Ok(!matches!(
        rationale,
        CanonicalValue::Object(entries) if entries.is_empty()
    ))
}

fn collect_context_items(
    context: &ContextOverview,
    verifications: &[VerificationSnapshot],
    workspace_runnable_tasks: Option<&RunnableTasksProjection>,
    profile: ContextProfile,
    scope: Option<&CanonicalValue>,
    transition_rationales: &[ContextTransitionRationaleSnapshot],
) -> Result<Vec<ContextItem>> {
    let mut items = Vec::new();
    let session = &context.session;
    let criteria_by_id = context
        .acceptance_criteria
        .iter()
        .map(|criterion| (criterion.snapshot.acceptance_criterion_entity_id, criterion))
        .collect::<BTreeMap<_, _>>();
    let requirements_by_id = context
        .verification_requirements
        .iter()
        .map(|requirement| (requirement.verification_requirement_entity_id, requirement))
        .collect::<BTreeMap<_, _>>();
    let resource_backed_verifications = resource_backed_verifications_by_requirement(verifications);
    let resource_requirement_indexes = ResourceRequirementContextIndexes {
        criteria_by_id: &criteria_by_id,
        requirements_by_id: &requirements_by_id,
        resource_backed_verifications: &resource_backed_verifications,
    };
    let tasks_by_id = context
        .tasks
        .iter()
        .map(|task| (task.task_entity_id, task))
        .collect::<BTreeMap<_, _>>();
    let plans_by_id = context
        .plans
        .iter()
        .map(|plan| (plan.plan_entity_id, plan))
        .collect::<BTreeMap<_, _>>();
    let goals_by_id = context
        .goals
        .iter()
        .map(|goal| (goal.goal_entity_id, goal))
        .collect::<BTreeMap<_, _>>();
    let primary_parent_by_child = primary_parent_by_child(&context.primary_containment_relations)?;
    push_context_item(
        &mut items,
        profile,
        ContextPriority::P0,
        ContextItemCategory::SessionAnchor,
        ContextItemSubject::Session {
            session_id: session.session_id,
        },
        format!(
            "session active workspace={} branch={} focus={}",
            session
                .active_workspace_id
                .map(|workspace_id| workspace_id.to_string())
                .unwrap_or_else(|| "none".to_owned()),
            session
                .active_branch_id
                .map(|branch_id| branch_id.to_string())
                .unwrap_or_else(|| "none".to_owned()),
            session
                .focus
                .as_ref()
                .map(|focus| focus.focus_entity_id.to_string())
                .unwrap_or_else(|| "none".to_owned())
        ),
    );
    push_context_item(
        &mut items,
        profile,
        ContextPriority::P0,
        ContextItemCategory::BranchOverview,
        ContextItemSubject::Branch {
            branch_id: context.branch.branch_id,
            commit_id: context.branch.head_commit_id,
        },
        format!(
            "branch {} head={} state={}",
            context.branch.name, context.branch.head_commit_id, context.branch.state_digest
        ),
    );
    for candidate in &context.runnable_tasks.candidates {
        if !context_candidate_is_current(candidate, session) {
            continue;
        }
        push_context_item(
            &mut items,
            profile,
            ContextPriority::P0,
            ContextItemCategory::CurrentTask,
            ContextItemSubject::Task {
                task_entity_id: candidate.task.task_entity_id,
            },
            format!(
                "task status={} priority={} runnable={}: {}",
                candidate.task.state.status,
                candidate.task.state.priority,
                candidate.runnable,
                candidate.task.state.description
            ),
        );
        if let Some(goal_plan_path_summary) = goal_plan_path_summary(
            candidate.task.task_entity_id,
            &primary_parent_by_child,
            &plans_by_id,
            &goals_by_id,
        )? {
            push_context_item(
                &mut items,
                profile,
                ContextPriority::P1,
                ContextItemCategory::GoalPlanPath,
                ContextItemSubject::GoalPlanPath {
                    task_entity_id: candidate.task.task_entity_id,
                },
                goal_plan_path_summary,
            );
        }
        for criterion_ref in &candidate.task.state.acceptance_criteria {
            let criterion = criteria_by_id
                .get(&criterion_ref.acceptance_criterion_entity_id)
                .ok_or_else(|| {
                    WorkVcsError::TaskInvalid(format!(
                        "task {} references missing acceptance criterion {}",
                        candidate.task.task_entity_id, criterion_ref.acceptance_criterion_entity_id
                    ))
                })?;
            if criterion.snapshot.task_entity_id != candidate.task.task_entity_id
                || criterion.snapshot.local_key != criterion_ref.local_key
            {
                return Err(WorkVcsError::TaskInvalid(format!(
                    "task {} acceptance criterion reference {} does not match stored identity",
                    candidate.task.task_entity_id, criterion_ref.local_key
                )));
            }
            push_context_item(
                &mut items,
                profile,
                ContextPriority::P1,
                ContextItemCategory::AcceptanceCriterion,
                ContextItemSubject::AcceptanceCriterion {
                    acceptance_criterion_entity_id: criterion
                        .snapshot
                        .acceptance_criterion_entity_id,
                },
                format!(
                    "acceptance criterion task={} local_key={} classification={} status={} requirements={}: {}",
                    criterion.snapshot.task_entity_id,
                    criterion.snapshot.local_key,
                    criterion.snapshot.state.classification,
                    criterion.effective_status,
                    criterion.snapshot.state.verification_requirements.len(),
                    criterion.snapshot.state.statement
                ),
            );
            for requirement_ref in &criterion.snapshot.state.verification_requirements {
                let requirement = requirements_by_id
                    .get(&requirement_ref.verification_requirement_entity_id)
                    .ok_or_else(|| {
                        WorkVcsError::TaskInvalid(format!(
                            "acceptance criterion {} references missing verification requirement {}",
                            criterion.snapshot.acceptance_criterion_entity_id,
                            requirement_ref.verification_requirement_entity_id
                        ))
                    })?;
                if requirement.acceptance_criterion_entity_id
                    != criterion.snapshot.acceptance_criterion_entity_id
                    || requirement.local_key != requirement_ref.local_key
                {
                    return Err(WorkVcsError::TaskInvalid(format!(
                        "acceptance criterion {} verification requirement reference {} does not match stored identity",
                        criterion.snapshot.acceptance_criterion_entity_id,
                        requirement_ref.local_key
                    )));
                }
                let mut summary = format!(
                    "verification requirement criterion={} local_key={}: {}",
                    requirement.acceptance_criterion_entity_id,
                    requirement.local_key,
                    requirement.state.statement
                );
                if let Some(verifications) = resource_backed_verifications
                    .get(&requirement.verification_requirement_entity_id)
                {
                    summary.push(' ');
                    summary.push_str(&verification_requirement_resource_basis_context_summary(
                        verifications,
                    ));
                }
                push_context_item(
                    &mut items,
                    profile,
                    ContextPriority::P1,
                    ContextItemCategory::VerificationRequirement,
                    ContextItemSubject::VerificationRequirement {
                        verification_requirement_entity_id: requirement
                            .verification_requirement_entity_id,
                    },
                    summary,
                );
            }
        }
        push_context_item(
            &mut items,
            profile,
            ContextPriority::P2,
            ContextItemCategory::TaskReadiness,
            ContextItemSubject::TaskReadiness {
                task_entity_id: candidate.task.task_entity_id,
            },
            format!(
                "readiness lifecycle_eligible={} dependency_ready={} claim={} blocked_reasons={} unsatisfied_dependencies={}",
                candidate.lifecycle_eligible,
                candidate.dependency_ready,
                claim_coordination_summary(&candidate.claim_coordination),
                blocked_reason_summary(&candidate.blocked_reasons),
                entity_id_list_summary(&candidate.unsatisfied_dependency_entity_ids)
            ),
        );
        for dependency_task_entity_id in &candidate.unsatisfied_dependency_entity_ids {
            let dependency_task = tasks_by_id.get(dependency_task_entity_id).ok_or_else(|| {
                WorkVcsError::TaskInvalid(format!(
                    "task {} references missing dependency task {}",
                    candidate.task.task_entity_id, dependency_task_entity_id
                ))
            })?;
            let mut summary = format!(
                "blocked dependency task={} dependency={} dependency_status={} dependency_priority={}: {}",
                candidate.task.task_entity_id,
                dependency_task.task_entity_id,
                dependency_task.state.status,
                dependency_task.state.priority,
                dependency_task.state.description
            );
            if let Some(resource_summary) = blocked_dependency_resource_basis_context_summary(
                dependency_task,
                &criteria_by_id,
                &requirements_by_id,
                &resource_backed_verifications,
            )? {
                summary.push(' ');
                summary.push_str(&resource_summary);
            }
            push_context_item(
                &mut items,
                profile,
                ContextPriority::P2,
                ContextItemCategory::BlockedDependency,
                ContextItemSubject::BlockedDependency {
                    task_entity_id: candidate.task.task_entity_id,
                    dependency_task_entity_id: *dependency_task_entity_id,
                },
                summary,
            );
        }
    }
    push_focused_peer_resource_requirement_context_items(
        &mut items,
        context,
        workspace_runnable_tasks,
        profile,
        &primary_parent_by_child,
        &resource_requirement_indexes,
    )?;
    push_focused_structural_reference_resource_requirement_context_items(
        &mut items,
        context,
        profile,
        &tasks_by_id,
        &plans_by_id,
        &resource_requirement_indexes,
    )?;
    for relation in &context.knowledge_relations.relations {
        push_context_item(
            &mut items,
            profile,
            ContextPriority::P3,
            ContextItemCategory::DirectCausalChain,
            ContextItemSubject::Relation {
                relation_id: relation.relation_id,
            },
            format!(
                "knowledge relation {} {} -> {}",
                relation.relation_type,
                relation.replacement_knowledge_entity_id,
                relation.prior_knowledge_entity_id
            ),
        );
    }
    for relation in &context.knowledge_exposure_relations {
        push_context_item(
            &mut items,
            profile,
            ContextPriority::P3,
            ContextItemCategory::DirectCausalChain,
            ContextItemSubject::Relation {
                relation_id: relation.relation_id,
            },
            format!(
                "knowledge exposure relation {} {}",
                why_relation_kind_summary(relation.relation_kind),
                relation.relation_id
            ),
        );
    }
    for relation in &context.record_relations.relations {
        push_context_item(
            &mut items,
            profile,
            ContextPriority::P3,
            ContextItemCategory::DirectCausalChain,
            ContextItemSubject::Relation {
                relation_id: relation.relation_id,
            },
            format!(
                "record relation {} {} -> {}",
                relation.relation_type,
                relation.source_record_entity_id,
                relation.target_record_entity_id
            ),
        );
    }
    for relation in &context.record_knowledge_relations.relations {
        push_context_item(
            &mut items,
            profile,
            ContextPriority::P3,
            ContextItemCategory::DirectCausalChain,
            ContextItemSubject::Relation {
                relation_id: relation.relation_id,
            },
            format!(
                "record knowledge relation {} {} -> {}",
                relation.relation_type,
                relation.source_record_entity_id,
                relation.target_knowledge_entity_id
            ),
        );
    }
    for rationale in transition_rationales {
        push_context_item(
            &mut items,
            profile,
            ContextPriority::P3,
            ContextItemCategory::TransitionRationale,
            ContextItemSubject::ChangeSet {
                changeset_id: rationale.changeset_id,
                commit_id: rationale.commit_id,
            },
            transition_rationale_context_summary(rationale),
        );
    }
    for record in &context.records.records {
        let (priority, category) =
            record_priority_and_category(record.state.kind, record.state.status);
        push_context_item(
            &mut items,
            profile,
            priority,
            category,
            ContextItemSubject::Record {
                record_entity_id: record.record_entity_id,
            },
            record_context_summary(record, &context.record_relations.relations)?,
        );
    }
    for knowledge in &context.knowledge.knowledge {
        if !knowledge_matches_context_scope(&knowledge.state.scope, scope) {
            continue;
        }
        push_context_item(
            &mut items,
            profile,
            ContextPriority::P7,
            ContextItemCategory::ScopedKnowledge,
            ContextItemSubject::Knowledge {
                knowledge_entity_id: knowledge.knowledge_entity_id,
            },
            format!(
                "knowledge {}: {}",
                knowledge.state.status, knowledge.state.statement
            ),
        );
    }
    for workspace_id in &session.context_workspaces {
        push_context_item(
            &mut items,
            profile,
            ContextPriority::P8,
            ContextItemCategory::SessionContinuity,
            ContextItemSubject::Workspace {
                workspace_id: *workspace_id,
            },
            format!("session context workspace {workspace_id}"),
        );
    }
    Ok(items)
}

fn needs_workspace_runnable_context_for_focused_task(
    profile: ContextProfile,
    context: &ContextOverview,
) -> bool {
    if profile == ContextProfile::Brief {
        return false;
    }
    let Some(focus) = &context.session.focus else {
        return false;
    };
    context
        .tasks
        .iter()
        .any(|task| task.task_entity_id == focus.focus_entity_id)
}

fn push_focused_peer_resource_requirement_context_items(
    items: &mut Vec<ContextItem>,
    context: &ContextOverview,
    workspace_runnable_tasks: Option<&RunnableTasksProjection>,
    profile: ContextProfile,
    primary_parent_by_child: &BTreeMap<EntityId, &PrimaryContainmentSnapshot>,
    resource_requirement_indexes: &ResourceRequirementContextIndexes<'_>,
) -> Result<()> {
    if profile == ContextProfile::Brief {
        return Ok(());
    }
    let Some(workspace_runnable_tasks) = workspace_runnable_tasks else {
        return Ok(());
    };
    let Some(focus) = &context.session.focus else {
        return Ok(());
    };
    let focused_task_id = focus.focus_entity_id;
    if !context
        .tasks
        .iter()
        .any(|task| task.task_entity_id == focused_task_id)
    {
        return Ok(());
    }
    let Some(focused_parent) = primary_parent_by_child.get(&focused_task_id) else {
        return Ok(());
    };
    if focused_parent.child_kind != PrimaryContainmentEndpointKind::Task {
        return Err(WorkVcsError::RelationInvalid(format!(
            "primary containment relation {} child kind {} does not match focused task {}",
            focused_parent.relation_id, focused_parent.child_kind, focused_task_id
        )));
    }
    if focused_parent.parent_kind != PrimaryContainmentEndpointKind::Plan {
        return Ok(());
    }
    let focused_plan_id = focused_parent.parent_entity_id;
    let focused_goal_id = direct_goal_parent_for_plan(focused_plan_id, primary_parent_by_child)?;
    let focused_candidate_ids = context
        .runnable_tasks
        .candidates
        .iter()
        .map(|candidate| candidate.task.task_entity_id)
        .collect::<BTreeSet<_>>();

    for candidate in &workspace_runnable_tasks.candidates {
        if !candidate.runnable
            || candidate.task.task_entity_id == focused_task_id
            || focused_candidate_ids.contains(&candidate.task.task_entity_id)
        {
            continue;
        }
        let Some(peer_parent) = primary_parent_by_child.get(&candidate.task.task_entity_id) else {
            continue;
        };
        if peer_parent.child_kind != PrimaryContainmentEndpointKind::Task {
            return Err(WorkVcsError::RelationInvalid(format!(
                "primary containment relation {} child kind {} does not match peer task {}",
                peer_parent.relation_id, peer_parent.child_kind, candidate.task.task_entity_id
            )));
        }
        if peer_parent.parent_kind != PrimaryContainmentEndpointKind::Plan {
            continue;
        }
        let peer_plan_id = peer_parent.parent_entity_id;
        let peer_context = if peer_plan_id == focused_plan_id {
            TaskResourceRequirementContext::SamePlan {
                parent_plan_id: focused_plan_id,
            }
        } else if let Some(parent_goal_id) = focused_goal_id {
            if direct_goal_parent_for_plan(peer_plan_id, primary_parent_by_child)?
                != Some(parent_goal_id)
            {
                continue;
            }
            TaskResourceRequirementContext::SameGoal {
                parent_goal_id,
                focused_plan_id,
                peer_plan_id,
            }
        } else {
            continue;
        };
        push_task_resource_requirement_context_items(
            items,
            &candidate.task,
            peer_context,
            profile,
            resource_requirement_indexes,
        )?;
    }
    Ok(())
}

fn push_focused_structural_reference_resource_requirement_context_items(
    items: &mut Vec<ContextItem>,
    context: &ContextOverview,
    profile: ContextProfile,
    tasks_by_id: &BTreeMap<EntityId, &TaskSnapshot>,
    plans_by_id: &BTreeMap<EntityId, &PlanSnapshot>,
    resource_requirement_indexes: &ResourceRequirementContextIndexes<'_>,
) -> Result<()> {
    if profile == ContextProfile::Brief {
        return Ok(());
    }
    let Some(focus) = &context.session.focus else {
        return Ok(());
    };
    let focused_entity_id = focus.focus_entity_id;
    let referrer_kind = if context
        .plans
        .iter()
        .any(|plan| plan.plan_entity_id == focused_entity_id)
    {
        StructuralReferenceEndpointKind::Plan
    } else if context
        .goals
        .iter()
        .any(|goal| goal.goal_entity_id == focused_entity_id)
    {
        StructuralReferenceEndpointKind::Goal
    } else {
        return Ok(());
    };

    let mut references = context
        .structural_references
        .iter()
        .filter(|reference| {
            reference.referrer_entity_id == focused_entity_id
                && reference.referrer_kind == referrer_kind
                && matches!(
                    reference.target_kind,
                    StructuralReferenceEndpointKind::Task | StructuralReferenceEndpointKind::Plan
                )
        })
        .collect::<Vec<_>>();
    references.sort_by_key(|reference| reference.relation_id);

    for reference in references {
        match reference.target_kind {
            StructuralReferenceEndpointKind::Task => {
                let referenced_task =
                    tasks_by_id
                        .get(&reference.target_entity_id)
                        .ok_or_else(|| {
                            WorkVcsError::TaskInvalid(format!(
                                "structural reference {} targets missing task {}",
                                reference.relation_id, reference.target_entity_id
                            ))
                        })?;
                push_task_resource_requirement_context_items(
                    items,
                    referenced_task,
                    TaskResourceRequirementContext::StructuralReference {
                        relation_id: reference.relation_id,
                        referrer_entity_id: focused_entity_id,
                        referrer_kind,
                    },
                    profile,
                    resource_requirement_indexes,
                )?;
            }
            StructuralReferenceEndpointKind::Plan => {
                if !plans_by_id.contains_key(&reference.target_entity_id) {
                    return Err(WorkVcsError::PlanInvalid(format!(
                        "structural reference {} targets missing plan {}",
                        reference.relation_id, reference.target_entity_id
                    )));
                }
                let mut child_task_relations = context
                    .primary_containment_relations
                    .iter()
                    .filter(|relation| {
                        relation.parent_entity_id == reference.target_entity_id
                            && relation.parent_kind == PrimaryContainmentEndpointKind::Plan
                            && relation.child_kind == PrimaryContainmentEndpointKind::Task
                    })
                    .collect::<Vec<_>>();
                child_task_relations.sort_by_key(|relation| relation.relation_id);
                for containment in child_task_relations {
                    let referenced_task = tasks_by_id
                        .get(&containment.child_entity_id)
                        .ok_or_else(|| {
                            WorkVcsError::TaskInvalid(format!(
                                "structural reference {} target plan {} contains missing task {}",
                                reference.relation_id,
                                reference.target_entity_id,
                                containment.child_entity_id
                            ))
                        })?;
                    push_task_resource_requirement_context_items(
                        items,
                        referenced_task,
                        TaskResourceRequirementContext::StructuralReferencePlanTarget {
                            relation_id: reference.relation_id,
                            referrer_entity_id: focused_entity_id,
                            referrer_kind,
                            referenced_plan_entity_id: reference.target_entity_id,
                            containment_relation_id: containment.relation_id,
                        },
                        profile,
                        resource_requirement_indexes,
                    )?;
                }
                let mut child_plan_relations = context
                    .primary_containment_relations
                    .iter()
                    .filter(|relation| {
                        relation.parent_entity_id == reference.target_entity_id
                            && relation.parent_kind == PrimaryContainmentEndpointKind::Plan
                            && relation.child_kind == PrimaryContainmentEndpointKind::Plan
                    })
                    .collect::<Vec<_>>();
                child_plan_relations.sort_by_key(|relation| relation.relation_id);
                for child_plan_containment in child_plan_relations {
                    if !plans_by_id.contains_key(&child_plan_containment.child_entity_id) {
                        return Err(WorkVcsError::PlanInvalid(format!(
                            "structural reference {} target plan {} contains missing child plan {}",
                            reference.relation_id,
                            reference.target_entity_id,
                            child_plan_containment.child_entity_id
                        )));
                    }
                    let mut nested_child_task_relations = context
                        .primary_containment_relations
                        .iter()
                        .filter(|relation| {
                            relation.parent_entity_id == child_plan_containment.child_entity_id
                                && relation.parent_kind == PrimaryContainmentEndpointKind::Plan
                                && relation.child_kind == PrimaryContainmentEndpointKind::Task
                        })
                        .collect::<Vec<_>>();
                    nested_child_task_relations.sort_by_key(|relation| relation.relation_id);
                    for task_containment in nested_child_task_relations {
                        let referenced_task =
                            tasks_by_id
                                .get(&task_containment.child_entity_id)
                                .ok_or_else(|| {
                                    WorkVcsError::TaskInvalid(format!(
                                        "structural reference {} target plan {} child plan {} contains missing task {}",
                                        reference.relation_id,
                                        reference.target_entity_id,
                                        child_plan_containment.child_entity_id,
                                        task_containment.child_entity_id
                                    ))
                                })?;
                        push_task_resource_requirement_context_items(
                            items,
                            referenced_task,
                            TaskResourceRequirementContext::StructuralReferenceNestedPlanTarget {
                                relation_id: reference.relation_id,
                                referrer_entity_id: focused_entity_id,
                                referrer_kind,
                                referenced_plan_entity_id: reference.target_entity_id,
                                nested_plan_entity_id: child_plan_containment.child_entity_id,
                                plan_plan_containment_relation_id: child_plan_containment
                                    .relation_id,
                                plan_task_containment_relation_id: task_containment.relation_id,
                            },
                            profile,
                            resource_requirement_indexes,
                        )?;
                    }
                    let mut second_level_child_plan_relations = context
                        .primary_containment_relations
                        .iter()
                        .filter(|relation| {
                            relation.parent_entity_id == child_plan_containment.child_entity_id
                                && relation.parent_kind == PrimaryContainmentEndpointKind::Plan
                                && relation.child_kind == PrimaryContainmentEndpointKind::Plan
                        })
                        .collect::<Vec<_>>();
                    second_level_child_plan_relations.sort_by_key(|relation| relation.relation_id);
                    for second_level_child_plan_containment in second_level_child_plan_relations {
                        if !plans_by_id
                            .contains_key(&second_level_child_plan_containment.child_entity_id)
                        {
                            return Err(WorkVcsError::PlanInvalid(format!(
                                "structural reference {} target plan {} child plan {} contains missing child plan {}",
                                reference.relation_id,
                                reference.target_entity_id,
                                child_plan_containment.child_entity_id,
                                second_level_child_plan_containment.child_entity_id
                            )));
                        }
                        let mut second_level_task_relations = context
                            .primary_containment_relations
                            .iter()
                            .filter(|relation| {
                                relation.parent_entity_id
                                    == second_level_child_plan_containment.child_entity_id
                                    && relation.parent_kind == PrimaryContainmentEndpointKind::Plan
                                    && relation.child_kind == PrimaryContainmentEndpointKind::Task
                            })
                            .collect::<Vec<_>>();
                        second_level_task_relations.sort_by_key(|relation| relation.relation_id);
                        for task_containment in second_level_task_relations {
                            let referenced_task =
                                tasks_by_id
                                    .get(&task_containment.child_entity_id)
                                    .ok_or_else(|| {
                                        WorkVcsError::TaskInvalid(format!(
                                            "structural reference {} target plan {} child plan {} second-level child plan {} contains missing task {}",
                                            reference.relation_id,
                                            reference.target_entity_id,
                                            child_plan_containment.child_entity_id,
                                            second_level_child_plan_containment.child_entity_id,
                                            task_containment.child_entity_id
                                        ))
                                    })?;
                            push_task_resource_requirement_context_items(
                                items,
                                referenced_task,
                                TaskResourceRequirementContext::StructuralReferenceTwoLevelNestedPlanTarget {
                                    relation_id: reference.relation_id,
                                    referrer_entity_id: focused_entity_id,
                                    referrer_kind,
                                    referenced_plan_entity_id: reference.target_entity_id,
                                    first_nested_plan_entity_id: child_plan_containment.child_entity_id,
                                    second_nested_plan_entity_id: second_level_child_plan_containment.child_entity_id,
                                    referenced_plan_containment_relation_id: child_plan_containment.relation_id,
                                    nested_plan_containment_relation_id: second_level_child_plan_containment.relation_id,
                                    plan_task_containment_relation_id: task_containment.relation_id,
                                },
                                profile,
                                resource_requirement_indexes,
                            )?;
                        }
                    }
                }
            }
            StructuralReferenceEndpointKind::Goal => {}
        }
    }
    Ok(())
}

#[derive(Clone, Copy)]
enum TaskResourceRequirementContext {
    SamePlan {
        parent_plan_id: EntityId,
    },
    SameGoal {
        parent_goal_id: EntityId,
        focused_plan_id: EntityId,
        peer_plan_id: EntityId,
    },
    StructuralReference {
        relation_id: RelationId,
        referrer_entity_id: EntityId,
        referrer_kind: StructuralReferenceEndpointKind,
    },
    StructuralReferencePlanTarget {
        relation_id: RelationId,
        referrer_entity_id: EntityId,
        referrer_kind: StructuralReferenceEndpointKind,
        referenced_plan_entity_id: EntityId,
        containment_relation_id: RelationId,
    },
    StructuralReferenceNestedPlanTarget {
        relation_id: RelationId,
        referrer_entity_id: EntityId,
        referrer_kind: StructuralReferenceEndpointKind,
        referenced_plan_entity_id: EntityId,
        nested_plan_entity_id: EntityId,
        plan_plan_containment_relation_id: RelationId,
        plan_task_containment_relation_id: RelationId,
    },
    StructuralReferenceTwoLevelNestedPlanTarget {
        relation_id: RelationId,
        referrer_entity_id: EntityId,
        referrer_kind: StructuralReferenceEndpointKind,
        referenced_plan_entity_id: EntityId,
        first_nested_plan_entity_id: EntityId,
        second_nested_plan_entity_id: EntityId,
        referenced_plan_containment_relation_id: RelationId,
        nested_plan_containment_relation_id: RelationId,
        plan_task_containment_relation_id: RelationId,
    },
}

impl TaskResourceRequirementContext {
    fn summary_prefix(self, peer_task_id: EntityId) -> String {
        match self {
            Self::SamePlan { parent_plan_id } => {
                format!("same_plan_peer_task={peer_task_id} parent_plan={parent_plan_id}")
            }
            Self::SameGoal {
                parent_goal_id,
                focused_plan_id,
                peer_plan_id,
            } => format!(
                "same_goal_peer_task={peer_task_id} parent_goal={parent_goal_id} focused_plan={focused_plan_id} peer_plan={peer_plan_id}"
            ),
            Self::StructuralReference {
                relation_id,
                referrer_entity_id,
                referrer_kind,
            } => format!(
                "structural_reference_task={peer_task_id} referrer={referrer_entity_id} referrer_kind={referrer_kind} relation_id={relation_id}"
            ),
            Self::StructuralReferencePlanTarget {
                relation_id,
                referrer_entity_id,
                referrer_kind,
                referenced_plan_entity_id,
                containment_relation_id,
            } => format!(
                "structural_reference_plan_task={peer_task_id} referrer={referrer_entity_id} referrer_kind={referrer_kind} referenced_plan={referenced_plan_entity_id} relation_id={relation_id} containment_relation_id={containment_relation_id}"
            ),
            Self::StructuralReferenceNestedPlanTarget {
                relation_id,
                referrer_entity_id,
                referrer_kind,
                referenced_plan_entity_id,
                nested_plan_entity_id,
                plan_plan_containment_relation_id,
                plan_task_containment_relation_id,
            } => format!(
                "structural_reference_nested_plan_task={peer_task_id} referrer={referrer_entity_id} referrer_kind={referrer_kind} referenced_plan={referenced_plan_entity_id} nested_plan={nested_plan_entity_id} relation_id={relation_id} plan_plan_containment_relation_id={plan_plan_containment_relation_id} plan_task_containment_relation_id={plan_task_containment_relation_id}"
            ),
            Self::StructuralReferenceTwoLevelNestedPlanTarget {
                relation_id,
                referrer_entity_id,
                referrer_kind,
                referenced_plan_entity_id,
                first_nested_plan_entity_id,
                second_nested_plan_entity_id,
                referenced_plan_containment_relation_id,
                nested_plan_containment_relation_id,
                plan_task_containment_relation_id,
            } => format!(
                "structural_reference_two_level_nested_plan_task={peer_task_id} referrer={referrer_entity_id} referrer_kind={referrer_kind} referenced_plan={referenced_plan_entity_id} first_nested_plan={first_nested_plan_entity_id} second_nested_plan={second_nested_plan_entity_id} relation_id={relation_id} referenced_plan_containment_relation_id={referenced_plan_containment_relation_id} nested_plan_containment_relation_id={nested_plan_containment_relation_id} plan_task_containment_relation_id={plan_task_containment_relation_id}"
            ),
        }
    }

    fn relevance_marker(self) -> &'static str {
        match self {
            Self::SamePlan { .. } | Self::SameGoal { .. } => "peer_runnable=true",
            Self::StructuralReference { .. } => "reference_direct=true",
            Self::StructuralReferencePlanTarget { .. } => {
                "reference_direct=true referenced_plan_direct_child=true"
            }
            Self::StructuralReferenceNestedPlanTarget { .. } => {
                "reference_direct=true referenced_plan_one_level_child=true"
            }
            Self::StructuralReferenceTwoLevelNestedPlanTarget { .. } => {
                "reference_direct=true referenced_plan_two_level_child=true"
            }
        }
    }
}

struct ResourceRequirementContextIndexes<'a> {
    criteria_by_id: &'a BTreeMap<EntityId, &'a ContextAcceptanceCriterionSnapshot>,
    requirements_by_id: &'a BTreeMap<EntityId, &'a VerificationRequirementSnapshot>,
    resource_backed_verifications: &'a BTreeMap<EntityId, Vec<&'a VerificationSnapshot>>,
}

fn push_task_resource_requirement_context_items(
    items: &mut Vec<ContextItem>,
    peer_task: &TaskSnapshot,
    task_context: TaskResourceRequirementContext,
    profile: ContextProfile,
    resource_requirement_indexes: &ResourceRequirementContextIndexes<'_>,
) -> Result<()> {
    for criterion_ref in &peer_task.state.acceptance_criteria {
        let criterion = resource_requirement_indexes
            .criteria_by_id
            .get(&criterion_ref.acceptance_criterion_entity_id)
            .ok_or_else(|| {
                WorkVcsError::TaskInvalid(format!(
                    "task {} references missing acceptance criterion {}",
                    peer_task.task_entity_id, criterion_ref.acceptance_criterion_entity_id
                ))
            })?;
        if criterion.snapshot.task_entity_id != peer_task.task_entity_id
            || criterion.snapshot.local_key != criterion_ref.local_key
        {
            return Err(WorkVcsError::TaskInvalid(format!(
                "task {} acceptance criterion reference {} does not match stored identity",
                peer_task.task_entity_id, criterion_ref.local_key
            )));
        }

        for requirement_ref in &criterion.snapshot.state.verification_requirements {
            let requirement = resource_requirement_indexes
                .requirements_by_id
                .get(&requirement_ref.verification_requirement_entity_id)
                .ok_or_else(|| {
                    WorkVcsError::TaskInvalid(format!(
                        "acceptance criterion {} references missing verification requirement {}",
                        criterion.snapshot.acceptance_criterion_entity_id,
                        requirement_ref.verification_requirement_entity_id
                    ))
                })?;
            if requirement.acceptance_criterion_entity_id
                != criterion.snapshot.acceptance_criterion_entity_id
                || requirement.local_key != requirement_ref.local_key
            {
                return Err(WorkVcsError::TaskInvalid(format!(
                    "acceptance criterion {} verification requirement reference {} does not match stored identity",
                    criterion.snapshot.acceptance_criterion_entity_id, requirement_ref.local_key
                )));
            }

            let Some(verifications) = resource_requirement_indexes
                .resource_backed_verifications
                .get(&requirement.verification_requirement_entity_id)
            else {
                continue;
            };
            push_context_item(
                items,
                profile,
                ContextPriority::P2,
                ContextItemCategory::VerificationRequirement,
                ContextItemSubject::VerificationRequirement {
                    verification_requirement_entity_id: requirement
                        .verification_requirement_entity_id,
                },
                format!(
                    "{} {} criterion={} local_key={}: {} {}",
                    task_context.summary_prefix(peer_task.task_entity_id),
                    task_context.relevance_marker(),
                    requirement.acceptance_criterion_entity_id,
                    requirement.local_key,
                    requirement.state.statement,
                    verification_requirement_resource_basis_context_summary(verifications)
                ),
            );
        }
    }
    Ok(())
}

fn direct_goal_parent_for_plan(
    plan_entity_id: EntityId,
    primary_parent_by_child: &BTreeMap<EntityId, &PrimaryContainmentSnapshot>,
) -> Result<Option<EntityId>> {
    let Some(parent) = primary_parent_by_child.get(&plan_entity_id) else {
        return Ok(None);
    };
    if parent.child_kind != PrimaryContainmentEndpointKind::Plan {
        return Err(WorkVcsError::RelationInvalid(format!(
            "primary containment relation {} child kind {} does not match plan {}",
            parent.relation_id, parent.child_kind, plan_entity_id
        )));
    }
    if parent.parent_kind == PrimaryContainmentEndpointKind::Goal {
        Ok(Some(parent.parent_entity_id))
    } else {
        Ok(None)
    }
}

fn resource_backed_verifications_by_requirement(
    verifications: &[VerificationSnapshot],
) -> BTreeMap<EntityId, Vec<&VerificationSnapshot>> {
    let mut by_requirement = BTreeMap::<EntityId, Vec<&VerificationSnapshot>>::new();
    for verification in verifications {
        if verification.state.resource_basis.is_empty() {
            continue;
        }
        if let VerificationTarget::VerificationRequirement(requirement_id) = verification.target {
            by_requirement
                .entry(requirement_id)
                .or_default()
                .push(verification);
        }
    }
    for verifications in by_requirement.values_mut() {
        verifications.sort_by_key(|verification| verification.verification_entity_id);
    }
    by_requirement
}

fn blocked_dependency_resource_basis_context_summary(
    dependency_task: &TaskSnapshot,
    criteria_by_id: &BTreeMap<EntityId, &ContextAcceptanceCriterionSnapshot>,
    requirements_by_id: &BTreeMap<EntityId, &VerificationRequirementSnapshot>,
    resource_backed_verifications: &BTreeMap<EntityId, Vec<&VerificationSnapshot>>,
) -> Result<Option<String>> {
    let mut resource_backed_requirements = 0usize;
    let mut selected_summary = None;

    for criterion_ref in &dependency_task.state.acceptance_criteria {
        let criterion = criteria_by_id
            .get(&criterion_ref.acceptance_criterion_entity_id)
            .ok_or_else(|| {
                WorkVcsError::TaskInvalid(format!(
                    "task {} references missing acceptance criterion {}",
                    dependency_task.task_entity_id, criterion_ref.acceptance_criterion_entity_id
                ))
            })?;
        if criterion.snapshot.task_entity_id != dependency_task.task_entity_id
            || criterion.snapshot.local_key != criterion_ref.local_key
        {
            return Err(WorkVcsError::TaskInvalid(format!(
                "task {} acceptance criterion reference {} does not match stored identity",
                dependency_task.task_entity_id, criterion_ref.local_key
            )));
        }

        for requirement_ref in &criterion.snapshot.state.verification_requirements {
            let requirement = requirements_by_id
                .get(&requirement_ref.verification_requirement_entity_id)
                .ok_or_else(|| {
                    WorkVcsError::TaskInvalid(format!(
                        "acceptance criterion {} references missing verification requirement {}",
                        criterion.snapshot.acceptance_criterion_entity_id,
                        requirement_ref.verification_requirement_entity_id
                    ))
                })?;
            if requirement.acceptance_criterion_entity_id
                != criterion.snapshot.acceptance_criterion_entity_id
                || requirement.local_key != requirement_ref.local_key
            {
                return Err(WorkVcsError::TaskInvalid(format!(
                    "acceptance criterion {} verification requirement reference {} does not match stored identity",
                    criterion.snapshot.acceptance_criterion_entity_id, requirement_ref.local_key
                )));
            }

            let Some(verifications) =
                resource_backed_verifications.get(&requirement.verification_requirement_entity_id)
            else {
                continue;
            };
            resource_backed_requirements += 1;
            if selected_summary.is_none() {
                selected_summary = Some((
                    criterion.snapshot.acceptance_criterion_entity_id,
                    requirement.verification_requirement_entity_id,
                    requirement.local_key.clone(),
                    verification_requirement_resource_basis_context_summary(verifications),
                ));
            }
        }
    }

    let Some((criterion_id, requirement_id, requirement_local_key, resource_summary)) =
        selected_summary
    else {
        return Ok(None);
    };
    Ok(Some(format!(
        "dependency_resource_requirements={} dependency_acceptance_criterion={} dependency_verification_requirement={} dependency_vr_local_key={} {}",
        resource_backed_requirements,
        criterion_id,
        requirement_id,
        requirement_local_key,
        resource_summary
    )))
}

fn verification_requirement_resource_basis_context_summary(
    verifications: &[&VerificationSnapshot],
) -> String {
    let resource_basis_count = verifications
        .iter()
        .map(|verification| verification.state.resource_basis.len())
        .sum::<usize>();
    let selected_verification = verifications
        .iter()
        .find(|verification| {
            !verification.state.resource_basis.is_empty()
                && verification
                    .state
                    .resource_basis
                    .iter()
                    .all(|basis| basis.baseline_observation_id.is_some())
        })
        .or_else(|| {
            verifications
                .iter()
                .find(|verification| !verification.state.resource_basis.is_empty())
        });
    let Some(selected_verification) = selected_verification else {
        return "resource_basis=0".to_owned();
    };
    let Some(selected_basis) = selected_verification.state.resource_basis.first() else {
        return "resource_basis=0".to_owned();
    };
    let baseline_observation_id = selected_basis
        .baseline_observation_id
        .map(|observation_id| observation_id.to_string())
        .unwrap_or_else(|| "none".to_owned());
    let refresh_hint = if selected_verification
        .state
        .resource_basis
        .iter()
        .all(|basis| basis.baseline_observation_id.is_some())
    {
        format!(
            "\"verification cache-refresh --verification {} --resource-content-from-basis\"",
            selected_verification.verification_entity_id
        )
    } else {
        "unavailable_missing_baseline_observation".to_owned()
    };
    format!(
        "resource_basis={} verification_id={} resource_id={} adapter={}@{} scope={}@{} baseline_observation_id={} refresh_hint={}",
        resource_basis_count,
        selected_verification.verification_entity_id,
        selected_basis.resource_id,
        selected_basis.adapter_kind,
        selected_basis.adapter_schema_version,
        selected_basis.scope_kind,
        selected_basis.scope_schema_version,
        baseline_observation_id,
        refresh_hint
    )
}

#[derive(Default)]
struct PathScopeSelectors {
    paths: BTreeSet<String>,
    prefixes: BTreeSet<String>,
}

impl PathScopeSelectors {
    fn from_scope(scope: &CanonicalValue) -> Self {
        let mut selectors = Self::default();
        collect_path_scope_selectors(scope, &mut selectors);
        selectors
    }

    fn is_empty(&self) -> bool {
        self.paths.is_empty() && self.prefixes.is_empty()
    }

    fn overlaps(&self, other: &Self) -> bool {
        self.paths.iter().any(|path| {
            other.paths.contains(path)
                || other
                    .prefixes
                    .iter()
                    .any(|prefix| path_is_within_prefix(path, prefix))
        }) || other.paths.iter().any(|path| {
            self.prefixes
                .iter()
                .any(|prefix| path_is_within_prefix(path, prefix))
        }) || self.prefixes.iter().any(|left| {
            other
                .prefixes
                .iter()
                .any(|right| prefixes_overlap(left, right))
        })
    }
}

fn knowledge_matches_context_scope(
    knowledge_scope: &CanonicalValue,
    context_scope: Option<&CanonicalValue>,
) -> bool {
    let knowledge_selectors = PathScopeSelectors::from_scope(knowledge_scope);
    if knowledge_selectors.is_empty() {
        return true;
    }

    let Some(context_scope) = context_scope else {
        return true;
    };
    let context_selectors = PathScopeSelectors::from_scope(context_scope);
    if context_selectors.is_empty() {
        return true;
    }

    knowledge_selectors.overlaps(&context_selectors)
}

fn collect_path_scope_selectors(scope: &CanonicalValue, selectors: &mut PathScopeSelectors) {
    let CanonicalValue::Object(entries) = scope else {
        return;
    };

    for (key, value) in entries {
        match key.as_str() {
            "path" => collect_string_selector(value, &mut selectors.paths),
            "paths" => collect_string_selectors(value, &mut selectors.paths),
            "path_prefix" => collect_string_selector(value, &mut selectors.prefixes),
            "path_prefixes" => collect_string_selectors(value, &mut selectors.prefixes),
            "scope_payload" => collect_path_scope_selectors(value, selectors),
            _ => {}
        }
    }
}

fn collect_string_selector(value: &CanonicalValue, selectors: &mut BTreeSet<String>) {
    if let CanonicalValue::String(text) = value
        && let Some(selector) = normalize_path_scope_selector(text)
    {
        selectors.insert(selector);
    }
}

fn collect_string_selectors(value: &CanonicalValue, selectors: &mut BTreeSet<String>) {
    if let CanonicalValue::Array(values) = value {
        for value in values {
            collect_string_selector(value, selectors);
        }
    }
}

fn path_is_within_prefix(path: &str, prefix: &str) -> bool {
    if path == prefix {
        return true;
    }
    if prefix.ends_with('/') {
        return path.starts_with(prefix);
    }
    path.strip_prefix(prefix)
        .is_some_and(|suffix| suffix.starts_with('/'))
}

fn prefixes_overlap(left: &str, right: &str) -> bool {
    path_is_within_prefix(left, right) || path_is_within_prefix(right, left)
}

fn normalize_path_scope_selector(text: &str) -> Option<String> {
    if text.is_empty() {
        return None;
    }

    let absolute = text.starts_with('/');
    let mut segments = Vec::new();
    for segment in text.split('/') {
        match segment {
            "" | "." => {}
            ".." => match segments.last() {
                Some(last) if *last != ".." => {
                    segments.pop();
                }
                _ if !absolute => segments.push(segment),
                _ => {}
            },
            _ => segments.push(segment),
        }
    }

    if absolute {
        if segments.is_empty() {
            Some("/".to_owned())
        } else {
            Some(format!("/{}", segments.join("/")))
        }
    } else if segments.is_empty() {
        Some(".".to_owned())
    } else {
        Some(segments.join("/"))
    }
}

fn context_packet_value(packet: &ContextPacket) -> Result<CanonicalValue> {
    CanonicalValue::object(vec![
        string_field("context_packet_format", CONTEXT_PACKET_FORMAT),
        integer_field(
            "context_packet_format_version",
            CONTEXT_PACKET_FORMAT_VERSION,
        )?,
        (
            "envelope".to_owned(),
            context_packet_envelope_value(&packet.envelope)?,
        ),
        string_field("profile", packet.profile.as_str()),
        (
            "budget_items".to_owned(),
            packet
                .budget_items
                .map(|value| usize_value(value.get()))
                .transpose()?
                .unwrap_or(CanonicalValue::Null),
        ),
        (
            "scope".to_owned(),
            packet.scope.clone().unwrap_or(CanonicalValue::Null),
        ),
        integer_field(
            "available_items",
            usize_to_i64("context packet available_items", packet.available_items)?,
        )?,
        (
            "items".to_owned(),
            CanonicalValue::Array(
                packet
                    .items
                    .iter()
                    .map(context_item_value)
                    .collect::<Result<Vec<_>>>()?,
            ),
        ),
        (
            "omission_summary".to_owned(),
            context_omission_summary_value(&packet.omission_summary)?,
        ),
    ])
}

fn context_packet_envelope_value(envelope: &ContextPacketEnvelope) -> Result<CanonicalValue> {
    CanonicalValue::object(vec![
        string_field("session_id", envelope.session_id.to_string()),
        string_field(
            "lifecycle_state",
            session_lifecycle_state_label(envelope.lifecycle_state),
        ),
        string_field("workspace_id", envelope.workspace_id.to_string()),
        string_field("branch_id", envelope.branch_id.to_string()),
        string_field("branch_name", envelope.branch_name.clone()),
        string_field("head_commit_id", envelope.head_commit_id.to_string()),
        string_field("state_digest", envelope.state_digest.to_string()),
        integer_field("started_at_us", envelope.started_at_us)?,
        (
            "last_activity_at_us".to_owned(),
            envelope
                .last_activity_at_us
                .map(CanonicalValue::safe_integer)
                .transpose()?
                .unwrap_or(CanonicalValue::Null),
        ),
        (
            "focus_entity_id".to_owned(),
            envelope
                .focus_entity_id
                .map(|focus_entity_id| CanonicalValue::String(focus_entity_id.to_string()))
                .unwrap_or(CanonicalValue::Null),
        ),
    ])
}

fn context_item_value(item: &ContextItem) -> Result<CanonicalValue> {
    CanonicalValue::object(vec![
        string_field("priority", item.priority.as_str()),
        string_field("category", item.category.as_str()),
        string_field("subject", item.subject.as_ref_string()),
        string_field("item_key", item.item_key.clone()),
        string_field("summary", item.summary.clone()),
    ])
}

fn context_omission_summary_value(summary: &ContextOmissionSummary) -> Result<CanonicalValue> {
    CanonicalValue::object(vec![
        integer_field(
            "total",
            usize_to_i64("context packet omitted_items", summary.total)?,
        )?,
        (
            "by_priority".to_owned(),
            CanonicalValue::Array(
                summary
                    .by_priority
                    .iter()
                    .map(|bucket| {
                        CanonicalValue::object(vec![
                            string_field("priority", bucket.priority.as_str()),
                            integer_field(
                                "omitted",
                                usize_to_i64("context packet omitted priority", bucket.omitted)?,
                            )?,
                        ])
                    })
                    .collect::<Result<Vec<_>>>()?,
            ),
        ),
        (
            "by_category".to_owned(),
            CanonicalValue::Array(
                summary
                    .by_category
                    .iter()
                    .map(|bucket| {
                        CanonicalValue::object(vec![
                            string_field("category", bucket.category.as_str()),
                            integer_field(
                                "omitted",
                                usize_to_i64("context packet omitted category", bucket.omitted)?,
                            )?,
                        ])
                    })
                    .collect::<Result<Vec<_>>>()?,
            ),
        ),
    ])
}

fn session_lifecycle_state_label(lifecycle_state: SessionLifecycleState) -> &'static str {
    match lifecycle_state {
        SessionLifecycleState::Active => "active",
        SessionLifecycleState::PotentiallyStale => "potentially_stale",
        SessionLifecycleState::Ended => "ended",
    }
}

fn parse_context_profile(label: &str, value: &str) -> Result<ContextProfile> {
    match value {
        "brief" => Ok(ContextProfile::Brief),
        "normal" => Ok(ContextProfile::Normal),
        "full" => Ok(ContextProfile::Full),
        other => Err(WorkVcsError::StorageFailure(format!(
            "{label} has unsupported context profile {other:?}"
        ))),
    }
}

fn string_field(name: &str, value: impl Into<String>) -> (String, CanonicalValue) {
    (name.to_owned(), CanonicalValue::String(value.into()))
}

fn integer_field(name: &str, value: i64) -> Result<(String, CanonicalValue)> {
    Ok((name.to_owned(), CanonicalValue::safe_integer(value)?))
}

fn usize_value(value: usize) -> Result<CanonicalValue> {
    CanonicalValue::safe_integer(usize_to_i64("context packet usize value", value)?)
}

fn usize_to_i64(label: &str, value: usize) -> Result<i64> {
    i64::try_from(value)
        .map_err(|_| WorkVcsError::QueryInvalid(format!("{label} does not fit i64")))
}

fn usize_from_i64(label: &str, value: i64) -> Result<usize> {
    if value < 0 {
        return Err(WorkVcsError::StorageFailure(format!(
            "{label} must be non-negative"
        )));
    }
    usize::try_from(value)
        .map_err(|_| WorkVcsError::StorageFailure(format!("{label} does not fit usize")))
}

fn non_zero_usize_from_i64(label: &str, value: i64) -> Result<NonZeroUsize> {
    let value = usize_from_i64(label, value)?;
    NonZeroUsize::new(value)
        .ok_or_else(|| WorkVcsError::StorageFailure(format!("{label} must be greater than zero")))
}

fn canonical_object_json_text(value: &CanonicalValue) -> Result<String> {
    if !matches!(value, CanonicalValue::Object(_)) {
        return Err(WorkVcsError::QueryInvalid(
            "context packet scope must be an object".to_owned(),
        ));
    }
    canonical_json_text(value)
}

fn parse_canonical_object_json(label: &str, input: &str) -> Result<CanonicalValue> {
    let value = parse_canonical_json(input.as_bytes())?;
    if !matches!(value, CanonicalValue::Object(_)) {
        return Err(WorkVcsError::StorageFailure(format!(
            "{label} must be a canonical JSON object"
        )));
    }
    let reencoded = canonical_json_text(&value)?;
    if reencoded != input {
        return Err(WorkVcsError::StorageFailure(format!(
            "{label} is not canonical JSON"
        )));
    }
    Ok(value)
}

fn canonical_json_text(value: &CanonicalValue) -> Result<String> {
    String::from_utf8(canonical_bytes(value)?).map_err(|error| {
        WorkVcsError::CanonicalEncodingInvalid(format!("canonical JSON was not UTF-8: {error}"))
    })
}

fn decode_context_packet_id(column: &str, bytes: Vec<u8>) -> Result<ContextPacketId> {
    ContextPacketId::from_bytes(fixed_bytes(column, bytes)?).map_err(|error| {
        WorkVcsError::StorageFailure(format!("{column} is not a UUIDv7 value: {error}"))
    })
}

fn decode_session_id(column: &str, bytes: Vec<u8>) -> Result<SessionId> {
    SessionId::from_bytes(fixed_bytes(column, bytes)?).map_err(|error| {
        WorkVcsError::StorageFailure(format!("{column} is not a UUIDv7 value: {error}"))
    })
}

fn decode_workspace_id(column: &str, bytes: Vec<u8>) -> Result<WorkspaceId> {
    WorkspaceId::from_bytes(fixed_bytes(column, bytes)?).map_err(|error| {
        WorkVcsError::StorageFailure(format!("{column} is not a UUIDv7 value: {error}"))
    })
}

fn decode_branch_id(column: &str, bytes: Vec<u8>) -> Result<BranchId> {
    BranchId::from_bytes(fixed_bytes(column, bytes)?).map_err(|error| {
        WorkVcsError::StorageFailure(format!("{column} is not a UUIDv7 value: {error}"))
    })
}

fn decode_commit_id(column: &str, bytes: Vec<u8>) -> Result<CommitId> {
    CommitId::from_bytes(fixed_bytes(column, bytes)?).map_err(|error| {
        WorkVcsError::StorageFailure(format!("{column} is not a UUIDv7 value: {error}"))
    })
}

fn decode_digest(column: &str, bytes: Vec<u8>) -> Result<Digest> {
    Ok(Digest::from_bytes(fixed_bytes(column, bytes)?))
}

fn fixed_bytes<const N: usize>(column: &str, bytes: Vec<u8>) -> Result<[u8; N]> {
    let len = bytes.len();
    bytes.try_into().map_err(|_| {
        WorkVcsError::StorageFailure(format!("{column} must be {N} bytes, found {len}"))
    })
}

fn primary_parent_by_child(
    relations: &[PrimaryContainmentSnapshot],
) -> Result<BTreeMap<EntityId, &PrimaryContainmentSnapshot>> {
    let mut parent_by_child = BTreeMap::new();
    for relation in relations {
        if relation.parent_entity_id == relation.child_entity_id {
            return Err(WorkVcsError::RelationInvalid(format!(
                "current primary containment relation {} is a self edge",
                relation.relation_id
            )));
        }
        if let Some(existing) = parent_by_child.insert(relation.child_entity_id, relation) {
            return Err(WorkVcsError::RelationInvalid(format!(
                "entity {} has multiple current primary containment parents: {} and {}",
                relation.child_entity_id, existing.parent_entity_id, relation.parent_entity_id
            )));
        }
    }
    Ok(parent_by_child)
}

fn goal_plan_path_summary(
    task_entity_id: EntityId,
    primary_parent_by_child: &BTreeMap<EntityId, &PrimaryContainmentSnapshot>,
    plans_by_id: &BTreeMap<EntityId, &PlanSnapshot>,
    goals_by_id: &BTreeMap<EntityId, &GoalSnapshot>,
) -> Result<Option<String>> {
    let mut current_entity_id = task_entity_id;
    let mut current_kind = PrimaryContainmentEndpointKind::Task;
    let mut seen = BTreeSet::new();
    let mut path_segments = Vec::new();

    while let Some(relation) = primary_parent_by_child.get(&current_entity_id) {
        if !seen.insert(current_entity_id) {
            return Err(WorkVcsError::RelationInvalid(format!(
                "current primary containment graph contains a cycle at entity {current_entity_id}"
            )));
        }
        if relation.child_kind != current_kind {
            return Err(WorkVcsError::RelationInvalid(format!(
                "primary containment relation {} child kind {} does not match resolved path kind {} for entity {}",
                relation.relation_id, relation.child_kind, current_kind, current_entity_id
            )));
        }

        match relation.parent_kind {
            PrimaryContainmentEndpointKind::Goal => {
                let goal = goals_by_id.get(&relation.parent_entity_id).ok_or_else(|| {
                    WorkVcsError::GoalInvalid(format!(
                        "primary containment relation {} references missing goal {}",
                        relation.relation_id, relation.parent_entity_id
                    ))
                })?;
                path_segments.push(format!(
                    "goal:{} status={} relation={}: {}",
                    goal.goal_entity_id,
                    goal.state.status,
                    relation.relation_id,
                    goal.state.description
                ));
            }
            PrimaryContainmentEndpointKind::Plan => {
                let plan = plans_by_id.get(&relation.parent_entity_id).ok_or_else(|| {
                    WorkVcsError::PlanInvalid(format!(
                        "primary containment relation {} references missing plan {}",
                        relation.relation_id, relation.parent_entity_id
                    ))
                })?;
                path_segments.push(format!(
                    "plan:{} status={} relation={}: {}",
                    plan.plan_entity_id,
                    plan.state.status,
                    relation.relation_id,
                    plan.state.description
                ));
            }
            PrimaryContainmentEndpointKind::Task => {}
        }

        current_entity_id = relation.parent_entity_id;
        current_kind = relation.parent_kind;
    }

    if path_segments.is_empty() {
        return Ok(None);
    }

    path_segments.reverse();
    Ok(Some(format!(
        "goal_plan_path task={} path={} > task:{}",
        task_entity_id,
        path_segments.join(" > "),
        task_entity_id
    )))
}

fn push_context_item(
    items: &mut Vec<ContextItem>,
    profile: ContextProfile,
    priority: ContextPriority,
    category: ContextItemCategory,
    subject: ContextItemSubject,
    summary: impl Into<String>,
) {
    if profile_allows_category(profile, category) {
        items.push(ContextItem::new(priority, category, subject, summary));
    }
}

fn profile_allows_category(profile: ContextProfile, category: ContextItemCategory) -> bool {
    match profile {
        ContextProfile::Brief => matches!(
            category,
            ContextItemCategory::SessionAnchor
                | ContextItemCategory::BranchOverview
                | ContextItemCategory::CurrentTask
                | ContextItemCategory::GoalPlanPath
                | ContextItemCategory::AcceptanceCriterion
                | ContextItemCategory::VerificationRequirement
                | ContextItemCategory::TaskReadiness
                | ContextItemCategory::BlockedDependency
                | ContextItemCategory::TransitionRationale
                | ContextItemCategory::ActiveDecision
                | ContextItemCategory::ActiveAssumption
                | ContextItemCategory::FailedAttempt
        ),
        ContextProfile::Normal => !matches!(category, ContextItemCategory::OlderProvenance),
        ContextProfile::Full => true,
    }
}

fn record_priority_and_category(
    kind: RecordKind,
    status: RecordStatus,
) -> (ContextPriority, ContextItemCategory) {
    match (kind, status) {
        (RecordKind::Decision, RecordStatus::Active) => {
            (ContextPriority::P4, ContextItemCategory::ActiveDecision)
        }
        (RecordKind::Assumption, RecordStatus::Unverified | RecordStatus::Validated) => {
            (ContextPriority::P4, ContextItemCategory::ActiveAssumption)
        }
        (RecordKind::Attempt, RecordStatus::Failed) => {
            (ContextPriority::P5, ContextItemCategory::FailedAttempt)
        }
        (RecordKind::Attempt, _) => (ContextPriority::P5, ContextItemCategory::Attempt),
        (RecordKind::Finding, RecordStatus::Active) => {
            (ContextPriority::P6, ContextItemCategory::Finding)
        }
        (RecordKind::Finding, _) => (ContextPriority::P9, ContextItemCategory::OlderProvenance),
        (RecordKind::Handoff, _) => (ContextPriority::P8, ContextItemCategory::RelevantHandoff),
        _ => (ContextPriority::P9, ContextItemCategory::OlderProvenance),
    }
}

fn record_context_summary(
    record: &RecordSnapshot,
    record_relations: &[RecordRelationSnapshot],
) -> Result<String> {
    if record.state.kind != RecordKind::Attempt {
        return Ok(format!(
            "{} {}: {}",
            record.state.kind, record.state.status, record.state.statement
        ));
    }

    let scope_json = String::from_utf8(canonical_bytes(&record.state.scope)?).map_err(|error| {
        WorkVcsError::RecordInvalid(format!("attempt scope encode produced non-UTF-8: {error}"))
    })?;
    let outgoing_record_relations = record_relations
        .iter()
        .filter(|relation| relation.source_record_entity_id == record.record_entity_id)
        .count();
    let incoming_record_relations = record_relations
        .iter()
        .filter(|relation| relation.target_record_entity_id == record.record_entity_id)
        .count();
    Ok(format!(
        "attempt detail status={} terminal={} record={} version={} state_digest={} scope_json={} record_relations_out={} record_relations_in={} record_relation_types={} statement={}",
        record.state.status,
        is_terminal_attempt_status(record.state.status),
        record.record_entity_id,
        record.record_entity_version_id,
        record.state_digest,
        scope_json,
        outgoing_record_relations,
        incoming_record_relations,
        record_relation_type_summary(record.record_entity_id, record_relations),
        record.state.statement
    ))
}

fn transition_rationale_context_summary(rationale: &ContextTransitionRationaleSnapshot) -> String {
    let origin_session = rationale
        .origin_session_id
        .map(|session_id| session_id.to_string())
        .unwrap_or_else(|| "none".to_owned());
    format!(
        "transition rationale commit={} changeset={} commit_kind={} operation={} schema_version={} committed_at_us={} changeset_created_at_us={} rationale_digest={} rationale_size_bytes={} change_operations={} causal_anchors={} events={} origin_session={} rationale_json={}",
        rationale.commit_id,
        rationale.changeset_id,
        rationale.commit_kind,
        rationale.operation_type,
        rationale.operation_schema_version,
        rationale.committed_at_us,
        rationale.changeset_created_at_us,
        rationale.rationale_digest,
        rationale.rationale_size_bytes,
        rationale.change_operation_count,
        rationale.causal_anchor_count,
        rationale.event_count,
        origin_session,
        rationale.rationale_json
    )
}

fn is_terminal_attempt_status(status: RecordStatus) -> bool {
    matches!(
        status,
        RecordStatus::Succeeded | RecordStatus::Failed | RecordStatus::Inconclusive
    )
}

fn record_relation_type_summary(
    record_entity_id: EntityId,
    relations: &[RecordRelationSnapshot],
) -> String {
    let mut buckets = BTreeMap::<String, usize>::new();
    for relation in relations {
        if relation.source_record_entity_id == record_entity_id {
            *buckets
                .entry(format!("out:{}", relation.relation_type))
                .or_default() += 1;
        }
        if relation.target_record_entity_id == record_entity_id {
            *buckets
                .entry(format!("in:{}", relation.relation_type))
                .or_default() += 1;
        }
    }
    relation_bucket_summary(&buckets)
}

fn relation_bucket_summary(buckets: &BTreeMap<String, usize>) -> String {
    if buckets.is_empty() {
        return "none".to_owned();
    }
    buckets
        .iter()
        .map(|(bucket, count)| format!("{bucket}={count}"))
        .collect::<Vec<_>>()
        .join(",")
}

fn claim_coordination_summary(claim: &runnable::RunnableTaskClaimCoordination) -> String {
    match claim {
        runnable::RunnableTaskClaimCoordination::Unclaimed => "unclaimed".to_owned(),
        runnable::RunnableTaskClaimCoordination::ClaimedBySession { claim_id } => {
            format!("claimed_by_session:{claim_id}")
        }
        runnable::RunnableTaskClaimCoordination::ClaimedByOtherSession {
            claim_id,
            session_id,
        } => format!("claimed_by_other_session:{claim_id}:{session_id}"),
        runnable::RunnableTaskClaimCoordination::Shared {
            claim_ids,
            session_ids,
            claimed_by_session,
        } => format!(
            "shared:claims={}:sessions={}:claimed_by_session={claimed_by_session}",
            claim_ids.len(),
            session_ids.len()
        ),
    }
}

fn blocked_reason_summary(reasons: &[runnable::RunnableTaskBlockedReason]) -> String {
    if reasons.is_empty() {
        return "none".to_owned();
    }
    reasons
        .iter()
        .map(|reason| match reason {
            runnable::RunnableTaskBlockedReason::LifecycleIneligible => "lifecycle_ineligible",
            runnable::RunnableTaskBlockedReason::DependencyBlocked => "dependency_blocked",
            runnable::RunnableTaskBlockedReason::ClaimBlocked => "claim_blocked",
        })
        .collect::<Vec<_>>()
        .join(",")
}

fn entity_id_list_summary(entity_ids: &[EntityId]) -> String {
    if entity_ids.is_empty() {
        return "none".to_owned();
    }
    entity_ids
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(",")
}

fn why_relation_kind_summary(kind: WhyRelationKind) -> &'static str {
    match kind {
        WhyRelationKind::PrimaryContainment => "primary_containment",
        WhyRelationKind::StructuralReference => "structural_reference",
        WhyRelationKind::Verifies => "verifies",
        WhyRelationKind::EvidencedBy => "evidenced_by",
        WhyRelationKind::RecordContradicts => "record_contradicts",
        WhyRelationKind::RecordDerivedFrom => "record_derived_from",
        WhyRelationKind::RecordInvalidates => "record_invalidates",
        WhyRelationKind::RecordRelatedTo => "record_related_to",
        WhyRelationKind::RecordSupports => "record_supports",
        WhyRelationKind::RecordSupersedes => "record_supersedes",
        WhyRelationKind::RecordValidates => "record_validates",
        WhyRelationKind::TaskDependsOn => "task_depends_on",
        WhyRelationKind::TaskOrderedBefore => "task_ordered_before",
        WhyRelationKind::KnowledgeExposureDerivedFrom => "knowledge_exposure_derived_from",
        WhyRelationKind::KnowledgeSupersedes => "knowledge_supersedes",
        WhyRelationKind::PlanSupersedes => "plan_supersedes",
    }
}

fn summarize_omitted_items(omitted: &[ContextItem]) -> ContextOmissionSummary {
    let mut by_priority = BTreeMap::<ContextPriority, usize>::new();
    let mut by_category = BTreeMap::<ContextItemCategory, usize>::new();
    for item in omitted {
        *by_priority.entry(item.priority).or_default() += 1;
        *by_category.entry(item.category).or_default() += 1;
    }
    ContextOmissionSummary {
        total: omitted.len(),
        by_priority: by_priority
            .into_iter()
            .map(|(priority, omitted)| ContextOmissionBucket { priority, omitted })
            .collect(),
        by_category: by_category
            .into_iter()
            .map(|(category, omitted)| ContextOmissionCategory { category, omitted })
            .collect(),
    }
}

fn knowledge_exposure_relations_for_context(
    connection: &StoreConnection,
    branch: &BranchHead,
    knowledge: &KnowledgeListResult,
) -> Result<Vec<WhyRelationEdge>> {
    let knowledge_entity_ids = knowledge
        .knowledge
        .iter()
        .map(|knowledge| knowledge.knowledge_entity_id)
        .collect::<Vec<_>>();
    history::knowledge_exposure_relation_edges_for_entities(
        connection,
        branch.head_commit_id,
        &knowledge_entity_ids,
    )
}

fn context_candidate_is_current(
    candidate: &RunnableTaskCandidate,
    session: &SessionSnapshot,
) -> bool {
    if !candidate.task.state.status.is_terminal() {
        return true;
    }
    if session
        .focus
        .as_ref()
        .is_some_and(|focus| focus.focus_entity_id == candidate.task.task_entity_id)
    {
        return true;
    }
    matches!(
        candidate.claim_coordination,
        RunnableTaskClaimCoordination::ClaimedBySession { .. }
            | RunnableTaskClaimCoordination::Shared {
                claimed_by_session: true,
                ..
            }
    )
}

fn ensure_active_session(session: &SessionSnapshot) -> Result<()> {
    if session.lifecycle_state == SessionLifecycleState::Active {
        Ok(())
    } else {
        Err(WorkVcsError::SessionInvalid(format!(
            "session {} is not active",
            session.session_id
        )))
    }
}

fn require_active_workspace_id(session: &SessionSnapshot) -> Result<WorkspaceId> {
    session.active_workspace_id.ok_or_else(|| {
        WorkVcsError::SessionInvalid(format!(
            "active session {} has no active workspace",
            session.session_id
        ))
    })
}
