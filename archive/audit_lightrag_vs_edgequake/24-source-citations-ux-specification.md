# Source Citations UX/UI Specification

**Document ID:** 24  
**Date:** 2025-12-31  
**Version:** 1.0  
**Status:** Design Complete

---

## 1. Design Vision

### 1.1 Design Principles

| Principle                   | Description                                  |
| --------------------------- | -------------------------------------------- |
| **Progressive Disclosure**  | Show summary first, reveal details on demand |
| **Business Language**       | Replace technical jargon with business terms |
| **Visual Hierarchy**        | Clear primary, secondary, tertiary content   |
| **Confidence First**        | Lead with confidence/relevance indicators    |
| **Document Attribution**    | Emphasize source provenance                  |
| **Knowledge Graph Glimpse** | Preview entity/relationship structure        |
| **SOTA Aesthetics**         | Clean, minimal, modern UI                    |

### 1.2 Target User Personas

| Persona              | Needs                                        |
| -------------------- | -------------------------------------------- |
| **Business Analyst** | Quick confidence check, document names       |
| **Researcher**       | Deep dive into sources, entity relationships |
| **Executive**        | Summary only, at-a-glance verification       |
| **Developer**        | Full technical details when needed           |

---

## 2. Visual Design Specification

### 2.1 Summary Bar (Collapsed State)

```
┌─────────────────────────────────────────────────────────────────────┐
│ 📚 7 Sources · 22 Topics · ●●●●○ High Confidence (91%)    [Expand ▼]│
└─────────────────────────────────────────────────────────────────────┘
```

**Components:**

| Element          | Spec                                              |
| ---------------- | ------------------------------------------------- |
| Icon             | `BookOpen` (lucide-react), 14px, muted-foreground |
| Source Count     | `{count} Sources` - bold weight                   |
| Topic Count      | `{count} Topics` - regular weight                 |
| Confidence Dots  | 5 dots (●○), filled based on percentage           |
| Confidence Label | "High/Medium/Low Confidence (N%)"                 |
| Expand Button    | Chevron icon, rotates on expand                   |

**Color Mapping:**

```css
/* Confidence thresholds */
--confidence-high: ≥80% → #22c55e (green-500)
--confidence-medium: 50-79% → #f59e0b (amber-500)
--confidence-low: <50% → #ef4444 (red-500)
```

### 2.2 Expanded Panel - Tabbed Layout

```
┌─────────────────────────────────────────────────────────────────────┐
│ 📚 7 Sources · 22 Topics · ●●●●○ High Confidence (91%)    [Collapse ▲]│
├─────────────────────────────────────────────────────────────────────┤
│ [◉ Documents]  [◯ Knowledge]  [◯ Explore Graph]                     │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│                         TAB CONTENT AREA                            │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

**Tab Specifications:**

| Tab       | Icon       | Description                    |
| --------- | ---------- | ------------------------------ |
| Documents | `FileText` | Source documents with excerpts |
| Knowledge | `Brain`    | Entities and relationships     |
| Explore   | `Network`  | Link to full graph view        |

### 2.3 Documents Tab Content

```
┌─────────────────────────────────────────────────────────────────────┐
│ [¹] MegaRAG: Multimodal Knowledge Graph                       98% ↗│
│     mega_rag_2512.20626v1.extracted.md                             │
│     "Multimodal Knowledge Graph-Based Retrieval Augmented          │
│     Generation system that integrates visual and textual..."       │
├─────────────────────────────────────────────────────────────────────┤
│ [²] LightRAG Architecture Overview                            95% ↗│
│     lightrag_architecture.md                                       │
│     "Graph-based retrieval system utilizing vector embeddings      │
│     for efficient knowledge graph traversal..."                    │
├─────────────────────────────────────────────────────────────────────┤
│ [³] Neural Network Approaches                                 87% ↗│
│     one_tool_2512.20957v2.extracted.md                             │
│     "Reinforcement Learning for Repository-Level LLM Agents..."   │
├─────────────────────────────────────────────────────────────────────┤
│                      [Show 4 more sources ▼]                       │
└─────────────────────────────────────────────────────────────────────┘
```

**Source Card Anatomy:**

```
┌──────────────────────────────────────────────────────────────────┐
│ [N] Document Title (from metadata)              [Confidence%] [↗]│
│     filename.ext                                                 │
│     "First 150 characters of most relevant excerpt with         │
│     ellipsis if truncated..."                                    │
└──────────────────────────────────────────────────────────────────┘
```

| Element         | Typography              | Color                                         |
| --------------- | ----------------------- | --------------------------------------------- |
| Citation Number | `text-sm font-semibold` | `bg-primary text-primary-foreground` (circle) |
| Document Title  | `text-sm font-medium`   | `foreground`                                  |
| Filename        | `text-xs`               | `muted-foreground`                            |
| Excerpt         | `text-xs line-clamp-2`  | `muted-foreground`                            |
| Confidence      | `text-xs font-medium`   | Semantic color (green/amber/red)              |
| External Link   | `ExternalLink` icon     | `primary` on hover                            |

### 2.4 Knowledge Tab Content

```
┌─────────────────────────────────────────────────────────────────────┐
│ KEY TOPICS                                                          │
│ ┌─────────────────────────────────────────────────────────────────┐ │
│ │ [MEGARAG] [KNOWLEDGE GRAPH] [LIGHTRAG] [RAG FRAMEWORK]          │ │
│ │ [TEXT EMBEDDING] [VECTOR DATABASE] [+8 more]                    │ │
│ └─────────────────────────────────────────────────────────────────┘ │
│                                                                     │
│ CONNECTIONS                                                         │
│ ┌─────────────────────────────────────────────────────────────────┐ │
│ │            ┌──────────────┐                                     │ │
│ │            │   MEGARAG    │                                     │ │
│ │            └──────┬───────┘                                     │ │
│ │                   │ integrates                                  │ │
│ │                   ▼                                             │ │
│ │  ┌────────────────────────────────────┐                         │ │
│ │  │        KNOWLEDGE GRAPH             │                         │ │
│ │  └────────────────────────────────────┘                         │ │
│ │                   ▲                                             │ │
│ │                   │ uses                                        │ │
│ │            ┌──────┴───────┐                                     │ │
│ │            │   LIGHTRAG   │                                     │ │
│ │            └──────────────┘                                     │ │
│ └─────────────────────────────────────────────────────────────────┘ │
│                                                                     │
│ [View all 22 topics →]  [Explore 14 connections →]                  │
└─────────────────────────────────────────────────────────────────────┘
```

**Entity Chip Specifications:**

| State   | Background      | Text                 | Border           |
| ------- | --------------- | -------------------- | ---------------- |
| Default | `bg-secondary`  | `foreground`         | none             |
| Hover   | `bg-primary/10` | `primary`            | `border-primary` |
| Active  | `bg-primary`    | `primary-foreground` | none             |

### 2.5 Explore Tab Content

```
┌─────────────────────────────────────────────────────────────────────┐
│                                                                     │
│                    ┌─────────────────────────┐                      │
│                    │                         │                      │
│                    │   [Mini Graph Preview]  │                      │
│                    │    (First 10 nodes)     │                      │
│                    │                         │                      │
│                    └─────────────────────────┘                      │
│                                                                     │
│           ┌──────────────────────────────────────────┐              │
│           │  Open Knowledge Graph Explorer      [↗]  │              │
│           │  Explore all 22 entities and 14          │              │
│           │  relationships in full interactive view  │              │
│           └──────────────────────────────────────────┘              │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

---

## 3. Component Specifications

### 3.1 SourceCitationsPanel

```typescript
interface SourceCitationsPanelProps {
  context: QueryContext;
  messageId: string;
  onDocumentClick?: (documentId: string) => void;
  onEntityClick?: (entityId: string) => void;
  onExploreGraph?: () => void;
  defaultExpanded?: boolean;
}
```

### 3.2 CitationsSummaryBar

```typescript
interface CitationsSummaryBarProps {
  sourceCount: number;
  entityCount: number;
  averageConfidence: number;
  isExpanded: boolean;
  onToggle: () => void;
}

// Confidence calculation
const calculateAverageConfidence = (context: QueryContext): number => {
  const scores = [
    ...(context.chunks?.map((c) => c.score) || []),
    ...(context.entities?.map((e) => e.relevance) || []),
    ...(context.relationships?.map((r) => r.relevance) || []),
  ];
  if (scores.length === 0) return 0;
  return scores.reduce((a, b) => a + b, 0) / scores.length;
};

// Confidence label
const getConfidenceLabel = (score: number): string => {
  if (score >= 0.8) return "High";
  if (score >= 0.5) return "Medium";
  return "Low";
};
```

### 3.3 SourceDocumentCard

```typescript
interface SourceDocumentCardProps {
  citationNumber: number;
  documentId: string;
  documentTitle: string; // From metadata, not UUID
  fileName?: string;
  excerpt: string;
  confidence: number;
  chunkCount: number;
  onView: () => void;
}

// Card states
type CardState = "default" | "hover" | "focused" | "active";
```

### 3.4 EntityChip

```typescript
interface EntityChipProps {
  id: string;
  label: string;
  relevance: number;
  sourceDocumentId?: string;
  onClick?: () => void;
}

// Visual variants based on entity type (if available)
const entityTypeColors: Record<string, string> = {
  PERSON: "bg-blue-100 text-blue-800",
  ORGANIZATION: "bg-purple-100 text-purple-800",
  TECHNOLOGY: "bg-green-100 text-green-800",
  CONCEPT: "bg-amber-100 text-amber-800",
  DEFAULT: "bg-secondary text-foreground",
};
```

### 3.5 RelationshipMiniGraph

```typescript
interface RelationshipMiniGraphProps {
  relationships: RelationshipReference[];
  maxNodes?: number; // Default: 8
  maxEdges?: number; // Default: 6
  onExpand?: () => void;
}

// Layout algorithm: Simple force-directed with constraints
// - Center most-connected node
// - Radial layout for connections
// - No overlapping labels
```

---

## 4. Interaction Specifications

### 4.1 Expand/Collapse Animation

```css
/* Smooth expand animation */
.citations-content {
  transition: max-height 0.3s ease-out, opacity 0.2s ease-out;
}

.citations-content[data-state="closed"] {
  max-height: 0;
  opacity: 0;
}

.citations-content[data-state="open"] {
  max-height: 500px;
  opacity: 1;
}
```

### 4.2 Tab Switching

- **Animation:** Fade + slide (200ms)
- **Default Tab:** Documents (most business-relevant)
- **Persist Selection:** Per message (in component state)

### 4.3 Document Card Hover

| Timing | Effect                             |
| ------ | ---------------------------------- |
| 0ms    | Cursor enters card                 |
| 100ms  | Background shifts to `bg-muted/50` |
| 100ms  | External link icon appears         |
| 150ms  | Subtle scale (1.01)                |

### 4.4 Entity Chip Click

1. Click entity chip
2. If `onEntityClick` provided: Navigate to graph with entity highlighted
3. If not: Show inline tooltip with entity details

### 4.5 Mobile Touch Interactions

| Gesture           | Action                      |
| ----------------- | --------------------------- |
| Tap summary bar   | Toggle expand               |
| Tap document card | Show full excerpt (sheet)   |
| Tap entity chip   | Show entity details (sheet) |
| Swipe tabs        | Switch active tab           |

---

## 5. Accessibility Specifications

### 5.1 ARIA Attributes

```html
<div role="region" aria-label="Source citations">
  <button aria-expanded="true/false" aria-controls="citations-content">
    Summary bar
  </button>

  <div id="citations-content" role="tabpanel" aria-labelledby="documents-tab">
    Content
  </div>
</div>
```

### 5.2 Keyboard Navigation

| Key           | Action                                  |
| ------------- | --------------------------------------- |
| `Enter/Space` | Toggle expand, activate tab, click card |
| `Tab`         | Move focus to next interactive element  |
| `←/→`         | Switch between tabs                     |
| `Escape`      | Collapse panel if expanded              |

### 5.3 Screen Reader Announcements

```typescript
// On expand
"Source citations expanded. 7 sources, 22 topics, high confidence.";

// On tab switch
"Documents tab selected. Showing 7 source documents.";

// On document focus
"Source 1 of 7: MegaRAG Multimodal Knowledge Graph. 98% confidence. Press Enter to view document.";
```

---

## 6. Responsive Design

### 6.1 Breakpoints

| Breakpoint | Width      | Layout Changes                                      |
| ---------- | ---------- | --------------------------------------------------- |
| Mobile     | <640px     | Full-width cards, vertical tabs, sheets for details |
| Tablet     | 640-1024px | 2-column entity grid, horizontal tabs               |
| Desktop    | >1024px    | Full layout as specified                            |

### 6.2 Mobile Layout

```
┌─────────────────────────┐
│ 📚 7 Sources · 22 Topics│
│ ●●●●○ High (91%)   [▼]  │
├─────────────────────────┤
│ [Docs] [Topics] [Graph] │
├─────────────────────────┤
│ ┌─────────────────────┐ │
│ │[1] MegaRAG      98%↗│ │
│ │mega_rag.md          │ │
│ │"Multimodal KG..."   │ │
│ └─────────────────────┘ │
│ ┌─────────────────────┐ │
│ │[2] LightRAG     95%↗│ │
│ │lightrag.md          │ │
│ │"Graph-based..."     │ │
│ └─────────────────────┘ │
│    [Show 5 more ▼]     │
└─────────────────────────┘
```

---

## 7. Document Title Resolution

### 7.1 Problem

Currently, source documents are identified only by UUIDs (`8ddd9d1b...`). Users need human-readable document names.

### 7.2 Solution: Fetch Document Metadata

The API already returns document metadata including `title`. The frontend needs to:

1. **Option A: Backend includes title in SourceReference**

   - Modify `SourceReference` to include `document_title?: string`
   - Backend populates this during context building

2. **Option B: Frontend fetches on demand**
   - Cache document metadata in client store
   - Fetch `/api/v1/documents/{id}` for unknown IDs
   - Display filename while loading title

### 7.3 Recommended: Option A

Modify backend `SourceReference` struct:

```rust
pub struct SourceReference {
    pub source_type: SourceType,
    pub id: String,
    pub content: Option<String>,
    pub score: f32,
    pub document_id: Option<String>,
    pub document_title: Option<String>,  // NEW
    pub file_name: Option<String>,       // NEW
    pub chunk_index: Option<usize>,
    // ... existing fields
}
```

---

## 8. Confidence Score Aggregation

### 8.1 Formula

```typescript
const calculateConfidence = (context: QueryContext): number => {
  const weights = {
    chunk: 0.5, // Document relevance most important
    entity: 0.3, // Entity match secondary
    relationship: 0.2, // Relationship context tertiary
  };

  const chunkScores = context.chunks?.map((c) => c.score) || [];
  const entityScores = context.entities?.map((e) => e.relevance) || [];
  const relScores = context.relationships?.map((r) => r.relevance) || [];

  const chunkAvg = average(chunkScores) || 0;
  const entityAvg = average(entityScores) || 0;
  const relAvg = average(relScores) || 0;

  return (
    chunkAvg * weights.chunk +
    entityAvg * weights.entity +
    relAvg * weights.relationship
  );
};
```

### 8.2 Display Thresholds

| Range   | Label             | Color     | Dots  |
| ------- | ----------------- | --------- | ----- |
| 80-100% | High Confidence   | green-500 | ●●●●● |
| 60-79%  | Good Confidence   | green-400 | ●●●●○ |
| 40-59%  | Medium Confidence | amber-500 | ●●●○○ |
| 20-39%  | Low Confidence    | amber-400 | ●●○○○ |
| 0-19%   | Very Low          | red-500   | ●○○○○ |

---

## 9. Copy Specifications

### 9.1 Microcopy

| Element             | Current                           | Proposed                                 |
| ------------------- | --------------------------------- | ---------------------------------------- |
| Summary             | "Sources: 7 chunks · 22 entities" | "📚 7 Sources · 22 Topics"               |
| Document header     | "SOURCE DOCUMENTS"                | "Documents" (tab)                        |
| Entity header       | "RELATED ENTITIES"                | "Key Topics"                             |
| Relationship header | "KEY RELATIONSHIPS"               | "Connections"                            |
| More items          | "+5 more chunks"                  | "Show 5 more sources"                    |
| Hover prompt        | (none)                            | "Click to view full document"            |
| Empty state         | (none)                            | "No sources available for this response" |

### 9.2 Tooltips

| Element         | Tooltip Text                                    |
| --------------- | ----------------------------------------------- |
| Confidence dots | "Confidence score based on semantic similarity" |
| Citation number | "Source #N - Click to view"                     |
| External link   | "Open document in new tab"                      |
| Entity chip     | "{Entity}: Click to explore in graph"           |

---

## 10. Error States

### 10.1 No Sources Available

```
┌─────────────────────────────────────────────────────────────────────┐
│ 📚 No source citations for this response                           │
│ This response was generated from the model's base knowledge.       │
└─────────────────────────────────────────────────────────────────────┘
```

### 10.2 Loading State

```
┌─────────────────────────────────────────────────────────────────────┐
│ 📚 Loading sources...                                      [●●●○○] │
├─────────────────────────────────────────────────────────────────────┤
│ ┌─────────────────────────────────────────────────────────────────┐ │
│ │ ░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░ │ │
│ │ ░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░ │ │
│ └─────────────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────────────┘
```

### 10.3 Failed to Load Documents

```
┌─────────────────────────────────────────────────────────────────────┐
│ ⚠️ Could not load document details                      [Retry ↻]  │
│ Click retry to attempt loading source documents again.             │
└─────────────────────────────────────────────────────────────────────┘
```

---

## 11. Animation Specifications

### 11.1 Expand Animation

```css
@keyframes expandPanel {
  from {
    opacity: 0;
    transform: translateY(-8px);
    max-height: 0;
  }
  to {
    opacity: 1;
    transform: translateY(0);
    max-height: 500px;
  }
}

.citations-panel[data-state="open"] {
  animation: expandPanel 0.25s ease-out;
}
```

### 11.2 Tab Content Transition

```css
@keyframes fadeSlide {
  from {
    opacity: 0;
    transform: translateX(8px);
  }
  to {
    opacity: 1;
    transform: translateX(0);
  }
}

.tab-content[data-state="active"] {
  animation: fadeSlide 0.2s ease-out;
}
```

### 11.3 Confidence Dots Animation

```css
.confidence-dot {
  transition: background-color 0.3s ease, transform 0.2s ease;
}

.confidence-dot.filled {
  animation: pulse 2s ease-in-out infinite;
}

@keyframes pulse {
  0%,
  100% {
    opacity: 1;
  }
  50% {
    opacity: 0.7;
  }
}
```

---

## 12. Design Tokens Reference

```typescript
// colors.ts
export const citationColors = {
  // Confidence
  confidenceHigh: "hsl(142.1 76.2% 36.3%)", // green-600
  confidenceMedium: "hsl(45.4 93.4% 47.5%)", // amber-500
  confidenceLow: "hsl(0 84.2% 60.2%)", // red-500

  // Entity types
  entityPerson: "hsl(221.2 83.2% 53.3%)", // blue-500
  entityOrg: "hsl(262.1 83.3% 57.8%)", // purple-500
  entityTech: "hsl(142.1 76.2% 36.3%)", // green-600
  entityConcept: "hsl(45.4 93.4% 47.5%)", // amber-500

  // Backgrounds
  cardBg: "hsl(var(--muted) / 0.3)",
  cardBgHover: "hsl(var(--muted) / 0.5)",

  // Borders
  cardBorder: "hsl(var(--border))",
  cardBorderHover: "hsl(var(--primary) / 0.3)",
};

// spacing.ts
export const citationSpacing = {
  panelPadding: "1rem",
  cardPadding: "0.75rem",
  cardGap: "0.5rem",
  sectionGap: "1rem",
  chipGap: "0.375rem",
};

// typography.ts
export const citationTypography = {
  summaryLabel: "text-sm font-medium",
  cardTitle: "text-sm font-medium leading-tight",
  cardFilename: "text-xs text-muted-foreground",
  cardExcerpt: "text-xs text-muted-foreground line-clamp-2",
  confidence: "text-xs font-semibold",
  chipLabel: "text-xs font-medium",
};
```

---

## 13. Testing Checklist

### 13.1 Visual Testing

- [ ] Summary bar renders correctly at all breakpoints
- [ ] Tab switching animation is smooth
- [ ] Confidence colors match thresholds
- [ ] Entity chips wrap correctly
- [ ] Document cards stack on mobile
- [ ] Dark mode colors are correct

### 13.2 Interaction Testing

- [ ] Expand/collapse works with click and keyboard
- [ ] Tabs are keyboard navigable
- [ ] Document links open in new tab
- [ ] Entity clicks navigate to graph
- [ ] Touch gestures work on mobile

### 13.3 Accessibility Testing

- [ ] Screen reader announces all interactive elements
- [ ] Focus indicators are visible
- [ ] Color contrast meets WCAG AA
- [ ] All text is at least 12px
- [ ] ARIA labels are meaningful

### 13.4 Performance Testing

- [ ] Initial render <50ms
- [ ] Tab switch <100ms
- [ ] No layout shift on expand
- [ ] Memoization prevents re-renders

---

## Next Steps

1. **Document 25**: Implementation Plan with code changes
2. Phase 1: Summary bar + terminology changes
3. Phase 2: Tab layout + document cards
4. Phase 3: Knowledge tab + mini graph
5. Phase 4: Polish + accessibility

---

**Document Status:** ✅ Complete  
**Next Document:** 25-source-citations-implementation-plan.md
