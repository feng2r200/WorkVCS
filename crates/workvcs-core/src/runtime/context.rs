use super::runnable::{self, RunnableTasksOptions, RunnableTasksProjection};
use super::session::{self, SessionLifecycleState, SessionSnapshot};
use crate::error::{Result, WorkVcsError};
use crate::history::{
    self, AcceptanceCriterionEffectiveStatus, AcceptanceCriterionSnapshot, BranchHead,
    KnowledgeListOptions, KnowledgeListResult, KnowledgeRelationListOptions,
    KnowledgeRelationListResult, KnowledgeStatus, RecordKind, RecordKnowledgeRelationListOptions,
    RecordKnowledgeRelationListResult, RecordListOptions, RecordListResult,
    RecordRelationListOptions, RecordRelationListResult, RecordStatus, TaskSnapshot,
    VerificationRequirementSnapshot, WhyQueryOptions, WhyQueryTarget, WhyRelationEdge,
    WhyRelationKind,
};
use crate::identity::{BranchId, CommitId, Digest, EntityId, RelationId, SessionId, WorkspaceId};
use crate::store::StoreConnection;
use std::collections::BTreeMap;
use std::num::NonZeroUsize;

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
}

impl ContextPacketOptions {
    pub fn new(session_id: SessionId) -> Self {
        Self {
            session_id,
            profile: ContextProfile::Normal,
            budget_items: None,
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

    pub fn session_id(&self) -> SessionId {
        self.session_id
    }

    pub fn profile(&self) -> ContextProfile {
        self.profile
    }

    pub fn budget_items(&self) -> Option<NonZeroUsize> {
        self.budget_items
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
    AcceptanceCriterion,
    VerificationRequirement,
    TaskReadiness,
    BlockedDependency,
    DirectCausalChain,
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
            Self::AcceptanceCriterion => "acceptance_criterion",
            Self::VerificationRequirement => "verification_requirement",
            Self::TaskReadiness => "task_readiness",
            Self::BlockedDependency => "blocked_dependency",
            Self::DirectCausalChain => "direct_causal_chain",
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
    pub available_items: usize,
    pub items: Vec<ContextItem>,
    pub omission_summary: ContextOmissionSummary,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContextOverview {
    pub session: SessionSnapshot,
    pub branch: BranchHead,
    pub runnable_tasks: RunnableTasksProjection,
    pub tasks: Vec<TaskSnapshot>,
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
    let mut items = collect_context_items(&overview, options.profile())?;
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
        available_items,
        items,
        omission_summary,
    })
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

fn collect_context_items(
    context: &ContextOverview,
    profile: ContextProfile,
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
    let tasks_by_id = context
        .tasks
        .iter()
        .map(|task| (task.task_entity_id, task))
        .collect::<BTreeMap<_, _>>();
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
                push_context_item(
                    &mut items,
                    profile,
                    ContextPriority::P1,
                    ContextItemCategory::VerificationRequirement,
                    ContextItemSubject::VerificationRequirement {
                        verification_requirement_entity_id: requirement
                            .verification_requirement_entity_id,
                    },
                    format!(
                        "verification requirement criterion={} local_key={}: {}",
                        requirement.acceptance_criterion_entity_id,
                        requirement.local_key,
                        requirement.state.statement
                    ),
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
            push_context_item(
                &mut items,
                profile,
                ContextPriority::P2,
                ContextItemCategory::BlockedDependency,
                ContextItemSubject::BlockedDependency {
                    task_entity_id: candidate.task.task_entity_id,
                    dependency_task_entity_id: *dependency_task_entity_id,
                },
                format!(
                    "blocked dependency task={} dependency={} dependency_status={} dependency_priority={}: {}",
                    candidate.task.task_entity_id,
                    dependency_task.task_entity_id,
                    dependency_task.state.status,
                    dependency_task.state.priority,
                    dependency_task.state.description
                ),
            );
        }
    }
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
            format!(
                "{} {}: {}",
                record.state.kind, record.state.status, record.state.statement
            ),
        );
    }
    for knowledge in &context.knowledge.knowledge {
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
                | ContextItemCategory::AcceptanceCriterion
                | ContextItemCategory::VerificationRequirement
                | ContextItemCategory::TaskReadiness
                | ContextItemCategory::BlockedDependency
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
        (RecordKind::Finding, _) => (ContextPriority::P6, ContextItemCategory::Finding),
        (RecordKind::Handoff, _) => (ContextPriority::P8, ContextItemCategory::RelevantHandoff),
        _ => (ContextPriority::P9, ContextItemCategory::OlderProvenance),
    }
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
        WhyRelationKind::KnowledgeExposureDerivedFrom => "knowledge_exposure_derived_from",
        WhyRelationKind::KnowledgeSupersedes => "knowledge_supersedes",
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
    let mut relations = Vec::new();
    for knowledge in &knowledge.knowledge {
        let why = history::explain_why(
            connection,
            &WhyQueryOptions::for_entity(
                WhyQueryTarget::commit(branch.head_commit_id),
                knowledge.knowledge_entity_id,
            ),
        )?;
        relations.extend(
            why.relation_edges
                .into_iter()
                .filter(|edge| edge.relation_kind == WhyRelationKind::KnowledgeExposureDerivedFrom),
        );
    }
    relations.sort_by(|left, right| {
        left.relation_kind
            .cmp(&right.relation_kind)
            .then_with(|| left.direction.cmp(&right.direction))
            .then_with(|| left.source.cmp(&right.source))
            .then_with(|| left.target.cmp(&right.target))
            .then_with(|| left.relation_id.cmp(&right.relation_id))
    });
    Ok(relations)
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
