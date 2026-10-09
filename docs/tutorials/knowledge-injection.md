---
title: "Tutorial: Knowledge injection"
description: Add domain glossaries and definitions to an EdgeQuake workspace so retrieval understands your terms, without adding them to answer citations.
---

In this tutorial you add a glossary to a workspace, check that it was processed and query with it. You also learn the limits and how to update or delete an entry.

**Prerequisites:** a workspace (see [First RAG app](first-rag-app.md)) and the variables `EQ_API` and `WORKSPACE_ID`. Available since v0.8.0 ([issue #131](https://github.com/raphaelmansuy/edgequake/issues/131), [specification](../../specifications/0002_knowledge_injection_issue_131/)).

## What it is

**Knowledge injection** lets you add short texts such as glossaries, acronym lists and synonym tables to a workspace. EdgeQuake runs them through the normal extraction pipeline, so the knowledge graph gains entities such as `OEE` and `OVERALL_EQUIPMENT_EFFECTIVENESS` and a link between them. Retrieval can then reach documents that use either term.

Injected text is never shown as a citation. Your users see answers that use your vocabulary, and the `sources` list contains only real documents.

```mermaid
%%{init: {"theme":"base","themeVariables":{"primaryColor":"#E0E7FF","primaryBorderColor":"#6366F1","primaryTextColor":"#1E1B4B","secondaryColor":"#D1FAE5","secondaryBorderColor":"#10B981","secondaryTextColor":"#064E3B","tertiaryColor":"#FEF3C7","tertiaryBorderColor":"#F59E0B","tertiaryTextColor":"#6B7A90","lineColor":"#7A889C","clusterBkg":"rgba(99,102,241,0.07)","clusterBorder":"#A5B4FC","noteBkgColor":"#FEF9C3","noteTextColor":"#422006","textColor":"#6B7A90","titleColor":"#6B7A90","signalColor":"#7A889C","signalTextColor":"#6B7A90","loopTextColor":"#6B7A90","edgeLabelBackground":"#F1F5F9","actorLineColor":"#94A3B8"}}}%%
%% eq-theme:v1
flowchart LR
  A["Glossary text or file"] --> B["Validate"]
  B --> C["Chunk and extract entities"]
  C --> D["Embed and merge into the graph"]
  D --> E["Tag as injection"]
  F["Query"] --> G["Retrieve from graph and chunks"]
  D --> G
  G --> H["Remove injection items from sources"]
  H --> I["Answer with real citations"]
```

Read it left to right. The top row runs once per entry. The bottom row runs on every query. Injected items help retrieval but are removed before the sources list is returned.

## Limits

| Limit | Value |
|-------|-------|
| Text size | 100 KB per entry |
| Name length | 1 to 100 characters |
| File types | `.txt`, `.md`, `.csv`, `.json` (UTF-8 text) |
## 1. Create an entry

Send the workspace ID in the `X-Workspace-ID` header. The server reads the workspace from that header. Use the same ID in the URL path.

```bash
curl -s -X PUT "$EQ_API/api/v1/workspaces/$WORKSPACE_ID/injection" \
  -H "Content-Type: application/json" \
  -H "X-Workspace-ID: $WORKSPACE_ID" \
  -d '{
    "name": "Manufacturing Glossary",
    "content": "OEE = Overall Equipment Effectiveness\nTPM = Total Productive Maintenance\nKPI = Key Performance Indicator\n"
  }' | tee inj.json | jq '.'

export INJECTION_ID=$(jq -r '.injection_id' inj.json)
```

Expected output (status `202 Accepted`):

```json
{
  "injection_id": "a1b2c3d4-...",
  "workspace_id": "3f6c1c0e-...",
  "version": 1,
  "status": "processing"
}
```

Each `PUT` creates a new entry with a new ID. To change an existing entry, use `PATCH` (step 4).

To upload a file instead:

```bash
curl -s -X PUT "$EQ_API/api/v1/workspaces/$WORKSPACE_ID/injection/file" \
  -H "X-Workspace-ID: $WORKSPACE_ID" \
  -F "name=Domain Glossary" \
  -F "file=@glossary.txt" | jq '.'
```

If you leave out `name`, the file name is used.

## 2. Wait for processing

Poll the entry until `status` is `completed` or `failed`:

```bash
curl -s "$EQ_API/api/v1/workspaces/$WORKSPACE_ID/injections/$INJECTION_ID" \
  -H "X-Workspace-ID: $WORKSPACE_ID" \
  | jq '{name, status, version, entity_count, source_type, error}'
```

Expected output:

```json
{
  "name": "Manufacturing Glossary",
  "status": "completed",
  "version": 1,
  "entity_count": 3,
  "source_type": "text",
  "error": null
}
```

An `entity_count` of `0` means the model found nothing to extract. The text may be too short, or the model too small. Add a sentence of context per term, or use a larger model.

## 3. Query

Ask a question that uses the glossary terms:

```bash
curl -s -X POST "$EQ_API/api/v1/query" \
  -H "Content-Type: application/json" \
  -H "X-Workspace-ID: $WORKSPACE_ID" \
  -d '{"query": "What does OEE measure?"}' | jq '{answer, sources: [.sources[] | {source_type, file_path}]}'
```

The answer can use the glossary. No source has the file path `injection`, because those items are filtered out.

## 4. Update, list and delete

| Goal | Call |
|------|------|
| List entries | `GET /api/v1/workspaces/{workspace_id}/injections?limit=20&offset=0` |
| Read one entry | `GET /api/v1/workspaces/{workspace_id}/injections/{injection_id}` |
| Rename | `PATCH` the same path with `{"name": "New name"}` |
| Replace the content | `PATCH` the same path with `{"content": "..."}`. The version number goes up and the entry is processed again. |
| Delete | `DELETE /api/v1/workspaces/{workspace_id}/injections/{injection_id}` |

A rename does not reprocess the entry. A content change does. Example:

```bash
curl -s -X PATCH "$EQ_API/api/v1/workspaces/$WORKSPACE_ID/injections/$INJECTION_ID" \
  -H "Content-Type: application/json" -H "X-Workspace-ID: $WORKSPACE_ID" \
  -d '{"content": "OEE = Overall Equipment Effectiveness\nMTBF = Mean Time Between Failures\n"}' | jq '.'
```

Deleting an entry removes its chunks, vectors and graph items.

## 5. Use the Web UI

1. Select a workspace and open **Knowledge** in the sidebar. The page title is **Knowledge Injection**.
2. Select **New Injection**.
3. Use the **Text** tab (a name and the content) or the **File** tab (a `.txt`, `.md`, `.csv` or `.json` file, with an optional name).
4. Select **Create** and watch the status change from `processing` to `completed`.

The list shows the status and the number of entities for each entry. You can open an entry to edit it or delete it.

## Tips

- Keep one entry per domain, such as manufacturing, finance or legal. Small entries are easier to update.
- Write one definition per line, with the short form first: `OEE = Overall Equipment Effectiveness`.
- Add context when a term is ambiguous: `Bearing (mechanical part, not a compass direction)`.
- Update an entry with `PATCH` instead of creating a new one. Otherwise old and new entries both stay in the graph.
- Injection is not a substitute for documents. Use it for short definitions, and upload long material as [documents](document-ingestion.md).

## Troubleshooting

| Symptom | Cause | Fix |
|---------|-------|-----|
| `400` "Name must be between 1 and 100 characters" | Empty or long name. | Shorten the name. |
| `400` "content exceeds 100KB limit" | Too much text. | Split into several entries. |
| `400` "Unsupported file type" | Not `.txt`, `.md`, `.csv` or `.json`. | Convert the file. |
| `status` is `failed` | Model error. Read `error`. | Fix the provider (`edgequake doctor`), then `PATCH` the entry with its content again. |
| The glossary has no effect | `entity_count` is `0`, or the question does not use the terms. | Check step 2 and add context. |

To see injected items in the graph, use `GET /api/v1/graph/entities?search=OEE`. See [Tracing entity sources](tracing-entity-sources.md) to follow them back to the entry.

## Next steps

- [Document ingestion](document-ingestion.md)
- [REST API reference](../api-reference/rest-api.md)
- [Query optimization](query-optimization.md)
