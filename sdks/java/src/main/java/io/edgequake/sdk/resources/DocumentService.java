package io.edgequake.sdk.resources;

import io.edgequake.sdk.internal.HttpHelper;
import io.edgequake.sdk.models.DocumentModels.*;

import java.util.LinkedHashMap;
import java.util.Map;

/** Document operations at /api/v1/documents. */
public class DocumentService {

    private final HttpHelper http;

    public DocumentService(HttpHelper http) { this.http = http; }

    public ListDocumentsResponse list(int page, int perPage) {
        Map<String, String> params = new LinkedHashMap<>();
        if (page > 0) params.put("page", String.valueOf(page));
        if (perPage > 0) params.put("per_page", String.valueOf(perPage));
        return http.get("/api/v1/documents", params, ListDocumentsResponse.class);
    }

    public Document get(String id) {
        return http.get("/api/v1/documents/" + id, null, Document.class);
    }

    /**
     * Upload text content as a document.
     * WHY: POST /api/v1/documents is the text upload handler.
     */
    public UploadResponse uploadText(String content, String title) {
        var body = new TextUploadRequest(content, title);
        return http.post("/api/v1/documents", body, UploadResponse.class);
    }

    public void delete(String id) {
        http.delete("/api/v1/documents/" + id);
    }

    public void deleteAll() {
        http.delete("/api/v1/documents");
    }

    public TrackStatus track(String trackId) {
        return http.get("/api/v1/documents/track/" + trackId, null, TrackStatus.class);
    }

    public ScanResponse scan(ScanRequest request) {
        return http.post("/api/v1/documents/scan", request, ScanResponse.class);
    }

    public DeletionImpact deletionImpact(String id) {
        return http.get("/api/v1/documents/" + id + "/deletion-impact", null, DeletionImpact.class);
    }

    // ── OODA-38: Added missing document methods ──────────────────────

    /** Get document chunks. */
    public DocumentChunksResponse chunks(String id) {
        return http.get("/api/v1/documents/" + id + "/chunks", null, DocumentChunksResponse.class);
    }

    /** Get document processing status. */
    public DocumentStatusResponse status(String id) {
        return http.get("/api/v1/documents/" + id + "/status", null, DocumentStatusResponse.class);
    }

    /** Reprocess a failed document. */
    public StatusResponse reprocess(String id) {
        return http.post("/api/v1/documents/" + id + "/reprocess", null, StatusResponse.class);
    }

    /** Recover stuck documents. */
    public StatusResponse recoverStuck() {
        return http.post("/api/v1/documents/recover-stuck", null, StatusResponse.class);
    }

    // ── OODA-40: Additional document methods ─────────────────────────

    /** Get document metadata. */
    public DocumentMetadataResponse getMetadata(String id) {
        return http.get("/api/v1/documents/" + id + "/metadata", null, DocumentMetadataResponse.class);
    }

    /** Update document metadata. */
    public DocumentMetadataResponse setMetadata(String id, Map<String, Object> metadata) {
        return http.patch("/api/v1/documents/" + id + "/metadata", Map.of("metadata", metadata), DocumentMetadataResponse.class);
    }

    /** Get failed chunks for a document. */
    public FailedChunksResponse failedChunks(String id) {
        return http.get("/api/v1/documents/" + id + "/failed-chunks", null, FailedChunksResponse.class);
    }

    /** Retry failed chunks for a document. */
    public StatusResponse retryChunks(String id) {
        return http.post("/api/v1/documents/" + id + "/retry-chunks", null, StatusResponse.class);
    }
}
