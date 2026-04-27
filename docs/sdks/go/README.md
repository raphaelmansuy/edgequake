# Go SDK

**Location:** `sdks/go`

## Use as module

```go
import "github.com/edgequake/edgequake-go"
```

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

## Test

```bash
cd sdks/go && go test ./...
```
