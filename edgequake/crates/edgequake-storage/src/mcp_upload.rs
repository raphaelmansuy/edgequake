//! MCP upload sessions (SPEC-173). One row shape for the in-process store and Postgres.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use async_trait::async_trait;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UploadTransfer {
    Presign,
    Relay,
    Chunk,
}

impl UploadTransfer {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Presign => "presign",
            Self::Relay => "relay",
            Self::Chunk => "chunk",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "presign" => Some(Self::Presign),
            "relay" => Some(Self::Relay),
            "chunk" => Some(Self::Chunk),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UploadStatus {
    Open,
    Committed,
    Aborted,
    Expired,
}

impl UploadStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::Committed => "committed",
            Self::Aborted => "aborted",
            Self::Expired => "expired",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "open" => Some(Self::Open),
            "committed" => Some(Self::Committed),
            "aborted" => Some(Self::Aborted),
            "expired" => Some(Self::Expired),
            _ => None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct UploadScope {
    pub tenant_id: Option<String>,
    pub workspace_id: Option<String>,
    pub user_id: Option<String>,
}

#[derive(Clone, Debug)]
pub struct UploadMeta {
    pub access: Option<serde_json::Value>,
    pub upload_id: String,
    pub scope: UploadScope,
    pub filename: String,
    pub media_type: String,
    pub byte_length: Option<u64>,
    pub sha256: Option<String>,
    pub staging_key: Option<String>,
    pub transfer: UploadTransfer,
    pub bind_bearer: bool,
    pub status: UploadStatus,
    pub received: u64,
    pub expires_at_unix: i64,
    pub document_id: Option<String>,
    pub task_id: Option<String>,
    pub commit_status: Option<String>,
    pub admitting: bool,
}

#[derive(Debug)]
pub enum UploadClaim {
    Ready(UploadMeta),
    Terminal(UploadMeta),
    Missing,
    Busy,
}

#[derive(Debug, thiserror::Error)]
#[error("{0}")]
pub struct UploadFault(pub String);

#[async_trait]
pub trait UploadSessions: Send + Sync {
    async fn insert(&self, meta: UploadMeta) -> Result<(), UploadFault>;
    async fn get(&self, scope: &UploadScope, id: &str) -> Result<Option<UploadMeta>, UploadFault>;
    async fn save(&self, scope: &UploadScope, meta: &UploadMeta) -> Result<(), UploadFault>;
    async fn claim(&self, scope: &UploadScope, id: &str) -> Result<UploadClaim, UploadFault>;
    async fn mark_expired(&self, id: &str) -> Result<bool, UploadFault>;
    async fn read_bytes(&self, scope: &UploadScope, id: &str) -> Result<Vec<u8>, UploadFault>;
    async fn append_bytes(
        &self,
        scope: &UploadScope,
        id: &str,
        chunk: &[u8],
        received: u64,
    ) -> Result<(), UploadFault>;
    async fn write_bytes(
        &self,
        scope: &UploadScope,
        id: &str,
        bytes: &[u8],
    ) -> Result<(), UploadFault>;
    async fn delete_bytes(&self, scope: &UploadScope, id: &str) -> Result<(), UploadFault>;
    /// Mark expired open rows, clear leftover staging bytes, and return object keys to delete.
    async fn reclaim_expired(&self) -> Result<Vec<String>, UploadFault>;
}

pub fn now_unix() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// A URL is expired only after this many seconds past `expires_at` (SPEC-173 clock skew).
pub const UPLOAD_EXPIRY_SKEW_SECS: i64 = 30;

pub fn upload_expired(now: i64, expires_at_unix: i64) -> bool {
    now > expires_at_unix.saturating_add(UPLOAD_EXPIRY_SKEW_SECS)
}

pub fn temp_path(upload_id: &str) -> PathBuf {
    std::env::temp_dir()
        .join("edgequake-mcp-uploads")
        .join(upload_id)
}

pub struct MemoryUploadSessions {
    inner: Mutex<HashMap<String, UploadMeta>>,
}

impl MemoryUploadSessions {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(HashMap::new()),
        }
    }
}

impl Default for MemoryUploadSessions {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl UploadSessions for MemoryUploadSessions {
    async fn insert(&self, meta: UploadMeta) -> Result<(), UploadFault> {
        let mut map = self.inner.lock().map_err(|e| UploadFault(e.to_string()))?;
        map.insert(meta.upload_id.clone(), meta);
        Ok(())
    }

    async fn get(&self, _scope: &UploadScope, id: &str) -> Result<Option<UploadMeta>, UploadFault> {
        let map = self.inner.lock().map_err(|e| UploadFault(e.to_string()))?;
        Ok(map.get(id).cloned())
    }

    async fn save(&self, _scope: &UploadScope, meta: &UploadMeta) -> Result<(), UploadFault> {
        let mut map = self.inner.lock().map_err(|e| UploadFault(e.to_string()))?;
        map.insert(meta.upload_id.clone(), meta.clone());
        Ok(())
    }

    async fn claim(&self, _scope: &UploadScope, id: &str) -> Result<UploadClaim, UploadFault> {
        let mut map = self.inner.lock().map_err(|e| UploadFault(e.to_string()))?;
        let Some(row) = map.get_mut(id) else {
            return Ok(UploadClaim::Missing);
        };
        if row.status != UploadStatus::Open {
            return Ok(UploadClaim::Terminal(row.clone()));
        }
        if upload_expired(now_unix(), row.expires_at_unix) {
            row.status = UploadStatus::Expired;
            return Ok(UploadClaim::Terminal(row.clone()));
        }
        if row.admitting {
            return Ok(UploadClaim::Busy);
        }
        row.admitting = true;
        Ok(UploadClaim::Ready(row.clone()))
    }

    async fn mark_expired(&self, id: &str) -> Result<bool, UploadFault> {
        let mut map = self.inner.lock().map_err(|e| UploadFault(e.to_string()))?;
        let Some(row) = map.get_mut(id) else {
            return Ok(false);
        };
        row.expires_at_unix = now_unix() - UPLOAD_EXPIRY_SKEW_SECS - 5;
        Ok(true)
    }

    async fn read_bytes(&self, _scope: &UploadScope, id: &str) -> Result<Vec<u8>, UploadFault> {
        let path = temp_path(id);
        if !path.exists() {
            return Ok(Vec::new());
        }
        tokio::fs::read(&path)
            .await
            .map_err(|e| UploadFault(format!("upload read: {e}")))
    }

    async fn append_bytes(
        &self,
        _scope: &UploadScope,
        id: &str,
        chunk: &[u8],
        received: u64,
    ) -> Result<(), UploadFault> {
        let _ = received;
        write_chunk(&temp_path(id), chunk, true).await
    }

    async fn write_bytes(
        &self,
        _scope: &UploadScope,
        id: &str,
        bytes: &[u8],
    ) -> Result<(), UploadFault> {
        write_chunk(&temp_path(id), bytes, false).await
    }

    async fn delete_bytes(&self, _scope: &UploadScope, id: &str) -> Result<(), UploadFault> {
        let path = temp_path(id);
        if path.exists() {
            tokio::fs::remove_file(&path)
                .await
                .map_err(|e| UploadFault(format!("upload delete: {e}")))?;
        }
        Ok(())
    }

    async fn reclaim_expired(&self) -> Result<Vec<String>, UploadFault> {
        let now = now_unix();
        let mut keys = Vec::new();
        let mut ids = Vec::new();
        {
            let mut map = self.inner.lock().map_err(|e| UploadFault(e.to_string()))?;
            for row in map.values_mut() {
                let open_expired =
                    row.status == UploadStatus::Open && upload_expired(now, row.expires_at_unix);
                let leftover = row.staging_key.is_some()
                    && row.status != UploadStatus::Open
                    && upload_expired(now, row.expires_at_unix);
                if !open_expired && !leftover {
                    continue;
                }
                if let Some(key) = row.staging_key.take() {
                    keys.push(key);
                }
                if open_expired {
                    row.status = UploadStatus::Expired;
                }
                ids.push(row.upload_id.clone());
            }
        }
        for id in ids {
            let path = temp_path(&id);
            if path.exists() {
                let _ = tokio::fs::remove_file(path).await;
            }
        }
        Ok(keys)
    }
}

async fn write_chunk(path: &Path, bytes: &[u8], append: bool) -> Result<(), UploadFault> {
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|e| UploadFault(format!("upload dir: {e}")))?;
    }
    if append && path.exists() {
        use tokio::io::AsyncWriteExt;
        let mut file = tokio::fs::OpenOptions::new()
            .append(true)
            .open(path)
            .await
            .map_err(|e| UploadFault(format!("upload write: {e}")))?;
        file.write_all(bytes)
            .await
            .map_err(|e| UploadFault(format!("upload write: {e}")))?;
        return Ok(());
    }
    tokio::fs::write(path, bytes)
        .await
        .map_err(|e| UploadFault(format!("upload write: {e}")))
}

#[cfg(feature = "postgres")]
mod pg {
    use super::*;
    use uuid::Uuid;

    pub struct PgUploadSessions {
        pool: sqlx::PgPool,
    }

    impl PgUploadSessions {
        pub fn new(pool: sqlx::PgPool) -> Self {
            Self { pool }
        }
    }

    fn parse_uuid(value: &Option<String>) -> Option<Uuid> {
        value.as_deref().and_then(|s| Uuid::parse_str(s).ok())
    }

    async fn begin_scope(
        pool: &sqlx::PgPool,
        scope: &UploadScope,
    ) -> Result<sqlx::Transaction<'static, sqlx::Postgres>, UploadFault> {
        let tenant = parse_uuid(&scope.tenant_id)
            .ok_or_else(|| UploadFault("upload tenant id is not a uuid".into()))?;
        let workspace = parse_uuid(&scope.workspace_id);
        let user = parse_uuid(&scope.user_id);
        let mut tx = pool
            .begin()
            .await
            .map_err(|e| UploadFault(format!("upload pool: {e}")))?;
        // set_tenant_context is transaction-local (migration 096). SET LOCAL ROLE
        // makes a superuser connection subject to row security.
        crate::adapters::postgres::rls::enforce_tenant_context(&mut tx, tenant, workspace, user)
            .await
            .map_err(|e| UploadFault(format!("upload tenant context: {e}")))?;
        Ok(tx)
    }

    #[async_trait]
    impl UploadSessions for PgUploadSessions {
        async fn insert(&self, meta: UploadMeta) -> Result<(), UploadFault> {
            let mut tx = begin_scope(&self.pool, &meta.scope).await?;
            let tenant = parse_uuid(&meta.scope.tenant_id).unwrap();
            let workspace = parse_uuid(&meta.scope.workspace_id)
                .ok_or_else(|| UploadFault("upload workspace id is not a uuid".into()))?;
            sqlx::query(
                "INSERT INTO mcp_upload_sessions (
                    upload_id, tenant_id, workspace_id, user_id, filename, media_type,
                    byte_length, sha256, staging_key, transfer, bind, status, received,
                    expires_at, access
                 ) VALUES (
                    $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, to_timestamp($14), $15
                 )",
            )
            .bind(&meta.upload_id)
            .bind(tenant)
            .bind(workspace)
            .bind(parse_uuid(&meta.scope.user_id))
            .bind(&meta.filename)
            .bind(&meta.media_type)
            .bind(meta.byte_length.map(|n| n as i64))
            .bind(&meta.sha256)
            .bind(&meta.staging_key)
            .bind(meta.transfer.as_str())
            .bind(if meta.bind_bearer {
                "bearer"
            } else {
                "capability"
            })
            .bind(meta.status.as_str())
            .bind(meta.received as i64)
            .bind(meta.expires_at_unix as f64)
            .bind(&meta.access)
            .execute(&mut *tx)
            .await
            .map_err(|e| UploadFault(format!("upload insert: {e}")))?;
            tx.commit()
                .await
                .map_err(|e| UploadFault(format!("upload insert: {e}")))?;
            Ok(())
        }

        async fn get(
            &self,
            scope: &UploadScope,
            id: &str,
        ) -> Result<Option<UploadMeta>, UploadFault> {
            let mut tx = begin_scope(&self.pool, scope).await?;
            let row = sqlx::query_as::<_, PgRow>(
                "SELECT upload_id, tenant_id, workspace_id, user_id, filename, media_type,
                        byte_length, sha256, staging_key, transfer, bind, status, received,
                        extract(epoch FROM expires_at)::bigint AS expires_at_unix,
                        document_id, task_id, commit_status, access
                 FROM mcp_upload_sessions WHERE upload_id = $1",
            )
            .bind(id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|e| UploadFault(format!("upload get: {e}")))?;
            tx.commit()
                .await
                .map_err(|e| UploadFault(format!("upload get: {e}")))?;
            Ok(row.map(PgRow::into_meta))
        }

        async fn save(&self, scope: &UploadScope, meta: &UploadMeta) -> Result<(), UploadFault> {
            let mut tx = begin_scope(&self.pool, scope).await?;
            sqlx::query(
                "UPDATE mcp_upload_sessions SET
                    status = $2, received = $3, staging_key = $4, document_id = $5,
                    task_id = $6, commit_status = $7, sha256 = $8, byte_length = $9
                 WHERE upload_id = $1",
            )
            .bind(&meta.upload_id)
            .bind(meta.status.as_str())
            .bind(meta.received as i64)
            .bind(&meta.staging_key)
            .bind(&meta.document_id)
            .bind(&meta.task_id)
            .bind(&meta.commit_status)
            .bind(&meta.sha256)
            .bind(meta.byte_length.map(|n| n as i64))
            .execute(&mut *tx)
            .await
            .map_err(|e| UploadFault(format!("upload save: {e}")))?;
            tx.commit()
                .await
                .map_err(|e| UploadFault(format!("upload save: {e}")))?;
            Ok(())
        }

        async fn claim(&self, scope: &UploadScope, id: &str) -> Result<UploadClaim, UploadFault> {
            let mut tx = begin_scope(&self.pool, scope).await?;
            let updated = sqlx::query(
                "UPDATE mcp_upload_sessions SET commit_status = 'admitting'
                 WHERE upload_id = $1 AND status = 'open' AND commit_status IS NULL
                   AND expires_at + interval '30 seconds' > now()",
            )
            .bind(id)
            .execute(&mut *tx)
            .await
            .map_err(|e| UploadFault(format!("upload claim: {e}")))?;
            tx.commit()
                .await
                .map_err(|e| UploadFault(format!("upload claim: {e}")))?;
            if updated.rows_affected() == 1 {
                let Some(mut meta) = self.get(scope, id).await? else {
                    return Ok(UploadClaim::Missing);
                };
                meta.admitting = true;
                return Ok(UploadClaim::Ready(meta));
            }
            match self.get(scope, id).await? {
                None => Ok(UploadClaim::Missing),
                Some(meta)
                    if meta.commit_status.as_deref() == Some("admitting")
                        && meta.status == UploadStatus::Open =>
                {
                    Ok(UploadClaim::Busy)
                }
                Some(meta) => Ok(UploadClaim::Terminal(meta)),
            }
        }

        async fn mark_expired(&self, id: &str) -> Result<bool, UploadFault> {
            let mut tx = self
                .pool
                .begin()
                .await
                .map_err(|e| UploadFault(format!("upload expire: {e}")))?;
            sqlx::query("SELECT set_config('edgequake.mcp_upload_reaper', '1', true)")
                .execute(&mut *tx)
                .await
                .map_err(|e| UploadFault(format!("upload expire: {e}")))?;
            let result = sqlx::query(
                "UPDATE mcp_upload_sessions SET expires_at = to_timestamp(0) WHERE upload_id = $1",
            )
            .bind(id)
            .execute(&mut *tx)
            .await
            .map_err(|e| UploadFault(format!("upload expire: {e}")))?;
            tx.commit()
                .await
                .map_err(|e| UploadFault(format!("upload expire: {e}")))?;
            Ok(result.rows_affected() == 1)
        }

        async fn read_bytes(&self, scope: &UploadScope, id: &str) -> Result<Vec<u8>, UploadFault> {
            let mut tx = begin_scope(&self.pool, scope).await?;
            let bytes: Option<Vec<u8>> = sqlx::query_scalar(
                "SELECT staging_bytes FROM mcp_upload_sessions WHERE upload_id = $1",
            )
            .bind(id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|e| UploadFault(format!("upload read: {e}")))?
            .flatten();
            tx.commit()
                .await
                .map_err(|e| UploadFault(format!("upload read: {e}")))?;
            Ok(bytes.unwrap_or_default())
        }

        async fn append_bytes(
            &self,
            scope: &UploadScope,
            id: &str,
            chunk: &[u8],
            received: u64,
        ) -> Result<(), UploadFault> {
            let mut tx = begin_scope(&self.pool, scope).await?;
            sqlx::query(
                "UPDATE mcp_upload_sessions
                 SET staging_bytes = coalesce(staging_bytes, ''::bytea) || $2, received = $3
                 WHERE upload_id = $1",
            )
            .bind(id)
            .bind(chunk)
            .bind(received as i64)
            .execute(&mut *tx)
            .await
            .map_err(|e| UploadFault(format!("upload append: {e}")))?;
            tx.commit()
                .await
                .map_err(|e| UploadFault(format!("upload append: {e}")))?;
            Ok(())
        }

        async fn write_bytes(
            &self,
            scope: &UploadScope,
            id: &str,
            bytes: &[u8],
        ) -> Result<(), UploadFault> {
            let mut tx = begin_scope(&self.pool, scope).await?;
            sqlx::query(
                "UPDATE mcp_upload_sessions SET staging_bytes = $2, received = $3 WHERE upload_id = $1",
            )
            .bind(id)
            .bind(bytes)
            .bind(bytes.len() as i64)
            .execute(&mut *tx)
            .await
            .map_err(|e| UploadFault(format!("upload write: {e}")))?;
            tx.commit()
                .await
                .map_err(|e| UploadFault(format!("upload write: {e}")))?;
            Ok(())
        }

        async fn delete_bytes(&self, scope: &UploadScope, id: &str) -> Result<(), UploadFault> {
            let mut tx = begin_scope(&self.pool, scope).await?;
            sqlx::query("UPDATE mcp_upload_sessions SET staging_bytes = NULL WHERE upload_id = $1")
                .bind(id)
                .execute(&mut *tx)
                .await
                .map_err(|e| UploadFault(format!("upload clear: {e}")))?;
            tx.commit()
                .await
                .map_err(|e| UploadFault(format!("upload clear: {e}")))?;
            Ok(())
        }

        async fn reclaim_expired(&self) -> Result<Vec<String>, UploadFault> {
            let mut tx = self
                .pool
                .begin()
                .await
                .map_err(|e| UploadFault(format!("upload reclaim: {e}")))?;
            sqlx::query("SELECT set_config('edgequake.mcp_upload_reaper', '1', true)")
                .execute(&mut *tx)
                .await
                .map_err(|e| UploadFault(format!("upload reclaim: {e}")))?;
            let rows: Vec<(Option<String>,)> = sqlx::query_as(
                "                 WITH doomed AS (
                    SELECT upload_id, staging_key FROM mcp_upload_sessions
                    WHERE (status = 'open' AND expires_at + interval '30 seconds' <= now())
                       OR (staging_key IS NOT NULL AND status <> 'open'
                           AND expires_at + interval '30 seconds' <= now())
                 )
                 UPDATE mcp_upload_sessions AS s
                 SET status = CASE WHEN s.status = 'open' THEN 'expired' ELSE s.status END,
                     staging_bytes = NULL,
                     staging_key = NULL
                 FROM doomed AS d
                 WHERE s.upload_id = d.upload_id
                 RETURNING d.staging_key",
            )
            .fetch_all(&mut *tx)
            .await
            .map_err(|e| UploadFault(format!("upload reclaim: {e}")))?;
            tx.commit()
                .await
                .map_err(|e| UploadFault(format!("upload reclaim: {e}")))?;
            Ok(rows.into_iter().filter_map(|row| row.0).collect())
        }
    }

    #[derive(sqlx::FromRow)]
    struct PgRow {
        access: Option<serde_json::Value>,
        upload_id: String,
        tenant_id: Uuid,
        workspace_id: Uuid,
        user_id: Option<Uuid>,
        filename: String,
        media_type: String,
        byte_length: Option<i64>,
        sha256: Option<String>,
        staging_key: Option<String>,
        transfer: String,
        bind: String,
        status: String,
        received: i64,
        expires_at_unix: i64,
        document_id: Option<String>,
        task_id: Option<String>,
        commit_status: Option<String>,
    }

    impl PgRow {
        fn into_meta(self) -> UploadMeta {
            UploadMeta {
                access: self.access,
                upload_id: self.upload_id,
                scope: UploadScope {
                    tenant_id: Some(self.tenant_id.to_string()),
                    workspace_id: Some(self.workspace_id.to_string()),
                    user_id: self.user_id.map(|id| id.to_string()),
                },
                filename: self.filename,
                media_type: self.media_type,
                byte_length: self.byte_length.map(|n| n as u64),
                sha256: self.sha256,
                staging_key: self.staging_key,
                transfer: UploadTransfer::parse(&self.transfer).unwrap_or(UploadTransfer::Chunk),
                bind_bearer: self.bind == "bearer",
                status: UploadStatus::parse(&self.status).unwrap_or(UploadStatus::Open),
                received: self.received as u64,
                expires_at_unix: self.expires_at_unix,
                document_id: self.document_id,
                task_id: self.task_id,
                admitting: self.commit_status.as_deref() == Some("admitting"),
                commit_status: self.commit_status,
            }
        }
    }

    pub fn postgres(pool: sqlx::PgPool) -> std::sync::Arc<dyn UploadSessions> {
        std::sync::Arc::new(PgUploadSessions::new(pool))
    }
}

#[cfg(feature = "postgres")]
pub use pg::postgres;

#[cfg(test)]
mod expiry_tests {
    use super::upload_expired;

    #[test]
    fn thirty_seconds_of_skew_is_still_open() {
        assert!(!upload_expired(1_000, 990));
        assert!(upload_expired(1_000, 960));
    }
}
