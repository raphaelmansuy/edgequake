# Phase 2: Graph & Query Experience

## Overview

Phase 2 focuses on enhancing the knowledge graph visualization and query experience. These changes improve discoverability and make the system more powerful for users.

**Duration:** 2-3 days  
**Priority:** P1 + P2 issues  
**Prerequisite:** Phase 1 complete

---

## 1. Graph Export (P2 Medium)

### Current State

- No way to export graph visualization
- Users cannot share or document findings

### Target State

- Export graph as PNG image
- Export graph as SVG vector
- Export graph data as JSON

### Implementation Steps

#### 1.1 Create Export Controls

**New File:** `edgequake_webui/src/components/graph/graph-export.tsx`

```tsx
import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { Download, FileJson, Image, FileCode } from "lucide-react";
import { useGraphStore } from "@/stores/use-graph-store";
import { toast } from "sonner";

export function GraphExport() {
  const { sigmaInstance, nodes, edges } = useGraphStore();

  const exportAsPNG = async () => {
    if (!sigmaInstance) return;

    // Get canvas and convert to blob
    const container = document.querySelector("[data-graph-container] canvas");
    if (!container) return;

    const canvas = container as HTMLCanvasElement;
    canvas.toBlob((blob) => {
      if (!blob) return;
      const url = URL.createObjectURL(blob);
      const a = document.createElement("a");
      a.href = url;
      a.download = `edgequake-graph-${Date.now()}.png`;
      a.click();
      URL.revokeObjectURL(url);
      toast.success("Graph exported as PNG");
    });
  };

  const exportAsSVG = async () => {
    // Use sigma's built-in SVG export if available
    // Or use canvas-to-svg library
    toast.info("SVG export coming soon");
  };

  const exportAsJSON = () => {
    const data = {
      nodes: nodes.map((n) => ({
        id: n.id,
        label: n.label,
        type: n.node_type,
        description: n.description,
        properties: n.properties,
      })),
      edges: edges.map((e) => ({
        source: e.source,
        target: e.target,
        type: e.relationship_type,
        properties: e.properties,
      })),
      exportedAt: new Date().toISOString(),
    };

    const blob = new Blob([JSON.stringify(data, null, 2)], {
      type: "application/json",
    });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = `edgequake-graph-${Date.now()}.json`;
    a.click();
    URL.revokeObjectURL(url);
    toast.success("Graph exported as JSON");
  };

  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button variant="ghost" size="icon" title="Export graph">
          <Download className="h-4 w-4" />
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end">
        <DropdownMenuItem onClick={exportAsPNG}>
          <Image className="h-4 w-4 mr-2" />
          Export as PNG
        </DropdownMenuItem>
        <DropdownMenuItem onClick={exportAsSVG}>
          <FileCode className="h-4 w-4 mr-2" />
          Export as SVG
        </DropdownMenuItem>
        <DropdownMenuItem onClick={exportAsJSON}>
          <FileJson className="h-4 w-4 mr-2" />
          Export as JSON
        </DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
```

#### 1.2 Add Export to Toolbar

**File:** `edgequake_webui/src/components/graph/graph-viewer.tsx`

```tsx
// In toolbar section, add:
import { GraphExport } from "./graph-export";

// In the toolbar buttons:
<GraphExport />;
```

### Test Cases

- [ ] PNG export downloads file
- [ ] JSON export contains correct data
- [ ] Toast confirms export success

---

## 2. Graph Search Autocomplete (P2 Medium)

### Current State

- Basic search filters nodes
- No autocomplete suggestions

### Target State

- Typeahead suggestions as user types
- Highlight matching nodes
- Show node type in suggestions

### Implementation Steps

#### 2.1 Update Graph Search Component

**File:** `edgequake_webui/src/components/graph/graph-search.tsx`

```tsx
import { useState, useMemo, useCallback } from "react";
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
import { Search } from "lucide-react";
import { Button } from "@/components/ui/button";

export function GraphSearch() {
  const [open, setOpen] = useState(false);
  const [query, setQuery] = useState("");
  const { nodes, setSearchQuery, focusNode } = useGraphStore();

  const suggestions = useMemo(() => {
    if (!query) return [];
    const q = query.toLowerCase();
    return nodes
      .filter(
        (n) =>
          n.label.toLowerCase().includes(q) ||
          n.description?.toLowerCase().includes(q)
      )
      .slice(0, 10);
  }, [nodes, query]);

  const handleSelect = useCallback(
    (nodeId: string) => {
      focusNode(nodeId);
      setOpen(false);
      setQuery("");
    },
    [focusNode]
  );

  return (
    <Popover open={open} onOpenChange={setOpen}>
      <PopoverTrigger asChild>
        <Button variant="ghost" size="icon">
          <Search className="h-4 w-4" />
        </Button>
      </PopoverTrigger>
      <PopoverContent className="w-80 p-0" align="end">
        <Command>
          <CommandInput
            placeholder="Search nodes..."
            value={query}
            onValueChange={setQuery}
          />
          <CommandList>
            <CommandEmpty>No nodes found</CommandEmpty>
            <CommandGroup>
              {suggestions.map((node) => (
                <CommandItem
                  key={node.id}
                  value={node.id}
                  onSelect={handleSelect}
                >
                  <div className="flex items-center gap-2">
                    <div
                      className="w-3 h-3 rounded-full"
                      style={{ backgroundColor: getNodeColor(node.node_type) }}
                    />
                    <div>
                      <div className="font-medium">{node.label}</div>
                      <div className="text-xs text-muted-foreground">
                        {node.node_type}
                      </div>
                    </div>
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

### Test Cases

- [ ] Suggestions appear while typing
- [ ] Clicking suggestion focuses node
- [ ] Node type colors are correct
- [ ] Empty state shows when no matches

---

## 3. Legend Type Filter (P2 Medium)

### Current State

- Legend shows node types with counts
- Clicking does not filter

### Target State

- Click legend item to toggle visibility
- Visual indication of hidden types
- "Show All" / "Hide All" buttons

### Implementation Steps

#### 3.1 Update Graph Legend

**File:** `edgequake_webui/src/components/graph/graph-legend.tsx`

```tsx
// Add click handlers to toggle visibility
const handleTypeClick = (type: string) => {
  const newVisible = new Set(visibleEntityTypes);
  if (newVisible.has(type)) {
    newVisible.delete(type);
  } else {
    newVisible.add(type);
  }
  setVisibleEntityTypes(newVisible);
};

// Add visual styling for hidden types
<button
  onClick={() => handleTypeClick(type)}
  className={cn(
    "flex items-center gap-2 px-2 py-1 rounded-md transition-all",
    visibleEntityTypes.has(type) ? "opacity-100" : "opacity-40 line-through"
  )}
>
  <span className="w-3 h-3 rounded-full" style={{ backgroundColor: color }} />
  <span>{type}</span>
  <Badge variant="secondary" className="ml-auto">
    {count}
  </Badge>
</button>;
```

#### 3.2 Add Show All / Hide All

```tsx
<div className="flex gap-2 mb-2">
  <Button
    variant="ghost"
    size="sm"
    onClick={() => setVisibleEntityTypes(new Set(allTypes))}
  >
    Show All
  </Button>
  <Button
    variant="ghost"
    size="sm"
    onClick={() => setVisibleEntityTypes(new Set())}
  >
    Hide All
  </Button>
</div>
```

### Test Cases

- [ ] Clicking type toggles visibility
- [ ] Hidden types show visual indicator
- [ ] Graph updates when types hidden
- [ ] Show All restores all types

---

## 4. Query History Improvements (P2 Medium)

### Current State

- History sidebar shows recent + favorites mixed
- Query text truncated without hover tooltip

### Target State

- Separate tabs for Recent and Favorites
- Full text on hover tooltip
- Search within history

### Implementation Steps

#### 4.1 Update Query History Component

**File:** `edgequake_webui/src/components/query/query-interface.tsx`

In the history sidebar section:

```tsx
<Tabs defaultValue="recent">
  <TabsList className="grid w-full grid-cols-2">
    <TabsTrigger value="recent">
      <Clock className="h-4 w-4 mr-1" />
      Recent
    </TabsTrigger>
    <TabsTrigger value="favorites">
      <Star className="h-4 w-4 mr-1" />
      Favorites
    </TabsTrigger>
  </TabsList>

  <TabsContent value="recent">
    <ScrollArea className="h-[300px]">
      {recentQueries.map((q) => (
        <TooltipProvider key={q.id}>
          <Tooltip>
            <TooltipTrigger asChild>
              <button className="w-full text-left p-2 hover:bg-muted rounded-md">
                <span className="truncate block">{q.content}</span>
                <span className="text-xs text-muted-foreground">
                  {formatDistanceToNow(q.timestamp)}
                </span>
              </button>
            </TooltipTrigger>
            <TooltipContent side="left" className="max-w-xs">
              <p className="whitespace-pre-wrap">{q.content}</p>
            </TooltipContent>
          </Tooltip>
        </TooltipProvider>
      ))}
    </ScrollArea>
  </TabsContent>

  <TabsContent value="favorites">
    <ScrollArea className="h-[300px]">
      {favoriteQueries.map((q) => (
        // Same structure as recent
      ))}
    </ScrollArea>
  </TabsContent>
</Tabs>
```

### Test Cases

- [ ] Tabs switch between recent and favorites
- [ ] Full text shows on hover
- [ ] Favorites persist across sessions
- [ ] Can add/remove favorites

---

## 5. Document "View in Graph" (P2 Medium)

### Current State

- Document detail dialog exists
- No link to view entities in graph

### Target State

- Button to navigate to graph with entity filter
- Pre-select related entities in graph

### Implementation Steps

#### 5.1 Update Document Detail Dialog

**File:** `edgequake_webui/src/components/documents/document-detail-dialog.tsx`

```tsx
// Add View in Graph button
<Button
  variant="outline"
  onClick={() => {
    // Navigate to graph with document filter
    window.location.href = `/graph?document=${encodeURIComponent(document.id)}`;
    onOpenChange(false);
  }}
>
  <Network className="h-4 w-4 mr-2" />
  View in Graph
</Button>
```

#### 5.2 Handle Document Filter in Graph

**File:** `edgequake_webui/src/app/(dashboard)/graph/page.tsx`

```tsx
// Read query param and filter graph
const searchParams = useSearchParams();
const documentId = searchParams.get("document");

useEffect(() => {
  if (documentId) {
    // Filter nodes to show only those from this document
    // This requires document_id to be stored in node properties
  }
}, [documentId]);
```

### Test Cases

- [ ] Button visible in document dialog
- [ ] Clicking navigates to graph
- [ ] Graph filters by document

---

## 6. Auto-Expand Query Textarea (P2 Medium)

### Current State

- Fixed height textarea
- Shift+Enter for new lines

### Target State

- Textarea expands as user types
- Maximum height with scroll

### Implementation Steps

#### 6.1 Create Auto-Resize Hook

**New File:** `edgequake_webui/src/hooks/use-auto-resize.ts`

```tsx
import { useCallback, useLayoutEffect, RefObject } from "react";

export function useAutoResize(
  textareaRef: RefObject<HTMLTextAreaElement>,
  value: string,
  maxHeight = 200
) {
  const resize = useCallback(() => {
    const textarea = textareaRef.current;
    if (!textarea) return;

    // Reset height to auto to get scroll height
    textarea.style.height = "auto";

    // Set new height, capped at max
    const newHeight = Math.min(textarea.scrollHeight, maxHeight);
    textarea.style.height = `${newHeight}px`;
  }, [textareaRef, maxHeight]);

  useLayoutEffect(() => {
    resize();
  }, [value, resize]);

  return resize;
}
```

#### 6.2 Apply to Query Input

**File:** `edgequake_webui/src/components/query/query-interface.tsx`

```tsx
const textareaRef = useRef<HTMLTextAreaElement>(null);
useAutoResize(textareaRef, query, 200);

<Textarea
  ref={textareaRef}
  value={query}
  onChange={(e) => setQuery(e.target.value)}
  placeholder={t("query.placeholder")}
  className="resize-none min-h-[60px]"
/>;
```

### Test Cases

- [ ] Textarea starts at minimum height
- [ ] Expands as text is added
- [ ] Stops at maximum height
- [ ] Scrolls when max reached

---

## 7. Stop Generation Button (P2 Medium)

### Current State

- Streaming can be started but not stopped
- Must wait for completion

### Target State

- Stop button during generation
- AbortController to cancel request

### Implementation Steps

#### 7.1 Add Abort Controller

**File:** `edgequake_webui/src/components/query/query-interface.tsx`

```tsx
const [abortController, setAbortController] = useState<AbortController | null>(
  null
);

const handleSubmit = async () => {
  const controller = new AbortController();
  setAbortController(controller);

  try {
    await queryStream(query, {
      signal: controller.signal,
      onToken: (token) => {
        /* ... */
      },
    });
  } catch (err) {
    if (err.name === "AbortError") {
      toast.info("Generation stopped");
    } else {
      throw err;
    }
  } finally {
    setAbortController(null);
  }
};

const handleStop = () => {
  if (abortController) {
    abortController.abort();
    setAbortController(null);
  }
};
```

#### 7.2 Add Stop Button UI

```tsx
{
  isStreaming ? (
    <Button onClick={handleStop} variant="destructive">
      <StopCircle className="h-4 w-4 mr-2" />
      Stop
    </Button>
  ) : (
    <Button onClick={handleSubmit}>
      <Send className="h-4 w-4 mr-2" />
      Send
    </Button>
  );
}
```

### Test Cases

- [ ] Stop button appears during generation
- [ ] Clicking stops the stream
- [ ] Partial response is preserved
- [ ] Toast confirms stop

---

## E2E Test Additions

**File:** `edgequake_webui/e2e/phase2-ux.spec.ts`

```typescript
import { test, expect } from "@playwright/test";

test.describe("Phase 2: Graph & Query UX", () => {
  test("graph export button exists", async ({ page }) => {
    await page.goto("/graph");
    await expect(page.getByRole("button", { name: /export/i })).toBeVisible();
  });

  test("graph search shows autocomplete", async ({ page }) => {
    await page.goto("/graph");
    // Click search button
    await page.getByRole("button", { name: /search/i }).click();
    // Type in search
    await page.getByPlaceholder(/search nodes/i).fill("test");
    // Suggestions should appear (if nodes exist)
  });

  test("query history has tabs", async ({ page }) => {
    await page.goto("/query");
    await page.getByRole("button", { name: /history/i }).click();
    await expect(page.getByRole("tab", { name: /recent/i })).toBeVisible();
    await expect(page.getByRole("tab", { name: /favorites/i })).toBeVisible();
  });

  test("query textarea expands", async ({ page }) => {
    await page.goto("/query");
    const textarea = page.getByPlaceholder(/ask/i);
    const initialHeight = await textarea.boundingBox();

    // Type multiple lines
    await textarea.fill("Line 1\nLine 2\nLine 3\nLine 4");

    const newHeight = await textarea.boundingBox();
    expect(newHeight?.height).toBeGreaterThan(initialHeight?.height || 0);
  });
});
```

---

## Verification Checklist

Before marking Phase 2 complete:

- [ ] All P2 issues in scope resolved
- [ ] No TypeScript errors (`npm run build`)
- [ ] Lint passes (`npm run lint`)
- [ ] Phase 1 E2E tests still pass
- [ ] Phase 2 E2E tests pass
- [ ] Manual testing on desktop
- [ ] Manual testing on mobile viewport

---

## Files Modified Summary

| File                                              | Action | Description          |
| ------------------------------------------------- | ------ | -------------------- |
| `components/graph/graph-export.tsx`               | Create | Export functionality |
| `components/graph/graph-search.tsx`               | Modify | Add autocomplete     |
| `components/graph/graph-legend.tsx`               | Modify | Add type toggle      |
| `components/graph/graph-viewer.tsx`               | Modify | Add export button    |
| `components/documents/document-detail-dialog.tsx` | Modify | Add View in Graph    |
| `components/query/query-interface.tsx`            | Modify | Tabs, textarea, stop |
| `hooks/use-auto-resize.ts`                        | Create | Auto-resize hook     |
| `e2e/phase2-ux.spec.ts`                           | Create | Phase 2 E2E tests    |

---

## Next Phase

After Phase 2 is complete and committed, proceed to:

- [Phase 3: Polish & Accessibility](./03-phase3-polish.md)
