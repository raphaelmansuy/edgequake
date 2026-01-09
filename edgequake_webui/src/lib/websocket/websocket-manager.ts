/**
 * @module websocket-manager
 * @description WebSocket Manager Singleton
 *
 * Provides a single shared WebSocket connection for the application.
 * Based on WebUI Specification Document WEBUI-005 (14-webui-websocket-progress.md)
 *
 * @implements FEAT0722 - Singleton WebSocket connection
 * @implements FEAT0723 - Auto-reconnect on disconnect
 *
 * @enforces BR0719 - Single connection per browser tab
 * @enforces BR0720 - Reconnect with exponential backoff
 */

import { ProgressWebSocket } from "./progress-websocket";

let instance: ProgressWebSocket | null = null;

/**
 * Get the WebSocket URL based on the current environment.
 *
 * Backend WebSocket endpoint is at /ws/pipeline/progress
 */
function getWebSocketUrl(): string {
  // Check for environment variable
  const baseUrl =
    process.env.NEXT_PUBLIC_WS_URL || process.env.NEXT_PUBLIC_API_URL;

  if (baseUrl) {
    // Convert http(s) to ws(s)
    const wsUrl = baseUrl.replace(/^https:/, "wss:").replace(/^http:/, "ws:");
    return `${wsUrl}/ws/pipeline/progress`;
  }

  // Browser-based detection
  if (typeof window !== "undefined") {
    const protocol = window.location.protocol === "https:" ? "wss:" : "ws:";
    // Use backend port 9621 for API, the frontend runs on 3000
    const apiHost = process.env.NEXT_PUBLIC_API_URL
      ? new URL(process.env.NEXT_PUBLIC_API_URL).host
      : "localhost:9621";
    return `${protocol}//${apiHost}/ws/pipeline/progress`;
  }

  // Fallback for server-side rendering
  return "ws://localhost:9621/ws/pipeline/progress";
}

/**
 * Get the shared WebSocket client instance.
 * Creates a new instance if one doesn't exist.
 */
export function getWebSocketClient(): ProgressWebSocket {
  if (!instance) {
    instance = new ProgressWebSocket({
      url: getWebSocketUrl(),
      reconnectInterval: 3000,
      maxReconnectAttempts: 10,
      heartbeatInterval: 30000,
    });
  }
  return instance;
}

/**
 * Disconnect and cleanup the WebSocket client.
 */
export function disconnectWebSocket(): void {
  if (instance) {
    instance.disconnect();
    instance = null;
  }
}

/**
 * Check if the WebSocket client is connected.
 */
export function isWebSocketConnected(): boolean {
  return instance?.connected ?? false;
}

/**
 * Check if the WebSocket client is reconnecting.
 */
export function isWebSocketReconnecting(): boolean {
  return instance?.reconnecting ?? false;
}
