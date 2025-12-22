'use client';

import { Badge } from '@/components/ui/badge';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Checkbox } from '@/components/ui/checkbox';
import { Input } from '@/components/ui/input';
import { ScrollArea } from '@/components/ui/scroll-area';
import { useGraphStore } from '@/stores/use-graph-store';
import { Search } from 'lucide-react';

// Color palette for entity types
const TYPE_COLORS: Record<string, string> = {
  PERSON: '#3b82f6',
  ORGANIZATION: '#10b981',
  LOCATION: '#f59e0b',
  EVENT: '#ef4444',
  CONCEPT: '#8b5cf6',
  DOCUMENT: '#6366f1',
  DEFAULT: '#64748b',
};

export function GraphFilters() {
  const {
    graph,
    visibleEntityTypes,
    searchQuery,
    toggleEntityType,
    setSearchQuery,
  } = useGraphStore();

  if (!graph?.metadata) return null;

  const entityTypes = graph.metadata.entity_types || [];
  const typeCounts = graph.nodes.reduce((acc, node) => {
    acc[node.node_type] = (acc[node.node_type] || 0) + 1;
    return acc;
  }, {} as Record<string, number>);

  return (
    <Card>
      <CardHeader className="pb-3">
        <CardTitle className="text-sm font-medium">Filters</CardTitle>
      </CardHeader>
      <CardContent className="space-y-4">
        {/* Search */}
        <div className="relative">
          <Search className="absolute left-2.5 top-2.5 h-4 w-4 text-muted-foreground" />
          <Input
            placeholder="Search entities..."
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            className="pl-8"
          />
        </div>

        {/* Entity Types */}
        <div>
          <h4 className="text-xs font-medium text-muted-foreground mb-2">Entity Types</h4>
          <ScrollArea className="h-48">
            <div className="space-y-2">
              {entityTypes.map((type) => {
                const color = TYPE_COLORS[type.toUpperCase()] || TYPE_COLORS.DEFAULT;
                const count = typeCounts[type] || 0;
                const isVisible = visibleEntityTypes.has(type);

                return (
                  <div key={type} className="flex items-center gap-2">
                    <Checkbox
                      id={`type-${type}`}
                      checked={isVisible}
                      onCheckedChange={() => toggleEntityType(type)}
                    />
                    <label
                      htmlFor={`type-${type}`}
                      className="flex-1 flex items-center gap-2 text-sm cursor-pointer"
                    >
                      <div
                        className="w-3 h-3 rounded-full"
                        style={{ backgroundColor: color }}
                      />
                      <span className="flex-1">{type}</span>
                      <Badge variant="secondary" className="text-xs">
                        {count}
                      </Badge>
                    </label>
                  </div>
                );
              })}
            </div>
          </ScrollArea>
        </div>
      </CardContent>
    </Card>
  );
}

export default GraphFilters;
