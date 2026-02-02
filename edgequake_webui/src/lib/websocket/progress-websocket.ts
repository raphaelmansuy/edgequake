/**
 * @module progress-websocket
 * @description WebSocket Client for Progress Tracking
 *
 * Provides real-time progress updates for document ingestion.
 * Based on WebUI Specification Document WEBUI-005 (14-webui-websocket-progress.md)
 *
 * @implements FEAT0724 - Real-time ingestion progress
 * @implements FEAT0725 - Heartbeat keep-alive
 * @implements FEAT0726 - Subscription-based updates
 *
 * @enforces BR0721 - Heartbeat every 30s
 * @enforces BR0722 - Max 5 reconnect attempts
 * @enforces BR0723 - Clean disconnect on page unload
 */

import type {
  ClientCommand,
  WebSocketProgressMessage,
} from "@/types/ingestion";

export interface ProgressWebSocketOptions {
  url: string;
  reconnectInterval?: number;
  maxReconnectAttempts?: number;
  heartbeatInterval?: number;
  onConnected?: () => void;
  onDisconnected?: (event: { code: number; reason: string }) => void;
  onReconnecting?: (attempt: number) => void;
  onMaxReconnectsReached?: () => void;
  onError?: (error: Error) => void;
  onMessage?: (message: WebSocketProgressMessage) => void;
}

type WebSocketEventType =
  | "connected"
  | "disconnected"
  | "reconnecting"
  | "max_reconnects_reached"
  | "error"
  | "progress"
  | "status_snapshot"
  | "pdf_progress";

type WebSocketEventCallback = (...args: unknown[]) => void;

/**
 * WebSocket client for real-time ingestion progress tracking.
 *
 * Features:
 * - Automatic reconnection with exponential backoff
 * - Heartbeat to keep connection alive
 * - Type-safe message handling
 * - Subscription management
 */
export class ProgressWebSocket {
  private ws: WebSocket | null = null;
  private reconnectAttempts = 0;
  private heartbeatTimer?: ReturnType<typeof setInterval>;
  private reconnectTimer?: ReturnType<typeof setTimeout>;
  private messageQueue: ClientCommand[] = [];
  private listeners: Map<WebSocketEventType, Set<WebSocketEventCallback>> =
    new Map();

  public readonly options: Required<
    Omit<
      ProgressWebSocketOptions,
      | "onConnected"
      | "onDisconnected"
      | "onReconnecting"
      | "onMaxReconnectsReached"
      | "onError"
      | "onMessage"
    >
  > &
    Partial<
      Pick<
        ProgressWebSocketOptions,
        | "onConnected"
        | "onDisconnected"
        | "onReconnecting"
        | "onMaxReconnectsReached"
        | "onError"
        | "onMessage"
      >
    >;

  private _connected = false;
  private _reconnecting = false;

  get connected(): boolean {
    return this._connected;
  }

  get reconnecting(): boolean {
    return this._reconnecting;
  }

  constructor(options: ProgressWebSocketOptions) {
    this.options = {
      reconnectInterval: 3000,
      maxReconnectAttempts: 10,
      heartbeatInterval: 30000,
      ...options,
    };
  }

  /**
   * Connect to the WebSocket server.
   */
  connect(): void {
    if (this.ws?.readyState === WebSocket.OPEN) {
      return;
    }

    try {
      this.ws = new WebSocket(this.options.url);
      this.setupEventHandlers();
    } catch (error) {
      this.handleError(error as Error);
    }
  }

  private setupEventHandlers(): void {
    if (!this.ws) return;

    this.ws.onopen = () => {
      this._connected = true;
      this._reconnecting = false;
      this.reconnectAttempts = 0;
      this.startHeartbeat();
      this.flushMessageQueue();
      this.emit("connected");
      this.options.onConnected?.();
    };

    this.ws.onmessage = (event) => {
      try {
        const message: WebSocketProgressMessage = JSON.parse(event.data);
        this.handleMessage(message);
      } catch (error) {
        console.error("[ProgressWebSocket] Failed to parse message:", error);
      }
    };

    this.ws.onclose = (event) => {
      this._connected = false;
      this.stopHeartbeat();
      this.emit("disconnected", { code: event.code, reason: event.reason });
      this.options.onDisconnected?.({ code: event.code, reason: event.reason });

      if (!event.wasClean) {
        this.attemptReconnect();
      }
    };

    this.ws.onerror = () => {
      this.handleError(new Error("WebSocket connection error"));
    };
  }

  private handleMessage(message: WebSocketProgressMessage): void {
    switch (message.type) {
      case "heartbeat":
      case "Heartbeat":
        // Connection is alive, no action needed
        break;
      case "Connected":
        // Backend connection confirmation
        console.log("[ProgressWebSocket] Backend confirmed connection");
        break;
      case "StatusSnapshot":
        // Full pipeline status snapshot
        this.emit("status_snapshot", message);
        this.options.onMessage?.(message);
        break;
      case "PdfPageProgress":
        // OODA-PERF-02: PDF page-by-page progress events
        this.emit("pdf_progress", message);
        this.options.onMessage?.(message);
        break;
      case "ingestion_started":
      case "stage_started":
      case "stage_progress":
      case "stage_completed":
      case "ingestion_completed":
      case "ingestion_failed":
      case "ChunkProgress":
        // SPEC-001/Objective-A: Chunk-level progress events for granular visibility
        this.emit("progress", message);
        this.options.onMessage?.(message);
        break;
      case "ChunkFailure":
        // SPEC-003: Chunk failure events for resilient extraction visibility
        this.emit("progress", message);
        this.options.onMessage?.(message);
        break;
      default:
        console.warn(
          "[ProgressWebSocket] Unknown message type:",
          (message as { type?: string }).type,
        );
    }
  }

  private startHeartbeat(): void {
    this.stopHeartbeat();
    this.heartbeatTimer = setInterval(() => {
      this.send({ type: "ping", client_time: new Date().toISOString() });
    }, this.options.heartbeatInterval);
  }

  private stopHeartbeat(): void {
    if (this.heartbeatTimer) {
      clearInterval(this.heartbeatTimer);
      this.heartbeatTimer = undefined;
    }
  }

  private attemptReconnect(): void {
    if (this.reconnectAttempts >= this.options.maxReconnectAttempts) {
      this.emit("max_reconnects_reached");
      this.options.onMaxReconnectsReached?.();
      return;
    }

    this._reconnecting = true;
    this.reconnectAttempts++;

    const delay =
      this.options.reconnectInterval * Math.pow(2, this.reconnectAttempts - 1);

    this.emit("reconnecting", this.reconnectAttempts);
    this.options.onReconnecting?.(this.reconnectAttempts);

    this.reconnectTimer = setTimeout(() => {
      this.connect();
    }, delay);
  }

  private handleError(error: Error): void {
    // Only log to console in development, and use warn instead of error for connection issues
    // This is expected when backend is not running
    if (process.env.NODE_ENV === "development") {
      console.warn(
        "[ProgressWebSocket] Connection unavailable - backend may not be running",
      );
    }
    this.emit("error", error);
    this.options.onError?.(error);
  }

  private flushMessageQueue(): void {
    while (this.messageQueue.length > 0) {
      const message = this.messageQueue.shift();
      if (message) {
        this.send(message);
      }
    }
  }

  /**
   * Send a command to the server.
   */
  send(command: ClientCommand): void {
    if (this.ws?.readyState === WebSocket.OPEN) {
      this.ws.send(JSON.stringify(command));
    } else {
      // Queue for later
      this.messageQueue.push(command);
    }
  }

  /**
   * Subscribe to ingestion progress updates for specific track IDs.
   */
  subscribe(trackIds: string[]): void {
    this.send({ type: "subscribe", track_ids: trackIds });
  }

  /**
   * Unsubscribe from ingestion progress updates.
   */
  unsubscribe(trackIds: string[]): void {
    this.send({ type: "unsubscribe", track_ids: trackIds });
  }

  /**
   * Request cancellation of an ingestion job.
   */
  cancel(trackId: string): void {
    this.send({ type: "cancel", track_id: trackId });
  }

  /**
   * Disconnect from the WebSocket server.
   */
  disconnect(): void {
    this.stopHeartbeat();
    if (this.reconnectTimer) {
      clearTimeout(this.reconnectTimer);
      this.reconnectTimer = undefined;
    }
    if (this.ws) {
      this.ws.close(1000, "Client disconnect");
      this.ws = null;
    }
    this._connected = false;
    this._reconnecting = false;
    this.reconnectAttempts = 0;
    this.messageQueue = [];
  }

  /**
   * Add an event listener.
   */
  on(event: WebSocketEventType, callback: WebSocketEventCallback): () => void {
    if (!this.listeners.has(event)) {
      this.listeners.set(event, new Set());
    }
    this.listeners.get(event)!.add(callback);

    // Return unsubscribe function
    return () => {
      this.listeners.get(event)?.delete(callback);
    };
  }

  /**
   * Remove an event listener.
   */
  off(event: WebSocketEventType, callback: WebSocketEventCallback): void {
    this.listeners.get(event)?.delete(callback);
  }

  private emit(event: WebSocketEventType, ...args: unknown[]): void {
    this.listeners.get(event)?.forEach((callback) => {
      try {
        callback(...args);
      } catch (error) {
        console.error("[ProgressWebSocket] Error in event handler:", error);
      }
    });
  }
}
