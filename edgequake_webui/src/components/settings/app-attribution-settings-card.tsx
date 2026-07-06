"use client";

/**
 * Application attribution configuration (SPEC-043).
 * Surfaces edgequake-llm ApplicationContext fields and provider header catalog.
 */
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { apiClient } from "@/lib/api/client";
import { getProviderDisplayName } from "@/lib/provider-display";
import { Badge } from "@/components/ui/badge";
import { Loader2, Save, Tags } from "lucide-react";
import { useCallback, useEffect, useState } from "react";
import { toast } from "sonner";

interface AttributionSettingsResponse {
  effective_context: {
    app_id?: string | null;
    app_name?: string | null;
    app_url?: string | null;
    active: boolean;
    sources: string[];
  };
  providers: Array<{
    id: string;
    display_name: string;
    attribution_support: string;
    headers: string[];
    body_fields: string[];
  }>;
  ingress_headers: string[];
  environment_variables: string[];
}

export function AppAttributionSettingsCard() {
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [data, setData] = useState<AttributionSettingsResponse | null>(null);
  const [appId, setAppId] = useState("");
  const [appName, setAppName] = useState("");
  const [appUrl, setAppUrl] = useState("");

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const resp = await apiClient<AttributionSettingsResponse>("/settings/attribution");
      setData(resp);
      setAppId(resp.effective_context.app_id ?? "");
      setAppName(resp.effective_context.app_name ?? "");
      setAppUrl(resp.effective_context.app_url ?? "");
    } catch (e) {
      toast.error("Failed to load attribution settings");
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
      await apiClient("/settings/app-attribution", {
        method: "PATCH",
        body: JSON.stringify({
          app_id: appId || null,
          app_name: appName || null,
          app_url: appUrl || null,
        }),
      });
      toast.success("Application attribution saved");
      await load();
    } catch (e) {
      toast.error("Failed to save attribution — set EDGEQUAKE_APP_* env or use admin API");
      console.error(e);
    } finally {
      setSaving(false);
    }
  };

  if (loading) {
    return (
      <Card data-testid="app-attribution-card">
        <CardContent className="py-8 flex justify-center">
          <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
        </CardContent>
      </Card>
    );
  }

  return (
    <Card data-testid="app-attribution-card">
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          <Tags className="h-5 w-5" />
          Application Attribution
        </CardTitle>
        <CardDescription>
          Identifies EdgeQuake to upstream LLM providers (OpenRouter referer, OpenAI client ID, etc.).
          Configure via env vars or save here when admin API is enabled.
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-6">
        <div className="grid gap-4 sm:grid-cols-3">
          <div className="space-y-2">
            <Label htmlFor="app-id">App ID</Label>
            <Input
              id="app-id"
              data-testid="app-attribution-app-id"
              value={appId}
              onChange={(e) => setAppId(e.target.value)}
              placeholder="edgequake"
            />
          </div>
          <div className="space-y-2">
            <Label htmlFor="app-name">App Name</Label>
            <Input
              id="app-name"
              data-testid="app-attribution-app-name"
              value={appName}
              onChange={(e) => setAppName(e.target.value)}
              placeholder="EdgeQuake"
            />
          </div>
          <div className="space-y-2">
            <Label htmlFor="app-url">App URL</Label>
            <Input
              id="app-url"
              data-testid="app-attribution-app-url"
              value={appUrl}
              onChange={(e) => setAppUrl(e.target.value)}
              placeholder="http://localhost:3000"
            />
          </div>
        </div>
        <div className="flex items-center gap-2">
          <Button onClick={handleSave} disabled={saving} data-testid="app-attribution-save">
            {saving ? <Loader2 className="h-4 w-4 animate-spin mr-2" /> : <Save className="h-4 w-4 mr-2" />}
            Save attribution
          </Button>
          {data?.effective_context.sources.length ? (
            <span className="text-xs text-muted-foreground">
              Sources: {data.effective_context.sources.join(", ")}
            </span>
          ) : null}
        </div>

        {data && (
          <div className="space-y-2">
            <h4 className="text-sm font-medium">Provider header catalog</h4>
            <div className="max-h-48 overflow-y-auto rounded-md border divide-y text-xs" data-testid="app-attribution-provider-catalog">
              {data.providers.map((p) => (
                <div key={p.id} className="p-2 flex flex-wrap items-center gap-2">
                  <span className="font-medium">{getProviderDisplayName(p.id)}</span>
                  <Badge variant="outline">{p.attribution_support}</Badge>
                  {p.headers.map((h) => (
                    <code key={h} className="bg-muted px-1 rounded">{h}</code>
                  ))}
                </div>
              ))}
            </div>
          </div>
        )}
      </CardContent>
    </Card>
  );
}
