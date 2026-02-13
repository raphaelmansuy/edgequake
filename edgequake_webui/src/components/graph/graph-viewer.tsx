/**
 * @module GraphViewer
 * @description Interactive knowledge graph visualization component using Sigma.js.
 * Renders entities as nodes and relationships as edges with full interactivity.
 * 
 * @implements UC0101 - User explores knowledge graph visually
 * @implements UC0104 - User filters entities by type, date, or relationship
 * @implements UC0107 - User exports graph for analysis
 * @implements FEAT0601 - Interactive graph visualization with Sigma.js
 * @implements FEAT0202 - Entity type filtering and search
 * @implements FEAT0205 - Node hover previews and context menus
 * @implements FEAT0206 - Minimap for large graph navigation
 * 
 * @enforces BR0009 - Graph must handle 1000+ nodes performantly
 * @enforces BR0201 - Entity selection syncs with detail panel
 * @enforces BR0602 - Streaming indicator for progressive loading
 * 
 * @see {@link docs/use_cases.md} UC0101, UC0104
 * @see {@link docs/features.md} FEAT0601, FEAT0202
 */
'use client';

import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert';
import { Button } from '@/components/ui/button';
import { ResizablePanel } from '@/components/ui/resizable-panel';
import { ScrollArea } from '@/components/ui/scroll-area';
import {
    Sheet,
    SheetContent,
    SheetHeader,
    SheetTitle,
} from '@/components/ui/sheet';
import { useGraphExpansion } from '@/hooks/use-graph-expansion';
import { useGraphKeyboardNavigation } from '@/hooks/use-graph-keyboard-navigation';
import { useGraphStream } from '@/hooks/use-graph-stream';
import { useMediaQuery } from '@/hooks/use-media-query';
import { getGraph } from '@/lib/api/edgequake';
import { focusCameraOnNode } from '@/lib/graph/camera-utils';
import { useGraphStore } from '@/stores/use-graph-store';
import { useTenantStore } from '@/stores/use-tenant-store';
import type { GraphNode } from '@/types';
import { useQuery } from '@tanstack/react-query';
import { AlertCircle, ChevronLeft, ChevronRight, Filter, Loader2, Maximize2, Menu, Network, PanelRightClose, RefreshCw, Upload, ZoomIn, ZoomOut } from 'lucide-react';
import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { toast } from 'sonner';
import { GraphEmptyIllustration } from '../illustrations/graph-empty-illustration';
import { BookmarksPanel } from './bookmarks-panel';
import { EntityBrowserPanel } from './entity-browser-panel';
import { GraphAccessibilityAnnouncer } from './graph-accessibility-announcer';
import { GraphControls } from './graph-controls';
import { GraphExport } from './graph-export';
import { GraphFilters } from './graph-filters';
import { GraphLegend } from './graph-legend';
import { GraphLoadingOverlay } from './graph-loading-overlay';
import { GraphMinimap } from './graph-minimap';
import { GraphRenderer } from './graph-renderer';
import { GraphSearch } from './graph-search';
import { GraphSettingsPanel } from './graph-settings-panel';
import { KeyboardShortcutsHelp } from './keyboard-shortcuts-help';
import { LayoutControl } from './layout-control';
import { LayoutController } from './layout-controller';
import { NodeContextMenu, useNodeContextMenu } from './node-context-menu';
import { NodeDetails } from './node-details';
import { StreamingIndicator, StreamingProgressBar } from './streaming-indicator';
import { TimeFilter } from './time-filter';
import { TruncationBanner, TruncationIndicator } from './truncation-banner';
import { ZoomControls } from './zoom-controls';

export function GraphViewer() {
  // Responsive breakpoints
  const isMobile = useMediaQuery('(max-width: 640px)');
  const isTablet = useMediaQuery('(min-width: 641px) and (max-width: 1024px)');
  const isSmallScreen = isMobile || isTablet;
  
  // Mobile drawer states
  const [mobileEntityDrawerOpen, setMobileEntityDrawerOpen] = useState(false);
  const [mobileDetailsDrawerOpen, setMobileDetailsDrawerOpen] = useState(false);
  const [mobileLegendVisible, setMobileLegendVisible] = useState(false);
  
  const {
    nodes: allNodes,
    edges: allEdges,
    selectedNodeId,
    showNodeDetails,
    rightPanelCollapsed,
    sigmaInstance,
    setGraph,
    selectNode,
    toggleNodeDetails,
    toggleRightPanel,
    hoverNode,
    setLoading,
    setError,
    visibleEntityTypes,
    visibleRelationshipTypes,
    searchQuery,
  } = useGraphStore();

  // Get tenant context for query key
  const { selectedTenantId, selectedWorkspaceId } = useTenantStore();

  // Memoize filtered nodes to prevent re-render loops
  const filteredNodes = useMemo(() => {
    return allNodes.filter((node) => {
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
  }, [allNodes, visibleEntityTypes, searchQuery]);

  // Memoize filtered edges
  const filteredEdges = useMemo(() => {
    const nodeIds = new Set(filteredNodes.map((n) => n.id));
    return allEdges.filter((edge) => {
      if (!visibleRelationshipTypes.has(edge.relationship_type)) return false;
      return nodeIds.has(edge.source) && nodeIds.has(edge.target);
    });
  }, [allEdges, filteredNodes, visibleRelationshipTypes]);

  // Context menu state
  const {
    contextMenuNode,
    contextMenuPosition,
    openContextMenu,
    closeContextMenu,
  } = useNodeContextMenu();

  // Initialize graph expansion hook (handles expand/prune logic)
  const { expandedNodes } = useGraphExpansion();
  
  // Get expand/prune triggers from store
  const triggerNodeExpand = useGraphStore((s) => s.triggerNodeExpand);
  const triggerNodePrune = useGraphStore((s) => s.triggerNodePrune);
  
  // Virtual query settings for SOTA 100k+ node support
  const maxNodes = useGraphStore((s) => s.maxNodes);
  const depth = useGraphStore((s) => s.depth);
  const startNode = useGraphStore((s) => s.startNode);
  const setStartNode = useGraphStore((s) => s.setStartNode);
  const setTruncationInfo = useGraphStore((s) => s.setTruncationInfo);
  
  // Streaming state for progressive loading
  const useStreaming = useGraphStore((s) => s.useStreaming);
  const addNodesToGraph = useGraphStore((s) => s.addNodesToGraph);
  const clearGraphForStreaming = useGraphStore((s) => s.clearGraphForStreaming);
  const setStreamingProgress = useGraphStore((s) => s.setStreamingProgress);
  const resetStreamingProgress = useGraphStore((s) => s.resetStreamingProgress);
  
  // Streaming hook for progressive graph loading
  const {
    nodes: streamedNodes,
    edges: streamedEdges,
    progress: streamingProgress,
    error: streamingError,
    isStreaming,
    startStream,
    cancel: cancelStream,
    reset: resetStream,
  } = useGraphStream({
    enabled: false, // Manual control - don't auto-start
    maxNodes,
    startNode: startNode || undefined,
    onMetadata: (metadata) => {
      // Clear existing graph when new streaming starts
      clearGraphForStreaming();
      setStreamingProgress({
        phase: 'metadata',
        totalNodes: metadata.nodes_to_stream,
        totalBatches: Math.ceil(metadata.nodes_to_stream / 50), // Default batch size
      });
    },
    onNodesBatch: (nodes, batchNumber, totalBatches) => {
      // Progressively add nodes to graph
      addNodesToGraph(nodes, []);
      setStreamingProgress({
        phase: 'nodes',
        nodesLoaded: streamedNodes.length + nodes.length,
        batchNumber,
        totalBatches,
      });
    },
    onEdges: (edges) => {
      // Add all edges at once
      addNodesToGraph([], edges);
      setStreamingProgress({
        phase: 'edges',
        edgesLoaded: edges.length,
      });
    },
    onComplete: (stats) => {
      setStreamingProgress({
        phase: 'complete',
        durationMs: stats.duration_ms,
        nodesLoaded: stats.nodes_count,
        edgesLoaded: stats.edges_count,
      });
      setTruncationInfo(
        stats.nodes_count < maxNodes, // Assume truncated if less than max
        stats.nodes_count,
        stats.edges_count
      );
    },
    onError: (error) => {
      setStreamingProgress({
        phase: 'error',
        errorMessage: error.message,
      });
      toast.error(`Failed to load graph: ${error.message}`);
    },
  });

  // Enable keyboard navigation for graph
  useGraphKeyboardNavigation({
    enabled: true,
    onNodeFocus: (nodeId) => {
      // Node focus is handled by the hook itself
    },
    onDeselect: () => {
      // Deselection is handled by the hook
    },
  });

  // Standard query for non-streaming mode (fallback)
  const { data, isLoading: isQueryLoading, isError, error, refetch } = useQuery({
    queryKey: ['graph', selectedTenantId, selectedWorkspaceId, maxNodes, depth, startNode],
    queryFn: () => getGraph({ 
      maxNodes,
      depth,
      startNode: startNode || undefined,
    }),
    staleTime: 5 * 60 * 1000, // 5 minutes - longer cache for better perf
    refetchOnWindowFocus: false, // Disable auto-refetch for better performance
    enabled: !useStreaming, // Disable when streaming is enabled
  });

  // Combined loading state
  const isLoading = useStreaming ? isStreaming : isQueryLoading;
  
  // WHY: When streaming is enabled but the useEffect hasn't fired yet to call
  // startStream(), isStreaming is false and allNodes is empty. Without this check,
  // users see a brief flash of "No knowledge graph yet" empty state before the
  // stream starts (~1 frame). Also covers the period during dynamic import when
  // GraphViewer just mounted but streaming hasn't initialized.
  // The !selectedTenantId || !selectedWorkspaceId check covers the race condition
  // where the first stream call happens before tenant/workspace context is available.
  const isStreamingInitializing = useStreaming && !isStreaming && allNodes.length === 0 
    && !isError && (
      !selectedTenantId || !selectedWorkspaceId 
      || streamingProgress.phase === 'idle' 
      || streamingProgress.phase === 'connecting'
    );
  // WHY: When a tenant/workspace switch happens, streaming for an empty workspace
  // can complete in <1 frame — the user sees the old graph vanish with zero feedback.
  // This transition state guarantees a minimum 800ms loading overlay so the user
  // always perceives "something happened" after switching context.
  const [isWorkspaceTransitioning, setIsWorkspaceTransitioning] = useState(false);
  const [transitionPhase, setTransitionPhase] = useState<string>("");
  const transitionTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const effectiveIsLoading = isLoading || isStreamingInitializing || isWorkspaceTransitioning;
  
  // WHY: Ref to prevent React StrictMode double-render from causing duplicate stream starts
  const streamingInitializedRef = useRef(false);
  const lastStreamParamsRef = useRef<string>("");
  
  // WHY: Track previous workspace/tenant to detect changes.
  // When workspace changes, the Zustand store still holds old nodes/edges from
  // the previous workspace. Without clearing, those stale nodes remain visible
  // until new data arrives. The transition state ensures the loading overlay
  // stays visible for at least 800ms so users see clear visual feedback.
  const prevWorkspaceKeyRef = useRef<string>("");
  useEffect(() => {
    const currentKey = `${selectedTenantId ?? ""}-${selectedWorkspaceId ?? ""}`;
    if (prevWorkspaceKeyRef.current !== "" && prevWorkspaceKeyRef.current !== currentKey) {
      clearGraphForStreaming();
      // WHY: Show loading overlay immediately with contextual message.
      // The 800ms minimum guarantees users see feedback even for fast/empty workspaces.
      setIsWorkspaceTransitioning(true);
      setTransitionPhase("Switching workspace...");
      if (transitionTimerRef.current) clearTimeout(transitionTimerRef.current);
      transitionTimerRef.current = setTimeout(() => {
        setIsWorkspaceTransitioning(false);
        setTransitionPhase("");
      }, 800);
    }
    prevWorkspaceKeyRef.current = currentKey;
    return () => { if (transitionTimerRef.current) clearTimeout(transitionTimerRef.current); };
  }, [selectedTenantId, selectedWorkspaceId, clearGraphForStreaming]);

  // Start streaming when in streaming mode
  useEffect(() => {
    if (!useStreaming) {
      streamingInitializedRef.current = false;
      return;
    }
    
    // WHY: Create param key to detect if we need to restart stream
    const paramKey = `${selectedTenantId}-${selectedWorkspaceId}-${maxNodes}-${startNode || ""}`;
    
    // WHY: Skip if already initialized with same params (prevents duplicate calls)
    if (streamingInitializedRef.current && lastStreamParamsRef.current === paramKey) {
      return;
    }
    
    // WHY: Clear stale graph data IMMEDIATELY when params change.
    // Without this, nodes/edges from a previous workspace or query remain visible
    // until the new stream's onMetadata callback fires (which can take seconds).
    // This is the root cause of "stale data from previous execution" bug.
    if (lastStreamParamsRef.current !== "" && lastStreamParamsRef.current !== paramKey) {
      clearGraphForStreaming();
    }
    
    streamingInitializedRef.current = true;
    lastStreamParamsRef.current = paramKey;
    resetStreamingProgress();
    startStream();
    
    // Cleanup: cancel stream on unmount or when switching modes
    return () => {
      if (useStreaming) {
        cancelStream();
        streamingInitializedRef.current = false;
      }
    };
    // Only re-run when these key params change
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [useStreaming, selectedTenantId, selectedWorkspaceId, maxNodes, startNode]);

  // Handle refetch for both modes
  const handleRefetch = useCallback(() => {
    if (useStreaming) {
      cancelStream();
      resetStreamingProgress();
      clearGraphForStreaming();
      startStream();
    } else {
      refetch();
    }
  }, [useStreaming, cancelStream, resetStreamingProgress, clearGraphForStreaming, startStream, refetch]);

  // Set graph data from non-streaming query (when streaming is disabled)
  useEffect(() => {
    if (data && !useStreaming) {
      setGraph(data);
      // Update truncation info from server response
      setTruncationInfo(
        data.is_truncated ?? false,
        data.total_nodes ?? data.nodes.length,
        data.total_edges ?? data.edges.length
      );
    }
  }, [data, setGraph, setTruncationInfo, useStreaming]);

  useEffect(() => {
    setLoading(effectiveIsLoading);
  }, [effectiveIsLoading, setLoading]);

  useEffect(() => {
    if (error) {
      setError(error instanceof Error ? error.message : 'Failed to load graph');
    }
  }, [error, setError]);

  const handleZoomIn = () => {
    if (sigmaInstance) {
      const camera = sigmaInstance.getCamera();
      camera.animatedZoom({ factor: 1.5 });
    }
  };

  const handleZoomOut = () => {
    if (sigmaInstance) {
      const camera = sigmaInstance.getCamera();
      camera.animatedUnzoom({ factor: 1.5 });
    }
  };

  const handleResetZoom = () => {
    if (sigmaInstance) {
      const camera = sigmaInstance.getCamera();
      camera.animatedReset();
    }
  };

  // Context menu handlers
  const handleNodeRightClick = useCallback((nodeId: string, x: number, y: number) => {
    const node = allNodes.find((n) => n.id === nodeId);
    if (node) {
      openContextMenu(node, x, y);
    }
  }, [allNodes, openContextMenu]);

  const handleViewDetails = useCallback((node: GraphNode) => {
    selectNode(node.id);
  }, [selectNode]);

  const handleExpandNeighborhood = useCallback((node: GraphNode) => {
    // Trigger node expansion via the store (handled by useGraphExpansion hook)
    triggerNodeExpand(node.id);
    
    // Focus camera on this node
    if (sigmaInstance) {
      focusCameraOnNode(sigmaInstance, node.id, {
        ratio: 0.3,
        duration: 500,
        highlight: false,
      });
    }
    selectNode(node.id);
  }, [sigmaInstance, selectNode, triggerNodeExpand]);

  const handlePruneNode = useCallback((node: GraphNode) => {
    // Trigger node pruning via the store (handled by useGraphExpansion hook)
    triggerNodePrune(node.id);
  }, [triggerNodePrune]);

  const handleFindRelated = useCallback((node: GraphNode) => {
    // Navigate to query page with pre-filled query
    window.location.href = `/query?q=Find entities related to ${encodeURIComponent(node.label)}`;
  }, []);

  const handleViewDocuments = useCallback((node: GraphNode) => {
    // Navigate to documents page with entity filter
    window.location.href = `/documents?entity=${encodeURIComponent(node.id)}`;
  }, []);

  const handleCopyId = useCallback((node: GraphNode) => {
    navigator.clipboard.writeText(node.id);
    toast.success(`Copied entity ID: ${node.id}`);
  }, []);

  // Handle settings change (triggers refetch)
  const handleSettingsChange = useCallback(() => {
    // Refetch is automatic via queryKey change when settings update the store
  }, []);

  const selectedNode = allNodes.find((n) => n.id === selectedNodeId);

  // Combine error states from both streaming and non-streaming modes
  const hasError = isError || (streamingError && !isStreaming);
  const errorMessage = error instanceof Error 
    ? error.message 
    : streamingError?.message || 'Failed to load knowledge graph';

  if (hasError && allNodes.length === 0) {
    return (
      <div className="p-6">
        <Alert variant="destructive">
          <AlertCircle className="h-4 w-4" />
          <AlertTitle>Error loading graph</AlertTitle>
          <AlertDescription>
            {errorMessage}
            <Button variant="link" className="ml-2 p-0" onClick={handleRefetch}>
              Try again
            </Button>
          </AlertDescription>
        </Alert>
      </div>
    );
  }

  return (
    <div className="flex h-full overflow-hidden">
      {/* Left Entity Browser - Hidden on mobile, shown on tablet+ */}
      {!isMobile && <EntityBrowserPanel />}

      {/* Main Graph Area */}
      <div className="flex-1 flex flex-col min-w-0 overflow-hidden">
        {/* Toolbar - compact and slick */}
        <header 
          className="flex items-center justify-between border-b px-2 sm:px-4 py-2 shrink-0 bg-card/50 backdrop-blur-sm"
          data-tour="graph-header"
        >
          <div className="flex items-center gap-1.5 sm:gap-2.5">
            {/* Mobile menu button */}
            {isMobile && (
              <Button 
                variant="ghost" 
                size="icon" 
                className="h-7 w-7"
                onClick={() => setMobileEntityDrawerOpen(true)}
                aria-label="Open entity browser"
              >
                <Menu className="h-4 w-4" />
              </Button>
            )}
            <h2 className="text-sm sm:text-base font-semibold tracking-tight">
              {isMobile ? 'Graph' : 'Knowledge Graph'}
            </h2>
            {effectiveIsLoading && <Loader2 className="h-3.5 w-3.5 animate-spin text-muted-foreground" />}
            {data?.metadata && !isMobile && (
              <span className="text-xs text-muted-foreground bg-muted px-2 py-0.5 rounded-md">
                {data.metadata.node_count.toLocaleString()} nodes · {data.metadata.edge_count.toLocaleString()} edges
              </span>
            )}
          </div>
          <div className="flex items-center gap-0.5 sm:gap-1">
            {/* Show filter button on mobile and tablet (right panel is hidden) */}
            {isSmallScreen && (
              <Button 
                variant="ghost" 
                size="icon" 
                className="h-7 w-7"
                onClick={() => setMobileDetailsDrawerOpen(true)}
                aria-label="Open filters"
              >
                <Filter className="h-3.5 w-3.5" />
              </Button>
            )}
            <div data-tour="graph-search"><GraphSearch /></div>
            {/* Truncation indicator (compact) */}
            {!isMobile && <TruncationIndicator />}
            <div data-tour="layout-control"><LayoutControl /></div>
            <LayoutController />
            {!isMobile && <GraphExport />}
            {/* Graph settings panel for virtual query */}
            {!isMobile && (
              <GraphSettingsPanel onSettingsChange={handleSettingsChange} />
            )}
            {!isMobile && <div data-tour="keyboard-help"><KeyboardShortcutsHelp /></div>}
            {!isMobile && <div className="w-px h-5 bg-border mx-1" />}
            <Button variant="ghost" size="icon" className="h-7 w-7" onClick={handleRefetch} title="Refresh">
              <RefreshCw className="h-3.5 w-3.5" />
            </Button>
            {!isMobile && (
              <>
                <Button variant="ghost" size="icon" className="h-7 w-7" onClick={handleZoomIn} title="Zoom In">
                  <ZoomIn className="h-3.5 w-3.5" />
                </Button>
                <Button variant="ghost" size="icon" className="h-7 w-7" onClick={handleZoomOut} title="Zoom Out">
                  <ZoomOut className="h-3.5 w-3.5" />
                </Button>
                <Button variant="ghost" size="icon" className="h-7 w-7" onClick={handleResetZoom} title="Reset View">
                  <Maximize2 className="h-3.5 w-3.5" />
                </Button>
              </>
            )}
          </div>
        </header>

        {/* Graph Canvas - bg-background ensures proper theme in fullscreen */}
        {/* WHY: role="application" tells screen readers this is an interactive app */}
        <div 
          className="flex-1 relative overflow-hidden bg-background text-foreground" 
          data-graph-container
          data-tour="graph-canvas"
          role="application"
          aria-label="Knowledge Graph Visualization - use Tab to navigate nodes, Enter to focus, Escape to deselect"
        >
          {/* Screen reader announcements for node selection */}
          <GraphAccessibilityAnnouncer />
          
          {effectiveIsLoading && allNodes.length === 0 ? (
            <GraphLoadingOverlay visible={true} phase={transitionPhase || undefined} />
          ) : allNodes.length === 0 ? (
            <div className="absolute inset-0 flex items-center justify-center">
              <div className="text-center max-w-md px-4">
                <div className="w-48 h-40 mx-auto mb-6">
                  <GraphEmptyIllustration animate={true} />
                </div>
                <h3 className="text-lg font-medium">No knowledge graph yet</h3>
                <p className="text-sm text-muted-foreground mt-2 mb-6">
                  Your knowledge graph is empty. Upload documents to automatically extract entities and relationships.
                </p>
                <Button
                  onClick={() => window.location.href = '/documents'}
                >
                  <Upload className="h-4 w-4 mr-2" />
                  Upload Documents
                </Button>
              </div>
            </div>
          ) : filteredNodes.length === 0 ? (
            <div className="absolute inset-0 flex items-center justify-center">
              <div className="text-center max-w-md px-4">
                <div className="w-40 h-32 mx-auto mb-4 opacity-50">
                  <GraphEmptyIllustration animate={false} />
                </div>
                <h3 className="text-lg font-medium">No visible nodes</h3>
                <p className="text-sm text-muted-foreground mt-2">
                  All node types are hidden. Use the legend below to show node categories.
                </p>
              </div>
            </div>
          ) : (
            <>
              <GraphRenderer
                nodes={filteredNodes}
                edges={filteredEdges}
                onNodeClick={selectNode}
                onNodeHover={hoverNode}
                onNodeRightClick={handleNodeRightClick}
              />
              
              {/* Truncation Banner - Shows when graph is truncated */}
              <TruncationBanner 
                onLoadMore={() => {
                  // WHY: Enforce max 500 nodes for performance
                  const currentMax = useGraphStore.getState().maxNodes;
                  useGraphStore.getState().setMaxNodes(Math.min(currentMax * 1.5, 500));
                }}
                isLoading={isLoading}
              />
              
              {/* Streaming Progress Indicator - Shows during progressive loading */}
              {useStreaming && isStreaming && (
                <>
                  <StreamingProgressBar 
                    progress={streamingProgress}
                    className="absolute top-0 left-0 right-0 z-20"
                  />
                  <StreamingIndicator 
                    progress={streamingProgress}
                    className="absolute top-4 left-1/2 -translate-x-1/2 z-20"
                    compact={isMobile}
                  />
                </>
              )}
              
              {/* Loading Overlay - Only for non-streaming refetch */}
              {isLoading && !useStreaming && allNodes.length > 0 && (
                <GraphLoadingOverlay visible={true} phase="Refreshing graph..." />
              )}
            </>
          )}

          {/* Node Context Menu */}
          <NodeContextMenu
            node={contextMenuNode}
            position={contextMenuPosition}
            onClose={closeContextMenu}
            onViewDetails={handleViewDetails}
            onExpandNeighborhood={handleExpandNeighborhood}
            onPruneNode={handlePruneNode}
            onFindRelated={handleFindRelated}
            onViewDocuments={handleViewDocuments}
            onCopyId={handleCopyId}
            isExpanded={contextMenuNode ? expandedNodes.has(contextMenuNode.id) : false}
          />

          {/* Graph Controls Overlay - Top Left */}
          <div className="absolute top-4 left-4 flex flex-col gap-2 z-20">
            <GraphControls />
          </div>

          {/* Minimap Overlay - Below controls on left side */}
          {!isMobile && filteredNodes.length > 0 && (
            <div className="absolute top-20 left-4 z-10">
              <GraphMinimap width={140} height={100} />
            </div>
          )}

          {/* Time Filter Overlay - Below Minimap on Left */}
          {!isMobile && filteredNodes.length > 0 && (
            <div className="absolute top-44 left-4 z-10">
              <TimeFilter collapsed />
            </div>
          )}

          {/* Bookmarks Panel - Below Time Filter on Left */}
          {!isMobile && filteredNodes.length > 0 && (
            <div className="absolute top-56 left-4 z-10">
              <BookmarksPanel collapsed />
            </div>
          )}

          {/* Zoom Controls Overlay - Right Side */}
          <div className="absolute top-4 right-4 flex flex-col gap-2">
            <ZoomControls />
          </div>
          
          {/* Legend Overlay - Bottom Right (toggle on mobile) */}
          {isMobile ? (
            mobileLegendVisible && (
              <div className="absolute bottom-4 left-1/2 -translate-x-1/2 z-20">
                <GraphLegend />
              </div>
            )
          ) : (
            <div className="absolute bottom-4 right-4">
              <GraphLegend />
            </div>
          )}
          
          {/* Mobile legend toggle */}
          {isMobile && (
            <Button
              variant="secondary"
              size="sm"
              className="absolute bottom-4 right-4 h-8 text-xs shadow-md"
              onClick={() => setMobileLegendVisible(!mobileLegendVisible)}
            >
              {mobileLegendVisible ? 'Hide Legend' : 'Legend'}
            </Button>
          )}
        </div>
      </div>

      {/* Right Sidebar - Hidden on mobile and tablet */}
      {!isSmallScreen && (
        rightPanelCollapsed ? (
          <div className="flex flex-col items-center py-2 w-10 border-l bg-card/80 backdrop-blur-sm shrink-0 transition-all duration-200">
            <Button
              variant="ghost"
              size="icon"
              className="h-7 w-7 hover:bg-muted"
              onClick={toggleRightPanel}
              aria-label="Expand details panel"
            >
              <ChevronLeft className="h-3.5 w-3.5" />
            </Button>
            <div className="mt-3 flex flex-col items-center gap-1.5">
              <PanelRightClose className="h-3.5 w-3.5 text-muted-foreground" />
              <span
                className="text-[10px] text-muted-foreground font-medium"
                style={{ writingMode: 'vertical-rl', textOrientation: 'mixed' }}
              >
                Details
              </span>
            </div>
          </div>
        ) : (
          <ResizablePanel
            side="right"
            defaultWidth={400}
            minWidth={280}
            maxWidth={480}
            className="border-l bg-card/95 backdrop-blur-sm"
            storageKey="edgequake.graph.rightPanelWidth"
            ariaLabel="Resize details panel"
          >
            <div className="flex flex-col h-full overflow-hidden" data-tour="details-panel">
              {/* Panel Header */}
              <div className="flex items-center justify-between px-4 py-2.5 border-b shrink-0 bg-muted/30">
                <h3 className="text-xs font-semibold text-muted-foreground uppercase tracking-wide">Details & Filters</h3>
                <Button
                  variant="ghost"
                  size="icon"
                  className="h-6 w-6"
                  onClick={toggleRightPanel}
                  aria-label="Collapse details panel"
                >
                  <ChevronRight className="h-3.5 w-3.5" />
                </Button>
              </div>

              {/* Panel Content - Full height scroll */}
              <ScrollArea className="flex-1 min-h-0 [&_[data-slot=scroll-area-viewport]>div]:!block" showShadows>
                <div className="px-4 py-4 space-y-5 overflow-hidden">
                  {/* Node Details - Primary content when selected */}
                  {selectedNode && showNodeDetails && (
                    <NodeDetails node={selectedNode} />
                  )}

                  {/* Show details button when panel is hidden but node is selected */}
                  {selectedNode && !showNodeDetails && (
                    <Button
                      variant="outline"
                      size="sm"
                      className="w-full h-8 text-xs"
                      onClick={toggleNodeDetails}
                    >
                      Show Node Details
                    </Button>
                  )}

                  {/* Empty state when no node selected */}
                  {!selectedNode && (
                    <div className="py-8 text-center">
                      <div className="w-10 h-10 mx-auto mb-2 rounded-full bg-muted/50 flex items-center justify-center">
                        <Network className="h-5 w-5 text-muted-foreground/50" />
                      </div>
                      <p className="text-xs text-muted-foreground">
                        Click on a node to view details
                      </p>
                    </div>
                  )}

                  {/* Filters Section */}
                  <div className="pt-4 border-t">
                    <GraphFilters />
                  </div>
                </div>
              </ScrollArea>
            </div>
          </ResizablePanel>
        )
      )}
      
      {/* Mobile Entity Browser Drawer */}
      <Sheet open={mobileEntityDrawerOpen} onOpenChange={setMobileEntityDrawerOpen}>
        <SheetContent side="left" className="w-75 p-0">
          <SheetHeader className="px-4 py-3 border-b">
            <SheetTitle className="text-sm flex items-center gap-2">
              <Network className="h-4 w-4" />
              Entity Browser
            </SheetTitle>
          </SheetHeader>
          <ScrollArea className="h-[calc(100vh-60px)]">
            <div className="p-4">
              <EntityBrowserPanel className="w-full border-none" />
            </div>
          </ScrollArea>
        </SheetContent>
      </Sheet>
      
      {/* Mobile Details/Filters Drawer */}
      <Sheet open={mobileDetailsDrawerOpen} onOpenChange={setMobileDetailsDrawerOpen}>
        <SheetContent side="right" className="w-75 p-0">
          <SheetHeader className="px-4 py-3 border-b">
            <SheetTitle className="text-sm flex items-center gap-2">
              <Filter className="h-4 w-4" />
              Details & Filters
            </SheetTitle>
          </SheetHeader>
          <ScrollArea className="h-[calc(100vh-60px)]">
            <div className="p-4 space-y-4">
              {/* Node Details - Primary content when selected */}
              {selectedNode && showNodeDetails && (
                <NodeDetails node={selectedNode} />
              )}
              
              {/* Empty state when no node selected */}
              {!selectedNode && (
                <div className="py-6 text-center">
                  <div className="w-10 h-10 mx-auto mb-2 rounded-full bg-muted/50 flex items-center justify-center">
                    <Network className="h-5 w-5 text-muted-foreground/50" />
                  </div>
                  <p className="text-xs text-muted-foreground">
                    Tap on a node to view details
                  </p>
                </div>
              )}

              {/* Filters Section */}
              <div className="pt-3 border-t">
                <GraphFilters />
              </div>
            </div>
          </ScrollArea>
        </SheetContent>
      </Sheet>
    </div>
  );
}

export default GraphViewer;
