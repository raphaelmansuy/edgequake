using System.Text.Json;

namespace EdgeQuakeSDK;

// ============================================================================
// Lineage & provenance service — maps 7 lineage API endpoints.
//
// WHY: Provides entity lineage, document lineage, chunk detail, chunk lineage,
// entity provenance, full lineage, and lineage export.
// OODA-24: Iteration 24 — C# SDK lineage service.
//
// Endpoints covered:
//   GET /api/v1/lineage/entities/{name}        → EntityLineageAsync
//   GET /api/v1/lineage/documents/{id}         → DocumentLineageAsync
//   GET /api/v1/documents/{id}/lineage         → DocumentFullLineageAsync
//   GET /api/v1/documents/{id}/lineage/export  → ExportLineageAsync
//   GET /api/v1/chunks/{id}                    → ChunkDetailAsync
//   GET /api/v1/chunks/{id}/lineage            → ChunkLineageAsync
//   GET /api/v1/entities/{id}/provenance       → EntityProvenanceAsync
// ============================================================================

/// <summary>
/// Service for lineage, provenance, and chunk detail API endpoints.
/// WHY: Each method maps 1:1 to a lineage API resource for discoverability.
/// </summary>
public class LineageService(HttpHelper http)
{
    /// <summary>Get entity lineage showing all source documents.</summary>
    public Task<EntityLineageResponse> EntityLineageAsync(string entityName) =>
        http.GetAsync<EntityLineageResponse>(
            $"/api/v1/lineage/entities/{Uri.EscapeDataString(entityName)}");

    /// <summary>Get document graph lineage with entities and relationships.</summary>
    public Task<DocumentGraphLineageResponse> DocumentLineageAsync(string documentId) =>
        http.GetAsync<DocumentGraphLineageResponse>(
            $"/api/v1/lineage/documents/{Uri.EscapeDataString(documentId)}");

    /// <summary>Get full document lineage including metadata.</summary>
    public Task<DocumentFullLineageResponse> DocumentFullLineageAsync(string documentId) =>
        http.GetAsync<DocumentFullLineageResponse>(
            $"/api/v1/documents/{Uri.EscapeDataString(documentId)}/lineage");

    /// <summary>
    /// Export document lineage as JSON or CSV.
    /// WHY: Returns dynamic JSON — use GetRawAsync + JsonDocument since JsonElement is a struct.
    /// </summary>
    public async Task<JsonElement> ExportLineageAsync(string documentId, string format = "json")
    {
        var raw = await http.GetRawAsync(
            $"/api/v1/documents/{Uri.EscapeDataString(documentId)}/lineage/export?format={Uri.EscapeDataString(format)}");
        using var doc = JsonDocument.Parse(raw);
        return doc.RootElement.Clone();
    }

    /// <summary>Get detailed chunk information with extracted entities.</summary>
    public Task<ChunkDetailResponse> ChunkDetailAsync(string chunkId) =>
        http.GetAsync<ChunkDetailResponse>(
            $"/api/v1/chunks/{Uri.EscapeDataString(chunkId)}");

    /// <summary>Get chunk lineage with parent document references.</summary>
    public Task<ChunkLineageResponse> ChunkLineageAsync(string chunkId) =>
        http.GetAsync<ChunkLineageResponse>(
            $"/api/v1/chunks/{Uri.EscapeDataString(chunkId)}/lineage");

    /// <summary>Get entity provenance with source documents and related entities.</summary>
    public Task<EntityProvenanceResponse> EntityProvenanceAsync(string entityId) =>
        http.GetAsync<EntityProvenanceResponse>(
            $"/api/v1/entities/{Uri.EscapeDataString(entityId)}/provenance");
}
