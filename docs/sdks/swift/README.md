---
title: Swift SDK
description: Use the EdgeQuake Swift package EdgeQuakeSDK (Swift 5.9+, macOS 13+). Path / SPM install from sdks/swift.
---

# Swift SDK

Swift client for EdgeQuake. Package product: **`EdgeQuakeSDK`**. Requires **Swift 5.9** and **macOS 13+**. Distributed as source under `sdks/swift` (Swift Package Manager path dependency).

```swift
// Package.swift dependency:
// .package(path: "sdks/swift")
import EdgeQuakeSDK

let client = EdgeQuakeClient(config: EdgeQuakeConfig(
    baseUrl: "http://localhost:8080",
    apiKey: "eq-..."
))

let docs = try await client.documents.list(page: 1, pageSize: 20)
let answer = try await client.query.execute(query: "What is in my documents?", mode: "mix")
print(answer.answer)
```

Class: `EdgeQuakeClient` with async services (`documents`, `query`, `lineage`, and others). Connections are not wrapped. See [SDK overview](../README.md).
