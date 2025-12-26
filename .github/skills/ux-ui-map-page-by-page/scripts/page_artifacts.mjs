#!/usr/bin/env node
import fs from 'node:fs';
import path from 'node:path';

function usage(exitCode = 1) {
  const msg = `\
Usage:
  node page_artifacts.mjs scaffold --page <slug> --route </path>
  node page_artifacts.mjs validate --page <slug>

Examples:
  node page_artifacts.mjs scaffold --page dashboard --route /
  node page_artifacts.mjs validate --page dashboard
`;
  process.stderr.write(msg);
  process.exit(exitCode);
}

function parseArgs(argv) {
  const [command, ...rest] = argv;
  if (!command) usage(1);

  const args = {};
  for (let i = 0; i < rest.length; i++) {
    const token = rest[i];
    if (!token.startsWith('--')) continue;
    const key = token.slice(2);
    const value = rest[i + 1];
    if (!value || value.startsWith('--')) usage(1);
    args[key] = value;
    i++;
  }

  return { command, args };
}

function ensureDir(dirPath) {
  fs.mkdirSync(dirPath, { recursive: true });
}

function readText(filePath) {
  return fs.readFileSync(filePath, 'utf8');
}

function writeIfMissing(filePath, content) {
  if (fs.existsSync(filePath)) return false;
  ensureDir(path.dirname(filePath));
  fs.writeFileSync(filePath, content, 'utf8');
  return true;
}

function isValidSlug(slug) {
  return /^[a-z0-9]+(?:-[a-z0-9]+)*$/.test(slug);
}

function repoRootFromSkillDir() {
  // skillDir: <repo>/.github/skills/ux-ui-map-page-by-page/scripts
  return path.resolve(process.cwd(), '../../../../..');
}

function scaffold({ page, route }) {
  if (!page || !route) usage(1);
  if (!isValidSlug(page)) {
    throw new Error(`Invalid --page slug: ${page}. Expected lowercase-hyphen.`);
  }
  if (!route.startsWith('/')) {
    throw new Error(`Invalid --route: ${route}. Expected leading '/'.`);
  }

  const repoRoot = repoRootFromSkillDir();
  const templatesDir = path.resolve(process.cwd(), '../assets');

  const uxUiMapDir = path.join(repoRoot, 'ux_ui_map');
  const pagesDir = path.join(uxUiMapDir, 'pages');
  const screenshotsDir = path.join(uxUiMapDir, 'screenshots', page);
  const requestsDir = path.join(uxUiMapDir, 'requests');

  ensureDir(pagesDir);
  ensureDir(screenshotsDir);
  ensureDir(requestsDir);

  const nowIso = new Date().toISOString();

  const pageTemplate = readText(path.join(templatesDir, 'page-template.md'))
    .replaceAll('{page}', page)
    .replaceAll('{route}', route)
    .replaceAll('{Name}', page)
    .replaceAll('{title}', '')
    .replaceAll('{layout_summary}', '')
    .replaceAll('{ascii_layout}', '')
    .replaceAll('{position}', '')
    .replaceAll('{dimensions}', '')
    .replaceAll('{behavior}', '')
    .replaceAll('{containers}', '')
    .replaceAll('{neutral_observations}', '');

  const requestTemplate = readText(path.join(templatesDir, 'request-template.json'))
    .replaceAll('{page}', page)
    .replaceAll('{route}', route)
    .replaceAll('{iso8601}', nowIso);

  const pageDocPath = path.join(pagesDir, `${page}.md`);
  const requestPath = path.join(requestsDir, `${page}.json`);

  const pageCreated = writeIfMissing(pageDocPath, pageTemplate);
  const requestCreated = writeIfMissing(requestPath, requestTemplate + '\n');

  const result = {
    ok: true,
    action: 'scaffold',
    page,
    route,
    created: {
      pageDoc: pageCreated,
      request: requestCreated,
      screenshotsDir: true,
    },
    paths: {
      pageDoc: path.relative(repoRoot, pageDocPath),
      request: path.relative(repoRoot, requestPath),
      screenshotsDir: path.relative(repoRoot, screenshotsDir),
    },
  };

  process.stdout.write(JSON.stringify(result, null, 2) + '\n');
}

function validate({ page }) {
  if (!page) usage(1);
  if (!isValidSlug(page)) {
    throw new Error(`Invalid --page slug: ${page}. Expected lowercase-hyphen.`);
  }

  const repoRoot = repoRootFromSkillDir();
  const uxUiMapDir = path.join(repoRoot, 'ux_ui_map');

  const pageDocPath = path.join(uxUiMapDir, 'pages', `${page}.md`);
  const requestPath = path.join(uxUiMapDir, 'requests', `${page}.json`);
  const screenshotsDir = path.join(uxUiMapDir, 'screenshots', page);

  const expectedScreenshots = {
    desktop: path.join(screenshotsDir, 'desktop.png'),
    tablet: path.join(screenshotsDir, 'tablet.png'),
    mobile: path.join(screenshotsDir, 'mobile.png'),
  };

  const status = {
    ok: true,
    action: 'validate',
    page,
    exists: {
      pageDoc: fs.existsSync(pageDocPath),
      request: fs.existsSync(requestPath),
      screenshotsDir: fs.existsSync(screenshotsDir),
      screenshots: {
        desktop: fs.existsSync(expectedScreenshots.desktop),
        tablet: fs.existsSync(expectedScreenshots.tablet),
        mobile: fs.existsSync(expectedScreenshots.mobile),
      },
    },
  };

  status.ok =
    status.exists.pageDoc &&
    status.exists.request &&
    status.exists.screenshotsDir &&
    status.exists.screenshots.desktop &&
    status.exists.screenshots.tablet &&
    status.exists.screenshots.mobile;

  process.stdout.write(JSON.stringify(status, null, 2) + '\n');
  process.exit(status.ok ? 0 : 2);
}

const { command, args } = parseArgs(process.argv.slice(2));
try {
  if (command === 'scaffold') scaffold({ page: args.page, route: args.route });
  else if (command === 'validate') validate({ page: args.page });
  else usage(1);
} catch (err) {
  process.stderr.write(String(err?.stack || err) + '\n');
  process.exit(1);
}
