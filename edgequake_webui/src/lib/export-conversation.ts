/**
 * @module export-conversation
 * @description Conversation Export Utilities
 *
 * Provides functions to export conversations to various formats:
 * - Markdown (.md)
 * - JSON (.json)
 *
 * @implements FEAT0727 - Export conversation to Markdown
 * @implements FEAT0728 - Export conversation to JSON
 *
 * @enforces BR0724 - Include message metadata in exports
 * @enforces BR0725 - Sanitize filenames for download
 */

import type { ConversationWithMessages, ServerMessage } from "@/types";

// ============================================================================
// Export to Markdown
// ============================================================================

/**
 * Format a single message for markdown export
 */
function formatMessageMarkdown(message: ServerMessage): string {
  const roleLabel = message.role === "user" ? "**You**" : "**Assistant**";
  const timestamp = new Date(message.created_at).toLocaleString();

  let content = `### ${roleLabel}\n\n`;
  content += `_${timestamp}_\n\n`;
  content += message.content;

  // Add context if available
  if (message.context?.sources && message.context.sources.length > 0) {
    content += "\n\n<details>\n<summary>📚 Sources</summary>\n\n";
    message.context.sources.forEach((source, idx) => {
      content += `${idx + 1}. **${
        source.title || "Source"
      }** (score: ${source.score.toFixed(2)})\n`;
      content += `   > ${source.content.slice(0, 200)}${
        source.content.length > 200 ? "..." : ""
      }\n\n`;
    });
    content += "</details>\n";
  }

  // Add thinking section if available
  if (message.context?.thinking) {
    content += "\n\n<details>\n<summary>💭 Thinking Process</summary>\n\n";
    content += message.context.thinking;
    content += "\n\n</details>\n";
  }

  return content;
}

/**
 * Export a conversation to Markdown format
 */
export function exportToMarkdown(
  conversation: ConversationWithMessages
): string {
  const lines: string[] = [];

  // Header
  lines.push(`# ${conversation.title}`);
  lines.push("");
  lines.push(`**Mode**: ${conversation.mode}`);
  lines.push(
    `**Created**: ${new Date(conversation.created_at).toLocaleString()}`
  );
  lines.push(
    `**Last Updated**: ${new Date(conversation.updated_at).toLocaleString()}`
  );
  lines.push(`**Messages**: ${conversation.messages.length}`);
  lines.push("");
  lines.push("---");
  lines.push("");

  // Messages
  conversation.messages.forEach((message) => {
    lines.push(formatMessageMarkdown(message));
    lines.push("");
    lines.push("---");
    lines.push("");
  });

  // Footer
  lines.push("---");
  lines.push("");
  lines.push(`_Exported from EdgeQuake on ${new Date().toLocaleString()}_`);

  return lines.join("\n");
}

// ============================================================================
// Export to JSON
// ============================================================================

/**
 * Export format for JSON that's both human-readable and importable
 */
export interface ConversationExportJSON {
  version: "1.0";
  exported_at: string;
  conversation: {
    id: string;
    title: string;
    mode: string;
    is_pinned: boolean;
    is_archived: boolean;
    created_at: string;
    updated_at: string;
    message_count: number;
  };
  messages: Array<{
    id: string;
    role: "user" | "assistant" | "system";
    content: string;
    mode?: string | null;
    tokens_used?: number | null;
    duration_ms?: number | null;
    thinking_time_ms?: number | null;
    context?: {
      sources?: Array<{
        id: string;
        title?: string;
        content: string;
        score: number;
        source_type?: string;
        document_id?: string;
        file_path?: string;
      }>;
      entities?:
        | Array<{
            name: string;
            entity_type: string;
            description?: string;
            score: number;
            source_document_id?: string;
            source_file_path?: string;
            source_chunk_ids?: string[];
          }>
        | string[];
      relationships?:
        | Array<{
            source: string;
            target: string;
            relation_type: string;
            description?: string;
            score: number;
            source_document_id?: string;
            source_file_path?: string;
          }>
        | string[];
      thinking?: string;
    } | null;
    created_at: string;
  }>;
}

/**
 * Export a conversation to JSON format
 */
export function exportToJSON(
  conversation: ConversationWithMessages
): ConversationExportJSON {
  return {
    version: "1.0",
    exported_at: new Date().toISOString(),
    conversation: {
      id: conversation.id,
      title: conversation.title,
      mode: conversation.mode,
      is_pinned: conversation.is_pinned,
      is_archived: conversation.is_archived,
      created_at: conversation.created_at,
      updated_at: conversation.updated_at,
      message_count: conversation.messages.length,
    },
    messages: conversation.messages.map((msg) => ({
      id: msg.id,
      role: msg.role,
      content: msg.content,
      mode: msg.mode,
      tokens_used: msg.tokens_used,
      duration_ms: msg.duration_ms,
      thinking_time_ms: msg.thinking_time_ms,
      context: msg.context
        ? {
            sources: msg.context.sources,
            entities: msg.context.entities,
            relationships: msg.context.relationships,
            thinking: msg.context.thinking,
          }
        : null,
      created_at: msg.created_at,
    })),
  };
}

// ============================================================================
// Download Utilities
// ============================================================================

/**
 * Trigger a file download in the browser
 */
export function downloadFile(
  content: string,
  filename: string,
  mimeType: string
): void {
  const blob = new Blob([content], { type: mimeType });
  const url = URL.createObjectURL(blob);

  const link = document.createElement("a");
  link.href = url;
  link.download = filename;
  link.style.display = "none";

  document.body.appendChild(link);
  link.click();

  // Cleanup
  setTimeout(() => {
    URL.revokeObjectURL(url);
    document.body.removeChild(link);
  }, 100);
}

/**
 * Generate a safe filename from a title
 */
export function sanitizeFilename(title: string): string {
  return title
    .replace(/[^a-z0-9]/gi, "-")
    .replace(/-+/g, "-")
    .replace(/^-|-$/g, "")
    .toLowerCase()
    .slice(0, 50);
}

/**
 * Export and download a conversation as Markdown
 */
export function downloadAsMarkdown(
  conversation: ConversationWithMessages
): void {
  const markdown = exportToMarkdown(conversation);
  const filename = `${sanitizeFilename(conversation.title)}.md`;
  downloadFile(markdown, filename, "text/markdown;charset=utf-8");
}

/**
 * Export and download a conversation as JSON
 */
export function downloadAsJSON(conversation: ConversationWithMessages): void {
  const json = exportToJSON(conversation);
  const content = JSON.stringify(json, null, 2);
  const filename = `${sanitizeFilename(conversation.title)}.json`;
  downloadFile(content, filename, "application/json;charset=utf-8");
}
