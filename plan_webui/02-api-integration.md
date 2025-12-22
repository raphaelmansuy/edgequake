# EdgeQuake WebUI - API Integration

> EdgeQuake API mapping, type definitions, and error handling specification.

**Parent Document**: [00-master-plan.md](./00-master-plan.md)

---

## Table of Contents

1. [API Overview](#api-overview)
2. [Endpoint Mapping](#endpoint-mapping)
3. [Type Definitions](#type-definitions)
4. [Authentication](#authentication)
5. [Error Handling](#error-handling)
6. [Streaming](#streaming)

---

## API Overview

### Base URL Configuration

```typescript
// lib/constants.ts
export const API_CONFIG = {
  baseUrl: process.env.NEXT_PUBLIC_EDGEQUAKE_API_URL || "http://localhost:8080",
  apiVersion: "v1",
  timeout: 30000,
} as const;

export const getApiUrl = (path: string) =>
  `${API_CONFIG.baseUrl}/api/${API_CONFIG.apiVersion}${path}`;
```

### Request Headers

| Header          | Description                 | Source                         |
| --------------- | --------------------------- | ------------------------------ |
| `Authorization` | Bearer JWT token            | `localStorage.EDGEQUAKE_TOKEN` |
| `X-Tenant-ID`   | Current tenant UUID         | `localStorage.SELECTED_TENANT` |
| `X-KB-ID`       | Current knowledge base UUID | `localStorage.SELECTED_KB`     |
| `Content-Type`  | Request content type        | `application/json`             |

---

## Endpoint Mapping

### Health & Status

| Method | Endpoint  | Description     | WebUI Usage                       |
| ------ | --------- | --------------- | --------------------------------- |
| GET    | `/health` | Health check    | StatusIndicator, periodic polling |
| GET    | `/ready`  | Readiness check | Initial app load                  |
| GET    | `/live`   | Liveness check  | Connection monitoring             |

### Authentication

| Method | Endpoint               | Description       | WebUI Usage          |
| ------ | ---------------------- | ----------------- | -------------------- |
| POST   | `/api/v1/auth/login`   | User login        | LoginPage            |
| POST   | `/api/v1/auth/refresh` | Refresh JWT token | Auto-refresh         |
| POST   | `/api/v1/auth/logout`  | User logout       | Header logout button |
| GET    | `/api/v1/auth/me`      | Get current user  | User display         |

### Documents

| Method | Endpoint                   | Description                | WebUI Usage          |
| ------ | -------------------------- | -------------------------- | -------------------- |
| GET    | `/api/v1/documents`        | List documents (paginated) | DocumentTable        |
| POST   | `/api/v1/documents`        | Upload document (JSON)     | Upload text          |
| POST   | `/api/v1/documents/upload` | Upload file (multipart)    | UploadDialog         |
| GET    | `/api/v1/documents/{id}`   | Get document details       | Document detail view |
| DELETE | `/api/v1/documents/{id}`   | Delete document            | DeleteDialog         |

### Query

| Method | Endpoint               | Description       | WebUI Usage                 |
| ------ | ---------------------- | ----------------- | --------------------------- |
| POST   | `/api/v1/query`        | Execute RAG query | QueryInterface (non-stream) |
| POST   | `/api/v1/query/stream` | Stream RAG query  | QueryInterface (stream)     |

### Graph

| Method | Endpoint                      | Description             | WebUI Usage    |
| ------ | ----------------------------- | ----------------------- | -------------- |
| GET    | `/api/v1/graph`               | Get graph (with params) | GraphViewer    |
| GET    | `/api/v1/graph/nodes/{id}`    | Get node details        | NodeProperties |
| GET    | `/api/v1/graph/labels/search` | Search labels           | GraphSearch    |

### Entities

| Method | Endpoint                        | Description    | WebUI Usage    |
| ------ | ------------------------------- | -------------- | -------------- |
| GET    | `/api/v1/graph/entities/{name}` | Get entity     | NodeProperties |
| POST   | `/api/v1/graph/entities`        | Create entity  | (future)       |
| PUT    | `/api/v1/graph/entities/{name}` | Update entity  | EditDialog     |
| DELETE | `/api/v1/graph/entities/{name}` | Delete entity  | DeleteDialog   |
| POST   | `/api/v1/graph/entities/merge`  | Merge entities | MergeDialog    |

### Relationships

| Method | Endpoint                           | Description         | WebUI Usage    |
| ------ | ---------------------------------- | ------------------- | -------------- |
| GET    | `/api/v1/graph/relationships/{id}` | Get relationship    | EdgeProperties |
| POST   | `/api/v1/graph/relationships`      | Create relationship | (future)       |
| PUT    | `/api/v1/graph/relationships/{id}` | Update relationship | EditDialog     |
| DELETE | `/api/v1/graph/relationships/{id}` | Delete relationship | DeleteDialog   |

### Tasks

| Method | Endpoint                          | Description     | WebUI Usage    |
| ------ | --------------------------------- | --------------- | -------------- |
| GET    | `/api/v1/tasks`                   | List tasks      | PipelineStatus |
| GET    | `/api/v1/tasks/{track_id}`        | Get task status | PipelineStatus |
| POST   | `/api/v1/tasks/{track_id}/cancel` | Cancel task     | PipelineStatus |

---

## Type Definitions

### Graph Types

```typescript
// types/graph.ts

export interface GraphNode {
  id: string;
  label: string;
  nodeType: string;
  description: string;
  degree: number;
  properties: Record<string, unknown>;
}

export interface GraphEdge {
  source: string;
  target: string;
  edgeType: string;
  weight: number;
  properties: Record<string, unknown>;
}

export interface KnowledgeGraph {
  nodes: GraphNode[];
  edges: GraphEdge[];
  isTruncated: boolean;
  totalNodes: number;
  totalEdges: number;
}

export interface GraphQueryParams {
  startNode?: string;
  depth?: number;
  maxNodes?: number;
}

export interface LabelSearchParams {
  query: string;
  limit?: number;
}
```

### Document Types

```typescript
// types/document.ts

export type DocumentStatus = "pending" | "processing" | "processed" | "failed";

export interface Document {
  id: string;
  title?: string;
  filePath?: string;
  status: DocumentStatus;
  chunkCount: number;
  createdAt: string;
  updatedAt: string;
  errorMessage?: string;
  metadata?: Record<string, unknown>;
}

export interface DocumentListResponse {
  documents: Document[];
  total: number;
  page: number;
  pageSize: number;
  totalPages: number;
}

export interface UploadDocumentRequest {
  content: string;
  title?: string;
  metadata?: Record<string, unknown>;
}

export interface UploadDocumentResponse {
  documentId: string;
  status: string;
  chunkCount: number;
  entityCount: number;
  relationshipCount: number;
}
```

### Query Types

```typescript
// types/query.ts

export type QueryMode =
  | "naive"
  | "local"
  | "global"
  | "hybrid"
  | "mix"
  | "bypass";

export interface ConversationMessage {
  role: "user" | "assistant" | "system";
  content: string;
  thinkingContent?: string;
  displayContent?: string;
  thinkingTime?: number | null;
}

export interface QueryRequest {
  query: string;
  mode?: QueryMode;
  contextOnly?: boolean;
  maxResults?: number;
  conversationHistory?: ConversationMessage[];
  enableRerank?: boolean;
  rerankModel?: string;
  rerankTopK?: number;
}

export interface SourceReference {
  sourceType: "chunk" | "entity" | "relationship";
  id: string;
  score: number;
  rerankScore?: number;
  snippet?: string;
}

export interface QueryStats {
  embeddingTimeMs: number;
  retrievalTimeMs: number;
  generationTimeMs: number;
  totalTimeMs: number;
  sourcesRetrieved: number;
  rerankTimeMs?: number;
}

export interface QueryResponse {
  answer: string;
  mode: string;
  sources: SourceReference[];
  stats: QueryStats;
  conversationId?: string;
  reranked: boolean;
}
```

### Auth Types

```typescript
// types/auth.ts

export interface LoginRequest {
  username: string;
  password: string;
}

export interface LoginResponse {
  accessToken: string;
  tokenType: string;
  expiresIn: number;
}

export interface User {
  id: string;
  username: string;
  email?: string;
  roles: string[];
}
```

### Tenant Types

```typescript
// types/tenant.ts

export interface Tenant {
  tenantId: string;
  name: string;
  description?: string;
  createdAt: string;
  updatedAt: string;
}

export interface KnowledgeBase {
  kbId: string;
  tenantId: string;
  name: string;
  description?: string;
  documentCount: number;
  entityCount: number;
  relationCount: number;
  createdAt: string;
  updatedAt: string;
}
```

---

## Authentication

### JWT Token Flow

```typescript
// lib/api/auth.ts

export async function login(credentials: LoginRequest): Promise<LoginResponse> {
  const response = await fetch(`${API_CONFIG.baseUrl}/api/v1/auth/login`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(credentials),
  });

  if (!response.ok) {
    throw new AuthError("Invalid credentials", response.status);
  }

  const data = await response.json();

  // Store token
  localStorage.setItem("EDGEQUAKE_TOKEN", data.accessToken);

  // Set expiry timer for refresh
  scheduleTokenRefresh(data.expiresIn);

  return data;
}

export async function refreshToken(): Promise<void> {
  const token = localStorage.getItem("EDGEQUAKE_TOKEN");
  if (!token) throw new AuthError("No token to refresh");

  const response = await fetch(`${API_CONFIG.baseUrl}/api/v1/auth/refresh`, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      Authorization: `Bearer ${token}`,
    },
  });

  if (!response.ok) {
    // Token expired, redirect to login
    localStorage.removeItem("EDGEQUAKE_TOKEN");
    window.location.href = "/login";
    return;
  }

  const data = await response.json();
  localStorage.setItem("EDGEQUAKE_TOKEN", data.accessToken);
  scheduleTokenRefresh(data.expiresIn);
}

function scheduleTokenRefresh(expiresIn: number) {
  // Refresh 1 minute before expiry
  const refreshTime = (expiresIn - 60) * 1000;
  setTimeout(() => refreshToken(), refreshTime);
}
```

### Auth Store

```typescript
// stores/use-auth-store.ts

import { create } from "zustand";
import { persist } from "zustand/middleware";

interface AuthState {
  isAuthenticated: boolean;
  user: User | null;
  login: (credentials: LoginRequest) => Promise<void>;
  logout: () => void;
  checkAuth: () => Promise<boolean>;
}

export const useAuthStore = create<AuthState>()(
  persist(
    (set, get) => ({
      isAuthenticated: false,
      user: null,

      login: async (credentials) => {
        const response = await login(credentials);
        const user = await fetchCurrentUser();
        set({ isAuthenticated: true, user });
      },

      logout: () => {
        localStorage.removeItem("EDGEQUAKE_TOKEN");
        localStorage.removeItem("SELECTED_TENANT");
        localStorage.removeItem("SELECTED_KB");
        set({ isAuthenticated: false, user: null });
        window.location.href = "/login";
      },

      checkAuth: async () => {
        const token = localStorage.getItem("EDGEQUAKE_TOKEN");
        if (!token) {
          set({ isAuthenticated: false, user: null });
          return false;
        }
        try {
          const user = await fetchCurrentUser();
          set({ isAuthenticated: true, user });
          return true;
        } catch {
          set({ isAuthenticated: false, user: null });
          return false;
        }
      },
    }),
    {
      name: "edgequake-auth",
      partialize: (state) => ({ isAuthenticated: state.isAuthenticated }),
    }
  )
);
```

---

## Error Handling

### Error Classes

```typescript
// lib/api/errors.ts

export class ApiError extends Error {
  constructor(
    message: string,
    public statusCode: number,
    public code?: string,
    public details?: unknown
  ) {
    super(message);
    this.name = "ApiError";
  }
}

export class AuthError extends ApiError {
  constructor(message: string, statusCode: number = 401) {
    super(message, statusCode, "AUTH_ERROR");
    this.name = "AuthError";
  }
}

export class ValidationError extends ApiError {
  constructor(message: string, details?: unknown) {
    super(message, 400, "VALIDATION_ERROR", details);
    this.name = "ValidationError";
  }
}

export class NetworkError extends ApiError {
  constructor(message: string = "Network connection failed") {
    super(message, 0, "NETWORK_ERROR");
    this.name = "NetworkError";
  }
}
```

### Error Handler

```typescript
// lib/api/client.ts

async function handleResponse<T>(response: Response): Promise<T> {
  if (!response.ok) {
    let errorData: { detail?: string; error_code?: string } | undefined;

    try {
      errorData = await response.json();
    } catch {
      // Response body not JSON
    }

    const message = errorData?.detail || `HTTP Error ${response.status}`;
    const code = errorData?.error_code;

    switch (response.status) {
      case 400:
        throw new ValidationError(message, errorData);
      case 401:
        throw new AuthError(message);
      case 403:
        throw new ApiError("Forbidden", 403, "FORBIDDEN");
      case 404:
        throw new ApiError("Not Found", 404, "NOT_FOUND");
      case 429:
        throw new ApiError("Too Many Requests", 429, "RATE_LIMITED");
      case 500:
      case 502:
      case 503:
        throw new ApiError("Server Error", response.status, "SERVER_ERROR");
      default:
        throw new ApiError(message, response.status, code);
    }
  }

  return response.json();
}
```

### Error Boundary Component

```tsx
// components/error-boundary.tsx

"use client";

import { useEffect } from "react";
import { Button } from "@/components/ui/button";

export function ErrorFallback({
  error,
  reset,
}: {
  error: Error;
  reset: () => void;
}) {
  useEffect(() => {
    console.error("Error:", error);
  }, [error]);

  return (
    <div className="flex flex-col items-center justify-center min-h-[400px] gap-4">
      <h2 className="text-xl font-semibold">Something went wrong</h2>
      <p className="text-muted-foreground">{error.message}</p>
      <Button onClick={reset}>Try again</Button>
    </div>
  );
}
```

---

## Streaming

### Stream Query Implementation

```typescript
// lib/api/edgequake.ts

export interface StreamCallbacks {
  onChunk: (chunk: string) => void;
  onThinking?: (content: string) => void;
  onError?: (error: string) => void;
  onComplete?: () => void;
}

export async function streamQuery(
  request: QueryRequest,
  callbacks: StreamCallbacks
): Promise<void> {
  const { onChunk, onThinking, onError, onComplete } = callbacks;

  const headers = getAuthHeaders();
  headers.set("Accept", "application/x-ndjson");

  const response = await fetch(`${API_CONFIG.baseUrl}/api/v1/query/stream`, {
    method: "POST",
    headers,
    body: JSON.stringify({
      ...request,
      stream: true,
    }),
  });

  if (!response.ok) {
    const errorText = await response.text();
    throw new ApiError(`Stream failed: ${errorText}`, response.status);
  }

  if (!response.body) {
    throw new ApiError("Response body is null", 500);
  }

  const reader = response.body.getReader();
  const decoder = new TextDecoder();
  let buffer = "";

  try {
    while (true) {
      const { done, value } = await reader.read();

      if (done) {
        // Process remaining buffer
        if (buffer.trim()) {
          processLine(buffer, callbacks);
        }
        break;
      }

      buffer += decoder.decode(value, { stream: true });

      // Process complete lines
      const lines = buffer.split("\n");
      buffer = lines.pop() || "";

      for (const line of lines) {
        if (line.trim()) {
          processLine(line, callbacks);
        }
      }
    }
  } finally {
    reader.releaseLock();
    onComplete?.();
  }
}

function processLine(line: string, callbacks: StreamCallbacks) {
  try {
    const data = JSON.parse(line);

    if (data.response) {
      callbacks.onChunk(data.response);
    }

    if (data.thinking && callbacks.onThinking) {
      callbacks.onThinking(data.thinking);
    }

    if (data.error && callbacks.onError) {
      callbacks.onError(data.error);
    }
  } catch (e) {
    console.warn("Failed to parse stream line:", line);
  }
}
```

### React Hook for Streaming

```typescript
// hooks/use-stream-query.ts

import { useState, useCallback, useRef } from "react";
import {
  streamQuery,
  QueryRequest,
  StreamCallbacks,
} from "@/lib/api/edgequake";

interface UseStreamQueryResult {
  response: string;
  isStreaming: boolean;
  isThinking: boolean;
  thinkingContent: string;
  error: string | null;
  execute: (request: QueryRequest) => Promise<void>;
  cancel: () => void;
}

export function useStreamQuery(): UseStreamQueryResult {
  const [response, setResponse] = useState("");
  const [isStreaming, setIsStreaming] = useState(false);
  const [isThinking, setIsThinking] = useState(false);
  const [thinkingContent, setThinkingContent] = useState("");
  const [error, setError] = useState<string | null>(null);

  const abortControllerRef = useRef<AbortController | null>(null);

  const execute = useCallback(async (request: QueryRequest) => {
    setResponse("");
    setThinkingContent("");
    setError(null);
    setIsStreaming(true);
    setIsThinking(false);

    abortControllerRef.current = new AbortController();

    const callbacks: StreamCallbacks = {
      onChunk: (chunk) => {
        setResponse((prev) => prev + chunk);
        setIsThinking(false);
      },
      onThinking: (content) => {
        setThinkingContent((prev) => prev + content);
        setIsThinking(true);
      },
      onError: (err) => {
        setError(err);
        setIsStreaming(false);
      },
      onComplete: () => {
        setIsStreaming(false);
        setIsThinking(false);
      },
    };

    try {
      await streamQuery(request, callbacks);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Unknown error");
      setIsStreaming(false);
    }
  }, []);

  const cancel = useCallback(() => {
    abortControllerRef.current?.abort();
    setIsStreaming(false);
    setIsThinking(false);
  }, []);

  return {
    response,
    isStreaming,
    isThinking,
    thinkingContent,
    error,
    execute,
    cancel,
  };
}
```

---

## Related Documents

- **Previous**: [01-architecture.md](./01-architecture.md) - System architecture
- **Next**: [03-component-mapping.md](./03-component-mapping.md) - Component migration guide
