/**
 * @module LazyMarkdownSections tests
 * @description Unit tests for the section splitting and lazy rendering logic.
 */
import type { Token, Tokens } from "marked";
import { describe, expect, it } from "vitest";
import {
  LAZY_SECTION_THRESHOLD,
  splitTokensIntoSections,
} from "../LazyMarkdownSections";

// ---------------------------------------------------------------------------
// Helpers to build minimal Token objects for testing
// ---------------------------------------------------------------------------

function heading(depth: number, text = "Heading"): Token {
  return {
    type: "heading",
    depth,
    raw: `${"#".repeat(depth)} ${text}\n`,
    text,
    tokens: [{ type: "text", raw: text, text }],
  } as unknown as Tokens.Heading;
}

function paragraph(text = "Some paragraph text."): Token {
  return {
    type: "paragraph",
    raw: `${text}\n`,
    text,
    tokens: [{ type: "text", raw: text, text }],
  } as unknown as Tokens.Paragraph;
}

function space(): Token {
  return { type: "space", raw: "\n" } as unknown as Token;
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe("splitTokensIntoSections", () => {
  it("returns empty array for empty input", () => {
    expect(splitTokensIntoSections([])).toEqual([]);
  });

  it("puts all tokens in one section when below maxPerSection", () => {
    const tokens = [heading(1), paragraph(), paragraph()];
    const sections = splitTokensIntoSections(tokens, 10);
    expect(sections).toHaveLength(1);
    expect(sections[0]).toEqual(tokens);
  });

  it("splits at h1 headings", () => {
    const tokens = [
      heading(1, "Chapter 1"),
      paragraph("Intro text"),
      heading(1, "Chapter 2"),
      paragraph("More text"),
    ];
    const sections = splitTokensIntoSections(tokens, 100);
    expect(sections).toHaveLength(2);
    expect(sections[0]).toHaveLength(2); // h1 + paragraph
    expect(sections[1]).toHaveLength(2); // h1 + paragraph
  });

  it("splits at h2 headings", () => {
    const tokens = [
      heading(2, "Section A"),
      paragraph(),
      paragraph(),
      heading(2, "Section B"),
      paragraph(),
    ];
    const sections = splitTokensIntoSections(tokens, 100);
    expect(sections).toHaveLength(2);
  });

  it("does NOT split at h3+ headings", () => {
    const tokens = [
      heading(1, "Title"),
      heading(3, "Subsection"),
      paragraph(),
      heading(4, "Sub-sub"),
      paragraph(),
    ];
    const sections = splitTokensIntoSections(tokens, 100);
    // Should be one section (h3/h4 don't trigger splits)
    expect(sections).toHaveLength(1);
  });

  it("splits when section exceeds maxPerSection", () => {
    // Create 10 paragraphs with maxPerSection=4
    const tokens = Array.from({ length: 10 }, (_, i) => paragraph(`Para ${i}`));
    const sections = splitTokensIntoSections(tokens, 4);
    // 10 / 4 → 3 sections (4 + 4 + 2)
    expect(sections).toHaveLength(3);
    expect(sections[0]).toHaveLength(4);
    expect(sections[1]).toHaveLength(4);
    expect(sections[2]).toHaveLength(2);
  });

  it("combines heading split with maxPerSection split", () => {
    const tokens = [
      heading(1, "A"),
      ...Array.from({ length: 8 }, () => paragraph()),
      heading(2, "B"),
      ...Array.from({ length: 3 }, () => paragraph()),
    ];
    // maxPerSection=5: section1=h1+4p, section2=4p, section3=h2+3p
    const sections = splitTokensIntoSections(tokens, 5);
    expect(sections).toHaveLength(3);
    expect(sections[0]).toHaveLength(5); // h1 + 4 paragraphs
    expect(sections[1]).toHaveLength(4); // 4 paragraphs (overflow split)
    expect(sections[2]).toHaveLength(4); // h2 + 3 paragraphs
  });

  it("handles leading space tokens correctly", () => {
    const tokens = [space(), heading(1), paragraph(), space()];
    const sections = splitTokensIntoSections(tokens, 100);
    // The space before h1 counts as content, so h1 triggers a split:
    // section 0 = [space], section 1 = [h1, paragraph, space]
    expect(sections).toHaveLength(2);
    expect(sections[0]).toHaveLength(1); // just the space
    expect(sections[1]).toHaveLength(3); // h1 + paragraph + space
  });

  it("preserves all tokens across splits (no data loss)", () => {
    const tokens = [
      heading(1, "A"),
      paragraph("1"),
      paragraph("2"),
      heading(2, "B"),
      paragraph("3"),
      heading(1, "C"),
      paragraph("4"),
      paragraph("5"),
      paragraph("6"),
    ];
    const sections = splitTokensIntoSections(tokens, 100);
    const reconstructed = sections.flat();
    expect(reconstructed).toHaveLength(tokens.length);
    expect(reconstructed).toEqual(tokens);
  });
});

describe("LAZY_SECTION_THRESHOLD", () => {
  it("is a positive number", () => {
    expect(LAZY_SECTION_THRESHOLD).toBeGreaterThan(0);
  });

  it("is at least 50 (avoid enabling for tiny docs)", () => {
    expect(LAZY_SECTION_THRESHOLD).toBeGreaterThanOrEqual(50);
  });
});
