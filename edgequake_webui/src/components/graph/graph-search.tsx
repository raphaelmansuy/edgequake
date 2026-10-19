/**
 * @module GraphSearch
 * @description Full-text search for graph entities with hybrid client/server search.
 * Uses MiniSearch for instant local results, automatically falls back to server
 * when graph is truncated and local search yields no results.
 * 
 * @implements UC0108 - User searches entities by name
 * @implements FEAT0202 - Full-text entity search
 * @implements FEAT0626 - Camera focus on selected entity
 * @implements FEAT0627 - Server-side search for full workspace
 * 
 * @enforces BR0616 - Search results sorted by relevance
 * @enforces BR0617 - Entity types color-coded in results
 * 
 * @see {@link docs/features.md} FEAT0202
 */
'use client';

import { AskAboutEntityButton } from '@/components/shared/ask-about-entity-button';
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
import { Kbd } from '@/components/shared/kbd';
import { searchNodes } from '@/lib/api/edgequake';
import { focusCameraOnNode } from '@/lib/graph/camera-utils';
import { displayEntityLabel } from '@/lib/graph/label-utils';
import { isEditableTarget, isOverlayOpen } from '@/lib/keyboard/dom';
import { useGraphStore } from '@/stores/use-graph-store';
import type { GraphNode } from '@/types';
import { Circle, Cloud, Loader2, Search } from 'lucide-react';
import MiniSearch from 'minisearch';
import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';

import { useEntityTypeColors } from '@/hooks/use-entity-type-colors';

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
  isServerResult?: boolean; // True if from server-side search
}

interface ServerSearchState {
  query: string;
  results: SearchResult[];
  error: string | null;
}

interface GraphSearchProps {
  onSelect?: (nodeId: string) => void;
}

export function GraphSearch({ onSelect }: GraphSearchProps) {
  const { t } = useTranslation();
  const { colorFor } = useEntityTypeColors();
  const { nodes, sigmaInstance, selectNode, isTruncated, addNodesToGraph } = useGraphStore();
  const [open, setOpen] = useState(false);
  const [query, setQuery] = useState('');
  const [activeServerSearchQuery, setActiveServerSearchQuery] = useState<string | null>(null);
  const [serverSearch, setServerSearch] = useState<ServerSearchState>({
    query: '',
    results: [],
    error: null,
  });
  const inputRef = useRef<HTMLInputElement>(null);
  
  // Debounce search query for better performance
  const debouncedQuery = useDebounce(query, 150);
  const serverResults = useMemo(
    () => (serverSearch.query === debouncedQuery ? serverSearch.results : []),
    [debouncedQuery, serverSearch.query, serverSearch.results],
  );
  const serverSearchError = useMemo(
    () => (serverSearch.query === debouncedQuery ? serverSearch.error : null),
    [debouncedQuery, serverSearch.error, serverSearch.query],
  );
  const isSearching = query !== debouncedQuery;
  const isServerSearching = activeServerSearchQuery === debouncedQuery;

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

    // Deduplicate nodes by ID to prevent MiniSearch errors
    // WHY: MiniSearch throws on duplicate IDs, backend may return duplicates
    const uniqueNodes = new Map<string, GraphNode>();
    const duplicates: string[] = [];
    
    nodes.forEach((node) => {
      if (uniqueNodes.has(node.id)) {
        duplicates.push(node.id);
      } else {
        uniqueNodes.set(node.id, node);
      }
    });

    // Log warning if duplicates found (helps debugging)
    if (duplicates.length > 0) {
      console.warn(
        `[GraphSearch] Found ${duplicates.length} duplicate node ID(s):`,
        duplicates.slice(0, 5), // Show first 5
        duplicates.length > 5 ? `... and ${duplicates.length - 5} more` : ''
      );
    }

    try {
      // Add unique nodes to search index
      const uniqueNodesArray = Array.from(uniqueNodes.values());
      miniSearch.addAll(uniqueNodesArray);
      
      return miniSearch;
    } catch (error) {
      console.error('[GraphSearch] Failed to initialize search index:', error);
      return null;
    }
  }, [nodes]);

  // Compute search results based on debounced query
  const results = useMemo<SearchResult[]>(() => {
    if (!searchEngine) return [];
    
    // If no query, show recent/popular nodes (first 8)
    if (!debouncedQuery.trim()) {
      // Deduplicate nodes for display (same robustness as search index)
      const uniqueDisplayNodes = new Map<string, GraphNode>();
      nodes.forEach((node) => {
        if (!uniqueDisplayNodes.has(node.id)) {
          uniqueDisplayNodes.set(node.id, node);
        }
      });
      
      return Array.from(uniqueDisplayNodes.values())
        .slice(0, 8)
        .map((node) => ({
          id: node.id,
          label: displayEntityLabel({ label: node.label, id: node.id }),
          entityType: node.node_type,
          description: node.description,
          score: 0,
        }));
    }

    try {
      // Get MiniSearch results first
      const miniSearchResults = searchEngine.search(debouncedQuery).slice(0, 10);
      
      // Convert to our SearchResult format
      const searchResults: SearchResult[] = miniSearchResults.map((r) => ({
        id: r.id,
        label: displayEntityLabel({ label: r.label, id: r.id }),
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
          label: displayEntityLabel({ label: node.label, id: node.id }),
          entityType: node.node_type,
          description: node.description,
          score: 0.1, // Lower score for fallback matches
        }));
      
      return [...searchResults, ...additionalMatches];
    }
    
    return searchResults;
    } catch (error) {
      console.error('[GraphSearch] Search failed:', error);
      return [];
    }
  }, [debouncedQuery, searchEngine, nodes]);

  // Server-side search — only when local results are insufficient (SPEC-053 F1).
  // WHY: FEAT0405 removed the isTruncated guard which caused a server search on
  // every keystroke (≥2 chars) regardless of local results. This created a burst
  // of search_nodes calls that competed with graph stream for the materialization
  // semaphore, causing 503 errors.
  //
  // Correct condition: only call the server when the local MiniSearch index
  // cannot satisfy the query — i.e., the graph is truncated (not all nodes loaded)
  // or local results are thin. This preserves the "full database" coverage for
  // truncated graphs while avoiding pointless server calls for full graphs.
  useEffect(() => {
    const queryTrimmed = debouncedQuery.trim();
    // WHY ≥3 (was 2): reduces server calls by ~50% while still covering all
    // meaningful queries (single/double chars are almost never meaningful entity names).
    const isQueryLongEnough = queryTrimmed.length >= 3;
    // Only hit the server when: (a) graph is truncated so local index is partial,
    // OR (b) local index returned fewer than 3 results (might be missing nodes).
    const localResultsThin = results.length < 3;
    const shouldServerSearch = isQueryLongEnough && (isTruncated || localResultsThin);

    if (!shouldServerSearch) {
      // Clear stale server error state when the condition no longer applies
      if (serverSearch.error) {
        setServerSearch({ query: '', results: [], error: null });
      }
      return;
    }

    let cancelled = false;
    Promise.resolve().then(() => {
      if (!cancelled) {
        setActiveServerSearchQuery(debouncedQuery);
      }
    });

    searchNodes({
      q: queryTrimmed,
      limit: 20,
      includeNeighbors: true,
      neighborDepth: 1,
    })
      .then((response) => {
        if (cancelled) return;

        // Add server nodes/edges to graph for visualization
        if (response.nodes.length > 0) {
          addNodesToGraph(response.nodes, response.edges);
        }

        // Convert to search results
        const serverSearchResults: SearchResult[] = response.nodes.map((node) => ({
          id: node.id,
          label: displayEntityLabel({ label: node.label, id: node.id }),
          entityType: node.node_type,
          description: node.description,
          score: 1, // Server results ranked by relevance
          isServerResult: true,
        }));

        setServerSearch({
          query: debouncedQuery,
          results: serverSearchResults,
          error: null,
        });
      })
      .catch((error) => {
        if (cancelled) return;
        // WHY silent fallback for capacity errors (SPEC-053 F2):
        // A 503 "Graph materialization capacity reached" is a transient server-load
        // condition. The local MiniSearch index likely has results already.
        // Showing an error banner confuses users — silently fall back to local results.
        const isTransientCapacity =
          error?.message?.includes('capacity') ||
          error?.message?.includes('503') ||
          error?.status === 503;
        if (!isTransientCapacity) {
          console.error('[GraphSearch] Server search failed:', error);
        }
        setServerSearch({
          query: debouncedQuery,
          results: [],
          error: isTransientCapacity ? null : (error.message || 'Search failed'),
        });
      })
      .finally(() => {
        if (!cancelled) {
          setActiveServerSearchQuery(null);
        }
      });

    return () => {
      cancelled = true;
    };
  }, [debouncedQuery, results.length, isTruncated, addNodesToGraph, serverSearch.error]);

  // Combine local and server results
  const combinedResults = useMemo(() => {
    if (serverResults.length > 0) {
      // When we have server results, show them (they're from the full database)
      return serverResults;
    }
    return results;
  }, [results, serverResults]);

  // Handle node selection
  const handleSelect = useCallback(
    (nodeId: string) => {
      setOpen(false);
      setQuery('');

      // Select node in store
      selectNode(nodeId);

      // Focus camera on node using normalized coordinates
      // WHY: Wait for Sigma to render the node before focusing
      // Server search results are added to graph asynchronously
      if (sigmaInstance) {
        const graph = sigmaInstance.getGraph();

        // Check if node exists in Sigma graph
        if (graph.hasNode(nodeId)) {
          // Node already rendered, focus immediately
          focusCameraOnNode(sigmaInstance, nodeId, {
            ratio: 0.5,
            duration: 500,
            highlight: false, // selectNode already handles highlighting
          });
        } else {
          // Node not yet rendered, wait for next frame
          // WHY: requestAnimationFrame ensures React has re-rendered and Sigma has updated
          requestAnimationFrame(() => {
            if (graph.hasNode(nodeId)) {
              focusCameraOnNode(sigmaInstance, nodeId, {
                ratio: 0.5,
                duration: 500,
                highlight: false,
              });
            } else {
              // Still not ready, try one more time after a short delay
              setTimeout(() => {
                focusCameraOnNode(sigmaInstance, nodeId, {
                  ratio: 0.5,
                  duration: 500,
                  highlight: false,
                });
              }, 100);
            }
          });
        }
      }

      onSelect?.(nodeId);
    },
    [sigmaInstance, selectNode, onSelect]
  );

  // `/` opens node search. ⌘K belongs to the global command palette (SPEC-168),
  // so binding it here too would open two surfaces at once.
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key !== '/' || e.defaultPrevented) return;
      if (e.metaKey || e.ctrlKey || e.altKey) return;
      if (isEditableTarget(e.target) || isOverlayOpen()) return;
      e.preventDefault();
      setOpen(true);
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
          <Kbd className="hidden lg:inline-flex">/</Kbd>
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
            {/* Server search in progress */}
            {isServerSearching && (
              <div className="py-4 text-center text-sm text-muted-foreground flex items-center justify-center gap-2">
                <Loader2 className="h-4 w-4 animate-spin" />
                <span>{t('graph.search.searchingServer', 'Searching full database...')}</span>
              </div>
            )}
            
            {/* Server search error */}
            {serverSearchError && !isServerSearching && (
              <div className="py-4 text-center text-sm text-destructive">
                {serverSearchError}
              </div>
            )}

            {/* No results - show only if not server searching */}
            {combinedResults.length === 0 && debouncedQuery.trim() && !isServerSearching && (
              <CommandEmpty className="py-6 text-center text-sm text-muted-foreground">
                {isTruncated 
                  ? t('graph.search.noResultsSearchingServer', 'No local matches. Searching server...')
                  : t('graph.search.noResults', 'No nodes found')
                }
              </CommandEmpty>
            )}
            
            {combinedResults.length > 0 && (
              <CommandGroup heading={
                serverResults.length > 0 
                  ? t('graph.search.serverResults', 'Server Results')
                  : debouncedQuery.trim() 
                    ? t('graph.search.results', 'Results') 
                    : t('graph.search.recent', 'Nodes')
              }>
                {combinedResults.map((result) => (
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
                        color: colorFor(result.entityType),
                        fill: colorFor(result.entityType)
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
                        {result.isServerResult && (
                          <Cloud className="h-3 w-3 text-blue-500 shrink-0" aria-label="Server result" />
                        )}
                      </div>
                      {result.description && (
                        <p className="text-xs text-muted-foreground truncate mt-0.5">
                          {result.description}
                        </p>
                      )}
                    </div>
                    <AskAboutEntityButton
                      node={{
                        id: result.id,
                        label: result.label,
                        node_type: result.entityType,
                      }}
                      testId="graph-search-ask"
                    />
                  </CommandItem>
                ))}
              </CommandGroup>
            )}
            {/* Keyboard hints */}
            <div className="border-t px-3 py-2 text-xs text-muted-foreground flex items-center gap-4">
              <span className="flex items-center gap-1">
                <kbd className="px-1 py-0.5 bg-muted rounded text-xs">↑↓</kbd>
                {t('graph.search.navigate', 'Navigate')}
              </span>
              <span className="flex items-center gap-1">
                <kbd className="px-1 py-0.5 bg-muted rounded text-xs">↵</kbd>
                {t('graph.search.select', 'Select')}
              </span>
              <span className="flex items-center gap-1">
                <kbd className="px-1 py-0.5 bg-muted rounded text-xs">esc</kbd>
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
