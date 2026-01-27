# EdgeQuake WebUI API Integration

> Standards for API communication, data fetching, streaming, and real-time updates.

**Version**: 1.0.0 | **Last Updated**: 2026-01-09

---

## 1. API Client Strategy

The WebUI interacts with the EdgeQuake Backend via two primary channels:
1.  **REST API (HTTP/2)**: For CRUD operations, search, and graph data.
2.  **WebSockets (WS)**: For long-running job progress (ingestion) and system notifications.

All HTTP requests are routed through a central `client.ts` wrapper to ensure consistency in:
-   Authentication injection (Bearer tokens).
-   Base URL handling.
-   Error normalization.
-   Tenant/Workspace context headers.

---

## 2. The API Client

Located at `src/lib/api/client.ts`, the client exports a typed wrapper around `fetch`.

### 2.1 Configuration

The client automatically determines the base URL:
1.  `NEXT_PUBLIC_API_URL` environment variable (for production/Docker).
2.  Falls back to `/api/v1` (relative) for local development with proxying.

```typescript
// Usage Example from a Service
import { api } from '@/lib/api/client';

export async function getDocuments(workspaceId: string): Promise<Document[]> {
  return api.get<Document[]>(`/workspaces/${workspaceId}/documents`);
}
```

### 2.2 Error Handling

All 4xx/5xx responses are thrown as typed errors:
-   `ApiRequestError`: Standard API error with `status`, `code`, and `details` JSON.
-   `AuthError` (401): Triggers auto-logout flow.
-   `NetworkError`: Connection failures.

**Response Type Safety**:
We do not use Zod for runtime validation of *every* API response to max performance, but we strictly type the generic return values `<T>`.

---

## 3. Streaming (Server-Sent Events)

EdgeQuake uses SSE for operations that generate incremental results, such as LLM answers and Graph Traversal.

### 3.1 Streaming Helper

`streamClient` in `client.ts` handles the `EventSource`-like reading of `ReadableStream`.

```typescript
// src/lib/api/edgequake.ts

export async function queryGraphStream(
  req: QueryRequest,
  onChunk: (chunk: string) => void
) {
  await streamClient('/query/stream', {
    method: 'POST',
    body: JSON.stringify(req)
  }, (data) => {
     // Process JSON chunk
     onChunk(data);
  });
}
```

**Key Features**:
-   **Text Decoding**: Automatically decodes UTF-8 chunks.
-   **JSON Parsing**: Handles partial JSON splitting across chunks (using `eventsource-parser` pattern if needed, or line-delimited JSON).
-   **AbortController**: Returns a controller to cancel generation.

---

## 4. WebSockets (Ingestion Progress)

Long-running file ingestion jobs push status updates via WebSocket.

### 4.1 Architecture

The WebSocket connection is a **Singleton** managed by `src/lib/websocket/websocket-client.ts`. It prevents multiple tabs/components from opening redundant connections.

**Protocol**:
-   **Connect**: `/ws?workspaceId={id}`
-   **Subscribe**: Client sends `{"action": "subscribe", "trackIds": ["job-123"]}`
-   **Message**: Server pushes `{"type": "progress", "trackId": "job-123", "progress": 45}`

### 4.2 React Hook

Status is exposed via `useWebSocket` and synced to `useIngestionStore`.

```tsx
// Component Usage
function IngestionStatus() {
  const { wsConnected } = useWebSocket();
  const progress = useIngestionStore(s => s.progress);

  if (!wsConnected) return <Badge variant="destructive">Disconnected</Badge>;

  return <ProgressBar value={progress} />;
}
```

---

## 5. React Query Integration

For standard server state (lists of documents, user profile), we use **TanStack Query (v5)**.

### 5.1 Query Keys

Centralized in `src/lib/api/query-keys.ts` to prevent cache collision.

```typescript
export const queryKeys = {
  documents: {
    list: (wsId: string) => ['documents', 'list', wsId],
    detail: (id: string) => ['documents', 'detail', id],
  },
  graph: {
    all: (wsId: string) => ['graph', wsId],
  }
}
```

### 5.2 Custom Hooks

We wrap `useQuery` in custom hooks for domain logic separation.

```typescript
// src/hooks/use-documents.ts
export function useDocuments(workspaceId: string) {
  return useQuery({
    queryKey: queryKeys.documents.list(workspaceId),
    queryFn: () => api.getDocuments(workspaceId),
    staleTime: 1000 * 60, // 1 minute cache
  });
}
```

---

## 6. Authentication Flow

1.  **Login**: `api.post('/auth/login', creds)` returns `accessToken` + `refreshToken`.
2.  **Storage**: Tokens stored in `localStorage` (for persistence) and memory (for access).
3.  **Interceptor**: `client.ts` injects `Authorization: Bearer <token>` into every request.
4.  **Expiry**: On 401, the client attempts `/auth/refresh`. If that fails, it clears storage and redirects to `/login`.

---

## 7. Best Practices

1.  **Never fetch in components**: Always create update `src/lib/api` methods and wrap in a Hook.
2.  **Optimistic Updates**: Use `queryClient.setQueryData` for immediate UI feedback on Mutations.
3.  **Loading States**: All Hooks return `isPending` state; pass this to UI Skeletons.
4.  **Error Boundaries**: Route-level `error.tsx` catches bubble-up API errors.
