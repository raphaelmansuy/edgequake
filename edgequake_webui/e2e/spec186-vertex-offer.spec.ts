/**
 * A saved Vertex connection is not offered until its test passes. The test
 * lives on the provider card, and a pass puts the provider on the organization page.
 */
import fs from "node:fs";
import path from "node:path";
import { expect, test, type Page } from "@playwright/test";
import { openSettings, settingsH1 as h1 } from "./spec175/helpers/open-settings";
import { installProviderFixture, type FixtureProvider } from "./spec179/helpers/fixtures";
import { applyTheme, setViewport } from "./spec179/helpers/shots";

const VERTEX_ID = "c-vertex";
const SHOTS = path.resolve(__dirname, "../../specs/186-vertex-offer/e2e");

const vertex = (): FixtureProvider => ({
  slug: "vertexai",
  display_name: "Vertex AI",
  origin: "fleet",
  api_shape: "vertexai",
  api_shape_label: "Vertex AI",
  auth: "google_service_account",
  accepts_saved_key: true,
  offered: false,
  withdrawn: false,
  probe: "unchecked",
  key_configured: true,
  connection_id: VERTEX_ID,
  models: [
    { id: "gemini-2.5-flash", usages: ["query", "vlm"] },
    { id: "gemini-embedding-001", usages: ["embedding"], embedding_dimension: 3072 },
  ],
});

async function shotCard(page: Page, name: string, theme: "light" | "dark" = "light") {
  fs.mkdirSync(SHOTS, { recursive: true });
  await setViewport(page, 1440);
  await applyTheme(page, theme);
  const card = page.getByTestId("deployment-provider-vertexai");
  await card.scrollIntoViewIfNeeded();
  await page.getByText("Vertex AI is offered to organizations.").waitFor({ state: "hidden", timeout: 8_000 }).catch(() => undefined);
  await card.screenshot({
    path: path.join(SHOTS, `${name}-${theme}-1440.png`),
    animations: "disabled",
    caret: "hide",
  });
}

test.describe("untested fleet provider becomes an offer @spec186", () => {
  test.beforeEach(({}, info) => info.setTimeout(120_000));

  test("testing Vertex on the provider card offers it to the organization", async ({ page }) => {
    await openSettings(page, "platform_admin", "/settings/deployment/providers");
    const recorder = await installProviderFixture(page, {
      extraProviders: [vertex()],
      offerMode: "unconstrained",
    });
    await page.reload({ waitUntil: "domcontentloaded" });
    await expect(h1(page)).toHaveText("Providers", { timeout: 30_000 });

    const card = page.getByTestId("deployment-provider-vertexai");
    await expect(card).toBeVisible();
    await expect(page.getByTestId("probe-vertexai")).toHaveText("Not tested");
    await expect(page.getByTestId("offer-state-vertexai")).toHaveText("Not offered");
    await expect(page.getByTestId("add-provider-link")).toHaveText("Add a provider");
    await expect(page.getByTestId("withdraw-vertexai")).toHaveText("Remove");
    await expect(page.getByTestId("test-provider-vertexai")).toHaveText("Test connection");
    await expect(page.getByTestId("test-first-vertexai")).toContainText("offers this provider");
    await expect(card.getByRole("button", { name: "Remove gemini-2.5-flash" })).toHaveCount(0);
    await expect(card.getByPlaceholder("Model id")).toHaveCount(0);
    await expect(page.getByTestId("test-first-stale")).toContainText("Open Connections");
    await shotCard(page, "01-vertex-not-offered");
    await shotCard(page, "01-vertex-not-offered", "dark");

    await page.getByTestId("test-provider-vertexai").click();
    await expect(page.getByTestId("probe-vertexai")).toHaveText("Healthy");
    await expect(page.getByTestId("offer-state-vertexai")).toHaveText("Offered");
    await expect(page.getByTestId("test-provider-vertexai")).toHaveCount(0);
    await expect(card.getByText("Models", { exact: true })).toBeVisible();
    await expect(card.getByTestId("add-model-vertexai")).toHaveText("Add model");
    await expect(card.getByPlaceholder("Model id")).toBeVisible();
    await expect(card.getByRole("button", { name: "Remove gemini-2.5-flash" })).toBeEnabled();
    expect(recorder.tests).toEqual([{ id: VERTEX_ID }]);
    await shotCard(page, "02-vertex-offered");

    await page.goto("/settings/organization/providers");
    await expect(h1(page)).toHaveText("Providers", { timeout: 30_000 });
    await expect(page.getByTestId("provider-offer-lead")).toContainText(
      "every provider this deployment offers",
    );
    const row = page.getByTestId("provider-row-vertexai");
    await expect(row).toBeVisible();
    await expect(row).toContainText("Vertex AI");
    await expect(row).toContainText("gemini-2.5-flash");
    await expect(page.getByTestId("provider-toggle-vertexai")).toBeChecked();
    await expect(page.getByTestId("add-provider")).toHaveText("Add a provider");
    await expect(page.getByTestId("remove-provider-vertexai")).toHaveText("Remove");
    await expect(page.getByTestId("update-models-vertexai")).toHaveText("Update models");
    fs.mkdirSync(SHOTS, { recursive: true });
    await page.getByTestId("provider-offer-card").screenshot({
      path: path.join(SHOTS, "03-organization-vertex-light-1440.png"),
      animations: "disabled",
      caret: "hide",
    });

    await page.goto("/settings/deployment/connections");
    await expect(h1(page)).toHaveText("Connections", { timeout: 30_000 });
    const saved = page.getByTestId("connection-row-vertexai");
    await expect(saved).toBeVisible();
    await expect(page.getByTestId("connection-models-vertexai")).toHaveText("Update models");
    await expect(page.getByTestId("connection-delete-vertexai")).toHaveText("Delete");
    await saved.screenshot({
      path: path.join(SHOTS, "04-connection-models-light-1440.png"),
      animations: "disabled",
      caret: "hide",
    });
  });
});
