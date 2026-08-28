use crate::error::Result;
use crate::store::bootstrap::{
    StoreInfo, StoreInitOptions, ensure_empty_database, initialize_manifest, validate_bootstrap,
};
use crate::store::connection::StoreConnection;
use crate::store::schema;
use std::path::Path;

pub(crate) struct Store {
    connection: StoreConnection,
    info: StoreInfo,
}

impl Store {
    pub(crate) fn init(path: &Path, options: StoreInitOptions) -> Result<Self> {
        let mut connection = StoreConnection::open(path)?;
        ensure_empty_database(&connection)?;
        schema::install(&connection)?;
        let info = initialize_manifest(&mut connection, &options)?;
        let info = validate_bootstrap(&connection).map(|validated| {
            debug_assert_eq!(validated, info);
            validated
        })?;
        Ok(Self { connection, info })
    }

    pub(crate) fn open(path: &Path) -> Result<Self> {
        let connection = StoreConnection::open(path)?;
        let info = validate_bootstrap(&connection)?;
        Ok(Self { connection, info })
    }

    pub(crate) fn info(&self) -> Result<StoreInfo> {
        let current = validate_bootstrap(&self.connection)?;
        debug_assert_eq!(current, self.info);
        Ok(current)
    }
}
