# E2E Testing Strategy

**Priority:** HIGH  
**Estimated Effort:** 2-3 days  
**Tools:** Playwright, Testing Library

## Testing Philosophy

> "Test user journeys, not implementation details"

Our E2E tests should verify complete user workflows from start to finish, ensuring the application works as users expect it to work.

## Test Scenarios

### 1. Workspace/Tenant Default Selection

#### Scenario 1.1: First-Time User Onboarding
```typescript
// e2e/workspace-onboarding.spec.ts
test('first-time user sees onboarding and creates workspace', async ({ page }) => {
  // Given: New user with no tenant/workspace
  await page.goto('/');
  
  // Then: Should see welcome onboarding
  await expect(page.getByRole('heading', { name: /welcome to edgequake/i })).toBeVisible();
  
  // When: User creates first workspace
  await page.getByLabel('Workspace Name').fill('My First Workspace');
  await page.getByLabel('Description').fill('Test workspace');
  await page.getByRole('button', { name: /create workspace/i }).click();
  
  // Then: Should be redirected to dashboard with workspace selected
  await expect(page).toHaveURL('/dashboard');
  await expect(page.getByText('My First Workspace')).toBeVisible();
  
  // And: Workspace should be persisted
  await page.reload();
  await expect(page.getByText('My First Workspace')).toBeVisible();
});
```

#### Scenario 1.2: Returning User Auto-Selection
```typescript
test('returning user automatically enters last workspace', async ({ page, context }) => {
  // Given: User previously selected a workspace
  await context.addCookies([
    { name: 'edgequake-tenant', value: JSON.stringify({
      selectedTenantId: 'tenant-123',
      selectedWorkspaceId: 'workspace-456',
      lastSelectionTime: Date.now()
    }), domain: 'localhost', path: '/' }
  ]);
  
  // When: User visits the app
  await page.goto('/');
  
  // Then: Should automatically load into that workspace
  await expect(page.getByTestId('workspace-selector')).toHaveText('My Workspace');
  
  // And: No onboarding or selection dialog shown
  await expect(page.getByRole('dialog')).not.toBeVisible();
  
  // And: Can interact with workspace immediately
  await page.getByRole('link', { name: /documents/i }).click();
  await expect(page).toHaveURL('/documents');
});
```

#### Scenario 1.3: Deleted Workspace Graceful Handling
```typescript
test('handles deleted workspace gracefully', async ({ page }) => {
  // Given: User had selected workspace that was deleted
  localStorage.setItem('edgequake-tenant', JSON.stringify({
    selectedWorkspaceId: 'deleted-workspace-id'
  }));
  
  // When: User visits app
  await page.goto('/');
  
  // Then: Should show notification and select next available workspace
  await expect(page.getByText(/workspace no longer available/i)).toBeVisible();
  await expect(page.getByTestId('workspace-selector')).toHaveText(/default workspace/i);
});
```

### 2. Document Detail Page

#### Scenario 2.1: Markdown Document Display
```typescript
// e2e/document-detail.spec.ts
test('displays markdown document with proper formatting', async ({ page }) => {
  // Given: A markdown document exists
  await page.goto('/documents/doc-markdown-123');
  
  // Then: Should render with proper formatting
  await expect(page.getByRole('heading', { level: 1 })).toBeVisible();
  await expect(page.locator('article.prose')).toBeVisible();
  
  // And: Math equations should be rendered
  await expect(page.locator('.katex')).toBeVisible();
  
  // And: Code blocks should have syntax highlighting
  await expect(page.locator('pre code.language-typescript')).toBeVisible();
  
  // And: Can copy content
  await page.getByRole('button', { name: /copy/i }).click();
  const clipboardText = await page.evaluate(() => navigator.clipboard.readText());
  expect(clipboardText).toContain('# Document Title');
});
```

#### Scenario 2.2: Code Document with Syntax Highlighting
```typescript
test('displays code document with syntax highlighting', async ({ page }) => {
  // Given: A Python code document
  await page.goto('/documents/doc-python-456');
  
  // Then: Should detect language and show badge
  await expect(page.getByText('python')).toBeVisible();
  
  // And: Should have line numbers
  await expect(page.locator('[data-line-number="1"]')).toBeVisible();
  
  // And: Should have syntax highlighting
  const codeBlock = page.locator('pre code');
  await expect(codeBlock.locator('.token.keyword')).toBeVisible();
  
  // When: User hovers over code
  await codeBlock.hover();
  
  // Then: Should show floating toolbar
  await expect(page.getByRole('button', { name: /copy/i })).toBeVisible();
  await expect(page.getByRole('button', { name: /download/i })).toBeVisible();
});
```

#### Scenario 2.3: Metadata Sidebar Navigation
```typescript
test('metadata sidebar shows lineage and allows exploration', async ({ page }) => {
  // Given: Document detail page is open
  await page.goto('/documents/doc-123');
  
  // Then: Key stats should be sticky and visible
  const keyStats = page.getByTestId('key-stats');
  await expect(keyStats).toBeVisible();
  await expect(keyStats.getByText(/chunks/i)).toBeVisible();
  await expect(keyStats.getByText(/entities/i)).toBeVisible();
  
  // When: User scrolls content
  await page.mouse.wheel(0, 500);
  
  // Then: Key stats remain visible (sticky)
  await expect(keyStats).toBeInViewport();
  
  // And: Can expand lineage section
  await page.getByRole('button', { name: /extraction lineage/i }).click();
  await expect(page.getByTestId('lineage-tree')).toBeVisible();
  
  // And: Can view entity graph preview
  await page.getByRole('button', { name: /knowledge graph/i }).click();
  await expect(page.getByTestId('mini-graph')).toBeVisible();
  
  // When: User clicks on entity
  await page.locator('[data-entity-id]').first().click();
  
  // Then: Should navigate to graph view with entity selected
  await expect(page).toHaveURL(/\/graph\?entity=/);
});
```

#### Scenario 2.4: Responsive Layout on Mobile
```typescript
test('adapts layout for mobile devices', async ({ page }) => {
  // Given: Mobile viewport
  await page.setViewportSize({ width: 375, height: 667 });
  await page.goto('/documents/doc-123');
  
  // Then: Should show tabs instead of sidebar
  await expect(page.getByRole('tab', { name: /content/i })).toBeVisible();
  await expect(page.getByRole('tab', { name: /details/i })).toBeVisible();
  
  // When: User switches to details tab
  await page.getByRole('tab', { name: /details/i }).click();
  
  // Then: Should show metadata
  await expect(page.getByTestId('key-stats')).toBeVisible();
  
  // And: Content should be hidden
  await expect(page.getByTestId('content-renderer')).not.toBeVisible();
});
```

#### Scenario 2.5: PDF Document Preview
```typescript
test('displays PDF document with preview', async ({ page }) => {
  // Given: A PDF document
  await page.goto('/documents/doc-pdf-789');
  
  // Then: Should show PDF viewer
  await expect(page.getByTestId('pdf-viewer')).toBeVisible();
  
  // And: Should show page navigation
  await expect(page.getByText(/page 1 of/i)).toBeVisible();
  
  // When: User navigates to next page
  await page.getByRole('button', { name: /next page/i }).click();
  
  // Then: Should show page 2
  await expect(page.getByText(/page 2 of/i)).toBeVisible();
  
  // And: Can download PDF
  const downloadPromise = page.waitForEvent('download');
  await page.getByRole('button', { name: /download/i }).click();
  const download = await downloadPromise;
  expect(download.suggestedFilename()).toMatch(/\.pdf$/);
});
```

### 3. Cross-Feature Integration

#### Scenario 3.1: View Document in Graph
```typescript
test('navigates from document to graph view', async ({ page }) => {
  // Given: Viewing a document
  await page.goto('/documents/doc-123');
  
  // When: User clicks "View in Graph"
  await page.getByRole('button', { name: /view in graph/i }).click();
  
  // Then: Should navigate to graph with document highlighted
  await expect(page).toHaveURL('/graph?highlight=doc-123');
  await expect(page.locator('[data-highlighted="true"]')).toBeVisible();
});
```

#### Scenario 3.2: Search Within Document
```typescript
test('can search within document content', async ({ page }) => {
  // Given: Large document is open
  await page.goto('/documents/doc-large-content');
  
  // When: User opens search (Cmd/Ctrl+F)
  await page.keyboard.press('Meta+F');
  
  // Then: Should show search overlay
  await expect(page.getByPlaceholder(/search in document/i)).toBeVisible();
  
  // When: User searches for term
  await page.getByPlaceholder(/search in document/i).fill('important keyword');
  
  // Then: Should highlight matches
  await expect(page.locator('mark.search-highlight')).toHaveCount(5);
  
  // And: Should show match counter
  await expect(page.getByText(/1 of 5/i)).toBeVisible();
  
  // When: User navigates to next match
  await page.getByRole('button', { name: /next/i }).click();
  
  // Then: Should scroll to and focus second match
  await expect(page.getByText(/2 of 5/i)).toBeVisible();
});
```

## Performance Testing

### Load Time Tests
```typescript
test('document detail page loads within performance budget', async ({ page }) => {
  // Start performance measurement
  await page.goto('/documents/doc-123');
  
  const metrics = await page.evaluate(() => {
    const navigation = performance.getEntriesByType('navigation')[0] as PerformanceNavigationTiming;
    return {
      domContentLoaded: navigation.domContentLoadedEventEnd - navigation.fetchStart,
      loadComplete: navigation.loadEventEnd - navigation.fetchStart,
      firstContentfulPaint: performance.getEntriesByName('first-contentful-paint')[0]?.startTime,
    };
  });
  
  // Assert performance budgets
  expect(metrics.domContentLoaded).toBeLessThan(1500); // < 1.5s
  expect(metrics.loadComplete).toBeLessThan(3000);     // < 3s
  expect(metrics.firstContentfulPaint).toBeLessThan(1000); // < 1s
});
```

### Lighthouse CI Integration
```typescript
// playwright.config.ts
export default defineConfig({
  // ... other config
  projects: [
    {
      name: 'lighthouse',
      testMatch: /.*\.lighthouse\.ts/,
      use: {
        lighthouse: {
          playAudit: true,
          reports: {
            formats: {
              html: true,
            },
          },
          thresholds: {
            performance: 90,
            accessibility: 95,
            'best-practices': 90,
            seo: 90,
          },
        },
      },
    },
  ],
});
```

## Accessibility Testing

### Keyboard Navigation
```typescript
test('supports full keyboard navigation', async ({ page }) => {
  await page.goto('/documents/doc-123');
  
  // Tab through interactive elements
  await page.keyboard.press('Tab'); // Focus back button
  await expect(page.getByRole('button', { name: /back/i })).toBeFocused();
  
  await page.keyboard.press('Tab'); // Focus copy ID button
  await expect(page.getByRole('button', { name: /copy.*id/i })).toBeFocused();
  
  await page.keyboard.press('Tab'); // Focus view in graph
  await expect(page.getByRole('button', { name: /view in graph/i })).toBeFocused();
  
  // Activate button with Enter
  await page.keyboard.press('Enter');
  await expect(page).toHaveURL(/\/graph/);
});
```

### Screen Reader Compatibility
```typescript
test('has proper ARIA labels and roles', async ({ page }) => {
  await page.goto('/documents/doc-123');
  
  // Check main landmarks
  await expect(page.getByRole('main')).toBeVisible();
  await expect(page.getByRole('complementary')).toBeVisible(); // Sidebar
  
  // Check heading hierarchy
  const headings = await page.getByRole('heading').all();
  expect(headings.length).toBeGreaterThan(0);
  
  // Check for skip links
  await expect(page.getByRole('link', { name: /skip to content/i })).toBeInViewport();
  
  // Verify alt text on images
  const images = await page.locator('img').all();
  for (const img of images) {
    await expect(img).toHaveAttribute('alt');
  }
});
```

## Visual Regression Testing

```typescript
// e2e/visual/document-detail.spec.ts
test('document detail page matches visual snapshot', async ({ page }) => {
  await page.goto('/documents/doc-123');
  
  // Wait for all content to load
  await page.waitForLoadState('networkidle');
  
  // Take full page screenshot
  await expect(page).toHaveScreenshot('document-detail-full.png', {
    fullPage: true,
    animations: 'disabled',
  });
  
  // Screenshot key components
  await expect(page.getByTestId('key-stats')).toHaveScreenshot('key-stats.png');
  await expect(page.getByTestId('lineage-tree')).toHaveScreenshot('lineage-tree.png');
});

test('dark mode renders correctly', async ({ page }) => {
  // Enable dark mode
  await page.emulateMedia({ colorScheme: 'dark' });
  await page.goto('/documents/doc-123');
  
  await expect(page).toHaveScreenshot('document-detail-dark.png', {
    fullPage: true,
  });
});
```

## Test Data Setup

### Fixtures
```typescript
// e2e/fixtures/documents.ts
export const testDocuments = {
  markdown: {
    id: 'doc-markdown-123',
    title: 'Sample Markdown Document',
    mime_type: 'text/markdown',
    content: `# Main Heading
    
## Subheading

This is a paragraph with **bold** and *italic* text.

\`\`\`typescript
function example() {
  return "code block";
}
\`\`\`

Math equation: $E = mc^2$
    `,
    entity_count: 5,
    relationship_count: 8,
    chunk_count: 3,
  },
  
  python: {
    id: 'doc-python-456',
    title: 'example.py',
    mime_type: 'text/x-python',
    content: `def hello_world():
    """A simple function"""
    print("Hello, World!")

if __name__ == "__main__":
    hello_world()
    `,
    entity_count: 2,
    relationship_count: 1,
    chunk_count: 1,
  },
  
  pdf: {
    id: 'doc-pdf-789',
    title: 'Research Paper.pdf',
    mime_type: 'application/pdf',
    url: '/test-files/sample.pdf',
    entity_count: 25,
    relationship_count: 45,
    chunk_count: 12,
  },
};
```

### API Mocking
```typescript
// e2e/mocks/handlers.ts
import { http, HttpResponse } from 'msw';

export const handlers = [
  http.get('/api/v1/documents/:id', ({ params }) => {
    const { id } = params;
    const doc = testDocuments[id as string];
    
    if (!doc) {
      return HttpResponse.json({ error: 'Not found' }, { status: 404 });
    }
    
    return HttpResponse.json(doc);
  }),
  
  http.get('/api/v1/graph', () => {
    return HttpResponse.json({
      nodes: [...],
      edges: [...],
    });
  }),
];
```

## CI/CD Integration

### GitHub Actions Workflow
```yaml
# .github/workflows/e2e-tests.yml
name: E2E Tests

on:
  pull_request:
  push:
    branches: [main]

jobs:
  e2e:
    runs-on: ubuntu-latest
    strategy:
      matrix:
        browser: [chromium, firefox, webkit]
    
    steps:
      - uses: actions/checkout@v4
      
      - name: Setup Bun
        uses: oven-sh/setup-bun@v1
      
      - name: Install dependencies
        run: bun install
      
      - name: Install Playwright browsers
        run: bunx playwright install --with-deps ${{ matrix.browser }}
      
      - name: Build application
        run: bun run build
      
      - name: Start server
        run: bun run start &
        
      - name: Run E2E tests
        run: bunx playwright test --project=${{ matrix.browser }}
      
      - name: Upload test results
        if: always()
        uses: actions/upload-artifact@v4
        with:
          name: playwright-report-${{ matrix.browser }}
          path: playwright-report/
      
      - name: Upload screenshots
        if: failure()
        uses: actions/upload-artifact@v4
        with:
          name: screenshots-${{ matrix.browser }}
          path: test-results/
```

## Test Coverage Goals

| Category | Target Coverage |
|----------|----------------|
| **User Flows** | 100% critical paths |
| **Page Load** | All document types |
| **Interactions** | All buttons, links, forms |
| **Responsive** | Mobile, tablet, desktop |
| **Accessibility** | WCAG 2.1 AA compliance |
| **Performance** | Core Web Vitals passing |
| **Visual** | Key components snapshot tested |

## Monitoring & Reporting

### Test Dashboard
- Playwright HTML Reporter
- GitHub Actions workflow summary
- Visual regression diff viewer
- Performance trend charts

### Alerts
- Failed test notifications in Slack
- Performance degradation warnings
- Visual regression changes requiring approval

---

**Next:** Implementation begins!
