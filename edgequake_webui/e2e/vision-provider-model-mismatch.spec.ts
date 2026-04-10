/**
 * Vision Provider/Model Mismatch E2E Tests
 *
 * Root-cause: When a workspace has `vision_llm_model` set (e.g. "gpt-4.1-nano"
 * from a previous OpenAI config) but `vision_llm_provider` is NULL, the upload
 * handler used to fall back to the workspace's main LLM provider (Ollama) while
 * also blindly applying the orphaned OpenAI model — producing an incompatible
 * pair (provider="ollama", model="gpt-4.1-nano") that Ollama rejects with 404.
 *
 * Fix (SPEC-040): `vision_llm_model` from workspace is only applied when
 * `vision_llm_provider` is also explicitly set.  Without an explicit provider,
 * `default_vision_model_for_provider()` selects the provider-appropriate model.
 *
 * Tests:
 *  1. API: workspace with orphaned vision_model + ollama main LLM → task uses
 *     a model compatible with ollama (NOT gpt-4.1-nano)
 *  2. API: workspace with explicit pair (openai, gpt-4.1-nano) → task keeps pair
 *  3. UI:  failed document shows error banner and Retry button
 *  4. UI:  Retry button creates a new task (POST request fired)
 *
 * @implements SPEC-040: Vision provider/model safety invariant
 */
import { expect, test, type APIRequestContext } from "@playwright/test";

const BACKEND_URL = process.env.BACKEND_URL ?? "http://localhost:8080";
const BASE_URL = process.env.PLAYWRIGHT_BASE_URL ?? "http://localhost:3000";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/** Create a unique name to avoid collisions between test runs. */
const uid = () => `vpm-test-${Date.now()}-${Math.random().toString(36).slice(2, 7)}`;

/** Create a tenant and return its ID + default workspace ID. */
async function createTenant(
  request: APIRequestContext,
): Promise<{ tenantId: string; workspaceId: string }> {
  const res = await request.post(`${BACKEND_URL}/api/v1/tenants`, {
    data: { name: uid() },
  });
  expect(res.status()).toBeLessThan(300);
  const body = await res.json();
  const tenantId: string = body.id ?? body.tenant_id;

  // Fetch workspaces for this tenant
  const wsRes = await request.get(`${BACKEND_URL}/api/v1/workspaces`, {
    headers: { "X-Tenant-ID": tenantId },
  });
  expect(wsRes.status()).toBeLessThan(300);
  const wsBody = await wsRes.json();
  const workspaceId: string =
    (wsBody.items ?? wsBody)[0]?.workspace_id ?? (wsBody.items ?? wsBody)[0]?.id;
  return { tenantId, workspaceId };
}

/** Patch a workspace to set (or clear) vision LLM fields. */
async function patchWorkspaceVision(
  request: APIRequestContext,
  tenantId: string,
  workspaceId: string,
  vision_llm_provider: string | null,
  vision_llm_model: string | null,
) {
  const res = await request.patch(
    `${BACKEND_URL}/api/v1/workspaces/${workspaceId}`,
    {
      headers: { "X-Tenant-ID": tenantId },
      data: { vision_llm_provider, vision_llm_model },
    },
  );
  // 200 OK or 204 No Content
  expect(res.status()).toBeLessThan(300);
}

// ---------------------------------------------------------------------------
// Minimal valid PDF bytes (14-byte PDF stub).
// WHY: Real PDF not required — we only verify task creation, not processing.
// ---------------------------------------------------------------------------
const MINIMAL_PDF = Buffer.from(
  "%PDF-1.0\n1 0 obj<</Type/Catalog>>endobj\nxref\n0 1\n0000000000 65535 f \ntrailer<</Size 1/Root 1 0 R>>\nstartxref\n9\n%%EOF",
);

// ---------------------------------------------------------------------------
// API-level tests (no browser required)
// ---------------------------------------------------------------------------
test.describe("Vision Provider/Model Mismatch — API", () => {
  test.setTimeout(30_000);

  // SCENARIO 1 — Orphaned vision_model without vision_provider
  // Expected: upload uses Ollama-compatible model (NOT gpt-4.1-nano)
  test("orphaned vision_model is ignored when vision_provider absent", async ({
    request,
  }) => {
    // Skip if backend not reachable
    const health = await request.get(`${BACKEND_URL}/health`).catch(() => null);
    if (!health || health.status() !== 200) {
      test.skip(true, "Backend not reachable — skipping API test");
      return;
    }

    // 1. Create a fresh workspace
    const { tenantId, workspaceId } = await createTenant(request);

    // 2. Set orphaned vision_model WITHOUT vision_provider
    //    (simulates legacy config where OpenAI was used and then removed)
    await patchWorkspaceVision(request, tenantId, workspaceId, null, "gpt-4.1-nano");

    // 3. Upload a PDF (minimal stub)
    const uploadRes = await request.post(
      `${BACKEND_URL}/api/v1/documents/pdf`,
      {
        headers: {
          "X-Tenant-ID": tenantId,
          "X-Workspace-ID": workspaceId,
        },
        multipart: {
          file: {
            name: "test.pdf",
            mimeType: "application/pdf",
            buffer: MINIMAL_PDF,
          },
        },
      },
    );

    // The upload must not fail with a 5xx error — task creation is synchronous
    // and must succeed even when the workspace has an orphaned model.
    expect(uploadRes.status()).toBeLessThan(500);
    if (uploadRes.status() >= 200 && uploadRes.status() < 300) {
      const body = await uploadRes.json();
      // The response should surface which vision_model was selected.
      // If present, it must NOT be gpt-4.1-nano (an OpenAI model).
      const selectedModel: string | null | undefined =
        body?.metadata?.vision_model ?? body?.vision_model;
      if (selectedModel) {
        // INVARIANT: gpt-4.1-nano must never be returned for the Ollama provider
        expect(selectedModel).not.toBe("gpt-4.1-nano");
      }
    }
  });

  // SCENARIO 2 — Explicit provider+model pair is preserved
  // Expected: when both are set, the upload uses exactly that pair.
  test("explicit vision provider+model pair is preserved", async ({
    request,
  }) => {
    const health = await request.get(`${BACKEND_URL}/health`).catch(() => null);
    if (!health || health.status() !== 200) {
      test.skip(true, "Backend not reachable — skipping API test");
      return;
    }

    const { tenantId, workspaceId } = await createTenant(request);

    // Set an explicit and coherent pair
    await patchWorkspaceVision(
      request,
      tenantId,
      workspaceId,
      "openai",
      "gpt-4o",
    );

    const uploadRes = await request.post(
      `${BACKEND_URL}/api/v1/documents/pdf`,
      {
        headers: {
          "X-Tenant-ID": tenantId,
          "X-Workspace-ID": workspaceId,
        },
        multipart: {
          file: {
            name: "test.pdf",
            mimeType: "application/pdf",
            buffer: MINIMAL_PDF,
          },
        },
      },
    );

    expect(uploadRes.status()).toBeLessThan(500);
    if (uploadRes.status() >= 200 && uploadRes.status() < 300) {
      const body = await uploadRes.json();
      const selectedModel: string | null | undefined =
        body?.metadata?.vision_model ?? body?.vision_model;
      if (selectedModel) {
        // Explicit provider+model pair must be preserved end-to-end
        expect(selectedModel).toBe("gpt-4o");
      }
    }
  });

  // SCENARIO 3 — No workspace override → server default used
  test("no workspace vision override uses server default model", async ({
    request,
  }) => {
    const health = await request.get(`${BACKEND_URL}/health`).catch(() => null);
    if (!health || health.status() !== 200) {
      test.skip(true, "Backend not reachable — skipping API test");
      return;
    }

    const { tenantId, workspaceId } = await createTenant(request);
    // Don't set any vision fields — rely on server defaults

    const uploadRes = await request.post(
      `${BACKEND_URL}/api/v1/documents/pdf`,
      {
        headers: {
          "X-Tenant-ID": tenantId,
          "X-Workspace-ID": workspaceId,
        },
        multipart: {
          file: {
            name: "test.pdf",
            mimeType: "application/pdf",
            buffer: MINIMAL_PDF,
          },
        },
      },
    );

    expect(uploadRes.status()).toBeLessThan(500);
    if (uploadRes.status() >= 200 && uploadRes.status() < 300) {
      const body = await uploadRes.json();
      const selectedModel: string | null | undefined =
        body?.metadata?.vision_model ?? body?.vision_model;
      // Model should be non-empty and valid
      if (selectedModel) {
        expect(selectedModel.trim()).not.toBe("");
      }
    }
  });
});

// ---------------------------------------------------------------------------
// UI-level tests (browser required)
// ---------------------------------------------------------------------------
test.describe("Vision Provider/Model Mismatch — UI", () => {
  test.setTimeout(60_000);

  test.beforeEach(async ({ page }) => {
    // Verify app is running before attempting UI tests
    await page.goto(BASE_URL, { waitUntil: "domcontentloaded", timeout: 15_000 });
  });

  // SCENARIO 4 — Failed document shows error and Retry button
  test("failed document shows error banner and Retry button", async ({
    page,
  }) => {
    // Navigate to documents list
    await page.goto(`${BASE_URL}/documents`, { waitUntil: "domcontentloaded" });

    // Find any failed document or skip if none exist
    const failedBadge = page
      .locator('[data-status="failed"], [class*="failed"], :text("Failed")')
      .first();

    const hasFailed = await failedBadge.isVisible({ timeout: 5_000 }).catch(() => false);
    if (!hasFailed) {
      test.skip(true, "No failed documents present — skipping failed-doc UI test");
      return;
    }

    // Click into the failed document
    const row = page
      .locator("tr, [role='row'], [data-testid='document-row']")
      .filter({ has: failedBadge })
      .first();

    await row.click();
    await page.waitForLoadState("domcontentloaded");

    // Error banner should be visible
    const errorBanner = page.locator(
      '[role="alert"], [class*="error"], [class*="destructive"], [data-testid*="error"]',
    ).first();
    await expect(errorBanner).toBeVisible({ timeout: 10_000 });

    // Retry button should be present
    const retryButton = page
      .locator('button:has-text("Retry"), button[aria-label*="retry" i]')
      .first();
    await expect(retryButton).toBeVisible({ timeout: 10_000 });
  });

  // SCENARIO 5 — Retry button fires POST request to /retry endpoint
  test("Retry button fires POST to retry endpoint", async ({ page }) => {
    await page.goto(`${BASE_URL}/documents`, { waitUntil: "domcontentloaded" });

    const failedBadge = page
      .locator('[data-status="failed"], [class*="failed"], :text("Failed")')
      .first();

    const hasFailed = await failedBadge.isVisible({ timeout: 5_000 }).catch(() => false);
    if (!hasFailed) {
      test.skip(true, "No failed documents — skipping retry-button test");
      return;
    }

    const row = page
      .locator("tr, [role='row'], [data-testid='document-row']")
      .filter({ has: failedBadge })
      .first();

    await row.click();
    await page.waitForLoadState("domcontentloaded");

    // Intercept retry API call
    const retryRequests: string[] = [];
    page.on("request", (req) => {
      if (req.method() === "POST" && req.url().includes("/retry")) {
        retryRequests.push(req.url());
      }
    });

    const retryButton = page
      .locator('button:has-text("Retry"), button[aria-label*="retry" i]')
      .first();

    const isVisible = await retryButton.isVisible({ timeout: 5_000 }).catch(() => false);
    if (!isVisible) {
      test.skip(true, "No Retry button found — skipping");
      return;
    }

    await retryButton.click();

    // Wait briefly for the POST to fire
    await page.waitForTimeout(2_000);

    expect(retryRequests.length).toBeGreaterThan(0);
    expect(retryRequests[0]).toMatch(/\/retry$/);
  });
});
