/**
 * SPEC-184 — the Query empty state must ask questions this corpus can answer.
 *
 * WHY this spec exists as a browser test and not only as a unit test: the defect was never in the
 * pure helper alone. Three separate plumbing bugs made the rule unreachable in the real screen —
 * the graph branch returned before the rule ran, the entity query used a workspace-independent
 * cache key, and the entity endpoint returns `{ nodes }` while the caller read `{ items }`, so the
 * entity suggestions never rendered at all. A unit test on the composer cannot see any of those.
 */

import { expect, test } from "@playwright/test";
import { openNormalWorkspace } from "./spec183/helpers/normal-workspace";
import { spec183Shot } from "./spec183/helpers/shots";

const suggestions = (page: import("@playwright/test").Page) =>
  page.locator('[role="listitem"]');

test.describe("@spec184 default questions", () => {
  test.beforeEach(({}, info) => info.setTimeout(120_000));

  test("a text-only normal corpus is asked no picture question", async ({ page }) => {
    await openNormalWorkspace(page, { path: "/query", images: false });
    await expect(page.getByTestId("query-composer")).toBeVisible({ timeout: 20_000 });

    const cards = await suggestions(page).allInnerTexts();
    const joined = cards.join(" ");
    // The workspace cannot embed a picture (EC-180-18), so no card may promise otherwise.
    expect(joined).not.toMatch(/picture|image/i);
    // And no fabricated subject from the old copy.
    expect(joined).not.toMatch(/refund|onboarding|the handbook/i);
    await spec183Shot(page, "23-spec184-query-text-only-suggestions");
  });

  test("an image corpus holding a picture is asked about pictures, never about a handbook", async ({
    page,
  }) => {
    await openNormalWorkspace(page, { path: "/query", images: true, withPicture: true });
    await expect(page.getByTestId("query-composer")).toBeVisible({ timeout: 20_000 });
    // The corpus query resolves after the composer mounts. Reading the cards
    // in that gap sees an empty list.
    const picture = suggestions(page).filter({ hasText: /picture/i });
    await expect(picture.first()).toBeVisible({ timeout: 20_000 });

    const joined = (await suggestions(page).allInnerTexts()).join(" ");
    expect(joined).toMatch(/picture/i);
    expect(joined).not.toMatch(/refund|onboarding|the handbook/i);
  });

  test("a workspace that can store pictures but holds none is not asked about them", async ({
    page,
  }) => {
    // Capability and contents are different facts. This workspace embeds images, so it *could*
    // hold a photograph — but it holds only text, so a picture question would be unanswerable,
    // which is the original defect one level down.
    await openNormalWorkspace(page, { path: "/query", images: true });
    await expect(page.getByTestId("query-composer")).toBeVisible({ timeout: 20_000 });

    const joined = (await suggestions(page).allInnerTexts()).join(" ");
    expect(joined).not.toMatch(/picture/i);
  });

  test("no card carries an unresolved template placeholder", async ({ page }) => {
    // The screen once rendered a literal `{{topic}}` because the lib and the component each
    // interpolated differently. This is the guard at the only place that matters: the DOM.
    await openNormalWorkspace(page, { path: "/query", images: true });
    await expect(page.getByTestId("query-composer")).toBeVisible({ timeout: 20_000 });

    const cards = await suggestions(page).allInnerTexts();
    for (const card of cards) {
      expect(card).not.toContain("{{");
      expect(card).not.toContain("}}");
      expect(card.trim()).not.toBe("");
    }
  });
});
