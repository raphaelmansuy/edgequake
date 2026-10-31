/**
 * SPEC-101 — Build create-tenant / create-workspace / reconfigure model payloads.
 * When useServerDefaults is true on create, omit model fields so the inheritance ladder applies.
 * On update, empty strings clear workspace overrides (SPEC-013).
 */

import { visionChoicePatch } from '@/lib/workspace/vision-choice';
import type { UpdateWorkspaceRequest } from '@/lib/api/edgequake/workspaces';
import type { PdfParserBackendDraft, WizardDraft } from '@/lib/onboarding/wizard-state';
import {
  profileCreateFields,
  withoutGraphFields,
} from '@/lib/onboarding/rag-profile-payload';
import type { WorkspaceConfigChangedKey } from '@/lib/onboarding/workspace-config-diff';
import type { Workspace } from '@/types/workspace';
import { extractionLanguageToUpdatePayload } from '@/constants/extraction-languages';
import {
  ACC_FAIR_CHUNK_OVERLAP,
  ACC_FAIR_CHUNK_TOKEN_SIZE,
  chunkingToUpdatePayload,
  parseChunkingMode,
  type ChunkingMode,
} from '@/constants/chunking-policy';
import {
  extractBudgetToUpdatePayload,
  LIGHTRAG_EXTRACT_MAX_ENTITIES,
  LIGHTRAG_EXTRACT_MAX_RECORDS,
  parseExtractBudgetMode,
  type ExtractBudgetMode,
} from '@/constants/extract-budget';

export interface ModelSelectionSlash {
  provider: string;
  model: string;
  fullId?: string;
  dimension?: number;
}

export interface TenantModelPayload {
  default_llm_model?: string;
  default_llm_provider?: string;
  default_embedding_model?: string;
  default_embedding_provider?: string;
  default_embedding_dimension?: number;
  default_vision_llm_model?: string;
  default_vision_llm_provider?: string;
  /** SPEC-109: seed fleet/workspace default reasoning effort */
  default_reasoning_effort?: string;
}

export interface WorkspaceModelPayload {
  llm_model?: string;
  llm_provider?: string;
  embedding_model?: string;
  embedding_provider?: string;
  embedding_dimension?: number;
  vision_llm_model?: string;
  vision_llm_provider?: string;
  /** SPEC-109: workspace metadata default_reasoning_effort */
  default_reasoning_effort?: string;
}

export function buildTenantModelPayload(args: {
  useServerDefaults: boolean;
  llm?: ModelSelectionSlash;
  embedding?: ModelSelectionSlash;
  vision?: ModelSelectionSlash;
  reasoningEffort?: string;
}): TenantModelPayload {
  if (args.useServerDefaults) {
    return {};
  }
  const payload: TenantModelPayload = {};
  if (args.llm?.provider && args.llm?.model) {
    payload.default_llm_provider = args.llm.provider;
    payload.default_llm_model = args.llm.model;
  }
  if (args.embedding?.provider && args.embedding?.model) {
    payload.default_embedding_provider = args.embedding.provider;
    payload.default_embedding_model = args.embedding.model;
    if (typeof args.embedding.dimension === 'number' && args.embedding.dimension > 0) {
      payload.default_embedding_dimension = args.embedding.dimension;
    }
  }
  if (args.vision?.provider && args.vision?.model) {
    payload.default_vision_llm_provider = args.vision.provider;
    payload.default_vision_llm_model = args.vision.model;
  }
  if (args.reasoningEffort?.trim()) {
    payload.default_reasoning_effort = args.reasoningEffort.trim();
  }
  return payload;
}

export function buildWorkspaceModelPayload(args: {
  useServerDefaults: boolean;
  llm?: ModelSelectionSlash;
  embedding?: ModelSelectionSlash;
  vision?: ModelSelectionSlash;
  reasoningEffort?: string;
}): WorkspaceModelPayload {
  if (args.useServerDefaults) {
    return {};
  }
  const payload: WorkspaceModelPayload = {};
  if (args.llm?.provider && args.llm?.model) {
    payload.llm_provider = args.llm.provider;
    payload.llm_model = args.llm.model;
  }
  if (args.embedding?.provider && args.embedding?.model) {
    payload.embedding_provider = args.embedding.provider;
    payload.embedding_model = args.embedding.model;
    // The server copies the dimension from the model card; the client never guesses it.
  }
  if (args.vision?.provider && args.vision?.model) {
    payload.vision_llm_provider = args.vision.provider;
    payload.vision_llm_model = args.vision.model;
  }
  if (args.reasoningEffort?.trim()) {
    payload.default_reasoning_effort = args.reasoningEffort.trim();
  }
  return payload;
}

export interface WorkspaceUpdatePayloadArgs {
  useServerDefaults: boolean;
  llm?: ModelSelectionSlash;
  embedding?: ModelSelectionSlash;
  vision?: ModelSelectionSlash;
  /** The workspace's saved `llm_roles`, so a vision save keeps the other roles. */
  savedLlmRoles?: Workspace['llm_roles'];
  pdfParserBackend: PdfParserBackendDraft;
  /** SPEC-015V */
  visionExtractImages?: boolean;
  visionExtractCharts?: boolean;
  visionExtractFigures?: boolean;
  visionPageSystemPrompt?: string;
  visionImageSystemPrompt?: string;
  visionChartSystemPrompt?: string;
  visionFigureSystemPrompt?: string;
  extractionLanguage: string | null;
  /** SPEC-116 */
  chunkingMode?: ChunkingMode | null;
  chunkTokenSize?: number | null;
  chunkOverlapTokenSize?: number | null;
  /** SPEC-117 */
  extractBudgetMode?: ExtractBudgetMode | null;
  extractMaxEntities?: number | null;
  extractMaxRecords?: number | null;
  entityTypes: string[];
  entityTypesStrict: boolean;
  entityTypeColors?: Record<string, string>;
  /** SPEC-114 / 114b */
  relationTypes?: string[];
  relationTypesStrict?: boolean;
  kgSchemaPreset?: string;
  relationEdges?: Array<{ source: string; relation: string; target: string }>;
  reasoningEffort?: string;
  decisionEnabled?: boolean;
  decisionModel?: string;
  decisionPreset?: string;
  decisionPackSize?: string;
  extractionMode?: 'inherit' | 'llm' | 'decision';
  /**
   * Sparse PUT: only emit these keys. When omitted:
   * - useServerDefaults → model-clear fields only (never a schema wipe)
   * - overrides → full snapshot (tests / callers without a diff)
   */
  changedKeys?: WorkspaceConfigChangedKey[];
}

function includeKey(
  changedKeys: WorkspaceConfigChangedKey[] | undefined,
  key: WorkspaceConfigChangedKey,
  opts?: { whenNoDiff?: boolean },
): boolean {
  if (!changedKeys) return opts?.whenNoDiff !== false;
  return changedKeys.includes(key);
}

/**
 * SPEC-101 Wave 8 — Build PUT /workspaces/{id} payload.
 * Empty strings clear LLM/embedding/vision overrides → server defaults (SPEC-013).
 * Sparse when `changedKeys` is set so a language-only save cannot promote painted models.
 */
export function buildWorkspaceUpdatePayload(
  args: WorkspaceUpdatePayloadArgs,
): UpdateWorkspaceRequest {
  const payload: UpdateWorkspaceRequest = {};
  const sparse = Boolean(args.changedKeys);
  const include = (key: WorkspaceConfigChangedKey, whenNoDiff = true) =>
    includeKey(args.changedKeys, key, { whenNoDiff });

  const modelSliceRequested =
    include('llm') || include('embedding') || include('vision');
  // Inherit-defaults with no diff: only clear models (EC-101-19), never wipe schema.
  const emitModelClears =
    args.useServerDefaults && (sparse ? modelSliceRequested : true);

  // One vision write path: `llm_roles.vlm` plus cleared legacy fields (SPEC-163).
  if (include('vision')) {
    Object.assign(
      payload,
      visionChoicePatch(args.savedLlmRoles, args.useServerDefaults ? undefined : args.vision),
    );
  }

  if (emitModelClears) {
    if (include('llm')) {
      payload.llm_model = '';
      payload.llm_provider = '';
    }
    if (include('embedding')) {
      payload.embedding_model = '';
      payload.embedding_provider = '';
      payload.embedding_dimension = 0;
    }
    if (!sparse) {
      payload.llm_model = '';
      payload.llm_provider = '';
      payload.embedding_model = '';
      payload.embedding_provider = '';
      payload.embedding_dimension = 0;
      return payload;
    }
  } else if (!args.useServerDefaults) {
    if (include('llm')) {
      payload.llm_model = args.llm?.model ?? '';
      payload.llm_provider = args.llm?.provider ?? '';
    }
    if (include('embedding')) {
      payload.embedding_model = args.embedding?.model ?? '';
      payload.embedding_provider = args.embedding?.provider ?? '';
    }
  }

  if (include('pdfParser')) {
    payload.pdf_parser_backend = args.pdfParserBackend;
  }
  if (include('visionExtract')) {
    payload.vision_extract_images = args.visionExtractImages ?? true;
    payload.vision_extract_charts = args.visionExtractCharts ?? true;
    payload.vision_extract_figures = args.visionExtractFigures ?? true;
    payload.vision_page_system_prompt = args.visionPageSystemPrompt ?? '';
    payload.vision_image_system_prompt = args.visionImageSystemPrompt ?? '';
    payload.vision_chart_system_prompt = args.visionChartSystemPrompt ?? '';
    payload.vision_figure_system_prompt = args.visionFigureSystemPrompt ?? '';
  }
  if (include('extractionLanguage')) {
    payload.extraction_language = extractionLanguageToUpdatePayload(
      args.extractionLanguage,
    );
  }
  if (include('chunking')) {
    Object.assign(
      payload,
      chunkingToUpdatePayload({
        mode: parseChunkingMode(args.chunkingMode),
        size: args.chunkTokenSize ?? ACC_FAIR_CHUNK_TOKEN_SIZE,
        overlap: args.chunkOverlapTokenSize ?? ACC_FAIR_CHUNK_OVERLAP,
      }),
    );
  }
  if (include('extractBudget')) {
    Object.assign(
      payload,
      extractBudgetToUpdatePayload({
        mode: parseExtractBudgetMode(
          args.extractBudgetMode,
          typeof args.extractMaxEntities === 'number' &&
            args.extractBudgetMode === 'custom',
        ),
        entities: args.extractMaxEntities ?? LIGHTRAG_EXTRACT_MAX_ENTITIES,
        records: args.extractMaxRecords ?? LIGHTRAG_EXTRACT_MAX_RECORDS,
      }),
    );
  }
  if (include('entityTypes')) {
    payload.entity_types = args.entityTypes;
  }
  if (include('entityTypesStrict')) {
    payload.entity_types_strict = args.entityTypesStrict;
  }
  if (include('entityTypeColors')) {
    payload.entity_type_colors = args.entityTypeColors ?? {};
  }
  if (include('relationTypes')) {
    payload.relation_types = args.relationTypes ?? [];
  }
  if (include('relationTypesStrict')) {
    payload.relation_types_strict = args.relationTypesStrict ?? true;
  }
  if (include('kgSchemaPreset')) {
    payload.kg_schema_preset = args.kgSchemaPreset?.trim() || 'custom';
  }
  if (include('relationEdges')) {
    payload.relation_edges = args.relationEdges ?? [];
  }
  if (include('reasoningEffort')) {
    const effort = args.reasoningEffort?.trim() ?? '';
    payload.default_reasoning_effort = effort.length > 0 ? effort : 'none';
  }
  if (include('decisionEngine')) {
    payload.decision_enabled = args.decisionEnabled === true;
    payload.decision_model = args.decisionModel?.trim() || 'inherit';
    payload.decision_gate_preset = args.decisionPreset?.trim() || 'inherit';
    const pack = args.decisionPackSize?.trim() ?? '';
    payload.decision_pack_size = pack === '' ? 0 : Number(pack);
  }
  if (include('extractionMode')) {
    payload.extraction_mode = args.extractionMode || 'inherit';
  }

  return payload;
}

export type WorkspaceIngestMode = 'create' | 'update';

/**
 * Shared ingest fields for create workspace / create tenant / first-run.
 * Inherit-equivalent values are omitted on create so the ladder applies.
 */
export function buildWorkspaceIngestPayload(
  draft: Pick<
    WizardDraft,
    | 'ragProfile'
    | 'embeddingModalities'
    | 'ftsRegconfig'
    | 'pdfParserBackend'
    | 'visionExtractImages'
    | 'visionExtractCharts'
    | 'visionExtractFigures'
    | 'visionPageSystemPrompt'
    | 'visionImageSystemPrompt'
    | 'visionChartSystemPrompt'
    | 'visionFigureSystemPrompt'
    | 'extractionLanguage'
    | 'chunkingMode'
    | 'chunkTokenSize'
    | 'chunkOverlapTokenSize'
    | 'extractBudgetMode'
    | 'extractMaxEntities'
    | 'extractMaxRecords'
    | 'entityTypes'
    | 'entityTypesStrict'
    | 'entityTypeColors'
    | 'relationTypes'
    | 'relationTypesStrict'
    | 'kgSchemaPreset'
    | 'relationEdges'
    | 'reasoningEffort'
    | 'decisionEnabled'
    | 'decisionModel'
    | 'decisionPreset'
    | 'decisionPackSize'
    | 'extractionMode'
  >,
  mode: WorkspaceIngestMode = 'create',
): UpdateWorkspaceRequest {
  const graphPayload = buildGraphIngestPayload(draft, mode);
  if (mode !== 'create') return graphPayload;
  // SPEC-180: the profile is chosen once, at create. A normal draft drops graph-only keys.
  const base = draft.ragProfile === 'normal' ? withoutGraphFields(graphPayload) : graphPayload;
  return { ...base, ...profileCreateFields(draft) };
}

function buildGraphIngestPayload(
  draft: Parameters<typeof buildWorkspaceIngestPayload>[0],
  mode: WorkspaceIngestMode,
): UpdateWorkspaceRequest {
  const payload: UpdateWorkspaceRequest = {};
  const parser = draft.pdfParserBackend;
  if (parser && parser !== 'none') {
    payload.pdf_parser_backend = parser;
  } else if (mode === 'update') {
    payload.pdf_parser_backend = 'none';
  }

  const visionNonDefault =
    draft.visionExtractImages === false ||
    draft.visionExtractCharts === false ||
    draft.visionExtractFigures === false ||
    Boolean(draft.visionPageSystemPrompt?.trim()) ||
    Boolean(draft.visionImageSystemPrompt?.trim()) ||
    Boolean(draft.visionChartSystemPrompt?.trim()) ||
    Boolean(draft.visionFigureSystemPrompt?.trim());
  if (visionNonDefault || mode === 'update') {
    payload.vision_extract_images = draft.visionExtractImages;
    payload.vision_extract_charts = draft.visionExtractCharts;
    payload.vision_extract_figures = draft.visionExtractFigures;
    payload.vision_page_system_prompt = draft.visionPageSystemPrompt ?? '';
    payload.vision_image_system_prompt = draft.visionImageSystemPrompt ?? '';
    payload.vision_chart_system_prompt = draft.visionChartSystemPrompt ?? '';
    payload.vision_figure_system_prompt = draft.visionFigureSystemPrompt ?? '';
  }

  if (draft.extractionLanguage) {
    payload.extraction_language = draft.extractionLanguage;
  } else if (mode === 'update') {
    payload.extraction_language = extractionLanguageToUpdatePayload(null);
  }

  if (draft.chunkingMode && draft.chunkingMode !== 'inherit') {
    Object.assign(
      payload,
      chunkingToUpdatePayload({
        mode: parseChunkingMode(draft.chunkingMode),
        size: draft.chunkTokenSize,
        overlap: draft.chunkOverlapTokenSize,
      }),
    );
  } else if (mode === 'update') {
    payload.chunking_mode = 'inherit';
  }

  if (draft.extractBudgetMode === 'custom') {
    Object.assign(
      payload,
      extractBudgetToUpdatePayload({
        mode: 'custom',
        entities: draft.extractMaxEntities,
        records: draft.extractMaxRecords,
      }),
    );
  } else if (mode === 'update') {
    payload.extract_budget_mode = 'inherit';
  }

  if (draft.entityTypes.length > 0) {
    payload.entity_types = draft.entityTypes;
  }
  payload.entity_types_strict = draft.entityTypesStrict;
  if (Object.keys(draft.entityTypeColors ?? {}).length > 0) {
    payload.entity_type_colors = draft.entityTypeColors;
  }
  if ((draft.relationTypes ?? []).length > 0) {
    payload.relation_types = draft.relationTypes;
  }
  payload.relation_types_strict = draft.relationTypesStrict;
  if (draft.kgSchemaPreset?.trim()) {
    payload.kg_schema_preset = draft.kgSchemaPreset.trim();
  }
  if ((draft.relationEdges ?? []).length > 0) {
    payload.relation_edges = draft.relationEdges;
  }
  if (draft.reasoningEffort?.trim()) {
    payload.default_reasoning_effort = draft.reasoningEffort.trim();
  }
  if (draft.extractionMode === 'llm' || draft.extractionMode === 'decision') {
    payload.extraction_mode = draft.extractionMode;
  }
  if (draft.decisionEnabled === false) {
    payload.decision_enabled = false;
  } else if (draft.decisionEnabled === true) {
    payload.decision_enabled = true;
  }
  const decisionModel = draft.decisionModel?.trim();
  if (decisionModel && decisionModel !== 'inherit') {
    payload.decision_model = decisionModel;
  }
  if (draft.decisionPreset && draft.decisionPreset !== 'inherit') {
    payload.decision_gate_preset = draft.decisionPreset;
  }
  const pack = draft.decisionPackSize?.trim() ?? '';
  if (pack !== '') {
    payload.decision_pack_size = Number(pack);
  }
  return payload;
}

/** Normalize legacy `provider:model` to slash form. */
export function normalizeModelFullId(value: string): { provider: string; model: string; fullId: string } {
  const slash = value.indexOf('/');
  if (slash !== -1) {
    const provider = value.slice(0, slash);
    const model = value.slice(slash + 1);
    return { provider, model, fullId: `${provider}/${model}` };
  }
  const colon = value.indexOf(':');
  if (colon !== -1) {
    const provider = value.slice(0, colon);
    const model = value.slice(colon + 1);
    return { provider, model, fullId: `${provider}/${model}` };
  }
  return { provider: 'unknown', model: value, fullId: `unknown/${value}` };
}
