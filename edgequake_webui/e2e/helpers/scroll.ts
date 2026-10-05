/**
 * Shared scroll assertions for E2E specs.
 *
 * Extracted (DRY) from `spec037-query-settings-scroll.spec.ts` so any spec can
 * assert that a container actually scrolls and can be scrolled to reveal
 * trailing content.
 */
import { expect, type Locator } from "@playwright/test";

export interface ScrollMetrics {
  scrollHeight: number;
  clientHeight: number;
}

/**
 * Assert that `locator` overflows vertically (scrollHeight > clientHeight),
 * i.e. it is genuinely scrollable. Returns the measured metrics so callers can
 * make further assertions.
 */
export async function expectScrollable(
  locator: Locator,
): Promise<ScrollMetrics> {
  await locator.waitFor({ state: "visible" });
  let metrics: ScrollMetrics = { scrollHeight: 0, clientHeight: 0 };
  await expect
    .poll(
      async () => {
        metrics = await locator.evaluate((el) => ({
          scrollHeight: el.scrollHeight,
          clientHeight: el.clientHeight,
        }));
        return (
          metrics.clientHeight > 0 && metrics.scrollHeight > metrics.clientHeight
        );
      },
      { message: "expected the rendered container to overflow vertically" },
    )
    .toBe(true);
  return metrics;
}

/** Scroll `locator` to the very bottom to reveal trailing content. */
export async function scrollToBottom(locator: Locator): Promise<void> {
  await locator.evaluate((el) => {
    el.scrollTop = el.scrollHeight;
  });
}
