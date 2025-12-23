# Proposed Solutions & Improvements

> **Document Version:** 1.0  
> **Date:** 2024-12-23  
> **Purpose:** Concrete solutions for each identified gap

---

## Table of Contents

1. [Overview](#overview)
2. [Graph Visualization Solutions](#graph-visualization-solutions)
3. [Document Management Solutions](#document-management-solutions)
4. [Query Interface Solutions](#query-interface-solutions)
5. [Internationalization Solutions](#internationalization-solutions)
6. [Cross-References](#cross-references)

---

## Overview

This document provides actionable solutions for each gap identified in [001-gap-analysis.md](./001-gap-analysis.md). Each solution includes:

- Implementation approach
- Code examples where applicable
- Dependencies required
- Estimated effort

---

## Graph Visualization Solutions

### GAP-001: Node Drag & Drop

**Solution:** Add GraphEvents component with drag handling

**Dependencies:** None (uses existing @react-sigma/core)

**Implementation:**

Create/update `src/components/graph/graph-events.tsx`:

```tsx
"use client";

import { useGraphStore } from "@/stores/use-graph-store";
import { useRegisterEvents, useSigma } from "@react-sigma/core";
import { useEffect, useState } from "react";

export function GraphEvents() {
  const sigma = useSigma();
  const registerEvents = useRegisterEvents();
  const [draggedNode, setDraggedNode] = useState<string | null>(null);
  const { selectNode } = useGraphStore();

  useEffect(() => {
    registerEvents({
      downNode: (e) => {
        setDraggedNode(e.node);
        sigma.getGraph().setNodeAttribute(e.node, "highlighted", true);
      },

      mousemovebody: (e) => {
        if (!draggedNode) return;
        const pos = sigma.viewportToGraph(e);
        sigma.getGraph().setNodeAttribute(draggedNode, "x", pos.x);
        sigma.getGraph().setNodeAttribute(draggedNode, "y", pos.y);
        e.preventSigmaDefault();
        e.original.preventDefault();
        e.original.stopPropagation();
      },

      mouseup: () => {
        if (draggedNode) {
          sigma.getGraph().removeNodeAttribute(draggedNode, "highlighted");
          setDraggedNode(null);
        }
      },

      mousedown: (e) => {
        const mouseEvent = e.original as MouseEvent;
        if (mouseEvent.buttons !== 0 && !sigma.getCustomBBox()) {
          sigma.setCustomBBox(sigma.getBBox());
        }
      },

      clickNode: (e) => {
        selectNode(e.node);
      },

      rightClickNode: (e) => {
        e.preventSigmaDefault();
        // Emit event for context menu
      },
    });
  }, [registerEvents, sigma, draggedNode, selectNode]);

  return null;
}
```

**Store Updates:**

Add to `use-graph-store.ts`:

```tsx
interface GraphState {
  // ... existing state
  enableNodeDrag: boolean;
  setEnableNodeDrag: (enable: boolean) => void;
}
```

**Effort:** 2-3 hours

---

### GAP-002: Multiple Layout Algorithms

**Solution:** Install additional layout packages and create selector

**Dependencies:**

```bash
bun add @react-sigma/layout-circular @react-sigma/layout-forceatlas2 @react-sigma/layout-noverlap @react-sigma/layout-random
```

**Implementation:**

Update `src/components/graph/layout-control.tsx`:

```tsx
"use client";

import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { useSettingsStore } from "@/stores/use-settings-store";
import { useLayoutCircular } from "@react-sigma/layout-circular";
import { useLayoutForceAtlas2 } from "@react-sigma/layout-forceatlas2";
import { useLayoutNoverlap } from "@react-sigma/layout-noverlap";
import { useLayoutRandom } from "@react-sigma/layout-random";
import { LayoutGrid } from "lucide-react";
import { useCallback } from "react";
import { useTranslation } from "react-i18next";

type LayoutType = "force" | "circular" | "forceAtlas2" | "random" | "noverlap";

const layouts: { value: LayoutType; labelKey: string }[] = [
  { value: "force", labelKey: "graph.layout.force" },
  { value: "circular", labelKey: "graph.layout.circular" },
  { value: "forceAtlas2", labelKey: "graph.layout.forceAtlas2" },
  { value: "random", labelKey: "graph.layout.random" },
  { value: "noverlap", labelKey: "graph.layout.noverlap" },
];

export function LayoutControl() {
  const { t } = useTranslation();
  const { graphSettings, setGraphSettings } = useSettingsStore();

  const { assign: assignCircular } = useLayoutCircular();
  const { start: startForceAtlas2, stop: stopForceAtlas2 } =
    useLayoutForceAtlas2();
  const { assign: assignRandom } = useLayoutRandom();
  const { assign: assignNoverlap } = useLayoutNoverlap();

  const handleLayoutChange = useCallback(
    (layout: LayoutType) => {
      setGraphSettings({ layout });

      switch (layout) {
        case "circular":
          assignCircular();
          break;
        case "forceAtlas2":
          startForceAtlas2({ iterations: 50 });
          setTimeout(() => stopForceAtlas2(), 2000);
          break;
        case "random":
          assignRandom();
          break;
        case "noverlap":
          assignNoverlap({ maxIterations: 50 });
          break;
        default:
          // Force layout is handled by default sigma
          break;
      }
    },
    [
      assignCircular,
      startForceAtlas2,
      stopForceAtlas2,
      assignRandom,
      assignNoverlap,
      setGraphSettings,
    ]
  );

  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button variant="ghost" size="icon" title={t("graph.layout.title")}>
          <LayoutGrid className="h-4 w-4" />
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end">
        {layouts.map(({ value, labelKey }) => (
          <DropdownMenuItem
            key={value}
            onClick={() => handleLayoutChange(value)}
            className={graphSettings.layout === value ? "bg-muted" : ""}
          >
            {t(labelKey)}
          </DropdownMenuItem>
        ))}
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
```

**Effort:** 3-4 hours

---

### GAP-003: Fuzzy Node Search

**Solution:** Enhance graph search with MiniSearch

**Dependencies:** Already installed (minisearch)

**Implementation:**

Update `src/components/graph/graph-search.tsx`:

```tsx
"use client";

import { Button } from "@/components/ui/button";
import {
  Command,
  CommandEmpty,
  CommandGroup,
  CommandInput,
  CommandItem,
  CommandList,
} from "@/components/ui/command";
import {
  Popover,
  PopoverContent,
  PopoverTrigger,
} from "@/components/ui/popover";
import { useGraphStore } from "@/stores/use-graph-store";
import MiniSearch from "minisearch";
import { Search } from "lucide-react";
import { useCallback, useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";

interface SearchResult {
  id: string;
  label: string;
  type: string;
  score: number;
}

export function GraphSearch() {
  const { t } = useTranslation();
  const { nodes, selectNode, sigmaInstance } = useGraphStore();
  const [open, setOpen] = useState(false);
  const [query, setQuery] = useState("");
  const [results, setResults] = useState<SearchResult[]>([]);

  // Build search index
  const miniSearch = useMemo(() => {
    const search = new MiniSearch({
      fields: ["label", "type", "description"],
      storeFields: ["label", "type"],
      searchOptions: {
        boost: { label: 2 },
        fuzzy: 0.2,
        prefix: true,
      },
    });

    search.addAll(
      nodes.map((node) => ({
        id: node.id,
        label: node.label,
        type: node.type || "Entity",
        description: node.description || "",
      }))
    );

    return search;
  }, [nodes]);

  // Search on query change
  useEffect(() => {
    if (query.trim()) {
      const searchResults = miniSearch.search(query).slice(0, 10);
      setResults(
        searchResults.map((r) => ({
          id: r.id,
          label: r.label,
          type: r.type,
          score: r.score,
        }))
      );
    } else {
      setResults([]);
    }
  }, [query, miniSearch]);

  const handleSelect = useCallback(
    (nodeId: string) => {
      selectNode(nodeId);

      // Focus camera on selected node
      if (sigmaInstance) {
        const graph = sigmaInstance.getGraph();
        const nodeData = graph.getNodeAttributes(nodeId);
        if (nodeData) {
          const camera = sigmaInstance.getCamera();
          camera.animate(
            { x: nodeData.x, y: nodeData.y, ratio: 0.3 },
            { duration: 500 }
          );
        }
      }

      setOpen(false);
      setQuery("");
    },
    [selectNode, sigmaInstance]
  );

  return (
    <Popover open={open} onOpenChange={setOpen}>
      <PopoverTrigger asChild>
        <Button variant="ghost" size="icon" title={t("graph.search.title")}>
          <Search className="h-4 w-4" />
        </Button>
      </PopoverTrigger>
      <PopoverContent className="w-80 p-0" align="end">
        <Command>
          <CommandInput
            placeholder={t("graph.search.placeholder")}
            value={query}
            onValueChange={setQuery}
          />
          <CommandList>
            <CommandEmpty>{t("graph.search.noResults")}</CommandEmpty>
            <CommandGroup heading={t("graph.search.entities")}>
              {results.map((result) => (
                <CommandItem
                  key={result.id}
                  value={result.id}
                  onSelect={() => handleSelect(result.id)}
                >
                  <div className="flex flex-col">
                    <span className="font-medium">{result.label}</span>
                    <span className="text-xs text-muted-foreground">
                      {result.type}
                    </span>
                  </div>
                </CommandItem>
              ))}
            </CommandGroup>
          </CommandList>
        </Command>
      </PopoverContent>
    </Popover>
  );
}
```

**Effort:** 2-3 hours

---

### GAP-009: Full-Screen Graph Mode

**Solution:** Add fullscreen toggle using browser API

**Implementation:**

Create `src/components/graph/fullscreen-control.tsx`:

```tsx
"use client";

import { Button } from "@/components/ui/button";
import { Maximize2, Minimize2 } from "lucide-react";
import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

export function FullscreenControl() {
  const { t } = useTranslation();
  const [isFullscreen, setIsFullscreen] = useState(false);

  useEffect(() => {
    const handleFullscreenChange = () => {
      setIsFullscreen(!!document.fullscreenElement);
    };

    document.addEventListener("fullscreenchange", handleFullscreenChange);
    return () =>
      document.removeEventListener("fullscreenchange", handleFullscreenChange);
  }, []);

  const toggleFullscreen = useCallback(async () => {
    const graphContainer = document.querySelector("[data-graph-container]");

    if (!graphContainer) return;

    try {
      if (!document.fullscreenElement) {
        await graphContainer.requestFullscreen();
      } else {
        await document.exitFullscreen();
      }
    } catch (err) {
      console.error("Fullscreen error:", err);
    }
  }, []);

  return (
    <Button
      variant="ghost"
      size="icon"
      onClick={toggleFullscreen}
      title={t(isFullscreen ? "graph.exitFullscreen" : "graph.enterFullscreen")}
    >
      {isFullscreen ? (
        <Minimize2 className="h-4 w-4" />
      ) : (
        <Maximize2 className="h-4 w-4" />
      )}
    </Button>
  );
}
```

**Effort:** 1 hour

---

### GAP-010: Entity Merge Functionality

**Solution:** Create merge dialog with API integration

**Implementation:**

Create `src/components/graph/merge-dialog.tsx`:

```tsx
"use client";

import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from "@/components/ui/alert-dialog";
import { mergeEntities } from "@/lib/api/edgequake";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";

interface MergeDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  sourceEntityId: string;
  targetEntityId: string;
  sourceLabel: string;
  targetLabel: string;
}

export function MergeDialog({
  open,
  onOpenChange,
  sourceEntityId,
  targetEntityId,
  sourceLabel,
  targetLabel,
}: MergeDialogProps) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();

  const mergeMutation = useMutation({
    mutationFn: () =>
      mergeEntities({
        source_entity_id: sourceEntityId,
        target_entity_id: targetEntityId,
      }),
    onSuccess: () => {
      toast.success(
        t("graph.merge.success", { source: sourceLabel, target: targetLabel })
      );
      queryClient.invalidateQueries({ queryKey: ["graph"] });
      onOpenChange(false);
    },
    onError: (error) => {
      toast.error(
        t("graph.merge.error", {
          error: error instanceof Error ? error.message : "Unknown error",
        })
      );
    },
  });

  return (
    <AlertDialog open={open} onOpenChange={onOpenChange}>
      <AlertDialogContent>
        <AlertDialogHeader>
          <AlertDialogTitle>{t("graph.merge.title")}</AlertDialogTitle>
          <AlertDialogDescription>
            {t("graph.merge.description", {
              source: sourceLabel,
              target: targetLabel,
            })}
          </AlertDialogDescription>
        </AlertDialogHeader>
        <AlertDialogFooter>
          <AlertDialogCancel>{t("common.cancel")}</AlertDialogCancel>
          <AlertDialogAction
            onClick={() => mergeMutation.mutate()}
            disabled={mergeMutation.isPending}
          >
            {mergeMutation.isPending ? t("common.merging") : t("common.merge")}
          </AlertDialogAction>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  );
}
```

**Effort:** 2-3 hours

---

## Document Management Solutions

### GAP-006: URL State Synchronization

**Solution:** Create useUrlState hook

**Implementation:**

Create `src/hooks/use-url-state.ts`:

```tsx
"use client";

import { usePathname, useRouter, useSearchParams } from "next/navigation";
import { useCallback, useMemo } from "react";

export function useUrlState<
  T extends Record<string, string | number | undefined>
>(defaults: T): [T, (updates: Partial<T>) => void] {
  const router = useRouter();
  const pathname = usePathname();
  const searchParams = useSearchParams();

  const state = useMemo(() => {
    const result = { ...defaults };

    for (const key of Object.keys(defaults)) {
      const value = searchParams.get(key);
      if (value !== null) {
        if (typeof defaults[key] === "number") {
          result[key as keyof T] = parseInt(value, 10) as T[keyof T];
        } else {
          result[key as keyof T] = value as T[keyof T];
        }
      }
    }

    return result;
  }, [searchParams, defaults]);

  const setState = useCallback(
    (updates: Partial<T>) => {
      const params = new URLSearchParams(searchParams.toString());

      for (const [key, value] of Object.entries(updates)) {
        if (value === undefined || value === defaults[key]) {
          params.delete(key);
        } else {
          params.set(key, String(value));
        }
      }

      const query = params.toString();
      router.push(`${pathname}${query ? `?${query}` : ""}`, { scroll: false });
    },
    [router, pathname, searchParams, defaults]
  );

  return [state, setState];
}
```

**Usage in DocumentManager:**

```tsx
const [urlState, setUrlState] = useUrlState({
  page: 1,
  pageSize: 20,
  status: "all",
  sortBy: "created_at",
  sortDir: "desc",
});
```

**Effort:** 2-3 hours

---

## Query Interface Solutions

### GAP-007: Query Mode Prefix Parsing

**Solution:** Add prefix parsing before submission

**Implementation:**

Add to `src/components/query/query-interface.tsx`:

```tsx
const ALLOWED_MODES: QueryMode[] = [
  "naive",
  "local",
  "global",
  "hybrid",
  "mix",
  "bypass",
];

function parseQueryWithMode(input: string): {
  query: string;
  mode?: QueryMode;
} {
  const prefixMatch = input.match(/^\/(\w+)\s+([\s\S]+)/);

  if (prefixMatch) {
    const modeCandidate = prefixMatch[1].toLowerCase() as QueryMode;
    if (ALLOWED_MODES.includes(modeCandidate)) {
      return { query: prefixMatch[2], mode: modeCandidate };
    }
  }

  return { query: input };
}

// In handleSubmit:
const { query: actualQuery, mode: modeOverride } = parseQueryWithMode(
  input.trim()
);
const effectiveMode = modeOverride || querySettings.mode;

// Show mode hint in input placeholder
const placeholder =
  t("query.placeholder") +
  " " +
  t("query.modeHint", {
    modes: "/naive, /local, /global, /hybrid, /mix",
  });
```

**Effort:** 1 hour

---

### GAP-008: Thinking Time Display

**Solution:** Track thinking start time and calculate duration

**Implementation:**

Update `src/components/query/query-interface.tsx`:

```tsx
interface Message {
  id: string;
  role: "user" | "assistant";
  content: string;
  mode?: QueryMode;
  tokensUsed?: number;
  durationMs?: number;
  thinkingTimeMs?: number; // Add thinking time
  context?: QueryContext;
}

const handleStreamQuery = async (queryText: string) => {
  const thinkingStartTime = Date.now();
  let thinkingEndTime: number | null = null;
  let isInThinkingBlock = false;

  // ... existing streaming code

  for await (const chunk of queryStream(/* ... */)) {
    if (chunk.type === "token" && chunk.content) {
      fullContent += chunk.content;

      // Track thinking time
      if (fullContent.includes("<think>") && !isInThinkingBlock) {
        isInThinkingBlock = true;
      }
      if (
        fullContent.includes("</think>") &&
        isInThinkingBlock &&
        !thinkingEndTime
      ) {
        thinkingEndTime = Date.now();
        isInThinkingBlock = false;
      }

      setStreamingContent(fullContent);
    }
  }

  // Calculate thinking time
  const thinkingTimeMs = thinkingEndTime
    ? thinkingEndTime - thinkingStartTime
    : undefined;

  // Add to message
  setMessages((prev) => [
    ...prev,
    {
      // ... existing fields
      thinkingTimeMs,
    },
  ]);
};
```

**Display in message:**

```tsx
{
  message.thinkingTimeMs && (
    <span className="text-xs text-muted-foreground">
      {t("query.thinkingTime", {
        time: (message.thinkingTimeMs / 1000).toFixed(1),
      })}
      s
    </span>
  );
}
```

**Effort:** 2 hours

---

## Internationalization Solutions

### GAP-005: Translation Coverage

**Solution:** Expand translation files

**Implementation:**

Add missing keys to `src/locales/en.json`:

```json
{
  "graph": {
    "layout": {
      "title": "Graph Layout",
      "force": "Force-directed",
      "circular": "Circular",
      "forceAtlas2": "ForceAtlas2",
      "random": "Random",
      "noverlap": "No Overlap"
    },
    "search": {
      "title": "Search Nodes",
      "placeholder": "Search entities...",
      "noResults": "No entities found",
      "entities": "Entities"
    },
    "merge": {
      "title": "Merge Entities",
      "description": "Merge \"{{source}}\" into \"{{target}}\"? This action cannot be undone.",
      "success": "Successfully merged {{source}} into {{target}}",
      "error": "Failed to merge: {{error}}"
    },
    "enterFullscreen": "Enter Fullscreen",
    "exitFullscreen": "Exit Fullscreen",
    "dragEnabled": "Drag nodes enabled",
    "dragDisabled": "Drag nodes disabled"
  },
  "documents": {
    "scan": "Scan for new documents",
    "scanning": "Scanning...",
    "scanComplete": "Scan complete. Found {{count}} new documents.",
    "resetStatus": "Reset Status",
    "batchSelect": "Select all",
    "batchDeselect": "Deselect all"
  },
  "query": {
    "modeHint": "Tip: Use {{modes}} to quickly switch modes",
    "thinkingTime": "Thought for {{time}}s"
  },
  "common": {
    "merge": "Merge",
    "merging": "Merging...",
    "platform": "Knowledge Graph RAG Platform"
  }
}
```

Also add corresponding keys to `zh.json` and `fr.json`.

**Effort:** 3-4 hours for all three languages

---

## Implementation Checklist

### Phase 1 - High Priority (Week 1-2)

- [ ] GAP-001: Node Drag & Drop
- [ ] GAP-002: Multiple Layout Algorithms
- [ ] GAP-003: Fuzzy Node Search
- [ ] GAP-004: Pipeline Status Dialog Enhancement

### Phase 2 - Medium Priority (Week 3-4)

- [ ] GAP-005: Translation Coverage
- [ ] GAP-006: URL State Sync
- [ ] GAP-007: Query Mode Prefix
- [ ] GAP-008: Thinking Time Display
- [ ] GAP-009: Full-Screen Graph
- [ ] GAP-010: Entity Merge

### Phase 3 - Polish (Week 5)

- [ ] Graph Legend
- [ ] Graph Settings Panel
- [ ] Inline Property Editing
- [ ] User Prompt History
- [ ] Copy Response to Clipboard

---

## Cross-References

- **Gap Analysis:** [001-gap-analysis.md](./001-gap-analysis.md)
- **Prioritization:** [004-prioritization-roadmap.md](./004-prioritization-roadmap.md)
- **UX Plan:** [005-ux-improvements.md](./005-ux-improvements.md)
- **Performance:** [006-performance-strategy.md](./006-performance-strategy.md)
