'use client';

import { Button } from '@/components/ui/button';
import { Label } from '@/components/ui/label';
import { Popover, PopoverContent, PopoverTrigger } from '@/components/ui/popover';
import { Slider } from '@/components/ui/slider';
import { Switch } from '@/components/ui/switch';
import { useGraphStore } from '@/stores/use-graph-store';
import { Settings2, X } from 'lucide-react';
import { useCallback, useEffect, useState } from 'react';

interface GraphSettingsPanelProps {
  /** Callback when settings change that require a graph refetch */
  onSettingsChange?: () => void;
}

/**
 * GraphSettingsPanel - Control panel for virtual query settings.
 * 
 * Allows users to configure:
 * - Max nodes to fetch (100-10000)
 * - Traversal depth (1-5)
 * - Include orphan nodes toggle
 * 
 * Settings persist to localStorage.
 */
export function GraphSettingsPanel({ onSettingsChange }: GraphSettingsPanelProps) {
  const [open, setOpen] = useState(false);
  const { 
    maxNodes, 
    depth,
    setMaxNodes,
    setDepth,
  } = useGraphStore();

  // Local state for sliders (only update store on release)
  const [localMaxNodes, setLocalMaxNodes] = useState(maxNodes);
  const [localDepth, setLocalDepth] = useState(depth);
  const [includeOrphans, setIncludeOrphans] = useState(false);

  // Sync local state with store
  useEffect(() => {
    setLocalMaxNodes(maxNodes);
  }, [maxNodes]);

  useEffect(() => {
    setLocalDepth(depth);
  }, [depth]);

  // Load settings from localStorage on mount
  useEffect(() => {
    try {
      const storedMaxNodes = localStorage.getItem('graph-max-nodes');
      const storedDepth = localStorage.getItem('graph-depth');
      const storedOrphans = localStorage.getItem('graph-include-orphans');
      
      if (storedMaxNodes) {
        const parsed = parseInt(storedMaxNodes, 10);
        if (!isNaN(parsed) && parsed >= 100 && parsed <= 10000) {
          setMaxNodes(parsed);
        }
      }
      if (storedDepth) {
        const parsed = parseInt(storedDepth, 10);
        if (!isNaN(parsed) && parsed >= 1 && parsed <= 5) {
          setDepth(parsed);
        }
      }
      if (storedOrphans) {
        setIncludeOrphans(storedOrphans === 'true');
      }
    } catch (e) {
      console.warn('Failed to load graph settings from localStorage:', e);
    }
  }, [setMaxNodes, setDepth]);

  const handleMaxNodesCommit = useCallback((value: number[]) => {
    setMaxNodes(value[0]);
    onSettingsChange?.();
  }, [setMaxNodes, onSettingsChange]);

  const handleDepthCommit = useCallback((value: number[]) => {
    setDepth(value[0]);
    onSettingsChange?.();
  }, [setDepth, onSettingsChange]);

  const handleOrphansChange = useCallback((checked: boolean) => {
    setIncludeOrphans(checked);
    try {
      localStorage.setItem('graph-include-orphans', String(checked));
    } catch (e) {
      console.warn('Failed to save orphans setting:', e);
    }
    onSettingsChange?.();
  }, [onSettingsChange]);

  // Format large numbers with commas
  const formatNumber = (num: number) => num.toLocaleString();

  return (
    <Popover open={open} onOpenChange={setOpen}>
      <PopoverTrigger asChild>
        <Button
          variant="outline"
          size="icon"
          className="h-8 w-8"
          title="Graph Settings"
        >
          <Settings2 className="h-4 w-4" />
        </Button>
      </PopoverTrigger>
      <PopoverContent 
        className="w-72" 
        align="end" 
        side="bottom"
        sideOffset={8}
      >
        <div className="space-y-4">
          {/* Header */}
          <div className="flex items-center justify-between">
            <h4 className="font-medium text-sm">Query Settings</h4>
            <Button
              variant="ghost"
              size="icon"
              className="h-6 w-6"
              onClick={() => setOpen(false)}
            >
              <X className="h-3 w-3" />
            </Button>
          </div>

          {/* Max Nodes Slider */}
          <div className="space-y-2">
            <div className="flex items-center justify-between">
              <Label className="text-xs text-muted-foreground">Max Nodes</Label>
              <span className="text-xs font-medium tabular-nums">
                {formatNumber(localMaxNodes)}
              </span>
            </div>
            <Slider
              value={[localMaxNodes]}
              onValueChange={([v]) => setLocalMaxNodes(v)}
              onValueCommit={handleMaxNodesCommit}
              min={100}
              max={10000}
              step={100}
              className="w-full"
            />
            <p className="text-[10px] text-muted-foreground">
              Limit the number of nodes fetched from the server.
            </p>
          </div>

          {/* Depth Slider */}
          <div className="space-y-2">
            <div className="flex items-center justify-between">
              <Label className="text-xs text-muted-foreground">Traversal Depth</Label>
              <span className="text-xs font-medium tabular-nums">{localDepth}</span>
            </div>
            <Slider
              value={[localDepth]}
              onValueChange={([v]) => setLocalDepth(v)}
              onValueCommit={handleDepthCommit}
              min={1}
              max={5}
              step={1}
              className="w-full"
            />
            <p className="text-[10px] text-muted-foreground">
              Depth of relationship traversal from the focus node.
            </p>
          </div>

          {/* Include Orphans Toggle */}
          <div className="flex items-center justify-between">
            <div className="space-y-0.5">
              <Label className="text-xs">Include Orphans</Label>
              <p className="text-[10px] text-muted-foreground">
                Show nodes with no connections.
              </p>
            </div>
            <Switch
              checked={includeOrphans}
              onCheckedChange={handleOrphansChange}
              className="scale-90"
            />
          </div>

          {/* Presets */}
          <div className="pt-2 border-t">
            <Label className="text-xs text-muted-foreground">Quick Presets</Label>
            <div className="flex gap-2 mt-2">
              <Button
                variant="outline"
                size="sm"
                className="flex-1 h-7 text-xs"
                onClick={() => {
                  setLocalMaxNodes(500);
                  setLocalDepth(2);
                  setMaxNodes(500);
                  setDepth(2);
                  onSettingsChange?.();
                }}
              >
                Default
              </Button>
              <Button
                variant="outline"
                size="sm"
                className="flex-1 h-7 text-xs"
                onClick={() => {
                  setLocalMaxNodes(2000);
                  setLocalDepth(3);
                  setMaxNodes(2000);
                  setDepth(3);
                  onSettingsChange?.();
                }}
              >
                Large
              </Button>
              <Button
                variant="outline"
                size="sm"
                className="flex-1 h-7 text-xs"
                onClick={() => {
                  setLocalMaxNodes(10000);
                  setLocalDepth(4);
                  setMaxNodes(10000);
                  setDepth(4);
                  onSettingsChange?.();
                }}
              >
                Max
              </Button>
            </div>
          </div>
        </div>
      </PopoverContent>
    </Popover>
  );
}
