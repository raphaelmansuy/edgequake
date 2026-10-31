'use client';

import { WizardShell } from '@/components/onboarding/wizard-shell';
import { resolveStepLabels, resolveStepMeta } from '@/components/onboarding/step-meta';
import { DocumentParsingStep } from '@/components/onboarding/steps/document-parsing-step';
import { ModelDefaultsStep } from '@/components/onboarding/steps/model-defaults-step';
import { ReviewStep } from '@/components/onboarding/steps/review-step';
import { WorkspaceBasicsStep } from '@/components/onboarding/steps/workspace-basics-step';
import { WorkspaceChunkingStep } from '@/components/onboarding/steps/workspace-chunking-step';
import { WorkspaceExtractBudgetStep } from '@/components/onboarding/steps/workspace-extract-budget-step';
import { WorkspaceRetrievalStep } from '@/components/onboarding/steps/workspace-retrieval-step';
import { WorkspaceExtractionStep } from '@/components/onboarding/steps/workspace-extraction-step';
import { ENTITY_PRESETS } from '@/constants/entity-presets';
import { EDGE_PRESETS, RELATION_PRESETS } from '@/constants/kg-schema-presets';
import { useInheritedModelDefaults } from '@/hooks/use-inherited-model-defaults';
import { useSettingsAccess } from '@/hooks/use-settings-context';
import { useWizardGate } from '@/hooks/use-wizard-gate';
import { useWizardSteps } from '@/hooks/use-wizard-steps';
import { describeCreateFailure } from '@/lib/onboarding/create-error';
import { useWizardDraftPicks } from '@/hooks/use-wizard-draft-picks';
import { createWorkspace } from '@/lib/api/edgequake';
import { buildWorkspaceIngestPayload, buildWorkspaceModelPayload } from '@/lib/onboarding/model-payload';
import { draftEmbedsImages, imageEmbeddingOk } from '@/lib/onboarding/retrieval-draft';
import { useWizardDraftPersistence } from '@/lib/onboarding/use-wizard-draft-persistence';
import {
  EMPTY_WIZARD_DRAFT,
  draftForStorage,
  stepsForWizard,
  type WizardDraft,
} from '@/lib/onboarding/wizard-state';
import type { Workspace } from '@/types';
import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { toast } from 'sonner';

export interface CreateWorkspaceWizardProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  tenantId: string | null;
  onCreated?: (workspace: Workspace) => void;
}

function initialWorkspaceDraft(): WizardDraft {
  return {
    ...EMPTY_WIZARD_DRAFT,
    entityTypes: [...ENTITY_PRESETS.general.types],
    relationTypes: [...RELATION_PRESETS.general],
    relationEdges: EDGE_PRESETS.general.map((e) => ({ ...e })),
    kgSchemaPreset: 'general',
  };
}

/**
 * SPEC-101 — Guided Create Workspace wizard.
 */
export function CreateWorkspaceWizard({
  open,
  onOpenChange,
  tenantId,
  onCreated,
}: CreateWorkspaceWizardProps) {
  const { t } = useTranslation();
  const baselineRef = useRef(initialWorkspaceDraft());
  const [draft, setDraft] = useState<WizardDraft>(initialWorkspaceDraft);
  // SPEC-180: choosing Normal removes the graph-only steps.
  const steps = useMemo(
    () =>
      stepsForWizard('create-workspace', {
        includeAdmin: false,
        includeExtraction: true,
        ragProfile: draft.ragProfile,
      }),
    [draft.ragProfile],
  );
  const nav = useWizardSteps(steps);
  const { stepIndex, setStepIndex, stepId, goToStep } = nav;
  const [submitting, setSubmitting] = useState(false);
  const inherited = useInheritedModelDefaults(tenantId);
  const { hasConfiguredDefaults } = inherited;
  const languagePrefillDone = useRef(false);
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
    'create-workspace',
    open,
    draft,
    setDraft,
    stepIndex,
    setStepIndex,
    steps.length,
  );

  const meta = resolveStepMeta(stepId, 'create-workspace');
  const stepLabels = useMemo(() => resolveStepLabels(steps, t), [steps, t]);

  const patchDraft = useCallback((patch: Partial<WizardDraft>) => {
    setDraft((d) => ({ ...d, ...patch }));
  }, []);

  // Prefill extraction language from tenant Default Workspace once (do not overwrite edits).
  useEffect(() => {
    if (!open) {
      languagePrefillDone.current = false;
      return;
    }
    if (languagePrefillDone.current || inherited.isLoading) return;
    if (inherited.extractionLanguage) {
      setDraft((d) =>
        d.extractionLanguage == null
          ? { ...d, extractionLanguage: inherited.extractionLanguage }
          : d,
      );
    }
    languagePrefillDone.current = true;
  }, [open, inherited.isLoading, inherited.extractionLanguage]);

  const access = useSettingsAccess(tenantId, null);
  // SPEC-179: the pair that will be saved must belong to this organization's offer.
  const { eligibility, eligibilityOk, canGoNext, blockedReason } = useWizardGate({
    scope: 'tenant',
    id: tenantId,
    stepId,
    draft,
    inherited,
    llm,
    embedding,
    vision,
    hasConfiguredDefaults,
    imageEmbeddingOk: imageEmbeddingOk(draft),
    // SPEC-180 08: an image draft asks the deployment before Review, not after create.
    probeImages: draftEmbedsImages(draft),
  });

  const isDirty =
    stepIndex > 0 ||
    JSON.stringify(draftForStorage(draft)) !==
      JSON.stringify(draftForStorage(baselineRef.current));

  const reset = useCallback(() => {
    const next = initialWorkspaceDraft();
    baselineRef.current = next;
    languagePrefillDone.current = false;
    nav.restart();
    setDraft(next);
    clearDraft();
  }, [clearDraft]);

  const handleOpenChange = (next: boolean) => {
    if (!next) reset();
    onOpenChange(next);
  };

  const finalize = async () => {
    if (!tenantId) {
      toast.error(t('workspace.noTenant', 'No tenant selected'));
      return;
    }
    if (!eligibilityOk) {
      goToStep('models');
      return;
    }
    setSubmitting(true);
    try {
      const models = buildWorkspaceModelPayload({
        useServerDefaults: draft.useServerDefaults,
        llm,
        embedding,
        vision,
        reasoningEffort: draft.reasoningEffort,
      });
      const ingest = buildWorkspaceIngestPayload(draft, 'create');
      const workspace = await createWorkspace(tenantId, {
        name: draft.workspaceName.trim(),
        slug: draft.workspaceSlug.trim() || undefined,
        description: draft.workspaceDescription.trim() || undefined,
        ...models,
        ...ingest,
      });
      // Success toast + navigation CTA owned by caller (header / tenant-guard).
      onCreated?.(workspace);
      handleOpenChange(false);
    } catch (error) {
      const failure = describeCreateFailure(error);
      toast.error(failure.title, {
        description: failure.description,
      });
      // A refused model sends the user to the step that fixes it; nothing was created.
      if (failure.step) goToStep(failure.step);
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
      case 'workspace-basics':
        return <WorkspaceBasicsStep draft={draft} onChange={patchDraft} />;
      case 'retrieval':
        return <WorkspaceRetrievalStep draft={draft} onChange={patchDraft} />;
      case 'models':
        return (
          <ModelDefaultsStep
            draft={draft}
            onChange={patchDraft}
            tenantId={tenantId}
            llm={llm}
            embedding={embedding}
            vision={vision}
            onLlmChange={commitLlm}
            onEmbeddingChange={commitEmbedding}
            onVisionChange={commitVision}
            advancedOpen={advancedOpen}
            onAdvancedOpenChange={setAdvancedOpen}
            eligibility={eligibility}
            canManageProviders={access.data?.tenant_manage === true}
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
        return <WorkspaceExtractionStep draft={draft} onChange={patchDraft} />;
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
            showDocumentParsing
            showRetrieval
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
      title={t('workspace.createNew', 'Create New Workspace')}
      description={t(
        'workspace.createDescription',
        'Create a new workspace within the current tenant to organize your documents.',
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
      finishLabel={t('onboarding.createWorkspace', 'Create workspace')}
      testId="create-workspace-wizard"
    >
      {body}
    </WizardShell>
  );
}
