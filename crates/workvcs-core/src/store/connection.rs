use crate::error::{Result, WorkVcsError, storage_error};
use rusqlite::Connection;
use std::path::Path;

pub(crate) struct StoreConnection {
    connection: Connection,
}

impl StoreConnection {
    pub(crate) fn open(path: &Path) -> Result<Self> {
        let connection = Connection::open(path).map_err(storage_error)?;
        let handle = Self { connection };
        handle.enable_foreign_keys()?;
        handle.verify_foreign_keys()?;
        Ok(handle)
    }

    pub(crate) fn execute_batch(&self, sql: &str) -> Result<()> {
        self.connection.execute_batch(sql).map_err(storage_error)
    }

    pub(crate) fn inner(&self) -> &Connection {
        &self.connection
    }

    pub(crate) fn inner_mut(&mut self) -> &mut Connection {
        &mut self.connection
    }

    pub(crate) fn foreign_keys_enabled(&self) -> Result<bool> {
        let enabled = self
            .connection
            .pragma_query_value(None, "foreign_keys", |row| row.get::<_, i64>(0))
            .map_err(storage_error)?;
        Ok(enabled == 1)
    }

    fn enable_foreign_keys(&self) -> Result<()> {
        self.connection
            .pragma_update(None, "foreign_keys", "ON")
            .map_err(storage_error)
    }

    fn verify_foreign_keys(&self) -> Result<()> {
        if self.foreign_keys_enabled()? {
            Ok(())
        } else {
            Err(WorkVcsError::StoreBootstrapInvalid(
                "SQLite foreign-key enforcement is not enabled for this Engine connection"
                    .to_owned(),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::StoreConnection;

    #[test]
    fn opens_with_foreign_keys_enabled() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let path = tempdir.path().join("store.sqlite");
        let connection = StoreConnection::open(&path).expect("connection");

        assert!(
            connection
                .foreign_keys_enabled()
                .expect("foreign key pragma")
        );
    }
}
