---
title: "Swift SDK"
description: "Swift SDK for EdgeQuake."
---

# Swift SDK

> **Product: v0.23.0** · SDK package **~0.4.0** (decoupled from server)

**Location:** `sdks/swift`

## Add to your app

Use Swift Package Manager and point to the `sdks/swift` folder, or follow `Package.swift` in that directory.

## Example

```swift
import EdgeQuakeSDK

let cfg = EdgeQuakeConfig(
    baseUrl: "http://localhost:8080",
    apiKey: ProcessInfo.processInfo.environment["EDGEQUAKE_API_KEY"],
    tenantId: "…",
    userId: "…",
    workspaceId: "…"
)
let client = EdgeQuakeClient(config: cfg)

let health = try await client.health.check()
print(health.status ?? "")

let convos = try await client.conversations.list()
print(convos.count)

let bulk = try await client.conversations.bulkDelete(ids: ["c1", "c2"])
print(bulk.affected ?? 0)
```

## Tests

```bash
cd sdks/swift && swift test
```

`ConversationService.bulkDelete` sends **`conversation_ids`** in the JSON body.

For v0.23 task cancel and progress WebSocket, spot-check OpenAPI — Tier 1 SDKs lead on typed helpers. See [Ingestion cancel & fairness](../../ingestion-cancel-and-fairness.md).

## v0.23 notes

- Document responses include `display_status` / `ui_phase` (SPEC-057 P4) — prefer them over raw `status`/`stage` for progress UI.
- **Stateless parse (SPEC-094):** no typed wrapper yet — raw HTTP `POST /api/v1/parse` (multipart `file` + `options`; sync ≤ 15 pages / 20 MiB, async ≤ 1000 pages) + `GET /api/v1/parse/backends` + `GET /api/v1/parse/jobs/{id}`. Async responses return `{job_id, status, request_id}`.
- **LLM cache (server-side):** `EDGEQUAKE_LLM_CACHE=1` default caches keywords + answers; `EDGEQUAKE_KEYWORD_CACHE` / `EDGEQUAKE_QUERY_ANSWER_CACHE` override — no client change.
