# frozen_string_literal: true

module EdgeQuake
  # WHY: Each service maps 1:1 to an API resource for discoverability.
  # OODA-34: Enhanced with complete API coverage (~80 methods across 20 services).

  class HealthService
    def initialize(http) = @http = http
    def check = @http.get("/health")
    def readiness = @http.get("/ready")
    def liveness = @http.get("/live")
    def metrics = @http.get_raw("/metrics")
  end

  class DocumentService
    def initialize(http) = @http = http

    def list(page: 1, page_size: 20)
      @http.get("/api/v1/documents?page=#{page}&page_size=#{page_size}")
    end

    def get(id:)
      @http.get("/api/v1/documents/#{id}")
    end

    def upload_text(title:, content:, file_type: "txt")
      @http.post("/api/v1/documents", { title: title, content: content, file_type: file_type })
    end

    def delete(id:)
      @http.delete("/api/v1/documents/#{id}")
    end

    def track(track_id:)
      enc = URI.encode_www_form_component(track_id.to_s)
      @http.get("/api/v1/documents/track/#{enc}")
    end

    def reprocess_failed
      @http.post("/api/v1/documents/reprocess", {})
    end

    def recover_stuck
      @http.post("/api/v1/documents/recover-stuck", {})
    end

    # OODA-42: Additional document methods
    def get_metadata(id:)
      @http.get("/api/v1/documents/#{id}/metadata")
    end

    def failed_chunks(id:)
      @http.get("/api/v1/documents/#{id}/failed-chunks")
    end

    def retry_chunks(id:)
      @http.post("/api/v1/documents/#{id}/retry-chunks", {})
    end

    def deletion_impact(id:)
      @http.get("/api/v1/documents/#{id}/deletion-impact")
    end

    def lineage(id:)
      @http.get("/api/v1/documents/#{id}/lineage")
    end
  end

  class EntityService
    def initialize(http) = @http = http

    def list(page: 1, page_size: 20)
      @http.get("/api/v1/graph/entities?page=#{page}&page_size=#{page_size}")
    end

    def get(name:)
      @http.get("/api/v1/graph/entities/#{name}")
    end

    def create(entity_name:, entity_type:, description:, source_id:)
      @http.post("/api/v1/graph/entities", {
        entity_name: entity_name,
        entity_type: entity_type,
        description: description,
        source_id: source_id
      })
    end

    def delete(name:)
      @http.delete("/api/v1/graph/entities/#{name}?confirm=true")
    end

    def exists?(name:)
      @http.get("/api/v1/graph/entities/exists?entity_name=#{name}")
    end

    def update(name:, description: nil, entity_type: nil)
      body = {}
      body[:description] = description if description
      body[:entity_type] = entity_type if entity_type
      @http.put("/api/v1/graph/entities/#{name}", body)
    end

    def merge(source_name:, target_name:)
      @http.post("/api/v1/graph/entities/merge", {
        source_name: source_name,
        target_name: target_name
      })
    end

    def neighborhood(name:, depth: 1)
      enc = URI.encode_www_form_component(name.to_s)
      @http.get("/api/v1/graph/entities/#{enc}/neighborhood?depth=#{depth}")
    end
  end

  class RelationshipService
    def initialize(http) = @http = http

    def list(page: 1, page_size: 20)
      @http.get("/api/v1/graph/relationships?page=#{page}&page_size=#{page_size}")
    end

    def create(src_id:, tgt_id:, keywords:, description:, source_id: "manual_entry", weight: 0.8, metadata: {})
      body = {
        src_id: src_id,
        tgt_id: tgt_id,
        keywords: keywords,
        description: description,
        source_id: source_id,
        weight: weight
      }
      body[:metadata] = metadata if metadata && !metadata.empty?
      @http.post("/api/v1/graph/relationships", body)
    end

    def get(id:)
      enc = URI.encode_www_form_component(id.to_s)
      @http.get("/api/v1/graph/relationships/#{enc}")
    end

    def delete(id:)
      @http.delete("/api/v1/graph/relationships/#{id}")
    end
  end

  class GraphService
    def initialize(http) = @http = http

    def get
      @http.get("/api/v1/graph")
    end

    def search(query:)
      encoded = URI.encode_www_form_component(query)
      @http.get("/api/v1/graph/nodes/search?q=#{encoded}")
    end

    def get_node(node_id:)
      enc = URI.encode_www_form_component(node_id.to_s)
      @http.get("/api/v1/graph/nodes/#{enc}")
    end

    def label_search(query:, limit: 20)
      q = URI.encode_www_form_component(query)
      @http.get("/api/v1/graph/labels/search?q=#{q}&limit=#{limit}")
    end

    def popular_labels(limit: 20)
      @http.get("/api/v1/graph/labels/popular?limit=#{limit}")
    end

    def degrees_batch(node_ids:)
      @http.post("/api/v1/graph/degrees/batch", { node_ids: node_ids })
    end
  end

  class QueryService
    def initialize(http) = @http = http

    def execute(query:, mode: "hybrid", system_prompt: nil)
      body = { query: query, mode: mode }
      body[:system_prompt] = system_prompt if system_prompt
      @http.post("/api/v1/query", body)
    end

    # OODA-42: Additional query methods
    def execute_with_context(query:, mode: "hybrid", top_k: 5, only_need_context: false)
      @http.post("/api/v1/query", {
        query: query,
        mode: mode,
        top_k: top_k,
        only_need_context: only_need_context
      })
    end

    def stream(query:, mode: "hybrid", &block)
      Enumerator.new do |yielder|
        @http.stream_post("/api/v1/query/stream", { query: query, mode: mode, stream: true }) do |chunk|
          yielder << chunk
        end
      end
    end
  end

  class ChatService
    def initialize(http) = @http = http

    def completions(message:, mode: "hybrid", stream: false, system_prompt: nil)
      body = { message: message, mode: mode, stream: stream }
      body[:system_prompt] = system_prompt if system_prompt
      @http.post("/api/v1/chat/completions", body)
    end

    # OODA-42: Additional chat methods
    def completions_with_conversation(message:, conversation_id:, mode: "hybrid", stream: false)
      @http.post("/api/v1/chat/completions", {
        message: message,
        conversation_id: conversation_id,
        mode: mode,
        stream: stream
      })
    end

    def stream(message:, mode: "hybrid", conversation_id: nil, &block)
      body = { message: message, mode: mode, stream: true }
      body[:conversation_id] = conversation_id if conversation_id
      Enumerator.new do |yielder|
        @http.stream_post("/api/v1/chat/completions/stream", body) do |chunk|
          yielder << chunk
        end
      end
    end
  end

  class TenantService
    def initialize(http) = @http = http
    def list = @http.get("/api/v1/tenants")
    def get(id:) = @http.get("/api/v1/tenants/#{id}")
    def create(name:, settings: nil)
      body = { name: name }
      body[:settings] = settings if settings
      @http.post("/api/v1/tenants", body)
    end
    def update(id:, name: nil, settings: nil)
      body = {}
      body[:name] = name if name
      body[:settings] = settings if settings
      @http.put("/api/v1/tenants/#{id}", body)
    end
    def delete(id:) = @http.delete("/api/v1/tenants/#{id}")
  end

  class UserService
    def initialize(http) = @http = http
    def list = @http.get("/api/v1/users")
    def get(id:) = @http.get("/api/v1/users/#{id}")
    def create(email:, name: nil, role: "user")
      body = { email: email, role: role }
      body[:name] = name if name
      @http.post("/api/v1/users", body)
    end
    def update(id:, name: nil, role: nil)
      body = {}
      body[:name] = name if name
      body[:role] = role if role
      @http.put("/api/v1/users/#{id}", body)
    end
    def delete(id:) = @http.delete("/api/v1/users/#{id}")
  end

  class ApiKeyService
    def initialize(http) = @http = http
    def list = @http.get("/api/v1/api-keys")
    def get(id:) = @http.get("/api/v1/api-keys/#{id}")
    def create(name:, permissions: [])
      @http.post("/api/v1/api-keys", { name: name, permissions: permissions })
    end
    def revoke(id:) = @http.delete("/api/v1/api-keys/#{id}")
    def rotate(id:) = @http.post("/api/v1/api-keys/#{id}/rotate", {})
  end

  class TaskService
    def initialize(http) = @http = http
    def list = @http.get("/api/v1/tasks")
    def get(id:) = @http.get("/api/v1/tasks/#{id}")
    def create(task_type:, parameters: {})
      @http.post("/api/v1/tasks", { task_type: task_type, parameters: parameters })
    end
    def cancel(id:) = @http.post("/api/v1/tasks/#{id}/cancel", {})
    def status(id:) = @http.get("/api/v1/tasks/#{id}/status")
  end

  class PipelineService
    def initialize(http) = @http = http

    def status
      @http.get("/api/v1/pipeline/status")
    end

    def queue_metrics
      @http.get("/api/v1/pipeline/queue-metrics")
    end

    def health
      @http.get("/api/v1/pipeline/health")
    end
  end

  class ModelService
    def initialize(http) = @http = http

    def catalog
      @http.get("/api/v1/models")
    end

    def health
      raw = @http.get_raw("/api/v1/models/health")
      JSON.parse(raw, symbolize_names: false)
    end

    def provider_status
      @http.get("/api/v1/settings/provider/status")
    end

    def list_providers
      @http.get("/api/v1/models/providers")
    end

    def get_model(id:)
      @http.get("/api/v1/models/#{id}")
    end

    def set_active(id:)
      @http.post("/api/v1/models/#{id}/activate", {})
    end

    def usage(id: nil, days: 7)
      path = id ? "/api/v1/models/#{id}/usage?days=#{days}" : "/api/v1/models/usage?days=#{days}"
      @http.get(path)
    end
  end

  class CostService
    def initialize(http) = @http = http

    def summary
      @http.get("/api/v1/costs/summary")
    end

    def breakdown(start_date: nil, end_date: nil)
      params = []
      params << "start_date=#{start_date}" if start_date
      params << "end_date=#{end_date}" if end_date
      query = params.empty? ? "" : "?#{params.join("&")}"
      @http.get("/api/v1/costs/breakdown#{query}")
    end

    def by_model(days: 30)
      @http.get("/api/v1/costs/by-model?days=#{days}")
    end

    def by_tenant(days: 30)
      @http.get("/api/v1/costs/by-tenant?days=#{days}")
    end

    def history(days: 30)
      @http.get("/api/v1/costs/history?days=#{days}")
    end
  end

  class ConversationService
    def initialize(http) = @http = http

    def list
      @http.get("/api/v1/conversations")
    end

    def get(id:)
      @http.get("/api/v1/conversations/#{id}")
    end

    def create(title:, mode: nil, folder_id: nil)
      body = { title: title }
      body[:mode] = mode if mode
      body[:folder_id] = folder_id if folder_id
      @http.post("/api/v1/conversations", body)
    end

    def update(id:, title: nil, folder_id: nil)
      body = {}
      body[:title] = title if title
      body[:folder_id] = folder_id if folder_id
      @http.put("/api/v1/conversations/#{id}", body)
    end

    def delete(id:)
      @http.delete("/api/v1/conversations/#{id}")
    end

    def messages(id:)
      @http.get("/api/v1/conversations/#{id}/messages")
    end

    def add_message(id:, role:, content:)
      @http.post("/api/v1/conversations/#{id}/messages", { role: role, content: content })
    end

    def delete_message(conversation_id:, message_id:)
      @http.delete("/api/v1/conversations/#{conversation_id}/messages/#{message_id}")
    end

    def search(query:)
      encoded = URI.encode_www_form_component(query)
      @http.get("/api/v1/conversations/search?q=#{encoded}")
    end

    def export(id:, format: "json")
      @http.get("/api/v1/conversations/#{id}/export?format=#{format}")
    end

    def clear_messages(id:)
      @http.delete("/api/v1/conversations/#{id}/messages")
    end

    # OODA-42: Additional conversation methods
    def share(id:)
      @http.post("/api/v1/conversations/#{id}/share", {})
    end

    def unshare(id:)
      @http.delete("/api/v1/conversations/#{id}/share")
    end

    def pin(id:)
      @http.post("/api/v1/conversations/#{id}/pin", {})
    end

    def unpin(id:)
      @http.delete("/api/v1/conversations/#{id}/pin")
    end

    def bulk_delete(ids:)
      @http.post("/api/v1/conversations/bulk/delete", { ids: ids })
    end

    def bulk_archive(ids:)
      @http.post("/api/v1/conversations/bulk/archive", { ids: ids })
    end

    def bulk_move(ids:, folder_id:)
      @http.post("/api/v1/conversations/bulk/move", { ids: ids, folder_id: folder_id })
    end

    def import_conversation(data:)
      @http.post("/api/v1/conversations/import", data)
    end
  end

  class FolderService
    def initialize(http) = @http = http

    def list
      @http.get("/api/v1/folders")
    end

    def get(id:)
      @http.get("/api/v1/folders/#{id}")
    end

    def create(name:, parent_id: nil)
      body = { name: name }
      body[:parent_id] = parent_id if parent_id
      @http.post("/api/v1/folders", body)
    end

    def update(id:, name:)
      @http.put("/api/v1/folders/#{id}", { name: name })
    end

    def delete(id:)
      @http.delete("/api/v1/folders/#{id}")
    end

    def contents(id:)
      @http.get("/api/v1/folders/#{id}/contents")
    end
  end

  # OODA-34: New services for complete API coverage.

  class AuthService
    def initialize(http) = @http = http

    def login(email:, password:)
      @http.post("/api/v1/auth/login", { email: email, password: password })
    end

    def logout
      @http.post("/api/v1/auth/logout", {})
    end

    def refresh
      @http.post("/api/v1/auth/refresh", {})
    end

    def current_user
      @http.get("/api/v1/auth/me")
    end
  end

  class WorkspaceService
    def initialize(http) = @http = http

    def list
      @http.get("/api/v1/workspaces")
    end

    def get(id:)
      @http.get("/api/v1/workspaces/#{id}")
    end

    def create(name:, description: nil)
      body = { name: name }
      body[:description] = description if description
      @http.post("/api/v1/workspaces", body)
    end

    def update(id:, name: nil, description: nil)
      body = {}
      body[:name] = name if name
      body[:description] = description if description
      @http.put("/api/v1/workspaces/#{id}", body)
    end

    def delete(id:)
      @http.delete("/api/v1/workspaces/#{id}")
    end

    def switch(id:)
      @http.post("/api/v1/workspaces/#{id}/switch", {})
    end
  end

  class SharedService
    def initialize(http) = @http = http

    def version
      @http.get("/api/v1/version")
    end

    def settings
      @http.get("/api/v1/settings")
    end

    def update_settings(settings:)
      @http.put("/api/v1/settings", settings)
    end

    def metrics
      @http.get("/api/v1/metrics")
    end
  end

  # WHY: Lineage & provenance service — maps 7 lineage API endpoints.
  # OODA-28: Ruby SDK lineage service.
  class LineageService
    def initialize(http) = @http = http

    # Get entity lineage showing all source documents.
    def entity_lineage(name:)
      encoded = URI.encode_www_form_component(name)
      @http.get("/api/v1/lineage/entities/#{encoded}")
    end

    # Get document graph lineage with entities and relationships.
    def document_lineage(id:)
      @http.get("/api/v1/lineage/documents/#{id}")
    end

    # Get full document lineage including metadata.
    def document_full_lineage(id:)
      @http.get("/api/v1/documents/#{id}/lineage")
    end

    # Export document lineage as JSON or CSV. Returns raw string.
    def export_lineage(id:, format: "json")
      fmt = URI.encode_www_form_component(format)
      @http.get_raw("/api/v1/documents/#{id}/lineage/export?format=#{fmt}")
    end

    # Get chunk detail with extracted entities and relationships.
    def chunk_detail(id:)
      @http.get("/api/v1/chunks/#{id}")
    end

    # Get chunk lineage with parent document references.
    def chunk_lineage(id:)
      @http.get("/api/v1/chunks/#{id}/lineage")
    end

    # Get entity provenance with source documents and related entities.
    def entity_provenance(id:)
      @http.get("/api/v1/entities/#{id}/provenance")
    end
  end
end
