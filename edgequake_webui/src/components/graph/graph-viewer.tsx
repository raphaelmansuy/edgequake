'use client';

import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert';
import { Button } from '@/components/ui/button';
import { getGraph } from '@/lib/api/edgequake';
import { focusCameraOnNode } from '@/lib/graph/camera-utils';
import { useGraphStore } from '@/stores/use-graph-store';
import { useTenantStore } from '@/stores/use-tenant-store';
import type { GraphNode } from '@/types';
import { useQuery } from '@tanstack/react-query';
import { AlertCircle, Loader2, Maximize2, Network, RefreshCw, Upload, ZoomIn, ZoomOut } from 'lucide-react';
import { useCallback, useEffect, useMemo } from 'react';
import { toast } from 'sonner';
import { EntityBrowserPanel } from './entity-browser-panel';
import { GraphControls } from './graph-controls';
import { GraphExport } from './graph-export';
import { GraphFilters } from './graph-filters';
import { GraphLegend } from './graph-legend';
import { GraphRenderer } from './graph-renderer';
import { GraphSearch } from './graph-search';
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
    sigmaInstance,
    setGraph,
    selectNode,
    toggleNodeDetails,
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

  const { data, isLoading, isError, error, refetch } = useQuery({
    queryKey: ['graph', selectedTenantId, selectedWorkspaceId],
    queryFn: () => getGraph({ limit: 500 }),
    staleTime: 5 * 60 * 1000, // 5 minutes
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
        {/* Toolbar - improved spacing */}
        <header className="flex items-center justify-between border-b px-6 py-3 shrink-0">
          <div className="flex items-center gap-3">
            <h2 className="text-xl font-semibold tracking-tight">Knowledge Graph</h2>
            {isLoading && <Loader2 className="h-4 w-4 animate-spin text-muted-foreground" />}
            {data?.metadata && (
              <span className="text-sm text-muted-foreground">
                {data.metadata.node_count.toLocaleString()} nodes · {data.metadata.edge_count.toLocaleString()} edges
              </span>
            )}
          </div>
          <div className="flex items-center gap-2">
            <GraphSearch />
            <LayoutControl />
            <GraphExport />
            <Button variant="ghost" size="icon" onClick={() => refetch()} title="Refresh">
              <RefreshCw className="h-4 w-4" />
            </Button>
            <Button variant="ghost" size="icon" onClick={handleZoomIn} title="Zoom In">
              <ZoomIn className="h-4 w-4" />
            </Button>
            <Button variant="ghost" size="icon" onClick={handleZoomOut} title="Zoom Out">
              <ZoomOut className="h-4 w-4" />
            </Button>
            <Button variant="ghost" size="icon" onClick={handleResetZoom} title="Reset View">
              <Maximize2 className="h-4 w-4" />
            </Button>
          </div>
        </header>

        {/* Graph Canvas */}
        <div className="flex-1 relative overflow-hidden" data-graph-container>
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
                <div className="w-16 h-16 rounded-full bg-muted flex items-center justify-center mx-auto mb-4">
                  <Network className="h-8 w-8 text-muted-foreground" />
                </div>
                <h3 className="text-lg font-medium">No knowledge graph yet</h3>
                <p className="text-sm text-muted-foreground mt-2">
                  Your knowledge graph is empty. Upload documents to automatically extract entities and relationships.
                </p>
                <Button
                  className="mt-4"
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
                <div className="w-16 h-16 rounded-full bg-muted flex items-center justify-center mx-auto mb-4">
                  <Network className="h-8 w-8 text-muted-foreground" />
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
          
          {/* Legend Overlay - Bottom Right */}
          <div className="absolute bottom-4 right-4">
            <GraphLegend />
          </div>
        </div>
      </div>

      {/* Right Sidebar - improved padding and scroll */}
      <aside className="w-80 border-l bg-card flex flex-col overflow-hidden shrink-0">
        <div className="flex-1 overflow-y-auto p-6 space-y-6">
          {/* Filters */}
          <GraphFilters />

          {/* Node Details */}
          {selectedNode && showNodeDetails && <NodeDetails node={selectedNode} />}
          
          {/* Show details button when panel is hidden but node is selected */}
          {selectedNode && !showNodeDetails && (
            <Button
              variant="outline"
              size="sm"
              className="w-full"
              onClick={toggleNodeDetails}
            >
              Show Node Details
            </Button>
          )}
        </div>
      </aside>
    </div>
  );
}

export default GraphViewer;
