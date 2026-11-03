import { describe, expect, it } from "vitest";
import {
  buildCitationHref,
  formatMatchPercent,
  resolveClickIntent,
} from "../citation-href";

const base = { metaKey: false, ctrlKey: false, shiftKey: false, button: 0 };

describe("buildCitationHref", () => {
  it("deep-links to page + chunk and highlights the passage", () => {
    const href = buildCitationHref({
      document_id: "doc-1",
      chunk_id: "c-9",
      page_start: 3,
      content: "Hello world",
      score: 0.9,
    });
    expect(href.startsWith("/documents/doc-1?")).toBe(true);
    expect(href).toContain("chunk=c-9");
    expect(href).toContain("page=3");
    expect(href).toContain("highlight=Hello+world");
  });

  it("prefers line ranges over highlight text", () => {
    const href = buildCitationHref({
      document_id: "d",
      content: "x",
      score: 1,
      start_line: 4,
      end_line: 9,
    });
    expect(href).toContain("start_line=4");
    expect(href).toContain("end_line=9");
    expect(href).not.toContain("highlight");
  });

  it("encodes unsafe document ids", () => {
    expect(
      buildCitationHref({ document_id: "a/b", content: "", score: 0 }),
    ).toBe("/documents/a%2Fb");
  });
});

describe("resolveClickIntent", () => {
  it("navigates in the same tab on hover-capable devices", () => {
    expect(resolveClickIntent({ ...base, canHover: true })).toBe("same-tab");
  });
  it("opens a new tab for modifier or middle clicks", () => {
    expect(resolveClickIntent({ ...base, metaKey: true, canHover: true })).toBe("new-tab");
    expect(resolveClickIntent({ ...base, ctrlKey: true, canHover: true })).toBe("new-tab");
    expect(resolveClickIntent({ ...base, button: 1, canHover: true })).toBe("new-tab");
  });
  it("previews on touch devices", () => {
    expect(resolveClickIntent({ ...base, canHover: false })).toBe("preview");
  });
});

describe("formatMatchPercent", () => {
  it("clamps raw scores above 1 (was showing 297%)", () => {
    expect(formatMatchPercent(2.97)).toBe(100);
  });
  it("rounds a genuine relatedness score", () => {
    expect(formatMatchPercent(0.456)).toBe(46);
    expect(formatMatchPercent(1)).toBe(100);
  });
  it("returns null for an RRF fusion score instead of a misleading 2%", () => {
    // WHY: on a normal workspace `score` is RRF, and its only reachable values are
    // 0.01538…0.01639. Multiplying by 100 printed "2%" beside a perfect match and "1%"
    // beside an unrelated picture. A rank score is not a percentage.
    for (const rrf of [0.01639, 0.01613, 0.01587, 0.01562, 0.01538]) {
      expect(formatMatchPercent(rrf)).toBeNull();
    }
    expect(formatMatchPercent(0)).toBeNull();
    expect(formatMatchPercent(-1)).toBeNull();
    expect(formatMatchPercent(Number.NaN)).toBeNull();
  });
});
