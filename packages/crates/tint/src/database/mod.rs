use crate::{
    account::keys::{EncryptionPubKey, NullifierPubKey},
    indexer::{IndexerState, indexed_account::IndexedAccountState},
};

pub mod memory;

/// Key-value async database interface.
#[async_trait::async_trait]
pub trait Database {
    async fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>, DatabaseError>;
    async fn set(&self, key: &[u8], value: &[u8]) -> Result<(), DatabaseError>;
    async fn delete(&self, key: &[u8]) -> Result<(), DatabaseError>;
}

#[derive(Debug, thiserror::Error)]
pub enum DatabaseError {
    #[error("Unsupported version: {0}")]
    UnsupportedVersion(u32),
    #[error("Storage error: {0}")]
    StorageError(String),
    #[error("Other error: {0}")]
    Other(#[source] Box<dyn std::error::Error + Send + Sync + 'static>),
}

pub(crate) trait TintDatabase: Database {
    async fn set_indexer(&self, state: &IndexerState) -> Result<(), DatabaseError>;
    async fn load_indexer(&self) -> Result<Option<IndexerState>, DatabaseError>;

    async fn set_indexed_account(
        &self,
        nullifier_pub: NullifierPubKey,
        encryption_pub: EncryptionPubKey,
        state: &IndexedAccountState,
    ) -> Result<(), DatabaseError>;
    async fn load_indexed_account(
        &self,
        nullifier_pub: NullifierPubKey,
        encryption_pub: EncryptionPubKey,
    ) -> Result<Option<IndexedAccountState>, DatabaseError>;
}

impl DatabaseError {
    pub fn other(e: impl std::error::Error + Send + Sync + 'static) -> Self {
        DatabaseError::Other(Box::new(e))
    }
}

impl<T: ?Sized + Database> TintDatabase for T {
    async fn set_indexer(&self, state: &IndexerState) -> Result<(), DatabaseError> {
        let serialized = postcard::to_stdvec(state).map_err(DatabaseError::other)?;
        self.set(b"indexer", &serialized).await
    }

    async fn load_indexer(&self) -> Result<Option<IndexerState>, DatabaseError> {
        let Some(serialized) = self.get(b"indexer").await? else {
            return Ok(None);
        };
        let state = postcard::from_bytes(&serialized).map_err(DatabaseError::other)?;
        Ok(Some(state))
    }

    async fn set_indexed_account(
        &self,
        nullifier_pub: NullifierPubKey,
        encryption_pub: EncryptionPubKey,
        state: &IndexedAccountState,
    ) -> Result<(), DatabaseError> {
        let key = indexed_account_key(nullifier_pub, encryption_pub)?;

        let serialized = postcard::to_stdvec(state).map_err(DatabaseError::other)?;
        self.set(&key, &serialized).await
    }

    async fn load_indexed_account(
        &self,
        nullifier_pub: NullifierPubKey,
        encryption_pub: EncryptionPubKey,
    ) -> Result<Option<IndexedAccountState>, DatabaseError> {
        let key = indexed_account_key(nullifier_pub, encryption_pub)?;

        let Some(serialized) = self.get(&key).await? else {
            return Ok(None);
        };
        let state = postcard::from_bytes(&serialized).map_err(DatabaseError::other)?;
        Ok(Some(state))
    }
}

/// Derives the database key for an account's indexed state from its viewing
/// and nullifying identity.
fn indexed_account_key(
    nullifier_pub_key: NullifierPubKey,
    encryption_pub_key: EncryptionPubKey,
) -> Result<Vec<u8>, DatabaseError> {
    postcard::to_stdvec(&(nullifier_pub_key, encryption_pub_key)).map_err(DatabaseError::other)
}
