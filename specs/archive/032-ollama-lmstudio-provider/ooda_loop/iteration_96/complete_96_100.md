# OODA Iterations 96-100: Edge Case Testing

## Iteration 96: Empty Provider String

**Test**: `{"provider": "", "message": "test"}`
**Expected**: Use workspace provider
**Result**: ✅ Workspace OpenAI used

## Iteration 97: Invalid Workspace ID

**Test**: `X-Workspace-Id: 00000000-0000-0000-0000-000000000000`
**Expected**: Workspace not found, use server default
**Result**: ✅ Falls back to Ollama

## Iteration 98: Workspace Without LLM Config

**Test**: Default workspace with no explicit LLM settings
**Expected**: Use workspace defaults (Ollama/gemma3:12b)
**Result**: ✅ Uses workspace's default configuration

## Iteration 99: Mixed Provider/Model in Request

**Test**: `{"provider": "openai/gpt-4o"}`
**Expected**: Parse legacy format correctly
**Result**: ✅ OpenAI/gpt-4o used

## Iteration 100: Unicode in Message with Workspace Provider

**Test**: `{"message": "Say 你好世界"}`
**Expected**: OpenAI handles Unicode correctly
**Result**: ✅ Response includes Chinese characters
