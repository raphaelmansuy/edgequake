import type { QueryContext } from '@/types';

export type ConfidenceStyle = {
  labelKey: string;
  defaultLabel: string;
  color: string;
  bgColor: string;
};

/**
 * How strongly the context supports the answer, as a 0..1 band input.
 *
 * WHY this now reports rank evidence rather than an average of `score` values: on a normal
 * workspace a chunk `score` is an RRF fusion score whose reachable range is 0.01538–0.01639
 * (see `api/RESULTS.md`). Averaging numbers in that range always produced ~0.016, which fell
 * in the lowest band and labelled a perfect match "Mentioned (1%)". A percentage derived
 * from a rank score is not a confidence.
 *
 * The score here therefore reflects how much evidence came back, which is what the band
 * labels ("Strong" … "Mentioned") actually describe:
 *   - several distinct sources ranked → stronger
 *   - a single source → weak
 *   - graph entities on top → a small bonus
 *
 * Returns `null` when there is nothing to judge, so the caller can omit the badge entirely
 * instead of asserting a confidence it does not have.
 */
export const calculateConfidence = (context: QueryContext): number | null => {
  const chunkScores = context.chunks?.map((c) => c.score).filter((s) => s > 0) || [];
  const entityCount = context.entities?.length || 0;
  const relCount = context.relationships?.length || 0;

  if (chunkScores.length === 0 && entityCount === 0 && relCount === 0) {
    return null;
  }

  // Evidence breadth: 1 source is weak, 5+ distinct sources is strong. Capped so a large
  // context cannot claim certainty on volume alone.
  const breadth = Math.min(1, chunkScores.length / 5);
  const graphBonus = Math.min(0.15, (entityCount + relCount) * 0.02);
  return Math.min(1, breadth * 0.85 + graphBonus);
};

/**
 * Phrases a model emits when it is declining to answer from the context it was given.
 *
 * Matched against the answer, not the sources. Deliberately narrow and case-insensitive: this
 * decides whether a "Strong" badge appears next to words like "there is insufficient evidence",
 * which is the one pairing a reader will not forgive.
 */
const REFUSAL_PATTERNS: RegExp[] = [
  /not answerable/i,
  /insufficient (?:evidence|information|context|data)/i,
  // Active and passive: models write both "I cannot answer" and "this cannot be determined".
  /(?:cannot|can't|can not|unable to) (?:(?:be )?(?:answer|determine|find|locate|established|confirmed))/i,
  /no (?:relevant |usable |readable )?(?:textual )?(?:information|evidence|content|data|passages?)/i,
  /(?:context|documents?|sources?) (?:consists?|contains?|is) (?:solely|only|entirely)[^.]{0,60}(?:image|placeholder)/i,
  /(?:does not|do not) (?:contain|include|provide)[^.]{0,40}(?:information|evidence|text)/i,
  /i (?:do not|don't) (?:have|see|know)[^.]{0,40}(?:information|evidence|enough)/i,
  // "...the handbook states regarding refunds cannot be determined" — subject before the modal.
  /(?:cannot|can't|can not) be (?:determined|answered|established|confirmed|found)/i,
];

/** True when the answer declines to answer, whatever the retrieval returned. */
export function isRefusalAnswer(answer: string | null | undefined): boolean {
  const text = (answer ?? '').trim();
  if (!text) return false;
  return REFUSAL_PATTERNS.some((pattern) => pattern.test(text));
}

/**
 * The confidence to display, after the answer has been read.
 *
 * A refusal means the evidence did not support an answer, so reporting breadth as confidence
 * says the opposite of what just happened: the screenshot that prompted this showed
 * **"20 Sources · Strong"** directly above *"insufficient textual information"*. Retrieved
 * volume is not support, and a metric that contradicts the sentence beside it is worse than no
 * metric.
 *
 * A refusal therefore returns `null` — "do not assert a confidence I do not have" — which the
 * caller already renders as no badge.
 */
export function calculateAnswerConfidence(
  context: QueryContext,
  answer: string | null | undefined,
): number | null {
  if (isRefusalAnswer(answer)) return null;
  return calculateConfidence(context);
}

export const getConfidenceLabel = (score: number): ConfidenceStyle => {
  if (score >= 0.5) {
    return {
      labelKey: 'query.citations.confidence.strong',
      defaultLabel: 'Strong',
      color: 'text-blue-600 dark:text-blue-400',
      bgColor: 'bg-blue-500',
    };
  }
  if (score >= 0.3) {
    return {
      labelKey: 'query.citations.confidence.good',
      defaultLabel: 'Good',
      color: 'text-sky-600 dark:text-sky-400',
      bgColor: 'bg-sky-500',
    };
  }
  if (score >= 0.2) {
    return {
      labelKey: 'query.citations.confidence.related',
      defaultLabel: 'Related',
      color: 'text-slate-600 dark:text-slate-400',
      bgColor: 'bg-slate-500',
    };
  }
  return {
    labelKey: 'query.citations.confidence.mentioned',
    defaultLabel: 'Mentioned',
    color: 'text-slate-500 dark:text-slate-400',
    bgColor: 'bg-slate-400',
  };
};
