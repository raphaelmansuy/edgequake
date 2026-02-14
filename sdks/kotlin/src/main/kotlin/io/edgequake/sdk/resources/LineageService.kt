package io.edgequake.sdk.resources

import io.edgequake.sdk.internal.HttpHelper
import io.edgequake.sdk.models.*
import java.net.URLEncoder
import java.nio.charset.StandardCharsets

/**
 * Lineage service — entity and document lineage tracking.
 *
 * WHY: Covers all 7 lineage endpoints from the API surface:
 *   GET /api/v1/lineage/entities/{name}
 *   GET /api/v1/lineage/documents/{id}
 *   GET /api/v1/documents/{id}/lineage
 *   GET /api/v1/documents/{id}/lineage/export?format=
 *   GET /api/v1/chunks/{id}
 *   GET /api/v1/chunks/{id}/lineage
 *   GET /api/v1/entities/{id}/provenance
 *
 * @see edgequake/crates/edgequake-api/src/handlers/lineage.rs
 */
class LineageService(private val http: HttpHelper) {

    /** Get entity lineage — which documents contributed to an entity. */
    fun entityLineage(entityName: String): EntityLineageResponse {
        // WHY: URLEncoder.encode(String, Charset) was added in Java 10, but we use
        // the String overload for broader compatibility with Java 8+.
        val encoded = URLEncoder.encode(entityName, "UTF-8")
        return http.get("/api/v1/lineage/entities/$encoded")
    }

    /** Get document graph lineage — entities and relationships from a document. */
    fun documentLineage(documentId: String): DocumentGraphLineageResponse =
        http.get("/api/v1/lineage/documents/$documentId")

    /** Get full document lineage with metadata. */
    fun documentFullLineage(documentId: String): DocumentFullLineageResponse =
        http.get("/api/v1/documents/$documentId/lineage")

    /** Export lineage data in specified format (json or csv). */
    fun exportLineage(documentId: String, format: String = "json"): Map<String, Any?> =
        http.get("/api/v1/documents/$documentId/lineage/export?format=$format")

    /** Get chunk detail with extracted entities and relationships. */
    fun chunkDetail(chunkId: String): ChunkDetailResponse =
        http.get("/api/v1/chunks/$chunkId")

    /** Get chunk lineage with parent document references. */
    fun chunkLineage(chunkId: String): ChunkLineageResponse =
        http.get("/api/v1/chunks/$chunkId/lineage")

    /** Get entity provenance — full source traceability. */
    fun entityProvenance(entityId: String): EntityProvenanceResponse =
        http.get("/api/v1/entities/$entityId/provenance")
}
