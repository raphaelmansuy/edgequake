use edgequake_storage_contracts::{
    AccessError, AccessResult, AccessScope, DocumentId, EmbeddingKey, VectorModelDescriptor,
};
use serde::Serialize;
use uuid::Uuid;

use super::client::QdrantClient;
use super::errors::parse_completed_operation;

const DELETE_BATCH_SIZE: usize = 256;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct QdrantPointPayload {
    pub tenant_id: Uuid,
    pub workspace_id: Uuid,
    pub family: String,
    pub subject_id: Uuid,
    pub document_id: DocumentId,
    pub contribution_ref: String,
    pub model_revision: String,
    pub content_revision: u64,
    pub manifest_id: Uuid,
    pub digest: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modality: Option<String>,
}

/// One immutable physical embedding revision.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct QdrantVectorPoint {
    /// Persisted physical revision UUID; this is the Qdrant point ID.
    pub key: EmbeddingKey,
    pub vector: Vec<f32>,
    pub payload: QdrantPointPayload,
}

#[derive(Debug, Serialize)]
struct UpsertBody<'a> {
    points: Vec<UpsertPoint<'a>>,
}

#[derive(Debug, Serialize)]
struct UpsertPoint<'a> {
    id: Uuid,
    vector: &'a [f32],
    payload: StoredPayload<'a>,
}

#[derive(Debug, Serialize)]
struct StoredPayload<'a> {
    #[serde(flatten)]
    payload: &'a QdrantPointPayload,
    physical_revision_id: Uuid,
}

/// Provider IDs include the mandatory scope, so a forged foreign revision ID cannot overwrite a point.
pub(super) fn scoped_point_id(scope: &AccessScope, revision: Uuid) -> Uuid {
    Uuid::new_v5(
        &Uuid::NAMESPACE_OID,
        format!(
            "edgequake:qdrant:{}:{}:{revision}",
            scope.tenant(),
            scope.workspace()
        )
        .as_bytes(),
    )
}

impl QdrantClient {
    /// Upsert immutable physical revisions and return a completed provider receipt.
    pub async fn upsert_points(
        &self,
        scope: &AccessScope,
        model: &VectorModelDescriptor,
        points: &[QdrantVectorPoint],
    ) -> AccessResult<Option<String>> {
        if points.is_empty() {
            return Ok(None);
        }
        for point in points {
            validate_point(model, point)?;
            if point.payload.tenant_id != scope.tenant().into_uuid()
                || point.payload.workspace_id != scope.workspace().into_uuid()
            {
                return Err(AccessError::InvalidInput(
                    "Qdrant point payload does not match mutation scope".into(),
                ));
            }
        }
        let body = UpsertBody {
            points: points
                .iter()
                .map(|point| UpsertPoint {
                    id: scoped_point_id(scope, point.key.into_uuid()),
                    vector: &point.vector,
                    payload: StoredPayload {
                        payload: &point.payload,
                        physical_revision_id: point.key.into_uuid(),
                    },
                })
                .collect(),
        };
        let path = format!("/collections/{}/points?wait=true", self.collection_name());
        let bytes = self.put_json(&path, &body).await?;
        parse_completed_operation(&bytes).map(Some)
    }

    /// Delete exact physical revision UUIDs in bounded batches.
    pub async fn delete_points(
        &self,
        scope: &AccessScope,
        point_ids: &[Uuid],
    ) -> AccessResult<Vec<String>> {
        let mut receipts = Vec::new();
        for batch in point_ids.chunks(DELETE_BATCH_SIZE) {
            let ids: Vec<Uuid> = batch.iter().map(|id| scoped_point_id(scope, *id)).collect();
            let body = serde_json::json!({ "filter": { "must": [
                { "has_id": ids },
                { "key": "tenant_id", "match": { "value": scope.tenant().to_string() } },
                { "key": "workspace_id", "match": { "value": scope.workspace().to_string() } }
            ] }});
            let path = format!(
                "/collections/{}/points/delete?wait=true",
                self.collection_name()
            );
            let bytes = self.post_json(&path, &body, None).await?;
            receipts.push(parse_completed_operation(&bytes)?);
        }
        Ok(receipts)
    }
}

fn validate_point(model: &VectorModelDescriptor, point: &QdrantVectorPoint) -> AccessResult<()> {
    if point.vector.len() != model.dimensions as usize {
        return Err(AccessError::InvalidInput(format!(
            "point {} dimension {} does not match model dimension {}",
            point.key,
            point.vector.len(),
            model.dimensions
        )));
    }
    if point.vector.iter().any(|value| !value.is_finite()) {
        return Err(AccessError::InvalidInput(format!(
            "point {} contains a non-finite vector value",
            point.key
        )));
    }
    if point.payload.model_revision != model.version {
        return Err(AccessError::InvalidInput(format!(
            "point {} model revision '{}' does not match binding revision '{}'",
            point.key, point.payload.model_revision, model.version
        )));
    }
    for (field, value) in [
        ("family", point.payload.family.as_str()),
        ("contribution_ref", point.payload.contribution_ref.as_str()),
        ("model_revision", point.payload.model_revision.as_str()),
        ("digest", point.payload.digest.as_str()),
    ] {
        if value.trim().is_empty() {
            return Err(AccessError::InvalidInput(format!(
                "point {} payload field {field} must not be empty",
                point.key
            )));
        }
    }
    if point.payload.content_revision == 0 {
        return Err(AccessError::InvalidInput(format!(
            "point {} content revision must be greater than zero",
            point.key
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn model() -> VectorModelDescriptor {
        VectorModelDescriptor {
            provider: "test".into(),
            model: "small".into(),
            version: "r1".into(),
            dimensions: 3,
            metric: "cosine".into(),
        }
    }

    fn point() -> QdrantVectorPoint {
        QdrantVectorPoint {
            key: EmbeddingKey::new(Uuid::from_u128(1)),
            vector: vec![1.0, 0.0, 0.0],
            payload: QdrantPointPayload {
                tenant_id: Uuid::from_u128(2),
                workspace_id: Uuid::from_u128(3),
                family: "chunk".into(),
                subject_id: Uuid::from_u128(4),
                document_id: DocumentId::new(Uuid::from_u128(5)),
                contribution_ref: "contribution:1".into(),
                model_revision: "r1".into(),
                content_revision: 1,
                manifest_id: Uuid::from_u128(6),
                digest: "digest-1".into(),
                modality: None,
            },
        }
    }

    #[test]
    fn point_validation_rejects_dimension_or_revision_drift() {
        let mut invalid = point();
        invalid.vector.pop();
        assert!(matches!(
            validate_point(&model(), &invalid),
            Err(AccessError::InvalidInput(_))
        ));

        let mut invalid = point();
        invalid.payload.model_revision = "r2".into();
        assert!(matches!(
            validate_point(&model(), &invalid),
            Err(AccessError::InvalidInput(_))
        ));
    }
}
