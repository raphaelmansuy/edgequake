/**
 * SPEC-168 diagnostic: names elements that poke past the viewport's right edge
 * (the usual cause of 2-up grids rendering one clipped column on phones).
 * Opt-in: SPEC168_DIAG=dashboard,settings [SPEC168_DIAG_WIDTH=375]
 */
import { test } from "@playwright/test";
import { prepareSpec168Page } from "./helpers/rich-mock";
import { ROUTES } from "./helpers/routes";

const ONLY = (process.env.SPEC168_DIAG ?? "").split(",").filter(Boolean);
const WIDTH = Number(process.env.SPEC168_DIAG_WIDTH ?? 375);

test.describe("SPEC-168 overflow diagnostics @spec168", () => {
  test.skip(ONLY.length === 0, "set SPEC168_DIAG=<route,...>");
  for (const [name, route] of ROUTES) {
    if (!ONLY.includes(name)) continue;
    test(`overflow ${name}`, async ({ page }) => {
      await prepareSpec168Page(page, {});
      await page.setViewportSize({ width: WIDTH, height: 812 });
      await page.goto(route, { waitUntil: "domcontentloaded" });
      await page.waitForTimeout(2500);
      const rows = await page.evaluate((vw) => {
        const out: string[] = [];
        for (const el of Array.from(document.querySelectorAll("body *"))) {
          const r = el.getBoundingClientRect();
          if (r.width === 0 || r.right <= vw + 1) continue;
          const cs = getComputedStyle(el);
          if (cs.position === "fixed" || cs.visibility === "hidden") continue;
          const own = Array.from(el.children).every((c) => c.getBoundingClientRect().right <= vw + 1);
          if (!own) continue; // only report the innermost offenders
          const cls = String((el as HTMLElement).className).replace(/\s+/g, " ").slice(0, 90);
          out.push(`${Math.round(r.right)}px ${el.tagName.toLowerCase()}.${cls} «${(el.textContent ?? "").trim().slice(0, 30)}»`);
        }
        // Widest chain: deepest elements that are themselves wider than the viewport.
        const wide = Array.from(document.querySelectorAll("body *"))
          .filter((el) => el.getBoundingClientRect().width > vw + 1 && !el.closest("svg"))
          .map((el) => {
            let depth = 0;
            for (let n: Element | null = el; n; n = n.parentElement) depth++;
            return { el, depth };
          })
          .sort((a, b) => b.depth - a.depth)
          .slice(0, 6)
          .map(({ el }) => `WIDE ${Math.round(el.getBoundingClientRect().width)}px ${el.tagName.toLowerCase()}.${String((el as HTMLElement).className).replace(/\s+/g, " ").slice(0, 100)}`);
        return [...wide, ...out.slice(0, 6)];
      }, WIDTH);
      console.log(`OVERFLOW ${name}@${WIDTH}:\n${rows.join("\n") || "(none)"}`);
    });
  }
});
