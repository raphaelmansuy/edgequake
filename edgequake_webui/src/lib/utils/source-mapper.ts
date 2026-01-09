/**
 * @module source-mapper
 * @description Source Mapper Utility
 *
 * Converts between API response format (SourceReference[]) and UI format (QueryContext).
 * This is necessary because the backend returns a flat list of sources with source_type,
 * while the UI components expect a structured QueryContext with separate arrays.
 *
 * @implements FEAT0718 - Source reference to QueryContext mapping
 * @implements FEAT0719 - Entity/relationship/chunk categorization
 *
 * @enforces BR0715 - Unknown source types go to chunks
 * @enforces BR0716 - Preserve source order for display
 */

import type { SourceReference } from "@/lib/api/chat";
import type { QueryContext } from "@/types";

/**
 * Maps SourceReference[] from API to QueryContext for UI display.
 *
 * @param sources - Array of source references from the API
 * @returns QueryContext object for use in UI components
 *
 * @example
 * ```typescript
 * const sources: SourceReference[] = [
 *   { source_type: 'entity', id: 'SARAH_CHEN', score: 0.95, document_id: 'doc-1', file_path: 'test.md' },
 *   { source_type: 'chunk', id: 'chunk-1', score: 0.85, snippet: 'Some content...', document_id: 'doc-1' },
 * ];
 * const context = mapSourcesToContext(sources);
 * // context.entities[0].source_document_id === 'doc-1'
 * ```
 */
export function mapSourcesToContext(sources: SourceReference[]): QueryContext {
  if (!sources || sources.length === 0) {
    return { chunks: [], entities: [], relationships: [] };
  }

  return {
    chunks: sources
      .filter((s) => s.source_type === "chunk")
      .map((s) => ({
        content: s.snippet || "",
        // Extract document ID from chunk ID (format: "uuid-chunk-N" -> "uuid")
        document_id: extractDocumentId(s.id),
        score: s.score,
        file_path: s.file_path,
      })),

    entities: sources
      .filter((s) => s.source_type === "entity")
      .map((s) => ({
        id: s.id,
        label: s.id, // Entity name is stored in the id field
        relevance: s.score,
        source_document_id: s.document_id,
        source_file_path: s.file_path,
        // Note: source_chunk_ids are not available in SourceReference
      })),

    relationships: sources
      .filter((s) => s.source_type === "relationship")
      .map((s) => {
        // Relationship id is formatted as "SOURCE->TARGET"
        const parts = s.id.split("->");
        const sourceEntity = parts[0]?.trim() || "";
        const targetEntity = parts[1]?.trim() || "";

        return {
          source: sourceEntity,
          target: targetEntity,
          type: extractRelationType(s.snippet) || "RELATED_TO",
          relevance: s.score,
          source_document_id: s.document_id,
          source_file_path: s.file_path,
        };
      }),
  };
}

/**
 * Extracts the relationship type from a snippet.
 * The snippet format is typically "SOURCE RELATION_TYPE TARGET"
 *
 * @param snippet - The relationship snippet
 * @returns The relationship type or undefined
 */
function extractRelationType(snippet: string | undefined): string | undefined {
  if (!snippet) return undefined;

  // Try to extract the middle word(s) from "SOURCE RELATION_TYPE TARGET"
  const words = snippet.trim().split(/\s+/);
  if (words.length >= 3) {
    // The relation type is everything between the first and last word
    return words.slice(1, -1).join("_").toUpperCase();
  }

  return undefined;
}

/**
 * Extracts the document UUID from a chunk ID.
 * Chunk IDs are formatted as "uuid-chunk-N" where uuid is a UUID and N is the chunk index.
 *
 * @param chunkId - The full chunk ID (e.g., "f0291a69-8b63-46d5-b44b-24095b3a8283-chunk-0")
 * @returns The document UUID (e.g., "f0291a69-8b63-46d5-b44b-24095b3a8283")
 */
function extractDocumentId(chunkId: string): string {
  if (!chunkId) return chunkId;

  // Look for "-chunk-" suffix and extract everything before it
  const chunkSuffixIndex = chunkId.lastIndexOf("-chunk-");
  if (chunkSuffixIndex > 0) {
    return chunkId.substring(0, chunkSuffixIndex);
  }

  // Fallback: return as-is if no "-chunk-" suffix found
  return chunkId;
}

/**
 * Checks if a QueryContext has any meaningful content.
 *
 * @param context - The QueryContext to check
 * @returns true if context has any chunks, entities, or relationships
 */
export function hasContextContent(
  context: QueryContext | undefined | null
): boolean {
  if (!context) return false;

  return (
    (context.chunks?.length ?? 0) > 0 ||
    (context.entities?.length ?? 0) > 0 ||
    (context.relationships?.length ?? 0) > 0
  );
}
