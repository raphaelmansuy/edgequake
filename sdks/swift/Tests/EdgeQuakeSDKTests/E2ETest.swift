import XCTest

@testable import EdgeQuakeSDK

/// End-to-end tests for EdgeQuakeSDK.
/// Requires a running EdgeQuake backend at localhost:8080.
/// Set EDGEQUAKE_BASE_URL to override.
final class E2ETest: XCTestCase {

    var client: EdgeQuakeClient!

    override func setUp() async throws {
        try XCTSkipIf(
            ProcessInfo.processInfo.environment["EDGEQUAKE_E2E"] != "1",
            "Set EDGEQUAKE_E2E=1 with a running EdgeQuake API (see IMPLEMENTATION-PROOF.md).")
        let base =
            ProcessInfo.processInfo.environment["EDGEQUAKE_BASE_URL"] ?? "http://localhost:8080"
        /// WHY: Default tenant/user IDs from database migration — avoids XCTSkip.
        let tenantId =
            ProcessInfo.processInfo.environment["EDGEQUAKE_TENANT_ID"]
                ?? "00000000-0000-0000-0000-000000000002"
        let userId =
            ProcessInfo.processInfo.environment["EDGEQUAKE_USER_ID"]
                ?? "00000000-0000-0000-0000-000000000001"
        let config = EdgeQuakeConfig(
            baseUrl: base,
            tenantId: tenantId,
            userId: userId
        )
        client = EdgeQuakeClient(config: config)
    }

    // MARK: - 1. Health

    func testHealthCheck() async throws {
        let h = try await client.health.check()
        XCTAssertEqual(h.status, "healthy")
        XCTAssertNotNil(h.version)
    }

    // MARK: - 2. Documents

    func testDocumentsListAndUpload() async throws {
        // List
        let list = try await client.documents.list()
        XCTAssertNotNil(list.documents)
        XCTAssertNotNil(list.total)

        // Upload
        let resp = try await client.documents.uploadText(
            title: "Swift SDK Test \(UUID().uuidString.prefix(8))",
            content: "Swift SDK integration test content. Knowledge graphs are powerful."
        )
        XCTAssertNotNil(resp.documentId)
        XCTAssertNotNil(resp.status)
    }

    // MARK: - 3. Graph

    func testGraphGet() async throws {
        let g = try await client.graph.get()
        XCTAssertNotNil(g)
    }

    func testGraphSearch() async throws {
        let r = try await client.graph.search(query: "test")
        XCTAssertNotNil(r)
    }

    // MARK: - 4. Entities CRUD

    func testEntityCrud() async throws {
        let entityName = "SWIFT_TEST_ENTITY_\(UUID().uuidString.prefix(6))"
        let req = CreateEntityRequest(
            entityName: entityName,
            entityType: "TEST",
            description: "Created by Swift E2E",
            sourceId: "swift-e2e"
        )
        let created = try await client.entities.create(req)
        XCTAssertNotNil(created.status)

        // List
        let list = try await client.entities.list()
        XCTAssertNotNil(list.items)

        // Get by name
        let fetched = try await client.entities.get(name: entityName)
        XCTAssertNotNil(fetched)

        // Delete
        let del = try await client.entities.delete(name: entityName)
        XCTAssertNotNil(del.status)
    }

    // MARK: - 5. Relationships

    func testRelationshipsList() async throws {
        let list = try await client.relationships.list()
        XCTAssertNotNil(list.items)
    }

    // MARK: - 6. Query

    func testQuery() async throws {
        let r = try await client.query.execute(
            query: "What is a knowledge graph?",
            mode: "hybrid"
        )
        XCTAssertNotNil(r.answer)
    }

    // MARK: - 7. Chat

    func testChat() async throws {
        do {
            let req = ChatCompletionRequest(message: "What entities exist?")
            let r = try await client.chat.completions(req)
            XCTAssertNotNil(r.content)
        } catch let e as EdgeQuakeError where e.statusCode == 401 || e.statusCode == 403 {
            // Chat may require auth — acceptable
            print("Chat: \(e.statusCode ?? 0) (expected if auth not configured)")
        }
    }

    // MARK: - 8. Tenants

    func testTenantsList() async throws {
        let list = try await client.tenants.list()
        XCTAssertNotNil(list.items)
    }

    // MARK: - 9. Users

    func testUsersList() async throws {
        let list = try await client.users.list()
        XCTAssertNotNil(list.users)
    }

    // MARK: - 10. API Keys

    func testApiKeysList() async throws {
        let list = try await client.apiKeys.list()
        XCTAssertNotNil(list.keys)
    }

    // MARK: - 11. Tasks

    func testTasksList() async throws {
        let list = try await client.tasks.list()
        XCTAssertNotNil(list.tasks)
    }

    // MARK: - 12. Pipeline Status

    func testPipelineStatus() async throws {
        let st = try await client.pipeline.status()
        XCTAssertNotNil(st.isBusy)
    }

    // MARK: - 13. Pipeline / Queue Metrics

    func testQueueMetrics() async throws {
        let m = try await client.pipeline.queueMetrics()
        XCTAssertNotNil(m.pendingCount)
        XCTAssertNotNil(m.activeWorkers)
    }

    // MARK: - 14. Models Catalog

    func testModelsCatalog() async throws {
        let catalog = try await client.models.catalog()
        XCTAssertNotNil(catalog.providers, "models catalog providers should not be nil")
    }

    // MARK: - 15. Models Health

    func testModelsHealth() async throws {
        let items = try await client.models.health()
        XCTAssertFalse(items.isEmpty, "models health should not be empty")
    }

    // MARK: - 16. Provider Status

    func testProviderStatus() async throws {
        let ps = try await client.models.providerStatus()
        XCTAssertNotNil(ps.provider)
    }

    // MARK: - 17. Conversations CRUD

    func testConversationsCRUD() async throws {
        // Create
        let conv = try await client.conversations.create(title: "Swift E2E Test \(UUID().uuidString.prefix(8))")
        XCTAssertNotNil(conv.id)
        print("Created conversation: \(conv.id ?? "nil") title=\(conv.title ?? "nil")")

        // List
        let convos = try await client.conversations.list()
        XCTAssertFalse(convos.isEmpty)
        print("Conversations: \(convos.count)")

        // Get detail
        let detail = try await client.conversations.get(id: conv.id!)
        XCTAssertNotNil(detail.conversation)
        XCTAssertEqual(detail.id, conv.id)

        // Delete (204 No Content)
        try await client.conversations.delete(id: conv.id!)
    }

    // MARK: - 18. Folders CRUD

    func testFoldersCRUD() async throws {
        // Create
        let folder = try await client.folders.create(name: "Swift E2E Folder \(UUID().uuidString.prefix(8))")
        XCTAssertNotNil(folder.id)
        print("Created folder: \(folder.id ?? "nil") name=\(folder.name ?? "nil")")

        // List
        let folders = try await client.folders.list()
        XCTAssertFalse(folders.isEmpty)
        print("Folders: \(folders.count)")

        // Delete (204 No Content)
        try await client.folders.delete(id: folder.id!)
    }

    // MARK: - 19. Costs

    func testCostsSummary() async throws {
        let c = try await client.costs.summary()
        XCTAssertNotNil(c)
    }

    // MARK: - 20. Full Workflow

    func testFullWorkflow() async throws {
        // Upload document
        let doc = try await client.documents.uploadText(
            title: "Swift Workflow \(UUID().uuidString.prefix(8))",
            content: "Knowledge graphs connect entities through relationships for better retrieval."
        )
        XCTAssertNotNil(doc.documentId)

        // Query
        let qr = try await client.query.execute(
            query: "What do knowledge graphs connect?",
            mode: "hybrid"
        )
        XCTAssertNotNil(qr.answer)

        // Verify pipeline status
        let ps = try await client.pipeline.status()
        XCTAssertNotNil(ps.isBusy)
    }
}
