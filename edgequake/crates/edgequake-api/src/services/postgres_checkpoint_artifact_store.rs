//! PostgreSQL adapter for checkpoint and document-artifact sidecars.

use async_trait::async_trait;
use edgequake_storage::contracts::{AccessError, AccessResult, CheckpointArtifactStore};
use serde_json::Value;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Clone)]
pub struct PostgresCheckpointArtifactStore {
    pool: PgPool,
}

impl PostgresCheckpointArtifactStore {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    async fn ensure_parent(&self, document_id: Uuid) -> AccessResult<()> {
        edgequake_storage::ensure_admission_document_row(&self.pool, document_id, None, None, "")
            .await
            .map_err(|error| AccessError::Unavailable(error.to_string()))
    }
}

#[async_trait]
impl CheckpointArtifactStore for PostgresCheckpointArtifactStore {
    async fn put_checkpoint(
        &self,
        document_id: Uuid,
        kind: &str,
        payload: &Value,
    ) -> AccessResult<()> {
        self.ensure_parent(document_id).await?;
        sqlx::query(
            "INSERT INTO public.pipeline_checkpoints (document_id, kind, payload) \
             VALUES ($1, $2, $3) \
             ON CONFLICT (document_id, kind) DO UPDATE SET \
                 payload = EXCLUDED.payload, updated_at = now()",
        )
        .bind(document_id)
        .bind(kind)
        .bind(payload)
        .execute(&self.pool)
        .await
        .map_err(database_error)?;
        Ok(())
    }

    async fn get_checkpoint(&self, document_id: Uuid, kind: &str) -> AccessResult<Option<Value>> {
        sqlx::query_scalar(
            "SELECT payload FROM public.pipeline_checkpoints \
             WHERE document_id = $1 AND kind = $2",
        )
        .bind(document_id)
        .bind(kind)
        .fetch_optional(&self.pool)
        .await
        .map_err(database_error)
    }

    async fn delete_checkpoint(&self, document_id: Uuid, kind: &str) -> AccessResult<()> {
        sqlx::query("DELETE FROM public.pipeline_checkpoints WHERE document_id = $1 AND kind = $2")
            .bind(document_id)
            .bind(kind)
            .execute(&self.pool)
            .await
            .map_err(database_error)?;
        Ok(())
    }

    async fn cleanup_stale_checkpoints(&self, max_age_secs: u64) -> AccessResult<u64> {
        let seconds = i64::try_from(max_age_secs)
            .map_err(|_| AccessError::InvalidInput("checkpoint age exceeds i64".into()))?;
        sqlx::query(
            "DELETE FROM public.pipeline_checkpoints \
             WHERE kind = 'checkpoint' AND updated_at < now() - make_interval(secs => $1)",
        )
        .bind(seconds)
        .execute(&self.pool)
        .await
        .map(|result| result.rows_affected())
        .map_err(database_error)
    }

    async fn put_artifact(
        &self,
        document_id: Uuid,
        kind: &str,
        payload: &Value,
    ) -> AccessResult<()> {
        self.ensure_parent(document_id).await?;
        sqlx::query(
            "INSERT INTO public.document_artifacts (document_id, kind, payload) \
             VALUES ($1, $2, $3) \
             ON CONFLICT (document_id, kind) DO UPDATE SET \
                 payload = EXCLUDED.payload, updated_at = now()",
        )
        .bind(document_id)
        .bind(kind)
        .bind(payload)
        .execute(&self.pool)
        .await
        .map_err(database_error)?;
        Ok(())
    }

    async fn get_artifact(&self, document_id: Uuid, kind: &str) -> AccessResult<Option<Value>> {
        sqlx::query_scalar(
            "SELECT payload FROM public.document_artifacts \
             WHERE document_id = $1 AND kind = $2",
        )
        .bind(document_id)
        .bind(kind)
        .fetch_optional(&self.pool)
        .await
        .map_err(database_error)
    }

    async fn delete_artifacts(&self, document_id: Uuid) -> AccessResult<()> {
        sqlx::query("DELETE FROM public.document_artifacts WHERE document_id = $1")
            .bind(document_id)
            .execute(&self.pool)
            .await
            .map_err(database_error)?;
        Ok(())
    }
}

fn database_error(error: sqlx::Error) -> AccessError {
    AccessError::Unavailable(format!("checkpoint/artifact store: {error}"))
}
