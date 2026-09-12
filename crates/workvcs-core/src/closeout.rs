use crate::engine::Engine;
use crate::error::{ErrorCode, Result, WorkVcsError};
use crate::identity::{
    BranchId, ClaimId, CommitId, Digest, EntityId, EntityVersionId, EvidenceId, SessionDiffId,
    SessionId,
};
use crate::{
    AcceptanceCriterionSnapshot, EvidenceSnapshot, TaskSnapshot, VerificationRequirementSnapshot,
    VerificationSnapshot, VerificationTarget,
};
use crate::{
    AuthorizationReceiptListOptions, AuthorizationReceiptSnapshot, CanonicalValue,
    ClaimLifecycleState, ClaimMode, ClaimSnapshot, GoalSnapshot, PlanSnapshot,
    PrimaryContainmentEndpointKind, PrimaryContainmentSnapshot, RecordKind, RecordListOptions,
    RecordStatus, SessionLifecycleState, SessionListOptions, SessionSnapshot,
};
use serde::Serialize;
use std::collections::BTreeSet;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub const CLOSEOUT_INSPECT_DEFAULT_BUDGET: usize = 50;
pub const CLOSEOUT_INSPECT_MAX_BUDGET: usize = 200;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CloseoutInspectOptions {
    source: CloseoutInspectSource,
    target: CloseoutInspectTarget,
    budget: usize,
}

impl CloseoutInspectOptions {
    pub fn for_task_on_branch(branch_id: BranchId, task_entity_id: EntityId) -> Self {
        Self {
            source: CloseoutInspectSource::Branch(branch_id),
            target: CloseoutInspectTarget::task(task_entity_id),
            budget: CLOSEOUT_INSPECT_DEFAULT_BUDGET,
        }
    }

    pub fn for_task_at_commit(commit_id: CommitId, task_entity_id: EntityId) -> Self {
        Self {
            source: CloseoutInspectSource::Commit(commit_id),
            target: CloseoutInspectTarget::task(task_entity_id),
            budget: CLOSEOUT_INSPECT_DEFAULT_BUDGET,
        }
    }

    pub fn for_plan_on_branch(branch_id: BranchId, plan_entity_id: EntityId) -> Self {
        Self {
            source: CloseoutInspectSource::Branch(branch_id),
            target: CloseoutInspectTarget::plan(plan_entity_id),
            budget: CLOSEOUT_INSPECT_DEFAULT_BUDGET,
        }
    }

    pub fn for_plan_at_commit(commit_id: CommitId, plan_entity_id: EntityId) -> Self {
        Self {
            source: CloseoutInspectSource::Commit(commit_id),
            target: CloseoutInspectTarget::plan(plan_entity_id),
            budget: CLOSEOUT_INSPECT_DEFAULT_BUDGET,
        }
    }

    pub fn for_goal_on_branch(branch_id: BranchId, goal_entity_id: EntityId) -> Self {
        Self {
            source: CloseoutInspectSource::Branch(branch_id),
            target: CloseoutInspectTarget::goal(goal_entity_id),
            budget: CLOSEOUT_INSPECT_DEFAULT_BUDGET,
        }
    }

    pub fn for_goal_at_commit(commit_id: CommitId, goal_entity_id: EntityId) -> Self {
        Self {
            source: CloseoutInspectSource::Commit(commit_id),
            target: CloseoutInspectTarget::goal(goal_entity_id),
            budget: CLOSEOUT_INSPECT_DEFAULT_BUDGET,
        }
    }

    pub fn with_budget(mut self, budget: usize) -> Result<Self> {
        validate_closeout_inspect_budget(budget)?;
        self.budget = budget;
        Ok(self)
    }

    pub fn source(&self) -> CloseoutInspectSource {
        self.source
    }

    pub fn target(&self) -> CloseoutInspectTarget {
        self.target
    }

    pub fn budget(&self) -> usize {
        self.budget
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CloseoutInspectSource {
    Branch(BranchId),
    Commit(CommitId),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct CloseoutInspectTarget {
    pub kind: CloseoutInspectTargetKind,
    pub entity_id: EntityId,
}

impl CloseoutInspectTarget {
    pub fn task(entity_id: EntityId) -> Self {
        Self {
            kind: CloseoutInspectTargetKind::Task,
            entity_id,
        }
    }

    pub fn plan(entity_id: EntityId) -> Self {
        Self {
            kind: CloseoutInspectTargetKind::Plan,
            entity_id,
        }
    }

    pub fn goal(entity_id: EntityId) -> Self {
        Self {
            kind: CloseoutInspectTargetKind::Goal,
            entity_id,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CloseoutInspectTargetKind {
    Goal,
    Plan,
    Task,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct CloseoutInspectTaskProjection {
    pub source: CloseoutInspectResolvedSource,
    pub target: CloseoutInspectTarget,
    pub target_resolution: CloseoutInspectTargetResolution,
    pub read_proof: CloseoutInspectReadProof,
    pub budget: CloseoutInspectBudgetReport,
    pub counts: CloseoutInspectCounts,
    pub omitted: Vec<CloseoutInspectOmitted>,
    pub runtime_summary: CloseoutInspectRuntimeSummary,
    pub truncated: bool,
    pub task: Option<CloseoutInspectTaskItem>,
    pub acceptance_criteria: Vec<CloseoutInspectAcceptanceCriterionItem>,
    pub verification_requirements: Vec<CloseoutInspectVerificationRequirementItem>,
    pub verifications: Vec<CloseoutInspectVerificationItem>,
    pub evidence: Vec<CloseoutInspectEvidenceItem>,
    pub gaps: Vec<CloseoutInspectGap>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct CloseoutInspectPlanProjection {
    pub source: CloseoutInspectResolvedSource,
    pub target: CloseoutInspectTarget,
    pub target_resolution: CloseoutInspectTargetResolution,
    pub read_proof: CloseoutInspectReadProof,
    pub budget: CloseoutInspectBudgetReport,
    pub counts: CloseoutInspectPlanCounts,
    pub omitted: Vec<CloseoutInspectOmitted>,
    pub non_expanded: Vec<CloseoutInspectNonExpanded>,
    pub runtime_summary: CloseoutInspectRuntimeSummary,
    pub truncated: bool,
    pub plan: Option<CloseoutInspectPlanItem>,
    pub direct_tasks: Vec<CloseoutInspectDirectTaskItem>,
    pub gaps: Vec<CloseoutInspectGap>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct CloseoutInspectGoalProjection {
    pub source: CloseoutInspectResolvedSource,
    pub target: CloseoutInspectTarget,
    pub target_resolution: CloseoutInspectTargetResolution,
    pub read_proof: CloseoutInspectReadProof,
    pub budget: CloseoutInspectBudgetReport,
    pub counts: CloseoutInspectGoalCounts,
    pub omitted: Vec<CloseoutInspectOmitted>,
    pub non_expanded: Vec<CloseoutInspectNonExpanded>,
    pub runtime_summary: CloseoutInspectRuntimeSummary,
    pub truncated: bool,
    pub goal: Option<CloseoutInspectGoalItem>,
    pub direct_plans: Vec<CloseoutInspectDirectPlanItem>,
    pub direct_tasks: Vec<CloseoutInspectDirectTaskItem>,
    pub gaps: Vec<CloseoutInspectGap>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct CloseoutInspectResolvedSource {
    pub kind: CloseoutInspectSourceKind,
    pub branch_id: Option<BranchId>,
    pub commit_id: CommitId,
    pub state_digest: Digest,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CloseoutInspectSourceKind {
    Branch,
    Commit,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CloseoutInspectTargetResolution {
    Found,
    TargetNotFound,
    WrongKind,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct CloseoutInspectReadProof {
    pub stable: bool,
    pub before: CloseoutInspectReadObservation,
    pub after: CloseoutInspectReadObservation,
    pub drift: Vec<CloseoutInspectReadDrift>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct CloseoutInspectReadObservation {
    pub branch_head: Option<CloseoutInspectBranchHeadProof>,
    pub source_state_digest: Digest,
    pub target_digest_status: CloseoutInspectTargetDigestStatus,
    pub target_state_digest: Option<Digest>,
    pub store_files: Vec<CloseoutInspectStoreFileMetadata>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct CloseoutInspectBranchHeadProof {
    pub branch_id: BranchId,
    pub head_commit_id: CommitId,
    pub state_digest: Digest,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CloseoutInspectTargetDigestStatus {
    Found,
    TargetNotFound,
    WrongKind,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct CloseoutInspectStoreFileMetadata {
    pub kind: CloseoutInspectStoreFileKind,
    pub path: String,
    pub exists: bool,
    pub size_bytes: Option<u64>,
    pub modified_unix_epoch_nanos: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CloseoutInspectStoreFileKind {
    Main,
    Wal,
    Shm,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct CloseoutInspectReadDrift {
    pub field: String,
    pub before: String,
    pub after: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct CloseoutInspectBudgetReport {
    pub requested: usize,
    pub hard_limit: usize,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct CloseoutInspectCounts {
    pub acceptance_criteria_total: usize,
    pub verification_requirements_total: usize,
    pub verifications_total: usize,
    pub evidence_total: usize,
    pub gaps_total: usize,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct CloseoutInspectPlanCounts {
    pub direct_tasks_total: usize,
    pub direct_child_plans_total: usize,
    pub gaps_total: usize,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct CloseoutInspectGoalCounts {
    pub direct_plans_total: usize,
    pub direct_tasks_total: usize,
    pub direct_child_goals_total: usize,
    pub gaps_total: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct CloseoutInspectOmitted {
    pub category: CloseoutInspectCategory,
    pub count: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct CloseoutInspectNonExpanded {
    pub category: CloseoutInspectNonExpandedCategory,
    pub count: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CloseoutInspectCategory {
    AuthorizationReceipts,
    AcceptanceCriteria,
    DirectPlans,
    DirectTasks,
    VerificationRequirements,
    Verifications,
    Evidence,
    Handoffs,
    RuntimeClaims,
    RuntimeGaps,
    RuntimeSessions,
    Gaps,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CloseoutInspectNonExpandedCategory {
    DirectChildGoals,
    DirectChildPlans,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct CloseoutInspectTaskItem {
    pub task_entity_id: EntityId,
    pub task_entity_version_id: EntityVersionId,
    pub status: String,
    pub state_digest: Digest,
    pub acceptance_criteria_count: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct CloseoutInspectPlanItem {
    pub plan_entity_id: EntityId,
    pub plan_entity_version_id: EntityVersionId,
    pub status: String,
    pub state_digest: Digest,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct CloseoutInspectGoalItem {
    pub goal_entity_id: EntityId,
    pub goal_entity_version_id: EntityVersionId,
    pub status: String,
    pub state_digest: Digest,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct CloseoutInspectContainmentItem {
    pub relation_id: crate::RelationId,
    pub relation_version_id: crate::RelationVersionId,
    pub relation_state_digest: Digest,
    pub parent_entity_id: EntityId,
    pub child_entity_id: EntityId,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct CloseoutInspectDirectPlanItem {
    pub containment: CloseoutInspectContainmentItem,
    pub plan: CloseoutInspectPlanItem,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct CloseoutInspectDirectTaskItem {
    pub containment: CloseoutInspectContainmentItem,
    pub task: CloseoutInspectTaskItem,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct CloseoutInspectRuntimeSummary {
    pub counts: CloseoutInspectRuntimeCounts,
    pub receipt_evaluation_at_us: i64,
    pub sessions: Vec<CloseoutInspectSessionItem>,
    pub claims: Vec<CloseoutInspectClaimItem>,
    pub handoffs: Vec<CloseoutInspectHandoffItem>,
    pub authorization_receipts: Vec<CloseoutInspectAuthorizationReceiptItem>,
    pub gaps: Vec<CloseoutInspectRuntimeGap>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct CloseoutInspectRuntimeCounts {
    pub sessions_total: usize,
    pub claims_total: usize,
    pub handoffs_total: usize,
    pub authorization_receipts_total: usize,
    pub gaps_total: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct CloseoutInspectSessionItem {
    pub session_id: SessionId,
    pub lifecycle_state: String,
    pub started_at_us: i64,
    pub last_activity_at_us: Option<i64>,
    pub active_workspace_id: Option<crate::WorkspaceId>,
    pub active_branch_id: Option<BranchId>,
    pub focus_entity_id: Option<EntityId>,
    pub focus_path_len: usize,
    pub session_diff_id: Option<SessionDiffId>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct CloseoutInspectClaimItem {
    pub claim_id: ClaimId,
    pub lifecycle_state: String,
    pub session_id: SessionId,
    pub workspace_id: crate::WorkspaceId,
    pub branch_id: BranchId,
    pub task_entity_id: EntityId,
    pub mode: String,
    pub created_at_us: i64,
    pub last_activity_at_us: Option<i64>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct CloseoutInspectHandoffItem {
    pub handoff_record_entity_id: EntityId,
    pub handoff_record_entity_version_id: EntityVersionId,
    pub status: String,
    pub state_digest: Digest,
    pub focus_entity_id: EntityId,
    pub session_id: Option<SessionId>,
    pub session_lifecycle_state: Option<String>,
    pub session_diff_id: Option<SessionDiffId>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct CloseoutInspectAuthorizationReceiptItem {
    pub receipt_entity_id: EntityId,
    pub receipt_entity_version_id: EntityVersionId,
    pub state_digest: Digest,
    pub evaluated_at_us: i64,
    pub record_status: String,
    pub mechanical_status: String,
    pub action: String,
    pub target_entity_id: EntityId,
    pub target_entity_kind: String,
    pub target_state_digest: Digest,
    pub contract_digest_domain: String,
    pub contract_digest: Digest,
    pub authority_ref_kind: String,
    pub authority_ref_digest: Digest,
    pub authority_digest: Digest,
    pub expires_at_us: Option<i64>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct CloseoutInspectRuntimeGap {
    pub category: CloseoutInspectRuntimeGapCategory,
    pub code: String,
    pub message: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CloseoutInspectRuntimeGapCategory {
    Claims,
    Handoffs,
    Receipts,
    Sessions,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct CloseoutInspectAcceptanceCriterionItem {
    pub local_key: String,
    pub task_entity_id: EntityId,
    pub acceptance_criterion_entity_id: EntityId,
    pub acceptance_criterion_entity_version_id: EntityVersionId,
    pub classification: String,
    pub state_digest: Digest,
    pub verification_requirements_count: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct CloseoutInspectVerificationRequirementItem {
    pub local_key: String,
    pub acceptance_criterion_entity_id: EntityId,
    pub verification_requirement_entity_id: EntityId,
    pub verification_requirement_entity_version_id: EntityVersionId,
    pub state_digest: Digest,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct CloseoutInspectVerificationItem {
    pub verification_entity_id: EntityId,
    pub verification_entity_version_id: EntityVersionId,
    pub target_kind: CloseoutInspectVerificationTargetKind,
    pub target_entity_id: EntityId,
    pub result: String,
    pub verified_at_commit_id: CommitId,
    pub state_digest: Digest,
    pub verifies_relation_state_digest: Digest,
    pub evidence_count: usize,
    pub semantic_dependency_count: usize,
    pub resource_basis_count: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CloseoutInspectVerificationTargetKind {
    AcceptanceCriterion,
    VerificationRequirement,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct CloseoutInspectEvidenceItem {
    pub evidence_id: EvidenceId,
    pub evidence_kind: String,
    pub captured_at_us: i64,
    pub source_session_id: Option<crate::SessionId>,
    pub contents: Vec<CloseoutInspectEvidenceContentItem>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct CloseoutInspectEvidenceContentItem {
    pub ordinal: usize,
    pub role: String,
    pub content_digest: Digest,
    pub size_bytes: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct CloseoutInspectGap {
    pub category: CloseoutInspectGapCategory,
    pub code: String,
    pub subject_id: Option<EntityId>,
    pub related_id: Option<EntityId>,
    pub message: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CloseoutInspectGapCategory {
    Target,
    AcceptanceCriterion,
    Containment,
    VerificationRequirement,
    Verification,
    Evidence,
}

pub(crate) fn inspect_task(
    engine: &Engine,
    options: CloseoutInspectOptions,
) -> Result<CloseoutInspectTaskProjection> {
    validate_closeout_inspect_budget(options.budget)?;
    ensure_target_kind(options.target, CloseoutInspectTargetKind::Task)?;
    let before = capture_read_observation(engine, options.source, options.target)?;
    let source = before.source;
    let state = before.state;
    let mut gaps = Vec::new();

    let target_present = state_contains_entity(&state, options.target.entity_id);
    let mut target_resolution = CloseoutInspectTargetResolution::Found;
    let task = if !target_present {
        gaps.push(gap(
            CloseoutInspectGapCategory::Target,
            "target_not_found",
            Some(options.target.entity_id),
            None,
            format!(
                "target task entity {} is not present at commit {}",
                options.target.entity_id, source.commit_id
            ),
        ));
        target_resolution = CloseoutInspectTargetResolution::TargetNotFound;
        None
    } else {
        match engine.task_at(source.commit_id, options.target.entity_id) {
            Ok(task) => Some(task),
            Err(error) if is_task_wrong_kind_error(&error) => {
                gaps.push(gap(
                    CloseoutInspectGapCategory::Target,
                    "target_wrong_kind",
                    Some(options.target.entity_id),
                    None,
                    format!(
                        "target entity {} is present at commit {} but is not a task",
                        options.target.entity_id, source.commit_id
                    ),
                ));
                target_resolution = CloseoutInspectTargetResolution::WrongKind;
                None
            }
            Err(error) => return Err(error),
        }
    };

    let (mut criteria, mut requirements, mut verifications, mut evidence) =
        if let Some(task) = task.as_ref() {
            let criteria = direct_acceptance_criteria(engine, source.commit_id, task, &mut gaps)?;
            let requirements =
                direct_verification_requirements(engine, source.commit_id, &criteria, &mut gaps)?;
            let verifications = direct_verifications(
                engine,
                source.commit_id,
                &criteria,
                &requirements,
                &mut gaps,
            )?;
            let evidence = direct_evidence(engine, &verifications, &mut gaps)?;
            add_absence_gaps(task, &criteria, &requirements, &verifications, &mut gaps);
            (criteria, requirements, verifications, evidence)
        } else {
            (Vec::new(), Vec::new(), Vec::new(), Vec::new())
        };

    criteria.sort_by(|left, right| {
        left.local_key.cmp(&right.local_key).then_with(|| {
            left.acceptance_criterion_entity_id
                .cmp(&right.acceptance_criterion_entity_id)
        })
    });
    requirements.sort_by(|left, right| {
        left.acceptance_criterion_entity_id
            .cmp(&right.acceptance_criterion_entity_id)
            .then_with(|| left.local_key.cmp(&right.local_key))
            .then_with(|| {
                left.verification_requirement_entity_id
                    .cmp(&right.verification_requirement_entity_id)
            })
    });
    verifications.sort_by(|left, right| {
        (left.target_kind, left.target_entity_id)
            .cmp(&(right.target_kind, right.target_entity_id))
            .then_with(|| {
                left.verification_entity_id
                    .cmp(&right.verification_entity_id)
            })
    });
    evidence.sort_by_key(|item| item.evidence_id);
    gaps.sort_by(|left, right| {
        left.category
            .cmp(&right.category)
            .then_with(|| left.code.cmp(&right.code))
            .then_with(|| left.subject_id.cmp(&right.subject_id))
            .then_with(|| left.related_id.cmp(&right.related_id))
            .then_with(|| left.message.cmp(&right.message))
    });
    let runtime_summary = runtime_summary(engine, &source, options.target)?;

    let counts = CloseoutInspectCounts {
        acceptance_criteria_total: criteria.len(),
        verification_requirements_total: requirements.len(),
        verifications_total: verifications.len(),
        evidence_total: evidence.len(),
        gaps_total: gaps.len(),
    };
    let mut budget = ProjectionBudget::new(options.budget);
    let acceptance_criteria = budget.take(CloseoutInspectCategory::AcceptanceCriteria, criteria);
    let verification_requirements = budget.take(
        CloseoutInspectCategory::VerificationRequirements,
        requirements,
    );
    let verifications = budget.take(CloseoutInspectCategory::Verifications, verifications);
    let evidence = budget.take(CloseoutInspectCategory::Evidence, evidence);
    let runtime_summary = budget_runtime_summary(&mut budget, runtime_summary);
    let gaps = budget.take(CloseoutInspectCategory::Gaps, gaps);
    let omitted = budget.omitted;
    let truncated = !omitted.is_empty();
    let verifications = verifications
        .into_iter()
        .map(|verification| verification.item)
        .collect();
    let after = capture_read_observation(engine, options.source, options.target)?;
    let read_proof = read_proof(before.observation, after.observation);

    Ok(CloseoutInspectTaskProjection {
        source,
        target: options.target,
        target_resolution,
        read_proof,
        budget: CloseoutInspectBudgetReport {
            requested: options.budget,
            hard_limit: CLOSEOUT_INSPECT_MAX_BUDGET,
        },
        counts,
        omitted,
        runtime_summary,
        truncated,
        task: task.as_ref().map(task_item),
        acceptance_criteria,
        verification_requirements,
        verifications,
        evidence,
        gaps,
    })
}

pub(crate) fn inspect_plan(
    engine: &Engine,
    options: CloseoutInspectOptions,
) -> Result<CloseoutInspectPlanProjection> {
    validate_closeout_inspect_budget(options.budget)?;
    ensure_target_kind(options.target, CloseoutInspectTargetKind::Plan)?;
    let before = capture_read_observation(engine, options.source, options.target)?;
    let source = before.source;
    let state = before.state;
    let mut gaps = Vec::new();

    let target_present = state_contains_entity(&state, options.target.entity_id);
    let mut target_resolution = CloseoutInspectTargetResolution::Found;
    let plan = if !target_present {
        gaps.push(gap(
            CloseoutInspectGapCategory::Target,
            "target_not_found",
            Some(options.target.entity_id),
            None,
            format!(
                "target plan entity {} is not present at commit {}",
                options.target.entity_id, source.commit_id
            ),
        ));
        target_resolution = CloseoutInspectTargetResolution::TargetNotFound;
        None
    } else {
        match engine.plan_at(source.commit_id, options.target.entity_id) {
            Ok(plan) => Some(plan),
            Err(error) if is_wrong_kind_error(&error, CloseoutInspectTargetKind::Plan) => {
                gaps.push(gap(
                    CloseoutInspectGapCategory::Target,
                    "target_wrong_kind",
                    Some(options.target.entity_id),
                    None,
                    format!(
                        "target entity {} is present at commit {} but is not a plan",
                        options.target.entity_id, source.commit_id
                    ),
                ));
                target_resolution = CloseoutInspectTargetResolution::WrongKind;
                None
            }
            Err(error) => return Err(error),
        }
    };

    let (direct_tasks, direct_child_plans_total) = if plan.is_some() {
        direct_plan_children(
            engine,
            source.commit_id,
            options.target.entity_id,
            &mut gaps,
        )?
    } else {
        (Vec::new(), 0)
    };
    gaps.sort_by(gap_order);
    let runtime_summary = runtime_summary(engine, &source, options.target)?;

    let counts = CloseoutInspectPlanCounts {
        direct_tasks_total: direct_tasks.len(),
        direct_child_plans_total,
        gaps_total: gaps.len(),
    };
    let mut budget = ProjectionBudget::new(options.budget);
    let direct_tasks = budget.take(CloseoutInspectCategory::DirectTasks, direct_tasks);
    let runtime_summary = budget_runtime_summary(&mut budget, runtime_summary);
    let gaps = budget.take(CloseoutInspectCategory::Gaps, gaps);
    let omitted = budget.omitted;
    let truncated = !omitted.is_empty();
    let non_expanded = non_expanded(
        CloseoutInspectNonExpandedCategory::DirectChildPlans,
        direct_child_plans_total,
    );
    let after = capture_read_observation(engine, options.source, options.target)?;
    let read_proof = read_proof(before.observation, after.observation);

    Ok(CloseoutInspectPlanProjection {
        source,
        target: options.target,
        target_resolution,
        read_proof,
        budget: CloseoutInspectBudgetReport {
            requested: options.budget,
            hard_limit: CLOSEOUT_INSPECT_MAX_BUDGET,
        },
        counts,
        omitted,
        non_expanded,
        runtime_summary,
        truncated,
        plan: plan.as_ref().map(plan_item),
        direct_tasks,
        gaps,
    })
}

pub(crate) fn inspect_goal(
    engine: &Engine,
    options: CloseoutInspectOptions,
) -> Result<CloseoutInspectGoalProjection> {
    validate_closeout_inspect_budget(options.budget)?;
    ensure_target_kind(options.target, CloseoutInspectTargetKind::Goal)?;
    let before = capture_read_observation(engine, options.source, options.target)?;
    let source = before.source;
    let state = before.state;
    let mut gaps = Vec::new();

    let target_present = state_contains_entity(&state, options.target.entity_id);
    let mut target_resolution = CloseoutInspectTargetResolution::Found;
    let goal = if !target_present {
        gaps.push(gap(
            CloseoutInspectGapCategory::Target,
            "target_not_found",
            Some(options.target.entity_id),
            None,
            format!(
                "target goal entity {} is not present at commit {}",
                options.target.entity_id, source.commit_id
            ),
        ));
        target_resolution = CloseoutInspectTargetResolution::TargetNotFound;
        None
    } else {
        match engine.goal_at(source.commit_id, options.target.entity_id) {
            Ok(goal) => Some(goal),
            Err(error) if is_wrong_kind_error(&error, CloseoutInspectTargetKind::Goal) => {
                gaps.push(gap(
                    CloseoutInspectGapCategory::Target,
                    "target_wrong_kind",
                    Some(options.target.entity_id),
                    None,
                    format!(
                        "target entity {} is present at commit {} but is not a goal",
                        options.target.entity_id, source.commit_id
                    ),
                ));
                target_resolution = CloseoutInspectTargetResolution::WrongKind;
                None
            }
            Err(error) => return Err(error),
        }
    };

    let (direct_plans, direct_tasks, direct_child_goals_total) = if goal.is_some() {
        direct_goal_children(
            engine,
            source.commit_id,
            options.target.entity_id,
            &mut gaps,
        )?
    } else {
        (Vec::new(), Vec::new(), 0)
    };
    gaps.sort_by(gap_order);
    let runtime_summary = runtime_summary(engine, &source, options.target)?;

    let counts = CloseoutInspectGoalCounts {
        direct_plans_total: direct_plans.len(),
        direct_tasks_total: direct_tasks.len(),
        direct_child_goals_total,
        gaps_total: gaps.len(),
    };
    let mut budget = ProjectionBudget::new(options.budget);
    let direct_plans = budget.take(CloseoutInspectCategory::DirectPlans, direct_plans);
    let direct_tasks = budget.take(CloseoutInspectCategory::DirectTasks, direct_tasks);
    let runtime_summary = budget_runtime_summary(&mut budget, runtime_summary);
    let gaps = budget.take(CloseoutInspectCategory::Gaps, gaps);
    let omitted = budget.omitted;
    let truncated = !omitted.is_empty();
    let non_expanded = non_expanded(
        CloseoutInspectNonExpandedCategory::DirectChildGoals,
        direct_child_goals_total,
    );
    let after = capture_read_observation(engine, options.source, options.target)?;
    let read_proof = read_proof(before.observation, after.observation);

    Ok(CloseoutInspectGoalProjection {
        source,
        target: options.target,
        target_resolution,
        read_proof,
        budget: CloseoutInspectBudgetReport {
            requested: options.budget,
            hard_limit: CLOSEOUT_INSPECT_MAX_BUDGET,
        },
        counts,
        omitted,
        non_expanded,
        runtime_summary,
        truncated,
        goal: goal.as_ref().map(goal_item),
        direct_plans,
        direct_tasks,
        gaps,
    })
}

struct CapturedReadObservation {
    source: CloseoutInspectResolvedSource,
    state: crate::ReplayedState,
    observation: CloseoutInspectReadObservation,
}

fn capture_read_observation(
    engine: &Engine,
    source: CloseoutInspectSource,
    target: CloseoutInspectTarget,
) -> Result<CapturedReadObservation> {
    let store_files = store_file_metadata(engine.store_path())?;
    match source {
        CloseoutInspectSource::Branch(branch_id) => {
            let head = engine.branch_head(branch_id)?;
            let state = engine.state_at(head.head_commit_id)?;
            let (target_digest_status, target_state_digest) =
                target_digest_at(engine, head.head_commit_id, &state, target)?;
            Ok(CapturedReadObservation {
                source: CloseoutInspectResolvedSource {
                    kind: CloseoutInspectSourceKind::Branch,
                    branch_id: Some(branch_id),
                    commit_id: head.head_commit_id,
                    state_digest: state.state_digest,
                },
                observation: CloseoutInspectReadObservation {
                    branch_head: Some(CloseoutInspectBranchHeadProof {
                        branch_id,
                        head_commit_id: head.head_commit_id,
                        state_digest: head.state_digest,
                    }),
                    source_state_digest: state.state_digest,
                    target_digest_status,
                    target_state_digest,
                    store_files,
                },
                state,
            })
        }
        CloseoutInspectSource::Commit(commit_id) => {
            let state = engine.state_at(commit_id)?;
            let (target_digest_status, target_state_digest) =
                target_digest_at(engine, commit_id, &state, target)?;
            Ok(CapturedReadObservation {
                source: CloseoutInspectResolvedSource {
                    kind: CloseoutInspectSourceKind::Commit,
                    branch_id: None,
                    commit_id,
                    state_digest: state.state_digest,
                },
                observation: CloseoutInspectReadObservation {
                    branch_head: None,
                    source_state_digest: state.state_digest,
                    target_digest_status,
                    target_state_digest,
                    store_files,
                },
                state,
            })
        }
    }
}

fn target_digest_at(
    engine: &Engine,
    commit_id: CommitId,
    state: &crate::ReplayedState,
    target: CloseoutInspectTarget,
) -> Result<(CloseoutInspectTargetDigestStatus, Option<Digest>)> {
    if !state_contains_entity(state, target.entity_id) {
        return Ok((CloseoutInspectTargetDigestStatus::TargetNotFound, None));
    }
    match target.kind {
        CloseoutInspectTargetKind::Goal => match engine.goal_at(commit_id, target.entity_id) {
            Ok(goal) => Ok((
                CloseoutInspectTargetDigestStatus::Found,
                Some(goal.state_digest),
            )),
            Err(error) if is_wrong_kind_error(&error, target.kind) => {
                Ok((CloseoutInspectTargetDigestStatus::WrongKind, None))
            }
            Err(error) => Err(error),
        },
        CloseoutInspectTargetKind::Plan => match engine.plan_at(commit_id, target.entity_id) {
            Ok(plan) => Ok((
                CloseoutInspectTargetDigestStatus::Found,
                Some(plan.state_digest),
            )),
            Err(error) if is_wrong_kind_error(&error, target.kind) => {
                Ok((CloseoutInspectTargetDigestStatus::WrongKind, None))
            }
            Err(error) => Err(error),
        },
        CloseoutInspectTargetKind::Task => match engine.task_at(commit_id, target.entity_id) {
            Ok(task) => Ok((
                CloseoutInspectTargetDigestStatus::Found,
                Some(task.state_digest),
            )),
            Err(error) if is_wrong_kind_error(&error, target.kind) => {
                Ok((CloseoutInspectTargetDigestStatus::WrongKind, None))
            }
            Err(error) => Err(error),
        },
    }
}

fn state_contains_entity(state: &crate::ReplayedState, target_entity_id: EntityId) -> bool {
    state
        .state
        .entities()
        .iter()
        .any(|(entity_id, _)| *entity_id == target_entity_id)
}

fn ensure_target_kind(
    target: CloseoutInspectTarget,
    expected: CloseoutInspectTargetKind,
) -> Result<()> {
    if target.kind == expected {
        Ok(())
    } else {
        Err(WorkVcsError::QueryInvalid(format!(
            "closeout inspect expected target kind {expected:?}, found {:?}",
            target.kind
        )))
    }
}

fn non_expanded(
    category: CloseoutInspectNonExpandedCategory,
    count: usize,
) -> Vec<CloseoutInspectNonExpanded> {
    if count == 0 {
        Vec::new()
    } else {
        vec![CloseoutInspectNonExpanded { category, count }]
    }
}

fn gap_order(left: &CloseoutInspectGap, right: &CloseoutInspectGap) -> std::cmp::Ordering {
    left.category
        .cmp(&right.category)
        .then_with(|| left.code.cmp(&right.code))
        .then_with(|| left.subject_id.cmp(&right.subject_id))
        .then_with(|| left.related_id.cmp(&right.related_id))
        .then_with(|| left.message.cmp(&right.message))
}

fn direct_plan_children(
    engine: &Engine,
    commit_id: CommitId,
    plan_entity_id: EntityId,
    gaps: &mut Vec<CloseoutInspectGap>,
) -> Result<(Vec<CloseoutInspectDirectTaskItem>, usize)> {
    let mut direct_tasks = Vec::new();
    let mut direct_child_plans_total = 0;
    for relation in direct_containment_relations(engine, commit_id, plan_entity_id)? {
        match relation.child_kind {
            PrimaryContainmentEndpointKind::Task => {
                match engine.task_at(commit_id, relation.child_entity_id) {
                    Ok(task) => direct_tasks.push(CloseoutInspectDirectTaskItem {
                        containment: containment_item(&relation),
                        task: task_item(&task),
                    }),
                    Err(error) if is_wrong_kind_error(&error, CloseoutInspectTargetKind::Task) => {
                        gaps.push(containment_gap(
                            "direct_task_wrong_kind",
                            &relation,
                            commit_id,
                        ));
                    }
                    Err(error) if error.code() == ErrorCode::TaskNotFound => {
                        gaps.push(containment_gap(
                            "direct_task_not_found",
                            &relation,
                            commit_id,
                        ));
                    }
                    Err(error) => return Err(error),
                }
            }
            PrimaryContainmentEndpointKind::Plan => {
                direct_child_plans_total += 1;
            }
            PrimaryContainmentEndpointKind::Goal => {
                gaps.push(containment_gap(
                    "unexpected_goal_child_for_plan",
                    &relation,
                    commit_id,
                ));
            }
        }
    }
    direct_tasks.sort_by(|left, right| {
        left.task
            .task_entity_id
            .cmp(&right.task.task_entity_id)
            .then_with(|| {
                left.containment
                    .relation_id
                    .cmp(&right.containment.relation_id)
            })
    });
    Ok((direct_tasks, direct_child_plans_total))
}

fn direct_goal_children(
    engine: &Engine,
    commit_id: CommitId,
    goal_entity_id: EntityId,
    gaps: &mut Vec<CloseoutInspectGap>,
) -> Result<(
    Vec<CloseoutInspectDirectPlanItem>,
    Vec<CloseoutInspectDirectTaskItem>,
    usize,
)> {
    let mut direct_plans = Vec::new();
    let mut direct_tasks = Vec::new();
    let mut direct_child_goals_total = 0;
    for relation in direct_containment_relations(engine, commit_id, goal_entity_id)? {
        match relation.child_kind {
            PrimaryContainmentEndpointKind::Plan => {
                match engine.plan_at(commit_id, relation.child_entity_id) {
                    Ok(plan) => direct_plans.push(CloseoutInspectDirectPlanItem {
                        containment: containment_item(&relation),
                        plan: plan_item(&plan),
                    }),
                    Err(error) if is_wrong_kind_error(&error, CloseoutInspectTargetKind::Plan) => {
                        gaps.push(containment_gap(
                            "direct_plan_wrong_kind",
                            &relation,
                            commit_id,
                        ));
                    }
                    Err(error) if error.code() == ErrorCode::PlanNotFound => {
                        gaps.push(containment_gap(
                            "direct_plan_not_found",
                            &relation,
                            commit_id,
                        ));
                    }
                    Err(error) => return Err(error),
                }
            }
            PrimaryContainmentEndpointKind::Task => {
                match engine.task_at(commit_id, relation.child_entity_id) {
                    Ok(task) => direct_tasks.push(CloseoutInspectDirectTaskItem {
                        containment: containment_item(&relation),
                        task: task_item(&task),
                    }),
                    Err(error) if is_wrong_kind_error(&error, CloseoutInspectTargetKind::Task) => {
                        gaps.push(containment_gap(
                            "direct_task_wrong_kind",
                            &relation,
                            commit_id,
                        ));
                    }
                    Err(error) if error.code() == ErrorCode::TaskNotFound => {
                        gaps.push(containment_gap(
                            "direct_task_not_found",
                            &relation,
                            commit_id,
                        ));
                    }
                    Err(error) => return Err(error),
                }
            }
            PrimaryContainmentEndpointKind::Goal => {
                direct_child_goals_total += 1;
            }
        }
    }
    direct_plans.sort_by(|left, right| {
        left.plan
            .plan_entity_id
            .cmp(&right.plan.plan_entity_id)
            .then_with(|| {
                left.containment
                    .relation_id
                    .cmp(&right.containment.relation_id)
            })
    });
    direct_tasks.sort_by(|left, right| {
        left.task
            .task_entity_id
            .cmp(&right.task.task_entity_id)
            .then_with(|| {
                left.containment
                    .relation_id
                    .cmp(&right.containment.relation_id)
            })
    });
    Ok((direct_plans, direct_tasks, direct_child_goals_total))
}

fn direct_containment_relations(
    engine: &Engine,
    commit_id: CommitId,
    parent_entity_id: EntityId,
) -> Result<Vec<PrimaryContainmentSnapshot>> {
    let mut relations = engine
        .primary_containment_relations_at(commit_id)?
        .into_iter()
        .filter(|relation| relation.parent_entity_id == parent_entity_id)
        .collect::<Vec<_>>();
    relations.sort_by(|left, right| {
        left.child_kind
            .cmp(&right.child_kind)
            .then_with(|| left.child_entity_id.cmp(&right.child_entity_id))
            .then_with(|| left.relation_id.cmp(&right.relation_id))
    });
    Ok(relations)
}

fn containment_item(relation: &PrimaryContainmentSnapshot) -> CloseoutInspectContainmentItem {
    CloseoutInspectContainmentItem {
        relation_id: relation.relation_id,
        relation_version_id: relation.relation_version_id,
        relation_state_digest: relation.state_digest,
        parent_entity_id: relation.parent_entity_id,
        child_entity_id: relation.child_entity_id,
    }
}

fn containment_gap(
    code: &'static str,
    relation: &PrimaryContainmentSnapshot,
    commit_id: CommitId,
) -> CloseoutInspectGap {
    gap(
        CloseoutInspectGapCategory::Containment,
        code,
        Some(relation.child_entity_id),
        Some(relation.parent_entity_id),
        format!(
            "primary containment relation {} points from {} to {} at commit {} but the child cannot be projected as {:?}",
            relation.relation_id,
            relation.parent_entity_id,
            relation.child_entity_id,
            commit_id,
            relation.child_kind
        ),
    )
}

fn runtime_summary(
    engine: &Engine,
    source: &CloseoutInspectResolvedSource,
    target: CloseoutInspectTarget,
) -> Result<CloseoutInspectRuntimeSummary> {
    let mut sessions = Vec::new();
    let mut claims = Vec::new();
    let (mut handoffs, mut gaps) = handoff_items(engine, source.commit_id, target)?;
    let receipt_evaluation_at_us = runtime_evaluation_at_us(engine, source)?;
    let mut authorization_receipts =
        authorization_receipt_items(engine, source, target, receipt_evaluation_at_us)?;

    if let Some(branch_id) = source.branch_id {
        let session_result =
            engine.sessions(SessionListOptions::all().with_active_branch_id(branch_id))?;
        for session in session_result.sessions {
            let mut session_matches = session
                .focus
                .as_ref()
                .is_some_and(|focus| focus.focus_entity_id == target.entity_id);
            if target.kind == CloseoutInspectTargetKind::Task {
                let claim_result = engine.active_claims_for_session(
                    crate::ClaimListOptions::for_session(session.session_id),
                )?;
                for claim in claim_result.claims {
                    if claim.task_entity_id == target.entity_id {
                        session_matches = true;
                        claims.push(claim_item(&claim));
                    }
                }
            }
            if session_matches {
                sessions.push(session_item(&session));
            }
        }
    } else {
        gaps.push(runtime_gap(
            CloseoutInspectRuntimeGapCategory::Sessions,
            "runtime_sessions_unavailable_for_commit_source",
            format!(
                "current runtime sessions are branch-scoped and are not projected onto explicit commit {}",
                source.commit_id
            ),
        ));
        gaps.push(runtime_gap(
            CloseoutInspectRuntimeGapCategory::Claims,
            "runtime_claims_unavailable_for_commit_source",
            format!(
                "current runtime claims are branch-scoped and are not projected onto explicit commit {}",
                source.commit_id
            ),
        ));
    }

    sessions.sort_by_key(|item| item.session_id);
    claims.sort_by_key(|item| item.claim_id);
    handoffs.sort_by_key(|item| item.handoff_record_entity_id);
    authorization_receipts.sort_by_key(|item| item.receipt_entity_id);
    gaps.sort_by(|left, right| {
        left.category
            .cmp(&right.category)
            .then_with(|| left.code.cmp(&right.code))
            .then_with(|| left.message.cmp(&right.message))
    });

    let counts = CloseoutInspectRuntimeCounts {
        sessions_total: sessions.len(),
        claims_total: claims.len(),
        handoffs_total: handoffs.len(),
        authorization_receipts_total: authorization_receipts.len(),
        gaps_total: gaps.len(),
    };
    Ok(CloseoutInspectRuntimeSummary {
        counts,
        receipt_evaluation_at_us,
        sessions,
        claims,
        handoffs,
        authorization_receipts,
        gaps,
    })
}

fn budget_runtime_summary(
    budget: &mut ProjectionBudget,
    summary: CloseoutInspectRuntimeSummary,
) -> CloseoutInspectRuntimeSummary {
    CloseoutInspectRuntimeSummary {
        counts: summary.counts,
        receipt_evaluation_at_us: summary.receipt_evaluation_at_us,
        sessions: budget.take(CloseoutInspectCategory::RuntimeSessions, summary.sessions),
        claims: budget.take(CloseoutInspectCategory::RuntimeClaims, summary.claims),
        handoffs: budget.take(CloseoutInspectCategory::Handoffs, summary.handoffs),
        authorization_receipts: budget.take(
            CloseoutInspectCategory::AuthorizationReceipts,
            summary.authorization_receipts,
        ),
        gaps: budget.take(CloseoutInspectCategory::RuntimeGaps, summary.gaps),
    }
}

fn session_item(session: &SessionSnapshot) -> CloseoutInspectSessionItem {
    CloseoutInspectSessionItem {
        session_id: session.session_id,
        lifecycle_state: session_lifecycle_state_str(session.lifecycle_state).to_owned(),
        started_at_us: session.started_at_us,
        last_activity_at_us: session.last_activity_at_us,
        active_workspace_id: session.active_workspace_id,
        active_branch_id: session.active_branch_id,
        focus_entity_id: session.focus.as_ref().map(|focus| focus.focus_entity_id),
        focus_path_len: session
            .focus
            .as_ref()
            .map(|focus| focus.path.len())
            .unwrap_or(0),
        session_diff_id: session.session_diff_id,
    }
}

fn claim_item(claim: &ClaimSnapshot) -> CloseoutInspectClaimItem {
    CloseoutInspectClaimItem {
        claim_id: claim.claim_id,
        lifecycle_state: claim_lifecycle_state_str(claim.lifecycle_state).to_owned(),
        session_id: claim.session_id,
        workspace_id: claim.workspace_id,
        branch_id: claim.branch_id,
        task_entity_id: claim.task_entity_id,
        mode: claim_mode_str(claim.mode).to_owned(),
        created_at_us: claim.created_at_us,
        last_activity_at_us: claim.last_activity_at_us,
    }
}

fn handoff_items(
    engine: &Engine,
    commit_id: CommitId,
    target: CloseoutInspectTarget,
) -> Result<(
    Vec<CloseoutInspectHandoffItem>,
    Vec<CloseoutInspectRuntimeGap>,
)> {
    let records =
        engine.records_at(RecordListOptions::new(commit_id).with_kind(RecordKind::Handoff))?;
    let mut handoffs = Vec::new();
    let mut gaps = Vec::new();
    for record in records.records {
        let scope = match parse_handoff_scope(&record.state.scope) {
            HandoffScopeParse::Focused(scope) => scope,
            HandoffScopeParse::Unfocused => continue,
            HandoffScopeParse::Invalid(error) => {
                gaps.push(handoff_scope_gap(record.record_entity_id, error));
                continue;
            }
        };
        if scope.focus_entity_id != target.entity_id {
            continue;
        }
        handoffs.push(CloseoutInspectHandoffItem {
            handoff_record_entity_id: record.record_entity_id,
            handoff_record_entity_version_id: record.record_entity_version_id,
            status: record_status_str(record.state.status).to_owned(),
            state_digest: record.state_digest,
            focus_entity_id: scope.focus_entity_id,
            session_id: scope.session_id,
            session_lifecycle_state: scope.session_lifecycle_state,
            session_diff_id: scope.session_diff_id,
        });
    }
    Ok((handoffs, gaps))
}

fn authorization_receipt_items(
    engine: &Engine,
    source: &CloseoutInspectResolvedSource,
    target: CloseoutInspectTarget,
    evaluation_at_us: i64,
) -> Result<Vec<CloseoutInspectAuthorizationReceiptItem>> {
    let target_kind = target_kind_str(target.kind);
    let receipts = engine.authorization_receipts_at(
        AuthorizationReceiptListOptions::new(source.commit_id)
            .with_target_entity_id(target.entity_id),
    )?;
    let (_, current_target_digest) = target_digest_at(
        engine,
        source.commit_id,
        &engine.state_at(source.commit_id)?,
        target,
    )?;
    let mut items = Vec::new();
    for receipt in receipts.receipts {
        if receipt.binding.target_entity_kind != target_kind {
            continue;
        }
        items.push(authorization_receipt_item(
            &receipt,
            current_target_digest,
            evaluation_at_us,
        ));
    }
    Ok(items)
}

fn source_commit_evaluation_at_us(engine: &Engine, commit_id: CommitId) -> Result<i64> {
    Ok(engine.commit(commit_id)?.committed_at_us)
}

fn runtime_evaluation_at_us(
    engine: &Engine,
    source: &CloseoutInspectResolvedSource,
) -> Result<i64> {
    if source.branch_id.is_some() {
        current_epoch_micros()
    } else {
        source_commit_evaluation_at_us(engine, source.commit_id)
    }
}

fn current_epoch_micros() -> Result<i64> {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| {
            WorkVcsError::StorageFailure(format!("system clock precedes Unix epoch: {error}"))
        })?;
    i64::try_from(duration.as_micros()).map_err(|_| {
        WorkVcsError::StorageFailure("system time exceeds supported microsecond range".to_owned())
    })
}

fn authorization_receipt_item(
    receipt: &AuthorizationReceiptSnapshot,
    current_target_digest: Option<Digest>,
    evaluation_at_us: i64,
) -> CloseoutInspectAuthorizationReceiptItem {
    let mechanical_status =
        authorization_receipt_mechanical_status(receipt, current_target_digest, evaluation_at_us);
    CloseoutInspectAuthorizationReceiptItem {
        receipt_entity_id: receipt.receipt_entity_id,
        receipt_entity_version_id: receipt.receipt_entity_version_id,
        state_digest: receipt.state_digest,
        evaluated_at_us: evaluation_at_us,
        record_status: record_status_str(receipt.status).to_owned(),
        mechanical_status,
        action: receipt.binding.action.clone(),
        target_entity_id: receipt.binding.target_entity_id,
        target_entity_kind: receipt.binding.target_entity_kind.clone(),
        target_state_digest: receipt.binding.target_state_digest,
        contract_digest_domain: receipt.binding.contract_digest_domain.clone(),
        contract_digest: receipt.binding.contract_digest,
        authority_ref_kind: receipt.binding.authority_ref_kind.clone(),
        authority_ref_digest: receipt.binding.authority_ref_digest,
        authority_digest: receipt.binding.authority_digest,
        expires_at_us: receipt.binding.expires_at_us,
    }
}

fn authorization_receipt_mechanical_status(
    receipt: &AuthorizationReceiptSnapshot,
    current_target_digest: Option<Digest>,
    evaluation_at_us: i64,
) -> String {
    if receipt.status == RecordStatus::Consumed {
        return "consumed".to_owned();
    }
    if receipt
        .binding
        .expires_at_us
        .is_some_and(|expires_at_us| expires_at_us <= evaluation_at_us)
    {
        return "expired".to_owned();
    }
    if current_target_digest != Some(receipt.binding.target_state_digest) {
        return "stale".to_owned();
    }
    record_status_str(receipt.status).to_owned()
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ParsedHandoffScope {
    focus_entity_id: EntityId,
    session_id: Option<SessionId>,
    session_lifecycle_state: Option<String>,
    session_diff_id: Option<SessionDiffId>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum HandoffScopeParse {
    Focused(ParsedHandoffScope),
    Unfocused,
    Invalid(HandoffScopeParseError),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct HandoffScopeParseError {
    code: &'static str,
    message: &'static str,
}

fn parse_handoff_scope(scope: &CanonicalValue) -> HandoffScopeParse {
    let CanonicalValue::Object(entries) = scope else {
        return HandoffScopeParse::Unfocused;
    };

    let has_focused_scope_field = canonical_object_field(entries, "handoff_scope_schema_version")
        .is_some()
        || canonical_object_field(entries, "focus_entity_id").is_some()
        || canonical_object_field(entries, "session_id").is_some()
        || canonical_object_field(entries, "session_diff_id").is_some()
        || canonical_object_field(entries, "session_lifecycle_state").is_some();
    if !has_focused_scope_field {
        return HandoffScopeParse::Unfocused;
    }

    let schema_version = match canonical_object_field(entries, "handoff_scope_schema_version") {
        Some(CanonicalValue::Integer(schema_version)) => schema_version,
        Some(_) => {
            return HandoffScopeParse::Invalid(handoff_scope_error(
                "handoff_scope_schema_invalid",
                "handoff focused scope schema version is not an integer",
            ));
        }
        None => {
            return HandoffScopeParse::Invalid(handoff_scope_error(
                "handoff_scope_schema_missing",
                "handoff focused scope is missing schema version",
            ));
        }
    };
    if schema_version.get() != 1 {
        return HandoffScopeParse::Invalid(handoff_scope_error(
            "handoff_scope_schema_unsupported",
            "handoff focused scope schema version is unsupported",
        ));
    }

    let focus_entity_id = match canonical_object_field(entries, "focus_entity_id") {
        Some(CanonicalValue::String(focus_entity_id)) => focus_entity_id,
        Some(_) => {
            return HandoffScopeParse::Invalid(handoff_scope_error(
                "handoff_focus_entity_id_invalid",
                "handoff focused scope focus_entity_id is not a string",
            ));
        }
        None => {
            return HandoffScopeParse::Invalid(handoff_scope_error(
                "handoff_focus_entity_id_missing",
                "handoff focused scope is missing focus_entity_id",
            ));
        }
    };

    let Ok(focus_entity_id) = EntityId::parse_canonical(focus_entity_id) else {
        return HandoffScopeParse::Invalid(handoff_scope_error(
            "handoff_focus_entity_id_invalid",
            "handoff focused scope focus_entity_id is not a canonical entity id",
        ));
    };

    let session_id = match canonical_object_optional_string(entries, "session_id") {
        Ok(Some(value)) => match SessionId::parse_canonical(value) {
            Ok(session_id) => Some(session_id),
            Err(_) => {
                return HandoffScopeParse::Invalid(handoff_scope_error(
                    "handoff_session_id_invalid",
                    "handoff focused scope session_id is not a canonical session id",
                ));
            }
        },
        Ok(None) => None,
        Err(error) => return HandoffScopeParse::Invalid(error),
    };
    let session_diff_id = match canonical_object_optional_string(entries, "session_diff_id") {
        Ok(Some(value)) => match SessionDiffId::parse_canonical(value) {
            Ok(session_diff_id) => Some(session_diff_id),
            Err(_) => {
                return HandoffScopeParse::Invalid(handoff_scope_error(
                    "handoff_session_diff_id_invalid",
                    "handoff focused scope session_diff_id is not a canonical session diff id",
                ));
            }
        },
        Ok(None) => None,
        Err(error) => return HandoffScopeParse::Invalid(error),
    };
    let session_lifecycle_state =
        match canonical_object_optional_string(entries, "session_lifecycle_state") {
            Ok(Some(value)) => Some(value.to_owned()),
            Ok(None) => None,
            Err(error) => return HandoffScopeParse::Invalid(error),
        };
    HandoffScopeParse::Focused(ParsedHandoffScope {
        focus_entity_id,
        session_id,
        session_lifecycle_state,
        session_diff_id,
    })
}

fn canonical_object_field<'a>(
    entries: &'a [(String, CanonicalValue)],
    key: &str,
) -> Option<&'a CanonicalValue> {
    entries
        .iter()
        .find_map(|(entry_key, value)| (entry_key == key).then_some(value))
}

fn canonical_object_optional_string<'a>(
    entries: &'a [(String, CanonicalValue)],
    key: &str,
) -> std::result::Result<Option<&'a str>, HandoffScopeParseError> {
    match canonical_object_field(entries, key) {
        Some(CanonicalValue::String(value)) => Ok(Some(value)),
        Some(CanonicalValue::Null) | None => Ok(None),
        Some(_) => Err(handoff_scope_error(
            match key {
                "session_id" => "handoff_session_id_invalid",
                "session_diff_id" => "handoff_session_diff_id_invalid",
                "session_lifecycle_state" => "handoff_session_lifecycle_state_invalid",
                _ => "handoff_optional_field_invalid",
            },
            match key {
                "session_id" => "handoff focused scope session_id is not a string or null",
                "session_diff_id" => {
                    "handoff focused scope session_diff_id is not a string or null"
                }
                "session_lifecycle_state" => {
                    "handoff focused scope session_lifecycle_state is not a string or null"
                }
                _ => "handoff focused scope optional field is invalid",
            },
        )),
    }
}

fn handoff_scope_error(code: &'static str, message: &'static str) -> HandoffScopeParseError {
    HandoffScopeParseError { code, message }
}

fn handoff_scope_gap(
    record_entity_id: EntityId,
    error: HandoffScopeParseError,
) -> CloseoutInspectRuntimeGap {
    runtime_gap(
        CloseoutInspectRuntimeGapCategory::Handoffs,
        error.code,
        format!(
            "handoff record {} has invalid focused scope: {}",
            record_entity_id, error.message
        ),
    )
}

fn runtime_gap(
    category: CloseoutInspectRuntimeGapCategory,
    code: impl Into<String>,
    message: String,
) -> CloseoutInspectRuntimeGap {
    CloseoutInspectRuntimeGap {
        category,
        code: code.into(),
        message,
    }
}

fn target_kind_str(kind: CloseoutInspectTargetKind) -> &'static str {
    match kind {
        CloseoutInspectTargetKind::Goal => "goal",
        CloseoutInspectTargetKind::Plan => "plan",
        CloseoutInspectTargetKind::Task => "task",
    }
}

fn session_lifecycle_state_str(state: SessionLifecycleState) -> &'static str {
    match state {
        SessionLifecycleState::Active => "active",
        SessionLifecycleState::PotentiallyStale => "potentially_stale",
        SessionLifecycleState::Ended => "ended",
    }
}

fn claim_lifecycle_state_str(state: ClaimLifecycleState) -> &'static str {
    match state {
        ClaimLifecycleState::Active => "active",
        ClaimLifecycleState::Released => "released",
    }
}

fn claim_mode_str(mode: ClaimMode) -> &'static str {
    match mode {
        ClaimMode::Exclusive => "exclusive",
        ClaimMode::Shared => "shared",
    }
}

fn record_status_str(status: RecordStatus) -> &'static str {
    match status {
        RecordStatus::Active => "active",
        RecordStatus::Consumed => "consumed",
        RecordStatus::Failed => "failed",
        RecordStatus::Inconclusive => "inconclusive",
        RecordStatus::Invalidated => "invalidated",
        RecordStatus::Running => "running",
        RecordStatus::Succeeded => "succeeded",
        RecordStatus::Superseded => "superseded",
        RecordStatus::Unverified => "unverified",
        RecordStatus::Validated => "validated",
        RecordStatus::Withdrawn => "withdrawn",
    }
}

fn read_proof(
    before: CloseoutInspectReadObservation,
    after: CloseoutInspectReadObservation,
) -> CloseoutInspectReadProof {
    let mut drift = Vec::new();
    push_drift_if_changed(
        &mut drift,
        "branch_head",
        &before.branch_head,
        &after.branch_head,
    );
    push_drift_if_changed(
        &mut drift,
        "source_state_digest",
        &before.source_state_digest,
        &after.source_state_digest,
    );
    push_drift_if_changed(
        &mut drift,
        "target_digest_status",
        &before.target_digest_status,
        &after.target_digest_status,
    );
    push_drift_if_changed(
        &mut drift,
        "target_state_digest",
        &before.target_state_digest,
        &after.target_state_digest,
    );
    push_drift_if_changed(
        &mut drift,
        "store_files",
        &before.store_files,
        &after.store_files,
    );
    CloseoutInspectReadProof {
        stable: drift.is_empty(),
        before,
        after,
        drift,
    }
}

fn push_drift_if_changed<T: std::fmt::Debug + PartialEq>(
    drift: &mut Vec<CloseoutInspectReadDrift>,
    field: &str,
    before: &T,
    after: &T,
) {
    if before != after {
        drift.push(CloseoutInspectReadDrift {
            field: field.to_owned(),
            before: format!("{before:?}"),
            after: format!("{after:?}"),
        });
    }
}

fn store_file_metadata(path: &Path) -> Result<Vec<CloseoutInspectStoreFileMetadata>> {
    Ok(vec![
        store_file_metadata_for(CloseoutInspectStoreFileKind::Main, path.to_path_buf())?,
        store_file_metadata_for(
            CloseoutInspectStoreFileKind::Wal,
            sqlite_sidecar_path(path, "-wal"),
        )?,
        store_file_metadata_for(
            CloseoutInspectStoreFileKind::Shm,
            sqlite_sidecar_path(path, "-shm"),
        )?,
    ])
}

fn sqlite_sidecar_path(path: &Path, suffix: &str) -> PathBuf {
    let mut raw = OsString::from(path.as_os_str());
    raw.push(suffix);
    PathBuf::from(raw)
}

fn store_file_metadata_for(
    kind: CloseoutInspectStoreFileKind,
    path: PathBuf,
) -> Result<CloseoutInspectStoreFileMetadata> {
    match std::fs::metadata(&path) {
        Ok(metadata) => Ok(CloseoutInspectStoreFileMetadata {
            kind,
            path: path.display().to_string(),
            exists: true,
            size_bytes: Some(metadata.len()),
            modified_unix_epoch_nanos: Some(system_time_epoch_nanos(metadata.modified().map_err(
                |error| {
                    WorkVcsError::StorageFailure(format!(
                        "cannot read modified time for {}: {error}",
                        path.display()
                    ))
                },
            )?)),
        }),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            Ok(CloseoutInspectStoreFileMetadata {
                kind,
                path: path.display().to_string(),
                exists: false,
                size_bytes: None,
                modified_unix_epoch_nanos: None,
            })
        }
        Err(error) => Err(WorkVcsError::StorageFailure(format!(
            "cannot read metadata for {}: {error}",
            path.display()
        ))),
    }
}

fn system_time_epoch_nanos(time: SystemTime) -> String {
    match time.duration_since(UNIX_EPOCH) {
        Ok(duration) => duration.as_nanos().to_string(),
        Err(error) => format!("-{}", error.duration().as_nanos()),
    }
}

fn direct_acceptance_criteria(
    engine: &Engine,
    commit_id: CommitId,
    task: &TaskSnapshot,
    gaps: &mut Vec<CloseoutInspectGap>,
) -> Result<Vec<CloseoutInspectAcceptanceCriterionItem>> {
    let mut refs = task.state.acceptance_criteria.clone();
    refs.sort_by(|left, right| {
        left.local_key.cmp(&right.local_key).then_with(|| {
            left.acceptance_criterion_entity_id
                .cmp(&right.acceptance_criterion_entity_id)
        })
    });
    let mut criteria = Vec::new();
    for criterion_ref in refs {
        match engine
            .acceptance_criterion_at(commit_id, criterion_ref.acceptance_criterion_entity_id)
        {
            Ok(snapshot) => {
                if snapshot.task_entity_id != task.task_entity_id {
                    gaps.push(gap(
                        CloseoutInspectGapCategory::AcceptanceCriterion,
                        "acceptance_criterion_parent_mismatch",
                        Some(snapshot.acceptance_criterion_entity_id),
                        Some(task.task_entity_id),
                        format!(
                            "acceptance criterion {} is referenced by task {} but belongs to task {}",
                            snapshot.acceptance_criterion_entity_id,
                            task.task_entity_id,
                            snapshot.task_entity_id
                        ),
                    ));
                    continue;
                }
                if snapshot.local_key != criterion_ref.local_key {
                    gaps.push(gap(
                        CloseoutInspectGapCategory::AcceptanceCriterion,
                        "acceptance_criterion_local_key_mismatch",
                        Some(snapshot.acceptance_criterion_entity_id),
                        Some(task.task_entity_id),
                        format!(
                            "task references acceptance criterion {} as local key {:?}, but snapshot local key is {:?}",
                            snapshot.acceptance_criterion_entity_id,
                            criterion_ref.local_key,
                            snapshot.local_key
                        ),
                    ));
                    continue;
                }
                criteria.push(acceptance_criterion_item(&snapshot));
            }
            Err(error) if error.code() == ErrorCode::TaskNotFound => {
                gaps.push(gap(
                    CloseoutInspectGapCategory::AcceptanceCriterion,
                    "acceptance_criterion_not_found",
                    Some(criterion_ref.acceptance_criterion_entity_id),
                    Some(task.task_entity_id),
                    format!(
                        "task {} references acceptance criterion {} which is not present at commit {}",
                        task.task_entity_id,
                        criterion_ref.acceptance_criterion_entity_id,
                        commit_id
                    ),
                ));
            }
            Err(error) => return Err(error),
        }
    }
    Ok(criteria)
}

fn direct_verification_requirements(
    engine: &Engine,
    commit_id: CommitId,
    criteria: &[CloseoutInspectAcceptanceCriterionItem],
    gaps: &mut Vec<CloseoutInspectGap>,
) -> Result<Vec<CloseoutInspectVerificationRequirementItem>> {
    let mut requirements = Vec::new();
    for criterion in criteria {
        let snapshot =
            engine.acceptance_criterion_at(commit_id, criterion.acceptance_criterion_entity_id)?;
        let mut refs = snapshot.state.verification_requirements.clone();
        refs.sort_by(|left, right| {
            left.local_key.cmp(&right.local_key).then_with(|| {
                left.verification_requirement_entity_id
                    .cmp(&right.verification_requirement_entity_id)
            })
        });
        for requirement_ref in refs {
            match engine.verification_requirement_at(
                commit_id,
                requirement_ref.verification_requirement_entity_id,
            ) {
                Ok(requirement) => {
                    if requirement.acceptance_criterion_entity_id
                        != criterion.acceptance_criterion_entity_id
                    {
                        gaps.push(gap(
                            CloseoutInspectGapCategory::VerificationRequirement,
                            "verification_requirement_parent_mismatch",
                            Some(requirement.verification_requirement_entity_id),
                            Some(criterion.acceptance_criterion_entity_id),
                            format!(
                                "verification requirement {} is referenced by acceptance criterion {} but belongs to acceptance criterion {}",
                                requirement.verification_requirement_entity_id,
                                criterion.acceptance_criterion_entity_id,
                                requirement.acceptance_criterion_entity_id
                            ),
                        ));
                        continue;
                    }
                    if requirement.local_key != requirement_ref.local_key {
                        gaps.push(gap(
                            CloseoutInspectGapCategory::VerificationRequirement,
                            "verification_requirement_local_key_mismatch",
                            Some(requirement.verification_requirement_entity_id),
                            Some(criterion.acceptance_criterion_entity_id),
                            format!(
                                "acceptance criterion {} references verification requirement {} as local key {:?}, but snapshot local key is {:?}",
                                criterion.acceptance_criterion_entity_id,
                                requirement.verification_requirement_entity_id,
                                requirement_ref.local_key,
                                requirement.local_key
                            ),
                        ));
                        continue;
                    }
                    requirements.push(verification_requirement_item(&requirement));
                }
                Err(error) if error.code() == ErrorCode::TaskNotFound => {
                    gaps.push(gap(
                        CloseoutInspectGapCategory::VerificationRequirement,
                        "verification_requirement_not_found",
                        Some(requirement_ref.verification_requirement_entity_id),
                        Some(criterion.acceptance_criterion_entity_id),
                        format!(
                            "acceptance criterion {} references verification requirement {} which is not present at commit {}",
                            criterion.acceptance_criterion_entity_id,
                            requirement_ref.verification_requirement_entity_id,
                            commit_id
                        ),
                    ));
                }
                Err(error) => return Err(error),
            }
        }
    }
    Ok(requirements)
}

fn direct_verifications(
    engine: &Engine,
    commit_id: CommitId,
    criteria: &[CloseoutInspectAcceptanceCriterionItem],
    requirements: &[CloseoutInspectVerificationRequirementItem],
    _gaps: &mut Vec<CloseoutInspectGap>,
) -> Result<Vec<VerificationWithEvidence>> {
    let criterion_ids = criteria
        .iter()
        .map(|criterion| criterion.acceptance_criterion_entity_id)
        .collect::<BTreeSet<_>>();
    let requirement_ids = requirements
        .iter()
        .map(|requirement| requirement.verification_requirement_entity_id)
        .collect::<BTreeSet<_>>();
    let mut verifications = Vec::new();
    for verification in engine.verifications_at(commit_id)? {
        match verification.target {
            VerificationTarget::AcceptanceCriterion(entity_id)
                if criterion_ids.contains(&entity_id) =>
            {
                verifications.push(verification_item(&verification));
            }
            VerificationTarget::VerificationRequirement(entity_id)
                if requirement_ids.contains(&entity_id) =>
            {
                verifications.push(verification_item(&verification));
            }
            _ => {}
        }
    }
    Ok(verifications)
}

fn direct_evidence(
    engine: &Engine,
    verifications: &[VerificationWithEvidence],
    gaps: &mut Vec<CloseoutInspectGap>,
) -> Result<Vec<CloseoutInspectEvidenceItem>> {
    let mut evidence_ids = BTreeSet::new();
    for verification in verifications {
        for evidence_id in &verification.evidence_ids {
            evidence_ids.insert(*evidence_id);
        }
    }

    let mut evidence = Vec::new();
    for evidence_id in evidence_ids {
        match engine.evidence(evidence_id) {
            Ok(snapshot) => evidence.push(evidence_item(&snapshot)),
            Err(error) if error.code() == ErrorCode::EvidenceNotFound => {
                gaps.push(gap(
                    CloseoutInspectGapCategory::Evidence,
                    "evidence_not_found",
                    None,
                    None,
                    format!("verification references evidence {evidence_id} which does not exist"),
                ));
            }
            Err(error) => return Err(error),
        }
    }
    Ok(evidence)
}

fn add_absence_gaps(
    task: &TaskSnapshot,
    criteria: &[CloseoutInspectAcceptanceCriterionItem],
    requirements: &[CloseoutInspectVerificationRequirementItem],
    verifications: &[VerificationWithEvidence],
    gaps: &mut Vec<CloseoutInspectGap>,
) {
    if task.state.acceptance_criteria.is_empty() {
        gaps.push(gap(
            CloseoutInspectGapCategory::AcceptanceCriterion,
            "task_has_no_acceptance_criteria",
            Some(task.task_entity_id),
            None,
            format!(
                "task {} has no direct acceptance criterion references",
                task.task_entity_id
            ),
        ));
    }

    let verification_targets = verifications
        .iter()
        .map(|verification| {
            (
                verification.item.target_kind,
                verification.item.target_entity_id,
            )
        })
        .collect::<BTreeSet<_>>();
    let requirement_parents = requirements
        .iter()
        .map(|requirement| requirement.acceptance_criterion_entity_id)
        .collect::<BTreeSet<_>>();

    for criterion in criteria {
        let has_direct_verification = verification_targets.contains(&(
            CloseoutInspectVerificationTargetKind::AcceptanceCriterion,
            criterion.acceptance_criterion_entity_id,
        ));
        let has_requirement =
            requirement_parents.contains(&criterion.acceptance_criterion_entity_id);
        if !has_direct_verification && !has_requirement {
            gaps.push(gap(
                CloseoutInspectGapCategory::AcceptanceCriterion,
                "acceptance_criterion_has_no_verification_or_requirement",
                Some(criterion.acceptance_criterion_entity_id),
                Some(task.task_entity_id),
                format!(
                    "acceptance criterion {} has no direct verification and no direct verification requirements",
                    criterion.acceptance_criterion_entity_id
                ),
            ));
        }
    }

    for requirement in requirements {
        if !verification_targets.contains(&(
            CloseoutInspectVerificationTargetKind::VerificationRequirement,
            requirement.verification_requirement_entity_id,
        )) {
            gaps.push(gap(
                CloseoutInspectGapCategory::VerificationRequirement,
                "verification_requirement_has_no_verification",
                Some(requirement.verification_requirement_entity_id),
                Some(requirement.acceptance_criterion_entity_id),
                format!(
                    "verification requirement {} has no direct verification",
                    requirement.verification_requirement_entity_id
                ),
            ));
        }
    }

    for verification in verifications {
        if verification.item.evidence_count == 0 {
            gaps.push(gap(
                CloseoutInspectGapCategory::Verification,
                "verification_has_no_evidence",
                Some(verification.item.verification_entity_id),
                Some(verification.item.target_entity_id),
                format!(
                    "verification {} has no direct evidence references",
                    verification.item.verification_entity_id
                ),
            ));
        }
    }
}

fn task_item(task: &TaskSnapshot) -> CloseoutInspectTaskItem {
    CloseoutInspectTaskItem {
        task_entity_id: task.task_entity_id,
        task_entity_version_id: task.task_entity_version_id,
        status: task.state.status.as_str().to_owned(),
        state_digest: task.state_digest,
        acceptance_criteria_count: task.state.acceptance_criteria.len(),
    }
}

fn plan_item(plan: &PlanSnapshot) -> CloseoutInspectPlanItem {
    CloseoutInspectPlanItem {
        plan_entity_id: plan.plan_entity_id,
        plan_entity_version_id: plan.plan_entity_version_id,
        status: plan.state.status.as_str().to_owned(),
        state_digest: plan.state_digest,
    }
}

fn goal_item(goal: &GoalSnapshot) -> CloseoutInspectGoalItem {
    CloseoutInspectGoalItem {
        goal_entity_id: goal.goal_entity_id,
        goal_entity_version_id: goal.goal_entity_version_id,
        status: goal.state.status.as_str().to_owned(),
        state_digest: goal.state_digest,
    }
}

fn acceptance_criterion_item(
    snapshot: &AcceptanceCriterionSnapshot,
) -> CloseoutInspectAcceptanceCriterionItem {
    CloseoutInspectAcceptanceCriterionItem {
        local_key: snapshot.local_key.clone(),
        task_entity_id: snapshot.task_entity_id,
        acceptance_criterion_entity_id: snapshot.acceptance_criterion_entity_id,
        acceptance_criterion_entity_version_id: snapshot.acceptance_criterion_entity_version_id,
        classification: snapshot.state.classification.as_str().to_owned(),
        state_digest: snapshot.state_digest,
        verification_requirements_count: snapshot.state.verification_requirements.len(),
    }
}

fn verification_requirement_item(
    snapshot: &VerificationRequirementSnapshot,
) -> CloseoutInspectVerificationRequirementItem {
    CloseoutInspectVerificationRequirementItem {
        local_key: snapshot.local_key.clone(),
        acceptance_criterion_entity_id: snapshot.acceptance_criterion_entity_id,
        verification_requirement_entity_id: snapshot.verification_requirement_entity_id,
        verification_requirement_entity_version_id: snapshot
            .verification_requirement_entity_version_id,
        state_digest: snapshot.state_digest,
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct VerificationWithEvidence {
    item: CloseoutInspectVerificationItem,
    evidence_ids: Vec<EvidenceId>,
}

impl std::ops::Deref for VerificationWithEvidence {
    type Target = CloseoutInspectVerificationItem;

    fn deref(&self) -> &Self::Target {
        &self.item
    }
}

fn verification_item(snapshot: &VerificationSnapshot) -> VerificationWithEvidence {
    let (target_kind, target_entity_id) = verification_target_parts(snapshot.target);
    let evidence_ids = snapshot
        .state
        .evidence
        .iter()
        .map(|evidence| evidence.evidence_id)
        .collect::<Vec<_>>();
    VerificationWithEvidence {
        item: CloseoutInspectVerificationItem {
            verification_entity_id: snapshot.verification_entity_id,
            verification_entity_version_id: snapshot.verification_entity_version_id,
            target_kind,
            target_entity_id,
            result: snapshot.state.result.as_str().to_owned(),
            verified_at_commit_id: snapshot.state.verified_at_commit_id,
            state_digest: snapshot.state_digest,
            verifies_relation_state_digest: snapshot.verifies_relation_state_digest,
            evidence_count: snapshot.state.evidence.len(),
            semantic_dependency_count: snapshot.state.semantic_dependencies.len(),
            resource_basis_count: snapshot.state.resource_basis.len(),
        },
        evidence_ids,
    }
}

fn evidence_item(snapshot: &EvidenceSnapshot) -> CloseoutInspectEvidenceItem {
    CloseoutInspectEvidenceItem {
        evidence_id: snapshot.evidence_id,
        evidence_kind: snapshot.evidence_kind.clone(),
        captured_at_us: snapshot.captured_at_us,
        source_session_id: snapshot.source_session_id,
        contents: snapshot
            .contents
            .iter()
            .map(|content| CloseoutInspectEvidenceContentItem {
                ordinal: content.ordinal,
                role: content.role.clone(),
                content_digest: content.content_digest,
                size_bytes: content.size_bytes,
            })
            .collect(),
    }
}

fn verification_target_parts(
    target: VerificationTarget,
) -> (CloseoutInspectVerificationTargetKind, EntityId) {
    match target {
        VerificationTarget::AcceptanceCriterion(entity_id) => (
            CloseoutInspectVerificationTargetKind::AcceptanceCriterion,
            entity_id,
        ),
        VerificationTarget::VerificationRequirement(entity_id) => (
            CloseoutInspectVerificationTargetKind::VerificationRequirement,
            entity_id,
        ),
    }
}

fn gap(
    category: CloseoutInspectGapCategory,
    code: impl Into<String>,
    subject_id: Option<EntityId>,
    related_id: Option<EntityId>,
    message: String,
) -> CloseoutInspectGap {
    CloseoutInspectGap {
        category,
        code: code.into(),
        subject_id,
        related_id,
        message,
    }
}

fn is_task_wrong_kind_error(error: &WorkVcsError) -> bool {
    is_wrong_kind_error(error, CloseoutInspectTargetKind::Task)
}

fn is_wrong_kind_error(error: &WorkVcsError, target_kind: CloseoutInspectTargetKind) -> bool {
    let (code, expected_kind) = match target_kind {
        CloseoutInspectTargetKind::Goal => (ErrorCode::GoalNotFound, "goal"),
        CloseoutInspectTargetKind::Plan => (ErrorCode::PlanNotFound, "plan"),
        CloseoutInspectTargetKind::Task => (ErrorCode::TaskNotFound, "task"),
    };
    if error.code() != code {
        return false;
    }
    let message = error.to_string();
    message.contains(" has kind ") && message.contains(&format!("not \"{expected_kind}\""))
}

fn validate_closeout_inspect_budget(budget: usize) -> Result<()> {
    if budget == 0 || budget > CLOSEOUT_INSPECT_MAX_BUDGET {
        Err(WorkVcsError::QueryInvalid(format!(
            "closeout inspect budget must be between 1 and {CLOSEOUT_INSPECT_MAX_BUDGET}, found {budget}"
        )))
    } else {
        Ok(())
    }
}

struct ProjectionBudget {
    remaining: usize,
    omitted: Vec<CloseoutInspectOmitted>,
}

impl ProjectionBudget {
    fn new(remaining: usize) -> Self {
        Self {
            remaining,
            omitted: Vec::new(),
        }
    }

    fn take<T>(&mut self, category: CloseoutInspectCategory, items: Vec<T>) -> Vec<T> {
        if items.len() <= self.remaining {
            self.remaining -= items.len();
            return items;
        }

        let omitted_count = items.len() - self.remaining;
        self.omitted.push(CloseoutInspectOmitted {
            category,
            count: omitted_count,
        });
        let take_count = self.remaining;
        self.remaining = 0;
        items.into_iter().take(take_count).collect()
    }
}
