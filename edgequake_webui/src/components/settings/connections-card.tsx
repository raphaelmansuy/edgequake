'use client';

import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { apiClient } from '@/lib/api/client';
import { ConnectionRow, type ConnectionRowData } from '@/components/settings/connection-row';
import { ConnectionDeleteDialog, type DeletableConnection } from '@/components/settings/connection-delete-dialog';
import { connectionLocality } from '@/components/settings/connection-locality';
import { cn } from '@/lib/utils';
import { PlugZap } from 'lucide-react';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { useId, useState, type ReactNode } from 'react';
import { toast } from 'sonner';

const CONNECTIONS_KEY = ['connections'] as const;

type ConnectionView = ConnectionRowData;

interface ConnectionShape {
  id: string;
  label: string;
  default_base_url: string;
  hint: string;
  /**
   * SPEC-185 — the server's security contract for this shape.
   *
   * Absent on the built-in fallback list and on an older server, so the form treats "unknown" as
   * "may take a key" and behaves as it did before. The server is authoritative when it answers.
   */
  auth?: string;
  requires_key?: boolean;
  accepts_saved_key?: boolean;
  stores_secret?: boolean;
}

/**
 * Built-in shape list, shown only while the server's `/connections/shapes` is unreachable.
 * The card says so and offers a retry; it never presents this list as the server's.
 *
 * `accepts_saved_key` is carried here too: the form must not offer a Vertex key field even when
 * the server is unreachable, because the client refuses a saved key for that shape either way.
 */
const BUILT_IN_SHAPES: ConnectionShape[] = [
  { id: 'openai', label: 'OpenAI', default_base_url: 'https://api.openai.com/v1', hint: 'API key required.', auth: 'api_key', accepts_saved_key: true, stores_secret: true },
  { id: 'anthropic', label: 'Anthropic', default_base_url: 'https://api.anthropic.com', hint: 'API key required. Messages API.', auth: 'api_key', accepts_saved_key: true, stores_secret: true },
  { id: 'gemini', label: 'Gemini', default_base_url: 'https://generativelanguage.googleapis.com', hint: 'Google AI Studio API key required.', auth: 'api_key', accepts_saved_key: true, stores_secret: true },
  { id: 'vertexai', label: 'Vertex AI', default_base_url: 'https://aiplatform.googleapis.com', hint: 'Uses the server\'s Google Cloud identity.', auth: 'server_google_identity', accepts_saved_key: false, stores_secret: false },
  { id: 'openrouter', label: 'OpenRouter', default_base_url: 'https://openrouter.ai/api/v1', hint: 'API key required. OpenAI-compatible.', auth: 'api_key', accepts_saved_key: true, stores_secret: true },
  { id: 'xai', label: 'xAI', default_base_url: 'https://api.x.ai/v1', hint: 'API key required.', auth: 'api_key', accepts_saved_key: true, stores_secret: true },
  { id: 'huggingface', label: 'Hugging Face', default_base_url: 'https://api-inference.huggingface.co/v1', hint: 'API key required.', auth: 'api_key', accepts_saved_key: true, stores_secret: true },
  { id: 'openai-compatible', label: 'OpenAI-compatible', default_base_url: 'https://api.openai.com/v1', hint: 'Chat completions API.', auth: 'optional_api_key', requires_key: false, accepts_saved_key: true, stores_secret: true },
  { id: 'ollama', label: 'Ollama', default_base_url: 'http://127.0.0.1:11434', hint: 'Local Ollama. A key is optional.', auth: 'optional_api_key', requires_key: false, accepts_saved_key: true, stores_secret: true },
  { id: 'lmstudio', label: 'LM Studio', default_base_url: 'http://127.0.0.1:1234/v1', hint: 'Local LM Studio.', auth: 'optional_api_key', requires_key: false, accepts_saved_key: true, stores_secret: true },
  { id: 'omlx', label: 'oMLX', default_base_url: 'http://127.0.0.1:9050/v1', hint: 'Local oMLX.', auth: 'optional_api_key', requires_key: false, accepts_saved_key: true, stores_secret: true },
  { id: 'mtplx', label: 'MTPLX', default_base_url: 'http://127.0.0.1:8080/v1', hint: 'Local MTPLX.', auth: 'optional_api_key', requires_key: false, accepts_saved_key: true, stores_secret: true },
  { id: 'llamacpp', label: 'llama.cpp', default_base_url: 'http://127.0.0.1:8080/v1', hint: 'llama-server.', auth: 'optional_api_key', requires_key: false, accepts_saved_key: true, stores_secret: true },
  { id: 'vllm-mlx', label: 'vLLM-MLX', default_base_url: 'http://127.0.0.1:8000/v1', hint: 'Local vLLM-MLX.', auth: 'optional_api_key', requires_key: false, accepts_saved_key: true, stores_secret: true },
  { id: 'mlx-lm', label: 'MLX-LM', default_base_url: 'http://127.0.0.1:8080/v1', hint: 'mlx_lm.server.', auth: 'optional_api_key', requires_key: false, accepts_saved_key: true, stores_secret: true },
  { id: 'vscode-copilot', label: 'VS Code Copilot', default_base_url: 'https://api.githubcopilot.com', hint: 'Server credentials.', auth: 'server_proxy', accepts_saved_key: false, stores_secret: false },
  { id: 'mistral', label: 'Mistral', default_base_url: 'https://api.mistral.ai/v1', hint: 'API key required.', auth: 'api_key', accepts_saved_key: true, stores_secret: true },
  { id: 'azure', label: 'Azure OpenAI', default_base_url: 'https://YOUR_RESOURCE.openai.azure.com', hint: 'API key required.', auth: 'api_key', accepts_saved_key: true, stores_secret: true },
  { id: 'nvidia', label: 'NVIDIA', default_base_url: 'https://integrate.api.nvidia.com/v1', hint: 'API key required.', auth: 'api_key', accepts_saved_key: true, stores_secret: true },
  { id: 'cohere', label: 'Cohere', default_base_url: 'https://api.cohere.com/v1', hint: 'API key required.', auth: 'api_key', accepts_saved_key: true, stores_secret: true },
  { id: 'bedrock', label: 'AWS Bedrock', default_base_url: 'https://bedrock-runtime.us-east-1.amazonaws.com', hint: 'Server credentials.', auth: 'aws_chain', accepts_saved_key: false, stores_secret: false },
];

interface ProbeResponse {
  ok: boolean;
  kind: string;
  message: string;
  latency_ms: number;
  embedding_dimension?: number | null;
}

/** A visible label, optional hint, and the input. Keeps the form markup uniform. */
function ConnectionField({
  id,
  label,
  hint,
  className,
  children,
}: {
  id: string;
  label: string;
  hint?: string;
  className?: string;
  children: ReactNode;
}) {
  return (
    <div className={cn('space-y-1.5', className)}>
      <Label htmlFor={id}>{label}</Label>
      {children}
      {hint ? <p className="text-xs text-muted-foreground">{hint}</p> : null}
    </div>
  );
}

export function ConnectionsCard() {
  const fieldId = useId();
  const queryClient = useQueryClient();
  const connections = useQuery({
    queryKey: CONNECTIONS_KEY,
    queryFn: () => apiClient<ConnectionView[]>('/connections'),
    select: (data) => (Array.isArray(data) ? data : []),
    retry: false,
  });
  const rows = connections.data ?? [];
  const shapesQuery = useQuery({
    queryKey: ['connection-shapes'],
    queryFn: () => apiClient<ConnectionShape[]>('/connections/shapes'),
    retry: false,
  });
  const usingBuiltInShapes = !shapesQuery.isPending && !shapesQuery.data?.length;
  const shapes = shapesQuery.data?.length ? shapesQuery.data : BUILT_IN_SHAPES;
  const [chosenShape, setShape] = useState('openai');
  // A server list that does not carry the default id must not leave the select blank.
  const shape = shapes.some((item) => item.id === chosenShape) ? chosenShape : (shapes[0]?.id ?? chosenShape);
  const selectedShape = shapes.find((item) => item.id === shape);
  const [baseUrl, setBaseUrl] = useState('https://api.openai.com/v1');
  const [slug, setSlug] = useState('local');
  const [apiKey, setApiKey] = useState('');
  const [busy, setBusy] = useState(false);
  const [editingId, setEditingId] = useState<string | null>(null);
  const [deleting, setDeleting] = useState<DeletableConnection | null>(null);

  /*
   * SPEC-185 — ask the contract, not the shape id.
   *
   * The server marks a shape that authenticates with an identity it already holds (Vertex AI,
   * Bedrock, the Copilot proxy) with `accepts_saved_key: false`. Showing a password field there
   * let an operator save a secret the client refuses to use. An older server, or the built-in
   * fallback list, may omit the flag, and then the form behaves as it always did.
   */
  const mayTakeKey = selectedShape?.accepts_saved_key !== false;
  const keyRequired = selectedShape?.requires_key === true;

  const refresh = () => queryClient.invalidateQueries({ queryKey: CONNECTIONS_KEY });

  const chooseShape = (next: string) => {
    const previous = shapes.find((item) => item.id === shape);
    const chosen = shapes.find((item) => item.id === next);
    setShape(next);
    if (chosen && (baseUrl.trim() === '' || baseUrl === previous?.default_base_url)) {
      setBaseUrl(chosen.default_base_url);
    }
  };

  const testStored = async (id: string) => {
    setBusy(true);
    try {
      const result = await apiClient<ProbeResponse>(`/connections/${id}/test`, { method: 'POST' });
      if (result.ok) toast.success(`Connected (${result.latency_ms} ms)`);
      else toast.error(result.kind || 'Test failed', { description: result.message });
      await refresh();
    } catch (err) {
      toast.error(err instanceof Error ? err.message : 'Test failed');
    } finally {
      setBusy(false);
    }
  };

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

  const beginUpdate = (row: ConnectionView) => {
    setEditingId(row.id);
    setSlug(row.slug);
    setShape(row.api_shape);
    setBaseUrl(row.base_url);
    setApiKey('');
  };

  const save = async () => {
    setBusy(true);
    try {
      const place = connectionLocality(baseUrl);
      const body = {
        slug,
        display_name: slug,
        api_shape: shape,
        base_url: baseUrl,
        api_key: apiKey || undefined,
        locality: place.locality,
        allow_private_network: place.allowPrivate,
      };
      if (editingId) {
        await apiClient(`/connections/${editingId}`, {
          method: 'PUT',
          omitTenant: true,
          body: JSON.stringify(body),
        });
        toast.success('Connection updated');
      } else {
        await apiClient('/connections', {
          method: 'POST',
          omitTenant: true,
          body: JSON.stringify(body),
        });
        toast.success('Connection saved (key write-only)');
      }
      setApiKey('');
      setEditingId(null);
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
          Connections
        </CardTitle>
        <CardDescription>
          Point EdgeQuake at OpenAI, Anthropic, Ollama, oMLX, MLX-LM, llama.cpp, or any
          OpenAI-/Anthropic-shaped local server. Keys are encrypted at rest and never shown again.
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-4">
        {usingBuiltInShapes && (
          <div
            role="status"
            data-testid="connections-shapes-fallback"
            className="flex flex-wrap items-center justify-between gap-2 rounded-md border border-amber-300 bg-amber-50 px-3 py-2 text-sm text-amber-900 dark:border-amber-800 dark:bg-amber-950/30 dark:text-amber-200"
          >
            <span>Showing built-in list, server unreachable. Shapes may not match this server.</span>
            <Button
              type="button"
              variant="outline"
              size="sm"
              disabled={shapesQuery.isFetching}
              onClick={() => void shapesQuery.refetch()}
            >
              Retry
            </Button>
          </div>
        )}
        <div className="grid gap-3 md:grid-cols-2">
          <ConnectionField id={`${fieldId}-name`} label="Name" hint="Short identifier, for example local">
            <Input
              id={`${fieldId}-name`}
              value={slug}
              onChange={(e) => setSlug(e.target.value)}
              placeholder="local"
            />
          </ConnectionField>
          <ConnectionField id={`${fieldId}-shape`} label="API shape" hint={shapes.find((item) => item.id === shape)?.hint ?? 'How the server speaks'}>
            <select
              id={`${fieldId}-shape`}
              data-testid="connection-shape"
              className="flex h-9 w-full rounded-md border border-input bg-transparent px-3 text-sm"
              value={shape}
              onChange={(e) => chooseShape(e.target.value)}
            >
              {shapes.map((item) => (
                <option key={item.id} value={item.id}>
                  {item.label}
                </option>
              ))}
            </select>
          </ConnectionField>
          <ConnectionField id={`${fieldId}-url`} label="Base URL" className="md:col-span-2">
            <Input
              id={`${fieldId}-url`}
              value={baseUrl}
              onChange={(e) => setBaseUrl(e.target.value)}
              placeholder="http://127.0.0.1:9050"
            />
          </ConnectionField>
          {mayTakeKey ? (
            <ConnectionField
              id={`${fieldId}-key`}
              label={keyRequired ? "API key" : "API key (optional)"}
              hint={
                editingId
                  ? "Leave the key blank to keep the current key. A new key replaces it."
                  : keyRequired
                    ? "Required. Encrypted at rest and never shown again."
                    : "Optional. A protected server needs one; a local one does not."
              }
              className="md:col-span-2"
            >
              <Input
                id={`${fieldId}-key`}
                type="password"
                value={apiKey}
                onChange={(e) => setApiKey(e.target.value)}
                placeholder={keyRequired ? "Paste the key" : "Leave empty for local servers"}
                autoComplete="off"
                required={keyRequired}
              />
            </ConnectionField>
          ) : (
            /*
             * SPEC-185 — this shape authenticates with an identity the server already holds.
             * Offering a key field here let an operator save a secret the client refuses to use,
             * and then wonder why nothing worked. Saying what it does need is the whole point.
             */
            <div
              className="rounded-lg border border-dashed px-3 py-2 text-xs text-muted-foreground md:col-span-2"
              data-testid="connection-no-api-key"
            >
              {selectedShape?.auth === "server_google_identity"
                ? "Vertex AI uses this server's Google Cloud identity — there is no API key to enter. Set GOOGLE_CLOUD_PROJECT (and optionally GOOGLE_CLOUD_REGION) on the server, and grant that identity aiplatform.endpoints.predict on the project."
                : selectedShape?.auth === "aws_chain"
                  ? "AWS Bedrock uses this server's AWS credential chain — there is no API key to enter. Configure AWS credentials on the server."
                  : "This provider uses credentials the server already holds. There is no API key to enter."}
            </div>
          )}
        </div>
        <div className="flex gap-2">
          <Button type="button" variant="secondary" disabled={busy} onClick={() => void testDraft()}>
            Test connection
          </Button>
          <Button type="button" disabled={busy} onClick={() => void save()} data-testid="connection-save">
            {editingId ? 'Update connection' : 'Save'}
          </Button>
          {editingId ? (
            <Button
              type="button"
              variant="ghost"
              disabled={busy}
              onClick={() => {
                setEditingId(null);
                setApiKey('');
              }}
            >
              Cancel
            </Button>
          ) : null}
        </div>
        <section aria-labelledby={`${fieldId}-saved`} className="border-t pt-4">
          <h3 id={`${fieldId}-saved`} className="text-sm font-medium">
            Saved connections
          </h3>
          <ul className="divide-y text-sm text-muted-foreground">
            {rows.map((r) => (
              <ConnectionRow
                key={`${r.source}-${r.id}-${r.slug}`}
                row={r}
                busy={busy}
                onUpdate={() => beginUpdate(r)}
                onTest={() => void testStored(r.id)}
                onDelete={() => setDeleting({ id: r.id, display_name: r.display_name })}
              />
            ))}
            {connections.isPending ? (
              <li className="h-5 py-3" aria-busy="true">Loading connections…</li>
            ) : connections.isError ? (
              <li role="alert" className="py-3 text-destructive">Could not load connections. Refresh the page to try again.</li>
            ) : rows.length === 0 ? (
              <li className="py-3">No saved connections. Env providers appear here when set.</li>
            ) : null}
          </ul>
        </section>
      </CardContent>
      <ConnectionDeleteDialog connection={deleting} onClose={() => setDeleting(null)} onDeleted={refresh} />
    </Card>
  );
}
