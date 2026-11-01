import type { GraphEdge, GraphNode } from "@/types";
import { degreeTotal } from "@/types/graph";
import { getGraphEdgeKeyFromEdge } from "@/lib/graph/ids";
import { formatEntityLabel } from "@/lib/graph/label-utils";
import { assignPairCurvatures, undirectedPairKey } from "./edge-curvature";
import type Graph from "graphology";
import type { GraphDelta } from "./types";

/**
 * A lone edge between two nodes is drawn straight ("arrow") — curving every edge
 * turns hubs into unreadable hairballs. Parallel / reverse edges use the curved
 * program so they stay distinguishable.
 * NB: `@sigma/edge-curve` renders nothing at curvature 0, hence two programs.
 */
export const STRAIGHT_EDGE_TYPE = "arrow";
export const CURVED_EDGE_TYPE = "curvedArrow";

/**
 * Bow applied to a self-loop (a relationship from an entity to itself).
 * A self-loop has no chord to bend along, so it is given a fixed pronounced
 * curvature; the curved program then draws a visible loop instead of nothing.
 */
export const SELF_LOOP_CURVATURE = 0.8;

export function resolveEdgeKey(
  edge: Pick<GraphEdge, "id" | "source" | "target" | "relationship_type">,
): string {
  if (edge.id && edge.id.trim() !== "") return edge.id;
  return getGraphEdgeKeyFromEdge(edge);
}

export function calculateNodeSize(degree: number, baseSize: number): number {
  if (degree === 0) return baseSize;
  const scaleFactor = Math.log2(degree + 1) * 2;
  return Math.min(baseSize + scaleFactor, baseSize * 3);
}

export interface ApplyDeltaContext {
  borderColor: string;
  edgeColor: string;
  nodeSize: number;
  showEdgeLabels: boolean;
  getNodeColor: (entityType?: string) => string;
}

/**
 * Mutate a long-lived MultiGraph in place. Never replaces the graph instance.
 */
export function applyDelta(
  graph: Graph,
  delta: GraphDelta,
  ctx: ApplyDeltaContext,
): { addedNodes: number; addedEdges: number; removedNodes: number; removedEdges: number } {
  let addedNodes = 0;
  let addedEdges = 0;
  let removedNodes = 0;
  let removedEdges = 0;

  for (const id of delta.removeEdgeIds ?? []) {
    if (graph.hasEdge(id)) {
      graph.dropEdge(id);
      removedEdges++;
    }
  }

  for (const id of delta.removeNodeIds ?? []) {
    if (graph.hasNode(id)) {
      graph.dropNode(id);
      removedNodes++;
    }
  }

  const existingCount = graph.order;
  (delta.upsertNodes ?? []).forEach((node, index) => {
    if (!node.id || typeof node.id !== "string") return;
    const degree = degreeTotal(node.degree);
    const size = calculateNodeSize(degree, ctx.nodeSize);
    const attrs = {
      label: formatEntityLabel(node.label),
      size,
      color: ctx.getNodeColor(node.node_type),
      borderColor: ctx.borderColor,
      borderSize: 0.2,
      type: "border",
      entityType: node.node_type,
      description: node.description,
      degree,
      community_id: node.community_id,
      created_at: node.created_at ?? node.properties?.created_at,
    };

    if (graph.hasNode(node.id)) {
      for (const [k, v] of Object.entries(attrs)) {
        graph.setNodeAttribute(node.id, k, v);
      }
      return;
    }

    const angle =
      (2 * Math.PI * (existingCount + index)) /
      Math.max(existingCount + (delta.upsertNodes?.length ?? 1), 1);
    const radius = 100 + existingCount * 2;
    graph.addNode(node.id, {
      ...attrs,
      x: Math.cos(angle) * radius,
      y: Math.sin(angle) * radius,
      // Provisional ring position — LayoutScheduler replaces it (placed → true).
      placed: false,
    });
    addedNodes++;
  });

  for (const edge of delta.upsertEdges ?? []) {
    if (!edge.source || !edge.target) continue;
    if (!graph.hasNode(edge.source) || !graph.hasNode(edge.target)) continue;
    const edgeId = resolveEdgeKey(edge);
    const isSelfLoop = edge.source === edge.target;
    const edgeAttrs = {
      label: edge.relationship_type,
      forceLabel: ctx.showEdgeLabels,
      size: Math.max(1, Math.min((edge.weight ?? 1) * 2, 5)),
      color: ctx.edgeColor,
      // WHY self-loops need the curved program: a self-loop has zero chord
      // length, so the straight arrow program has nothing to draw and the edge
      // silently disappears. Bending it produces the visible loop an ER diagram
      // would show for a reflexive relationship.
      type: isSelfLoop ? CURVED_EDGE_TYPE : STRAIGHT_EDGE_TYPE,
      ...(isSelfLoop ? { curvature: SELF_LOOP_CURVATURE } : {}),
      relationshipType: edge.relationship_type,
    };

    if (graph.hasEdge(edgeId)) {
      for (const [k, v] of Object.entries(edgeAttrs)) {
        graph.setEdgeAttribute(edgeId, k, v);
      }
      continue;
    }

    try {
      graph.addEdgeWithKey(edgeId, edge.source, edge.target, edgeAttrs);
      addedEdges++;
    } catch {
      // MultiGraph race / invalid — skip
    }
  }

  refreshParallelCurvature(graph);
  return { addedNodes, addedEdges, removedNodes, removedEdges };
}

/** Spread parallel edges by curvature without pulling WebGL into unit tests. */
export function refreshParallelCurvature(graph: Graph): void {
  if (graph.size === 0) return;

  // Group edges by UNDIRECTED pair. Antiparallel edges (A→B and B→A) belong to
  // the same fan: they must bow to opposite sides, and their magnitudes must be
  // balanced, otherwise the pair reads as one thick arc plus one stray curve.
  //
  // Self-loops are excluded: they have no chord and already carry their own
  // fixed curvature from `applyDelta`.
  const pairs = new Map<string, Array<{ edge: string; source: string; target: string }>>();
  graph.forEachEdge((edge, _attrs, source, target) => {
    if (source === target) return;
    const key = undirectedPairKey(source, target);
    const bucket = pairs.get(key);
    if (bucket) bucket.push({ edge, source, target });
    else pairs.set(key, [{ edge, source, target }]);
  });

  for (const bucket of pairs.values()) {
    // Deterministic order: the same graph must always paint identically.
    bucket.sort((a, b) => (a.edge < b.edge ? -1 : a.edge > b.edge ? 1 : 0));

    if (bucket.length === 1) {
      const { edge } = bucket[0]!;
      graph.setEdgeAttribute(edge, "type", STRAIGHT_EDGE_TYPE);
      graph.removeEdgeAttribute(edge, "curvature");
      graph.setEdgeAttribute(edge, "parallelIndex", 0);
      graph.setEdgeAttribute(edge, "parallelMaxIndex", 0);
      continue;
    }

    const curvatures = assignPairCurvatures(bucket);
    bucket.forEach(({ edge }, index) => {
      graph.setEdgeAttribute(edge, "type", CURVED_EDGE_TYPE);
      graph.setEdgeAttribute(edge, "curvature", curvatures[index]!);
      graph.setEdgeAttribute(edge, "parallelIndex", index);
      graph.setEdgeAttribute(edge, "parallelMaxIndex", bucket.length - 1);
    });
  }
}

/**
 * Diff current graphology contents against desired node/edge arrays.
 * Positions of existing nodes are preserved (no full rebuild).
 */
export function diffToDelta(
  graph: Graph,
  nodes: GraphNode[],
  edges: GraphEdge[],
): GraphDelta {
  const desiredNodes = new Set(nodes.map((n) => n.id));
  const desiredEdges = new Set(edges.map((e) => resolveEdgeKey(e)));

  const removeNodeIds: string[] = [];
  graph.forEachNode((id) => {
    if (!desiredNodes.has(id)) removeNodeIds.push(id);
  });

  const removeEdgeIds: string[] = [];
  graph.forEachEdge((id) => {
    if (!desiredEdges.has(id)) removeEdgeIds.push(id);
  });

  return {
    upsertNodes: nodes,
    upsertEdges: edges,
    removeNodeIds,
    removeEdgeIds,
  };
}
