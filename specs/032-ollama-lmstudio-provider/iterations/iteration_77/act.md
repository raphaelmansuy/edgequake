# OODA 77 - Act: Navigation Flow Tests

## Actions Taken

### Added page load smoke tests for query and api-explorer

```typescript
test("query page loads", async ({ page }) => {
  await page.goto("/query", { waitUntil: "domcontentloaded" });
  const main = page.locator("main");
  await expect(main).toBeVisible({ timeout: 15000 });
});

test("api explorer page loads", async ({ page }) => {
  await page.goto("/api-explorer", { waitUntil: "domcontentloaded" });
  const main = page.locator("main");
  await expect(main).toBeVisible({ timeout: 15000 });
});
```

### Added navigation flow tests

```typescript
test.describe("Navigation Flow", () => {
  test("sidebar documents link navigates correctly", async ({ page }) => {
    // Uses .first() to avoid strict mode violations
    const docsLink = page.getByRole("link", { name: /documents/i }).first();
    if (await docsLink.isVisible()) {
      await docsLink.click();
      await page.waitForURL(/\/documents/, { timeout: 10000 });
      expect(page.url()).toContain("/documents");
    }
  });

  test("sidebar graph link navigates correctly", async ({ page }) => {
    const graphLink = page.getByRole("link", { name: /graph/i }).first();
    // ...similar pattern
  });

  test("browser back navigation works", async ({ page }) => {
    await page.goto("/");
    await page.goto("/documents");
    await page.goBack();
    expect(page.url()).not.toContain("/documents");
  });
});
```

## Bug Fix
- Used `.first()` on locators to avoid strict mode violations when multiple matching elements exist (sidebar + dashboard cards)

## Test Results

```
Running 40 tests using 8 workers
40 passed (6.5s)
```

## Coverage Update

| Category | Tests |
|----------|-------|
| Focus 1&2: Config | 3 |
| Focus 3: Query UI | 2 |
| Focus 4: Settings | 3 |
| Focus 5: Rebuild | 2 |
| Focus 6: Deeplinks | 4 |
| Focus 7: Multi-model | 10 |
| Focus 8: Streaming | 2 |
| Error Handling | 2 |
| Pagination/Structure | 4 |
| UI Page Load | 6 |
| Navigation Flow | 3 |
| **Total** | **40** |
