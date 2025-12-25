# Task Log: Tenant/Workspace Screen Refresh

**Date:** 2024-12-24 16:05 UTC

## Actions

- Updated `document-manager.tsx` to include `selectedTenantId` and `selectedWorkspaceId` in query key
- Updated `graph-viewer.tsx` to include `selectedTenantId` and `selectedWorkspaceId` in query key
- Updated `app/page.tsx` to include tenant context in documents and graph query keys
- Updated `app/(dashboard)/page.tsx` to include tenant context in documents and graph query keys
- Verified React Query prefix matching works correctly for existing invalidation calls

## Decisions

- Used query key approach (include tenant/workspace IDs) rather than explicit invalidation on context change
- This is React Query best practice for context-dependent queries
- Existing `invalidateQueries({ queryKey: ['documents'] })` calls work correctly with prefix matching

## Next Steps

- All changes complete and tested
- Frontend now correctly refreshes data when tenant/workspace is changed
- No additional changes needed

## Lessons/Insights

- React Query uses prefix matching by default for `invalidateQueries`
- Query keys should include all context that affects the query result
- `useTenantStore` provides easy access to current tenant/workspace selection

## Files Modified

- `edgequake_webui/src/components/documents/document-manager.tsx`
- `edgequake_webui/src/components/graph/graph-viewer.tsx`
- `edgequake_webui/src/app/page.tsx`
- `edgequake_webui/src/app/(dashboard)/page.tsx`
