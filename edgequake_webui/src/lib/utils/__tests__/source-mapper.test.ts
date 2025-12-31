/**
 * Tests for source-mapper utility
 */

import type { SourceReference } from "@/lib/api/chat";
import { describe, expect, it } from "vitest";
import { hasContextContent, mapSourcesToContext } from "../source-mapper";

describe("mapSourcesToContext", () => {
  it("should return empty context for empty sources array", () => {
    const result = mapSourcesToContext([]);
    expect(result.chunks).toEqual([]);
    expect(result.entities).toEqual([]);
    expect(result.relationships).toEqual([]);
  });

  it("should map chunk sources correctly", () => {
    const sources: SourceReference[] = [
      {
        source_type: "chunk",
        id: "f0291a69-8b63-46d5-b44b-24095b3a8283-chunk-0",
        score: 0.95,
        snippet: "This is some sample content from the document.",
        document_id: "doc-123",
        file_path: "/uploads/test.md",
      },
      {
        source_type: "chunk",
        id: "bc6a87d5-6b38-4a3d-9948-b74477e2247c-chunk-1",
        score: 0.85,
        snippet: "Another chunk of content.",
      },
    ];

    const result = mapSourcesToContext(sources);

    expect(result.chunks).toHaveLength(2);
    expect(result.chunks[0]).toEqual({
      content: "This is some sample content from the document.",
      document_id: "f0291a69-8b63-46d5-b44b-24095b3a8283",
      score: 0.95,
      file_path: "/uploads/test.md",
    });
    expect(result.chunks[1]).toEqual({
      content: "Another chunk of content.",
      document_id: "bc6a87d5-6b38-4a3d-9948-b74477e2247c",
      score: 0.85,
      file_path: undefined,
    });
  });

  it("should map entity sources with source tracking", () => {
    const sources: SourceReference[] = [
      {
        source_type: "entity",
        id: "SARAH_CHEN",
        score: 0.92,
        snippet: "Lead researcher at the quantum computing lab.",
        document_id: "doc-456",
        file_path: "/data/research.md",
      },
    ];

    const result = mapSourcesToContext(sources);

    expect(result.entities).toHaveLength(1);
    expect(result.entities[0]).toEqual({
      id: "SARAH_CHEN",
      label: "SARAH_CHEN",
      relevance: 0.92,
      source_document_id: "doc-456",
      source_file_path: "/data/research.md",
    });
  });

  it("should map relationship sources and parse ID correctly", () => {
    const sources: SourceReference[] = [
      {
        source_type: "relationship",
        id: "SARAH_CHEN->QUANTUM_LAB",
        score: 0.88,
        snippet: "SARAH_CHEN WORKS_AT QUANTUM_LAB",
        document_id: "doc-789",
        file_path: "/data/relations.md",
      },
    ];

    const result = mapSourcesToContext(sources);

    expect(result.relationships).toHaveLength(1);
    expect(result.relationships[0]).toEqual({
      source: "SARAH_CHEN",
      target: "QUANTUM_LAB",
      type: "WORKS_AT",
      relevance: 0.88,
      source_document_id: "doc-789",
      source_file_path: "/data/relations.md",
    });
  });

  it("should handle malformed relationship IDs gracefully", () => {
    const sources: SourceReference[] = [
      {
        source_type: "relationship",
        id: "NO_ARROW_HERE",
        score: 0.5,
      },
    ];

    const result = mapSourcesToContext(sources);

    expect(result.relationships).toHaveLength(1);
    // Should use the full ID as source and empty target when no -> found
    expect(result.relationships[0].source).toBe("NO_ARROW_HERE");
  });

  it("should separate sources by type correctly", () => {
    const sources: SourceReference[] = [
      { source_type: "chunk", id: "c1", score: 0.9 },
      { source_type: "entity", id: "e1", score: 0.8 },
      { source_type: "relationship", id: "r1->r2", score: 0.7 },
      { source_type: "chunk", id: "c2", score: 0.6 },
      { source_type: "entity", id: "e2", score: 0.5 },
    ];

    const result = mapSourcesToContext(sources);

    expect(result.chunks).toHaveLength(2);
    expect(result.entities).toHaveLength(2);
    expect(result.relationships).toHaveLength(1);
  });

  it("should handle missing optional fields", () => {
    const sources: SourceReference[] = [
      {
        source_type: "entity",
        id: "MINIMAL_ENTITY",
        score: 0.75,
        // No document_id or file_path
      },
    ];

    const result = mapSourcesToContext(sources);

    expect(result.entities[0].source_document_id).toBeUndefined();
    expect(result.entities[0].source_file_path).toBeUndefined();
  });
});

describe("hasContextContent", () => {
  it("should return false for undefined context", () => {
    expect(hasContextContent(undefined)).toBe(false);
  });

  it("should return false for null context", () => {
    expect(hasContextContent(null)).toBe(false);
  });

  it("should return false for empty context", () => {
    expect(
      hasContextContent({ chunks: [], entities: [], relationships: [] })
    ).toBe(false);
  });

  it("should return true when chunks exist", () => {
    expect(
      hasContextContent({
        chunks: [{ content: "test", document_id: "d1", score: 0.5 }],
        entities: [],
        relationships: [],
      })
    ).toBe(true);
  });

  it("should return true when entities exist", () => {
    expect(
      hasContextContent({
        chunks: [],
        entities: [{ id: "e1", label: "E1", relevance: 0.5 }],
        relationships: [],
      })
    ).toBe(true);
  });

  it("should return true when relationships exist", () => {
    expect(
      hasContextContent({
        chunks: [],
        entities: [],
        relationships: [
          { source: "a", target: "b", type: "r", relevance: 0.5 },
        ],
      })
    ).toBe(true);
  });
});
