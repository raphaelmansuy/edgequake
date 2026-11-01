/**
 * SPEC-180 — the retrieval profile is a decision at create time, and it is permanent.
 *
 * SPEC-181 replaced the eight-step workspace wizard with a three-decision dialog, so the
 * pre-181 assertions on `wizard-step-retrieval` / `wizard-next` no longer described any
 * screen: the specs timed out instead of failing, which read as a hang rather than as rot.
 * What SPEC-180 actually owns survives the rewrite — the profile is asked once, defaults to
 * Graph, and travels on the create POST as an immutable `rag_profile`.
 *
 * The image-embedding probe these specs used to cover now lives in
 * `ModelDefaultsStep` (the first-run wizard), not the create dialog: the dialog inherits the
 * organization's models instead of asking for them (LAW-181-4, LAW-181-13). Probing is
 * therefore not reachable from this flow, and asserting its absence is the real contract.
 */
import { expect, test, type Page } from "@playwright/test";
import { prepareAdminPage, type AdminMockOptions } from "../spec181/helpers/admin-mock";
import { applyTheme, setViewport, shot, shotLocator } from "./helpers/shots";

const open = async (page: Page, opts: AdminMockOptions = {}) => {
  const state = await prepareAdminPage(page, opts);
  await setViewport(page, 1440);
  await page.goto("/", { waitUntil: "domcontentloaded" });
  return state;
};

/** Open the header switcher, then the create-workspace dialog. */
const openCreate = async (page: Page, opts: AdminMockOptions = {}) => {
  const state = await open(page, opts);
  await page.getByTestId("workspace-selector").click();
  await expect(page.getByTestId("context-selector-search")).toBeVisible();
  await page.getByTestId("header-create-workspace").click();
  const dialog = page.getByTestId("create-workspace-dialog");
  await expect(dialog).toBeVisible();
  return { state, dialog };
};

/** The profile a create POST actually carried, or `undefined` if none was sent. */
const createdProfile = (state: Awaited<ReturnType<typeof open>>) => {
  const posts = state.calls.filter((c) => c.method === "POST" && c.path.includes("/workspaces"));
  return posts.length === 1 ? (posts[0].body as Record<string, unknown>).rag_profile : null;
};

test.describe("@spec180 create workspace retrieval profile", () => {
  test.beforeEach(({}, info) => info.setTimeout(120_000));

  test("T-180-ui-wizard: Graph is preselected", async ({ page }) => {
    await openCreate(page);

    await expect(page.getByTestId("create-workspace-retrieval-graph")).toHaveAttribute(
      "aria-checked",
      "true",
    );
    await expect(page.getByTestId("create-workspace-retrieval-normal")).toHaveAttribute(
      "aria-checked",
      "false",
    );
    await shot(page, "20-create-profile-graph-default");
  });

  test("T-180-ui-wizard: the choice is labelled permanent", async ({ page }) => {
    const { dialog } = await openCreate(page);

    // A one-way choice must say so before it is made, not fail after.
    await expect(dialog.getByText("This is permanent")).toBeVisible();
    await shotLocator(page, dialog, "21-create-profile-permanent", { theme: "light" });
  });

  test("T-180-ui-wizard: Normal is sent as the immutable rag_profile", async ({ page }) => {
    const { state } = await openCreate(page);
    await page.getByTestId("create-workspace-name").fill("Policy desk");
    await page.getByTestId("create-workspace-retrieval-normal").click();
    await page.getByTestId("create-workspace-dialog-finish").click();

    await expect(page.getByTestId("create-workspace-dialog")).toBeHidden({ timeout: 20_000 });
    expect(createdProfile(state)).toBe("normal");
  });

  test("T-180-ui-wizard: Graph keeps its extraction steps and sends graph", async ({ page }) => {
    // Graph is the default and needs no click, but the payload must still say so.
    const { state } = await openCreate(page);
    await page.getByTestId("create-workspace-name").fill("Research");
    await page.getByTestId("create-workspace-dialog-finish").click();

    await expect(page.getByTestId("create-workspace-dialog")).toBeHidden({ timeout: 20_000 });
    expect(createdProfile(state)).toBe("graph");
  });

  test("T-180-ui-wizard: an image workspace probes no model on the create dialog", async ({
    page,
  }) => {
    // The dialog inherits the org's models rather than asking for them, so the
    // `GET /models/embedding-support` probe is not part of this flow. If it reappears here,
    // the create dialog has drifted back toward the eight-step wizard.
    const { state, dialog } = await openCreate(page);
    await page.getByTestId("create-workspace-name").fill("Scans");
    await page.getByTestId("create-workspace-retrieval-normal").click();

    await expect(dialog.getByText("Inherit from")).toBeVisible();
    await expect(page.getByTestId("wizard-image-probe-ok")).toHaveCount(0);
    await expect(page.getByTestId("wizard-image-probe-failed")).toHaveCount(0);
    expect(state.calls.filter((c) => c.path.includes("/models/embedding-support"))).toHaveLength(0);
  });

  test("T-180-ui-wizard: the create dialog has no stepper", async ({ page }) => {
    // LAW-181-4: the wizard progress bar is what the three-decision dialog removed.
    const { dialog } = await openCreate(page);
    await expect(page.getByTestId("wizard-progress")).toHaveCount(0);
    await expect(page.getByTestId("wizard-next")).toHaveCount(0);
    await expect(dialog.getByTestId("create-workspace-dialog-finish")).toBeVisible();
  });

  test("T-180-ui-wizard: dark and phone frames", async ({ page }) => {
    const { dialog } = await openCreate(page);
    await page.getByTestId("create-workspace-retrieval-normal").click();

    await applyTheme(page, "dark");
    await shotLocator(page, dialog, "28-create-profile-normal", { theme: "dark" });

    await applyTheme(page, "light");
    await setViewport(page, 390);
    await expect(dialog).toBeVisible();
    await shot(page, "29-create-profile-phone", { width: 390 });
    await setViewport(page, 1440);
  });
});
