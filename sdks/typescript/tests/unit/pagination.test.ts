/**
 * Unit tests for the pagination utility.
 *
 * @module tests/pagination.test
 */

import { describe, expect, it, vi } from "vitest";
import { Paginator } from "../../src/pagination.js";
import type { Page } from "../../src/types/common.js";

function createPage<T>(items: T[], page: number, hasMore: boolean): Page<T> {
  return {
    items,
    total: hasMore ? 100 : items.length + (page - 1) * items.length,
    page,
    pageSize: items.length,
    hasMore,
  };
}

describe("Paginator", () => {
  it("iterates through all pages", async () => {
    const fetcher = vi
      .fn()
      .mockResolvedValueOnce(createPage([1, 2, 3], 1, true))
      .mockResolvedValueOnce(createPage([4, 5, 6], 2, true))
      .mockResolvedValueOnce(createPage([7], 3, false));

    const paginator = new Paginator(fetcher, 3);
    const results: number[] = [];

    for await (const item of paginator) {
      results.push(item);
    }

    expect(results).toEqual([1, 2, 3, 4, 5, 6, 7]);
    expect(fetcher).toHaveBeenCalledTimes(3);
  });

  it("handles single page", async () => {
    const fetcher = vi
      .fn()
      .mockResolvedValueOnce(createPage(["a", "b"], 1, false));

    const paginator = new Paginator(fetcher, 10);
    const results: string[] = [];

    for await (const item of paginator) {
      results.push(item);
    }

    expect(results).toEqual(["a", "b"]);
    expect(fetcher).toHaveBeenCalledTimes(1);
  });

  it("handles empty results", async () => {
    const fetcher = vi.fn().mockResolvedValueOnce(createPage([], 1, false));

    const paginator = new Paginator(fetcher, 10);
    const results: unknown[] = [];

    for await (const item of paginator) {
      results.push(item);
    }

    expect(results).toEqual([]);
  });

  it("getPage returns a specific page", async () => {
    const fetcher = vi
      .fn()
      .mockResolvedValueOnce(createPage([10, 20], 5, true));

    const paginator = new Paginator(fetcher, 2);
    const page = await paginator.getPage(5);

    expect(page.items).toEqual([10, 20]);
    expect(page.page).toBe(5);
    expect(fetcher).toHaveBeenCalledWith(5, 2);
  });

  it("toArray collects all items", async () => {
    const fetcher = vi
      .fn()
      .mockResolvedValueOnce(createPage(["x", "y"], 1, true))
      .mockResolvedValueOnce(createPage(["z"], 2, false));

    const paginator = new Paginator(fetcher, 2);
    const all = await paginator.toArray();

    expect(all).toEqual(["x", "y", "z"]);
  });

  it("firstPage returns first page for quick access", async () => {
    const fetcher = vi
      .fn()
      .mockResolvedValueOnce(createPage([1, 2, 3], 1, true));

    const paginator = new Paginator(fetcher, 3);
    const page = await paginator.firstPage();

    expect(page.items).toEqual([1, 2, 3]);
    expect(page.hasMore).toBe(true);
    expect(fetcher).toHaveBeenCalledWith(1, 3);
  });
});
