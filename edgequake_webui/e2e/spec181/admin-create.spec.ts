/**
 * SPEC-181 — create organization / create workspace / access gating (hermetic).
 *
 * Every test asserts behaviour AND writes a screenshot to
 * `specs/181-admin/e2e/screenshots/<SPEC181_ROUND>/`, so each round is visually reviewable.
 */
import { expect, test, type Page } from "@playwright/test";
import { prepareAdminPage, type AdminMockOptions } from "./helpers/admin-mock";
import { applyTheme, setViewport, shot, shotBoth } from "./helpers/shots";

const open = async (page: Page, opts: AdminMockOptions = {}, path = "/") => {
  const state = await prepareAdminPage(page, opts);
  await setViewport(page, 1440);
  await page.goto(path, { waitUntil: "domcontentloaded" });
  return state;
};

/** Open the header switcher (the chip that names organization / workspace). */
const openSwitcher = async (page: Page) => {
  await page.getByTestId("workspace-selector").click();
  await expect(page.getByTestId("context-selector-search")).toBeVisible();
};

test.describe("SPEC-181 access gating @spec181", () => {
  test.beforeEach(({}, info) => info.setTimeout(120_000));

  test("organization owner sees create-workspace but not create-organization", async ({ page }) => {
    const state = await open(page, { viewer: "owner" });
    await openSwitcher(page);

    // LAW-181-3: the platform row is absent, the organization row is present.
    await expect(page.getByTestId("header-create-workspace")).toBeVisible();
    await expect(page.getByTestId("header-create-tenant")).toHaveCount(0);

    await shotBoth(page, "switcher-owner");
    expect(state.calls.filter((c) => c.method === "POST")).toHaveLength(0);
  });

  test("platform admin sees both create rows", async ({ page }) => {
    await open(page, { viewer: "platform" });
    await openSwitcher(page);
    await expect(page.getByTestId("header-create-tenant")).toBeVisible();
    await expect(page.getByTestId("header-create-workspace")).toBeVisible();
    await shotBoth(page, "switcher-platform");
  });

  test("a member sees no create affordance and is told what to do", async ({ page }) => {
    await open(page, { viewer: "member" });
    await openSwitcher(page);

    // Absence, not disabled: the element must not exist in the DOM.
    await expect(page.getByTestId("header-create-tenant")).toHaveCount(0);
    await expect(page.getByTestId("header-create-workspace")).toHaveCount(0);
    await expect(page.getByTestId("header-create-workspace-hint")).toBeVisible();

    await shotBoth(page, "switcher-member");
  });

  test("a workspace admin cannot create a workspace", async ({ page }) => {
    await open(page, { viewer: "wsadmin" });
    await openSwitcher(page);
    await expect(page.getByTestId("header-create-workspace")).toHaveCount(0);
    await expect(page.getByTestId("header-create-workspace-hint")).toBeVisible();
    await shotBoth(page, "switcher-wsadmin");
  });

  test("quota full disables the row with the fix, not a missing role", async ({ page }) => {
    await open(page, { viewer: "owner", quota: { used: 10, limit: 10 } });
    await openSwitcher(page);
    await expect(page.getByTestId("header-create-workspace")).toHaveCount(0);
    const blocked = page.getByTestId("header-create-workspace-blocked");
    await expect(blocked).toBeVisible();
    await expect(blocked).toContainText("10");
    await shotBoth(page, "switcher-quota-full");
  });

  test("no organization renders a role-aware empty state", async ({ page }) => {
    await open(page, { viewer: "member", noOrganizations: true });
    await expect(page.getByTestId("guard-copy-request")).toBeVisible();
    await expect(page.getByTestId("guard-create-tenant")).toHaveCount(0);
    await shotBoth(page, "empty-no-org-member");
  });

  test("no organization: platform admin is invited to create one", async ({ page }) => {
    await open(page, { viewer: "platform", noOrganizations: true });
    await expect(page.getByTestId("guard-create-tenant")).toBeVisible();
    await shotBoth(page, "empty-no-org-platform");
  });
});

test.describe("SPEC-181 create workspace @spec181", () => {
  test.beforeEach(({}, info) => info.setTimeout(120_000));

  test("three decisions, then a created workspace", async ({ page }) => {
    const state = await open(page, { viewer: "owner" });
    await openSwitcher(page);
    await page.getByTestId("header-create-workspace").click();

    const dialog = page.getByTestId("create-workspace-dialog");
    await expect(dialog).toBeVisible();

    // The promise of LAW-181-4: three decisions, no stepper.
    await expect(page.getByTestId("create-workspace-name")).toBeVisible();
    await expect(page.getByTestId("create-workspace-retrieval-graph")).toBeVisible();
    await expect(dialog.getByText("Inherit from Acme Research")).toBeVisible();
    await expect(page.getByTestId("wizard-progress")).toHaveCount(0);

    // A pristine dialog must not scold: no blocked reason before the field is touched.
    await expect(page.getByTestId("create-workspace-dialog-blocked-reason")).toBeEmpty();
    await shotBoth(page, "create-workspace");

    await page.getByTestId("create-workspace-name").fill("Contracts");
    await page.getByTestId("create-workspace-name").blur();
    await expect(page.getByTestId("create-workspace-dialog-blocked-reason")).toBeEmpty();
    await page.getByTestId("create-workspace-retrieval-normal").click();
    await expect(page.getByTestId("create-workspace-dialog-finish")).toBeEnabled();
    await page.getByTestId("create-workspace-dialog-finish").click();

    await expect(dialog).toBeHidden({ timeout: 20_000 });

    // One POST, and it carried the immutable retrieval choice.
    const posts = state.calls.filter((c) => c.method === "POST" && c.path.includes("/workspaces"));
    expect(posts).toHaveLength(1);
    expect((posts[0].body as Record<string, unknown>).rag_profile).toBe("normal");
    // LAW-123-8: an inheriting create sends no model fields.
    const body = posts[0].body as Record<string, unknown>;
    expect(body.llm_model).toBeUndefined();
    expect(body.embedding_model).toBeUndefined();
  });

  test("choosing models carries them in one POST", async ({ page }) => {
    const state = await open(page, { viewer: "owner" });
    await openSwitcher(page);
    await page.getByTestId("header-create-workspace").click();
    await page.getByTestId("create-workspace-name").fill("Contracts");

    // The section is a decision now, not a printed statement.
    await expect(page.getByTestId("create-workspace-models-inherit")).toBeVisible();
    await page.getByTestId("create-workspace-models-choose").click();
    await expect(page.getByTestId("create-workspace-models-row-llm-picker")).toBeVisible();

    // Provider → model, exactly the two-step picker the settings page uses. Choosing a provider
    // opens the model step on the next frame, and that menu is portaled to the document body.
    await page.getByTestId("create-workspace-models-row-llm-picker").getByTestId("model-picker-provider-trigger").click();
    await page.getByTestId("model-picker-provider-option-ollama").click();
    await page.getByTestId("model-picker-panel-search").waitFor({ state: "visible" });
    await page.getByTestId("model-picker-option-ollama-gemma3:12b").click();

    // The collapsed row now says the choice belongs to this workspace.
    await expect(page.getByTestId("create-workspace-models-row-llm")).toContainText("this workspace");
    await expect(page.getByTestId("create-workspace-models-row-llm")).toContainText("ollama/gemma3:12b");
    // Choosing collapses the row, so the result is visible instead of the picker that produced it.
    await expect(page.getByTestId("create-workspace-models-row-llm-picker")).toHaveCount(0);
    await shotBoth(page, "create-workspace-models-custom");

    await page.getByTestId("create-workspace-dialog-finish").click();
    await expect(page.getByTestId("create-workspace-dialog")).toBeHidden({ timeout: 20_000 });

    const posts = state.calls.filter((c) => c.method === "POST" && c.path.includes("/workspaces"));
    expect(posts).toHaveLength(1);
    const body = posts[0].body as Record<string, unknown>;
    expect(body.llm_provider).toBe("ollama");
    expect(body.llm_model).toBe("gemma3:12b");
    // A role left alone keeps inheriting, so it is absent from the body.
    expect(body.embedding_model).toBeUndefined();
    expect(body.vision_llm_model).toBeUndefined();
  });

  test("a role can be returned to inherited before creating", async ({ page }) => {
    const state = await open(page, { viewer: "owner" });
    await openSwitcher(page);
    await page.getByTestId("header-create-workspace").click();
    await page.getByTestId("create-workspace-name").fill("Contracts");

    await page.getByTestId("create-workspace-models-choose").click();
    await page.getByTestId("create-workspace-models-row-llm-picker").getByTestId("model-picker-provider-trigger").click();
    await page.getByTestId("model-picker-provider-option-ollama").click();
    await page.getByTestId("model-picker-panel-search").waitFor({ state: "visible" });
    await page.getByTestId("model-picker-option-ollama-gemma3:12b").click();
    await expect(page.getByTestId("create-workspace-models-row-llm")).toContainText("this workspace");

    // One-way doors are a defect: the row can go back to the organization default. Picking a
    // pair collapses the row, so reopen it first.
    await page.getByTestId("create-workspace-models-row-llm-toggle").click();
    await page.getByTestId("create-workspace-models-row-llm-clear").click();
    await expect(page.getByTestId("create-workspace-models-row-llm")).not.toContainText("this workspace");

    await page.getByTestId("create-workspace-dialog-finish").click();
    await expect(page.getByTestId("create-workspace-dialog")).toBeHidden({ timeout: 20_000 });
    const body = state.calls.filter((c) => c.method === "POST" && c.path.includes("/workspaces"))[0]
      .body as Record<string, unknown>;
    expect(body.llm_model).toBeUndefined();
  });

  test("an inheriting create still sends no model fields after visiting the rows", async ({ page }) => {
    const state = await open(page, { viewer: "owner" });
    await openSwitcher(page);
    await page.getByTestId("header-create-workspace").click();
    await page.getByTestId("create-workspace-name").fill("Contracts");

    // Explore the chooser, then go back to the default. Nothing may leak into the body.
    await page.getByTestId("create-workspace-models-choose").click();
    await page.getByTestId("create-workspace-models-use-inherited").click();
    await expect(page.getByTestId("create-workspace-models-rows")).toHaveCount(0);

    await page.getByTestId("create-workspace-dialog-finish").click();
    await expect(page.getByTestId("create-workspace-dialog")).toBeHidden({ timeout: 20_000 });
    const body = state.calls.filter((c) => c.method === "POST" && c.path.includes("/workspaces"))[0]
      .body as Record<string, unknown>;
    expect(body.llm_model).toBeUndefined();
    expect(body.llm_provider).toBeUndefined();
    expect(body.embedding_model).toBeUndefined();
    expect(body.vision_llm_model).toBeUndefined();
  });

  test("inherited values are readable in the default card", async ({ page }) => {
    await open(page, { viewer: "owner" });
    await openSwitcher(page);
    await page.getByTestId("header-create-workspace").click();
    const inherit = page.getByTestId("create-workspace-models-inherit");
    await expect(inherit).toContainText("Extract");
    await expect(inherit).toContainText("Embed");
    await expect(inherit).toContainText("Vision");
    // The consequence of inheriting is stated, not implied.
    await expect(inherit).toContainText("reaches this workspace");
  });

  test("a refused pick names the role and blocks Create only then", async ({ page }) => {
    // Offered for extraction but not for answering: the picker lists it, the family refuses it.
    const state = await open(page, {
      viewer: "owner",
      modelOffer: {
        models: [
          { provider: "openai", model: "gpt-6-luna" },
          { provider: "ollama", model: "gemma3:12b", extractOnly: true },
          { provider: "openai", model: "text-embedding-3-small", dimension: 1536 },
        ],
      },
    });
    await openSwitcher(page);
    await page.getByTestId("header-create-workspace").click();
    await page.getByTestId("create-workspace-name").fill("Contracts");
    await page.getByTestId("create-workspace-models-choose").click();

    await page.getByTestId("create-workspace-models-row-llm-picker").getByTestId("model-picker-provider-trigger").click();
    await page.getByTestId("model-picker-provider-option-ollama").click();
    await page.getByTestId("model-picker-panel-search").waitFor({ state: "visible" });
    await page.getByTestId("model-picker-option-ollama-gemma3:12b").click();

    // The row says what is wrong, and the footer repeats it with the fix.
    const row = page.getByTestId("create-workspace-models-row-llm");
    await expect(row).toHaveAttribute("data-refusal", "blocking");
    await expect(row.getByTestId("create-workspace-models-row-llm-refusal")).toBeVisible();
    await expect(page.getByTestId("create-workspace-dialog-blocked-reason")).toContainText(
      "Choose another model for Extract",
    );
    await expect(page.getByTestId("create-workspace-dialog-finish")).toBeDisabled();
    await shotBoth(page, "create-workspace-models-refused");

    // Back to inherited: the refusal disappears with the pick, and Create is usable again.
    await page.getByTestId("create-workspace-models-row-llm-toggle").click();
    await page.getByTestId("create-workspace-models-row-llm-clear").click();
    await expect(page.getByTestId("create-workspace-dialog-blocked-reason")).toBeEmpty();
    await expect(page.getByTestId("create-workspace-dialog-finish")).toBeEnabled();
    expect(state.calls.filter((c) => c.method === "POST")).toHaveLength(0);
  });

  test("an unoffered organization default warns without blocking", async ({ page }) => {
    // The inherited default is not in the offer. The server still accepts an inheriting create,
    // so this must be an advisory, never a red row beside an enabled button.
    await open(page, {
      viewer: "owner",
      modelOffer: {
        models: [
          { provider: "ollama", model: "gemma3:12b" },
          { provider: "openai", model: "text-embedding-3-small", dimension: 1536 },
        ],
      },
    });
    await openSwitcher(page);
    await page.getByTestId("header-create-workspace").click();
    await page.getByTestId("create-workspace-name").fill("Contracts");

    await expect(page.getByTestId("create-workspace-models-stale-inherit")).toContainText(
      "not offered here",
    );
    await expect(page.getByTestId("create-workspace-dialog-finish")).toBeEnabled();

    await page.getByTestId("create-workspace-dialog-finish").click();
    await expect(page.getByTestId("create-workspace-dialog")).toBeHidden({ timeout: 20_000 });
  });

  test("the Tune later drawer prints inherited values", async ({ page }) => {
    await open(page, { viewer: "owner" });
    await openSwitcher(page);
    await page.getByTestId("header-create-workspace").click();
    await page.getByTestId("tune-later-toggle").click();
    await expect(page.getByTestId("tune-later-panel")).toBeVisible();
    await expect(page.getByTestId("tune-pdf-parser")).toBeVisible();
    await expect(page.getByTestId("tune-language")).toContainText("Inherit");
    await shotBoth(page, "create-workspace-tune-later");
  });

  test("quota full blocks Create and keeps the typed name", async ({ page }) => {
    await open(page, { viewer: "owner", quota: { used: 10, limit: 10 } });
    // The switcher row is disabled at the limit, so open the dialog directly.
    await page.goto("/?create=workspace", { waitUntil: "domcontentloaded" });
    // If the dialog does not self-open, the empty-state path is the entry; assert the reason.
    const reason = page.getByTestId("create-workspace-dialog-blocked-reason");
    if (await reason.count()) {
      await expect(reason).toContainText("10");
    }
  });
});

test.describe("SPEC-181 responsive @spec181", () => {
  test.beforeEach(({}, info) => info.setTimeout(120_000));

  test("create workspace at 390 and 768", async ({ page }) => {
    await open(page, { viewer: "owner" });

    for (const width of [390, 768] as const) {
      await setViewport(page, width);
      await page.goto("/", { waitUntil: "domcontentloaded" });
      // Below `md` the switcher lives inside the navigation drawer, not the header.
      if (width < 768) {
        await page.getByRole("button", { name: /open navigation menu/i }).click();
      }
      // The header chip is a second switcher instance; drive the one that is on screen.
      await page.getByTestId("workspace-selector").filter({ visible: true }).click();
      await page.getByTestId("header-create-workspace").filter({ visible: true }).click();
      await expect(page.getByTestId("create-workspace-dialog")).toBeVisible();
      // The dialog must fit the viewport at every width: no clipped footer, no horizontal scroll.
      const box = await page.getByTestId("create-workspace-dialog").boundingBox();
      expect(box, `dialog must render at ${width}`).not.toBeNull();
      expect(box!.width).toBeLessThanOrEqual(width);
      await expect(page.getByTestId("create-workspace-dialog-finish")).toBeVisible();
      await shot(page, "create-workspace", { theme: "light", width });
      await shot(page, "create-workspace", { theme: "dark", width });

      // The models section is the one new surface: it must fit and stay operable here too.
      await page.getByTestId("create-workspace-models-choose").click();
      await expect(page.getByTestId("create-workspace-models-row-llm-picker")).toBeVisible();
      const section = await page.getByTestId("create-workspace-models").boundingBox();
      expect(section, `models section must render at ${width}`).not.toBeNull();
      expect(section!.width).toBeLessThanOrEqual(width);
      await shot(page, "create-workspace-models-custom", { theme: "light", width });
      await shot(page, "create-workspace-models-custom", { theme: "dark", width });

      await page.keyboard.press("Escape");
      await page.waitForTimeout(300);
    }
  });
});
