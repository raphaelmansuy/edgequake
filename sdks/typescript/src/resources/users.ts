/**
 * Users resource — CRUD for user management (admin).
 *
 * WHY: Updated to match Rust ListUsersResponse with pagination.
 * @module resources/users
 * @see edgequake/crates/edgequake-api/src/handlers/auth.rs
 */

import type {
  CreateUserRequest,
  CreateUserResponse,
  ListUsersQuery,
  ListUsersResponse,
  UserInfo,
} from "../types/auth.js";
import { Resource } from "./base.js";

export class UsersResource extends Resource {
  /** Create a new user. */
  async create(request: CreateUserRequest): Promise<CreateUserResponse> {
    return this._post("/api/v1/users", request);
  }

  /**
   * List all users with pagination.
   * WHY: Rust returns ListUsersResponse { users, total, page, page_size, total_pages }.
   */
  async list(query?: ListUsersQuery): Promise<ListUsersResponse> {
    const params = new URLSearchParams();
    if (query?.page) params.set("page", String(query.page));
    if (query?.page_size) params.set("page_size", String(query.page_size));
    if (query?.role) params.set("role", query.role);
    const qs = params.toString();
    return this._get(`/api/v1/users${qs ? `?${qs}` : ""}`);
  }

  /** Get user by ID. */
  async get(userId: string): Promise<UserInfo> {
    return this._get(`/api/v1/users/${userId}`);
  }

  /** Delete a user. */
  async delete(userId: string): Promise<void> {
    await this._del(`/api/v1/users/${userId}`);
  }
}
