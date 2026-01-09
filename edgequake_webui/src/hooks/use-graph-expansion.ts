"use client";

/**
 * @module use-graph-expansion
 * @description Hook for expanding graph nodes by fetching neighbors.
 * Uses ForceAtlas2 layout for node positioning with animation.
 *
 * @implements UC0305 - User expands entity to see neighbors
 * @implements FEAT0637 - Node expansion with neighbor fetching
 * @implements FEAT0638 - ForceAtlas2 layout for expanded nodes
 *
 * @enforces BR0623 - Max expansion depth to prevent infinite graphs
 * @enforces BR0624 - Deduplicate edges on expansion
 */

import { getEntityNeighborhood } from "@/lib/api/edgequake";
import { useGraphStore } from "@/stores/use-graph-store";
import type { GraphEdge } from "@/types";
import forceAtlas2 from "graphology-layout-forceatlas2";
import { useCallback, useEffect, useRef } from "react";
import { useTranslation } from "react-i18next";
import { animateNodes } from "sigma/utils";
import { toast } from "sonner";

// Color palette for entity types (same as graph-renderer)
const TYPE_COLORS: Record<string, string> = {
  PERSON: "#3b82f6",
  ORGANIZATION: "#10b981",
  LOCATION: "#f59e0b",
  EVENT: "#ef4444",
  CONCEPT: "#8b5cf6",
  DOCUMENT: "#6366f1",
  DEFAULT: "#64748b",
};

function getNodeColor(entityType: string | undefined): string {
  if (!entityType) return TYPE_COLORS.DEFAULT;
  return TYPE_COLORS[entityType.toUpperCase()] || TYPE_COLORS.DEFAULT;
}

/**
 * Hook to manage graph node expansion and pruning
 *
 * This hook listens to nodeToExpand and nodeToPrune state changes
 * and performs the appropriate graph operations:
 * - Expand: Fetches neighborhood data and adds nodes/edges to the graph
 * - Prune: Removes node and its orphaned neighbors from the graph
 */
export function useGraphExpansion() {
  const { t } = useTranslation();

  // Get state and actions from the store
  const nodeToExpand = useGraphStore((s) => s.nodeToExpand);
  const nodeToPrune = useGraphStore((s) => s.nodeToPrune);
  const sigmaInstance = useGraphStore((s) => s.sigmaInstance);
  const nodes = useGraphStore((s) => s.nodes);
  const expandedNodes = useGraphStore((s) => s.expandedNodes);

  // Actions
  const triggerNodeExpand = useGraphStore((s) => s.triggerNodeExpand);
  const triggerNodePrune = useGraphStore((s) => s.triggerNodePrune);
  const setIsExpanding = useGraphStore((s) => s.setIsExpanding);
  const setIsPruning = useGraphStore((s) => s.setIsPruning);
  const addExpandedNode = useGraphStore((s) => s.addExpandedNode);
  const addNodesToGraph = useGraphStore((s) => s.addNodesToGraph);
  const removeNodeFromGraph = useGraphStore((s) => s.removeNodeFromGraph);
  const clearSelection = useGraphStore((s) => s.clearSelection);

  // Prevent multiple simultaneous operations
  const isOperatingRef = useRef(false);

  /**
   * Handle node expansion
   * Fetches neighborhood data and adds new nodes/edges to the graph
   */
  const handleNodeExpand = useCallback(
    async (nodeId: string) => {
      if (isOperatingRef.current) return;
      isOperatingRef.current = true;
      setIsExpanding(true);

      try {
        // Check if already expanded
        if (expandedNodes.has(nodeId)) {
          toast.info(
            t("graph.expand.alreadyExpanded", "Node already expanded")
          );
          return;
        }

        // Fetch neighborhood data from API (depth 1 for immediate neighbors)
        const neighborhood = await getEntityNeighborhood(nodeId, 1);

        if (!neighborhood.nodes?.length) {
          toast.info(t("graph.expand.noNeighbors", "No new neighbors found"));
          return;
        }

        // Get the sigma graph instance
        const sigmaGraph = sigmaInstance?.getGraph();
        if (!sigmaGraph) {
          console.error("No sigma graph available");
          return;
        }

        // Get existing node IDs
        const existingNodeIds = new Set(nodes.map((n) => n.id));

        // Find the position of the node being expanded
        let expandX = 0;
        let expandY = 0;
        try {
          expandX = sigmaGraph.getNodeAttribute(nodeId, "x") ?? 0;
          expandY = sigmaGraph.getNodeAttribute(nodeId, "y") ?? 0;
        } catch {
          // Node might not be in sigma graph yet
        }

        // Filter out existing nodes
        const newNodes = neighborhood.nodes.filter(
          (n) => !existingNodeIds.has(n.id)
        );

        if (newNodes.length === 0) {
          toast.info(
            t("graph.expand.noNewNodes", "All neighbors already in graph")
          );
          addExpandedNode(nodeId);
          return;
        }

        // Filter edges to only include those connecting to the graph
        const allNodeIds = new Set([
          ...existingNodeIds,
          ...newNodes.map((n) => n.id),
        ]);
        const validEdges = neighborhood.edges.filter(
          (e) => allNodeIds.has(e.source) && allNodeIds.has(e.target)
        );

        // Add new nodes to sigma graph with positions around the expanded node
        const spreadFactor = Math.max(50, Math.sqrt(newNodes.length) * 30);

        newNodes.forEach((node, index) => {
          const angle = (2 * Math.PI * index) / newNodes.length;
          const x = expandX + Math.cos(angle) * spreadFactor;
          const y = expandY + Math.sin(angle) * spreadFactor;

          try {
            if (!sigmaGraph.hasNode(node.id)) {
              sigmaGraph.addNode(node.id, {
                label: node.label,
                x,
                y,
                size: 10,
                color: getNodeColor(node.node_type),
                borderColor: "#ffffff",
                borderSize: 0.15,
                entityType: node.node_type,
                description: node.description,
              });
            }
          } catch (error) {
            console.error("Error adding node to sigma graph:", error);
          }
        });

        // Add new edges to sigma graph
        validEdges.forEach((edge) => {
          try {
            if (
              sigmaGraph.hasNode(edge.source) &&
              sigmaGraph.hasNode(edge.target) &&
              !sigmaGraph.hasEdge(edge.source, edge.target)
            ) {
              sigmaGraph.addEdge(edge.source, edge.target, {
                label: edge.relationship_type,
                size: Math.max(1, Math.min((edge.weight || 1) * 2, 5)),
                color: "#4b5563",
                type: "curvedArrow",
                curvature: 0.25,
              });
            }
          } catch {
            // Edge might already exist
          }
        });

        // Run local ForceAtlas2 to settle the new nodes
        const sensibleSettings = forceAtlas2.inferSettings(sigmaGraph);
        forceAtlas2.assign(sigmaGraph, {
          iterations: 50,
          settings: {
            ...sensibleSettings,
            gravity: 1,
            scalingRatio: 2,
            strongGravityMode: true,
            barnesHutOptimize: sigmaGraph.order > 100,
          },
        });

        // Animate nodes to their new positions
        const newPositions: Record<string, { x: number; y: number }> = {};
        sigmaGraph.forEachNode((nId) => {
          newPositions[nId] = {
            x: sigmaGraph.getNodeAttribute(nId, "x"),
            y: sigmaGraph.getNodeAttribute(nId, "y"),
          };
        });
        animateNodes(sigmaGraph, newPositions, {
          duration: 300,
          easing: "quadraticInOut",
        });

        // Update the store with new nodes and edges
        addNodesToGraph(newNodes, validEdges as GraphEdge[]);
        addExpandedNode(nodeId);

        toast.success(
          t("graph.expand.success", "Added {{count}} new nodes", {
            count: newNodes.length,
          })
        );
      } catch (error) {
        console.error("Error expanding node:", error);
        toast.error(t("graph.expand.error", "Failed to expand node"));
      } finally {
        setIsExpanding(false);
        triggerNodeExpand(null);
        isOperatingRef.current = false;
      }
    },
    [
      t,
      sigmaInstance,
      nodes,
      expandedNodes,
      setIsExpanding,
      triggerNodeExpand,
      addExpandedNode,
      addNodesToGraph,
    ]
  );

  /**
   * Handle node pruning
   * Removes the node and any orphaned neighbors from the graph
   */
  const handleNodePrune = useCallback(
    (nodeId: string) => {
      if (isOperatingRef.current) return;
      isOperatingRef.current = true;
      setIsPruning(true);

      try {
        const sigmaGraph = sigmaInstance?.getGraph();
        if (!sigmaGraph) {
          console.error("No sigma graph available");
          return;
        }

        // Check if this is the last node
        if (sigmaGraph.order === 1) {
          toast.error(
            t("graph.prune.cannotDeleteLast", "Cannot delete the last node")
          );
          return;
        }

        // Find nodes that would become orphaned (only connected to the pruned node)
        const nodesToDelete = new Set<string>([nodeId]);

        sigmaGraph.forEachNode((nId) => {
          if (nId === nodeId) return;

          const neighbors = sigmaGraph.neighbors(nId);
          // If this node only has the pruned node as a neighbor, it will be orphaned
          if (neighbors.length === 1 && neighbors[0] === nodeId) {
            nodesToDelete.add(nId);
          }
        });

        // Clear selection first
        clearSelection();

        // Remove nodes from sigma graph
        nodesToDelete.forEach((nId) => {
          try {
            if (sigmaGraph.hasNode(nId)) {
              sigmaGraph.dropNode(nId);
            }
          } catch (error) {
            console.error("Error removing node from sigma graph:", error);
          }
        });

        // Remove from store
        nodesToDelete.forEach((nId) => {
          removeNodeFromGraph(nId);
        });

        toast.success(
          t("graph.prune.success", "Removed {{count}} nodes", {
            count: nodesToDelete.size,
          })
        );
      } catch (error) {
        console.error("Error pruning node:", error);
        toast.error(t("graph.prune.error", "Failed to prune node"));
      } finally {
        setIsPruning(false);
        triggerNodePrune(null);
        isOperatingRef.current = false;
      }
    },
    [
      t,
      sigmaInstance,
      setIsPruning,
      triggerNodePrune,
      clearSelection,
      removeNodeFromGraph,
    ]
  );

  // Listen for expand trigger
  useEffect(() => {
    if (nodeToExpand) {
      handleNodeExpand(nodeToExpand);
    }
  }, [nodeToExpand, handleNodeExpand]);

  // Listen for prune trigger
  useEffect(() => {
    if (nodeToPrune) {
      handleNodePrune(nodeToPrune);
    }
  }, [nodeToPrune, handleNodePrune]);

  return {
    isExpanding: useGraphStore((s) => s.isExpanding),
    isPruning: useGraphStore((s) => s.isPruning),
    expandedNodes: useGraphStore((s) => s.expandedNodes),
  };
}

export default useGraphExpansion;
