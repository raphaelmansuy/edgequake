/**
 * SPEC-146 screenshot capture — real EdgeQuake UI only (no fixtureHtml).
 * Saves PNGs to specs/146-rbac-attributes-based-securty/e2e/screnshist/
 *
 * Requires UI on PLAYWRIGHT_BASE_URL (default http://localhost:3010).
 * Fails if the stack is down or TenantGuard is showing.
 *
 * Run:
 *   cd edgequake_webui && bun e2e/scripts/spec146-capture-screenshots.mjs
 */
import { chromium } from "@playwright/test";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = path.resolve(__dirname, "../../..");
const OUT_DIR = path.join(
  REPO_ROOT,
  "specs/146-rbac-attributes-based-securty/e2e/screnshist",
);

const BASE =
  process.env.PLAYWRIGHT_BASE_URL?.replace(/\/$/, "") ||
  "http://localhost:3010";

const VIEWPORTS = [
  { name: "desktop-1440", width: 1440, height: 900 },
  { name: "tablet-768", width: 768, height: 1024 },
  { name: "mobile-375", width: 375, height: 812 },
];

const ROUTES = [
  ["documents-breakglass", "/documents"],
  ["settings-authz", "/settings"],
  ["query-empty", "/query"],
  ["upload-labels", "/documents"],
];

async function assertLive(page) {
  const text = (await page.locator("body").innerText()).toLowerCase();
  if (
    text.includes("create tenant") ||
    text.includes("edgequake · spec-146") ||
    text.includes("this site can’t be reached") ||
    text.includes("connection refused")
  ) {
    throw new Error(
      `SPEC-146 capture refused soft evidence (TenantGuard/fixture/down) at ${page.url()}`,
    );
  }
}

async function main() {
  fs.mkdirSync(OUT_DIR, { recursive: true });
  const browser = await chromium.launch();

  try {
    const probe = await browser.newPage();
    const resp = await probe.goto(BASE, { waitUntil: "domcontentloaded", timeout: 15_000 });
    if (!resp || !resp.ok()) {
      throw new Error(
        `SPEC-146 capture FAILED: ${BASE} not healthy (status ${resp?.status()}). Start make dev-bg / UI on :3010.`,
      );
    }
    await probe.close();
  } catch (e) {
    await browser.close();
    console.error(String(e));
    process.exit(1);
  }

  for (const vp of VIEWPORTS) {
    for (const [name, route] of ROUTES) {
      const page = await browser.newPage({
        viewport: { width: vp.width, height: vp.height },
      });
      await page.goto(`${BASE}${route}`, {
        waitUntil: "domcontentloaded",
        timeout: 30_000,
      });
      await assertLive(page);
      const out = path.join(OUT_DIR, `${name}-${vp.name}.png`);
      await page.screenshot({ path: out, fullPage: true });
      console.log("wrote", out);
      await page.close();
    }
  }

  await browser.close();
  console.log("SPEC-146 live screenshots complete →", OUT_DIR);
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
