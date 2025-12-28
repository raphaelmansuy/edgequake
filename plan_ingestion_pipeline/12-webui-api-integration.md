# WebUI Specification: API Integration

> Document ID: WEBUI-003
> Version: 1.0
> Created: 2024-12-28
> Status: SPECIFICATION

---

## Table of Contents

1. [API Integration Overview](#1-api-integration-overview)
2. [New API Endpoints](#2-new-api-endpoints)
3. [TypeScript Types](#3-typescript-types)
4. [React Query Hooks](#4-react-query-hooks)
5. [API Client Updates](#5-api-client-updates)
6. [Error Handling](#6-error-handling)

---

## 1. API Integration Overview

### 1.1 API Architecture

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                         API INTEGRATION LAYER                               │
└─────────────────────────────────────────────────────────────────────────────┘

┌────────────────────────────────────────────────────────────────────────────┐
│                              REACT COMPONENTS                               │
└───────────────────────────────────────┬────────────────────────────────────┘
                                        │
                                        ▼
┌────────────────────────────────────────────────────────────────────────────┐
│                           REACT QUERY HOOKS                                 │
│  ┌────────────────┐ ┌────────────────┐ ┌────────────────┐ ┌──────────────┐ │
│  │ useDocuments   │ │ useIngestion   │ │ useLineage     │ │ useCost      │ │
│  │ useDocument    │ │ Progress       │ │ useChunks      │ │ useBudget    │ │
│  └────────────────┘ └────────────────┘ └────────────────┘ └──────────────┘ │
└───────────────────────────────────────┬────────────────────────────────────┘
                                        │
                    ┌───────────────────┴───────────────────┐
                    │                                       │
                    ▼                                       ▼
┌───────────────────────────────────┐   ┌───────────────────────────────────┐
│         REST API CLIENT           │   │       WEBSOCKET CLIENT            │
│  ┌─────────────────────────────┐  │   │  ┌─────────────────────────────┐  │
│  │ api.get, api.post, api.del  │  │   │  │ connect, subscribe, send   │  │
│  └─────────────────────────────┘  │   │  └─────────────────────────────┘  │
└───────────────────────────────────┘   └───────────────────────────────────┘
                    │                                       │
                    └───────────────────┬───────────────────┘
                                        │
                                        ▼
┌────────────────────────────────────────────────────────────────────────────┐
│                          EDGEQUAKE BACKEND                                  │
│  Base URL: /api/v1                                                         │
│  WebSocket: /api/v1/ws/progress                                            │
└────────────────────────────────────────────────────────────────────────────┘
```

### 1.2 Endpoint Categories

| Category  | Base Path                            | Purpose                          |
| --------- | ------------------------------------ | -------------------------------- |
| Documents | `/documents`                         | Document CRUD, upload, reprocess |
| Lineage   | `/documents/{id}/lineage`, `/chunks` | Lineage tracking                 |
| Progress  | `/documents/track/{id}`, WebSocket   | Real-time status                 |
| Cost      | `/costs`                             | Cost tracking and summaries      |
| Entities  | `/entities/{id}/provenance`          | Entity source tracking           |

---

## 2. New API Endpoints

### 2.1 Lineage Endpoints

```typescript
// GET /api/v1/documents/{id}/lineage
interface DocumentLineageResponse {
  document_id: string;
  document_name: string;
  job_id: string;
  ingestion_config: IngestionConfig;
  summary: {
    total_chunks: number;
    total_entities: number;
    total_relationships: number;
    deduplication_rate: number;
  };
  chunks: ChunkLineage[];
  entities: EntityLineageSummary[];
  relationships: RelationshipLineageSummary[];
  created_at: string;
}

interface ChunkLineage {
  chunk_id: string;
  chunk_index: number;
  start_line: number;
  end_line: number;
  start_offset: number;
  end_offset: number;
  token_count: number;
  entities: string[]; // Entity IDs
  relationships: string[]; // Relationship keys
  extraction_metadata: ExtractionMetadata;
}

interface EntityLineageSummary {
  entity_id: string;
  entity_name: string;
  entity_type: string;
  source_chunks: string[];
  first_seen_line: number;
  mention_count: number;
}

interface ExtractionMetadata {
  model: string;
  gleaning_iterations: number;
  extraction_time_ms: number;
  input_tokens: number;
  output_tokens: number;
  cache_hit: boolean;
}
```

### 2.2 Chunk Detail Endpoint

```typescript
// GET /api/v1/chunks/{chunk_id}
interface ChunkDetailResponse {
  chunk_id: string;
  document_id: string;
  document_name: string;
  content: string;
  position: {
    index: number;
    start_offset: number;
    end_offset: number;
    start_line: number;
    end_line: number;
  };
  token_count: number;
  entities: ExtractedEntityDetail[];
  relationships: ExtractedRelationshipDetail[];
  extraction_metadata: ExtractionMetadata;
}

interface ExtractedEntityDetail {
  id: string;
  name: string;
  type: string;
  description: string;
  importance: number;
  source_span: {
    start_offset: number;
    end_offset: number;
    text: string;
  };
}

interface ExtractedRelationshipDetail {
  source_id: string;
  source_name: string;
  target_id: string;
  target_name: string;
  relation_type: string;
  description: string;
  weight: number;
  keywords: string[];
}
```

### 2.3 Cost Endpoints

```typescript
// GET /api/v1/costs/{track_id}
interface IngestionCostResponse {
  track_id: string;
  document_id: string;
  total_cost_usd: number;
  breakdown: CostBreakdown;
  token_usage: TokenUsage;
  calculated_at: string;
}

interface CostBreakdown {
  extraction: OperationCost;
  gleaning: OperationCost;
  summarization: OperationCost;
  embedding: OperationCost;
}

interface OperationCost {
  api_calls: number;
  input_tokens: number;
  output_tokens: number;
  cost_usd: number;
  model: string;
}

interface TokenUsage {
  total_input_tokens: number;
  total_output_tokens: number;
  total_embedding_tokens: number;
  total_tokens: number;
}

// GET /api/v1/costs/summary?start_date=...&end_date=...&group_by=day
interface CostSummaryResponse {
  workspace_id: string;
  period: {
    start: string;
    end: string;
  };
  summary: {
    total_cost_usd: number;
    total_documents: number;
    total_tokens: number;
    average_cost_per_document: number;
  };
  breakdown_by_operation: {
    extraction: number;
    gleaning: number;
    summarization: number;
    embedding: number;
  };
  breakdown_by_period: PeriodCost[];
  budget: BudgetInfo | null;
}

interface PeriodCost {
  period: string; // ISO date
  cost_usd: number;
  documents: number;
}

interface BudgetInfo {
  monthly_budget_usd: number;
  spent_usd: number;
  remaining_usd: number;
  alert_threshold: number;
  is_over_budget: boolean;
}
```

### 2.4 Entity Provenance Endpoint

```typescript
// GET /api/v1/entities/{entity_id}/provenance
interface EntityProvenanceResponse {
  entity_id: string;
  entity_name: string;
  entity_type: string;
  description: string;
  sources: EntitySource[];
  total_extraction_count: number;
  description_history: DescriptionHistoryEntry[];
  related_entities: RelatedEntity[];
}

interface EntitySource {
  document_id: string;
  document_name: string;
  chunks: ChunkSource[];
  first_extracted_at: string;
}

interface ChunkSource {
  chunk_id: string;
  start_line: number;
  end_line: number;
  source_text: string; // Excerpt
}

interface DescriptionHistoryEntry {
  description: string;
  source: "extraction" | "merge" | "manual";
  created_at: string;
}

interface RelatedEntity {
  entity_id: string;
  entity_name: string;
  relationship_type: string;
  shared_documents: number;
}
```

### 2.5 Document Impact Analysis

```typescript
// GET /api/v1/documents/{id}/impact
interface DocumentImpactResponse {
  document_id: string;
  document_name: string;
  impact: {
    chunks_to_remove: number;
    entities_affected: EntityImpact[];
    relationships_affected: RelationshipImpact[];
    total_entities_to_update: number;
    total_entities_to_remove: number;
    total_relationships_to_remove: number;
  };
}

interface EntityImpact {
  entity_id: string;
  entity_name: string;
  other_source_count: number;
  action: "update" | "remove";
}

interface RelationshipImpact {
  relationship_id: string;
  source_name: string;
  target_name: string;
  other_source_count: number;
  action: "update" | "remove";
}
```

---

## 3. TypeScript Types

### 3.1 New Types File Updates

```typescript
// src/types/index.ts - ADD these types

// ============================================================================
// Lineage Types
// ============================================================================

export interface DocumentLineage {
  llm_model?: string;
  embedding_model?: string;
  embedding_dimensions?: number;
  keywords?: string[];
  entity_types?: string[];
  relationship_types?: string[];
  chunking_strategy?: string;
  avg_chunk_size?: number;
  processing_duration_ms?: number;
  // NEW fields
  chunk_count?: number;
  total_entities?: number;
  total_relationships?: number;
  deduplication_rate?: number;
}

export interface ChunkLineage {
  chunk_id: string;
  chunk_index: number;
  start_line: number;
  end_line: number;
  start_offset: number;
  end_offset: number;
  token_count: number;
  entities: string[];
  relationships: string[];
  extraction_metadata: ExtractionMetadata;
}

export interface ExtractionMetadata {
  model: string;
  gleaning_iterations: number;
  extraction_time_ms: number;
  input_tokens: number;
  output_tokens: number;
  cache_hit: boolean;
}

// ============================================================================
// Cost Types
// ============================================================================

export interface IngestionCost {
  track_id: string;
  document_id: string;
  total_cost_usd: number;
  breakdown: CostBreakdown;
  token_usage: TokenUsage;
  calculated_at: string;
}

export interface CostBreakdown {
  extraction: OperationCost;
  gleaning: OperationCost;
  summarization: OperationCost;
  embedding: OperationCost;
}

export interface OperationCost {
  api_calls: number;
  input_tokens: number;
  output_tokens: number;
  cost_usd: number;
  model: string;
}

export interface TokenUsage {
  total_input_tokens: number;
  total_output_tokens: number;
  total_embedding_tokens: number;
  total_tokens: number;
}

export interface CostSummary {
  workspace_id: string;
  period: { start: string; end: string };
  summary: {
    total_cost_usd: number;
    total_documents: number;
    total_tokens: number;
    average_cost_per_document: number;
  };
  breakdown_by_operation: Record<string, number>;
  breakdown_by_period: Array<{
    period: string;
    cost_usd: number;
    documents: number;
  }>;
  budget: BudgetInfo | null;
}

export interface BudgetInfo {
  monthly_budget_usd: number;
  spent_usd: number;
  remaining_usd: number;
  alert_threshold: number;
  is_over_budget: boolean;
}

// ============================================================================
// Progress Types (Enhanced)
// ============================================================================

export interface IngestionProgress {
  track_id: string;
  document_id: string;
  status: IngestionStatus;
  progress: ProgressDetail;
  cost?: IngestionCost;
  error?: IngestionError;
  started_at: string;
  updated_at: string;
  completed_at?: string;
}

export type IngestionStatus =
  | "pending"
  | "preprocessing"
  | "chunking"
  | "extracting"
  | "merging"
  | "embedding"
  | "indexing"
  | "completed"
  | "failed"
  | "cancelled";

export interface ProgressDetail {
  current_stage: IngestionStatus;
  completion_percentage: number;
  eta_seconds?: number;
  latest_message: string;
  stages: StageProgress[];
}

export interface StageProgress {
  stage: IngestionStatus;
  status: "pending" | "running" | "completed" | "failed";
  total_items: number;
  completed_items: number;
  started_at?: string;
  completed_at?: string;
  message?: string;
}

export interface IngestionError {
  code: string;
  message: string;
  stage: string;
  reason: string;
  suggestion: string;
  recoverable: boolean;
  partial_result?: {
    chunks_processed: number;
    entities_extracted: number;
    relationships_found: number;
  };
}

// ============================================================================
// WebSocket Types
// ============================================================================

export type WebSocketMessage =
  | { type: "auth"; token: string }
  | { type: "auth_ok" }
  | { type: "subscribe"; channel: "ingestion"; track_id: string }
  | { type: "subscribed"; channel: string; track_id: string }
  | { type: "unsubscribe"; channel: string; track_id: string }
  | {
      type: "progress";
      track_id: string;
      stage: string;
      completion_percentage: number;
      message: string;
      timestamp: string;
    }
  | {
      type: "cost_update";
      track_id: string;
      cost: Partial<IngestionCost>;
      timestamp: string;
    }
  | {
      type: "stage_completed";
      track_id: string;
      stage: string;
      result: Record<string, unknown>;
      next_stage?: string;
      timestamp: string;
    }
  | {
      type: "completed";
      track_id: string;
      document_id: string;
      result: Record<string, unknown>;
      cost: IngestionCost;
      timestamp: string;
    }
  | {
      type: "error";
      track_id: string;
      error: IngestionError;
      timestamp: string;
    };

export interface WebSocketState {
  connected: boolean;
  reconnecting: boolean;
  subscriptions: Set<string>;
  error?: string;
}
```

---

## 4. React Query Hooks

### 4.1 Lineage Hooks

```typescript
// src/hooks/use-lineage.ts

import { useQuery } from "@tanstack/react-query";
import {
  getDocumentLineage,
  getChunkDetail,
  getEntityProvenance,
} from "@/lib/api/edgequake";
import { queryKeys } from "@/lib/api/query-keys";

export function useDocumentLineage(documentId: string) {
  return useQuery({
    queryKey: queryKeys.lineage.document(documentId),
    queryFn: () => getDocumentLineage(documentId),
    enabled: !!documentId,
    staleTime: 5 * 60 * 1000, // 5 minutes
  });
}

export function useChunkDetail(chunkId: string) {
  return useQuery({
    queryKey: queryKeys.lineage.chunk(chunkId),
    queryFn: () => getChunkDetail(chunkId),
    enabled: !!chunkId,
    staleTime: 10 * 60 * 1000, // 10 minutes (chunks don't change)
  });
}

export function useEntityProvenance(entityId: string) {
  return useQuery({
    queryKey: queryKeys.entities.provenance(entityId),
    queryFn: () => getEntityProvenance(entityId),
    enabled: !!entityId,
    staleTime: 5 * 60 * 1000,
  });
}

export function useDocumentImpact(documentId: string, enabled = false) {
  return useQuery({
    queryKey: queryKeys.documents.impact(documentId),
    queryFn: () => getDocumentImpact(documentId),
    enabled: enabled && !!documentId,
    staleTime: 30 * 1000, // 30 seconds (may change with other ingestions)
  });
}
```

### 4.2 Cost Hooks

```typescript
// src/hooks/use-cost.ts

import { useQuery } from "@tanstack/react-query";
import { getIngestionCost, getCostSummary } from "@/lib/api/edgequake";
import { queryKeys } from "@/lib/api/query-keys";

export function useIngestionCost(trackId: string) {
  return useQuery({
    queryKey: queryKeys.costs.ingestion(trackId),
    queryFn: () => getIngestionCost(trackId),
    enabled: !!trackId,
    staleTime: 60 * 1000, // 1 minute
  });
}

export function useCostSummary(options?: {
  startDate?: string;
  endDate?: string;
  groupBy?: "day" | "week" | "month";
}) {
  return useQuery({
    queryKey: queryKeys.costs.summary(options),
    queryFn: () => getCostSummary(options),
    staleTime: 5 * 60 * 1000, // 5 minutes
  });
}

export function useBudgetStatus() {
  const { data: summary } = useCostSummary();

  return {
    budget: summary?.budget ?? null,
    isOverBudget: summary?.budget?.is_over_budget ?? false,
    remainingBudget: summary?.budget?.remaining_usd ?? 0,
    spentPercent: summary?.budget
      ? (summary.budget.spent_usd / summary.budget.monthly_budget_usd) * 100
      : 0,
  };
}
```

### 4.3 Progress Hooks

```typescript
// src/hooks/use-ingestion-progress.ts

import { useEffect, useCallback } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { getTrackStatus } from "@/lib/api/edgequake";
import { useIngestionStore } from "@/stores/use-ingestion-store";
import { useWebSocket } from "./use-websocket";
import { queryKeys } from "@/lib/api/query-keys";
import type { IngestionProgress, WebSocketMessage } from "@/types";

export function useIngestionProgress(trackId: string) {
  const queryClient = useQueryClient();
  const { activeJobs, updateProgress, completeJob, failJob } =
    useIngestionStore();
  const { subscribe, unsubscribe, connected } = useWebSocket();

  // Fallback polling when WebSocket not available
  const { data: polledStatus } = useQuery({
    queryKey: queryKeys.documents.track(trackId),
    queryFn: () => getTrackStatus(trackId),
    enabled: !!trackId && !connected,
    refetchInterval: (query) => {
      const data = query.state.data;
      if (data?.is_complete) return false;
      return 2000; // Poll every 2 seconds
    },
  });

  // WebSocket subscription
  useEffect(() => {
    if (!trackId || !connected) return;

    const handleMessage = (message: WebSocketMessage) => {
      if (message.type === "progress" && message.track_id === trackId) {
        updateProgress(trackId, {
          stage: message.stage,
          percentage: message.completion_percentage,
          message: message.message,
        });
      } else if (message.type === "completed" && message.track_id === trackId) {
        completeJob(trackId, message.result);
        queryClient.invalidateQueries({ queryKey: queryKeys.documents.list() });
      } else if (message.type === "error" && message.track_id === trackId) {
        failJob(trackId, message.error);
      }
    };

    subscribe("ingestion", trackId, handleMessage);
    return () => unsubscribe("ingestion", trackId);
  }, [
    trackId,
    connected,
    subscribe,
    unsubscribe,
    updateProgress,
    completeJob,
    failJob,
    queryClient,
  ]);

  // Return real-time data from store or polled data as fallback
  const liveProgress = activeJobs.get(trackId);

  return {
    progress: liveProgress ?? polledStatus,
    isLive: connected && !!liveProgress,
    isComplete:
      polledStatus?.is_complete ?? liveProgress?.status === "completed",
  };
}

export function useActiveIngestions() {
  const { activeJobs } = useIngestionStore();
  return Array.from(activeJobs.values());
}
```

### 4.4 Query Keys Update

```typescript
// src/lib/api/query-keys.ts - ADD these keys

export const queryKeys = {
  // ... existing keys ...

  lineage: {
    document: (id: string) => ["lineage", "document", id] as const,
    chunk: (id: string) => ["lineage", "chunk", id] as const,
    chunks: (docId: string) => ["lineage", "chunks", docId] as const,
  },

  costs: {
    ingestion: (trackId: string) => ["costs", "ingestion", trackId] as const,
    summary: (options?: Record<string, unknown>) =>
      ["costs", "summary", options] as const,
    document: (docId: string) => ["costs", "document", docId] as const,
  },

  entities: {
    // ... existing keys ...
    provenance: (id: string) => ["entities", id, "provenance"] as const,
  },

  documents: {
    // ... existing keys ...
    track: (trackId: string) => ["documents", "track", trackId] as const,
    impact: (id: string) => ["documents", id, "impact"] as const,
  },
};
```

---

## 5. API Client Updates

### 5.1 New API Functions

```typescript
// src/lib/api/edgequake.ts - ADD these functions

// ============================================================================
// Lineage
// ============================================================================

export async function getDocumentLineage(
  documentId: string
): Promise<DocumentLineageResponse> {
  return api.get<DocumentLineageResponse>(`/documents/${documentId}/lineage`);
}

export async function getChunkDetail(
  chunkId: string
): Promise<ChunkDetailResponse> {
  return api.get<ChunkDetailResponse>(`/chunks/${chunkId}`);
}

export async function getEntityProvenance(
  entityId: string
): Promise<EntityProvenanceResponse> {
  return api.get<EntityProvenanceResponse>(`/entities/${entityId}/provenance`);
}

export async function getDocumentImpact(
  documentId: string
): Promise<DocumentImpactResponse> {
  return api.get<DocumentImpactResponse>(`/documents/${documentId}/impact`);
}

// ============================================================================
// Costs
// ============================================================================

export async function getIngestionCost(
  trackId: string
): Promise<IngestionCostResponse> {
  return api.get<IngestionCostResponse>(`/costs/${trackId}`);
}

export async function getCostSummary(options?: {
  startDate?: string;
  endDate?: string;
  groupBy?: "day" | "week" | "month";
}): Promise<CostSummaryResponse> {
  const params = new URLSearchParams();
  if (options?.startDate) params.set("start_date", options.startDate);
  if (options?.endDate) params.set("end_date", options.endDate);
  if (options?.groupBy) params.set("group_by", options.groupBy);

  const query = params.toString();
  return api.get<CostSummaryResponse>(
    `/costs/summary${query ? `?${query}` : ""}`
  );
}

// ============================================================================
// Ingestion Control
// ============================================================================

export async function cancelIngestion(
  trackId: string
): Promise<{ message: string }> {
  return api.post<{ message: string }>(`/tasks/${trackId}/cancel`);
}

export async function retryIngestion(
  trackId: string,
  options?: {
    configOverrides?: Partial<IngestionConfig>;
  }
): Promise<{ track_id: string; message: string }> {
  return api.post<{ track_id: string; message: string }>(
    `/tasks/${trackId}/retry`,
    options
  );
}
```

### 5.2 WebSocket Client

```typescript
// src/lib/api/websocket.ts (NEW)

import type { WebSocketMessage, WebSocketState } from "@/types";

type MessageHandler = (message: WebSocketMessage) => void;

class WebSocketClient {
  private ws: WebSocket | null = null;
  private url: string;
  private token: string | null = null;
  private reconnectAttempts = 0;
  private maxReconnectAttempts = 5;
  private reconnectDelay = 1000;
  private messageHandlers = new Map<string, Set<MessageHandler>>();
  private state: WebSocketState = {
    connected: false,
    reconnecting: false,
    subscriptions: new Set(),
  };
  private stateListeners = new Set<(state: WebSocketState) => void>();

  constructor(baseUrl: string) {
    const wsUrl = baseUrl.replace(/^http/, "ws");
    this.url = `${wsUrl}/api/v1/ws/progress`;
  }

  connect(token: string): Promise<void> {
    this.token = token;
    return new Promise((resolve, reject) => {
      try {
        this.ws = new WebSocket(this.url);

        this.ws.onopen = () => {
          this.send({ type: "auth", token: this.token! });
        };

        this.ws.onmessage = (event) => {
          const message = JSON.parse(event.data) as WebSocketMessage;

          if (message.type === "auth_ok") {
            this.updateState({ connected: true, reconnecting: false });
            this.reconnectAttempts = 0;
            // Resubscribe to previous subscriptions
            this.state.subscriptions.forEach((trackId) => {
              this.send({
                type: "subscribe",
                channel: "ingestion",
                track_id: trackId,
              });
            });
            resolve();
          } else {
            this.handleMessage(message);
          }
        };

        this.ws.onclose = () => {
          this.updateState({ connected: false });
          this.attemptReconnect();
        };

        this.ws.onerror = (error) => {
          console.error("WebSocket error:", error);
          reject(error);
        };
      } catch (error) {
        reject(error);
      }
    });
  }

  private attemptReconnect() {
    if (this.reconnectAttempts >= this.maxReconnectAttempts || !this.token) {
      this.updateState({
        reconnecting: false,
        error: "Max reconnect attempts reached",
      });
      return;
    }

    this.updateState({ reconnecting: true });
    this.reconnectAttempts++;

    setTimeout(() => {
      this.connect(this.token!).catch(() => {
        this.attemptReconnect();
      });
    }, this.reconnectDelay * this.reconnectAttempts);
  }

  subscribe(channel: string, trackId: string, handler: MessageHandler) {
    const key = `${channel}:${trackId}`;

    if (!this.messageHandlers.has(key)) {
      this.messageHandlers.set(key, new Set());
    }
    this.messageHandlers.get(key)!.add(handler);

    this.state.subscriptions.add(trackId);

    if (this.state.connected) {
      this.send({ type: "subscribe", channel, track_id: trackId });
    }
  }

  unsubscribe(channel: string, trackId: string) {
    const key = `${channel}:${trackId}`;
    this.messageHandlers.delete(key);
    this.state.subscriptions.delete(trackId);

    if (this.state.connected) {
      this.send({ type: "unsubscribe", channel, track_id: trackId });
    }
  }

  private handleMessage(message: WebSocketMessage) {
    if ("track_id" in message) {
      const key = `ingestion:${message.track_id}`;
      const handlers = this.messageHandlers.get(key);
      handlers?.forEach((handler) => handler(message));
    }

    // Also notify global handlers
    const globalHandlers = this.messageHandlers.get("*");
    globalHandlers?.forEach((handler) => handler(message));
  }

  private send(message: object) {
    if (this.ws?.readyState === WebSocket.OPEN) {
      this.ws.send(JSON.stringify(message));
    }
  }

  private updateState(partial: Partial<WebSocketState>) {
    this.state = { ...this.state, ...partial };
    this.stateListeners.forEach((listener) => listener(this.state));
  }

  onStateChange(listener: (state: WebSocketState) => void) {
    this.stateListeners.add(listener);
    return () => this.stateListeners.delete(listener);
  }

  getState(): WebSocketState {
    return this.state;
  }

  disconnect() {
    this.ws?.close();
    this.ws = null;
    this.updateState({ connected: false, reconnecting: false });
  }
}

// Singleton instance
let wsClient: WebSocketClient | null = null;

export function getWebSocketClient(): WebSocketClient {
  if (!wsClient) {
    const baseUrl = process.env.NEXT_PUBLIC_API_URL || "";
    wsClient = new WebSocketClient(baseUrl);
  }
  return wsClient;
}

export function useWebSocket() {
  const client = getWebSocketClient();
  // ... React hook implementation
  return {
    connect: client.connect.bind(client),
    subscribe: client.subscribe.bind(client),
    unsubscribe: client.unsubscribe.bind(client),
    disconnect: client.disconnect.bind(client),
    connected: client.getState().connected,
  };
}
```

---

## 6. Error Handling

### 6.1 Error Types

```typescript
// src/lib/api/errors.ts

export class ApiError extends Error {
  constructor(
    public code: string,
    message: string,
    public status: number,
    public details?: Record<string, unknown>
  ) {
    super(message);
    this.name = "ApiError";
  }

  get isRetryable(): boolean {
    return ["E003", "E007", "E008", "E009", "E010"].includes(this.code);
  }

  get userMessage(): string {
    const messages: Record<string, string> = {
      E001: "Invalid request. Please check your input.",
      E002: "Authentication failed. Please log in again.",
      E003: "Rate limit exceeded. Please wait a moment.",
      E004: "Document not found.",
      E005: "Ingestion job not found.",
      E006: "Document is too large. Maximum size is 10MB.",
      E007: "LLM service temporarily unavailable.",
      E008: "Storage error. Please try again.",
      E009: "Extraction failed. The document may not be processable.",
      E010: "Embedding failed. Please try again.",
    };
    return messages[this.code] || this.message;
  }
}

export function handleApiError(error: unknown): ApiError {
  if (error instanceof ApiError) {
    return error;
  }

  if (error instanceof Response) {
    // Will be parsed from response
    return new ApiError(
      "UNKNOWN",
      "An unexpected error occurred",
      error.status
    );
  }

  return new ApiError("UNKNOWN", String(error), 500);
}
```

### 6.2 Error Display Component

```typescript
// src/components/shared/api-error-boundary.tsx

import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import { AlertCircle, RefreshCw } from "lucide-react";
import type { ApiError } from "@/lib/api/errors";

interface ApiErrorDisplayProps {
  error: ApiError;
  onRetry?: () => void;
}

export function ApiErrorDisplay({ error, onRetry }: ApiErrorDisplayProps) {
  return (
    <Alert variant="destructive">
      <AlertCircle className="h-4 w-4" />
      <AlertTitle>Error {error.code}</AlertTitle>
      <AlertDescription className="mt-2">
        <p>{error.userMessage}</p>
        {error.isRetryable && onRetry && (
          <Button
            variant="outline"
            size="sm"
            className="mt-3"
            onClick={onRetry}
          >
            <RefreshCw className="h-4 w-4 mr-2" />
            Retry
          </Button>
        )}
      </AlertDescription>
    </Alert>
  );
}
```

---

## Appendix: Query Key Constants

```typescript
// Complete query keys reference
export const QUERY_KEYS = {
  documents: {
    all: ["documents"],
    list: (filters) => ["documents", "list", filters],
    detail: (id) => ["documents", id],
    track: (trackId) => ["documents", "track", trackId],
    impact: (id) => ["documents", id, "impact"],
  },
  lineage: {
    document: (id) => ["lineage", "document", id],
    chunk: (id) => ["lineage", "chunk", id],
  },
  costs: {
    ingestion: (trackId) => ["costs", "ingestion", trackId],
    summary: (options) => ["costs", "summary", options],
  },
  entities: {
    provenance: (id) => ["entities", id, "provenance"],
  },
} as const;
```

---

_End of Document WEBUI-003_
