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
import { cn } from "@/lib/utils";
import { useGraphStore } from "@/stores/use-graph-store";
import type { GraphNode } from "@/types";
import {
    ChevronDown,
    ChevronLeft,
    ChevronRight,
    Link2,
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
        "w-full text-left px-3 py-2.5 rounded-lg transition-all duration-150",
        "flex items-center gap-2.5 group outline-none",
        "focus-visible:ring-2 focus-visible:ring-primary focus-visible:ring-offset-1",
        isSelected
          ? "bg-primary text-primary-foreground shadow-sm border-l-4 border-primary"
          : isFocused
          ? "bg-muted ring-1 ring-primary/50"
          : "hover:bg-muted/70 hover:translate-x-0.5"
      )}
    >
      <div
        className={cn(
          "w-2.5 h-2.5 rounded-full shrink-0 ring-2 transition-transform",
          isSelected 
            ? "ring-primary-foreground/30 scale-125" 
            : "ring-white dark:ring-gray-800"
        )}
        style={{
          backgroundColor: getEntityTypeColor(node.node_type),
        }}
      />
      <div className="flex-1 min-w-0">
        <p className={cn(
          "text-sm font-medium truncate",
          isSelected && "font-semibold"
        )}>
          {node.label}
        </p>
        <div className="flex items-center gap-2">
          <span className={cn(
            "text-[10px] uppercase tracking-wider",
            isSelected ? "text-primary-foreground/70" : "text-muted-foreground"
          )}>
            {node.node_type}
          </span>
          {node.degree && node.degree > 0 && (
            <>
              <span className={cn(
                "text-[10px]",
                isSelected ? "text-primary-foreground/50" : "text-muted-foreground/50"
              )}>·</span>
              <div className="flex items-center gap-1">
                <div className="w-12 h-1 bg-muted/50 rounded-full overflow-hidden">
                  <div 
                    className={cn(
                      "h-full rounded-full transition-all",
                      isSelected ? "bg-primary-foreground/60" : "bg-primary/60"
                    )}
                    style={{ width: `${connectionStrength * 100}%` }}
                  />
                </div>
                <span className={cn(
                  "text-[10px] font-medium",
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

  return (
    <Collapsible open={isOpen} onOpenChange={setIsOpen}>
      <CollapsibleTrigger asChild>
        <Button
          variant="ghost"
          className="w-full justify-between px-3 py-2 h-auto"
        >
          <div className="flex items-center gap-2">
            <div
              className="w-3 h-3 rounded-full"
              style={{ backgroundColor: getEntityTypeColor(type) }}
            />
            <span className="text-sm font-medium">{type}</span>
          </div>
          <div className="flex items-center gap-2">
            <Badge variant="secondary" className="text-xs">
              {nodes.length}
            </Badge>
            {isOpen ? (
              <ChevronDown className="h-4 w-4" />
            ) : (
              <ChevronRight className="h-4 w-4" />
            )}
          </div>
        </Button>
      </CollapsibleTrigger>
      <CollapsibleContent className="pl-2 space-y-1">
        {nodes.map((node) => (
          <EntityItem
            key={node.id}
            node={node}
            isSelected={node.id === selectedNodeId}
            isFocused={false}
            onClick={() => onNodeClick(node.id)}
          />
        ))}
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
  const [isOpen, setIsOpen] = useState(true);
  const [searchQuery, setSearchQuery] = useState("");
  const [sortBy, setSortBy] = useState<SortOption>("name");
  const [sortDirection, setSortDirection] = useState<SortDirection>("asc");
  const [viewMode, setViewMode] = useState<"list" | "grouped">("grouped");
  const [focusedIndex, setFocusedIndex] = useState(-1);
  const listRef = useRef<HTMLDivElement>(null);

  const { nodes, selectedNodeId, selectNode, sigmaInstance } = useGraphStore();

  // Filter nodes by search query
  const filteredNodes = useMemo(() => {
    if (!searchQuery.trim()) return nodes;
    const query = searchQuery.toLowerCase();
    return nodes.filter(
      (node) =>
        node.label.toLowerCase().includes(query) ||
        node.node_type.toLowerCase().includes(query) ||
        node.description?.toLowerCase().includes(query)
    );
  }, [nodes, searchQuery]);

  // Sort nodes
  const sortedNodes = useMemo(() => {
    const sorted = [...filteredNodes].sort((a, b) => {
      let comparison = 0;
      switch (sortBy) {
        case "name":
          comparison = a.label.localeCompare(b.label);
          break;
        case "degree":
          comparison = (b.degree ?? 0) - (a.degree ?? 0);
          break;
        case "type":
          comparison = a.node_type.localeCompare(b.node_type);
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
      if (sigmaInstance) {
        const camera = sigmaInstance.getCamera();
        const nodePosition = sigmaInstance.getNodeDisplayData(nodeId);
        if (nodePosition) {
          camera.animate(
            { x: nodePosition.x, y: nodePosition.y, ratio: 0.3 },
            { duration: 500 }
          );
        }
      }
    },
    [selectNode, sigmaInstance]
  );

  // Toggle sort direction
  const toggleSortDirection = useCallback(() => {
    setSortDirection((prev) => (prev === "asc" ? "desc" : "asc"));
  }, []);

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
          <Search className="absolute left-2 top-1/2 -translate-y-1/2 h-3 w-3 text-muted-foreground" />
          <Input
            placeholder={t("graph.entityBrowser.search", "Search entities...")}
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            className="h-7 pl-7 text-xs bg-muted/30 border-muted focus:bg-background transition-colors"
          />
        </div>
      </div>

      {/* Sort Controls */}
      <div className="flex items-center gap-0.5 px-2 py-1.5 border-b shrink-0">
        <span className="text-[10px] text-muted-foreground mr-1">
          {t("common.sortBy", "Sort:")}
        </span>
        <Button
          variant={sortBy === "name" ? "secondary" : "ghost"}
          size="sm"
          className="h-5 text-[10px] px-1.5"
          onClick={() => setSortBy("name")}
        >
          {t("common.name", "Name")}
        </Button>
        <Button
          variant={sortBy === "degree" ? "secondary" : "ghost"}
          size="sm"
          className="h-5 text-[10px] px-1.5"
          onClick={() => setSortBy("degree")}
        >
          {t("graph.degree", "Degree")}
        </Button>
        <Button
          variant="ghost"
          size="icon"
          className="h-5 w-5 ml-auto"
          onClick={toggleSortDirection}
        >
          {sortDirection === "asc" ? (
            <SortAsc className="h-2.5 w-2.5" />
          ) : (
            <SortDesc className="h-2.5 w-2.5" />
          )}
        </Button>
      </div>

      {/* View Mode Toggle */}
      <div className="flex items-center gap-0.5 p-1.5 border-b shrink-0">
        <Button
          variant={viewMode === "grouped" ? "secondary" : "ghost"}
          size="sm"
          className="flex-1 h-6 text-[10px]"
          onClick={() => setViewMode("grouped")}
        >
          {t("graph.entityBrowser.grouped", "Grouped")}
        </Button>
        <Button
          variant={viewMode === "list" ? "secondary" : "ghost"}
          size="sm"
          className="flex-1 h-6 text-[10px]"
          onClick={() => setViewMode("list")}
        >
          {t("graph.entityBrowser.list", "List")}
        </Button>
      </div>

      {/* Entity List */}
      <ScrollArea className="flex-1">
        <div className="p-1.5">
          {filteredNodes.length === 0 ? (
            <div className="py-6 text-center">
              <Network className="h-6 w-6 mx-auto text-muted-foreground/50 mb-1.5" />
              <p className="text-xs text-muted-foreground">
                {searchQuery
                  ? t("graph.entityBrowser.noResults", "No entities found")
                  : t("graph.entityBrowser.empty", "No entities yet")}
              </p>
            </div>
          ) : viewMode === "grouped" ? (
            <div className="space-y-0.5">
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
          ) : (
            <div 
              ref={listRef}
              role="listbox"
              aria-label={t("graph.entityBrowser.entityList", "Entity list")}
              tabIndex={0}
              onKeyDown={handleListKeyDown}
              onFocus={() => {
                if (focusedIndex === -1 && sortedNodes.length > 0) {
                  setFocusedIndex(0);
                }
              }}
              className="space-y-1 outline-none focus-visible:ring-2 focus-visible:ring-primary focus-visible:ring-offset-2 rounded-md"
            >
              {sortedNodes.map((node, index) => (
                <EntityItem
                  key={node.id}
                  node={node}
                  isSelected={node.id === selectedNodeId}
                  isFocused={index === focusedIndex}
                  onClick={() => handleNodeClick(node.id)}
                  onKeyDown={handleListKeyDown}
                />
              ))}
            </div>
          )}
        </div>
      </ScrollArea>

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
