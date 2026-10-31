/**
 * Plain-language reason a wizard step cannot advance (the footer hint).
 * Pure: one `WizardBlockerCode` in, one short sentence out — never a dead button.
 */

import { blockedRoles, ROLE_LABEL, type Verdicts, type WizardRole } from '@/lib/onboarding/defaults-eligibility';
import type { WizardBlockerCode } from '@/lib/onboarding/wizard-state';

export type BlockerTranslate = (
  key: string,
  fallback: string,
  options?: Record<string, string>,
) => string;

export interface BlockerContext {
  /** Per-role eligibility verdicts (models step). */
  verdicts?: Verdicts;
  /** True when no provider is offered at all. */
  nothingOffered?: boolean;
  /** Whose offer the refusal is about. Workspace names the workspace offer. */
  scope?: 'deployment' | 'tenant' | 'workspace';
  /** SPEC-180 08: the server's own sentence about why the image embed was refused. */
  imageProbeMessage?: string;
}

const roleList = (roles: WizardRole[]) => roles.map((r) => ROLE_LABEL[r]).join(', ');

function describeNotEligible(t: BlockerTranslate, ctx: BlockerContext): string {
  if (ctx.nothingOffered) {
    if (ctx.scope === 'workspace') {
      return t(
        'onboarding.blocked.nothingOfferedWorkspace',
        'No provider is offered to this workspace yet.',
      );
    }
    return t(
      'onboarding.blocked.nothingOffered',
      'No provider is offered to this organization yet.',
    );
  }
  const roles = ctx.verdicts ? blockedRoles(ctx.verdicts) : [];
  if (roles.length === 0) {
    return t('onboarding.blocked.notEligible', 'Replace the model marked above to continue.');
  }
  if (ctx.scope === 'workspace') {
    return t(
      'onboarding.blocked.notEligibleWorkspace',
      'Turn on a provider for this workspace, or choose another model for: {{roles}}.',
      { roles: roleList(roles) },
    );
  }
  return t('onboarding.blocked.notEligibleRoles', 'Choose another model for: {{roles}}.', {
    roles: roleList(roles),
  });
}

/** Message for a blocker code; `null` when nothing blocks. */
export function describeBlocker(
  code: WizardBlockerCode | null,
  t: BlockerTranslate,
  ctx: BlockerContext = {},
): string | null {
  switch (code) {
    case null:
      return null;
    case 'workspace_name_required':
      return t('onboarding.blocked.workspaceName', 'Enter a workspace name to continue.');
    case 'tenant_name_required':
      return t('onboarding.blocked.tenantName', 'Enter an organization name to continue.');
    case 'admin_invalid':
      return t(
        'onboarding.blocked.admin',
        'Use a username of 3+ characters and matching passwords of 8+.',
      );
    case 'models_not_eligible':
      return describeNotEligible(t, ctx);
    case 'embedding_not_multimodal':
      // The picker already shows an inline notice with this exact advice; do not repeat it
      // in the footer with different words.
      return null;
    case 'embedding_probe_failed':
      // The server's sentence names the project and the fix; fall back to the generic one.
      return (
        ctx.imageProbeMessage ??
        t(
          'ragProfile.blocked.embeddingProbeFailed',
          'This deployment cannot embed images with that model. Choose another image embedding model, or ask a deployment admin to fix its credentials.',
        )
      );
    case 'embedding_probe_checking':
      return t(
        'ragProfile.blocked.embeddingProbeChecking',
        'Checking that this deployment can embed images…',
      );
    case 'defaults_incomplete':
      return t(
        'onboarding.blocked.defaultsIncomplete',
        'No default models are configured. Choose models for this workspace.',
      );
    case 'models_incomplete':
      return t(
        'onboarding.blocked.modelsIncomplete',
        'Pick at least one model to continue.',
      );
    case 'chunking_invalid':
      return t(
        'onboarding.blocked.chunking',
        'Fix the fixed chunk size and overlap to continue.',
      );
    case 'extract_budget_invalid':
      return t('onboarding.blocked.extractBudget', 'Enter valid entity and record caps to continue.');
    case 'no_changes':
      return t('onboarding.blocked.noChanges', 'Nothing has changed yet. Edit a step to apply.');
  }
}
