'use client';

import { Button } from '@/components/ui/button';
import {
    DropdownMenu,
    DropdownMenuContent,
    DropdownMenuItem,
    DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import { useGraphStore } from '@/stores/use-graph-store';
import forceAtlas2 from 'graphology-layout-forceatlas2';
import circular from 'graphology-layout/circular';
import random from 'graphology-layout/random';
import { LayoutGrid, Loader2 } from 'lucide-react';
import { useCallback, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { toast } from 'sonner';

type LayoutType = 'force' | 'circular' | 'random';

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

      try {
        // Apply layout based on type
        switch (layout) {
          case 'force':
            forceAtlas2.assign(graph, {
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
            circular.assign(graph);
            break;

          case 'random':
            random.assign(graph);
            // Apply a few iterations of force-directed to space out
            forceAtlas2.assign(graph, {
              iterations: 50,
              settings: {
                gravity: 2,
                scalingRatio: 1,
              },
            });
            break;
        }

        // Refresh the sigma display
        sigmaInstance.refresh();
        
        // Reset camera to show all nodes
        sigmaInstance.getCamera().animatedReset({ duration: 500 });

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
      <DropdownMenuTrigger asChild>
        <Button 
          variant="ghost" 
          size="icon" 
          title={t('graph.layouts.title')}
          disabled={isApplying}
        >
          {isApplying ? (
            <Loader2 className="h-4 w-4 animate-spin" />
          ) : (
            <LayoutGrid className="h-4 w-4" />
          )}
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent>
        <DropdownMenuItem 
          onClick={() => applyLayout('force')}
          className={currentLayout === 'force' ? 'bg-accent' : ''}
        >
          ⚡ {t('graph.layouts.force')}
        </DropdownMenuItem>
        <DropdownMenuItem 
          onClick={() => applyLayout('circular')}
          className={currentLayout === 'circular' ? 'bg-accent' : ''}
        >
          ⭕ {t('graph.layouts.circular')}
        </DropdownMenuItem>
        <DropdownMenuItem 
          onClick={() => applyLayout('random')}
          className={currentLayout === 'random' ? 'bg-accent' : ''}
        >
          🎲 {t('graph.layouts.random')}
        </DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}

export default LayoutControl;
