//! Serve vision page / chart-crop PNG assets (SPEC-047 MV-28 / durable mm-assets).
//!
//! First principles:
//! - Identity = `(document_id, asset_id)` where `asset_id` is the filename stem
//!   (`page-0001`, `page-0001-chart`) — see `asset_id_from_path`.
//! - Path-based URL remains for markdown `![…](assets/…)` rewrite compatibility.
//!
//! Routes:
//! - `GET /documents/{document_id}/assets` — list summaries (no BYTEA)
//! - `GET /documents/{document_id}/assets/{asset_id}` — binary by id
//! - `GET /documents/{document_id}/mm-assets/{*asset_path}` — binary by relative path

use std::path::{Component, Path, PathBuf};

use axum::body::Body;
use axum::extract::{Path as AxumPath, State};
use axum::http::header::{CACHE_CONTROL, CONTENT_TYPE};
use axum::http::{HeaderValue, StatusCode};
use axum::response::Response;
use axum::Json;
use tracing::debug;

use crate::error::{ApiError, ApiResult};
use crate::middleware::TenantContext;
use crate::services::document_assets::document_mm_assets_root;
#[cfg(not(feature = "postgres"))]
use crate::services::document_metadata_scan::metadata_key_for_document;
use crate::state::AppState;

/// Resolve a relative asset path under the document mm-assets root (path-traversal safe).
pub fn resolve_mm_asset_path(document_id: &str, asset_path: &str) -> ApiResult<PathBuf> {
    let trimmed = asset_path.trim_start_matches('/');
    if trimmed.is_empty() {
        return Err(ApiError::BadRequest("empty asset path".into()));
    }
    let rel = Path::new(trimmed);
    if rel.is_absolute()
        || rel
            .components()
            .any(|c| matches!(c, Component::ParentDir | Component::RootDir))
    {
        return Err(ApiError::BadRequest("invalid asset path".into()));
    }
    let root = document_mm_assets_root(document_id);
    let full = root.join(rel);
    let root_canon = root.canonicalize().unwrap_or(root.clone());
    let full_canon = full
        .canonicalize()
        .map_err(|_| ApiError::NotFound(format!("mm-asset not found: {trimmed}")))?;
    if !full_canon.starts_with(&root_canon) {
        return Err(ApiError::BadRequest(
            "asset path escapes mm-assets root".into(),
        ));
    }
    Ok(full_canon)
}

#[cfg(not(feature = "postgres"))]
fn guess_content_type(path: &Path) -> &'static str {
    match path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase()
        .as_str()
    {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        "gif" => "image/gif",
        _ => "application/octet-stream",
    }
}

/// Ensure the document exists in the current workspace (authz via tenant middleware).
///
/// SPEC-091 SSOT: existence is decided by the typed `public.documents` row, NOT the
/// retired generic KV store — migration 125 dropped the `eq_*_kv` tables, so a KV
/// `get_by_ids` here 42P01-fails ("relation … does not exist") and turned *every*
/// mm-asset request into a 500 (the images-not-served defect). The KV read remains only
/// in the non-postgres build, where the typed table does not exist.
async fn ensure_document_exists(
    state: &AppState,
    tenant: &TenantContext,
    document_id: &str,
) -> ApiResult<()> {
    #[cfg(feature = "postgres")]
    {
        // Resolve the workspace exactly as the asset read path does (default-alias
        // safe) so the scope check and the subsequent byte fetch agree.
        let mut scoped = tenant.clone();
        scoped.workspace_id = Some(tenant.workspace_id_or_default());
        let scope = crate::document_read_model::relational_document_scope(
            state.optional_pg_pool(),
            document_id,
            &scoped,
        )
        .await?;
        if scope.is_none() {
            return Err(ApiError::NotFound(format!(
                "Document not found: {document_id}"
            )));
        }
        Ok(())
    }
    #[cfg(not(feature = "postgres"))]
    {
        let _ = tenant;
        let key = metadata_key_for_document(document_id);
        let found = state
            .storage
            .kv_storage
            .get_by_ids(&[key])
            .await
            .map_err(ApiError::from)?
            .into_iter()
            .next();
        if found.is_none() {
            return Err(ApiError::NotFound(format!(
                "Document not found: {document_id}"
            )));
        }
        Ok(())
    }
}

fn binary_asset_response(
    bytes: Vec<u8>,
    content_type: &str,
    document_id: &str,
    label: &str,
) -> Response {
    debug!(
        %document_id,
        asset = %label,
        bytes = bytes.len(),
        "Serving document mm-asset"
    );
    let mut res = Response::new(Body::from(bytes));
    *res.status_mut() = StatusCode::OK;
    let ct = HeaderValue::from_str(content_type)
        .unwrap_or_else(|_| HeaderValue::from_static("application/octet-stream"));
    res.headers_mut().insert(CONTENT_TYPE, ct);
    res.headers_mut().insert(
        CACHE_CONTROL,
        HeaderValue::from_static("private, max-age=3600"),
    );
    res
}

async fn read_mm_asset_payload(
    state: &AppState,
    tenant: &TenantContext,
    document_id: &str,
    asset_path: &str,
) -> ApiResult<(Vec<u8>, String)> {
    #[cfg(feature = "postgres")]
    {
        let workspace_id = uuid::Uuid::parse_str(&tenant.workspace_id_or_default()).ok();
        let storage = state.storage.mm_asset_storage.as_deref();
        return crate::services::load_mm_asset_bytes(
            storage,
            document_id,
            workspace_id,
            asset_path,
        )
        .await;
    }
    #[cfg(not(feature = "postgres"))]
    {
        let _ = (state, tenant);
        let full = resolve_mm_asset_path(document_id, asset_path)?;
        let bytes = tokio::fs::read(&full)
            .await
            .map_err(|e| ApiError::NotFound(format!("mm-asset unreadable: {asset_path} ({e})")))?;
        Ok((bytes, guess_content_type(&full).to_string()))
    }
}

async fn read_mm_asset_payload_by_id(
    state: &AppState,
    tenant: &TenantContext,
    document_id: &str,
    asset_id: &str,
) -> ApiResult<(Vec<u8>, String)> {
    #[cfg(feature = "postgres")]
    {
        let workspace_id = uuid::Uuid::parse_str(&tenant.workspace_id_or_default()).ok();
        let storage = state.storage.mm_asset_storage.as_deref();
        return crate::services::load_mm_asset_bytes_by_id(
            storage,
            document_id,
            workspace_id,
            asset_id,
        )
        .await;
    }
    #[cfg(not(feature = "postgres"))]
    {
        let _ = (state, tenant);
        let path = format!("assets/{asset_id}.png");
        let full = resolve_mm_asset_path(document_id, &path)?;
        let bytes = tokio::fs::read(&full)
            .await
            .map_err(|e| ApiError::NotFound(format!("mm-asset unreadable: {asset_id} ({e})")))?;
        Ok((bytes, guess_content_type(&full).to_string()))
    }
}

/// GET `/api/v1/documents/{document_id}/assets` — list asset summaries (no binary).
#[utoipa::path(
    get,
    path = "/api/v1/documents/{document_id}/assets",
    params(
        ("document_id" = String, Path, description = "Document ID")
    ),
    responses(
        (status = 200, description = "Asset summaries"),
        (status = 404, description = "Document not found")
    ),
    tag = "documents"
)]
pub async fn list_document_assets(
    State(state): State<AppState>,
    tenant: TenantContext,
    AxumPath(document_id): AxumPath<String>,
) -> ApiResult<Json<serde_json::Value>> {
    ensure_document_exists(&state, &tenant, &document_id).await?;
    #[cfg(feature = "postgres")]
    {
        let workspace_id = uuid::Uuid::parse_str(&tenant.workspace_id_or_default())
            .map_err(|_| ApiError::BadRequest("invalid workspace id".into()))?;
        let storage = state.storage.mm_asset_storage.as_deref();
        let assets = crate::services::list_mm_asset_summaries_for_document(
            storage,
            &document_id,
            workspace_id,
        )
        .await?;
        Ok(Json(serde_json::json!({
            "document_id": document_id,
            "assets": assets,
        })))
    }
    #[cfg(not(feature = "postgres"))]
    {
        let _ = tenant;
        Ok(Json(serde_json::json!({
            "document_id": document_id,
            "assets": [],
        })))
    }
}

/// POST `/api/v1/documents/{document_id}/assets/include-from-pdf`
///
/// First principles: the linked PDF is the source of visual truth — render page
/// PNGs, persist mm-assets, and enrich markdown figure headings with `![…](assets/…)`.
#[utoipa::path(
    post,
    path = "/api/v1/documents/{document_id}/assets/include-from-pdf",
    params(
        ("document_id" = String, Path, description = "Document ID")
    ),
    responses(
        (status = 200, description = "Assets included from PDF"),
        (status = 400, description = "No linked PDF"),
        (status = 404, description = "Document or PDF not found")
    ),
    tag = "documents"
)]
pub async fn include_document_assets_from_pdf(
    State(state): State<AppState>,
    tenant: TenantContext,
    AxumPath(document_id): AxumPath<String>,
) -> ApiResult<Json<crate::services::IncludePdfAssetsResult>> {
    ensure_document_exists(&state, &tenant, &document_id).await?;
    let result =
        crate::services::include_extracted_pdf_assets(&state, &tenant, &document_id).await?;
    Ok(Json(result))
}

/// GET `/api/v1/documents/{document_id}/assets/{asset_id}` — binary by stable id.
#[utoipa::path(
    get,
    path = "/api/v1/documents/{document_id}/assets/{asset_id}",
    params(
        ("document_id" = String, Path, description = "Document ID"),
        ("asset_id" = String, Path, description = "Stable asset id (filename stem, e.g. page-0001-chart)")
    ),
    responses(
        (status = 200, description = "Asset bytes", content_type = "image/png"),
        (status = 404, description = "Not found")
    ),
    tag = "documents"
)]
pub async fn download_document_asset_by_id(
    State(state): State<AppState>,
    tenant: TenantContext,
    AxumPath((document_id, asset_id)): AxumPath<(String, String)>,
) -> ApiResult<Response> {
    ensure_document_exists(&state, &tenant, &document_id).await?;
    let (bytes, content_type) =
        read_mm_asset_payload_by_id(&state, &tenant, &document_id, &asset_id).await?;
    Ok(binary_asset_response(
        bytes,
        &content_type,
        &document_id,
        &asset_id,
    ))
}

/// GET `/api/v1/documents/{document_id}/mm-assets/{*asset_path}`
#[utoipa::path(
    get,
    path = "/api/v1/documents/{document_id}/mm-assets/{asset_path}",
    params(
        ("document_id" = String, Path, description = "Document ID"),
        ("asset_path" = String, Path, description = "Relative path under mm-assets (e.g. assets/page-0001.png)")
    ),
    responses(
        (status = 200, description = "Asset bytes", content_type = "image/png"),
        (status = 404, description = "Not found")
    ),
    tag = "documents"
)]
pub async fn download_document_mm_asset(
    State(state): State<AppState>,
    tenant: TenantContext,
    AxumPath((document_id, asset_path)): AxumPath<(String, String)>,
) -> ApiResult<Response> {
    ensure_document_exists(&state, &tenant, &document_id).await?;
    let (bytes, content_type) =
        read_mm_asset_payload(&state, &tenant, &document_id, &asset_path).await?;
    Ok(binary_asset_response(
        bytes,
        &content_type,
        &document_id,
        &asset_path,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn rejects_path_traversal() {
        assert!(resolve_mm_asset_path("doc-1", "../../x").is_err());
    }

    #[test]
    fn resolves_nested_asset_when_present() {
        let root = document_mm_assets_root("mm-asset-test-doc");
        let assets = root.join("assets");
        let _ = fs::create_dir_all(&assets);
        let file = assets.join("page-0001.png");
        fs::write(&file, b"\x89PNG").unwrap();
        let resolved = resolve_mm_asset_path("mm-asset-test-doc", "assets/page-0001.png").unwrap();
        assert!(resolved.ends_with("page-0001.png"));
        let _ = fs::remove_dir_all(&root);
    }

    /// Contract (realized 2026-07-29): in the postgres build, document existence must
    /// be decided by the typed `public.documents` SSOT (`relational_document_scope`),
    /// never the retired generic KV store (`kv_storage.get_by_ids`). Migration 125
    /// dropped the `eq_*_kv` tables, so the old KV existence check 42P01-failed and
    /// turned *every* mm-asset request into a 500 (images not served). The KV read is
    /// confined to the `cfg(not(postgres))` fallback.
    #[test]
    fn contract_spec091_existence_uses_typed_ssot_not_kv() {
        let src = include_str!("mm_assets.rs");
        let fn_start = src
            .find("async fn ensure_document_exists")
            .expect("ensure fn");
        let fn_body = &src[fn_start..];
        let pg_cfg = fn_body
            .find("#[cfg(feature = \"postgres\")]")
            .expect("postgres branch");
        let non_pg_cfg = fn_body
            .find("#[cfg(not(feature = \"postgres\"))]")
            .expect("non-postgres branch");
        let pg_region = &fn_body[pg_cfg..non_pg_cfg];
        assert!(
            pg_region.contains("relational_document_scope"),
            "postgres existence path must use the typed documents SSOT"
        );
        assert!(
            !pg_region.contains("kv_storage"),
            "postgres existence path must not touch the retired KV store"
        );
    }
}
