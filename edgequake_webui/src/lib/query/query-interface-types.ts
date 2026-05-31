import type { QueryContext, QueryMode } from "@/types";

export type StreamingState =
  | "idle"
  | "thinking"
  | "generating"
  | "complete"
  | "error";

/** Local message shape compatible with ChatMessage. */
export interface QueryMessage {
  id: string;
  role: "user" | "assistant";
  content: string;
  mode?: QueryMode;
  tokensUsed?: number;
  durationMs?: number;
  thinkingTimeMs?: number;
  context?: QueryContext;
  isError?: boolean;
  isStreaming?: boolean;
  timestamp?: number;
  llmProvider?: string;
  llmModel?: string;
}

export interface AttachedImage {
  data: string;
  mime_type: string;
  preview: string;
}

export const QUERY_MAX_IMAGES = 4;
export const QUERY_ACCEPTED_IMAGE_MIME = [
  "image/jpeg",
  "image/png",
  "image/gif",
  "image/webp",
] as const;
