/**
 * SPEC-168 CLS gate — every route, three persisted-preference states.
 *
 * Budget is far stricter than web.dev "good" (0.1): perfect = no unexpected
 * movement (<= 0.005). States cover the classic hydration shift sources:
 * persisted collapsed sidebar and persisted dark theme.
 *
 * Writes specs/168-perfect-ui/e2e/reports/cls-<round>.json for the ledger.
 * Set SPEC168_CLS_REPORT_ONLY=1 to record without failing (baseline runs).
 */
import fs from "node:fs";
import path from "node:path";
import { expect, test } from "@playwright/test";
import {
  installStabilityProbe,
  readStability,
} from "../helpers/stability-probe";
import { prepareSpec168Page } from "./helpers/rich-mock";
import { ROUND, SPEC168_SHOTS_ROOT } from "./helpers/shots";
import { ROUTES } from "./helpers/routes";

export const CLS_PERFECT = 0.005;
const REPORT_ONLY = process.env.SPEC168_CLS_REPORT_ONLY === "1";
const REPORT_DIR = path.join(SPEC168_SHOTS_ROOT, "..", "reports");

const STATES = [
  { id: "default", opts: {} },
  { id: "collapsed-dark", opts: { collapsedSidebar: true, theme: "dark" as const } },
  { id: "empty-workspace", opts: { empty: true } },
] as const;

type Row = { route: string; state: string; width: number; cls: number; shifts: number };
const rows: Row[] = [];

test.describe.configure({ mode: "serial" });

test.afterAll(() => {
  fs.mkdirSync(REPORT_DIR, { recursive: true });
  fs.writeFileSync(
    path.join(REPORT_DIR, `cls-${ROUND}.json`),
    JSON.stringify({ budget: CLS_PERFECT, rows }, null, 2),
  );
});

for (const state of STATES) {
  for (const width of [1440, 390]) {
    test.describe(`CLS ${state.id} @${width} @spec168`, () => {
      for (const [name, route] of ROUTES) {
        if (name === "api-explorer") continue; // third-party Scalar iframe-like renderer
        test(`${name}`, async ({ page }) => {
          test.setTimeout(90_000);
          await installStabilityProbe(page);
          await prepareSpec168Page(page, state.opts);
          await page.setViewportSize({ width, height: width < 500 ? 844 : 900 });
          await page.goto(route, { waitUntil: "domcontentloaded" });
          await page.waitForTimeout(3500);
          const snap = await readStability(page);
          rows.push({ route: name, state: state.id, width, cls: snap.cls, shifts: snap.shifts });
          if (!REPORT_ONLY) {
            expect(
              snap.cls,
              `CLS ${snap.cls.toFixed(4)} over ${CLS_PERFECT} (${snap.shifts} shifts) on ${name}/${state.id}/${width}`,
            ).toBeLessThanOrEqual(CLS_PERFECT);
          }
        });
      }
    });
  }
}
