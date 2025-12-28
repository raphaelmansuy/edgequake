# Data Models: SOTA Ingestion Pipeline

> Document ID: DATA-001
> Version: 1.0
> Created: 2024-12-28

## Table of Contents

1. [Overview](#1-overview)
2. [Core Domain Models](#2-core-domain-models)
3. [Pipeline Models](#3-pipeline-models)
4. [Lineage Models](#4-lineage-models)
5. [Cost Tracking Models](#5-cost-tracking-models)
6. [Progress & Status Models](#6-progress--status-models)
7. [Storage Schema](#7-storage-schema)
8. [API Response Models](#8-api-response-models)

---

## 1. Overview

This document defines all data models required for the SOTA GenAI ingestion pipeline. Models are organized by domain and include Rust struct definitions, database schemas, and API contracts.

### 1.1 Model Naming Conventions

- **DM-XXX**: Domain Model (core business entities)
- **PM-XXX**: Pipeline Model (processing artifacts)
- **LM-XXX**: Lineage Model (tracking and traceability)
- **CM-XXX**: Cost Model (financial tracking)
- **SM-XXX**: Status Model (progress and monitoring)

---

## 2. Core Domain Models

### DM-001: Document

Represents an ingested document.

```rust
/// DM-001: Core document representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    /// Unique identifier
    pub id: String,
    
    /// Original filename
    pub filename: String,
    
    /// Document content (raw text)
    pub content: String,
    
    /// Content hash for deduplication
    pub content_hash: String,
    
    /// MIME type (e.g., "text/plain", "application/pdf")
    pub mime_type: String,
    
    /// File size in bytes
    pub size_bytes: usize,
    
    /// Character count
    pub char_count: usize,
    
    /// Estimated token count
    pub token_count: usize,
    
    /// Tenant context
    pub tenant_id: String,
    
    /// Workspace context
    pub workspace_id: String,
    
    /// Processing status
    pub status: DocumentStatus,
    
    /// Custom metadata
    pub metadata: HashMap<String, serde_json::Value>,
    
    /// Timestamps
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    
    /// Soft delete marker
    pub deleted_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum DocumentStatus {
    Pending,
    Processing,
    Completed,
    Failed,
    Deleted,
}
```

### DM-002: TextChunk (Enhanced)

Represents a chunk of text with full lineage information.

```rust
/// DM-002: Enhanced text chunk with line tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextChunk {
    /// Unique identifier (format: "{doc_id}-chunk-{index}")
    pub id: String,
    
    /// Chunk content
    pub content: String,
    
    /// Zero-based index within document
    pub index: usize,
    
    // === Character Position (existing) ===
    /// Character offset from document start
    pub start_offset: usize,
    /// Character offset to chunk end
    pub end_offset: usize,
    
    // === Line Position (NEW - R001) ===
    /// Starting line number (1-based)
    pub start_line: usize,
    /// Ending line number (1-based, inclusive)
    pub end_line: usize,
    
    // === Token Information ===
    /// Estimated token count
    pub token_count: usize,
    
    // === Source Reference ===
    /// Parent document ID
    pub document_id: String,
    /// Document filename for citations
    pub document_name: Option<String>,
    
    // === Processing Metadata ===
    /// Chunking strategy used
    pub chunking_strategy: String,
    /// Overlap with previous chunk (in tokens)
    pub overlap_tokens: usize,
    
    // === Embedding ===
    pub embedding: Option<Vec<f32>>,
    
    // === Lineage (NEW) ===
    /// IDs of entities extracted from this chunk
    pub entity_ids: Vec<String>,
    /// IDs of relationships extracted from this chunk
    pub relationship_ids: Vec<String>,
    /// LLM cache entries used for extraction
    pub llm_cache_ids: Vec<String>,
    
    // === Timestamps ===
    pub created_at: DateTime<Utc>,
}
```

### DM-003: Entity (Enhanced)

Represents a knowledge graph entity with full provenance.

```rust
/// DM-003: Enhanced entity with provenance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    /// Normalized entity key (UPPERCASE_WITH_UNDERSCORES)
    pub id: String,
    
    /// Display name (original casing)
    pub name: String,
    
    /// Entity type (PERSON, ORGANIZATION, etc.)
    pub entity_type: String,
    
    /// Merged description from all sources
    pub description: String,
    
    /// Importance score (0.0 to 1.0)
    pub importance: f32,
    
    /// Entity embedding
    pub embedding: Option<Vec<f32>>,
    
    // === Provenance (NEW - R001) ===
    /// Source document IDs
    pub source_document_ids: Vec<String>,
    /// Source chunk IDs
    pub source_chunk_ids: Vec<String>,
    /// Original text spans where entity was found
    pub source_spans: Vec<SourceSpan>,
    
    // === Multi-tenancy ===
    pub tenant_id: String,
    pub workspace_id: String,
    
    // === Statistics ===
    /// Number of times this entity was extracted
    pub extraction_count: usize,
    /// Number of relationships involving this entity
    pub relationship_count: usize,
    
    // === Timestamps ===
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}

/// Source span for entity provenance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceSpan {
    /// Chunk ID where entity was found
    pub chunk_id: String,
    /// Start line in original document
    pub start_line: usize,
    /// End line in original document
    pub end_line: usize,
    /// Original text excerpt
    pub text: String,
}
```

### DM-004: Relationship (Enhanced)

Represents a knowledge graph relationship with full provenance.

```rust
/// DM-004: Enhanced relationship with provenance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relationship {
    /// Unique relationship ID
    pub id: String,
    
    /// Source entity ID (normalized)
    pub source_id: String,
    
    /// Target entity ID (normalized)
    pub target_id: String,
    
    /// Relationship type/label
    pub relation_type: String,
    
    /// Relationship description
    pub description: String,
    
    /// Keywords associated with relationship
    pub keywords: Vec<String>,
    
    /// Relationship strength/weight (0.0 to 1.0)
    pub weight: f32,
    
    /// Relationship embedding
    pub embedding: Option<Vec<f32>>,
    
    // === Provenance (NEW - R001) ===
    /// Source document IDs
    pub source_document_ids: Vec<String>,
    /// Source chunk IDs
    pub source_chunk_ids: Vec<String>,
    /// Original text spans where relationship was found
    pub source_spans: Vec<SourceSpan>,
    
    // === Multi-tenancy ===
    pub tenant_id: String,
    pub workspace_id: String,
    
    // === Statistics ===
    /// Number of times this relationship was extracted
    pub extraction_count: usize,
    
    // === Timestamps ===
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}
```

---

## 3. Pipeline Models

### PM-001: IngestionJob

Represents a complete ingestion job.

```rust
/// PM-001: Ingestion job definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestionJob {
    /// Job ID (also used as track_id)
    pub id: String,
    
    /// Document being processed
    pub document_id: String,
    
    /// Job configuration
    pub config: IngestionConfig,
    
    /// Current status
    pub status: JobStatus,
    
    /// Processing result (if completed)
    pub result: Option<IngestionResult>,
    
    /// Error details (if failed)
    pub error: Option<IngestionError>,
    
    /// Cost tracking
    pub cost: IngestionCost,
    
    /// Progress tracking
    pub progress: IngestionProgress,
    
    // === Context ===
    pub tenant_id: String,
    pub workspace_id: String,
    pub user_id: Option<String>,
    
    // === Timestamps ===
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum JobStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}
```

### PM-002: IngestionConfig

Configuration for ingestion processing.

```rust
/// PM-002: Ingestion configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestionConfig {
    // === Chunking ===
    /// Target chunk size in tokens
    pub chunk_size: usize,
    /// Overlap between chunks in tokens
    pub chunk_overlap: usize,
    /// Minimum chunk size
    pub min_chunk_size: usize,
    /// Chunking strategy name
    pub chunking_strategy: ChunkingStrategy,
    
    // === Extraction ===
    /// LLM model for extraction
    pub extraction_model: String,
    /// Entity types to extract
    pub entity_types: Vec<String>,
    /// Maximum gleaning iterations
    pub max_gleaning: usize,
    /// Maximum concurrent extraction tasks
    pub max_concurrent_extractions: usize,
    
    // === Embedding ===
    /// Embedding model
    pub embedding_model: String,
    /// Embedding dimension
    pub embedding_dim: usize,
    /// Batch size for embedding generation
    pub embedding_batch_size: usize,
    
    // === Summarization ===
    /// Enable MapReduce summarization
    pub enable_mapreduce_summary: bool,
    /// Token threshold for LLM summarization
    pub summary_context_size: usize,
    /// Force LLM summary when descriptions exceed this count
    pub force_llm_summary_on_merge: usize,
    /// Target summary length
    pub summary_length: usize,
    
    // === Feature Flags ===
    pub enable_entity_extraction: bool,
    pub enable_relationship_extraction: bool,
    pub enable_chunk_embeddings: bool,
    pub enable_entity_embeddings: bool,
    pub enable_relationship_embeddings: bool,
    pub enable_caching: bool,
    
    // === Language ===
    pub extraction_language: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChunkingStrategy {
    TokenBased,
    CharacterBased { separator: String },
    Semantic,
    Custom { name: String },
}

impl Default for IngestionConfig {
    fn default() -> Self {
        Self {
            chunk_size: 1200,
            chunk_overlap: 100,
            min_chunk_size: 100,
            chunking_strategy: ChunkingStrategy::TokenBased,
            extraction_model: "gpt-4o-mini".to_string(),
            entity_types: vec![
                "PERSON".to_string(),
                "ORGANIZATION".to_string(),
                "LOCATION".to_string(),
                "CONCEPT".to_string(),
                "EVENT".to_string(),
            ],
            max_gleaning: 1,
            max_concurrent_extractions: 4,
            embedding_model: "text-embedding-3-small".to_string(),
            embedding_dim: 1536,
            embedding_batch_size: 100,
            enable_mapreduce_summary: true,
            summary_context_size: 4000,
            force_llm_summary_on_merge: 6,
            summary_length: 500,
            enable_entity_extraction: true,
            enable_relationship_extraction: true,
            enable_chunk_embeddings: true,
            enable_entity_embeddings: true,
            enable_relationship_embeddings: true,
            enable_caching: true,
            extraction_language: "English".to_string(),
        }
    }
}
```

### PM-003: IngestionResult

Result of a completed ingestion job.

```rust
/// PM-003: Ingestion result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestionResult {
    /// Job ID
    pub job_id: String,
    
    /// Document ID
    pub document_id: String,
    
    // === Chunk Statistics ===
    pub chunk_count: usize,
    pub total_chunk_tokens: usize,
    pub avg_chunk_size: usize,
    
    // === Entity Statistics ===
    pub entity_count: usize,
    pub entities_created: usize,
    pub entities_updated: usize,
    pub unique_entity_types: Vec<String>,
    
    // === Relationship Statistics ===
    pub relationship_count: usize,
    pub relationships_created: usize,
    pub relationships_updated: usize,
    pub unique_relationship_types: Vec<String>,
    
    // === Keyword Statistics ===
    pub keywords: Vec<String>,
    pub keyword_count: usize,
    
    // === Processing Info ===
    pub processing_time_ms: u64,
    pub llm_calls: usize,
    pub embedding_calls: usize,
    
    // === Model Info ===
    pub extraction_model: String,
    pub embedding_model: String,
    pub chunking_strategy: String,
    
    // === Lineage ===
    pub chunk_ids: Vec<String>,
    pub entity_ids: Vec<String>,
    pub relationship_ids: Vec<String>,
}
```

### PM-004: ExtractionResult (Enhanced)

Result from entity/relationship extraction.

```rust
/// PM-004: Enhanced extraction result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractionResult {
    /// Source chunk ID
    pub chunk_id: String,
    
    /// Extracted entities
    pub entities: Vec<ExtractedEntity>,
    
    /// Extracted relationships
    pub relationships: Vec<ExtractedRelationship>,
    
    // === Extraction Metadata ===
    pub extraction_model: String,
    pub gleaning_iterations: usize,
    pub extraction_time_ms: u64,
    
    // === Token Usage ===
    pub input_tokens: usize,
    pub output_tokens: usize,
    
    // === Cache Info ===
    pub cache_hit: bool,
    pub cache_id: Option<String>,
    
    /// Additional metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Extracted entity (pre-normalization)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedEntity {
    /// Entity name (as extracted)
    pub name: String,
    /// Normalized name (UPPERCASE_WITH_UNDERSCORES)
    pub normalized_name: String,
    /// Entity type
    pub entity_type: String,
    /// Description from extraction
    pub description: String,
    /// Importance score
    pub importance: f32,
    /// Source text spans
    pub source_spans: Vec<String>,
    /// Embedding (if generated)
    pub embedding: Option<Vec<f32>>,
}

/// Extracted relationship (pre-normalization)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedRelationship {
    /// Source entity name
    pub source: String,
    /// Normalized source name
    pub normalized_source: String,
    /// Target entity name
    pub target: String,
    /// Normalized target name
    pub normalized_target: String,
    /// Relationship type
    pub relation_type: String,
    /// Description
    pub description: String,
    /// Keywords
    pub keywords: Vec<String>,
    /// Weight
    pub weight: f32,
    /// Embedding (if generated)
    pub embedding: Option<Vec<f32>>,
}
```

---

## 4. Lineage Models

### LM-001: DocumentLineage

Complete lineage tracking for a document.

```rust
/// LM-001: Document lineage tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentLineage {
    /// Document ID
    pub document_id: String,
    
    /// Document filename
    pub document_name: String,
    
    /// Ingestion job that created this lineage
    pub job_id: String,
    
    /// Ingestion configuration used
    pub config: IngestionConfig,
    
    /// All chunks from this document
    pub chunks: Vec<ChunkLineage>,
    
    /// All entities extracted from this document
    pub entities: Vec<EntityLineage>,
    
    /// All relationships extracted from this document
    pub relationships: Vec<RelationshipLineage>,
    
    // === Statistics ===
    pub total_chunks: usize,
    pub total_entities: usize,
    pub total_relationships: usize,
    
    // === Timestamps ===
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
```

### LM-002: ChunkLineage

Lineage information for a single chunk.

```rust
/// LM-002: Chunk lineage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkLineage {
    /// Chunk ID
    pub chunk_id: String,
    
    /// Chunk index in document
    pub chunk_index: usize,
    
    // === Position ===
    pub start_line: usize,
    pub end_line: usize,
    pub start_offset: usize,
    pub end_offset: usize,
    
    // === Entities extracted from this chunk ===
    pub entity_ids: Vec<String>,
    
    // === Relationships extracted from this chunk ===
    pub relationship_ids: Vec<String>,
    
    // === Extraction info ===
    pub extraction_metadata: ExtractionMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractionMetadata {
    pub llm_model: String,
    pub gleaning_iterations: usize,
    pub extraction_time_ms: u64,
    pub input_tokens: usize,
    pub output_tokens: usize,
    pub cache_hit: bool,
    pub cache_id: Option<String>,
}
```

### LM-003: EntityLineage

Lineage information for an entity across all sources.

```rust
/// LM-003: Entity lineage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityLineage {
    /// Entity ID
    pub entity_id: String,
    
    /// Entity name
    pub entity_name: String,
    
    /// All documents where this entity was found
    pub sources: Vec<EntitySource>,
    
    /// Total extraction count
    pub extraction_count: usize,
    
    /// Description history (for audit)
    pub description_history: Vec<DescriptionVersion>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntitySource {
    pub document_id: String,
    pub document_name: String,
    pub chunk_ids: Vec<String>,
    pub source_spans: Vec<SourceSpan>,
    pub extracted_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DescriptionVersion {
    pub description: String,
    pub source: String, // "extraction" | "merge" | "summary"
    pub created_at: DateTime<Utc>,
}
```

---

## 5. Cost Tracking Models

### CM-001: IngestionCost

Complete cost tracking for an ingestion job.

```rust
/// CM-001: Ingestion cost tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestionCost {
    /// Job ID
    pub job_id: String,
    
    /// Document ID
    pub document_id: String,
    
    /// Total cost in USD
    pub total_cost_usd: f64,
    
    /// Cost breakdown by operation
    pub breakdown: CostBreakdown,
    
    /// Token usage summary
    pub token_usage: TokenUsageSummary,
    
    /// Timestamps
    pub calculated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostBreakdown {
    // === Extraction ===
    pub extraction: OperationCost,
    
    // === Gleaning ===
    pub gleaning: OperationCost,
    
    // === Summarization ===
    pub summarization: OperationCost,
    
    // === Embedding ===
    pub embedding: OperationCost,
    
    // === Total ===
    pub total_usd: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationCost {
    /// Number of API calls
    pub api_calls: usize,
    
    /// Input tokens
    pub input_tokens: usize,
    
    /// Output tokens
    pub output_tokens: usize,
    
    /// Cost in USD
    pub cost_usd: f64,
    
    /// Model used
    pub model: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsageSummary {
    pub total_input_tokens: usize,
    pub total_output_tokens: usize,
    pub total_embedding_tokens: usize,
    pub total_tokens: usize,
}
```

### CM-002: CostConfig

Cost configuration for different models.

```rust
/// CM-002: Model cost configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostConfig {
    /// Cost per 1K input tokens
    pub input_cost_per_1k: f64,
    
    /// Cost per 1K output tokens
    pub output_cost_per_1k: f64,
    
    /// Cost per 1K embedding tokens
    pub embedding_cost_per_1k: f64,
}

impl CostConfig {
    /// GPT-4o-mini pricing (as of Dec 2024)
    pub fn gpt_4o_mini() -> Self {
        Self {
            input_cost_per_1k: 0.00015,  // $0.15 per 1M
            output_cost_per_1k: 0.0006,   // $0.60 per 1M
            embedding_cost_per_1k: 0.0,
        }
    }
    
    /// text-embedding-3-small pricing
    pub fn text_embedding_3_small() -> Self {
        Self {
            input_cost_per_1k: 0.0,
            output_cost_per_1k: 0.0,
            embedding_cost_per_1k: 0.00002, // $0.02 per 1M
        }
    }
}
```

---

## 6. Progress & Status Models

### SM-001: IngestionProgress

Real-time progress tracking.

```rust
/// SM-001: Ingestion progress
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestionProgress {
    /// Job ID
    pub job_id: String,
    
    /// Document ID
    pub document_id: String,
    
    /// Overall status
    pub status: IngestionStatus,
    
    /// Current stage
    pub current_stage: PipelineStage,
    
    /// Stage progress details
    pub stages: Vec<StageProgress>,
    
    /// Overall completion percentage (0-100)
    pub completion_percentage: f32,
    
    /// Estimated time remaining (seconds)
    pub eta_seconds: Option<u64>,
    
    /// Latest status message
    pub latest_message: String,
    
    /// Message history
    pub history_messages: Vec<ProgressMessage>,
    
    /// Error details (if any)
    pub errors: Vec<IngestionError>,
    
    // === Timestamps ===
    pub started_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum IngestionStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum PipelineStage {
    Preprocessing,
    Chunking,
    Extracting,
    Gleaning,
    Merging,
    Summarizing,
    Embedding,
    Storing,
    Finalizing,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageProgress {
    pub stage: PipelineStage,
    pub status: StageStatus,
    pub total_items: usize,
    pub completed_items: usize,
    pub completion_percentage: f32,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum StageStatus {
    Pending,
    Running,
    Completed,
    Skipped,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressMessage {
    pub message: String,
    pub level: MessageLevel,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum MessageLevel {
    Debug,
    Info,
    Warning,
    Error,
}
```

### SM-002: IngestionError

Error tracking for ingestion.

```rust
/// SM-002: Ingestion error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestionError {
    /// Error code (e.g., "E001", "E002")
    pub code: String,
    
    /// Human-readable error message
    pub message: String,
    
    /// Error details (for debugging)
    pub details: Option<String>,
    
    /// Stage where error occurred
    pub stage: PipelineStage,
    
    /// Item that caused error (chunk_id, entity_name, etc.)
    pub item_id: Option<String>,
    
    /// Whether error is recoverable
    pub recoverable: bool,
    
    /// Retry count
    pub retry_count: usize,
    
    /// Timestamp
    pub occurred_at: DateTime<Utc>,
}
```

---

## 7. Storage Schema

### 7.1 PostgreSQL Tables

```sql
-- Documents table
CREATE TABLE documents (
    id VARCHAR(255) PRIMARY KEY,
    filename VARCHAR(1024) NOT NULL,
    content TEXT NOT NULL,
    content_hash VARCHAR(64) NOT NULL,
    mime_type VARCHAR(255) NOT NULL,
    size_bytes BIGINT NOT NULL,
    char_count INTEGER NOT NULL,
    token_count INTEGER NOT NULL,
    tenant_id VARCHAR(255) NOT NULL,
    workspace_id VARCHAR(255) NOT NULL,
    status VARCHAR(50) NOT NULL DEFAULT 'pending',
    metadata JSONB DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    deleted_at TIMESTAMPTZ,
    
    INDEX idx_documents_tenant_workspace (tenant_id, workspace_id),
    INDEX idx_documents_status (status),
    INDEX idx_documents_content_hash (content_hash)
);

-- Chunks table
CREATE TABLE chunks (
    id VARCHAR(255) PRIMARY KEY,
    document_id VARCHAR(255) NOT NULL REFERENCES documents(id),
    content TEXT NOT NULL,
    chunk_index INTEGER NOT NULL,
    start_offset INTEGER NOT NULL,
    end_offset INTEGER NOT NULL,
    start_line INTEGER NOT NULL,
    end_line INTEGER NOT NULL,
    token_count INTEGER NOT NULL,
    chunking_strategy VARCHAR(100) NOT NULL,
    overlap_tokens INTEGER NOT NULL DEFAULT 0,
    embedding vector(1536),
    entity_ids TEXT[] DEFAULT '{}',
    relationship_ids TEXT[] DEFAULT '{}',
    llm_cache_ids TEXT[] DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    
    INDEX idx_chunks_document (document_id),
    INDEX idx_chunks_embedding USING ivfflat (embedding vector_cosine_ops)
);

-- Document lineage table
CREATE TABLE document_lineage (
    id VARCHAR(255) PRIMARY KEY,
    document_id VARCHAR(255) NOT NULL REFERENCES documents(id),
    job_id VARCHAR(255) NOT NULL,
    config JSONB NOT NULL,
    total_chunks INTEGER NOT NULL DEFAULT 0,
    total_entities INTEGER NOT NULL DEFAULT 0,
    total_relationships INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    
    INDEX idx_lineage_document (document_id),
    INDEX idx_lineage_job (job_id)
);

-- Ingestion jobs table
CREATE TABLE ingestion_jobs (
    id VARCHAR(255) PRIMARY KEY,
    document_id VARCHAR(255) NOT NULL REFERENCES documents(id),
    tenant_id VARCHAR(255) NOT NULL,
    workspace_id VARCHAR(255) NOT NULL,
    user_id VARCHAR(255),
    config JSONB NOT NULL,
    status VARCHAR(50) NOT NULL DEFAULT 'pending',
    result JSONB,
    error JSONB,
    cost JSONB,
    progress JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    started_at TIMESTAMPTZ,
    completed_at TIMESTAMPTZ,
    
    INDEX idx_jobs_tenant_workspace (tenant_id, workspace_id),
    INDEX idx_jobs_status (status),
    INDEX idx_jobs_document (document_id)
);

-- Ingestion costs table
CREATE TABLE ingestion_costs (
    id VARCHAR(255) PRIMARY KEY,
    job_id VARCHAR(255) NOT NULL REFERENCES ingestion_jobs(id),
    document_id VARCHAR(255) NOT NULL REFERENCES documents(id),
    total_cost_usd DECIMAL(10, 6) NOT NULL DEFAULT 0,
    breakdown JSONB NOT NULL,
    token_usage JSONB NOT NULL,
    calculated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    
    INDEX idx_costs_job (job_id),
    INDEX idx_costs_document (document_id)
);

-- LLM cache table
CREATE TABLE llm_cache (
    id VARCHAR(255) PRIMARY KEY,
    cache_type VARCHAR(50) NOT NULL, -- 'extract', 'summary', 'glean'
    chunk_id VARCHAR(255),
    prompt_hash VARCHAR(64) NOT NULL,
    response TEXT NOT NULL,
    input_tokens INTEGER NOT NULL,
    output_tokens INTEGER NOT NULL,
    model VARCHAR(100) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    
    INDEX idx_cache_type (cache_type),
    INDEX idx_cache_chunk (chunk_id),
    INDEX idx_cache_prompt_hash (prompt_hash)
);
```

---

## 8. API Response Models

### 8.1 Document Upload Response

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UploadDocumentResponse {
    pub document_id: String,
    pub track_id: String,
    pub status: String,
    pub message: String,
    pub created_at: DateTime<Utc>,
}
```

### 8.2 Track Status Response

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackStatusResponse {
    pub track_id: String,
    pub document_id: String,
    pub status: IngestionStatus,
    pub progress: IngestionProgress,
    pub result: Option<IngestionResult>,
    pub cost: Option<IngestionCost>,
    pub error: Option<IngestionError>,
}
```

### 8.3 Document Lineage Response

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentLineageResponse {
    pub document_id: String,
    pub document_name: String,
    pub lineage: DocumentLineage,
    pub chunks_summary: Vec<ChunkSummary>,
    pub entities_summary: Vec<EntitySummary>,
    pub relationships_summary: Vec<RelationshipSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkSummary {
    pub chunk_id: String,
    pub chunk_index: usize,
    pub start_line: usize,
    pub end_line: usize,
    pub entity_count: usize,
    pub relationship_count: usize,
}
```

---

## Appendix: Entity-Relationship Diagram

```
┌─────────────────────────────────────────────────────────────────────────┐
│                        DATA MODEL E-R DIAGRAM                           │
└─────────────────────────────────────────────────────────────────────────┘

┌──────────────┐     1:N     ┌──────────────┐     N:M     ┌──────────────┐
│   Document   │────────────▶│    Chunk     │────────────▶│    Entity    │
│              │             │              │             │              │
│ - id         │             │ - id         │             │ - id         │
│ - filename   │             │ - content    │             │ - name       │
│ - content    │             │ - start_line │             │ - type       │
│ - status     │             │ - end_line   │             │ - description│
│              │             │ - embedding  │             │ - embedding  │
└──────────────┘             └──────────────┘             └──────────────┘
       │                            │                            │
       │ 1:1                        │ N:M                        │ N:M
       ▼                            ▼                            ▼
┌──────────────┐             ┌──────────────┐             ┌──────────────┐
│ DocumentLine │             │ Relationship │◀────────────│ Relationship │
│   age        │             │              │             │    (self)    │
│              │             │ - source_id  │             │              │
│ - job_id     │             │ - target_id  │             │              │
│ - config     │             │ - type       │             │              │
│ - chunks[]   │             │ - keywords   │             │              │
│ - entities[] │             │ - embedding  │             │              │
└──────────────┘             └──────────────┘             └──────────────┘
       │
       │ 1:1
       ▼
┌──────────────┐
│IngestionJob  │
│              │
│ - id         │
│ - config     │
│ - status     │
│ - cost       │
│ - progress   │
└──────────────┘
       │
       │ 1:1
       ▼
┌──────────────┐
│IngestionCost│
│              │
│ - breakdown  │
│ - tokens     │
│ - cost_usd   │
└──────────────┘
```

---
