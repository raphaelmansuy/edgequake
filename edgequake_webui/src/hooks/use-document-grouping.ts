/**
 * @module useDocumentGrouping
 * @description Groups documents by enrichment_topic for accordion display.
 * Extracted as a separate hook for SRP compliance and independent testability.
 *
 * @implements Document Topic Grouping feature
 */
"use client";

import type { Document } from "@/types";
import { useMemo } from "react";

const NO_TOPIC_KEY = "Tanpa Topic";

/**
 * Groups an array of documents by their `enrichment_topic`.
 * Documents without a topic are placed under "Tanpa Topic", always last.
 * Other groups are sorted alphabetically.
 *
 * Exported as a named function so it can be tested independently of React.
 */
export function groupDocumentsByTopic(documents: Document[]): Map<string, Document[]> {
  const groups = new Map<string, Document[]>();

  for (const doc of documents) {
    const topic =
      doc.enrichment_topic && doc.enrichment_topic.trim() !== ""
        ? doc.enrichment_topic.trim()
        : NO_TOPIC_KEY;

    const existing = groups.get(topic);
    if (existing) {
      existing.push(doc);
    } else {
      groups.set(topic, [doc]);
    }
  }

  // Sort: alphabetical first, "Tanpa Topic" last
  const sorted = new Map<string, Document[]>();
  const keys = Array.from(groups.keys())
    .filter((k) => k !== NO_TOPIC_KEY)
    .sort((a, b) => a.localeCompare(b));

  for (const key of keys) {
    sorted.set(key, groups.get(key)!);
  }
  if (groups.has(NO_TOPIC_KEY)) {
    sorted.set(NO_TOPIC_KEY, groups.get(NO_TOPIC_KEY)!);
  }

  return sorted;
}

/**
 * React hook wrapping groupDocumentsByTopic with memoization.
 * Re-groups only when the documents array reference changes.
 */
export function useDocumentGrouping(documents: Document[]): Map<string, Document[]> {
  return useMemo(() => groupDocumentsByTopic(documents), [documents]);
}
