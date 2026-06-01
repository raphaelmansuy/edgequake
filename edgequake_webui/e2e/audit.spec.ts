import { test, expect } from "@playwright/test";
import fs from "fs";
import path from "path";

test.describe("@audit site audit", () => {
  test("crawl and screenshot", async ({ page, baseURL }) => {
  const start = baseURL || "/";
  const visited = new Set<string>();
  const toVisit: { url: string; depth: number }[] = [{ url: start, depth: 0 }];
  const screenshotsDir = path.resolve(process.cwd(), "e2e/screenshots/crawl");
  const auditMd = path.resolve(process.cwd(), "specs/audit.md");

  if (!fs.existsSync(path.dirname(auditMd))) fs.mkdirSync(path.dirname(auditMd), { recursive: true });
  if (!fs.existsSync(screenshotsDir)) fs.mkdirSync(screenshotsDir, { recursive: true });

  const mdLines: string[] = ["# Site Audit", "", `base: ${start}`, ""];

  // Attach console listener early
  const consoleMessages = new Map<string, string[]>();
  page.on("console", (msg) => {
    const text = `${msg.type()}: ${msg.text()}`;
    const url = page.url() || start;
    if (!consoleMessages.has(url)) consoleMessages.set(url, []);
    consoleMessages.get(url)!.push(text);
  });

  // Limit total pages to avoid long runs
  const MAX_PAGES = 20;

  // Use faster navigation readiness and smaller per-navigation timeout
  page.setDefaultNavigationTimeout(20000);

  while (toVisit.length && visited.size < MAX_PAGES) {
    const { url, depth } = toVisit.shift()!;
    const normalized = url.split("#")[0];
    if (visited.has(normalized) || depth > 2) continue;
    visited.add(normalized);

    try {
      await page.goto(normalized, { waitUntil: "domcontentloaded" });
      const title = (await page.title()) || "(no title)";
      const pathname = new URL(normalized).pathname.replace(/\/$/, "") || "/";
      const name = pathname === "/" ? "home" : pathname.replace(/[^a-z0-9_-]/gi, "_");
      const fileName = `${visited.size.toString().padStart(2, "0")}-${name || "page"}.png`;
      const filePath = path.join(screenshotsDir, fileName);
      await page.screenshot({ path: filePath, fullPage: true });

      const h1 = await page.locator("h1").first().innerText().catch(() => "");

      mdLines.push(`## ${normalized}`);
      mdLines.push("");
      mdLines.push(`- title: ${title}`);
      mdLines.push(`- h1: ${h1 || "(none)"}`);
      mdLines.push(`- screenshot: ./e2e/screenshots/crawl/${fileName}`);

      const msgs = consoleMessages.get(page.url() || normalized) || [];
      if (msgs.length) {
        mdLines.push(`- console_messages:`);
        for (const m of msgs) mdLines.push(`  - ${m}`);
      }

      mdLines.push("");

      // discover internal links
      const anchors = await page.$$eval("a[href]", (els) => els.map((e) => (e as HTMLAnchorElement).href));
      for (const a of anchors) {
        try {
          const u = new URL(a);
          const base = new URL(start);
          if (u.origin === base.origin) {
            toVisit.push({ url: u.href.split("#")[0], depth: depth + 1 });
          }
        } catch (e) {
          // ignore malformed
        }
      }
    } catch (err) {
      mdLines.push(`## ${normalized}`);
      mdLines.push("");
      mdLines.push(`- error: ${(err as Error).message}`);
      mdLines.push("");
    }
  }

  fs.writeFileSync(auditMd, mdLines.join("\n"));
});
});
