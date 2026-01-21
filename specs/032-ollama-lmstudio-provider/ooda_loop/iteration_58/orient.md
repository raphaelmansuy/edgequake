# OODA Loop Iteration 58 - Orient

## Analysis Date

2025-01-27

## Strategic Assessment

### Current Routing Architecture

```
edgequake_webui/src/app/
├── (auth)/                    # Auth routes
│   ├── login/
│   └── register/
├── (dashboard)/               # Dashboard routes (require auth)
│   ├── api-explorer/
│   ├── costs/
│   ├── documents/
│   ├── graph/
│   ├── query/                 # <-- Query page (not workspace-scoped)
│   ├── settings/
│   ├── workspace/             # <-- Workspace settings (tenant-scoped)
│   ├── layout.tsx
│   └── page.tsx
├── api/                       # API routes
└── layout.tsx
```

### Target Routing Architecture

```
edgequake_webui/src/app/
├── (auth)/...
├── (dashboard)/...
├── w/                         # NEW: Workspace deeplinks
│   └── [slug]/               # Dynamic route by workspace slug
│       ├── page.tsx          # Redirect to query or overview
│       ├── query/
│       │   └── page.tsx      # Workspace-scoped query
│       └── settings/
│           └── page.tsx      # Workspace settings by slug
└── ...
```

### Design Decisions

#### D1: Use `/w/[slug]` Pattern

- Short, memorable URL: `/w/my-project/query`
- Follows industry patterns (GitHub: `/u/username`, Slack: `/archives/channel`)
- Supports bookmarking specific workspaces

#### D2: Keep Dashboard Routes for Tenant-Level Views

- `/workspace` shows current tenant's default workspace settings
- `/w/my-slug/settings` shows specific workspace settings by slug
- Coexist for backward compatibility

#### D3: Use API to Resolve Slug to Workspace

- Frontend calls `getWorkspaceBySlug(tenantId, slug)` API
- API already exists in backend ([workspaces.rs](../../../../edgequake/crates/edgequake-api/src/handlers/workspaces.rs))
- Client API function exists ([edgequake.ts#getWorkspaceBySlug](../../../../edgequake_webui/src/lib/api/edgequake.ts))

### Implementation Plan

1. Create `/w` route group outside dashboard (public-ish with auth guard)
2. Create `[slug]` dynamic route
3. Create `settings/page.tsx` that:
   - Gets slug from params
   - Fetches workspace by slug
   - Renders workspace settings form
4. Create `query/page.tsx` that:
   - Gets slug from params
   - Sets workspace context
   - Renders query interface

### Risk Assessment

| Risk                     | Impact | Mitigation                     |
| ------------------------ | ------ | ------------------------------ |
| Slug not found           | Medium | 404 page with helpful message  |
| Cross-tenant access      | High   | Verify tenant ownership in API |
| Breaking existing routes | Low    | Keep existing routes unchanged |
