use crate::canonical::{CanonicalValue, canonical_bytes};
use crate::error::{Result, WorkVcsError, storage_error};
use crate::identity::{BranchId, CommitId, Digest, EventId, WorkspaceId};
use crate::store::{StoreConnection, current_epoch_micros};
use rusqlite::{OptionalExtension, Transaction, TransactionBehavior, params};

const ACTIVE_BRANCH_LIFECYCLE_STATE: &str = "active";
const BRANCH_FORKED_EVENT_KIND: &str = "branch.forked";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BranchForkSource {
    BranchHead(BranchId),
    Commit(CommitId),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BranchForkOptions {
    source: BranchForkSource,
    name: String,
}

impl BranchForkOptions {
    pub fn from_branch(source_branch_id: BranchId, name: impl Into<String>) -> Result<Self> {
        Self::new(BranchForkSource::BranchHead(source_branch_id), name)
    }

    pub fn from_commit(source_commit_id: CommitId, name: impl Into<String>) -> Result<Self> {
        Self::new(BranchForkSource::Commit(source_commit_id), name)
    }

    fn new(source: BranchForkSource, name: impl Into<String>) -> Result<Self> {
        let name = name.into();
        validate_branch_name(&name)?;
        Ok(Self { source, name })
    }

    pub fn source(&self) -> BranchForkSource {
        self.source
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BranchForkResult {
    pub workspace_id: WorkspaceId,
    pub branch_id: BranchId,
    pub name: String,
    pub source_branch_id: Option<BranchId>,
    pub head_commit_id: CommitId,
    pub state_digest: Digest,
    pub event_id: EventId,
    pub created_at_us: i64,
}

struct ResolvedBranchSource {
    workspace_id: WorkspaceId,
    source_branch_id: Option<BranchId>,
    head_commit_id: CommitId,
    state_digest: Digest,
}

pub(crate) fn fork_branch(
    connection: &mut StoreConnection,
    options: &BranchForkOptions,
) -> Result<BranchForkResult> {
    connection.verify_foreign_keys()?;
    validate_branch_name(options.name())?;
    let created_at_us = current_epoch_micros()?;
    let branch_id = BranchId::new_v7();
    let event_id = EventId::new_v7();

    let transaction = connection
        .inner_mut()
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(storage_error)?;
    let source = resolve_branch_source(&transaction, options.source())?;
    ensure_branch_name_available(&transaction, source.workspace_id, options.name())?;

    transaction
        .execute(
            "INSERT INTO branch(
                branch_id,
                workspace_id,
                name,
                head_commit_id,
                lifecycle_state,
                created_at_us
             )
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                &branch_id.raw_bytes()[..],
                &source.workspace_id.raw_bytes()[..],
                options.name(),
                &source.head_commit_id.raw_bytes()[..],
                ACTIVE_BRANCH_LIFECYCLE_STATE,
                created_at_us
            ],
        )
        .map_err(storage_error)?;

    let payload_json = branch_forked_payload_json(
        source.workspace_id,
        branch_id,
        options.name(),
        source.source_branch_id,
        source.head_commit_id,
    )?;
    transaction
        .execute(
            "INSERT INTO event(
                event_id,
                workspace_id,
                changeset_id,
                session_id,
                event_kind,
                occurred_at_us,
                payload_json
             )
             VALUES (?1, ?2, NULL, NULL, ?3, ?4, ?5)",
            params![
                &event_id.raw_bytes()[..],
                &source.workspace_id.raw_bytes()[..],
                BRANCH_FORKED_EVENT_KIND,
                created_at_us,
                payload_json
            ],
        )
        .map_err(storage_error)?;
    transaction.commit().map_err(storage_error)?;

    Ok(BranchForkResult {
        workspace_id: source.workspace_id,
        branch_id,
        name: options.name().to_owned(),
        source_branch_id: source.source_branch_id,
        head_commit_id: source.head_commit_id,
        state_digest: source.state_digest,
        event_id,
        created_at_us,
    })
}

fn resolve_branch_source(
    transaction: &Transaction<'_>,
    source: BranchForkSource,
) -> Result<ResolvedBranchSource> {
    match source {
        BranchForkSource::BranchHead(source_branch_id) => {
            let branch = load_branch(transaction, source_branch_id)?;
            let commit = load_commit(transaction, branch.head_commit_id)?;
            if commit.workspace_id != branch.workspace_id {
                return Err(WorkVcsError::WorkspaceInvalid(format!(
                    "branch {source_branch_id} head {} belongs to workspace {}, not {}",
                    branch.head_commit_id, commit.workspace_id, branch.workspace_id
                )));
            }
            Ok(ResolvedBranchSource {
                workspace_id: branch.workspace_id,
                source_branch_id: Some(source_branch_id),
                head_commit_id: branch.head_commit_id,
                state_digest: commit.state_digest,
            })
        }
        BranchForkSource::Commit(source_commit_id) => {
            let commit = load_commit(transaction, source_commit_id)?;
            Ok(ResolvedBranchSource {
                workspace_id: commit.workspace_id,
                source_branch_id: None,
                head_commit_id: source_commit_id,
                state_digest: commit.state_digest,
            })
        }
    }
}

struct BranchRow {
    workspace_id: WorkspaceId,
    head_commit_id: CommitId,
}

fn load_branch(transaction: &Transaction<'_>, branch_id: BranchId) -> Result<BranchRow> {
    let row = transaction
        .query_row(
            "SELECT workspace_id, head_commit_id
             FROM branch
             WHERE branch_id = ?1",
            params![&branch_id.raw_bytes()[..]],
            |row| Ok((row.get::<_, Vec<u8>>(0)?, row.get::<_, Vec<u8>>(1)?)),
        )
        .optional()
        .map_err(storage_error)?;

    let Some((workspace_id, head_commit_id)) = row else {
        return Err(WorkVcsError::BranchNotFound(format!(
            "branch {branch_id} does not exist"
        )));
    };

    Ok(BranchRow {
        workspace_id: decode_workspace_id("branch.workspace_id", workspace_id)?,
        head_commit_id: decode_commit_id("branch.head_commit_id", head_commit_id)?,
    })
}

struct CommitRow {
    workspace_id: WorkspaceId,
    state_digest: Digest,
}

fn load_commit(transaction: &Transaction<'_>, commit_id: CommitId) -> Result<CommitRow> {
    let row = transaction
        .query_row(
            "SELECT workspace_id, state_digest
             FROM workstate_commit
             WHERE commit_id = ?1",
            params![&commit_id.raw_bytes()[..]],
            |row| Ok((row.get::<_, Vec<u8>>(0)?, row.get::<_, Vec<u8>>(1)?)),
        )
        .optional()
        .map_err(storage_error)?;

    let Some((workspace_id, state_digest)) = row else {
        return Err(WorkVcsError::CommitNotFound(format!(
            "commit {commit_id} does not exist"
        )));
    };

    Ok(CommitRow {
        workspace_id: decode_workspace_id("workstate_commit.workspace_id", workspace_id)?,
        state_digest: decode_digest("workstate_commit.state_digest", state_digest)?,
    })
}

fn ensure_branch_name_available(
    transaction: &Transaction<'_>,
    workspace_id: WorkspaceId,
    name: &str,
) -> Result<()> {
    let exists = transaction
        .query_row(
            "SELECT 1
             FROM branch
             WHERE workspace_id = ?1
               AND name = ?2",
            params![&workspace_id.raw_bytes()[..], name],
            |_| Ok(()),
        )
        .optional()
        .map_err(storage_error)?
        .is_some();
    if exists {
        return Err(WorkVcsError::WorkspaceInvalid(format!(
            "branch name {name:?} already exists in workspace {workspace_id}"
        )));
    }
    Ok(())
}

fn branch_forked_payload_json(
    workspace_id: WorkspaceId,
    branch_id: BranchId,
    name: &str,
    source_branch_id: Option<BranchId>,
    head_commit_id: CommitId,
) -> Result<String> {
    let source_branch_id = match source_branch_id {
        Some(source_branch_id) => CanonicalValue::String(source_branch_id.to_string()),
        None => CanonicalValue::Null,
    };
    canonical_json_string(&CanonicalValue::object(vec![
        (
            "branch_id".to_owned(),
            CanonicalValue::String(branch_id.to_string()),
        ),
        (
            "head_commit_id".to_owned(),
            CanonicalValue::String(head_commit_id.to_string()),
        ),
        ("name".to_owned(), CanonicalValue::String(name.to_owned())),
        ("source_branch_id".to_owned(), source_branch_id),
        (
            "workspace_id".to_owned(),
            CanonicalValue::String(workspace_id.to_string()),
        ),
    ])?)
}

fn canonical_json_string(value: &CanonicalValue) -> Result<String> {
    String::from_utf8(canonical_bytes(value)?).map_err(|error| {
        WorkVcsError::CanonicalEncodingInvalid(format!("canonical JSON was not UTF-8: {error}"))
    })
}

pub(crate) fn validate_branch_name(value: &str) -> Result<()> {
    if value.is_empty() {
        return Err(WorkVcsError::WorkspaceInvalid(
            "branch name must not be empty".to_owned(),
        ));
    }
    if value.trim() != value {
        return Err(WorkVcsError::WorkspaceInvalid(
            "branch name must not have leading or trailing whitespace".to_owned(),
        ));
    }
    if value
        .as_bytes()
        .iter()
        .any(|byte| *byte == b'\0' || *byte < 0x20 || *byte == 0x7f)
    {
        return Err(WorkVcsError::WorkspaceInvalid(
            "branch name must not contain NUL or ASCII control characters".to_owned(),
        ));
    }
    Ok(())
}

fn decode_workspace_id(column: &str, bytes: Vec<u8>) -> Result<WorkspaceId> {
    let bytes = bytes.try_into().map_err(|bytes: Vec<u8>| {
        WorkVcsError::WorkspaceInvalid(format!("{column} must be 16 bytes, found {}", bytes.len()))
    })?;
    WorkspaceId::from_bytes(bytes).map_err(|error| {
        WorkVcsError::WorkspaceInvalid(format!("{column} is not a valid WorkspaceId: {error}"))
    })
}

fn decode_commit_id(column: &str, bytes: Vec<u8>) -> Result<CommitId> {
    let bytes = bytes.try_into().map_err(|bytes: Vec<u8>| {
        WorkVcsError::ReplayInvalid(format!("{column} must be 16 bytes, found {}", bytes.len()))
    })?;
    CommitId::from_bytes(bytes).map_err(|error| {
        WorkVcsError::ReplayInvalid(format!("{column} is not a valid CommitId: {error}"))
    })
}

fn decode_digest(column: &str, bytes: Vec<u8>) -> Result<Digest> {
    let bytes = bytes.try_into().map_err(|bytes: Vec<u8>| {
        WorkVcsError::ReplayInvalid(format!("{column} must be 32 bytes, found {}", bytes.len()))
    })?;
    Ok(Digest::from_bytes(bytes))
}
