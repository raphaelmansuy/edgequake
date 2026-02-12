/**
 * E2E Test Helpers — Utilities for running tests against a live EdgeQuake backend.
 *
 * WHY: E2E tests validate the SDK against a real server, catching issues
 * that unit tests with mock transport cannot (serialization, auth, timing).
 *
 * Usage:
 *   EDGEQUAKE_E2E_URL=http://localhost:8080 npm test -- tests/e2e/
 *
 * Prerequisites:
 *   make dev  # Start database + backend + frontend
 */

import { EdgeQuake } from "../../src/index.js";

/** URL of the running EdgeQuake backend, or undefined to skip E2E tests */
export const E2E_URL = process.env.EDGEQUAKE_E2E_URL;

/** API key for authenticated endpoints (optional — some endpoints are public) */
export const E2E_API_KEY = process.env.EDGEQUAKE_API_KEY;

/** Workspace ID for multi-tenant tests.
 * WHY: Chat API rejects non-UUID workspace IDs. Leave empty to use server default. */
export const E2E_WORKSPACE = process.env.EDGEQUAKE_WORKSPACE ?? "";

/** Tenant ID for multi-tenant E2E tests.
 * WHY: Default migration-created tenant works for all conversation/folder ops */
export const E2E_TENANT_ID =
  process.env.EDGEQUAKE_TENANT_ID || "00000000-0000-0000-0000-000000000002";

/** User ID for user-scoped E2E tests.
 * WHY: Default migration-created user works for all user-scoped ops */
export const E2E_USER_ID =
  process.env.EDGEQUAKE_USER_ID || "00000000-0000-0000-0000-000000000001";

/** Whether E2E tests should run */
export const E2E_ENABLED = !!E2E_URL;

/**
 * Create a pre-configured EdgeQuake client for E2E tests.
 * Returns undefined if E2E_URL is not set.
 */
export function createE2EClient(): EdgeQuake | undefined {
  if (!E2E_URL) return undefined;
  return new EdgeQuake({
    baseUrl: E2E_URL,
    apiKey: E2E_API_KEY,
    tenantId: E2E_TENANT_ID,
    userId: E2E_USER_ID,
    workspaceId: E2E_WORKSPACE,
  });
}

/**
 * Wait for a condition to become true, polling at intervals.
 * Useful for waiting on async processing (document ingestion, entity extraction).
 */
export async function waitFor(
  condition: () => Promise<boolean>,
  options: { timeoutMs?: number; intervalMs?: number; label?: string } = {},
): Promise<void> {
  const {
    timeoutMs = 30_000,
    intervalMs = 1_000,
    label = "condition",
  } = options;
  const deadline = Date.now() + timeoutMs;

  while (Date.now() < deadline) {
    if (await condition()) return;
    await sleep(intervalMs);
  }

  throw new Error(`Timed out waiting for ${label} after ${timeoutMs}ms`);
}

/** Sleep for a given number of milliseconds */
export function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

/**
 * Generate a unique test identifier for resource isolation.
 * WHY: Prevents test pollution when multiple test runs hit the same backend.
 */
export function testId(prefix = "sdk-e2e"): string {
  const ts = Date.now().toString(36);
  const rand = Math.random().toString(36).slice(2, 6);
  return `${prefix}-${ts}-${rand}`;
}
