# Analysis - Iteration 02

## Gap

Per-chunk model metadata missing → cannot trace which model processed each chunk individually.

## Solution

Add `llm_model`, `embedding_model`, `embedding_dimension` as Optional fields to Chunk struct with builder pattern `with_models()`.

## Risk: Low — all Optional, backward compatible.
