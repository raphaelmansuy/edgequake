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
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import { ResizablePanel } from '@/components/ui/resizable-panel';
import { ScrollArea } from '@/components/ui/scroll-area';
import {
    Sheet,
    SheetContent,
    SheetHeader,
    SheetTitle,
} from '@/components/ui/sheet';
import { useGraphExpansion } from '@/hooks/use-graph-expansion';
import { useGraphDocumentScope } from '@/hooks/use-graph-document-scope';
import { useGraphDocumentFilterUrl } from '@/hooks/use-graph-document-filter';
import { useGraphKeyboardNavigation } from '@/hooks/use-graph-keyboard-navigation';
import { useGraphStream } from '@/hooks/use-graph-stream';
import {
  entityHandoffFromNode,
  useQueryHandoff,
} from '@/hooks/use-query-handoff';
import { useMediaQuery } from '@/hooks/use-media-query';
import { deleteEntity, getGraph } from '@/lib/api/edgequake';
import { focusCameraOnNode } from '@/lib/graph/camera-utils';
import {
  filterGraphData,
  resolveTruncationInfo,
  type GraphFiltersState,
} from '@/lib/graph/engine';
import { formatEntityLabel } from '@/lib/graph/label-utils';
import { useGraphStore } from '@/stores/use-graph-store';
import { useTenantStore } from '@/stores/use-tenant-store';
import type { GraphNode } from '@/types';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { AlertCircle, ChevronLeft, ChevronRight, Filter, Loader2, Maximize2, Menu, MoreHorizontal, Network, PanelRightClose, RefreshCw, Table2, Upload } from 'lucide-react';
import { useRouter, useSearchParams } from 'next/navigation';
import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { toast } from 'sonner';
import { useShallow } from 'zustand/react/shallow';
import { GraphEmptyIllustration } from '../illustrations/graph-empty-illustration';
import { BookmarksPanel } from './bookmarks-panel';
import { EntityBrowserPanel } from './entity-browser-panel';
import { fitNeighbourhood } from '@/hooks/use-neighbourhood';
import { EgoDepthSlider } from './ego-depth-slider';
import { GraphAccessibilityAnnouncer } from './graph-accessibility-announcer';
import { GraphAsTable } from './graph-as-table';
import { GraphControls } from './graph-controls';
import { GraphDocumentFilterBar } from './graph-document-filter-bar';
import { GraphExport } from './graph-export';
import { GraphFilters } from './graph-filters';
import { GraphLegend } from './graph-legend';
import { GraphLoadingOverlay } from './graph-loading-overlay';
import { GraphMinimap } from './graph-minimap';
import { GraphRenderer } from './graph-renderer';
import { NoMatchingNodesHint } from './no-matching-nodes-hint';
import { GraphSearch } from './graph-search';
import { GraphSettingsPanel } from './graph-settings-panel';
import { KeyboardShortcutsHelp } from './keyboard-shortcuts-help';
import { LayoutControl } from './layout-control';
import { LayoutController } from './layout-controller';
import { NodeContextMenu, useNodeContextMenu } from './node-context-menu';
import { NodeDetails } from './node-details';
import { GraphTourTrigger } from './graph-tour-wrapper';
import { StreamingIndicator, StreamingProgressBar } from './streaming-indicator';
import { TimeFilter } from './time-filter';
import { TruncationBanner, TruncationIndicator } from './truncation-banner';
import { ZoomControls } from './zoom-controls';
import { isAutomatedBrowser } from '@/lib/runtime/browser-detection';
import { useAnswerGraphStore } from '@/stores/use-answer-graph-store';

export function GraphViewer() {
  const { documentFilterId, setDocumentFilter } = useGraphDocumentFilterUrl();
  const isDocumentScopedRef = useRef(!!documentFilterId);
  useEffect(() => {
    isDocumentScopedRef.current = !!documentFilterId;
  }, [documentFilterId]);

  const {
    isDocumentScoped,
    isLoading: isLineageLoading,
    isError: isLineageError,
    error: lineageError,
    refetch: refetchLineage,
  } = useGraphDocumentScope(documentFilterId);

  // Responsive breakpoints
  const isMobile = useMediaQuery('(max-width: 640px)');
  const isTablet = useMediaQuery('(min-width: 641px) and (max-width: 1024px)');
  const isSmallScreen = isMobile || isTablet;
  
  // Mobile drawer states
  const [mobileEntityDrawerOpen, setMobileEntityDrawerOpen] = useState(false);
  const [mobileDetailsDrawerOpen, setMobileDetailsDrawerOpen] = useState(false);
  const [mobileLegendVisible, setMobileLegendVisible] = useState(false);
  
  // G11: selectors + useShallow — hover must not re-render the whole viewer
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
    setSearchQuery,
    timeFilterEnabled,
    timeFilterStart,
    timeFilterEnd,
  } = useGraphStore(
    useShallow((s) => ({
      nodes: s.nodes,
      edges: s.edges,
      selectedNodeId: s.selectedNodeId,
      showNodeDetails: s.showNodeDetails,
      rightPanelCollapsed: s.rightPanelCollapsed,
      sigmaInstance: s.sigmaInstance,
      setGraph: s.setGraph,
      selectNode: s.selectNode,
      toggleNodeDetails: s.toggleNodeDetails,
      toggleRightPanel: s.toggleRightPanel,
      hoverNode: s.hoverNode,
      setLoading: s.setLoading,
      setError: s.setError,
      visibleEntityTypes: s.visibleEntityTypes,
      visibleRelationshipTypes: s.visibleRelationshipTypes,
      searchQuery: s.searchQuery,
      setSearchQuery: s.setSearchQuery,
      timeFilterEnabled: s.timeFilterEnabled,
      timeFilterStart: s.timeFilterStart,
      timeFilterEnd: s.timeFilterEnd,
    })),
  );

  const queryClient = useQueryClient();

  // Get tenant context for query key
  const { selectedTenantId, selectedWorkspaceId } = useTenantStore();
  const router = useRouter();
  const searchParams = useSearchParams();
  const viewMode = searchParams.get('view') === 'table' ? 'table' : 'canvas';
  const setEngineFocus = useGraphStore((s) => s.setEngineFocus);

  // SPEC-155 W6: answer-on-graph deep link
  useEffect(() => {
    const answerMessage = searchParams.get('answerMessage');
    const focusMode = searchParams.get('focus');
    if (!answerMessage && focusMode !== 'answer') return;
    if (allNodes.length === 0) return;

    const entry = answerMessage
      ? useAnswerGraphStore.getState().getAnswer(answerMessage)
      : useAnswerGraphStore.getState().lastAnswer;
    if (!entry) return;

    const byId = new Set(allNodes.map((n) => n.id));
    const byLabel = new Map(
      allNodes.map((n) => [n.label.toUpperCase(), n.id]),
    );
    const resolved: string[] = [];
    for (const id of entry.nodeIds) {
      if (byId.has(id)) resolved.push(id);
    }
    for (const name of entry.entityNames) {
      const hit = byLabel.get(name.toUpperCase());
      if (hit && !resolved.includes(hit)) resolved.push(hit);
    }
    if (resolved.length === 0) return;
    setEngineFocus({ mode: 'answer', ids: resolved });
    if (resolved[0]) selectNode(resolved[0]);
  }, [searchParams, allNodes, setEngineFocus, selectNode]);

  const toggleTableView = useCallback(() => {
    const params = new URLSearchParams(searchParams.toString());
    if (viewMode === 'table') {
      params.delete('view');
    } else {
      params.set('view', 'table');
    }
    const qs = params.toString();
    router.replace(qs ? `/graph?${qs}` : '/graph', { scroll: false });
  }, [router, searchParams, viewMode]);

  // Engine filter payload (dim layer) — time filter included (G06)
  const engineFilters = useMemo((): Partial<GraphFiltersState> & {
    types: Set<string>;
    relTypes: Set<string>;
  } => {
    return {
      types: visibleEntityTypes,
      relTypes: visibleRelationshipTypes,
      query: searchQuery,
      timeRange: {
        enabled: timeFilterEnabled,
        start: timeFilterStart,
        end: timeFilterEnd,
      },
    };
  }, [
    visibleEntityTypes,
    visibleRelationshipTypes,
    searchQuery,
    timeFilterEnabled,
    timeFilterStart,
    timeFilterEnd,
  ]);

  // UI-only filtered counts (empty-state); renderer gets full data + dim filters
  const { nodes: filteredNodes } = useMemo(
    () => filterGraphData(allNodes, allEdges, engineFilters as GraphFiltersState),
    [allNodes, allEdges, engineFilters],
  );

  // Stream metadata for truthful truncation (G03)
  const streamMetaRef = useRef<{
    total_nodes: number;
    total_edges: number;
    is_truncated?: boolean;
  } | null>(null);

  // Context menu state
  const {
    contextMenuNode,
    contextMenuPosition,
    openContextMenu,
    closeContextMenu,
  } = useNodeContextMenu();

  // Playwright-only hook: open the node context menu at a known viewport point
  // without depending on Sigma's pixel-precise right-click hit detection.
  useEffect(() => {
    if (!isAutomatedBrowser()) return;
    const handler = (event: Event) => {
      const detail = (event as CustomEvent<{ x?: number; y?: number; nodeId?: string }>).detail;
      const node =
        (detail?.nodeId
          ? allNodes.find((n) => n.id === detail.nodeId)
          : undefined) ?? allNodes[0];
      if (!node) return;
      openContextMenu(
        node,
        detail?.x ?? Math.floor(window.innerWidth / 2),
        detail?.y ?? Math.floor(window.innerHeight / 2),
      );
    };
    window.addEventListener('eq:e2e-open-node-menu', handler);
    return () => window.removeEventListener('eq:e2e-open-node-menu', handler);
  }, [allNodes, openContextMenu]);

  // Initialize graph expansion hook (handles expand/prune logic)
  const { expandedNodes } = useGraphExpansion();
  
  // Get expand/prune triggers from store
  const triggerNodeExpand = useGraphStore((s) => s.triggerNodeExpand);
  const triggerNodePrune = useGraphStore((s) => s.triggerNodePrune);
  
  // Virtual query settings for SOTA 100k+ node support
  const maxNodes = useGraphStore((s) => s.maxNodes);
  const depth = useGraphStore((s) => s.depth);
  const startNode = useGraphStore((s) => s.startNode);
  const setTruncationInfo = useGraphStore((s) => s.setTruncationInfo);
  
  // Streaming state for progressive loading
  const useStreaming = useGraphStore((s) => s.useStreaming);
  const setUseStreaming = useGraphStore((s) => s.setUseStreaming);
  const addNodesToGraph = useGraphStore((s) => s.addNodesToGraph);
  const clearGraphForStreaming = useGraphStore((s) => s.clearGraphForStreaming);
  const graphResetToken = useGraphStore((s) => s.graphResetToken);
  const setStreamingProgress = useGraphStore((s) => s.setStreamingProgress);
  const resetStreamingProgress = useGraphStore((s) => s.resetStreamingProgress);

  useEffect(() => {
    const streamMode = searchParams.get('stream');

    // WHY: Allow a deterministic fallback to non-streaming graph loading.
    // This helps users and browser tests avoid SSE timing issues when needed
    // without changing the default production behavior.
    if (streamMode === '0' || streamMode === 'false') {
      setUseStreaming(false);
      return;
    }

    if (streamMode === '1' || streamMode === 'true') {
      setUseStreaming(true);
      return;
    }

    // Document scope always uses lineage subgraph — never stream the full graph.
    if (isDocumentScoped) {
      setUseStreaming(false);
    }
  }, [searchParams, setUseStreaming, isDocumentScoped]);
  
  // Streaming hook for progressive graph loading
  const {
    nodes: streamedNodes,
    progress: streamingProgress,
    error: streamingError,
    isStreaming,
    startStream,
    cancel: cancelStream,
  } = useGraphStream({
    enabled: false, // Manual control - don't auto-start
    maxNodes,
    startNode: startNode || undefined,
    onMetadata: (metadata) => {
      if (isDocumentScopedRef.current) return;
      // Clear existing graph when new streaming starts
      clearGraphForStreaming();
      streamMetaRef.current = {
        total_nodes: metadata.total_nodes,
        total_edges: metadata.total_edges,
        is_truncated:
          metadata.total_nodes > metadata.nodes_to_stream ||
          metadata.total_edges > metadata.edges_to_stream,
      };
      setStreamingProgress({
        phase: 'metadata',
        totalNodes: metadata.nodes_to_stream,
        totalBatches: Math.ceil(metadata.nodes_to_stream / 50), // Default batch size
      });
    },
    onNodesBatch: (nodes, batchNumber, totalBatches) => {
      if (isDocumentScopedRef.current) return;
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
      if (isDocumentScopedRef.current) return;
      // Add all edges at once
      addNodesToGraph([], edges);
      setStreamingProgress({
        phase: 'edges',
        edgesLoaded: edges.length,
      });
    },
    onComplete: (stats) => {
      if (isDocumentScopedRef.current) return;
      setStreamingProgress({
        phase: 'complete',
        durationMs: stats.duration_ms,
        nodesLoaded: stats.nodes_count,
        edgesLoaded: stats.edges_count,
      });
      // G03: honour server totals from stream metadata — never invert vs maxNodes
      const meta = streamMetaRef.current;
      const truncation = resolveTruncationInfo({
        streamedNodes: stats.nodes_count,
        streamedEdges: stats.edges_count,
        totalNodes: meta?.total_nodes,
        totalEdges: meta?.total_edges,
        isTruncated: meta?.is_truncated,
      });
      setTruncationInfo(
        truncation.isTruncated,
        truncation.totalNodes,
        truncation.totalEdges,
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

  // Double-click = "focus here": select and frame the node's neighbourhood.
  const handleNodeDoubleClick = useCallback(
    (nodeId: string) => {
      selectNode(nodeId);
      fitNeighbourhood(nodeId);
    },
    [selectNode],
  );

  // Enable keyboard navigation for graph
  useGraphKeyboardNavigation({
    enabled: true,
    onOpenContextMenu: (nodeId, x, y) => {
      const node = allNodes.find((n) => n.id === nodeId);
      if (node) openContextMenu(node, x, y);
    },
    onNodeFocus: () => {
      // Node focus is handled by the hook itself
    },
    onDeselect: () => {
      // Deselection is handled by the hook
    },
  });

  const { data, isLoading: isQueryLoading, isError, error, refetch } = useQuery({
    queryKey: ['graph', selectedTenantId, selectedWorkspaceId, maxNodes, depth, startNode],
    queryFn: () => getGraph({ 
      maxNodes,
      depth,
      startNode: startNode || undefined,
    }),
    staleTime: 5 * 60 * 1000, // 5 minutes - longer cache for better perf
    refetchOnWindowFocus: false, // Disable auto-refetch for better performance
    enabled: !useStreaming && !isDocumentScoped, // Full graph only when not document-scoped
  });

  // Combined loading state
  const isLoading = isDocumentScoped
    ? isLineageLoading
    : useStreaming
      ? isStreaming
      : isQueryLoading;
  
  // WHY: When streaming is enabled but the useEffect hasn't fired yet to call
  // startStream(), isStreaming is false and allNodes is empty. Without this check,
  // users see a brief flash of "No knowledge graph yet" empty state before the
  // stream starts (~1 frame). Also covers the period during dynamic import when
  // GraphViewer just mounted but streaming hasn't initialized.
  // The !selectedTenantId || !selectedWorkspaceId check covers the race condition
  // where the first stream call happens before tenant/workspace context is available.
  const isStreamingInitializing = useStreaming && !isDocumentScoped && !isStreaming && allNodes.length === 0 
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
  const lastStreamScopeRef = useRef<string>("");
  
  // WHY: Track previous workspace/tenant to detect changes.
  // When workspace changes, the Zustand store still holds old nodes/edges from
  // the previous workspace. Without clearing, those stale nodes remain visible
  // until new data arrives. The transition state ensures the loading overlay
  // stays visible for at least 800ms so users see clear visual feedback.
  //
  // IMPORTANT: Do not put `setDocumentFilter` in deps — it is recreated on every
  // searchParams change and would clear the 800ms timer in cleanup without
  // restarting it, leaving `isWorkspaceTransitioning` stuck true forever.
  const prevWorkspaceKeyRef = useRef<string>("");
  const clearGraphRef = useRef(clearGraphForStreaming);
  useEffect(() => {
    clearGraphRef.current = clearGraphForStreaming;
  }, [clearGraphForStreaming]);
  const setDocumentFilterId = useGraphStore((s) => s.setDocumentFilterId);

  useEffect(() => {
    const currentKey = `${selectedTenantId ?? ""}-${selectedWorkspaceId ?? ""}`;
    if (prevWorkspaceKeyRef.current !== "" && prevWorkspaceKeyRef.current !== currentKey) {
      clearGraphRef.current();
      // Clear document scope in store only — avoid router.replace churn mid-transition.
      setDocumentFilterId(null);
      setIsWorkspaceTransitioning(true);
      setTransitionPhase("Switching workspace...");
      if (transitionTimerRef.current) clearTimeout(transitionTimerRef.current);
      transitionTimerRef.current = setTimeout(() => {
        transitionTimerRef.current = null;
        setIsWorkspaceTransitioning(false);
        setTransitionPhase("");
      }, 800);
    }
    prevWorkspaceKeyRef.current = currentKey;
  }, [selectedTenantId, selectedWorkspaceId, setDocumentFilterId]);

  // Unmount-only cleanup for the transition timer.
  useEffect(() => {
    return () => {
      if (transitionTimerRef.current) clearTimeout(transitionTimerRef.current);
    };
  }, []);

  // Start streaming when in streaming mode
  useEffect(() => {
    if (isDocumentScoped || documentFilterId) {
      streamingInitializedRef.current = false;
      if (isStreaming) {
        cancelStream();
      }
      return;
    }

    if (!useStreaming) {
      streamingInitializedRef.current = false;
      return;
    }
    
    // WHY: Create param key to detect if we need to restart stream
    const streamScopeKey = `${selectedTenantId}-${selectedWorkspaceId}-${maxNodes}-${startNode || ""}`;
    const paramKey = `${streamScopeKey}-${graphResetToken}`;
    
    // WHY: Skip if already initialized with same params (prevents duplicate calls)
    if (streamingInitializedRef.current && lastStreamParamsRef.current === paramKey) {
      return;
    }
    
    // WHY: Clear stale graph data when workspace/query scope changes — not when only
    // graphResetToken bumps (document delete already cleared via invalidateKnowledgeGraph).
    if (
      lastStreamScopeRef.current !== "" &&
      lastStreamScopeRef.current !== streamScopeKey
    ) {
      clearGraphForStreaming();
    }
    
    streamingInitializedRef.current = true;
    lastStreamParamsRef.current = paramKey;
    lastStreamScopeRef.current = streamScopeKey;
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
  }, [useStreaming, isDocumentScoped, documentFilterId, selectedTenantId, selectedWorkspaceId, maxNodes, startNode, graphResetToken]);

  // Handle refetch for both modes
  const handleRefetch = useCallback(() => {
    if (isDocumentScoped) {
      clearGraphForStreaming();
      refetchLineage();
      return;
    }
    if (useStreaming) {
      cancelStream();
      resetStreamingProgress();
      clearGraphForStreaming();
      startStream();
    } else {
      refetch();
    }
  }, [isDocumentScoped, useStreaming, cancelStream, resetStreamingProgress, clearGraphForStreaming, startStream, refetch, refetchLineage]);

  // Set graph data from non-streaming query (when streaming is disabled)
  useEffect(() => {
    if (data && !useStreaming && !isDocumentScoped) {
      setGraph(data);
      // Update truncation info from server response (G03)
      const truncation = resolveTruncationInfo({
        streamedNodes: data.nodes.length,
        streamedEdges: data.edges.length,
        totalNodes: data.total_nodes,
        totalEdges: data.total_edges,
        isTruncated: data.is_truncated,
      });
      setTruncationInfo(
        truncation.isTruncated,
        truncation.totalNodes,
        truncation.totalEdges,
      );
    }
  }, [data, setGraph, setTruncationInfo, useStreaming, isDocumentScoped]);

  useEffect(() => {
    setLoading(effectiveIsLoading);
  }, [effectiveIsLoading, setLoading]);

  useEffect(() => {
    const activeError = isDocumentScoped ? lineageError : error;
    if (activeError) {
      setError(activeError instanceof Error ? activeError.message : 'Failed to load graph');
    } else if (!isDocumentScoped && !error) {
      setError(null);
    }
  }, [error, lineageError, isDocumentScoped, setError]);

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

  // Context menu handlers
  // 1. View Details — select node AND open the right details panel
  const handleViewDetails = useCallback((node: GraphNode) => {
    selectNode(node.id);
    if (rightPanelCollapsed) toggleRightPanel();
    if (!showNodeDetails) toggleNodeDetails();
  }, [selectNode, rightPanelCollapsed, toggleRightPanel, showNodeDetails, toggleNodeDetails]);

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

  // 3. Prune Node — remove this node and its exclusive edges from the view
  const handlePruneNode = useCallback((node: GraphNode) => {
    // Trigger node pruning via the store (handled by useGraphExpansion hook)
    triggerNodePrune(node.id);
  }, [triggerNodePrune]);

  // 4. Find Related — use the graph's own search to highlight related nodes.
  // WHY: Previously used window.location.href which lost all graph state AND
  // the query page ignores the ?q= param. Staying on the graph page and setting
  // the search query is better UX: user sees the related nodes immediately.
  const handleFindRelated = useCallback((node: GraphNode) => {
    const label = formatEntityLabel(node.label ?? '');
    setSearchQuery(label);
    if (sigmaInstance) {
      focusCameraOnNode(sigmaInstance, node.id, { ratio: 0.4, duration: 600, highlight: false });
    }
    toast.info(`Showing nodes related to "${label}"`, { duration: 2500 });
  }, [setSearchQuery, sigmaInstance]);

  // SPEC-159: Ask about this — handoff uses the menu node (not ambient selection).
  const { ask: askQuery } = useQueryHandoff();
  const handleAskAboutThis = useCallback(
    (node: GraphNode) => {
      askQuery(entityHandoffFromNode(node));
    },
    [askQuery],
  );

  // 5. View Documents — navigate to documents page (workspace-scoped).
  // WHY: Previously used window.location.href (full page reload, loses state).
  // Using router.push preserves the Next.js client state and is faster.
  const handleViewDocuments = useCallback(() => {
    const searchParams = new URLSearchParams(window.location.search);
    const ws = searchParams.get('workspace');
    const destination = ws ? `/documents?workspace=${ws}` : '/documents';
    router.push(destination);
  }, [router]);

  // 6. Copy Entity ID — copy raw ID for API use; toast shows human-readable name
  const handleCopyId = useCallback((node: GraphNode) => {
    navigator.clipboard.writeText(node.id);
    const label = formatEntityLabel(node.label ?? node.id);
    toast.success(`Copied ID for "${label}"`);
  }, []);

  // Handle settings change (triggers Sigma refresh for visual query settings)
  const handleSettingsChange = useCallback(() => {
    // Sigma refreshes automatically via ref update inside GraphSettingsPanel
  }, []);

  // 7. Delete Entity — call API, invalidate graph cache, show toast
  const handleDeleteNode = useCallback(async (node: GraphNode) => {
    try {
      await deleteEntity(node.id);
      queryClient.invalidateQueries({ queryKey: ['graph'] });
      const label = formatEntityLabel(node.label ?? node.id);
      toast.success(`"${label}" deleted from knowledge graph`);
    } catch (err) {
      toast.error(
        `Failed to delete: ${err instanceof Error ? err.message : 'Unknown error'}`,
      );
    }
  }, [queryClient]);

  const selectedNode = allNodes.find((n) => n.id === selectedNodeId);
  const graphMetadata = useGraphStore((s) => s.graph?.metadata);
  const headerNodeCount = isDocumentScoped
    ? allNodes.length
    : (graphMetadata?.node_count ?? data?.metadata?.node_count ?? allNodes.length);
  const headerEdgeCount = isDocumentScoped
    ? allEdges.length
    : (graphMetadata?.edge_count ?? data?.metadata?.edge_count ?? allEdges.length);

  // Combine error states from streaming, full-graph, and document-scoped modes
  const hasError = isDocumentScoped
    ? isLineageError
    : isError || (streamingError && !isStreaming);
  const errorMessage = isDocumentScoped
    ? (lineageError instanceof Error
        ? lineageError.message
        : 'Failed to load document graph')
    : (error instanceof Error
        ? error.message
        : streamingError?.message || 'Failed to load knowledge graph');

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
        {/* Toolbar - compact; secondary actions collapse on mobile */}
        <header 
          className="eq-toolbar justify-between border-b px-2 sm:px-4 py-2 shrink-0 bg-card/90 backdrop-blur-sm"
          data-tour="graph-header"
        >
          <div className="flex items-center gap-1.5 sm:gap-2.5 min-w-0">
            {/* Mobile menu button */}
            {isMobile && (
              <Button 
                variant="ghost" 
                size="icon" 
                className="h-9 w-9 shrink-0"
                onClick={() => setMobileEntityDrawerOpen(true)}
                aria-label="Open entity browser"
              >
                <Menu className="h-4 w-4" />
              </Button>
            )}
            <h2 className="text-sm sm:text-base font-semibold tracking-tight truncate">
              {isMobile ? 'Graph' : 'Knowledge Graph'}
            </h2>
            {effectiveIsLoading && <Loader2 className="h-3.5 w-3.5 animate-spin text-muted-foreground shrink-0" />}
            {/* SPEC-100: always reserve count chip so load→data does not shove toolbar */}
            {!isMobile && (
              <span
                className="min-w-0 max-w-full truncate text-xs text-muted-foreground bg-muted px-2 py-0.5 rounded-md tabular-nums"
                data-testid="spec100-graph-count-slot"
              >
                {headerNodeCount > 0 || headerEdgeCount > 0 || !isDocumentScoped
                  ? `${headerNodeCount.toLocaleString()} nodes · ${headerEdgeCount.toLocaleString()} edges`
                  : "— nodes · — edges"}
              </span>
            )}
          </div>
          <div className="eq-toolbar min-w-0 shrink-0">
            {/* Show filter button on mobile and tablet (right panel is hidden) */}
            {isSmallScreen && (
              <Button 
                variant="ghost" 
                size="icon" 
                className="h-9 w-9"
                onClick={() => setMobileDetailsDrawerOpen(true)}
                aria-label="Open filters"
              >
                <Filter className="h-4 w-4" />
              </Button>
            )}
            {/* Search — always primary */}
            <div data-tour="graph-search"><GraphSearch /></div>
            {/* Accessible table alternative (SPEC-155 W5) */}
            <Button
              variant={viewMode === 'table' ? 'secondary' : 'ghost'}
              size="icon"
              className="h-9 w-9"
              onClick={toggleTableView}
              title={viewMode === 'table' ? 'Show canvas' : 'Show as table'}
              aria-label={viewMode === 'table' ? 'Show canvas' : 'Show as table'}
              data-testid="graph-view-table-toggle"
            >
              {viewMode === 'table' ? (
                <Network className="h-4 w-4" />
              ) : (
                <Table2 className="h-4 w-4" />
              )}
            </Button>

            {/* Desktop secondary toolbar (desktop only — tablet uses overflow) */}
            {!isSmallScreen && (
              <>
                <TruncationIndicator />
                <div className="w-px h-4 bg-border/60 mx-0.5" aria-hidden="true" />
                <div data-tour="layout-control"><LayoutControl /></div>
                <LayoutController />
                <div className="w-px h-4 bg-border/60 mx-0.5" aria-hidden="true" />
                <GraphExport />
                <Button variant="ghost" size="icon" className="h-9 w-9" onClick={handleRefetch} title="Refresh graph data">
                  <RefreshCw className="h-4 w-4" />
                </Button>
                <div className="w-px h-4 bg-border/60 mx-0.5" aria-hidden="true" />
                <GraphSettingsPanel onSettingsChange={handleSettingsChange} />
                <div data-tour="keyboard-help"><KeyboardShortcutsHelp /></div>
              </>
            )}

            {/* Mobile/tablet overflow — layout / export / refresh / settings */}
            {isSmallScreen && (
              <DropdownMenu>
                <DropdownMenuTrigger asChild>
                  <Button
                    variant="ghost"
                    size="icon"
                    className="h-9 w-9"
                    aria-label="More graph actions"
                    data-testid="graph-toolbar-more"
                  >
                    <MoreHorizontal className="h-4 w-4" />
                  </Button>
                </DropdownMenuTrigger>
                <DropdownMenuContent align="end" className="w-48">
                  <DropdownMenuItem onSelect={() => handleRefetch()}>
                    <RefreshCw className="h-4 w-4 mr-2" />
                    Refresh
                  </DropdownMenuItem>
                  <DropdownMenuItem onSelect={() => handleResetZoom()}>
                    <Maximize2 className="h-4 w-4 mr-2" />
                    Fit to screen
                  </DropdownMenuItem>
                  <DropdownMenuSeparator />
                  <div className="px-2 py-1.5" data-tour="layout-control">
                    <LayoutControl />
                  </div>
                  <div className="px-2 py-1">
                    <GraphExport />
                  </div>
                  <div className="px-2 py-1">
                    <GraphSettingsPanel onSettingsChange={handleSettingsChange} />
                  </div>
                </DropdownMenuContent>
              </DropdownMenu>
            )}
          </div>
        </header>

        <GraphDocumentFilterBar
          documentId={documentFilterId}
          onDocumentChange={setDocumentFilter}
          disabled={effectiveIsLoading}
        />

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
                {isDocumentScoped ? (
                  <>
                    <h3 className="text-lg font-medium">No entities from this document</h3>
                    <p className="text-sm text-muted-foreground mt-2 mb-6">
                      This document has no extracted entities yet. Processing may still be in progress, or extraction produced no graph data.
                    </p>
                    <Button variant="outline" onClick={() => setDocumentFilter(null)}>
                      Show full graph
                    </Button>
                  </>
                ) : (
                  <>
                    <h3 className="text-lg font-medium">No knowledge graph yet</h3>
                    <p className="text-sm text-muted-foreground mt-2 mb-6">
                      Your knowledge graph is empty. Upload documents to automatically extract entities and relationships.
                    </p>
                    <Button
                      onClick={() => router.push('/documents')}
                    >
                      <Upload className="h-4 w-4 mr-2" />
                      Upload Documents
                    </Button>
                  </>
                )}
              </div>
            </div>
          ) : viewMode === 'table' ? (
            <GraphAsTable />
          ) : (
            <>
              <GraphRenderer
                nodes={allNodes}
                edges={allEdges}
                filters={engineFilters}
                onNodeClick={selectNode}
                onNodeDoubleClick={handleNodeDoubleClick}
                onStageClick={() => selectNode(null)}
                onNodeHover={hoverNode}
                onNodeRightClick={handleNodeRightClick}
                contextTargetId={contextMenuNode?.id ?? null}
              />
              <NoMatchingNodesHint
                active={filteredNodes.length === 0}
                selectedNodeId={selectedNodeId}
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
            onAskAboutThis={handleAskAboutThis}
            onViewDocuments={handleViewDocuments}
            onCopyId={handleCopyId}
            onDelete={handleDeleteNode}
            isExpanded={contextMenuNode ? expandedNodes.has(contextMenuNode.id) : false}
          />

          {/* Graph Controls Overlay — only when canvas has data (avoid empty-state clutter) */}
          {allNodes.length > 0 && viewMode !== 'table' && (
            <>
              <div
                className="absolute top-3 left-3 z-20 flex flex-col gap-2 max-w-[min(100%-1.5rem,16rem)] pointer-events-none [&>*]:pointer-events-auto"
                data-tour="graph-overlay-left"
              >
                <GraphControls />
                {/* Tablet/phone: the depth card only earns its space once a node is selected. */}
                {(!isSmallScreen || selectedNodeId) && (
                  <div className="graph-overlay-surface w-48 px-3 py-2.5">
                    <EgoDepthSlider />
                  </div>
                )}
                <GraphTourTrigger />
                {!isMobile && filteredNodes.length > 0 && (
                  <>
                    {!isSmallScreen && <GraphMinimap width={140} height={100} />}
                    <TimeFilter collapsed />
                    <BookmarksPanel collapsed />
                  </>
                )}
              </div>

              <div
                className="absolute top-3 right-3 z-20 flex flex-col gap-2 pointer-events-none [&>*]:pointer-events-auto"
                data-tour="graph-overlay-right"
              >
                <ZoomControls />
              </div>
          
              <div
                className="absolute bottom-3 right-3 z-20 flex items-end gap-2 pointer-events-none [&>*]:pointer-events-auto"
                data-tour="graph-overlay-bottom"
              >
                {isMobile ? (
                  <>
                    {mobileLegendVisible && <GraphLegend />}
                    <Button
                      variant="secondary"
                      size="sm"
                      className="h-9 min-w-11 text-xs shadow-md graph-overlay-surface"
                      onClick={() => setMobileLegendVisible(!mobileLegendVisible)}
                    >
                      {mobileLegendVisible ? 'Hide Legend' : 'Legend'}
                    </Button>
                  </>
                ) : (
                  <GraphLegend />
                )}
              </div>
            </>
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
                className="text-xs text-muted-foreground font-medium"
                style={{ writingMode: 'vertical-rl', textOrientation: 'mixed' }}
              >
                Details & filters
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
                <h3 className="text-xs font-medium text-muted-foreground">Details</h3>
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

              {/* Node details (scrollable when tall) + filters that fill remaining height */}
              <div className="flex flex-col flex-1 min-h-0 overflow-hidden">
                <div className="shrink-0 max-h-[40%] overflow-y-auto border-b border-border/40">
                  <div className="px-4 py-4 space-y-3">
                    {selectedNode && showNodeDetails && (
                      <NodeDetails node={selectedNode} />
                    )}

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

                    {!selectedNode && (
                      <div className="py-4 text-center px-2">
                        <div className="w-9 h-9 mx-auto mb-2 rounded-full bg-muted/50 flex items-center justify-center">
                          <Network className="h-4 w-4 text-muted-foreground/40" />
                        </div>
                        <p className="text-xs font-medium text-muted-foreground mb-1">
                          Select a node
                        </p>
                        <p className="text-xs text-muted-foreground leading-relaxed">
                          Click any node to explore its connections and sources
                        </p>
                      </div>
                    )}
                  </div>
                </div>

                <div className="flex-1 min-h-0 px-4 py-3">
                  <GraphFilters fillHeight />
                </div>
              </div>
            </div>
          </ResizablePanel>
        )
      )}
      
      {/* Mobile Entity Browser Drawer */}
      <Sheet open={mobileEntityDrawerOpen} onOpenChange={setMobileEntityDrawerOpen}>
        <SheetContent side={isMobile ? "bottom" : "left"} size="sm" className={isMobile ? "h-[75vh] p-0 rounded-t-xl" : "w-[300px] p-0"}>
          <SheetHeader className="border-b">
            <SheetTitle className="text-sm flex items-center gap-2">
              <Network className="h-4 w-4" />
              Entity Browser
            </SheetTitle>
          </SheetHeader>
          <ScrollArea className={isMobile ? "h-[calc(75vh-60px)]" : "h-[calc(100vh-60px)]"}>
            <div className="px-5 py-4 sm:px-6">
              <EntityBrowserPanel className="w-full border-none" />
            </div>
          </ScrollArea>
        </SheetContent>
      </Sheet>
      
      {/* Mobile Details/Filters Drawer */}
      <Sheet open={mobileDetailsDrawerOpen} onOpenChange={setMobileDetailsDrawerOpen}>
        <SheetContent side={isMobile ? "bottom" : "right"} size="sm" className={isMobile ? "h-[75vh] p-0 rounded-t-xl" : "w-[300px] p-0"}>
          <SheetHeader className="border-b">
            <SheetTitle className="text-sm flex items-center gap-2">
              <Filter className="h-4 w-4" />
              Details & Filters
            </SheetTitle>
          </SheetHeader>
          <ScrollArea className={isMobile ? "h-[calc(75vh-60px)]" : "h-[calc(100vh-60px)]"}>
            <div className="px-5 py-4 space-y-4 sm:px-6">
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

              {/* Filters Section — fill remaining drawer height */}
              <div className="pt-3 border-t min-h-[50vh]">
                <GraphFilters fillHeight />
              </div>
            </div>
          </ScrollArea>
        </SheetContent>
      </Sheet>
    </div>
  );
}

export default GraphViewer;
