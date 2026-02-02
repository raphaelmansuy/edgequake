# OODA-70: React Query Hydration

**Date**: 2026-02-01
**Focus**: Server/Client State Sync

## OBSERVE

### Mission Requirements (Re-read from specs/002-unify-ingestion-pipeline.md)
- Fast initial page loads
- No layout shift

### TanStack Query Setup

**Provider (providers.tsx):**
```typescript
'use client';

import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { useState } from 'react';

export function Providers({ children }: { children: React.ReactNode }) {
  const [queryClient] = useState(
    () => new QueryClient({
      defaultOptions: {
        queries: {
          staleTime: 60 * 1000,
          refetchOnWindowFocus: false,
        },
      },
    })
  );
  
  return (
    <QueryClientProvider client={queryClient}>
      {children}
    </QueryClientProvider>
  );
}
```

## ORIENT

### Hydration Strategies

| Strategy | Use Case | Implementation |
|----------|----------|----------------|
| SSR fetch + hydrate | Critical data | Server Component + dehydrate |
| Client-only | Dynamic data | useQuery |
| Prefetch | Predictable navigation | prefetchQuery |

### Current Approach

EdgeQuake uses client-only fetching:
- Documents list: Client fetch on mount
- Document detail: Client fetch with loading state
- Reason: Multi-tenant data requires client context

## DECIDE

**Decision**: Client-only fetching is correct for multi-tenant app

Why:
1. Workspace ID from client context
2. No SSR auth issues
3. Simpler implementation
4. Skeleton loading provides good UX

## ACT

### Prefetch on Hover

```typescript
const DocumentRow = ({ doc }: { doc: Document }) => {
  const queryClient = useQueryClient();
  
  const handleMouseEnter = () => {
    // Prefetch document detail on hover
    queryClient.prefetchQuery({
      queryKey: ['document', doc.id],
      queryFn: () => getDocument(doc.id, workspaceId),
      staleTime: 30 * 1000,
    });
  };
  
  return (
    <TableRow 
      onMouseEnter={handleMouseEnter}
      onDoubleClick={() => router.push(`/documents/${doc.id}`)}
    >
      {/* ... */}
    </TableRow>
  );
};
```

### Optimistic Navigation

Combined with prefetch:
```typescript
// Navigation feels instant because data is prefetched
onDoubleClick={() => {
  // Data likely already cached from hover
  router.push(`/documents/${doc.id}`);
}}
```

**Status**: ✅ VERIFIED - Query state management correct
