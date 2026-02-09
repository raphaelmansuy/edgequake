/**
 * @module GraphFilters
 * @description Entity type filter panel for knowledge graph.
 * Allows toggling visibility of entity types with counts.
 * 
 * @implements UC0104 - User filters entities by type
 * @implements FEAT0202 - Entity type filtering
 * @implements FEAT0627 - Type-based visibility toggles
 * 
 * @enforces BR0618 - Filter state syncs with graph view
 * @enforces BR0619 - Type counts update on filter change
 * 
 * @see {@link docs/use_cases.md} UC0104
 */
'use client';

import { Badge } from '@/components/ui/badge';
import { Checkbox } from '@/components/ui/checkbox';
import { Input } from '@/components/ui/input';
import { ScrollArea } from '@/components/ui/scroll-area';
import { useGraphStore } from '@/stores/use-graph-store';
import { Filter, Search } from 'lucide-react';

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
    <div className="space-y-3.5">
      {/* Header */}
      <div className="flex items-center gap-1.5">
        <Filter className="h-3.5 w-3.5 text-muted-foreground" />
        <h4 className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">
          Filters
        </h4>
      </div>

      {/* Search */}
      <div className="relative">
        <Search className="absolute left-2.5 top-1/2 -translate-y-1/2 h-3 w-3 text-muted-foreground pointer-events-none" />
        <Input
          placeholder="Search entities..."
          value={searchQuery}
          onChange={(e) => setSearchQuery(e.target.value)}
          className="h-8 pl-8 pr-3 text-xs bg-muted/30 border-muted focus:bg-background transition-colors"
        />
      </div>

      {/* Entity Types */}
      <div>
        <h5 className="text-[10px] font-semibold text-muted-foreground mb-2 uppercase tracking-wider">
          Entity Types
        </h5>
        <ScrollArea className="h-40 border border-border/40 rounded-md" showShadows>
          <div className="space-y-0.5 p-1.5">
            {entityTypes.map((type) => {
              const color = TYPE_COLORS[type.toUpperCase()] || TYPE_COLORS.DEFAULT;
              const count = typeCounts[type] || 0;
              const isVisible = visibleEntityTypes.has(type);

              return (
                <div
                  key={type}
                  className="flex items-center gap-2 py-1.5 px-2 rounded-md hover:bg-muted/50 transition-colors"
                >
                  <Checkbox
                    id={`type-${type}`}
                    checked={isVisible}
                    onCheckedChange={() => toggleEntityType(type)}
                    className="h-3.5 w-3.5 shrink-0"
                  />
                  <label
                    htmlFor={`type-${type}`}
                    className="flex-1 flex items-center gap-2 text-xs cursor-pointer min-w-0"
                  >
                    <div
                      className="w-2.5 h-2.5 rounded-full shrink-0 ring-1 ring-black/10"
                      style={{ backgroundColor: color }}
                    />
                    <span className="flex-1 truncate font-medium">{type}</span>
                    <Badge variant="secondary" className="text-[9px] h-4 px-1.5 shrink-0 tabular-nums">
                      {count}
                    </Badge>
                  </label>
                </div>
              );
            })}
          </div>
        </ScrollArea>
      </div>
    </div>
  );
}

export default GraphFilters;
