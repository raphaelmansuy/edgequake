import Foundation

// ============================================================================
// Lineage, chunk detail, and entity provenance models.
//
// WHY: These 19 types match Rust lineage_types.rs exactly, providing rich
// entity lineage, document graph lineage, chunk details, and entity provenance.
// OODA-26: Iteration 26 — Swift SDK lineage model expansion.
//
// @see edgequake/crates/edgequake-api/src/handlers/lineage_types.rs
// ============================================================================

// MARK: - Entity Lineage

/// Entity lineage response showing all source documents.
public struct EntityLineageResponse: Codable, Sendable {
    public let entityName: String?
    public let entityType: String?
    public let sourceDocuments: [SourceDocumentInfo]?
    public let sourceCount: Int?
    public let totalSourceDocuments: Int?
    public let totalChunks: Int?
    public let descriptionVersions: [DescriptionVersionResponse]?
}

/// Information about a source document.
public struct SourceDocumentInfo: Codable, Sendable {
    public let documentId: String?
    public let documentTitle: String?
    public let chunkIds: [String]?
    public let lineRanges: [LineRangeInfo]?
}

/// Line range information for entity location.
public struct LineRangeInfo: Codable, Sendable {
    public let startLine: Int?
    public let endLine: Int?
}

/// Description version for tracking entity description evolution.
public struct DescriptionVersionResponse: Codable, Sendable {
    public let version: Int?
    public let description: String?
    public let sourceChunkId: String?
    public let createdAt: String?
}

// MARK: - Document Graph Lineage

/// Graph lineage summary for a document.
public struct DocumentGraphLineageResponse: Codable, Sendable {
    public let documentId: String?
    public let documentTitle: String?
    public let chunkCount: Int?
    public let entities: [EntitySummaryResponse]?
    public let relationships: [RelationshipSummaryResponse]?
    public let extractionStats: ExtractionStatsResponse?
}

/// Entity summary in lineage response.
public struct EntitySummaryResponse: Codable, Sendable {
    public let name: String?
    public let entityName: String?
    public let entityType: String?
    public let mentions: Int?
    public let sourceChunks: [String]?
    public let isShared: Bool?
}

/// Relationship summary in lineage response.
public struct RelationshipSummaryResponse: Codable, Sendable {
    public let source: String?
    public let target: String?
    public let type: String?
    public let keywords: String?
    public let mentions: Int?
    public let sourceChunks: [String]?
}

/// Extraction statistics for a document.
public struct ExtractionStatsResponse: Codable, Sendable {
    public let totalEntities: Int?
    public let uniqueEntities: Int?
    public let totalRelationships: Int?
    public let uniqueRelationships: Int?
    public let processingTimeMs: Int?
}

// MARK: - Chunk Detail

/// Chunk detail response with full content and extracted info.
public struct ChunkDetailResponse: Codable, Sendable {
    public let chunkId: String?
    public let documentId: String?
    public let documentName: String?
    public let content: String?
    public let index: Int?
    public let chunkIndex: Int?
    public let charRange: CharRange?
    public let tokenCount: Int?
    public let entities: [ExtractedEntityInfo]?
    public let relationships: [ExtractedRelationshipInfo]?
    public let extractionMetadata: ExtractionMetadataInfo?
}

/// Character range for chunk position in document.
public struct CharRange: Codable, Sendable {
    public let start: Int?
    public let end: Int?
}

/// Entity extracted from a chunk.
public struct ExtractedEntityInfo: Codable, Sendable {
    public let id: String?
    public let name: String?
    public let entityName: String?
    public let entityType: String?
    public let description: String?
    public let confidence: Double?
}

/// Relationship extracted from a chunk.
public struct ExtractedRelationshipInfo: Codable, Sendable {
    public let source: String?
    public let sourceName: String?
    public let target: String?
    public let targetName: String?
    public let type: String?
    public let relationType: String?
    public let description: String?
    public let weight: Double?
}

/// Extraction metadata for LLM processing details.
public struct ExtractionMetadataInfo: Codable, Sendable {
    public let model: String?
    public let gleaningIterations: Int?
    public let durationMs: Int?
    public let inputTokens: Int?
    public let outputTokens: Int?
    public let cached: Bool?
}

// MARK: - Entity Provenance

/// Entity provenance response with sources and related entities.
public struct EntityProvenanceResponse: Codable, Sendable {
    public let entityId: String?
    public let entityName: String?
    public let entityType: String?
    public let description: String?
    public let sources: [EntitySourceInfo]?
    public let totalExtractionCount: Int?
    public let totalSources: Int?
    public let relatedEntities: [RelatedEntityInfo]?
}

/// Entity source information showing document provenance.
public struct EntitySourceInfo: Codable, Sendable {
    public let documentId: String?
    public let documentTitle: String?
    public let documentName: String?
    public let chunkId: String?
    public let chunks: [ChunkSourceInfo]?
    public let confidence: Double?
    public let firstExtractedAt: String?
}

/// Chunk source info for entity extraction location.
public struct ChunkSourceInfo: Codable, Sendable {
    public let chunkId: String?
    public let startLine: Int?
    public let endLine: Int?
    public let sourceText: String?
}

/// Related entity info for provenance connections.
public struct RelatedEntityInfo: Codable, Sendable {
    public let entityId: String?
    public let entityName: String?
    public let relationshipType: String?
    public let sharedDocuments: Int?
}

// MARK: - Document Full Lineage

/// Complete document lineage from GET /documents/{id}/lineage.
/// WHY: Returns persisted lineage + document metadata in a single call.
public struct DocumentFullLineageResponse: Codable, Sendable {
    public let documentId: String?
    public let documentTitle: String?
    public let status: String?
    public let chunkCount: Int?
    public let entityCount: Int?
    public let relationshipCount: Int?
    public let metadata: AnyCodable?
    public let lineage: AnyCodable?
}

// MARK: - Chunk Lineage

/// Chunk lineage from GET /chunks/{id}/lineage.
/// WHY: Lightweight chunk lineage with parent document refs and position info.
public struct ChunkLineageResponse: Codable, Sendable {
    public let chunkId: String?
    public let documentId: String?
    public let documentTitle: String?
    public let documentName: String?
    public let documentType: String?
    public let index: Int?
    public let chunkIndex: Int?
    public let totalChunksInDocument: Int?
    public let startLine: Int?
    public let endLine: Int?
    public let startOffset: Int?
    public let endOffset: Int?
    public let tokenCount: Int?
    public let contentPreview: String?
    public let entityCount: Int?
    public let relationshipCount: Int?
    public let entityNames: [String]?
    public let documentMetadata: AnyCodable?
}
