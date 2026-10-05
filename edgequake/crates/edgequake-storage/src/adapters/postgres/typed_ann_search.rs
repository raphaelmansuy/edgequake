//! Shared bounded search session for typed chunk and fleet embedding adapters.

use sqlx::PgConnection;
use tokio::sync::OnceCell;

use super::ann_exact_reorder_policy::AnnExactReorderPolicy;
use super::capabilities::{extension_version_at_least, PGVECTOR_MIN_ITERATIVE_SCAN};
use super::config::VectorIndexType;
use super::statement_timeout::{vector_query_statement_timeout_ms, LocalTimeoutTx};
use super::vector::PgVectorStorage;
use crate::error::{Result, StorageError};

#[derive(Default)]
pub(crate) struct TypedAnnSearch {
    iterative_scan_supported: OnceCell<bool>,
}

impl TypedAnnSearch {
    pub(crate) fn candidate_limit(top_k: usize) -> usize {
        AnnExactReorderPolicy::for_search("relaxed_order", top_k)
            .effective_candidate_k(top_k)
            .max(top_k)
    }

    pub(crate) fn ordered_sql(sql: &str, top_k: u32) -> String {
        // Halfvec cosine index ordering can differ slightly from the projected
        // score. Materialize candidates and sort their stored-vector scores.
        format!(
            "WITH candidates AS MATERIALIZED ({sql})
             SELECT * FROM candidates ORDER BY score + 0 DESC LIMIT {top_k}"
        )
    }

    pub(crate) async fn begin<'c>(
        &self,
        conn: &'c mut PgConnection,
        top_k: usize,
        tenant: Option<uuid::Uuid>,
        workspace: Option<uuid::Uuid>,
    ) -> Result<LocalTimeoutTx<'c>> {
        let mut tx = LocalTimeoutTx::begin(conn, vector_query_statement_timeout_ms()).await?;
        super::rls::enforce_workspace_context(tx.as_mut(), tenant, workspace).await?;
        // Nullable scope/lineage binds have very different selectivity per workspace.
        // A cached generic plan can scan the broad workspace after learning a
        // selective one. Replan this bounded ANN statement with its actual binds.
        sqlx::query("SET LOCAL plan_cache_mode = force_custom_plan")
            .execute(tx.as_mut())
            .await
            .map_err(StorageError::from)?;
        // Probe on the held connection, inside the deadline. Acquiring another
        // connection here would deadlock single-connection or saturated pools.
        let supported = *self
            .iterative_scan_supported
            .get_or_try_init(|| async {
                let version: String = sqlx::query_scalar(
                    "SELECT extversion FROM pg_extension WHERE extname = 'vector'",
                )
                .fetch_one(tx.as_mut())
                .await
                .map_err(StorageError::from)?;
                Ok::<_, StorageError>(extension_version_at_least(
                    &version,
                    PGVECTOR_MIN_ITERATIVE_SCAN,
                ))
            })
            .await?;
        // Model/workspace/lineage filters can reject shared-index candidates.
        // The outer score reorder also protects against halfvec index precision.
        for statement in
            PgVectorStorage::search_tuning_statements(VectorIndexType::HNSW, top_k, true, supported)
        {
            sqlx::query(&statement)
                .execute(tx.as_mut())
                .await
                .map_err(StorageError::from)?;
        }
        Ok(tx)
    }
}
