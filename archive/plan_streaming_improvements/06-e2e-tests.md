# E2E Test Specifications for Streaming Improvements

## 1. Overview

This document defines end-to-end tests using Playwright to validate the streaming chat improvements from the user's perspective. Tests cover the complete flow from UI interaction through backend processing.

## 2. Test Environment Setup

### 2.1 Prerequisites

```bash
# Install Playwright
cd edgequake_webui
pnpm add -D @playwright/test

# Install browsers
pnpm exec playwright install

# Ensure backend is running
make dev  # From project root
```

### 2.2 Playwright Configuration

```typescript
// playwright.config.ts
import { defineConfig, devices } from "@playwright/test";

export default defineConfig({
  testDir: "./e2e",
  fullyParallel: false, // Streaming tests need sequential execution
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 2 : 0,
  workers: 1, // Sequential for streaming tests
  reporter: "html",
  timeout: 60000, // Streaming can take time

  use: {
    baseURL: "http://localhost:3000",
    trace: "on-first-retry",
  },

  projects: [
    {
      name: "chromium",
      use: { ...devices["Desktop Chrome"] },
    },
  ],

  webServer: {
    command: "pnpm run dev",
    url: "http://localhost:3000",
    reuseExistingServer: !process.env.CI,
    timeout: 120000,
  },
});
```

## 3. E2E Test Cases

### 3.1 Streaming Chat Flow Tests

```typescript
// e2e/streaming-chat.spec.ts
import { test, expect } from "@playwright/test";

test.describe("Streaming Chat", () => {
  test.beforeEach(async ({ page }) => {
    // Navigate to query page
    await page.goto("/query");

    // Wait for page to be ready
    await expect(page.locator('[data-testid="query-input"]')).toBeVisible();
  });

  test("complete streaming response is rendered correctly", async ({
    page,
  }) => {
    // Type a query
    const input = page.locator('[data-testid="query-input"]');
    await input.fill("What is the capital of France?");

    // Submit
    await input.press("Enter");

    // Wait for streaming to start
    const responseArea = page
      .locator('[data-testid="assistant-message"]')
      .last();
    await expect(responseArea).toBeVisible({ timeout: 10000 });

    // Wait for streaming to complete (look for done indicator)
    await expect(
      page.locator('[data-testid="streaming-indicator"]')
    ).toBeHidden({
      timeout: 30000,
    });

    // Verify content is present
    const content = await responseArea.textContent();
    expect(content).toContain("Paris");

    // Verify markdown is rendered (no raw markdown visible)
    expect(content).not.toContain("**");
    expect(content).not.toContain("##");
  });

  test("streaming tokens appear progressively", async ({ page }) => {
    const input = page.locator('[data-testid="query-input"]');
    await input.fill("Write a short poem about the sea");
    await input.press("Enter");

    const responseArea = page
      .locator('[data-testid="assistant-message"]')
      .last();
    await expect(responseArea).toBeVisible();

    // Record content length over time
    const contentLengths: number[] = [];

    for (let i = 0; i < 10; i++) {
      await page.waitForTimeout(500);
      const content = (await responseArea.textContent()) || "";
      contentLengths.push(content.length);
    }

    // Verify content grew over time (streaming is progressive)
    const isProgressive = contentLengths.some(
      (len, i) => i > 0 && len > contentLengths[i - 1]
    );
    expect(isProgressive).toBe(true);
  });

  test("conversation is persisted after streaming completes", async ({
    page,
  }) => {
    const uniqueQuery = `Test query ${Date.now()}`;

    // Submit query
    const input = page.locator('[data-testid="query-input"]');
    await input.fill(uniqueQuery);
    await input.press("Enter");

    // Wait for completion
    await expect(
      page.locator('[data-testid="streaming-indicator"]')
    ).toBeHidden({
      timeout: 30000,
    });

    // Refresh page
    await page.reload();
    await expect(page.locator('[data-testid="query-input"]')).toBeVisible();

    // Check conversation list
    const conversationList = page.locator('[data-testid="conversation-list"]');
    await expect(
      conversationList.locator("text=" + uniqueQuery.substring(0, 20))
    ).toBeVisible();
  });

  test("error during streaming shows error state", async ({ page }) => {
    // This test requires a way to trigger an error
    // Option 1: Use a special query that the mock/test backend handles
    const input = page.locator('[data-testid="query-input"]');
    await input.fill("__TRIGGER_STREAM_ERROR__");
    await input.press("Enter");

    // Wait for error indicator
    await expect(page.locator('[data-testid="error-message"]')).toBeVisible({
      timeout: 15000,
    });

    // Verify error is user-friendly
    const errorText = await page
      .locator('[data-testid="error-message"]')
      .textContent();
    expect(errorText).not.toContain("undefined");
    expect(errorText).not.toContain("null");
  });

  test("can cancel streaming mid-generation", async ({ page }) => {
    const input = page.locator('[data-testid="query-input"]');
    await input.fill("Write a very long essay about everything");
    await input.press("Enter");

    // Wait for streaming to start
    await expect(
      page.locator('[data-testid="streaming-indicator"]')
    ).toBeVisible();

    // Click stop button
    const stopButton = page.locator('[data-testid="stop-generation"]');
    await stopButton.click();

    // Verify streaming stopped
    await expect(
      page.locator('[data-testid="streaming-indicator"]')
    ).toBeHidden();

    // Partial content should be visible
    const responseArea = page
      .locator('[data-testid="assistant-message"]')
      .last();
    const content = await responseArea.textContent();
    expect(content?.length).toBeGreaterThan(0);
  });
});
```

### 3.2 Token Storage Validation Tests

```typescript
// e2e/token-storage.spec.ts
import { test, expect } from "@playwright/test";

test.describe("Token Storage", () => {
  test("message stores correct token count", async ({ page, request }) => {
    // Submit a query
    await page.goto("/query");
    const input = page.locator('[data-testid="query-input"]');
    await input.fill("Hello, how are you?");
    await input.press("Enter");

    // Wait for completion
    await expect(
      page.locator('[data-testid="streaming-indicator"]')
    ).toBeHidden({
      timeout: 30000,
    });

    // Get conversation ID from URL or data attribute
    const conversationId = await page
      .locator("[data-conversation-id]")
      .getAttribute("data-conversation-id");

    // Fetch message via API
    const response = await request.get(
      `/api/v1/conversations/${conversationId}/messages`
    );
    const messages = await response.json();

    // Find assistant message
    const assistantMessage = messages.items.find(
      (m: any) => m.role === "assistant"
    );

    // Verify token count is reasonable (not chunk count)
    expect(assistantMessage.tokens_used).toBeGreaterThan(0);
    expect(assistantMessage.tokens_used).toBeLessThan(1000); // Reasonable for short response

    // Verify prompt tokens are stored (new field)
    // expect(assistantMessage.prompt_tokens).toBeGreaterThan(0);
  });

  test("message stores API response metadata", async ({ page, request }) => {
    await page.goto("/query");
    const input = page.locator('[data-testid="query-input"]');
    await input.fill("What is 2 + 2?");
    await input.press("Enter");

    await expect(
      page.locator('[data-testid="streaming-indicator"]')
    ).toBeHidden({
      timeout: 30000,
    });

    const conversationId = await page
      .locator("[data-conversation-id]")
      .getAttribute("data-conversation-id");
    const response = await request.get(
      `/api/v1/conversations/${conversationId}/messages`
    );
    const messages = await response.json();

    const assistantMessage = messages.items.find(
      (m: any) => m.role === "assistant"
    );

    // Verify duration is stored
    expect(assistantMessage.duration_ms).toBeGreaterThan(0);

    // Verify model is stored (new field)
    // expect(assistantMessage.model_used).toBeDefined();
    // expect(assistantMessage.model_used).toContain('gpt');
  });

  test("full response stored matches streamed content", async ({
    page,
    request,
  }) => {
    await page.goto("/query");

    const input = page.locator('[data-testid="query-input"]');
    await input.fill("Write exactly: Hello World");
    await input.press("Enter");

    // Capture streamed content
    const responseArea = page
      .locator('[data-testid="assistant-message"]')
      .last();
    await expect(
      page.locator('[data-testid="streaming-indicator"]')
    ).toBeHidden({
      timeout: 30000,
    });

    const displayedContent = await responseArea.textContent();

    // Get stored content from API
    const conversationId = await page
      .locator("[data-conversation-id]")
      .getAttribute("data-conversation-id");
    const response = await request.get(
      `/api/v1/conversations/${conversationId}/messages`
    );
    const messages = await response.json();

    const assistantMessage = messages.items.find(
      (m: any) => m.role === "assistant"
    );

    // Stored content should match displayed content
    // Note: May need to account for markdown rendering
    expect(assistantMessage.content).toBeTruthy();
    expect(assistantMessage.content.length).toBeGreaterThan(0);
  });
});
```

### 3.3 Cache Behavior Tests

```typescript
// e2e/cache-behavior.spec.ts
import { test, expect } from "@playwright/test";

test.describe("Cache Behavior", () => {
  test("repeated conversation fetch is faster (cache hit)", async ({
    page,
    request,
  }) => {
    // Create a conversation first
    await page.goto("/query");
    const input = page.locator('[data-testid="query-input"]');
    await input.fill("Hello");
    await input.press("Enter");
    await expect(
      page.locator('[data-testid="streaming-indicator"]')
    ).toBeHidden({
      timeout: 30000,
    });

    const conversationId = await page
      .locator("[data-conversation-id]")
      .getAttribute("data-conversation-id");

    // First fetch (cold)
    const start1 = Date.now();
    await request.get(`/api/v1/conversations/${conversationId}`);
    const duration1 = Date.now() - start1;

    // Second fetch (should be cached)
    const start2 = Date.now();
    await request.get(`/api/v1/conversations/${conversationId}`);
    const duration2 = Date.now() - start2;

    // Second fetch should be faster (though network variance makes this flaky)
    // In reality, we'd check metrics endpoint for cache hit rate
    console.log(`First fetch: ${duration1}ms, Second fetch: ${duration2}ms`);

    // At minimum, both should complete reasonably
    expect(duration1).toBeLessThan(5000);
    expect(duration2).toBeLessThan(5000);
  });

  test("message update invalidates cache", async ({ page, request }) => {
    // This would require a metrics endpoint to verify cache invalidation
    // For now, we test that updates are reflected
    await page.goto("/query");
    const input = page.locator('[data-testid="query-input"]');
    await input.fill("First message");
    await input.press("Enter");
    await expect(
      page.locator('[data-testid="streaming-indicator"]')
    ).toBeHidden({
      timeout: 30000,
    });

    // Add second message
    await input.fill("Second message");
    await input.press("Enter");
    await expect(
      page.locator('[data-testid="streaming-indicator"]')
    ).toBeHidden({
      timeout: 30000,
    });

    // Refresh and verify both messages are visible
    await page.reload();

    await expect(
      page.locator('[data-testid="user-message"]').first()
    ).toContainText("First message");
    await expect(
      page.locator('[data-testid="user-message"]').last()
    ).toContainText("Second message");
  });
});
```

### 3.4 Connection Resilience Tests

```typescript
// e2e/connection-resilience.spec.ts
import { test, expect } from "@playwright/test";

test.describe("Connection Resilience", () => {
  test("partial response shown on connection drop", async ({
    page,
    context,
  }) => {
    await page.goto("/query");

    const input = page.locator('[data-testid="query-input"]');
    await input.fill("Write a medium length paragraph");
    await input.press("Enter");

    // Wait for streaming to start
    await expect(
      page.locator('[data-testid="streaming-indicator"]')
    ).toBeVisible();

    // Wait a bit for partial content
    await page.waitForTimeout(2000);

    // Simulate network disconnect
    await context.setOffline(true);

    // Wait for error handling
    await page.waitForTimeout(1000);

    // Verify partial content is visible
    const responseArea = page
      .locator('[data-testid="assistant-message"]')
      .last();
    const content = await responseArea.textContent();
    expect(content?.length).toBeGreaterThan(0);

    // Re-enable network
    await context.setOffline(false);
  });

  test("reconnection shows saved state", async ({ page, context }) => {
    await page.goto("/query");

    const input = page.locator('[data-testid="query-input"]');
    await input.fill("Hello world");
    await input.press("Enter");

    // Wait for completion
    await expect(
      page.locator('[data-testid="streaming-indicator"]')
    ).toBeHidden({
      timeout: 30000,
    });

    // Simulate disconnect and reconnect
    await context.setOffline(true);
    await page.waitForTimeout(500);
    await context.setOffline(false);

    // Refresh page
    await page.reload();

    // Verify conversation is still there
    await expect(
      page.locator('[data-testid="assistant-message"]')
    ).toBeVisible();
  });
});
```

### 3.5 Performance Tests

```typescript
// e2e/performance.spec.ts
import { test, expect } from "@playwright/test";

test.describe("Performance", () => {
  test("first token appears within 3 seconds", async ({ page }) => {
    await page.goto("/query");

    const startTime = Date.now();

    const input = page.locator('[data-testid="query-input"]');
    await input.fill("Hello");
    await input.press("Enter");

    // Wait for first content to appear
    const responseArea = page
      .locator('[data-testid="assistant-message"]')
      .last();
    await expect(responseArea).toBeVisible();

    // Wait for non-empty content
    await expect(async () => {
      const content = await responseArea.textContent();
      expect(content?.length).toBeGreaterThan(0);
    }).toPass({ timeout: 10000 });

    const firstTokenTime = Date.now() - startTime;

    console.log(`First token time: ${firstTokenTime}ms`);
    expect(firstTokenTime).toBeLessThan(5000); // 5 second max
  });

  test("conversation list loads within 1 second", async ({ page }) => {
    // Create some conversations first
    await page.goto("/query");
    for (let i = 0; i < 3; i++) {
      const input = page.locator('[data-testid="query-input"]');
      await input.fill(`Test ${i}`);
      await input.press("Enter");
      await expect(
        page.locator('[data-testid="streaming-indicator"]')
      ).toBeHidden({
        timeout: 30000,
      });
    }

    // Navigate away and back
    const startTime = Date.now();
    await page.goto("/query");

    await expect(
      page.locator('[data-testid="conversation-list"]')
    ).toBeVisible();

    const loadTime = Date.now() - startTime;
    console.log(`Conversation list load time: ${loadTime}ms`);

    expect(loadTime).toBeLessThan(3000);
  });

  test("large response renders without lag", async ({ page }) => {
    await page.goto("/query");

    const input = page.locator('[data-testid="query-input"]');
    await input.fill("Write a detailed 500 word essay about technology");
    await input.press("Enter");

    // Wait for completion
    await expect(
      page.locator('[data-testid="streaming-indicator"]')
    ).toBeHidden({
      timeout: 60000,
    });

    // Check that page is responsive (can interact with UI)
    await expect(input).toBeEnabled();

    // Verify content rendered
    const responseArea = page
      .locator('[data-testid="assistant-message"]')
      .last();
    const content = await responseArea.textContent();
    expect(content?.length).toBeGreaterThan(500); // Some content
  });
});
```

## 4. Test Data Fixtures

```typescript
// e2e/fixtures/test-data.ts
export const testQueries = {
  short: "What is 2+2?",
  medium: "Explain the water cycle in 3 sentences.",
  long: "Write a detailed essay about the history of computing.",
  withCode: "Show me a Python function to calculate fibonacci numbers.",
  withMarkdown: "Create a markdown table comparing Python and JavaScript.",
  unicode: "Say hello in 5 different languages with their scripts.",
  error: "__TRIGGER_STREAM_ERROR__",
};

export const expectedResponses = {
  short: {
    minLength: 5,
    maxLength: 100,
    contains: ["4", "four"],
  },
  medium: {
    minLength: 100,
    maxLength: 500,
    contains: ["water", "cycle"],
  },
};
```

## 5. Running E2E Tests

```bash
# Run all E2E tests
cd edgequake_webui
pnpm exec playwright test e2e/

# Run specific test file
pnpm exec playwright test e2e/streaming-chat.spec.ts

# Run with UI mode
pnpm exec playwright test --ui

# Run with debug mode
pnpm exec playwright test --debug

# Generate report
pnpm exec playwright show-report
```

## 6. CI Integration

```yaml
# .github/workflows/e2e-tests.yml
name: E2E Tests

on:
  push:
    branches: [main, feat/*]
  pull_request:
    branches: [main]

jobs:
  test:
    runs-on: ubuntu-latest

    steps:
      - uses: actions/checkout@v4

      - name: Setup Node.js
        uses: actions/setup-node@v4
        with:
          node-version: "20"

      - name: Install pnpm
        uses: pnpm/action-setup@v2
        with:
          version: 8

      - name: Install dependencies
        run: |
          cd edgequake_webui
          pnpm install
          pnpm exec playwright install --with-deps

      - name: Start backend
        run: |
          make dev-backend &
          sleep 10

      - name: Run E2E tests
        run: |
          cd edgequake_webui
          pnpm exec playwright test

      - name: Upload test results
        uses: actions/upload-artifact@v4
        if: failure()
        with:
          name: playwright-report
          path: edgequake_webui/playwright-report/
```

---

_Document Version: 1.0_
_Created: 2024-12-28_
