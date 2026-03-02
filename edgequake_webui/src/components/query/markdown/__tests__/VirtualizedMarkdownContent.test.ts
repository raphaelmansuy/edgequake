/**
 * @module VirtualizedMarkdownContent tests
 * @description Unit tests for the chunk-splitting logic used by VirtualizedMarkdownContent.
 */
import { describe, expect, it } from "vitest";
import {
  VIRTUALIZATION_CHAR_THRESHOLD,
  splitMarkdownIntoChunks,
} from "../VirtualizedMarkdownContent";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/** Generate a string of exactly `n` characters (repeating 'a'). */
function chars(n: number): string {
  return "a".repeat(n);
}

/** Build a markdown string with N sections separated by `---` page breaks. */
function pdfMarkdown(sections: string[]): string {
  return sections.join("\n---\n");
}

// ---------------------------------------------------------------------------
// VIRTUALIZATION_CHAR_THRESHOLD
// ---------------------------------------------------------------------------

describe("VIRTUALIZATION_CHAR_THRESHOLD", () => {
  it("is a positive number", () => {
    expect(VIRTUALIZATION_CHAR_THRESHOLD).toBeGreaterThan(0);
  });

  it("is at least 10_000 chars (avoid virtualizing small docs)", () => {
    expect(VIRTUALIZATION_CHAR_THRESHOLD).toBeGreaterThanOrEqual(10_000);
  });
});

// ---------------------------------------------------------------------------
// splitMarkdownIntoChunks
// ---------------------------------------------------------------------------

describe("splitMarkdownIntoChunks", () => {
  // --- Trivial / edge cases ---

  it("returns empty array for empty string", () => {
    expect(splitMarkdownIntoChunks("")).toEqual([]);
  });

  it("returns single chunk for short content", () => {
    const content = "# Hello\n\nSome paragraph.";
    const chunks = splitMarkdownIntoChunks(content);
    expect(chunks).toHaveLength(1);
    expect(chunks[0]).toBe(content);
  });

  it("returns single chunk for content exactly at TARGET_CHUNK_SIZE", () => {
    // TARGET_CHUNK_SIZE is 25_000; content at that size should be a single chunk
    const content = chars(25_000);
    const chunks = splitMarkdownIntoChunks(content);
    expect(chunks).toHaveLength(1);
    expect(chunks[0]).toBe(content);
  });

  // --- Splitting at horizontal rules (PDF page breaks) ---

  it("splits at horizontal rules (---) as first priority", () => {
    const section1 = chars(20_000);
    const section2 = chars(20_000);
    const content = pdfMarkdown([section1, section2]);
    const chunks = splitMarkdownIntoChunks(content);
    expect(chunks.length).toBeGreaterThanOrEqual(2);
    // First chunk should contain the first section
    expect(chunks[0]).toContain(section1.slice(0, 100));
  });

  it("splits at the latest --- within the target window", () => {
    // Two small sections that fit in one chunk, then a third that overflows
    const sec1 = chars(8_000);
    const sec2 = chars(8_000);
    const sec3 = chars(20_000);
    const content = pdfMarkdown([sec1, sec2, sec3]);
    const chunks = splitMarkdownIntoChunks(content);
    // sec1 + \n---\n + sec2 = ~16_005 chars → fits in one chunk
    // sec3 is 20_000 → separate chunk
    expect(chunks.length).toBeGreaterThanOrEqual(2);
  });

  // --- Splitting at headings ---

  it("splits at h1 headings when no --- is available", () => {
    const para = chars(20_000);
    const content = `${para}\n# Chapter Two\n${chars(20_000)}`;
    const chunks = splitMarkdownIntoChunks(content);
    expect(chunks.length).toBeGreaterThanOrEqual(2);
    // The second chunk should start with "# Chapter Two"
    const secondChunk = chunks[1];
    expect(secondChunk).toMatch(/^# Chapter Two/);
  });

  it("splits at h2 headings when no --- or h1 is available", () => {
    const para = chars(20_000);
    const content = `${para}\n## Section B\n${chars(20_000)}`;
    const chunks = splitMarkdownIntoChunks(content);
    expect(chunks.length).toBeGreaterThanOrEqual(2);
    const secondChunk = chunks[1];
    expect(secondChunk).toMatch(/^## Section B/);
  });

  // --- Splitting at paragraph boundaries ---

  it("splits at double-newline paragraph boundaries as fallback", () => {
    // Create content with no headings or --- but with paragraph breaks
    const lines = [];
    for (let i = 0; i < 500; i++) {
      lines.push(`Paragraph ${i}: ${chars(50)}`);
    }
    const content = lines.join("\n\n");
    const chunks = splitMarkdownIntoChunks(content);
    expect(chunks.length).toBeGreaterThanOrEqual(2);
    // Each chunk should end at a paragraph boundary (trailing \n\n)
    for (let i = 0; i < chunks.length - 1; i++) {
      expect(chunks[i].endsWith("\n\n")).toBe(true);
    }
  });

  // --- Hard split fallback ---

  it("hard splits when no natural boundary is found", () => {
    // One enormous line with no newlines
    const content = chars(60_000);
    const chunks = splitMarkdownIntoChunks(content);
    expect(chunks.length).toBeGreaterThanOrEqual(2);
    // Reconstructed should equal original
    expect(chunks.join("")).toBe(content);
  });

  // --- Data integrity ---

  it("preserves all content across chunks (no data loss)", () => {
    const sections = Array.from(
      { length: 10 },
      (_, i) => `# Section ${i}\n\n${chars(8_000)}`,
    );
    const content = pdfMarkdown(sections);
    const chunks = splitMarkdownIntoChunks(content);
    const reconstructed = chunks.join("");
    expect(reconstructed).toBe(content);
  });

  it("preserves content for very large documents (200KB+)", () => {
    const content = chars(200_000);
    const chunks = splitMarkdownIntoChunks(content);
    expect(chunks.length).toBeGreaterThanOrEqual(8); // 200K / 25K ≈ 8
    expect(chunks.join("")).toBe(content);
  });

  // --- Chunk size constraints ---

  it("produces chunks no larger than ~2x TARGET_CHUNK_SIZE", () => {
    // Even in worst case, chunks should not be excessively large
    const sections = Array.from(
      { length: 15 },
      (_, i) => `# Part ${i}\n\n${chars(10_000)}`,
    );
    const content = sections.join("\n---\n");
    const chunks = splitMarkdownIntoChunks(content);
    for (const chunk of chunks) {
      // Allow 2x TARGET_CHUNK_SIZE as generous upper bound
      expect(chunk.length).toBeLessThanOrEqual(50_000);
    }
  });

  // --- Realistic PDF markdown ---

  it("handles realistic PDF extraction markdown with --- separators", () => {
    // Simulate typical PDF extraction output
    const pdfPages = Array.from({ length: 50 }, (_, i) => {
      const heading = `## Page ${i + 1}\n\n`;
      const body = `This is the content of page ${i + 1}. `.repeat(80);
      return heading + body;
    });
    const content = pdfPages.join("\n---\n");
    const chunks = splitMarkdownIntoChunks(content);
    // Should have some chunks (not all 50 because some PDF pages are small
    // enough to group together)
    expect(chunks.length).toBeGreaterThanOrEqual(2);
    expect(chunks.length).toBeLessThanOrEqual(50);
    // No data loss
    expect(chunks.join("")).toBe(content);
  });

  it("handles content with mixed boundary types correctly", () => {
    const blocks = [
      chars(5_000),
      "\n---\n",
      chars(5_000),
      "\n# Big Heading\n",
      chars(5_000),
      "\n## Sub Heading\n",
      chars(5_000),
      "\n\n",
      chars(10_000),
    ];
    const content = blocks.join("");
    const chunks = splitMarkdownIntoChunks(content);
    expect(chunks.join("")).toBe(content);
  });
});
