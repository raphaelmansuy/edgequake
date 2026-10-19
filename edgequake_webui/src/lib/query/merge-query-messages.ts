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

  if (pendingMessage) {
    // An empty bubble is kept while streaming so the "waiting for the first
    // token" state has a place to render (no dead gap after the question).
    const includeEmpty = Boolean(pendingMessage.stopped || pendingMessage.isStreaming);
    if (pendingMessage.content || includeEmpty) {
      const lastServerMsg = serverMessages[serverMessages.length - 1];
      const alreadyFromServer =
        lastServerMsg?.role === "assistant" &&
        pendingMessage.content &&
        lastServerMsg.content === pendingMessage.content;
      if (!alreadyFromServer) {
        result.push(pendingMessage);
      }
    }
  }

  return result;
}

/**
 * True when the refetched conversation already contains the streamed answer,
 * so the optimistic pending bubble can be dropped without blanking the thread.
 */
export function conversationEchoesPending(
  messages: { role?: string; content?: string }[] | undefined,
  pendingContent: string | undefined,
): boolean {
  // An empty local bubble is not proof the server saved an answer.
  // Drop it only after an assistant message is actually on the thread.
  if (!pendingContent) {
    return (messages ?? []).some(
      (message) => message.role === "assistant" && Boolean(message.content),
    );
  }
  return (messages ?? []).some(
    (message) =>
      message.role === "assistant" && message.content === pendingContent,
  );
}
