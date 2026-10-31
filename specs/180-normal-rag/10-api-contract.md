# 10 — API contract

Parent: [README](README.md) · Prev: [09-retrieval-and-fusion](09-retrieval-and-fusion.md) · Next: [11-ux-ui-spec](11-ux-ui-spec.md)

OpenAPI: add the fields, then `make codegen-openapi-refresh`. Contract test `cargo test -p edgequake-api --test spec027_api_contract` must pass. SDKs regenerate in the same change. MCP tool schemas in [tools.rs](../../edgequake/crates/edgequake-api/src/mcp/gateway/tools.rs) stay hand-written and must match.

## Create workspace

`POST /tenants/{tenant_id}/workspaces`

New fields on the create body:

| Field | Type | Default | Notes |
|-------|------|---------|-------|
| `rag_profile` | `graph` \| `normal` | `graph` | Immutable after this call |
| `embedding_modalities` | `text` \| `text,image` | `text` | Immutable. `text,image` only with `normal` |
| `fts_regconfig` | enum in [06](06-data-and-db-contract.md) | `english` | Immutable |

Response `Workspace` includes the three fields plus `rag_profile_capabilities`, a read-only object of the booleans the UI needs (`extracts_graph`, `embeds_images`, `shows_graph_ui`). The UI does not re-derive the matrix.

### Create errors

| HTTP | Code | When |
|------|------|------|
| 400 | `invalid_rag_profile` | not `graph` or `normal` |
| 400 | `invalid_embedding_modalities` | not the two allowed strings |
| 400 | `invalid_fts_regconfig` | not in the check list |
| 400 | `extraction_mode_not_applicable` | normal and extraction_mode set |
| 422 | `embedding_model_not_multimodal` | `text,image` and model card lacks `embeds_image` |
| 422 | `embedding_dimension_rejected` | not 768, 1536, or 3072 for `gemini-embedding-2` |
| 422 | `modalities_require_normal` | `text,image` with `graph` |

Existing SPEC-178 choice errors are unchanged.

## Update workspace

`PUT /workspaces/{id}`

If the body contains `rag_profile`, `embedding_modalities`, or `fts_regconfig`: **409** with code `rag_profile_immutable`, `embedding_modalities_immutable`, or `fts_regconfig_immutable`. Other fields update as today. Sending the current value still 409, so a client cannot "confirm" the column. The trigger is the backstop if a bug skips the handler.

`POST /workspaces/{id}/rebuild-knowledge-graph` on normal: **409** `profile_has_no_graph`.

`POST /workspaces/{id}/rebuild-embeddings`: allowed. If the new model lacks `embeds_image` and modalities are `text,image`: **422** `embedding_model_not_multimodal`.

## Upload

Multipart and JSON upload:

- `extraction_mode` set on a normal workspace: **400** `extraction_mode_not_applicable`.
- Image content type on a text-only workspace: **422** `embedding_model_not_multimodal`.
- Animated image: **422** `image_animated_unsupported`.

Document GET adds `doc_kind` and does not require `entity_count` to be non-zero for `status=completed`.

Reprocess body `mode=entities_only` or `merge_only` on normal: **409** `reprocess_mode_not_applicable`.

## Query and chat

`DocumentFilter` and `ChatCompletionRequest` gain `doc_kinds`, `mime_types`, `source_modalities`.

Query `mode` on normal:

| mode | Result |
|------|--------|
| omitted | normal retrieval |
| `naive` | normal retrieval |
| `bypass` | no retrieval, as today |
| `local`, `global`, `hybrid`, `mix` | **422** `mode_not_supported_by_profile` |

Chat uses the same table. The UI default for a normal workspace is omitted mode, not Mix.

Multipart query image: field `query_image`. See [09-retrieval-and-fusion](09-retrieval-and-fusion.md). **422** `image_query_not_supported` when modalities are `text`. **400** `query_image_invalid` when the part is empty or not an image.

`top_k` on chat is currently ignored ([query findings](04-code-findings.md)). This pack does not fix that. Normal retrieval uses `max_chunks` from engine config. Do not pretend `top_k` works.

## MCP

`eq_search` schema: add the three filter fields and document that on a normal workspace `mode` defaults to naive-equivalent and graph modes error. Implementation sets `include_subgraph: false` when `queries_graph` is false ([F-180-16](04-code-findings.md)).

`QUERY_INSTRUCTIONS` become profile-aware. Normal text: the workspace is a chunk and embedding index, entity tools are absent, do not invent entities.

Entity tools (`eq_entity_search`, `eq_entity_get`, `eq_neighborhood`, graph image): **409** `profile_has_no_graph`.

## Error body

Existing error JSON shape. `code` is stable. `message` is one sentence an operator can show. No stack trace.

## Compatibility

- Old clients that omit `rag_profile` create a graph workspace.
- Old clients that PUT a workspace without the new fields succeed.
- Old clients that PUT `rag_profile: "graph"` on an existing graph workspace get 409. Release notes must say so. The wizard and the generated SDK must stop sending the field on update. [EC-180-04](14-edge-cases.md).
- Graph query responses are unchanged.
