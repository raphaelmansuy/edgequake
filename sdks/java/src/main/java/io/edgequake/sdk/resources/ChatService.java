package io.edgequake.sdk.resources;

import io.edgequake.sdk.internal.HttpHelper;
import io.edgequake.sdk.models.QueryModels.*;

/** Chat operations at /api/v1/chat. */
public class ChatService {

    private final HttpHelper http;

    public ChatService(HttpHelper http) { this.http = http; }

    public ChatCompletionResponse completions(ChatCompletionRequest request) {
        return http.post("/api/v1/chat/completions", request, ChatCompletionResponse.class);
    }

    // ── OODA-38: Added streaming chat methods ────────────────────────

    /** Stream chat completions (SSE). */
    public String stream(ChatCompletionRequest request) {
        return http.postRaw("/api/v1/chat/completions/stream", request);
    }

    /** Completions with conversation ID. */
    public ChatCompletionResponse completionsWithConversation(String conversationId, String message) {
        var request = new ChatCompletionRequest();
        request.message = message;
        request.conversationId = conversationId;
        return http.post("/api/v1/chat/completions", request, ChatCompletionResponse.class);
    }
}
