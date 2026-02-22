error id: file://<WORKSPACE>/sdks/java/src/main/java/io/edgequake/sdk/models/AuthModels.java:com/fasterxml/jackson/annotation/JsonProperty#
file://<WORKSPACE>/sdks/java/src/main/java/io/edgequake/sdk/models/AuthModels.java
empty definition using pc, found symbol in pc: com/fasterxml/jackson/annotation/JsonProperty#
empty definition using semanticdb
empty definition using fallback
non-local guesses:

offset: 136
uri: file://<WORKSPACE>/sdks/java/src/main/java/io/edgequake/sdk/models/AuthModels.java
text:
```scala
package io.edgequake.sdk.models;

import com.fasterxml.jackson.annotation.JsonIgnoreProperties;
import com.fasterxml.jackson.annotation.@@JsonProperty;
import java.util.List;

/** Auth, tenant, user, and API key model classes. */
public class AuthModels {

    // ── Auth ─────────────────────────────────────────────────────────

    public static class LoginRequest {
        @JsonProperty("username") public String username;
        @JsonProperty("password") public String password;

        public LoginRequest() {}
        public LoginRequest(String username, String password) {
            this.username = username;
            this.password = password;
        }
    }

    public static class TokenResponse {
        @JsonProperty("access_token") public String accessToken;
        @JsonProperty("refresh_token") public String refreshToken;
        @JsonProperty("token_type") public String tokenType;
        @JsonProperty("expires_in") public Integer expiresIn;
    }

    public static class RefreshRequest {
        @JsonProperty("refresh_token") public String refreshToken;

        public RefreshRequest() {}
        public RefreshRequest(String refreshToken) { this.refreshToken = refreshToken; }
    }

    // ── Users ────────────────────────────────────────────────────────

    public static class UserInfo {
        @JsonProperty("id") public String id;
        @JsonProperty("username") public String username;
        @JsonProperty("email") public String email;
        @JsonProperty("role") public String role;
    }

    public static class CreateUserRequest {
        @JsonProperty("username") public String username;
        @JsonProperty("email") public String email;
        @JsonProperty("password") public String password;
        @JsonProperty("role") public String role;
    }

    /** WHY: Users list uses "users" key (not "items"). */
    public static class UserListResponse {
        @JsonProperty("users") public List<UserInfo> users;
        @JsonProperty("total") public int total;
        @JsonProperty("page") public int page;
        @JsonProperty("page_size") public int pageSize;
        @JsonProperty("total_pages") public int totalPages;
    }

    // ── API Keys ─────────────────────────────────────────────────────

    public static class ApiKeyResponse {
        @JsonProperty("id") public String id;
        @JsonProperty("key") public String key;
        @JsonProperty("name") public String name;
        @JsonProperty("created_at") public String createdAt;
    }

    public static class ApiKeyInfo {
        @JsonProperty("id") public String id;
        @JsonProperty("name") public String name;
        @JsonProperty("created_at") public String createdAt;
    }

    /** WHY: API keys list uses "keys" key (not "items"). */
    public static class ApiKeyListResponse {
        @JsonProperty("keys") public List<ApiKeyInfo> keys;
        @JsonProperty("total") public int total;
        @JsonProperty("page") public int page;
        @JsonProperty("page_size") public int pageSize;
        @JsonProperty("total_pages") public int totalPages;
    }

    public static class CreateApiKeyRequest {
        @JsonProperty("name") public String name;

        public CreateApiKeyRequest() {}
        public CreateApiKeyRequest(String name) { this.name = name; }
    }

    // ── Tenants ──────────────────────────────────────────────────────

    public static class TenantInfo {
        @JsonProperty("id") public String id;
        @JsonProperty("name") public String name;
        @JsonProperty("slug") public String slug;
        @JsonProperty("plan") public String plan;
        @JsonProperty("is_active") public boolean isActive;
        @JsonProperty("max_workspaces") public int maxWorkspaces;
        // Default LLM configuration for new workspaces.
        @JsonProperty("default_llm_model") public String defaultLlmModel;
        @JsonProperty("default_llm_provider") public String defaultLlmProvider;
        @JsonProperty("default_llm_full_id") public String defaultLlmFullId;
        // Default embedding configuration for new workspaces.
        @JsonProperty("default_embedding_model") public String defaultEmbeddingModel;
        @JsonProperty("default_embedding_provider") public String defaultEmbeddingProvider;
        @JsonProperty("default_embedding_dimension") public int defaultEmbeddingDimension;
        @JsonProperty("default_embedding_full_id") public String defaultEmbeddingFullId;
        // Default vision LLM for PDF image extraction (SPEC-041).
        @JsonProperty("default_vision_llm_model") public String defaultVisionLlmModel;
        @JsonProperty("default_vision_llm_provider") public String defaultVisionLlmProvider;
        @JsonProperty("created_at") public String createdAt;
        @JsonProperty("updated_at") public String updatedAt;
    }

    public static class CreateTenantRequest {
        @JsonProperty("name") public String name;
        @JsonProperty("slug") public String slug;
        @JsonProperty("description") public String description;
        @JsonProperty("plan") public String plan;
        // Default LLM configuration.
        @JsonProperty("default_llm_model") public String defaultLlmModel;
        @JsonProperty("default_llm_provider") public String defaultLlmProvider;
        // Default embedding configuration.
        @JsonProperty("default_embedding_model") public String defaultEmbeddingModel;
        @JsonProperty("default_embedding_provider") public String defaultEmbeddingProvider;
        @JsonProperty("default_embedding_dimension") public Integer defaultEmbeddingDimension;
        // Default vision LLM (SPEC-041).
        @JsonProperty("default_vision_llm_model") public String defaultVisionLlmModel;
        @JsonProperty("default_vision_llm_provider") public String defaultVisionLlmProvider;

        public CreateTenantRequest() {}
        public CreateTenantRequest(String name, String slug) {
            this.name = name;
            this.slug = slug;
        }
    }

    /** WHY: Tenants list uses "items" key. */
    public static class TenantListResponse {
        @JsonProperty("items") public List<TenantInfo> items;
        @JsonProperty("total") public int total;
        @JsonProperty("page") public int page;
        @JsonProperty("page_size") public int pageSize;
        @JsonProperty("total_pages") public int totalPages;
    }

    // ── Conversations ────────────────────────────────────────────────

    public static class ConversationInfo {
        @JsonProperty("id") public String id;
        @JsonProperty("title") public String title;
        @JsonProperty("folder_id") public String folderId;
        @JsonProperty("message_count") public Integer messageCount;
        @JsonProperty("is_pinned") public boolean isPinned;
        @JsonProperty("created_at") public String createdAt;
        @JsonProperty("updated_at") public String updatedAt;
    }

    /** WHY: GET /api/v1/conversations returns {"items":[...]} wrapper, not raw array. */
    @JsonIgnoreProperties(ignoreUnknown = true)
    public static class ConversationListResponse {
        @JsonProperty("items") public List<ConversationInfo> items;
    }

    public static class CreateConversationRequest {
        @JsonProperty("title") public String title;
        @JsonProperty("mode") public String mode;
        @JsonProperty("folder_id") public String folderId;

        public CreateConversationRequest() {}
        public CreateConversationRequest(String title) { this.title = title; }
    }

    /** WHY: GET /conversations/{id} returns {"conversation":{...},"messages":[...]} wrapper. */
    public static class ConversationDetail {
        @JsonProperty("conversation") public ConversationInfo conversation;
        @JsonProperty("messages") public List<Message> messages;

        /** Convenience accessor for conversation id. */
        public String getId() {
            return conversation != null ? conversation.id : null;
        }
    }

    public static class Message {
        @JsonProperty("id") public String id;
        @JsonProperty("role") public String role;
        @JsonProperty("content") public String content;
        @JsonProperty("created_at") public String createdAt;
    }

    public static class CreateMessageRequest {
        @JsonProperty("role") public String role;
        @JsonProperty("content") public String content;

        public CreateMessageRequest() {}
        public CreateMessageRequest(String role, String content) {
            this.role = role;
            this.content = content;
        }
    }

    public static class ShareLink {
        @JsonProperty("share_id") public String shareId;
        @JsonProperty("url") public String url;
        @JsonProperty("created_at") public String createdAt;
        @JsonProperty("expires_at") public String expiresAt;
    }

    public static class BulkDeleteResponse {
        @JsonProperty("deleted_count") public int deletedCount;
    }

    // ── Folders ──────────────────────────────────────────────────────

    public static class FolderInfo {
        @JsonProperty("id") public String id;
        @JsonProperty("name") public String name;
        @JsonProperty("parent_id") public String parentId;
        @JsonProperty("conversation_count") public int conversationCount;
    }

    public static class CreateFolderRequest {
        @JsonProperty("name") public String name;
        @JsonProperty("parent_id") public String parentId;

        public CreateFolderRequest() {}
        public CreateFolderRequest(String name) { this.name = name; }
    }

    // ── Workspaces ───────────────────────────────────────────────────

    public static class WorkspaceInfo {
        @JsonProperty("id") public String id;
        @JsonProperty("name") public String name;
        @JsonProperty("slug") public String slug;
        @JsonProperty("description") public String description;
        @JsonProperty("tenant_id") public String tenantId;
        @JsonProperty("created_at") public String createdAt;
    }

    public static class CreateWorkspaceRequest {
        @JsonProperty("name") public String name;
        @JsonProperty("slug") public String slug;
        @JsonProperty("description") public String description;
    }

    public static class WorkspaceStats {
        @JsonProperty("workspace_id") public String workspaceId;
        @JsonProperty("document_count") public int documentCount;
        @JsonProperty("entity_count") public int entityCount;
        @JsonProperty("relationship_count") public int relationshipCount;
        @JsonProperty("chunk_count") public int chunkCount;
        @JsonProperty("query_count") public int queryCount;
        @JsonProperty("storage_size_bytes") public long storageSizeBytes;
    }

    public static class RebuildResponse {
        @JsonProperty("status") public String status;
        @JsonProperty("message") public String message;
        @JsonProperty("track_id") public String trackId;
    }

    // ── OODA-40: Additional models ───────────────────────────────────

    /** Message list response. */
    public static class MessageListResponse {
        @JsonProperty("messages") public List<Message> messages;
        @JsonProperty("total") public int total;
    }

    /** Workspace metrics history response. */
    public static class MetricsHistoryResponse {
        @JsonProperty("metrics") public List<WorkspaceMetric> metrics;
    }

    /** Workspace metric. */
    public static class WorkspaceMetric {
        @JsonProperty("date") public String date;
        @JsonProperty("documents") public int documents;
        @JsonProperty("entities") public int entities;
        @JsonProperty("queries") public int queries;
    }
}

```


#### Short summary: 

empty definition using pc, found symbol in pc: com/fasterxml/jackson/annotation/JsonProperty#