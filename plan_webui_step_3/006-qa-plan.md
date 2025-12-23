# Quality Assurance Plan

> **Document Version:** 1.0  
> **Date:** 2024-12-23  
> **Purpose:** Testing strategy, code quality standards, and CI/CD integration

---

## Table of Contents

1. [Testing Strategy Overview](#testing-strategy-overview)
2. [Test Pyramid](#test-pyramid)
3. [Unit Testing](#unit-testing)
4. [Integration Testing](#integration-testing)
5. [End-to-End Testing](#end-to-end-testing)
6. [Performance Testing](#performance-testing)
7. [Accessibility Testing](#accessibility-testing)
8. [Code Quality Standards](#code-quality-standards)
9. [CI/CD Pipeline](#cicd-pipeline)
10. [Release Checklist](#release-checklist)

---

## Testing Strategy Overview

### Current State

EdgeQuake WebUI has:

- ✅ Playwright E2E tests (`e2e/` directory)
- ✅ ESLint configuration
- ❌ No unit tests
- ❌ No integration tests
- ❌ No accessibility tests

### Target State

| Test Type     | Coverage Target | Automation |
| ------------- | --------------- | ---------- |
| Unit Tests    | 80%             | Yes        |
| Integration   | 60%             | Yes        |
| E2E           | Critical paths  | Yes        |
| Accessibility | All pages       | Yes        |
| Performance   | Core metrics    | Yes        |

---

## Test Pyramid

```
        ▲
       /E2E\           ~10% of tests
      /─────\          Slow, expensive, high confidence
     /Integration\     ~20% of tests
    /─────────────\    Medium speed, mock APIs
   /  Unit Tests   \   ~70% of tests
  /─────────────────\  Fast, isolated, focused
```

### Test Distribution Goals

| Layer         | Count | Run Time | Frequency    |
| ------------- | ----- | -------- | ------------ |
| Unit          | 200+  | < 30s    | Every commit |
| Integration   | 50+   | < 2min   | Every PR     |
| E2E           | 20+   | < 5min   | Every PR     |
| Accessibility | 10+   | < 1min   | Every PR     |
| Performance   | 5+    | < 3min   | Nightly      |

---

## Unit Testing

### Framework Setup

```bash
# Install testing dependencies
bun add -D vitest @testing-library/react @testing-library/jest-dom jsdom
```

```typescript
// vitest.config.ts
import { defineConfig } from "vitest/config";
import react from "@vitejs/plugin-react";
import path from "path";

export default defineConfig({
  plugins: [react()],
  test: {
    environment: "jsdom",
    globals: true,
    setupFiles: "./src/test/setup.ts",
    coverage: {
      provider: "v8",
      reporter: ["text", "html", "lcov"],
      exclude: ["node_modules/", "e2e/", "*.config.*"],
    },
    include: ["src/**/*.test.{ts,tsx}"],
  },
  resolve: {
    alias: {
      "@": path.resolve(__dirname, "./src"),
    },
  },
});
```

```typescript
// src/test/setup.ts
import "@testing-library/jest-dom";
import { vi } from "vitest";

// Mock next/navigation
vi.mock("next/navigation", () => ({
  useRouter: () => ({
    push: vi.fn(),
    replace: vi.fn(),
    back: vi.fn(),
  }),
  useSearchParams: () => new URLSearchParams(),
  usePathname: () => "/",
}));
```

### Unit Test Examples

**a) Hook Testing**

```typescript
// src/hooks/use-debounce.test.ts
import { renderHook, act } from "@testing-library/react";
import { useDebounce } from "./use-debounce";
import { vi } from "vitest";

describe("useDebounce", () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it("returns initial value immediately", () => {
    const { result } = renderHook(() => useDebounce("test", 500));
    expect(result.current).toBe("test");
  });

  it("debounces value changes", () => {
    const { result, rerender } = renderHook(
      ({ value }) => useDebounce(value, 500),
      { initialProps: { value: "initial" } }
    );

    rerender({ value: "updated" });
    expect(result.current).toBe("initial");

    act(() => {
      vi.advanceTimersByTime(500);
    });

    expect(result.current).toBe("updated");
  });
});
```

**b) Component Testing**

```typescript
// src/components/ui/button.test.tsx
import { render, screen, fireEvent } from "@testing-library/react";
import { Button } from "./button";

describe("Button", () => {
  it("renders children correctly", () => {
    render(<Button>Click me</Button>);
    expect(screen.getByRole("button")).toHaveTextContent("Click me");
  });

  it("calls onClick when clicked", () => {
    const handleClick = vi.fn();
    render(<Button onClick={handleClick}>Click</Button>);

    fireEvent.click(screen.getByRole("button"));
    expect(handleClick).toHaveBeenCalledTimes(1);
  });

  it("is disabled when disabled prop is true", () => {
    render(<Button disabled>Disabled</Button>);
    expect(screen.getByRole("button")).toBeDisabled();
  });

  it("applies variant classes correctly", () => {
    render(<Button variant="destructive">Delete</Button>);
    expect(screen.getByRole("button")).toHaveClass("bg-destructive");
  });
});
```

**c) Store Testing**

```typescript
// src/stores/graph-store.test.ts
import { describe, it, expect, beforeEach } from "vitest";
import { useGraphStore } from "./graph-store";

describe("graphStore", () => {
  beforeEach(() => {
    useGraphStore.setState({
      selectedNode: null,
      layout: "force",
      filters: {},
    });
  });

  it("selects a node", () => {
    const { selectNode, selectedNode } = useGraphStore.getState();

    selectNode("node-123");

    expect(useGraphStore.getState().selectedNode).toBe("node-123");
  });

  it("clears selection", () => {
    useGraphStore.setState({ selectedNode: "node-123" });

    useGraphStore.getState().clearSelection();

    expect(useGraphStore.getState().selectedNode).toBeNull();
  });

  it("updates layout", () => {
    useGraphStore.getState().setLayout("circular");

    expect(useGraphStore.getState().layout).toBe("circular");
  });
});
```

---

## Integration Testing

### API Mocking with MSW

```typescript
// src/test/mocks/handlers.ts
import { http, HttpResponse } from "msw";

export const handlers = [
  http.get("/api/health", () => {
    return HttpResponse.json({ status: "healthy" });
  }),

  http.get("/api/graph", () => {
    return HttpResponse.json({
      nodes: [
        { id: "1", label: "Entity A", type: "PERSON" },
        { id: "2", label: "Entity B", type: "ORG" },
      ],
      edges: [{ source: "1", target: "2", label: "works_at" }],
    });
  }),

  http.post("/api/documents", async ({ request }) => {
    const formData = await request.formData();
    const file = formData.get("file") as File;

    return HttpResponse.json({
      id: "doc-123",
      name: file.name,
      status: "uploaded",
    });
  }),

  http.post("/api/query", async ({ request }) => {
    const body = await request.json();

    // Return streaming response
    const encoder = new TextEncoder();
    const stream = new ReadableStream({
      start(controller) {
        controller.enqueue(encoder.encode('data: {"chunk":"Hello"}\n\n'));
        controller.enqueue(encoder.encode('data: {"chunk":" World"}\n\n'));
        controller.enqueue(encoder.encode("data: [DONE]\n\n"));
        controller.close();
      },
    });

    return new HttpResponse(stream, {
      headers: { "Content-Type": "text/event-stream" },
    });
  }),
];
```

```typescript
// src/test/mocks/server.ts
import { setupServer } from "msw/node";
import { handlers } from "./handlers";

export const server = setupServer(...handlers);
```

### Integration Test Examples

```typescript
// src/features/documents/document-upload.test.tsx
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClientProvider, QueryClient } from "@tanstack/react-query";
import { DocumentUploader } from "./document-uploader";
import { server } from "@/test/mocks/server";

const queryClient = new QueryClient({
  defaultOptions: { queries: { retry: false } },
});

const wrapper = ({ children }: { children: React.ReactNode }) => (
  <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
);

describe("DocumentUploader Integration", () => {
  beforeAll(() => server.listen());
  afterEach(() => server.resetHandlers());
  afterAll(() => server.close());

  it("uploads a document and shows success", async () => {
    const user = userEvent.setup();
    render(<DocumentUploader />, { wrapper });

    const file = new File(["content"], "test.txt", { type: "text/plain" });
    const input = screen.getByLabelText(/upload/i);

    await user.upload(input, file);

    await waitFor(() => {
      expect(screen.getByText(/uploaded successfully/i)).toBeInTheDocument();
    });
  });
});
```

---

## End-to-End Testing

### Playwright Configuration

```typescript
// playwright.config.ts (existing - enhance)
import { defineConfig, devices } from "@playwright/test";

export default defineConfig({
  testDir: "./e2e",
  fullyParallel: true,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 2 : 0,
  workers: process.env.CI ? 1 : undefined,
  reporter: [["html"], ["json", { outputFile: "test-results/results.json" }]],
  use: {
    baseURL: "http://localhost:3000",
    trace: "on-first-retry",
    screenshot: "only-on-failure",
  },
  projects: [
    { name: "chromium", use: devices["Desktop Chrome"] },
    { name: "firefox", use: devices["Desktop Firefox"] },
    { name: "mobile-chrome", use: devices["Pixel 5"] },
  ],
  webServer: {
    command: "bun run dev",
    url: "http://localhost:3000",
    reuseExistingServer: !process.env.CI,
  },
});
```

### E2E Test Examples

```typescript
// e2e/document-workflow.spec.ts
import { test, expect } from "@playwright/test";

test.describe("Document Workflow", () => {
  test("uploads, processes, and queries document", async ({ page }) => {
    // Navigate to documents page
    await page.goto("/documents");
    await expect(
      page.getByRole("heading", { name: /documents/i })
    ).toBeVisible();

    // Upload a document
    const fileInput = page.getByLabel(/upload/i);
    await fileInput.setInputFiles({
      name: "test-doc.txt",
      mimeType: "text/plain",
      buffer: Buffer.from(
        "Test document content about artificial intelligence"
      ),
    });

    // Wait for upload to complete
    await expect(page.getByText("test-doc.txt")).toBeVisible();

    // Wait for processing (with timeout)
    await expect(page.getByText(/processed/i)).toBeVisible({ timeout: 30000 });

    // Navigate to query and test
    await page.getByRole("link", { name: /query/i }).click();
    await expect(page.url()).toContain("/query");

    // Submit a query
    await page.getByPlaceholder(/ask a question/i).fill("What is AI?");
    await page.getByRole("button", { name: /send/i }).click();

    // Verify response appears
    await expect(page.getByTestId("response-message")).toBeVisible({
      timeout: 10000,
    });
  });
});
```

```typescript
// e2e/graph-interactions.spec.ts
import { test, expect } from "@playwright/test";

test.describe("Graph Viewer", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/graph");
    await expect(page.getByTestId("graph-container")).toBeVisible();
  });

  test("selects node and shows details", async ({ page }) => {
    // Click on a node
    await page
      .getByTestId("graph-container")
      .click({ position: { x: 200, y: 200 } });

    // Verify details panel appears
    await expect(page.getByTestId("node-details-panel")).toBeVisible();
  });

  test("zooms with controls", async ({ page }) => {
    const zoomIn = page.getByRole("button", { name: /zoom in/i });
    const initialZoom = await page.evaluate(() => {
      // Get sigma zoom level from DOM
      return window.__sigma?.getCamera().ratio;
    });

    await zoomIn.click();
    await zoomIn.click();

    const newZoom = await page.evaluate(
      () => window.__sigma?.getCamera().ratio
    );
    expect(newZoom).toBeLessThan(initialZoom || 1);
  });

  test("searches for nodes", async ({ page }) => {
    await page.getByRole("button", { name: /search/i }).click();
    await page.getByPlaceholder(/search/i).fill("test entity");

    // Verify search results
    await expect(page.getByRole("listitem")).toHaveCount.greaterThan(0);
  });
});
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
      - uses: oven-sh/setup-bun@v1
      - run: bun install
      - run: bun run build
      - name: Run Lighthouse
        uses: treosh/lighthouse-ci-action@v11
        with:
          urls: |
            http://localhost:3000/
            http://localhost:3000/graph
            http://localhost:3000/documents
          budgetPath: ./budget.json
          uploadArtifacts: true
```

```json
// budget.json
[
  {
    "path": "/*",
    "timings": [
      { "metric": "first-contentful-paint", "budget": 1000 },
      { "metric": "largest-contentful-paint", "budget": 2000 },
      { "metric": "interactive", "budget": 3000 }
    ],
    "resourceSizes": [
      { "resourceType": "script", "budget": 350 },
      { "resourceType": "total", "budget": 800 }
    ]
  }
]
```

---

## Accessibility Testing

### Automated Checks

```typescript
// e2e/accessibility.spec.ts
import { test, expect } from "@playwright/test";
import AxeBuilder from "@axe-core/playwright";

test.describe("Accessibility", () => {
  const pages = [
    { name: "Home", path: "/" },
    { name: "Graph", path: "/graph" },
    { name: "Documents", path: "/documents" },
    { name: "Query", path: "/query" },
    { name: "Settings", path: "/settings" },
  ];

  pages.forEach(({ name, path }) => {
    test(`${name} page has no accessibility violations`, async ({ page }) => {
      await page.goto(path);

      const accessibilityScanResults = await new AxeBuilder({ page })
        .withTags(["wcag2a", "wcag2aa", "wcag21a", "wcag21aa"])
        .exclude(".sigma-container") // Exclude canvas-based graph
        .analyze();

      expect(accessibilityScanResults.violations).toEqual([]);
    });
  });
});
```

---

## Code Quality Standards

### ESLint Configuration

```javascript
// eslint.config.mjs (enhance existing)
import eslint from "@eslint/js";
import tseslint from "typescript-eslint";
import react from "eslint-plugin-react";
import reactHooks from "eslint-plugin-react-hooks";
import a11y from "eslint-plugin-jsx-a11y";

export default tseslint.config(
  eslint.configs.recommended,
  ...tseslint.configs.recommended,
  {
    plugins: {
      react,
      "react-hooks": reactHooks,
      "jsx-a11y": a11y,
    },
    rules: {
      // React rules
      "react/prop-types": "off",
      "react-hooks/rules-of-hooks": "error",
      "react-hooks/exhaustive-deps": "warn",

      // TypeScript rules
      "@typescript-eslint/no-unused-vars": [
        "warn",
        { argsIgnorePattern: "^_" },
      ],
      "@typescript-eslint/explicit-function-return-type": "off",
      "@typescript-eslint/no-explicit-any": "warn",

      // Accessibility rules
      "jsx-a11y/alt-text": "error",
      "jsx-a11y/aria-props": "error",
      "jsx-a11y/click-events-have-key-events": "warn",
      "jsx-a11y/no-static-element-interactions": "warn",
    },
  }
);
```

### Pre-Commit Hooks

```json
// package.json - add scripts
{
  "scripts": {
    "lint": "eslint src --ext .ts,.tsx",
    "lint:fix": "eslint src --ext .ts,.tsx --fix",
    "type-check": "tsc --noEmit",
    "test": "vitest",
    "test:coverage": "vitest run --coverage",
    "test:e2e": "playwright test",
    "prepare": "husky install"
  }
}
```

```bash
# .husky/pre-commit
#!/bin/sh
. "$(dirname "$0")/_/husky.sh"

bun run lint
bun run type-check
bun test --run
```

---

## CI/CD Pipeline

```yaml
# .github/workflows/ci.yml
name: CI

on:
  push:
    branches: [main, develop]
  pull_request:
    branches: [main]

jobs:
  quality:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: oven-sh/setup-bun@v1

      - name: Install dependencies
        run: bun install

      - name: Lint
        run: bun run lint

      - name: Type check
        run: bun run type-check

      - name: Unit tests
        run: bun run test:coverage

      - name: Upload coverage
        uses: codecov/codecov-action@v3
        with:
          files: ./coverage/lcov.info

  e2e:
    runs-on: ubuntu-latest
    needs: quality
    steps:
      - uses: actions/checkout@v4
      - uses: oven-sh/setup-bun@v1

      - name: Install dependencies
        run: bun install

      - name: Install Playwright browsers
        run: bunx playwright install --with-deps

      - name: Build
        run: bun run build

      - name: Run E2E tests
        run: bun run test:e2e

      - name: Upload test results
        if: failure()
        uses: actions/upload-artifact@v3
        with:
          name: playwright-report
          path: playwright-report/
```

---

## Release Checklist

### Pre-Release

- [ ] All tests passing (unit, integration, E2E)
- [ ] Code coverage ≥ 80%
- [ ] No ESLint errors or warnings
- [ ] TypeScript compiles without errors
- [ ] Accessibility audit passed
- [ ] Performance budget met
- [ ] Documentation updated
- [ ] CHANGELOG updated

### Release

- [ ] Version bumped in package.json
- [ ] Git tag created
- [ ] Build successful
- [ ] Deployed to staging
- [ ] Smoke tests passed on staging
- [ ] Deployed to production
- [ ] Health checks passing

### Post-Release

- [ ] Monitor error rates
- [ ] Monitor performance metrics
- [ ] Collect user feedback
- [ ] Update roadmap if needed

---

## Cross-References

- **Gap Analysis:** [001-gap-analysis.md](./001-gap-analysis.md)
- **UX Improvements:** [004-ux-improvements.md](./004-ux-improvements.md)
- **Performance Strategy:** [005-performance-strategy.md](./005-performance-strategy.md)
- **Success Criteria:** [008-success-criteria.md](./008-success-criteria.md)
