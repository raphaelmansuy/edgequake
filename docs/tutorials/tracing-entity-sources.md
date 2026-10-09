---
title: "Tutorial: Trace entities to their sources"
description: Follow an answer, an entity or a relationship in EdgeQuake back to the exact chunk, line range and document it came from, using the lineage and provenance endpoints.
---

In this tutorial you trace facts back to their origin. You start from a query answer, an entity or a document, and end at the chunk and line range that support it. Use this to audit answers and to find bad extractions.

**Prerequisites:** a workspace with at least one completed document (see [First RAG app](first-rag-app.md)) and the variables `EQ_API` and `WORKSPACE_ID`.

## The lineage chain

Every fact in the graph keeps a link to the text it came from. The diagram shows the chain from a document down to an entity, and the three ways to walk it.

```mermaid
%%{init: {"theme":"base","themeVariables":{"primaryColor":"#E0E7FF","primaryBorderColor":"#6366F1","primaryTextColor":"#1E1B4B","secondaryColor":"#D1FAE5","secondaryBorderColor":"#10B981","secondaryTextColor":"#064E3B","tertiaryColor":"#FEF3C7","tertiaryBorderColor":"#F59E0B","tertiaryTextColor":"#6B7A90","lineColor":"#7A889C","clusterBkg":"rgba(99,102,241,0.07)","clusterBorder":"#A5B4FC","noteBkgColor":"#FEF9C3","noteTextColor":"#422006","textColor":"#6B7A90","titleColor":"#6B7A90","signalColor":"#7A889C","signalTextColor":"#6B7A90","loopTextColor":"#6B7A90","edgeLabelBackground":"#F1F5F9","actorLineColor":"#94A3B8"}}}%%
%% eq-theme:v1
erDiagram
  DOCUMENT ||--o{ CHUNK : "split into"
  CHUNK ||--o{ ENTITY : "mentions"
  CHUNK ||--o{ RELATIONSHIP : "supports"
  ENTITY ||--o{ RELATIONSHIP : "connects"
```

Read it from the top: a document is split into chunks, and each chunk can produce entities and relationships. An entity that appears in many chunks keeps all of them as sources.

Pick the endpoint for your starting point:

| You have | Call | You get |
|----------|------|---------|
| A document ID | `GET /api/v1/lineage/documents/{document_id}` | All entities and relationships of the document, each with its source chunks. |
| An entity name | `GET /api/v1/lineage/entities/{entity_name}` | Every document, chunk and line range that mentions it, and its description history. |
| An entity name | `GET /api/v1/entities/{entity_id}/provenance` | Sources with the source text, plus related entities. The ID is the uppercase entity name. |
| A chunk ID | `GET /api/v1/chunks/{chunk_id}` | The chunk text, line range, entities, relationships and extraction metadata. |
| A chunk ID | `GET /api/v1/chunks/{chunk_id}/lineage` | The parent document, position and entity names. |
| A document ID | `GET /api/v1/documents/{document_id}/lineage` | The chunk tree of the document with entity links. |

Send `X-Workspace-ID` on every call. Chunk IDs have the form `{document_id}-chunk-{N}`.

## 1. Start from an answer

Run a query and read the `sources` array. Each source has a `source_type` and an `id`. For chunk sources, the `id` is a chunk ID.

```bash
curl -s -X POST "$EQ_API/api/v1/query" \
  -H "Content-Type: application/json" \
  -H "X-Workspace-ID: $WORKSPACE_ID" \
  -d '{"query": "Who leads the NeuralSearch project?", "include_references": true}' \
  | tee answer.json | jq '.sources[] | {source_type, id, document_id, file_path, start_line, end_line, score}'
```

Expected output (values vary):

```json
{
  "source_type": "chunk",
  "id": "9d1e0a52-...-chunk-0",
  "document_id": "9d1e0a52-...",
  "file_path": "sample.md",
  "start_line": 1,
  "end_line": 8,
  "score": 0.82
}
```

Pick a chunk ID for the next step:

```bash
export CHUNK_ID=$(jq -r '[.sources[] | select(.source_type=="chunk")][0].id' answer.json)
```

## 2. Read the chunk

```bash
curl -s "$EQ_API/api/v1/chunks/$CHUNK_ID" -H "X-Workspace-ID: $WORKSPACE_ID" \
  | jq '{document_id, document_name, index, start_line, end_line, token_count, entities: [.entities[].name], extraction_metadata}'
```

Expected output:

```json
{
  "document_id": "9d1e0a52-...",
  "document_name": "sample.md",
  "index": 0,
  "start_line": 1,
  "end_line": 8,
  "token_count": 96,
  "entities": ["SARAH_CHEN", "TECHCORP", "NEURALSEARCH"],
  "extraction_metadata": { "model": "...", "gleaning_iterations": 1, "cached": false }
}
```

The `extraction_metadata` block, when present, names the model that extracted the chunk. It shows whether gleaning ran and whether the result came from cache. Use it to compare extraction quality between models. Chunks from PDFs can also carry `page_start` and `page_end`.

## 3. Trace an entity

Ask where an entity appears. The name is the uppercase form shown in the graph.

```bash
curl -s "$EQ_API/api/v1/lineage/entities/SARAH_CHEN" -H "X-Workspace-ID: $WORKSPACE_ID" \
  | jq '{entity_name, entity_type, source_count, sources: [.source_documents[] | {document_id, chunk_ids, line_ranges}]}'
```

Expected output:

```json
{
  "entity_name": "SARAH_CHEN",
  "entity_type": "PERSON",
  "source_count": 1,
  "sources": [
    { "document_id": "9d1e0a52-...", "chunk_ids": ["9d1e0a52-...-chunk-0"], "line_ranges": [{ "start_line": 1, "end_line": 8 }] }
  ]
}
```

The response also has `description_versions`. When an entity appears in several chunks, EdgeQuake merges the descriptions. Each version lists its `source_chunk_id`, so you can see which text added which detail.

For the source text itself and for related entities, use the provenance route:

```bash
curl -s "$EQ_API/api/v1/entities/SARAH_CHEN/provenance" -H "X-Workspace-ID: $WORKSPACE_ID" \
  | jq '{total_extraction_count, sources: [.sources[] | {document_name, chunks: [.chunks[] | {chunk_id, source_text}]}], related: [.related_entities[].entity_name]}'
```

## 4. Trace a document

List everything one document contributed to the graph:

```bash
curl -s "$EQ_API/api/v1/lineage/documents/$DOC_ID" -H "X-Workspace-ID: $WORKSPACE_ID" \
  | jq '{chunk_count, extraction_stats, entities: [.entities[] | {name, entity_type, is_shared, source_chunks}]}'
```

The field `is_shared` is `true` when the entity also appears in another document. The numbers in `extraction_stats` compare raw extractions with unique ones, so they show how much merging happened.

To export a document's lineage, call `GET /api/v1/documents/{document_id}/lineage/export`.

## 5. Audit a suspicious answer

Follow this loop when an answer looks wrong:

```mermaid
%%{init: {"theme":"base","themeVariables":{"primaryColor":"#E0E7FF","primaryBorderColor":"#6366F1","primaryTextColor":"#1E1B4B","secondaryColor":"#D1FAE5","secondaryBorderColor":"#10B981","secondaryTextColor":"#064E3B","tertiaryColor":"#FEF3C7","tertiaryBorderColor":"#F59E0B","tertiaryTextColor":"#6B7A90","lineColor":"#7A889C","clusterBkg":"rgba(99,102,241,0.07)","clusterBorder":"#A5B4FC","noteBkgColor":"#FEF9C3","noteTextColor":"#422006","textColor":"#6B7A90","titleColor":"#6B7A90","signalColor":"#7A889C","signalTextColor":"#6B7A90","loopTextColor":"#6B7A90","edgeLabelBackground":"#F1F5F9","actorLineColor":"#94A3B8"}}}%%
%% eq-theme:v1
flowchart TD
  A["Wrong or odd answer"] --> B["Read sources in the query response"]
  B --> C["Fetch each source chunk"]
  C --> D{"Chunk text supports the answer?"}
  D -- "Yes" --> E["Answer is grounded"]
  D -- "No" --> F["Check extraction metadata and entities of the chunk"]
  F --> G["Fix: bigger model, entity types, reprocess"]
%% eq-classes
classDef eqLlm fill:#FEF3C7,stroke:#F59E0B,color:#451A03
class G eqLlm
```

Read it top to bottom. If the chunk text supports the answer, the fault is in the question or the prompt. If it does not, the extraction is the problem. See [Document ingestion](document-ingestion.md) for the remedies and [Query optimization](query-optimization.md) for retrieval fixes.

Common findings:

| Finding | Meaning | Action |
|---------|---------|--------|
| The source path is `injection` | The text came from [knowledge injection](knowledge-injection.md), not from a document. | Check the injection list. |
| The same real-world thing has two entity names | Names did not normalize to the same ID. | See [Entity normalization](../deep-dives/entity-normalization.md); merge with `POST /api/v1/graph/entities/merge`. |
| The description mixes unrelated facts | Two things share one name. | Read `description_versions` and split the sources. |
| Extraction metadata shows `cached: true` for a bad result | A cached extraction was reused. | Reprocess the document after you fix the cause. |

## Next steps

- [Lineage tracking architecture](../architecture/lineage-tracking.md)
- [Lineage API reference](../api-reference/lineage-endpoints.md)
- [Knowledge injection](knowledge-injection.md)
