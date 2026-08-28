use crate::CommitId;
use crate::WorkspaceId;
use crate::error::Result;
use crate::history::{
    EntityTransitionCommit, EntityTransitionOptions, ReplayedState, WorkspaceInfo,
    WorkspaceInitOptions,
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

    pub fn commit_entity_transition(
        &mut self,
        options: EntityTransitionOptions,
    ) -> Result<EntityTransitionCommit> {
        self.store.commit_entity_transition(&options)
    }
}
