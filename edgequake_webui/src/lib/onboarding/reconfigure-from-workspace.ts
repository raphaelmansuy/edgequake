/**
 * SPEC-101 Wave 8 — Prefill reconfigure wizard from an existing Workspace.
 */

import {
  ACC_FAIR_CHUNK_OVERLAP,
  ACC_FAIR_CHUNK_TOKEN_SIZE,
  parseChunkingMode,
} from '@/constants/chunking-policy';
import { ENTITY_PRESETS, type PresetKey } from '@/constants/entity-presets';
import {
  EDGE_PRESETS,
  RELATION_PRESETS,
  detectKgSchemaPreset,
  type RelationEdge,
} from '@/constants/kg-schema-presets';
import {
  parseEmbeddingModalities,
  parseFtsRegconfig,
  parseRagProfile,
} from '@/constants/rag-profile';
import type { EmbeddingSelection } from '@/components/workspace/embedding-model-selector';
import type { LLMSelection } from '@/components/workspace/llm-model-selector';
import {
  embeddingSelectionFromPick,
  llmSelectionFromPick,
} from '@/lib/onboarding/wizard-picks';
import {
  EMPTY_WIZARD_DRAFT,
  type WizardDraft,
} from '@/lib/onboarding/wizard-state';
import type { WorkspaceConfigSnapshot } from '@/lib/onboarding/workspace-config-diff';
import {
  getWorkspaceEmbeddingSelection,
  getWorkspaceLlmSelection,
  getWorkspacePdfParserBackend,
  getWorkspaceVisionSelection,
} from '@/lib/workspace/drafts';
import type { Workspace } from '@/types';

export interface ReconfigurePrefill {
  draft: WizardDraft;
  llm: LLMSelection | undefined;
  embedding: EmbeddingSelection | undefined;
  vision: LLMSelection | undefined;
  /** Open Advanced when workspace already has model overrides. */
  advancedOpen: boolean;
  snapshot: WorkspaceConfigSnapshot;
}

/**
 * Resolve relation allow-list + typed edges for the reconfigure draft.
 *
 * - Persisted relations/edges win.
 * - When relations empty but `kg_schema_preset` names a known domain,
 *   seed RELATION_PRESETS + EDGE_PRESETS (snapshot matches draft).
 * - Otherwise keep empty (free-form / unconstrained endpoints).
 */
function resolveSchemaForPrefill(
  workspace: Workspace,
  entityTypes: string[],
): {
  relationTypes: string[];
  relationEdges: RelationEdge[];
  kgSchemaPreset: PresetKey | string;
} {
  const persistedEdges: RelationEdge[] = (workspace.relation_edges ?? []).map(
    (e) => ({
      source: e.source,
      relation: e.relation,
      target: e.target,
    }),
  );

  if (workspace.relation_types?.length) {
    const preset =
      workspace.kg_schema_preset ??
      detectKgSchemaPreset(
        entityTypes,
        workspace.relation_types,
        persistedEdges,
      );
    return {
      relationTypes: [...workspace.relation_types],
      relationEdges: persistedEdges,
      kgSchemaPreset: preset,
    };
  }

  const named = workspace.kg_schema_preset?.trim();
  if (named && named !== 'custom' && named in RELATION_PRESETS) {
    const key = named as Exclude<PresetKey, 'custom'>;
    // Blank: keep empty vocabulary (do not seed General defaults).
    if (key === 'blank') {
      return {
        relationTypes: [],
        relationEdges: persistedEdges,
        kgSchemaPreset: 'blank',
      };
    }
    return {
      relationTypes: [...RELATION_PRESETS[key]],
      relationEdges: persistedEdges.length
        ? persistedEdges
        : EDGE_PRESETS[key].map((e) => ({ ...e })),
      kgSchemaPreset: key,
    };
  }

  return {
    relationTypes: [],
    relationEdges: persistedEdges,
    kgSchemaPreset:
      workspace.kg_schema_preset ??
      detectKgSchemaPreset(entityTypes, [], persistedEdges),
  };
}

export function prefillReconfigureFromWorkspace(workspace: Workspace): ReconfigurePrefill {
  const llm = getWorkspaceLlmSelection(workspace, { overridesOnly: true });
  const embedding = getWorkspaceEmbeddingSelection(workspace, {
    overridesOnly: true,
  });
  const vision = getWorkspaceVisionSelection(workspace, { overridesOnly: true });
  const pdfParserBackend = getWorkspacePdfParserBackend(workspace);
  const hasOverrides = Boolean(llm || embedding || vision);

  const entityTypes = workspace.entity_types?.length
    ? [...workspace.entity_types]
    : [...ENTITY_PRESETS.general.types];
  const { relationTypes, relationEdges, kgSchemaPreset } = resolveSchemaForPrefill(
    workspace,
    entityTypes,
  );

  const draft: WizardDraft = {
    ...EMPTY_WIZARD_DRAFT,
    ragProfile: parseRagProfile(workspace.rag_profile),
    embeddingModalities: parseEmbeddingModalities(workspace.embedding_modalities),
    ftsRegconfig: parseFtsRegconfig(workspace.fts_regconfig),
    workspaceName: workspace.name,
    workspaceSlug: workspace.slug ?? '',
    workspaceDescription: workspace.description ?? '',
    useServerDefaults: !hasOverrides,
    extractionLanguage: workspace.extraction_language ?? null,
    chunkingMode: workspace.chunking_mode
      ? parseChunkingMode(workspace.chunking_mode)
      : null,
    chunkTokenSize: workspace.chunk_token_size ?? ACC_FAIR_CHUNK_TOKEN_SIZE,
    chunkOverlapTokenSize:
      workspace.chunk_overlap_token_size ?? ACC_FAIR_CHUNK_OVERLAP,
    extractBudgetMode:
      typeof workspace.extract_max_entities === 'number' ? 'custom' : null,
    extractMaxEntities: workspace.extract_max_entities ?? 40,
    extractMaxRecords: workspace.extract_max_records ?? 100,
    entityTypes,
    entityTypesStrict: workspace.entity_types_strict ?? true,
    entityTypeColors: { ...(workspace.entity_type_colors ?? {}) },
    relationTypes,
    relationTypesStrict: workspace.relation_types_strict ?? true,
    kgSchemaPreset,
    relationEdges,
    pdfParserBackend,
    visionExtractImages: workspace.vision_extract_images ?? true,
    visionExtractCharts: workspace.vision_extract_charts ?? true,
    visionExtractFigures: workspace.vision_extract_figures ?? true,
    visionPageSystemPrompt: workspace.vision_page_system_prompt ?? '',
    visionImageSystemPrompt: workspace.vision_image_system_prompt ?? '',
    visionChartSystemPrompt: workspace.vision_chart_system_prompt ?? '',
    visionFigureSystemPrompt: workspace.vision_figure_system_prompt ?? '',
    reasoningEffort: workspace.default_reasoning_effort ?? undefined,
    decisionEnabled: workspace.decision_enabled !== false,
    decisionModel: workspace.decision_model?.trim() || 'tev1:0.8b',
    decisionPreset:
      workspace.decision_gate_preset === 'strict' ||
      workspace.decision_gate_preset === 'balanced' ||
      workspace.decision_gate_preset === 'recall'
        ? workspace.decision_gate_preset
        : 'inherit',
    decisionPackSize:
      typeof workspace.decision_pack_size === 'number' && workspace.decision_pack_size > 0
        ? String(workspace.decision_pack_size)
        : '',
    extractionMode:
      workspace.extraction_mode === 'llm' || workspace.extraction_mode === 'decision'
        ? workspace.extraction_mode
        : 'inherit',
    llmPick: llm
      ? { provider: llm.provider, model: llm.model, fullId: llm.fullId }
      : undefined,
    embeddingPick: embedding
      ? {
          provider: embedding.provider,
          model: embedding.model,
          dimension: embedding.dimension,
        }
      : undefined,
    visionPick: vision
      ? { provider: vision.provider, model: vision.model, fullId: vision.fullId }
      : undefined,
    advancedOpen: hasOverrides,
  };

  const snapshot = snapshotFromWizardState({ draft, llm, embedding, vision });

  return {
    draft,
    llm,
    embedding,
    vision,
    advancedOpen: hasOverrides,
    snapshot,
  };
}

export function snapshotFromWizardState(args: {
  draft: WizardDraft;
  llm?: LLMSelection;
  embedding?: EmbeddingSelection;
  vision?: LLMSelection;
}): WorkspaceConfigSnapshot {
  const llm = args.llm ?? llmSelectionFromPick(args.draft.llmPick);
  const embedding =
    args.embedding ?? embeddingSelectionFromPick(args.draft.embeddingPick);
  const vision = args.vision ?? llmSelectionFromPick(args.draft.visionPick);
  return {
    useServerDefaults: args.draft.useServerDefaults,
    llm,
    embedding,
    vision,
    pdfParserBackend: args.draft.pdfParserBackend,
    extractionLanguage: args.draft.extractionLanguage,
    chunkingMode: args.draft.chunkingMode,
    chunkTokenSize: args.draft.chunkTokenSize,
    chunkOverlapTokenSize: args.draft.chunkOverlapTokenSize,
    extractBudgetMode: args.draft.extractBudgetMode,
    extractMaxEntities: args.draft.extractMaxEntities,
    extractMaxRecords: args.draft.extractMaxRecords,
    entityTypes: [...args.draft.entityTypes],
    entityTypesStrict: args.draft.entityTypesStrict,
    entityTypeColors: { ...(args.draft.entityTypeColors ?? {}) },
    relationTypes: [...(args.draft.relationTypes ?? [])],
    relationTypesStrict: args.draft.relationTypesStrict ?? true,
    kgSchemaPreset: args.draft.kgSchemaPreset,
    relationEdges: (args.draft.relationEdges ?? []).map((e) => ({ ...e })),
    visionExtractImages: args.draft.visionExtractImages,
    visionExtractCharts: args.draft.visionExtractCharts,
    visionExtractFigures: args.draft.visionExtractFigures,
    visionPageSystemPrompt: args.draft.visionPageSystemPrompt,
    visionImageSystemPrompt: args.draft.visionImageSystemPrompt,
    visionChartSystemPrompt: args.draft.visionChartSystemPrompt,
    visionFigureSystemPrompt: args.draft.visionFigureSystemPrompt,
    reasoningEffort: args.draft.reasoningEffort,
    decisionEnabled: args.draft.decisionEnabled,
    decisionModel: args.draft.decisionModel,
    decisionPreset: args.draft.decisionPreset,
    decisionPackSize: args.draft.decisionPackSize,
    extractionMode: args.draft.extractionMode,
  };
}

/**
 * When restoring a session draft, never keep useServerDefaults=true while Advanced
 * is open or the workspace already has concrete picks — Apply would clear overrides.
 */
export function resolveHydratedUseServerDefaults(args: {
  prefillAdvancedOpen: boolean;
  prefillUseServerDefaults: boolean;
  hasPrefillPicks: boolean;
  storedUseServerDefaults: boolean;
}): boolean {
  const advanced =
    args.prefillAdvancedOpen ||
    !args.prefillUseServerDefaults ||
    args.hasPrefillPicks;
  return advanced ? false : args.storedUseServerDefaults;
}
