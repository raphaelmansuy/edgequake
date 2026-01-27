/**
 * @module error-categories
 * @description Utility to categorize and parse error messages for user-friendly display
 *
 * @implements OODA-09 - Error categorization and actionable suggestions
 * @implements BR0302 - Failed documents show clear error information
 *
 * Categorizes errors into:
 * - LLM: Rate limits, API unavailable, token limits
 * - Embedding: Dimension mismatch, embedding API errors
 * - Storage: Database errors, connection issues
 * - Pipeline: Parse failures, chunk issues, content validation
 * - Network: Timeout, connection refused
 * - Unknown: Uncategorized errors
 */

export type ErrorCategory =
  | "llm"
  | "embedding"
  | "storage"
  | "pipeline"
  | "network"
  | "unknown";

export interface CategorizedError {
  /** The error category */
  category: ErrorCategory;
  /** User-friendly category label */
  categoryLabel: string;
  /** User-friendly summary of the error */
  summary: string;
  /** Whether the error is likely transient (retryable) */
  isTransient: boolean;
  /** Suggested action for the user */
  suggestion: string;
  /** Original error message for technical details */
  originalMessage: string;
}

interface ErrorPattern {
  category: ErrorCategory;
  patterns: RegExp[];
  isTransient: boolean;
  suggestion: string;
}

const ERROR_PATTERNS: ErrorPattern[] = [
  // LLM Rate Limit Errors
  {
    category: "llm",
    patterns: [
      /rate.?limit/i,
      /too.?many.?requests/i,
      /quota.?exceeded/i,
      /429/i,
      /TPM.*limit/i,
      /RPM.*limit/i,
    ],
    isTransient: true,
    suggestion: "Wait a few minutes and try again, or reduce batch size.",
  },
  // LLM API Errors
  {
    category: "llm",
    patterns: [
      /api.?key/i,
      /authentication/i,
      /unauthorized/i,
      /invalid.*token/i,
      /openai/i,
      /ollama/i,
      /llm.*error/i,
    ],
    isTransient: false,
    suggestion: "Check your API key configuration and LLM provider status.",
  },
  // LLM Context Length Errors
  {
    category: "llm",
    patterns: [
      /context.*length/i,
      /too.*long/i,
      /max.*tokens/i,
      /token.*limit/i,
      /maximum.*context/i,
    ],
    isTransient: false,
    suggestion:
      "The document may be too large. Try splitting it into smaller parts.",
  },
  // Embedding Errors
  {
    category: "embedding",
    patterns: [
      /embedding/i,
      /dimension.*mismatch/i,
      /vector.*dimension/i,
      /encode.*error/i,
      /failed.*to.*encode/i,
      /embed.*error/i,
    ],
    isTransient: false,
    suggestion:
      "Check embedding model configuration. Dimensions must match storage.",
  },
  // Storage/Database Errors
  {
    category: "storage",
    patterns: [
      /database/i,
      /postgres/i,
      /connection.*refused/i,
      /constraint.*violation/i,
      /unique.*constraint/i,
      /storage.*error/i,
      /deadlock/i,
      /transaction/i,
    ],
    isTransient: true,
    suggestion: "Database may be temporarily unavailable. Try again shortly.",
  },
  // Pipeline/Parsing Errors
  {
    category: "pipeline",
    patterns: [
      /parse.*error/i,
      /invalid.*format/i,
      /chunk.*error/i,
      /extract.*failed/i,
      /failed.*extract/i,
      /malformed/i,
      /invalid.*content/i,
      /empty.*content/i,
      /no.*text/i,
      /pdf.*error/i,
      /corrupt/i,
    ],
    isTransient: false,
    suggestion:
      "The document format may not be supported. Check the file and try again.",
  },
  // Network Errors
  {
    category: "network",
    patterns: [
      /timeout/i,
      /timed.?out/i,
      /network.*error/i,
      /connection.*reset/i,
      /ECONNREFUSED/i,
      /ETIMEDOUT/i,
      /failed.*to.*fetch/i,
      /unreachable/i,
    ],
    isTransient: true,
    suggestion: "Network connection issue. Check connectivity and try again.",
  },
];

const CATEGORY_LABELS: Record<ErrorCategory, string> = {
  llm: "LLM Provider",
  embedding: "Embedding",
  storage: "Database",
  pipeline: "Processing",
  network: "Network",
  unknown: "Unknown",
};

/**
 * Categorize an error message for user-friendly display.
 *
 * @param message - The error message to categorize
 * @returns Categorized error with summary and suggestions
 *
 * @example
 * ```ts
 * const error = categorizeError("API rate limit exceeded");
 * // { category: 'llm', isTransient: true, ... }
 * ```
 */
export function categorizeError(message: string): CategorizedError {
  // Find matching pattern
  for (const pattern of ERROR_PATTERNS) {
    for (const regex of pattern.patterns) {
      if (regex.test(message)) {
        return {
          category: pattern.category,
          categoryLabel: CATEGORY_LABELS[pattern.category],
          summary: extractSummary(message),
          isTransient: pattern.isTransient,
          suggestion: pattern.suggestion,
          originalMessage: message,
        };
      }
    }
  }

  // Default to unknown
  return {
    category: "unknown",
    categoryLabel: CATEGORY_LABELS.unknown,
    summary: extractSummary(message),
    isTransient: false,
    suggestion: "Check the error details and try again.",
    originalMessage: message,
  };
}

/**
 * Extract a brief summary from an error message.
 * Removes stack traces and technical jargon for user display.
 */
function extractSummary(message: string): string {
  // Take first line or first 100 chars
  const firstLine = message.split("\n")[0];
  const cleaned = firstLine
    .replace(/^(Error|Exception|Panic):\s*/i, "")
    .replace(/at\s+\S+:\d+:\d+/g, "")
    .trim();

  if (cleaned.length > 100) {
    return cleaned.slice(0, 97) + "...";
  }

  return cleaned || "An error occurred";
}

/**
 * Get icon name for error category (lucide-react icon names)
 */
export function getCategoryIcon(category: ErrorCategory): string {
  switch (category) {
    case "llm":
      return "Brain";
    case "embedding":
      return "Cpu";
    case "storage":
      return "Database";
    case "pipeline":
      return "FileWarning";
    case "network":
      return "Wifi";
    case "unknown":
    default:
      return "AlertCircle";
  }
}

/**
 * Get color class for error category
 */
export function getCategoryColor(category: ErrorCategory): {
  bg: string;
  text: string;
  border: string;
} {
  switch (category) {
    case "llm":
      return {
        bg: "bg-purple-50 dark:bg-purple-950/50",
        text: "text-purple-700 dark:text-purple-400",
        border: "border-purple-200 dark:border-purple-800",
      };
    case "embedding":
      return {
        bg: "bg-blue-50 dark:bg-blue-950/50",
        text: "text-blue-700 dark:text-blue-400",
        border: "border-blue-200 dark:border-blue-800",
      };
    case "storage":
      return {
        bg: "bg-orange-50 dark:bg-orange-950/50",
        text: "text-orange-700 dark:text-orange-400",
        border: "border-orange-200 dark:border-orange-800",
      };
    case "pipeline":
      return {
        bg: "bg-yellow-50 dark:bg-yellow-950/50",
        text: "text-yellow-700 dark:text-yellow-400",
        border: "border-yellow-200 dark:border-yellow-800",
      };
    case "network":
      return {
        bg: "bg-cyan-50 dark:bg-cyan-950/50",
        text: "text-cyan-700 dark:text-cyan-400",
        border: "border-cyan-200 dark:border-cyan-800",
      };
    case "unknown":
    default:
      return {
        bg: "bg-red-50 dark:bg-red-950/50",
        text: "text-red-700 dark:text-red-400",
        border: "border-red-200 dark:border-red-800",
      };
  }
}
