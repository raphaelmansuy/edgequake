# UX/UI Mapping Scratchpad

Append-only raw notes and observations during mapping.

---

2025-12-26 UTC — Initial assessment: edgequake_webui is Next.js 16.1.0 + React 19, uses Radix UI + Tailwind CSS v4. App router structure with (dashboard) and (auth) route groups.

2025-12-26 UTC — Routes identified:

- `/` — Dashboard (dashboard route group)
- `/documents` — Document management
- `/query` — RAG query interface
- `/graph` — Knowledge graph visualization (Sigma.js)
- `/settings` — Settings (TODO)
- `/api-explorer` — API Explorer (TODO)
- `/login` — Auth login (TODO)
- `/select-tenant` — Tenant selection (TODO - may be part of login flow)

2025-12-26 UTC — Existing screenshots: dashboard (3), documents (6), graph (10), query (16). Settings, api-explorer, login folders are empty.

2025-12-26 UTC — Component inventory is empty. Need to create: buttons.md, inputs.md, cards.md, dialogs.md, tables.md, navigation.md based on cross-references in page docs.

---

2025-12-26 10:45 UTC — Captured Settings page screenshots at 3 viewports (1440px desktop, 768px tablet, 375px mobile). Used Playwright MCP browser tools. Screenshots initially saved to `.playwright-mcp/`, then copied to `screenshots/settings/`.

2025-12-26 10:47 UTC — Captured API Explorer page screenshots. Captured both the endpoint list view and a view with an endpoint expanded to show request/response panels.

2025-12-26 10:48 UTC — Captured Login page screenshots at all 3 viewports.

2025-12-26 10:50 UTC — Created `pages/settings.md` with full region/container/component/element hierarchy. 4 main card sections: Appearance, Graph Visualization, Query Defaults, Data Management.

2025-12-26 10:52 UTC — Created `pages/api-explorer.md` documenting two-panel layout, HTTP method badges, collapsible endpoint categories.

2025-12-26 10:53 UTC — Created `pages/login.md` documenting auth flow, centered card layout, form fields.

2025-12-26 10:55 UTC — Created component inventory files:
- buttons.md — 6 variants, 6 sizes, badge variants
- inputs.md — Input, Textarea, Select, Switch, Checkbox
- cards.md — Standard, Stats, Danger, Message variants
- dialogs.md — Dialog, AlertDialog, Sheet, Popover, Dropdown, ContextMenu, Tooltip, Command, Collapsible
- tables.md — Table with sub-components
- navigation.md — Sidebar, Header, Breadcrumb, Tabs, ScrollArea, Separator

2025-12-26 10:57 UTC — Created 7 request JSON files in `requests/` directory.

2025-12-26 10:58 UTC — Updated README.md with complete page index table (all 7 pages) and component library table.

2025-12-26 10:59 UTC — Spec 14-ux-ui-mapping.md execution complete. All pages documented, all screenshots captured, component inventory created, request JSON files generated.
---

2025-12-26 22:30 UTC — UX/UI fixes session completed:

**Changes Implemented:**

1. **Tenant/Workspace Selector (header-tenant-selector.tsx)** — Complete rewrite with:
   - React Query hooks for fetching tenants/workspaces
   - useMutation hooks for create operations
   - Dialog components for "Create New Tenant" and "Create New Workspace" forms
   - Proper state management integration with useTenantStore
   - E2E tested and working

2. **Node Details Panel (node-details.tsx)** — Removed Card paradigm:
   - Changed from Card-based styling to flex layout
   - Added `h-full flex flex-col` for proper containment
   - Made content section scrollable with `flex-1 overflow-y-auto`
   - More compact header with `shrink-0`

3. **Query Interface (query-interface.tsx)** — Multiple improvements:
   - Removed "No Documents Yet" warning section
   - Made header more compact (single line with title and subtitle inline)
   - Replaced all purple/violet colors with primary/muted colors:
     - Empty state sparkles icon gradient
     - Avatar gradients
     - Thinking section styling
     - Streaming bounce dots
     - Graph stats dot colors (purple → amber for relationships)
     - Brain icons in settings

4. **Thinking Display (thinking-display.tsx)** — Replaced purple colors:
   - Brain icon: purple-500 → muted-foreground
   - Border styling: purple-300/700 → muted-foreground/30

5. **Query Mode Selector (query-mode-selector.tsx)** — Color update:
   - Hybrid mode icon: purple-500 → primary

**Legend & Entity Scrolling:**
- Verified that graph-legend.tsx uses `<ScrollArea className="max-h-56">` for proper overflow
- Verified that entity-browser-panel.tsx uses `<ScrollArea className="flex-1">` with proper flex layout

**Search Input Padding:**
- Verified that graph-search.tsx uses proper CommandInput with `px-3 py-1` wrapper styling

**Visual Results:**
- Query page now has cohesive neutral/primary color scheme
- No more purple accent colors that conflict with design language
- Tenant selector is fully functional with E2E tested create dialogs
- Header is more compact and slick

---

2025-12-26 09:20 UTC — UX/UI fixes session #2 completed:

**Issues Addressed:**

1. **Mobile Tenant Selector (sidebar.tsx)** — Issue #1
   - Added HeaderTenantSelector to MobileSidebar
   - Import: `import { HeaderTenantSelector } from './header-tenant-selector';`
   - Positioned at top of mobile sheet with `border-b p-3` styling
   - Full-width dropdown with `w-full` class
   - Verified working at 375x812 viewport

2. **Graph Entity Browser Keyboard Navigation (entity-browser-panel.tsx)** — Issue #2
   - Added `focusedIndex` state and `listRef` to main component
   - Added `handleListKeyDown` callback for ArrowUp/Down/Home/End/Enter/Escape
   - Added `useEffect` to reset focusedIndex on search/sort changes
   - Updated EntityItem component with:
     - `isFocused` and `onKeyDown` props in interface
     - `itemRef` using useRef for focus management
     - `useEffect` for auto-scroll into view
     - `role="option"`, `aria-selected`, proper tabIndex
     - `focus-visible:ring-2 focus-visible:ring-primary` styling
   - List container has `role="listbox"`, `tabIndex={0}`, focus handlers

3. **Node Details Panel Cleanup (node-details.tsx, graph-viewer.tsx)** — Issue #3
   - Removed X close button from NodeDetails (panel header already has collapse button)
   - Removed nested scrolling by changing root to `space-y-2.5` (was `h-full flex flex-col`)
   - Content div changed from `flex-1 overflow-y-auto` to just `space-y-2.5`
   - Removed padding from NodeDetails header (padding in parent ScrollArea)
   - Removed wrapper div with `space-y-3` in graph-viewer.tsx
   - Removed unused `toggleNodeDetails` from store destructuring
   - Removed unused X icon import

4. **Documents Page Compact Header (document-manager.tsx)** — Issue #4
   - Changed page header from `text-3xl font-bold` to `text-xl font-semibold`
   - Changed subtitle from `text-base` to `text-sm`
   - Changed space-y-6 to space-y-4 for tighter spacing
   - Removed ScanDocumentsButton import and usage (hidden per request)
   - Documents card: added `border-0 shadow-sm` styling
   - CardHeader: added `py-3 px-4`, title with `text-sm font-medium`
   - CardContent: `p-0` with proper internal padding
   - Added `overflow-x-auto` wrapper for table
   - Pagination: moved to `p-4 border-t`

5. **Query Conversation History Mobile (conversation-history-panel.tsx)** — Issue #5
   - Added `hidden md:flex` to both collapsed and expanded panel states
   - Panel now only visible on desktop (md breakpoint = 768px)
   - Prevents history panel from overlapping content on mobile
   - Mobile query interface now uses full width

6. **Dashboard LLM Provider Info (system-status.tsx, health.rs, types/index.ts)** — Issue #6
   - Backend: Added `llm_provider_name: Option<String>` to HealthResponse
   - Backend: Import `edgequake_llm::traits::LLMProvider` trait
   - Backend: `state.llm_provider.name().to_string()` to get provider name
   - Frontend types: Added `llm_provider_name?: string` to HealthResponse
   - Frontend types: Made components union types for both old/new API formats
   - SystemStatus: Now shows provider name (e.g., "Openai", "Mock") instead of just "Available"
   - Storage section updated to handle boolean values from new API

**Files Modified:**
- `sidebar.tsx` — Added HeaderTenantSelector to MobileSidebar
- `entity-browser-panel.tsx` — Keyboard navigation, ARIA, focus management
- `node-details.tsx` — Removed close button, simplified layout
- `graph-viewer.tsx` — Removed wrapper div around NodeDetails
- `document-manager.tsx` — Compact header, removed scan button, better styling
- `conversation-history-panel.tsx` — Hidden on mobile
- `system-status.tsx` — Shows LLM provider name
- `types/index.ts` — Updated HealthResponse type
- `health.rs` — Added llm_provider_name field

**Visual Results:**
- Mobile views are much cleaner with proper layout
- Keyboard navigation in graph entity browser is fully functional
- Documents page is more compact and professional
- Dashboard shows actual LLM provider instead of just availability status