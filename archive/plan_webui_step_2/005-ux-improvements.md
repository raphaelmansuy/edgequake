# UX Improvements Plan

> **Document Version:** 1.0  
> **Date:** 2024-12-23  
> **Purpose:** User experience enhancements and design improvements

---

## Table of Contents

1. [UX Principles](#ux-principles)
2. [Current State Assessment](#current-state-assessment)
3. [Improvement Areas](#improvement-areas)
4. [Detailed UX Enhancements](#detailed-ux-enhancements)
5. [Accessibility Considerations](#accessibility-considerations)
6. [Responsive Design](#responsive-design)
7. [Interaction Patterns](#interaction-patterns)

---

## UX Principles

### Design Philosophy

EdgeQuake WebUI should embody these core UX principles:

| Principle                | Description                                    |
| ------------------------ | ---------------------------------------------- |
| **Clarity**              | Information should be instantly understandable |
| **Efficiency**           | Common tasks should require minimal clicks     |
| **Feedback**             | Every action should have clear visual feedback |
| **Consistency**          | Similar actions should work the same way       |
| **Accessibility**        | Usable by people with diverse abilities        |
| **Internationalization** | Localized for global audiences                 |

### Nielsen's Heuristics Applied

| Heuristic                   | Application                                            |
| --------------------------- | ------------------------------------------------------ |
| Visibility of system status | Loading states, progress indicators, connection status |
| Match system & real world   | Familiar graph terminology, intuitive icons            |
| User control & freedom      | Undo actions, clear navigation, escape routes          |
| Consistency & standards     | Follow established UI patterns                         |
| Error prevention            | Confirmation dialogs, input validation                 |
| Recognition over recall     | Visible options, search with suggestions               |
| Flexibility & efficiency    | Keyboard shortcuts, power user features                |
| Aesthetic & minimalist      | Clean interface, progressive disclosure                |
| Help users with errors      | Clear error messages, recovery guidance                |
| Help & documentation        | Contextual help, tooltips                              |

---

## Current State Assessment

### Strengths ✅

| Area                | Current Implementation           |
| ------------------- | -------------------------------- |
| Clean layout        | Modern sidebar + content layout  |
| Theme support       | Dark/light mode with next-themes |
| Responsive sidebar  | Collapsible for mobile           |
| Loading states      | Skeleton loaders, spinners       |
| Toast notifications | Sonner integration               |
| Form validation     | Basic validation present         |

### Weaknesses ❌

| Area                      | Issue                     | Impact                     |
| ------------------------- | ------------------------- | -------------------------- |
| No i18n                   | English only              | Excludes non-English users |
| Limited graph interaction | No drag, no search        | Reduces exploration        |
| No pagination             | All docs loaded at once   | Performance issues         |
| No filtering              | Cannot find specific docs | Productivity loss          |
| No COT display            | Thinking process hidden   | Lacks transparency         |
| No math rendering         | LaTeX shows as text       | Academic use limited       |
| No keyboard shortcuts     | Mouse-only navigation     | Power users frustrated     |

---

## Improvement Areas

### Priority UX Improvements by Area

```
┌─────────────────────────────────────────────────────────────────┐
│                    UX IMPROVEMENT AREAS                          │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐  │
│  │   NAVIGATION    │  │   DOCUMENTS     │  │     GRAPH       │  │
│  │                 │  │                 │  │                 │  │
│  │ • i18n          │  │ • Pagination    │  │ • Node search   │  │
│  │ • Breadcrumbs   │  │ • Filtering     │  │ • Drag & drop   │  │
│  │ • Keyboard nav  │  │ • Sorting       │  │ • Full-screen   │  │
│  │ • Search        │  │ • Bulk actions  │  │ • Legend        │  │
│  │                 │  │ • Status filter │  │ • Tooltips      │  │
│  └─────────────────┘  └─────────────────┘  └─────────────────┘  │
│                                                                  │
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐  │
│  │     QUERY       │  │    FEEDBACK     │  │   SETTINGS      │  │
│  │                 │  │                 │  │                 │  │
│  │ • COT display   │  │ • Pipeline      │  │ • Preferences   │  │
│  │ • LaTeX render  │  │ • Progress      │  │ • Persistence   │  │
│  │ • Mermaid       │  │ • Error states  │  │ • Reset option  │  │
│  │ • History       │  │ • Empty states  │  │ • Export/Import │  │
│  │ • Mode prefix   │  │ • Success msgs  │  │                 │  │
│  └─────────────────┘  └─────────────────┘  └─────────────────┘  │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

---

## Detailed UX Enhancements

### 1. Navigation & Global UX

#### 1.1 Language Selector

**Priority:** Critical  
**Gap Reference:** GAP-001

**Current State:**

- No language option
- All text hardcoded

**Target State:**

- Language dropdown in header
- Instant language switching
- Language persisted in settings

**UI Mockup:**

```
┌─────────────────────────────────────────────────────────────┐
│  EdgeQuake    [Docs] [Graph] [Query]    🌐 [EN ▼]  [☀️]  [👤] │
│                                         ├──────────┤         │
│                                         │ 🇺🇸 English │        │
│                                         │ 🇨🇳 中文    │        │
│                                         │ 🇫🇷 Français│        │
│                                         └──────────┘         │
└─────────────────────────────────────────────────────────────┘
```

---

#### 1.2 Keyboard Shortcuts

**Priority:** Medium  
**New Feature**

**Proposed Shortcuts:**

| Shortcut         | Action                        |
| ---------------- | ----------------------------- |
| `⌘/Ctrl + K`     | Open command palette / search |
| `⌘/Ctrl + /`     | Show keyboard shortcuts       |
| `⌘/Ctrl + 1`     | Go to Documents               |
| `⌘/Ctrl + 2`     | Go to Graph                   |
| `⌘/Ctrl + 3`     | Go to Query                   |
| `⌘/Ctrl + Enter` | Submit query                  |
| `Esc`            | Close dialogs, deselect       |
| `?`              | Show help                     |

**Implementation:**

```tsx
// Already exists: src/hooks/use-keyboard-shortcuts.ts
// Extend with global shortcuts
```

---

#### 1.3 Command Palette

**Priority:** Low  
**New Feature**

A spotlight-like search for quick navigation:

```
┌─────────────────────────────────────────────────────────────┐
│  🔍 Search commands, documents, or entities...              │
├─────────────────────────────────────────────────────────────┤
│  📄 Documents                                                │
│      → Upload document                                       │
│      → Search documents                                      │
│  🕸️ Graph                                                   │
│      → Search nodes                                          │
│      → Change layout                                         │
│  ❓ Query                                                    │
│      → New query                                             │
└─────────────────────────────────────────────────────────────┘
```

---

### 2. Document Management UX

#### 2.1 Pagination Controls

**Priority:** High  
**Gap Reference:** GAP-005

**Current State:**

- Loads all documents
- No page controls

**Target State:**

```
┌─────────────────────────────────────────────────────────────┐
│  Documents (156)                                   [🔄] [🗑️] │
├─────────────────────────────────────────────────────────────┤
│  [All ▼] [Sort: Updated ▼ ⬇️]              [🔍 Search...]   │
├─────────────────────────────────────────────────────────────┤
│  │☐│ Title           │ Status    │ Entities │ Updated      │
│  │─│─────────────────│───────────│──────────│──────────────│
│  │☐│ Report Q4 2024  │ ✅ Done   │ 45       │ 2 hours ago  │
│  │☐│ Meeting Notes   │ 🔄 Process│ --       │ 5 mins ago   │
│  │☐│ Research Paper  │ ⏳ Pending│ --       │ Just now     │
├─────────────────────────────────────────────────────────────┤
│  Rows: [10 ▼]         Page 1 of 16           [◀️] 1 2 3 [▶️]  │
└─────────────────────────────────────────────────────────────┘
```

**Key UX Elements:**

- Page size selector (10, 20, 50, 100)
- Page navigation with current/total
- Row count display
- Keyboard navigation (← →)

---

#### 2.2 Status Filtering

**Priority:** High  
**Gap Reference:** GAP-006

**Filter Pills Design:**

```
┌─────────────────────────────────────────────────────────────┐
│  [All (156)] [✅ Done (120)] [🔄 Processing (5)]            │
│  [⏳ Pending (28)] [❌ Failed (3)]                           │
└─────────────────────────────────────────────────────────────┘
```

**Features:**

- Active filter highlighted
- Count badges on each filter
- URL state sync for sharing

---

#### 2.3 Bulk Actions

**Priority:** Medium  
**New Enhancement**

**When items selected:**

```
┌─────────────────────────────────────────────────────────────┐
│  ☑️ 5 selected                                              │
│  [🗑️ Delete] [🔄 Reprocess] [❌ Cancel]                      │
└─────────────────────────────────────────────────────────────┘
```

---

#### 2.4 Pipeline Status Indicator

**Priority:** High  
**Gap Reference:** GAP-007

**Header Indicator:**

```
┌─────────────────────────────────────────────────────────────┐
│  EdgeQuake    [Docs] [Graph] [Query]      🔴 Pipeline Busy  │
│                                           └──────────────┘   │
│                                           Click for details  │
└─────────────────────────────────────────────────────────────┘
```

**Status Dialog:**

```
┌─────────────────────────────────────────────────────────────┐
│  📊 Pipeline Status                                    [×]  │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  Job: Document Processing                                    │
│  Started: 2:34:15 PM                                         │
│                                                              │
│  Progress:                                                   │
│  ████████████░░░░░░░░ 60%                                   │
│  12 of 20 documents processed                                │
│                                                              │
│  Recent Messages:                                            │
│  ┌────────────────────────────────────────────────────┐     │
│  │ Processing: Research Paper.pdf                      │     │
│  │ Extracted 45 entities, 23 relationships             │     │
│  │ Processing: Meeting Notes.docx                      │     │
│  └────────────────────────────────────────────────────┘     │
│                                                              │
│                                       [Cancel Pipeline]      │
└─────────────────────────────────────────────────────────────┘
```

---

### 3. Graph Visualization UX

#### 3.1 Node Search

**Priority:** High  
**Gap Reference:** GAP-004

**Search Popover:**

```
┌─────────────────────────────────────────────────────────────┐
│  🔍 Search nodes...                                          │
├─────────────────────────────────────────────────────────────┤
│  🔵 Sarah Chen                    Person                     │
│  🔵 Dr. Sarah Martinez            Person                     │
│  🟢 Quantum Computing             Technology                 │
│  🟡 Research Lab                  Organization               │
├─────────────────────────────────────────────────────────────┤
│  Showing 4 of 12 matches • Press ↵ to select                │
└─────────────────────────────────────────────────────────────┘
```

**Features:**

- Fuzzy matching (MiniSearch)
- Color-coded by entity type
- Keyboard navigation (↑↓↵)
- Camera focus on selection

---

#### 3.2 Node Drag Feedback

**Priority:** High  
**Gap Reference:** GAP-002

**Visual Feedback:**

- Node highlights on hover
- Cursor changes to grab/grabbing
- Connected edges follow node
- Snap-back on release (optional)

---

#### 3.3 Layout Selector

**Priority:** Medium  
**Gap Reference:** GAP-003

**Dropdown Design:**

```
┌─────────────────────────────────────────────────────────────┐
│  Layout: [Force Atlas ▼]                                    │
│          ├─────────────────────────────────────────┐        │
│          │ ⚡ Force Atlas    (Current)              │        │
│          │ ⭕ Circular       Nodes in a circle      │        │
│          │ 📦 Circle Pack   Packed circles          │        │
│          │ 🎲 Random        Random placement        │        │
│          └─────────────────────────────────────────┘        │
└─────────────────────────────────────────────────────────────┘
```

---

#### 3.4 Graph Legend

**Priority:** Low  
**Gap Reference:** GAP-020

```
┌───────────────────────────┐
│  Legend                   │
├───────────────────────────┤
│  🔵 Person       (23)     │
│  🟢 Technology   (15)     │
│  🟡 Organization (8)      │
│  🟣 Location     (5)      │
│  ⚪ Other        (12)     │
├───────────────────────────┤
│  📏 Node size = Importance│
│  ━━ Edge = Relationship   │
└───────────────────────────┘
```

---

### 4. Query Interface UX

#### 4.1 Thinking Display

**Priority:** High  
**Gap Reference:** GAP-010

**Collapsed State:**

```
┌─────────────────────────────────────────────────────────────┐
│  🤔 Thought for 3.2s                                    [▼] │
└─────────────────────────────────────────────────────────────┘
```

**Expanded State:**

```
┌─────────────────────────────────────────────────────────────┐
│  🤔 Thought for 3.2s                                    [▲] │
├─────────────────────────────────────────────────────────────┤
│  │ The user is asking about quantum computing research.     │
│  │ I should look for entities related to:                   │
│  │ - Quantum computing concepts                             │
│  │ - Researchers in the field                               │
│  │ - Recent publications                                    │
│  │                                                          │
│  │ Let me search the knowledge graph for relevant nodes...  │
└─────────────────────────────────────────────────────────────┘
```

**During Thinking:**

```
┌─────────────────────────────────────────────────────────────┐
│  🔄 Thinking...                                         [▼] │
│  │ The user is asking about quantum computing research.     │
│  │ I should look for...█                                    │
└─────────────────────────────────────────────────────────────┘
```

---

#### 4.2 LaTeX Rendering

**Priority:** High  
**Gap Reference:** GAP-008

**Before (Raw Text):**

```
The equation is: $E = mc^2$ and $$\int_0^\infty e^{-x} dx = 1$$
```

**After (Rendered):**

```
The equation is: E = mc² and

     ∫₀^∞ e⁻ˣ dx = 1
```

---

#### 4.3 Mermaid Diagrams

**Priority:** High  
**Gap Reference:** GAP-009

**Rendered Example:**

```
┌─────────────────────────────────────────────────────────────┐
│  Here's the system architecture:                            │
│                                                              │
│         ┌──────────┐                                        │
│         │  Client  │                                        │
│         └────┬─────┘                                        │
│              │                                              │
│              ▼                                              │
│         ┌──────────┐      ┌──────────┐                     │
│         │   API    │─────▶│ Database │                     │
│         └────┬─────┘      └──────────┘                     │
│              │                                              │
│              ▼                                              │
│         ┌──────────┐                                        │
│         │   LLM    │                                        │
│         └──────────┘                                        │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

---

#### 4.4 Query Mode Prefix

**Priority:** Medium  
**Gap Reference:** GAP-017

**Input Hint:**

```
┌─────────────────────────────────────────────────────────────┐
│  💬 Ask a question...                                       │
│  Tip: Start with /local, /global, /hybrid for mode override │
└─────────────────────────────────────────────────────────────┘
```

**Mode Indicator:**

```
┌─────────────────────────────────────────────────────────────┐
│  /global What are the main research topics?                 │
│  ↳ Using Global mode                                        │
└─────────────────────────────────────────────────────────────┘
```

---

#### 4.5 User Prompt History

**Priority:** Medium  
**Gap Reference:** GAP-014

**History Dropdown:**

```
┌─────────────────────────────────────────────────────────────┐
│  📝 System prompt...                                    [▼] │
├─────────────────────────────────────────────────────────────┤
│  Recent Prompts:                                            │
│  ├─ Answer concisely with bullet points           [×]       │
│  ├─ Provide detailed analysis with sources        [×]       │
│  └─ Summarize in one paragraph                    [×]       │
└─────────────────────────────────────────────────────────────┘
```

---

### 5. Feedback & Status

#### 5.1 Connection Status

**Current:** Basic dot indicator  
**Enhanced:**

```
┌─────────────────────────────────────────────────────────────┐
│  🟢 Connected to EdgeQuake v0.1.0                           │
│     Last sync: 2 seconds ago                                │
└─────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────┐
│  🔴 Disconnected                                            │
│     Attempting to reconnect... [Retry Now]                  │
└─────────────────────────────────────────────────────────────┘
```

---

#### 5.2 Empty States

**Consistent empty state design:**

```
┌─────────────────────────────────────────────────────────────┐
│                                                              │
│                    📄                                        │
│                                                              │
│              No documents yet                                │
│                                                              │
│     Upload documents to build your knowledge graph.          │
│                                                              │
│              [📤 Upload Documents]                           │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

---

## Accessibility Considerations

### WCAG 2.1 Compliance Targets

| Level | Requirement          | Implementation                     |
| ----- | -------------------- | ---------------------------------- |
| A     | Keyboard accessible  | All interactive elements focusable |
| A     | Alt text             | Icons have aria-labels             |
| A     | Color contrast       | 4.5:1 minimum ratio                |
| AA    | Focus visible        | Clear focus indicators             |
| AA    | Error identification | Form errors clearly marked         |
| AAA   | Enhanced contrast    | 7:1 ratio option                   |

### Implementation Checklist

- [ ] All buttons have accessible names
- [ ] Form inputs have labels
- [ ] Dialogs trap focus
- [ ] Skip to content link
- [ ] Announcements for dynamic content (aria-live)
- [ ] Reduced motion preference respected

---

## Responsive Design

### Breakpoints

| Breakpoint | Width      | Layout                  |
| ---------- | ---------- | ----------------------- |
| Mobile     | < 640px    | Stacked, hamburger menu |
| Tablet     | 640-1024px | Collapsed sidebar       |
| Desktop    | > 1024px   | Full sidebar            |

### Mobile-Specific Considerations

| Component | Mobile Adaptation |
| --------- | ----------------- |
| Sidebar   | Sheet overlay     |
| Graph     | Touch gestures    |
| Tables    | Horizontal scroll |
| Dialogs   | Full screen       |
| Filters   | Collapsible       |

---

## Interaction Patterns

### Standard Patterns

| Pattern                  | Usage            |
| ------------------------ | ---------------- |
| Click to select          | Documents, nodes |
| Double-click to open     | Document details |
| Right-click context menu | Node actions     |
| Drag to move             | Node positioning |
| Scroll to zoom           | Graph zooming    |
| Hover for tooltip        | Additional info  |

### Loading States

| State              | Visual               |
| ------------------ | -------------------- |
| Initial load       | Skeleton             |
| Background refresh | Subtle spinner       |
| Action pending     | Button loading       |
| Error              | Red alert with retry |

### Confirmation Patterns

| Action          | Confirmation                 |
| --------------- | ---------------------------- |
| Delete document | Alert dialog                 |
| Clear all       | Alert dialog with text input |
| Cancel pipeline | Alert dialog                 |
| Merge entities  | Confirmation dialog          |

---

## Cross-References

| Document                                          | Relationship        |
| ------------------------------------------------- | ------------------- |
| [Gap Analysis](./002-gap-analysis.md)             | Source requirements |
| [Proposed Solutions](./003-proposed-solutions.md) | Implementation      |
| [Success Criteria](./008-success-criteria.md)     | UX metrics          |
| [Developer Guide](./009-developer-guide.md)       | Component usage     |

---

_Document defines user experience improvements and design guidelines_
