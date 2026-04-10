# OODA-35 — Decide

## Tests (~18)
1. ChunkExtractionOutcome::Success — is_success=true, is_failure=false
2. ChunkExtractionOutcome::Failed — is_success=false, is_failure=true
3. chunk_index for Success and Failed
4. as_result returns Some for Success, None for Failed
5. into_result returns Some for Success, None for Failed
6. as_failure returns None for Success, Some for Failed
7. ResilientExtractionResult::from_outcomes — empty vec
8. from_outcomes — all successes
9. from_outcomes — all failures
10. from_outcomes — mixed
11. success_rate — 0 chunks → 1.0
12. success_rate — 3 of 4 → 0.75
13. is_complete_success / is_complete_failure
14. has_any_success
15. summary format
16. PipelineError::ExtractionTimeout Display
17. PipelineError::RetryExhausted Display
18. PipelineError::CircuitBreakerOpen Display
