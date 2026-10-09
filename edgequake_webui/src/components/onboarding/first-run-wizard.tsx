'use client';

import { WizardShell } from '@/components/onboarding/wizard-shell';
import { STEP_META } from '@/components/onboarding/step-meta';
import { AdminCredentialsStep } from '@/components/onboarding/steps/admin-credentials-step';
import { DocumentParsingStep } from '@/components/onboarding/steps/document-parsing-step';
import { ModelDefaultsStep } from '@/components/onboarding/steps/model-defaults-step';
import { ReviewStep } from '@/components/onboarding/steps/review-step';
import { TenantBasicsStep } from '@/components/onboarding/steps/tenant-basics-step';
import { WorkspaceBasicsStep } from '@/components/onboarding/steps/workspace-basics-step';
import { WorkspaceChunkingStep } from '@/components/onboarding/steps/workspace-chunking-step';
import { WorkspaceExtractBudgetStep } from '@/components/onboarding/steps/workspace-extract-budget-step';
import { WorkspaceExtractionStep } from '@/components/onboarding/steps/workspace-extraction-step';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Skeleton } from '@/components/ui/skeleton';
import { ENTITY_PRESETS } from '@/constants/entity-presets';
import { EDGE_PRESETS, RELATION_PRESETS } from '@/constants/kg-schema-presets';
import { useServerModelDefaults } from '@/hooks/use-server-model-defaults';
import { useSetupStatus } from '@/hooks/use-setup-status';
import { useWizardDraftPicks } from '@/hooks/use-wizard-draft-picks';
import { login } from '@/lib/api/edgequake';
import { initializeSetup } from '@/lib/api/setup';
import { buildTenantModelPayload, buildWorkspaceIngestPayload } from '@/lib/onboarding/model-payload';
import { useWizardDraftPersistence } from '@/lib/onboarding/use-wizard-draft-persistence';
import {
  EMPTY_WIZARD_DRAFT,
  canProceed,
  clampStepIndex,
  stepsForWizard,
  type WizardDraft,
  type WizardStepId,
} from '@/lib/onboarding/wizard-state';
import { useAuthStore } from '@/stores/use-auth-store';
import { useTenantStore } from '@/stores/use-tenant-store';
import { useQueryClient } from '@tanstack/react-query';
import { useRouter } from 'next/navigation';
import { useCallback, useMemo, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { toast } from 'sonner';

export interface FirstRunWizardProps {
  /**
   * `login` — empty install before any admin exists.
   * `dashboard` — authenticated but still missing tenant/workspace.
   */
  surface?: 'login' | 'dashboard';
}

function initialFirstRunDraft(): WizardDraft {
  return {
    ...EMPTY_WIZARD_DRAFT,
    tenantName: 'My Organization',
    workspaceName: 'Default Workspace',
    entityTypes: [...ENTITY_PRESETS.general.types],
    relationTypes: [...RELATION_PRESETS.general],
    relationEdges: EDGE_PRESETS.general.map((e) => ({ ...e })),
    kgSchemaPreset: 'general',
  };
}

/**
 * SPEC-101 — Secure first-run onboarding (admin → tenant → workspace).
 */
export function FirstRunWizard({ surface = 'dashboard' }: FirstRunWizardProps) {
  const { t } = useTranslation();
  const router = useRouter();
  const queryClient = useQueryClient();
  const { data: status, isLoading } = useSetupStatus();
  const authLogin = useAuthStore((s) => s.login);
  const selectTenant = useTenantStore((s) => s.selectTenant);
  const selectWorkspace = useTenantStore((s) => s.selectWorkspace);
  const setNeedsOnboarding = useTenantStore((s) => s.setNeedsOnboarding);

  const includeAdmin = Boolean(
    status?.auth_enabled &&
      !status.has_login_users &&
      !status.bootstrap_admin_configured,
  );

  const steps = useMemo(
    () => stepsForWizard('first-run', { includeAdmin, includeExtraction: true }),
    [includeAdmin],
  );

  const [stepIndex, setStepIndex] = useState(0);
  const [draft, setDraft] = useState<WizardDraft>(initialFirstRunDraft);
  const [submitting, setSubmitting] = useState(false);
  const { hasConfiguredDefaults } = useServerModelDefaults();
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

  const open = (() => {
    if (isLoading || !status?.needs_setup) return false;
    // SPEC-163: show the wizard on the Docker quickstart path (auth off) when
    // no tenant exists yet, so users can pick models without a login wall.
    if (!status.auth_enabled) {
      return status.tenant_count === 0;
    }
    if (surface === 'login') {
      return !status.has_login_users;
    }
    return status.has_login_users && status.tenant_count === 0;
  })();

  const { clearDraft } = useWizardDraftPersistence(
    'first-run',
    open,
    draft,
    setDraft,
    stepIndex,
    setStepIndex,
    steps.length,
  );

  const stepId = steps[clampStepIndex(stepIndex, steps.length)];
  const meta = STEP_META[stepId];

  const patchDraft = useCallback((patch: Partial<WizardDraft>) => {
    setDraft((d) => ({ ...d, ...patch }));
  }, []);

  const advancedValid = Boolean(llm?.provider && embedding?.provider && vision?.provider);
  const canGoNext = canProceed(stepId, draft, {
    hasConfiguredDefaults,
    advancedModelsValid: advancedValid,
  });

  const goToStep = (target: WizardStepId) => {
    const idx = steps.indexOf(target);
    if (idx >= 0) setStepIndex(idx);
  };

  const finalize = async () => {
    setSubmitting(true);
    try {
      const models = buildTenantModelPayload({
        useServerDefaults: draft.useServerDefaults,
        llm,
        embedding,
        vision,
        reasoningEffort: draft.reasoningEffort,
      });
      const ingest = buildWorkspaceIngestPayload(draft, 'create');
      const result = await initializeSetup({
        admin_username: includeAdmin ? draft.adminUsername.trim() : undefined,
        admin_email: includeAdmin ? draft.adminEmail.trim() || undefined : undefined,
        admin_password: includeAdmin ? draft.adminPassword : undefined,
        tenant_name: draft.tenantName.trim(),
        tenant_description: draft.tenantDescription.trim() || undefined,
        workspace_name: draft.workspaceName.trim(),
        workspace_slug: draft.workspaceSlug.trim() || undefined,
        workspace_description: draft.workspaceDescription.trim() || undefined,
        ...models,
        ...ingest,
      });
      selectTenant(result.tenant.id);
      selectWorkspace(result.workspace.id);
      setNeedsOnboarding(false);
      clearDraft();
      await queryClient.invalidateQueries({ queryKey: ['setup', 'status'] });
      await queryClient.invalidateQueries({ queryKey: ['tenants'] });
      await queryClient.invalidateQueries({ queryKey: ['workspaces'] });

      if (includeAdmin && draft.adminPassword) {
        try {
          const session = await login({
            username: draft.adminUsername.trim() || 'admin',
            password: draft.adminPassword,
          });
          authLogin(session);
          toast.success(t('onboarding.setupComplete', 'Setup complete. Welcome to EdgeQuake!'));
          router.push('/documents');
          return;
        } catch {
          toast.success(
            t(
              'onboarding.setupCompleteLogin',
              'Setup complete. Please sign in with your new admin account.',
            ),
          );
          return;
        }
      }

      toast.success(t('onboarding.setupComplete', 'Setup complete. Welcome to EdgeQuake!'));
    } catch (error) {
      const message = error instanceof Error ? error.message : 'Unknown error';
      if (message.toLowerCase().includes('already initialized') || message.includes('409')) {
        setNeedsOnboarding(false);
        clearDraft();
        await queryClient.invalidateQueries({ queryKey: ['setup', 'status'] });
        toast.success(t('onboarding.alreadyInitialized', 'Instance already initialized'));
        return;
      }
      toast.error(t('onboarding.setupFailed', 'Setup failed'), { description: message });
    } finally {
      setSubmitting(false);
    }
  };

  const onNext = () => {
    if (stepIndex >= steps.length - 1) {
      void finalize();
      return;
    }
    setStepIndex((i) => clampStepIndex(i + 1, steps.length));
  };

  const body = (() => {
    switch (stepId) {
      case 'admin':
        return <AdminCredentialsStep draft={draft} onChange={patchDraft} />;
      case 'tenant-basics':
        return <TenantBasicsStep draft={draft} onChange={patchDraft} />;
      case 'models':
        return (
          <ModelDefaultsStep
            draft={draft}
            onChange={patchDraft}
            llm={llm}
            embedding={embedding}
            vision={vision}
            onLlmChange={commitLlm}
            onEmbeddingChange={commitEmbedding}
            onVisionChange={commitVision}
            advancedOpen={advancedOpen}
            onAdvancedOpenChange={setAdvancedOpen}
          />
        );
      case 'document-parsing':
        return <DocumentParsingStep draft={draft} onChange={patchDraft} />;
      case 'workspace-basics':
        return <WorkspaceBasicsStep draft={draft} onChange={patchDraft} />;
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
            includeAdmin={includeAdmin}
            includeTenant
            llm={llm}
            embedding={embedding}
            vision={vision}
            onEditStep={goToStep}
            showDocumentParsing
          />
        );
      default:
        return null;
    }
  })();

  if (isLoading && surface === 'login') {
    return (
      <Dialog open>
        <DialogContent
          showCloseButton={false}
          className="sm:max-w-lg"
          data-testid="first-run-wizard-loading"
          onPointerDownOutside={(e) => e.preventDefault()}
          onEscapeKeyDown={(e) => e.preventDefault()}
        >
          <DialogHeader>
            <DialogTitle>{t('onboarding.firstRunTitle', 'Welcome — set up EdgeQuake')}</DialogTitle>
            <DialogDescription>
              {t('onboarding.loadingSetup', 'Checking setup status…')}
            </DialogDescription>
          </DialogHeader>
          <div className="space-y-3 py-2" aria-busy="true">
            <Skeleton className="h-4 w-1/3" />
            <Skeleton className="h-2 w-full" />
            <Skeleton className="h-6 w-2/3" />
            <Skeleton className="h-24 w-full" />
            <Skeleton className="h-9 w-full" />
          </div>
        </DialogContent>
      </Dialog>
    );
  }

  if (!open) return null;

  return (
    <WizardShell
      open={open}
      onOpenChange={() => {
        /* first-run cannot dismiss until complete */
      }}
      title={t('onboarding.firstRunTitle', 'Welcome — set up EdgeQuake')}
      description={t(
        'onboarding.firstRunDescription',
        'Create your admin account, organization, and first workspace.',
      )}
      stepIndex={stepIndex}
      stepCount={steps.length}
      stepTitle={t(meta.titleKey, meta.title)}
      stepDescription={t(meta.descriptionKey, meta.description)}
      canGoNext={canGoNext}
      isLastStep={stepIndex >= steps.length - 1}
      isSubmitting={submitting}
      dismissible={false}
      hideCancel
      showCloseButton={false}
      onBack={() => setStepIndex((i) => clampStepIndex(i - 1, steps.length))}
      onNext={onNext}
      finishLabel={t('onboarding.finishSetup', 'Finish setup')}
      testId="first-run-wizard"
    >
      {body}
    </WizardShell>
  );
}
