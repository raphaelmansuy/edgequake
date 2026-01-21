# OODA 76 - Act: Core UI Page Load Tests

## Actions Taken

### Added Core UI Page Load smoke tests

```typescript
test.describe("Core UI Page Load", () => {
  test("dashboard page loads", async ({ page }) => {
    await page.goto("/", { waitUntil: "domcontentloaded" });
    const main = page.locator("main");
    await expect(main).toBeVisible({ timeout: 15000 });
  });

  test("documents page loads", async ({ page }) => {
    await page.goto("/documents", { waitUntil: "domcontentloaded" });
    const main = page.locator("main");
    await expect(main).toBeVisible({ timeout: 15000 });
  });

  test("graph page loads", async ({ page }) => {
    await page.goto("/graph", { waitUntil: "domcontentloaded" });
    const main = page.locator("main");
    await expect(main).toBeVisible({ timeout: 15000 });
  });

  test("costs page loads", async ({ page }) => {
    await page.goto("/costs", { waitUntil: "domcontentloaded" });
    const main = page.locator("main");
    await expect(main).toBeVisible({ timeout: 15000 });
  });
});
```

## Test Results

```
Running 35 tests using 8 workers
  ✓ ... (all previous tests)
  ✓ dashboard page loads  <-- NEW
  ✓ documents page loads  <-- NEW
  ✓ graph page loads  <-- NEW
  ✓ costs page loads  <-- NEW
  1 skipped
  34 passed (5.9s)
```

## Coverage Update

| Category             | Tests  |
| -------------------- | ------ |
| Focus 1&2: Config    | 3      |
| Focus 3: Query UI    | 2      |
| Focus 4: Settings    | 3      |
| Focus 5: Rebuild     | 2      |
| Focus 6: Deeplinks   | 4      |
| Focus 7: Multi-model | 10     |
| Focus 8: Streaming   | 2      |
| Error Handling       | 2      |
| Pagination/Structure | 4      |
| UI Page Load         | 4      |
| **Total**            | **35** |
