/**
 * SPEC-168: the documents table header must share the exact width and edges of
 * the rows, before and after scrolling (header is sticky inside the scroller).
 */
import { expect, test } from "@playwright/test";
import { prepareSpec168Page } from "./helpers/rich-mock";

for (const [w, h] of [[1440, 900], [1024, 640], [768, 800]] as const) {
  test(`documents table header aligns with rows ${w}x${h} @spec168`, async ({ page }) => {
    await prepareSpec168Page(page, {});
    await page.setViewportSize({ width: w, height: h });
    await page.goto("/documents", { waitUntil: "domcontentloaded" });
    const head = page.getByTestId("documents-table-head");
    await expect(head).toBeVisible({ timeout: 20_000 });
    const edges = async () =>
      page.evaluate(() => {
        const box = (el: Element | null) => {
          const r = el!.getBoundingClientRect();
          return { left: Math.round(r.left), right: Math.round(r.right), top: Math.round(r.top) };
        };
        const th = document.querySelector("[data-testid='documents-table-head'] > div")!;
        const body = document.querySelector("[data-testid='documents-virtual-spacer'] > div")!;
        const cell = (root: Element, i: number) => box(root.querySelectorAll("tr")[0].children[i]);
        return {
          head: box(th),
          body: box(body),
          headTitle: cell(th, 1),
          rowTitle: cell(body, 1),
          headLast: cell(th, th.querySelectorAll("tr")[0].children.length - 1),
          rowLast: cell(body, body.querySelectorAll("tr")[0].children.length - 1),
        };
      });
    const check = (e: Awaited<ReturnType<typeof edges>>) => {
      expect(e.head.left).toBe(e.body.left);
      expect(e.head.right).toBe(e.body.right);
      expect(e.headTitle.left).toBe(e.rowTitle.left);
      expect(e.headLast.right).toBe(e.rowLast.right);
    };
    check(await edges());
    const top = (await head.boundingBox())!.y;
    await page.getByTestId("documents-table-scroll").evaluate((el) => (el.scrollTop = 120));
    await page.waitForTimeout(200);
    check(await edges());
    expect(Math.abs((await head.boundingBox())!.y - top)).toBeLessThanOrEqual(1); // pinned
    await page.screenshot({ path: `/tmp/table-${w}x${h}.png` });
  });
}
