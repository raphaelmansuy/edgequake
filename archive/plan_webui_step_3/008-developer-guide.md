# Developer Quick Start Guide

> **Document Version:** 1.0  
> **Date:** 2024-12-23  
> **Purpose:** Onboarding guide for developers contributing to EdgeQuake WebUI

---

## Table of Contents

1. [Prerequisites](#prerequisites)
2. [Getting Started](#getting-started)
3. [Project Structure](#project-structure)
4. [Key Technologies](#key-technologies)
5. [Development Workflow](#development-workflow)
6. [Common Tasks](#common-tasks)
7. [Architecture Overview](#architecture-overview)
8. [Contributing Guidelines](#contributing-guidelines)
9. [Troubleshooting](#troubleshooting)

---

## Prerequisites

### Required Software

| Tool        | Version | Purpose                   |
| ----------- | ------- | ------------------------- |
| **Node.js** | 20+     | JavaScript runtime        |
| **Bun**     | 1.1+    | Package manager & runtime |
| **Git**     | 2.40+   | Version control           |

### Recommended IDE Setup

**VS Code Extensions:**

- ESLint
- Prettier
- Tailwind CSS IntelliSense
- TypeScript and JavaScript Language Features
- Pretty TypeScript Errors
- GitLens

### Environment Setup

```bash
# Install Bun (if not installed)
curl -fsSL https://bun.sh/install | bash

# Verify installations
node --version  # Should be 20+
bun --version   # Should be 1.1+
```

---

## Getting Started

### 1. Clone and Install

```bash
# Clone the repository
git clone https://github.com/your-org/edgequake.git
cd edgequake/edgequake_webui

# Install dependencies
bun install
```

### 2. Configure Environment

```bash
# Copy environment template
cp .env.example .env.local

# Edit .env.local with your settings
# NEXT_PUBLIC_API_URL=http://localhost:8000/api
```

### 3. Start Development Server

```bash
# Start the dev server
bun run dev

# Open http://localhost:3000
```

### 4. Verify Setup

Visit these pages to verify everything works:

- http://localhost:3000 - Home page
- http://localhost:3000/graph - Graph viewer
- http://localhost:3000/documents - Document manager
- http://localhost:3000/query - Query interface

---

## Project Structure

```
edgequake_webui/
├── public/                    # Static assets
│   ├── favicon.ico
│   └── locales/               # i18n translation files
│       ├── en/
│       ├── zh/
│       └── fr/
├── src/
│   ├── app/                   # Next.js App Router pages
│   │   ├── layout.tsx         # Root layout
│   │   ├── page.tsx           # Home page
│   │   ├── graph/             # Graph viewer page
│   │   ├── documents/         # Document manager page
│   │   ├── query/             # Query interface page
│   │   └── settings/          # Settings page
│   ├── components/            # React components
│   │   ├── ui/                # shadcn/ui components
│   │   ├── layout/            # Layout components
│   │   ├── graph/             # Graph-related components
│   │   ├── documents/         # Document components
│   │   ├── query/             # Query components
│   │   └── chat/              # Chat/response components
│   ├── hooks/                 # Custom React hooks
│   ├── lib/                   # Utilities and helpers
│   │   ├── api/               # API client functions
│   │   ├── utils.ts           # General utilities
│   │   └── i18n.ts            # Internationalization config
│   ├── stores/                # Zustand stores
│   └── types/                 # TypeScript type definitions
├── e2e/                       # Playwright E2E tests
├── package.json
├── next.config.ts             # Next.js configuration
├── tailwind.config.ts         # Tailwind CSS configuration
└── tsconfig.json              # TypeScript configuration
```

---

## Key Technologies

### Framework & Runtime

| Technology     | Version | Purpose                         |
| -------------- | ------- | ------------------------------- |
| **Next.js**    | 16.1.0  | React framework with App Router |
| **React**      | 19.1.0  | UI library                      |
| **TypeScript** | 5.8.3   | Type safety                     |
| **Bun**        | 1.1+    | Package manager & runtime       |

### State Management

| Technology      | Purpose                           |
| --------------- | --------------------------------- |
| **Zustand**     | Global state (settings, UI state) |
| **React Query** | Server state (API data caching)   |

### UI & Styling

| Technology       | Purpose                       |
| ---------------- | ----------------------------- |
| **Tailwind CSS** | Utility-first CSS             |
| **shadcn/ui**    | Radix-based component library |
| **Lucide**       | Icon library                  |
| **Sigma.js**     | Graph visualization           |

### Testing

| Technology     | Purpose                    |
| -------------- | -------------------------- |
| **Playwright** | E2E testing                |
| **Vitest**     | Unit testing (to be added) |

---

## Development Workflow

### Branch Strategy

```
main          ← Production-ready code
  └── develop ← Integration branch
        └── feature/XXX   ← Feature branches
        └── fix/XXX       ← Bug fix branches
        └── refactor/XXX  ← Refactoring branches
```

### Standard Workflow

```bash
# 1. Create feature branch
git checkout develop
git pull origin develop
git checkout -b feature/node-drag-drop

# 2. Make changes and commit
git add .
git commit -m "feat: add node drag-drop support"

# 3. Run tests before push
bun run lint
bun run type-check
bun run test:e2e

# 4. Push and create PR
git push origin feature/node-drag-drop
```

### Commit Message Format

```
type(scope): description

Types: feat, fix, refactor, docs, test, chore
Scope: graph, documents, query, ui, api, i18n

Examples:
feat(graph): add drag-drop node repositioning
fix(query): handle streaming connection errors
refactor(api): extract common fetch logic
docs(readme): update setup instructions
```

---

## Common Tasks

### Adding a New Component

```bash
# 1. Create component file
touch src/components/graph/NodeSearch.tsx

# 2. Component template
```

```tsx
// src/components/graph/NodeSearch.tsx
"use client";

import { useState } from "react";
import { Search } from "lucide-react";
import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";

interface NodeSearchProps {
  onSearch: (query: string) => void;
}

export function NodeSearch({ onSearch }: NodeSearchProps) {
  const [query, setQuery] = useState("");

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    onSearch(query);
  };

  return (
    <form onSubmit={handleSubmit} className="flex gap-2">
      <Input
        value={query}
        onChange={(e) => setQuery(e.target.value)}
        placeholder="Search nodes..."
      />
      <Button type="submit" size="icon">
        <Search className="h-4 w-4" />
      </Button>
    </form>
  );
}
```

### Adding a New API Endpoint

```tsx
// src/lib/api/graph.ts
import { apiClient } from "./client";

export async function searchNodes(query: string) {
  const response = await apiClient.get<NodeSearchResult[]>(
    `/api/graph/search?q=${encodeURIComponent(query)}`
  );
  return response.data;
}
```

### Adding a Translation

```json
// public/locales/en/common.json
{
  "graph": {
    "search": "Search nodes",
    "searchPlaceholder": "Type to search..."
  }
}
```

```tsx
// Usage in component
import { useTranslation } from "react-i18next";

function NodeSearch() {
  const { t } = useTranslation();

  return <Input placeholder={t("graph.searchPlaceholder")} />;
}
```

### Adding a Zustand Store

```tsx
// src/stores/graph-store.ts
import { create } from "zustand";
import { persist } from "zustand/middleware";

interface GraphState {
  layout: "force" | "circular" | "grid";
  selectedNode: string | null;
  setLayout: (layout: GraphState["layout"]) => void;
  selectNode: (id: string | null) => void;
}

export const useGraphStore = create<GraphState>()(
  persist(
    (set) => ({
      layout: "force",
      selectedNode: null,
      setLayout: (layout) => set({ layout }),
      selectNode: (id) => set({ selectedNode: id }),
    }),
    { name: "edgequake-graph" }
  )
);
```

### Adding a React Query Hook

```tsx
// src/hooks/use-graph-data.ts
import { useQuery } from "@tanstack/react-query";
import { fetchGraph } from "@/lib/api/graph";

export function useGraphData(workspaceId: string) {
  return useQuery({
    queryKey: ["graph", workspaceId],
    queryFn: () => fetchGraph(workspaceId),
    staleTime: 1000 * 60 * 5, // 5 minutes
  });
}
```

---

## Architecture Overview

### Data Flow

```
User Action
    ↓
React Component (UI)
    ↓
Custom Hook (useGraphData)
    ↓
React Query (caching layer)
    ↓
API Client (fetch wrapper)
    ↓
EdgeQuake Backend
    ↓
Response (cached by React Query)
    ↓
Component Re-render
```

### State Management Strategy

| State Type      | Solution     | Examples                             |
| --------------- | ------------ | ------------------------------------ |
| Server state    | React Query  | Graph data, documents, query results |
| Global UI state | Zustand      | Theme, sidebar, settings             |
| Local UI state  | useState     | Form inputs, dialogs                 |
| URL state       | searchParams | Filters, selected node               |

### Component Patterns

**Container Pattern:**

```tsx
// Container: handles data fetching
function GraphPageContainer() {
  const { data, isLoading, error } = useGraphData()

  if (isLoading) return <GraphSkeleton />
  if (error) return <ErrorMessage error={error} />

  return <GraphViewer data={data} />
}

// Presenter: pure UI component
function GraphViewer({ data }: { data: GraphData }) {
  return (/* render graph */)
}
```

**Compound Component Pattern:**

```tsx
// Used in shadcn/ui components
<Dialog>
  <DialogTrigger>Open</DialogTrigger>
  <DialogContent>
    <DialogHeader>
      <DialogTitle>Title</DialogTitle>
    </DialogHeader>
    <DialogDescription>Content</DialogDescription>
  </DialogContent>
</Dialog>
```

---

## Contributing Guidelines

### Before Submitting a PR

- [ ] Code passes `bun run lint`
- [ ] Code passes `bun run type-check`
- [ ] E2E tests pass `bun run test:e2e`
- [ ] New features have tests
- [ ] Translations added for new text
- [ ] No console.log statements
- [ ] Components are accessible
- [ ] Responsive on mobile

### Code Review Checklist

- [ ] Code is readable and well-organized
- [ ] TypeScript types are explicit (no `any`)
- [ ] Error states are handled
- [ ] Loading states are handled
- [ ] Components are reasonably sized
- [ ] Reusable logic is extracted to hooks
- [ ] No hardcoded strings (use i18n)

### File Naming Conventions

| Type       | Convention                | Example             |
| ---------- | ------------------------- | ------------------- |
| Components | PascalCase                | `GraphViewer.tsx`   |
| Hooks      | camelCase with use prefix | `use-graph-data.ts` |
| Utilities  | camelCase                 | `format-date.ts`    |
| Types      | PascalCase                | `graph-types.ts`    |
| Stores     | kebab-case                | `graph-store.ts`    |

---

## Troubleshooting

### Common Issues

**"Module not found" error**

```bash
# Clear Bun cache and reinstall
rm -rf node_modules bun.lockb
bun install
```

**TypeScript errors not showing in IDE**

```bash
# Restart TypeScript server in VS Code
Cmd+Shift+P → "TypeScript: Restart TS Server"
```

**E2E tests timing out**

```bash
# Increase timeout in playwright.config.ts
timeout: 60000

# Or run with headed mode to debug
bun run test:e2e --headed
```

**API connection refused**

```bash
# Ensure EdgeQuake backend is running
cd ../edgequake
cargo run --bin edgequake-api

# Check API URL in .env.local
NEXT_PUBLIC_API_URL=http://localhost:8000/api
```

**Graph not rendering**

```bash
# Check browser console for WebGL errors
# Sigma.js requires WebGL support

# Try disabling hardware acceleration in browser
# Or use software rendering fallback
```

### Debug Tips

```tsx
// Enable React Query devtools
import { ReactQueryDevtools } from "@tanstack/react-query-devtools";

// In layout.tsx
<ReactQueryDevtools initialIsOpen={false} />;

// Add to Sigma for debugging
window.__sigma = sigma; // In GraphViewer component
```

---

## Useful Commands

```bash
# Development
bun run dev           # Start dev server
bun run build         # Production build
bun run start         # Start production server

# Quality
bun run lint          # Run ESLint
bun run lint:fix      # Auto-fix lint issues
bun run type-check    # TypeScript check

# Testing
bun run test:e2e      # Run Playwright tests
bun run test:e2e --ui # Run with UI mode

# Dependencies
bun add <package>     # Add dependency
bun add -D <package>  # Add dev dependency
bun update            # Update all dependencies
```

---

## Quick Reference

### API Endpoints

| Endpoint               | Method   | Description              |
| ---------------------- | -------- | ------------------------ |
| `/api/health`          | GET      | Health check             |
| `/api/graph`           | GET      | Get graph data           |
| `/api/documents`       | GET/POST | List/upload documents    |
| `/api/documents/:id`   | DELETE   | Delete document          |
| `/api/query`           | POST     | Submit query (streaming) |
| `/api/pipeline/status` | GET      | Pipeline status          |

### Environment Variables

| Variable              | Description     | Default                     |
| --------------------- | --------------- | --------------------------- |
| `NEXT_PUBLIC_API_URL` | Backend API URL | `http://localhost:8000/api` |
| `NEXT_PUBLIC_WS_URL`  | WebSocket URL   | `ws://localhost:8000/ws`    |

---

## Cross-References

- **Gap Analysis:** [001-gap-analysis.md](./001-gap-analysis.md)
- **Proposed Solutions:** [002-proposed-solutions.md](./002-proposed-solutions.md)
- **Roadmap:** [003-prioritization-roadmap.md](./003-prioritization-roadmap.md)
- **QA Plan:** [006-qa-plan.md](./006-qa-plan.md)
