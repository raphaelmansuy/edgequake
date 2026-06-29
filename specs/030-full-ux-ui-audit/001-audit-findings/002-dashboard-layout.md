# Audit: Dashboard Layout

**Component:** `src/app/(dashboard)/page.tsx`, `src/components/dashboard/`  
**Route:** `/`  
**Screenshot:** `e2e/screenshots/01-dashboard.png`

---

## Current Layout (ASCII)

```
┌─ Page (full width, padded) ─────────────────────────────────────────────────┐
│  Dashboard                                                                   │
│  Welcome to EdgeQuake - Your Knowledge Graph RAG Platform                   │
│  ──────────────────────────────────────────────────────────────────────────  │
│  ┌───────────┐ ┌───────────┐ ┌───────────┐ ┌───────────┐                   │
│  │ Documents │ │ Entities  │ │Relations  │ │Ent. Types │  ← stats grid      │
│  │    56     │ │  42,394   │ │  51,776   │ │   1,750   │                   │
│  └───────────┘ └───────────┘ └───────────┘ └───────────┘                   │
│  ─────────────────────────────────────────────────────────────────────────── │
│  ┌── Quick Actions ──────────────────────────────────────────────────────┐  │
│  │  [Blue-tint: Upload] [Purple-tint: Query] [Green-tint: View Graph]    │  │
│  └──────────────────────────────────────────────────────────────────────┘  │
│  ─────────────────────────────────────────────────────────────────────────── │
│  ┌── Recent Activity (70% width) ──┐  ┌─ System Status (30%) ─────────┐   │
│  │  document list items...         │  │  ● All systems operational    │   │
│  └─────────────────────────────────┘  └──────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## Findings

### F-DB-01 · Quick Actions use arbitrary color tints · MED
**Problem:** Blue/purple/green gradient cards are decorative noise. They don't carry semantic meaning and violate the minimalist style target.  
**Code ref:** `quick-actions.tsx` — `bgColor: 'bg-blue-500/10 hover:bg-blue-500/20'`  
**Principle:** "Saturate sparingly. Color should carry meaning." (Refactoring UI)  
**Fix:** Replace color tints with neutral bordered cards. Use a single accent color (primary) for hover state. Icons can carry the type distinction.

```
Current (colorful):              Target (minimal):
┌── blue tint ──┐                ┌─────────────────┐
│   📄 blue     │                │  ↑ Upload       │
│ Upload Docs   │   →            │  Add documents  │
│               │                │                 │
└───────────────┘                └─────────────────┘ (hover: subtle ring)
```

### F-DB-02 · Welcome subtitle text is generic filler · LOW
**Problem:** "Welcome to EdgeQuake - Your Knowledge Graph RAG Platform" adds no value on every visit. It's marketing copy on an operational dashboard.  
**Fix:** Replace with contextual status: "Default Workspace · 56 documents · Last activity: 31 min ago"

### F-DB-03 · Stats cards have no micro-interaction differentiation · LOW
**Problem:** All 4 stats cards have the same shadow-on-hover. There's no distinct visual encoding for the stat type beyond icon color.  
**Fix:** Consider subtle left-border accent color per variant (`border-l-2 border-l-blue-500` for documents, etc.).

### F-DB-04 · System Status widget is undersized relative to the column it sits in · MED
**Problem:** A single line "● All systems operational · ollama" in a full column panel looks unfinished — too much whitespace for too little content.  
**Code ref:** `src/components/dashboard/system-status.tsx`  
**Fix:** Either collapse it to a header badge OR expand it to show the health breakdown (storage, LLM, vector).

### F-DB-05 · Page title "Dashboard" lacks context anchoring · LOW
**Problem:** "Dashboard" is the page title everywhere. When a workspace has a real name, the heading should reflect it: "Default Workspace — Overview".

### F-DB-06 · No empty-state for first-time users (zero documents) · MED
**Problem:** If stats are all 0, four "0" cards with no guidance creates a dead-end UX.  
**Code ref:** `StatsCard` does have `zeroHint` prop but it's not prominently surfaced.  
**Fix:** When `documentValue === 0`, show a hero empty-state with a single CTA "Upload your first document".

### F-DB-07 · Scrollable page uses ScrollArea inside an overflow-hidden parent · PERF
**Problem:** The `ScrollArea` component wraps native scroll. This can cause focus-management issues and double scrollbar appearance on some OS.  
**Code ref:** `page.tsx` uses `<ScrollArea className="h-full">`.

---

## Proposed Layout (ASCII)

```
┌─ Page ──────────────────────────────────────────────────────────────────────┐
│  Default Workspace  ·  56 documents  ·  last activity 31 min ago            │  ← contextual header
│  ─────────────────────────────────────────────────────────────────────────  │
│  ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────┐                          │
│  │ 56 docs │ │ 42k ent │ │ 51k rel │ │1,750 typ│   ← clean neutral cards   │
│  │ ▲ +2    │ │ ▲ +1.2k │ │ ▲ +3k  │ │ — same │                           │
│  └─────────┘ └─────────┘ └─────────┘ └─────────┘                          │
│  ─────────────────────────────────────────────────────────────────────────  │
│  ┌─ Activity ── 65% ───────────────────────────┐  ┌─ Actions ─ 35% ──────┐ │
│  │  residual-risk-matrix.pdf   Completed  31m  │  │  → Upload Documents  │ │
│  │  math_2605.22763v1.pdf      Completed  39m  │  │  → Query Knowledge   │ │
│  │  ...                                        │  │  → Explore Graph     │ │
│  │  [ See all 56 documents → ]                 │  │  ──────────────────  │ │
│  └─────────────────────────────────────────────┘  │  ● All systems OK   │ │
│                                                    │    ollama · v0.12   │ │
│                                                    └───────────────────── │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## Summary Score

| Dimension             | Score | Notes                                 |
| --------------------- | ----- | ------------------------------------- |
| Information Hierarchy | 6/10  | Stats → actions → activity — OK order |
| Visual Noise          | 5/10  | Color tints add noise                 |
| Empty State           | 4/10  | No first-time flow                    |
| Typography            | 7/10  | Good scale, generic title             |
| Layout Balance        | 6/10  | Status column underused               |
