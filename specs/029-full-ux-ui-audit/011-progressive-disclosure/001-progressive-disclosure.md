# 001 — Progressive Disclosure Audit

**First Principle: Economy** — Show only what the user needs to act next.

---

## Definition

Progressive disclosure is the practice of revealing information and controls gradually, based on user need and context. Advanced users can access full complexity; new users see only what they need.

---

## Settings Page: Information Overload

### Current State

The settings page renders all sections simultaneously as a long scrollable page:

```
Settings page (estimated ~40+ controls):
─────────────────────────────────────────────────
• Appearance (Theme, Language)
• Query Settings
  - Default mode
  - Max tokens  
  - Temperature
  - Top-K
  - Stream responses
  - System prompt
• Ingestion Settings
  - Chunk size
  - Overlap
  - Entity extraction
• Graph Settings
  - Max depth
  - Max nodes
  - Entity types
• Provider Settings
  - LLM provider status
  - Vision LLM settings
  - PDF parser settings
• Admin
  - Quota management
  - User management
  - Rebuild embeddings
• Data Management
  - Export/Import settings
  - Clear query history
  - Reset all settings
• Config Explainability Panel
```

**PD-01 · Settings Page Has No Visual Hierarchy**

All sections are at the same visual weight. A user who just wants to change the theme has to scroll past 30+ controls to find the appearance section.

**Fix: Grouped Settings with Navigation**

Implement a settings page with left-nav categories (similar to VS Code, Linear, Slack settings):

```
┌──────────────────────────────────────────────────────────┐
│  Settings                                                │
├──────────────┬───────────────────────────────────────────┤
│              │                                           │
│  General     │  ← selected section content              │
│  Appearance  │                                           │
│  Query       │  [Appearance]                             │
│  Ingestion   │  ─────────────────────────────────────── │
│  Graph       │  Theme                                    │
│  Providers   │  [Light]  [Dark]  [System]               │
│  Admin ─ ─ ─│                                           │
│  Data        │  Language                                 │
│              │  [English ▼]                              │
└──────────────┴───────────────────────────────────────────┘
```

### Query Settings Sheet

The `QuerySettingsSheet` slides in from the right with query-specific settings (stream toggle, topK, temperature, maxTokens, systemPrompt). This IS a good progressive disclosure pattern — advanced options hidden behind a gear icon.

**PD-02 · Query Settings: Show Defaults, Not All Options**

The settings sheet likely shows all options simultaneously. Better approach:

```
Basic settings:     [Show all ▼]
  Stream responses  [●──────] On
  
Advanced settings:  [collapsed by default]
  Max tokens        [2048]
  Temperature       [0.7]
  Top-K             [10]
  System prompt     [empty textarea]
```

---

## Document Manager: Feature Complexity

### PDF Backend Selector

```typescript
// document-toolbar-section.tsx
<PdfParserBackendField
  value={pdfParserBackend}
  onChange={setPdfParserBackend}
/>
```

The PDF backend selector (default / vision / edgeparse) is exposed in the primary toolbar. This is a **power user** option that most users will never change.

**PD-03 · Move PDF Backend to Per-Document or Advanced Upload Modal**

Instead of a persistent toolbar control, offer this option in the upload dialog:

```
┌─────────────────────────────────────────────────────────┐
│  Upload Documents                                       │
│  ─────────────────────────────────────────────────────  │
│  Drop files here or click to browse                     │
│  PDF, DOCX, MD, TXT supported                           │
│                                                         │
│  [Advanced options ▼]                                   │
│    Processing: [Auto ▼]  ← hidden by default            │
│                                                         │
│  [Upload]   [Cancel]                                    │
└─────────────────────────────────────────────────────────┘
```

---

## Onboarding: Missing Progressive Introduction

### PD-04 · No First-Time User Experience

The app jumps directly to the dashboard (or login) with no onboarding. A first-time user sees:

```
Dashboard:
- 0 Documents
- 0 Entities  
- 0 Relationships
- 0 Types
+ Quick Actions (Upload / Query / Browse)
+ System Status (connected)
```

This communicates **nothing** about what EdgeQuake does or how to start.

**Fix: Progressive Welcome State**

When the workspace has 0 documents, replace the stats cards with a guided onboarding:

```
┌──────────────────────────────────────────────────────────────────┐
│                                                                  │
│  Welcome to EdgeQuake 👋                                         │
│  ──────────────────────────────────────────────────────────────  │
│                                                                  │
│  Build your knowledge graph in 3 steps:                          │
│                                                                  │
│  [1] ↑ Upload documents      ← highlighted, primary CTA         │
│      PDFs, Word docs, Markdown                                   │
│                                                                  │
│  [2] ⚙ Processing              ← dimmed/secondary               │
│      AI extracts entities and relationships                      │
│                                                                  │
│  [3] ☐ Ask questions           ← dimmed/secondary               │
│      Query your knowledge graph naturally                        │
│                                                                  │
└──────────────────────────────────────────────────────────────────┘
```

Note: The app already has a `tour-provider.tsx` (onboarding tour) and `graph-tour-wrapper.tsx`. This first-time experience should trigger the tour or reference it.

---

## Navigation: Progressive Feature Discovery

### PD-05 · Advanced Features in Primary Nav

"API Explorer" and "Costs" are visible in the primary navigation from day one. These are rarely needed features that take up space and add cognitive load for new users.

**Options:**

1. **Hide until used:** Only show API Explorer and Costs after user has uploaded ≥1 document
2. **Pin system:** Let users choose what to show in sidebar (like VS Code extensions)
3. **Secondary group:** Always show but in a clearly "secondary" visual group (already recommended in navigation audit)

---

## Batch Operations: Confirmation Complexity

### PD-06 · Bulk Reprocess Dialog Shows Too Early

The `BulkReprocessDialog` appears when user clicks "Reprocess" on selected documents. It asks: "Full re-process or entities only?"

This is a good question for power users but confusing for new users who just want to fix a failed document.

**Fix: Default action with advanced option**

```
┌──────────────────────────────────────────────────────┐
│  Reprocess 3 documents?                              │
│  ──────────────────────────────────────────────────  │
│  This will re-extract entities and update the        │
│  knowledge graph.                                    │
│                                                      │
│  [Reprocess]       ← default, full reprocess         │
│                                                      │
│  [Advanced options ▼]                                │
│    ○ Entities only (faster, reuses existing PDF)    │
│    ● Full reprocess (re-converts PDF + entities)    │
│                                                      │
│  [Cancel]                                            │
└──────────────────────────────────────────────────────┘
```

---

## Graph Viewer: Progressive Context

### PD-07 · Graph Controls Always Expanded

The `GraphViewer` has multiple side panels and controls visible by default:
- Search panel
- Filters panel  
- Legend
- Bookmarks
- Settings panel

For a 50-node graph, all these panels are useful. For a 5,000-node graph, they may be overwhelming. For a new user with 0 nodes, they're noise.

**Fix: Contextual panel visibility**

```typescript
// graph-viewer.tsx
const shouldShowPanels = nodes.length > 0;
const shouldShowAdvancedControls = nodes.length > 100;

return (
  <>
    {shouldShowPanels && <GraphFilters />}
    {shouldShowAdvancedControls && <GraphSettings />}
  </>
);
```

---

## Progressive Disclosure Scorecard

| Area                 | Current                  | Target                             | Priority |
| -------------------- | ------------------------ | ---------------------------------- | -------- |
| Settings page        | All options flat         | Grouped with left-nav              | P1       |
| Query toolbar        | 5 controls visible       | 3 visible, 2 in sheet              | P1       |
| Document toolbar     | 6 controls               | 3 visible, 3 in overflow           | P1       |
| Onboarding           | None                     | Welcome state for 0-doc workspaces | P1       |
| PDF backend selector | In primary toolbar       | In upload advanced options         | P2       |
| Bulk reprocess       | Choice dialog first      | Default + advanced                 | P2       |
| Graph controls       | All visible              | Progressive by graph size          | P2       |
| Admin settings       | Mixed with user settings | Admin section (role-gated)         | P2       |

---

## External References

- [Progressive Disclosure — NNGroup](https://www.nngroup.com/articles/progressive-disclosure/)
- [10 Usability Heuristics — Jakob Nielsen](https://www.nngroup.com/articles/ten-usability-heuristics/)
- [Settings UX Best Practices — UX Collective](https://uxdesign.cc/design-better-settings-screens)
- [Onboarding Design Patterns — Product Hunt](https://www.producthunt.com/)
- [Linear Settings UX](https://linear.app/settings) — reference implementation
