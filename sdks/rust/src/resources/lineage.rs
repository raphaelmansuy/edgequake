//! Lineage resource — dedicated entity/document lineage + export.
//!
//! WHY: Separates lineage-specific routes (under /api/v1/lineage/) from
//! provenance and per-document lineage. Added in OODA-31.

use crate::client::EdgeQuakeClient;
use crate::error::Result;
use crate::types::operations::{DocumentFullLineage, LineageGraph};

pub struct LineageResource<'a> {
    pub(crate) client: &'a EdgeQuakeClient,
}

impl<'a> LineageResource<'a> {
    /// `GET /api/v1/lineage/entities/{entity_name}`
    ///
    /// Returns the lineage graph for a named entity.
    pub async fn entity_lineage(&self, entity_name: &str) -> Result<LineageGraph> {
        self.client
            .get(&format!(
                "/api/v1/lineage/entities/{}",
                urlencoding::encode(entity_name)
            ))
            .await
    }

    /// `GET /api/v1/lineage/documents/{document_id}`
    ///
    /// Returns the lineage graph for a document.
    pub async fn document_lineage(&self, document_id: &str) -> Result<LineageGraph> {
        self.client
            .get(&format!("/api/v1/lineage/documents/{document_id}"))
            .await
    }

    /// `GET /api/v1/documents/{id}/lineage`
    ///
    /// Returns complete document lineage with metadata.
    pub async fn document_full_lineage(
        &self,
        document_id: &str,
    ) -> Result<DocumentFullLineage> {
        self.client
            .get(&format!("/api/v1/documents/{document_id}/lineage"))
            .await
    }

    /// `GET /api/v1/documents/{id}/lineage/export?format={format}`
    ///
    /// Exports lineage as JSON or CSV (returns raw bytes).
    pub async fn export_lineage(
        &self,
        document_id: &str,
        format: &str,
    ) -> Result<Vec<u8>> {
        self.client
            .get_raw(&format!(
                "/api/v1/documents/{document_id}/lineage/export?format={format}"
            ))
            .await
    }
}
