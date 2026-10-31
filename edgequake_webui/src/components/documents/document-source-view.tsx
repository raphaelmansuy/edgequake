/**
 * SPEC-157 — Presentational document viewer shared by the Query companion
 * pane. A PDF shows the real page (the evidence); text-like documents show
 * the rendered content with the cited passage highlighted. Data loading
 * lives in `useDocumentSource` so this stays a pure view.
 */
"use client";

import { ContentRenderer } from "@/components/document/content-renderer";
import {
  DocumentImageViewer,
  isImageDocument,
} from "@/components/documents/document-image-viewer";
import { PDFViewer } from "@/components/documents/pdf-viewer";
import { Skeleton } from "@/components/ui/skeleton";
import { getPdfDownloadUrl } from "@/lib/api/edgequake";
import type { SourceLocation } from "@/lib/query/companion-pane";
import type { Document } from "@/types";
import { useState } from "react";

export type SourceViewMode = "page" | "text";

interface DocumentSourceViewProps {
  document: Document;
  pdfId: string | null;
  location: SourceLocation;
  mode: SourceViewMode;
  /** Changes on every explicit open so a repeated click re-navigates. */
  seq: number;
}

/** Page the viewer should show: a manual scroll wins until the next open. */
function useViewPage(location: SourceLocation, seq: number) {
  const key = `${location.documentId}:${location.page ?? 0}:${seq}`;
  const [manual, setManual] = useState<{ key: string; page: number } | null>(
    null,
  );
  const page = manual?.key === key ? manual.page : (location.page ?? 1);
  return { page, setPage: (p: number) => setManual({ key, page: p }) };
}

export function DocumentSourceView({
  document,
  pdfId,
  location,
  mode,
  seq,
}: DocumentSourceViewProps) {
  const { page, setPage } = useViewPage(location, seq);

  if (pdfId && mode === "page") {
    return (
      <div className="h-full min-h-0" data-testid="companion-source-pdf">
        <PDFViewer
          key={pdfId}
          file={getPdfDownloadUrl(pdfId)}
          initialPage={page}
          currentPage={page}
          onPageChange={setPage}
          documentId={document.id}
        />
      </div>
    );
  }

  // SPEC-180: an image document has no text body — its `content` is the file name. Show the
  // picture, so a cited image is evidence and not a filename in a text pane.
  if (isImageDocument(document)) {
    return (
      <div className="h-full min-h-0" data-testid="companion-source-image">
        <DocumentImageViewer document={document} className="h-full" />
      </div>
    );
  }

  return (
    <div
      className="h-full min-h-0 overflow-auto"
      data-testid="companion-source-text"
    >
      <ContentRenderer
        document={document}
        highlightText={location.passage?.slice(0, 100)}
        startLine={location.startLine}
        endLine={location.endLine}
        activePage={location.page}
      />
    </div>
  );
}

export function DocumentSourceSkeleton() {
  return (
    <div className="space-y-3 p-4" aria-hidden data-testid="companion-source-skeleton">
      <Skeleton className="h-8 w-2/3" />
      <Skeleton className="h-[320px] w-full" />
      <Skeleton className="h-4 w-1/2" />
    </div>
  );
}
