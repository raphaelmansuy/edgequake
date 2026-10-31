import { describe, expect, it } from 'vitest';
import {
  EMPTY_WIZARD_DRAFT,
  canProceed,
  clampStepIndex,
  draftForStorage,
  progressPercent,
  stepsForWizard,
} from '../wizard-state';

describe('wizard-state', () => {
  it('builds create-tenant steps with ingest tuning before extraction', () => {
    expect(stepsForWizard('create-tenant')).toEqual([
      'tenant-basics',
      'models',
      'document-parsing',
      'workspace-basics',
      'chunking',
      'extract-budget',
      'extraction',
      'review',
    ]);
  });

  it('builds reconfigure-workspace steps (LAW-101-12 + ingest tuning)', () => {
    expect(stepsForWizard('reconfigure-workspace')).toEqual([
      'models',
      'document-parsing',
      'chunking',
      'extract-budget',
      'extraction',
      'review',
    ]);
  });

  it('builds create-workspace steps with ingest tuning before extraction', () => {
    expect(
      stepsForWizard('create-workspace', {
        includeAdmin: false,
        includeExtraction: true,
      }),
    ).toEqual([
      'workspace-basics',
      'retrieval',
      'models',
      'document-parsing',
      'chunking',
      'extract-budget',
      'extraction',
      'review',
    ]);
  });

  it('offers a normal workspace no extract-budget or extraction step', () => {
    expect(
      stepsForWizard('create-workspace', {
        includeAdmin: false,
        includeExtraction: true,
        ragProfile: 'normal',
      }),
    ).toEqual(['workspace-basics', 'retrieval', 'models', 'document-parsing', 'chunking', 'review']);
  });

  it('disables review Apply when reconfigure has no config changes', () => {
    expect(
      canProceed('review', EMPTY_WIZARD_DRAFT, {
        hasConfiguredDefaults: true,
        advancedModelsValid: true,
        hasConfigChanges: false,
      }),
    ).toBe(false);
    expect(
      canProceed('review', EMPTY_WIZARD_DRAFT, {
        hasConfiguredDefaults: true,
        advancedModelsValid: true,
        hasConfigChanges: true,
      }),
    ).toBe(true);
  });

  it('allows document-parsing step always', () => {
    expect(canProceed('document-parsing', EMPTY_WIZARD_DRAFT)).toBe(true);
  });

  it('blocks chunking Next when fixed pair is invalid', () => {
    expect(canProceed('chunking', EMPTY_WIZARD_DRAFT)).toBe(true);
    expect(
      canProceed('chunking', {
        ...EMPTY_WIZARD_DRAFT,
        chunkingMode: 'fixed',
        chunkTokenSize: 100,
        chunkOverlapTokenSize: 100,
      }),
    ).toBe(false);
    expect(
      canProceed('chunking', {
        ...EMPTY_WIZARD_DRAFT,
        chunkingMode: 'fixed',
        chunkTokenSize: 1200,
        chunkOverlapTokenSize: 100,
      }),
    ).toBe(true);
  });

  it('blocks extract-budget Next when custom pair is invalid', () => {
    expect(canProceed('extract-budget', EMPTY_WIZARD_DRAFT)).toBe(true);
    expect(
      canProceed('extract-budget', {
        ...EMPTY_WIZARD_DRAFT,
        extractBudgetMode: 'custom',
        extractMaxEntities: 50,
        extractMaxRecords: 40,
      }),
    ).toBe(false);
    expect(
      canProceed('extract-budget', {
        ...EMPTY_WIZARD_DRAFT,
        extractBudgetMode: 'custom',
        extractMaxEntities: 40,
        extractMaxRecords: 100,
      }),
    ).toBe(true);
  });

  it('includes admin on first-run when requested', () => {
    expect(stepsForWizard('first-run', { includeAdmin: true, includeExtraction: true })).toEqual([
      'admin',
      'tenant-basics',
      'models',
      'document-parsing',
      'workspace-basics',
      'chunking',
      'extract-budget',
      'extraction',
      'review',
    ]);
  });

  it('validates admin password match and length', () => {
    const draft = {
      ...EMPTY_WIZARD_DRAFT,
      adminUsername: 'admin',
      adminPassword: 'short',
      adminPasswordConfirm: 'short',
    };
    expect(canProceed('admin', draft)).toBe(false);
    draft.adminPassword = 'longenough';
    draft.adminPasswordConfirm = 'longenough';
    expect(canProceed('admin', draft)).toBe(true);
  });

  it('requires tenant and workspace names', () => {
    expect(canProceed('tenant-basics', EMPTY_WIZARD_DRAFT)).toBe(false);
    expect(canProceed('tenant-basics', { ...EMPTY_WIZARD_DRAFT, tenantName: 'Acme' })).toBe(true);
    expect(canProceed('workspace-basics', { ...EMPTY_WIZARD_DRAFT, workspaceName: 'Main' })).toBe(
      true,
    );
  });

  it('requires configured defaults when using server defaults', () => {
    expect(
      canProceed('models', { ...EMPTY_WIZARD_DRAFT, useServerDefaults: true }, {
        hasConfiguredDefaults: false,
        advancedModelsValid: false,
      }),
    ).toBe(false);
    expect(
      canProceed('models', { ...EMPTY_WIZARD_DRAFT, useServerDefaults: true }, {
        hasConfiguredDefaults: true,
        advancedModelsValid: false,
      }),
    ).toBe(true);
  });

  it('clamps step index and computes progress', () => {
    expect(clampStepIndex(-1, 4)).toBe(0);
    expect(clampStepIndex(9, 4)).toBe(3);
    expect(progressPercent(0, 4)).toBe(25);
    expect(progressPercent(3, 4)).toBe(100);
  });

  it('never persists passwords in storage draft', () => {
    const safe = draftForStorage({
      ...EMPTY_WIZARD_DRAFT,
      adminPassword: 'secret-value',
      adminPasswordConfirm: 'secret-value',
      tenantName: 'Acme',
    });
    expect(safe).not.toHaveProperty('adminPassword');
    expect(safe).not.toHaveProperty('adminPasswordConfirm');
    expect(safe.tenantName).toBe('Acme');
  });
});
