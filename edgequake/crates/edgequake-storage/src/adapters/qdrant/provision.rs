use std::collections::HashMap;

use edgequake_storage_contracts::{AccessError, AccessResult, VectorModelDescriptor};
use serde::{Deserialize, Serialize};

use super::client::QdrantClient;
use super::errors::parse_completed_operation;

pub const REQUIRED_PAYLOAD_INDEXES: [&str; 4] =
    ["tenant_id", "workspace_id", "family", "document_id"];

#[derive(Debug, Serialize)]
struct CreateCollectionBody {
    vectors: VectorParams,
}

#[derive(Debug, Serialize)]
struct VectorParams {
    size: u32,
    distance: &'static str,
}

#[derive(Debug, Serialize)]
struct CreateIndexBody<'a> {
    field_name: &'a str,
    field_schema: &'static str,
}

#[derive(Debug, Deserialize)]
struct BoolEnvelope {
    result: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct CollectionEnvelope {
    result: Option<CollectionInfo>,
}

#[derive(Debug, Deserialize)]
struct CollectionInfo {
    status: String,
    config: CollectionConfig,
    payload_schema: HashMap<String, PayloadSchema>,
}

#[derive(Debug, Deserialize)]
struct CollectionConfig {
    params: CollectionParams,
}

#[derive(Debug, Deserialize)]
struct CollectionParams {
    vectors: CollectionVectorParams,
}

#[derive(Debug, Deserialize)]
struct CollectionVectorParams {
    size: u32,
    distance: String,
}

#[derive(Debug, Deserialize)]
struct PayloadSchema {
    data_type: String,
}

/// Explicit provisioning command. Startup readiness never calls this function.
pub async fn provision_qdrant_binding(
    client: &QdrantClient,
    model: &VectorModelDescriptor,
) -> AccessResult<()> {
    client.health().await?;
    match collection_info(client).await {
        Ok(info) => verify_info(client, model, &info, false)?,
        Err(AccessError::NotFound(_)) => create_collection(client, model).await?,
        Err(error) => return Err(error),
    }

    let mut info = collection_info(client).await?;
    verify_info(client, model, &info, false)?;
    for field in REQUIRED_PAYLOAD_INDEXES {
        if !info.payload_schema.contains_key(field) {
            create_payload_index(client, field).await?;
        }
    }
    info = collection_info(client).await?;
    verify_info(client, model, &info, true)
}

/// Verify-only startup/readiness path: no collection or index creation.
pub async fn verify_qdrant_binding(
    client: &QdrantClient,
    model: Option<&VectorModelDescriptor>,
) -> AccessResult<()> {
    client.health().await?;
    let info = collection_info(client).await?;
    if let Some(model) = model {
        verify_info(client, model, &info, true)
    } else {
        verify_indexes_and_status(client, &info)
    }
}

/// Administrative cleanup helper for integration tests and retired bindings.
pub async fn drop_qdrant_binding(client: &QdrantClient) -> AccessResult<()> {
    let path = format!("/collections/{}", client.collection_name());
    let bytes = client.delete(&path).await?;
    let envelope: BoolEnvelope = serde_json::from_slice(&bytes).map_err(|error| {
        AccessError::CorruptData(format!("invalid Qdrant drop response: {error}"))
    })?;
    match envelope.result {
        Some(true) => Ok(()),
        _ => Err(AccessError::UnknownOutcome(
            "Qdrant did not confirm collection deletion".into(),
        )),
    }
}

async fn create_collection(
    client: &QdrantClient,
    model: &VectorModelDescriptor,
) -> AccessResult<()> {
    let body = CreateCollectionBody {
        vectors: VectorParams {
            size: model.dimensions,
            distance: qdrant_distance(&model.metric)?,
        },
    };
    let path = format!("/collections/{}", client.collection_name());
    let bytes = client.put_json(&path, &body).await?;
    let envelope: BoolEnvelope = serde_json::from_slice(&bytes).map_err(|error| {
        AccessError::CorruptData(format!("invalid Qdrant create response: {error}"))
    })?;
    match envelope.result {
        Some(true) => Ok(()),
        _ => Err(AccessError::UnknownOutcome(
            "Qdrant did not confirm collection creation".into(),
        )),
    }
}

async fn create_payload_index(client: &QdrantClient, field: &str) -> AccessResult<()> {
    let body = CreateIndexBody {
        field_name: field,
        field_schema: "keyword",
    };
    let path = format!("/collections/{}/index?wait=true", client.collection_name());
    let bytes = client.put_json(&path, &body).await?;
    parse_completed_operation(&bytes)?;
    Ok(())
}

async fn collection_info(client: &QdrantClient) -> AccessResult<CollectionInfo> {
    let path = format!("/collections/{}", client.collection_name());
    let envelope: CollectionEnvelope = client.get_json(&path).await?;
    envelope
        .result
        .ok_or_else(|| AccessError::CorruptData("Qdrant collection response omitted result".into()))
}

fn verify_info(
    client: &QdrantClient,
    model: &VectorModelDescriptor,
    info: &CollectionInfo,
    require_indexes: bool,
) -> AccessResult<()> {
    let info = verify_vector_config(client, model, info)?;
    if require_indexes {
        verify_indexes_and_status(client, info)
    } else {
        verify_status(client, info)
    }
}

fn verify_vector_config<'a>(
    client: &QdrantClient,
    model: &VectorModelDescriptor,
    info: &'a CollectionInfo,
) -> AccessResult<&'a CollectionInfo> {
    let expected_distance = qdrant_distance(&model.metric)?;
    if info.config.params.vectors.size != model.dimensions
        || !info
            .config
            .params
            .vectors
            .distance
            .eq_ignore_ascii_case(expected_distance)
    {
        return Err(AccessError::Conflict(format!(
            "Qdrant collection '{}' has vector size/distance {}/{}, expected {}/{}",
            client.collection_name(),
            info.config.params.vectors.size,
            info.config.params.vectors.distance,
            model.dimensions,
            expected_distance
        )));
    }
    Ok(info)
}

fn verify_indexes_and_status(client: &QdrantClient, info: &CollectionInfo) -> AccessResult<()> {
    verify_status(client, info)?;
    for field in REQUIRED_PAYLOAD_INDEXES {
        let schema = info.payload_schema.get(field).ok_or_else(|| {
            AccessError::Unavailable(format!(
                "Qdrant collection '{}' is missing required payload index '{field}'",
                client.collection_name()
            ))
        })?;
        if !matches!(schema.data_type.as_str(), "keyword" | "uuid") {
            return Err(AccessError::Unavailable(format!(
                "Qdrant payload index '{field}' has unsupported type '{}'",
                schema.data_type
            )));
        }
    }
    Ok(())
}

fn verify_status(client: &QdrantClient, info: &CollectionInfo) -> AccessResult<()> {
    if info.status.eq_ignore_ascii_case("red") {
        return Err(AccessError::Unavailable(format!(
            "Qdrant collection '{}' is red",
            client.collection_name()
        )));
    }
    Ok(())
}

fn qdrant_distance(metric: &str) -> AccessResult<&'static str> {
    match metric.trim().to_ascii_lowercase().as_str() {
        "cosine" => Ok("Cosine"),
        "euclid" | "euclidean" | "l2" => Ok("Euclid"),
        "dot" | "inner_product" | "inner-product" => Ok("Dot"),
        "manhattan" | "l1" => Ok("Manhattan"),
        other => Err(AccessError::UnsupportedCapability(format!(
            "Qdrant distance metric '{other}' is not supported"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_metric_maps_to_qdrant_distance() {
        assert_eq!(qdrant_distance("cosine").unwrap(), "Cosine");
        assert_eq!(qdrant_distance("L2").unwrap(), "Euclid");
        assert_eq!(qdrant_distance("inner_product").unwrap(), "Dot");
        assert!(matches!(
            qdrant_distance("hamming"),
            Err(AccessError::UnsupportedCapability(_))
        ));
    }
}
