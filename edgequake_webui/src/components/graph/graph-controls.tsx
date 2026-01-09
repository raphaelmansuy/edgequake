/**
 * @module GraphControls
 * @description Graph layout and display controls.
 * Provides layout algorithm selection, clustering toggles, and rendering options.
 *
 * @implements FEAT0603 - Layout algorithm selection
 * @implements FEAT0604 - Clustering toggle
 * @implements FEAT0751 - Graph rendering options
 *
 * @enforces BR0601 - Layout changes animate smoothly
 * @enforces BR0751 - Settings persist across sessions
 */
'use client';

import { Button } from '@/components/ui/button';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Label } from '@/components/ui/label';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { Separator } from '@/components/ui/separator';
import { Switch } from '@/components/ui/switch';
import {
    Tooltip,
    TooltipContent,
    TooltipProvider,
    TooltipTrigger,
} from '@/components/ui/tooltip';
import { useGraphStore } from '@/stores/use-graph-store';
import { useSettingsStore } from '@/stores/use-settings-store';
import { Circle, GitBranch, Info, Settings2, Type, X } from 'lucide-react';
import { useState } from 'react';

export function GraphControls() {
  const { graphSettings, setGraphSettings } = useSettingsStore();
  const { setColorMode } = useGraphStore();
  const [isExpanded, setIsExpanded] = useState(false);

  const handleColorByChange = (value: 'type' | 'community' | 'degree') => {
    setGraphSettings({ colorBy: value });
    // Also update graph store color mode
    if (value === 'community') {
      setColorMode('community');
    } else {
      setColorMode('entity-type');
    }
  };

  if (!isExpanded) {
    return (
      <TooltipProvider>
        <Tooltip>
          <TooltipTrigger asChild>
            <Button
              variant="outline"
              size="icon"
              className="bg-background/95 backdrop-blur-sm shadow-lg border-border/50 hover:bg-accent hover:border-primary/30 hover:shadow-xl transition-all duration-200"
              onClick={() => setIsExpanded(true)}
            >
              <Settings2 className="h-4 w-4" />
            </Button>
          </TooltipTrigger>
          <TooltipContent side="right">
            <p>Graph Settings</p>
          </TooltipContent>
        </Tooltip>
      </TooltipProvider>
    );
  }

  return (
    <Card className="w-56 shadow-lg bg-background/95 backdrop-blur-sm">
      <CardHeader className="py-2 px-3">
        <div className="flex items-center justify-between">
          <CardTitle className="text-xs font-medium flex items-center gap-1.5">
            <Settings2 className="h-3.5 w-3.5" />
            Graph Settings
          </CardTitle>
          <Button
            variant="ghost"
            size="icon"
            className="h-5 w-5"
            onClick={() => setIsExpanded(false)}
          >
            <X className="h-3 w-3" />
          </Button>
        </div>
      </CardHeader>
      <CardContent className="p-3 pt-0 space-y-4">
        {/* Layout Section */}
        <div className="space-y-2">
          <div className="flex items-center gap-1.5">
            <GitBranch className="h-3.5 w-3.5 text-muted-foreground" />
            <Label className="text-xs font-medium">Layout</Label>
          </div>
          <Select
            value={graphSettings.layout}
            onValueChange={(value: 'force' | 'circular' | 'random' | 'circlepack' | 'noverlaps' | 'force-directed' | 'hierarchical') =>
              setGraphSettings({ layout: value })
            }
          >
            <SelectTrigger className="h-8 text-xs">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="force">ForceAtlas2 (FA2)</SelectItem>
              <SelectItem value="force-directed">Force-Directed</SelectItem>
              <SelectItem value="circular">Circular</SelectItem>
              <SelectItem value="circlepack">Circle Pack</SelectItem>
              <SelectItem value="random">Random</SelectItem>
              <SelectItem value="noverlaps">Noverlaps (Anti-collision)</SelectItem>
              <SelectItem value="hierarchical">Hierarchical</SelectItem>
            </SelectContent>
          </Select>
        </div>

        <Separator />

        {/* Appearance Section */}
        <div className="space-y-3">
          <div className="flex items-center gap-1.5">
            <Circle className="h-3.5 w-3.5 text-muted-foreground" />
            <Label className="text-xs font-medium">Appearance</Label>
          </div>
          
          {/* Node Size */}
          <div className="space-y-1.5">
            <div className="flex items-center justify-between">
              <span className="text-[11px] text-muted-foreground">Node Size</span>
              <span className="text-[10px] font-medium bg-muted px-1.5 py-0.5 rounded">
                {graphSettings.nodeSize}
              </span>
            </div>
            <Select
              value={graphSettings.nodeSize}
              onValueChange={(value: 'small' | 'medium' | 'large') =>
                setGraphSettings({ nodeSize: value })
              }
            >
              <SelectTrigger className="h-7 text-xs">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="small">Small</SelectItem>
                <SelectItem value="medium">Medium</SelectItem>
                <SelectItem value="large">Large</SelectItem>
              </SelectContent>
            </Select>
          </div>

          {/* Color By */}
          <div className="space-y-1.5">
            <div className="flex items-center justify-between">
              <span className="text-[11px] text-muted-foreground">Color By</span>
              <TooltipProvider>
                <Tooltip>
                  <TooltipTrigger asChild>
                    <Info className="h-3 w-3 text-muted-foreground cursor-help" />
                  </TooltipTrigger>
                  <TooltipContent side="right" className="max-w-[200px]">
                    <p className="text-xs">
                      <strong>Entity Type:</strong> Color by category<br />
                      <strong>Community:</strong> Cluster detection<br />
                      <strong>Connections:</strong> By link count
                    </p>
                  </TooltipContent>
                </Tooltip>
              </TooltipProvider>
            </div>
            <Select
              value={graphSettings.colorBy}
              onValueChange={handleColorByChange}
            >
              <SelectTrigger className="h-7 text-xs">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="type">Entity Type</SelectItem>
                <SelectItem value="community">Community</SelectItem>
                <SelectItem value="degree">Connections</SelectItem>
              </SelectContent>
            </Select>
          </div>
        </div>

        <Separator />

        {/* Display Options */}
        <div className="space-y-3">
          <div className="flex items-center gap-1.5">
            <Type className="h-3.5 w-3.5 text-muted-foreground" />
            <Label className="text-xs font-medium">Display</Label>
          </div>

          <div className="space-y-2">
            <div className="flex items-center justify-between">
              <span className="text-[11px] text-muted-foreground">Show Labels</span>
              <Switch
                checked={graphSettings.showLabels ?? true}
                onCheckedChange={(checked) => setGraphSettings({ showLabels: checked })}
                className="scale-75"
              />
            </div>

            <div className="flex items-center justify-between">
              <span className="text-[11px] text-muted-foreground">Show Edge Labels</span>
              <Switch
                checked={graphSettings.showEdgeLabels ?? false}
                onCheckedChange={(checked) => setGraphSettings({ showEdgeLabels: checked })}
                className="scale-75"
              />
            </div>

            <div className="flex items-center justify-between">
              <span className="text-[11px] text-muted-foreground">Enable Node Drag</span>
              <Switch
                checked={graphSettings.enableNodeDrag ?? true}
                onCheckedChange={(checked) => setGraphSettings({ enableNodeDrag: checked })}
                className="scale-75"
              />
            </div>

            <div className="flex items-center justify-between">
              <span className="text-[11px] text-muted-foreground">Highlight Neighbors</span>
              <Switch
                checked={graphSettings.highlightNeighbors ?? true}
                onCheckedChange={(checked) => setGraphSettings({ highlightNeighbors: checked })}
                className="scale-75"
              />
            </div>

            <div className="flex items-center justify-between">
              <span className="text-[11px] text-muted-foreground">Hide Unselected Edges</span>
              <Switch
                checked={graphSettings.hideUnselectedEdges ?? false}
                onCheckedChange={(checked) => setGraphSettings({ hideUnselectedEdges: checked })}
                className="scale-75"
              />
            </div>
          </div>
        </div>
      </CardContent>
    </Card>
  );
}

export default GraphControls;
