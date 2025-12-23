'use client';

import { Button } from '@/components/ui/button';
import {
    Command,
    CommandEmpty,
    CommandGroup,
    CommandInput,
    CommandItem,
    CommandList,
} from '@/components/ui/command';
import {
    Popover,
    PopoverContent,
    PopoverTrigger,
} from '@/components/ui/popover';
import { useGraphStore } from '@/stores/use-graph-store';
import type { GraphNode } from '@/types';
import { Circle, Search } from 'lucide-react';
import MiniSearch from 'minisearch';
import { useCallback, useMemo, useState } from 'react';
import { useTranslation } from 'react-i18next';

// Color palette for entity types (matching graph-renderer.tsx)
const TYPE_COLORS: Record<string, string> = {
  PERSON: '#3b82f6',
  ORGANIZATION: '#10b981',
  LOCATION: '#f59e0b',
  EVENT: '#ef4444',
  CONCEPT: '#8b5cf6',
  DOCUMENT: '#6366f1',
  DEFAULT: '#64748b',
};

function getNodeColor(entityType: string | undefined): string {
  if (!entityType) return TYPE_COLORS.DEFAULT;
  return TYPE_COLORS[entityType.toUpperCase()] || TYPE_COLORS.DEFAULT;
}

interface SearchResult {
  id: string;
  label: string;
  entityType?: string;
  score: number;
}

interface GraphSearchProps {
  onSelect?: (nodeId: string) => void;
}

export function GraphSearch({ onSelect }: GraphSearchProps) {
  const { t } = useTranslation();
  const { nodes, sigmaInstance, selectNode } = useGraphStore();
  const [open, setOpen] = useState(false);
  const [query, setQuery] = useState('');

  // Create search index when nodes change
  const searchEngine = useMemo(() => {
    if (nodes.length === 0) return null;

    const miniSearch = new MiniSearch<GraphNode>({
      idField: 'id',
      fields: ['label', 'description', 'node_type'],
      storeFields: ['label', 'node_type'],
      searchOptions: {
        prefix: true,
        fuzzy: 0.2,
        boost: { label: 2, node_type: 1 },
      },
    });

    miniSearch.addAll(nodes);
    return miniSearch;
  }, [nodes]);

  // Compute search results based on query
  const results = useMemo<SearchResult[]>(() => {
    if (!searchEngine || !query.trim()) {
      return [];
    }

    const searchResults = searchEngine.search(query).slice(0, 10);
    return searchResults.map((r) => ({
      id: r.id,
      label: r.label || r.id,
      entityType: r.node_type,
      score: r.score,
    }));
  }, [query, searchEngine]);

  // Handle node selection
  const handleSelect = useCallback(
    (nodeId: string) => {
      setOpen(false);
      setQuery('');

      // Select node in store
      selectNode(nodeId);

      // Focus camera on node
      if (sigmaInstance) {
        const graph = sigmaInstance.getGraph();
        const nodeData = graph.getNodeAttributes(nodeId);
        if (nodeData) {
          sigmaInstance
            .getCamera()
            .animate(
              { x: nodeData.x, y: nodeData.y, ratio: 0.5 },
              { duration: 500 }
            );
        }
      }

      onSelect?.(nodeId);
    },
    [sigmaInstance, selectNode, onSelect]
  );

  return (
    <Popover open={open} onOpenChange={setOpen}>
      <PopoverTrigger asChild>
        <Button variant="outline" size="sm" className="gap-2">
          <Search className="h-4 w-4" />
          <span className="hidden sm:inline">{t('graph.search.placeholder')}</span>
        </Button>
      </PopoverTrigger>
      <PopoverContent className="w-80 p-0" align="start">
        <Command shouldFilter={false}>
          <CommandInput
            placeholder={t('graph.search.placeholder')}
            value={query}
            onValueChange={setQuery}
          />
          <CommandList>
            <CommandEmpty>{t('graph.search.noResults')}</CommandEmpty>
            {results.length > 0 && (
              <CommandGroup>
                {results.map((result) => (
                  <CommandItem
                    key={result.id}
                    value={result.id}
                    onSelect={() => handleSelect(result.id)}
                    className="flex items-center gap-2"
                  >
                    <Circle
                      className="h-3 w-3 flex-shrink-0"
                      style={{ 
                        color: getNodeColor(result.entityType),
                        fill: getNodeColor(result.entityType)
                      }}
                    />
                    <span className="flex-1 truncate">{result.label}</span>
                    {result.entityType && (
                      <span className="text-xs text-muted-foreground">
                        {result.entityType}
                      </span>
                    )}
                  </CommandItem>
                ))}
              </CommandGroup>
            )}
          </CommandList>
        </Command>
      </PopoverContent>
    </Popover>
  );
}

export default GraphSearch;
