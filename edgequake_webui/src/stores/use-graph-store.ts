"use client";

import type { GraphEdge, GraphNode, KnowledgeGraph } from "@/types";
import Sigma from "sigma";
import { create } from "zustand";

export type ColorMode = 'entity-type' | 'community';

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

  // Filter state
  visibleEntityTypes: Set<string>;
  visibleRelationshipTypes: Set<string>;
  searchQuery: string;

  // Display settings
  colorMode: ColorMode;
  showClustering: boolean;

  // Sigma instance reference
  sigmaInstance: Sigma | null;

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
  visibleEntityTypes: new Set(),
  visibleRelationshipTypes: new Set(),
  searchQuery: "",
  colorMode: 'entity-type',
  showClustering: false,
  sigmaInstance: null,
  isLoading: false,
  error: null,
};

export const useGraphStore = create<GraphStore>()((set, get) => ({
  ...initialState,

  // Data actions
  setGraph: (graph) => {
    const entityTypes = new Set(graph.nodes.map((n) => n.entity_type));
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
  selectNode: (nodeId) => set({ selectedNodeId: nodeId }),

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
    if (graph) {
      set({
        visibleEntityTypes: new Set(graph.metadata.entity_types),
        visibleRelationshipTypes: new Set(graph.metadata.relationship_types),
        searchQuery: "",
      });
    }
  },

  // Display settings
  setColorMode: (mode) => set({ colorMode: mode }),
  toggleClustering: () => set((state) => ({ 
    showClustering: !state.showClustering,
    colorMode: state.showClustering ? 'entity-type' : 'community',
  })),

  // Sigma instance
  setSigmaInstance: (sigma) => set({ sigmaInstance: sigma }),

  // Loading
  setLoading: (loading) => set({ isLoading: loading }),
  setError: (error) => set({ error, isLoading: false }),
}));

// Selectors
export const useFilteredNodes = () => {
  const { nodes, visibleEntityTypes, searchQuery } = useGraphStore();

  return nodes.filter((node) => {
    if (!visibleEntityTypes.has(node.entity_type)) return false;
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
  const { edges, visibleRelationshipTypes } = useGraphStore();
  const filteredNodes = useFilteredNodes();
  const nodeIds = new Set(filteredNodes.map((n) => n.id));

  return edges.filter((edge) => {
    if (!visibleRelationshipTypes.has(edge.relationship_type)) return false;
    return nodeIds.has(edge.source) && nodeIds.has(edge.target);
  });
};

export default useGraphStore;
