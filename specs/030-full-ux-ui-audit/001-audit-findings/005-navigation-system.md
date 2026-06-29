# Audit: Navigation System

**Component:** `src/components/layout/sidebar.tsx`, `src/components/layout/header.tsx`  
**Screenshot:** `e2e/screenshots/01-dashboard.png`

---

## Current State

The sidebar starts in collapsed icon-only mode. Labels are shown only on expand (`>` button at bottom).

```
Collapsed:                    Expanded:
┌──┐                          ┌─────────────────┐
│🏠│                          │ EdgeQuake        │
│📄│                          │────────────────  │
│💬│                          │ 🏠 Dashboard     │
│🔗│                          │ 📄 Documents     │
│──│                          │ 💬 Query         │
│📚│                          │ 🔗 Graph         │
│📁│                          │────────────────  │
│──│                          │ Knowledge        │
│🌊│                          │ 📚 Knowledge     │
│📊│                          │ 📁 Workspace     │
│>_│                          │────────────────  │
│⚙ │                          │ System           │
│  │                          │ 🌊 Pipeline      │
│[>]│                         │ 📊 Costs         │
└──┘                          │ >_ API Explorer  │
                              │ ⚙ Settings       │
                              │                  │
                              │ [←] collapse     │
                              └─────────────────┘
```

---

## Findings

### F-NAV-01 · Sidebar collapsed by default kills discoverability · HIGH
**Problem:** New users see 10 icons with no labels. Tooltips require hover — they're not visible until interaction. This violates Nielsen #6 (Recognition over recall).  
**Fix:** Default to expanded mode. Persist user preference. On mobile, use a drawer instead of collapsed state.

### F-NAV-02 · No active route breadcrumb on deep pages · MED
**Problem:** Inside `/documents/[id]` or `/graph`, there's no clear indication of the page hierarchy.  
**Code ref:** `src/components/layout/dynamic-breadcrumb.tsx` exists but may not render at all depths.  
**Fix:** Ensure breadcrumbs show for all nested routes.

### F-NAV-03 · Keyboard shortcut hints not surfaced in navigation · LOW
**Problem:** The app has keyboard shortcuts (`useKeyboardShortcuts` hook) but they're not discoverable from the navigation.  
**Fix:** Add `?` shortcut to show help overlay. Show primary shortcuts in sidebar tooltips.

### F-NAV-04 · "Knowledge" and "Knowledge Base" navigation items overlap · LOW
**Problem:** `/knowledge` and `/workspace` are both under a "Knowledge" group. The relationship is unclear.

### F-NAV-05 · Bottom "N" avatar button has no accessible label · HIGH
**Problem:** The bottom-left avatar circle shows "N" with no accessible name or role description.  
**Code ref:** Bottom of sidebar in collapsed state.

### F-NAV-06 · Sidebar toggle button position is not thumb-friendly on mobile · LOW
**Problem:** The `[>]` / `[<]` toggle is at the bottom of the sidebar, far from the top where nav items appear.

---

## Summary Score

| Dimension         | Score | Notes                    |
| ----------------- | ----- | ------------------------ |
| Discoverability   | 5/10  | Icons alone insufficient |
| Keyboard Nav      | 7/10  | Good focus indicators    |
| Context Awareness | 6/10  | Breadcrumbs present      |
| Mobile            | 6/10  | Drawer exists            |
| Accessibility     | 6/10  | Some missing labels      |
