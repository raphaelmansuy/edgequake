/**
 * Streaming Utilities for Markdown Rendering
 *
 * Provides utilities for handling incomplete markdown structures during streaming.
 * Implements buffering logic for tables, code blocks, and other multi-line structures.
 */

/**
 * Check if content ends with an HR pattern that might be part of something else.
 * HR patterns (---, ***, ___) at the very end of streaming content are suspect
 * because they could be:
 * - Part of a table separator row (|---|---|)
 * - YAML frontmatter delimiter
 * - About to have more dashes/asterisks added
 *
 * We only flag as incomplete if it's at the very end of content with nothing after.
 */
export function hasIncompleteHR(content: string): boolean {
  // Check if content ends with potential HR patterns
  const trimmedContent = content.trimEnd();
  const lines = trimmedContent.split("\n");
  const lastLine = lines[lines.length - 1]?.trim() || "";

  // HR patterns: ---, ***, ___ (3+ of the same character)
  const hrPattern = /^[-*_]{3,}$/;

  // If last line is an HR pattern, check context
  if (hrPattern.test(lastLine)) {
    // If this is the very first line, it might be incomplete frontmatter
    if (lines.length === 1) {
      return true;
    }

    // If previous line also looks like it could be part of a structure, wait
    const prevLine = lines[lines.length - 2]?.trim() || "";

    // Could be a table separator if previous line has pipes
    if (prevLine.includes("|")) {
      return true;
    }

    // Could be YAML frontmatter if previous lines look like yaml
    // (starts with ---, has key: value pairs)
    if (lines.length >= 2 && lines[0].trim() === "---") {
      // Check if we're inside yaml frontmatter (not closed yet)
      const dashCount = lines.filter((l) => l.trim() === "---").length;
      if (dashCount % 2 !== 0) {
        return true;
      }
    }
  }

  // Check for incomplete HR being formed (1 or 2 characters)
  if (/^[-*_]{1,2}$/.test(lastLine) && lines.length > 1) {
    // Could be in the process of forming an HR
    return true;
  }

  return false;
}

/**
 * Extract content before an incomplete HR pattern
 */
export function extractContentBeforeIncompleteHR(content: string): {
  safeContent: string;
  pendingHR: string;
} {
  const trimmedContent = content.trimEnd();
  const lines = trimmedContent.split("\n");

  // Remove the last line if it's an HR pattern
  if (lines.length > 1) {
    const lastLine = lines[lines.length - 1]?.trim() || "";
    if (/^[-*_]{1,}$/.test(lastLine)) {
      const safeLines = lines.slice(0, -1);
      return {
        safeContent: safeLines.join("\n"),
        pendingHR: lines[lines.length - 1],
      };
    }
  }

  return { safeContent: content, pendingHR: "" };
}

/**
 * Check if a markdown table structure is complete.
 * A complete table has:
 * - At least a header row
 * - A separator row (|---|...)
 * - Optionally data rows
 * - No trailing pipe without closing
 */
export function isTableComplete(content: string): boolean {
  const lines = content.split("\n");
  const tableLines = lines.filter((line) => line.trim().startsWith("|"));

  // Need at least header and separator
  if (tableLines.length < 2) return false;

  // Check if there's a separator row (|---|...|)
  const hasSeparator = tableLines.some((line) => /^\s*\|[\s\-:]+\|/.test(line));
  if (!hasSeparator) return false;

  // Check if last table line is complete (ends with |)
  const lastTableLine = tableLines[tableLines.length - 1];
  if (!lastTableLine.trim().endsWith("|")) return false;

  // Check for balanced pipes in last row
  const pipeCount = (lastTableLine.match(/\|/g) || []).length;
  const headerPipeCount = (tableLines[0].match(/\|/g) || []).length;

  return pipeCount === headerPipeCount;
}

/**
 * Check if a code block is complete (has closing ```)
 */
export function isCodeBlockComplete(content: string): boolean {
  const codeBlockPattern = /```[\s\S]*?```/g;
  const openPattern = /```[^\n]*\n?/g;

  const opens = (content.match(openPattern) || []).length;
  const closes = (content.match(/```(?:\n|$)/g) || []).length;

  // Also check for ``` at end without newline
  const endsWithTripleBacktick = content.trimEnd().endsWith("```");

  return opens === closes || (opens > 0 && endsWithTripleBacktick);
}

/**
 * Check if a math block is complete (has closing $$)
 */
export function isMathBlockComplete(content: string): boolean {
  const mathMatches = content.match(/\$\$/g) || [];
  return mathMatches.length % 2 === 0;
}

/**
 * Detect if content has an incomplete table at the end
 */
export function hasIncompleteTable(content: string): boolean {
  // Look for table pattern at the end of content
  const lines = content.split("\n");
  let foundTableStart = false;
  const tableLines: string[] = [];

  // Scan from end backwards to find table
  for (let i = lines.length - 1; i >= 0; i--) {
    const line = lines[i].trim();

    if (line.startsWith("|")) {
      tableLines.unshift(lines[i]);
      foundTableStart = true;
    } else if (foundTableStart && line === "") {
      // Empty line before table content
      continue;
    } else if (foundTableStart) {
      // Non-table line found, stop
      break;
    }
  }

  if (!foundTableStart || tableLines.length === 0) return false;

  // Reconstruct table content and check completeness
  const tableContent = tableLines.join("\n");
  return !isTableComplete(tableContent);
}

/**
 * Extract content before an incomplete table
 */
export function extractContentBeforeIncompleteTable(content: string): {
  safeContent: string;
  pendingTable: string;
} {
  const lines = content.split("\n");
  let tableStartIndex = -1;

  // Find where the table starts (scan backwards)
  for (let i = lines.length - 1; i >= 0; i--) {
    const line = lines[i].trim();

    if (line.startsWith("|")) {
      tableStartIndex = i;
    } else if (tableStartIndex !== -1 && line !== "") {
      // Found non-table content, table starts after this
      break;
    }
  }

  if (tableStartIndex === -1) {
    return { safeContent: content, pendingTable: "" };
  }

  // Find actual start (skip empty lines before table)
  while (tableStartIndex > 0 && lines[tableStartIndex - 1].trim() === "") {
    tableStartIndex--;
  }

  const safeContent = lines.slice(0, tableStartIndex).join("\n");
  const pendingTable = lines.slice(tableStartIndex).join("\n");

  return { safeContent, pendingTable };
}

/**
 * Detect if content ends with incomplete code block
 */
export function hasIncompleteCodeBlock(content: string): boolean {
  // Count opening and closing ```
  const lines = content.split("\n");
  let inCodeBlock = false;

  for (const line of lines) {
    if (line.trim().startsWith("```")) {
      inCodeBlock = !inCodeBlock;
    }
  }

  return inCodeBlock;
}

/**
 * Token completion status for streaming
 */
export interface StreamingCompletionStatus {
  isComplete: boolean;
  incompleteType?: "table" | "code" | "math" | "hr" | "none";
  safeToRenderContent?: string;
  pendingContent?: string;
}

/**
 * Analyze streaming content for completeness
 */
export function analyzeStreamingContent(
  content: string
): StreamingCompletionStatus {
  // Check for incomplete code blocks first (most common)
  if (hasIncompleteCodeBlock(content)) {
    const lastCodeBlockStart = content.lastIndexOf("```");
    return {
      isComplete: false,
      incompleteType: "code",
      safeToRenderContent: content.slice(0, lastCodeBlockStart),
      pendingContent: content.slice(lastCodeBlockStart),
    };
  }

  // Check for incomplete math blocks
  if (!isMathBlockComplete(content)) {
    const lastMathStart = content.lastIndexOf("$$");
    return {
      isComplete: false,
      incompleteType: "math",
      safeToRenderContent: content.slice(0, lastMathStart),
      pendingContent: content.slice(lastMathStart),
    };
  }

  // Check for incomplete tables
  if (hasIncompleteTable(content)) {
    const { safeContent, pendingTable } =
      extractContentBeforeIncompleteTable(content);
    return {
      isComplete: false,
      incompleteType: "table",
      safeToRenderContent: safeContent,
      pendingContent: pendingTable,
    };
  }

  // Check for potentially incomplete HR patterns at the end
  if (hasIncompleteHR(content)) {
    const { safeContent, pendingHR } =
      extractContentBeforeIncompleteHR(content);
    return {
      isComplete: false,
      incompleteType: "hr",
      safeToRenderContent: safeContent,
      pendingContent: pendingHR,
    };
  }

  return {
    isComplete: true,
    incompleteType: "none",
    safeToRenderContent: content,
    pendingContent: "",
  };
}
