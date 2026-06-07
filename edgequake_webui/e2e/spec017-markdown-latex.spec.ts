/**
 * SPEC-017 — LaTeX / KaTeX markdown display proof.
 * Artifacts: specs/017-dry-and-solid-audit/013-edgequake-webui/e2e/
 */
import { expect, test } from "@playwright/test";
import { GOTO_OPTS } from "./helpers/app-ready";
import {
  spec017LatexScreenshot,
} from "./helpers/spec017-latex-artifacts";

test.describe("SPEC-017 markdown LaTeX rendering", () => {
  test("fixture page renders KaTeX for all delimiter styles", async ({ page }) => {
    test.setTimeout(90_000);
    // Use localhost (not 127.0.0.1) so Next.js dev HMR hydrates the client bundle.
    await page.goto("/e2e-fixtures/markdown-latex", {
      ...GOTO_OPTS,
      waitUntil: "load",
    });

    await page.waitForSelector('[data-testid="markdown-latex-fixture"]', {
      timeout: 60_000,
    });
    await expect(page.getByText("LaTeX rendering proof").first()).toBeVisible();

    const katexNodes = page.locator(".katex");
    await expect(katexNodes.first()).toBeVisible({ timeout: 15_000 });
    expect(await katexNodes.count()).toBeGreaterThanOrEqual(4);

    await page.screenshot({
      path: spec017LatexScreenshot("01-markdown-latex-fixture-full.png"),
      fullPage: true,
    });

    await page.getByTestId("markdown-latex-fixture").screenshot({
      path: spec017LatexScreenshot("02-markdown-latex-fixture-panel.png"),
    });

    const bodyText = (await page.textContent("body")) ?? "";
    expect(bodyText).not.toContain("$E = mc^2$");
  });
});
