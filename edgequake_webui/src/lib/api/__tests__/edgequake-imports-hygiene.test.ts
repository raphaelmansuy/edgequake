import { readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";

const EDGEQUAKE_DIR = join(process.cwd(), "src/lib/api/edgequake");

/** Types that appeared in the pre-split god import — none should all appear together. */
const GOD_IMPORT_MARKERS = [
  "LoginRequest",
  "CreateWorkspaceRequest",
  "PdfUploadOptions",
  "QueryStreamChunk",
  "MergeEntitiesRequest",
  "TrackStatusResponse",
  "WorkspacePdfParserBackendUpdate",
];

function extractBarrelTypes(source: string): string[] {
  const match = source.match(/import type \{([\s\S]+?)\} from "@\/types";/);
  if (!match) {
    return [];
  }
  return match[1]
    .split(",")
    .map((part) => part.trim())
    .filter(Boolean);
}

describe("edgequake domain import hygiene (UI-DRY-001)", () => {
  const modules = readdirSync(EDGEQUAKE_DIR).filter(
    (name) => name.endsWith(".ts") && name !== "default-export.ts",
  );

  it("each module imports at most 12 types from @/types barrel", () => {
    for (const file of modules) {
      const source = readFileSync(join(EDGEQUAKE_DIR, file), "utf8");
      const types = extractBarrelTypes(source);
      expect(types.length, `${file} barrel import count`).toBeLessThanOrEqual(12);
    }
  });

  it("no module retains the pre-split god import block", () => {
    for (const file of modules) {
      const source = readFileSync(join(EDGEQUAKE_DIR, file), "utf8");
      const markersPresent = GOD_IMPORT_MARKERS.filter((marker) =>
        source.includes(marker),
      );
      expect(
        markersPresent.length,
        `${file} should not contain all god-import markers`,
      ).toBeLessThan(GOD_IMPORT_MARKERS.length);
    }
  });
});
