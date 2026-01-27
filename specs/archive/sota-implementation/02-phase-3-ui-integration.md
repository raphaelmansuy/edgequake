# Phase 3: UI Integration

## Objective

Add UI components to control SOTA features and display enhanced feedback.

## Duration: 3-4 hours

---

## Task 3.1: Create Advanced Settings Panel

### Location

`edgequake_webui/src/components/query/AdvancedSettings.tsx` (NEW)

### Implementation

```tsx
import { useState } from "react";
import { Settings2, ChevronDown, ChevronUp } from "lucide-react";
import { cn } from "@/lib/utils";

export interface AdvancedQuerySettings {
  enableRerank: boolean;
  rerankModel: string;
  rerankTopK: number;
  minRerankScore: number;
  enableKeywords: boolean;
  enableDegreeRanking: boolean;
  maxEntities: number;
  maxRelationships: number;
  tokenBudget: number;
  includeSources: boolean;
}

interface AdvancedSettingsProps {
  settings: AdvancedQuerySettings;
  onChange: (settings: AdvancedQuerySettings) => void;
  disabled?: boolean;
}

export function AdvancedSettings({
  settings,
  onChange,
  disabled,
}: AdvancedSettingsProps) {
  const [isExpanded, setIsExpanded] = useState(false);

  const update = <K extends keyof AdvancedQuerySettings>(
    key: K,
    value: AdvancedQuerySettings[K]
  ) => {
    onChange({ ...settings, [key]: value });
  };

  return (
    <div className="border rounded-lg bg-card">
      <button
        onClick={() => setIsExpanded(!isExpanded)}
        className={cn(
          "w-full flex items-center justify-between p-3",
          "hover:bg-accent/50 transition-colors",
          disabled && "opacity-50 cursor-not-allowed"
        )}
        disabled={disabled}
      >
        <div className="flex items-center gap-2">
          <Settings2 className="w-4 h-4 text-muted-foreground" />
          <span className="font-medium">Advanced Settings</span>
        </div>
        {isExpanded ? (
          <ChevronUp className="w-4 h-4 text-muted-foreground" />
        ) : (
          <ChevronDown className="w-4 h-4 text-muted-foreground" />
        )}
      </button>

      {isExpanded && (
        <div className="p-4 border-t space-y-6">
          {/* Reranking Section */}
          <div className="space-y-3">
            <h4 className="font-medium text-sm flex items-center gap-2">
              <span className="w-2 h-2 rounded-full bg-blue-500" />
              Reranking
            </h4>

            <div className="flex items-center justify-between">
              <label htmlFor="enableRerank" className="text-sm">
                Enable Reranking
              </label>
              <input
                type="checkbox"
                id="enableRerank"
                checked={settings.enableRerank}
                onChange={(e) => update("enableRerank", e.target.checked)}
                disabled={disabled}
                className="toggle"
              />
            </div>

            {settings.enableRerank && (
              <div className="space-y-2 pl-4 border-l-2 border-blue-500/20">
                <div className="flex items-center justify-between">
                  <label className="text-sm text-muted-foreground">Model</label>
                  <select
                    value={settings.rerankModel}
                    onChange={(e) => update("rerankModel", e.target.value)}
                    disabled={disabled}
                    className="select select-sm"
                  >
                    <option value="jina">Jina Reranker</option>
                    <option value="cohere">Cohere Rerank</option>
                    <option value="aliyun">Aliyun</option>
                  </select>
                </div>

                <div className="flex items-center justify-between">
                  <label className="text-sm text-muted-foreground">Top K</label>
                  <input
                    type="number"
                    value={settings.rerankTopK}
                    onChange={(e) =>
                      update("rerankTopK", parseInt(e.target.value) || 10)
                    }
                    disabled={disabled}
                    min={1}
                    max={50}
                    className="input input-sm w-20"
                  />
                </div>

                <div className="flex items-center justify-between">
                  <label className="text-sm text-muted-foreground">
                    Min Score
                  </label>
                  <input
                    type="number"
                    value={settings.minRerankScore}
                    onChange={(e) =>
                      update(
                        "minRerankScore",
                        parseFloat(e.target.value) || 0.3
                      )
                    }
                    disabled={disabled}
                    min={0}
                    max={1}
                    step={0.1}
                    className="input input-sm w-20"
                  />
                </div>
              </div>
            )}
          </div>

          {/* Entity Ranking Section */}
          <div className="space-y-3">
            <h4 className="font-medium text-sm flex items-center gap-2">
              <span className="w-2 h-2 rounded-full bg-green-500" />
              Entity Ranking
            </h4>

            <div className="flex items-center justify-between">
              <label className="text-sm">Enable Keyword Extraction</label>
              <input
                type="checkbox"
                checked={settings.enableKeywords}
                onChange={(e) => update("enableKeywords", e.target.checked)}
                disabled={disabled}
                className="toggle"
              />
            </div>

            <div className="flex items-center justify-between">
              <label className="text-sm">Enable Degree Ranking</label>
              <input
                type="checkbox"
                checked={settings.enableDegreeRanking}
                onChange={(e) =>
                  update("enableDegreeRanking", e.target.checked)
                }
                disabled={disabled}
                className="toggle"
              />
            </div>
          </div>

          {/* Limits Section */}
          <div className="space-y-3">
            <h4 className="font-medium text-sm flex items-center gap-2">
              <span className="w-2 h-2 rounded-full bg-purple-500" />
              Limits
            </h4>

            <div className="grid grid-cols-2 gap-4">
              <div className="space-y-1">
                <label className="text-sm text-muted-foreground">
                  Max Entities
                </label>
                <input
                  type="number"
                  value={settings.maxEntities}
                  onChange={(e) =>
                    update("maxEntities", parseInt(e.target.value) || 20)
                  }
                  disabled={disabled}
                  min={1}
                  max={100}
                  className="input input-sm w-full"
                />
              </div>

              <div className="space-y-1">
                <label className="text-sm text-muted-foreground">
                  Max Relationships
                </label>
                <input
                  type="number"
                  value={settings.maxRelationships}
                  onChange={(e) =>
                    update("maxRelationships", parseInt(e.target.value) || 50)
                  }
                  disabled={disabled}
                  min={1}
                  max={200}
                  className="input input-sm w-full"
                />
              </div>

              <div className="space-y-1">
                <label className="text-sm text-muted-foreground">
                  Token Budget
                </label>
                <input
                  type="number"
                  value={settings.tokenBudget}
                  onChange={(e) =>
                    update("tokenBudget", parseInt(e.target.value) || 4096)
                  }
                  disabled={disabled}
                  min={512}
                  max={16384}
                  step={512}
                  className="input input-sm w-full"
                />
              </div>

              <div className="space-y-1 flex items-end">
                <div className="flex items-center gap-2">
                  <input
                    type="checkbox"
                    checked={settings.includeSources}
                    onChange={(e) => update("includeSources", e.target.checked)}
                    disabled={disabled}
                    className="toggle"
                  />
                  <label className="text-sm">Include Sources</label>
                </div>
              </div>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

export const defaultAdvancedSettings: AdvancedQuerySettings = {
  enableRerank: true,
  rerankModel: "jina",
  rerankTopK: 10,
  minRerankScore: 0.3,
  enableKeywords: true,
  enableDegreeRanking: true,
  maxEntities: 20,
  maxRelationships: 50,
  tokenBudget: 4096,
  includeSources: true,
};
```

---

## Task 3.2: Create Ingestion Settings Panel

### Location

`edgequake_webui/src/components/ingest/IngestionSettings.tsx` (NEW)

### Implementation

```tsx
import { useState } from "react";
import { Sparkles, Layers, FileText } from "lucide-react";

export interface IngestionConfig {
  enableGleaning: boolean;
  maxGleaning: number;
  useLlmSummarization: boolean;
  chunkSize: number;
  chunkOverlap: number;
}

interface IngestionSettingsProps {
  settings: IngestionConfig;
  onChange: (settings: IngestionConfig) => void;
  disabled?: boolean;
}

export function IngestionSettings({
  settings,
  onChange,
  disabled,
}: IngestionSettingsProps) {
  const update = <K extends keyof IngestionConfig>(
    key: K,
    value: IngestionConfig[K]
  ) => {
    onChange({ ...settings, [key]: value });
  };

  return (
    <div className="space-y-4">
      {/* Gleaning */}
      <div className="flex items-center justify-between p-3 bg-card rounded-lg border">
        <div className="flex items-center gap-3">
          <div className="p-2 bg-amber-500/10 rounded-lg">
            <Sparkles className="w-4 h-4 text-amber-500" />
          </div>
          <div>
            <p className="font-medium text-sm">Multi-Pass Gleaning</p>
            <p className="text-xs text-muted-foreground">
              Extract more entities with multiple LLM passes
            </p>
          </div>
        </div>
        <div className="flex items-center gap-3">
          {settings.enableGleaning && (
            <select
              value={settings.maxGleaning}
              onChange={(e) => update("maxGleaning", parseInt(e.target.value))}
              disabled={disabled}
              className="select select-sm"
            >
              <option value={1}>1 pass</option>
              <option value={2}>2 passes</option>
              <option value={3}>3 passes</option>
            </select>
          )}
          <input
            type="checkbox"
            checked={settings.enableGleaning}
            onChange={(e) => update("enableGleaning", e.target.checked)}
            disabled={disabled}
            className="toggle"
          />
        </div>
      </div>

      {/* LLM Summarization */}
      <div className="flex items-center justify-between p-3 bg-card rounded-lg border">
        <div className="flex items-center gap-3">
          <div className="p-2 bg-blue-500/10 rounded-lg">
            <Layers className="w-4 h-4 text-blue-500" />
          </div>
          <div>
            <p className="font-medium text-sm">LLM Description Merging</p>
            <p className="text-xs text-muted-foreground">
              Intelligently merge duplicate entity descriptions
            </p>
          </div>
        </div>
        <input
          type="checkbox"
          checked={settings.useLlmSummarization}
          onChange={(e) => update("useLlmSummarization", e.target.checked)}
          disabled={disabled}
          className="toggle"
        />
      </div>

      {/* Chunking */}
      <div className="p-3 bg-card rounded-lg border space-y-3">
        <div className="flex items-center gap-3">
          <div className="p-2 bg-purple-500/10 rounded-lg">
            <FileText className="w-4 h-4 text-purple-500" />
          </div>
          <div>
            <p className="font-medium text-sm">Chunking Configuration</p>
            <p className="text-xs text-muted-foreground">
              Control document splitting behavior
            </p>
          </div>
        </div>

        <div className="grid grid-cols-2 gap-4 pt-2">
          <div className="space-y-1">
            <label className="text-xs text-muted-foreground">
              Chunk Size (tokens)
            </label>
            <input
              type="number"
              value={settings.chunkSize}
              onChange={(e) =>
                update("chunkSize", parseInt(e.target.value) || 1200)
              }
              disabled={disabled}
              min={100}
              max={4000}
              step={100}
              className="input input-sm w-full"
            />
          </div>

          <div className="space-y-1">
            <label className="text-xs text-muted-foreground">
              Chunk Overlap (tokens)
            </label>
            <input
              type="number"
              value={settings.chunkOverlap}
              onChange={(e) =>
                update("chunkOverlap", parseInt(e.target.value) || 100)
              }
              disabled={disabled}
              min={0}
              max={500}
              step={10}
              className="input input-sm w-full"
            />
          </div>
        </div>
      </div>
    </div>
  );
}

export const defaultIngestionConfig: IngestionConfig = {
  enableGleaning: true,
  maxGleaning: 1,
  useLlmSummarization: true,
  chunkSize: 1200,
  chunkOverlap: 100,
};
```

---

## Task 3.3: Create Sources Display Component

### Location

`edgequake_webui/src/components/query/SourcesDisplay.tsx` (NEW)

### Implementation

```tsx
import { useState } from "react";
import { FileText, ChevronDown, ChevronUp, ExternalLink } from "lucide-react";
import { cn } from "@/lib/utils";

interface Source {
  docId: string;
  content: string;
  score: number;
  rerankScore?: number;
}

interface SourcesDisplayProps {
  sources: Source[];
  className?: string;
}

export function SourcesDisplay({ sources, className }: SourcesDisplayProps) {
  const [expandedIndex, setExpandedIndex] = useState<number | null>(null);

  if (sources.length === 0) {
    return null;
  }

  return (
    <div className={cn("space-y-2", className)}>
      <h4 className="font-medium text-sm flex items-center gap-2">
        <FileText className="w-4 h-4 text-muted-foreground" />
        Sources ({sources.length})
      </h4>

      <div className="space-y-1">
        {sources.map((source, index) => (
          <div key={index} className="border rounded-lg overflow-hidden">
            <button
              onClick={() =>
                setExpandedIndex(expandedIndex === index ? null : index)
              }
              className="w-full flex items-center justify-between p-2 hover:bg-accent/50 transition-colors"
            >
              <div className="flex items-center gap-2 text-left flex-1 min-w-0">
                <span className="text-xs font-mono bg-muted px-1.5 py-0.5 rounded shrink-0">
                  #{index + 1}
                </span>
                <span className="text-sm truncate text-muted-foreground">
                  {source.docId}
                </span>
              </div>

              <div className="flex items-center gap-2 shrink-0">
                <span
                  className={cn(
                    "text-xs px-1.5 py-0.5 rounded",
                    source.score > 0.8
                      ? "bg-green-500/10 text-green-600"
                      : source.score > 0.5
                      ? "bg-yellow-500/10 text-yellow-600"
                      : "bg-red-500/10 text-red-600"
                  )}
                >
                  {(source.score * 100).toFixed(0)}%
                </span>

                {source.rerankScore !== undefined && (
                  <span className="text-xs px-1.5 py-0.5 rounded bg-blue-500/10 text-blue-600">
                    R: {(source.rerankScore * 100).toFixed(0)}%
                  </span>
                )}

                {expandedIndex === index ? (
                  <ChevronUp className="w-4 h-4 text-muted-foreground" />
                ) : (
                  <ChevronDown className="w-4 h-4 text-muted-foreground" />
                )}
              </div>
            </button>

            {expandedIndex === index && (
              <div className="p-3 border-t bg-muted/30">
                <p className="text-sm text-muted-foreground whitespace-pre-wrap">
                  {source.content}
                </p>
              </div>
            )}
          </div>
        ))}
      </div>
    </div>
  );
}
```

---

## Task 3.4: Create Query Stats Display

### Location

`edgequake_webui/src/components/query/QueryStats.tsx` (NEW)

### Implementation

```tsx
import { Clock, Box, GitBranch, FileText, Hash, Zap } from "lucide-react";
import { cn } from "@/lib/utils";

interface Stats {
  latencyMs: number;
  retrievalMs: number;
  generationMs: number;
  entitiesCount: number;
  relationshipsCount: number;
  chunksCount: number;
  contextTokens: number;
  keywords: string[];
}

interface QueryStatsProps {
  stats: Stats;
  mode: string;
  reranked: boolean;
  className?: string;
}

export function QueryStats({
  stats,
  mode,
  reranked,
  className,
}: QueryStatsProps) {
  const formatMs = (ms: number) => {
    if (ms < 1000) return `${ms}ms`;
    return `${(ms / 1000).toFixed(1)}s`;
  };

  return (
    <div className={cn("grid grid-cols-2 md:grid-cols-4 gap-3", className)}>
      {/* Mode */}
      <div className="p-2 bg-card rounded-lg border">
        <div className="flex items-center gap-1.5 text-muted-foreground mb-1">
          <Zap className="w-3 h-3" />
          <span className="text-xs">Mode</span>
        </div>
        <p className="font-medium text-sm capitalize flex items-center gap-1">
          {mode}
          {reranked && (
            <span className="text-[10px] px-1 py-0.5 bg-blue-500/10 text-blue-600 rounded">
              +rerank
            </span>
          )}
        </p>
      </div>

      {/* Latency */}
      <div className="p-2 bg-card rounded-lg border">
        <div className="flex items-center gap-1.5 text-muted-foreground mb-1">
          <Clock className="w-3 h-3" />
          <span className="text-xs">Latency</span>
        </div>
        <p className="font-medium text-sm">{formatMs(stats.latencyMs)}</p>
        <p className="text-[10px] text-muted-foreground">
          {formatMs(stats.retrievalMs)} retrieval •{" "}
          {formatMs(stats.generationMs)} gen
        </p>
      </div>

      {/* Entities */}
      <div className="p-2 bg-card rounded-lg border">
        <div className="flex items-center gap-1.5 text-muted-foreground mb-1">
          <Box className="w-3 h-3" />
          <span className="text-xs">Entities</span>
        </div>
        <p className="font-medium text-sm">{stats.entitiesCount}</p>
      </div>

      {/* Relationships */}
      <div className="p-2 bg-card rounded-lg border">
        <div className="flex items-center gap-1.5 text-muted-foreground mb-1">
          <GitBranch className="w-3 h-3" />
          <span className="text-xs">Relationships</span>
        </div>
        <p className="font-medium text-sm">{stats.relationshipsCount}</p>
      </div>

      {/* Chunks */}
      <div className="p-2 bg-card rounded-lg border">
        <div className="flex items-center gap-1.5 text-muted-foreground mb-1">
          <FileText className="w-3 h-3" />
          <span className="text-xs">Chunks</span>
        </div>
        <p className="font-medium text-sm">{stats.chunksCount}</p>
      </div>

      {/* Tokens */}
      <div className="p-2 bg-card rounded-lg border">
        <div className="flex items-center gap-1.5 text-muted-foreground mb-1">
          <Hash className="w-3 h-3" />
          <span className="text-xs">Context Tokens</span>
        </div>
        <p className="font-medium text-sm">
          {stats.contextTokens.toLocaleString()}
        </p>
      </div>

      {/* Keywords */}
      {stats.keywords.length > 0 && (
        <div className="p-2 bg-card rounded-lg border col-span-2">
          <div className="flex items-center gap-1.5 text-muted-foreground mb-1">
            <span className="text-xs">Keywords</span>
          </div>
          <div className="flex flex-wrap gap-1">
            {stats.keywords.slice(0, 5).map((kw, i) => (
              <span key={i} className="text-xs px-1.5 py-0.5 bg-accent rounded">
                {kw}
              </span>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}
```

---

## Task 3.5: Integrate into Query Page

### Location

`edgequake_webui/src/pages/query.tsx` or `edgequake_webui/src/app/query/page.tsx`

### Changes Required

```tsx
import { useState } from "react";
import { QueryModeSelector } from "@/components/query/QueryModeSelector";
import {
  AdvancedSettings,
  defaultAdvancedSettings,
  type AdvancedQuerySettings,
} from "@/components/query/AdvancedSettings";
import { SourcesDisplay } from "@/components/query/SourcesDisplay";
import { QueryStats } from "@/components/query/QueryStats";
import { useQuery } from "@/hooks/useQuery";

export default function QueryPage() {
  const [mode, setMode] = useState<string>("adaptive");
  const [advancedSettings, setAdvancedSettings] =
    useState<AdvancedQuerySettings>(defaultAdvancedSettings);

  const { query, isLoading, result, error } = useQuery();

  const handleSubmit = async (queryText: string) => {
    await query({
      query: queryText,
      mode,
      enable_rerank: advancedSettings.enableRerank,
      rerank_model: advancedSettings.rerankModel,
      rerank_top_k: advancedSettings.rerankTopK,
      min_rerank_score: advancedSettings.minRerankScore,
      enable_keywords: advancedSettings.enableKeywords,
      enable_degree_ranking: advancedSettings.enableDegreeRanking,
      max_entities: advancedSettings.maxEntities,
      max_relationships: advancedSettings.maxRelationships,
      token_budget: advancedSettings.tokenBudget,
      include_sources: advancedSettings.includeSources,
    });
  };

  return (
    <div className="container mx-auto p-6 space-y-6">
      <h1 className="text-2xl font-bold">Knowledge Graph Query</h1>

      {/* Query Input */}
      <QueryInput onSubmit={handleSubmit} disabled={isLoading} />

      {/* Mode Selector */}
      <QueryModeSelector value={mode} onChange={setMode} disabled={isLoading} />

      {/* Advanced Settings */}
      <AdvancedSettings
        settings={advancedSettings}
        onChange={setAdvancedSettings}
        disabled={isLoading}
      />

      {/* Error Display */}
      {error && (
        <div className="p-4 bg-destructive/10 text-destructive rounded-lg">
          {error.message}
        </div>
      )}

      {/* Results */}
      {result && (
        <div className="space-y-6">
          {/* Answer */}
          <div className="p-4 bg-card rounded-lg border">
            <p className="whitespace-pre-wrap">{result.answer}</p>
          </div>

          {/* Stats */}
          <QueryStats
            stats={result.stats}
            mode={result.mode}
            reranked={result.reranked}
          />

          {/* Sources */}
          <SourcesDisplay sources={result.sources} />
        </div>
      )}
    </div>
  );
}
```

---

## Task 3.6: Integrate into Ingest Page

### Location

`edgequake_webui/src/pages/documents.tsx` or `edgequake_webui/src/app/documents/page.tsx`

### Changes Required

```tsx
import { useState } from "react";
import {
  IngestionSettings,
  defaultIngestionConfig,
  type IngestionConfig,
} from "@/components/ingest/IngestionSettings";
import { useIngest } from "@/hooks/useIngest";

export default function DocumentsPage() {
  const [ingestionConfig, setIngestionConfig] = useState<IngestionConfig>(
    defaultIngestionConfig
  );

  const { ingest, isLoading, result } = useIngest();

  const handleUpload = async (file: File) => {
    await ingest({
      file,
      enable_gleaning: ingestionConfig.enableGleaning,
      max_gleaning: ingestionConfig.maxGleaning,
      use_llm_summarization: ingestionConfig.useLlmSummarization,
      chunk_size: ingestionConfig.chunkSize,
      chunk_overlap: ingestionConfig.chunkOverlap,
    });
  };

  return (
    <div className="container mx-auto p-6 space-y-6">
      <h1 className="text-2xl font-bold">Document Ingestion</h1>

      {/* Upload Area */}
      <FileUpload onUpload={handleUpload} disabled={isLoading} />

      {/* Ingestion Settings */}
      <IngestionSettings
        settings={ingestionConfig}
        onChange={setIngestionConfig}
        disabled={isLoading}
      />

      {/* Results */}
      {result && <IngestionResult result={result} />}
    </div>
  );
}
```

---

## Task 3.7: Update API Hooks

### Location

`edgequake_webui/src/hooks/useQuery.ts`

### Changes Required

```tsx
import { useState, useCallback } from "react";
import { useMutation } from "@tanstack/react-query";

interface QueryRequest {
  query: string;
  mode: string;
  enable_rerank: boolean;
  rerank_model: string;
  rerank_top_k: number;
  min_rerank_score: number;
  enable_keywords: boolean;
  enable_degree_ranking: boolean;
  max_entities: number;
  max_relationships: number;
  token_budget: number;
  include_sources: boolean;
}

interface QueryResponse {
  answer: string;
  mode: string;
  reranked: boolean;
  sources: Array<{
    docId: string;
    content: string;
    score: number;
    rerankScore?: number;
  }>;
  stats: {
    latencyMs: number;
    retrievalMs: number;
    generationMs: number;
    entitiesCount: number;
    relationshipsCount: number;
    chunksCount: number;
    contextTokens: number;
    keywords: string[];
  };
}

export function useQuery() {
  const mutation = useMutation({
    mutationFn: async (request: QueryRequest): Promise<QueryResponse> => {
      const response = await fetch("/api/query", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          query: request.query,
          mode: request.mode,
          enable_rerank: request.enable_rerank,
          rerank_model: request.rerank_model,
          rerank_top_k: request.rerank_top_k,
          min_rerank_score: request.min_rerank_score,
          enable_keywords: request.enable_keywords,
          enable_degree_ranking: request.enable_degree_ranking,
          max_entities: request.max_entities,
          max_relationships: request.max_relationships,
          token_budget: request.token_budget,
          include_sources: request.include_sources,
        }),
      });

      if (!response.ok) {
        throw new Error(`Query failed: ${response.statusText}`);
      }

      return response.json();
    },
  });

  return {
    query: mutation.mutateAsync,
    isLoading: mutation.isPending,
    result: mutation.data,
    error: mutation.error,
    reset: mutation.reset,
  };
}
```

---

## Verification Checklist

- [ ] `pnpm build` passes (in edgequake_webui)
- [ ] `pnpm typecheck` passes
- [ ] `pnpm lint` passes
- [ ] `pnpm test` passes (if tests exist)
- [ ] Components render without errors
- [ ] Settings persist across page navigation
- [ ] API integration works correctly

---

## Cross-References

- **Previous Phase**: [01-phase-2-api-integration.md](01-phase-2-api-integration.md)
- **Next Phase**: [03-phase-4-testing.md](03-phase-4-testing.md)
- **Current State**: [00-current-state-analysis.md](00-current-state-analysis.md)
