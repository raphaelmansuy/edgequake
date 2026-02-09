'use client';

import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Popover, PopoverContent, PopoverTrigger } from '@/components/ui/popover';
import { Slider } from '@/components/ui/slider';
import { Switch } from '@/components/ui/switch';
import { useDebounce } from '@/hooks/use-debounce';
import { searchLabels } from '@/lib/api/edgequake';
import { calculateOptimalMaxNodes, detectDeviceTier, formatNodeCount, type DeviceTier, type OptimizedSettings } from '@/lib/graph/auto-optimize';
import { useGraphStore, MAX_DISPLAY_NODES } from '@/stores/use-graph-store';
import { useQuery } from '@tanstack/react-query';
import { Search, Settings2, Sparkles, X, Zap } from 'lucide-react';
import { useCallback, useEffect, useMemo, useState } from 'react';

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
    startNode,
    totalNodesInStorage,
    setMaxNodes,
    setDepth,
    setStartNode,
  } = useGraphStore();

  // Local state for sliders (only update store on release)
  const [localMaxNodes, setLocalMaxNodes] = useState(maxNodes);
  const [localDepth, setLocalDepth] = useState(depth);
  const [includeOrphans, setIncludeOrphans] = useState(false);
  const [autoOptimize, setAutoOptimize] = useState(false);
  
  // WHY: Detect device tier once on mount for consistent recommendations
  const [deviceTier, setDeviceTier] = useState<DeviceTier>('medium');
  
  useEffect(() => {
    setDeviceTier(detectDeviceTier());
  }, []);
  
  // WHY: Calculate optimized settings based on workspace size and device
  const optimizedSettings = useMemo<OptimizedSettings | null>(() => {
    if (totalNodesInStorage === 0) return null;
    return calculateOptimalMaxNodes(totalNodesInStorage, deviceTier);
  }, [totalNodesInStorage, deviceTier]);
  
  // Focus entity search state
  const [focusQuery, setFocusQuery] = useState(startNode || '');
  const debouncedFocusQuery = useDebounce(focusQuery, 300);

  // Search for matching labels
  const { data: searchResults } = useQuery({
    queryKey: ['labels-search', debouncedFocusQuery],
    queryFn: () => searchLabels(debouncedFocusQuery, 5),
    enabled: debouncedFocusQuery.length >= 2,
    staleTime: 30000,
  });

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
        // WHY: Cap at MAX_DISPLAY_NODES to enforce performance limit
        if (!isNaN(parsed) && parsed >= 100 && parsed <= MAX_DISPLAY_NODES) {
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

  // WHY: Auto-optimize applies the calculated settings based on workspace size
  const handleAutoOptimize = useCallback(() => {
    if (!optimizedSettings) return;
    
    setLocalMaxNodes(optimizedSettings.maxNodes);
    setLocalDepth(optimizedSettings.depth);
    setMaxNodes(optimizedSettings.maxNodes);
    setDepth(optimizedSettings.depth);
    setAutoOptimize(true);
    
    try {
      localStorage.setItem('graph-max-nodes', String(optimizedSettings.maxNodes));
      localStorage.setItem('graph-depth', String(optimizedSettings.depth));
      localStorage.setItem('graph-auto-optimize', 'true');
    } catch (e) {
      console.warn('Failed to save optimized settings:', e);
    }
    
    onSettingsChange?.();
  }, [optimizedSettings, setMaxNodes, setDepth, onSettingsChange]);

  // Handle focus entity selection
  const handleFocusSelect = useCallback((label: string) => {
    setFocusQuery(label);
    setStartNode(label);
    onSettingsChange?.();
  }, [setStartNode, onSettingsChange]);

  // Handle clearing focus
  const handleClearFocus = useCallback(() => {
    setFocusQuery('');
    setStartNode(null);
    onSettingsChange?.();
  }, [setStartNode, onSettingsChange]);

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

          {/* Focus Entity Input */}
          <div className="space-y-2">
            <Label className="text-xs text-muted-foreground">Focus Entity</Label>
            <div className="relative">
              <Search className="absolute left-2 top-1/2 h-3.5 w-3.5 -translate-y-1/2 text-muted-foreground" />
              <Input
                value={focusQuery}
                onChange={(e) => setFocusQuery(e.target.value)}
                placeholder="Search entity to focus..."
                className="h-8 pl-7 pr-7 text-xs"
              />
              {focusQuery && (
                <Button
                  variant="ghost"
                  size="icon"
                  className="absolute right-0 top-0 h-8 w-8"
                  onClick={handleClearFocus}
                >
                  <X className="h-3 w-3" />
                </Button>
              )}
            </div>
            {/* Search suggestions */}
            {searchResults?.labels && searchResults.labels.length > 0 && debouncedFocusQuery.length >= 2 && (
              <div className="border rounded-md divide-y max-h-32 overflow-y-auto">
                {searchResults.labels.map((label: string) => (
                  <button
                    key={label}
                    className="w-full px-2 py-1.5 text-left text-xs hover:bg-muted"
                    onClick={() => handleFocusSelect(label)}
                  >
                    <span className="truncate">{label}</span>
                  </button>
                ))}
              </div>
            )}
            <p className="text-[10px] text-muted-foreground">
              {startNode ? `Focused on: ${startNode}` : 'Leave empty to show most connected nodes.'}
            </p>
          </div>

          {/* Auto-Optimize Button */}
          {totalNodesInStorage > 0 && optimizedSettings && (
            <div className="space-y-2 pb-2 border-b">
              <div className="flex items-center justify-between">
                <div className="flex items-center gap-1.5">
                  <Zap className="h-3.5 w-3.5 text-amber-500" />
                  <Label className="text-xs font-medium">Auto-Optimize</Label>
                </div>
                <span className="text-[10px] text-muted-foreground capitalize">
                  {deviceTier} perf device
                </span>
              </div>
              <Button
                variant="outline"
                size="sm"
                className="w-full h-8 text-xs gap-1.5"
                onClick={handleAutoOptimize}
              >
                <Sparkles className="h-3.5 w-3.5" />
                Apply Optimal Settings
              </Button>
              <p className="text-[10px] text-muted-foreground">
                Workspace: {formatNodeCount(totalNodesInStorage)} nodes → Recommended: {formatNodeCount(optimizedSettings.maxNodes)} max
              </p>
            </div>
          )}

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
              max={MAX_DISPLAY_NODES}
              step={50}
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
                  setLocalMaxNodes(200);
                  setLocalDepth(2);
                  setMaxNodes(200);
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
                  setLocalMaxNodes(400);
                  setLocalDepth(3);
                  setMaxNodes(400);
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
                  setLocalMaxNodes(MAX_DISPLAY_NODES);
                  setLocalDepth(4);
                  setMaxNodes(MAX_DISPLAY_NODES);
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
