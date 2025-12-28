# WebUI Specification: WebSocket Real-Time Progress

> Document ID: WEBUI-005
> Version: 1.0
> Created: 2024-12-28
> Status: SPECIFICATION

---

## Table of Contents

1. [Overview](#1-overview)
2. [WebSocket Protocol](#2-websocket-protocol)
3. [Client Implementation](#3-client-implementation)
4. [State Management](#4-state-management)
5. [React Hooks](#5-react-hooks)
6. [UI Integration](#6-ui-integration)
7. [Error Handling & Recovery](#7-error-handling--recovery)
8. [Testing Strategy](#8-testing-strategy)

---

## 1. Overview

### 1.1 Purpose

This document specifies the WebSocket-based real-time progress tracking system for the EdgeQuake WebUI. It enables live updates of ingestion progress without polling, providing immediate feedback to users.

### 1.2 Key Flows

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                     WEBSOCKET COMMUNICATION FLOW                            │
└─────────────────────────────────────────────────────────────────────────────┘

    Client                          Server (API)                Pipeline
       │                                │                           │
       │ 1. POST /api/v1/documents/upload                          │
       │ ──────────────────────────────▶│                          │
       │ ◀────────── 202 + track_id ────│                          │
       │                                │ 2. Queue task            │
       │                                │ ────────────────────────▶│
       │ 3. WS CONNECT /ws/progress?track_id=X                     │
       │ ══════════════════════════════▶│                          │
       │                                │                           │
       │                                │◀──── progress events ────│
       │ ◀══════ stage_start ══════════│                          │
       │                                │                           │
       │ ◀══════ stage_progress ═══════│◀──── chunk complete ────│
       │                                │                           │
       │ ◀══════ stage_complete ═══════│◀──── stage done ────────│
       │                                │                           │
       │ ◀══════ cost_update ══════════│◀──── LLM call done ─────│
       │                                │                           │
       │ ◀════════ complete ═══════════│◀──── ingestion done ────│
       │                                │                           │
       │ WS CLOSE                       │                           │
       └────────────────────────────────└───────────────────────────┘
```

### 1.3 Requirements

| Requirement | Description                                    |
| ----------- | ---------------------------------------------- |
| REQ-WS-001  | WebSocket connection with auto-reconnect       |
| REQ-WS-002  | Fallback to polling when WebSocket unavailable |
| REQ-WS-003  | Multi-track subscription support               |
| REQ-WS-004  | Connection state indicators in UI              |
| REQ-WS-005  | Graceful handling of disconnections            |
| REQ-WS-006  | Memory-efficient message buffering             |

---

## 2. WebSocket Protocol

### 2.1 Connection URL

```
ws://{host}/api/v1/ws/progress?tenant={tenant}&track_id={track_id}

Examples:
- Single track: ws://localhost:9621/api/v1/ws/progress?tenant=default&track_id=abc123
- Multiple tracks: ws://localhost:9621/api/v1/ws/progress?tenant=default&track_id=abc123,def456
- All (admin): ws://localhost:9621/api/v1/ws/progress?tenant=default
```

### 2.2 Message Types

#### 2.2.1 Server → Client Messages

```typescript
// Union type for all server messages
type WebSocketMessage =
  | IngestionStarted
  | StageStarted
  | StageProgress
  | StageCompleted
  | CostUpdate
  | IngestionCompleted
  | IngestionFailed
  | Heartbeat;

interface IngestionStarted {
  type: "ingestion_started";
  track_id: string;
  document_id: string;
  document_name: string;
  started_at: string;
  estimated_duration_ms?: number;
}

interface StageStarted {
  type: "stage_started";
  track_id: string;
  stage: IngestionStage;
  started_at: string;
}

interface StageProgress {
  type: "stage_progress";
  track_id: string;
  stage: IngestionStage;
  progress: number; // 0-100
  message?: string;
  current_item?: number;
  total_items?: number;
}

interface StageCompleted {
  type: "stage_completed";
  track_id: string;
  stage: IngestionStage;
  completed_at: string;
  duration_ms: number;
  result?: {
    chunks_created?: number;
    entities_extracted?: number;
    relationships_created?: number;
  };
}

interface CostUpdate {
  type: "cost_update";
  track_id: string;
  stage: IngestionStage;
  operation: string;
  cost_usd: number;
  tokens_used?: {
    input: number;
    output: number;
  };
  cumulative_cost_usd: number;
}

interface IngestionCompleted {
  type: "ingestion_completed";
  track_id: string;
  document_id: string;
  completed_at: string;
  total_duration_ms: number;
  summary: {
    chunks: number;
    entities: number;
    relationships: number;
    total_cost_usd: number;
  };
}

interface IngestionFailed {
  type: "ingestion_failed";
  track_id: string;
  document_id?: string;
  stage: IngestionStage;
  error: {
    code: string;
    message: string;
    recoverable: boolean;
    retry_after_ms?: number;
  };
  failed_at: string;
}

interface Heartbeat {
  type: "heartbeat";
  timestamp: string;
  server_time: string;
}
```

#### 2.2.2 Client → Server Messages

```typescript
// Client commands
type ClientMessage = Subscribe | Unsubscribe | CancelIngestion | Ping;

interface Subscribe {
  type: "subscribe";
  track_ids: string[];
}

interface Unsubscribe {
  type: "unsubscribe";
  track_ids: string[];
}

interface CancelIngestion {
  type: "cancel";
  track_id: string;
}

interface Ping {
  type: "ping";
  client_time: string;
}
```

### 2.3 Message Sequence

```
┌────────────────────────────────────────────────────────────────────────────┐
│                    TYPICAL INGESTION MESSAGE SEQUENCE                       │
└────────────────────────────────────────────────────────────────────────────┘

Time  │ Message Type         │ Data
──────┼──────────────────────┼────────────────────────────────────────────────
t+0   │ ingestion_started    │ track=X, doc="report.pdf", est=45s
t+1   │ stage_started        │ stage=preprocessing
t+3   │ stage_completed      │ stage=preprocessing, duration=2000ms
t+3   │ stage_started        │ stage=chunking
t+5   │ stage_completed      │ stage=chunking, chunks=10
t+5   │ stage_started        │ stage=extracting
t+7   │ stage_progress       │ stage=extracting, progress=10, chunk 1/10
t+8   │ cost_update          │ stage=extracting, cost=$0.0012, cum=$0.0012
t+9   │ stage_progress       │ stage=extracting, progress=20, chunk 2/10
...   │ ...                  │ ...
t+25  │ stage_progress       │ stage=extracting, progress=100, chunk 10/10
t+25  │ stage_completed      │ stage=extracting, entities=28
t+26  │ stage_started        │ stage=merging
...   │ ...                  │ ...
t+45  │ ingestion_completed  │ chunks=10, entities=18, cost=$0.0045
```

---

## 3. Client Implementation

### 3.1 WebSocket Client Class

```typescript
// src/lib/websocket/progress-websocket.ts

import { EventEmitter } from "events";

export interface WebSocketClientOptions {
  url: string;
  reconnectInterval?: number;
  maxReconnectAttempts?: number;
  heartbeatInterval?: number;
}

export class ProgressWebSocket extends EventEmitter {
  private ws: WebSocket | null = null;
  private reconnectAttempts = 0;
  private heartbeatTimer?: NodeJS.Timer;
  private reconnectTimer?: NodeJS.Timer;

  public readonly options: Required<WebSocketClientOptions>;
  public connected = false;
  public reconnecting = false;

  constructor(options: WebSocketClientOptions) {
    super();
    this.options = {
      reconnectInterval: 3000,
      maxReconnectAttempts: 10,
      heartbeatInterval: 30000,
      ...options,
    };
  }

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
      this.connected = true;
      this.reconnecting = false;
      this.reconnectAttempts = 0;
      this.startHeartbeat();
      this.emit("connected");
    };

    this.ws.onmessage = (event) => {
      try {
        const message: WebSocketMessage = JSON.parse(event.data);
        this.handleMessage(message);
      } catch (error) {
        console.error("Failed to parse WebSocket message:", error);
      }
    };

    this.ws.onclose = (event) => {
      this.connected = false;
      this.stopHeartbeat();
      this.emit("disconnected", { code: event.code, reason: event.reason });

      if (!event.wasClean) {
        this.attemptReconnect();
      }
    };

    this.ws.onerror = (error) => {
      this.handleError(new Error("WebSocket error"));
    };
  }

  private handleMessage(message: WebSocketMessage): void {
    switch (message.type) {
      case "heartbeat":
        // No action needed, just confirms connection is alive
        break;
      case "ingestion_started":
      case "stage_started":
      case "stage_progress":
      case "stage_completed":
      case "cost_update":
      case "ingestion_completed":
      case "ingestion_failed":
        this.emit("progress", message);
        break;
      default:
        console.warn("Unknown message type:", (message as any).type);
    }
  }

  private startHeartbeat(): void {
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
      return;
    }

    this.reconnecting = true;
    this.reconnectAttempts++;

    this.emit("reconnecting", { attempt: this.reconnectAttempts });

    this.reconnectTimer = setTimeout(() => {
      this.connect();
    }, this.options.reconnectInterval * Math.pow(2, this.reconnectAttempts - 1));
  }

  private handleError(error: Error): void {
    this.emit("error", error);
  }

  send(message: ClientMessage): void {
    if (this.ws?.readyState === WebSocket.OPEN) {
      this.ws.send(JSON.stringify(message));
    } else {
      console.warn("WebSocket not connected, queuing message");
      // Could implement message queue here
    }
  }

  subscribe(trackIds: string[]): void {
    this.send({ type: "subscribe", track_ids: trackIds });
  }

  unsubscribe(trackIds: string[]): void {
    this.send({ type: "unsubscribe", track_ids: trackIds });
  }

  cancel(trackId: string): void {
    this.send({ type: "cancel", track_id: trackId });
  }

  disconnect(): void {
    this.stopHeartbeat();
    if (this.reconnectTimer) {
      clearTimeout(this.reconnectTimer);
    }
    this.ws?.close();
    this.ws = null;
    this.connected = false;
  }
}
```

### 3.2 Singleton Manager

```typescript
// src/lib/websocket/websocket-manager.ts

import { ProgressWebSocket } from "./progress-websocket";
import { getWebSocketUrl } from "../config";

let instance: ProgressWebSocket | null = null;

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

export function disconnectWebSocket(): void {
  instance?.disconnect();
  instance = null;
}
```

---

## 4. State Management

### 4.1 Ingestion Store

```typescript
// src/lib/stores/use-ingestion-store.ts

import { create } from "zustand";
import { immer } from "zustand/middleware/immer";
import type {
  WebSocketMessage,
  IngestionProgress,
  IngestionStage,
} from "@/types";

interface IngestionState {
  // Active ingestion tracks
  tracks: Map<string, IngestionProgress>;

  // WebSocket connection status
  wsConnected: boolean;
  wsReconnecting: boolean;

  // Actions
  startTracking: (
    trackId: string,
    documentId: string,
    documentName: string
  ) => void;
  updateProgress: (message: WebSocketMessage) => void;
  stopTracking: (trackId: string) => void;
  setWsStatus: (connected: boolean, reconnecting?: boolean) => void;
  clearCompleted: () => void;
}

const initialStageState = (): Record<IngestionStage, StageProgress> => ({
  preprocessing: { status: "pending", progress: 0 },
  chunking: { status: "pending", progress: 0 },
  extracting: { status: "pending", progress: 0 },
  merging: { status: "pending", progress: 0 },
  indexing: { status: "pending", progress: 0 },
});

export const useIngestionStore = create<IngestionState>()(
  immer((set, get) => ({
    tracks: new Map(),
    wsConnected: false,
    wsReconnecting: false,

    startTracking: (trackId, documentId, documentName) => {
      set((state) => {
        state.tracks.set(trackId, {
          track_id: trackId,
          document_id: documentId,
          document_name: documentName,
          status: "queued",
          overall_progress: 0,
          stages: initialStageState(),
          cost_usd: 0,
          started_at: new Date().toISOString(),
        });
      });
    },

    updateProgress: (message) => {
      set((state) => {
        const track = state.tracks.get(message.track_id);
        if (!track) return;

        switch (message.type) {
          case "ingestion_started":
            track.status = "processing";
            track.started_at = message.started_at;
            if (message.estimated_duration_ms) {
              track.estimated_completion = new Date(
                Date.now() + message.estimated_duration_ms
              ).toISOString();
            }
            break;

          case "stage_started":
            track.current_stage = message.stage;
            track.stages[message.stage].status = "running";
            break;

          case "stage_progress":
            track.stages[message.stage].progress = message.progress;
            track.stages[message.stage].message = message.message;
            track.overall_progress = calculateOverallProgress(track.stages);
            break;

          case "stage_completed":
            track.stages[message.stage].status = "completed";
            track.stages[message.stage].progress = 100;
            track.stages[message.stage].duration_ms = message.duration_ms;
            break;

          case "cost_update":
            track.cost_usd = message.cumulative_cost_usd;
            break;

          case "ingestion_completed":
            track.status = "completed";
            track.overall_progress = 100;
            track.completed_at = message.completed_at;
            track.summary = message.summary;
            break;

          case "ingestion_failed":
            track.status = "failed";
            track.stages[message.stage].status = "failed";
            track.stages[message.stage].error = message.error;
            track.error = message.error;
            break;
        }
      });
    },

    stopTracking: (trackId) => {
      set((state) => {
        state.tracks.delete(trackId);
      });
    },

    setWsStatus: (connected, reconnecting = false) => {
      set((state) => {
        state.wsConnected = connected;
        state.wsReconnecting = reconnecting;
      });
    },

    clearCompleted: () => {
      set((state) => {
        for (const [id, track] of state.tracks) {
          if (track.status === "completed" || track.status === "failed") {
            state.tracks.delete(id);
          }
        }
      });
    },
  }))
);

function calculateOverallProgress(
  stages: Record<IngestionStage, StageProgress>
): number {
  const weights: Record<IngestionStage, number> = {
    preprocessing: 5,
    chunking: 10,
    extracting: 60,
    merging: 15,
    indexing: 10,
  };

  let totalWeight = 0;
  let weightedProgress = 0;

  for (const [stage, data] of Object.entries(stages)) {
    const weight = weights[stage as IngestionStage];
    totalWeight += weight;
    weightedProgress += (data.progress / 100) * weight;
  }

  return Math.round((weightedProgress / totalWeight) * 100);
}
```

---

## 5. React Hooks

### 5.1 useWebSocket Hook

```typescript
// src/lib/hooks/use-websocket.ts

import { useEffect, useCallback } from "react";
import {
  getWebSocketClient,
  disconnectWebSocket,
} from "../websocket/websocket-manager";
import { useIngestionStore } from "../stores/use-ingestion-store";

export function useWebSocket() {
  const { setWsStatus, updateProgress, wsConnected, wsReconnecting } =
    useIngestionStore();

  useEffect(() => {
    const client = getWebSocketClient();

    const handleConnected = () => {
      setWsStatus(true, false);
    };

    const handleDisconnected = () => {
      setWsStatus(false, false);
    };

    const handleReconnecting = () => {
      setWsStatus(false, true);
    };

    const handleProgress = (message: WebSocketMessage) => {
      updateProgress(message);
    };

    client.on("connected", handleConnected);
    client.on("disconnected", handleDisconnected);
    client.on("reconnecting", handleReconnecting);
    client.on("progress", handleProgress);

    // Connect if not already connected
    if (!client.connected) {
      client.connect();
    }

    return () => {
      client.off("connected", handleConnected);
      client.off("disconnected", handleDisconnected);
      client.off("reconnecting", handleReconnecting);
      client.off("progress", handleProgress);
    };
  }, [setWsStatus, updateProgress]);

  const subscribe = useCallback((trackIds: string[]) => {
    getWebSocketClient().subscribe(trackIds);
  }, []);

  const unsubscribe = useCallback((trackIds: string[]) => {
    getWebSocketClient().unsubscribe(trackIds);
  }, []);

  const cancel = useCallback((trackId: string) => {
    getWebSocketClient().cancel(trackId);
  }, []);

  return {
    connected: wsConnected,
    reconnecting: wsReconnecting,
    subscribe,
    unsubscribe,
    cancel,
  };
}
```

### 5.2 useIngestionProgress Hook

```typescript
// src/lib/hooks/use-ingestion-progress.ts

import { useEffect } from "react";
import { useQuery } from "@tanstack/react-query";
import { useIngestionStore } from "../stores/use-ingestion-store";
import { useWebSocket } from "./use-websocket";
import { getIngestionProgress } from "../api/edgequake";

interface UseIngestionProgressOptions {
  pollingInterval?: number; // Fallback polling interval
  enablePolling?: boolean; // Force polling instead of WebSocket
}

export function useIngestionProgress(
  trackId: string,
  options: UseIngestionProgressOptions = {}
) {
  const { pollingInterval = 3000, enablePolling = false } = options;

  const { connected, subscribe, unsubscribe } = useWebSocket();
  const { tracks, startTracking } = useIngestionStore();

  // Subscribe to WebSocket updates
  useEffect(() => {
    if (connected && !enablePolling) {
      subscribe([trackId]);
      return () => unsubscribe([trackId]);
    }
  }, [connected, trackId, subscribe, unsubscribe, enablePolling]);

  // Fallback polling when WebSocket unavailable
  const shouldPoll = enablePolling || !connected;

  const {
    data: polledData,
    isLoading,
    error,
  } = useQuery({
    queryKey: ["ingestion-progress", trackId],
    queryFn: () => getIngestionProgress(trackId),
    enabled: shouldPoll,
    refetchInterval: shouldPoll ? pollingInterval : false,
    staleTime: 1000,
  });

  // Merge WebSocket and polled data
  const wsData = tracks.get(trackId);
  const progress = wsData ?? polledData ?? null;

  return {
    progress,
    isLive: connected && !enablePolling, // Using WebSocket
    isPolling: shouldPoll,
    isLoading,
    error,
  };
}
```

### 5.3 useBatchProgress Hook

```typescript
// src/lib/hooks/use-batch-progress.ts

import { useMemo } from "react";
import { useIngestionStore } from "../stores/use-ingestion-store";

export function useBatchProgress(trackIds: string[]) {
  const { tracks } = useIngestionStore();

  return useMemo(() => {
    const items = trackIds
      .map((id) => tracks.get(id))
      .filter((item): item is IngestionProgress => item !== undefined);

    const completed = items.filter((i) => i.status === "completed").length;
    const failed = items.filter((i) => i.status === "failed").length;
    const processing = items.filter((i) => i.status === "processing").length;

    const totalProgress = items.reduce((sum, i) => sum + i.overall_progress, 0);
    const averageProgress = items.length > 0 ? totalProgress / items.length : 0;

    const totalCost = items.reduce((sum, i) => sum + (i.cost_usd || 0), 0);

    return {
      items,
      total: trackIds.length,
      completed,
      failed,
      processing,
      progress: Math.round(averageProgress),
      totalCost,
      isComplete: completed + failed === trackIds.length,
    };
  }, [trackIds, tracks]);
}
```

---

## 6. UI Integration

### 6.1 WebSocket Provider

```typescript
// src/providers/websocket-provider.tsx

"use client";

import { useEffect, createContext, useContext, ReactNode } from "react";
import { useWebSocket } from "@/lib/hooks/use-websocket";

interface WebSocketContextValue {
  connected: boolean;
  reconnecting: boolean;
  subscribe: (trackIds: string[]) => void;
  unsubscribe: (trackIds: string[]) => void;
  cancel: (trackId: string) => void;
}

const WebSocketContext = createContext<WebSocketContextValue | null>(null);

export function WebSocketProvider({ children }: { children: ReactNode }) {
  const ws = useWebSocket();

  return (
    <WebSocketContext.Provider value={ws}>{children}</WebSocketContext.Provider>
  );
}

export function useWebSocketContext(): WebSocketContextValue {
  const ctx = useContext(WebSocketContext);
  if (!ctx) {
    throw new Error(
      "useWebSocketContext must be used within WebSocketProvider"
    );
  }
  return ctx;
}
```

### 6.2 App Layout Integration

```typescript
// src/app/layout.tsx

import { WebSocketProvider } from "@/providers/websocket-provider";

export default function RootLayout({ children }: { children: ReactNode }) {
  return (
    <html>
      <body>
        <Providers>
          <WebSocketProvider>
            <Header />
            <main>{children}</main>
            <WebSocketStatus /> {/* Global connection indicator */}
          </WebSocketProvider>
        </Providers>
      </body>
    </html>
  );
}
```

### 6.3 Progress Panel Usage

```typescript
// src/components/documents/ingestion-progress-panel.tsx

import { useIngestionProgress } from "@/lib/hooks/use-ingestion-progress";
import { StageIndicator } from "@/components/progress/stage-indicator";
import { CostBadge } from "./cost-badge";
import { EtaDisplay } from "@/components/progress/eta-display";

export function IngestionProgressPanel({
  trackId,
  documentName,
  onComplete,
  onCancel,
}: Props) {
  const { progress, isLive, error } = useIngestionProgress(trackId);

  useEffect(() => {
    if (progress?.status === "completed") {
      onComplete?.();
    }
  }, [progress?.status, onComplete]);

  if (!progress) {
    return <Skeleton className="h-48" />;
  }

  return (
    <Card>
      <CardHeader className="flex-row items-center justify-between">
        <div className="flex items-center gap-2">
          <h3 className="font-medium">{documentName}</h3>
          {isLive && <WebSocketStatus showLabel={false} />}
        </div>
        <div className="flex gap-2">
          <Button variant="outline" size="sm" onClick={onCancel}>
            Cancel
          </Button>
        </div>
      </CardHeader>

      <CardContent>
        <div className="space-y-4">
          {/* Overall progress */}
          <div>
            <div className="flex justify-between text-sm mb-1">
              <span>Overall Progress</span>
              <span>{progress.overall_progress}%</span>
            </div>
            <AnimatedProgress value={progress.overall_progress} />
          </div>

          {/* Stage breakdown */}
          <StageIndicator
            stages={Object.entries(progress.stages).map(([id, data]) => ({
              id,
              label: id.charAt(0).toUpperCase() + id.slice(1),
              status: data.status,
              progress: data.progress,
              duration: data.duration_ms,
              message: data.message,
            }))}
            currentStage={progress.current_stage || "preprocessing"}
            variant="vertical"
          />

          {/* Cost and ETA */}
          <div className="flex justify-between items-center">
            <CostBadge cost={progress.cost_usd || 0} />
            <EtaDisplay
              etaSeconds={progress.eta_seconds}
              startedAt={progress.started_at}
            />
          </div>
        </div>
      </CardContent>
    </Card>
  );
}
```

---

## 7. Error Handling & Recovery

### 7.1 Connection Recovery Flow

```
┌────────────────────────────────────────────────────────────────────────────┐
│                       CONNECTION RECOVERY FLOW                              │
└────────────────────────────────────────────────────────────────────────────┘

     ┌─────────────┐                ┌─────────────────────────────────────────┐
     │ CONNECTED   │──disconnect───▶│ DISCONNECTED                            │
     │   🟢         │                │   🔴                                     │
     └─────────────┘                └───────────────────┬─────────────────────┘
           ▲                                            │
           │                                            │ auto-reconnect
           │                                            ▼
           │                        ┌─────────────────────────────────────────┐
           │                        │ RECONNECTING (attempt N)                │
           │                        │   🟡  Exponential backoff               │
           │                        └───────────────────┬─────────────────────┘
           │                                            │
           │ success                      failure       │
           │◀────────────────────────────────┬──────────┘
           │                                 │
           │                                 │ N < max_attempts
           │                                 ▼
           │                        ┌─────────────────────────────────────────┐
           │                        │ Wait (3s * 2^N)                         │
           │                        │   Then retry...                         │
           │                        └─────────────────────────────────────────┘
           │
           │ N >= max_attempts
           │                        ┌─────────────────────────────────────────┐
           └────────────────────────│ FALLBACK TO POLLING                     │
                                    │   User can manually reconnect           │
                                    └─────────────────────────────────────────┘
```

### 7.2 Error Display Component

```typescript
// src/components/shared/connection-error.tsx

interface ConnectionErrorProps {
  onRetry: () => void;
  className?: string;
}

export function ConnectionError({ onRetry, className }: ConnectionErrorProps) {
  const { reconnecting, connected } = useWebSocket();

  if (connected) return null;

  return (
    <Alert variant="warning" className={className}>
      <AlertCircle className="h-4 w-4" />
      <AlertTitle>
        {reconnecting ? "Reconnecting..." : "Connection Lost"}
      </AlertTitle>
      <AlertDescription>
        {reconnecting
          ? "Attempting to reconnect to live updates..."
          : "Live updates are unavailable. Using polling fallback."}
      </AlertDescription>
      {!reconnecting && (
        <Button variant="outline" size="sm" onClick={onRetry}>
          Retry Connection
        </Button>
      )}
    </Alert>
  );
}
```

---

## 8. Testing Strategy

### 8.1 Mock WebSocket Server

```typescript
// src/lib/websocket/__mocks__/mock-websocket.ts

export class MockWebSocketServer {
  private clients: Set<MockWebSocketClient> = new Set();

  addClient(client: MockWebSocketClient): void {
    this.clients.add(client);
  }

  removeClient(client: MockWebSocketClient): void {
    this.clients.delete(client);
  }

  broadcast(message: WebSocketMessage): void {
    for (const client of this.clients) {
      client.receive(message);
    }
  }

  // Simulate progress for testing
  simulateIngestion(trackId: string, durationMs: number): void {
    const stages: IngestionStage[] = [
      "preprocessing",
      "chunking",
      "extracting",
      "merging",
      "indexing",
    ];
    const stageDelay = durationMs / stages.length;

    this.broadcast({
      type: "ingestion_started",
      track_id: trackId,
      document_id: "doc-1",
      document_name: "test.txt",
      started_at: new Date().toISOString(),
      estimated_duration_ms: durationMs,
    });

    stages.forEach((stage, index) => {
      setTimeout(() => {
        this.broadcast({
          type: "stage_started",
          track_id: trackId,
          stage,
          started_at: new Date().toISOString(),
        });

        // Simulate progress
        for (let p = 10; p <= 100; p += 10) {
          setTimeout(() => {
            this.broadcast({
              type: "stage_progress",
              track_id: trackId,
              stage,
              progress: p,
            });
          }, (p / 100) * (stageDelay - 100));
        }

        setTimeout(() => {
          this.broadcast({
            type: "stage_completed",
            track_id: trackId,
            stage,
            completed_at: new Date().toISOString(),
            duration_ms: stageDelay,
          });
        }, stageDelay - 50);
      }, index * stageDelay);
    });

    setTimeout(() => {
      this.broadcast({
        type: "ingestion_completed",
        track_id: trackId,
        document_id: "doc-1",
        completed_at: new Date().toISOString(),
        total_duration_ms: durationMs,
        summary: {
          chunks: 10,
          entities: 18,
          relationships: 12,
          total_cost_usd: 0.0045,
        },
      });
    }, durationMs);
  }
}
```

### 8.2 Component Tests

```typescript
// __tests__/components/ingestion-progress-panel.test.tsx

import { render, screen, waitFor } from "@testing-library/react";
import { IngestionProgressPanel } from "@/components/documents/ingestion-progress-panel";
import { MockWebSocketServer } from "@/lib/websocket/__mocks__/mock-websocket";

describe("IngestionProgressPanel", () => {
  let mockServer: MockWebSocketServer;

  beforeEach(() => {
    mockServer = new MockWebSocketServer();
    // Inject mock server
  });

  it("displays real-time progress updates", async () => {
    render(
      <IngestionProgressPanel trackId="test-track" documentName="test.txt" />
    );

    // Simulate ingestion
    mockServer.simulateIngestion("test-track", 5000);

    // Check initial state
    expect(screen.getByText("test.txt")).toBeInTheDocument();

    // Wait for progress
    await waitFor(() => {
      expect(screen.getByText(/preprocessing/i)).toBeInTheDocument();
    });

    // Wait for completion
    await waitFor(
      () => {
        expect(screen.getByText("100%")).toBeInTheDocument();
      },
      { timeout: 6000 }
    );
  });

  it("handles connection loss gracefully", async () => {
    // Simulate disconnect and reconnect
  });

  it("calls onComplete when ingestion finishes", async () => {
    const onComplete = vi.fn();

    render(
      <IngestionProgressPanel
        trackId="test-track"
        documentName="test.txt"
        onComplete={onComplete}
      />
    );

    mockServer.simulateIngestion("test-track", 1000);

    await waitFor(
      () => {
        expect(onComplete).toHaveBeenCalledTimes(1);
      },
      { timeout: 2000 }
    );
  });
});
```

---

## Appendix: Configuration

### Environment Variables

```bash
# .env.local
NEXT_PUBLIC_WS_URL=ws://localhost:9621/api/v1/ws
NEXT_PUBLIC_WS_RECONNECT_INTERVAL=3000
NEXT_PUBLIC_WS_MAX_RECONNECT_ATTEMPTS=10
NEXT_PUBLIC_WS_HEARTBEAT_INTERVAL=30000
```

### WebSocket URL Builder

```typescript
// src/lib/config.ts

export function getWebSocketUrl(): string {
  const baseUrl =
    process.env.NEXT_PUBLIC_WS_URL || "ws://localhost:9621/api/v1/ws";
  const tenant = getCurrentTenant();
  return `${baseUrl}/progress?tenant=${tenant}`;
}
```

---

_End of Document WEBUI-005_
