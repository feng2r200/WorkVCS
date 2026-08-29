use crate::error::{Result, WorkVcsError, storage_error};
use crate::identity::{BranchId, CheckpointId, CommitId, EventId};
use crate::store::StoreConnection;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IntegrityReport {
    pub checked_branches: usize,
    pub checked_commits: usize,
    pub checked_events: usize,
    pub checked_checkpoints: usize,
    pub invalid_checkpoints: usize,
}

pub(crate) fn validate_integrity(connection: &StoreConnection) -> Result<IntegrityReport> {
    connection
        .verify_foreign_keys()
        .map_err(|error| integrity_error("foreign-key enforcement is unavailable", error))?;
    validate_sqlite_integrity(connection)?;
    validate_foreign_key_integrity(connection)?;

    let branch_ids = load_branch_ids(connection)?;
    for branch_id in &branch_ids {
        let head = super::branch_head(connection, *branch_id).map_err(|error| {
            integrity_error(format!("Branch {branch_id} head is invalid"), error)
        })?;
        let replayed = super::state_at(connection, head.head_commit_id).map_err(|error| {
            integrity_error(
                format!(
                    "Branch {branch_id} head {} cannot be replayed",
                    head.head_commit_id
                ),
                error,
            )
        })?;
        if replayed.workspace_id != head.workspace_id {
            return Err(WorkVcsError::IntegrityInvalid(format!(
                "Branch {branch_id} head {} replays workspace {}, not {}",
                head.head_commit_id, replayed.workspace_id, head.workspace_id
            )));
        }
        if replayed.state_digest != head.state_digest {
            return Err(WorkVcsError::IntegrityInvalid(format!(
                "Branch {branch_id} head {} digest does not match replay",
                head.head_commit_id
            )));
        }
    }

    let commit_ids = load_commit_ids(connection)?;
    for commit_id in &commit_ids {
        super::state_at(connection, *commit_id).map_err(|error| {
            integrity_error(format!("Commit {commit_id} cannot be replayed"), error)
        })?;
    }

    let event_ids = load_event_ids(connection)?;
    for event_id in &event_ids {
        super::event(connection, *event_id)
            .map_err(|error| integrity_error(format!("Event {event_id} is invalid"), error))?;
    }

    let checkpoint_statuses = load_checkpoint_statuses(connection)?;
    let invalid_checkpoints = checkpoint_statuses
        .iter()
        .filter(|status| status.usability_state == "invalid")
        .count();

    Ok(IntegrityReport {
        checked_branches: branch_ids.len(),
        checked_commits: commit_ids.len(),
        checked_events: event_ids.len(),
        checked_checkpoints: checkpoint_statuses.len(),
        invalid_checkpoints,
    })
}

fn validate_sqlite_integrity(connection: &StoreConnection) -> Result<()> {
    let result = connection
        .inner()
        .query_row("PRAGMA integrity_check", [], |row| row.get::<_, String>(0))
        .map_err(storage_error)?;
    if result == "ok" {
        Ok(())
    } else {
        Err(WorkVcsError::IntegrityInvalid(format!(
            "SQLite integrity_check failed: {result}"
        )))
    }
}

fn validate_foreign_key_integrity(connection: &StoreConnection) -> Result<()> {
    let mut statement = connection
        .inner()
        .prepare("PRAGMA foreign_key_check")
        .map_err(storage_error)?;
    let mut rows = statement.query([]).map_err(storage_error)?;
    if let Some(row) = rows.next().map_err(storage_error)? {
        let table = row.get::<_, String>(0).map_err(storage_error)?;
        let rowid = row.get::<_, i64>(1).map_err(storage_error)?;
        let parent = row.get::<_, String>(2).map_err(storage_error)?;
        return Err(WorkVcsError::IntegrityInvalid(format!(
            "foreign key violation in {table} row {rowid} referencing {parent}"
        )));
    }
    Ok(())
}

fn load_branch_ids(connection: &StoreConnection) -> Result<Vec<BranchId>> {
    let mut statement = connection
        .inner()
        .prepare("SELECT branch_id FROM branch ORDER BY branch_id")
        .map_err(storage_error)?;
    let rows = statement
        .query_map([], |row| row.get::<_, Vec<u8>>(0))
        .map_err(storage_error)?;
    let mut branch_ids = Vec::new();
    for row in rows {
        branch_ids.push(decode_branch_id(
            "branch.branch_id",
            row.map_err(storage_error)?,
        )?);
    }
    Ok(branch_ids)
}

fn load_commit_ids(connection: &StoreConnection) -> Result<Vec<CommitId>> {
    let mut statement = connection
        .inner()
        .prepare("SELECT commit_id FROM workstate_commit ORDER BY committed_at_us, commit_id")
        .map_err(storage_error)?;
    let rows = statement
        .query_map([], |row| row.get::<_, Vec<u8>>(0))
        .map_err(storage_error)?;
    let mut commit_ids = Vec::new();
    for row in rows {
        commit_ids.push(decode_commit_id(
            "workstate_commit.commit_id",
            row.map_err(storage_error)?,
        )?);
    }
    Ok(commit_ids)
}

fn load_event_ids(connection: &StoreConnection) -> Result<Vec<EventId>> {
    let mut statement = connection
        .inner()
        .prepare("SELECT event_id FROM event ORDER BY occurred_at_us, event_id")
        .map_err(storage_error)?;
    let rows = statement
        .query_map([], |row| row.get::<_, Vec<u8>>(0))
        .map_err(storage_error)?;
    let mut event_ids = Vec::new();
    for row in rows {
        event_ids.push(decode_event_id(
            "event.event_id",
            row.map_err(storage_error)?,
        )?);
    }
    Ok(event_ids)
}

struct CheckpointStatusRow {
    usability_state: String,
}

fn load_checkpoint_statuses(connection: &StoreConnection) -> Result<Vec<CheckpointStatusRow>> {
    let mut statement = connection
        .inner()
        .prepare(
            "SELECT checkpoint.checkpoint_id,
                    checkpoint_status.usability_state
             FROM checkpoint
             LEFT JOIN checkpoint_status
               ON checkpoint_status.checkpoint_id = checkpoint.checkpoint_id
             ORDER BY checkpoint.created_at_us, checkpoint.checkpoint_id",
        )
        .map_err(storage_error)?;
    let rows = statement
        .query_map([], |row| {
            Ok((row.get::<_, Vec<u8>>(0)?, row.get::<_, Option<String>>(1)?))
        })
        .map_err(storage_error)?;

    let mut statuses = Vec::new();
    for row in rows {
        let (checkpoint_id, usability_state) = row.map_err(storage_error)?;
        let checkpoint_id = decode_checkpoint_id("checkpoint.checkpoint_id", checkpoint_id)?;
        let Some(usability_state) = usability_state else {
            return Err(WorkVcsError::IntegrityInvalid(format!(
                "Checkpoint {checkpoint_id} is missing checkpoint_status"
            )));
        };
        validate_stored_text("checkpoint_status.usability_state", &usability_state)?;
        statuses.push(CheckpointStatusRow { usability_state });
    }
    Ok(statuses)
}

fn decode_branch_id(column: &str, bytes: Vec<u8>) -> Result<BranchId> {
    let bytes = decode_16(column, bytes)?;
    BranchId::from_bytes(bytes).map_err(|error| {
        WorkVcsError::IntegrityInvalid(format!("{column} is not a UUIDv7 value: {error}"))
    })
}

fn decode_checkpoint_id(column: &str, bytes: Vec<u8>) -> Result<CheckpointId> {
    let bytes = decode_16(column, bytes)?;
    CheckpointId::from_bytes(bytes).map_err(|error| {
        WorkVcsError::IntegrityInvalid(format!("{column} is not a UUIDv7 value: {error}"))
    })
}

fn decode_commit_id(column: &str, bytes: Vec<u8>) -> Result<CommitId> {
    let bytes = decode_16(column, bytes)?;
    CommitId::from_bytes(bytes).map_err(|error| {
        WorkVcsError::IntegrityInvalid(format!("{column} is not a UUIDv7 value: {error}"))
    })
}

fn decode_event_id(column: &str, bytes: Vec<u8>) -> Result<EventId> {
    let bytes = decode_16(column, bytes)?;
    EventId::from_bytes(bytes).map_err(|error| {
        WorkVcsError::IntegrityInvalid(format!("{column} is not a UUIDv7 value: {error}"))
    })
}

fn validate_stored_text(label: &str, value: &str) -> Result<()> {
    if value.is_empty() {
        return Err(WorkVcsError::IntegrityInvalid(format!(
            "{label} cannot be empty"
        )));
    }
    if value.chars().any(char::is_control) {
        return Err(WorkVcsError::IntegrityInvalid(format!(
            "{label} cannot contain control characters"
        )));
    }
    Ok(())
}

fn decode_16(column: &str, bytes: Vec<u8>) -> Result<[u8; 16]> {
    bytes.try_into().map_err(|bytes: Vec<u8>| {
        WorkVcsError::IntegrityInvalid(format!("{column} must be 16 bytes, found {}", bytes.len()))
    })
}

fn integrity_error(context: impl AsRef<str>, error: WorkVcsError) -> WorkVcsError {
    WorkVcsError::IntegrityInvalid(format!("{}: {error}", context.as_ref()))
}
