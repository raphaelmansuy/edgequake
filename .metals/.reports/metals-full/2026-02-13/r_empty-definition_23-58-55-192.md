error id: file://<WORKSPACE>/sdks/java/src/main/java/io/edgequake/sdk/models/DocumentModels.java:java/util/Map#
file://<WORKSPACE>/sdks/java/src/main/java/io/edgequake/sdk/models/DocumentModels.java
empty definition using pc, found symbol in pc: java/util/Map#
empty definition using semanticdb
empty definition using fallback
non-local guesses:

offset: 128
uri: file://<WORKSPACE>/sdks/java/src/main/java/io/edgequake/sdk/models/DocumentModels.java
text:
```scala
package io.edgequake.sdk.models;

import com.fasterxml.jackson.annotation.JsonProperty;
import java.util.List;
import java.util.@@Map;

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
}

```


#### Short summary: 

empty definition using pc, found symbol in pc: java/util/Map#