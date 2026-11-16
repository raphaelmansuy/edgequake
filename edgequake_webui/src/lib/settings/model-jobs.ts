/**
 * How a person chooses a model for a job.
 *
 * A job is the question ("what may read a page?"). A model is an answer only when it can do
 * that job. Chat, vision, and embedding are different tools: a checkbox grid that offers all
 * three to every row asks the question backwards.
 */

import type { UsageKey } from "@/lib/api/edgequake/model-catalog";

/** What the job is for, in one sentence. The catalog, not a vendor list, decides who can do it. */
export const JOB_PURPOSE: Record<UsageKey, string> = {
  extract: "Pulls entities and relations into the graph. It needs a model that follows a schema.",
  query: "Writes the answer people read, from the passages that were retrieved.",
  summary: "Shortens a document. A smaller model is enough.",
  vlm: "Reads a page as an image. Only a model that accepts images can do this.",
  keyword: "Picks a few search terms. A small, fast model is enough.",
  embedding: "Turns text into vectors for search. Only an embedding model can do this.",
  decision: "A yes or no about the text, not the answer people read. A small chat model is enough.",
};

/**
 * A name that follows whatever the local app has loaded, rather than one model.
 * A limit on it changes the next time someone loads a different model.
 */
export function isMovingAlias(text: string): boolean {
  return /currently loaded|loaded model/i.test(text);
}

/** Case-insensitive substring match. An empty query matches everything. */
export function modelMatchesQuery(label: string, query: string): boolean {
  const needle = query.trim().toLowerCase();
  if (!needle) return true;
  return label.toLowerCase().includes(needle);
}

export interface ProviderGroup<T> {
  provider: string;
  rows: T[];
}

/** Groups in the order the providers first appear, so the catalog's order stays put. */
export function groupByProvider<T>(rows: readonly T[], providerOf: (row: T) => string): ProviderGroup<T>[] {
  const order: string[] = [];
  const groups = new Map<string, T[]>();
  for (const row of rows) {
    const provider = providerOf(row);
    const existing = groups.get(provider);
    if (existing) {
      existing.push(row);
      continue;
    }
    order.push(provider);
    groups.set(provider, [row]);
  }
  return order.map((provider) => ({ provider, rows: groups.get(provider) ?? [] }));
}
