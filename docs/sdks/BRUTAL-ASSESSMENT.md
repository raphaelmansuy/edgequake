# Brutal honest SDK assessment

**Date:** 2026-04-24  
**Law:** `edgequake/crates/edgequake-api/src/routes.rs` + generated OpenAPI.

This is not a marketing page. It states what is true in the repo today.

## What “code is law” means here

- **Paths and verbs** must match `routes.rs`. Invented URLs (e.g. old `/costs/workspace`) are bugs, not features.
- **Request bodies** must match handler DTOs (e.g. bulk conversation ops use **`conversation_ids`**, not `ids`).
- **Responses** should deserialize the common JSON the API actually returns (e.g. **`affected`** on bulk ops, paginated **`items`** + **`pagination`** for conversations).

OpenAPI still wins for **every optional field** on large structs; SDKs often trim models for ergonomics.

## Tier 1 (Rust, Python, TypeScript)

| Area | Verdict |
|------|---------|
| Maintenance | Highest; CI and refactors land here first. |
| Conversation list / messages | Query params aligned with API list handlers; paginated wrappers modeled. |
| Document list | **Was** a real gap: nonexistent `status` / `search` query params appeared in some clients. **Fixed** to match `ListDocumentsRequest`: `page`, `page_size`, `date_from`, `date_to`, `document_pattern`. Rust gains `list_with_query`. |
| Costs / pipeline | TypeScript uses shared path constants to avoid drift. |
| Streaming | SSE / WebSocket helpers exist in places but are **not** a unified typed event layer across all three. |

**Bottom line:** Tier 1 is the reference track. If you need predictable behavior, prefer Tier 1.

## Tier 2 (Kotlin, Swift, Go, Java, C#)

| Area | Verdict |
|------|---------|
| Coverage | Broad surface area (documents, graph, chat, costs, etc.) but **less** systematically audited than Tier 1. |
| Conversations bulk delete | **Was wrong** (`ids` + `deleted` / `deleted_count` shapes). **Fixed** in this iteration to `conversation_ids` + `affected` (Java keeps JSON aliases for older mocks). |
| Conversation list filters | Still **thin** in several Tier 2 SDKs (often “list with no query”). Tier 1 is ahead for `filter[…]` / cursor parity. |
| Java message update path | Some methods may still target older path shapes; verify against `routes.rs` before relying on them in production. |

**Bottom line:** Tier 2 is usable and improving, but **you should verify** critical paths against OpenAPI or Tier 1 when stakes are high.

## Ruby / PHP (present in repo)

- **Ruby:** Gem layout is **incomplete** in this tree (`lib/` missing from the snapshot we ship here). Treat as **not production-ready** until the package layout is restored and tested.
- **PHP:** Exists under `sdks/php`; same caveat — confirm CI and parity before betting a product on it.

## Documentation

- **`docs/sdks/*`**: Example-oriented guides; they do not replace OpenAPI.
- **Spec folder** `specs/009-skd-update/`: Coverage matrices and per-language GAP notes.

## Residual risk (honest)

1. **Field parity:** SDK models may omit rarely used fields; upgrades can add them silently in the API.
2. **Streaming:** Progress and PDF SSE differ per language; expect to read bytes or thin helpers, not rich typed events everywhere.
3. **Tier 2 velocity:** Fixes land after Tier 1 unless someone drives parity explicitly.

## Verdict

- **Tier 1:** Suitable as **default** for new integrations; document list and bulk conversation shapes are aligned with `routes.rs` after this pass.
- **Tier 2:** Suitable with **spot checks** on the endpoints you care about; bulk conversation delete/move/archive on JVM / .NET / Go / Kotlin / Swift now send **lawful** JSON keys.
