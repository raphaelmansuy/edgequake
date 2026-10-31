/**
 * Empty-state copy for Query UI: Chat (bypass), graph RAG and, with no graph, Normal RAG (SPEC-180).
 * Pure SSOT so UI and tests share the same strings (DRY).
 */

import type { QueryMode } from "@/types/query";

export interface QueryEmptyCopy {
  title: string;
  description: string;
  suggestions: string[];
  /** i18n keys, so the component never branches on which copy it got. */
  keys: { title: string; description: string; suggestions: string; subtitle: string };
  /** Subtitle under the page title. */
  subtitle: string;
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
  keys: {
    title: "query.emptyTitle",
    description: "query.emptyDescription",
    suggestions: "query.suggestions",
    subtitle: "query.subtitle",
  },
  subtitle: "Ask questions about your knowledge graph",
};

const NORMAL_COPY: QueryEmptyCopy = {
  title: "Ask about your documents",
  description:
    "I find the passages that answer your question, quote them and point to the page.",
  suggestions: [
    "What does the handbook say about refunds?",
    "Find the passage that mentions deadlines",
    "Which documents cover onboarding?",
    "Quote the section about security requirements",
  ],
  keys: {
    title: "query.normalEmptyTitle",
    description: "query.normalEmptyDescription",
    suggestions: "query.normalSuggestions",
    subtitle: "query.normalSubtitle",
  },
  subtitle: "Ask questions about your documents",
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
  keys: {
    title: "query.chatEmptyTitle",
    description: "query.chatEmptyDescription",
    suggestions: "query.chatSuggestions",
    subtitle: "query.chatSubtitle",
  },
  subtitle: "General chat — no knowledge graph retrieval",
};

/** `hasGraph` is the workspace's `shows_graph_ui` capability. */
export function getQueryEmptyCopy(
  mode: QueryMode = "mix",
  hasGraph = true,
): QueryEmptyCopy {
  if (mode === "bypass") return CHAT_COPY;
  return hasGraph ? RAG_COPY : NORMAL_COPY;
}

export function isChatQueryMode(mode: QueryMode): boolean {
  return mode === "bypass";
}
