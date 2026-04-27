import Foundation
import XCTest

@testable import EdgeQuakeSDK

// MARK: - Entity Lineage Tests

/// WHY: Verify all Entity fields round-trip through JSON, including metadata and lineage fields.
final class EntityLineageTest: XCTestCase {
    func testFullDeserialization() async throws {
        let json = """
            {
                "entity": {
                    "id": "e-1", "entityName": "SARAH_CHEN", "entityType": "PERSON",
                    "description": "AI researcher", "sourceId": "doc-1",
                    "createdAt": "2025-01-01T00:00:00Z", "updatedAt": "2025-01-02T00:00:00Z",
                    "degree": 5, "metadata": {"confidence": 0.95, "source": "pdf"}
                },
                "relationships": {"incoming": 2, "outgoing": 3},
                "statistics": {"pageRank": 0.42}
            }
            """
        let http = mockHelper(json: json)
        let svc = EntityService(http)
        let result = try await svc.get(id: "e-1")
        XCTAssertEqual(result.entity?.id, "e-1")
        XCTAssertEqual(result.entity?.entityName, "SARAH_CHEN")
        XCTAssertEqual(result.entity?.entityType, "PERSON")
        XCTAssertEqual(result.entity?.description, "AI researcher")
        XCTAssertEqual(result.entity?.sourceId, "doc-1")
        XCTAssertEqual(result.entity?.createdAt, "2025-01-01T00:00:00Z")
        XCTAssertEqual(result.entity?.updatedAt, "2025-01-02T00:00:00Z")
        XCTAssertEqual(result.entity?.degree, 5)
        // Metadata via AnyCodable
        let meta = result.entity?.metadata
        XCTAssertNotNil(meta)
        XCTAssertEqual(meta?["confidence"]?.value as? Double, 0.95)
        XCTAssertEqual(meta?["source"]?.value as? String, "pdf")
        // Relationships and statistics via AnyCodable
        XCTAssertNotNil(result.relationships)
        XCTAssertNotNil(result.statistics)
    }

    func testNullableDefaults() async throws {
        let json = #"{"entity":{}}"#
        let http = mockHelper(json: json)
        let svc = EntityService(http)
        let result = try await svc.get(id: "x")
        XCTAssertNil(result.entity?.id)
        XCTAssertNil(result.entity?.entityName)
        XCTAssertNil(result.entity?.entityType)
        XCTAssertNil(result.entity?.description)
        XCTAssertNil(result.entity?.sourceId)
        XCTAssertNil(result.entity?.degree)
        XCTAssertNil(result.entity?.metadata)
        XCTAssertNil(result.relationships)
        XCTAssertNil(result.statistics)
    }

    func testListPagination() async throws {
        let json = """
            {"items":[{"id":"e-1"},{"id":"e-2"}],"total":100,"page":2,"pageSize":10,"totalPages":10}
            """
        let http = mockHelper(json: json)
        let svc = EntityService(http)
        let result = try await svc.list(page: 2, pageSize: 10)
        XCTAssertEqual(result.items?.count, 2)
        XCTAssertEqual(result.total, 100)
        XCTAssertEqual(result.page, 2)
        XCTAssertEqual(result.pageSize, 10)
        XCTAssertEqual(result.totalPages, 10)
    }

    func testExistsTrue() async throws {
        let json = #"{"entityId":"e-1","exists":true}"#
        let http = mockHelper(json: json)
        let svc = EntityService(http)
        let result = try await svc.exists(name: "SARAH_CHEN")
        XCTAssertEqual(result.entityId, "e-1")
        XCTAssertEqual(result.exists, true)
    }

    func testExistsFalse() async throws {
        let json = #"{"exists":false}"#
        let http = mockHelper(json: json)
        let svc = EntityService(http)
        let result = try await svc.exists(name: "UNKNOWN")
        XCTAssertNil(result.entityId)
        XCTAssertEqual(result.exists, false)
    }
}

// MARK: - Entity Delete Lineage Tests

final class EntityDeleteLineageTest: XCTestCase {
    func testAffectedEntities() async throws {
        let json = """
            {"status":"deleted","message":"Removed","deletedEntityId":"e-1",
             "deletedRelationships":3,"affectedEntities":["e-2","e-3","e-4"]}
            """
        let http = mockHelper(json: json)
        let svc = EntityService(http)
        let result = try await svc.delete(id: "e-1")
        XCTAssertEqual(result.status, "deleted")
        XCTAssertEqual(result.message, "Removed")
        XCTAssertEqual(result.deletedEntityId, "e-1")
        XCTAssertEqual(result.deletedRelationships, 3)
        XCTAssertEqual(result.affectedEntities, ["e-2", "e-3", "e-4"])
    }

    func testZeroRelationships() async throws {
        let json =
            #"{"status":"deleted","deletedEntityId":"e-1","deletedRelationships":0,"affectedEntities":[]}"#
        let http = mockHelper(json: json)
        let svc = EntityService(http)
        let result = try await svc.delete(id: "e-1")
        XCTAssertEqual(result.deletedRelationships, 0)
        XCTAssertEqual(result.affectedEntities?.count, 0)
    }
}

// MARK: - Create Entity Lineage Tests

final class CreateEntityLineageTest: XCTestCase {
    func testResponseWithEntity() async throws {
        let json = """
            {"status":"created","message":"Entity created","entity":{"id":"e-new","entityName":"ALICE","entityType":"PERSON","description":"Engineer","sourceId":"doc-1"}}
            """
        let http = mockHelper(json: json)
        let svc = EntityService(http)
        let req = CreateEntityRequest(
            entityName: "ALICE", entityType: "PERSON", description: "Engineer", sourceId: "doc-1")
        let result = try await svc.create(request: req)
        XCTAssertEqual(result.status, "created")
        XCTAssertEqual(result.entity?.id, "e-new")
        XCTAssertEqual(result.entity?.entityName, "ALICE")
    }

    func testRequestEncoding() async throws {
        let json = #"{"status":"created"}"#
        let http = mockHelper(json: json)
        let svc = EntityService(http)
        let req = CreateEntityRequest(
            entityName: "BOB", entityType: "ORG", description: "Corp", sourceId: "s1")
        _ = try await svc.create(request: req)
        let body = MockURLProtocol.lastRequest?.body
        XCTAssertNotNil(body)
        // WHY: HttpHelper uses .convertToSnakeCase, so decode with matching strategy.
        let dec = JSONDecoder()
        dec.keyDecodingStrategy = .convertFromSnakeCase
        let decoded = try dec.decode(CreateEntityRequest.self, from: body!)
        XCTAssertEqual(decoded.entityName, "BOB")
        XCTAssertEqual(decoded.entityType, "ORG")
        XCTAssertEqual(decoded.sourceId, "s1")
    }
}

// MARK: - Relationship Lineage Tests

final class RelationshipLineageTest: XCTestCase {
    func testAllFields() async throws {
        let json = """
            {"items":[{"id":"r-1","src_id":"e-1","tgt_id":"e-2","relation_type":"WORKS_WITH",
              "keywords":"colleague","weight":0.85,"description":"Colleagues","source_id":"doc-1",
              "created_at":"2025-01-01T00:00:00Z","updated_at":"2025-01-01T00:00:00Z"}],
             "total":1,"page":1,"page_size":20,"total_pages":1}
            """
        let http = mockHelper(json: json)
        let svc = RelationshipService(http)
        let result = try await svc.list()
        let rel = result.items?.first
        XCTAssertEqual(rel?.id, "r-1")
        XCTAssertEqual(rel?.srcId, "e-1")
        XCTAssertEqual(rel?.tgtId, "e-2")
        XCTAssertEqual(rel?.relationType, "WORKS_WITH")
        XCTAssertEqual(rel?.weight, 0.85)
        XCTAssertEqual(rel?.description, "Colleagues")
        XCTAssertEqual(rel?.sourceId, "doc-1")
        XCTAssertEqual(rel?.createdAt, "2025-01-01T00:00:00Z")
        XCTAssertEqual(result.total, 1)
        XCTAssertEqual(result.totalPages, 1)
    }

    func testPagination() async throws {
        let json = #"{"items":[],"total":200,"page":5,"pageSize":20,"totalPages":10}"#
        let http = mockHelper(json: json)
        let svc = RelationshipService(http)
        let result = try await svc.list(page: 5, pageSize: 20)
        XCTAssertEqual(result.items?.count, 0)
        XCTAssertEqual(result.total, 200)
        XCTAssertEqual(result.page, 5)
        XCTAssertEqual(result.totalPages, 10)
    }
}

// MARK: - Graph Lineage Tests

final class GraphLineageTest: XCTestCase {
    func testNodesWithProperties() async throws {
        let json = """
            {"nodes":[{"id":"n1","label":"ALICE","entityType":"PERSON","properties":{"rank":0.9,"tags":["ai","ml"]}}],
             "edges":[{"source":"n1","target":"n2","label":"KNOWS","weight":0.7}]}
            """
        let http = mockHelper(json: json)
        let svc = GraphService(http)
        let result = try await svc.get()
        let node = result.nodes?.first
        XCTAssertEqual(node?.id, "n1")
        XCTAssertEqual(node?.label, "ALICE")
        XCTAssertEqual(node?.entityType, "PERSON")
        XCTAssertEqual(node?.properties?["rank"]?.value as? Double, 0.9)
        let edge = result.edges?.first
        XCTAssertEqual(edge?.source, "n1")
        XCTAssertEqual(edge?.target, "n2")
        XCTAssertEqual(edge?.label, "KNOWS")
        XCTAssertEqual(edge?.weight, 0.7)
    }

    func testSearchWithTotal() async throws {
        let json = #"{"nodes":[{"id":"n1","label":"BOB"}],"total":42}"#
        let http = mockHelper(json: json)
        let svc = GraphService(http)
        let result = try await svc.search(query: "BOB")
        XCTAssertEqual(result.nodes?.count, 1)
        XCTAssertEqual(result.total, 42)
    }

    func testEmptyGraph() async throws {
        let json = #"{"nodes":[],"edges":[]}"#
        let http = mockHelper(json: json)
        let svc = GraphService(http)
        let result = try await svc.get()
        XCTAssertEqual(result.nodes?.count, 0)
        XCTAssertEqual(result.edges?.count, 0)
    }
}

// MARK: - Document Lineage Tests

final class DocumentLineageTest: XCTestCase {
    func testFullFields() async throws {
        let json = """
            {"id":"d-1","title":"Paper","status":"completed","fileType":"pdf",
             "createdAt":"2025-01-01","updatedAt":"2025-01-02","fileSize":1024,"chunkCount":12}
            """
        let http = mockHelper(json: json)
        let svc = DocumentService(http)
        let result = try await svc.get(id: "d-1")
        XCTAssertEqual(result.id, "d-1")
        XCTAssertEqual(result.title, "Paper")
        XCTAssertEqual(result.status, "completed")
        XCTAssertEqual(result.fileType, "pdf")
        XCTAssertEqual(result.createdAt, "2025-01-01")
        XCTAssertEqual(result.updatedAt, "2025-01-02")
        XCTAssertEqual(result.fileSize, 1024)
        XCTAssertEqual(result.chunkCount, 12)
    }

    func testListAllPaginationFields() async throws {
        let json = """
            {"documents":[{"id":"d-1"}],"total":50,"page":1,"pageSize":20,"totalPages":3,"hasMore":true}
            """
        let http = mockHelper(json: json)
        let svc = DocumentService(http)
        let result = try await svc.list()
        XCTAssertEqual(result.documents?.count, 1)
        XCTAssertEqual(result.total, 50)
        XCTAssertEqual(result.page, 1)
        XCTAssertEqual(result.pageSize, 20)
        XCTAssertEqual(result.totalPages, 3)
        XCTAssertEqual(result.hasMore, true)
    }

    func testUploadDuplicateDetection() async throws {
        let json =
            #"{"documentId":"d-1","status":"duplicate","message":"Already exists","trackId":"t-1","duplicateOf":"d-orig"}"#
        let http = mockHelper(json: json)
        let svc = DocumentService(http)
        let result = try await svc.uploadText(
            request: TextUploadRequest(title: "Dup", content: "test"))
        XCTAssertEqual(result.documentId, "d-1")
        XCTAssertEqual(result.status, "duplicate")
        XCTAssertEqual(result.duplicateOf, "d-orig")
        XCTAssertEqual(result.trackId, "t-1")
        XCTAssertEqual(result.message, "Already exists")
    }

    func testUploadProcessing() async throws {
        let json = #"{"documentId":"d-2","status":"processing","message":"Queued"}"#
        let http = mockHelper(json: json)
        let svc = DocumentService(http)
        let result = try await svc.uploadText(
            request: TextUploadRequest(title: "New", content: "data"))
        XCTAssertEqual(result.status, "processing")
        XCTAssertNil(result.duplicateOf)
    }
}

// MARK: - Pipeline Lineage Tests

final class PipelineLineageTest: XCTestCase {
    func testAllFields() async throws {
        let json = """
            {"isBusy":true,"totalDocuments":100,"processedDocuments":75,
             "pendingTasks":10,"processingTasks":5,"completedTasks":80,"failedTasks":2,
             "cancellationRequested":false}
            """
        let http = mockHelper(json: json)
        let svc = PipelineService(http)
        let result = try await svc.status()
        XCTAssertEqual(result.isBusy, true)
        XCTAssertEqual(result.totalDocuments, 100)
        XCTAssertEqual(result.processedDocuments, 75)
        XCTAssertEqual(result.pendingTasks, 10)
        XCTAssertEqual(result.processingTasks, 5)
        XCTAssertEqual(result.completedTasks, 80)
        XCTAssertEqual(result.failedTasks, 2)
        XCTAssertEqual(result.cancellationRequested, false)
    }

    func testIdleState() async throws {
        let json =
            #"{"isBusy":false,"totalDocuments":0,"processedDocuments":0,"pendingTasks":0,"processingTasks":0,"completedTasks":0,"failedTasks":0,"cancellationRequested":false}"#
        let http = mockHelper(json: json)
        let svc = PipelineService(http)
        let result = try await svc.status()
        XCTAssertEqual(result.isBusy, false)
        XCTAssertEqual(result.totalDocuments, 0)
        XCTAssertEqual(result.failedTasks, 0)
    }

    func testQueueMetricsAllFields() async throws {
        let json = """
            {"pendingCount":5,"processingCount":2,"activeWorkers":3,"maxWorkers":8,
             "workerUtilization":37,"avgWaitTimeSeconds":1.5,"throughputPerMinute":12.0,"rateLimited":false}
            """
        let http = mockHelper(json: json)
        let svc = PipelineService(http)
        let result = try await svc.queueMetrics()
        XCTAssertEqual(result.pendingCount, 5)
        XCTAssertEqual(result.processingCount, 2)
        XCTAssertEqual(result.activeWorkers, 3)
        XCTAssertEqual(result.maxWorkers, 8)
        XCTAssertEqual(result.workerUtilization, 37)
        XCTAssertEqual(result.avgWaitTimeSeconds, 1.5)
        XCTAssertEqual(result.throughputPerMinute, 12.0)
        XCTAssertEqual(result.rateLimited, false)
    }

    func testQueueMetricsRateLimited() async throws {
        let json =
            #"{"pendingCount":100,"processingCount":8,"activeWorkers":8,"maxWorkers":8,"workerUtilization":100,"avgWaitTimeSeconds":30.0,"throughputPerMinute":2.0,"rateLimited":true}"#
        let http = mockHelper(json: json)
        let svc = PipelineService(http)
        let result = try await svc.queueMetrics()
        XCTAssertEqual(result.rateLimited, true)
        XCTAssertEqual(result.workerUtilization, 100)
    }
}

// MARK: - Chat Lineage Tests

final class ChatLineageTest: XCTestCase {
    func testAllFields() async throws {
        let json = """
            {"conversationId":"c-1","userMessageId":"m-1","assistantMessageId":"m-2",
             "content":"Answer here","mode":"hybrid",
             "sources":[{"entity":"e-1","score":0.9}],
             "stats":{"tokens":150},
             "tokensUsed":150,"durationMs":1200,
             "llmProvider":"openai","llmModel":"gpt-5-nano"}
            """
        let http = mockHelper(json: json)
        let svc = ChatService(http)
        let req = ChatCompletionRequest(message: "test")
        let result = try await svc.complete(request: req)
        XCTAssertEqual(result.conversationId, "c-1")
        XCTAssertEqual(result.userMessageId, "m-1")
        XCTAssertEqual(result.assistantMessageId, "m-2")
        XCTAssertEqual(result.content, "Answer here")
        XCTAssertEqual(result.mode, "hybrid")
        XCTAssertNotNil(result.sources)
        XCTAssertEqual(result.sources?.count, 1)
        XCTAssertNotNil(result.stats)
        XCTAssertEqual(result.tokensUsed, 150)
        XCTAssertEqual(result.durationMs, 1200)
        XCTAssertEqual(result.llmProvider, "openai")
        XCTAssertEqual(result.llmModel, "gpt-5-nano")
    }

    func testNoSources() async throws {
        let json = #"{"conversationId":"c-2","content":"No context","sources":[]}"#
        let http = mockHelper(json: json)
        let svc = ChatService(http)
        let req = ChatCompletionRequest(message: "tell me")
        let result = try await svc.complete(request: req)
        XCTAssertEqual(result.sources?.count, 0)
        XCTAssertNil(result.llmProvider)
        XCTAssertNil(result.llmModel)
    }
}

// MARK: - Query Lineage Tests

final class QueryLineageTest: XCTestCase {
    func testSourcesWithLineage() async throws {
        let json = """
            {"answer":"The capital is Paris","sources":[{"entity":"FRANCE","score":0.95,"document":"doc-1"}],"mode":"hybrid"}
            """
        let http = mockHelper(json: json)
        let svc = QueryService(http)
        let req = QueryRequest(query: "capital of France")
        let result = try await svc.query(request: req)
        XCTAssertEqual(result.answer, "The capital is Paris")
        XCTAssertEqual(result.mode, "hybrid")
        XCTAssertNotNil(result.sources)
        XCTAssertEqual(result.sources?.count, 1)
    }

    func testEmptySources() async throws {
        let json = #"{"answer":"I don't know","sources":[],"mode":"naive"}"#
        let http = mockHelper(json: json)
        let svc = QueryService(http)
        let req = QueryRequest(query: "unknown")
        let result = try await svc.query(request: req)
        XCTAssertEqual(result.sources?.count, 0)
        XCTAssertEqual(result.mode, "naive")
    }
}

// MARK: - Cost Lineage Tests

final class CostLineageTest: XCTestCase {
    func testAllFields() async throws {
        let json = """
            {"workspace_id":"ws-1","total_cost":15.50,"document_count":100,"total_tokens":500,
             "average_cost_per_document":0.155,"period_start":"2025-01-01","period_end":"2025-01-31",
             "by_operation":[{"operation":"query","cost":15.5,"percentage":100,"input_tokens":1,
               "output_tokens":2,"total_tokens":3,"call_count":4}]}
            """
        let http = mockHelper(json: json)
        let svc = CostService(http)
        let result = try await svc.summary()
        XCTAssertEqual(result.totalCost, 15.50)
        XCTAssertEqual(result.documentCount, 100)
        XCTAssertEqual(result.totalTokens, 500)
        XCTAssertEqual(result.byOperation?.count, 1)
    }

    func testZeroCost() async throws {
        let json = #"{"workspace_id":"ws-1","total_cost":0.0,"document_count":0,"total_tokens":0,"by_operation":[]}"#
        let http = mockHelper(json: json)
        let svc = CostService(http)
        let result = try await svc.summary()
        XCTAssertEqual(result.totalCost, 0.0)
        XCTAssertTrue(result.byOperation?.isEmpty ?? true)
    }
}

// MARK: - Conversation Lineage Tests

final class ConversationLineageTest: XCTestCase {
    func testDetailFull() async throws {
        let json = """
            {"conversation":{"id":"c-1","tenantId":"t-1","workspaceId":"ws-1",
              "title":"Chat","mode":"hybrid","isPinned":true,"folderId":"f-1",
              "createdAt":"2025-01-01","updatedAt":"2025-01-02","messageCount":5},
             "messages":[{"id":"m-1","conversationId":"c-1","parentId":null,
              "role":"user","content":"Hello","mode":"hybrid","tokensUsed":10,"createdAt":"2025-01-01"}]}
            """
        let http = mockHelper(json: json)
        let svc = ChatService(http)
        let result = try await svc.getConversation(id: "c-1")
        let conv = result.conversation
        XCTAssertEqual(conv?.id, "c-1")
        XCTAssertEqual(conv?.tenantId, "t-1")
        XCTAssertEqual(conv?.workspaceId, "ws-1")
        XCTAssertEqual(conv?.title, "Chat")
        XCTAssertEqual(conv?.mode, "hybrid")
        XCTAssertEqual(conv?.isPinned, true)
        XCTAssertEqual(conv?.folderId, "f-1")
        XCTAssertEqual(conv?.createdAt, "2025-01-01")
        XCTAssertEqual(conv?.updatedAt, "2025-01-02")
        XCTAssertEqual(conv?.messageCount, 5)
        XCTAssertEqual(result.id, "c-1")
        let msg = result.messages?.first
        XCTAssertEqual(msg?.id, "m-1")
        XCTAssertEqual(msg?.conversationId, "c-1")
        XCTAssertNil(msg?.parentId)
        XCTAssertEqual(msg?.role, "user")
        XCTAssertEqual(msg?.content, "Hello")
        XCTAssertEqual(msg?.tokensUsed, 10)
    }

    func testInfoAllFields() async throws {
        let json = """
            {"items":[{"id":"c-1","tenantId":"t-1","workspaceId":"ws-1","title":"Test",
              "mode":"local","isPinned":false,"folderId":null,"createdAt":"2025-01-01",
              "updatedAt":"2025-01-02","messageCount":0}]}
            """
        let http = mockHelper(json: json)
        let svc = ChatService(http)
        let result = try await svc.listConversations()
        let info = result.items?.first
        XCTAssertEqual(info?.id, "c-1")
        XCTAssertEqual(info?.isPinned, false)
        XCTAssertNil(info?.folderId)
        XCTAssertEqual(info?.messageCount, 0)
    }

    func testBulkDelete() async throws {
        let json = #"{"affected":5,"status":"ok"}"#
        let http = mockHelper(json: json)
        let svc = ChatService(http)
        let result = try await svc.bulkDeleteConversations(ids: ["c-1", "c-2", "c-3", "c-4", "c-5"])
        XCTAssertEqual(result.affected, 5)
        XCTAssertEqual(result.status, "ok")
    }
}

// MARK: - Provider Lineage Tests

final class ProviderLineageTest: XCTestCase {
    func testHealthAllFields() async throws {
        let json = """
            {"name":"openai","displayName":"OpenAI","providerType":"cloud",
             "enabled":true,"priority":1,"models":[{"id":"gpt-5-nano","name":"GPT-5 Nano"}]}
            """
        let http = mockHelper(json: json)
        let svc = ModelService(http)
        let result = try await svc.getProvider(name: "openai")
        XCTAssertEqual(result.name, "openai")
        XCTAssertEqual(result.displayName, "OpenAI")
        XCTAssertEqual(result.providerType, "cloud")
        XCTAssertEqual(result.enabled, true)
        XCTAssertEqual(result.priority, 1)
        XCTAssertNotNil(result.models)
        XCTAssertEqual(result.models?.count, 1)
    }

    func testDisabled() async throws {
        let json =
            #"{"name":"local","displayName":"Local","providerType":"local","enabled":false,"priority":99,"models":[]}"#
        let http = mockHelper(json: json)
        let svc = ModelService(http)
        let result = try await svc.getProvider(name: "local")
        XCTAssertEqual(result.enabled, false)
        XCTAssertEqual(result.priority, 99)
        XCTAssertEqual(result.models?.count, 0)
    }

    func testStatusAllSections() async throws {
        let json = """
            {"provider":{"name":"openai"},"embedding":{"model":"text-embedding-3-small"},
             "storage":{"type":"postgresql"},"metadata":{"version":"0.1.0"}}
            """
        let http = mockHelper(json: json)
        let svc = ModelService(http)
        let result = try await svc.status()
        XCTAssertNotNil(result.provider)
        XCTAssertNotNil(result.embedding)
        XCTAssertNotNil(result.storage)
        XCTAssertNotNil(result.metadata)
    }

    func testCatalogWithModels() async throws {
        let json = """
            {"providers":[{"name":"openai","displayName":"OpenAI","models":[{"id":"gpt-5-nano"}]},
                           {"name":"ollama","displayName":"Ollama","models":[]}]}
            """
        let http = mockHelper(json: json)
        let svc = ModelService(http)
        let result = try await svc.catalog()
        XCTAssertEqual(result.providers?.count, 2)
        XCTAssertEqual(result.providers?.first?.name, "openai")
        XCTAssertEqual(result.providers?.first?.models?.count, 1)
        XCTAssertEqual(result.providers?.last?.models?.count, 0)
    }
}

// MARK: - Folder Lineage Tests

final class FolderLineageTest: XCTestCase {
    func testAllFields() async throws {
        let json = """
            [{"id":"f-1","tenantId":"t-1","name":"Research","createdAt":"2025-01-01","updatedAt":"2025-01-02"}]
            """
        let http = mockHelper(json: json)
        let svc = ChatService(http)
        let result = try await svc.listFolders()
        let folder = result.first
        XCTAssertEqual(folder?.id, "f-1")
        XCTAssertEqual(folder?.tenantId, "t-1")
        XCTAssertEqual(folder?.name, "Research")
        XCTAssertEqual(folder?.createdAt, "2025-01-01")
        XCTAssertEqual(folder?.updatedAt, "2025-01-02")
    }
}

// MARK: - Task Lineage Tests

final class TaskLineageTest: XCTestCase {
    func testAllFields() async throws {
        let json = """
            {"tasks":[{"id":"t-1","trackId":"trk-1","status":"completed","taskType":"extraction","createdAt":"2025-01-01"}],"total":1}
            """
        let http = mockHelper(json: json)
        let svc = TaskService(http)
        let result = try await svc.list()
        let task = result.tasks?.first
        XCTAssertEqual(task?.id, "t-1")
        XCTAssertEqual(task?.trackId, "trk-1")
        XCTAssertEqual(task?.status, "completed")
        XCTAssertEqual(task?.taskType, "extraction")
        XCTAssertEqual(task?.createdAt, "2025-01-01")
        XCTAssertEqual(result.total, 1)
    }
}

// MARK: - Health Lineage Tests

final class HealthLineageTest: XCTestCase {
    func testAllFields() async throws {
        let json = """
            {"status":"healthy","version":"0.1.0","storageMode":"postgresql",
             "workspaceId":"default","components":{"kv":true,"vector":true,"graph":true},
             "llmProviderName":"openai"}
            """
        let http = mockHelper(json: json)
        let svc = HealthService(http)
        let result = try await svc.check()
        XCTAssertEqual(result.status, "healthy")
        XCTAssertEqual(result.version, "0.1.0")
        XCTAssertEqual(result.storageMode, "postgresql")
        XCTAssertEqual(result.workspaceId, "default")
        XCTAssertNotNil(result.components)
        XCTAssertEqual(result.llmProviderName, "openai")
    }

    func testNullableDefaults() async throws {
        let json = #"{"status":"healthy"}"#
        let http = mockHelper(json: json)
        let svc = HealthService(http)
        let result = try await svc.check()
        XCTAssertEqual(result.status, "healthy")
        XCTAssertNil(result.version)
        XCTAssertNil(result.storageMode)
        XCTAssertNil(result.workspaceId)
        XCTAssertNil(result.components)
        XCTAssertNil(result.llmProviderName)
    }
}

// MARK: - LineageService Unit Tests

/// WHY: Verify all 7 LineageService methods hit correct endpoints and decode responses.
/// OODA-26: Swift SDK lineage service tests.
final class LineageServiceTest: XCTestCase {

    // -- entityLineage --

    func testEntityLineage() async throws {
        let json = """
            {"entityName":"SARAH_CHEN","entityType":"PERSON",
             "sourceDocuments":[{"documentId":"doc-1","documentTitle":"Paper","chunkIds":["c-1","c-2"],
               "lineRanges":[{"startLine":10,"endLine":15}]}],
             "descriptionVersions":[{"version":1,"description":"AI researcher","sourceChunkId":"c-1","createdAt":"2025-01-01"}],
             "totalSourceDocuments":1,"totalChunks":2}
            """
        let http = mockHelper(json: json)
        let svc = LineageService(http)
        let result = try await svc.entityLineage(name: "SARAH_CHEN")
        XCTAssertEqual(result.entityName, "SARAH_CHEN")
        XCTAssertEqual(result.entityType, "PERSON")
        XCTAssertEqual(result.sourceDocuments?.count, 1)
        XCTAssertEqual(result.sourceDocuments?.first?.documentId, "doc-1")
        XCTAssertEqual(result.sourceDocuments?.first?.chunkIds, ["c-1", "c-2"])
        XCTAssertEqual(result.sourceDocuments?.first?.lineRanges?.count, 1)
        XCTAssertEqual(result.sourceDocuments?.first?.lineRanges?.first?.startLine, 10)
        XCTAssertEqual(result.descriptionVersions?.count, 1)
        XCTAssertEqual(result.descriptionVersions?.first?.version, 1)
        XCTAssertEqual(result.totalSourceDocuments, 1)
        XCTAssertEqual(result.totalChunks, 2)
        // Verify correct URL path
        let req = MockURLProtocol.lastRequest
        XCTAssertEqual(req?.method, "GET")
        XCTAssertTrue(req?.url.contains("/api/v1/lineage/entities/SARAH_CHEN") ?? false)
    }

    func testEntityLineageEmpty() async throws {
        let json =
            #"{"entityName":"UNKNOWN","sourceDocuments":[],"descriptionVersions":[],"totalSourceDocuments":0,"totalChunks":0}"#
        let http = mockHelper(json: json)
        let svc = LineageService(http)
        let result = try await svc.entityLineage(name: "UNKNOWN")
        XCTAssertEqual(result.entityName, "UNKNOWN")
        XCTAssertEqual(result.sourceDocuments?.count, 0)
        XCTAssertEqual(result.totalSourceDocuments, 0)
        XCTAssertEqual(result.totalChunks, 0)
    }

    func testEntityLineageUrlEncoding() async throws {
        let json = #"{"entityName":"HELLO WORLD"}"#
        let http = mockHelper(json: json)
        let svc = LineageService(http)
        _ = try await svc.entityLineage(name: "HELLO WORLD")
        let req = MockURLProtocol.lastRequest
        XCTAssertTrue(req?.url.contains("/api/v1/lineage/entities/HELLO%20WORLD") ?? false)
    }

    // -- documentLineage --

    func testDocumentLineage() async throws {
        let json = """
            {"documentId":"doc-1","documentTitle":"Paper",
             "entities":[{"entityName":"ALICE","entityType":"PERSON","mentions":3}],
             "relationships":[{"source":"ALICE","target":"BOB","type":"WORKS_WITH","mentions":2}],
             "extractionStats":{"totalEntities":5,"totalRelationships":3,"totalChunks":10}}
            """
        let http = mockHelper(json: json)
        let svc = LineageService(http)
        let result = try await svc.documentLineage(id: "doc-1")
        XCTAssertEqual(result.documentId, "doc-1")
        XCTAssertEqual(result.documentTitle, "Paper")
        XCTAssertEqual(result.entities?.count, 1)
        XCTAssertEqual(result.entities?.first?.entityName, "ALICE")
        XCTAssertEqual(result.relationships?.count, 1)
        XCTAssertEqual(result.extractionStats?.totalEntities, 5)
        let req = MockURLProtocol.lastRequest
        XCTAssertTrue(req?.url.contains("/api/v1/lineage/documents/doc-1") ?? false)
    }

    func testDocumentLineageEmpty() async throws {
        let json = #"{"documentId":"doc-x","entities":[],"relationships":[]}"#
        let http = mockHelper(json: json)
        let svc = LineageService(http)
        let result = try await svc.documentLineage(id: "doc-x")
        XCTAssertEqual(result.entities?.count, 0)
        XCTAssertEqual(result.relationships?.count, 0)
    }

    // -- documentFullLineage --

    func testDocumentFullLineage() async throws {
        let json = """
            {"documentId":"doc-1","documentTitle":"Full Paper","status":"completed",
             "chunkCount":12,"entityCount":5,"relationshipCount":3}
            """
        let http = mockHelper(json: json)
        let svc = LineageService(http)
        let result = try await svc.documentFullLineage(id: "doc-1")
        XCTAssertEqual(result.documentId, "doc-1")
        XCTAssertEqual(result.documentTitle, "Full Paper")
        XCTAssertEqual(result.status, "completed")
        XCTAssertEqual(result.chunkCount, 12)
        XCTAssertEqual(result.entityCount, 5)
        XCTAssertEqual(result.relationshipCount, 3)
        let req = MockURLProtocol.lastRequest
        XCTAssertTrue(req?.url.contains("/api/v1/documents/doc-1/lineage") ?? false)
        // Make sure it does NOT contain /export
        XCTAssertFalse(req?.url.contains("export") ?? true)
    }

    // -- exportLineage --

    func testExportLineageRawData() async throws {
        let json = #"{"entities":[{"name":"ALICE"}],"format":"json"}"#
        let http = mockHelper(json: json)
        let svc = LineageService(http)
        let data = try await svc.exportLineage(id: "doc-1")
        let str = String(data: data, encoding: .utf8)!
        XCTAssertTrue(str.contains("ALICE"))
        let req = MockURLProtocol.lastRequest
        XCTAssertTrue(req?.url.contains("/api/v1/documents/doc-1/lineage/export") ?? false)
        XCTAssertTrue(req?.url.contains("format=json") ?? false)
    }

    func testExportLineageCsvFormat() async throws {
        let csv = "entity,type\nALICE,PERSON"
        MockURLProtocol.reset(json: csv)
        let http = mockHelper(json: csv)
        let svc = LineageService(http)
        let data = try await svc.exportLineage(id: "doc-2", format: "csv")
        let str = String(data: data, encoding: .utf8)!
        XCTAssertTrue(str.contains("ALICE"))
        let req = MockURLProtocol.lastRequest
        XCTAssertTrue(req?.url.contains("format=csv") ?? false)
    }

    // -- chunkDetail --

    func testChunkDetail() async throws {
        let json = """
            {"chunkId":"c-1","documentId":"doc-1","content":"Some text content",
             "chunkIndex":0,"charRange":{"start":0,"end":500},
             "entities":[{"entityName":"BOB","entityType":"PERSON","confidence":0.92}],
             "relationships":[{"source":"BOB","target":"CAROL","type":"KNOWS","weight":0.8}]}
            """
        let http = mockHelper(json: json)
        let svc = LineageService(http)
        let result = try await svc.chunkDetail(id: "c-1")
        XCTAssertEqual(result.chunkId, "c-1")
        XCTAssertEqual(result.documentId, "doc-1")
        XCTAssertEqual(result.content, "Some text content")
        XCTAssertEqual(result.chunkIndex, 0)
        XCTAssertEqual(result.charRange?.start, 0)
        XCTAssertEqual(result.charRange?.end, 500)
        XCTAssertEqual(result.entities?.count, 1)
        XCTAssertEqual(result.entities?.first?.entityName, "BOB")
        XCTAssertEqual(result.relationships?.count, 1)
        let req = MockURLProtocol.lastRequest
        XCTAssertTrue(req?.url.contains("/api/v1/chunks/c-1") ?? false)
        // Should NOT contain /lineage
        XCTAssertFalse(req?.url.hasSuffix("/lineage") ?? true)
    }

    func testChunkDetailMinimal() async throws {
        let json = #"{"chunkId":"c-x"}"#
        let http = mockHelper(json: json)
        let svc = LineageService(http)
        let result = try await svc.chunkDetail(id: "c-x")
        XCTAssertEqual(result.chunkId, "c-x")
        XCTAssertNil(result.documentId)
        XCTAssertNil(result.content)
        XCTAssertNil(result.entities)
    }

    // -- chunkLineage --

    func testChunkLineage() async throws {
        let json = """
            {"chunkId":"c-1","documentId":"doc-1","documentTitle":"Paper",
             "chunkIndex":2,"totalChunksInDocument":10}
            """
        let http = mockHelper(json: json)
        let svc = LineageService(http)
        let result = try await svc.chunkLineage(id: "c-1")
        XCTAssertEqual(result.chunkId, "c-1")
        XCTAssertEqual(result.documentId, "doc-1")
        XCTAssertEqual(result.documentTitle, "Paper")
        XCTAssertEqual(result.chunkIndex, 2)
        XCTAssertEqual(result.totalChunksInDocument, 10)
        let req = MockURLProtocol.lastRequest
        XCTAssertTrue(req?.url.contains("/api/v1/chunks/c-1/lineage") ?? false)
    }

    // -- entityProvenance --

    func testEntityProvenance() async throws {
        let json = """
            {"entityId":"e-1","entityName":"ALICE","entityType":"PERSON",
             "sources":[{"documentId":"doc-1","documentTitle":"Paper","chunkId":"c-1","confidence":0.95}],
             "relatedEntities":[{"entityId":"e-2","entityName":"BOB","relationshipType":"WORKS_WITH"}],
             "totalSources":1}
            """
        let http = mockHelper(json: json)
        let svc = LineageService(http)
        let result = try await svc.entityProvenance(id: "e-1")
        XCTAssertEqual(result.entityId, "e-1")
        XCTAssertEqual(result.entityName, "ALICE")
        XCTAssertEqual(result.entityType, "PERSON")
        XCTAssertEqual(result.sources?.count, 1)
        XCTAssertEqual(result.sources?.first?.documentId, "doc-1")
        XCTAssertEqual(result.sources?.first?.confidence, 0.95)
        XCTAssertEqual(result.relatedEntities?.count, 1)
        XCTAssertEqual(result.relatedEntities?.first?.entityName, "BOB")
        XCTAssertEqual(result.totalSources, 1)
        let req = MockURLProtocol.lastRequest
        XCTAssertTrue(req?.url.contains("/api/v1/entities/e-1/provenance") ?? false)
    }

    func testEntityProvenanceMinimal() async throws {
        let json = #"{"entityId":"e-x","entityName":"UNKNOWN","sources":[]}"#
        let http = mockHelper(json: json)
        let svc = LineageService(http)
        let result = try await svc.entityProvenance(id: "e-x")
        XCTAssertEqual(result.entityId, "e-x")
        XCTAssertEqual(result.sources?.count, 0)
        XCTAssertNil(result.relatedEntities)
        XCTAssertNil(result.totalSources)
    }

    // -- Client wiring --

    func testClientHasLineageService() {
        let client = EdgeQuakeClient()
        XCTAssertNotNil(client.lineage)
    }
}

// MARK: - AnyCodable Lineage Tests

final class AnyCodableLineageTest: XCTestCase {
    func testAllValueTypes() throws {
        let json = """
            {"string":"hello","int":42,"double":3.14,"bool":true,"null":null,
             "array":[1,2,3],"object":{"key":"value"}}
            """
        let data = json.data(using: .utf8)!
        let decoded = try JSONDecoder().decode([String: AnyCodable].self, from: data)
        XCTAssertEqual(decoded["string"]?.value as? String, "hello")
        XCTAssertEqual(decoded["int"]?.value as? Int, 42)
        XCTAssertEqual(decoded["double"]?.value as? Double, 3.14)
        XCTAssertEqual(decoded["bool"]?.value as? Bool, true)
        XCTAssertTrue(decoded["null"]?.value is NSNull)
        XCTAssertNotNil(decoded["array"]?.value as? [Any])
        XCTAssertNotNil(decoded["object"]?.value as? [String: Any])
    }

    func testRoundTrip() throws {
        let original: [String: AnyCodable] = [
            "name": AnyCodable("test"),
            "count": AnyCodable(42),
            "score": AnyCodable(0.95),
            "active": AnyCodable(true),
        ]
        let encoded = try JSONEncoder().encode(original)
        let decoded = try JSONDecoder().decode([String: AnyCodable].self, from: encoded)
        XCTAssertEqual(decoded["name"]?.value as? String, "test")
        XCTAssertEqual(decoded["count"]?.value as? Int, 42)
        XCTAssertEqual(decoded["score"]?.value as? Double, 0.95)
        XCTAssertEqual(decoded["active"]?.value as? Bool, true)
    }
}
