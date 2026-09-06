/**
 * Empty-state copy for Query UI — Chat (bypass) vs RAG modes.
 * Pure SSOT so UI and tests share the same strings (DRY).
 *
 * SPEC-146: zero-authz post-query answer must be ZERO_AUTHZ_ANSWER (LAW existence-hiding).
 */

import type { QueryMode } from "@/types/query";

export interface QueryEmptyCopy {
  title: string;
  description: string;
  suggestions: string[];
}

/** Post-query empty answer when allow-set ∩ retrieval is empty (G-146-52). */
export const ZERO_AUTHZ_ANSWER = "No matching results.";

/** Refine help under zero-authz / true-empty answers (004-ux normative). */
export const ZERO_AUTHZ_HELP = "Try different terms or filters.";

/** Existence-hiding placeholder — never render in citations (G-146-42). */
export function isRestrictedCitationLabel(label?: string | null): boolean {
  if (!label) return false;
  return label.trim().toLowerCase() === "restricted";
}

const RAG_COPY: QueryEmptyCopy = {
  title: "Ask about your knowledge graph",
  description:
    "I can help you explore entities, find connections, and uncover insights from your documents.",
  suggestions: [
    "What are the main entities in my knowledge graph?",
    "Summarize the key relationships between documents",
    "Find connections between people and organizations",
    "What topics are covered in my documents?",
  ],
};

const CHAT_COPY: QueryEmptyCopy = {
  title: "Chat with your assistant",
  description:
    "General conversation without document or graph retrieval. Follow-ups use recent chat history.",
  suggestions: [
    "Help me brainstorm ideas",
    "Explain a concept in simple terms",
    "What should I consider before starting a project?",
    "Summarize the trade-offs of two approaches",
  ],
};

export function getQueryEmptyCopy(mode: QueryMode = "mix"): QueryEmptyCopy {
  return mode === "bypass" ? CHAT_COPY : RAG_COPY;
}

export function isChatQueryMode(mode: QueryMode): boolean {
  return mode === "bypass";
}

/** True when a completed RAG answer should show the zero-authz SSOT string. */
export function isZeroAuthzAnswer(
  answer: string | undefined | null,
  sourceCount: number,
): boolean {
  if (sourceCount > 0) return false;
  const trimmed = (answer ?? "").trim();
  return trimmed.length === 0 || trimmed === ZERO_AUTHZ_ANSWER;
}

/** Normalize empty RAG answers to ZERO_AUTHZ_ANSWER for display. */
export function displayQueryAnswer(
  answer: string | undefined | null,
  sourceCount: number,
  mode: QueryMode = "mix",
): string {
  if (mode === "bypass") return answer ?? "";
  if (isZeroAuthzAnswer(answer, sourceCount)) return ZERO_AUTHZ_ANSWER;
  return answer ?? "";
}
