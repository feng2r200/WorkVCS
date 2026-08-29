use super::claim::{self, ClaimNextOptions, ClaimNextResult};
use super::context::{self, ContextOverview, ContextOverviewOptions};
use crate::error::Result;
use crate::identity::SessionId;
use crate::store::StoreConnection;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NextWorkOptions {
    session_id: SessionId,
}

impl NextWorkOptions {
    pub fn new(session_id: SessionId) -> Self {
        Self { session_id }
    }

    pub fn session_id(&self) -> SessionId {
        self.session_id
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
    let claim_next =
        claim::claim_next_task(connection, &ClaimNextOptions::new(options.session_id()))?;
    let context = context::context_overview(
        connection,
        &ContextOverviewOptions::new(options.session_id()),
    )?;
    Ok(NextWorkResult {
        claim_next,
        context,
    })
}
