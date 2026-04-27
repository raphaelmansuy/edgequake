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
        let http = mockHelper(json: #"{"affected":3,"status":"ok"}"#)
        let svc = ConversationService(http)
        let result = try await svc.bulkDelete(ids: ["c1", "c2", "c3"])
        XCTAssertEqual(result.affected, 3)
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
        let http = mockHelper(json: "OK", status: 200)
        let res = try await HealthService(http).readiness()
        XCTAssertTrue(res.contains("OK"))
        XCTAssertTrue(MockURLProtocol.lastRequest!.url.contains("/ready"))
    }

    func testLiveness() async throws {
        let http = mockHelper(json: "OK", status: 200)
        let res = try await HealthService(http).liveness()
        XCTAssertTrue(res.contains("OK"))
        XCTAssertTrue(MockURLProtocol.lastRequest!.url.contains("/live"))
    }

    func testMetrics() async throws {
        let http = mockHelper(json: "# TYPE eq_up gauge\neq_up 1\n", status: 200)
        let res = try await HealthService(http).metrics()
        XCTAssertTrue(res.contains("TYPE"))
        XCTAssertTrue(MockURLProtocol.lastRequest!.url.contains("/metrics"))
    }
}

// MARK: - Document Extended Tests

final class DocumentExtendedTests: XCTestCase {
    func testTrack() async throws {
        let http = mockHelper(
            json: #"{"track_id":"tk-1","status":"processing","progress":0.5}"#)
        let res = try await DocumentService(http).track(trackId: "tk-1")
        XCTAssertEqual(res.trackId, "tk-1")
        XCTAssertTrue(MockURLProtocol.lastRequest!.url.contains("/documents/track/tk-1"))
    }

    func testReprocessFailed() async throws {
        let http = mockHelper(json: #"{"status":"ok"}"#)
        _ = try await DocumentService(http).reprocessFailed()
        XCTAssertEqual(MockURLProtocol.lastRequest?.method, "POST")
        XCTAssertTrue(MockURLProtocol.lastRequest!.url.contains("/documents/reprocess"))
    }

    func testRecoverStuck() async throws {
        let http = mockHelper(json: #"{"status":"ok"}"#)
        _ = try await DocumentService(http).recoverStuck()
        XCTAssertTrue(MockURLProtocol.lastRequest!.url.contains("/recover-stuck"))
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

    func testNeighborhood() async throws {
        let http = mockHelper(json: #"{"nodes":[],"edges":[]}"#)
        _ = try await EntityService(http).neighborhood(name: "ALICE", depth: 2)
        XCTAssertTrue(MockURLProtocol.lastRequest!.url.contains("/neighborhood"))
        XCTAssertTrue(MockURLProtocol.lastRequest!.url.contains("depth=2"))
    }
}

// MARK: - Relationship Extended Tests

final class RelationshipExtendedTests: XCTestCase {
    func testCreateRelationship() async throws {
        let json = #"""
            {"status":"success","message":"ok","relationship":{"id":"rel-1","src_id":"A","tgt_id":"B",
             "relation_type":"KNOWS","keywords":"knows","weight":0.8,"description":"d","source_id":"manual_entry",
             "created_at":"2025-01-01T00:00:00Z","updated_at":"2025-01-01T00:00:00Z","metadata":null}}
            """#
        let http = mockHelper(json: json)
        let res = try await RelationshipService(http).create(
            srcId: "A", tgtId: "B", keywords: "knows", description: "d")
        XCTAssertEqual(res.status, "success")
        XCTAssertEqual(res.relationship?.srcId, "A")
        XCTAssertEqual(MockURLProtocol.lastRequest?.method, "POST")
    }

    func testDeleteRelationship() async throws {
        let http = mockHelper(json: "{}", status: 204)
        try await RelationshipService(http).delete(id: "rel-1")
        XCTAssertEqual(MockURLProtocol.lastRequest?.method, "DELETE")
    }

    func testGetRelationship() async throws {
        let json = #"{"relationship":{"id":"rel-1","src_id":"A","tgt_id":"B"}}"#
        let http = mockHelper(json: json)
        let res = try await RelationshipService(http).get(id: "rel-1")
        XCTAssertEqual(res.relationship?.id, "rel-1")
    }
}

// MARK: - Graph Extended Tests

final class GraphExtendedTests: XCTestCase {
    func testGetNode() async throws {
        let http = mockHelper(json: #"{"id":"n1","label":"X"}"#)
        let res = try await GraphService(http).getNode(nodeId: "n1")
        XCTAssertEqual(res.id, "n1")
        XCTAssertTrue(MockURLProtocol.lastRequest!.url.contains("/graph/nodes/n1"))
    }

    func testLabelSearch() async throws {
        let http = mockHelper(json: #"{"labels":["PERSON"]}"#)
        let res = try await GraphService(http).labelSearch(query: "PER")
        XCTAssertEqual(res.labels?.count, 1)
    }

    func testPopularLabels() async throws {
        let http = mockHelper(json: #"{"labels":[{"label":"A","degree":3}],"total_entities":10}"#)
        let res = try await GraphService(http).popularLabels(limit: 5)
        XCTAssertEqual(res.labels?.count, 1)
    }

    func testDegreesBatch() async throws {
        let http = mockHelper(json: #"{"degrees":[{"node_id":"a","degree":2}],"count":1}"#)
        let res = try await GraphService(http).degreesBatch(nodeIds: ["a", "b"])
        XCTAssertEqual(res.count, 1)
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
        let http = mockHelper(json: #"{"id":"u-2","email":"new@example.com","username":"newuser"}"#)
        let res = try await UserService(http).create(
            username: "newuser", email: "new@example.com", password: "secret")
        XCTAssertEqual(res.email, "new@example.com")
        XCTAssertEqual(MockURLProtocol.lastRequest?.method, "POST")
    }

    func testDeleteUser() async throws {
        let http = mockHelper(json: "{}", status: 204)
        try await UserService(http).delete(id: "u-1")
        XCTAssertEqual(MockURLProtocol.lastRequest?.method, "DELETE")
    }
}

// MARK: - ApiKey Extended Tests

final class ApiKeyExtendedTests: XCTestCase {
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

}

// MARK: - Task Extended Tests

final class TaskExtendedTests: XCTestCase {
    func testGetTask() async throws {
        let http = mockHelper(json: #"{"id":"task-1","status":"running"}"#)
        let res = try await TaskService(http).get(trackId: "task-1")
        XCTAssertEqual(res.status, "running")
    }

    func testCancelTask() async throws {
        let http = mockHelper(json: #"{"id":"task-1","status":"cancelled"}"#)
        let res = try await TaskService(http).cancel(trackId: "task-1")
        XCTAssertEqual(res.status, "cancelled")
    }

    func testRetryTask() async throws {
        let http = mockHelper(json: #"{"id":"task-1","status":"pending"}"#)
        let res = try await TaskService(http).retry(trackId: "task-1")
        XCTAssertEqual(MockURLProtocol.lastRequest?.method, "POST")
        XCTAssertTrue(MockURLProtocol.lastRequest!.url.contains("/retry"))
        XCTAssertEqual(res.status, "pending")
    }
}

// MARK: - Pipeline Extended Tests

final class PipelineExtendedTests: XCTestCase {
    func testPipelineCancel() async throws {
        let http = mockHelper(json: #"{"status":"ok"}"#)
        _ = try await PipelineService(http).cancel()
        XCTAssertTrue(MockURLProtocol.lastRequest!.url.contains("/pipeline/cancel"))
    }
}

// MARK: - Model Extended Tests

final class ModelExtendedTests: XCTestCase {
    func testModelCatalog() async throws {
        let http = mockHelper(
            json: #"{"providers":[],"default_llm_provider":"x","default_llm_model":"y","default_embedding_provider":"e","default_embedding_model":"m"}"#
        )
        let res = try await ModelService(http).catalog()
        XCTAssertNotNil(res.defaultLlmProvider)
        XCTAssertTrue(MockURLProtocol.lastRequest!.url.hasSuffix("/models"))
    }

    func testListLlmModels() async throws {
        let http = mockHelper(json: #"{"models":[],"default_provider":"a","default_model":"b"}"#)
        let res = try await ModelService(http).listLlmModels()
        XCTAssertEqual(res.defaultProvider, "a")
    }

    func testListAvailableProviders() async throws {
        let http = mockHelper(json: #"[{"name":"openai"}]"#)
        let res = try await ModelService(http).listAvailableProviders()
        XCTAssertEqual(res.first?.name, "openai")
    }

    func testGetModelCard() async throws {
        let http = mockHelper(json: #"{"name":"m1","display_name":"M","model_type":"llm"}"#)
        let res = try await ModelService(http).getModel(provider: "openai", model: "m1")
        XCTAssertEqual(res.name, "m1")
    }
}

// MARK: - Cost Extended Tests

final class CostExtendedTests: XCTestCase {
    func testHistory() async throws {
        let http = mockHelper(json: #"[{"timestamp":"2024-01-01","total_cost":1.0,"total_tokens":2,"document_count":3}]"#)
        let res = try await CostService(http).history()
        XCTAssertEqual(res.first?.totalCost, 1.0)
    }

    func testBudget() async throws {
        let http = mockHelper(
            json: #"{"monthly_budget_usd":100,"spent_usd":0,"remaining_usd":100,"alert_threshold":80,"is_over_budget":false}"#
        )
        let res = try await CostService(http).budget()
        XCTAssertEqual(res.monthlyBudgetUsd, 100)
    }
}

// MARK: - Conversation Extended Tests

final class ConversationExtendedTests: XCTestCase {
    func testUpdateConversation() async throws {
        let http = mockHelper(json: #"{"id":"conv-1","title":"Updated"}"#)
        let res = try await ConversationService(http).update(id: "conv-1", title: "Updated")
        XCTAssertEqual(res.title, "Updated")
        XCTAssertEqual(MockURLProtocol.lastRequest?.method, "PATCH")
    }

    func testMessages() async throws {
        let http = mockHelper(
            json: #"{"items":[{"id":"msg-1","content":"Hello"}],"pagination":{"total":1,"has_more":false}}"#
        )
        let res = try await ConversationService(http).messages(id: "conv-1")
        XCTAssertEqual(res.items?.first?.content, "Hello")
    }

    func testAddMessage() async throws {
        let http = mockHelper(json: #"{"id":"msg-2","role":"user","content":"Hi"}"#)
        let res = try await ConversationService(http).addMessage(
            conversationId: "conv-1", role: "user", content: "Hi")
        XCTAssertEqual(res.content, "Hi")
    }

    func testDeleteMessage() async throws {
        let http = mockHelper(json: "{}", status: 204)
        try await ConversationService(http).deleteMessage(messageId: "msg-1")
        XCTAssertEqual(MockURLProtocol.lastRequest?.method, "DELETE")
        XCTAssertTrue(MockURLProtocol.lastRequest!.url.contains("/api/v1/messages/msg-1"))
    }
}

// MARK: - Folder Extended Tests

final class FolderExtendedTests: XCTestCase {
    func testUpdateFolder() async throws {
        let http = mockHelper(json: #"{"id":"folder-1","name":"Updated"}"#)
        let res = try await FolderService(http).update(id: "folder-1", name: "Updated")
        XCTAssertEqual(res.name, "Updated")
        XCTAssertEqual(MockURLProtocol.lastRequest?.method, "PATCH")
    }
}

// MARK: - Auth Service Tests (New)

final class AuthServiceTests: XCTestCase {
    func testLogin() async throws {
        let http = mockHelper(json: #"{"access_token":"tok-xxx","refresh_token":"ref-xxx"}"#)
        let res = try await AuthService(http).login(username: "test@example.com", password: "secret")
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

}

// MARK: - Workspace Service Tests (New)

final class WorkspaceServiceTests: XCTestCase {
    func testListWorkspaces() async throws {
        let http = mockHelper(json: #"{"items":[],"total":0}"#)
        let res = try await WorkspaceService(http).list(tenantId: "t-1")
        XCTAssertEqual(res.total, 0)
        XCTAssertTrue(MockURLProtocol.lastRequest!.url.contains("/tenants/t-1/workspaces"))
    }

    func testGetWorkspace() async throws {
        let http = mockHelper(json: #"{"id":"ws-1","name":"Default"}"#)
        let res = try await WorkspaceService(http).get(id: "ws-1")
        XCTAssertEqual(res.name, "Default")
    }

    func testCreateWorkspace() async throws {
        let http = mockHelper(json: #"{"id":"ws-2","name":"New"}"#)
        let res = try await WorkspaceService(http).create(tenantId: "t-1", name: "New")
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

}

// MARK: - Shared Service Tests (New)

final class SharedServiceTests: XCTestCase {
    func testGetShared() async throws {
        let json = """
            {"conversation":{"id":"c-1","title":"Shared"},"messages":[]}
            """
        let http = mockHelper(json: json)
        let res = try await SharedService(http).get(shareId: "shr-1")
        XCTAssertEqual(res.conversation?.id, "c-1")
        XCTAssertTrue(MockURLProtocol.lastRequest!.url.contains("/api/v1/shared/shr-1"))
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

    func testDocumentTrackReturnsStatus() async throws {
        let http = mockHelper(json: #"{"track_id":"tk","status":"completed"}"#)
        let res = try await DocumentService(http).track(trackId: "tk")
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

    func testEntityNeighborhood() async throws {
        let http = mockHelper(json: #"{"nodes":[],"edges":[]}"#)
        let res = try await EntityService(http).neighborhood(name: "E1")
        XCTAssertTrue(res.nodes?.isEmpty ?? true)
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

    func testGraphPopularLabels() async throws {
        let http = mockHelper(json: #"{"labels":[],"total_entities":0}"#)
        let res = try await GraphService(http).popularLabels(limit: 5)
        XCTAssertEqual(res.totalEntities, 0)
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
        let http = mockHelper(
            json: #"{"workspace_id":"ws","total_cost":100.50,"document_count":1,"total_tokens":2,"by_operation":[]}"#
        )
        let res = try await CostService(http).summary()
        XCTAssertEqual(res.totalCost, 100.5)
    }

    func testCostHistoryReturns() async throws {
        let http = mockHelper(json: #"[]"#)
        let res = try await CostService(http).history()
        XCTAssertTrue(res.isEmpty)
    }

    // Model edge cases
    func testModelCatalogReturns() async throws {
        let http = mockHelper(
            json: #"{"providers":[],"default_llm_provider":"a","default_llm_model":"b","default_embedding_provider":"c","default_embedding_model":"d"}"#
        )
        let res = try await ModelService(http).catalog()
        XCTAssertTrue(res.providers?.isEmpty ?? true)
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
        let res = try await TaskService(http).get(trackId: "t-1")
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

    func testRelationshipGetReturns() async throws {
        let http = mockHelper(json: #"{"relationship":{"id":"r1"}}"#)
        let res = try await RelationshipService(http).get(id: "r1")
        XCTAssertEqual(res.relationship?.id, "r1")
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
