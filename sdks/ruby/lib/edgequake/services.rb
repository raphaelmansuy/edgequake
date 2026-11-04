# frozen_string_literal: true

require "uri"

module EdgeQuake
  class Service
    def initialize(http)
      @http = http
    end

    private

    def encode(value) = URI.encode_www_form_component(value.to_s).gsub("+", "%20")
    def query(values) = URI.encode_www_form(values.reject { |_, value| value.nil? })
  end

  class HealthService < Service
    def check = @http.get("/health")
    def readiness = @http.get("/ready")
    def liveness = @http.get("/live")
    def metrics = @http.get_raw("/metrics")
  end

  class DocumentService < Service
    BASE = "/api/v1/documents"

    def list(page: 1, page_size: 20) = @http.get("#{BASE}?#{query(page: page, page_size: page_size)}")
    def get(id:) = @http.get("#{BASE}/#{encode(id)}")
    def upload_text(title:, content:, file_type: "txt") = @http.post(BASE, { title: title, content: content, file_type: file_type })
    def delete(id:) = @http.delete("#{BASE}/#{encode(id)}")
    def track(track_id:) = @http.get("#{BASE}/track/#{encode(track_id)}")
    def reprocess_failed = @http.post("#{BASE}/reprocess")
    def recover_stuck = @http.post("#{BASE}/recover-stuck")
    def get_metadata(id:) = @http.get("#{BASE}/#{encode(id)}/metadata")
    def failed_chunks(id:) = @http.get("#{BASE}/#{encode(id)}/failed-chunks")
    def retry_chunks(id:) = @http.post("#{BASE}/#{encode(id)}/retry-chunks")
    def deletion_impact(id:) = @http.get("#{BASE}/#{encode(id)}/deletion-impact")
    def lineage(id:) = @http.get("#{BASE}/#{encode(id)}/lineage")
  end

  class EntityService < Service
    BASE = "/api/v1/graph/entities"

    def list(page: 1, page_size: 20) = @http.get("#{BASE}?#{query(page: page, page_size: page_size)}")
    def get(name:) = @http.get("#{BASE}/#{encode(name)}")

    def create(entity_name:, entity_type:, description:, source_id:)
      @http.post(BASE, {
        entity_name: entity_name,
        entity_type: entity_type,
        description: description,
        source_id: source_id
      })
    end

    def update(name:, description: nil, entity_type: nil)
      @http.put("#{BASE}/#{encode(name)}", { description: description, entity_type: entity_type }.compact)
    end

    def delete(name:) = @http.delete("#{BASE}/#{encode(name)}?confirm=true")
    def exists?(name:) = @http.get("#{BASE}/exists?#{query(entity_name: name)}")
    def merge(source_name:, target_name:) = @http.post("#{BASE}/merge", { source_entity: source_name, target_entity: target_name })
    def neighborhood(name:, depth: 1) = @http.get("#{BASE}/#{encode(name)}/neighborhood?#{query(depth: depth)}")
  end

  class RelationshipService < Service
    BASE = "/api/v1/graph/relationships"

    def list(page: 1, page_size: 20) = @http.get("#{BASE}?#{query(page: page, page_size: page_size)}")

    # WHY: the server's CreateRelationshipRequest requires `source_id`
    # (no serde default), so omitting it makes every create fail with 422.
    def create(src_id:, tgt_id:, keywords:, description:, source_id: "manual_entry", weight: nil)
      @http.post(BASE, {
        src_id: src_id,
        tgt_id: tgt_id,
        keywords: keywords,
        description: description,
        source_id: source_id,
        weight: weight
      }.compact)
    end

    def get(id:) = @http.get("#{BASE}/#{encode(id)}")
    def delete(id:) = @http.delete("#{BASE}/#{encode(id)}")
  end

  class GraphService < Service
    BASE = "/api/v1/graph"

    def get = @http.get(BASE)
    def search(query:) = @http.get("#{BASE}/nodes/search?#{self.query(q: query)}")
    def get_node(node_id:) = @http.get("#{BASE}/nodes/#{encode(node_id)}")
    def label_search(query:) = @http.get("#{BASE}/labels/search?#{self.query(q: query)}")
    def popular_labels(limit: 20) = @http.get("#{BASE}/labels/popular?#{query(limit: limit)}")
    def degrees_batch(node_ids:) = @http.post("#{BASE}/degrees/batch", { node_ids: node_ids })
  end

  class QueryService < Service
    BASE = "/api/v1/query"

    def execute(query:, mode: nil)
      @http.post(BASE, { query: query, mode: mode }.compact)
    end

    def execute_with_context(query:, top_k: nil, only_need_context: nil, mode: nil)
      @http.post(BASE, {
        query: query,
        max_results: top_k,
        only_need_context: only_need_context,
        mode: mode
      }.compact)
    end

    def stream(query:, mode: nil)
      @http.post_events("#{BASE}/stream", { query: query, mode: mode }.compact)
    end
  end

  class ChatService < Service
    BASE = "/api/v1/chat/completions"

    def completions(message:, mode: nil, stream: nil, top_k: nil)
      @http.post(BASE, { message: message, mode: mode, stream: stream, top_k: top_k }.compact)
    end

    def completions_with_conversation(message:, conversation_id:, mode: nil, stream: nil)
      @http.post(BASE, {
        message: message,
        conversation_id: conversation_id,
        mode: mode,
        stream: stream
      }.compact)
    end

    def stream(message:, mode: nil)
      @http.post_events("#{BASE}/stream", { message: message, mode: mode }.compact)
    end
  end

  class TenantService < Service
    BASE = "/api/v1/tenants"

    def list = @http.get(BASE)
    def get(id:) = @http.get("#{BASE}/#{encode(id)}")
    def create(name:, settings: nil) = @http.post(BASE, { name: name, settings: settings }.compact)
    def update(id:, name: nil, settings: nil) = @http.put("#{BASE}/#{encode(id)}", { name: name, settings: settings }.compact)
    def delete(id:) = @http.delete("#{BASE}/#{encode(id)}")
  end

  class UserService < Service
    BASE = "/api/v1/users"

    def list = @http.get(BASE)
    def get(id:) = @http.get("#{BASE}/#{encode(id)}")
    def create(email:, name: nil, role: "user") = @http.post(BASE, { email: email, name: name, role: role }.compact)
    def update(id:, name: nil, role: nil) = @http.patch("#{BASE}/#{encode(id)}", { display_name: name, role: role }.compact)
    def delete(id:) = @http.delete("#{BASE}/#{encode(id)}")
  end

  class ApiKeyService < Service
    BASE = "/api/v1/api-keys"

    def list = @http.get(BASE)
    def create(name:, permissions:) = @http.post(BASE, { name: name, permissions: permissions })
    def revoke(id:) = @http.delete("#{BASE}/#{encode(id)}")
  end

  class TaskService < Service
    BASE = "/api/v1/tasks"

    def list = @http.get(BASE)
    def get(id:) = @http.get("#{BASE}/#{encode(id)}")
    def cancel(id:) = @http.post("#{BASE}/#{encode(id)}/cancel")
  end

  class PipelineService < Service
    BASE = "/api/v1/pipeline"

    def status = @http.get("#{BASE}/status")
    def queue_metrics = @http.get("#{BASE}/queue-metrics")
  end

  class ModelService < Service
    BASE = "/api/v1/models"

    def catalog = @http.get(BASE)
    def health = @http.get("#{BASE}/health")
    def provider_status = @http.get("/api/v1/settings/provider/status")
    def list_providers = @http.get("/api/v1/settings/providers")
    def get_model(id:) = @http.get("#{BASE}/#{encode(id)}")
  end

  class CostService < Service
    BASE = "/api/v1/costs"

    def summary = @http.get("#{BASE}/summary")
    def history(days: 30) = @http.get("#{BASE}/history?#{query(days: days)}")
  end

  class ConversationService < Service
    BASE = "/api/v1/conversations"

    def list = @http.get(BASE)
    def get(id:) = @http.get("#{BASE}/#{encode(id)}")
    def create(title:, mode: nil, folder_id: nil) = @http.post(BASE, { title: title, mode: mode, folder_id: folder_id }.compact)
    def update(id:, title:) = @http.patch("#{BASE}/#{encode(id)}", { title: title })
    def delete(id:) = @http.delete("#{BASE}/#{encode(id)}")
    def messages(id:) = @http.get("#{BASE}/#{encode(id)}/messages")
    def add_message(id:, role:, content:) = @http.post("#{BASE}/#{encode(id)}/messages", { role: role, content: content })

    def delete_message(message_id:)
      @http.delete("/api/v1/messages/#{encode(message_id)}")
    end

    def share(id:) = @http.post("#{BASE}/#{encode(id)}/share")
    def unshare(id:) = @http.delete("#{BASE}/#{encode(id)}/share")
    def bulk_delete(ids:) = @http.post("#{BASE}/bulk/delete", { conversation_ids: ids })
    def bulk_archive(ids:, archive: true) = @http.post("#{BASE}/bulk/archive", { conversation_ids: ids, archive: archive })
    def bulk_move(ids:, folder_id:) = @http.post("#{BASE}/bulk/move", { conversation_ids: ids, folder_id: folder_id })
    def import_conversation(data:) = @http.post("#{BASE}/import", data)
  end

  class FolderService < Service
    BASE = "/api/v1/folders"

    def list = @http.get(BASE)
    def create(name:, parent_id: nil) = @http.post(BASE, { name: name, parent_id: parent_id }.compact)
    def update(id:, name:) = @http.patch("#{BASE}/#{encode(id)}", { name: name })
    def delete(id:) = @http.delete("#{BASE}/#{encode(id)}")
  end

  class LineageService < Service
    def entity_lineage(name:) = @http.get("/api/v1/lineage/entities/#{encode(name)}")
    def document_lineage(id:) = @http.get("/api/v1/lineage/documents/#{encode(id)}")
    def document_full_lineage(id:) = @http.get("/api/v1/documents/#{encode(id)}/lineage")
    def export_lineage(id:, format: "json") = @http.get_raw("/api/v1/documents/#{encode(id)}/lineage/export?#{query(format: format)}")
    def chunk_detail(id:) = @http.get("/api/v1/chunks/#{encode(id)}")
    def chunk_lineage(id:) = @http.get("/api/v1/chunks/#{encode(id)}/lineage")
    def entity_provenance(id:) = @http.get("/api/v1/entities/#{encode(id)}/provenance")
  end

  class AuthService < Service
    BASE = "/api/v1/auth"

    def login(email:, password:) = @http.post("#{BASE}/login", { username: email, password: password })
    def logout = @http.post("#{BASE}/logout")
    def refresh = @http.post("#{BASE}/refresh")
    def current_user = @http.get("#{BASE}/me")
  end

  class WorkspaceService < Service
    BASE = "/api/v1/workspaces"

    def get(id:) = @http.get("#{BASE}/#{encode(id)}")
    def update(id:, name: nil, description: nil) = @http.put("#{BASE}/#{encode(id)}", { name: name, description: description }.compact)
    def delete(id:) = @http.delete("#{BASE}/#{encode(id)}")
  end

  class SharedService < Service
    def version = @http.get("/api/version")
    def metrics = @http.get("/metrics")
  end

  class ConnectionService < Service
    BASE = "/api/v1/connections"

    def list = @http.get(BASE)

    def create(slug:, display_name:, api_shape:, base_url:, locality: nil,
               auth_scheme: nil, api_key: nil, timeout_secs: nil,
               allow_private_network: nil)
      @http.post(BASE, connection_body(
        slug: slug, display_name: display_name, api_shape: api_shape,
        base_url: base_url, locality: locality, auth_scheme: auth_scheme,
        api_key: api_key, timeout_secs: timeout_secs,
        allow_private_network: allow_private_network
      ))
    end

    def update(id:, slug:, display_name:, api_shape:, base_url:, locality: nil,
               auth_scheme: nil, api_key: nil, timeout_secs: nil,
               allow_private_network: nil)
      @http.put("#{BASE}/#{encode(id)}", connection_body(
        slug: slug, display_name: display_name, api_shape: api_shape,
        base_url: base_url, locality: locality, auth_scheme: auth_scheme,
        api_key: api_key, timeout_secs: timeout_secs,
        allow_private_network: allow_private_network
      ))
    end

    def delete(id:) = @http.delete("#{BASE}/#{encode(id)}")
    def test_stored(id:) = @http.post("#{BASE}/#{encode(id)}/test")

    def test_provider(shape:, base_url: nil, model: nil, embedding_model: nil,
                      api_key: nil, auth_scheme: nil, allow_private_network: nil,
                      expected_dimension: nil)
      @http.post("/api/v1/providers/test", {
        shape: shape,
        base_url: base_url,
        model: model,
        embedding_model: embedding_model,
        api_key: api_key,
        auth_scheme: auth_scheme,
        allow_private_network: allow_private_network,
        expected_dimension: expected_dimension
      }.compact)
    end

    private

    def connection_body(slug:, display_name:, api_shape:, base_url:, locality:,
                        auth_scheme:, api_key:, timeout_secs:, allow_private_network:)
      {
        slug: slug,
        display_name: display_name,
        api_shape: api_shape,
        base_url: base_url,
        locality: locality,
        auth_scheme: auth_scheme,
        api_key: api_key,
        timeout_secs: timeout_secs,
        allow_private_network: allow_private_network
      }.compact
    end
  end
end
