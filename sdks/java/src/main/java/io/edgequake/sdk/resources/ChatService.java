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
}
