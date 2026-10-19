/**
 * SPEC-168 diagnostic: names the DOM nodes behind every layout shift.
 * Opt-in: SPEC168_DIAG=settings,login pnpm exec playwright test e2e/spec168/diag-shift.spec.ts
 */
import { test } from "@playwright/test";
import { prepareSpec168Page } from "./helpers/rich-mock";
import { ROUTES } from "./helpers/routes";

const ONLY = (process.env.SPEC168_DIAG ?? "").split(",").filter(Boolean);
const WIDTH = Number(process.env.SPEC168_DIAG_WIDTH ?? 390);

test.describe("SPEC-168 shift diagnostics @spec168", () => {
  test.skip(ONLY.length === 0, "set SPEC168_DIAG=<route,...>");
  for (const [name, route] of ROUTES) {
    if (!ONLY.includes(name)) continue;
    test(`diag ${name}`, async ({ page }) => {
      await page.addInitScript(() => {
        const w = window as unknown as { __shifts: unknown[] };
        w.__shifts = [];
        new PerformanceObserver((list) => {
          for (const e of list.getEntries()) {
            const ls = e as PerformanceEntry & {
              value: number;
              hadRecentInput: boolean;
              sources?: Array<{ node?: Element; previousRect: DOMRect; currentRect: DOMRect }>;
            };
            if (ls.hadRecentInput) continue;
            w.__shifts.push({
              t: Math.round(ls.startTime),
              value: ls.value,
              sources: (ls.sources ?? []).map((s) => ({
                node: s.node
                  ? `${s.node.tagName.toLowerCase()}.${String((s.node as HTMLElement).className?.toString?.() ?? "").slice(0, 70)}|${(s.node.parentElement?.getAttribute("data-testid") ?? s.node.getAttribute("data-testid") ?? "")} «${(s.node.textContent ?? "").trim().slice(0, 40)}»`
                  : "(removed)",
                from: [s.previousRect.x, s.previousRect.y, s.previousRect.width, s.previousRect.height].map(Math.round),
                to: [s.currentRect.x, s.currentRect.y, s.currentRect.width, s.currentRect.height].map(Math.round),
              })),
            });
          }
        }).observe({ type: "layout-shift", buffered: true } as PerformanceObserverInit);
      });
      await prepareSpec168Page(page, { empty: process.env.SPEC168_EMPTY === "1" });
      await page.setViewportSize({ width: WIDTH, height: WIDTH < 500 ? 844 : 900 });
      await page.goto(route, { waitUntil: "domcontentloaded" });
      await page.waitForTimeout(3500);
      const shifts = await page.evaluate(() => (window as unknown as { __shifts: unknown[] }).__shifts);
      console.log(`SHIFTS ${name}@${WIDTH}: ${JSON.stringify(shifts, null, 1)}`);
    });
  }
});
