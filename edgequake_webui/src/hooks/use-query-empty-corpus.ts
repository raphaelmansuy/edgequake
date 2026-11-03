"use client";

/**
 * SPEC-183 — what the empty Query screen should suggest, given what this corpus holds.
 *
 * The default suggestions used to be one hardcoded list of text-document questions ("What does
 * the handbook say about refunds?"), shown to every Normal workspace. On a workspace of twenty
 * photographs all four were unanswerable, and clicking one produced an honest refusal — so the
 * UI asked a question its own data could not answer.
 *
 * Counting needs the document list, because the stats endpoint reports totals only. The list is
 * already cached by the documents screen, so the cost is bounded: one page, no polling, and the
 * suggestion text is cosmetic — if the request fails, the text corpus copy is used, which is the
 * pre-existing behaviour.
 */

import { useQuery } from "@tanstack/react-query";
import { api } from "@/lib/api/client";
import { useWorkspaceCapabilities } from "@/hooks/use-workspace-capabilities";
import type { CorpusShape } from "@/lib/query/query-empty-copy";
import { useTenantStore } from "@/stores/use-tenant-store";

interface DocumentRow {
  source_type?: string | null;
  content_type?: string | null;
  file_name?: string | null;
  title?: string | null;
}

/** Document family, mirroring the server's `doc_kind` (migration 225). */
export type DocumentFamily = "image" | "pdf" | "text" | "office" | "other";

const OFFICE_MIME = [
  "application/msword",
  "application/vnd.ms-excel",
  "application/vnd.ms-powerpoint",
  "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
  "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
  "application/vnd.openxmlformats-officedocument.presentationml.presentation",
];

/**
 * Classify one document row.
 *
 * WHY `source_type` first: the upload path records the family it actually ingested as
 * (`source_type: "image"`), which is more reliable than re-deriving it from a MIME string the
 * same request may not have carried. The MIME and the extension are fallbacks for older rows.
 */
export function documentFamily(row: DocumentRow): DocumentFamily {
  const source = (row.source_type ?? "").toLowerCase();
  if (source === "image") return "image";
  if (source === "pdf") return "pdf";
  if (source === "markdown" || source === "text") return "text";

  const mime = (row.content_type ?? "").toLowerCase();
  if (mime.startsWith("image/")) return "image";
  if (mime === "application/pdf") return "pdf";
  if (mime.startsWith("text/")) return "text";
  if (OFFICE_MIME.includes(mime)) return "office";

  const name = `${row.file_name ?? ""} ${row.title ?? ""}`.toLowerCase();
  if (/\.(png|jpe?g|gif|webp|bmp|tiff?)\b/.test(name)) return "image";
  if (/\.pdf\b/.test(name)) return "pdf";
  if (/\.(md|markdown|txt|csv|json)\b/.test(name)) return "text";
  if (/\.(docx?|xlsx?|pptx?)\b/.test(name)) return "office";
  return "other";
}

/** True when one document row is a picture. Kept for callers that only need the boolean. */
export function isImageDocument(row: DocumentRow): boolean {
  return documentFamily(row) === "image";
}

/** How a page of documents maps onto the empty state's needs. Pure, so it is testable. */
export function corpusShapeFromDocuments(
  hasGraph: boolean,
  embedsImages: boolean,
  rows: readonly DocumentRow[],
): CorpusShape {
  const count = (family: DocumentFamily) =>
    rows.filter((row) => documentFamily(row) === family).length;
  return {
    hasGraph,
    embedsImages,
    chunkCount: rows.length,
    imageDocumentCount: count("image"),
    pdfDocumentCount: count("pdf"),
    textDocumentCount: count("text"),
    officeDocumentCount: count("office"),
    sampleTitles: rows
      .map((row) => row.title ?? row.file_name ?? "")
      .filter((t) => t.length > 0)
      .slice(0, 10),
  };
}

export function useQueryEmptyCorpus(enabled: boolean): CorpusShape {
  const caps = useWorkspaceCapabilities();
  const workspaceId = useTenantStore((s) => s.selectedWorkspaceId);

  const { data } = useQuery({
    queryKey: ["query-empty-corpus", workspaceId],
    queryFn: async () => {
      const res = await api.get<{ documents?: DocumentRow[]; items?: DocumentRow[] }>(
        "/documents?limit=100",
      );
      return res.documents ?? res.items ?? [];
    },
    enabled: enabled && Boolean(workspaceId),
    staleTime: 60_000,
  });

  return corpusShapeFromDocuments(caps.shows_graph_ui, caps.embeds_images, data ?? []);
}
