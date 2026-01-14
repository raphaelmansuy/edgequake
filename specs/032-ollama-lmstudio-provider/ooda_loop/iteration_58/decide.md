# OODA Loop Iteration 58 - Decide

## Decision Date
2025-01-27

## Decisions Made

### D1: Create Minimal Deeplink Routes

**Decision**: Create `/w/[slug]` routes that leverage existing components.

**Rationale**: 
- Reuse existing `WorkspacePage` logic
- Minimize code duplication
- Quick implementation

### D2: Route Structure

```
/w/[slug]           → Redirect to /w/[slug]/query
/w/[slug]/query     → Query interface for workspace
/w/[slug]/settings  → Workspace settings page
```

### D3: Implementation Details

1. **Slug Resolution Hook**:
   ```typescript
   // useWorkspaceBySlug.ts
   const useWorkspaceBySlug = (slug: string) => {
     const { selectedTenantId } = useTenantStore();
     return useQuery({
       queryKey: ['workspace', 'by-slug', selectedTenantId, slug],
       queryFn: () => getWorkspaceBySlug(selectedTenantId, slug),
       enabled: !!selectedTenantId && !!slug,
     });
   };
   ```

2. **Settings Page**: Reuse workspace page components with slug lookup

3. **Query Page**: Set workspace context and render query interface

## Acceptance Criteria

- [ ] `/w/{slug}` redirects to query page
- [ ] `/w/{slug}/query` shows query interface for workspace
- [ ] `/w/{slug}/settings` shows workspace settings
- [ ] 404 shown for invalid slugs
- [ ] Cross-tenant access prevented
- [ ] Navigation updates to show workspace-specific links
