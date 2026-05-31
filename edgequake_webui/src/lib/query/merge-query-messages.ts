import type { QueryMessage } from "./query-interface-types";

/**
 * Merge server messages with optimistic user message and pending assistant stream.
 * Deduplicates during streaming → server handoff window.
 */
export function mergeQueryMessages(
  serverMessages: QueryMessage[],
  optimisticUserMessage: QueryMessage | null,
  pendingMessage: QueryMessage | null,
): QueryMessage[] {
  const result = [...serverMessages];

  if (optimisticUserMessage) {
    const alreadyFromServer = serverMessages.some(
      (message) =>
        message.role === "user" &&
        message.content === optimisticUserMessage.content,
    );
    if (!alreadyFromServer) {
      result.push(optimisticUserMessage);
    }
  }

  if (pendingMessage?.content) {
    const lastServerMsg = serverMessages[serverMessages.length - 1];
    const alreadyFromServer =
      lastServerMsg?.role === "assistant" &&
      lastServerMsg.content === pendingMessage.content;
    if (!alreadyFromServer) {
      result.push(pendingMessage);
    }
  }

  return result;
}
