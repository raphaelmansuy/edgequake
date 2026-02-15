/**
 * Lineage & metadata tests — verifies lineage, provenance, chunk lineage,
 * document full-lineage, and metadata field coverage.
 *
 * WHY: OODA-16 — Ensure TypeScript SDK covers all lineage/metadata fields
 * from the API surface (EntityLineageResponse, DocumentGraphLineageResponse,
 * ChunkLineageResponse, DocumentFullLineageResponse, EntityProvenanceResponse).
 *
 * @module tests/unit/lineage
 */

import { beforeEach, describe, expect, it } from "vitest";
import type { HttpTransport } from "../../src/transport/types.js";
import { createMockTransport } from "../helpers/mock-transport.js";

// Resource imports
import { ChunksResource } from "../../src/resources/chunks.js";
import { DocumentsResource } from "../../src/resources/documents.js";
import { GraphResource } from "../../src/resources/graph.js";
import { LineageResource } from "../../src/resources/lineage.js";
import { ProvenanceResource } from "../../src/resources/provenance.js";

// Type imports for type-level checks
import type {
  CreateEntityRequest,
  CreateRelationshipRequest,
  GraphEdge,
  GraphNode,
  MergeEntitiesRequest,
} from "../../src/types/graph.js";
import type {
  ChunkDetailResponse,
  ChunkLineageResponse,
  ChunkSourceInfo,
  DescriptionVersionResponse,
  DocumentFullLineageResponse,
  DocumentGraphLineageResponse,
  EntityLineageResponse,
  EntityProvenanceResponse,
  EntitySourceInfo,
  ExtractionMetadataInfo,
  ExtractionStatsResponse,
  LineRangeInfo,
  RelatedEntityInfo,
  SourceDocumentInfo,
} from "../../src/types/lineage.js";

// ─────────────────────── Entity Lineage (rich data) ───────────────────────

describe("LineageResource — entity lineage with full data", () => {
  let mock: ReturnType<typeof createMockTransport>;
  let lineage: LineageResource;

  const entityLineageBody: EntityLineageResponse = {
    entity_name: "ALICE",
    entity_type: "PERSON",
    source_documents: [
      {
        document_id: "doc-1",
        chunk_ids: ["chunk-a", "chunk-b"],
        line_ranges: [
          { start_line: 10, end_line: 15 },
          { start_line: 42, end_line: 50 },
        ],
      },
      {
        document_id: "doc-2",
        chunk_ids: ["chunk-c"],
        line_ranges: [{ start_line: 1, end_line: 5 }],
      },
    ],
    source_count: 2,
    description_versions: [
      {
        version: 1,
        description: "A researcher",
        source_chunk_id: "chunk-a",
        created_at: "2026-01-15T10:00:00Z",
      },
      {
        version: 2,
        description: "A senior researcher at MIT",
        source_chunk_id: "chunk-c",
        created_at: "2026-01-16T12:00:00Z",
      },
    ],
  };

  beforeEach(() => {
    mock = createMockTransport({
      "GET /api/v1/lineage/entities/ALICE": { body: entityLineageBody },
    });
    lineage = new LineageResource(mock as unknown as HttpTransport);
  });

  it("returns entity_name and entity_type", async () => {
    const res = await lineage.entity("ALICE");
    expect(res.entity_name).toBe("ALICE");
    expect(res.entity_type).toBe("PERSON");
  });

  it("returns source_documents with chunk_ids and line_ranges", async () => {
    const res = await lineage.entity("ALICE");
    expect(res.source_documents).toHaveLength(2);
    expect(res.source_documents[0].document_id).toBe("doc-1");
    expect(res.source_documents[0].chunk_ids).toEqual(["chunk-a", "chunk-b"]);
    expect(res.source_documents[0].line_ranges).toHaveLength(2);
    expect(res.source_documents[0].line_ranges[0].start_line).toBe(10);
    expect(res.source_documents[0].line_ranges[0].end_line).toBe(15);
  });

  it("returns source_count", async () => {
    const res = await lineage.entity("ALICE");
    expect(res.source_count).toBe(2);
  });

  it("returns description_versions with version history", async () => {
    const res = await lineage.entity("ALICE");
    expect(res.description_versions).toHaveLength(2);
    expect(res.description_versions[0].version).toBe(1);
    expect(res.description_versions[0].description).toBe("A researcher");
    expect(res.description_versions[0].source_chunk_id).toBe("chunk-a");
    expect(res.description_versions[1].version).toBe(2);
    expect(res.description_versions[1].created_at).toBe("2026-01-16T12:00:00Z");
  });
});

// ─────────────────────── Document Graph Lineage ───────────────────────

describe("LineageResource — document graph lineage with extraction stats", () => {
  let mock: ReturnType<typeof createMockTransport>;
  let lineage: LineageResource;

  const docLineageBody: DocumentGraphLineageResponse = {
    document_id: "doc-1",
    chunk_count: 5,
    entities: [
      {
        name: "ALICE",
        entity_type: "PERSON",
        source_chunks: ["c1", "c3"],
        is_shared: true,
      },
      {
        name: "MIT",
        entity_type: "ORGANIZATION",
        source_chunks: ["c2"],
        is_shared: false,
      },
    ],
    relationships: [
      {
        source: "ALICE",
        target: "MIT",
        keywords: "WORKS_AT",
        source_chunks: ["c2"],
      },
    ],
    extraction_stats: {
      total_entities: 10,
      unique_entities: 5,
      total_relationships: 8,
      unique_relationships: 4,
      processing_time_ms: 1500,
    },
  };

  beforeEach(() => {
    mock = createMockTransport({
      "GET /api/v1/lineage/documents/doc-1": { body: docLineageBody },
    });
    lineage = new LineageResource(mock as unknown as HttpTransport);
  });

  it("returns entities with source_chunks and is_shared flag", async () => {
    const res = await lineage.document("doc-1");
    expect(res.entities).toHaveLength(2);
    expect(res.entities[0].name).toBe("ALICE");
    expect(res.entities[0].is_shared).toBe(true);
    expect(res.entities[0].source_chunks).toEqual(["c1", "c3"]);
  });

  it("returns relationships with source_chunks", async () => {
    const res = await lineage.document("doc-1");
    expect(res.relationships).toHaveLength(1);
    expect(res.relationships[0].source).toBe("ALICE");
    expect(res.relationships[0].target).toBe("MIT");
    expect(res.relationships[0].keywords).toBe("WORKS_AT");
  });

  it("returns extraction_stats with processing_time_ms", async () => {
    const res = await lineage.document("doc-1");
    const stats = res.extraction_stats;
    expect(stats.total_entities).toBe(10);
    expect(stats.unique_entities).toBe(5);
    expect(stats.total_relationships).toBe(8);
    expect(stats.unique_relationships).toBe(4);
    expect(stats.processing_time_ms).toBe(1500);
  });
});

// ─────────────────────── Document Full Lineage ───────────────────────

describe("DocumentsResource.getLineage — full document lineage", () => {
  let mock: ReturnType<typeof createMockTransport>;
  let docs: DocumentsResource;

  const fullLineageBody: DocumentFullLineageResponse = {
    document_id: "doc-1",
    metadata: {
      title: "Research Paper",
      author: "Dr. Smith",
      tags: ["AI", "NLP"],
    },
    lineage: {
      entities_extracted: 15,
      relationships_extracted: 8,
      pipeline_version: "1.2.0",
    },
  };

  beforeEach(() => {
    mock = createMockTransport({
      "GET /api/v1/documents/doc-1/lineage": { body: fullLineageBody },
      // Required by DocumentsResource constructor
      "GET /api/v1/documents": { body: { documents: [], total: 0 } },
    });
    docs = new DocumentsResource(mock as unknown as HttpTransport);
  });

  it("getLineage → GET /api/v1/documents/:id/lineage", async () => {
    const res = await docs.getLineage("doc-1");
    expect(mock.lastRequest?.path).toBe("/api/v1/documents/doc-1/lineage");
    expect(mock.lastRequest?.method).toBe("GET");
  });

  it("returns document_id and metadata", async () => {
    const res = await docs.getLineage("doc-1");
    expect(res.document_id).toBe("doc-1");
    expect(res.metadata).toBeDefined();
    expect(res.metadata?.title).toBe("Research Paper");
    expect(res.metadata?.author).toBe("Dr. Smith");
  });

  it("returns lineage data with pipeline info", async () => {
    const res = await docs.getLineage("doc-1");
    expect(res.lineage).toBeDefined();
    expect(res.lineage?.entities_extracted).toBe(15);
    expect(res.lineage?.pipeline_version).toBe("1.2.0");
  });
});

// ─────────────────────── Document Metadata ───────────────────────

describe("DocumentsResource.getMetadata — document metadata", () => {
  let mock: ReturnType<typeof createMockTransport>;
  let docs: DocumentsResource;

  beforeEach(() => {
    mock = createMockTransport({
      "GET /api/v1/documents/doc-1/metadata": {
        body: {
          author: "Jane Doe",
          category: "research",
          tags: ["graph", "knowledge"],
          created_date: "2026-01-15",
        },
      },
      "GET /api/v1/documents": { body: { documents: [], total: 0 } },
    });
    docs = new DocumentsResource(mock as unknown as HttpTransport);
  });

  it("getMetadata → GET /api/v1/documents/:id/metadata", async () => {
    const meta = await docs.getMetadata("doc-1");
    expect(mock.lastRequest?.path).toBe("/api/v1/documents/doc-1/metadata");
    expect(meta.author).toBe("Jane Doe");
    expect(meta.category).toBe("research");
    expect(meta.tags).toEqual(["graph", "knowledge"]);
  });
});

// ─────────────────────── Document Lineage Export ───────────────────────

describe("DocumentsResource.exportLineage — lineage export (OODA-07)", () => {
  let mock: ReturnType<typeof createMockTransport>;
  let docs: DocumentsResource;

  const jsonBlob = new Blob(['{"entities":[],"relationships":[]}'], {
    type: "application/json",
  });
  const csvBlob = new Blob(["entity,type,source\nALICE,PERSON,doc-1"], {
    type: "text/csv",
  });

  beforeEach(() => {
    mock = createMockTransport({
      "GET /api/v1/documents/doc-1/lineage/export": { blob: jsonBlob },
      "GET /api/v1/documents/doc-1/lineage/export?format=json": {
        blob: jsonBlob,
      },
      "GET /api/v1/documents/doc-1/lineage/export?format=csv": {
        blob: csvBlob,
      },
      "GET /api/v1/documents": { body: { documents: [], total: 0 } },
    });
    docs = new DocumentsResource(mock as unknown as HttpTransport);
  });

  it("exportLineage (default) → GET /api/v1/documents/:id/lineage/export", async () => {
    const blob = await docs.exportLineage("doc-1");
    expect(mock.lastRequest?.path).toBe(
      "/api/v1/documents/doc-1/lineage/export",
    );
    expect(blob).toBeInstanceOf(Blob);
  });

  it("exportLineage (json) → includes format=json query param", async () => {
    const blob = await docs.exportLineage("doc-1", { format: "json" });
    expect(mock.lastRequest?.path).toBe(
      "/api/v1/documents/doc-1/lineage/export?format=json",
    );
    expect(blob).toBeInstanceOf(Blob);
  });

  it("exportLineage (csv) → includes format=csv query param", async () => {
    const blob = await docs.exportLineage("doc-1", { format: "csv" });
    expect(mock.lastRequest?.path).toBe(
      "/api/v1/documents/doc-1/lineage/export?format=csv",
    );
    expect(blob).toBeInstanceOf(Blob);
  });

  it("exportLineage returns blob that can be converted to text", async () => {
    const blob = await docs.exportLineage("doc-1", { format: "csv" });
    const text = await blob.text();
    expect(text).toContain("entity,type,source");
    expect(text).toContain("ALICE");
  });
});

// ─────────────────────── Chunk Lineage ───────────────────────

describe("ChunksResource.getLineage — chunk lineage", () => {
  let mock: ReturnType<typeof createMockTransport>;
  let chunks: ChunksResource;

  const chunkLineageBody: ChunkLineageResponse = {
    chunk_id: "chunk-1",
    document_id: "doc-1",
    document_name: "Research Paper.pdf",
    document_type: "pdf",
    index: 3,
    start_line: 42,
    end_line: 60,
    start_offset: 1200,
    end_offset: 1800,
    token_count: 150,
    content_preview: "Alice works at MIT...",
    entity_count: 3,
    relationship_count: 2,
    entity_names: ["ALICE", "MIT", "BOB"],
    document_metadata: { author: "Dr. Smith" },
  };

  beforeEach(() => {
    mock = createMockTransport({
      "GET /api/v1/chunks/chunk-1/lineage": { body: chunkLineageBody },
      "GET /api/v1/chunks/chunk-1": {
        body: {
          chunk_id: "chunk-1",
          document_id: "doc-1",
          content: "text",
          index: 0,
          char_range: { start: 0, end: 4 },
          token_count: 1,
          entities: [],
          relationships: [],
        },
      },
    });
    chunks = new ChunksResource(mock as unknown as HttpTransport);
  });

  it("getLineage → GET /api/v1/chunks/:id/lineage", async () => {
    const res = await chunks.getLineage("chunk-1");
    expect(mock.lastRequest?.path).toBe("/api/v1/chunks/chunk-1/lineage");
    expect(mock.lastRequest?.method).toBe("GET");
  });

  it("returns chunk position info (start_line, end_line, offsets)", async () => {
    const res = await chunks.getLineage("chunk-1");
    expect(res.chunk_id).toBe("chunk-1");
    expect(res.start_line).toBe(42);
    expect(res.end_line).toBe(60);
    expect(res.start_offset).toBe(1200);
    expect(res.end_offset).toBe(1800);
    expect(res.index).toBe(3);
  });

  it("returns parent document info", async () => {
    const res = await chunks.getLineage("chunk-1");
    expect(res.document_id).toBe("doc-1");
    expect(res.document_name).toBe("Research Paper.pdf");
    expect(res.document_type).toBe("pdf");
  });

  it("returns entity_names and counts", async () => {
    const res = await chunks.getLineage("chunk-1");
    expect(res.entity_count).toBe(3);
    expect(res.relationship_count).toBe(2);
    expect(res.entity_names).toEqual(["ALICE", "MIT", "BOB"]);
  });

  it("returns document_metadata from KV storage", async () => {
    const res = await chunks.getLineage("chunk-1");
    expect(res.document_metadata).toBeDefined();
    expect(res.document_metadata?.author).toBe("Dr. Smith");
  });

  it("returns content_preview and token_count", async () => {
    const res = await chunks.getLineage("chunk-1");
    expect(res.content_preview).toBe("Alice works at MIT...");
    expect(res.token_count).toBe(150);
  });
});

// ─────────────────────── Chunk Detail with Extraction Metadata ───────────────

describe("ChunksResource — chunk detail with extraction metadata", () => {
  let mock: ReturnType<typeof createMockTransport>;
  let chunks: ChunksResource;

  const detailBody: ChunkDetailResponse = {
    chunk_id: "c1",
    document_id: "d1",
    document_name: "Paper.pdf",
    content: "Alice and Bob collaborate on AI research at MIT.",
    index: 0,
    char_range: { start: 0, end: 47 },
    token_count: 10,
    entities: [
      {
        id: "e1",
        name: "ALICE",
        entity_type: "PERSON",
        description: "A researcher",
      },
      { id: "e2", name: "BOB", entity_type: "PERSON" },
      { id: "e3", name: "MIT", entity_type: "ORGANIZATION" },
    ],
    relationships: [
      {
        source_name: "ALICE",
        target_name: "BOB",
        relation_type: "COLLABORATES_WITH",
        description: "Research collaboration",
      },
      {
        source_name: "ALICE",
        target_name: "MIT",
        relation_type: "WORKS_AT",
      },
    ],
    extraction_metadata: {
      model: "gpt-4o",
      gleaning_iterations: 2,
      duration_ms: 1200,
      input_tokens: 500,
      output_tokens: 300,
      cached: false,
    },
  };

  beforeEach(() => {
    mock = createMockTransport({
      "GET /api/v1/chunks/c1": { body: detailBody },
    });
    chunks = new ChunksResource(mock as unknown as HttpTransport);
  });

  it("returns entities array with type and description", async () => {
    const c = await chunks.get("c1");
    expect(c.entities).toHaveLength(3);
    expect(c.entities[0].name).toBe("ALICE");
    expect(c.entities[0].entity_type).toBe("PERSON");
    expect(c.entities[0].description).toBe("A researcher");
    expect(c.entities[2].entity_type).toBe("ORGANIZATION");
  });

  it("returns relationships with source_name/target_name", async () => {
    const c = await chunks.get("c1");
    expect(c.relationships).toHaveLength(2);
    expect(c.relationships[0].source_name).toBe("ALICE");
    expect(c.relationships[0].target_name).toBe("BOB");
    expect(c.relationships[0].relation_type).toBe("COLLABORATES_WITH");
  });

  it("returns extraction_metadata with model and timings", async () => {
    const c = await chunks.get("c1");
    expect(c.extraction_metadata).toBeDefined();
    expect(c.extraction_metadata!.model).toBe("gpt-4o");
    expect(c.extraction_metadata!.gleaning_iterations).toBe(2);
    expect(c.extraction_metadata!.duration_ms).toBe(1200);
    expect(c.extraction_metadata!.input_tokens).toBe(500);
    expect(c.extraction_metadata!.output_tokens).toBe(300);
    expect(c.extraction_metadata!.cached).toBe(false);
  });

  it("returns char_range for chunk position", async () => {
    const c = await chunks.get("c1");
    expect(c.char_range.start).toBe(0);
    expect(c.char_range.end).toBe(47);
  });
});

// ─────────────────────── Provenance (rich data) ───────────────────────

describe("ProvenanceResource — entity provenance with full sources", () => {
  let mock: ReturnType<typeof createMockTransport>;
  let prov: ProvenanceResource;

  const provBody: EntityProvenanceResponse = {
    entity_id: "e1",
    entity_name: "ALICE",
    entity_type: "PERSON",
    description: "A senior researcher at MIT",
    sources: [
      {
        document_id: "doc-1",
        document_name: "Paper.pdf",
        chunks: [
          {
            chunk_id: "c1",
            start_line: 10,
            end_line: 15,
            source_text: "Alice is a researcher...",
          },
          {
            chunk_id: "c2",
            start_line: 42,
            end_line: 50,
          },
        ],
        first_extracted_at: "2026-01-15T10:00:00Z",
      },
    ],
    total_extraction_count: 5,
    related_entities: [
      {
        entity_id: "e2",
        entity_name: "BOB",
        relationship_type: "COLLABORATES_WITH",
        shared_documents: 2,
      },
      {
        entity_id: "e3",
        entity_name: "MIT",
        relationship_type: "WORKS_AT",
        shared_documents: 1,
      },
    ],
  };

  beforeEach(() => {
    mock = createMockTransport({
      "GET /api/v1/entities/e1/provenance": { body: provBody },
    });
    prov = new ProvenanceResource(mock as unknown as HttpTransport);
  });

  it("returns entity provenance with entity_name and entity_type", async () => {
    const res = await prov.get("e1");
    expect(res.entity_name).toBe("ALICE");
    expect(res.entity_type).toBe("PERSON");
    expect(res.description).toBe("A senior researcher at MIT");
  });

  it("returns sources with document and chunk info", async () => {
    const res = await prov.get("e1");
    expect(res.sources).toHaveLength(1);
    expect(res.sources[0].document_id).toBe("doc-1");
    expect(res.sources[0].document_name).toBe("Paper.pdf");
    expect(res.sources[0].chunks).toHaveLength(2);
    expect(res.sources[0].chunks[0].chunk_id).toBe("c1");
    expect(res.sources[0].chunks[0].start_line).toBe(10);
    expect(res.sources[0].chunks[0].source_text).toBe(
      "Alice is a researcher...",
    );
    expect(res.sources[0].first_extracted_at).toBe("2026-01-15T10:00:00Z");
  });

  it("returns total_extraction_count", async () => {
    const res = await prov.get("e1");
    expect(res.total_extraction_count).toBe(5);
  });

  it("returns related_entities with shared_documents count", async () => {
    const res = await prov.get("e1");
    expect(res.related_entities).toHaveLength(2);
    expect(res.related_entities[0].entity_name).toBe("BOB");
    expect(res.related_entities[0].relationship_type).toBe("COLLABORATES_WITH");
    expect(res.related_entities[0].shared_documents).toBe(2);
    expect(res.related_entities[1].entity_name).toBe("MIT");
  });
});

// ─────────────────────── Entity Create with Metadata ───────────────────────

describe("GraphResource.entities.create — metadata in request body", () => {
  let mock: ReturnType<typeof createMockTransport>;
  let graph: GraphResource;

  beforeEach(() => {
    mock = createMockTransport({
      "POST /api/v1/graph/entities": {
        body: {
          name: "DATA_SCIENCE",
          entity_type: "TECHNOLOGY",
          source_id: "doc-research",
          metadata: { confidence: 0.95, extraction_model: "gpt-4o" },
        },
      },
      // Required routes for GraphResource
      "GET /api/v1/graph": { body: { nodes: [], edges: [] } },
    });
    graph = new GraphResource(mock as unknown as HttpTransport);
  });

  it("sends source_id in create request body", async () => {
    await graph.entities.create({
      entity_name: "DATA_SCIENCE",
      entity_type: "TECHNOLOGY",
      description: "A field of study",
      source_id: "doc-research",
    });
    expect(mock.lastRequest?.body).toBeDefined();
    const body = mock.lastRequest!.body as CreateEntityRequest;
    expect(body.source_id).toBe("doc-research");
  });

  it("sends metadata in create request body", async () => {
    await graph.entities.create({
      entity_name: "DATA_SCIENCE",
      entity_type: "TECHNOLOGY",
      description: "A field of study",
      source_id: "manual_entry",
      metadata: { confidence: 0.95, extraction_model: "gpt-4o" },
    });
    expect(mock.lastRequest?.body).toBeDefined();
    const body = mock.lastRequest!.body as CreateEntityRequest;
    expect(body.metadata).toBeDefined();
    expect(body.metadata?.confidence).toBe(0.95);
    expect(body.metadata?.extraction_model).toBe("gpt-4o");
  });
});

// ─────────────────────── Relationship Create with Metadata ──────────────────

describe("GraphResource.relationships.create — metadata fields", () => {
  let mock: ReturnType<typeof createMockTransport>;
  let graph: GraphResource;

  beforeEach(() => {
    mock = createMockTransport({
      "POST /api/v1/graph/relationships": {
        body: { id: "r1", source: "ALICE", target: "BOB" },
      },
      "GET /api/v1/graph": { body: { nodes: [], edges: [] } },
    });
    graph = new GraphResource(mock as unknown as HttpTransport);
  });

  it("sends weight and description in create request", async () => {
    await graph.relationships.create({
      source: "ALICE",
      target: "BOB",
      relationship_type: "COLLABORATES_WITH",
      weight: 0.8,
      description: "Research collaboration",
    });
    const body = mock.lastRequest!.body as CreateRelationshipRequest;
    expect(body.weight).toBe(0.8);
    expect(body.description).toBe("Research collaboration");
  });
});

// ─────────────────────── Type-Level Interface Tests ───────────────────────

describe("Lineage type interfaces — compile-time structural checks", () => {
  it("LineRangeInfo has start_line and end_line", () => {
    const range: LineRangeInfo = { start_line: 1, end_line: 10 };
    expect(range.start_line).toBe(1);
    expect(range.end_line).toBe(10);
  });

  it("SourceDocumentInfo has document_id, chunk_ids, line_ranges", () => {
    const src: SourceDocumentInfo = {
      document_id: "d1",
      chunk_ids: ["c1"],
      line_ranges: [{ start_line: 1, end_line: 5 }],
    };
    expect(src.document_id).toBe("d1");
    expect(src.chunk_ids).toHaveLength(1);
    expect(src.line_ranges).toHaveLength(1);
  });

  it("DescriptionVersionResponse has version, description, source_chunk_id, created_at", () => {
    const dv: DescriptionVersionResponse = {
      version: 1,
      description: "First description",
      source_chunk_id: "c1",
      created_at: "2026-01-01T00:00:00Z",
    };
    expect(dv.version).toBe(1);
    expect(dv.source_chunk_id).toBe("c1");
  });

  it("ExtractionStatsResponse has all counters and optional processing_time_ms", () => {
    const stats: ExtractionStatsResponse = {
      total_entities: 10,
      unique_entities: 5,
      total_relationships: 8,
      unique_relationships: 4,
    };
    expect(stats.processing_time_ms).toBeUndefined();
    const statsWithTime: ExtractionStatsResponse = {
      ...stats,
      processing_time_ms: 1000,
    };
    expect(statsWithTime.processing_time_ms).toBe(1000);
  });

  it("EntitySourceInfo has document_id, chunks, first_extracted_at", () => {
    const src: EntitySourceInfo = {
      document_id: "d1",
      chunks: [{ chunk_id: "c1", start_line: 1, end_line: 5 }],
      first_extracted_at: "2026-01-01T00:00:00Z",
    };
    expect(src.chunks).toHaveLength(1);
    expect(src.first_extracted_at).toBeDefined();
  });

  it("ChunkSourceInfo has optional source_text", () => {
    const cs: ChunkSourceInfo = {
      chunk_id: "c1",
      source_text: "Hello world",
    };
    expect(cs.source_text).toBe("Hello world");
  });

  it("RelatedEntityInfo has shared_documents count", () => {
    const rel: RelatedEntityInfo = {
      entity_id: "e2",
      entity_name: "BOB",
      relationship_type: "KNOWS",
      shared_documents: 3,
    };
    expect(rel.shared_documents).toBe(3);
  });

  it("ExtractionMetadataInfo has model, gleaning_iterations, duration_ms, tokens, cached", () => {
    const meta: ExtractionMetadataInfo = {
      model: "gpt-4o",
      gleaning_iterations: 2,
      duration_ms: 500,
      input_tokens: 200,
      output_tokens: 100,
      cached: true,
    };
    expect(meta.cached).toBe(true);
    expect(meta.gleaning_iterations).toBe(2);
  });

  it("DocumentFullLineageResponse has optional metadata and lineage objects", () => {
    const full: DocumentFullLineageResponse = {
      document_id: "d1",
    };
    expect(full.metadata).toBeUndefined();
    expect(full.lineage).toBeUndefined();

    const fullWithData: DocumentFullLineageResponse = {
      document_id: "d1",
      metadata: { title: "Test" },
      lineage: { pipeline: "v1" },
    };
    expect(fullWithData.metadata?.title).toBe("Test");
  });

  it("ChunkLineageResponse has all optional position and entity fields", () => {
    const cl: ChunkLineageResponse = {
      chunk_id: "c1",
    };
    expect(cl.document_id).toBeUndefined();
    expect(cl.entity_count).toBeUndefined();

    const clFull: ChunkLineageResponse = {
      chunk_id: "c1",
      document_id: "d1",
      document_name: "test.pdf",
      document_type: "pdf",
      index: 0,
      start_line: 1,
      end_line: 10,
      start_offset: 0,
      end_offset: 100,
      token_count: 25,
      content_preview: "Hello...",
      entity_count: 3,
      relationship_count: 2,
      entity_names: ["A", "B", "C"],
      document_metadata: { author: "Smith" },
    };
    expect(clFull.entity_names).toHaveLength(3);
    expect(clFull.document_metadata?.author).toBe("Smith");
  });

  it("GraphNode has optional node_type, description, degree, properties", () => {
    const node: GraphNode = { id: "n1", label: "PERSON" };
    expect(node.node_type).toBeUndefined();

    const fullNode: GraphNode = {
      id: "n1",
      label: "PERSON",
      node_type: "entity",
      description: "A person node",
      degree: 5,
      properties: { source: "doc-1" },
    };
    expect(fullNode.degree).toBe(5);
    expect(fullNode.properties?.source).toBe("doc-1");
  });

  it("GraphEdge has edge_type, optional weight and properties", () => {
    const edge: GraphEdge = { source: "n1", target: "n2", edge_type: "KNOWS" };
    expect(edge.weight).toBeUndefined();

    const fullEdge: GraphEdge = {
      source: "n1",
      target: "n2",
      edge_type: "KNOWS",
      weight: 0.8,
      properties: { context: "work" },
    };
    expect(fullEdge.weight).toBe(0.8);
  });

  it("MergeEntitiesRequest has source_entity, target_entity, optional strategy", () => {
    const merge: MergeEntitiesRequest = {
      source_entity: "A",
      target_entity: "B",
    };
    expect(merge.strategy).toBeUndefined();

    const mergeWithStrategy: MergeEntitiesRequest = {
      source_entity: "A",
      target_entity: "B",
      strategy: "merge",
    };
    expect(mergeWithStrategy.strategy).toBe("merge");
  });
});
