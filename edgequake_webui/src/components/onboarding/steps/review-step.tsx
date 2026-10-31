'use client';

import { ReviewRetrievalGroup } from '@/components/onboarding/steps/review-retrieval-group';
import { Group, Row } from '@/components/onboarding/steps/review-parts';
import type { EmbeddingSelection } from '@/components/workspace/embedding-model-selector';
import type { LLMSelection } from '@/components/workspace/llm-model-selector';
import { useInheritedModelDefaults } from '@/hooks/use-inherited-model-defaults';
import type { WorkspaceConfigChangedKey, WorkspaceConfigDiff } from '@/lib/onboarding/workspace-config-diff';
import type { WizardDraft, WizardStepId } from '@/lib/onboarding/wizard-state';
import { formatServerDefaultExtractionLanguageLabel } from '@/constants/extraction-languages';
import { formatServerDefaultPdfParserLabel } from '@/lib/pdf/resolve-pdf-parser-backend';
import { AlertTriangle } from 'lucide-react';
import { useTranslation } from 'react-i18next';

export interface ReviewStepProps {
  draft: WizardDraft;
  includeAdmin?: boolean;
  includeTenant?: boolean;
  /** When set (Create Workspace), show tenant → server inherited IDs. */
  tenantId?: string | null;
  llm?: LLMSelection;
  embedding?: EmbeddingSelection;
  vision?: LLMSelection;
  /** Jump to a prior step from review (LAW Wave 6). */
  onEditStep?: (step: WizardStepId) => void;
  /** When false, omit the duplicate review hint (shell step description already shows it). */
  showHint?: boolean;
  /** Reconfigure: hide identity group (name/slug not editable). */
  hideWorkspaceIdentity?: boolean;
  /** Reconfigure: show PDF parser row. */
  showDocumentParsing?: boolean;
  /** Reconfigure: Impact block (LAW-101-12). */
  impact?: WorkspaceConfigDiff | null;
  /** Document count for soft messaging when zero. */
  documentCount?: number;
  /** SPEC-180: Create Workspace states the retrieval choice and its permanence. */
  showRetrieval?: boolean;
}

function pdfLabel(
  value: WizardDraft['pdfParserBackend'],
  t: (key: string, fallback: string, options?: { value: string }) => string,
): string {
  switch (value) {
    case 'edgeparse':
      return t('settings.pdfParser.edgeparse', 'EdgeParse');
    case 'vision':
      return t('settings.pdfParser.vision', 'Vision');
    default:
      return formatServerDefaultPdfParserLabel(t);
  }
}

const CHANGE_LABEL_KEYS: Record<
  WorkspaceConfigChangedKey,
  { key: string; fallback: string }
> = {
  llm: { key: 'onboarding.impactLlm', fallback: 'LLM model' },
  embedding: { key: 'onboarding.impactEmbedding', fallback: 'Embedding model' },
  vision: { key: 'onboarding.impactVision', fallback: 'Vision LLM' },
  pdfParser: { key: 'onboarding.impactPdfParser', fallback: 'PDF parser' },
  extractionLanguage: {
    key: 'onboarding.impactLanguage',
    fallback: 'Extraction language',
  },
  chunking: { key: 'onboarding.impactChunking', fallback: 'Chunking' },
  extractBudget: {
    key: 'onboarding.impactExtractBudget',
    fallback: 'Extract budget',
  },
  entityTypes: { key: 'onboarding.impactEntityTypes', fallback: 'Entity types' },
  entityTypesStrict: {
    key: 'onboarding.impactStrict',
    fallback: 'Strict entity types',
  },
  entityTypeColors: {
    key: 'onboarding.impactEntityTypeColors',
    fallback: 'Entity type colors',
  },
  relationTypes: {
    key: 'onboarding.impactRelationTypes',
    fallback: 'Relation types',
  },
  relationTypesStrict: {
    key: 'onboarding.impactRelationStrict',
    fallback: 'Strict relation types',
  },
  kgSchemaPreset: {
    key: 'onboarding.impactKgSchemaPreset',
    fallback: 'KG schema domain',
  },
  relationEdges: {
    key: 'onboarding.impactRelationEdges',
    fallback: 'Typed edges',
  },
  visionExtract: {
    key: 'onboarding.impactVisionExtract',
    fallback: 'Vision extraction',
  },
  reasoningEffort: {
    key: 'onboarding.impactReasoningEffort',
    fallback: 'Reasoning effort',
  },
  decisionEngine: {
    key: 'onboarding.impactDecisionEngine',
    fallback: 'Decision engine',
  },
  extractionMode: {
    key: 'onboarding.impactExtractionMode',
    fallback: 'Extraction mode',
  },
};

export function ReviewStep({
  draft,
  includeAdmin = false,
  includeTenant = true,
  tenantId,
  llm,
  embedding,
  vision,
  onEditStep,
  showHint = false,
  hideWorkspaceIdentity = false,
  showDocumentParsing = false,
  impact = null,
  documentCount = 0,
  showRetrieval = false,
}: ReviewStepProps) {
  const { t } = useTranslation();
  const defaults = useInheritedModelDefaults(tenantId ?? null);
  const editLabel = t('onboarding.editStep', 'Edit');

  const inheritLabel =
    defaults.source === 'tenant'
      ? t('onboarding.usingTenantDefaults', 'Using organization defaults')
      : t('onboarding.usingServerDefaults', 'Using server defaults');

  const llmId = draft.useServerDefaults
    ? `${defaults.defaultLlmProvider}/${defaults.defaultLlmModel}`
    : llm?.fullId ?? '—';
  const embId = draft.useServerDefaults
    ? `${defaults.defaultEmbeddingProvider}/${defaults.defaultEmbeddingModel}`
    : embedding
      ? `${embedding.provider}/${embedding.model}`
      : '—';
  const visionId = draft.useServerDefaults
    ? `${defaults.defaultVisionProvider}/${defaults.defaultVisionModel}`
    : vision?.fullId ?? '—';

  const entitySample =
    draft.entityTypes.length === 0
      ? '—'
      : draft.entityTypes.length <= 3
        ? draft.entityTypes.join(', ')
        : `${draft.entityTypes.slice(0, 3).join(', ')} +${draft.entityTypes.length - 3}`;

  const showImpact = impact != null && impact.hasChanges;
  // SPEC-180: a normal workspace has no extraction engine, budget or schema to review.
  const hasGraph = draft.ragProfile !== 'normal';

  return (
    <div className="space-y-3" data-testid="wizard-step-review">
      {showHint ? (
        <p className="text-sm text-muted-foreground">
          {t('onboarding.reviewHint', 'Nothing is saved until you confirm.')}
        </p>
      ) : null}

      {showImpact ? (
        <div
          className="rounded-md border border-amber-200/80 bg-amber-50/60 dark:bg-amber-950/20 dark:border-amber-900 p-2.5 space-y-2"
          data-testid="wizard-reconfigure-impact"
        >
          <div className="flex items-center gap-2 text-sm font-medium">
            <AlertTriangle className="h-4 w-4 text-amber-600 shrink-0" />
            {t('onboarding.impactTitle', 'Impact of these changes')}
          </div>
          <ul className="text-xs text-muted-foreground space-y-1 list-disc pl-5">
            {impact.changedKeys.map((key) => {
              const meta = CHANGE_LABEL_KEYS[key] ?? {
                key: key,
                fallback: key,
              };
              return <li key={key}>{t(meta.key, meta.fallback)}</li>;
            })}
          </ul>
          {impact.rebuildHints.embeddings ||
          impact.rebuildHints.extraction ||
          impact.rebuildHints.vision ? (
            <p className="text-xs text-foreground/90" data-testid="wizard-reconfigure-rebuild-hint">
              {t(
                'onboarding.impactRebuildRequired',
                'Existing documents may need a rebuild after you apply.',
              )}
              {impact.rebuildHints.embeddings
                ? ` ${t('onboarding.impactRebuildEmbeddings', 'Rebuild Embeddings for the new embedding model.')}`
                : ''}
              {impact.rebuildHints.extraction && impact.changedKeys.includes('llm')
                ? ` ${t('onboarding.impactRebuildKg', 'Rebuild Knowledge Graph for the new LLM.')}`
                : ''}
              {impact.rebuildHints.extraction && !impact.changedKeys.includes('llm')
                ? ` ${t(
                    'onboarding.impactRebuildKgSchema',
                    'Rebuild Knowledge Graph so existing documents pick up chunking, budget, and schema changes.',
                  )}`
                : ''}
              {impact.rebuildHints.vision
                ? ` ${t('onboarding.impactRebuildVision', 'Rebuild Knowledge Graph for vision/PDF changes.')}`
                : ''}
            </p>
          ) : documentCount === 0 ? (
            <p className="text-xs text-muted-foreground" data-testid="wizard-reconfigure-zero-docs">
              {t(
                'onboarding.impactZeroDocs',
                'No documents yet — changes apply to future ingestions.',
              )}
            </p>
          ) : (
            <p className="text-xs text-muted-foreground">
              {t(
                'onboarding.impactFutureOnly',
                'Changes apply to future ingestions. Rebuild Knowledge Graph to reprocess existing documents.',
              )}
            </p>
          )}
        </div>
      ) : null}

      {includeAdmin ? (
        <Group
          title={t('onboarding.adminSection', 'Admin')}
          onEdit={onEditStep ? () => onEditStep('admin') : undefined}
          editLabel={editLabel}
          testId="wizard-review-admin"
        >
          <Row label={t('onboarding.adminUsername', 'Username')} value={draft.adminUsername} />
          <Row
            label={t('onboarding.adminEmail', 'Email')}
            value={draft.adminEmail || 'admin@localhost'}
          />
        </Group>
      ) : null}

      {includeTenant ? (
        <Group
          title={t('tenant.tenant', 'Organization')}
          onEdit={onEditStep ? () => onEditStep('tenant-basics') : undefined}
          editLabel={editLabel}
          testId="wizard-review-tenant"
        >
          <Row label={t('tenant.name', 'Name')} value={draft.tenantName} />
          {draft.tenantDescription ? (
            <Row label={t('tenant.description', 'Description')} value={draft.tenantDescription} />
          ) : null}
        </Group>
      ) : null}

      {!hideWorkspaceIdentity ? (
        <Group
          title={t('workspace.workspace', 'Workspace')}
          onEdit={onEditStep ? () => onEditStep('workspace-basics') : undefined}
          editLabel={editLabel}
          testId="wizard-review-workspace"
        >
          <Row label={t('workspace.name', 'Name')} value={draft.workspaceName} />
          <Row label={t('workspace.slug', 'Slug')} value={draft.workspaceSlug || '(auto)'} />
        </Group>
      ) : draft.workspaceName ? (
        <Group
          title={t('workspace.workspace', 'Workspace')}
          editLabel={editLabel}
          testId="wizard-review-workspace"
        >
          <Row label={t('workspace.name', 'Name')} value={draft.workspaceName} />
        </Group>
      ) : null}

      {showRetrieval ? (
        <ReviewRetrievalGroup draft={draft} onEditStep={onEditStep} editLabel={editLabel} />
      ) : null}

      <Group
        title={t('onboarding.modelsSection', 'Models')}
        onEdit={onEditStep ? () => onEditStep('models') : undefined}
        editLabel={editLabel}
        testId="wizard-review-models"
      >
        <Row
          label={t('onboarding.modelsMode', 'Mode')}
          value={
            draft.useServerDefaults
              ? inheritLabel
              : t('onboarding.customModels', 'Custom model selection')
          }
        />
        <Row label="LLM" value={llmId} mono />
        <Row label="Embedding" value={embId} mono />
        <Row label="Vision" value={visionId} mono />
        {hasGraph ? (
          <Row
            label={t('extractionMode.workspace.model', 'Decision model')}
            value={
              draft.decisionEnabled
                ? draft.decisionModel || 'tev1:0.8b'
                : t('extractionMode.engine.badgeInactive', 'Not activated')
            }
            mono={draft.decisionEnabled}
          />
        ) : null}
      </Group>

      {showDocumentParsing ? (
        <Group
          title={t('onboarding.documentParsingSection', 'Document parsing')}
          onEdit={onEditStep ? () => onEditStep('document-parsing') : undefined}
          editLabel={editLabel}
          testId="wizard-review-document-parsing"
        >
          <Row
            label={t('onboarding.pdfParserHeading', 'PDF parser')}
            value={pdfLabel(draft.pdfParserBackend, t)}
          />
        </Group>
      ) : null}

      <Group
        title={t('onboarding.chunkingSection', 'Chunking')}
        onEdit={onEditStep ? () => onEditStep('chunking') : undefined}
        editLabel={editLabel}
        testId="wizard-review-chunking"
      >
        <Row
          label={t('workspace.chunking.mode', 'Mode')}
          value={
            !draft.chunkingMode || draft.chunkingMode === 'inherit'
              ? t('workspace.chunking.inherit', 'Inherit (fleet env)')
              : draft.chunkingMode === 'adaptive'
                ? t('workspace.chunking.adaptive', 'Adaptive')
                : t('workspace.chunking.fixedSummary', 'Fixed · {{size}}/{{overlap}}', {
                    size: draft.chunkTokenSize,
                    overlap: draft.chunkOverlapTokenSize,
                  })
          }
        />
      </Group>

      {hasGraph ? (
      <>
      <Group
        title={t('onboarding.extractBudgetSection', 'Extract budget')}
        onEdit={onEditStep ? () => onEditStep('extract-budget') : undefined}
        editLabel={editLabel}
        testId="wizard-review-extract-budget"
      >
        <Row
          label={t('workspace.extractBudget.title', 'Extract budget')}
          value={
            draft.extractBudgetMode === 'custom'
              ? t(
                  'workspace.extractBudget.customSummary',
                  'Custom · {{entities}}/{{records}}',
                  {
                    entities: draft.extractMaxEntities,
                    records: draft.extractMaxRecords,
                  },
                )
              : t(
                  'workspace.extractBudget.inherit',
                  'Inherit fleet (usually 40 entities / 100 records)',
                )
          }
        />
      </Group>

      <Group
        title={t('onboarding.extractionSection', 'Extraction')}
        onEdit={onEditStep ? () => onEditStep('extraction') : undefined}
        editLabel={editLabel}
        testId="wizard-review-extraction"
      >
        <Row
          label={t('extractionMode.workspace.mode', 'Default mode')}
          value={
            draft.extractionMode === 'decision'
              ? t('extractionMode.option.decision', 'Decision · closed questions')
              : draft.extractionMode === 'llm'
                ? t('extractionMode.option.llm', 'LLM · reads the text')
                : t('extractionMode.workspace.serverDefault', 'Server default')
          }
        />
        <Row
          label={t('workspace.extractionLanguage', 'Language')}
          value={
            draft.extractionLanguage ||
            formatServerDefaultExtractionLanguageLabel(t)
          }
        />
        <Row label={t('workspace.entityTypes', 'Entity types')} value={entitySample} />
        <Row
          label={t('workspace.relationTypes', 'Relation types')}
          value={
            draft.relationTypes.length === 0
              ? t('relationTypes.freeForm', 'Free-form')
              : draft.relationTypes.length <= 3
                ? draft.relationTypes.join(', ')
                : `${draft.relationTypes.slice(0, 3).join(', ')} +${draft.relationTypes.length - 3}`
          }
        />
        <Row
          label={t('workspace.relationEdges', 'Typed edges')}
          value={
            (draft.relationEdges?.length ?? 0) === 0
              ? t('kgSchema.edgesUnconstrained', 'Unconstrained endpoints')
              : draft.relationEdges.length <= 2
                ? draft.relationEdges
                    .map((e) => `${e.source}—${e.relation}→${e.target}`)
                    .join('; ')
                : `${draft.relationEdges.length} edges`
          }
        />
        {hideWorkspaceIdentity ? (
          <Row
            label={t('onboarding.strictMode', 'Strict')}
            value={
              draft.entityTypesStrict
                ? t('common.enabled', 'Enabled')
                : t('common.disabled', 'Disabled')
            }
          />
        ) : null}
      </Group>
      </>
      ) : null}
    </div>
  );
}
