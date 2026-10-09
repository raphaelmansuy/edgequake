import { test, expect } from '@playwright/test';
import { skipUnlessLiveStack } from './helpers/live-stack';

test.describe('SPEC-163 connections UI', () => {
  test('settings shows LLM connections card', async ({ page }) => {
    skipUnlessLiveStack();
    await page.goto('/settings');
    await expect(page.getByText('LLM connections')).toBeVisible();
    await expect(page.getByRole('button', { name: /Test connection/i })).toBeVisible();
    await expect(page.getByTestId('role-matrix-card')).toBeVisible();
    await expect(page.getByTestId('provider-status-hub')).toBeVisible();
  });
});
