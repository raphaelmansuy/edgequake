#!/usr/bin/env node
// Give every ```mermaid fence under docs/ the shared EdgeQuake palette so it
// looks the same on GitHub and on the docs site. Idempotent.
//   node scripts/style_docs_mermaid.mjs [--check] [path ...]
// A diagram that already starts with its own `%%{init` (no marker) or a YAML
// frontmatter block is left alone.
import { readFileSync, writeFileSync, readdirSync, statSync } from "node:fs";
import { join, resolve, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const theme = JSON.parse(readFileSync(join(root, "scripts", "mermaid-theme.json"), "utf8"));
const MARK = "%% eq-theme:v1";
const CORE = ["primaryColor", "primaryBorderColor", "primaryTextColor", "secondaryColor", "secondaryBorderColor", "secondaryTextColor", "tertiaryColor", "tertiaryBorderColor", "tertiaryTextColor", "lineColor", "clusterBkg", "clusterBorder", "noteBkgColor", "noteTextColor", "textColor", "titleColor", "signalColor", "signalTextColor", "loopTextColor", "edgeLabelBackground", "actorLineColor"];
const core = Object.fromEntries(CORE.map((k) => [k, theme.themeVariables[k]]));
const header = `%%{init: ${JSON.stringify({ theme: theme.theme, themeVariables: core })}}%%\n${MARK}\n`;

const CLASS_MARK = "%% eq-classes";
const roles = theme.roles.map(([cls, re]) => [cls, new RegExp(re, "i")]);
const NODE = /(?:^|[\s;&>|-])([A-Za-z_][\w]*)\s*(?:\[\(|\[\[|\(\(|\[|\(|\{|>)\s*"?([^"\]\)\}\n]*)/g;
const SKIP = /^\s*(subgraph|class|classDef|style|linkStyle|click|direction|end|%%)/;
// Flowcharts only: tag nodes by role from their label, so colour carries meaning.
function withRoleClasses(body) {
  const cut = body.indexOf(CLASS_MARK);
  const clean = (cut >= 0 ? body.slice(0, cut) : body).replace(/\s+$/, "\n");
  if (!/^\s*(?:%%[^\n]*\n\s*)*(flowchart|graph)\b/m.test(clean)) return clean;
  const hit = {};
  for (const line of clean.split("\n")) {
    if (SKIP.test(line)) continue;
    for (const m of line.matchAll(NODE)) {
      const role = roles.find(([, re]) => re.test(m[2]));
      if (role) (hit[role[0]] ??= new Set()).add(m[1]);
    }
  }
  const used = Object.keys(hit);
  if (!used.length) return clean;
  const defs = used.map((c) => `classDef ${c} ${theme.classes[c]}`);
  const uses = used.map((c) => `class ${[...hit[c]].join(",")} ${c}`);
  return `${clean}${CLASS_MARK}\n${[...defs, ...uses].join("\n")}\n`;
}

const args = process.argv.slice(2);
const check = args.includes("--check");
const targets = args.filter((a) => !a.startsWith("--"));
const walk = (p, out = []) => {
  if (statSync(p).isDirectory()) for (const f of readdirSync(p)) walk(join(p, f), out);
  else if (/\.mdx?$/.test(p)) out.push(p);
  return out;
};
let changed = 0, skipped = 0, files = [];
for (const t of targets.length ? targets : ["docs"]) files.push(...walk(resolve(root, t)));
for (const file of files) {
  const src = readFileSync(file, "utf8");
  const out = src.replace(/^```mermaid[^\n]*\n([\s\S]*?)^```/gm, (all, body) => {
    const lines = body.split("\n");
    let start = 0;
    if (lines[0].startsWith("%%{init") && lines[1] === MARK) start = 2;      // ours: refresh
    else if (lines[0].startsWith("%%{init") || lines[0].trim() === "---") { skipped++; return all; }
    const rest = withRoleClasses(lines.slice(start).join("\n"));
    return all.replace(body, header + rest);
  });
  if (out !== src) { changed++; if (!check) writeFileSync(file, out); }
}
console.log(`${check ? "would restyle" : "restyled"} ${changed} file(s); left ${skipped} custom diagram(s)`);
process.exit(check && changed ? 1 : 0);
