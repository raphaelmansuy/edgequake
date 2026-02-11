/**
 * Auth resource — login, refresh, logout, current user.
 *
 * @module resources/auth
 * @see edgequake/crates/edgequake-api/src/handlers/auth.rs
 */

import type {
  LoginRequest,
  LoginResponse,
  RefreshTokenRequest,
  RefreshTokenResponse,
  UserInfo,
} from "../types/auth.js";
import { Resource } from "./base.js";

export class AuthResource extends Resource {
  /** Login with username and password. Returns JWT tokens. */
  async login(request: LoginRequest): Promise<LoginResponse> {
    return this._post("/api/v1/auth/login", request);
  }

  /** Refresh an expired access token using a refresh token. */
  async refresh(request: RefreshTokenRequest): Promise<RefreshTokenResponse> {
    return this._post("/api/v1/auth/refresh", request);
  }

  /** Logout and invalidate current tokens. */
  async logout(): Promise<void> {
    await this._post("/api/v1/auth/logout");
  }

  /** Get current authenticated user information. */
  async me(): Promise<UserInfo> {
    return this._get("/api/v1/auth/me");
  }
}
