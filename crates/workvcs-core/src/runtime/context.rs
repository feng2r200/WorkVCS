use super::runnable::{self, RunnableTasksOptions, RunnableTasksProjection};
use super::session::{self, SessionLifecycleState, SessionSnapshot};
use crate::error::{Result, WorkVcsError};
use crate::history::{
    self, BranchHead, KnowledgeListOptions, KnowledgeListResult, KnowledgeRelationListOptions,
    KnowledgeRelationListResult, KnowledgeStatus, RecordKnowledgeRelationListOptions,
    RecordKnowledgeRelationListResult, RecordListOptions, RecordListResult,
    RecordRelationListOptions, RecordRelationListResult, WhyQueryOptions, WhyQueryTarget,
    WhyRelationEdge, WhyRelationKind,
};
use crate::identity::{SessionId, WorkspaceId};
use crate::store::StoreConnection;

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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContextOverview {
    pub session: SessionSnapshot,
    pub branch: BranchHead,
    pub runnable_tasks: RunnableTasksProjection,
    pub knowledge: KnowledgeListResult,
    pub knowledge_relations: KnowledgeRelationListResult,
    pub knowledge_exposure_relations: Vec<WhyRelationEdge>,
    pub records: RecordListResult,
    pub record_relations: RecordRelationListResult,
    pub record_knowledge_relations: RecordKnowledgeRelationListResult,
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
        knowledge,
        knowledge_relations,
        knowledge_exposure_relations,
        records,
        record_relations,
        record_knowledge_relations,
    })
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
