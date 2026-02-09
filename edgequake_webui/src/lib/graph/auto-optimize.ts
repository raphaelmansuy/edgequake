/**
 * @module AutoOptimize
 * @description Automatic performance optimization for graph visualization.
 * Calculates optimal settings based on workspace size and device capabilities.
 *
 * @implements FEAT0607 - Auto-optimization for large graphs
 * @enforces BR0009 - Performant rendering for 1000+ nodes
 */

import { MAX_DISPLAY_NODES } from "@/stores/use-graph-store";

/**
 * Device performance tier based on available memory and cores
 */
export type DeviceTier = "low" | "medium" | "high";

/**
 * Optimized settings calculated for the graph
 */
export interface OptimizedSettings {
  maxNodes: number;
  depth: number;
  batchSize: number;
  layoutIterations: number;
  enableBarnesHut: boolean;
  tier: DeviceTier;
}

/**
 * WHY: Detect device performance tier based on available resources
 * - Uses deviceMemory API when available (Chrome)
 * - Falls back to hardwareConcurrency (CPU cores)
 * - Conservative defaults for unknown devices
 */
export function detectDeviceTier(): DeviceTier {
  if (typeof window === "undefined") return "medium";

  // Check device memory (Chrome only)
  // @ts-expect-error deviceMemory is only available in Chrome
  const deviceMemory = navigator.deviceMemory as number | undefined;

  // Check CPU cores
  const cpuCores = navigator.hardwareConcurrency || 4;

  // High-end: 8GB+ RAM or 8+ cores
  if ((deviceMemory && deviceMemory >= 8) || cpuCores >= 8) {
    return "high";
  }

  // Low-end: 2GB or less RAM, or 2 cores or less
  if ((deviceMemory && deviceMemory <= 2) || cpuCores <= 2) {
    return "low";
  }

  return "medium";
}

/**
 * WHY: Calculate optimal maxNodes based on total nodes in storage
 * - For small graphs (< 500 nodes): Show all nodes
 * - For medium graphs (500-5000): Show 20% of nodes, min 100, max 500
 * - For large graphs (5000+): Show 2-5% of nodes, max based on device tier
 *
 * @param totalNodesInStorage - Total count of nodes in the workspace
 * @param deviceTier - Performance tier of the device (optional, auto-detected)
 */
export function calculateOptimalMaxNodes(
  totalNodesInStorage: number,
  deviceTier?: DeviceTier,
): OptimizedSettings {
  const tier = deviceTier || detectDeviceTier();

  // WHY: Base limits per device tier - all capped at MAX_DISPLAY_NODES for performance
  const maxLimits = {
    low: { maxNodes: 200, batchSize: 25, layoutIterations: 30 },
    medium: { maxNodes: Math.min(400, MAX_DISPLAY_NODES), batchSize: 50, layoutIterations: 50 },
    high: { maxNodes: MAX_DISPLAY_NODES, batchSize: 100, layoutIterations: 80 },
  };

  const limits = maxLimits[tier];

  // WHY: Small graphs don't need optimization
  if (totalNodesInStorage <= 100) {
    return {
      maxNodes: totalNodesInStorage,
      depth: 3,
      batchSize: limits.batchSize,
      layoutIterations: limits.layoutIterations,
      enableBarnesHut: false,
      tier,
    };
  }

  // WHY: Medium graphs - show enough to be useful
  if (totalNodesInStorage <= 500) {
    return {
      maxNodes: Math.min(totalNodesInStorage, limits.maxNodes),
      depth: 2,
      batchSize: limits.batchSize,
      layoutIterations: limits.layoutIterations,
      enableBarnesHut: totalNodesInStorage > 200,
      tier,
    };
  }

  // WHY: Large graphs (500-5000) - show top connected nodes
  if (totalNodesInStorage <= 5000) {
    const optimalMax = Math.max(
      100,
      Math.min(
        Math.floor(totalNodesInStorage * 0.2), // 20%
        limits.maxNodes,
      ),
    );

    return {
      maxNodes: optimalMax,
      depth: 2,
      batchSize: limits.batchSize,
      layoutIterations: Math.max(30, Math.floor(limits.layoutIterations * 0.7)),
      enableBarnesHut: true,
      tier,
    };
  }

  // WHY: Very large graphs (5000+) - strict limiting
  const optimalMax = Math.max(
    100,
    Math.min(
      Math.floor(totalNodesInStorage * 0.05), // 5%
      limits.maxNodes,
    ),
  );

  return {
    maxNodes: optimalMax,
    depth: 1, // Lower depth for huge graphs
    batchSize: limits.batchSize,
    layoutIterations: Math.max(20, Math.floor(limits.layoutIterations * 0.5)),
    enableBarnesHut: true,
    tier,
  };
}

/**
 * WHY: Format node count for display with human-readable suffixes
 */
export function formatNodeCount(count: number): string {
  if (count < 1000) return count.toString();
  if (count < 1000000) return `${(count / 1000).toFixed(1)}K`;
  return `${(count / 1000000).toFixed(1)}M`;
}

/**
 * WHY: Estimate memory usage in MB for a given node/edge count
 * Based on typical GraphNode/GraphEdge object sizes
 * - Node: ~500 bytes (id, label, type, description, properties)
 * - Edge: ~300 bytes (source, target, type, weight)
 */
export function estimateMemoryUsage(
  nodeCount: number,
  edgeCount: number,
): number {
  const nodeMemory = (nodeCount * 500) / (1024 * 1024); // Convert to MB
  const edgeMemory = (edgeCount * 300) / (1024 * 1024);
  const overheadFactor = 1.5; // Index structures and overhead

  return Math.round((nodeMemory + edgeMemory) * overheadFactor);
}

/**
 * WHY: Check if current settings might cause performance issues
 */
export function checkPerformanceWarnings(
  maxNodes: number,
  totalNodes: number,
  deviceTier: DeviceTier,
): string[] {
  const warnings: string[] = [];

  const thresholds = {
    low: { warn: 150, danger: 300 },
    medium: { warn: 400, danger: 700 },
    high: { warn: 700, danger: 1200 },
  };

  const { warn, danger } = thresholds[deviceTier];

  if (maxNodes > danger) {
    warnings.push(
      `Loading ${maxNodes} nodes may cause slowdowns on this device`,
    );
  } else if (maxNodes > warn) {
    warnings.push(`Performance may be affected with ${maxNodes} nodes`);
  }

  if (totalNodes > 10000 && maxNodes > totalNodes * 0.1) {
    warnings.push("Consider reducing max nodes for very large workspaces");
  }

  return warnings;
}
