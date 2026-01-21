# Using OpenAI Provider with EdgeQuake

## Quick Start

### Option 1: Using OpenAI Provider

```bash
# Set your API key
export OPENAI_API_KEY="sk-proj-your-key-here"

# Start EdgeQuake
cd /path/to/edgequake
make dev-bg

# You should see:
# 📝 Using OpenAI provider
# Provider: OpenAI (configured)
```

### Option 2: Using Ollama (Default)

```bash
# No OPENAI_API_KEY needed
cd /path/to/edgequake
make dev-bg

# You should see:
# 📝 Using Ollama as default LLM provider
# Provider: Ollama (http://localhost:11434)
```

## How It Works

### Environment Variable Flow

1. **Shell Environment**:
   ```bash
   export OPENAI_API_KEY="sk-proj-..."
   ```

2. **Makefile Capture**:
   ```makefile
   OPENAI_API_KEY ?= $(shell echo $$OPENAI_API_KEY)
   ```

3. **Backend Process**:
   ```makefile
   OPENAI_API_KEY="$(OPENAI_API_KEY)" cargo run
   ```

4. **Factory Validation**:
   ```rust
   let api_key = std::env::var("OPENAI_API_KEY")?;
   if api_key.is_empty() { return Err(...) }
   ```

### Provider Selection Logic

EdgeQuake supports **two levels** of provider selection:

#### 1. Default Provider (from models.toml)

```toml
[defaults]
llm_provider = "ollama"              # System default
llm_model = "gemma3:12b"
```

This is what the health endpoint shows:
```json
{"llm_provider_name": "ollama"}
```

#### 2. Explicit Provider Selection (from UI/API)

When you select a provider in the UI or API request:
```json
{
  "query": "What is AI?",
  "provider": "openai",
  "model": "gpt-4o-mini"
}
```

The backend will:
1. Try to create the requested provider (OpenAI)
2. Validate configuration (OPENAI_API_KEY must be set)
3. **Return error if validation fails** (OODA Loop 51 fix)
4. Use the requested provider if validation succeeds

## Configuration Check

### Verify OPENAI_API_KEY is Set

```bash
# In your shell
echo $OPENAI_API_KEY
# Should show: sk-proj-fwcb60s...

# In backend process
PID=$(ps aux | grep "target/debug/edgequake$" | grep -v grep | awk '{print $2}')
ps eww -p $PID | grep OPENAI_API_KEY
# Should show: OPENAI_API_KEY=sk-proj-fwcb60s...
```

### Test OpenAI Provider

```bash
curl -X POST http://localhost:8080/api/v1/query \
  -H "Content-Type: application/json" \
  -d '{
    "query": "Test query",
    "provider": "openai",
    "model": "gpt-4o-mini",
    "max_results": 1
  }'
```

**Success response**:
```json
{
  "answer": "Generated response from OpenAI...",
  "conversation_id": "...",
  "sources": [...]
}
```

**Error response** (if key not set):
```json
{
  "error": "Cannot use provider 'openai': Configuration error: OPENAI_API_KEY is empty or invalid. Provide a valid API key from https://platform.openai.com/account/api-keys or select a different provider (ollama, lmstudio, mock)"
}
```

## Troubleshooting

### Issue: "OPENAI_API_KEY is empty or invalid"

**Symptoms**:
- UI shows OpenAI error even though `echo $OPENAI_API_KEY` works
- Health endpoint shows `"llm_provider_name": "ollama"`

**Diagnosis**:
```bash
# Check if backend process has the key
PID=$(ps aux | grep "target/debug/edgequake$" | grep -v grep | awk '{print $2}')
ps eww -p $PID | grep OPENAI_API_KEY
```

**Fix**:
1. Stop backend: `make stop`
2. Set key: `export OPENAI_API_KEY="sk-proj-..."`
3. Restart: `make dev-bg`
4. Verify: Check startup message shows "Using OpenAI provider"

### Issue: Backend started before setting OPENAI_API_KEY

**Problem**: You started `make dev-bg` THEN set `OPENAI_API_KEY`

**Solution**: Environment variables must be set **before** starting the process
```bash
# Wrong order ❌
make dev-bg
export OPENAI_API_KEY="..."  # Too late!

# Correct order ✅
export OPENAI_API_KEY="..."
make dev-bg
```

### Issue: Using wrong make target

**Problem**: `make dev` (interactive) doesn't use Makefile's OPENAI_API_KEY variable

**Solution**: Use `make dev-bg` for background mode with proper env var handling
```bash
# For background/agent mode (recommended)
export OPENAI_API_KEY="..."
make dev-bg

# For interactive mode (terminal stays attached)
DATABASE_URL="..." OPENAI_API_KEY="..." make dev
```

## Provider Priority

When you **don't** explicitly select a provider:

1. **Check `EDGEQUAKE_LLM_PROVIDER`** env var
2. **Auto-detect**: OLLAMA_HOST → LMSTUDIO_HOST → OPENAI_API_KEY
3. **Fallback**: Mock provider (testing)

When you **explicitly** select a provider (UI dropdown or API):

1. **Validate configuration** (e.g., OPENAI_API_KEY for OpenAI)
2. **Return error if invalid** (OODA Loop 51)
3. **Use requested provider** if valid

## API Key Security

### Do's ✅
- Set `OPENAI_API_KEY` in your shell profile (`~/.zshrc`, `~/.bashrc`)
- Use environment variable, never commit to git
- Rotate keys regularly
- Use project-specific keys when possible

### Don'ts ❌
- Don't hardcode keys in code
- Don't commit keys to version control
- Don't share keys in screenshots/logs
- Don't use same key across all projects

## Performance & Costs

### OpenAI (gpt-4o-mini)
- **Speed**: ~2-5 seconds per query
- **Cost**: $0.00015 per 1K input tokens, $0.0006 per 1K output tokens
- **Best for**: Production, high-quality answers, complex reasoning

### Ollama (gemma3:12b)
- **Speed**: ~1-3 seconds per query
- **Cost**: Free (runs locally)
- **Best for**: Development, testing, privacy-sensitive data

### LM Studio
- **Speed**: ~1-4 seconds per query
- **Cost**: Free (runs locally)
- **Best for**: Custom models, offline operation

## Related Documentation

- **SPEC-032**: Multi-provider LLM support specification
- **OODA Loop 51**: Provider error handling implementation
- **models.toml**: Provider and model configuration
- **factory.rs**: Provider creation and validation logic
