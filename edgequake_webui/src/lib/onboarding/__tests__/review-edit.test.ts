import { describe, expect, it, vi } from 'vitest';
import { editHandlerFor } from '../review-edit';
import type { WizardStepId } from '../wizard-state';

const normalReconfigure: WizardStepId[] = ['models', 'document-parsing', 'chunking', 'review'];
const graphReconfigure: WizardStepId[] = [
  'models',
  'document-parsing',
  'extraction',
  'review',
];

describe('SPEC-183 review Edit reachability', () => {
  it('offers no Edit for a step the wizard does not have', () => {
    // The bug: a normal workspace has no retrieval step, so its Edit link was a dead button.
    expect(editHandlerFor(normalReconfigure, 'retrieval', vi.fn())).toBeUndefined();
  });

  it('offers Edit for a step that is present, and calls back with that step', () => {
    const goToStep = vi.fn();
    const handler = editHandlerFor(normalReconfigure, 'models', goToStep);
    expect(handler).toBeTypeOf('function');
    handler?.();
    expect(goToStep).toHaveBeenCalledWith('models');
  });

  it('offers no Edit when the wizard has no navigation at all', () => {
    expect(editHandlerFor(graphReconfigure, 'models', undefined)).toBeUndefined();
  });

  it('a graph reconfigure still reaches retrieval only if the step exists', () => {
    expect(editHandlerFor(graphReconfigure, 'extraction', vi.fn())).toBeTypeOf('function');
    expect(editHandlerFor(graphReconfigure, 'retrieval', vi.fn())).toBeUndefined();
  });
});
