/**
 * Tests for source-mapper utility
 */

import type { SourceReference } from "@/lib/api/chat";
import { describe, expect, it } from "vitest";
import {
    buildQueryContextFromRetrieval,
    buildQueryContextFromStreamChunk,
    hasContextContent,
    mapServerMessageContextToQueryContext,
    mapSourcesToContext,
    mapSubgraphToQueryContext,
} from "../source-mapper";
import type { SubgraphBundle } from "../subgraph-types";

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
      chunk_id: "f0291a69-8b63-46d5-b44b-24095b3a8283-chunk-0",
    });
    expect(result.chunks[1]).toEqual({
      content: "Another chunk of content.",
      document_id: "bc6a87d5-6b38-4a3d-9948-b74477e2247c",
      score: 0.85,
      file_path: undefined,
      chunk_id: "bc6a87d5-6b38-4a3d-9948-b74477e2247c-chunk-1",
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

  it("should populate chunk_id for deep-linking from query citations", () => {
    const sources: SourceReference[] = [
      {
        source_type: "chunk",
        id: "abcd1234-0000-0000-0000-000000000000-chunk-0",
        score: 0.9,
        snippet: "Chunk content.",
        document_id: "abcd1234-0000-0000-0000-000000000000",
      },
    ];

    const result = mapSourcesToContext(sources);

    expect(result.chunks[0].chunk_id).toBe(
      "abcd1234-0000-0000-0000-000000000000-chunk-0",
    );
    // document_id is extracted (strips -chunk-N suffix)
    expect(result.chunks[0].document_id).toBe(
      "abcd1234-0000-0000-0000-000000000000",
    );
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

describe("buildQueryContextFromRetrieval with subgraph", () => {
  const subgraph: SubgraphBundle = {
    entities: [
      {
        id: "ent:EDGEQUAKE",
        name: "EDGEQUAKE",
        entity_type: "TECHNOLOGY",
        description: "RAG framework",
        score: 0.91,
        degree: 5,
        lineage: {
          source_document_id: "doc-1",
          source_file_path: "spec.md",
          source_chunk_ids: ["chk-1"],
        },
      },
    ],
    relationships: [
      {
        id: "rel:EDGEQUAKE:IMPLEMENTS:LIGHT_RAG",
        source: "EDGEQUAKE",
        target: "LIGHT_RAG",
        relation_type: "IMPLEMENTS",
        description: "implements LightRAG",
        score: 0.87,
        lineage: {
          source_document_id: "doc-1",
          source_file_path: "spec.md",
        },
      },
    ],
  };

  it("prefers subgraph entities over flat source parsing", () => {
    const sources: SourceReference[] = [
      {
        source_type: "chunk",
        id: "doc-1-chunk-0",
        score: 0.9,
        snippet: "chunk text",
        document_id: "doc-1",
      },
      {
        source_type: "entity",
        id: "WRONG_FLAT_NAME",
        score: 0.1,
      },
    ];

    const result = buildQueryContextFromRetrieval(sources, subgraph);

    expect(result.entities).toHaveLength(1);
    expect(result.entities[0].label).toBe("EDGEQUAKE");
    expect(result.entities[0].entity_type).toBe("TECHNOLOGY");
    expect(result.entities[0].degree).toBe(5);
    expect(result.relationships[0].type).toBe("IMPLEMENTS");
    expect(result.relationships[0].target).toBe("LIGHT_RAG");
  });

  it("mapSubgraphToQueryContext preserves relation_type without snippet parsing", () => {
    const graph = mapSubgraphToQueryContext(subgraph);
    expect(graph.relationships[0].type).toBe("IMPLEMENTS");
  });
});

describe("mapServerMessageContextToQueryContext", () => {
  it("maps persisted message context with entity types", () => {
    const result = mapServerMessageContextToQueryContext({
      sources: [
        {
          id: "doc-uuid-chunk-3",
          content: "chunk text",
          score: 0.9,
          source_type: "chunk",
          title: "report.pdf",
        },
      ],
      entities: [{ name: "ENTITY_A", entity_type: "PERSON", score: 1 }],
      relationships: [
        {
          source: "A",
          target: "B",
          relation_type: "WORKS_AT",
          score: 0.8,
        },
      ],
    });

    expect(result.chunks[0].document_id).toBe("doc-uuid");
    expect(result.entities[0].entity_type).toBe("PERSON");
    expect(result.relationships[0].type).toBe("WORKS_AT");
  });
});

describe("buildQueryContextFromStreamChunk", () => {
  it("prefers sources + subgraph over legacy nested context", () => {
    const subgraph: SubgraphBundle = {
      entities: [
        {
          id: "e1",
          name: "FROM_SUBGRAPH",
          entity_type: "ORG",
          description: "",
          score: 1,
          degree: 2,
        },
      ],
      relationships: [],
    };

    const result = buildQueryContextFromStreamChunk({
      sources: [
        {
          source_type: "chunk",
          id: "d1-chunk-0",
          score: 0.5,
          snippet: "text",
          document_id: "d1",
        },
      ],
      subgraph,
      context: {
        chunks: [],
        entities: [{ id: "legacy", label: "LEGACY", relevance: 1 }],
        relationships: [],
      },
    });

    expect(result?.entities[0].label).toBe("FROM_SUBGRAPH");
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
      hasContextContent({ chunks: [], entities: [], relationships: [] }),
    ).toBe(false);
  });

  it("should return true when chunks exist", () => {
    expect(
      hasContextContent({
        chunks: [{ content: "test", document_id: "d1", score: 0.5 }],
        entities: [],
        relationships: [],
      }),
    ).toBe(true);
  });

  it("should return true when entities exist", () => {
    expect(
      hasContextContent({
        chunks: [],
        entities: [{ id: "e1", label: "E1", relevance: 0.5 }],
        relationships: [],
      }),
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
      }),
    ).toBe(true);
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// SPEC-033: Page attribution propagation
// ─────────────────────────────────────────────────────────────────────────────

describe("SPEC-033 page_start propagation through source-mapper", () => {
  it("mapChunkSources propagates page_start/page_end from SourceReference", () => {
    const sources: SourceReference[] = [
      {
        source_type: "chunk",
        id: "doc-1-chunk-0",
        score: 0.9,
        snippet: "page 3 content",
        document_id: "doc-1",
        page_start: 3,
        page_end: 3,
      },
      {
        source_type: "chunk",
        id: "doc-1-chunk-1",
        score: 0.8,
        snippet: "no page content",
        document_id: "doc-1",
        // no page_start — non-PDF chunk
      },
    ];

    const ctx = buildQueryContextFromRetrieval(sources);
    expect(ctx.chunks[0].page_start).toBe(3);
    expect(ctx.chunks[0].page_end).toBe(3);
    expect(ctx.chunks[1].page_start).toBeUndefined();
    expect(ctx.chunks[1].page_end).toBeUndefined();
  });

  it("groupPassagesByPage: chunks with page_start group correctly", () => {
    // The grouping logic is in source-citations.tsx (not exported), so we
    // verify indirectly: chunks with page_start should produce non-null grouping.
    const sources: SourceReference[] = [
      { source_type: "chunk", id: "doc-1-chunk-0", score: 0.9, snippet: "c0", document_id: "doc-1", page_start: 1, page_end: 1 },
      { source_type: "chunk", id: "doc-1-chunk-1", score: 0.8, snippet: "c1", document_id: "doc-1", page_start: 2, page_end: 2 },
      { source_type: "chunk", id: "doc-1-chunk-2", score: 0.7, snippet: "c2", document_id: "doc-1", page_start: 2, page_end: 2 },
    ];

    const ctx = buildQueryContextFromRetrieval(sources);
    const chunks = ctx.chunks;

    // Group manually using the same logic as source-citations.tsx
    const hasPages = chunks.some(c => c.page_start !== undefined);
    expect(hasPages).toBe(true);

    const groupedByPage = new Map<number | null, typeof chunks>();
    for (const chunk of chunks) {
      const key = chunk.page_start ?? null;
      groupedByPage.set(key, [...(groupedByPage.get(key) ?? []), chunk]);
    }

    expect(groupedByPage.get(1)).toHaveLength(1);
    expect(groupedByPage.get(2)).toHaveLength(2);
    expect(groupedByPage.has(null)).toBe(false);
  });

  it("non-PDF chunks produce null grouping", () => {
    const sources: SourceReference[] = [
      { source_type: "chunk", id: "doc-2-chunk-0", score: 0.9, snippet: "c0", document_id: "doc-2" },
      { source_type: "chunk", id: "doc-2-chunk-1", score: 0.8, snippet: "c1", document_id: "doc-2" },
    ];
    const ctx = buildQueryContextFromRetrieval(sources);
    const hasPages = ctx.chunks.some(c => c.page_start !== undefined);
    expect(hasPages).toBe(false);
  });
});

