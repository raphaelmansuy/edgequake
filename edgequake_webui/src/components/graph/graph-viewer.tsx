'use client';

import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert';
import { Button } from '@/components/ui/button';
import { ResizablePanel } from '@/components/ui/resizable-panel';
import { ScrollArea } from '@/components/ui/scroll-area';
import { useGraphKeyboardNavigation } from '@/hooks/use-graph-keyboard-navigation';
import { getGraph } from '@/lib/api/edgequake';
import { focusCameraOnNode } from '@/lib/graph/camera-utils';
import { useGraphStore } from '@/stores/use-graph-store';
import { useTenantStore } from '@/stores/use-tenant-store';
import type { GraphNode } from '@/types';
import { useQuery } from '@tanstack/react-query';
import { AlertCircle, ChevronLeft, ChevronRight, Loader2, Maximize2, Network, PanelRightClose, RefreshCw, Upload, ZoomIn, ZoomOut } from 'lucide-react';
import { useCallback, useEffect, useMemo } from 'react';
import { toast } from 'sonner';
import { GraphEmptyIllustration } from '../illustrations/graph-empty-illustration';
import { EntityBrowserPanel } from './entity-browser-panel';
import { GraphControls } from './graph-controls';
import { GraphExport } from './graph-export';
import { GraphFilters } from './graph-filters';
import { GraphLegend } from './graph-legend';
import { GraphRenderer } from './graph-renderer';
import { GraphSearch } from './graph-search';
import { GraphTourTrigger } from './graph-tour-wrapper';
import { KeyboardShortcutsHelp } from './keyboard-shortcuts-help';
import { LayoutControl } from './layout-control';
import { NodeContextMenu, useNodeContextMenu } from './node-context-menu';
import { NodeDetails } from './node-details';
import { ZoomControls } from './zoom-controls';

export function GraphViewer() {
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

  const { data, isLoading, isError, error, refetch } = useQuery({
    queryKey: ['graph', selectedTenantId, selectedWorkspaceId],
    queryFn: () => getGraph({ limit: 500 }),
    staleTime: 2 * 60 * 1000, // 2 minutes
    refetchOnMount: 'always', // Always refetch when component mounts (navigation)
    refetchOnWindowFocus: true, // Refetch when window regains focus
  });

  useEffect(() => {
    if (data) {
      setGraph(data);
    }
  }, [data, setGraph]);

  useEffect(() => {
    setLoading(isLoading);
  }, [isLoading, setLoading]);

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
    // Focus camera on this node and highlight its neighborhood
    if (sigmaInstance) {
      focusCameraOnNode(sigmaInstance, node.id, {
        ratio: 0.3,
        duration: 500,
        highlight: false, // selectNode handles highlighting
      });
    }
    selectNode(node.id);
    toast.success(`Expanded neighborhood for ${node.label}`);
  }, [sigmaInstance, selectNode]);

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

  const selectedNode = allNodes.find((n) => n.id === selectedNodeId);

  if (isError) {
    return (
      <div className="p-6">
        <Alert variant="destructive">
          <AlertCircle className="h-4 w-4" />
          <AlertTitle>Error loading graph</AlertTitle>
          <AlertDescription>
            {error instanceof Error ? error.message : 'Failed to load knowledge graph'}
            <Button variant="link" className="ml-2 p-0" onClick={() => refetch()}>
              Try again
            </Button>
          </AlertDescription>
        </Alert>
      </div>
    );
  }

  return (
    <div className="flex h-full overflow-hidden">
      {/* Left Entity Browser */}
      <EntityBrowserPanel />

      {/* Main Graph Area */}
      <div className="flex-1 flex flex-col min-w-0 overflow-hidden">
        {/* Toolbar - compact and slick */}
        <header 
          className="flex items-center justify-between border-b px-4 py-2 shrink-0 bg-card/50 backdrop-blur-sm"
          data-tour="graph-header"
        >
          <div className="flex items-center gap-2.5">
            <h2 className="text-base font-semibold tracking-tight">Knowledge Graph</h2>
            {isLoading && <Loader2 className="h-3.5 w-3.5 animate-spin text-muted-foreground" />}
            {data?.metadata && (
              <span className="text-xs text-muted-foreground bg-muted px-2 py-0.5 rounded-md">
                {data.metadata.node_count.toLocaleString()} nodes · {data.metadata.edge_count.toLocaleString()} edges
              </span>
            )}
          </div>
          <div className="flex items-center gap-1">
            <div data-tour="graph-search"><GraphSearch /></div>
            <div data-tour="layout-control"><LayoutControl /></div>
            <GraphExport />
            <div data-tour="keyboard-help"><KeyboardShortcutsHelp /></div>
            <GraphTourTrigger />
            <div className="w-px h-5 bg-border mx-1" />
            <Button variant="ghost" size="icon" className="h-7 w-7" onClick={() => refetch()} title="Refresh">
              <RefreshCw className="h-3.5 w-3.5" />
            </Button>
            <Button variant="ghost" size="icon" className="h-7 w-7" onClick={handleZoomIn} title="Zoom In">
              <ZoomIn className="h-3.5 w-3.5" />
            </Button>
            <Button variant="ghost" size="icon" className="h-7 w-7" onClick={handleZoomOut} title="Zoom Out">
              <ZoomOut className="h-3.5 w-3.5" />
            </Button>
            <Button variant="ghost" size="icon" className="h-7 w-7" onClick={handleResetZoom} title="Reset View">
              <Maximize2 className="h-3.5 w-3.5" />
            </Button>
          </div>
        </header>

        {/* Graph Canvas - bg-background ensures proper theme in fullscreen */}
        <div 
          className="flex-1 relative overflow-hidden bg-background text-foreground" 
          data-graph-container
          data-tour="graph-canvas"
        >
          {isLoading && allNodes.length === 0 ? (
            <div className="absolute inset-0 flex items-center justify-center">
              <div className="text-center">
                <Loader2 className="h-8 w-8 animate-spin text-muted-foreground mx-auto mb-2" />
                <p className="text-sm text-muted-foreground">Loading knowledge graph...</p>
              </div>
            </div>
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
              
              {/* Loading Overlay */}
              {isLoading && allNodes.length > 0 && (
                <div className="absolute inset-0 flex items-center justify-center bg-background/60 backdrop-blur-sm z-10">
                  <div className="text-center">
                    <Loader2 className="h-8 w-8 animate-spin text-primary mx-auto mb-2" />
                    <p className="text-sm font-medium">Refreshing graph...</p>
                  </div>
                </div>
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
            onFindRelated={handleFindRelated}
            onViewDocuments={handleViewDocuments}
            onCopyId={handleCopyId}
          />

          {/* Graph Controls Overlay - Bottom Left */}
          <div className="absolute bottom-4 left-4 flex flex-col gap-2">
            <GraphControls />
          </div>
          
          {/* Zoom Controls Overlay - Right Side */}
          <div className="absolute top-4 right-4 flex flex-col gap-2">
            <ZoomControls />
          </div>
          
          {/* Legend Overlay - Bottom Right (hidden on mobile to prevent overlay) */}
          <div className="absolute bottom-4 right-4 hidden md:block">
            <GraphLegend />
          </div>
        </div>
      </div>

      {/* Right Sidebar - Resizable */}
      {rightPanelCollapsed ? (
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
          defaultWidth={320}
          minWidth={280}
          maxWidth={480}
          className="border-l bg-card/95 backdrop-blur-sm"
          storageKey="edgequake.graph.rightPanelWidth"
          ariaLabel="Resize details panel"
        >
          <div className="flex flex-col h-full overflow-hidden" data-tour="details-panel">
            {/* Panel Header */}
            <div className="flex items-center justify-between px-3 py-2 border-b shrink-0 bg-muted/30">
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
            <ScrollArea className="flex-1 min-h-0" showShadows>
              <div className="p-3 space-y-4">
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
                  <div className="py-6 text-center">
                    <div className="w-10 h-10 mx-auto mb-2 rounded-full bg-muted/50 flex items-center justify-center">
                      <Network className="h-5 w-5 text-muted-foreground/50" />
                    </div>
                    <p className="text-xs text-muted-foreground">
                      Click on a node to view details
                    </p>
                  </div>
                )}
                
                {/* Filters Section */}
                <div className="pt-2 border-t">
                  <GraphFilters />
                </div>
              </div>
            </ScrollArea>
          </div>
        </ResizablePanel>
      )}
    </div>
  );
}

export default GraphViewer;
