/**
 * SPEC-179 — Deployment › Providers: cards, per-provider refresh, withdraw
 * confirm, embedding dimension. Hermetic (mock-api).
 */
import { expect, test } from "@playwright/test";
import { openSettings, settingsH1 as h1 } from "../spec175/helpers/open-settings";
import { installProviderFixture } from "./helpers/fixtures";
import { applyTheme, setViewport, shot } from "./helpers/shots";

async function open(page: import("@playwright/test").Page, options = {}) {
  await openSettings(page, "platform_admin", "/settings/deployment/providers");
  const recorder = await installProviderFixture(page, options);
  await page.reload({ waitUntil: "domcontentloaded" });
  await expect(h1(page)).toHaveText("Providers", { timeout: 30_000 });
  return recorder;
}

test.describe("SPEC-179 deployment providers @spec179", () => {
  test.beforeEach(({}, info) => info.setTimeout(120_000));

  test("cards show origin, probe, key, refresh age and capabilities", async ({ page }) => {
    await open(page);
    const openai = page.getByTestId("deployment-provider-openai");
    await expect(openai).toContainText("OpenAI");
    await expect(page.getByTestId("probe-openai")).toHaveText("Healthy");
    await expect(page.getByTestId("key-openai")).toContainText("Key set");
    await expect(page.getByTestId("refresh-line-openai")).toContainText("Refreshed");
    await expect(page.getByTestId("diff-added-openai")).toHaveText("+2 new");
    await expect(page.getByTestId("diff-removed-openai")).toHaveText("−1 gone");
    await expect(openai).toContainText("text-embedding-3-small");
    await expect(openai).toContainText("embedding");

    const stale = page.getByTestId("deployment-provider-stale");
    await expect(page.getByTestId("probe-stale")).toHaveText("Check failed");
    await expect(page.getByTestId("key-stale")).toContainText("No key");
    await expect(stale.getByTestId("refresh-line-stale")).toContainText("401 from upstream");
    await expect(page.getByTestId("fleet-refresh-stale")).toHaveCount(0);
    await shot(page, "deployment-providers-rich", { theme: "light", width: 1440 });
  });

  test("refreshing an env provider records the diff and updates the card", async ({ page }) => {
    const recorder = await open(page);
    await page.getByTestId("env-refresh-openai").click();
    await expect.poll(() => recorder.refreshes).toContain("openai");
    await expect(page.getByTestId("diff-added-openai")).toHaveText("+3 new");
  });

  test("withdraw of a provider in use asks first, then confirms", async ({ page }) => {
    const recorder = await open(page, { impactMessage: "4 assignments still use openai." });
    await page.getByTestId("withdraw-openai").click();
    const dialog = page.getByTestId("withdraw-confirm");
    await expect(dialog).toBeVisible();
    await expect(dialog).toContainText("4 assignments still use openai.");
    await shot(page, "deployment-withdraw-confirm", { theme: "light", width: 1440 });
    await dialog.getByRole("button", { name: "Remove" }).click();
    await expect(page.getByTestId("offer-state-openai")).toHaveText("Hidden from organizations");
    expect(recorder.withdraws).toEqual([
      { slug: "openai", confirm: false, withdrawn: true },
      { slug: "openai", confirm: true, withdrawn: true },
    ]);
    await expect(page.getByTestId("withdraw-openai")).toHaveText("Restore");
  });

  test("cancelling the confirm leaves the provider offered", async ({ page }) => {
    const recorder = await open(page, { impactMessage: "2 assignments still use openai." });
    await page.getByTestId("withdraw-openai").click();
    await page.getByTestId("withdraw-confirm").getByRole("button", { name: "Cancel" }).click();
    await expect(page.getByTestId("offer-state-openai")).toHaveText("Offered");
    expect(recorder.withdraws).toHaveLength(1);
  });

  test("an unused provider withdraws without a prompt", async ({ page }) => {
    const recorder = await open(page);
    await page.getByTestId("withdraw-acme").click();
    await expect(page.getByTestId("offer-state-acme")).toHaveText("Hidden from organizations");
    expect(recorder.withdraws).toEqual([{ slug: "acme", confirm: false, withdrawn: true }]);
  });

  test("an embedding model needs its dimension before it can be saved", async ({ page }) => {
    const recorder = await open(page);
    const acme = page.getByTestId("fleet-models-acme");
    await acme.getByLabel("Model id for acme").fill("embed-new");
    await acme.getByLabel("Type for acme").selectOption("embedding");
    await expect(acme.getByRole("button", { name: "Add model", exact: true })).toBeDisabled();
    await acme.getByLabel("Dimension", { exact: true }).fill("1024");
    await acme.getByRole("button", { name: "Add model", exact: true }).click();
    await expect(page.getByTestId("fleet-model-acme-embed-new")).toBeVisible();
    await shot(page, "deployment-fleet-models-draft", { theme: "light", width: 1440 });
    await acme.getByRole("button", { name: "Save", exact: true }).click();
    await expect.poll(() => recorder.modelPuts.length).toBe(1);
    expect(recorder.modelPuts[0]).toEqual({
      slug: "acme",
      body: {
        models: [
          { id: "acme-chat", supports_chat: true, supports_vision: false, supports_embedding: false },
          { id: "acme-embed-768", supports_chat: false, supports_vision: false, supports_embedding: true, embedding_dimension: 768 },
          { id: "embed-new", supports_chat: false, supports_vision: false, supports_embedding: true, embedding_dimension: 1024 },
        ],
      },
    });
  });

  test("dark and phone frames", async ({ page }) => {
    await open(page);
    await applyTheme(page, "dark");
    await shot(page, "deployment-providers-rich", { theme: "dark", width: 1440 });
    await applyTheme(page, "light");
    await setViewport(page, 390);
    await expect(page.getByTestId("deployment-provider-openai")).toBeVisible();
    await shot(page, "deployment-providers-rich", { theme: "light", width: 390 });
  });
});
