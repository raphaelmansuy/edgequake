# 001 — Sidebar & Navigation Audit

**First Principle: Economy** — Every navigation item must pay rent.

---

## Current State

### Sidebar Architecture

```
┌──────────────────────────────┐
│  ⬡ EdgeQuake         [< ]   │  ← Logo + collapse toggle
├──────────────────────────────┤
│  [Workspace Selector]        │  ← HeaderTenantSelector (duplicated!)
├──────────────────────────────┤
│  ⊞ Dashboard                │
│  ⋈ Knowledge Graph          │
│  ≡ Documents                │
│  ≋ Pipeline                 │
│  ☐ Query                    │
│  ☱ Workspace                │
│  $ Costs                    │
│  ≡ Knowledge                │  ← "Knowledge" AND "Knowledge Graph"?
│  > API Explorer             │
│  ⚙ Settings                 │
├──────────────────────────────┤
│  v0.1.0                     │  ← Version badge at bottom
└──────────────────────────────┘
```

**Issues:**
1. **10 flat items** — exceeds the 7±2 Miller's Law limit for cognitive load
2. **"Knowledge Graph" and "Knowledge"** — two visually similar items with confusing distinction
3. **"Workspace" and "Workspace Selector"** — the selector in header AND a workspace management page 
4. **"Pipeline"** — monitoring page, not a primary workflow; advanced users only
5. **"API Explorer"** — developer tool in primary nav, not discoverable enough to warrant P1 placement  
6. **"Costs"** — billing/analytics item mixed with core product features

### Collapsed Sidebar State

When collapsed, items show icons with tooltips (correct) but:
- Touch target is `min-h-[40px]` (40px) — WCAG 2.5.5 recommends 44×44px
- `TooltipProvider delayDuration={0}` is aggressive on mobile hover simulation

### Breadcrumb Redundancy

```
Current layout stack:
┌──────────────────────────────────────────────┐
│ Header: [☰] EdgeQuake | [WorkspaceSelector]  │  ← Location signal 1
├──────────────────────────────────────────────┤
│ Breadcrumb: EdgeQuake > Documents             │  ← Location signal 2
├──────────────────────────────────────────────┤
│ [Sidebar active state highlighted]           │  ← Location signal 3
└──────────────────────────────────────────────┘
```

Three separate "you are here" indicators. The breadcrumb is visible on `/documents` but the sidebar already highlights `Documents` in `bg-primary`. **For a flat navigation, breadcrumb is pure noise.**

Breadcrumb only earns its place at depth ≥ 2 (e.g., `/documents/[id]`).

---

## Identified Issues

| ID     | Severity | Issue                                                                      | Code Reference                    |
| ------ | -------- | -------------------------------------------------------------------------- | --------------------------------- |
| NAV-01 | High     | 10 flat nav items exceeds cognitive limit                                  | `sidebar.tsx:43` `navItems` array |
| NAV-02 | High     | Workspace selector duplicated (header + sidebar `HeaderTenantSelector`)    | `sidebar.tsx:97`, `header.tsx:47` |
| NAV-03 | Medium   | "Knowledge Graph" and "Knowledge" items confusingly similar                | `sidebar.tsx:44-45`               |
| NAV-04 | Medium   | Breadcrumb shown on depth-1 routes (redundant with sidebar active state)   | `dynamic-breadcrumb.tsx:69`       |
| NAV-05 | Medium   | Touch target 40px — below 44px WCAG 2.5.5 recommendation                   | `sidebar.tsx:105` `min-h-[40px]`  |
| NAV-06 | Low      | Version badge (`APP_VERSION`) has no semantic meaning for navigation       | `sidebar.tsx`                     |
| NAV-07 | Low      | Mobile drawer uses `Sheet` but no focus trap confirmation on large content | `sidebar.tsx`                     |
| NAV-08 | Low      | `DollarSign` icon for "Costs" — ambiguous, could be confused with pricing  | `sidebar.tsx:48`                  |

---

## Recommendations

### NAV-01 · Group and Reduce Navigation Items

**Principle:** Group related items. Expose secondary items via settings or a "More" overflow.

**Proposed grouped structure:**

```
┌──────────────────────────────────┐
│  ⬡ EdgeQuake            [< ]    │
├──────────────────────────────────┤
│  [Workspace: "default"    ▼]    │
├──────────────────────────────────┤
│  CORE WORKFLOWS                  │
│  ⊞ Dashboard                    │
│  ≡ Documents                    │
│  ☐ Query                        │
│  ⋈ Graph                        │
├ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─┤
│  KNOWLEDGE                       │
│  ≡ Knowledge Base               │
├ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─┤
│  SYSTEM  (subtle, smaller text) │
│  ≋ Pipeline                     │
│  > API                          │
│  $ Costs                        │
│  ⚙ Settings                     │
└──────────────────────────────────┘
```

**Code change — `sidebar.tsx`:**

```typescript
// BEFORE: flat navItems array
const navItems = [
  { href: '/', icon: Home, labelKey: 'nav.dashboard' },
  // ... 9 more items
];

// AFTER: grouped structure
const navGroups = [
  {
    label: null, // no header for primary group
    items: [
      { href: '/', icon: Home, labelKey: 'nav.dashboard' },
      { href: '/documents', icon: FileText, labelKey: 'nav.documents' },
      { href: '/query', icon: MessageSquare, labelKey: 'nav.query' },
      { href: '/graph', icon: Network, labelKey: 'nav.graph' },
    ],
  },
  {
    label: 'nav.groupKnowledge',
    items: [
      { href: '/knowledge', icon: BookOpen, labelKey: 'nav.knowledge' },
    ],
  },
  {
    label: 'nav.groupSystem',
    items: [
      { href: '/pipeline', icon: Activity, labelKey: 'nav.pipeline' },
      { href: '/api-explorer', icon: Terminal, labelKey: 'nav.apiExplorer' },
      { href: '/costs', icon: BarChart2, labelKey: 'nav.costs' }, // BarChart2 > DollarSign
      { href: '/settings', icon: Settings, labelKey: 'nav.settings' },
    ],
  },
];
```

**Visual treatment for groups:**

```
┌──────────────────────────────────┐
│  [Items]                         │  ← Core items: full opacity, normal weight
├ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─┤
│  SYSTEM          (10px, caps,   │  ← Group labels: muted-foreground,
│  muted-foreground)              │    text-[10px] uppercase tracking-wider
│  [Items]                         │    items: slightly smaller opacity
└──────────────────────────────────┘
```

### NAV-02 · Remove Workspace Selector Duplication

The `HeaderTenantSelector` appears in both the sidebar (`SidebarContent`) and the header. Pick **one canonical location**:

- **Sidebar** (recommended): The workspace context is persistent; it belongs in the sidebar where persistent state lives
- Remove `HeaderTenantSelector` from the header on desktop

### NAV-03 · Rename "Knowledge" to "Knowledge Base"

- Rename `/knowledge` nav item to **"Knowledge Base"** or **"Entities"**
- Rename `/graph` nav item to **"Graph"** (not "Knowledge Graph" — the word "Knowledge" appears twice)
- Updated icons: `Network` for Graph, `Database` or `Archive` for Knowledge Base

### NAV-04 · Conditionally Suppress Breadcrumb at Depth 1

```typescript
// dynamic-breadcrumb.tsx
// CURRENT: Don't show breadcrumbs on root page only
if (items.length <= 1) return null;

// FIX: Don't show breadcrumbs at depth 1 either
// (sidebar active state already communicates location)
if (items.length <= 2) return null; // depth 1 = items [EdgeQuake, Documents]
```

At depth 2+ (e.g., `/documents/[id]`), the breadcrumb is genuinely valuable.

### NAV-05 · Increase Touch Target Size

```typescript
// sidebar.tsx:105
// CURRENT:
className="... min-h-[40px] ..."

// FIX:
className="... min-h-[44px] ..." // WCAG 2.5.5 AAA: 44×44px
```

### NAV-08 · Replace `DollarSign` with `BarChart2` for Costs

`DollarSign` implies "pricing/billing" whereas the page is about usage analytics. `BarChart2` or `TrendingUp` communicates "analytics" more accurately.

---

## Reference Patterns

### Well-Executed Navigation Systems

| Product          | Pattern                                   | Why it works                                 |
| ---------------- | ----------------------------------------- | -------------------------------------------- |
| Linear.app       | 5 primary items, team items below divider | Clear hierarchy, muscle memory forms quickly |
| Vercel Dashboard | Grouped with section labels               | Context switching without cognitive overload |
| GitHub           | Global nav at top, repo nav at left       | Two-level hierarchy is natural               |
| Notion           | Sidebar with collapsible groups           | Progressive disclosure of secondary items    |

### ARIA Requirements for Navigation

```html
<!-- CORRECT (already implemented in sidebar.tsx) -->
<nav aria-label="Main navigation">
  <a href="/" aria-current="page">Dashboard</a>
</nav>

<!-- ALSO NEEDED: landmark for sidebar wrapper -->
<aside aria-label="Application sidebar">
  <nav aria-label="Main navigation">
    ...
  </nav>
</aside>
```

The sidebar's outer `<div>` should become `<aside>` for proper landmark semantics.

---

## Measurement Criteria

- [ ] Navigation item count ≤ 7 in primary group
- [ ] Touch targets ≥ 44×44px in all interactive elements
- [ ] Breadcrumb hidden at depth ≤ 1
- [ ] Single canonical workspace selector location
- [ ] `<aside>` landmark wrapping sidebar
- [ ] Grouped items with `<nav>` sub-sections where appropriate
- [ ] Screen reader announces current page (`aria-current="page"`)

---

## External References

- [Miller's Law — 7±2](https://www.nngroup.com/articles/chunking/)
- [Navigation — WCAG 2.4.1](https://www.w3.org/WAI/WCAG21/Understanding/bypass-blocks)
- [WCAG 2.5.5 Target Size](https://www.w3.org/WAI/WCAG21/Understanding/target-size.html)
- [Sidebar navigation patterns — NNGroup](https://www.nngroup.com/articles/vertical-nav/)
- [ARIA Landmark Regions](https://www.w3.org/WAI/ARIA/apg/patterns/landmarks/)
