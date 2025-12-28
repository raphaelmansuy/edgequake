# Implementation Plan: SOTA Ingestion Pipeline

> Document ID: IMPL-001
> Version: 1.0
> Created: 2024-12-28

## Table of Contents

1. [Executive Summary](#1-executive-summary)
2. [Implementation Phases](#2-implementation-phases)
3. [Phase 1: Core Enhancements](#3-phase-1-core-enhancements)
4. [Phase 2: MapReduce & Caching](#4-phase-2-mapreduce--caching)
5. [Phase 3: Progress & Cost Tracking](#5-phase-3-progress--cost-tracking)
6. [Phase 4: Lineage & Document Management](#6-phase-4-lineage--document-management)
7. [Phase 5: API & Integration](#7-phase-5-api--integration)
8. [Code Changes Reference](#8-code-changes-reference)
9. [Migration Strategy](#9-migration-strategy)
10. [Risk Assessment](#10-risk-assessment)

---

## 1. Executive Summary

This implementation plan outlines the step-by-step approach to enhance EdgeQuake's ingestion pipeline to SOTA (State-of-the-Art) standards. The implementation is divided into 5 phases spanning approximately 4-6 weeks.

### 1.1 Goals

| Goal | Priority | Phase |
|------|----------|-------|
| Line number tracking in chunks | P0 | Phase 1 |
| Parallel chunk processing | P0 | Phase 1 |
| MapReduce description summarization | P0 | Phase 2 |
| Comprehensive LLM caching | P0 | Phase 2 |
| Real-time progress tracking | P0 | Phase 3 |
| Cost tracking per operation | P0 | Phase 3 |
| Full lineage tracking | P1 | Phase 4 |
| Document suppression | P1 | Phase 4 |
| Enhanced API endpoints | P1 | Phase 5 |
| WebSocket progress events | P2 | Phase 5 |

### 1.2 Timeline

```
Week 1-2: Phase 1 - Core Enhancements
Week 2-3: Phase 2 - MapReduce & Caching
Week 3-4: Phase 3 - Progress & Cost Tracking
Week 4-5: Phase 4 - Lineage & Document Management
Week 5-6: Phase 5 - API & Integration
```

---

## 2. Implementation Phases

```
┌─────────────────────────────────────────────────────────────────────────┐
│                      IMPLEMENTATION PHASES                              │
└─────────────────────────────────────────────────────────────────────────┘

Phase 1: Core Enhancements
══════════════════════════
  ┌─────────────────┐
  │ Line Number     │
  │ Tracking        │────┐
  └─────────────────┘    │
  ┌─────────────────┐    │
  │ Parallel        │────┼───▶ Phase 1 Complete
  │ Processing      │    │
  └─────────────────┘    │
  ┌─────────────────┐    │
  │ Token Usage     │────┘
  │ Enhancement     │
  └─────────────────┘

Phase 2: MapReduce & Caching
════════════════════════════
  ┌─────────────────┐
  │ MapReduce       │
  │ Summarization   │────┐
  └─────────────────┘    │
  ┌─────────────────┐    │
  │ LLM Response    │────┼───▶ Phase 2 Complete
  │ Caching         │    │
  └─────────────────┘    │
  ┌─────────────────┐    │
  │ Rebuild from    │────┘
  │ Cache           │
  └─────────────────┘

Phase 3: Progress & Cost
════════════════════════
  ┌─────────────────┐
  │ Progress        │
  │ Tracking        │────┐
  └─────────────────┘    │
  ┌─────────────────┐    │
  │ Cost Tracking   │────┼───▶ Phase 3 Complete
  │ Per Operation   │    │
  └─────────────────┘    │
  ┌─────────────────┐    │
  │ Event           │────┘
  │ Streaming       │
  └─────────────────┘

Phase 4: Lineage & Docs
═══════════════════════
  ┌─────────────────┐
  │ Full Lineage    │
  │ Storage         │────┐
  └─────────────────┘    │
  ┌─────────────────┐    │
  │ Document        │────┼───▶ Phase 4 Complete
  │ Suppression     │    │
  └─────────────────┘    │
  ┌─────────────────┐    │
  │ Entity CRUD     │────┘
  │ Cascade         │
  └─────────────────┘

Phase 5: API & Integration
══════════════════════════
  ┌─────────────────┐
  │ Enhanced API    │
  │ Endpoints       │────┐
  └─────────────────┘    │
  ┌─────────────────┐    │
  │ WebSocket       │────┼───▶ Phase 5 Complete
  │ Events          │    │
  └─────────────────┘    │
  ┌─────────────────┐    │
  │ Integration     │────┘
  │ Tests           │
  └─────────────────┘
```

---

## 3. Phase 1: Core Enhancements

### 3.1 Task List

| Task ID | Task | File(s) | Effort | Dependencies |
|---------|------|---------|--------|--------------|
| P1-01 | Add line number tracking to TextChunk | chunker.rs | 2h | None |
| P1-02 | Implement line number calculation | chunker.rs | 3h | P1-01 |
| P1-03 | Add parallel chunk processing | pipeline.rs | 4h | None |
| P1-04 | Enhance token usage tracking | extractor.rs | 2h | None |
| P1-05 | Add processing metadata to extraction | extractor.rs | 2h | P1-04 |
| P1-06 | Update tests for new fields | tests/*.rs | 3h | P1-01..05 |

### 3.2 Detailed Implementation

#### P1-01: Add Line Number Tracking to TextChunk

**File:** `edgequake/crates/edgequake-pipeline/src/chunker.rs`

```rust
// ADD to TextChunk struct
pub struct TextChunk {
    pub id: String,
    pub content: String,
    pub index: usize,
    // Existing
    pub start_offset: usize,
    pub end_offset: usize,
    // NEW: Line number tracking
    pub start_line: usize,      // 1-based line number
    pub end_line: usize,        // 1-based, inclusive
    pub token_count: usize,
    pub embedding: Option<Vec<f32>>,
}
```

#### P1-02: Implement Line Number Calculation

**File:** `edgequake/crates/edgequake-pipeline/src/chunker.rs`

```rust
// ADD helper function
fn calculate_line_numbers(full_text: &str, start_offset: usize, end_offset: usize) -> (usize, usize) {
    let before_chunk = &full_text[..start_offset];
    let chunk_text = &full_text[start_offset..end_offset];
    
    // Count newlines before start
    let start_line = before_chunk.chars().filter(|&c| c == '\n').count() + 1;
    
    // Count newlines in chunk
    let lines_in_chunk = chunk_text.chars().filter(|&c| c == '\n').count();
    let end_line = start_line + lines_in_chunk;
    
    (start_line, end_line)
}

// MODIFY chunk_sync to calculate line numbers
fn chunk_sync(&self, text: &str, doc_id: &str) -> Result<Vec<TextChunk>> {
    // ... existing chunking logic ...
    
    Ok(chunks
        .into_iter()
        .enumerate()
        .map(|(index, (content, start, end))| {
            let (start_line, end_line) = calculate_line_numbers(text, start, end);
            let id = format!("{}-chunk-{}", doc_id, index);
            TextChunk {
                id,
                content,
                index,
                start_offset: start,
                end_offset: end,
                start_line,
                end_line,
                token_count: estimate_tokens(&content),
                embedding: None,
            }
        })
        .collect())
}
```

#### P1-03: Implement Parallel Chunk Processing

**File:** `edgequake/crates/edgequake-pipeline/src/pipeline.rs`

```rust
use futures::stream::{self, StreamExt};

impl Pipeline {
    /// Process chunks in parallel with semaphore control
    async fn extract_parallel(
        &self,
        chunks: &[TextChunk],
        extractor: &Arc<dyn EntityExtractor>,
    ) -> Result<Vec<ExtractionResult>> {
        let semaphore = Arc::new(tokio::sync::Semaphore::new(
            self.config.max_concurrent_extractions
        ));
        
        let futures: Vec<_> = chunks.iter().map(|chunk| {
            let semaphore = semaphore.clone();
            let extractor = extractor.clone();
            let chunk = chunk.clone();
            
            async move {
                let _permit = semaphore.acquire().await
                    .map_err(|e| PipelineError::ExtractionError(e.to_string()))?;
                extractor.extract(&chunk).await
            }
        }).collect();
        
        let results: Vec<Result<ExtractionResult>> = stream::iter(futures)
            .buffer_unordered(self.config.max_concurrent_extractions)
            .collect()
            .await;
        
        results.into_iter().collect()
    }
    
    /// Updated process method using parallel extraction
    pub async fn process(&self, document_id: &str, content: &str) -> Result<ProcessingResult> {
        let start = std::time::Instant::now();
        let mut stats = ProcessingStats::default();

        // Step 1: Chunk the document
        let mut chunks = self.chunker.chunk(content, document_id)?;
        stats.chunk_count = chunks.len();

        // Step 2: Extract in parallel
        let mut extractions = Vec::new();
        if self.config.enable_entity_extraction || self.config.enable_relationship_extraction {
            if let Some(extractor) = &self.extractor {
                extractions = self.extract_parallel(&chunks, extractor).await?;
                
                // Aggregate stats
                for extraction in &extractions {
                    stats.entity_count += extraction.entities.len();
                    stats.relationship_count += extraction.relationships.len();
                    stats.llm_calls += 1;
                }
            }
        }
        
        // ... rest of processing ...
    }
}
```

#### P1-04: Enhance Token Usage Tracking

**File:** `edgequake/crates/edgequake-pipeline/src/extractor.rs`

```rust
// ADD to ExtractionResult
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExtractionResult {
    pub entities: Vec<ExtractedEntity>,
    pub relationships: Vec<ExtractedRelationship>,
    pub source_chunk_id: String,
    pub metadata: HashMap<String, serde_json::Value>,
    // NEW: Token usage tracking
    pub input_tokens: usize,
    pub output_tokens: usize,
    pub extraction_time_ms: u64,
}

// MODIFY LLMExtractor.extract to track tokens
#[async_trait]
impl<L> EntityExtractor for LLMExtractor<L>
where
    L: edgequake_llm::LLMProvider + Send + Sync + ?Sized,
{
    async fn extract(&self, chunk: &TextChunk) -> Result<ExtractionResult> {
        let start = std::time::Instant::now();
        let prompt = self.build_prompt(&chunk.content);

        let response = self
            .llm_provider
            .complete(&prompt)
            .await
            .map_err(|e| PipelineError::ExtractionError(format!("LLM error: {}", e)))?;

        let mut result = self.parse_response(&response.content, &chunk.id)?;
        
        // NEW: Track token usage
        result.input_tokens = response.input_tokens.unwrap_or(0);
        result.output_tokens = response.output_tokens.unwrap_or(0);
        result.extraction_time_ms = start.elapsed().as_millis() as u64;
        
        Ok(result)
    }
}
```

### 3.3 Acceptance Criteria

- [ ] TextChunk includes start_line and end_line fields
- [ ] Line numbers are correctly calculated for all chunks
- [ ] Parallel processing works with configurable concurrency
- [ ] Token usage is tracked per extraction
- [ ] All existing tests pass
- [ ] New tests for line number tracking pass

---

## 4. Phase 2: MapReduce & Caching

### 4.1 Task List

| Task ID | Task | File(s) | Effort | Dependencies |
|---------|------|---------|--------|--------------|
| P2-01 | Create MapReduce summarizer | summarizer.rs | 6h | None |
| P2-02 | Add LLM response caching trait | cache.rs (new) | 4h | None |
| P2-03 | Implement in-memory cache | cache.rs | 3h | P2-02 |
| P2-04 | Implement PostgreSQL cache | cache.rs | 4h | P2-02 |
| P2-05 | Integrate caching into extractor | extractor.rs | 3h | P2-03 |
| P2-06 | Implement rebuild from cache | pipeline.rs | 4h | P2-05 |
| P2-07 | Integrate MapReduce into merger | merger.rs | 3h | P2-01 |
| P2-08 | Add tests for caching | tests/*.rs | 4h | P2-01..07 |

### 4.2 Detailed Implementation

#### P2-01: Create MapReduce Summarizer

**File:** `edgequake/crates/edgequake-pipeline/src/summarizer.rs`

```rust
use async_trait::async_trait;
use edgequake_llm::LLMProvider;
use std::sync::Arc;

/// Configuration for MapReduce summarization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SummarizerConfig {
    /// Maximum context size in tokens
    pub context_size: usize,
    /// Target summary length in tokens
    pub summary_length: usize,
    /// Minimum descriptions before forcing LLM summary
    pub force_llm_summary_on_merge: usize,
    /// Separator between descriptions
    pub separator: String,
}

impl Default for SummarizerConfig {
    fn default() -> Self {
        Self {
            context_size: 4000,
            summary_length: 500,
            force_llm_summary_on_merge: 6,
            separator: "\n\n".to_string(),
        }
    }
}

/// MapReduce description summarizer
pub struct MapReduceSummarizer<L: LLMProvider> {
    llm_provider: Arc<L>,
    config: SummarizerConfig,
}

impl<L: LLMProvider + Send + Sync> MapReduceSummarizer<L> {
    pub fn new(llm_provider: Arc<L>, config: SummarizerConfig) -> Self {
        Self { llm_provider, config }
    }
    
    /// Summarize descriptions using map-reduce approach
    pub async fn summarize(&self, descriptions: Vec<String>) -> Result<(String, bool)> {
        // Base case: single description
        if descriptions.len() == 1 {
            return Ok((descriptions[0].clone(), false));
        }
        
        let total_tokens: usize = descriptions.iter()
            .map(|d| estimate_tokens(d))
            .sum();
        
        // If within limits, just join
        if total_tokens <= self.config.context_size 
            && descriptions.len() < self.config.force_llm_summary_on_merge 
        {
            return Ok((descriptions.join(&self.config.separator), false));
        }
        
        // MAP phase: split into chunks and summarize each
        let chunks = self.split_into_chunks(&descriptions);
        let mut summaries = Vec::new();
        
        for chunk in chunks {
            if chunk.len() == 1 {
                summaries.push(chunk[0].clone());
            } else {
                let summary = self.llm_summarize(&chunk).await?;
                summaries.push(summary);
            }
        }
        
        // REDUCE phase: recursively summarize summaries
        if summaries.len() > 1 {
            Box::pin(self.summarize(summaries)).await
        } else {
            Ok((summaries.into_iter().next().unwrap_or_default(), true))
        }
    }
    
    fn split_into_chunks(&self, descriptions: &[String]) -> Vec<Vec<String>> {
        let mut chunks = Vec::new();
        let mut current_chunk = Vec::new();
        let mut current_tokens = 0;
        
        for desc in descriptions {
            let desc_tokens = estimate_tokens(desc);
            
            if current_tokens + desc_tokens > self.config.context_size && !current_chunk.is_empty() {
                chunks.push(current_chunk);
                current_chunk = Vec::new();
                current_tokens = 0;
            }
            
            current_chunk.push(desc.clone());
            current_tokens += desc_tokens;
        }
        
        if !current_chunk.is_empty() {
            chunks.push(current_chunk);
        }
        
        chunks
    }
    
    async fn llm_summarize(&self, descriptions: &[String]) -> Result<String> {
        let prompt = format!(
            r#"Summarize the following descriptions into a single, comprehensive description.
Keep all important facts and details. Maximum length: {} tokens.

Descriptions:
{}

Summary:"#,
            self.config.summary_length,
            descriptions.join("\n---\n")
        );
        
        let response = self.llm_provider.complete(&prompt).await
            .map_err(|e| PipelineError::SummarizationError(e.to_string()))?;
        
        Ok(response.content.trim().to_string())
    }
}
```

#### P2-02: Add LLM Response Caching

**File:** `edgequake/crates/edgequake-pipeline/src/cache.rs` (NEW)

```rust
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Cache entry for LLM responses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry {
    pub id: String,
    pub cache_type: CacheType,
    pub chunk_id: Option<String>,
    pub prompt_hash: String,
    pub response: String,
    pub input_tokens: usize,
    pub output_tokens: usize,
    pub model: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum CacheType {
    Extract,
    Glean,
    Summary,
}

/// Trait for LLM response caching
#[async_trait]
pub trait LLMCache: Send + Sync {
    /// Get cached response by prompt hash
    async fn get(&self, prompt_hash: &str) -> Result<Option<CacheEntry>>;
    
    /// Store response in cache
    async fn set(&self, entry: CacheEntry) -> Result<()>;
    
    /// Get all cache entries for a chunk
    async fn get_by_chunk(&self, chunk_id: &str) -> Result<Vec<CacheEntry>>;
    
    /// Delete cache entries by chunk ID
    async fn delete_by_chunk(&self, chunk_id: &str) -> Result<usize>;
    
    /// Clear all cache entries
    async fn clear(&self) -> Result<()>;
}

/// In-memory cache implementation
pub struct MemoryLLMCache {
    entries: tokio::sync::RwLock<HashMap<String, CacheEntry>>,
    chunk_index: tokio::sync::RwLock<HashMap<String, Vec<String>>>,
}

impl MemoryLLMCache {
    pub fn new() -> Self {
        Self {
            entries: tokio::sync::RwLock::new(HashMap::new()),
            chunk_index: tokio::sync::RwLock::new(HashMap::new()),
        }
    }
}

#[async_trait]
impl LLMCache for MemoryLLMCache {
    async fn get(&self, prompt_hash: &str) -> Result<Option<CacheEntry>> {
        let entries = self.entries.read().await;
        Ok(entries.get(prompt_hash).cloned())
    }
    
    async fn set(&self, entry: CacheEntry) -> Result<()> {
        let mut entries = self.entries.write().await;
        let mut chunk_index = self.chunk_index.write().await;
        
        if let Some(chunk_id) = &entry.chunk_id {
            chunk_index
                .entry(chunk_id.clone())
                .or_default()
                .push(entry.prompt_hash.clone());
        }
        
        entries.insert(entry.prompt_hash.clone(), entry);
        Ok(())
    }
    
    async fn get_by_chunk(&self, chunk_id: &str) -> Result<Vec<CacheEntry>> {
        let entries = self.entries.read().await;
        let chunk_index = self.chunk_index.read().await;
        
        let hashes = chunk_index.get(chunk_id);
        let mut results = Vec::new();
        
        if let Some(hashes) = hashes {
            for hash in hashes {
                if let Some(entry) = entries.get(hash) {
                    results.push(entry.clone());
                }
            }
        }
        
        Ok(results)
    }
    
    async fn delete_by_chunk(&self, chunk_id: &str) -> Result<usize> {
        let mut entries = self.entries.write().await;
        let mut chunk_index = self.chunk_index.write().await;
        
        let hashes = chunk_index.remove(chunk_id).unwrap_or_default();
        let count = hashes.len();
        
        for hash in hashes {
            entries.remove(&hash);
        }
        
        Ok(count)
    }
    
    async fn clear(&self) -> Result<()> {
        let mut entries = self.entries.write().await;
        let mut chunk_index = self.chunk_index.write().await;
        
        entries.clear();
        chunk_index.clear();
        
        Ok(())
    }
}
```

### 4.3 Acceptance Criteria

- [ ] MapReduce summarizer handles large description sets
- [ ] LLM caching reduces redundant API calls
- [ ] Cache hit rate is tracked in stats
- [ ] Rebuild from cache works correctly
- [ ] Integration tests pass for caching scenarios

---

## 5. Phase 3: Progress & Cost Tracking

### 5.1 Task List

| Task ID | Task | File(s) | Effort | Dependencies |
|---------|------|---------|--------|--------------|
| P3-01 | Create progress tracking types | types/progress.rs | 3h | None |
| P3-02 | Create cost tracking types | types/cost.rs | 3h | None |
| P3-03 | Implement progress reporter | progress.rs (new) | 4h | P3-01 |
| P3-04 | Implement cost calculator | cost.rs (new) | 3h | P3-02 |
| P3-05 | Integrate into pipeline | pipeline.rs | 4h | P3-03, P3-04 |
| P3-06 | Add progress storage | storage/*.rs | 3h | P3-03 |
| P3-07 | Add event streaming | events.rs (new) | 4h | P3-03 |
| P3-08 | Add tests | tests/*.rs | 3h | P3-01..07 |

### 5.2 Detailed Implementation

#### P3-03: Implement Progress Reporter

**File:** `edgequake/crates/edgequake-core/src/progress.rs` (NEW)

```rust
use chrono::{DateTime, Utc};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Progress tracking for ingestion jobs
pub struct ProgressTracker {
    job_id: String,
    document_id: String,
    state: Arc<RwLock<ProgressState>>,
    event_sender: Option<tokio::sync::broadcast::Sender<ProgressEvent>>,
}

#[derive(Debug, Clone)]
struct ProgressState {
    status: IngestionStatus,
    current_stage: PipelineStage,
    stages: Vec<StageProgress>,
    messages: Vec<ProgressMessage>,
    errors: Vec<IngestionError>,
    started_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl ProgressTracker {
    pub fn new(job_id: String, document_id: String) -> Self {
        let stages = vec![
            StageProgress::new(PipelineStage::Preprocessing),
            StageProgress::new(PipelineStage::Chunking),
            StageProgress::new(PipelineStage::Extracting),
            StageProgress::new(PipelineStage::Merging),
            StageProgress::new(PipelineStage::Embedding),
            StageProgress::new(PipelineStage::Storing),
        ];
        
        Self {
            job_id,
            document_id,
            state: Arc::new(RwLock::new(ProgressState {
                status: IngestionStatus::Pending,
                current_stage: PipelineStage::Preprocessing,
                stages,
                messages: Vec::new(),
                errors: Vec::new(),
                started_at: Utc::now(),
                updated_at: Utc::now(),
            })),
            event_sender: None,
        }
    }
    
    pub fn with_event_channel(mut self, sender: tokio::sync::broadcast::Sender<ProgressEvent>) -> Self {
        self.event_sender = Some(sender);
        self
    }
    
    /// Start processing
    pub async fn start(&self) {
        let mut state = self.state.write().await;
        state.status = IngestionStatus::Running;
        state.updated_at = Utc::now();
        
        self.emit_event(ProgressEvent::Started {
            job_id: self.job_id.clone(),
            document_id: self.document_id.clone(),
        }).await;
    }
    
    /// Begin a stage
    pub async fn begin_stage(&self, stage: PipelineStage, total_items: usize) {
        let mut state = self.state.write().await;
        state.current_stage = stage;
        
        if let Some(s) = state.stages.iter_mut().find(|s| s.stage == stage) {
            s.status = StageStatus::Running;
            s.total_items = total_items;
            s.started_at = Some(Utc::now());
        }
        
        state.updated_at = Utc::now();
        self.add_message(&mut state, format!("Starting {}", stage.as_str()));
        
        self.emit_event(ProgressEvent::StageStarted {
            job_id: self.job_id.clone(),
            stage,
            total_items,
        }).await;
    }
    
    /// Update stage progress
    pub async fn update_progress(&self, completed_items: usize, message: Option<&str>) {
        let mut state = self.state.write().await;
        
        if let Some(s) = state.stages.iter_mut().find(|s| s.stage == state.current_stage) {
            s.completed_items = completed_items;
        }
        
        if let Some(msg) = message {
            self.add_message(&mut state, msg.to_string());
        }
        
        state.updated_at = Utc::now();
        
        let progress = self.calculate_percentage(&state);
        self.emit_event(ProgressEvent::Progress {
            job_id: self.job_id.clone(),
            stage: state.current_stage,
            completed: completed_items,
            percentage: progress,
        }).await;
    }
    
    /// Complete a stage
    pub async fn complete_stage(&self, stage: PipelineStage) {
        let mut state = self.state.write().await;
        
        if let Some(s) = state.stages.iter_mut().find(|s| s.stage == stage) {
            s.status = StageStatus::Completed;
            s.completed_items = s.total_items;
            s.completed_at = Some(Utc::now());
        }
        
        state.updated_at = Utc::now();
        self.add_message(&mut state, format!("Completed {}", stage.as_str()));
        
        self.emit_event(ProgressEvent::StageCompleted {
            job_id: self.job_id.clone(),
            stage,
        }).await;
    }
    
    /// Record an error
    pub async fn record_error(&self, error: IngestionError) {
        let mut state = self.state.write().await;
        state.errors.push(error.clone());
        state.updated_at = Utc::now();
        
        if !error.recoverable {
            state.status = IngestionStatus::Failed;
        }
        
        self.emit_event(ProgressEvent::Error {
            job_id: self.job_id.clone(),
            error,
        }).await;
    }
    
    /// Complete the job
    pub async fn complete(&self, result: IngestionResult) {
        let mut state = self.state.write().await;
        state.status = IngestionStatus::Completed;
        state.updated_at = Utc::now();
        
        self.emit_event(ProgressEvent::Completed {
            job_id: self.job_id.clone(),
            result,
        }).await;
    }
    
    fn calculate_percentage(&self, state: &ProgressState) -> f32 {
        let total_weight: f32 = state.stages.len() as f32;
        let completed: f32 = state.stages.iter()
            .map(|s| match s.status {
                StageStatus::Completed => 1.0,
                StageStatus::Running if s.total_items > 0 => {
                    s.completed_items as f32 / s.total_items as f32
                }
                _ => 0.0,
            })
            .sum();
        
        (completed / total_weight) * 100.0
    }
    
    fn add_message(&self, state: &mut ProgressState, message: String) {
        state.messages.push(ProgressMessage {
            message,
            level: MessageLevel::Info,
            timestamp: Utc::now(),
        });
    }
    
    async fn emit_event(&self, event: ProgressEvent) {
        if let Some(sender) = &self.event_sender {
            let _ = sender.send(event);
        }
    }
    
    /// Get current progress snapshot
    pub async fn snapshot(&self) -> IngestionProgress {
        let state = self.state.read().await;
        IngestionProgress {
            job_id: self.job_id.clone(),
            document_id: self.document_id.clone(),
            status: state.status,
            current_stage: state.current_stage,
            completion_percentage: self.calculate_percentage(&state),
            stages: state.stages.clone(),
            latest_message: state.messages.last()
                .map(|m| m.message.clone())
                .unwrap_or_default(),
            history_messages: state.messages.clone(),
            errors: state.errors.clone(),
            started_at: state.started_at,
            updated_at: state.updated_at,
            completed_at: None,
            eta_seconds: None,
        }
    }
}
```

#### P3-04: Implement Cost Calculator

**File:** `edgequake/crates/edgequake-core/src/cost.rs` (NEW)

```rust
use std::collections::HashMap;

/// Cost configuration for different models
#[derive(Debug, Clone)]
pub struct ModelCost {
    /// Cost per 1000 input tokens
    pub input_per_1k: f64,
    /// Cost per 1000 output tokens
    pub output_per_1k: f64,
}

/// Known model costs (as of Dec 2024)
lazy_static::lazy_static! {
    static ref MODEL_COSTS: HashMap<&'static str, ModelCost> = {
        let mut m = HashMap::new();
        m.insert("gpt-4o-mini", ModelCost { input_per_1k: 0.00015, output_per_1k: 0.0006 });
        m.insert("gpt-4o", ModelCost { input_per_1k: 0.005, output_per_1k: 0.015 });
        m.insert("gpt-4", ModelCost { input_per_1k: 0.03, output_per_1k: 0.06 });
        m.insert("text-embedding-3-small", ModelCost { input_per_1k: 0.00002, output_per_1k: 0.0 });
        m.insert("text-embedding-3-large", ModelCost { input_per_1k: 0.00013, output_per_1k: 0.0 });
        m
    };
}

/// Cost calculator for ingestion operations
pub struct CostCalculator {
    custom_costs: HashMap<String, ModelCost>,
}

impl CostCalculator {
    pub fn new() -> Self {
        Self {
            custom_costs: HashMap::new(),
        }
    }
    
    pub fn with_custom_cost(mut self, model: &str, cost: ModelCost) -> Self {
        self.custom_costs.insert(model.to_string(), cost);
        self
    }
    
    /// Calculate cost for an operation
    pub fn calculate(
        &self,
        model: &str,
        input_tokens: usize,
        output_tokens: usize,
    ) -> f64 {
        let cost = self.custom_costs.get(model)
            .or_else(|| MODEL_COSTS.get(model))
            .cloned()
            .unwrap_or(ModelCost { input_per_1k: 0.0, output_per_1k: 0.0 });
        
        let input_cost = (input_tokens as f64 / 1000.0) * cost.input_per_1k;
        let output_cost = (output_tokens as f64 / 1000.0) * cost.output_per_1k;
        
        input_cost + output_cost
    }
    
    /// Create cost breakdown from processing stats
    pub fn create_breakdown(
        &self,
        extraction_model: &str,
        embedding_model: &str,
        stats: &ProcessingStats,
    ) -> CostBreakdown {
        let extraction_cost = self.calculate(
            extraction_model,
            stats.extraction_input_tokens,
            stats.extraction_output_tokens,
        );
        
        let gleaning_cost = self.calculate(
            extraction_model,
            stats.gleaning_input_tokens,
            stats.gleaning_output_tokens,
        );
        
        let summarization_cost = self.calculate(
            extraction_model,
            stats.summarization_input_tokens,
            stats.summarization_output_tokens,
        );
        
        let embedding_cost = self.calculate(
            embedding_model,
            stats.embedding_tokens,
            0,
        );
        
        CostBreakdown {
            extraction: OperationCost {
                api_calls: stats.extraction_calls,
                input_tokens: stats.extraction_input_tokens,
                output_tokens: stats.extraction_output_tokens,
                cost_usd: extraction_cost,
                model: extraction_model.to_string(),
            },
            gleaning: OperationCost {
                api_calls: stats.gleaning_calls,
                input_tokens: stats.gleaning_input_tokens,
                output_tokens: stats.gleaning_output_tokens,
                cost_usd: gleaning_cost,
                model: extraction_model.to_string(),
            },
            summarization: OperationCost {
                api_calls: stats.summarization_calls,
                input_tokens: stats.summarization_input_tokens,
                output_tokens: stats.summarization_output_tokens,
                cost_usd: summarization_cost,
                model: extraction_model.to_string(),
            },
            embedding: OperationCost {
                api_calls: stats.embedding_calls,
                input_tokens: stats.embedding_tokens,
                output_tokens: 0,
                cost_usd: embedding_cost,
                model: embedding_model.to_string(),
            },
            total_usd: extraction_cost + gleaning_cost + summarization_cost + embedding_cost,
        }
    }
}
```

### 5.3 Acceptance Criteria

- [ ] Progress is tracked at stage level
- [ ] Messages are recorded in history
- [ ] Errors are tracked with context
- [ ] Cost is calculated accurately per model
- [ ] Events are emitted in real-time
- [ ] Progress can be queried via API

---

## 6. Phase 4: Lineage & Document Management

### 6.1 Task List

| Task ID | Task | File(s) | Effort | Dependencies |
|---------|------|---------|--------|--------------|
| P4-01 | Create lineage types | types/lineage.rs | 3h | None |
| P4-02 | Implement lineage storage | storage/lineage.rs | 4h | P4-01 |
| P4-03 | Integrate lineage into pipeline | pipeline.rs | 4h | P4-02 |
| P4-04 | Implement document suppression | documents.rs | 4h | P4-03 |
| P4-05 | Implement cascade delete | graph.rs | 4h | P4-04 |
| P4-06 | Add impact analysis | handlers/documents.rs | 3h | P4-05 |
| P4-07 | Add tests | tests/*.rs | 4h | P4-01..06 |

### 6.2 Acceptance Criteria

- [ ] Lineage tracks document → chunk → entity/relationship
- [ ] Line numbers are preserved in lineage
- [ ] Document suppression removes associated graph entries
- [ ] Orphaned entities are handled correctly
- [ ] Impact analysis shows deletion effects before execution

---

## 7. Phase 5: API & Integration

### 7.1 Task List

| Task ID | Task | File(s) | Effort | Dependencies |
|---------|------|---------|--------|--------------|
| P5-01 | Add progress endpoints | handlers/pipeline.rs | 3h | Phase 3 |
| P5-02 | Add lineage endpoints | handlers/documents.rs | 3h | Phase 4 |
| P5-03 | Add cost endpoints | handlers/costs.rs | 2h | Phase 3 |
| P5-04 | Implement WebSocket handler | ws.rs (new) | 6h | Phase 3 |
| P5-05 | Update OpenAPI spec | openapi.rs | 3h | P5-01..04 |
| P5-06 | Create E2E tests | e2e/*.rs | 6h | P5-01..05 |
| P5-07 | Update documentation | docs/*.md | 4h | P5-01..06 |

### 7.2 Acceptance Criteria

- [ ] All API endpoints documented in OpenAPI
- [ ] WebSocket events work for progress tracking
- [ ] E2E tests cover full ingestion flow
- [ ] Documentation is updated with new features

---

## 8. Code Changes Reference

### 8.1 Files to Modify

| File | Phase | Changes |
|------|-------|---------|
| edgequake-pipeline/src/chunker.rs | 1 | Line number tracking |
| edgequake-pipeline/src/pipeline.rs | 1, 2 | Parallel processing, caching |
| edgequake-pipeline/src/extractor.rs | 1, 2 | Token tracking, caching |
| edgequake-pipeline/src/merger.rs | 2 | MapReduce integration |
| edgequake-pipeline/src/summarizer.rs | 2 | MapReduce summarizer |
| edgequake-core/src/orchestrator.rs | 3, 4 | Progress, lineage |
| edgequake-storage/src/traits/*.rs | 4 | Lineage storage |
| edgequake-api/src/handlers/*.rs | 5 | New endpoints |

### 8.2 New Files to Create

| File | Phase | Purpose |
|------|-------|---------|
| edgequake-pipeline/src/cache.rs | 2 | LLM caching |
| edgequake-core/src/progress.rs | 3 | Progress tracking |
| edgequake-core/src/cost.rs | 3 | Cost calculation |
| edgequake-core/src/types/lineage.rs | 4 | Lineage types |
| edgequake-storage/src/adapters/lineage.rs | 4 | Lineage storage |
| edgequake-api/src/ws.rs | 5 | WebSocket handler |

---

## 9. Migration Strategy

### 9.1 Database Migrations

```sql
-- Migration 001: Add line numbers to chunks
ALTER TABLE chunks ADD COLUMN start_line INTEGER NOT NULL DEFAULT 1;
ALTER TABLE chunks ADD COLUMN end_line INTEGER NOT NULL DEFAULT 1;

-- Migration 002: Add lineage tables
CREATE TABLE document_lineage (...);
CREATE TABLE chunk_lineage (...);
CREATE TABLE entity_lineage (...);

-- Migration 003: Add cost tracking
CREATE TABLE ingestion_costs (...);

-- Migration 004: Add LLM cache
CREATE TABLE llm_cache (...);
```

### 9.2 Backward Compatibility

- All new fields have sensible defaults
- API changes are additive (new endpoints)
- Existing data remains valid
- Lineage can be backfilled from existing data

---

## 10. Risk Assessment

| Risk | Impact | Probability | Mitigation |
|------|--------|-------------|------------|
| Parallel processing increases LLM costs | Medium | Medium | Rate limiting, cost alerts |
| MapReduce adds latency | Low | High | Make optional, tune thresholds |
| Cache invalidation complexity | Medium | Medium | Clear invalidation rules |
| WebSocket scaling issues | High | Low | Load testing, connection limits |
| Migration data loss | High | Low | Backup before migration, rollback plan |

---

## Appendix: Quick Reference

### Commands

```bash
# Run tests
cargo test --package edgequake-pipeline

# Build with all features
cargo build --release --all-features

# Run specific migration
cargo run --bin migrate -- up 001

# Generate OpenAPI spec
cargo run --bin api -- --openapi-only
```

### Feature Flags

```toml
[features]
default = ["parallel", "caching"]
parallel = []
caching = []
mapreduce = []
websocket = ["tokio-tungstenite"]
```

---
