# OODA-75: E2E Test Strategy

**Date**: 2026-02-01
**Focus**: End-to-End Testing for Documents

## OBSERVE

### Mission Requirements (Re-read from specs/002-unify-ingestion-pipeline.md)
- Comprehensive test coverage
- Automated validation

### Current E2E Test Structure

**Playwright Tests:**
```
edgequake_webui/e2e/
├── document-upload.spec.ts
├── document-viewer.spec.ts
├── pdf-rendering.spec.ts
└── navigation.spec.ts
```

## ORIENT

### Test Scenarios

| Test | Description | Priority |
|------|-------------|----------|
| PDF Upload | Upload PDF, verify in list | P0 |
| Double-click Navigation | Navigate to detail page | P0 |
| View Details Button | Panel → Detail navigation | P0 |
| Side-by-side Viewer | PDF and Markdown visible | P1 |
| Download PDF | Verify download URL | P1 |
| Mobile Tabs | Tab switching works | P2 |

### Test Data

```
test-docs/
├── sample.pdf          # Small PDF for tests
├── multi-page.pdf      # 10 pages for scroll tests
└── sample.txt          # Text for comparison
```

## DECIDE

**Decision**: Define E2E test cases for new features

Critical tests:
1. PDF upload and immediate visibility
2. Double-click navigation
3. Side-by-side rendering

## ACT

### E2E Test: PDF Upload Visibility

```typescript
// e2e/document-upload.spec.ts
test('PDF appears immediately after upload', async ({ page }) => {
  await page.goto('/documents');
  
  // Upload PDF
  const fileChooserPromise = page.waitForEvent('filechooser');
  await page.click('[data-testid="upload-button"]');
  const fileChooser = await fileChooserPromise;
  await fileChooser.setFiles('test-docs/sample.pdf');
  
  // Verify immediate appearance
  await expect(page.getByText('sample.pdf')).toBeVisible({ timeout: 2000 });
  await expect(page.getByText('Processing')).toBeVisible();
});
```

### E2E Test: Double-click Navigation

```typescript
// e2e/navigation.spec.ts
test('double-click navigates to document detail', async ({ page }) => {
  await page.goto('/documents');
  
  // Wait for documents to load
  const row = page.getByRole('row').filter({ hasText: 'sample.pdf' });
  await expect(row).toBeVisible();
  
  // Double-click
  await row.dblclick();
  
  // Verify navigation
  await expect(page).toHaveURL(/\/documents\/[a-f0-9-]+/);
  await expect(page.getByRole('heading', { name: 'sample.pdf' })).toBeVisible();
});
```

### E2E Test: Side-by-side Viewer

```typescript
// e2e/pdf-rendering.spec.ts
test('PDF document shows side-by-side view', async ({ page }) => {
  await page.goto('/documents/[pdf-doc-id]');
  
  // Both viewers visible on desktop
  await expect(page.getByTestId('pdf-viewer')).toBeVisible();
  await expect(page.getByTestId('markdown-viewer')).toBeVisible();
});
```

**Status**: 📋 DOCUMENTED - E2E test cases defined
