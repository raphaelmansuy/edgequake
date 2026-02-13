using System.Text.Json;

namespace EdgeQuakeSDK;

// ============================================================================
// Lineage, chunk detail, and entity provenance models.
//
// WHY: These 19 types match Rust lineage_types.rs exactly, providing rich
// entity lineage, document graph lineage, chunk details, and entity provenance.
// OODA-24: Iteration 24 — C# SDK lineage model expansion.
//
// @see edgequake/crates/edgequake-api/src/handlers/lineage_types.rs
// ============================================================================

// ── Entity Lineage ──────────────────────────────────────────────

/// <summary>Entity lineage response showing all source documents.</summary>
public class EntityLineageResponse
{
    public string? EntityName { get; set; }
    public string? EntityType { get; set; }
    public List<SourceDocumentInfo>? SourceDocuments { get; set; }
    public int? SourceCount { get; set; }
    public List<DescriptionVersionResponse>? DescriptionVersions { get; set; }
}

/// <summary>Information about a source document.</summary>
public class SourceDocumentInfo
{
    public string? DocumentId { get; set; }
    public List<string>? ChunkIds { get; set; }
    public List<LineRangeInfo>? LineRanges { get; set; }
}

/// <summary>Line range information for entity location.</summary>
public class LineRangeInfo
{
    public int? StartLine { get; set; }
    public int? EndLine { get; set; }
}

/// <summary>Description version for tracking entity description evolution.</summary>
public class DescriptionVersionResponse
{
    public int? Version { get; set; }
    public string? Description { get; set; }
    public string? SourceChunkId { get; set; }
    public string? CreatedAt { get; set; }
}

// ── Document Graph Lineage ──────────────────────────────────────

/// <summary>Graph lineage summary for a document.</summary>
public class DocumentGraphLineageResponse
{
    public string? DocumentId { get; set; }
    public int? ChunkCount { get; set; }
    public List<EntitySummaryResponse>? Entities { get; set; }
    public List<RelationshipSummaryResponse>? Relationships { get; set; }
    public ExtractionStatsResponse? ExtractionStats { get; set; }
}

/// <summary>Entity summary in lineage response.</summary>
public class EntitySummaryResponse
{
    public string? Name { get; set; }
    public string? EntityType { get; set; }
    public List<string>? SourceChunks { get; set; }
    public bool? IsShared { get; set; }
}

/// <summary>Relationship summary in lineage response.</summary>
public class RelationshipSummaryResponse
{
    public string? Source { get; set; }
    public string? Target { get; set; }
    public string? Keywords { get; set; }
    public List<string>? SourceChunks { get; set; }
}

/// <summary>Extraction statistics for a document.</summary>
public class ExtractionStatsResponse
{
    public int? TotalEntities { get; set; }
    public int? UniqueEntities { get; set; }
    public int? TotalRelationships { get; set; }
    public int? UniqueRelationships { get; set; }
    public long? ProcessingTimeMs { get; set; }
}

// ── Chunk Detail ────────────────────────────────────────────────

/// <summary>Chunk detail response with full content and extracted info.</summary>
public class ChunkDetailResponse
{
    public string? ChunkId { get; set; }
    public string? DocumentId { get; set; }
    public string? DocumentName { get; set; }
    public string? Content { get; set; }
    public int? Index { get; set; }
    public CharRange? CharRangeInfo { get; set; }
    public int? TokenCount { get; set; }
    public List<ExtractedEntityInfo>? Entities { get; set; }
    public List<ExtractedRelationshipInfo>? Relationships { get; set; }
    public ExtractionMetadataInfo? ExtractionMetadata { get; set; }
}

/// <summary>Character range for chunk position in document.</summary>
public class CharRange
{
    public int? Start { get; set; }
    public int? End { get; set; }
}

/// <summary>Entity extracted from a chunk.</summary>
public class ExtractedEntityInfo
{
    public string? Id { get; set; }
    public string? Name { get; set; }
    public string? EntityType { get; set; }
    public string? Description { get; set; }
}

/// <summary>Relationship extracted from a chunk.</summary>
public class ExtractedRelationshipInfo
{
    public string? SourceName { get; set; }
    public string? TargetName { get; set; }
    public string? RelationType { get; set; }
    public string? Description { get; set; }
}

/// <summary>Extraction metadata for LLM processing details.</summary>
public class ExtractionMetadataInfo
{
    public string? Model { get; set; }
    public int? GleaningIterations { get; set; }
    public long? DurationMs { get; set; }
    public int? InputTokens { get; set; }
    public int? OutputTokens { get; set; }
    public bool? Cached { get; set; }
}

// ── Entity Provenance ───────────────────────────────────────────

/// <summary>Entity provenance response with sources and related entities.</summary>
public class EntityProvenanceResponse
{
    public string? EntityId { get; set; }
    public string? EntityName { get; set; }
    public string? EntityType { get; set; }
    public string? Description { get; set; }
    public List<EntitySourceInfo>? Sources { get; set; }
    public int? TotalExtractionCount { get; set; }
    public List<RelatedEntityInfo>? RelatedEntities { get; set; }
}

/// <summary>Entity source information showing document provenance.</summary>
public class EntitySourceInfo
{
    public string? DocumentId { get; set; }
    public string? DocumentName { get; set; }
    public List<ChunkSourceInfo>? Chunks { get; set; }
    public string? FirstExtractedAt { get; set; }
}

/// <summary>Chunk source info for entity extraction location.</summary>
public class ChunkSourceInfo
{
    public string? ChunkId { get; set; }
    public int? StartLine { get; set; }
    public int? EndLine { get; set; }
    public string? SourceText { get; set; }
}

/// <summary>Related entity info for provenance connections.</summary>
public class RelatedEntityInfo
{
    public string? EntityId { get; set; }
    public string? EntityName { get; set; }
    public string? RelationshipType { get; set; }
    public int? SharedDocuments { get; set; }
}

// ── Document Full Lineage ───────────────────────────────────────

/// <summary>
/// Complete document lineage from GET /documents/{id}/lineage.
/// WHY: Returns persisted lineage + document metadata in a single call.
/// </summary>
public class DocumentFullLineageResponse
{
    public string? DocumentId { get; set; }
    public JsonElement? Metadata { get; set; }
    public JsonElement? Lineage { get; set; }
}

// ── Chunk Lineage ───────────────────────────────────────────────

/// <summary>
/// Chunk lineage from GET /chunks/{id}/lineage.
/// WHY: Lightweight chunk lineage with parent document refs and position info.
/// </summary>
public class ChunkLineageResponse
{
    public string? ChunkId { get; set; }
    public string? DocumentId { get; set; }
    public string? DocumentName { get; set; }
    public string? DocumentType { get; set; }
    public int? Index { get; set; }
    public int? StartLine { get; set; }
    public int? EndLine { get; set; }
    public int? StartOffset { get; set; }
    public int? EndOffset { get; set; }
    public int? TokenCount { get; set; }
    public string? ContentPreview { get; set; }
    public int? EntityCount { get; set; }
    public int? RelationshipCount { get; set; }
    public List<string>? EntityNames { get; set; }
    public JsonElement? DocumentMetadata { get; set; }
}
