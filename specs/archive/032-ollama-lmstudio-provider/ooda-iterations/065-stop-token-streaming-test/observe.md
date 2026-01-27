# Observe: Stop Token Streaming Test

## Objective

Test that stop tokens are correctly passed through the streaming API and work with Ollama provider.

## Current State

In OODA 63, we implemented `stream_with_options()` method in all providers:

- Ollama: Passes stop tokens via `ChatOptions::stop`
- OpenAI: Uses `.stop(stop.clone())` builder pattern
- LMStudio: Added `stop` field to `ChatCompletionRequest`
- Gemini: Uses `GenerationConfig::stop_sequences`

## Test Approach

1. Create a unit test that uses streaming with stop tokens
2. Verify the stop token is respected by Ollama
3. Check the output is truncated at the stop sequence

## Observations

Need to verify:

- Stop tokens are correctly serialized in Ollama API request
- Streaming response stops at the stop sequence
- No regression in normal streaming behavior
