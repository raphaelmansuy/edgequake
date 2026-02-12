import Foundation

// MARK: - Service classes

/// WHY: Each service maps 1:1 to an API resource for discoverability.

public final class HealthService: @unchecked Sendable {
    private let http: HttpHelper
    init(_ http: HttpHelper) { self.http = http }

    public func check() async throws -> HealthResponse {
        try await http.get("/health")
    }
}

public final class DocumentService: @unchecked Sendable {
    private let http: HttpHelper
    init(_ http: HttpHelper) { self.http = http }

    public func list(page: Int = 1, pageSize: Int = 20) async throws -> ListDocumentsResponse {
        try await http.get("/api/v1/documents?page=\(page)&page_size=\(pageSize)")
    }

    public func get(id: String) async throws -> Document {
        try await http.get("/api/v1/documents/\(id)")
    }

    public func uploadText(title: String, content: String) async throws -> UploadResponse {
        try await http.post(
            "/api/v1/documents", body: TextUploadRequest(title: title, content: content))
    }

    /// WHY: DELETE returns 204 No Content — use deleteRaw to avoid decoding empty body.
    public func delete(id: String) async throws {
        _ = try await http.deleteRaw("/api/v1/documents/\(id)")
    }
}

public final class EntityService: @unchecked Sendable {
    private let http: HttpHelper
    init(_ http: HttpHelper) { self.http = http }

    public func list(page: Int = 1, pageSize: Int = 20) async throws -> EntityListResponse {
        try await http.get("/api/v1/graph/entities?page=\(page)&page_size=\(pageSize)")
    }

    public func get(name: String) async throws -> EntityDetailResponse {
        try await http.get("/api/v1/graph/entities/\(name)")
    }

    public func create(_ request: CreateEntityRequest) async throws -> CreateEntityResponse {
        try await http.post("/api/v1/graph/entities", body: request)
    }

    public func delete(name: String) async throws -> EntityDeleteResponse {
        try await http.delete("/api/v1/graph/entities/\(name)?confirm=true")
    }

    public func exists(name: String) async throws -> EntityExistsResponse {
        try await http.get("/api/v1/graph/entities/exists?entity_name=\(name)")
    }
}

public final class RelationshipService: @unchecked Sendable {
    private let http: HttpHelper
    init(_ http: HttpHelper) { self.http = http }

    public func list(page: Int = 1, pageSize: Int = 20) async throws -> RelationshipListResponse {
        try await http.get("/api/v1/graph/relationships?page=\(page)&page_size=\(pageSize)")
    }
}

public final class GraphService: @unchecked Sendable {
    private let http: HttpHelper
    init(_ http: HttpHelper) { self.http = http }

    public func get() async throws -> GraphResponse {
        try await http.get("/api/v1/graph")
    }

    public func search(query: String) async throws -> SearchNodesResponse {
        let encoded = query.addingPercentEncoding(withAllowedCharacters: .urlQueryAllowed) ?? query
        return try await http.get("/api/v1/graph/nodes/search?q=\(encoded)")
    }
}

public final class QueryService: @unchecked Sendable {
    private let http: HttpHelper
    init(_ http: HttpHelper) { self.http = http }

    public func execute(query: String, mode: String = "hybrid") async throws -> QueryResponse {
        try await http.post("/api/v1/query", body: QueryRequest(query: query, mode: mode))
    }
}

public final class ChatService: @unchecked Sendable {
    private let http: HttpHelper
    init(_ http: HttpHelper) { self.http = http }

    public func completions(_ request: ChatCompletionRequest) async throws -> ChatCompletionResponse
    {
        try await http.post("/api/v1/chat/completions", body: request)
    }
}

public final class TenantService: @unchecked Sendable {
    private let http: HttpHelper
    init(_ http: HttpHelper) { self.http = http }

    public func list() async throws -> TenantListResponse {
        try await http.get("/api/v1/tenants")
    }
}

public final class UserService: @unchecked Sendable {
    private let http: HttpHelper
    init(_ http: HttpHelper) { self.http = http }

    public func list() async throws -> UserListResponse {
        try await http.get("/api/v1/users")
    }
}

public final class ApiKeyService: @unchecked Sendable {
    private let http: HttpHelper
    init(_ http: HttpHelper) { self.http = http }

    public func list() async throws -> ApiKeyListResponse {
        try await http.get("/api/v1/api-keys")
    }
}

public final class TaskService: @unchecked Sendable {
    private let http: HttpHelper
    init(_ http: HttpHelper) { self.http = http }

    public func list() async throws -> TaskListResponse {
        try await http.get("/api/v1/tasks")
    }
}

public final class PipelineService: @unchecked Sendable {
    private let http: HttpHelper
    init(_ http: HttpHelper) { self.http = http }

    public func status() async throws -> PipelineStatus {
        try await http.get("/api/v1/pipeline/status")
    }

    public func queueMetrics() async throws -> QueueMetrics {
        try await http.get("/api/v1/pipeline/queue-metrics")
    }
}

public final class ModelService: @unchecked Sendable {
    private let http: HttpHelper
    init(_ http: HttpHelper) { self.http = http }

    public func catalog() async throws -> ProviderCatalog {
        try await http.get("/api/v1/models")
    }

    public func health() async throws -> [ProviderHealthInfo] {
        let data = try await http.getRaw("/api/v1/models/health")
        return try http.decodeJSON([ProviderHealthInfo].self, from: data)
    }

    public func providerStatus() async throws -> ProviderStatus {
        try await http.get("/api/v1/settings/provider/status")
    }
}

public final class CostService: @unchecked Sendable {
    private let http: HttpHelper
    init(_ http: HttpHelper) { self.http = http }

    public func summary() async throws -> CostSummary {
        try await http.get("/api/v1/costs/summary")
    }
}

public final class ConversationService: @unchecked Sendable {
    private let http: HttpHelper
    init(_ http: HttpHelper) { self.http = http }

    /// WHY: GET /api/v1/conversations returns {"items":[...]} wrapper, not raw array.
    public func list() async throws -> [ConversationInfo] {
        let wrapper: ConversationListResponse = try await http.get("/api/v1/conversations")
        return wrapper.items ?? []
    }

    public func create(title: String) async throws -> ConversationInfo {
        try await http.post("/api/v1/conversations", body: CreateConversationRequest(title: title))
    }

    public func get(id: String) async throws -> ConversationDetail {
        try await http.get("/api/v1/conversations/\(id)")
    }

    /// WHY: DELETE returns 204 No Content — use deleteRaw to avoid decoding empty body.
    public func delete(id: String) async throws {
        _ = try await http.deleteRaw("/api/v1/conversations/\(id)")
    }

    public func bulkDelete(ids: [String]) async throws -> BulkDeleteResponse {
        try await http.post("/api/v1/conversations/bulk/delete", body: ["ids": ids])
    }
}

public final class FolderService: @unchecked Sendable {
    private let http: HttpHelper
    init(_ http: HttpHelper) { self.http = http }

    public func list() async throws -> [FolderInfo] {
        try await http.get("/api/v1/folders")
    }

    public func create(name: String) async throws -> FolderInfo {
        try await http.post("/api/v1/folders", body: CreateFolderRequest(name: name))
    }

    /// WHY: DELETE returns 204 No Content — use deleteRaw to avoid decoding empty body.
    public func delete(id: String) async throws {
        _ = try await http.deleteRaw("/api/v1/folders/\(id)")
    }
}
