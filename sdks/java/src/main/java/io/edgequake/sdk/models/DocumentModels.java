package io.edgequake.sdk.models;

import com.fasterxml.jackson.annotation.JsonProperty;
import java.util.List;
import java.util.Map;

/** Document-related model classes. */
public class DocumentModels {

    public static class Document {
        @JsonProperty("id") public String id;
        @JsonProperty("file_name") public String fileName;
        @JsonProperty("title") public String title;
        @JsonProperty("status") public String status;
        @JsonProperty("file_size") public Long fileSize;
        @JsonProperty("mime_type") public String mimeType;
        @JsonProperty("entity_count") public Integer entityCount;
        @JsonProperty("chunk_count") public Integer chunkCount;
        @JsonProperty("created_at") public String createdAt;
        @JsonProperty("updated_at") public String updatedAt;
    }

    public static class UploadResponse {
        /** WHY: Real API returns "document_id", not "id". */
        @JsonProperty("document_id") public String documentId;
        @JsonProperty("status") public String status;
        @JsonProperty("track_id") public String trackId;
        @JsonProperty("message") public String message;
        @JsonProperty("chunk_count") public Integer chunkCount;
        @JsonProperty("entity_count") public Integer entityCount;
        @JsonProperty("relationship_count") public Integer relationshipCount;
    }

    public static class ListDocumentsResponse {
        @JsonProperty("documents") public List<Document> documents;
        @JsonProperty("pagination") public PaginationInfo pagination;
    }

    public static class PaginationInfo {
        @JsonProperty("page") public int page;
        @JsonProperty("per_page") public int perPage;
        @JsonProperty("total") public int total;
        @JsonProperty("total_pages") public int totalPages;
    }

    public static class TrackStatus {
        @JsonProperty("track_id") public String trackId;
        @JsonProperty("status") public String status;
        @JsonProperty("progress") public Double progress;
        @JsonProperty("message") public String message;
        @JsonProperty("document_id") public String documentId;
    }

    public static class ScanRequest {
        @JsonProperty("path") public String path;
        @JsonProperty("recursive") public Boolean recursive;
        @JsonProperty("extensions") public List<String> extensions;
    }

    public static class ScanResponse {
        @JsonProperty("files_found") public int filesFound;
        @JsonProperty("files_queued") public int filesQueued;
        @JsonProperty("files_skipped") public int filesSkipped;
    }

    public static class DeletionImpact {
        @JsonProperty("entity_count") public int entityCount;
        @JsonProperty("relationship_count") public int relationshipCount;
        @JsonProperty("chunk_count") public int chunkCount;
    }

    /** Request body for uploading text content. */
    public static class TextUploadRequest {
        @JsonProperty("content") public String content;
        @JsonProperty("title") public String title;

        public TextUploadRequest() {}
        public TextUploadRequest(String content, String title) {
            this.content = content;
            this.title = title;
        }
    }

    // ── OODA-38: Added missing document models ───────────────────────

    /** Document chunks response. */
    public static class DocumentChunksResponse {
        @JsonProperty("document_id") public String documentId;
        @JsonProperty("chunks") public List<ChunkInfo> chunks;
        @JsonProperty("total") public int total;
    }

    /** Chunk information. */
    public static class ChunkInfo {
        @JsonProperty("id") public String id;
        @JsonProperty("content") public String content;
        @JsonProperty("index") public Integer index;
        @JsonProperty("start_line") public Integer startLine;
        @JsonProperty("end_line") public Integer endLine;
    }

    /** Document status response. */
    public static class DocumentStatusResponse {
        @JsonProperty("document_id") public String documentId;
        @JsonProperty("status") public String status;
        @JsonProperty("progress") public Double progress;
        @JsonProperty("error") public String error;
    }

    /** Generic status response. */
    public static class StatusResponse {
        @JsonProperty("status") public String status;
        @JsonProperty("message") public String message;
    }

    // ── OODA-40: Additional document models ─────────────────────────

    /** Document metadata response. */
    public static class DocumentMetadataResponse {
        @JsonProperty("document_id") public String documentId;
        @JsonProperty("metadata") public Map<String, Object> metadata;
    }

    /** Failed chunks response. */
    public static class FailedChunksResponse {
        @JsonProperty("document_id") public String documentId;
        @JsonProperty("chunks") public List<FailedChunkInfo> chunks;
        @JsonProperty("total") public int total;
    }

    /** Failed chunk info. */
    public static class FailedChunkInfo {
        @JsonProperty("id") public String id;
        @JsonProperty("index") public Integer index;
        @JsonProperty("error") public String error;
        @JsonProperty("retries") public Integer retries;
    }
}
