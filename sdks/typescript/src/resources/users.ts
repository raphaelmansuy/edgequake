/**
 * Users resource — CRUD for user management (admin).
 *
 * @module resources/users
 * @see edgequake/crates/edgequake-api/src/handlers/auth.rs
 */

import { Resource } from "./base.js";
import type {
  CreateUserRequest,
  CreateUserResponse,
  UserInfo,
} from "../types/auth.js";

export class UsersResource extends Resource {
  /** Create a new user. */
  async create(request: CreateUserRequest): Promise<CreateUserResponse> {
    return this._post("/api/v1/users", request);
  }

  /** List all users. */
  async list(): Promise<UserInfo[]> {
    return this._get("/api/v1/users");
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
