---
title: Go SDK
description: Use the EdgeQuake Go client from the monorepo module github.com/edgequake/edgequake-go. Not yet on pkg.go.dev.
---

# Go SDK

Go client for EdgeQuake. Module: **`github.com/edgequake/edgequake-go`** (Go 1.21+). **Not published** on pkg.go.dev; install from the monorepo or a git replace.

```bash
# From a checkout of this repo:
go get github.com/edgequake/edgequake-go@main
# or replace with ./sdks/go in go.mod
```

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
    if err != nil { panic(err) }
    fmt.Println(len(docs.Documents))

    ans, err := c.Query.Execute(context.Background(), &edgequake.QueryRequest{
        Query: "What is in my documents?",
        Mode:  "mix",
    })
    if err != nil { panic(err) }
    fmt.Println(ans.Answer)
}
```

**Known bug:** `Documents.List` sends query param `per_page`. The API expects `page_size`. Prefer raw HTTP or patch locally until fixed. Connections are not wrapped. See [SDK overview](../README.md).
