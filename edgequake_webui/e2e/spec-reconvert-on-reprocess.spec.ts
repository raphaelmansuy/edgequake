/**
 * PDF Re-conversion on Reprocess — verifies mode=full re-runs PDF -> markdown.
 *
 * Flow:
 *  1. Bootstrap tenant + workspace.
 *  2. Upload a small sample PDF via POST /api/v1/documents/pdf.
 *  3. Poll GET /api/v1/documents until the document reaches a terminal state.
 *  4. Capture the linked pdf_id and the cached markdown.
 *  5. POST /api/v1/documents/reprocess with mode=full (force re-conversion).
 *  6. Poll until the document transitions back to completed.
 *  7. Assert the reprocess was accepted (track_id returned), the same document
 *     id was reused (no orphan), and markdown is present after re-conversion.
 *
 * Run (live stack required):
 *   cd edgequake_webui && EQ_BACKEND_URL=http://localhost:8081 E2E_LIVE_STACK=1 \
 *     PLAYWRIGHT_BASE_URL=http://localhost:3000 pnpm exec playwright test \
 *     spec-reconvert-on-reprocess.spec.ts
 */
import { expect, test, type APIRequestContext, type Page } from "@playwright/test";
import { API_V1_URL, BACKEND_URL } from "./helpers/backend-url";
import { bootstrapDeterministicUiContext } from "./helpers/bootstrap-ui";
import { skipUnlessLiveStack } from "./helpers/live-stack";
import { reconvertScreenshotPath } from "./helpers/reconvert-artifacts";

const SAMPLE_PDF = require.resolve(
  "../../zz-explore/pymupdf4llm/examples/country-capitals/national-capitals.pdf",
);

type DocSummary = {
  id: string;
  status?: string;
  pdf_id?: string;
  source_type?: string;
};

type ListResponse = {
  documents: DocSummary[];
  total: number;
};

type PdfContentResponse = {
  pdf_id: string;
  markdown_content?: string | null;
  is_processed: boolean;
};

async function capture(page: Page, name: string): Promise<void> {
  await page.screenshot({
    path: reconvertScreenshotPath(name),
    fullPage: true,
  });
}

async function uploadPdf(
  request: APIRequestContext,
  headers: Record<string, string>,
  filePath: string,
): Promise<{ document_id?: string; pdf_id?: string; track_id?: string }> {
  const buffer = await (await import("node:fs/promises")).readFile(filePath);
  const filename = filePath.split("/").pop() ?? "sample.pdf";
  const response = await request.post(`${API_V1_URL}/documents/pdf`, {
    headers,
    multipart: {
      file: { name: filename, mimeType: "application/pdf", buffer },
    },
  });
  expect(response.ok(), await response.text()).toBeTruthy();
  return (await response.json()) as {
    document_id?: string;
    pdf_id?: string;
    track_id?: string;
  };
}

async function listDocuments(
  request: APIRequestContext,
  headers: Record<string, string>,
): Promise<DocSummary[]> {
  const resp = await request.get(`${API_V1_URL}/documents`, { headers });
  expect(resp.ok()).toBeTruthy();
  const body = (await resp.json()) as ListResponse;
  return body.documents;
}

async function findDoc(
  request: APIRequestContext,
  headers: Record<string, string>,
  criteria: { pdfId?: string; documentId?: string },
): Promise<DocSummary | undefined> {
  const docs = await listDocuments(request, headers);
  return docs.find((d) => {
    if (criteria.documentId && d.id === criteria.documentId) return true;
    if (criteria.pdfId && d.pdf_id === criteria.pdfId) return true;
    return false;
  });
}

async function pollUntilTerminal(
  request: APIRequestContext,
  headers: Record<string, string>,
  finder: { pdfId?: string; documentId?: string },
  timeoutMs = 300_000,
): Promise<DocSummary | undefined> {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    const doc = await findDoc(request, headers, finder);
    if (doc) {
      const status = (doc.status ?? "").toLowerCase();
      if (["completed", "indexed", "failed", "partial_failure", "cancelled"].includes(status)) {
        return doc;
      }
    }
    await new Promise((r) => setTimeout(r, 3000));
  }
  return undefined;
}

async function getPdfMarkdown(
  request: APIRequestContext,
  headers: Record<string, string>,
  pdfId: string,
): Promise<PdfContentResponse> {
  const resp = await request.get(`${API_V1_URL}/documents/pdf/${pdfId}/content`, { headers });
  expect(resp.ok(), await resp.text()).toBeTruthy();
  return (await resp.json()) as PdfContentResponse;
}

test.describe("PDF Re-conversion on Reprocess", () => {
  test("reprocess mode=full re-runs PDF -> markdown conversion", async ({
    page,
    request,
  }) => {
    skipUnlessLiveStack();

    const ctx = await bootstrapDeterministicUiContext(
      page,
      request,
      "reconvert-reprocess",
    );
    const headers = {
      "X-Tenant-ID": ctx.tenantId,
      "X-Workspace-ID": ctx.workspaceId,
    };

    // 1. Upload the sample PDF.
    const upload = await uploadPdf(request, headers, SAMPLE_PDF);
    expect(upload.pdf_id, "upload must return a pdf_id").toBeTruthy();
    const uploadPdfId = upload.pdf_id!;

    // 2. Wait for first processing to reach a terminal state. The document_id
    //    is only linked once the worker processes the PDF, so find by pdf_id.
    const first = await pollUntilTerminal(
      request,
      headers,
      { pdfId: uploadPdfId },
      300_000,
    );
    expect(first, "document did not reach a terminal state after upload").toBeDefined();

    // If the first pass failed (e.g. vision provider unavailable in CI), we
    // cannot meaningfully assert re-conversion. Skip with a clear reason
    // instead of masking the real infra problem as a test failure.
    const firstStatus = (first!.status ?? "").toLowerCase();
    test.skip(
      !["completed", "indexed"].includes(firstStatus),
      `initial processing did not complete (status=${firstStatus}); re-conversion assertion requires a completed baseline`,
    );

    const documentId = first!.id;
    const pdfId = first!.pdf_id ?? uploadPdfId;
    expect(pdfId, "completed PDF document must expose a pdf_id").toBeTruthy();

    // 3. Capture the cached markdown produced by the first conversion.
    const before = await getPdfMarkdown(request, headers, pdfId);
    const markdownBefore = before.markdown_content ?? "";
    expect(markdownBefore.trim().length, "first conversion must produce markdown").toBeGreaterThan(
      0,
    );

    // 4. Trigger a full re-conversion via the reprocess endpoint with mode=full.
    const reprocessResp = await request.post(`${API_V1_URL}/documents/reprocess`, {
      headers: { ...headers, "Content-Type": "application/json" },
      data: {
        document_id: documentId,
        force: true,
        max_documents: 1,
        mode: "full",
      },
    });
    expect(reprocessResp.ok(), await reprocessResp.text()).toBeTruthy();
    const reprocessBody = (await reprocessResp.json()) as {
      track_id?: string;
      message?: string;
      count?: number;
    };
    expect(reprocessBody.track_id, "reprocess must return a track_id").toBeTruthy();

    // 5. Poll until the document returns to a terminal state.
    const after = await pollUntilTerminal(
      request,
      headers,
      { documentId },
      300_000,
    );
    expect(after, "document did not return to a terminal state after reprocess").toBeDefined();

    const afterStatus = (after!.status ?? "").toLowerCase();
    // A successful re-conversion must end in completed/indexed. If it failed,
    // surface the failure rather than silently passing.
    expect(
      ["completed", "indexed"].includes(afterStatus),
      `reprocess mode=full should complete, got status=${afterStatus}`,
    ).toBeTruthy();

    // 6. The same document id must be reused (no orphan).
    expect(after!.id).toBe(documentId);

    // 7. Markdown must be present again after re-conversion.
    const afterContent = await getPdfMarkdown(request, headers, pdfId);
    const markdownAfter = afterContent.markdown_content ?? "";
    expect(
      markdownAfter.trim().length,
      "re-conversion must regenerate markdown",
    ).toBeGreaterThan(0);

    await page.goto("/documents");
    await page.waitForSelector("main", { timeout: 15_000 });
    await capture(page, "reconvert-on-reprocess-completed.png");
  });

  test("reprocess mode=entities is accepted by the API", async ({ page, request }) => {
    skipUnlessLiveStack();
    // WHY: a separate, cheap assertion that the entities mode path is wired
    // through the API without requiring a full vision re-conversion run.
    const ctx = await bootstrapDeterministicUiContext(
      page,
      request,
      "reconvert-entities-mode",
    );
    const headers = {
      "X-Tenant-ID": ctx.tenantId,
      "X-Workspace-ID": ctx.workspaceId,
      "Content-Type": "application/json",
    };
    // Use a non-existent document id; the API should still accept the mode
    // field (it returns count=0 rather than 400 for unknown docs).
    const resp = await request.post(`${API_V1_URL}/documents/reprocess`, {
      headers,
      data: {
        document_id: "00000000-0000-0000-0000-000000000000",
        force: true,
        max_documents: 1,
        mode: "entities",
      },
    });
    // 200 (count 0) or 404 are both acceptable; a 400 would mean the mode
    // field is rejected — that is the regression we are guarding against.
    expect(resp.status(), await resp.text()).not.toBe(400);
  });

  // Sanity: the spec file resolves against the live backend URL constant so
  // CI does not silently hit the wrong port.
  test("backend URL is configured", async () => {
    skipUnlessLiveStack();
    expect(BACKEND_URL).toBeTruthy();
    expect(API_V1_URL.endsWith("/api/v1")).toBeTruthy();
  });
});
