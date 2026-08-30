use super::claim::{self, ClaimMode, ClaimNextOptions, ClaimNextResult};
use super::context::{self, ContextOverview, ContextOverviewOptions};
use crate::error::Result;
use crate::identity::SessionId;
use crate::store::StoreConnection;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NextWorkOptions {
    session_id: SessionId,
    mode: ClaimMode,
}

impl NextWorkOptions {
    pub fn new(session_id: SessionId) -> Self {
        Self {
            session_id,
            mode: ClaimMode::Exclusive,
        }
    }

    pub fn with_mode(mut self, mode: ClaimMode) -> Self {
        self.mode = mode;
        self
    }

    pub fn session_id(&self) -> SessionId {
        self.session_id
    }

    pub fn mode(&self) -> ClaimMode {
        self.mode
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NextWorkResult {
    pub claim_next: ClaimNextResult,
    pub context: ContextOverview,
}

pub(crate) fn next_work(
    connection: &mut StoreConnection,
    options: &NextWorkOptions,
) -> Result<NextWorkResult> {
    let claim_next = claim::claim_next_task(
        connection,
        &ClaimNextOptions::new(options.session_id()).with_mode(options.mode()),
    )?;
    let context = context::context_overview(
        connection,
        &ContextOverviewOptions::new(options.session_id()),
    )?;
    Ok(NextWorkResult {
        claim_next,
        context,
    })
}
