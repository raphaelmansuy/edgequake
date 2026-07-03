/**
 * PDF parser backend resolution (SPEC-038) — mirrors backend priority chain.
 *
 * Priority: Upload override > Workspace default > Server env > Vision fallback.
 * @see edgequake-api/src/handlers/pdf_upload/types.rs `resolved_backend`
 */

import type { PdfParserBackend } from "@/types/graph";

/** Per-upload selector value on the Documents dropzone. */
export type UploadPdfParserChoice = "default" | "vision" | "edgeparse";

export type PdfParserResolutionSource = "upload" | "workspace" | "server";

export interface PdfParserResolutionContext {
  /** Upload-level selector (`default` = inherit workspace/server). */
  uploadChoice: UploadPdfParserChoice;
  /** Workspace default from `workspace.pdf_parser_backend`. */
  workspaceBackend?: PdfParserBackend | null;
  /** Optional server override (tests / health); else env + Vision default. */
  serverBackend?: PdfParserBackend;
}

export interface PdfParserResolution {
  backend: PdfParserBackend;
  source: PdfParserResolutionSource;
  /** True when upload or workspace explicitly set the backend. */
  isExplicit: boolean;
}

/** Server default from `EDGEQUAKE_PDF_PARSER_BACKEND` (NEXT_PUBLIC mirror for UI). */
export function getServerDefaultPdfParserBackend(): PdfParserBackend {
  const raw =
    process.env.NEXT_PUBLIC_EDGEQUAKE_PDF_PARSER_BACKEND?.trim().toLowerCase() ??
    "";
  if (raw === "edgeparse" || raw === "edge-parse" || raw === "edge_parse") {
    return "edgeparse";
  }
  return "vision";
}

/**
 * Resolve effective PDF parser backend using the same chain as the upload API.
 */
export function resolvePdfParserBackend(
  ctx: PdfParserResolutionContext,
): PdfParserResolution {
  if (ctx.uploadChoice !== "default") {
    return {
      backend: ctx.uploadChoice,
      source: "upload",
      isExplicit: true,
    };
  }
  if (ctx.workspaceBackend) {
    return {
      backend: ctx.workspaceBackend,
      source: "workspace",
      isExplicit: true,
    };
  }
  const server = ctx.serverBackend ?? getServerDefaultPdfParserBackend();
  return {
    backend: server,
    source: "server",
    isExplicit: false,
  };
}

/** True when the resolved parser is Vision (LLM vision path). */
export function resolvesToVisionParser(
  ctx: PdfParserResolutionContext,
): boolean {
  return resolvePdfParserBackend(ctx).backend === "vision";
}
