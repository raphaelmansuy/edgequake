'use client';

import { WizardShell } from '@/components/onboarding/wizard-shell';
import { resolveStepLabels, resolveStepMeta } from '@/components/onboarding/step-meta';
import { DocumentParsingStep } from '@/components/onboarding/steps/document-parsing-step';
import { ModelDefaultsStep } from '@/components/onboarding/steps/model-defaults-step';
import { ReviewStep } from '@/components/onboarding/steps/review-step';
import { WorkspaceChunkingStep } from '@/components/onboarding/steps/workspace-chunking-step';
import { WorkspaceExtractBudgetStep } from '@/components/onboarding/steps/workspace-extract-budget-step';
import { WorkspaceExtractionStep } from '@/components/onboarding/steps/workspace-extraction-step';
import { useInheritedModelDefaults } from '@/hooks/use-inherited-model-defaults';
import { useSettingsAccess } from '@/hooks/use-settings-context';
import { toSettingsAccess } from '@/lib/settings/access';
import { useWizardGate } from '@/hooks/use-wizard-gate';
import { useWizardSteps } from '@/hooks/use-wizard-steps';
import { useWizardDraftPicks } from '@/hooks/use-wizard-draft-picks';
import { updateWorkspace } from '@/lib/api/edgequake';
import { buildWorkspaceUpdatePayload } from '@/lib/onboarding/model-payload';
import {
  prefillReconfigureFromWorkspace,
  resolveHydratedUseServerDefaults,
  snapshotFromWizardState,
} from '@/lib/onboarding/reconfigure-from-workspace';
import { parseRagProfile } from '@/constants/rag-profile';
import { draftEmbedsImages, imageEmbeddingOk } from '@/lib/onboarding/retrieval-draft';
import { useWizardDraftPersistence } from '@/lib/onboarding/use-wizard-draft-persistence';
import {
  hydrateWizardDraft,
  loadWizardDraft,
} from '@/lib/onboarding/wizard-draft-storage';
import {
  diffWorkspaceConfig,
  toPendingRebuild,
  type WorkspaceConfigSnapshot,
  type WorkspaceRebuildHints,
} from '@/lib/onboarding/workspace-config-diff';
import {
  EMPTY_WIZARD_DRAFT,
  clampStepIndex,
  draftForStorage,
  stepsForWizard,
  type WizardDraft,
} from '@/lib/onboarding/wizard-state';
import type { Workspace } from '@/types';
import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { toast } from 'sonner';

export interface ReconfigureWorkspaceWizardProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  tenantId: string | null;
  workspace: Workspace | null;
  /** Document count for Impact rebuild urgency (EC-101-16…18). */
  documentCount?: number;
  onApplied?: (result: {
    workspace: Workspace;
    pendingRebuild: WorkspaceRebuildHints | null;
    extractionLanguageChanged: boolean;
  }) => void;
}

/**
 * SPEC-101 Wave 8 — Guided Reconfigure Workspace wizard (LAW-101-12).
 */
export function ReconfigureWorkspaceWizard({
  open,
  onOpenChange,
  tenantId,
  workspace,
  documentCount = 0,
  onApplied,
}: ReconfigureWorkspaceWizardProps) {
  const { t } = useTranslation();
  const ragProfile = parseRagProfile(workspace?.rag_profile);
  // SPEC-180: a normal workspace has no extraction step to reconfigure.
  const steps = useMemo(
    () => stepsForWizard('reconfigure-workspace', { includeAdmin: false, includeExtraction: true, ragProfile }),
    [ragProfile],
  );
  const baselineRef = useRef(EMPTY_WIZARD_DRAFT);
  const baselineSnapshotRef = useRef<WorkspaceConfigSnapshot | null>(null);
  const nav = useWizardSteps(steps);
  const { stepIndex, setStepIndex, stepId, goToStep } = nav;
  const [draft, setDraft] = useState<WizardDraft>(EMPTY_WIZARD_DRAFT);
  const [submitting, setSubmitting] = useState(false);
  const workspaceId = workspace?.id ?? null;
  const inherited = useInheritedModelDefaults(tenantId);
  const { hasConfiguredDefaults } = inherited;
  const prefilledForId = useRef<string | null>(null);
  // Persist only after the workspace prefill ran; otherwise the empty first-render draft is saved
  // and then "restored" over the prefill, wiping the workspace's own models and settings.
  const [draftReady, setDraftReady] = useState(false);
  const {
    llm,
    embedding,
    vision,
    advancedOpen,
    commitLlm,
    commitEmbedding,
    commitVision,
    setAdvancedOpen,
  } = useWizardDraftPicks(draft, setDraft);

  const { clearDraft } = useWizardDraftPersistence(
    'reconfigure-workspace',
    open && draftReady,
    draft,
    setDraft,
    stepIndex,
    setStepIndex,
    steps.length,
    workspaceId,
    false, // caller hydrates + prefills (EC-101-22)
  );

  // Prefill from workspace, or restore session draft for this workspace id.
  useEffect(() => {
    if (!open || !workspace) {
      if (!open) {
        prefilledForId.current = null;
        setDraftReady(false);
      }
      return;
    }
    if (prefilledForId.current === workspace.id) return;
    const prefill = prefillReconfigureFromWorkspace(workspace);
    baselineRef.current = prefill.draft;
    baselineSnapshotRef.current = prefill.snapshot;
    const stored = loadWizardDraft('reconfigure-workspace', workspace.id);
    if (stored) {
      const hydrated = hydrateWizardDraft(prefill.draft, stored);
      // Never keep useServerDefaults=true while Advanced is open / picks exist —
      // that makes Apply clear overrides and drop picker selections.
      const advanced =
        stored.modelPicks?.advancedOpen ??
        (prefill.advancedOpen ||
          !hydrated.useServerDefaults ||
          Boolean(
            stored.modelPicks?.llm ||
              stored.modelPicks?.embedding ||
              stored.modelPicks?.vision ||
              prefill.llm ||
              prefill.embedding ||
              prefill.vision,
          ));
      setDraft({
        ...hydrated,
        advancedOpen: advanced,
        useServerDefaults: resolveHydratedUseServerDefaults({
          prefillAdvancedOpen: prefill.advancedOpen,
          prefillUseServerDefaults: prefill.draft.useServerDefaults,
          hasPrefillPicks: Boolean(prefill.llm || prefill.embedding || prefill.vision),
          storedUseServerDefaults: hydrated.useServerDefaults,
        }),
      });
      setStepIndex(clampStepIndex(stored.stepIndex, steps.length));
    } else {
      setDraft(prefill.draft);
      setStepIndex(0);
    }
    prefilledForId.current = workspace.id;
    setDraftReady(true);
  }, [open, workspace, steps.length]);

  const meta = resolveStepMeta(stepId, 'reconfigure-workspace');
  const stepLabels = useMemo(() => resolveStepLabels(steps, t), [steps, t]);

  const patchDraft = useCallback((patch: Partial<WizardDraft>) => {
    setDraft((d) => ({ ...d, ...patch }));
  }, []);

  const currentSnapshot = useMemo(
    () => snapshotFromWizardState({ draft }),
    [draft],
  );

  const impact = useMemo(() => {
    const baseline = baselineSnapshotRef.current;
    if (!baseline) {
      return diffWorkspaceConfig(currentSnapshot, currentSnapshot, {
        documentCount,
      });
    }
    return diffWorkspaceConfig(baseline, currentSnapshot, { documentCount });
  }, [currentSnapshot, documentCount]);

  const access = useSettingsAccess(tenantId, workspaceId);
  // Vision is optional on reconfigure (empty clears the override → server default).
  const { eligibility, canGoNext, blockedReason } = useWizardGate({
    scope: 'workspace',
    id: workspaceId,
    stepId,
    draft,
    inherited,
    llm,
    embedding,
    vision,
    hasConfiguredDefaults,
    hasConfigChanges: stepId === 'review' ? impact.hasChanges : undefined,
    imageEmbeddingOk: imageEmbeddingOk(draft, {
      provider: workspace?.embedding_provider,
      model: workspace?.embedding_model,
    }),
    // The saved model was already accepted for this workspace, so only a *new* image pick
    // needs the deployment's answer.
    probeImages:
      draftEmbedsImages(draft) &&
      !(
        draft.embeddingPick?.provider === workspace?.embedding_provider &&
        draft.embeddingPick?.model === workspace?.embedding_model
      ),
  });

  const isDirty =
    stepIndex > 0 ||
    JSON.stringify(draftForStorage(draft)) !==
      JSON.stringify(draftForStorage(baselineRef.current)) ||
    impact.hasChanges;

  const reset = useCallback(() => {
    prefilledForId.current = null;
    setDraftReady(false);
    baselineRef.current = EMPTY_WIZARD_DRAFT;
    baselineSnapshotRef.current = null;
    nav.restart();
    setDraft(EMPTY_WIZARD_DRAFT);
    clearDraft();
  }, [clearDraft]);

  const handleOpenChange = (next: boolean) => {
    if (!next) reset();
    onOpenChange(next);
  };

  const finalize = async () => {
    if (!tenantId || !workspace) {
      toast.error(t('workspace.noWorkspaceSelected', 'No Workspace Selected'));
      return;
    }
    if (!impact.hasChanges) {
      toast.info(t('onboarding.noChanges', 'No changes to apply'));
      return;
    }
    setSubmitting(true);
    try {
      const payload = buildWorkspaceUpdatePayload({
        useServerDefaults: draft.useServerDefaults,
        llm,
        embedding,
        vision,
        savedLlmRoles: workspace.llm_roles,
        pdfParserBackend: draft.pdfParserBackend,
        visionExtractImages: draft.visionExtractImages,
        visionExtractCharts: draft.visionExtractCharts,
        visionExtractFigures: draft.visionExtractFigures,
        visionPageSystemPrompt: draft.visionPageSystemPrompt,
        visionImageSystemPrompt: draft.visionImageSystemPrompt,
        visionChartSystemPrompt: draft.visionChartSystemPrompt,
        visionFigureSystemPrompt: draft.visionFigureSystemPrompt,
        extractionLanguage: draft.extractionLanguage,
        chunkingMode: draft.chunkingMode,
        chunkTokenSize: draft.chunkTokenSize,
        chunkOverlapTokenSize: draft.chunkOverlapTokenSize,
        extractBudgetMode: draft.extractBudgetMode,
        extractMaxEntities: draft.extractMaxEntities,
        extractMaxRecords: draft.extractMaxRecords,
        entityTypes: draft.entityTypes,
        entityTypesStrict: draft.entityTypesStrict,
        entityTypeColors: draft.entityTypeColors,
        relationTypes: draft.relationTypes,
        relationTypesStrict: draft.relationTypesStrict,
        kgSchemaPreset: draft.kgSchemaPreset,
        relationEdges: draft.relationEdges,
        reasoningEffort: draft.reasoningEffort,
        decisionEnabled: draft.decisionEnabled,
        decisionModel: draft.decisionModel,
        decisionPreset: draft.decisionPreset,
        decisionPackSize: draft.decisionPackSize,
        extractionMode: draft.extractionMode,
        changedKeys: impact.changedKeys,
      });
      const updated = await updateWorkspace(tenantId, workspace.id, payload);
      if (updated.embedding_rebuild_required) {
        toast.warning(
          t('workspace.embeddingRebuildRequired', 'Embeddings must be rebuilt'),
          {
            description: t(
              'workspace.embeddingRebuildDescription',
              'The embedding model changed (dimension {{from}} → {{to}}). Rebuild embeddings to re-index documents.',
              {
                from: updated.previous_embedding_dimension ?? '?',
                to: updated.resolved_embedding_dimension ?? '?',
              },
            ),
          },
        );
      }
      const pendingRebuild = toPendingRebuild(impact.rebuildHints);
      const extractionLanguageChanged = impact.changedKeys.includes('extractionLanguage');
      onApplied?.({
        workspace: updated,
        pendingRebuild,
        extractionLanguageChanged,
      });
      handleOpenChange(false);
    } catch (error) {
      const message = error instanceof Error ? error.message : 'Unknown error';
      if (/404|not found/i.test(message)) {
        toast.error(t('workspace.notFound', 'Workspace Not Found'));
        handleOpenChange(false);
        return;
      }
      toast.error(t('workspace.updateFailed', 'Failed to update workspace'), {
        description: message,
      });
    } finally {
      setSubmitting(false);
    }
  };

  const onNext = () => {
    if (nav.isLastStep) {
      void finalize();
      return;
    }
    nav.advance();
  };

  const body = (() => {
    switch (stepId) {
      case 'models':
        return (
          <ModelDefaultsStep
            draft={draft}
            onChange={patchDraft}
            tenantId={tenantId}
            choiceScope="workspace"
            llm={llm}
            embedding={embedding}
            vision={vision}
            onLlmChange={commitLlm}
            onEmbeddingChange={commitEmbedding}
            onVisionChange={commitVision}
            advancedOpen={advancedOpen}
            onAdvancedOpenChange={setAdvancedOpen}
            eligibility={eligibility}
            canManageProviders={access.data ? toSettingsAccess(access.data).workspaceManage : false}
            hidesExtractionEngine={draft.ragProfile === 'normal'}
            requiresImageEmbedding={draftEmbedsImages(draft)}
          />
        );
      case 'document-parsing':
        return <DocumentParsingStep draft={draft} onChange={patchDraft} />;
      case 'chunking':
        return <WorkspaceChunkingStep draft={draft} onChange={patchDraft} />;
      case 'extract-budget':
        return <WorkspaceExtractBudgetStep draft={draft} onChange={patchDraft} />;
      case 'extraction':
        return (
          <WorkspaceExtractionStep draft={draft} onChange={patchDraft} showStrict />
        );
      case 'review':
        return (
          <ReviewStep
            draft={draft}
            includeAdmin={false}
            includeTenant={false}
            tenantId={tenantId}
            llm={llm}
            embedding={embedding}
            vision={vision}
            onEditStep={goToStep}
            steps={steps}
            hideWorkspaceIdentity
            showDocumentParsing
            showRetrieval
            impact={impact}
            documentCount={documentCount}
          />
        );
      default:
        return null;
    }
  })();

  return (
    <WizardShell
      open={open}
      onOpenChange={handleOpenChange}
      title={t('onboarding.reconfigureTitle', 'Reconfigure workspace')}
      description={t(
        'onboarding.reconfigureDescription',
        'Update models, document parsing, chunking, and extraction for this workspace.',
      )}
      stepIndex={stepIndex}
      stepCount={steps.length}
      stepTitle={t(meta.titleKey, meta.title)}
      stepDescription={t(meta.descriptionKey, meta.description)}
      canGoNext={canGoNext}
      stepLabels={stepLabels}
      onStepSelect={nav.goToIndex}
      blockedReason={blockedReason}
      isLastStep={nav.isLastStep}
      isSubmitting={submitting}
      isDirty={isDirty}
      onBack={nav.goBack}
      onNext={onNext}
      finishLabel={t('onboarding.applyChanges', 'Apply')}
      testId="reconfigure-workspace-wizard"
    >
      {body}
    </WizardShell>
  );
}
