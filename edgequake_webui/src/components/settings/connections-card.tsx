'use client';

import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { ApiRequestError, apiClient } from '@/lib/api/client';
import { ConnectionRow, type ConnectionRowData } from '@/components/settings/connection-row';
import { ConnectionDeleteDialog, type DeletableConnection } from '@/components/settings/connection-delete-dialog';
import { connectionLocality } from '@/components/settings/connection-locality';
import { cn } from '@/lib/utils';
import { PlugZap } from 'lucide-react';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { useId, useState, type ReactNode } from 'react';
import { toast } from 'sonner';

const CONNECTIONS_KEY = ['connections'] as const;
const DEFAULT_SLUG = 'local';
/** Fleet credentials are deployment-scoped. Auth fills a tenant even when the header is omitted. */
const DEPLOYMENT_SCOPE = '?scope=deployment';

function connectionError(err: unknown, fallback: string): string {
  if (err instanceof ApiRequestError && err.code === 'deployment_admin_required') {
    return err.message && err.message !== 'Forbidden'
      ? err.message
      : 'Only a platform admin can add a provider on the deployment.';
  }
  return err instanceof Error ? err.message : fallback;
}

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
  /** `server_identity`, `service_account_json`, `api_key`. Absent on an older server. */
  secret_kinds?: string[];
  /** False when the client builds its own URL (Gemini, Vertex, Bedrock). Absent = editable. */
  base_url_editable?: boolean;
}

/**
 * Built-in shape list, shown only while the server's `/connections/shapes` is unreachable.
 * The card says so and offers a retry; it never presents this list as the server's.
 *
 * Vertex lists the secrets it can store: the server identity, a service-account JSON file, or an
 * API key. Bedrock and Copilot list none, because a pasted value would be ignored.
 */
const BUILT_IN_SHAPES: ConnectionShape[] = [
  { id: 'openai', label: 'OpenAI', default_base_url: 'https://api.openai.com/v1', hint: 'API key required.', auth: 'api_key', accepts_saved_key: true, stores_secret: true },
  { id: 'anthropic', label: 'Anthropic', default_base_url: 'https://api.anthropic.com', hint: 'API key required. Messages API.', auth: 'api_key', accepts_saved_key: true, stores_secret: true },
  { id: 'gemini', label: 'Gemini', default_base_url: 'https://generativelanguage.googleapis.com', hint: 'Google AI Studio API key required.', auth: 'api_key', requires_key: true, accepts_saved_key: true, stores_secret: true, base_url_editable: false },
  { id: 'vertexai', label: 'Vertex AI', default_base_url: 'https://aiplatform.googleapis.com', hint: 'Service-account JSON, a Vertex API key, or this server\'s Google Cloud identity.', auth: 'server_google_identity', accepts_saved_key: true, stores_secret: true, requires_key: false, secret_kinds: ['server_identity', 'service_account_json', 'api_key'] },
  { id: 'openrouter', label: 'OpenRouter', default_base_url: 'https://openrouter.ai/api/v1', hint: 'API key required. OpenAI-compatible.', auth: 'api_key', accepts_saved_key: true, stores_secret: true },
  { id: 'xai', label: 'xAI', default_base_url: 'https://api.x.ai/v1', hint: 'API key required.', auth: 'api_key', accepts_saved_key: true, stores_secret: true },
  { id: 'huggingface', label: 'Hugging Face', default_base_url: 'https://router.huggingface.co/v1', hint: 'Access token required (HF_TOKEN). OpenAI-compatible router.', auth: 'api_key', requires_key: true, accepts_saved_key: true, stores_secret: true },
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
  { id: 'azure', label: 'Azure OpenAI', default_base_url: 'https://YOUR_RESOURCE.openai.azure.com/openai/v1', hint: 'API key required. The URL ends in /openai/v1.', auth: 'api_key_and_endpoint', requires_key: true, accepts_saved_key: true, stores_secret: true },
  { id: 'nvidia', label: 'NVIDIA', default_base_url: 'https://integrate.api.nvidia.com/v1', hint: 'API key required.', auth: 'api_key', accepts_saved_key: true, stores_secret: true },
  { id: 'cohere', label: 'Cohere', default_base_url: 'https://api.cohere.ai/compatibility/v1', hint: 'API key required. OpenAI-compatible endpoint.', auth: 'api_key', requires_key: true, accepts_saved_key: true, stores_secret: true },
  { id: 'bedrock', label: 'AWS Bedrock', default_base_url: 'https://bedrock-runtime.us-east-1.amazonaws.com', hint: 'Server AWS chain or AWS_BEARER_TOKEN_BEDROCK. A pasted key is not used.', auth: 'aws_chain', accepts_saved_key: false, stores_secret: false, secret_kinds: [] },
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
  const [slug, setSlug] = useState(DEFAULT_SLUG);
  const [apiKey, setApiKey] = useState('');
  const [credentialKind, setCredentialKind] = useState('server_identity');
  const [credentialRegion, setCredentialRegion] = useState('us-central1');
  const [googleProject, setGoogleProject] = useState('');
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
  const secretKinds = selectedShape?.secret_kinds;
  const googleCredential = secretKinds?.includes('service_account_json') === true;
  const mayTakeKey = googleCredential
    ? credentialKind !== 'server_identity'
    : selectedShape?.accepts_saved_key !== false;
  const keyRequired = googleCredential ? credentialKind !== 'server_identity' : selectedShape?.requires_key === true;
  const keyIsJson = googleCredential && credentialKind === 'service_account_json';
  const projectRequired = googleCredential && credentialKind !== 'service_account_json' && googleProject.trim() === '';
  // The server states whether a Base URL changes anything for this shape. An older server omits
  // the flag, and then the field behaves as it always did.
  const baseUrlLocked =
    selectedShape?.base_url_editable === false ||
    selectedShape?.auth === 'server_google_identity' ||
    selectedShape?.auth === 'aws_chain' ||
    selectedShape?.auth === 'server_proxy';
  const canSubmit = !busy && slug.trim() !== '' && (!keyRequired || apiKey.trim() !== '') && !projectRequired;

  const refresh = () => queryClient.invalidateQueries({ queryKey: CONNECTIONS_KEY });

  const chooseShape = (next: string) => {
    const previous = shapes.find((item) => item.id === shape);
    const chosen = shapes.find((item) => item.id === next);
    setShape(next);
    if (chosen && (baseUrl.trim() === '' || baseUrl === previous?.default_base_url)) {
      setBaseUrl(chosen.default_base_url);
    }
    // The placeholder name is a real slug. Leaving it as "local" while the shape says Vertex AI
    // saves a Vertex connection called "local", which is how the reviewed screenshot read.
    if (slug === DEFAULT_SLUG || slug === previous?.id) {
      setSlug(next);
    }
    setApiKey('');
    setGoogleProject('');
    setCredentialKind(chosen?.secret_kinds?.includes('service_account_json') ? 'server_identity' : 'api_key');
  };

  const testStored = async (id: string) => {
    setBusy(true);
    try {
      const result = await apiClient<ProbeResponse>(`/connections/${id}/test${DEPLOYMENT_SCOPE}`, {
        method: 'POST',
        omitTenant: true,
      });
      if (result.ok) toast.success(`Connected (${result.latency_ms} ms)`);
      else toast.error(result.kind || 'Test failed', { description: result.message });
      await refresh();
    } catch (err) {
      toast.error(connectionError(err, 'Test failed'));
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
          api_key: mayTakeKey ? apiKey || undefined : undefined,
          credential_region: googleCredential ? credentialRegion : undefined,
          google_project: googleCredential ? googleProject.trim() || undefined : undefined,
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
    setGoogleProject(row.google_project ?? '');
    if (row.api_shape === 'vertexai' || row.api_shape === 'vertex') {
      setCredentialKind(row.key_configured ? 'service_account_json' : 'server_identity');
    }
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
        api_key: mayTakeKey ? apiKey || undefined : undefined,
        credential_region: googleCredential ? credentialRegion : undefined,
        google_project: googleCredential ? googleProject.trim() || undefined : undefined,
        locality: place.locality,
        allow_private_network: place.allowPrivate,
      };
      if (editingId) {
        await apiClient(`/connections/${editingId}${DEPLOYMENT_SCOPE}`, {
          method: 'PUT',
          omitTenant: true,
          body: JSON.stringify(body),
        });
        toast.success('Connection updated');
      } else {
        await apiClient(`/connections${DEPLOYMENT_SCOPE}`, {
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
      toast.error(connectionError(err, 'Save failed'));
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
          Add a cloud provider (OpenAI, Anthropic, Vertex AI, Azure, Mistral, Bedrock) or a local
          server (Ollama, LM Studio, oMLX, llama.cpp). A saved key or key file is encrypted at rest
          and never shown again.
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
          <ConnectionField
            id={`${fieldId}-url`}
            label="Base URL"
            hint={
              baseUrlLocked
                ? "Fixed for this provider. The client builds its own endpoint from the credential or the region."
                : undefined
            }
            className="md:col-span-2"
          >
            <Input
              id={`${fieldId}-url`}
              value={baseUrl}
              onChange={(e) => setBaseUrl(e.target.value)}
              placeholder="http://127.0.0.1:9050"
              disabled={baseUrlLocked}
            />
          </ConnectionField>
          {googleCredential ? (
            <ConnectionField
              id={`${fieldId}-kind`}
              label="Credential"
              hint={
                credentialKind === 'api_key'
                  ? "An Express mode key, or a Google Cloud API key bound to a service account."
                  : credentialKind === 'service_account_json'
                    ? "The key file behind gcloud auth activate-service-account. Needs project_id and private_key."
                    : "Calls use the identity this server already holds. The project id is saved; the key is not."
              }
              className="md:col-span-2"
            >
              <select
                id={`${fieldId}-kind`}
                data-testid="connection-credential-kind"
                className="flex h-9 w-full rounded-md border border-input bg-transparent px-3 text-sm"
                value={credentialKind}
                onChange={(e) => {
                  setCredentialKind(e.target.value);
                  setApiKey('');
                }}
              >
                <option value="server_identity">This server&apos;s Google Cloud identity</option>
                <option value="service_account_json">Service account JSON</option>
                <option value="api_key">API key</option>
              </select>
            </ConnectionField>
          ) : null}
          {googleCredential ? (
            <ConnectionField
              id={`${fieldId}-project`}
              label="Google Cloud project"
              hint="Saved with this connection. Queries use it instead of GOOGLE_CLOUD_PROJECT."
            >
              <Input
                id={`${fieldId}-project`}
                data-testid="connection-google-project"
                value={googleProject}
                onChange={(e) => setGoogleProject(e.target.value)}
                placeholder="my-project"
                autoComplete="off"
                spellCheck={false}
              />
            </ConnectionField>
          ) : null}
          {googleCredential && credentialKind !== 'server_identity' ? (
            <ConnectionField id={`${fieldId}-region`} label="Location" hint="us-central1 or global. Not a secret.">
              <Input
                id={`${fieldId}-region`}
                value={credentialRegion}
                onChange={(e) => setCredentialRegion(e.target.value)}
                autoComplete="off"
              />
            </ConnectionField>
          ) : null}
          {mayTakeKey ? (
            <ConnectionField
              id={`${fieldId}-key`}
              label={keyIsJson ? "Service account JSON" : keyRequired ? "API key" : "API key (optional)"}
              hint={
                keyIsJson
                  ? "The whole JSON file. Encrypted at rest and never shown again."
                  : editingId
                  ? "Leave the key blank to keep the current key. A new key replaces it."
                  : keyRequired
                    ? "Required. Encrypted at rest and never shown again."
                    : "Optional. A protected server needs one; a local one does not."
              }
              className="md:col-span-2"
            >
              {keyIsJson ? (
                <textarea
                  id={`${fieldId}-key`}
                  data-testid="connection-service-account"
                  className="flex min-h-28 w-full rounded-md border border-input bg-transparent px-3 py-2 font-mono text-xs"
                  value={apiKey}
                  onChange={(e) => setApiKey(e.target.value)}
                  placeholder='{"type":"service_account",...}'
                  autoComplete="off"
                  spellCheck={false}
                  required
                />
              ) : (
                <Input
                  id={`${fieldId}-key`}
                  type="password"
                  value={apiKey}
                  onChange={(e) => setApiKey(e.target.value)}
                  placeholder={keyRequired ? "Paste the key" : "Leave empty for local servers"}
                  autoComplete="off"
                  required={keyRequired}
                />
              )}
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
                ? "No secret is stored. Calls use this server's Google Cloud identity. Choose Service account JSON to store a key file instead."
                : selectedShape?.auth === "aws_chain"
                  ? "No secret is stored. The Bedrock client signs with this server's AWS chain: an instance role, AWS_ACCESS_KEY_ID and AWS_SECRET_ACCESS_KEY, or AWS_BEARER_TOKEN_BEDROCK."
                  : "No secret is stored. This provider uses credentials the server already holds."}
            </div>
          )}
        </div>
        <div className="flex gap-2">
          <Button type="button" variant="secondary" disabled={busy} onClick={() => void testDraft()}>
            Test connection
          </Button>
          <Button type="button" disabled={!canSubmit} onClick={() => void save()} data-testid="connection-save">
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
