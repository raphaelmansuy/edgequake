'use client';

import { Button } from '@/components/ui/button';
import { Separator } from '@/components/ui/separator';
import {
    Tooltip,
    TooltipContent,
    TooltipProvider,
    TooltipTrigger,
} from '@/components/ui/tooltip';
import { useGraphStore } from '@/stores/use-graph-store';
import {
    Focus,
    Maximize2,
    Minimize2,
    RotateCcw,
    RotateCw,
    ZoomIn,
    ZoomOut,
} from 'lucide-react';
import { useCallback, useState } from 'react';
import { useTranslation } from 'react-i18next';

/**
 * ZoomControls - SOTA zoom and camera controls for graph visualization
 * 
 * Features:
 * - Zoom in/out with smooth animation
 * - Reset zoom to fit graph
 * - Camera rotation (clockwise/counter-clockwise)
 * - Fullscreen toggle
 * - Focus on selected node
 */
export function ZoomControls() {
  const { t } = useTranslation();
  const sigmaInstance = useGraphStore((s) => s.sigmaInstance);
  const selectedNodeId = useGraphStore((s) => s.selectedNodeId);
  const [isFullscreen, setIsFullscreen] = useState(false);

  const handleZoomIn = useCallback(() => {
    if (sigmaInstance) {
      const camera = sigmaInstance.getCamera();
      camera.animatedZoom({ duration: 200, factor: 1.5 });
    }
  }, [sigmaInstance]);

  const handleZoomOut = useCallback(() => {
    if (sigmaInstance) {
      const camera = sigmaInstance.getCamera();
      camera.animatedUnzoom({ duration: 200, factor: 1.5 });
    }
  }, [sigmaInstance]);

  const handleResetZoom = useCallback(() => {
    if (sigmaInstance) {
      try {
        // Clear custom bounding box and refresh
        sigmaInstance.setCustomBBox(null);
        sigmaInstance.refresh();
        
        const graph = sigmaInstance.getGraph();
        
        // Check if graph has nodes
        if (!graph?.order || graph.nodes().length === 0) {
          sigmaInstance.getCamera().animate(
            { x: 0.5, y: 0.5, ratio: 1 },
            { duration: 300 }
          );
          return;
        }

        // Reset to fit all nodes with some padding
        sigmaInstance.getCamera().animate(
          { x: 0.5, y: 0.5, ratio: 1.1, angle: 0 },
          { duration: 500 }
        );
      } catch (error) {
        console.error('Error resetting zoom:', error);
        sigmaInstance.getCamera().animate(
          { x: 0.5, y: 0.5, ratio: 1, angle: 0 },
          { duration: 300 }
        );
      }
    }
  }, [sigmaInstance]);

  const handleRotateClockwise = useCallback(() => {
    if (sigmaInstance) {
      const camera = sigmaInstance.getCamera();
      const currentAngle = camera.angle;
      camera.animate(
        { angle: currentAngle + Math.PI / 8 },
        { duration: 200 }
      );
    }
  }, [sigmaInstance]);

  const handleRotateCounterClockwise = useCallback(() => {
    if (sigmaInstance) {
      const camera = sigmaInstance.getCamera();
      const currentAngle = camera.angle;
      camera.animate(
        { angle: currentAngle - Math.PI / 8 },
        { duration: 200 }
      );
    }
  }, [sigmaInstance]);

  const handleFocusOnNode = useCallback(() => {
    if (sigmaInstance && selectedNodeId) {
      const graph = sigmaInstance.getGraph();
      
      if (graph.hasNode(selectedNodeId)) {
        const nodePosition = {
          x: graph.getNodeAttribute(selectedNodeId, 'x'),
          y: graph.getNodeAttribute(selectedNodeId, 'y'),
        };

        // Animate to center on node with zoom
        sigmaInstance.getCamera().animate(
          {
            x: nodePosition.x,
            y: nodePosition.y,
            ratio: 0.3, // Zoom in
          },
          { duration: 500 }
        );

        // Highlight the node
        graph.setNodeAttribute(selectedNodeId, 'highlighted', true);
        sigmaInstance.refresh();
      }
    }
  }, [sigmaInstance, selectedNodeId]);

  const handleFullscreen = useCallback(() => {
    const container = document.querySelector('[data-graph-container]');
    
    if (!container) return;

    if (!isFullscreen) {
      if (container.requestFullscreen) {
        container.requestFullscreen();
        setIsFullscreen(true);
      }
    } else {
      if (document.exitFullscreen) {
        document.exitFullscreen();
        setIsFullscreen(false);
      }
    }
  }, [isFullscreen]);

  // Listen for fullscreen changes
  if (typeof window !== 'undefined') {
    document.addEventListener('fullscreenchange', () => {
      setIsFullscreen(!!document.fullscreenElement);
    });
  }

  return (
    <TooltipProvider>
      <div 
        className="flex flex-col gap-1 bg-background/80 backdrop-blur-sm rounded-lg border shadow-lg p-1"
        role="toolbar"
        aria-label={t('graph.controls.title', 'Graph controls')}
      >
        {/* Zoom Controls */}
        <Tooltip>
          <TooltipTrigger asChild>
            <Button
              variant="ghost"
              size="icon"
              className="h-8 w-8"
              onClick={handleZoomIn}
              aria-label={t('graph.zoomIn', 'Zoom In')}
            >
              <ZoomIn className="h-4 w-4" aria-hidden="true" />
            </Button>
          </TooltipTrigger>
          <TooltipContent side="left">
            {t('graph.zoomIn', 'Zoom In')}
          </TooltipContent>
        </Tooltip>

        <Tooltip>
          <TooltipTrigger asChild>
            <Button
              variant="ghost"
              size="icon"
              className="h-8 w-8"
              onClick={handleZoomOut}
              aria-label={t('graph.zoomOut', 'Zoom Out')}
            >
              <ZoomOut className="h-4 w-4" aria-hidden="true" />
            </Button>
          </TooltipTrigger>
          <TooltipContent side="left">
            {t('graph.zoomOut', 'Zoom Out')}
          </TooltipContent>
        </Tooltip>

        <Separator className="my-1" />

        {/* Rotation Controls */}
        <Tooltip>
          <TooltipTrigger asChild>
            <Button
              variant="ghost"
              size="icon"
              className="h-8 w-8"
              onClick={handleRotateClockwise}
              aria-label={t('graph.rotateClockwise', 'Rotate Clockwise')}
            >
              <RotateCw className="h-4 w-4" aria-hidden="true" />
            </Button>
          </TooltipTrigger>
          <TooltipContent side="left">
            {t('graph.rotateClockwise', 'Rotate Clockwise')}
          </TooltipContent>
        </Tooltip>

        <Tooltip>
          <TooltipTrigger asChild>
            <Button
              variant="ghost"
              size="icon"
              className="h-8 w-8"
              onClick={handleRotateCounterClockwise}
              aria-label={t('graph.rotateCounterClockwise', 'Rotate Counter-Clockwise')}
            >
              <RotateCcw className="h-4 w-4" aria-hidden="true" />
            </Button>
          </TooltipTrigger>
          <TooltipContent side="left">
            {t('graph.rotateCounterClockwise', 'Rotate Counter-Clockwise')}
          </TooltipContent>
        </Tooltip>

        <Separator className="my-1" />

        {/* Focus on Node */}
        {selectedNodeId && (
          <Tooltip>
            <TooltipTrigger asChild>
              <Button
                variant="ghost"
                size="icon"
                className="h-8 w-8"
                onClick={handleFocusOnNode}
                aria-label={t('graph.focusOnNode', 'Focus on Selected Node')}
              >
                <Focus className="h-4 w-4" aria-hidden="true" />
              </Button>
            </TooltipTrigger>
            <TooltipContent side="left">
              {t('graph.focusOnNode', 'Focus on Selected Node')}
            </TooltipContent>
          </Tooltip>
        )}

        {/* Reset Zoom */}
        <Tooltip>
          <TooltipTrigger asChild>
            <Button
              variant="ghost"
              size="icon"
              className="h-8 w-8"
              onClick={handleResetZoom}
              aria-label={t('graph.resetZoom', 'Reset View')}
            >
              <Maximize2 className="h-4 w-4" aria-hidden="true" />
            </Button>
          </TooltipTrigger>
          <TooltipContent side="left">
            {t('graph.resetZoom', 'Reset View')}
          </TooltipContent>
        </Tooltip>

        {/* Fullscreen Toggle */}
        <Tooltip>
          <TooltipTrigger asChild>
            <Button
              variant="ghost"
              size="icon"
              className="h-8 w-8"
              onClick={handleFullscreen}
              aria-label={isFullscreen
                ? t('graph.exitFullscreen', 'Exit Fullscreen')
                : t('graph.enterFullscreen', 'Fullscreen')}
            >
              {isFullscreen ? (
                <Minimize2 className="h-4 w-4" aria-hidden="true" />
              ) : (
                <Maximize2 className="h-4 w-4" aria-hidden="true" />
              )}
            </Button>
          </TooltipTrigger>
          <TooltipContent side="left">
            {isFullscreen
              ? t('graph.exitFullscreen', 'Exit Fullscreen')
              : t('graph.enterFullscreen', 'Fullscreen')}
          </TooltipContent>
        </Tooltip>
      </div>
    </TooltipProvider>
  );
}

export default ZoomControls;
