/**
 * @module GraphRenderer
 * @description Sigma.js-powered graph renderer with layout algorithms.
 * Handles WebGL rendering, node/edge programs, and force-directed layouts.
 * 
 * @implements FEAT0601 - WebGL graph rendering with Sigma.js
 * @implements FEAT0603 - Force-directed layout algorithms (ForceAtlas2, circular, noverlap)
 * @implements FEAT0604 - Community detection and clustering visualization
 * @implements FEAT0605 - Theme-aware node and edge styling
 * 
 * @enforces BR0009 - Performant rendering for 1000+ nodes
 * @enforces BR0601 - Layout animations are smooth (60fps target)
 * @enforces BR0603 - Entity types have consistent color coding
 * 
 * @see {@link docs/features.md} FEAT0601-0605
 */
'use client';

import { detectCommunities, getCommunityColor } from '@/lib/graph/clustering';
import { getGraphEdgeKeyFromEdge } from '@/lib/graph/ids';
import { formatEntityLabel, getEntityTypeColor } from '@/lib/graph/label-utils';
import {
    applyLayoutToGraph,
    calculateLayoutPositions,
    getGraphPerformanceProfile,
    type GraphLayoutType,
} from '@/lib/graph/layouts';
import {
    drawEdgeLabelWithBackground,
    drawNodeHoverWithCard,
    drawNodeLabelWithBackground,
} from '@/lib/graph/sigma-renderers';
import { useGraphStore } from '@/stores/use-graph-store';
import { useSettingsStore } from '@/stores/use-settings-store';
import type { GraphEdge, GraphNode } from '@/types';
import { EdgeCurvedArrowProgram, createEdgeCurveProgram } from '@sigma/edge-curve';
import { NodeBorderProgram } from '@sigma/node-border';
import Graph from 'graphology';
import { useTheme } from 'next-themes';
import { useCallback, useEffect, useRef, useState } from 'react';
import Sigma from 'sigma';
import { animateNodes } from 'sigma/utils';

// Node size mapping
const NODE_SIZES: Record<string, number> = {
  small: 6,
  medium: 10,
  large: 14,
};

// WHY: Dynamic node sizing based on degree (connections)
// More connected nodes are larger → easier to spot important entities
function calculateNodeSize(degree: number, baseSize: number): number {
  if (degree === 0) return baseSize;

  // Scale size logarithmically: size = baseSize + log2(degree + 1) * 2
  // Examples: 0 connections = baseSize, 1 = baseSize+2, 3 = baseSize+4, 7 = baseSize+6
  const scaleFactor = Math.log2(degree + 1) * 2;
  return Math.min(baseSize + scaleFactor, baseSize * 3); // Cap at 3x base size
}

// Theme-aware label colors
const LABEL_COLORS = {
  light: '#374151', // gray-700
  dark: '#e2e8f0',  // slate-200
};

function getNodeColor(entityType: string | undefined): string {
  return getEntityTypeColor(entityType);
}

interface GraphRendererProps {
  nodes: GraphNode[];
  edges: GraphEdge[];
  onNodeClick?: (nodeId: string) => void;
  onNodeHover?: (nodeId: string | null) => void;
  onNodeRightClick?: (nodeId: string, x: number, y: number) => void;
}

interface HoverState {
  nodeId: string | null;
  neighborIds: Set<string>;
  edgeId: string | null;
}

export function GraphRenderer({ nodes, edges, onNodeClick, onNodeHover, onNodeRightClick }: GraphRendererProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const sigmaRef = useRef<Sigma | null>(null);
  const graphRef = useRef<Graph | null>(null);
  const previousLayoutRef = useRef<GraphLayoutType | null>(null);
  const setSigmaInstance = useGraphStore((s) => s.setSigmaInstance);
  const selectedNodeId = useGraphStore((s) => s.selectedNodeId);
  const colorMode = useGraphStore((s) => s.colorMode);
  const streamingProgress = useGraphStore((s) => s.streamingProgress);
  const useStreaming = useGraphStore((s) => s.useStreaming);
  const { graphSettings } = useSettingsStore();
  const { resolvedTheme } = useTheme();
  const isDark = resolvedTheme === 'dark';
  
  // Track previous node/edge counts for incremental updates
  const prevNodesCountRef = useRef(0);
  const prevEdgesCountRef = useRef(0);
  const layoutUpdateTimerRef = useRef<NodeJS.Timeout | null>(null);
  const [renderError, setRenderError] = useState<string | null>(null);

  // Get settings with defaults
  const showLabels = graphSettings.showLabels ?? true;
  const showEdgeLabels = graphSettings.showEdgeLabels ?? false;
  const enableNodeDrag = graphSettings.enableNodeDrag ?? true;
  const highlightNeighbors = graphSettings.highlightNeighbors ?? true;
  const hideUnselectedEdges = graphSettings.hideUnselectedEdges ?? false;
  const nodeSize = NODE_SIZES[graphSettings.nodeSize] ?? NODE_SIZES.medium;
  const layout = graphSettings.layout ?? 'force';
  const hoverStateRef = useRef<HoverState>({
    nodeId: null,
    neighborIds: new Set<string>(),
    edgeId: null,
  });
  const selectedNodeIdRef = useRef<string | null>(selectedNodeId);
  const showEdgeLabelsRef = useRef(showEdgeLabels);
  const showLabelsRef = useRef(showLabels);
  const enableNodeDragRef = useRef(enableNodeDrag);
  const highlightNeighborsRef = useRef(highlightNeighbors);
  const hideUnselectedEdgesRef = useRef(hideUnselectedEdges);
  const nodeSizeRef = useRef(nodeSize);
  const layoutRef = useRef<GraphLayoutType>(layout);
  const onNodeClickRef = useRef(onNodeClick);
  const onNodeHoverRef = useRef(onNodeHover);
  const onNodeRightClickRef = useRef(onNodeRightClick);
  
  // Check if currently streaming
  const isActivelyStreaming = useStreaming && 
    (streamingProgress.phase === 'nodes' || streamingProgress.phase === 'edges' || streamingProgress.phase === 'metadata');

  useEffect(() => {
    selectedNodeIdRef.current = selectedNodeId;
  }, [selectedNodeId]);

  useEffect(() => {
    showLabelsRef.current = showLabels;
    showEdgeLabelsRef.current = showEdgeLabels;
    enableNodeDragRef.current = enableNodeDrag;
    highlightNeighborsRef.current = highlightNeighbors;
    hideUnselectedEdgesRef.current = hideUnselectedEdges;
    nodeSizeRef.current = nodeSize;
    layoutRef.current = layout;
    onNodeClickRef.current = onNodeClick;
    onNodeHoverRef.current = onNodeHover;
    onNodeRightClickRef.current = onNodeRightClick;
  }, [
    enableNodeDrag,
    hideUnselectedEdges,
    highlightNeighbors,
    layout,
    nodeSize,
    onNodeClick,
    onNodeHover,
    onNodeRightClick,
    showEdgeLabels,
    showLabels,
  ]);

  // Function to add nodes to existing graph (for streaming)
  const addNodesToGraph = useCallback((graph: Graph, newNodes: GraphNode[]) => {
    const borderColor = isDark ? '#374151' : '#ffffff';
    const existingNodeCount = graph.order;

    newNodes.forEach((node, index) => {
      if (graph.hasNode(node.id)) return; // Skip existing nodes

      // Position new nodes in a spiral pattern from existing nodes
      const angle = (2 * Math.PI * (existingNodeCount + index)) / Math.max(existingNodeCount + newNodes.length, 1);
      const radius = 100 + (existingNodeCount * 2);

      // WHY: Dynamic node sizing based on degree (connections)
      const nodeDegree = node.degree || 0;
      const dynamicSize = calculateNodeSize(nodeDegree, nodeSizeRef.current);

      graph.addNode(node.id, {
        label: formatEntityLabel(node.label),
        x: Math.cos(angle) * radius,
        y: Math.sin(angle) * radius,
        size: dynamicSize, // Use dynamic size based on connections
        color: getNodeColor(node.node_type),
        borderColor: borderColor,
        borderSize: 0.15,
        type: 'border', // Explicitly set node type for NodeBorderProgram
        entityType: node.node_type,
        description: node.description,
        degree: nodeDegree, // Store degree for later reference
      });
    });
  }, [isDark]);
  
  // Function to add edges to existing graph (for streaming)
  // WHY: forceLabel mirrors the showEdgeLabels setting so edge type text always
  // appears when the user has toggled "Show Edge Labels" on, even when the
  // endpoint-node label threshold has not been met (sigma 3.x only draws edge
  // labels automatically when both endpoint node-labels are visible).
  const addEdgesToGraph = useCallback((graph: Graph, newEdges: GraphEdge[]) => {
    newEdges.forEach((edge) => {
      if (!graph.hasNode(edge.source) || !graph.hasNode(edge.target)) return;
      
      const edgeId = getGraphEdgeKeyFromEdge(edge);
      if (graph.hasEdge(edgeId)) return; // Skip existing edges
      
      try {
        graph.addEdgeWithKey(edgeId, edge.source, edge.target, {
          label: edge.relationship_type,
          forceLabel: showEdgeLabelsRef.current, // WHY: force-show label when setting is on
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
  
  // WHY: Track layout performance for adaptive iteration count
  const layoutMetricsRef = useRef({
    lastDurationMs: 0,
    avgDurationMs: 0,
    updateCount: 0,
  });
  
  // WHY: RAF-id for cancellation on cleanup 
  const rafIdRef = useRef<number | null>(null);
  
  // Debounced layout update for streaming - uses requestAnimationFrame for non-blocking execution
  const scheduleLayoutUpdate = useCallback(() => {
    if (layoutUpdateTimerRef.current) {
      clearTimeout(layoutUpdateTimerRef.current);
    }
    
    // Delay layout update to batch multiple node additions
    layoutUpdateTimerRef.current = setTimeout(() => {
      const graph = graphRef.current;
      const sigma = sigmaRef.current;
      
      if (!graph || !sigma || graph.order === 0) return;
      
      // WHY: Use requestAnimationFrame to avoid blocking main thread during layout
      if (rafIdRef.current) {
        cancelAnimationFrame(rafIdRef.current);
      }
      
      rafIdRef.current = requestAnimationFrame(() => {
        const startTime = performance.now();
        const nodeCount = graph.order;
        
        try {
          applyLayoutToGraph(graph, 'force', 'streaming');
          sigma.scheduleRefresh();
        } catch (e) {
          console.warn('Layout update failed:', e);
        }
        
        // WHY: Track performance metrics for adaptive iteration count
        const duration = performance.now() - startTime;
        const metrics = layoutMetricsRef.current;
        metrics.lastDurationMs = duration;
        metrics.updateCount++;
        metrics.avgDurationMs = (metrics.avgDurationMs * (metrics.updateCount - 1) + duration) / metrics.updateCount;
        
        if (duration > 100) {
          console.warn(`[GraphRenderer] Layout took ${duration.toFixed(1)}ms (${nodeCount} nodes, streaming mode)`);
        }
        
        rafIdRef.current = null;
      });
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

    const borderColor = isDark ? '#374151' : '#ffffff';
    const defaultEdgeColor = isDark ? '#4b5563' : '#94a3b8';
    
    let addedNodeCount = 0;
    let skippedNodeCount = 0;
    
    nodes.forEach((node, index) => {
      // Validate node ID
      if (!node.id || typeof node.id !== 'string' || node.id.trim() === '') {
        console.error(`[GraphRenderer] Invalid node ID at index ${index}:`, node);
        skippedNodeCount++;
        return;
      }
      
      // Skip if node already exists (defensive check for duplicates)
      if (graph.hasNode(node.id)) {
        console.warn(
          `[GraphRenderer] Duplicate node detected: "${node.id}" (${node.label}). ` +
          'This indicates the backend returned duplicate data.'
        );
        skippedNodeCount++;
        return;
      }
      
      const angle = (2 * Math.PI * index) / nodes.length;
      const radius = 100;

      // WHY: Dynamic node sizing based on degree (connections)
      const nodeDegree = node.degree || 0;
      const dynamicSize = calculateNodeSize(nodeDegree, nodeSizeRef.current);

      try {
        graph.addNode(node.id, {
          label: formatEntityLabel(node.label),
          x: Math.cos(angle) * radius,
          y: Math.sin(angle) * radius,
          size: dynamicSize, // Use dynamic size based on connections
          color: getNodeColor(node.node_type),
          borderColor: borderColor,
          borderSize: 0.2, // Slightly larger border for better visibility
          type: 'border', // Explicitly set node type for NodeBorderProgram
          entityType: node.node_type,
          description: node.description,
          degree: nodeDegree, // Store degree for later reference
        });
        addedNodeCount++;
      } catch (error) {
        console.error(
          `[GraphRenderer] Failed to add node "${node.id}":`,
          error,
          'Node data:',
          node
        );
        skippedNodeCount++;
      }
    });
    
    // Log node addition stats
    if (skippedNodeCount > 0) {
      console.warn(
        `[GraphRenderer] Skipped ${skippedNodeCount} nodes ` +
        `(${addedNodeCount} successfully added)`
      );
    }

    // Add edges with curved arrow styling
    let addedEdgeCount = 0;
    let skippedEdgeCount = 0;
    
    edges.forEach((edge) => {
      // Validate edge has valid source and target
      if (!edge.source || !edge.target ||
          typeof edge.source !== 'string' || typeof edge.target !== 'string' ||
          edge.source.trim() === '' || edge.target.trim() === '') {
        console.error('[GraphRenderer] Invalid edge source/target:', edge);
        skippedEdgeCount++;
        return;
      }
      
      if (graph.hasNode(edge.source) && graph.hasNode(edge.target)) {
        try {
          graph.addEdgeWithKey(getGraphEdgeKeyFromEdge(edge), edge.source, edge.target, {
            label: edge.relationship_type,
            forceLabel: showEdgeLabelsRef.current, // WHY: force-show label when setting is on
            size: Math.max(1, Math.min(edge.weight * 2, 5)),
            color: defaultEdgeColor,
            type: 'curvedArrow',
            curvature: 0.25,
          });
          addedEdgeCount++;
        } catch {
          // Edge already exists or invalid - silently skip
          skippedEdgeCount++;
        }
      } else {
        // Source or target node doesn't exist
        console.warn(
          `[GraphRenderer] Skipping edge because nodes don't exist: ` +
          `"${edge.source}" → "${edge.target}" (${edge.relationship_type})`
        );
        skippedEdgeCount++;
      }
    });
    
    // Log edge addition stats
    if (skippedEdgeCount > 0) {
      console.warn(
        `[GraphRenderer] Skipped ${skippedEdgeCount} edges ` +
        `(${addedEdgeCount} successfully added)`
      );
    }

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

    // Apply initial layout
    if (graph.order > 0) {
      const positions = calculateLayoutPositions(graph, layoutRef.current, 'initial');
      // Apply positions directly for initial load (no animation yet)
      Object.entries(positions).forEach(([nodeId, pos]) => {
        graph.setNodeAttribute(nodeId, 'x', pos.x);
        graph.setNodeAttribute(nodeId, 'y', pos.y);
      });
    }

    const performanceProfile = getGraphPerformanceProfile(graph.order, graph.size);

    // Pre-compute degree threshold for hub nodes (force-label the top 15% by degree).
    // Done ONCE at init — not per frame. Uses graphology's O(1) degree() lookups.
    const allDegrees = graph.nodes().map((n) => graph.degree(n)).sort((a, b) => b - a);
    const hubThreshold = allDegrees[Math.floor(allDegrees.length * 0.15)] ?? 0;

    const nodeReducer = (node: string, attrs: Record<string, unknown>) => {
      const hoverState = hoverStateRef.current;
      const isSelected = selectedNodeIdRef.current === node;
      const isHoveredNode = hoverState.nodeId === node;

      // Compute forceLabel first — hub nodes always show their label
      const degree = typeof attrs.degree === 'number' ? attrs.degree : 0;
      const isHub = degree >= hubThreshold && hubThreshold > 0;

      // Fast path: no hover state, no selection — only apply forceLabel if changed
      if (hoverState.nodeId === null && !isSelected) {
        if (!isHub) return attrs; // Nothing to change
        return attrs.forceLabel === true ? attrs : { ...attrs, forceLabel: true };
      }

      const isNeighbor = hoverState.neighborIds.has(node);
      const isEdgeEndpoint =
        hoverState.edgeId !== null &&
        graph.hasEdge(hoverState.edgeId) &&
        (graph.source(hoverState.edgeId) === node || graph.target(hoverState.edgeId) === node);

      // Hide non-relevant nodes when highlight mode is active
      if (
        highlightNeighborsRef.current &&
        hoverState.nodeId !== null &&
        !isSelected &&
        !isHoveredNode &&
        !isNeighbor
      ) {
        return attrs.hidden === true && attrs.forceLabel === false
          ? attrs
          : { ...attrs, hidden: true, forceLabel: false };
      }

      if (!isSelected && !isHoveredNode && !isNeighbor && !isEdgeEndpoint) {
        const fl = isHub;
        return attrs.forceLabel === fl ? attrs : { ...attrs, forceLabel: fl };
      }

      // Node is in focus — apply selection/hover styling
      const next: Record<string, unknown> = { ...attrs, forceLabel: true };

      if (isSelected) {
        next.size = (typeof attrs.size === 'number' ? attrs.size : nodeSizeRef.current) * 1.8;
        next.borderSize = 3.5;
        next.borderColor = isDark ? '#60a5fa' : '#2563eb';
        next.zIndex = 999;
        return next;
      }

      next.borderSize = Math.max(typeof attrs.borderSize === 'number' ? attrs.borderSize : 0.2, 1.5);
      next.borderColor = isDark ? '#60a5fa' : '#2563eb';
      next.zIndex = 50;
      return next;
    };

    const edgeReducer = (edge: string, attrs: Record<string, unknown>) => {
      const hoverState = hoverStateRef.current;
      const selectedNodeId = selectedNodeIdRef.current;

      // Fast path: nothing focused → no edge label, return attrs unchanged
      if (hoverState.nodeId === null && selectedNodeId === null && hoverState.edgeId === null) {
        return attrs.forceLabel === false || attrs.forceLabel === undefined
          ? attrs
          : { ...attrs, forceLabel: false };
      }

      const source = graph.source(edge);
      const target = graph.target(edge);

      const isConnectedToFocus =
        hoverState.nodeId === source ||
        hoverState.nodeId === target ||
        selectedNodeId === source ||
        selectedNodeId === target;

      // Hide unrelated edges in highlight mode
      if (
        highlightNeighborsRef.current &&
        hideUnselectedEdgesRef.current &&
        hoverState.nodeId !== null &&
        hoverState.edgeId !== edge
      ) {
        if (!isConnectedToFocus) {
          return attrs.hidden === true ? attrs : { ...attrs, hidden: true, forceLabel: false };
        }
      }

      // Highlight the directly hovered edge
      if (hoverState.edgeId === edge) {
        return {
          ...attrs,
          forceLabel: !!attrs.label,
          color: isDark ? '#60a5fa' : '#3b82f6',
          size: (typeof attrs.size === 'number' ? attrs.size : 2) * 2,
        };
      }

      // Show edge label when connected to focused node
      const forceLabel = isConnectedToFocus && !!attrs.label;
      return attrs.forceLabel === forceLabel ? attrs : { ...attrs, forceLabel };
    };
    
    // Create Sigma instance with visual quality settings and LOD optimizations
    let sigma: Sigma;
    try {
      sigma = new Sigma(graph, containerRef.current, {
        renderLabels: showLabelsRef.current,
        // renderEdgeLabels: only process edge labels when a node is focused.
        // Using the hover/selected state stored in refs to gate this setting
        // avoids Sigma iterating 4000+ edges on every frame when nothing is focused.
        renderEdgeLabels: showEdgeLabelsRef.current,
        // Custom label renderer: background pill for readability over dense graphs
        defaultDrawNodeLabel: drawNodeLabelWithBackground,
        // Custom hover card: color-accented tooltip with entity type sub-label
        defaultDrawNodeHover: drawNodeHoverWithCard,
        // Custom edge label: formatted relationship type with background pill
        defaultDrawEdgeLabel: drawEdgeLabelWithBackground,
        labelSize: 11,
        labelWeight: '500',
        labelColor: { color: isDark ? LABEL_COLORS.dark : LABEL_COLORS.light },
        labelFont: 'Inter, ui-sans-serif, system-ui, sans-serif',
        edgeLabelSize: 10,
        edgeLabelFont: 'Inter, ui-sans-serif, system-ui, sans-serif',
        edgeLabelWeight: '500',
        edgeLabelColor: { color: isDark ? '#e2e8f0' : '#334155' },
        labelGridCellSize: performanceProfile.labelGridCellSize,
        labelRenderedSizeThreshold: performanceProfile.labelRenderedSizeThreshold,
        labelDensity: performanceProfile.labelDensity,
        // Critical for panning performance: hide edges/labels while camera is moving
        hideEdgesOnMove: performanceProfile.hideEdgesOnMove,
        hideLabelsOnMove: performanceProfile.hideLabelsOnMove,
        defaultNodeColor: '#94a3b8',
        defaultEdgeColor: defaultEdgeColor,
        defaultNodeType: 'border',
        defaultEdgeType: 'curvedArrow',
        nodeProgramClasses: {
          border: NodeBorderProgram,
        },
        edgeProgramClasses: {
          curvedArrow: EdgeCurvedArrowProgram,
          curved: createEdgeCurveProgram(),
        },
        minCameraRatio: 0.05,
        maxCameraRatio: 10,
        enableEdgeEvents: !performanceProfile.disableEdgeEvents,
        stagePadding: 50,
        zIndex: true,
        minEdgeThickness: 1.5,
        nodeReducer,
        edgeReducer,
      });
      setRenderError(null);
    } catch (error) {
      const message = error instanceof Error ? error.message : 'Failed to initialize graph renderer';
      console.warn('[GraphRenderer] Sigma initialization failed:', error);
      setRenderError(message);
      sigmaRef.current = null;
      graphRef.current = graph;
      setSigmaInstance(null);
      return () => {
        graphRef.current = null;
        setSigmaInstance(null);
      };
    }
    
    // WHY: Log performance info for debugging
    if (performanceProfile.isLargeGraph) {
      console.info(
        `[GraphRenderer] Large graph detected: ${performanceProfile.nodeCount} nodes, ${performanceProfile.edgeCount} edges. ` +
        `Applied LOD optimizations: labelDensity=${performanceProfile.labelDensity}, gridCellSize=${performanceProfile.labelGridCellSize}`
      );
    }

    // Event handlers
    let draggedNode: string | null = null;

    // Node click
    sigma.on('clickNode', ({ node }) => {
      onNodeClickRef.current?.(node);
    });

    // Node right-click
    sigma.on('rightClickNode', ({ node, event }) => {
      // Prevent default browser context menu
      if (containerRef.current) {
        containerRef.current.addEventListener('contextmenu', (e) => e.preventDefault(), { once: true });
      }

      // WHY: event.x/y from Sigma are canvas-relative coordinates.
      // Adding container.left/top converts them to absolute viewport (screen) coords,
      // which is what fixed-position elements expect.
      // The cursor is on the node when right-clicking, so this naturally
      // anchors the menu at the click point on the node edge.
      if (containerRef.current) {
        const container = containerRef.current.getBoundingClientRect();
        const screenX = container.left + event.x;
        const screenY = container.top  + event.y;
        onNodeRightClickRef.current?.(node, screenX, screenY);
      } else {
        onNodeRightClickRef.current?.(node, event.x, event.y);
      }
    });

    sigma.on('downNode', (e) => {
      if (!enableNodeDragRef.current) return;
      draggedNode = e.node;
    });

    sigma.getMouseCaptor().on('mousemovebody', (e) => {
      if (!draggedNode) return;
      
      const pos = sigma.viewportToGraph(e);
      
      graph.setNodeAttribute(draggedNode, 'x', pos.x);
      graph.setNodeAttribute(draggedNode, 'y', pos.y);
      
      e.preventSigmaDefault();
      e.original.preventDefault();
      e.original.stopPropagation();
    });

    sigma.getMouseCaptor().on('mouseup', () => {
      draggedNode = null;
    });

    // Node hover - with optional neighbor highlighting and edge hiding
    sigma.on('enterNode', ({ node }) => {
      onNodeHoverRef.current?.(node);

      if (highlightNeighborsRef.current) {
        const neighborIds = new Set<string>();
        graph.forEachNeighbor(node, (neighbor) => neighborIds.add(neighbor));
        hoverStateRef.current = {
          ...hoverStateRef.current,
          nodeId: node,
          neighborIds,
        };
        sigma.scheduleRefresh();
      }
    });

    sigma.on('leaveNode', () => {
      onNodeHoverRef.current?.(null);
      hoverStateRef.current = {
        ...hoverStateRef.current,
        nodeId: null,
        neighborIds: new Set<string>(),
      };
      sigma.scheduleRefresh();
    });

    // Edge hover - highlight edge and connected nodes
    sigma.on('enterEdge', ({ edge }) => {
      hoverStateRef.current = {
        ...hoverStateRef.current,
        edgeId: edge,
      };
      sigma.scheduleRefresh();
    });

    sigma.on('leaveEdge', () => {
      hoverStateRef.current = {
        ...hoverStateRef.current,
        edgeId: null,
      };
      sigma.scheduleRefresh();
    });

    sigmaRef.current = sigma;
    setSigmaInstance(sigma);
    previousLayoutRef.current = layoutRef.current;

    return () => {
      sigma.kill();
      sigmaRef.current = null;
      graphRef.current = null;
      hoverStateRef.current = {
        nodeId: null,
        neighborIds: new Set<string>(),
        edgeId: null,
      };
      setSigmaInstance(null);
    };
  }, [nodes, edges, colorMode, isDark, setSigmaInstance]);

  // Animate layout changes (when layout prop changes after initial render)
  useEffect(() => {
    const graph = graphRef.current;
    
    // Only animate if we have an existing graph and the layout actually changed
    if (!graph || !previousLayoutRef.current || previousLayoutRef.current === layout) {
      return;
    }
    const newPositions = calculateLayoutPositions(graph, layout, 'interactive');
    
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
        sigma.scheduleRefresh();
      }
    }
    
    // Check for new edges
    if (currentEdgeCount > prevEdgeCount) {
      const newEdges = edges.filter(e => {
        const edgeId = getGraphEdgeKeyFromEdge(e);
        return !graph.hasEdge(edgeId) && graph.hasNode(e.source) && graph.hasNode(e.target);
      });
      if (newEdges.length > 0) {
        addEdgesToGraph(graph, newEdges);
        sigma.scheduleRefresh();
      }
    }
    
    prevNodesCountRef.current = currentNodeCount;
    prevEdgesCountRef.current = currentEdgeCount;
  }, [nodes, edges, isActivelyStreaming, addNodesToGraph, addEdgesToGraph, scheduleLayoutUpdate]);
  
  // Cleanup layout update timer and RAF on unmount
  useEffect(() => {
    return () => {
      if (layoutUpdateTimerRef.current) {
        clearTimeout(layoutUpdateTimerRef.current);
      }
      // WHY: Cancel any pending RAF to prevent memory leaks
      if (rafIdRef.current) {
        cancelAnimationFrame(rafIdRef.current);
      }
    };
  }, []);

  useEffect(() => {
    const sigma = sigmaRef.current;
    if (!sigma) return;
    sigma.scheduleRefresh();
  }, [selectedNodeId]);

  useEffect(() => {
    const graph = graphRef.current;
    const sigma = sigmaRef.current;

    if (!graph || !sigma) return;

    graph.forEachNode((nodeId) => {
      const degree = graph.getNodeAttribute(nodeId, 'degree') || 0;
      graph.setNodeAttribute(nodeId, 'size', calculateNodeSize(degree, nodeSize));
    });

    sigma.scheduleRefresh();
  }, [nodeSize]);

  useEffect(() => {
    const graph = graphRef.current;
    const sigma = sigmaRef.current;

    if (!graph || !sigma) return;

    graph.forEachEdge((edgeId) => {
      graph.setEdgeAttribute(edgeId, 'forceLabel', showEdgeLabels);
    });

    sigma.setSetting('renderLabels', showLabels);
    sigma.setSetting(
      'renderEdgeLabels',
      showEdgeLabels && !getGraphPerformanceProfile(graph.order, graph.size).isVeryLargeGraph,
    );
    sigma.scheduleRefresh();
  }, [showLabels, showEdgeLabels]);

  useEffect(() => {
    const cleanup = initializeGraph();
    return () => cleanup?.();
  }, [initializeGraph]);

  return (
    <div
      ref={containerRef}
      className="relative h-full min-h-100 w-full rounded-lg bg-muted/20"
    >
      {renderError && (
        <div
          className="absolute inset-0 z-10 flex items-center justify-center rounded-lg border border-dashed bg-background/90 p-4 text-center"
          role="status"
          aria-live="polite"
        >
          <div>
            <p className="font-medium text-sm">Graph visualization unavailable</p>
            <p className="mt-1 text-xs text-muted-foreground">
              {renderError}
            </p>
          </div>
        </div>
      )}
    </div>
  );
}

export default GraphRenderer;
