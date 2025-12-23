'use client';

import { detectCommunities, getCommunityColor } from '@/lib/graph/clustering';
import { useGraphStore } from '@/stores/use-graph-store';
import type { GraphEdge, GraphNode } from '@/types';
import Graph from 'graphology';
import forceAtlas2 from 'graphology-layout-forceatlas2';
import { useCallback, useEffect, useRef } from 'react';
import Sigma from 'sigma';

// Color palette for entity types
const TYPE_COLORS: Record<string, string> = {
  PERSON: '#3b82f6',
  ORGANIZATION: '#10b981',
  LOCATION: '#f59e0b',
  EVENT: '#ef4444',
  CONCEPT: '#8b5cf6',
  DOCUMENT: '#6366f1',
  DEFAULT: '#64748b',
};

function getNodeColor(entityType: string | undefined): string {
  if (!entityType) return TYPE_COLORS.DEFAULT;
  return TYPE_COLORS[entityType.toUpperCase()] || TYPE_COLORS.DEFAULT;
}

interface GraphRendererProps {
  nodes: GraphNode[];
  edges: GraphEdge[];
  onNodeClick?: (nodeId: string) => void;
  onNodeHover?: (nodeId: string | null) => void;
  onNodeRightClick?: (nodeId: string, x: number, y: number) => void;
}

export function GraphRenderer({ nodes, edges, onNodeClick, onNodeHover, onNodeRightClick }: GraphRendererProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const sigmaRef = useRef<Sigma | null>(null);
  const setSigmaInstance = useGraphStore((s) => s.setSigmaInstance);
  const colorMode = useGraphStore((s) => s.colorMode);

  const initializeGraph = useCallback(() => {
    if (!containerRef.current || nodes.length === 0) return;

    // Cleanup previous instance
    if (sigmaRef.current) {
      sigmaRef.current.kill();
      sigmaRef.current = null;
    }

    // Create graphology graph
    const graph = new Graph();

    // Add nodes
    nodes.forEach((node, index) => {
      const angle = (2 * Math.PI * index) / nodes.length;
      const radius = 100;
      
      graph.addNode(node.id, {
        label: node.label,
        x: Math.cos(angle) * radius,
        y: Math.sin(angle) * radius,
        size: 10,
        color: getNodeColor(node.node_type),
        entityType: node.node_type,
        description: node.description,
      });
    });

    // Add edges
    edges.forEach((edge) => {
      if (graph.hasNode(edge.source) && graph.hasNode(edge.target)) {
        try {
          graph.addEdge(edge.source, edge.target, {
            label: edge.relationship_type,
            size: Math.max(1, edge.weight * 2),
            color: '#94a3b8',
            type: 'arrow',
          });
        } catch {
          // Edge already exists or invalid
        }
      }
    });

    // Apply community detection if in community color mode
    if (colorMode === 'community' && graph.order > 1 && graph.size > 0) {
      try {
        const clusteringResult = detectCommunities(graph);
        // Apply community colors
        graph.forEachNode((nodeId) => {
          const communityId = clusteringResult.nodeToCommuntiy.get(nodeId);
          if (communityId !== undefined) {
            graph.setNodeAttribute(nodeId, 'color', getCommunityColor(communityId));
            graph.setNodeAttribute(nodeId, 'community', communityId);
          }
        });
      } catch (e) {
        // Clustering failed, keep default colors
        console.warn('Community detection failed:', e);
      }
    }

    // Apply force-directed layout
    if (graph.order > 0) {
      forceAtlas2.assign(graph, {
        iterations: 100,
        settings: {
          gravity: 1,
          scalingRatio: 2,
          strongGravityMode: true,
          barnesHutOptimize: graph.order > 100,
        },
      });
    }

    // Create Sigma instance
    const sigma = new Sigma(graph, containerRef.current, {
      renderLabels: true,
      labelSize: 12,
      labelColor: { color: '#374151' },
      labelFont: 'Inter, sans-serif',
      defaultNodeColor: '#64748b',
      defaultEdgeColor: '#94a3b8',
      minCameraRatio: 0.1,
      maxCameraRatio: 10,
    });

    // Event handlers
    let draggedNode: string | null = null;

    // Node click
    sigma.on('clickNode', ({ node }) => {
      onNodeClick?.(node);
    });

    // Node right-click
    sigma.on('rightClickNode', ({ node, event }) => {
      // Prevent default browser context menu
      if (containerRef.current) {
        containerRef.current.addEventListener('contextmenu', (e) => e.preventDefault(), { once: true });
      }
      onNodeRightClick?.(node, event.x, event.y);
    });

    // Node drag - start
    sigma.on('downNode', (e) => {
      draggedNode = e.node;
      graph.setNodeAttribute(e.node, 'highlighted', true);
    });

    // Mouse move for dragging
    sigma.getMouseCaptor().on('mousemovebody', (e) => {
      if (!draggedNode) return;
      
      // Get position in graph coordinates
      const pos = sigma.viewportToGraph(e);
      
      // Update node position
      graph.setNodeAttribute(draggedNode, 'x', pos.x);
      graph.setNodeAttribute(draggedNode, 'y', pos.y);
      
      // Prevent camera movement
      e.preventSigmaDefault();
      e.original.preventDefault();
      e.original.stopPropagation();
    });

    // Mouse up - end drag
    sigma.getMouseCaptor().on('mouseup', () => {
      if (draggedNode) {
        graph.removeNodeAttribute(draggedNode, 'highlighted');
        draggedNode = null;
      }
    });

    // Node hover
    sigma.on('enterNode', ({ node }) => {
      onNodeHover?.(node);
      // Highlight connected nodes
      const connectedNodes = new Set<string>();
      graph.forEachNeighbor(node, (neighbor) => connectedNodes.add(neighbor));
      
      graph.forEachNode((n) => {
        if (n === node) {
          graph.setNodeAttribute(n, 'highlighted', true);
        } else if (connectedNodes.has(n)) {
          graph.setNodeAttribute(n, 'highlighted', true);
        } else {
          graph.setNodeAttribute(n, 'hidden', true);
        }
      });
      
      sigma.refresh();
    });

    sigma.on('leaveNode', () => {
      onNodeHover?.(null);
      // Reset all nodes
      graph.forEachNode((n) => {
        graph.removeNodeAttribute(n, 'hidden');
        graph.removeNodeAttribute(n, 'highlighted');
      });
      sigma.refresh();
    });

    sigmaRef.current = sigma;
    setSigmaInstance(sigma);

    return () => {
      sigma.kill();
      sigmaRef.current = null;
      setSigmaInstance(null);
    };
  }, [nodes, edges, colorMode, onNodeClick, onNodeHover, onNodeRightClick, setSigmaInstance]);

  useEffect(() => {
    const cleanup = initializeGraph();
    return () => cleanup?.();
  }, [initializeGraph]);

  return (
    <div
      ref={containerRef}
      className="w-full h-full min-h-[400px] bg-muted/20 rounded-lg"
    />
  );
}

export default GraphRenderer;
