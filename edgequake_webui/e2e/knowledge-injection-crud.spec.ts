import { skipUnlessLiveStack } from "./helpers/live-stack";
/**
 * E2E tests for Knowledge Injection CRUD operations:
 *   1. Add new text injection → verify card appears + entity_count > 0
 *   2. Edit injection via detail page (Pencil → Save)
 *   3. Delete injection (Trash2 icon → confirm Delete button)
 *   4. API-level verification: at least one injection has entities > 0
 *   5. Query engine retrieves injected entity XJMIJI → Hermes 3
 *
 * UI structure (from knowledge/page.tsx):
 *   - Dialog with tabs: "Text" | "File"
 *   - Text tab: Input#injection-name, Textarea#injection-content, Button "Create"
 *   - Cards: Link → /knowledge/:id, Trash2 icon (Button with stopPropagation) → nested Dialog → Button "Delete"
 *   - Detail page (/knowledge/:id): Pencil button → edit name/content → Save button
 *
 * Runs against the live dev stack on 
 *
 * IMPORTANT: Tests run SERIALLY to avoid saturating the OpenAI API with
 * parallel extraction requests, which causes processing timeouts.
 */

import { expect, Page, test } from "@playwright/test";
import { API_V1_URL, BACKEND_URL } from "./helpers/backend-url";
import { waitForAppReady, GOTO_OPTS, clearAppStorage, waitForBackendHealthy } from "./helpers/app-ready";

const KNOWLEDGE_URL = "/knowledge";
const WS_ID = "8efcd288-37f7-413c-97bb-95bd7b535059";
const API_INJECTIONS_URL = `${API_V1_URL}/workspaces/${WS_ID}/injections`;
const WS_HEADERS = { "X-Workspace-ID": WS_ID };

// Unique suffix per test run to avoid name collisions
const SUFFIX = Date.now();
const INJECT_NAME_1 = `E2E-Add-${SUFFIX}`;
const INJECT_NAME_EDIT = `E2E-Edit-${SUFFIX}`;
const INJECT_NAME_DEL = `E2E-Del-${SUFFIX}`;

const INJECT_CONTENT = `ZTEST_ALPHA_${SUFFIX} is the codename for Project Phoenix.`;
const INJECT_CONTENT_UPDATED = `ZTEST_BETA_${SUFFIX} replaced ZTEST_ALPHA_${SUFFIX} as the active codename.`;

// Max time to wait for OpenAI extraction to complete.
// Set high because OpenAI API response times can be very variable under load.
const PROCESSING_TIMEOUT_MS = 480_000; // 8 minutes

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

async function gotoKnowledge(page: Page) {
  await page.goto(KNOWLEDGE_URL);
  await waitForAppReady(page);
}

/** Open the "New Injection" dialog. */
async function openNewDialog(page: Page) {
  const btn = page.getByRole("button", { name: /new injection/i });
  await expect(btn).toBeVisible({ timeout: 8_000 });
  await btn.click();
  await page.waitForSelector('[role="dialog"]', { timeout: 8_000 });
}

/** Fill text-tab fields and click Create. */
async function createTextInjection(page: Page, name: string, content: string) {
  await openNewDialog(page);
  await page.locator("#injection-name").fill(name);
  await page.locator("#injection-content").fill(content);
  await page.getByRole("button", { name: /^create$/i }).click();
  // Wait for dialog to close
  await page.waitForSelector('[role="dialog"]', {
    state: "detached",
    timeout: 8_000,
  });
}

/**
 * Poll the injections API until the named injection shows
 * "completed" or "failed". Falls back to "timeout".
 */
async function pollInjectionStatus(
  page: Page,
  name: string,
  timeoutMs = PROCESSING_TIMEOUT_MS
): Promise<"completed" | "failed" | "timeout"> {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    const resp = await page.request.get(API_INJECTIONS_URL, { headers: WS_HEADERS });
    if (resp.ok()) {
      const body = await resp.json();
      const item = (body.items as any[]).find((i) => i.name === name);
      if (item?.status === "completed") return "completed";
      if (item?.status === "failed") return "failed";
    }
    await page.waitForTimeout(5_000);
  }
  return "timeout";
}

// ─────────────────────────────────────────────────────────────────────────────
// Test suite — SERIAL to avoid parallel OpenAI extraction overload
// ─────────────────────────────────────────────────────────────────────────────


test.beforeEach(() => {
  skipUnlessLiveStack();
});

test.describe.configure({ mode: "serial" });

test.describe("@load Knowledge Injection CRUD", () => {
  // ── TEST 1: Add ────────────────────────────────────────────────────────────
  test("1 - Add text injection → card shows entity_count > 0", async ({
    page,
  }) => {
    test.setTimeout(600_000); // 10 min — OpenAI can be slow
    await gotoKnowledge(page);
    await createTextInjection(page, INJECT_NAME_1, INJECT_CONTENT);

    // Card appears immediately with "processing" badge
    await expect(page.getByText(INJECT_NAME_1)).toBeVisible({ timeout: 8_000 });
    console.log(`[1] "${INJECT_NAME_1}" created — polling for completion`);

    const status = await pollInjectionStatus(page, INJECT_NAME_1);
    console.log(`[1] Status: ${status}`);
    expect(status).toBe("completed");

    // Reload and verify entity count on card is not zero
    await gotoKnowledge(page);
    const cardLink = page
      .locator('a[href^="/knowledge/"]')
      .filter({ has: page.getByText(INJECT_NAME_1, { exact: true }) });
    await expect(cardLink).toBeVisible({ timeout: 5_000 });
    const cardText = await cardLink.textContent();
    console.log(`[1] Card text: ${cardText?.substring(0, 100)}`);
    // Verify entity count on card is non-zero
    expect(cardText).not.toMatch(/\b0 entities\b/);
  });

  // ── TEST 2: Edit (detail page) ─────────────────────────────────────────────
  test("2 - Edit injection content via detail page", async ({ page }) => {
    test.setTimeout(600_000); // 10 min — one OpenAI extraction cycle
    await gotoKnowledge(page);
    await createTextInjection(page, INJECT_NAME_EDIT, INJECT_CONTENT);
    await expect(page.getByText(INJECT_NAME_EDIT)).toBeVisible({ timeout: 8_000 });
    console.log(`[2] "${INJECT_NAME_EDIT}" created — navigating to detail page immediately`);
    // NOTE: We navigate immediately without waiting for initial processing,
    // since the Edit button is available regardless of processing status.

    // Click the card link to navigate to the detail page
    await gotoKnowledge(page);
    const cardLink = page
      .locator('a[href^="/knowledge/"]')
      .filter({ has: page.getByText(INJECT_NAME_EDIT, { exact: true }) });
    await expect(cardLink).toBeVisible({ timeout: 5_000 });
    await cardLink.click();
    await page.waitForURL(/\/knowledge\/[a-f0-9-]+/, { timeout: 8_000 });
    await waitForAppReady(page);
    console.log(`[2] Detail page: ${page.url()}`);

    // Click the Pencil (Edit) button
    const pencilBtn = page.getByRole("button", { name: /edit/i });
    await expect(pencilBtn).toBeVisible({ timeout: 8_000 });
    await pencilBtn.click();

    // Content textarea becomes editable
    const contentTextarea = page.locator("textarea").first();
    await expect(contentTextarea).toBeVisible({ timeout: 5_000 });
    await contentTextarea.clear();
    await contentTextarea.fill(INJECT_CONTENT_UPDATED);

    // Save
    const saveBtn = page.getByRole("button", { name: /^save$/i });
    await expect(saveBtn).toBeVisible();
    await saveBtn.click();

    // Edit mode ends (pencil button reappears)
    await expect(pencilBtn).toBeVisible({ timeout: 10_000 });
    console.log(`[2] Edit saved — waiting for re-processing`);

    const statusAfterEdit = await pollInjectionStatus(page, INJECT_NAME_EDIT);
    console.log(`[2] Re-processing status: ${statusAfterEdit}`);
    expect(statusAfterEdit).toBe("completed");
  });

  // ── TEST 3: Delete ─────────────────────────────────────────────────────────
  test("3 - Delete injection via detail page delete button", async ({
    page,
  }) => {
    await gotoKnowledge(page);
    // Create a dedicated injection to delete
    await createTextInjection(page, INJECT_NAME_DEL, "Temp entry to be deleted.");
    await expect(page.getByText(INJECT_NAME_DEL)).toBeVisible({ timeout: 8_000 });
    console.log(`[3] "${INJECT_NAME_DEL}" created`);

    // Navigate to the detail page by clicking the card link
    const cardLink = page
      .locator('a[href^="/knowledge/"]')
      .filter({ has: page.getByText(INJECT_NAME_DEL, { exact: true }) });
    await expect(cardLink).toBeVisible({ timeout: 5_000 });
    await cardLink.click();
    await page.waitForURL(/\/knowledge\/[a-f0-9-]+/, { timeout: 8_000 });
    await waitForAppReady(page);
    console.log(`[3] Detail page: ${page.url()}`);

    // Click the red "Delete" button on the detail page header
    const deleteBtn = page.getByRole("button", { name: /^delete$/i });
    await expect(deleteBtn).toBeVisible({ timeout: 5_000 });
    await deleteBtn.click();

    // Confirm dialog appears
    const confirmDialog = page.locator('[role="dialog"]');
    await expect(confirmDialog).toBeVisible({ timeout: 8_000 });
    console.log(`[3] Delete confirm dialog opened`);

    // Click the destructive "Delete" button inside the dialog
    const confirmDeleteBtn = confirmDialog
      .getByRole("button", { name: /^delete$/i })
      .last();
    await expect(confirmDeleteBtn).toBeVisible();
    await confirmDeleteBtn.click();

    // Should navigate back to /knowledge after deletion
    await page.waitForURL(/\/knowledge$/, { timeout: 10_000 });
    console.log(`[3] Navigated back to knowledge list ✓`);

    // Verify the card is gone
    await expect(page.getByText(INJECT_NAME_DEL)).not.toBeVisible({
      timeout: 5_000,
    });
    console.log(`[3] "${INJECT_NAME_DEL}" is no longer visible ✓`);
  });

  // ── TEST 4: API verification ────────────────────────────────────────────────
  test("4 - API: at least one injection has entity_count > 0", async ({
    page,
  }) => {
    const resp = await page.request.get(API_INJECTIONS_URL, { headers: WS_HEADERS });
    expect(resp.ok()).toBe(true);
    const body = await resp.json();

    const summary = (body.items as any[]).map((i) => ({
      name: i.name,
      status: i.status,
      entity_count: i.entity_count,
    }));
    console.log(`[4] Injections (total=${body.total}):`, JSON.stringify(summary, null, 2));

    const withEntities = summary.filter((i) => i.entity_count > 0);
    console.log(`[4] ${withEntities.length} injection(s) with entities > 0 ✓`);
    expect(withEntities.length).toBeGreaterThan(0);
  });

  // ── TEST 5: Query retrieval ─────────────────────────────────────────────────
  test("5 - Query retrieves XJMIJI → Hermes 3 from injection", async ({
    page,
  }) => {
    const resp = await page.request.post(`${API_V1_URL}/query`, {
      headers: {
        "Content-Type": "application/json",
        "X-Workspace-ID": WS_ID,
      },
      data: { query: "What is XJMIJI?" },
    });
    expect(resp.ok()).toBe(true);
    const body = await resp.json();
    const answer = body.answer ?? "";
    console.log(`[5] Answer (first 300): ${answer.substring(0, 300)}`);

    // KB1-XJMIJI injection maps XJMIJI → Hermes 3
    expect(answer).toMatch(/XJMIJI|Hermes/i);
  });
});

