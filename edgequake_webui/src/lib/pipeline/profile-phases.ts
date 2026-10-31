/**
 * SPEC-180 — what an ingest actually does, per retrieval profile.
 *
 * WHY this exists: a `normal` workspace (classical RAG) never extracts entities, never
 * gleans, never merges a graph. A `graph` workspace (GraphRAG) does all three. One shared
 * four-phase strip claimed "Extract / Materialize" for both, so a Normal ingest appeared to
 * be building a knowledge graph it will never have — and the progress could stall at
 * "Extract" with nothing to report.
 *
 * The UI must describe the work that is really happening. This module is the single place
 * that maps a run onto its profile's real stages, so the strip, its tooltips and the tests
 * cannot disagree.
 */

import {
  mapWireStageToPhase,
  type IngestionRunView,
} from "@/lib/pipeline/ingestion-run-view";
import type { RagProfile } from "@/constants/rag-profile";

/** The phases a profile can show. `extract` exists only for GraphRAG. */
export type ProfilePhaseId = "admit" | "prepare" | "extract" | "index";

/** Wire stages that only a graph ingest runs. */
const GRAPH_ONLY_STAGES = new Set([
  "extracting",
  "gleaning",
  "merging",
  "summarizing",
]);

/**
 * Wire stages both profiles run. `storing` belongs here because a Normal ingest still
 * persists chunks and vectors; only the graph projection differs.
 */
const SHARED_STAGES = new Set([
  "cleaning",
  "queued",
  "uploading",
  "converting",
  "preprocessing",
  "chunking",
  "embedding",
  "storing",
  "projecting",
  "indexing",
  "processing",
]);

export interface ProfilePhase {
  id: ProfilePhaseId;
  /** Short label in the strip. */
  label: string;
  /** One sentence: what this phase produces for this profile. */
  hint: string;
  /** Wire stages that light this phase up, in order. */
  stages: readonly string[];
}

const GRAPH_PHASES: readonly ProfilePhase[] = [
  {
    id: "admit",
    label: "Admit",
    hint: "Upload accepted, queued for a worker",
    stages: ["cleaning", "queued", "uploading"],
  },
  {
    id: "prepare",
    label: "Prepare",
    hint: "PDF converted, text split into chunks",
    stages: ["converting", "preprocessing", "chunking"],
  },
  {
    id: "extract",
    label: "Extract",
    hint: "Entities and relationships read by the LLM, then merged into the graph",
    stages: ["extracting", "gleaning", "merging", "summarizing"],
  },
  {
    id: "index",
    label: "Index",
    hint: "Embeddings and graph projections stored",
    stages: ["embedding", "storing", "projecting"],
  },
] as const;

/**
 * Classical RAG has two phases after Prepare, and they say what really happens: vectors are
 * computed, then chunks and vectors are stored. There is no extraction and no graph.
 */
const NORMAL_PHASES: readonly ProfilePhase[] = [
  {
    id: "admit",
    label: "Admit",
    hint: "Upload accepted, queued for a worker",
    stages: ["cleaning", "queued", "uploading"],
  },
  {
    id: "prepare",
    label: "Prepare",
    hint: "PDF converted, text split into chunks",
    stages: ["converting", "preprocessing", "chunking"],
  },
  {
    id: "index",
    label: "Index",
    hint: "Chunks embedded and stored for retrieval",
    stages: ["embedding", "storing", "projecting"],
  },
] as const;

export function phasesForProfile(profile: RagProfile): readonly ProfilePhase[] {
  return profile === "normal" ? NORMAL_PHASES : GRAPH_PHASES;
}

/** Index of the phase a wire stage belongs to, or 0 when unknown. */
export function phaseIndexOf(
  phases: readonly ProfilePhase[],
  wireStage: string | undefined | null,
): number {
  const key = (wireStage || "").toLowerCase();
  const found = phases.findIndex((phase) => phase.stages.includes(key));
  if (found >= 0) return found;
  // A stage this profile never runs (e.g. `extracting` reported on a normal workspace):
  // do NOT fall back to phase 0, which would paint progress as having gone backwards.
  // Land on the last phase the profile does have — that is where such a run really is.
  return PHASE_FALLBACK[key] !== undefined
    ? Math.min(phases.length - 1, PHASE_FALLBACK[key]!)
    : 0;
}

/**
 * Where a stage that this profile lacks should land.
 *
 * `extracting`/`gleaning`/`merging` on a normal workspace happen in place of vector work, so
 * they belong to the final phase, not to the start of the run.
 */
const PHASE_FALLBACK: Record<string, number> = {
  extracting: 2,
  gleaning: 2,
  merging: 2,
  summarizing: 2,
};

export function activeProfilePhase(
  profile: RagProfile,
  wireStage: string | undefined | null,
): ProfilePhaseId {
  const phases = phasesForProfile(profile);
  return phases[phaseIndexOf(phases, wireStage)]!.id;
}

/**
 * True when this run is executing (or reporting) a stage its profile does not have.
 *
 * A Normal workspace reporting `extracting` means the server and the UI disagree about what
 * this workspace is. Surfacing it is better than drawing an "Extract" phase that can never
 * complete — but it is not something the user can fix, so the strip warns rather than fails.
 */
export function isStageOutsideProfile(
  profile: RagProfile,
  wireStage: string | undefined | null,
): boolean {
  const key = (wireStage || "").toLowerCase();
  if (!key) return false;
  const known = GRAPH_ONLY_STAGES.has(key) || SHARED_STAGES.has(key);
  if (!known) return false;
  return profile === "normal" && GRAPH_ONLY_STAGES.has(key);
}

/**
 * The four-phase ids the strip painted before SPEC-180 modelled profiles. Kept so the
 * shared `mapWireStageToPhase` and existing e2e markers still resolve.
 */
export function legacyPhaseFor(stage: string | undefined | null): string {
  return mapWireStageToPhase(stage);
}

/** A run's profile phases plus where it currently is, for the strip. */
export interface RunProfilePhases {
  phases: readonly ProfilePhase[];
  activeIndex: number;
  activeId: ProfilePhaseId;
}

export function runProfilePhases(
  run: Pick<IngestionRunView, "stage">,
  profile: RagProfile,
): RunProfilePhases {
  const phases = phasesForProfile(profile);
  const activeIndex = phaseIndexOf(phases, run.stage);
  return { phases, activeIndex, activeId: phases[activeIndex]!.id };
}
