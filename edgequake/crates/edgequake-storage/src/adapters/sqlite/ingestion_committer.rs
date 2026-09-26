//! SQLite transactional authority committer.

use crate::projection_manifest::{
    logical_key_for_payload, role_completion_proof, EventManifestItem,
};
use async_trait::async_trait;
use edgequake_storage_contracts::{
    checked_i64, decode_commit_receipt, physical_revision_id, validate_prepared_ingestion_batch,
    AccessError, AccessResult, AccessScope, CommitReceipt, CommittedRevision, CursorPage,
    DeleteDocument, DeleteReceipt, DocumentId, DocumentPageRequest, DocumentReader, DocumentView,
    IngestionCommitter, LifecycleCommitter, PreparedIngestionBatch, PreparedRecord,
};
use sqlx::{Connection, Sqlite, SqlitePool, Transaction};
use uuid::Uuid;

const COMMIT_OPERATION: &str = "ingestion.commit_batch";
const DELETE_OPERATION: &str = "lifecycle.tombstone_document";

#[derive(Clone)]
pub struct SqliteIngestionCommitter {
    pool: SqlitePool,
}

impl SqliteIngestionCommitter {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl IngestionCommitter for SqliteIngestionCommitter {
    async fn commit_batch(&self, command: &PreparedIngestionBatch) -> AccessResult<CommitReceipt> {
        let validated = validate_prepared_ingestion_batch(command)?;
        let mut connection = self.pool.acquire().await.map_err(database_error)?;
        let mut tx = connection
            .begin_with("BEGIN IMMEDIATE")
            .await
            .map_err(database_error)?;

        if let Some(receipt) = find_receipt(&mut tx, command).await? {
            tx.rollback().await.map_err(database_error)?;
            return Ok(receipt);
        }
        ensure_document(&mut tx, command).await?;
        verify_expected_revision(&mut tx, command, validated.expected_revision).await?;

        for (kind, records) in [
            ("chunk", command.chunks.as_slice()),
            ("fact", command.facts.as_slice()),
            ("contribution", command.contributions.as_slice()),
            ("embedding", command.embeddings.as_slice()),
        ] {
            insert_records(&mut tx, command, kind, records).await?;
        }
        insert_contributions(&mut tx, command).await?;

        sqlx::query(
            "INSERT INTO ingest_batches \
             (tenant_id,workspace_id,document_id,generation,batch_ordinal,digest,expected_count,state) \
             VALUES (?,?,?,?,?,?,?,'staged')",
        )
        .bind(command.scope.tenant().to_string())
        .bind(command.scope.workspace().to_string())
        .bind(command.document_id.to_string())
        .bind(checked_i64(command.ingest_generation, "ingest generation")?)
        .bind(i64::from(validated.batch_ordinal))
        .bind(command.canonical_digest.to_vec())
        .bind(i64::from(validated.expected_count))
        .execute(&mut *tx)
        .await
        .map_err(database_error)?;

        // Reject silent ignore of conflicting batch digests: verify the row.
        let batch_digest: Vec<u8> = sqlx::query_scalar(
            "SELECT digest FROM ingest_batches \
             WHERE tenant_id=? AND workspace_id=? AND document_id=? \
               AND generation=? AND batch_ordinal=?",
        )
        .bind(command.scope.tenant().to_string())
        .bind(command.scope.workspace().to_string())
        .bind(command.document_id.to_string())
        .bind(checked_i64(command.ingest_generation, "ingest generation")?)
        .bind(i64::from(validated.batch_ordinal))
        .fetch_one(&mut *tx)
        .await
        .map_err(database_error)?;
        if batch_digest != command.canonical_digest {
            return Err(AccessError::Conflict(
                "ingest batch digest conflicts with persisted content".into(),
            ));
        }

        advance_document_revision(&mut tx, command).await?;

        let event_id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO projection_events \
             (event_id,tenant_id,workspace_id,object_kind,object_id,object_revision, \
              schema_version,operation,manifest_ref,digest) \
             VALUES (?,?,?,?,?,?,?,?,?,?)",
        )
        .bind(event_id.to_string())
        .bind(command.scope.tenant().to_string())
        .bind(command.scope.workspace().to_string())
        .bind("document_batch")
        .bind(command.document_id.to_string())
        .bind(checked_i64(command.ingest_generation, "ingest generation")?)
        .bind(i64::from(command.schema_version))
        .bind(format!("ingest_batch:{}", command.batch_ordinal))
        .bind(format!(
            "ingest-batch://{}/{}/{}",
            command.document_id, command.ingest_generation, command.batch_ordinal
        ))
        .bind(command.canonical_digest.to_vec())
        .execute(&mut *tx)
        .await
        .map_err(database_error)?;

        ensure_sqlite_p0_bindings(
            &mut tx,
            &command.scope.tenant().to_string(),
            &command.scope.workspace().to_string(),
        )
        .await?;
        let delivery_count = sqlx::query(
            "INSERT OR IGNORE INTO projection_deliveries (event_id,binding_id,state) \
             SELECT ?,binding_id,'pending' FROM data_bindings \
             WHERE tenant_id=? AND workspace_id=? AND state IN ('active','draining') \
               AND role IN ('graph','vector')",
        )
        .bind(event_id.to_string())
        .bind(command.scope.tenant().to_string())
        .bind(command.scope.workspace().to_string())
        .execute(&mut *tx)
        .await
        .map_err(database_error)?
        .rows_affected();
        if delivery_count < 2 {
            return Err(AccessError::Unavailable(
                "durable ingest refused: scope is missing graph or vector bindings".into(),
            ));
        }
        write_sqlite_manifest(
            &mut tx,
            event_id,
            &manifest_items("graph", "graph_contribution", &command.contributions),
            &manifest_items("vector", "embedding", &command.embeddings),
        )
        .await?;

        let receipt = build_receipt(command, event_id);
        let receipt_bytes = serde_json::to_vec(&receipt)
            .map_err(|error| AccessError::CorruptData(error.to_string()))?;
        sqlx::query(
            "INSERT INTO mutation_requests \
             (tenant_id,workspace_id,operation,idempotency_key,digest,receipt) \
             VALUES (?,?,?,?,?,?)",
        )
        .bind(command.scope.tenant().to_string())
        .bind(command.scope.workspace().to_string())
        .bind(COMMIT_OPERATION)
        .bind(&command.idempotency_key)
        .bind(command.canonical_digest.to_vec())
        .bind(receipt_bytes)
        .execute(&mut *tx)
        .await
        .map_err(database_error)?;

        tx.commit().await.map_err(|error| {
            AccessError::UnknownOutcome(format!(
                "SQLite ingestion commit {}: {error}",
                command.idempotency_key
            ))
        })?;
        Ok(receipt)
    }
}

#[async_trait]
impl LifecycleCommitter for SqliteIngestionCommitter {
    async fn tombstone_document(&self, command: &DeleteDocument) -> AccessResult<DeleteReceipt> {
        let expected_revision = checked_i64(command.expected_revision, "expected revision")?;
        let tombstone_revision = command
            .expected_revision
            .checked_add(1)
            .ok_or_else(|| AccessError::InvalidInput("tombstone revision overflow".into()))?;
        let tombstone_revision_i64 = checked_i64(tombstone_revision, "tombstone revision")?;

        let mut connection = self.pool.acquire().await.map_err(database_error)?;
        let mut tx = connection
            .begin_with("BEGIN IMMEDIATE")
            .await
            .map_err(database_error)?;

        if let Some((digest, receipt)) = sqlx::query_as::<_, (Vec<u8>, Vec<u8>)>(
            "SELECT digest,receipt FROM mutation_requests \
             WHERE tenant_id=? AND workspace_id=? AND operation=? AND idempotency_key=?",
        )
        .bind(command.scope.tenant().to_string())
        .bind(command.scope.workspace().to_string())
        .bind(DELETE_OPERATION)
        .bind(&command.idempotency_key)
        .fetch_optional(&mut *tx)
        .await
        .map_err(database_error)?
        {
            if digest != command.command_digest {
                return Err(AccessError::Conflict(
                    "tombstone idempotency key was reused with another digest".into(),
                ));
            }
            let receipt = serde_json::from_slice(&receipt).map_err(|error| {
                AccessError::CorruptData(format!("decode tombstone receipt: {error}"))
            })?;
            tx.rollback().await.map_err(database_error)?;
            return Ok(receipt);
        }

        let current_revision = sqlx::query_scalar::<_, Option<i64>>(
            "SELECT MAX(revision) FROM object_revisions \
             WHERE tenant_id=? AND workspace_id=? AND kind='document' AND logical_id=?",
        )
        .bind(command.scope.tenant().to_string())
        .bind(command.scope.workspace().to_string())
        .bind(command.document_id.to_string())
        .fetch_one(&mut *tx)
        .await
        .map_err(database_error)?
        .unwrap_or(0);
        if current_revision != expected_revision {
            return Err(AccessError::Conflict(format!(
                "document revision changed: expected {expected_revision}, current {current_revision}"
            )));
        }

        let cleanup_manifest_id = Uuid::new_v4();
        let physical_id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO object_revisions \
             (tenant_id,workspace_id,kind,logical_id,revision,state,physical_id,digest,payload) \
             VALUES (?,?,?,?,?,'tombstoned',?,?,?)",
        )
        .bind(command.scope.tenant().to_string())
        .bind(command.scope.workspace().to_string())
        .bind("document")
        .bind(command.document_id.to_string())
        .bind(tombstone_revision_i64)
        .bind(physical_id.to_string())
        .bind(command.command_digest.to_vec())
        .bind(Vec::<u8>::new())
        .execute(&mut *tx)
        .await
        .map_err(database_error)?;

        sqlx::query("UPDATE documents SET deleted=1, revision=? WHERE id=? AND tenant_id=? AND workspace_id=?")
            .bind(tombstone_revision_i64)
            .bind(command.document_id.to_string())
            .bind(command.scope.tenant().to_string())
            .bind(command.scope.workspace().to_string())
            .execute(&mut *tx)
            .await
            .map_err(database_error)?;

        let target_binding_ids = sqlx::query_scalar::<_, String>(
            "SELECT binding_id FROM data_bindings \
             WHERE tenant_id=? AND workspace_id=? AND state IN ('active','draining') \
             ORDER BY binding_id",
        )
        .bind(command.scope.tenant().to_string())
        .bind(command.scope.workspace().to_string())
        .fetch_all(&mut *tx)
        .await
        .map_err(database_error)?
        .into_iter()
        .filter_map(|raw| Uuid::parse_str(&raw).ok())
        .collect::<Vec<_>>();

        let event_id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO projection_events \
             (event_id,tenant_id,workspace_id,object_kind,object_id,object_revision, \
              schema_version,operation,manifest_ref,digest) \
             VALUES (?,?,?,?,?,?,1,'delete',?,?)",
        )
        .bind(event_id.to_string())
        .bind(command.scope.tenant().to_string())
        .bind(command.scope.workspace().to_string())
        .bind("document")
        .bind(command.document_id.to_string())
        .bind(tombstone_revision_i64)
        .bind(format!("cleanup://{cleanup_manifest_id}"))
        .bind(command.command_digest.to_vec())
        .execute(&mut *tx)
        .await
        .map_err(database_error)?;
        for binding_id in &target_binding_ids {
            sqlx::query(
                "INSERT INTO projection_cleanup_intents \
                 (cleanup_manifest_id,binding_id,tenant_id,workspace_id,document_id,tombstone_revision) \
                 VALUES (?,?,?,?,?,?)",
            )
            .bind(cleanup_manifest_id.to_string())
            .bind(binding_id.to_string())
            .bind(command.scope.tenant().to_string())
            .bind(command.scope.workspace().to_string())
            .bind(command.document_id.to_string())
            .bind(tombstone_revision_i64)
            .execute(&mut *tx)
            .await
            .map_err(database_error)?;
            sqlx::query(
                "INSERT INTO projection_deliveries (event_id,binding_id,state) VALUES (?,?, 'pending')",
            )
            .bind(event_id.to_string())
            .bind(binding_id.to_string())
            .execute(&mut *tx)
            .await
            .map_err(database_error)?;
        }
        let contributions = sqlx::query_as::<_, (String, i64, Vec<u8>, Vec<u8>)>(
            "SELECT contribution_id, source_generation, payload_digest, payload \
             FROM graph_contributions WHERE tenant_id=? AND workspace_id=? AND source_document_id=?",
        )
        .bind(command.scope.tenant().to_string())
        .bind(command.scope.workspace().to_string())
        .bind(command.document_id.to_string())
        .fetch_all(&mut *tx)
        .await
        .map_err(database_error)?;
        let graph_items = contributions
            .into_iter()
            .filter_map(|(id, revision, digest, payload)| {
                let record_id = Uuid::parse_str(&id).ok()?;
                let digest: [u8; 32] = digest.try_into().ok()?;
                Some(EventManifestItem {
                    role: "graph".into(),
                    item_kind: "graph_contribution".into(),
                    record_id,
                    record_revision: revision.max(1),
                    digest,
                    logical_key: logical_key_for_payload(&payload, record_id),
                })
            })
            .collect::<Vec<_>>();
        write_sqlite_manifest(&mut tx, event_id, &graph_items, &[]).await?;

        let receipt = DeleteReceipt {
            scope: command.scope,
            document_id: command.document_id,
            tombstone_revision,
            cleanup_manifest_id,
            target_binding_ids,
        };
        let receipt_bytes = serde_json::to_vec(&receipt)
            .map_err(|error| AccessError::CorruptData(error.to_string()))?;
        sqlx::query(
            "INSERT INTO mutation_requests \
             (tenant_id,workspace_id,operation,idempotency_key,digest,receipt) \
             VALUES (?,?,?,?,?,?)",
        )
        .bind(command.scope.tenant().to_string())
        .bind(command.scope.workspace().to_string())
        .bind(DELETE_OPERATION)
        .bind(&command.idempotency_key)
        .bind(command.command_digest.to_vec())
        .bind(receipt_bytes)
        .execute(&mut *tx)
        .await
        .map_err(database_error)?;

        tx.commit().await.map_err(|error| {
            AccessError::UnknownOutcome(format!(
                "SQLite tombstone commit {}: {error}",
                command.idempotency_key
            ))
        })?;
        Ok(receipt)
    }
}

#[async_trait]
impl DocumentReader for SqliteIngestionCommitter {
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
        let wanted: Vec<String> = ids.iter().map(ToString::to_string).collect();
        let filter = serde_json::to_string(&wanted)
            .map_err(|error| AccessError::InvalidInput(format!("document id filter: {error}")))?;
        let rows = sqlx::query_as::<_, (String, i64, String, Vec<u8>)>(
            "SELECT logical_id, revision, state, digest FROM object_revisions r \
             WHERE tenant_id = ?1 AND workspace_id = ?2 AND kind = 'document' \
               AND logical_id IN (SELECT value FROM json_each(?3)) \
               AND revision = ( \
                 SELECT MAX(r2.revision) FROM object_revisions r2 \
                 WHERE r2.tenant_id = r.tenant_id \
                   AND r2.workspace_id = r.workspace_id \
                   AND r2.kind = 'document' \
                   AND r2.logical_id = r.logical_id \
               )",
        )
        .bind(scope.tenant().to_string())
        .bind(scope.workspace().to_string())
        .bind(filter)
        .fetch_all(&self.pool)
        .await
        .map_err(database_error)?;
        let mut by_id = std::collections::HashMap::new();
        for (logical_id, revision, state, digest) in rows {
            let document_id = Uuid::parse_str(&logical_id).map_err(|_| {
                AccessError::CorruptData("document logical id is not a UUID".into())
            })?;
            by_id.insert(
                document_id,
                document_view(scope, document_id, revision, &state, digest)?,
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
        let limit = request.limit.clamp(1, 100) as i64;
        let cursor = match request.cursor.as_deref() {
            None => String::new(),
            Some(raw) => Uuid::parse_str(raw)
                .map_err(|_| {
                    AccessError::InvalidInput("document page cursor is not a UUID".into())
                })?
                .to_string(),
        };
        let rows = sqlx::query_as::<_, (String, i64, String, Vec<u8>)>(
            "SELECT logical_id, revision, state, digest FROM object_revisions \
             WHERE tenant_id=? AND workspace_id=? AND kind='document' \
               AND logical_id > ? \
             AND revision = ( \
               SELECT MAX(r2.revision) FROM object_revisions r2 \
               WHERE r2.tenant_id=object_revisions.tenant_id \
                 AND r2.workspace_id=object_revisions.workspace_id \
                 AND r2.kind='document' AND r2.logical_id=object_revisions.logical_id \
             ) \
             ORDER BY logical_id ASC LIMIT ?",
        )
        .bind(scope.tenant().to_string())
        .bind(scope.workspace().to_string())
        .bind(cursor)
        .bind(limit + 1)
        .fetch_all(&self.pool)
        .await
        .map_err(database_error)?;

        let ids: Vec<String> = rows
            .iter()
            .map(|(logical_id, _, _, _)| logical_id.clone())
            .collect();
        let next_cursor = edgequake_storage_contracts::last_included_cursor(limit as usize, &ids);
        let mut items = Vec::new();
        for (logical_id, revision, state, digest) in rows.into_iter().take(limit as usize) {
            let document_id = Uuid::parse_str(&logical_id)
                .map_err(|_| AccessError::CorruptData("document id is not a UUID".into()))?;
            let digest: [u8; 32] = digest
                .try_into()
                .map_err(|_| AccessError::CorruptData("document digest is not 32 bytes".into()))?;
            items.push(DocumentView {
                scope: *scope,
                document_id: DocumentId::new(document_id),
                revision: u64::try_from(revision.max(0)).unwrap_or(0),
                digest,
                deleted: state == "tombstoned",
            });
        }
        Ok(CursorPage { items, next_cursor })
    }
}

async fn find_receipt(
    tx: &mut Transaction<'_, Sqlite>,
    command: &PreparedIngestionBatch,
) -> AccessResult<Option<CommitReceipt>> {
    let row = sqlx::query_as::<_, (Vec<u8>, Vec<u8>)>(
        "SELECT digest,receipt FROM mutation_requests \
         WHERE tenant_id=? AND workspace_id=? AND operation=? AND idempotency_key=?",
    )
    .bind(command.scope.tenant().to_string())
    .bind(command.scope.workspace().to_string())
    .bind(COMMIT_OPERATION)
    .bind(&command.idempotency_key)
    .fetch_optional(&mut **tx)
    .await
    .map_err(database_error)?;
    row.map(|(digest, bytes)| decode_commit_receipt(command, &digest, &bytes))
        .transpose()
}

async fn ensure_document(
    tx: &mut Transaction<'_, Sqlite>,
    command: &PreparedIngestionBatch,
) -> AccessResult<()> {
    sqlx::query(
        "INSERT OR IGNORE INTO documents (id,tenant_id,workspace_id,title) VALUES (?,?,?,'')",
    )
    .bind(command.document_id.to_string())
    .bind(command.scope.tenant().to_string())
    .bind(command.scope.workspace().to_string())
    .execute(&mut **tx)
    .await
    .map_err(database_error)?;
    let scope = sqlx::query_as::<_, (String, String)>(
        "SELECT tenant_id,workspace_id FROM documents WHERE id=?",
    )
    .bind(command.document_id.to_string())
    .fetch_one(&mut **tx)
    .await
    .map_err(database_error)?;
    if scope
        != (
            command.scope.tenant().to_string(),
            command.scope.workspace().to_string(),
        )
    {
        return Err(AccessError::ForbiddenScope(
            "document belongs to another tenant/workspace".into(),
        ));
    }
    Ok(())
}

async fn verify_expected_revision(
    tx: &mut Transaction<'_, Sqlite>,
    command: &PreparedIngestionBatch,
    expected: Option<i64>,
) -> AccessResult<()> {
    let Some(expected) = expected else {
        return Ok(());
    };
    let current = sqlx::query_scalar::<_, Option<i64>>(
        "SELECT MAX(revision) FROM object_revisions \
         WHERE tenant_id=? AND workspace_id=? AND kind='document' AND logical_id=?",
    )
    .bind(command.scope.tenant().to_string())
    .bind(command.scope.workspace().to_string())
    .bind(command.document_id.to_string())
    .fetch_one(&mut **tx)
    .await
    .map_err(database_error)?
    .unwrap_or(0);
    if current != expected {
        return Err(AccessError::Conflict(format!(
            "document revision changed: expected {expected}, current {current}"
        )));
    }
    Ok(())
}

async fn advance_document_revision(
    tx: &mut Transaction<'_, Sqlite>,
    command: &PreparedIngestionBatch,
) -> AccessResult<()> {
    let revision = checked_i64(command.ingest_generation, "ingest generation")?;
    let physical_id = Uuid::new_v5(
        &Uuid::NAMESPACE_OID,
        format!(
            "{}:{}:document:{}:{}",
            command.scope.tenant(),
            command.scope.workspace(),
            command.document_id,
            command.ingest_generation
        )
        .as_bytes(),
    );
    sqlx::query(
        "INSERT OR IGNORE INTO object_revisions \
         (tenant_id,workspace_id,kind,logical_id,revision,state,physical_id,digest,payload) \
         VALUES (?,?,?,?,?,'active',?,?,?)",
    )
    .bind(command.scope.tenant().to_string())
    .bind(command.scope.workspace().to_string())
    .bind("document")
    .bind(command.document_id.to_string())
    .bind(revision)
    .bind(physical_id.to_string())
    .bind(command.canonical_digest.to_vec())
    .bind(Vec::<u8>::new())
    .execute(&mut **tx)
    .await
    .map_err(database_error)?;
    sqlx::query(
        "UPDATE documents SET revision=?, deleted=0 WHERE id=? AND tenant_id=? AND workspace_id=?",
    )
    .bind(revision)
    .bind(command.document_id.to_string())
    .bind(command.scope.tenant().to_string())
    .bind(command.scope.workspace().to_string())
    .execute(&mut **tx)
    .await
    .map_err(database_error)?;
    Ok(())
}

async fn insert_records(
    tx: &mut Transaction<'_, Sqlite>,
    command: &PreparedIngestionBatch,
    kind: &str,
    records: &[PreparedRecord],
) -> AccessResult<()> {
    for record in records {
        let physical_id = physical_revision_id(command, kind, record);
        sqlx::query(
            "INSERT OR IGNORE INTO object_revisions \
             (tenant_id,workspace_id,kind,logical_id,revision,state,physical_id,digest,payload) \
             VALUES (?,?,?,?,?,'staged',?,?,?)",
        )
        .bind(command.scope.tenant().to_string())
        .bind(command.scope.workspace().to_string())
        .bind(kind)
        .bind(record.id.to_string())
        .bind(checked_i64(record.revision, "record revision")?)
        .bind(physical_id.to_string())
        .bind(record.digest.to_vec())
        .bind(&record.payload)
        .execute(&mut **tx)
        .await
        .map_err(database_error)?;
        let actual = sqlx::query_as::<_, (Vec<u8>, Vec<u8>)>(
            "SELECT digest,payload FROM object_revisions \
             WHERE tenant_id=? AND workspace_id=? AND kind=? AND logical_id=? AND revision=?",
        )
        .bind(command.scope.tenant().to_string())
        .bind(command.scope.workspace().to_string())
        .bind(kind)
        .bind(record.id.to_string())
        .bind(checked_i64(record.revision, "record revision")?)
        .fetch_one(&mut **tx)
        .await
        .map_err(database_error)?;
        if actual != (record.digest.to_vec(), record.payload.clone()) {
            return Err(AccessError::Conflict(format!(
                "{kind} {} revision {} conflicts with persisted content",
                record.id, record.revision
            )));
        }
    }
    Ok(())
}

async fn insert_contributions(
    tx: &mut Transaction<'_, Sqlite>,
    command: &PreparedIngestionBatch,
) -> AccessResult<()> {
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
            "INSERT OR IGNORE INTO graph_contributions \
             (tenant_id,workspace_id,fact_id,fact_revision,contribution_id, \
              source_document_id,source_generation,payload_digest,payload) \
             VALUES (?,?,?,?,?,?,?,?,?)",
        )
        .bind(command.scope.tenant().to_string())
        .bind(command.scope.workspace().to_string())
        .bind(fact_id.to_string())
        .bind(fact_revision)
        .bind(record.id.to_string())
        .bind(command.document_id.to_string())
        .bind(source_generation)
        .bind(record.digest.to_vec())
        .bind(serde_json::to_vec(&payload).unwrap_or_default())
        .execute(&mut **tx)
        .await
        .map_err(database_error)?;

        let actual = sqlx::query_as::<_, (String, String, String, i64, i64, Vec<u8>)>(
            "SELECT tenant_id,workspace_id,fact_id,fact_revision,source_generation,payload_digest \
             FROM graph_contributions WHERE contribution_id=?",
        )
        .bind(record.id.to_string())
        .fetch_one(&mut **tx)
        .await
        .map_err(database_error)?;
        if actual
            != (
                command.scope.tenant().to_string(),
                command.scope.workspace().to_string(),
                fact_id.to_string(),
                fact_revision,
                source_generation,
                record.digest.to_vec(),
            )
        {
            return Err(AccessError::Conflict(format!(
                "contribution {} conflicts with persisted lineage",
                record.id
            )));
        }
    }
    Ok(())
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

fn sqlite_binding_id(tenant: &str, workspace: &str, role: &str) -> Uuid {
    Uuid::new_v5(
        &Uuid::NAMESPACE_OID,
        format!("p0:{tenant}:{workspace}:{role}").as_bytes(),
    )
}

async fn ensure_sqlite_p0_bindings(
    tx: &mut Transaction<'_, Sqlite>,
    tenant: &str,
    workspace: &str,
) -> AccessResult<()> {
    for role in ["graph", "vector"] {
        sqlx::query(
            "INSERT OR IGNORE INTO data_bindings \
             (binding_id,tenant_id,workspace_id,role,generation,state) \
             VALUES (?,?,?,?,1,'active')",
        )
        .bind(sqlite_binding_id(tenant, workspace, role).to_string())
        .bind(tenant)
        .bind(workspace)
        .bind(role)
        .execute(&mut **tx)
        .await
        .map_err(database_error)?;
    }
    Ok(())
}

async fn write_sqlite_manifest(
    tx: &mut Transaction<'_, Sqlite>,
    event_id: Uuid,
    graph_items: &[EventManifestItem],
    vector_items: &[EventManifestItem],
) -> AccessResult<()> {
    for (role, items) in [("graph", graph_items), ("vector", vector_items)] {
        for (ordinal, item) in items.iter().enumerate() {
            sqlx::query(
                "INSERT OR IGNORE INTO projection_event_items \
                 (event_id,role,ordinal,item_kind,record_id,record_revision,digest,logical_key) \
                 VALUES (?,?,?,?,?,?,?,?)",
            )
            .bind(event_id.to_string())
            .bind(role)
            .bind(ordinal as i64)
            .bind(&item.item_kind)
            .bind(item.record_id.to_string())
            .bind(item.record_revision)
            .bind(item.digest.to_vec())
            .bind(&item.logical_key)
            .execute(&mut **tx)
            .await
            .map_err(database_error)?;
        }
        sqlx::query(
            "INSERT OR IGNORE INTO projection_event_role_proofs (event_id,role,expected_digest) \
             VALUES (?,?,?)",
        )
        .bind(event_id.to_string())
        .bind(role)
        .bind(role_completion_proof(items).to_vec())
        .execute(&mut **tx)
        .await
        .map_err(database_error)?;
    }
    Ok(())
}

fn build_receipt(command: &PreparedIngestionBatch, event_id: Uuid) -> CommitReceipt {
    CommitReceipt {
        request_key: command.idempotency_key.clone(),
        command_digest: command.canonical_digest,
        document_generation: command.ingest_generation,
        committed: command
            .chunks
            .iter()
            .chain(&command.facts)
            .chain(&command.contributions)
            .chain(&command.embeddings)
            .map(|record| CommittedRevision {
                id: record.id,
                revision: record.revision,
            })
            .collect(),
        manifest_id: event_id,
        durable_commit_token: format!("sqlite:{event_id}"),
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

fn database_error(error: sqlx::Error) -> AccessError {
    if let sqlx::Error::Database(database) = &error {
        if matches!(database.code().as_deref(), Some("5" | "6")) {
            return AccessError::SerializationRetry(format!("SQLite busy/locked: {error}"));
        }
        if matches!(database.code().as_deref(), Some("1555" | "2067" | "19")) {
            return AccessError::Conflict(format!("SQLite constraint: {error}"));
        }
    }
    AccessError::Unavailable(format!("SQLite ingestion: {error}"))
}
