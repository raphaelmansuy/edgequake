# OODA-66: URL Routing Structure

**Date**: 2026-02-01
**Focus**: Document Page Routing

## OBSERVE

### Mission Requirements (Re-read from specs/002-unify-ingestion-pipeline.md)
- Clean URL structure
- Deep linking support

### Current Route Structure

**App Router Pages:**
```
app/
├── (dashboard)/
│   ├── documents/
│   │   ├── page.tsx          # Document list
│   │   └── [id]/
│   │       └── page.tsx      # Document detail
│   ├── graph/
│   │   └── page.tsx          # Knowledge graph view
│   └── query/
│       └── page.tsx          # Query interface
```

**Route Patterns:**
| Route | Page | Purpose |
|-------|------|---------|
| /documents | DocumentList | View all documents |
| /documents/:id | DocumentDetail | Single document view |
| /graph | GraphView | Knowledge graph |
| /graph?entity=X | GraphView | Focus on entity |
| /query | QueryPage | Ask questions |

## ORIENT

### Navigation Patterns

```
[Document List]
     ↓ (double-click)
[/documents/:id] ← Document Detail Page
     ↓ (View in Graph)
[/graph?entity=:entity_name]
```

### Link Generation

```typescript
const documentLink = (id: string) => `/documents/${id}`;
const graphEntityLink = (entity: string) => `/graph?entity=${encodeURIComponent(entity)}`;
const editLink = (id: string) => `/documents/${id}/edit`;
```

## DECIDE

**Decision**: Routing structure is correct

The Next.js App Router provides:
- File-based routing
- Dynamic segments with [id]
- Query parameter support
- Automatic code splitting

## ACT

### Document Detail Page Route

```typescript
// app/(dashboard)/documents/[id]/page.tsx
import { use } from 'react';

interface PageProps {
  params: Promise<{ id: string }>;
}

export default function DocumentDetailPage({ params }: PageProps) {
  const { id } = use(params);
  
  // Fetch document using id
  const { data: document } = useQuery({
    queryKey: ['document', id],
    queryFn: () => getDocument(id, workspaceId),
  });
  
  return <DocumentDetail document={document} />;
}
```

### Navigation Usage

```typescript
// From document-manager.tsx
import { useRouter } from 'next/navigation';

const router = useRouter();

const handleDocumentDoubleClick = (doc: Document) => {
  router.push(`/documents/${doc.id}`);
};
```

**Status**: ✅ VERIFIED - URL routing complete
