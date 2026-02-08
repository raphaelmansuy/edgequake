/**
 * @module progress-formatter
 * @description Utility functions for formatting ingestion progress messages.
 *
 * WHY: Centralizes progress formatting logic following DRY principle.
 * Provides consistent, human-readable progress messages across the UI.
 *
 * @implements DRY: Single source of truth for progress formatting
 * @implements SRP: Each function has one responsibility
 */

import type {
  ChunkProgress,
  IngestionProgress,
  IngestionStage,
  PdfProgress,
} from "@/types/ingestion";

// ============================================================================
// Time Formatting
// ============================================================================

/**
 * Format duration in seconds to human-readable string.
 *
 * Examples:
 * - 30 → "30s"
 * - 90 → "1m 30s"
 * - 3665 → "1h 1m"
 */
export function formatDuration(seconds: number): string {
  if (seconds < 60) {
    return `${Math.round(seconds)}s`;
  }

  const hours = Math.floor(seconds / 3600);
  const minutes = Math.floor((seconds % 3600) / 60);
  const secs = Math.round(seconds % 60);

  const parts: string[] = [];
  if (hours > 0) parts.push(`${hours}h`);
  if (minutes > 0) parts.push(`${minutes}m`);
  if (secs > 0 && hours === 0) parts.push(`${secs}s`);

  return parts.join(" ");
}

/**
 * Format ETA (Estimated Time Remaining).
 *
 * Examples:
 * - undefined → ""
 * - 0 → ""
 * - 30 → " (ETA: 30s)"
 * - 90 → " (ETA: 1m 30s)"
 */
export function formatEta(seconds?: number): string {
  if (!seconds || seconds <= 0) return "";
  return ` (ETA: ${formatDuration(seconds)})`;
}

// ============================================================================
// Cost Formatting
// ============================================================================

/**
 * Format cost in USD to human-readable string.
 *
 * Examples:
 * - 0.0001 → "$0.0001"
 * - 0.123456 → "$0.1235"
 * - 1.5 → "$1.50"
 */
export function formatCost(usd: number): string {
  if (usd < 0.01) {
    // Show 4 decimal places for small amounts
    return `$${usd.toFixed(4)}`;
  }
  // Show 2 decimal places for normal amounts
  return `$${usd.toFixed(2)}`;
}

// ============================================================================
// Stage Formatting
// ============================================================================

/**
 * Get human-readable stage name.
 *
 * WHY: Backend stage names are technical (e.g., "extracting").
 * This provides user-friendly names for the UI.
 */
export function formatStageName(stage: IngestionStage): string {
  const stageNames: Partial<Record<IngestionStage, string>> = {
    uploading: "Uploading",
    converting: "Converting PDF",
    preprocessing: "Preprocessing",
    chunking: "Chunking",
    extracting: "Extracting Entities",
    gleaning: "Refining Extraction",
    merging: "Merging Graph",
    summarizing: "Summarizing",
    embedding: "Generating Embeddings",
    storing: "Storing",
    indexing: "Indexing",
    completed: "Completed",
    failed: "Failed",
    pending: "Pending",
  };

  return stageNames[stage] || stage;
}

// ============================================================================
// PDF Progress Formatting
// ============================================================================

/**
 * Format PDF extraction progress message.
 *
 * @implements OODA-06: PDF page-by-page progress display
 *
 * Examples:
 * - { current_page: 0, total_pages: 10 } → "Starting PDF extraction (10 pages)..."
 * - { current_page: 5, total_pages: 10, progress: 50 } → "Converting PDF: page 5/10 (50%)"
 * - { phase: "complete", total_pages: 10 } → "PDF conversion complete (10 pages)"
 */
export function formatPdfProgress(pdf: PdfProgress): string {
  if (pdf.phase === "complete") {
    return `PDF conversion complete (${pdf.total_pages} pages)`;
  }

  if (pdf.current_page === 0) {
    return `Starting PDF extraction (${pdf.total_pages} pages)...`;
  }

  return `Converting PDF: page ${pdf.current_page}/${pdf.total_pages} (${pdf.progress}%)`;
}

/**
 * Format PDF progress with optional error.
 */
export function formatPdfProgressWithError(pdf: PdfProgress): string {
  const baseMsg = formatPdfProgress(pdf);

  if (pdf.last_error) {
    return `${baseMsg} - Error: ${pdf.last_error}`;
  }

  return baseMsg;
}

// ============================================================================
// Chunk Progress Formatting
// ============================================================================

/**
 * Format chunk extraction progress message.
 *
 * @implements SPEC-001/Objective-A: Chunk-level progress display
 *
 * Examples:
 * - { current_chunk: 1, total_chunks: 10 } → "Extracting: chunk 1/10"
 * - { current_chunk: 5, total_chunks: 10, eta_seconds: 30 } → "Extracting: chunk 5/10 (ETA: 30s)"
 * - { current_chunk: 5, total_chunks: 10, failed_chunks: 2 } → "Extracting: chunk 5/10 (2 failed)"
 */
export function formatChunkProgress(chunk: ChunkProgress): string {
  const baseMsg = `Extracting: chunk ${chunk.current_chunk}/${chunk.total_chunks}`;

  const parts: string[] = [];

  if (chunk.eta_seconds && chunk.eta_seconds > 0) {
    parts.push(`ETA: ${formatDuration(chunk.eta_seconds)}`);
  }

  if (chunk.failed_chunks > 0) {
    parts.push(`${chunk.failed_chunks} failed`);
  }

  if (chunk.cumulative_cost && chunk.cumulative_cost > 0) {
    parts.push(formatCost(chunk.cumulative_cost));
  }

  if (parts.length > 0) {
    return `${baseMsg} (${parts.join(", ")})`;
  }

  return baseMsg;
}

/**
 * Format chunk preview for tooltip/detail view.
 *
 * Truncates long previews to 80 characters.
 */
export function formatChunkPreview(preview?: string): string {
  if (!preview) return "";

  const maxLength = 80;
  if (preview.length <= maxLength) {
    return preview;
  }

  return `${preview.substring(0, maxLength - 3)}...`;
}

// ============================================================================
// Overall Progress Formatting
// ============================================================================

/**
 * Format overall ingestion progress message.
 *
 * WHY: Provides a single, comprehensive status message that prioritizes
 * the most granular progress information available:
 * 1. Chunk progress (during extraction)
 * 2. PDF progress (during conversion)
 * 3. Stage-specific message
 * 4. Fallback to generic stage name
 */
export function formatOverallProgress(track: IngestionProgress): string {
  // Priority 1: Chunk progress (most granular during extraction)
  if (track.progress.chunk_progress) {
    return formatChunkProgress(track.progress.chunk_progress);
  }

  // Priority 2: PDF progress (during conversion)
  if (track.progress.pdf_progress) {
    return formatPdfProgress(track.progress.pdf_progress);
  }

  // Priority 3: Custom stage message
  if (track.progress.latest_message) {
    return track.progress.latest_message;
  }

  // Fallback: Generic stage name
  return formatStageName(track.progress.current_stage);
}

/**
 * Format progress percentage with optional details.
 *
 * Examples:
 * - 0 → "0%"
 * - 42 → "42%"
 * - 100 → "100%"
 */
export function formatProgressPercentage(progress: number): string {
  return `${Math.round(progress)}%`;
}

/**
 * Format progress bar description (for accessibility).
 */
export function formatProgressAccessibility(track: IngestionProgress): string {
  const pct = formatProgressPercentage(track.overall_progress);
  const stage = formatStageName(track.progress.current_stage);
  const msg = formatOverallProgress(track);

  return `${pct} complete - ${stage}: ${msg}`;
}
