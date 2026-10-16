/**
 * One end-to-end test per EdgeQuake user journey.
 *
 * Live stack: PostgreSQL + Mistral (LLM, vision, embeddings). Document
 * journeys upload the repo PDF and markdown fixtures — not generated stubs.
 *
 * Server env (do not log the key): MISTRAL_API_KEY, EDGEQUAKE_LLM_PROVIDER=mistral,
 * EDGEQUAKE_EMBEDDING_PROVIDER=mistral, EDGEQUAKE_VISION_PROVIDER=mistral,
 * EDGEQUAKE_VISION_MODEL=mistral-small-latest, EDGEQUAKE_LLM_MODEL=mistral-small-latest.
 *
 *   USER_JOURNEY_MEDIA_DIR=... E2E_LIVE_STACK=1 \
 *     pnpm exec playwright test e2e/user-journeys.spec.ts --project=chromium
 */
import { expect, test, type Page } from "@playwright/test";

import { waitForAppReady } from "./helpers/app-ready";
import { BACKEND_URL } from "./helpers/backend-url";
import { skipUnlessLiveStack } from "./helpers/live-stack";
import {
  MISTRAL_EMBEDDING_MODEL,
  MISTRAL_LLM_MODEL,
  assertWorkspaceUsesMistral,
  createTenantWorkspaceViaApi,
  mistralWorkspacePayload,
} from "./helpers/spec013-api";
import { seedTenantStoreOnPage } from "./helpers/spec013-bootstrap";
import { uploadFilesOnDocumentsPage } from "./helpers/upload";
import { gotoQueryPage, submitQueryAndWait } from "./helpers/qc-query";
import {
  MARKDOWN_PATH,
  MARKDOWN_RELATIVE,
  PDF_PATH,
  PDF_RELATIVE,
  apiHeaders,
  apiJson,
  assertCompleted,
  captureJson,
  captureScreenshot,
  captureText,
  docId,
  listDocuments,
  loadJourneyState,
  recordJourney,
  saveJourneyState,
  waitForDocument,
  type JourneyContext,
} from "./helpers/user-journey";

const state: JourneyContext = {
  tenantId: "",
  workspaceId: "",
  workspaceName: "",
  workspaceSlug: "",
  headers: {},
};

test.beforeEach(async () => {
  skipUnlessLiveStack();
  const loaded = await loadJourneyState();
  if (loaded?.workspaceId) {
    state.tenantId = loaded.tenantId;
    state.workspaceId = loaded.workspaceId;
    state.workspaceName = loaded.workspaceName;
    state.workspaceSlug = loaded.workspaceSlug;
    state.headers = loaded.headers;
    state.docs = loaded.docs;
  }
});

test.afterEach(async ({ page }, info) => {
  if (info.status !== "passed") {
    const slug = info.title.toLowerCase().replace(/[^a-z0-9]+/g, "-").replace(/^-|-$/g, "");
    await captureScreenshot(page, `fail-${slug}`).catch(() => undefined);
  }
  await recordJourney(info, {
    workspaceId: state.workspaceId || undefined,
    pdfId: state.docs?.pdfId,
    markdownId: state.docs?.markdownId,
  });
});

async function requireIngested(request: import("@playwright/test").APIRequestContext): Promise<void> {
  expect(state.docs, "upload did not record document ids").toBeTruthy();
  const pdf = await apiJson(request, "GET", `/documents/${state.docs!.pdfId}`, state.headers);
  const markdown = await apiJson(
    request,
    "GET",
    `/documents/${state.docs!.markdownId}`,
    state.headers,
  );
  expect(pdf.status, pdf.text.slice(0, 300)).toBe(200);
  expect(markdown.status, markdown.text.slice(0, 300)).toBe(200);
  assertCompleted(pdf.body as { status?: string; error_message?: string | null; failure_code?: string | null }, "pdf");
  assertCompleted(
    markdown.body as { status?: string; error_message?: string | null; failure_code?: string | null },
    "markdown",
  );
}

async function ready(page: Page): Promise<void> {
  await seedTenantStoreOnPage(page, {
    tenantId: state.tenantId,
    tenantName: state.workspaceName,
    workspaceId: state.workspaceId,
    workspaceName: state.workspaceName,
    workspaceSlug: state.workspaceSlug,
  });
}

test("login", async ({ page }) => {
  test.setTimeout(60_000);
  await page.goto("/login", { waitUntil: "domcontentloaded" });
  await expect(page.getByText("EdgeQuake", { exact: true })).toBeVisible({ timeout: 20_000 });
  await expect(page.getByRole("button", { name: "Sign In" })).toBeVisible();
  await captureScreenshot(page, "01-login");
});

test("sso callback", async ({ page }) => {
  test.setTimeout(60_000);
  await page.goto("/auth/callback", { waitUntil: "domcontentloaded" });
  await expect(page.getByTestId("sso-error")).toBeVisible({ timeout: 20_000 });
  await captureScreenshot(page, "02-sso-callback");
});

test("provider health", async ({ request }) => {
  test.setTimeout(60_000);
  const health = await request.get(`${BACKEND_URL}/health`);
  expect(health.ok()).toBeTruthy();
  const body = (await health.json()) as {
    status?: string;
    storage_mode?: string;
    llm_provider_name?: string;
  };
  expect(body.status).toMatch(/healthy|degraded/);
  expect(body.storage_mode).toBe("postgresql");
  expect(body.llm_provider_name).toBe("mistral");
  await captureJson("03-provider-health", body);
});

test("tenant and workspace", async ({ page, request }) => {
  test.setTimeout(90_000);
  const created = await createTenantWorkspaceViaApi(request, "user-journey", {
    slug: `user-journey-${Date.now()}`,
    workspacePayload: mistralWorkspacePayload("pending", [
      "PERSON",
      "ORGANIZATION",
      "LOCATION",
      "CONCEPT",
      "PRODUCT",
      "OTHER",
    ], {
      entityTypesStrict: false,
      relationTypesStrict: false,
    }),
  });
  state.tenantId = created.tenantId;
  state.workspaceId = created.workspaceId;
  state.workspaceName = created.workspaceName;
  state.workspaceSlug = created.workspaceSlug;
  state.headers = apiHeaders(state);
  await saveJourneyState(state);

  const ws = await apiJson(request, "GET", `/workspaces/${state.workspaceId}`, state.headers);
  expect(ws.status, ws.text).toBe(200);
  const workspace = ws.body as Record<string, unknown>;
  assertWorkspaceUsesMistral(workspace);
  expect(workspace.vision_llm_provider ?? workspace.vision_provider).toBe("mistral");
  const visionModel = String(
    workspace.vision_llm_model ?? workspace.vision_model ?? MISTRAL_LLM_MODEL,
  );
  expect(visionModel.toLowerCase()).toContain("mistral");
  expect(workspace.embedding_model).toBe(MISTRAL_EMBEDDING_MODEL);
  await captureJson("04-tenant-workspace", {
    tenantId: state.tenantId,
    workspaceId: state.workspaceId,
    slug: state.workspaceSlug,
    llm_provider: workspace.llm_provider,
    llm_model: workspace.llm_model,
    embedding_provider: workspace.embedding_provider,
    embedding_model: workspace.embedding_model,
    vision_llm_provider: workspace.vision_llm_provider ?? workspace.vision_provider,
    vision_llm_model: workspace.vision_llm_model ?? workspace.vision_model,
  });

  await ready(page);
  await expect(page.getByTestId("workspace-selector")).toBeVisible();
  await captureScreenshot(page, "04-tenant-workspace");
});

test("dashboard", async ({ page }) => {
  test.setTimeout(60_000);
  await ready(page);
  await page.goto("/", { waitUntil: "domcontentloaded" });
  await waitForAppReady(page);
  await expect(page.getByTestId("spec100-dashboard-subtitle")).toBeVisible({
    timeout: 20_000,
  });
  await captureScreenshot(page, "05-dashboard");
});

test("upload pdf and markdown", async ({ page, request }) => {
  test.setTimeout(180_000);
  await ready(page);
  await uploadFilesOnDocumentsPage(page, [PDF_PATH, MARKDOWN_PATH]);
  await expect(page.getByTestId("documents-page-shell")).toBeVisible();

  const deadline = Date.now() + 90_000;
  let pdfId = "";
  let markdownId = "";
  while (Date.now() < deadline && (!pdfId || !markdownId)) {
    const rows = await listDocuments(request, state);
    for (const row of rows) {
      const name = `${row.file_name ?? ""} ${row.title ?? ""}`;
      if (name.includes("scottish_smes")) pdfId = docId(row);
      if (name.includes("EF-extract-208")) markdownId = docId(row);
    }
    if (!pdfId || !markdownId) {
      await page.waitForTimeout(2_000);
    }
  }
  expect(pdfId, "uploaded PDF did not appear in the document list").not.toBe("");
  expect(markdownId, "uploaded markdown did not appear in the document list").not.toBe("");
  state.docs = {
    pdfId,
    markdownId,
    pdfName: "scottish_smes.pdf",
    markdownName: "EF-extract-208.md",
  };
  await saveJourneyState(state);
  await captureJson("06-upload", {
    pdf: PDF_RELATIVE,
    markdown: MARKDOWN_RELATIVE,
    ...state.docs,
  });
  await captureScreenshot(page, "06-upload");
});

test("processing", async ({ page, request }) => {
  test.setTimeout(20 * 60_000);
  expect(state.docs).toBeTruthy();
  const pdf = await waitForDocument(request, state, state.docs!.pdfId, 12 * 60_000);
  const markdown = await waitForDocument(request, state, state.docs!.markdownId, 8 * 60_000);
  await captureJson("07-processing", {
    pdf: { id: state.docs!.pdfId, file: PDF_RELATIVE, ...pdf },
    markdown: { id: state.docs!.markdownId, file: MARKDOWN_RELATIVE, ...markdown },
  });
  await ready(page);
  await page.goto("/documents", { waitUntil: "domcontentloaded" });
  await waitForAppReady(page);
  await captureScreenshot(page, "07-processing");
  assertCompleted(pdf, "pdf");
  assertCompleted(markdown, "markdown");
  expect(pdf.chunk_count ?? 0).toBeGreaterThan(0);
  expect(markdown.chunk_count ?? 0).toBeGreaterThan(0);

  await page.goto("/pipeline", { waitUntil: "domcontentloaded" });
  await expect(page.getByTestId("pipeline-page")).toBeVisible();
});

test("document view", async ({ page, request }) => {
  test.setTimeout(120_000);
  await requireIngested(request);
  await ready(page);

  await page.goto(`/documents/${state.docs!.markdownId}`, { waitUntil: "domcontentloaded" });
  await expect(page.getByRole("heading", { name: "EF-extract-208.md", exact: true })).toBeVisible({
    timeout: 30_000,
  });
  await expect(page.getByRole("heading", { name: "PEUGEOT 208", exact: true })).toBeVisible({
    timeout: 30_000,
  });
  await captureScreenshot(page, "08-document-view-markdown");

  await page.goto(`/documents/${state.docs!.pdfId}`, { waitUntil: "domcontentloaded" });
  await expect(page.getByRole("heading", { name: /scottish_smes/i })).toBeVisible({
    timeout: 30_000,
  });
  await expect(page.getByTestId("pdf-viewer")).toBeVisible({ timeout: 30_000 });
  await expect(page.getByText("Extracted Markdown")).toBeVisible();
  await captureScreenshot(page, "08-document-view-pdf");
});

test("query", async ({ page, request }) => {
  test.setTimeout(8 * 60_000);
  await requireIngested(request);
  await ready(page);
  await gotoQueryPage(page);
  await page.getByRole("button", { name: "All docs" }).click();
  await page.getByRole("option", { name: /EF-extract-208|PEUGEOT/i }).click();

  const markdownAnswer = await submitQueryAndWait(
    page,
    "What are the Peugeot 208 hybrid prices in the uploaded brochure?",
    { answerTimeoutMs: 180_000, processingTimeoutMs: 180_000 },
  );
  expect(markdownAnswer.length).toBeGreaterThan(40);
  expect(markdownAnswer).not.toMatch(/Mock response/i);
  expect(markdownAnswer).toMatch(/208|Peugeot|hybrid|€|euro/i);
  await captureScreenshot(page, "09-query-markdown");

  const pdfAnswer = await submitQueryAndWait(
    page,
    "Which organizations or training providers are named in the Scottish SME PDF?",
    { answerTimeoutMs: 180_000, processingTimeoutMs: 180_000 },
  );
  expect(pdfAnswer.length).toBeGreaterThan(40);
  expect(pdfAnswer).not.toMatch(/Mock response/i);
  await captureScreenshot(page, "09-query-pdf");
  await captureText(
    "09-query.txt",
    `markdown question document: ${MARKDOWN_RELATIVE}\n${markdownAnswer}\n\npdf question document: ${PDF_RELATIVE}\n${pdfAnswer}\n`,
  );

  const apiAnswer = await apiJson(request, "POST", "/query", state.headers, {
    query: "Summarize the Peugeot 208 electric and hybrid range from the brochure.",
    mode: "hybrid",
    llm_provider: "mistral",
    llm_model: MISTRAL_LLM_MODEL,
  });
  expect(apiAnswer.status, apiAnswer.text.slice(0, 500)).toBe(200);
  await captureJson("09-query-api", {
    status: apiAnswer.status,
    document: MARKDOWN_RELATIVE,
    body: apiAnswer.body,
  });
});

test("knowledge graph", async ({ page, request }) => {
  test.setTimeout(180_000);
  await requireIngested(request);
  const graph = await apiJson(request, "GET", "/graph?limit=200", state.headers);
  expect(graph.status, graph.text.slice(0, 400)).toBe(200);
  const body = graph.body as { nodes?: unknown[]; entities?: unknown[] };
  const nodes = body.nodes ?? body.entities ?? [];
  expect(nodes.length).toBeGreaterThan(0);
  await captureJson("10-knowledge-graph", {
    nodeCount: nodes.length,
    pdf: PDF_RELATIVE,
    markdown: MARKDOWN_RELATIVE,
    sample: nodes.slice(0, 8),
  });

  await ready(page);
  await page.goto("/graph", { waitUntil: "domcontentloaded" });
  await waitForAppReady(page);
  await expect(page.locator("canvas").first()).toBeVisible({ timeout: 60_000 });
  await expect(page.getByTestId("graph-filter-search")).toBeVisible();
  await captureScreenshot(page, "10-knowledge-graph");
});

test("lineage", async ({ page, request }) => {
  test.setTimeout(120_000);
  await requireIngested(request);
  const pdfLineage = await apiJson(
    request,
    "GET",
    `/documents/${state.docs!.pdfId}/lineage`,
    state.headers,
  );
  const mdLineage = await apiJson(
    request,
    "GET",
    `/documents/${state.docs!.markdownId}/lineage`,
    state.headers,
  );
  expect(pdfLineage.status, pdfLineage.text.slice(0, 400)).toBe(200);
  expect(mdLineage.status, mdLineage.text.slice(0, 400)).toBe(200);
  await captureJson("11-lineage", {
    pdf: { file: PDF_RELATIVE, id: state.docs!.pdfId, body: pdfLineage.body },
    markdown: { file: MARKDOWN_RELATIVE, id: state.docs!.markdownId, body: mdLineage.body },
  });

  await ready(page);
  await page.goto(`/documents/${state.docs!.markdownId}`, { waitUntil: "domcontentloaded" });
  await expect(page.getByTestId("data-hierarchy-section")).toBeVisible({ timeout: 30_000 });
  await captureScreenshot(page, "11-lineage-markdown");
  await page.goto(`/documents/${state.docs!.pdfId}`, { waitUntil: "domcontentloaded" });
  await expect(page.getByTestId("data-hierarchy-section")).toBeVisible({ timeout: 30_000 });
  await captureScreenshot(page, "11-lineage-pdf");
});

test("knowledge injection", async ({ page, request }) => {
  test.setTimeout(4 * 60_000);
  await ready(page);
  await page.goto("/knowledge", { waitUntil: "domcontentloaded" });
  await expect(page.getByRole("heading", { name: "Knowledge Injection" })).toBeVisible({
    timeout: 20_000,
  });
  await page.getByRole("button", { name: "New Injection" }).click();
  await page.getByRole("tab", { name: "File" }).click();
  await page.locator('input[type="file"]').last().setInputFiles(MARKDOWN_PATH);
  await expect(page.getByText("EF-extract-208.md")).toBeVisible();
  await captureScreenshot(page, "12-knowledge-injection-dialog");
  const uploaded = page.waitForResponse(
    (response) =>
      response.url().includes("/injection/file") &&
      response.request().method() === "PUT" &&
      response.ok(),
  );
  await page.getByRole("button", { name: "Upload" }).click();
  const created = (await (await uploaded).json()) as { injection_id?: string };
  const injectionId = created.injection_id ?? "";
  expect(injectionId, "upload did not return an injection id").not.toBe("");

  const deadline = Date.now() + 150_000;
  let injection: Record<string, unknown> | undefined;
  let listedBody: unknown;
  while (Date.now() < deadline) {
    const listed = await apiJson(
      request,
      "GET",
      `/workspaces/${state.workspaceId}/injections`,
      state.headers,
    );
    listedBody = listed.body;
    const items = ((listed.body as { items?: Record<string, unknown>[] })?.items ??
      (Array.isArray(listed.body) ? listed.body : [])) as Record<string, unknown>[];
    injection = items.find((item) => String(item.injection_id ?? "") === injectionId);
    const status = String(injection?.status ?? "");
    if (status === "completed" || status === "failed") break;
    await page.waitForTimeout(3_000);
  }
  await captureJson("12-knowledge-injection", {
    file: MARKDOWN_RELATIVE,
    injection: injection ?? null,
    list: listedBody,
  });
  await captureScreenshot(page, "12-knowledge-injection");
  expect(injection, "this upload was not listed").toBeTruthy();
  expect(String(injection?.status), JSON.stringify(injection)).toBe("completed");
  await expect(page.getByRole("heading", { name: "No knowledge injections yet" })).toHaveCount(0);
});

test("workspace admin", async ({ page }) => {
  test.setTimeout(60_000);
  await ready(page);
  await page.goto("/workspace", { waitUntil: "domcontentloaded" });
  await expect(page.getByRole("heading", { name: state.workspaceName })).toBeVisible({
    timeout: 20_000,
  });
  await expect(page.getByTestId("workspace-kg-schema-row")).toBeVisible();
  await captureScreenshot(page, "13-workspace");
});

test("pipeline monitor", async ({ page }) => {
  test.setTimeout(60_000);
  await ready(page);
  await page.goto("/pipeline", { waitUntil: "domcontentloaded" });
  await expect(page.getByTestId("pipeline-page")).toBeVisible({ timeout: 20_000 });
  await expect(page.getByRole("heading", { name: "Pipeline Status" })).toBeVisible();
  await captureScreenshot(page, "14-pipeline");
});

test("costs", async ({ page }) => {
  test.setTimeout(60_000);
  await ready(page);
  await page.goto("/costs", { waitUntil: "domcontentloaded" });
  await expect(page.getByRole("heading", { name: "Cost Dashboard" })).toBeVisible({
    timeout: 20_000,
  });
  await captureScreenshot(page, "15-costs");
});

test("api explorer", async ({ page }) => {
  test.setTimeout(90_000);
  await ready(page);
  await page.goto("/api-explorer", { waitUntil: "domcontentloaded" });
  await expect(page.getByTestId("api-explorer-page")).toBeVisible({ timeout: 30_000 });
  await captureScreenshot(page, "16-api-explorer");
});

test("settings", async ({ page }) => {
  test.setTimeout(60_000);
  await ready(page);
  await page.goto("/settings", { waitUntil: "domcontentloaded" });
  await expect(page.getByRole("heading", { name: "Settings" })).toBeVisible({
    timeout: 20_000,
  });
  await expect(page.getByText(/mistral/i).first()).toBeVisible({ timeout: 20_000 });
  await captureScreenshot(page, "17-settings");
});

test("workspace deeplink", async ({ page }) => {
  test.setTimeout(90_000);
  await ready(page);
  await page.goto(`/w/${state.workspaceSlug}/documents`, { waitUntil: "domcontentloaded" });
  await expect(page.getByTestId("documents-page-shell")).toBeVisible({ timeout: 30_000 });
  await expect(page.getByText(/scottish_smes|EF-extract-208/i).first()).toBeVisible();
  await captureScreenshot(page, "18-workspace-deeplink");
});

test("documents library", async ({ page }) => {
  test.setTimeout(60_000);
  await ready(page);
  await page.goto("/documents", { waitUntil: "domcontentloaded" });
  await expect(page.getByRole("heading", { name: "Documents" })).toBeVisible({ timeout: 20_000 });
  await expect(page.getByText("scottish_smes.pdf").first()).toBeVisible();
  await expect(page.getByText("EF-extract-208.md").first()).toBeVisible();
  await captureScreenshot(page, "19-documents-library");
});
