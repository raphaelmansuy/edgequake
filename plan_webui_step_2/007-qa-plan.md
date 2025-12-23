# Quality Assurance Plan

> **Document Version:** 1.0  
> **Date:** 2024-12-23  
> **Purpose:** Comprehensive testing strategy and quality assurance processes

---

## Table of Contents

1. [QA Objectives](#qa-objectives)
2. [Testing Strategy](#testing-strategy)
3. [Unit Testing](#unit-testing)
4. [Integration Testing](#integration-testing)
5. [End-to-End Testing](#end-to-end-testing)
6. [Visual Regression Testing](#visual-regression-testing)
7. [Performance Testing](#performance-testing)
8. [Accessibility Testing](#accessibility-testing)
9. [Code Quality Standards](#code-quality-standards)
10. [CI/CD Quality Gates](#cicd-quality-gates)

---

## QA Objectives

### Primary Goals

1. **Reliability:** Ensure all features work consistently
2. **Maintainability:** Catch regressions early
3. **Accessibility:** WCAG 2.1 AA compliance
4. **Performance:** Meet defined benchmarks
5. **Cross-Browser:** Support major browsers

### Coverage Targets

| Test Type | Target Coverage |
|-----------|----------------|
| Unit Tests | 80% statements |
| Integration Tests | 70% of API endpoints |
| E2E Tests | 100% critical paths |
| Accessibility | WCAG 2.1 AA |

---

## Testing Strategy

### Testing Pyramid

```
                    ┌───────┐
                    │  E2E  │         10%
                    └───────┘
               ┌─────────────────┐
               │   Integration   │    20%
               └─────────────────┘
          ┌───────────────────────────┐
          │        Unit Tests         │   70%
          └───────────────────────────┘
```

### Test Distribution by Feature

| Feature Area | Unit | Integration | E2E |
|--------------|------|-------------|-----|
| Graph Visualization | 15 | 5 | 3 |
| Document Management | 20 | 10 | 5 |
| Query Interface | 25 | 8 | 4 |
| i18n System | 30 | 5 | 2 |
| API Layer | 20 | 15 | 3 |
| UI Components | 40 | 10 | 5 |
| **Total** | **150** | **53** | **22** |

---

## Unit Testing

### Framework & Configuration

**Stack:**
- Vitest (compatible with Next.js)
- @testing-library/react
- @testing-library/user-event
- msw (Mock Service Worker)

**Configuration:**

```ts
// vitest.config.ts
import { defineConfig } from 'vitest/config';
import react from '@vitejs/plugin-react';
import path from 'path';

export default defineConfig({
  plugins: [react()],
  test: {
    environment: 'jsdom',
    globals: true,
    setupFiles: ['./tests/setup.ts'],
    include: ['**/*.{test,spec}.{ts,tsx}'],
    coverage: {
      provider: 'v8',
      reporter: ['text', 'json', 'html'],
      exclude: ['node_modules/', 'tests/'],
      thresholds: {
        statements: 80,
        branches: 75,
        functions: 80,
        lines: 80,
      },
    },
  },
  resolve: {
    alias: {
      '@': path.resolve(__dirname, './src'),
    },
  },
});
```

---

### Test Setup

```ts
// tests/setup.ts
import '@testing-library/jest-dom';
import { vi } from 'vitest';

// Mock Next.js router
vi.mock('next/navigation', () => ({
  useRouter: () => ({
    push: vi.fn(),
    replace: vi.fn(),
    prefetch: vi.fn(),
  }),
  usePathname: () => '/documents',
  useSearchParams: () => new URLSearchParams(),
}));

// Mock i18n
vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string) => key,
    i18n: { language: 'en', changeLanguage: vi.fn() },
  }),
}));

// Global fetch mock
global.fetch = vi.fn();
```

---

### Unit Test Examples

#### Component Test

```tsx
// components/document-row.test.tsx
import { render, screen, fireEvent } from '@testing-library/react';
import { DocumentRow } from './document-row';

describe('DocumentRow', () => {
  const mockDoc = {
    id: '1',
    title: 'Test Document',
    status: 'completed',
    created_at: '2024-01-15T10:00:00Z',
    chunk_count: 5,
  };

  it('renders document title', () => {
    render(<DocumentRow doc={mockDoc} />);
    expect(screen.getByText('Test Document')).toBeInTheDocument();
  });

  it('displays correct status badge', () => {
    render(<DocumentRow doc={mockDoc} />);
    expect(screen.getByText('completed')).toHaveClass('bg-green-500');
  });

  it('calls onDelete when delete button clicked', async () => {
    const onDelete = vi.fn();
    render(<DocumentRow doc={mockDoc} onDelete={onDelete} />);
    
    await fireEvent.click(screen.getByRole('button', { name: /delete/i }));
    expect(onDelete).toHaveBeenCalledWith('1');
  });

  it('formats date correctly for different locales', () => {
    render(<DocumentRow doc={mockDoc} locale="de" />);
    expect(screen.getByText('15.01.2024')).toBeInTheDocument();
  });
});
```

---

#### Hook Test

```tsx
// hooks/use-documents.test.ts
import { renderHook, waitFor } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { useDocuments } from './use-documents';

const wrapper = ({ children }: { children: React.ReactNode }) => {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  return (
    <QueryClientProvider client={queryClient}>
      {children}
    </QueryClientProvider>
  );
};

describe('useDocuments', () => {
  beforeEach(() => {
    vi.mocked(fetch).mockResolvedValue({
      ok: true,
      json: async () => ({
        items: [{ id: '1', title: 'Doc 1' }],
        total: 1,
        page: 1,
      }),
    } as Response);
  });

  it('fetches documents on mount', async () => {
    const { result } = renderHook(() => useDocuments(), { wrapper });

    await waitFor(() => expect(result.current.isSuccess).toBe(true));
    expect(result.current.data?.items).toHaveLength(1);
  });

  it('refetches when page changes', async () => {
    const { result, rerender } = renderHook(
      ({ page }) => useDocuments({ page }),
      { wrapper, initialProps: { page: 1 } }
    );

    await waitFor(() => expect(result.current.isSuccess).toBe(true));
    
    rerender({ page: 2 });
    await waitFor(() => expect(fetch).toHaveBeenCalledTimes(2));
  });
});
```

---

#### Store Test

```tsx
// stores/graph-store.test.ts
import { describe, it, expect, beforeEach } from 'vitest';
import { useGraphStore } from './graph-store';

describe('GraphStore', () => {
  beforeEach(() => {
    useGraphStore.setState({
      nodes: [],
      edges: [],
      selectedNode: null,
    });
  });

  it('adds nodes correctly', () => {
    useGraphStore.getState().setNodes([
      { id: '1', label: 'Node 1' },
      { id: '2', label: 'Node 2' },
    ]);

    expect(useGraphStore.getState().nodes).toHaveLength(2);
  });

  it('selects node by id', () => {
    useGraphStore.getState().setNodes([{ id: '1', label: 'Node 1' }]);
    useGraphStore.getState().selectNode('1');

    expect(useGraphStore.getState().selectedNode?.id).toBe('1');
  });

  it('clears selection when node not found', () => {
    useGraphStore.getState().selectNode('nonexistent');
    expect(useGraphStore.getState().selectedNode).toBeNull();
  });
});
```

---

## Integration Testing

### API Integration Tests

```tsx
// tests/integration/api-documents.test.ts
import { setupServer } from 'msw/node';
import { http, HttpResponse } from 'msw';
import { documentsApi } from '@/lib/api/documents';

const server = setupServer(
  http.get('/api/documents', () => {
    return HttpResponse.json({
      items: [{ id: '1', title: 'Test' }],
      total: 1,
      page: 1,
    });
  }),

  http.post('/api/documents', async ({ request }) => {
    const body = await request.json();
    return HttpResponse.json({
      id: 'new-id',
      ...body,
    }, { status: 201 });
  }),

  http.delete('/api/documents/:id', ({ params }) => {
    return HttpResponse.json({ success: true });
  })
);

beforeAll(() => server.listen());
afterEach(() => server.resetHandlers());
afterAll(() => server.close());

describe('Documents API', () => {
  it('fetches document list', async () => {
    const result = await documentsApi.getDocuments({ page: 1, pageSize: 10 });
    expect(result.items).toHaveLength(1);
    expect(result.items[0].title).toBe('Test');
  });

  it('creates document', async () => {
    const result = await documentsApi.createDocument({
      content: 'Test content',
      filename: 'test.txt',
    });
    expect(result.id).toBe('new-id');
  });

  it('deletes document', async () => {
    const result = await documentsApi.deleteDocument('1');
    expect(result.success).toBe(true);
  });

  it('handles API errors', async () => {
    server.use(
      http.get('/api/documents', () => {
        return HttpResponse.json(
          { error: 'Server error' },
          { status: 500 }
        );
      })
    );

    await expect(documentsApi.getDocuments()).rejects.toThrow('Server error');
  });
});
```

---

### Component Integration Tests

```tsx
// tests/integration/document-manager.test.tsx
import { render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { DocumentManager } from '@/components/document-manager';
import { TestProviders } from '../test-utils';

describe('DocumentManager Integration', () => {
  it('loads and displays documents', async () => {
    render(
      <TestProviders>
        <DocumentManager />
      </TestProviders>
    );

    await waitFor(() => {
      expect(screen.getByText('Test Document 1')).toBeInTheDocument();
    });
  });

  it('filters documents by search', async () => {
    const user = userEvent.setup();
    
    render(
      <TestProviders>
        <DocumentManager />
      </TestProviders>
    );

    await waitFor(() => screen.getByText('Test Document 1'));

    const searchInput = screen.getByPlaceholderText(/search/i);
    await user.type(searchInput, 'specific');

    await waitFor(() => {
      expect(screen.queryByText('Test Document 1')).not.toBeInTheDocument();
      expect(screen.getByText('Specific Document')).toBeInTheDocument();
    });
  });

  it('paginates through documents', async () => {
    const user = userEvent.setup();
    
    render(
      <TestProviders>
        <DocumentManager />
      </TestProviders>
    );

    await waitFor(() => screen.getByText('Page 1'));

    const nextButton = screen.getByRole('button', { name: /next/i });
    await user.click(nextButton);

    await waitFor(() => {
      expect(screen.getByText('Page 2')).toBeInTheDocument();
    });
  });
});
```

---

## End-to-End Testing

### Framework: Playwright

```ts
// playwright.config.ts
import { defineConfig, devices } from '@playwright/test';

export default defineConfig({
  testDir: './e2e',
  fullyParallel: true,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 2 : 0,
  workers: process.env.CI ? 1 : undefined,
  reporter: [
    ['html'],
    ['json', { outputFile: 'test-results.json' }],
  ],
  use: {
    baseURL: 'http://localhost:3000',
    trace: 'on-first-retry',
    screenshot: 'only-on-failure',
  },
  projects: [
    { name: 'chromium', use: { ...devices['Desktop Chrome'] } },
    { name: 'firefox', use: { ...devices['Desktop Firefox'] } },
    { name: 'webkit', use: { ...devices['Desktop Safari'] } },
    { name: 'Mobile Chrome', use: { ...devices['Pixel 5'] } },
    { name: 'Mobile Safari', use: { ...devices['iPhone 12'] } },
  ],
  webServer: {
    command: 'bun run dev',
    url: 'http://localhost:3000',
    reuseExistingServer: !process.env.CI,
  },
});
```

---

### Critical Path E2E Tests

```ts
// e2e/document-upload.spec.ts
import { test, expect } from '@playwright/test';

test.describe('Document Upload Flow', () => {
  test('uploads document and displays in list', async ({ page }) => {
    await page.goto('/documents');

    // Click upload button
    await page.getByRole('button', { name: /upload/i }).click();

    // Upload file
    const fileInput = page.locator('input[type="file"]');
    await fileInput.setInputFiles('./e2e/fixtures/sample.txt');

    // Wait for upload
    await expect(page.getByText('Upload successful')).toBeVisible();

    // Verify in list
    await expect(page.getByText('sample.txt')).toBeVisible();
  });

  test('shows progress during upload', async ({ page }) => {
    await page.goto('/documents');
    await page.getByRole('button', { name: /upload/i }).click();

    const fileInput = page.locator('input[type="file"]');
    await fileInput.setInputFiles('./e2e/fixtures/large-file.txt');

    // Progress should appear
    await expect(page.getByRole('progressbar')).toBeVisible();
  });

  test('handles upload errors gracefully', async ({ page }) => {
    // Mock failed upload
    await page.route('/api/documents', (route) => {
      route.fulfill({ status: 500, body: 'Server error' });
    });

    await page.goto('/documents');
    await page.getByRole('button', { name: /upload/i }).click();
    
    const fileInput = page.locator('input[type="file"]');
    await fileInput.setInputFiles('./e2e/fixtures/sample.txt');

    await expect(page.getByText(/error/i)).toBeVisible();
  });
});
```

---

```ts
// e2e/graph-interaction.spec.ts
import { test, expect } from '@playwright/test';

test.describe('Graph Interaction', () => {
  test('displays graph and allows node selection', async ({ page }) => {
    await page.goto('/graph');

    // Wait for graph to load
    await expect(page.locator('canvas')).toBeVisible();

    // Click on a node (approximate position)
    await page.locator('canvas').click({ position: { x: 400, y: 300 } });

    // Info panel should show
    await expect(page.getByTestId('node-info-panel')).toBeVisible();
  });

  test('search filters graph nodes', async ({ page }) => {
    await page.goto('/graph');

    await page.getByPlaceholder(/search/i).fill('ENTITY_NAME');
    await page.keyboard.press('Enter');

    // Should highlight matching nodes
    await expect(page.getByTestId('search-results')).toContainText('1 result');
  });
});
```

---

```ts
// e2e/query-flow.spec.ts
import { test, expect } from '@playwright/test';

test.describe('Query Flow', () => {
  test('submits query and displays response', async ({ page }) => {
    await page.goto('/query');

    // Type query
    await page.getByRole('textbox', { name: /query/i }).fill('What is EdgeQuake?');

    // Submit
    await page.getByRole('button', { name: /submit|send/i }).click();

    // Response should appear
    await expect(page.getByTestId('query-response')).toBeVisible({ timeout: 30000 });
  });

  test('shows streaming response', async ({ page }) => {
    await page.goto('/query');

    await page.getByRole('textbox', { name: /query/i }).fill('Explain the architecture');
    await page.getByRole('button', { name: /submit/i }).click();

    // Should show loading state
    await expect(page.getByTestId('streaming-indicator')).toBeVisible();

    // Content should stream in
    await expect(async () => {
      const content = await page.getByTestId('query-response').textContent();
      expect(content?.length).toBeGreaterThan(50);
    }).toPass({ timeout: 15000 });
  });

  test('displays chain of thought when enabled', async ({ page }) => {
    await page.goto('/query');

    // Enable COT
    await page.getByLabel(/chain of thought/i).check();

    await page.getByRole('textbox', { name: /query/i }).fill('Test query');
    await page.getByRole('button', { name: /submit/i }).click();

    // COT section should appear
    await expect(page.getByTestId('chain-of-thought')).toBeVisible({ timeout: 30000 });
  });
});
```

---

## Visual Regression Testing

### Chromatic Integration

```json
// package.json
{
  "scripts": {
    "chromatic": "chromatic --project-token=$CHROMATIC_TOKEN"
  }
}
```

### Storybook Stories for Visual Testing

```tsx
// components/document-table.stories.tsx
import type { Meta, StoryObj } from '@storybook/react';
import { DocumentTable } from './document-table';

const meta: Meta<typeof DocumentTable> = {
  title: 'Components/DocumentTable',
  component: DocumentTable,
  parameters: {
    chromatic: { viewports: [375, 768, 1280] },
  },
};

export default meta;
type Story = StoryObj<typeof DocumentTable>;

export const Empty: Story = {
  args: {
    documents: [],
  },
};

export const WithDocuments: Story = {
  args: {
    documents: generateMockDocuments(10),
  },
};

export const Loading: Story = {
  args: {
    isLoading: true,
  },
};

export const WithPagination: Story = {
  args: {
    documents: generateMockDocuments(100),
    pageSize: 10,
    showPagination: true,
  },
};
```

---

## Performance Testing

### Lighthouse CI

```yaml
# .github/workflows/lighthouse.yml
name: Lighthouse CI

on: [push]

jobs:
  lighthouse:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
        with:
          node-version: 20

      - run: bun install
      - run: bun run build

      - uses: treosh/lighthouse-ci-action@v11
        with:
          configPath: './lighthouserc.json'
          uploadArtifacts: true
```

```json
// lighthouserc.json
{
  "ci": {
    "collect": {
      "staticDistDir": "./.next/static"
    },
    "assert": {
      "assertions": {
        "categories:performance": ["warn", { "minScore": 0.8 }],
        "categories:accessibility": ["error", { "minScore": 0.9 }],
        "categories:best-practices": ["warn", { "minScore": 0.9 }],
        "categories:seo": ["warn", { "minScore": 0.9 }]
      }
    }
  }
}
```

---

## Accessibility Testing

### Automated a11y Testing

```tsx
// tests/a11y/document-manager.a11y.test.tsx
import { render } from '@testing-library/react';
import { axe, toHaveNoViolations } from 'jest-axe';
import { DocumentManager } from '@/components/document-manager';

expect.extend(toHaveNoViolations);

describe('DocumentManager Accessibility', () => {
  it('has no accessibility violations', async () => {
    const { container } = render(
      <TestProviders>
        <DocumentManager documents={mockDocuments} />
      </TestProviders>
    );

    const results = await axe(container);
    expect(results).toHaveNoViolations();
  });
});
```

### Playwright a11y Scanning

```ts
// e2e/a11y.spec.ts
import { test, expect } from '@playwright/test';
import AxeBuilder from '@axe-core/playwright';

test.describe('Accessibility Audit', () => {
  test('documents page has no a11y violations', async ({ page }) => {
    await page.goto('/documents');
    
    const accessibilityScanResults = await new AxeBuilder({ page })
      .withTags(['wcag2a', 'wcag2aa', 'wcag21aa'])
      .analyze();

    expect(accessibilityScanResults.violations).toEqual([]);
  });

  test('graph page has no a11y violations', async ({ page }) => {
    await page.goto('/graph');
    
    const accessibilityScanResults = await new AxeBuilder({ page })
      .exclude('canvas') // Canvas has limited a11y
      .analyze();

    expect(accessibilityScanResults.violations).toEqual([]);
  });
});
```

---

## Code Quality Standards

### ESLint Configuration

```js
// eslint.config.mjs
import eslint from '@eslint/js';
import tseslint from 'typescript-eslint';
import react from 'eslint-plugin-react';
import reactHooks from 'eslint-plugin-react-hooks';

export default tseslint.config(
  eslint.configs.recommended,
  ...tseslint.configs.strictTypeChecked,
  {
    plugins: {
      react,
      'react-hooks': reactHooks,
    },
    rules: {
      'react-hooks/rules-of-hooks': 'error',
      'react-hooks/exhaustive-deps': 'warn',
      '@typescript-eslint/no-unused-vars': 'error',
      '@typescript-eslint/no-explicit-any': 'warn',
    },
  }
);
```

---

### Prettier Configuration

```json
// .prettierrc
{
  "semi": true,
  "singleQuote": true,
  "tabWidth": 2,
  "trailingComma": "es5",
  "printWidth": 100
}
```

---

### Pre-commit Hooks

```json
// package.json
{
  "scripts": {
    "lint": "eslint . --ext .ts,.tsx",
    "format": "prettier --write .",
    "type-check": "tsc --noEmit",
    "test": "vitest run",
    "prepare": "husky install"
  },
  "lint-staged": {
    "*.{ts,tsx}": [
      "eslint --fix",
      "prettier --write"
    ]
  }
}
```

---

## CI/CD Quality Gates

### GitHub Actions Workflow

```yaml
# .github/workflows/ci.yml
name: CI

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

jobs:
  lint:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: oven-sh/setup-bun@v1

      - run: bun install
      - run: bun run lint

  type-check:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: oven-sh/setup-bun@v1

      - run: bun install
      - run: bun run type-check

  unit-tests:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: oven-sh/setup-bun@v1

      - run: bun install
      - run: bun run test -- --coverage

      - uses: codecov/codecov-action@v4
        with:
          files: ./coverage/lcov.info

  e2e-tests:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: oven-sh/setup-bun@v1

      - run: bun install
      - run: bunx playwright install --with-deps

      - run: bun run build
      - run: bun run test:e2e

      - uses: actions/upload-artifact@v4
        if: failure()
        with:
          name: playwright-report
          path: playwright-report/

  build:
    runs-on: ubuntu-latest
    needs: [lint, type-check, unit-tests]
    steps:
      - uses: actions/checkout@v4
      - uses: oven-sh/setup-bun@v1

      - run: bun install
      - run: bun run build

      - uses: actions/upload-artifact@v4
        with:
          name: build
          path: .next/
```

---

### Quality Gate Thresholds

| Gate | Threshold | Action on Fail |
|------|-----------|----------------|
| Lint errors | 0 | Block merge |
| Type errors | 0 | Block merge |
| Unit test coverage | 80% | Warn |
| Unit tests passing | 100% | Block merge |
| E2E tests passing | 100% | Block merge |
| Bundle size | +10KB | Warn |
| Lighthouse performance | 80 | Warn |
| Lighthouse a11y | 90 | Block merge |

---

## Test Execution Schedule

| Test Type | When | Duration |
|-----------|------|----------|
| Unit tests | Every commit | ~30s |
| Integration tests | Every commit | ~1m |
| E2E tests (fast) | Every PR | ~5m |
| E2E tests (full) | Nightly | ~30m |
| Visual regression | Weekly | ~15m |
| Performance | Weekly | ~10m |

---

## Cross-References

| Document | Relationship |
|----------|--------------|
| [Gap Analysis](./002-gap-analysis.md) | Test gaps identified |
| [Performance Strategy](./006-performance-strategy.md) | Performance test targets |
| [Success Criteria](./008-success-criteria.md) | Coverage requirements |
| [Developer Guide](./009-developer-guide.md) | How to run tests |

---

*Document defines comprehensive testing strategy and quality assurance processes*
