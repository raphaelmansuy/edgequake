# OODA Loop Iteration #5 - Orient Phase

**Date:** 2025-01-10  
**Mission:** WebUI Provider Selector - API & UI Design  
**Phase:** Orient (Analysis & Architecture Design)

---

## Executive Summary

**Decision:** Build read-only provider status visibility UI (MVP) in this iteration, defer dynamic switching to Iteration #8.

**Rationale:**

- ✅ Delivers immediate value (users can see current provider config)
- ✅ Minimal backend changes (single read-only endpoint)
- ✅ No AppState hot-swap complexity
- ✅ Foundation for future dynamic switching

**Architecture:** Backend status introspection endpoint + Frontend polling display

---

## 1. Architecture Decision: Read-Only Status (MVP)

### 1.1 Option Analysis

**Option A: Full Dynamic Switching (Deferred)**

```
Frontend → POST /api/settings/provider → Backend recreates AppState → Swap global state
```

- ✅ Pros: Complete solution, no restart needed
- ❌ Cons: Complex AppState hot-swap, 6-8 hours effort, risky for Iteration #5
- **Decision:** DEFER to Iteration #8

**Option B: Read-Only Status Display (CHOSEN)**

```
Frontend → GET /api/settings/provider/status → Backend returns current config
```

- ✅ Pros: Simple, safe, 4 hours effort, immediate value
- ✅ Pros: Foundation for future switching
- ❌ Cons: User must restart to change provider
- **Decision:** IMPLEMENT in Iteration #5

**Option C: Cookie-Based Provider Hint**

```
Frontend sets cookie → Middleware reads cookie → ProviderFactory uses hint
```

- ❌ Cons: Hacky, breaks multi-tenant, not per-request
- **Decision:** REJECTED

### 1.2 Chosen Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                        EdgeQuake WebUI                          │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │  Settings Page (React Component)                         │   │
│  │  ┌────────────────────────────────────────────────────┐  │   │
│  │  │  LLM Provider Card                                 │  │   │
│  │  │  - Provider: "Ollama" (from status API)           │  │   │
│  │  │  - Status: ✅ Connected                            │  │   │
│  │  │  - Dimension: 768                                  │  │   │
│  │  │  - Model: gemma3:12b                               │  │   │
│  │  │  - Instructions: "To change, update .env..."      │  │   │
│  │  └────────────────────────────────────────────────────┘  │   │
│  └──────────────────────────────────────────────────────────┘   │
│                              ↑                                  │
│                    Polls every 30s                              │
│                              │                                  │
└──────────────────────────────┼──────────────────────────────────┘
                               │
                               ↓ GET /api/settings/provider/status
┌─────────────────────────────────────────────────────────────────┐
│                    EdgeQuake Backend API                        │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │  Provider Status Endpoint (NEW)                          │   │
│  │  ───────────────────────────────────────────────────     │   │
│  │  async fn get_provider_status(                           │   │
│  │      State(app_state): State<Arc<AppState>>              │   │
│  │  ) -> Json<ProviderStatusResponse>                       │   │
│  │  {                                                        │   │
│  │      // Read from AppState (introspection only)          │   │
│  │      let llm_name = app_state.llm_provider.name();       │   │
│  │      let llm_model = app_state.llm_provider.model();     │   │
│  │      let emb_dim = app_state.embedding_provider.dimension();│ │
│  │      let storage_dim = app_state.vector_storage.dimension();│ │
│  │                                                           │   │
│  │      // Check if provider is responsive                  │   │
│  │      let status = check_provider_health(...);            │   │
│  │                                                           │   │
│  │      Json(ProviderStatusResponse { ... })                │   │
│  │  }                                                        │   │
│  └──────────────────────────────────────────────────────────┘   │
│                              ↑                                  │
│                    Reads from AppState                          │
│                              │                                  │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │  AppState (existing)                                     │   │
│  │  ───────────────────────────────────────────────────     │   │
│  │  - llm_provider: Arc<dyn LLMProvider>                    │   │
│  │  - embedding_provider: Arc<dyn EmbeddingProvider>        │   │
│  │  - vector_storage: Arc<dyn VectorStorage>                │   │
│  │                                                           │   │
│  │  Created at startup via ProviderFactory::from_env()      │   │
│  └──────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
```

---

## 2. Backend API Design

### 2.1 Endpoint Specification

**Route:** `GET /api/settings/provider/status`

**Authentication:** Required (uses existing JWT middleware)

**Response Format:**

```json
{
  "provider": {
    "name": "ollama",
    "type": "llm",
    "status": "connected",
    "model": "gemma3:12b",
    "base_url": "http://localhost:11434",
    "config": {
      "timeout_ms": 30000
    }
  },
  "embedding": {
    "name": "ollama",
    "type": "embedding",
    "status": "connected",
    "model": "embeddinggemma:latest",
    "dimension": 768
  },
  "storage": {
    "type": "postgres",
    "dimension": 768,
    "dimension_mismatch": false,
    "namespace": "default"
  },
  "metadata": {
    "checked_at": "2025-01-10T15:30:00Z",
    "uptime_seconds": 3600
  }
}
```

**Response Type (Rust):**

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderStatusResponse {
    pub provider: LLMProviderStatus,
    pub embedding: EmbeddingProviderStatus,
    pub storage: StorageStatus,
    pub metadata: StatusMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMProviderStatus {
    pub name: String,          // "ollama", "openai", "lmstudio", "mock"
    #[serde(rename = "type")]
    pub provider_type: String, // "llm"
    pub status: ConnectionStatus,
    pub model: String,         // "gemma3:12b", "gpt-4o-mini", etc.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,
    pub config: serde_json::Value, // Provider-specific config
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingProviderStatus {
    pub name: String,
    #[serde(rename = "type")]
    pub provider_type: String, // "embedding"
    pub status: ConnectionStatus,
    pub model: String,
    pub dimension: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageStatus {
    #[serde(rename = "type")]
    pub storage_type: String,  // "memory" or "postgres"
    pub dimension: usize,
    pub dimension_mismatch: bool,
    pub namespace: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ConnectionStatus {
    Connected,    // Provider responsive
    Connecting,   // Checking status (shouldn't happen in response)
    Disconnected, // Provider not reachable
    Error,        // Configuration error
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusMetadata {
    pub checked_at: String,    // ISO 8601 timestamp
    pub uptime_seconds: u64,   // Server uptime
}
```

### 2.2 Implementation Plan

**File:** `edgequake/crates/edgequake-api/src/handlers/settings.rs` (NEW)

**Handler Implementation:**

```rust
use axum::{extract::State, Json};
use std::sync::Arc;
use crate::state::AppState;
use crate::errors::AppError;

pub async fn get_provider_status(
    State(app_state): State<Arc<AppState>>,
) -> Result<Json<ProviderStatusResponse>, AppError> {
    // 1. Get LLM provider info
    let llm_name = app_state.llm_provider.name();
    let llm_model = app_state.llm_provider.model().unwrap_or("unknown".to_string());

    // 2. Check LLM provider health (timeout: 2 seconds)
    let llm_status = match check_llm_health(&app_state.llm_provider).await {
        Ok(_) => ConnectionStatus::Connected,
        Err(_) => ConnectionStatus::Disconnected,
    };

    // 3. Get embedding provider info
    let emb_name = app_state.embedding_provider.name();
    let emb_model = app_state.embedding_provider.model().unwrap_or("unknown".to_string());
    let emb_dim = app_state.embedding_provider.dimension();

    // 4. Check embedding provider health
    let emb_status = match check_embedding_health(&app_state.embedding_provider).await {
        Ok(_) => ConnectionStatus::Connected,
        Err(_) => ConnectionStatus::Disconnected,
    };

    // 5. Get storage info
    let storage_dim = app_state.vector_storage.dimension();
    let storage_type = if std::any::type_name_of_val(&*app_state.vector_storage).contains("Memory") {
        "memory"
    } else {
        "postgres"
    };
    let dimension_mismatch = storage_dim != emb_dim;
    let namespace = app_state.vector_storage.namespace();

    // 6. Get metadata
    let uptime = app_state.start_time.elapsed().as_secs();
    let checked_at = chrono::Utc::now().to_rfc3339();

    // 7. Build response
    Ok(Json(ProviderStatusResponse {
        provider: LLMProviderStatus {
            name: llm_name,
            provider_type: "llm".to_string(),
            status: llm_status,
            model: llm_model,
            base_url: None, // TODO: Extract from provider config
            config: serde_json::json!({}),
        },
        embedding: EmbeddingProviderStatus {
            name: emb_name,
            provider_type: "embedding".to_string(),
            status: emb_status,
            model: emb_model,
            dimension: emb_dim,
        },
        storage: StorageStatus {
            storage_type: storage_type.to_string(),
            dimension: storage_dim,
            dimension_mismatch,
            namespace: namespace.to_string(),
        },
        metadata: StatusMetadata {
            checked_at,
            uptime_seconds: uptime,
        },
    }))
}

/// Check LLM provider health with timeout
async fn check_llm_health(provider: &Arc<dyn LLMProvider>) -> Result<(), LlmError> {
    // Try a lightweight health check
    // Option 1: Call provider.complete() with minimal prompt (if supported)
    // Option 2: Skip health check and assume connected (MVP approach)

    // MVP: Skip actual health check, assume connected
    Ok(())
}

/// Check embedding provider health with timeout
async fn check_embedding_health(provider: &Arc<dyn EmbeddingProvider>) -> Result<(), LlmError> {
    // MVP: Skip actual health check, assume connected
    Ok(())
}
```

**Route Registration:**

**File:** `edgequake/crates/edgequake-api/src/routes.rs`

```rust
use crate::handlers::settings;

pub fn create_router(app_state: Arc<AppState>) -> Router {
    Router::new()
        // ... existing routes ...
        .route("/api/settings/provider/status", get(settings::get_provider_status))
        .with_state(app_state)
}
```

---

## 3. Frontend UI Design

### 3.1 Component Hierarchy

```
<SettingsPage>
  ├── <AppearanceCard>
  ├── <ProviderStatusCard>  ← NEW
  │   ├── <ProviderBadge provider="ollama" status="connected" />
  │   ├── <DimensionDisplay dimension={768} mismatch={false} />
  │   ├── <ModelDisplay model="gemma3:12b" />
  │   └── <ConfigurationInstructions provider="ollama" />
  ├── <GraphVisualizationCard>
  ├── <QueryDefaultsCard>
  ├── <IngestionSettingsCard>
  └── <DataManagementCard>
```

### 3.2 Provider Status Card Design

**Location:** Insert after Appearance card, before Graph Visualization

**Visual Layout:**

```
┌──────────────────────────────────────────────────────────────┐
│  🔌 LLM Provider                                             │
│  View current provider configuration                         │
│                                                              │
│  ┌────────────────────────────────────────────────────────┐  │
│  │                                                        │  │
│  │  Current Provider                                      │  │
│  │  ┌────────────────────────────────────────┐           │  │
│  │  │  Ollama  ✅ Connected                  │           │  │
│  │  └────────────────────────────────────────┘           │  │
│  │                                                        │  │
│  │  ────────────────────────────────────────────────     │  │
│  │                                                        │  │
│  │  Provider Details                                      │  │
│  │                                                        │  │
│  │  Model:               gemma3:12b                       │  │
│  │  Embedding Model:     embeddinggemma:latest            │  │
│  │  Embedding Dimension: 768                              │  │
│  │  Storage Type:        PostgreSQL                       │  │
│  │  Storage Dimension:   768  ✅ Match                    │  │
│  │                                                        │  │
│  │  ────────────────────────────────────────────────     │  │
│  │                                                        │  │
│  │  Configuration                                         │  │
│  │                                                        │  │
│  │  To change provider, update environment variables:    │  │
│  │                                                        │  │
│  │  ┌──────────────────────────────────────────────────┐ │  │
│  │  │ export OLLAMA_HOST=http://localhost:11434        │ │  │
│  │  │ export OLLAMA_MODEL=gemma3:12b                   │ │  │
│  │  └──────────────────────────────────────────────────┘ │  │
│  │                                      [📋 Copy]         │  │
│  │                                                        │  │
│  │  [📖 Provider Documentation]  [🔄 Refresh Status]     │  │
│  │                                                        │  │
│  └────────────────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────────────────┘
```

### 3.3 Component Implementation

**File:** `edgequake_webui/src/components/settings/provider-status-card.tsx` (NEW)

```tsx
"use client";

import { useEffect, useState } from "react";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Separator } from "@/components/ui/separator";
import { Copy, ExternalLink, RefreshCw, Plug } from "lucide-react";
import { toast } from "sonner";

interface ProviderStatusCardProps {
  // Optional: Allow disabling auto-refresh for testing
  autoRefresh?: boolean;
}

export function ProviderStatusCard({
  autoRefresh = true,
}: ProviderStatusCardProps) {
  const [status, setStatus] = useState<ProviderStatusResponse | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const fetchStatus = async () => {
    try {
      setLoading(true);
      const response = await fetch("/api/settings/provider/status");
      if (!response.ok) throw new Error("Failed to fetch provider status");
      const data = await response.json();
      setStatus(data);
      setError(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Unknown error");
      toast.error("Failed to fetch provider status");
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    fetchStatus();

    // Poll every 30 seconds if auto-refresh enabled
    if (autoRefresh) {
      const interval = setInterval(fetchStatus, 30000);
      return () => clearInterval(interval);
    }
  }, [autoRefresh]);

  const copyConfig = () => {
    const config = getProviderConfig(status?.provider.name || "unknown");
    navigator.clipboard.writeText(config);
    toast.success("Configuration copied to clipboard");
  };

  if (loading && !status) {
    return (
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Plug className="h-5 w-5 animate-pulse" />
            LLM Provider
          </CardTitle>
          <CardDescription>Loading provider status...</CardDescription>
        </CardHeader>
      </Card>
    );
  }

  if (error) {
    return (
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Plug className="h-5 w-5" />
            LLM Provider
          </CardTitle>
          <CardDescription className="text-destructive">
            Failed to load provider status: {error}
          </CardDescription>
        </CardHeader>
      </Card>
    );
  }

  const { provider, embedding, storage, metadata } = status!;

  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          <Plug className="h-5 w-5" />
          LLM Provider
        </CardTitle>
        <CardDescription>View current provider configuration</CardDescription>
      </CardHeader>

      <CardContent className="space-y-4">
        {/* Current Provider */}
        <div>
          <label className="text-sm font-medium">Current Provider</label>
          <div className="mt-2 flex items-center gap-2">
            <Badge
              variant={
                provider.status === "connected" ? "default" : "destructive"
              }
              className="text-sm"
            >
              {formatProviderName(provider.name)}
            </Badge>
            <StatusBadge status={provider.status} />
          </div>
        </div>

        <Separator />

        {/* Provider Details */}
        <div className="space-y-2">
          <label className="text-sm font-medium">Provider Details</label>
          <div className="text-sm space-y-1">
            <div className="flex justify-between">
              <span className="text-muted-foreground">Model:</span>
              <span className="font-mono">{provider.model}</span>
            </div>
            <div className="flex justify-between">
              <span className="text-muted-foreground">Embedding Model:</span>
              <span className="font-mono">{embedding.model}</span>
            </div>
            <div className="flex justify-between">
              <span className="text-muted-foreground">
                Embedding Dimension:
              </span>
              <span className="font-mono">{embedding.dimension}</span>
            </div>
            <div className="flex justify-between">
              <span className="text-muted-foreground">Storage Type:</span>
              <span className="font-mono">{storage.type}</span>
            </div>
            <div className="flex justify-between items-center">
              <span className="text-muted-foreground">Storage Dimension:</span>
              <div className="flex items-center gap-2">
                <span className="font-mono">{storage.dimension}</span>
                {storage.dimension_mismatch ? (
                  <Badge variant="destructive" className="text-xs">
                    Mismatch!
                  </Badge>
                ) : (
                  <Badge variant="secondary" className="text-xs">
                    ✓ Match
                  </Badge>
                )}
              </div>
            </div>
          </div>
        </div>

        {/* Dimension Mismatch Warning */}
        {storage.dimension_mismatch && (
          <div className="p-3 bg-destructive/10 border border-destructive/20 rounded-md">
            <p className="text-sm text-destructive font-medium">
              ⚠️ Dimension Mismatch Detected
            </p>
            <p className="text-xs text-muted-foreground mt-1">
              Storage dimension ({storage.dimension}) doesn't match provider
              dimension ({embedding.dimension}). Queries may return incorrect
              results.
            </p>
          </div>
        )}

        <Separator />

        {/* Configuration Instructions */}
        <div className="space-y-2">
          <label className="text-sm font-medium">Configuration</label>
          <p className="text-xs text-muted-foreground">
            To change provider, update environment variables and restart the
            server:
          </p>
          <div className="relative">
            <pre className="bg-muted p-3 rounded-md text-xs overflow-x-auto">
              <code>{getProviderConfig(provider.name)}</code>
            </pre>
            <Button
              size="sm"
              variant="ghost"
              className="absolute top-2 right-2"
              onClick={copyConfig}
            >
              <Copy className="h-3 w-3" />
            </Button>
          </div>
        </div>

        {/* Actions */}
        <div className="flex gap-2">
          <Button
            size="sm"
            variant="outline"
            onClick={fetchStatus}
            disabled={loading}
          >
            <RefreshCw
              className={`h-4 w-4 mr-2 ${loading ? "animate-spin" : ""}`}
            />
            Refresh Status
          </Button>
          <Button size="sm" variant="outline" asChild>
            <a href="https://docs.edgequake.ai/providers" target="_blank">
              <ExternalLink className="h-4 w-4 mr-2" />
              Provider Docs
            </a>
          </Button>
        </div>

        {/* Last Updated */}
        <p className="text-xs text-muted-foreground">
          Last updated: {new Date(metadata.checked_at).toLocaleString()}
        </p>
      </CardContent>
    </Card>
  );
}

// Helper functions

function formatProviderName(name: string): string {
  const nameMap: Record<string, string> = {
    openai: "OpenAI",
    ollama: "Ollama",
    lmstudio: "LM Studio",
    mock: "Mock (Testing)",
  };
  return nameMap[name] || name;
}

function StatusBadge({ status }: { status: string }) {
  const statusConfig = {
    connected: { label: "✅ Connected", variant: "default" as const },
    connecting: { label: "⏳ Connecting...", variant: "secondary" as const },
    disconnected: { label: "❌ Disconnected", variant: "destructive" as const },
    error: { label: "⚠️ Error", variant: "destructive" as const },
  };

  const config = statusConfig[status] || statusConfig["error"];
  return (
    <Badge variant={config.variant} className="text-xs">
      {config.label}
    </Badge>
  );
}

function getProviderConfig(provider: string): string {
  const configs: Record<string, string> = {
    openai: `export OPENAI_API_KEY="sk-proj-..."
export EDGEQUAKE_LLM_MODEL="gpt-4o-mini"
export EDGEQUAKE_EMBEDDING_MODEL="text-embedding-3-small"`,

    ollama: `export OLLAMA_HOST="http://localhost:11434"
export OLLAMA_MODEL="gemma3:12b"
export OLLAMA_EMBEDDING_MODEL="embeddinggemma:latest"`,

    lmstudio: `export EDGEQUAKE_LLM_PROVIDER="lmstudio"
export OPENAI_BASE_URL="http://localhost:1234/v1"
export OPENAI_API_KEY="lm-studio"
export OPENAI_MODEL="your-model-name"`,

    mock: `export EDGEQUAKE_LLM_PROVIDER="mock"
# No additional configuration required`,
  };

  return configs[provider] || "# Unknown provider";
}
```

### 3.4 Types Definition

**File:** `edgequake_webui/src/types/provider.ts` (NEW)

```typescript
export interface ProviderStatusResponse {
  provider: LLMProviderStatus;
  embedding: EmbeddingProviderStatus;
  storage: StorageStatus;
  metadata: StatusMetadata;
}

export interface LLMProviderStatus {
  name: string;
  type: "llm";
  status: ConnectionStatus;
  model: string;
  base_url?: string;
  config: Record<string, any>;
}

export interface EmbeddingProviderStatus {
  name: string;
  type: "embedding";
  status: ConnectionStatus;
  model: string;
  dimension: number;
}

export interface StorageStatus {
  type: "memory" | "postgres";
  dimension: number;
  dimension_mismatch: boolean;
  namespace: string;
}

export type ConnectionStatus =
  | "connected"
  | "connecting"
  | "disconnected"
  | "error";

export interface StatusMetadata {
  checked_at: string; // ISO 8601
  uptime_seconds: number;
}
```

---

## 4. Integration Plan

### 4.1 Settings Page Modification

**File:** `edgequake_webui/src/app/(dashboard)/settings/page.tsx`

**Change:** Add ProviderStatusCard after Appearance card

```tsx
import { ProviderStatusCard } from "@/components/settings/provider-status-card";

export default function SettingsPage() {
  // ... existing code ...

  return (
    <ScrollArea className="h-full">
      <div className="container max-w-4xl py-8 space-y-6">
        {/* Appearance Card */}
        <Card>...</Card>

        {/* Provider Status Card - NEW */}
        <ProviderStatusCard />

        {/* Graph Visualization Card */}
        <Card>...</Card>

        {/* ... rest of cards ... */}
      </div>
    </ScrollArea>
  );
}
```

---

## 5. Testing Strategy

### 5.1 Backend Tests

**File:** `edgequake/crates/edgequake-api/tests/e2e_provider_status.rs` (NEW)

```rust
#[tokio::test]
#[serial]
async fn test_provider_status_endpoint_ollama() {
    // Setup: Use Ollama provider
    std::env::set_var("OLLAMA_HOST", "http://localhost:11434");
    std::env::remove_var("OPENAI_API_KEY");

    let app_state = AppState::new_memory(None::<String>);
    let app = create_router(Arc::new(app_state));

    // Act: GET /api/settings/provider/status
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/settings/provider/status")
                .body(Body::empty())
                .unwrap()
        )
        .await
        .unwrap();

    // Assert: 200 OK
    assert_eq!(response.status(), StatusCode::OK);

    // Assert: Response structure
    let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
    let status: ProviderStatusResponse = serde_json::from_slice(&body).unwrap();

    assert_eq!(status.provider.name, "ollama");
    assert_eq!(status.embedding.dimension, 768);
    assert_eq!(status.storage.type, "memory");
    assert_eq!(status.storage.dimension_mismatch, false);
}

#[tokio::test]
#[serial]
async fn test_provider_status_dimension_mismatch() {
    // Setup: OpenAI provider (1536) + Ollama storage (768) - simulated mismatch
    // Note: In real scenario, this would require pre-existing storage

    std::env::set_var("OPENAI_API_KEY", "sk-test");

    // Create AppState with mocked storage dimension
    let app_state = AppState::new_memory(None::<String>);

    // ... test dimension mismatch detection ...
}
```

### 5.2 Frontend Tests

**File:** `edgequake_webui/src/components/settings/__tests__/provider-status-card.test.tsx` (NEW)

```tsx
import { render, screen, waitFor } from "@testing-library/react";
import { ProviderStatusCard } from "../provider-status-card";

// Mock fetch
global.fetch = jest.fn();

describe("ProviderStatusCard", () => {
  beforeEach(() => {
    jest.clearAllMocks();
  });

  it("renders loading state initially", () => {
    (global.fetch as jest.Mock).mockImplementation(() => new Promise(() => {}));

    render(<ProviderStatusCard autoRefresh={false} />);

    expect(screen.getByText("Loading provider status...")).toBeInTheDocument();
  });

  it("displays provider status after successful fetch", async () => {
    const mockStatus = {
      provider: { name: "ollama", status: "connected", model: "gemma3:12b" },
      embedding: {
        name: "ollama",
        dimension: 768,
        model: "embeddinggemma:latest",
      },
      storage: { type: "memory", dimension: 768, dimension_mismatch: false },
      metadata: { checked_at: "2025-01-10T15:30:00Z", uptime_seconds: 3600 },
    };

    (global.fetch as jest.Mock).mockResolvedValueOnce({
      ok: true,
      json: async () => mockStatus,
    });

    render(<ProviderStatusCard autoRefresh={false} />);

    await waitFor(() => {
      expect(screen.getByText("Ollama")).toBeInTheDocument();
      expect(screen.getByText("✅ Connected")).toBeInTheDocument();
      expect(screen.getByText("768")).toBeInTheDocument();
    });
  });

  it("displays dimension mismatch warning", async () => {
    const mockStatus = {
      provider: { name: "openai", status: "connected", model: "gpt-4o-mini" },
      embedding: {
        name: "openai",
        dimension: 1536,
        model: "text-embedding-3-small",
      },
      storage: { type: "postgres", dimension: 768, dimension_mismatch: true },
      metadata: { checked_at: "2025-01-10T15:30:00Z", uptime_seconds: 3600 },
    };

    (global.fetch as jest.Mock).mockResolvedValueOnce({
      ok: true,
      json: async () => mockStatus,
    });

    render(<ProviderStatusCard autoRefresh={false} />);

    await waitFor(() => {
      expect(
        screen.getByText(/Dimension Mismatch Detected/i)
      ).toBeInTheDocument();
      expect(screen.getByText(/Mismatch!/i)).toBeInTheDocument();
    });
  });
});
```

---

## 6. Time Estimate Validation

| Task                          | Original    | Revised     | Rationale                          |
| ----------------------------- | ----------- | ----------- | ---------------------------------- |
| 5E.1: Backend status endpoint | 45 min      | 60 min      | +15min for health check logic      |
| 5E.2: Provider info struct    | 15 min      | 20 min      | +5min for comprehensive types      |
| 5E.3: Frontend UI component   | 60 min      | 90 min      | +30min for polished UI + helpers   |
| 5E.4: Provider status hook    | 30 min      | 30 min      | ✓ Estimate accurate                |
| 5E.5: Configuration docs      | 20 min      | 15 min      | -5min (integrated into component)  |
| 5E.6: E2E testing             | 40 min      | 50 min      | +10min for dimension mismatch test |
| 5E.7: Documentation           | 30 min      | 25 min      | -5min (smaller scope)              |
| **Total**                     | **240 min** | **290 min** | **4h 50min** (~5 hours)            |

**Revised Estimate:** 5 hours (acceptable for single iteration)

---

## 7. Decision Summary

### 7.1 Key Architectural Decisions

| Decision Point            | Chosen Approach                                 | Rationale                                               |
| ------------------------- | ----------------------------------------------- | ------------------------------------------------------- |
| **Provider Switching**    | Read-only status (MVP), defer dynamic switching | Minimize risk, deliver value quickly                    |
| **Health Checks**         | Skip in MVP (always show "connected")           | Avoid complexity, 2-second timeout difficult to achieve |
| **Polling Frequency**     | 30 seconds, with manual refresh button          | Balance freshness vs server load                        |
| **Dimension Warning**     | Inline warning badge + alert box                | High visibility for critical issue                      |
| **Configuration Display** | Copy-pasteable .env snippets                    | User-friendly, no manual typing                         |
| **Authentication**        | Reuse existing JWT middleware                   | No new auth logic needed                                |

### 7.2 Deferred Decisions (Future Iterations)

- **Dynamic Provider Switching:** Iteration #8 (AppState hot-swap)
- **Per-Tenant Providers:** Iteration #12 (multi-tenant config)
- **Real Health Checks:** Iteration #10 (timeout-based provider ping)
- **Provider Metrics:** Iteration #15 (latency, cost tracking)

---

## 8. Risk Mitigation

| Risk                                       | Mitigation                                          |
| ------------------------------------------ | --------------------------------------------------- |
| **R1:** Backend types don't match frontend | Define TypeScript types from Rust serde output      |
| **R2:** Polling causes performance issues  | Make auto-refresh optional, add manual refresh      |
| **R3:** Provider health check fails        | Skip health check in MVP (assume connected)         |
| **R4:** Dimension mismatch logic complex   | Reuse existing validation from Iteration #4         |
| **R5:** UI doesn't fit mobile              | Test responsive layout, use Card component (proven) |

---

## 9. Success Metrics

**Iteration #5 Success Criteria:**

1. ✅ Backend endpoint returns provider status in <500ms
2. ✅ Frontend displays provider name, model, dimension
3. ✅ Dimension mismatch warning appears when applicable
4. ✅ Configuration snippets copy to clipboard
5. ✅ No regressions in existing settings (all tests pass)

---

## 10. Next Steps → Decide Phase

**Decide Phase Will Address:**

1. Exact implementation plan with file paths and line numbers
2. Commit strategy (atomic commits per subsystem)
3. Testing checklist (unit, integration, E2E)
4. Rollback plan if issues arise

**Key Deliverables:**

- Implementation checklist
- Code snippets with exact line numbers
- Test scenarios matrix
- Documentation updates list

---

**OODA Progress:** 5/50 iterations (10%)  
**Phase Progress:** Iteration #5 - Orient ✅ COMPLETE

**Next Phase:** Decide - Create detailed implementation plan

---

**End of Orient Phase**
