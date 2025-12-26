#!/usr/bin/env node
import fs from 'node:fs';
import path from 'node:path';

function usage(exitCode = 1) {
  const msg = `\
Usage:
  node derive_routes.mjs [--format json|jsonl]

Emits a list of concrete Next.js App Router routes based on page.tsx files.
- Ignores route groups: (group)
- Skips dynamic segments: [param], [...param], [[...param]]

Default format: json
`;
  process.stderr.write(msg);
  process.exit(exitCode);
}

function parseArgs(argv) {
  const args = { format: 'json' };
  for (let i = 0; i < argv.length; i++) {
    const token = argv[i];
    if (token === '--format') {
      const value = argv[i + 1];
      if (!value) usage(1);
      args.format = value;
      i++;
    } else {
      usage(1);
    }
  }
  if (!['json', 'jsonl'].includes(args.format)) usage(1);
  return args;
}

function repoRootFromSkillDir() {
  // skillDir: <repo>/.github/skills/playwright-ux-ui-capture/scripts
  return path.resolve(process.cwd(), '../../../../..');
}

function isDynamicSegment(segment) {
  return /^\[\[?\.{0,3}.+\]?\]$/.test(segment) || segment.startsWith('[');
}

function stripRouteGroups(segment) {
  // (dashboard) -> null (ignored)
  if (segment.startsWith('(') && segment.endsWith(')')) return null;
  return segment;
}

function walk(dir, onFile) {
  const entries = fs.readdirSync(dir, { withFileTypes: true });
  for (const entry of entries) {
    const full = path.join(dir, entry.name);
    if (entry.isDirectory()) walk(full, onFile);
    else if (entry.isFile()) onFile(full);
  }
}

function routeFromPageTsx(appRoot, pageFilePath) {
  const rel = path.relative(appRoot, pageFilePath);
  const parts = rel.split(path.sep);
  if (parts[parts.length - 1] !== 'page.tsx') return null;

  const segments = parts.slice(0, -1);
  const urlSegments = [];

  for (const seg of segments) {
    const cleaned = stripRouteGroups(seg);
    if (!cleaned) continue;
    if (isDynamicSegment(cleaned)) return null;
    urlSegments.push(cleaned);
  }

  const route = '/' + urlSegments.join('/');
  return route === '/' ? '/' : route.replace(/\/+$/, '');
}

function slugFromRoute(route) {
  if (route === '/') return 'dashboard';
  return route
    .replace(/^\//, '')
    .split('/')
    .filter(Boolean)
    .join('-');
}

const args = parseArgs(process.argv.slice(2));
const repoRoot = repoRootFromSkillDir();
const appRoot = path.join(repoRoot, 'edgequake_webui', 'src', 'app');

if (!fs.existsSync(appRoot)) {
  process.stderr.write(`Expected Next.js app dir not found: ${appRoot}\n`);
  process.exit(2);
}

const routes = [];
walk(appRoot, (filePath) => {
  if (!filePath.endsWith(path.join(path.sep, 'page.tsx'))) return;
  const route = routeFromPageTsx(appRoot, filePath);
  if (!route) return;
  routes.push({
    page: slugFromRoute(route),
    route,
    source: path.relative(repoRoot, filePath),
  });
});

// Deduplicate by route
const uniqueByRoute = new Map();
for (const r of routes) uniqueByRoute.set(r.route, r);
const out = Array.from(uniqueByRoute.values()).sort((a, b) => a.route.localeCompare(b.route));

if (args.format === 'json') {
  process.stdout.write(JSON.stringify(out, null, 2) + '\n');
} else {
  for (const row of out) process.stdout.write(JSON.stringify(row) + '\n');
}
