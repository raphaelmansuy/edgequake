'use client';

import { Button } from '@/components/ui/button';
import { Tooltip, TooltipContent, TooltipTrigger } from '@/components/ui/tooltip';
import { useGraphStore } from '@/stores/use-graph-store';
import { useSettingsStore } from '@/stores/use-settings-store';
import forceAtlas2 from 'graphology-layout-forceatlas2';
import FA2Layout from 'graphology-layout-forceatlas2/worker';
import circular from 'graphology-layout/circular';
import random from 'graphology-layout/random';
import { Pause, Play, RotateCw } from 'lucide-react';
import { useCallback, useEffect, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { animateNodes } from 'sigma/utils';

interface LayoutControllerProps {
  className?: string;
}

/**
 * Layout Controller Component
 * 
 * Provides controls for running ForceAtlas2 layout algorithm in a Web Worker.
 * This prevents UI blocking for large graphs (500+ nodes).
 * 
 * Features:
 * - Play/Pause button for continuous ForceAtlas2 animation
 * - Instant layout button for one-shot layout calculation
 * - Web Worker for non-blocking computation
 * - Automatic layout settings inference based on graph size
 */
export function LayoutController({ className }: LayoutControllerProps) {
  const { t } = useTranslation();
  const sigmaInstance = useGraphStore((s) => s.sigmaInstance);
  const { graphSettings } = useSettingsStore();
  
  const [isRunning, setIsRunning] = useState(false);
  const [isApplying, setIsApplying] = useState(false);
  const fa2LayoutRef = useRef<FA2Layout | null>(null);
  const animationFrameRef = useRef<number | null>(null);

  const layout = graphSettings.layout ?? 'force';

  // Cleanup on unmount
  useEffect(() => {
    return () => {
      if (fa2LayoutRef.current) {
        fa2LayoutRef.current.kill();
        fa2LayoutRef.current = null;
      }
      if (animationFrameRef.current) {
        cancelAnimationFrame(animationFrameRef.current);
        animationFrameRef.current = null;
      }
    };
  }, []);

  // Stop layout when layout setting changes
  useEffect(() => {
    if (fa2LayoutRef.current && isRunning) {
      fa2LayoutRef.current.stop();
      fa2LayoutRef.current.kill();
      fa2LayoutRef.current = null;
      setIsRunning(false);
    }
  }, [layout, isRunning]);

  /**
   * Start/Stop the ForceAtlas2 Web Worker layout
   */
  const toggleLayout = useCallback(() => {
    if (!sigmaInstance) return;

    const graph = sigmaInstance.getGraph();
    if (!graph || graph.order === 0) return;

    if (isRunning && fa2LayoutRef.current) {
      // Stop the layout
      fa2LayoutRef.current.stop();
      fa2LayoutRef.current.kill();
      fa2LayoutRef.current = null;
      setIsRunning(false);
      return;
    }

    // Start the layout
    try {
      // Infer optimal settings based on graph size
      const sensibleSettings = forceAtlas2.inferSettings(graph);
      
      // Create FA2 Web Worker layout
      fa2LayoutRef.current = new FA2Layout(graph, {
        settings: {
          ...sensibleSettings,
          gravity: 1,
          scalingRatio: 2,
          strongGravityMode: true,
          barnesHutOptimize: graph.order > 100,
        },
      });

      // Start the layout computation
      fa2LayoutRef.current.start();
      setIsRunning(true);

      // Auto-stop after 5 seconds to prevent infinite running
      setTimeout(() => {
        if (fa2LayoutRef.current?.isRunning()) {
          fa2LayoutRef.current.stop();
          fa2LayoutRef.current.kill();
          fa2LayoutRef.current = null;
          setIsRunning(false);
        }
      }, 5000);
    } catch (error) {
      console.error('Error starting FA2 layout:', error);
      setIsRunning(false);
    }
  }, [sigmaInstance, isRunning]);

  /**
   * Apply layout instantly (one-shot, no animation)
   */
  const applyLayout = useCallback(() => {
    if (!sigmaInstance) return;

    const graph = sigmaInstance.getGraph();
    if (!graph || graph.order === 0) return;

    setIsApplying(true);

    try {
      // Calculate new positions based on current layout setting
      const tempGraph = graph.copy();
      
      switch (layout) {
        case 'circular':
          circular.assign(tempGraph);
          break;
        case 'random':
          random.assign(tempGraph);
          break;
        case 'force':
        default:
          // Use synchronous FA2 for instant layout
          const sensibleSettings = forceAtlas2.inferSettings(tempGraph);
          forceAtlas2.assign(tempGraph, {
            iterations: 100,
            settings: {
              ...sensibleSettings,
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

      // Animate to new positions
      animateNodes(graph, newPositions, { 
        duration: 300, 
        easing: 'quadraticInOut' 
      });
    } catch (error) {
      console.error('Error applying layout:', error);
    } finally {
      setTimeout(() => setIsApplying(false), 300);
    }
  }, [sigmaInstance, layout]);

  // Don't render if no sigma instance
  if (!sigmaInstance) {
    return null;
  }

  return (
    <div className={`flex items-center gap-1 ${className ?? ''}`}>
      {/* Play/Pause ForceAtlas2 button - only for force layout */}
      {layout === 'force' && (
        <Tooltip>
          <TooltipTrigger asChild>
            <Button
              variant="ghost"
              size="icon"
              onClick={toggleLayout}
              className="h-8 w-8"
              aria-label={isRunning ? t('graph.layout.stop', 'Stop Animation') : t('graph.layout.start', 'Start Animation')}
            >
              {isRunning ? (
                <Pause className="h-4 w-4" />
              ) : (
                <Play className="h-4 w-4" />
              )}
            </Button>
          </TooltipTrigger>
          <TooltipContent>
            {isRunning 
              ? t('graph.layout.stopTooltip', 'Stop ForceAtlas2 animation') 
              : t('graph.layout.startTooltip', 'Start ForceAtlas2 animation (Web Worker)')}
          </TooltipContent>
        </Tooltip>
      )}

      {/* Apply layout instantly button */}
      <Tooltip>
        <TooltipTrigger asChild>
          <Button
            variant="ghost"
            size="icon"
            onClick={applyLayout}
            className="h-8 w-8"
            disabled={isApplying || isRunning}
            aria-label={t('graph.layout.apply', 'Apply Layout')}
          >
            <RotateCw className={`h-4 w-4 ${isApplying ? 'animate-spin' : ''}`} />
          </Button>
        </TooltipTrigger>
        <TooltipContent>
          {t('graph.layout.applyTooltip', 'Apply layout instantly')}
        </TooltipContent>
      </Tooltip>
    </div>
  );
}

export default LayoutController;
