import type { WizardKind, WizardStepId } from '@/lib/onboarding/wizard-state';

export const STEP_META: Record<
  WizardStepId,
  { titleKey: string; title: string; descriptionKey: string; description: string }
> = {
  admin: {
    titleKey: 'onboarding.adminTitle',
    title: 'Create admin account',
    descriptionKey: 'onboarding.adminSubtitle',
    description: 'This password secures your EdgeQuake instance.',
  },
  'tenant-basics': {
    titleKey: 'onboarding.tenantTitle',
    title: 'Name your organization',
    descriptionKey: 'onboarding.tenantSubtitle',
    description: 'Organizations isolate workspaces and data.',
  },
  models: {
    titleKey: 'onboarding.modelsTitle',
    title: 'Confirm AI models',
    descriptionKey: 'onboarding.modelsSubtitle',
    description: 'Inherited defaults apply unless you override.',
  },
  'workspace-basics': {
    titleKey: 'onboarding.workspaceTitle',
    title: 'Name your workspace',
    descriptionKey: 'onboarding.workspaceSubtitle',
    description: 'Documents and extracted knowledge live here.',
  },
  retrieval: {
    titleKey: 'ragProfile.stepTitle',
    title: 'Retrieval type',
    descriptionKey: 'ragProfile.stepDescription',
    description: 'How this workspace finds answers. This choice is permanent.',
  },
  'document-parsing': {
    titleKey: 'onboarding.documentParsingTitle',
    title: 'Document parsing',
    descriptionKey: 'onboarding.documentParsingSubtitle',
    description: 'Choose how PDFs are converted to text.',
  },
  chunking: {
    titleKey: 'onboarding.chunkingTitle',
    title: 'Chunking',
    descriptionKey: 'onboarding.chunkingSubtitle',
    description: 'How documents are split before entity extraction.',
  },
  'extract-budget': {
    titleKey: 'onboarding.extractBudgetTitle',
    title: 'Extract budget',
    descriptionKey: 'onboarding.extractBudgetSubtitle',
    description: 'Per-response entity and record caps for LLM extraction.',
  },
  extraction: {
    titleKey: 'onboarding.extractionTitle',
    title: 'Extraction preferences',
    descriptionKey: 'onboarding.extractionSubtitle',
    description: 'Language and knowledge-graph schema for extraction.',
  },
  review: {
    titleKey: 'onboarding.reviewTitle',
    title: 'Review and create',
    descriptionKey: 'onboarding.reviewSubtitle',
    description: 'Nothing is saved until you confirm.',
  },
};

/** Reconfigure uses Apply-oriented review copy (LAW-101-12). */
export const RECONFIGURE_REVIEW_META = {
  titleKey: 'onboarding.reconfigureReviewTitle',
  title: 'Review and apply',
  descriptionKey: 'onboarding.reconfigureReviewSubtitle',
  description: 'Confirm changes. Rebuild may be required for existing docs.',
} as const;

/** Short stepper label per step (the long title stays in the step header). */
export const STEP_LABEL: Record<WizardStepId, { key: string; label: string }> = {
  admin: { key: 'onboarding.stepAdmin', label: 'Admin' },
  'tenant-basics': { key: 'onboarding.stepTenant', label: 'Organization' },
  models: { key: 'onboarding.stepModels', label: 'Models' },
  'workspace-basics': { key: 'onboarding.stepWorkspace', label: 'Name' },
  retrieval: { key: 'ragProfile.stepLabel', label: 'Retrieval' },
  'document-parsing': { key: 'onboarding.stepParsing', label: 'Parsing' },
  chunking: { key: 'onboarding.stepChunking', label: 'Chunking' },
  'extract-budget': { key: 'onboarding.stepBudget', label: 'Budget' },
  extraction: { key: 'onboarding.stepExtraction', label: 'Extraction' },
  review: { key: 'onboarding.stepReview', label: 'Review' },
};

type StepMeta = (typeof STEP_META)[WizardStepId];

/** Title + description for a step, honouring Apply-style review copy on reconfigure. */
export function resolveStepMeta(
  stepId: WizardStepId,
  kind: WizardKind,
): StepMeta | typeof RECONFIGURE_REVIEW_META {
  return stepId === 'review' && kind === 'reconfigure-workspace'
    ? RECONFIGURE_REVIEW_META
    : STEP_META[stepId];
}

export function resolveStepLabels(
  steps: readonly WizardStepId[],
  t: (key: string, fallback: string) => string,
): string[] {
  return steps.map((id) => t(STEP_LABEL[id].key, STEP_LABEL[id].label));
}
