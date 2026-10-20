#!/usr/bin/env node
/**
 * Recapture Astra picture-tour + showcase screenshots from demo.edgequake.com
 * so UI/API badges show v0.36.x.
 *
 *   DEMO_ACCESS_URL='https://demo.edgequake.com/access/…' node capture-tutorial-demo.mjs
 *
 * Never commit DEMO_ACCESS_URL. Magic links may be one-time; keep one browser session.
 */
import pw from "/Users/raphaelmansuy/Github/03-working/edgequake/edgequake_webui/node_modules/playwright-core/index.js";
import { mkdirSync, copyFileSync, writeFileSync, existsSync } from "node:fs";
import { join } from "node:path";
import { execFileSync } from "node:child_process";

const { chromium } = pw;
const ROOT = "/Users/raphaelmansuy/Github/03-working/edgequake";
const OUT_TUTORIAL = join(ROOT, "tutorial/images");
const OUT_PUBLIC = join(ROOT, "edgequake-website/public/tutorial");
const OUT_SHOWCASE = join(ROOT, "edgequake-website/public");
const BASE = "https://demo.edgequake.com";
const ACCESS = process.env.DEMO_ACCESS_URL;

if (!ACCESS) {
  console.error("Set DEMO_ACCESS_URL to the full magic-link URL.");
  process.exit(2);
}

mkdirSync(OUT_TUTORIAL, { recursive: true });
mkdirSync(OUT_PUBLIC, { recursive: true });

const browser = await chromium.launch({ headless: true, channel: "chrome" });
const ctx = await browser.newContext({
  viewport: { width: 1600, height: 1000 },
  deviceScaleFactor: 2,
});
const page = await ctx.newPage();
page.setDefaultTimeout(45000);

async function forceLight() {
  await page.evaluate(() => {
    document.documentElement.classList.remove("dark");
    document.documentElement.classList.add("light");
    document.documentElement.setAttribute("data-theme", "light");
    try {
      localStorage.setItem("theme", "light");
    } catch {}
  });
  // Prefer UI control if present
  const btn = page.getByRole("button", { name: /toggle theme|theme/i }).first();
  if ((await btn.count().catch(() => 0)) > 0) {
    await btn.click().catch(() => {});
    await page.waitForTimeout(400);
    const light = page.getByRole("menuitem", { name: /^light$/i }).first();
    if ((await light.count().catch(() => 0)) > 0) {
      await light.click().catch(() => {});
    } else {
      // dismiss if already light
      await page.keyboard.press("Escape").catch(() => {});
    }
    await page.waitForTimeout(500);
  }
}

function writeBoth(name, bufPath) {
  const destA = join(OUT_TUTORIAL, name);
  const destB = join(OUT_PUBLIC, name);
  copyFileSync(bufPath, destA);
  copyFileSync(bufPath, destB);
  console.log(`  → ${name}`);
}

async function shot(name, { fullPage = false } = {}) {
  await forceLight();
  await page.waitForTimeout(800);
  const tmp = join(OUT_PUBLIC, `.tmp-${name}`);
  await page.screenshot({ path: tmp, fullPage });
  writeBoth(name, tmp);
  try {
    const { unlinkSync } = await import("node:fs");
    unlinkSync(tmp);
  } catch {}
}

async function goto(path) {
  await page.goto(BASE + path, { waitUntil: "networkidle", timeout: 90000 });
  await forceLight();
  await page.waitForTimeout(1200);
}

// --- Auth via magic link ---
console.log("Opening magic link…");
await page.goto(ACCESS, { waitUntil: "networkidle", timeout: 90000 });
await page.waitForTimeout(2500);

// Wait until we leave access/login
for (let i = 0; i < 20; i++) {
  const path = new URL(page.url()).pathname;
  if (!path.startsWith("/access") && path !== "/login") break;
  // Click redeem / continue if present
  const go = page.getByRole("button", { name: /continue|accept|sign in|open|redeem|use link/i }).first();
  if ((await go.count().catch(() => 0)) > 0) await go.click().catch(() => {});
  await page.waitForTimeout(1500);
}
console.log("session path:", new URL(page.url()).pathname);

await forceLight();
await page.waitForTimeout(1500);

const body = await page.evaluate(() => document.body?.innerText ?? "");
if (!/0\.36/.test(body) && !/v0\.36/.test(body)) {
  // Header badge may be delayed — hit health
  const health = await page
    .evaluate(async () => {
      try {
        const r = await fetch("/health");
        return await r.text();
      } catch (e) {
        return String(e);
      }
    })
    .catch(() => "");
  console.log("health snippet:", health.slice(0, 200));
  const combined = body + health;
  if (!/0\.36/.test(combined)) {
    console.error("FAIL: demo does not appear to be on v0.36.x (still seeing older pin?).");
    console.error("body sample:", body.slice(0, 400));
    await browser.close();
    process.exit(1);
  }
}
console.log("✓ version assert: 0.36 present");

const log = [];

// --- Sign-in (logged-out capture in a fresh context) ---
{
  const ctx2 = await browser.newContext({
    viewport: { width: 1600, height: 1000 },
    deviceScaleFactor: 2,
  });
  const p2 = await ctx2.newPage();
  await p2.goto(BASE + "/login", { waitUntil: "networkidle" });
  await p2.waitForTimeout(1200);
  const tmp = join(OUT_PUBLIC, ".tmp-02-sign-in.png");
  await p2.screenshot({ path: tmp });
  writeBoth("02-sign-in.png", tmp);
  try {
    const { unlinkSync } = await import("node:fs");
    unlinkSync(tmp);
  } catch {}
  await ctx2.close();
  log.push("02-sign-in.png");
}

// Dashboard + workspace switcher
await goto("/");
await shot("03-dashboard.png");
log.push("03-dashboard.png");

// Workspace switcher — click org/workspace chip in header
{
  const chip = page
    .locator("button, [role=button]")
    .filter({ hasText: /workspace|default/i })
    .first();
  if ((await chip.count().catch(() => 0)) > 0) {
    await chip.click().catch(() => {});
    await page.waitForTimeout(800);
  } else {
    // fallback: text near header
    await page.getByText(/Default Workspace|Default —/i).first().click().catch(() => {});
    await page.waitForTimeout(800);
  }
  await shot("02-workspace-switcher.png");
  await page.keyboard.press("Escape").catch(() => {});
  log.push("02-workspace-switcher.png");
}

await goto("/documents");
await shot("04-documents.png");
log.push("04-documents.png");

// Document preview / detail — click first completed/ready row
{
  const row = page.locator("table tbody tr, [data-testid='document-row'], a[href*='/documents/']").first();
  if ((await row.count().catch(() => 0)) > 0) {
    await row.click().catch(() => {});
    await page.waitForTimeout(2500);
    await shot("05-document-preview.png");
    log.push("05-document-preview.png");
    // Prefer open / view document if a button exists
    const open = page.getByRole("button", { name: /open|view|preview/i }).first();
    if ((await open.count().catch(() => 0)) > 0) {
      await open.click().catch(() => {});
      await page.waitForTimeout(2500);
    }
    await shot("05-document.png", { fullPage: false });
    log.push("05-document.png");
  } else {
    console.warn("! no document row — skipping 05-*");
  }
}

await goto("/query");
await shot("06-query.png");
log.push("06-query.png");

// Modes menu
{
  const mode = page.getByRole("button", { name: /smart|mode|retrieval/i }).first();
  if ((await mode.count().catch(() => 0)) > 0) {
    await mode.click().catch(() => {});
    await page.waitForTimeout(700);
    await shot("06-query-modes.png");
    await page.keyboard.press("Escape").catch(() => {});
    log.push("06-query-modes.png");
  }
}

// Answer — click an example question if present
{
  const ex = page.locator("button, a").filter({ hasText: /\?$/ }).first();
  if ((await ex.count().catch(() => 0)) > 0) {
    await ex.click().catch(() => {});
  } else {
    const box = page.locator("textarea, [contenteditable=true], input[type=text]").first();
    if ((await box.count().catch(() => 0)) > 0) {
      await box.fill("What is this workspace about?").catch(() => {});
      await page.getByRole("button", { name: /ask|send|submit|search/i }).first().click().catch(() => {});
    }
  }
  // Wait for answer content (may take a while)
  await page.waitForTimeout(12000);
  await shot("06-answer.png");
  log.push("06-answer.png");
}

// Graph
await goto("/");
await page.getByText("Knowledge Graph", { exact: false }).first().click().catch(() => {});
await page.waitForTimeout(7000);
await shot("07-graph.png");
log.push("07-graph.png");
{
  // Click a canvas / entity list item
  const ent = page.locator("[data-testid='entity-list'] button, aside button, .entity-item").first();
  if ((await ent.count().catch(() => 0)) > 0) {
    await ent.click().catch(() => {});
    await page.waitForTimeout(1500);
  } else {
    const canvas = page.locator("canvas").first();
    if ((await canvas.count().catch(() => 0)) > 0) {
      const box = await canvas.boundingBox();
      if (box) await page.mouse.click(box.x + box.width / 2, box.y + box.height / 2);
      await page.waitForTimeout(1500);
    }
  }
  await shot("07-graph-node.png");
  log.push("07-graph-node.png");
}

await goto("/knowledge");
await shot("08-knowledge.png");
log.push("08-knowledge.png");
{
  const create = page.getByRole("button", { name: /create|new|inject/i }).first();
  if ((await create.count().catch(() => 0)) > 0) {
    await create.click().catch(() => {});
    await page.waitForTimeout(1000);
    await shot("08-knowledge-new.png");
    await page.keyboard.press("Escape").catch(() => {});
    log.push("08-knowledge-new.png");
  }
}

await goto("/workspace");
await shot("09-workspace.png", { fullPage: true });
log.push("09-workspace.png");

await goto("/pipeline");
await shot("10-pipeline.png");
log.push("10-pipeline.png");

await goto("/costs");
await shot("11-costs.png");
log.push("11-costs.png");

await goto("/api-explorer");
await shot("12-api-explorer.png");
log.push("12-api-explorer.png");

await goto("/settings");
await shot("13-settings.png");
log.push("13-settings.png");

// Settings sections — scroll / click tabs if present
async function settingsSection(name, label) {
  const tab = page.getByRole("tab", { name: new RegExp(label, "i") }).first();
  const link = page.getByRole("button", { name: new RegExp(label, "i") }).first();
  const heading = page.getByText(new RegExp(label, "i")).first();
  if ((await tab.count().catch(() => 0)) > 0) await tab.click().catch(() => {});
  else if ((await link.count().catch(() => 0)) > 0) await link.click().catch(() => {});
  else if ((await heading.count().catch(() => 0)) > 0) {
    await heading.scrollIntoViewIfNeeded().catch(() => {});
  }
  await page.waitForTimeout(800);
  await shot(name);
  log.push(name);
}

await settingsSection("13-settings-graph.png", "Graph");
await settingsSection("13-settings-query.png", "Query");
await settingsSection("13-settings-connections.png", "Connection");
await settingsSection("13-settings-data.png", "Data");

await goto("/members");
await shot("14-members.png");
log.push("14-members.png");
{
  const invite = page.getByRole("button", { name: /invite/i }).first();
  if ((await invite.count().catch(() => 0)) > 0) {
    await invite.click().catch(() => {});
    await page.waitForTimeout(1000);
    await shot("14-invite.png");
    await page.keyboard.press("Escape").catch(() => {});
    log.push("14-invite.png");
  }
  const row = page.locator("table tbody tr, [data-testid='member-row']").first();
  if ((await row.count().catch(() => 0)) > 0) {
    await row.click().catch(() => {});
    await page.waitForTimeout(1000);
    await shot("14-member-detail.png");
    log.push("14-member-detail.png");
  }
}

// Showcase (marketing home) — light dashboard + query
await goto("/");
{
  const tmp = join(OUT_SHOWCASE, "showcase-dashboard-light.png");
  await page.screenshot({ path: tmp });
  maybeWebp(tmp);
  console.log("  → showcase-dashboard-light.png");
}
await goto("/query");
{
  const tmp = join(OUT_SHOWCASE, "showcase-query-light.png");
  await page.screenshot({ path: tmp });
  maybeWebp(tmp);
  console.log("  → showcase-query-light.png");
}

function maybeWebp(pngPath) {
  const webp = pngPath.replace(/\.png$/, ".webp");
  try {
    execFileSync("cwebp", ["-q", "82", pngPath, "-o", webp], { stdio: "ignore" });
    console.log("  →", webp.split("/").pop());
  } catch {
    // Keep existing webp if cwebp missing
    if (!existsSync(webp)) console.warn("! cwebp not available; left old webp for", webp);
  }
}

writeFileSync(
  join(ROOT, "specs/170-perfect-astra-design/e2e/screenshots/tutorial-capture-log.json"),
  JSON.stringify({ at: new Date().toISOString(), files: log }, null, 2),
);

await browser.close();
console.log(`done — ${log.length} tutorial shots`);
