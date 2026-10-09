---
title: "C# / .NET SDK"
description: "C# / .NET SDK for EdgeQuake."
---

# C# / .NET SDK

> **Product: v0.23.0** · SDK package **~0.4.0** (decoupled from server)

**Location:** `sdks/csharp`

## Example

```csharp
using EdgeQuakeSDK;

var http = new HttpHelper(new EdgeQuakeConfig {
    BaseUrl = "http://localhost:8080",
    ApiKey = Environment.GetEnvironmentVariable("EDGEQUAKE_API_KEY"),
    TenantId = Environment.GetEnvironmentVariable("EDGEQUAKE_TENANT_ID"),
    UserId = Environment.GetEnvironmentVariable("EDGEQUAKE_USER_ID"),
    WorkspaceId = Environment.GetEnvironmentVariable("EDGEQUAKE_WORKSPACE_ID"),
});

var health = await new HealthService(http).CheckAsync();
Console.WriteLine(health.Status);

var bulk = await new ConversationService(http).BulkDeleteAsync(new List<string> { "c1", "c2" });
Console.WriteLine(bulk.Affected);
```

`BulkDeleteAsync` posts a body with **`conversation_ids`**.

Task cancel and PDF progress: verify against OpenAPI or use raw HTTP — see [Ingestion cancel & fairness](../../ingestion-cancel-and-fairness.md).

## v0.23 notes

- Document responses include `display_status` / `ui_phase` (SPEC-057 P4) — prefer them over raw `status`/`stage` for progress UI.
- **Stateless parse (SPEC-094):** no typed wrapper yet — raw HTTP `POST /api/v1/parse` (multipart `file` + `options`; sync ≤ 15 pages / 20 MiB, async ≤ 1000 pages) + `GET /api/v1/parse/backends` + `GET /api/v1/parse/jobs/{id}`. Async responses return `{job_id, status, request_id}`.
- **LLM cache (server-side):** `EDGEQUAKE_LLM_CACHE=1` default; `EDGEQUAKE_KEYWORD_CACHE` / `EDGEQUAKE_QUERY_ANSWER_CACHE` override — no client change.

## Test

```bash
cd sdks/csharp && dotnet test
```
