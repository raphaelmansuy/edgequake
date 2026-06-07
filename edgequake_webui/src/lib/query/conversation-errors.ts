import { ApiRequestError } from "@/lib/api/client";

/** Detect stale or missing conversation errors (404 / not found). */
export function isConversationNotFoundError(error: unknown): boolean {
  if (error instanceof ApiRequestError && error.status === 404) {
    return true;
  }
  if (error instanceof Error) {
    const message = error.message.toLowerCase();
    return message.includes("not found") && message.includes("conversation");
  }
  return false;
}

/** Optimistic/local messages are never persisted server-side. */
export function isServerPersistedMessageId(id: string): boolean {
  return !id.startsWith("optimistic-");
}
