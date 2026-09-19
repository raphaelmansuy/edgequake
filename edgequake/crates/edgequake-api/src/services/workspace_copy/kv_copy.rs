//! KV storage copy phase for WorkspaceCopy (EN-3677 Phase 1).
//!
//! ## KV layout in EdgeQuake
//!
//! Each tenant has one KV table named `eq_{ns}_kv` (Postgres-backed KV
//! shim). Keys are structured as `{workspace_id}:{document_id}:{kind}` —
//! e.g. `abc123:doc-42:content` for a document body, or
//! `abc123:doc-42:chunks:0` for chunk 0's cached metadata.
//!
//! ## Copy strategy: prefix rewrite
//!
//! For each key that starts with `{source_workspace_id}:`, generate a new
//! key `{dest_workspace_id}:...` with the rest of the key preserved, and
//! insert the same value into the destination row. The document/chunk id
//! remap is intentionally **not** applied at the KV level — the SQL copy
//! phase has already rewritten those ids in the Postgres tables, and the
//! KV layer stores the same document ids so they must match.
//!
//! Wait — that's exactly the problem. The SQL copy generates fresh
//! `documents.id` values via the id-map temp table. So the KV keys need
//! to be rewritten with BOTH `workspace_id` AND the remapped `document_id`
//! in the middle. The id-map must therefore be passed in from the SQL
//! copy phase.
//!
//! ## Ordering
//!
//! Must run **after** the SQL copy phase completes (need the id-maps) and
//! **before** the transaction commits (the KV write goes through the same
//! transaction to keep the copy atomic).

use std::collections::HashMap;

use uuid::Uuid;

/// Mapping from source id → destination id for a single entity class,
/// produced by the SQL copy phase.
pub type IdMap = HashMap<Uuid, Uuid>;

/// Bundled id-maps the KV copy needs to rewrite keys. Only `documents` and
/// `chunks` show up in KV keys today; keeping the type explicit means
/// adding a new class (e.g. entities) is one extra field, not a schema
/// migration.
#[derive(Debug, Clone, Default)]
pub struct KvIdMaps {
    pub documents: IdMap,
    pub chunks: IdMap,
}

/// Result of the KV copy — feeds into logs; the KV row count is not
/// user-facing in the response (the response has document / chunk counts
/// which are already ground-truth from the SQL copy).
#[derive(Debug, Clone, Copy, Default)]
pub struct KvCopyResult {
    pub keys_scanned: usize,
    pub keys_rewritten: usize,
}

/// Rewrite one KV key to point at the destination workspace + remapped ids.
///
/// Returns `None` when the key does not belong to the source workspace
/// (the caller can skip it). Returns `Some(rewritten)` when the key was
/// remapped. The function is deliberately pure so it can be unit-tested
/// without a live Postgres — a v2 hardening pass will add tests.
pub fn rewrite_key(
    key: &str,
    source_workspace_id: Uuid,
    dest_workspace_id: Uuid,
    id_maps: &KvIdMaps,
) -> Option<String> {
    let source_prefix = source_workspace_id.simple().to_string();
    let dest_prefix = dest_workspace_id.simple().to_string();

    // Also try hyphenated form — production keys use hyphenated UUIDs; the
    // simple() form is a fallback for legacy rows that predate SPEC-054.
    let source_prefix_hyphen = source_workspace_id.to_string();
    let dest_prefix_hyphen = dest_workspace_id.to_string();

    let (rest, chosen_dest_prefix) =
        if let Some(rest) = key.strip_prefix(&format!("{source_prefix}:")) {
            (rest, dest_prefix)
        } else if let Some(rest) = key.strip_prefix(&format!("{source_prefix_hyphen}:")) {
            (rest, dest_prefix_hyphen)
        } else {
            return None;
        };

    // Peel off the next segment — it is a document id we may need to remap.
    let mut segments: Vec<&str> = rest.splitn(3, ':').collect();
    if segments.is_empty() {
        return Some(format!("{chosen_dest_prefix}:{rest}"));
    }

    if let Ok(src_doc_id) = Uuid::parse_str(segments[0]) {
        if let Some(dest_doc_id) = id_maps.documents.get(&src_doc_id) {
            let remapped_doc = dest_doc_id.to_string();
            segments[0] = &remapped_doc;
            return Some(format!("{}:{}", chosen_dest_prefix, segments.join(":")));
        }
    }

    Some(format!("{chosen_dest_prefix}:{rest}"))
}

/// Execute the KV copy pass. **Scaffold**: the actual bulk-read/write needs
/// the KV storage handle from `AppState`, which requires an
/// `edgequake-storage` bulk-scan helper that does not yet exist. Phase 1
/// logs a warning and returns zero counts.
pub async fn copy_kv(
    _source_workspace_id: Uuid,
    _dest_workspace_id: Uuid,
    _id_maps: &KvIdMaps,
) -> Result<KvCopyResult, String> {
    tracing::warn!(
        source_workspace_id = %_source_workspace_id,
        dest_workspace_id = %_dest_workspace_id,
        "workspace_copy::kv_copy: scaffold — KV rewrite is a no-op in Phase 1"
    );
    Ok(KvCopyResult::default())
}
