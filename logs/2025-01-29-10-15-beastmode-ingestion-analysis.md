# Task Log: Ingestion Performance Analysis

**Date:** 2025-01-29 10:15  
**Mode:** Beast Mode  
**Query:** "Why the ingestion is so SLOW -> Do we use a map reduce / parallel for ingestion"

## Actions

- Analyzed `edgequake-pipeline/src/pipeline.rs` - found PipelineConfig and process flow
- Analyzed `edgequake-pipeline/src/extractor.rs` - found LLM extraction logic
- Analyzed `edgequake-llm/src/providers/openai.rs` - found embedding batch implementation
- Analyzed `edgequake-tasks/src/worker.rs` - found worker pool configuration
- Analyzed `edgequake-api/src/processor.rs` - found document processing handler

## Decisions

- Identified 5 major bottlenecks causing slow ingestion
- Confirmed MapReduce/parallel patterns ARE implemented but underutilized
- Default `max_concurrent_extractions: 4` is too conservative

## Key Findings

### Parallelization EXISTS:

1. Entity extraction uses semaphore + `buffer_unordered()` (4 concurrent max)
2. Worker pool uses `num_cpus::get().max(2)` workers
3. MapReduce summarization in summarizer.rs
4. Embedding provider supports batch API

### Bottlenecks Identified:

1. **Sequential phases** - no overlap between extract/embed/store
2. **Low concurrency** - only 4 concurrent LLM calls
3. **Entity embeddings sequential** - loop over extractions
4. **Document batch processing sequential** - no parallel documents
5. **Graph storage sequential** - one entity at a time

## Next Steps

1. Increase `max_concurrent_extractions` to 8-16 (config change)
2. Parallelize embedding generation with `join_all`
3. Add parallel document batch processing
4. Implement bulk graph storage operations
5. Consider overlapping phases architecture

## Lessons/Insights

- Having parallelism at one level (extraction) isn't enough if other phases are sequential
- Default conservative settings (4 concurrent) may be too low for production
- Batch APIs exist but aren't fully leveraged across all phases
