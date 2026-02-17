# SPEC-002: Multi-LLM Provider Improvements

## Context

EdgeQuake has migrated from a local `edgequake-llm` crate (v0.2.3) to the externally published `edgequake-llm` crate (v0.2.1 on crates.io). The external crate is a superset of the original, adding:

- **5 new providers**: Anthropic (Claude 3.5/4), OpenRouter (616+ models), xAI (Grok), VSCode Copilot, OpenAI-Compatible (generic)
- **Middleware system**: `LLMMiddleware` trait + `LLMMiddlewareStack` for pluggable request/response processing
- **Cost tracking**: `SessionCostTracker` with per-model pricing and `CostSummary`
- **Tool/function calling**: `ToolDefinition`, `ToolCall`, `ToolResult`, `FunctionDefinition`
- **Provider registry**: `ProviderRegistry` for dynamic provider management
- **Retry system**: `RetryStrategy` + `RetryExecutor` with configurable backoff
- **Inference metrics**: `InferenceMetrics` for streaming performance data
- **Prompt caching**: `CacheControl` for Anthropic prompt caching

This specification identifies high-signal opportunities to leverage these new capabilities in both the EdgeQuake server (edgequake-api) and WebUI (edgequake_webui).

## Current State

### Server

- **6 configurable providers**: OpenAI, Azure OpenAI, Ollama, LM Studio, Gemini, Mock
- **Per-workspace provider config**: `llm_provider`, `llm_model`, `embedding_provider`, `embedding_model`, `embedding_dimension`
- **Provider resolution priority**: Request → Workspace → Server default
- **Safety limits**: Max tokens (32768 hard cap), timeout (10 min), via `SafetyLimitedProviderWrapper`
- **Model catalog**: `models.toml` with 40+ model entries, costs, and capabilities
- **No cost tracking**: Token usage is counted but not priced
- **No middleware pipeline**: Safety limits applied directly via wrapper pattern
- **No tool/function calling**: Chat is text-only

### WebUI

- **Model selector**: Dropdown grouped by provider with capability badges
- **Provider status card**: Health monitoring with manual refresh
- **Settings store**: Per-session provider/model override via Zustand/localStorage
- **No cost dashboard**: Token usage displayed but no billing/cost information
- **No provider comparison**: No way to compare providers side-by-side
- **No provider configuration**: API keys and endpoints managed only via environment variables

---

## Improvement Opportunities

### 1. New Provider Activation

**Priority**: High
**Impact**: Immediately unlocks Anthropic Claude, OpenRouter (616+ models), xAI Grok, and generic OpenAI-compatible endpoints

#### 1.1 Server Changes

- **Update `models.toml`**: Add model cards for Anthropic (claude-sonnet-4-20250514, claude-3.5-haiku), xAI (grok-2, grok-3), OpenRouter top models
- **Update `ProviderFactory` usage**: The external crate already supports these providers; ensure `create_llm_provider` and `create_embedding_provider` accept the new provider names
- **Provider status endpoint**: Extend `GET /api/v1/settings/provider/status` to report health for all available providers (including new ones)
- **Update `default_model_for_provider`** in `safety_limits.rs`: Already done during migration for anthropic, gemini, xai, openrouter
- **Environment variable docs**: Document `ANTHROPIC_API_KEY`, `XAI_API_KEY`, `OPENROUTER_API_KEY`

#### 1.2 WebUI Changes

- **Model selector**: Add provider icons and group entries for Anthropic, xAI, OpenRouter, Gemini
- **Provider configuration page**: Add UI for entering API keys per provider (stored server-side, encrypted at rest)
- **OpenRouter model browser**: Since OpenRouter supports 616+ models, add a searchable model picker with filtering by cost, context length, and capabilities

#### 1.3 Configuration

```toml
# models.toml additions

[[providers]]
name = "anthropic"
display_name = "Anthropic"
env_key = "ANTHROPIC_API_KEY"
base_url = "https://api.anthropic.com"

[[models]]
name = "claude-sonnet-4-20250514"
provider = "anthropic"
display_name = "Claude Sonnet 4"
model_type = "llm"
[models.capabilities]
context_length = 200000
max_output = 8192
vision = true
function_calling = true
streaming = true
json_mode = true
[models.cost]
input_per_1k = 0.003
output_per_1k = 0.015

[[providers]]
name = "openrouter"
display_name = "OpenRouter"
env_key = "OPENROUTER_API_KEY"
base_url = "https://openrouter.ai/api/v1"
```

---

### 2. Cost Tracking & Budget Management

**Priority**: High
**Impact**: Enables cost visibility, per-workspace budgets, and cost optimization decisions

#### 2.1 Server Changes

- **Integrate `SessionCostTracker`**: Wrap each LLM call with cost tracking using the external crate's `SessionCostTracker`
- **Per-workspace cost accumulation**: Store cumulative costs in the workspace record (new fields: `total_input_tokens`, `total_output_tokens`, `total_cost_usd`, `last_cost_reset`)
- **Cost API endpoints**:
  - `GET /api/v1/workspaces/{id}/costs` - Current period cost summary
  - `GET /api/v1/workspaces/{id}/costs/history` - Daily/weekly cost breakdown
  - `POST /api/v1/workspaces/{id}/costs/reset` - Reset cost counters
- **Budget enforcement**:
  - Add `budget_limit_usd` to workspace config
  - Return `429 Too Many Requests` when budget exceeded
  - Warning headers when approaching limit (e.g., `X-EdgeQuake-Budget-Remaining`)
- **Cost-per-document tracking**: Track LLM costs during document ingestion (extraction + embedding)
- **Cost optimization recommendations**: Endpoint that analyzes usage patterns and suggests cheaper alternatives

#### 2.2 WebUI Changes

- **Cost dashboard widget**: Real-time cost display per workspace with daily/weekly charts
- **Budget configuration**: Set monthly/daily budget limits per workspace
- **Cost-per-query display**: Show estimated cost alongside each query result
- **Cost comparison**: Side-by-side comparison of same query across different providers/models
- **Alert thresholds**: Visual warnings when approaching budget limits

#### 2.3 Data Model

```sql
-- New cost tracking table
CREATE TABLE workspace_costs (
    id UUID PRIMARY KEY,
    workspace_id UUID REFERENCES workspaces(id),
    period_start TIMESTAMPTZ NOT NULL,
    period_end TIMESTAMPTZ,
    provider VARCHAR(50) NOT NULL,
    model VARCHAR(100) NOT NULL,
    input_tokens BIGINT DEFAULT 0,
    output_tokens BIGINT DEFAULT 0,
    embedding_tokens BIGINT DEFAULT 0,
    estimated_cost_usd DECIMAL(10, 6) DEFAULT 0,
    request_count INTEGER DEFAULT 0,
    created_at TIMESTAMPTZ DEFAULT NOW()
);
```

---

### 3. Middleware Pipeline

**Priority**: Medium
**Impact**: Extensible request/response processing, observability, and custom logic injection

#### 3.1 Server Changes

- **Adopt `LLMMiddlewareStack`**: Replace the direct `SafetyLimitedProviderWrapper` with a composable middleware stack
- **Built-in middleware layers**:
  1. **Safety limits** (existing): Token capping and timeout enforcement
  2. **Cost tracking**: Intercept responses to accumulate costs
  3. **Request logging**: Structured logging of all LLM requests/responses (opt-in, with PII redaction)
  4. **Metrics collection**: OpenTelemetry-compatible metrics (latency, token counts, error rates)
  5. **Rate limiting**: Per-provider rate limiting to respect API quotas
  6. **Retry with backoff**: Use `RetryExecutor` for transient failures
- **Configurable pipeline**: Allow enabling/disabling middleware per workspace or globally

#### 3.2 Configuration

```toml
[middleware]
# Global middleware settings
enable_cost_tracking = true
enable_request_logging = false
enable_metrics = true
enable_retry = true

[middleware.retry]
max_retries = 3
initial_delay_ms = 1000
max_delay_ms = 30000
strategy = "exponential_backoff"

[middleware.rate_limit]
openai_rpm = 500
anthropic_rpm = 100
```

---

### 4. Tool/Function Calling Support

**Priority**: Medium
**Impact**: Enables agentic workflows, structured data extraction, and dynamic tool use

#### 4.1 Server Changes

- **Chat endpoint enhancement**: Extend `POST /api/v1/chat` to accept tool definitions in the request
- **Built-in tools**:
  1. **Document search**: Search workspace documents as a callable function
  2. **Knowledge graph query**: Query the graph as a tool
  3. **Web search** (optional): External search integration
  4. **Code execution** (optional): Sandboxed code execution for data analysis
- **Tool result injection**: When the LLM returns a `ToolCall`, execute the tool and feed results back
- **Multi-turn tool use**: Support iterative tool calling (up to configurable max rounds)
- **Streaming with tools**: Leverage `chat_with_tools_stream` for real-time tool execution feedback

#### 4.2 WebUI Changes

- **Tool indicators**: Show when a response used tools (e.g., "Used: document search, graph query")
- **Tool execution timeline**: Visual timeline showing tool calls and their results
- **Tool configuration**: Per-workspace tool enablement settings
- **Tool result display**: Rich rendering of tool results (tables, graphs, code blocks)

#### 4.3 API Extension

```json
// POST /api/v1/chat request with tools
{
  "message": "Find all documents about machine learning and summarize their key findings",
  "tools": [
    {
      "type": "function",
      "function": {
        "name": "search_documents",
        "description": "Search workspace documents",
        "parameters": {
          "type": "object",
          "properties": {
            "query": { "type": "string" },
            "limit": { "type": "integer", "default": 10 }
          }
        }
      }
    }
  ],
  "tool_choice": "auto"
}
```

---

### 5. Provider Comparison & A/B Testing

**Priority**: Low
**Impact**: Enables data-driven provider selection and quality benchmarking

#### 5.1 Server Changes

- **Multi-provider query**: New endpoint `POST /api/v1/query/compare` that sends the same query to multiple providers in parallel
- **Quality metrics**: Track response quality signals (response length, entity count, user feedback)
- **A/B testing framework**: Randomly route a percentage of queries to alternative providers for comparison
- **Provider switchover**: Automated provider switching when a provider degrades (latency > threshold, error rate > threshold)

#### 5.2 WebUI Changes

- **Comparison view**: Side-by-side response comparison for multiple providers
- **Quality graphs**: Historical quality metrics per provider
- **Provider recommendation**: Based on cost/quality/latency tradeoffs
- **Provider health timeline**: Historical uptime and latency charts

---

### 6. Enhanced Provider Configuration UI

**Priority**: Medium
**Impact**: Reduces operational friction, enables self-service provider management

#### 6.1 WebUI Changes

- **Provider management page**: Full CRUD for provider configurations
  - Add/remove providers
  - Configure API keys (encrypted storage)
  - Set base URLs for self-hosted models
  - Test connectivity
- **Model browser**: Searchable catalog of all available models across providers
  - Filter by: capability (vision, function calling), cost tier, context length, provider
  - Sort by: cost, context length, popularity
  - Display: pricing, capabilities, recommended use cases
- **Quick-switch provider**: One-click provider switch for the current workspace
- **Provider templates**: Pre-configured provider setups (e.g., "Local-only", "Cloud-optimized", "Hybrid")
- **Import/export config**: Export/import workspace provider configuration as JSON/TOML

#### 6.2 Server Changes

- **Provider validation endpoint**: `POST /api/v1/providers/validate` - Test if a provider config (API key, endpoint) is valid
- **Provider listing endpoint**: `GET /api/v1/providers` - List all available providers with their capabilities and required configuration
- **Secure key storage**: Encrypt API keys at rest in the database, never expose in API responses
- **Connection pooling**: Reuse HTTP connections per-provider to reduce latency

---

### 7. Prompt Caching (Anthropic)

**Priority**: Low
**Impact**: Up to 90% cost reduction for repeated system prompts (Anthropic only)

#### 7.1 Server Changes

- **Automatic cache control**: When using Anthropic providers, automatically set `CacheControl::Ephemeral` on system prompts
- **Cache hit tracking**: Use `cache_hit_tokens` from `LLMResponse` to track cache efficiency
- **Cache metrics endpoint**: `GET /api/v1/providers/anthropic/cache-stats` - Cache hit rate, tokens saved, cost saved

#### 7.2 WebUI Changes

- **Cache efficiency indicator**: Show cache hit rate for Anthropic workspaces
- **Cost savings display**: Show estimated savings from prompt caching

---

## Implementation Priority Matrix

| Opportunity                | Priority | Effort | Impact | Dependencies                      |
| -------------------------- | -------- | ------ | ------ | --------------------------------- |
| 1. New Provider Activation | High     | Low    | High   | None - providers already in crate |
| 2. Cost Tracking           | High     | Medium | High   | DB schema migration               |
| 3. Middleware Pipeline     | Medium   | Medium | Medium | Refactor safety_limits            |
| 4. Tool/Function Calling   | Medium   | High   | High   | Chat handler redesign             |
| 5. Provider Comparison     | Low      | Medium | Medium | Cost tracking (#2)                |
| 6. Provider Config UI      | Medium   | Medium | High   | Secure key storage                |
| 7. Prompt Caching          | Low      | Low    | Low    | Anthropic provider only           |

## Recommended Implementation Order

1. **New Provider Activation** (immediate value, minimal changes)
2. **Cost Tracking & Budget Management** (financial visibility, enabler for other features)
3. **Enhanced Provider Configuration UI** (operational improvement)
4. **Middleware Pipeline** (architectural improvement, enables extensibility)
5. **Tool/Function Calling** (new capabilities, agentic workflows)
6. **Provider Comparison** (optimization, builds on cost tracking)
7. **Prompt Caching** (optimization, Anthropic-specific)
