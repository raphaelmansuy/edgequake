# frozen_string_literal: true

require "minitest/autorun"
require_relative "mock_http_helper"

# Unit tests for the EdgeQuake Ruby SDK.
# WHY: Verify all components without making real HTTP calls.
module EdgeQuake
  class ConfigTest < Minitest::Test
    def test_defaults
      c = Config.new
      assert_equal "http://localhost:8080", c.base_url
      assert_nil c.api_key
      assert_nil c.tenant_id
      assert_nil c.user_id
      assert_nil c.workspace_id
      assert_equal 60, c.timeout
    end

    def test_custom_values
      c = Config.new(
        base_url: "https://api.example.com",
        api_key: "sk-test",
        tenant_id: "t-1",
        user_id: "u-1",
        workspace_id: "ws-1",
        timeout: 120
      )
      assert_equal "https://api.example.com", c.base_url
      assert_equal "sk-test", c.api_key
      assert_equal "t-1", c.tenant_id
      assert_equal "u-1", c.user_id
      assert_equal "ws-1", c.workspace_id
      assert_equal 120, c.timeout
    end

    def test_strips_trailing_slash
      c = Config.new(base_url: "http://localhost:8080/")
      assert_equal "http://localhost:8080", c.base_url
    end
  end

  class ApiErrorTest < Minitest::Test
    def test_message_and_properties
      err = ApiError.new("bad request", status_code: 400, response_body: '{"error":"fail"}')
      assert_equal "bad request", err.message
      assert_equal 400, err.status_code
      assert_equal '{"error":"fail"}', err.response_body
    end

    def test_is_standard_error
      err = ApiError.new("test")
      assert_kind_of StandardError, err
    end

    def test_nil_defaults
      err = ApiError.new("test")
      assert_nil err.status_code
      assert_nil err.response_body
    end
  end

  class ClientTest < Minitest::Test
    def test_initializes_all_services
      client = Client.new
      assert_instance_of HealthService, client.health
      assert_instance_of DocumentService, client.documents
      assert_instance_of EntityService, client.entities
      assert_instance_of RelationshipService, client.relationships
      assert_instance_of GraphService, client.graph
      assert_instance_of QueryService, client.query
      assert_instance_of ChatService, client.chat
      assert_instance_of TenantService, client.tenants
      assert_instance_of UserService, client.users
      assert_instance_of ApiKeyService, client.api_keys
      assert_instance_of TaskService, client.tasks
      assert_instance_of PipelineService, client.pipeline
      assert_instance_of ModelService, client.models
      assert_instance_of CostService, client.costs
    end

    # OODA-34: New service tests
    def test_initializes_auth_service
      client = Client.new
      assert_instance_of AuthService, client.auth
    end

    def test_initializes_workspaces_service
      client = Client.new
      assert_instance_of WorkspaceService, client.workspaces
    end

    def test_initializes_shared_service
      client = Client.new
      assert_instance_of SharedService, client.shared
    end
  end

  class HealthServiceTest < Minitest::Test
    def test_check
      mock = MockHttpHelper.new('{"status":"healthy","version":"0.1.0"}')
      svc = HealthService.new(mock)
      result = svc.check
      assert_equal "healthy", result["status"]
      assert_equal :get, mock.last_call[:method]
      assert_equal "/health", mock.last_call[:path]
    end

    def test_check_error
      mock = MockHttpHelper.new.will_return("{}", 500)
      svc = HealthService.new(mock)
      assert_raises(ApiError) { svc.check }
    end

    # OODA-34: New health endpoint tests
    def test_readiness
      mock = MockHttpHelper.new('{"ready":true}')
      svc = HealthService.new(mock)
      result = svc.readiness
      assert_equal true, result["ready"]
      assert_equal "/ready", mock.last_call[:path]
    end

    def test_liveness
      mock = MockHttpHelper.new('{"live":true}')
      svc = HealthService.new(mock)
      result = svc.liveness
      assert_equal true, result["live"]
      assert_equal "/live", mock.last_call[:path]
    end

    def test_metrics_raw
      mock = MockHttpHelper.new("# TYPE x gauge\nx 1\n")
      svc = HealthService.new(mock)
      result = svc.metrics
      assert_includes result, "TYPE"
      assert_equal "/metrics", mock.last_call[:path]
    end
  end

  class DocumentServiceTest < Minitest::Test
    def test_list
      mock = MockHttpHelper.new('{"documents":[{"id":"d1"}]}')
      svc = DocumentService.new(mock)
      result = svc.list
      assert_equal 1, result["documents"].size
      assert_includes mock.last_call[:path], "page=1"
      assert_includes mock.last_call[:path], "page_size=20"
    end

    def test_list_pagination
      mock = MockHttpHelper.new('{"documents":[]}')
      svc = DocumentService.new(mock)
      svc.list(page: 3, page_size: 50)
      assert_includes mock.last_call[:path], "page=3"
      assert_includes mock.last_call[:path], "page_size=50"
    end

    def test_get
      mock = MockHttpHelper.new('{"id":"d1","file_name":"test.pdf"}')
      svc = DocumentService.new(mock)
      result = svc.get(id: "d1")
      assert_equal "d1", result["id"]
      assert_includes mock.last_call[:path], "/api/v1/documents/d1"
    end

    def test_upload_text
      mock = MockHttpHelper.new('{"id":"d2","status":"processing"}')
      svc = DocumentService.new(mock)
      result = svc.upload_text(title: "My Title", content: "Hello World")
      assert_equal "d2", result["id"]
      assert_equal :post, mock.last_call[:method]
      assert_equal "My Title", mock.last_call[:body][:title]
    end

    def test_upload_text_custom_file_type
      mock = MockHttpHelper.new('{"id":"d3"}')
      svc = DocumentService.new(mock)
      svc.upload_text(title: "T", content: "C", file_type: "md")
      assert_equal "md", mock.last_call[:body][:file_type]
    end

    def test_delete
      mock = MockHttpHelper.new('{"status":"deleted"}')
      svc = DocumentService.new(mock)
      svc.delete(id: "d1")
      assert_equal :delete, mock.last_call[:method]
      assert_includes mock.last_call[:path], "/api/v1/documents/d1"
    end

    def test_track
      mock = MockHttpHelper.new('{"track_id":"tk1","status":"processing"}')
      svc = DocumentService.new(mock)
      result = svc.track(track_id: "tk1")
      assert_equal "tk1", result["track_id"]
      assert_includes mock.last_call[:path], "/api/v1/documents/track/tk1"
    end

    def test_reprocess_failed
      mock = MockHttpHelper.new('{"status":"ok"}')
      svc = DocumentService.new(mock)
      result = svc.reprocess_failed
      assert_equal "ok", result["status"]
      assert_equal :post, mock.last_call[:method]
      assert_includes mock.last_call[:path], "/api/v1/documents/reprocess"
    end

    def test_recover_stuck
      mock = MockHttpHelper.new('{"status":"ok"}')
      svc = DocumentService.new(mock)
      svc.recover_stuck
      assert_includes mock.last_call[:path], "/api/v1/documents/recover-stuck"
    end

    def test_list_error
      mock = MockHttpHelper.new.will_return("{}", 500)
      svc = DocumentService.new(mock)
      assert_raises(ApiError) { svc.list }
    end

    def test_get_error
      mock = MockHttpHelper.new.will_return("{}", 404)
      svc = DocumentService.new(mock)
      assert_raises(ApiError) { svc.get(id: "missing") }
    end

    # OODA-42: New document tests
    def test_get_metadata
      mock = MockHttpHelper.new('{"author":"John Doe","category":"research"}')
      svc = DocumentService.new(mock)
      result = svc.get_metadata(id: "d1")
      assert_equal "John Doe", result["author"]
      assert_includes mock.last_call[:path], "/api/v1/documents/d1/metadata"
    end

    def test_failed_chunks
      mock = MockHttpHelper.new('[{"chunk_id":"c1","error":"timeout"}]')
      svc = DocumentService.new(mock)
      result = svc.failed_chunks(id: "d1")
      assert_equal 1, result.size
      assert_includes mock.last_call[:path], "/api/v1/documents/d1/failed-chunks"
    end

    def test_retry_chunks
      mock = MockHttpHelper.new('{"retried":3}')
      svc = DocumentService.new(mock)
      result = svc.retry_chunks(id: "d1")
      assert_equal 3, result["retried"]
      assert_equal :post, mock.last_call[:method]
      assert_includes mock.last_call[:path], "/api/v1/documents/d1/retry-chunks"
    end

    def test_deletion_impact
      mock = MockHttpHelper.new('{"chunk_count":10,"entity_count":5}')
      svc = DocumentService.new(mock)
      result = svc.deletion_impact(id: "d1")
      assert_equal 10, result["chunk_count"]
      assert_includes mock.last_call[:path], "/api/v1/documents/d1/deletion-impact"
    end

    def test_lineage
      mock = MockHttpHelper.new('{"document_id":"d1","chunks":[],"entities":[]}')
      svc = DocumentService.new(mock)
      result = svc.lineage(id: "d1")
      assert_equal "d1", result["document_id"]
      assert_includes mock.last_call[:path], "/api/v1/documents/d1/lineage"
    end
  end

  class EntityServiceTest < Minitest::Test
    def test_list
      mock = MockHttpHelper.new('{"items":[{"entity_name":"ALICE"}],"total":1}')
      svc = EntityService.new(mock)
      result = svc.list
      assert_equal 1, result["items"].size
      assert_equal "ALICE", result["items"][0]["entity_name"]
    end

    def test_get
      mock = MockHttpHelper.new('{"entity_name":"ALICE","entity_type":"person"}')
      svc = EntityService.new(mock)
      result = svc.get(name: "ALICE")
      assert_equal "person", result["entity_type"]
    end

    def test_create
      mock = MockHttpHelper.new('{"status":"success"}')
      svc = EntityService.new(mock)
      result = svc.create(entity_name: "BOB", entity_type: "person", description: "A person", source_id: "src-1")
      assert_equal "success", result["status"]
      assert_equal :post, mock.last_call[:method]
      assert_equal "BOB", mock.last_call[:body][:entity_name]
      assert_equal "person", mock.last_call[:body][:entity_type]
    end

    def test_delete
      mock = MockHttpHelper.new('{}')
      svc = EntityService.new(mock)
      svc.delete(name: "BOB")
      assert_equal :delete, mock.last_call[:method]
      assert_includes mock.last_call[:path], "confirm=true"
    end

    def test_exists
      mock = MockHttpHelper.new('{"exists":true}')
      svc = EntityService.new(mock)
      result = svc.exists?(name: "ALICE")
      assert_equal true, result["exists"]
      assert_includes mock.last_call[:path], "entity_name=ALICE"
    end

    # OODA-34: New entity method tests
    def test_update
      mock = MockHttpHelper.new('{"entity_name":"ALICE","description":"Updated"}')
      svc = EntityService.new(mock)
      result = svc.update(name: "ALICE", description: "Updated")
      assert_equal "Updated", result["description"]
      assert_equal :put, mock.last_call[:method]
      assert_includes mock.last_call[:path], "/api/v1/graph/entities/ALICE"
    end

    def test_merge
      mock = MockHttpHelper.new('{"status":"merged","target":"ALICE"}')
      svc = EntityService.new(mock)
      result = svc.merge(source_name: "ALICIA", target_name: "ALICE")
      assert_equal "merged", result["status"]
      assert_equal :post, mock.last_call[:method]
      assert_equal "ALICIA", mock.last_call[:body][:source_name]
      assert_equal "ALICE", mock.last_call[:body][:target_name]
    end

    def test_neighborhood
      mock = MockHttpHelper.new('{"nodes":[],"edges":[]}')
      svc = EntityService.new(mock)
      svc.neighborhood(name: "ALICE", depth: 2)
      assert_includes mock.last_call[:path], "/neighborhood"
      assert_includes mock.last_call[:path], "depth=2"
    end

    def test_list_error
      mock = MockHttpHelper.new.will_return("{}", 500)
      svc = EntityService.new(mock)
      assert_raises(ApiError) { svc.list }
    end
  end

  class RelationshipServiceTest < Minitest::Test
    def test_list
      mock = MockHttpHelper.new('{"items":[{"source":"A","target":"B"}],"total":1}')
      svc = RelationshipService.new(mock)
      result = svc.list
      assert_equal 1, result["items"].size
    end

    def test_list_pagination
      mock = MockHttpHelper.new('{"items":[]}')
      svc = RelationshipService.new(mock)
      svc.list(page: 2, page_size: 10)
      assert_includes mock.last_call[:path], "page=2"
      assert_includes mock.last_call[:path], "page_size=10"
    end

    # OODA-34: New relationship method tests
    def test_create
      mock = MockHttpHelper.new('{"status":"success","relationship":{"id":"r1"}}')
      svc = RelationshipService.new(mock)
      result = svc.create(src_id: "A", tgt_id: "B", keywords: "knows", description: "A knows B")
      assert_equal "success", result["status"]
      assert_equal :post, mock.last_call[:method]
      assert_equal "A", mock.last_call[:body][:src_id]
      assert_equal "B", mock.last_call[:body][:tgt_id]
    end

    def test_get
      mock = MockHttpHelper.new('{"relationship":{"id":"r1"}}')
      svc = RelationshipService.new(mock)
      result = svc.get(id: "r1")
      assert_equal "r1", result.dig("relationship", "id")
      assert_includes mock.last_call[:path], "/api/v1/graph/relationships/r1"
    end

    def test_delete
      mock = MockHttpHelper.new('{}')
      svc = RelationshipService.new(mock)
      svc.delete(id: "r1")
      assert_equal :delete, mock.last_call[:method]
      assert_includes mock.last_call[:path], "/api/v1/graph/relationships/r1"
    end

    def test_list_error
      mock = MockHttpHelper.new.will_return("{}", 500)
      svc = RelationshipService.new(mock)
      assert_raises(ApiError) { svc.list }
    end
  end

  class GraphServiceTest < Minitest::Test
    def test_get
      mock = MockHttpHelper.new('{"nodes":[],"edges":[]}')
      svc = GraphService.new(mock)
      result = svc.get
      assert result.key?("nodes")
    end

    def test_search
      mock = MockHttpHelper.new('{"nodes":[{"id":"n1"}]}')
      svc = GraphService.new(mock)
      result = svc.search(query: "Alice")
      assert_equal 1, result["nodes"].size
      assert_includes mock.last_call[:path], "q=Alice"
    end

    def test_search_url_encoding
      mock = MockHttpHelper.new('{"nodes":[]}')
      svc = GraphService.new(mock)
      svc.search(query: "hello world")
      assert_includes mock.last_call[:path], "q=hello+world"
    end

    def test_get_node
      mock = MockHttpHelper.new('{"id":"n1","label":"X"}')
      svc = GraphService.new(mock)
      result = svc.get_node(node_id: "n1")
      assert_equal "n1", result["id"]
      assert_includes mock.last_call[:path], "/api/v1/graph/nodes/n1"
    end

    def test_label_search
      mock = MockHttpHelper.new('{"labels":["PERSON"]}')
      svc = GraphService.new(mock)
      result = svc.label_search(query: "PER")
      assert_equal 1, result["labels"].size
      assert_includes mock.last_call[:path], "/labels/search"
    end

    def test_degrees_batch
      mock = MockHttpHelper.new('{"degrees":[{"node_id":"a","degree":2}],"count":1}')
      svc = GraphService.new(mock)
      result = svc.degrees_batch(node_ids: %w[a b])
      assert_equal 1, result["degrees"].size
      assert_equal :post, mock.last_call[:method]
    end

    def test_get_error
      mock = MockHttpHelper.new.will_return("{}", 500)
      svc = GraphService.new(mock)
      assert_raises(ApiError) { svc.get }
    end

    def test_search_error
      mock = MockHttpHelper.new.will_return("{}", 500)
      svc = GraphService.new(mock)
      assert_raises(ApiError) { svc.search(query: "test") }
    end
  end

  class QueryServiceTest < Minitest::Test
    def test_execute
      mock = MockHttpHelper.new('{"answer":"42","sources":[]}')
      svc = QueryService.new(mock)
      result = svc.execute(query: "meaning of life")
      assert_equal "42", result["answer"]
      assert_equal :post, mock.last_call[:method]
      assert_equal "meaning of life", mock.last_call[:body][:query]
    end

    def test_execute_with_mode
      mock = MockHttpHelper.new('{"answer":"yes"}')
      svc = QueryService.new(mock)
      svc.execute(query: "test", mode: "local")
      assert_equal "local", mock.last_call[:body][:mode]
    end

    def test_execute_error
      mock = MockHttpHelper.new.will_return("{}", 500)
      svc = QueryService.new(mock)
      assert_raises(ApiError) { svc.execute(query: "test") }
    end

    # OODA-42: New query tests
    def test_execute_with_context
      mock = MockHttpHelper.new('{"answer":"yes","sources":[{"id":"d1"}]}')
      svc = QueryService.new(mock)
      result = svc.execute_with_context(query: "test", top_k: 10, only_need_context: true)
      assert_equal "yes", result["answer"]
      body = mock.last_call[:body]
      assert_equal 10, body[:top_k]
      assert_equal true, body[:only_need_context]
    end

    def test_stream_returns_enumerator
      mock = MockHttpHelper.new("{}")
      svc = QueryService.new(mock)
      result = svc.stream(query: "Test")
      assert_kind_of Enumerator, result
    end
  end

  class ChatServiceTest < Minitest::Test
    def test_completions
      mock = MockHttpHelper.new('{"choices":[{"message":{"content":"Hello!"}}]}')
      svc = ChatService.new(mock)
      result = svc.completions(message: "Hi")
      assert_equal 1, result["choices"].size
      assert_equal :post, mock.last_call[:method]
    end

    def test_completions_with_options
      mock = MockHttpHelper.new('{"choices":[]}')
      svc = ChatService.new(mock)
      svc.completions(message: "hi", mode: "global", stream: true)
      body = mock.last_call[:body]
      assert_equal "hi", body[:message]
      assert_equal "global", body[:mode]
      assert_equal true, body[:stream]
    end

    def test_completions_error
      mock = MockHttpHelper.new.will_return("{}", 500)
      svc = ChatService.new(mock)
      assert_raises(ApiError) { svc.completions(message: "test") }
    end

    # OODA-42: New chat tests
    def test_completions_with_conversation
      mock = MockHttpHelper.new('{"choices":[{"message":{"content":"Response"}}]}')
      svc = ChatService.new(mock)
      result = svc.completions_with_conversation(message: "Hi", conversation_id: "conv-1")
      assert_equal 1, result["choices"].size
      body = mock.last_call[:body]
      assert_equal "Hi", body[:message]
      assert_equal "conv-1", body[:conversation_id]
    end

    def test_stream_returns_enumerator
      mock = MockHttpHelper.new("{}")
      svc = ChatService.new(mock)
      result = svc.stream(message: "Test")
      assert_kind_of Enumerator, result
    end
  end

  class TenantServiceTest < Minitest::Test
    def test_list
      mock = MockHttpHelper.new('{"items":[{"id":"t1"}]}')
      svc = TenantService.new(mock)
      result = svc.list
      assert_equal 1, result["items"].size
    end

    # OODA-34: New tenant method tests
    def test_get
      mock = MockHttpHelper.new('{"id":"t1","name":"Tenant 1"}')
      svc = TenantService.new(mock)
      result = svc.get(id: "t1")
      assert_equal "t1", result["id"]
      assert_includes mock.last_call[:path], "/api/v1/tenants/t1"
    end

    def test_create
      mock = MockHttpHelper.new('{"id":"t2","name":"New Tenant"}')
      svc = TenantService.new(mock)
      result = svc.create(name: "New Tenant")
      assert_equal "t2", result["id"]
      assert_equal :post, mock.last_call[:method]
      assert_equal "New Tenant", mock.last_call[:body][:name]
    end

    def test_update
      mock = MockHttpHelper.new('{"id":"t1","name":"Updated"}')
      svc = TenantService.new(mock)
      result = svc.update(id: "t1", name: "Updated")
      assert_equal "Updated", result["name"]
      assert_equal :put, mock.last_call[:method]
    end

    def test_delete
      mock = MockHttpHelper.new('{}')
      svc = TenantService.new(mock)
      svc.delete(id: "t1")
      assert_equal :delete, mock.last_call[:method]
      assert_includes mock.last_call[:path], "/api/v1/tenants/t1"
    end

    def test_list_error
      mock = MockHttpHelper.new.will_return("{}", 500)
      svc = TenantService.new(mock)
      assert_raises(ApiError) { svc.list }
    end
  end

  class UserServiceTest < Minitest::Test
    def test_list
      mock = MockHttpHelper.new('[{"id":"u1","username":"admin"}]')
      svc = UserService.new(mock)
      result = svc.list
      assert_equal 1, result.size
    end

    # OODA-34: New user method tests
    def test_get
      mock = MockHttpHelper.new('{"id":"u1","email":"user@test.com"}')
      svc = UserService.new(mock)
      result = svc.get(id: "u1")
      assert_equal "u1", result["id"]
      assert_includes mock.last_call[:path], "/api/v1/users/u1"
    end

    def test_create
      mock = MockHttpHelper.new('{"id":"u2","email":"new@test.com"}')
      svc = UserService.new(mock)
      result = svc.create(email: "new@test.com", name: "New User", role: "admin")
      assert_equal "u2", result["id"]
      assert_equal :post, mock.last_call[:method]
      assert_equal "new@test.com", mock.last_call[:body][:email]
      assert_equal "admin", mock.last_call[:body][:role]
    end

    def test_update
      mock = MockHttpHelper.new('{"id":"u1","name":"Updated Name"}')
      svc = UserService.new(mock)
      result = svc.update(id: "u1", name: "Updated Name")
      assert_equal "Updated Name", result["name"]
      assert_equal :put, mock.last_call[:method]
    end

    def test_delete
      mock = MockHttpHelper.new('{}')
      svc = UserService.new(mock)
      svc.delete(id: "u1")
      assert_equal :delete, mock.last_call[:method]
      assert_includes mock.last_call[:path], "/api/v1/users/u1"
    end

    def test_list_error
      mock = MockHttpHelper.new.will_return("{}", 500)
      svc = UserService.new(mock)
      assert_raises(ApiError) { svc.list }
    end
  end

  class ApiKeyServiceTest < Minitest::Test
    def test_list
      mock = MockHttpHelper.new('[{"id":"ak-1"}]')
      svc = ApiKeyService.new(mock)
      result = svc.list
      assert_equal 1, result.size
    end

    # OODA-34: New api key method tests
    def test_get
      mock = MockHttpHelper.new('{"id":"ak-1","name":"My Key"}')
      svc = ApiKeyService.new(mock)
      result = svc.get(id: "ak-1")
      assert_equal "ak-1", result["id"]
      assert_includes mock.last_call[:path], "/api/v1/api-keys/ak-1"
    end

    def test_create
      mock = MockHttpHelper.new('{"id":"ak-2","key":"sk-new"}')
      svc = ApiKeyService.new(mock)
      result = svc.create(name: "New Key", permissions: %w[read write])
      assert_equal "ak-2", result["id"]
      assert_equal :post, mock.last_call[:method]
      assert_equal "New Key", mock.last_call[:body][:name]
      assert_equal %w[read write], mock.last_call[:body][:permissions]
    end

    def test_revoke
      mock = MockHttpHelper.new('{}')
      svc = ApiKeyService.new(mock)
      svc.revoke(id: "ak-1")
      assert_equal :delete, mock.last_call[:method]
      assert_includes mock.last_call[:path], "/api/v1/api-keys/ak-1"
    end

    def test_rotate
      mock = MockHttpHelper.new('{"id":"ak-1","key":"sk-rotated"}')
      svc = ApiKeyService.new(mock)
      result = svc.rotate(id: "ak-1")
      assert_equal "ak-1", result["id"]
      assert_equal :post, mock.last_call[:method]
      assert_includes mock.last_call[:path], "/api/v1/api-keys/ak-1/rotate"
    end

    def test_list_error
      mock = MockHttpHelper.new.will_return("{}", 500)
      svc = ApiKeyService.new(mock)
      assert_raises(ApiError) { svc.list }
    end
  end

  class TaskServiceTest < Minitest::Test
    def test_list
      mock = MockHttpHelper.new('{"tasks":[{"track_id":"trk-1"}]}')
      svc = TaskService.new(mock)
      result = svc.list
      assert_equal 1, result["tasks"].size
    end

    # OODA-34: New task method tests
    def test_get
      mock = MockHttpHelper.new('{"id":"t1","status":"completed"}')
      svc = TaskService.new(mock)
      result = svc.get(id: "t1")
      assert_equal "t1", result["id"]
      assert_includes mock.last_call[:path], "/api/v1/tasks/t1"
    end

    def test_create
      mock = MockHttpHelper.new('{"id":"t2","status":"pending"}')
      svc = TaskService.new(mock)
      result = svc.create(task_type: "extraction", parameters: { doc_id: "d1" })
      assert_equal "t2", result["id"]
      assert_equal :post, mock.last_call[:method]
      assert_equal "extraction", mock.last_call[:body][:task_type]
    end

    def test_cancel
      mock = MockHttpHelper.new('{"id":"t1","status":"cancelled"}')
      svc = TaskService.new(mock)
      result = svc.cancel(id: "t1")
      assert_equal "cancelled", result["status"]
      assert_equal :post, mock.last_call[:method]
      assert_includes mock.last_call[:path], "/api/v1/tasks/t1/cancel"
    end

    def test_status
      mock = MockHttpHelper.new('{"id":"t1","status":"running"}')
      svc = TaskService.new(mock)
      result = svc.status(id: "t1")
      assert_equal "running", result["status"]
      assert_includes mock.last_call[:path], "/api/v1/tasks/t1/status"
    end

    def test_list_error
      mock = MockHttpHelper.new.will_return("{}", 500)
      svc = TaskService.new(mock)
      assert_raises(ApiError) { svc.list }
    end
  end

  class PipelineServiceTest < Minitest::Test
    def test_status
      mock = MockHttpHelper.new('{"is_busy":true,"pending_tasks":5}')
      svc = PipelineService.new(mock)
      result = svc.status
      assert_equal true, result["is_busy"]
    end

    def test_queue_metrics
      mock = MockHttpHelper.new('{"queue_depth":10}')
      svc = PipelineService.new(mock)
      result = svc.queue_metrics
      assert_equal 10, result["queue_depth"]
    end

    # OODA-34: New pipeline method tests
    def test_health
      mock = MockHttpHelper.new('{"healthy":true,"workers":4}')
      svc = PipelineService.new(mock)
      result = svc.health
      assert_equal true, result["healthy"]
      assert_includes mock.last_call[:path], "/api/v1/pipeline/health"
    end

    def test_status_error
      mock = MockHttpHelper.new.will_return("{}", 500)
      svc = PipelineService.new(mock)
      assert_raises(ApiError) { svc.status }
    end

    def test_queue_metrics_error
      mock = MockHttpHelper.new.will_return("{}", 500)
      svc = PipelineService.new(mock)
      assert_raises(ApiError) { svc.queue_metrics }
    end
  end

  class ModelServiceTest < Minitest::Test
    def test_catalog
      mock = MockHttpHelper.new('{"providers":[{"name":"openai"}]}')
      svc = ModelService.new(mock)
      result = svc.catalog
      assert_equal 1, result["providers"].size
    end

    def test_health
      mock = MockHttpHelper.new('{"status":"ok","models":["qwen2.5"]}')
      svc = ModelService.new(mock)
      result = svc.health
      assert_equal "ok", result["status"]
    end

    def test_provider_status
      mock = MockHttpHelper.new('{"current_provider":"ollama"}')
      svc = ModelService.new(mock)
      result = svc.provider_status
      assert_equal "ollama", result["current_provider"]
    end

    # OODA-34: New model method tests
    def test_list_providers
      mock = MockHttpHelper.new('{"providers":["openai","ollama"]}')
      svc = ModelService.new(mock)
      result = svc.list_providers
      assert_equal 2, result["providers"].size
      assert_includes mock.last_call[:path], "/api/v1/models/providers"
    end

    def test_get_model
      mock = MockHttpHelper.new('{"id":"m1","name":"gpt-4"}')
      svc = ModelService.new(mock)
      result = svc.get_model(id: "m1")
      assert_equal "m1", result["id"]
      assert_includes mock.last_call[:path], "/api/v1/models/m1"
    end

    def test_set_active
      mock = MockHttpHelper.new('{"id":"m1","active":true}')
      svc = ModelService.new(mock)
      result = svc.set_active(id: "m1")
      assert_equal true, result["active"]
      assert_equal :post, mock.last_call[:method]
      assert_includes mock.last_call[:path], "/api/v1/models/m1/activate"
    end

    def test_usage
      mock = MockHttpHelper.new('{"total_tokens":1000}')
      svc = ModelService.new(mock)
      result = svc.usage(days: 14)
      assert_equal 1000, result["total_tokens"]
      assert_includes mock.last_call[:path], "days=14"
    end

    def test_usage_for_specific_model
      mock = MockHttpHelper.new('{"model_id":"m1","total_tokens":500}')
      svc = ModelService.new(mock)
      result = svc.usage(id: "m1", days: 7)
      assert_equal 500, result["total_tokens"]
      assert_includes mock.last_call[:path], "/api/v1/models/m1/usage"
    end

    def test_catalog_error
      mock = MockHttpHelper.new.will_return("{}", 500)
      svc = ModelService.new(mock)
      assert_raises(ApiError) { svc.catalog }
    end

    def test_health_error
      mock = MockHttpHelper.new.will_return("{}", 502)
      svc = ModelService.new(mock)
      assert_raises(ApiError) { svc.health }
    end
  end

  class CostServiceTest < Minitest::Test
    def test_summary
      mock = MockHttpHelper.new('{"total_cost_usd":12.5}')
      svc = CostService.new(mock)
      result = svc.summary
      assert_equal 12.5, result["total_cost_usd"]
    end

    # OODA-34: New cost method tests
    def test_breakdown
      mock = MockHttpHelper.new('{"breakdown":[{"category":"llm","cost":10}]}')
      svc = CostService.new(mock)
      result = svc.breakdown(start_date: "2025-01-01")
      assert_equal 1, result["breakdown"].size
      assert_includes mock.last_call[:path], "start_date=2025-01-01"
    end

    def test_by_model
      mock = MockHttpHelper.new('{"models":[{"name":"gpt-4","cost":5}]}')
      svc = CostService.new(mock)
      result = svc.by_model(days: 14)
      assert_equal 1, result["models"].size
      assert_includes mock.last_call[:path], "days=14"
    end

    def test_by_tenant
      mock = MockHttpHelper.new('{"tenants":[{"id":"t1","cost":8}]}')
      svc = CostService.new(mock)
      result = svc.by_tenant(days: 7)
      assert_equal 1, result["tenants"].size
      assert_includes mock.last_call[:path], "days=7"
    end

    def test_history
      mock = MockHttpHelper.new('{"history":[{"date":"2025-01-01","cost":1}]}')
      svc = CostService.new(mock)
      result = svc.history(days: 30)
      assert_equal 1, result["history"].size
      assert_includes mock.last_call[:path], "/api/v1/costs/history"
    end

    def test_summary_error
      mock = MockHttpHelper.new.will_return("{}", 500)
      svc = CostService.new(mock)
      assert_raises(ApiError) { svc.summary }
    end
  end

  # --- ConversationService Tests ---
  class ConversationServiceTest < Minitest::Test
    def test_list
      mock = MockHttpHelper.new('{"items":[{"id":"c1","title":"Chat 1"}],"total":1}')
      svc = ConversationService.new(mock)
      result = svc.list
      assert_equal 1, result["items"].size
      assert_equal "c1", result["items"][0]["id"]
      assert_equal :get, mock.last_call[:method]
      assert_includes mock.last_call[:path], "/api/v1/conversations"
    end

    def test_list_empty
      mock = MockHttpHelper.new('{"items":[],"total":0}')
      svc = ConversationService.new(mock)
      result = svc.list
      assert_equal 0, result["items"].size
    end

    def test_create
      mock = MockHttpHelper.new('{"id":"c2","title":"New Chat"}')
      svc = ConversationService.new(mock)
      result = svc.create(title: "New Chat")
      assert_equal "c2", result["id"]
      assert_equal :post, mock.last_call[:method]
      assert_equal "New Chat", mock.last_call[:body][:title]
    end

    def test_create_with_mode
      mock = MockHttpHelper.new('{"id":"c3"}')
      svc = ConversationService.new(mock)
      svc.create(title: "T", mode: "local")
      assert_equal "local", mock.last_call[:body][:mode]
    end

    def test_create_with_folder_id
      mock = MockHttpHelper.new('{"id":"c4"}')
      svc = ConversationService.new(mock)
      svc.create(title: "T", folder_id: "f-1")
      assert_equal "f-1", mock.last_call[:body][:folder_id]
    end

    def test_create_with_all_options
      mock = MockHttpHelper.new('{"id":"c5"}')
      svc = ConversationService.new(mock)
      svc.create(title: "Full", mode: "global", folder_id: "f-2")
      body = mock.last_call[:body]
      assert_equal "Full", body[:title]
      assert_equal "global", body[:mode]
      assert_equal "f-2", body[:folder_id]
    end

    def test_create_omits_nil_mode
      mock = MockHttpHelper.new('{"id":"c6"}')
      svc = ConversationService.new(mock)
      svc.create(title: "Simple")
      refute mock.last_call[:body].key?(:mode)
    end

    def test_create_omits_nil_folder_id
      mock = MockHttpHelper.new('{"id":"c7"}')
      svc = ConversationService.new(mock)
      svc.create(title: "Simple")
      refute mock.last_call[:body].key?(:folder_id)
    end

    # OODA-34: New conversation method tests
    def test_get
      mock = MockHttpHelper.new('{"id":"c1","title":"Chat 1"}')
      svc = ConversationService.new(mock)
      result = svc.get(id: "c1")
      assert_equal "c1", result["id"]
      assert_includes mock.last_call[:path], "/api/v1/conversations/c1"
    end

    def test_update
      mock = MockHttpHelper.new('{"id":"c1","title":"Updated Title"}')
      svc = ConversationService.new(mock)
      result = svc.update(id: "c1", title: "Updated Title")
      assert_equal "Updated Title", result["title"]
      assert_equal :put, mock.last_call[:method]
    end

    def test_delete
      mock = MockHttpHelper.new('{}')
      svc = ConversationService.new(mock)
      svc.delete(id: "c1")
      assert_equal :delete, mock.last_call[:method]
      assert_includes mock.last_call[:path], "/api/v1/conversations/c1"
    end

    def test_messages
      mock = MockHttpHelper.new('{"messages":[{"role":"user","content":"Hi"}]}')
      svc = ConversationService.new(mock)
      result = svc.messages(id: "c1")
      assert_equal 1, result["messages"].size
      assert_includes mock.last_call[:path], "/api/v1/conversations/c1/messages"
    end

    def test_add_message
      mock = MockHttpHelper.new('{"id":"m1","role":"user","content":"Hello"}')
      svc = ConversationService.new(mock)
      result = svc.add_message(id: "c1", role: "user", content: "Hello")
      assert_equal "m1", result["id"]
      assert_equal :post, mock.last_call[:method]
      assert_equal "user", mock.last_call[:body][:role]
    end

    def test_delete_message
      mock = MockHttpHelper.new('{}')
      svc = ConversationService.new(mock)
      svc.delete_message(conversation_id: "c1", message_id: "m1")
      assert_equal :delete, mock.last_call[:method]
      assert_includes mock.last_call[:path], "/api/v1/conversations/c1/messages/m1"
    end

    def test_search
      mock = MockHttpHelper.new('{"conversations":[{"id":"c1"}]}')
      svc = ConversationService.new(mock)
      result = svc.search(query: "test")
      assert_equal 1, result["conversations"].size
      assert_includes mock.last_call[:path], "q=test"
    end

    def test_export
      mock = MockHttpHelper.new('{"id":"c1","messages":[]}')
      svc = ConversationService.new(mock)
      result = svc.export(id: "c1")
      assert mock.last_call[:path].include?("/api/v1/conversations/c1/export")
    end

    def test_clear_messages
      mock = MockHttpHelper.new('{}')
      svc = ConversationService.new(mock)
      svc.clear_messages(id: "c1")
      assert_equal :delete, mock.last_call[:method]
      assert_equal "/api/v1/conversations/c1/messages", mock.last_call[:path]
    end

    def test_list_error
      mock = MockHttpHelper.new.will_return("{}", 500)
      svc = ConversationService.new(mock)
      assert_raises(ApiError) { svc.list }
    end

    def test_create_error
      mock = MockHttpHelper.new.will_return("{}", 422)
      svc = ConversationService.new(mock)
      assert_raises(ApiError) { svc.create(title: "Bad") }
    end

    # OODA-42: New conversation tests
    def test_share
      mock = MockHttpHelper.new('{"share_id":"sh-1","url":"https://app.co/share/sh-1"}')
      svc = ConversationService.new(mock)
      result = svc.share(id: "c1")
      assert_equal "sh-1", result["share_id"]
      assert_equal :post, mock.last_call[:method]
      assert_includes mock.last_call[:path], "/api/v1/conversations/c1/share"
    end

    def test_unshare
      mock = MockHttpHelper.new('{}')
      svc = ConversationService.new(mock)
      svc.unshare(id: "c1")
      assert_equal :delete, mock.last_call[:method]
      assert_includes mock.last_call[:path], "/api/v1/conversations/c1/share"
    end

    def test_pin
      mock = MockHttpHelper.new('{}')
      svc = ConversationService.new(mock)
      svc.pin(id: "c1")
      assert_equal :post, mock.last_call[:method]
      assert_includes mock.last_call[:path], "/api/v1/conversations/c1/pin"
    end

    def test_unpin
      mock = MockHttpHelper.new('{}')
      svc = ConversationService.new(mock)
      svc.unpin(id: "c1")
      assert_equal :delete, mock.last_call[:method]
      assert_includes mock.last_call[:path], "/api/v1/conversations/c1/pin"
    end

    def test_bulk_delete
      mock = MockHttpHelper.new('{"deleted_count":3}')
      svc = ConversationService.new(mock)
      result = svc.bulk_delete(ids: %w[c1 c2 c3])
      assert_equal 3, result["deleted_count"]
      assert_equal :post, mock.last_call[:method]
      assert_includes mock.last_call[:path], "/api/v1/conversations/bulk/delete"
      assert_equal %w[c1 c2 c3], mock.last_call[:body][:ids]
    end

    def test_bulk_archive
      mock = MockHttpHelper.new('{"archived_count":2}')
      svc = ConversationService.new(mock)
      result = svc.bulk_archive(ids: %w[c1 c2])
      assert_equal 2, result["archived_count"]
      assert_includes mock.last_call[:path], "/api/v1/conversations/bulk/archive"
    end

    def test_bulk_move
      mock = MockHttpHelper.new('{"moved_count":2}')
      svc = ConversationService.new(mock)
      result = svc.bulk_move(ids: %w[c1 c2], folder_id: "f1")
      assert_equal 2, result["moved_count"]
      assert_includes mock.last_call[:path], "/api/v1/conversations/bulk/move"
      body = mock.last_call[:body]
      assert_equal %w[c1 c2], body[:ids]
      assert_equal "f1", body[:folder_id]
    end

    def test_import_conversation
      mock = MockHttpHelper.new('{"id":"c-import","title":"Imported"}')
      svc = ConversationService.new(mock)
      result = svc.import_conversation(data: { title: "Imported", messages: [] })
      assert_equal "c-import", result["id"]
      assert_equal :post, mock.last_call[:method]
      assert_includes mock.last_call[:path], "/api/v1/conversations/import"
    end
  end

  # --- FolderService Tests ---
  class FolderServiceTest < Minitest::Test
    def test_list
      mock = MockHttpHelper.new('{"items":[{"id":"f1","name":"Folder 1"}],"total":1}')
      svc = FolderService.new(mock)
      result = svc.list
      assert_equal 1, result["items"].size
      assert_equal "f1", result["items"][0]["id"]
      assert_equal :get, mock.last_call[:method]
      assert_includes mock.last_call[:path], "/api/v1/folders"
    end

    def test_list_empty
      mock = MockHttpHelper.new('{"items":[],"total":0}')
      svc = FolderService.new(mock)
      result = svc.list
      assert_equal 0, result["items"].size
    end

    def test_create
      mock = MockHttpHelper.new('{"id":"f2","name":"New Folder"}')
      svc = FolderService.new(mock)
      result = svc.create(name: "New Folder")
      assert_equal "f2", result["id"]
      assert_equal :post, mock.last_call[:method]
      assert_equal "New Folder", mock.last_call[:body][:name]
    end

    # OODA-34: New folder method tests
    def test_get
      mock = MockHttpHelper.new('{"id":"f1","name":"Folder 1"}')
      svc = FolderService.new(mock)
      result = svc.get(id: "f1")
      assert_equal "f1", result["id"]
      assert_includes mock.last_call[:path], "/api/v1/folders/f1"
    end

    def test_create_with_parent
      mock = MockHttpHelper.new('{"id":"f3","name":"Subfolder"}')
      svc = FolderService.new(mock)
      result = svc.create(name: "Subfolder", parent_id: "f1")
      assert_equal "f3", result["id"]
      assert_equal "f1", mock.last_call[:body][:parent_id]
    end

    def test_update
      mock = MockHttpHelper.new('{"id":"f1","name":"Updated Name"}')
      svc = FolderService.new(mock)
      result = svc.update(id: "f1", name: "Updated Name")
      assert_equal "Updated Name", result["name"]
      assert_equal :put, mock.last_call[:method]
    end

    def test_delete
      mock = MockHttpHelper.new('{}')
      svc = FolderService.new(mock)
      svc.delete(id: "f1")
      assert_equal :delete, mock.last_call[:method]
      assert_includes mock.last_call[:path], "/api/v1/folders/f1"
    end

    def test_contents
      mock = MockHttpHelper.new('{"items":[{"id":"d1","type":"document"}]}')
      svc = FolderService.new(mock)
      result = svc.contents(id: "f1")
      assert_equal 1, result["items"].size
      assert_includes mock.last_call[:path], "/api/v1/folders/f1/contents"
    end

    def test_list_error
      mock = MockHttpHelper.new.will_return("{}", 500)
      svc = FolderService.new(mock)
      assert_raises(ApiError) { svc.list }
    end

    def test_create_error
      mock = MockHttpHelper.new.will_return("{}", 409)
      svc = FolderService.new(mock)
      assert_raises(ApiError) { svc.create(name: "Duplicate") }
    end
  end

  # --- URL Validation Tests ---
  class UrlValidationTest < Minitest::Test
    def test_health_url
      mock = MockHttpHelper.new('{"status":"ok"}')
      svc = HealthService.new(mock)
      svc.check
      assert_equal "/health", mock.last_call[:path]
    end

    def test_tasks_url
      mock = MockHttpHelper.new('{"tasks":[]}')
      svc = TaskService.new(mock)
      svc.list
      assert_equal "/api/v1/tasks", mock.last_call[:path]
    end

    def test_api_keys_url
      mock = MockHttpHelper.new("[]")
      svc = ApiKeyService.new(mock)
      svc.list
      assert_equal "/api/v1/api-keys", mock.last_call[:path]
    end

    def test_users_url
      mock = MockHttpHelper.new("[]")
      svc = UserService.new(mock)
      svc.list
      assert_equal "/api/v1/users", mock.last_call[:path]
    end

    def test_tenants_url
      mock = MockHttpHelper.new('{"items":[]}')
      svc = TenantService.new(mock)
      svc.list
      assert_equal "/api/v1/tenants", mock.last_call[:path]
    end

    def test_costs_url
      mock = MockHttpHelper.new('{"total_cost_usd":0}')
      svc = CostService.new(mock)
      svc.summary
      assert_equal "/api/v1/costs/summary", mock.last_call[:path]
    end

    def test_pipeline_status_url
      mock = MockHttpHelper.new('{"is_busy":false}')
      svc = PipelineService.new(mock)
      svc.status
      assert_equal "/api/v1/pipeline/status", mock.last_call[:path]
    end

    def test_models_catalog_url
      mock = MockHttpHelper.new('{"providers":[]}')
      svc = ModelService.new(mock)
      svc.catalog
      assert_equal "/api/v1/models", mock.last_call[:path]
    end

    def test_conversations_url
      mock = MockHttpHelper.new('{"items":[]}')
      svc = ConversationService.new(mock)
      svc.list
      assert_equal "/api/v1/conversations", mock.last_call[:path]
    end

    def test_folders_url
      mock = MockHttpHelper.new('{"items":[]}')
      svc = FolderService.new(mock)
      svc.list
      assert_equal "/api/v1/folders", mock.last_call[:path]
    end
  end

  # --- Client Service Availability Tests ---
  class ClientServiceAvailabilityTest < Minitest::Test
    def test_has_conversations_service
      client = Client.new
      assert_instance_of ConversationService, client.conversations
    end

    def test_has_folders_service
      client = Client.new
      assert_instance_of FolderService, client.folders
    end

    def test_all_services_count
      client = Client.new
      services = %i[health documents entities relationships graph query chat
                     tenants users api_keys tasks pipeline models costs
                     conversations folders]
      services.each do |svc_name|
        assert_respond_to client, svc_name, "Client should respond to #{svc_name}"
      end
    end
  end

  # --- Edge Case Tests ---
  class EdgeCaseTest < Minitest::Test
    def test_query_default_mode_not_set
      mock = MockHttpHelper.new('{"answer":"ok"}')
      svc = QueryService.new(mock)
      svc.execute(query: "test")
      # WHY: mode should default to nil/"hybrid" — verify body contains query but not forced mode
      assert_equal "test", mock.last_call[:body][:query]
    end

    def test_chat_default_stream_not_set
      mock = MockHttpHelper.new('{"choices":[]}')
      svc = ChatService.new(mock)
      svc.completions(message: "hello")
      # WHY: stream should only be sent when explicitly set
      assert_equal "hello", mock.last_call[:body][:message]
    end

    def test_entity_pagination_defaults
      mock = MockHttpHelper.new('{"items":[],"total":0}')
      svc = EntityService.new(mock)
      svc.list
      # WHY: verify default pagination params are sent
      assert_includes mock.last_call[:path], "page=1"
      assert_includes mock.last_call[:path], "page_size=20"
    end

    def test_error_status_429
      mock = MockHttpHelper.new.will_return('{"error":"rate limited"}', 429)
      svc = HealthService.new(mock)
      err = assert_raises(ApiError) { svc.check }
      assert_equal 429, err.status_code
    end

    def test_error_status_502
      mock = MockHttpHelper.new.will_return('{"error":"bad gateway"}', 502)
      svc = HealthService.new(mock)
      err = assert_raises(ApiError) { svc.check }
      assert_equal 502, err.status_code
    end

    def test_error_preserves_response_body
      mock = MockHttpHelper.new.will_return('{"detail":"not found"}', 404)
      svc = HealthService.new(mock)
      err = assert_raises(ApiError) { svc.check }
      assert_includes err.response_body, "not found"
    end

    def test_config_strips_single_trailing_slash
      # WHY: Ruby's chomp("/") only strips one trailing slash
      c = Config.new(base_url: "http://example.com/")
      assert_equal "http://example.com", c.base_url
    end
  end

  class MockHttpHelperTest < Minitest::Test
    def test_tracks_all_calls
      mock = MockHttpHelper.new("{}")
      svc = HealthService.new(mock)
      svc.check
      svc.check
      assert_equal 2, mock.calls.size
    end

    def test_error_includes_status_code
      mock = MockHttpHelper.new.will_return('{"error":"not found"}', 404)
      svc = HealthService.new(mock)
      err = assert_raises(ApiError) { svc.check }
      assert_equal 404, err.status_code
    end

    def test_will_return_chaining
      mock = MockHttpHelper.new.will_return('{"a":1}', 200)
      svc = HealthService.new(mock)
      result = svc.check
      assert_equal 1, result["a"]
    end
  end

  # ── Lineage Service Tests ────────────────────────────────────────

  class LineageServiceTest < Minitest::Test
    def test_client_has_lineage_service
      client = Client.new
      assert_instance_of LineageService, client.lineage
    end

    def test_entity_lineage
      mock = MockHttpHelper.new('{"entity_name":"ALICE","entity_type":"person","description_history":[]}')
      svc = LineageService.new(mock)
      result = svc.entity_lineage(name: "ALICE")
      assert_equal "ALICE", result["entity_name"]
      assert_equal "person", result["entity_type"]
      assert_kind_of Array, result["description_history"]
      assert_equal :get, mock.last_call[:method]
      assert_equal "/api/v1/lineage/entities/ALICE", mock.last_call[:path]
    end

    def test_entity_lineage_url_encoding
      mock = MockHttpHelper.new('{"entity_name":"HELLO WORLD"}')
      svc = LineageService.new(mock)
      svc.entity_lineage(name: "HELLO WORLD")
      assert_equal "/api/v1/lineage/entities/HELLO+WORLD", mock.last_call[:path]
    end

    def test_entity_lineage_special_chars
      mock = MockHttpHelper.new('{"entity_name":"O\'BRIEN"}')
      svc = LineageService.new(mock)
      svc.entity_lineage(name: "O'BRIEN")
      assert_includes mock.last_call[:path], "/api/v1/lineage/entities/"
      assert_equal :get, mock.last_call[:method]
    end

    def test_document_lineage
      mock = MockHttpHelper.new('{"document_id":"d1","entities":[],"relationships":[]}')
      svc = LineageService.new(mock)
      result = svc.document_lineage(id: "d1")
      assert_equal "d1", result["document_id"]
      assert_kind_of Array, result["entities"]
      assert_kind_of Array, result["relationships"]
      assert_equal :get, mock.last_call[:method]
      assert_equal "/api/v1/lineage/documents/d1", mock.last_call[:path]
    end

    def test_document_lineage_empty
      mock = MockHttpHelper.new('{"document_id":"d2","entities":[],"relationships":[],"extraction_stats":null}')
      svc = LineageService.new(mock)
      result = svc.document_lineage(id: "d2")
      assert_equal "d2", result["document_id"]
      assert_empty result["entities"]
      assert_nil result["extraction_stats"]
    end

    def test_document_full_lineage
      mock = MockHttpHelper.new('{"document_id":"d1","chunks":[],"total_chunks":5}')
      svc = LineageService.new(mock)
      result = svc.document_full_lineage(id: "d1")
      assert_equal "d1", result["document_id"]
      assert_kind_of Array, result["chunks"]
      assert_equal 5, result["total_chunks"]
      assert_equal :get, mock.last_call[:method]
      assert_equal "/api/v1/documents/d1/lineage", mock.last_call[:path]
    end

    def test_export_lineage_json
      mock = MockHttpHelper.new('{"document_id":"d1","format":"json"}')
      svc = LineageService.new(mock)
      result = svc.export_lineage(id: "d1")
      assert_kind_of String, result
      assert_includes result, "document_id"
      assert_equal :get, mock.last_call[:method]
      assert_equal "/api/v1/documents/d1/lineage/export?format=json", mock.last_call[:path]
    end

    def test_export_lineage_csv
      mock = MockHttpHelper.new("entity_name,entity_type\nALICE,person")
      svc = LineageService.new(mock)
      result = svc.export_lineage(id: "d1", format: "csv")
      assert_kind_of String, result
      assert_includes result, "ALICE"
      assert_equal "/api/v1/documents/d1/lineage/export?format=csv", mock.last_call[:path]
    end

    def test_chunk_detail
      mock = MockHttpHelper.new('{"chunk_id":"c1","content":"hello","entities":[],"relationships":[]}')
      svc = LineageService.new(mock)
      result = svc.chunk_detail(id: "c1")
      assert_equal "c1", result["chunk_id"]
      assert_equal "hello", result["content"]
      assert_kind_of Array, result["entities"]
      assert_equal :get, mock.last_call[:method]
      assert_equal "/api/v1/chunks/c1", mock.last_call[:path]
    end

    def test_chunk_detail_minimal
      mock = MockHttpHelper.new('{"chunk_id":"c2","content":""}')
      svc = LineageService.new(mock)
      result = svc.chunk_detail(id: "c2")
      assert_equal "c2", result["chunk_id"]
      assert_equal "", result["content"]
    end

    def test_chunk_lineage
      mock = MockHttpHelper.new('{"chunk_id":"c1","document_id":"d1","entities":[],"relationships":[]}')
      svc = LineageService.new(mock)
      result = svc.chunk_lineage(id: "c1")
      assert_equal "c1", result["chunk_id"]
      assert_equal "d1", result["document_id"]
      assert_equal :get, mock.last_call[:method]
      assert_equal "/api/v1/chunks/c1/lineage", mock.last_call[:path]
    end

    def test_entity_provenance
      mock = MockHttpHelper.new('{"entity_name":"BOB","source_documents":[],"related_entities":[]}')
      svc = LineageService.new(mock)
      result = svc.entity_provenance(id: "ent-1")
      assert_equal "BOB", result["entity_name"]
      assert_kind_of Array, result["source_documents"]
      assert_kind_of Array, result["related_entities"]
      assert_equal :get, mock.last_call[:method]
      assert_equal "/api/v1/entities/ent-1/provenance", mock.last_call[:path]
    end

    def test_entity_provenance_minimal
      mock = MockHttpHelper.new('{"entity_name":"X"}')
      svc = LineageService.new(mock)
      result = svc.entity_provenance(id: "ent-2")
      assert_equal "X", result["entity_name"]
      assert_equal "/api/v1/entities/ent-2/provenance", mock.last_call[:path]
    end

    def test_lineage_error_handling
      mock = MockHttpHelper.new.will_return('{"error":"Not Found"}', 404)
      svc = LineageService.new(mock)
      assert_raises(ApiError) { svc.entity_lineage(name: "MISSING") }
    end
  end

  # ── OODA-34: New Service Tests ────────────────────────────────────────

  class AuthServiceTest < Minitest::Test
    def test_login
      mock = MockHttpHelper.new('{"token":"jwt-token","user":{"id":"u1"}}')
      svc = AuthService.new(mock)
      result = svc.login(email: "test@test.com", password: "pass123")
      assert_equal "jwt-token", result["token"]
      assert_equal :post, mock.last_call[:method]
      assert_equal "test@test.com", mock.last_call[:body][:email]
      assert_equal "pass123", mock.last_call[:body][:password]
    end

    def test_logout
      mock = MockHttpHelper.new('{"success":true}')
      svc = AuthService.new(mock)
      result = svc.logout
      assert_equal true, result["success"]
      assert_equal :post, mock.last_call[:method]
      assert_includes mock.last_call[:path], "/api/v1/auth/logout"
    end

    def test_refresh
      mock = MockHttpHelper.new('{"token":"new-jwt-token"}')
      svc = AuthService.new(mock)
      result = svc.refresh
      assert_equal "new-jwt-token", result["token"]
      assert_equal :post, mock.last_call[:method]
      assert_includes mock.last_call[:path], "/api/v1/auth/refresh"
    end

    def test_current_user
      mock = MockHttpHelper.new('{"id":"u1","email":"me@test.com","role":"admin"}')
      svc = AuthService.new(mock)
      result = svc.current_user
      assert_equal "u1", result["id"]
      assert_equal "me@test.com", result["email"]
      assert_equal :get, mock.last_call[:method]
      assert_includes mock.last_call[:path], "/api/v1/auth/me"
    end

    def test_login_error
      mock = MockHttpHelper.new.will_return('{"error":"Invalid credentials"}', 401)
      svc = AuthService.new(mock)
      assert_raises(ApiError) { svc.login(email: "bad@test.com", password: "wrong") }
    end
  end

  class WorkspaceServiceTest < Minitest::Test
    def test_list
      mock = MockHttpHelper.new('{"items":[{"id":"ws1","name":"Main"}]}')
      svc = WorkspaceService.new(mock)
      result = svc.list
      assert_equal 1, result["items"].size
      assert_equal :get, mock.last_call[:method]
      assert_includes mock.last_call[:path], "/api/v1/workspaces"
    end

    def test_get
      mock = MockHttpHelper.new('{"id":"ws1","name":"Main Workspace"}')
      svc = WorkspaceService.new(mock)
      result = svc.get(id: "ws1")
      assert_equal "ws1", result["id"]
      assert_includes mock.last_call[:path], "/api/v1/workspaces/ws1"
    end

    def test_create
      mock = MockHttpHelper.new('{"id":"ws2","name":"New Workspace"}')
      svc = WorkspaceService.new(mock)
      result = svc.create(name: "New Workspace", description: "My new workspace")
      assert_equal "ws2", result["id"]
      assert_equal :post, mock.last_call[:method]
      assert_equal "New Workspace", mock.last_call[:body][:name]
      assert_equal "My new workspace", mock.last_call[:body][:description]
    end

    def test_update
      mock = MockHttpHelper.new('{"id":"ws1","name":"Updated Workspace"}')
      svc = WorkspaceService.new(mock)
      result = svc.update(id: "ws1", name: "Updated Workspace")
      assert_equal "Updated Workspace", result["name"]
      assert_equal :put, mock.last_call[:method]
    end

    def test_delete
      mock = MockHttpHelper.new('{}')
      svc = WorkspaceService.new(mock)
      svc.delete(id: "ws1")
      assert_equal :delete, mock.last_call[:method]
      assert_includes mock.last_call[:path], "/api/v1/workspaces/ws1"
    end

    def test_switch
      mock = MockHttpHelper.new('{"current_workspace":"ws2"}')
      svc = WorkspaceService.new(mock)
      result = svc.switch(id: "ws2")
      assert_equal "ws2", result["current_workspace"]
      assert_equal :post, mock.last_call[:method]
      assert_includes mock.last_call[:path], "/api/v1/workspaces/ws2/switch"
    end

    def test_list_error
      mock = MockHttpHelper.new.will_return("{}", 500)
      svc = WorkspaceService.new(mock)
      assert_raises(ApiError) { svc.list }
    end
  end

  class SharedServiceTest < Minitest::Test
    def test_version
      mock = MockHttpHelper.new('{"version":"1.0.0","build":"abc123"}')
      svc = SharedService.new(mock)
      result = svc.version
      assert_equal "1.0.0", result["version"]
      assert_includes mock.last_call[:path], "/api/v1/version"
    end

    def test_settings
      mock = MockHttpHelper.new('{"theme":"dark","language":"en"}')
      svc = SharedService.new(mock)
      result = svc.settings
      assert_equal "dark", result["theme"]
      assert_includes mock.last_call[:path], "/api/v1/settings"
    end

    def test_update_settings
      mock = MockHttpHelper.new('{"theme":"light","language":"fr"}')
      svc = SharedService.new(mock)
      result = svc.update_settings(settings: { theme: "light", language: "fr" })
      assert_equal "light", result["theme"]
      assert_equal :put, mock.last_call[:method]
      assert_includes mock.last_call[:path], "/api/v1/settings"
    end

    def test_metrics
      mock = MockHttpHelper.new('{"requests_total":1000,"active_users":50}')
      svc = SharedService.new(mock)
      result = svc.metrics
      assert_equal 1000, result["requests_total"]
      assert_includes mock.last_call[:path], "/api/v1/metrics"
    end

    def test_version_error
      mock = MockHttpHelper.new.will_return("{}", 500)
      svc = SharedService.new(mock)
      assert_raises(ApiError) { svc.version }
    end
  end

  # ── OODA-50: Additional tests for comprehensive coverage ────────────────────

  class Ooda50DocumentsTest < Minitest::Test
    def test_list_empty_ooda50
      mock = MockHttpHelper.new('{"documents":[],"total":0}')
      svc = DocumentService.new(mock)
      result = svc.list
      assert_empty result["documents"]
      assert_equal "/api/v1/documents?page=1&page_size=20", mock.last_call[:path]
    end

    def test_upload_text_minimal_ooda50
      mock = MockHttpHelper.new('{"id":"d-min","status":"pending"}')
      svc = DocumentService.new(mock)
      result = svc.upload_text(title: "Minimal", content: "")
      assert_equal "d-min", result["id"]
      assert_equal "", mock.last_call[:body][:content]
    end

    def test_track_processing_ooda50
      mock = MockHttpHelper.new('{"track_id":"tk-50","status":"processing","progress":0.4}')
      svc = DocumentService.new(mock)
      result = svc.track(track_id: "tk-50")
      assert_equal "tk-50", result["track_id"]
      assert_includes mock.last_call[:path], "/api/v1/documents/track/tk-50"
    end

    def test_reprocess_failed_ooda50
      mock = MockHttpHelper.new('{"requeued":3,"status":"ok"}')
      svc = DocumentService.new(mock)
      result = svc.reprocess_failed
      assert_equal 3, result["requeued"]
      assert_equal :post, mock.last_call[:method]
    end
  end

  class Ooda50EntitiesTest < Minitest::Test
    def test_list_with_pagination_ooda50
      mock = MockHttpHelper.new('{"items":[],"total":0}')
      svc = EntityService.new(mock)
      svc.list(page: 5, page_size: 100)
      assert_includes mock.last_call[:path], "page=5"
      assert_includes mock.last_call[:path], "page_size=100"
    end

    def test_create_success_ooda50
      mock = MockHttpHelper.new('{"entity_name":"OODA50_ENTITY","entity_type":"concept"}')
      svc = EntityService.new(mock)
      result = svc.create(entity_name: "OODA50_ENTITY", entity_type: "concept", description: "Test entity", source_id: "src-50")
      assert_equal "OODA50_ENTITY", result["entity_name"]
      body = mock.last_call[:body]
      assert_equal "concept", body[:entity_type]
      assert_equal "src-50", body[:source_id]
    end

    def test_update_type_only_ooda50
      mock = MockHttpHelper.new('{"entity_name":"ENT1","entity_type":"organization"}')
      svc = EntityService.new(mock)
      result = svc.update(name: "ENT1", entity_type: "organization")
      assert_equal "organization", result["entity_type"]
      body = mock.last_call[:body]
      assert_equal "organization", body[:entity_type]
      refute body.key?(:description)
    end
  end

  class Ooda50PipelineTest < Minitest::Test
    def test_status_idle_ooda50
      mock = MockHttpHelper.new('{"is_busy":false,"pending_tasks":0}')
      svc = PipelineService.new(mock)
      result = svc.status
      assert_equal false, result["is_busy"]
      assert_equal 0, result["pending_tasks"]
    end

    def test_status_busy_ooda50
      mock = MockHttpHelper.new('{"is_busy":true,"pending_tasks":25}')
      svc = PipelineService.new(mock)
      result = svc.status
      assert_equal true, result["is_busy"]
      assert_equal 25, result["pending_tasks"]
    end

    def test_health_with_workers_ooda50
      mock = MockHttpHelper.new('{"healthy":true,"workers":8,"queue_depth":3}')
      svc = PipelineService.new(mock)
      result = svc.health
      assert_equal 8, result["workers"]
      assert_equal 3, result["queue_depth"]
    end
  end

  class Ooda50TasksTest < Minitest::Test
    def test_get_completed_ooda50
      mock = MockHttpHelper.new('{"id":"task-50","status":"completed","result":{"entities_extracted":42}}')
      svc = TaskService.new(mock)
      result = svc.get(id: "task-50")
      assert_equal "completed", result["status"]
      assert_equal 42, result["result"]["entities_extracted"]
    end

    def test_cancel_success_ooda50
      mock = MockHttpHelper.new('{"id":"task-50","status":"cancelled"}')
      svc = TaskService.new(mock)
      result = svc.cancel(id: "task-50")
      assert_equal "cancelled", result["status"]
      assert_equal :post, mock.last_call[:method]
    end

    def test_status_pending_ooda50
      mock = MockHttpHelper.new('{"id":"task-50","status":"pending","progress":0}')
      svc = TaskService.new(mock)
      result = svc.status(id: "task-50")
      assert_equal "pending", result["status"]
      assert_equal 0, result["progress"]
    end
  end

  class Ooda50ModelsTest < Minitest::Test
    def test_catalog_empty_ooda50
      mock = MockHttpHelper.new('{"providers":[]}')
      svc = ModelService.new(mock)
      result = svc.catalog
      assert_empty result["providers"]
    end

    def test_get_model_ooda50
      mock = MockHttpHelper.new('{"id":"model-50","name":"gpt-4o","provider":"openai"}')
      svc = ModelService.new(mock)
      result = svc.get_model(id: "model-50")
      assert_equal "model-50", result["id"]
      assert_equal "gpt-4o", result["name"]
    end

    def test_set_active_ooda50
      mock = MockHttpHelper.new('{"id":"model-50","active":true}')
      svc = ModelService.new(mock)
      result = svc.set_active(id: "model-50")
      assert_equal true, result["active"]
      assert_includes mock.last_call[:path], "/activate"
    end
  end

  class Ooda50CostsTest < Minitest::Test
    def test_summary_ooda50
      mock = MockHttpHelper.new('{"total_cost_usd":42.50}')
      svc = CostService.new(mock)
      result = svc.summary
      assert_equal 42.50, result["total_cost_usd"]
    end

    def test_breakdown_date_range_ooda50
      mock = MockHttpHelper.new('{"breakdown":[{"category":"embedding","cost":5.0}]}')
      svc = CostService.new(mock)
      result = svc.breakdown(start_date: "2025-01-01", end_date: "2025-01-31")
      path = mock.last_call[:path]
      assert_includes path, "start_date=2025-01-01"
      assert_includes path, "end_date=2025-01-31"
    end

    def test_history_custom_days_ooda50
      mock = MockHttpHelper.new('{"history":[]}')
      svc = CostService.new(mock)
      svc.history(days: 90)
      assert_includes mock.last_call[:path], "days=90"
    end
  end

  class Ooda50GraphTest < Minitest::Test
    def test_popular_labels_ooda50
      mock = MockHttpHelper.new('{"labels":[{"label":"PERSON","count":120},{"label":"ORG","count":45}]}')
      svc = GraphService.new(mock)
      result = svc.popular_labels(limit: 10)
      assert_equal 2, result["labels"].size
      assert_includes mock.last_call[:path], "limit=10"
    end

    def test_degrees_batch_ooda50
      mock = MockHttpHelper.new('{"degrees":[{"node_id":"A","degree":3},{"node_id":"B","degree":1}],"count":2}')
      svc = GraphService.new(mock)
      result = svc.degrees_batch(node_ids: %w[A B C])
      assert_equal 2, result["degrees"].size
      body = mock.last_call[:body]
      assert_equal %w[A B C], body[:node_ids]
    end
  end

  class Ooda50ApiKeysTest < Minitest::Test
    def test_list_empty_ooda50
      mock = MockHttpHelper.new('[]')
      svc = ApiKeyService.new(mock)
      result = svc.list
      assert_empty result
    end

    def test_create_with_permissions_ooda50
      mock = MockHttpHelper.new('{"id":"ak-50","name":"Test Key","permissions":["read","write","delete"]}')
      svc = ApiKeyService.new(mock)
      result = svc.create(name: "Test Key", permissions: %w[read write delete])
      assert_equal "ak-50", result["id"]
      body = mock.last_call[:body]
      assert_equal %w[read write delete], body[:permissions]
    end

    def test_revoke_ooda50
      mock = MockHttpHelper.new('{}')
      svc = ApiKeyService.new(mock)
      svc.revoke(id: "ak-50")
      assert_equal :delete, mock.last_call[:method]
      assert_includes mock.last_call[:path], "/api/v1/api-keys/ak-50"
    end
  end

  class Ooda50UsersTest < Minitest::Test
    def test_list_empty_ooda50
      mock = MockHttpHelper.new('[]')
      svc = UserService.new(mock)
      result = svc.list
      assert_empty result
    end

    def test_create_minimal_ooda50
      mock = MockHttpHelper.new('{"id":"u-50","email":"test@ooda50.com"}')
      svc = UserService.new(mock)
      result = svc.create(email: "test@ooda50.com")
      assert_equal "u-50", result["id"]
      body = mock.last_call[:body]
      assert_equal "user", body[:role]
    end

    def test_update_role_ooda50
      mock = MockHttpHelper.new('{"id":"u-50","role":"admin"}')
      svc = UserService.new(mock)
      result = svc.update(id: "u-50", role: "admin")
      assert_equal "admin", result["role"]
      body = mock.last_call[:body]
      assert_equal "admin", body[:role]
    end
  end

  class Ooda50TenantsTest < Minitest::Test
    def test_create_with_settings_ooda50
      mock = MockHttpHelper.new('{"id":"t-50","name":"OODA50 Tenant","settings":{"max_users":10}}')
      svc = TenantService.new(mock)
      result = svc.create(name: "OODA50 Tenant", settings: { max_users: 10 })
      assert_equal "t-50", result["id"]
      body = mock.last_call[:body]
      assert_equal({ max_users: 10 }, body[:settings])
    end

    def test_update_settings_ooda50
      mock = MockHttpHelper.new('{"id":"t-50","settings":{"max_users":20}}')
      svc = TenantService.new(mock)
      result = svc.update(id: "t-50", settings: { max_users: 20 })
      body = mock.last_call[:body]
      assert_equal({ max_users: 20 }, body[:settings])
    end
  end

  class Ooda50LinageTest < Minitest::Test
    def test_entity_lineage_with_history_ooda50
      mock = MockHttpHelper.new('{"entity_name":"OODA50","description_history":[{"version":1,"description":"First"},{"version":2,"description":"Updated"}]}')
      svc = LineageService.new(mock)
      result = svc.entity_lineage(name: "OODA50")
      assert_equal 2, result["description_history"].size
      assert_equal "Updated", result["description_history"][1]["description"]
    end

    def test_document_lineage_with_stats_ooda50
      mock = MockHttpHelper.new('{"document_id":"d-50","entities":[{"name":"E1"}],"relationships":[{"source":"E1","target":"E2"}],"extraction_stats":{"total_entities":1}}')
      svc = LineageService.new(mock)
      result = svc.document_lineage(id: "d-50")
      assert_equal 1, result["entities"].size
      assert_equal 1, result["relationships"].size
      assert_equal 1, result["extraction_stats"]["total_entities"]
    end

    def test_chunk_lineage_with_parents_ooda50
      mock = MockHttpHelper.new('{"chunk_id":"c-50","document_id":"d-50","line_start":100,"line_end":150,"entities":["E1","E2"]}')
      svc = LineageService.new(mock)
      result = svc.chunk_lineage(id: "c-50")
      assert_equal "c-50", result["chunk_id"]
      assert_equal 100, result["line_start"]
      assert_equal 150, result["line_end"]
    end
  end
end
