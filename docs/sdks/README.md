---
title: "EdgeQuake SDKs"
description: "EdgeQuake client SDKs: install, versions, and language index."
---

# EdgeQuake SDKs

> **Product: v0.23.0** · Contract: [OpenAPI snapshot](../../edgequake_webui/openapi/openapi.snapshot.json) · Spec ops: [Ingestion cancel & fairness](../ingestion-cancel-and-fairness.md)

Official HTTP clients for the EdgeQuake API. SDK **package** versions (~**0.4.0**) are decoupled from the server — check `pyproject.toml`, `package.json`, or crate manifests for the client semver you install.

**Canonical routing** lives in `edgequake/crates/edgequake-api/src/routes.rs`; OpenAPI is the field-level truth for DTOs.

Use these docs for **copy-paste examples** and day-to-day integration. For honest gaps and parity, read [Brutal assessment](./BRUTAL-ASSESSMENT.md) and the spec tracker [SDK-API-COVERAGE.md](../../specs/009-skd-update/SDK-API-COVERAGE.md).

## By language

| Tier | SDK | Folder | Package / crate |
|------|-----|--------|-----------------|
| 1 | Rust | [rust](./rust/) | `sdks/rust` — `edgequake-sdk` |
| 1 | Python | [python](./python/) | `sdks/python` — PyPI `edgequake-sdk` ~0.4.0 |
| 1 | TypeScript / Node | [typescript](./typescript/) | `sdks/typescript` — `@edgequake/sdk` ~0.4.0 |
| 2 | Kotlin/JVM | [kotlin](./kotlin/) | `sdks/kotlin` |
| 2 | Swift | [swift](./swift/) | `sdks/swift` |
| 2 | Go | [go](./go/) | `sdks/go` — monorepo path; not on pkg.go.dev yet |
| 2 | Java | [java](./java/) | `sdks/java` — Maven Central ~0.4.0 |
| 2 | C# / .NET | [csharp](./csharp/) | `sdks/csharp` |
| 2 | Ruby | [ruby](./ruby/) | `sdks/ruby` — path install; `lib/` present |
| 2 | PHP | [php](./php/) | `sdks/php` — experimental; monorepo / OpenAPI |

## Version decoupling

| What | Version | Notes |
|------|---------|-------|
| EdgeQuake server / Docker | **0.23.0** | `ghcr.io/raphaelmansuy/edgequake:0.23.0` |
| SDK packages (PyPI, npm, Maven, …) | **~0.4.0** | Independent release cadence |
| API contract | OpenAPI snapshot | Must match server you target |

Upgrade the **server** and **SDK** on independent schedules; regenerate or bump SDKs when OpenAPI drifts.

## Headers and tenancy (all SDKs)

Most `/api/v1/*` calls expect workspace context:

- `Authorization: Bearer <jwt>` (or API key per server config)
- `X-Tenant-ID`, `X-User-ID`, `X-Workspace-ID` as required by your deployment

Configure these on the client **once**; every resource reuses the same transport.

## Quick actions

1. **Health** — `GET /health` (unversioned) before anything else.
2. **List documents** — `GET /api/v1/documents` with optional `page`, `page_size`, `date_from`, `date_to`, `document_pattern`. Document responses expose `display_status` (badge key from `IngestionStatusMapper`) and `ui_phase` (`idle | running | stopping | terminal`) — prefer those over raw `status`/`stage` for UI (SPEC-057 P4).
3. **Batch ingestion** — SDKs expose:
   - `POST /api/v1/documents/upload/batch` (text/images — **not** PDFs)
   - `POST /api/v1/documents/pdf/batch` (PDFs; WebUI uses N× `/documents/pdf`)
4. **Progress / cancel (v0.23)** — WebSocket `/ws/progress/{track_id}` or `POST /api/v1/tasks/{track_id}/cancel`; see [Ingestion cancel & fairness](../ingestion-cancel-and-fairness.md). Tier 1 SDKs lead; Tier 2 may need raw HTTP.
5. **Stateless parse (v0.23 / SPEC-094)** — `POST /api/v1/parse` converts a PDF to Markdown **without** ingestion residue. Tier 1 SDKs (Rust, Python, TypeScript) ship a typed `parse` resource: `parse()`, `backends()`, `job()`. Sync is default (≤ 15 pages / 20 MiB); pass `async: true` (or `Prefer: respond-async`) for jobs up to 1000 pages, polled via `GET /api/v1/parse/jobs/{id}`. Tier 2 SDKs: use raw HTTP until parity lands.
6. **Conversations** — list uses cursor filters (`filter[folder_id]`, etc.); bulk delete body uses **`conversation_ids`**; response uses **`affected`**.

**SPEC-103 LLM cache (server-side, no SDK field):** query keyword extraction and answers are cached by default (`EDGEQUAKE_LLM_CACHE=1`; overrides `EDGEQUAKE_KEYWORD_CACHE` / `EDGEQUAKE_QUERY_ANSWER_CACHE`). Caching is a server concern — clients just send the same query and get the same answer; set the env vars on the **server**, not the client.

## See also

- [REST API overview](../api-reference/rest-api.md)
- [Multi-tenant tutorial](../tutorials/multi-tenant.md)
- [Integrations](../integrations/index.md)
