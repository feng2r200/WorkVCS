use crate::error::Result;
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
}
