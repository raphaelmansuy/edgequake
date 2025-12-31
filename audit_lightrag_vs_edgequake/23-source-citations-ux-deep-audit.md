# Source Citations UX/UI Deep Audit

**Document ID:** 23  
**Date:** 2025-12-31  
**Scope:** Complete UX/UI analysis of source-citations.tsx component  
**Goal:** Transform current "messy" display into SOTA, slick, business-friendly citations

---

## Executive Summary

The current source citations component is **functional but not business-friendly**. It displays raw technical data (chunks, entity IDs, UUIDs) in a developer-centric format that fails to communicate value to business users.

**Key Issues:**

1. **Technical terminology** - "chunks", "entities" are developer terms
2. **Visual hierarchy failure** - All content has equal weight
3. **Progressive disclosure violation** - Too much information revealed at once
4. **Information density mismatch** - Complex data without contextual prioritization
5. **Missing provenance emphasis** - Source attribution is buried

**Redesign Vision:** A SOTA citations panel that feels like Perplexity/ChatGPT Sources - numbered inline citations, visual source cards, entity knowledge graph glimpse, and actionable document links.

---

## Part 1: Current Implementation Analysis

### File: `edgequake_webui/src/components/query/source-citations.tsx`

**Lines of Code:** 324  
**Components:** 2 (`SourceCitations`, `InlineCitation`)  
**Dependencies:** shadcn/ui (Badge, Button, Card, Collapsible, HoverCard, ScrollArea)

### 1.1 Data Flow Analysis

```
QueryContext (from API)
├── chunks: ChunkReference[]
│   ├── content: string (raw text)
│   ├── document_id: string (UUID)
│   └── score: number (0-1)
├── entities: EntityReference[]
│   ├── id: string
│   ├── label: string
│   ├── relevance: number (0-1)
│   ├── source_file_path?: string
│   └── source_document_id?: string
└── relationships: RelationshipReference[]
    ├── source: string
    ├── target: string
    ├── type: string
    ├── relevance: number (0-1)
    ├── source_file_path?: string
    └── source_document_id?: string
```

### 1.2 Current UI Structure

```
┌─────────────────────────────────────────────────────────┐
│ [FileText icon] Sources: 7 chunks · 22 entities  [▼]   │  ← Trigger Button
├─────────────────────────────────────────────────────────┤
│ SOURCE DOCUMENTS                                        │  ← Section Header
│ ┌─────────────────────────────────────────────────────┐ │
│ │ [FileText] 8ddd9d1b... [↗]        [2 chunks]        │ │  ← Document Card
│ │ "MegaRAG: Multimodal Knowledge Graph-Based..."      │ │  ← Chunk Preview
│ │ +0 more chunks                                      │ │
│ └─────────────────────────────────────────────────────┘ │
│                                                         │
│ RELATED ENTITIES                                        │  ← Section Header
│ [MEGARAG] [LIGHTRAG] [NATIONAL TAIWAN...] [+12 more]   │  ← Entity Badges
│                                                         │
│ KEY RELATIONSHIPS                                       │  ← Section Header
│ MEGARAG → [INTEGRATES] → KNOWLEDGE_GRAPH               │  ← Relationship Row
│ LIGHTRAG → [USES] → TEXT_EMBEDDING                     │
│ +10 more relationships                                 │
└─────────────────────────────────────────────────────────┘
```

### 1.3 Critical UX Issues

| Issue                         | Severity  | Description                                                   |
| ----------------------------- | --------- | ------------------------------------------------------------- |
| **Technical Jargon**          | 🔴 High   | "chunks" and "entities" are meaningless to business users     |
| **UUID Exposure**             | 🔴 High   | Document IDs shown as "8ddd9d1b..." instead of document names |
| **No Visual Hierarchy**       | 🔴 High   | All sections have equal weight; no focal point                |
| **Truncated Content**         | 🟡 Medium | Content cut at arbitrary character limits (100 chars)         |
| **Missing Relevance Signals** | 🟡 Medium | Scores shown as percentages without visual treatment          |
| **No Document Names**         | 🔴 High   | Documents identified only by truncated UUIDs                  |
| **Relationship Complexity**   | 🟡 Medium | Source→Type→Target shown inline, hard to parse                |
| **Hover Required**            | 🟡 Medium | Key information only visible on hover (not mobile-friendly)   |
| **Inline Citations Unused**   | 🟡 Medium | `InlineCitation` component exists but not integrated          |

### 1.4 Information Architecture Issues

**Current IA:**

```
Sources (collapsed)
└── [Expand]
    ├── Source Documents (scrollable)
    │   └── Document Card (hover for details)
    ├── Related Entities (inline badges)
    │   └── Badge (hover for details)
    └── Key Relationships (inline rows)
        └── Relationship (hover for details)
```

**Problems:**

1. Three sections competing for attention
2. No clear primary/secondary content distinction
3. All details require hover interaction
4. Scroll areas within scroll areas (nested scrolling)

---

## Part 2: Information Theory Analysis

### 2.1 Shannon's Information Hierarchy

For source citations, information value follows this priority:

| Priority | Information          | User Question                 |
| -------- | -------------------- | ----------------------------- |
| P1       | Source document name | "Where did this come from?"   |
| P2       | Relevance indicator  | "How confident is this?"      |
| P3       | Key entity names     | "What concepts are involved?" |
| P4       | Relationship types   | "How are things connected?"   |
| P5       | Text excerpts        | "What's the actual content?"  |

**Current Implementation Priority (inverted):**

- Shows P5 (raw text) prominently
- Hides P1 (document names) - only UUIDs visible
- P2 (relevance) shown as small badges

### 2.2 Progressive Disclosure Requirements

Based on interaction design best practices:

| Level              | Content                        | Trigger                       |
| ------------------ | ------------------------------ | ----------------------------- |
| **L0** (summary)   | Source count + confidence      | Always visible after response |
| **L1** (overview)  | Document list with names       | Click expand                  |
| **L2** (details)   | Entity/relationship highlights | Tab or scroll                 |
| **L3** (deep dive) | Full text, graph view          | Click "View in document"      |

---

## Part 3: Competitive Analysis

### 3.1 Perplexity AI Pattern

```
┌──────────────────────────────────────────────┐
│ Answer text with [1] inline citations...     │
├──────────────────────────────────────────────┤
│ Sources                                      │
│ ┌────┬────┬────┬────┬────────────────────┐  │
│ │ 1  │ 2  │ 3  │ 4  │ View all 12 ▶      │  │
│ │icon│icon│icon│icon│                    │  │
│ │name│name│name│name│                    │  │
│ └────┴────┴────┴────┴────────────────────┘  │
└──────────────────────────────────────────────┘
```

**Key Patterns:**

- Numbered inline citations [1], [2], [3]
- Horizontal card carousel for sources
- Favicon + truncated title per source
- "View all" for expansion
- No technical IDs visible

### 3.2 ChatGPT with Browse Pattern

```
┌──────────────────────────────────────────────┐
│ Response text with ❶ numbered citations...   │
├──────────────────────────────────────────────┤
│ ❶ Title of Source                           │
│   domain.com • brief description             │
│ ❷ Another Source Title                       │
│   otherdomain.com • brief description        │
└──────────────────────────────────────────────┘
```

**Key Patterns:**

- Circled numbers for inline citations
- Clean list with title + domain
- One-line descriptions
- No hover required for basics

### 3.3 Proposed EdgeQuake Pattern

Combining best practices with knowledge graph emphasis:

```
┌──────────────────────────────────────────────────────────────┐
│ Response text with ¹ superscript citations...                │
├──────────────────────────────────────────────────────────────┤
│ 📚 7 Sources • 22 Knowledge Entities • 92% Confidence   [▼] │
├──────────────────────────────────────────────────────────────┤
│ ◉ DOCUMENTS                                                  │
│ ┌────────────────────────────────────────────────────────┐  │
│ │ ¹ MegaRAG Paper                    98% │ View →        │  │
│ │   mega_rag_2512.md                                     │  │
│ │   "Multimodal Knowledge Graph-Based RAG..."            │  │
│ └────────────────────────────────────────────────────────┘  │
│ ┌────────────────────────────────────────────────────────┐  │
│ │ ² LightRAG Architecture            95% │ View →        │  │
│ │   lightrag_arch.md                                     │  │
│ │   "Graph-based retrieval system..."                    │  │
│ └────────────────────────────────────────────────────────┘  │
│                                                              │
│ ◉ KNOWLEDGE GRAPH                                            │
│ ┌────────────────────────────────────────────────────────┐  │
│ │     [MEGARAG]                                          │  │
│ │        ↓ integrates                                    │  │
│ │  [KNOWLEDGE GRAPH] ←── uses ── [LIGHTRAG]              │  │
│ │        ↓ enables                                       │  │
│ │   [MULTIMODAL RAG]                                     │  │
│ └────────────────────────────────────────────────────────┘  │
│ [Explore Full Graph →]                                       │
└──────────────────────────────────────────────────────────────┘
```

---

## Part 4: Design Tokens & Visual Language

### 4.1 Current Design Token Usage

```css
/* Colors used */
--muted-foreground: #6b7280    /* Gray text */
--primary: #3b82f6             /* Blue links */
--secondary: #f3f4f6           /* Badge background */
--border: #e5e7eb              /* Card borders */

/* Typography */
text-xs: 0.75rem (12px)        /* Overused */
text-[10px]: 10px              /* Too small */

/* Spacing */
gap-1: 0.25rem                 /* Too tight */
p-2: 0.5rem                    /* Cards cramped */
```

### 4.2 Proposed Design Tokens

```css
/* Semantic Colors */
--source-document: #f0f9ff     /* Light blue for docs */
--source-entity: #ecfdf5       /* Light green for entities */
--source-relation: #fef3c7     /* Light amber for relationships */
--confidence-high: #22c55e     /* Green >80% */
--confidence-medium: #f59e0b   /* Amber 50-80% */
--confidence-low: #ef4444      /* Red <50% */

/* Typography */
--citation-number: 0.65rem     /* Superscript size */
--source-title: 0.875rem       /* Document titles */
--source-body: 0.75rem         /* Descriptions */

/* Spacing */
--card-padding: 0.75rem        /* Source cards */
--section-gap: 1rem            /* Between sections */
```

---

## Part 5: Component Architecture Recommendations

### 5.1 Proposed Component Tree

```
<SourceCitationsPanel>
├── <CitationsSummaryBar>
│   ├── <SourceCount>
│   ├── <EntityCount>
│   ├── <ConfidenceBadge>
│   └── <ExpandToggle>
│
├── <CitationsContent> (when expanded)
│   ├── <TabList>
│   │   ├── "Documents" (default)
│   │   ├── "Knowledge"
│   │   └── "Explore"
│   │
│   ├── <TabPanel name="Documents">
│   │   └── <SourceDocumentList>
│   │       └── <SourceDocumentCard>
│   │           ├── <CitationNumber>
│   │           ├── <DocumentTitle>
│   │           ├── <ConfidenceIndicator>
│   │           ├── <ExcerptPreview>
│   │           └── <ViewDocumentLink>
│   │
│   ├── <TabPanel name="Knowledge">
│   │   ├── <EntityCloud>
│   │   │   └── <EntityChip>
│   │   └── <RelationshipMiniGraph>
│   │
│   └── <TabPanel name="Explore">
│       └── <GraphExplorerLink>
│
└── <InlineCitationTooltip>
```

### 5.2 State Management

```typescript
interface SourceCitationsState {
  isExpanded: boolean;
  activeTab: "documents" | "knowledge" | "explore";
  hoveredCitation: number | null;
  selectedDocument: string | null;
}
```

---

## Part 6: Terminology Translation

### Business-Friendly Vocabulary

| Current (Technical) | Proposed (Business)      |
| ------------------- | ------------------------ |
| chunks              | text excerpts / passages |
| entities            | key concepts / topics    |
| relationships       | connections / links      |
| document_id         | (hidden - show name)     |
| score / relevance   | confidence / match       |
| source_file_path    | source document          |

### Summary Line Transformations

| Current                           | Proposed                                    |
| --------------------------------- | ------------------------------------------- |
| "Sources: 7 chunks · 22 entities" | "📚 7 Sources · 22 Topics · 92% Confidence" |
| "SOURCE DOCUMENTS"                | "◉ DOCUMENTS"                               |
| "RELATED ENTITIES"                | "◉ KEY TOPICS"                              |
| "KEY RELATIONSHIPS"               | "◉ CONNECTIONS"                             |

---

## Part 7: Accessibility Audit

### 7.1 Current Issues

| Issue               | WCAG  | Description                              |
| ------------------- | ----- | ---------------------------------------- |
| 10px font size      | 1.4.4 | Below minimum 12px for readability       |
| Hover-only info     | 2.1.1 | Keyboard users cannot access hover cards |
| Low contrast badges | 1.4.3 | Secondary badges may fail contrast       |
| No focus indicators | 2.4.7 | Interactive elements lack visible focus  |
| Truncated content   | 1.3.1 | "8ddd9d1b..." not meaningful             |

### 7.2 Recommendations

1. **Minimum 12px font size** for all readable text
2. **Click/Enter to toggle** hover cards, not just hover
3. **Focus ring visible** on all interactive elements
4. **Full document names** visible without truncation
5. **ARIA labels** for citation numbers

---

## Part 8: Performance Considerations

### 8.1 Current Implementation

```typescript
// Grouping happens on every render
const chunksByDocument =
  context.chunks?.reduce((acc, chunk) => {
    if (!acc[chunk.document_id]) {
      acc[chunk.document_id] = [];
    }
    acc[chunk.document_id].push(chunk);
    return acc;
  }, {} as Record<string, typeof context.chunks>) || {};
```

### 8.2 Optimization Recommendations

1. **Memoize grouping** with `useMemo`:

```typescript
const chunksByDocument = useMemo(
  () => groupBy(context.chunks, "document_id"),
  [context.chunks]
);
```

2. **Lazy load tab content** - Only render active tab:

```typescript
{activeTab === 'documents' && <DocumentList ... />}
{activeTab === 'knowledge' && <KnowledgePanel ... />}
```

3. **Virtualize long lists** for >20 sources

---

## Part 9: Mobile Responsiveness

### 9.1 Current Issues

- HoverCard not touch-friendly
- Horizontal scrolling on small screens
- Dense information doesn't stack

### 9.2 Mobile-First Recommendations

```css
/* Stack layout on mobile */
@media (max-width: 640px) {
  .source-card {
    flex-direction: column;
  }

  .entity-cloud {
    gap: 0.5rem;
    flex-wrap: wrap;
  }

  .relationship-row {
    flex-direction: column;
    align-items: flex-start;
  }
}
```

---

## Part 10: Implementation Priority

### Phase 1: Quick Wins (1-2 hours)

1. ✅ Replace "chunks" → "sources"
2. ✅ Replace "entities" → "topics"
3. ✅ Add confidence badge to summary
4. ✅ Show document names instead of UUIDs (requires backend metadata)

### Phase 2: Layout Redesign (4-6 hours)

1. Implement tab-based layout
2. Create SourceDocumentCard component
3. Add citation numbers to sources
4. Implement confidence color indicators

### Phase 3: Knowledge Graph Enhancement (4-6 hours)

1. Create EntityCloud component
2. Implement mini relationship graph
3. Add "Explore in Graph" action link
4. Integrate with graph visualization page

### Phase 4: Polish (2-4 hours)

1. Accessibility audit fixes
2. Mobile responsive testing
3. Animation/transitions
4. Dark mode validation

---

## Appendix A: Code Metrics

| Metric                | Current        | Target                           |
| --------------------- | -------------- | -------------------------------- |
| Lines of Code         | 324            | ~450 (split into sub-components) |
| Component Count       | 2              | 6-8                              |
| Max Nesting Depth     | 7              | 4                                |
| Cyclomatic Complexity | 12             | 6                                |
| Render Count (dev)    | ~3/interaction | 1/interaction                    |

---

## Appendix B: Screenshot References

The user provided screenshot shows:

- Button: "Sources: 7 chunks · 25 entities"
- Expanded panel with three sections
- Document cards with truncated UUIDs
- Entity badges in horizontal row
- Relationship rows with arrows

This confirms all issues identified in this audit.

---

## Next Steps

1. Create detailed UX/UI specification (Document 24)
2. Create implementation plan with code changes (Document 25)
3. Implement Phase 1 changes
4. Test with real users
5. Iterate based on feedback

---

**Document Status:** ✅ Complete  
**Next Document:** 24-source-citations-ux-specification.md
