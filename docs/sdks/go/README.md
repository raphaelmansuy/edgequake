---
title: Go SDK
description: Use the EdgeQuake Go client from the monorepo module github.com/edgequake/edgequake-go. Not on the Go module proxy; install from a local checkout.
---

# Go SDK

The Go client lives in `sdks/go` and uses the module path `github.com/edgequake/edgequake-go` with Go 1.21+. The module is **not on the public Go module proxy**, so `go get github.com/edgequake/edgequake-go` fails. Install it from a local checkout with a `replace` directive.

## Install from a checkout

```bash
# Run from your module directory. Replace the path with your checkout.
go mod edit \
  -require=github.com/edgequake/edgequake-go@v0.0.0 \
  -replace=github.com/edgequake/edgequake-go=/path/to/edgequake/sdks/go
go mod tidy
```

```mermaid
%%{init: {"theme":"base","themeVariables":{"primaryColor":"#E0E7FF","primaryBorderColor":"#6366F1","primaryTextColor":"#1E1B4B","secondaryColor":"#D1FAE5","secondaryBorderColor":"#10B981","secondaryTextColor":"#064E3B","tertiaryColor":"#FEF3C7","tertiaryBorderColor":"#F59E0B","tertiaryTextColor":"#6B7A90","lineColor":"#7A889C","clusterBkg":"rgba(99,102,241,0.07)","clusterBorder":"#A5B4FC","noteBkgColor":"#FEF9C3","noteTextColor":"#422006","textColor":"#6B7A90","titleColor":"#6B7A90","signalColor":"#7A889C","signalTextColor":"#6B7A90","loopTextColor":"#6B7A90","edgeLabelBackground":"#F1F5F9","actorLineColor":"#94A3B8"}}}%%
%% eq-theme:v1
flowchart LR
    App["Your Go module"] -->|"go mod edit -replace"| Local["sdks/go in the checkout"]
    App -.->|"go get (fails: not on proxy)"| Proxy["Go module proxy"]
%% eq-classes
classDef eqBad fill:#FEE2E2,stroke:#EF4444,color:#7F1D1D
class get eqBad
```

The solid path works. The dotted path is what the old install instructions suggested, and it returns a 404.

## Example

```go
package main

import (
    "context"
    "fmt"

    "github.com/edgequake/edgequake-go"
)

func main() {
    c := edgequake.NewClient(edgequake.WithBaseURL("http://localhost:8080"))
    docs, err := c.Documents.List(context.Background(), 1, 20)
    if err != nil {
        panic(err)
    }
    fmt.Println(len(docs.Documents))

    ans, err := c.Query.Execute(context.Background(), &edgequake.QueryRequest{
        Query: "What is in my documents?",
        Mode:  "mix",
    })
    if err != nil {
        panic(err)
    }
    fmt.Println(ans.Answer)
}
```

## Known issues

| Issue | Effect | Workaround |
|-------|--------|------------|
| List calls send `per_page` | `Documents.List`, entity, relationship and task lists. The server reads `page_size`, so the page size is ignored. | Call the REST route directly, or patch the query parameter in `sdks/go/services.go` |
| No registry release | Nothing to pin in `go.sum` | Use the local `replace` above |
| Connections not wrapped | No Go method for `/api/v1/connections` | Use raw HTTP ([Connections](../../api-reference/connections.md)) |

Overview of all clients: [SDK overview](../README.md). Gap list: [Brutal assessment](../BRUTAL-ASSESSMENT.md).
