//! SPEC-120 P0 / A3: epoch-conditional side effects.

/// Epoch held by a writer before it begins expensive processing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FenceEpoch(pub i64);

#[derive(Debug, thiserror::Error)]
pub enum FenceError {
    #[error("fence epoch mismatch: expected {expected}, got {actual}")]
    Stale { expected: i64, actual: i64 },
    #[error("document not found for fence check: {0}")]
    NotFound(String),
    #[error("storage: {0}")]
    Storage(String),
}

fn record_stale_fence() {
    edgequake_observability::record_fence_rejected_write("stale_epoch");
}

/// Read `documents.fence_epoch` for `document_id`.
///
/// A runtime without a PostgreSQL pool uses epoch zero.
pub async fn read_fence_epoch(
    document_id: &str,
    #[cfg(feature = "postgres")] pool: crate::services::OptionalPgPool<'_>,
) -> Result<FenceEpoch, FenceError> {
    #[cfg(feature = "postgres")]
    if let Some(pool) = pool {
        let epoch =
            sqlx::query_scalar::<_, i64>("SELECT fence_epoch FROM public.documents WHERE id::text = $1")
                .bind(document_id)
                .fetch_optional(pool)
                .await
                .map_err(|error| FenceError::Storage(error.to_string()))?
                .ok_or_else(|| FenceError::NotFound(document_id.to_string()))?;

        return Ok(FenceEpoch(epoch));
    }

    let _ = document_id;
    Ok(FenceEpoch(0))
}

/// Atomically bump `fence_epoch` by one and return the new epoch.
///
/// Only call this when delete, wipe, or reprocess supersedes prior writers.
pub async fn bump_fence_epoch(
    document_id: &str,
    #[cfg(feature = "postgres")] pool: crate::services::OptionalPgPool<'_>,
) -> Result<FenceEpoch, FenceError> {
    #[cfg(feature = "postgres")]
    if let Some(pool) = pool {
        let epoch = sqlx::query_scalar::<_, i64>(
            "UPDATE public.documents
             SET fence_epoch = fence_epoch + 1, updated_at = NOW()
             WHERE id::text = $1
             RETURNING fence_epoch",
        )
        .bind(document_id)
        .fetch_optional(pool)
        .await
        .map_err(|error| FenceError::Storage(error.to_string()))?
        .ok_or_else(|| FenceError::NotFound(document_id.to_string()))?;

        return Ok(FenceEpoch(epoch));
    }

    let _ = document_id;
    Ok(FenceEpoch(0))
}

/// Atomically start a new document run and invalidate every prior writer.
///
/// The returned epoch is the run identity used by all subsequent stage writes.
/// `track_id` may initially be a provisional admission id and can be rebound
/// with [`bind_document_run_track`] once the durable task is created.
#[allow(clippy::too_many_arguments)]
pub async fn begin_document_run(
    document_id: &str,
    track_id: &str,
    stage: &str,
    stage_rank: u16,
    stage_message: &str,
    stage_progress: f64,
    #[cfg(feature = "postgres")] pool: crate::services::OptionalPgPool<'_>,
) -> Result<FenceEpoch, FenceError> {
    #[cfg(feature = "postgres")]
    if let Some(pool) = pool {
        let patch = serde_json::json!({
            "track_id": track_id,
            "current_stage": stage,
            "stage_rank": stage_rank,
            "stage_message": stage_message,
            "stage_progress": stage_progress.clamp(0.0, 1.0),
            "updated_at": chrono::Utc::now().to_rfc3339(),
        });
        let epoch = sqlx::query_scalar::<_, i64>(
            r#"
            UPDATE public.documents SET
                fence_epoch = fence_epoch + 1,
                track_id = $2,
                status = 'processing',
                metadata = COALESCE(metadata, '{}'::jsonb) || $3::jsonb,
                updated_at = NOW()
            WHERE id::text = $1
            RETURNING fence_epoch
            "#,
        )
        .bind(document_id)
        .bind(track_id)
        .bind(patch)
        .fetch_optional(pool)
        .await
        .map_err(|error| FenceError::Storage(error.to_string()))?
        .ok_or_else(|| FenceError::NotFound(document_id.to_string()))?;

        return Ok(FenceEpoch(epoch));
    }

    let _ = (
        document_id,
        track_id,
        stage,
        stage_rank,
        stage_message,
        stage_progress,
    );
    Ok(FenceEpoch(0))
}

/// Bind a provisional run to its durable task without changing the epoch.
pub async fn bind_document_run_track(
    document_id: &str,
    epoch: FenceEpoch,
    expected_track_id: &str,
    task_track_id: &str,
    #[cfg(feature = "postgres")] pool: crate::services::OptionalPgPool<'_>,
) -> Result<(), FenceError> {
    #[cfg(feature = "postgres")]
    if let Some(pool) = pool {
        let result = sqlx::query(
            r#"
            UPDATE public.documents SET
                track_id = $4,
                metadata = COALESCE(metadata, '{}'::jsonb)
                    || jsonb_build_object(
                        'track_id', $4::text,
                        'updated_at', NOW()::text
                    ),
                updated_at = NOW()
            WHERE id::text = $1
              AND fence_epoch = $2
              AND track_id = $3
            "#,
        )
        .bind(document_id)
        .bind(epoch.0)
        .bind(expected_track_id)
        .bind(task_track_id)
        .execute(pool)
        .await
        .map_err(|error| FenceError::Storage(error.to_string()))?;

        if result.rows_affected() == 1 {
            return Ok(());
        }
        record_stale_fence();
        let actual = read_fence_epoch(document_id, Some(pool)).await?;
        return Err(FenceError::Stale {
            expected: epoch.0,
            actual: actual.0,
        });
    }

    let _ = (document_id, epoch, expected_track_id, task_track_id);
    Ok(())
}

/// Assert that the current document epoch still matches the writer's held epoch.
pub async fn assert_fence(
    held: FenceEpoch,
    document_id: &str,
    #[cfg(feature = "postgres")] pool: crate::services::OptionalPgPool<'_>,
) -> Result<(), FenceError> {
    let actual = read_fence_epoch(
        document_id,
        #[cfg(feature = "postgres")]
        pool,
    )
    .await?;

    if actual == held {
        Ok(())
    } else {
        record_stale_fence();
        Err(FenceError::Stale {
            expected: held.0,
            actual: actual.0,
        })
    }
}
