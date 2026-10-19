//! Set-based ledger writers for the durable ingestion committer (SPEC-149).
//!
//! # WHY this module exists
//!
//! `commit_batch` runs inside one transaction and must prove that every
//! immutable row already stored equals the row being committed. The first
//! implementation did that per record: one `INSERT`, then one `SELECT` to
//! verify. A 37-page PDF yields ~13k object revisions, ~6.5k contributions and
//! ~6.6k manifest items, so the transaction issued ~47k sequential statements.
//! At ~2 ms per round trip (RLS check + row write) that was **92 s** of the
//! "Storing" stage, measured on a real document.
//!
//! Round trips, not rows, were the cost. Every writer here sends up to
//! [`SLICE_ROWS`] rows per statement as `unnest` arrays, then verifies the slice
//! with one anti-join that returns the first mismatching row. The conflict
//! semantics are unchanged: `ON CONFLICT DO NOTHING` followed by an equality
//! check, so a replay passes and divergent content is a `Conflict`.

use edgequake_storage_contracts::{
    checked_i64, physical_revision_id, AccessError, AccessResult, PreparedIngestionBatch,
    PreparedRecord,
};
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use super::ingestion_committer::classify_sqlx;
use crate::projection_manifest::{role_completion_proof, EventManifestItem};

/// Rows per statement. Bounds the bind payload (payloads are ~1 KB each) while
/// keeping the statement count at `rows / 1000 * 2`.
const SLICE_ROWS: usize = 1_000;

/// Columns shared by `object_revisions` and `embedding_manifests` writes.
#[derive(Default)]
struct RecordColumns {
    ids: Vec<Uuid>,
    revisions: Vec<i64>,
    physical_ids: Vec<Uuid>,
    digests: Vec<Vec<u8>>,
    payloads: Vec<Vec<u8>>,
}

impl RecordColumns {
    fn with_capacity(rows: usize) -> Self {
        Self {
            ids: Vec::with_capacity(rows),
            revisions: Vec::with_capacity(rows),
            physical_ids: Vec::with_capacity(rows),
            digests: Vec::with_capacity(rows),
            payloads: Vec::with_capacity(rows),
        }
    }

    fn push(&mut self, record: &PreparedRecord, revision: i64, physical_id: Uuid) {
        self.ids.push(record.id);
        self.revisions.push(revision);
        self.physical_ids.push(physical_id);
        self.digests.push(record.digest.to_vec());
        self.payloads.push(record.payload.clone());
    }
}

/// Insert immutable object revisions, then verify stored content equals ours.
pub(super) async fn insert_object_revisions(
    tx: &mut Transaction<'_, Postgres>,
    command: &PreparedIngestionBatch,
    kind: &str,
    records: &[PreparedRecord],
) -> AccessResult<()> {
    let tenant_id = command.scope.tenant().into_uuid();
    let workspace_id = command.scope.workspace().into_uuid();
    for slice in records.chunks(SLICE_ROWS) {
        let mut cols = RecordColumns::with_capacity(slice.len());
        for record in slice {
            let revision = checked_i64(record.revision, "record revision")?;
            cols.push(
                record,
                revision,
                physical_revision_id(command, kind, record),
            );
        }
        sqlx::query(
            r#"
            INSERT INTO public.object_revisions (
                tenant_id, workspace_id, kind, logical_id, revision, state,
                physical_id, digest, payload_ref, payload
            )
            SELECT $1, $2, $3, u.logical_id, u.revision, 'staged',
                   u.physical_id, u.digest, NULL, u.payload
            FROM unnest($4::uuid[], $5::bigint[], $6::uuid[], $7::bytea[], $8::bytea[])
                 AS u(logical_id, revision, physical_id, digest, payload)
            ON CONFLICT (tenant_id, workspace_id, kind, logical_id, revision)
                DO NOTHING
            "#,
        )
        .bind(tenant_id)
        .bind(workspace_id)
        .bind(kind)
        .bind(&cols.ids)
        .bind(&cols.revisions)
        .bind(&cols.physical_ids)
        .bind(&cols.digests)
        .bind(&cols.payloads)
        .execute(&mut **tx)
        .await
        .map_err(|error| classify_sqlx("insert immutable object revision", error))?;

        let divergent: Option<(Uuid, i64)> = sqlx::query_as(
            r#"
            SELECT u.logical_id, u.revision
            FROM unnest($4::uuid[], $5::bigint[], $6::uuid[], $7::bytea[], $8::bytea[])
                 AS u(logical_id, revision, physical_id, digest, payload)
            LEFT JOIN public.object_revisions r
              ON r.tenant_id = $1 AND r.workspace_id = $2 AND r.kind = $3
             AND r.logical_id = u.logical_id AND r.revision = u.revision
            WHERE r.logical_id IS NULL
               OR r.physical_id IS DISTINCT FROM u.physical_id
               OR r.digest IS DISTINCT FROM u.digest
               OR r.payload IS DISTINCT FROM u.payload
            LIMIT 1
            "#,
        )
        .bind(tenant_id)
        .bind(workspace_id)
        .bind(kind)
        .bind(&cols.ids)
        .bind(&cols.revisions)
        .bind(&cols.physical_ids)
        .bind(&cols.digests)
        .bind(&cols.payloads)
        .fetch_optional(&mut **tx)
        .await
        .map_err(|error| classify_sqlx("verify immutable object revision", error))?;
        if let Some((id, revision)) = divergent {
            return Err(AccessError::Conflict(format!(
                "{kind} {id} revision {revision} conflicts with persisted content"
            )));
        }
    }
    Ok(())
}

/// Columns of one `graph_contributions` slice.
#[derive(Default)]
struct ContributionColumns {
    fact_ids: Vec<Uuid>,
    fact_revisions: Vec<i64>,
    contribution_ids: Vec<Uuid>,
    digests: Vec<Vec<u8>>,
    payloads: Vec<serde_json::Value>,
}

impl ContributionColumns {
    fn from_records(records: &[PreparedRecord]) -> AccessResult<Self> {
        let mut cols = Self::default();
        for record in records {
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
            cols.fact_ids.push(fact_id);
            cols.fact_revisions.push(fact_revision);
            cols.contribution_ids.push(record.id);
            cols.digests.push(record.digest.to_vec());
            cols.payloads.push(payload);
        }
        Ok(cols)
    }
}

/// Insert graph contributions, then verify scope, fact and digest match.
pub(super) async fn insert_contributions(
    tx: &mut Transaction<'_, Postgres>,
    command: &PreparedIngestionBatch,
) -> AccessResult<()> {
    let tenant_id = command.scope.tenant().into_uuid();
    let workspace_id = command.scope.workspace().into_uuid();
    let source_document_id = command.document_id.into_uuid();
    let source_generation = checked_i64(command.ingest_generation, "ingest generation")?;
    for slice in command.contributions.chunks(SLICE_ROWS) {
        let cols = ContributionColumns::from_records(slice)?;
        sqlx::query(
            r#"
            INSERT INTO public.graph_contributions (
                tenant_id, workspace_id, fact_id, fact_revision, contribution_id,
                source_document_id, source_generation, source_chunk_id,
                payload_digest, payload
            )
            SELECT $1, $2, u.fact_id, u.fact_revision, u.contribution_id,
                   $3, $4, NULL, u.digest, u.payload
            FROM unnest($5::uuid[], $6::bigint[], $7::uuid[], $8::bytea[], $9::jsonb[])
                 AS u(fact_id, fact_revision, contribution_id, digest, payload)
            ON CONFLICT (contribution_id) DO NOTHING
            "#,
        )
        .bind(tenant_id)
        .bind(workspace_id)
        .bind(source_document_id)
        .bind(source_generation)
        .bind(&cols.fact_ids)
        .bind(&cols.fact_revisions)
        .bind(&cols.contribution_ids)
        .bind(&cols.digests)
        .bind(&cols.payloads)
        .execute(&mut **tx)
        .await
        .map_err(|error| classify_sqlx("insert graph contribution", error))?;

        let divergent: Option<Uuid> = sqlx::query_scalar(
            r#"
            SELECT u.contribution_id
            FROM unnest($5::uuid[], $6::bigint[], $7::uuid[], $8::bytea[])
                 AS u(fact_id, fact_revision, contribution_id, digest)
            LEFT JOIN public.graph_contributions g ON g.contribution_id = u.contribution_id
            WHERE g.contribution_id IS NULL
               OR g.tenant_id <> $1
               OR g.workspace_id <> $2
               OR g.fact_id <> u.fact_id
               OR g.fact_revision <> u.fact_revision
               OR g.source_generation <> $4
               OR g.payload_digest IS DISTINCT FROM u.digest
            LIMIT 1
            "#,
        )
        .bind(tenant_id)
        .bind(workspace_id)
        .bind(source_document_id)
        .bind(source_generation)
        .bind(&cols.fact_ids)
        .bind(&cols.fact_revisions)
        .bind(&cols.contribution_ids)
        .bind(&cols.digests)
        .fetch_optional(&mut **tx)
        .await
        .map_err(|error| classify_sqlx("verify graph contribution", error))?;
        if let Some(id) = divergent {
            return Err(AccessError::Conflict(format!(
                "contribution {id} conflicts with persisted content"
            )));
        }
    }
    Ok(())
}

/// Insert embedding manifests, then verify stored content equals ours.
pub(super) async fn insert_embeddings(
    tx: &mut Transaction<'_, Postgres>,
    command: &PreparedIngestionBatch,
) -> AccessResult<()> {
    let tenant_id = command.scope.tenant().into_uuid();
    let workspace_id = command.scope.workspace().into_uuid();
    let content_revision = checked_i64(command.ingest_generation, "ingest generation")?;
    for slice in command.embeddings.chunks(SLICE_ROWS) {
        let mut cols = RecordColumns::with_capacity(slice.len());
        for record in slice {
            let physical_id = physical_revision_id(command, "embedding", record);
            cols.push(record, content_revision, physical_id);
        }
        sqlx::query(
            r#"
            INSERT INTO public.embedding_manifests (
                tenant_id, workspace_id, subject_id, family, model_revision,
                content_revision, physical_id, dimension, metric, digest,
                payload_ref, payload
            )
            SELECT $1, $2, u.subject_id, 'default', 'unspecified',
                   $3, u.physical_id, 0, 'unspecified', u.digest, NULL, u.payload
            FROM unnest($4::uuid[], $5::uuid[], $6::bytea[], $7::bytea[])
                 AS u(subject_id, physical_id, digest, payload)
            ON CONFLICT (
                tenant_id, workspace_id, subject_id, family,
                model_revision, content_revision
            ) DO NOTHING
            "#,
        )
        .bind(tenant_id)
        .bind(workspace_id)
        .bind(content_revision)
        .bind(&cols.ids)
        .bind(&cols.physical_ids)
        .bind(&cols.digests)
        .bind(&cols.payloads)
        .execute(&mut **tx)
        .await
        .map_err(|error| classify_sqlx("insert embedding manifest", error))?;

        let divergent: Option<Uuid> = sqlx::query_scalar(
            r#"
            SELECT u.subject_id
            FROM unnest($4::uuid[], $5::uuid[], $6::bytea[], $7::bytea[])
                 AS u(subject_id, physical_id, digest, payload)
            LEFT JOIN public.embedding_manifests m
              ON m.tenant_id = $1 AND m.workspace_id = $2
             AND m.subject_id = u.subject_id AND m.family = 'default'
             AND m.model_revision = 'unspecified' AND m.content_revision = $3
            WHERE m.subject_id IS NULL
               OR m.physical_id IS DISTINCT FROM u.physical_id
               OR m.digest IS DISTINCT FROM u.digest
               OR m.payload IS DISTINCT FROM u.payload
            LIMIT 1
            "#,
        )
        .bind(tenant_id)
        .bind(workspace_id)
        .bind(content_revision)
        .bind(&cols.ids)
        .bind(&cols.physical_ids)
        .bind(&cols.digests)
        .bind(&cols.payloads)
        .fetch_optional(&mut **tx)
        .await
        .map_err(|error| classify_sqlx("verify embedding manifest", error))?;
        if let Some(id) = divergent {
            return Err(AccessError::Conflict(format!(
                "embedding {id} generation {} conflicts with persisted content",
                command.ingest_generation
            )));
        }
    }
    Ok(())
}

/// Columns of one `projection_event_items` slice.
#[derive(Default)]
struct ManifestColumns {
    ordinals: Vec<i32>,
    item_kinds: Vec<String>,
    record_ids: Vec<Uuid>,
    revisions: Vec<i64>,
    digests: Vec<Vec<u8>>,
    logical_keys: Vec<String>,
}

impl ManifestColumns {
    /// `first_ordinal` is the slice's offset inside its role, so ordinals stay
    /// the global position the role completion proof was computed over.
    fn from_items(first_ordinal: usize, items: &[EventManifestItem]) -> AccessResult<Self> {
        let mut cols = Self::default();
        for (offset, item) in items.iter().enumerate() {
            let ordinal = i32::try_from(first_ordinal + offset)
                .map_err(|_| AccessError::InvalidInput("manifest ordinal exceeds i32".into()))?;
            cols.ordinals.push(ordinal);
            cols.item_kinds.push(item.item_kind.clone());
            cols.record_ids.push(item.record_id);
            cols.revisions.push(item.record_revision);
            cols.digests.push(item.digest.to_vec());
            cols.logical_keys.push(item.logical_key.clone());
        }
        Ok(cols)
    }
}

/// Write the event manifest and the per-role completion proofs.
pub(super) async fn write_event_manifest(
    tx: &mut Transaction<'_, Postgres>,
    event_id: Uuid,
    graph_items: &[EventManifestItem],
    vector_items: &[EventManifestItem],
) -> AccessResult<()> {
    for (role, items) in [("graph", graph_items), ("vector", vector_items)] {
        insert_manifest_items(tx, event_id, role, items).await?;
        write_role_proof(tx, event_id, role, items).await?;
    }
    Ok(())
}

async fn insert_manifest_items(
    tx: &mut Transaction<'_, Postgres>,
    event_id: Uuid,
    role: &str,
    items: &[EventManifestItem],
) -> AccessResult<()> {
    for (slice_index, slice) in items.chunks(SLICE_ROWS).enumerate() {
        let cols = ManifestColumns::from_items(slice_index * SLICE_ROWS, slice)?;
        sqlx::query(
            "INSERT INTO public.projection_event_items (
                 event_id, role, ordinal, item_kind, record_id,
                 record_revision, digest, logical_key
             )
             SELECT $1, $2, u.ordinal, u.item_kind, u.record_id,
                    u.record_revision, u.digest, u.logical_key
             FROM unnest($3::int4[], $4::text[], $5::uuid[], $6::bigint[], $7::bytea[], $8::text[])
                  AS u(ordinal, item_kind, record_id, record_revision, digest, logical_key)
             ON CONFLICT (event_id, role, ordinal) DO NOTHING",
        )
        .bind(event_id)
        .bind(role)
        .bind(&cols.ordinals)
        .bind(&cols.item_kinds)
        .bind(&cols.record_ids)
        .bind(&cols.revisions)
        .bind(&cols.digests)
        .bind(&cols.logical_keys)
        .execute(&mut **tx)
        .await
        .map_err(|error| classify_sqlx("insert projection manifest item", error))?;
    }
    Ok(())
}

async fn write_role_proof(
    tx: &mut Transaction<'_, Postgres>,
    event_id: Uuid,
    role: &str,
    items: &[EventManifestItem],
) -> AccessResult<()> {
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
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(n: u8) -> EventManifestItem {
        EventManifestItem {
            role: "graph".into(),
            item_kind: "graph_contribution".into(),
            record_id: Uuid::from_u128(n as u128),
            record_revision: 1,
            digest: [n; 32],
            logical_key: format!("key-{n}"),
        }
    }

    #[test]
    fn manifest_ordinals_continue_across_slices() {
        let items = [item(1), item(2), item(3)];
        let cols = ManifestColumns::from_items(SLICE_ROWS, &items).expect("columns");
        assert_eq!(
            cols.ordinals,
            vec![
                SLICE_ROWS as i32,
                SLICE_ROWS as i32 + 1,
                SLICE_ROWS as i32 + 2
            ]
        );
        assert_eq!(cols.logical_keys, vec!["key-1", "key-2", "key-3"]);
    }

    #[test]
    fn manifest_ordinal_overflow_is_invalid_input() {
        let result = ManifestColumns::from_items(i32::MAX as usize, &[item(1), item(2)]);
        assert!(matches!(result, Err(AccessError::InvalidInput(_))));
    }

    #[test]
    fn contribution_payload_names_fact_and_revision() {
        let fact = Uuid::from_u128(7);
        let record = PreparedRecord {
            id: Uuid::from_u128(9),
            revision: 3,
            digest: [1; 32],
            payload: serde_json::to_vec(
                &serde_json::json!({ "fact_id": fact.to_string(), "fact_revision": 5 }),
            )
            .expect("json"),
        };
        let cols = ContributionColumns::from_records(&[record]).expect("columns");
        assert_eq!(cols.fact_ids, vec![fact]);
        assert_eq!(cols.fact_revisions, vec![5]);
        assert_eq!(cols.contribution_ids, vec![Uuid::from_u128(9)]);
    }

    #[test]
    fn contribution_without_fact_fields_falls_back_to_record() {
        let record = PreparedRecord {
            id: Uuid::from_u128(9),
            revision: 3,
            digest: [1; 32],
            payload: b"not json".to_vec(),
        };
        let cols = ContributionColumns::from_records(&[record]).expect("columns");
        assert_eq!(cols.fact_ids, vec![Uuid::from_u128(9)]);
        assert_eq!(cols.fact_revisions, vec![3]);
        assert_eq!(cols.payloads[0]["bytes"].as_array().map(Vec::len), Some(8));
    }
}
