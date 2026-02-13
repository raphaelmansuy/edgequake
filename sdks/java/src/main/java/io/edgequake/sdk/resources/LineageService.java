package io.edgequake.sdk.resources;

import io.edgequake.sdk.internal.HttpHelper;
import io.edgequake.sdk.models.LineageModels.*;

import java.net.URLEncoder;
import java.nio.charset.StandardCharsets;
import java.util.Map;

/**
 * Lineage operations: entity lineage, document lineage, chunk detail/lineage,
 * entity provenance, and lineage export.
 *
 * WHY: These endpoints provide full provenance tracking — tracing entities
 * back to source documents, chunks, and line ranges. Required by the
 * improve-lineage mission for all SDKs.
 *
 * @see edgequake/crates/edgequake-api/src/handlers/lineage.rs
 */
public class LineageService {

    private final HttpHelper http;

    public LineageService(HttpHelper http) { this.http = http; }

    // ── Entity Lineage ───────────────────────────────────────────────

    /**
     * Get entity lineage — which documents contributed to an entity.
     * GET /api/v1/lineage/entities/{name}
     */
    public EntityLineageResponse entityLineage(String entityName) {
        return http.get("/api/v1/lineage/entities/" + encode(entityName),
                null, EntityLineageResponse.class);
    }

    // ── Document Lineage ─────────────────────────────────────────────

    /**
     * Get document graph lineage — which entities were extracted from a document.
     * GET /api/v1/lineage/documents/{id}
     */
    public DocumentGraphLineageResponse documentLineage(String documentId) {
        return http.get("/api/v1/lineage/documents/" + encode(documentId),
                null, DocumentGraphLineageResponse.class);
    }

    /**
     * Get full document lineage with metadata.
     * GET /api/v1/documents/{id}/lineage
     */
    public DocumentFullLineageResponse documentFullLineage(String documentId) {
        return http.get("/api/v1/documents/" + encode(documentId) + "/lineage",
                null, DocumentFullLineageResponse.class);
    }

    /**
     * Export document lineage as raw data.
     * GET /api/v1/documents/{id}/lineage/export?format=json
     *
     * WHY: Returns the export payload. Use format="json" or "csv".
     */
    @SuppressWarnings("unchecked")
    public Map<String, Object> exportLineage(String documentId, String format) {
        String fmt = (format != null && !format.isEmpty()) ? format : "json";
        return http.get(
                "/api/v1/documents/" + encode(documentId) + "/lineage/export?format=" + fmt,
                null, Map.class);
    }

    // ── Chunk Detail & Lineage ───────────────────────────────────────

    /**
     * Get chunk detail.
     * GET /api/v1/chunks/{id}
     */
    public ChunkDetailResponse chunkDetail(String chunkId) {
        return http.get("/api/v1/chunks/" + encode(chunkId),
                null, ChunkDetailResponse.class);
    }

    /**
     * Get chunk lineage with parent document references.
     * GET /api/v1/chunks/{id}/lineage
     */
    public ChunkLineageResponse chunkLineage(String chunkId) {
        return http.get("/api/v1/chunks/" + encode(chunkId) + "/lineage",
                null, ChunkLineageResponse.class);
    }

    // ── Entity Provenance ────────────────────────────────────────────

    /**
     * Get entity provenance — source documents, chunks, and related entities.
     * GET /api/v1/entities/{id}/provenance
     */
    public EntityProvenanceResponse entityProvenance(String entityId) {
        return http.get("/api/v1/entities/" + encode(entityId) + "/provenance",
                null, EntityProvenanceResponse.class);
    }

    // ── Helpers ──────────────────────────────────────────────────────

    private static String encode(String value) {
        return URLEncoder.encode(value, StandardCharsets.UTF_8);
    }
}
