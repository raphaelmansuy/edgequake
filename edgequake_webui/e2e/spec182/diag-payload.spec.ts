/**
 * SPEC-182 diagnostic — what does the composer actually put on the wire?
 *
 * Not a screenshot test. It records the multipart POST the browser sends for a picture-only
 * search and prints the request JSON plus whether a `query_image` part is present, so a
 * ranking difference between the UI and `curl` can be traced to a field rather than guessed.
 */
import { expect, test } from "@playwright/test";
import path from "node:path";

const TENANT_ID = process.env.SPEC182_TENANT_ID ?? "";
const WORKSPACE_ID = process.env.SPEC182_WORKSPACE_ID ?? "";
const PROBE =
  process.env.SPEC182_PROBE_PIZZA ??
  path.resolve(__dirname, "../../../specs/182-rags/images/probes/probe-pizza.jpg");

test.skip(!TENANT_ID || !WORKSPACE_ID, "run e2e/record.sh first");

test("T-182-diag: record the picture-only request payload", async ({ page }) => {
  await page.addInitScript(
    ({ tenantId, workspaceId }) => {
      localStorage.setItem(
        "edgequake-tenant",
        JSON.stringify({
          state: {
            selectedTenantId: tenantId,
            selectedWorkspaceId: workspaceId,
            primaryColorByTenant: {},
          },
          version: 1,
        }),
      );
      localStorage.setItem("userId", "00000000-0000-0000-0000-000000000001");
    },
    { tenantId: TENANT_ID, workspaceId: WORKSPACE_ID },
  );
  await page.setViewportSize({ width: 1440, height: 900 });

  // Intercept without altering: log the parts, then let it continue.
  const captured: string[] = [];
  await page.route("**/api/v1/chat/completions/stream", async (route) => {
    const req = route.request();
    const body = req.postData() ?? "";
    captured.push(`content-type: ${req.headers()["content-type"] ?? "?"}`);
    captured.push(`has query_image part: ${body.includes('name="query_image"')}`);
    const m = body.match(/name="request"\r?\n\r?\n(\{[\s\S]*?\})\r?\n--/);
    captured.push(`request part: ${m?.[1] ?? "(not found)"}`);
    await route.continue();
  });

  await page.goto("/query", { waitUntil: "domcontentloaded" });
  const fresh = page.getByTestId("query-new");
  if (await fresh.isVisible().catch(() => false)) await fresh.click();
  await expect(page.getByTestId("query-search-image-button")).toBeVisible({ timeout: 60_000 });

  await page.getByTestId("query-search-image-input").setInputFiles(PROBE);
  await expect(page.getByTestId("query-search-image-chip")).toBeVisible();
  await page.getByTestId("query-send").click();
  await expect(page.getByTestId("query-image-match").first()).toBeVisible({ timeout: 180_000 });

  console.log("\n=== CAPTURED REQUEST ===");
  for (const line of captured) console.log(line);
  console.log("=== FIRST THREE TILES ===");
  const tiles = page.getByTestId("query-image-match");
  const n = Math.min(3, await tiles.count());
  for (let i = 0; i < n; i += 1) {
    console.log(`  ${i + 1}. ${(await tiles.nth(i).innerText()).replace(/\n/g, " | ")}`);
  }
});
