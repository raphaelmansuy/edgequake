use std::time::Duration;

use async_trait::async_trait;
use chrono::Utc;
use edgequake_storage_contracts::{
    AccessError, AccessResult, EmbeddingKey, ScopedVectorSearch, VectorSearchHit, VectorSearchMode,
    VectorSearchPage, VectorSearchRequest,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::traits::MetadataFilter;

use super::client::QdrantClient;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompiledFilter {
    MatchNone,
    Predicate(Value),
}

enum RequestFilter {
    MatchNone,
    Predicate(Filter),
}

#[derive(Debug, Clone, Serialize)]
struct Filter {
    must: Vec<FieldCondition>,
}

#[derive(Debug, Clone, Serialize)]
struct FieldCondition {
    key: &'static str,
    r#match: MatchExpression,
}

#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
enum MatchExpression {
    Value { value: String },
    Any { any: Vec<String> },
}

#[derive(Debug, Serialize)]
struct QueryPointsBody<'a> {
    query: &'a [f32],
    filter: Filter,
    params: SearchParams,
    score_threshold: Option<f32>,
    limit: u64,
    with_payload: bool,
    with_vector: bool,
}

#[derive(Debug, Serialize)]
struct SearchParams {
    exact: bool,
}

#[derive(Debug, Deserialize)]
struct QueryEnvelope {
    result: Option<QueryResult>,
}

#[derive(Debug, Deserialize)]
struct QueryResult {
    points: Vec<ScoredPoint>,
}

#[derive(Debug, Deserialize)]
struct ScoredPoint {
    id: Value,
    score: f32,
    payload: Option<Value>,
}

/// Compile the legacy vector metadata predicate without weakening empty lists.
pub fn compile_metadata_filter(filter: &MetadataFilter) -> AccessResult<CompiledFilter> {
    if filter
        .document_ids
        .as_ref()
        .is_some_and(|values| values.is_empty())
        || filter
            .modalities
            .as_ref()
            .is_some_and(|values| values.is_empty())
    {
        return Ok(CompiledFilter::MatchNone);
    }

    let mut conditions = Vec::new();
    push_value(&mut conditions, "tenant_id", filter.tenant_id.as_deref());
    push_value(
        &mut conditions,
        "workspace_id",
        filter.workspace_id.as_deref(),
    );
    push_value(&mut conditions, "family", filter.vector_type.as_deref());
    push_any(&mut conditions, "document_id", filter.document_ids.as_ref());
    push_any(&mut conditions, "modality", filter.modalities.as_ref());
    let value = serde_json::to_value(Filter { must: conditions })
        .map_err(|error| AccessError::CorruptData(error.to_string()))?;
    Ok(CompiledFilter::Predicate(value))
}

fn compile_request_filter(request: &VectorSearchRequest) -> RequestFilter {
    if request
        .document_ids
        .as_ref()
        .is_some_and(|values| values.is_empty())
        || request
            .modalities
            .as_ref()
            .is_some_and(|values| values.is_empty())
        || request
            .filter_ids
            .as_ref()
            .is_some_and(|values| values.is_empty())
    {
        return RequestFilter::MatchNone;
    }

    let mut conditions = vec![
        value_condition("tenant_id", request.scope.tenant().to_string()),
        value_condition("workspace_id", request.scope.workspace().to_string()),
        value_condition("family", request.family.clone()),
    ];
    if let Some(document_ids) = &request.document_ids {
        conditions.push(any_condition(
            "document_id",
            document_ids.iter().map(ToString::to_string).collect(),
        ));
    }
    if let Some(modalities) = &request.modalities {
        conditions.push(any_condition("modality", modalities.clone()));
    }
    if let Some(filter_ids) = &request.filter_ids {
        conditions.push(any_condition(
            "subject_id",
            filter_ids.iter().map(ToString::to_string).collect(),
        ));
    }
    RequestFilter::Predicate(Filter { must: conditions })
}

fn push_value(conditions: &mut Vec<FieldCondition>, key: &'static str, value: Option<&str>) {
    if let Some(value) = value {
        conditions.push(value_condition(key, value.to_string()));
    }
}

fn push_any(conditions: &mut Vec<FieldCondition>, key: &'static str, values: Option<&Vec<String>>) {
    if let Some(values) = values {
        conditions.push(any_condition(key, values.clone()));
    }
}

fn value_condition(key: &'static str, value: String) -> FieldCondition {
    FieldCondition {
        key,
        r#match: MatchExpression::Value { value },
    }
}

fn any_condition(key: &'static str, any: Vec<String>) -> FieldCondition {
    FieldCondition {
        key,
        r#match: MatchExpression::Any { any },
    }
}

#[async_trait]
impl ScopedVectorSearch for QdrantClient {
    async fn search(&self, request: &VectorSearchRequest) -> AccessResult<VectorSearchPage> {
        validate_request(request)?;
        let filter = match compile_request_filter(request) {
            RequestFilter::MatchNone => {
                return Ok(VectorSearchPage {
                    hits: Vec::new(),
                    next_cursor: None,
                    budget_exhausted: false,
                });
            }
            RequestFilter::Predicate(filter) => filter,
        };

        if request.top_k == 0 {
            return Ok(VectorSearchPage {
                hits: Vec::new(),
                next_cursor: None,
                budget_exhausted: false,
            });
        }

        let remaining = request
            .deadline
            .signed_duration_since(Utc::now())
            .to_std()
            .map_err(|_| AccessError::DeadlineExceeded("vector deadline already elapsed".into()))?;
        let limit = u64::from(request.top_k).min(request.scan_budget);
        let budget_exhausted = limit < u64::from(request.top_k);
        let body = QueryPointsBody {
            query: &request.embedding,
            filter,
            params: SearchParams {
                exact: matches!(request.search_mode, VectorSearchMode::Exact),
            },
            score_threshold: request.threshold,
            limit,
            with_payload: true,
            with_vector: false,
        };
        let path = format!("/collections/{}/points/query", self.collection_name());
        let bytes = self
            .post_json(&path, &body, Some(remaining.max(Duration::from_millis(1))))
            .await?;
        let envelope: QueryEnvelope = serde_json::from_slice(&bytes)
            .map_err(|error| AccessError::CorruptData(format!("invalid Qdrant query: {error}")))?;
        let result = envelope
            .result
            .ok_or_else(|| AccessError::CorruptData("Qdrant query omitted result".into()))?;
        let hits = result
            .points
            .into_iter()
            .map(scored_point_to_hit)
            .collect::<AccessResult<Vec<_>>>()?;
        Ok(VectorSearchPage {
            hits,
            next_cursor: None,
            budget_exhausted,
        })
    }
}

fn validate_request(request: &VectorSearchRequest) -> AccessResult<()> {
    if request.embedding.len() != request.model.dimensions as usize {
        return Err(AccessError::InvalidInput(format!(
            "embedding dimension {} does not match model dimension {}",
            request.embedding.len(),
            request.model.dimensions
        )));
    }
    if request.embedding.iter().any(|value| !value.is_finite()) {
        return Err(AccessError::InvalidInput(
            "embedding contains a non-finite value".into(),
        ));
    }
    if request.model.metric.eq_ignore_ascii_case("cosine")
        && !request.embedding.iter().any(|value| *value != 0.0)
    {
        return Err(AccessError::InvalidInput(
            "cosine embedding must have non-zero norm".into(),
        ));
    }
    if request.scan_budget == 0 {
        return Err(AccessError::InvalidInput(
            "vector scan budget must be greater than zero".into(),
        ));
    }
    Ok(())
}

fn scored_point_to_hit(point: ScoredPoint) -> AccessResult<VectorSearchHit> {
    let point_id = point
        .id
        .as_str()
        .ok_or_else(|| AccessError::CorruptData("Qdrant point ID is not a UUID string".into()))
        .and_then(parse_uuid)?;
    let payload = point
        .payload
        .ok_or_else(|| AccessError::CorruptData("Qdrant hit omitted payload".into()))?;
    let subject_id = payload_uuid(&payload, "subject_id")?;
    let content_revision = payload
        .get("content_revision")
        .and_then(Value::as_u64)
        .ok_or_else(|| {
            AccessError::CorruptData("Qdrant payload omitted content_revision".into())
        })?;
    Ok(VectorSearchHit {
        key: EmbeddingKey::new(point_id),
        subject_id,
        content_revision,
        score: point.score,
    })
}

fn payload_uuid(payload: &Value, key: &str) -> AccessResult<Uuid> {
    payload
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| AccessError::CorruptData(format!("Qdrant payload omitted {key}")))
        .and_then(parse_uuid)
}

fn parse_uuid(value: &str) -> AccessResult<Uuid> {
    Uuid::parse_str(value).map_err(|error| {
        AccessError::CorruptData(format!("invalid Qdrant UUID '{value}': {error}"))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration as ChronoDuration;
    use edgequake_storage_contracts::{
        AccessScope, DocumentId, TenantId, VectorModelDescriptor, WorkspaceId,
    };

    fn must(value: &CompiledFilter) -> &[Value] {
        match value {
            CompiledFilter::Predicate(value) => value["must"].as_array().unwrap(),
            CompiledFilter::MatchNone => panic!("expected predicate"),
        }
    }

    #[test]
    fn metadata_filter_compiles_and_combines_predicates() {
        let compiled = compile_metadata_filter(&MetadataFilter {
            document_ids: Some(vec!["doc-a".into(), "doc-b".into()]),
            tenant_id: Some("tenant-a".into()),
            workspace_id: Some("workspace-a".into()),
            vector_type: Some("chunk".into()),
            modalities: Some(vec!["table".into()]),
        })
        .unwrap();
        let conditions = must(&compiled);
        assert_eq!(conditions.len(), 5);
        assert!(conditions
            .iter()
            .any(|condition| condition["key"] == "tenant_id"));
        assert!(conditions
            .iter()
            .any(|condition| condition["key"] == "workspace_id"));
        assert!(conditions
            .iter()
            .any(|condition| condition["key"] == "family"));
        assert!(conditions
            .iter()
            .any(|condition| condition["match"]["any"][0] == "doc-a"));
    }

    #[test]
    fn empty_business_filter_compiles_to_match_none() {
        for filter in [
            MetadataFilter {
                document_ids: Some(Vec::new()),
                ..MetadataFilter::default()
            },
            MetadataFilter {
                modalities: Some(Vec::new()),
                ..MetadataFilter::default()
            },
        ] {
            assert_eq!(
                compile_metadata_filter(&filter).unwrap(),
                CompiledFilter::MatchNone
            );
        }
    }

    #[tokio::test]
    async fn empty_document_filter_returns_without_network_call() {
        let client = QdrantClient::new("http://127.0.0.1:1", Uuid::from_u128(149)).unwrap();
        let request = VectorSearchRequest {
            scope: AccessScope::new(
                TenantId::new(Uuid::from_u128(1)),
                WorkspaceId::new(Uuid::from_u128(2)),
            ),
            model: VectorModelDescriptor {
                provider: "test".into(),
                model: "small".into(),
                version: "r1".into(),
                dimensions: 3,
                metric: "cosine".into(),
            },
            family: "chunk".into(),
            embedding: vec![1.0, 0.0, 0.0],
            top_k: 10,
            document_ids: Some(Vec::<DocumentId>::new()),
            modalities: None,
            filter_ids: None,
            threshold: None,
            search_mode: VectorSearchMode::Exact,
            deadline: Utc::now() + ChronoDuration::seconds(1),
            scan_budget: 100,
        };

        let page = client.search(&request).await.unwrap();

        assert!(page.hits.is_empty());
    }
}
