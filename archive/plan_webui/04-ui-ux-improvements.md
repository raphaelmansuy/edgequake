# EdgeQuake WebUI - UI/UX Improvements

> Planned enhancements over LightRAG WebUI for better user experience.

**Parent Document**: [00-master-plan.md](./00-master-plan.md)

---

## Table of Contents

1. [Overview](#overview)
2. [Navigation Improvements](#navigation-improvements)
3. [Graph Visualization Enhancements](#graph-visualization-enhancements)
4. [Query Experience](#query-experience)
5. [Document Management](#document-management)
6. [Accessibility](#accessibility)
7. [Performance](#performance)
8. [Mobile Responsiveness](#mobile-responsiveness)
9. [New Features](#new-features)

---

## Overview

This document outlines UI/UX improvements planned for EdgeQuake WebUI. Each section includes:

- **Problem**: Issue in current LightRAG implementation
- **Solution**: Proposed improvement
- **Priority**: P0 (Critical), P1 (High), P2 (Medium), P3 (Low)
- **Files**: Affected components

---

## Navigation Improvements

### 1. Tab-to-Sidebar Navigation

**Problem**: Tab-based navigation in [App.tsx](../lightrag_webui/src/App.tsx) limits scalability and wastes horizontal space.

**Solution**: Convert to persistent sidebar navigation with icons.

**Priority**: P1

**Implementation**:

```tsx
// components/layout/sidebar.tsx
"use client";

import Link from "next/link";
import { usePathname } from "next/navigation";
import {
  Network,
  FileText,
  MessageSquare,
  Settings,
  Terminal,
} from "lucide-react";

const navItems = [
  { href: "/graph", icon: Network, label: "Knowledge Graph" },
  { href: "/documents", icon: FileText, label: "Documents" },
  { href: "/query", icon: MessageSquare, label: "Query" },
  { href: "/api-explorer", icon: Terminal, label: "API Explorer" },
  { href: "/settings", icon: Settings, label: "Settings" },
];

export function Sidebar() {
  const pathname = usePathname();

  return (
    <aside className="w-64 border-r bg-muted/30 flex flex-col">
      <div className="p-4 border-b">
        <h1 className="text-xl font-bold">EdgeQuake</h1>
      </div>
      <nav className="flex-1 p-2">
        {navItems.map(({ href, icon: Icon, label }) => (
          <Link
            key={href}
            href={href}
            className={cn(
              "flex items-center gap-3 px-3 py-2 rounded-md transition-colors",
              pathname === href
                ? "bg-primary text-primary-foreground"
                : "hover:bg-muted"
            )}
          >
            <Icon className="h-5 w-5" />
            <span>{label}</span>
          </Link>
        ))}
      </nav>
    </aside>
  );
}
```

---

### 2. Breadcrumb Navigation

**Problem**: No context for current location in deep pages.

**Solution**: Add dynamic breadcrumbs.

**Priority**: P2

**Files**: `components/layout/breadcrumb.tsx`

---

### 3. Keyboard Shortcuts

**Problem**: No keyboard navigation support.

**Solution**: Add global keyboard shortcuts (Cmd+K for search, etc.).

**Priority**: P2

**Implementation**:

```tsx
// hooks/use-keyboard-shortcuts.ts
"use client";

import { useEffect } from "react";
import { useRouter } from "next/navigation";

export function useKeyboardShortcuts() {
  const router = useRouter();

  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      // Cmd/Ctrl + K: Open command palette
      if ((e.metaKey || e.ctrlKey) && e.key === "k") {
        e.preventDefault();
        document.getElementById("command-palette")?.click();
      }
      // Cmd/Ctrl + G: Go to Graph
      if ((e.metaKey || e.ctrlKey) && e.key === "g") {
        e.preventDefault();
        router.push("/graph");
      }
    };

    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [router]);
}
```

---

## Graph Visualization Enhancements

### 1. Node Clustering

**Problem**: Large graphs become unreadable with many nodes.

**Solution**: Add automatic clustering for related entities.

**Priority**: P1

**Reference**: [GraphViewer.tsx](../lightrag_webui/src/features/GraphViewer.tsx)

**Implementation**:

```tsx
// lib/graph/clustering.ts
import { Graph } from "graphology";
import louvain from "graphology-communities-louvain";

export function clusterGraph(graph: Graph) {
  // Detect communities using Louvain algorithm
  const communities = louvain(graph);

  // Assign community colors
  graph.forEachNode((node) => {
    const community = communities[node];
    graph.setNodeAttribute(node, "community", community);
    graph.setNodeAttribute(node, "color", getCommunityColor(community));
  });

  return graph;
}
```

---

### 2. Mini-Map Navigator

**Problem**: Losing context in large graph views.

**Solution**: Add mini-map overlay for navigation.

**Priority**: P2

**Files**: `components/graph/mini-map.tsx`

---

### 3. Timeline Slider

**Problem**: No way to visualize graph evolution over time.

**Solution**: Add timeline control to filter by document date.

**Priority**: P3

**Implementation**:

```tsx
// components/graph/timeline-slider.tsx
"use client";

import { Slider } from "@/components/ui/slider";
import { format } from "date-fns";

interface TimelineSliderProps {
  minDate: Date;
  maxDate: Date;
  value: [Date, Date];
  onChange: (range: [Date, Date]) => void;
}

export function TimelineSlider({
  minDate,
  maxDate,
  value,
  onChange,
}: TimelineSliderProps) {
  return (
    <div className="space-y-2">
      <div className="flex justify-between text-sm text-muted-foreground">
        <span>{format(value[0], "MMM d, yyyy")}</span>
        <span>{format(value[1], "MMM d, yyyy")}</span>
      </div>
      <Slider
        min={minDate.getTime()}
        max={maxDate.getTime()}
        value={[value[0].getTime(), value[1].getTime()]}
        onValueChange={([start, end]) =>
          onChange([new Date(start), new Date(end)])
        }
        step={86400000} // 1 day
      />
    </div>
  );
}
```

---

### 4. Node Context Menu

**Problem**: Limited node interaction options.

**Solution**: Right-click context menu with actions.

**Priority**: P1

**Actions**:

- View details
- Find related entities
- Merge with another node
- Delete entity
- Copy entity ID
- Expand neighborhood

---

### 5. Improved Edge Labels

**Problem**: Relationship labels overlap and are hard to read.

**Solution**: Smart label positioning with collision detection.

**Priority**: P2

---

## Query Experience

### 1. Real-time Streaming UI

**Problem**: [RetrievalTesting.tsx](../lightrag_webui/src/features/RetrievalTesting.tsx) waits for full response.

**Solution**: Token-by-token streaming with typing animation.

**Priority**: P0

**Implementation**:

```tsx
// components/query/streaming-response.tsx
"use client";

import { useEffect, useState } from "react";
import ReactMarkdown from "react-markdown";

interface StreamingResponseProps {
  stream: AsyncIterable<string>;
}

export function StreamingResponse({ stream }: StreamingResponseProps) {
  const [content, setContent] = useState("");
  const [isComplete, setIsComplete] = useState(false);

  useEffect(() => {
    let mounted = true;

    (async () => {
      for await (const chunk of stream) {
        if (!mounted) break;
        setContent((prev) => prev + chunk);
      }
      if (mounted) setIsComplete(true);
    })();

    return () => {
      mounted = false;
    };
  }, [stream]);

  return (
    <div className="prose dark:prose-invert max-w-none">
      <ReactMarkdown>{content}</ReactMarkdown>
      {!isComplete && (
        <span className="inline-block w-2 h-4 bg-foreground animate-pulse" />
      )}
    </div>
  );
}
```

---

### 2. Query Mode Selector

**Problem**: Mode dropdown is not intuitive (local/global/hybrid/naive).

**Solution**: Visual mode selector with explanations.

**Priority**: P1

**Implementation**:

```tsx
// components/query/mode-selector.tsx
const modes = [
  {
    id: "local",
    name: "Local",
    description: "Search within entity neighborhood",
    icon: Target,
  },
  {
    id: "global",
    name: "Global",
    description: "Search entire knowledge graph",
    icon: Globe,
  },
  {
    id: "hybrid",
    name: "Hybrid",
    description: "Combine local and global search",
    icon: Combine,
  },
];
```

---

### 3. Source Citations

**Problem**: No clear source attribution in answers.

**Solution**: Inline citations with hover previews.

**Priority**: P1

**Implementation**:

```tsx
// components/query/citation.tsx
"use client";

import {
  HoverCard,
  HoverCardContent,
  HoverCardTrigger,
} from "@/components/ui/hover-card";

interface CitationProps {
  index: number;
  source: {
    id: string;
    title: string;
    excerpt: string;
    documentId: string;
  };
}

export function Citation({ index, source }: CitationProps) {
  return (
    <HoverCard>
      <HoverCardTrigger asChild>
        <sup className="text-primary cursor-pointer hover:underline">
          [{index}]
        </sup>
      </HoverCardTrigger>
      <HoverCardContent className="w-80">
        <h4 className="font-semibold">{source.title}</h4>
        <p className="text-sm text-muted-foreground mt-1">{source.excerpt}</p>
      </HoverCardContent>
    </HoverCard>
  );
}
```

---

### 4. Query History with Search

**Problem**: History in [stores/settings.ts](../lightrag_webui/src/stores/settings.ts) has no search.

**Solution**: Searchable history with favorite queries.

**Priority**: P2

---

### 5. Suggested Questions

**Problem**: Cold start - users don't know what to ask.

**Solution**: AI-generated question suggestions based on graph content.

**Priority**: P3

---

## Document Management

### 1. Drag & Drop Upload Zone

**Problem**: Upload button is not prominent enough.

**Solution**: Full-page drag & drop with visual feedback.

**Priority**: P1

**Implementation**:

```tsx
// components/documents/drop-zone.tsx
"use client";

import { useDropzone } from "react-dropzone";
import { Upload } from "lucide-react";

export function DropZone({ onDrop }: { onDrop: (files: File[]) => void }) {
  const { getRootProps, getInputProps, isDragActive } = useDropzone({
    onDrop,
    accept: {
      "application/pdf": [".pdf"],
      "text/plain": [".txt"],
      "text/markdown": [".md"],
      "application/json": [".json"],
    },
  });

  return (
    <div
      {...getRootProps()}
      className={cn(
        "border-2 border-dashed rounded-lg p-12 text-center transition-colors",
        isDragActive
          ? "border-primary bg-primary/10"
          : "border-muted-foreground/25 hover:border-primary/50"
      )}
    >
      <input {...getInputProps()} />
      <Upload className="h-12 w-12 mx-auto text-muted-foreground" />
      <p className="mt-4 text-lg">
        {isDragActive
          ? "Drop files here"
          : "Drag & drop files or click to upload"}
      </p>
      <p className="mt-2 text-sm text-muted-foreground">
        PDF, TXT, MD, JSON up to 10MB
      </p>
    </div>
  );
}
```

---

### 2. Processing Pipeline Visualization

**Problem**: [PipelineStatusDialog.tsx](../lightrag_webui/src/components/documents/PipelineStatusDialog.tsx) shows status but no progress detail.

**Solution**: Visual pipeline stages with real-time progress.

**Priority**: P1

**Stages**:

1. Upload → Parsing
2. Parsing → Chunking
3. Chunking → Entity Extraction
4. Entity Extraction → Relationship Building
5. Relationship Building → Embedding
6. Embedding → Complete

---

### 3. Document Preview

**Problem**: Cannot preview document content before processing.

**Solution**: In-app document viewer with extraction preview.

**Priority**: P2

---

### 4. Batch Operations

**Problem**: No multi-select for bulk delete/reprocess.

**Solution**: Checkbox selection with batch action bar.

**Priority**: P2

---

## Accessibility

### 1. ARIA Labels

**Problem**: Missing accessibility labels on interactive elements.

**Solution**: Add comprehensive ARIA support.

**Priority**: P1

---

### 2. Focus Management

**Problem**: Tab order is inconsistent.

**Solution**: Implement focus trapping in modals, logical tab order.

**Priority**: P1

---

### 3. Screen Reader Support

**Problem**: Graph is not accessible to screen readers.

**Solution**: Add alternative text view of graph relationships.

**Priority**: P2

---

### 4. High Contrast Mode

**Problem**: Some colors have insufficient contrast.

**Solution**: Add high-contrast theme option.

**Priority**: P3

---

## Performance

### 1. Virtualized Lists

**Problem**: Document list in [DocumentManager.tsx](../lightrag_webui/src/features/DocumentManager.tsx) renders all items.

**Solution**: Virtual scrolling for large lists.

**Priority**: P1

**Implementation**:

```tsx
// components/documents/document-list.tsx
"use client";

import { useVirtualizer } from "@tanstack/react-virtual";
import { useRef } from "react";

export function DocumentList({ documents }) {
  const parentRef = useRef<HTMLDivElement>(null);

  const virtualizer = useVirtualizer({
    count: documents.length,
    getScrollElement: () => parentRef.current,
    estimateSize: () => 72, // Row height
    overscan: 5,
  });

  return (
    <div ref={parentRef} className="h-[600px] overflow-auto">
      <div
        style={{
          height: `${virtualizer.getTotalSize()}px`,
          position: "relative",
        }}
      >
        {virtualizer.getVirtualItems().map((virtualRow) => (
          <DocumentRow
            key={documents[virtualRow.index].id}
            document={documents[virtualRow.index]}
            style={{
              position: "absolute",
              top: 0,
              left: 0,
              width: "100%",
              transform: `translateY(${virtualRow.start}px)`,
            }}
          />
        ))}
      </div>
    </div>
  );
}
```

---

### 2. Graph WebGL Rendering

**Problem**: Canvas rendering slows with 1000+ nodes.

**Solution**: Enable WebGL2 renderer in Sigma.js.

**Priority**: P1

**Implementation**:

```tsx
import { WebGL2PixiRenderer } from "@sigma/webgl2-pixi-renderer";

const sigma = new Sigma(graph, container, {
  renderer: "webgl2",
  // ...
});
```

---

### 3. Lazy Loading

**Problem**: All feature code loads on initial page.

**Solution**: Route-based code splitting with Next.js dynamic imports.

**Priority**: P1

---

### 4. Optimistic Updates

**Problem**: UI waits for API response to update.

**Solution**: Optimistic updates with rollback on error.

**Priority**: P2

---

## Mobile Responsiveness

### 1. Responsive Sidebar

**Problem**: Sidebar doesn't collapse on mobile.

**Solution**: Collapsible sidebar with hamburger menu.

**Priority**: P1

---

### 2. Touch Graph Navigation

**Problem**: Graph requires mouse for interaction.

**Solution**: Touch gestures for pan, pinch-zoom, tap-select.

**Priority**: P2

---

### 3. Mobile Query Input

**Problem**: Query input is hard to use on small screens.

**Solution**: Full-screen query modal on mobile.

**Priority**: P2

---

## New Features

### 1. Command Palette (Cmd+K)

**Solution**: Global command palette for quick navigation and actions.

**Priority**: P1

**Implementation**:

```tsx
// components/command-palette.tsx
"use client";

import {
  CommandDialog,
  CommandInput,
  CommandList,
  CommandEmpty,
  CommandGroup,
  CommandItem,
} from "@/components/ui/command";

export function CommandPalette() {
  const [open, setOpen] = useState(false);

  // ... Cmd+K listener

  return (
    <CommandDialog open={open} onOpenChange={setOpen}>
      <CommandInput placeholder="Type a command or search..." />
      <CommandList>
        <CommandEmpty>No results found.</CommandEmpty>
        <CommandGroup heading="Navigation">
          <CommandItem onSelect={() => router.push("/graph")}>
            Go to Graph
          </CommandItem>
          {/* ... */}
        </CommandGroup>
        <CommandGroup heading="Actions">
          <CommandItem onSelect={() => openUploadDialog()}>
            Upload Document
          </CommandItem>
          {/* ... */}
        </CommandGroup>
      </CommandList>
    </CommandDialog>
  );
}
```

---

### 2. Dashboard Overview

**Solution**: New home page with key metrics and quick actions.

**Priority**: P1

**Metrics**:

- Total documents
- Entity count
- Relationship count
- Recent queries
- Processing status

---

### 3. Entity Merging Wizard

**Solution**: Guided flow for merging duplicate entities.

**Priority**: P2

---

### 4. Export/Import Graph

**Solution**: Export knowledge graph to various formats.

**Priority**: P2

**Formats**:

- JSON (GraphQL)
- Neo4j Cypher
- RDF/Turtle
- CSV (nodes + edges)

---

### 5. Multi-tenant Workspace Switcher

**Solution**: Quick workspace switching without login.

**Priority**: P2

---

## Priority Summary

| Priority | Count | Description                |
| -------- | ----- | -------------------------- |
| P0       | 1     | Critical - streaming UI    |
| P1       | 15    | High - core improvements   |
| P2       | 12    | Medium - nice to have      |
| P3       | 4     | Low - future consideration |

---

## Related Documents

- **Previous**: [03-component-mapping.md](./03-component-mapping.md) - Component migration
- **Next**: [05-technology-migration.md](./05-technology-migration.md) - Tech stack migration
