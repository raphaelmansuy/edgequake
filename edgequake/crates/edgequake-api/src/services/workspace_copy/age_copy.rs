//! AGE graph copy phase for WorkspaceCopy (EN-3677 Phase 1).
//!
//! ## What lives in AGE (per workspace)
//!
//! Each tenant has one AGE graph named `eq_{ns}_graph` where `{ns}` is a
//! deterministic namespace derived from tenant + workspace. Within that
//! graph, workspaces manifest as vertices/edges tagged with `workspace_id`
//! properties on the two Apache-AGE-internal labels:
//!
//! - `_ag_label_vertex` — every entity/node the pipeline created
//! - `_ag_label_edge`   — every relationship
//!
//! ## Copy strategy
//!
//! Use `scan_ops::list_vertices_for_workspace(source)` + the matching
//! `list_edges_for_workspace(source)` to enumerate the source graph, then
//! issue per-vertex / per-edge `CREATE` Cypher against the destination
//! workspace's graph namespace. Entities/relationships in Postgres already
//! carry their AGE `graphid` — the copy needs to update those graphid
//! columns on the destination rows to point at the newly-created AGE nodes
//! (the SQL copy phase writes the Postgres rows first with `graphid = NULL`
//! placeholder, then this phase fills them in).
//!
//! ## Ordering with SQL copy
//!
//! Must run **after** SQL copy inserts the destination `entities` and
//! `relationships` rows — otherwise the graphid-update step has nothing to
//! write to. `mod.rs` enforces this ordering; do not call this module in
//! isolation.

use uuid::Uuid;

/// Result of the AGE graph copy — feeds into the job's entity/relationship
/// counters (these are the ground-truth counts because Postgres and AGE
/// must stay in sync; the SQL copy pass returns "rows written to Postgres"
/// which is the same number if we do our job right).
#[derive(Debug, Clone, Copy, Default)]
pub struct AgeCopyResult {
    pub vertices_copied: usize,
    pub edges_copied: usize,
}

/// Deterministic AGE graph name per tenant.
///
/// Kept as a helper so a rename in `edgequake-graph` is a single-symbol
/// change here. The `ns` is currently `tenant_slug` but future work may
/// switch to a hash — callers should not depend on the specific format.
pub fn tenant_graph_name(tenant_namespace: &str) -> String {
    format!("eq_{tenant_namespace}_graph")
}

/// Cypher template for copying one vertex, parameterised on
/// `(source_workspace_id, dest_workspace_id, vertex_props_json)`.
///
/// Uses `apoc.create.vertex`-equivalent AGE syntax via `CREATE` on the
/// destination-scoped label. The `properties` map is dumped verbatim
/// with `workspace_id` overridden to the destination — every other
/// property (name, type, extraction metadata) is preserved.
///
/// Kept as a `&'static str` template rather than string-interpolated
/// because AGE is finicky about quote handling; the SQL-side parameter
/// binding is safer.
pub const CREATE_VERTEX_CYPHER: &str = "CREATE (v {props}) RETURN id(v)";

/// Cypher template for copying one edge, parameterised on
/// `(source_edge_id, src_vertex_dest_id, tgt_vertex_dest_id, edge_props_json)`.
///
/// The vertex-id remap requires a lookup table populated by the vertex
/// pass — the caller must not call `copy_edges` before `copy_vertices`
/// completes.
pub const CREATE_EDGE_CYPHER: &str =
    "MATCH (a), (b) WHERE id(a) = $src AND id(b) = $tgt CREATE (a)-[e {props}]->(b) RETURN id(e)";

/// Copy the AGE vertex layer for one workspace. **Scaffold**: the real
/// implementation needs `scan_ops::list_vertices_for_workspace` which
/// lives in `edgequake-graph` — wiring that into the API crate is
/// deferred to the follow-up storage-crate commit. Phase 1 returns
/// zero counts and logs a warning.
pub async fn copy_vertices(
    _source_workspace_id: Uuid,
    _dest_workspace_id: Uuid,
    _tenant_namespace: &str,
) -> Result<usize, String> {
    tracing::warn!(
        source_workspace_id = %_source_workspace_id,
        dest_workspace_id = %_dest_workspace_id,
        "workspace_copy::age_copy: scaffold — AGE vertex copy is a no-op in Phase 1"
    );
    Ok(0)
}

/// Copy the AGE edge layer. See `copy_vertices` notes; same scaffold caveat.
pub async fn copy_edges(
    _source_workspace_id: Uuid,
    _dest_workspace_id: Uuid,
    _tenant_namespace: &str,
) -> Result<usize, String> {
    tracing::warn!(
        source_workspace_id = %_source_workspace_id,
        dest_workspace_id = %_dest_workspace_id,
        "workspace_copy::age_copy: scaffold — AGE edge copy is a no-op in Phase 1"
    );
    Ok(0)
}

/// Convenience wrapper — copies vertices then edges in the required order.
pub async fn copy_age_graph(
    source_workspace_id: Uuid,
    dest_workspace_id: Uuid,
    tenant_namespace: &str,
) -> Result<AgeCopyResult, String> {
    let vertices_copied =
        copy_vertices(source_workspace_id, dest_workspace_id, tenant_namespace).await?;
    let edges_copied =
        copy_edges(source_workspace_id, dest_workspace_id, tenant_namespace).await?;
    Ok(AgeCopyResult {
        vertices_copied,
        edges_copied,
    })
}
