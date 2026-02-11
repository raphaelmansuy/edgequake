# Streaming Guide — @edgequake/sdk

## Overview

EdgeQuake supports two streaming protocols:

| Protocol                     | Use Case                             | Transport |
| ---------------------------- | ------------------------------------ | --------- |
| **SSE** (Server-Sent Events) | Query streaming, chat streaming      | HTTP      |
| **WebSocket**                | Pipeline progress, real-time updates | WS        |

## SSE Streaming

### Query Streaming

```typescript
import { EdgeQuake } from "@edgequake/sdk";

const client = new EdgeQuake({
  baseUrl: "http://localhost:8080",
  apiKey: "key",
});

// Stream tokens as they're generated
for await (const event of client.query.stream({
  query: "Explain knowledge graphs",
  mode: "hybrid",
})) {
  process.stdout.write(event.chunk ?? "");
}
```

### Chat Streaming (OpenAI-compatible)

```typescript
for await (const chunk of client.chat.stream({
  model: "edgequake",
  messages: [{ role: "user", content: "Hello" }],
})) {
  const delta = chunk.choices?.[0]?.delta?.content;
  if (delta) process.stdout.write(delta);
}
```

### Aborting a Stream

```typescript
const controller = new AbortController();

// Cancel after 5 seconds
setTimeout(() => controller.abort(), 5000);

try {
  for await (const event of client.query.stream({
    query: "Long query...",
    signal: controller.signal,
  })) {
    process.stdout.write(event.chunk ?? "");
  }
} catch (err) {
  if (err instanceof DOMException && err.name === "AbortError") {
    console.log("Stream cancelled");
  }
}
```

### Graph Streaming

```typescript
// Stream graph data as SSE events
for await (const event of client.graph.stream()) {
  console.log("Graph event:", event);
}
```

## WebSocket Streaming

### Pipeline Progress

```typescript
import { EdgeQuakeWebSocket } from "@edgequake/sdk";

const wsUrl = client.transport.websocketUrl("/ws/pipeline/progress");
const ws = new EdgeQuakeWebSocket(wsUrl);

for await (const event of ws) {
  console.log(`${event.type}: ${event.progress}%`);
  if (event.type === "complete") break;
}

ws.close();
```

### WebSocket Event Types

```typescript
interface WebSocketEvent {
  type: "progress" | "complete" | "error" | "heartbeat";
  task_id?: string;
  progress?: number;
  message?: string;
  timestamp?: string;
}
```

## How SSE Parsing Works

The SDK's SSE parser (`parseSSEStream`) handles:

1. **Buffering**: Accumulates bytes until a complete SSE event (`\n\n`)
2. **Data extraction**: Parses `data: {...}` lines
3. **Sentinel detection**: Stops on `data: [DONE]`
4. **Comment filtering**: Ignores lines starting with `:`
5. **JSON parsing**: Converts data lines to typed objects

```
data: {"token":"Hello"}

data: {"token":" world"}

data: [DONE]

```

## Error Handling

```typescript
try {
  for await (const event of client.query.stream({ query: "..." })) {
    // process event
  }
} catch (err) {
  if (err instanceof NetworkError) {
    console.error("Connection lost during streaming");
  } else if (err instanceof TimeoutError) {
    console.error("Stream timed out");
  }
}
```

## Best Practices

1. **Always handle errors** — streams can disconnect mid-flight
2. **Use AbortController** for "stop generating" functionality
3. **Process events incrementally** — don't buffer everything in memory
4. **Close WebSocket** when done — prevents resource leaks
5. **Set timeouts** for WebSocket connections in production
