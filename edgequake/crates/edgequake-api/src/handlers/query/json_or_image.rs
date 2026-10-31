//! SPEC-180 09: a query body that is JSON, or multipart with an optional `query_image`.
//!
//! JSON keeps its old behavior and its old rejections. A multipart body carries the same
//! JSON under the field `request` (or a bare `query` text for a one-line search) and one
//! file under `query_image`. The file is held in memory for the search only.

use axum::extract::{FromRequest, Request};
use axum::http::header::CONTENT_TYPE;
use axum::response::{IntoResponse, Response};
use axum::Json;
use axum_extra::extract::Multipart;
use serde::de::DeserializeOwned;

use crate::error::ApiError;
use crate::services::query_image::QueryImage;

/// Upper bound of the raw upload; the embedder re-encodes large images itself.
const MAX_QUERY_IMAGE_BYTES: usize = 25 * 1024 * 1024;

/// The request body and, when sent, the query image.
pub struct JsonOrImage<T> {
    pub body: T,
    pub image: Option<QueryImage>,
}

impl<T> JsonOrImage<T> {
    /// A text-only request.
    pub fn text(body: T) -> Self {
        Self { body, image: None }
    }
}

fn is_multipart(request: &Request) -> bool {
    request
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| {
            v.trim_start()
                .to_ascii_lowercase()
                .starts_with("multipart/form-data")
        })
}

fn bad(message: impl Into<String>) -> Response {
    ApiError::BadRequest(message.into()).into_response()
}

/// Read the `request` JSON (or `query` text) and the `query_image` file of a form.
async fn read_form<T: DeserializeOwned>(
    mut form: Multipart,
) -> Result<(T, Option<QueryImage>), Response> {
    let mut json: Option<String> = None;
    let mut query: Option<String> = None;
    let mut image: Option<QueryImage> = None;
    while let Some(field) = form.next_field().await.map_err(|e| bad(e.to_string()))? {
        match field.name().unwrap_or("") {
            "request" => json = Some(field.text().await.map_err(|e| bad(e.to_string()))?),
            "query" | "message" => {
                query = Some(field.text().await.map_err(|e| bad(e.to_string()))?)
            }
            "query_image" => {
                if image.is_some() {
                    return Err(bad("send one query_image per request"));
                }
                let bytes = field.bytes().await.map_err(|e| bad(e.to_string()))?;
                if bytes.len() > MAX_QUERY_IMAGE_BYTES {
                    return Err(bad("query_image is larger than 25 MiB"));
                }
                image = Some(
                    QueryImage::from_bytes(bytes.to_vec()).map_err(IntoResponse::into_response)?,
                );
            }
            _ => {}
        }
    }
    let raw = json
        .or_else(|| query.map(|q| serde_json::json!({ "query": q, "message": q }).to_string()))
        .unwrap_or_else(|| "{}".to_string());
    let body = serde_json::from_str(&raw).map_err(|e| bad(format!("invalid request JSON: {e}")))?;
    Ok((body, image))
}

impl<S, T> FromRequest<S> for JsonOrImage<T>
where
    S: Send + Sync,
    T: DeserializeOwned,
{
    type Rejection = Response;

    async fn from_request(request: Request, state: &S) -> Result<Self, Self::Rejection> {
        if is_multipart(&request) {
            let form = Multipart::from_request(request, state)
                .await
                .map_err(IntoResponse::into_response)?;
            let (body, image) = read_form(form).await?;
            return Ok(Self { body, image });
        }
        let Json(body) = Json::<T>::from_request(request, state)
            .await
            .map_err(IntoResponse::into_response)?;
        Ok(Self { body, image: None })
    }
}
