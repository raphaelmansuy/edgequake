/**
 * SPEC-175 — settings scopes, sections and the searchable index.
 *
 * Pure data and pure functions: no React, no fetch. The shell, the nav, the
 * search box and the Playwright gates all read this one registry (DRY).
 *
 * LAW-175-8: scope is the navigation. One section = one route = one h1.
 * EC-175-33: search runs on the index after the role filter.
 * EC-175-34 / EC-175-43: a section the caller cannot open is not offered.
 */

import type { SettingsAccess } from "./access";

export type SettingsScope = "you" | "workspace" | "organization" | "deployment";

export type SettingsSectionId =
  | "appearance"
  | "graph"
  | "browser-data"
  | "models"
  | "ingestion"
  | "access"
  | "people"
  | "workspaces"
  | "branding"
  | "org-providers"
  | "secrets"
  | "connections"
  | "providers"
  | "fleet"
  | "observability"
  | "organizations";

/**
 * Who may open a section.
 * - `everyone`: any signed-in member (read-only where the section writes)
 * - `workspaceEditor`: owner or admin of the selected workspace
 * - `tenantManage`: tenant-wide owner or admin (Super admin) or Platform admin
 * - `instanceAdmin`: Platform admin only
 */
export type SettingsGate = "everyone" | "workspaceEditor" | "tenantManage" | "instanceAdmin";

/**
 * How much of the content column a section may use (SPEC-175 shell width).
 *
 * - `form`: rows of label + one control. A long measure would leave the control
 *   stranded far from its label, so the column stays narrow.
 * - `wide`: grids, matrices and tag lists. These only become readable when the
 *   column grows, so the section claims the whole content area.
 *
 * The shell owns the number; the registry only names the intent, so a section
 * never hard-codes a pixel width (DRY, one place to change).
 */
export type SettingsSectionWidth = "form" | "wide";

export interface SettingsScopeDef {
  id: SettingsScope;
  label: string;
  fallback: string;
  gate: SettingsGate;
  /** First section of the scope. The scope redirects here. */
  defaultSection: SettingsSectionId;
}

export interface SettingsSectionDef {
  id: SettingsSectionId;
  scope: SettingsScope;
  label: string;
  /** Path segment under the scope base, e.g. `models`. */
  segment: string;
  gate: SettingsGate;
  /** Words that find this section in search, beyond its label. */
  keywords: readonly string[];
  /** Defaults to `form`; only matrices and card galleries opt into `wide`. */
  width?: SettingsSectionWidth;
}

export const SETTINGS_SCOPES: readonly SettingsScopeDef[] = [
  { id: "you", label: "You", fallback: "You", gate: "everyone", defaultSection: "appearance" },
  { id: "workspace", label: "Workspace", fallback: "Workspace", gate: "everyone", defaultSection: "models" },
  { id: "organization", label: "Organization", fallback: "Organization", gate: "tenantManage", defaultSection: "people" },
  { id: "deployment", label: "Deployment", fallback: "Deployment", gate: "instanceAdmin", defaultSection: "connections" },
];

export const SETTINGS_SECTIONS: readonly SettingsSectionDef[] = [
  {
    id: "appearance",
    scope: "you",
    label: "Appearance",
    segment: "appearance",
    gate: "everyone",
    keywords: ["theme", "dark", "light", "system", "language", "locale", "color", "display"],
  },
  {
    id: "graph",
    scope: "you",
    label: "Graph display",
    segment: "graph",
    gate: "everyone",
    keywords: ["graph", "labels", "node size", "layout", "edges", "visualization"],
  },
  {
    id: "browser-data",
    scope: "you",
    label: "Browser data",
    segment: "browser-data",
    gate: "everyone",
    keywords: ["export", "import", "backup", "history", "reset", "local", "clear"],
  },
  {
    id: "models",
    scope: "workspace",
    label: "Models",
    segment: "models",
    // Provider tag lists, the usage table and the 7-job narrowing matrix all need room.
    width: "wide",
    gate: "everyone",
    keywords: [
      "model", "models", "llm", "extract", "query", "answer", "summary", "vision", "vlm",
      "keyword", "embedding", "embed", "decision", "effort", "inherit", "provider", "connection",
    ],
  },
  {
    id: "ingestion",
    scope: "workspace",
    label: "Ingestion",
    segment: "ingestion",
    // Two editors side by side once the column is wide enough.
    width: "wide",
    gate: "workspaceEditor",
    keywords: ["pdf", "parser", "chunk", "chunking", "budget", "entity types", "language", "vision prompt", "gleaning"],
  },
  {
    id: "access",
    scope: "workspace",
    label: "Access",
    segment: "access",
    // Profile list rows with summaries benefit from a wider measure.
    width: "wide",
    gate: "workspaceEditor",
    keywords: ["labels", "clearance", "document access", "vocabulary", "profile", "enforcement"],
  },
  {
    id: "people",
    scope: "organization",
    label: "People",
    segment: "people",
    gate: "tenantManage",
    keywords: ["people", "members", "invite", "roles", "sso", "seats"],
  },
  {
    id: "workspaces",
    scope: "organization",
    label: "Workspaces",
    segment: "workspaces",
    // A list with usage meters and per-row actions needs the wider column.
    width: "wide",
    gate: "tenantManage",
    keywords: ["workspace", "workspaces", "create", "archive", "quota", "plan", "limit"],
  },
  {
    id: "branding",
    scope: "organization",
    label: "Branding",
    segment: "branding",
    gate: "tenantManage",
    keywords: ["brand", "branding", "color", "colour", "primary", "accent", "theme", "tenant", "organization"],
  },
  {
    id: "org-providers",
    scope: "organization",
    label: "Providers",
    segment: "providers",
    // Provider tag lists plus the 7-job ceiling matrix.
    width: "wide",
    gate: "tenantManage",
    keywords: ["provider", "providers", "ceiling", "model", "anthropic", "openai", "offer"],
  },
  {
    id: "secrets",
    scope: "organization",
    label: "Secrets",
    segment: "secrets",
    gate: "tenantManage",
    keywords: ["secret", "secrets", "infisical", "api key", "vault", "encryption", "master key"],
  },
  {
    id: "connections",
    scope: "deployment",
    label: "Connections",
    segment: "connections",
    // Form fields in two columns, plus the saved-connection list.
    width: "wide",
    gate: "instanceAdmin",
    keywords: ["connections", "api key", "base url", "provider key", "test", "secret"],
  },
  {
    id: "providers",
    scope: "deployment",
    label: "Providers",
    segment: "providers",
    // A gallery of provider cards with model chips.
    width: "wide",
    gate: "instanceAdmin",
    keywords: ["provider", "offer", "withdraw", "model", "fleet provider"],
  },
  {
    id: "fleet",
    scope: "deployment",
    label: "Fleet",
    segment: "fleet",
    // Provider health rows carry a lot of detail when expanded.
    width: "wide",
    gate: "instanceAdmin",
    keywords: ["fleet", "server defaults", "health", "provider health", "priority", "env", "status"],
  },
  {
    id: "organizations",
    scope: "deployment",
    label: "Organizations",
    segment: "organizations",
    width: "wide",
    gate: "instanceAdmin",
    keywords: ["organizations", "tenants", "customers", "quota", "plan", "create", "delete"],
  },
  {
    id: "observability",
    scope: "deployment",
    label: "Observability",
    segment: "observability",
    gate: "instanceAdmin",
    keywords: ["langfuse", "attribution", "quota", "sso", "app name", "tracing"],
  },
];

export const SETTINGS_SECTION_IDS: readonly SettingsSectionId[] = SETTINGS_SECTIONS.map((s) => s.id);

export function getSection(id: SettingsSectionId): SettingsSectionDef {
  const found = SETTINGS_SECTIONS.find((s) => s.id === id);
  if (!found) throw new Error(`Unknown settings section: ${id}`);
  return found;
}

export function getScope(id: SettingsScope): SettingsScopeDef {
  const found = SETTINGS_SCOPES.find((s) => s.id === id);
  if (!found) throw new Error(`Unknown settings scope: ${id}`);
  return found;
}

/** Sections of one scope, in nav order. */
export function sectionsOf(scope: SettingsScope): readonly SettingsSectionDef[] {
  return SETTINGS_SECTIONS.filter((s) => s.scope === scope);
}

/** The width intent of a section; `form` unless the section opted into `wide`. */
export function sectionWidth(id: SettingsSectionId): SettingsSectionWidth {
  return getSection(id).width ?? "form";
}

/**
 * The content-column width, in pixels, for one width intent.
 *
 * `form` caps the measure so a label and its control never drift apart.
 * `wide` gives the section everything the viewport can spare, up to a ceiling
 * that keeps prose lines readable on very large monitors.
 */
export const SETTINGS_CONTENT_WIDTH: Record<SettingsSectionWidth, number> = {
  form: 720,
  wide: 1600,
};

/**
 * Href for a section. Workspace sections use `/w/{slug}/settings/...` when the
 * shell was opened by slug (deeplink), otherwise the header-selected workspace
 * at `/settings/workspace/...`.
 */
export function sectionHref(id: SettingsSectionId, workspaceSlug?: string | null): string {
  const section = getSection(id);
  if (section.scope === "workspace") {
    if (workspaceSlug) {
      return section.id === "models"
        ? `/w/${encodeURIComponent(workspaceSlug)}/settings`
        : `/w/${encodeURIComponent(workspaceSlug)}/settings/${section.segment}`;
    }
    return `/settings/workspace/${section.segment}`;
  }
  return `/settings/${section.scope}/${section.segment}`;
}

/** Route used when a caller opens a scope without a section. */
export function scopeHref(scope: SettingsScope, workspaceSlug?: string | null): string {
  return sectionHref(getScope(scope).defaultSection, workspaceSlug);
}

/** Whether the caller may open this section. Pure over `SettingsAccess`. */
export function canOpenSection(access: SettingsAccess, section: SettingsSectionDef): boolean {
  return gatePasses(access, section.gate);
}

export function gatePasses(access: SettingsAccess, gate: SettingsGate): boolean {
  switch (gate) {
    case "everyone":
      return true;
    case "workspaceEditor":
      return access.workspaceManage;
    case "tenantManage":
      return access.tenantManage;
    case "instanceAdmin":
      return access.instanceAdmin;
  }
}

export interface SettingsRouteMatch {
  section: SettingsSectionId;
  /** Set for `/w/{slug}/settings` deeplinks. */
  workspaceSlug: string | null;
  /** True for a bare scope path such as `/settings/workspace`; it canonicalizes to the section URL. */
  bare?: boolean;
}

/**
 * Map a pathname to the section it shows. `null` for paths outside settings.
 * A bare scope path maps to that scope's default section, so the layout can
 * redirect it before rendering.
 */
export function sectionFromPathname(pathname: string): SettingsRouteMatch | null {
  const path = pathname.replace(/\/+$/, "");
  const deeplink = path.match(/^\/w\/([^/]+)\/settings(?:\/([^/]+))?$/);
  if (deeplink) {
    const slug = decodeURIComponent(deeplink[1]);
    const segment = deeplink[2];
    const section = segment
      ? SETTINGS_SECTIONS.find((s) => s.scope === "workspace" && s.segment === segment)
      : getSection("models");
    return section ? { section: section.id, workspaceSlug: slug } : null;
  }
  const scoped = path.match(/^\/settings\/([^/]+)(?:\/([^/]+))?$/);
  if (!scoped) return null;
  const scope = SETTINGS_SCOPES.find((s) => s.id === scoped[1]);
  if (!scope) return null;
  const segment = scoped[2];
  if (!segment) return { section: scope.defaultSection, workspaceSlug: null, bare: true };
  const section = SETTINGS_SECTIONS.find((s) => s.scope === scope.id && s.segment === segment);
  return section ? { section: section.id, workspaceSlug: null } : null;
}

/** Scopes with at least one section the caller can open. Nav and search read this. */
export function visibleScopes(access: SettingsAccess): readonly SettingsScopeDef[] {
  return SETTINGS_SCOPES.filter((scope) => {
    if (!gatePasses(access, scope.gate)) return false;
    return sectionsOf(scope.id).some((s) => canOpenSection(access, s));
  });
}

export function visibleSections(access: SettingsAccess, scope: SettingsScope): readonly SettingsSectionDef[] {
  return sectionsOf(scope).filter((s) => canOpenSection(access, s));
}

export interface SettingsSearchHit {
  section: SettingsSectionDef;
  /** Usage anchor inside the section, when the match is a specific row. */
  anchor?: string;
}

/**
 * Search the index. Matches label, scope label and keywords. Hidden sections are
 * removed before matching, so a forbidden section cannot be a result (EC-175-43).
 */
export function searchSections(
  access: SettingsAccess,
  query: string,
): readonly SettingsSearchHit[] {
  const q = query.trim().toLowerCase();
  if (!q) return [];
  const terms = q.split(/\s+/);
  return SETTINGS_SECTIONS.filter((s) => canOpenSection(access, s))
    .filter((s) => {
      const haystack = [s.label, getScope(s.scope).fallback, ...s.keywords].join(" ").toLowerCase();
      return terms.every((term) => haystack.includes(term));
    })
    .map((section) => ({ section }));
}

/**
 * Breadcrumb label for one path segment inside settings, or null when the segment is not a
 * settings name. Position-aware: `graph` under settings is "Graph display", not the
 * top-level "Knowledge Graph" page.
 */
export function settingsCrumbLabel(segments: readonly string[], index: number): string | null {
  const at = segments.indexOf("settings");
  if (at === -1 || index <= at) return null;
  const relative = index - at;
  const segment = segments[index];
  if (segments[0] === "settings" && relative === 1) {
    return SETTINGS_SCOPES.find((s) => s.id === segment)?.fallback ?? null;
  }
  const sectionPosition = segments[0] === "settings" ? 2 : 1;
  if (relative === sectionPosition) {
    return SETTINGS_SECTIONS.find((s) => s.segment === segment)?.label ?? null;
  }
  return null;
}
