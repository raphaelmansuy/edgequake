//! Provenance resource.

use crate::client::EdgeQuakeClient;
use crate::error::Result;
use crate::types::operations::{LineageGraph, ProvenanceRecord};

pub struct ProvenanceResource<'a> {
    pub(crate) client: &'a EdgeQuakeClient,
}

impl<'a> ProvenanceResource<'a> {
    /// `GET /api/v1/entities/{name}/provenance`
    pub async fn for_entity(&self, entity_name: &str) -> Result<Vec<ProvenanceRecord>> {
        self.client
            .get(&format!(
                "/api/v1/entities/{}/provenance",
                urlencoding::encode(entity_name)
            ))
            .await
    }

    /// `GET /api/v1/lineage/entities/{name}`
    ///
    /// WHY: Route is under /lineage/ prefix, not /entities/. Fixed in OODA-31.
    pub async fn lineage(&self, entity_name: &str) -> Result<LineageGraph> {
        self.client
            .get(&format!(
                "/api/v1/lineage/entities/{}",
                urlencoding::encode(entity_name)
            ))
            .await
    }
}
