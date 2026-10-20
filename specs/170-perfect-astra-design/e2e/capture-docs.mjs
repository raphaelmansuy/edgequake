// Capture representative Starlight docs pages × desktop/mobile × light/dark.
// Routes chosen to cover sidebar, code, mermaid, dense nav, and long prose.
import pw from "/Users/raphaelmansuy/Github/03-working/edgequake/edgequake_webui/node_modules/playwright-core/index.js";
const { chromium } = pw;
import { mkdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";

const ROOT =
  "/Users/raphaelmansuy/Github/03-working/edgequake/specs/170-perfect-astra-design/e2e/screenshots/docs";
mkdirSync(ROOT, { recursive: true });
const BASE = process.argv[2] || "http://127.0.0.1:4321";

const ROUTES = [
  ["docs-index", "/docs/"],
  ["docs-quick-start", "/docs/getting-started/quick-start/"],
  ["docs-architecture", "/docs/architecture/"],
  ["docs-api-reference", "/docs/api-reference/"],
  ["docs-faq", "/docs/faq/"],
];

const VP = {
  desktop: { width: 1440, height: 900 },
  mobile: { width: 390, height: 844 },
};

async function run() {
  const browser = await chromium.launch();
  const index = [];
  for (const [name, path] of ROUTES) {
    for (const [vn, vp] of Object.entries(VP)) {
      for (const theme of ["light", "dark"]) {
        const ctx = await browser.newContext({
          viewport: vp,
          deviceScaleFactor: 1,
          colorScheme: theme,
        });
        const page = await ctx.newPage();
        await page.addInitScript((t) => {
          localStorage.setItem("starlight-theme", t);
          document.documentElement.setAttribute("data-theme", t);
        }, theme);
        const resp = await page.goto(BASE + path, {
          waitUntil: "load",
          timeout: 45000,
        });
        await page.evaluate(async () => {
          await (document.fonts ? document.fonts.ready : 0);
          document
            .querySelectorAll("img[loading=lazy]")
            .forEach((i) => (i.loading = "eager"));
          for (let y = 0; y <= document.body.scrollHeight; y += 500) {
            scrollTo(0, y);
            await new Promise((r) => setTimeout(r, 20));
          }
          scrollTo(0, 0);
        });
        await page.waitForTimeout(400);

        // Audit prose link styles (global a reset can bleach Starlight links).
        const linkAudit = await page.evaluate(() => {
          const links = [
            ...document.querySelectorAll(".sl-markdown-content a[href]"),
          ].slice(0, 8);
          return links.map((a) => {
            const cs = getComputedStyle(a);
            return {
              text: (a.textContent || "").trim().slice(0, 40),
              color: cs.color,
              decoration: cs.textDecorationLine,
              href: a.getAttribute("href"),
            };
          });
        });

        const file = `${name}-${vn}-${theme}.png`;
        await page.screenshot({ path: join(ROOT, file), fullPage: true });
        index.push({
          name,
          path,
          vn,
          theme,
          status: resp?.status() ?? 0,
          file,
          linkAudit,
        });
        console.log(`✓ ${file} (links=${linkAudit.length})`);
        await ctx.close();
      }
    }
  }
  writeFileSync(join(ROOT, "index.json"), JSON.stringify(index, null, 2));
  await browser.close();
}

run().catch((e) => {
  console.error(e);
  process.exit(1);
});
