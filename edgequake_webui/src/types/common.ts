/** Shared API error, pagination, and query history types. */

import type { QueryMode } from "./query";

export interface ApiError {
  message: string;
  code?: string;
  details?: Record<string, unknown>;
  status: number;
}

// Pagination types
export interface PaginatedResponse<T> {
  items: T[];
  total: number;
  page: number;
  page_size: number;
  /** Total number of pages. */
  total_pages?: number;
  has_more: boolean;
}

export interface PaginationParams {
  page?: number;
  page_size?: number;
  sort_by?: string;
  sort_order?: "asc" | "desc";
}

// Query history
export interface QueryHistoryItem {
  id: string;
  query: string;
  mode: QueryMode;
  response?: string;
  timestamp: string;
  isFavorite: boolean;
}
