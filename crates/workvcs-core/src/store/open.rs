use crate::error::Result;
use crate::history::{
    BranchHead, EntityTransitionCommit, EntityTransitionOptions, HistoryQueryOptions,
    HistoryQueryResult, IntegrityReport, ReplayedState, TaskCreateCommit, TaskCreateOptions,
    TaskSnapshot, WorkspaceInfo, WorkspaceInitOptions,
};
use crate::store::bootstrap::{
    StoreInfo, StoreInitOptions, ensure_empty_database, initialize_manifest, validate_bootstrap,
};
use crate::store::connection::StoreConnection;
use crate::store::schema;
use crate::{BranchId, CommitId, EntityId, WorkspaceId, history};
use std::path::Path;

pub(crate) struct Store {
    connection: StoreConnection,
    info: StoreInfo,
}

impl Store {
    pub(crate) fn init(path: &Path, options: StoreInitOptions) -> Result<Self> {
        let mut connection = StoreConnection::open(path)?;
        ensure_empty_database(&connection)?;
        schema::install(&connection)?;
        let info = initialize_manifest(&mut connection, &options)?;
        let info = validate_bootstrap(&connection).map(|validated| {
            debug_assert_eq!(validated, info);
            validated
        })?;
        Ok(Self { connection, info })
    }

    pub(crate) fn open(path: &Path) -> Result<Self> {
        let connection = StoreConnection::open(path)?;
        let info = validate_bootstrap(&connection)?;
        Ok(Self { connection, info })
    }

    pub(crate) fn info(&self) -> Result<StoreInfo> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        Ok(current)
    }

    pub(crate) fn create_workspace(
        &mut self,
        options: &WorkspaceInitOptions,
    ) -> Result<WorkspaceInfo> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        history::create_workspace(&mut self.connection, current.store_id, options)
    }

    pub(crate) fn workspace_info(&self, workspace_id: WorkspaceId) -> Result<WorkspaceInfo> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        history::load_workspace_info(&self.connection, workspace_id)
    }

    pub(crate) fn state_at(&self, commit_id: CommitId) -> Result<ReplayedState> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        history::state_at(&self.connection, commit_id)
    }

    pub(crate) fn show_at(&self, commit_id: CommitId) -> Result<ReplayedState> {
        self.state_at(commit_id)
    }

    pub(crate) fn branch_head(&self, branch_id: BranchId) -> Result<BranchHead> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        history::branch_head(&self.connection, branch_id)
    }

    pub(crate) fn history(&self, options: &HistoryQueryOptions) -> Result<HistoryQueryResult> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        history::query_history(&self.connection, options)
    }

    pub(crate) fn validate_integrity(&self) -> Result<IntegrityReport> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        history::validate_integrity(&self.connection)
    }

    pub(crate) fn commit_entity_transition(
        &mut self,
        options: &EntityTransitionOptions,
    ) -> Result<EntityTransitionCommit> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        history::commit_entity_transition(&mut self.connection, options)
    }

    pub(crate) fn create_task(&mut self, options: &TaskCreateOptions) -> Result<TaskCreateCommit> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        history::create_task(&mut self.connection, options)
    }

    pub(crate) fn task_at(
        &self,
        commit_id: CommitId,
        task_entity_id: EntityId,
    ) -> Result<TaskSnapshot> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        history::task_at(&self.connection, commit_id, task_entity_id)
    }
}
