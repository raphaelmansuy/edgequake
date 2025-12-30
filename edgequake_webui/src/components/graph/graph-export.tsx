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
import { Download, FileCode, FileJson, ImageIcon } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { toast } from 'sonner';

export function GraphExport() {
  const { t } = useTranslation();
  const { nodes, edges } = useGraphStore();

  const exportAsPNG = async () => {
    try {
      // Find the sigma canvas element
      const container = document.querySelector('[data-graph-container] canvas, .sigma-container canvas');
      if (!container) {
        toast.error(t('graph.export.noCanvas', 'No graph canvas found'));
        return;
      }

      const canvas = container as HTMLCanvasElement;
      
      // Create a new canvas with white background for better visibility
      const exportCanvas = document.createElement('canvas');
      exportCanvas.width = canvas.width;
      exportCanvas.height = canvas.height;
      const ctx = exportCanvas.getContext('2d');
      
      if (ctx) {
        // White background
        ctx.fillStyle = '#ffffff';
        ctx.fillRect(0, 0, exportCanvas.width, exportCanvas.height);
        // Draw the graph
        ctx.drawImage(canvas, 0, 0);
      }

      exportCanvas.toBlob((blob) => {
        if (!blob) {
          toast.error(t('graph.export.failed', 'Export failed'));
          return;
        }
        const url = URL.createObjectURL(blob);
        const a = document.createElement('a');
        a.href = url;
        a.download = `edgequake-graph-${Date.now()}.png`;
        a.click();
        URL.revokeObjectURL(url);
        toast.success(t('graph.export.pngSuccess', 'Graph exported as PNG'));
      }, 'image/png');
    } catch (error) {
      console.error('PNG export error:', error);
      toast.error(t('graph.export.failed', 'Export failed'));
    }
  };

  const exportAsSVG = async () => {
    // SVG export is more complex and requires specialized libraries
    // For now, show info message
    toast.info(t('graph.export.svgComingSoon', 'SVG export coming soon'));
  };

  const exportAsJSON = () => {
    try {
      const data = {
        metadata: {
          exportedAt: new Date().toISOString(),
          nodeCount: nodes.length,
          edgeCount: edges.length,
          source: 'EdgeQuake Knowledge Graph',
        },
        nodes: nodes.map((n) => ({
          id: n.id,
          label: n.label,
          type: n.node_type,
          description: n.description,
          properties: n.properties,
        })),
        edges: edges.map((e) => ({
          source: e.source,
          target: e.target,
          type: e.relationship_type,
          description: e.description,
          weight: e.weight,
          properties: e.properties,
        })),
      };

      const blob = new Blob([JSON.stringify(data, null, 2)], {
        type: 'application/json',
      });
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = `edgequake-graph-${Date.now()}.json`;
      a.click();
      URL.revokeObjectURL(url);
      toast.success(t('graph.export.jsonSuccess', 'Graph exported as JSON'));
    } catch (error) {
      console.error('JSON export error:', error);
      toast.error(t('graph.export.failed', 'Export failed'));
    }
  };

  const isGraphEmpty = nodes.length === 0;

  return (
    <DropdownMenu>
      <Tooltip>
        <TooltipTrigger asChild>
          <DropdownMenuTrigger asChild>
            <Button 
              variant="ghost" 
              size="icon" 
              aria-label={t('graph.export.title', 'Export graph')}
              disabled={isGraphEmpty}
            >
              <Download className="h-4 w-4" />
              <span className="sr-only">{t('graph.export.title', 'Export graph')}</span>
            </Button>
          </DropdownMenuTrigger>
        </TooltipTrigger>
        <TooltipContent side="bottom">
          <div className="space-y-1">
            <div className="font-medium text-xs">{t('graph.export.title', 'Export Graph')}</div>
            <p className="text-[10px] opacity-80">Save as PNG, SVG, or JSON</p>
          </div>
        </TooltipContent>
      </Tooltip>
      <DropdownMenuContent align="end">
        <DropdownMenuItem onClick={exportAsPNG}>
          <ImageIcon className="h-4 w-4 mr-2" />
          {t('graph.export.png', 'Export as PNG')}
        </DropdownMenuItem>
        <DropdownMenuItem onClick={exportAsSVG}>
          <FileCode className="h-4 w-4 mr-2" />
          {t('graph.export.svg', 'Export as SVG')}
        </DropdownMenuItem>
        <DropdownMenuSeparator />
        <DropdownMenuItem onClick={exportAsJSON}>
          <FileJson className="h-4 w-4 mr-2" />
          {t('graph.export.json', 'Export as JSON')}
        </DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
