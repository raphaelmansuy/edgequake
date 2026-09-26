//! SPEC-091 Wave B4/B5 — typed sidecar store (migration 116 tables).
//!
//! Single owner for `public.pipeline_checkpoints` and `public.document_artifacts`
//! I/O, replacing the per-document KV blobs (`{doc}-pipeline-checkpoint`,
//! `{doc}-extraction-snapshot`, `{doc}-lineage`, `{doc}-multimodal-manifest`,
//! `{doc}-multimodal-chunks`, MM cache).
//!
//! Cutover pattern (identical to B1/B3):
//! - **Writes**: callers keep KV authoritative; every save also lands here
//!   (warn-only) so the typed table converges while KV can still be rolled
//!   back to with a flag flip.
//! - **Reads**: flag-gated (`EDGEQUAKE_KV_FAMILY_CHECKPOINT` /
//!   `EDGEQUAKE_KV_FAMILY_ARTIFACT` = relational) typed-first; any gap
//!   (flag off, no pool, non-UUID doc id, typed miss/error) falls back to KV.
//!
//! A temporary process registry serves legacy call sites. It owns one
//! replaceable `Arc<PgPool>`; re-registration drops the previous outer owner,
//! so separate test runtimes do not accumulate leaked pools.

use serde_json::Value;
use std::sync::{Arc, RwLock};

use edgequake_storage::contracts::CheckpointArtifactStore;
use edgequake_storage::kv_family_cutover::{
    kv_family_mode_from_env, KvFamilyMode, KV_FAMILY_ARTIFACT, KV_FAMILY_CHECKPOINT,
};

pub const CHECKPOINT_KIND_CRASH: &str = "checkpoint";
pub const CHECKPOINT_KIND_SNAPSHOT: &str = "snapshot";
pub const ARTIFACT_KIND_LINEAGE: &str = "lineage";
pub const ARTIFACT_KIND_MM_MANIFEST: &str = "multimodal-manifest";
pub const ARTIFACT_KIND_MM_CHUNKS: &str = "multimodal-chunks";

struct SidecarStoreRegistry {
    store: RwLock<Option<Arc<dyn CheckpointArtifactStore>>>,
}

impl SidecarStoreRegistry {
    const fn new() -> Self {
        Self {
            store: RwLock::new(None),
        }
    }

    fn replace(&self, store: Arc<dyn CheckpointArtifactStore>) {
        *self.store.write().expect("sidecar store lock") = Some(store);
    }

    fn get(&self) -> Option<Arc<dyn CheckpointArtifactStore>> {
        self.store
            .read()
            .expect("sidecar store lock")
            .as_ref()
            .map(Arc::clone)
    }
}

static SIDECAR_STORE: SidecarStoreRegistry = SidecarStoreRegistry::new();

tokio::task_local! {
    static INSTALLED_PORTS: InstalledPorts;
}

/// Ports for the current request or task. Explicit values win over the process registry.
#[derive(Clone, Default)]
pub struct InstalledPorts {
    #[cfg(feature = "postgres")]
    pool: Option<Arc<sqlx::PgPool>>,
    store: Option<Arc<dyn CheckpointArtifactStore>>,
}

impl InstalledPorts {
    pub fn new(
        #[cfg(feature = "postgres")] pool: Option<Arc<sqlx::PgPool>>,
        store: Option<Arc<dyn CheckpointArtifactStore>>,
    ) -> Self {
        Self {
            #[cfg(feature = "postgres")]
            pool,
            store,
        }
    }
}

/// Run `future` with operational ports visible to sidecar readers.
pub async fn with_ports<F, T>(ports: InstalledPorts, future: F) -> T
where
    F: std::future::Future<Output = T>,
{
    INSTALLED_PORTS.scope(ports, future).await
}

fn installed_store() -> Option<Arc<dyn CheckpointArtifactStore>> {
    INSTALLED_PORTS
        .try_with(|ports| ports.store.clone())
        .ok()
        .flatten()
}

#[cfg(feature = "postgres")]
fn installed_pool() -> Option<Arc<sqlx::PgPool>> {
    INSTALLED_PORTS
        .try_with(|ports| ports.pool.clone())
        .ok()
        .flatten()
}

#[cfg(feature = "postgres")]
static LEGACY_SIDECAR_POOL: RwLock<Option<Arc<sqlx::PgPool>>> = RwLock::new(None);

/// Install a provider-independent sidecar store.
pub fn register_sidecar_store(store: Arc<dyn CheckpointArtifactStore>) {
    SIDECAR_STORE.replace(store);
}

/// Register the Postgres pool for all sidecar I/O.
///
/// Re-registration **replaces** the stored pool (last call wins). Production
/// registers once at startup; tests register a fresh pool per runtime. A
/// `OnceLock` (first-call-wins) is wrong for tests because each `#[tokio::test]`
/// may run its own runtime. Keeping one replaceable `Arc` lets a later
/// `AppState` replace the prior pool without leaking it forever.
#[cfg(feature = "postgres")]
pub fn register_sidecar_pool(pool: impl Into<Arc<sqlx::PgPool>>) {
    let pool = pool.into();
    *LEGACY_SIDECAR_POOL
        .write()
        .expect("legacy sidecar pool lock") = Some(Arc::clone(&pool));
    register_sidecar_store(Arc::new(
        crate::services::postgres_checkpoint_artifact_store::PostgresCheckpointArtifactStore::new(
            pool.as_ref().clone(),
        ),
    ));
}

/// Compatibility accessor for relational groups not yet extracted in J21.
#[cfg(feature = "postgres")]
pub fn sidecar_pool() -> Option<Arc<sqlx::PgPool>> {
    if let Some(pool) = installed_pool() {
        return Some(pool);
    }
    LEGACY_SIDECAR_POOL
        .read()
        .expect("legacy sidecar pool lock")
        .as_ref()
        .map(Arc::clone)
}

pub fn sidecar_store() -> Option<Arc<dyn CheckpointArtifactStore>> {
    if let Some(store) = installed_store() {
        return Some(store);
    }
    SIDECAR_STORE.get()
}

pub fn checkpoints_prefer_relational() -> bool {
    kv_family_mode_from_env(KV_FAMILY_CHECKPOINT) == KvFamilyMode::Relational
}

pub fn artifacts_prefer_relational() -> bool {
    kv_family_mode_from_env(KV_FAMILY_ARTIFACT) == KvFamilyMode::Relational
}

/// SPEC-091 Wave D: typed writes are warn-only during dual-write; once the
/// family flag flips relational they are the ONLY write (the adapter write-stop
/// drops the KV upsert) — escalate failures to error! so an authoritative
/// loss is loud. Rollback = flip the family flag back to `kv`.
fn log_write_failure(relational: bool, op: &str, document_id: &str, kind: &str, e: &str) {
    if relational {
        tracing::error!(
            document_id,
            kind,
            op,
            error = %e,
            "SPEC-091: authoritative typed sidecar write FAILED — data is not persisted; \
             investigate the typed store or roll the family flag back to kv"
        );
    } else {
        tracing::warn!(document_id, kind, op, error = %e, "typed sidecar dual-write failed (KV remains)");
    }
}

/// Parse a UUID document id — typed sidecars are keyed by `documents.id`.
fn doc_uuid(document_id: &str) -> Option<uuid::Uuid> {
    uuid::Uuid::parse_str(document_id).ok()
}

// ── pipeline_checkpoints ────────────────────────────────────────────────────

/// True when a typed checkpoint write can land (store present + UUID doc id).
pub fn typed_checkpoint_writable(
    store: Option<&dyn CheckpointArtifactStore>,
    document_id: &str,
) -> bool {
    store.is_some() && doc_uuid(document_id).is_some()
}

/// Typed upsert (warn-only). No-op without a store or for non-UUID ids.
/// Returns whether the row was written successfully.
pub async fn typed_checkpoint_put(
    store: Option<&dyn CheckpointArtifactStore>,
    document_id: &str,
    kind: &str,
    payload: &Value,
) -> bool {
    let (Some(store), Some(doc)) = (store, doc_uuid(document_id)) else {
        return false;
    };
    match store.put_checkpoint(doc, kind, payload).await {
        Ok(()) => true,
        Err(error) => {
            log_write_failure(
                checkpoints_prefer_relational(),
                "checkpoint_upsert",
                document_id,
                kind,
                &error.to_string(),
            );
            false
        }
    }
}

/// Typed read. `None` on miss, error, no store, or non-UUID id (→ KV fallback).
pub async fn typed_checkpoint_get(
    store: Option<&dyn CheckpointArtifactStore>,
    document_id: &str,
    kind: &str,
) -> Option<Value> {
    let (store, doc) = (store?, doc_uuid(document_id)?);
    match store.get_checkpoint(doc, kind).await {
        Ok(value) => value,
        Err(error) => {
            tracing::warn!(document_id, kind, error = %error, "typed checkpoint read failed");
            None
        }
    }
}

/// Typed delete (warn-only), paired with the caller's KV delete.
pub async fn typed_checkpoint_delete(
    store: Option<&dyn CheckpointArtifactStore>,
    document_id: &str,
    kind: &str,
) {
    let (Some(store), Some(doc)) = (store, doc_uuid(document_id)) else {
        return;
    };
    if let Err(error) = store.delete_checkpoint(doc, kind).await {
        log_write_failure(
            checkpoints_prefer_relational(),
            "checkpoint_delete",
            document_id,
            kind,
            &error.to_string(),
        );
    }
}

/// Startup sweep mirroring `cleanup_stale_checkpoints` for typed rows.
pub async fn cleanup_stale_typed_checkpoints(
    store: Option<&dyn CheckpointArtifactStore>,
    max_age_secs: u64,
) {
    let Some(store) = store else { return };
    match store.cleanup_stale_checkpoints(max_age_secs).await {
        Ok(cleaned) if cleaned > 0 => tracing::info!(
            cleaned,
            "cleaned up stale typed pipeline checkpoints on startup"
        ),
        Ok(_) => {}
        Err(error) => tracing::warn!(error = %error, "typed checkpoint sweep failed"),
    }
}

// ── document_artifacts ──────────────────────────────────────────────────────

/// Typed upsert (warn-only). No-op without a store or for non-UUID ids.
pub async fn typed_artifact_put(
    store: Option<&dyn CheckpointArtifactStore>,
    document_id: &str,
    kind: &str,
    payload: &Value,
) {
    let (Some(store), Some(doc)) = (store, doc_uuid(document_id)) else {
        return;
    };
    if let Err(error) = store.put_artifact(doc, kind, payload).await {
        log_write_failure(
            artifacts_prefer_relational(),
            "artifact_upsert",
            document_id,
            kind,
            &error.to_string(),
        );
    }
}

/// Typed read. `None` on miss, error, no store, or non-UUID id (→ KV fallback).
pub async fn typed_artifact_get(
    store: Option<&dyn CheckpointArtifactStore>,
    document_id: &str,
    kind: &str,
) -> Option<Value> {
    let (store, doc) = (store?, doc_uuid(document_id)?);
    match store.get_artifact(doc, kind).await {
        Ok(value) => value,
        Err(error) => {
            tracing::warn!(document_id, kind, error = %error, "typed artifact read failed");
            None
        }
    }
}

/// Delete every typed artifact for a document (deletion parity with the
/// legacy per-family KV key deletes).
pub async fn typed_artifact_delete_all(
    store: Option<&dyn CheckpointArtifactStore>,
    document_id: &str,
) {
    let (Some(store), Some(doc)) = (store, doc_uuid(document_id)) else {
        return;
    };
    if let Err(error) = store.delete_artifacts(doc).await {
        log_write_failure(
            artifacts_prefer_relational(),
            "artifact_delete_all",
            document_id,
            "*",
            &error.to_string(),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// SPEC-091 Wave D: flags default to RELATIONAL; typed accessors are
    /// still inert without a registered pool (memory/test mode unaffected),
    /// and the `kv` rollback env keeps working.
    #[tokio::test]
    async fn typed_accessors_inert_without_pool() {
        std::env::remove_var("EDGEQUAKE_KV_FAMILY_CHECKPOINT");
        std::env::remove_var("EDGEQUAKE_KV_FAMILY_ARTIFACT");
        assert!(checkpoints_prefer_relational());
        assert!(artifacts_prefer_relational());
        typed_checkpoint_put(
            None,
            "doc",
            CHECKPOINT_KIND_CRASH,
            &serde_json::json!({"a": 1}),
        )
        .await;
        typed_artifact_put(
            None,
            "doc",
            ARTIFACT_KIND_LINEAGE,
            &serde_json::json!({"b": 2}),
        )
        .await;
        typed_checkpoint_delete(None, "doc", CHECKPOINT_KIND_CRASH).await;
        typed_artifact_delete_all(None, "doc").await;

        std::env::set_var("EDGEQUAKE_KV_FAMILY_CHECKPOINT", "kv");
        std::env::set_var("EDGEQUAKE_KV_FAMILY_ARTIFACT", "kv");
        assert!(!checkpoints_prefer_relational());
        assert!(!artifacts_prefer_relational());
        std::env::remove_var("EDGEQUAKE_KV_FAMILY_CHECKPOINT");
        std::env::remove_var("EDGEQUAKE_KV_FAMILY_ARTIFACT");
    }

    #[cfg(feature = "postgres")]
    #[tokio::test]
    async fn sidecar_store_replacement_drops_previous_arc_owner() {
        use crate::services::postgres_checkpoint_artifact_store::PostgresCheckpointArtifactStore;

        let registry = SidecarStoreRegistry::new();
        let first = Arc::new(PostgresCheckpointArtifactStore::new(
            sqlx::postgres::PgPoolOptions::new()
                .connect_lazy("postgres://localhost/edgequake")
                .expect("lazy first pool"),
        ));
        let first_weak = Arc::downgrade(&first);
        registry
            .replace(Arc::clone(&first)
                as Arc<dyn edgequake_storage::contracts::CheckpointArtifactStore>);
        drop(first);
        assert!(first_weak.upgrade().is_some());

        let second = Arc::new(PostgresCheckpointArtifactStore::new(
            sqlx::postgres::PgPoolOptions::new()
                .connect_lazy("postgres://localhost/edgequake")
                .expect("lazy second pool"),
        ));
        registry
            .replace(Arc::clone(&second)
                as Arc<dyn edgequake_storage::contracts::CheckpointArtifactStore>);
        assert!(
            first_weak.upgrade().is_none(),
            "re-registering a second runtime must release the previous Arc"
        );
    }
}
