import { test, expect } from '@playwright/test';

test.describe('Dashboard Stats Fix', () => {
  test('should auto-select workspace and display correct stats', async ({ page }) => {
    // Enable console logging
    page.on('console', (msg) => {
      console.log(`[Browser ${msg.type()}]`, msg.text());
    });

    // Navigate to dashboard root (no query params)
    await page.goto('http://localhost:3000/', { waitUntil: 'networkidle' });

    // Wait a bit for React effects to run
    await page.waitForTimeout(2000);

    // Check current URL
    const currentUrl = page.url();
    console.log('Current URL:', currentUrl);

    // Take screenshot of initial state
    await page.screenshot({ path: 'dashboard-initial.png', fullPage: true });

    // Check if URL was updated with workspace parameter
    const hasWorkspaceParam = currentUrl.includes('workspace=');
    console.log('Has workspace param:', hasWorkspaceParam);

    // Get the stats from the dashboard
    const documentsText = await page.locator('text=Documents').locator('..').locator('..').textContent();
    const entitiesText = await page.locator('text=Entities').locator('..').locator('..').textContent();
    const relationshipsText = await page.locator('text=Relationships').locator('..').locator('..').textContent();

    console.log('Dashboard Stats:');
    console.log('  Documents:', documentsText);
    console.log('  Entities:', entitiesText);
    console.log('  Relationships:', relationshipsText);

    // Extract actual numbers
    const documentsMatch = documentsText?.match(/Documents\s*(\d+)/);
    const entitiesMatch = entitiesText?.match(/Entities\s*(\d+)/);
    const relationshipsMatch = relationshipsText?.match(/Relationships\s*(\d+)/);

    const documentsCount = documentsMatch ? parseInt(documentsMatch[1]) : 0;
    const entitiesCount = entitiesMatch ? parseInt(entitiesMatch[1]) : 0;
    const relationshipsCount = relationshipsMatch ? parseInt(relationshipsMatch[1]) : 0;

    console.log('Extracted Stats:');
    console.log('  Documents:', documentsCount);
    console.log('  Entities:', entitiesCount);
    console.log('  Relationships:', relationshipsCount);

    // Check workspace selector
    const workspaceSelector = await page.locator('[class*="workspace"]').first().textContent();
    console.log('Workspace Selector:', workspaceSelector);

    // Wait for network activity to settle
    await page.waitForTimeout(1000);

    // Take final screenshot
    await page.screenshot({ path: 'dashboard-final.png', fullPage: true });

    // Print final assessment
    console.log('\n=== ASSESSMENT ===');
    console.log('URL updated:', hasWorkspaceParam ? '✅' : '❌');
    console.log('Stats correct:', entitiesCount === 8 && relationshipsCount === 6 ? '✅' : '❌');
    console.log('Expected: Entities=8, Relationships=6');
    console.log(`Actual: Entities=${entitiesCount}, Relationships=${relationshipsCount}`);

    // Assertions (these will fail if the fix doesn't work)
    expect(hasWorkspaceParam, 'URL should include workspace parameter').toBeTruthy();
    expect(entitiesCount, 'Should have 8 entities from Apple-Sandbox-Guide-v1.0.md').toBe(8);
    expect(relationshipsCount, 'Should have 6 relationships from Apple-Sandbox-Guide-v1.0.md').toBe(6);
  });

  test('should check localStorage and Zustand store', async ({ page }) => {
    // Navigate to dashboard
    await page.goto('http://localhost:3000/', { waitUntil: 'networkidle' });
    await page.waitForTimeout(2000);

    // Check localStorage for tenant store
    const tenantStore = await page.evaluate(() => {
      const stored = localStorage.getItem('tenant-storage');
      return stored ? JSON.parse(stored) : null;
    });

    console.log('\n=== ZUSTAND STORE (localStorage) ===');
    console.log(JSON.stringify(tenantStore, null, 2));

    // Check if selectedWorkspaceId is set
    const selectedWorkspaceId = tenantStore?.state?.selectedWorkspaceId;
    console.log('\nSelected Workspace ID:', selectedWorkspaceId);
    console.log('Selected Workspace ID set:', selectedWorkspaceId ? '✅' : '❌');

    expect(selectedWorkspaceId, 'Should have a selected workspace ID').toBeTruthy();
  });

  test('should check API requests', async ({ page }) => {
    const requests: string[] = [];

    // Monitor network requests
    page.on('request', (request) => {
      if (request.url().includes('/api/v1/')) {
        requests.push(`${request.method()} ${request.url()}`);
      }
    });

    // Navigate to dashboard
    await page.goto('http://localhost:3000/', { waitUntil: 'networkidle' });
    await page.waitForTimeout(3000);

    console.log('\n=== API REQUESTS ===');
    requests.forEach((req) => console.log(req));

    // Check if stats endpoint was called
    const statsRequest = requests.find((req) => req.includes('/stats'));
    console.log('\nStats request made:', statsRequest ? '✅' : '❌');
    if (statsRequest) {
      console.log('Stats request:', statsRequest);
    }

    expect(statsRequest, 'Should make a request to stats endpoint').toBeTruthy();
  });
});
