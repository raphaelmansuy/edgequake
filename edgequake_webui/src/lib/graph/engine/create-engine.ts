import { detectCommunities, getCommunityColor } from "@/lib/graph/clustering";
import {
  applyLayoutToGraph,
  calculateLayoutPositions,
  getGraphPerformanceProfile,
  type GraphLayoutType,
} from "@/lib/graph/layouts";
import {
  drawEdgeLabelWithBackground,
  drawNodeHoverWithCard,
  drawNodeLabelWithBackground,
} from "@/lib/graph/sigma-renderers";
import { EdgeCurvedArrowProgram, createEdgeCurveProgram } from "@sigma/edge-curve";
import { NodeBorderProgram } from "@sigma/node-border";
import { MultiGraph } from "graphology";
import Sigma from "sigma";
import { EdgeArrowProgram } from "sigma/rendering";
import { animateNodes } from "sigma/utils";
import { applyDelta, diffToDelta, type ApplyDeltaContext } from "./apply-delta";
import { clampEgoDepth, collectHopDistances, hopFade } from "./ego";
import { bindInteractions, type InteractionHandle } from "./interactions";
import { LayoutScheduler, markAllPlaced } from "./layout-scheduler";
import { exportGraphImage, type ExportImageOptions } from "./export";
import { filterGraphData } from "./filter-pipeline";
import {
  bindFocusEdgeOverlay,
  type FocusEdgeOverlayHandle,
} from "./focus-edge-overlay";
import {
  edgeReducerAttrs,
  edgeStackZIndex,
  focusModeFromHoverSelect,
  isEdgeDimmed,
  isLiftedFocusEdge,
  isNodeDimmed,
  nodeNeedsHoverLayer,
  nodeReducerAttrs,
  nodeStackZIndex,
  resolveNodeStackRole,
  type FocusContext,
} from "./focus-strategies";
import { labelColorForTheme, resolveGraphTheme } from "./theme";
import type {
  FocusState,
  GraphDelta,
  GraphEngineId,
  GraphEngineOptions,
  GraphFiltersState,
  GraphThemeTokens,
  LodTier,
} from "./types";
import type { GraphEdge, GraphNode } from "@/types";
import { createEmptyFilters } from "./filter-pipeline";

let engineSeq = 0;

function nextEngineId(): GraphEngineId {
  engineSeq += 1;
  return `ge-${engineSeq}-${Date.now().toString(36)}`;
}

function communityColorFromId(id: string, palette: string[]): string {
  let h = 0;
  for (let i = 0; i < id.length; i++) {
    h = (h * 31 + id.charCodeAt(i)) >>> 0;
  }
  return palette[h % palette.length] ?? "#64748b";
}

/** A focused node labels its edges only up to this many incident edges. */
const MAX_INCIDENT_EDGE_LABELS = 6;

const NODE_SIZES: Record<string, number> = {
  small: 6,
  medium: 10,
  large: 14,
};

export class GraphEngine {
  readonly id: GraphEngineId;
  private readonly container: HTMLElement;
  private readonly graph: MultiGraph;
  private sigma: Sigma | null = null;
  private destroyed = false;
  private options: Required<
    Pick<
      GraphEngineOptions,
      | "isDark"
      | "showLabels"
      | "showEdgeLabels"
      | "enableNodeDrag"
      | "highlightNeighbors"
      | "hideUnselectedEdges"
      | "nodeSize"
      | "layout"
      | "colorMode"
    >
  > &
    GraphEngineOptions;

  private theme: GraphThemeTokens;
  private filters: GraphFiltersState = createEmptyFilters();
  private focus: FocusState = { mode: "none", ids: [] };
  private matchingNodeIds = new Set<string>();
  private neighborIds = new Set<string>();
  /** Hop distance (1..depth) of each bright neighbour of the focus node. */
  private neighborHops = new Map<string, number>();
  private egoDepth = 1;
  private hoveredEdgeId: string | null = null;
  private hoveredNodeId: string | null = null;
  private selectedNodeId: string | null = null;
  private sourceNodes: GraphNode[] = [];
  private readonly layoutScheduler: LayoutScheduler;
  private perfTierKey = "";
  private interactions: InteractionHandle | null = null;
  private focusEdgeOverlay: FocusEdgeOverlayHandle | null = null;
  /** Node whose context menu is open (kept emphasised; selection untouched). */
  private contextTargetId: string | null = null;
  private resizeObserver: ResizeObserver | null = null;

  constructor(container: HTMLElement, options: GraphEngineOptions = {}) {
    this.id = nextEngineId();
    this.container = container;
    this.graph = new MultiGraph({ type: "directed" });
    this.options = {
      isDark: false,
      showLabels: true,
      showEdgeLabels: false,
      enableNodeDrag: true,
      highlightNeighbors: true,
      hideUnselectedEdges: false,
      nodeSize: NODE_SIZES.medium,
      layout: "force",
      colorMode: "entity-type",
      ...options,
    };
    this.theme = resolveGraphTheme(container);
    this.layoutScheduler = new LayoutScheduler({
      graph: this.graph,
      getLayout: () => this.options.layout,
      getViewportPx: () => this.viewportPx(),
      onApplied: () => this.sigma?.scheduleRefresh(),
    });
    this.container.setAttribute("data-graph-engine-id", this.id);
    this.container.setAttribute("data-graph-engine", "1");
    this.bootstrapSigma();
  }

  getSigma(): Sigma | null {
    return this.sigma;
  }

  getGraph(): MultiGraph {
    return this.graph;
  }

  getId(): GraphEngineId {
    return this.id;
  }

  applyDelta(delta: GraphDelta): void {
    if (this.destroyed) return;
    const orderBefore = this.graph.order;
    const ctx = this.deltaContext();
    const result = applyDelta(this.graph, delta, ctx);
    this.recomputeMatches();
    if (this.focus.mode !== "none") this.recomputeNeighborhood();
    // Position new nodes with the *selected* layout (not the provisional ring).
    this.layoutScheduler.notify(result, orderBefore);
    this.applyPerformanceProfile();
    this.sigma?.scheduleRefresh();
  }

  /** Sync full node/edge arrays without tearing down Sigma (G01). */
  syncData(nodes: GraphNode[], edges: GraphEdge[]): void {
    if (this.destroyed) return;
    this.sourceNodes = nodes;
    const delta = diffToDelta(this.graph, nodes, edges);
    this.applyDelta(delta);
    if (this.options.colorMode === "community") {
      this.applyCommunityColors();
    }
  }

  setFilters(partial: Partial<GraphFiltersState> & {
    types?: Set<string> | string[];
    relTypes?: Set<string> | string[];
  }): void {
    if (partial.types) {
      this.filters.types =
        partial.types instanceof Set ? partial.types : new Set(partial.types);
    }
    if (partial.relTypes) {
      this.filters.relTypes =
        partial.relTypes instanceof Set
          ? partial.relTypes
          : new Set(partial.relTypes);
    }
    if (partial.query !== undefined) this.filters.query = partial.query;
    if (partial.timeRange) this.filters.timeRange = partial.timeRange;
    if (partial.documentIds) this.filters.documentIds = partial.documentIds;
    this.recomputeMatches();
    this.sigma?.scheduleRefresh();
  }

  setFocus(focus: FocusState): void {
    // "none" means "no explicit focus", not "forget the user's selection".
    this.focus =
      focus.mode === "none"
        ? focusModeFromHoverSelect(this.hoveredNodeId, this.selectedNodeId)
        : focus;
    this.recomputeNeighborhood();
    this.sigma?.scheduleRefresh();
  }

  /**
   * Hover is a *preview*: with nothing selected it lights the node's
   * neighbourhood; with a selection it must not discard the selection's focus
   * (that made the neighbourhood vanish whenever the pointer crossed a node),
   * so it only emphasises the hovered node.
   */
  setHover(nodeId: string | null): void {
    this.hoveredNodeId = nodeId;
    if (!this.selectedNodeId) {
      this.focus = focusModeFromHoverSelect(nodeId, null);
      this.recomputeNeighborhood();
    }
    this.sigma?.scheduleRefresh();
  }

  /**
   * Context menu target. While set, the node stays emphasised (so the menu's
   * subject is unambiguous) and the pointer FSM is parked in MENU; clearing it
   * resumes normal hover.
   */
  setContextTarget(nodeId: string | null): void {
    if (this.contextTargetId === nodeId) return;
    const closed = this.contextTargetId !== null && nodeId === null;
    this.contextTargetId = nodeId;
    if (closed) this.interactions?.menuClosed();
    this.sigma?.scheduleRefresh();
  }

  setHoverEdge(edgeId: string | null): void {
    if (this.hoveredEdgeId === edgeId) return;
    this.hoveredEdgeId = edgeId;
    this.sigma?.scheduleRefresh();
  }

  setSelected(nodeId: string | null): void {
    this.selectedNodeId = nodeId;
    this.focus = focusModeFromHoverSelect(this.hoveredNodeId, nodeId);
    this.recomputeNeighborhood();
    this.sigma?.scheduleRefresh();
  }

  /** Hops (1–3) of the neighbourhood kept bright around the selected node. */
  setEgoDepth(depth: number): void {
    const next = clampEgoDepth(depth);
    if (next === this.egoDepth) return;
    this.egoDepth = next;
    this.recomputeNeighborhood();
    this.sigma?.scheduleRefresh();
  }

  getEgoDepth(): number {
    return this.egoDepth;
  }

  /**
   * Neighbours that stay bright for the current focus. Hover is always one hop
   * (cheap, transient); selection honours the ego depth; explicit `ego` focus
   * carries its own depth.
   */
  private recomputeNeighborhood(): void {
    this.neighborHops = new Map();
    const seed = this.focus.ids[0];
    if (seed && this.graph.hasNode(seed)) {
      switch (this.focus.mode) {
        case "ego":
          this.neighborHops = collectHopDistances(this.graph, seed, this.focus.depth ?? 1);
          break;
        case "select":
          this.neighborHops = collectHopDistances(this.graph, seed, this.egoDepth);
          break;
        case "hover":
          this.neighborHops = collectHopDistances(this.graph, seed, 1);
          break;
        default:
          break;
      }
    }
    this.neighborIds = new Set(this.neighborHops.keys());
  }

  setLod(tier: LodTier): void {
    void tier;
    this.sigma?.scheduleRefresh();
  }

  getLayout(): GraphLayoutType {
    return this.options.layout;
  }

  /** Switch algorithm and animate existing nodes to the new positions. */
  setLayoutMode(mode: GraphLayoutType): void {
    this.layoutScheduler.cancel();
    this.options.layout = mode;
    if (!this.sigma || this.graph.order === 0) return;
    const positions = calculateLayoutPositions(this.graph, mode, "interactive", {
      viewportPx: this.viewportPx(),
    });
    markAllPlaced(this.graph);
    animateNodes(this.graph, positions, {
      duration: 300,
      easing: "quadraticInOut",
    });
  }

  startLayout(): void {
    if (this.graph.order === 0) return;
    this.layoutScheduler.cancel();
    applyLayoutToGraph(this.graph, this.options.layout, "interactive", {
      viewportPx: this.viewportPx(),
    });
    markAllPlaced(this.graph);
    this.sigma?.scheduleRefresh();
  }

  stopLayout(): void {
    this.layoutScheduler.cancel();
  }

  pin(nodeId: string): void {
    if (this.graph.hasNode(nodeId)) {
      this.graph.setNodeAttribute(nodeId, "fixed", true);
    }
  }

  setTheme(tokens?: Partial<GraphThemeTokens>, isDark?: boolean): void {
    this.theme = { ...resolveGraphTheme(this.container), ...tokens };
    if (typeof isDark === "boolean") this.options.isDark = isDark;
    const edgeColor = this.theme.edge;
    this.graph.forEachEdge((id) => {
      this.graph.setEdgeAttribute(id, "color", edgeColor);
    });
    if (this.sigma) {
      this.sigma.setSetting("labelColor", {
        color: labelColorForTheme(this.options.isDark, this.theme),
      });
      this.sigma.setSetting("defaultEdgeColor", edgeColor);
      this.sigma.scheduleRefresh();
    }
  }

  updateOptions(partial: Partial<GraphEngineOptions>): void {
    Object.assign(this.options, partial);
    if (partial.nodeSize !== undefined) {
      this.graph.forEachNode((id) => {
        const degree = (this.graph.getNodeAttribute(id, "degree") as number) || 0;
        this.graph.setNodeAttribute(
          id,
          "size",
          // inline to avoid circular import of calculateNodeSize path issues
          degree === 0
            ? partial.nodeSize!
            : Math.min(
                partial.nodeSize! + Math.log2(degree + 1) * 2,
                partial.nodeSize! * 3,
              ),
        );
      });
    }
    if (this.sigma) {
      if (partial.showLabels !== undefined) {
        this.sigma.setSetting("renderLabels", partial.showLabels);
      }
      if (partial.showEdgeLabels !== undefined) {
        this.graph.forEachEdge((id) => {
          this.graph.setEdgeAttribute(id, "forceLabel", partial.showEdgeLabels);
        });
        this.sigma.setSetting("renderEdgeLabels", partial.showEdgeLabels);
      }
      this.sigma.scheduleRefresh();
    }
  }

  getCamera() {
    return this.sigma?.getCamera().getState() ?? null;
  }

  setCamera(state: { x?: number; y?: number; ratio?: number; angle?: number }) {
    this.sigma?.getCamera().setState(state);
  }

  async exportImage(options?: ExportImageOptions) {
    if (!this.sigma) return { ok: false as const, format: "png" as const, reason: "no_sigma" };
    return exportGraphImage(this.sigma, {
      ...options,
      backgroundColor: options?.backgroundColor ?? this.theme.canvasBg,
    });
  }

  destroy(): void {
    if (this.destroyed) return;
    this.destroyed = true;
    this.stopLayout();
    this.resizeObserver?.disconnect();
    this.resizeObserver = null;
    this.interactions?.unbind();
    this.interactions = null;
    this.focusEdgeOverlay?.unbind();
    this.focusEdgeOverlay = null;
    this.sigma?.kill();
    this.sigma = null;
    this.container.removeAttribute("data-graph-engine-id");
    this.container.removeAttribute("data-graph-engine");
  }

  private deltaContext(): ApplyDeltaContext {
    return {
      borderColor: this.options.isDark ? "#374151" : "#ffffff",
      edgeColor: this.theme.edge || (this.options.isDark ? "#4b5563" : "#94a3b8"),
      nodeSize: this.options.nodeSize,
      showEdgeLabels: this.options.showEdgeLabels,
      getNodeColor:
        this.options.getNodeColor ??
        ((t?: string) => (t ? "#64748b" : "#94a3b8")),
    };
  }

  private recomputeMatches(): void {
    const nodes: GraphNode[] =
      this.sourceNodes.length > 0
        ? this.sourceNodes
        : this.graph.mapNodes((id) => {
            const entityType = this.graph.getNodeAttribute(id, "entityType") as
              | string
              | undefined;
            return {
              id,
              label: String(this.graph.getNodeAttribute(id, "label") ?? id),
              node_type: entityType ?? "UNKNOWN",
              description: this.graph.getNodeAttribute(id, "description") as
                | string
                | undefined,
              created_at: this.graph.getNodeAttribute(id, "created_at") as
                | string
                | undefined,
            };
          });

    if (nodes.length === 0) {
      this.matchingNodeIds = new Set();
      return;
    }

    const { nodeIds } = filterGraphData(nodes, [], this.filters);
    this.matchingNodeIds = nodeIds;
  }

  /** Shorter canvas side minus Sigma's stage padding (2 × 50px). */
  private viewportPx(): number | undefined {
    const { clientWidth, clientHeight } = this.container;
    const side = Math.min(clientWidth, clientHeight) - 100;
    return side > 0 ? side : undefined;
  }

  /**
   * Sigma is bootstrapped with the empty-graph profile; re-tune label culling
   * and edge-on-move hiding once the real size crosses a tier boundary.
   */
  private applyPerformanceProfile(): void {
    if (!this.sigma) return;
    const profile = getGraphPerformanceProfile(this.graph.order, this.graph.size);
    const key = `${profile.isLargeGraph}:${profile.isVeryLargeGraph}`;
    if (key === this.perfTierKey) return;
    this.perfTierKey = key;
    this.sigma.setSetting("labelGridCellSize", profile.labelGridCellSize);
    this.sigma.setSetting("labelRenderedSizeThreshold", profile.labelRenderedSizeThreshold);
    this.sigma.setSetting("labelDensity", profile.labelDensity);
    this.sigma.setSetting("hideEdgesOnMove", profile.hideEdgesOnMove);
    this.sigma.setSetting("hideLabelsOnMove", profile.hideLabelsOnMove);
    this.sigma.setSetting("enableEdgeEvents", !profile.disableEdgeEvents);
  }

  private focusContext(): FocusContext {
    return {
      focus: this.focus,
      filters: this.filters,
      matchingNodeIds: this.matchingNodeIds,
      neighborIds: this.neighborIds,
      highlightNeighbors: this.options.highlightNeighbors,
      focusDepth: this.focus.mode === "select" ? this.egoDepth : 1,
    };
  }

  private applyCommunityColors(): void {
    // Prefer server community_id (SPEC-155 W3); fall back to client Louvain.
    const palette = this.theme.communities.length
      ? this.theme.communities
      : ["#3b82f6", "#10b981", "#f59e0b", "#ef4444", "#8b5cf6", "#06b6d4"];

    let usedServer = 0;
    this.graph.forEachNode((nodeId) => {
      const serverId = this.graph.getNodeAttribute(nodeId, "community_id") as
        | string
        | undefined;
      if (serverId) {
        const color = communityColorFromId(serverId, palette);
        this.graph.setNodeAttribute(nodeId, "color", color);
        this.graph.setNodeAttribute(nodeId, "community", serverId);
        usedServer++;
      }
    });

    if (usedServer > 0) return;
    if (this.graph.order <= 1 || this.graph.size === 0) return;
    try {
      const clusteringResult = detectCommunities(this.graph);
      this.graph.forEachNode((nodeId) => {
        const communityId = clusteringResult.nodeToCommuntiy.get(nodeId);
        if (communityId !== undefined) {
          this.graph.setNodeAttribute(
            nodeId,
            "color",
            getCommunityColor(communityId),
          );
          this.graph.setNodeAttribute(nodeId, "community", communityId);
        }
      });
    } catch (e) {
      console.warn("[GraphEngine] Community detection failed:", e);
    }
  }

  /** Brightness of a node by its hop ring (1 for seed / ring 1 / no focus). */
  private ringFade(nodeId: string): number {
    if (this.focus.mode !== "select" && this.focus.mode !== "ego") return 1;
    return hopFade(this.neighborHops.get(nodeId));
  }

  /**
   * Emphasised edges only carry a label when that stays legible: the hovered
   * edge always does; a node's incident edges only while there are few of them
   * (a hub would otherwise paint dozens of overlapping labels).
   */
  private shouldLabelEdge(edge: string): boolean {
    if (this.hoveredEdgeId === edge) return true;
    const focusId = this.hoveredNodeId ?? this.selectedNodeId;
    if (!focusId || !this.graph.hasNode(focusId)) return false;
    return this.graph.degree(focusId) <= MAX_INCIDENT_EDGE_LABELS;
  }

  private bootstrapSigma(): void {
    const profile = getGraphPerformanceProfile(0, 0);
    const edgeColor = this.theme.edge || (this.options.isDark ? "#4b5563" : "#94a3b8");

    const nodeReducer = (node: string, attrs: Record<string, unknown>) => {
      const ctx = this.focusContext();
      const dimmed = isNodeDimmed(node, ctx);
      const role = resolveNodeStackRole({
        nodeId: node,
        selectedNodeId: this.selectedNodeId,
        hoveredNodeId: this.hoveredNodeId,
        contextTargetId: this.contextTargetId,
        focus: this.focus,
        neighborIds: this.neighborIds,
        dimmed,
      });
      const emphasized = role === "focus" || role === "hover";
      return nodeReducerAttrs(attrs, dimmed, emphasized, this.theme.focus, {
        fade: this.ringFade(node),
        zIndex: nodeStackZIndex(role),
        // Bright set redraws above focusEdges; card only while the pointer is on it.
        highlighted: nodeNeedsHoverLayer(role),
        showHoverCard: this.hoveredNodeId === node,
      });
    };

    const edgeReducer = (edge: string, attrs: Record<string, unknown>) => {
      const ctx = this.focusContext();
      const source = this.graph.source(edge);
      const target = this.graph.target(edge);
      const dimmed = isEdgeDimmed(edge, source, target, ctx);
      const touchesFocus =
        this.hoveredNodeId === source ||
        this.hoveredNodeId === target ||
        this.selectedNodeId === source ||
        this.selectedNodeId === target;
      const emphasized = touchesFocus || this.hoveredEdgeId === edge;
      return edgeReducerAttrs(
        attrs,
        dimmed,
        emphasized,
        this.theme.focus,
        edgeColor,
        {
          showLabel: this.shouldLabelEdge(edge),
          fade: Math.min(this.ringFade(source), this.ringFade(target)),
          zIndex: edgeStackZIndex({
            dimmed,
            emphasized,
            focus: ctx.focus,
            highlightNeighbors: ctx.highlightNeighbors,
          }),
        },
      );
    };

    try {
      const sigma = new Sigma(this.graph, this.container, {
        renderLabels: this.options.showLabels,
        renderEdgeLabels: this.options.showEdgeLabels,
        defaultDrawNodeLabel: drawNodeLabelWithBackground,
        defaultDrawNodeHover: drawNodeHoverWithCard,
        defaultDrawEdgeLabel: drawEdgeLabelWithBackground,
        labelSize: 11,
        labelWeight: "500",
        labelColor: {
          color: labelColorForTheme(this.options.isDark, this.theme),
        },
        labelFont: "var(--font-geist-sans), ui-sans-serif, system-ui, sans-serif",
        edgeLabelSize: 10,
        edgeLabelFont:
          "var(--font-geist-sans), ui-sans-serif, system-ui, sans-serif",
        edgeLabelWeight: "500",
        edgeLabelColor: { color: this.options.isDark ? "#e2e8f0" : "#334155" },
        labelGridCellSize: profile.labelGridCellSize,
        labelRenderedSizeThreshold: profile.labelRenderedSizeThreshold,
        labelDensity: profile.labelDensity,
        hideEdgesOnMove: profile.hideEdgesOnMove,
        hideLabelsOnMove: profile.hideLabelsOnMove,
        defaultNodeColor: "#94a3b8",
        defaultEdgeColor: edgeColor,
        defaultNodeType: "border",
        defaultEdgeType: "arrow",
        nodeProgramClasses: { border: NodeBorderProgram },
        edgeProgramClasses: {
          arrow: EdgeArrowProgram,
          curvedArrow: EdgeCurvedArrowProgram,
          curved: createEdgeCurveProgram(),
        },
        minCameraRatio: 0.05,
        maxCameraRatio: 10,
        enableEdgeEvents: true,
        stagePadding: 50,
        zIndex: true,
        minEdgeThickness: 1.5,
        nodeReducer,
        edgeReducer,
      });
      this.sigma = sigma;
      this.resizeObserver?.disconnect();
      this.resizeObserver = new ResizeObserver(() => {
        sigma.resize();
        sigma.refresh();
      });
      this.resizeObserver.observe(this.container);
      this.bindInteractions(sigma);
      this.focusEdgeOverlay = bindFocusEdgeOverlay(sigma, {
        getColor: () => this.theme.focus,
        isLifted: (edgeId) => {
          const ctx = this.focusContext();
          if (!this.graph.hasEdge(edgeId)) return false;
          const source = this.graph.source(edgeId);
          const target = this.graph.target(edgeId);
          const dimmed = isEdgeDimmed(edgeId, source, target, ctx);
          const touchesFocus =
            this.hoveredNodeId === source ||
            this.hoveredNodeId === target ||
            this.selectedNodeId === source ||
            this.selectedNodeId === target;
          const emphasized =
            touchesFocus || this.hoveredEdgeId === edgeId;
          return isLiftedFocusEdge({
            dimmed,
            emphasized,
            focus: ctx.focus,
            highlightNeighbors: ctx.highlightNeighbors,
          });
        },
      });
    } catch (error) {
      const err =
        error instanceof Error
          ? error
          : new Error("Failed to initialize WebGL graph renderer");
      console.warn("[GraphEngine] Sigma init failed:", error);
      this.sigma = null;
      this.options.onWebglError?.(err);
    }
  }

  private bindInteractions(sigma: Sigma): void {
    this.interactions = bindInteractions(sigma, {
      graph: this.graph,
      container: this.container,
      isDragEnabled: () => this.options.enableNodeDrag,
      onHoverNode: (node) => {
        this.setHover(node);
        this.options.onNodeHover?.(node);
      },
      onHoverEdge: (edge) => this.setHoverEdge(edge),
      onNodeClick: (node) => this.options.onNodeClick?.(node),
      onNodeDoubleClick: (node) => this.options.onNodeDoubleClick?.(node),
      onStageClick: () => this.options.onStageClick?.(),
      onNodeRightClick: (node, x, y) => this.options.onNodeRightClick?.(node, x, y),
    });
  }
}

export function createGraphEngine(
  container: HTMLElement,
  options?: GraphEngineOptions,
): GraphEngine {
  return new GraphEngine(container, options);
}
