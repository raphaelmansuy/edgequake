// E2E tests for workspace/tenant default selection
import { test, expect } from '@playwright/test';

test.describe('Workspace/Tenant Default Selection', () => {
  test.beforeEach(async ({ page }) => {
    // Clear localStorage before each test
    await page.goto('/');
    await page.evaluate(() => localStorage.clear());
  });

  test('first-time user sees workspace selector initialized', async ({ page }) => {
    await page.goto('/');
    
    // Wait for workspace selector to be visible
    await expect(page.getByTestId('workspace-selector')).toBeVisible({ timeout: 10000 });
    
    // Should auto-select first available workspace
    const selectorText = await page.getByTestId('workspace-selector').textContent();
    expect(selectorText).toBeTruthy();
    expect(selectorText).not.toContain('Select workspace');
  });

  test('returning user automatically enters last workspace', async ({ page, context }) => {
    // Set up a returning user context
    await context.addCookies([
      {
        name: 'edgequake-tenant',
        value: JSON.stringify({
          state: {
            selectedTenantId: 'test-tenant-id',
            selectedWorkspaceId: 'test-workspace-id',
          },
        }),
        domain: 'localhost',
        path: '/',
      },
    ]);

    await page.goto('/');
    
    // Should load directly into the app
    await expect(page).toHaveURL(/\/(dashboard|documents|query|graph)/);
    
    // Workspace selector should show selected workspace
    await expect(page.getByTestId('workspace-selector')).toBeVisible();
  });

  test('can manually switch workspace', async ({ page }) => {
    await page.goto('/');
    
    // Wait for initialization
    await page.waitForLoadState('networkidle');
    
    // Click workspace selector
    await page.getByTestId('workspace-selector').click();
    
    // Should show dropdown with workspaces
    await expect(page.getByRole('menuitem').first()).toBeVisible();
    
    // Click a workspace
    await page.getByRole('menuitem').first().click();
    
    // Selector should update
    await expect(page.getByTestId('workspace-selector')).toBeVisible();
  });
});
