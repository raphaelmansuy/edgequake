/**
 * Markdown Components Module
 *
 * Exports the streaming markdown renderer and related components.
 */

// Main renderer
export {
  default as StreamingMarkdown,
  StreamingMarkdownRenderer,
} from "./StreamingMarkdownRenderer";

// Block-level components
export { CodeBlock } from "./CodeBlock";
export { DetailsBlock } from "./DetailsBlock";
export { GitHubAlert } from "./GitHubAlert";
export { MarkdownTokens } from "./MarkdownTokens";
export { MermaidBlock } from "./MermaidBlock";
export { TableSkeleton } from "./TableSkeleton";

// Inline components
export { KatexMath } from "./KatexMath";
export { MarkdownInlineTokens } from "./MarkdownInlineTokens";

// Configuration
export { configureMarked } from "./utils/configure-marked";
export type { AlertType } from "./utils/configure-marked";

// Utilities
export {
  isHtmlSafe,
  sanitizeHtml,
  sanitizeMarkdownHtml,
  stripHtml,
} from "./utils/sanitize-html";
export {
  analyzeStreamingContent,
  isCodeBlockComplete,
  isMathBlockComplete,
  isTableComplete,
  type StreamingCompletionStatus,
} from "./utils/streaming-utils";

// Re-export types
export type { Token, Tokens } from "marked";
