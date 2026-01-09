/**
 * @module GraphSearch
 * @description Full-text search for graph entities using MiniSearch.
 * Provides fuzzy search with type highlighting and camera focus.
 * 
 * @implements UC0108 - User searches entities by name
 * @implements FEAT0202 - Full-text entity search
 * @implements FEAT0626 - Camera focus on selected entity
 * 
 * @enforces BR0616 - Search results sorted by relevance
 * @enforces BR0617 - Entity types color-coded in results
 * 
 * @see {@link docs/features.md} FEAT0202
 */
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
import { focusCameraOnNode } from '@/lib/graph/camera-utils';
import { useGraphStore } from '@/stores/use-graph-store';
import type { GraphNode } from '@/types';
import { Circle, Loader2, Search } from 'lucide-react';
import MiniSearch from 'minisearch';
import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
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

/**
 * Custom debounce hook
 */
function useDebounce<T>(value: T, delay: number): T {
  const [debouncedValue, setDebouncedValue] = useState<T>(value);

  useEffect(() => {
    const handler = setTimeout(() => {
      setDebouncedValue(value);
    }, delay);

    return () => {
      clearTimeout(handler);
    };
  }, [value, delay]);

  return debouncedValue;
}

interface SearchResult {
  id: string;
  label: string;
  entityType?: string;
  description?: string;
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
  const [isSearching, setIsSearching] = useState(false);
  const inputRef = useRef<HTMLInputElement>(null);
  
  // Debounce search query for better performance
  const debouncedQuery = useDebounce(query, 150);

  // Create search index when nodes change
  const searchEngine = useMemo(() => {
    if (nodes.length === 0) return null;

    const miniSearch = new MiniSearch<GraphNode>({
      idField: 'id',
      fields: ['label', 'description', 'node_type'],
      storeFields: ['label', 'node_type', 'description'],
      searchOptions: {
        prefix: true,
        fuzzy: 0.2,
        boost: { label: 3, description: 1, node_type: 0.5 },
      },
    });

    miniSearch.addAll(nodes);
    return miniSearch;
  }, [nodes]);

  // Show searching state while debouncing
  useEffect(() => {
    if (query !== debouncedQuery) {
      setIsSearching(true);
    } else {
      setIsSearching(false);
    }
  }, [query, debouncedQuery]);

  // Compute search results based on debounced query
  const results = useMemo<SearchResult[]>(() => {
    if (!searchEngine) return [];
    
    // If no query, show recent/popular nodes (first 8)
    if (!debouncedQuery.trim()) {
      return nodes.slice(0, 8).map((node) => ({
        id: node.id,
        label: node.label || node.id,
        entityType: node.node_type,
        description: node.description,
        score: 0,
      }));
    }

    // Get MiniSearch results first
    const miniSearchResults = searchEngine.search(debouncedQuery).slice(0, 10);
    
    // Convert to our SearchResult format
    const searchResults: SearchResult[] = miniSearchResults.map((r) => ({
      id: r.id,
      label: r.label || r.id,
      entityType: r.node_type,
      description: r.description,
      score: r.score,
    }));
    
    // Middle-content matching fallback (like LightRAG)
    if (searchResults.length < 5) {
      const queryLower = debouncedQuery.toLowerCase();
      const additionalMatches = nodes
        .filter((node) => {
          const label = (node.label || '').toLowerCase();
          const desc = (node.description || '').toLowerCase();
          return (
            label.includes(queryLower) || desc.includes(queryLower)
          ) && !searchResults.some((r) => r.id === node.id);
        })
        .slice(0, 5 - searchResults.length)
        .map((node): SearchResult => ({
          id: node.id,
          label: node.label || node.id,
          entityType: node.node_type,
          description: node.description,
          score: 0.1, // Lower score for fallback matches
        }));
      
      return [...searchResults, ...additionalMatches];
    }
    
    return searchResults;
  }, [debouncedQuery, searchEngine, nodes]);

  // Handle node selection
  const handleSelect = useCallback(
    (nodeId: string) => {
      setOpen(false);
      setQuery('');

      // Select node in store
      selectNode(nodeId);

      // Focus camera on node using normalized coordinates
      if (sigmaInstance) {
        focusCameraOnNode(sigmaInstance, nodeId, {
          ratio: 0.5,
          duration: 500,
          highlight: false, // selectNode already handles highlighting
        });
      }

      onSelect?.(nodeId);
    },
    [sigmaInstance, selectNode, onSelect]
  );

  // Handle keyboard shortcut to open search
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      // Ctrl/Cmd + K to open search
      if ((e.ctrlKey || e.metaKey) && e.key === 'k') {
        e.preventDefault();
        setOpen(true);
      }
      // / key also opens search (when not in input)
      if (e.key === '/' && !['INPUT', 'TEXTAREA'].includes((e.target as HTMLElement)?.tagName || '')) {
        e.preventDefault();
        setOpen(true);
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, []);

  // Focus input when popover opens
  useEffect(() => {
    if (open) {
      setTimeout(() => inputRef.current?.focus(), 0);
    }
  }, [open]);

  return (
    <Popover open={open} onOpenChange={setOpen}>
      <PopoverTrigger asChild>
        <Button 
          variant="outline" 
          size="sm" 
          className="gap-2"
          aria-label={t('graph.search.placeholder', 'Search nodes')}
        >
          <Search className="h-4 w-4" aria-hidden="true" />
          <span className="hidden sm:inline">{t('graph.search.placeholder')}</span>
          <kbd className="hidden lg:inline-flex h-5 items-center gap-1 rounded border bg-muted px-1.5 font-mono text-[10px] font-medium text-muted-foreground">
            <span className="text-xs">⌘</span>K
          </kbd>
        </Button>
      </PopoverTrigger>
      <PopoverContent 
        className="w-96 p-0" 
        align="start"
        role="dialog"
        aria-label={t('graph.search.placeholder', 'Search nodes')}
      >
        <Command shouldFilter={false}>
          <div className="flex items-center border-b px-3 py-1 bg-muted/30" cmdk-input-wrapper="">
            {isSearching ? (
              <Loader2 className="mr-2 h-4 w-4 shrink-0 animate-spin text-muted-foreground" />
            ) : (
              <Search className="mr-2 h-4 w-4 shrink-0 text-muted-foreground" aria-hidden="true" />
            )}
            <CommandInput
              ref={inputRef}
              placeholder={t('graph.search.placeholder', 'Search nodes...')}
              value={query}
              onValueChange={setQuery}
              className="flex h-10 w-full rounded-md bg-transparent py-3 text-sm outline-none focus:outline-none placeholder:text-muted-foreground disabled:cursor-not-allowed disabled:opacity-50"
              aria-label={t('graph.search.placeholder', 'Search nodes')}
            />
          </div>
          <CommandList className="max-h-80">
            {results.length === 0 && debouncedQuery.trim() && (
              <CommandEmpty className="py-6 text-center text-sm text-muted-foreground">
                {t('graph.search.noResults', 'No nodes found')}
              </CommandEmpty>
            )}
            {results.length > 0 && (
              <CommandGroup heading={debouncedQuery.trim() ? t('graph.search.results', 'Results') : t('graph.search.recent', 'Nodes')}>
                {results.map((result) => (
                  <CommandItem
                    key={result.id}
                    value={result.id}
                    onSelect={() => handleSelect(result.id)}
                    className="flex items-start gap-3 px-3 py-2 cursor-pointer"
                    role="option"
                  >
                    <Circle
                      className="h-4 w-4 shrink-0 mt-0.5"
                      style={{ 
                        color: getNodeColor(result.entityType),
                        fill: getNodeColor(result.entityType)
                      }}
                      aria-hidden="true"
                    />
                    <div className="flex-1 min-w-0">
                      <div className="flex items-center gap-2">
                        <span className="font-medium truncate">{result.label}</span>
                        {result.entityType && (
                          <span className="text-xs text-muted-foreground bg-muted px-1.5 py-0.5 rounded shrink-0">
                            {result.entityType}
                          </span>
                        )}
                      </div>
                      {result.description && (
                        <p className="text-xs text-muted-foreground truncate mt-0.5">
                          {result.description}
                        </p>
                      )}
                    </div>
                  </CommandItem>
                ))}
              </CommandGroup>
            )}
            {/* Keyboard hints */}
            <div className="border-t px-3 py-2 text-xs text-muted-foreground flex items-center gap-4">
              <span className="flex items-center gap-1">
                <kbd className="px-1 py-0.5 bg-muted rounded text-[10px]">↑↓</kbd>
                {t('graph.search.navigate', 'Navigate')}
              </span>
              <span className="flex items-center gap-1">
                <kbd className="px-1 py-0.5 bg-muted rounded text-[10px]">↵</kbd>
                {t('graph.search.select', 'Select')}
              </span>
              <span className="flex items-center gap-1">
                <kbd className="px-1 py-0.5 bg-muted rounded text-[10px]">esc</kbd>
                {t('common.close', 'Close')}
              </span>
            </div>
          </CommandList>
        </Command>
      </PopoverContent>
    </Popover>
  );
}

export default GraphSearch;
