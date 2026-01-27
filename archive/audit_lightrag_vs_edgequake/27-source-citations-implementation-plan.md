# Source Citations Deep Link Implementation Plan

**Document ID:** 27  
**Date:** 2025-12-31  
**Version:** 1.0  
**Status:** Ready for Implementation

---

## Implementation Overview

This plan provides precision code changes for implementing:

1. Accurate confidence score calculation
2. Document deep linking with chunk highlighting
3. Graph subgraph filtering from query context

**Total Estimated Time:** 6-8 hours

---

## Phase 1: Fix Confidence Score Calculation

**Time:** 30 minutes

### 1.1 Update calculateConfidence Function

**File:** `edgequake_webui/src/components/query/source-citations.tsx`
**Lines:** 36-51

**Current:**

```typescript
const calculateConfidence = (context: QueryContext): number => {
  const scores = [
    ...(context.chunks?.map((c) => c.score) || []),
    ...(context.entities?.map((e) => e.relevance) || []),
    ...(context.relationships?.map((r) => r.relevance) || []),
  ];
  if (scores.length === 0) return 0;
  return scores.reduce((a, b) => a + b, 0) / scores.length;
};
```

**Replace with:**

```typescript
const calculateConfidence = (context: QueryContext): number => {
  // Use chunk scores as primary signal (they're reliable cosine similarities)
  const chunkScores =
    context.chunks?.map((c) => c.score).filter((s) => s > 0) || [];

  if (chunkScores.length === 0) {
    // No chunks: use entity/relationship relevance, filtering zeros
    const entityScores =
      context.entities?.map((e) => e.relevance).filter((r) => r > 0) || [];
    const relScores =
      context.relationships?.map((r) => r.relevance).filter((r) => r > 0) || [];
    const allScores = [...entityScores, ...relScores];
    if (allScores.length === 0) return 0.5; // Default medium confidence
    return allScores.reduce((a, b) => a + b, 0) / allScores.length;
  }

  // Weighted calculation: max score (60%) + average (30%) + entity bonus (10%)
  const maxScore = Math.max(...chunkScores);
  const avgScore = chunkScores.reduce((a, b) => a + b, 0) / chunkScores.length;
  const entityBonus = Math.min(0.1, (context.entities?.length || 0) * 0.005);

  return Math.min(1.0, maxScore * 0.6 + avgScore * 0.3 + entityBonus);
};
```

---

## Phase 2: Fix Document Click Handler

**Time:** 30 minutes

### 2.1 Update SourceCitationsProps Interface

**File:** `edgequake_webui/src/components/query/source-citations.tsx`
**Lines:** 27-32

**Current:**

```typescript
interface SourceCitationsProps {
  context: QueryContext;
  onEntityClick?: (entityId: string) => void;
  onDocumentClick?: (documentId: string) => void;
  onExploreGraph?: () => void;
}
```

**Replace with:**

```typescript
interface SourceCitationsProps {
  context: QueryContext;
  onEntityClick?: (entityId: string) => void;
  onDocumentClick?: (
    documentId: string,
    chunkContent?: string,
    chunkIndex?: number
  ) => void;
  onExploreGraph?: (entityLabels: string[]) => void;
}
```

### 2.2 Update DocumentsTab Click Handler

**File:** `edgequake_webui/src/components/query/source-citations.tsx`
**Lines:** 137-141

**Current:**

```typescript
onClick={() => onDocumentClick?.(docId)}
```

**Replace with:**

```typescript
onClick={() => onDocumentClick?.(docId, chunks[0]?.content, 0)}
```

### 2.3 Update chat-message.tsx Handler

**File:** `edgequake_webui/src/components/query/chat-message.tsx`
**Lines:** 435-437

**Current:**

```typescript
onDocumentClick={(documentId) => {
  window.location.href = `/documents?id=${encodeURIComponent(documentId)}`;
}}
```

**Replace with:**

```typescript
onDocumentClick={(documentId, chunkContent) => {
  const highlight = chunkContent
    ? `?highlight=${encodeURIComponent(chunkContent.slice(0, 100))}`
    : '';
  window.location.href = `/documents/${encodeURIComponent(documentId)}${highlight}`;
}}
```

---

## Phase 3: Wire onExploreGraph Handler

**Time:** 30 minutes

### 3.1 Update ExploreTab Component

**File:** `edgequake_webui/src/components/query/source-citations.tsx`
**Lines:** 354-391

**Update ExploreTab interface:**

```typescript
const ExploreTab = ({
  entities,
  entityCount,
  relationshipCount,
  onExploreGraph,
}: {
  entities?: QueryContext["entities"];
  entityCount: number;
  relationshipCount: number;
  onExploreGraph?: (entityLabels: string[]) => void;
}) => {
  const handleExplore = () => {
    const entityLabels = entities?.map((e) => e.label) || [];
    onExploreGraph?.(entityLabels);
  };

  return (
    <div className="flex flex-col items-center justify-center py-8 space-y-4">
      <div className="relative">
        <div className="w-20 h-20 rounded-full bg-gradient-to-br from-primary/20 to-primary/5 flex items-center justify-center">
          <Network className="h-8 w-8 text-primary" />
        </div>
        <div className="absolute -top-1 -right-1 w-6 h-6 rounded-full bg-primary text-primary-foreground text-[10px] font-semibold flex items-center justify-center">
          {entityCount}
        </div>
      </div>
      <div className="text-center space-y-1">
        <p className="text-sm font-semibold">Explore Knowledge Graph</p>
        <p className="text-xs text-muted-foreground">
          {entityCount} topics · {relationshipCount} connections
        </p>
      </div>
      <Button onClick={handleExplore} className="gap-2" size="sm">
        <Network className="h-4 w-4" />
        Open Graph Explorer
      </Button>
    </div>
  );
};
```

### 3.2 Update TabsContent for Explore

**File:** `edgequake_webui/src/components/query/source-citations.tsx`
**Lines:** ~490-497

**Update to pass entities:**

```typescript
<TabsContent value="explore" className="mt-0 focus-visible:outline-none">
  <ExploreTab
    entities={context.entities}
    entityCount={context.entities?.length || 0}
    relationshipCount={context.relationships?.length || 0}
    onExploreGraph={onExploreGraph}
  />
</TabsContent>
```

### 3.3 Add onExploreGraph in chat-message.tsx

**File:** `edgequake_webui/src/components/query/chat-message.tsx`
**After line 437, add:**

```typescript
onExploreGraph={(entityLabels) => {
  const entities = entityLabels.slice(0, 10).join(',');
  const focus = entityLabels[0] || '';
  window.location.href = `/graph?entities=${encodeURIComponent(entities)}&focus=${encodeURIComponent(focus)}`;
}}
```

---

## Phase 4: Document Detail Page Highlighting

**Time:** 2 hours

### 4.1 Add URL Param Reading

**File:** `edgequake_webui/src/app/(dashboard)/documents/[id]/page.tsx`
**Add import and state:**

```typescript
import { useSearchParams } from "next/navigation";

export default function DocumentViewPage() {
  const searchParams = useSearchParams();
  const highlightText = searchParams?.get("highlight");

  // ... existing code ...

  return (
    // ... existing layout ...
    <ContentRenderer
      document={document}
      highlightText={
        highlightText ? decodeURIComponent(highlightText) : undefined
      }
    />
  );
}
```

### 4.2 Update ContentRenderer

**File:** `edgequake_webui/src/components/document/content-renderer.tsx`

**Add highlight support:**

```typescript
interface ContentRendererProps {
  document: Document;
  highlightText?: string;
}

export function ContentRenderer({
  document,
  highlightText,
}: ContentRendererProps) {
  const contentRef = useRef<HTMLDivElement>(null);

  // Scroll to and highlight matching text
  useEffect(() => {
    if (!highlightText || !contentRef.current) return;

    const scrollToHighlight = () => {
      const content = contentRef.current;
      if (!content) return;

      // Find text in content
      const searchText = highlightText.slice(0, 50).toLowerCase();
      const walker = document.createTreeWalker(content, NodeFilter.SHOW_TEXT);
      let node: Text | null;

      while ((node = walker.nextNode() as Text)) {
        if (node.textContent?.toLowerCase().includes(searchText)) {
          // Create highlight wrapper
          const parent = node.parentElement;
          if (parent && !parent.classList.contains("citation-highlight")) {
            parent.classList.add(
              "citation-highlight",
              "bg-yellow-200",
              "dark:bg-yellow-800/50"
            );
            parent.scrollIntoView({ behavior: "smooth", block: "center" });

            // Remove highlight after 3 seconds
            setTimeout(() => {
              parent.classList.remove(
                "citation-highlight",
                "bg-yellow-200",
                "dark:bg-yellow-800/50"
              );
            }, 5000);
          }
          break;
        }
      }
    };

    // Delay to ensure content is rendered
    const timer = setTimeout(scrollToHighlight, 500);
    return () => clearTimeout(timer);
  }, [highlightText]);

  return (
    <div ref={contentRef} className="p-8 max-w-4xl mx-auto">
      {/* ... existing renderer logic ... */}
    </div>
  );
}
```

---

## Phase 5: Graph Page URL Filtering

**Time:** 2 hours

### 5.1 Update Graph Page

**File:** `edgequake_webui/src/app/(dashboard)/graph/page.tsx`

**Replace entire file:**

```typescript
"use client";

import { Skeleton } from "@/components/ui/skeleton";
import { useGraphStore } from "@/stores/use-graph-store";
import dynamic from "next/dynamic";
import { useSearchParams } from "next/navigation";
import { Suspense, useEffect } from "react";

// Dynamic import for GraphViewer
const GraphViewer = dynamic(() => import("@/components/graph/graph-viewer"), {
  ssr: false,
  loading: () => <GraphSkeleton />,
});

const GraphTourWrapper = dynamic(
  () => import("@/components/graph/graph-tour-wrapper"),
  { ssr: false }
);

function GraphSkeleton() {
  return (
    <div className="flex h-full">
      <div className="flex-1 flex flex-col">
        <div className="flex items-center justify-between border-b px-4 py-2">
          <Skeleton className="h-6 w-32" />
          <Skeleton className="h-8 w-8" />
        </div>
        <div className="flex-1 flex items-center justify-center">
          <Skeleton className="h-64 w-64 rounded-full" />
        </div>
      </div>
    </div>
  );
}

function GraphPageContent() {
  const searchParams = useSearchParams();
  const entitiesParam = searchParams?.get("entities");
  const focusParam = searchParams?.get("focus");

  const setStartNode = useGraphStore((s) => s.setStartNode);
  const setSearchQuery = useGraphStore((s) => s.setSearchQuery);

  useEffect(() => {
    // Set focus node for neighborhood view
    if (focusParam) {
      setStartNode(decodeURIComponent(focusParam));
    }

    // Set search query to filter visible entities
    if (entitiesParam) {
      const entities = decodeURIComponent(entitiesParam).split(",");
      // Use first entity as search query for filtering
      if (entities.length > 0) {
        setSearchQuery(entities[0]);
      }
    }
  }, [focusParam, entitiesParam, setStartNode, setSearchQuery]);

  return (
    <GraphTourWrapper>
      <GraphViewer />
    </GraphTourWrapper>
  );
}

export default function GraphPage() {
  return (
    <div className="h-full overflow-hidden">
      <Suspense fallback={<GraphSkeleton />}>
        <GraphPageContent />
      </Suspense>
    </div>
  );
}
```

### 5.2 Add Query Context Banner (Optional Enhancement)

**File:** `edgequake_webui/src/components/graph/graph-viewer.tsx`

Add a banner showing the query context when entities are passed:

```typescript
// Add to GraphViewer component
const searchParams = useSearchParams();
const queryContext = searchParams?.get("entities");

// In the return JSX, add after the header:
{
  queryContext && (
    <div className="absolute top-12 left-4 right-4 z-40 bg-primary/10 border border-primary/20 rounded-lg p-2 flex items-center justify-between">
      <div className="flex items-center gap-2 text-sm">
        <Network className="h-4 w-4 text-primary" />
        <span>Showing entities from your query</span>
      </div>
      <Button
        variant="ghost"
        size="sm"
        onClick={() => window.history.pushState({}, "", "/graph")}
      >
        Show all
      </Button>
    </div>
  );
}
```

---

## Phase 6: Create Synthetic Test Document

**Time:** 30 minutes

### 6.1 Create Test Document File

**File:** `test_edge_quake_research.md`

```markdown
# EdgeQuake Research Document

## Abstract

EdgeQuake is an advanced Retrieval-Augmented Generation (RAG) framework designed
by Dr. Sarah Chen at Stanford University. It combines graph-based knowledge
representation with large language model capabilities to enable sophisticated
question-answering across diverse document collections.

## Key Components

The EdgeQuake system consists of several integrated components:

- **GraphRAG Engine**: The core retrieval system that leverages knowledge graph
  traversal for context gathering. It supports local, global, and hybrid query modes.

- **LightRAG Integration**: A performance optimization layer that provides
  efficient entity extraction and relationship mapping.

- **Vector Store**: Embeddings storage powered by PostgreSQL AGE, enabling
  scalable similarity search across millions of documents.

- **Streaming Pipeline**: Real-time document ingestion with progressive
  entity extraction and graph building.

## Relationships

The following relationships define the EdgeQuake architecture:

Dr. Sarah Chen leads the EdgeQuake project at Stanford University.
GraphRAG Engine is the core component of EdgeQuake.
LightRAG Integration enhances EdgeQuake's extraction capabilities.
PostgreSQL AGE provides graph storage for EdgeQuake.
The Streaming Pipeline feeds documents into EdgeQuake for processing.

## Performance Benchmarks

EdgeQuake achieves exceptional performance on standard benchmarks:

- 95% accuracy on multi-hop question answering
- 40% improvement over NaiveRAG in comprehensive retrieval
- 3x faster query response than GraphRAG baseline
- Supports 100,000+ nodes in knowledge graph

## Related Work

EdgeQuake builds on several foundational works:

- MegaRAG: Response generation methodology comparison
- GraphRAG: Microsoft's graph-based retrieval approach
- LightRAG: Efficient RAG implementation by Jiang et al.
- NaiveRAG: Baseline chunking approach for comparison

## Conclusion

EdgeQuake represents a significant advancement in knowledge graph-enhanced
retrieval systems, offering superior accuracy and performance for enterprise
question-answering applications.
```

---

## Phase 7: Playwright E2E Test

**Time:** 1 hour

### 7.1 Create Test Spec

**File:** `edgequake_webui/e2e/source-citations-deep-link.spec.ts`

```typescript
import { expect, test } from '@playwright/test';

test.describe('Source Citations Deep Linking', () => {
  test.beforeEach(async ({ page }) => {
    // Navigate to query page
    await page.goto('/query');
    await page.waitForLoadState('networkidle');
  });

  test('should display accurate confidence score', async ({ page }) => {
    // Submit a query
    await page.fill('[placeholder="Ask a question..."]', 'What is EdgeQuake?');
    await page.press('[placeholder="Ask a question..."]', 'Enter');

    // Wait for response
    await page.waitForSelector('[data-testid="source-citations"]', { timeout: 30000 });

    // Check confidence is reasonable (not 4-5%)
    const confidenceText = await page.locator('text=/\\d+%/').first().textContent();
    const confidenceValue = parseInt(confidenceText?.match(/\\d+/)?.[0] || '0');
    expect(confidenceValue).toBeGreaterThan(30);
  });

  test('should navigate to document with highlight', async ({ page }) => {
    // Submit a query
    await page.fill('[placeholder="Ask a question..."]', 'What is EdgeQuake?');
    await page.press('[placeholder="Ask a question..."]', 'Enter');

    // Wait for source citations
    await page.waitForSelector('[data-testid="source-citations"]');

    // Expand citations
    await page.click('[data-testid="source-citations"]');

    // Click on first document
    await page.click('[data-testid="document-link"]');

    // Verify navigation to document detail page
    await expect(page).toHaveURL(/\\/documents\\/[a-f0-9-]+/);

    // Verify highlight parameter is present
    await expect(page).toHaveURL(/highlight=/);
  });

  test('should navigate to graph with entity filter', async ({ page }) => {
    // Submit a query
    await page.fill('[placeholder="Ask a question..."]', 'What is EdgeQuake?');
    await page.press('[placeholder="Ask a question..."]', 'Enter');

    // Wait for source citations
    await page.waitForSelector('[data-testid="source-citations"]');

    // Expand citations and go to Explore tab
    await page.click('[data-testid="source-citations"]');
    await page.click('text=Explore');

    // Click Open Graph Explorer
    await page.click('text=Open Graph Explorer');

    // Verify navigation to graph with entities param
    await expect(page).toHaveURL(/\\/graph\\?entities=/);
  });
});
```

---

## File Change Summary

| File                                 | Action  | Changes                                 |
| ------------------------------------ | ------- | --------------------------------------- |
| `source-citations.tsx`               | Modify  | Confidence calc, ExploreTab, interfaces |
| `chat-message.tsx`                   | Modify  | Document + graph handlers               |
| `documents/[id]/page.tsx`            | Modify  | URL param reading, highlight passing    |
| `content-renderer.tsx`               | Modify  | Highlight + scroll support              |
| `graph/page.tsx`                     | Replace | URL param filtering                     |
| `test_edge_quake_research.md`        | Create  | Test document                           |
| `source-citations-deep-link.spec.ts` | Create  | E2E test                                |

---

## Validation Checklist

- [ ] Confidence score shows 50-95% for good matches
- [ ] Document click opens `/documents/{id}` not `/documents?id=`
- [ ] Document page scrolls to and highlights chunk text
- [ ] "Open Graph Explorer" navigates to `/graph?entities=...`
- [ ] Graph page filters to show query-relevant entities
- [ ] E2E tests pass

---

**Document Status:** ✅ Complete  
**Ready for Implementation:** Yes
