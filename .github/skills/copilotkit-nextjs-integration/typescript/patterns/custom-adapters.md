# Custom LLM Adapters

Integrate any LLM service (self-hosted, proprietary, or custom) with CopilotKit.

## Overview

An adapter is a bridge between CopilotKit and an LLM provider. It translates CopilotKit's chat format into your LLM's API format.

## Adapter Interface

```typescript
import { CopilotServiceAdapter } from "@copilotkit/runtime";

interface CopilotServiceAdapter {
  streamChatCompletion(
    options: ChatCompletionOptions
  ): Promise<ReadableStream>;
  
  // Optional methods (check your CopilotKit version)
  process?(request: any): Promise<Response>;
}

interface ChatCompletionOptions {
  messages: Array<{ role: string; content: string }>;
  model?: string;
  temperature?: number;
  max_tokens?: number;
  // ... other LLM parameters
}
```

## Simple Custom Adapter

For a custom LLM service at `https://my-llm.example.com/api/chat`:

```typescript
// lib/custom-llm-adapter.ts
import type { CopilotServiceAdapter, ChatCompletionOptions } from "@copilotkit/runtime";

export class CustomLLMAdapter implements CopilotServiceAdapter {
  private apiBaseUrl: string;
  private apiKey: string;

  constructor(apiBaseUrl: string, apiKey: string) {
    this.apiBaseUrl = apiBaseUrl;
    this.apiKey = apiKey;
  }

  async streamChatCompletion(
    options: ChatCompletionOptions
  ): Promise<ReadableStream> {
    // Call your LLM API
    const response = await fetch(`${this.apiBaseUrl}/api/chat`, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        "Authorization": `Bearer ${this.apiKey}`
      },
      body: JSON.stringify({
        messages: options.messages,
        temperature: options.temperature ?? 0.7,
        max_tokens: options.max_tokens ?? 1024
      })
    });

    if (!response.ok) {
      throw new Error(
        `LLM API error: ${response.status} ${response.statusText}`
      );
    }

    // Return the streaming response
    if (!response.body) {
      throw new Error("LLM API did not return a stream");
    }

    return response.body;
  }
}
```

**Use it in your runtime endpoint:**

```typescript
// app/api/copilotkit/route.ts
import { CopilotRuntime } from "@copilotkit/runtime";
import { CustomLLMAdapter } from "@/lib/custom-llm-adapter";

const runtime = new CopilotRuntime();

export const POST = async (req: Request) => {
  const adapter = new CustomLLMAdapter(
    process.env.CUSTOM_LLM_BASE_URL!,
    process.env.CUSTOM_LLM_API_KEY!
  );

  return runtime.handleRequest(req, adapter);
};
```

## Ollama / Local LLM Adapter

For Ollama running locally at `http://localhost:11434`:

```typescript
// lib/ollama-adapter.ts
import type { CopilotServiceAdapter, ChatCompletionOptions } from "@copilotkit/runtime";

export class OllamaAdapter implements CopilotServiceAdapter {
  private modelName: string;

  constructor(modelName: string = "llama2") {
    this.modelName = modelName;
  }

  async streamChatCompletion(
    options: ChatCompletionOptions
  ): Promise<ReadableStream> {
    const response = await fetch("http://localhost:11434/api/chat", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        model: this.modelName,
        messages: options.messages,
        stream: true,
        temperature: options.temperature ?? 0.7
      })
    });

    if (!response.ok) {
      throw new Error(`Ollama error: ${response.statusText}`);
    }

    if (!response.body) {
      throw new Error("Ollama did not return a stream");
    }

    return response.body;
  }
}
```

## LLM Studio / LM Studio Adapter

For LM Studio's OpenAI-compatible API:

```typescript
// lib/lmstudio-adapter.ts
import { OpenAIAdapter } from "@copilotkit/runtime";

export class LMStudioAdapter extends OpenAIAdapter {
  constructor() {
    // LM Studio runs on localhost:1234 with OpenAI-compatible API
    super({
      apiKey: "not-needed", // LM Studio doesn't require an API key
      baseURL: "http://localhost:1234/v1" // Custom base URL
    });
  }
}
```

## Hugging Face Inference API Adapter

```typescript
// lib/huggingface-adapter.ts
import type { CopilotServiceAdapter, ChatCompletionOptions } from "@copilotkit/runtime";

export class HuggingFaceAdapter implements CopilotServiceAdapter {
  private apiToken: string;
  private model: string;

  constructor(apiToken: string, model: string = "meta-llama/Llama-2-7b-chat") {
    this.apiToken = apiToken;
    this.model = model;
  }

  async streamChatCompletion(
    options: ChatCompletionOptions
  ): Promise<ReadableStream> {
    const response = await fetch(
      "https://api-inference.huggingface.co/models/meta-llama/Llama-2-7b-chat",
      {
        method: "POST",
        headers: {
          Authorization: `Bearer ${this.apiToken}`,
          "Content-Type": "application/json"
        },
        body: JSON.stringify({
          inputs: options.messages
            .map(m => `${m.role}: ${m.content}`)
            .join("\n"),
          parameters: {
            max_new_tokens: options.max_tokens ?? 1024,
            temperature: options.temperature ?? 0.7
          },
          stream: true
        })
      }
    );

    if (!response.ok) {
      throw new Error(`Hugging Face error: ${response.statusText}`);
    }

    if (!response.body) {
      throw new Error("No stream returned");
    }

    return response.body;
  }
}
```

## Advanced: Adapter with Request Transformation

For APIs with different message formats:

```typescript
// lib/custom-format-adapter.ts
import type { CopilotServiceAdapter, ChatCompletionOptions } from "@copilotkit/runtime";

interface CustomMessageFormat {
  speaker: string; // 'user' or 'assistant'
  text: string;
}

export class CustomFormatAdapter implements CopilotServiceAdapter {
  async streamChatCompletion(
    options: ChatCompletionOptions
  ): Promise<ReadableStream> {
    // Transform CopilotKit format to custom format
    const customMessages: CustomMessageFormat[] = options.messages.map(msg => ({
      speaker: msg.role === "user" ? "user" : "assistant",
      text: msg.content
    }));

    const response = await fetch("https://my-llm-api.example.com/chat", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        conversation: customMessages,
        maxLength: options.max_tokens ?? 1024
      })
    });

    if (!response.ok) {
      throw new Error(`API error: ${response.statusText}`);
    }

    if (!response.body) {
      throw new Error("No stream");
    }

    return response.body;
  }
}
```

## Adapter with Error Handling & Retries

```typescript
// lib/resilient-adapter.ts
import type { CopilotServiceAdapter, ChatCompletionOptions } from "@copilotkit/runtime";

export class ResilientAdapter implements CopilotServiceAdapter {
  private baseURL: string;
  private apiKey: string;
  private maxRetries: number = 3;

  constructor(baseURL: string, apiKey: string) {
    this.baseURL = baseURL;
    this.apiKey = apiKey;
  }

  private async fetchWithRetries<T>(
    url: string,
    options: RequestInit,
    retries: number = 0
  ): Promise<Response> {
    try {
      const response = await fetch(url, options);

      if (!response.ok && response.status >= 500 && retries < this.maxRetries) {
        // Retry on server error
        await new Promise(resolve => setTimeout(resolve, 1000 * (retries + 1)));
        return this.fetchWithRetries(url, options, retries + 1);
      }

      return response;
    } catch (error) {
      if (retries < this.maxRetries) {
        await new Promise(resolve => setTimeout(resolve, 1000 * (retries + 1)));
        return this.fetchWithRetries(url, options, retries + 1);
      }
      throw error;
    }
  }

  async streamChatCompletion(
    options: ChatCompletionOptions
  ): Promise<ReadableStream> {
    const response = await this.fetchWithRetries(
      `${this.baseURL}/api/chat`,
      {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          "Authorization": `Bearer ${this.apiKey}`
        },
        body: JSON.stringify(options)
      }
    );

    if (!response.ok) {
      throw new Error(
        `LLM API failed: ${response.status} ${response.statusText}`
      );
    }

    if (!response.body) {
      throw new Error("No stream returned");
    }

    return response.body;
  }
}
```

## Adapter with Logging

```typescript
// lib/logged-adapter.ts
import type { CopilotServiceAdapter, ChatCompletionOptions } from "@copilotkit/runtime";

export class LoggedAdapter implements CopilotServiceAdapter {
  private innerAdapter: CopilotServiceAdapter;

  constructor(innerAdapter: CopilotServiceAdapter) {
    this.innerAdapter = innerAdapter;
  }

  async streamChatCompletion(
    options: ChatCompletionOptions
  ): Promise<ReadableStream> {
    console.log("[Copilot] Chat completion started", {
      messageCount: options.messages.length,
      temperature: options.temperature,
      maxTokens: options.max_tokens
    });

    const startTime = Date.now();

    try {
      const stream = await this.innerAdapter.streamChatCompletion(options);
      console.log("[Copilot] Stream obtained in", Date.now() - startTime, "ms");
      return stream;
    } catch (error) {
      console.error("[Copilot] Error:", error);
      throw error;
    }
  }
}
```

## Complete Example: Production Adapter

```typescript
// lib/production-adapter.ts
import type { CopilotServiceAdapter, ChatCompletionOptions } from "@copilotkit/runtime";

interface AdapterConfig {
  apiBaseUrl: string;
  apiKey: string;
  modelName: string;
  timeout?: number;
  maxRetries?: number;
}

export class ProductionAdapter implements CopilotServiceAdapter {
  private config: AdapterConfig;

  constructor(config: AdapterConfig) {
    this.config = {
      timeout: 30000,
      maxRetries: 2,
      ...config
    };
  }

  private async fetchWithTimeout(
    url: string,
    options: RequestInit
  ): Promise<Response> {
    const controller = new AbortController();
    const timeout = setTimeout(
      () => controller.abort(),
      this.config.timeout!
    );

    try {
      return await fetch(url, {
        ...options,
        signal: controller.signal
      });
    } finally {
      clearTimeout(timeout);
    }
  }

  async streamChatCompletion(
    options: ChatCompletionOptions
  ): Promise<ReadableStream> {
    let lastError: Error | null = null;

    for (let attempt = 0; attempt <= this.config.maxRetries!; attempt++) {
      try {
        const response = await this.fetchWithTimeout(
          `${this.config.apiBaseUrl}/chat`,
          {
            method: "POST",
            headers: {
              "Content-Type": "application/json",
              "Authorization": `Bearer ${this.config.apiKey}`
            },
            body: JSON.stringify({
              model: this.config.modelName,
              messages: options.messages,
              temperature: options.temperature ?? 0.7,
              max_tokens: options.max_tokens ?? 1024,
              stream: true
            })
          }
        );

        if (response.ok && response.body) {
          return response.body;
        }

        if (response.status >= 500) {
          throw new Error(
            `Server error: ${response.status} ${response.statusText}`
          );
        }

        throw new Error(
          `HTTP ${response.status}: ${response.statusText}`
        );
      } catch (error) {
        lastError = error instanceof Error ? error : new Error(String(error));

        if (attempt < this.config.maxRetries!) {
          // Exponential backoff
          await new Promise(resolve =>
            setTimeout(resolve, 1000 * Math.pow(2, attempt))
          );
        }
      }
    }

    throw new Error(
      `Failed after ${this.config.maxRetries! + 1} attempts: ${lastError?.message}`
    );
  }
}
```

## Testing Your Adapter

```typescript
// __tests__/custom-adapter.test.ts
import { CustomLLMAdapter } from "@/lib/custom-llm-adapter";

describe("CustomLLMAdapter", () => {
  it("should handle chat completion", async () => {
    const adapter = new CustomLLMAdapter(
      "http://localhost:8000",
      "test-key"
    );

    const stream = await adapter.streamChatCompletion({
      messages: [{ role: "user", content: "Hello" }],
      temperature: 0.7,
      max_tokens: 1024
    });

    expect(stream).toBeDefined();
  });

  it("should handle errors gracefully", async () => {
    const adapter = new CustomLLMAdapter(
      "http://invalid-url:9999",
      "test-key"
    );

    await expect(
      adapter.streamChatCompletion({
        messages: [{ role: "user", content: "Hello" }]
      })
    ).rejects.toThrow();
  });
});
```

## Deployment Considerations

- **Security**: Keep API keys in environment variables only
- **Timeouts**: Set appropriate request timeouts for your LLM
- **Rate Limiting**: Implement rate limiting on your API
- **Monitoring**: Log adapter calls for debugging and analytics
- **Fallback**: Consider a fallback adapter if primary fails

## Next Steps

- Deploy with authentication (see [production-security.md](production-security.md))
- Monitor adapter performance
- Set up alert for API failures
- Consider caching responses for common queries
