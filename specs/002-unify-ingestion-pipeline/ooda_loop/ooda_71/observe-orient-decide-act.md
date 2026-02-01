# OODA-71: TypeScript Type Safety

**Date**: 2026-02-01
**Focus**: Frontend Type Definitions

## OBSERVE

### Mission Requirements (Re-read from specs/002-unify-ingestion-pipeline.md)
- Type-safe API integration
- Compile-time error detection

### Current Type Definitions

**Document Types (types/document.ts):**
```typescript
export interface Document {
  id: string;
  title: string;
  content: string;
  source_type: 'text' | 'pdf' | 'url';
  status: 'pending' | 'processing' | 'completed' | 'failed' | 'archived';
  pdf_id?: string;
  workspace_id: string;
  created_at: string;
  updated_at: string;
  entity_count?: number;
  relationship_count?: number;
  tags?: string[];
}

export interface DocumentUploadResponse {
  document_id: string;
  status: string;
}

export interface PdfUploadResponse {
  pdf_id: string;
  document_id?: string;
  markdown?: string;
}
```

## ORIENT

### Type Coverage

| Area | Status | Notes |
|------|--------|-------|
| API responses | ✅ Typed | Zod validation could add runtime checks |
| Component props | ✅ Typed | All props interfaces defined |
| Event handlers | ✅ Typed | React event types |
| Query keys | ⚠️ Partial | Could use @tanstack/query keys factory |

## DECIDE

**Decision**: Type system correctly configured

Benefits:
- Compile-time error detection
- IDE autocomplete
- Refactoring safety

## ACT

### Type-Safe Query Keys

```typescript
// queryKeys.ts
export const queryKeys = {
  documents: {
    all: (workspaceId: string) => ['documents', workspaceId] as const,
    detail: (id: string) => ['document', id] as const,
  },
  pdf: {
    content: (pdfId: string) => ['pdf-content', pdfId] as const,
  },
} as const;

// Usage
const { data } = useQuery({
  queryKey: queryKeys.documents.all(workspaceId),
  queryFn: () => getDocuments(workspaceId),
});
```

### API Response Validation (Optional)

```typescript
import { z } from 'zod';

const DocumentSchema = z.object({
  id: z.string(),
  title: z.string(),
  content: z.string(),
  status: z.enum(['pending', 'processing', 'completed', 'failed', 'archived']),
  // ...
});

const fetchDocument = async (id: string): Promise<Document> => {
  const response = await fetch(`/api/documents/${id}`);
  const data = await response.json();
  return DocumentSchema.parse(data); // Runtime validation
};
```

**Status**: ✅ VERIFIED - TypeScript properly configured
