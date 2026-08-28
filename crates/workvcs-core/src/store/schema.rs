use crate::error::{Result, WorkVcsError, storage_error};
use crate::store::connection::StoreConnection;
use rusqlite::Connection;

pub(crate) const SCHEMA_SQL: &str = include_str!("../../../../schema/schema-v0.1.sql");

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

fn expected_schema_objects() -> Result<Vec<SchemaObject>> {
    let connection = Connection::open_in_memory().map_err(storage_error)?;
    connection
        .execute_batch(SCHEMA_SQL)
        .map_err(storage_error)?;
    schema_objects(&connection)
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
