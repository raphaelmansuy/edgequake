/**
 * WebSocket Progress — EdgeQuake TypeScript SDK
 *
 * WHY: WebSocket provides real-time pipeline progress updates
 * (e.g., document processing, entity extraction) without polling.
 *
 * Usage:
 *   npx tsx examples/websocket_progress.ts
 */
import { EdgeQuake, EdgeQuakeWebSocket } from "@edgequake/sdk";

async function main() {
  const client = new EdgeQuake({
    baseUrl: process.env.EDGEQUAKE_URL ?? "http://localhost:8080",
    apiKey: process.env.EDGEQUAKE_API_KEY ?? "demo-key",
  });

  // ── 1. Get pipeline status ────────────────────────────────

  const status = await client.pipeline.status();
  console.log("Pipeline status:", status);

  // ── 2. Connect to WebSocket for real-time progress ────────

  // WHY: The SDK provides EdgeQuakeWebSocket, an async iterable wrapper
  // around native WebSocket. It enables `for await...of` syntax.
  const wsUrl = client.transport.websocketUrl("/ws/pipeline/progress");
  console.log(`Connecting to WebSocket: ${wsUrl}`);

  const ws = new EdgeQuakeWebSocket(wsUrl);

  // Set a timeout to close after 30 seconds
  const timeout = setTimeout(() => {
    console.log("\n[Closing WebSocket after 30 seconds]");
    ws.close();
  }, 30_000);

  try {
    for await (const event of ws) {
      console.log(`[${event.type}] Progress: ${event.progress ?? "N/A"}%`);
      if (event.type === "complete") {
        console.log("Pipeline processing complete!");
        break;
      }
    }
  } finally {
    clearTimeout(timeout);
    ws.close();
  }

  // ── 3. Task tracking ──────────────────────────────────────

  // WHY: Individual tasks can be tracked by ID separately from
  // the global pipeline WebSocket.
  const tasks = await client.tasks.list();
  console.log(`\nActive tasks: ${tasks.length}`);
  for (const task of tasks.slice(0, 5)) {
    console.log(`  ${task.id}: ${task.status} (${task.progress}%)`);
  }
}

main().catch(console.error);
