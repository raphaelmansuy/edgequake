---
title: "Knowledge Injection — Tutorial"
description: "Enrich your workspace with domain glossaries that improve retrieval without polluting citations"
---

# Knowledge Injection

> **Available since v0.8.0** · Issue [#131](https://github.com/raphaelmansuy/edgequake/issues/131) · Spec [SPEC-0002](../../specifications/0002_knowledge_injection_issue_131/)

## What It Is

Knowledge Injection lets you add **domain glossaries, acronym definitions, and synonym mappings** to a workspace. The definitions are processed by the same entity-extraction pipeline as regular documents, so the knowledge graph learns that "OEE" equals "Overall Equipment Effectiveness". At query time, those relationships expand your search terms automatically.

**Key property**: injection entries are **never listed as source citations**. Your users see answers enriched by your domain knowledge, but the citation list only contains real documents.

---

## Quick Start (5 minutes)

### 1. Inject a text glossary

```bash
curl -X PUT http://localhost:8080/api/v1/workspaces/default/injection \
  -H "Content-Type: application/json" \
  -H "X-Workspace-ID: default" \
  -d '{
    "name": "Manufacturing Glossary",
    "content": "OEE = Overall Equipment Effectiveness\nTPM = Total Productive Maintenance\nKPI = Key Performance Indicator\n"
  }'
```

Response:

```json
{
  "injection_id": "a1b2c3d4-e5f6-...",
  "workspace_id": "default",
  "name": "Manufacturing Glossary",
  "status": "processing"
}
```

### 2. Poll until complete

```bash
curl http://localhost:8080/api/v1/workspaces/default/injection/a1b2c3d4-e5f6-... \
  -H "X-Workspace-ID: default"
```

```json
{
  "injection_id": "a1b2c3d4-e5f6-...",
  "name": "Manufacturing Glossary",
  "status": "completed",
  "entity_count": 3,
  ...
}
```

### 3. Query — injection enriches the answer

```bash
curl -X POST http://localhost:8080/api/v1/query \
  -H "Content-Type: application/json" \
  -H "X-Workspace-ID: default" \
  -d '{"query": "What is the OEE of Line 3?", "mode": "hybrid"}'
```

The engine now understands "OEE" ≡ "Overall Equipment Effectiveness" and retrieves documents that use either term. The glossary is **not** listed as a source.

---

## Using the Web UI

1. Click **Knowledge** in the sidebar (BookOpen icon).
2. Click **+ Add Knowledge**.
3. Choose the **Text** tab, type a name, paste your definitions, and click **Save**.
   - Or choose the **File** tab to upload a `.txt` or `.md` file.
4. Watch the status badge change from `processing` → `completed`.
5. The entity count shows how many concepts were extracted.

To edit an entry: click its name → Edit button.  
To delete: click the trash icon → confirm in the dialog.

---

## File Upload

```bash
curl -X PUT http://localhost:8080/api/v1/workspaces/default/injection/file \
  -H "X-Workspace-ID: default" \
  -F "name=Domain Glossary" \
  -F "file=@glossary.txt"
```

Accepted formats: `.txt`, `.md`, plain text.

---

## Full CRUD Reference

| Operation      | Method   | Path                                                            |
| -------------- | -------- | --------------------------------------------------------------- |
| List all       | `GET`    | `/api/v1/workspaces/:id/injections`                              |
| Create (text)  | `PUT`    | `/api/v1/workspaces/:id/injection`                              |
| Create (file)  | `PUT`    | `/api/v1/workspaces/:id/injection/file`                       |
| Get detail     | `GET`    | `/api/v1/workspaces/:id/injection/:injection_id`                |
| Update         | `PATCH`  | `/api/v1/workspaces/:id/injections/:injection_id`                |
| Delete         | `DELETE` | `/api/v1/workspaces/:id/injection/:injection_id`                |

See [REST API Reference](../api-reference/rest-api.md#knowledge-injection-api) for full request/response schemas.

---

## Best Practices

1. **Keep glossaries focused** — one entry per domain (manufacturing, finance, legal). It makes them easier to update.
2. **Use canonical forms** — write `OEE = Overall Equipment Effectiveness` rather than vice versa; the LLM extracts both forms as synonymous entities.
3. **Update, don't delete+recreate** — `PATCH` with new content triggers a clean replace of all old entities, saving pipeline budget.
4. **Check entity count** — a count of 0 after processing usually means the content was too short or the LLM didn't extract any entities. Try adding richer descriptions.
5. **Combine with document uploads** — injected entities merge with document-extracted entities, enriching descriptions automatically.

---

## How It Works Internally

```
PUT .../injection { content: "OEE = Overall Equipment Effectiveness" }
        │
        ▼
1. Generate stable doc_id  →  "injection::{workspace_id}::{injection_id}"
        │
        ▼
2. Delete old pipeline artifacts (if updating)
   KV chunks + vectors + graph nodes tagged with this doc_id
        │
        ▼
3. Run standard pipeline
   Chunking → Entity extraction → Embedding → Graph merge
   All artifacts tagged: source_type = "injection"
        │
        ▼
4. Query engine: SourceReference filter
   sources where source_type == "injection" → removed from response
```

The tagging strategy means zero changes to the query engine's core logic — citation exclusion is a lightweight filter at the response-serialization layer.
