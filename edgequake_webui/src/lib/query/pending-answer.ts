/**
 * Pure model for the "waiting for the first token" state of an answer.
 *
 * The pipeline always walks Search → Read → Think → Write, so the UI can show
 * real progress (not a bare spinner) and reassure the user when a slow local
 * model is simply taking its time.
 */
import type { StreamStage } from "./stream-session-reducer";

export const PENDING_STEPS = ["retrieving", "reading", "thinking", "generating"] as const;
export type PendingStep = (typeof PENDING_STEPS)[number];

export type StepState = "done" | "active" | "todo";

/** Index of the active step, or -1 when the stage is not part of the happy path. */
export function activeStepIndex(stage: StreamStage | null): number {
  if (!stage) return 0; // a fresh stream starts by searching
  return PENDING_STEPS.indexOf(stage as PendingStep);
}

export function stepStates(stage: StreamStage | null): StepState[] {
  const active = activeStepIndex(stage);
  return PENDING_STEPS.map((_, i) =>
    active < 0 ? "todo" : i < active ? "done" : i === active ? "active" : "todo",
  );
}

export type WaitHint = "none" | "working" | "slow";

/** Seconds before we show the elapsed counter. */
export const ELAPSED_VISIBLE_AFTER_S = 3;
export const WORKING_HINT_AFTER_S = 8;
export const SLOW_HINT_AFTER_S = 20;

export function waitHint(elapsedSeconds: number): WaitHint {
  if (elapsedSeconds >= SLOW_HINT_AFTER_S) return "slow";
  if (elapsedSeconds >= WORKING_HINT_AFTER_S) return "working";
  return "none";
}

export function elapsedSeconds(startedAt: number | undefined, now: number): number {
  if (!startedAt) return 0;
  return Math.max(0, Math.floor((now - startedAt) / 1000));
}
