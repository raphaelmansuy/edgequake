//! SPEC-091 Migration Console — the Advisor (doc 15 §4).
//!
//! One module derives the migration posture from the live schema (SSOT — it
//! persists nothing, LAW-C1) and hands it to a pure rule engine (`rules`) that
//! produces explicit instructions and gated actions. The CLI renders that
//! output; a future WebUI/API consumes the same `posture()` with no
//! re-derivation (DRY).
//!
//! Layout (SRP):
//! - [`types`]   — the derived-posture types (facts + phases).
//! - [`residue`] — fact #5, the migration-125 durable-row guard reused verbatim.
//! - [`rules`]   — the pure rule engine (posture → instructions + actions).
//! - this module — the `PostureSource` port + Postgres adapter (fact collection).

use sqlx::PgPool;
use uuid::Uuid;

use super::{lease, verify, MigrationMode};
use crate::chunk_text_authority::{chunk_text_authority_from_env, ChunkTextAuthority};
use crate::error::StorageError;
use crate::kv_family_cutover::{kv_family_mode_from_env, KvFamilyMode};
use crate::serving_fence::serving_fence_enabled_from_env;

pub mod residue;
pub mod rules;
pub mod types;

pub use residue::{
    guard_durable_total, kv_durable_residue, kv_durable_residue_all, list_kv_tables,
};
pub use rules::{derive_actions, derive_cutover_phase, derive_family_phase, derive_guidance};
pub use types::{
    CutoverPhase, FamilyMode, FamilyPhase, FamilyPosture, FamilySpec, GuardedAction, Guidance,
    InstrKind, Instruction, JobSnapshot, MigrationPosture, ResidueReport, VectorPosture,
    VerifySummary, FAMILIES,
};

/// The chunk backfill engine step id (SSOT — `chunk_text_backfill.rs`).
const CHUNK_BACKFILL_STEP: &str = "w1-chunk-text-backfill";

/// Port (DIP): a source that derives the migration posture. The Postgres
/// adapter below is the production implementation; tests can construct
/// `MigrationPosture` fixtures directly and exercise the pure rule engine.
pub trait PostureSource {
    fn collect(
        &self,
    ) -> impl std::future::Future<Output = Result<MigrationPosture, StorageError>> + Send;
}

/// Postgres adapter: reads the eight facts (doc 15 §3) from the live schema.
pub struct PostgresPostureSource {
    pool: PgPool,
}

impl PostgresPostureSource {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

impl PostureSource for PostgresPostureSource {
    async fn collect(&self) -> Result<MigrationPosture, StorageError> {
        let pool = &self.pool;

        // Fact #2 — is the KV store dropped? (125 applied OR no eq_*_kv remain).
        let kv_tables = residue::list_kv_tables(pool).await?;
        let drop_applied = drop_migration_applied(pool, 125).await?;
        let kv_store_dropped = drop_applied || kv_tables.is_empty();

        // EC-C3 — is the engine ledger installed?
        let engine_installed = engine_installed(pool).await?;

        // Fact #5 — durable residue across every remaining eq_*_kv table.
        let residue_report = residue::kv_durable_residue_all(pool).await?;

        // Facts #7/#8 — fence + engine mode (env).
        let serving_fence_on = serving_fence_enabled_from_env();
        let engine_mode = MigrationMode::from_env();

        // Fact #1/#4 — chunk backfill job + content verify.
        let chunk_job = chunk_backfill_job(pool).await?;
        let chunk_verify = chunk_verify(pool, &kv_tables).await;

        // Read every family's mode first: the global ready-to-drop signal
        // depends on all of them plus the residue (not on any single phase).
        let modes: Vec<FamilyMode> = FAMILIES.iter().map(family_mode).collect();
        let all_relational = modes.iter().all(|m| *m == FamilyMode::Relational);
        let global_ready_to_drop =
            !kv_store_dropped && residue_report.total() == 0 && all_relational;

        let mut families = Vec::with_capacity(FAMILIES.len());
        for (i, spec) in FAMILIES.iter().enumerate() {
            let mode = modes[i];
            let residue_rows = residue_report.for_family(spec.name);
            let typed_rows = count_typed(pool, spec.typed_tables).await?;
            let backfill = if spec.is_chunk {
                chunk_job.clone()
            } else {
                None
            };
            let verify_result = if spec.is_chunk { chunk_verify } else { None };
            let phase = rules::derive_family_phase(
                mode,
                spec.is_chunk,
                backfill.as_ref(),
                verify_result.as_ref(),
                residue_rows,
                kv_store_dropped,
                global_ready_to_drop,
            );
            families.push(FamilyPosture {
                family: spec.name,
                mode,
                phase,
                durable: spec.durable,
                backfill,
                verify: verify_result,
                kv_residue_rows: residue_rows,
                typed_rows,
                typed_tables: spec.typed_tables,
                env_flag: spec.env_flag,
            });
        }

        // SPEC-091 W3 — VECTOR posture (chunk embeddings cutover), derived from
        // the same live schema (backend flag env + typed/legacy counts + job).
        let vector = vector_posture(pool).await?;

        let mut posture = MigrationPosture {
            kv_store_dropped,
            engine_installed,
            engine_mode,
            serving_fence_on,
            families,
            residue: residue_report,
            cutover_phase: CutoverPhase::InProgress,
            vector,
        };
        posture.cutover_phase = rules::derive_cutover_phase(&posture);
        Ok(posture)
    }
}

/// Derive the migration posture (convenience wrapper over the Postgres adapter).
pub async fn posture(pool: &PgPool) -> Result<MigrationPosture, StorageError> {
    PostgresPostureSource::new(pool.clone()).collect().await
}

/// Fact #3 — read one family's mode from its owning env flag.
fn family_mode(spec: &FamilySpec) -> FamilyMode {
    if spec.is_chunk {
        match chunk_text_authority_from_env() {
            ChunkTextAuthority::Kv => FamilyMode::Kv,
            ChunkTextAuthority::Dual => FamilyMode::Dual,
            ChunkTextAuthority::Relational => FamilyMode::Relational,
        }
    } else {
        match kv_family_mode_from_env(spec.name) {
            KvFamilyMode::Kv => FamilyMode::Kv,
            KvFamilyMode::Relational => FamilyMode::Relational,
        }
    }
}

/// Migration version recorded as successfully applied (42P01-tolerant for very
/// old databases that predate the `_sqlx_migrations` bookkeeping).
async fn drop_migration_applied(pool: &PgPool, version: i64) -> Result<bool, StorageError> {
    match sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM _sqlx_migrations WHERE version = $1 AND success)",
    )
    .bind(version)
    .fetch_one(pool)
    .await
    {
        Ok(v) => Ok(v),
        Err(sqlx::Error::Database(db)) if db.code().as_deref() == Some("42P01") => Ok(false),
        Err(e) => Err(StorageError::Database(format!(
            "advisor drop_migration_applied({version}) failed: {e}"
        ))),
    }
}

/// Is the migration engine ledger (migration 106) installed? `to_regclass`
/// returns NULL rather than erroring, so this is fail-safe.
async fn engine_installed(pool: &PgPool) -> Result<bool, StorageError> {
    let reg: Option<String> =
        sqlx::query_scalar("SELECT to_regclass('edgequake.edgequake_migration_job')::text")
            .fetch_one(pool)
            .await
            .map_err(|e| StorageError::Database(format!("advisor engine_installed failed: {e}")))?;
    Ok(reg.is_some())
}

/// Latest chunk backfill job (fact #1), 42P01-tolerant when the ledger is absent.
async fn chunk_backfill_job(pool: &PgPool) -> Result<Option<JobSnapshot>, StorageError> {
    let job_id: Option<Uuid> = match sqlx::query_scalar::<_, Uuid>(
        "SELECT job_id FROM edgequake.edgequake_migration_job \
         WHERE step_id = $1 ORDER BY schema_generation DESC LIMIT 1",
    )
    .bind(CHUNK_BACKFILL_STEP)
    .fetch_optional(pool)
    .await
    {
        Ok(v) => v,
        Err(sqlx::Error::Database(db)) if db.code().as_deref() == Some("42P01") => None,
        Err(e) => {
            return Err(StorageError::Database(format!(
                "advisor chunk_backfill_job failed: {e}"
            )))
        }
    };
    let Some(id) = job_id else { return Ok(None) };
    let detail = lease::job_detail(pool, id).await?;
    Ok(detail.map(snapshot_from_detail))
}

fn snapshot_from_detail(d: lease::MigrationJobDetail) -> JobSnapshot {
    JobSnapshot {
        step_id: d.step_id,
        job_id: Some(d.job_id),
        state: d.state,
        completion_pct: d.completion_pct,
        processed_count: d.processed_count,
        estimated_total: d.estimated_total,
        rows_per_sec: d.rows_per_sec,
        eta_seconds: d.eta_seconds,
        throttle_reason: d.throttle_reason,
        last_error: d.last_error.map(|v| v.to_string()),
    }
}

/// Fact #4 — chunk content verify (read-only), combined across every remaining
/// KV table. Returns `None` on a hard error (fail-closed: the family then never
/// reports ReadyToFlip). A dropped store verifies clean by definition.
async fn chunk_verify(pool: &PgPool, kv_tables: &[String]) -> Option<VerifySummary> {
    if kv_tables.is_empty() {
        return Some(VerifySummary {
            expected: 0,
            actual: 0,
            sampled: 0,
            mismatches: 0,
        });
    }
    let mut agg = VerifySummary {
        expected: 0,
        actual: 0,
        sampled: 0,
        mismatches: 0,
    };
    for table in kv_tables {
        match verify::verify_chunk_text_backfill(pool, table).await {
            Ok(r) => {
                agg.expected += r.expected;
                agg.actual += r.actual;
                agg.sampled += r.sampled;
                agg.mismatches += r.mismatches;
            }
            Err(_) => return None,
        }
    }
    Some(agg)
}

/// Fact #6 — sum of `COUNT(*)` across a family's typed SSOT tables (42P01 → 0).
async fn count_typed(pool: &PgPool, tables: &[&str]) -> Result<i64, StorageError> {
    let mut total = 0;
    for table in tables {
        total += count_table(pool, table).await?;
    }
    Ok(total)
}

async fn count_table(pool: &PgPool, table: &str) -> Result<i64, StorageError> {
    // `table` comes from the const FAMILIES specs (trusted identifiers), so the
    // format! interpolation is safe.
    match sqlx::query_scalar::<_, i64>(&format!("SELECT COUNT(*) FROM public.{table}"))
        .fetch_one(pool)
        .await
    {
        Ok(n) => Ok(n),
        Err(sqlx::Error::Database(db)) if db.code().as_deref() == Some("42P01") => Ok(0),
        Err(e) => Err(StorageError::Database(format!(
            "advisor count_table({table}) failed: {e}"
        ))),
    }
}

const CHUNK_EMBEDDING_BACKFILL_STEP: &str = "w3-chunk-embedding-backfill";
const FLEET_EMBEDDING_BACKFILL_STEP: &str = "iw2-fleet-embedding-backfill";

/// SPEC-091 W3/IW2 — derive the VECTOR posture.
async fn vector_posture(pool: &PgPool) -> Result<VectorPosture, StorageError> {
    let backend = crate::vector_backend_from_env().as_str().to_string();
    let typed_rows = count_table(pool, "chunk_embeddings").await?;
    let typed_entity_rows = count_table(pool, "entity_embeddings").await?;
    let typed_relationship_rows = count_table(pool, "relationship_embeddings").await?;
    let typed_report_rows = count_table(pool, "report_embeddings").await?;
    let legacy_chunk_rows = count_legacy_chunk_rows(pool).await?;
    let legacy_fleet_rows = count_legacy_fleet_rows(pool).await?;
    let uncovered_chunk_split =
        crate::migration_engine::coverage::count_uncovered_chunk_split(pool).await?;
    let uncovered_chunk_rows = uncovered_chunk_split.total();
    let uncovered_fleet_rows =
        crate::migration_engine::coverage::count_uncovered_fleet_rows(pool).await?;
    let chunk_fleet_dropped = drop_migration_applied(pool, 126).await?;
    let dropped = legacy_vectors_dropped(pool).await?;
    let backfill = vector_backfill_job(pool, CHUNK_EMBEDDING_BACKFILL_STEP).await?;
    let fleet_backfill = vector_backfill_job(pool, FLEET_EMBEDDING_BACKFILL_STEP).await?;
    let verify_chunk = vector_verify_chunk(pool).await;
    let verify_fleet = vector_verify_fleet(pool).await;
    let verify = merge_verify_summaries(verify_chunk, verify_fleet);
    let provenance_stall_rows =
        crate::migration_engine::coverage::count_provenance_stall_rows(pool).await?;

    Ok(VectorPosture {
        backend,
        backfill,
        fleet_backfill,
        verify_chunk,
        verify_fleet,
        verify,
        provenance_stall_rows,
        typed_rows,
        typed_entity_rows,
        typed_relationship_rows,
        typed_report_rows,
        legacy_chunk_rows,
        legacy_fleet_rows,
        uncovered_chunk_rows,
        uncovered_chunk_missing_spine_rows: uncovered_chunk_split.missing_spine,
        uncovered_chunk_missing_embedding_rows: uncovered_chunk_split.missing_embedding,
        uncovered_fleet_rows,
        chunk_fleet_dropped,
        dropped,
    })
}

fn merge_verify_summaries(
    chunk: Option<VerifySummary>,
    fleet: Option<VerifySummary>,
) -> Option<VerifySummary> {
    match (chunk, fleet) {
        (None, None) => None,
        (Some(a), None) | (None, Some(a)) => Some(a),
        (Some(a), Some(b)) => Some(VerifySummary {
            expected: a.expected + b.expected,
            actual: a.actual + b.actual,
            sampled: a.sampled + b.sampled,
            mismatches: a.mismatches + b.mismatches,
        }),
    }
}

async fn count_legacy_fleet_rows(pool: &PgPool) -> Result<i64, StorageError> {
    let tables = crate::migration_engine::coverage::list_vector_tables(pool).await?;
    let mut total = 0i64;
    for t in tables {
        let n = match sqlx::query_scalar::<_, i64>(&format!(
            "SELECT COUNT(*) FROM public.{t} WHERE id LIKE 'entity:%' \
             OR id LIKE 'community_report:%' \
             OR (id LIKE '%->%:%' AND id NOT LIKE 'entity:%' AND id NOT LIKE 'community_report:%')"
        ))
        .fetch_one(pool)
        .await
        {
            Ok(n) => n,
            Err(sqlx::Error::Database(db)) if db.code().as_deref() == Some("42P01") => 0,
            Err(e) => {
                return Err(StorageError::Database(format!(
                    "advisor count legacy fleet failed: {e}"
                )))
            }
        };
        total += n;
    }
    Ok(total)
}

/// Sum chunk-row counts across every remaining `eq_*_vectors` relation.
async fn count_legacy_chunk_rows(pool: &PgPool) -> Result<i64, StorageError> {
    let tables = crate::migration_engine::coverage::list_vector_tables(pool).await?;
    let mut total = 0;
    for t in tables {
        let n = match sqlx::query_scalar::<_, i64>(&format!(
            "SELECT COUNT(*) FROM public.{t} WHERE id ~ '{re}'",
            re = crate::migration_engine::coverage::LEGACY_CHUNK_VECTOR_ID_RE
        ))
        .fetch_one(pool)
        .await
        {
            Ok(n) => n,
            Err(sqlx::Error::Database(db)) if db.code().as_deref() == Some("42P01") => 0,
            Err(e) => {
                return Err(StorageError::Database(format!(
                    "advisor count legacy chunks failed: {e}"
                )))
            }
        };
        total += n;
    }
    Ok(total)
}

/// Whether every legacy `eq_*_vectors` relation is gone.
async fn legacy_vectors_dropped(pool: &PgPool) -> Result<bool, StorageError> {
    let n: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM information_schema.tables \
         WHERE table_schema = 'public' AND table_name LIKE 'eq\\_%\\_vectors'",
    )
    .fetch_one(pool)
    .await
    .map_err(|e| StorageError::Database(format!("advisor vectors dropped check failed: {e}")))?;
    Ok(n == 0)
}

/// Latest vector backfill job (42P01-tolerant when ledger absent).
async fn vector_backfill_job(
    pool: &PgPool,
    step_id: &str,
) -> Result<Option<JobSnapshot>, StorageError> {
    let job_id: Option<Uuid> = match sqlx::query_scalar::<_, Uuid>(
        "SELECT job_id FROM edgequake.edgequake_migration_job \
         WHERE step_id = $1 ORDER BY schema_generation DESC LIMIT 1",
    )
    .bind(step_id)
    .fetch_optional(pool)
    .await
    {
        Ok(v) => v,
        Err(sqlx::Error::Database(db)) if db.code().as_deref() == Some("42P01") => None,
        Err(e) => {
            return Err(StorageError::Database(format!(
                "advisor vector_backfill_job failed: {e}"
            )))
        }
    };
    let Some(id) = job_id else { return Ok(None) };
    let detail = lease::job_detail(pool, id).await?;
    Ok(detail.map(snapshot_from_detail))
}

/// Chunk-embedding verify across remaining vector tables (migration 126).
async fn vector_verify_chunk(pool: &PgPool) -> Option<VerifySummary> {
    let tables = crate::migration_engine::coverage::list_vector_tables(pool)
        .await
        .ok()?;
    let model = std::env::var("EDGEQUAKE_EMBEDDING_MODEL")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| "text-embedding-3-small".to_string());
    if tables.is_empty() {
        return Some(VerifySummary {
            expected: 0,
            actual: 0,
            sampled: 0,
            mismatches: 0,
        });
    }
    let mut agg = VerifySummary {
        expected: 0,
        actual: 0,
        sampled: 0,
        mismatches: 0,
    };
    for table in tables {
        match verify::verify_chunk_embedding_backfill(pool, &table, &model).await {
            Ok(r) => {
                agg.expected += r.expected;
                agg.actual += r.actual;
                agg.sampled += r.sampled;
                agg.mismatches += r.mismatches;
            }
            Err(_) => return None,
        }
    }
    Some(agg)
}

/// Fleet-embedding verify across remaining vector tables (migration 131).
async fn vector_verify_fleet(pool: &PgPool) -> Option<VerifySummary> {
    let tables = crate::migration_engine::coverage::list_vector_tables(pool)
        .await
        .ok()?;
    let model = std::env::var("EDGEQUAKE_EMBEDDING_MODEL")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| "text-embedding-3-small".to_string());
    if tables.is_empty() {
        return Some(VerifySummary {
            expected: 0,
            actual: 0,
            sampled: 0,
            mismatches: 0,
        });
    }
    let mut agg = VerifySummary {
        expected: 0,
        actual: 0,
        sampled: 0,
        mismatches: 0,
    };
    for table in tables {
        for family in crate::embedding_family::EmbeddingFamily::FLEET_BACKFILL_FAMILIES {
            match verify::verify_fleet_embedding_backfill(pool, &table, family, &model).await {
                Ok(r) => {
                    agg.expected += r.expected;
                    agg.actual += r.actual;
                    agg.sampled += r.sampled;
                    agg.mismatches += r.mismatches;
                }
                Err(_) => return None,
            }
        }
    }
    Some(agg)
}
