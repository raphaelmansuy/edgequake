---
title: "Brutal honest SDK assessment"
description: "Historical assessment of the SDK surface."
---

# Brutal honest SDK assessment

> **Product: v0.23.0** · Contract: OpenAPI · Spec ops: [Ingestion cancel & fairness](../ingestion-cancel-and-fairness.md)

**Date:** 2026-07-18  
**Server:** EdgeQuake **v0.23.0**  
**Law:** `edgequake/crates/edgequake-api/src/routes.rs` + generated OpenAPI.

This is not a marketing page. It states what is true in the repo today.

## What “code is law” means here

- **Paths and verbs** must match `routes.rs`. Invented URLs (e.g. old `/costs/workspace`) are bugs, not features.
- **Request bodies** must match handler DTOs (e.g. bulk conversation ops use **`conversation_ids`**, not `ids`).
- **Responses** should deserialize the common JSON the API actually returns (e.g. **`affected`** on bulk ops, paginated **`items`** + **`pagination`** for conversations).

OpenAPI still wins for **every optional field** on large structs; SDKs often trim models for ergonomics.

## Version decoupling

SDK package semver (~**0.4.0**) is **not** the product version (**0.23.0**). A 0.4.x client can talk to a 0.19 server if paths match OpenAPI; gaps are feature coverage, not wire incompatibility by default.

## Tier 1 (Rust, Python, TypeScript)

| Area | Verdict |
|------|---------|
| Maintenance | Highest; CI and refactors land here first. |
| Conversation list / messages | Query params aligned with API list handlers; paginated wrappers modeled. |
| Document list | Lawful params: `page`, `page_size`, `date_from`, `date_to`, `document_pattern`. |
| Costs / pipeline | TypeScript uses shared path constants to avoid drift. |
| Streaming | SSE / WebSocket helpers exist in places but are **not** a unified typed event layer across all three. |
| v0.23 cancel / progress | Python leads on `tasks.cancel`, PDF upload + `track_id`; TS/Rust catching up on typed progress events. |
| **v0.23 parse API (SPEC-094)** | All three Tier 1 SDKs ship a typed `parse` resource: `parse()`, `backends()`, `job()` (`POST /api/v1/parse`, `GET /api/v1/parse/backends`, `GET /api/v1/parse/jobs/{id}`). |
| v0.23 presentation fields | `display_status` / `ui_phase` are documented in the SDKs but **not** yet first-class typed fields on every model — read them from raw JSON when needed. |

**Bottom line:** Tier 1 is the reference track. If you need predictable behavior, prefer Tier 1.

## Tier 2 (Kotlin, Swift, Go, Java, C#)

| Area | Verdict |
|------|---------|
| Coverage | Broad surface area (documents, graph, chat, costs, etc.) but **less** systematically audited than Tier 1. |
| Conversations bulk delete | Aligned to `conversation_ids` + `affected` in recent passes. |
| Conversation list filters | Still **thin** in several Tier 2 SDKs (often “list with no query”). Tier 1 is ahead for `filter[…]` / cursor parity. |
| Java message update path | Some methods may still target older path shapes; verify against `routes.rs` before relying on them in production. |
| v0.23 gaps | Task cancel, `display_status` / `ui_phase`, PDF progress SSE — **spot-check** or use Tier 1 / raw HTTP. |
| **v0.23 parse API (SPEC-094)** | **Not wrapped** in Tier 2 SDKs — use raw HTTP `POST /api/v1/parse` (multipart; sync ≤ 15 pages / 20 MiB, async ≤ 1000 pages) or a Tier 1 client. |

**Bottom line:** Tier 2 is usable and improving, but **you should verify** critical paths against OpenAPI or Tier 1 when stakes are high.

## Ruby

- **`lib/` exists** (`lib/edgequake/` — client, config, services). Path install from `sdks/ruby`.
- Unit tests + CI present; **not** on RubyGems as a standalone published gem yet.
- Treat as **Tier 2 experimental** — same v0.23 cancel/progress/parse gaps as other Tier 2 clients until parity work lands.

## PHP

- Lives at `sdks/php` with PHPUnit + CI; **experimental**, not Packagist-first.
- Maturity: solid HTTP helper + core services; **weaker** on v0.23 progress/cancel wrappers than Python.
- Prefer OpenAPI or Tier 1 for ingestion cancel SSOT until PHP helpers catch up.

## SPEC-103 LLM cache (all SDKs)

Query keyword extraction + answer caching is **server-side only** (`EDGEQUAKE_LLM_CACHE=1` default; `EDGEQUAKE_KEYWORD_CACHE` / `EDGEQUAKE_QUERY_ANSWER_CACHE` overrides). No SDK request/response field changes — the server returns cached answers transparently. Set flags on the **server**; Acc runs pin cache off for fair peers.

## Go / Java publish honesty

| SDK | Consume today | Published registry |
|-----|---------------|-------------------|
| **Go** | Monorepo `replace` / `go.mod` path: `github.com/edgequake/edgequake-go` | **Not** on pkg.go.dev from this repo yet — import path is aspirational; use local path. |
| **Java** | Maven Central `io.edgequake:edgequake-sdk:~0.4.0` | Yes — tags `sdk-java-v*` trigger publish workflow. |

## Documentation

- **`docs/sdks/*`**: Example-oriented guides; they do not replace OpenAPI.
- **Spec folder** `specs/009-skd-update/`: Coverage matrices and per-language GAP notes.

## Residual risk (honest)

1. **Field parity:** SDK models may omit v0.23 presentation fields (`display_status`, `ui_phase`); UI may need raw JSON.
2. **Streaming:** Progress and PDF SSE differ per language; expect to read bytes or thin helpers, not rich typed events everywhere.
3. **Tier 2 velocity:** Fixes land after Tier 1 unless someone drives parity explicitly.
4. **Cancel SSOT:** Multiple HTTP paths cancel tasks; SDKs may only wrap one — verify [Ingestion cancel & fairness](../ingestion-cancel-and-fairness.md).
5. **Parse API Tier 2 gap:** `POST /api/v1/parse` is typed in Tier 1 only; Tier 2 must hand-roll multipart until parity lands.

## Verdict

- **Tier 1:** Suitable as **default** for new integrations against v0.23.
- **Tier 2:** Suitable with **spot checks** on the endpoints you care about.
- **Ruby / PHP:** Experimental — path/monorepo install; confirm CI green before betting a product on them.
