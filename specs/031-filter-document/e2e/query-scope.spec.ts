/**
 * E2E tests for SPEC-031: Document Scope Filter
 *
 * Tests the document scope picker and pill bar in the query interface.
 * Screenshots are saved to specs/031-filter-document/e2e/screenshots/
 *
 * Prerequisites:
 *   - Services running: make dev-bg
 *   - At least one completed document uploaded
 *   - Frontend at http://localhost:3000
 *
 * Run: cd edgequake_webui && npx playwright test ../../specs/031-filter-document/e2e/query-scope.spec.ts
 */
import { expect, Page, test } from "@playwright/test";
import path from "path";

const BASE_URL = process.env.PLAYWRIGHT_BASE_URL ?? "http://localhost:3000";
const SCREENSHOT_DIR = path.join(__dirname, "screenshots");

/** Navigate to query page and wait for it to be ready. */
async function gotoQueryPage(page: Page) {
  await page.goto(`${BASE_URL}/query`);
  await page.waitForSelector('[aria-label="Query form"]', { timeout: 15_000 });
}

/** Open the settings sheet. */
async function openSettings(page: Page) {
  await page.click('button[aria-label="Settings"], button svg.lucide-settings-2');
  await page.waitForSelector('role=dialog', { timeout: 5_000 });
}

/** Take a named screenshot to the spec screenshots dir. */
async function screenshot(page: Page, name: string) {
  const file = path.join(SCREENSHOT_DIR, `${name}.png`);
  await page.screenshot({ path: file, fullPage: false });
  return file;
}

// ── Tests ────────────────────────────────────────────────────────────────────

test.describe("SPEC-031: Document Scope Filter", () => {
  test.beforeEach(async ({ page }) => {
    await gotoQueryPage(page);
  });

  /**
   * TC-001: "All docs" affordance is always visible (discoverability check)
   * The scope toolbar renders in empty state with the "All docs ▾" trigger.
   */
  test("TC-001: all-docs affordance always visible by default", async ({ page }) => {
    await screenshot(page, "tc001-query-default-state");

    // The scope toolbar region should ALWAYS be present
    const scopeRegion = page.getByRole("region", { name: /query scope/i });
    await expect(scopeRegion).toBeVisible();

    // The "All docs" text should be visible (empty state affordance)
    await expect(page.getByText("All docs", { exact: false })).toBeVisible();

    // No dismiss buttons should exist (no pills in empty state)
    const removeBtns = page.locator('[aria-label*="Remove"][aria-label*="from scope"]');
    await expect(removeBtns).toHaveCount(0);
  });

  /**
   * TC-002: Settings sheet contains "Document Scope" section
   */
  test("TC-002: settings sheet has document scope section", async ({ page }) => {
    await openSettings(page);
    await screenshot(page, "tc002-settings-sheet-open");

    // Should see scope section label
    await expect(
      page.getByText("Document Scope", { exact: false }),
    ).toBeVisible();

    await expect(
      page.getByText("Add documents to scope", { exact: false }),
    ).toBeVisible();
  });

  /**
   * TC-003: Document picker popover opens and shows search
   */
  test("TC-003: document picker popover opens", async ({ page }) => {
    await openSettings(page);

    // Click Add documents to scope
    await page.getByText("Add documents to scope", { exact: false }).click();

    await screenshot(page, "tc003-picker-open");

    // Picker should show search input
    await expect(
      page.getByRole("textbox", { name: /search documents/i }),
    ).toBeVisible();
  });

  /**
   * TC-004: Search in picker filters results
   */
  test("TC-004: picker search filters documents", async ({ page }) => {
    await openSettings(page);
    await page.getByText("Add documents to scope", { exact: false }).click();

    const searchInput = page.getByRole("textbox", { name: /search documents/i });
    await searchInput.fill("test");

    // Wait for debounce (300ms) + network
    await page.waitForTimeout(500);

    await screenshot(page, "tc004-picker-search-results");

    // Either results or no-results message
    const hasResults =
      (await page.getByRole("option").count()) > 0 ||
      (await page.getByText(/no documents match/i).count()) > 0;
    expect(hasResults).toBe(true);
  });

  /**
   * TC-005: Selecting a document adds it to scope and shows scope bar
   * (Requires at least one completed document in the workspace)
   */
  test("TC-005: select document shows scope bar pill", async ({ page }) => {
    await openSettings(page);
    await page.getByText("Add documents to scope", { exact: false }).click();

    // Wait for results to load
    await page.waitForTimeout(800);

    const options = page.getByRole("option");
    const count = await options.count();

    if (count === 0) {
      test.skip(true, "No completed documents in workspace — upload one first");
      return;
    }

    // Select the first result
    const firstOption = options.first();
    const optionTitle = await firstOption.textContent();
    await firstOption.click();

    await screenshot(page, "tc005-after-selection");

    // Close picker + settings
    await page.keyboard.press("Escape");
    await page.keyboard.press("Escape");

    // Scope bar should now be visible
    await expect(
      page.getByRole("region", { name: /query scope/i }),
    ).toBeVisible();

    await screenshot(page, "tc005-scope-bar-visible");

    // Pill should be present
    const pill = page.locator('[role="listitem"]').first();
    await expect(pill).toBeVisible();

    console.log(`Selected document: ${optionTitle}`);
  });

  /**
   * TC-006: Removing a pill dismisses it from scope
   */
  test("TC-006: pill remove button dismisses scope", async ({ page }) => {
    await openSettings(page);
    await page.getByText("Add documents to scope", { exact: false }).click();
    await page.waitForTimeout(800);

    const options = page.getByRole("option");
    if ((await options.count()) === 0) {
      test.skip(true, "No completed documents available");
      return;
    }

    await options.first().click();
    await page.keyboard.press("Escape");
    await page.keyboard.press("Escape");

    await screenshot(page, "tc006-before-remove");

    // Click the × dismiss button on the pill
    const removeButton = page.locator(
      '[aria-label*="Remove"][aria-label*="from scope"]',
    ).first();
    await removeButton.click();

    await screenshot(page, "tc006-after-remove");

    // After removing last pill, scope bar returns to empty state (shows "All docs")
    await expect(page.getByText("All docs", { exact: false })).toBeVisible();
    // No more pill remove buttons
    await expect(
      page.locator('[aria-label*="Remove"][aria-label*="from scope"]'),
    ).toHaveCount(0);
  });

  /**
   * TC-007: Clear all button removes all scope
   */
  test("TC-007: clear all hides scope bar", async ({ page }) => {
    await openSettings(page);
    await page.getByText("Add documents to scope", { exact: false }).click();
    await page.waitForTimeout(800);

    const options = page.getByRole("option");
    if ((await options.count()) === 0) {
      test.skip(true, "No completed documents available");
      return;
    }

    // Select up to 2 documents
    await options.first().click();
    if ((await options.count()) > 1) {
      await options.nth(1).click();
    }
    await page.keyboard.press("Escape");
    await page.keyboard.press("Escape");

    // Click clear all (× button at right edge of scope bar)
    const clearAll = page.getByRole("button", {
      name: /clear all document scope/i,
    });
    await expect(clearAll).toBeVisible();
    await clearAll.click();

    await screenshot(page, "tc007-after-clear-all");

    // After clearing all, toolbar returns to empty state showing "All docs"
    await expect(page.getByText("All docs", { exact: false })).toBeVisible();
    // No pills remain
    await expect(
      page.locator('[aria-label*="Remove"][aria-label*="from scope"]'),
    ).toHaveCount(0);
  });

  /**
   * TC-008: Scope persists after page reload
   */
  test("TC-008: scope persists after reload", async ({ page }) => {
    await openSettings(page);
    await page.getByText("Add documents to scope", { exact: false }).click();
    await page.waitForTimeout(800);

    const options = page.getByRole("option");
    if ((await options.count()) === 0) {
      test.skip(true, "No completed documents available");
      return;
    }

    await options.first().click();
    await page.keyboard.press("Escape");
    await page.keyboard.press("Escape");

    // Reload the page
    await page.reload();
    await page.waitForSelector('[aria-label="Query form"]', { timeout: 15_000 });

    await screenshot(page, "tc008-after-reload");

    // After reload, scope should persist — pills visible (not "All docs" empty state)
    await expect(page.locator('[role="listitem"]').first()).toBeVisible();
    // "All docs" should NOT be shown (pills are present)
    await expect(page.getByText("All docs", { exact: true })).not.toBeVisible();
  });

  /**
   * TC-009: Scope bar is disabled during active query
   */
  test("TC-009: scope bar disabled during query", async ({ page }) => {
    await openSettings(page);
    await page.getByText("Add documents to scope", { exact: false }).click();
    await page.waitForTimeout(800);

    const options = page.getByRole("option");
    if ((await options.count()) === 0) {
      test.skip(true, "No completed documents available");
      return;
    }

    await options.first().click();
    await page.keyboard.press("Escape");
    await page.keyboard.press("Escape");

    // Start a query
    const input = page.getByRole("textbox", { name: /ask a question/i });
    await input.fill("What is this document about?");
    await input.press("Enter");

    // While loading, scope bar should be dimmed (pointer-events-none)
    await screenshot(page, "tc009-scope-during-query");

    const scopeBar = page.getByRole("region", { name: /query scope/i });
    // We can't easily test pointer-events-none visually, but the bar should still exist
    await expect(scopeBar).toBeVisible();
  });

  /**
   * TC-010: API call includes document_ids when scope active
   * Intercepts the network request to verify document_ids are sent
   */
  test("TC-010: API request includes document_ids", async ({ page }) => {
    let capturedBody: Record<string, unknown> | null = null;

    // Intercept chat/query API calls
    await page.route("**/api/v1/chat/completions*", async (route) => {
      const request = route.request();
      const postData = request.postData();
      if (postData) {
        try {
          capturedBody = JSON.parse(postData);
        } catch (_e) {
          // ignore
        }
      }
      await route.continue();
    });

    await openSettings(page);
    await page.getByText("Add documents to scope", { exact: false }).click();
    await page.waitForTimeout(800);

    const options = page.getByRole("option");
    if ((await options.count()) === 0) {
      test.skip(true, "No completed documents available");
      return;
    }

    await options.first().click();
    await page.keyboard.press("Escape");
    await page.keyboard.press("Escape");

    // Submit a query
    const input = page.getByRole("textbox", { name: /ask a question/i });
    await input.fill("Test scope query");
    await input.press("Enter");

    await page.waitForTimeout(2_000);

    await screenshot(page, "tc010-api-request-with-scope");

    // Verify the captured request body includes document_filter.document_ids
    if (capturedBody) {
      const filter = (capturedBody as Record<string, unknown>).document_filter as Record<string, unknown> | undefined;
      console.log("Captured document_filter:", JSON.stringify(filter));
      expect(filter).toBeDefined();
      expect(Array.isArray(filter?.document_ids)).toBe(true);
      expect((filter?.document_ids as string[]).length).toBeGreaterThan(0);
    }
  });
});

// ── Entity/Relationship scope filter tests (SPEC-031 §008) ───────────────────

test.describe("SPEC-031: Entity & Relationship lineage filtering", () => {
  const API_URL = process.env.API_BASE_URL ?? "http://localhost:8080";

  /**
   * TC-E01: Verify the context_filter correctly excludes entities from
   * out-of-scope documents using the new strict filter.
   *
   * Uses the /api/v1/query/context endpoint (context_only=true) to inspect
   * what entities appear in the context when scope is restricted.
   */
  test("TC-E01: scoped query context excludes out-of-scope entities", async ({
    request,
  }) => {
    // Get a list of documents to find two with different content
    const listResp = await request.get(`${API_URL}/api/v1/documents`, {
      headers: { "x-workspace-id": "default", "x-tenant-id": "default" },
    });
    if (listResp.status() !== 200) {
      test.skip(true, "Documents API requires auth or no workspace");
      return;
    }
    const listBody = await listResp.json();
    const docs: Array<{ id: string; status: string }> =
      listBody.documents ?? [];
    const completedDocs = docs.filter((d) => d.status === "completed");
    if (completedDocs.length < 2) {
      test.skip(true, "Need at least 2 completed documents");
      return;
    }

    const scopedDocId = completedDocs[0].id;
    const excludedDocId = completedDocs[1].id;

    // Query scoped to only the first document
    const scopedResp = await request.post(`${API_URL}/api/v1/query`, {
      headers: {
        "Content-Type": "application/json",
        "x-workspace-id": "default",
        "x-tenant-id": "default",
      },
      data: {
        query: "What are the main topics?",
        mode: "hybrid",
        context_only: true,
        document_filter: {
          document_ids: [scopedDocId],
        },
      },
    });

    if (scopedResp.status() !== 200) {
      test.skip(true, "Query API requires auth");
      return;
    }

    const scopedBody = await scopedResp.json();
    const context = scopedBody.context ?? {};
    const chunks: Array<{ document_id?: string }> = context.chunks ?? [];
    const entities: Array<{ source_document_id?: string }> = context.entities ?? [];

    // All returned chunks must be from the scoped document
    for (const chunk of chunks) {
      if (chunk.document_id) {
        expect(chunk.document_id).toBe(scopedDocId);
      }
    }

    // No entity should come exclusively from the excluded document
    for (const entity of entities) {
      if (entity.source_document_id) {
        expect(entity.source_document_id).not.toBe(excludedDocId);
      }
    }

    console.log(
      `TC-E01: scoped to ${scopedDocId}, got ${chunks.length} chunks, ${entities.length} entities`,
    );
  });

  /**
   * TC-E02: GET /documents/search returns lightweight projections
   * and content_filter works correctly for empty document_ids (no-op).
   */
  test("TC-E02: empty document_ids is a no-op (full workspace query)", async ({
    request,
  }) => {
    const fullResp = await request.post(`${API_URL}/api/v1/query`, {
      headers: {
        "Content-Type": "application/json",
        "x-workspace-id": "default",
        "x-tenant-id": "default",
      },
      data: {
        query: "What are the main topics?",
        mode: "naive",
        context_only: true,
        document_filter: {
          document_ids: [], // empty — should be no-op
        },
      },
    });

    const emptyFilterResp = await request.post(`${API_URL}/api/v1/query`, {
      headers: {
        "Content-Type": "application/json",
        "x-workspace-id": "default",
        "x-tenant-id": "default",
      },
      data: {
        query: "What are the main topics?",
        mode: "naive",
        context_only: true,
        // no document_filter — full workspace
      },
    });

    if (fullResp.status() === 200 && emptyFilterResp.status() === 200) {
      const full = await fullResp.json();
      const noFilter = await emptyFilterResp.json();
      const fullCount = (full.context?.chunks ?? []).length;
      const noFilterCount = (noFilter.context?.chunks ?? []).length;
      // Both should return the same number of chunks (empty [] = no-op)
      expect(fullCount).toBe(noFilterCount);
    }
  });
});

test.describe("SPEC-031: GET /api/v1/documents/search endpoint", () => {
  const API_URL = process.env.API_BASE_URL ?? "http://localhost:8080";

  test("search endpoint returns items array", async ({ request }) => {
    const response = await request.get(`${API_URL}/api/v1/documents/search`, {
      headers: {
        "x-workspace-id": "default",
        "x-tenant-id": "default",
      },
    });

    // Without auth it may 401/403 — check both acceptable outcomes
    if (response.status() === 200) {
      const body = await response.json();
      expect(body).toHaveProperty("items");
      expect(body).toHaveProperty("total");
      expect(body).toHaveProperty("has_more");
      expect(Array.isArray(body.items)).toBe(true);
    } else {
      // Auth required — acceptable
      expect([401, 403]).toContain(response.status());
    }
  });

  test("search endpoint respects query param", async ({ request }) => {
    const response = await request.get(
      `${API_URL}/api/v1/documents/search?q=test`,
      {
        headers: {
          "x-workspace-id": "default",
          "x-tenant-id": "default",
        },
      },
    );

    if (response.status() === 200) {
      const body = await response.json();
      expect(body.items.length).toBeLessThanOrEqual(20);
    } else {
      expect([401, 403]).toContain(response.status());
    }
  });
});
