package io.edgequake.sdk

import io.edgequake.sdk.internal.HttpHelper
import io.edgequake.sdk.models.*
import io.edgequake.sdk.resources.*
import org.junit.jupiter.api.*
import org.junit.jupiter.api.Assertions.*

/**
 * Comprehensive unit tests for all Kotlin SDK services.
 *
 * Uses FakeHttpClient to intercept HTTP calls and return
 * pre-configured responses without network I/O.
 *
 * Coverage target: 90%+ across all source files.
 */
class UnitTest {

    private lateinit var http: HttpHelper
    private lateinit var fake: FakeHttpClient

    @BeforeEach
    fun setup() {
        val (h, f) = createTestHelper(
            EdgeQuakeConfig(
                baseUrl = "http://test:8080",
                apiKey = "test-key",
                tenantId = "t1",
                userId = "u1",
                workspaceId = "w1"
            )
        )
        http = h
        fake = f
    }

    // ── EdgeQuakeConfig ──────────────────────────────────────────────

    @Test
    fun `config defaults`() {
        val c = EdgeQuakeConfig()
        assertEquals("http://localhost:8080", c.baseUrl)
        assertNull(c.apiKey)
        assertNull(c.tenantId)
        assertNull(c.userId)
        assertNull(c.workspaceId)
        assertEquals(30, c.timeoutSeconds)
    }

    @Test
    fun `config custom values`() {
        val c = EdgeQuakeConfig(
            baseUrl = "http://custom:9090",
            apiKey = "key123",
            tenantId = "tenant1",
            userId = "user1",
            workspaceId = "ws1",
            timeoutSeconds = 60
        )
        assertEquals("http://custom:9090", c.baseUrl)
        assertEquals("key123", c.apiKey)
        assertEquals("tenant1", c.tenantId)
        assertEquals("user1", c.userId)
        assertEquals("ws1", c.workspaceId)
        assertEquals(60, c.timeoutSeconds)
    }

    @Test
    fun `config data class equality`() {
        val c1 = EdgeQuakeConfig(baseUrl = "http://a")
        val c2 = EdgeQuakeConfig(baseUrl = "http://a")
        assertEquals(c1, c2)
        assertEquals(c1.hashCode(), c2.hashCode())
    }

    // ── EdgeQuakeException ───────────────────────────────────────────

    @Test
    fun `exception properties`() {
        val ex = EdgeQuakeException("test error", 404, """{"detail":"not found"}""")
        assertEquals("test error", ex.message)
        assertEquals(404, ex.statusCode)
        assertEquals("""{"detail":"not found"}""", ex.responseBody)
        assertNull(ex.cause)
    }

    @Test
    fun `exception with cause`() {
        val cause = RuntimeException("root cause")
        val ex = EdgeQuakeException("wrapped", 500, cause = cause)
        assertEquals(cause, ex.cause)
        assertEquals(500, ex.statusCode)
    }

    @Test
    fun `exception defaults`() {
        val ex = EdgeQuakeException("msg")
        assertEquals(0, ex.statusCode)
        assertNull(ex.responseBody)
    }

    // ── EdgeQuakeClient ──────────────────────────────────────────────

    @Test
    fun `client creates all services`() {
        val client = EdgeQuakeClient()
        assertNotNull(client.health)
        assertNotNull(client.documents)
        assertNotNull(client.entities)
        assertNotNull(client.relationships)
        assertNotNull(client.graph)
        assertNotNull(client.query)
        assertNotNull(client.chat)
        assertNotNull(client.auth)
        assertNotNull(client.users)
        assertNotNull(client.apiKeys)
        assertNotNull(client.tenants)
        assertNotNull(client.conversations)
        assertNotNull(client.folders)
        assertNotNull(client.tasks)
        assertNotNull(client.pipeline)
        assertNotNull(client.models)
        assertNotNull(client.workspaces)
        assertNotNull(client.pdf)
        assertNotNull(client.costs)
    }

    // ── HttpHelper ───────────────────────────────────────────────────

    @Test
    fun `buildRequest includes headers`() {
        val config = EdgeQuakeConfig(
            baseUrl = "http://test:8080",
            apiKey = "my-key",
            tenantId = "t1",
            userId = "u1",
            workspaceId = "w1"
        )
        val helper = HttpHelper(config)
        val req = helper.buildRequest("/api/v1/health", "GET", null)
        assertEquals("http://test:8080/api/v1/health", req.uri().toString())
        assertEquals("GET", req.method())
        assertTrue(req.headers().map()["X-API-Key"]?.contains("my-key") == true)
        assertTrue(req.headers().map()["X-Tenant-ID"]?.contains("t1") == true)
        assertTrue(req.headers().map()["X-User-ID"]?.contains("u1") == true)
        assertTrue(req.headers().map()["X-Workspace-ID"]?.contains("w1") == true)
    }

    @Test
    fun `buildRequest skips null headers`() {
        val config = EdgeQuakeConfig(baseUrl = "http://test:8080")
        val helper = HttpHelper(config)
        val req = helper.buildRequest("/test", "GET", null)
        assertFalse(req.headers().map().containsKey("X-API-Key"))
        assertFalse(req.headers().map().containsKey("X-Tenant-ID"))
    }

    @Test
    fun `buildRequest POST with body`() {
        val config = EdgeQuakeConfig(baseUrl = "http://test:8080")
        val helper = HttpHelper(config)
        val req = helper.buildRequest("/test", "POST", mapOf("key" to "value"))
        assertEquals("POST", req.method())
        assertTrue(req.bodyPublisher().isPresent)
    }

    @Test
    fun `buildRequest POST without body sends empty json`() {
        val config = EdgeQuakeConfig(baseUrl = "http://test:8080")
        val helper = HttpHelper(config)
        val req = helper.buildRequest("/test", "POST", null)
        assertTrue(req.bodyPublisher().isPresent)
    }

    @Test
    fun `buildRequest PUT with body`() {
        val config = EdgeQuakeConfig(baseUrl = "http://test:8080")
        val helper = HttpHelper(config)
        val req = helper.buildRequest("/test", "PUT", mapOf("a" to 1))
        assertEquals("PUT", req.method())
    }

    @Test
    fun `buildRequest PATCH without body sends empty json`() {
        val config = EdgeQuakeConfig(baseUrl = "http://test:8080")
        val helper = HttpHelper(config)
        val req = helper.buildRequest("/test", "PATCH", null)
        assertEquals("PATCH", req.method())
    }

    @Test
    fun `buildRequest DELETE has no body`() {
        val config = EdgeQuakeConfig(baseUrl = "http://test:8080")
        val helper = HttpHelper(config)
        val req = helper.buildRequest("/test", "DELETE", null)
        assertEquals("DELETE", req.method())
    }

    @Test
    fun `execute throws EdgeQuakeException on error status`() {
        fake.respondWithError(500, """{"error":"internal"}""")
        assertThrows(EdgeQuakeException::class.java) {
            http.get<Map<String, Any?>>("/test")
        }
    }

    @Test
    fun `execute wraps unexpected exceptions`() {
        fake.throwOnSend(RuntimeException("network error"))
        val ex = assertThrows(EdgeQuakeException::class.java) {
            http.get<Map<String, Any?>>("/test")
        }
        assertTrue(ex.message!!.contains("network error"))
    }

    @Test
    fun `getRaw throws on error status`() {
        fake.respondWithError(404)
        assertThrows(EdgeQuakeException::class.java) {
            http.getRaw("/test")
        }
    }

    @Test
    fun `postRaw throws on error status`() {
        fake.respondWithError(422)
        assertThrows(EdgeQuakeException::class.java) {
            http.postRaw("/test", mapOf("a" to 1))
        }
    }

    @Test
    fun `deleteRaw throws on error status`() {
        fake.respondWithError(403)
        assertThrows(EdgeQuakeException::class.java) {
            http.deleteRaw("/test")
        }
    }

    @Test
    fun `put method works`() {
        fake.respondWith("""{"ok":true}""")
        val result: Map<String, Any?> = http.put("/test", mapOf("x" to 1))
        assertEquals(true, result["ok"])
        assertEquals("PUT", fake.lastRequest().method)
    }

    @Test
    fun `patch method works`() {
        fake.respondWith("""{"ok":true}""")
        val result: Map<String, Any?> = http.patch("/test", mapOf("x" to 1))
        assertEquals(true, result["ok"])
        assertEquals("PATCH", fake.lastRequest().method)
    }

    @Test
    fun `delete method works`() {
        fake.respondWith("""{"deleted":true}""")
        val result: Map<String, Any?> = http.delete("/test")
        assertEquals(true, result["deleted"])
        assertEquals("DELETE", fake.lastRequest().method)
    }

    // ── HealthService ────────────────────────────────────────────────

    @Test
    fun `health check returns parsed response`() {
        fake.respondWith("""{"status":"healthy","version":"0.1.0","storage_mode":"postgresql","workspace_id":"default","llm_provider_name":"ollama"}""")
        val svc = HealthService(http)
        val health = svc.check()
        assertEquals("healthy", health.status)
        assertEquals("0.1.0", health.version)
        assertEquals("postgresql", health.storageMode)
        assertEquals("ollama", health.llmProviderName)
    }

    @Test
    fun `health check error`() {
        fake.respondWithError(503, """{"error":"unavailable"}""")
        val svc = HealthService(http)
        assertThrows(EdgeQuakeException::class.java) { svc.check() }
    }

    // ── DocumentService ──────────────────────────────────────────────

    @Test
    fun `documents list`() {
        fake.respondWith("""{"documents":[{"id":"d1","title":"Test","status":"completed"}],"total":1,"page":1,"page_size":20}""")
        val svc = DocumentService(http)
        val result = svc.list()
        assertEquals(1, result.total)
        assertEquals("d1", result.documents?.first()?.id)
    }

    @Test
    fun `documents list with pagination`() {
        fake.respondWith("""{"documents":[],"total":50,"page":3,"page_size":10,"has_more":true}""")
        val svc = DocumentService(http)
        val result = svc.list(page = 3, pageSize = 10)
        assertEquals(50, result.total)
        assertEquals(3, result.page)
        assertTrue(result.hasMore == true)
        assertTrue(fake.lastRequest().uri.contains("page=3"))
        assertTrue(fake.lastRequest().uri.contains("page_size=10"))
    }

    @Test
    fun `documents get by id`() {
        fake.respondWith("""{"id":"d1","title":"My Doc","status":"completed","file_type":"txt","chunk_count":5}""")
        val svc = DocumentService(http)
        val doc = svc.get("d1")
        assertEquals("d1", doc.id)
        assertEquals("My Doc", doc.title)
        assertEquals(5, doc.chunkCount)
    }

    @Test
    fun `documents upload text`() {
        fake.respondWith("""{"document_id":"d-new","status":"processing","message":"Upload received","track_id":"t-123"}""")
        val svc = DocumentService(http)
        val result = svc.uploadText("Test Title", "Hello World")
        assertEquals("d-new", result.documentId)
        assertEquals("processing", result.status)
        assertEquals("t-123", result.trackId)
        assertTrue(fake.lastRequest().uri.contains("/api/v1/documents"))
    }

    @Test
    fun `documents delete`() {
        fake.respondWith("")
        val svc = DocumentService(http)
        svc.delete("d1")
        assertTrue(fake.lastRequest().uri.contains("/api/v1/documents/d1"))
    }

    @Test
    fun `documents scan`() {
        fake.respondWith("""{"status":"ok","message":"Scan completed","files_found":3}""")
        val svc = DocumentService(http)
        val result = svc.scan("/path/to/dir")
        assertEquals(3, result.filesFound)
    }

    @Test
    fun `documents error handling`() {
        fake.respondWithError(404)
        val svc = DocumentService(http)
        assertThrows(EdgeQuakeException::class.java) { svc.get("nonexistent") }
    }

    // ── EntityService ────────────────────────────────────────────────

    @Test
    fun `entities list`() {
        fake.respondWith("""{"items":[{"entity_name":"ALICE","entity_type":"PERSON"}],"total":1,"page":1,"page_size":20}""")
        val svc = EntityService(http)
        val result = svc.list()
        assertEquals(1, result.total)
        assertEquals("ALICE", result.items?.first()?.entityName)
    }

    @Test
    fun `entities get detail`() {
        fake.respondWith("""{"entity":{"entity_name":"BOB","entity_type":"PERSON","description":"A person"},"relationships":{}}""")
        val svc = EntityService(http)
        val result = svc.get("BOB")
        assertEquals("BOB", result.entity?.entityName)
    }

    @Test
    fun `entities create`() {
        fake.respondWith("""{"status":"success","message":"Created","entity":{"entity_name":"NEW_ENTITY","entity_type":"TEST"}}""")
        val svc = EntityService(http)
        val result = svc.create(CreateEntityRequest("NEW_ENTITY", "TEST", "desc", "src"))
        assertEquals("success", result.status)
        assertEquals("NEW_ENTITY", result.entity?.entityName)
    }

    @Test
    fun `entities delete`() {
        fake.respondWith("""{"status":"success","deleted_entity_id":"e1","deleted_relationships":3}""")
        val svc = EntityService(http)
        val result = svc.delete("TEST_ENTITY")
        assertEquals("success", result.status)
        assertEquals(3, result.deletedRelationships)
    }

    @Test
    fun `entities exists`() {
        fake.respondWith("""{"entity_id":"e1","exists":true}""")
        val svc = EntityService(http)
        val result = svc.exists("ALICE")
        assertTrue(result.exists == true)
        assertEquals("e1", result.entityId)
    }

    @Test
    fun `entities merge`() {
        fake.respondWith("""{"status":"merged"}""")
        val svc = EntityService(http)
        val result = svc.merge("SOURCE", "TARGET")
        assertEquals("merged", result["status"])
    }

    @Test
    fun `entities error`() {
        fake.respondWithError(404)
        val svc = EntityService(http)
        assertThrows(EdgeQuakeException::class.java) { svc.get("NONEXISTENT") }
    }

    // ── RelationshipService ──────────────────────────────────────────

    @Test
    fun `relationships list`() {
        fake.respondWith("""{"items":[{"source":"A","target":"B","relationship_type":"KNOWS","weight":1.0}],"total":1}""")
        val svc = RelationshipService(http)
        val result = svc.list()
        assertEquals(1, result.total)
        assertEquals("A", result.items?.first()?.source)
        assertEquals("KNOWS", result.items?.first()?.relationshipType)
    }

    @Test
    fun `relationships list with pagination`() {
        fake.respondWith("""{"items":[],"total":0}""")
        val svc = RelationshipService(http)
        svc.list(page = 2, pageSize = 5)
        assertTrue(fake.lastRequest().uri.contains("page=2"))
    }

    // ── GraphService ─────────────────────────────────────────────────

    @Test
    fun `graph get`() {
        fake.respondWith("""{"nodes":[{"id":"n1","label":"Alice","entity_type":"PERSON"}],"edges":[{"source":"n1","target":"n2","label":"KNOWS"}]}""")
        val svc = GraphService(http)
        val result = svc.get()
        assertEquals(1, result.nodes?.size)
        assertEquals("Alice", result.nodes?.first()?.label)
        assertEquals(1, result.edges?.size)
    }

    @Test
    fun `graph search`() {
        fake.respondWith("""{"nodes":[{"id":"n1","label":"Result"}],"total":1}""")
        val svc = GraphService(http)
        val result = svc.search("test")
        assertEquals(1, result.total)
        assertTrue(fake.lastRequest().uri.contains("q=test"))
    }

    @Test
    fun `graph error`() {
        fake.respondWithError(500)
        val svc = GraphService(http)
        assertThrows(EdgeQuakeException::class.java) { svc.get() }
    }

    // ── QueryService ─────────────────────────────────────────────────

    @Test
    fun `query execute`() {
        fake.respondWith("""{"answer":"The answer is 42.","sources":[{"title":"doc1"}],"mode":"hybrid"}""")
        val svc = QueryService(http)
        val result = svc.execute("What is the answer?")
        assertEquals("The answer is 42.", result.answer)
        assertEquals("hybrid", result.mode)
        assertEquals(1, result.sources?.size)
    }

    @Test
    fun `query with mode`() {
        fake.respondWith("""{"answer":"Local answer","mode":"local"}""")
        val svc = QueryService(http)
        val result = svc.execute("test", mode = "local")
        assertEquals("local", result.mode)
    }

    @Test
    fun `query error`() {
        fake.respondWithError(422)
        val svc = QueryService(http)
        assertThrows(EdgeQuakeException::class.java) { svc.execute("") }
    }

    // ── ChatService ──────────────────────────────────────────────────

    @Test
    fun `chat completions`() {
        fake.respondWith("""{"conversation_id":"conv-1","user_message_id":"msg-1","assistant_message_id":"msg-2","content":"Hello!","mode":"hybrid","sources":[]}""")
        val svc = ChatService(http)
        val result = svc.completions(
            ChatCompletionRequest(
                message = "Hi"
            )
        )
        assertEquals("conv-1", result.conversationId)
        assertEquals("Hello!", result.content)
    }

    @Test
    fun `chat error`() {
        fake.respondWithError(500)
        val svc = ChatService(http)
        assertThrows(EdgeQuakeException::class.java) {
            svc.completions(ChatCompletionRequest(message = "Hi"))
        }
    }

    // ── AuthService ──────────────────────────────────────────────────

    @Test
    fun `auth login`() {
        fake.respondWith("""{"token":"jwt-token-123","expires_at":"2026-12-31T23:59:59Z"}""")
        val svc = AuthService(http)
        val result = svc.login("admin", "password")
        assertEquals("jwt-token-123", result.token)
        assertNotNull(result.expiresAt)
    }

    @Test
    fun `auth login error`() {
        fake.respondWithError(401)
        val svc = AuthService(http)
        assertThrows(EdgeQuakeException::class.java) { svc.login("bad", "creds") }
    }

    // ── UserService ──────────────────────────────────────────────────

    @Test
    fun `users list`() {
        fake.respondWith("""{"users":[{"id":"u1","username":"admin","email":"a@b.com","role":"admin"}]}""")
        val svc = UserService(http)
        val result = svc.list()
        assertEquals(1, result.users?.size)
        assertEquals("admin", result.users?.first()?.username)
    }

    @Test
    fun `users error`() {
        fake.respondWithError(403)
        val svc = UserService(http)
        assertThrows(EdgeQuakeException::class.java) { svc.list() }
    }

    // ── ApiKeyService ────────────────────────────────────────────────

    @Test
    fun `api keys list`() {
        fake.respondWith("""{"keys":[{"id":"k1","name":"my-key","prefix":"sk-abc"}]}""")
        val svc = ApiKeyService(http)
        val result = svc.list()
        assertEquals(1, result.keys?.size)
        assertEquals("my-key", result.keys?.first()?.name)
    }

    // ── TenantService ────────────────────────────────────────────────

    @Test
    fun `tenants list`() {
        fake.respondWith("""{"items":[{"id":"t1","name":"Default","slug":"default"}]}""")
        val svc = TenantService(http)
        val result = svc.list()
        assertEquals(1, result.items?.size)
        assertEquals("Default", result.items?.first()?.name)
    }

    // ── ConversationService ──────────────────────────────────────────

    @Test
    fun `conversations list`() {
        fake.respondWith("""{"items":[{"id":"c1","title":"Test Chat","message_count":5}]}""")
        val svc = ConversationService(http)
        val result = svc.list()
        assertEquals(1, result.size)
        assertEquals("Test Chat", result.first().title)
    }

    @Test
    fun `conversations create`() {
        fake.respondWith("""{"id":"c-new","title":"New Chat"}""")
        val svc = ConversationService(http)
        val result = svc.create("New Chat")
        assertEquals("c-new", result.id)
        assertEquals("New Chat", result.title)
    }

    @Test
    fun `conversations get`() {
        fake.respondWith("""{"conversation":{"id":"c1","title":"Chat"},"messages":[{"id":"m1","role":"user","content":"Hello"}]}""")
        val svc = ConversationService(http)
        val result = svc.get("c1")
        assertEquals("c1", result.conversation?.id)
        assertEquals(1, result.messages?.size)
    }

    @Test
    fun `conversations delete`() {
        fake.respondWith("")
        val svc = ConversationService(http)
        svc.delete("c1")
        assertTrue(fake.lastRequest().uri.contains("/api/v1/conversations/c1"))
    }

    @Test
    fun `conversations bulk delete`() {
        fake.respondWith("""{"affected":3,"status":"success"}""")
        val svc = ConversationService(http)
        val result = svc.bulkDelete(listOf("c1", "c2", "c3"))
        assertEquals(3, result.affected)
        assertTrue(fake.lastRequest().body?.contains("conversation_ids") == true)
    }

    @Test
    fun `conversations error`() {
        fake.respondWithError(404)
        val svc = ConversationService(http)
        assertThrows(EdgeQuakeException::class.java) { svc.get("nonexistent") }
    }

    // ── FolderService ────────────────────────────────────────────────

    @Test
    fun `folders list`() {
        fake.respondWith("""[{"id":"f1","name":"My Folder"}]""")
        val svc = FolderService(http)
        val result = svc.list()
        assertEquals(1, result.size)
        assertEquals("My Folder", result.first().name)
    }

    @Test
    fun `folders create`() {
        fake.respondWith("""{"id":"f-new","name":"New Folder"}""")
        val svc = FolderService(http)
        val result = svc.create("New Folder")
        assertEquals("f-new", result.id)
    }

    @Test
    fun `folders delete`() {
        fake.respondWith("")
        val svc = FolderService(http)
        svc.delete("f1")
        assertTrue(fake.lastRequest().uri.contains("/api/v1/folders/f1"))
    }

    // ── TaskService ──────────────────────────────────────────────────

    @Test
    fun `tasks list`() {
        fake.respondWith("""{"tasks":[{"id":"t1","status":"completed","task_type":"extraction"}],"total":1}""")
        val svc = TaskService(http)
        val result = svc.list()
        assertEquals(1, result.total)
        assertEquals("completed", result.tasks?.first()?.status)
    }

    @Test
    fun `tasks get`() {
        fake.respondWith("""{"id":"t1","status":"running","task_type":"ingestion","progress":0.5}""")
        val svc = TaskService(http)
        val result = svc.get("t1")
        assertEquals("t1", result.id)
        assertEquals("running", result.status)
    }

    @Test
    fun `tasks error`() {
        fake.respondWithError(404)
        val svc = TaskService(http)
        assertThrows(EdgeQuakeException::class.java) { svc.get("nonexistent") }
    }

    // ── PipelineService ──────────────────────────────────────────────

    @Test
    fun `pipeline status`() {
        fake.respondWith("""{"is_busy":false,"total_documents":10,"processed_documents":8,"pending_tasks":2,"processing_tasks":0,"completed_tasks":8,"failed_tasks":0}""")
        val svc = PipelineService(http)
        val result = svc.status()
        assertEquals(false, result.isBusy)
        assertEquals(10, result.totalDocuments)
        assertEquals(2, result.pendingTasks)
    }

    @Test
    fun `pipeline queue metrics`() {
        fake.respondWith("""{"pending_count":5,"processing_count":2,"active_workers":3,"max_workers":8,"worker_utilization":37,"throughput_per_minute":12.5}""")
        val svc = PipelineService(http)
        val result = svc.queueMetrics()
        assertEquals(5, result.pendingCount)
        assertEquals(3, result.activeWorkers)
        assertEquals(12.5, result.throughputPerMinute)
    }

    // ── ModelService ─────────────────────────────────────────────────

    @Test
    fun `models catalog`() {
        fake.respondWith("""{"providers":[{"name":"ollama","display_name":"Ollama","models":[{"id":"llama3"}]}]}""")
        val svc = ModelService(http)
        val result = svc.catalog()
        assertEquals(1, result.providers?.size)
        assertEquals("ollama", result.providers?.first()?.name)
    }

    @Test
    fun `models health`() {
        fake.respondWith("""[{"name":"ollama","display_name":"Ollama","enabled":true,"priority":1}]""")
        val svc = ModelService(http)
        val result = svc.health()
        assertEquals(1, result.size)
        assertEquals(true, result.first().enabled)
    }

    @Test
    fun `models provider status`() {
        fake.respondWith("""{"provider":{"name":"ollama"},"embedding":{"name":"ollama"},"storage":{"mode":"postgresql"}}""")
        val svc = ModelService(http)
        val result = svc.providerStatus()
        assertNotNull(result.provider)
        assertNotNull(result.embedding)
    }

    @Test
    fun `models error`() {
        fake.respondWithError(500)
        val svc = ModelService(http)
        assertThrows(EdgeQuakeException::class.java) { svc.catalog() }
    }

    // ── WorkspaceService ─────────────────────────────────────────────

    @Test
    fun `workspaces list`() {
        fake.respondWith("""{"items":[{"id":"w1","name":"Default","slug":"default"}],"total":1}""")
        val svc = WorkspaceService(http)
        val result = svc.list("t1")
        assertEquals(1, result.size)
        assertEquals("Default", result.first().name)
        assertTrue(fake.lastRequest().uri.contains("/api/v1/tenants/t1/workspaces"))
    }

    @Test
    fun `workspaces error`() {
        fake.respondWithError(403)
        val svc = WorkspaceService(http)
        assertThrows(EdgeQuakeException::class.java) { svc.list("t1") }
    }

    // ── PdfService ───────────────────────────────────────────────────

    @Test
    fun `pdf progress`() {
        fake.respondWith("""{"track_id":"tk-1","status":"processing","progress":0.75}""")
        val svc = PdfService(http)
        val result = svc.progress("tk-1")
        assertEquals("tk-1", result.trackId)
        assertEquals("processing", result.status)
    }

    @Test
    fun `pdf content`() {
        fake.respondWith("""{"content":"# Title\n\nHello world","page_count":3}""")
        val svc = PdfService(http)
        val result = svc.content("pdf-1")
        assertTrue(result.content?.contains("Hello world") == true)
        assertEquals(3, result.pageCount)
    }

    @Test
    fun `pdf error`() {
        fake.respondWithError(404)
        val svc = PdfService(http)
        assertThrows(EdgeQuakeException::class.java) { svc.progress("nonexistent") }
    }

    // ── CostService ──────────────────────────────────────────────────

    @Test
    fun `costs summary`() {
        fake.respondWith("""{"total_cost":12.50,"document_count":100,"query_count":500,"entries":[]}""")
        val svc = CostService(http)
        val result = svc.summary()
        assertEquals(12.50, result.totalCost)
        assertEquals(100, result.documentCount)
        assertEquals(500, result.queryCount)
    }

    @Test
    fun `costs error`() {
        fake.respondWithError(403)
        val svc = CostService(http)
        assertThrows(EdgeQuakeException::class.java) { svc.summary() }
    }

    // ── Model data classes ───────────────────────────────────────────

    @Test
    fun `Document model defaults`() {
        val d = Document()
        assertNull(d.id)
        assertNull(d.title)
        assertNull(d.status)
    }

    @Test
    fun `Entity model`() {
        val e = Entity(entityName = "TEST", entityType = "PERSON", description = "desc")
        assertEquals("TEST", e.entityName)
        assertEquals("PERSON", e.entityType)
    }

    @Test
    fun `Relationship model`() {
        val r = Relationship(source = "A", target = "B", weight = 0.8)
        assertEquals("A", r.source)
        assertEquals(0.8, r.weight)
    }

    @Test
    fun `ChatMessage model`() {
        val m = ChatMessage(role = "user", content = "Hello")
        assertEquals("user", m.role)
        assertEquals("Hello", m.content)
    }

    @Test
    fun `ChatCompletionRequest defaults`() {
        val r = ChatCompletionRequest(message = "Hi")
        assertEquals(false, r.stream)
        assertEquals("Hi", r.message)
    }

    @Test
    fun `QueryRequest defaults`() {
        val q = QueryRequest(query = "test")
        assertEquals("hybrid", q.mode)
    }

    @Test
    fun `WorkspaceInfo model`() {
        val w = WorkspaceInfo(id = "w1", name = "Test")
        assertEquals("w1", w.id)
    }

    @Test
    fun `TaskInfo model`() {
        val t = TaskInfo(id = "t1", status = "running", taskType = "extraction")
        assertEquals("extraction", t.taskType)
    }

    @Test
    fun `PipelineStatus model`() {
        val p = PipelineStatus(isBusy = true, pendingTasks = 5)
        assertTrue(p.isBusy == true)
        assertEquals(5, p.pendingTasks)
    }

    @Test
    fun `QueueMetrics model`() {
        val q = QueueMetrics(pendingCount = 3, activeWorkers = 2)
        assertEquals(3, q.pendingCount)
    }

    @Test
    fun `ProviderCatalog model`() {
        val c = ProviderCatalog(providers = listOf(ProviderInfo(name = "ollama")))
        assertEquals(1, c.providers?.size)
    }

    @Test
    fun `UploadResponse model`() {
        val u = UploadResponse(documentId = "d1", status = "processing")
        assertEquals("d1", u.documentId)
    }

    @Test
    fun `ScanResponse model`() {
        val s = ScanResponse(filesFound = 5, status = "ok")
        assertEquals(5, s.filesFound)
    }

    @Test
    fun `BulkDeleteResponse model`() {
        val b = BulkDeleteResponse(affected = 3, status = "success")
        assertEquals(3, b.affected)
    }

    @Test
    fun `ConversationDetail model`() {
        val c = ConversationDetail(
            conversation = ConversationInfo(id = "c1", title = "Chat"),
            messages = listOf(Message(id = "m1", role = "user", content = "Hi"))
        )
        assertEquals(1, c.messages?.size)
        assertEquals("c1", c.conversation?.id)
    }

    @Test
    fun `CostSummary model`() {
        val c = CostSummary(totalCost = 5.0, documentCount = 10)
        assertEquals(5.0, c.totalCost)
    }

    @Test
    fun `PdfProgressResponse model`() {
        val p = PdfProgressResponse(trackId = "t1", status = "complete")
        assertEquals("t1", p.trackId)
    }

    @Test
    fun `PdfContentResponse model`() {
        val p = PdfContentResponse(content = "hello", pageCount = 2)
        assertEquals(2, p.pageCount)
    }

    // ── Request capture verification ─────────────────────────────────

    @Test
    fun `requests hit correct endpoints`() {
        fake.respondWith("""{"status":"healthy"}""")
        HealthService(http).check()
        assertTrue(fake.lastRequest().uri.contains("/health"))

        fake.respondWith("""{"documents":[],"total":0}""")
        DocumentService(http).list()
        assertTrue(fake.lastRequest().uri.contains("/api/v1/documents"))

        fake.respondWith("""{"items":[],"total":0}""")
        EntityService(http).list()
        assertTrue(fake.lastRequest().uri.contains("/api/v1/graph/entities"))

        fake.respondWith("""{"items":[],"total":0}""")
        RelationshipService(http).list()
        assertTrue(fake.lastRequest().uri.contains("/api/v1/graph/relationships"))

        fake.respondWith("""{"nodes":[],"edges":[]}""")
        GraphService(http).get()
        assertTrue(fake.lastRequest().uri.contains("/api/v1/graph"))
    }

    @Test
    fun `all request methods used correctly`() {
        fake.respondWith("""{"status":"healthy"}""")
        HealthService(http).check()
        assertEquals("GET", fake.lastRequest().method)

        fake.respondWith("""{"answer":"ok"}""")
        QueryService(http).execute("test")
        assertEquals("POST", fake.lastRequest().method)

        fake.respondWith("""{"deleted":true}""")
        DocumentService(http).delete("d1")
        assertEquals("DELETE", fake.lastRequest().method)
    }

    // ── Lineage & Metadata Tests ─────────────────────────────────────
    // WHY: The improve-lineage mission requires source_id, metadata,
    // and provenance fields to be properly tested across all SDKs.

    @Test
    fun `entity model has sourceId`() {
        val e = Entity(entityName = "ALICE", entityType = "person", sourceId = "doc-123")
        assertEquals("doc-123", e.sourceId)
    }

    @Test
    fun `entity model has metadata`() {
        val e = Entity(entityName = "BOB", metadata = mapOf("key" to "value"))
        assertNotNull(e.metadata)
        assertEquals("value", e.metadata!!["key"])
    }

    @Test
    fun `entity model has timestamps`() {
        val e = Entity(entityName = "EVE", createdAt = "2025-01-01T00:00:00Z", updatedAt = "2025-01-02T00:00:00Z")
        assertNotNull(e.createdAt)
        assertNotNull(e.updatedAt)
    }

    @Test
    fun `createEntityRequest includes sourceId`() {
        val req = CreateEntityRequest(entityName = "ALICE", entityType = "person", description = "A researcher", sourceId = "doc-456")
        assertEquals("doc-456", req.sourceId)
    }

    @Test
    fun `entity create sends sourceId in request body`() {
        fake.respondWith("""{"status":"success","message":"created"}""")
        val req = CreateEntityRequest(entityName = "ALICE", entityType = "person", description = "test", sourceId = "doc-lineage-1")
        EntityService(http).create(req)
        val body = fake.lastRequest().body
        assertTrue(body?.contains("doc-lineage-1") == true)
    }

    @Test
    fun `relationship model has sourceId`() {
        val r = Relationship(source = "A", target = "B", relationshipType = "KNOWS", sourceId = "doc-rel-1")
        assertEquals("doc-rel-1", r.sourceId)
    }

    @Test
    fun `relationship model has createdAt`() {
        val r = Relationship(source = "A", target = "B", createdAt = "2025-01-01T00:00:00Z")
        assertNotNull(r.createdAt)
    }

    @Test
    fun `entityDeleteResponse has lineage info`() {
        val del = EntityDeleteResponse(status = "deleted", deletedRelationships = 5, affectedEntities = listOf("e2", "e3"))
        assertEquals(5, del.deletedRelationships)
        assertEquals(2, del.affectedEntities?.size)
    }

    @Test
    fun `graphNode has properties for provenance`() {
        val node = GraphNode(id = "n1", label = "ALICE", properties = mapOf("source_document" to "doc-1"))
        assertEquals("doc-1", node.properties!!["source_document"])
    }

    @Test
    fun `graphEdge has weight for lineage scoring`() {
        val edge = GraphEdge(source = "A", target = "B", label = "COLLAB", weight = 0.85)
        assertEquals(0.85, edge.weight)
    }

    @Test
    fun `uploadResponse contains lineage documentId`() {
        val u = UploadResponse(documentId = "doc-up-1", status = "processing")
        assertEquals("doc-up-1", u.documentId)
    }

    @Test
    fun `chatCompletionRequest has conversationId for lineage`() {
        val req = ChatCompletionRequest(message = "Hello", conversationId = "conv-1", parentId = "msg-parent-1")
        assertEquals("conv-1", req.conversationId)
        assertEquals("msg-parent-1", req.parentId)
    }

    @Test
    fun `conversation has createdAt and updatedAt timestamps`() {
        val conv = ConversationInfo(id = "c1", title = "Test", createdAt = "2025-01-01T00:00:00Z", updatedAt = "2025-01-02T00:00:00Z")
        assertNotNull(conv.createdAt)
        assertNotNull(conv.updatedAt)
    }

    @Test
    fun `message has createdAt timestamp`() {
        val msg = Message(role = "user", content = "Hi", createdAt = "2025-01-01T00:00:00Z")
        assertNotNull(msg.createdAt)
    }

    @Test
    fun `folder has parentId for hierarchy lineage`() {
        val f = FolderInfo(id = "f1", name = "Root", parentId = "f0", createdAt = "2025-01-01T00:00:00Z")
        assertEquals("f0", f.parentId)
    }

    @Test
    fun `document has chunk count for lineage`() {
        val d = Document(id = "d1", title = "test.pdf", chunkCount = 42)
        assertEquals(42, d.chunkCount)
    }

    @Test
    fun `entityListResponse has pagination for lineage queries`() {
        val resp = EntityListResponse(items = emptyList(), total = 100, page = 2, pageSize = 20, totalPages = 5)
        assertEquals(100, resp.total)
        assertEquals(5, resp.totalPages)
    }

    @Test
    fun `entityExistsResponse has exists flag`() {
        val resp = EntityExistsResponse(exists = true, entityId = "ent-1")
        assertEquals(true, resp.exists)
        assertEquals("ent-1", resp.entityId)
    }

    @Test
    fun `mergeEntities sends source and target`() {
        fake.respondWith("""{"merged_entity":{"entity_name":"ALICE"},"merged_count":2}""")
        EntityService(http).merge("ALICE_1", "ALICE_2")
        val body = fake.lastRequest().body
        assertTrue(body?.contains("source_name") == true)
        assertTrue(body?.contains("target_name") == true)
        assertTrue(body?.contains("ALICE_1") == true)
        assertTrue(body?.contains("ALICE_2") == true)
    }

    @Test
    fun `providerStatus has metadata`() {
        val ps = ProviderStatus(metadata = mapOf("version" to "1.0"))
        assertNotNull(ps.metadata)
        assertEquals("1.0", ps.metadata!!["version"])
    }

    @Test
    fun `bulkDeleteResponse has deleted count`() {
        val resp = BulkDeleteResponse(deleted = 3)
        assertEquals(3, resp.deleted)
    }

    @Test
    fun `taskInfo has createdAt for lineage tracking`() {
        val task = TaskInfo(id = "t1", status = "running", createdAt = "2025-01-01T00:00:00Z")
        assertNotNull(task.createdAt)
    }

    @Test
    fun `costSummary tracks usage lineage`() {
        val c = CostSummary(totalCost = 5.0, documentCount = 10)
        assertEquals(5.0, c.totalCost)
        assertEquals(10, c.documentCount)
    }

    // ── LineageService Endpoint Tests ────────────────────────────────

    @Test
    fun `lineageService entityLineage hits correct endpoint`() {
        fake.respondWith("""{"entity_name":"ALICE","entity_type":"PERSON","source_documents":[],"source_count":0,"description_versions":[]}""")
        val svc = LineageService(http)
        val res = svc.entityLineage("ALICE")
        assertEquals("ALICE", res.entityName)
        assertTrue(fake.lastRequest().uri.contains("/api/v1/lineage/entities/ALICE"))
    }

    @Test
    fun `lineageService entityLineage URL-encodes special chars`() {
        fake.respondWith("""{"entity_name":"HELLO WORLD","source_count":0}""")
        val svc = LineageService(http)
        svc.entityLineage("HELLO WORLD")
        assertTrue(fake.lastRequest().uri.contains("HELLO+WORLD") || fake.lastRequest().uri.contains("HELLO%20WORLD"))
    }

    @Test
    fun `lineageService documentLineage hits correct endpoint`() {
        fake.respondWith("""{"document_id":"doc-1","chunk_count":5,"entities":[],"relationships":[],"extraction_stats":{"total_entities":10,"unique_entities":5,"total_relationships":8,"unique_relationships":4}}""")
        val svc = LineageService(http)
        val res = svc.documentLineage("doc-1")
        assertEquals("doc-1", res.documentId)
        assertEquals(5, res.chunkCount)
        assertTrue(fake.lastRequest().uri.contains("/api/v1/lineage/documents/doc-1"))
    }

    @Test
    fun `lineageService documentFullLineage hits correct endpoint`() {
        fake.respondWith("""{"document_id":"doc-1","metadata":{"title":"Test"},"lineage":{"entities_extracted":15}}""")
        val svc = LineageService(http)
        val res = svc.documentFullLineage("doc-1")
        assertEquals("doc-1", res.documentId)
        assertNotNull(res.metadata)
        assertTrue(fake.lastRequest().uri.contains("/api/v1/documents/doc-1/lineage"))
    }

    @Test
    fun `lineageService exportLineage json format`() {
        fake.respondWith("""{"format":"json","data":[]}""")
        val svc = LineageService(http)
        val res = svc.exportLineage("doc-1", "json")
        assertTrue(fake.lastRequest().uri.contains("/api/v1/documents/doc-1/lineage/export?format=json"))
        assertEquals("json", res["format"])
    }

    @Test
    fun `lineageService exportLineage csv format`() {
        fake.respondWith("""{"format":"csv","data":""}""")
        val svc = LineageService(http)
        val res = svc.exportLineage("doc-1", "csv")
        assertTrue(fake.lastRequest().uri.contains("format=csv"))
    }

    @Test
    fun `lineageService chunkDetail hits correct endpoint`() {
        fake.respondWith("""{"chunk_id":"c1","document_id":"d1","content":"text","index":0,"char_range":{"start":0,"end":4},"token_count":1,"entities":[],"relationships":[]}""")
        val svc = LineageService(http)
        val res = svc.chunkDetail("c1")
        assertEquals("c1", res.chunkId)
        assertTrue(fake.lastRequest().uri.contains("/api/v1/chunks/c1"))
    }

    @Test
    fun `lineageService chunkLineage hits correct endpoint`() {
        fake.respondWith("""{"chunk_id":"c1","document_id":"d1","index":3,"start_line":42,"end_line":60}""")
        val svc = LineageService(http)
        val res = svc.chunkLineage("c1")
        assertEquals("c1", res.chunkId)
        assertEquals(42, res.startLine)
        assertTrue(fake.lastRequest().uri.contains("/api/v1/chunks/c1/lineage"))
    }

    @Test
    fun `lineageService entityProvenance hits correct endpoint`() {
        fake.respondWith("""{"entity_id":"e1","entity_name":"ALICE","entity_type":"PERSON","sources":[],"total_extraction_count":5,"related_entities":[]}""")
        val svc = LineageService(http)
        val res = svc.entityProvenance("e1")
        assertEquals("ALICE", res.entityName)
        assertEquals(5, res.totalExtractionCount)
        assertTrue(fake.lastRequest().uri.contains("/api/v1/entities/e1/provenance"))
    }

    // ── LineageModels Unit Tests ─────────────────────────────────────

    @Test
    fun `EntityLineageResponse fields`() {
        val r = EntityLineageResponse(entityName = "BOB", entityType = "PERSON", sourceCount = 3)
        assertEquals("BOB", r.entityName)
        assertEquals("PERSON", r.entityType)
        assertEquals(3, r.sourceCount)
    }

    @Test
    fun `SourceDocumentInfo fields`() {
        val s = SourceDocumentInfo(documentId = "d1", chunkIds = listOf("c1", "c2"), lineRanges = listOf(LineRangeInfo(1, 10)))
        assertEquals("d1", s.documentId)
        assertEquals(2, s.chunkIds?.size)
        assertEquals(1, s.lineRanges?.size)
    }

    @Test
    fun `LineRangeInfo fields`() {
        val lr = LineRangeInfo(startLine = 10, endLine = 20)
        assertEquals(10, lr.startLine)
        assertEquals(20, lr.endLine)
    }

    @Test
    fun `DescriptionVersionResponse fields`() {
        val dv = DescriptionVersionResponse(version = 2, description = "Updated", sourceChunkId = "c1", createdAt = "2026-01-01T00:00:00Z")
        assertEquals(2, dv.version)
        assertEquals("Updated", dv.description)
        assertEquals("c1", dv.sourceChunkId)
    }

    @Test
    fun `DocumentGraphLineageResponse fields`() {
        val d = DocumentGraphLineageResponse(documentId = "d1", chunkCount = 10, entities = emptyList(), relationships = emptyList())
        assertEquals("d1", d.documentId)
        assertEquals(10, d.chunkCount)
        assertTrue(d.entities!!.isEmpty())
    }

    @Test
    fun `EntitySummaryResponse fields`() {
        val e = EntitySummaryResponse(name = "ALICE", entityType = "PERSON", sourceChunks = listOf("c1"), isShared = true)
        assertEquals("ALICE", e.name)
        assertTrue(e.isShared == true)
    }

    @Test
    fun `RelationshipSummaryResponse fields`() {
        val r = RelationshipSummaryResponse(source = "A", target = "B", keywords = "KNOWS", sourceChunks = listOf("c1"))
        assertEquals("A", r.source)
        assertEquals("B", r.target)
        assertEquals("KNOWS", r.keywords)
    }

    @Test
    fun `ExtractionStatsResponse fields`() {
        val s = ExtractionStatsResponse(totalEntities = 10, uniqueEntities = 5, totalRelationships = 8, uniqueRelationships = 4, processingTimeMs = 1500)
        assertEquals(10, s.totalEntities)
        assertEquals(1500, s.processingTimeMs)
    }

    @Test
    fun `ChunkDetailResponse fields`() {
        val c = ChunkDetailResponse(chunkId = "c1", documentId = "d1", content = "text", index = 0, tokenCount = 10)
        assertEquals("c1", c.chunkId)
        assertEquals(10, c.tokenCount)
    }

    @Test
    fun `CharRange fields`() {
        val cr = CharRange(start = 0, end = 100)
        assertEquals(0, cr.start)
        assertEquals(100, cr.end)
    }

    @Test
    fun `ExtractedEntityInfo fields`() {
        val e = ExtractedEntityInfo(id = "e1", name = "ALICE", entityType = "PERSON", description = "A researcher")
        assertEquals("e1", e.id)
        assertEquals("A researcher", e.description)
    }

    @Test
    fun `ExtractedRelationshipInfo fields`() {
        val r = ExtractedRelationshipInfo(sourceName = "A", targetName = "B", relationType = "KNOWS")
        assertEquals("A", r.sourceName)
        assertEquals("B", r.targetName)
    }

    @Test
    fun `ExtractionMetadataInfo fields`() {
        val m = ExtractionMetadataInfo(model = "gpt-4o", gleaningIterations = 2, durationMs = 1200, inputTokens = 500, outputTokens = 300, cached = false)
        assertEquals("gpt-4o", m.model)
        assertEquals(false, m.cached)
    }

    @Test
    fun `EntityProvenanceResponse fields`() {
        val p = EntityProvenanceResponse(entityId = "e1", entityName = "ALICE", totalExtractionCount = 5)
        assertEquals("e1", p.entityId)
        assertEquals(5, p.totalExtractionCount)
    }

    @Test
    fun `EntitySourceInfo fields`() {
        val s = EntitySourceInfo(documentId = "d1", documentName = "Paper.pdf", chunks = emptyList(), firstExtractedAt = "2026-01-01T00:00:00Z")
        assertEquals("d1", s.documentId)
        assertNotNull(s.firstExtractedAt)
    }

    @Test
    fun `ChunkSourceInfo fields`() {
        val cs = ChunkSourceInfo(chunkId = "c1", startLine = 10, endLine = 15, sourceText = "Alice...")
        assertEquals("c1", cs.chunkId)
        assertEquals("Alice...", cs.sourceText)
    }

    @Test
    fun `RelatedEntityInfo fields`() {
        val rel = RelatedEntityInfo(entityId = "e2", entityName = "BOB", relationshipType = "KNOWS", sharedDocuments = 3)
        assertEquals(3, rel.sharedDocuments)
    }

    @Test
    fun `DocumentFullLineageResponse fields`() {
        val fl = DocumentFullLineageResponse(documentId = "d1", metadata = mapOf("title" to "Test"), lineage = mapOf("pipeline" to "v1"))
        assertEquals("d1", fl.documentId)
        assertEquals("Test", fl.metadata?.get("title"))
    }

    @Test
    fun `ChunkLineageResponse fields`() {
        val cl = ChunkLineageResponse(chunkId = "c1", documentId = "d1", documentName = "test.pdf", documentType = "pdf", index = 3, startLine = 42, endLine = 60, tokenCount = 150, entityCount = 3, entityNames = listOf("ALICE", "BOB"))
        assertEquals("c1", cl.chunkId)
        assertEquals(3, cl.entityCount)
        assertEquals(listOf("ALICE", "BOB"), cl.entityNames)
    }

    // ── Lineage Edge Cases ──────────────────────────────────────────

    @Test
    fun `lineageService error handling returns EdgeQuakeException`() {
        fake.respondWithError(404, """{"error":"Entity not found"}""")
        val svc = LineageService(http)
        assertThrows<EdgeQuakeException> { svc.entityLineage("UNKNOWN") }
    }

    @Test
    fun `lineageModels null defaults`() {
        val empty = EntityLineageResponse()
        assertNull(empty.entityName)
        assertNull(empty.entityType)
        assertNull(empty.sourceDocuments)
        assertNull(empty.sourceCount)
        assertNull(empty.descriptionVersions)
    }

    @Test
    fun `chunkLineageResponse minimal construction`() {
        val cl = ChunkLineageResponse(chunkId = "c1")
        assertNull(cl.documentId)
        assertNull(cl.entityCount)
        assertNull(cl.entityNames)
    }

    @Test
    fun `entityProvenance with empty sources and related`() {
        val p = EntityProvenanceResponse(entityId = "e1", sources = emptyList(), relatedEntities = emptyList())
        assertTrue(p.sources!!.isEmpty())
        assertTrue(p.relatedEntities!!.isEmpty())
    }

    @Test
    fun `lineageService accessible from EdgeQuakeClient`() {
        val client = EdgeQuakeClient(EdgeQuakeConfig(baseUrl = "http://test:8080"))
        assertNotNull(client.lineage)
    }

    // ── OODA-37: Extended Service Tests ──────────────────────────────
    // WHY: Adding comprehensive tests for all new service methods added in OODA-37

    // ── Health Extended Tests ────────────────────────────────────────

    @Test
    fun `health ready`() {
        fake.respondWith("""{"ready":true,"checks":{"database":"ok","provider":"ok"}}""")
        val svc = HealthService(http)
        val result = svc.ready()
        assertEquals(true, result.ready)
        assertTrue(fake.lastRequest().uri.contains("/ready"))
    }

    @Test
    fun `health live`() {
        fake.respondWith("""{"alive":true,"uptime":12345}""")
        val svc = HealthService(http)
        val result = svc.live()
        assertEquals(true, result.alive)
        assertTrue(fake.lastRequest().uri.contains("/live"))
    }

    @Test
    fun `health metrics`() {
        fake.respondWith("# TYPE edgequake_requests_total counter\nedgequake_requests_total 100")
        val svc = HealthService(http)
        val result = svc.metrics()
        assertTrue(result.contains("edgequake_requests_total"))
        assertTrue(fake.lastRequest().uri.contains("/metrics"))
    }

    // ── Document Extended Tests ──────────────────────────────────────

    @Test
    fun `documents track`() {
        fake.respondWith("""{"track_id":"tk1","status":"processing","progress":0.5}""")
        val svc = DocumentService(http)
        val result = svc.track("tk1")
        assertEquals("tk1", result.trackId)
        assertTrue(fake.lastRequest().uri.contains("/api/v1/documents/track/tk1"))
    }

    @Test
    fun `documents reprocessFailed`() {
        fake.respondWith("""{"status":"ok","message":"Reprocessing started"}""")
        val svc = DocumentService(http)
        val result = svc.reprocessFailed()
        assertEquals("ok", result.status)
        assertTrue(fake.lastRequest().uri.contains("/api/v1/documents/reprocess"))
    }

    @Test
    fun `documents recoverStuck`() {
        fake.respondWith("""{"status":"ok","message":"Recovered 3 documents"}""")
        val svc = DocumentService(http)
        val result = svc.recoverStuck()
        assertEquals("ok", result.status)
    }

    // ── Entity Extended Tests ────────────────────────────────────────

    @Test
    fun `entities neighborhood`() {
        fake.respondWith(
            """{"nodes":[{"id":"ALICE","entity_type":"PERSON","degree":2},{"id":"BOB","entity_type":"PERSON","degree":1}],"edges":[{"id":"e1","source":"ALICE","target":"BOB","relation_type":"KNOWS"}]}"""
        )
        val svc = EntityService(http)
        val result = svc.neighborhood("ALICE")
        assertEquals("ALICE", result.nodes?.first()?.id)
        assertEquals(1, result.edges?.size)
        assertTrue(fake.lastRequest().uri.contains("neighborhood"))
    }

    // ── Relationship Extended Tests ──────────────────────────────────

    @Test
    fun `relationships get`() {
        fake.respondWith("""{"relationship":{"id":"r1","source":"A","target":"B"},"source":{"entity_name":"A"},"target":{"entity_name":"B"}}""")
        val svc = RelationshipService(http)
        val result = svc.get("r1")
        assertEquals("r1", result.relationship?.id)
    }

    @Test
    fun `relationships create`() {
        fake.respondWith("""{"status":"created","relationship":{"id":"r-new","source":"X","target":"Y"}}""")
        val svc = RelationshipService(http)
        val result = svc.create(CreateRelationshipRequest("X", "Y", "KNOWS"))
        assertEquals("created", result.status)
    }

    @Test
    fun `relationships delete`() {
        fake.respondWith("")
        val svc = RelationshipService(http)
        svc.delete("r1")
        assertTrue(fake.lastRequest().uri.contains("/api/v1/graph/relationships/r1"))
    }

    // ── Graph Extended Tests ─────────────────────────────────────────

    @Test
    fun `graph labelSearch`() {
        fake.respondWith("""{"labels":["PERSON","PERSON_NAME"]}""")
        val svc = GraphService(http)
        val result = svc.labelSearch("PER")
        assertTrue(result.labels?.contains("PERSON") == true)
    }

    @Test
    fun `graph popularLabels`() {
        fake.respondWith("""{"labels":[{"label":"PERSON","degree":3,"entity_type":"x","description":""}],"total_entities":10}""")
        val svc = GraphService(http)
        val result = svc.popularLabels()
        assertEquals(1, result.labels?.size)
        assertEquals(10, result.totalEntities)
    }

    @Test
    fun `graph batchDegrees`() {
        fake.respondWith("""{"degrees":[{"node_id":"node1","degree":5},{"node_id":"node2","degree":3}],"count":2}""")
        val svc = GraphService(http)
        val result = svc.batchDegrees(listOf("node1", "node2"))
        assertEquals(5, result.degrees?.firstOrNull { it.nodeId == "node1" }?.degree)
    }

    // ── Query Extended Tests ─────────────────────────────────────────

    @Test
    fun `query stream`() {
        fake.respondWith("data: {\"chunk\":\"Hello\"}\n\n")
        val svc = QueryService(http)
        val result = svc.stream("test query")
        assertTrue(result.contains("Hello"))
    }

    // ── Chat Extended Tests ──────────────────────────────────────────

    @Test
    fun `chat stream`() {
        fake.respondWith("data: {\"delta\":\"world\"}\n\n")
        val svc = ChatService(http)
        val result = svc.stream(ChatCompletionRequest(message = "Hi"))
        assertTrue(result.contains("world"))
    }

    @Test
    fun `chat completionsWithConversation`() {
        fake.respondWith("""{"conversation_id":"c1","content":"Response"}""")
        val svc = ChatService(http)
        val result = svc.completionsWithConversation("c1", "Hello")
        assertEquals("c1", result.conversationId)
    }

    // ── Auth Extended Tests ──────────────────────────────────────────

    @Test
    fun `auth logout`() {
        fake.respondWith("")
        val svc = AuthService(http)
        svc.logout()
        assertTrue(fake.lastRequest().uri.contains("/api/v1/auth/logout"))
    }

    @Test
    fun `auth refresh`() {
        fake.respondWith("""{"token":"new-token","expires_at":"2027-01-01T00:00:00Z"}""")
        val svc = AuthService(http)
        val result = svc.refresh()
        assertEquals("new-token", result.token)
    }

    @Test
    fun `auth me`() {
        fake.respondWith("""{"id":"u1","username":"admin","email":"admin@test.com","role":"admin","permissions":["read","write"]}""")
        val svc = AuthService(http)
        val result = svc.me()
        assertEquals("admin", result.username)
    }

    @Test
    fun `auth changePassword`() {
        fake.respondWith("""{"status":"ok","message":"Password changed"}""")
        val svc = AuthService(http)
        val result = svc.changePassword("old", "new")
        assertEquals("ok", result.status)
    }

    // ── User Extended Tests ──────────────────────────────────────────

    @Test
    fun `users get`() {
        fake.respondWith("""{"id":"u1","username":"john","email":"john@test.com"}""")
        val svc = UserService(http)
        val result = svc.get("u1")
        assertEquals("john", result.username)
    }

    @Test
    fun `users create`() {
        fake.respondWith("""{"id":"u-new","username":"jane","email":"jane@test.com","role":"user"}""")
        val svc = UserService(http)
        val result = svc.create("jane", "jane@test.com", "password123")
        assertEquals("jane", result.username)
    }

    @Test
    fun `users update`() {
        fake.respondWith("""{"id":"u1","username":"john","email":"new@test.com"}""")
        val svc = UserService(http)
        val result = svc.update("u1", mapOf("email" to "new@test.com"))
        assertEquals("new@test.com", result.email)
    }

    @Test
    fun `users delete`() {
        fake.respondWith("")
        val svc = UserService(http)
        svc.delete("u1")
        assertTrue(fake.lastRequest().uri.contains("/api/v1/users/u1"))
    }

    // ── ApiKey Extended Tests ────────────────────────────────────────

    @Test
    fun `apiKeys get`() {
        fake.respondWith("""{"id":"k1","name":"test-key","prefix":"sk-abc"}""")
        val svc = ApiKeyService(http)
        val result = svc.get("k1")
        assertEquals("test-key", result.name)
    }

    @Test
    fun `apiKeys create`() {
        fake.respondWith("""{"id":"k-new","name":"new-key","key":"sk-full-key"}""")
        val svc = ApiKeyService(http)
        val result = svc.create("new-key")
        assertEquals("sk-full-key", result.key)
    }

    @Test
    fun `apiKeys revoke`() {
        fake.respondWith("")
        val svc = ApiKeyService(http)
        svc.revoke("k1")
        assertTrue(fake.lastRequest().uri.contains("/api/v1/api-keys/k1"))
    }

    @Test
    fun `apiKeys rotate`() {
        fake.respondWith("""{"id":"k1","key":"sk-new-rotated"}""")
        val svc = ApiKeyService(http)
        val result = svc.rotate("k1")
        assertEquals("sk-new-rotated", result.key)
    }

    // ── Tenant Extended Tests ────────────────────────────────────────

    @Test
    fun `tenants get`() {
        fake.respondWith("""{"id":"t1","name":"Acme","slug":"acme"}""")
        val svc = TenantService(http)
        val result = svc.get("t1")
        assertEquals("Acme", result.name)
    }

    @Test
    fun `tenants create`() {
        fake.respondWith("""{"id":"t-new","name":"NewCorp","slug":"newcorp"}""")
        val svc = TenantService(http)
        val result = svc.create("NewCorp", "newcorp")
        assertEquals("NewCorp", result.name)
    }

    @Test
    fun `tenants update`() {
        fake.respondWith("""{"id":"t1","name":"Updated","slug":"updated"}""")
        val svc = TenantService(http)
        val result = svc.update("t1", mapOf("name" to "Updated"))
        assertEquals("Updated", result.name)
    }

    @Test
    fun `tenants delete`() {
        fake.respondWith("")
        val svc = TenantService(http)
        svc.delete("t1")
        assertTrue(fake.lastRequest().uri.contains("/api/v1/tenants/t1"))
    }

    // ── Conversation Extended Tests ──────────────────────────────────

    @Test
    fun `conversations update`() {
        fake.respondWith("""{"id":"c1","title":"Updated Title"}""")
        val svc = ConversationService(http)
        val result = svc.update("c1", "Updated Title")
        assertEquals("Updated Title", result.title)
        assertEquals("PATCH", fake.lastRequest().method)
    }

    @Test
    fun `conversations messages`() {
        fake.respondWith(
            """{"items":[{"id":"m1","role":"user","content":"Hi"}],"pagination":{"total":1,"has_more":false}}"""
        )
        val svc = ConversationService(http)
        val result = svc.messages("c1")
        assertEquals(1, result.items?.size)
        assertEquals(1, result.pagination?.total)
    }

    @Test
    fun `conversations addMessage`() {
        fake.respondWith("""{"id":"m-new","role":"user","content":"Hello"}""")
        val svc = ConversationService(http)
        val result = svc.addMessage("c1", "user", "Hello")
        assertEquals("user", result.role)
    }

    @Test
    fun `conversations deleteMessage`() {
        fake.respondWith("")
        val svc = ConversationService(http)
        svc.deleteMessage("m1")
        assertTrue(fake.lastRequest().uri.contains("/api/v1/messages/m1"))
    }

    @Test
    fun `conversations share`() {
        fake.respondWith("""{"share_id":"s1","share_url":"/api/v1/shared/s1"}""")
        val svc = ConversationService(http)
        val result = svc.share("c1")
        assertEquals("s1", result.shareId)
        assertEquals("/api/v1/shared/s1", result.shareUrl)
    }

    @Test
    fun `conversations unshare`() {
        fake.respondWith("")
        val svc = ConversationService(http)
        svc.unshare("c1")
        assertTrue(fake.lastRequest().uri.contains("/api/v1/conversations/c1/share"))
        assertEquals("DELETE", fake.lastRequest().method)
    }

    // ── Folder Extended Tests ────────────────────────────────────────

    @Test
    fun `folders update`() {
        fake.respondWith("""{"id":"f1","name":"Renamed"}""")
        val svc = FolderService(http)
        val result = svc.update("f1", "Renamed")
        assertEquals("Renamed", result.name)
        assertEquals("PATCH", fake.lastRequest().method)
    }

    // ── Task Extended Tests ──────────────────────────────────────────

    @Test
    fun `tasks create`() {
        fake.respondWith("""{"id":"t-new","status":"pending","task_type":"extraction"}""")
        val svc = TaskService(http)
        val result = svc.create("extraction")
        assertEquals("pending", result.status)
    }

    @Test
    fun `tasks cancel`() {
        fake.respondWith("""{"status":"cancelled"}""")
        val svc = TaskService(http)
        val result = svc.cancel("t1")
        assertEquals("cancelled", result.status)
    }

    @Test
    fun `tasks status`() {
        fake.respondWith("""{"status":"running","progress":0.5}""")
        val svc = TaskService(http)
        val result = svc.status("t1")
        assertEquals("running", result.status)
    }

    @Test
    fun `tasks retry`() {
        fake.respondWith("""{"id":"t1","status":"pending"}""")
        val svc = TaskService(http)
        val result = svc.retry("t1")
        assertEquals("pending", result.status)
    }

    // ── Pipeline Extended Tests ──────────────────────────────────────

    @Test
    fun `pipeline processing`() {
        fake.respondWith("""{"items":[{"id":"p1","status":"processing","document_id":"d1"}],"total":1}""")
        val svc = PipelineService(http)
        val result = svc.processing()
        assertEquals(1, result.total)
    }

    @Test
    fun `pipeline pause`() {
        fake.respondWith("""{"status":"paused"}""")
        val svc = PipelineService(http)
        val result = svc.pause()
        assertEquals("paused", result.status)
    }

    @Test
    fun `pipeline resume`() {
        fake.respondWith("""{"status":"resumed"}""")
        val svc = PipelineService(http)
        val result = svc.resume()
        assertEquals("resumed", result.status)
    }

    @Test
    fun `pipeline cancel`() {
        fake.respondWith("""{"status":"cancelled"}""")
        val svc = PipelineService(http)
        val result = svc.cancel()
        assertEquals("cancelled", result.status)
    }

    @Test
    fun `pipeline costEstimate`() {
        fake.respondWith("""{"estimated_cost":2.50,"token_count":5000,"model_used":"gpt-4o"}""")
        val svc = PipelineService(http)
        val result = svc.costEstimate(10)
        assertEquals(2.50, result.estimatedCost)
    }

    // ── Model Extended Tests ─────────────────────────────────────────

    @Test
    fun `models list`() {
        fake.respondWith("""{"models":[{"id":"m1","name":"GPT-4","provider":"openai"}],"total":1}""")
        val svc = ModelService(http)
        val result = svc.list()
        assertEquals(1, result.total)
    }

    @Test
    fun `models get`() {
        fake.respondWith("""{"id":"m1","name":"GPT-4","provider":"openai","context_length":8192}""")
        val svc = ModelService(http)
        val result = svc.get("m1")
        assertEquals("GPT-4", result.name)
    }

    @Test
    fun `models providers`() {
        fake.respondWith("""{"providers":[{"id":"p1","name":"OpenAI","enabled":true}]}""")
        val svc = ModelService(http)
        val result = svc.providers()
        assertEquals(1, result.providers?.size)
    }

    @Test
    fun `models setDefault`() {
        fake.respondWith("""{"status":"ok"}""")
        val svc = ModelService(http)
        val result = svc.setDefault("openai", "gpt-4")
        assertEquals("ok", result.status)
    }

    @Test
    fun `models test`() {
        fake.respondWith("""{"success":true,"response_time_ms":150}""")
        val svc = ModelService(http)
        val result = svc.test("m1")
        assertEquals(true, result.success)
    }

    // ── Workspace Extended Tests ─────────────────────────────────────

    @Test
    fun `workspaces get`() {
        fake.respondWith("""{"id":"w1","name":"Main","slug":"main"}""")
        val svc = WorkspaceService(http)
        val result = svc.get("w1")
        assertEquals("Main", result.name)
    }

    @Test
    fun `workspaces create`() {
        fake.respondWith("""{"id":"w-new","name":"NewWS","slug":"newws"}""")
        val svc = WorkspaceService(http)
        val result = svc.create("t1", "NewWS", "newws")
        assertEquals("NewWS", result.name)
        assertTrue(fake.lastRequest().uri.contains("/api/v1/tenants/t1/workspaces"))
    }

    @Test
    fun `workspaces update`() {
        fake.respondWith("""{"id":"w1","name":"Updated"}""")
        val svc = WorkspaceService(http)
        val result = svc.update("w1", mapOf("name" to "Updated"))
        assertEquals("Updated", result.name)
    }

    @Test
    fun `workspaces delete`() {
        fake.respondWith("")
        val svc = WorkspaceService(http)
        svc.delete("w1")
        assertTrue(fake.lastRequest().uri.contains("/api/v1/workspaces/w1"))
    }

    @Test
    fun `workspaces stats`() {
        fake.respondWith("""{"workspace_id":"w1","document_count":50,"entity_count":100,"relationship_count":150}""")
        val svc = WorkspaceService(http)
        val result = svc.stats("w1")
        assertEquals(50, result.documentCount)
    }

    @Test
    fun `workspaces rebuildEmbeddings`() {
        fake.respondWith("""{"status":"rebuilding"}""")
        val svc = WorkspaceService(http)
        val result = svc.rebuildEmbeddings("w1")
        assertEquals("rebuilding", result.status)
        assertTrue(fake.lastRequest().uri.contains("/api/v1/workspaces/w1/rebuild-embeddings"))
    }

    // ── Cost Extended Tests ──────────────────────────────────────────

    @Test
    fun `costs daily`() {
        fake.respondWith("""{"date":"2026-01-15","cost":5.50,"breakdown":{"llm":4.0,"embedding":1.5}}""")
        val svc = CostService(http)
        val result = svc.daily()
        assertEquals(5.50, result.cost)
    }

    @Test
    fun `costs byProvider`() {
        fake.respondWith("""{"providers":{"openai":10.0,"ollama":0.0},"total":10.0}""")
        val svc = CostService(http)
        val result = svc.byProvider()
        assertEquals(10.0, result.total)
    }

    @Test
    fun `costs byModel`() {
        fake.respondWith("""{"models":{"gpt-4":8.0,"gpt-3.5":2.0},"total":10.0}""")
        val svc = CostService(http)
        val result = svc.byModel()
        assertEquals(10.0, result.total)
    }

    @Test
    fun `costs history`() {
        fake.respondWith("""{"history":[{"date":"2026-01-14","cost":3.0},{"date":"2026-01-15","cost":5.0}],"total":8.0}""")
        val svc = CostService(http)
        val result = svc.history("2026-01-14", "2026-01-15")
        assertEquals(2, result.history?.size)
    }

    @Test
    fun `costs export`() {
        fake.respondWith("date,cost\n2026-01-15,5.50")
        val svc = CostService(http)
        val result = svc.export("csv")
        assertTrue(result.contains("date,cost"))
    }

    @Test
    fun `costs budget`() {
        fake.respondWith("""{"amount":100.0,"period":"monthly","used":25.0,"remaining":75.0}""")
        val svc = CostService(http)
        val result = svc.budget()
        assertEquals(100.0, result.amount)
    }

    @Test
    fun `costs setBudget`() {
        fake.respondWith("""{"status":"ok"}""")
        val svc = CostService(http)
        val result = svc.setBudget(200.0)
        assertEquals("ok", result.status)
    }

    // ── Shared Service Tests ─────────────────────────────────────────

    @Test
    fun `shared get`() {
        fake.respondWith("""{"conversation":{"id":"c1","title":"Shared Chat"},"messages":[]}""")
        val svc = SharedService(http)
        val result = svc.get("s1")
        assertEquals("Shared Chat", result.conversation?.title)
        assertTrue(fake.lastRequest().uri.contains("/api/v1/shared/s1"))
    }

    // ── New Model Type Tests ─────────────────────────────────────────

    @Test
    fun `ReadinessResponse fields`() {
        val r = ReadinessResponse(ready = true, checks = mapOf("db" to "ok"))
        assertEquals(true, r.ready)
    }

    @Test
    fun `LivenessResponse fields`() {
        val l = LivenessResponse(alive = true, uptime = 12345)
        assertEquals(12345, l.uptime)
    }

    @Test
    fun `DocumentChunksResponse fields`() {
        val d = DocumentChunksResponse(documentId = "d1", chunks = listOf(ChunkInfo(id = "c1")), total = 1)
        assertEquals(1, d.total)
    }

    @Test
    fun `EntityNeighborhoodResponse fields`() {
        val e = EntityNeighborhoodResponse(
            nodes = listOf(NeighborhoodNode(id = "n1", entityType = "PERSON")),
            edges = listOf(NeighborhoodEdge(id = "e1", source = "n1", target = "n2"))
        )
        assertEquals("n1", e.nodes?.first()?.id)
        assertEquals("e1", e.edges?.first()?.id)
    }

    @Test
    fun `GraphStatsResponse fields`() {
        val g = GraphStatsResponse(nodeCount = 100, edgeCount = 200)
        assertEquals(100, g.nodeCount)
    }

    @Test
    fun `CreateApiKeyResponse fields`() {
        val c = CreateApiKeyResponse(id = "k1", key = "sk-secret")
        assertEquals("sk-secret", c.key)
    }

    @Test
    fun `WorkspaceStatsResponse fields`() {
        val w = WorkspaceStatsResponse(workspaceId = "w1", documentCount = 50, storageBytes = 1024000)
        assertEquals(1024000, w.storageBytes)
    }

    @Test
    fun `BudgetInfo fields`() {
        val b = BudgetInfo(amount = 100.0, used = 25.0, remaining = 75.0)
        assertEquals(75.0, b.remaining)
    }

    @Test
    fun `ShareLinkResponse fields`() {
        val s = ShareLinkResponse(shareId = "s1", shareUrl = "/api/v1/shared/s1")
        assertEquals("s1", s.shareId)
        assertEquals("/api/v1/shared/s1", s.shareUrl)
    }

    // ── Client Service Availability ──────────────────────────────────

    @Test
    fun `client has shared service`() {
        val client = EdgeQuakeClient(EdgeQuakeConfig(baseUrl = "http://test:8080"))
        assertNotNull(client.shared)
    }

    @Test
    fun `client has 21 services`() {
        val client = EdgeQuakeClient()
        assertNotNull(client.health)
        assertNotNull(client.documents)
        assertNotNull(client.entities)
        assertNotNull(client.relationships)
        assertNotNull(client.graph)
        assertNotNull(client.query)
        assertNotNull(client.chat)
        assertNotNull(client.auth)
        assertNotNull(client.users)
        assertNotNull(client.apiKeys)
        assertNotNull(client.tenants)
        assertNotNull(client.conversations)
        assertNotNull(client.folders)
        assertNotNull(client.tasks)
        assertNotNull(client.pipeline)
        assertNotNull(client.models)
        assertNotNull(client.workspaces)
        assertNotNull(client.pdf)
        assertNotNull(client.costs)
        assertNotNull(client.lineage)
        assertNotNull(client.shared)
    }

    // ── OODA-49: Additional Edge Case Tests ─────────────────────────────

    @Test
    fun `documents list empty OODA-49`() {
        fake.respondWith("""{"documents":[],"total":0,"page":1,"page_size":20}""")
        val svc = DocumentService(http)
        val result = svc.list()
        assertEquals(0, result.total)
        assertTrue(fake.lastRequest().uri.contains("/documents"))
    }

    @Test
    fun `entities list empty OODA-49`() {
        fake.respondWith("""{"items":[],"total":0,"page":1,"page_size":20}""")
        val svc = EntityService(http)
        val result = svc.list()
        assertEquals(0, result.total)
        assertTrue(result.items?.isEmpty() == true)
    }

    @Test
    fun `entities create success OODA-49`() {
        fake.respondWith("""{"status":"success","message":"Created","entity":{"entity_name":"TEST","entity_type":"PERSON"}}""")
        val svc = EntityService(http)
        val result = svc.create(CreateEntityRequest("TEST", "PERSON", "desc", "doc-1"))
        assertEquals("success", result.status)
        assertEquals("POST", fake.lastRequest().method)
    }

    @Test
    fun `pipeline status idle OODA-49`() {
        fake.respondWith("""{"is_busy":false,"pending_tasks":0,"processing_tasks":0}""")
        val svc = PipelineService(http)
        val result = svc.status()
        assertEquals(false, result.isBusy)
        assertEquals(0, result.pendingTasks)
    }

    @Test
    fun `pipeline status busy OODA-49`() {
        fake.respondWith("""{"is_busy":true,"pending_tasks":10,"processing_tasks":2}""")
        val svc = PipelineService(http)
        val result = svc.status()
        assertEquals(true, result.isBusy)
        assertEquals(10, result.pendingTasks)
    }

    @Test
    fun `tasks get completed OODA-49`() {
        fake.respondWith("""{"id":"t1","status":"completed","task_type":"extraction","progress":1.0}""")
        val svc = TaskService(http)
        val result = svc.get("t1")
        assertEquals("completed", result.status)
    }

    @Test
    fun `tasks cancel OODA-49`() {
        fake.respondWith("""{"status":"cancelled","message":"Task cancelled"}""")
        val svc = TaskService(http)
        val result = svc.cancel("t1")
        assertEquals("cancelled", result.status)
        assertTrue(fake.lastRequest().uri.contains("/cancel"))
    }

    @Test
    fun `models catalog empty OODA-49`() {
        fake.respondWith("""{"providers":[]}""")
        val svc = ModelService(http)
        val result = svc.catalog()
        assertTrue(result.providers?.isEmpty() == true)
    }

    @Test
    fun `models health all healthy OODA-49`() {
        fake.respondWith("""[{"name":"ollama","enabled":true},{"name":"openai","enabled":true}]""")
        val svc = ModelService(http)
        val result = svc.health()
        assertEquals(2, result.size)
        assertTrue(result.all { it.enabled == true })
    }

    @Test
    fun `costs summary OODA-49`() {
        fake.respondWith("""{"total_cost":125.50,"document_count":100,"query_count":500}""")
        val svc = CostService(http)
        val result = svc.summary()
        assertEquals(125.50, result.totalCost)
        assertEquals(100, result.documentCount)
    }

    @Test
    fun `folders list empty OODA-49`() {
        fake.respondWith("""[]""")
        val svc = FolderService(http)
        val result = svc.list()
        assertEquals(0, result.size)
    }

    @Test
    fun `folders create OODA-49`() {
        fake.respondWith("""{"id":"f1","name":"TestFolder"}""")
        val svc = FolderService(http)
        val result = svc.create("TestFolder")
        assertEquals("f1", result.id)
        assertEquals("POST", fake.lastRequest().method)
    }

    @Test
    fun `conversations get OODA-49`() {
        fake.respondWith("""{"conversation":{"id":"c1","title":"Test Conversation"},"messages":[]}""")
        val svc = ConversationService(http)
        val result = svc.get("c1")
        assertEquals("Test Conversation", result.conversation?.title)
        assertTrue(fake.lastRequest().uri.contains("/conversations/c1"))
    }

    @Test
    fun `relationships list empty OODA-49`() {
        fake.respondWith("""{"items":[],"total":0,"page":1,"page_size":20}""")
        val svc = RelationshipService(http)
        val result = svc.list()
        assertEquals(0, result.total)
    }

    @Test
    fun `users list empty OODA-49`() {
        fake.respondWith("""{"users":[]}""")
        val svc = UserService(http)
        val result = svc.list()
        assertTrue(result.users?.isEmpty() == true)
    }

    @Test
    fun `tenants get OODA-49`() {
        fake.respondWith("""{"id":"t1","name":"Test Tenant"}""")
        val svc = TenantService(http)
        val result = svc.get("t1")
        assertEquals("Test Tenant", result.name)
    }

    @Test
    fun `api keys list empty OODA-49`() {
        fake.respondWith("""{"keys":[]}""")
        val svc = ApiKeyService(http)
        val result = svc.list()
        assertTrue(result.keys?.isEmpty() == true)
    }

    @Test
    fun `api keys revoke OODA-49`() {
        fake.respondWith("")
        val svc = ApiKeyService(http)
        svc.revoke("key-1")
        assertTrue(fake.lastRequest().uri.contains("key-1"))
    }

    @Test
    fun `workspaces list OODA-49`() {
        fake.respondWith("""{"items":[{"id":"w1","name":"Default","slug":"default"}],"total":1}""")
        val svc = WorkspaceService(http)
        val result = svc.list("00000000-0000-0000-0000-000000000002")
        assertEquals(1, result.size)
        assertTrue(fake.lastRequest().uri.contains("/api/v1/tenants/00000000-0000-0000-0000-000000000002/workspaces"))
    }

    @Test
    fun `pdf content OODA-49`() {
        fake.respondWith("""{"content":"# Title\n\nHello world","page_count":5}""")
        val svc = PdfService(http)
        val result = svc.content("pdf-1")
        assertTrue(result.content?.contains("Title") == true)
        assertEquals(5, result.pageCount)
    }

    @Test
    fun `pdf progress OODA-49`() {
        fake.respondWith("""{"track_id":"tk-1","status":"completed","progress":1.0}""")
        val svc = PdfService(http)
        val result = svc.progress("tk-1")
        assertEquals("completed", result.status)
    }

    @Test
    fun `lineage entity OODA-49`() {
        fake.respondWith("""{"entity_name":"ALICE","entity_type":"PERSON","source_documents":[],"source_count":0}""")
        val svc = LineageService(http)
        val result = svc.entityLineage("ALICE")
        assertEquals("ALICE", result.entityName)
        assertTrue(fake.lastRequest().uri.contains("/lineage/entities/ALICE"))
    }

    @Test
    fun `lineage document OODA-49`() {
        fake.respondWith("""{"document_id":"d1","chunk_count":5,"entities":[],"relationships":[]}""")
        val svc = LineageService(http)
        val result = svc.documentLineage("d1")
        assertEquals("d1", result.documentId)
    }

    @Test
    fun `health ready OODA-49`() {
        fake.respondWith("""{"status":"ready","checks":{"db":"ok"}}""")
        val svc = HealthService(http)
        val result = svc.ready()
        assertNotNull(result)
        assertTrue(fake.lastRequest().uri.contains("/ready"))
    }

    @Test
    fun `auth logout OODA-49`() {
        fake.respondWith("""{"status":"logged_out"}""")
        val svc = AuthService(http)
        val result = svc.logout()
        assertNotNull(result)
        assertEquals("POST", fake.lastRequest().method)
    }

    @Test
    fun `query with hybrid mode OODA-49`() {
        fake.respondWith("""{"answer":"Test answer","mode":"hybrid","sources":[]}""")
        val svc = QueryService(http)
        val result = svc.execute("test query", mode = "hybrid")
        assertEquals("hybrid", result.mode)
    }

    @Test
    fun `chat completions OODA-49`() {
        fake.respondWith("""{"conversation_id":"c1","content":"Hello!","sources":[]}""")
        val svc = ChatService(http)
        val result = svc.completions(ChatCompletionRequest(message = "Hello"))
        assertEquals("c1", result.conversationId)
    }

    @Test
    fun `graph search OODA-49`() {
        fake.respondWith("""{"nodes":[],"edges":[],"total_nodes":0}""")
        val svc = GraphService(http)
        val result = svc.search("ALICE")
        assertTrue(fake.lastRequest().uri.contains("q=ALICE"))
    }

    @Test
    fun `graph popular labels OODA-49`() {
        fake.respondWith("""{"labels":[{"label":"PERSON","degree":50,"entity_type":"PERSON","description":""}],"total_entities":100}""")
        val svc = GraphService(http)
        val result = svc.popularLabels(10)
        assertTrue(result.labels?.isNotEmpty() == true)
    }

    @Test
    fun `shared get empty messages OODA-49`() {
        fake.respondWith("""{"conversation":{"id":"c1","title":"Public"},"messages":[]}""")
        val svc = SharedService(http)
        val result = svc.get("s-empty")
        assertTrue(result.messages?.isEmpty() == true)
        assertTrue(fake.lastRequest().uri.contains("/api/v1/shared/s-empty"))
    }
}
