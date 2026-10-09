#!/usr/bin/env node
// Validate every ```mermaid fence under docs/ with the real mermaid parser.
// Usage (from repo root): node scripts/check_docs_mermaid.mjs [path ...]
// Needs deps from edgequake-website (pnpm install there first).
import { createRequire } from "node:module";
import { readFileSync, readdirSync, statSync } from "node:fs";
import { join, resolve } from "node:path";

const root = resolve(new URL("..", import.meta.url).pathname);
const require = createRequire(join(root, "edgequake-website", "package.json"));
const { JSDOM } = require("jsdom");
const dom = new JSDOM("<!doctype html><body></body>");
globalThis.window = dom.window;
globalThis.document = dom.window.document;
const mermaid = (await import(require.resolve("mermaid/dist/mermaid.core.mjs"))).default;
mermaid.initialize({ startOnLoad: false });

function walk(p, out = []) {
  const st = statSync(p);
  if (st.isDirectory()) for (const f of readdirSync(p)) walk(join(p, f), out);
  else if (p.endsWith(".md") || p.endsWith(".mdx")) out.push(p);
  return out;
}

const targets = process.argv.slice(2).length ? process.argv.slice(2) : [join(root, "docs")];
let blocks = 0, bad = 0;
for (const t of targets) for (const file of walk(resolve(t))) {
  const text = readFileSync(file, "utf8");
  const re = /^```mermaid[^\n]*\n([\s\S]*?)^```/gm;
  let m;
  while ((m = re.exec(text))) {
    blocks++;
    const line = text.slice(0, m.index).split("\n").length;
    try { await mermaid.parse(m[1]); }
    catch (e) { bad++; console.error(`FAIL ${file.replace(root + "/", "")}:${line}: ${String(e.message || e).split("\n")[0]}`); }
  }
}
console.log(`mermaid blocks: ${blocks}, failures: ${bad}`);
process.exit(bad ? 1 : 0);
