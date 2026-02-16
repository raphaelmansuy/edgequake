package io.edgequake.sdk.models;

import java.util.List;

import com.fasterxml.jackson.annotation.JsonProperty;

/** Query and Chat model classes. */
public class QueryModels {

    public static class QueryRequest {
        @JsonProperty("query") public String query;
        @JsonProperty("mode") public String mode;
        @JsonProperty("top_k") public Integer topK;
        @JsonProperty("stream") public Boolean stream;
        @JsonProperty("only_need_context") public Boolean onlyNeedContext;

        public QueryRequest() {}
        public QueryRequest(String query, String mode) {
            this.query = query;
            this.mode = mode;
        }
    }

    public static class QueryResponse {
        @JsonProperty("answer") public String answer;
        @JsonProperty("sources") public List<SourceReference> sources;
        @JsonProperty("mode") public String mode;
    }

    public static class SourceReference {
        @JsonProperty("document_id") public String documentId;
        @JsonProperty("chunk_id") public String chunkId;
        @JsonProperty("content") public String content;
        @JsonProperty("score") public Double score;
        @JsonProperty("file_path") public String filePath;
    }

    // ── Chat ─────────────────────────────────────────────────────────
    // WHY: EdgeQuake uses `message` (singular string), NOT `messages` (array).
    // This is EdgeQuake's native RAG-aware chat format.

    public static class ChatMessage {
        @JsonProperty("role") public String role;
        @JsonProperty("content") public String content;

        public ChatMessage() {}
        public ChatMessage(String role, String content) {
            this.role = role;
            this.content = content;
        }
    }

    public static class ChatCompletionRequest {
        @JsonProperty("message") public String message;
        @JsonProperty("stream") public Boolean stream;
        @JsonProperty("mode") public String mode;
        @JsonProperty("conversation_id") public String conversationId;
        @JsonProperty("max_tokens") public Integer maxTokens;
        @JsonProperty("temperature") public Double temperature;
        @JsonProperty("top_k") public Integer topK;
        @JsonProperty("parent_id") public String parentId;
        @JsonProperty("provider") public String provider;
        @JsonProperty("model") public String model;

        public ChatCompletionRequest() {}
        public ChatCompletionRequest(String message) {
            this.message = message;
            this.stream = false;
        }
    }

    public static class ChatSourceReference {
        @JsonProperty("source_type") public String sourceType;
        @JsonProperty("id") public String id;
        @JsonProperty("score") public Double score;
        @JsonProperty("snippet") public String snippet;
        @JsonProperty("document_id") public String documentId;
        @JsonProperty("file_path") public String filePath;
    }

    public static class ChatCompletionResponse {
        @JsonProperty("conversation_id") public String conversationId;
        @JsonProperty("user_message_id") public String userMessageId;
        @JsonProperty("assistant_message_id") public String assistantMessageId;
        @JsonProperty("content") public String content;
        @JsonProperty("mode") public String mode;
        @JsonProperty("sources") public List<ChatSourceReference> sources;
    }
}
