/**
 * Authentication types.
 *
 * WHY: Types match Rust DTOs in auth_types.rs exactly.
 * @module types/auth
 * @see edgequake/crates/edgequake-api/src/handlers/auth_types.rs
 */

// ── Login ─────────────────────────────────────────────────────

export interface LoginRequest {
  username: string;
  password: string;
}

export interface LoginResponse {
  access_token: string;
  token_type: string;
  expires_in: number;
  refresh_token: string;
  user: UserInfo;
}

// ── Token ─────────────────────────────────────────────────────

export interface RefreshTokenRequest {
  refresh_token: string;
}

export interface RefreshTokenResponse {
  access_token: string;
  token_type: string;
  expires_in: number;
}

// ── User ──────────────────────────────────────────────────────

export interface UserInfo {
  user_id: string;
  username: string;
  email: string;
  role: string;
}

export interface CreateUserRequest {
  username: string;
  email: string;
  password: string;
  role?: string;
}

/** WHY: Rust CreateUserResponse wraps user in a `user` field + `created_at`. */
export interface CreateUserResponse {
  user: UserInfo;
  created_at: string;
}

/** WHY: Rust GetMeResponse wraps user in a `user` field. */
export interface GetMeResponse {
  user: UserInfo;
}

/** WHY: Matches Rust ListUsersResponse with pagination. */
export interface ListUsersResponse {
  users: UserInfo[];
  total: number;
  page: number;
  page_size: number;
  total_pages: number;
}

export interface ListUsersQuery {
  page?: number;
  page_size?: number;
  role?: string;
}

// ── API Keys ──────────────────────────────────────────────────

/** WHY: Rust fields are all optional. */
export interface CreateApiKeyRequest {
  name?: string;
  scopes?: string[];
  expires_in_days?: number;
}

/** WHY: Matches Rust CreateApiKeyResponse — includes prefix and scopes. */
export interface ApiKeyResponse {
  key_id: string;
  api_key: string;
  prefix: string;
  scopes: string[];
  expires_at?: string;
  created_at: string;
}

/** WHY: Matches Rust ApiKeySummary — for listing (no raw key). */
export interface ApiKeyInfo {
  key_id: string;
  prefix: string;
  name?: string;
  scopes: string[];
  is_active: boolean;
  last_used_at?: string;
  expires_at?: string;
  created_at: string;
}

/** WHY: Matches Rust ListApiKeysResponse with pagination. */
export interface ListApiKeysResponse {
  keys: ApiKeyInfo[];
  total: number;
  page: number;
  page_size: number;
  total_pages: number;
}

/** WHY: Matches Rust RevokeApiKeyResponse. */
export interface RevokeApiKeyResponse {
  key_id: string;
  message: string;
}
