---
title: "Go SDK"
description: "Go SDK for EdgeQuake."
---

# Go SDK

> **Product: v0.23.0** · SDK package: **~0.4.0** (decoupled from server)

**Location:** `sdks/go`

## Install honesty

The module path is `github.com/edgequake/edgequake-go`, but **this repo does not publish to pkg.go.dev yet**. Use a monorepo path in your `go.mod`:

```go
replace github.com/edgequake/edgequake-go => ../sdks/go
```

Or vendor `sdks/go` directly. Do not assume `go get github.com/edgequake/edgequake-go` resolves until a publish workflow exists.

## Example

```go
ctx := context.Background()
c := edgequake.NewClient(
    edgequake.WithBaseURL("http://localhost:8080"),
    edgequake.WithAPIKey(os.Getenv("EDGEQUAKE_API_KEY")),
    edgequake.WithTenantID(os.Getenv("EDGEQUAKE_TENANT_ID")),
    edgequake.WithUserID(os.Getenv("EDGEQUAKE_USER_ID")),
    edgequake.WithWorkspaceID(os.Getenv("EDGEQUAKE_WORKSPACE_ID")),
)

h, err := c.Health.Check(ctx)
if err != nil { log.Fatal(err) }
log.Println(h.Status)

out, err := c.Conversations.BulkDelete(ctx, []string{"c1", "c2"})
if err != nil { log.Fatal(err) }
log.Println(out.Affected)
```

`BulkDelete` sends `conversation_ids` in the POST body.

## v0.23 notes

- Task cancel: `c.Tasks.Cancel(ctx, trackID)` — verify against [Ingestion cancel & fairness](../../ingestion-cancel-and-fairness.md).
- PDF progress SSE and `display_status` / `ui_phase` fields may require raw JSON decoding; Tier 1 SDKs lead on typed helpers.
- **Stateless parse (SPEC-094):** no typed `parse` resource yet — use raw HTTP `POST /api/v1/parse` (multipart `file` + `options` JSON; sync ≤ 15 pages / 20 MiB, async ≤ 1000 pages), `GET /api/v1/parse/backends`, and `GET /api/v1/parse/jobs/{id}`. Response shapes: `ParseResponse` (`markdown`, `page_count`, `metrics.total_ms`, …), `ParseAsyncAccepted` (`job_id`), `ParseJobStatusResponse` (`status`, `result`, `error`).
- **LLM cache (server-side):** `EDGEQUAKE_LLM_CACHE=1` default caches keywords + answers; `EDGEQUAKE_KEYWORD_CACHE` / `EDGEQUAKE_QUERY_ANSWER_CACHE` override. No client change — set flags on the server.

## Test

```bash
cd sdks/go && go test ./...
```

## See also

- In-repo reference: `sdks/go/README.md`
- [Brutal assessment](../BRUTAL-ASSESSMENT.md)
