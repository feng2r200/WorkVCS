use super::runnable::{self, RunnableTasksOptions, RunnableTasksProjection};
use super::session::{self, SessionLifecycleState, SessionSnapshot};
use crate::error::{Result, WorkVcsError};
use crate::history::{self, BranchHead};
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

    Ok(ContextOverview {
        session,
        branch,
        runnable_tasks,
    })
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
