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
 * Check if a table has enough structure to be rendered progressively.
 * Returns true if the table has at least header + separator row,
 * meaning we can render complete rows even if the last row is still being typed.
 */
export function isTableRenderable(content: string): boolean {
  const lines = content.split("\n");
  const tableLines = lines.filter((line) => line.trim().startsWith("|"));

  if (tableLines.length < 2) return false;

  // Must have a separator row (|---|...|)
  return tableLines.some((line) => /^\s*\|[\s\-:]+\|/.test(line));
}

/**
 * Trim incomplete last row from a table at the end of streaming content.
 * Returns the full content with only complete table rows preserved.
 *
 * WHY: During streaming, the last row of a table may be partially typed
 * (e.g. "| ALICE | PER" without a closing "|"). Instead of hiding the
 * entire table behind a skeleton, we drop only the partial trailing row
 * so that all previously complete rows render progressively.
 *
 * If the content has no table at the end, or the table's last row is
 * already complete, the content is returned unchanged.
 */
export function trimIncompleteTableRow(content: string): string {
  const lines = content.split("\n");

  // Find the trailing table block (scan backwards)
  let tableEndIdx = -1;
  let tableStartIdx = -1;

  for (let i = lines.length - 1; i >= 0; i--) {
    const trimmed = lines[i].trim();
    if (trimmed.startsWith("|")) {
      if (tableEndIdx === -1) tableEndIdx = i;
      tableStartIdx = i;
    } else if (tableEndIdx !== -1 && trimmed === "") {
      // blank line inside/above table area — keep scanning
      continue;
    } else if (tableEndIdx !== -1) {
      // Non-table content above — stop
      break;
    }
  }

  // No table found at end of content
  if (tableEndIdx === -1 || tableStartIdx === -1) return content;

  // Collect only actual table lines (skip blanks in the middle)
  const tableLines: string[] = [];
  for (let i = tableStartIdx; i <= tableEndIdx; i++) {
    if (lines[i].trim().startsWith("|")) {
      tableLines.push(lines[i]);
    }
  }

  // Need at least header + separator to have a renderable table
  if (tableLines.length < 2) return content;

  // Check if there's a separator
  const hasSeparator = tableLines.some((l) => /^\s*\|[\s\-:]+\|/.test(l));
  if (!hasSeparator) return content;

  // Check the last table line
  const lastTableLine = tableLines[tableLines.length - 1];
  const headerPipeCount = (tableLines[0].match(/\|/g) || []).length;
  const lastPipeCount = (lastTableLine.match(/\|/g) || []).length;
  const lastLineComplete =
    lastTableLine.trim().endsWith("|") && lastPipeCount === headerPipeCount;

  // If last line is the separator row itself, table is still forming
  if (
    /^\s*\|[\s\-:]+\|$/.test(lastTableLine.trim()) &&
    tableLines.length === 2
  ) {
    return content; // header + separator only, nothing to trim
  }

  if (lastLineComplete) return content; // all complete, nothing to trim

  // Drop the incomplete last row by removing it from the original lines
  // Find the actual line index of the last table line in the original array
  let dropIdx = -1;
  for (let i = tableEndIdx; i >= tableStartIdx; i--) {
    if (lines[i].trim().startsWith("|")) {
      dropIdx = i;
      break;
    }
  }

  if (dropIdx === -1) return content;

  const trimmedLines = [
    ...lines.slice(0, dropIdx),
    ...lines.slice(dropIdx + 1),
  ];
  return trimmedLines.join("\n");
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
 * Detect if content has an incomplete table at the end.
 *
 * KEY BEHAVIOR CHANGE: A table with header + separator but an incomplete
 * last data row is no longer considered "incomplete". Instead, we trim
 * the partial row and render the table progressively via
 * `trimIncompleteTableRow`. This prevents the entire table from flickering
 * behind a skeleton every time a new row starts streaming.
 *
 * Only returns true when the table is truly un-renderable:
 * - Has pipe-starting lines but no separator row yet (header still forming)
 * - Last line is the separator itself (no data rows yet, more coming)
 */
export function hasIncompleteTable(content: string): boolean {
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
      continue;
    } else if (foundTableStart) {
      break;
    }
  }

  if (!foundTableStart || tableLines.length === 0) return false;

  // If we have a renderable table (header + separator), we handle partial
  // rows via trimIncompleteTableRow instead of buffering  the whole table.
  const tableContent = tableLines.join("\n");
  if (isTableRenderable(tableContent)) {
    // Table is renderable — NOT "incomplete" in the skeleton sense.
    // The streaming renderer will just trim the partial last row.
    return false;
  }

  // No separator yet → truly incomplete, show skeleton
  return true;
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
  content: string,
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
