/**
 * Shared error formatting for MCP tool responses.
 */

export function formatError(error: unknown): {
  content: Array<{ type: "text"; text: string }>;
  isError: true;
} {
  const message =
    error instanceof Error ? error.message : String(error);
  return {
    content: [{ type: "text" as const, text: `Error: ${message}` }],
    isError: true,
  };
}
