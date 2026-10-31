/**
 * SPEC-101 — Pure wizard step navigation + validation (unit-tested).
 */

import type { RelationEdge } from '@/constants/kg-schema-presets';
import { isChunkingDraftValid } from '@/lib/onboarding/draft-chunking';
import { isExtractBudgetDraftValid } from '@/lib/onboarding/draft-extract-budget';
import {
  DEFAULT_EMBEDDING_MODALITIES,
  DEFAULT_FTS_REGCONFIG,
  DEFAULT_RAG_PROFILE,
  type EmbeddingModalities,
  type FtsRegconfig,
  type RagProfile,
} from '@/constants/rag-profile';

export type WizardKind =
  | 'first-run'
  | 'create-tenant'
  | 'create-workspace'
  | 'reconfigure-workspace';

export type WizardStepId =
  | 'admin'
  | 'tenant-basics'
  | 'models'
  | 'workspace-basics'
  | 'retrieval'
  | 'document-parsing'
  | 'chunking'
  | 'extract-budget'
  | 'extraction'
  | 'review';

export type PdfParserBackendDraft =
  | 'none'
  | 'vision'
  | 'edgeparse'
  | 'edgeparse-ocr'
  | 'auto';

export interface WizardDraft {
  /** SPEC-180 — retrieval type. Fixed once the workspace is created. */
  ragProfile: RagProfile;
  /** SPEC-180 — what the embedding model reads (`text,image` needs `normal`). */
  embeddingModalities: EmbeddingModalities;
  /** SPEC-180 — keyword-index language. */
  ftsRegconfig: FtsRegconfig;
  adminUsername: string;
  adminEmail: string;
  adminPassword: string;
  adminPasswordConfirm: string;
  tenantName: string;
  tenantDescription: string;
  workspaceName: string;
  workspaceSlug: string;
  workspaceDescription: string;
  useServerDefaults: boolean;
  extractionLanguage: string | null;
  /** SPEC-116 — inherit | adaptive | fixed (null ≡ inherit). */
  chunkingMode: 'inherit' | 'adaptive' | 'fixed' | null;
  chunkTokenSize: number;
  chunkOverlapTokenSize: number;
  /** SPEC-117 — inherit | custom (null ≡ inherit). */
  extractBudgetMode: 'inherit' | 'custom' | null;
  extractMaxEntities: number;
  extractMaxRecords: number;
  entityTypes: string[];
  /** SPEC-101 Wave 8 — PDF parser (reconfigure). */
  pdfParserBackend: PdfParserBackendDraft;
  /** SPEC-015V — Vision extract policy (document parsing). */
  visionExtractImages: boolean;
  visionExtractCharts: boolean;
  visionExtractFigures: boolean;
  visionPageSystemPrompt: string;
  visionImageSystemPrompt: string;
  visionChartSystemPrompt: string;
  visionFigureSystemPrompt: string;
  /** SPEC-101 Wave 8 — strict entity types (reconfigure). */
  entityTypesStrict: boolean;
  /** SPEC-102 — entity type → hex color overrides. */
  entityTypeColors: Record<string, string>;
  /** SPEC-114 — relation type allow-list (empty = free-form). */
  relationTypes: string[];
  /** SPEC-114 — strict relation types. */
  relationTypesStrict: boolean;
  /** SPEC-114 — optional domain preset id. */
  kgSchemaPreset?: string;
  /** SPEC-114b — typed edges (source — relation → target). */
  relationEdges: RelationEdge[];
  /** SPEC-109 — seed default reasoning effort. */
  reasoningEffort?: string;
  /** SPEC-160 — workspace Decision engine (reconfigure + create). */
  decisionEnabled: boolean;
  decisionModel: string;
  decisionPreset: 'inherit' | 'strict' | 'balanced' | 'recall';
  decisionPackSize: string;
  /** SPEC-160 — default extraction mode (`inherit` leaves the server default). */
  extractionMode: 'inherit' | 'llm' | 'decision';
  /** Model overrides (persisted with the draft — LAW-101-9). */
  llmPick?: WizardModelPick;
  embeddingPick?: WizardModelPick;
  visionPick?: WizardModelPick;
  advancedOpen: boolean;
}

/** Slash-style model pick stored on the wizard draft. */
export interface WizardModelPick {
  provider: string;
  model: string;
  fullId?: string;
  dimension?: number;
  /** SPEC-180 — the embedding model accepts images (from the catalog, never guessed). */
  embedsImages?: boolean;
}

export const EMPTY_WIZARD_DRAFT: WizardDraft = {
  ragProfile: DEFAULT_RAG_PROFILE,
  embeddingModalities: DEFAULT_EMBEDDING_MODALITIES,
  ftsRegconfig: DEFAULT_FTS_REGCONFIG,
  adminUsername: 'admin',
  adminEmail: '',
  adminPassword: '',
  adminPasswordConfirm: '',
  tenantName: '',
  tenantDescription: '',
  workspaceName: '',
  workspaceSlug: '',
  workspaceDescription: '',
  useServerDefaults: true,
  extractionLanguage: null,
  chunkingMode: null,
  chunkTokenSize: 1200,
  chunkOverlapTokenSize: 100,
  extractBudgetMode: null,
  extractMaxEntities: 40,
  extractMaxRecords: 100,
  entityTypes: [],
  pdfParserBackend: 'none',
  visionExtractImages: true,
  visionExtractCharts: true,
  visionExtractFigures: true,
  visionPageSystemPrompt: '',
  visionImageSystemPrompt: '',
  visionChartSystemPrompt: '',
  visionFigureSystemPrompt: '',
  entityTypesStrict: true,
  entityTypeColors: {},
  relationTypes: [],
  relationTypesStrict: true,
  kgSchemaPreset: undefined,
  relationEdges: [],
  reasoningEffort: undefined,
  decisionEnabled: true,
  decisionModel: 'tev1:0.8b',
  decisionPreset: 'inherit',
  decisionPackSize: '',
  extractionMode: 'inherit',
  llmPick: undefined,
  embeddingPick: undefined,
  visionPick: undefined,
  advancedOpen: false,
};

/**
 * Insert ingest-tuning steps before extraction:
 * chunking → extract-budget → extraction (language / KG schema).
 */
function withIngestTuningBeforeExtraction(
  steps: WizardStepId[],
  includeExtraction: boolean,
): WizardStepId[] {
  if (!includeExtraction) return steps;
  const out: WizardStepId[] = [];
  for (const step of steps) {
    if (step === 'extraction') {
      out.push('chunking', 'extract-budget');
    }
    out.push(step);
  }
  return out;
}

/** Document parsing sits immediately after models on every wizard kind. */
function withDocumentParsingAfterModels(steps: WizardStepId[]): WizardStepId[] {
  if (steps.includes('document-parsing')) return steps;
  const out: WizardStepId[] = [];
  for (const step of steps) {
    out.push(step);
    if (step === 'models') out.push('document-parsing');
  }
  return out;
}

export interface StepsOptions {
  includeAdmin: boolean;
  includeExtraction: boolean;
  /** SPEC-180 — a normal draft never enters extract-budget or extraction. */
  ragProfile?: RagProfile;
}

/** Create-workspace steps for a normal draft: the graph-only steps are not offered. */
function normalCreateSteps(): WizardStepId[] {
  return ['workspace-basics', 'retrieval', 'models', 'chunking', 'review'];
}

export function stepsForWizard(
  kind: WizardKind,
  opts: StepsOptions = {
    includeAdmin: false,
    includeExtraction: true,
  },
): WizardStepId[] {
  let steps: WizardStepId[];
  if (kind === 'reconfigure-workspace' && opts.ragProfile === 'normal') {
    steps = ['models', 'chunking', 'review'];
  } else if (kind === 'reconfigure-workspace') {
    steps = withIngestTuningBeforeExtraction(
      ['models', 'document-parsing', 'extraction', 'review'],
      true,
    );
  } else if (kind === 'create-tenant') {
    steps = withIngestTuningBeforeExtraction(
      ['tenant-basics', 'models', 'workspace-basics', 'extraction', 'review'],
      true,
    );
  } else if (kind === 'create-workspace' && opts.ragProfile === 'normal') {
    steps = normalCreateSteps();
  } else if (kind === 'create-workspace') {
    const create: WizardStepId[] = ['workspace-basics', 'retrieval', 'models'];
    if (opts.includeExtraction) create.push('extraction');
    create.push('review');
    steps = withIngestTuningBeforeExtraction(create, opts.includeExtraction);
  } else {
    // first-run
    const first: WizardStepId[] = [];
    if (opts.includeAdmin) first.push('admin');
    first.push('tenant-basics', 'models', 'workspace-basics');
    if (opts.includeExtraction) first.push('extraction');
    first.push('review');
    steps = withIngestTuningBeforeExtraction(first, opts.includeExtraction);
  }
  return withDocumentParsingAfterModels(steps);
}

/** Why “Next” is disabled — SSOT for validation and for the footer hint. */
export type WizardBlockerCode =
  | 'admin_invalid'
  | 'tenant_name_required'
  | 'workspace_name_required'
  | 'models_not_eligible'
  | 'embedding_not_multimodal'
  | 'embedding_probe_failed'
  | 'embedding_probe_checking'
  | 'defaults_incomplete'
  | 'models_incomplete'
  | 'chunking_invalid'
  | 'extract_budget_invalid'
  | 'no_changes';

export interface WizardGateOptions {
  hasConfiguredDefaults: boolean;
  advancedModelsValid: boolean;
  /** When false (reconfigure), Apply requires at least one config diff. */
  hasConfigChanges?: boolean;
  /** SPEC-179: false when the pair to save is not offered or has no vector size. */
  eligibilityOk?: boolean;
  /** SPEC-180: false when `text,image` is chosen and the embedding pick cannot read images. */
  imageEmbeddingOk?: boolean;
  /** SPEC-180 08: the deployment refused the image embedding probe for the chosen pair. */
  imageProbeFailed?: boolean;
  /** SPEC-180 08: the probe is in flight; Next waits rather than racing it. */
  imageProbeChecking?: boolean;
}

const OPEN_GATE: WizardGateOptions = {
  hasConfiguredDefaults: true,
  advancedModelsValid: true,
};

function modelsBlocker(
  draft: WizardDraft,
  opts: WizardGateOptions,
): WizardBlockerCode | null {
  if (opts.eligibilityOk === false) return 'models_not_eligible';
  if (opts.imageEmbeddingOk === false) return 'embedding_not_multimodal';
  if (opts.imageProbeFailed === true) return 'embedding_probe_failed';
  // Waiting beats racing: advancing while the probe is in flight would reach Review with an
  // answer that had not arrived yet.
  if (opts.imageProbeChecking === true) return 'embedding_probe_checking';
  if (draft.useServerDefaults) {
    return opts.hasConfiguredDefaults ? null : 'defaults_incomplete';
  }
  return opts.advancedModelsValid ? null : 'models_incomplete';
}

function adminBlocker(draft: WizardDraft): WizardBlockerCode | null {
  const userOk = draft.adminUsername.trim().length >= 3;
  const passOk = draft.adminPassword.length >= 8;
  const match = draft.adminPassword === draft.adminPasswordConfirm;
  return userOk && passOk && match ? null : 'admin_invalid';
}

function reviewBlocker(opts: WizardGateOptions): WizardBlockerCode | null {
  if (opts.eligibilityOk === false) return 'models_not_eligible';
  return opts.hasConfigChanges === false ? 'no_changes' : null;
}

/** The first reason `step` cannot advance, or `null` when it can. */
export function wizardBlocker(
  step: WizardStepId,
  draft: WizardDraft,
  opts: WizardGateOptions = OPEN_GATE,
): WizardBlockerCode | null {
  switch (step) {
    case 'admin':
      return adminBlocker(draft);
    case 'tenant-basics':
      return draft.tenantName.trim() ? null : 'tenant_name_required';
    case 'workspace-basics':
      return draft.workspaceName.trim() ? null : 'workspace_name_required';
    case 'models':
      return modelsBlocker(draft, opts);
    case 'chunking':
      return isChunkingDraftValid(draft) ? null : 'chunking_invalid';
    case 'extract-budget':
      return isExtractBudgetDraftValid(draft) ? null : 'extract_budget_invalid';
    case 'review':
      return reviewBlocker(opts);
    case 'retrieval':
    case 'document-parsing':
    case 'extraction':
      return null;
    default:
      return 'no_changes';
  }
}

export function canProceed(
  step: WizardStepId,
  draft: WizardDraft,
  opts: WizardGateOptions = OPEN_GATE,
): boolean {
  return wizardBlocker(step, draft, opts) === null;
}

export function clampStepIndex(index: number, stepCount: number): number {
  if (stepCount <= 0) return 0;
  return Math.max(0, Math.min(index, stepCount - 1));
}

export function progressPercent(index: number, stepCount: number): number {
  if (stepCount <= 0) return 0;
  return Math.round(((index + 1) / stepCount) * 100);
}

/** Persist non-secret draft fields (never password). */
export function draftForStorage(draft: WizardDraft): Omit<
  WizardDraft,
  'adminPassword' | 'adminPasswordConfirm'
> {
  const {
    adminPassword: _p,
    adminPasswordConfirm: _c,
    ...safe
  } = draft;
  return safe;
}
