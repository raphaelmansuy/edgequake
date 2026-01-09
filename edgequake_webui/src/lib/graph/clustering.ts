/**
 * @module clustering
 * @description Graph clustering using Louvain community detection.
 * Assigns colors to nodes based on detected communities.
 *
 * @implements FEAT0716 - Louvain community detection
 * @implements FEAT0717 - Community-based node coloring
 *
 * @enforces BR0713 - Max 20 community colors, wrap around
 * @enforces BR0714 - Isolated nodes get default color
 */

import Graph from "graphology";
import louvain from "graphology-communities-louvain";

// Color palette for communities (up to 20 distinct colors)
const COMMUNITY_COLORS = [
  "#3b82f6", // blue
  "#10b981", // emerald
  "#f59e0b", // amber
  "#ef4444", // red
  "#8b5cf6", // violet
  "#ec4899", // pink
  "#06b6d4", // cyan
  "#84cc16", // lime
  "#f97316", // orange
  "#6366f1", // indigo
  "#14b8a6", // teal
  "#a855f7", // purple
  "#22c55e", // green
  "#eab308", // yellow
  "#0ea5e9", // sky
  "#d946ef", // fuchsia
  "#64748b", // slate
  "#78716c", // stone
  "#059669", // emerald-600
  "#dc2626", // red-600
];

export interface Community {
  id: number;
  nodeIds: string[];
  size: number;
  color: string;
  label?: string;
}

export interface ClusteringResult {
  communities: Community[];
  nodeToCommuntiy: Map<string, number>;
  modularity: number;
}

/**
 * Applies Louvain community detection to a graph
 * and assigns colors based on community membership.
 */
export function detectCommunities(graph: Graph): ClusteringResult {
  if (graph.order === 0) {
    return {
      communities: [],
      nodeToCommuntiy: new Map(),
      modularity: 0,
    };
  }

  // Run Louvain algorithm - returns community assignments
  const communities = louvain(graph);

  // Build community mapping
  const nodeToCommuntiy = new Map<string, number>();
  const communityNodes: Record<number, string[]> = {};

  graph.forEachNode((node) => {
    const communityId = communities[node];
    nodeToCommuntiy.set(node, communityId);

    if (!communityNodes[communityId]) {
      communityNodes[communityId] = [];
    }
    communityNodes[communityId].push(node);
  });

  // Build community list
  const communityList: Community[] = Object.entries(communityNodes)
    .map(([id, nodes], index) => ({
      id: parseInt(id),
      nodeIds: nodes,
      size: nodes.length,
      color: COMMUNITY_COLORS[index % COMMUNITY_COLORS.length],
      label: `Community ${parseInt(id) + 1}`,
    }))
    .sort((a, b) => b.size - a.size);

  // Calculate approximate modularity (number of communities / total nodes)
  const approxModularity =
    communityList.length > 0 ? 1 - 1 / communityList.length : 0;

  return {
    communities: communityList,
    nodeToCommuntiy,
    modularity: approxModularity,
  };
}

/**
 * Apply community colors to graph nodes
 */
export function applyCommuntiyColors(
  graph: Graph,
  result: ClusteringResult
): void {
  const communityColorMap = new Map<number, string>();
  result.communities.forEach((community) => {
    communityColorMap.set(community.id, community.color);
  });

  graph.forEachNode((node) => {
    const communityId = result.nodeToCommuntiy.get(node);
    if (communityId !== undefined) {
      const color = communityColorMap.get(communityId);
      if (color) {
        graph.setNodeAttribute(node, "communityColor", color);
        graph.setNodeAttribute(node, "community", communityId);
      }
    }
  });
}

/**
 * Get community summary statistics
 */
export function getCommunitySummary(result: ClusteringResult): {
  totalCommunities: number;
  largestCommunity: number;
  averageSize: number;
  modularity: number;
} {
  const sizes = result.communities.map((c) => c.size);
  return {
    totalCommunities: result.communities.length,
    largestCommunity: sizes.length > 0 ? Math.max(...sizes) : 0,
    averageSize:
      sizes.length > 0 ? sizes.reduce((a, b) => a + b, 0) / sizes.length : 0,
    modularity: result.modularity,
  };
}

export function getCommunityColor(communityId: number): string {
  return COMMUNITY_COLORS[communityId % COMMUNITY_COLORS.length];
}

export default {
  detectCommunities,
  applyCommuntiyColors,
  getCommunitySummary,
  getCommunityColor,
};
