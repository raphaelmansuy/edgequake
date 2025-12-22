'use client';

import { useEffect } from 'react';
import { useQuery } from '@tanstack/react-query';
import { Loader2, RefreshCw, ZoomIn, ZoomOut, Maximize2, AlertCircle, Network, Upload } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert';
import { GraphRenderer } from './graph-renderer';
import { GraphControls } from './graph-controls';
import { NodeDetails } from './node-details';
import { GraphFilters } from './graph-filters';
import { useGraphStore } from '@/stores/use-graph-store';
import { getGraph } from '@/lib/api/edgequake';

export function GraphViewer() {
  const {
    nodes,
    edges,
    selectedNodeId,
    sigmaInstance,
    setGraph,
    selectNode,
    hoverNode,
    setLoading,
    setError,
  } = useGraphStore();

  const { data, isLoading, isError, error, refetch } = useQuery({
    queryKey: ['graph'],
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

  const selectedNode = nodes.find((n) => n.id === selectedNodeId);

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
    <div className="flex h-full">
      {/* Main Graph Area */}
      <div className="flex-1 flex flex-col">
        {/* Toolbar */}
        <div className="flex items-center justify-between border-b px-4 py-2">
          <div className="flex items-center gap-2">
            <h2 className="text-lg font-semibold">Knowledge Graph</h2>
            {isLoading && <Loader2 className="h-4 w-4 animate-spin text-muted-foreground" />}
            {data && (
              <span className="text-sm text-muted-foreground">
                {data.metadata.node_count} nodes · {data.metadata.edge_count} edges
              </span>
            )}
          </div>
          <div className="flex items-center gap-1">
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
        </div>

        {/* Graph Canvas */}
        <div className="flex-1 relative">
          {isLoading && nodes.length === 0 ? (
            <div className="absolute inset-0 flex items-center justify-center">
              <div className="text-center">
                <Loader2 className="h-8 w-8 animate-spin text-muted-foreground mx-auto mb-2" />
                <p className="text-sm text-muted-foreground">Loading knowledge graph...</p>
              </div>
            </div>
          ) : nodes.length === 0 ? (
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
          ) : (
            <GraphRenderer
              nodes={nodes}
              edges={edges}
              onNodeClick={selectNode}
              onNodeHover={hoverNode}
            />
          )}

          {/* Graph Controls Overlay */}
          <div className="absolute bottom-4 left-4">
            <GraphControls />
          </div>
        </div>
      </div>

      {/* Right Sidebar */}
      <div className="w-80 border-l bg-card overflow-auto">
        <div className="p-4 space-y-4">
          {/* Filters */}
          <GraphFilters />

          {/* Node Details */}
          {selectedNode && <NodeDetails node={selectedNode} />}
        </div>
      </div>
    </div>
  );
}

export default GraphViewer;
