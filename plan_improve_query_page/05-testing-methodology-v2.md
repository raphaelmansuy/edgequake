# Markdown Testing Methodology (Updated)

**Date:** December 28, 2025  
**Version:** 2.0  
**Purpose:** Comprehensive testing strategy for perfect markdown rendering

---

## Testing Strategy Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                    Testing Pyramid                               │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│                    ┌─────────────────┐                           │
│                    │   E2E Tests     │  (10%) - Full user flows  │
│                    └────────┬────────┘                           │
│               ┌─────────────┴─────────────┐                      │
│               │   Integration Tests       │  (20%) - Component   │
│               └─────────────┬─────────────┘        combinations  │
│          ┌──────────────────┴──────────────────┐                 │
│          │        Unit Tests                   │  (70%) - Logic  │
│          └─────────────────────────────────────┘                 │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

---

## Unit Test Categories

### Category 1: Normalization Functions

**File:** `__tests__/utils/normalize-markdown.test.ts`

````typescript
import { describe, it, expect } from "vitest";
import { normalizeMarkdownForStreaming } from "../normalize-markdown";

describe("normalizeMarkdownForStreaming", () => {
  describe("Bold normalization", () => {
    it.each([
      // [input, expected, description]
      ["**text**", "**text**", "standard bold unchanged"],
      ["**text **", "**text**", "trailing space removed"],
      ["** text**", "**text**", "leading space removed"],
      ["** text **", "**text**", "both spaces removed"],
      ["**Products **:", "**Products**:", "real LLM case"],
      ["Hello **world ** there", "Hello **world** there", "mid-sentence"],
      ["**multi word text **", "**multi word text**", "multi-word trailing"],
      ["** multi word text**", "**multi word text**", "multi-word leading"],
    ])('normalizes "%s" to "%s" (%s)', (input, expected, _desc) => {
      expect(normalizeMarkdownForStreaming(input)).toBe(expected);
    });

    it("does not modify content inside code blocks", () => {
      const input = "```\n** text **\n```";
      expect(normalizeMarkdownForStreaming(input)).toBe(input);
    });

    it("handles multiple bold sections", () => {
      const input = "**first ** and **second **";
      expect(normalizeMarkdownForStreaming(input)).toBe(
        "**first** and **second**"
      );
    });
  });

  describe("Italic normalization", () => {
    it.each([
      ["*text*", "*text*", "standard italic unchanged"],
      ["*text *", "*text*", "trailing space removed"],
      ["* text*", "*text*", "leading space removed"],
      ["* text *", "*text*", "both spaces removed"],
    ])('normalizes "%s" to "%s" (%s)', (input, expected, _desc) => {
      expect(normalizeMarkdownForStreaming(input)).toBe(expected);
    });

    it("does not confuse italic with bold", () => {
      const input = "**bold** and *italic *";
      expect(normalizeMarkdownForStreaming(input)).toBe(
        "**bold** and *italic*"
      );
    });
  });

  describe("Underscore variants", () => {
    it.each([
      ["__text__", "__text__", "underscore bold unchanged"],
      ["__text __", "__text__", "underscore bold trailing"],
      ["__ text__", "__text__", "underscore bold leading"],
      ["_text_", "_text_", "underscore italic unchanged"],
      ["_text _", "_text_", "underscore italic trailing"],
      ["_ text_", "_text_", "underscore italic leading"],
    ])('normalizes "%s" to "%s" (%s)', (input, expected, _desc) => {
      expect(normalizeMarkdownForStreaming(input)).toBe(expected);
    });
  });

  describe("Strikethrough normalization", () => {
    it.each([
      ["~~text~~", "~~text~~", "standard strikethrough unchanged"],
      ["~~text ~~", "~~text~~", "trailing space removed"],
      ["~~ text~~", "~~text~~", "leading space removed"],
    ])('normalizes "%s" to "%s" (%s)', (input, expected, _desc) => {
      expect(normalizeMarkdownForStreaming(input)).toBe(expected);
    });
  });

  describe("Edge cases", () => {
    it("handles empty string", () => {
      expect(normalizeMarkdownForStreaming("")).toBe("");
    });

    it("handles null/undefined", () => {
      expect(normalizeMarkdownForStreaming(null as any)).toBe(null);
      expect(normalizeMarkdownForStreaming(undefined as any)).toBe(undefined);
    });

    it("handles nested markdown", () => {
      const input = "**bold with *italic * inside **";
      // Should fix both bold and italic
      expect(normalizeMarkdownForStreaming(input)).toBe(
        "**bold with *italic* inside**"
      );
    });

    it("handles markdown in lists", () => {
      const input = "1. **Products **:\n   - Item 1\n   - Item 2";
      expect(normalizeMarkdownForStreaming(input)).toContain("**Products**:");
    });
  });
});
````

### Category 2: Token Rendering Tests

**File:** `__tests__/MarkdownTokens.test.tsx`

````typescript
import { describe, it, expect, beforeAll } from "vitest";
import { render, screen } from "@testing-library/react";
import { marked } from "marked";
import { MarkdownTokens } from "../MarkdownTokens";
import { configureMarked } from "../utils/configure-marked";

beforeAll(() => {
  configureMarked();
});

describe("MarkdownTokens", () => {
  describe("Block elements", () => {
    it("renders headings with correct hierarchy", () => {
      const markdown = "# H1\n## H2\n### H3";
      const tokens = marked.lexer(markdown);
      render(<MarkdownTokens tokens={tokens} />);

      expect(screen.getByRole("heading", { level: 1 })).toHaveTextContent("H1");
      expect(screen.getByRole("heading", { level: 2 })).toHaveTextContent("H2");
      expect(screen.getByRole("heading", { level: 3 })).toHaveTextContent("H3");
    });

    it("renders paragraphs", () => {
      const markdown = "First paragraph.\n\nSecond paragraph.";
      const tokens = marked.lexer(markdown);
      render(<MarkdownTokens tokens={tokens} />);

      expect(screen.getByText("First paragraph.")).toBeInTheDocument();
      expect(screen.getByText("Second paragraph.")).toBeInTheDocument();
    });

    it("renders code blocks with language", () => {
      const markdown = '```javascript\nconsole.log("hello");\n```';
      const tokens = marked.lexer(markdown);
      render(<MarkdownTokens tokens={tokens} />);

      expect(screen.getByText(/console\.log/)).toBeInTheDocument();
    });

    it("renders blockquotes", () => {
      const markdown = "> This is a quote";
      const tokens = marked.lexer(markdown);
      render(<MarkdownTokens tokens={tokens} />);

      expect(screen.getByText("This is a quote")).toBeInTheDocument();
    });

    it("renders GitHub alerts", () => {
      const markdown = "> [!NOTE]\n> Important note here";
      const tokens = marked.lexer(markdown);
      render(<MarkdownTokens tokens={tokens} />);

      expect(screen.getByText("Important note here")).toBeInTheDocument();
    });
  });

  describe("List elements", () => {
    it("renders unordered list without checkboxes", () => {
      const markdown = "- Item 1\n- Item 2\n- Item 3";
      const tokens = marked.lexer(markdown);
      render(<MarkdownTokens tokens={tokens} />);

      expect(screen.queryAllByRole("checkbox")).toHaveLength(0);
      expect(screen.getByText("Item 1")).toBeInTheDocument();
    });

    it("renders ordered list with correct numbering", () => {
      const markdown = "1. First\n2. Second\n3. Third";
      const tokens = marked.lexer(markdown);
      render(<MarkdownTokens tokens={tokens} />);

      expect(screen.queryAllByRole("checkbox")).toHaveLength(0);
      const list = document.querySelector("ol");
      expect(list).toHaveAttribute("start", "1");
    });

    it("renders task list with checkboxes", () => {
      const markdown = "- [ ] Unchecked\n- [x] Checked";
      const tokens = marked.lexer(markdown);
      render(<MarkdownTokens tokens={tokens} />);

      const checkboxes = screen.getAllByRole("checkbox");
      expect(checkboxes).toHaveLength(2);
      expect(checkboxes[0]).not.toBeChecked();
      expect(checkboxes[1]).toBeChecked();
    });

    it("renders nested lists correctly", () => {
      const markdown = "- Parent\n  - Child 1\n  - Child 2";
      const tokens = marked.lexer(markdown);
      render(<MarkdownTokens tokens={tokens} />);

      expect(screen.getByText("Parent")).toBeInTheDocument();
      expect(screen.getByText("Child 1")).toBeInTheDocument();
    });

    it("renders list with inline formatting", () => {
      // After normalization, this should work
      const markdown = "- **Bold item**\n- *Italic item*";
      const tokens = marked.lexer(markdown);
      render(<MarkdownTokens tokens={tokens} />);

      expect(document.querySelector("strong")).toBeInTheDocument();
      expect(document.querySelector("em")).toBeInTheDocument();
    });
  });

  describe("Table elements", () => {
    it("renders tables with headers", () => {
      const markdown = "| Name | Age |\n|------|-----|\n| Alice | 30 |";
      const tokens = marked.lexer(markdown);
      render(<MarkdownTokens tokens={tokens} />);

      expect(screen.getByText("Name")).toBeInTheDocument();
      expect(screen.getByText("Alice")).toBeInTheDocument();
    });
  });
});

describe("MarkdownInlineTokens", () => {
  describe("Inline formatting", () => {
    it("renders bold text", () => {
      const markdown = "This is **bold** text";
      const tokens = marked.lexer(markdown);
      render(<MarkdownTokens tokens={tokens} />);

      const strong = document.querySelector("strong");
      expect(strong).toHaveTextContent("bold");
    });

    it("renders italic text", () => {
      const markdown = "This is *italic* text";
      const tokens = marked.lexer(markdown);
      render(<MarkdownTokens tokens={tokens} />);

      const em = document.querySelector("em");
      expect(em).toHaveTextContent("italic");
    });

    it("renders inline code", () => {
      const markdown = "Use `console.log()` for debugging";
      const tokens = marked.lexer(markdown);
      render(<MarkdownTokens tokens={tokens} />);

      const code = document.querySelector("code");
      expect(code).toHaveTextContent("console.log()");
    });

    it("renders links", () => {
      const markdown = "Visit [Google](https://google.com)";
      const tokens = marked.lexer(markdown);
      render(<MarkdownTokens tokens={tokens} />);

      const link = screen.getByRole("link");
      expect(link).toHaveAttribute("href", "https://google.com");
      expect(link).toHaveTextContent("Google");
    });

    it("renders combined formatting", () => {
      const markdown = "***bold italic*** and ~~strikethrough~~";
      const tokens = marked.lexer(markdown);
      render(<MarkdownTokens tokens={tokens} />);

      expect(document.querySelector("strong")).toBeInTheDocument();
      expect(document.querySelector("em")).toBeInTheDocument();
      expect(document.querySelector("del")).toBeInTheDocument();
    });
  });
});
````

---

## Integration Tests

**File:** `__tests__/StreamingMarkdownRenderer.test.tsx`

```typescript
import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import { StreamingMarkdownRenderer } from "../StreamingMarkdownRenderer";

describe("StreamingMarkdownRenderer", () => {
  describe("Normalization integration", () => {
    it("renders normalized bold text", () => {
      const content = "**Products **:";
      render(<StreamingMarkdownRenderer content={content} />);

      // Should render as bold, not literal asterisks
      expect(document.querySelector("strong")).toHaveTextContent("Products");
      expect(screen.queryByText("**")).not.toBeInTheDocument();
    });

    it("renders normalized italic text", () => {
      const content = "*italic * text";
      render(<StreamingMarkdownRenderer content={content} />);

      expect(document.querySelector("em")).toHaveTextContent("italic");
    });

    it("handles real LLM-style lists", () => {
      const content = `1. **Products **:
   - Code2Doc
   - GraphCodebert
2. **Concepts **:
   - Machine learning`;

      render(<StreamingMarkdownRenderer content={content} />);

      // Bold should be rendered properly
      const strongElements = document.querySelectorAll("strong");
      expect(strongElements.length).toBeGreaterThanOrEqual(2);

      // No literal asterisks
      expect(screen.queryByText(/\*\*/)).not.toBeInTheDocument();
    });
  });

  describe("Streaming mode", () => {
    it("handles incomplete table during streaming", () => {
      const content = "| Header 1 | Header 2 |\n|----------|";
      render(
        <StreamingMarkdownRenderer content={content} isStreaming={true} />
      );

      // Should show skeleton or buffer, not broken table
      expect(document.querySelector("table")).toBeNull();
    });

    it("renders complete content when not streaming", () => {
      const content = "**Bold** and *italic*";
      render(
        <StreamingMarkdownRenderer content={content} isStreaming={false} />
      );

      expect(document.querySelector("strong")).toBeInTheDocument();
      expect(document.querySelector("em")).toBeInTheDocument();
    });
  });
});
```

---

## E2E Tests

**File:** `e2e/markdown-rendering.spec.ts`

```typescript
import { test, expect } from "@playwright/test";

test.describe("Markdown Rendering", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/query");
  });

  test("renders bold text correctly from LLM response", async ({ page }) => {
    // Mock LLM response with problematic bold
    await page.route("**/api/query/stream**", async (route) => {
      const body =
        'data: {"content":"1. **Products **:\\n- Item 1\\n- Item 2"}\n\n';
      await route.fulfill({
        body,
        headers: { "Content-Type": "text/event-stream" },
      });
    });

    await page.getByTestId("query-input").fill("List products");
    await page.getByTestId("send-button").click();

    await page.waitForSelector('[data-testid="message-content"]');

    // Should have bold text
    const boldElement = page.locator('[data-testid="message-content"] strong');
    await expect(boldElement).toHaveText("Products");

    // Should NOT have literal asterisks
    const content = await page
      .locator('[data-testid="message-content"]')
      .textContent();
    expect(content).not.toContain("**");
  });

  test("renders lists without spurious checkboxes", async ({ page }) => {
    await page.route("**/api/query/stream**", async (route) => {
      const body = 'data: {"content":"- Item 1\\n- Item 2\\n- Item 3"}\n\n';
      await route.fulfill({
        body,
        headers: { "Content-Type": "text/event-stream" },
      });
    });

    await page.getByTestId("query-input").fill("List items");
    await page.getByTestId("send-button").click();

    await page.waitForSelector('[data-testid="message-content"]');

    // Should have list items
    const listItems = page.locator('[data-testid="message-content"] li');
    await expect(listItems).toHaveCount(3);

    // Should NOT have checkboxes
    const checkboxes = page.locator(
      '[data-testid="message-content"] input[type="checkbox"]'
    );
    await expect(checkboxes).toHaveCount(0);
  });

  test("renders task lists with checkboxes", async ({ page }) => {
    await page.route("**/api/query/stream**", async (route) => {
      const body =
        'data: {"content":"- [ ] Todo 1\\n- [x] Done 1\\n- [ ] Todo 2"}\n\n';
      await route.fulfill({
        body,
        headers: { "Content-Type": "text/event-stream" },
      });
    });

    await page.getByTestId("query-input").fill("Show tasks");
    await page.getByTestId("send-button").click();

    await page.waitForSelector('[data-testid="message-content"]');

    const checkboxes = page.locator(
      '[data-testid="message-content"] input[type="checkbox"]'
    );
    await expect(checkboxes).toHaveCount(3);

    // Verify checked states
    await expect(checkboxes.nth(0)).not.toBeChecked();
    await expect(checkboxes.nth(1)).toBeChecked();
    await expect(checkboxes.nth(2)).not.toBeChecked();
  });

  test("renders GitHub alerts", async ({ page }) => {
    await page.route("**/api/query/stream**", async (route) => {
      const body =
        'data: {"content":"> [!WARNING]\\n> This is a warning message."}\n\n';
      await route.fulfill({
        body,
        headers: { "Content-Type": "text/event-stream" },
      });
    });

    await page.getByTestId("query-input").fill("Show warning");
    await page.getByTestId("send-button").click();

    await page.waitForSelector('[data-testid="message-content"]');

    // Alert should be rendered (not as blockquote)
    const alert = page.locator(
      '[data-testid="message-content"] [class*="alert"]'
    );
    await expect(alert).toBeVisible();
  });
});
```

---

## Test Fixtures

**File:** `__tests__/fixtures/llm-output-samples.ts`

```typescript
/**
 * Real examples of LLM output that should be normalized
 */
export const LLM_OUTPUT_SAMPLES = {
  // Common bold issues
  boldTrailingSpace: {
    input: "**Products **:",
    expectedBold: "Products",
  },

  // Complex list from real LLM
  productList: {
    input: `The main entities in your knowledge graph include:

1. **Products **:
   - Code2Doc
   - Code2Doc Dataset
   - Graphcodebert
   - Detectgpt
   - Codebertscore

2. ** Concepts **:
   - The curse of recursion
   - Machine learning`,
    expectedBolds: ["Products", "Concepts"],
    expectedNoCheckboxes: true,
  },

  // Mixed formatting
  mixedFormatting: {
    input: "This has **bold **, *italic *, and `code` together.",
    expectedBold: "bold",
    expectedItalic: "italic",
  },
};

/**
 * Malformed markdown that should still render
 */
export const MALFORMED_MARKDOWN = {
  unclosedBold: "**This is unclosed",
  unclosedItalic: "*This is unclosed",
  mismatchedMarkers: "**This is *mixed**",
  emptyBold: "****",
  spaceOnlyBold: "**   **",
};

/**
 * Security test cases
 */
export const SECURITY_TEST_CASES = {
  scriptInjection: '<script>alert("xss")</script>',
  eventHandler: '<img src="x" onerror="alert(\'xss\')">',
  iframeInjection: "<iframe src=\"javascript:alert('xss')\">",
  styleExpression: '<div style="background:url(javascript:alert(1))">',
};
```

---

## Running Tests

```bash
# Run all markdown tests
cd edgequake_webui
pnpm test src/components/query/markdown

# Run with coverage
pnpm test:coverage src/components/query/markdown

# Run E2E tests
pnpm exec playwright test e2e/markdown

# Run specific test file
pnpm test src/components/query/markdown/__tests__/normalize-markdown.test.ts
```

---

## CI/CD Integration

```yaml
# .github/workflows/markdown-tests.yml
name: Markdown Tests

on:
  push:
    paths:
      - "edgequake_webui/src/components/query/markdown/**"
  pull_request:

jobs:
  unit-tests:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: pnpm/action-setup@v2
      - uses: actions/setup-node@v4
        with:
          node-version: 20
          cache: "pnpm"
          cache-dependency-path: edgequake_webui/pnpm-lock.yaml
      - run: cd edgequake_webui && pnpm install
      - run: cd edgequake_webui && pnpm test src/components/query/markdown

  e2e-tests:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: pnpm/action-setup@v2
      - uses: actions/setup-node@v4
        with:
          node-version: 20
          cache: "pnpm"
      - run: cd edgequake_webui && pnpm install
      - run: cd edgequake_webui && pnpm exec playwright install --with-deps
      - run: cd edgequake_webui && pnpm build
      - run: cd edgequake_webui && pnpm exec playwright test e2e/markdown
```

---

## Success Criteria

| Test Type                | Minimum Coverage | Target |
| ------------------------ | ---------------- | ------ |
| Normalization Unit Tests | 100%             | 100%   |
| Token Rendering Tests    | 90%              | 95%    |
| Integration Tests        | 85%              | 90%    |
| E2E Tests                | 80%              | 85%    |

All tests must pass before merging any markdown-related changes.
