# Implementations Reference

## P0 Changes (Implemented)

### 1. Workspace Fuzzy Search Selector
**File:** `edgequake_webui/src/components/layout/header-tenant-selector.tsx`  
**Change:** Replaced `DropdownMenu` with `Popover` + `Command` (cmdk) pattern.  
**Result:** Users can now fuzzy-search across tenants and workspaces by typing in the Command palette search input.

### 2. Entity Label Formatting  
**Files:**
- `edgequake_webui/src/lib/graph/label-utils.ts` (NEW)
- `edgequake_webui/src/components/graph/entity-browser-panel.tsx`
- `edgequake_webui/src/components/graph/node-details.tsx`

**Change:** Added `formatEntityLabel()` and `formatEntityType()` utilities. Applied to entity browser list, entity type group headers, and node details panel.  
**Result:** Entity names now display as "Ab Carval Aviation Leasing..." instead of "AB_CARVAL_AVIATION_LEASING_FU".

### 3. Deeplinks for Tenant/Workspace
**File:** `edgequake_webui/src/hooks/use-workspace-url.ts`  
**Change:** Extended URL sync to also write `?tenant=<slug>` alongside `?workspace=<slug>`. Added init-from-URL logic to read both params on page load.  
**Result:** Sharing `/?tenant=default&workspace=my-project` preserves full context.

## P1 Changes (Implemented)

### 4. Dashboard Quick Actions — Remove Color Tints
**File:** `edgequake_webui/src/components/dashboard/quick-actions.tsx`  
**Change:** Removed `bg-blue-500/10`, `bg-purple-500/10`, `bg-green-500/10` color classes. Cards now use neutral `bg-card` with hover states.  
**Result:** Clean, minimalist action cards consistent with the design system.

### 5. Dashboard Contextual Header
**File:** `edgequake_webui/src/app/(dashboard)/page.tsx`  
**Change:** Page title now shows the workspace name (when available). Subtitle shows document/entity counts instead of generic marketing copy.

## Remaining Work (Future Iterations)

| Item                                 | File               | Priority |
| ------------------------------------ | ------------------ | -------- |
| Sidebar expanded by default          | `sidebar.tsx`      | P1       |
| Documents pagination                 | `documents/` page  | P1       |
| "Clear All" button to danger zone    | `documents/` page  | P0       |
| Graph toolbar grouping with dividers | `graph-viewer.tsx` | P1       |
| Minimap show/hide toggle             | `graph-viewer.tsx` | P2       |
