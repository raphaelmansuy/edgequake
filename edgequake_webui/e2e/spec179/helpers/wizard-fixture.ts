/**
 * SPEC-179 hermetic fixture for the Create / Reconfigure Workspace wizards:
 * an organization with defaults, an offer per usage, and a create route that can refuse.
 */
import type { Page, Route } from "@playwright/test";
import { SPEC155_TENANT, SPEC155_WORKSPACE } from "../../spec155/helpers/mock-api";

export interface OfferedModel {
  provider: string;
  model: string;
  display_name?: string;
  supports_vision?: boolean;
  embedding_dimension?: number | null;
}

export interface WizardFixtureOptions {
  /** What the organization offers, per usage group. Missing groups default to a healthy offer. */
  offered?: { chat?: OfferedModel[]; embedding?: OfferedModel[]; vlm?: OfferedModel[] };
  /**
   * Eligible models when the request names a workspace. Reconfigure reads this set.
   * Absent means the workspace sees the same list as the organization.
   */
  workspaceOffered?: { chat?: OfferedModel[]; embedding?: OfferedModel[]; vlm?: OfferedModel[] };
  /**
   * Workspace GET and list carry no saved model roles, so Reconfigure inherits the organization defaults.
   */
  workspaceInheritsModels?: boolean;
  /** Organization defaults the wizard inherits. */
  defaults?: {
    llm: [string, string];
    embedding: [string, string, number];
    vision: [string, string];
  };
  /** Create answers this refusal instead of succeeding. */
  refuseCreate?: { status: number; code: string; message: string };
  /**
   * `GET /me/access` capabilities. The create affordance is gated on
   * `organization.workspaces.create`, which is *not* implied by `tenant_manage` in the settings
   * payload — a workspace admin manages a workspace but may not create one. Defaults to an
   * organization owner who may create workspaces.
   */
  canCreateWorkspace?: boolean;
  /**
   * Whether the caller may manage the organization offer (`organization.settings.manage`).
   * `false` models a workspace admin: they can create the workspace but cannot open organization
   * providers, so the stale-default advisory must name who can.
   */
  canManageProviders?: boolean;
  /**
   * SPEC-180 08 — the deployment's answer to the image embedding probe. `auth` reproduces a
   * project the identity cannot use, which used to surface only at the last wizard step.
   */
  imageProbe?: "ok" | "auth";
}

const CHAT: OfferedModel[] = [
  { provider: "openai", model: "gpt-5-nano", display_name: "GPT-5 nano" },
  { provider: "openai", model: "gpt-5.2", display_name: "GPT-5.2", supports_vision: true },
];
const EMBEDDING: OfferedModel[] = [
  { provider: "openai", model: "text-embedding-3-small", display_name: "Embedding 3 small", embedding_dimension: 1536 },
];
const VLM: OfferedModel[] = [{ provider: "openai", model: "gpt-5.2", display_name: "GPT-5.2", supports_vision: true }];

export interface WizardRecorder {
  creates: unknown[];
  /** Bodies of `PUT /workspaces/{id}` (Reconfigure → Apply). */
  updates: unknown[];
  eligibleAsks: string[];
}

const usageGroup = (usage: string): "chat" | "embedding" | "vlm" =>
  usage === "embedding" ? "embedding" : usage === "vlm" ? "vlm" : "chat";

export async function installWizardFixture(page: Page, options: WizardFixtureOptions = {}): Promise<WizardRecorder> {
  const recorder: WizardRecorder = { creates: [], updates: [], eligibleAsks: [] };
  const defaults = options.defaults ?? {
    llm: ["openai", "gpt-5-nano"],
    embedding: ["openai", "text-embedding-3-small", 1536],
    vision: ["openai", "gpt-5.2"],
  };
  const offered = {
    chat: options.offered?.chat ?? CHAT,
    embedding: options.offered?.embedding ?? EMBEDDING,
    vlm: options.offered?.vlm ?? VLM,
  };
  const workspaceOffered = options.workspaceOffered
    ? {
        chat: options.workspaceOffered.chat ?? offered.chat,
        embedding: options.workspaceOffered.embedding ?? offered.embedding,
        vlm: options.workspaceOffered.vlm ?? offered.vlm,
      }
    : null;

  const modelsFor = (usage: string, forWorkspace: boolean) => {
    const source = forWorkspace && workspaceOffered ? workspaceOffered : offered;
    return source[usageGroup(usage)].map((m) => ({
      provider: m.provider,
      provider_display_name: m.provider,
      model: m.model,
      display_name: m.display_name ?? m.model,
      supports_vision: m.supports_vision ?? false,
      embedding_dimension: m.embedding_dimension ?? null,
    }));
  };

  const inheritedWorkspace = {
    ...SPEC155_WORKSPACE,
    tenant_id: SPEC155_TENANT.id,
    is_active: true,
    llm_roles: {},
    llm_resolution_source: "tenant",
    embedding_resolution_source: "tenant",
    vision_llm_resolution_source: "tenant",
  };

  /**
   * `GET /me/access` — the single rights read the create affordance depends on.
   *
   * Without this route the hook fails closed (`NO_ACCESS`) and no create row is rendered at all,
   * which is the correct behaviour but makes these tests unopenable. The payload mirrors
   * `Access` in `src/lib/access/use-access.ts`.
   *
   * `canManageProviders` models the real grant: only an organization-level manager may open the
   * provider offer. `workspace_admin` administers one workspace and must be *told* whom to ask —
   * the same distinction `WizardEligibilityNotice` draws.
   */
  await page.route("**/api/v1/me/access**", (route) => {
    if (route.request().method() !== "GET") return route.fallback();
    const canCreate = options.canCreateWorkspace !== false;
    const managesOrg = options.canManageProviders !== false;
    const orgCaps = [
      "organization.read",
      ...(managesOrg
        ? ["organization.settings.manage", "organization.members.manage"]
        : []),
      ...(canCreate ? ["organization.workspaces.create"] : []),
    ];
    return route.fulfill({
      json: {
        user_id: "spec179-user",
        platform: { role: "user", admin: false, capabilities: [] },
        organization: {
          id: SPEC155_TENANT.id,
          name: "SPEC155 Tenant",
          slug: "spec155-tenant",
          role: managesOrg ? "owner" : "member",
          level: managesOrg ? 4 : 2,
          is_active: true,
          is_archived: false,
          capabilities: orgCaps,
        },
        workspaces: [
          {
            id: SPEC155_WORKSPACE.id,
            name: "SPEC155 Workspace",
            slug: "spec155-workspace",
            role: "admin",
            level: 3,
            inherited: true,
            capabilities: [
              "workspace.read",
              "workspace.content.write",
              "workspace.settings.manage",
            ],
          },
        ],
        quota: { workspaces: { used: 3, limit: 10 }, seats: { used: 1, limit: 10 } },
        can_create: { organization: false, workspace: canCreate },
        can_delete: { organization: false },
        reason: {
          organization: null,
          workspace: canCreate
            ? null
            : "capability_required:organization.workspaces.create",
        },
      },
    });
  });

  await page.route(/\/api\/v1\/tenants\/[^/?]+\/?(\?.*)?$/, (route: Route) => {
    if (route.request().method() !== "GET") return route.fallback();
    const id = new URL(route.request().url()).pathname.split("/").filter(Boolean).at(-1);
    return route.fulfill({
      json: {
        id,
        name: "SPEC155 Tenant",
        slug: "spec155-tenant",
        plan: "free",
        default_llm_provider: defaults.llm[0],
        default_llm_model: defaults.llm[1],
        default_embedding_provider: defaults.embedding[0],
        default_embedding_model: defaults.embedding[1],
        default_embedding_dimension: defaults.embedding[2],
        default_vision_llm_provider: defaults.vision[0],
        default_vision_llm_model: defaults.vision[1],
      },
    });
  });

  await page.route("**/api/v1/models/eligible**", (route) => {
    const url = new URL(route.request().url());
    const usage = url.searchParams.get("usage") ?? "query";
    const forWorkspace = url.searchParams.has("workspace_id");
    recorder.eligibleAsks.push(url.search);
    const models = modelsFor(usage, forWorkspace);
    return route.fulfill({
      json: { usage, scope: forWorkspace ? "workspace" : "tenant", policy_mode: "constrained", models },
    });
  });

  // SPEC-180 08: the probe the model step asks before Review. Hermetic: no vendor call.
  await page.route("**/api/v1/models/embedding-support**", (route) => {
    const url = new URL(route.request().url());
    const provider = url.searchParams.get("provider") ?? "";
    const model = url.searchParams.get("model") ?? "";
    const card = offered.embedding.find((m) => m.provider === provider && m.model === model);
    const imageCapable = card?.supports_vision === true;
    const refused = imageCapable && options.imageProbe === "auth";
    return route.fulfill({
      json: {
        provider,
        model,
        eligible: Boolean(card),
        multimodal: imageCapable,
        embeds_images: imageCapable && !refused,
        code: refused ? "embedding_auth" : imageCapable ? null : "embedding_model_not_multimodal",
        message: refused
          ? "Vertex refused the image embedding call for project demo-project: embedContent returned 403 Forbidden. Grant this identity aiplatform.endpoints.predict, or set GOOGLE_CLOUD_PROJECT to the project that grants it."
          : imageCapable
            ? null
            : "this model reads text only; pick an image-capable embedding model or use Text only",
      },
    });
  });

  if (options.workspaceInheritsModels) {
    await page.route("**/api/v1/tenants/*/workspaces**", (route) => {
      if (route.request().method() !== "GET") return route.fallback();
      if (route.request().url().includes("/by-slug/")) {
        return route.fulfill({ json: inheritedWorkspace });
      }
      return route.fulfill({
        json: { items: [inheritedWorkspace], total: 1, offset: 0, limit: 20 },
      });
    });
  }

  await page.route(/\/api\/v1\/tenants\/[^/]+\/workspaces\/?$/, (route) => {
    if (route.request().method() !== "POST") return route.fallback();
    recorder.creates.push(route.request().postDataJSON());
    if (options.refuseCreate) {
      const { status, code, message } = options.refuseCreate;
      return route.fulfill({ status, contentType: "application/json", json: { code, message, status } });
    }
    return route.fulfill({
      status: 201,
      json: { id: "ws-new", tenant_id: "t", name: "New", slug: "new", is_active: true },
    });
  });

  await page.route(/\/api\/v1\/workspaces\/[^/?]+\/?(\?.*)?$/, (route) => {
    if (route.request().method() === "GET" && options.workspaceInheritsModels) {
      return route.fulfill({ json: inheritedWorkspace });
    }
    if (route.request().method() !== "PUT") return route.fallback();
    const body = route.request().postDataJSON() as Record<string, unknown>;
    recorder.updates.push(body);
    const id = new URL(route.request().url()).pathname.split("/").filter(Boolean).at(-1);
    return route.fulfill({ json: { id, name: "Workspace", slug: "workspace", is_active: true, ...body } });
  });
  return recorder;
}
