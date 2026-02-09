'use client';

import { Button } from '@/components/ui/button';
import { Tooltip, TooltipContent, TooltipTrigger } from '@/components/ui/tooltip';
import { useGraphStore } from '@/stores/use-graph-store';
import { useSettingsStore } from '@/stores/use-settings-store';
import forceLayout from 'graphology-layout-force';
import forceAtlas2 from 'graphology-layout-forceatlas2';
import noverlap from 'graphology-layout-noverlap';
import circlepack from 'graphology-layout/circlepack';
import circular from 'graphology-layout/circular';
import random from 'graphology-layout/random';
import { RotateCw } from 'lucide-react';
import { useCallback, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { animateNodes } from 'sigma/utils';

interface LayoutControllerProps {
  className?: string;
}

/**
 * Layout Controller Component
 *
 * Provides instant layout application button for the graph.
 * Applies the selected layout algorithm with smooth animation.
 */
export function LayoutController({ className }: LayoutControllerProps) {
  const { t } = useTranslation();
  const sigmaInstance = useGraphStore((s) => s.sigmaInstance);
  const { graphSettings } = useSettingsStore();

  const [isApplying, setIsApplying] = useState(false);

  const layout = graphSettings.layout ?? 'force';

  /**
   * Apply layout instantly (one-shot, with smooth animation)
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
          // Hierarchical layout - tree-like structure
          // Use circular as fallback for now (proper hierarchical needs custom impl)
          circular.assign(tempGraph);
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
      {/* Apply layout instantly button */}
      <Tooltip>
        <TooltipTrigger asChild>
          <Button
            variant="ghost"
            size="icon"
            onClick={applyLayout}
            className="h-8 w-8"
            disabled={isApplying}
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
