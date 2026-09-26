//! SPEC-091 W4 serving fence — SQL pushdown for vector search results.
//!
//! Fail-closed: when the serving fence is enabled (default on; unset counts as
//! on), chunk vectors are visible only with `chunk_serving_state.state = 'ready'`.
//! Non-chunk ids (entity / relationship vectors) always pass through.
//!
//! The state lookup resolves chunk ids via the `UNIQUE (document_id,
//! chunk_index)` btree — one indexed round trip per search, only when on.

use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicU64, Ordering};

use async_trait::async_trait;
use edgequake_storage_contracts::{
    AccessError, AccessResult, AccessScope, VisibilityKey, VisibilityLifecycle,
    VisibilityRepository, VisibilityState,
};
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::StorageError;
use crate::kv_key_schema::kv_keys;
use crate::serving_fence::serving_fence_enabled_from_env;
use crate::traits::VectorSearchResult;

static FENCE_FILTERED_TOTAL: AtomicU64 = AtomicU64::new(0);
static FENCE_OPENED_TOTAL: AtomicU64 = AtomicU64::new(0);

#[derive(Clone)]
pub struct PgVisibilityRepository {
    pool: PgPool,
}

impl PgVisibilityRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct VisibilityRow {
    ordinal: i64,
    object_kind: String,
    object_id: Uuid,
    object_revision: i64,
    lifecycle: Option<String>,
    is_current: bool,
    required_bindings: i64,
    completed_bindings: i64,
}

#[async_trait]
impl VisibilityRepository for PgVisibilityRepository {
    async fn batch_current(
        &self,
        scope: &AccessScope,
        keys: &[VisibilityKey],
    ) -> AccessResult<Vec<Option<VisibilityState>>> {
        if keys.is_empty() {
            return Ok(Vec::new());
        }
        let kinds: Vec<&str> = keys.iter().map(|key| key.object_kind.as_str()).collect();
        let ids: Vec<Uuid> = keys.iter().map(|key| key.object_id).collect();
        let revisions: Vec<i64> = keys
            .iter()
            .map(|key| {
                i64::try_from(key.object_revision).map_err(|_| {
                    AccessError::InvalidInput("visibility revision exceeds i64".into())
                })
            })
            .collect::<AccessResult<_>>()?;

        let rows = sqlx::query_as::<_, VisibilityRow>(VISIBILITY_BATCH_SQL)
            .bind(scope.tenant().into_uuid())
            .bind(scope.workspace().into_uuid())
            .bind(&kinds)
            .bind(&ids)
            .bind(&revisions)
            .fetch_all(&self.pool)
            .await
            .map_err(|error| AccessError::from(StorageError::from(error)))?;

        let mut output = vec![None; keys.len()];
        for row in rows {
            let Some(lifecycle) = row.lifecycle.as_deref().map(parse_lifecycle).transpose()? else {
                continue;
            };
            let index = usize::try_from(row.ordinal - 1)
                .map_err(|_| AccessError::CorruptData("invalid visibility ordinal".into()))?;
            let required_bindings = u32::try_from(row.required_bindings)
                .map_err(|_| AccessError::CorruptData("invalid required binding count".into()))?;
            let completed_bindings = u32::try_from(row.completed_bindings)
                .map_err(|_| AccessError::CorruptData("invalid completed binding count".into()))?;
            let serving = lifecycle == VisibilityLifecycle::Active
                && row.is_current
                && required_bindings > 0
                && completed_bindings == required_bindings;
            let state = VisibilityState {
                key: VisibilityKey {
                    object_kind: row.object_kind,
                    object_id: row.object_id,
                    object_revision: u64::try_from(row.object_revision).map_err(|_| {
                        AccessError::CorruptData("negative visibility revision".into())
                    })?,
                },
                lifecycle,
                is_current: row.is_current,
                serving,
                required_bindings,
                completed_bindings,
            };
            if let Some(slot) = output.get_mut(index) {
                *slot = Some(state);
            } else {
                return Err(AccessError::CorruptData(
                    "visibility result ordinal exceeds request".into(),
                ));
            }
        }
        Ok(output)
    }
}

fn parse_lifecycle(value: &str) -> AccessResult<VisibilityLifecycle> {
    match value {
        "staged" => Ok(VisibilityLifecycle::Staged),
        "active" => Ok(VisibilityLifecycle::Active),
        "tombstoned" => Ok(VisibilityLifecycle::Tombstoned),
        unknown => Err(AccessError::CorruptData(format!(
            "unknown object lifecycle '{unknown}'"
        ))),
    }
}

const VISIBILITY_BATCH_SQL: &str = r#"
WITH requested AS (
    SELECT object_kind, object_id, object_revision, ordinality::bigint AS ordinal
    FROM unnest($3::text[], $4::uuid[], $5::bigint[])
         WITH ORDINALITY AS r(object_kind, object_id, object_revision, ordinality)
)
SELECT r.ordinal, r.object_kind, r.object_id, r.object_revision,
       o.state AS lifecycle,
       COALESCE(o.revision = (
           SELECT max(current.revision)
           FROM public.object_revisions current
           WHERE current.tenant_id = $1 AND current.workspace_id = $2
             AND current.kind = r.object_kind AND current.logical_id = r.object_id
       ), false) AS is_current,
       (SELECT count(*) FROM public.data_bindings b
        WHERE b.tenant_id = $1 AND b.workspace_id = $2 AND b.state = 'active')
           AS required_bindings,
       (SELECT count(*) FROM public.projection_visibility v
        JOIN public.data_bindings b ON b.binding_id = v.binding_id AND b.state = 'active'
        WHERE v.tenant_id = $1 AND v.workspace_id = $2
          AND v.object_kind = r.object_kind AND v.object_id = r.object_id
          AND v.object_revision = r.object_revision)
           AS completed_bindings
FROM requested r
LEFT JOIN public.object_revisions o
  ON o.tenant_id = $1 AND o.workspace_id = $2
 AND o.kind = r.object_kind AND o.logical_id = r.object_id
 AND o.revision = r.object_revision
ORDER BY r.ordinal
"#;

/// Results hidden by the serving fence since process start (SRE signal).
pub fn serving_fence_filtered_total() -> u64 {
    FENCE_FILTERED_TOTAL.load(Ordering::Relaxed)
}

/// Chunks marked `ready` by the single serving-fence writer since process start.
pub fn serving_fence_opened_total() -> u64 {
    FENCE_OPENED_TOTAL.load(Ordering::Relaxed)
}

/// Record rows touched when opening the fence (call only from the storage writer).
pub(crate) fn record_serving_fence_opened(rows: u64) {
    if rows > 0 {
        FENCE_OPENED_TOTAL.fetch_add(rows, Ordering::Relaxed);
    }
}

/// Post-filter `results` by serving readiness. No-op when the fence is off.
pub async fn apply_serving_fence(
    pool: &PgPool,
    results: Vec<VectorSearchResult>,
) -> Result<Vec<VectorSearchResult>, StorageError> {
    if results.is_empty() {
        return Ok(results);
    }
    if !serving_fence_enabled_from_env() {
        return apply_authoritative_vector_visibility(pool, results).await;
    }

    let mut parseable: Vec<(Uuid, i32)> = Vec::new();
    for result in &results {
        if let Some((doc_str, index)) = kv_keys::parse_doc_chunk(&result.id) {
            if let Ok(doc_uuid) = Uuid::parse_str(doc_str) {
                parseable.push((doc_uuid, index as i32));
            }
        }
    }
    if parseable.is_empty() {
        return apply_authoritative_vector_visibility(pool, results).await;
    }

    let docs: Vec<Uuid> = parseable.iter().map(|p| p.0).collect();
    let idxs: Vec<i32> = parseable.iter().map(|p| p.1).collect();
    let ready_rows = sqlx::query_as::<_, (Uuid, i32)>(
        "SELECT c.document_id, c.chunk_index \
         FROM chunks c \
         JOIN public.chunk_serving_state s \
           ON s.chunk_id = c.id AND s.state = 'ready' \
         WHERE (c.document_id, c.chunk_index) IN (SELECT * FROM unnest($1::uuid[], $2::int[]))",
    )
    .bind(&docs)
    .bind(&idxs)
    .fetch_all(pool)
    .await
    .map_err(|e| StorageError::Database(format!("serving fence state query failed: {e}")))?;

    let ready: HashSet<(Uuid, i32)> = ready_rows.into_iter().collect();
    let before = results.len();
    let filtered: Vec<VectorSearchResult> = results
        .into_iter()
        .filter(|result| {
            let Some((doc_str, index)) = kv_keys::parse_doc_chunk(&result.id) else {
                return true; // non-chunk vector — visible
            };
            match Uuid::parse_str(doc_str) {
                // Relational chunks always have UUID document ids: ready-only.
                Ok(doc_uuid) => ready.contains(&(doc_uuid, index as i32)),
                // Non-UUID ids cannot reference `chunks` (FK is uuid) → outside
                // the fence domain (e.g. entity vectors named "x-chunk-1").
                Err(_) => true,
            }
        })
        .collect();

    let hidden = before - filtered.len();
    if hidden > 0 {
        FENCE_FILTERED_TOTAL.fetch_add(hidden as u64, Ordering::Relaxed);
        tracing::debug!(
            hidden,
            kept = filtered.len(),
            "SPEC-091 serving fence hid non-ready chunks"
        );
    }
    apply_authoritative_vector_visibility(pool, filtered).await
}

async fn apply_authoritative_vector_visibility(
    pool: &PgPool,
    results: Vec<VectorSearchResult>,
) -> Result<Vec<VectorSearchResult>, StorageError> {
    type Scope = (Uuid, Uuid);
    let mut groups: HashMap<Scope, Vec<(usize, Vec<VisibilityKey>)>> = HashMap::new();
    let mut keep = vec![true; results.len()];
    for (index, result) in results.iter().enumerate() {
        match visibility_keys_from_metadata(&result.metadata) {
            MetadataVisibility::Legacy => {}
            MetadataVisibility::Invalid => keep[index] = false,
            MetadataVisibility::Scoped {
                tenant,
                workspace,
                keys,
            } => groups
                .entry((tenant, workspace))
                .or_default()
                .push((index, keys)),
        }
    }

    let repository = PgVisibilityRepository::new(pool.clone());
    for ((tenant, workspace), items) in groups {
        let keys: Vec<VisibilityKey> = items
            .iter()
            .flat_map(|(_, keys)| keys.iter().cloned())
            .collect();
        let states = repository
            .batch_current(
                &AccessScope::new(
                    edgequake_storage_contracts::TenantId::new(tenant),
                    edgequake_storage_contracts::WorkspaceId::new(workspace),
                ),
                &keys,
            )
            .await
            .map_err(StorageError::from)?;
        let mut offset = 0;
        for (index, item_keys) in items {
            let end = offset + item_keys.len();
            keep[index] = edgequake_storage_contracts::all_contributors_serving(
                states.get(offset..end).unwrap_or_default(),
            );
            offset = end;
        }
    }

    Ok(results
        .into_iter()
        .zip(keep)
        .filter_map(|(result, keep)| keep.then_some(result))
        .collect())
}

enum MetadataVisibility {
    Legacy,
    Invalid,
    Scoped {
        tenant: Uuid,
        workspace: Uuid,
        keys: Vec<VisibilityKey>,
    },
}

fn visibility_keys_from_metadata(metadata: &serde_json::Value) -> MetadataVisibility {
    let Some(object_kind) = metadata.get("object_kind") else {
        return MetadataVisibility::Legacy;
    };
    let parsed = (|| {
        let tenant = Uuid::parse_str(metadata.get("tenant_id")?.as_str()?).ok()?;
        let workspace = Uuid::parse_str(metadata.get("workspace_id")?.as_str()?).ok()?;
        let object_id = Uuid::parse_str(metadata.get("object_id")?.as_str()?).ok()?;
        let revision = metadata.get("object_revision")?.as_u64()?;
        let mut keys = vec![VisibilityKey {
            object_kind: object_kind.as_str()?.to_string(),
            object_id,
            object_revision: revision,
        }];
        if let Some(contributors) = metadata.get("contributing_documents") {
            for contributor in contributors.as_array()? {
                keys.push(VisibilityKey {
                    object_kind: "document".into(),
                    object_id: Uuid::parse_str(contributor.get("id")?.as_str()?).ok()?,
                    object_revision: contributor.get("revision")?.as_u64()?,
                });
            }
        }
        Some((tenant, workspace, keys))
    })();
    match parsed {
        Some((tenant, workspace, keys)) => MetadataVisibility::Scoped {
            tenant,
            workspace,
            keys,
        },
        None => MetadataVisibility::Invalid,
    }
}

#[cfg(test)]
mod tests {
    use super::VISIBILITY_BATCH_SQL;

    #[test]
    fn contract_spec091_fence_uses_ready_join_not_seqscan() {
        // Regression guard: fence must resolve via chunks UNIQUE(document_id,
        // chunk_index) + serving-state PK — never a metadata->>'key' scan.
        // (Banned literal built at runtime so this test file stays clean.)
        let src = include_str!("serving_fence_query.rs");
        assert!(src.contains("s.state = 'ready'"));
        assert!(src.contains("unnest($1::uuid[], $2::int[])"));
        let banned = format!("metadata->>{}", "'legacy_chunk_key'");
        assert!(!src.contains(&banned));
        // Regression guard (realized 2026-07-29): the fence must join the SSOT
        // table `public.chunk_serving_state` — never the `edgequake` compat
        // schema, which has no chunk_serving_state view (write path uses public).
        // Built at runtime so the literal does not self-match via include_str!.
        assert!(src.contains("public.chunk_serving_state"));
        let wrong_schema = format!("{}.{}", "edgequake", "chunk_serving_state");
        assert!(!src.contains(&wrong_schema));
    }

    #[test]
    fn authoritative_visibility_is_positional_and_fail_closed() {
        assert!(VISIBILITY_BATCH_SQL.contains("WITH ORDINALITY"));
        assert!(VISIBILITY_BATCH_SQL.contains("public.projection_visibility"));
        assert!(VISIBILITY_BATCH_SQL.contains("b.state = 'active'"));
    }
}
