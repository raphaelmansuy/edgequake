"use client";

/**
 * Server-wide LLM defaults saved to PostgreSQL `server_config` (SPEC-043).
 * Includes configurable ENV vs Server priority mode.
 */
import { EmbeddingModelSelector, type EmbeddingSelection } from "@/components/workspace/embedding-model-selector";
import { LLMModelSelector, type LLMSelection } from "@/components/workspace/llm-model-selector";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Label } from "@/components/ui/label";
import { RadioGroup, RadioGroupItem } from "@/components/ui/radio-group";
import { Separator } from "@/components/ui/separator";
import { apiClient, ApiRequestError } from "@/lib/api/client";
import { formatModelFullId } from "@/components/models/model-picker-panel";
import { Database, Info, Loader2, Save, Server } from "lucide-react";
import { useCallback, useEffect, useState } from "react";
import { toast } from "sonner";

interface LlmDefaultsEffective {
  llm_provider: string;
  llm_model: string;
  embedding_provider: string;
  embedding_model: string;
  vision_provider: string;
  vision_model: string;
}

interface SavedLlmDefaults {
  llm_provider?: string | null;
  llm_model?: string | null;
  embedding_provider?: string | null;
  embedding_model?: string | null;
  vision_provider?: string | null;
  vision_model?: string | null;
}

interface LlmDefaultsResponse {
  effective: LlmDefaultsEffective;
  sources: Record<string, string>;
  saved: SavedLlmDefaults;
  priority_mode: "server" | "env";
  editable: boolean;
  requires_restart: boolean;
  note: string;
}

function toLlmSelection(provider?: string | null, model?: string | null): LLMSelection | undefined {
  if (!provider || !model) return undefined;
  return {
    provider,
    model,
    fullId: formatModelFullId(provider, model),
  };
}

function toEmbeddingSelection(
  provider?: string | null,
  model?: string | null,
): EmbeddingSelection | undefined {
  if (!provider || !model) return undefined;
  return { provider, model, dimension: 0 };
}

export function ServerLlmConfigCard() {
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [data, setData] = useState<LlmDefaultsResponse | null>(null);

  const [priorityMode, setPriorityMode] = useState<"server" | "env">("server");
  const [llm, setLlm] = useState<LLMSelection | undefined>();
  const [embedding, setEmbedding] = useState<EmbeddingSelection | undefined>();
  const [vision, setVision] = useState<LLMSelection | undefined>();

  const [loadError, setLoadError] = useState<string | null>(null);

  const load = useCallback(async () => {
    setLoading(true);
    setLoadError(null);
    try {
      const resp = await apiClient<LlmDefaultsResponse>("/settings/llm-defaults");
      setData(resp);
      setPriorityMode(resp.priority_mode === "env" ? "env" : "server");
      setLlm(toLlmSelection(resp.saved.llm_provider, resp.saved.llm_model));
      setEmbedding(
        toEmbeddingSelection(resp.saved.embedding_provider, resp.saved.embedding_model),
      );
      setVision(toLlmSelection(resp.saved.vision_provider, resp.saved.vision_model));
    } catch (e) {
      if (e instanceof ApiRequestError && e.status === 404) {
        setLoadError(
          "Backend is missing GET /api/v1/settings/llm-defaults (stale binary). Run: cargo build -p edgequake && make backend-restart",
        );
      } else {
        setLoadError(e instanceof Error ? e.message : "Failed to load server LLM defaults");
      }
      toast.error("Failed to load server LLM defaults");
      console.error(e);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    load();
  }, [load]);

  const handleSave = async () => {
    setSaving(true);
    try {
      await apiClient("/settings/llm-defaults", {
        method: "PATCH",
        body: JSON.stringify({
          llm_provider: llm?.provider ?? null,
          llm_model: llm?.model ?? null,
          embedding_provider: embedding?.provider ?? null,
          embedding_model: embedding?.model ?? null,
          vision_provider: vision?.provider ?? null,
          vision_model: vision?.model ?? null,
          priority_mode: priorityMode,
        }),
      });
      toast.success("Server LLM defaults saved");
      await load();
      // Hint explainability panel consumers to refresh
      window.dispatchEvent(new CustomEvent("edgequake:config-changed"));
    } catch (e) {
      toast.error("Failed to save — admin API or PostgreSQL required");
      console.error(e);
    } finally {
      setSaving(false);
    }
  };

  return (
    <Card data-testid="server-llm-config-card">
      <CardHeader className="pb-4">
        <CardTitle className="flex items-center gap-2">
          <Database className="h-5 w-5 text-primary" />
          Server LLM Configuration
        </CardTitle>
        <CardDescription>
          Persisted in the database and applied to the resolution chain below workspace overrides.
          Choose whether saved values or environment variables win when both are set.
        </CardDescription>
      </CardHeader>

      <CardContent className="space-y-6">
        {loading && !data ? (
          <div className="flex items-center gap-2 text-sm text-muted-foreground py-4">
            <Loader2 className="h-4 w-4 animate-spin" />
            Loading server defaults…
          </div>
        ) : (
          <>
            {loadError && (
              <div className="flex items-start gap-2 rounded-md border border-destructive/30 bg-destructive/10 px-3 py-2 text-xs text-destructive">
                <Info className="h-3.5 w-3.5 mt-0.5 shrink-0" />
                <span>{loadError}</span>
              </div>
            )}
            {/* Priority mode — crystal clear */}
            <div className="rounded-lg border bg-muted/30 p-4 space-y-3">
              <div className="flex items-start gap-2">
                <Server className="h-4 w-4 mt-0.5 text-primary shrink-0" />
                <div>
                  <Label className="text-sm font-semibold">Configuration priority</Label>
                  <p className="text-xs text-muted-foreground mt-1">
                    Controls which source wins when both the database and environment define the same field.
                  </p>
                </div>
              </div>

              <RadioGroup
                value={priorityMode}
                onValueChange={(v) => setPriorityMode(v as "server" | "env")}
                className="grid gap-3 sm:grid-cols-2"
                data-testid="config-priority-mode"
              >
                <label
                  htmlFor="priority-server"
                  className={`flex cursor-pointer items-start gap-3 rounded-md border p-3 transition-colors ${
                    priorityMode === "server" ? "border-primary bg-primary/5" : "border-border"
                  }`}
                >
                  <RadioGroupItem value="server" id="priority-server" className="mt-0.5" />
                  <div>
                    <span className="text-sm font-medium">Server first (recommended)</span>
                    <p className="text-xs text-muted-foreground mt-0.5">
                      Settings saved here override <code className="text-[10px]">EDGEQUAKE_*</code> env vars.
                    </p>
                  </div>
                </label>

                <label
                  htmlFor="priority-env"
                  className={`flex cursor-pointer items-start gap-3 rounded-md border p-3 transition-colors ${
                    priorityMode === "env" ? "border-primary bg-primary/5" : "border-border"
                  }`}
                >
                  <RadioGroupItem value="env" id="priority-env" className="mt-0.5" />
                  <div>
                    <span className="text-sm font-medium">Environment first</span>
                    <p className="text-xs text-muted-foreground mt-0.5">
                      Env vars win; database values fill gaps only when env is unset.
                    </p>
                  </div>
                </label>
              </RadioGroup>

              {data && (
                <div className="flex flex-wrap gap-2 text-xs">
                  <Badge variant="outline">Effective LLM: {data.effective.llm_provider}/{data.effective.llm_model}</Badge>
                  <Badge variant="secondary">
                    Source: {data.sources.llm_provider ?? "unknown"}
                  </Badge>
                </div>
              )}
            </div>

            <Separator />

            {/* Model pickers */}
            <div className="space-y-4">
              <div className="space-y-2">
                <Label>Chat & extraction (LLM)</Label>
                <LLMModelSelector value={llm} onChange={setLlm} />
              </div>

              <div className="space-y-2">
                <Label>Embedding</Label>
                <EmbeddingModelSelector value={embedding} onChange={setEmbedding} />
              </div>

              <div className="space-y-2">
                <Label>Vision / PDF</Label>
                <LLMModelSelector value={vision} onChange={setVision} filterVision />
                <p className="text-[11px] text-muted-foreground">
                  Leave unset to inherit from LLM server defaults.
                </p>
              </div>
            </div>

            {data && !data.editable && (
              <div className="flex items-start gap-2 rounded-md border border-amber-200 bg-amber-50 dark:bg-amber-950/30 px-3 py-2 text-xs text-amber-700 dark:text-amber-400">
                <Info className="h-3.5 w-3.5 mt-0.5 shrink-0" />
                PostgreSQL is required to persist server defaults. Values shown are read-only.
              </div>
            )}

            <div className="flex justify-end">
              <Button
                onClick={handleSave}
                disabled={saving || loading || data?.editable === false}
                data-testid="save-server-llm-defaults"
              >
                {saving ? (
                  <Loader2 className="h-4 w-4 mr-2 animate-spin" />
                ) : (
                  <Save className="h-4 w-4 mr-2" />
                )}
                Save server defaults
              </Button>
            </div>

            {data?.note && (
              <p className="text-[10px] text-muted-foreground">{data.note}</p>
            )}
          </>
        )}
      </CardContent>
    </Card>
  );
}
