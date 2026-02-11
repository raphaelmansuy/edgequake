/**
 * Paginator — auto-iterating pagination helper.
 *
 * WHY: The EdgeQuake API uses both offset-based and page-based pagination.
 * The Paginator normalizes both into a single AsyncIterable<T> interface:
 *
 *   for await (const doc of client.documents.list()) { ... }
 *
 * Consumers can also get a single page or collect all results.
 *
 * @module pagination
 */

import type { Page } from "./types/common.js";

/**
 * Generic paginator that auto-fetches pages and yields items.
 *
 * Implements AsyncIterable so you can use `for await...of` directly.
 */
export class Paginator<T> implements AsyncIterable<T> {
  constructor(
    private readonly fetcher: (
      page: number,
      pageSize: number,
    ) => Promise<Page<T>>,
    private readonly pageSize: number = 20,
  ) {}

  /** Iterate through all items across all pages. */
  async *[Symbol.asyncIterator](): AsyncIterator<T> {
    let page = 1;
    let hasMore = true;

    while (hasMore) {
      const result = await this.fetcher(page, this.pageSize);
      for (const item of result.items) {
        yield item;
      }
      hasMore = result.hasMore;
      page++;
    }
  }

  /** Get a specific page of results. */
  async getPage(page: number): Promise<Page<T>> {
    return this.fetcher(page, this.pageSize);
  }

  /** Collect all results into an array. Use with caution for large datasets. */
  async toArray(): Promise<T[]> {
    const items: T[] = [];
    for await (const item of this) {
      items.push(item);
    }
    return items;
  }

  /** Get the first page only. Shorthand for getPage(1). */
  async firstPage(): Promise<Page<T>> {
    return this.getPage(1);
  }
}
