import Foundation

// ============================================================================
// Lineage & provenance service — maps 7 lineage API endpoints.
//
// WHY: Provides entity lineage, document lineage, chunk detail, chunk lineage,
// entity provenance, full lineage, and lineage export.
// OODA-26: Iteration 26 — Swift SDK lineage service.
//
// Endpoints covered:
//   GET /api/v1/lineage/entities/{name}        → entityLineage
//   GET /api/v1/lineage/documents/{id}         → documentLineage
//   GET /api/v1/documents/{id}/lineage         → documentFullLineage
//   GET /api/v1/documents/{id}/lineage/export  → exportLineage
//   GET /api/v1/chunks/{id}                    → chunkDetail
//   GET /api/v1/chunks/{id}/lineage            → chunkLineage
//   GET /api/v1/entities/{id}/provenance       → entityProvenance
// ============================================================================

/// Service for lineage, provenance, and chunk detail API endpoints.
/// WHY: Each method maps 1:1 to a lineage API resource for discoverability.
public final class LineageService: @unchecked Sendable {
    private let http: HttpHelper
    init(_ http: HttpHelper) { self.http = http }

    /// Get entity lineage showing all source documents.
    public func entityLineage(name: String) async throws -> EntityLineageResponse {
        let encoded = name.addingPercentEncoding(withAllowedCharacters: .urlPathAllowed) ?? name
        return try await http.get("/api/v1/lineage/entities/\(encoded)")
    }

    /// Get document graph lineage with entities and relationships.
    public func documentLineage(id: String) async throws -> DocumentGraphLineageResponse {
        let encoded = id.addingPercentEncoding(withAllowedCharacters: .urlPathAllowed) ?? id
        return try await http.get("/api/v1/lineage/documents/\(encoded)")
    }

    /// Get full document lineage including metadata.
    public func documentFullLineage(id: String) async throws -> DocumentFullLineageResponse {
        let encoded = id.addingPercentEncoding(withAllowedCharacters: .urlPathAllowed) ?? id
        return try await http.get("/api/v1/documents/\(encoded)/lineage")
    }

    /// Export document lineage as JSON or CSV.
    /// WHY: Returns raw Data since format may be CSV or arbitrary JSON.
    public func exportLineage(id: String, format: String = "json") async throws -> Data {
        let encoded = id.addingPercentEncoding(withAllowedCharacters: .urlPathAllowed) ?? id
        let fmtEncoded =
            format.addingPercentEncoding(withAllowedCharacters: .urlQueryAllowed) ?? format
        return try await http.getRaw(
            "/api/v1/documents/\(encoded)/lineage/export?format=\(fmtEncoded)")
    }

    /// Get detailed chunk information with extracted entities.
    public func chunkDetail(id: String) async throws -> ChunkDetailResponse {
        let encoded = id.addingPercentEncoding(withAllowedCharacters: .urlPathAllowed) ?? id
        return try await http.get("/api/v1/chunks/\(encoded)")
    }

    /// Get chunk lineage with parent document references.
    public func chunkLineage(id: String) async throws -> ChunkLineageResponse {
        let encoded = id.addingPercentEncoding(withAllowedCharacters: .urlPathAllowed) ?? id
        return try await http.get("/api/v1/chunks/\(encoded)/lineage")
    }

    /// Get entity provenance with source documents and related entities.
    public func entityProvenance(id: String) async throws -> EntityProvenanceResponse {
        let encoded = id.addingPercentEncoding(withAllowedCharacters: .urlPathAllowed) ?? id
        return try await http.get("/api/v1/entities/\(encoded)/provenance")
    }
}
