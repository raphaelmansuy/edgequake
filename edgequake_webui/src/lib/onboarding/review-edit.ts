/**
 * SPEC-183 — a review "Edit" link may only point at a step the wizard actually has.
 *
 * The reconfigure wizard for a normal workspace is `models → parsing → chunking → review`:
 * there is no `retrieval` step, because the profile is immutable (LAW-180-1). `ReviewStep`
 * still printed a Retrieval group with an Edit button, and `goToStep('retrieval')` looked up
 * an index that was not in the list, found `-1`, and did nothing. The button looked alive
 * and was not — the worst kind of control.
 *
 * The rule is one function so the review cannot drift from the step list again: ask the
 * wizard, do not assume.
 */

import type { WizardStepId } from '@/lib/onboarding/wizard-state';

/** The Edit callback a review group should receive, or `undefined` to print no link. */
export function editHandlerFor(
  steps: readonly WizardStepId[],
  step: WizardStepId,
  goToStep: ((target: WizardStepId) => void) | undefined,
): (() => void) | undefined {
  if (!goToStep) return undefined;
  // A step outside this wizard's list has nowhere to go, so it gets no control at all.
  if (!steps.includes(step)) return undefined;
  return () => goToStep(step);
}
