-- Serve the graph list pages in index order.
--
-- list_nodes / list_edges (and the keyset scans) filter one workspace and then
-- ORDER BY node_id (nodes) or source_id, target_id, rel_type (edges). The only
-- scope index was (namespace, tenant_id, workspace_id), which cannot return
-- rows in that order. Every page therefore bitmap-scanned the whole workspace
-- and sorted it, spilling to disk. Workspace of 66,682 nodes, first 500 rows,
-- as edgequake_tenant_access:
--
--     before   1,550 ms   Sort Method: external merge, Disk: 55,400kB
--     after        3 ms   Index Scan, stops at LIMIT
--
-- The new indexes extend the old ones with the sort key, so the old ones are
-- a strict prefix and are dropped; the table gains no extra index to maintain.
-- Deep OFFSETs still walk the skipped rows; callers that sweep a whole graph
-- use the keyset scans (scan_nodes_after / scan_edges_after).

CREATE INDEX IF NOT EXISTS idx_eq_graph_nodes_scope_node
    ON public.eq_graph_nodes (namespace, tenant_id, workspace_id, node_id);

CREATE INDEX IF NOT EXISTS idx_eq_graph_edges_scope_pk
    ON public.eq_graph_edges (namespace, tenant_id, workspace_id, source_id, target_id, rel_type);

DROP INDEX IF EXISTS public.idx_eq_graph_nodes_tenant_ws;
DROP INDEX IF EXISTS public.idx_eq_graph_edges_tenant_ws;
