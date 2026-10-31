/**
 * SPEC-180 — what a workspace's retrieval type allows in the composer (pure, unit-tested).
 *
 * The user's saved mode is never rewritten. A normal workspace shows the mode it can run,
 * and the saved Smart choice comes back when the user returns to a graph workspace.
 */
import type { RagProfileCapabilities } from '@/constants/rag-profile';
import type { QuerySettings } from '@/types/settings';
import type { QueryMode } from '@/types/query';
import { QUERY_MODE_META, type QueryModeMeta } from './query-mode-meta';

type ModeView = Pick<RagProfileCapabilities, 'allows_graph_modes'>;

/** Modes a workspace without a graph can run: chunks and plain chat. */
const NORMAL_MODES: readonly QueryMode[] = ['naive', 'bypass'];

/** The default mode when the user has not chosen one this workspace can run. */
export const NORMAL_DEFAULT_MODE: QueryMode = 'naive';

export function modesForProfile(caps: ModeView): readonly QueryModeMeta[] {
  if (caps.allows_graph_modes) return QUERY_MODE_META;
  return QUERY_MODE_META.filter((m) => NORMAL_MODES.includes(m.id));
}

export function effectiveMode(mode: QueryMode, caps: ModeView): QueryMode {
  if (caps.allows_graph_modes || NORMAL_MODES.includes(mode)) return mode;
  return NORMAL_DEFAULT_MODE;
}

/** Settings as the active workspace can use them: a runnable mode, and its kind chips. */
export function applyProfile(
  settings: QuerySettings,
  caps: ModeView & Pick<RagProfileCapabilities, 'extracts_graph'>,
  docKinds: readonly string[],
): QuerySettings {
  const mode = effectiveMode(settings.mode, caps);
  const kinds = caps.extracts_graph ? [] : [...docKinds];
  if (mode === settings.mode && kinds.length === 0 && !settings.docKinds?.length) return settings;
  return { ...settings, mode, docKinds: kinds.length > 0 ? kinds : undefined };
}
