"use client";

import type { GraphEdge, GraphNode, KnowledgeGraph } from "@/types";
import Sigma from "sigma";
import { create } from "zustand";

export type ColorMode = "entity-type" | "community";

interface GraphState {
  // Graph data
  graph: KnowledgeGraph | null;
  nodes: GraphNode[];
  edges: GraphEdge[];

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
}

interface GraphActions {
  // Data actions
  setGraph: (graph: KnowledgeGraph) => void;
  clearGraph: () => void;

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
}

type GraphStore = GraphState & GraphActions;

const initialState: GraphState = {
  graph: null,
  nodes: [],
  edges: [],
  selectedNodeId: null,
  focusedNodeId: null,
  hoveredNodeId: null,
  selectedNodes: new Set(),
  showNodeDetails: true,
  rightPanelCollapsed: false,
  visibleEntityTypes: new Set(),
  visibleRelationshipTypes: new Set(),
  searchQuery: "",
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
};

export const useGraphStore = create<GraphStore>()((set, get) => ({
  ...initialState,

  // Data actions
  setGraph: (graph) => {
    const entityTypes = new Set(graph.nodes.map((n) => n.node_type));
    const relationshipTypes = new Set(
      graph.edges.map((e) => e.relationship_type)
    );

    set({
      graph,
      nodes: graph.nodes,
      edges: graph.edges,
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
      selectedNodeId: null,
      focusedNodeId: null,
      selectedNodes: new Set(),
    }),

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
          { duration: 500 }
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
          graph.metadata.relationship_types || []
        ),
        searchQuery: "",
      });
    }
  },

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
        state.edges.map((e) => `${e.source}-${e.target}-${e.relationship_type}`)
      );

      // Filter out duplicates
      const nodesToAdd = newNodes.filter((n) => !existingNodeIds.has(n.id));
      const edgesToAdd = newEdges.filter(
        (e) => !existingEdgeIds.has(`${e.source}-${e.target}-${e.relationship_type}`)
      );

      // Update entity types if needed
      const newEntityTypes = new Set(state.visibleEntityTypes);
      nodesToAdd.forEach((n) => newEntityTypes.add(n.node_type));

      // Update relationship types if needed
      const newRelationshipTypes = new Set(state.visibleRelationshipTypes);
      edgesToAdd.forEach((e) => newRelationshipTypes.add(e.relationship_type));

      return {
        nodes: [...state.nodes, ...nodesToAdd],
        edges: [...state.edges, ...edgesToAdd],
        visibleEntityTypes: newEntityTypes,
        visibleRelationshipTypes: newRelationshipTypes,
      };
    }),

  removeNodeFromGraph: (nodeId) =>
    set((state) => {
      // Remove the node
      const nodes = state.nodes.filter((n) => n.id !== nodeId);
      
      // Remove all edges connected to this node
      const edges = state.edges.filter(
        (e) => e.source !== nodeId && e.target !== nodeId
      );
      
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
        selectedNodeId,
        selectedNodes,
        expandedNodes,
        showNodeDetails: selectedNodeId !== null,
      };
    }),

  // Loading
  setLoading: (loading) => set({ isLoading: loading }),
  setError: (error) => set({ error, isLoading: false }),
}));

// Selectors - these return new arrays on each call, so use with useMemo in components
export const useFilteredNodes = () => {
  const nodes = useGraphStore((state) => state.nodes);
  const visibleEntityTypes = useGraphStore((state) => state.visibleEntityTypes);
  const searchQuery = useGraphStore((state) => state.searchQuery);

  // Filter nodes based on visibility and search query
  return nodes.filter((node) => {
    if (!visibleEntityTypes.has(node.node_type)) return false;
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
    (state) => state.visibleRelationshipTypes
  );
  const searchQuery = useGraphStore((state) => state.searchQuery);

  // Compute filtered node IDs
  const nodeIds = new Set(
    nodes
      .filter((node) => {
        if (!visibleEntityTypes.has(node.node_type)) return false;
        if (searchQuery) {
          const query = searchQuery.toLowerCase();
          return (
            node.label.toLowerCase().includes(query) ||
            node.description?.toLowerCase().includes(query)
          );
        }
        return true;
      })
      .map((n) => n.id)
  );

  return edges.filter((edge) => {
    if (!visibleRelationshipTypes.has(edge.relationship_type)) return false;
    return nodeIds.has(edge.source) && nodeIds.has(edge.target);
  });
};

export default useGraphStore;
