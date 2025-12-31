# Source Citations Implementation Plan

**Document ID:** 25  
**Date:** 2025-12-31  
**Version:** 1.0  
**Status:** Ready for Implementation

---

## Implementation Overview

This document provides a precision implementation plan for transforming the source citations component from a technical, developer-centric display to a SOTA, business-friendly, slick citations panel.

**Target Files:**

- `edgequake_webui/src/components/query/source-citations.tsx` (main component)
- `edgequake_webui/src/types/index.ts` (type definitions)
- `edgequake/crates/edgequake-api/src/handlers/chat.rs` (backend metadata)

**Estimated Total Time:** 12-16 hours  
**Phases:** 4

---

## Phase 1: Quick Wins (2-3 hours)

### 1.1 Terminology Updates

**File:** `source-citations.tsx`

**Change 1: Summary Bar Text**

```diff
- <span className="text-xs">
-   Sources: {context.chunks?.length || 0} chunks · {context.entities?.length || 0} entities
- </span>
+ <span className="text-xs">
+   {context.chunks?.length || 0} Sources · {context.entities?.length || 0} Topics
+ </span>
```

**Change 2: Section Headers**

```diff
- <h4 className="text-xs font-medium text-muted-foreground uppercase tracking-wide">
-   Source Documents
- </h4>
+ <h4 className="text-xs font-medium text-muted-foreground tracking-wide">
+   Documents
+ </h4>

- <h4 className="text-xs font-medium text-muted-foreground uppercase tracking-wide">
-   Related Entities
- </h4>
+ <h4 className="text-xs font-medium text-muted-foreground tracking-wide">
+   Key Topics
+ </h4>

- <h4 className="text-xs font-medium text-muted-foreground uppercase tracking-wide">
-   Key Relationships
- </h4>
+ <h4 className="text-xs font-medium text-muted-foreground tracking-wide">
+   Connections
+ </h4>
```

**Change 3: "More" Labels**

```diff
- <p className="text-[10px] text-muted-foreground">
-   +{chunks.length - 2} more chunks
- </p>
+ <p className="text-xs text-muted-foreground cursor-pointer hover:text-foreground">
+   Show {chunks.length - 2} more sources
+ </p>
```

### 1.2 Add Confidence Badge

**Add Import:**

```typescript
import {
  BookOpen,
  ChevronDown,
  ChevronUp,
  ExternalLink,
  FileText,
  Brain,
  Network,
} from "lucide-react";
```

**Add Helper Functions:**

```typescript
// Add after interface definitions
const calculateConfidence = (context: QueryContext): number => {
  const scores = [
    ...(context.chunks?.map((c) => c.score) || []),
    ...(context.entities?.map((e) => e.relevance) || []),
    ...(context.relationships?.map((r) => r.relevance) || []),
  ];
  if (scores.length === 0) return 0;
  return scores.reduce((a, b) => a + b, 0) / scores.length;
};

const getConfidenceLabel = (
  score: number
): { label: string; color: string } => {
  if (score >= 0.8) return { label: "High", color: "text-green-600" };
  if (score >= 0.6) return { label: "Good", color: "text-green-500" };
  if (score >= 0.4) return { label: "Medium", color: "text-amber-500" };
  return { label: "Low", color: "text-red-500" };
};

const ConfidenceDots = ({ score }: { score: number }) => {
  const filled = Math.round(score * 5);
  return (
    <span
      className="inline-flex gap-0.5"
      title={`${Math.round(score * 100)}% confidence`}
    >
      {[...Array(5)].map((_, i) => (
        <span
          key={i}
          className={`w-1.5 h-1.5 rounded-full ${
            i < filled ? "bg-current" : "bg-muted-foreground/30"
          }`}
        />
      ))}
    </span>
  );
};
```

**Update Summary Bar:**

```typescript
// Inside SourceCitations component, add:
const confidence = calculateConfidence(context);
const { label: confidenceLabel, color: confidenceColor } =
  getConfidenceLabel(confidence);

// Update the button content:
<Button variant="ghost" size="sm" className="w-full ...">
  <span className="flex items-center gap-2">
    <BookOpen className="h-3.5 w-3.5" />
    <span className="text-xs">
      {context.chunks?.length || 0} Sources · {context.entities?.length || 0}{" "}
      Topics
    </span>
    <span className={`text-xs flex items-center gap-1 ${confidenceColor}`}>
      <ConfidenceDots score={confidence} />
      <span>
        {confidenceLabel} ({Math.round(confidence * 100)}%)
      </span>
    </span>
  </span>
  ...
</Button>;
```

### 1.3 Add Citation Numbers

**Update Document Cards:**

```typescript
{
  Object.entries(chunksByDocument).map(([docId, chunks], index) => (
    <Card key={docId} className="bg-muted/30">
      <CardContent className="p-3">
        <div className="flex items-start gap-2">
          {/* Citation number */}
          <span className="flex-shrink-0 w-5 h-5 rounded-full bg-primary text-primary-foreground text-xs flex items-center justify-center font-semibold">
            {index + 1}
          </span>

          <div className="flex-1 min-w-0">
            <div className="flex items-center justify-between mb-1">
              <button
                onClick={() => onDocumentClick?.(docId)}
                className="text-sm font-medium truncate max-w-[200px] hover:text-primary flex items-center gap-1"
              >
                {/* TODO: Show document title instead of UUID */}
                {docId.slice(0, 8)}...
                <ExternalLink className="h-3 w-3" />
              </button>
              <Badge variant="secondary" className="text-[10px]">
                {chunks.length} source{chunks.length !== 1 ? "s" : ""}
              </Badge>
            </div>
            ...
          </div>
        </div>
      </CardContent>
    </Card>
  ));
}
```

---

## Phase 2: Layout Redesign - Tabs (4-6 hours)

### 2.1 Add Tab Components

**Add Import:**

```typescript
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
```

### 2.2 Restructure Component

**Complete Rewrite of Main Component:**

```typescript
"use client";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import {
  Collapsible,
  CollapsibleContent,
  CollapsibleTrigger,
} from "@/components/ui/collapsible";
import {
  HoverCard,
  HoverCardContent,
  HoverCardTrigger,
} from "@/components/ui/hover-card";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import type { QueryContext } from "@/types";
import {
  BookOpen,
  Brain,
  ChevronDown,
  ChevronUp,
  ExternalLink,
  FileText,
  Network,
} from "lucide-react";
import { useMemo, useState } from "react";

// Types
interface SourceCitationsProps {
  context: QueryContext;
  onEntityClick?: (entityId: string) => void;
  onDocumentClick?: (documentId: string) => void;
  onExploreGraph?: () => void;
}

// Helpers
const calculateConfidence = (context: QueryContext): number => {
  const scores = [
    ...(context.chunks?.map((c) => c.score) || []),
    ...(context.entities?.map((e) => e.relevance) || []),
    ...(context.relationships?.map((r) => r.relevance) || []),
  ];
  if (scores.length === 0) return 0;
  return scores.reduce((a, b) => a + b, 0) / scores.length;
};

const getConfidenceLabel = (
  score: number
): { label: string; color: string } => {
  if (score >= 0.8) return { label: "High", color: "text-green-600" };
  if (score >= 0.6) return { label: "Good", color: "text-green-500" };
  if (score >= 0.4) return { label: "Medium", color: "text-amber-500" };
  return { label: "Low", color: "text-red-500" };
};

// Sub-components
const ConfidenceDots = ({ score }: { score: number }) => {
  const filled = Math.round(score * 5);
  return (
    <span
      className="inline-flex gap-0.5"
      title={`${Math.round(score * 100)}% confidence`}
    >
      {[...Array(5)].map((_, i) => (
        <span
          key={i}
          className={`w-1.5 h-1.5 rounded-full ${
            i < filled ? "bg-current" : "bg-muted-foreground/30"
          }`}
        />
      ))}
    </span>
  );
};

// Documents Tab Component
const DocumentsTab = ({
  chunksByDocument,
  onDocumentClick,
}: {
  chunksByDocument: Record<string, NonNullable<QueryContext["chunks"]>>;
  onDocumentClick?: (docId: string) => void;
}) => {
  const [showAll, setShowAll] = useState(false);
  const entries = Object.entries(chunksByDocument);
  const visibleEntries = showAll ? entries : entries.slice(0, 3);

  return (
    <div className="space-y-2">
      <ScrollArea className="max-h-[280px]">
        <div className="space-y-2 pr-2">
          {visibleEntries.map(([docId, chunks], index) => (
            <Card
              key={docId}
              className="bg-muted/30 hover:bg-muted/50 transition-colors"
            >
              <CardContent className="p-3">
                <div className="flex items-start gap-3">
                  {/* Citation number */}
                  <span className="flex-shrink-0 w-6 h-6 rounded-full bg-primary text-primary-foreground text-xs flex items-center justify-center font-semibold">
                    {index + 1}
                  </span>

                  <div className="flex-1 min-w-0">
                    <div className="flex items-center justify-between gap-2 mb-1">
                      <span className="text-sm font-medium truncate">
                        {/* TODO: Replace with document title from metadata */}
                        Document {docId.slice(0, 8)}
                      </span>
                      <div className="flex items-center gap-2 flex-shrink-0">
                        <span
                          className={`text-xs font-medium ${
                            chunks[0].score >= 0.8
                              ? "text-green-600"
                              : chunks[0].score >= 0.5
                              ? "text-amber-500"
                              : "text-red-500"
                          }`}
                        >
                          {Math.round(chunks[0].score * 100)}%
                        </span>
                        <Button
                          variant="ghost"
                          size="sm"
                          className="h-6 w-6 p-0"
                          onClick={() => onDocumentClick?.(docId)}
                        >
                          <ExternalLink className="h-3.5 w-3.5" />
                        </Button>
                      </div>
                    </div>

                    <p className="text-xs text-muted-foreground mb-1">
                      {docId.slice(0, 12)}...
                    </p>

                    <p className="text-xs text-muted-foreground line-clamp-2">
                      "{chunks[0].content.slice(0, 150)}..."
                    </p>
                  </div>
                </div>
              </CardContent>
            </Card>
          ))}
        </div>
      </ScrollArea>

      {entries.length > 3 && !showAll && (
        <Button
          variant="ghost"
          size="sm"
          className="w-full text-xs text-muted-foreground hover:text-foreground"
          onClick={() => setShowAll(true)}
        >
          Show {entries.length - 3} more sources
          <ChevronDown className="h-3 w-3 ml-1" />
        </Button>
      )}
    </div>
  );
};

// Knowledge Tab Component
const KnowledgeTab = ({
  entities,
  relationships,
  onEntityClick,
}: {
  entities: QueryContext["entities"];
  relationships: QueryContext["relationships"];
  onEntityClick?: (entityId: string) => void;
}) => {
  const [showAllEntities, setShowAllEntities] = useState(false);
  const visibleEntities = showAllEntities ? entities : entities?.slice(0, 8);

  return (
    <div className="space-y-4">
      {/* Entities */}
      {entities && entities.length > 0 && (
        <div className="space-y-2">
          <h4 className="text-xs font-medium text-muted-foreground">
            Key Topics
          </h4>
          <div className="flex flex-wrap gap-1.5">
            {visibleEntities?.map((entity) => (
              <Badge
                key={entity.id}
                variant="secondary"
                className="cursor-pointer hover:bg-primary/10 hover:text-primary transition-colors text-xs py-1"
                onClick={() => onEntityClick?.(entity.id)}
              >
                {entity.label}
              </Badge>
            ))}
            {entities.length > 8 && !showAllEntities && (
              <Badge
                variant="outline"
                className="cursor-pointer hover:bg-muted text-xs py-1"
                onClick={() => setShowAllEntities(true)}
              >
                +{entities.length - 8} more
              </Badge>
            )}
          </div>
        </div>
      )}

      {/* Relationships */}
      {relationships && relationships.length > 0 && (
        <div className="space-y-2">
          <h4 className="text-xs font-medium text-muted-foreground">
            Connections
          </h4>
          <div className="space-y-1.5">
            {relationships.slice(0, 5).map((rel, idx) => (
              <div
                key={idx}
                className="flex items-center gap-1.5 text-xs p-1.5 rounded hover:bg-muted/50 transition-colors"
              >
                <span
                  className="font-medium cursor-pointer hover:text-primary truncate max-w-[120px]"
                  onClick={() => onEntityClick?.(rel.source)}
                >
                  {rel.source}
                </span>
                <span className="text-primary">→</span>
                <Badge variant="outline" className="text-[10px] px-1.5">
                  {rel.type.toLowerCase().replace(/_/g, " ")}
                </Badge>
                <span className="text-primary">→</span>
                <span
                  className="font-medium cursor-pointer hover:text-primary truncate max-w-[120px]"
                  onClick={() => onEntityClick?.(rel.target)}
                >
                  {rel.target}
                </span>
              </div>
            ))}
            {relationships.length > 5 && (
              <p className="text-xs text-muted-foreground pl-1.5">
                +{relationships.length - 5} more connections
              </p>
            )}
          </div>
        </div>
      )}
    </div>
  );
};

// Explore Tab Component
const ExploreTab = ({
  entityCount,
  relationshipCount,
  onExploreGraph,
}: {
  entityCount: number;
  relationshipCount: number;
  onExploreGraph?: () => void;
}) => (
  <div className="flex flex-col items-center justify-center py-6 space-y-4">
    <div className="w-24 h-24 rounded-full bg-muted/50 flex items-center justify-center">
      <Network className="h-10 w-10 text-muted-foreground" />
    </div>
    <div className="text-center space-y-1">
      <p className="text-sm font-medium">Explore Knowledge Graph</p>
      <p className="text-xs text-muted-foreground">
        {entityCount} topics and {relationshipCount} connections
      </p>
    </div>
    <Button onClick={onExploreGraph} className="gap-2">
      <Network className="h-4 w-4" />
      Open Graph Explorer
    </Button>
  </div>
);

// Main Component
export function SourceCitations({
  context,
  onEntityClick,
  onDocumentClick,
  onExploreGraph,
}: SourceCitationsProps) {
  const [isExpanded, setIsExpanded] = useState(false);

  // Memoized calculations
  const hasChunks = context.chunks && context.chunks.length > 0;
  const hasEntities = context.entities && context.entities.length > 0;
  const hasRelationships =
    context.relationships && context.relationships.length > 0;

  const chunksByDocument = useMemo(
    () =>
      context.chunks?.reduce((acc, chunk) => {
        if (!acc[chunk.document_id]) {
          acc[chunk.document_id] = [];
        }
        acc[chunk.document_id].push(chunk);
        return acc;
      }, {} as Record<string, NonNullable<typeof context.chunks>>) || {},
    [context.chunks]
  );

  const confidence = useMemo(() => calculateConfidence(context), [context]);
  const { label: confidenceLabel, color: confidenceColor } =
    getConfidenceLabel(confidence);

  if (!hasChunks && !hasEntities && !hasRelationships) {
    return null;
  }

  return (
    <Collapsible open={isExpanded} onOpenChange={setIsExpanded}>
      <CollapsibleTrigger asChild>
        <Button
          variant="ghost"
          size="sm"
          className="w-full flex items-center justify-between text-muted-foreground hover:text-foreground py-2"
        >
          <span className="flex items-center gap-2">
            <BookOpen className="h-4 w-4" />
            <span className="text-xs font-medium">
              {context.chunks?.length || 0} Sources ·{" "}
              {context.entities?.length || 0} Topics
            </span>
            <span
              className={`text-xs flex items-center gap-1.5 ${confidenceColor}`}
            >
              <ConfidenceDots score={confidence} />
              <span className="font-medium">
                {confidenceLabel} ({Math.round(confidence * 100)}%)
              </span>
            </span>
          </span>
          {isExpanded ? (
            <ChevronUp className="h-4 w-4" />
          ) : (
            <ChevronDown className="h-4 w-4" />
          )}
        </Button>
      </CollapsibleTrigger>

      <CollapsibleContent className="mt-2">
        <Card className="border-muted">
          <CardContent className="p-3">
            <Tabs defaultValue="documents" className="w-full">
              <TabsList className="grid w-full grid-cols-3 h-8">
                <TabsTrigger value="documents" className="text-xs gap-1.5">
                  <FileText className="h-3 w-3" />
                  Documents
                </TabsTrigger>
                <TabsTrigger value="knowledge" className="text-xs gap-1.5">
                  <Brain className="h-3 w-3" />
                  Knowledge
                </TabsTrigger>
                <TabsTrigger value="explore" className="text-xs gap-1.5">
                  <Network className="h-3 w-3" />
                  Explore
                </TabsTrigger>
              </TabsList>

              <TabsContent value="documents" className="mt-3">
                <DocumentsTab
                  chunksByDocument={chunksByDocument}
                  onDocumentClick={onDocumentClick}
                />
              </TabsContent>

              <TabsContent value="knowledge" className="mt-3">
                <KnowledgeTab
                  entities={context.entities}
                  relationships={context.relationships}
                  onEntityClick={onEntityClick}
                />
              </TabsContent>

              <TabsContent value="explore" className="mt-3">
                <ExploreTab
                  entityCount={context.entities?.length || 0}
                  relationshipCount={context.relationships?.length || 0}
                  onExploreGraph={onExploreGraph}
                />
              </TabsContent>
            </Tabs>
          </CardContent>
        </Card>
      </CollapsibleContent>
    </Collapsible>
  );
}

export default SourceCitations;
```

---

## Phase 3: Backend Document Metadata (2-3 hours)

### 3.1 Update SourceReference Type

**File:** `edgequake/crates/edgequake-api/src/types/mod.rs`

Add fields to `SourceReference`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceReference {
    pub source_type: SourceType,
    pub id: String,
    pub content: Option<String>,
    pub score: f32,
    pub document_id: Option<String>,
    pub document_title: Option<String>,  // NEW
    pub file_name: Option<String>,       // NEW
    pub chunk_index: Option<usize>,
    pub entity_type: Option<String>,
    pub relationship_type: Option<String>,
    pub source_node: Option<String>,
    pub target_node: Option<String>,
}
```

### 3.2 Populate Metadata in Chat Handler

**File:** `edgequake/crates/edgequake-api/src/handlers/chat.rs`

In the context building section, fetch document metadata:

```rust
// When building SourceReference for chunks
for chunk in chunks {
    // Fetch document metadata
    let doc_metadata = kv_store
        .get(&format!("{}-metadata", chunk.document_id))
        .await
        .ok()
        .flatten()
        .and_then(|v| serde_json::from_slice::<serde_json::Value>(&v).ok());

    let document_title = doc_metadata
        .as_ref()
        .and_then(|m| m.get("title"))
        .and_then(|t| t.as_str())
        .map(String::from);

    sources.push(SourceReference {
        source_type: SourceType::Chunk,
        id: chunk.id.clone(),
        content: Some(chunk.content.clone()),
        score: chunk.score,
        document_id: Some(chunk.document_id.clone()),
        document_title,  // NEW
        file_name: None, // Can be extracted from path if available
        chunk_index: chunk.chunk_index,
        ..Default::default()
    });
}
```

### 3.3 Update Frontend Types

**File:** `edgequake_webui/src/types/index.ts`

```typescript
export interface ChunkReference {
  content: string;
  document_id: string;
  document_title?: string; // NEW
  file_name?: string; // NEW
  score: number;
  chunk_index?: number;
}
```

### 3.4 Use Title in UI

Update `DocumentsTab` component:

```typescript
<span className="text-sm font-medium truncate">
  {chunks[0].document_title || `Document ${docId.slice(0, 8)}`}
</span>
```

---

## Phase 4: Polish & Accessibility (2-3 hours)

### 4.1 Add ARIA Labels

```typescript
<Collapsible open={isExpanded} onOpenChange={setIsExpanded}>
  <CollapsibleTrigger asChild>
    <Button
      aria-expanded={isExpanded}
      aria-controls="source-citations-content"
      aria-label={`Source citations: ${context.chunks?.length || 0} sources, ${context.entities?.length || 0} topics, ${confidenceLabel} confidence`}
      ...
    >
```

### 4.2 Add Focus Styles

```typescript
<Badge
  className="cursor-pointer hover:bg-primary/10 hover:text-primary focus:ring-2 focus:ring-primary focus:ring-offset-2 transition-colors"
  tabIndex={0}
  onKeyDown={(e) => {
    if (e.key === 'Enter' || e.key === ' ') {
      onEntityClick?.(entity.id);
    }
  }}
>
```

### 4.3 Add Skeleton Loading State

```typescript
const SourceCitationsSkeleton = () => (
  <div className="w-full animate-pulse">
    <div className="flex items-center gap-2 py-2">
      <div className="h-4 w-4 rounded bg-muted" />
      <div className="h-3 w-32 rounded bg-muted" />
      <div className="h-3 w-24 rounded bg-muted" />
    </div>
  </div>
);
```

### 4.4 Add Animation

```css
/* Add to global.css or tailwind config */
@keyframes fade-in-up {
  from {
    opacity: 0;
    transform: translateY(8px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}

.animate-fade-in-up {
  animation: fade-in-up 0.25s ease-out;
}
```

```typescript
<CollapsibleContent className="mt-2 animate-fade-in-up">
```

### 4.5 Mobile Responsive Fixes

```typescript
// In TabsList
<TabsList className="grid w-full grid-cols-3 h-8 sm:h-9">
  <TabsTrigger value="documents" className="text-xs gap-1 sm:gap-1.5">
    <FileText className="h-3 w-3 sm:h-3.5 sm:w-3.5" />
    <span className="hidden sm:inline">Documents</span>
    <span className="sm:hidden">Docs</span>
  </TabsTrigger>
  ...
</TabsList>

// In entity badges on mobile
<div className="flex flex-wrap gap-1 sm:gap-1.5">
```

---

## Testing Checklist

### Unit Tests

```typescript
// source-citations.test.tsx
describe("SourceCitations", () => {
  it("renders summary bar with correct counts", () => {});
  it("calculates confidence correctly", () => {});
  it("expands and collapses on click", () => {});
  it("switches tabs correctly", () => {});
  it("calls onDocumentClick when document clicked", () => {});
  it("calls onEntityClick when entity clicked", () => {});
  it("renders nothing when no context", () => {});
  it("shows correct confidence color", () => {});
});
```

### E2E Tests

```typescript
// e2e/source-citations.spec.ts
test("source citations display and interact", async ({ page }) => {
  // Query with sources
  await page.fill('[data-testid="query-input"]', "Tell me about MegaRAG");
  await page.click('[data-testid="send-button"]');

  // Wait for response
  await page.waitForSelector('[data-testid="source-citations"]');

  // Check summary
  await expect(page.locator("text=/\\d+ Sources/")).toBeVisible();

  // Expand
  await page.click('[data-testid="source-citations-trigger"]');

  // Check tabs
  await expect(page.locator('role=tab[name="Documents"]')).toBeVisible();
  await expect(page.locator('role=tab[name="Knowledge"]')).toBeVisible();

  // Switch to Knowledge tab
  await page.click('role=tab[name="Knowledge"]');
  await expect(page.locator("text=Key Topics")).toBeVisible();
});
```

---

## File Changes Summary

| File                        | Action  | Lines Changed |
| --------------------------- | ------- | ------------- |
| `source-citations.tsx`      | Replace | ~400 lines    |
| `types/index.ts`            | Modify  | +5 lines      |
| `chat.rs`                   | Modify  | +15 lines     |
| `global.css`                | Add     | +10 lines     |
| `source-citations.test.tsx` | Create  | ~150 lines    |

---

## Rollout Plan

1. **Day 1**: Phase 1 - Quick wins (terminology, confidence)
2. **Day 2**: Phase 2 - Tab layout restructure
3. **Day 3**: Phase 3 - Backend metadata
4. **Day 4**: Phase 4 - Polish & testing
5. **Day 5**: QA & deployment

---

## Success Metrics

| Metric                | Before | Target                  |
| --------------------- | ------ | ----------------------- |
| User comprehension    | N/A    | >90% understand sources |
| Click-through to docs | N/A    | >20%                    |
| Tab engagement        | N/A    | >50% click Knowledge    |
| Mobile usability      | Poor   | Excellent               |
| Accessibility score   | ~60%   | >95%                    |

---

## Appendix: Component Dependency Tree

```
SourceCitations
├── ConfidenceDots
├── DocumentsTab
│   └── Card (source card)
├── KnowledgeTab
│   ├── Badge (entity chip)
│   └── RelationshipRow
└── ExploreTab
    └── Button (explore action)
```

---

**Document Status:** ✅ Complete  
**Ready for Implementation:** Yes  
**Estimated Total Time:** 12-16 hours
