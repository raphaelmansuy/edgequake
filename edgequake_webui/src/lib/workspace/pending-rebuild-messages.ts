/** Pending rebuild flags after model config changes. */
export interface WorkspacePendingRebuild {
  embeddings: boolean;
  extraction: boolean;
  vision?: boolean;
}

export function hasPendingRebuild(pending: WorkspacePendingRebuild | null): boolean {
  if (!pending) return false;
  return pending.embeddings || pending.extraction || !!pending.vision;
}

/** i18n key suffix for pending rebuild banner (SPEC-017 UI-P3-002). */
export function getPendingRebuildMessageKey(
  pending: WorkspacePendingRebuild,
  options: { includeVision?: boolean } = {},
): string {
  if (pending.embeddings && pending.extraction) {
    return options.includeVision
      ? "workspace.rebuildBothPending"
      : "workspace.rebuildBothPending";
  }
  if (pending.embeddings) {
    return "workspace.rebuildEmbeddingsPending";
  }
  if (options.includeVision && pending.vision) {
    return "workspace.rebuildVisionPending";
  }
  return "workspace.rebuildExtractionPending";
}

export function getPendingRebuildDefaultMessage(
  key: string,
  includeVision: boolean,
): string {
  switch (key) {
    case "workspace.rebuildBothPending":
      return includeVision
        ? 'You changed both LLM and embedding models. Click "Rebuild Knowledge Graph" to reprocess all documents from original files with the new configuration.'
        : 'You changed both LLM and embedding models. Click "Rebuild Embeddings" to reprocess all documents with the new configuration.';
    case "workspace.rebuildEmbeddingsPending":
      return 'You changed the embedding model. Click "Rebuild Embeddings" to regenerate vector embeddings.';
    case "workspace.rebuildVisionPending":
      return 'You changed the Vision LLM model. Click "Rebuild Knowledge Graph" to re-extract all PDF documents from their original files using the new vision model.';
    default:
      return includeVision
        ? 'You changed the LLM model. Click "Rebuild Knowledge Graph" to re-extract entities from all documents.'
        : 'You changed the LLM model. Click "Rebuild Embeddings" to re-extract entities from all documents.';
  }
}
