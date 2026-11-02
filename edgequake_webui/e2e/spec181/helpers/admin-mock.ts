/**
 * SPEC-181 hermetic mock: tenancy, access, quota, purge.
 *
 * Stateful on purpose. `E-success-clears-draft` and `E-two-grant-rows` are only meaningful if a
 * create and a grant actually change the next response. Contract: `handlers/me_access.rs`,
 * `handlers/workspaces/tenants.rs`, `services/tenancy/*`.
 *
 * Layered on the SPEC-168 rich mock, which already answers documents/graph/query, so the
 * shell renders and the switcher is populated.
 */

import type { Page, Route } from "@playwright/test";
import { prepareMembersPage } from "../../spec169/helpers/members-mock";
import { prepareSpec168Page } from "../../spec168/helpers/rich-mock";

const json = (data: unknown, status = 200) => ({
  status,
  contentType: "application/json",
  body: JSON.stringify(data),
});

/** The five personas the access model distinguishes. */
export type Viewer = "platform" | "owner" | "orgadmin" | "wsadmin" | "member" | "viewer";

export const SPEC181_ORG = {
  id: "spec155-tenant-001",
  name: "Acme Research",
  slug: "acme",
};

export const SPEC181_ORG_2 = {
  id: "spec181-org-globex",
  name: "Globex Holdings",
  slug: "globex",
};

export const SPEC181_WS = {
  legal: { id: "spec181-ws-legal", name: "Legal", slug: "legal" },
  finance: { id: "spec181-ws-finance", name: "Finance", slug: "finance" },
  research: { id: "spec181-ws-research", name: "Research", slug: "research" },
};

export interface AdminMockOptions {
  viewer?: Viewer;
  /** Compose the SPEC-169 members directory mock (needed by the grants tests). */
  withMembers?: boolean;
  /** Workspace budget. Defaults to 3 used of 10. */
  quota?: { used: number; limit: number };
  seats?: { used: number; limit: number };
  /** Zero organizations: exercises W-02. */
  noOrganizations?: boolean;
  noWorkspaces?: boolean;
  /** Force a specific failure on the next create. */
  failCreate?: "quota" | "slug" | "server";
  /** The model offer the create dialog reads. Defaults to two providers with three roles each. */
  modelOffer?: AdminModelOffer;
  theme?: "light" | "dark";
}

export interface AdminMockState {
  organizations: { id: string; name: string; slug: string; plan: string; max_workspaces: number; max_users: number }[];
  workspaces: { id: string; tenant_id: string; name: string; slug: string; archived_at: string | null }[];
  /** `METHOD path` of mutating calls, in order — assertions read this. */
  calls: { method: string; path: string; body: unknown }[];
  lastCreatedOrgId: string | null;
  lastCreatedWorkspaceId: string | null;
  purgeReceipts: Map<string, unknown>;
}

const CAPS = {
  organization: {
    owner: [
      "organization.read",
      "organization.settings.manage",
      "organization.members.manage",
      "organization.grant_owner",
      "organization.delete",
      "organization.workspaces.create",
    ],
    admin: [
      "organization.read",
      "organization.settings.manage",
      "organization.members.manage",
      "organization.workspaces.create",
    ],
    member: ["organization.read"],
    readonly: ["organization.read"],
  },
  workspace: {
    owner: [
      "workspace.read",
      "workspace.content.write",
      "workspace.settings.manage",
      "workspace.members.manage",
      "workspace.delete",
    ],
    admin: [
      "workspace.read",
      "workspace.content.write",
      "workspace.settings.manage",
      "workspace.members.manage",
    ],
    member: ["workspace.read", "workspace.content.write"],
    readonly: ["workspace.read"],
  },
} as const;

type Role = keyof typeof CAPS.organization;

const LEVEL: Record<Role, number> = { readonly: 1, member: 2, admin: 3, owner: 4 };

/** What `me/access` should answer for each persona. */
function accessFor(opts: AdminMockOptions) {
  const viewer = opts.viewer ?? "owner";
  const quota = opts.quota ?? { used: 3, limit: 10 };
  const seats = opts.seats ?? { used: 9, limit: 25 };
  const platformAdmin = viewer === "platform";
  const orgRole: Role | null =
    viewer === "owner" ? "owner" : viewer === "orgadmin" ? "admin" : viewer === "platform" ? "owner" : "member";
  const wsAdmin = viewer === "wsadmin";

  const orgCaps = platformAdmin
    ? CAPS.organization.owner
    : orgRole
      ? CAPS.organization[orgRole]
      : [];
  const quotaBlocked = quota.limit > 0 && quota.used >= quota.limit;

  const workspaces = opts.noWorkspaces
    ? []
    : [
        { ...SPEC181_WS.legal, inherited: !wsAdmin, role: wsAdmin ? "admin" : (orgRole ?? "member") },
        // A workspace admin sees only their own workspace; an org member sees all they inherit.
        ...(wsAdmin
          ? []
          : [
              { ...SPEC181_WS.finance, inherited: true, role: orgRole ?? "readonly" },
              { ...SPEC181_WS.research, inherited: true, role: orgRole ?? "readonly" },
            ]),
      ].map((w) => ({
        id: w.id,
        name: w.name,
        slug: w.slug,
        role: w.role,
        level: LEVEL[w.role as Role] ?? 1,
        inherited: w.inherited,
        capabilities: [...CAPS.workspace[(w.role as Role) ?? "readonly"]],
      }));

  return {
    user_id: "spec181-user",
    platform: {
      role: platformAdmin ? "admin" : "user",
      admin: platformAdmin,
      capabilities: platformAdmin
        ? [
            "platform.deployment.manage",
            "platform.organizations.list",
            "platform.organizations.create",
            "platform.organizations.delete",
          ]
        : [],
    },
    organization: opts.noOrganizations
      ? null
      : {
          id: SPEC181_ORG.id,
          name: SPEC181_ORG.name,
          slug: SPEC181_ORG.slug,
          role: orgRole,
          level: orgRole ? LEVEL[orgRole] : 0,
          is_active: true,
          is_archived: false,
          capabilities: [...orgCaps],
        },
    workspaces,
    quota: { workspaces: quota, seats },
    can_create: {
      organization: platformAdmin,
      workspace: orgCaps.includes("organization.workspaces.create") && !quotaBlocked,
    },
    can_delete: { organization: orgCaps.includes("organization.delete") },
    reason: {
      organization: platformAdmin ? undefined : "capability_required:platform.organizations.create",
      workspace: orgCaps.includes("organization.workspaces.create")
        ? quotaBlocked
          ? "workspace_quota_reached"
          : undefined
        : "capability_required:organization.workspaces.create",
    },
  };
}

const PLAN_QUOTA: Record<string, { max_workspaces: number; max_users: number }> = {
  free: { max_workspaces: 10, max_users: 3 },
  basic: { max_workspaces: 100, max_users: 10 },
  pro: { max_workspaces: 500, max_users: 50 },
  enterprise: { max_workspaces: 500, max_users: 0 },
};

/** The model offer the create dialog's pickers read. Two providers, so a change is visible. */
export interface AdminModelOffer {
  /** Pairs the organization may use. Empty means "no provider is offered". */
  models?: {
    provider: string;
    model: string;
    dimension?: number;
    vision?: boolean;
    /**
     * Offered for `extract` only, not for `query`/`summary`/`keyword`. A real shape: a model
     * good enough to extract but not to answer. The Extract picker lists it, and the wizard's
     * chat-family judgement then refuses it — which is what the dialog must surface.
     */
    extractOnly?: boolean;
  }[];
}

const DEFAULT_OFFER: NonNullable<AdminModelOffer["models"]> = [
  // The dialog's inherited vision resolves to the LLM pair, so both must be offered for VLM.
  { provider: "openai", model: "gpt-5-nano", vision: true },
  { provider: "openai", model: "gpt-6-luna", vision: true },
  { provider: "openai", model: "gpt-6-mini" },
  { provider: "ollama", model: "gemma3:12b", vision: true },
  { provider: "openai", model: "text-embedding-3-small", dimension: 1536 },
  { provider: "ollama", model: "embeddinggemma:latest", dimension: 768 },
];

/**
 * `GET /models/eligible` for the create dialog.
 *
 * Registering the generic `GET /api/v1/models` mock also matches `/models/eligible`, so a
 * specific route has to win. The answer mirrors the real DTO (`EligibleModelDto`), because the
 * dialog's row summaries and the pickers both read those fields.
 */
async function mockCreateDialogOffer(page: Page, offer: AdminModelOffer): Promise<void> {
  const models = offer.models ?? DEFAULT_OFFER;
  await page.route("**/api/v1/models/eligible**", async (route: Route) => {
    const usage = new URL(route.request().url()).searchParams.get("usage") ?? "query";
    const offered = models
      .filter((m) =>
        usage === "embedding" ? m.dimension !== undefined : m.dimension === undefined,
      )
      .filter((m) => (usage === "vlm" ? m.vision === true : true))
      .filter((m) => (m.extractOnly === true ? usage === "extract" : true))
      .map((m) => ({
        provider: m.provider,
        provider_display_name: m.provider === "openai" ? "OpenAI" : "Ollama",
        model: m.model,
        display_name: m.model,
        supports_vision: m.vision === true,
        supports_streaming: usage !== "embedding",
        embedding_dimension: m.dimension ?? null,
      }));
    await route.fulfill(
      json({ usage, scope: "tenant", policy_mode: "unconstrained", models: offered }),
    );
  });
}

/**
 * Install the mock. Returns the live state so a test can assert what the UI actually sent.
 */
export async function prepareAdminPage(
  page: Page,
  opts: AdminMockOptions = {},
): Promise<AdminMockState> {
  // SPEC-169 already owns the members directory contract (viewer, roles, invitations). Reuse it
  // instead of reimplementing a second directory shape — that duplication is what produced a
  // `viewer.instance_admin` crash on /members during round-19.
  if (opts.withMembers) {
    await prepareMembersPage(page, {
      viewerRole: opts.viewer === "member" || opts.viewer === "viewer" ? null : "owner",
    });
  } else {
    await prepareSpec168Page(page, { theme: opts.theme, empty: opts.noWorkspaces });
  }

  // After the SPEC-168 layer, so `/models/eligible` beats its generic `/models` route.
  await mockCreateDialogOffer(page, opts.modelOffer ?? {});

  const state: AdminMockState = {
    organizations: opts.noOrganizations
      ? []
      : [
          {
            ...SPEC181_ORG,
            plan: "free",
            max_workspaces: PLAN_QUOTA.free.max_workspaces,
            max_users: PLAN_QUOTA.free.max_users,
          },
          {
            ...SPEC181_ORG_2,
            plan: "pro",
            max_workspaces: PLAN_QUOTA.pro.max_workspaces,
            max_users: PLAN_QUOTA.pro.max_users,
          },
        ],
    workspaces: opts.noWorkspaces
      ? []
      : [
          { ...SPEC181_WS.legal, tenant_id: SPEC181_ORG.id, archived_at: null },
          { ...SPEC181_WS.finance, tenant_id: SPEC181_ORG.id, archived_at: null },
          { ...SPEC181_WS.research, tenant_id: SPEC181_ORG.id, archived_at: null },
        ],
    calls: [],
    lastCreatedOrgId: null,
    lastCreatedWorkspaceId: null,
    purgeReceipts: new Map(),
  };

  const access = accessFor(opts);

  // ── /me/access: the single rights read ────────────────────────────────────
  await page.route("**/api/v1/me/access**", async (route: Route) => {
    if (route.request().method() !== "GET") return route.fallback();
    await route.fulfill(json(access));
  });

  // ── tenants ───────────────────────────────────────────────────────────────
  await page.route("**/api/v1/tenants?**", async (route) => {
    if (route.request().method() !== "GET") return route.fallback();
    // Counters come from the live state so the usage meters are honest after a create.
    const items = state.organizations.map((o) => ({
      ...o,
      workspaces_used: state.workspaces.filter((w) => w.tenant_id === o.id && !w.archived_at).length,
      members_used: 1,
      my_role: "owner",
      is_active: true,
      archived_at: null,
    }));
    await route.fulfill(json({ items, total: items.length, offset: 0, limit: 100 }));
  });

  await page.route("**/api/v1/tenants", async (route) => {
    const req = route.request();
    if (req.method() !== "POST") return route.fallback();
    if (opts.failCreate === "slug") {
      await route.fulfill(json({ code: "CONFLICT", message: "tenant_slug_taken:acme" }, 409));
      return;
    }
    if (opts.failCreate === "server") {
      await route.fulfill(json({ code: "INTERNAL", message: "boom" }, 500));
      return;
    }
    const body = req.postDataJSON() as Record<string, unknown>;
    state.calls.push({ method: "POST", path: "/api/v1/tenants", body });
    const plan = (body.plan as string) ?? "free";
    const q = PLAN_QUOTA[plan] ?? PLAN_QUOTA.free;
    const created = {
      id: `spec181-org-created-${state.organizations.length + 1}`,
      name: String(body.name ?? "New org"),
      slug: String(body.slug ?? "new-org"),
      plan,
      max_workspaces: q.max_workspaces,
      max_users: q.max_users,
    };
    state.organizations.push(created);
    state.lastCreatedOrgId = created.id;
    state.workspaces.push({
      id: `spec181-ws-created-${state.workspaces.length + 1}`,
      tenant_id: created.id,
      name: "Default Workspace",
      slug: "default",
      archived_at: null,
    });
    await route.fulfill(
      json(
        {
          ...created,
          is_active: true,
          workspaces_used: 1,
          members_used: 1,
          default_llm_model: "gpt-5-nano",
          default_llm_provider: "openai",
          default_llm_full_id: "openai/gpt-5-nano",
          default_embedding_model: "text-embedding-3-small",
          default_embedding_provider: "openai",
          default_embedding_dimension: 1536,
          default_embedding_full_id: "openai/text-embedding-3-small",
          default_vision_llm_model: null,
          default_vision_llm_provider: null,
          primary_color: null,
          created_at: new Date().toISOString(),
          updated_at: new Date().toISOString(),
        },
        201,
      ),
    );
  });

  // ── workspaces of a tenant ────────────────────────────────────────────────
  await page.route("**/api/v1/tenants/*/workspaces**", async (route) => {
    const req = route.request();
    const url = new URL(req.url());
    const tenantId = url.pathname.split("/tenants/")[1]?.split("/")[0] ?? SPEC181_ORG.id;

    if (req.method() === "GET") {
      const rows = state.workspaces
        .filter((w) => w.tenant_id === tenantId && !w.archived_at)
        .map((w) => ({
          id: w.id,
          tenant_id: w.tenant_id,
          name: w.name,
          slug: w.slug,
          is_active: true,
          rag_profile: "graph",
          embedding_modalities: "text",
          fts_regconfig: "english",
          llm_model: "gpt-5-nano",
          llm_provider: "openai",
          llm_full_id: "openai/gpt-5-nano",
          embedding_model: "text-embedding-3-small",
          embedding_provider: "openai",
          embedding_dimension: 1536,
          embedding_full_id: "openai/text-embedding-3-small",
          created_at: "2026-01-01T00:00:00Z",
          updated_at: "2026-01-01T00:00:00Z",
        }));
      await route.fulfill(json({ items: rows, total: rows.length, offset: 0, limit: 100 }));
      return;
    }

    if (req.method() === "POST") {
      // Stateful: the budget must reflect creates already made in this test, not a static option.
      const liveUsed = state.workspaces.filter(
        (w) => w.tenant_id === tenantId && !w.archived_at,
      ).length;
      const limit = opts.quota?.limit ?? 10;
      if (opts.failCreate === "quota" || (limit > 0 && liveUsed >= limit)) {
        await route.fulfill(
          json({ code: "CONFLICT", message: "workspace_quota_reached: 10 of 10" }, 409),
        );
        return;
      }
      const body = req.postDataJSON() as Record<string, unknown>;
      state.calls.push({ method: "POST", path: `/api/v1/tenants/${tenantId}/workspaces`, body });
      const created = {
        id: `spec181-ws-new-${state.workspaces.length + 1}`,
        tenant_id: tenantId,
        name: String(body.name ?? "New workspace"),
        slug: String(body.slug ?? "new-workspace"),
        archived_at: null,
      };
      state.workspaces.push(created);
      state.lastCreatedWorkspaceId = created.id;
      await route.fulfill(
        json(
          {
            ...created,
            is_active: true,
            rag_profile: body.rag_profile ?? "graph",
            embedding_modalities: "text",
            fts_regconfig: "english",
            llm_model: "gpt-5-nano",
            llm_provider: "openai",
            llm_full_id: "openai/gpt-5-nano",
            embedding_model: "text-embedding-3-small",
            embedding_provider: "openai",
            embedding_dimension: 1536,
            embedding_full_id: "openai/text-embedding-3-small",
            created_at: new Date().toISOString(),
            updated_at: new Date().toISOString(),
          },
          201,
        ),
      );
      return;
    }
    await route.fallback();
  });

  // ── purge preview + delete ────────────────────────────────────────────────
  await page.route("**/api/v1/tenants/*/purge-preview", async (route) => {
    await route.fulfill(
      json({
        scope_kind: "organization",
        scope_id: SPEC181_ORG.id,
        name: SPEC181_ORG.name,
        workspaces: 3,
        documents: 412,
        entities: 1903,
        relationships: 2410,
        vector_rows: 8821,
        original_objects: 37,
      }),
    );
  });

  await page.route("**/api/v1/tenants/*", async (route) => {
    const req = route.request();
    if (req.method() === "DELETE") {
      const id = req.url().split("/tenants/")[1]?.split("?")[0] ?? "";
      state.calls.push({ method: "DELETE", path: `/api/v1/tenants/${id}`, body: null });
      const receipt = {
        receipt_id: "spec181-receipt-1",
        scope_kind: "organization",
        scope_id: id,
        tenant_id: id,
        status: "done",
        planes: Object.fromEntries(
          [
            "tasks",
            "vectors",
            "graph",
            "kv",
            "pdf_rows",
            "originals",
            "decision",
            "vector_table",
            "row",
          ].map((p) => [p, { status: "done" }]),
        ),
        planes_total: 9,
        planes_done: 9,
        started_at: new Date().toISOString(),
        finished_at: new Date().toISOString(),
      };
      state.purgeReceipts.set(receipt.receipt_id, receipt);
      await route.fulfill(json(receipt, 202));
      return;
    }
    await route.fallback();
  });

  // ── roles catalog (the matrix reads this) ─────────────────────────────────
  await page.route("**/api/v1/roles", async (route) => {
    const caps = [
      ["organization.read", "organization", "Organization", "View the organization", ["owner", "admin", "member", "readonly"]],
      ["organization.settings.manage", "organization", "Organization", "Manage settings and branding", ["owner", "admin"]],
      ["organization.members.manage", "organization", "Organization", "Invite and manage people", ["owner", "admin"]],
      ["organization.workspaces.create", "organization", "Organization", "Create and configure workspaces", ["owner", "admin"]],
      ["organization.grant_owner", "organization", "Organization", "Grant the owner role", ["owner"]],
      ["organization.delete", "organization", "Organization", "Delete the organization", ["owner"]],
      ["workspace.read", "workspace", "Workspace", "View documents, graph and answers", ["owner", "admin", "member", "readonly"]],
      ["workspace.content.write", "workspace", "Workspace", "Upload, edit and delete documents", ["owner", "admin", "member"]],
      ["workspace.settings.manage", "workspace", "Workspace", "Manage models, ingestion and labels", ["owner", "admin"]],
      ["workspace.members.manage", "workspace", "Workspace", "Invite and manage workspace members", ["owner", "admin"]],
      ["workspace.delete", "workspace", "Workspace", "Delete the workspace", ["owner"]],
    ].map(([id, axis, group, label, roles]) => ({ id, axis, group, label, roles }));

    await route.fulfill(
      json({
        organization_roles: [
          { id: "owner", label: "Owner", description: "Full control", level: 4 },
          { id: "admin", label: "Admin", description: "Manages members", level: 3 },
          { id: "member", label: "Member", description: "Works with knowledge", level: 2 },
          { id: "readonly", label: "Viewer", description: "Read-only", level: 1 },
        ],
        instance_roles: [
          { id: "admin", label: "Instance admin", description: "Platform", permission_count: 31 },
          { id: "user", label: "Standard", description: "Regular", permission_count: 19 },
          { id: "readonly", label: "Read-only", description: "Read guard", permission_count: 7 },
        ],
        capabilities: caps,
      }),
    );
  });

  // ── SPEC-181: workspace grants (the write path F-181-05 was missing) ───────
  await page.route("**/api/v1/tenants/*/grants**", async (route) => {
    const req = route.request();
    const url = new URL(req.url());
    const userId = url.searchParams.get("user_id");
    if (req.method() === "GET") {
      const base = userId ?? "spec181-user";
      await route.fulfill(
        json([
          {
            membership_id: "grant-org-1",
            user_id: base,
            scope: "organization",
            workspace_id: null,
            workspace_name: null,
            role: "admin",
            level: 3,
            is_active: true,
            sso_managed: false,
            joined_at: "2026-01-01T00:00:00Z",
          },
          {
            membership_id: "grant-ws-1",
            user_id: base,
            scope: "workspace",
            workspace_id: SPEC181_WS.legal.id,
            workspace_name: "Legal",
            role: "admin",
            level: 3,
            is_active: true,
            sso_managed: false,
            joined_at: "2026-01-01T00:00:00Z",
          },
        ]),
      );
      return;
    }
    if (req.method() === "POST") {
      state.calls.push({ method: "POST", path: "/grants", body: req.postDataJSON() });
      await route.fulfill(
        json({ membership_id: "grant-new", user_id: "x", scope: "workspace", role: "member", level: 2, is_active: true, sso_managed: false, joined_at: "2026-01-01T00:00:00Z" }, 201),
      );
      return;
    }
    state.calls.push({ method: req.method(), path: "/grants", body: req.postDataJSON?.() ?? null });
    await route.fulfill(req.method() === "DELETE" ? { status: 204, body: "" } : json({ ok: true }));
  });

  // ── SPEC-181: archive / restore ───────────────────────────────────────────
  await page.route("**/api/v1/workspaces/*/archive", async (route) => {
    const wsId = route.request().url().split("/workspaces/")[1]?.split("/")[0] ?? "";
    state.calls.push({ method: "POST", path: `/workspaces/${wsId}/archive`, body: route.request().postDataJSON?.() ?? null });
    const row = state.workspaces.find((w) => w.id === wsId);
    if (row) row.archived_at = new Date().toISOString();
    await route.fulfill(json({ id: wsId, name: row?.name ?? "Legal", slug: row?.slug ?? "legal", is_active: false, archived_at: new Date().toISOString() }));
  });

  return state;
}
