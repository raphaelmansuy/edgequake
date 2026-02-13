/**
 * @module EntityBrowserPanel
 * @description Virtual-scrolling entity browser with sorting and grouping.
 * Lists all entities with efficient rendering for large datasets.
 * Supports server-side search when graph is truncated.
 * 
 * @implements UC0109 - User browses all entities
 * @implements FEAT0628 - Virtual scrolling for 1000+ entities
 * @implements FEAT0629 - Sort by name, degree, or type
 * @implements FEAT0630 - Group entities by type
 * @implements FEAT0631 - Server-side search for truncated graphs
 * 
 * @enforces BR0009 - Handle 1000+ entities performantly
 * @enforces BR0620 - Selection syncs with graph view
 * 
 * @see {@link docs/features.md} FEAT0628-0631
 */
"use client";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
    Collapsible,
    CollapsibleContent,
    CollapsibleTrigger,
} from "@/components/ui/collapsible";
import { Input } from "@/components/ui/input";
import { ScrollArea } from "@/components/ui/scroll-area";
import { searchNodes } from "@/lib/api/edgequake";
import { focusCameraOnNode } from "@/lib/graph/camera-utils";
import { cn } from "@/lib/utils";
import { useGraphStore } from "@/stores/use-graph-store";
import { useUIPreferencesStore } from "@/stores/use-ui-preferences-store";
import type { GraphNode } from "@/types";
import { useVirtualizer } from "@tanstack/react-virtual";
import {
    ChevronDown,
    ChevronLeft,
    ChevronRight,
    Cloud,
    Link2,
    Loader2,
    Network,
    Search,
    SortAsc,
    SortDesc,
} from "lucide-react";
import { memo, useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";

// ============================================================================
// Entity Item Component
// ============================================================================

interface EntityItemProps {
  node: GraphNode;
  isSelected: boolean;
  isFocused: boolean;
  onClick: () => void;
  onKeyDown?: (e: React.KeyboardEvent) => void;
}

const EntityItem = memo(function EntityItem({
  node,
  isSelected,
  isFocused,
  onClick,
  onKeyDown,
}: EntityItemProps) {
  const itemRef = useRef<HTMLButtonElement>(null);
  const connectionStrength = Math.min((node.degree || 0) / 10, 1); // Normalize to 0-1
  
  // Focus element when isFocused changes
  useEffect(() => {
    if (isFocused && itemRef.current) {
      itemRef.current.focus();
      itemRef.current.scrollIntoView({ block: 'nearest', behavior: 'smooth' });
    }
  }, [isFocused]);
  
  return (
    <button
      ref={itemRef}
      onClick={onClick}
      onKeyDown={onKeyDown}
      role="option"
      aria-selected={isSelected}
      tabIndex={isFocused ? 0 : -1}
      className={cn(
        "w-full text-left px-2.5 py-1.5 rounded-md transition-all duration-150",
        "flex items-center gap-2 group outline-none",
        "focus-visible:ring-2 focus-visible:ring-primary focus-visible:ring-offset-1",
        isSelected
          ? "bg-primary text-primary-foreground shadow-sm border-l-3 border-primary"
          : isFocused
          ? "bg-muted ring-1 ring-primary/50"
          : "hover:bg-muted/60"
      )}
    >
      <div
        className={cn(
          "w-2 h-2 rounded-full shrink-0 ring-1.5 transition-transform",
          isSelected 
            ? "ring-primary-foreground/30 scale-110" 
            : "ring-white dark:ring-gray-800"
        )}
        style={{
          backgroundColor: getEntityTypeColor(node.node_type ?? "unknown"),
        }}
      />
      <div className="flex-1 min-w-0">
        <p className={cn(
          "text-xs font-medium truncate leading-tight",
          isSelected && "font-semibold"
        )}>
          {node.label ?? node.id ?? "Unknown"}
        </p>
        <div className="flex items-center gap-1.5 mt-0.5">
          <span className={cn(
            "text-[9px] uppercase tracking-wider",
            isSelected ? "text-primary-foreground/70" : "text-muted-foreground"
          )}>
            {node.node_type ?? "unknown"}
          </span>
          {node.degree && node.degree > 0 && (
            <>
              <span className={cn(
                "text-[9px]",
                isSelected ? "text-primary-foreground/50" : "text-muted-foreground/50"
              )}>·</span>
              <div className="flex items-center gap-0.5">
                <div className="w-8 h-0.5 bg-muted/50 rounded-full overflow-hidden">
                  <div 
                    className={cn(
                      "h-full rounded-full transition-all",
                      isSelected ? "bg-primary-foreground/60" : "bg-primary/60"
                    )}
                    style={{ width: `${connectionStrength * 100}%` }}
                  />
                </div>
                <span className={cn(
                  "text-[9px] font-medium tabular-nums",
                  isSelected ? "text-primary-foreground/70" : "text-muted-foreground"
                )}>
                  {node.degree}
                </span>
              </div>
            </>
          )}
        </div>
      </div>
    </button>
  );
});

// Entity type color mapping
function getEntityTypeColor(type: string): string {
  const colorMap: Record<string, string> = {
    PERSON: "#3b82f6",
    ORGANIZATION: "#8b5cf6",
    LOCATION: "#22c55e",
    EVENT: "#f97316",
    CONCEPT: "#ec4899",
    DOCUMENT: "#6366f1",
    TECHNOLOGY: "#14b8a6",
    PRODUCT: "#f59e0b",
    DEFAULT: "#94a3b8",
  };
  return colorMap[type.toUpperCase()] || colorMap.DEFAULT;
}

// ============================================================================
// Virtualized Entity List Component (for performance with large datasets)
// ============================================================================

interface VirtualizedEntityListProps {
  nodes: GraphNode[];
  selectedNodeId: string | null;
  focusedIndex: number;
  onNodeClick: (nodeId: string) => void;
  onKeyDown: (e: React.KeyboardEvent) => void;
  onFocusChange: (index: number) => void;
}

const VirtualizedEntityList = memo(function VirtualizedEntityList({
  nodes,
  selectedNodeId,
  focusedIndex,
  onNodeClick,
  onKeyDown,
  onFocusChange,
}: VirtualizedEntityListProps) {
  const { t } = useTranslation();
  const parentRef = useRef<HTMLDivElement>(null);
  
  const virtualizer = useVirtualizer({
    count: nodes.length,
    getScrollElement: () => parentRef.current,
    estimateSize: () => 52, // Estimated height of each EntityItem
    overscan: 5, // Render 5 extra items above/below viewport
  });

  // Scroll to focused item
  useEffect(() => {
    if (focusedIndex >= 0 && focusedIndex < nodes.length) {
      virtualizer.scrollToIndex(focusedIndex, { align: 'auto', behavior: 'smooth' });
    }
  }, [focusedIndex, nodes.length, virtualizer]);

  return (
    <div
      ref={parentRef}
      role="listbox"
      aria-label={t("graph.entityBrowser.entityList", "Entity list")}
      tabIndex={0}
      onKeyDown={onKeyDown}
      onFocus={() => {
        if (focusedIndex === -1 && nodes.length > 0) {
          onFocusChange(0);
        }
      }}
      className="h-full overflow-auto outline-none focus-visible:ring-2 focus-visible:ring-primary focus-visible:ring-offset-2 rounded-md"
      style={{ contain: 'strict' }}
    >
      <div
        style={{
          height: `${virtualizer.getTotalSize()}px`,
          width: '100%',
          position: 'relative',
        }}
      >
        {virtualizer.getVirtualItems().map((virtualRow) => {
          const node = nodes[virtualRow.index];
          return (
            <div
              key={node.id}
              style={{
                position: 'absolute',
                top: 0,
                left: 0,
                width: '100%',
                height: `${virtualRow.size}px`,
                transform: `translateY(${virtualRow.start}px)`,
              }}
            >
              <EntityItem
                node={node}
                isSelected={node.id === selectedNodeId}
                isFocused={virtualRow.index === focusedIndex}
                onClick={() => onNodeClick(node.id)}
                onKeyDown={onKeyDown}
              />
            </div>
          );
        })}
      </div>
    </div>
  );
});

// ============================================================================
// Entity Type Group Component
// ============================================================================

interface EntityTypeGroupProps {
  type: string;
  nodes: GraphNode[];
  selectedNodeId: string | null;
  onNodeClick: (nodeId: string) => void;
  defaultOpen?: boolean;
}

const EntityTypeGroup = memo(function EntityTypeGroup({
  type,
  nodes,
  selectedNodeId,
  onNodeClick,
  defaultOpen = false,
}: EntityTypeGroupProps) {
  const [isOpen, setIsOpen] = useState(defaultOpen);
  const { t } = useTranslation();
  const groupId = `entity-group-${type.toLowerCase().replace(/\s+/g, '-')}`;

  return (
    <Collapsible open={isOpen} onOpenChange={setIsOpen}>
      <CollapsibleTrigger asChild>
        <Button
          variant="ghost"
          className="w-full justify-between px-3 py-2 h-auto"
          aria-expanded={isOpen}
          aria-controls={groupId}
        >
          <div className="flex items-center gap-2">
            <div
              className="w-3 h-3 rounded-full"
              style={{ backgroundColor: getEntityTypeColor(type) }}
              aria-hidden="true"
            />
            <span className="text-sm font-medium">{type}</span>
          </div>
          <div className="flex items-center gap-2">
            <Badge variant="secondary" className="text-xs" aria-label={t("graph.entityBrowser.entityCount", "{{count}} entities", { count: nodes.length })}>
              {nodes.length}
            </Badge>
            {isOpen ? (
              <ChevronDown className="h-4 w-4" aria-hidden="true" />
            ) : (
              <ChevronRight className="h-4 w-4" aria-hidden="true" />
            )}
          </div>
        </Button>
      </CollapsibleTrigger>
      <CollapsibleContent className="pl-2 space-y-1" id={groupId}>
        <div role="group" aria-label={`${type} ${t("graph.entityBrowser.entities", "entities")}`}>
          {nodes.map((node) => (
            <EntityItem
              key={node.id}
              node={node}
              isSelected={node.id === selectedNodeId}
              isFocused={false}
              onClick={() => onNodeClick(node.id)}
            />
          ))}
        </div>
      </CollapsibleContent>
    </Collapsible>
  );
});

// ============================================================================
// Sort Options
// ============================================================================

type SortOption = "name" | "degree" | "type";
type SortDirection = "asc" | "desc";

// ============================================================================
// Main Entity Browser Panel
// ============================================================================

interface EntityBrowserPanelProps {
  className?: string;
}

export function EntityBrowserPanel({ className }: EntityBrowserPanelProps) {
  const { t } = useTranslation();
  
  // Persisted UI preferences
  const {
    graphEntityBrowserCollapsed,
    setGraphEntityBrowserCollapsed,
    entityBrowserViewMode,
    setEntityBrowserViewMode,
    entityBrowserSortBy,
    setEntityBrowserSortBy,
    entityBrowserSortAsc,
    setEntityBrowserSortAsc,
  } = useUIPreferencesStore();
  
  // Local state (not persisted)
  const [searchQuery, setSearchQuery] = useState("");
  const [focusedIndex, setFocusedIndex] = useState(-1);
  const [isServerSearching, setIsServerSearching] = useState(false);
  const [serverSearchNodes, setServerSearchNodes] = useState<GraphNode[]>([]);
  const listRef = useRef<HTMLDivElement>(null);
  
  // Derived state from preferences
  const isOpen = !graphEntityBrowserCollapsed;
  const setIsOpen = (open: boolean) => setGraphEntityBrowserCollapsed(!open);
  const viewMode = entityBrowserViewMode;
  const setViewMode = setEntityBrowserViewMode;
  const sortBy = entityBrowserSortBy;
  const setSortBy = setEntityBrowserSortBy;
  const sortDirection: SortDirection = entityBrowserSortAsc ? "asc" : "desc";
  const setSortDirection = (dir: SortDirection) => setEntityBrowserSortAsc(dir === "asc");

  const { nodes, selectedNodeId, selectNode, sigmaInstance, isTruncated, addNodesToGraph } = useGraphStore();

  // Filter nodes by search query (local/client-side)
  const localFilteredNodes = useMemo(() => {
    if (!searchQuery.trim()) return nodes;
    const query = searchQuery.toLowerCase();
    return nodes.filter(
      (node) =>
        node.label?.toLowerCase().includes(query) ||
        node.node_type?.toLowerCase().includes(query) ||
        node.description?.toLowerCase().includes(query)
    );
  }, [nodes, searchQuery]);

  // Server-side search: always query server for comprehensive results
  // WHY: Users expect search to find all entities in the knowledge base,
  // not just what's currently visible in the graph
  useEffect(() => {
    // Reset server results when query changes
    setServerSearchNodes([]);

    // FEAT0405: Enable server search for any query >= 2 chars
    // Removed isTruncated check to always search the full knowledge base
    const shouldServerSearch = searchQuery.trim().length >= 2;

    if (!shouldServerSearch) {
      setIsServerSearching(false);
      return;
    }

    let cancelled = false;
    setIsServerSearching(true);

    searchNodes({
      q: searchQuery.trim(),
      limit: 50,
      includeNeighbors: true,
      neighborDepth: 1,
    })
      .then((response) => {
        if (cancelled) return;

        // Add server nodes/edges to graph for visualization
        if (response.nodes.length > 0) {
          addNodesToGraph(response.nodes, response.edges);
        }

        setServerSearchNodes(response.nodes);
      })
      .catch((error) => {
        if (cancelled) return;
        console.error('[EntityBrowser] Server search failed:', error);
      })
      .finally(() => {
        if (!cancelled) {
          setIsServerSearching(false);
        }
      });

    return () => {
      cancelled = true;
    };
  }, [searchQuery, addNodesToGraph]);

  // Combine local and server results
  const filteredNodes = useMemo(() => {
    if (serverSearchNodes.length > 0) {
      // Merge: local results first, then add unique server results
      const localIds = new Set(localFilteredNodes.map(n => n.id));
      const uniqueServerNodes = serverSearchNodes.filter(n => !localIds.has(n.id));
      return [...localFilteredNodes, ...uniqueServerNodes];
    }
    return localFilteredNodes;
  }, [localFilteredNodes, serverSearchNodes]);

  // Sort nodes
  const sortedNodes = useMemo(() => {
    const sorted = [...filteredNodes].sort((a, b) => {
      let comparison = 0;
      switch (sortBy) {
        case "name":
          comparison = (a.label ?? "").localeCompare(b.label ?? "");
          break;
        case "degree":
          comparison = (b.degree ?? 0) - (a.degree ?? 0);
          break;
        case "type":
          comparison = (a.node_type ?? "").localeCompare(b.node_type ?? "");
          break;
      }
      return sortDirection === "asc" ? comparison : -comparison;
    });
    return sorted;
  }, [filteredNodes, sortBy, sortDirection]);

  // Group nodes by type
  const groupedNodes = useMemo(() => {
    const groups: Record<string, GraphNode[]> = {};
    for (const node of sortedNodes) {
      const type = node.node_type || "Unknown";
      if (!groups[type]) {
        groups[type] = [];
      }
      groups[type].push(node);
    }
    // Sort groups by count (descending)
    return Object.entries(groups).sort((a, b) => b[1].length - a[1].length);
  }, [sortedNodes]);

  // Handle node click with camera focus
  const handleNodeClick = useCallback(
    (nodeId: string) => {
      selectNode(nodeId);

      // Focus camera on selected node
      // WHY: Wait for Sigma to render the node before focusing
      // Server search results are added to graph asynchronously
      if (sigmaInstance) {
        const graph = sigmaInstance.getGraph();

        // Check if node exists in Sigma graph
        if (graph.hasNode(nodeId)) {
          // Node already rendered, focus immediately
          focusCameraOnNode(sigmaInstance, nodeId, {
            ratio: 0.3,
            duration: 500,
            highlight: false, // selectNode already handles highlighting
          });
        } else {
          // Node not yet rendered, wait for next frame
          // WHY: requestAnimationFrame ensures React has re-rendered and Sigma has updated
          requestAnimationFrame(() => {
            if (graph.hasNode(nodeId)) {
              focusCameraOnNode(sigmaInstance, nodeId, {
                ratio: 0.3,
                duration: 500,
                highlight: false,
              });
            } else {
              // Still not ready, try one more time after a short delay
              setTimeout(() => {
                focusCameraOnNode(sigmaInstance, nodeId, {
                  ratio: 0.3,
                  duration: 500,
                  highlight: false,
                });
              }, 100);
            }
          });
        }
      }
    },
    [selectNode, sigmaInstance]
  );

  // Toggle sort direction
  const toggleSortDirection = useCallback(() => {
    setSortDirection(sortDirection === "asc" ? "desc" : "asc");
  }, [sortDirection, setSortDirection]);

  // Keyboard navigation for list view
  const handleListKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      if (viewMode !== "list" || sortedNodes.length === 0) return;

      switch (e.key) {
        case "ArrowDown":
          e.preventDefault();
          setFocusedIndex((prev) =>
            prev < sortedNodes.length - 1 ? prev + 1 : 0
          );
          break;
        case "ArrowUp":
          e.preventDefault();
          setFocusedIndex((prev) =>
            prev > 0 ? prev - 1 : sortedNodes.length - 1
          );
          break;
        case "Home":
          e.preventDefault();
          setFocusedIndex(0);
          break;
        case "End":
          e.preventDefault();
          setFocusedIndex(sortedNodes.length - 1);
          break;
        case "Enter":
        case " ":
          e.preventDefault();
          if (focusedIndex >= 0 && focusedIndex < sortedNodes.length) {
            handleNodeClick(sortedNodes[focusedIndex].id);
          }
          break;
        case "Escape":
          setFocusedIndex(-1);
          listRef.current?.blur();
          break;
      }
    },
    [viewMode, sortedNodes, focusedIndex, handleNodeClick]
  );

  // Reset focused index when search or sort changes
  useEffect(() => {
    // Intentional: Reset focus when filters change for better UX
    // eslint-disable-next-line react-hooks/set-state-in-effect
    setFocusedIndex(-1);
  }, [searchQuery, sortBy, sortDirection]);

  // Collapsed state
  if (!isOpen) {
    return (
      <div
        className={cn(
          "flex flex-col items-center justify-start py-2 w-10 border-r bg-card/80 backdrop-blur-sm shrink-0 transition-all duration-200",
          className
        )}
      >
        <Button
          variant="ghost"
          size="icon"
          className="h-7 w-7 hover:bg-muted"
          onClick={() => setIsOpen(true)}
          aria-label={t("graph.entityBrowser.expand", "Expand entity browser")}
        >
          <ChevronRight className="h-3.5 w-3.5" />
        </Button>
        <div className="mt-3 flex flex-col items-center gap-1.5">
          <Network className="h-3.5 w-3.5 text-muted-foreground" />
          <span
            className="text-[10px] text-muted-foreground font-medium"
            style={{ writingMode: "vertical-rl", textOrientation: "mixed" }}
          >
            {t("graph.entityBrowser.title", "Entities")}
          </span>
          <Badge variant="secondary" className="text-[9px] h-4 px-1">
            {nodes.length}
          </Badge>
        </div>
      </div>
    );
  }

  return (
    <aside
      className={cn(
        "flex flex-col w-64 border-r bg-card/95 backdrop-blur-sm shrink-0 overflow-hidden transition-all duration-200",
        className
      )}
      aria-label={t("graph.entityBrowser.title", "Entity browser")}
      data-tour="entity-browser"
    >
      {/* Header - More compact */}
      <div className="flex items-center justify-between px-3 py-2 border-b shrink-0 bg-muted/20">
        <div className="flex items-center gap-1.5">
          <Network className="h-3.5 w-3.5 text-muted-foreground" />
          <h2 className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">
            {t("graph.entityBrowser.title", "Entities")}
          </h2>
          <Badge variant="secondary" className="text-[9px] h-4 px-1.5">
            {filteredNodes.length}
            {filteredNodes.length !== nodes.length && `/${nodes.length}`}
          </Badge>
        </div>
        <Button
          variant="ghost"
          size="icon"
          className="h-6 w-6"
          onClick={() => setIsOpen(false)}
          aria-label={t("graph.entityBrowser.collapse", "Collapse entity browser")}
        >
          <ChevronLeft className="h-3.5 w-3.5" />
        </Button>
      </div>

      {/* Search */}
      <div className="p-2 border-b shrink-0">
        <div className="relative">
          {isServerSearching ? (
            <Loader2 className="absolute left-2 top-1/2 -translate-y-1/2 h-3 w-3 text-muted-foreground animate-spin" aria-hidden="true" />
          ) : (
            <Search className="absolute left-2 top-1/2 -translate-y-1/2 h-3 w-3 text-muted-foreground" aria-hidden="true" />
          )}
          <Input
            placeholder={t("graph.entityBrowser.search", "Search entities...")}
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            className="h-7 pl-7 pr-7 text-xs bg-muted/30 border-muted focus:bg-background transition-colors"
            aria-label={t("graph.entityBrowser.searchLabel", "Search entities by name, type, or description")}
            aria-describedby="entity-search-hint"
          />
          {serverSearchNodes.length > 0 && (
            <Cloud className="absolute right-2 top-1/2 -translate-y-1/2 h-3 w-3 text-blue-500" aria-label="Including server results" />
          )}
          <span id="entity-search-hint" className="sr-only">
            {t("graph.entityBrowser.searchHint", "Type to filter the list of entities. Results update automatically.")}
          </span>
        </div>
        {isTruncated && searchQuery.trim().length >= 2 && (
          <p className="text-[10px] text-muted-foreground mt-1 flex items-center gap-1">
            <Cloud className="h-2.5 w-2.5" />
            {isServerSearching 
              ? t("graph.entityBrowser.searchingServer", "Searching full database...")
              : serverSearchNodes.length > 0 
                ? t("graph.entityBrowser.serverResultsCount", "{{count}} from server", { count: serverSearchNodes.length })
                : t("graph.entityBrowser.willSearchServer", "Will search server if needed")
            }
          </p>
        )}
      </div>

      {/* Sort Controls */}
      <div className="flex items-center gap-0.5 px-2 py-1.5 border-b shrink-0" role="group" aria-label={t("graph.entityBrowser.sortControls", "Sort controls")}>
        <span className="text-[10px] text-muted-foreground mr-1" id="sort-label">
          {t("common.sortBy", "Sort:")}
        </span>
        <Button
          variant={sortBy === "name" ? "secondary" : "ghost"}
          size="sm"
          className="h-5 text-[10px] px-1.5"
          onClick={() => setSortBy("name")}
          aria-pressed={sortBy === "name"}
          aria-describedby="sort-label"
        >
          {t("common.name", "Name")}
        </Button>
        <Button
          variant={sortBy === "degree" ? "secondary" : "ghost"}
          size="sm"
          className="h-5 text-[10px] px-1.5"
          onClick={() => setSortBy("degree")}
          aria-pressed={sortBy === "degree"}
          aria-describedby="sort-label"
        >
          {t("graph.degree", "Degree")}
        </Button>
        <Button
          variant="ghost"
          size="icon"
          className="h-5 w-5 ml-auto"
          onClick={toggleSortDirection}
          aria-label={sortDirection === "asc" 
            ? t("graph.entityBrowser.sortAscending", "Sort ascending, click to sort descending") 
            : t("graph.entityBrowser.sortDescending", "Sort descending, click to sort ascending")}
        >
          {sortDirection === "asc" ? (
            <SortAsc className="h-2.5 w-2.5" aria-hidden="true" />
          ) : (
            <SortDesc className="h-2.5 w-2.5" aria-hidden="true" />
          )}
        </Button>
      </div>

      {/* View Mode Toggle */}
      <div className="flex items-center gap-0.5 p-1.5 border-b shrink-0" role="tablist" aria-label={t("graph.entityBrowser.viewModeLabel", "View mode")}>
        <Button
          variant={viewMode === "grouped" ? "secondary" : "ghost"}
          size="sm"
          className="flex-1 h-6 text-[10px]"
          onClick={() => setViewMode("grouped")}
          role="tab"
          aria-selected={viewMode === "grouped"}
          aria-controls="entity-panel-content"
        >
          {t("graph.entityBrowser.grouped", "Grouped")}
        </Button>
        <Button
          variant={viewMode === "list" ? "secondary" : "ghost"}
          size="sm"
          className="flex-1 h-6 text-[10px]"
          onClick={() => setViewMode("list")}
          role="tab"
          aria-selected={viewMode === "list"}
          aria-controls="entity-panel-content"
        >
          {t("graph.entityBrowser.list", "List")}
        </Button>
      </div>

      {/* Entity List */}
      <div className="flex-1 min-h-0 overflow-hidden" id="entity-panel-content" role="tabpanel">
        {filteredNodes.length === 0 ? (
          <div className="py-6 text-center" role="status" aria-live="polite">
            <Network className="h-6 w-6 mx-auto text-muted-foreground/50 mb-1.5" aria-hidden="true" />
            <p className="text-xs text-muted-foreground">
              {searchQuery
                ? t("graph.entityBrowser.noResults", "No entities found")
                : t("graph.entityBrowser.empty", "No entities yet")}
            </p>
          </div>
        ) : viewMode === "grouped" ? (
          <ScrollArea className="h-full" showShadows>
            <div className="py-2 px-1.5 space-y-0.5">
              {groupedNodes.map(([type, typeNodes]) => (
                <EntityTypeGroup
                  key={type}
                  type={type}
                  nodes={typeNodes}
                  selectedNodeId={selectedNodeId}
                  onNodeClick={handleNodeClick}
                  defaultOpen={typeNodes.length <= 10}
                />
              ))}
            </div>
          </ScrollArea>
        ) : (
          <div className="h-full p-1.5">
            <VirtualizedEntityList
              nodes={sortedNodes}
              selectedNodeId={selectedNodeId}
              focusedIndex={focusedIndex}
              onNodeClick={handleNodeClick}
              onKeyDown={handleListKeyDown}
              onFocusChange={setFocusedIndex}
            />
          </div>
        )}
      </div>

      {/* Footer with stats */}
      <div className="p-3 border-t shrink-0 bg-muted/20">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            <Badge variant="secondary" className="text-[10px] px-2 py-0.5">
              {groupedNodes.length} {t("graph.entityBrowser.types", "types")}
            </Badge>
          </div>
          <div className="flex items-center gap-1.5 text-xs text-muted-foreground">
            <Link2 className="h-3 w-3" />
            <span className="font-medium">
              {Math.floor(filteredNodes.reduce((acc, n) => acc + (n.degree ?? 0), 0) / 2)}
            </span>
            <span>{t("graph.entityBrowser.connections", "connections")}</span>
          </div>
        </div>
      </div>
    </aside>
  );
}

export default EntityBrowserPanel;
