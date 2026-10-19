import { describe, expect, it } from "vitest";
import { CHORD_TIMEOUT_MS, createChordTracker } from "../chords";
import { DESTINATIONS, destinationForChord, isActiveDestination } from "../destinations";

describe("chord tracker", () => {
  it("ignores keys when not armed", () => {
    const t = createChordTracker();
    expect(t.press("d", 0)).toEqual({ kind: "none" });
  });

  it("arms on the leader and completes on the next key", () => {
    const t = createChordTracker();
    expect(t.press("g", 0)).toEqual({ kind: "armed" });
    expect(t.armed).toBe(true);
    expect(t.press("d", 100)).toEqual({ kind: "complete", key: "d" });
    expect(t.armed).toBe(false);
  });

  it("supports g g (second leader completes)", () => {
    const t = createChordTracker();
    t.press("g", 0);
    expect(t.press("G", 50)).toEqual({ kind: "complete", key: "g" });
  });

  it("expires after the timeout and re-arms on a late leader", () => {
    const t = createChordTracker();
    t.press("g", 0);
    expect(t.press("g", CHORD_TIMEOUT_MS + 1)).toEqual({ kind: "armed" });
    expect(t.press("d", CHORD_TIMEOUT_MS + 2)).toEqual({ kind: "complete", key: "d" });
  });

  it("a stale non-leader key is ignored", () => {
    const t = createChordTracker();
    t.press("g", 0);
    expect(t.press("d", CHORD_TIMEOUT_MS + 5)).toEqual({ kind: "none" });
  });

  it("reset drops the armed leader", () => {
    const t = createChordTracker();
    t.press("g", 0);
    t.reset();
    expect(t.press("d", 10)).toEqual({ kind: "none" });
  });
});

describe("destinations registry", () => {
  it("has unique chords and hrefs", () => {
    const chords = DESTINATIONS.map((d) => d.chord);
    const hrefs = DESTINATIONS.map((d) => d.href);
    expect(new Set(chords).size).toBe(chords.length);
    expect(new Set(hrefs).size).toBe(hrefs.length);
  });

  it("resolves chords case-insensitively", () => {
    expect(destinationForChord("D")?.href).toBe("/documents");
    expect(destinationForChord("g")?.href).toBe("/graph");
    expect(destinationForChord("z")).toBeUndefined();
  });

  it("active-route rule does not let / match everything", () => {
    expect(isActiveDestination("/", "/")).toBe(true);
    expect(isActiveDestination("/documents/abc", "/")).toBe(false);
    expect(isActiveDestination("/documents/abc", "/documents")).toBe(true);
    expect(isActiveDestination("/documentsx", "/documents")).toBe(false);
  });
});
