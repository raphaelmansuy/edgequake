# OODA-37: API Client Integration

**Date**: 2025-01-27  
**Focus**: Frontend API Layer

## OBSERVE

### API Client Structure

```typescript
// edgequake_webui/src/lib/api/edgequake.ts

export const edgequakeApi = {
  documents: {
    // List all documents
    listDocuments: async () => { ... },

    // Get single document
    getDocument: async (id: string) => { ... },

    // Get PDF download URL
    getPdfDownloadUrl: (id: string) =>
      `${API_BASE_URL}/documents/pdf/${id}/download`,

    // Get PDF content (markdown)
    getPdfContent: async (id: string) => {
      const response = await fetch(
        `${API_BASE_URL}/documents/pdf/${id}/content`,
        { headers: getHeaders() }
      );
      return response.json();
    },
  },
  // ... other namespaces
};
```

### Header Management

```typescript
const getHeaders = () => ({
  "Content-Type": "application/json",
  "X-Workspace-ID": getWorkspaceId(),
  Authorization: getAuthToken(),
});
```

### TanStack Query Integration

```typescript
// In components
const { data, isLoading, error } = useQuery({
  queryKey: ["document", documentId],
  queryFn: () => edgequakeApi.documents.getDocument(documentId),
});
```

## ORIENT

### First Principle: Type-Safe API Access

- Centralized API client reduces duplication
- TypeScript ensures correct usage
- Query keys enable caching

### API Client Patterns

1. ✅ Namespace organization (documents, entities, etc.)
2. ✅ Consistent error handling
3. ✅ Header injection for auth/tenancy
4. ✅ URL construction centralized

## DECIDE

**Decision**: API client architecture is sound

### Rationale

- Single source of truth for API endpoints
- Easy to update base URL
- Headers automatically included
- TanStack Query handles caching/invalidation

## ACT

### Verification Points

- ✅ PDF download URL correctly constructed
- ✅ PDF content endpoint returns markdown
- ✅ Workspace ID header included
- ✅ Error states propagate correctly

### Usage Example

```typescript
// DocumentViewerDialog.tsx
const { data: pdfContent } = useQuery({
  queryKey: ["pdf-content", documentId],
  queryFn: () => edgequakeApi.documents.getPdfContent(documentId),
  enabled: !!documentId,
});

// Access markdown
const markdown = pdfContent?.markdown;
```

### Error Handling

```typescript
const { error } = useQuery(...);

if (error) {
  // Display user-friendly error
  toast.error('Failed to load document');
}
```

**Status**: ✅ COMPLETE - API integration verified
