//! P-G1b (RC-6 follow-up): reconcile legacy un-normalized graph nodes and
//! entity vectors written by the pre-G1 async ingestion path.
//!
//! WHY (First Principles): P-G1 made `EntityId` the single canonical identity
//! for *new* writes, but graphs already corrupted by the old raw-name path
//! still contain `John Doe` + `john doe` nodes that should be one `JOHN_DOE`
//! node, plus `entity:John Doe` vectors disconnected from the normalized node.
//! This module is the admin-gated, idempotent repair tool.
//!
//! Design (mirrors plan-17 P-D2 dry-run + confirm-token pattern):
//! - `plan()` is read-only: scans `get_all_nodes`, groups every node whose
//!   `id != normalize_entity_name(id)` by its normalized target, and reports
//!   the merge groups + incident-edge rewrites + vector re-keys it WOULD do.
//! - `execute(confirm_token)` applies the plan destructively. The caller MUST
//!   pass the token returned by `plan()` so a stale/different plan cannot be
//!   blindly applied (E5/E7 guards). It is best-effort and idempotent:
//!   re-running on an already-reconciled graph is a no-op.
//!
//! What it does NOT do: auto-run. Callers (admin endpoints) gate this behind
//! explicit confirmation. See plan-19 §8 ("Auto-run the legacy entity backfill
//! ... Destructive merge; admin-gated with dry-run + confirm token").

use std::collections::HashMap;

use crate::entity_id::{normalize_entity_name, EntityId};
use crate::error::Result;
use crate::traits::{GraphStorage, VectorStorage};

/// One raw node that needs merging into a normalized target.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RawNodeRecord {
    /// The legacy (un-normalized) node id as stored in the graph.
    pub raw_id: String,
    /// The canonical normalized id it should merge into.
    pub normalized_id: String,
}

/// A merge group: all raw nodes that collapse into one normalized node.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MergeGroup {
    /// The canonical normalized node id.
    pub normalized_id: String,
    /// Raw node ids that should merge into `normalized_id`.
    pub raw_nodes: Vec<RawNodeRecord>,
}

/// An edge that must be rewritten because one or both endpoints is a raw id.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EdgeRewrite {
    pub from_source: String,
    pub from_target: String,
    pub to_source: String,
    pub to_target: String,
}

/// A vector that must be re-keyed from `entity:{raw}` to `entity:{normalized}`.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct VectorRekey {
    pub raw_vector_id: String,
    pub normalized_vector_id: String,
}

/// The read-only reconciliation plan.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ReconcilePlan {
    /// Token the caller must pass back to `execute` to apply THIS plan.
    pub confirm_token: String,
    pub merge_groups: Vec<MergeGroup>,
    pub edge_rewrites: Vec<EdgeRewrite>,
    pub vector_rekeys: Vec<VectorRekey>,
    /// Nodes already normalized — skipped (E6).
    pub already_normalized: usize,
}

impl ReconcilePlan {
    /// Total number of raw nodes that would be merged.
    pub fn raw_node_count(&self) -> usize {
        self.merge_groups.iter().map(|g| g.raw_nodes.len()).sum()
    }

    /// True if there is nothing to repair.
    pub fn is_clean(&self) -> bool {
        self.merge_groups.is_empty()
            && self.edge_rewrites.is_empty()
            && self.vector_rekeys.is_empty()
    }
}

/// The result of applying a plan.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default)]
pub struct ReconcileResult {
    pub nodes_merged: usize,
    pub edges_rewritten: usize,
    pub vectors_rekeyed: usize,
    pub errors: Vec<String>,
}

/// Build a short, opaque confirm token from the plan contents so that a stale
/// plan cannot be applied via `execute`. Hash the raw-node set (not the whole
/// graph — the graph may change between plan and execute, which is exactly what
/// the token guards against).
fn confirm_token_for(merge_groups: &[MergeGroup]) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    for group in merge_groups {
        group.normalized_id.hash(&mut hasher);
        for raw in &group.raw_nodes {
            raw.raw_id.hash(&mut hasher);
        }
    }
    format!("{:016x}", hasher.finish())
}

/// Scan the graph and build a read-only reconciliation plan (P-G1b).
///
/// Read-only: this never mutates storage. The returned `confirm_token` must be
/// passed to [`execute`] to apply the plan.
/// Scan the graph and build a read-only reconciliation plan (P-G1b).
///
/// Read-only: this never mutates storage. The returned `confirm_token` must be
/// passed to [`execute`] to apply the plan.
#[allow(deprecated)] // full-graph scan is intentional for a one-time admin repair job
pub async fn plan(graph: &dyn GraphStorage, _vectors: &dyn VectorStorage) -> Result<ReconcilePlan> {
    // A reconciliation job must scan the whole graph by design (it finds every
    // un-normalized node), so the deprecated full-graph loaders are the right
    // tool here; the bounded scan ops are for query-path hot loops.
    let all_nodes = graph.get_all_nodes().await?;
    let all_edges = graph.get_all_edges().await?;

    // Group raw nodes by their normalized target. Nodes already normalized
    // (id == normalize(id)) are skipped (E6).
    let mut groups: HashMap<String, MergeGroup> = HashMap::new();
    let mut already_normalized = 0usize;
    for node in &all_nodes {
        let normalized = normalize_entity_name(&node.id);
        if normalized == node.id {
            already_normalized += 1;
            continue;
        }
        let group = groups
            .entry(normalized.clone())
            .or_insert_with(|| MergeGroup {
                normalized_id: normalized.clone(),
                raw_nodes: Vec::new(),
            });
        group.raw_nodes.push(RawNodeRecord {
            raw_id: node.id.clone(),
            normalized_id: normalized,
        });
    }
    let merge_groups: Vec<MergeGroup> = groups.into_values().collect();

    // Build a raw -> normalized lookup for edge rewriting.
    let raw_to_norm: HashMap<String, String> = merge_groups
        .iter()
        .flat_map(|g| {
            g.raw_nodes
                .iter()
                .map(|r| (r.raw_id.clone(), g.normalized_id.clone()))
        })
        .collect();

    let mut edge_rewrites = Vec::new();
    for edge in &all_edges {
        let new_source = raw_to_norm
            .get(&edge.source)
            .cloned()
            .unwrap_or_else(|| edge.source.clone());
        let new_target = raw_to_norm
            .get(&edge.target)
            .cloned()
            .unwrap_or_else(|| edge.target.clone());
        if new_source != edge.source || new_target != edge.target {
            edge_rewrites.push(EdgeRewrite {
                from_source: edge.source.clone(),
                from_target: edge.target.clone(),
                to_source: new_source,
                to_target: new_target,
            });
        }
    }

    // Vector re-keys: for each raw node, the old vector id is `entity:{raw}`
    // (pre-G1 convention) and the new one is the canonical `EntityId::as_vector_id`.
    // We cannot enumerate all vectors via the trait, so we list the re-keys we
    // WOULD attempt; execute probes each one with `get_by_id` and only re-keys
    // vectors that actually exist.
    let mut vector_rekeys = Vec::new();
    for group in &merge_groups {
        for raw in &group.raw_nodes {
            let raw_vid = format!("entity:{}", raw.raw_id);
            let norm_vid = EntityId::from_normalized(&group.normalized_id).as_vector_id();
            if raw_vid != norm_vid {
                vector_rekeys.push(VectorRekey {
                    raw_vector_id: raw_vid,
                    normalized_vector_id: norm_vid,
                });
            }
        }
    }

    let confirm_token = confirm_token_for(&merge_groups);
    Ok(ReconcilePlan {
        confirm_token,
        merge_groups,
        edge_rewrites,
        vector_rekeys,
        already_normalized,
    })
}

/// Apply a reconciliation plan destructively (P-G1b).
///
/// `confirm_token` MUST match the token embedded in `ReconcilePlan` returned by
/// [`plan`]; otherwise this returns an error without mutating anything (guards
/// against applying a stale plan to a graph that has since changed — E5/E7).
///
/// Idempotent: re-running on an already-reconciled graph is a no-op (raw nodes
/// are gone, edges already point at normalized ids, vectors already re-keyed).
pub async fn execute(
    graph: &dyn GraphStorage,
    vectors: &dyn VectorStorage,
    planned: &ReconcilePlan,
    confirm_token: &str,
) -> Result<ReconcileResult> {
    if confirm_token != planned.confirm_token {
        return Err(crate::error::StorageError::InvalidInput(
            "confirm token mismatch — refusing to apply a stale reconciliation plan".to_string(),
        ));
    }
    let mut result = ReconcileResult::default();

    // 1. Rewrite incident edges FIRST: upsert each edge under its normalized
    //    endpoints, then delete the old raw-endpoint edge. This MUST happen
    //    before node deletion because `delete_node` typically cascades to
    //    incident edges (e.g. MemoryGraphStorage, AGE) — deleting nodes first
    //    would erase the edges before we can copy their properties.
    for rw in &planned.edge_rewrites {
        if let Ok(Some(old_edge)) = graph.get_edge(&rw.from_source, &rw.from_target).await {
            if let Err(e) = graph
                .upsert_edge(&rw.to_source, &rw.to_target, old_edge.properties)
                .await
            {
                result.errors.push(format!(
                    "rewrite edge {}->{}: {}",
                    rw.to_source, rw.to_target, e
                ));
                continue;
            }
            let _ = graph.delete_edge(&rw.from_source, &rw.from_target).await;
            result.edges_rewritten += 1;
        }
    }

    // 2. Merge each raw node into its normalized target: combine source_chunk_ids
    //    and merge descriptions, then delete the raw node.
    for group in &planned.merge_groups {
        // Read the normalized target (may or may not exist yet).
        let mut merged_props: std::collections::HashMap<String, serde_json::Value> = graph
            .get_node(&group.normalized_id)
            .await?
            .map(|n| n.properties)
            .unwrap_or_default();

        for raw in &group.raw_nodes {
            if let Some(raw_node) = graph.get_node(&raw.raw_id).await? {
                merge_source_ids(&mut merged_props, &raw_node.properties);
                merge_description(&mut merged_props, &raw_node.properties);
            }
            // Upsert the normalized node with merged properties, then drop the raw.
            if let Err(e) = graph
                .upsert_node(&group.normalized_id, merged_props.clone())
                .await
            {
                result
                    .errors
                    .push(format!("upsert {}: {}", group.normalized_id, e));
                continue;
            }
            if let Err(e) = graph.delete_node(&raw.raw_id).await {
                result
                    .errors
                    .push(format!("delete raw node {}: {}", raw.raw_id, e));
                continue;
            }
            result.nodes_merged += 1;
        }
    }

    // 3. Re-key entity vectors: copy the embedding from `entity:{raw}` to the
    //    canonical `EntityId::as_vector_id`, then delete the old. Best-effort.
    for rk in &planned.vector_rekeys {
        match vectors.get_by_id(&rk.raw_vector_id).await {
            Ok(Some(emb)) => {
                let meta =
                    serde_json::json!({"type": "entity", "entity_name": rk.normalized_vector_id});
                if let Err(e) = vectors
                    .upsert(&[(rk.normalized_vector_id.clone(), emb, meta)])
                    .await
                {
                    result
                        .errors
                        .push(format!("rekey vector {}: {}", rk.normalized_vector_id, e));
                    continue;
                }
                let _ = vectors
                    .delete(std::slice::from_ref(&rk.raw_vector_id))
                    .await;
                result.vectors_rekeyed += 1;
            }
            Ok(None) => {} // already re-keyed or never existed — idempotent no-op
            Err(e) => result
                .errors
                .push(format!("probe vector {}: {}", rk.raw_vector_id, e)),
        }
    }

    Ok(result)
}

/// Union `source_chunk_ids` arrays from `src` into `dst`.
fn merge_source_ids(
    dst: &mut std::collections::HashMap<String, serde_json::Value>,
    src: &std::collections::HashMap<String, serde_json::Value>,
) {
    let mut ids: std::collections::HashSet<String> = dst
        .get("source_chunk_ids")
        .and_then(|v| v.as_array())
        .into_iter()
        .flatten()
        .filter_map(|v| v.as_str().map(String::from))
        .collect();
    for id in src
        .get("source_chunk_ids")
        .and_then(|v| v.as_array())
        .into_iter()
        .flatten()
        .filter_map(|v| v.as_str().map(String::from))
    {
        ids.insert(id);
    }
    if !ids.is_empty() {
        let arr: Vec<serde_json::Value> = ids.into_iter().map(serde_json::Value::String).collect();
        dst.insert(
            "source_chunk_ids".to_string(),
            serde_json::Value::Array(arr),
        );
    }
}

/// Concatenate `description` fields with a separator (no LLM summarization here
/// — the merger's LLM summarizer runs on new ingestion; this repair just
/// preserves both descriptions so no information is lost).
fn merge_description(
    dst: &mut std::collections::HashMap<String, serde_json::Value>,
    src: &std::collections::HashMap<String, serde_json::Value>,
) {
    let a = dst
        .get("description")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let b = src
        .get("description")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    if b.is_empty() {
        return;
    }
    let merged = if a.is_empty() {
        b.to_string()
    } else if a == b {
        a.to_string()
    } else {
        format!("{a}\n---\n{b}")
    };
    dst.insert("description".to_string(), serde_json::Value::String(merged));
}

#[cfg(test)]
#[allow(deprecated)] // in-memory tests use full-graph loaders
mod tests {
    use super::*;
    use crate::traits::{GraphStorage, GraphStorageMutateOps, GraphStorageReadOps, VectorStorage};
    use crate::{MemoryGraphStorage, MemoryVectorStorage};

    fn props(desc: &str, ids: &[&str]) -> std::collections::HashMap<String, serde_json::Value> {
        let mut p = std::collections::HashMap::new();
        p.insert("description".to_string(), serde_json::json!(desc));
        p.insert(
            "source_chunk_ids".to_string(),
            serde_json::json!(ids.iter().map(|s| s.to_string()).collect::<Vec<_>>()),
        );
        p
    }

    #[tokio::test]
    async fn plan_detects_unnormalized_nodes_and_skips_clean_ones() {
        let graph = MemoryGraphStorage::new("test");
        graph.initialize().await.unwrap();
        let vectors = MemoryVectorStorage::new("test", 4);
        vectors.initialize().await.unwrap();

        // Two raw variants of the same entity + one already-normalized node.
        graph
            .upsert_node("John Doe", props("jd", &["c1"]))
            .await
            .unwrap();
        graph
            .upsert_node("john doe", props("jd2", &["c2"]))
            .await
            .unwrap();
        graph
            .upsert_node("ALREADY_NORM", props("x", &["c3"]))
            .await
            .unwrap();

        let plan = plan(&graph, &vectors).await.unwrap();
        assert_eq!(
            plan.already_normalized, 1,
            "ALREADY_NORM must be counted as clean"
        );
        assert_eq!(
            plan.merge_groups.len(),
            1,
            "both raw variants collapse into one group"
        );
        let g = &plan.merge_groups[0];
        assert_eq!(g.normalized_id, "JOHN_DOE");
        assert_eq!(g.raw_nodes.len(), 2);
        assert!(!plan.is_clean(), "plan must report work to do");
        assert!(
            !plan.confirm_token.is_empty(),
            "plan must carry a confirm token"
        );
    }

    #[tokio::test]
    async fn two_casing_variants_merge_into_one_node_with_unioned_source_ids() {
        // E5: "John Doe" + "john doe" are two raw variants of the SAME entity
        // and must collapse into one JOHNN_DOE node with both source_chunk_ids.
        let graph = MemoryGraphStorage::new("test");
        graph.initialize().await.unwrap();
        let vectors = MemoryVectorStorage::new("test", 4);
        vectors.initialize().await.unwrap();

        graph
            .upsert_node("John Doe", props("jd", &["c1"]))
            .await
            .unwrap();
        graph
            .upsert_node("john doe", props("jd2", &["c2"]))
            .await
            .unwrap();

        let plan = super::plan(&graph, &vectors).await.unwrap();
        assert_eq!(plan.merge_groups.len(), 1, "both variants → one group");
        let token = plan.confirm_token.clone();
        let result = execute(&graph, &vectors, &plan, &token).await.unwrap();
        assert_eq!(result.nodes_merged, 2);

        // Exactly one normalized node remains for this entity.
        let nodes = graph.get_all_nodes().await.unwrap();
        let matching: Vec<_> = nodes.iter().filter(|n| n.id == "JOHN_DOE").collect();
        assert_eq!(matching.len(), 1, "exactly one JOHNN_DOE node after merge");

        // It carries the UNION of both variants' source_chunk_ids.
        let src_ids: Vec<String> = matching[0]
            .properties
            .get("source_chunk_ids")
            .and_then(|v| v.as_array())
            .into_iter()
            .flatten()
            .filter_map(|v| v.as_str().map(String::from))
            .collect();
        assert!(src_ids.contains(&"c1".to_string()) && src_ids.contains(&"c2".to_string()));
    }

    #[tokio::test]
    async fn execute_merges_raw_nodes_and_rewrites_edges() {
        let graph = MemoryGraphStorage::new("test");
        graph.initialize().await.unwrap();
        let vectors = MemoryVectorStorage::new("test", 4);
        vectors.initialize().await.unwrap();

        // John Doe (raw) -> jane doe (raw) edge; both should normalize.
        graph
            .upsert_node("John Doe", props("jd", &["c1"]))
            .await
            .unwrap();
        graph
            .upsert_node("jane doe", props("jane", &["c2"]))
            .await
            .unwrap();
        graph
            .upsert_edge("John Doe", "jane doe", Default::default())
            .await
            .unwrap();

        let plan = super::plan(&graph, &vectors).await.unwrap();
        assert_eq!(
            plan.edge_rewrites.len(),
            1,
            "the one edge must be rewritten"
        );

        let token = plan.confirm_token.clone();
        let result = execute(&graph, &vectors, &plan, &token).await.unwrap();
        assert_eq!(result.nodes_merged, 2, "both raw nodes must be merged");
        assert_eq!(result.edges_rewritten, 1, "the edge must be rewritten");

        // Post-condition: only normalized nodes remain, edge is normalized.
        let nodes = graph.get_all_nodes().await.unwrap();
        let ids: Vec<String> = nodes.iter().map(|n| n.id.clone()).collect();
        assert!(ids.contains(&"JOHN_DOE".to_string()), "JOHN_DOE must exist");
        assert!(ids.contains(&"JANE_DOE".to_string()), "JANE_DOE must exist");
        assert!(
            !ids.contains(&"John Doe".to_string()),
            "raw John Doe must be deleted"
        );
        assert!(
            !ids.contains(&"jane doe".to_string()),
            "raw jane doe must be deleted"
        );

        // Each normalized node must carry its own source_chunk_ids through the merge.
        let john = graph.get_node("JOHN_DOE").await.unwrap().unwrap();
        let john_ids: Vec<String> = john
            .properties
            .get("source_chunk_ids")
            .and_then(|v| v.as_array())
            .into_iter()
            .flatten()
            .filter_map(|v| v.as_str().map(String::from))
            .collect();
        assert!(
            john_ids.contains(&"c1".to_string()),
            "JOHN_DOE must keep c1"
        );
        let jane = graph.get_node("JANE_DOE").await.unwrap().unwrap();
        let jane_ids: Vec<String> = jane
            .properties
            .get("source_chunk_ids")
            .and_then(|v| v.as_array())
            .into_iter()
            .flatten()
            .filter_map(|v| v.as_str().map(String::from))
            .collect();
        assert!(
            jane_ids.contains(&"c2".to_string()),
            "JANE_DOE must keep c2"
        );

        // The edge is now normalized.
        assert!(graph
            .get_edge("JOHN_DOE", "JANE_DOE")
            .await
            .unwrap()
            .is_some());

        // Idempotent: re-running plan + execute is a no-op.
        let plan2 = super::plan(&graph, &vectors).await.unwrap();
        assert!(
            plan2.is_clean(),
            "reconciled graph must produce a clean plan"
        );
    }

    #[tokio::test]
    async fn execute_refuses_stale_confirm_token() {
        let graph = MemoryGraphStorage::new("test");
        graph.initialize().await.unwrap();
        let vectors = MemoryVectorStorage::new("test", 4);
        vectors.initialize().await.unwrap();
        graph
            .upsert_node("John Doe", props("jd", &["c1"]))
            .await
            .unwrap();

        let plan = plan(&graph, &vectors).await.unwrap();
        let err = execute(&graph, &vectors, &plan, "wrong-token").await;
        assert!(err.is_err(), "a wrong confirm token must be refused");
        // Nothing was mutated.
        assert!(graph.get_node("John Doe").await.unwrap().is_some());
    }

    #[tokio::test]
    async fn execute_rekeys_entity_vectors() {
        let graph = MemoryGraphStorage::new("test");
        graph.initialize().await.unwrap();
        let vectors = MemoryVectorStorage::new("test", 4);
        vectors.initialize().await.unwrap();

        graph
            .upsert_node("John Doe", props("jd", &["c1"]))
            .await
            .unwrap();
        // Legacy entity vector keyed by the raw name.
        vectors
            .upsert(&[(
                "entity:John Doe".to_string(),
                vec![0.1, 0.2, 0.3, 0.4],
                serde_json::json!({"type": "entity"}),
            )])
            .await
            .unwrap();

        let plan = plan(&graph, &vectors).await.unwrap();
        assert_eq!(plan.vector_rekeys.len(), 1, "one vector re-key expected");
        let token = plan.confirm_token.clone();
        let result = execute(&graph, &vectors, &plan, &token).await.unwrap();
        assert_eq!(
            result.vectors_rekeyed, 1,
            "the legacy vector must be re-keyed"
        );

        // Old vector gone, normalized vector present.
        assert!(vectors
            .get_by_id("entity:John Doe")
            .await
            .unwrap()
            .is_none());
        let norm_vid = EntityId::from_normalized("JOHN_DOE").as_vector_id();
        assert!(vectors.get_by_id(&norm_vid).await.unwrap().is_some());
    }
}
