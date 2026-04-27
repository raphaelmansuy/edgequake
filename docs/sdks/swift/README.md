# Swift SDK

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
