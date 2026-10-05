//! SPEC-091 W3 shared helpers — seed workspaces/documents/chunks and legacy
//! chunk vectors so the typed `chunk_embeddings` cutover can be exercised.
#![allow(dead_code)]

use std::fs::File;
use std::io;
use std::path::PathBuf;

use sqlx::PgPool;
use uuid::Uuid;

pub const W3_STEP: &str = "w3-chunk-embedding-backfill";

/// Serialize env mutation + shared schema across W3 tests in one binary.
pub fn w3_lock() -> &'static tokio::sync::Mutex<()> {
    static LOCK: std::sync::OnceLock<tokio::sync::Mutex<()>> = std::sync::OnceLock::new();
    LOCK.get_or_init(|| tokio::sync::Mutex::new(()))
}

/// Cross-process exclusive lockfile so parallel cargo test binaries cannot race
/// `EDGEQUAKE_VECTOR_BACKEND` (in-process [`w3_lock`] alone is not enough).
pub struct W3EnvFileLock {
    path: PathBuf,
    _file: File,
}

impl W3EnvFileLock {
    pub async fn acquire() -> io::Result<Self> {
        let path = std::env::temp_dir().join("edgequake-w3-vector-backend.lock");
        // Stale lock from a crashed test older than 10 minutes — reclaim.
        if let Ok(meta) = std::fs::metadata(&path) {
            if let Ok(modified) = meta.modified() {
                if modified.elapsed().unwrap_or_default().as_secs() > 600 {
                    let _ = std::fs::remove_file(&path);
                }
            }
        }
        loop {
            match File::options().write(true).create_new(true).open(&path) {
                Ok(file) => {
                    return Ok(Self { path, _file: file });
                }
                Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {
                    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                }
                Err(e) => return Err(e),
            }
        }
    }
}

impl Drop for W3EnvFileLock {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

/// Hold both in-process mutex and cross-process lockfile for VECTOR_BACKEND tests.
pub async fn w3_env_guard() -> (tokio::sync::MutexGuard<'static, ()>, W3EnvFileLock) {
    let g = w3_lock().lock().await;
    let file = W3EnvFileLock::acquire().await.expect("w3 env lockfile");
    (g, file)
}

/// Insert a workspace row (tenant + workspace) and return its UUID.
pub async fn seed_workspace(pool: &PgPool, tag: &str) -> Uuid {
    let tenant = Uuid::new_v4();
    let ws = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO tenants (tenant_id, name, slug, is_active) VALUES ($1, $2, $3, TRUE) \
         ON CONFLICT (tenant_id) DO NOTHING",
    )
    .bind(tenant)
    .bind(format!("w3-{tag}"))
    .bind(format!("w3-{}", tenant.as_simple()))
    .execute(pool)
    .await
    .expect("seed tenant");
    sqlx::query(
        "INSERT INTO workspaces (workspace_id, tenant_id, name, slug) VALUES ($1, $2, $3, $4) \
         ON CONFLICT (workspace_id) DO NOTHING",
    )
    .bind(ws)
    .bind(tenant)
    .bind(format!("w3-{tag}"))
    .bind(format!("w3-{}", ws.as_simple()))
    .execute(pool)
    .await
    .expect("seed workspace");
    ws
}

/// Insert a document row; returns its UUID.
pub async fn seed_document(pool: &PgPool, workspace: Uuid) -> Uuid {
    let doc = Uuid::new_v4();
    let tenant = workspace_tenant(pool, workspace).await;
    sqlx::query(
        "INSERT INTO documents (id, workspace_id, title, content, status, tenant_id) \
         VALUES ($1, $2, 'w3 doc', 'w3 content', 'indexed', $3)",
    )
    .bind(doc)
    .bind(workspace)
    .bind(tenant)
    .execute(pool)
    .await
    .expect("seed document");
    doc
}

/// Fixture rows use the same tenant/workspace ownership as scoped provider reads.
pub async fn workspace_tenant(pool: &PgPool, workspace: Uuid) -> Uuid {
    sqlx::query_scalar("SELECT tenant_id FROM workspaces WHERE workspace_id = $1")
        .bind(workspace)
        .fetch_one(pool)
        .await
        .expect("workspace has a tenant")
}

/// Insert a relational chunk row; returns its `chunks.id`.
pub async fn seed_chunk(
    pool: &PgPool,
    doc: Uuid,
    workspace: Uuid,
    chunk_index: i32,
    content: &str,
) -> Uuid {
    let id = Uuid::new_v4();
    let tenant = workspace_tenant(pool, workspace).await;
    sqlx::query(
        "INSERT INTO chunks (id, document_id, workspace_id, chunk_index, content, metadata, tenant_id) \
         VALUES ($1, $2, $3, $4, $5, $6, $7)",
    )
    .bind(id)
    .bind(doc)
    .bind(workspace)
    .bind(chunk_index)
    .bind(content)
    .bind(serde_json::json!({"legacy_chunk_key": format!("{doc}-chunk-{chunk_index}")}))
    .bind(tenant)
    .execute(pool)
    .await
    .expect("seed chunk");
    id
}

/// Deterministic pseudo-embedding of length `dim` (no RNG dependency).
pub fn make_embedding(dim: usize, seed: u32) -> Vec<f32> {
    (0..dim)
        .map(|i| {
            let x = (seed.wrapping_mul(31).wrapping_add(i as u32) % 1000) as f32 / 1000.0;
            x * 2.0 - 1.0
        })
        .collect()
}

pub fn vector_to_text(v: &[f32]) -> String {
    let mut s = String::from("[");
    for (i, x) in v.iter().enumerate() {
        if i > 0 {
            s.push(',');
        }
        s.push_str(&x.to_string());
    }
    s.push(']');
    s
}

/// Create an isolated legacy vectors table `public.eq_<ns>_vectors`.
pub async fn create_vectors_table(pool: &PgPool, ns: &str) -> String {
    let table = format!("eq_{ns}_vectors");
    // Dedicated connection + ROLLBACK: prior `sqlx::raw_sql` multi-statement
    // failures can return a pooled conn stuck in 25P02 (aborted TX).
    let mut conn = pool
        .acquire()
        .await
        .expect("acquire for create_vectors_table");
    let _ = sqlx::query("ROLLBACK").execute(&mut *conn).await;
    sqlx::query(&format!("DROP TABLE IF EXISTS public.{table} CASCADE"))
        .execute(&mut *conn)
        .await
        .expect("drop vectors table");
    sqlx::query(&format!(
        "CREATE TABLE public.{table} (id TEXT PRIMARY KEY, embedding vector, metadata JSONB DEFAULT '{{}}')"
    ))
    .execute(&mut *conn)
    .await
    .expect("create vectors table");
    table
}

/// Drop every `eq_%_vectors` relation except `keep` so global advisor posture
/// sees a sole fixture table (SPEC-111 honesty closeout / E2E-111-11+).
pub async fn drop_all_vector_tables_except(pool: &PgPool, keep: &str) {
    let tables: Vec<String> = sqlx::query_scalar(
        "SELECT c.relname FROM pg_class c \
         JOIN pg_namespace n ON n.oid = c.relnamespace \
         WHERE n.nspname = 'public' AND c.relkind = 'r' \
           AND c.relname LIKE 'eq\\_%\\_vectors' ESCAPE '\\'",
    )
    .fetch_all(pool)
    .await
    .unwrap_or_default();
    for t in tables {
        if t == keep {
            continue;
        }
        let _ = sqlx::query(&format!("DROP TABLE IF EXISTS public.{t} CASCADE"))
            .execute(pool)
            .await;
    }
}

/// Seed a legacy chunk vector row (`{doc}-chunk-{i}`) into `table`.
pub async fn seed_legacy_chunk_vector(
    pool: &PgPool,
    table: &str,
    doc: Uuid,
    chunk_index: i32,
    embedding: &[f32],
) {
    sqlx::query(&format!(
        "INSERT INTO public.{table} (id, embedding) VALUES ($1, $2::vector) \
         ON CONFLICT (id) DO NOTHING"
    ))
    .bind(format!("{doc}-chunk-{chunk_index}"))
    .bind(vector_to_text(embedding))
    .execute(pool)
    .await
    .expect("seed legacy chunk vector");
}

/// Clean up a W3 workspace cascade (embeddings → chunks → document → workspace).
pub async fn cleanup_workspace(pool: &PgPool, workspace: Uuid) {
    sqlx::query("DELETE FROM chunk_embeddings WHERE workspace_id = $1")
        .bind(workspace)
        .execute(pool)
        .await
        .ok();
    sqlx::query("DELETE FROM chunks WHERE workspace_id = $1")
        .bind(workspace)
        .execute(pool)
        .await
        .ok();
    sqlx::query("DELETE FROM documents WHERE workspace_id = $1")
        .bind(workspace)
        .execute(pool)
        .await
        .ok();
    sqlx::query("DELETE FROM workspaces WHERE workspace_id = $1")
        .bind(workspace)
        .execute(pool)
        .await
        .ok();
}

/// Drop an arbitrary public table (test artifact).
pub async fn drop_table(pool: &PgPool, table: &str) {
    sqlx::query(&format!("DROP TABLE IF EXISTS public.{table} CASCADE"))
        .execute(pool)
        .await
        .ok();
}

/// Relational residue counts for a workspace (SPEC-091 IW5 scale/delete proofs).
pub async fn count_workspace_residue(pool: &PgPool, workspace: Uuid) -> (i64, i64, i64) {
    let chunks: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM chunks WHERE workspace_id = $1")
        .bind(workspace)
        .fetch_one(pool)
        .await
        .unwrap_or(0);
    let embeddings: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM chunk_embeddings WHERE workspace_id = $1")
            .bind(workspace)
            .fetch_one(pool)
            .await
            .unwrap_or(0);
    let documents: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM documents WHERE workspace_id = $1")
            .bind(workspace)
            .fetch_one(pool)
            .await
            .unwrap_or(0);
    (chunks, embeddings, documents)
}

/// Bulk-insert relational chunks for one document (batched unnest).
pub async fn seed_chunks_bulk(pool: &PgPool, doc: Uuid, workspace: Uuid, count: usize) {
    const BATCH: usize = 500;
    let tenant = workspace_tenant(pool, workspace).await;
    for batch_start in (0..count).step_by(BATCH) {
        let batch_end = (batch_start + BATCH).min(count);
        let n = batch_end - batch_start;
        let ids: Vec<Uuid> = (0..n).map(|_| Uuid::new_v4()).collect();
        let docs: Vec<Uuid> = vec![doc; n];
        let workspaces: Vec<Uuid> = vec![workspace; n];
        let indexes: Vec<i32> = (batch_start..batch_end).map(|i| i as i32).collect();
        let contents: Vec<String> = indexes.iter().map(|i| format!("bulk chunk {i}")).collect();
        let metadata: Vec<serde_json::Value> = indexes
            .iter()
            .map(|i| serde_json::json!({"legacy_chunk_key": format!("{doc}-chunk-{i}")}))
            .collect();
        sqlx::query(
            "INSERT INTO chunks (id, document_id, workspace_id, chunk_index, content, metadata, tenant_id) \
             SELECT t.*, $7 FROM unnest($1::uuid[], $2::uuid[], $3::uuid[], $4::int[], $5::text[], $6::jsonb[]) AS t",
        )
        .bind(&ids)
        .bind(&docs)
        .bind(&workspaces)
        .bind(&indexes)
        .bind(&contents)
        .bind(&metadata)
        .bind(tenant)
        .execute(pool)
        .await
        .expect("seed chunks bulk");
    }
}

/// Typed embeddings for existing chunk rows (one model, fixed dim).
pub async fn seed_typed_embeddings_bulk(
    pool: &PgPool,
    workspace: Uuid,
    model_name: &str,
    dim: usize,
    seed_base: u32,
) {
    let chunk_ids: Vec<Uuid> =
        sqlx::query_scalar("SELECT id FROM chunks WHERE workspace_id = $1 ORDER BY chunk_index")
            .bind(workspace)
            .fetch_all(pool)
            .await
            .expect("list chunks");
    if chunk_ids.is_empty() {
        return;
    }
    let model_id: Uuid = sqlx::query_scalar(
        "INSERT INTO embedding_models (name, dimensions) VALUES ($1, $2) \
         ON CONFLICT (name, dimensions) DO UPDATE SET name = EXCLUDED.name RETURNING id",
    )
    .bind(model_name)
    .bind(dim as i32)
    .fetch_one(pool)
    .await
    .expect("model");
    const BATCH: usize = 200;
    for batch_start in (0..chunk_ids.len()).step_by(BATCH) {
        let batch = &chunk_ids[batch_start..batch_start + BATCH.min(chunk_ids.len() - batch_start)];
        let workspaces: Vec<Uuid> = vec![workspace; batch.len()];
        let dims: Vec<i32> = vec![dim as i32; batch.len()];
        let vectors: Vec<String> = (0..batch.len())
            .map(|i| {
                vector_to_text(&make_embedding(
                    dim,
                    seed_base + batch_start as u32 + i as u32,
                ))
            })
            .collect();
        sqlx::query(
            "INSERT INTO chunk_embeddings (model_id, chunk_id, workspace_id, embedding, dimensions) \
             SELECT $1, c, w, v::halfvec, d \
             FROM unnest($2::uuid[], $3::uuid[], $4::text[], $5::int[]) AS t(c, w, v, d) \
             ON CONFLICT (model_id, chunk_id) DO NOTHING",
        )
        .bind(model_id)
        .bind(batch)
        .bind(&workspaces)
        .bind(&vectors)
        .bind(&dims)
        .execute(pool)
        .await
        .expect("seed typed embeddings bulk");
    }
}

/// Product-shaped workspace delete (FK cascade on typed relational SSOT).
pub async fn delete_workspace_cascade(pool: &PgPool, workspace: Uuid) {
    sqlx::query("DELETE FROM workspaces WHERE workspace_id = $1")
        .bind(workspace)
        .execute(pool)
        .await
        .expect("delete workspace");
}

/// Remove W3 engine job rows (ledger cleanup between tests).
pub async fn clear_w3_job(pool: &PgPool) {
    sqlx::query(
        "DELETE FROM edgequake.edgequake_migration_batch WHERE job_id IN \
         (SELECT job_id FROM edgequake.edgequake_migration_job WHERE step_id = $1)",
    )
    .bind(W3_STEP)
    .execute(pool)
    .await
    .ok();
    sqlx::query("DELETE FROM edgequake.edgequake_migration_job WHERE step_id = $1")
        .bind(W3_STEP)
        .execute(pool)
        .await
        .ok();
}
