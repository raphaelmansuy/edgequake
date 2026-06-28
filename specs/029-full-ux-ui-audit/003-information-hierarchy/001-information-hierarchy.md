# 001 — Information Hierarchy Audit

**First Principle: Hierarchy** — Guide the eye to what matters most.

---

## Dashboard Page

### Current State

```
┌─────────────────────────────────────────────────────────────────┐
│ Header (h-12)                                                   │
├─────────────────────────────────────────────────────────────────┤
│ Breadcrumb                                                      │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  [Documents 0]  [Entities 0]  [Relationships 0]  [Types 0]    │
│   stats card     stats card     stats card         stats card  │
│                                                                 │
│  ┌─────────────────────────┐  ┌──────────────────────────────┐ │
│  │ Quick Actions           │  │ System Status                │ │
│  │ - Upload Documents      │  │ - Backend: connected         │ │
│  │ - Query KG              │  │ - Version: 0.1.0             │ │
│  │ - Browse Graph          │  └──────────────────────────────┘ │
│  └─────────────────────────┘                                   │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │ Recent Activity                                          │  │
│  └──────────────────────────────────────────────────────────┘  │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### Issues

**IH-01 · Stats Cards: Numbers Without Narrative**

Four stats cards show raw numbers (0, 0, 0, 0) with no contextual meaning for new users. 

- What does "0 entities" mean to a first-time user?
- Cards have equal visual weight — no primary metric
- `border-0 shadow-sm` styling is subtly inconsistent with other cards

**IH-02 · Quick Actions: CTA Hierarchy Flat**

Three equal-weight quick action cards. The primary action (Upload Documents to start the workflow) should be visually dominant.

```
CURRENT: [Upload] [Query] [Graph]  ← equal visual weight

BETTER: One primary CTA + 2 secondary
        [Upload Documents — Primary/Filled] 
        [Query]  [Graph]  ← ghost/outline
```

**IH-03 · System Status: Low-Value Prime Real Estate**

"Backend: connected" and "Version: 0.1.0" occupy the top-right card on the dashboard. For 99% of sessions where the backend IS connected, this card communicates nothing. It should collapse when healthy, or be demoted to the Settings page.

**IH-04 · "Recent Activity" Missing**

The `RecentActivity` component appears to be a stub or renders empty state for new users. If empty, it should not take up space that could be used for onboarding/guidance content.

---

## Documents Page

### Current State

```
┌─────────────────────────────────────────────────────────────────┐
│ [Upload] [Scan] [PDF Backend ▼] [Filter ▼] [Sort ▼] [⋮ More]  │  ← Toolbar
├─────────────────────────────────────────────────────────────────┤
│ Document count: 12 total, 3 selected                           │
├─────────────────────────────────────────────────────────────────┤
│ [☐] Name          Status    Chunks  Cost   Date    [⋯]        │  ← Table header
│ ─────────────────────────────────────────────────────────────  │
│ [☐] document.pdf  ● Completed  42   $0.02  2h ago  [⋯]       │
│ [☐] report.pdf    ● Extracting  -    -     1m ago  [⋯]       │
└─────────────────────────────────────────────────────────────────┘
```

### Issues

**IH-05 · Table Column Weight Imbalance**

The `Name` column is most important but has the same visual weight as `Chunks`, `Cost`, `Date`. The `Cost` and `Chunks` columns are rarely referenced during primary workflows (checking status of uploads).

**Proposed column priority:**

```
Primary:   Name (flex-grow), Status
Secondary: Date (relative)
Tertiary:  Chunks, Cost (hidden by default, expandable via column toggle)
```

**IH-06 · Toolbar Density**

The document toolbar has 6 controls in a single row. When documents are selected, a batch action bar appears below — creating two toolbar rows.

```
CURRENT FLOW:
[Upload] [Scan] [PDF ▼] [Filter ▼] [Sort ▼] [Search] [⋮]
↓ (when items selected)
[2 selected] [Reprocess] [Delete] [×]
```

The batch action bar slides in from below (good) but the primary toolbar remains, creating visual density.

**IH-07 · Status Badge Information Density**

The status badge column shows a colored dot + text in a small badge. For processing documents the dot animates. However:

- `partial_failure` and `partial_success` are visually similar (amber/orange)  
- Long status labels ("Converting PDF") truncate in small screens
- No progress percentage visible during processing

---

## Query Page

### Current State

```
┌─────────────────────────────────────────────────────────────────┐
│ Header: [☰] Query  Ask your knowledge graph  | [New] [Provider▼] [Mode▼] [Filter▼] [⚙] │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  [Conversation history sidebar — hidden on mobile]             │
│                                                                 │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │                                                         │   │
│  │           Empty state OR messages                       │   │
│  │                                                         │   │
│  └─────────────────────────────────────────────────────────┘   │
│                                                                 │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │ 📎  [Type your question...              ] [↑ Send]      │   │
│  └─────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
```

**IH-08 · Header Bar Overloaded**

The query header packs: page title, subtitle, new-conversation button, provider selector, mode selector, document filter, and settings sheet into a single 48px row. On smaller viewports (768-1024px), these overflow or truncate.

**Count of controls in query header: 5 interactive elements**

Compare to Linear.app's chat: title + 1 action button + overflow menu.

**IH-09 · Mode Selector: Exposed Complexity**

The query mode selector (Local / Global / Hybrid / Simple) is permanently visible. Most users will use "Hybrid" always. The mode is technical jargon — "search strategy" would be friendlier. The selector should default to Hybrid and be accessible but not prominent.

---

## Recommendations

### IH-01 Fix · Stats Cards with Narrative Context

```
┌─────────────────────────────┐
│ 📄 Documents                │
│ ─────────────────────────── │
│      0                      │  ← Large number
│ No documents yet            │  ← Contextual sub-text
│ [Upload your first →]       │  ← CTA only when 0
└─────────────────────────────┘
```

### IH-03 Fix · Collapse System Status Card

```typescript
// system-status.tsx
// Only show expanded when there IS a problem
// Default: compact badge in header or hide on dashboard

const isHealthy = connectionStatus === 'connected';

if (isHealthy && !showDetails) {
  return null; // Don't take up dashboard real estate
}
```

### IH-08 Fix · Query Header Control Hierarchy

```
┌──────────────────────────────────────────────────────────────────┐
│ Query                    [+ New]           [Mode: Hybrid ▼] [⚙] │
└──────────────────────────────────────────────────────────────────┘
```

Move provider selector and document filter into the `[⚙]` settings sheet. Most users never change these. Only expose them when needed.

---

## Visual Hierarchy Framework

```
Level 1: Page Title (h1) — 24px / semibold
Level 2: Section Title — 16px / semibold
Level 3: Card Title — 14px / medium
Level 4: Label — 12px / medium, muted
Level 5: Value — varies, foreground
Level 6: Metadata — 11px, muted-foreground
```

Every page should use these levels consistently. Currently, the dashboard uses `text-base` (16px) for card titles, and the query page uses `text-base sm:text-lg` for its h1 — inconsistent.

---

## External References

- [F-Pattern in Reading — NNGroup](https://www.nngroup.com/articles/f-shaped-pattern-reading-web-content/)
- [Visual Hierarchy in UX — NNGroup](https://www.nngroup.com/articles/visual-hierarchy-ux-definition/)
- [Information Architecture — Peter Morville](https://www.infoarch.ai/)
- [Contrast and Visual Hierarchy — Refactoring UI](https://www.refactoringui.com/)
