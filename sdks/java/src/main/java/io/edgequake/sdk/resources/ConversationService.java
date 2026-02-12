package io.edgequake.sdk.resources;

import com.fasterxml.jackson.core.type.TypeReference;
import io.edgequake.sdk.internal.HttpHelper;
import io.edgequake.sdk.models.AuthModels.*;

import java.util.List;
import java.util.Map;

/**
 * Conversation operations at /api/v1/conversations.
 * WHY: Conversations require X-Tenant-ID and X-User-ID headers.
 */
public class ConversationService {

    private final HttpHelper http;

    public ConversationService(HttpHelper http) { this.http = http; }

    public ConversationInfo create(CreateConversationRequest request) {
        return http.post("/api/v1/conversations", request, ConversationInfo.class);
    }

    /** WHY: GET /api/v1/conversations returns {"items":[...]} wrapper, not raw array. */
    public List<ConversationInfo> list() {
        var wrapper = http.get("/api/v1/conversations", null, ConversationListResponse.class);
        return wrapper.items != null ? wrapper.items : List.of();
    }

    public ConversationDetail get(String id) {
        return http.get("/api/v1/conversations/" + id, null, ConversationDetail.class);
    }

    public void delete(String id) {
        http.delete("/api/v1/conversations/" + id);
    }

    public Message createMessage(String conversationId, CreateMessageRequest request) {
        return http.post("/api/v1/conversations/" + conversationId + "/messages",
                request, Message.class);
    }

    public ShareLink share(String id) {
        return http.post("/api/v1/conversations/" + id + "/share", null, ShareLink.class);
    }

    /**
     * WHY: Bulk delete uses /conversations/bulk/delete (not /bulk-delete).
     * Verified against routes.rs.
     */
    public BulkDeleteResponse bulkDelete(List<String> ids) {
        return http.post("/api/v1/conversations/bulk/delete",
                Map.of("ids", ids), BulkDeleteResponse.class);
    }

    /** WHY: Pin/unpin via PATCH /api/v1/conversations/{id} with is_pinned field. */
    public void pin(String id) {
        http.patch("/api/v1/conversations/" + id, Map.of("is_pinned", true));
    }

    public void unpin(String id) {
        http.patch("/api/v1/conversations/" + id, Map.of("is_pinned", false));
    }
}
