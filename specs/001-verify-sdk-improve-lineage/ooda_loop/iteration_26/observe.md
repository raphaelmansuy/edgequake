# OODA-26 Observe: Swift SDK Lineage

## SDK State

- Swift SDK at `sdks/swift/` — Swift Package Manager project
- 16 services wired into EdgeQuakeClient, 0 lineage support
- 129 unit tests passing (UnitTest.swift + LineageTest.swift pre-existing tests for non-lineage services)
- 21 E2E tests (all fail — need live backend, expected)
- Models use `Codable + Sendable`, `AnyCodable` for dynamic JSON
- HttpHelper uses `.convertToSnakeCase`/`.convertFromSnakeCase` key strategies
- MockURLProtocol pattern with `requestHistory` for unit testing

## Files Examined

- `Sources/EdgeQuakeSDK/Models.swift` — 452 lines, all response/request types
- `Sources/EdgeQuakeSDK/Services.swift` — 227 lines, 16 services
- `Sources/EdgeQuakeSDK/EdgeQuakeClient.swift` — 52 lines, wires services
- `Sources/EdgeQuakeSDK/HttpHelper.swift` — get<T>, post<T>, getRaw, postRaw, decodeJSON
- `Tests/EdgeQuakeSDKTests/UnitTest.swift` — 815 lines, MockURLProtocol
- `Tests/EdgeQuakeSDKTests/LineageTest.swift` — 672 lines, lineage tests for Entity/Relationship/Graph/Document/Pipeline/Chat/Query/Cost/Conversation services

## Gaps

- No LineageModels.swift — zero lineage type definitions
- No LineageService.swift — zero lineage API methods
- EdgeQuakeClient missing `lineage` property
- Some service methods missing: ChatService.complete, ChatService.getConversation, etc.
- Pre-existing test bug: CreateEntityLineageTest uses plain JSONDecoder but HttpHelper uses snake_case encoding
