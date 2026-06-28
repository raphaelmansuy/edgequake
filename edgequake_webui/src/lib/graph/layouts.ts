import type { GraphSettings } from "@/types";
import Graph from "graphology";
import forceLayout from "graphology-layout-force";
import forceAtlas2 from "graphology-layout-forceatlas2";
import noverlap from "graphology-layout-noverlap";
import circlepack from "graphology-layout/circlepack";
import circular from "graphology-layout/circular";
import random from "graphology-layout/random";

export type GraphLayoutType = GraphSettings["layout"];
export type GraphLayoutMode = "initial" | "interactive" | "streaming";

export interface GraphPerformanceProfile {
  nodeCount: number;
  edgeCount: number;
  isLargeGraph: boolean;
  isVeryLargeGraph: boolean;
  labelGridCellSize: number;
  labelDensity: number;
  labelRenderedSizeThreshold: number;
  disableEdgeEvents: boolean;
  /** Hide edges while the camera is moving to maintain 60fps panning. */
  hideEdgesOnMove: boolean;
  /** Hide labels while the camera is moving (Canvas 2D labels block main thread). */
  hideLabelsOnMove: boolean;
}

export type LayoutPositions = Record<string, Record<string, number>>;

export function getGraphPerformanceProfile(
  nodeCount: number,
  edgeCount: number,
): GraphPerformanceProfile {
  const isLargeGraph = nodeCount > 200 || edgeCount > 400;
  const isVeryLargeGraph = nodeCount > 500 || edgeCount > 1000;

  return {
    nodeCount,
    edgeCount,
    isLargeGraph,
    isVeryLargeGraph,
    // Larger cell = fewer labels shown = less canvas draw work per frame
    labelGridCellSize: isVeryLargeGraph ? 160 : isLargeGraph ? 120 : 80,
    // Higher density = more labels per cell; balanced with grid culling
    labelDensity: isVeryLargeGraph ? 0.5 : isLargeGraph ? 0.8 : 1.0,
    // Minimum rendered-size threshold: skip labels for nodes too small to read
    labelRenderedSizeThreshold: isVeryLargeGraph ? 5 : isLargeGraph ? 3 : 2,
    disableEdgeEvents: isVeryLargeGraph,
    // Hide edges during panning: critical for graphs with thousands of edges
    hideEdgesOnMove: isLargeGraph,
    // Hide labels during pan: Canvas 2D label drawing blocks main thread
    hideLabelsOnMove: isLargeGraph,
  };
}

function getForceAtlas2Iterations(nodeCount: number, mode: GraphLayoutMode): number {
  const baseIterations =
    nodeCount > 500 ? 40 : nodeCount > 200 ? 60 : nodeCount > 100 ? 80 : 100;

  if (mode === "streaming") {
    return Math.max(12, Math.floor(baseIterations * 0.3));
  }

  if (mode === "interactive") {
    return Math.max(20, Math.floor(baseIterations * 0.6));
  }

  return baseIterations;
}

function getNoverlapIterations(nodeCount: number, mode: GraphLayoutMode): number {
  const baseIterations = nodeCount > 400 ? 60 : nodeCount > 150 ? 90 : 120;

  if (mode === "streaming") {
    return Math.max(20, Math.floor(baseIterations * 0.33));
  }

  if (mode === "interactive") {
    return Math.max(40, Math.floor(baseIterations * 0.66));
  }

  return baseIterations;
}

function applyHierarchicalLayout(graph: Graph): void {
  const nodesByType = new Map<string, string[]>();

  graph.forEachNode((nodeId, attrs) => {
    const nodeType =
      (typeof attrs.entityType === "string" && attrs.entityType) ||
      (typeof attrs.node_type === "string" && attrs.node_type) ||
      "unknown";

    if (!nodesByType.has(nodeType)) {
      nodesByType.set(nodeType, []);
    }

    nodesByType.get(nodeType)!.push(nodeId);
  });

  const levels = Array.from(nodesByType.keys()).sort();
  const levelHeight = 200;
  const nodeSpacing = 100;

  levels.forEach((type, levelIndex) => {
    const nodes = nodesByType.get(type) ?? [];
    const offset = (nodes.length - 1) / 2;

    nodes.forEach((nodeId, nodeIndex) => {
      graph.setNodeAttribute(nodeId, "x", (nodeIndex - offset) * nodeSpacing);
      graph.setNodeAttribute(nodeId, "y", levelIndex * levelHeight);
    });
  });
}

export function applyLayoutToGraph(
  graph: Graph,
  layout: GraphLayoutType,
  mode: GraphLayoutMode = "initial",
): void {
  switch (layout) {
    case "circular":
      circular.assign(graph);
      return;

    case "circlepack":
      circlepack.assign(graph, {
        hierarchyAttributes: ["entityType", "node_type"],
        scale: 100,
      });
      return;

    case "random":
      random.assign(graph);
      return;

    case "noverlaps":
      noverlap.assign(graph, {
        maxIterations: getNoverlapIterations(graph.order, mode),
        settings: {
          margin: 5,
          expansion: 1.1,
          gridSize: graph.order > 100 ? 20 : 1,
          ratio: 1,
          speed: 3,
        },
      });
      return;

    case "force-directed":
      forceLayout.assign(graph, {
        maxIterations: mode === "streaming" ? 30 : mode === "interactive" ? 60 : 100,
        settings: {
          attraction: 0.0003,
          repulsion: 0.02,
          gravity: 0.02,
          inertia: 0.4,
          maxMove: 100,
        },
      });
      return;

    case "hierarchical":
      applyHierarchicalLayout(graph);
      return;

    case "force":
    default: {
      const inferred = forceAtlas2.inferSettings(graph);

      forceAtlas2.assign(graph, {
        iterations: getForceAtlas2Iterations(graph.order, mode),
        settings: {
          ...inferred,
          gravity: 1,
          scalingRatio: 2,
          strongGravityMode: true,
          barnesHutOptimize: graph.order > 50,
          barnesHutTheta: graph.order > 200 ? 0.7 : 0.6,
          slowDown: mode === "streaming" ? 2.5 : 2,
          edgeWeightInfluence: 0.5,
        },
      });
    }
  }
}

export function calculateLayoutPositions(
  graph: Graph,
  layout: GraphLayoutType,
  mode: GraphLayoutMode = "initial",
): LayoutPositions {
  const tempGraph = graph.copy();
  applyLayoutToGraph(tempGraph, layout, mode);

  const positions: LayoutPositions = {};
  tempGraph.forEachNode((nodeId) => {
    positions[nodeId] = {
      x: tempGraph.getNodeAttribute(nodeId, "x"),
      y: tempGraph.getNodeAttribute(nodeId, "y"),
    };
  });

  return positions;
}
