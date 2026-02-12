# frozen_string_literal: true

module EdgeQuake
  # WHY: Each service maps 1:1 to an API resource for discoverability.

  class HealthService
    def initialize(http) = @http = http
    def check = @http.get("/health")
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
  end

  class RelationshipService
    def initialize(http) = @http = http

    def list(page: 1, page_size: 20)
      @http.get("/api/v1/graph/relationships?page=#{page}&page_size=#{page_size}")
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
  end

  class QueryService
    def initialize(http) = @http = http

    def execute(query:, mode: "hybrid")
      @http.post("/api/v1/query", { query: query, mode: mode })
    end
  end

  class ChatService
    def initialize(http) = @http = http

    def completions(message:, mode: "hybrid", stream: false)
      @http.post("/api/v1/chat/completions", {
        message: message, mode: mode, stream: stream
      })
    end
  end

  class TenantService
    def initialize(http) = @http = http
    def list = @http.get("/api/v1/tenants")
  end

  class UserService
    def initialize(http) = @http = http
    def list = @http.get("/api/v1/users")
  end

  class ApiKeyService
    def initialize(http) = @http = http
    def list = @http.get("/api/v1/api-keys")
  end

  class TaskService
    def initialize(http) = @http = http
    def list = @http.get("/api/v1/tasks")
  end

  class PipelineService
    def initialize(http) = @http = http

    def status
      @http.get("/api/v1/pipeline/status")
    end

    def queue_metrics
      @http.get("/api/v1/pipeline/queue-metrics")
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
  end

  class CostService
    def initialize(http) = @http = http

    def summary
      @http.get("/api/v1/costs/summary")
    end
  end

  class ConversationService
    def initialize(http) = @http = http

    def list
      @http.get("/api/v1/conversations")
    end

    def create(title:, mode: nil, folder_id: nil)
      body = { title: title }
      body[:mode] = mode if mode
      body[:folder_id] = folder_id if folder_id
      @http.post("/api/v1/conversations", body)
    end
  end

  class FolderService
    def initialize(http) = @http = http

    def list
      @http.get("/api/v1/folders")
    end

    def create(name:)
      @http.post("/api/v1/folders", { name: name })
    end
  end
end
