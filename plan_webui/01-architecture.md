# EdgeQuake WebUI - Architecture Overview

> System architecture, component hierarchy, and data flow specification.

**Parent Document**: [00-master-plan.md](./00-master-plan.md)

---

## Table of Contents

1. [High-Level Architecture](#high-level-architecture)
2. [Directory Structure](#directory-structure)
3. [Component Hierarchy](#component-hierarchy)
4. [Data Flow](#data-flow)
5. [Key Patterns](#key-patterns)

---

## High-Level Architecture

```
┌─────────────────────────────────────────────────────────────────────────┐
│                           EdgeQuake WebUI                                │
│                          (Next.js 15 App)                                │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  ┌─────────────────────────────────────────────────────────────────┐    │
│  │                     Presentation Layer                           │    │
│  │  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────────────┐    │    │
│  │  │  Graph   │ │ Documents│ │  Query   │ │   API Explorer   │    │    │
│  │  │  Viewer  │ │ Manager  │ │Interface │ │                  │    │    │
│  │  └──────────┘ └──────────┘ └──────────┘ └──────────────────┘    │    │
│  └─────────────────────────────────────────────────────────────────┘    │
│                                    │                                     │
│  ┌─────────────────────────────────────────────────────────────────┐    │
│  │                     Application Layer                            │    │
│  │  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────────────┐    │    │
│  │  │  Stores  │ │  Hooks   │ │ Services │ │    Providers     │    │    │
│  │  │ (Zustand)│ │ (Custom) │ │  (API)   │ │ (Theme/Auth/i18n)│    │    │
│  │  └──────────┘ └──────────┘ └──────────┘ └──────────────────┘    │    │
│  └─────────────────────────────────────────────────────────────────┘    │
│                                    │                                     │
│  ┌─────────────────────────────────────────────────────────────────┐    │
│  │                      Data Layer                                  │    │
│  │  ┌──────────────────┐  ┌────────────────────────────────────┐   │    │
│  │  │   API Client     │  │         Server Actions             │   │    │
│  │  │   (Fetch-based)  │  │    (Next.js Server Functions)      │   │    │
│  │  └──────────────────┘  └────────────────────────────────────┘   │    │
│  └─────────────────────────────────────────────────────────────────┘    │
│                                                                          │
└─────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                        EdgeQuake Rust API                                │
│                      (REST API + SSE Streaming)                          │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐    │
│  │   /health   │  │  /api/v1/   │  │   /graph    │  │   /query    │    │
│  │   /ready    │  │  documents  │  │   /nodes    │  │   /stream   │    │
│  └─────────────┘  └─────────────┘  └─────────────┘  └─────────────┘    │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## Directory Structure

```
edgequake_webui/
├── src/
│   ├── app/                      # Next.js App Router
│   │   ├── (auth)/               # Auth layout group
│   │   │   ├── login/
│   │   │   │   └── page.tsx
│   │   │   └── layout.tsx
│   │   ├── (dashboard)/          # Main app layout group
│   │   │   ├── graph/
│   │   │   │   └── page.tsx
│   │   │   ├── documents/
│   │   │   │   └── page.tsx
│   │   │   ├── query/
│   │   │   │   └── page.tsx
│   │   │   ├── api-explorer/
│   │   │   │   └── page.tsx
│   │   │   └── layout.tsx
│   │   ├── api/                  # API Route Handlers (if needed)
│   │   │   └── proxy/
│   │   │       └── [...path]/
│   │   │           └── route.ts
│   │   ├── layout.tsx            # Root layout
│   │   ├── page.tsx              # Home redirect
│   │   └── globals.css           # Global styles
│   │
│   ├── components/               # Shared components
│   │   ├── ui/                   # shadcn/ui components
│   │   │   ├── button.tsx
│   │   │   ├── card.tsx
│   │   │   ├── dialog.tsx
│   │   │   ├── input.tsx
│   │   │   ├── select.tsx
│   │   │   ├── table.tsx
│   │   │   ├── tabs.tsx
│   │   │   └── ...
│   │   ├── graph/                # Graph visualization components
│   │   │   ├── graph-container.tsx
│   │   │   ├── graph-controls.tsx
│   │   │   ├── graph-search.tsx
│   │   │   ├── node-properties.tsx
│   │   │   ├── layout-controls.tsx
│   │   │   └── legend.tsx
│   │   ├── documents/            # Document management components
│   │   │   ├── document-table.tsx
│   │   │   ├── upload-dialog.tsx
│   │   │   ├── delete-dialog.tsx
│   │   │   └── pipeline-status.tsx
│   │   ├── query/                # Query interface components
│   │   │   ├── chat-message.tsx
│   │   │   ├── query-input.tsx
│   │   │   ├── query-settings.tsx
│   │   │   └── markdown-renderer.tsx
│   │   ├── layout/               # Layout components
│   │   │   ├── header.tsx
│   │   │   ├── sidebar.tsx
│   │   │   ├── footer.tsx
│   │   │   └── nav-tabs.tsx
│   │   └── shared/               # Common shared components
│   │       ├── theme-toggle.tsx
│   │       ├── language-toggle.tsx
│   │       ├── tenant-selector.tsx
│   │       ├── status-indicator.tsx
│   │       └── loading-spinner.tsx
│   │
│   ├── lib/                      # Core libraries
│   │   ├── api/                  # API client
│   │   │   ├── client.ts         # Base fetch client
│   │   │   ├── edgequake.ts      # EdgeQuake API functions
│   │   │   ├── types.ts          # API types
│   │   │   └── errors.ts         # Error classes
│   │   ├── constants.ts          # App constants
│   │   ├── utils.ts              # Utility functions
│   │   └── cn.ts                 # Class name utility
│   │
│   ├── stores/                   # Zustand stores
│   │   ├── use-settings-store.ts
│   │   ├── use-tenant-store.ts
│   │   ├── use-graph-store.ts
│   │   ├── use-auth-store.ts
│   │   └── use-backend-store.ts
│   │
│   ├── hooks/                    # Custom React hooks
│   │   ├── use-debounce.ts
│   │   ├── use-graph.ts
│   │   ├── use-theme.ts
│   │   └── use-media-query.ts
│   │
│   ├── providers/                # Context providers
│   │   ├── theme-provider.tsx
│   │   ├── query-provider.tsx
│   │   └── tenant-provider.tsx
│   │
│   └── types/                    # TypeScript types
│       ├── graph.ts
│       ├── document.ts
│       ├── query.ts
│       └── api.ts
│
├── public/                       # Static assets
│   ├── favicon.ico
│   └── logo.svg
│
├── tests/                        # Test files
│   ├── unit/
│   ├── integration/
│   └── e2e/
│
├── .env.example                  # Environment template
├── .env.local                    # Local environment (gitignored)
├── next.config.ts                # Next.js configuration
├── tailwind.config.ts            # Tailwind CSS configuration
├── tsconfig.json                 # TypeScript configuration
├── package.json                  # Dependencies
├── bun.lock                      # Bun lockfile
└── README.md                     # Project documentation
```

---

## Component Hierarchy

### Root Layout Tree

```
<html>
├── <ThemeProvider>
│   ├── <QueryClientProvider>
│   │   ├── <TenantProvider>
│   │   │   ├── <body>
│   │   │   │   ├── <Toaster />              # Toast notifications
│   │   │   │   └── {children}               # Route content
│   │   │   │       ├── (auth)/layout
│   │   │   │       │   └── LoginPage
│   │   │   │       └── (dashboard)/layout
│   │   │   │           ├── Header
│   │   │   │           │   ├── Logo
│   │   │   │           │   ├── TenantSelector
│   │   │   │           │   ├── NavTabs
│   │   │   │           │   ├── StatusIndicator
│   │   │   │           │   ├── ThemeToggle
│   │   │   │           │   └── LanguageToggle
│   │   │   │           └── {children}
│   │   │   │               ├── GraphPage
│   │   │   │               ├── DocumentsPage
│   │   │   │               ├── QueryPage
│   │   │   │               └── ApiExplorerPage
```

### Feature Components

#### Graph Viewer

```
<GraphPage>
├── <GraphContainer>
│   ├── <SigmaContainer>
│   │   ├── <GraphEvents />
│   │   └── <FocusOnNode />
│   ├── <GraphSearch />
│   ├── <GraphLabels />
│   ├── <GraphControls>
│   │   ├── <LayoutsControl />
│   │   ├── <ZoomControl />
│   │   └── <FullScreenControl />
│   ├── <NodeProperties />
│   └── <Legend />
```

#### Document Manager

```
<DocumentsPage>
├── <DocumentTable>
│   ├── <TableHeader />
│   ├── <TableBody>
│   │   └── <DocumentRow />
│   └── <TablePagination />
├── <UploadDialog />
├── <DeleteDialog />
├── <PipelineStatus />
└── <ActionBar>
    ├── <UploadButton />
    ├── <ScanButton />
    └── <RefreshButton />
```

#### Query Interface

```
<QueryPage>
├── <QuerySettings />
├── <ChatContainer>
│   ├── <ChatMessage type="user" />
│   ├── <ChatMessage type="assistant">
│   │   ├── <ThinkingIndicator />
│   │   └── <MarkdownRenderer />
│   └── <ChatMessage ... />
├── <QueryInput>
│   ├── <TextArea />
│   └── <SendButton />
└── <QueryHistory />
```

---

## Data Flow

### State Management Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                        State Layer                               │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  ┌─────────────────┐     ┌─────────────────┐                    │
│  │  useAuthStore   │     │ useSettingsStore│                    │
│  │  ─────────────  │     │  ─────────────  │                    │
│  │  • isAuth       │     │  • theme        │                    │
│  │  • token        │     │  • language     │                    │
│  │  • user         │     │  • graphSettings│                    │
│  └─────────────────┘     └─────────────────┘                    │
│                                                                  │
│  ┌─────────────────┐     ┌─────────────────┐                    │
│  │ useTenantStore  │     │  useGraphStore  │                    │
│  │  ─────────────  │     │  ─────────────  │                    │
│  │  • tenant       │     │  • graph        │                    │
│  │  • knowledgeBase│     │  • selectedNode │                    │
│  │  • tenantList   │     │  • focusedNode  │                    │
│  └─────────────────┘     └─────────────────┘                    │
│                                                                  │
│  ┌─────────────────┐                                            │
│  │ useBackendStore │                                            │
│  │  ─────────────  │                                            │
│  │  • health       │                                            │
│  │  • pipelineBusy │                                            │
│  │  • status       │                                            │
│  └─────────────────┘                                            │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                        API Layer                                 │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │                     API Client                            │   │
│  │  • Fetch-based with interceptors                          │   │
│  │  • Automatic token injection                              │   │
│  │  • Tenant/KB header injection                             │   │
│  │  • Error handling & retry                                 │   │
│  └──────────────────────────────────────────────────────────┘   │
│                                                                  │
│  ┌────────────────┐  ┌────────────────┐  ┌────────────────┐    │
│  │ Graph API      │  │ Document API   │  │ Query API      │    │
│  │ • getGraph()   │  │ • list()       │  │ • query()      │    │
│  │ • getLabels()  │  │ • upload()     │  │ • stream()     │    │
│  │ • searchNodes()│  │ • delete()     │  │                │    │
│  └────────────────┘  └────────────────┘  └────────────────┘    │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                    EdgeQuake Rust API                            │
└─────────────────────────────────────────────────────────────────┘
```

### Request Flow

```
User Action
    │
    ▼
Component (onClick, onSubmit, etc.)
    │
    ▼
Store Action / Hook
    │
    ├──────────────────────────────────┐
    │ Read from Store                  │ Write to Store
    │                                  │
    ▼                                  ▼
┌─────────────────┐           ┌─────────────────┐
│  Optimistic     │           │  API Client     │
│  Update         │           │  Request        │
└─────────────────┘           └─────────────────┘
                                      │
                                      ▼
                              ┌─────────────────┐
                              │  EdgeQuake API  │
                              └─────────────────┘
                                      │
                                      ▼
                              ┌─────────────────┐
                              │  Response       │
                              └─────────────────┘
                                      │
                    ┌─────────────────┼─────────────────┐
                    │ Success         │                 │ Error
                    ▼                 ▼                 ▼
          ┌─────────────────┐ ┌─────────────────┐ ┌─────────────────┐
          │ Confirm Update  │ │ Store Update    │ │ Rollback/Toast  │
          │ (if optimistic) │ │ (data)          │ │ Error Display   │
          └─────────────────┘ └─────────────────┘ └─────────────────┘
```

---

## Key Patterns

### 1. Server Components + Client Components Split

```tsx
// Server Component (default)
// app/(dashboard)/documents/page.tsx
import { DocumentTable } from '@/components/documents/document-table';

export default async function DocumentsPage() {
  // Can do server-side data fetching here
  return (
    <div className="container mx-auto py-6">
      <h1 className="text-2xl font-bold mb-4">Documents</h1>
      <DocumentTable /> {/* Client Component */}
    </div>
  );
}

// Client Component
// components/documents/document-table.tsx
'use client';

import { useState, useEffect } from 'react';
import { useQuery } from '@tanstack/react-query';

export function DocumentTable() {
  // Client-side state and data fetching
  const { data, isLoading } = useQuery({
    queryKey: ['documents'],
    queryFn: () => fetchDocuments(),
  });

  return (/* ... */);
}
```

### 2. Zustand Store Pattern with Persistence

```tsx
// stores/use-settings-store.ts
import { create } from "zustand";
import { persist } from "zustand/middleware";

interface SettingsState {
  theme: "light" | "dark" | "system";
  language: "en" | "zh" | "fr";
  graphMaxNodes: number;
  setTheme: (theme: SettingsState["theme"]) => void;
  setLanguage: (lang: SettingsState["language"]) => void;
}

export const useSettingsStore = create<SettingsState>()(
  persist(
    (set) => ({
      theme: "system",
      language: "en",
      graphMaxNodes: 1000,
      setTheme: (theme) => set({ theme }),
      setLanguage: (language) => set({ language }),
    }),
    {
      name: "edgequake-settings",
    }
  )
);
```

### 3. API Client with Interceptors

```tsx
// lib/api/client.ts
const BASE_URL = process.env.NEXT_PUBLIC_API_URL || "http://localhost:8080";

async function apiClient<T>(
  endpoint: string,
  options: RequestInit = {}
): Promise<T> {
  const token = localStorage.getItem("EDGEQUAKE_TOKEN");
  const tenant = localStorage.getItem("SELECTED_TENANT");
  const kb = localStorage.getItem("SELECTED_KB");

  const headers = new Headers(options.headers);
  headers.set("Content-Type", "application/json");

  if (token) headers.set("Authorization", `Bearer ${token}`);
  if (tenant) headers.set("X-Tenant-ID", JSON.parse(tenant).tenant_id);
  if (kb) headers.set("X-KB-ID", JSON.parse(kb).kb_id);

  const response = await fetch(`${BASE_URL}${endpoint}`, {
    ...options,
    headers,
  });

  if (!response.ok) {
    if (response.status === 401) {
      // Handle unauthorized - redirect to login
      window.location.href = "/login";
    }
    throw new Error(`API Error: ${response.status}`);
  }

  return response.json();
}
```

### 4. Streaming Response Handler

```tsx
// lib/api/edgequake.ts
export async function streamQuery(
  request: QueryRequest,
  onChunk: (chunk: string) => void,
  onError?: (error: string) => void
): Promise<void> {
  const response = await fetch(`${BASE_URL}/api/v1/query/stream`, {
    method: "POST",
    headers: getHeaders(),
    body: JSON.stringify(request),
  });

  if (!response.ok) {
    throw new Error(`Stream error: ${response.status}`);
  }

  const reader = response.body?.getReader();
  const decoder = new TextDecoder();

  while (reader) {
    const { done, value } = await reader.read();
    if (done) break;

    const chunk = decoder.decode(value, { stream: true });
    const lines = chunk.split("\n").filter(Boolean);

    for (const line of lines) {
      try {
        const data = JSON.parse(line);
        if (data.response) onChunk(data.response);
        if (data.error && onError) onError(data.error);
      } catch (e) {
        // Partial JSON, continue
      }
    }
  }
}
```

---

## Related Documents

- **Next**: [02-api-integration.md](./02-api-integration.md) - Detailed API mapping
- **Previous**: [00-master-plan.md](./00-master-plan.md) - Master plan overview
