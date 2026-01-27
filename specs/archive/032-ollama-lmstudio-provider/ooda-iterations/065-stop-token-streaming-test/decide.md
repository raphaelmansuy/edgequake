# Decide: Test Strategy for Stop Tokens

## Decision

Write an integration test that:

1. Calls Ollama with stop tokens via `stream_with_options()`
2. Verifies the output is truncated at the stop sequence
3. Uses real Ollama instance (if available) or mock for CI

## Test Cases

### Test 1: Stop at newline

- Prompt: "Count from 1 to 10, one per line"
- Stop: ["5"]
- Expected: Output stops when "5" appears

### Test 2: Multiple stop sequences

- Prompt: "Tell me about A, B, and C"
- Stop: ["B", "C"]
- Expected: Output stops at first occurrence of B or C

### Test 3: No stop token

- Prompt: "Say hello"
- Stop: []
- Expected: Normal completion

## Implementation

Add test to `ollama.rs` test module.
