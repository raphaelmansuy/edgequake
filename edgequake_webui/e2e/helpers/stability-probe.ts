/**
 * Hermetic visual-stability probe (Ideas Lab: CLS, long tasks, LoAF, INP).
 *
 * Install before `page.goto`. CLS excludes shifts flagged `hadRecentInput`
 * (user-caused movement within 500 ms). Budget matches web.dev "good" CLS.
 */
import { expect, type Page } from "@playwright/test";

/** web.dev "good" Cumulative Layout Shift. */
export const CLS_BUDGET = 0.1;

export interface StabilitySnapshot {
  cls: number;
  shifts: number;
  longTasks: number;
  longFrames: number;
  /** Event timing durations (ms) observed after install. */
  interactions: number[];
}

export async function installStabilityProbe(page: Page): Promise<void> {
  await page.addInitScript(() => {
    type Snap = {
      cls: number;
      shifts: number;
      longTasks: number;
      longFrames: number;
      interactions: number[];
    };
    const w = window as unknown as { __eqStability?: Snap };
    const snap: Snap = {
      cls: 0,
      shifts: 0,
      longTasks: 0,
      longFrames: 0,
      interactions: [],
    };
    w.__eqStability = snap;

    const observe = (type: string, onEntry: (entry: PerformanceEntry) => void) => {
      try {
        const obs = new PerformanceObserver((list) => {
          for (const entry of list.getEntries()) onEntry(entry);
        });
        obs.observe({ type, buffered: true } as PerformanceObserverInit);
      } catch {
        /* API missing in this browser build */
      }
    };

    observe("layout-shift", (entry) => {
      const ls = entry as PerformanceEntry & {
        value?: number;
        hadRecentInput?: boolean;
      };
      if (!ls.hadRecentInput) {
        snap.cls += ls.value ?? 0;
        snap.shifts += 1;
      }
    });
    observe("longtask", () => {
      snap.longTasks += 1;
    });
    observe("long-animation-frame", (entry) => {
      if (entry.duration > 50) snap.longFrames += 1;
    });
    try {
      const ev = new PerformanceObserver((list) => {
        for (const entry of list.getEntries()) {
          snap.interactions.push(entry.duration);
        }
      });
      ev.observe({
        type: "event",
        buffered: true,
        durationThreshold: 16,
      } as PerformanceObserverInit);
    } catch {
      /* Event Timing unsupported */
    }
  });
}

export async function readStability(page: Page): Promise<StabilitySnapshot> {
  return page.evaluate(() => {
    const snap = (
      window as unknown as { __eqStability?: StabilitySnapshot }
    ).__eqStability;
    return (
      snap ?? {
        cls: 0,
        shifts: 0,
        longTasks: 0,
        longFrames: 0,
        interactions: [],
      }
    );
  });
}

export async function expectClsWithinBudget(
  page: Page,
  budget: number = CLS_BUDGET,
): Promise<StabilitySnapshot> {
  const snap = await readStability(page);
  expect(
    snap.cls,
    `CLS ${snap.cls.toFixed(3)} exceeds budget ${budget} (${snap.shifts} unexpected shifts)`,
  ).toBeLessThanOrEqual(budget);
  return snap;
}
