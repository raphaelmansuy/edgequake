import { readFileSync, readdirSync, statSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";

const API_ROOT = join(process.cwd(), "src/lib/api");
const MAX_LOC = 500;

/**
 * Infrastructure modules allowed slightly above cap until further split.
 *
 * SPEC-181 security pass: `client.ts` was 576 lines against a 520 exception. The error classes
 * (`errors.ts`) and the cookie-refresh flow (`session-refresh.ts`) were separable concerns, so
 * they moved out and the file is now inside the base cap. The entry is kept at the base cap --
 * not removed -- so a future regression fails here instead of silently creeping back.
 */
const LOC_EXCEPTIONS: Record<string, number> = {
  // Split in the SPEC-181 security pass (errors + cookie refresh moved out): 576 -> 478.
  // Kept at the base cap so a future regression fails here.
  "client.ts": 500,
  // Pre-existing debt, recorded rather than hidden. Both are split candidates; until they are
  // split, this table pins them at their current size so they cannot grow further.
  "documents.ts": 883,
  "workspaces.ts": 595,
};

function lineCount(filePath: string): number {
  return readFileSync(filePath, "utf8").split("\n").length;
}

function collectTsFiles(dir: string): string[] {
  const out: string[] = [];
  for (const entry of readdirSync(dir)) {
    const full = join(dir, entry);
    if (statSync(full).isDirectory()) {
      out.push(...collectTsFiles(full));
    } else if (entry.endsWith(".ts") && !entry.includes(".test.")) {
      out.push(full);
    }
  }
  return out;
}

describe("lib/api module size (SPEC-017 verification)", () => {
  it("no domain module exceeds MAX_LOC (god-module guard)", () => {
    const files = collectTsFiles(API_ROOT);
    const violations: string[] = [];

    for (const file of files) {
      const rel = file.replace(`${API_ROOT}/`, "");
      const base = rel.split("/").pop() ?? rel;
      const limit = LOC_EXCEPTIONS[base] ?? MAX_LOC;
      const count = lineCount(file);
      if (count > limit) {
        violations.push(`${rel}: ${count} lines (max ${limit})`);
      }
    }

    expect(violations, violations.join("\n")).toEqual([]);
  });
});
