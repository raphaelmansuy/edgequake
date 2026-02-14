# OODA-26 Decide: Swift SDK Lineage Implementation

## Priority Actions

1. Create LineageModels.swift — 19 Codable+Sendable structs with all optional fields
2. Create LineageService.swift — 7 async throws methods with URL encoding
3. Wire LineageService into EdgeQuakeClient (16→17 services)
4. Add convenience methods to ChatService, QueryService, DocumentService, EntityService, ModelService
5. Fix CreateEntityLineageTest JSONDecoder snake_case bug
6. Add 19 LineageService unit tests to LineageTest.swift
7. Update testInitializesAllServices to include conversations, folders, lineage
8. Build and verify all 129+ unit tests pass
