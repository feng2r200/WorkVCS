use crate::BranchId;
use crate::CommitId;
use crate::EntityId;
use crate::WorkspaceId;
use crate::error::Result;
use crate::history::{
    BranchHead, EntityTransitionCommit, EntityTransitionOptions, HistoryQueryOptions,
    HistoryQueryResult, IntegrityReport, ReplayedState, TaskCreateCommit, TaskCreateOptions,
    TaskSnapshot, TaskTransitionCommit, TaskTransitionOptions, WorkspaceInfo, WorkspaceInitOptions,
};
use crate::store::{Store, StoreInfo, StoreInitOptions};
use std::path::Path;

pub struct Engine {
    store: Store,
}

impl Engine {
    pub fn init(path: impl AsRef<Path>, options: StoreInitOptions) -> Result<Self> {
        Ok(Self {
            store: Store::init(path.as_ref(), options)?,
        })
    }

    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        Ok(Self {
            store: Store::open(path.as_ref())?,
        })
    }

    pub fn store_info(&self) -> Result<StoreInfo> {
        self.store.info()
    }

    pub fn create_workspace(&mut self, options: WorkspaceInitOptions) -> Result<WorkspaceInfo> {
        self.store.create_workspace(&options)
    }

    pub fn workspace_info(&self, workspace_id: WorkspaceId) -> Result<WorkspaceInfo> {
        self.store.workspace_info(workspace_id)
    }

    pub fn state_at(&self, commit_id: CommitId) -> Result<ReplayedState> {
        self.store.state_at(commit_id)
    }

    pub fn show_at(&self, commit_id: CommitId) -> Result<ReplayedState> {
        self.store.show_at(commit_id)
    }

    pub fn branch_head(&self, branch_id: BranchId) -> Result<BranchHead> {
        self.store.branch_head(branch_id)
    }

    pub fn history(&self, options: HistoryQueryOptions) -> Result<HistoryQueryResult> {
        self.store.history(&options)
    }

    pub fn validate_integrity(&self) -> Result<IntegrityReport> {
        self.store.validate_integrity()
    }

    pub fn commit_entity_transition(
        &mut self,
        options: EntityTransitionOptions,
    ) -> Result<EntityTransitionCommit> {
        self.store.commit_entity_transition(&options)
    }

    pub fn create_task(&mut self, options: TaskCreateOptions) -> Result<TaskCreateCommit> {
        self.store.create_task(&options)
    }

    pub fn transition_task(
        &mut self,
        options: TaskTransitionOptions,
    ) -> Result<TaskTransitionCommit> {
        self.store.transition_task(&options)
    }

    pub fn task_at(&self, commit_id: CommitId, task_entity_id: EntityId) -> Result<TaskSnapshot> {
        self.store.task_at(commit_id, task_entity_id)
    }
}
