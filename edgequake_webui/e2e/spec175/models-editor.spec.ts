/**
 * SPEC-175 W4/W5 — the typed model editors: eligible-only pickers, locked and broken states,
 * the organization ceiling, and workspace narrowing. Hermetic (mock-api project). Every
 * assertion reads what the page sent (PUT bodies) or shows, never a hard-coded success.
 */
import { expect, test, type Page } from "@playwright/test";
import { applyTheme, shot } from "./helpers/shots";
import { openSettings, settingsH1 as h1 } from "./helpers/open-settings";

const roles = (page: Page) => page.getByTestId("model-roles-card");
const ceiling = (page: Page) => page.getByTestId("tenant-model-ceiling-card");
const narrowing = (page: Page) => page.getByTestId("workspace-model-narrowing-card");

test.describe("SPEC-175 model editors @spec175", () => {
  test.beforeEach(({}, info) => info.setTimeout(120_000));

  test("the role picker lists only eligible models for the usage (EC-175-12)", async ({ page }) => {
    await openSettings(page, "workspace_admin", "/settings/workspace/models");
    await expect(h1(page)).toHaveText("Models");
    const picker = page.getByLabel("Model for Query");
    await expect(picker.locator("option", { hasText: "GPT-5 nano" })).toHaveCount(1);
    await expect(picker.locator("option", { hasText: "Gemma 3" })).toHaveCount(1);
    // Qwen 3 is registered but its last probe failed, so it is not eligible.
    await expect(picker.locator("option", { hasText: "Qwen 3" })).toHaveCount(0);
    await shot(page, "wsadmin-model-roles");
  });

  test("a saved model that is no longer eligible is flagged, not hidden (EC-175-12)", async ({ page }) => {
    await openSettings(page, "workspace_admin", "/settings/workspace/models");
    // Vision is saved as Qwen 3, which is not eligible for vision.
    const row = page.getByTestId("usage-row-vlm");
    await expect(row.getByTestId("usage-broken")).toHaveText("This model is not allowed for this workspace.");
    /*
     * SPEC-184 — the "pick another one to save" instruction moved from every row to one summary
     * line in the card header. Four identical sentences made the table tall and noisy; the count
     * keeps it actionable, and this asserts it is present and singular.
     */
    await expect(page.getByTestId("model-roles-ineligible-summary")).toContainText(
      "Pick another one for each to save.",
    );
    await expect(page.getByText("The saved model is no longer allowed here.")).toHaveCount(0);
    // The picker shows the saved pair, not "Inherit": the user sees what is actually saved.
    await expect(page.getByLabel("Model for Vision (PDF pages)")).toHaveValue("ollama::qwen3");
  });

  test("an operator-locked usage shows its source and cannot be changed (EC-175-14)", async ({ page }) => {
    await openSettings(page, "workspace_admin", "/settings/workspace/models", 1440, { lockKeyword: true });
    const row = page.getByTestId("usage-row-keyword");
    await expect(row.getByText("Set by the operator", { exact: true })).toBeVisible();
    await expect(page.getByLabel("Model for Keywords")).toBeDisabled();
  });

  test("decision has no role of its own and says so", async ({ page }) => {
    await openSettings(page, "workspace_admin", "/settings/workspace/models");
    await expect(page.getByTestId("usage-row-decision")).toContainText("Follows the workspace default");
    await expect(page.getByLabel("Model for Decision")).toHaveCount(0);
  });

  test("saving a role sends only the changed usage in llm_roles (EC-175-15)", async ({ page }) => {
    const recorder = await openSettings(page, "workspace_admin", "/settings/workspace/models");
    const save = roles(page).getByRole("button", { name: "Save", exact: true });
    await expect(save).toBeDisabled();

    await page.getByLabel("Model for Query").selectOption("ollama::gemma3");
    await expect(save).toBeEnabled();
    await save.click();

    await expect(page.getByText("Model roles saved")).toBeVisible();
    expect(recorder.rolePuts).toHaveLength(1);
    expect(recorder.rolePuts[0].query).toEqual({ provider: "ollama", model: "gemma3" });
    expect(recorder.rolePuts[0].extract).toEqual({ provider: "ollama", model: "gemma3" });
    expect(recorder.rolePuts[0]).not.toHaveProperty("decision");
    await expect(page.getByTestId("usage-resolved").first()).toBeVisible();
    await expect(save).toBeDisabled();
  });

  test("discard restores the saved role without a request", async ({ page }) => {
    const recorder = await openSettings(page, "workspace_admin", "/settings/workspace/models");
    const picker = page.getByLabel("Model for Query");
    await picker.selectOption("ollama::gemma3");
    await roles(page).getByRole("button", { name: "Discard" }).click();
    await expect(picker).toHaveValue("openai::gpt-5-nano");
    expect(recorder.rolePuts).toHaveLength(0);
  });

  test("the workspace ceiling limits the picker (EC-175-10)", async ({ page }) => {
    await openSettings(page, "workspace_admin", "/settings/workspace/models", 1440, {
      tenantRows: [{ provider: "ollama", model: "gemma3", usage: "query", enabled: true }],
    });
    const picker = page.getByLabel("Model for Query");
    await expect(picker.locator("option", { hasText: "Gemma 3" })).toHaveCount(1);
    await expect(picker.locator("option", { hasText: "GPT-5 nano" })).toHaveCount(0);
  });

  test("narrowing a model removes it from that usage for this workspace (EC-175-10)", async ({ page }) => {
    const recorder = await openSettings(page, "workspace_admin", "/settings/workspace/models");
    await expect(narrowing(page).getByTestId("narrowing-row-ollama::gemma3")).toBeVisible();
    await expect(narrowing(page).getByTestId("narrowing-row-ollama::qwen3")).toHaveCount(0);

    await narrowing(page).getByRole("checkbox", { name: "Use Gemma 3 for Query" }).click();
    await narrowing(page).getByRole("button", { name: "Save", exact: true }).click();

    await expect(page.getByText("Workspace models saved")).toBeVisible();
    expect(recorder.workspacePuts).toEqual([[{ provider: "ollama", model: "gemma3", usage: "query", enabled: false }]]);
    await expect(page.getByLabel("Model for Query").locator("option", { hasText: "Gemma 3" })).toHaveCount(0);
    await shot(page, "wsadmin-narrowing");
  });

  test("another workspace's admin sees narrowing read-only (EC-175-39)", async ({ page }) => {
    await openSettings(page, "other_workspace_admin", "/settings/workspace/models");
    await expect(narrowing(page).getByText("Only workspace owners and admins can narrow these models.")).toBeVisible();
    await expect(narrowing(page).getByRole("checkbox", { name: "Use Gemma 3 for Query" })).toBeDisabled();
  });

  test("the organization ceiling shows registered models with health and saves a full replacement (EC-175-06)", async ({ page }) => {
    const recorder = await openSettings(page, "super_admin", "/settings/organization/providers");
    await expect(h1(page)).toHaveText("Providers");
    await expect(ceiling(page).getByTestId("ceiling-row-ollama::qwen3").getByText("Check failed")).toBeVisible();
    await expect(ceiling(page).getByTestId("ceiling-row-ollama::nomic-embed").getByText("Not checked")).toBeVisible();
    await expect(ceiling(page).getByTestId("ceiling-summary-query")).toHaveText("Any model");

    await ceiling(page).getByRole("checkbox", { name: "Allow OpenAI · GPT-5 nano for Query" }).click();
    await ceiling(page).getByRole("checkbox", { name: "Allow Ollama · Gemma 3 for Query" }).click();
    await expect(ceiling(page).getByTestId("ceiling-summary-query")).toHaveText("2 allowed");

    await ceiling(page).getByRole("button", { name: "Save", exact: true }).click();
    await expect(page.getByText("Model ceiling saved")).toBeVisible();
    expect(recorder.tenantPuts).toEqual([
      [
        { provider: "ollama", model: "gemma3", usage: "query", enabled: true },
        { provider: "openai", model: "gpt-5-nano", usage: "query", enabled: true },
      ],
    ]);
    await shot(page, "tenant-ceiling-saved");
  });

  test("clearing the ceiling returns the organization to unconstrained", async ({ page }) => {
    const recorder = await openSettings(page, "super_admin", "/settings/organization/providers", 1440, {
      tenantRows: [{ provider: "ollama", model: "gemma3", usage: "query", enabled: true }],
    });
    await expect(ceiling(page).getByTestId("ceiling-summary-query")).toHaveText("1 allowed");
    await ceiling(page).getByRole("button", { name: "Clear all limits" }).click();
    await ceiling(page).getByRole("button", { name: "Save", exact: true }).click();
    await expect(page.getByText("Model ceiling saved")).toBeVisible();
    expect(recorder.tenantPuts).toEqual([[]]);
  });

  test("a saved pair that left the catalog is kept on save, never silently dropped", async ({ page }) => {
    const recorder = await openSettings(page, "super_admin", "/settings/organization/providers", 1440, {
      tenantRows: [{ provider: "retired", model: "old-model", usage: "query", enabled: true }],
    });
    await expect(ceiling(page).getByTestId("ceiling-row-retired::old-model").getByText("Not registered")).toBeVisible();
    await ceiling(page).getByRole("checkbox", { name: "Allow OpenAI · GPT-5 nano for Query" }).click();
    await ceiling(page).getByRole("button", { name: "Save", exact: true }).click();
    await expect(page.getByText("Model ceiling saved")).toBeVisible();
    expect(recorder.tenantPuts[0]).toEqual(
      expect.arrayContaining([{ provider: "retired", model: "old-model", usage: "query", enabled: true }]),
    );
  });

  test("the model roles and ceiling render in dark mode (visual review)", async ({ page }) => {
    await openSettings(page, "super_admin", "/settings/organization/providers");
    await applyTheme(page, "dark");
    await expect(ceiling(page)).toBeVisible();
    await shot(page, "tenant-ceiling", { theme: "dark" });
  });
});
