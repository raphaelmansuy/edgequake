'use client';

import { Button } from '@/components/ui/button';
import { Separator } from '@/components/ui/separator';
import {
    Tooltip,
    TooltipContent,
    TooltipProvider,
    TooltipTrigger,
} from '@/components/ui/tooltip';
import { focusCameraOnNode, resetCameraToFitGraph } from '@/lib/graph/camera-utils';
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
import { useCallback, useEffect, useState } from 'react';
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
      resetCameraToFitGraph(sigmaInstance, 500);
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
    if (!sigmaInstance || !selectedNodeId) return;
    
    focusCameraOnNode(sigmaInstance, selectedNodeId, {
      ratio: 0.4,
      duration: 500,
      highlight: true,
    });
  }, [sigmaInstance, selectedNodeId]);

  const handleFullscreen = useCallback(() => {
    const container = document.querySelector('[data-graph-container]');
    
    if (!container) return;

    if (!isFullscreen) {
      // Copy dark class to container for proper theming in fullscreen
      const isDark = document.documentElement.classList.contains('dark');
      if (isDark) {
        container.classList.add('dark');
      }
      
      if (container.requestFullscreen) {
        container.requestFullscreen();
        setIsFullscreen(true);
      }
    } else {
      // Remove dark class when exiting fullscreen
      container.classList.remove('dark');
      
      if (document.exitFullscreen) {
        document.exitFullscreen();
        setIsFullscreen(false);
      }
    }
  }, [isFullscreen]);

  // Listen for fullscreen changes and sync dark mode
  useEffect(() => {
    const handleFullscreenChange = () => {
      const isNowFullscreen = !!document.fullscreenElement;
      setIsFullscreen(isNowFullscreen);
      
      // Sync dark class with fullscreen element
      const container = document.querySelector('[data-graph-container]');
      if (container) {
        const isDark = document.documentElement.classList.contains('dark');
        if (isNowFullscreen && isDark) {
          container.classList.add('dark');
        } else {
          container.classList.remove('dark');
        }
      }
    };

    document.addEventListener('fullscreenchange', handleFullscreenChange);
    return () => document.removeEventListener('fullscreenchange', handleFullscreenChange);
  }, []);

  return (
    <TooltipProvider>
      <div 
        className="flex flex-col gap-1 bg-background/95 backdrop-blur-sm rounded-lg border border-border/50 shadow-lg p-1 hover:shadow-xl transition-shadow duration-200"
        role="toolbar"
        aria-label={t('graph.controls.title', 'Graph controls')}
        data-tour="zoom-controls"
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
          <TooltipContent side="left" className="flex items-center gap-2">
            <span>{t('graph.zoomIn', 'Zoom In')}</span>
            <kbd className="px-1.5 py-0.5 text-[10px] font-mono bg-background/20 rounded border border-background/10">+</kbd>
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
          <TooltipContent side="left" className="flex items-center gap-2">
            <span>{t('graph.zoomOut', 'Zoom Out')}</span>
            <kbd className="px-1.5 py-0.5 text-[10px] font-mono bg-background/20 rounded border border-background/10">-</kbd>
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
            <TooltipContent side="left" className="flex items-center gap-2">
              <span>{t('graph.focusOnNode', 'Focus on Selected')}</span>
              <kbd className="px-1.5 py-0.5 text-[10px] font-mono bg-background/20 rounded border border-background/10">Enter</kbd>
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
          <TooltipContent side="left" className="flex items-center gap-2">
            <span>{t('graph.resetZoom', 'Reset View')}</span>
            <kbd className="px-1.5 py-0.5 text-[10px] font-mono bg-background/20 rounded border border-background/10">0</kbd>
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
          <TooltipContent side="left" className="flex items-center gap-2">
            <span>{isFullscreen
              ? t('graph.exitFullscreen', 'Exit Fullscreen')
              : t('graph.enterFullscreen', 'Fullscreen')}</span>
            <kbd className="px-1.5 py-0.5 text-[10px] font-mono bg-background/20 rounded border border-background/10">F</kbd>
          </TooltipContent>
        </Tooltip>
      </div>
    </TooltipProvider>
  );
}

export default ZoomControls;
