# frozen_string_literal: true

require "minitest/autorun"
require_relative "../lib/edgequake"

# E2E tests for EdgeQuake Ruby SDK.
# Requires running backend at localhost:8080 (or EDGEQUAKE_BASE_URL).
class E2ETest < Minitest::Test
  def setup
    base = ENV.fetch("EDGEQUAKE_BASE_URL", "http://localhost:8080")
    tenant_id = ENV.fetch("EDGEQUAKE_TENANT_ID", "00000000-0000-0000-0000-000000000002")
    user_id = ENV.fetch("EDGEQUAKE_USER_ID", "00000000-0000-0000-0000-000000000001")
    config = EdgeQuake::Config.new(base_url: base, tenant_id: tenant_id, user_id: user_id)
    @client = EdgeQuake::Client.new(config: config)
  end

  # 1. Health
  def test_health_check
    h = @client.health.check
    assert_equal "healthy", h["status"]
    refute_nil h["version"]
  end

  # 2. Documents
  def test_documents_list_and_upload
    list = @client.documents.list
    refute_nil list["documents"]
    refute_nil list["total"]

    resp = @client.documents.upload_text(
      title: "Ruby SDK Test #{SecureRandom.hex(4)}",
      content: "Ruby SDK integration test. Knowledge graphs are powerful."
    )
    refute_nil resp["document_id"]
    refute_nil resp["status"]
  end

  # 3. Graph
  def test_graph_get
    g = @client.graph.get
    refute_nil g
  end

  def test_graph_search
    r = @client.graph.search(query: "test")
    refute_nil r
  end

  # 4. Entity CRUD
  def test_entity_crud
    name = "RUBY_TEST_ENTITY_#{SecureRandom.hex(3).upcase}"

    # Create
    created = @client.entities.create(
      entity_name: name,
      entity_type: "TEST",
      description: "Created by Ruby E2E",
      source_id: "ruby-e2e"
    )
    refute_nil created["status"]

    # List
    list = @client.entities.list
    refute_nil list["items"]

    # Get
    fetched = @client.entities.get(name: name)
    refute_nil fetched

    # Delete
    del = @client.entities.delete(name: name)
    refute_nil del["status"]
  end

  # 5. Relationships
  def test_relationships_list
    list = @client.relationships.list
    refute_nil list["items"]
  end

  # 6. Query
  def test_query
    r = @client.query.execute(query: "What is a knowledge graph?", mode: "hybrid")
    refute_nil r["answer"]
  end

  # 7. Chat
  def test_chat
    r = @client.chat.completions(message: "What entities exist?")
    refute_nil r["content"]
  rescue EdgeQuake::ApiError => e
    # Chat may require auth
    assert [401, 403].include?(e.status_code), "Unexpected error: #{e.message}"
  end

  # 8. Tenants
  def test_tenants_list
    list = @client.tenants.list
    refute_nil list["items"]
  end

  # 9. Users
  def test_users_list
    list = @client.users.list
    refute_nil list["users"]
  end

  # 10. API Keys
  def test_api_keys_list
    list = @client.api_keys.list
    refute_nil list["keys"]
  end

  # 11. Tasks
  def test_tasks_list
    list = @client.tasks.list
    refute_nil list["tasks"]
  end

  # 12. Pipeline Status
  def test_pipeline_status
    st = @client.pipeline.status
    refute_nil st.key?("is_busy")
  end

  # 13. Queue Metrics
  def test_queue_metrics
    m = @client.pipeline.queue_metrics
    refute_nil m["pending_count"]
    refute_nil m["active_workers"]
  end

  # 14. Models Catalog
  def test_models_catalog
    cat = @client.models.catalog
    refute_nil cat["providers"]
  end

  # 15. Models Health
  def test_models_health
    items = @client.models.health
    assert items.is_a?(Array), "models health should be an array"
    refute items.empty?, "models health should not be empty"
  end

  # 16. Provider Status
  def test_provider_status
    ps = @client.models.provider_status
    refute_nil ps["provider"]
  end

  # 17. Conversations
  def test_conversations_list
    list = @client.conversations.list
    refute_nil list
  end

  def test_conversations_create
    conv = @client.conversations.create(title: "Ruby E2E Test #{SecureRandom.hex(4)}")
    refute_nil conv
    assert(conv.key?("id") || conv.key?("conversation_id"), "should return conversation id")
  end

  # 18. Folders
  def test_folders_list
    list = @client.folders.list
    assert list.is_a?(Array), "folders should be an array"
  end

  def test_folders_create
    folder = @client.folders.create(name: "Ruby E2E Folder #{SecureRandom.hex(4)}")
    refute_nil folder
    assert(folder.key?("id") || folder.key?("name"), "should return folder")
  end

  # 19. Costs
  def test_costs_summary
    c = @client.costs.summary
    refute_nil c
  end

  # 20. Full Workflow
  def test_full_workflow
    # Upload
    doc = @client.documents.upload_text(
      title: "Ruby Workflow #{SecureRandom.hex(4)}",
      content: "Knowledge graphs connect entities through relationships."
    )
    refute_nil doc["document_id"]

    # Query
    qr = @client.query.execute(query: "What do knowledge graphs connect?", mode: "hybrid")
    refute_nil qr["answer"]

    # Pipeline status
    ps = @client.pipeline.status
    refute_nil ps.key?("is_busy")
  end
end
