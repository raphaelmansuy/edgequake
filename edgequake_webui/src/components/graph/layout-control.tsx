'use client';

import { Button } from '@/components/ui/button';
import {
    DropdownMenu,
    DropdownMenuContent,
    DropdownMenuItem,
    DropdownMenuSeparator,
    DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import {
    Tooltip,
    TooltipContent,
    TooltipTrigger,
} from '@/components/ui/tooltip';
import { useGraphStore } from '@/stores/use-graph-store';
import forceAtlas2 from 'graphology-layout-forceatlas2';
import noverlap from 'graphology-layout-noverlap';
import circlepack from 'graphology-layout/circlepack';
import circular from 'graphology-layout/circular';
import random from 'graphology-layout/random';
import { LayoutGrid, Loader2 } from 'lucide-react';
import { useCallback, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { animateNodes } from 'sigma/utils';
import { toast } from 'sonner';

type LayoutType = 'force' | 'circular' | 'random' | 'noverlaps' | 'circlepack' | 'force-directed' | 'hierarchical';

export function LayoutControl() {
  const { t } = useTranslation();
  const { sigmaInstance } = useGraphStore();
  const [isApplying, setIsApplying] = useState(false);
  const [currentLayout, setCurrentLayout] = useState<LayoutType>('force');

  const applyLayout = useCallback(
    async (layout: LayoutType) => {
      if (!sigmaInstance) {
        toast.error('Graph not ready');
        return;
      }

      setIsApplying(true);
      setCurrentLayout(layout);

      const graph = sigmaInstance.getGraph();
      
      // Store current positions for animation
      const startPositions: Record<string, { x: number; y: number }> = {};
      graph.forEachNode((node) => {
        startPositions[node] = {
          x: graph.getNodeAttribute(node, 'x'),
          y: graph.getNodeAttribute(node, 'y'),
        };
      });

      try {
        // Create a copy of the graph to calculate new positions
        const tempGraph = graph.copy();
        
        // Apply layout based on type
        switch (layout) {
          case 'force':
            forceAtlas2.assign(tempGraph, {
              iterations: 100,
              settings: {
                gravity: 1,
                scalingRatio: 2,
                strongGravityMode: true,
                barnesHutOptimize: graph.order > 100,
              },
            });
            break;

          case 'circular':
            circular.assign(tempGraph);
            break;

          case 'random':
            random.assign(tempGraph);
            // Apply a few iterations of force-directed to space out
            forceAtlas2.assign(tempGraph, {
              iterations: 50,
              settings: {
                gravity: 2,
                scalingRatio: 1,
              },
            });
            break;
            
          case 'noverlaps':
            // First apply force layout, then remove overlaps
            forceAtlas2.assign(tempGraph, {
              iterations: 50,
              settings: {
                gravity: 1,
                scalingRatio: 2,
              },
            });
            noverlap.assign(tempGraph, {
              maxIterations: 200,
              settings: {
                margin: 5,
                expansion: 1.1,
                ratio: 1.0,
              },
            });
            break;
            
          case 'circlepack':
            circlepack.assign(tempGraph, {
              hierarchyAttributes: ['node_type', 'entityType'],
              scale: 100,
            });
            break;
            
          case 'force-directed':
            // Force-directed layout with different parameters than ForceAtlas2
            // More spread out, less clustering
            forceAtlas2.assign(tempGraph, {
              iterations: 150,
              settings: {
                gravity: 0.5,
                scalingRatio: 5,
                strongGravityMode: false,
                barnesHutOptimize: graph.order > 100,
                linLogMode: true,
                outboundAttractionDistribution: true,
              },
            });
            break;
            
          case 'hierarchical':
            // Hierarchical layout: organize by node types in levels
            // First, group by entity type
            const nodesByType: Record<string, string[]> = {};
            tempGraph.forEachNode((node) => {
              const nodeType = tempGraph.getNodeAttribute(node, 'node_type') || 'unknown';
              if (!nodesByType[nodeType]) {
                nodesByType[nodeType] = [];
              }
              nodesByType[nodeType].push(node);
            });
            
            const typeOrder = Object.keys(nodesByType).sort();
            const levelHeight = 200;
            const nodeSpacing = 100;
            
            typeOrder.forEach((type, levelIndex) => {
              const nodesInType = nodesByType[type];
              const levelWidth = nodesInType.length * nodeSpacing;
              nodesInType.forEach((node, nodeIndex) => {
                const x = (nodeIndex - nodesInType.length / 2) * nodeSpacing;
                const y = levelIndex * levelHeight;
                tempGraph.setNodeAttribute(node, 'x', x);
                tempGraph.setNodeAttribute(node, 'y', y);
              });
            });
            break;
        }

        // Extract new positions
        const newPositions: Record<string, { x: number; y: number }> = {};
        tempGraph.forEachNode((node) => {
          newPositions[node] = {
            x: tempGraph.getNodeAttribute(node, 'x'),
            y: tempGraph.getNodeAttribute(node, 'y'),
          };
        });

        // Animate to new positions
        animateNodes(graph, newPositions, {
          duration: 500,
          easing: 'quadraticInOut',
        });
        
        // Reset camera to show all nodes after animation
        setTimeout(() => {
          sigmaInstance.getCamera().animatedReset({ duration: 300 });
        }, 500);

        toast.success(`Applied ${layout} layout`);
      } catch (error) {
        console.error('Layout failed:', error);
        toast.error('Failed to apply layout');
      } finally {
        setIsApplying(false);
      }
    },
    [sigmaInstance]
  );

  return (
    <DropdownMenu>
      <Tooltip>
        <TooltipTrigger asChild>
          <DropdownMenuTrigger asChild>
            <Button 
              variant="ghost" 
              size="icon" 
              aria-label={t('graph.layouts.title', 'Change layout')}
              disabled={isApplying}
            >
              {isApplying ? (
                <Loader2 className="h-4 w-4 animate-spin" />
              ) : (
                <LayoutGrid className="h-4 w-4" />
              )}
            </Button>
          </DropdownMenuTrigger>
        </TooltipTrigger>
        <TooltipContent side="bottom">
          <div className="space-y-1">
            <div className="font-medium text-xs">{t('graph.layouts.title', 'Graph Layout')}</div>
            <p className="text-[10px] opacity-80">Rearrange nodes with different algorithms</p>
          </div>
        </TooltipContent>
      </Tooltip>
      <DropdownMenuContent>
        <DropdownMenuItem 
          onClick={() => applyLayout('force')}
          className={currentLayout === 'force' ? 'bg-accent' : ''}
        >
          ⚡ {t('graph.layouts.force', 'Force Atlas')}
        </DropdownMenuItem>
        <DropdownMenuItem 
          onClick={() => applyLayout('force-directed')}
          className={currentLayout === 'force-directed' ? 'bg-accent' : ''}
        >
          🔄 {t('graph.layouts.forceDirected', 'Force Directed')}
        </DropdownMenuItem>
        <DropdownMenuItem 
          onClick={() => applyLayout('circular')}
          className={currentLayout === 'circular' ? 'bg-accent' : ''}
        >
          ⭕ {t('graph.layouts.circular', 'Circular')}
        </DropdownMenuItem>
        <DropdownMenuItem 
          onClick={() => applyLayout('random')}
          className={currentLayout === 'random' ? 'bg-accent' : ''}
        >
          🎲 {t('graph.layouts.random', 'Random')}
        </DropdownMenuItem>
        <DropdownMenuSeparator />
        <DropdownMenuItem 
          onClick={() => applyLayout('noverlaps')}
          className={currentLayout === 'noverlaps' ? 'bg-accent' : ''}
        >
          📐 {t('graph.layouts.noverlap', 'No Overlap')}
        </DropdownMenuItem>
        <DropdownMenuItem 
          onClick={() => applyLayout('circlepack')}
          className={currentLayout === 'circlepack' ? 'bg-accent' : ''}
        >
          🎯 {t('graph.layouts.circlepack', 'Circle Pack')}
        </DropdownMenuItem>
        <DropdownMenuItem 
          onClick={() => applyLayout('hierarchical')}
          className={currentLayout === 'hierarchical' ? 'bg-accent' : ''}
        >
          🌳 {t('graph.layouts.hierarchical', 'Hierarchical')}
        </DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}

export default LayoutControl;
