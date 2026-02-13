# Decision - Iteration 01

## Changes to Make

1. **Add position metadata fields to Chunk struct** (`edgequake-core/src/types/chunk.rs`)
   - Add `start_line: Option<usize>`, `end_line: Option<usize>`
   - Add `start_offset: Option<usize>`, `end_offset: Option<usize>`
   - Make Optional to maintain backward compatibility with existing chunks

2. **Update Chunk::new()** to accept position info via builder pattern
   - Add `with_position()` method for setting line/offset info
   - Keep existing constructor backward compatible

3. **Add tests** for new Chunk position fields

## Priority

1. High impact, low effort — Chunk position fields are foundational

## Expected Outcome

After implementation:

- Chunk struct carries position metadata for traceability
- Existing code continues to work (fields are Optional)
- Foundation laid for subsequent iterations to propagate these fields through pipeline
