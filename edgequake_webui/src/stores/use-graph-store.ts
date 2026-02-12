/**
 * Graph Store - Manages knowledge graph visualization state.
 *
 * @implements UC0101 - Explore Entity Neighborhood
 * @implements UC0104 - View Graph Statistics
 * @implements FEAT0601 - Knowledge Graph Visualization
 * @implements FEAT0202 - Graph Traversal (via API calls)
 * @implements FEAT0205 - Community Detection (color coding)
 *
 * @enforces BR0009 - Max 1000 nodes per visualization (via backend)
 * @enforces BR0201 - Tenant isolation (graph filtered by workspace)
 *
 * @description
 * This Zustand store manages:
 * - Graph data (nodes, edges) with indexed lookups
 * - Sigma.js instance for rendering
 * - Node/edge selection and filtering
 * - Entity type and relationship type visibility
 * - Graph bookmarks for saved views
 * - SSE streaming progress for large graphs
 *
 * @see Sigma.js for graph rendering
 * @see useSettingsStore for graph layout preferences
 */

"use client";

import type { GraphEdge, GraphNode, KnowledgeGraph } from "@/types";
import Sigma from "sigma";
import { create } from "zustand";

/**
 * Maximum number of nodes to display in the graph visualization.
 * WHY: Graphs with >500 nodes become cluttered and slow to render.
 * @enforces BR0009 - Max nodes per visualization for performance
 */
export const MAX_DISPLAY_NODES = 500;

/** Color mode for node coloring strategy */
export type ColorMode = "entity-type" | "community";

// Streaming progress phase
export type StreamingPhase =
  | "idle"
  | "connecting"
  | "metadata"
  | "nodes"
  | "edges"
  | "complete"
  | "error";

// Streaming progress state
export interface StreamingProgress {
  phase: StreamingPhase;
  totalNodes: number;
  nodesLoaded: number;
  batchNumber: number;
  totalBatches: number;
  edgesLoaded: number;
  durationMs: number;
  errorMessage?: string;
}

// Bookmark type for saving graph views
export interface GraphBookmark {
  id: string;
  name: string;
  createdAt: Date;
  // Saved state
  visibleNodeIds: string[];
  cameraState: { x: number; y: number; ratio: number } | null;
  visibleEntityTypes: string[];
  visibleRelationshipTypes: string[];
  searchQuery: string;
  timeFilterEnabled: boolean;
  timeFilterStart: Date | null;
  timeFilterEnd: Date | null;
}

interface GraphState {
  // Graph data
  graph: KnowledgeGraph | null;
  nodes: GraphNode[];
  edges: GraphEdge[];

  // Indexed data structures for O(1) lookups
  nodeMap: Map<string, GraphNode>;
  edgeMap: Map<string, GraphEdge>;
  nodesByType: Map<string, Set<string>>; // type → node IDs
  edgesBySource: Map<string, Set<string>>; // nodeId → edge IDs
  edgesByTarget: Map<string, Set<string>>; // nodeId → edge IDs

  // Selection state
  selectedNodeId: string | null;
  focusedNodeId: string | null;
  hoveredNodeId: string | null;
  selectedNodes: Set<string>;
  showNodeDetails: boolean; // Controls visibility of node details panel
  rightPanelCollapsed: boolean; // Controls visibility of right panel

  // Filter state
  visibleEntityTypes: Set<string>;
  visibleRelationshipTypes: Set<string>;
  searchQuery: string;

  // Time-based filtering
  timeFilterEnabled: boolean;
  timeFilterStart: Date | null;
  timeFilterEnd: Date | null;

  // Display settings
  colorMode: ColorMode;
  showClustering: boolean;

  // Sigma instance reference
  sigmaInstance: Sigma | null;

  // Expand/Prune state
  nodeToExpand: string | null;
  nodeToPrune: string | null;
  isExpanding: boolean;
  isPruning: boolean;
  expandedNodes: Set<string>; // Track which nodes have been expanded

  // Loading state
  isLoading: boolean;
  error: string | null;

  // Bookmarks for saving graph views
  bookmarks: GraphBookmark[];

  // Virtual Query Settings (Phase 6: SOTA 100k+ nodes)
  maxNodes: number; // Max nodes to fetch (default: 500)
  depth: number; // Traversal depth (default: 2)
  startNode: string | null; // Focus on specific node neighborhood

  // Truncation info from server
  isTruncated: boolean;
  totalNodesInStorage: number;
  totalEdgesInStorage: number;

  // Streaming state for progressive loading
  useStreaming: boolean;
  streamingProgress: StreamingProgress;
}

interface GraphActions {
  // Data actions
  setGraph: (graph: KnowledgeGraph) => void;
  clearGraph: () => void;

  // Indexed lookups (O(1) performance)
  getNodeById: (nodeId: string) => GraphNode | undefined;
  getEdgeById: (edgeId: string) => GraphEdge | undefined;
  getNodesByType: (type: string) => GraphNode[];
  getEdgesForNode: (nodeId: string) => GraphEdge[];

  // Selection actions
  selectNode: (nodeId: string | null) => void;
  focusNode: (nodeId: string | null) => void;
  hoverNode: (nodeId: string | null) => void;
  toggleNodeSelection: (nodeId: string) => void;
  toggleNodeDetails: () => void;
  toggleRightPanel: () => void;
  clearSelection: () => void;

  // Filter actions
  toggleEntityType: (type: string) => void;
  toggleRelationshipType: (type: string) => void;
  setVisibleEntityTypes: (types: string[]) => void;
  setVisibleRelationshipTypes: (types: string[]) => void;
  setSearchQuery: (query: string) => void;
  resetFilters: () => void;

  // Time filter actions
  setTimeFilterEnabled: (enabled: boolean) => void;
  setTimeFilterRange: (start: Date | null, end: Date | null) => void;
  clearTimeFilter: () => void;

  // Display settings
  setColorMode: (mode: ColorMode) => void;
  toggleClustering: () => void;

  // Sigma instance
  setSigmaInstance: (sigma: Sigma | null) => void;

  // Expand/Prune actions
  triggerNodeExpand: (nodeId: string | null) => void;
  triggerNodePrune: (nodeId: string | null) => void;
  setIsExpanding: (isExpanding: boolean) => void;
  setIsPruning: (isPruning: boolean) => void;
  addExpandedNode: (nodeId: string) => void;
  removeExpandedNode: (nodeId: string) => void;
  addNodesToGraph: (nodes: GraphNode[], edges: GraphEdge[]) => void;
  removeNodeFromGraph: (nodeId: string) => void;

  // Loading
  setLoading: (loading: boolean) => void;
  setError: (error: string | null) => void;

  // Bookmark actions
  saveBookmark: (name: string) => GraphBookmark | null;
  loadBookmark: (bookmarkId: string) => void;
  deleteBookmark: (bookmarkId: string) => void;
  renameBookmark: (bookmarkId: string, newName: string) => void;

  // Virtual Query actions (Phase 6: SOTA 100k+ nodes)
  setMaxNodes: (maxNodes: number) => void;
  setDepth: (depth: number) => void;
  setStartNode: (nodeId: string | null) => void;
  setTruncationInfo: (
    isTruncated: boolean,
    totalNodes: number,
    totalEdges: number,
  ) => void;

  // Streaming actions for progressive loading
  setUseStreaming: (enabled: boolean) => void;
  setStreamingProgress: (progress: Partial<StreamingProgress>) => void;
  resetStreamingProgress: () => void;
  clearGraphForStreaming: () => void;
}

type GraphStore = GraphState & GraphActions;

const initialState: GraphState = {
  graph: null,
  nodes: [],
  edges: [],
  nodeMap: new Map(),
  edgeMap: new Map(),
  nodesByType: new Map(),
  edgesBySource: new Map(),
  edgesByTarget: new Map(),
  selectedNodeId: null,
  focusedNodeId: null,
  hoveredNodeId: null,
  selectedNodes: new Set(),
  showNodeDetails: true,
  rightPanelCollapsed: false,
  visibleEntityTypes: new Set(),
  visibleRelationshipTypes: new Set(),
  searchQuery: "",
  timeFilterEnabled: false,
  timeFilterStart: null,
  timeFilterEnd: null,
  colorMode: "entity-type",
  showClustering: false,
  sigmaInstance: null,
  nodeToExpand: null,
  nodeToPrune: null,
  isExpanding: false,
  isPruning: false,
  expandedNodes: new Set(),
  isLoading: false,
  error: null,
  bookmarks: [],
  // Virtual Query defaults (Phase 6)
  maxNodes: 200, // Reduced from 500 for faster initial load
  depth: 2,
  startNode: null,
  isTruncated: false,
  totalNodesInStorage: 0,
  totalEdgesInStorage: 0,
  // Streaming enabled - backend SSE verified working
  useStreaming: true,
  streamingProgress: {
    phase: "idle",
    totalNodes: 0,
    nodesLoaded: 0,
    batchNumber: 0,
    totalBatches: 0,
    edgesLoaded: 0,
    durationMs: 0,
  },
};

// Load bookmarks from localStorage
const loadBookmarksFromStorage = (): GraphBookmark[] => {
  if (typeof window === "undefined") return [];
  try {
    const stored = localStorage.getItem("graph-bookmarks");
    if (stored) {
      const parsed = JSON.parse(stored);
      // Restore Date objects
      return parsed.map((b: GraphBookmark) => ({
        ...b,
        createdAt: new Date(b.createdAt),
        timeFilterStart: b.timeFilterStart ? new Date(b.timeFilterStart) : null,
        timeFilterEnd: b.timeFilterEnd ? new Date(b.timeFilterEnd) : null,
      }));
    }
  } catch (e) {
    console.warn("Failed to load bookmarks from localStorage:", e);
  }
  return [];
};

export const useGraphStore = create<GraphStore>()((set, get) => ({
  ...initialState,
  bookmarks: loadBookmarksFromStorage(),

  // Data actions
  setGraph: (graph) => {
    // Deduplicate nodes by ID (keep last occurrence)
    // Also filter out invalid nodes (null/undefined/empty IDs)
    const uniqueNodesMap = new Map<string, GraphNode>();
    let invalidNodeCount = 0;

    for (const node of graph.nodes) {
      // Validate node has a valid ID
      if (!node.id || typeof node.id !== "string" || node.id.trim() === "") {
        console.warn("[GraphStore] Skipping node with invalid ID:", node);
        invalidNodeCount++;
        continue;
      }
      uniqueNodesMap.set(node.id, node);
    }
    const uniqueNodes = Array.from(uniqueNodesMap.values());

    // Log deduplication stats
    const originalNodeCount = graph.nodes.length;
    const deduplicatedCount = originalNodeCount - uniqueNodes.length;
    if (deduplicatedCount > 0) {
      console.warn(
        `[GraphStore] Deduplicated ${deduplicatedCount} duplicate nodes ` +
          `(${originalNodeCount} → ${uniqueNodes.length})`,
      );
    }
    if (invalidNodeCount > 0) {
      console.warn(
        `[GraphStore] Filtered out ${invalidNodeCount} nodes with invalid IDs`,
      );
    }

    // Deduplicate edges by source-target-type combination
    const uniqueEdgesMap = new Map<string, GraphEdge>();
    let invalidEdgeCount = 0;

    for (const edge of graph.edges) {
      // Validate edge has valid source and target
      if (
        !edge.source ||
        !edge.target ||
        typeof edge.source !== "string" ||
        typeof edge.target !== "string" ||
        edge.source.trim() === "" ||
        edge.target.trim() === ""
      ) {
        console.warn(
          "[GraphStore] Skipping edge with invalid source/target:",
          edge,
        );
        invalidEdgeCount++;
        continue;
      }
      const edgeKey = `${edge.source}-${edge.target}-${edge.relationship_type}`;
      uniqueEdgesMap.set(edgeKey, edge);
    }
    const uniqueEdges = Array.from(uniqueEdgesMap.values());

    // Log edge deduplication stats
    const originalEdgeCount = graph.edges.length;
    const deduplicatedEdgeCount = originalEdgeCount - uniqueEdges.length;
    if (deduplicatedEdgeCount > 0) {
      console.warn(
        `[GraphStore] Deduplicated ${deduplicatedEdgeCount} duplicate edges ` +
          `(${originalEdgeCount} → ${uniqueEdges.length})`,
      );
    }
    if (invalidEdgeCount > 0) {
      console.warn(
        `[GraphStore] Filtered out ${invalidEdgeCount} edges with invalid source/target`,
      );
    }

    const entityTypes = new Set(uniqueNodes.map((n) => n.node_type));
    const relationshipTypes = new Set(
      uniqueEdges.map((e) => e.relationship_type),
    );

    // Build indexed data structures for O(1) lookups
    const nodeMap = new Map<string, GraphNode>();
    const edgeMap = new Map<string, GraphEdge>();
    const nodesByType = new Map<string, Set<string>>();
    const edgesBySource = new Map<string, Set<string>>();
    const edgesByTarget = new Map<string, Set<string>>();

    // Index nodes
    for (const node of uniqueNodes) {
      nodeMap.set(node.id, node);

      // Index by type
      if (!nodesByType.has(node.node_type)) {
        nodesByType.set(node.node_type, new Set());
      }
      nodesByType.get(node.node_type)!.add(node.id);
    }

    // Index edges
    for (const edge of uniqueEdges) {
      const edgeId = `${edge.source}-${edge.target}-${edge.relationship_type}`;
      edgeMap.set(edgeId, edge);

      // Index by source
      if (!edgesBySource.has(edge.source)) {
        edgesBySource.set(edge.source, new Set());
      }
      edgesBySource.get(edge.source)!.add(edgeId);

      // Index by target
      if (!edgesByTarget.has(edge.target)) {
        edgesByTarget.set(edge.target, new Set());
      }
      edgesByTarget.get(edge.target)!.add(edgeId);
    }

    set({
      graph: {
        ...graph,
        nodes: uniqueNodes,
        edges: uniqueEdges,
      },
      nodes: uniqueNodes,
      edges: uniqueEdges,
      nodeMap,
      edgeMap,
      nodesByType,
      edgesBySource,
      edgesByTarget,
      visibleEntityTypes: entityTypes,
      visibleRelationshipTypes: relationshipTypes,
      isLoading: false,
      error: null,
    });
  },

  clearGraph: () =>
    set({
      graph: null,
      nodes: [],
      edges: [],
      nodeMap: new Map(),
      edgeMap: new Map(),
      nodesByType: new Map(),
      edgesBySource: new Map(),
      edgesByTarget: new Map(),
      selectedNodeId: null,
      focusedNodeId: null,
      selectedNodes: new Set(),
    }),

  // Indexed lookup helpers (O(1) access)
  getNodeById: (nodeId: string) => get().nodeMap.get(nodeId),

  getEdgeById: (edgeId: string) => get().edgeMap.get(edgeId),

  getNodesByType: (type: string) => {
    const state = get();
    const nodeIds = state.nodesByType.get(type);
    if (!nodeIds) return [];
    return Array.from(nodeIds)
      .map((id) => state.nodeMap.get(id))
      .filter((node): node is GraphNode => node !== undefined);
  },

  getEdgesForNode: (nodeId: string) => {
    const state = get();
    const sourceEdgeIds = state.edgesBySource.get(nodeId) ?? new Set();
    const targetEdgeIds = state.edgesByTarget.get(nodeId) ?? new Set();
    const allEdgeIds = new Set([...sourceEdgeIds, ...targetEdgeIds]);
    return Array.from(allEdgeIds)
      .map((id) => state.edgeMap.get(id))
      .filter((edge): edge is GraphEdge => edge !== undefined);
  },

  // Selection actions
  selectNode: (nodeId) =>
    set({ selectedNodeId: nodeId, showNodeDetails: nodeId !== null }),

  toggleNodeDetails: () =>
    set((state) => ({ showNodeDetails: !state.showNodeDetails })),

  toggleRightPanel: () =>
    set((state) => ({ rightPanelCollapsed: !state.rightPanelCollapsed })),

  focusNode: (nodeId) => {
    set({ focusedNodeId: nodeId });

    // Camera animation if sigma instance exists
    const { sigmaInstance } = get();
    if (sigmaInstance && nodeId) {
      const nodePosition = sigmaInstance.getNodeDisplayData(nodeId);
      if (nodePosition) {
        sigmaInstance.getCamera().animate(
          {
            x: nodePosition.x,
            y: nodePosition.y,
            ratio: 0.5,
          },
          { duration: 500 },
        );
      }
    }
  },

  hoverNode: (nodeId) => set({ hoveredNodeId: nodeId }),

  toggleNodeSelection: (nodeId) =>
    set((state) => {
      const newSelection = new Set(state.selectedNodes);
      if (newSelection.has(nodeId)) {
        newSelection.delete(nodeId);
      } else {
        newSelection.add(nodeId);
      }
      return { selectedNodes: newSelection };
    }),

  clearSelection: () =>
    set({
      selectedNodeId: null,
      selectedNodes: new Set(),
    }),

  // Filter actions
  toggleEntityType: (type) =>
    set((state) => {
      const newTypes = new Set(state.visibleEntityTypes);
      if (newTypes.has(type)) {
        newTypes.delete(type);
      } else {
        newTypes.add(type);
      }
      return { visibleEntityTypes: newTypes };
    }),

  toggleRelationshipType: (type) =>
    set((state) => {
      const newTypes = new Set(state.visibleRelationshipTypes);
      if (newTypes.has(type)) {
        newTypes.delete(type);
      } else {
        newTypes.add(type);
      }
      return { visibleRelationshipTypes: newTypes };
    }),

  setVisibleEntityTypes: (types) => set({ visibleEntityTypes: new Set(types) }),

  setVisibleRelationshipTypes: (types) =>
    set({ visibleRelationshipTypes: new Set(types) }),

  setSearchQuery: (query) => set({ searchQuery: query }),

  resetFilters: () => {
    const { graph } = get();
    if (graph?.metadata) {
      set({
        visibleEntityTypes: new Set(graph.metadata.entity_types || []),
        visibleRelationshipTypes: new Set(
          graph.metadata.relationship_types || [],
        ),
        searchQuery: "",
        timeFilterEnabled: false,
        timeFilterStart: null,
        timeFilterEnd: null,
      });
    }
  },

  // Time filter actions
  setTimeFilterEnabled: (enabled) => set({ timeFilterEnabled: enabled }),

  setTimeFilterRange: (start, end) =>
    set({
      timeFilterStart: start,
      timeFilterEnd: end,
      timeFilterEnabled: start !== null || end !== null,
    }),

  clearTimeFilter: () =>
    set({
      timeFilterEnabled: false,
      timeFilterStart: null,
      timeFilterEnd: null,
    }),

  // Display settings
  setColorMode: (mode) => set({ colorMode: mode }),
  toggleClustering: () =>
    set((state) => ({
      showClustering: !state.showClustering,
      colorMode: state.showClustering ? "entity-type" : "community",
    })),

  // Sigma instance
  setSigmaInstance: (sigma) => set({ sigmaInstance: sigma }),

  // Expand/Prune actions
  triggerNodeExpand: (nodeId) => set({ nodeToExpand: nodeId }),

  triggerNodePrune: (nodeId) => set({ nodeToPrune: nodeId }),

  setIsExpanding: (isExpanding) => set({ isExpanding }),

  setIsPruning: (isPruning) => set({ isPruning }),

  addExpandedNode: (nodeId) =>
    set((state) => {
      const newExpandedNodes = new Set(state.expandedNodes);
      newExpandedNodes.add(nodeId);
      return { expandedNodes: newExpandedNodes };
    }),

  removeExpandedNode: (nodeId) =>
    set((state) => {
      const newExpandedNodes = new Set(state.expandedNodes);
      newExpandedNodes.delete(nodeId);
      return { expandedNodes: newExpandedNodes };
    }),

  addNodesToGraph: (newNodes, newEdges) =>
    set((state) => {
      // Create sets of existing IDs for quick lookup
      const existingNodeIds = new Set(state.nodes.map((n) => n.id));
      const existingEdgeIds = new Set(
        state.edges.map(
          (e) => `${e.source}-${e.target}-${e.relationship_type}`,
        ),
      );

      // Filter out duplicates
      const nodesToAdd = newNodes.filter((n) => !existingNodeIds.has(n.id));
      const edgesToAdd = newEdges.filter(
        (e) =>
          !existingEdgeIds.has(
            `${e.source}-${e.target}-${e.relationship_type}`,
          ),
      );

      // Update entity types if needed
      const newEntityTypes = new Set(state.visibleEntityTypes);
      nodesToAdd.forEach((n) => newEntityTypes.add(n.node_type));

      // Update relationship types if needed
      const newRelationshipTypes = new Set(state.visibleRelationshipTypes);
      edgesToAdd.forEach((e) => newRelationshipTypes.add(e.relationship_type));

      // Update indexed data structures
      const nodeMap = new Map(state.nodeMap);
      const edgeMap = new Map(state.edgeMap);
      const nodesByType = new Map(state.nodesByType);
      const edgesBySource = new Map(state.edgesBySource);
      const edgesByTarget = new Map(state.edgesByTarget);

      // Index new nodes
      for (const node of nodesToAdd) {
        nodeMap.set(node.id, node);
        const nodeType = node.node_type || "unknown";
        if (!nodesByType.has(nodeType)) {
          nodesByType.set(nodeType, new Set());
        }
        nodesByType.get(nodeType)!.add(node.id);
      }

      // Index new edges
      for (const edge of edgesToAdd) {
        const edgeId = `${edge.source}-${edge.target}-${edge.relationship_type}`;
        edgeMap.set(edgeId, edge);
        if (!edgesBySource.has(edge.source)) {
          edgesBySource.set(edge.source, new Set());
        }
        edgesBySource.get(edge.source)!.add(edgeId);
        if (!edgesByTarget.has(edge.target)) {
          edgesByTarget.set(edge.target, new Set());
        }
        edgesByTarget.get(edge.target)!.add(edgeId);
      }

      return {
        nodes: [...state.nodes, ...nodesToAdd],
        edges: [...state.edges, ...edgesToAdd],
        nodeMap,
        edgeMap,
        nodesByType,
        edgesBySource,
        edgesByTarget,
        visibleEntityTypes: newEntityTypes,
        visibleRelationshipTypes: newRelationshipTypes,
      };
    }),

  removeNodeFromGraph: (nodeId) =>
    set((state) => {
      // Find the node to get its type for index cleanup
      const nodeToRemove = state.nodeMap.get(nodeId);

      // Remove the node
      const nodes = state.nodes.filter((n) => n.id !== nodeId);

      // Find edges to remove (those connected to this node)
      const edgesToRemove = state.edges.filter(
        (e) => e.source === nodeId || e.target === nodeId,
      );

      // Remove all edges connected to this node
      const edges = state.edges.filter(
        (e) => e.source !== nodeId && e.target !== nodeId,
      );

      // Update indexed data structures
      const nodeMap = new Map(state.nodeMap);
      const edgeMap = new Map(state.edgeMap);
      const nodesByType = new Map(state.nodesByType);
      const edgesBySource = new Map(state.edgesBySource);
      const edgesByTarget = new Map(state.edgesByTarget);

      // Remove node from indexes
      nodeMap.delete(nodeId);
      if (nodeToRemove) {
        const nodeType = nodeToRemove.node_type || "unknown";
        const typeSet = nodesByType.get(nodeType);
        if (typeSet) {
          typeSet.delete(nodeId);
          if (typeSet.size === 0) {
            nodesByType.delete(nodeType);
          }
        }
      }

      // Remove edges from indexes
      for (const edge of edgesToRemove) {
        const edgeId = `${edge.source}-${edge.target}-${edge.relationship_type}`;
        edgeMap.delete(edgeId);

        const sourceSet = edgesBySource.get(edge.source);
        if (sourceSet) {
          sourceSet.delete(edgeId);
          if (sourceSet.size === 0) {
            edgesBySource.delete(edge.source);
          }
        }

        const targetSet = edgesByTarget.get(edge.target);
        if (targetSet) {
          targetSet.delete(edgeId);
          if (targetSet.size === 0) {
            edgesByTarget.delete(edge.target);
          }
        }
      }

      // Clear selection if the removed node was selected
      const selectedNodeId =
        state.selectedNodeId === nodeId ? null : state.selectedNodeId;

      // Update selected nodes set
      const selectedNodes = new Set(state.selectedNodes);
      selectedNodes.delete(nodeId);

      // Remove from expanded nodes
      const expandedNodes = new Set(state.expandedNodes);
      expandedNodes.delete(nodeId);

      return {
        nodes,
        edges,
        nodeMap,
        edgeMap,
        nodesByType,
        edgesBySource,
        edgesByTarget,
        selectedNodeId,
        selectedNodes,
        expandedNodes,
        showNodeDetails: selectedNodeId !== null,
      };
    }),

  // Loading
  setLoading: (loading) => set({ isLoading: loading }),
  setError: (error) => set({ error, isLoading: false }),

  // Bookmark actions
  saveBookmark: (name: string) => {
    const state = get();
    if (!state.sigmaInstance) return null;

    const camera = state.sigmaInstance.getCamera();
    const cameraState = {
      x: camera.x,
      y: camera.y,
      ratio: camera.ratio,
    };

    const bookmark: GraphBookmark = {
      id: `bookmark-${Date.now()}`,
      name,
      createdAt: new Date(),
      visibleNodeIds: state.nodes.map((n) => n.id),
      cameraState,
      visibleEntityTypes: Array.from(state.visibleEntityTypes),
      visibleRelationshipTypes: Array.from(state.visibleRelationshipTypes),
      searchQuery: state.searchQuery,
      timeFilterEnabled: state.timeFilterEnabled,
      timeFilterStart: state.timeFilterStart,
      timeFilterEnd: state.timeFilterEnd,
    };

    // Save to localStorage
    const updatedBookmarks = [...state.bookmarks, bookmark];
    try {
      localStorage.setItem("graph-bookmarks", JSON.stringify(updatedBookmarks));
    } catch (e) {
      console.warn("Failed to save bookmarks to localStorage:", e);
    }

    set({ bookmarks: updatedBookmarks });
    return bookmark;
  },

  loadBookmark: (bookmarkId: string) => {
    const state = get();
    const bookmark = state.bookmarks.find((b) => b.id === bookmarkId);
    if (!bookmark) return;

    // Restore filters
    set({
      visibleEntityTypes: new Set(bookmark.visibleEntityTypes),
      visibleRelationshipTypes: new Set(bookmark.visibleRelationshipTypes),
      searchQuery: bookmark.searchQuery,
      timeFilterEnabled: bookmark.timeFilterEnabled,
      timeFilterStart: bookmark.timeFilterStart,
      timeFilterEnd: bookmark.timeFilterEnd,
    });

    // Restore camera position
    if (bookmark.cameraState && state.sigmaInstance) {
      state.sigmaInstance.getCamera().setState({
        x: bookmark.cameraState.x,
        y: bookmark.cameraState.y,
        ratio: bookmark.cameraState.ratio,
      });
    }
  },

  deleteBookmark: (bookmarkId: string) => {
    const state = get();
    const updatedBookmarks = state.bookmarks.filter((b) => b.id !== bookmarkId);

    try {
      localStorage.setItem("graph-bookmarks", JSON.stringify(updatedBookmarks));
    } catch (e) {
      console.warn("Failed to save bookmarks to localStorage:", e);
    }

    set({ bookmarks: updatedBookmarks });
  },

  renameBookmark: (bookmarkId: string, newName: string) => {
    const state = get();
    const updatedBookmarks = state.bookmarks.map((b) =>
      b.id === bookmarkId ? { ...b, name: newName } : b,
    );

    try {
      localStorage.setItem("graph-bookmarks", JSON.stringify(updatedBookmarks));
    } catch (e) {
      console.warn("Failed to save bookmarks to localStorage:", e);
    }

    set({ bookmarks: updatedBookmarks });
  },

  // Virtual Query actions (Phase 6: SOTA 100k+ nodes)
  setMaxNodes: (maxNodes: number) => {
    // Persist to localStorage for session persistence
    try {
      localStorage.setItem("graph-max-nodes", String(maxNodes));
    } catch (e) {
      console.warn("Failed to save maxNodes to localStorage:", e);
    }
    set({ maxNodes });
  },

  setDepth: (depth: number) => {
    try {
      localStorage.setItem("graph-depth", String(depth));
    } catch (e) {
      console.warn("Failed to save depth to localStorage:", e);
    }
    set({ depth });
  },

  setStartNode: (nodeId: string | null) => {
    set({ startNode: nodeId });
  },

  setTruncationInfo: (
    isTruncated: boolean,
    totalNodes: number,
    totalEdges: number,
  ) => {
    set({
      isTruncated,
      totalNodesInStorage: totalNodes,
      totalEdgesInStorage: totalEdges,
    });
  },

  // Streaming actions for progressive loading
  setUseStreaming: (enabled: boolean) => {
    try {
      localStorage.setItem("graph-use-streaming", String(enabled));
    } catch (e) {
      console.warn("Failed to save streaming preference to localStorage:", e);
    }
    set({ useStreaming: enabled });
  },

  setStreamingProgress: (progress: Partial<StreamingProgress>) => {
    set((state) => ({
      streamingProgress: {
        ...state.streamingProgress,
        ...progress,
      },
    }));
  },

  resetStreamingProgress: () => {
    set({
      streamingProgress: {
        phase: "idle",
        totalNodes: 0,
        nodesLoaded: 0,
        batchNumber: 0,
        totalBatches: 0,
        edgesLoaded: 0,
        durationMs: 0,
        errorMessage: undefined,
      },
    });
  },

  clearGraphForStreaming: () => {
    // WHY: Clear ALL graph data including metadata and filter state.
    // Previously only nodes/edges were cleared but visibleEntityTypes,
    // visibleRelationshipTypes, and graph metadata from the previous
    // workspace/query persisted — causing stale legends, stale entity
    // type pills, and stale metadata display.
    set({
      graph: null,
      nodes: [],
      edges: [],
      nodeMap: new Map(),
      edgeMap: new Map(),
      nodesByType: new Map(),
      edgesBySource: new Map(),
      edgesByTarget: new Map(),
      visibleEntityTypes: new Set<string>(),
      visibleRelationshipTypes: new Set<string>(),
      selectedNodeId: null,
      focusedNodeId: null,
      selectedNodes: new Set<string>(),
      expandedNodes: new Set<string>(),
      isTruncated: false,
      totalNodesInStorage: 0,
      totalEdgesInStorage: 0,
    });
  },
}));

// Selectors - these return new arrays on each call, so use with useMemo in components
export const useFilteredNodes = () => {
  const nodes = useGraphStore((state) => state.nodes);
  const visibleEntityTypes = useGraphStore((state) => state.visibleEntityTypes);
  const searchQuery = useGraphStore((state) => state.searchQuery);
  const timeFilterEnabled = useGraphStore((state) => state.timeFilterEnabled);
  const timeFilterStart = useGraphStore((state) => state.timeFilterStart);
  const timeFilterEnd = useGraphStore((state) => state.timeFilterEnd);

  // Filter nodes based on visibility, search query, and time range
  return nodes.filter((node) => {
    if (!visibleEntityTypes.has(node.node_type)) return false;

    // Time-based filtering
    if (timeFilterEnabled && node.created_at) {
      const nodeDate = new Date(node.created_at);
      if (timeFilterStart && nodeDate < timeFilterStart) return false;
      if (timeFilterEnd && nodeDate > timeFilterEnd) return false;
    }

    if (searchQuery) {
      const query = searchQuery.toLowerCase();
      return (
        node.label.toLowerCase().includes(query) ||
        node.description?.toLowerCase().includes(query)
      );
    }
    return true;
  });
};

export const useFilteredEdges = () => {
  const edges = useGraphStore((state) => state.edges);
  const nodes = useGraphStore((state) => state.nodes);
  const visibleEntityTypes = useGraphStore((state) => state.visibleEntityTypes);
  const visibleRelationshipTypes = useGraphStore(
    (state) => state.visibleRelationshipTypes,
  );
  const searchQuery = useGraphStore((state) => state.searchQuery);
  const timeFilterEnabled = useGraphStore((state) => state.timeFilterEnabled);
  const timeFilterStart = useGraphStore((state) => state.timeFilterStart);
  const timeFilterEnd = useGraphStore((state) => state.timeFilterEnd);

  // Compute filtered node IDs (with time filtering)
  const nodeIds = new Set(
    nodes
      .filter((node) => {
        if (!visibleEntityTypes.has(node.node_type)) return false;

        // Time-based filtering
        if (timeFilterEnabled && node.created_at) {
          const nodeDate = new Date(node.created_at);
          if (timeFilterStart && nodeDate < timeFilterStart) return false;
          if (timeFilterEnd && nodeDate > timeFilterEnd) return false;
        }

        if (searchQuery) {
          const query = searchQuery.toLowerCase();
          return (
            node.label.toLowerCase().includes(query) ||
            node.description?.toLowerCase().includes(query)
          );
        }
        return true;
      })
      .map((n) => n.id),
  );

  return edges.filter((edge) => {
    if (!visibleRelationshipTypes.has(edge.relationship_type)) return false;
    return nodeIds.has(edge.source) && nodeIds.has(edge.target);
  });
};

export default useGraphStore;
