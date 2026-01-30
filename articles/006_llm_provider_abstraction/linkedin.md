# LLM Agnostic Design: Write Once, Deploy Anywhere

**LinkedIn Post** (~2900 chars)

---

Our startup's OpenAI bill jumped from $2.4k to $10.8k overnight.

The CTO asked: "Can we switch to a cheaper model?"

The answer: "Two weeks of refactoring."

Two weeks. For changing a model.

That's provider lock-in.

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

𝗧𝗛𝗘 𝗣𝗥𝗢𝗕𝗟𝗘𝗠

```
OpenAI SDK calls in 50+ files
├── Cost spike? Major refactor
├── Enterprise needs Azure? Rewrite
├── Privacy requires local? Impossible
└── Testing? Paying API costs
```

The LLM landscape changes monthly.
Your code shouldn't have to.

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

𝗧𝗛𝗘 𝗦𝗢𝗟𝗨𝗧𝗜𝗢𝗡

EdgeQuake uses trait-based abstraction:

```
   Your Code
       │
       ▼
┌─────────────────┐
│ LLMProvider     │ ← Single interface
│ Trait           │
└─────────────────┘
       │
   ┌───┴───┬───────┬───────┐
   ▼       ▼       ▼       ▼
OpenAI  Ollama  Azure   Mock
```

Switch providers with ONE environment variable.

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

𝗦𝗨𝗣𝗣𝗢𝗥𝗧𝗘𝗗 𝗣𝗥𝗢𝗩𝗜𝗗𝗘𝗥𝗦

• **OpenAI** (gpt-4o, gpt-4o-mini)
• **Ollama** (llama3.2, qwen2.5) - FREE
• **LM Studio** (any local model)
• **Azure OpenAI** (enterprise)
• **Mock** (testing, CI/CD)

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

𝗖𝗢𝗦𝗧 𝗢𝗣𝗧𝗜𝗠𝗜𝗭𝗔𝗧𝗜𝗢𝗡

| Environment | Provider    | Cost     |
| ----------- | ----------- | -------- |
| Development | Ollama      | $0       |
| CI/Testing  | Mock        | $0       |
| Production  | gpt-4o-mini | $0.75/1M |

1,000 docs/day:
• Ollama: $0/month
• gpt-4o-mini: $42/month
• gpt-4o: $1,200/month

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

𝗛𝗢𝗪 𝗜𝗧 𝗪𝗢𝗥𝗞𝗦

```bash
# Development (free)
export OLLAMA_HOST="http://localhost:11434"

# Production (cloud)
export OPENAI_API_KEY="sk-..."

# Testing (no API calls)
export EDGEQUAKE_LLM_PROVIDER="mock"
```

Same code. Zero changes.

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

EdgeQuake auto-detects your provider.

Set environment variables.
We handle the rest.

Open source: github.com/raphaelmansuy/edgequake

---

The LLM landscape changes monthly.
Your code shouldn't have to.

#AI #LLM #RAG #OpenAI #Ollama #Engineering #Startup
