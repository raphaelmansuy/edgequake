//! MCP upload handle. Signed PUT is the PDF path. Chunked write stays for a small body.
//! Commit admits asynchronously; it does not wait for the pipeline.

use std::sync::Arc;
use std::time::Duration;

use base64::{engine::general_purpose::STANDARD, Engine};
use edgequake_storage::mcp_upload::{
    now_unix, upload_expired, MemoryUploadSessions, UploadClaim, UploadMeta, UploadScope,
    UploadSessions, UploadStatus, UploadTransfer,
};
use hmac::{Hmac, Mac};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use crate::error::{ApiError, ApiResult};
use crate::file_validation::sanitize_filename;
use crate::handlers::documents::upload::{
    admit_document_for_processing, DocumentAdmissionInput, GleaningAdmissionOptions,
    UploadExtraction,
};
use crate::middleware::TenantContext;
use crate::services::ContentHasher;
use crate::state::AppState;

use super::blob::upload_ttl_secs;
use super::budget::BudgetClass;
use super::envelope::EnvelopeBuilder;
use super::errors::{eq_error, ErrorCode};
use super::ingest::envelope_from_admission;

type HmacSha256 = Hmac<Sha256>;

const INSTRUCTION: &str = "Upload is open. PUT the file bytes to url with exactly the headers shown. Do not repeat the url. Do not paste the file. Then call eq_upload_commit with this upload_id and poll eq_task_get until indexed.";

pub struct McpUploadStore {
    inner: Arc<dyn UploadSessions>,
}

impl McpUploadStore {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(MemoryUploadSessions::new()),
        }
    }

    #[cfg(feature = "postgres")]
    pub fn postgres(pool: sqlx::PgPool) -> Self {
        Self {
            inner: edgequake_storage::mcp_upload::postgres(pool),
        }
    }

    pub async fn mark_expired(&self, upload_id: &str) -> bool {
        self.inner.mark_expired(upload_id).await.unwrap_or(false)
    }
}

impl Default for McpUploadStore {
    fn default() -> Self {
        Self::new()
    }
}

pub async fn eq_upload_begin(
    state: &AppState,
    tenant_ctx: &TenantContext,
    args: &Value,
) -> ApiResult<Value> {
    let access = args
        .get("access")
        .map(|value| {
            serde_json::from_value::<crate::services::document_access_types::DocumentAccess>(
                value.clone(),
            )
            .map_err(|_| {
                ApiError::bad_request_coded("INVALID_ACCESS_PAYLOAD", "Invalid access envelope")
            })
        })
        .transpose()?;
    #[cfg(feature = "postgres")]
    if let Some(pool) = state.optional_pg_pool() {
        crate::services::document_access::validate_upload(pool, tenant_ctx, access.as_ref())
            .await?;
    }
    let access = access
        .map(serde_json::to_value)
        .transpose()
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    let raw_name = args
        .get("filename")
        .and_then(|v| v.as_str())
        .unwrap_or("upload.bin");
    let filename = sanitize_filename(raw_name);
    let media_type = args
        .get("media_type")
        .and_then(|v| v.as_str())
        .unwrap_or("application/octet-stream")
        .to_string();
    let sha256 = args
        .get("sha256")
        .and_then(|v| v.as_str())
        .map(str::to_string);
    let byte_length = args.get("byte_length").and_then(|v| v.as_u64());
    if let Some(n) = byte_length {
        if n as usize > state.config.max_document_size || n == 0 {
            let code = if n == 0 {
                ErrorCode::InvalidId
            } else {
                ErrorCode::BudgetExceeded
            };
            let message = if n == 0 {
                "byte_length must be at least 1"
            } else {
                "upload exceeds max document size"
            };
            return Ok(eq_error(code, message, None));
        }
    }
    let chunked = args
        .get("chunked")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let scope = scope_of(tenant_ctx);
    let upload_id = format!("upl_{}", uuid::Uuid::new_v4());
    let expires = now_unix() + upload_ttl_secs() as i64;

    let (transfer, url, headers, staging_key) = if chunked {
        (UploadTransfer::Chunk, None, Vec::new(), None)
    } else {
        match open_signed(
            state,
            &scope,
            &upload_id,
            &media_type,
            byte_length,
            sha256.as_deref(),
            expires,
        )
        .await?
        {
            SignedOpen::Error(value) => return Ok(value),
            SignedOpen::Ready {
                transfer,
                url,
                headers,
                staging_key,
                sha256: sha,
            } => {
                return finish_begin(
                    state,
                    BeginFinish {
                        access: access.clone(),
                        scope,
                        upload_id,
                        filename,
                        media_type,
                        byte_length,
                        sha256: Some(sha),
                        transfer,
                        url: Some(url),
                        headers,
                        staging_key,
                        expires,
                        bind_bearer: bearer_bind(),
                    },
                )
                .await;
            }
        }
    };

    finish_begin(
        state,
        BeginFinish {
            access,
            scope,
            upload_id,
            filename,
            media_type,
            byte_length,
            sha256,
            transfer,
            url,
            headers,
            staging_key,
            expires,
            bind_bearer: false,
        },
    )
    .await
}

struct BeginFinish {
    access: Option<Value>,
    scope: UploadScope,
    upload_id: String,
    filename: String,
    media_type: String,
    byte_length: Option<u64>,
    sha256: Option<String>,
    transfer: UploadTransfer,
    url: Option<String>,
    headers: Vec<(String, String)>,
    staging_key: Option<String>,
    expires: i64,
    bind_bearer: bool,
}

enum SignedOpen {
    Error(Value),
    Ready {
        transfer: UploadTransfer,
        url: String,
        headers: Vec<(String, String)>,
        staging_key: Option<String>,
        sha256: String,
    },
}

async fn open_signed(
    state: &AppState,
    scope: &UploadScope,
    upload_id: &str,
    media_type: &str,
    byte_length: Option<u64>,
    sha256: Option<&str>,
    expires_at: i64,
) -> ApiResult<SignedOpen> {
    let Some(len) = byte_length else {
        return Ok(SignedOpen::Error(eq_error(
            ErrorCode::InvalidId,
            "byte_length is required",
            None,
        )));
    };
    let Some(sha) = sha256.filter(|s| is_sha256_hex(s)) else {
        return Ok(SignedOpen::Error(eq_error(
            ErrorCode::InvalidId,
            "sha256 must be 64 hex characters",
            None,
        )));
    };
    let bind_bearer = bearer_bind();
    if let Some(presign) = try_presign(state, scope, media_type, sha, bind_bearer).await? {
        return Ok(SignedOpen::Ready {
            transfer: UploadTransfer::Presign,
            url: presign.url,
            headers: presign.headers,
            staging_key: Some(presign.key),
            sha256: sha.to_string(),
        });
    }
    let Some(key) = mac_key(state) else {
        return Ok(SignedOpen::Error(eq_error(
            ErrorCode::NotImplemented,
            "upload relay has no MAC key",
            None,
        )));
    };
    let tid = scope.tenant_id.clone().unwrap_or_default();
    let wid = scope.workspace_id.clone().unwrap_or_default();
    let exp = expires_at.to_string();
    let sig = sign_mac(
        &key,
        &mac_message(upload_id, &exp, sha, media_type, len, &tid, &wid),
    );
    let base = crate::mcp::config::McpPublicConfig::resolve(&axum::http::HeaderMap::new());
    let url = format!(
        "{}/api/v1/mcp/uploads/{upload_id}?exp={exp}&tid={tid}&wid={wid}&sha={sha}&len={len}&sig={sig}",
        base.public_base
    );
    Ok(SignedOpen::Ready {
        transfer: UploadTransfer::Relay,
        url,
        headers: vec![("content-type".into(), media_type.into())],
        staging_key: None,
        sha256: sha.to_string(),
    })
}

struct PresignReady {
    url: String,
    headers: Vec<(String, String)>,
    key: String,
}

async fn try_presign(
    state: &AppState,
    scope: &UploadScope,
    media_type: &str,
    sha256: &str,
    bind_bearer: bool,
) -> ApiResult<Option<PresignReady>> {
    if bind_bearer {
        return Ok(None);
    }
    let Some(client) = state.storage.original_store.client.clone() else {
        return Ok(None);
    };
    let Some(signer) = client.blobs.signer() else {
        return Ok(None);
    };
    let Some(tenant) = scope
        .tenant_id
        .as_deref()
        .and_then(|s| uuid::Uuid::parse_str(s).ok())
    else {
        return Ok(None);
    };
    let Some(workspace) = scope
        .workspace_id
        .as_deref()
        .and_then(|s| uuid::Uuid::parse_str(s).ok())
    else {
        return Ok(None);
    };
    let key = edgequake_storage::originals::object_key(
        &client.prefix,
        edgequake_storage::originals::OwnerKind::Upload,
        tenant,
        workspace,
        uuid::Uuid::new_v4(),
    );
    match edgequake_storage::originals::presign_put(
        signer.as_ref(),
        &key,
        Duration::from_secs(upload_ttl_secs()),
        media_type,
        sha256,
        client.blobs.checksum_header(),
    )
    .await
    {
        Ok(signed) => {
            if edgequake_storage::originals::reject_insecure_presign(
                &signed.url,
                client.blobs.allow_http_presign(),
            )
            .is_err()
            {
                return Ok(None);
            }
            Ok(Some(PresignReady {
                url: signed.url.to_string(),
                headers: signed.headers,
                key,
            }))
        }
        Err(edgequake_storage::originals::OriginalsError::NotSupported(_)) => Ok(None),
        Err(err) => Err(ApiError::Internal(format!("presign: {err}"))),
    }
}

async fn finish_begin(state: &AppState, begun: BeginFinish) -> ApiResult<Value> {
    let BeginFinish {
        access,
        scope,
        upload_id,
        filename,
        media_type,
        byte_length,
        sha256,
        transfer,
        url,
        headers,
        staging_key,
        expires,
        bind_bearer,
    } = begun;
    let meta = UploadMeta {
        access,
        upload_id: upload_id.clone(),
        scope: scope.clone(),
        filename,
        media_type: media_type.clone(),
        byte_length,
        sha256: sha256.clone(),
        staging_key,
        transfer,
        bind_bearer,
        status: UploadStatus::Open,
        received: 0,
        expires_at_unix: expires,
        document_id: None,
        task_id: None,
        commit_status: None,
        admitting: false,
    };
    state
        .mcp_uploads
        .inner
        .insert(meta)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    if transfer == UploadTransfer::Chunk {
        let _ = state
            .mcp_uploads
            .inner
            .write_bytes(&scope, &upload_id, b"")
            .await;
    }
    let mut envelope = EnvelopeBuilder::new("upload_begin", BudgetClass::Standard)
        .insert("upload_id", json!(upload_id))
        .insert("status", json!("open"))
        .insert("transfer", json!(transfer.as_str()))
        .insert("expires_at", json!(expires));
    if let Some(url) = url {
        let header_map: serde_json::Map<String, Value> =
            headers.into_iter().map(|(k, v)| (k, json!(v))).collect();
        envelope = envelope
            .insert("method", json!("PUT"))
            .insert("url", json!(url))
            .insert("headers", Value::Object(header_map))
            .insert("sha256", json!(sha256))
            .insert("byte_length", json!(byte_length))
            .insert("instruction", json!(INSTRUCTION));
    }
    Ok(envelope.build())
}

pub async fn eq_upload_write(
    state: &AppState,
    tenant_ctx: &TenantContext,
    args: &Value,
) -> ApiResult<Value> {
    let upload_id = args
        .get("upload_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| ApiError::BadRequest("upload_id required".into()))?;
    let offset = args.get("offset").and_then(|v| v.as_u64()).unwrap_or(0);
    let chunk_b64 = args
        .get("chunk_base64")
        .and_then(|v| v.as_str())
        .or_else(|| args.get("data_base64").and_then(|v| v.as_str()))
        .unwrap_or("");
    let chunk = match STANDARD.decode(chunk_b64) {
        Ok(b) => b,
        Err(_) => {
            return Ok(eq_error(
                ErrorCode::InvalidId,
                "chunk_base64 must be standard Base64",
                None,
            ));
        }
    };
    let scope = scope_of(tenant_ctx);
    let Some(mut meta) = state
        .mcp_uploads
        .inner
        .get(&scope, upload_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
    else {
        return Ok(eq_error(
            ErrorCode::NotFound,
            format!("upload_id not found: {upload_id}"),
            None,
        ));
    };
    if let Some(err) = gate_open(&meta, tenant_ctx) {
        return Ok(err);
    }
    if meta.transfer != UploadTransfer::Chunk {
        return Ok(eq_error(
            ErrorCode::InvalidId,
            "bytes for this upload_id belong on the PUT url",
            None,
        ));
    }
    if offset != meta.received {
        return Ok(eq_error(
            ErrorCode::InvalidId,
            format!(
                "offset must be the next contiguous byte ({}); got {offset}",
                meta.received
            ),
            None,
        ));
    }
    let next = meta.received + chunk.len() as u64;
    if next as usize > state.config.max_document_size {
        return Ok(eq_error(
            ErrorCode::BudgetExceeded,
            "upload exceeds max document size",
            None,
        ));
    }
    if let Some(max) = meta.byte_length {
        if next > max {
            return Ok(eq_error(
                ErrorCode::BudgetExceeded,
                "chunk exceeds declared byte_length",
                None,
            ));
        }
    }
    state
        .mcp_uploads
        .inner
        .append_bytes(&scope, upload_id, &chunk, next)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    meta.received = next;
    state
        .mcp_uploads
        .inner
        .save(&scope, &meta)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    Ok(EnvelopeBuilder::new("upload_write", BudgetClass::Standard)
        .insert("upload_id", json!(upload_id))
        .insert("offset", json!(next))
        .insert("received_bytes", json!(next))
        .build())
}

pub async fn eq_upload_commit(
    state: &AppState,
    tenant_ctx: &TenantContext,
    args: &Value,
) -> ApiResult<Value> {
    let upload_id = args
        .get("upload_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| ApiError::BadRequest("upload_id required".into()))?;
    commit_upload_id(state, tenant_ctx, upload_id).await
}

pub async fn eq_upload_abort(
    state: &AppState,
    tenant_ctx: &TenantContext,
    args: &Value,
) -> ApiResult<Value> {
    let upload_id = args
        .get("upload_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| ApiError::BadRequest("upload_id required".into()))?;
    let scope = scope_of(tenant_ctx);
    let Some(mut meta) = state
        .mcp_uploads
        .inner
        .get(&scope, upload_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
    else {
        return Ok(eq_error(
            ErrorCode::NotFound,
            format!("upload_id not found: {upload_id}"),
            None,
        ));
    };
    if !owner_matches(&meta, tenant_ctx) {
        return Ok(eq_error(
            ErrorCode::Forbidden,
            "upload_id belongs to another workspace",
            None,
        ));
    }
    if meta.status != UploadStatus::Open {
        return Ok(eq_error(ErrorCode::NotFound, "upload_id is not open", None));
    }
    meta.status = UploadStatus::Aborted;
    state
        .mcp_uploads
        .inner
        .save(&scope, &meta)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    let _ = state
        .mcp_uploads
        .inner
        .delete_bytes(&scope, upload_id)
        .await;
    drop_staging(state, meta.staging_key.as_deref()).await;
    Ok(EnvelopeBuilder::new("upload_abort", BudgetClass::Standard)
        .insert("upload_id", json!(upload_id))
        .insert("status", json!("aborted"))
        .build())
}

pub async fn commit_upload_id(
    state: &AppState,
    tenant_ctx: &TenantContext,
    upload_id: &str,
) -> ApiResult<Value> {
    let scope = scope_of(tenant_ctx);
    let meta = loop {
        match state
            .mcp_uploads
            .inner
            .claim(&scope, upload_id)
            .await
            .map_err(|e| ApiError::Internal(e.to_string()))?
        {
            UploadClaim::Ready(meta) => break meta,
            UploadClaim::Missing => {
                return Ok(eq_error(
                    ErrorCode::NotFound,
                    format!("upload_id not found: {upload_id}"),
                    None,
                ));
            }
            UploadClaim::Terminal(meta) => {
                if !owner_matches(&meta, tenant_ctx) {
                    return Ok(eq_error(
                        ErrorCode::Forbidden,
                        "upload_id belongs to another workspace",
                        None,
                    ));
                }
                if meta.status == UploadStatus::Committed {
                    return Ok(idempotent_commit_envelope(&meta));
                }
                return Ok(eq_error(ErrorCode::NotFound, "upload_id is not open", None));
            }
            UploadClaim::Busy => tokio::time::sleep(Duration::from_millis(25)).await,
        }
    };
    if !owner_matches(&meta, tenant_ctx) {
        release_claim(state, &meta).await?;
        return Ok(eq_error(
            ErrorCode::Forbidden,
            "upload_id belongs to another workspace",
            None,
        ));
    }
    let bytes = match load_bytes(state, &meta).await {
        Ok(bytes) => bytes,
        Err(err) => {
            release_claim(state, &meta).await?;
            return Ok(err);
        }
    };
    if bytes.is_empty() {
        release_claim(state, &meta).await?;
        return Ok(eq_error(ErrorCode::NotFound, "upload bytes missing", None));
    }
    if let Some(exp) = meta.byte_length {
        if bytes.len() as u64 != exp {
            release_claim(state, &meta).await?;
            return Ok(eq_error(
                ErrorCode::InvalidId,
                "received bytes do not match declared byte_length",
                None,
            ));
        }
    }
    if let Some(exp) = &meta.sha256 {
        let got = hex::encode(Sha256::digest(&bytes));
        if !got.eq_ignore_ascii_case(exp) {
            release_claim(state, &meta).await?;
            return Ok(eq_error(ErrorCode::InvalidId, "sha256 mismatch", None));
        }
    }
    let mut admit_ctx = tenant_ctx.clone();
    if admit_ctx.tenant_id_uuid().is_none() {
        admit_ctx.tenant_id = Some(crate::middleware::default_tenant_uuid().to_string());
    }
    if admit_ctx.workspace_id_uuid().is_none() {
        admit_ctx.workspace_id = Some(crate::middleware::default_workspace_uuid().to_string());
    }
    let access = meta
        .access
        .clone()
        .map(serde_json::from_value)
        .transpose()
        .map_err(|_| {
            ApiError::bad_request_coded("INVALID_ACCESS_PAYLOAD", "Invalid upload access")
        })?;
    let envelope = admit_bytes_with_access(
        state,
        &admit_ctx,
        &meta.filename,
        &meta.media_type,
        bytes,
        access,
    )
    .await?;
    if envelope.get("ok").and_then(|v| v.as_bool()) == Some(false) {
        release_claim(state, &meta).await?;
        return Ok(envelope);
    }
    let mut done = meta.clone();
    done.status = UploadStatus::Committed;
    done.admitting = false;
    done.document_id = envelope
        .get("document_id")
        .and_then(|v| v.as_str())
        .map(str::to_string);
    done.task_id = envelope
        .get("task_id")
        .and_then(|v| v.as_str())
        .map(str::to_string);
    done.commit_status = envelope
        .get("status")
        .and_then(|v| v.as_str())
        .map(str::to_string);
    state
        .mcp_uploads
        .inner
        .save(&scope, &done)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    let _ = state
        .mcp_uploads
        .inner
        .delete_bytes(&scope, upload_id)
        .await;
    drop_staging(state, done.staging_key.as_deref()).await;
    Ok(envelope)
}

/// HTTPS PUT target for a relay upload. The query string is the capability.
pub async fn accept_relay_body(
    state: &AppState,
    upload_id: &str,
    query: &[(String, String)],
    content_type: &str,
    authorization: Option<&str>,
    body: &[u8],
) -> Result<(), RelayReject> {
    let q = |name: &str| {
        query
            .iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.as_str())
    };
    let (Some(exp), Some(sig), Some(sha), Some(len_s)) = (q("exp"), q("sig"), q("sha"), q("len"))
    else {
        return Err(RelayReject::Forbidden);
    };
    let tid = q("tid").unwrap_or("");
    let wid = q("wid").unwrap_or("");
    let Ok(len) = len_s.parse::<u64>() else {
        return Err(RelayReject::Forbidden);
    };
    let Some(key) = mac_key(state) else {
        return Err(RelayReject::Forbidden);
    };
    let message = mac_message(upload_id, exp, sha, content_type, len, tid, wid);
    if !verify_mac(&key, &message, sig) {
        return Err(RelayReject::Forbidden);
    }
    let scope = UploadScope {
        tenant_id: Some(tid.to_string()).filter(|s| !s.is_empty()),
        workspace_id: Some(wid.to_string()).filter(|s| !s.is_empty()),
        user_id: None,
    };
    let Some(meta) = state
        .mcp_uploads
        .inner
        .get(&scope, upload_id)
        .await
        .map_err(|_| RelayReject::Forbidden)?
    else {
        return Err(RelayReject::NotFound);
    };
    if meta.status != UploadStatus::Open || upload_expired(now_unix(), meta.expires_at_unix) {
        return Err(if meta.status == UploadStatus::Open {
            RelayReject::NotFound
        } else {
            RelayReject::Conflict
        });
    }
    if meta.transfer != UploadTransfer::Relay {
        return Err(RelayReject::Conflict);
    }
    if meta.bind_bearer {
        let Some(header) = authorization.filter(|h| !h.is_empty()) else {
            return Err(RelayReject::Forbidden);
        };
        if state.auth.config.auth_enabled {
            let token = header.trim_start_matches("Bearer ").trim();
            if crate::services::auth_validation::decide(
                state,
                token,
                crate::services::auth_validation::TokenProfile::McpResource,
                &axum::http::HeaderMap::new(),
            )
            .await
            .ok()
            .flatten()
            .is_none()
            {
                return Err(RelayReject::Forbidden);
            }
        }
    }
    if !content_type.eq_ignore_ascii_case(&meta.media_type) {
        return Err(RelayReject::Forbidden);
    }
    if meta
        .sha256
        .as_deref()
        .is_some_and(|row| !row.eq_ignore_ascii_case(sha))
    {
        return Err(RelayReject::Forbidden);
    }
    if body.len() as u64 > len {
        return Err(RelayReject::TooLarge);
    }
    if body.len() as u64 != len {
        return Err(RelayReject::Forbidden);
    }
    let got = hex::encode(Sha256::digest(body));
    if !got.eq_ignore_ascii_case(sha) {
        return Err(RelayReject::Forbidden);
    }
    tracing::info!(
        upload_id,
        byte_length = body.len(),
        "mcp upload relay stored"
    );
    state
        .mcp_uploads
        .inner
        .write_bytes(&scope, upload_id, body)
        .await
        .map_err(|_| RelayReject::Forbidden)?;
    Ok(())
}

#[derive(Debug)]
pub enum RelayReject {
    Forbidden,
    NotFound,
    Conflict,
    TooLarge,
}

fn gate_open(meta: &UploadMeta, tenant_ctx: &TenantContext) -> Option<Value> {
    if upload_expired(now_unix(), meta.expires_at_unix) || meta.status != UploadStatus::Open {
        return Some(eq_error(ErrorCode::NotFound, "upload_id expired", None));
    }
    if !owner_matches(meta, tenant_ctx) {
        return Some(eq_error(
            ErrorCode::Forbidden,
            "upload_id belongs to another workspace",
            None,
        ));
    }
    None
}

fn owner_matches(meta: &UploadMeta, tenant_ctx: &TenantContext) -> bool {
    if meta.scope.workspace_id != tenant_ctx.workspace_id
        || meta.scope.tenant_id != tenant_ctx.tenant_id
    {
        return false;
    }
    match (&meta.scope.user_id, &tenant_ctx.user_id) {
        (Some(row), Some(caller)) => row == caller,
        _ => true,
    }
}

fn scope_of(ctx: &TenantContext) -> UploadScope {
    UploadScope {
        tenant_id: ctx.tenant_id.clone(),
        workspace_id: ctx.workspace_id.clone(),
        user_id: ctx.user_id.clone(),
    }
}

async fn load_bytes(state: &AppState, meta: &UploadMeta) -> Result<Vec<u8>, Value> {
    if let Some(key) = &meta.staging_key {
        let Some(client) = state.storage.original_store.client.as_ref() else {
            return Err(eq_error(ErrorCode::NotFound, "upload bytes missing", None));
        };
        return match client.blobs.get_all(key).await {
            Ok(bytes) => Ok(bytes.to_vec()),
            Err(_) => Err(eq_error(ErrorCode::NotFound, "upload bytes missing", None)),
        };
    }
    state
        .mcp_uploads
        .inner
        .read_bytes(&meta.scope, &meta.upload_id)
        .await
        .map_err(|_| eq_error(ErrorCode::NotFound, "upload bytes missing", None))
}

async fn release_claim(state: &AppState, meta: &UploadMeta) -> ApiResult<()> {
    let mut row = meta.clone();
    row.admitting = false;
    row.commit_status = None;
    state
        .mcp_uploads
        .inner
        .save(&row.scope, &row)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    Ok(())
}

async fn drop_staging(state: &AppState, key: Option<&str>) {
    let Some(key) = key else {
        return;
    };
    let Some(client) = state.storage.original_store.client.as_ref() else {
        return;
    };
    let _ = client.blobs.delete_key(key).await;
}

fn idempotent_commit_envelope(meta: &UploadMeta) -> Value {
    EnvelopeBuilder::new("ingest", BudgetClass::Standard)
        .insert("document_id", json!(meta.document_id))
        .insert("task_id", json!(meta.task_id))
        .insert(
            "status",
            json!(meta.commit_status.as_deref().unwrap_or("pending")),
        )
        .insert("ready", json!(false))
        .build()
}

fn is_sha256_hex(value: &str) -> bool {
    value.len() == 64 && value.chars().all(|c| c.is_ascii_hexdigit())
}

fn bearer_bind() -> bool {
    std::env::var("EDGEQUAKE_MCP_UPLOAD_BIND")
        .ok()
        .is_some_and(|v| v.eq_ignore_ascii_case("bearer"))
}

fn mac_key(state: &AppState) -> Option<String> {
    if let Ok(key) = std::env::var("EDGEQUAKE_MCP_UPLOAD_HMAC_KEY") {
        if !key.is_empty() {
            return Some(key);
        }
    }
    let secret = &state.auth.config.jwt_secret;
    if secret == edgequake_auth::DEFAULT_INSECURE_JWT_SECRET {
        return None;
    }
    Some(secret.clone())
}

fn mac_message(
    upload_id: &str,
    exp: &str,
    sha256: &str,
    content_type: &str,
    byte_length: u64,
    tenant_id: &str,
    workspace_id: &str,
) -> String {
    format!(
        "{upload_id}\n{exp}\n{sha256}\n{content_type}\n{byte_length}\n{tenant_id}\n{workspace_id}"
    )
}

fn sign_mac(key: &str, message: &str) -> String {
    let mut mac = HmacSha256::new_from_slice(key.as_bytes()).expect("hmac key");
    mac.update(message.as_bytes());
    hex::encode(mac.finalize().into_bytes())
}

fn verify_mac(key: &str, message: &str, sig_hex: &str) -> bool {
    let Ok(sig) = hex::decode(sig_hex) else {
        return false;
    };
    let Ok(mut mac) = HmacSha256::new_from_slice(key.as_bytes()) else {
        return false;
    };
    mac.update(message.as_bytes());
    mac.verify_slice(&sig).is_ok()
}

pub async fn reap_uploads(state: &AppState) -> Result<(), String> {
    let keys = state
        .mcp_uploads
        .inner
        .reclaim_expired()
        .await
        .map_err(|e| e.to_string())?;
    for key in keys {
        drop_staging(state, Some(&key)).await;
    }
    Ok(())
}

pub async fn admit_bytes(
    state: &AppState,
    tenant_ctx: &TenantContext,
    filename: &str,
    media_type: &str,
    bytes: Vec<u8>,
) -> ApiResult<Value> {
    admit_bytes_with_access(state, tenant_ctx, filename, media_type, bytes, None).await
}

pub async fn admit_bytes_with_access(
    state: &AppState,
    tenant_ctx: &TenantContext,
    filename: &str,
    media_type: &str,
    bytes: Vec<u8>,
    access: Option<crate::services::document_access_types::DocumentAccess>,
) -> ApiResult<Value> {
    let is_pdf = filename.to_ascii_lowercase().ends_with(".pdf")
        || media_type.eq_ignore_ascii_case("application/pdf");
    if is_pdf {
        let resp = crate::handlers::pdf_upload::admit_pdf_bytes_with_access(
            state,
            tenant_ctx,
            filename.to_string(),
            bytes,
            access.clone(),
        )
        .await?;
        let status = if resp.status == "duplicate_processing" {
            "duplicate_processing"
        } else {
            "pending"
        };
        return Ok(EnvelopeBuilder::new("ingest", BudgetClass::Standard)
            .insert(
                "document_id",
                json!(resp.document_id.unwrap_or_else(|| resp.pdf_id.clone())),
            )
            .insert("task_id", json!(resp.task_id))
            .insert("status", json!(status))
            .insert("ready", json!(false))
            .insert(
                "poll",
                json!({
                    "tool": "eq_task_get",
                    "until": ["indexed", "failed", "cancelled"],
                }),
            )
            .build());
    }

    let resolved = crate::services::resolve_upload_content(
        state,
        tenant_ctx.workspace_id_uuid(),
        filename,
        &bytes,
    )
    .await?;
    let content_hash = ContentHasher::hash_bytes(&bytes);
    let outcome = admit_document_for_processing(
        state,
        tenant_ctx,
        DocumentAdmissionInput {
            access,
            text_content: resolved.text_content,
            title: filename.to_string(),
            source_type: resolved.meta.source_type,
            mime_type: Some(resolved.mime_type.clone()),
            raw_byte_size: bytes.len(),
            content_hash,
            custom_metadata: UploadExtraction {
                mode: Some("llm".into()),
                gate_preset: None,
            }
            .merge_into(None),
            track_id: None,
            expected_batch_count: None,
            gleaning: GleaningAdmissionOptions::default(),
            document_type: None,
            chunk_strategy: None,
            chunk_options: None,
            extract_max_entities: None,
            extract_max_records: None,
            multimodal: resolved.meta.multimodal,
            ingest_mode: resolved.meta.ingest_mode,
            multimodal_manifest: resolved.manifest,
        },
        "upload",
    )
    .await?;
    let env = envelope_from_admission(outcome);
    #[cfg(feature = "postgres")]
    if let Some(doc_id) = env.get("document_id").and_then(|v| v.as_str()) {
        let _ = crate::services::persist_uploaded_original(
            state,
            tenant_ctx,
            doc_id,
            filename,
            &resolved.mime_type,
            resolved.meta.source_type,
            &bytes,
        )
        .await;
    }
    Ok(env)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::middleware::{default_tenant_uuid, default_workspace_uuid, TenantContext};

    fn ctx(ws: &str) -> TenantContext {
        TenantContext {
            tenant_id: Some(default_tenant_uuid().to_string()),
            workspace_id: Some(ws.to_string()),
            user_id: None,
        }
    }

    #[tokio::test]
    async fn expired_upload_id_is_not_found() {
        let state = AppState::test_state();
        let tenant = ctx(&default_workspace_uuid().to_string());
        let begin = eq_upload_begin(
            &state,
            &tenant,
            &json!({"filename": "a.txt", "byte_length": 3, "chunked": true}),
        )
        .await
        .unwrap();
        let uid = begin["upload_id"].as_str().unwrap().to_string();
        assert!(state.mcp_uploads.mark_expired(&uid).await);
        let write = eq_upload_write(
            &state,
            &tenant,
            &json!({
                "upload_id": uid,
                "offset": 0,
                "chunk_base64": STANDARD.encode(b"abc"),
            }),
        )
        .await
        .unwrap();
        assert_eq!(write["ok"], false);
        assert_eq!(write["error"]["code"], "eq/not_found");
    }

    #[tokio::test]
    async fn cross_workspace_upload_denied() {
        let state = AppState::test_state();
        let owner = ctx(&default_workspace_uuid().to_string());
        let other = ctx("00000000-0000-0000-0000-000000000099");
        let begin = eq_upload_begin(
            &state,
            &owner,
            &json!({"filename": "a.txt", "chunked": true}),
        )
        .await
        .unwrap();
        let uid = begin["upload_id"].as_str().unwrap();
        let write = eq_upload_write(
            &state,
            &other,
            &json!({
                "upload_id": uid,
                "offset": 0,
                "chunk_base64": STANDARD.encode(b"abc"),
            }),
        )
        .await
        .unwrap();
        assert_eq!(write["error"]["code"], "eq/forbidden");
    }

    #[tokio::test]
    async fn write_over_max_document_size_budget() {
        let mut state = AppState::test_state();
        state.config.max_document_size = 4;
        let tenant = ctx(&default_workspace_uuid().to_string());
        let begin = eq_upload_begin(
            &state,
            &tenant,
            &json!({"filename": "a.txt", "chunked": true}),
        )
        .await
        .unwrap();
        let uid = begin["upload_id"].as_str().unwrap();
        let write = eq_upload_write(
            &state,
            &tenant,
            &json!({
                "upload_id": uid,
                "offset": 0,
                "chunk_base64": STANDARD.encode(b"0123456789"),
            }),
        )
        .await
        .unwrap();
        assert_eq!(write["error"]["code"], "eq/budget_exceeded");
    }

    #[tokio::test]
    async fn data_base64_alias_appends() {
        let state = AppState::test_state();
        let tenant = ctx(&default_workspace_uuid().to_string());
        let begin = eq_upload_begin(
            &state,
            &tenant,
            &json!({"filename": "a.txt", "chunked": true}),
        )
        .await
        .unwrap();
        let uid = begin["upload_id"].as_str().unwrap();
        let write = eq_upload_write(
            &state,
            &tenant,
            &json!({
                "upload_id": uid,
                "offset": 0,
                "data_base64": STANDARD.encode(b"ab"),
            }),
        )
        .await
        .unwrap();
        assert_eq!(write["ok"], true);
        assert_eq!(write["received_bytes"], 2);
    }

    #[tokio::test]
    async fn different_user_is_forbidden() {
        let state = AppState::test_state();
        let mut owner = ctx(&default_workspace_uuid().to_string());
        owner.user_id = Some("user-a".into());
        let mut other = owner.clone();
        other.user_id = Some("user-b".into());
        let begin = eq_upload_begin(
            &state,
            &owner,
            &json!({"filename": "a.txt", "chunked": true}),
        )
        .await
        .unwrap();
        let uid = begin["upload_id"].as_str().unwrap();
        let write = eq_upload_write(
            &state,
            &other,
            &json!({
                "upload_id": uid,
                "offset": 0,
                "chunk_base64": STANDARD.encode(b"a"),
            }),
        )
        .await
        .unwrap();
        assert_eq!(write["error"]["code"], "eq/forbidden");
    }

    #[tokio::test]
    async fn reaper_removes_an_expired_upload() {
        let state = AppState::test_state();
        let tenant = ctx(&default_workspace_uuid().to_string());
        let begin = eq_upload_begin(
            &state,
            &tenant,
            &json!({"filename": "a.txt", "chunked": true}),
        )
        .await
        .unwrap();
        let uid = begin["upload_id"].as_str().unwrap().to_string();
        eq_upload_write(
            &state,
            &tenant,
            &json!({
                "upload_id": uid,
                "offset": 0,
                "chunk_base64": STANDARD.encode(b"abc"),
            }),
        )
        .await
        .unwrap();
        assert!(state.mcp_uploads.mark_expired(&uid).await);
        reap_uploads(&state).await.unwrap();
        let commit = eq_upload_commit(&state, &tenant, &json!({"upload_id": uid}))
            .await
            .unwrap();
        assert_eq!(commit["error"]["code"], "eq/not_found");
    }

    #[tokio::test]
    async fn bearer_bind_requires_an_authorization_header() {
        std::env::set_var("EDGEQUAKE_MCP_UPLOAD_BIND", "bearer");
        let mut state = AppState::test_state();
        state.auth.config.jwt_secret = "spec173-bearer-test-key-long-enough".into();
        let tenant = ctx(&default_workspace_uuid().to_string());
        let begin = eq_upload_begin(
            &state,
            &tenant,
            &json!({
                "filename": "a.txt",
                "media_type": "text/plain",
                "byte_length": 4,
                "sha256": hex::encode(Sha256::digest(b"bind")),
            }),
        )
        .await
        .unwrap();
        std::env::remove_var("EDGEQUAKE_MCP_UPLOAD_BIND");
        assert_eq!(begin["transfer"], "relay", "{begin}");
        let url = begin["url"].as_str().unwrap();
        let query = url.split('?').nth(1).unwrap();
        let pairs: Vec<(String, String)> = query
            .split('&')
            .filter_map(|part| {
                let (k, v) = part.split_once('=')?;
                Some((k.to_string(), v.to_string()))
            })
            .collect();
        let id = begin["upload_id"].as_str().unwrap();
        let denied = accept_relay_body(&state, id, &pairs, "text/plain", None, b"bind").await;
        assert!(denied.is_err());
        accept_relay_body(
            &state,
            id,
            &pairs,
            "text/plain",
            Some("Bearer test-token"),
            b"bind",
        )
        .await
        .unwrap();
    }
}
