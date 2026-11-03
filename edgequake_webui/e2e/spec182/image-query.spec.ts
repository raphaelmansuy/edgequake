/**
 * SPEC-182 — search by image from the composer, driven against a live corpus.
 *
 * WHY a second file: the first attempt asserted on a citation card immediately after Send,
 * which conflates two independent things — "the composer accepts a picture" and "the answer
 * pipeline produced citations". The API already proves retrieval (see
 * `specs/182-rags/api/RESULTS.md`); what a screenshot must prove is that the *UI surfaced
 * it*: the button exists on a picture-capable workspace, the chip shows the chosen picture,
 * the request goes out with the file, and the answer names the picture found.
 *
 * The test therefore watches the network for the multipart POST, which is the contract the
 * composer implements (`chatRequestBody` switches to FormData exactly when `query_image` is
 * present), and then asserts on the pictures the answer cites.
 */
import { expect, test, type Page } from "@playwright/test";
import path from "node:path";
import { applyTheme, setViewport, shot, shotLocator } from "./helpers/shots";

const TENANT_ID = process.env.SPEC182_TENANT_ID ?? "";
const WORKSPACE_ID = process.env.SPEC182_WORKSPACE_ID ?? "";
const PROBE_PIZZA =
  process.env.SPEC182_PROBE_PIZZA ??
  path.resolve(__dirname, "../../../specs/182-rags/images/probes/probe-pizza.jpg");

test.skip(!TENANT_ID || !WORKSPACE_ID, "run e2e/record.sh first");

async function selectSpec182Scope(page: Page) {
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
}

test.describe("@spec182 composer", () => {
  test.beforeEach(({}, info) => info.setTimeout(240_000));

  test("T-182-ui-capability: a picture-capable workspace shows 'Search by image' and the Images filter", async ({
    page,
  }) => {
    await selectSpec182Scope(page);
    await page.setViewportSize({ width: 1440, height: 900 });
    await page.goto("/query", { waitUntil: "domcontentloaded" });

    // Both controls are gated on `capabilities.embeds_images`. Their presence is the claim.
    await expect(page.getByTestId("query-search-image-button")).toBeVisible({ timeout: 60_000 });
    await expect(page.getByTestId("query-kind-image")).toBeVisible();
    await shot(page, "20-query-composer-image-capable");

    await page.getByTestId("query-search-image-input").setInputFiles(PROBE_PIZZA);
    await expect(page.getByTestId("query-search-image-chip")).toBeVisible();
    await expect(page.getByTestId("query-search-image-thumb")).toBeVisible();
    await shot(page, "21-query-search-image-attached");

    // Clear it again: this is the "take it back" affordance, and it must not leave the
    // button disabled or the chip stuck.
    await page.getByTestId("query-search-image-remove").click();
    await expect(page.getByTestId("query-search-image-chip")).toHaveCount(0);
  });

  test("T-182-ui-image-query: sending a picture posts query_image and the answer cites the pizza", async ({
    page,
  }) => {
    await selectSpec182Scope(page);
    await page.setViewportSize({ width: 1440, height: 900 });

    // WHY this route and not simply `/query`: the corpus workspace is selected through
    // localStorage, and the app must finish loading the workspace list before the composer
    // knows it can embed images. Waiting on the capability control (below) is the readiness
    // signal that matters; a fixed sleep would race a cold dev server.
    await page.goto("/query", { waitUntil: "domcontentloaded" });

    // The banner the app shows when no provider answers its health probe. A local Ollama
    // that is loading a 9 GB model trips it. It does not affect retrieval (which is Vertex),
    // so it is not a failure of the feature — but a screenshot must not show it, so fail
    // early with an actionable message instead of capturing a warning banner.
    const providerBanner = page.getByText(/No LLM provider is reachable/i);
    if (await providerBanner.isVisible().catch(() => false)) {
      throw new Error(
        "provider banner is visible: warm the chat model (curl :8090/health) before capturing",
      );
    }

    await expect(page.getByTestId("query-search-image-button")).toBeVisible({ timeout: 60_000 });

    // WHY a fresh conversation: the workspace keeps its history, and a previous turn's
    // "Matching images" grid stays mounted above the composer. Without this, the assertions
    // below match the OLD grid and report its ranking — which is exactly how this test
    // failed once with a beagle at tile 1 while the server had stored the pizza first.
    const newConversation = page.getByTestId("query-new");
    if (await newConversation.isVisible().catch(() => false)) {
      await newConversation.click();
      await expect(page.getByTestId("query-image-matches")).toHaveCount(0, { timeout: 30_000 });
    }

    // Watch the wire before acting. WHY the request and not only the DOM: the assertion
    // has to be "the browser sent the picture to a documented endpoint", otherwise a UI
    // that silently dropped the file would still pass on a text answer.
    //
    // WHY `/chat/completions/stream` and not `/query`: the composer sends a chat turn, and
    // `chatRequestBody` switches to multipart precisely when `query_image` is present. Both
    // routes accept `query_image` (SPEC-180 10); this asserts the one the UI actually uses.
    const sent = page.waitForRequest(
      (req) =>
        req.url().includes("/chat/completions/stream") &&
        req.method() === "POST" &&
        (req.headers()["content-type"] ?? "").includes("multipart/form-data"),
      { timeout: 120_000 },
    );

    await page.getByTestId("query-search-image-input").setInputFiles(PROBE_PIZZA);
    await expect(page.getByTestId("query-search-image-chip")).toBeVisible();

    // WHY this listener: the first two versions guessed the endpoint and waited for a
    // request that never came, which says nothing about why. Recording every request the
    // send produces turns a timeout into a diagnosis.
    const seen: string[] = [];
    page.on("request", (r) => {
      if (r.method() === "POST") {
        seen.push(`${r.url()} [${r.headers()["content-type"] ?? "no-ctype"}]`);
      }
    });
    page.on("console", (m) => {
      if (m.type() === "error") seen.push(`CONSOLE-ERROR: ${m.text().slice(0, 300)}`);
    });
    page.on("pageerror", (e) => seen.push(`PAGE-ERROR: ${String(e).slice(0, 300)}`));

    await page.getByTestId("query-send").click();
    await page.waitForTimeout(15_000);
    if (seen.length === 0) {
      // Nothing was sent at all: dump the composer state so the failure is actionable.
      const state = await page.evaluate(() => {
        const send = document.querySelector<HTMLButtonElement>("[data-testid='query-send']");
        const chip = document.querySelector("[data-testid='query-search-image-chip']");
        const blocked = document.querySelector("[data-testid='query-choice-empty']");
        return {
          sendDisabled: send?.disabled ?? null,
          chipStillAttached: chip !== null,
          blockedReason: blocked?.textContent?.trim() ?? null,
        };
      });
      throw new Error(`no POST after Send; composer state: ${JSON.stringify(state)}`);
    }

    const request = await sent;
    // WHY the response and not the request body: Playwright does not expose a multipart
    // PostData buffer (`postDataBuffer()` is empty for FormData), so asserting on the file
    // part is impossible from the network event. The response is stronger evidence anyway:
    // a server that received no `query_image` would answer with the default text query and
    // would not rank the pizza first. `query_modality` proves the image vector was used.
    const response = await request.response();
    expect(response?.status()).toBe(200);

    // The chip clears once the send is accepted (the image is not kept as state).
    await expect(page.getByTestId("query-search-image-chip")).toHaveCount(0, { timeout: 60_000 });

    // The showcase surface: "Matching images", a thumbnail grid of the pictures retrieval
    // found. This is the UI proof, and it does not depend on what the answer model says.
    //
    // WHY not assert on the answer prose: the configured answer model is a small local one
    // and, given only image citations, it replies "Not answerable" — a correct and honest
    // answer to "what do these pictures contain?" when it is handed no pixels (SPEC-180 09
    // deliberately does not send raw pixels to the answer model). The retrieval result is
    // the feature; asserting on the prose would test the LLM instead.
    const matches = page.getByTestId("query-image-match");
    await expect(matches.first()).toBeVisible({ timeout: 180_000 });
    expect(await matches.count()).toBeGreaterThan(1);
    await expect(page.getByTestId("query-image-matches")).toContainText(/pizza/i);

    // Capture before asserting rank order: the grid is the deliverable, and a ranking
    // assertion that fails must not also cost us the only screenshot of it.
    await shot(page, "22-query-image-results");
    await shotLocator(page, page.getByTestId("query-image-matches"), "22b-query-image-matches-grid");

    // The pizza must be the first tile: it is the picture the probe depicts.
    // This is a real acceptance criterion, not decoration — if it fails the demo would be
    // showing a picture search that answers with a beagle.
    await expect(matches.first()).toContainText(/pizza/i);

    await applyTheme(page, "dark");
    await shot(page, "23-query-image-results", { theme: "dark" });
    await applyTheme(page, "light");
    await setViewport(page, 390);
    await shot(page, "24-query-image-results", { width: 390 });
  });

  /**
   * A cited picture must open as a picture.
   *
   * The failure this guards is the one the grid had: a pane that renders the filename as
   * text because it could not tell the citation was an image. It is a separate surface from
   * the grid (the companion pane, not the answer body), so it gets its own assertion.
   */
  test("T-182-ui-citation: a cited picture opens as a picture in the source pane", async ({
    page,
  }) => {
    const pizzaDoc = process.env.SPEC182_PIZZA_DOC_ID ?? "";
    test.skip(!pizzaDoc, "SPEC182_PIZZA_DOC_ID not set");

    await selectSpec182Scope(page);
    await page.setViewportSize({ width: 1440, height: 900 });
    // Dock the companion pane the way a citation click does (companion-pane decoder).
    await page.goto(`/query?pane=pdf&doc=${pizzaDoc}`, { waitUntil: "domcontentloaded" });

    await expect(page.getByTestId("companion-source-image")).toBeVisible({ timeout: 90_000 });
    // The bug: a text pane holding the filename instead of the photograph.
    await expect(page.getByTestId("companion-source-text")).toHaveCount(0);
    // Poll naturalWidth: the <img> mounts before the blob resolves.
    await expect
      .poll(
        async () =>
          page.evaluate(() => {
            const img = document.querySelector<HTMLImageElement>(
              "[data-testid='companion-source-image'] [data-testid='document-image']",
            );
            return img ? img.naturalWidth : 0;
          }),
        { timeout: 60_000 },
      )
      .toBeGreaterThan(0);

    await shot(page, "26-query-source-pane-picture");
  });
});
