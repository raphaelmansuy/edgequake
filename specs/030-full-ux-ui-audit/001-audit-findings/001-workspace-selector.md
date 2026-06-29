# Audit: Workspace / Tenant Selector

**Component:** `src/components/layout/header-tenant-selector.tsx`  
**Route:** All routes (header-mounted)  
**Screenshot:** `e2e/screenshots/01-dashboard.png`

---

## Current State (ASCII Diagram)

```
┌─ Header Bar ──────────────────────────────────────────────────────────────────┐
│  [≡]  [ □ Default / Default Workspace ▾ ]          API v0.12.11  ☀  🌐  👤   │
└───────────────────────────────────────────────────────────────────────────────┘
         │
         ▼  (dropdown opens)
┌─ DropdownMenu ──────────────────────────┐
│  ORGANIZATIONS                          │
│  ● Default                              │
│  + New Organization                     │
│  ─────────────────────                  │
│  WORKSPACES (Default)                   │
│  ● Default Workspace                    │
│  + New Workspace                        │
└─────────────────────────────────────────┘
```

---

## Findings

### F-WS-01 · No fuzzy/filter search · CRITICAL
**Problem:** With 10+ tenants or 20+ workspaces the list becomes unscrollable noise.  
DropdownMenu has no built-in search; the user must visually scan.  
**Principle:** Progressive disclosure + efficiency for power users (Nielsen #7).  
**Reference:** [cmdk patterns](https://cmdk.paco.me/) — Command palette pattern is the established solution for filterable lists.

```
Current:                        Should be:
┌─ Dropdown ──────┐             ┌─ Popover + Command ──────────┐
│ Default         │             │  🔍 Search workspaces...     │
│ Production      │             │  ─────────────────────────── │
│ Staging         │    →        │  ORGANIZATIONS               │
│ Dev-EU          │             │  ● Default ✓                 │
│ Dev-US          │             │  ○ Production                │
│ ...20 more...   │             │  ○ Staging                   │
└─────────────────┘             │  ─────────────────────────── │
                                │  WORKSPACES                  │
                                │  ● My Workspace ✓            │
                                └──────────────────────────────┘
```

### F-WS-02 · No deeplink for tenant context · HIGH
**Problem:** Switching tenant is not reflected in the URL. A shared link always opens the first workspace, breaking collaboration and bookmarking.  
**Code ref:** `src/stores/use-tenant-store.ts` stores selection in localStorage only.  
**Fix:** Add `?t=<tenantSlug>` param alongside `?workspace=<slug>`.

### F-WS-03 · Display name truncation is fragile · MED
**Problem:** Manual `.slice(0, 15)` and `.slice(0, 20)` in `displayName` computation produces unpredictable ellipsis positions.  
**Code ref:** `header-tenant-selector.tsx` lines ~355–365.  
```ts
// current (fragile)
const tenantPart = selectedTenant.name.length > 15
  ? selectedTenant.name.slice(0, 15) + '...'
  : selectedTenant.name;
```
**Fix:** Use CSS `truncate` with a fixed `max-w` instead of JS slicing. The tooltip provides the full name for long values.

### F-WS-04 · Trigger button has no visual affordance for current workspace · LOW
**Problem:** FolderKanban icon + text only. No clear indication of "tenant" vs "workspace" hierarchy in the button label.  
**Fix:** Two-line compact display:  
```
[ ≡ Tenant Name      ]
[   Workspace Name ▾ ]
```

### F-WS-05 · Keyboard access to selector requires 2 clicks · MED
**Problem:** The trigger button opens DropdownMenu. Finding a workspace requires mouse scanning. No keyboard shortcut to open selector.  
**Fix:** Add `Cmd+K`-style shortcut or at minimum `Ctrl+Shift+W` to focus selector. The Command component handles keyboard navigation natively.

### F-WS-06 · No empty state for zero workspaces · LOW
**Problem:** If a tenant has no workspaces, the menu shows an empty section with only "+ New Workspace". No guidance.  
**Fix:** Show an inline illustration + call to action.

---

## Summary Score

| Dimension            | Score | Notes                             |
| -------------------- | ----- | --------------------------------- |
| Discoverability      | 4/10  | No search                         |
| Keyboard Navigation  | 5/10  | Focus trap but no shortcut        |
| Progressive Disclose | 4/10  | All items shown flat              |
| Context Clarity      | 6/10  | "Tenant / Workspace" format helps |
| Deep-linking         | 2/10  | localStorage only                 |
