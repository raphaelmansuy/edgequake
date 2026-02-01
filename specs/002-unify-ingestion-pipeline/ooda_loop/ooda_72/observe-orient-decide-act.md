# OODA-72: API Client Architecture

**Date**: 2026-02-01
**Focus**: Frontend API Integration

## OBSERVE

### Mission Requirements (Re-read from specs/002-unify-ingestion-pipeline.md)
- Consistent API communication
- Error handling

### Current API Client (lib/edgequake.ts)

```typescript
const API_URL = process.env.NEXT_PUBLIC_API_URL || 'http://localhost:3001/api';

// Base fetch wrapper
async function apiRequest<T>(
  endpoint: string,
  options: RequestInit = {}
): Promise<T> {
  const response = await fetch(`${API_URL}${endpoint}`, {
    ...options,
    headers: {
      'Content-Type': 'application/json',
      ...options.headers,
    },
  });
  
  if (!response.ok) {
    const error = await response.json();
    throw new APIError(error);
  }
  
  return response.json();
}

// Document endpoints
export const getDocuments = (workspaceId: string) =>
  apiRequest<Document[]>(`/documents?workspace_id=${workspaceId}`);

export const getDocument = (id: string, workspaceId: string) =>
  apiRequest<Document>(`/documents/${id}?workspace_id=${workspaceId}`);

export const uploadDocument = (data: UploadDocumentRequest) =>
  apiRequest<DocumentUploadResponse>('/documents', {
    method: 'POST',
    body: JSON.stringify(data),
  });
```

## ORIENT

### API Client Functions

| Function | Method | Endpoint |
|----------|--------|----------|
| getDocuments | GET | /documents |
| getDocument | GET | /documents/:id |
| uploadDocument | POST | /documents |
| uploadPdfDocument | POST | /pdf/upload |
| deleteDocument | DELETE | /documents/:id |
| getPdfDownloadUrl | - | /pdf/:id (URL only) |

## DECIDE

**Decision**: API client properly structured

Provides:
- Type-safe responses
- Consistent error handling
- URL construction

## ACT

### PDF Upload Function

```typescript
export async function uploadPdfDocument(
  file: File,
  options: PdfUploadOptions
): Promise<PdfUploadResponse> {
  const formData = new FormData();
  formData.append('file', file);
  formData.append('workspace_id', options.workspaceId);
  if (options.title) formData.append('title', options.title);
  
  const response = await fetch(`${API_URL}/pdf/upload`, {
    method: 'POST',
    body: formData,
    // No Content-Type header for FormData
  });
  
  if (!response.ok) {
    const error = await response.json();
    throw new APIError(error);
  }
  
  return response.json();
}
```

### URL Helper

```typescript
export function getPdfDownloadUrl(
  pdfId: string,
  workspaceId: string
): string {
  return `${API_URL}/pdf/${pdfId}?workspace_id=${workspaceId}`;
}
```

**Status**: ✅ VERIFIED - API client complete
