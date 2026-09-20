//! SQLite transactional authority committer.

use async_trait::async_trait;
use edgequake_storage_contracts::{
    checked_i64, decode_commit_receipt, physical_revision_id, validate_prepared_ingestion_batch,
    AccessError, AccessResult, CommitReceipt, CommittedRevision, CursorPage, DeleteDocument,
    DeleteReceipt, DocumentId, DocumentPageRequest, DocumentReader, DocumentView,
    IngestionCommitter, LifecycleCommitter, PreparedIngestionBatch, PreparedRecord, AccessScope,
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

        let delivery_count = sqlx::query(
            "INSERT OR IGNORE INTO projection_deliveries (event_id,binding_id,state) \
             SELECT ?,binding_id,'pending' FROM data_bindings \
             WHERE tenant_id=? AND workspace_id=? AND state='active'",
        )
        .bind(event_id.to_string())
        .bind(command.scope.tenant().to_string())
        .bind(command.scope.workspace().to_string())
        .execute(&mut *tx)
        .await
        .map_err(database_error)?
        .rows_affected();

        let active_bindings: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM data_bindings \
             WHERE tenant_id=? AND workspace_id=? AND state='active'",
        )
        .bind(command.scope.tenant().to_string())
        .bind(command.scope.workspace().to_string())
        .fetch_one(&mut *tx)
        .await
        .map_err(database_error)?;
        if active_bindings > 0 && delivery_count == 0 {
            return Err(AccessError::Conflict(
                "active bindings exist but no projection deliveries were created".into(),
            ));
        }

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
        let mut out = Vec::with_capacity(ids.len());
        for document_id in ids {
            let row = sqlx::query_as::<_, (i64, String, Vec<u8>)>(
                "SELECT revision, state, digest FROM object_revisions \
                 WHERE tenant_id=? AND workspace_id=? AND kind='document' AND logical_id=? \
                 ORDER BY revision DESC LIMIT 1",
            )
            .bind(scope.tenant().to_string())
            .bind(scope.workspace().to_string())
            .bind(document_id.to_string())
            .fetch_optional(&self.pool)
            .await
            .map_err(database_error)?;
            out.push(row.map(|(revision, state, digest)| {
                let mut digest_arr = [0u8; 32];
                if digest.len() == 32 {
                    digest_arr.copy_from_slice(&digest);
                }
                DocumentView {
                    scope: *scope,
                    document_id: *document_id,
                    revision: u64::try_from(revision.max(0)).unwrap_or(0),
                    digest: digest_arr,
                    deleted: state == "tombstoned",
                }
            }));
        }
        Ok(out)
    }

    async fn list(
        &self,
        scope: &AccessScope,
        request: &DocumentPageRequest,
    ) -> AccessResult<CursorPage<DocumentView>> {
        let limit = request.limit.clamp(1, 100) as i64;
        let cursor = request
            .cursor
            .as_deref()
            .and_then(|raw| Uuid::parse_str(raw).ok())
            .map(|id| id.to_string())
            .unwrap_or_default();
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

        let mut next_cursor = None;
        let mut items = Vec::new();
        for (idx, (logical_id, revision, state, digest)) in rows.into_iter().enumerate() {
            if idx as i64 >= limit {
                next_cursor = Some(logical_id);
                break;
            }
            let Ok(document_id) = Uuid::parse_str(&logical_id) else {
                continue;
            };
            let mut digest_arr = [0u8; 32];
            if digest.len() == 32 {
                digest_arr.copy_from_slice(&digest);
            }
            items.push(DocumentView {
                scope: *scope,
                document_id: DocumentId::new(document_id),
                revision: u64::try_from(revision.max(0)).unwrap_or(0),
                digest: digest_arr,
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
