import { readFileSync, readdirSync, statSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";

const API_ROOT = join(process.cwd(), "src/lib/api");
const MAX_LOC = 500;

/** Infrastructure modules allowed slightly above cap until further split. */
const LOC_EXCEPTIONS: Record<string, number> = {
  "client.ts": 520,
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
