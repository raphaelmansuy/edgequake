//! Chunks resource.

use crate::client::EdgeQuakeClient;
use crate::error::Result;
use crate::types::operations::{ChunkDetail, ChunkLineageInfo};

pub struct ChunksResource<'a> {
    pub(crate) client: &'a EdgeQuakeClient,
}

impl<'a> ChunksResource<'a> {
    /// `GET /api/v1/documents/{doc_id}/chunks`
    pub async fn list(&self, document_id: &str) -> Result<Vec<ChunkDetail>> {
        self.client
            .get(&format!("/api/v1/documents/{document_id}/chunks"))
            .await
    }

    /// `GET /api/v1/chunks/{id}`
    pub async fn get(&self, id: &str) -> Result<ChunkDetail> {
        self.client.get(&format!("/api/v1/chunks/{id}")).await
    }

    /// `GET /api/v1/chunks/{id}/lineage`
    ///
    /// Returns chunk lineage with parent document refs and position info.
    /// @implements F3 — Every chunk contains parent_document_id and position info.
    /// @implements F8 — PDF → Document → Chunk → Entity chain traceable.
    pub async fn get_lineage(&self, id: &str) -> Result<ChunkLineageInfo> {
        self.client
            .get(&format!("/api/v1/chunks/{id}/lineage"))
            .await
    }
}
