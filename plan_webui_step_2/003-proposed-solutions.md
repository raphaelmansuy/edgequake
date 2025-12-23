# Proposed Solutions & Improvements

> **Document Version:** 1.0  
> **Date:** 2024-12-23  
> **Purpose:** Concrete solutions for each identified gap

---

## Table of Contents

1. [Solution Overview](#solution-overview)
2. [Critical Priority Solutions](#critical-priority-solutions)
3. [High Priority Solutions](#high-priority-solutions)
4. [Medium Priority Solutions](#medium-priority-solutions)
5. [Low Priority Solutions](#low-priority-solutions)
6. [Architecture Recommendations](#architecture-recommendations)

---

## Solution Overview

This document provides implementation guidance for each gap identified in the [Gap Analysis](./002-gap-analysis.md). Solutions are organized by priority and include:

- Implementation approach
- Code snippets where applicable
- Required dependencies
- File locations
- Effort estimates

### Effort Scale

- **XS:** < 2 hours
- **S:** 2-4 hours
- **M:** 4-8 hours (1 day)
- **L:** 8-16 hours (2 days)
- **XL:** 16-40 hours (1 week)

---

## Critical Priority Solutions

### SOL-001: Internationalization (i18n)

**Gap Reference:** GAP-001  
**Effort:** L (2 days)

#### Solution Approach

Implement i18next with react-i18next for Next.js App Router.

#### Step 1: Install Dependencies

```bash
bun add i18next react-i18next i18next-browser-languagedetector
```

#### Step 2: Create i18n Configuration

**File:** `src/lib/i18n.ts`

```typescript
import i18n from "i18next";
import { initReactI18next } from "react-i18next";
import LanguageDetector from "i18next-browser-languagedetector";

import en from "@/locales/en.json";
import zh from "@/locales/zh.json";
import fr from "@/locales/fr.json";

i18n
  .use(LanguageDetector)
  .use(initReactI18next)
  .init({
    resources: {
      en: { translation: en },
      zh: { translation: zh },
      fr: { translation: fr },
    },
    fallbackLng: "en",
    interpolation: {
      escapeValue: false,
    },
  });

export default i18n;
```

#### Step 3: Create Locale Files

**File:** `src/locales/en.json`

```json
{
  "common": {
    "cancel": "Cancel",
    "save": "Save",
    "delete": "Delete",
    "refresh": "Refresh",
    "loading": "Loading..."
  },
  "header": {
    "documents": "Documents",
    "knowledgeGraph": "Knowledge Graph",
    "query": "Query",
    "settings": "Settings"
  },
  "documents": {
    "title": "Documents",
    "upload": "Upload",
    "clearAll": "Clear All",
    "noDocuments": "No documents yet"
  },
  "graph": {
    "title": "Knowledge Graph",
    "zoomIn": "Zoom In",
    "zoomOut": "Zoom Out",
    "reset": "Reset View"
  },
  "query": {
    "placeholder": "Ask a question...",
    "submit": "Send",
    "modes": {
      "local": "Local",
      "global": "Global",
      "hybrid": "Hybrid",
      "naive": "Naive"
    }
  }
}
```

#### Step 4: Create I18n Provider

**File:** `src/providers/i18n-provider.tsx`

```tsx
"use client";

import { useEffect, useState } from "react";
import "@/lib/i18n";

export function I18nProvider({ children }: { children: React.ReactNode }) {
  const [isHydrated, setIsHydrated] = useState(false);

  useEffect(() => {
    setIsHydrated(true);
  }, []);

  if (!isHydrated) {
    return null;
  }

  return <>{children}</>;
}
```

#### Step 5: Create Language Selector

**File:** `src/components/shared/language-selector.tsx`

```tsx
"use client";

import { useTranslation } from "react-i18next";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";

const languages = [
  { code: "en", name: "English" },
  { code: "zh", name: "中文" },
  { code: "fr", name: "Français" },
];

export function LanguageSelector() {
  const { i18n } = useTranslation();

  return (
    <Select value={i18n.language} onValueChange={i18n.changeLanguage}>
      <SelectTrigger className="w-32">
        <SelectValue />
      </SelectTrigger>
      <SelectContent>
        {languages.map((lang) => (
          <SelectItem key={lang.code} value={lang.code}>
            {lang.name}
          </SelectItem>
        ))}
      </SelectContent>
    </Select>
  );
}
```

#### Step 6: Update Components

Replace hardcoded strings with `t()` calls:

```tsx
// Before
<Button>Upload Documents</Button>;

// After
import { useTranslation } from "react-i18next";

function Component() {
  const { t } = useTranslation();
  return <Button>{t("documents.upload")}</Button>;
}
```

---

### SOL-002: Graph Node Drag & Drop

**Gap Reference:** GAP-002  
**Effort:** S (4 hours)

#### Solution Approach

Port the GraphEvents component from LightRAG.

**File:** `src/components/graph/graph-events.tsx`

```tsx
"use client";

import { useEffect, useState } from "react";
import { useRegisterEvents, useSigma } from "@react-sigma/core";

export function GraphEvents() {
  const registerEvents = useRegisterEvents();
  const sigma = useSigma();
  const [draggedNode, setDraggedNode] = useState<string | null>(null);

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
    });
  }, [registerEvents, sigma, draggedNode]);

  return null;
}
```

#### Integration

Add to `GraphRenderer` component inside `SigmaContainer`:

```tsx
<SigmaContainer settings={sigmaSettings}>
  <GraphEvents />
  {/* ... other components */}
</SigmaContainer>
```

---

## High Priority Solutions

### SOL-003: Graph Layout Algorithms

**Gap Reference:** GAP-003  
**Effort:** M (1 day)

#### Step 1: Install Dependencies

```bash
bun add @react-sigma/layout-circular @react-sigma/layout-circlepack \
  @react-sigma/layout-random @react-sigma/layout-noverlap
```

#### Step 2: Create Layout Control Component

**File:** `src/components/graph/layout-control.tsx`

```tsx
"use client";

import { useLayoutCircular } from "@react-sigma/layout-circular";
import { useLayoutCirclepack } from "@react-sigma/layout-circlepack";
import { useLayoutForceAtlas2 } from "@react-sigma/layout-forceatlas2";
import { useLayoutRandom } from "@react-sigma/layout-random";
import { useLayoutNoverlap } from "@react-sigma/layout-noverlap";
import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { LayoutGrid } from "lucide-react";

type LayoutType = "force" | "circular" | "circlepack" | "random";

export function LayoutControl() {
  const { assign: assignCircular } = useLayoutCircular();
  const { assign: assignCirclepack } = useLayoutCirclepack();
  const { assign: assignForce } = useLayoutForceAtlas2();
  const { assign: assignRandom } = useLayoutRandom();
  const { assign: assignNoverlap } = useLayoutNoverlap();

  const applyLayout = (layout: LayoutType) => {
    switch (layout) {
      case "force":
        assignForce();
        break;
      case "circular":
        assignCircular();
        break;
      case "circlepack":
        assignCirclepack();
        break;
      case "random":
        assignRandom();
        assignNoverlap(); // Apply noverlap after random
        break;
    }
  };

  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button variant="ghost" size="icon" title="Change Layout">
          <LayoutGrid className="h-4 w-4" />
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent>
        <DropdownMenuItem onClick={() => applyLayout("force")}>
          Force Atlas
        </DropdownMenuItem>
        <DropdownMenuItem onClick={() => applyLayout("circular")}>
          Circular
        </DropdownMenuItem>
        <DropdownMenuItem onClick={() => applyLayout("circlepack")}>
          Circle Pack
        </DropdownMenuItem>
        <DropdownMenuItem onClick={() => applyLayout("random")}>
          Random
        </DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
```

---

### SOL-004: Graph Node Search

**Gap Reference:** GAP-004  
**Effort:** M (1 day)

#### Step 1: Install MiniSearch

```bash
bun add minisearch
```

#### Step 2: Create Graph Search Component

**File:** `src/components/graph/graph-search.tsx`

```tsx
"use client";

import { useCallback, useEffect, useMemo, useState } from "react";
import { useSigma } from "@react-sigma/core";
import MiniSearch from "minisearch";
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
import { Button } from "@/components/ui/button";
import { Search } from "lucide-react";

interface SearchResult {
  id: string;
  label: string;
  score: number;
}

export function GraphSearch({
  onSelect,
}: {
  onSelect?: (nodeId: string) => void;
}) {
  const sigma = useSigma();
  const [open, setOpen] = useState(false);
  const [query, setQuery] = useState("");
  const [results, setResults] = useState<SearchResult[]>([]);

  // Create search index
  const searchEngine = useMemo(() => {
    const graph = sigma.getGraph();
    if (!graph || graph.nodes().length === 0) return null;

    const miniSearch = new MiniSearch({
      idField: "id",
      fields: ["label"],
      searchOptions: {
        prefix: true,
        fuzzy: 0.2,
        boost: { label: 2 },
      },
    });

    const documents = graph.nodes().map((id) => ({
      id,
      label: graph.getNodeAttribute(id, "label") || id,
    }));

    miniSearch.addAll(documents);
    return miniSearch;
  }, [sigma]);

  // Search handler
  useEffect(() => {
    if (!searchEngine || !query.trim()) {
      setResults([]);
      return;
    }

    const searchResults = searchEngine.search(query).slice(0, 10);
    setResults(
      searchResults.map((r) => ({
        id: r.id,
        label: sigma.getGraph().getNodeAttribute(r.id, "label") || r.id,
        score: r.score,
      }))
    );
  }, [query, searchEngine, sigma]);

  const handleSelect = useCallback(
    (nodeId: string) => {
      setOpen(false);
      setQuery("");

      // Focus camera on node
      const graph = sigma.getGraph();
      const nodeData = graph.getNodeAttributes(nodeId);
      if (nodeData) {
        sigma
          .getCamera()
          .animate(
            { x: nodeData.x, y: nodeData.y, ratio: 0.5 },
            { duration: 500 }
          );
      }

      onSelect?.(nodeId);
    },
    [sigma, onSelect]
  );

  return (
    <Popover open={open} onOpenChange={setOpen}>
      <PopoverTrigger asChild>
        <Button variant="outline" size="sm" className="gap-2">
          <Search className="h-4 w-4" />
          Search Nodes
        </Button>
      </PopoverTrigger>
      <PopoverContent className="w-80 p-0" align="start">
        <Command>
          <CommandInput
            placeholder="Search nodes..."
            value={query}
            onValueChange={setQuery}
          />
          <CommandList>
            <CommandEmpty>No nodes found.</CommandEmpty>
            <CommandGroup>
              {results.map((result) => (
                <CommandItem
                  key={result.id}
                  value={result.id}
                  onSelect={() => handleSelect(result.id)}
                >
                  <span>{result.label}</span>
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

---

### SOL-005: Document Pagination

**Gap Reference:** GAP-005  
**Effort:** M (1 day)

#### Solution Approach

Implement pagination with URL state synchronization.

**File:** `src/components/documents/pagination-controls.tsx`

```tsx
"use client";

import { Button } from "@/components/ui/button";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { ChevronLeft, ChevronRight } from "lucide-react";

interface PaginationControlsProps {
  currentPage: number;
  totalPages: number;
  pageSize: number;
  onPageChange: (page: number) => void;
  onPageSizeChange: (size: number) => void;
}

export function PaginationControls({
  currentPage,
  totalPages,
  pageSize,
  onPageChange,
  onPageSizeChange,
}: PaginationControlsProps) {
  return (
    <div className="flex items-center justify-between px-2 py-4">
      <div className="flex items-center gap-2">
        <span className="text-sm text-muted-foreground">Rows per page:</span>
        <Select
          value={String(pageSize)}
          onValueChange={(v) => onPageSizeChange(Number(v))}
        >
          <SelectTrigger className="w-20">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            {[10, 20, 50, 100].map((size) => (
              <SelectItem key={size} value={String(size)}>
                {size}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      </div>

      <div className="flex items-center gap-2">
        <span className="text-sm text-muted-foreground">
          Page {currentPage} of {totalPages}
        </span>
        <Button
          variant="outline"
          size="icon"
          onClick={() => onPageChange(currentPage - 1)}
          disabled={currentPage <= 1}
        >
          <ChevronLeft className="h-4 w-4" />
        </Button>
        <Button
          variant="outline"
          size="icon"
          onClick={() => onPageChange(currentPage + 1)}
          disabled={currentPage >= totalPages}
        >
          <ChevronRight className="h-4 w-4" />
        </Button>
      </div>
    </div>
  );
}
```

#### URL State Hook

**File:** `src/hooks/use-url-state.ts`

```typescript
"use client";

import { usePathname, useRouter, useSearchParams } from "next/navigation";
import { useCallback } from "react";

export function useUrlState<
  T extends Record<string, string | number | undefined>
>(defaultValues: T) {
  const router = useRouter();
  const pathname = usePathname();
  const searchParams = useSearchParams();

  const getState = useCallback((): T => {
    const state = { ...defaultValues };
    for (const key of Object.keys(defaultValues)) {
      const value = searchParams.get(key);
      if (value !== null) {
        state[key as keyof T] = (
          typeof defaultValues[key] === "number" ? Number(value) : value
        ) as T[keyof T];
      }
    }
    return state;
  }, [searchParams, defaultValues]);

  const setState = useCallback(
    (updates: Partial<T>) => {
      const params = new URLSearchParams(searchParams.toString());
      for (const [key, value] of Object.entries(updates)) {
        if (value === undefined || value === defaultValues[key]) {
          params.delete(key);
        } else {
          params.set(key, String(value));
        }
      }
      router.push(`${pathname}?${params.toString()}`, { scroll: false });
    },
    [router, pathname, searchParams, defaultValues]
  );

  return { state: getState(), setState };
}
```

---

### SOL-006: Document Filtering & Sorting

**Gap Reference:** GAP-006  
**Effort:** S (4 hours)

**File:** `src/components/documents/document-filters.tsx`

```tsx
"use client";

import { Button } from "@/components/ui/button";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { ArrowDown, ArrowUp } from "lucide-react";

type DocStatus = "all" | "pending" | "processing" | "completed" | "failed";
type SortField = "created_at" | "updated_at" | "title";
type SortDirection = "asc" | "desc";

interface DocumentFiltersProps {
  status: DocStatus;
  sortField: SortField;
  sortDirection: SortDirection;
  onStatusChange: (status: DocStatus) => void;
  onSortChange: (field: SortField, direction: SortDirection) => void;
}

export function DocumentFilters({
  status,
  sortField,
  sortDirection,
  onStatusChange,
  onSortChange,
}: DocumentFiltersProps) {
  const toggleSort = (field: SortField) => {
    if (sortField === field) {
      onSortChange(field, sortDirection === "asc" ? "desc" : "asc");
    } else {
      onSortChange(field, "desc");
    }
  };

  return (
    <div className="flex items-center gap-4 mb-4">
      <Select
        value={status}
        onValueChange={(v) => onStatusChange(v as DocStatus)}
      >
        <SelectTrigger className="w-40">
          <SelectValue placeholder="Filter by status" />
        </SelectTrigger>
        <SelectContent>
          <SelectItem value="all">All Status</SelectItem>
          <SelectItem value="pending">Pending</SelectItem>
          <SelectItem value="processing">Processing</SelectItem>
          <SelectItem value="completed">Completed</SelectItem>
          <SelectItem value="failed">Failed</SelectItem>
        </SelectContent>
      </Select>

      <div className="flex items-center gap-1">
        <span className="text-sm text-muted-foreground">Sort by:</span>
        <Button
          variant={sortField === "created_at" ? "secondary" : "ghost"}
          size="sm"
          onClick={() => toggleSort("created_at")}
        >
          Created
          {sortField === "created_at" &&
            (sortDirection === "asc" ? (
              <ArrowUp className="ml-1 h-3 w-3" />
            ) : (
              <ArrowDown className="ml-1 h-3 w-3" />
            ))}
        </Button>
        <Button
          variant={sortField === "updated_at" ? "secondary" : "ghost"}
          size="sm"
          onClick={() => toggleSort("updated_at")}
        >
          Updated
          {sortField === "updated_at" &&
            (sortDirection === "asc" ? (
              <ArrowUp className="ml-1 h-3 w-3" />
            ) : (
              <ArrowDown className="ml-1 h-3 w-3" />
            ))}
        </Button>
      </div>
    </div>
  );
}
```

---

### SOL-007: Pipeline Status Monitoring

**Gap Reference:** GAP-007  
**Effort:** M (1 day)

**File:** `src/components/documents/pipeline-status-dialog.tsx`

```tsx
"use client";

import { useQuery } from "@tanstack/react-query";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { Progress } from "@/components/ui/progress";
import { Activity, Loader2, XCircle } from "lucide-react";
import { getPipelineStatus, cancelPipeline } from "@/lib/api/edgequake";

interface PipelineStatusDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

export function PipelineStatusDialog({
  open,
  onOpenChange,
}: PipelineStatusDialogProps) {
  const { data, isLoading } = useQuery({
    queryKey: ["pipeline-status"],
    queryFn: getPipelineStatus,
    refetchInterval: open ? 2000 : false,
  });

  const handleCancel = async () => {
    try {
      await cancelPipeline();
    } catch (error) {
      console.error("Failed to cancel pipeline:", error);
    }
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <Activity className="h-5 w-5" />
            Pipeline Status
          </DialogTitle>
        </DialogHeader>

        {isLoading ? (
          <div className="flex items-center justify-center py-8">
            <Loader2 className="h-8 w-8 animate-spin" />
          </div>
        ) : data?.is_busy ? (
          <div className="space-y-4">
            <div>
              <p className="text-sm font-medium">Job: {data.job_name}</p>
              <p className="text-sm text-muted-foreground">
                Started: {new Date(data.start_time).toLocaleTimeString()}
              </p>
            </div>

            <div className="space-y-2">
              <div className="flex justify-between text-sm">
                <span>Progress</span>
                <span>{data.progress}%</span>
              </div>
              <Progress value={data.progress} />
            </div>

            {data.messages && data.messages.length > 0 && (
              <div className="space-y-1">
                <p className="text-sm font-medium">Messages:</p>
                <div className="max-h-32 overflow-y-auto rounded bg-muted p-2">
                  {data.messages.map((msg: string, i: number) => (
                    <p key={i} className="text-xs">
                      {msg}
                    </p>
                  ))}
                </div>
              </div>
            )}

            <Button
              variant="destructive"
              onClick={handleCancel}
              className="w-full"
            >
              <XCircle className="mr-2 h-4 w-4" />
              Cancel Pipeline
            </Button>
          </div>
        ) : (
          <div className="py-8 text-center text-muted-foreground">
            Pipeline is idle
          </div>
        )}
      </DialogContent>
    </Dialog>
  );
}
```

---

### SOL-008: LaTeX Rendering

**Gap Reference:** GAP-008  
**Effort:** M (1 day)

#### Step 1: Install Dependencies

```bash
bun add katex remark-math rehype-katex
```

#### Step 2: Update Markdown Component

**File:** `src/components/query/markdown-renderer.tsx`

```tsx
"use client";

import { useEffect, useState } from "react";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import remarkMath from "remark-math";
import rehypeHighlight from "rehype-highlight";
import "katex/dist/katex.min.css";

export function MarkdownRenderer({ content }: { content: string }) {
  const [rehypeKatex, setRehypeKatex] = useState<any>(null);

  useEffect(() => {
    import("rehype-katex").then((module) => {
      setRehypeKatex(() => module.default);
    });
  }, []);

  return (
    <ReactMarkdown
      remarkPlugins={[remarkGfm, remarkMath]}
      rehypePlugins={[rehypeHighlight, ...(rehypeKatex ? [rehypeKatex] : [])]}
    >
      {content}
    </ReactMarkdown>
  );
}
```

---

### SOL-009: Mermaid Diagram Rendering

**Gap Reference:** GAP-009  
**Effort:** M (1 day)

#### Step 1: Install Mermaid

```bash
bun add mermaid
```

#### Step 2: Create Mermaid Component

**File:** `src/components/query/mermaid-diagram.tsx`

```tsx
"use client";

import { useEffect, useRef, useState } from "react";
import mermaid from "mermaid";
import { useTheme } from "next-themes";

interface MermaidDiagramProps {
  code: string;
}

export function MermaidDiagram({ code }: MermaidDiagramProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const [svg, setSvg] = useState<string>("");
  const { resolvedTheme } = useTheme();

  useEffect(() => {
    mermaid.initialize({
      startOnLoad: false,
      theme: resolvedTheme === "dark" ? "dark" : "default",
    });
  }, [resolvedTheme]);

  useEffect(() => {
    const renderDiagram = async () => {
      try {
        const id = `mermaid-${Math.random().toString(36).substr(2, 9)}`;
        const { svg } = await mermaid.render(id, code);
        setSvg(svg);
      } catch (error) {
        console.error("Failed to render mermaid diagram:", error);
        setSvg(`<pre class="text-red-500">Failed to render diagram</pre>`);
      }
    };

    if (code) {
      renderDiagram();
    }
  }, [code]);

  return (
    <div
      ref={containerRef}
      className="my-4 overflow-x-auto"
      dangerouslySetInnerHTML={{ __html: svg }}
    />
  );
}
```

---

### SOL-010: Chain-of-Thought Display

**Gap Reference:** GAP-010  
**Effort:** M (1 day)

**File:** `src/components/query/thinking-display.tsx`

```tsx
"use client";

import { useState } from "react";
import { ChevronDown, Loader2 } from "lucide-react";
import { cn } from "@/lib/utils";

interface ThinkingDisplayProps {
  isThinking: boolean;
  thinkingContent?: string;
  thinkingTime?: number | null;
}

export function ThinkingDisplay({
  isThinking,
  thinkingContent,
  thinkingTime,
}: ThinkingDisplayProps) {
  const [isExpanded, setIsExpanded] = useState(false);

  if (!isThinking && !thinkingTime) {
    return null;
  }

  return (
    <div className="mb-2">
      <button
        className="flex items-center text-sm text-muted-foreground hover:text-foreground transition-colors"
        onClick={() => setIsExpanded(!isExpanded)}
      >
        {isThinking ? (
          <>
            <Loader2 className="mr-2 h-4 w-4 animate-spin" />
            <span>Thinking...</span>
          </>
        ) : (
          <span>Thought for {thinkingTime}s</span>
        )}
        {thinkingContent && (
          <ChevronDown
            className={cn(
              "ml-2 h-4 w-4 transition-transform",
              isExpanded && "rotate-180"
            )}
          />
        )}
      </button>

      {isExpanded && thinkingContent && (
        <div className="mt-2 pl-4 border-l-2 border-primary/20 text-sm prose dark:prose-invert max-w-none">
          {thinkingContent}
        </div>
      )}
    </div>
  );
}
```

**File:** `src/lib/utils/cot-parser.ts`

```typescript
export interface COTResult {
  isThinking: boolean;
  thinkingContent: string;
  displayContent: string;
  hasValidThinkBlock: boolean;
}

export function parseCOTContent(content: string): COTResult {
  const thinkStartTag = "<think>";
  const thinkEndTag = "</think>";

  const startMatches: number[] = [];
  const endMatches: number[] = [];

  let startIndex = 0;
  while ((startIndex = content.indexOf(thinkStartTag, startIndex)) !== -1) {
    startMatches.push(startIndex);
    startIndex += thinkStartTag.length;
  }

  let endIndex = 0;
  while ((endIndex = content.indexOf(thinkEndTag, endIndex)) !== -1) {
    endMatches.push(endIndex);
    endIndex += thinkEndTag.length;
  }

  const hasThinkStart = startMatches.length > 0;
  const hasThinkEnd = endMatches.length > 0;
  const isThinking = hasThinkStart && startMatches.length > endMatches.length;

  let thinkingContent = "";
  let displayContent = content;

  if (hasThinkStart) {
    if (hasThinkEnd && startMatches.length === endMatches.length) {
      const lastStartIndex = startMatches[startMatches.length - 1];
      const lastEndIndex = endMatches[endMatches.length - 1];

      if (lastEndIndex > lastStartIndex) {
        thinkingContent = content
          .substring(lastStartIndex + thinkStartTag.length, lastEndIndex)
          .trim();
        displayContent = content
          .substring(lastEndIndex + thinkEndTag.length)
          .trim();
      }
    } else if (isThinking) {
      const lastStartIndex = startMatches[startMatches.length - 1];
      thinkingContent = content.substring(
        lastStartIndex + thinkStartTag.length
      );
      displayContent = "";
    }
  }

  return {
    isThinking,
    thinkingContent,
    displayContent,
    hasValidThinkBlock:
      hasThinkStart && hasThinkEnd && startMatches.length === endMatches.length,
  };
}
```

---

## Medium Priority Solutions

### SOL-011: Enhanced Syntax Highlighting

**Gap Reference:** GAP-011  
**Effort:** S (2 hours)

```bash
bun add react-syntax-highlighter @types/react-syntax-highlighter
```

### SOL-012: Entity Property Editing

**Gap Reference:** GAP-012  
**Effort:** L (2 days)

Requires API endpoint implementation and UI components.

### SOL-013: Entity Merge

**Gap Reference:** GAP-013  
**Effort:** M (1 day)

Requires merge API and confirmation dialog.

### SOL-014: User Prompt History UI

**Gap Reference:** GAP-014  
**Effort:** S (4 hours)

Port `UserPromptInputWithHistory` component.

### SOL-015: Search History Manager

**Gap Reference:** GAP-015  
**Effort:** S (4 hours)

Port `SearchHistoryManager` utility class.

### SOL-016: Frontend Tests

**Gap Reference:** GAP-016  
**Effort:** XL (1 week)

Create test infrastructure and initial test suite.

### SOL-017: Query Mode Prefix

**Gap Reference:** GAP-017  
**Effort:** S (2 hours)

Add prefix parsing to query submission.

---

## Low Priority Solutions

### SOL-018: Tab Visibility Optimization

**Gap Reference:** GAP-018  
**Effort:** S (4 hours)

### SOL-019: Graph Full-Screen Mode

**Gap Reference:** GAP-019  
**Effort:** XS (2 hours)

### SOL-020: Graph Legend

**Gap Reference:** GAP-020  
**Effort:** S (4 hours)

---

## Architecture Recommendations

### 1. Component Organization

```
src/
├── components/
│   ├── documents/
│   │   ├── document-filters.tsx
│   │   ├── document-manager.tsx
│   │   ├── document-table.tsx
│   │   ├── pagination-controls.tsx
│   │   └── pipeline-status-dialog.tsx
│   ├── graph/
│   │   ├── graph-events.tsx
│   │   ├── graph-search.tsx
│   │   ├── graph-viewer.tsx
│   │   ├── layout-control.tsx
│   │   ├── legend.tsx
│   │   └── properties-panel.tsx
│   ├── query/
│   │   ├── chat-message.tsx
│   │   ├── markdown-renderer.tsx
│   │   ├── mermaid-diagram.tsx
│   │   ├── query-interface.tsx
│   │   └── thinking-display.tsx
│   └── shared/
│       ├── language-selector.tsx
│       └── theme-toggle.tsx
├── hooks/
│   ├── use-debounce.ts
│   ├── use-graph.ts
│   ├── use-url-state.ts
│   └── use-tab-visibility.ts
├── lib/
│   ├── i18n.ts
│   └── utils/
│       ├── cot-parser.ts
│       └── search-history.ts
├── locales/
│   ├── en.json
│   ├── zh.json
│   └── fr.json
└── __tests__/
    └── ...
```

### 2. State Management Strategy

- **Zustand:** Global app state (settings, auth, tenant)
- **React Query:** Server state (documents, graph data)
- **URL State:** Pagination, filters, sort (for shareable URLs)
- **Local State:** UI-only state (dialogs, dropdowns)

### 3. Performance Patterns

- Lazy load heavy components (Mermaid, KaTeX)
- Use React.memo for list items
- Implement virtual scrolling for large lists
- Tab visibility optimization for background tabs

---

## Cross-References

| Document                                                    | Relationship             |
| ----------------------------------------------------------- | ------------------------ |
| [Gap Analysis](./002-gap-analysis.md)                       | Source of requirements   |
| [Prioritization & Roadmap](./004-prioritization-roadmap.md) | Implementation order     |
| [UX Improvements](./005-ux-improvements.md)                 | UX-specific details      |
| [Performance Strategy](./006-performance-strategy.md)       | Performance optimization |
| [QA Plan](./007-qa-plan.md)                                 | Testing requirements     |

---

_Document provides implementation guidance for all identified gaps_
