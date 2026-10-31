/**
 * SPEC-180 — what a create or update request says about the retrieval profile.
 *
 * Pure. The wizard asks these functions; no component builds profile fields by hand.
 */

import {
  IMMUTABLE_PROFILE_FIELDS,
  type EmbeddingModalities,
  type FtsRegconfig,
  type RagProfile,
} from '@/constants/rag-profile';
import type { WizardDraft } from '@/lib/onboarding/wizard-state';

export interface ProfileCreateFields {
  rag_profile: RagProfile;
  embedding_modalities: EmbeddingModalities;
  fts_regconfig: FtsRegconfig;
}

/** Fields only the create request carries. */
export function profileCreateFields(
  draft: Pick<WizardDraft, 'ragProfile' | 'embeddingModalities' | 'ftsRegconfig'>,
): ProfileCreateFields {
  return {
    rag_profile: draft.ragProfile,
    // Images need a normal workspace: a graph draft never sends `text,image`.
    embedding_modalities: draft.ragProfile === 'normal' ? draft.embeddingModalities : 'text',
    fts_regconfig: draft.ftsRegconfig,
  };
}

/** Keys that only mean something to a workspace with a knowledge graph. */
export const GRAPH_ONLY_PAYLOAD_KEYS = [
  'extraction_language',
  'entity_types',
  'entity_types_strict',
  'entity_type_colors',
  'relation_types',
  'relation_types_strict',
  'kg_schema_preset',
  'relation_edges',
  'extraction_mode',
  'decision_enabled',
  'decision_model',
  'decision_gate_preset',
  'decision_pack_size',
  'extract_budget_mode',
  'extract_max_entities',
  'extract_max_records',
] as const;

/** A copy of `payload` without the graph-only keys. The server would refuse some of them. */
export function withoutGraphFields<T extends object>(payload: T): Partial<T> {
  const drop = new Set<string>(GRAPH_ONLY_PAYLOAD_KEYS);
  return Object.fromEntries(
    Object.entries(payload).filter(([key]) => !drop.has(key)),
  ) as Partial<T>;
}

/**
 * A copy of `body` without the fields the server fixes at create time.
 * Callers that spread a whole workspace into an update would otherwise get a 409 (EC-180-04).
 */
export function withoutImmutableProfileFields<T extends object>(body: T): T {
  const drop = new Set<string>(IMMUTABLE_PROFILE_FIELDS);
  return Object.fromEntries(
    Object.entries(body).filter(([key]) => !drop.has(key)),
  ) as T;
}
