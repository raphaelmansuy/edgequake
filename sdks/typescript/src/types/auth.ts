/**
 * Authentication types.
 *
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

export interface CreateUserResponse {
  user_id: string;
  username: string;
  email: string;
  role: string;
}

// ── API Keys ──────────────────────────────────────────────────

export interface CreateApiKeyRequest {
  name: string;
  expires_in_days?: number;
}

export interface ApiKeyResponse {
  key_id: string;
  api_key: string;
  name: string;
  created_at: string;
  expires_at?: string;
}

export interface ApiKeyInfo {
  key_id: string;
  name: string;
  created_at: string;
  expires_at?: string;
  last_used_at?: string;
}
