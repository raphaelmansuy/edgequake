//! SPEC-180 09: the image of an image query, and the normal-RAG plan built around it.
//!
//! One place turns a request into a [`NormalQueryPlan`] for query, context, chat and MCP:
//! kind filters, the query-side text vector for a multimodal workspace (the query prefix,
//! never the document prefix) and the image vector. The image is embedded for the search
//! only. It is not stored as a document.

use edgequake_core::multimodal_embed::{query_prefix, EmbedInput, MultimodalEmbedder};
use edgequake_core::rag_profile::{codes, ProfileError};
use edgequake_core::Workspace;
use edgequake_query::normal::NormalQueryPlan;

use crate::embeddings::image_limits::{looks_like_image, prepare_image};
use crate::error::{ApiError, ApiResult};
use crate::handlers::query_types::DocumentFilter;

/// One uploaded query image.
#[derive(Debug, Clone)]
pub struct QueryImage {
    pub bytes: Vec<u8>,
}

impl QueryImage {
    /// 400 `query_image_invalid` when the part is empty or not an image.
    pub fn from_bytes(bytes: Vec<u8>) -> ApiResult<Self> {
        if bytes.is_empty() || !looks_like_image(&bytes) {
            return Err(ProfileError::new(
                codes::QUERY_IMAGE_INVALID,
                "query_image must be a non-empty png, jpeg or webp file",
            )
            .into());
        }
        Ok(Self { bytes })
    }
}

fn multimodal_client(
    workspace: &Workspace,
) -> Option<ApiResult<std::sync::Arc<dyn MultimodalEmbedder>>> {
    crate::embeddings::multimodal_embedder_for(
        &workspace.embedding_provider,
        &workspace.embedding_model,
        workspace.embedding_dimension,
    )
    .map(|client| client.map_err(|e| crate::embeddings::api_error(&e)))
}

async fn embed_one(client: &dyn MultimodalEmbedder, input: EmbedInput) -> ApiResult<Vec<f32>> {
    client
        .embed(&[input])
        .await
        .map_err(|e| crate::embeddings::api_error(&e))?
        .into_iter()
        .next()
        .ok_or_else(|| ApiError::Internal("the embedder returned no vector".into()))
}

async fn image_vector(workspace: &Workspace, image: &QueryImage) -> ApiResult<Vec<f32>> {
    workspace.capabilities().check_image_query()?;
    let client = multimodal_client(workspace).ok_or_else(|| {
        ProfileError::new(
            codes::IMAGE_QUERY_NOT_SUPPORTED,
            "this workspace's embedding model cannot embed images",
        )
    })??;
    let prepared = prepare_image(
        &image.bytes,
        edgequake_pdf::image_guard::max_image_bytes_from_env(),
    )
    .map_err(|e| crate::embeddings::api_error(&e))?;
    embed_one(
        client.as_ref(),
        EmbedInput::image(prepared.mime, prepared.bytes),
    )
    .await
}

/// The text query vector of a multimodal workspace: the question under the query prefix.
async fn text_vector(workspace: &Workspace, query: &str) -> ApiResult<Option<Vec<f32>>> {
    if query.trim().is_empty() {
        return Ok(None);
    }
    let Some(client) = multimodal_client(workspace) else {
        return Ok(None);
    };
    let vector = embed_one(client?.as_ref(), EmbedInput::text(query_prefix(query))).await?;
    Ok(Some(vector))
}

/// The plan for one request, `None` when the engine needs none (graph workspace without
/// kind filters or an image).
pub async fn normal_plan(
    workspace: Option<&Workspace>,
    query: &str,
    filter: Option<&DocumentFilter>,
    image: Option<&QueryImage>,
) -> ApiResult<Option<NormalQueryPlan>> {
    let kinds_plan = crate::services::chunk_kind_filter::plan_for_filter(workspace, filter)?;
    let Some(workspace) = workspace else {
        return Ok(kinds_plan);
    };
    let caps = workspace.capabilities();
    if image.is_some() {
        caps.check_image_query()?;
    }
    if caps.queries_graph {
        return Ok(kinds_plan);
    }
    let mut plan = kinds_plan.unwrap_or_else(NormalQueryPlan::text);
    if let Some(image) = image {
        plan = plan.with_image_vector(image_vector(workspace, image).await?);
    }
    if let Some(vector) = text_vector(workspace, query).await? {
        plan = plan.with_text_vector(vector);
    }
    Ok(Some(plan))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_empty_part_is_invalid() {
        let err = QueryImage::from_bytes(Vec::new()).unwrap_err();
        assert!(err.to_string().contains("query_image"));
    }

    #[test]
    fn text_is_not_an_image() {
        assert!(QueryImage::from_bytes(b"hello world".to_vec()).is_err());
    }

    #[test]
    fn png_bytes_are_accepted() {
        let png = b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR".to_vec();
        assert!(QueryImage::from_bytes(png).is_ok());
    }

    #[tokio::test]
    async fn no_workspace_and_no_filter_means_no_plan() {
        assert!(normal_plan(None, "q", None, None).await.unwrap().is_none());
    }
}
