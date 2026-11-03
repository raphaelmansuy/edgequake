/**
 * SPEC-182 — visual proof that picture search is reachable from the product, not only from
 * the API.
 *
 * Every test does two things: it drives the real UI against a live backend, and it asserts
 * the behaviour the resulting screenshot is supposed to demonstrate. A screenshot that no
 * longer reflects a working feature therefore fails here instead of ageing quietly.
 *
 * WHY no route mocking: `specs/182-rags/api/RESULTS.md` already proves the API with real
 * vectors. Mocking here would only prove that the React tree can render JSON someone typed.
 * These tests talk to the dev stack on :3010 → :8090, which must hold the SPEC-182 corpus
 * created by `e2e/run_demo.py`.
 *
 * Required env (written by `record.sh`):
 *   SPEC182_TENANT_ID, SPEC182_WORKSPACE_ID, SPEC182_BASE_URL (default :3010)
 */
import { expect, test, type Page } from "@playwright/test";
import { applyTheme, setViewport, shot } from "./helpers/shots";

const TENANT_ID = process.env.SPEC182_TENANT_ID ?? "";
const WORKSPACE_ID = process.env.SPEC182_WORKSPACE_ID ?? "";
const USER_ID = "00000000-0000-0000-0000-000000000001";
/**
 * The cappuccino document in the SPEC-182 corpus. WHY a constant and not a lookup: the
 * detail page is reached by route, and a lookup would make the test depend on list
 * pagination order — noise that has nothing to do with what the screenshot proves.
 * `e2e/record.sh` exports the real id after it seeds the corpus.
 */
const CAPPUCCINO_DOC_ID = process.env.SPEC182_CAPPUCCINO_DOC_ID ?? "";

test.skip(
  !TENANT_ID || !WORKSPACE_ID,
  "SPEC182_TENANT_ID / SPEC182_WORKSPACE_ID are required; run e2e/record.sh",
);

/**
 * Point the app at the SPEC-182 tenant and workspace before the first paint.
 *
 * WHY localStorage and not the UI: the tenant picker is a multi-step flow that adds noise
 * to a screenshot about pictures. The persisted shape is the contract the store already
 * reads (`ZUSTAND_STORAGE_KEYS.TENANT_STORE`, version 1), so seeding it is exactly what a
 * returning user arrives with.
 */
async function selectSpec182Scope(page: Page) {
  await page.addInitScript(
    ({ tenantId, workspaceId, userId }) => {
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
      localStorage.setItem("userId", userId);
    },
    { tenantId: TENANT_ID, workspaceId: WORKSPACE_ID, userId: USER_ID },
  );
}

test.describe("@spec182 picture search", () => {
  test.beforeEach(({}, info) => info.setTimeout(180_000));

  test("T-182-ui-documents: the corpus lists as named pictures", async ({ page }) => {
    await selectSpec182Scope(page);
    await page.setViewportSize({ width: 1440, height: 900 });
    await page.goto("/documents", { waitUntil: "domcontentloaded" });

    // The corpus has 20 named pictures. Asserting on a distinctive caption proves both that
    // the list loaded and that titles are readable (not filenames) — the regression
    // `display_title_from_filename` exists for.
    await expect(page.getByText("Two cappuccinos with latte art on a wooden table")).toBeVisible({
      timeout: 60_000,
    });
    await expect(page.getByTestId("documents-page")).toBeVisible();
    await shot(page, "10-documents-corpus");
  });

  test("T-182-ui-image: a picture document shows the picture, not its filename", async ({
    page,
  }) => {
    await selectSpec182Scope(page);
    await page.setViewportSize({ width: 1440, height: 900 });
    // WHY the direct route: clicking a row opens a preview aside whose content depends on
    // list state; the detail route is the page the article talks about.
    await page.goto(`/documents/${CAPPUCCINO_DOC_ID}`, { waitUntil: "domcontentloaded" });

    await expect(page.getByTestId("document-image")).toBeVisible({ timeout: 60_000 });
    // The bug this guards: the document body used to be the filename painted as text.
    await expect(page.getByTestId("document-image-stage")).not.toContainText(".jpg");
    // Poll naturalWidth: the <img> mounts before the blob resolves, and a screenshot taken
    // at "visible" once captured a broken-image icon.
    await expect
      .poll(
        async () =>
          page.evaluate(() => {
            const img = document.querySelector<HTMLImageElement>("[data-testid='document-image']");
            return img ? img.naturalWidth : 0;
          }),
        { timeout: 45_000 },
      )
      .toBeGreaterThan(0);
    await shot(page, "11-document-image-detail");

    await applyTheme(page, "dark");
    await shot(page, "12-document-image-detail", { theme: "dark" });
    await applyTheme(page, "light");
    await setViewport(page, 390);
    await shot(page, "13-document-image-detail", { width: 390 });
  });
});
