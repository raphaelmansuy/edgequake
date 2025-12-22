'use client';

import { Card, CardContent } from '@/components/ui/card';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { useSettingsStore } from '@/stores/use-settings-store';

export function GraphControls() {
  const { graphSettings, setGraphSettings } = useSettingsStore();

  return (
    <Card className="w-48 shadow-lg">
      <CardContent className="p-3 space-y-3">
        {/* Layout */}
        <div className="space-y-1">
          <label className="text-xs font-medium text-muted-foreground">Layout</label>
          <Select
            value={graphSettings.layout}
            onValueChange={(value: 'force' | 'circular' | 'random') =>
              setGraphSettings({ layout: value })
            }
          >
            <SelectTrigger className="h-8 text-xs">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="force">Force-Directed</SelectItem>
              <SelectItem value="circular">Circular</SelectItem>
              <SelectItem value="random">Random</SelectItem>
            </SelectContent>
          </Select>
        </div>

        {/* Node Size */}
        <div className="space-y-1">
          <label className="text-xs font-medium text-muted-foreground">Node Size</label>
          <Select
            value={graphSettings.nodeSize}
            onValueChange={(value: 'small' | 'medium' | 'large') =>
              setGraphSettings({ nodeSize: value })
            }
          >
            <SelectTrigger className="h-8 text-xs">
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
        <div className="space-y-1">
          <label className="text-xs font-medium text-muted-foreground">Color By</label>
          <Select
            value={graphSettings.colorBy}
            onValueChange={(value: 'type' | 'community' | 'degree') =>
              setGraphSettings({ colorBy: value })
            }
          >
            <SelectTrigger className="h-8 text-xs">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="type">Entity Type</SelectItem>
              <SelectItem value="community">Community</SelectItem>
              <SelectItem value="degree">Connections</SelectItem>
            </SelectContent>
          </Select>
        </div>
      </CardContent>
    </Card>
  );
}

export default GraphControls;
