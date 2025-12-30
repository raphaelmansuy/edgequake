'use client';

import { detectCommunities, getCommunityColor } from '@/lib/graph/clustering';
import { useGraphStore } from '@/stores/use-graph-store';
import { useSettingsStore } from '@/stores/use-settings-store';
import type { GraphEdge, GraphNode } from '@/types';
import { EdgeCurvedArrowProgram, createEdgeCurveProgram } from '@sigma/edge-curve';
import { NodeBorderProgram } from '@sigma/node-border';
import Graph from 'graphology';
import forceAtlas2 from 'graphology-layout-forceatlas2';
import circular from 'graphology-layout/circular';
import circlepack from 'graphology-layout/circlepack';
import random from 'graphology-layout/random';
import noverlap from 'graphology-layout-noverlap';
import forceLayout from 'graphology-layout-force';
import { useTheme } from 'next-themes';
import { useCallback, useEffect, useMemo, useRef } from 'react';
import Sigma from 'sigma';
import { animateNodes } from 'sigma/utils';

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

// Node size mapping
const NODE_SIZES: Record<string, number> = {
  small: 6,
  medium: 10,
  large: 14,
};

// Theme-aware label colors
const LABEL_COLORS = {
  light: '#374151', // gray-700
  dark: '#e2e8f0',  // slate-200
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
  const graphRef = useRef<Graph | null>(null);
  const previousLayoutRef = useRef<string | null>(null);
  const setSigmaInstance = useGraphStore((s) => s.setSigmaInstance);
  const colorMode = useGraphStore((s) => s.colorMode);
  const streamingProgress = useGraphStore((s) => s.streamingProgress);
  const useStreaming = useGraphStore((s) => s.useStreaming);
  const { graphSettings } = useSettingsStore();
  const { resolvedTheme } = useTheme();
  const isDark = resolvedTheme === 'dark';
  
  // Track previous node/edge counts for incremental updates
  const prevNodesCountRef = useRef(0);
  const prevEdgesCountRef = useRef(0);
  const pendingLayoutUpdateRef = useRef(false);
  const layoutUpdateTimerRef = useRef<NodeJS.Timeout | null>(null);

  // Get settings with defaults
  const showLabels = graphSettings.showLabels ?? true;
  const showEdgeLabels = graphSettings.showEdgeLabels ?? false;
  const enableNodeDrag = graphSettings.enableNodeDrag ?? true;
  const highlightNeighbors = graphSettings.highlightNeighbors ?? true;
  const hideUnselectedEdges = graphSettings.hideUnselectedEdges ?? false;
  const nodeSize = NODE_SIZES[graphSettings.nodeSize] ?? NODE_SIZES.medium;
  const layout = graphSettings.layout ?? 'force';
  
  // Check if currently streaming
  const isActivelyStreaming = useStreaming && 
    (streamingProgress.phase === 'nodes' || streamingProgress.phase === 'edges' || streamingProgress.phase === 'metadata');
  
  // Memoize node and edge sets for efficient diffing
  const nodeIdSet = useMemo(() => new Set(nodes.map(n => n.id)), [nodes]);
  const edgeIdSet = useMemo(() => {
    const set = new Set<string>();
    edges.forEach(e => set.add(`${e.source}-${e.target}-${e.relationship_type}`));
    return set;
  }, [edges]);

  // Function to add nodes to existing graph (for streaming)
  const addNodesToGraph = useCallback((graph: Graph, newNodes: GraphNode[]) => {
    const borderColor = isDark ? '#374151' : '#ffffff';
    const existingNodeCount = graph.order;
    
    newNodes.forEach((node, index) => {
      if (graph.hasNode(node.id)) return; // Skip existing nodes
      
      // Position new nodes in a spiral pattern from existing nodes
      const angle = (2 * Math.PI * (existingNodeCount + index)) / Math.max(existingNodeCount + newNodes.length, 1);
      const radius = 100 + (existingNodeCount * 2);
      
      graph.addNode(node.id, {
        label: node.label,
        x: Math.cos(angle) * radius,
        y: Math.sin(angle) * radius,
        size: nodeSize,
        color: getNodeColor(node.node_type),
        borderColor: borderColor,
        borderSize: 0.15,
        entityType: node.node_type,
        description: node.description,
      });
    });
  }, [isDark, nodeSize]);
  
  // Function to add edges to existing graph (for streaming)
  const addEdgesToGraph = useCallback((graph: Graph, newEdges: GraphEdge[]) => {
    newEdges.forEach((edge) => {
      if (!graph.hasNode(edge.source) || !graph.hasNode(edge.target)) return;
      
      const edgeId = `${edge.source}-${edge.target}-${edge.relationship_type}`;
      if (graph.hasEdge(edgeId)) return; // Skip existing edges
      
      try {
        graph.addEdgeWithKey(edgeId, edge.source, edge.target, {
          label: edge.relationship_type,
          size: Math.max(1, Math.min(edge.weight * 2, 5)),
          color: isDark ? '#4b5563' : '#94a3b8',
          type: 'curvedArrow',
          curvature: 0.25,
        });
      } catch {
        // Edge already exists or invalid
      }
    });
  }, [isDark]);
  
  // Debounced layout update for streaming
  const scheduleLayoutUpdate = useCallback(() => {
    if (layoutUpdateTimerRef.current) {
      clearTimeout(layoutUpdateTimerRef.current);
    }
    
    pendingLayoutUpdateRef.current = true;
    
    // Delay layout update to batch multiple node additions
    layoutUpdateTimerRef.current = setTimeout(() => {
      const graph = graphRef.current;
      const sigma = sigmaRef.current;
      
      if (!graph || !sigma || graph.order === 0) return;
      
      // Apply incremental force layout for new nodes only
      try {
        forceAtlas2.assign(graph, {
          iterations: 50, // Fewer iterations for streaming updates
          settings: {
            gravity: 1,
            scalingRatio: 2,
            strongGravityMode: true,
            barnesHutOptimize: graph.order > 100,
            slowDown: 2, // Slower convergence for smoother animation
          },
        });
        
        sigma.refresh();
      } catch (e) {
        console.warn('Layout update failed:', e);
      }
      
      pendingLayoutUpdateRef.current = false;
    }, 100); // 100ms debounce
  }, []);

  const initializeGraph = useCallback(() => {
    if (!containerRef.current || nodes.length === 0) return;

    // Cleanup previous instance
    if (sigmaRef.current) {
      sigmaRef.current.kill();
      sigmaRef.current = null;
    }

    // Create graphology graph
    const graph = new Graph();
    graphRef.current = graph;

    // Add nodes with border styling
    const nodeColor = (type: string | undefined) => getNodeColor(type);
    const borderColor = isDark ? '#374151' : '#ffffff';
    
    nodes.forEach((node, index) => {
      const angle = (2 * Math.PI * index) / nodes.length;
      const radius = 100;
      
      graph.addNode(node.id, {
        label: node.label,
        x: Math.cos(angle) * radius,
        y: Math.sin(angle) * radius,
        size: nodeSize,
        color: nodeColor(node.node_type),
        borderColor: borderColor,
        borderSize: 0.15, // Border width as ratio of node size
        entityType: node.node_type,
        description: node.description,
      });
    });

    // Add edges with curved arrow styling
    edges.forEach((edge) => {
      if (graph.hasNode(edge.source) && graph.hasNode(edge.target)) {
        try {
          graph.addEdge(edge.source, edge.target, {
            label: edge.relationship_type,
            size: Math.max(1, Math.min(edge.weight * 2, 5)),
            color: isDark ? '#4b5563' : '#94a3b8',
            type: 'curvedArrow',
            curvature: 0.25,
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

    // Apply layout based on settings (calculate positions first, then animate)
    const calculateLayoutPositions = () => {
      const positions: Record<string, { x: number; y: number }> = {};
      
      // First calculate positions in a temporary graph
      const tempGraph = graph.copy();
      
      switch (layout) {
        case 'circular':
          circular.assign(tempGraph);
          break;
        case 'circlepack':
          circlepack.assign(tempGraph);
          break;
        case 'random':
          random.assign(tempGraph);
          break;
        case 'noverlaps':
          // Apply noverlap to prevent overlaps
          noverlap.assign(tempGraph, {
            maxIterations: 100,
            settings: {
              margin: 5,
              expansion: 1.1,
              gridSize: 1,
              ratio: 1,
              speed: 3,
            },
          });
          break;
        case 'force-directed':
          // Use synchronous force-directed layout
          forceLayout.assign(tempGraph, {
            maxIterations: 100,
            settings: {
              attraction: 0.0003,
              repulsion: 0.02,
              gravity: 0.02,
              inertia: 0.4,
              maxMove: 100,
            },
          });
          break;
        case 'hierarchical':
          // Hierarchical layout (using circular as fallback)
          circular.assign(tempGraph);
          break;
        case 'force':
        default:
          forceAtlas2.assign(tempGraph, {
            iterations: 100,
            settings: {
              gravity: 1,
              scalingRatio: 2,
              strongGravityMode: true,
              barnesHutOptimize: tempGraph.order > 100,
            },
          });
          break;
      }
      
      // Extract positions
      tempGraph.forEachNode((nodeId) => {
        positions[nodeId] = {
          x: tempGraph.getNodeAttribute(nodeId, 'x'),
          y: tempGraph.getNodeAttribute(nodeId, 'y'),
        };
      });
      
      return positions;
    };

    // Apply initial layout
    if (graph.order > 0) {
      const positions = calculateLayoutPositions();
      // Apply positions directly for initial load (no animation yet)
      Object.entries(positions).forEach(([nodeId, pos]) => {
        graph.setNodeAttribute(nodeId, 'x', pos.x);
        graph.setNodeAttribute(nodeId, 'y', pos.y);
      });
    }

    // Create Sigma instance with visual quality settings and LOD optimizations
    const sigma = new Sigma(graph, containerRef.current, {
      renderLabels: showLabels,
      renderEdgeLabels: showEdgeLabels,
      labelSize: 12,
      labelColor: { color: isDark ? LABEL_COLORS.dark : LABEL_COLORS.light },
      labelFont: 'Inter, ui-sans-serif, system-ui, sans-serif',
      labelGridCellSize: 120,           // Larger cells = fewer labels, 120 is balanced
      labelRenderedSizeThreshold: 6,     // Show labels for smaller nodes (was 12)
      labelDensity: 0.7,                 // Higher density = more labels visible (was 0.1)
      defaultNodeColor: '#64748b',
      defaultEdgeColor: isDark ? '#4b5563' : '#94a3b8',
      defaultNodeType: 'border',
      defaultEdgeType: 'curvedArrow',
      nodeProgramClasses: {
        border: NodeBorderProgram,
      },
      edgeProgramClasses: {
        curvedArrow: EdgeCurvedArrowProgram,
        curved: createEdgeCurveProgram(),
      },
      minCameraRatio: 0.1,
      maxCameraRatio: 10,
      enableEdgeEvents: true,
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

    // Node drag - only if enabled
    if (enableNodeDrag) {
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
    }

    // Node hover - with optional neighbor highlighting and edge hiding
    sigma.on('enterNode', ({ node }) => {
      onNodeHover?.(node);
      
      if (highlightNeighbors) {
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
        
        // Hide unselected edges if setting is enabled
        if (hideUnselectedEdges) {
          graph.forEachEdge((edge, attrs, source, target) => {
            const isConnected = source === node || target === node;
            if (!isConnected) {
              graph.setEdgeAttribute(edge, 'hidden', true);
            }
          });
        }
        
        sigma.refresh();
      }
    });

    sigma.on('leaveNode', () => {
      onNodeHover?.(null);
      
      if (highlightNeighbors) {
        // Reset all nodes
        graph.forEachNode((n) => {
          graph.removeNodeAttribute(n, 'hidden');
          graph.removeNodeAttribute(n, 'highlighted');
        });
        
        // Reset all edges
        if (hideUnselectedEdges) {
          graph.forEachEdge((edge) => {
            graph.removeEdgeAttribute(edge, 'hidden');
          });
        }
        
        sigma.refresh();
      }
    });

    // Edge hover - highlight edge and connected nodes
    sigma.on('enterEdge', ({ edge }) => {
      const source = graph.source(edge);
      const target = graph.target(edge);
      
      // Store original edge attributes for restoration
      const originalSize = graph.getEdgeAttribute(edge, 'size') || 2;
      const originalColor = graph.getEdgeAttribute(edge, 'color');
      
      // Highlight the edge
      graph.setEdgeAttribute(edge, 'size', originalSize * 2);
      graph.setEdgeAttribute(edge, 'color', isDark ? '#60a5fa' : '#3b82f6');
      graph.setEdgeAttribute(edge, 'originalSize', originalSize);
      graph.setEdgeAttribute(edge, 'originalColor', originalColor);
      
      // Highlight connected nodes
      graph.setNodeAttribute(source, 'highlighted', true);
      graph.setNodeAttribute(target, 'highlighted', true);
      
      sigma.refresh();
    });

    sigma.on('leaveEdge', ({ edge }) => {
      // Restore original edge attributes
      const originalSize = graph.getEdgeAttribute(edge, 'originalSize');
      const originalColor = graph.getEdgeAttribute(edge, 'originalColor');
      
      if (originalSize !== undefined) {
        graph.setEdgeAttribute(edge, 'size', originalSize);
        graph.removeEdgeAttribute(edge, 'originalSize');
      }
      if (originalColor !== undefined) {
        graph.setEdgeAttribute(edge, 'color', originalColor);
        graph.removeEdgeAttribute(edge, 'originalColor');
      }
      
      // Reset connected nodes
      const source = graph.source(edge);
      const target = graph.target(edge);
      graph.removeNodeAttribute(source, 'highlighted');
      graph.removeNodeAttribute(target, 'highlighted');
      
      sigma.refresh();
    });

    sigmaRef.current = sigma;
    setSigmaInstance(sigma);
    previousLayoutRef.current = layout;

    return () => {
      sigma.kill();
      sigmaRef.current = null;
      graphRef.current = null;
      setSigmaInstance(null);
    };
  }, [nodes, edges, colorMode, layout, nodeSize, showLabels, showEdgeLabels, enableNodeDrag, highlightNeighbors, hideUnselectedEdges, isDark, onNodeClick, onNodeHover, onNodeRightClick, setSigmaInstance]);

  // Animate layout changes (when layout prop changes after initial render)
  useEffect(() => {
    const graph = graphRef.current;
    const sigma = sigmaRef.current;
    
    // Only animate if we have an existing graph and the layout actually changed
    if (!graph || !sigma || !previousLayoutRef.current || previousLayoutRef.current === layout) {
      return;
    }
    
    // Calculate new positions
    const tempGraph = graph.copy();
    
    switch (layout) {
      case 'circular':
        circular.assign(tempGraph);
        break;
      case 'circlepack':
        circlepack.assign(tempGraph);
        break;
      case 'random':
        random.assign(tempGraph);
        break;
      case 'noverlaps':
        noverlap.assign(tempGraph, {
          maxIterations: 100,
          settings: {
            margin: 5,
            expansion: 1.1,
            gridSize: 1,
            ratio: 1,
            speed: 3,
          },
        });
        break;
      case 'force-directed':
        forceLayout.assign(tempGraph, {
          maxIterations: 100,
          settings: {
            attraction: 0.0003,
            repulsion: 0.02,
            gravity: 0.02,
            inertia: 0.4,
            maxMove: 100,
          },
        });
        break;
      case 'hierarchical':
        circular.assign(tempGraph);
        break;
      case 'force':
      default:
        forceAtlas2.assign(tempGraph, {
          iterations: 100,
          settings: {
            gravity: 1,
            scalingRatio: 2,
            strongGravityMode: true,
            barnesHutOptimize: tempGraph.order > 100,
          },
        });
        break;
    }
    
    // Extract new positions
    const newPositions: Record<string, { x: number; y: number }> = {};
    tempGraph.forEachNode((nodeId) => {
      newPositions[nodeId] = {
        x: tempGraph.getNodeAttribute(nodeId, 'x'),
        y: tempGraph.getNodeAttribute(nodeId, 'y'),
      };
    });
    
    // Animate to new positions (300ms transition)
    animateNodes(graph, newPositions, { duration: 300, easing: 'quadraticInOut' });
    
    previousLayoutRef.current = layout;
  }, [layout]);
  
  // Incremental update for streaming - add new nodes/edges without full re-render
  useEffect(() => {
    const graph = graphRef.current;
    const sigma = sigmaRef.current;
    
    // Skip if no graph/sigma, or if this is the initial render
    if (!graph || !sigma) return;
    
    // Check if we're in streaming mode and there are new nodes
    const currentNodeCount = nodes.length;
    const currentEdgeCount = edges.length;
    const prevNodeCount = prevNodesCountRef.current;
    const prevEdgeCount = prevEdgesCountRef.current;
    
    // Only do incremental updates during active streaming
    if (!isActivelyStreaming) {
      prevNodesCountRef.current = currentNodeCount;
      prevEdgesCountRef.current = currentEdgeCount;
      return;
    }
    
    // Check for new nodes
    if (currentNodeCount > prevNodeCount) {
      const newNodes = nodes.filter(n => !graph.hasNode(n.id));
      if (newNodes.length > 0) {
        addNodesToGraph(graph, newNodes);
        scheduleLayoutUpdate();
        sigma.refresh();
      }
    }
    
    // Check for new edges
    if (currentEdgeCount > prevEdgeCount) {
      const newEdges = edges.filter(e => {
        const edgeId = `${e.source}-${e.target}-${e.relationship_type}`;
        return !graph.hasEdge(edgeId) && graph.hasNode(e.source) && graph.hasNode(e.target);
      });
      if (newEdges.length > 0) {
        addEdgesToGraph(graph, newEdges);
        sigma.refresh();
      }
    }
    
    prevNodesCountRef.current = currentNodeCount;
    prevEdgesCountRef.current = currentEdgeCount;
  }, [nodes, edges, isActivelyStreaming, addNodesToGraph, addEdgesToGraph, scheduleLayoutUpdate]);
  
  // Cleanup layout update timer on unmount
  useEffect(() => {
    return () => {
      if (layoutUpdateTimerRef.current) {
        clearTimeout(layoutUpdateTimerRef.current);
      }
    };
  }, []);

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
