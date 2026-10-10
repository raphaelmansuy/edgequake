'use client';

import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import { apiClient } from '@/lib/api/client';
import { connectionLocality } from '@/components/settings/connection-locality';
import { PlugZap } from 'lucide-react';
import { useCallback, useEffect, useState } from 'react';
import { toast } from 'sonner';

interface ConnectionView {
  id: string;
  slug: string;
  display_name: string;
  api_shape: string;
  base_url: string;
  source: string;
  key_configured: boolean;
  key_fingerprint?: string | null;
  last_test_ok?: boolean | null;
}

interface ProbeResponse {
  ok: boolean;
  kind: string;
  message: string;
  latency_ms: number;
  embedding_dimension?: number | null;
}

export function ConnectionsCard() {
  const [rows, setRows] = useState<ConnectionView[]>([]);
  const [shape, setShape] = useState('openai_chat');
  const [baseUrl, setBaseUrl] = useState('http://127.0.0.1:9050');
  const [slug, setSlug] = useState('local');
  const [apiKey, setApiKey] = useState('');
  const [busy, setBusy] = useState(false);

  const refresh = useCallback(async () => {
    try {
      const data = await apiClient<ConnectionView[]>('/connections');
      setRows(data);
    } catch (err) {
      console.error(err);
    }
  }, []);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  const testDraft = async () => {
    setBusy(true);
    try {
      const place = connectionLocality(baseUrl);
      const result = await apiClient<ProbeResponse>('/providers/test', {
        method: 'POST',
        body: JSON.stringify({
          shape,
          base_url: baseUrl,
          api_key: apiKey || undefined,
          allow_private_network: place.allowPrivate,
        }),
      });
      if (result.ok) {
        toast.success(`Connected (${result.latency_ms} ms)`, {
          description: result.embedding_dimension
            ? `Embedding dimension ${result.embedding_dimension}`
            : result.message,
        });
      } else {
        toast.error(result.kind, { description: result.message });
      }
    } catch (err) {
      toast.error(err instanceof Error ? err.message : 'Test failed');
    } finally {
      setBusy(false);
    }
  };

  const save = async () => {
    setBusy(true);
    try {
      const place = connectionLocality(baseUrl);
      await apiClient('/connections', {
        method: 'POST',
        body: JSON.stringify({
          slug,
          display_name: slug,
          api_shape: shape,
          base_url: baseUrl,
          api_key: apiKey || undefined,
          locality: place.locality,
          allow_private_network: place.allowPrivate,
        }),
      });
      setApiKey('');
      toast.success('Connection saved (key write-only)');
      await refresh();
    } catch (err) {
      toast.error(err instanceof Error ? err.message : 'Save failed');
    } finally {
      setBusy(false);
    }
  };

  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          <PlugZap className="h-5 w-5" />
          LLM connections
        </CardTitle>
        <CardDescription>
          Point EdgeQuake at OpenAI, Anthropic, Ollama, oMLX, MLX-LM, llama.cpp, or any
          OpenAI-/Anthropic-shaped local server. Keys are encrypted at rest and never shown again.
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-4">
        <div className="grid gap-2 md:grid-cols-2">
          <Input value={slug} onChange={(e) => setSlug(e.target.value)} placeholder="slug" />
          <Input
            value={shape}
            onChange={(e) => setShape(e.target.value)}
            placeholder="openai_chat | anthropic_messages | ollama"
          />
          <Input
            className="md:col-span-2"
            value={baseUrl}
            onChange={(e) => setBaseUrl(e.target.value)}
            placeholder="http://127.0.0.1:9050"
          />
          <Input
            className="md:col-span-2"
            type="password"
            value={apiKey}
            onChange={(e) => setApiKey(e.target.value)}
            placeholder="API key (optional, never echoed back)"
            autoComplete="off"
          />
        </div>
        <div className="flex gap-2">
          <Button type="button" variant="secondary" disabled={busy} onClick={() => void testDraft()}>
            Test connection
          </Button>
          <Button type="button" disabled={busy} onClick={() => void save()}>
            Save
          </Button>
        </div>
        <ul className="space-y-1 text-sm text-muted-foreground">
          {rows.map((r) => (
            <li key={`${r.source}-${r.id}-${r.slug}`}>
              <span className="font-medium text-foreground">{r.display_name}</span>{' '}
              {r.api_shape} · {r.base_url} · {r.source}
              {r.key_configured ? ` · key ${r.key_fingerprint ?? 'set'}` : ''}
              {r.last_test_ok === false ? ' · last test failed' : ''}
            </li>
          ))}
          {rows.length === 0 ? <li>No saved connections. Env providers appear here when set.</li> : null}
        </ul>
      </CardContent>
    </Card>
  );
}
