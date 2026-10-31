/**
 * Single SSOT for chat completion request payloads (stream + non-stream).
 * @implements SPEC-155 W7Q DRY
 */
import type { ChatCompletionRequest } from "@/lib/api/chat";
import type { DocumentFilter, QueryMode } from "@/types";

export interface QueryRequestSettings {
  mode: QueryMode;
  maxTokens: number;
  temperature: number;
  topK: number;
  stream: boolean;
  provider?: string;
  model?: string;
  reasoningEffort?: string;
  systemPrompt?: string;
  fullChunkContent?: boolean;
  documentFilter?: DocumentFilter;
  scopedDocumentIds?: string[];
  /** SPEC-180: kind chips (normal workspaces). Empty or absent means all. */
  docKinds?: readonly string[];
}

export interface BuildChatRequestOptions {
  settings: QueryRequestSettings;
  message: string;
  conversationId?: string | null;
  language?: string;
  images?: Array<{ data: string; mime_type: string }>;
  /** SPEC-180: one image to search by (multimodal normal workspaces). Sent as a multipart part. */
  queryImage?: File;
  /** Force stream flag (overrides settings.stream when set). */
  stream?: boolean;
  /** Ask companion entity id(s) for graph-first admission. */
  seedEntityIds?: string[];
}

/**
 * Merge SPEC-031 scopedDocumentIds into a DocumentFilter for the API call.
 * - If neither documentFilter nor scopedDocumentIds are set → undefined
 * - Otherwise merges them: scopedDocumentIds maps to document_ids field
 */
export function buildDocumentFilter(
  settings: Pick<QueryRequestSettings, "documentFilter" | "scopedDocumentIds" | "docKinds">,
): DocumentFilter | undefined {
  const hasScopedIds =
    settings.scopedDocumentIds && settings.scopedDocumentIds.length > 0;
  const hasDateOrPattern =
    settings.documentFilter?.date_from ||
    settings.documentFilter?.date_to ||
    settings.documentFilter?.document_pattern;

  const hasKinds = settings.docKinds && settings.docKinds.length > 0;

  if (!hasScopedIds && !hasDateOrPattern && !hasKinds) return undefined;

  return {
    ...settings.documentFilter,
    document_ids: hasScopedIds ? settings.scopedDocumentIds : undefined,
    doc_kinds: hasKinds ? [...(settings.docKinds ?? [])] : undefined,
  };
}

/** Build a ChatCompletionRequest from UI settings (DRY for stream + non-stream). */
export function buildChatRequest(
  options: BuildChatRequestOptions,
): ChatCompletionRequest {
  const {
    settings,
    message,
    conversationId,
    language,
    images,
    queryImage,
    stream,
    seedEntityIds,
  } = options;

  const seeds = seedEntityIds
    ?.map((id) => id.trim())
    .filter((id) => id.length > 0);

  return {
    conversation_id: conversationId || undefined,
    message,
    mode: settings.mode,
    max_tokens: settings.maxTokens,
    temperature: settings.temperature,
    top_k: settings.topK,
    stream: stream ?? settings.stream,
    provider: settings.provider,
    model: settings.model,
    reasoning_effort: settings.reasoningEffort || undefined,
    language,
    system_prompt: settings.systemPrompt || undefined,
    document_filter: buildDocumentFilter(settings),
    content_granularity: settings.fullChunkContent ? "agent" : "citation",
    images,
    query_image: queryImage,
    seed_entity_ids: seeds && seeds.length > 0 ? seeds : undefined,
  };
}
