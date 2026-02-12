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
  end

  class TenantServiceTest < Minitest::Test
    def test_list
      mock = MockHttpHelper.new('{"items":[{"id":"t1"}]}')
      svc = TenantService.new(mock)
      result = svc.list
      assert_equal 1, result["items"].size
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

    def test_summary_error
      mock = MockHttpHelper.new.will_return("{}", 500)
      svc = CostService.new(mock)
      assert_raises(ApiError) { svc.summary }
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
end
