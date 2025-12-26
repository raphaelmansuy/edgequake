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
