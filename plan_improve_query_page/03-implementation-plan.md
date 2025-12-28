# Markdown Rendering Implementation Plan

**Date:** December 28, 2025  
**Version:** 1.0  
**Status:** Ready for Implementation

---

## Overview

This document provides a step-by-step implementation plan to fix markdown rendering issues and establish a robust testing infrastructure for the EdgeQuake Query Page.

---

## Phase 1: Critical Bug Fixes (Day 1)

### Task 1.1: Fix Task List Detection Bug

**Priority:** P0 - Critical  
**Estimated Time:** 30 minutes  
**File:** `src/components/query/markdown/MarkdownTokens.tsx`

#### Current (Buggy) Code

```tsx
case 'list': {
  const list = token as Tokens.List;
  // ...
  return (
    <Tag className={listStyle} start={list.start || undefined}>
      {list.items.map((item, index) => (
        <li key={index} className="leading-7">
          {item.task !== undefined && (  // ❌ BUG
            <input type="checkbox" ... />
          )}
          ...
        </li>
      ))}
    </Tag>
  );
}
```

#### Fixed Code

```tsx
case 'list': {
  const list = token as Tokens.List;
  const Tag = list.ordered ? 'ol' : 'ul';
  const listStyle = list.ordered
    ? 'list-decimal pl-6 my-3 space-y-1'
    : 'list-disc pl-6 my-3 space-y-1';

  return (
    <Tag className={listStyle} start={list.start || undefined}>
      {list.items.map((item, index) => (
        <li
          key={index}
          className={cn(
            'leading-7',
            item.task && 'flex items-start gap-2 list-none -ml-6'
          )}
        >
          {item.task && (  // ✅ FIX: Check for truthy value
            <input
              type="checkbox"
              checked={item.checked ?? false}
              disabled
              className="mt-1.5 h-4 w-4 rounded border-zinc-600 text-primary focus:ring-primary"
            />
          )}
          <div className={item.task ? 'flex-1' : undefined}>
            <MarkdownTokens
              tokens={item.tokens}
              isStreaming={isStreaming && index === list.items.length - 1}
              onSourceClick={onSourceClick}
            />
          </div>
        </li>
      ))}
    </Tag>
  );
}
```

#### Key Changes

1. Change `item.task !== undefined` to `item.task` (truthy check)
2. Add conditional styling for task items (`flex items-start gap-2`)
3. Use `-ml-6 list-none` to remove bullet/number for task items
4. Wrap content in `<div>` for proper flex layout
5. Add `checked={item.checked ?? false}` for safety

---

### Task 1.2: Add Loose List Handling

**Priority:** P1 - High  
**Estimated Time:** 20 minutes  
**File:** `src/components/query/markdown/MarkdownTokens.tsx`

The `loose` property indicates whether list items should be wrapped in paragraphs.

```tsx
case 'list': {
  const list = token as Tokens.List;
  // ... existing code ...

  return (
    <Tag className={listStyle} start={list.start || undefined}>
      {list.items.map((item, index) => (
        <li
          key={index}
          className={cn(
            item.loose ? 'leading-loose' : 'leading-7',
            item.task && 'flex items-start gap-2 list-none -ml-6'
          )}
        >
          {item.task && (
            <input type="checkbox" ... />
          )}
          <div className={item.task ? 'flex-1' : undefined}>
            {/* Use item.loose to determine rendering style */}
            <MarkdownTokens
              tokens={item.tokens}
              isStreaming={isStreaming && index === list.items.length - 1}
              onSourceClick={onSourceClick}
            />
          </div>
        </li>
      ))}
    </Tag>
  );
}
```

---

## Phase 2: Test Infrastructure (Day 1-2)

### Task 2.1: Set Up Test Structure

**Estimated Time:** 1 hour

Create the test directory structure:

```bash
mkdir -p edgequake_webui/src/components/query/markdown/__tests__/fixtures
mkdir -p edgequake_webui/src/components/query/markdown/__tests__/utils
```

### Task 2.2: Create Test Fixtures

**File:** `src/components/query/markdown/__tests__/fixtures/markdown-samples.ts`

```typescript
export const MARKDOWN_SAMPLES = {
  // Regular lists (should NOT have checkboxes)
  unorderedList: `- Item 1\n- Item 2\n- Item 3`,
  orderedList: `1. First\n2. Second\n3. Third`,
  nestedList: `- Parent\n  - Child\n    - Grandchild`,

  // Task lists (SHOULD have checkboxes)
  taskList: `- [ ] Todo\n- [x] Done`,
  mixedTaskList: `- Regular\n- [ ] Task\n- Another regular`,

  // Edge cases
  emptyList: ``,
  singleItem: `- Only one`,
  deepNesting: `- L1\n  - L2\n    - L3\n      - L4`,
};

export const MALICIOUS_INPUTS = {
  scriptTag: `<script>alert('xss')</script>`,
  onclickHandler: `<div onclick="alert('xss')">Click</div>`,
  imgOnerror: `<img src="x" onerror="alert('xss')">`,
  iframeInjection: `<iframe src="javascript:alert('xss')"></iframe>`,
  styleExpression: `<div style="background:url(javascript:alert('xss'))">`,
};
```

### Task 2.3: Create Unit Tests

**File:** `src/components/query/markdown/__tests__/MarkdownTokens.test.tsx`

```typescript
import { describe, it, expect, beforeAll } from "vitest";
import { render, screen } from "@testing-library/react";
import { marked } from "marked";
import { MarkdownTokens } from "../MarkdownTokens";
import { configureMarked } from "../utils/configure-marked";
import {
  MARKDOWN_SAMPLES,
  MALICIOUS_INPUTS,
} from "./fixtures/markdown-samples";

beforeAll(() => {
  configureMarked();
});

describe("MarkdownTokens - List Rendering", () => {
  it("renders unordered list without checkboxes", () => {
    const tokens = marked.lexer(MARKDOWN_SAMPLES.unorderedList);
    render(<MarkdownTokens tokens={tokens} />);

    expect(screen.queryAllByRole("checkbox")).toHaveLength(0);
    expect(screen.getByText("Item 1")).toBeInTheDocument();
  });

  it("renders ordered list without checkboxes", () => {
    const tokens = marked.lexer(MARKDOWN_SAMPLES.orderedList);
    render(<MarkdownTokens tokens={tokens} />);

    expect(screen.queryAllByRole("checkbox")).toHaveLength(0);
  });

  it("renders task list with checkboxes", () => {
    const tokens = marked.lexer(MARKDOWN_SAMPLES.taskList);
    render(<MarkdownTokens tokens={tokens} />);

    const checkboxes = screen.getAllByRole("checkbox");
    expect(checkboxes).toHaveLength(2);
    expect(checkboxes[0]).not.toBeChecked();
    expect(checkboxes[1]).toBeChecked();
  });

  it("renders mixed list with correct checkbox count", () => {
    const tokens = marked.lexer(MARKDOWN_SAMPLES.mixedTaskList);
    render(<MarkdownTokens tokens={tokens} />);

    // Only 1 checkbox for the task item
    expect(screen.getAllByRole("checkbox")).toHaveLength(1);
    // But 3 list items total
    expect(screen.getAllByRole("listitem")).toHaveLength(3);
  });
});

describe("MarkdownTokens - Security", () => {
  it.each(Object.entries(MALICIOUS_INPUTS))("sanitizes %s", (name, input) => {
    const tokens = marked.lexer(input);
    const { container } = render(<MarkdownTokens tokens={tokens} />);

    // No script elements
    expect(container.querySelector("script")).toBeNull();
    // No event handlers
    expect(container.querySelector("[onclick]")).toBeNull();
    expect(container.querySelector("[onerror]")).toBeNull();
  });
});
```

### Task 2.4: Configure Vitest

**File:** `vitest.config.ts` (update if needed)

```typescript
import { defineConfig } from "vitest/config";
import react from "@vitejs/plugin-react";
import path from "path";

export default defineConfig({
  plugins: [react()],
  test: {
    environment: "jsdom",
    setupFiles: ["./src/test/setup.ts"],
    globals: true,
    coverage: {
      reporter: ["text", "html"],
      include: ["src/components/query/markdown/**"],
      exclude: ["**/__tests__/**", "**/*.d.ts"],
    },
  },
  resolve: {
    alias: {
      "@": path.resolve(__dirname, "./src"),
    },
  },
});
```

---

## Phase 3: E2E Test Suite (Day 2-3)

### Task 3.1: Create E2E Test File

**File:** `e2e/markdown-rendering.spec.ts`

```typescript
import { test, expect } from "@playwright/test";

test.describe("Markdown Rendering - Lists", () => {
  test("regular list has no checkboxes", async ({ page }) => {
    // Navigate and trigger a list response
    await page.goto("/query");

    // Mock API response with regular list
    await page.route("**/api/query/stream", async (route) => {
      await route.fulfill({
        body: 'data: {"content":"- Item 1\\n- Item 2\\n- Item 3"}\n\n',
        headers: { "Content-Type": "text/event-stream" },
      });
    });

    await page.getByTestId("query-input").fill("List items");
    await page.getByTestId("send-button").click();

    await page.waitForSelector('[data-testid="message-content"]');

    const checkboxes = page.locator('input[type="checkbox"]');
    await expect(checkboxes).toHaveCount(0);
  });

  test("task list has correct checkboxes", async ({ page }) => {
    await page.goto("/query");

    await page.route("**/api/query/stream", async (route) => {
      await route.fulfill({
        body: 'data: {"content":"- [ ] Task 1\\n- [x] Task 2"}\n\n',
        headers: { "Content-Type": "text/event-stream" },
      });
    });

    await page.getByTestId("query-input").fill("Show tasks");
    await page.getByTestId("send-button").click();

    await page.waitForSelector('[data-testid="message-content"]');

    const checkboxes = page.locator('input[type="checkbox"]');
    await expect(checkboxes).toHaveCount(2);
  });
});
```

### Task 3.2: Create Visual Test Page

**File:** `src/app/dev/markdown-test/page.tsx`

```tsx
"use client";

import { marked } from "marked";
import { MarkdownTokens } from "@/components/query/markdown/MarkdownTokens";
import { configureMarked } from "@/components/query/markdown/utils/configure-marked";
import { useEffect, useState } from "react";

const SAMPLES = {
  lists: `
## Regular Lists

### Unordered
- Item 1
- Item 2
- Item 3

### Ordered
1. First
2. Second
3. Third

### Task List
- [ ] Unchecked task
- [x] Checked task
- [ ] Another task

### Nested
- Parent
  - Child 1
  - Child 2
    - Grandchild
`,
  tables: `
## Tables

| Feature | Status | Notes |
|---------|--------|-------|
| Lists   | ✅     | Working |
| Tables  | ✅     | Working |
| Math    | ✅     | Working |
`,
  alerts: `
## GitHub Alerts

> [!NOTE]
> This is a note.

> [!TIP]
> This is a tip.

> [!WARNING]
> This is a warning.

> [!CAUTION]
> This is a caution.

> [!IMPORTANT]
> This is important.
`,
};

export default function MarkdownTestPage() {
  const [tokens, setTokens] = useState<Record<string, any[]>>({});

  useEffect(() => {
    configureMarked();
    setTokens({
      lists: marked.lexer(SAMPLES.lists),
      tables: marked.lexer(SAMPLES.tables),
      alerts: marked.lexer(SAMPLES.alerts),
    });
  }, []);

  return (
    <div className="container mx-auto py-8 space-y-12">
      <h1 className="text-3xl font-bold">Markdown Test Page</h1>

      <section id="list-sample">
        <h2 className="text-xl font-semibold mb-4">Lists</h2>
        <div className="border rounded-lg p-4 bg-card">
          {tokens.lists && <MarkdownTokens tokens={tokens.lists} />}
        </div>
      </section>

      <section id="table-sample">
        <h2 className="text-xl font-semibold mb-4">Tables</h2>
        <div className="border rounded-lg p-4 bg-card">
          {tokens.tables && <MarkdownTokens tokens={tokens.tables} />}
        </div>
      </section>

      <section id="alerts-sample">
        <h2 className="text-xl font-semibold mb-4">Alerts</h2>
        <div className="border rounded-lg p-4 bg-card">
          {tokens.alerts && <MarkdownTokens tokens={tokens.alerts} />}
        </div>
      </section>
    </div>
  );
}
```

---

## Phase 4: Enhanced Features (Day 3-4)

### Task 4.1: Interactive Task Checkboxes (Optional)

Add callback support for task list interaction:

```tsx
interface MarkdownTokensProps {
  tokens: Token[];
  isStreaming?: boolean;
  className?: string;
  onSourceClick?: (sourceId: string) => void;
  onTaskClick?: (task: TaskClickEvent) => void; // New
}

interface TaskClickEvent {
  listIndex: number;
  itemIndex: number;
  checked: boolean;
  raw: string;
}
```

### Task 4.2: Improve List Styling

**File:** `src/app/globals.css`

```css
/* Task list styling */
.markdown-content li.task-item {
  list-style: none;
  margin-left: -1.5rem;
}

.markdown-content li.task-item input[type="checkbox"] {
  @apply h-4 w-4 rounded border-zinc-600;
  @apply text-primary focus:ring-primary focus:ring-offset-0;
  margin-top: 0.375rem;
}

.markdown-content li.task-item.checked {
  @apply text-muted-foreground line-through;
}

/* Nested list indentation */
.markdown-content ul ul,
.markdown-content ol ol,
.markdown-content ul ol,
.markdown-content ol ul {
  @apply ml-4 mt-1;
}
```

---

## Phase 5: CI/CD Integration (Day 4)

### Task 5.1: Add Test Commands to package.json

```json
{
  "scripts": {
    "test": "vitest run",
    "test:watch": "vitest",
    "test:coverage": "vitest run --coverage",
    "test:markdown": "vitest run src/components/query/markdown",
    "test:e2e": "playwright test",
    "test:e2e:markdown": "playwright test e2e/markdown"
  }
}
```

### Task 5.2: Create GitHub Actions Workflow

**File:** `.github/workflows/test-markdown.yml`

```yaml
name: Markdown Tests

on:
  push:
    paths:
      - "edgequake_webui/src/components/query/markdown/**"
      - "edgequake_webui/e2e/markdown*.spec.ts"
  pull_request:
    paths:
      - "edgequake_webui/src/components/query/markdown/**"
      - "edgequake_webui/e2e/markdown*.spec.ts"

jobs:
  unit-tests:
    runs-on: ubuntu-latest
    defaults:
      run:
        working-directory: edgequake_webui
    steps:
      - uses: actions/checkout@v4
      - uses: pnpm/action-setup@v2
        with:
          version: 9
      - uses: actions/setup-node@v4
        with:
          node-version: 20
          cache: "pnpm"
          cache-dependency-path: edgequake_webui/pnpm-lock.yaml
      - run: pnpm install
      - run: pnpm test:markdown

  e2e-tests:
    runs-on: ubuntu-latest
    defaults:
      run:
        working-directory: edgequake_webui
    steps:
      - uses: actions/checkout@v4
      - uses: pnpm/action-setup@v2
        with:
          version: 9
      - uses: actions/setup-node@v4
        with:
          node-version: 20
          cache: "pnpm"
          cache-dependency-path: edgequake_webui/pnpm-lock.yaml
      - run: pnpm install
      - run: pnpm exec playwright install --with-deps
      - run: pnpm build
      - run: pnpm test:e2e:markdown
```

---

## Implementation Timeline

| Day    | Phase   | Tasks                      | Deliverables              |
| ------ | ------- | -------------------------- | ------------------------- |
| 1 (AM) | Phase 1 | Fix task list bug          | Working list rendering    |
| 1 (PM) | Phase 2 | Set up test infrastructure | Test structure + fixtures |
| 2 (AM) | Phase 2 | Create unit tests          | 90% coverage              |
| 2 (PM) | Phase 3 | Create E2E tests           | E2E test suite            |
| 3 (AM) | Phase 3 | Visual test page           | Dev test page             |
| 3 (PM) | Phase 4 | Enhanced features          | Task interactivity        |
| 4      | Phase 5 | CI/CD integration          | Automated testing         |

---

## Verification Checklist

### After Phase 1

- [ ] Lists without task syntax have NO checkboxes
- [ ] Task lists (`- [ ]`, `- [x]`) have checkboxes
- [ ] Nested lists render correctly
- [ ] Mixed lists (some tasks, some not) work correctly
- [ ] `pnpm typecheck` passes
- [ ] `pnpm build` succeeds

### After Phase 2-3

- [ ] All unit tests pass
- [ ] Coverage > 90% for MarkdownTokens
- [ ] E2E tests pass
- [ ] Visual test page works at `/dev/markdown-test`

### After Phase 4-5

- [ ] CI pipeline runs on PR
- [ ] Tests run on markdown file changes
- [ ] Visual regression baseline established

---

## Rollback Plan

If issues are discovered after deployment:

1. Revert to previous commit: `git revert HEAD`
2. The critical bug fix is isolated to one line change
3. All changes are in `src/components/query/markdown/`
4. No backend changes required

---

## Success Metrics

| Metric             | Target | Current |
| ------------------ | ------ | ------- |
| List checkbox bug  | Fixed  | Broken  |
| Unit test coverage | > 90%  | 0%      |
| E2E test coverage  | > 80%  | 0%      |
| Build time         | < 30s  | ~5s     |
| Test run time      | < 60s  | N/A     |
