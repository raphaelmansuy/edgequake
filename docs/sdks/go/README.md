---
title: "Go SDK"
---

# Go SDK

> **Product: v0.19.0** · SDK package: **~0.4.0** (decoupled from server)

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

## v0.19 notes

- Task cancel: `c.Tasks.Cancel(ctx, trackID)` — verify against [Ingestion cancel & fairness](../../ingestion-cancel-and-fairness.md).
- PDF progress SSE and `display_status` fields may require raw HTTP; Tier 1 SDKs lead on typed helpers.

## Test

```bash
cd sdks/go && go test ./...
```

## See also

- In-repo reference: `sdks/go/README.md`
- [Brutal assessment](../BRUTAL-ASSESSMENT.md)
