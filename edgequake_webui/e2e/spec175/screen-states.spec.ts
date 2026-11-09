/**
 * SPEC-175 — screen states. Every settings screen must show real content or an explicit state
 * (loading, error, read-only), never a blank card or a stuck placeholder. Hermetic (mock-api).
 */
import { expect, test } from "@playwright/test";
import { openSettings, settingsH1 as h1 } from "./helpers/open-settings";

test.describe("SPEC-175 screen states @spec175", () => {
  test.beforeEach(({}, info) => info.setTimeout(120_000));

  test("Access lists departments and profiles for a workspace admin @spec175", async ({ page }) => {
    await openSettings(page, "workspace_admin", "/settings/workspace/access");
    await expect(h1(page)).toHaveText("Access");

    const card = page.getByTestId("document-labels-card");
    await expect(card).toBeVisible();
    await expect(page.getByTestId("access-profiles-loading")).toHaveCount(0);
    const profiles = card.getByTestId("access-profiles");
    await expect(profiles.getByText("Finance analysts", { exact: true })).toBeVisible();
    await expect(profiles.getByText("Engineering", { exact: true })).toBeVisible();
    await expect(profiles.getByText("Contractors", { exact: true })).toBeVisible();
    await expect(card.getByTestId("vocabulary-row")).toContainText("Finance, Engineering, Legal");
    await expect(page.getByTestId("document-labels-status")).toHaveCount(0);
  });

  test("Access explains a failed settings read and offers a retry @spec175", async ({ page }) => {
    await openSettings(page, "workspace_admin", "/settings/workspace/access", 1440, {}, { failSettings: true });
    const status = page.getByTestId("document-labels-status");
    await expect(status).toHaveAttribute("role", "alert");
    await expect(status).toContainText("Could not load access settings.");
    await expect(page.getByRole("button", { name: "Try again" })).toBeVisible();
    await expect(page.getByTestId("document-labels-card")).toHaveCount(0);
  });

  test("Observability shows the not-configured state, not a stuck loader @spec175", async ({ page }) => {
    await openSettings(page, "platform_admin", "/settings/deployment/observability");
    const card = page.getByTestId("langfuse-settings-card");
    await expect(card).toBeVisible();
    await expect(page.getByTestId("langfuse-status")).toHaveText("Not configured");
    await expect(card.locator("pre")).toContainText("LANGFUSE_PUBLIC_KEY");
    await expect(card.locator("pre")).not.toContainText("Loading…");
  });

  test("Ingestion keeps the source lowercase and the resolved model readable @spec175", async ({ page }) => {
    await openSettings(page, "workspace_admin", "/settings/workspace/ingestion");
    const resolves = page.getByTestId("vision-resolves-to");
    await expect(resolves).toBeVisible();
    // The fixture pins a workspace VLM role (ollama/qwen3), so the card names that override and its source.
    await expect(page.getByText("Workspace override", { exact: true })).toBeVisible();
    await expect(resolves).toHaveText("Resolves to ollama/qwen3");
    // Title-cased source ("Source=Workspace") is the bug this guards against: the source stays lowercase.
    await expect(resolves).toHaveAttribute("title", "source=workspace");
  });

  test("Graph node size reads Small / Medium / Large and selects share one width @spec175", async ({ page }) => {
    await openSettings(page, "member", "/settings/you/graph");
    const size = page.getByRole("combobox", { name: "Node size" });
    const layout = page.getByRole("combobox", { name: "Default layout" });
    await expect(size).toBeVisible();

    await size.click();
    for (const label of ["Small", "Medium", "Large"]) {
      await expect(page.getByRole("option", { name: label, exact: true })).toBeVisible();
    }
    await page.keyboard.press("Escape");

    const [sizeBox, layoutBox] = await Promise.all([size.boundingBox(), layout.boundingBox()]);
    expect(sizeBox?.width).toBeGreaterThan(0);
    expect(Math.round(sizeBox!.width)).toBe(Math.round(layoutBox!.width));
  });

  test("Connections labels every field and keeps the summary free of doubled separators @spec175", async ({ page }) => {
    await openSettings(page, "platform_admin", "/settings/deployment/connections");
    for (const label of ["Name", "API shape", "Base URL"]) {
      await expect(page.getByLabel(label, { exact: true })).toBeVisible();
    }
    // OpenAI does not set requires_key, so the field reads "API key (optional)".
    await expect(page.getByLabel(/^API key/)).toBeVisible();
    const summaries = page.locator("li span.text-xs");
    const count = await summaries.count();
    for (let i = 0; i < count; i += 1) {
      const text = (await summaries.nth(i).textContent()) ?? "";
      expect(text).not.toMatch(/·\s*·/);
    }
  });
});
