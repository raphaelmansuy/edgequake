import { describe, expect, it } from "vitest";
import {
  getPendingRebuildDefaultMessage,
  getPendingRebuildMessageKey,
  hasPendingRebuild,
} from "../pending-rebuild-messages";

describe("pending-rebuild-messages (UI-P3-002)", () => {
  it("detects any pending rebuild flag", () => {
    expect(hasPendingRebuild(null)).toBe(false);
    expect(hasPendingRebuild({ embeddings: false, extraction: false })).toBe(
      false,
    );
    expect(hasPendingRebuild({ embeddings: true, extraction: false })).toBe(
      true,
    );
  });

  it("selects message key by priority", () => {
    expect(
      getPendingRebuildMessageKey({
        embeddings: true,
        extraction: true,
      }),
    ).toBe("workspace.rebuildBothPending");
    expect(
      getPendingRebuildMessageKey({
        embeddings: true,
        extraction: false,
      }),
    ).toBe("workspace.rebuildEmbeddingsPending");
    expect(
      getPendingRebuildMessageKey(
        { embeddings: false, extraction: false, vision: true },
        { includeVision: true },
      ),
    ).toBe("workspace.rebuildVisionPending");
  });

  it("returns non-empty default messages", () => {
    const key = getPendingRebuildMessageKey({
      embeddings: false,
      extraction: true,
    });
    expect(getPendingRebuildDefaultMessage(key, false).length).toBeGreaterThan(
      10,
    );
  });
});
