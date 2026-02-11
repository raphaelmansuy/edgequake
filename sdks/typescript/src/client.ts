/**
 * EdgeQuake client — main entry point for the SDK.
 *
 * WHY: Provides a single, discoverable entry point with resource namespaces.
 * Follows the Stripe/OpenAI pattern: `new EdgeQuake({ apiKey }).documents.list()`
 *
 * @module client
 */

import {
  resolveConfig,
  type EdgeQuakeConfig,
  type ResolvedConfig,
} from "./config.js";
import { createTransport } from "./transport/index.js";
import type { HttpTransport } from "./transport/types.js";
import type { HealthResponse } from "./types/health.js";

// Resource imports
import { ApiKeysResource } from "./resources/api-keys.js";
import { AuthResource } from "./resources/auth.js";
import { ChatResource } from "./resources/chat.js";
import { ChunksResource } from "./resources/chunks.js";
import { ConversationsResource } from "./resources/conversations.js";
import { CostsResource } from "./resources/costs.js";
import { DocumentsResource } from "./resources/documents.js";
import { FoldersResource } from "./resources/folders.js";
import { GraphResource } from "./resources/graph.js";
import { LineageResource } from "./resources/lineage.js";
import { ModelsResource } from "./resources/models.js";
import { OllamaResource } from "./resources/ollama.js";
import { PipelineResource } from "./resources/pipeline.js";
import { ProvenanceResource } from "./resources/provenance.js";
import { QueryResource } from "./resources/query.js";
import { SettingsResource } from "./resources/settings.js";
import { SharedResource } from "./resources/shared.js";
import { TasksResource } from "./resources/tasks.js";
import { TenantsResource } from "./resources/tenants.js";
import { UsersResource } from "./resources/users.js";
import { WorkspacesResource } from "./resources/workspaces.js";

/**
 * EdgeQuake SDK client.
 *
 * @example
 * ```ts
 * import { EdgeQuake } from "@edgequake/sdk";
 *
 * const client = new EdgeQuake({
 *   baseUrl: "http://localhost:8080",
 *   apiKey: "eq-key-xxx",
 * });
 *
 * // Health check
 * const health = await client.health();
 *
 * // Upload a document
 * const doc = await client.documents.upload({
 *   title: "My Doc",
 *   content: "Hello world",
 * });
 *
 * // Query the knowledge graph
 * const result = await client.query.execute({
 *   query: "What is EdgeQuake?",
 *   mode: "hybrid",
 * });
 *
 * // Stream chat completions
 * for await (const event of client.chat.stream({
 *   messages: [{ role: "user", content: "Hello!" }],
 * })) {
 *   process.stdout.write(event.choices?.[0]?.delta?.content ?? "");
 * }
 * ```
 */
export class EdgeQuake {
  /** Resolved configuration. */
  private readonly _config: ResolvedConfig;

  /** HTTP transport for API requests. */
  private readonly _transport: HttpTransport;

  // ──────────────────────────── Resource Namespaces ────────────────────────────

  /** Authentication — login, refresh, logout, current user. */
  readonly auth: AuthResource;

  /** User management (admin). */
  readonly users: UsersResource;

  /** API key management. */
  readonly apiKeys: ApiKeysResource;

  /** Document ingestion and management, including PDF sub-resource. */
  readonly documents: DocumentsResource;

  /** RAG query execution and streaming. */
  readonly query: QueryResource;

  /** Chat completions (unified chat API). */
  readonly chat: ChatResource;

  /** Conversation history and messages. */
  readonly conversations: ConversationsResource;

  /** Conversation folder management. */
  readonly folders: FoldersResource;

  /** Public shared conversation access. */
  readonly shared: SharedResource;

  /** Knowledge graph with entities and relationships sub-resources. */
  readonly graph: GraphResource;

  /** Multi-tenant management. */
  readonly tenants: TenantsResource;

  /** Workspace management and actions. */
  readonly workspaces: WorkspacesResource;

  /** Async task tracking. */
  readonly tasks: TasksResource;

  /** Pipeline status and control. */
  readonly pipeline: PipelineResource;

  /** Cost tracking, history, and budgets. */
  readonly costs: CostsResource;

  /** Entity and document lineage tracing. */
  readonly lineage: LineageResource;

  /** Chunk-level detail access. */
  readonly chunks: ChunksResource;

  /** Entity provenance tracing. */
  readonly provenance: ProvenanceResource;

  /** Provider settings and status. */
  readonly settings: SettingsResource;

  /** Model configuration and health. */
  readonly models: ModelsResource;

  /** Ollama-compatible API. */
  readonly ollama: OllamaResource;

  constructor(config?: EdgeQuakeConfig) {
    this._config = resolveConfig(config);
    // WHY: Allow test code to inject a mock transport via config._transport
    this._transport = config?._transport ?? createTransport(this._config);

    // Initialize all resource namespaces
    this.auth = new AuthResource(this._transport);
    this.users = new UsersResource(this._transport);
    this.apiKeys = new ApiKeysResource(this._transport);
    this.documents = new DocumentsResource(this._transport);
    this.query = new QueryResource(this._transport);
    this.chat = new ChatResource(this._transport);
    this.conversations = new ConversationsResource(this._transport);
    this.folders = new FoldersResource(this._transport);
    this.shared = new SharedResource(this._transport);
    this.graph = new GraphResource(this._transport);
    this.tenants = new TenantsResource(this._transport);
    this.workspaces = new WorkspacesResource(this._transport);
    this.tasks = new TasksResource(this._transport);
    this.pipeline = new PipelineResource(this._transport);
    this.costs = new CostsResource(this._transport);
    this.lineage = new LineageResource(this._transport);
    this.chunks = new ChunksResource(this._transport);
    this.provenance = new ProvenanceResource(this._transport);
    this.settings = new SettingsResource(this._transport);
    this.models = new ModelsResource(this._transport);
    this.ollama = new OllamaResource(this._transport);
  }

  // ──────────────────────────── Top-Level Convenience ────────────────────────────

  /** Health check — verifies API is operational. */
  async health(): Promise<HealthResponse> {
    return this._transport.request({ method: "GET", path: "/health" });
  }

  /** Readiness check — Kubernetes readiness probe. Returns "OK" when ready. */
  async ready(): Promise<string> {
    return this._transport.request({ method: "GET", path: "/ready" });
  }

  /** Liveness check — Kubernetes liveness probe. Returns "OK" when alive. */
  async live(): Promise<string> {
    return this._transport.request({ method: "GET", path: "/live" });
  }

  /** Get base URL. */
  get baseUrl(): string {
    return this._config.baseUrl;
  }

  /** Get the underlying transport (advanced usage). */
  get transport(): HttpTransport {
    return this._transport;
  }
}
