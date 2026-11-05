/** SPEC-175 Connections: one-line facts about a saved connection. Pure, no React. */

export interface ConnectionSummaryInput {
  api_shape: string;
  base_url: string;
  source: string;
  key_configured: boolean;
  key_fingerprint?: string | null;
  last_test_ok?: boolean | null;
}

const SOURCE_LABELS: Readonly<Record<string, string>> = {
  user: "Saved by you",
  fleet: "Deployment",
  env: "From environment",
};

/** Human label for where a connection came from. Unknown sources pass through. */
export function connectionSourceLabel(source?: string | null): string {
  if (!source) return "";
  const label = Object.hasOwn(SOURCE_LABELS, source) ? SOURCE_LABELS[source] : undefined;
  return label ?? source;
}

/**
 * A shape id is not a name. The saved row printed `vertexai` and `openai_chat` beside providers
 * the settings dialog calls "Vertex AI" and "OpenAI-compatible", so one screen used two names for
 * the same thing. The server's shape list wins when it has an entry; this covers the rest offline.
 */
const SHAPE_LABELS: Readonly<Record<string, string>> = {
  openai: "OpenAI",
  openai_chat: "OpenAI-compatible",
  openai_responses: "OpenAI Responses",
  "openai-compatible": "OpenAI-compatible",
  anthropic: "Anthropic",
  anthropic_messages: "Anthropic",
  gemini: "Gemini",
  google: "Gemini",
  vertexai: "Vertex AI",
  vertex: "Vertex AI",
  ollama: "Ollama",
  lmstudio: "LM Studio",
  omlx: "oMLX",
  mtplx: "MTPLX",
  llamacpp: "llama.cpp",
  "vllm-mlx": "vLLM-MLX",
  "mlx-lm": "MLX-LM",
  azure: "Azure OpenAI",
  bedrock: "AWS Bedrock",
  mistral: "Mistral",
  nvidia: "NVIDIA",
  cohere: "Cohere",
  xai: "xAI",
  openrouter: "OpenRouter",
  huggingface: "Hugging Face",
  "vscode-copilot": "VS Code Copilot",
};

export function connectionShapeLabel(apiShape?: string | null, fromServer?: string | null): string {
  if (fromServer?.trim()) return fromServer;
  const key = (apiShape ?? "").trim().toLowerCase();
  if (!key) return "";
  return Object.hasOwn(SHAPE_LABELS, key) ? (SHAPE_LABELS[key] as string) : (apiShape as string);
}

/**
 * What secret the server holds, in words. The row printed `key c2VydmljZQ` — a base64 fragment of
 * a fingerprint — which told an operator nothing about *which kind* of credential was stored.
 */
const CREDENTIAL_LABELS: Readonly<Record<string, string>> = {
  bearer: "API key saved",
  x_api_key: "API key saved",
  "google_service_account": "Service account saved",
  google_authorized_user: "Google user credential saved",
  google_api_key: "Vertex API key saved",
  env: "From environment",
  none: "",
};

export function connectionCredentialLabel(authScheme?: string | null): string {
  if (!authScheme) return "";
  const key = authScheme.trim().toLowerCase();
  return Object.hasOwn(CREDENTIAL_LABELS, key) ? (CREDENTIAL_LABELS[key] as string) : "";
}

/**
 * The non-empty facts about a connection, in display order. Blank fields are
 * dropped here so the caller can join with " · " without doubled separators.
 */
export function connectionSummaryParts(connection: ConnectionSummaryInput): string[] {
  const parts: Array<string | null | undefined | false> = [
    connectionShapeLabel(connection.api_shape),
    connection.base_url,
    connectionSourceLabel(connection.source),
    connection.key_configured && `key ${connection.key_fingerprint ?? "set"}`,
    connection.last_test_ok === false && "last test failed",
  ];
  return parts.filter((part): part is string => typeof part === "string" && part.trim() !== "");
}
/** The summary line shown under a connection's name. */
export function connectionSummary(connection: ConnectionSummaryInput): string {
  return connectionSummaryParts(connection).join(" · ");
}
