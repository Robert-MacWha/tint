use std::{collections::HashMap, sync::Mutex};

use crate::database::{Database, DatabaseError};

/// Basic in-memory KV database implementation.
#[derive(Default)]
pub struct MemoryDatabase {
    store: Mutex<HashMap<Vec<u8>, Vec<u8>>>,
}

impl MemoryDatabase {
    #[must_use]
    pub fn new() -> Self {
        Self {
            store: Mutex::new(HashMap::new()),
        }
    }
}

#[async_trait::async_trait]
impl Database for MemoryDatabase {
    async fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>, DatabaseError> {
        let store = self
            .store
            .lock()
            .map_err(|_| DatabaseError::storage("Failed to acquire lock"))?;
        Ok(store.get(key).cloned())
    }

    async fn set(&self, key: &[u8], value: &[u8]) -> Result<(), DatabaseError> {
        let mut store = self
            .store
            .lock()
            .map_err(|_| DatabaseError::storage("Failed to acquire lock"))?;
        store.insert(key.to_vec(), value.to_vec());
        Ok(())
    }

    async fn delete(&self, key: &[u8]) -> Result<(), DatabaseError> {
        let mut store = self
            .store
            .lock()
            .map_err(|_| DatabaseError::storage("Failed to acquire lock"))?;
        store.remove(key);
        Ok(())
    }
}
