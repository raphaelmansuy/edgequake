import { describe, expect, it } from "bun:test";

import {
  getQueryEmptyCopy,
  isChatQueryMode,
} from "@/lib/query/query-empty-copy";

describe("query-empty-copy Chat mode", () => {
  it("uses chatbot copy for bypass and not KG framing", () => {
    expect(isChatQueryMode("bypass")).toBe(true);
    const copy = getQueryEmptyCopy("bypass");
    expect(copy.title).toBe("Chat with your assistant");
    expect(copy.description.toLowerCase()).toContain("without document");
    expect(copy.suggestions.length).toBe(4);
    expect(copy.title.toLowerCase()).not.toContain("knowledge graph");
  });

  it("keeps KG copy for RAG modes", () => {
    expect(isChatQueryMode("mix")).toBe(false);
    const copy = getQueryEmptyCopy("mix");
    expect(copy.title).toContain("knowledge graph");
  });

  it("frames a workspace without a graph around documents and passages", () => {
    const copy = getQueryEmptyCopy("naive", false);
    expect(copy.title).toBe("Ask about your documents");
    expect(`${copy.title} ${copy.description} ${copy.subtitle} ${copy.suggestions.join(" ")}`.toLowerCase())
      .not.toMatch(/graph|entit|relationship/);
    expect(copy.suggestions.length).toBe(4);
  });

  it("keeps Chat copy on a workspace without a graph", () => {
    expect(getQueryEmptyCopy("bypass", false).title).toBe("Chat with your assistant");
  });

  it("every copy names its own i18n keys", () => {
    const keys = [getQueryEmptyCopy("mix"), getQueryEmptyCopy("naive", false), getQueryEmptyCopy("bypass")]
      .map((c) => c.keys.title);
    expect(new Set(keys).size).toBe(3);
  });
});
