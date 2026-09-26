//! SPEC-149 PostgreSQL transactional ingestion committer.

use async_trait::async_trait;
use edgequake_storage_contracts::{
    checked_i64, decode_commit_receipt, last_included_cursor, physical_revision_id,
    require_active_roles, validate_prepared_ingestion_batch, AccessError, AccessResult,
    AccessScope, BindingRole, BindingState, CommitReceipt, CommittedRevision, CursorPage,
    DeleteDocument, DeleteReceipt, DocumentId, DocumentPageRequest, DocumentReader, DocumentView,
    IngestionCommitter, LifecycleCommitter, PreparedIngestionBatch, PreparedRecord,
    P0_REQUIRED_ROLES,
};
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use super::chunk_repository::{
    ensure_document_parent_in_transaction, insert_prepared_chunks_in_transaction,
};
use crate::projection_manifest::{
    logical_key_for_payload, role_completion_proof, EventManifestItem,
};

const COMMIT_OPERATION: &str = "ingestion.commit_batch";
const DELETE_OPERATION: &str = "lifecycle.tombstone_document";

/// PostgreSQL authority adapter for one bounded ingestion batch.
#[derive(Clone)]
pub struct PgIngestionCommitter {
    pool: PgPool,
    fail_before_event_append: bool,
}

impl PgIngestionCommitter {
    pub fn new(pool: PgPool) -> Self {
        Self {
            pool,
            fail_before_event_append: false,
        }
    }

    /// Deterministic fault injection for rollback contract tests.
    #[doc(hidden)]
    pub fn with_fault_before_event_append(mut self) -> Self {
        self.fail_before_event_append = true;
        self
    }

    async fn load_receipt(&self, command: &PreparedIngestionBatch) -> AccessResult<CommitReceipt> {
        let tenant_id = command.scope.tenant().into_uuid();
        let workspace_id = command.scope.workspace().into_uuid();
        let row = sqlx::query_as::<_, (Vec<u8>, Vec<u8>)>(
            r#"
            SELECT digest, receipt
            FROM public.mutation_requests
            WHERE tenant_id = $1
              AND workspace_id = $2
              AND operation = $3
              AND idempotency_key = $4
            "#,
        )
        .bind(tenant_id)
        .bind(workspace_id)
        .bind(COMMIT_OPERATION)
        .bind(&command.idempotency_key)
        .fetch_optional(&self.pool)
        .await
        .map_err(|error| classify_sqlx("read committed ingestion receipt", error))?
        .ok_or_else(|| {
            AccessError::UnknownOutcome(format!(
                "receipt {} was not visible after a concurrent commit",
                command.idempotency_key
            ))
        })?;
        decode_commit_receipt(command, &row.0, &row.1)
    }
}

#[async_trait]
impl IngestionCommitter for PgIngestionCommitter {
    async fn commit_batch(&self, command: &PreparedIngestionBatch) -> AccessResult<CommitReceipt> {
        let validated = validate_prepared_ingestion_batch(command)?;
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|error| classify_sqlx("begin ingestion transaction", error))?;

        if let Some(receipt) = find_receipt_in_transaction(&mut tx, command).await? {
            tx.rollback()
                .await
                .map_err(|error| classify_sqlx("close replay transaction", error))?;
            return Ok(receipt);
        }

        lock_scoped_document(&mut tx, command).await?;
        // A concurrent identical command may have been uncommitted during the
        // admission read and committed while this transaction waited for the
        // document lock. Recheck before expected-revision validation.
        if let Some(receipt) = find_receipt_in_transaction(&mut tx, command).await? {
            tx.rollback()
                .await
                .map_err(|error| classify_sqlx("close post-lock replay transaction", error))?;
            return Ok(receipt);
        }
        verify_expected_revision(&mut tx, command, validated.expected_revision).await?;
        insert_prepared_chunks_in_transaction(
            &mut tx,
            command.document_id.into_uuid(),
            validated.tenant_id,
            validated.workspace_id,
            validated.batch_ordinal,
            &command.chunks,
        )
        .await
        .map_err(AccessError::from)?;

        insert_object_revisions(&mut tx, command, "chunk", &command.chunks).await?;
        insert_object_revisions(&mut tx, command, "fact", &command.facts).await?;
        insert_object_revisions(&mut tx, command, "contribution", &command.contributions).await?;
        insert_contributions(&mut tx, command).await?;
        insert_embeddings(&mut tx, command).await?;
        insert_object_revisions(&mut tx, command, "embedding", &command.embeddings).await?;
        insert_ingest_batch(&mut tx, command, validated.expected_count).await?;
        advance_document_revision(&mut tx, command).await?;

        if self.fail_before_event_append {
            tx.rollback()
                .await
                .map_err(|error| classify_sqlx("rollback injected ingestion failure", error))?;
            return Err(AccessError::Unavailable(
                "injected failure before projection event append".into(),
            ));
        }

        let event_id = append_event_and_deliveries(&mut tx, command).await?;
        let receipt = build_receipt(command, event_id);
        let receipt_bytes = serde_json::to_vec(&receipt).map_err(|error| {
            AccessError::CorruptData(format!("serialize ingestion receipt: {error}"))
        })?;

        let inserted = sqlx::query(
            r#"
            INSERT INTO public.mutation_requests (
                tenant_id, workspace_id, operation, idempotency_key, digest, receipt
            )
            VALUES ($1, $2, $3, $4, $5, $6)
            ON CONFLICT (tenant_id, workspace_id, operation, idempotency_key)
                DO NOTHING
            "#,
        )
        .bind(validated.tenant_id)
        .bind(validated.workspace_id)
        .bind(COMMIT_OPERATION)
        .bind(&command.idempotency_key)
        .bind(command.canonical_digest.as_slice())
        .bind(&receipt_bytes)
        .execute(&mut *tx)
        .await
        .map_err(|error| classify_sqlx("insert ingestion mutation receipt", error))?
        .rows_affected();

        if inserted == 0 {
            tx.rollback()
                .await
                .map_err(|error| classify_sqlx("rollback duplicate ingestion", error))?;
            return self.load_receipt(command).await;
        }

        tx.commit().await.map_err(|error| {
            AccessError::UnknownOutcome(format!(
                "ingestion commit outcome is unknown for request {}: {error}",
                command.idempotency_key
            ))
        })?;
        Ok(receipt)
    }
}

#[async_trait]
impl LifecycleCommitter for PgIngestionCommitter {
    async fn tombstone_document(&self, command: &DeleteDocument) -> AccessResult<DeleteReceipt> {
        let tenant_id = command.scope.tenant().into_uuid();
        let workspace_id = command.scope.workspace().into_uuid();
        let document_id = command.document_id.into_uuid();
        let expected_revision = checked_i64(command.expected_revision, "expected revision")?;
        let tombstone_revision = command
            .expected_revision
            .checked_add(1)
            .ok_or_else(|| AccessError::InvalidInput("tombstone revision overflow".into()))?;
        let tombstone_revision_i64 = checked_i64(tombstone_revision, "tombstone revision")?;

        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|error| classify_sqlx("begin tombstone transaction", error))?;

        if let Some((digest, receipt)) = sqlx::query_as::<_, (Vec<u8>, Vec<u8>)>(
            "SELECT digest, receipt FROM public.mutation_requests \
             WHERE tenant_id = $1 AND workspace_id = $2 \
               AND operation = $3 AND idempotency_key = $4",
        )
        .bind(tenant_id)
        .bind(workspace_id)
        .bind(DELETE_OPERATION)
        .bind(&command.idempotency_key)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|error| classify_sqlx("read tombstone receipt", error))?
        {
            if digest != command.command_digest {
                return Err(AccessError::Conflict(
                    "tombstone idempotency key was reused with another digest".into(),
                ));
            }
            let receipt = serde_json::from_slice(&receipt).map_err(|error| {
                AccessError::CorruptData(format!("decode tombstone receipt: {error}"))
            })?;
            tx.rollback()
                .await
                .map_err(|error| classify_sqlx("close tombstone replay", error))?;
            return Ok(receipt);
        }

        let current = sqlx::query_as::<_, (i64, String)>(
            "SELECT revision, state FROM public.object_revisions \
             WHERE tenant_id = $1 AND workspace_id = $2 \
               AND kind = 'document' AND logical_id = $3 \
             ORDER BY revision DESC LIMIT 1 FOR UPDATE",
        )
        .bind(tenant_id)
        .bind(workspace_id)
        .bind(document_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|error| classify_sqlx("lock document revision", error))?;
        let (current_revision, current_state) = current.unwrap_or((0, "absent".into()));
        if let Some((digest, receipt)) = sqlx::query_as::<_, (Vec<u8>, Vec<u8>)>(
            "SELECT digest, receipt FROM public.mutation_requests \
             WHERE tenant_id = $1 AND workspace_id = $2 \
               AND operation = $3 AND idempotency_key = $4",
        )
        .bind(tenant_id)
        .bind(workspace_id)
        .bind(DELETE_OPERATION)
        .bind(&command.idempotency_key)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|error| classify_sqlx("reread tombstone receipt", error))?
        {
            if digest != command.command_digest {
                return Err(AccessError::Conflict(
                    "tombstone idempotency key was reused with another digest".into(),
                ));
            }
            let receipt = serde_json::from_slice(&receipt).map_err(|error| {
                AccessError::CorruptData(format!("decode tombstone receipt: {error}"))
            })?;
            tx.rollback()
                .await
                .map_err(|error| classify_sqlx("close tombstone replay", error))?;
            return Ok(receipt);
        }
        if current_state == "tombstoned" {
            if let Some(receipt) = sqlx::query_scalar::<_, Vec<u8>>(
                "SELECT receipt FROM public.mutation_requests \
                 WHERE tenant_id = $1 AND workspace_id = $2 AND operation = $3 \
                   AND receipt::jsonb->>'document_id' = $4 \
                 ORDER BY created_at DESC LIMIT 1",
            )
            .bind(tenant_id)
            .bind(workspace_id)
            .bind(DELETE_OPERATION)
            .bind(document_id.to_string())
            .fetch_optional(&mut *tx)
            .await
            .map_err(|error| classify_sqlx("load existing tombstone receipt", error))?
            {
                let receipt = serde_json::from_slice(&receipt).map_err(|error| {
                    AccessError::CorruptData(format!("decode tombstone receipt: {error}"))
                })?;
                tx.rollback()
                    .await
                    .map_err(|error| classify_sqlx("close tombstone resume", error))?;
                return Ok(receipt);
            }
        }
        if current_revision != expected_revision {
            return Err(AccessError::Conflict(format!(
                "document revision changed: expected {expected_revision}, current {current_revision}"
            )));
        }

        // Same as durable ingest: provision P0 graph/vector bindings before
        // locking targets. Legacy KV-only seeds (and fresh workspaces with no
        // prior commit) otherwise tombstone with zero bindings and the API
        // refuses physical cleanup (SPEC-098 / LAW-098-12 CI flake on clean DB).
        ensure_p0_bindings_in_transaction(&mut tx, tenant_id, workspace_id).await?;

        let cleanup_manifest_id = Uuid::new_v4();
        let event_id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO public.object_revisions (
                 tenant_id, workspace_id, kind, logical_id, revision, state,
                 physical_id, digest, payload_ref, payload
             ) VALUES ($1, $2, 'document', $3, $4, 'tombstoned',
                       $5, $6, $7, $8)",
        )
        .bind(tenant_id)
        .bind(workspace_id)
        .bind(document_id)
        .bind(tombstone_revision_i64)
        .bind(Uuid::new_v4())
        .bind(command.command_digest.as_slice())
        .bind(format!("cleanup://{cleanup_manifest_id}"))
        .bind(Vec::<u8>::new())
        .execute(&mut *tx)
        .await
        .map_err(|error| classify_sqlx("insert document tombstone", error))?;

        let target_binding_ids = sqlx::query_scalar::<_, Uuid>(
            "SELECT binding_id FROM public.data_bindings \
             WHERE tenant_id = $1 AND workspace_id = $2 \
               AND state IN ('active', 'draining') \
             ORDER BY binding_id FOR UPDATE",
        )
        .bind(tenant_id)
        .bind(workspace_id)
        .fetch_all(&mut *tx)
        .await
        .map_err(|error| classify_sqlx("lock deletion target bindings", error))?;

        sqlx::query(
            "INSERT INTO public.projection_events (
                 event_id, tenant_id, workspace_id, object_kind, object_id,
                 object_revision, schema_version, operation, manifest_ref, digest
             ) VALUES ($1, $2, $3, 'document', $4, $5, 1, 'delete', $6, $7)",
        )
        .bind(event_id)
        .bind(tenant_id)
        .bind(workspace_id)
        .bind(document_id)
        .bind(tombstone_revision_i64)
        .bind(format!("cleanup://{cleanup_manifest_id}"))
        .bind(command.command_digest.as_slice())
        .execute(&mut *tx)
        .await
        .map_err(|error| classify_sqlx("append tombstone event", error))?;

        insert_tombstone_manifest(&mut tx, event_id, tenant_id, workspace_id, document_id).await?;

        for binding_id in &target_binding_ids {
            sqlx::query(
                "INSERT INTO public.projection_cleanup_intents (
                     cleanup_manifest_id, binding_id, tenant_id, workspace_id,
                     document_id, tombstone_revision
                 ) VALUES ($1, $2, $3, $4, $5, $6)",
            )
            .bind(cleanup_manifest_id)
            .bind(binding_id)
            .bind(tenant_id)
            .bind(workspace_id)
            .bind(document_id)
            .bind(tombstone_revision_i64)
            .execute(&mut *tx)
            .await
            .map_err(|error| classify_sqlx("record cleanup intent", error))?;
            sqlx::query(
                "INSERT INTO public.projection_deliveries (event_id, binding_id, state) \
                 VALUES ($1, $2, 'pending')",
            )
            .bind(event_id)
            .bind(binding_id)
            .execute(&mut *tx)
            .await
            .map_err(|error| classify_sqlx("append delete delivery", error))?;
        }

        let receipt = DeleteReceipt {
            scope: command.scope,
            document_id: command.document_id,
            tombstone_revision,
            cleanup_manifest_id,
            target_binding_ids,
        };
        let receipt_bytes = serde_json::to_vec(&receipt).map_err(|error| {
            AccessError::CorruptData(format!("encode tombstone receipt: {error}"))
        })?;
        sqlx::query(
            "INSERT INTO public.mutation_requests (
                 tenant_id, workspace_id, operation, idempotency_key, digest, receipt
             ) VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(tenant_id)
        .bind(workspace_id)
        .bind(DELETE_OPERATION)
        .bind(&command.idempotency_key)
        .bind(command.command_digest.as_slice())
        .bind(receipt_bytes)
        .execute(&mut *tx)
        .await
        .map_err(|error| classify_sqlx("persist tombstone receipt", error))?;

        tx.commit()
            .await
            .map_err(|error| classify_sqlx("commit tombstone transaction", error))?;
        Ok(receipt)
    }
}

#[async_trait]
impl DocumentReader for PgIngestionCommitter {
    async fn get_many(
        &self,
        scope: &AccessScope,
        ids: &[DocumentId],
    ) -> AccessResult<Vec<Option<DocumentView>>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        if ids.len() > 100 {
            return Err(AccessError::InvalidInput(
                "document get_many accepts at most 100 ids".into(),
            ));
        }
        let wanted: Vec<Uuid> = ids.iter().map(|id| id.into_uuid()).collect();
        let rows = sqlx::query_as::<_, (Uuid, i64, String, Vec<u8>)>(
            r#"
            SELECT DISTINCT ON (logical_id) logical_id, revision, state, digest
            FROM public.object_revisions
            WHERE tenant_id = $1
              AND workspace_id = $2
              AND kind = 'document'
              AND logical_id = ANY($3)
            ORDER BY logical_id, revision DESC
            "#,
        )
        .bind(scope.tenant().into_uuid())
        .bind(scope.workspace().into_uuid())
        .bind(&wanted)
        .fetch_all(&self.pool)
        .await
        .map_err(|error| classify_sqlx("read document revisions", error))?;
        let mut by_id = std::collections::HashMap::new();
        for (logical_id, revision, state, digest) in rows {
            by_id.insert(
                logical_id,
                document_view(scope, logical_id, revision, &state, digest)?,
            );
        }
        Ok(ids
            .iter()
            .map(|id| by_id.get(&id.into_uuid()).cloned())
            .collect())
    }

    async fn list(
        &self,
        scope: &AccessScope,
        request: &DocumentPageRequest,
    ) -> AccessResult<CursorPage<DocumentView>> {
        let limit = i64::from(request.limit.clamp(1, 100));
        let cursor = match request.cursor.as_deref() {
            None => None,
            Some(raw) => Some(Uuid::parse_str(raw).map_err(|_| {
                AccessError::InvalidInput("document page cursor is not a UUID".into())
            })?),
        };
        let rows = sqlx::query_as::<_, (Uuid, i64, String, Vec<u8>)>(
            r#"
            SELECT logical_id, revision, state, digest
            FROM public.object_revisions r
            WHERE tenant_id = $1
              AND workspace_id = $2
              AND kind = 'document'
              AND ($3::uuid IS NULL OR logical_id > $3)
              AND revision = (
                  SELECT MAX(r2.revision)
                  FROM public.object_revisions r2
                  WHERE r2.tenant_id = r.tenant_id
                    AND r2.workspace_id = r.workspace_id
                    AND r2.kind = 'document'
                    AND r2.logical_id = r.logical_id
              )
            ORDER BY logical_id ASC
            LIMIT $4
            "#,
        )
        .bind(scope.tenant().into_uuid())
        .bind(scope.workspace().into_uuid())
        .bind(cursor)
        .bind(limit + 1)
        .fetch_all(&self.pool)
        .await
        .map_err(|error| classify_sqlx("list document revisions", error))?;

        let ids: Vec<String> = rows
            .iter()
            .map(|(logical_id, _, _, _)| logical_id.to_string())
            .collect();
        let next_cursor = last_included_cursor(limit as usize, &ids);
        let mut items = Vec::new();
        for (logical_id, revision, state, digest) in rows.into_iter().take(limit as usize) {
            items.push(document_view(scope, logical_id, revision, &state, digest)?);
        }
        Ok(CursorPage { items, next_cursor })
    }
}

async fn find_receipt_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    command: &PreparedIngestionBatch,
) -> AccessResult<Option<CommitReceipt>> {
    let row = sqlx::query_as::<_, (Vec<u8>, Vec<u8>)>(
        r#"
        SELECT digest, receipt
        FROM public.mutation_requests
        WHERE tenant_id = $1
          AND workspace_id = $2
          AND operation = $3
          AND idempotency_key = $4
        "#,
    )
    .bind(command.scope.tenant().into_uuid())
    .bind(command.scope.workspace().into_uuid())
    .bind(COMMIT_OPERATION)
    .bind(&command.idempotency_key)
    .fetch_optional(&mut **tx)
    .await
    .map_err(|error| classify_sqlx("read ingestion replay receipt", error))?;
    row.map(|(digest, receipt)| decode_commit_receipt(command, &digest, &receipt))
        .transpose()
}

async fn lock_scoped_document(
    tx: &mut Transaction<'_, Postgres>,
    command: &PreparedIngestionBatch,
) -> AccessResult<()> {
    let tenant_id = command.scope.tenant().into_uuid();
    let workspace_id = command.scope.workspace().into_uuid();
    let document_id = command.document_id.into_uuid();
    ensure_document_parent_in_transaction(tx, document_id, Some(tenant_id), Some(workspace_id))
        .await
        .map_err(AccessError::from)?;

    let scope = sqlx::query_as::<_, (Option<Uuid>, Option<Uuid>)>(
        "SELECT tenant_id, workspace_id FROM public.documents WHERE id = $1 FOR UPDATE",
    )
    .bind(document_id)
    .fetch_one(&mut **tx)
    .await
    .map_err(|error| classify_sqlx("lock ingestion document", error))?;
    if scope != (Some(tenant_id), Some(workspace_id)) {
        return Err(AccessError::ForbiddenScope(format!(
            "document {document_id} belongs to another tenant or workspace"
        )));
    }

    Ok(())
}

async fn verify_expected_revision(
    tx: &mut Transaction<'_, Postgres>,
    command: &PreparedIngestionBatch,
    expected_revision: Option<i64>,
) -> AccessResult<()> {
    let Some(expected) = expected_revision else {
        return Ok(());
    };
    let current = sqlx::query_scalar::<_, Option<i64>>(
        r#"
        SELECT max(revision)
        FROM public.object_revisions
        WHERE tenant_id = $1 AND workspace_id = $2
          AND kind = 'document' AND logical_id = $3
        "#,
    )
    .bind(command.scope.tenant().into_uuid())
    .bind(command.scope.workspace().into_uuid())
    .bind(command.document_id.into_uuid())
    .fetch_one(&mut **tx)
    .await
    .map_err(|error| classify_sqlx("read current document generation", error))?
    .unwrap_or(0);
    if current != expected {
        return Err(AccessError::Conflict(format!(
            "document {} expected revision {expected}, current revision is {current}",
            command.document_id
        )));
    }
    Ok(())
}

async fn insert_object_revisions(
    tx: &mut Transaction<'_, Postgres>,
    command: &PreparedIngestionBatch,
    kind: &str,
    records: &[PreparedRecord],
) -> AccessResult<()> {
    let tenant_id = command.scope.tenant().into_uuid();
    let workspace_id = command.scope.workspace().into_uuid();
    for record in records {
        let revision = checked_i64(record.revision, "record revision")?;
        let physical_id = physical_revision_id(command, kind, record);
        sqlx::query(
            r#"
            INSERT INTO public.object_revisions (
                tenant_id, workspace_id, kind, logical_id, revision, state,
                physical_id, digest, payload_ref, payload
            )
            VALUES ($1, $2, $3, $4, $5, 'staged', $6, $7, NULL, $8)
            ON CONFLICT (tenant_id, workspace_id, kind, logical_id, revision)
                DO NOTHING
            "#,
        )
        .bind(tenant_id)
        .bind(workspace_id)
        .bind(kind)
        .bind(record.id)
        .bind(revision)
        .bind(physical_id)
        .bind(record.digest.as_slice())
        .bind(&record.payload)
        .execute(&mut **tx)
        .await
        .map_err(|error| classify_sqlx("insert immutable object revision", error))?;

        let actual = sqlx::query_as::<_, (Uuid, Vec<u8>, Vec<u8>)>(
            r#"
            SELECT physical_id, digest, payload
            FROM public.object_revisions
            WHERE tenant_id = $1 AND workspace_id = $2 AND kind = $3
              AND logical_id = $4 AND revision = $5
            "#,
        )
        .bind(tenant_id)
        .bind(workspace_id)
        .bind(kind)
        .bind(record.id)
        .bind(revision)
        .fetch_one(&mut **tx)
        .await
        .map_err(|error| classify_sqlx("verify immutable object revision", error))?;
        if actual.0 != physical_id || actual.1 != record.digest || actual.2 != record.payload {
            return Err(AccessError::Conflict(format!(
                "{kind} {} revision {} conflicts with persisted content",
                record.id, record.revision
            )));
        }
    }
    Ok(())
}

async fn insert_contributions(
    tx: &mut Transaction<'_, Postgres>,
    command: &PreparedIngestionBatch,
) -> AccessResult<()> {
    let tenant_id = command.scope.tenant().into_uuid();
    let workspace_id = command.scope.workspace().into_uuid();
    let source_document_id = command.document_id.into_uuid();
    let source_generation = checked_i64(command.ingest_generation, "ingest generation")?;
    for record in &command.contributions {
        let revision = checked_i64(record.revision, "contribution revision")?;
        let payload = serde_json::from_slice::<serde_json::Value>(&record.payload)
            .unwrap_or_else(|_| serde_json::json!({ "bytes": record.payload }));
        let fact_id = payload
            .get("fact_id")
            .and_then(|value| value.as_str())
            .and_then(|raw| Uuid::parse_str(raw).ok())
            .unwrap_or(record.id);
        let fact_revision = payload
            .get("fact_revision")
            .and_then(|value| value.as_u64())
            .map(|value| checked_i64(value, "fact revision"))
            .transpose()?
            .unwrap_or(revision);
        sqlx::query(
            r#"
            INSERT INTO public.graph_contributions (
                tenant_id, workspace_id, fact_id, fact_revision, contribution_id,
                source_document_id, source_generation, source_chunk_id,
                payload_digest, payload
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, NULL, $8, $9)
            ON CONFLICT (contribution_id) DO NOTHING
            "#,
        )
        .bind(tenant_id)
        .bind(workspace_id)
        .bind(fact_id)
        .bind(fact_revision)
        .bind(record.id)
        .bind(source_document_id)
        .bind(source_generation)
        .bind(record.digest.as_slice())
        .bind(&payload)
        .execute(&mut **tx)
        .await
        .map_err(|error| classify_sqlx("insert graph contribution", error))?;

        let actual = sqlx::query_as::<_, (Uuid, Uuid, Uuid, i64, i64, Vec<u8>)>(
            r#"
            SELECT tenant_id, workspace_id, fact_id, fact_revision, source_generation, payload_digest
            FROM public.graph_contributions
            WHERE contribution_id = $1
            "#,
        )
        .bind(record.id)
        .fetch_one(&mut **tx)
        .await
        .map_err(|error| classify_sqlx("verify graph contribution", error))?;
        if actual
            != (
                tenant_id,
                workspace_id,
                fact_id,
                fact_revision,
                source_generation,
                record.digest.to_vec(),
            )
        {
            return Err(AccessError::Conflict(format!(
                "contribution {} conflicts with persisted content",
                record.id
            )));
        }
    }
    Ok(())
}

async fn insert_embeddings(
    tx: &mut Transaction<'_, Postgres>,
    command: &PreparedIngestionBatch,
) -> AccessResult<()> {
    let tenant_id = command.scope.tenant().into_uuid();
    let workspace_id = command.scope.workspace().into_uuid();
    let content_revision = checked_i64(command.ingest_generation, "ingest generation")?;
    for record in &command.embeddings {
        let physical_id = physical_revision_id(command, "embedding", record);
        sqlx::query(
            r#"
            INSERT INTO public.embedding_manifests (
                tenant_id, workspace_id, subject_id, family, model_revision,
                content_revision, physical_id, dimension, metric, digest,
                payload_ref, payload
            )
            VALUES ($1, $2, $3, 'default', 'unspecified', $4, $5, 0,
                    'unspecified', $6, NULL, $7)
            ON CONFLICT (
                tenant_id, workspace_id, subject_id, family,
                model_revision, content_revision
            ) DO NOTHING
            "#,
        )
        .bind(tenant_id)
        .bind(workspace_id)
        .bind(record.id)
        .bind(content_revision)
        .bind(physical_id)
        .bind(record.digest.as_slice())
        .bind(&record.payload)
        .execute(&mut **tx)
        .await
        .map_err(|error| classify_sqlx("insert embedding manifest", error))?;

        let actual = sqlx::query_as::<_, (Uuid, Vec<u8>, Option<Vec<u8>>)>(
            r#"
            SELECT physical_id, digest, payload
            FROM public.embedding_manifests
            WHERE tenant_id = $1 AND workspace_id = $2 AND subject_id = $3
              AND family = 'default' AND model_revision = 'unspecified'
              AND content_revision = $4
            "#,
        )
        .bind(tenant_id)
        .bind(workspace_id)
        .bind(record.id)
        .bind(content_revision)
        .fetch_one(&mut **tx)
        .await
        .map_err(|error| classify_sqlx("verify embedding manifest", error))?;
        if actual
            != (
                physical_id,
                record.digest.to_vec(),
                Some(record.payload.clone()),
            )
        {
            return Err(AccessError::Conflict(format!(
                "embedding {} generation {} conflicts with persisted content",
                record.id, command.ingest_generation
            )));
        }
    }
    Ok(())
}

async fn insert_ingest_batch(
    tx: &mut Transaction<'_, Postgres>,
    command: &PreparedIngestionBatch,
    expected_count: i32,
) -> AccessResult<()> {
    let generation = checked_i64(command.ingest_generation, "ingest generation")?;
    let ordinal = i32::try_from(command.batch_ordinal)
        .map_err(|_| AccessError::InvalidInput("batch ordinal exceeds i32".into()))?;
    sqlx::query(
        r#"
        INSERT INTO public.ingest_batches (
            tenant_id, workspace_id, document_id, generation, batch_ordinal,
            digest, expected_count, state
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, 'staged')
        ON CONFLICT (
            tenant_id, workspace_id, document_id, generation, batch_ordinal
        ) DO NOTHING
        "#,
    )
    .bind(command.scope.tenant().into_uuid())
    .bind(command.scope.workspace().into_uuid())
    .bind(command.document_id.into_uuid())
    .bind(generation)
    .bind(ordinal)
    .bind(command.canonical_digest.as_slice())
    .bind(expected_count)
    .execute(&mut **tx)
    .await
    .map_err(|error| classify_sqlx("insert ingest batch manifest", error))?;

    let actual = sqlx::query_as::<_, (Vec<u8>, i32)>(
        r#"
        SELECT digest, expected_count
        FROM public.ingest_batches
        WHERE tenant_id = $1 AND workspace_id = $2 AND document_id = $3
          AND generation = $4 AND batch_ordinal = $5
        "#,
    )
    .bind(command.scope.tenant().into_uuid())
    .bind(command.scope.workspace().into_uuid())
    .bind(command.document_id.into_uuid())
    .bind(generation)
    .bind(ordinal)
    .fetch_one(&mut **tx)
    .await
    .map_err(|error| classify_sqlx("verify ingest batch manifest", error))?;
    if actual != (command.canonical_digest.to_vec(), expected_count) {
        return Err(AccessError::Conflict(format!(
            "document {} generation {} batch {} conflicts with persisted manifest",
            command.document_id, command.ingest_generation, command.batch_ordinal
        )));
    }
    Ok(())
}

async fn append_event_and_deliveries(
    tx: &mut Transaction<'_, Postgres>,
    command: &PreparedIngestionBatch,
) -> AccessResult<Uuid> {
    let tenant_id = command.scope.tenant().into_uuid();
    let workspace_id = command.scope.workspace().into_uuid();
    let generation = checked_i64(command.ingest_generation, "ingest generation")?;
    let schema_version = i32::try_from(command.schema_version)
        .map_err(|_| AccessError::InvalidInput("schema version exceeds i32".into()))?;
    let operation = format!("ingest_batch:{}", command.batch_ordinal);
    let manifest_ref = format!(
        "ingest-batch://{}/{}/{}",
        command.document_id, command.ingest_generation, command.batch_ordinal
    );
    let proposed_event_id = Uuid::new_v4();

    ensure_p0_bindings_in_transaction(tx, tenant_id, workspace_id).await?;

    let inserted = sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO public.projection_events (
            event_id, tenant_id, workspace_id, object_kind, object_id,
            object_revision, schema_version, operation, manifest_ref, digest
        )
        VALUES ($1, $2, $3, 'document_batch', $4, $5, $6, $7, $8, $9)
        ON CONFLICT (
            tenant_id, workspace_id, object_kind, object_id,
            object_revision, operation, schema_version
        ) DO NOTHING
        RETURNING event_id
        "#,
    )
    .bind(proposed_event_id)
    .bind(tenant_id)
    .bind(workspace_id)
    .bind(command.document_id.into_uuid())
    .bind(generation)
    .bind(schema_version)
    .bind(&operation)
    .bind(&manifest_ref)
    .bind(command.canonical_digest.as_slice())
    .fetch_optional(&mut **tx)
    .await
    .map_err(|error| classify_sqlx("append projection event", error))?;

    let event_id = if let Some(event_id) = inserted {
        event_id
    } else {
        let (event_id, digest) = sqlx::query_as::<_, (Uuid, Vec<u8>)>(
            r#"
            SELECT event_id, digest
            FROM public.projection_events
            WHERE tenant_id = $1 AND workspace_id = $2
              AND object_kind = 'document_batch' AND object_id = $3
              AND object_revision = $4 AND operation = $5 AND schema_version = $6
            "#,
        )
        .bind(tenant_id)
        .bind(workspace_id)
        .bind(command.document_id.into_uuid())
        .bind(generation)
        .bind(&operation)
        .bind(schema_version)
        .fetch_one(&mut **tx)
        .await
        .map_err(|error| classify_sqlx("resolve projection event replay", error))?;
        if digest != command.canonical_digest {
            return Err(AccessError::Conflict(format!(
                "projection event for document {} generation {} batch {} has another digest",
                command.document_id, command.ingest_generation, command.batch_ordinal
            )));
        }
        event_id
    };

    let inserted_deliveries = sqlx::query(
        r#"
        INSERT INTO public.projection_deliveries (event_id, binding_id, state)
        SELECT $1, binding_id, 'pending'
        FROM public.data_bindings
        WHERE tenant_id = $2 AND workspace_id = $3 AND state IN ('active', 'draining')
          AND role = ANY($4::text[])
        ON CONFLICT (event_id, binding_id) DO NOTHING
        "#,
    )
    .bind(event_id)
    .bind(tenant_id)
    .bind(workspace_id)
    .bind(
        P0_REQUIRED_ROLES
            .iter()
            .map(|role| role.as_str())
            .collect::<Vec<_>>(),
    )
    .execute(&mut **tx)
    .await
    .map_err(|error| classify_sqlx("append projection deliveries", error))?
    .rows_affected();

    let delivery_count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM public.projection_deliveries WHERE event_id = $1")
            .bind(event_id)
            .fetch_one(&mut **tx)
            .await
            .map_err(|error| classify_sqlx("count projection deliveries", error))?;

    if delivery_count == 0 {
        return Err(AccessError::Unavailable(
            "durable ingest refused: scope has zero active graph/vector bindings".into(),
        ));
    }
    if delivery_count < P0_REQUIRED_ROLES.len() as i64 {
        return Err(AccessError::Unavailable(format!(
            "durable ingest refused: expected {} active P0 bindings, found {delivery_count} \
             (inserted_new={inserted_deliveries})",
            P0_REQUIRED_ROLES.len()
        )));
    }
    let graph_items = manifest_items("graph", "graph_contribution", &command.contributions);
    let vector_items = manifest_items("vector", "embedding", &command.embeddings);
    write_event_manifest(tx, event_id, &graph_items, &vector_items).await?;
    Ok(event_id)
}

async fn ensure_p0_bindings_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: Uuid,
    workspace_id: Uuid,
) -> AccessResult<()> {
    use super::binding_registry::PgBindingRegistry;
    use edgequake_storage_contracts::{AccessScope, TenantId, WorkspaceId};

    let scope = AccessScope::new(TenantId::new(tenant_id), WorkspaceId::new(workspace_id));
    for role in P0_REQUIRED_ROLES {
        let descriptor = PgBindingRegistry::descriptor_for_role(scope, *role);
        // Inline upsert keeps provisioning inside the authority transaction.
        sqlx::query(
            r#"
            INSERT INTO public.data_bindings (
                binding_id, tenant_id, workspace_id, role, provider, config_ref,
                layout, physical_index, model_descriptor, generation, state
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            ON CONFLICT (binding_id) DO NOTHING
            "#,
        )
        .bind(descriptor.binding_id)
        .bind(tenant_id)
        .bind(workspace_id)
        .bind(descriptor.role.as_str())
        .bind(&descriptor.provider)
        .bind(&descriptor.config_ref)
        .bind(&descriptor.layout)
        .bind(&descriptor.physical_index)
        .bind(descriptor.model_descriptor.as_deref())
        .bind(
            i64::try_from(descriptor.generation)
                .map_err(|_| AccessError::InvalidInput("binding generation exceeds i64".into()))?,
        )
        .bind(BindingState::Active.as_str())
        .execute(&mut **tx)
        .await
        .map_err(|error| classify_sqlx("ensure P0 data binding", error))?;
    }

    let active: Vec<(Uuid, String, String)> = sqlx::query_as(
        r#"
        SELECT binding_id, role, state
        FROM public.data_bindings
        WHERE tenant_id = $1 AND workspace_id = $2 AND state IN ('active', 'draining')
          AND role = ANY($3::text[])
        "#,
    )
    .bind(tenant_id)
    .bind(workspace_id)
    .bind(
        P0_REQUIRED_ROLES
            .iter()
            .map(|role| role.as_str())
            .collect::<Vec<_>>(),
    )
    .fetch_all(&mut **tx)
    .await
    .map_err(|error| classify_sqlx("verify P0 data bindings", error))?;

    let mut descriptors = Vec::new();
    for (binding_id, role, _state) in active {
        descriptors.push(edgequake_storage_contracts::DataBindingDescriptor {
            binding_id,
            scope,
            role: BindingRole::parse(&role)?,
            provider: String::new(),
            config_ref: String::new(),
            layout: String::new(),
            physical_index: String::new(),
            model_descriptor: None,
            generation: 1,
            state: BindingState::Active,
        });
    }
    // Draining bindings still receive deliveries; treat them as present for admission.
    require_active_roles(&descriptors, P0_REQUIRED_ROLES)?;
    Ok(())
}

async fn advance_document_revision(
    tx: &mut Transaction<'_, Postgres>,
    command: &PreparedIngestionBatch,
) -> AccessResult<()> {
    let tenant_id = command.scope.tenant().into_uuid();
    let workspace_id = command.scope.workspace().into_uuid();
    let document_id = command.document_id.into_uuid();
    let generation = checked_i64(command.ingest_generation, "ingest generation")?;
    let physical_id = Uuid::new_v5(
        &Uuid::NAMESPACE_OID,
        format!("document:{document_id}:{generation}").as_bytes(),
    );
    sqlx::query(
        r#"
        INSERT INTO public.object_revisions (
            tenant_id, workspace_id, kind, logical_id, revision, state,
            physical_id, digest, payload_ref, payload
        )
        VALUES ($1, $2, 'document', $3, $4, 'active', $5, $6, $7, $8)
        ON CONFLICT (tenant_id, workspace_id, kind, logical_id, revision)
            DO NOTHING
        "#,
    )
    .bind(tenant_id)
    .bind(workspace_id)
    .bind(document_id)
    .bind(generation)
    .bind(physical_id)
    .bind(command.canonical_digest.as_slice())
    .bind(format!(
        "ingest-batch://{}/{}/{}",
        command.document_id, command.ingest_generation, command.batch_ordinal
    ))
    .bind(command.canonical_digest.as_slice())
    .execute(&mut **tx)
    .await
    .map_err(|error| classify_sqlx("advance document revision", error))?;
    Ok(())
}

fn build_receipt(command: &PreparedIngestionBatch, manifest_id: Uuid) -> CommitReceipt {
    let committed = command
        .chunks
        .iter()
        .chain(&command.facts)
        .chain(&command.contributions)
        .chain(&command.embeddings)
        .map(|record| CommittedRevision {
            id: record.id,
            revision: record.revision,
        })
        .collect();
    CommitReceipt {
        request_key: command.idempotency_key.clone(),
        command_digest: command.canonical_digest,
        document_generation: command.ingest_generation,
        committed,
        manifest_id,
        durable_commit_token: format!("postgres:{manifest_id}"),
    }
}

fn document_view(
    scope: &AccessScope,
    logical_id: Uuid,
    revision: i64,
    state: &str,
    digest: Vec<u8>,
) -> AccessResult<DocumentView> {
    let digest: [u8; 32] = digest
        .try_into()
        .map_err(|_| AccessError::CorruptData("document digest is not 32 bytes".into()))?;
    Ok(DocumentView {
        scope: *scope,
        document_id: DocumentId::new(logical_id),
        revision: u64::try_from(revision.max(0)).unwrap_or(0),
        digest,
        deleted: state == "tombstoned",
    })
}

fn manifest_items(
    role: &str,
    item_kind: &str,
    records: &[PreparedRecord],
) -> Vec<EventManifestItem> {
    records
        .iter()
        .map(|record| EventManifestItem {
            role: role.to_string(),
            item_kind: item_kind.to_string(),
            record_id: record.id,
            record_revision: i64::try_from(record.revision).unwrap_or(1),
            digest: record.digest,
            logical_key: logical_key_for_payload(&record.payload, record.id),
        })
        .collect()
}

async fn write_event_manifest(
    tx: &mut Transaction<'_, Postgres>,
    event_id: Uuid,
    graph_items: &[EventManifestItem],
    vector_items: &[EventManifestItem],
) -> AccessResult<()> {
    for (role, items) in [("graph", graph_items), ("vector", vector_items)] {
        for (ordinal, item) in items.iter().enumerate() {
            let ordinal = i32::try_from(ordinal)
                .map_err(|_| AccessError::InvalidInput("manifest ordinal exceeds i32".into()))?;
            sqlx::query(
                "INSERT INTO public.projection_event_items (
                     event_id, role, ordinal, item_kind, record_id,
                     record_revision, digest, logical_key
                 ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
                 ON CONFLICT (event_id, role, ordinal) DO NOTHING",
            )
            .bind(event_id)
            .bind(role)
            .bind(ordinal)
            .bind(&item.item_kind)
            .bind(item.record_id)
            .bind(item.record_revision)
            .bind(item.digest.as_slice())
            .bind(&item.logical_key)
            .execute(&mut **tx)
            .await
            .map_err(|error| classify_sqlx("insert projection manifest item", error))?;
        }
        let proof = role_completion_proof(items);
        let inserted = sqlx::query_scalar::<_, Vec<u8>>(
            "INSERT INTO public.projection_event_role_proofs (event_id, role, expected_digest)
             VALUES ($1, $2, $3)
             ON CONFLICT (event_id, role) DO NOTHING
             RETURNING expected_digest",
        )
        .bind(event_id)
        .bind(role)
        .bind(proof.as_slice())
        .fetch_optional(&mut **tx)
        .await
        .map_err(|error| classify_sqlx("insert projection role proof", error))?;
        let stored = if let Some(stored) = inserted {
            stored
        } else {
            sqlx::query_scalar(
                "SELECT expected_digest FROM public.projection_event_role_proofs \
                 WHERE event_id = $1 AND role = $2",
            )
            .bind(event_id)
            .bind(role)
            .fetch_one(&mut **tx)
            .await
            .map_err(|error| classify_sqlx("read projection role proof", error))?
        };
        if stored.as_slice() != proof {
            return Err(AccessError::Conflict(format!(
                "projection role proof for {role} does not match the event manifest"
            )));
        }
    }
    Ok(())
}

async fn insert_tombstone_manifest(
    tx: &mut Transaction<'_, Postgres>,
    event_id: Uuid,
    tenant_id: Uuid,
    workspace_id: Uuid,
    document_id: Uuid,
) -> AccessResult<()> {
    let contributions = sqlx::query_as::<_, (Uuid, i64, Vec<u8>, serde_json::Value)>(
        "SELECT contribution_id, source_generation, payload_digest, payload \
         FROM public.graph_contributions \
         WHERE tenant_id = $1 AND workspace_id = $2 AND source_document_id = $3 \
         ORDER BY contribution_id",
    )
    .bind(tenant_id)
    .bind(workspace_id)
    .bind(document_id)
    .fetch_all(&mut **tx)
    .await
    .map_err(|error| classify_sqlx("load tombstone graph membership", error))?;
    let graph_items = contributions
        .into_iter()
        .map(|(id, revision, digest, payload)| {
            let digest: [u8; 32] = digest.try_into().map_err(|_| {
                AccessError::CorruptData("contribution digest is not 32 bytes".into())
            })?;
            Ok(EventManifestItem {
                role: "graph".into(),
                item_kind: "graph_contribution".into(),
                record_id: id,
                record_revision: revision.max(1),
                digest,
                logical_key: logical_key_for_payload(
                    &serde_json::to_vec(&payload).unwrap_or_default(),
                    id,
                ),
            })
        })
        .collect::<AccessResult<Vec<_>>>()?;
    let embeddings = sqlx::query_as::<_, (Uuid, i64, Vec<u8>)>(
        "SELECT m.subject_id, m.content_revision, m.digest \
         FROM public.embedding_manifests m \
         JOIN public.chunks c ON c.id = m.subject_id \
         WHERE m.tenant_id = $1 AND m.workspace_id = $2 AND c.document_id = $3 \
         ORDER BY m.subject_id, m.content_revision",
    )
    .bind(tenant_id)
    .bind(workspace_id)
    .bind(document_id)
    .fetch_all(&mut **tx)
    .await
    .map_err(|error| classify_sqlx("load tombstone vector membership", error))?;
    let vector_items = embeddings
        .into_iter()
        .map(|(id, revision, digest)| {
            let digest: [u8; 32] = digest
                .try_into()
                .map_err(|_| AccessError::CorruptData("embedding digest is not 32 bytes".into()))?;
            Ok(EventManifestItem {
                role: "vector".into(),
                item_kind: "embedding".into(),
                record_id: id,
                record_revision: revision.max(1),
                digest,
                logical_key: id.to_string(),
            })
        })
        .collect::<AccessResult<Vec<_>>>()?;
    write_event_manifest(tx, event_id, &graph_items, &vector_items).await
}

fn classify_sqlx(context: &str, error: sqlx::Error) -> AccessError {
    if let sqlx::Error::Database(database) = &error {
        match database.code().as_deref() {
            Some("40001" | "40P01") => {
                return AccessError::SerializationRetry(format!("{context}: {error}"));
            }
            Some("23503" | "23505" | "23514") => {
                return AccessError::Conflict(format!("{context}: {error}"));
            }
            _ => {}
        }
    }
    AccessError::Unavailable(format!("{context}: {error}"))
}
