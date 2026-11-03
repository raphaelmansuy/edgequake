/**
 * SPEC-180 — Retrieval profile vocabulary (single source of truth for the UI).
 *
 * The server owns the rules. The UI needs the wire words it may send, the capability
 * booleans it may read, and the pure helpers below. No component compares the profile
 * name: it reads `rag_profile_capabilities` (the server's view) through these helpers.
 */

export const RAG_PROFILES = ['graph', 'normal'] as const;
export type RagProfile = (typeof RAG_PROFILES)[number];
export const DEFAULT_RAG_PROFILE: RagProfile = 'graph';

export const EMBEDDING_MODALITIES = ['text', 'text,image'] as const;
export type EmbeddingModalities = (typeof EMBEDDING_MODALITIES)[number];
export const DEFAULT_EMBEDDING_MODALITIES: EmbeddingModalities = 'text';

/** Text-search configurations the keyword index supports (closed list on the server). */
export const FTS_REGCONFIGS = ['english', 'simple', 'french', 'german', 'spanish'] as const;
export type FtsRegconfig = (typeof FTS_REGCONFIGS)[number];
export const DEFAULT_FTS_REGCONFIG: FtsRegconfig = 'english';

/**
 * Document kinds a normal workspace can filter on (wire values of `doc_kinds`).
 *
 * WHY no `office`: LAW-121-8 makes DOCX/XLSX/PPTX a non-goal for v1 — nothing in the ingest path
 * converts a word-processor file, so the supported product matrix is TXT, MD, JSON, PDF and raster
 * images, and an `office` chip can only ever filter a workspace down to zero results. The server
 * still accepts the word (its `DocKind` enum is the storage vocabulary), so adding it back is a
 * one-line change once an Office adapter exists and converts to Markdown.
 */
export const DOC_KINDS = ['pdf', 'image', 'markdown', 'text'] as const;
export type DocKind = (typeof DOC_KINDS)[number];

/** Read-only booleans the server sends on every workspace (`rag_profile_capabilities`). */
export interface RagProfileCapabilities {
  extracts_graph: boolean;
  embeds_images: boolean;
  queries_graph: boolean;
  allows_graph_modes: boolean;
  shows_graph_ui: boolean;
}

/** What a workspace without the field behaves like: every older workspace is a graph one. */
export const GRAPH_CAPABILITIES: RagProfileCapabilities = {
  extracts_graph: true,
  embeds_images: false,
  queries_graph: true,
  allows_graph_modes: true,
  shows_graph_ui: true,
};

/** Capabilities of a profile chosen in the wizard, before the server has answered. */
export function capabilitiesForDraft(
  profile: RagProfile,
  modalities: EmbeddingModalities,
): RagProfileCapabilities {
  if (profile === 'graph') return GRAPH_CAPABILITIES;
  return {
    extracts_graph: false,
    embeds_images: modalities === 'text,image',
    queries_graph: false,
    allows_graph_modes: false,
    shows_graph_ui: false,
  };
}

/** The workspace's capabilities, graph when the server did not say. */
export function capabilitiesOf(
  workspace: { rag_profile_capabilities?: RagProfileCapabilities } | null | undefined,
): RagProfileCapabilities {
  return workspace?.rag_profile_capabilities ?? GRAPH_CAPABILITIES;
}

export function parseRagProfile(raw: string | null | undefined): RagProfile {
  const word = (raw ?? '').trim().toLowerCase();
  return (RAG_PROFILES as readonly string[]).includes(word) ? (word as RagProfile) : 'graph';
}

export function parseEmbeddingModalities(raw: string | null | undefined): EmbeddingModalities {
  const word = (raw ?? '').replace(/\s+/g, '').toLowerCase();
  return (EMBEDDING_MODALITIES as readonly string[]).includes(word)
    ? (word as EmbeddingModalities)
    : 'text';
}

export function parseFtsRegconfig(raw: string | null | undefined): FtsRegconfig {
  const word = (raw ?? '').trim().toLowerCase();
  return (FTS_REGCONFIGS as readonly string[]).includes(word)
    ? (word as FtsRegconfig)
    : DEFAULT_FTS_REGCONFIG;
}

/** Fields the server fixes at create time. An update never sends them (EC-180-04). */
export const IMMUTABLE_PROFILE_FIELDS = [
  'rag_profile',
  'embedding_modalities',
  'fts_regconfig',
] as const;
