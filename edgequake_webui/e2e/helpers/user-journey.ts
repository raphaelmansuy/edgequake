/**
 * Shared helpers for the full user-journey suite.
 * Artifacts go to USER_JOURNEY_MEDIA_DIR (screenshots and response captures).
 */
import { mkdir, readFile, writeFile } from "node:fs/promises";
import path from "node:path";

import { expect, type APIRequestContext, type Page, type TestInfo } from "@playwright/test";

import { API_V1_URL } from "./backend-url";
import { tenantHeaders, type AuthHeaders } from "./spec013-api";

export const PDF_RELATIVE =
  "legacy/edgequake-pdf/test-data/scottish_smes.pdf";
export const MARKDOWN_RELATIVE =
  "edgequake/crates/edgequake-query/tests/fixtures/EF-extract-208.md";

export const REPO_ROOT = path.resolve(__dirname, "../../..");
export const PDF_PATH = path.join(REPO_ROOT, PDF_RELATIVE);
export const MARKDOWN_PATH = path.join(REPO_ROOT, MARKDOWN_RELATIVE);

const MEDIA_DIR =
  process.env.USER_JOURNEY_MEDIA_DIR ??
  path.join(REPO_ROOT, "edgequake_webui/e2e/artifacts/user-journeys");

export type JourneyDocs = {
  pdfId: string;
  markdownId: string;
  pdfName: string;
  markdownName: string;
};

export type JourneyContext = {
  tenantId: string;
  workspaceId: string;
  workspaceName: string;
  workspaceSlug: string;
  headers: AuthHeaders;
  docs?: JourneyDocs;
};

/** Credential-shaped values only. Field names such as `api_key` are not secrets. */
const SECRETISH = /\bsk-[A-Za-z0-9]{8,}\b|bearer\s+[A-Za-z0-9._-]{12,}/i;

export function mediaDir(): string {
  return MEDIA_DIR;
}

const STATE_FILE = () => path.join(MEDIA_DIR, "journey-state.json");

/** Survive Playwright worker restarts after a failed test. */
export async function saveJourneyState(ctx: JourneyContext): Promise<void> {
  await ensureMediaDir();
  await writeFile(STATE_FILE(), JSON.stringify(ctx));
}

export async function loadJourneyState(): Promise<JourneyContext | null> {
  try {
    return JSON.parse(await readFile(STATE_FILE(), "utf8")) as JourneyContext;
  } catch {
    return null;
  }
}

export async function ensureMediaDir(): Promise<string> {
  await mkdir(MEDIA_DIR, { recursive: true });
  return MEDIA_DIR;
}

/** Persist a redacted JSON capture. Never write credential material. */
export async function captureJson(name: string, value: unknown): Promise<string> {
  const dir = await ensureMediaDir();
  const file = path.join(dir, `${name}.json`);
  const text = JSON.stringify(value, null, 2);
  if (SECRETISH.test(text)) {
    throw new Error(`refusing to write ${name}: payload looks like a secret`);
  }
  await writeFile(file, text);
  return file;
}

export async function captureScreenshot(page: Page, name: string): Promise<string> {
  const dir = await ensureMediaDir();
  const file = path.join(dir, `${name}.png`);
  await page.screenshot({ path: file, fullPage: true });
  return file;
}

export async function captureText(name: string, text: string): Promise<string> {
  const dir = await ensureMediaDir();
  const file = path.join(dir, name);
  await writeFile(file, text);
  return file;
}

export function apiHeaders(ctx: Pick<JourneyContext, "tenantId" | "workspaceId">): AuthHeaders {
  return tenantHeaders(ctx.tenantId, ctx.workspaceId);
}

export async function apiJson(
  request: APIRequestContext,
  method: "GET" | "POST" | "DELETE",
  pathname: string,
  headers: AuthHeaders,
  data?: unknown,
): Promise<{ status: number; body: unknown; text: string }> {
  const response = await request.fetch(`${API_V1_URL}${pathname}`, {
    method,
    headers,
    data,
  });
  const text = await response.text();
  let body: unknown = text;
  try {
    body = text ? JSON.parse(text) : null;
  } catch {
    body = text;
  }
  return { status: response.status(), body, text };
}

type DocRow = {
  id?: string;
  document_id?: string;
  title?: string | null;
  file_name?: string | null;
  status?: string;
  error_message?: string | null;
  failure_code?: string | null;
  entity_count?: number;
  chunk_count?: number;
  source_type?: string;
};

function asDocList(body: unknown): DocRow[] {
  if (!body || typeof body !== "object") return [];
  const record = body as { documents?: DocRow[]; items?: DocRow[] };
  return record.documents ?? record.items ?? [];
}

export function docId(row: DocRow): string {
  return row.id ?? row.document_id ?? "";
}

export async function listDocuments(
  request: APIRequestContext,
  ctx: JourneyContext,
): Promise<DocRow[]> {
  const listed = await apiJson(
    request,
    "GET",
    "/documents?page=1&page_size=50",
    apiHeaders(ctx),
  );
  expect(listed.status, listed.text).toBe(200);
  return asDocList(listed.body);
}

const TERMINAL = new Set([
  "completed",
  "indexed",
  "failed",
  "partial_failure",
  "cancelled",
]);

export async function waitForDocument(
  request: APIRequestContext,
  ctx: JourneyContext,
  id: string,
  timeoutMs: number,
): Promise<DocRow> {
  const started = Date.now();
  let last: DocRow = { id, status: "missing" };
  while (Date.now() - started < timeoutMs) {
    const got = await apiJson(request, "GET", `/documents/${id}`, apiHeaders(ctx));
    if (got.status === 200 && got.body && typeof got.body === "object") {
      last = got.body as DocRow;
      if (last.status && TERMINAL.has(last.status)) return last;
    }
    await new Promise((resolve) => setTimeout(resolve, 5_000));
  }
  return last;
}

export function assertCompleted(row: DocRow, label: string): void {
  const status = row.status ?? "missing";
  expect(
    status,
    `${label} status=${status} error=${row.error_message ?? ""} code=${row.failure_code ?? ""}`,
  ).toMatch(/completed|indexed/);
}

export async function recordJourney(
  info: TestInfo,
  extra: Record<string, unknown>,
): Promise<void> {
  const dir = await ensureMediaDir();
  const file = path.join(dir, "results.jsonl");
  const line = JSON.stringify({
    journey: info.title,
    status: info.status,
    durationMs: info.duration,
    error: info.error?.message?.slice(0, 500),
    pdf: PDF_RELATIVE,
    markdown: MARKDOWN_RELATIVE,
    ...extra,
  });
  const { appendFile } = await import("node:fs/promises");
  await appendFile(file, `${line}\n`);
}
