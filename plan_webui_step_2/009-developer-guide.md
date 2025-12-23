# Developer Quick Start Guide

> **Document Version:** 1.0  
> **Date:** 2024-12-23  
> **Purpose:** Help developers quickly begin implementing the gap closure plan

---

## Table of Contents

1. [Prerequisites](#prerequisites)
2. [Repository Setup](#repository-setup)
3. [Project Structure](#project-structure)
4. [Development Workflow](#development-workflow)
5. [Key Implementation Patterns](#key-implementation-patterns)
6. [Common Tasks](#common-tasks)
7. [Troubleshooting](#troubleshooting)
8. [Resources](#resources)

---

## Prerequisites

### Required Software

| Tool | Version | Purpose |
|------|---------|---------|
| Node.js | ≥ 20.x | Runtime |
| Bun | ≥ 1.0 | Package manager & runner |
| Git | ≥ 2.x | Version control |
| VS Code | Latest | Recommended IDE |

### Recommended VS Code Extensions

```json
{
  "recommendations": [
    "dbaeumer.vscode-eslint",
    "esbenp.prettier-vscode",
    "bradlc.vscode-tailwindcss",
    "ms-vscode.vscode-typescript-next",
    "lokalise.i18n-ally"
  ]
}
```

---

## Repository Setup

### 1. Clone and Install

```bash
# Clone the repository
git clone https://github.com/raphaelmansuy/edgequake.git
cd edgequake

# Navigate to WebUI
cd edgequake_webui

# Install dependencies
bun install
```

### 2. Environment Configuration

```bash
# Copy environment template
cp .env.example .env.local

# Edit with your settings
vim .env.local
```

**.env.local contents:**

```env
# API Configuration
NEXT_PUBLIC_API_URL=http://localhost:8080
NEXT_PUBLIC_API_VERSION=v1

# Feature Flags
NEXT_PUBLIC_ENABLE_I18N=true
NEXT_PUBLIC_ENABLE_MERMAID=true
NEXT_PUBLIC_ENABLE_LATEX=true

# Debug
NEXT_PUBLIC_DEBUG=true
```

### 3. Start Development Server

```bash
# Start Next.js dev server
bun run dev

# Open in browser
open http://localhost:3000
```

### 4. Verify Setup

```bash
# Run type check
bun run type-check

# Run linter
bun run lint

# Run tests
bun test
```

---

## Project Structure

### Directory Layout

```
edgequake_webui/
├── src/
│   ├── app/                    # Next.js App Router pages
│   │   ├── (dashboard)/        # Dashboard layout group
│   │   │   ├── documents/      # Document management page
│   │   │   ├── graph/          # Graph visualization page
│   │   │   └── query/          # Query interface page
│   │   ├── api/                # API route handlers (if any)
│   │   ├── layout.tsx          # Root layout
│   │   └── page.tsx            # Home page
│   │
│   ├── components/             # React components
│   │   ├── ui/                 # Shadcn/Radix primitives
│   │   ├── graph/              # Graph-related components
│   │   ├── documents/          # Document-related components
│   │   ├── query/              # Query-related components
│   │   └── shared/             # Shared components
│   │
│   ├── hooks/                  # Custom React hooks
│   │   ├── use-documents.ts
│   │   ├── use-graph.ts
│   │   └── use-query.ts
│   │
│   ├── lib/                    # Utilities and configuration
│   │   ├── api/                # API client functions
│   │   ├── i18n/               # i18n configuration (NEW)
│   │   └── utils.ts            # Utility functions
│   │
│   ├── stores/                 # Zustand stores
│   │   ├── graph-store.ts
│   │   ├── settings-store.ts
│   │   └── query-store.ts
│   │
│   ├── locales/                # Translation files (NEW)
│   │   ├── en/
│   │   ├── zh/
│   │   ├── fr/
│   │   ├── ar/
│   │   └── zh_TW/
│   │
│   └── types/                  # TypeScript type definitions
│       ├── api.ts
│       ├── graph.ts
│       └── document.ts
│
├── public/                     # Static assets
├── tests/                      # Test files
│   ├── unit/
│   ├── integration/
│   └── e2e/
│
├── next.config.ts              # Next.js configuration
├── tailwind.config.js          # Tailwind CSS configuration
├── tsconfig.json               # TypeScript configuration
└── package.json
```

---

### Key Files to Know

| File | Purpose | Modify When |
|------|---------|-------------|
| `src/app/layout.tsx` | Root layout, providers | Adding global providers |
| `src/lib/api/client.ts` | API client setup | Changing API config |
| `src/stores/settings-store.ts` | App settings state | Adding settings |
| `src/components/ui/*` | UI primitives | Rarely (Shadcn) |
| `tailwind.config.js` | Styling config | Adding design tokens |

---

## Development Workflow

### Branch Strategy

```
main                    # Production-ready
├── develop             # Integration branch
│   ├── feature/GAP-01-i18n
│   ├── feature/GAP-05-pagination
│   └── feature/GAP-12-latex
```

### Commit Convention

```
<type>(<scope>): <description>

feat(i18n): add language selector component
fix(graph): correct node dragging position
docs(readme): update setup instructions
test(documents): add pagination tests
```

**Types:** `feat`, `fix`, `docs`, `style`, `refactor`, `test`, `chore`

---

### PR Workflow

1. **Create branch:** `git checkout -b feature/GAP-XX-description`
2. **Make changes:** Implement the gap solution
3. **Test locally:** `bun test && bun run type-check`
4. **Commit:** `git commit -m "feat(scope): description"`
5. **Push:** `git push origin feature/GAP-XX-description`
6. **Open PR:** Reference gap number in title
7. **Review:** Address feedback
8. **Merge:** Squash and merge to develop

---

## Key Implementation Patterns

### 1. Adding a New i18n String

**Step 1:** Add to English locale

```json
// src/locales/en/common.json
{
  "documents": {
    "uploadButton": "Upload Document",
    "deleteConfirm": "Are you sure you want to delete?"
  }
}
```

**Step 2:** Add to other locales

```json
// src/locales/zh/common.json
{
  "documents": {
    "uploadButton": "上传文档",
    "deleteConfirm": "确定要删除吗？"
  }
}
```

**Step 3:** Use in component

```tsx
import { useTranslation } from 'react-i18next';

function DocumentUpload() {
  const { t } = useTranslation('common');

  return (
    <Button>{t('documents.uploadButton')}</Button>
  );
}
```

---

### 2. Adding a New API Endpoint Hook

**Step 1:** Define API function

```ts
// src/lib/api/documents.ts
export async function getDocuments(params: GetDocumentsParams) {
  const response = await fetch(`${API_URL}/documents?${new URLSearchParams(params)}`);
  if (!response.ok) throw new Error('Failed to fetch documents');
  return response.json() as Promise<DocumentsResponse>;
}
```

**Step 2:** Create hook

```ts
// src/hooks/use-documents.ts
import { useQuery } from '@tanstack/react-query';
import { getDocuments } from '@/lib/api/documents';

export function useDocuments(params: GetDocumentsParams) {
  return useQuery({
    queryKey: ['documents', params],
    queryFn: () => getDocuments(params),
  });
}
```

**Step 3:** Use in component

```tsx
function DocumentList() {
  const { data, isLoading, error } = useDocuments({
    page: 1,
    pageSize: 10,
  });

  if (isLoading) return <Skeleton />;
  if (error) return <ErrorMessage error={error} />;

  return <DocumentTable documents={data.items} />;
}
```

---

### 3. Adding a Zustand Store Action

**Step 1:** Add action to store

```ts
// src/stores/graph-store.ts
interface GraphStore {
  nodes: Node[];
  selectedNode: Node | null;
  
  // Actions
  setNodes: (nodes: Node[]) => void;
  selectNode: (id: string) => void;
  updateNodePosition: (id: string, x: number, y: number) => void; // NEW
}

export const useGraphStore = create<GraphStore>((set, get) => ({
  nodes: [],
  selectedNode: null,

  setNodes: (nodes) => set({ nodes }),
  
  selectNode: (id) => {
    const node = get().nodes.find(n => n.id === id);
    set({ selectedNode: node ?? null });
  },

  // NEW action
  updateNodePosition: (id, x, y) => {
    set((state) => ({
      nodes: state.nodes.map(node =>
        node.id === id ? { ...node, x, y } : node
      ),
    }));
  },
}));
```

**Step 2:** Use in component

```tsx
function GraphNode({ node }) {
  const updateNodePosition = useGraphStore((s) => s.updateNodePosition);

  const handleDragEnd = (e: DragEndEvent) => {
    updateNodePosition(node.id, e.x, e.y);
  };

  return <DraggableNode onDragEnd={handleDragEnd} />;
}
```

---

### 4. Creating a New Component

**Step 1:** Create component file

```tsx
// src/components/documents/document-filter.tsx
'use client';

import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Select, SelectContent, SelectItem, SelectTrigger } from '@/components/ui/select';

interface DocumentFilterProps {
  onFilterChange: (filters: Filters) => void;
  initialFilters?: Filters;
}

export function DocumentFilter({ onFilterChange, initialFilters }: DocumentFilterProps) {
  const { t } = useTranslation('common');
  const [status, setStatus] = useState(initialFilters?.status ?? 'all');

  const handleStatusChange = (value: string) => {
    setStatus(value);
    onFilterChange({ ...initialFilters, status: value });
  };

  return (
    <div className="flex gap-4">
      <Select value={status} onValueChange={handleStatusChange}>
        <SelectTrigger className="w-[180px]">
          {t('documents.filterByStatus')}
        </SelectTrigger>
        <SelectContent>
          <SelectItem value="all">{t('common.all')}</SelectItem>
          <SelectItem value="pending">{t('status.pending')}</SelectItem>
          <SelectItem value="completed">{t('status.completed')}</SelectItem>
        </SelectContent>
      </Select>
    </div>
  );
}
```

**Step 2:** Export from index

```ts
// src/components/documents/index.ts
export { DocumentFilter } from './document-filter';
```

**Step 3:** Write tests

```tsx
// tests/unit/document-filter.test.tsx
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { DocumentFilter } from '@/components/documents';

describe('DocumentFilter', () => {
  it('calls onFilterChange when status changes', async () => {
    const onFilterChange = vi.fn();
    render(<DocumentFilter onFilterChange={onFilterChange} />);

    await userEvent.click(screen.getByRole('combobox'));
    await userEvent.click(screen.getByText('Completed'));

    expect(onFilterChange).toHaveBeenCalledWith({ status: 'completed' });
  });
});
```

---

## Common Tasks

### Task 1: Run Tests

```bash
# Run all tests
bun test

# Run with coverage
bun test --coverage

# Run specific test file
bun test document-filter

# Run in watch mode
bun test --watch

# Run E2E tests
bun run test:e2e
```

---

### Task 2: Add a New Dependency

```bash
# Add runtime dependency
bun add <package-name>

# Add dev dependency
bun add -D <package-name>

# Check for vulnerabilities
bun audit
```

---

### Task 3: Build for Production

```bash
# Create production build
bun run build

# Analyze bundle size
ANALYZE=true bun run build

# Start production server locally
bun run start
```

---

### Task 4: Update i18n Translations

1. Use i18n-ally extension to find missing translations
2. Run extraction script (if configured): `bun run i18n:extract`
3. Update all locale files
4. Test with each language: Change language in app

---

### Task 5: Debug a Component

```tsx
// Add to component for debugging
console.log('[DocumentFilter] props:', props);
console.log('[DocumentFilter] state:', { status });

// Use React DevTools
// Press F12 → Components tab → Search for component

// Use React Query DevTools (already integrated)
// Look for floating button in bottom-right
```

---

## Troubleshooting

### Issue: Module Not Found

```bash
# Clear node_modules and reinstall
rm -rf node_modules bun.lockb
bun install
```

### Issue: Type Errors After Update

```bash
# Regenerate types
bun run type-check

# Clear TypeScript cache
rm -rf .next/cache
```

### Issue: Tests Failing

```bash
# Clear test cache
bun test --clearCache

# Run single test in isolation
bun test document-filter.test.tsx --no-cache
```

### Issue: Dev Server Slow

```bash
# Clear Next.js cache
rm -rf .next

# Restart dev server
bun run dev
```

### Issue: Styles Not Updating

```bash
# Rebuild Tailwind
bun run dev

# Check tailwind.config.js content paths
```

---

## Resources

### Documentation Links

| Resource | URL |
|----------|-----|
| Next.js Docs | https://nextjs.org/docs |
| React Query | https://tanstack.com/query |
| Zustand | https://docs.pmnd.rs/zustand |
| Tailwind CSS | https://tailwindcss.com/docs |
| Radix UI | https://radix-ui.com |
| i18next | https://react.i18next.com |
| Vitest | https://vitest.dev |
| Playwright | https://playwright.dev |

---

### Project Documents

| Document | Purpose |
|----------|---------|
| [Gap Analysis](./002-gap-analysis.md) | What to implement |
| [Proposed Solutions](./003-proposed-solutions.md) | How to implement |
| [Prioritization](./004-prioritization-roadmap.md) | When to implement |
| [UX Improvements](./005-ux-improvements.md) | Design guidance |
| [Performance Strategy](./006-performance-strategy.md) | Optimization tips |
| [QA Plan](./007-qa-plan.md) | Testing approach |
| [Success Criteria](./008-success-criteria.md) | Definition of done |

---

### Code Examples (Reference)

For implementation patterns, refer to the LightRAG WebUI codebase:

| Feature | LightRAG File |
|---------|---------------|
| i18n setup | `lightrag_webui/src/i18n/index.ts` |
| Graph store | `lightrag_webui/src/stores/graph.ts` |
| Document manager | `lightrag_webui/src/features/DocumentManager.tsx` |
| Query interface | `lightrag_webui/src/features/RetrievalTesting.tsx` |
| Settings | `lightrag_webui/src/stores/settings.ts` |

---

## Quick Reference Card

```
┌─────────────────────────────────────────────────────────────┐
│                    Developer Quick Reference                │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  Start Development:    bun run dev                          │
│  Run Tests:            bun test                             │
│  Type Check:           bun run type-check                   │
│  Lint:                 bun run lint                         │
│  Build:                bun run build                        │
│                                                             │
│  Branch Naming:        feature/GAP-XX-description           │
│  Commit Format:        type(scope): description             │
│                                                             │
│  Key Directories:                                           │
│  ├─ Components:        src/components/                      │
│  ├─ Hooks:             src/hooks/                           │
│  ├─ Stores:            src/stores/                          │
│  ├─ API:               src/lib/api/                         │
│  └─ Locales:           src/locales/                         │
│                                                             │
│  Add i18n String:                                           │
│  1. Add to src/locales/en/*.json                            │
│  2. Add to other locale files                               │
│  3. Use: const { t } = useTranslation()                     │
│                                                             │
│  Add API Hook:                                              │
│  1. Define in src/lib/api/                                  │
│  2. Create hook in src/hooks/                               │
│  3. Use useQuery/useMutation from React Query               │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

---

*Document provides quick start guidance for implementing gap closure*
