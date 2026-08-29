use crate::error::{Result, WorkVcsError, storage_error};
use crate::identity::KnowledgeSpaceId;
use crate::store::{StoreConnection, current_epoch_micros};
use rusqlite::{OptionalExtension, Transaction, TransactionBehavior, params};

const KNOWLEDGE_SPACE_OBJECT_KIND: &str = "knowledge_space";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KnowledgeSpaceCreateOptions {
    name: String,
}

impl KnowledgeSpaceCreateOptions {
    pub fn new(name: impl Into<String>) -> Result<Self> {
        let name = name.into();
        validate_knowledge_space_name(&name)?;
        Ok(Self { name })
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KnowledgeSpaceCreateResult {
    pub knowledge_space: KnowledgeSpaceSnapshot,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KnowledgeSpaceSnapshot {
    pub knowledge_space_id: KnowledgeSpaceId,
    pub name: String,
    pub created_at_us: i64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KnowledgeSpaceListOptions {
    limit: usize,
}

impl KnowledgeSpaceListOptions {
    pub fn new() -> Self {
        Self { limit: 50 }
    }

    pub fn with_limit(mut self, limit: usize) -> Result<Self> {
        if limit == 0 {
            return Err(WorkVcsError::QueryInvalid(
                "knowledge space list limit must be positive".to_owned(),
            ));
        }
        self.limit = limit;
        Ok(self)
    }

    fn limit(self) -> usize {
        self.limit
    }
}

impl Default for KnowledgeSpaceListOptions {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KnowledgeSpaceListResult {
    pub knowledge_spaces: Vec<KnowledgeSpaceSnapshot>,
}

pub(crate) fn create_knowledge_space(
    connection: &mut StoreConnection,
    options: &KnowledgeSpaceCreateOptions,
) -> Result<KnowledgeSpaceCreateResult> {
    connection.verify_foreign_keys()?;
    validate_knowledge_space_name(options.name())?;
    let created_at_us = current_epoch_micros()?;
    let knowledge_space_id = KnowledgeSpaceId::new_v7();

    let transaction = connection
        .inner_mut()
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(storage_error)?;
    ensure_knowledge_space_name_available(&transaction, options.name())?;

    let knowledge_space_id_bytes = knowledge_space_id.raw_bytes();
    transaction
        .execute(
            "INSERT INTO object_identity(object_id, object_kind, created_at_us)
             VALUES (?1, ?2, ?3)",
            params![
                &knowledge_space_id_bytes[..],
                KNOWLEDGE_SPACE_OBJECT_KIND,
                created_at_us
            ],
        )
        .map_err(storage_error)?;
    transaction
        .execute(
            "INSERT INTO knowledge_space(knowledge_space_id, name, created_at_us)
             VALUES (?1, ?2, ?3)",
            params![&knowledge_space_id_bytes[..], options.name(), created_at_us],
        )
        .map_err(storage_error)?;
    transaction.commit().map_err(storage_error)?;

    Ok(KnowledgeSpaceCreateResult {
        knowledge_space: KnowledgeSpaceSnapshot {
            knowledge_space_id,
            name: options.name().to_owned(),
            created_at_us,
        },
    })
}

pub(crate) fn knowledge_space(
    connection: &StoreConnection,
    knowledge_space_id: KnowledgeSpaceId,
) -> Result<KnowledgeSpaceSnapshot> {
    connection.verify_foreign_keys()?;
    load_knowledge_space_snapshot(connection, knowledge_space_id)?.ok_or_else(|| {
        WorkVcsError::QueryInvalid(format!("KnowledgeSpace {knowledge_space_id} not found"))
    })
}

pub(crate) fn knowledge_spaces(
    connection: &StoreConnection,
    options: KnowledgeSpaceListOptions,
) -> Result<KnowledgeSpaceListResult> {
    connection.verify_foreign_keys()?;
    let limit = usize_to_i64("knowledge space list limit", options.limit())?;
    let mut statement = connection
        .inner()
        .prepare(
            "SELECT knowledge_space_id
             FROM knowledge_space
             ORDER BY created_at_us DESC, knowledge_space_id DESC
             LIMIT ?1",
        )
        .map_err(storage_error)?;
    let rows = statement
        .query_map(params![limit], |row| row.get::<_, Vec<u8>>(0))
        .map_err(storage_error)?;

    let mut knowledge_spaces = Vec::new();
    for row in rows {
        let knowledge_space_id = decode_knowledge_space_id(
            "knowledge_space.knowledge_space_id",
            row.map_err(storage_error)?,
        )?;
        knowledge_spaces.push(knowledge_space(connection, knowledge_space_id)?);
    }
    Ok(KnowledgeSpaceListResult { knowledge_spaces })
}

fn ensure_knowledge_space_name_available(transaction: &Transaction<'_>, name: &str) -> Result<()> {
    let existing = transaction
        .query_row(
            "SELECT knowledge_space_id
             FROM knowledge_space
             WHERE name = ?1",
            params![name],
            |row| row.get::<_, Vec<u8>>(0),
        )
        .optional()
        .map_err(storage_error)?;
    if existing.is_some() {
        Err(WorkVcsError::QueryInvalid(format!(
            "knowledge space name {name:?} already exists"
        )))
    } else {
        Ok(())
    }
}

fn load_knowledge_space_snapshot(
    connection: &StoreConnection,
    knowledge_space_id: KnowledgeSpaceId,
) -> Result<Option<KnowledgeSpaceSnapshot>> {
    let row = connection
        .inner()
        .query_row(
            "SELECT object_identity.object_kind,
                    knowledge_space.name,
                    knowledge_space.created_at_us
             FROM knowledge_space
             JOIN object_identity
               ON object_identity.object_id = knowledge_space.knowledge_space_id
             WHERE knowledge_space.knowledge_space_id = ?1",
            params![&knowledge_space_id.raw_bytes()[..]],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                ))
            },
        )
        .optional()
        .map_err(storage_error)?;
    let Some((object_kind, name, created_at_us)) = row else {
        return Ok(None);
    };
    if object_kind != KNOWLEDGE_SPACE_OBJECT_KIND {
        return Err(WorkVcsError::QueryInvalid(format!(
            "knowledge space {knowledge_space_id} has object kind {object_kind:?}"
        )));
    }
    validate_knowledge_space_name(&name)?;
    validate_positive_i64("knowledge_space.created_at_us", created_at_us)?;
    Ok(Some(KnowledgeSpaceSnapshot {
        knowledge_space_id,
        name,
        created_at_us,
    }))
}

fn validate_knowledge_space_name(value: &str) -> Result<()> {
    if value.is_empty() {
        return Err(WorkVcsError::QueryInvalid(
            "knowledge space name must not be empty".to_owned(),
        ));
    }
    if value.trim() != value {
        return Err(WorkVcsError::QueryInvalid(
            "knowledge space name must not have leading or trailing whitespace".to_owned(),
        ));
    }
    if value
        .as_bytes()
        .iter()
        .any(|byte| *byte == b'\0' || *byte < 0x20 || *byte == 0x7f)
    {
        return Err(WorkVcsError::QueryInvalid(
            "knowledge space name must not contain NUL or ASCII control characters".to_owned(),
        ));
    }
    Ok(())
}

fn validate_positive_i64(label: &str, value: i64) -> Result<()> {
    if value <= 0 {
        return Err(WorkVcsError::QueryInvalid(format!(
            "{label} must be positive"
        )));
    }
    Ok(())
}

fn usize_to_i64(label: &str, value: usize) -> Result<i64> {
    i64::try_from(value)
        .map_err(|_| WorkVcsError::QueryInvalid(format!("{label} does not fit i64")))
}

fn decode_knowledge_space_id(column: &str, bytes: Vec<u8>) -> Result<KnowledgeSpaceId> {
    let bytes: [u8; 16] = bytes.try_into().map_err(|bytes: Vec<u8>| {
        WorkVcsError::QueryInvalid(format!("{column} must be 16 bytes, found {}", bytes.len()))
    })?;
    KnowledgeSpaceId::from_bytes(bytes)
        .map_err(|error| WorkVcsError::QueryInvalid(format!("{column}: {error}")))
}
