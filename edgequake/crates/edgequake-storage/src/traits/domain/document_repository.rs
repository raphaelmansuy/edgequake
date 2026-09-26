//! SPEC-091 DocumentRepository port stub.

use async_trait::async_trait;

use crate::error::StorageError;

use super::types::{DocumentId, UnitOfWork};

/// Document lifecycle port (stub — expanded in later waves).
#[async_trait]
pub trait DocumentRepository: Send + Sync {
    async fn touch_indexed(
        &self,
        _tx: &mut UnitOfWork,
        _document_id: DocumentId,
    ) -> Result<(), StorageError> {
        Err(StorageError::UnsupportedCapability(
            "DocumentRepository::touch_indexed not implemented".into(),
        ))
    }
}
