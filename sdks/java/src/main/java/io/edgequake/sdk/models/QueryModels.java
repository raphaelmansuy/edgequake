package io.edgequake.sdk.models;

import com.fasterxml.jackson.annotation.JsonProperty;
import java.util.List;

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
    }

    // ── Chat ─────────────────────────────────────────────────────────

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
        @JsonProperty("messages") public List<ChatMessage> messages;
        @JsonProperty("model") public String model;
        @JsonProperty("temperature") public Double temperature;
        @JsonProperty("max_tokens") public Integer maxTokens;
        @JsonProperty("stream") public Boolean stream;

        public ChatCompletionRequest() {}
        public ChatCompletionRequest(List<ChatMessage> messages) {
            this.messages = messages;
        }
    }

    public static class ChatCompletionResponse {
        @JsonProperty("id") public String id;
        @JsonProperty("choices") public List<ChatChoice> choices;
        @JsonProperty("model") public String model;
        @JsonProperty("usage") public ChatUsage usage;
    }

    public static class ChatChoice {
        @JsonProperty("index") public int index;
        @JsonProperty("message") public ChatMessage message;
        @JsonProperty("finish_reason") public String finishReason;
    }

    public static class ChatUsage {
        @JsonProperty("prompt_tokens") public int promptTokens;
        @JsonProperty("completion_tokens") public int completionTokens;
        @JsonProperty("total_tokens") public int totalTokens;
    }
}
