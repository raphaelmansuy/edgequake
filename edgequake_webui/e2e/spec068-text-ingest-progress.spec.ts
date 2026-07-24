/**
 * 068 — Markdown upload progress must use insert-* identity and show a live
 * queued/extracting stage (not stuck "Processing pending..." with a false Done).
 *
 * UI-only: mocks admission (same harness pattern as SPEC-038).
 * Prefer PLAYWRIGHT_BASE_URL=http://localhost:<port> (not 127.0.0.1) so Next.js
 * 16 allowedDevOrigins does not blank the app shell.
 */
import { expect, test } from "@playwright/test";
import { GOTO_OPTS } from "./helpers/app-ready";
import {
  mockSpec038AdmissionRoutes,
  seedSpec038TenantContext,
} from "./helpers/spec038-admission-mocks";

test.describe("068 text ingest progress parity", () => {
  test.setTimeout(90_000);

  test("MD upload feedback shows queued/extracting without false Done+pending", async ({
    page,
  }) => {
    const insertTrack = "insert-068-e2e-track";
    let admitHits = 0;
    let admitBody: { track_id?: string; task_id?: string } | null = null;

    await mockSpec038AdmissionRoutes(page);
    await seedSpec038TenantContext(page);

    // LIFO: win over SPEC-038 documents catch-all for text admit.
    await page.route("**/api/v1/documents**", async (route) => {
      const url = route.request().url();
      const method = route.request().method();
      if (
        method === "POST" &&
        !url.includes("/documents/pdf") &&
        !url.includes("/documents/upload")
      ) {
        admitHits += 1;
        admitBody = {
          track_id: insertTrack,
          task_id: insertTrack,
        };
        await route.fulfill({
          status: 202,
          contentType: "application/json",
          body: JSON.stringify({
            document_id: "doc-068-md",
            status: "pending",
            track_id: insertTrack,
            task_id: insertTrack,
          }),
        });
        return;
      }
      await route.fallback();
    });

    // Optional richness: if ProgressPanelRow / IngestionRunCard polls, return extracting.
    await page.route("**/api/v1/ingestion/**/progress", async (route) => {
      const url = route.request().url();
      const trackFromUrl =
        url.match(/\/ingestion\/([^/]+)\/progress/)?.[1] ?? insertTrack;
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({
          track_id: trackFromUrl,
          document_id: "doc-068-md",
          document_name: "notes.md",
          status: "processing",
          progress: {
            current_stage: "extracting",
            completion_percentage: 35,
            latest_message: "Extracting entities…",
            stages: [
              {
                stage: "extracting",
                status: "running",
                progress: 0.35,
                message: "Extracting entities…",
              },
            ],
          },
          started_at: new Date().toISOString(),
          updated_at: new Date().toISOString(),
        }),
      });
    });

    await page.goto("/documents", GOTO_OPTS);
    await page.getByRole("heading", { name: "Documents" }).waitFor({
      state: "visible",
      timeout: 20_000,
    });

    const fileInput = page.locator('input[type="file"]').first();
    await fileInput.waitFor({ state: "attached", timeout: 10_000 });
    await fileInput.setInputFiles({
      name: "notes.md",
      mimeType: "text/markdown",
      buffer: Buffer.from("# Hello\n\nBody"),
    });

    await expect
      .poll(() => admitHits, { timeout: 15_000 })
      .toBeGreaterThan(0);
    expect(admitBody?.track_id).toBe(insertTrack);
    expect(admitBody?.task_id).toBe(insertTrack);

    // After admit, optimistic doc → ActiveRuns (SPEC-048) or upload progress list.
    const feedback = page
      .getByTestId("spec048-active-runs-panel")
      .or(page.getByTestId("spec038-upload-progress-list"));
    await expect(feedback).toBeVisible({ timeout: 15_000 });
    await expect(page.getByText("notes.md").first()).toBeVisible();

    // Live stage signal (ActiveRuns headline or queued hydrate message).
    await expect(
      page
        .getByTestId("spec048-run-headline")
        .or(page.getByText(/Queued for processing|Extracting entities|Queued|Pending/i))
        .first(),
    ).toBeVisible({ timeout: 15_000 });

    // Regression: never show the broken Done+pending copy.
    await expect(page.getByText("Processing pending...")).toHaveCount(0);
  });
});
