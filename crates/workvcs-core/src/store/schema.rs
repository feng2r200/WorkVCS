use crate::error::{Result, WorkVcsError, storage_error};
use crate::store::connection::StoreConnection;
use rusqlite::{Connection, TransactionBehavior};

pub(crate) const SCHEMA_SQL: &str = include_str!("../../../../schema/schema-v0.1.sql");
const CONTEXT_PACKET_SNAPSHOT_OBJECTS: &[(&str, &str)] = &[
    ("table", "context_packet_snapshot"),
    ("index", "idx_context_packet_snapshot_branch_head"),
    ("index", "idx_context_packet_snapshot_session_created"),
];

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ContextPacketSnapshotSchemaMigration {
    pub(crate) added_schema_objects: Vec<String>,
}

pub(crate) fn install(connection: &StoreConnection) -> Result<()> {
    connection.execute_batch(SCHEMA_SQL)
}

pub(crate) fn validate_installed_schema(connection: &StoreConnection) -> Result<()> {
    let actual = schema_objects(connection.inner())?;
    let expected = expected_schema_objects()?;
    if actual == expected {
        Ok(())
    } else {
        Err(WorkVcsError::StoreBootstrapInvalid(format!(
            "installed schema does not match frozen schema-v0.1 object set: {}",
            schema_mismatch_summary(&actual, &expected)
        )))
    }
}

pub(crate) fn migrate_context_packet_snapshot_schema(
    connection: &mut StoreConnection,
) -> Result<ContextPacketSnapshotSchemaMigration> {
    connection.verify_foreign_keys()?;
    let actual = schema_objects(connection.inner())?;
    let expected = expected_schema_objects()?;
    if actual == expected {
        return Ok(ContextPacketSnapshotSchemaMigration {
            added_schema_objects: Vec::new(),
        });
    }

    let actual_without_context_packet = actual
        .iter()
        .filter(|object| !is_context_packet_snapshot_object(object))
        .cloned()
        .collect::<Vec<_>>();
    let expected_without_context_packet = expected
        .iter()
        .filter(|object| !is_context_packet_snapshot_object(object))
        .cloned()
        .collect::<Vec<_>>();
    if actual_without_context_packet != expected_without_context_packet {
        return Err(WorkVcsError::StoreBootstrapInvalid(format!(
            "cannot migrate context packet snapshot schema because the installed schema is not a recognized pre-4LS schema: {}",
            schema_mismatch_summary(&actual, &expected)
        )));
    }

    for object in actual
        .iter()
        .filter(|object| is_context_packet_snapshot_object(object))
    {
        let Some(expected_object) = expected.iter().find(|candidate| {
            candidate.object_type == object.object_type && candidate.name == object.name
        }) else {
            return Err(WorkVcsError::StoreBootstrapInvalid(format!(
                "cannot migrate context packet snapshot schema because installed schema contains unexpected {} {}",
                object.object_type, object.name
            )));
        };
        if object != expected_object {
            return Err(WorkVcsError::StoreBootstrapInvalid(format!(
                "cannot migrate context packet snapshot schema because installed {} {} does not match schema-v0.1",
                object.object_type, object.name
            )));
        }
    }

    let missing = expected
        .iter()
        .filter(|object| is_context_packet_snapshot_object(object))
        .filter(|object| {
            !actual.iter().any(|candidate| {
                candidate.object_type == object.object_type && candidate.name == object.name
            })
        })
        .cloned()
        .collect::<Vec<_>>();

    if missing.is_empty() {
        return Ok(ContextPacketSnapshotSchemaMigration {
            added_schema_objects: Vec::new(),
        });
    }

    let transaction = connection
        .inner_mut()
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(storage_error)?;
    for object in &missing {
        transaction
            .execute_batch(&object.sql)
            .map_err(storage_error)?;
    }
    transaction.commit().map_err(storage_error)?;

    let added_schema_objects = missing
        .into_iter()
        .map(|object| format!("{}:{}", object.object_type, object.name))
        .collect();
    Ok(ContextPacketSnapshotSchemaMigration {
        added_schema_objects,
    })
}

fn expected_schema_objects() -> Result<Vec<SchemaObject>> {
    let connection = Connection::open_in_memory().map_err(storage_error)?;
    connection
        .execute_batch(SCHEMA_SQL)
        .map_err(storage_error)?;
    schema_objects(&connection)
}

fn is_context_packet_snapshot_object(object: &SchemaObject) -> bool {
    CONTEXT_PACKET_SNAPSHOT_OBJECTS
        .iter()
        .any(|(object_type, name)| object.object_type == *object_type && object.name == *name)
}

fn schema_objects(connection: &Connection) -> Result<Vec<SchemaObject>> {
    let mut statement = connection
        .prepare(
            "SELECT type, name, COALESCE(sql, '')
             FROM sqlite_schema
             WHERE name NOT LIKE 'sqlite_%'
               AND type IN ('table', 'index', 'view', 'trigger')
             ORDER BY
               CASE type
                 WHEN 'table' THEN 0
                 WHEN 'index' THEN 1
                 WHEN 'view' THEN 2
                 WHEN 'trigger' THEN 3
                 ELSE 4
               END,
               name",
        )
        .map_err(storage_error)?;
    let rows = statement
        .query_map([], |row| {
            Ok(SchemaObject {
                object_type: row.get(0)?,
                name: row.get(1)?,
                sql: row.get(2)?,
            })
        })
        .map_err(storage_error)?;

    let mut objects = Vec::new();
    for row in rows {
        objects.push(row.map_err(storage_error)?);
    }
    Ok(objects)
}

fn schema_mismatch_summary(actual: &[SchemaObject], expected: &[SchemaObject]) -> String {
    if actual.len() != expected.len() {
        return format!(
            "expected {} schema objects, found {}",
            expected.len(),
            actual.len()
        );
    }
    match actual
        .iter()
        .zip(expected)
        .enumerate()
        .find(|(_, (actual, expected))| actual != expected)
    {
        Some((index, (actual, expected))) => format!(
            "first mismatch at object #{index}: expected {} {}, found {} {}",
            expected.object_type, expected.name, actual.object_type, actual.name
        ),
        None => "unknown schema mismatch".to_owned(),
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct SchemaObject {
    object_type: String,
    name: String,
    sql: String,
}
