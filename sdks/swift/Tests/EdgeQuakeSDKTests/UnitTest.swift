import Foundation
import XCTest

@testable import EdgeQuakeSDK

// MARK: - MockURLProtocol

/// Mock URL protocol that returns predefined responses without network calls.
/// WHY: Enables stateless unit testing of all service methods.
final class MockURLProtocol: URLProtocol {
    static var responseData: Data = "{}".data(using: .utf8)!
    static var responseStatusCode: Int = 200
    static var requestHistory: [(method: String, url: String, body: Data?)] = []

    override class func canInit(with request: URLRequest) -> Bool { true }
    override class func canonicalRequest(for request: URLRequest) -> URLRequest { request }

    override func startLoading() {
        let method = request.httpMethod ?? "GET"
        let url = request.url?.absoluteString ?? ""
        // WHY: URLProtocol strips httpBody; read from httpBodyStream instead
        var body = request.httpBody
        if body == nil, let stream = request.httpBodyStream {
            stream.open()
            var data = Data()
            let bufferSize = 4096
            let buffer = UnsafeMutablePointer<UInt8>.allocate(capacity: bufferSize)
            defer { buffer.deallocate() }
            while stream.hasBytesAvailable {
                let read = stream.read(buffer, maxLength: bufferSize)
                if read > 0 {
                    data.append(buffer, count: read)
                } else {
                    break
                }
            }
            stream.close()
            body = data.isEmpty ? nil : data
        }
        MockURLProtocol.requestHistory.append((method: method, url: url, body: body))

        let response = HTTPURLResponse(
            url: request.url!, statusCode: MockURLProtocol.responseStatusCode,
            httpVersion: "HTTP/1.1", headerFields: ["Content-Type": "application/json"]
        )!
        client?.urlProtocol(self, didReceive: response, cacheStoragePolicy: .notAllowed)
        client?.urlProtocol(self, didLoad: MockURLProtocol.responseData)
        client?.urlProtocolDidFinishLoading(self)
    }

    override func stopLoading() {}

    static func reset(json: String = "{}", status: Int = 200) {
        responseData = json.data(using: .utf8)!
        responseStatusCode = status
        requestHistory = []
    }

    static var lastRequest: (method: String, url: String, body: Data?)? {
        requestHistory.last
    }
}

// MARK: - Test Helpers

func mockHelper(json: String = "{}", status: Int = 200) -> HttpHelper {
    MockURLProtocol.reset(json: json, status: status)
    let config = URLSessionConfiguration.ephemeral
    config.protocolClasses = [MockURLProtocol.self]
    let session = URLSession(configuration: config)
    return HttpHelper(config: EdgeQuakeConfig(), session: session)
}

// MARK: - Config Tests

final class ConfigTest: XCTestCase {
    func testDefaults() {
        let c = EdgeQuakeConfig()
        XCTAssertEqual(c.baseUrl, "http://localhost:8080")
        XCTAssertNil(c.apiKey)
        XCTAssertNil(c.tenantId)
        XCTAssertNil(c.userId)
        XCTAssertNil(c.workspaceId)
        XCTAssertEqual(c.timeoutSeconds, 30)
    }

    func testCustomValues() {
        let c = EdgeQuakeConfig(
            baseUrl: "https://api.example.com", apiKey: "sk-test",
            tenantId: "t-1", userId: "u-1", workspaceId: "ws-1", timeoutSeconds: 120
        )
        XCTAssertEqual(c.baseUrl, "https://api.example.com")
        XCTAssertEqual(c.apiKey, "sk-test")
        XCTAssertEqual(c.tenantId, "t-1")
        XCTAssertEqual(c.userId, "u-1")
        XCTAssertEqual(c.workspaceId, "ws-1")
        XCTAssertEqual(c.timeoutSeconds, 120)
    }
}

// MARK: - Error Tests

final class ErrorTest: XCTestCase {
    func testProperties() {
        let err = EdgeQuakeError(message: "bad request", statusCode: 400, responseBody: "{}")
        XCTAssertEqual(err.message, "bad request")
        XCTAssertEqual(err.statusCode, 400)
        XCTAssertEqual(err.responseBody, "{}")
        XCTAssertEqual(err.errorDescription, "bad request")
    }

    func testIsError() {
        let err = EdgeQuakeError(message: "test")
        XCTAssertTrue(err is Error)
    }

    func testDefaults() {
        let err = EdgeQuakeError(message: "test")
        XCTAssertEqual(err.statusCode, 0)
        XCTAssertNil(err.responseBody)
    }
}

// MARK: - Client Tests

final class ClientTest: XCTestCase {
    func testInitializesAllServices() {
        let client = EdgeQuakeClient()
        XCTAssertNotNil(client.health)
        XCTAssertNotNil(client.documents)
        XCTAssertNotNil(client.entities)
        XCTAssertNotNil(client.relationships)
        XCTAssertNotNil(client.graph)
        XCTAssertNotNil(client.query)
        XCTAssertNotNil(client.chat)
        XCTAssertNotNil(client.tenants)
        XCTAssertNotNil(client.users)
        XCTAssertNotNil(client.apiKeys)
        XCTAssertNotNil(client.tasks)
        XCTAssertNotNil(client.pipeline)
        XCTAssertNotNil(client.models)
        XCTAssertNotNil(client.costs)
        XCTAssertNotNil(client.conversations)
        XCTAssertNotNil(client.folders)
        XCTAssertNotNil(client.lineage)
    }
}

// MARK: - Health Tests

final class HealthServiceTest: XCTestCase {
    func testCheck() async throws {
        let http = mockHelper(json: #"{"status":"healthy","version":"0.1.0"}"#)
        let svc = HealthService(http)
        let result = try await svc.check()
        XCTAssertEqual(result.status, "healthy")
        XCTAssertEqual(result.version, "0.1.0")
        XCTAssertEqual(MockURLProtocol.lastRequest?.method, "GET")
        XCTAssertTrue(MockURLProtocol.lastRequest!.url.contains("/health"))
    }

    func testCheckError() async {
        let http = mockHelper(json: "{}", status: 500)
        let svc = HealthService(http)
        do {
            _ = try await svc.check()
            XCTFail("Expected error")
        } catch {
            if let err = error as? EdgeQuakeError {
                XCTAssertEqual(err.statusCode, 500)
            }
        }
    }
}

// MARK: - Document Tests

final class DocumentServiceTest: XCTestCase {
    func testList() async throws {
        let http = mockHelper(json: #"{"documents":[{"id":"d1","title":"Test"}],"total":1}"#)
        let svc = DocumentService(http)
        let result = try await svc.list()
        XCTAssertEqual(result.documents?.count, 1)
        XCTAssertTrue(MockURLProtocol.lastRequest!.url.contains("page=1"))
        XCTAssertTrue(MockURLProtocol.lastRequest!.url.contains("page_size=20"))
    }

    func testListPagination() async throws {
        let http = mockHelper(json: #"{"documents":[]}"#)
        let svc = DocumentService(http)
        _ = try await svc.list(page: 3, pageSize: 50)
        XCTAssertTrue(MockURLProtocol.lastRequest!.url.contains("page=3"))
        XCTAssertTrue(MockURLProtocol.lastRequest!.url.contains("page_size=50"))
    }

    func testGet() async throws {
        let http = mockHelper(json: #"{"id":"d1","title":"Test"}"#)
        let svc = DocumentService(http)
        let result = try await svc.get(id: "d1")
        XCTAssertEqual(result.id, "d1")
        XCTAssertTrue(MockURLProtocol.lastRequest!.url.contains("/api/v1/documents/d1"))
    }

    func testUploadText() async throws {
        let http = mockHelper(json: #"{"document_id":"d2","status":"processing"}"#)
        let svc = DocumentService(http)
        let result = try await svc.uploadText(title: "My Title", content: "Hello World")
        XCTAssertEqual(result.documentId, "d2")
        XCTAssertEqual(MockURLProtocol.lastRequest?.method, "POST")
    }

    func testDelete() async throws {
        let http = mockHelper(json: #"{"status":"deleted"}"#)
        let svc = DocumentService(http)
        _ = try await svc.delete(id: "d1")
        XCTAssertEqual(MockURLProtocol.lastRequest?.method, "DELETE")
        XCTAssertTrue(MockURLProtocol.lastRequest!.url.contains("/api/v1/documents/d1"))
    }

    func testListError() async {
        let http = mockHelper(json: "{}", status: 500)
        let svc = DocumentService(http)
        do {
            _ = try await svc.list()
            XCTFail("Expected error")
        } catch {
            // OK
        }
    }
}

// MARK: - Entity Tests

final class EntityServiceTest: XCTestCase {
    func testList() async throws {
        let http = mockHelper(json: #"{"items":[{"entity_name":"ALICE"}],"total":1}"#)
        let svc = EntityService(http)
        let result = try await svc.list()
        XCTAssertEqual(result.items?.count, 1)
    }

    func testGet() async throws {
        let http = mockHelper(json: #"{"entity":{"entity_name":"ALICE"}}"#)
        let svc = EntityService(http)
        _ = try await svc.get(name: "ALICE")
        XCTAssertTrue(MockURLProtocol.lastRequest!.url.contains("/api/v1/graph/entities/ALICE"))
    }

    func testCreate() async throws {
        let http = mockHelper(json: #"{"status":"success"}"#)
        let svc = EntityService(http)
        let req = CreateEntityRequest(
            entityName: "BOB", entityType: "person", description: "A person", sourceId: "src-1")
        let result = try await svc.create(req)
        XCTAssertEqual(result.status, "success")
        XCTAssertEqual(MockURLProtocol.lastRequest?.method, "POST")
    }

    func testDelete() async throws {
        let http = mockHelper(json: #"{"status":"deleted"}"#)
        let svc = EntityService(http)
        _ = try await svc.delete(name: "BOB")
        XCTAssertEqual(MockURLProtocol.lastRequest?.method, "DELETE")
        XCTAssertTrue(MockURLProtocol.lastRequest!.url.contains("confirm=true"))
    }

    func testExists() async throws {
        let http = mockHelper(json: #"{"exists":true}"#)
        let svc = EntityService(http)
        let result = try await svc.exists(name: "ALICE")
        XCTAssertEqual(result.exists, true)
    }

    func testListError() async {
        let http = mockHelper(json: "{}", status: 500)
        let svc = EntityService(http)
        do {
            _ = try await svc.list()
            XCTFail("Expected error")
        } catch {
            // OK
        }
    }
}

// MARK: - Relationship Tests

final class RelationshipServiceTest: XCTestCase {
    func testList() async throws {
        let http = mockHelper(json: #"{"items":[{"source":"A","target":"B"}],"total":1}"#)
        let svc = RelationshipService(http)
        let result = try await svc.list()
        XCTAssertEqual(result.items?.count, 1)
    }

    func testListPagination() async throws {
        let http = mockHelper(json: #"{"items":[]}"#)
        let svc = RelationshipService(http)
        _ = try await svc.list(page: 2, pageSize: 10)
        XCTAssertTrue(MockURLProtocol.lastRequest!.url.contains("page=2"))
    }

    func testListError() async {
        let http = mockHelper(json: "{}", status: 500)
        let svc = RelationshipService(http)
        do {
            _ = try await svc.list()
            XCTFail("Expected error")
        } catch {
            // OK
        }
    }
}

// MARK: - Graph Tests

final class GraphServiceTest: XCTestCase {
    func testGet() async throws {
        let http = mockHelper(json: #"{"nodes":[],"edges":[]}"#)
        let svc = GraphService(http)
        let result = try await svc.get()
        XCTAssertNotNil(result.nodes)
    }

    func testSearch() async throws {
        let http = mockHelper(json: #"{"nodes":[{"id":"n1"}]}"#)
        let svc = GraphService(http)
        let result = try await svc.search(query: "Alice")
        XCTAssertEqual(result.nodes?.count, 1)
        XCTAssertTrue(MockURLProtocol.lastRequest!.url.contains("q=Alice"))
    }

    func testGetError() async {
        let http = mockHelper(json: "{}", status: 500)
        let svc = GraphService(http)
        do {
            _ = try await svc.get()
            XCTFail("Expected error")
        } catch {
            // OK
        }
    }
}

// MARK: - Query Tests

final class QueryServiceTest: XCTestCase {
    func testExecute() async throws {
        let http = mockHelper(json: #"{"answer":"42","sources":[]}"#)
        let svc = QueryService(http)
        let result = try await svc.execute(query: "meaning of life")
        XCTAssertEqual(result.answer, "42")
        XCTAssertEqual(MockURLProtocol.lastRequest?.method, "POST")
    }

    func testExecuteWithMode() async throws {
        let http = mockHelper(json: #"{"answer":"yes"}"#)
        let svc = QueryService(http)
        _ = try await svc.execute(query: "test", mode: "local")
        guard let body = MockURLProtocol.lastRequest?.body,
            let str = String(data: body, encoding: .utf8)
        else {
            XCTFail("No body")
            return
        }
        XCTAssertTrue(str.contains("local"))
    }

    func testExecuteError() async {
        let http = mockHelper(json: "{}", status: 500)
        let svc = QueryService(http)
        do {
            _ = try await svc.execute(query: "test")
            XCTFail("Expected error")
        } catch {
            // OK
        }
    }
}

// MARK: - Chat Tests

final class ChatServiceTest: XCTestCase {
    func testCompletions() async throws {
        let http = mockHelper(json: #"{"content":"Hello!"}"#)
        let svc = ChatService(http)
        let req = ChatCompletionRequest(message: "Hi")
        let result = try await svc.completions(req)
        XCTAssertEqual(result.content, "Hello!")
        XCTAssertEqual(MockURLProtocol.lastRequest?.method, "POST")
    }

    func testCompletionsError() async {
        let http = mockHelper(json: "{}", status: 500)
        let svc = ChatService(http)
        let req = ChatCompletionRequest(message: "test")
        do {
            _ = try await svc.completions(req)
            XCTFail("Expected error")
        } catch {
            // OK
        }
    }
}

// MARK: - Tenant Tests

final class TenantServiceTest: XCTestCase {
    func testList() async throws {
        let http = mockHelper(json: #"{"items":[{"id":"t1"}]}"#)
        let svc = TenantService(http)
        let result = try await svc.list()
        XCTAssertEqual(result.items?.count, 1)
    }

    func testListError() async {
        let http = mockHelper(json: "{}", status: 500)
        let svc = TenantService(http)
        do {
            _ = try await svc.list()
            XCTFail("Expected error")
        } catch {
            // OK
        }
    }
}

// MARK: - User Tests

final class UserServiceTest: XCTestCase {
    func testList() async throws {
        let http = mockHelper(json: #"{"users":[{"id":"u1"}]}"#)
        let svc = UserService(http)
        let result = try await svc.list()
        XCTAssertEqual(result.users?.count, 1)
    }

    func testListError() async {
        let http = mockHelper(json: "{}", status: 500)
        let svc = UserService(http)
        do {
            _ = try await svc.list()
            XCTFail("Expected error")
        } catch {
            // OK
        }
    }
}

// MARK: - API Key Tests

final class ApiKeyServiceTest: XCTestCase {
    func testList() async throws {
        let http = mockHelper(json: #"{"keys":[{"id":"ak-1"}]}"#)
        let svc = ApiKeyService(http)
        let result = try await svc.list()
        XCTAssertEqual(result.keys?.count, 1)
    }

    func testListError() async {
        let http = mockHelper(json: "{}", status: 500)
        let svc = ApiKeyService(http)
        do {
            _ = try await svc.list()
            XCTFail("Expected error")
        } catch {
            // OK
        }
    }
}

// MARK: - Task Tests

final class TaskServiceTest: XCTestCase {
    func testList() async throws {
        let http = mockHelper(json: #"{"tasks":[{"track_id":"trk-1"}]}"#)
        let svc = TaskService(http)
        let result = try await svc.list()
        XCTAssertEqual(result.tasks?.count, 1)
    }

    func testListError() async {
        let http = mockHelper(json: "{}", status: 500)
        let svc = TaskService(http)
        do {
            _ = try await svc.list()
            XCTFail("Expected error")
        } catch {
            // OK
        }
    }
}

// MARK: - Pipeline Tests

final class PipelineServiceTest: XCTestCase {
    func testStatus() async throws {
        let http = mockHelper(json: #"{"is_busy":true,"pending_tasks":5}"#)
        let svc = PipelineService(http)
        let result = try await svc.status()
        XCTAssertEqual(result.isBusy, true)
    }

    func testQueueMetrics() async throws {
        let http = mockHelper(json: #"{"pending_count":10}"#)
        let svc = PipelineService(http)
        let result = try await svc.queueMetrics()
        XCTAssertEqual(result.pendingCount, 10)
    }

    func testStatusError() async {
        let http = mockHelper(json: "{}", status: 500)
        let svc = PipelineService(http)
        do {
            _ = try await svc.status()
            XCTFail("Expected error")
        } catch {
            // OK
        }
    }
}

// MARK: - Model Tests

final class ModelServiceTest: XCTestCase {
    func testCatalog() async throws {
        let http = mockHelper(json: #"{"providers":[{"name":"openai"}]}"#)
        let svc = ModelService(http)
        let result = try await svc.catalog()
        XCTAssertEqual(result.providers?.count, 1)
    }

    func testHealth() async throws {
        let http = mockHelper(json: #"[{"name":"ollama","enabled":true}]"#)
        let svc = ModelService(http)
        let result = try await svc.health()
        XCTAssertEqual(result.count, 1)
        XCTAssertEqual(result[0].name, "ollama")
    }

    func testProviderStatus() async throws {
        let http = mockHelper(json: #"{"provider":{"name":"ollama"}}"#)
        let svc = ModelService(http)
        let result = try await svc.providerStatus()
        XCTAssertNotNil(result.provider)
    }

    func testCatalogError() async {
        let http = mockHelper(json: "{}", status: 500)
        let svc = ModelService(http)
        do {
            _ = try await svc.catalog()
            XCTFail("Expected error")
        } catch {
            // OK
        }
    }
}

// MARK: - Cost Tests

final class CostServiceTest: XCTestCase {
    func testSummary() async throws {
        let http = mockHelper(json: #"{"total_cost":12.5}"#)
        let svc = CostService(http)
        let result = try await svc.summary()
        XCTAssertEqual(result.totalCost, 12.5)
    }

    func testSummaryError() async {
        let http = mockHelper(json: "{}", status: 500)
        let svc = CostService(http)
        do {
            _ = try await svc.summary()
            XCTFail("Expected error")
        } catch {
            // OK
        }
    }
}

// MARK: - Mock Tests

final class MockTests: XCTestCase {
    func testTracksAllCalls() async throws {
        let http = mockHelper(json: #"{"status":"healthy"}"#)
        let svc = HealthService(http)
        _ = try await svc.check()
        _ = try await svc.check()
        XCTAssertEqual(MockURLProtocol.requestHistory.count, 2)
    }
}

// MARK: - Conversation Tests

final class ConversationServiceTest: XCTestCase {
    func testList() async throws {
        let http = mockHelper(json: #"{"items":[{"id":"c1","title":"Test Chat"}]}"#)
        let svc = ConversationService(http)
        let result = try await svc.list()
        XCTAssertEqual(result.count, 1)
        XCTAssertEqual(result[0].id, "c1")
    }

    func testListEmpty() async throws {
        let http = mockHelper(json: #"{"items":[]}"#)
        let svc = ConversationService(http)
        let result = try await svc.list()
        XCTAssertEqual(result.count, 0)
    }

    func testListNullItems() async throws {
        let http = mockHelper(json: #"{}"#)
        let svc = ConversationService(http)
        let result = try await svc.list()
        XCTAssertEqual(result.count, 0)
    }

    func testCreate() async throws {
        let http = mockHelper(json: #"{"id":"c2","title":"New Chat"}"#)
        let svc = ConversationService(http)
        let result = try await svc.create(title: "New Chat")
        XCTAssertEqual(result.id, "c2")
        XCTAssertEqual(result.title, "New Chat")
        let last = MockURLProtocol.lastRequest
        XCTAssertEqual(last?.method, "POST")
    }

    func testGet() async throws {
        let http = mockHelper(json: #"{"conversation":{"id":"c1","title":"Test"},"messages":[]}"#)
        let svc = ConversationService(http)
        let result = try await svc.get(id: "c1")
        XCTAssertNotNil(result.conversation)
        let last = MockURLProtocol.lastRequest
        XCTAssertTrue(last!.url.contains("/api/v1/conversations/c1"))
    }

    func testDelete() async throws {
        let http = mockHelper(json: #"{}"#)
        let svc = ConversationService(http)
        try await svc.delete(id: "c1")
        let last = MockURLProtocol.lastRequest
        XCTAssertEqual(last?.method, "DELETE")
        XCTAssertTrue(last!.url.contains("/api/v1/conversations/c1"))
    }

    func testBulkDelete() async throws {
        let http = mockHelper(json: #"{"deleted":3,"status":"ok"}"#)
        let svc = ConversationService(http)
        let result = try await svc.bulkDelete(ids: ["c1", "c2", "c3"])
        XCTAssertEqual(result.deleted, 3)
        let last = MockURLProtocol.lastRequest
        XCTAssertEqual(last?.method, "POST")
        XCTAssertTrue(last!.url.contains("bulk/delete"))
    }

    func testListError() async {
        let http = mockHelper(json: "{}", status: 500)
        let svc = ConversationService(http)
        do {
            _ = try await svc.list()
            XCTFail("Expected error")
        } catch {
            // Expected
        }
    }
}

// MARK: - Folder Tests

final class FolderServiceTest: XCTestCase {
    func testList() async throws {
        let http = mockHelper(json: #"[{"id":"f1","name":"Research"}]"#)
        let svc = FolderService(http)
        let result = try await svc.list()
        XCTAssertEqual(result.count, 1)
        XCTAssertEqual(result[0].name, "Research")
    }

    func testCreate() async throws {
        let http = mockHelper(json: #"{"id":"f2","name":"New Folder"}"#)
        let svc = FolderService(http)
        let result = try await svc.create(name: "New Folder")
        XCTAssertEqual(result.id, "f2")
        XCTAssertEqual(result.name, "New Folder")
        let last = MockURLProtocol.lastRequest
        XCTAssertEqual(last?.method, "POST")
    }

    func testDelete() async throws {
        let http = mockHelper(json: #"{}"#)
        let svc = FolderService(http)
        try await svc.delete(id: "f1")
        let last = MockURLProtocol.lastRequest
        XCTAssertEqual(last?.method, "DELETE")
        XCTAssertTrue(last!.url.contains("/api/v1/folders/f1"))
    }

    func testListError() async {
        let http = mockHelper(json: "{}", status: 500)
        let svc = FolderService(http)
        do {
            _ = try await svc.list()
            XCTFail("Expected error")
        } catch {
            // Expected
        }
    }
}

// MARK: - URL Validation Tests

final class URLValidationTests: XCTestCase {
    func testHealthUrl() async throws {
        let http = mockHelper(json: #"{"status":"ok"}"#)
        _ = try await HealthService(http).check()
        XCTAssertTrue(MockURLProtocol.lastRequest!.url.hasSuffix("/health"))
    }

    func testTasksUrl() async throws {
        let http = mockHelper(json: #"{"tasks":[]}"#)
        _ = try await TaskService(http).list()
        XCTAssertTrue(MockURLProtocol.lastRequest!.url.contains("/api/v1/tasks"))
    }

    func testPipelineStatusUrl() async throws {
        let http = mockHelper(json: #"{"is_busy":false}"#)
        _ = try await PipelineService(http).status()
        XCTAssertTrue(MockURLProtocol.lastRequest!.url.contains("/api/v1/pipeline/status"))
    }

    func testCostsSummaryUrl() async throws {
        let http = mockHelper(json: #"{"total_cost":0}"#)
        _ = try await CostService(http).summary()
        XCTAssertTrue(MockURLProtocol.lastRequest!.url.contains("/api/v1/costs/summary"))
    }

    func testModelsUrl() async throws {
        let http = mockHelper(json: #"{"providers":[]}"#)
        _ = try await ModelService(http).catalog()
        XCTAssertTrue(MockURLProtocol.lastRequest!.url.contains("/api/v1/models"))
    }

    func testEntitiesDeleteUrl() async throws {
        let http = mockHelper(json: #"{"status":"deleted"}"#)
        _ = try await EntityService(http).delete(name: "BOB")
        XCTAssertTrue(MockURLProtocol.lastRequest!.url.contains("/api/v1/graph/entities/BOB"))
    }
}

// MARK: - Client Service Availability Tests

final class ClientServiceAvailabilityTest: XCTestCase {
    func testHasConversations() {
        let client = EdgeQuakeClient()
        XCTAssertNotNil(client.conversations)
    }

    func testHasFolders() {
        let client = EdgeQuakeClient()
        XCTAssertNotNil(client.folders)
    }
}

// MARK: - Edge Case Tests

final class EdgeCaseTests: XCTestCase {
    func testQueryDefaultMode() async throws {
        let http = mockHelper(json: #"{"answer":"x"}"#)
        _ = try await QueryService(http).execute(query: "test")
        let body = MockURLProtocol.lastRequest?.body
        if let data = body, let str = String(data: data, encoding: .utf8) {
            XCTAssertTrue(str.contains("hybrid"))
        }
    }

    func testEntityCreateBody() async throws {
        let http = mockHelper(json: #"{"status":"success"}"#)
        let request = CreateEntityRequest(
            entityName: "NODE", entityType: "concept", description: "A concept", sourceId: "src-1")
        _ = try await EntityService(http).create(request)
        let body = MockURLProtocol.lastRequest?.body
        if let data = body, let str = String(data: data, encoding: .utf8) {
            XCTAssertTrue(str.contains("NODE"))
            XCTAssertTrue(str.contains("concept"))
        }
    }

    func testDocumentsListPaginationDefault() async throws {
        let http = mockHelper(json: #"{"documents":[]}"#)
        _ = try await DocumentService(http).list()
        let url = MockURLProtocol.lastRequest!.url
        XCTAssertTrue(url.contains("page=1"))
        XCTAssertTrue(url.contains("page_size=20"))
    }

    func testErrorStatus502() async {
        let http = mockHelper(json: "{}", status: 502)
        let svc = HealthService(http)
        do {
            _ = try await svc.check()
            XCTFail("Expected error")
        } catch {
            // Expected
        }
    }

    func testErrorStatus429() async {
        let http = mockHelper(json: #"{"error":"rate limited"}"#, status: 429)
        let svc = QueryService(http)
        do {
            _ = try await svc.execute(query: "test")
            XCTFail("Expected error")
        } catch {
            // Expected
        }
    }
}

// MARK: - OODA-35: New Service Tests for Enhanced API Coverage

// MARK: - Health Extended Tests

final class HealthExtendedTests: XCTestCase {
    func testReadiness() async throws {
        let http = mockHelper(json: #"{"ready":true,"status":"ok"}"#)
        let res = try await HealthService(http).readiness()
        XCTAssertEqual(res.ready, true)
    }

    func testLiveness() async throws {
        let http = mockHelper(json: #"{"alive":true,"status":"ok"}"#)
        let res = try await HealthService(http).liveness()
        XCTAssertEqual(res.alive, true)
    }

    func testDetailed() async throws {
        let http = mockHelper(json: #"{"status":"healthy","version":"1.0","uptime":3600}"#)
        let res = try await HealthService(http).detailed()
        XCTAssertEqual(res.status, "healthy")
    }
}

// MARK: - Document Extended Tests

final class DocumentExtendedTests: XCTestCase {
    func testUpdate() async throws {
        let http = mockHelper(json: #"{"id":"doc-1","title":"Updated"}"#)
        let res = try await DocumentService(http).update(id: "doc-1", title: "Updated")
        XCTAssertEqual(res.title, "Updated")
        XCTAssertEqual(MockURLProtocol.lastRequest?.method, "PUT")
    }

    func testSearch() async throws {
        let http = mockHelper(json: #"{"documents":[],"total":0}"#)
        _ = try await DocumentService(http).search(query: "test")
        XCTAssertTrue(MockURLProtocol.lastRequest!.url.contains("/search"))
    }

    func testChunks() async throws {
        let http = mockHelper(json: #"{"document_id":"doc-1","chunks":[]}"#)
        _ = try await DocumentService(http).chunks(id: "doc-1")
        XCTAssertTrue(MockURLProtocol.lastRequest!.url.contains("/chunks"))
    }

    func testStatus() async throws {
        let http = mockHelper(json: #"{"id":"doc-1","status":"completed"}"#)
        let res = try await DocumentService(http).status(id: "doc-1")
        XCTAssertEqual(res.status, "completed")
    }

    func testReprocess() async throws {
        let http = mockHelper(json: #"{"document_id":"doc-1","status":"queued"}"#)
        _ = try await DocumentService(http).reprocess(id: "doc-1")
        XCTAssertEqual(MockURLProtocol.lastRequest?.method, "POST")
    }
}

// MARK: - Entity Extended Tests

final class EntityExtendedTests: XCTestCase {
    func testUpdateEntity() async throws {
        let http = mockHelper(json: #"{"entity":{"entity_name":"TEST"}}"#)
        _ = try await EntityService(http).update(name: "TEST", description: "Updated desc")
        XCTAssertEqual(MockURLProtocol.lastRequest?.method, "PUT")
    }

    func testMergeEntities() async throws {
        let http = mockHelper(json: #"{"status":"merged"}"#)
        _ = try await EntityService(http).merge(sourceName: "A", targetName: "B")
        XCTAssertTrue(MockURLProtocol.lastRequest!.url.contains("/merge"))
    }

    func testEntityTypes() async throws {
        let http = mockHelper(json: #"{"types":["PERSON","ORG"]}"#)
        let res = try await EntityService(http).types()
        XCTAssertEqual(res.types, ["PERSON", "ORG"])
    }
}

// MARK: - Relationship Extended Tests

final class RelationshipExtendedTests: XCTestCase {
    func testCreateRelationship() async throws {
        let http = mockHelper(json: #"{"id":"rel-1","source":"A","target":"B"}"#)
        let res = try await RelationshipService(http).create(
            source: "A", target: "B", relationshipType: "KNOWS")
        XCTAssertEqual(res.source, "A")
        XCTAssertEqual(MockURLProtocol.lastRequest?.method, "POST")
    }

    func testDeleteRelationship() async throws {
        let http = mockHelper(json: "{}", status: 204)
        try await RelationshipService(http).delete(id: "rel-1")
        XCTAssertEqual(MockURLProtocol.lastRequest?.method, "DELETE")
    }

    func testRelationshipTypes() async throws {
        let http = mockHelper(json: #"{"types":["KNOWS","WORKS_WITH"]}"#)
        let res = try await RelationshipService(http).types()
        XCTAssertEqual(res.types, ["KNOWS", "WORKS_WITH"])
    }
}

// MARK: - Graph Extended Tests

final class GraphExtendedTests: XCTestCase {
    func testGraphStats() async throws {
        let http = mockHelper(json: #"{"node_count":100,"edge_count":200}"#)
        let res = try await GraphService(http).stats()
        XCTAssertEqual(res.nodeCount, 100)
    }

    func testGraphClear() async throws {
        let http = mockHelper(json: "{}", status: 204)
        try await GraphService(http).clear()
        XCTAssertEqual(MockURLProtocol.lastRequest?.method, "DELETE")
    }

    func testNeighbors() async throws {
        let http = mockHelper(json: #"{"nodes":[],"edges":[]}"#)
        _ = try await GraphService(http).neighbors(name: "TEST", depth: 2)
        XCTAssertTrue(MockURLProtocol.lastRequest!.url.contains("depth=2"))
    }

    func testSubgraph() async throws {
        let http = mockHelper(json: #"{"nodes":[],"edges":[]}"#)
        _ = try await GraphService(http).subgraph(entityNames: ["A", "B"])
        XCTAssertEqual(MockURLProtocol.lastRequest?.method, "POST")
    }
}

// MARK: - Tenant Extended Tests

final class TenantExtendedTests: XCTestCase {
    func testGetTenant() async throws {
        let http = mockHelper(json: #"{"id":"t-1","name":"Test"}"#)
        let res = try await TenantService(http).get(id: "t-1")
        XCTAssertEqual(res.name, "Test")
    }

    func testCreateTenant() async throws {
        let http = mockHelper(json: #"{"id":"t-2","name":"New"}"#)
        let res = try await TenantService(http).create(name: "New")
        XCTAssertEqual(res.name, "New")
        XCTAssertEqual(MockURLProtocol.lastRequest?.method, "POST")
    }

    func testUpdateTenant() async throws {
        let http = mockHelper(json: #"{"id":"t-1","name":"Updated"}"#)
        let res = try await TenantService(http).update(id: "t-1", name: "Updated")
        XCTAssertEqual(res.name, "Updated")
        XCTAssertEqual(MockURLProtocol.lastRequest?.method, "PUT")
    }

    func testDeleteTenant() async throws {
        let http = mockHelper(json: "{}", status: 204)
        try await TenantService(http).delete(id: "t-1")
        XCTAssertEqual(MockURLProtocol.lastRequest?.method, "DELETE")
    }
}

// MARK: - User Extended Tests

final class UserExtendedTests: XCTestCase {
    func testGetUser() async throws {
        let http = mockHelper(json: #"{"id":"u-1","email":"test@example.com"}"#)
        let res = try await UserService(http).get(id: "u-1")
        XCTAssertEqual(res.email, "test@example.com")
    }

    func testCreateUser() async throws {
        let http = mockHelper(json: #"{"id":"u-2","email":"new@example.com"}"#)
        let res = try await UserService(http).create(email: "new@example.com")
        XCTAssertEqual(res.email, "new@example.com")
        XCTAssertEqual(MockURLProtocol.lastRequest?.method, "POST")
    }

    func testUpdateUser() async throws {
        let http = mockHelper(json: #"{"id":"u-1","name":"Updated Name"}"#)
        let res = try await UserService(http).update(id: "u-1", name: "Updated Name")
        XCTAssertEqual(MockURLProtocol.lastRequest?.method, "PUT")
    }

    func testDeleteUser() async throws {
        let http = mockHelper(json: "{}", status: 204)
        try await UserService(http).delete(id: "u-1")
        XCTAssertEqual(MockURLProtocol.lastRequest?.method, "DELETE")
    }
}

// MARK: - ApiKey Extended Tests

final class ApiKeyExtendedTests: XCTestCase {
    func testGetApiKey() async throws {
        let http = mockHelper(json: #"{"id":"key-1","name":"Test Key"}"#)
        let res = try await ApiKeyService(http).get(id: "key-1")
        XCTAssertEqual(res.name, "Test Key")
    }

    func testCreateApiKey() async throws {
        let http = mockHelper(json: #"{"id":"key-2","key":"sk-xxx","name":"New"}"#)
        let res = try await ApiKeyService(http).create(name: "New")
        XCTAssertEqual(res.name, "New")
        XCTAssertEqual(MockURLProtocol.lastRequest?.method, "POST")
    }

    func testRevokeApiKey() async throws {
        let http = mockHelper(json: "{}", status: 204)
        try await ApiKeyService(http).revoke(id: "key-1")
        XCTAssertEqual(MockURLProtocol.lastRequest?.method, "DELETE")
    }

    func testRotateApiKey() async throws {
        let http = mockHelper(json: #"{"id":"key-1","key":"sk-new"}"#)
        _ = try await ApiKeyService(http).rotate(id: "key-1")
        XCTAssertTrue(MockURLProtocol.lastRequest!.url.contains("/rotate"))
    }
}

// MARK: - Task Extended Tests

final class TaskExtendedTests: XCTestCase {
    func testGetTask() async throws {
        let http = mockHelper(json: #"{"id":"task-1","status":"running"}"#)
        let res = try await TaskService(http).get(id: "task-1")
        XCTAssertEqual(res.status, "running")
    }

    func testCreateTask() async throws {
        let http = mockHelper(json: #"{"id":"task-2","task_type":"extraction"}"#)
        let res = try await TaskService(http).create(taskType: "extraction")
        XCTAssertEqual(res.taskType, "extraction")
        XCTAssertEqual(MockURLProtocol.lastRequest?.method, "POST")
    }

    func testCancelTask() async throws {
        let http = mockHelper(json: #"{"id":"task-1","status":"cancelled"}"#)
        let res = try await TaskService(http).cancel(id: "task-1")
        XCTAssertEqual(res.status, "cancelled")
    }

    func testTaskStatus() async throws {
        let http = mockHelper(json: #"{"id":"task-1","status":"completed","progress":1.0}"#)
        let res = try await TaskService(http).status(id: "task-1")
        XCTAssertEqual(res.status, "completed")
    }
}

// MARK: - Pipeline Extended Tests

final class PipelineExtendedTests: XCTestCase {
    func testProcessingList() async throws {
        let http = mockHelper(json: #"{"items":[],"total":0}"#)
        let res = try await PipelineService(http).processingList()
        XCTAssertEqual(res.total, 0)
    }

    func testPause() async throws {
        let http = mockHelper(json: #"{"is_busy":false}"#)
        _ = try await PipelineService(http).pause()
        XCTAssertTrue(MockURLProtocol.lastRequest!.url.contains("/pause"))
    }

    func testResume() async throws {
        let http = mockHelper(json: #"{"is_busy":true}"#)
        _ = try await PipelineService(http).resume()
        XCTAssertTrue(MockURLProtocol.lastRequest!.url.contains("/resume"))
    }

    func testConfig() async throws {
        let http = mockHelper(json: #"{"max_workers":4,"batch_size":10}"#)
        let res = try await PipelineService(http).config()
        XCTAssertEqual(res.maxWorkers, 4)
    }
}

// MARK: - Model Extended Tests

final class ModelExtendedTests: XCTestCase {
    func testModelList() async throws {
        let http = mockHelper(json: #"{"models":[],"total":0}"#)
        _ = try await ModelService(http).list()
        XCTAssertTrue(MockURLProtocol.lastRequest!.url.contains("/list"))
    }

    func testGetModel() async throws {
        let http = mockHelper(json: #"{"id":"gpt-4","name":"GPT-4"}"#)
        let res = try await ModelService(http).get(id: "gpt-4")
        XCTAssertEqual(res.name, "GPT-4")
    }

    func testProviders() async throws {
        let http = mockHelper(json: #"[{"name":"openai"}]"#)
        let res = try await ModelService(http).providers()
        XCTAssertEqual(res.first?.name, "openai")
    }

    func testSetDefault() async throws {
        let http = mockHelper(json: #"{"provider":"openai","model":"gpt-4"}"#)
        let res = try await ModelService(http).setDefault(provider: "openai", model: "gpt-4")
        XCTAssertEqual(res.provider, "openai")
    }

    func testTestModel() async throws {
        let http = mockHelper(json: #"{"success":true,"latency_ms":100}"#)
        let res = try await ModelService(http).test(provider: "openai", model: "gpt-4")
        XCTAssertEqual(res.success, true)
    }
}

// MARK: - Cost Extended Tests

final class CostExtendedTests: XCTestCase {
    func testDaily() async throws {
        let http = mockHelper(json: #"[{"date":"2024-01-01","cost":10.5}]"#)
        let res = try await CostService(http).daily()
        XCTAssertEqual(res.first?.cost, 10.5)
    }

    func testByProvider() async throws {
        let http = mockHelper(json: #"[{"provider":"openai","cost":50.0}]"#)
        let res = try await CostService(http).byProvider()
        XCTAssertEqual(res.first?.provider, "openai")
    }

    func testByModel() async throws {
        let http = mockHelper(json: #"[{"model":"gpt-4","cost":30.0}]"#)
        let res = try await CostService(http).byModel()
        XCTAssertEqual(res.first?.model, "gpt-4")
    }

    func testExport() async throws {
        let http = mockHelper(json: "date,cost\n2024-01-01,10.5")
        let data = try await CostService(http).export(format: "csv")
        XCTAssertFalse(data.isEmpty)
    }
}

// MARK: - Conversation Extended Tests

final class ConversationExtendedTests: XCTestCase {
    func testUpdateConversation() async throws {
        let http = mockHelper(json: #"{"id":"conv-1","title":"Updated"}"#)
        let res = try await ConversationService(http).update(id: "conv-1", title: "Updated")
        XCTAssertEqual(res.title, "Updated")
        XCTAssertEqual(MockURLProtocol.lastRequest?.method, "PUT")
    }

    func testMessages() async throws {
        let http = mockHelper(json: #"[{"id":"msg-1","content":"Hello"}]"#)
        let res = try await ConversationService(http).messages(id: "conv-1")
        XCTAssertEqual(res.first?.content, "Hello")
    }

    func testAddMessage() async throws {
        let http = mockHelper(json: #"{"id":"msg-2","role":"user","content":"Hi"}"#)
        let res = try await ConversationService(http).addMessage(
            conversationId: "conv-1", role: "user", content: "Hi")
        XCTAssertEqual(res.content, "Hi")
    }

    func testDeleteMessage() async throws {
        let http = mockHelper(json: "{}", status: 204)
        try await ConversationService(http).deleteMessage(
            conversationId: "conv-1", messageId: "msg-1")
        XCTAssertEqual(MockURLProtocol.lastRequest?.method, "DELETE")
    }

    func testSearchConversations() async throws {
        let http = mockHelper(json: #"[{"id":"conv-1","title":"Test"}]"#)
        let res = try await ConversationService(http).search(query: "test")
        XCTAssertEqual(res.first?.title, "Test")
    }

    func testExportMessages() async throws {
        let http = mockHelper(json: #"[{"id":"msg-1"}]"#)
        let data = try await ConversationService(http).exportMessages(id: "conv-1")
        XCTAssertFalse(data.isEmpty)
    }
}

// MARK: - Folder Extended Tests

final class FolderExtendedTests: XCTestCase {
    func testGetFolder() async throws {
        let http = mockHelper(json: #"{"id":"folder-1","name":"Test"}"#)
        let res = try await FolderService(http).get(id: "folder-1")
        XCTAssertEqual(res.name, "Test")
    }

    func testUpdateFolder() async throws {
        let http = mockHelper(json: #"{"id":"folder-1","name":"Updated"}"#)
        let res = try await FolderService(http).update(id: "folder-1", name: "Updated")
        XCTAssertEqual(res.name, "Updated")
        XCTAssertEqual(MockURLProtocol.lastRequest?.method, "PUT")
    }

    func testMoveConversation() async throws {
        let http = mockHelper(json: #"{"id":"conv-1","folder_id":"folder-1"}"#)
        _ = try await FolderService(http).moveConversation(
            conversationId: "conv-1", folderId: "folder-1")
        XCTAssertEqual(MockURLProtocol.lastRequest?.method, "POST")
    }

    func testFolderConversations() async throws {
        let http = mockHelper(json: #"[{"id":"conv-1","title":"In Folder"}]"#)
        let res = try await FolderService(http).conversations(id: "folder-1")
        XCTAssertEqual(res.first?.title, "In Folder")
    }
}

// MARK: - Auth Service Tests (New)

final class AuthServiceTests: XCTestCase {
    func testLogin() async throws {
        let http = mockHelper(json: #"{"access_token":"tok-xxx","refresh_token":"ref-xxx"}"#)
        let res = try await AuthService(http).login(email: "test@example.com", password: "secret")
        XCTAssertEqual(res.accessToken, "tok-xxx")
        XCTAssertEqual(MockURLProtocol.lastRequest?.method, "POST")
    }

    func testLogout() async throws {
        let http = mockHelper(json: "{}", status: 204)
        try await AuthService(http).logout()
        XCTAssertTrue(MockURLProtocol.lastRequest!.url.contains("/logout"))
    }

    func testRefresh() async throws {
        let http = mockHelper(json: #"{"access_token":"new-tok"}"#)
        let res = try await AuthService(http).refresh(refreshToken: "ref-xxx")
        XCTAssertEqual(res.accessToken, "new-tok")
    }

    func testMe() async throws {
        let http = mockHelper(json: #"{"id":"u-1","email":"me@example.com"}"#)
        let res = try await AuthService(http).me()
        XCTAssertEqual(res.email, "me@example.com")
    }

    func testChangePassword() async throws {
        let http = mockHelper(json: "{}", status: 204)
        try await AuthService(http).changePassword(currentPassword: "old", newPassword: "new")
        XCTAssertTrue(MockURLProtocol.lastRequest!.url.contains("/change-password"))
    }
}

// MARK: - Workspace Service Tests (New)

final class WorkspaceServiceTests: XCTestCase {
    func testListWorkspaces() async throws {
        let http = mockHelper(json: #"{"items":[],"total":0}"#)
        let res = try await WorkspaceService(http).list()
        XCTAssertEqual(res.total, 0)
    }

    func testGetWorkspace() async throws {
        let http = mockHelper(json: #"{"id":"ws-1","name":"Default"}"#)
        let res = try await WorkspaceService(http).get(id: "ws-1")
        XCTAssertEqual(res.name, "Default")
    }

    func testCreateWorkspace() async throws {
        let http = mockHelper(json: #"{"id":"ws-2","name":"New"}"#)
        let res = try await WorkspaceService(http).create(name: "New")
        XCTAssertEqual(res.name, "New")
        XCTAssertEqual(MockURLProtocol.lastRequest?.method, "POST")
    }

    func testUpdateWorkspace() async throws {
        let http = mockHelper(json: #"{"id":"ws-1","name":"Updated"}"#)
        let res = try await WorkspaceService(http).update(id: "ws-1", name: "Updated")
        XCTAssertEqual(res.name, "Updated")
        XCTAssertEqual(MockURLProtocol.lastRequest?.method, "PUT")
    }

    func testDeleteWorkspace() async throws {
        let http = mockHelper(json: "{}", status: 204)
        try await WorkspaceService(http).delete(id: "ws-1")
        XCTAssertEqual(MockURLProtocol.lastRequest?.method, "DELETE")
    }

    func testWorkspaceStats() async throws {
        let http = mockHelper(json: #"{"document_count":10,"entity_count":50}"#)
        let res = try await WorkspaceService(http).stats(id: "ws-1")
        XCTAssertEqual(res.documentCount, 10)
    }

    func testSwitchWorkspace() async throws {
        let http = mockHelper(json: #"{"id":"ws-2","name":"Switched"}"#)
        let res = try await WorkspaceService(http).switchTo(id: "ws-2")
        XCTAssertEqual(res.name, "Switched")
    }
}

// MARK: - Shared Service Tests (New)

final class SharedServiceTests: XCTestCase {
    func testCreateLink() async throws {
        let http = mockHelper(json: #"{"id":"link-1","url":"https://share.example.com/xxx"}"#)
        let res = try await SharedService(http).createLink(
            resourceType: "document", resourceId: "doc-1")
        XCTAssertEqual(res.id, "link-1")
        XCTAssertEqual(MockURLProtocol.lastRequest?.method, "POST")
    }

    func testGetLink() async throws {
        let http = mockHelper(json: #"{"id":"link-1","resource_type":"document"}"#)
        let res = try await SharedService(http).getLink(id: "link-1")
        XCTAssertEqual(res.resourceType, "document")
    }

    func testDeleteLink() async throws {
        let http = mockHelper(json: "{}", status: 204)
        try await SharedService(http).deleteLink(id: "link-1")
        XCTAssertEqual(MockURLProtocol.lastRequest?.method, "DELETE")
    }

    func testAccess() async throws {
        let http = mockHelper(json: #"{"resource_type":"document","resource_id":"doc-1"}"#)
        let res = try await SharedService(http).access(token: "xxx-token")
        XCTAssertEqual(res.resourceType, "document")
    }

    func testListLinks() async throws {
        let http = mockHelper(json: #"[{"id":"link-1"}]"#)
        let res = try await SharedService(http).listLinks()
        XCTAssertEqual(res.first?.id, "link-1")
    }
}

// MARK: - Client Extended Service Availability Tests

final class ClientExtendedServiceAvailabilityTests: XCTestCase {
    func testHasAuth() {
        let client = EdgeQuakeClient()
        XCTAssertNotNil(client.auth)
    }

    func testHasWorkspaces() {
        let client = EdgeQuakeClient()
        XCTAssertNotNil(client.workspaces)
    }

    func testHasShared() {
        let client = EdgeQuakeClient()
        XCTAssertNotNil(client.shared)
    }
}

// MARK: - OODA-45: Additional Edge Case Tests

final class OODA45EdgeCaseTests: XCTestCase {
    // Document edge cases
    func testDocumentListReturnsResponse() async throws {
        let http = mockHelper(json: #"{"items":[],"total":0,"page":1,"total_pages":0}"#)
        let res = try await DocumentService(http).list()
        XCTAssertEqual(res.total, 0)
    }

    func testDocumentGetReturnsDocument() async throws {
        let http = mockHelper(json: #"{"id":"d-1","title":"Test"}"#)
        let res = try await DocumentService(http).get(id: "d-1")
        XCTAssertEqual(res.id, "d-1")
    }

    func testDocumentChunksReturnsChunks() async throws {
        let http = mockHelper(json: #"{"document_id":"d-1","chunks":[]}"#)
        let res = try await DocumentService(http).chunks(id: "d-1")
        XCTAssertEqual(res.documentId, "d-1")
    }

    func testDocumentStatusReturnsStatus() async throws {
        let http = mockHelper(json: #"{"status":"completed"}"#)
        let res = try await DocumentService(http).status(id: "d-1")
        XCTAssertEqual(res.status, "completed")
    }

    // Entity edge cases
    func testEntityListReturnsResponse() async throws {
        let http = mockHelper(json: #"{"items":[],"total":0}"#)
        let res = try await EntityService(http).list()
        XCTAssertEqual(res.total, 0)
    }

    func testEntityExistsReturnsTrue() async throws {
        let http = mockHelper(json: #"{"exists":true,"entity_id":"e-123"}"#)
        let res = try await EntityService(http).exists(name: "TEST_ENTITY")
        XCTAssertEqual(res.exists, true)
    }

    func testEntityExistsReturnsFalse() async throws {
        let http = mockHelper(json: #"{"exists":false}"#)
        let res = try await EntityService(http).exists(name: "MISSING")
        XCTAssertEqual(res.exists, false)
    }

    func testEntityTypesReturnsTypes() async throws {
        let http = mockHelper(json: #"{"types":["PERSON","ORGANIZATION"]}"#)
        let res = try await EntityService(http).types()
        XCTAssertEqual(res.types?.count, 2)
    }

    // Graph edge cases
    func testGraphGetReturnsGraph() async throws {
        let http = mockHelper(json: #"{"nodes":[],"edges":[]}"#)
        let res = try await GraphService(http).get()
        XCTAssertTrue(res.nodes?.isEmpty ?? true)
    }

    func testGraphSearchReturnsResults() async throws {
        let http = mockHelper(json: #"{"nodes":[]}"#)
        let res = try await GraphService(http).search(query: "test")
        XCTAssertTrue(res.nodes?.isEmpty ?? true)
    }

    func testGraphStatsReturnsStats() async throws {
        let http = mockHelper(json: #"{"node_count":10,"edge_count":20}"#)
        let res = try await GraphService(http).stats()
        XCTAssertEqual(res.nodeCount, 10)
    }

    // Pipeline edge cases
    func testPipelineStatusReturnsStatus() async throws {
        let http = mockHelper(json: #"{"is_busy":false,"pending_tasks":0}"#)
        let res = try await PipelineService(http).status()
        XCTAssertEqual(res.isBusy, false)
    }

    func testPipelineQueueMetricsReturns() async throws {
        let http = mockHelper(json: #"{"pending_count":12,"processing_count":3}"#)
        let res = try await PipelineService(http).queueMetrics()
        XCTAssertEqual(res.pendingCount, 12)
    }

    // Cost edge cases
    func testCostSummaryReturns() async throws {
        let http = mockHelper(json: #"{"total_cost":100.50}"#)
        let res = try await CostService(http).summary()
        XCTAssertEqual(res.totalCost, 100.5)
    }

    func testCostDailyReturns() async throws {
        let http = mockHelper(json: #"[]"#)
        let res = try await CostService(http).daily()
        XCTAssertTrue(res.isEmpty)
    }

    // Model edge cases
    func testModelListReturns() async throws {
        let http = mockHelper(json: #"{"models":[]}"#)
        let res = try await ModelService(http).list()
        XCTAssertTrue(res.models?.isEmpty ?? true)
    }

    func testModelHealthReturns() async throws {
        let http = mockHelper(json: #"[{"name":"openai","enabled":true}]"#)
        let res = try await ModelService(http).health()
        XCTAssertEqual(res.count, 1)
    }

    // Task edge cases
    func testTaskListReturns() async throws {
        let http = mockHelper(json: #"{"tasks":[],"total":0}"#)
        let res = try await TaskService(http).list()
        XCTAssertEqual(res.total, 0)
    }

    func testTaskGetReturns() async throws {
        let http = mockHelper(json: #"{"id":"t-1","status":"completed"}"#)
        let res = try await TaskService(http).get(id: "t-1")
        XCTAssertEqual(res.status, "completed")
    }

    // Folder edge cases
    func testFolderListReturnsArray() async throws {
        let http = mockHelper(json: #"[]"#)
        let res = try await FolderService(http).list()
        XCTAssertTrue(res.isEmpty)
    }

    func testFolderCreateReturnsFolder() async throws {
        let http = mockHelper(json: #"{"id":"f-1","name":"Test"}"#)
        let res = try await FolderService(http).create(name: "Test")
        XCTAssertEqual(res.name, "Test")
    }
}

// MARK: - OODA-45: Relationship & Conversation Edge Cases

final class OODA45RelationshipConversationTests: XCTestCase {
    func testRelationshipListReturns() async throws {
        let http = mockHelper(json: #"{"items":[],"total":0}"#)
        let res = try await RelationshipService(http).list()
        XCTAssertEqual(res.total, 0)
    }

    func testRelationshipTypesReturns() async throws {
        let http = mockHelper(json: #"{"types":["WORKS_FOR","LOCATED_IN"]}"#)
        let res = try await RelationshipService(http).types()
        XCTAssertEqual(res.types?.count, 2)
    }

    func testConversationListReturnsEmpty() async throws {
        let http = mockHelper(json: #"{"items":[]}"#)
        let res = try await ConversationService(http).list()
        XCTAssertTrue(res.isEmpty)
    }

    func testConversationGetReturnsDetail() async throws {
        let http = mockHelper(json: #"{"conversation":{"id":"conv-1","title":"Test"},"messages":[]}"#)
        let res = try await ConversationService(http).get(id: "conv-1")
        XCTAssertEqual(res.id, "conv-1")
    }

    func testConversationSearchReturnsArray() async throws {
        let http = mockHelper(json: #"[]"#)
        let res = try await ConversationService(http).search(query: "test")
        XCTAssertTrue(res.isEmpty)
    }
}

// MARK: - OODA-45: Tenant & User Service Tests

final class OODA45TenantUserTests: XCTestCase {
    func testTenantListReturns() async throws {
        let http = mockHelper(json: #"{"items":[]}"#)
        let res = try await TenantService(http).list()
        XCTAssertTrue(res.items?.isEmpty ?? true)
    }

    func testTenantCreateReturns() async throws {
        let http = mockHelper(json: #"{"id":"t-1","name":"Test"}"#)
        let res = try await TenantService(http).create(name: "Test")
        XCTAssertEqual(res.id, "t-1")
    }

    func testUserListReturns() async throws {
        let http = mockHelper(json: #"{"users":[]}"#)
        let res = try await UserService(http).list()
        XCTAssertTrue(res.users?.isEmpty ?? true)
    }

    func testUserGetReturns() async throws {
        let http = mockHelper(json: #"{"id":"u-1","email":"test@example.com"}"#)
        let res = try await UserService(http).get(id: "u-1")
        XCTAssertEqual(res.email, "test@example.com")
    }
}
