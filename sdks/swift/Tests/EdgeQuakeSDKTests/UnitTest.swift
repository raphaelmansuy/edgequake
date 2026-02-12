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
