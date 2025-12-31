# Source Citations Deep Link & Graph Filter Audit

**Document ID:** 26  
**Date:** 2025-12-31  
**Version:** 1.0  
**Status:** Analysis Complete  

---

## Executive Summary

This audit analyzes three key enhancements requested for the Source Citations component:

1. **Relevance Score Accuracy** - Test and adjust confidence/relevance scores to ensure they bring value
2. **Document Deep Linking** - Link directly to document detail page with chunk highlighting
3. **Graph Subgraph Filtering** - "Open Graph Explorer" should filter to show only query-relevant entities

---

## Part 1: Current Implementation Analysis

### 1.1 Relevance Score Calculation

**File:** `source-citations.tsx` (lines 36-51)

```typescript
const calculateConfidence = (context: QueryContext): number => {
  const scores = [
    ...(context.chunks?.map(c => c.score) || []),
    ...(context.entities?.map(e => e.relevance) || []),
    ...(context.relationships?.map(r => r.relevance) || []),
  ];
  if (scores.length === 0) return 0;
  return scores.reduce((a, b) => a + b, 0) / scores.length;
};
```

**Issues Identified:**
1. **Mixed signal types**: Chunks have `score` (0-1 cosine similarity), entities/relationships have `relevance` (0-1 retrieval score)
2. **Equal weighting**: All sources weighted equally regardless of importance
3. **Score range mismatch**: Entity/relationship relevance often defaults to 0% in current backend
4. **Low confidence display**: Screenshots show "Low (4-5%)" which doesn't inspire user confidence

**Root Cause:**
The backend returns entities/relationships with `relevance: 0.0` when provenance isn't computed, dragging down the average.

### 1.2 Document Click Handler

**File:** `chat-message.tsx` (lines 435-437)

```typescript
onDocumentClick={(documentId) => {
  window.location.href = `/documents?id=${encodeURIComponent(documentId)}`;
}}
```

**Issues:**
1. **Wrong URL pattern**: Uses `/documents?id=` but actual detail page is `/documents/[id]`
2. **No chunk highlighting**: Doesn't pass chunk content or position
3. **Full page reload**: Uses `window.location.href` instead of Next.js router
4. **No chunk index**: Doesn't pass which chunk was clicked for scroll-to

### 1.3 Explore Graph Handler

**File:** `source-citations.tsx` (ExploreTab component)

```typescript
<Button 
  onClick={onExploreGraph} 
  className="gap-2"
  size="sm"
>
  <Network className="h-4 w-4" />
  Open Graph Explorer
</Button>
```

**File:** `chat-message.tsx` - **NO onExploreGraph prop passed!**

```typescript
<SourceCitations
  context={message.context}
  onEntityClick={(entityId) => { ... }}
  onDocumentClick={(documentId) => { ... }}
  // Missing: onExploreGraph
/>
```

**Issues:**
1. **Handler not wired**: `onExploreGraph` prop is never passed
2. **No filtering logic**: Even if wired, it would just navigate to `/graph` without filters
3. **No entity list passing**: Need to pass entity labels to pre-filter the graph

---

## Part 2: Backend Context Analysis

### 2.1 Query Context Structure

**File:** `types/index.ts` (lines 245-274)

```typescript
export interface QueryContext {
  chunks: Array<{
    content: string;
    document_id: string;
    score: number;          // Cosine similarity 0-1
    file_path?: string;
  }>;
  entities: Array<{
    id: string;
    label: string;
    relevance: number;      // Often 0.0 from backend
    source_document_id?: string;
    source_file_path?: string;
    source_chunk_ids?: string[];
  }>;
  relationships: Array<{
    source: string;
    target: string;
    type: string;
    relevance: number;      // Often 0.0 from backend
    source_document_id?: string;
    source_file_path?: string;
  }>;
}
```

### 2.2 Graph Store Filtering

**File:** `stores/use-graph-store.ts` (lines 111-133)

The graph store supports:
- `startNode`: Focus on specific node neighborhood
- `searchQuery`: Text filter for node labels
- `setVisibleEntityTypes`: Filter by entity type
- `maxNodes` / `depth`: Virtual query limits

**Key API:**
```typescript
// API already supports filtered graph fetch
getGraph(workspaceId, {
  startNode?: string;      // Focus node label
  maxNodes?: number;       // Limit results
  depth?: number;          // Traversal depth
})
```

---

## Part 3: URL Parameter Analysis

### 3.1 Document Detail Page

**File:** `app/(dashboard)/documents/[id]/page.tsx`

Accepts:
- Route param: `/documents/{documentId}`
- Currently NO query params for chunk highlighting

**Needed:**
- `?chunk={chunkId}` or `?highlight={text}` for scroll-to and highlight

### 3.2 Graph Page

**File:** `app/(dashboard)/graph/page.tsx`

Currently NO URL param reading. Graph settings are in Zustand store only.

**Needed:**
- `?entities={entity1,entity2,...}` - Pre-filter to specific entities
- `?focus={entityLabel}` - Set startNode for neighborhood view
- `?query={originalQuery}` - Display query context

---

## Part 4: Relevance Score Fix Strategy

### 4.1 Problem: Backend Returns 0.0 Relevance

When entities are extracted but provenance isn't computed, they have `relevance: 0.0`.

### 4.2 Solution: Smart Confidence Calculation

```typescript
const calculateConfidence = (context: QueryContext): number => {
  // Only use chunk scores - they're reliable cosine similarities
  const chunkScores = context.chunks?.map(c => c.score) || [];
  
  if (chunkScores.length === 0) {
    // Fallback: If we have entities/relationships but no chunks,
    // use their relevance but filter out zeros
    const entityScores = context.entities?.map(e => e.relevance).filter(r => r > 0) || [];
    const relScores = context.relationships?.map(r => r.relevance).filter(r => r > 0) || [];
    const allScores = [...entityScores, ...relScores];
    if (allScores.length === 0) return 0.5; // Default medium confidence
    return allScores.reduce((a, b) => a + b, 0) / allScores.length;
  }
  
  // Weighted average: chunks are primary signal (70%), entities secondary (30%)
  const avgChunkScore = chunkScores.reduce((a, b) => a + b, 0) / chunkScores.length;
  
  // Entity count bonus: more entities = higher confidence
  const entityBonus = Math.min(0.1, (context.entities?.length || 0) * 0.01);
  
  return Math.min(1.0, avgChunkScore * 0.8 + entityBonus + 0.1);
};
```

### 4.3 Alternative: Use Highest Score

```typescript
const calculateConfidence = (context: QueryContext): number => {
  const chunkScores = context.chunks?.map(c => c.score) || [];
  if (chunkScores.length === 0) return 0.5;
  
  // Use weighted combination of max and average
  const maxScore = Math.max(...chunkScores);
  const avgScore = chunkScores.reduce((a, b) => a + b, 0) / chunkScores.length;
  
  // 60% max + 40% average = balanced confidence
  return maxScore * 0.6 + avgScore * 0.4;
};
```

---

## Part 5: Document Deep Link Implementation

### 5.1 URL Structure

```
/documents/{documentId}?chunk={chunkIndex}&highlight={encodedText}
```

### 5.2 Document Detail Page Changes

**File:** `app/(dashboard)/documents/[id]/page.tsx`

Add URL param reading:

```typescript
import { useSearchParams } from 'next/navigation';

export default function DocumentViewPage() {
  const searchParams = useSearchParams();
  const chunkIndex = searchParams.get('chunk');
  const highlightText = searchParams.get('highlight');
  
  // Pass to ContentRenderer
  return (
    <ContentRenderer 
      document={document}
      highlightText={highlightText ? decodeURIComponent(highlightText) : undefined}
      scrollToChunk={chunkIndex ? parseInt(chunkIndex) : undefined}
    />
  );
}
```

### 5.3 ContentRenderer Enhancement

Add highlight support with scroll-to:

```typescript
interface ContentRendererProps {
  document: Document;
  highlightText?: string;
  scrollToChunk?: number;
}

export function ContentRenderer({ document, highlightText, scrollToChunk }: ContentRendererProps) {
  const contentRef = useRef<HTMLDivElement>(null);
  
  useEffect(() => {
    if (highlightText && contentRef.current) {
      // Find and highlight matching text
      const textNodes = getTextNodes(contentRef.current);
      for (const node of textNodes) {
        if (node.textContent?.includes(highlightText.slice(0, 50))) {
          // Scroll to and highlight
          const range = document.createRange();
          range.selectNodeContents(node);
          const rect = range.getBoundingClientRect();
          window.scrollTo({ top: rect.top - 100, behavior: 'smooth' });
          // Add highlight class
          const mark = document.createElement('mark');
          mark.className = 'bg-yellow-200 dark:bg-yellow-800';
          // ... wrap text
          break;
        }
      }
    }
  }, [highlightText]);
}
```

### 5.4 Source Citations Link Update

```typescript
// In chat-message.tsx
onDocumentClick={(documentId, chunkContent) => {
  const highlight = chunkContent ? encodeURIComponent(chunkContent.slice(0, 100)) : '';
  router.push(`/documents/${documentId}${highlight ? `?highlight=${highlight}` : ''}`);
}}
```

---

## Part 6: Graph Subgraph Filter Implementation

### 6.1 URL Structure

```
/graph?entities={entity1,entity2,...}&focus={mainEntity}
```

### 6.2 Graph Page Changes

**File:** `app/(dashboard)/graph/page.tsx`

```typescript
'use client';

import { useSearchParams } from 'next/navigation';
import { useEffect } from 'react';
import { useGraphStore } from '@/stores/use-graph-store';

export default function GraphPage() {
  const searchParams = useSearchParams();
  const entities = searchParams.get('entities');
  const focus = searchParams.get('focus');
  
  const { setStartNode, setSearchQuery } = useGraphStore();
  
  useEffect(() => {
    if (focus) {
      setStartNode(focus);
    }
    if (entities) {
      // Filter to show only these entities
      const entityList = entities.split(',');
      // Set visible entity filter
    }
  }, [focus, entities, setStartNode]);
  
  return <GraphViewer />;
}
```

### 6.3 ExploreTab Handler

```typescript
// In source-citations.tsx
const ExploreTab = ({
  entities,
  entityCount,
  relationshipCount,
  onExploreGraph,
}: {
  entities: QueryContext['entities'];
  entityCount: number;
  relationshipCount: number;
  onExploreGraph?: (entities: string[]) => void;
}) => {
  const handleExplore = () => {
    const entityLabels = entities?.map(e => e.label) || [];
    onExploreGraph?.(entityLabels);
  };
  
  return (
    <Button onClick={handleExplore}>
      Open Graph Explorer
    </Button>
  );
};
```

### 6.4 Chat Message Handler

```typescript
// In chat-message.tsx
onExploreGraph={(entities) => {
  const entityParam = entities.slice(0, 10).join(',');
  const focus = entities[0] || '';
  router.push(`/graph?entities=${encodeURIComponent(entityParam)}&focus=${encodeURIComponent(focus)}`);
}}
```

---

## Part 7: Test Scenario Design

### 7.1 Synthetic Test Document

Create a document with known entities and relationships:

```markdown
# EdgeQuake Research Document

## Abstract
EdgeQuake is an advanced Retrieval-Augmented Generation (RAG) framework 
designed by **Dr. Sarah Chen** at **Stanford University**.

## Key Components
- **GraphRAG Engine**: Core retrieval system
- **LightRAG Integration**: Performance optimization layer
- **Vector Store**: Embeddings storage using **PostgreSQL AGE**

## Relationships
Dr. Sarah Chen leads the EdgeQuake project.
GraphRAG Engine is part of EdgeQuake.
LightRAG Integration enhances EdgeQuake performance.
PostgreSQL AGE stores EdgeQuake embeddings.

## Performance
EdgeQuake achieves 95% accuracy on benchmark datasets.
It outperforms NaiveRAG by 40% in comprehensive retrieval.
```

### 7.2 Expected Entities

| Entity | Type | Expected Relevance |
|--------|------|-------------------|
| EdgeQuake | PRODUCT | High (0.9+) |
| Dr. Sarah Chen | PERSON | High (0.8+) |
| Stanford University | ORGANIZATION | Medium (0.6+) |
| GraphRAG Engine | CONCEPT | High (0.8+) |
| LightRAG Integration | CONCEPT | Medium (0.7+) |
| PostgreSQL AGE | TECHNOLOGY | Medium (0.6+) |

### 7.3 Expected Chunks

| Chunk | Content | Expected Score |
|-------|---------|----------------|
| 1 | Abstract section | 0.85+ |
| 2 | Key Components section | 0.75+ |
| 3 | Relationships section | 0.80+ |
| 4 | Performance section | 0.70+ |

### 7.4 Test Queries

1. "What is EdgeQuake?" → Should show high confidence (80%+)
2. "Who created EdgeQuake?" → Should highlight Dr. Sarah Chen
3. "How does EdgeQuake relate to LightRAG?" → Should show relationship
4. "What are EdgeQuake's performance metrics?" → Should show Performance section

---

## Part 8: Implementation Priority

### Phase 1: Quick Fixes (1-2 hours)

1. **Fix confidence calculation** - Use chunk scores only, filter zero relevance
2. **Fix document URL** - Change from `/documents?id=` to `/documents/`
3. **Wire onExploreGraph** - Pass handler in chat-message.tsx

### Phase 2: Document Deep Linking (2-3 hours)

1. Add URL param reading to document detail page
2. Implement highlight text scrolling in ContentRenderer
3. Pass chunk content in onDocumentClick callback
4. Add highlight styling (mark element with yellow background)

### Phase 3: Graph Subgraph Filter (2-3 hours)

1. Add URL param reading to graph page
2. Pass entity labels in onExploreGraph callback
3. Pre-set graph store filters from URL params
4. Add "Query Context" indicator in graph UI

### Phase 4: Testing & Polish (2 hours)

1. Create synthetic test document
2. Upload and process
3. Run test queries
4. Verify all links work correctly
5. Take screenshots for documentation

---

## Code Cross-Reference Index

| Feature | Files | Lines |
|---------|-------|-------|
| Confidence Calculation | `source-citations.tsx` | 36-51 |
| Document Click Handler | `chat-message.tsx` | 435-437 |
| Document Detail Page | `documents/[id]/page.tsx` | Full file |
| ContentRenderer | `document/content-renderer.tsx` | 1-244 |
| Graph Page | `graph/page.tsx` | Full file |
| Graph Store | `stores/use-graph-store.ts` | 111-133 |
| GraphViewer | `components/graph/graph-viewer.tsx` | 1-765 |
| ExploreTab | `source-citations.tsx` | 354-391 |
| QueryContext Type | `types/index.ts` | 245-274 |

---

## Success Metrics

| Metric | Current | Target |
|--------|---------|--------|
| Confidence accuracy | 4-5% (broken) | 70-95% (accurate) |
| Document link works | ❌ Wrong URL | ✅ Opens detail page |
| Chunk highlighting | ❌ None | ✅ Yellow highlight + scroll |
| Graph filter | ❌ Not wired | ✅ Shows query subgraph |
| Entity click | ⚠️ Partial | ✅ Filters to entity |

---

**Document Status:** ✅ Complete  
**Ready for Implementation:** Yes
