---
title: Brutal honest SDK assessment
description: What the EdgeQuake SDKs actually cover against the live API for product v0.32.2, including known client bugs and gaps for v0.33.0.
---

# Brutal honest SDK assessment

This page states what is true in the repo today. It is for maintainers and integrators who need gaps, not marketing. **Date refreshed:** 2026-10-09. **Server:** EdgeQuake **v0.32.2** (plus v0.33.0 Connections in HEAD). **Law:** `routes.rs` + OpenAPI.

## What "code is law" means

- Paths and verbs must match `routes.rs`. Invented URLs are bugs.
- Request bodies must match handler DTOs (for example bulk conversation ops use `conversation_ids`, not `ids`).
- Responses should deserialize the JSON the API returns (`affected` on bulk ops, `items` + `pagination` for conversations, `page` / `page_size` for documents).

OpenAPI still wins for every optional field on large structs. SDKs often trim models.

## Version decoupling

SDK source is **0.4.0**. Product is **0.32.2**. Published registries lag: PyPI **0.3.0**, npm **0.1.0**, crates.io **0.4.0**. Java, Kotlin, Go, C#, Ruby, PHP and Swift are monorepo-only until published. See [VERSION-POLICY](VERSION-POLICY.md).

## Tier 1 (Rust, Python, TypeScript)

| Area | Verdict |
|------|---------|
| Maintenance | Highest; CI and refactors land here first |
| Document list | Lawful params: `page`, `page_size`, `date_from`, `date_to`, `document_pattern` (Python and TS) |
| Parse API (SPEC-094) | All three ship `parse()`, `backends()`, `job()` |
| Query body | **TypeScript** sends `max_results` / `enable_rerank`. **Python** still sends `top_k` / `rerank` (ignored by the server). **Rust** still models `top_k` on `QueryRequest` |
| Cancel / progress | Python leads on `tasks.cancel` and PDF `task_id`; TS/Rust catching up on typed progress events |
| Presentation fields | `display_status` / `ui_phase` exist in JSON; not always first-class on models |
| Connections / providers/test | **Missing** in all three |

## Tier 2 (Go, Java, Kotlin, C#, Ruby, PHP, Swift)

Useful for documents, query, graph and a subset of ops. Gaps are larger:

| Issue | Detail |
|-------|--------|
| Go list pagination | Sends `per_page`; API expects `page_size` |
| Registry presence | None of these are on Maven Central, NuGet, RubyGems, Packagist or pkg.go.dev as of this check |
| Connections | None wrap SPEC-163 |
| Query defaults | Several clients default `mode` to `"hybrid"`; server default is `"mix"` |

## Suspected client bugs (do not paper over in docs)

1. Python `QueryResource.execute` body uses `top_k` and `rerank` instead of `max_results` and `enable_rerank`.
2. Rust `QueryRequest` exposes `top_k` rather than `max_results`.
3. Go `DocumentService.List` query param `per_page` should be `page_size`.
4. TypeScript package.json name is `edgequake-sdk`, but some source comments still say `@edgequake/sdk`.
5. MCP stdio package `@edgequake/mcp-server` depends on `edgequake-sdk ^0.1.0` while source is 0.4.0.

## Bottom line

Use Tier 1 for new work. Install from the monorepo when you need 0.4.0 features. Call Connections and provider test with raw HTTP until an SDK release adds them. Track coverage in [SDK-API-COVERAGE](../../specs/009-skd-update/SDK-API-COVERAGE.md).
