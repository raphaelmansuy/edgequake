# Markdown Rendering Testing Methodology

**Date:** December 28, 2025  
**Version:** 1.0  
**Purpose:** Define a comprehensive testing strategy to ensure perfect markdown rendering

---

## Testing Philosophy

> "If it's not tested, it's broken."

The goal is to create a multi-layered testing strategy that catches rendering issues at every level:

1. **Unit Tests** - Test individual token rendering
2. **Integration Tests** - Test the full rendering pipeline
3. **Visual Regression Tests** - Catch visual differences
4. **E2E Tests** - Test real user scenarios
5. **Snapshot Tests** - Catch unexpected output changes

---

## Test Categories

### Category 1: Block-Level Token Tests

| Token Type     | Test Cases                                          | Priority     |
| -------------- | --------------------------------------------------- | ------------ |
| `heading`      | h1-h6, with inline formatting                       | High         |
| `paragraph`    | Simple, with inline tokens                          | High         |
| `code`         | Single-line, multi-line, various languages, mermaid | High         |
| `list`         | Ordered, unordered, nested, task lists              | **Critical** |
| `table`        | Simple, with alignment, streaming                   | High         |
| `blockquote`   | Simple, nested, with alerts                         | High         |
| `hr`           | Horizontal rule                                     | Low          |
| `html`         | Safe HTML, sanitized                                | Medium       |
| `math_block`   | LaTeX equations                                     | Medium       |
| `github_alert` | NOTE, TIP, WARNING, CAUTION, IMPORTANT              | Medium       |
| `details`      | Collapsible blocks                                  | Medium       |

### Category 2: Inline Token Tests

| Token Type    | Test Cases                     | Priority |
| ------------- | ------------------------------ | -------- |
| `text`        | Plain text, with nested tokens | High     |
| `strong`      | Bold text                      | High     |
| `em`          | Italic text                    | High     |
| `del`         | Strikethrough                  | Medium   |
| `codespan`    | Inline code                    | High     |
| `link`        | External, internal, with title | High     |
| `image`       | With alt text, lazy loading    | Medium   |
| `br`          | Line breaks                    | Low      |
| `math_inline` | Inline LaTeX                   | Medium   |
| `citation`    | Source references              | Medium   |
| `escape`      | Escaped characters             | Low      |

### Category 3: Edge Cases

| Scenario           | Description                          | Priority     |
| ------------------ | ------------------------------------ | ------------ |
| Empty content      | Empty string, whitespace only        | High         |
| Malformed markdown | Unclosed blocks, invalid syntax      | High         |
| XSS attempts       | Script injection, event handlers     | **Critical** |
| Unicode            | Emoji, CJK characters, RTL text      | Medium       |
| Large content      | 10KB+, 100KB+ documents              | Medium       |
| Streaming          | Incomplete tokens during stream      | High         |
| Nested structures  | Lists in blockquotes, code in tables | High         |

---

## Test Implementation

### 1. Unit Tests with Vitest

**File:** `src/components/query/markdown/__tests__/MarkdownTokens.test.tsx`

```tsx
import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import { marked } from "marked";
import { MarkdownTokens } from "../MarkdownTokens";
import { configureMarked } from "../utils/configure-marked";

// Configure marked before tests
configureMarked();

describe("MarkdownTokens", () => {
  describe("List Rendering", () => {
    it("renders unordered list without checkboxes", () => {
      const markdown = `
- Item 1
- Item 2
- Item 3
`;
      const tokens = marked.lexer(markdown);
      render(<MarkdownTokens tokens={tokens} />);

      // Should NOT have checkboxes
      expect(screen.queryByRole("checkbox")).toBeNull();

      // Should have list items
      expect(screen.getByText("Item 1")).toBeInTheDocument();
      expect(screen.getByText("Item 2")).toBeInTheDocument();
      expect(screen.getByText("Item 3")).toBeInTheDocument();
    });

    it("renders ordered list without checkboxes", () => {
      const markdown = `
1. First
2. Second
3. Third
`;
      const tokens = marked.lexer(markdown);
      render(<MarkdownTokens tokens={tokens} />);

      expect(screen.queryByRole("checkbox")).toBeNull();
    });

    it("renders task list WITH checkboxes", () => {
      const markdown = `
- [ ] Unchecked task
- [x] Checked task
`;
      const tokens = marked.lexer(markdown);
      render(<MarkdownTokens tokens={tokens} />);

      const checkboxes = screen.getAllByRole("checkbox");
      expect(checkboxes).toHaveLength(2);
      expect(checkboxes[0]).not.toBeChecked();
      expect(checkboxes[1]).toBeChecked();
    });

    it("renders nested lists correctly", () => {
      const markdown = `
- Parent 1
  - Child 1.1
  - Child 1.2
- Parent 2
`;
      const tokens = marked.lexer(markdown);
      render(<MarkdownTokens tokens={tokens} />);

      expect(screen.queryByRole("checkbox")).toBeNull();
      expect(screen.getByText("Parent 1")).toBeInTheDocument();
      expect(screen.getByText("Child 1.1")).toBeInTheDocument();
    });

    it("renders mixed list (some tasks, some not)", () => {
      const markdown = `
- Regular item
- [ ] Task item
- Another regular
`;
      const tokens = marked.lexer(markdown);
      render(<MarkdownTokens tokens={tokens} />);

      // Only one checkbox for the task item
      const checkboxes = screen.getAllByRole("checkbox");
      expect(checkboxes).toHaveLength(1);
    });
  });

  describe("GitHub Alerts", () => {
    it.each([
      ["NOTE", "note"],
      ["TIP", "tip"],
      ["WARNING", "warning"],
      ["CAUTION", "caution"],
      ["IMPORTANT", "important"],
    ])("renders [!%s] alert correctly", (type, expected) => {
      const markdown = `> [!${type}]
> This is a ${type.toLowerCase()} alert.
`;
      const tokens = marked.lexer(markdown);
      render(<MarkdownTokens tokens={tokens} />);

      expect(
        screen.getByText(`This is a ${type.toLowerCase()} alert.`)
      ).toBeInTheDocument();
    });
  });

  describe("XSS Prevention", () => {
    it("sanitizes script tags in HTML tokens", () => {
      const markdown = `<script>alert('xss')</script>`;
      const tokens = marked.lexer(markdown);
      render(<MarkdownTokens tokens={tokens} />);

      // Script should not execute
      expect(document.querySelector("script")).toBeNull();
    });

    it("sanitizes onclick handlers", () => {
      const markdown = `<div onclick="alert('xss')">Click me</div>`;
      const tokens = marked.lexer(markdown);
      const { container } = render(<MarkdownTokens tokens={tokens} />);

      const div = container.querySelector("div");
      expect(div?.getAttribute("onclick")).toBeNull();
    });
  });
});
```

### 2. Snapshot Tests

**File:** `src/components/query/markdown/__tests__/MarkdownTokens.snapshot.test.tsx`

````tsx
import { describe, it, expect } from "vitest";
import { render } from "@testing-library/react";
import { marked } from "marked";
import { MarkdownTokens } from "../MarkdownTokens";

describe("MarkdownTokens Snapshots", () => {
  it.each([
    ["simple paragraph", "Hello, world!"],
    ["bold text", "**bold**"],
    ["italic text", "*italic*"],
    ["ordered list", "1. One\n2. Two\n3. Three"],
    ["unordered list", "- A\n- B\n- C"],
    ["task list", "- [ ] Todo\n- [x] Done"],
    ["code block", '```js\nconsole.log("hi");\n```'],
    ["table", "| A | B |\n|---|---|\n| 1 | 2 |"],
    ["blockquote", "> Quote"],
    ["github note", "> [!NOTE]\n> This is a note"],
    ["heading h1", "# Heading 1"],
    ["heading h2", "## Heading 2"],
    ["link", "[Link](https://example.com)"],
    ["image", "![Alt](image.png)"],
    ["inline code", "`code`"],
    ["math block", "$$E = mc^2$$"],
    ["math inline", "$x^2$"],
  ])("renders %s correctly", (name, markdown) => {
    const tokens = marked.lexer(markdown);
    const { container } = render(<MarkdownTokens tokens={tokens} />);
    expect(container).toMatchSnapshot();
  });
});
````

### 3. E2E Tests with Playwright

**File:** `e2e/markdown-rendering.spec.ts`

```typescript
import { test, expect } from "@playwright/test";

test.describe("Markdown Rendering", () => {
  test.beforeEach(async ({ page }) => {
    // Navigate to query page
    await page.goto("/query");
  });

  test("renders list without spurious checkboxes", async ({ page }) => {
    // Send a query that returns a list
    await page
      .getByPlaceholder(/ask a question/i)
      .fill("List the main products");
    await page.getByRole("button", { name: /send/i }).click();

    // Wait for response
    await page.waitForSelector('[data-testid="assistant-message"]');

    // Get the message content
    const message = page.locator('[data-testid="assistant-message"]').first();

    // Should have list items
    await expect(message.locator("li")).toHaveCount({ min: 1 });

    // Should NOT have checkboxes (unless it's actually a task list)
    const checkboxes = message.locator('input[type="checkbox"]');
    await expect(checkboxes).toHaveCount(0);
  });

  test("renders task list with checkboxes", async ({ page }) => {
    // This test assumes we have a document with task lists
    // Or we can mock the response

    // Mock response with task list
    await page.route("**/api/query", async (route) => {
      await route.fulfill({
        json: {
          answer: "- [ ] Task 1\n- [x] Task 2 (done)",
          sources: [],
        },
      });
    });

    await page.getByPlaceholder(/ask a question/i).fill("Show me tasks");
    await page.getByRole("button", { name: /send/i }).click();

    await page.waitForSelector('[data-testid="assistant-message"]');

    const checkboxes = page.locator('input[type="checkbox"]');
    await expect(checkboxes).toHaveCount(2);

    // First checkbox unchecked, second checked
    await expect(checkboxes.nth(0)).not.toBeChecked();
    await expect(checkboxes.nth(1)).toBeChecked();
  });

  test("renders GitHub alerts correctly", async ({ page }) => {
    await page.route("**/api/query", async (route) => {
      await route.fulfill({
        json: {
          answer: "> [!NOTE]\n> This is important information.",
          sources: [],
        },
      });
    });

    await page.getByPlaceholder(/ask a question/i).fill("Show me a note");
    await page.getByRole("button", { name: /send/i }).click();

    await page.waitForSelector('[data-testid="assistant-message"]');

    // Should have alert styling, not regular blockquote
    const alert = page.locator('[class*="alert"]');
    await expect(alert).toBeVisible();
  });

  test("renders tables without flickering during streaming", async ({
    page,
  }) => {
    // This is a visual test - we'll check that TableSkeleton appears
    await page.route("**/api/query/stream", async (route) => {
      // Simulate streaming response with incomplete table
      const encoder = new TextEncoder();
      const stream = new ReadableStream({
        async start(controller) {
          controller.enqueue(encoder.encode("| Column 1 | Column 2 |\n"));
          await new Promise((r) => setTimeout(r, 100));
          controller.enqueue(encoder.encode("|----------|----------|\n"));
          await new Promise((r) => setTimeout(r, 100));
          controller.enqueue(encoder.encode("| Data 1   | Data 2   |\n"));
          controller.close();
        },
      });

      await route.fulfill({
        body: stream,
        headers: { "Content-Type": "text/event-stream" },
      });
    });

    await page.getByPlaceholder(/ask a question/i).fill("Show table");
    await page.getByRole("button", { name: /send/i }).click();

    // During streaming, should show skeleton or buffer table
    // After completion, should show full table
    await page.waitForSelector("table");
    expect(await page.locator("table").count()).toBe(1);
  });

  test("sanitizes malicious HTML", async ({ page }) => {
    await page.route("**/api/query", async (route) => {
      await route.fulfill({
        json: {
          answer: "<script>window.xssExecuted=true</script>Test",
          sources: [],
        },
      });
    });

    await page.getByPlaceholder(/ask a question/i).fill("Test XSS");
    await page.getByRole("button", { name: /send/i }).click();

    await page.waitForSelector('[data-testid="assistant-message"]');

    // XSS should not execute
    const xssExecuted = await page.evaluate(() => (window as any).xssExecuted);
    expect(xssExecuted).toBeUndefined();
  });
});
```

### 4. Visual Regression Tests

**File:** `e2e/markdown-visual.spec.ts`

```typescript
import { test, expect } from "@playwright/test";

test.describe("Markdown Visual Regression", () => {
  test("list rendering matches expected appearance", async ({ page }) => {
    // Render a test page with markdown samples
    await page.goto("/dev/markdown-test"); // Need to create this page

    // Take screenshot
    await expect(page.locator("#list-sample")).toHaveScreenshot(
      "list-sample.png"
    );
  });

  test("table rendering matches expected appearance", async ({ page }) => {
    await page.goto("/dev/markdown-test");
    await expect(page.locator("#table-sample")).toHaveScreenshot(
      "table-sample.png"
    );
  });

  test("github alerts rendering matches expected appearance", async ({
    page,
  }) => {
    await page.goto("/dev/markdown-test");
    await expect(page.locator("#alerts-sample")).toHaveScreenshot(
      "alerts-sample.png"
    );
  });
});
```

---

## Test Data Fixtures

### Fixture 1: Standard Markdown Samples

**File:** `src/components/query/markdown/__tests__/fixtures/markdown-samples.ts`

```typescript
export const MARKDOWN_SAMPLES = {
  // Lists
  unorderedList: `
- Item 1
- Item 2
- Item 3
`,
  orderedList: `
1. First
2. Second
3. Third
`,
  taskList: `
- [ ] Unchecked
- [x] Checked
`,
  nestedList: `
- Parent 1
  - Child 1.1
  - Child 1.2
    - Grandchild 1.2.1
- Parent 2
`,
  mixedList: `
1. Ordered
   - Nested unordered
   - Another
2. Continue ordered
`,

  // Tables
  simpleTable: `
| Name | Age |
|------|-----|
| Alice | 30 |
| Bob | 25 |
`,
  alignedTable: `
| Left | Center | Right |
|:-----|:------:|------:|
| L    |   C    |     R |
`,

  // Alerts
  noteAlert: `> [!NOTE]
> This is a note.
`,
  warningAlert: `> [!WARNING]
> Be careful!
`,

  // Code
  inlineCode: "Use `console.log()` for debugging.",
  codeBlock: `
\`\`\`javascript
function hello() {
  console.log('Hello!');
}
\`\`\`
`,

  // Math
  inlineMath: "The formula is $E = mc^2$.",
  blockMath: `
$$
\\int_0^\\infty e^{-x^2} dx = \\frac{\\sqrt{\\pi}}{2}
$$
`,

  // Complex
  complexDocument: `
# Heading 1

This is a paragraph with **bold** and *italic* text.

## Heading 2

> [!NOTE]
> Important information here.

### List Example

1. First item
2. Second item
   - Nested bullet
   - Another bullet
3. Third item

### Code Example

\`\`\`python
def greet(name):
    print(f"Hello, {name}!")
\`\`\`

### Table Example

| Feature | Status |
|---------|--------|
| Lists   | ✅     |
| Tables  | ✅     |
| Math    | ✅     |

---

*End of document*
`,
};
```

---

## CI/CD Integration

### GitHub Actions Workflow

**File:** `.github/workflows/markdown-tests.yml`

```yaml
name: Markdown Rendering Tests

on:
  push:
    paths:
      - "edgequake_webui/src/components/query/markdown/**"
  pull_request:
    paths:
      - "edgequake_webui/src/components/query/markdown/**"

jobs:
  unit-tests:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: oven-sh/setup-bun@v1
      - run: cd edgequake_webui && bun install
      - run: cd edgequake_webui && bun test src/components/query/markdown

  e2e-tests:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: oven-sh/setup-bun@v1
      - run: cd edgequake_webui && bun install
      - run: cd edgequake_webui && bunx playwright install --with-deps
      - run: cd edgequake_webui && bun run build
      - run: cd edgequake_webui && bunx playwright test e2e/markdown

  visual-regression:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: oven-sh/setup-bun@v1
      - run: cd edgequake_webui && bun install
      - run: cd edgequake_webui && bunx playwright install --with-deps
      - run: cd edgequake_webui && bun run build
      - run: cd edgequake_webui && bunx playwright test e2e/markdown-visual
      - uses: actions/upload-artifact@v4
        if: failure()
        with:
          name: visual-diff
          path: edgequake_webui/test-results/
```

---

## Test Coverage Requirements

### Minimum Coverage Targets

| Component                     | Line Coverage | Branch Coverage |
| ----------------------------- | ------------- | --------------- |
| MarkdownTokens.tsx            | 90%           | 85%             |
| MarkdownInlineTokens.tsx      | 90%           | 85%             |
| StreamingMarkdownRenderer.tsx | 85%           | 80%             |
| configure-marked.ts           | 80%           | 75%             |
| sanitize-html.ts              | 95%           | 90%             |

### Running Coverage Report

```bash
cd edgequake_webui
bun test --coverage src/components/query/markdown
```

---

## Test Documentation

### Test Naming Convention

```
[Component]_[Scenario]_[ExpectedBehavior]
```

Examples:

- `MarkdownTokens_UnorderedList_RendersWithoutCheckboxes`
- `MarkdownTokens_TaskList_RendersWithCheckboxes`
- `MarkdownTokens_XSSAttempt_SanitizesInput`

### Test Organization

```
src/components/query/markdown/
├── __tests__/
│   ├── fixtures/
│   │   ├── markdown-samples.ts
│   │   └── malicious-inputs.ts
│   ├── MarkdownTokens.test.tsx
│   ├── MarkdownTokens.snapshot.test.tsx
│   ├── MarkdownInlineTokens.test.tsx
│   ├── StreamingMarkdownRenderer.test.tsx
│   └── utils/
│       ├── configure-marked.test.ts
│       └── sanitize-html.test.ts
e2e/
├── markdown-rendering.spec.ts
├── markdown-visual.spec.ts
└── markdown-streaming.spec.ts
```

---

## Next Steps

1. Implement the unit test file structure
2. Create the E2E test suite
3. Set up visual regression baseline
4. Add CI/CD workflow
5. Document test maintenance procedures
