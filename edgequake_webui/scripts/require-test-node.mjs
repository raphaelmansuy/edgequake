#!/usr/bin/env node
/**
 * Refuse to run the WebUI test suites on a Node the test environment cannot load.
 *
 * WHY this exists: `jsdom` depends on `html-encoding-sniffer@7`, whose `engines` field is
 * `^22.13.0 || >=24.0.0`. Below that floor the package is CommonJS but its own dependency
 * (`@exodus/bytes`) is ESM-only, and only Node >= 22.13 / >= 24 can `require()` an ES module.
 * On Node 20 the loader throws `ERR_REQUIRE_ESM` while vitest is constructing the worker, so
 * every `.dom.test.ts(x)` file dies *before* its tests run: the run reports the remaining files
 * as passed and still exits non-zero with 24 opaque "unhandled errors". That is the worst
 * possible failure shape — it looks like a flake and hides which tests never ran.
 *
 * Failing here instead names the real problem and the command that fixes it. Keep the floor in
 * sync with `engines.node` in package.json and `.nvmrc`.
 */
const FLOOR = [22, 13, 0];

function parts(version) {
  return version
    .replace(/^v/, '')
    .split('.')
    .map((n) => Number.parseInt(n, 10) || 0);
}

/** `^22.13.0 || >=24.0.0` expressed as a predicate, matching package.json `engines.node`. */
export function isSupportedNode(version) {
  const [major, minor, patch] = parts(version);
  if (major >= 24) return true;
  if (major !== 22) return false;
  if (minor !== FLOOR[1]) return minor > FLOOR[1];
  return patch >= FLOOR[2];
}

// Only act when executed as a script, so the predicate stays unit-testable.
const invokedDirectly =
  process.argv[1] && import.meta.url === new URL(`file://${process.argv[1]}`).href;

const current = process.versions.node;

if (invokedDirectly && !isSupportedNode(current)) {
  const wanted = `${FLOOR.join('.')}`;
  process.stderr.write(
    [
      '',
      `  Node ${current} cannot run the WebUI test suites.`,
      '',
      `  The DOM tests (jsdom) need Node >= ${wanted} or >= 24.0.0. On older Node,`,
      '  every .dom.test.ts(x) file fails to load and silently reports nothing.',
      '',
      '  Fix one of:',
      '    nvm install 22.13.0 && nvm use 22.13.0',
      '    nvm use 24',
      '',
      '  Then re-run the suite.',
      '',
    ].join('\n'),
  );
  process.exit(1);
}
